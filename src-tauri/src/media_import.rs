// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::Serialize;

use crate::filesystem::{
    self, AudioFileContext, RejectedPath, RejectedPathReason, ResolvedAudioPaths,
};
use crate::playback::{PlaybackEngine, PlaybackFailure, PlaybackSnapshot, RodioPlaybackEngine};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivePlaylistKind {
    Default,
    User,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivePlaylistSnapshot {
    pub kind: ActivePlaylistKind,
    pub playlist_id: Option<i64>,
}

impl ActivePlaylistSnapshot {
    fn default_playlist() -> Self {
        Self {
            kind: ActivePlaylistKind::Default,
            playlist_id: None,
        }
    }

    fn user_playlist(playlist_id: i64) -> Self {
        Self {
            kind: ActivePlaylistKind::User,
            playlist_id: Some(playlist_id),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefaultPlaylistItem {
    pub id: u64,
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
    pub active_playlist: ActivePlaylistSnapshot,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistPlaybackResult {
    pub playback: PlaybackSnapshot,
    pub active_playlist: ActivePlaylistSnapshot,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefaultPlaylistMutationResult {
    pub default_playlist: DefaultPlaylistSnapshot,
    pub rejected: Vec<RejectedPath>,
}

#[derive(Clone, Default)]
struct DefaultPlaylistState {
    revision: u64,
    source_directory: Option<PathBuf>,
    selected_index: Option<usize>,
    paths: Vec<PathBuf>,
    item_ids: Vec<u64>,
    next_item_id: u64,
    active_playlist: Option<ActivePlaylistSnapshot>,
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

    pub fn active_playlist(&self) -> Result<Option<ActivePlaylistSnapshot>, PlaybackFailure> {
        let state = self.state.lock().map_err(|_| state_poisoned())?;
        Ok(state.active_playlist.clone())
    }

    pub fn open_media_context(&self, path: &Path) -> Result<OpenMediaResult, PlaybackFailure> {
        let context = filesystem::audio_file_context(path)?;
        let mut state = self.state.lock().map_err(|_| state_poisoned())?;
        let playback = self.play_paths(&state.paths, &context.paths, context.selected_index)?;
        commit_context(&mut state, context);
        state.active_playlist = Some(ActivePlaylistSnapshot::default_playlist());
        Ok(OpenMediaResult {
            playback,
            default_playlist: snapshot_from_state(&state),
            active_playlist: ActivePlaylistSnapshot::default_playlist(),
        })
    }

    pub fn add_paths(
        &self,
        paths: Vec<PathBuf>,
        position: Option<usize>,
    ) -> Result<DefaultPlaylistMutationResult, PlaybackFailure> {
        let resolved = filesystem::resolve_audio_paths(paths);
        let mut state = self.state.lock().map_err(|_| state_poisoned())?;
        let previous = state.clone();
        let (result, added, insertion) = insert_resolved_paths(&mut state, resolved, position);
        if !added.is_empty()
            && state.active_playlist == Some(ActivePlaylistSnapshot::default_playlist())
        {
            let queue = self.engine.snapshot().map_err(|error| error.failure())?;
            if !queue_paths_match(&queue, &previous.paths) {
                *state = previous;
                return Err(sequence_mismatch());
            }
            if let Err(error) = self.engine.insert_queue(added, insertion) {
                *state = previous;
                return Err(error.failure());
            }
        }
        Ok(result)
    }

    pub fn play_item(&self, item_id: u64) -> Result<OpenMediaResult, PlaybackFailure> {
        let mut state = self.state.lock().map_err(|_| state_poisoned())?;
        let selected_index = state
            .item_ids
            .iter()
            .position(|candidate| *candidate == item_id)
            .ok_or_else(|| {
                PlaybackFailure::task_failed("default playlist target is unavailable".to_owned())
            })?;
        let playback = self.play_paths(&state.paths, &state.paths, selected_index)?;
        state.selected_index = Some(selected_index);
        state.active_playlist = Some(ActivePlaylistSnapshot::default_playlist());
        Ok(OpenMediaResult {
            playback,
            default_playlist: snapshot_from_state(&state),
            active_playlist: ActivePlaylistSnapshot::default_playlist(),
        })
    }

    pub fn remove_items(
        &self,
        item_ids: &[u64],
    ) -> Result<DefaultPlaylistSnapshot, PlaybackFailure> {
        if item_ids.is_empty() {
            return self.snapshot();
        }
        let mut state = self.state.lock().map_err(|_| state_poisoned())?;
        let mut positions = item_ids
            .iter()
            .map(|item_id| {
                state
                    .item_ids
                    .iter()
                    .position(|candidate| candidate == item_id)
            })
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                PlaybackFailure::task_failed("default playlist item not found".to_owned())
            })?;
        positions.sort_unstable();
        positions.dedup();
        if positions.len() != item_ids.len() {
            return Err(PlaybackFailure::task_failed(
                "default playlist item selection contains duplicates".to_owned(),
            ));
        }
        let previous_paths = state.paths.clone();
        if state.active_playlist == Some(ActivePlaylistSnapshot::default_playlist()) {
            let queue = self.engine.snapshot().map_err(|error| error.failure())?;
            if !queue_paths_match(&queue, &previous_paths) {
                return Err(sequence_mismatch());
            }
            for position in positions.iter().rev() {
                let queue_item = queue.queue.get(*position).ok_or_else(sequence_mismatch)?;
                self.engine
                    .remove_queue_item(queue_item.id)
                    .map_err(|error| error.failure())?;
            }
        }
        for position in positions.iter().rev() {
            state.paths.remove(*position);
            state.item_ids.remove(*position);
        }
        state.revision = state.revision.wrapping_add(1).max(1);
        state.source_directory = common_parent(&state.paths);
        state.selected_index = state.selected_index.and_then(|selected| {
            if positions.binary_search(&selected).is_ok() {
                None
            } else {
                Some(
                    selected
                        - positions
                            .iter()
                            .filter(|position| **position < selected)
                            .count(),
                )
            }
        });
        Ok(snapshot_from_state(&state))
    }

    pub fn clear_default_playlist(&self) -> Result<DefaultPlaylistSnapshot, PlaybackFailure> {
        let item_ids = {
            let state = self.state.lock().map_err(|_| state_poisoned())?;
            state.item_ids.clone()
        };
        self.remove_items(&item_ids)
    }

    pub fn move_item(
        &self,
        item_id: u64,
        to_position: usize,
    ) -> Result<DefaultPlaylistSnapshot, PlaybackFailure> {
        let mut state = self.state.lock().map_err(|_| state_poisoned())?;
        let from_position = state
            .item_ids
            .iter()
            .position(|candidate| *candidate == item_id)
            .ok_or_else(|| {
                PlaybackFailure::task_failed("default playlist item not found".to_owned())
            })?;
        let target = to_position.min(state.paths.len().saturating_sub(1));
        if from_position == target {
            return Ok(snapshot_from_state(&state));
        }
        if state.active_playlist == Some(ActivePlaylistSnapshot::default_playlist()) {
            let queue = self.engine.snapshot().map_err(|error| error.failure())?;
            if !queue_paths_match(&queue, &state.paths) {
                return Err(sequence_mismatch());
            }
            let queue_item = queue
                .queue
                .get(from_position)
                .ok_or_else(sequence_mismatch)?;
            self.engine
                .move_queue_item(queue_item.id, target)
                .map_err(|error| error.failure())?;
        }
        let selected_id = state
            .selected_index
            .and_then(|index| state.item_ids.get(index))
            .copied();
        let path = state.paths.remove(from_position);
        let id = state.item_ids.remove(from_position);
        state.paths.insert(target, path);
        state.item_ids.insert(target, id);
        state.selected_index = selected_id
            .and_then(|selected_id| state.item_ids.iter().position(|id| *id == selected_id));
        state.revision = state.revision.wrapping_add(1).max(1);
        Ok(snapshot_from_state(&state))
    }

    pub fn play_user_playlist(
        &self,
        playlist_id: i64,
        paths: Vec<PathBuf>,
        selected_index: usize,
    ) -> Result<PlaylistPlaybackResult, PlaybackFailure> {
        if selected_index >= paths.len() {
            return Err(PlaybackFailure::task_failed(
                "playlist target is unavailable".to_owned(),
            ));
        }
        let mut state = self.state.lock().map_err(|_| state_poisoned())?;
        let playback = self
            .engine
            .replace_queue_and_play(paths, selected_index)
            .map_err(|error| error.failure())?;
        let active_playlist = ActivePlaylistSnapshot::user_playlist(playlist_id);
        state.active_playlist = Some(active_playlist.clone());
        Ok(PlaylistPlaybackResult {
            playback,
            active_playlist,
        })
    }

    pub fn sync_user_insert(
        &self,
        playlist_id: i64,
        previous_paths: &[PathBuf],
        next_paths: &[PathBuf],
    ) -> Result<Option<PlaybackSnapshot>, PlaybackFailure> {
        let state = self.state.lock().map_err(|_| state_poisoned())?;
        if state.active_playlist != Some(ActivePlaylistSnapshot::user_playlist(playlist_id)) {
            return Ok(None);
        }
        let queue = self.engine.snapshot().map_err(|error| error.failure())?;
        if !queue_paths_match(&queue, previous_paths) {
            return Err(sequence_mismatch());
        }
        let (position, added) =
            inserted_segment(previous_paths, next_paths).ok_or_else(sequence_mismatch)?;
        let playback = if position == previous_paths.len() {
            self.engine.append_queue(added.to_vec())
        } else {
            self.engine.insert_queue(added.to_vec(), position)
        }
        .map_err(|error| error.failure())?;
        Ok(Some(playback))
    }

    pub fn sync_user_remove(
        &self,
        playlist_id: i64,
        previous_paths: &[PathBuf],
        next_paths: &[PathBuf],
    ) -> Result<Option<PlaybackSnapshot>, PlaybackFailure> {
        let state = self.state.lock().map_err(|_| state_poisoned())?;
        if state.active_playlist != Some(ActivePlaylistSnapshot::user_playlist(playlist_id)) {
            return Ok(None);
        }
        let queue = self.engine.snapshot().map_err(|error| error.failure())?;
        if !queue_paths_match(&queue, previous_paths) {
            return Err(sequence_mismatch());
        }
        let position =
            removed_position(previous_paths, next_paths).ok_or_else(sequence_mismatch)?;
        let queue_item = queue.queue.get(position).ok_or_else(sequence_mismatch)?;
        self.engine
            .remove_queue_item(queue_item.id)
            .map(Some)
            .map_err(|error| error.failure())
    }

    pub fn sync_user_remove_many(
        &self,
        playlist_id: i64,
        previous_paths: &[PathBuf],
        next_paths: &[PathBuf],
    ) -> Result<Option<PlaybackSnapshot>, PlaybackFailure> {
        let state = self.state.lock().map_err(|_| state_poisoned())?;
        if state.active_playlist != Some(ActivePlaylistSnapshot::user_playlist(playlist_id)) {
            return Ok(None);
        }
        let queue = self.engine.snapshot().map_err(|error| error.failure())?;
        if !queue_paths_match(&queue, previous_paths) {
            return Err(sequence_mismatch());
        }
        let positions =
            removed_positions(previous_paths, next_paths).ok_or_else(sequence_mismatch)?;
        let mut playback = None;
        for position in positions.into_iter().rev() {
            let queue_item = queue.queue.get(position).ok_or_else(sequence_mismatch)?;
            playback = Some(
                self.engine
                    .remove_queue_item(queue_item.id)
                    .map_err(|error| error.failure())?,
            );
        }
        Ok(playback)
    }

    pub fn sync_user_move(
        &self,
        playlist_id: i64,
        previous_paths: &[PathBuf],
        from_position: usize,
        to_position: usize,
    ) -> Result<Option<PlaybackSnapshot>, PlaybackFailure> {
        let state = self.state.lock().map_err(|_| state_poisoned())?;
        if state.active_playlist != Some(ActivePlaylistSnapshot::user_playlist(playlist_id)) {
            return Ok(None);
        }
        let queue = self.engine.snapshot().map_err(|error| error.failure())?;
        if !queue_paths_match(&queue, previous_paths) {
            return Err(sequence_mismatch());
        }
        let queue_item = queue
            .queue
            .get(from_position)
            .ok_or_else(sequence_mismatch)?;
        self.engine
            .move_queue_item(queue_item.id, to_position)
            .map(Some)
            .map_err(|error| error.failure())
    }

    pub fn detach_deleted_user_playlist(
        &self,
        playlist_id: i64,
    ) -> Result<Option<DefaultPlaylistSnapshot>, PlaybackFailure> {
        let mut state = self.state.lock().map_err(|_| state_poisoned())?;
        if state.active_playlist != Some(ActivePlaylistSnapshot::user_playlist(playlist_id)) {
            return Ok(None);
        }
        let playback = self.engine.snapshot().map_err(|error| error.failure())?;
        replace_default_paths(
            &mut state,
            playback
                .queue
                .iter()
                .map(|item| PathBuf::from(&item.path))
                .collect(),
        );
        state.revision = state.revision.wrapping_add(1).max(1);
        state.source_directory = common_parent(&state.paths);
        state.selected_index = playback
            .current_item_id
            .and_then(|id| playback.queue.iter().position(|item| item.id == id));
        state.active_playlist = Some(ActivePlaylistSnapshot::default_playlist());
        Ok(Some(snapshot_from_state(&state)))
    }

    fn play_paths(
        &self,
        current_paths: &[PathBuf],
        target_paths: &[PathBuf],
        selected_index: usize,
    ) -> Result<PlaybackSnapshot, PlaybackFailure> {
        let queue = self.engine.snapshot().map_err(|error| error.failure())?;
        if current_paths == target_paths && queue_paths_match(&queue, target_paths) {
            let item = queue.queue.get(selected_index).ok_or_else(|| {
                PlaybackFailure::task_failed("default playlist target is unavailable".to_owned())
            })?;
            self.engine
                .play_queue_item(item.id)
                .map_err(|error| error.failure())
        } else {
            self.engine
                .replace_queue_and_play(target_paths.to_vec(), selected_index)
                .map_err(|error| error.failure())
        }
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
        replace_default_paths(state, context.paths);
    }
    state.selected_index = Some(context.selected_index);
}

fn insert_resolved_paths(
    state: &mut DefaultPlaylistState,
    resolved: ResolvedAudioPaths,
    position: Option<usize>,
) -> (DefaultPlaylistMutationResult, Vec<PathBuf>, usize) {
    let mut rejected = resolved.rejected;
    let mut known = state.paths.iter().cloned().collect::<HashSet<_>>();
    let mut accepted = Vec::new();
    for path in resolved.paths {
        if known.insert(path.clone()) {
            accepted.push(path);
        } else {
            rejected.push(RejectedPath {
                path: path.to_string_lossy().into_owned(),
                reason: RejectedPathReason::Duplicate,
            });
        }
    }
    let insertion = position.unwrap_or(state.paths.len()).min(state.paths.len());
    if !accepted.is_empty() {
        let item_ids = (0..accepted.len())
            .map(|_| next_default_item_id(state))
            .collect::<Vec<_>>();
        state
            .paths
            .splice(insertion..insertion, accepted.iter().cloned());
        state.item_ids.splice(insertion..insertion, item_ids);
        if let Some(selected) = state.selected_index.as_mut() {
            if *selected >= insertion {
                *selected += accepted.len();
            }
        }
        state.revision = state.revision.wrapping_add(1).max(1);
        state.source_directory = common_parent(&state.paths);
    }
    (
        DefaultPlaylistMutationResult {
            default_playlist: snapshot_from_state(state),
            rejected,
        },
        accepted,
        insertion,
    )
}

fn common_parent(paths: &[PathBuf]) -> Option<PathBuf> {
    let first = paths.first()?.parent()?;
    paths
        .iter()
        .all(|path| path.parent() == Some(first))
        .then(|| first.to_owned())
}

fn queue_paths_match(snapshot: &PlaybackSnapshot, paths: &[PathBuf]) -> bool {
    snapshot.queue.len() == paths.len()
        && snapshot
            .queue
            .iter()
            .zip(paths)
            .all(|(item, path)| Path::new(&item.path) == path)
}

fn inserted_segment<'a>(
    previous: &[PathBuf],
    next: &'a [PathBuf],
) -> Option<(usize, &'a [PathBuf])> {
    let added_count = next.len().checked_sub(previous.len())?;
    if added_count == 0 {
        return Some((previous.len(), &next[next.len()..]));
    }
    let prefix = previous
        .iter()
        .zip(next)
        .take_while(|(left, right)| left == right)
        .count();
    (previous[prefix..] == next[prefix + added_count..])
        .then_some((prefix, &next[prefix..prefix + added_count]))
}

fn removed_position(previous: &[PathBuf], next: &[PathBuf]) -> Option<usize> {
    if previous.len() != next.len().saturating_add(1) {
        return None;
    }
    let prefix = previous
        .iter()
        .zip(next)
        .take_while(|(left, right)| left == right)
        .count();
    (previous[prefix + 1..] == next[prefix..]).then_some(prefix)
}

fn removed_positions(previous: &[PathBuf], next: &[PathBuf]) -> Option<Vec<usize>> {
    if next.len() > previous.len() {
        return None;
    }
    let mut retained = next.iter();
    let mut expected = retained.next();
    let mut removed = Vec::new();
    for (index, path) in previous.iter().enumerate() {
        if expected == Some(path) {
            expected = retained.next();
        } else {
            removed.push(index);
        }
    }
    expected.is_none().then_some(removed)
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
            .zip(&state.item_ids)
            .map(|(path, id)| DefaultPlaylistItem {
                id: *id,
                path: path.to_string_lossy().into_owned(),
                display_name: path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.to_string_lossy().into_owned()),
            })
            .collect(),
    }
}

fn replace_default_paths(state: &mut DefaultPlaylistState, paths: Vec<PathBuf>) {
    let mut known_ids = state
        .paths
        .iter()
        .cloned()
        .zip(state.item_ids.iter().copied())
        .collect::<std::collections::HashMap<_, _>>();
    let item_ids = paths
        .iter()
        .map(|path| {
            known_ids
                .remove(path)
                .unwrap_or_else(|| next_default_item_id(state))
        })
        .collect();
    state.paths = paths;
    state.item_ids = item_ids;
}

fn next_default_item_id(state: &mut DefaultPlaylistState) -> u64 {
    state.next_item_id = state.next_item_id.wrapping_add(1).max(1);
    state.next_item_id
}

fn state_poisoned() -> PlaybackFailure {
    PlaybackFailure::task_failed("default playlist state is unavailable".to_owned())
}

fn sequence_mismatch() -> PlaybackFailure {
    PlaybackFailure::task_failed(
        "active playlist and playback sequence are inconsistent".to_owned(),
    )
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::sync::atomic::{AtomicU64, Ordering};
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
        let ids = state.item_ids.clone();

        File::create(root.join("3.mp3")).expect("create third audio");
        commit_context(
            &mut state,
            filesystem::audio_file_context(&second).expect("changed context"),
        );
        assert_eq!(state.revision, revision + 1);
        assert_eq!(state.paths.len(), 3);
        assert_eq!(state.item_ids[..2], ids);
        assert_ne!(state.item_ids[2], ids[0]);
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

    #[test]
    fn appends_resolved_paths_without_persisting_duplicates() {
        let root = test_directory();
        let first = root.join("1.wav");
        let second = root.join("2.flac");
        File::create(&first).expect("create first audio");
        File::create(&second).expect("create second audio");

        let mut state = DefaultPlaylistState::default();
        let (first_result, _, _) = insert_resolved_paths(
            &mut state,
            filesystem::resolve_audio_paths(vec![root.clone()]),
            None,
        );
        assert_eq!(first_result.default_playlist.items.len(), 2);
        assert_eq!(first_result.default_playlist.revision, 1);
        assert_eq!(state.source_directory.as_deref(), Some(root.as_path()));

        let (duplicate_result, _, _) = insert_resolved_paths(
            &mut state,
            filesystem::resolve_audio_paths(vec![first]),
            None,
        );
        assert_eq!(duplicate_result.default_playlist.revision, 1);
        assert_eq!(duplicate_result.rejected.len(), 1);
        assert_eq!(
            duplicate_result.rejected[0].reason,
            RejectedPathReason::Duplicate
        );

        let other_root = test_directory();
        let other = other_root.join("3.mp3");
        File::create(&other).expect("create other audio");
        let (mixed_result, _, _) = insert_resolved_paths(
            &mut state,
            filesystem::resolve_audio_paths(vec![other]),
            None,
        );
        assert_eq!(mixed_result.default_playlist.items.len(), 3);
        assert_eq!(mixed_result.default_playlist.revision, 2);
        assert_eq!(mixed_result.default_playlist.source_directory, None);

        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(other_root);
    }

    #[test]
    fn inserts_resolved_paths_at_the_requested_default_playlist_gap() {
        let root = test_directory();
        let first = root.join("1.wav");
        let second = root.join("2.flac");
        let inserted = root.join("3.mp3");
        for path in [&first, &second, &inserted] {
            File::create(path).expect("create audio");
        }

        let mut state = DefaultPlaylistState::default();
        let _ = insert_resolved_paths(
            &mut state,
            filesystem::resolve_audio_paths(vec![first.clone(), second.clone()]),
            None,
        );
        state.selected_index = Some(1);
        let (_, added, position) = insert_resolved_paths(
            &mut state,
            filesystem::resolve_audio_paths(vec![inserted.clone()]),
            Some(1),
        );

        assert_eq!(position, 1);
        assert_eq!(added, std::slice::from_ref(&inserted));
        assert_eq!(state.paths, [first, inserted, second]);
        assert_eq!(state.selected_index, Some(2));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn inserting_into_the_active_default_playlist_updates_the_runtime_sequence() {
        let root = test_directory();
        let first = root.join("1.wav");
        let second = root.join("2.flac");
        let inserted = root.join("3.mp3");
        for path in [&first, &second, &inserted] {
            File::create(path).expect("create audio");
        }
        let engine = Arc::new(RodioPlaybackEngine::new());
        engine
            .restore_session(crate::playback::RestoredPlaybackSession {
                paths: vec![first.clone(), second.clone()],
                current_path: None,
                position_ms: 0,
                volume: 1.0,
                playback_mode: crate::playback::PlaybackMode::Sequential,
            })
            .expect("restore sequence");
        let service = MediaImportService::new(Arc::clone(&engine));
        {
            let mut state = service.state.lock().expect("lock playlist state");
            let _ = insert_resolved_paths(
                &mut state,
                filesystem::resolve_audio_paths(vec![first.clone(), second.clone()]),
                None,
            );
            state.active_playlist = Some(ActivePlaylistSnapshot::default_playlist());
        }

        service
            .add_paths(vec![inserted.clone()], Some(1))
            .expect("insert active default item");

        assert_eq!(
            engine
                .snapshot()
                .expect("sequence snapshot")
                .queue
                .iter()
                .map(|item| PathBuf::from(&item.path))
                .collect::<Vec<_>>(),
            [first, inserted, second]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn active_user_playlist_edits_update_the_runtime_sequence() {
        let root = test_directory();
        let first = root.join("1.wav");
        let second = root.join("2.flac");
        let inserted = root.join("3.mp3");
        for path in [&first, &second, &inserted] {
            File::create(path).expect("create audio");
        }
        let engine = Arc::new(RodioPlaybackEngine::new());
        engine
            .restore_session(crate::playback::RestoredPlaybackSession {
                paths: vec![first.clone(), second.clone()],
                current_path: None,
                position_ms: 0,
                volume: 1.0,
                playback_mode: crate::playback::PlaybackMode::Sequential,
            })
            .expect("restore sequence");
        let service = MediaImportService::new(Arc::clone(&engine));
        service
            .state
            .lock()
            .expect("lock playlist state")
            .active_playlist = Some(ActivePlaylistSnapshot::user_playlist(7));

        service
            .sync_user_insert(
                7,
                &[first.clone(), second.clone()],
                &[first.clone(), inserted.clone(), second.clone()],
            )
            .expect("insert active item");
        service
            .sync_user_move(7, &[first.clone(), inserted.clone(), second.clone()], 2, 0)
            .expect("move active item");
        service
            .sync_user_remove(
                7,
                &[second.clone(), first.clone(), inserted.clone()],
                &[second.clone(), inserted.clone()],
            )
            .expect("remove active item");

        let snapshot = engine.snapshot().expect("sequence snapshot");
        assert_eq!(
            snapshot
                .queue
                .iter()
                .map(|item| PathBuf::from(&item.path))
                .collect::<Vec<_>>(),
            [second, inserted]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn inactive_user_playlist_edits_do_not_change_the_runtime_sequence() {
        let root = test_directory();
        let first = root.join("1.wav");
        let second = root.join("2.flac");
        for path in [&first, &second] {
            File::create(path).expect("create audio");
        }
        let engine = Arc::new(RodioPlaybackEngine::new());
        engine
            .restore_session(crate::playback::RestoredPlaybackSession {
                paths: vec![first.clone()],
                current_path: None,
                position_ms: 0,
                volume: 1.0,
                playback_mode: crate::playback::PlaybackMode::Sequential,
            })
            .expect("restore sequence");
        let service = MediaImportService::new(Arc::clone(&engine));

        assert!(service
            .sync_user_insert(7, std::slice::from_ref(&first), &[first.clone(), second])
            .expect("ignore inactive edit")
            .is_none());
        assert_eq!(engine.snapshot().expect("sequence snapshot").queue.len(), 1);
        let _ = fs::remove_dir_all(root);
    }

    fn test_directory() -> PathBuf {
        static TEST_DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let sequence = TEST_DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "resona-default-list-test-{}-{nonce}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create test directory");
        path
    }
}
