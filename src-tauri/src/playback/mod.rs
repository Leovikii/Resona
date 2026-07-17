// SPDX-License-Identifier: GPL-3.0-only

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};
use serde::Serialize;
use thiserror::Error;

const COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
const ACTOR_TICK: Duration = Duration::from_millis(150);

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

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct PlaybackSnapshot {
    pub status: PlaybackStatus,
    pub path: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Error)]
pub enum PlaybackError {
    #[error("只支持 WAV 或 FLAC 文件")]
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
    #[error("播放引擎不可用")]
    EngineUnavailable,
    #[error("播放引擎响应超时")]
    EngineTimeout,
}

pub trait PlaybackEngine {
    fn play(&self, path: PathBuf) -> Result<PlaybackSnapshot, PlaybackError>;
    fn pause(&self) -> Result<PlaybackSnapshot, PlaybackError>;
    fn resume(&self) -> Result<PlaybackSnapshot, PlaybackError>;
    fn stop(&self) -> Result<PlaybackSnapshot, PlaybackError>;
    fn snapshot(&self) -> Result<PlaybackSnapshot, PlaybackError>;
}

type CommandResult = Result<PlaybackSnapshot, PlaybackError>;
type ResponseSender = Sender<CommandResult>;

enum AudioCommand {
    Play {
        path: PathBuf,
        response: ResponseSender,
    },
    Pause(ResponseSender),
    Resume(ResponseSender),
    Stop(ResponseSender),
    Snapshot(ResponseSender),
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
    fn play(&self, path: PathBuf) -> CommandResult {
        self.request(|response| AudioCommand::Play { path, response })
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

    fn snapshot(&self) -> CommandResult {
        self.request(AudioCommand::Snapshot)
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
    output: Option<MixerDeviceSink>,
    player: Option<Player>,
    snapshot: PlaybackSnapshot,
}

impl AudioActor {
    fn new() -> Self {
        Self {
            output: None,
            player: None,
            snapshot: PlaybackSnapshot::default(),
        }
    }

    fn run(mut self, receiver: Receiver<AudioCommand>) {
        loop {
            match receiver.recv_timeout(ACTOR_TICK) {
                Ok(AudioCommand::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
                Ok(command) => self.handle(command),
                Err(RecvTimeoutError::Timeout) => self.refresh_finished_playback(),
            }
        }

        if let Some(player) = &self.player {
            player.stop();
        }
    }

    fn handle(&mut self, command: AudioCommand) {
        let (result, response) = match command {
            AudioCommand::Play { path, response } => (self.play(path), response),
            AudioCommand::Pause(response) => (self.pause(), response),
            AudioCommand::Resume(response) => (self.resume(), response),
            AudioCommand::Stop(response) => (self.stop(), response),
            AudioCommand::Snapshot(response) => (Ok(self.snapshot.clone()), response),
            AudioCommand::Shutdown => return,
        };

        if let Err(error) = &result {
            self.snapshot.status = PlaybackStatus::Failed;
            self.snapshot.error = Some(error.to_string());
        }
        let _ = response.send(result);
    }

    fn play(&mut self, path: PathBuf) -> CommandResult {
        if let Some(player) = self.player.take() {
            player.stop();
        }
        self.snapshot = PlaybackSnapshot::default();

        validate_audio_path(&path)?;

        let file = File::open(&path).map_err(|error| PlaybackError::OpenFile(error.to_string()))?;
        let decoder =
            Decoder::try_from(file).map_err(|error| PlaybackError::Decode(error.to_string()))?;

        if self.output.is_none() {
            let mut output = DeviceSinkBuilder::open_default_sink()
                .map_err(|error| PlaybackError::OpenOutput(error.to_string()))?;
            output.log_on_drop(false);
            self.output = Some(output);
        }

        let output = self
            .output
            .as_ref()
            .ok_or(PlaybackError::EngineUnavailable)?;
        let player = Player::connect_new(output.mixer());
        player.append(decoder);
        player.play();

        self.player = Some(player);
        self.snapshot = PlaybackSnapshot {
            status: PlaybackStatus::Playing,
            path: Some(path.to_string_lossy().into_owned()),
            error: None,
        };
        Ok(self.snapshot.clone())
    }

    fn pause(&mut self) -> CommandResult {
        if self.snapshot.status != PlaybackStatus::Playing {
            return Err(PlaybackError::NothingLoaded);
        }
        let player = self.player.as_ref().ok_or(PlaybackError::NothingLoaded)?;
        player.pause();
        self.snapshot.status = PlaybackStatus::Paused;
        self.snapshot.error = None;
        Ok(self.snapshot.clone())
    }

    fn resume(&mut self) -> CommandResult {
        if self.snapshot.status != PlaybackStatus::Paused {
            return Err(PlaybackError::NothingLoaded);
        }
        let player = self.player.as_ref().ok_or(PlaybackError::NothingLoaded)?;
        player.play();
        self.snapshot.status = PlaybackStatus::Playing;
        self.snapshot.error = None;
        Ok(self.snapshot.clone())
    }

    fn stop(&mut self) -> CommandResult {
        let player = self.player.take().ok_or(PlaybackError::NothingLoaded)?;
        player.stop();
        self.snapshot = PlaybackSnapshot {
            status: PlaybackStatus::Stopped,
            path: None,
            error: None,
        };
        Ok(self.snapshot.clone())
    }

    fn refresh_finished_playback(&mut self) {
        if self.snapshot.status == PlaybackStatus::Playing
            && self.player.as_ref().is_some_and(Player::empty)
        {
            self.player = None;
            self.snapshot = PlaybackSnapshot {
                status: PlaybackStatus::Stopped,
                path: None,
                error: None,
            };
        }
    }
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
        .is_some_and(|extension| matches!(extension.to_ascii_lowercase().as_str(), "wav" | "flac"));

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

    use super::*;

    #[test]
    fn validates_supported_extensions_case_insensitively() {
        let wav = create_test_wav("WAV");
        assert!(validate_audio_path(&wav).is_ok());
        let _ = std::fs::remove_file(wav);
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
    #[ignore = "requires a system audio output device"]
    fn opens_default_output_and_accepts_a_wav() {
        let path = create_test_wav("wav");
        let engine = RodioPlaybackEngine::new();
        let result = engine.play(path.clone()).expect("play WAV fixture");
        assert_eq!(result.status, PlaybackStatus::Playing);
        let stopped = engine.stop().expect("stop WAV fixture");
        assert_eq!(stopped.status, PlaybackStatus::Stopped);
        assert_eq!(stopped.path, None);
        assert!(matches!(engine.resume(), Err(PlaybackError::NothingLoaded)));
        let _ = std::fs::remove_file(path);
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
