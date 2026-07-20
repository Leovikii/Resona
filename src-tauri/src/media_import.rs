// SPDX-License-Identifier: GPL-3.0-only

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::Serialize;

use crate::filesystem::{self, AudioFileContext};
use crate::playback::{PlaybackEngine, PlaybackFailure, PlaybackSnapshot, RodioPlaybackEngine};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefaultPlaylistItem {
    pub path: String,
    pub display_name: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefaultPlaylistSnapshot {
    pub revision: u64,
    pub source_directory: Option<String>,
    pub selected_index: Option<usize>,
    pub items: Vec<DefaultPlaylistItem>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenMediaResult {
    pub playback: PlaybackSnapshot,
    pub default_playlist: DefaultPlaylistSnapshot,
}

#[derive(Default)]
struct DefaultPlaylistState {
    revision: u64,
    source_directory: Option<PathBuf>,
    selected_index: Option<usize>,
    paths: Vec<PathBuf>,
}

pub struct MediaImportService {
    engine: Arc<RodioPlaybackEngine>,
    state: Mutex<DefaultPlaylistState>,
}

impl MediaImportService {
    pub fn new(engine: Arc<RodioPlaybackEngine>) -> Self {
        Self {
            engine,
            state: Mutex::new(DefaultPlaylistState::default()),
        }
    }

    pub fn snapshot(&self) -> Result<DefaultPlaylistSnapshot, PlaybackFailure> {
        let state = self.state.lock().map_err(|_| state_poisoned())?;
        Ok(snapshot_from_state(&state))
    }

    pub fn open_media_context(&self, path: &Path) -> Result<OpenMediaResult, PlaybackFailure> {
        let context = filesystem::audio_file_context(path)?;
        let mut state = self.state.lock().map_err(|_| state_poisoned())?;
        let queue = self.engine.snapshot().map_err(|error| error.failure())?;
        let same_default = state.paths == context.paths;
        let queue_matches = queue_paths_match(&queue, &context.paths);
        let playback = if same_default && queue_matches {
            let item = queue.queue.get(context.selected_index).ok_or_else(|| {
                PlaybackFailure::task_failed("default playlist target is unavailable".to_owned())
            })?;
            self.engine
                .play_queue_item(item.id)
                .map_err(|error| error.failure())?
        } else {
            self.engine
                .replace_queue_and_play(context.paths.clone(), context.selected_index)
                .map_err(|error| error.failure())?
        };
        commit_context(&mut state, context);
        Ok(OpenMediaResult {
            playback,
            default_playlist: snapshot_from_state(&state),
        })
    }
}

pub fn external_media_path<I, S>(arguments: I, current_directory: &Path) -> Option<PathBuf>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    arguments.into_iter().skip(1).find_map(|argument| {
        let argument = argument.as_ref();
        if argument.starts_with('-') {
            return None;
        }
        let candidate = PathBuf::from(argument);
        let candidate = if candidate.is_absolute() {
            candidate
        } else {
            current_directory.join(candidate)
        };
        (candidate.is_file() && filesystem::is_supported_audio(&candidate)).then_some(candidate)
    })
}

fn commit_context(state: &mut DefaultPlaylistState, context: AudioFileContext) {
    if state.paths != context.paths {
        state.revision = state.revision.wrapping_add(1).max(1);
        state.source_directory = context
            .paths
            .first()
            .and_then(|path| path.parent())
            .map(Path::to_owned);
        state.paths = context.paths;
    }
    state.selected_index = Some(context.selected_index);
}

fn queue_paths_match(snapshot: &PlaybackSnapshot, paths: &[PathBuf]) -> bool {
    snapshot.queue.len() == paths.len()
        && snapshot
            .queue
            .iter()
            .zip(paths)
            .all(|(item, path)| Path::new(&item.path) == path)
}

fn snapshot_from_state(state: &DefaultPlaylistState) -> DefaultPlaylistSnapshot {
    DefaultPlaylistSnapshot {
        revision: state.revision,
        source_directory: state
            .source_directory
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        selected_index: state.selected_index,
        items: state
            .paths
            .iter()
            .map(|path| DefaultPlaylistItem {
                path: path.to_string_lossy().into_owned(),
                display_name: path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.to_string_lossy().into_owned()),
            })
            .collect(),
    }
}

fn state_poisoned() -> PlaybackFailure {
    PlaybackFailure::task_failed("default playlist state is unavailable".to_owned())
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn identical_directory_snapshot_keeps_revision_and_updates_selection() {
        let root = test_directory();
        let first = root.join("1.wav");
        let second = root.join("2.flac");
        File::create(&first).expect("create first audio");
        File::create(&second).expect("create second audio");

        let mut state = DefaultPlaylistState::default();
        commit_context(
            &mut state,
            filesystem::audio_file_context(&first).expect("first context"),
        );
        let revision = state.revision;
        commit_context(
            &mut state,
            filesystem::audio_file_context(&second).expect("second context"),
        );
        assert_eq!(state.revision, revision);
        assert_eq!(state.selected_index, Some(1));

        File::create(root.join("3.mp3")).expect("create third audio");
        commit_context(
            &mut state,
            filesystem::audio_file_context(&second).expect("changed context"),
        );
        assert_eq!(state.revision, revision + 1);
        assert_eq!(state.paths.len(), 3);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn external_arguments_accept_supported_relative_or_absolute_files() {
        let root = test_directory();
        let audio = root.join("track.flac");
        File::create(&audio).expect("create audio");
        assert_eq!(
            external_media_path(["resona.exe", "track.flac"], &root),
            Some(audio.clone())
        );
        assert_eq!(
            external_media_path(
                [
                    "resona.exe".to_owned(),
                    audio.to_string_lossy().into_owned()
                ],
                &root,
            ),
            Some(audio)
        );
        assert_eq!(external_media_path(["resona.exe", "--flag"], &root), None);
        let _ = fs::remove_dir_all(root);
    }

    fn test_directory() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("resona-default-list-test-{nonce}"));
        fs::create_dir_all(&path).expect("create test directory");
        path
    }
}
