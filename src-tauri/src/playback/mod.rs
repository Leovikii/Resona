// SPDX-License-Identifier: GPL-3.0-only

mod output;

use std::collections::VecDeque;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use rodio::source::Done;
use rodio::source::SeekError;
use rodio::{ChannelCount, Decoder, Player, SampleRate, Source};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use output::OutputDeviceManager;
use output::OutputSnapshot;

const COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
const ACTOR_TICK: Duration = Duration::from_millis(150);
const DEVICE_POLL_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaybackStatus {
    #[default]
    Idle,
    Playing,
    Paused,
    Stopped,
    Failed,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackMode {
    #[default]
    Sequential,
    RepeatOne,
    RepeatAll,
    Shuffle,
}

impl PlaybackMode {
    pub fn storage_key(self) -> &'static str {
        match self {
            Self::Sequential => "sequential",
            Self::RepeatOne => "repeat_one",
            Self::RepeatAll => "repeat_all",
            Self::Shuffle => "shuffle",
        }
    }

    pub fn from_storage_key(value: &str) -> Self {
        match value {
            "repeat_one" => Self::RepeatOne,
            "repeat_all" => Self::RepeatAll,
            "shuffle" => Self::Shuffle,
            _ => Self::Sequential,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueItemStatus {
    Pending,
    Playing,
    Paused,
    Played,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueItemSnapshot {
    pub id: u64,
    pub path: String,
    pub display_name: String,
    pub duration_ms: Option<u64>,
    pub status: QueueItemStatus,
    pub error: Option<PlaybackFailure>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSnapshot {
    pub status: PlaybackStatus,
    pub path: Option<String>,
    pub error: Option<PlaybackFailure>,
    pub position_ms: u64,
    pub duration_ms: Option<u64>,
    pub volume: f32,
    pub seekable: bool,
    pub queue: Vec<QueueItemSnapshot>,
    pub current_item_id: Option<u64>,
    pub playback_mode: PlaybackMode,
    pub output: OutputSnapshot,
}

#[derive(Clone, Debug)]
pub struct RestoredPlaybackSession {
    pub paths: Vec<PathBuf>,
    pub current_path: Option<PathBuf>,
    pub position_ms: u64,
    pub volume: f32,
    pub playback_mode: PlaybackMode,
}

impl Default for PlaybackSnapshot {
    fn default() -> Self {
        Self {
            status: PlaybackStatus::Idle,
            path: None,
            error: None,
            position_ms: 0,
            duration_ms: None,
            volume: 1.0,
            seekable: false,
            queue: Vec::new(),
            current_item_id: None,
            playback_mode: PlaybackMode::Sequential,
            output: OutputSnapshot::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackErrorCode {
    UnsupportedFormat,
    InvalidFile,
    OpenFile,
    Decode,
    OpenOutput,
    NothingLoaded,
    InvalidVolume,
    Seek,
    EngineUnavailable,
    EngineTimeout,
    TaskFailed,
    QueueItemNotFound,
    ListOutputDevices,
    InvalidOutputDevice,
    OutputDeviceUnavailable,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PlaybackFailure {
    pub code: PlaybackErrorCode,
    pub message: String,
}

impl PlaybackFailure {
    pub fn task_failed(message: String) -> Self {
        Self {
            code: PlaybackErrorCode::TaskFailed,
            message,
        }
    }
}

#[derive(Debug, Error)]
pub enum PlaybackError {
    #[error("只支持 MP3、WAV 或 FLAC 文件")]
    UnsupportedFormat,
    #[error("音频文件不存在或不是普通文件：{0}")]
    InvalidFile(String),
    #[error("无法打开音频文件：{0}")]
    OpenFile(String),
    #[error("无法解码音频文件：{0}")]
    Decode(String),
    #[error("无法打开默认音频输出设备：{0}")]
    OpenOutput(String),
    #[error("当前没有可控制的音频")]
    NothingLoaded,
    #[error("音量必须是 0.0 到 1.0 之间的有限数值")]
    InvalidVolume,
    #[error("无法定位音频：{0}")]
    Seek(String),
    #[error("播放引擎不可用")]
    EngineUnavailable,
    #[error("播放引擎响应超时")]
    EngineTimeout,
    #[error("播放列表项目不存在")]
    QueueItemNotFound,
    #[error("无法列出输出设备：{0}")]
    ListOutputDevices(String),
    #[error("输出设备 ID 无效：{0}")]
    InvalidOutputDevice(String),
    #[error("输出设备不可用：{0}")]
    OutputDeviceUnavailable(String),
}

impl PlaybackError {
    pub fn failure(&self) -> PlaybackFailure {
        let code = match self {
            Self::UnsupportedFormat => PlaybackErrorCode::UnsupportedFormat,
            Self::InvalidFile(_) => PlaybackErrorCode::InvalidFile,
            Self::OpenFile(_) => PlaybackErrorCode::OpenFile,
            Self::Decode(_) => PlaybackErrorCode::Decode,
            Self::OpenOutput(_) => PlaybackErrorCode::OpenOutput,
            Self::NothingLoaded => PlaybackErrorCode::NothingLoaded,
            Self::InvalidVolume => PlaybackErrorCode::InvalidVolume,
            Self::Seek(_) => PlaybackErrorCode::Seek,
            Self::EngineUnavailable => PlaybackErrorCode::EngineUnavailable,
            Self::EngineTimeout => PlaybackErrorCode::EngineTimeout,
            Self::QueueItemNotFound => PlaybackErrorCode::QueueItemNotFound,
            Self::ListOutputDevices(_) => PlaybackErrorCode::ListOutputDevices,
            Self::InvalidOutputDevice(_) => PlaybackErrorCode::InvalidOutputDevice,
            Self::OutputDeviceUnavailable(_) => PlaybackErrorCode::OutputDeviceUnavailable,
        };
        PlaybackFailure {
            code,
            message: self.to_string(),
        }
    }
}

pub trait PlaybackEngine {
    fn append_queue(&self, paths: Vec<PathBuf>) -> Result<PlaybackSnapshot, PlaybackError>;
    fn insert_queue(
        &self,
        paths: Vec<PathBuf>,
        at_index: usize,
    ) -> Result<PlaybackSnapshot, PlaybackError>;
    fn replace_queue_and_play(
        &self,
        paths: Vec<PathBuf>,
        selected_index: usize,
    ) -> Result<PlaybackSnapshot, PlaybackError>;
    fn play_queue_item(&self, id: u64) -> Result<PlaybackSnapshot, PlaybackError>;
    fn remove_queue_item(&self, id: u64) -> Result<PlaybackSnapshot, PlaybackError>;
    fn move_queue_item(&self, id: u64, to_index: usize) -> Result<PlaybackSnapshot, PlaybackError>;
    fn previous(&self) -> Result<PlaybackSnapshot, PlaybackError>;
    fn next(&self) -> Result<PlaybackSnapshot, PlaybackError>;
    fn set_playback_mode(&self, mode: PlaybackMode) -> Result<PlaybackSnapshot, PlaybackError>;
    fn refresh_output_devices(&self) -> Result<PlaybackSnapshot, PlaybackError>;
    fn select_output_device(
        &self,
        device_id: Option<String>,
    ) -> Result<PlaybackSnapshot, PlaybackError>;
    fn pause(&self) -> Result<PlaybackSnapshot, PlaybackError>;
    fn resume(&self) -> Result<PlaybackSnapshot, PlaybackError>;
    fn stop(&self) -> Result<PlaybackSnapshot, PlaybackError>;
    fn seek(&self, position_ms: u64) -> Result<PlaybackSnapshot, PlaybackError>;
    fn set_volume(&self, volume: f32) -> Result<PlaybackSnapshot, PlaybackError>;
    fn snapshot(&self) -> Result<PlaybackSnapshot, PlaybackError>;
    fn restore_session(
        &self,
        session: RestoredPlaybackSession,
    ) -> Result<PlaybackSnapshot, PlaybackError>;
}

type CommandResult = Result<PlaybackSnapshot, PlaybackError>;
type ResponseSender = Sender<CommandResult>;

enum AudioCommand {
    AppendQueue {
        paths: Vec<PathBuf>,
        response: ResponseSender,
    },
    InsertQueue {
        paths: Vec<PathBuf>,
        at_index: usize,
        response: ResponseSender,
    },
    ReplaceQueueAndPlay {
        paths: Vec<PathBuf>,
        selected_index: usize,
        response: ResponseSender,
    },
    PlayQueueItem {
        id: u64,
        response: ResponseSender,
    },
    RemoveQueueItem {
        id: u64,
        response: ResponseSender,
    },
    MoveQueueItem {
        id: u64,
        to_index: usize,
        response: ResponseSender,
    },
    Previous(ResponseSender),
    Next(ResponseSender),
    SetPlaybackMode {
        mode: PlaybackMode,
        response: ResponseSender,
    },
    RefreshOutputDevices(ResponseSender),
    SelectOutputDevice {
        device_id: Option<String>,
        response: ResponseSender,
    },
    Pause(ResponseSender),
    Resume(ResponseSender),
    Stop(ResponseSender),
    Seek {
        position_ms: u64,
        response: ResponseSender,
    },
    SetVolume {
        volume: f32,
        response: ResponseSender,
    },
    Snapshot(ResponseSender),
    RestoreSession {
        session: RestoredPlaybackSession,
        response: ResponseSender,
    },
    Shutdown,
}

pub struct RodioPlaybackEngine {
    commands: Sender<AudioCommand>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl RodioPlaybackEngine {
    pub fn new() -> Self {
        let (commands, receiver) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("resona-audio".to_owned())
            .spawn(move || AudioActor::new().run(receiver))
            .expect("failed to start audio actor");

        Self {
            commands,
            worker: Mutex::new(Some(worker)),
        }
    }

    fn request<F>(&self, build: F) -> CommandResult
    where
        F: FnOnce(ResponseSender) -> AudioCommand,
    {
        let (response, receiver) = mpsc::channel();
        self.commands
            .send(build(response))
            .map_err(|_| PlaybackError::EngineUnavailable)?;

        receiver
            .recv_timeout(COMMAND_TIMEOUT)
            .map_err(|error| match error {
                mpsc::RecvTimeoutError::Timeout => PlaybackError::EngineTimeout,
                mpsc::RecvTimeoutError::Disconnected => PlaybackError::EngineUnavailable,
            })?
    }
}

impl Default for RodioPlaybackEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaybackEngine for RodioPlaybackEngine {
    fn append_queue(&self, paths: Vec<PathBuf>) -> CommandResult {
        self.request(|response| AudioCommand::AppendQueue { paths, response })
    }

    fn insert_queue(&self, paths: Vec<PathBuf>, at_index: usize) -> CommandResult {
        self.request(|response| AudioCommand::InsertQueue {
            paths,
            at_index,
            response,
        })
    }

    fn replace_queue_and_play(&self, paths: Vec<PathBuf>, selected_index: usize) -> CommandResult {
        self.request(|response| AudioCommand::ReplaceQueueAndPlay {
            paths,
            selected_index,
            response,
        })
    }

    fn play_queue_item(&self, id: u64) -> CommandResult {
        self.request(|response| AudioCommand::PlayQueueItem { id, response })
    }

    fn remove_queue_item(&self, id: u64) -> CommandResult {
        self.request(|response| AudioCommand::RemoveQueueItem { id, response })
    }

    fn move_queue_item(&self, id: u64, to_index: usize) -> CommandResult {
        self.request(|response| AudioCommand::MoveQueueItem {
            id,
            to_index,
            response,
        })
    }

    fn previous(&self) -> CommandResult {
        self.request(AudioCommand::Previous)
    }

    fn next(&self) -> CommandResult {
        self.request(AudioCommand::Next)
    }

    fn set_playback_mode(&self, mode: PlaybackMode) -> CommandResult {
        self.request(|response| AudioCommand::SetPlaybackMode { mode, response })
    }

    fn refresh_output_devices(&self) -> CommandResult {
        self.request(AudioCommand::RefreshOutputDevices)
    }

    fn select_output_device(&self, device_id: Option<String>) -> CommandResult {
        self.request(|response| AudioCommand::SelectOutputDevice {
            device_id,
            response,
        })
    }

    fn pause(&self) -> CommandResult {
        self.request(AudioCommand::Pause)
    }

    fn resume(&self) -> CommandResult {
        self.request(AudioCommand::Resume)
    }

    fn stop(&self) -> CommandResult {
        self.request(AudioCommand::Stop)
    }

    fn seek(&self, position_ms: u64) -> CommandResult {
        self.request(|response| AudioCommand::Seek {
            position_ms,
            response,
        })
    }

    fn set_volume(&self, volume: f32) -> CommandResult {
        self.request(|response| AudioCommand::SetVolume { volume, response })
    }

    fn snapshot(&self) -> CommandResult {
        self.request(AudioCommand::Snapshot)
    }

    fn restore_session(&self, session: RestoredPlaybackSession) -> CommandResult {
        self.request(|response| AudioCommand::RestoreSession { session, response })
    }
}

impl Drop for RodioPlaybackEngine {
    fn drop(&mut self) {
        let _ = self.commands.send(AudioCommand::Shutdown);
        if let Ok(mut worker) = self.worker.lock() {
            if let Some(worker) = worker.take() {
                let _ = worker.join();
            }
        }
    }
}

struct AudioActor {
    output: OutputDeviceManager,
    player: Option<Player>,
    snapshot: PlaybackSnapshot,
    queue: Vec<QueueItem>,
    next_queue_id: u64,
    current_index: Option<usize>,
    source_signals: VecDeque<QueuedSourceSignal>,
    last_device_poll: Instant,
    recovery: Option<OutputRecovery>,
    restored_position_ms: Option<u64>,
}

const SOURCE_QUEUED: usize = 0;
const SOURCE_STARTED: usize = 1;
const SOURCE_CANCELLED: usize = 2;

struct QueuedSourceSignal {
    index: usize,
    done: Arc<AtomicUsize>,
    state: Arc<AtomicUsize>,
}

impl QueuedSourceSignal {
    fn cancel_if_queued(&self) -> bool {
        self.state
            .compare_exchange(
                SOURCE_QUEUED,
                SOURCE_CANCELLED,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }
}

struct CancelableSource<S> {
    input: S,
    state: Arc<AtomicUsize>,
}

impl<S> Iterator for CancelableSource<S>
where
    S: Source,
{
    type Item = S::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let state = self.state.load(Ordering::Acquire);
        if state == SOURCE_CANCELLED {
            return None;
        }
        if state == SOURCE_QUEUED {
            let _ = self.state.compare_exchange(
                SOURCE_QUEUED,
                SOURCE_STARTED,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
        }
        if self.state.load(Ordering::Acquire) == SOURCE_CANCELLED {
            None
        } else {
            self.input.next()
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.state.load(Ordering::Acquire) == SOURCE_CANCELLED {
            (0, Some(0))
        } else {
            self.input.size_hint()
        }
    }
}

impl<S> Source for CancelableSource<S>
where
    S: Source,
{
    fn current_span_len(&self) -> Option<usize> {
        if self.state.load(Ordering::Acquire) == SOURCE_CANCELLED {
            Some(0)
        } else {
            self.input.current_span_len()
        }
    }

    fn channels(&self) -> ChannelCount {
        self.input.channels()
    }

    fn sample_rate(&self) -> SampleRate {
        self.input.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.input.total_duration()
    }

    fn try_seek(&mut self, position: Duration) -> Result<(), SeekError> {
        self.input.try_seek(position)
    }
}

#[derive(Clone, Copy, Debug)]
struct OutputRecovery {
    index: usize,
    position_ms: u64,
    paused: bool,
}

#[derive(Clone, Debug)]
struct QueueItem {
    id: u64,
    path: PathBuf,
    display_name: String,
    duration_ms: Option<u64>,
    status: QueueItemStatus,
    error: Option<PlaybackFailure>,
}

impl QueueItem {
    fn new(id: u64, path: PathBuf) -> Self {
        let display_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        Self {
            id,
            path,
            display_name,
            duration_ms: None,
            status: QueueItemStatus::Pending,
            error: None,
        }
    }

    fn snapshot(&self) -> QueueItemSnapshot {
        QueueItemSnapshot {
            id: self.id,
            path: self.path.to_string_lossy().into_owned(),
            display_name: self.display_name.clone(),
            duration_ms: self.duration_ms,
            status: self.status,
            error: self.error.clone(),
        }
    }
}

impl AudioActor {
    fn new() -> Self {
        let output = OutputDeviceManager::new();
        Self {
            snapshot: PlaybackSnapshot {
                output: output.snapshot(),
                ..PlaybackSnapshot::default()
            },
            output,
            player: None,
            queue: Vec::new(),
            next_queue_id: 1,
            current_index: None,
            source_signals: VecDeque::new(),
            last_device_poll: Instant::now(),
            recovery: None,
            restored_position_ms: None,
        }
    }

    fn run(mut self, receiver: Receiver<AudioCommand>) {
        loop {
            match receiver.recv_timeout(ACTOR_TICK) {
                Ok(AudioCommand::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
                Ok(command) => self.handle(command),
                Err(RecvTimeoutError::Timeout) => {
                    self.refresh_finished_playback();
                    self.refresh_output_lifecycle();
                }
            }
        }

        if let Some(player) = &self.player {
            player.stop();
        }
    }

    fn handle(&mut self, command: AudioCommand) {
        let (result, response, replace_state_on_error) = match command {
            AudioCommand::AppendQueue { paths, response } => {
                (self.append_queue(paths), response, false)
            }
            AudioCommand::InsertQueue {
                paths,
                at_index,
                response,
            } => (self.insert_queue(paths, at_index), response, false),
            AudioCommand::ReplaceQueueAndPlay {
                paths,
                selected_index,
                response,
            } => (
                self.replace_queue_and_play(paths, selected_index),
                response,
                true,
            ),
            AudioCommand::PlayQueueItem { id, response } => {
                (self.play_queue_item(id), response, true)
            }
            AudioCommand::RemoveQueueItem { id, response } => {
                (self.remove_queue_item(id), response, false)
            }
            AudioCommand::MoveQueueItem {
                id,
                to_index,
                response,
            } => (self.move_queue_item(id, to_index), response, false),
            AudioCommand::Previous(response) => (self.previous(), response, false),
            AudioCommand::Next(response) => (self.next(), response, true),
            AudioCommand::SetPlaybackMode { mode, response } => {
                (self.set_playback_mode(mode), response, false)
            }
            AudioCommand::RefreshOutputDevices(response) => {
                (self.refresh_output_devices(), response, false)
            }
            AudioCommand::SelectOutputDevice {
                device_id,
                response,
            } => (self.select_output_device(device_id), response, false),
            AudioCommand::Pause(response) => (self.pause(), response, false),
            AudioCommand::Resume(response) => (self.resume(), response, false),
            AudioCommand::Stop(response) => (self.stop(), response, false),
            AudioCommand::Seek {
                position_ms,
                response,
            } => (self.seek(position_ms), response, false),
            AudioCommand::SetVolume { volume, response } => {
                (self.set_volume(volume), response, false)
            }
            AudioCommand::Snapshot(response) => {
                self.refresh_playback_state();
                self.sync_output_snapshot();
                (Ok(self.snapshot.clone()), response, false)
            }
            AudioCommand::RestoreSession { session, response } => {
                (self.restore_session(session), response, false)
            }
            AudioCommand::Shutdown => return,
        };

        if let Err(error) = &result {
            if replace_state_on_error && !matches!(error, PlaybackError::NothingLoaded) {
                self.snapshot.status = PlaybackStatus::Failed;
            }
            self.snapshot.error = Some(error.failure());
        }
        let _ = response.send(result);
    }

    fn replace_queue(&mut self, paths: Vec<PathBuf>) -> CommandResult {
        self.stop_player();
        self.queue = paths
            .into_iter()
            .map(|path| {
                let id = self.next_queue_id;
                self.next_queue_id = self.next_queue_id.saturating_add(1);
                QueueItem::new(id, path)
            })
            .collect();
        self.current_index = None;
        self.snapshot = PlaybackSnapshot {
            status: if self.queue.is_empty() {
                PlaybackStatus::Idle
            } else {
                PlaybackStatus::Stopped
            },
            volume: self.snapshot.volume,
            playback_mode: self.snapshot.playback_mode,
            queue: self.queue.iter().map(QueueItem::snapshot).collect(),
            output: self.output.snapshot(),
            ..PlaybackSnapshot::default()
        };
        Ok(self.snapshot.clone())
    }

    fn restore_session(&mut self, session: RestoredPlaybackSession) -> CommandResult {
        let paths = session
            .paths
            .into_iter()
            .filter(|path| validate_audio_path(path).is_ok())
            .collect::<Vec<_>>();
        self.snapshot.playback_mode = session.playback_mode;
        self.snapshot.volume = if session.volume.is_finite() {
            session.volume.clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.replace_queue(paths)?;
        let Some(current_path) = session.current_path else {
            return Ok(self.snapshot.clone());
        };
        let Some(index) = self.queue.iter().position(|item| item.path == current_path) else {
            return Ok(self.snapshot.clone());
        };
        let (_, duration_ms) = decode_path(&self.queue[index].path)?;
        self.queue[index].duration_ms = duration_ms;
        self.current_index = Some(index);
        let position_ms = duration_ms.map_or(session.position_ms, |duration| {
            session.position_ms.min(duration)
        });
        self.snapshot.status = PlaybackStatus::Stopped;
        self.snapshot.position_ms = position_ms;
        self.restored_position_ms = Some(position_ms);
        self.sync_snapshot_queue();
        Ok(self.snapshot.clone())
    }

    fn append_queue(&mut self, paths: Vec<PathBuf>) -> CommandResult {
        for path in paths {
            let id = self.next_queue_id;
            self.next_queue_id = self.next_queue_id.saturating_add(1);
            self.queue.push(QueueItem::new(id, path));
        }
        if self.current_index.is_some() && self.player.is_some() {
            self.preload_next()?;
        }
        self.sync_snapshot_queue();
        Ok(self.snapshot.clone())
    }

    fn insert_queue(&mut self, paths: Vec<PathBuf>, at_index: usize) -> CommandResult {
        if paths.is_empty() {
            return Ok(self.snapshot.clone());
        }
        let insertion = at_index.min(self.queue.len());
        if insertion == self.queue.len() {
            return self.append_queue(paths);
        }
        let added = paths
            .into_iter()
            .map(|path| {
                let id = self.next_queue_id;
                self.next_queue_id = self.next_queue_id.saturating_add(1);
                QueueItem::new(id, path)
            })
            .collect::<Vec<_>>();
        let added_count = added.len();
        self.queue.splice(insertion..insertion, added);
        if let Some(current) = self.current_index {
            self.current_index = Some(if insertion <= current {
                current + added_count
            } else {
                current
            });
            if matches!(
                self.snapshot.status,
                PlaybackStatus::Playing | PlaybackStatus::Paused
            ) {
                self.rebuild_current_player()?;
            }
        }
        self.sync_snapshot_queue();
        Ok(self.snapshot.clone())
    }

    fn replace_queue_and_play(
        &mut self,
        paths: Vec<PathBuf>,
        selected_index: usize,
    ) -> CommandResult {
        if selected_index >= paths.len() {
            return Err(PlaybackError::QueueItemNotFound);
        }
        self.replace_queue(paths)?;
        self.start_from_index(selected_index)
    }

    fn play_queue_item(&mut self, id: u64) -> CommandResult {
        let index = self
            .queue
            .iter()
            .position(|item| item.id == id)
            .ok_or(PlaybackError::QueueItemNotFound)?;
        let position_ms = if self.current_index == Some(index)
            && self.snapshot.status == PlaybackStatus::Stopped
        {
            self.restored_position_ms.take().unwrap_or(0)
        } else {
            self.restored_position_ms = None;
            0
        };
        self.start_from_index_at(index, position_ms)
    }

    fn remove_queue_item(&mut self, id: u64) -> CommandResult {
        let index = self
            .queue
            .iter()
            .position(|item| item.id == id)
            .ok_or(PlaybackError::QueueItemNotFound)?;
        let was_current = self.current_index == Some(index);
        self.queue.remove(index);

        if self.queue.is_empty() {
            self.stop_player();
            self.current_index = None;
            self.snapshot = PlaybackSnapshot {
                status: PlaybackStatus::Idle,
                volume: self.snapshot.volume,
                playback_mode: self.snapshot.playback_mode,
                output: self.output.snapshot(),
                ..PlaybackSnapshot::default()
            };
            return Ok(self.snapshot.clone());
        }

        if let Some(current) = self.current_index {
            if was_current {
                let next = current.min(self.queue.len().saturating_sub(1));
                return self.start_from_index(next);
            }
            self.current_index = Some(if index < current {
                current - 1
            } else {
                current
            });
            if self.snapshot.status == PlaybackStatus::Playing
                || self.snapshot.status == PlaybackStatus::Paused
            {
                self.rebuild_current_player()?;
            }
        }
        self.sync_snapshot_queue();
        Ok(self.snapshot.clone())
    }

    fn move_queue_item(&mut self, id: u64, to_index: usize) -> CommandResult {
        let from_index = self
            .queue
            .iter()
            .position(|item| item.id == id)
            .ok_or(PlaybackError::QueueItemNotFound)?;
        if self.queue.len() < 2 {
            return Ok(self.snapshot.clone());
        }
        let bounded_index = to_index.min(self.queue.len() - 1);
        let item = self.queue.remove(from_index);
        self.queue.insert(bounded_index, item);

        if let Some(current) = self.current_index {
            self.current_index = Some(if current == from_index {
                bounded_index
            } else if from_index < current && bounded_index >= current {
                current - 1
            } else if from_index > current && bounded_index <= current {
                current + 1
            } else {
                current
            });
            if self.snapshot.status == PlaybackStatus::Playing
                || self.snapshot.status == PlaybackStatus::Paused
            {
                self.rebuild_current_player()?;
            }
        }
        self.sync_snapshot_queue();
        Ok(self.snapshot.clone())
    }

    fn next(&mut self) -> CommandResult {
        let current = self.current_index.ok_or(PlaybackError::NothingLoaded)?;
        let next = self
            .manual_next_index(current)
            .ok_or(PlaybackError::NothingLoaded)?;
        self.start_from_index(next)
    }

    fn set_playback_mode(&mut self, mode: PlaybackMode) -> CommandResult {
        self.refresh_playback_state();
        if self.snapshot.playback_mode == mode {
            return Ok(self.snapshot.clone());
        }
        self.snapshot.playback_mode = mode;
        self.cancel_preloaded_sources();
        self.preload_next()?;
        Ok(self.snapshot.clone())
    }

    fn previous(&mut self) -> CommandResult {
        const RESTART_THRESHOLD_MS: u64 = 3_000;
        if self.snapshot.position_ms > RESTART_THRESHOLD_MS {
            return self.seek(0);
        }
        let current = self.current_index.ok_or(PlaybackError::NothingLoaded)?;
        let previous = self
            .queue
            .iter()
            .enumerate()
            .take(current)
            .rev()
            .find(|(_, item)| item.status != QueueItemStatus::Failed)
            .map(|(index, _)| index)
            .unwrap_or(current);
        self.start_from_index(previous)
    }

    fn refresh_output_devices(&mut self) -> CommandResult {
        self.output.refresh()?;
        self.sync_output_snapshot();
        Ok(self.snapshot.clone())
    }

    fn select_output_device(&mut self, device_id: Option<String>) -> CommandResult {
        let recovery = self.capture_output_recovery();
        self.output.select(device_id)?;
        if let Some(recovery) = recovery {
            self.stop_player();
            self.recovery = Some(recovery);
            self.restore_output_recovery()?;
        }
        self.snapshot.error = None;
        self.sync_output_snapshot();
        Ok(self.snapshot.clone())
    }

    fn start_from_index(&mut self, index: usize) -> CommandResult {
        self.start_from_index_at(index, 0)
    }

    fn start_from_index_at(&mut self, index: usize, position_ms: u64) -> CommandResult {
        if index >= self.queue.len() {
            return Err(PlaybackError::QueueItemNotFound);
        }
        self.stop_player();
        self.current_index = None;
        for item in &mut self.queue {
            if matches!(
                item.status,
                QueueItemStatus::Playing | QueueItemStatus::Paused
            ) {
                item.status = QueueItemStatus::Pending;
            }
        }

        let mut candidate = index;
        loop {
            let (decoder, duration_ms) = match decode_path(&self.queue[candidate].path) {
                Ok(value) => value,
                Err(error) => {
                    self.queue[candidate].status = QueueItemStatus::Failed;
                    self.queue[candidate].error = Some(error.failure());
                    let Some(next) = self.next_valid_index(candidate + 1) else {
                        self.snapshot.status = PlaybackStatus::Failed;
                        self.sync_snapshot_queue();
                        return Err(PlaybackError::Decode(
                            self.queue[candidate]
                                .error
                                .as_ref()
                                .map(|failure| failure.message.clone())
                                .unwrap_or_else(|| "无法解码队列项目".to_owned()),
                        ));
                    };
                    candidate = next;
                    continue;
                }
            };
            self.queue[candidate].duration_ms = duration_ms;
            self.ensure_output()?;
            let output = self
                .output
                .output()
                .ok_or(PlaybackError::EngineUnavailable)?;
            let player = Player::connect_new(output.mixer());
            player.set_volume(self.snapshot.volume);
            let signal = append_marked(&player, decoder, candidate);
            player.play();
            let bounded_position =
                duration_ms.map_or(position_ms, |duration| position_ms.min(duration));
            if bounded_position > 0 {
                player
                    .try_seek(Duration::from_millis(bounded_position))
                    .map_err(|error| PlaybackError::Seek(error.to_string()))?;
            }
            self.player = Some(player);
            self.source_signals.push_back(signal);
            self.current_index = Some(candidate);
            self.queue[candidate].status = QueueItemStatus::Playing;
            self.queue[candidate].error = None;
            self.snapshot.status = PlaybackStatus::Playing;
            self.snapshot.position_ms = bounded_position;
            self.restored_position_ms = None;
            self.snapshot.error = None;
            self.sync_snapshot_queue();
            self.preload_next()?;
            return Ok(self.snapshot.clone());
        }
    }

    fn ensure_output(&mut self) -> Result<(), PlaybackError> {
        self.output.ensure_open()?;
        self.sync_output_snapshot();
        Ok(())
    }

    fn sync_output_snapshot(&mut self) {
        self.snapshot.output = self.output.snapshot();
    }

    fn capture_output_recovery(&self) -> Option<OutputRecovery> {
        let index = self.current_index?;
        if !matches!(
            self.snapshot.status,
            PlaybackStatus::Playing | PlaybackStatus::Paused
        ) {
            return None;
        }
        Some(OutputRecovery {
            index,
            position_ms: self
                .player
                .as_ref()
                .map(|player| duration_to_millis(player.get_pos()))
                .unwrap_or(self.snapshot.position_ms),
            paused: self.snapshot.status == PlaybackStatus::Paused,
        })
    }

    fn restore_output_recovery(&mut self) -> Result<(), PlaybackError> {
        let Some(recovery) = self.recovery.take() else {
            return Ok(());
        };
        if recovery.index >= self.queue.len() {
            return Ok(());
        }
        self.start_from_index(recovery.index)?;
        if recovery.position_ms > 0 {
            let player = self.player.as_ref().ok_or(PlaybackError::NothingLoaded)?;
            player
                .try_seek(Duration::from_millis(recovery.position_ms))
                .map_err(|error| PlaybackError::Seek(error.to_string()))?;
            self.snapshot.position_ms = recovery.position_ms;
        }
        if recovery.paused {
            self.player
                .as_ref()
                .ok_or(PlaybackError::NothingLoaded)?
                .pause();
            self.snapshot.status = PlaybackStatus::Paused;
            if let Some(index) = self.current_index {
                self.queue[index].status = QueueItemStatus::Paused;
            }
        }
        self.sync_snapshot_queue();
        Ok(())
    }

    fn suspend_output(&mut self, error: PlaybackError) {
        self.recovery = self.capture_output_recovery();
        self.stop_player();
        self.output.mark_unavailable(&error);
        self.snapshot.status = if self.recovery.is_some() {
            PlaybackStatus::Failed
        } else {
            self.snapshot.status
        };
        self.snapshot.error = Some(error.failure());
        self.sync_output_snapshot();
        self.sync_snapshot_queue();
    }

    fn refresh_output_lifecycle(&mut self) {
        if let Some(error) = self.output.take_stream_error() {
            self.suspend_output(PlaybackError::OutputDeviceUnavailable(error));
            return;
        }
        if self.last_device_poll.elapsed() < DEVICE_POLL_INTERVAL {
            return;
        }
        self.last_device_poll = Instant::now();

        if self.recovery.is_some() {
            if self
                .output
                .selected_available()
                .is_ok_and(|available| available)
                && self.output.ensure_open().is_ok()
            {
                let _ = self.restore_output_recovery();
                self.snapshot.error = None;
                self.sync_output_snapshot();
            }
            return;
        }

        if !self.output.has_output() {
            return;
        }
        match self.output.needs_reopen() {
            Ok(true) => {
                let recovery = self.capture_output_recovery();
                if self.output.reopen().is_ok() {
                    if let Some(recovery) = recovery {
                        self.stop_player();
                        self.recovery = Some(recovery);
                        let _ = self.restore_output_recovery();
                    }
                    self.snapshot.error = None;
                    self.sync_output_snapshot();
                }
            }
            Ok(false) => {}
            Err(error) => self.suspend_output(error),
        }
    }

    fn stop_player(&mut self) {
        if let Some(player) = self.player.take() {
            player.stop();
        }
        self.source_signals.clear();
    }

    fn next_valid_index(&self, start: usize) -> Option<usize> {
        (start..self.queue.len()).find(|&index| self.queue[index].status != QueueItemStatus::Failed)
    }

    fn preload_next(&mut self) -> Result<(), PlaybackError> {
        if self.current_index.is_none()
            || self.source_signals.len() >= 2
            || matches!(
                self.snapshot.playback_mode,
                PlaybackMode::RepeatOne | PlaybackMode::Shuffle
            )
        {
            return Ok(());
        }
        let last_loaded = self
            .source_signals
            .back()
            .map(|source| source.index)
            .or(self.current_index)
            .unwrap_or(0);
        let mut next = self.next_valid_index(last_loaded.saturating_add(1));
        if next.is_none() && self.snapshot.playback_mode == PlaybackMode::RepeatAll {
            next = self.next_valid_index(0);
        }
        while let Some(index) = next {
            if self.current_index == Some(index) {
                return Ok(());
            }
            let (decoder, duration_ms) = match decode_path(&self.queue[index].path) {
                Ok(value) => value,
                Err(error) => {
                    self.queue[index].status = QueueItemStatus::Failed;
                    self.queue[index].error = Some(error.failure());
                    next = self.next_valid_index(index.saturating_add(1));
                    if next.is_none() && self.snapshot.playback_mode == PlaybackMode::RepeatAll {
                        next = self.next_valid_index(0);
                    }
                    continue;
                }
            };
            self.queue[index].duration_ms = duration_ms;
            let player = self.player.as_ref().ok_or(PlaybackError::NothingLoaded)?;
            let signal = append_marked(player, decoder, index);
            self.source_signals.push_back(signal);
            return Ok(());
        }
        Ok(())
    }

    fn sync_current_snapshot(&mut self) {
        let Some(index) = self.current_index else {
            self.snapshot.path = None;
            self.snapshot.duration_ms = None;
            self.snapshot.seekable = false;
            self.snapshot.current_item_id = None;
            return;
        };
        let item = &self.queue[index];
        self.snapshot.path = Some(item.path.to_string_lossy().into_owned());
        self.snapshot.duration_ms = item.duration_ms;
        self.snapshot.seekable = item.duration_ms.is_some();
        self.snapshot.current_item_id = Some(item.id);
    }

    fn sync_snapshot_queue(&mut self) {
        self.snapshot.queue = self.queue.iter().map(QueueItem::snapshot).collect();
        self.sync_current_snapshot();
    }

    fn cancel_preloaded_sources(&mut self) {
        let mut retained = VecDeque::with_capacity(self.source_signals.len());
        if let Some(current) = self.source_signals.pop_front() {
            retained.push_back(current);
        }
        while let Some(source) = self.source_signals.pop_front() {
            if !source.cancel_if_queued() {
                retained.push_back(source);
            }
        }
        self.source_signals = retained;
    }

    fn rebuild_current_player(&mut self) -> Result<(), PlaybackError> {
        let Some(index) = self.current_index else {
            return Ok(());
        };
        let position_ms = self
            .player
            .as_ref()
            .map(|player| duration_to_millis(player.get_pos()))
            .unwrap_or(self.snapshot.position_ms);
        let paused = self.snapshot.status == PlaybackStatus::Paused;
        self.start_from_index(index)?;
        if position_ms > 0 {
            let player = self.player.as_ref().ok_or(PlaybackError::NothingLoaded)?;
            player
                .try_seek(Duration::from_millis(position_ms))
                .map_err(|error| PlaybackError::Seek(error.to_string()))?;
            self.snapshot.position_ms = position_ms;
        }
        if paused {
            self.player
                .as_ref()
                .ok_or(PlaybackError::NothingLoaded)?
                .pause();
            self.snapshot.status = PlaybackStatus::Paused;
            if let Some(index) = self.current_index {
                self.queue[index].status = QueueItemStatus::Paused;
            }
        }
        Ok(())
    }

    fn pause(&mut self) -> CommandResult {
        if self.snapshot.status != PlaybackStatus::Playing {
            return Err(PlaybackError::NothingLoaded);
        }
        let player = self.player.as_ref().ok_or(PlaybackError::NothingLoaded)?;
        player.pause();
        self.snapshot.status = PlaybackStatus::Paused;
        if let Some(index) = self.current_index {
            self.queue[index].status = QueueItemStatus::Paused;
        }
        self.snapshot.error = None;
        self.sync_snapshot_queue();
        Ok(self.snapshot.clone())
    }

    fn resume(&mut self) -> CommandResult {
        if self.snapshot.status != PlaybackStatus::Paused {
            return Err(PlaybackError::NothingLoaded);
        }
        let player = self.player.as_ref().ok_or(PlaybackError::NothingLoaded)?;
        player.play();
        self.snapshot.status = PlaybackStatus::Playing;
        if let Some(index) = self.current_index {
            self.queue[index].status = QueueItemStatus::Playing;
        }
        self.snapshot.error = None;
        self.sync_snapshot_queue();
        Ok(self.snapshot.clone())
    }

    fn stop(&mut self) -> CommandResult {
        if self.player.is_none() {
            return Err(PlaybackError::NothingLoaded);
        }
        self.stop_player();
        self.snapshot.status = PlaybackStatus::Stopped;
        self.snapshot.position_ms = 0;
        if let Some(index) = self.current_index {
            self.queue[index].status = QueueItemStatus::Pending;
        }
        self.snapshot.error = None;
        self.sync_snapshot_queue();
        Ok(self.snapshot.clone())
    }

    fn seek(&mut self, position_ms: u64) -> CommandResult {
        if !matches!(
            self.snapshot.status,
            PlaybackStatus::Playing | PlaybackStatus::Paused
        ) {
            return Err(PlaybackError::NothingLoaded);
        }
        let player = self.player.as_ref().ok_or(PlaybackError::NothingLoaded)?;
        let bounded_position = self
            .snapshot
            .duration_ms
            .map_or(position_ms, |duration| position_ms.min(duration));
        player
            .try_seek(Duration::from_millis(bounded_position))
            .map_err(|error| PlaybackError::Seek(error.to_string()))?;
        self.snapshot.position_ms = duration_to_millis(player.get_pos());
        self.snapshot.error = None;
        Ok(self.snapshot.clone())
    }

    fn set_volume(&mut self, volume: f32) -> CommandResult {
        if !volume.is_finite() || !(0.0..=1.0).contains(&volume) {
            return Err(PlaybackError::InvalidVolume);
        }
        if let Some(player) = &self.player {
            player.set_volume(volume);
        }
        self.snapshot.volume = volume;
        self.snapshot.error = None;
        Ok(self.snapshot.clone())
    }

    fn refresh_playback_state(&mut self) {
        if let Some(player) = &self.player {
            self.snapshot.position_ms = duration_to_millis(player.get_pos());
        }
        while self
            .source_signals
            .front()
            .is_some_and(|source| source.done.load(Ordering::Relaxed) == 0)
        {
            let Some(finished) = self.source_signals.pop_front() else {
                break;
            };
            self.finish_source(finished.index);
        }
        if self.snapshot.status == PlaybackStatus::Playing
            && self.source_signals.is_empty()
            && self.player.as_ref().is_some_and(Player::empty)
        {
            self.snapshot.position_ms = self
                .snapshot
                .duration_ms
                .unwrap_or(self.snapshot.position_ms);
            self.stop_player();
            self.current_index = None;
            self.snapshot.status = PlaybackStatus::Stopped;
            self.snapshot.error = None;
            self.sync_snapshot_queue();
        }
    }

    fn refresh_finished_playback(&mut self) {
        self.refresh_playback_state();
    }

    fn finish_source(&mut self, finished_index: usize) {
        if let Some(item) = self.queue.get_mut(finished_index) {
            item.status = QueueItemStatus::Played;
        }

        if self.current_index != Some(finished_index) {
            self.sync_snapshot_queue();
            return;
        }

        let queued_next = self.source_signals.front().map(|source| source.index);
        if let Some(next_index) = queued_next {
            self.current_index = Some(next_index);
            if let Some(item) = self.queue.get_mut(next_index) {
                item.status = QueueItemStatus::Playing;
            }
            self.snapshot.position_ms = 0;
            self.snapshot.status = PlaybackStatus::Playing;
            self.snapshot.error = None;
            self.sync_current_snapshot();
            let _ = self.preload_next();
        } else {
            let next = self.natural_next_index(finished_index);
            if let Some(next_index) = next {
                if self.append_index(next_index).is_ok() {
                    self.current_index = Some(next_index);
                    if let Some(item) = self.queue.get_mut(next_index) {
                        item.status = QueueItemStatus::Playing;
                    }
                    self.snapshot.position_ms = 0;
                    self.snapshot.status = PlaybackStatus::Playing;
                    self.snapshot.error = None;
                    self.sync_current_snapshot();
                    let _ = self.preload_next();
                } else {
                    self.snapshot.status = PlaybackStatus::Failed;
                    self.current_index = None;
                }
            } else {
                self.snapshot.position_ms = self
                    .snapshot
                    .duration_ms
                    .unwrap_or(self.snapshot.position_ms);
                self.stop_player();
                self.current_index = None;
                self.snapshot.status = PlaybackStatus::Stopped;
                self.snapshot.error = None;
            }
        }
        self.sync_snapshot_queue();
    }

    fn append_index(&mut self, index: usize) -> Result<(), PlaybackError> {
        let (decoder, duration_ms) = decode_path(&self.queue[index].path)?;
        self.queue[index].duration_ms = duration_ms;
        let player = self.player.as_ref().ok_or(PlaybackError::NothingLoaded)?;
        let signal = append_marked(player, decoder, index);
        self.source_signals.push_back(signal);
        Ok(())
    }

    fn manual_next_index(&self, current: usize) -> Option<usize> {
        match self.snapshot.playback_mode {
            PlaybackMode::Shuffle => self.random_valid_index(current),
            PlaybackMode::RepeatAll => self
                .next_valid_index(current.saturating_add(1))
                .or_else(|| self.next_valid_index(0)),
            PlaybackMode::Sequential | PlaybackMode::RepeatOne => {
                self.next_valid_index(current.saturating_add(1))
            }
        }
    }

    fn natural_next_index(&self, current: usize) -> Option<usize> {
        match self.snapshot.playback_mode {
            PlaybackMode::RepeatOne => Some(current),
            PlaybackMode::RepeatAll => self
                .next_valid_index(current.saturating_add(1))
                .or_else(|| self.next_valid_index(0)),
            PlaybackMode::Shuffle => self.random_valid_index(current),
            PlaybackMode::Sequential => self.next_valid_index(current.saturating_add(1)),
        }
    }

    fn random_valid_index(&self, current: usize) -> Option<usize> {
        let valid = self
            .queue
            .iter()
            .enumerate()
            .filter(|(index, item)| *index != current && item.status != QueueItemStatus::Failed)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if valid.is_empty() {
            self.queue
                .get(current)
                .is_some_and(|item| item.status != QueueItemStatus::Failed)
                .then_some(current)
        } else {
            valid.get(fastrand::usize(..valid.len())).copied()
        }
    }
}

fn duration_to_millis(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

fn decode_path(path: &Path) -> Result<(Decoder<BufReader<File>>, Option<u64>), PlaybackError> {
    validate_audio_path(path)?;
    let file = File::open(path).map_err(|error| PlaybackError::OpenFile(error.to_string()))?;
    let decoder =
        Decoder::try_from(file).map_err(|error| PlaybackError::Decode(error.to_string()))?;
    let duration_ms = decoder.total_duration().map(duration_to_millis);
    Ok((decoder, duration_ms))
}

fn append_marked<S>(player: &Player, source: S, index: usize) -> QueuedSourceSignal
where
    S: Source + Send + 'static,
{
    let done = Arc::new(AtomicUsize::new(1));
    let state = Arc::new(AtomicUsize::new(SOURCE_QUEUED));
    player.append(Done::new(
        CancelableSource {
            input: source,
            state: Arc::clone(&state),
        },
        Arc::clone(&done),
    ));
    QueuedSourceSignal { index, done, state }
}

fn validate_audio_path(path: &Path) -> Result<(), PlaybackError> {
    if !path.is_file() {
        return Err(PlaybackError::InvalidFile(
            path.to_string_lossy().into_owned(),
        ));
    }

    let supported = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "mp3" | "wav" | "flac"
            )
        });

    if supported {
        Ok(())
    } else {
        Err(PlaybackError::UnsupportedFormat)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    use rodio::source::Zero;

    use super::*;

    #[test]
    fn queued_source_can_be_cancelled_without_starting_it() {
        let state = Arc::new(AtomicUsize::new(SOURCE_QUEUED));
        let mut source = CancelableSource {
            input: Zero::new_samples(
                ChannelCount::new(2).expect("channel count"),
                SampleRate::new(44_100).expect("sample rate"),
                8,
            ),
            state: Arc::clone(&state),
        };
        let control = QueuedSourceSignal {
            index: 1,
            done: Arc::new(AtomicUsize::new(1)),
            state,
        };

        assert!(control.cancel_if_queued());
        assert_eq!(source.next(), None);
    }

    #[test]
    fn started_source_cannot_be_cancelled_as_preload() {
        let state = Arc::new(AtomicUsize::new(SOURCE_QUEUED));
        let mut source = CancelableSource {
            input: Zero::new_samples(
                ChannelCount::new(2).expect("channel count"),
                SampleRate::new(44_100).expect("sample rate"),
                8,
            ),
            state: Arc::clone(&state),
        };
        let control = QueuedSourceSignal {
            index: 1,
            done: Arc::new(AtomicUsize::new(1)),
            state,
        };

        assert_eq!(source.next(), Some(0.0));
        assert!(!control.cancel_if_queued());
        assert_eq!(source.next(), Some(0.0));
    }

    #[test]
    fn validates_supported_extensions_case_insensitively() {
        for extension in ["WAV", "FLAC", "MP3"] {
            let path = unique_test_path(extension);
            File::create(&path).expect("create supported fixture");
            assert!(validate_audio_path(&path).is_ok());
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn rejects_unsupported_extensions() {
        let path = unique_test_path("txt");
        File::create(&path).expect("create unsupported fixture");
        assert!(matches!(
            validate_audio_path(&path),
            Err(PlaybackError::UnsupportedFormat)
        ));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn decodes_a_pcm_wav_fixture_without_an_audio_device() {
        let path = create_test_wav("wav");
        let file = File::open(&path).expect("open WAV fixture");
        let decoder = Decoder::try_from(file).expect("decode WAV fixture");
        assert!(decoder.take(32).count() > 0);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn decodes_the_generated_format_matrix() {
        let fixture_directory = fixture_directory();
        let mut count = 0;

        for entry in std::fs::read_dir(&fixture_directory).expect("read fixture directory") {
            let path = entry.expect("read fixture entry").path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let Some(rate) = name
                .split('_')
                .nth(1)
                .and_then(|value| value.parse::<u32>().ok())
            else {
                continue;
            };
            if name.starts_with("seek_")
                || (name.starts_with("flac_") && name.contains("_32_"))
                || !matches!(
                    path.extension().and_then(|extension| extension.to_str()),
                    Some("wav" | "flac" | "mp3")
                )
            {
                continue;
            }

            let file = File::open(&path).expect("open generated fixture");
            let mut decoder = Decoder::try_from(file)
                .unwrap_or_else(|error| panic!("decode generated fixture {name}: {error}"));
            assert_eq!(decoder.sample_rate().get(), rate, "fixture {name}");
            assert!(decoder.total_duration().is_some(), "fixture {name}");
            assert!(decoder.by_ref().take(64).count() > 0, "fixture {name}");
            count += 1;
        }

        assert_eq!(
            count, 31,
            "supported matrix and mono fixture must be covered"
        );
    }

    #[test]
    fn rejects_flac_32_bit_fixtures_as_unsupported() {
        for name in [
            "flac_44100_32_stereo.flac",
            "flac_48000_32_stereo.flac",
            "flac_96000_32_stereo.flac",
            "flac_192000_32_stereo.flac",
        ] {
            let file =
                File::open(fixture_directory().join(name)).expect("open 32-bit FLAC fixture");
            assert!(Decoder::try_from(file).is_err(), "fixture {name}");
        }
    }

    #[test]
    fn rejects_empty_and_truncated_fixtures() {
        for name in ["empty.wav", "truncated.wav"] {
            let file = File::open(fixture_directory().join(name)).expect("open invalid fixture");
            assert!(Decoder::try_from(file).is_err(), "fixture {name}");
        }
    }

    #[test]
    fn decodes_content_when_a_supported_extension_is_mislabeled() {
        let file = File::open(fixture_directory().join("wav_content_as_flac.flac"))
            .expect("open mislabeled fixture");
        let mut decoder = Decoder::try_from(file).expect("decode mislabeled fixture");
        assert_eq!(decoder.sample_rate().get(), 44_100);
        assert!(decoder.by_ref().take(32).count() > 0);
    }

    #[test]
    fn seeks_wav_flac_and_mp3_sources() {
        for name in [
            "seek_48000_24_stereo.flac",
            "wav_44100_16_stereo.wav",
            "mp3_44100_cbr128_stereo.mp3",
        ] {
            let file = File::open(fixture_directory().join(name)).expect("open seek fixture");
            let mut decoder = Decoder::try_from(file).expect("decode seek fixture");
            decoder
                .try_seek(Duration::from_millis(100))
                .expect("seek fixture");
            assert!(decoder.take(32).count() > 0, "fixture {name}");
        }
    }

    #[test]
    fn volume_can_be_set_without_opening_an_audio_device() {
        let engine = RodioPlaybackEngine::new();
        let snapshot = engine.set_volume(0.25).expect("set volume");
        assert_eq!(snapshot.volume, 0.25);
        assert_eq!(snapshot.status, PlaybackStatus::Idle);
        assert!(matches!(
            engine.set_volume(f32::NAN),
            Err(PlaybackError::InvalidVolume)
        ));
    }

    #[test]
    fn queue_operations_preserve_ids_across_append_move_and_remove() {
        let mut engine = AudioActor::new();
        let first = PathBuf::from("first.wav");
        let second = PathBuf::from("second.flac");
        let third = PathBuf::from("third.mp3");
        let inserted = PathBuf::from("inserted.flac");

        let replaced = engine
            .replace_queue(vec![first.clone(), second.clone()])
            .expect("replace queue");
        assert_eq!(replaced.queue.len(), 2);
        assert_eq!(replaced.queue[0].id, 1);
        assert_eq!(replaced.queue[1].id, 2);
        assert_eq!(replaced.queue[0].display_name, "first.wav");

        let appended = engine.append_queue(vec![third]).expect("append queue");
        assert_eq!(
            appended
                .queue
                .iter()
                .map(|item| item.display_name.as_str())
                .collect::<Vec<_>>(),
            ["first.wav", "second.flac", "third.mp3"]
        );

        let inserted = engine
            .insert_queue(vec![inserted], 1)
            .expect("insert queue item");
        assert_eq!(
            inserted
                .queue
                .iter()
                .map(|item| item.id)
                .collect::<Vec<_>>(),
            [1, 4, 2, 3]
        );

        let moved = engine.move_queue_item(3, 1).expect("move queue item");
        assert_eq!(
            moved.queue.iter().map(|item| item.id).collect::<Vec<_>>(),
            [1, 3, 4, 2]
        );

        let removed = engine.remove_queue_item(1).expect("remove queue item");
        assert_eq!(
            removed.queue.iter().map(|item| item.id).collect::<Vec<_>>(),
            [3, 4, 2]
        );
    }

    #[test]
    fn next_without_a_target_preserves_the_active_playback_state() {
        let mut engine = AudioActor::new();
        engine
            .replace_queue(vec![PathBuf::from("only-track.flac")])
            .expect("replace single-item queue");
        engine.current_index = Some(0);
        engine.snapshot.status = PlaybackStatus::Playing;
        engine.queue[0].status = QueueItemStatus::Playing;

        let (response, receiver) = mpsc::channel();
        engine.handle(AudioCommand::Next(response));

        assert!(matches!(
            receiver.recv().expect("receive next result"),
            Err(PlaybackError::NothingLoaded)
        ));
        assert_eq!(engine.snapshot.status, PlaybackStatus::Playing);
        assert_eq!(engine.queue[0].status, QueueItemStatus::Playing);
        assert_eq!(
            engine.snapshot.error.as_ref().map(|failure| failure.code),
            Some(PlaybackErrorCode::NothingLoaded)
        );
    }

    #[test]
    fn playback_modes_round_trip_through_storage_keys() {
        for (mode, key) in [
            (PlaybackMode::Sequential, "sequential"),
            (PlaybackMode::RepeatOne, "repeat_one"),
            (PlaybackMode::RepeatAll, "repeat_all"),
            (PlaybackMode::Shuffle, "shuffle"),
        ] {
            assert_eq!(mode.storage_key(), key);
            assert_eq!(PlaybackMode::from_storage_key(key), mode);
        }
        assert_eq!(
            PlaybackMode::from_storage_key("future-mode"),
            PlaybackMode::Sequential
        );
    }

    #[test]
    fn restores_valid_session_without_opening_output_or_playing() {
        let valid = fixture_directory().join("wav_44100_16_stereo.wav");
        let engine = RodioPlaybackEngine::new();
        let snapshot = engine
            .restore_session(RestoredPlaybackSession {
                paths: vec![PathBuf::from("missing.wav"), valid.clone()],
                current_path: Some(valid),
                position_ms: 120,
                volume: 0.4,
                playback_mode: PlaybackMode::RepeatAll,
            })
            .expect("restore session");
        assert_eq!(snapshot.status, PlaybackStatus::Stopped);
        assert_eq!(snapshot.output.status, output::OutputStatus::Closed);
        assert_eq!(snapshot.queue.len(), 1);
        assert_eq!(snapshot.position_ms, 120);
        assert_eq!(snapshot.volume, 0.4);
        assert_eq!(snapshot.playback_mode, PlaybackMode::RepeatAll);
    }

    #[test]
    #[ignore = "requires a system audio output device"]
    fn opens_default_output_and_accepts_a_flac() {
        let path = fixture_directory().join("seek_48000_24_stereo.flac");
        let engine = RodioPlaybackEngine::new();
        let result = engine
            .replace_queue_and_play(vec![path], 0)
            .expect("play FLAC fixture");
        assert_eq!(result.status, PlaybackStatus::Playing);
        assert_eq!(result.duration_ms, Some(4000));
        let stopped = engine.stop().expect("stop FLAC fixture");
        assert_eq!(stopped.status, PlaybackStatus::Stopped);
        assert_eq!(stopped.position_ms, 0);
        assert!(matches!(engine.resume(), Err(PlaybackError::NothingLoaded)));
    }

    #[test]
    #[ignore = "requires a system audio output device"]
    fn enumerates_and_selects_the_system_default_output() {
        let engine = RodioPlaybackEngine::new();
        let refreshed = engine
            .refresh_output_devices()
            .expect("refresh output devices");
        assert!(!refreshed.output.devices.is_empty());
        assert!(refreshed.output.follow_system_default);

        let selected = engine
            .select_output_device(None)
            .expect("select system default output");
        assert_eq!(selected.output.status, output::OutputStatus::Ready);
        assert!(selected.output.active_device_id.is_some());
    }

    #[test]
    #[ignore = "requires a system audio output device"]
    fn opens_default_output_and_accepts_an_mp3() {
        let path = fixture_directory().join("mp3_44100_cbr128_stereo.mp3");
        let engine = RodioPlaybackEngine::new();
        let result = engine
            .replace_queue_and_play(vec![path], 0)
            .expect("play MP3 fixture");
        assert_eq!(result.status, PlaybackStatus::Playing);
        assert!(result.duration_ms.is_some());
        engine.stop().expect("stop MP3 fixture");
    }

    #[test]
    #[ignore = "requires a system audio output device"]
    fn opens_default_output_and_accepts_a_192_khz_32_bit_wav() {
        let path = fixture_directory().join("wav_192000_32_stereo.wav");
        let engine = RodioPlaybackEngine::new();
        let result = engine
            .replace_queue_and_play(vec![path], 0)
            .expect("play 192 kHz/32-bit WAV fixture");
        assert_eq!(result.status, PlaybackStatus::Playing);
        engine.stop().expect("stop 192 kHz/32-bit WAV fixture");
    }

    #[test]
    #[ignore = "requires a system audio output device"]
    fn recovers_after_a_32_bit_flac_decode_failure() {
        let engine = RodioPlaybackEngine::new();
        let unsupported = fixture_directory().join("flac_48000_32_stereo.flac");
        assert!(matches!(
            engine.replace_queue_and_play(vec![unsupported], 0),
            Err(PlaybackError::Decode(_))
        ));

        let failed = engine.snapshot().expect("read failed snapshot");
        assert_eq!(failed.status, PlaybackStatus::Failed);
        assert_eq!(
            failed.error.map(|failure| failure.code),
            Some(PlaybackErrorCode::Decode)
        );

        let supported = fixture_directory().join("flac_48000_24_stereo.flac");
        let playing = engine
            .replace_queue_and_play(vec![supported], 0)
            .expect("recover with supported FLAC");
        assert_eq!(playing.status, PlaybackStatus::Playing);
        assert_eq!(playing.error, None);
        engine.stop().expect("stop recovered playback");
    }

    #[test]
    #[ignore = "requires a system audio output device"]
    fn preloads_and_advances_a_lossless_queue() {
        let engine = RodioPlaybackEngine::new();
        let first = fixture_directory().join("seek_48000_24_stereo.flac");
        let second = fixture_directory().join("wav_44100_16_stereo.wav");
        let playing = engine
            .replace_queue_and_play(vec![first, second], 0)
            .expect("replace queue and play");
        assert_eq!(playing.status, PlaybackStatus::Playing);
        assert_eq!(playing.queue[1].status, QueueItemStatus::Pending);

        std::thread::sleep(Duration::from_millis(4_200));
        let advanced = engine.snapshot().expect("read advanced queue state");
        assert!(matches!(
            advanced.status,
            PlaybackStatus::Playing | PlaybackStatus::Stopped
        ));
        assert_eq!(advanced.queue[0].status, QueueItemStatus::Played);
        engine.stop().ok();
    }

    #[test]
    #[ignore = "requires a system audio output device"]
    fn changing_playback_mode_keeps_the_current_source_running() {
        let engine = RodioPlaybackEngine::new();
        let first = fixture_directory().join("seek_48000_24_stereo.flac");
        let second = fixture_directory().join("wav_44100_16_stereo.wav");
        let playing = engine
            .replace_queue_and_play(vec![first, second], 0)
            .expect("replace queue and play");
        let current_id = playing.current_item_id;

        std::thread::sleep(Duration::from_millis(250));
        let before = engine.snapshot().expect("snapshot before mode change");
        let changed = engine
            .set_playback_mode(PlaybackMode::Shuffle)
            .expect("change playback mode");
        std::thread::sleep(Duration::from_millis(150));
        let after = engine.snapshot().expect("snapshot after mode change");

        assert_eq!(changed.current_item_id, current_id);
        assert_eq!(after.current_item_id, current_id);
        assert_eq!(after.status, PlaybackStatus::Playing);
        assert!(changed.position_ms >= before.position_ms);
        assert!(after.position_ms >= changed.position_ms);
        engine.stop().ok();
    }

    fn fixture_directory() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("tests")
            .join("fixtures")
            .join("audio")
    }

    fn create_test_wav(extension: &str) -> PathBuf {
        let path = unique_test_path(extension);
        let samples = [0_i16, 1000, -1000, 0];
        let data_len = (samples.len() * std::mem::size_of::<i16>()) as u32;
        let mut file = File::create(&path).expect("create WAV fixture");

        file.write_all(b"RIFF").unwrap();
        file.write_all(&(36 + data_len).to_le_bytes()).unwrap();
        file.write_all(b"WAVEfmt ").unwrap();
        file.write_all(&16_u32.to_le_bytes()).unwrap();
        file.write_all(&1_u16.to_le_bytes()).unwrap();
        file.write_all(&1_u16.to_le_bytes()).unwrap();
        file.write_all(&44_100_u32.to_le_bytes()).unwrap();
        file.write_all(&(44_100_u32 * 2).to_le_bytes()).unwrap();
        file.write_all(&2_u16.to_le_bytes()).unwrap();
        file.write_all(&16_u16.to_le_bytes()).unwrap();
        file.write_all(b"data").unwrap();
        file.write_all(&data_len.to_le_bytes()).unwrap();
        for sample in samples {
            file.write_all(&sample.to_le_bytes()).unwrap();
        }
        file.flush().unwrap();
        path
    }

    fn unique_test_path(extension: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("resona-{nonce}.{extension}"))
    }
}
