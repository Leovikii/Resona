// SPDX-License-Identifier: GPL-3.0-only

use std::path::PathBuf;

use serde::Serialize;

use crate::filesystem::{self, RejectedPath};
use crate::persistence::{
    PersistenceError, PersistenceService, PlaylistDetails, PlaylistItemInput, PlaylistItemRecord,
    PlaylistSummary,
};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistMutationResult {
    pub playlist: PlaylistSummary,
    pub items: Vec<PlaylistItemRecord>,
    pub rejected: Vec<RejectedPath>,
}

pub fn create_playlist(
    persistence: &PersistenceService,
    name: &str,
    paths: Vec<PathBuf>,
    position: Option<i64>,
    ensure_unique_name: bool,
    require_items: bool,
) -> Result<PlaylistMutationResult, PersistenceError> {
    let name = if ensure_unique_name && require_items {
        suggested_playlist_name(&paths, name)
    } else {
        name.to_owned()
    };
    let resolved = filesystem::resolve_audio_paths(paths);
    if require_items && resolved.items.is_empty() {
        return Err(PersistenceError::Query(
            "没有可加入播放列表的受支持音频".to_owned(),
        ));
    }
    let item_inputs = item_inputs(resolved.items);
    let PlaylistDetails { playlist, items } = persistence.create_playlist_with_items(
        &name,
        &item_inputs,
        position,
        ensure_unique_name,
    )?;
    Ok(PlaylistMutationResult {
        playlist,
        items,
        rejected: resolved.rejected,
    })
}

fn suggested_playlist_name(paths: &[PathBuf], fallback: &str) -> String {
    let sources = paths
        .iter()
        .filter_map(|path| {
            let directory = if path.is_dir() {
                path.as_path()
            } else {
                path.parent()?
            };
            directory
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .collect::<Vec<_>>();
    sources
        .first()
        .filter(|first| sources.iter().all(|source| source == *first))
        .cloned()
        .unwrap_or_else(|| fallback.to_owned())
}

pub fn add_playlist_items(
    persistence: &PersistenceService,
    playlist_id: i64,
    paths: Vec<PathBuf>,
    position: Option<i64>,
) -> Result<PlaylistMutationResult, PersistenceError> {
    let resolved = filesystem::resolve_audio_paths(paths);
    let item_inputs = item_inputs(resolved.items);
    let items = persistence.add_playlist_items(playlist_id, &item_inputs, position)?;
    let playlist = persistence
        .list_playlists()?
        .into_iter()
        .find(|playlist| playlist.id == playlist_id)
        .ok_or(PersistenceError::PlaylistNotFound)?;
    Ok(PlaylistMutationResult {
        playlist,
        items,
        rejected: resolved.rejected,
    })
}

fn item_inputs(items: Vec<filesystem::ResolvedAudioItem>) -> Vec<PlaylistItemInput> {
    items
        .into_iter()
        .map(|item| PlaylistItemInput {
            path: item.path.to_string_lossy().into_owned(),
            folder_root: item
                .folder_root
                .map(|path| path.to_string_lossy().into_owned()),
        })
        .collect()
}
