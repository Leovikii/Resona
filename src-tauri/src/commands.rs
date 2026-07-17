// SPDX-License-Identifier: GPL-3.0-only

use std::path::PathBuf;
use std::sync::Arc;

use tauri::State;

use crate::playback::{PlaybackEngine, PlaybackSnapshot, RodioPlaybackEngine};

type ManagedPlaybackEngine = Arc<RodioPlaybackEngine>;

async fn run_engine_operation<F>(
    engine: ManagedPlaybackEngine,
    operation: F,
) -> Result<PlaybackSnapshot, String>
where
    F: FnOnce(&RodioPlaybackEngine) -> Result<PlaybackSnapshot, crate::playback::PlaybackError>
        + Send
        + 'static,
{
    tauri::async_runtime::spawn_blocking(move || operation(&engine))
        .await
        .map_err(|error| format!("playback task failed: {error}"))?
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn play_file(
    path: String,
    engine: State<'_, ManagedPlaybackEngine>,
) -> Result<PlaybackSnapshot, String> {
    let engine = Arc::clone(engine.inner());
    run_engine_operation(engine, move |engine| engine.play(PathBuf::from(path))).await
}

#[tauri::command]
pub async fn pause_playback(
    engine: State<'_, ManagedPlaybackEngine>,
) -> Result<PlaybackSnapshot, String> {
    let engine = Arc::clone(engine.inner());
    run_engine_operation(engine, PlaybackEngine::pause).await
}

#[tauri::command]
pub async fn resume_playback(
    engine: State<'_, ManagedPlaybackEngine>,
) -> Result<PlaybackSnapshot, String> {
    let engine = Arc::clone(engine.inner());
    run_engine_operation(engine, PlaybackEngine::resume).await
}

#[tauri::command]
pub async fn stop_playback(
    engine: State<'_, ManagedPlaybackEngine>,
) -> Result<PlaybackSnapshot, String> {
    let engine = Arc::clone(engine.inner());
    run_engine_operation(engine, PlaybackEngine::stop).await
}

#[tauri::command]
pub async fn get_playback_state(
    engine: State<'_, ManagedPlaybackEngine>,
) -> Result<PlaybackSnapshot, String> {
    let engine = Arc::clone(engine.inner());
    run_engine_operation(engine, PlaybackEngine::snapshot).await
}
