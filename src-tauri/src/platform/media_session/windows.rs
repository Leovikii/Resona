// SPDX-License-Identifier: GPL-3.0-only

use std::path::Path;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
    SeekDirection,
};

use crate::playback::{PlaybackEngine, PlaybackStatus, RodioPlaybackEngine};

const STATE_POLL_INTERVAL: Duration = Duration::from_millis(500);
const SEEK_STEP: Duration = Duration::from_secs(10);

enum MediaSessionCommand {
    Event(MediaControlEvent),
    Shutdown,
}

pub struct MediaSessionAdapter {
    commands: Sender<MediaSessionCommand>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl MediaSessionAdapter {
    pub fn start(hwnd: isize, engine: Arc<RodioPlaybackEngine>) -> Result<Self, String> {
        let (commands, receiver) = mpsc::channel();
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let callback_sender = commands.clone();
        let worker = thread::Builder::new()
            .name("resona-smtc".to_owned())
            .spawn(move || run_session(hwnd, engine, receiver, ready_sender, callback_sender))
            .map_err(|error| format!("failed to start SMTC worker: {error}"))?;

        match ready_receiver.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => Ok(Self {
                commands,
                worker: Mutex::new(Some(worker)),
            }),
            Ok(Err(error)) => {
                let _ = worker.join();
                Err(error)
            }
            Err(error) => {
                let _ = commands.send(MediaSessionCommand::Shutdown);
                let _ = worker.join();
                Err(format!("SMTC worker did not initialize: {error}"))
            }
        }
    }
}

impl Drop for MediaSessionAdapter {
    fn drop(&mut self) {
        let _ = self.commands.send(MediaSessionCommand::Shutdown);
        if let Ok(mut worker) = self.worker.lock() {
            if let Some(worker) = worker.take() {
                let _ = worker.join();
            }
        }
    }
}

fn run_session(
    hwnd: isize,
    engine: Arc<RodioPlaybackEngine>,
    receiver: Receiver<MediaSessionCommand>,
    ready_sender: SyncSender<Result<(), String>>,
    callback_sender: Sender<MediaSessionCommand>,
) {
    let config = PlatformConfig {
        dbus_name: "io.github.vki.resona",
        display_name: "Resona",
        hwnd: Some(hwnd as *mut std::ffi::c_void),
    };
    let mut controls = match MediaControls::new(config) {
        Ok(controls) => controls,
        Err(error) => {
            let _ = ready_sender.send(Err(format!("failed to initialize Windows SMTC: {error}")));
            return;
        }
    };

    let callback_sender_for_attach = callback_sender.clone();
    if let Err(error) = controls.attach(move |event| {
        let _ = callback_sender_for_attach.send(MediaSessionCommand::Event(event));
    }) {
        let _ = ready_sender.send(Err(format!(
            "failed to attach Windows SMTC events: {error}"
        )));
        return;
    }

    if ready_sender.send(Ok(())).is_err() {
        return;
    }

    let mut last_sync = Instant::now() - STATE_POLL_INTERVAL;
    let mut last_metadata: Option<(String, Option<u64>)> = None;
    loop {
        let timeout = STATE_POLL_INTERVAL.saturating_sub(last_sync.elapsed());
        match receiver.recv_timeout(timeout) {
            Ok(MediaSessionCommand::Event(event)) => {
                handle_event(&engine, event);
                sync_controls(&engine, &mut controls, &mut last_metadata);
                last_sync = Instant::now();
            }
            Ok(MediaSessionCommand::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {
                sync_controls(&engine, &mut controls, &mut last_metadata);
                last_sync = Instant::now();
            }
        }
    }
}

fn handle_event(engine: &RodioPlaybackEngine, event: MediaControlEvent) {
    let result = match event {
        MediaControlEvent::Play | MediaControlEvent::Toggle => toggle_playback(engine),
        MediaControlEvent::Pause => engine.pause(),
        MediaControlEvent::Stop => engine.stop(),
        MediaControlEvent::Next => engine.next(),
        MediaControlEvent::Previous => previous_track(engine),
        MediaControlEvent::Seek(direction) => seek_by(engine, direction, SEEK_STEP),
        MediaControlEvent::SeekBy(direction, amount) => seek_by(engine, direction, amount),
        MediaControlEvent::SetPosition(MediaPosition(position)) => {
            engine.seek(position.as_millis().min(u128::from(u64::MAX)) as u64)
        }
        MediaControlEvent::Raise | MediaControlEvent::Quit | MediaControlEvent::OpenUri(_) => {
            return;
        }
        MediaControlEvent::SetVolume(_) => return,
    };

    if let Err(error) = result {
        eprintln!("SMTC command ignored: {}", error.failure().message);
    }
}

fn toggle_playback(
    engine: &RodioPlaybackEngine,
) -> Result<crate::playback::PlaybackSnapshot, crate::playback::PlaybackError> {
    let snapshot = engine.snapshot()?;
    match snapshot.status {
        PlaybackStatus::Playing => engine.pause(),
        PlaybackStatus::Paused => engine.resume(),
        PlaybackStatus::Stopped => snapshot
            .current_item_id
            .map_or(Err(crate::playback::PlaybackError::NothingLoaded), |id| {
                engine.play_queue_item(id)
            }),
        PlaybackStatus::Idle | PlaybackStatus::Failed => {
            Err(crate::playback::PlaybackError::NothingLoaded)
        }
    }
}

fn previous_track(
    engine: &RodioPlaybackEngine,
) -> Result<crate::playback::PlaybackSnapshot, crate::playback::PlaybackError> {
    engine.previous()
}

fn seek_by(
    engine: &RodioPlaybackEngine,
    direction: SeekDirection,
    amount: Duration,
) -> Result<crate::playback::PlaybackSnapshot, crate::playback::PlaybackError> {
    let snapshot = engine.snapshot()?;
    let amount_ms = amount.as_millis().min(u128::from(u64::MAX)) as u64;
    let position_ms = match direction {
        SeekDirection::Forward => snapshot.position_ms.saturating_add(amount_ms),
        SeekDirection::Backward => snapshot.position_ms.saturating_sub(amount_ms),
    };
    engine.seek(position_ms)
}

fn sync_controls(
    engine: &RodioPlaybackEngine,
    controls: &mut MediaControls,
    last_metadata: &mut Option<(String, Option<u64>)>,
) {
    let Ok(snapshot) = engine.snapshot() else {
        return;
    };

    let title = snapshot
        .path
        .as_deref()
        .and_then(|path| Path::new(path).file_name())
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_owned();
    let metadata_key = (title.clone(), snapshot.duration_ms);
    if last_metadata.as_ref() != Some(&metadata_key) {
        let _ = controls.set_metadata(MediaMetadata {
            title: (!title.is_empty()).then_some(title.as_str()),
            duration: snapshot.duration_ms.map(Duration::from_millis),
            ..MediaMetadata::default()
        });
        *last_metadata = Some(metadata_key);
    }

    let playback = match snapshot.status {
        PlaybackStatus::Playing => MediaPlayback::Playing {
            progress: Some(MediaPosition(Duration::from_millis(snapshot.position_ms))),
        },
        PlaybackStatus::Paused => MediaPlayback::Paused {
            progress: Some(MediaPosition(Duration::from_millis(snapshot.position_ms))),
        },
        PlaybackStatus::Idle | PlaybackStatus::Stopped | PlaybackStatus::Failed => {
            MediaPlayback::Stopped
        }
    };
    let _ = controls.set_playback(playback);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires a system audio output device"]
    fn media_control_commands_drive_the_playback_engine() {
        let engine = RodioPlaybackEngine::new();
        let fixture_directory =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/audio");
        let first = fixture_directory.join("flac_44100_16_stereo.flac");
        let second = fixture_directory.join("mp3_44100_cbr128_stereo.mp3");

        let playing = engine
            .replace_queue_and_play(vec![first.clone(), second], 0)
            .expect("start playback");
        assert_eq!(playing.status, PlaybackStatus::Playing);

        handle_event(&engine, MediaControlEvent::Pause);
        assert_eq!(
            engine.snapshot().expect("snapshot").status,
            PlaybackStatus::Paused
        );

        handle_event(&engine, MediaControlEvent::Play);
        assert_eq!(
            engine.snapshot().expect("snapshot").status,
            PlaybackStatus::Playing
        );

        handle_event(&engine, MediaControlEvent::Next);
        assert_eq!(
            engine.snapshot().expect("snapshot").status,
            PlaybackStatus::Playing
        );

        handle_event(&engine, MediaControlEvent::Previous);
        let previous = engine.snapshot().expect("previous snapshot");
        assert_eq!(previous.status, PlaybackStatus::Playing);
        assert_eq!(
            previous.path.as_deref(),
            Some(first.to_string_lossy().as_ref())
        );

        handle_event(&engine, MediaControlEvent::Previous);
        let first_again = engine.snapshot().expect("first snapshot");
        assert_eq!(first_again.status, PlaybackStatus::Playing);
        assert_eq!(
            first_again.path.as_deref(),
            Some(first.to_string_lossy().as_ref())
        );

        handle_event(&engine, MediaControlEvent::Stop);
        assert_eq!(
            engine.snapshot().expect("snapshot").status,
            PlaybackStatus::Stopped
        );
    }
}
