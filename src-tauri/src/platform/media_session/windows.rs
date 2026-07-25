// SPDX-License-Identifier: GPL-3.0-only

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use sha2::{Digest, Sha256};
use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
    SeekDirection,
};
use windows::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED};

use crate::metadata::Artwork;
use crate::platform::playback_projection::NativePlaybackSnapshot;
use crate::playback::{PlaybackEngine, PlaybackStatus, RodioPlaybackEngine};

const COMMAND_POLL_INTERVAL: Duration = Duration::from_millis(100);
const SEEK_STEP: Duration = Duration::from_secs(10);
const TIMELINE_SYNC_INTERVAL_MS: u64 = 5_000;

fn playback_sync_key(status: PlaybackStatus, position_ms: u64) -> (PlaybackStatus, u64) {
    let position_key = match status {
        PlaybackStatus::Playing => position_ms / TIMELINE_SYNC_INTERVAL_MS,
        PlaybackStatus::Paused => position_ms,
        PlaybackStatus::Idle | PlaybackStatus::Stopped | PlaybackStatus::Failed => 0,
    };
    (status, position_key)
}

enum MediaSessionCommand {
    Event(MediaControlEvent),
    Shutdown,
}

pub struct MediaSessionAdapter {
    commands: Sender<MediaSessionCommand>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

struct MediaSessionArtwork {
    app_logo_url: String,
    cache_dir: PathBuf,
}

impl MediaSessionAdapter {
    pub fn start(
        hwnd: isize,
        engine: Arc<RodioPlaybackEngine>,
        app_logo_path: &Path,
        artwork_cache_dir: PathBuf,
        projection: Receiver<NativePlaybackSnapshot>,
    ) -> Result<Self, String> {
        let (commands, receiver) = mpsc::channel();
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let callback_sender = commands.clone();
        let artwork = MediaSessionArtwork {
            app_logo_url: file_url(app_logo_path)?,
            cache_dir: artwork_cache_dir,
        };
        let worker = thread::Builder::new()
            .name("resona-smtc".to_owned())
            .spawn(move || {
                run_session(
                    hwnd,
                    engine,
                    receiver,
                    ready_sender,
                    callback_sender,
                    artwork,
                    projection,
                )
            })
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
    artwork: MediaSessionArtwork,
    projection: Receiver<NativePlaybackSnapshot>,
) {
    clear_artwork_cache(&artwork.cache_dir);
    if let Err(error) = unsafe { RoInitialize(RO_INIT_MULTITHREADED) } {
        let _ = ready_sender.send(Err(format!(
            "failed to initialize Windows Runtime for SMTC: {error}"
        )));
        return;
    }
    let config = PlatformConfig {
        dbus_name: "io.github.vki.resona",
        display_name: "Resona",
        hwnd: Some(hwnd as *mut std::ffi::c_void),
    };
    let mut controls = match MediaControls::new(config) {
        Ok(controls) => controls,
        Err(error) => {
            let _ = ready_sender.send(Err(format!("failed to initialize Windows SMTC: {error}")));
            unsafe { RoUninitialize() };
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
        unsafe { RoUninitialize() };
        return;
    }

    if ready_sender.send(Ok(())).is_err() {
        unsafe { RoUninitialize() };
        return;
    }

    let mut last_metadata: Option<(String, String, Option<u64>, Option<usize>)> = None;
    let mut last_playback: Option<(PlaybackStatus, u64)> = None;
    loop {
        match receiver.recv_timeout(COMMAND_POLL_INTERVAL) {
            Ok(MediaSessionCommand::Event(event)) => {
                handle_event(&engine, event);
            }
            Ok(MediaSessionCommand::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }
        if let Some(projection) = projection.try_iter().last() {
            sync_controls(
                &projection,
                &mut controls,
                &mut last_metadata,
                &mut last_playback,
                &artwork.app_logo_url,
                &artwork.cache_dir,
            );
        }
    }
    drop(controls);
    clear_artwork_cache(&artwork.cache_dir);
    unsafe { RoUninitialize() };
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
    projection: &NativePlaybackSnapshot,
    controls: &mut MediaControls,
    last_metadata: &mut Option<(String, String, Option<u64>, Option<usize>)>,
    last_playback: &mut Option<(PlaybackStatus, u64)>,
    app_logo_url: &str,
    artwork_cache_dir: &Path,
) {
    let snapshot = &projection.playback;
    let title = current_track_title(snapshot);
    let metadata_key = (
        snapshot.path.clone().unwrap_or_default(),
        title.clone(),
        snapshot.duration_ms,
        projection
            .artwork
            .as_ref()
            .map(|artwork| Arc::as_ptr(artwork) as usize),
    );
    if last_metadata.as_ref() != Some(&metadata_key) {
        let cover_url = projection
            .artwork
            .as_ref()
            .and_then(
                |artwork| match materialize_artwork(artwork_cache_dir, artwork) {
                    Ok(url) => Some(url),
                    Err(error) => {
                        eprintln!("Windows SMTC artwork cache failed; using placeholder: {error}");
                        None
                    }
                },
            )
            .unwrap_or_else(|| app_logo_url.to_owned());
        let metadata = MediaMetadata {
            title: (!title.is_empty()).then_some(title.as_str()),
            cover_url: Some(cover_url.as_str()),
            duration: snapshot.duration_ms.map(Duration::from_millis),
            ..MediaMetadata::default()
        };
        let mut metadata_applied = false;
        match controls.set_metadata(metadata) {
            Ok(()) => metadata_applied = true,
            Err(cover_error) => {
                eprintln!("Windows SMTC artwork update failed: {cover_error}");
                if cover_url != app_logo_url {
                    match controls.set_metadata(MediaMetadata {
                        title: (!title.is_empty()).then_some(title.as_str()),
                        cover_url: Some(app_logo_url),
                        duration: snapshot.duration_ms.map(Duration::from_millis),
                        ..MediaMetadata::default()
                    }) {
                        Ok(()) => metadata_applied = true,
                        Err(error) => {
                            eprintln!("Windows SMTC placeholder update failed: {error}");
                        }
                    }
                }
                if !metadata_applied {
                    match controls.set_metadata(MediaMetadata {
                        title: (!title.is_empty()).then_some(title.as_str()),
                        duration: snapshot.duration_ms.map(Duration::from_millis),
                        ..MediaMetadata::default()
                    }) {
                        Ok(()) => metadata_applied = true,
                        Err(error) => {
                            eprintln!("Windows SMTC metadata update failed: {error}");
                        }
                    }
                }
            }
        }
        if metadata_applied {
            *last_metadata = Some(metadata_key);
            *last_playback = None;
        }
    }

    let playback_key = playback_sync_key(snapshot.status, snapshot.position_ms);
    if last_playback.as_ref() == Some(&playback_key) {
        return;
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
    if let Err(error) = controls.set_playback(playback) {
        eprintln!("Windows SMTC playback update failed: {error}");
    } else {
        *last_playback = Some(playback_key);
    }
}

fn materialize_artwork(cache_dir: &Path, artwork: &Artwork) -> Result<String, String> {
    std::fs::create_dir_all(cache_dir).map_err(|error| error.to_string())?;
    let digest = Sha256::digest(artwork.encoded.as_ref());
    let mut fingerprint = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut fingerprint, "{byte:02x}").expect("writing to a string cannot fail");
    }
    let path = cache_dir.join(format!("{fingerprint}.png"));
    if !path.is_file() {
        let temporary = cache_dir.join(format!(".{}.tmp", fastrand::u64(..)));
        std::fs::write(&temporary, artwork.encoded.as_ref()).map_err(|error| error.to_string())?;
        if let Err(error) = std::fs::rename(&temporary, &path) {
            let _ = std::fs::remove_file(&temporary);
            if !path.is_file() {
                return Err(error.to_string());
            }
        }
    }
    file_url(&path)
}

fn file_url(path: &Path) -> Result<String, String> {
    let absolute = std::fs::canonicalize(path).map_err(|error| error.to_string())?;
    let raw = absolute.to_string_lossy();
    let winrt_path = if let Some(path) = raw.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{path}")
    } else if let Some(path) = raw.strip_prefix(r"\\?\") {
        path.to_owned()
    } else {
        raw.into_owned()
    };
    Ok(format!("file://{winrt_path}"))
}

fn clear_artwork_cache(cache_dir: &Path) {
    let Ok(entries) = std::fs::read_dir(cache_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let _ = std::fs::remove_file(path);
    }
}

fn current_track_title(snapshot: &crate::playback::PlaybackSnapshot) -> String {
    snapshot
        .current_item_id
        .and_then(|id| snapshot.queue.iter().find(|item| item.id == id))
        .map(|item| item.display_name.clone())
        .or_else(|| {
            snapshot
                .path
                .as_deref()
                .and_then(|path| Path::new(path).file_name())
                .and_then(|name| name.to_str())
                .map(str::to_owned)
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_track_title_prefers_the_authoritative_queue_item() {
        let snapshot = crate::playback::PlaybackSnapshot {
            path: Some(r"C:\Music\fallback.flac".to_owned()),
            current_item_id: Some(7),
            queue: vec![crate::playback::QueueItemSnapshot {
                id: 7,
                path: r"C:\Music\fallback.flac".to_owned(),
                display_name: "媒体标题.flac".to_owned(),
                duration_ms: Some(1_000),
                status: crate::playback::QueueItemStatus::Playing,
                error: None,
            }],
            ..Default::default()
        };

        assert_eq!(current_track_title(&snapshot), "媒体标题.flac");
    }

    #[test]
    fn smtc_artwork_is_materialized_as_a_reusable_file_url() {
        let cache_dir =
            std::env::temp_dir().join(format!("resona-smtc-artwork-{}", fastrand::u64(..)));
        let artwork = Artwork {
            mime_type: "image/png".to_owned(),
            encoded: Arc::from(&b"encoded artwork"[..]),
            bgra: Arc::from(&b"\0\0\0\xff"[..]),
            width: 1,
            height: 1,
        };

        let first = materialize_artwork(&cache_dir, &artwork).expect("materialize artwork");
        let second = materialize_artwork(&cache_dir, &artwork).expect("reuse artwork");
        assert_eq!(first, second);
        assert!(first.ends_with(".png"));
        assert_eq!(
            std::fs::read(first.trim_start_matches("file://")).expect("read cached artwork"),
            artwork.encoded.as_ref()
        );

        clear_artwork_cache(&cache_dir);
        let _ = std::fs::remove_dir(cache_dir);
    }

    #[test]
    fn smtc_artwork_files_remain_available_until_adapter_shutdown() {
        let cache_dir =
            std::env::temp_dir().join(format!("resona-smtc-retain-{}", fastrand::u64(..)));
        let artwork = |encoded: &'static [u8]| Artwork {
            mime_type: "image/png".to_owned(),
            encoded: Arc::from(encoded),
            bgra: Arc::from(&b"\0\0\0\xff"[..]),
            width: 1,
            height: 1,
        };

        let first =
            materialize_artwork(&cache_dir, &artwork(b"first")).expect("materialize first artwork");
        let second = materialize_artwork(&cache_dir, &artwork(b"second"))
            .expect("materialize second artwork");

        assert_ne!(first, second);
        assert!(Path::new(first.trim_start_matches("file://")).is_file());
        assert!(Path::new(second.trim_start_matches("file://")).is_file());

        clear_artwork_cache(&cache_dir);
        assert!(!Path::new(first.trim_start_matches("file://")).exists());
        assert!(!Path::new(second.trim_start_matches("file://")).exists());
        let _ = std::fs::remove_dir(cache_dir);
    }

    #[test]
    fn smtc_file_urls_are_accepted_windows_paths() {
        let cache_dir =
            std::env::temp_dir().join(format!("resona-smtc-path-{}", fastrand::u64(..)));
        std::fs::create_dir_all(&cache_dir).expect("create cache directory");
        let file = cache_dir.join("cover.png");
        std::fs::write(&file, b"cover").expect("write cover");

        let url = file_url(&file).expect("create file URL");
        let windows_path = url.trim_start_matches("file://");
        assert!(!windows_path.starts_with(r"\\?\"));
        assert!(Path::new(windows_path).is_absolute());
        assert_eq!(
            std::fs::read(windows_path).expect("read URL path"),
            b"cover"
        );

        let _ = std::fs::remove_dir_all(cache_dir);
    }

    #[test]
    fn playing_timeline_is_throttled_but_state_and_paused_position_are_exact() {
        assert_eq!(
            playback_sync_key(PlaybackStatus::Playing, 5_100),
            playback_sync_key(PlaybackStatus::Playing, 9_900)
        );
        assert_ne!(
            playback_sync_key(PlaybackStatus::Playing, 9_900),
            playback_sync_key(PlaybackStatus::Playing, 10_000)
        );
        assert_ne!(
            playback_sync_key(PlaybackStatus::Playing, 5_100),
            playback_sync_key(PlaybackStatus::Paused, 5_100)
        );
        assert_ne!(
            playback_sync_key(PlaybackStatus::Paused, 5_100),
            playback_sync_key(PlaybackStatus::Paused, 5_101)
        );
    }

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
