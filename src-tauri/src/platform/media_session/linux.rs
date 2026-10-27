// SPDX-License-Identifier: GPL-3.0-only

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::platform::playback_projection::NativePlaybackSnapshot;
use crate::playback::{
    PlaybackEngine, PlaybackMode, PlaybackSnapshot, PlaybackStatus, RodioPlaybackEngine,
};
use mpris_server::{LoopStatus, Metadata, PlaybackStatus as Status, Player, Time, TrackId};

enum Command {
    Play,
    Pause,
    Toggle,
    Stop,
    Next,
    Previous,
    Seek(i64),
    Position(TrackId, i64),
    Volume(f64),
    Loop(LoopStatus),
    Shuffle(bool),
    Raise,
    Quit,
    Shutdown,
}

pub struct MediaSessionAdapter {
    commands: Sender<Command>,
    worker: Mutex<Option<JoinHandle<()>>>,
}
impl MediaSessionAdapter {
    pub fn start(
        app: tauri::AppHandle,
        engine: Arc<RodioPlaybackEngine>,
        placeholder: &Path,
        cache: PathBuf,
        projection: Receiver<NativePlaybackSnapshot>,
    ) -> Result<Self, String> {
        let (commands, receiver) = mpsc::channel();
        let events = commands.clone();
        let placeholder = file_url(placeholder)?;
        let worker = thread::Builder::new()
            .name("resona-mpris".into())
            .spawn(move || {
                let context = gtk::glib::MainContext::new();
                let result = context.with_thread_default(|| {
                    context.block_on(async {
                        let build = Player::builder("io.github.vki.resona")
                            .identity("Resona")
                            .desktop_entry("io.github.vki.resona")
                            .can_raise(true)
                            .can_quit(true)
                            .build();
                        let player = match futures_util::future::select(
                            Box::pin(build),
                            Box::pin(gtk::glib::timeout_future(Duration::from_secs(4))),
                        )
                        .await
                        {
                            futures_util::future::Either::Left((result, _)) => result?,
                            futures_util::future::Either::Right(_) => {
                                log::warn!("MPRIS session bus connection timed out");
                                return Ok::<_, mpris_server::zbus::Error>(());
                            }
                        };
                        connect(&player, &events);
                        let task = context.spawn_local(player.run());
                        log::info!("MPRIS registered: org.mpris.MediaPlayer2.io.github.vki.resona");
                        if let Ok(entries) = std::fs::read_dir(&cache) {
                            for entry in entries.flatten() {
                                if entry
                                    .path()
                                    .extension()
                                    .is_some_and(|extension| extension == "png")
                                {
                                    if let Err(error) = std::fs::remove_file(entry.path()) {
                                        log::warn!("MPRIS stale artwork cleanup: {error}");
                                    }
                                }
                            }
                        }
                        let mut last_metadata = None;
                        let mut artwork_files = VecDeque::new();
                        let mut timeline: Option<(PlaybackSnapshot, Instant)> = None;
                        loop {
                            gtk::glib::timeout_future(Duration::from_millis(100)).await;
                            let mut shutdown = false;
                            for command in receiver.try_iter() {
                                if matches!(command, Command::Shutdown) {
                                    shutdown = true;
                                    break;
                                }
                                match command {
                                    Command::Raise => {
                                        if let Err(error) =
                                            crate::application_lifetime::show_main_window(&app)
                                        {
                                            log::warn!("MPRIS raise failed: {}", error.message);
                                        }
                                    }
                                    Command::Quit => crate::request_application_exit(&app),
                                    command => {
                                        let seeked = matches!(
                                            command,
                                            Command::Seek(_) | Command::Position(..)
                                        );
                                        match handle_command(&engine, command) {
                                            Ok(snapshot) => {
                                                player.set_position(time(snapshot.position_ms));
                                                if seeked {
                                                    player
                                                        .seeked(time(snapshot.position_ms))
                                                        .await?;
                                                }
                                            }
                                            Err(error) => log::warn!(
                                                "MPRIS command failed: {}",
                                                error.failure().message
                                            ),
                                        }
                                    }
                                }
                            }
                            if shutdown {
                                break;
                            }
                            if let Some(snapshot) = projection.try_iter().last() {
                                let now = Instant::now();
                                let discontinuity =
                                    timeline.as_ref().is_some_and(|(previous, at)| {
                                        timeline_discontinuity(
                                            previous,
                                            &snapshot.playback,
                                            now.duration_since(*at).as_millis() as u64,
                                        )
                                    });
                                sync(
                                    &player,
                                    &snapshot,
                                    &placeholder,
                                    &cache,
                                    &mut artwork_files,
                                    &mut last_metadata,
                                )
                                .await?;
                                if discontinuity {
                                    player.seeked(time(snapshot.playback.position_ms)).await?;
                                }
                                timeline = Some((snapshot.playback, now));
                            }
                        }
                        task.abort();
                        for file in artwork_files {
                            if let Err(error) = std::fs::remove_file(file) {
                                log::warn!("MPRIS artwork cleanup: {error}");
                            }
                        }
                        Ok(())
                    })
                });
                match result {
                    Ok(Ok(())) => {}
                    other => log::warn!("MPRIS worker stopped: {other:?}"),
                }
            })
            .map_err(|error| error.to_string())?;
        Ok(Self {
            commands,
            worker: Mutex::new(Some(worker)),
        })
    }
}
impl Drop for MediaSessionAdapter {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Shutdown);
        if let Some(worker) = self.worker.lock().unwrap().take() {
            let _ = worker.join();
        }
    }
}
fn connect(player: &Player, events: &Sender<Command>) {
    macro_rules! connect {
        ($method:ident, $command:expr) => {{
            let events = events.clone();
            player.$method(move |_| {
                let _ = events.send($command);
            });
        }};
    }
    connect!(connect_play, Command::Play);
    connect!(connect_pause, Command::Pause);
    connect!(connect_play_pause, Command::Toggle);
    connect!(connect_stop, Command::Stop);
    connect!(connect_next, Command::Next);
    connect!(connect_previous, Command::Previous);
    connect!(connect_raise, Command::Raise);
    connect!(connect_quit, Command::Quit);
    let tx = events.clone();
    player.connect_seek(move |_, offset| {
        let _ = tx.send(Command::Seek(offset.as_millis()));
    });
    let tx = events.clone();
    player.connect_set_position(move |_, id, position| {
        let _ = tx.send(Command::Position(id.clone(), position.as_millis()));
    });
    let tx = events.clone();
    player.connect_set_loop_status(move |_, status| {
        let _ = tx.send(Command::Loop(status));
    });
    let tx = events.clone();
    player.connect_set_shuffle(move |_, shuffle| {
        let _ = tx.send(Command::Shuffle(shuffle));
    });
    let tx = events.clone();
    player.connect_set_volume(move |_, volume| {
        let _ = tx.send(Command::Volume(volume));
    });
}
fn handle_command(
    engine: &RodioPlaybackEngine,
    command: Command,
) -> Result<PlaybackSnapshot, crate::playback::PlaybackError> {
    let snapshot = engine.snapshot()?;
    match command {
        Command::Play | Command::Toggle if snapshot.status != PlaybackStatus::Playing => {
            match snapshot.status {
                PlaybackStatus::Paused => engine.resume(),
                _ => snapshot
                    .current_item_id
                    .or_else(|| snapshot.queue.first().map(|item| item.id))
                    .map_or(Ok(snapshot), |id| engine.play_queue_item(id)),
            }
        }
        Command::Toggle => engine.pause(),
        Command::Pause if snapshot.status == PlaybackStatus::Playing => engine.pause(),
        Command::Stop if snapshot.path.is_some() => engine.stop(),
        Command::Next => engine.next(),
        Command::Previous => engine.previous(),
        Command::Seek(offset) if snapshot.seekable => {
            engine.seek(seek_target(snapshot.position_ms, offset))
        }
        Command::Position(id, position)
            if snapshot.seekable && valid_position(&snapshot, &id, position) =>
        {
            engine.seek(position as u64)
        }
        Command::Loop(status) => engine.set_playback_mode(match status {
            LoopStatus::Track => PlaybackMode::RepeatOne,
            LoopStatus::Playlist => PlaybackMode::RepeatAll,
            LoopStatus::None => PlaybackMode::Sequential,
        }),
        Command::Shuffle(shuffle) => engine.set_playback_mode(if shuffle {
            PlaybackMode::Shuffle
        } else {
            PlaybackMode::Sequential
        }),
        Command::Volume(volume) if volume.is_finite() => {
            engine.set_volume(volume.clamp(0.0, 1.0) as f32)
        }
        _ => Ok(snapshot),
    }
}
fn timeline_discontinuity(
    previous: &PlaybackSnapshot,
    current: &PlaybackSnapshot,
    elapsed_ms: u64,
) -> bool {
    if previous.current_item_id.is_none() || previous.current_item_id != current.current_item_id {
        return false;
    }
    let expected =
        previous
            .position_ms
            .saturating_add(if previous.status == PlaybackStatus::Playing {
                elapsed_ms
            } else {
                0
            });
    current.position_ms.abs_diff(expected)
        > if previous.status == PlaybackStatus::Playing {
            750
        } else {
            0
        }
}

fn seek_target(current: u64, offset: i64) -> u64 {
    if offset < 0 {
        current.saturating_sub(offset.unsigned_abs())
    } else {
        current.saturating_add(offset as u64)
    }
}
fn track_id(snapshot: &PlaybackSnapshot) -> TrackId {
    TrackId::try_from(format!(
        "/io/github/vki/resona/track/{}",
        snapshot.current_item_id.unwrap_or(0)
    ))
    .expect("numeric track ID is a valid object path")
}
fn valid_position(snapshot: &PlaybackSnapshot, id: &TrackId, position: i64) -> bool {
    *id == track_id(snapshot)
        && position >= 0
        && snapshot
            .duration_ms
            .is_none_or(|duration| position as u64 <= duration)
}
fn time(milliseconds: u64) -> Time {
    Time::from_micros(milliseconds.saturating_mul(1000).min(i64::MAX as u64) as i64)
}

#[derive(PartialEq)]
struct MetadataKey {
    id: Option<u64>,
    path: Option<String>,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    duration: Option<u64>,
    artwork: Option<[u8; 32]>,
}
async fn sync(
    player: &Player,
    projection: &NativePlaybackSnapshot,
    placeholder: &str,
    cache: &Path,
    files: &mut VecDeque<PathBuf>,
    last: &mut Option<MetadataKey>,
) -> mpris_server::zbus::Result<()> {
    let snapshot = &projection.playback;
    let key = MetadataKey {
        id: snapshot.current_item_id,
        path: snapshot.path.clone(),
        title: projection.title.clone(),
        artist: projection.artist.clone(),
        album: projection.album.clone(),
        duration: snapshot.duration_ms,
        artwork: projection.artwork.as_ref().map(|art| art.fingerprint),
    };
    if last.as_ref() != Some(&key) {
        let mut metadata = Metadata::new();
        if snapshot.path.is_some() {
            metadata.set_trackid(Some(track_id(snapshot)));
            let title = projection.title.clone().or_else(|| {
                snapshot
                    .queue
                    .iter()
                    .find(|item| Some(item.id) == snapshot.current_item_id)
                    .map(|item| item.display_name.clone())
            });
            metadata.set_title(title);
            metadata.set_artist(projection.artist.as_ref().map(|artist| [artist.as_str()]));
            metadata.set_album(projection.album.clone());
            metadata.set_length(snapshot.duration_ms.map(time));
            metadata.set_url(
                snapshot
                    .path
                    .as_deref()
                    .and_then(|path| file_url(Path::new(path)).ok()),
            );
            let artwork = projection
                .artwork
                .as_ref()
                .and_then(|art| match materialize_artwork(cache, art, files) {
                    Ok(url) => Some(url),
                    Err(error) => {
                        log::warn!("MPRIS artwork: {error}");
                        None
                    }
                })
                .unwrap_or_else(|| placeholder.to_owned());
            metadata.set_art_url(Some(artwork));
        }
        player.set_metadata(metadata).await?;
        *last = Some(key);
    }
    player
        .set_playback_status(match snapshot.status {
            PlaybackStatus::Playing => Status::Playing,
            PlaybackStatus::Paused => Status::Paused,
            _ => Status::Stopped,
        })
        .await?;
    player.set_position(time(snapshot.position_ms));
    player.set_volume(f64::from(snapshot.volume)).await?;
    player
        .set_loop_status(match snapshot.playback_mode {
            PlaybackMode::RepeatOne => LoopStatus::Track,
            PlaybackMode::RepeatAll => LoopStatus::Playlist,
            _ => LoopStatus::None,
        })
        .await?;
    player
        .set_shuffle(snapshot.playback_mode == PlaybackMode::Shuffle)
        .await?;
    player.set_can_play(!snapshot.queue.is_empty()).await?;
    player
        .set_can_pause(matches!(
            snapshot.status,
            PlaybackStatus::Playing | PlaybackStatus::Paused
        ))
        .await?;
    player.set_can_seek(snapshot.seekable).await?;
    player.set_can_go_next(snapshot.queue.len() > 1).await?;
    player
        .set_can_go_previous(!snapshot.queue.is_empty())
        .await?;
    Ok(())
}
fn file_url(path: &Path) -> Result<String, String> {
    tauri::Url::from_file_path(std::fs::canonicalize(path).map_err(|e| e.to_string())?)
        .map(String::from)
        .map_err(|_| "invalid local media URL".into())
}
fn materialize_artwork(
    cache: &Path,
    artwork: &crate::metadata::Artwork,
    files: &mut VecDeque<PathBuf>,
) -> Result<String, String> {
    std::fs::create_dir_all(cache).map_err(|error| error.to_string())?;
    let path = cache.join(format!("{}.png", artwork.fingerprint_hex()));
    if !path.is_file() {
        std::fs::write(&path, artwork.encoded.as_ref()).map_err(|error| error.to_string())?;
    }
    files.retain(|file| file != &path);
    files.push_back(path.clone());
    while files.len() > 16 {
        if let Some(file) = files.pop_front() {
            std::fs::remove_file(file).map_err(|error| error.to_string())?;
        }
    }
    file_url(&path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_and_out_of_range_positions_are_ignored() {
        let snapshot = PlaybackSnapshot {
            current_item_id: Some(7),
            duration_ms: Some(1000),
            ..Default::default()
        };
        assert!(valid_position(&snapshot, &track_id(&snapshot), 500));
        assert!(!valid_position(&snapshot, &track_id(&snapshot), -1));
        assert!(!valid_position(&snapshot, &track_id(&snapshot), 1001));
        assert!(!valid_position(
            &snapshot,
            &TrackId::try_from("/old/track").unwrap(),
            500
        ));
        assert_eq!(seek_target(10, i64::MIN), 0);
        assert_eq!(seek_target(u64::MAX, 10), u64::MAX);
    }
    #[test]
    fn timeline_only_signals_discontinuities_within_the_same_track() {
        let previous = PlaybackSnapshot {
            current_item_id: Some(7),
            status: PlaybackStatus::Playing,
            position_ms: 1000,
            ..Default::default()
        };
        let mut current = PlaybackSnapshot {
            position_ms: 1500,
            ..previous.clone()
        };
        assert!(!timeline_discontinuity(&previous, &current, 500));
        current.position_ms = 10000;
        assert!(timeline_discontinuity(&previous, &current, 500));
        current.current_item_id = Some(8);
        assert!(!timeline_discontinuity(&previous, &current, 500));
    }
    #[test]
    fn volume_uses_the_authoritative_engine_without_an_audio_device() {
        let engine = RodioPlaybackEngine::new();
        handle_command(&engine, Command::Volume(0.25)).unwrap();
        assert_eq!(engine.snapshot().unwrap().volume, 0.25);
        handle_command(&engine, Command::Volume(f64::NAN)).unwrap();
        assert_eq!(engine.snapshot().unwrap().volume, 0.25);
    }
    #[test]
    fn loop_and_shuffle_update_the_authoritative_mode() {
        let engine = RodioPlaybackEngine::new();
        assert_eq!(
            handle_command(&engine, Command::Loop(LoopStatus::Track))
                .unwrap()
                .playback_mode,
            PlaybackMode::RepeatOne
        );
        assert_eq!(
            handle_command(&engine, Command::Shuffle(true))
                .unwrap()
                .playback_mode,
            PlaybackMode::Shuffle
        );
    }
    #[test]
    fn local_urls_escape_unicode_spaces_and_fragments() {
        let root = std::env::temp_dir().join(format!("resona-mpris-{}", fastrand::u64(..)));
        std::fs::create_dir(&root).unwrap();
        let file = root.join("封面 #1.png");
        std::fs::write(&file, b"cover").unwrap();
        let url = file_url(&file).unwrap();
        assert!(url.contains("%20%231"));
        assert_eq!(
            tauri::Url::parse(&url).unwrap().to_file_path().unwrap(),
            file
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
