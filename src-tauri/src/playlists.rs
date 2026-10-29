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
    let resolved = filesystem::resolve_audio_paths(paths);
    let name = if ensure_unique_name && require_items {
        suggested_playlist_name(&resolved.items, name)
    } else {
        name.to_owned()
    };
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

fn suggested_playlist_name(items: &[filesystem::ResolvedAudioItem], fallback: &str) -> String {
    let sources = items
        .iter()
        .filter_map(|item| {
            let directory = item
                .folder_root
                .as_deref()
                .or_else(|| item.source.path.parent())?;
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

#[cfg(all(test, target_os = "linux"))]
mod network_tests {
    use super::*;
    use crate::playback::RodioPlaybackEngine;
    use std::io::Read;
    use std::sync::Arc;

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct TreeFixture {
        location: String,
        relative_audio: Vec<String>,
    }

    #[test]
    #[ignore = "requires private RESONA_TEST_NFS_TREE manifest and RESONA_TEST_BINARY in a KDE session"]
    fn nfs_recursive_tree_matches_manifest_and_measures_local_baseline() {
        let fixture: TreeFixture =
            serde_json::from_str(&std::env::var("RESONA_TEST_NFS_TREE").unwrap()).unwrap();
        let root = crate::platform::file_access::resolve_location(&fixture.location).unwrap();
        let data = tempfile::tempdir().unwrap();
        let database = PersistenceService::open(&data.path().join("lists.sqlite")).unwrap();
        let start = std::time::Instant::now();
        let created = create_playlist(
            &database,
            "Tree regression",
            vec![fixture.location.into()],
            None,
            true,
            true,
        )
        .unwrap();
        let create_ms = start.elapsed().as_secs_f64() * 1000.0;
        assert!(
            created.rejected.is_empty(),
            "complete tree must have no unreadable branches"
        );
        let mut actual = created
            .items
            .iter()
            .map(|item| {
                assert_eq!(item.folder_root.as_deref(), root.to_str());
                std::path::Path::new(&item.path)
                    .strip_prefix(&root)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned()
            })
            .collect::<Vec<_>>();
        actual.sort();
        assert!(
            actual == fixture.relative_audio,
            "recursive import must match every expected relative audio path"
        );
        let local = data.path().join("local");
        for relative in &fixture.relative_audio {
            let path = local.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, b"scan only").unwrap();
        }
        for run in 0..3 {
            let start = std::time::Instant::now();
            let network = crate::filesystem::resolve_audio_paths(vec![root.clone()]);
            let network_ms = start.elapsed().as_secs_f64() * 1000.0;
            let start = std::time::Instant::now();
            let baseline = crate::filesystem::resolve_audio_paths(vec![local.clone()]);
            let local_ms = start.elapsed().as_secs_f64() * 1000.0;
            assert!(network.rejected.is_empty());
            assert_eq!(network.items.len(), fixture.relative_audio.len());
            assert_eq!(baseline.items.len(), network.items.len());
            eprintln!("Recursive import run {run}: network {network_ms:.2} ms, local audio structure {local_ms:.2} ms");
        }
        let appended =
            add_playlist_items(&database, created.playlist.id, vec![root.clone()], None).unwrap();
        assert_eq!(appended.items.len(), created.items.len() * 2);
        eprintln!(
            "Tree create/commit {create_ms:.2} ms; {} paths, root contexts and append verified",
            created.items.len()
        );
    }

    #[test]
    #[ignore = "requires RESONA_TEST_NFS_LOCATIONS and RESONA_TEST_BINARY in a KDE session"]
    fn nfs_folders_create_append_and_default_import_without_invalidating_files() {
        let locations: Vec<String> =
            serde_json::from_str(&std::env::var("RESONA_TEST_NFS_LOCATIONS").unwrap()).unwrap();
        let root = tempfile::tempdir().unwrap();
        let persistence = PersistenceService::open(&root.path().join("lists.sqlite")).unwrap();
        let engine = Arc::new(RodioPlaybackEngine::new());
        let imports = crate::media_import::MediaImportService::new(Arc::clone(&engine));
        for location in locations {
            let uri = tauri::Url::parse(&location).unwrap().join(".").unwrap();
            let folder = crate::platform::file_access::resolve_location(uri.as_str()).unwrap();
            let selected = crate::platform::file_access::resolve_location(&location).unwrap();
            let created = create_playlist(
                &persistence,
                "NFS test",
                vec![PathBuf::from(uri.as_str())],
                None,
                true,
                true,
            )
            .unwrap();
            assert!(
                created.rejected.is_empty(),
                "network folder must not silently lose entries"
            );
            assert!(created
                .items
                .iter()
                .any(|item| item.path == selected.to_string_lossy()));
            assert!(created
                .items
                .iter()
                .all(|item| item.folder_root.as_deref() == folder.to_str()));
            assert!(created
                .playlist
                .name
                .starts_with(&folder.file_name().unwrap().to_string_lossy().to_string()));
            let empty =
                create_playlist(&persistence, "append test", vec![], None, true, false).unwrap();
            let appended =
                add_playlist_items(&persistence, empty.playlist.id, vec![folder.clone()], None)
                    .unwrap();
            assert_eq!(appended.items.len(), created.items.len());
            let repeated = add_playlist_items(
                &persistence,
                empty.playlist.id,
                vec![PathBuf::from(uri.as_str()), folder.clone()],
                None,
            )
            .unwrap();
            assert_eq!(
                repeated.items.len(),
                created.items.len() * 2,
                "one request deduplicates URI and mapped folder; a new append retains existing semantics"
            );
            let default = imports
                .add_paths(vec![PathBuf::from(uri.as_str())], None)
                .unwrap();
            assert!(default
                .default_playlist
                .items
                .iter()
                .any(|item| item.path == selected.to_string_lossy()));
            let mut header = [0; 4];
            std::fs::File::open(&selected)
                .unwrap()
                .read_exact(&mut header)
                .unwrap();
            assert!(crate::filesystem::audio_file_context(&selected)
                .unwrap()
                .warning
                .is_none());
            eprintln!(
                "NFS folder import: {} items; create, append, dedup, default and reread passed",
                created.items.len()
            );
        }
    }
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
            source: item.source,
            folder_root: item
                .folder_root
                .map(|path| path.to_string_lossy().into_owned()),
        })
        .collect()
}
