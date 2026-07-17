// SPDX-License-Identifier: GPL-3.0-only

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};
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
        };
        PlaybackFailure {
            code,
            message: self.to_string(),
        }
    }
}

pub trait PlaybackEngine {
    fn play(&self, path: PathBuf) -> Result<PlaybackSnapshot, PlaybackError>;
    fn pause(&self) -> Result<PlaybackSnapshot, PlaybackError>;
    fn resume(&self) -> Result<PlaybackSnapshot, PlaybackError>;
    fn stop(&self) -> Result<PlaybackSnapshot, PlaybackError>;
    fn seek(&self, position_ms: u64) -> Result<PlaybackSnapshot, PlaybackError>;
    fn set_volume(&self, volume: f32) -> Result<PlaybackSnapshot, PlaybackError>;
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
    Seek {
        position_ms: u64,
        response: ResponseSender,
    },
    SetVolume {
        volume: f32,
        response: ResponseSender,
    },
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
        let (result, response, replace_state_on_error) = match command {
            AudioCommand::Play { path, response } => (self.play(path), response, true),
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
                (Ok(self.snapshot.clone()), response, false)
            }
            AudioCommand::Shutdown => return,
        };

        if let Err(error) = &result {
            if replace_state_on_error {
                self.snapshot.status = PlaybackStatus::Failed;
            }
            self.snapshot.error = Some(error.failure());
        }
        let _ = response.send(result);
    }

    fn play(&mut self, path: PathBuf) -> CommandResult {
        if let Some(player) = self.player.take() {
            player.stop();
        }
        self.snapshot = PlaybackSnapshot {
            path: Some(path.to_string_lossy().into_owned()),
            volume: self.snapshot.volume,
            ..PlaybackSnapshot::default()
        };

        validate_audio_path(&path)?;

        let file = File::open(&path).map_err(|error| PlaybackError::OpenFile(error.to_string()))?;
        let decoder =
            Decoder::try_from(file).map_err(|error| PlaybackError::Decode(error.to_string()))?;
        let duration_ms = decoder.total_duration().map(duration_to_millis);

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
        player.set_volume(self.snapshot.volume);
        player.play();

        self.player = Some(player);
        self.snapshot = PlaybackSnapshot {
            status: PlaybackStatus::Playing,
            path: Some(path.to_string_lossy().into_owned()),
            error: None,
            position_ms: 0,
            duration_ms,
            volume: self.snapshot.volume,
            seekable: duration_ms.is_some(),
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
        self.snapshot.status = PlaybackStatus::Stopped;
        self.snapshot.position_ms = 0;
        self.snapshot.error = None;
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
        if self.snapshot.status == PlaybackStatus::Playing
            && self.player.as_ref().is_some_and(Player::empty)
        {
            self.snapshot.position_ms = self
                .snapshot
                .duration_ms
                .unwrap_or(self.snapshot.position_ms);
            self.player = None;
            self.snapshot.status = PlaybackStatus::Stopped;
            self.snapshot.error = None;
        }
    }

    fn refresh_finished_playback(&mut self) {
        self.refresh_playback_state();
    }
}

fn duration_to_millis(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
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

    use super::*;

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
    #[ignore = "Rodio 0.22.2's FLAC adapters cannot emit samples from valid 32-bit FLAC"]
    fn decodes_flac_32_bit_fixtures_when_upstream_supports_them() {
        for name in [
            "flac_44100_32_stereo.flac",
            "flac_48000_32_stereo.flac",
            "flac_96000_32_stereo.flac",
            "flac_192000_32_stereo.flac",
        ] {
            let file =
                File::open(fixture_directory().join(name)).expect("open 32-bit FLAC fixture");
            let decoder = Decoder::try_from(file).expect("decode 32-bit FLAC fixture");
            assert!(decoder.take(64).count() > 0, "fixture {name}");
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
    #[ignore = "requires a system audio output device"]
    fn opens_default_output_and_accepts_a_flac() {
        let path = fixture_directory().join("seek_48000_24_stereo.flac");
        let engine = RodioPlaybackEngine::new();
        let result = engine.play(path).expect("play FLAC fixture");
        assert_eq!(result.status, PlaybackStatus::Playing);
        assert_eq!(result.duration_ms, Some(4000));
        let stopped = engine.stop().expect("stop FLAC fixture");
        assert_eq!(stopped.status, PlaybackStatus::Stopped);
        assert_eq!(stopped.position_ms, 0);
        assert!(matches!(engine.resume(), Err(PlaybackError::NothingLoaded)));
    }

    #[test]
    #[ignore = "requires a system audio output device"]
    fn opens_default_output_and_accepts_an_mp3() {
        let path = fixture_directory().join("mp3_44100_cbr128_stereo.mp3");
        let engine = RodioPlaybackEngine::new();
        let result = engine.play(path).expect("play MP3 fixture");
        assert_eq!(result.status, PlaybackStatus::Playing);
        assert!(result.duration_ms.is_some());
        engine.stop().expect("stop MP3 fixture");
    }

    #[test]
    #[ignore = "requires a system audio output device"]
    fn opens_default_output_and_accepts_a_192_khz_32_bit_wav() {
        let path = fixture_directory().join("wav_192000_32_stereo.wav");
        let engine = RodioPlaybackEngine::new();
        let result = engine.play(path).expect("play 192 kHz/32-bit WAV fixture");
        assert_eq!(result.status, PlaybackStatus::Playing);
        engine.stop().expect("stop 192 kHz/32-bit WAV fixture");
    }

    #[test]
    #[ignore = "requires a system audio output device"]
    fn recovers_after_a_32_bit_flac_decode_failure() {
        let engine = RodioPlaybackEngine::new();
        let unsupported = fixture_directory().join("flac_48000_32_stereo.flac");
        assert!(matches!(
            engine.play(unsupported),
            Err(PlaybackError::Decode(_))
        ));

        let failed = engine.snapshot().expect("read failed snapshot");
        assert_eq!(failed.status, PlaybackStatus::Failed);
        assert_eq!(
            failed.error.map(|failure| failure.code),
            Some(PlaybackErrorCode::Decode)
        );

        let supported = fixture_directory().join("flac_48000_24_stereo.flac");
        let playing = engine.play(supported).expect("recover with supported FLAC");
        assert_eq!(playing.status, PlaybackStatus::Playing);
        assert_eq!(playing.error, None);
        engine.stop().expect("stop recovered playback");
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
