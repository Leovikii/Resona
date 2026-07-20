// SPDX-License-Identifier: GPL-3.0-only

use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, State};

use crate::compression::{
    CompressionFailure, CompressionPreset, CompressionScanSnapshot, CompressionService,
    CompressionSnapshot,
};
use crate::compression_window;
use crate::lyrics::{LyricsService, LyricsSnapshot};
use crate::media_import::{DefaultPlaylistSnapshot, MediaImportService, OpenMediaResult};
use crate::metadata::{normalized_path, read_track_details, TrackDetails};
use crate::persistence::{
    PersistenceFailure, PersistenceService, PlaylistItemRecord, PlaylistSummary, RecentPlayRecord,
};
use crate::platform::desktop_lyrics::{
    DesktopLyricsWindowFailure, DesktopLyricsWindowService, DesktopLyricsWindowSnapshot,
};
use crate::playback::{
    PlaybackEngine, PlaybackFailure, PlaybackMode, PlaybackSnapshot, RodioPlaybackEngine,
};
use crate::playlists::PlaylistMutationResult;

type ManagedPlaybackEngine = Arc<RodioPlaybackEngine>;
type ManagedPersistence = Arc<PersistenceService>;
type ManagedMediaImportService = Arc<MediaImportService>;
type ManagedLyricsService = Arc<LyricsService>;
type ManagedDesktopLyricsWindowService = Arc<DesktopLyricsWindowService>;
type ManagedCompressionService = Arc<CompressionService>;

#[tauri::command]
pub async fn show_audio_compression_window(app: AppHandle) -> Result<(), CompressionFailure> {
    compression_window::show(&app)
}

#[tauri::command]
pub async fn scan_audio_compression_inputs(
    paths: Vec<String>,
    service: State<'_, ManagedCompressionService>,
) -> Result<CompressionScanSnapshot, CompressionFailure> {
    Arc::clone(service.inner()).add_scan_inputs(paths.into_iter().map(PathBuf::from).collect())
}

#[tauri::command]
pub async fn remove_audio_compression_inputs(
    paths: Vec<String>,
    service: State<'_, ManagedCompressionService>,
) -> Result<CompressionScanSnapshot, CompressionFailure> {
    Arc::clone(service.inner()).remove_scan_inputs(paths.into_iter().map(PathBuf::from).collect())
}

#[tauri::command]
pub async fn clear_audio_compression_inputs(
    service: State<'_, ManagedCompressionService>,
) -> Result<CompressionScanSnapshot, CompressionFailure> {
    service.clear_scan_inputs()
}

#[tauri::command]
pub async fn get_audio_compression_scan_state(
    service: State<'_, ManagedCompressionService>,
) -> Result<CompressionScanSnapshot, CompressionFailure> {
    Ok(service.scan_snapshot())
}

#[tauri::command]
pub async fn cancel_audio_compression_scan(
    service: State<'_, ManagedCompressionService>,
) -> Result<CompressionScanSnapshot, CompressionFailure> {
    Ok(service.cancel_scan())
}

#[tauri::command]
pub async fn start_audio_compression(
    paths: Vec<String>,
    preset: CompressionPreset,
    delete_source: bool,
    deletion_confirmed: bool,
    service: State<'_, ManagedCompressionService>,
) -> Result<CompressionSnapshot, CompressionFailure> {
    Arc::clone(service.inner()).start(
        paths.into_iter().map(PathBuf::from).collect(),
        preset,
        delete_source,
        deletion_confirmed,
    )
}

#[tauri::command]
pub async fn get_audio_compression_state(
    service: State<'_, ManagedCompressionService>,
) -> Result<CompressionSnapshot, CompressionFailure> {
    Ok(service.snapshot())
}

#[tauri::command]
pub async fn cancel_audio_compression(
    service: State<'_, ManagedCompressionService>,
) -> Result<CompressionSnapshot, CompressionFailure> {
    Ok(service.cancel())
}

#[tauri::command]
pub async fn get_track_details(path: String) -> TrackDetails {
    tauri::async_runtime::spawn_blocking(move || read_track_details(&normalized_path(path)))
        .await
        .unwrap_or_else(|error| {
            read_track_details(&PathBuf::from(format!("metadata-task-{error}")))
        })
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NowPlayingSnapshot {
    pub playback: PlaybackSnapshot,
    pub lyrics: LyricsSnapshot,
}

async fn run_engine_operation<F>(
    engine: ManagedPlaybackEngine,
    operation: F,
) -> Result<PlaybackSnapshot, PlaybackFailure>
where
    F: FnOnce(&RodioPlaybackEngine) -> Result<PlaybackSnapshot, crate::playback::PlaybackError>
        + Send
        + 'static,
{
    tauri::async_runtime::spawn_blocking(move || operation(&engine))
        .await
        .map_err(|error| PlaybackFailure::task_failed(format!("playback task failed: {error}")))?
        .map_err(|error| error.failure())
}

#[tauri::command]
pub async fn open_media_context(
    path: String,
    media_import: State<'_, ManagedMediaImportService>,
    persistence: State<'_, ManagedPersistence>,
) -> Result<OpenMediaResult, PlaybackFailure> {
    let media_import = Arc::clone(media_import.inner());
    let result = tauri::async_runtime::spawn_blocking(move || {
        media_import.open_media_context(std::path::Path::new(&path))
    })
    .await
    .map_err(|error| PlaybackFailure::task_failed(format!("playback task failed: {error}")))??;
    record_recent(persistence.inner(), &result.playback).await;
    Ok(result)
}

#[tauri::command]
pub async fn get_default_playlist(
    media_import: State<'_, ManagedMediaImportService>,
) -> Result<DefaultPlaylistSnapshot, PlaybackFailure> {
    let media_import = Arc::clone(media_import.inner());
    tauri::async_runtime::spawn_blocking(move || media_import.snapshot())
        .await
        .map_err(|error| {
            PlaybackFailure::task_failed(format!("default playlist task failed: {error}"))
        })?
}

#[tauri::command]
pub async fn replace_queue(
    paths: Vec<String>,
    engine: State<'_, ManagedPlaybackEngine>,
) -> Result<PlaybackSnapshot, PlaybackFailure> {
    let engine = Arc::clone(engine.inner());
    run_engine_operation(engine, move |engine| {
        engine.replace_queue(paths.into_iter().map(PathBuf::from).collect())
    })
    .await
}

#[tauri::command]
pub async fn replace_queue_and_play(
    paths: Vec<String>,
    selected_index: usize,
    engine: State<'_, ManagedPlaybackEngine>,
    persistence: State<'_, ManagedPersistence>,
) -> Result<PlaybackSnapshot, PlaybackFailure> {
    let engine = Arc::clone(engine.inner());
    let snapshot = run_engine_operation(engine, move |engine| {
        engine.replace_queue_and_play(
            paths.into_iter().map(PathBuf::from).collect(),
            selected_index,
        )
    })
    .await?;
    record_recent(persistence.inner(), &snapshot).await;
    Ok(snapshot)
}

#[tauri::command]
pub async fn append_to_queue(
    paths: Vec<String>,
    engine: State<'_, ManagedPlaybackEngine>,
) -> Result<PlaybackSnapshot, PlaybackFailure> {
    let engine = Arc::clone(engine.inner());
    run_engine_operation(engine, move |engine| {
        engine.append_queue(paths.into_iter().map(PathBuf::from).collect())
    })
    .await
}

#[tauri::command]
pub async fn play_queue_item(
    id: u64,
    engine: State<'_, ManagedPlaybackEngine>,
    persistence: State<'_, ManagedPersistence>,
) -> Result<PlaybackSnapshot, PlaybackFailure> {
    let engine = Arc::clone(engine.inner());
    let snapshot = run_engine_operation(engine, move |engine| engine.play_queue_item(id)).await?;
    record_recent(persistence.inner(), &snapshot).await;
    Ok(snapshot)
}

#[tauri::command]
pub async fn remove_queue_item(
    id: u64,
    engine: State<'_, ManagedPlaybackEngine>,
) -> Result<PlaybackSnapshot, PlaybackFailure> {
    let engine = Arc::clone(engine.inner());
    run_engine_operation(engine, move |engine| engine.remove_queue_item(id)).await
}

#[tauri::command]
pub async fn move_queue_item(
    id: u64,
    to_index: usize,
    engine: State<'_, ManagedPlaybackEngine>,
) -> Result<PlaybackSnapshot, PlaybackFailure> {
    let engine = Arc::clone(engine.inner());
    run_engine_operation(engine, move |engine| engine.move_queue_item(id, to_index)).await
}

#[tauri::command]
pub async fn next_playback(
    engine: State<'_, ManagedPlaybackEngine>,
    persistence: State<'_, ManagedPersistence>,
) -> Result<PlaybackSnapshot, PlaybackFailure> {
    let engine = Arc::clone(engine.inner());
    let snapshot = run_engine_operation(engine, PlaybackEngine::next).await?;
    record_recent(persistence.inner(), &snapshot).await;
    Ok(snapshot)
}

#[tauri::command]
pub async fn set_playback_mode(
    mode: PlaybackMode,
    engine: State<'_, ManagedPlaybackEngine>,
) -> Result<PlaybackSnapshot, PlaybackFailure> {
    let engine = Arc::clone(engine.inner());
    run_engine_operation(engine, move |engine| engine.set_playback_mode(mode)).await
}

#[tauri::command]
pub async fn previous_playback(
    engine: State<'_, ManagedPlaybackEngine>,
    persistence: State<'_, ManagedPersistence>,
) -> Result<PlaybackSnapshot, PlaybackFailure> {
    let engine = Arc::clone(engine.inner());
    let snapshot = run_engine_operation(engine, PlaybackEngine::previous).await?;
    record_recent(persistence.inner(), &snapshot).await;
    Ok(snapshot)
}

#[tauri::command]
pub async fn clear_queue(
    engine: State<'_, ManagedPlaybackEngine>,
) -> Result<PlaybackSnapshot, PlaybackFailure> {
    let engine = Arc::clone(engine.inner());
    run_engine_operation(engine, PlaybackEngine::clear_queue).await
}

#[tauri::command]
pub async fn refresh_output_devices(
    engine: State<'_, ManagedPlaybackEngine>,
) -> Result<PlaybackSnapshot, PlaybackFailure> {
    let engine = Arc::clone(engine.inner());
    run_engine_operation(engine, PlaybackEngine::refresh_output_devices).await
}

#[tauri::command]
pub async fn select_output_device(
    device_id: Option<String>,
    engine: State<'_, ManagedPlaybackEngine>,
) -> Result<PlaybackSnapshot, PlaybackFailure> {
    let engine = Arc::clone(engine.inner());
    run_engine_operation(engine, move |engine| engine.select_output_device(device_id)).await
}

#[tauri::command]
pub async fn pause_playback(
    engine: State<'_, ManagedPlaybackEngine>,
) -> Result<PlaybackSnapshot, PlaybackFailure> {
    let engine = Arc::clone(engine.inner());
    run_engine_operation(engine, PlaybackEngine::pause).await
}

#[tauri::command]
pub async fn resume_playback(
    engine: State<'_, ManagedPlaybackEngine>,
) -> Result<PlaybackSnapshot, PlaybackFailure> {
    let engine = Arc::clone(engine.inner());
    run_engine_operation(engine, PlaybackEngine::resume).await
}

#[tauri::command]
pub async fn stop_playback(
    engine: State<'_, ManagedPlaybackEngine>,
) -> Result<PlaybackSnapshot, PlaybackFailure> {
    let engine = Arc::clone(engine.inner());
    run_engine_operation(engine, PlaybackEngine::stop).await
}

#[tauri::command]
pub async fn seek_playback(
    position_ms: u64,
    engine: State<'_, ManagedPlaybackEngine>,
) -> Result<PlaybackSnapshot, PlaybackFailure> {
    let engine = Arc::clone(engine.inner());
    run_engine_operation(engine, move |engine| engine.seek(position_ms)).await
}

#[tauri::command]
pub async fn set_playback_volume(
    volume: f32,
    engine: State<'_, ManagedPlaybackEngine>,
) -> Result<PlaybackSnapshot, PlaybackFailure> {
    let engine = Arc::clone(engine.inner());
    run_engine_operation(engine, move |engine| engine.set_volume(volume)).await
}

#[tauri::command]
pub async fn get_playback_state(
    engine: State<'_, ManagedPlaybackEngine>,
) -> Result<PlaybackSnapshot, PlaybackFailure> {
    let engine = Arc::clone(engine.inner());
    run_engine_operation(engine, PlaybackEngine::snapshot).await
}

#[tauri::command]
pub async fn get_now_playing_state(
    known_lyrics_revision: Option<u64>,
    engine: State<'_, ManagedPlaybackEngine>,
    lyrics: State<'_, ManagedLyricsService>,
) -> Result<NowPlayingSnapshot, PlaybackFailure> {
    let engine = Arc::clone(engine.inner());
    let lyrics = Arc::clone(lyrics.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let playback = engine.snapshot().map_err(|error| error.failure())?;
        let lyrics = lyrics.snapshot(&playback, known_lyrics_revision);
        Ok(NowPlayingSnapshot { playback, lyrics })
    })
    .await
    .map_err(|error| PlaybackFailure::task_failed(format!("now playing task failed: {error}")))?
}

#[tauri::command]
pub async fn get_desktop_lyrics_window_state(
    service: State<'_, ManagedDesktopLyricsWindowService>,
) -> Result<DesktopLyricsWindowSnapshot, DesktopLyricsWindowFailure> {
    Ok(service.snapshot())
}

#[tauri::command]
pub async fn show_desktop_lyrics_window(
    app: AppHandle,
    service: State<'_, ManagedDesktopLyricsWindowService>,
) -> Result<DesktopLyricsWindowSnapshot, DesktopLyricsWindowFailure> {
    Arc::clone(service.inner()).show(&app)
}

#[tauri::command]
pub async fn desktop_lyrics_window_ready(
    service: State<'_, ManagedDesktopLyricsWindowService>,
) -> Result<DesktopLyricsWindowSnapshot, DesktopLyricsWindowFailure> {
    service.window_ready()
}

#[tauri::command]
pub async fn hide_desktop_lyrics_window(
    app: AppHandle,
    service: State<'_, ManagedDesktopLyricsWindowService>,
) -> Result<DesktopLyricsWindowSnapshot, DesktopLyricsWindowFailure> {
    service.hide(&app)
}

#[tauri::command]
pub async fn lock_desktop_lyrics_window(
    service: State<'_, ManagedDesktopLyricsWindowService>,
) -> Result<DesktopLyricsWindowSnapshot, DesktopLyricsWindowFailure> {
    service.lock()
}

#[tauri::command]
pub async fn unlock_desktop_lyrics_window(
    service: State<'_, ManagedDesktopLyricsWindowService>,
) -> Result<DesktopLyricsWindowSnapshot, DesktopLyricsWindowFailure> {
    service.unlock()
}

#[tauri::command]
pub async fn start_desktop_lyrics_drag(
    service: State<'_, ManagedDesktopLyricsWindowService>,
) -> Result<(), DesktopLyricsWindowFailure> {
    service.start_dragging()
}

#[tauri::command]
pub async fn open_main_settings(app: AppHandle) -> Result<(), DesktopLyricsWindowFailure> {
    let window = app.get_webview_window("main").ok_or_else(|| {
        DesktopLyricsWindowFailure::new(
            "main_window_unavailable",
            "the main application window is unavailable",
        )
    })?;
    window.show().map_err(|error| {
        DesktopLyricsWindowFailure::new("main_window_show_failed", error.to_string())
    })?;
    window.set_focus().map_err(|error| {
        DesktopLyricsWindowFailure::new("main_window_focus_failed", error.to_string())
    })?;
    app.emit_to("main", "resona://open-settings", ())
        .map_err(|error| {
            DesktopLyricsWindowFailure::new("main_window_event_failed", error.to_string())
        })
}

async fn run_persistence_operation<T, F>(
    persistence: ManagedPersistence,
    operation: F,
) -> Result<T, PersistenceFailure>
where
    T: Send + 'static,
    F: FnOnce(&PersistenceService) -> Result<T, crate::persistence::PersistenceError>
        + Send
        + 'static,
{
    tauri::async_runtime::spawn_blocking(move || operation(&persistence))
        .await
        .map_err(|error| PersistenceFailure {
            code: "persistence_task_failed".to_owned(),
            message: format!("本地数据任务失败：{error}"),
        })?
        .map_err(|error| error.failure())
}

async fn record_recent(persistence: &ManagedPersistence, snapshot: &PlaybackSnapshot) {
    let Some(path) = snapshot.path.clone() else {
        return;
    };
    let persistence = Arc::clone(persistence);
    let result = tauri::async_runtime::spawn_blocking(move || {
        if let Err(error) = persistence.record_recent(&path) {
            eprintln!("recent playback record failed: {error}");
        }
    })
    .await;
    if let Err(error) = result {
        eprintln!("recent playback task failed: {error}");
    }
}

#[tauri::command]
pub async fn list_playlists(
    persistence: State<'_, ManagedPersistence>,
) -> Result<Vec<PlaylistSummary>, PersistenceFailure> {
    run_persistence_operation(Arc::clone(persistence.inner()), |service| {
        service.list_playlists()
    })
    .await
}

#[tauri::command]
pub async fn create_playlist(
    name: String,
    paths: Vec<String>,
    position: Option<i64>,
    ensure_unique_name: bool,
    require_items: bool,
    persistence: State<'_, ManagedPersistence>,
) -> Result<PlaylistMutationResult, PersistenceFailure> {
    run_persistence_operation(Arc::clone(persistence.inner()), move |service| {
        crate::playlists::create_playlist(
            service,
            &name,
            paths.into_iter().map(PathBuf::from).collect(),
            position,
            ensure_unique_name,
            require_items,
        )
    })
    .await
}

#[tauri::command]
pub async fn rename_playlist(
    id: i64,
    name: String,
    persistence: State<'_, ManagedPersistence>,
) -> Result<PlaylistSummary, PersistenceFailure> {
    run_persistence_operation(Arc::clone(persistence.inner()), move |service| {
        service.rename_playlist(id, &name)
    })
    .await
}

#[tauri::command]
pub async fn delete_playlist(
    id: i64,
    persistence: State<'_, ManagedPersistence>,
) -> Result<(), PersistenceFailure> {
    run_persistence_operation(Arc::clone(persistence.inner()), move |service| {
        service.delete_playlist(id)
    })
    .await
}

#[tauri::command]
pub async fn move_playlist(
    id: i64,
    to_position: i64,
    persistence: State<'_, ManagedPersistence>,
) -> Result<Vec<PlaylistSummary>, PersistenceFailure> {
    run_persistence_operation(Arc::clone(persistence.inner()), move |service| {
        service.move_playlist(id, to_position)
    })
    .await
}

#[tauri::command]
pub async fn list_playlist_items(
    playlist_id: i64,
    persistence: State<'_, ManagedPersistence>,
) -> Result<Vec<PlaylistItemRecord>, PersistenceFailure> {
    run_persistence_operation(Arc::clone(persistence.inner()), move |service| {
        service.list_playlist_items(playlist_id)
    })
    .await
}

#[tauri::command]
pub async fn add_playlist_items(
    playlist_id: i64,
    paths: Vec<String>,
    position: Option<i64>,
    persistence: State<'_, ManagedPersistence>,
) -> Result<PlaylistMutationResult, PersistenceFailure> {
    run_persistence_operation(Arc::clone(persistence.inner()), move |service| {
        crate::playlists::add_playlist_items(
            service,
            playlist_id,
            paths.into_iter().map(PathBuf::from).collect(),
            position,
        )
    })
    .await
}

#[tauri::command]
pub async fn remove_playlist_item(
    playlist_id: i64,
    item_id: i64,
    persistence: State<'_, ManagedPersistence>,
) -> Result<Vec<PlaylistItemRecord>, PersistenceFailure> {
    run_persistence_operation(Arc::clone(persistence.inner()), move |service| {
        service.remove_playlist_item(playlist_id, item_id)
    })
    .await
}

#[tauri::command]
pub async fn move_playlist_item(
    playlist_id: i64,
    item_id: i64,
    to_position: i64,
    persistence: State<'_, ManagedPersistence>,
) -> Result<Vec<PlaylistItemRecord>, PersistenceFailure> {
    run_persistence_operation(Arc::clone(persistence.inner()), move |service| {
        service.move_playlist_item(playlist_id, item_id, to_position)
    })
    .await
}

#[tauri::command]
pub async fn list_recent_play(
    limit: Option<u32>,
    persistence: State<'_, ManagedPersistence>,
) -> Result<Vec<RecentPlayRecord>, PersistenceFailure> {
    run_persistence_operation(Arc::clone(persistence.inner()), move |service| {
        service.list_recent(limit.unwrap_or(50))
    })
    .await
}
