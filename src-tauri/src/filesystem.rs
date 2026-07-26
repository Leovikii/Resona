// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::playback::PlaybackFailure;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectedPath {
    pub path: String,
    pub reason: RejectedPathReason,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectedPathReason {
    Missing,
    Unsupported,
    Unreadable,
    LinkedPath,
    EmptyFolder,
    Duplicate,
}

pub struct AudioFileContext {
    pub paths: Vec<PathBuf>,
    pub selected_index: usize,
}

pub struct ResolvedAudioPaths {
    pub items: Vec<ResolvedAudioItem>,
    pub rejected: Vec<RejectedPath>,
}

pub struct ResolvedAudioItem {
    pub path: PathBuf,
    pub folder_root: Option<PathBuf>,
}

pub fn audio_file_context(path: &Path) -> Result<AudioFileContext, PlaybackFailure> {
    if !path.is_file() || !is_supported_audio(path) {
        return Err(PlaybackFailure::task_failed(format!(
            "Unsupported or missing audio file: {}",
            path.display()
        )));
    }
    let parent = path.parent().ok_or_else(|| {
        PlaybackFailure::task_failed(format!("Audio file has no parent: {}", path.display()))
    })?;
    let paths = audio_files_in_directory(parent)?;
    let selected_index = paths
        .iter()
        .position(|candidate| candidate == path)
        .ok_or_else(|| {
            PlaybackFailure::task_failed(format!(
                "Selected audio file is not visible in its folder: {}",
                path.display()
            ))
        })?;
    Ok(AudioFileContext {
        paths,
        selected_index,
    })
}

pub fn resolve_audio_paths(paths: Vec<PathBuf>) -> ResolvedAudioPaths {
    let mut resolved = Vec::new();
    let mut rejected = Vec::new();
    let mut seen = HashSet::new();

    for path in paths {
        match fs::symlink_metadata(&path) {
            Ok(metadata) if is_link_like(&metadata) => {
                rejected.push(rejected_path(path, RejectedPathReason::LinkedPath));
            }
            Ok(metadata) if metadata.is_file() => {
                if is_supported_audio(&path) {
                    push_unique(
                        ResolvedAudioItem {
                            path: normalize_audio_path(path),
                            folder_root: None,
                        },
                        &mut resolved,
                        &mut rejected,
                        &mut seen,
                    );
                } else {
                    rejected.push(rejected_path(path, RejectedPathReason::Unsupported));
                }
            }
            Ok(metadata) if metadata.is_dir() => {
                let root = normalize_audio_path(path.clone());
                let rejected_before = rejected.len();
                let audio = audio_files_in_tree(&root, &mut rejected);
                if audio.is_empty() && rejected.len() == rejected_before {
                    rejected.push(rejected_path(path, RejectedPathReason::EmptyFolder));
                } else {
                    for path in audio {
                        push_unique(
                            ResolvedAudioItem {
                                path,
                                folder_root: Some(root.clone()),
                            },
                            &mut resolved,
                            &mut rejected,
                            &mut seen,
                        );
                    }
                }
            }
            Ok(_) => rejected.push(rejected_path(path, RejectedPathReason::Unsupported)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                rejected.push(rejected_path(path, RejectedPathReason::Missing));
            }
            Err(_) => rejected.push(rejected_path(path, RejectedPathReason::Unreadable)),
        }
    }

    ResolvedAudioPaths {
        items: resolved,
        rejected,
    }
}

fn push_unique(
    item: ResolvedAudioItem,
    resolved: &mut Vec<ResolvedAudioItem>,
    rejected: &mut Vec<RejectedPath>,
    seen: &mut HashSet<String>,
) {
    if seen.insert(path_identity_key(&item.path)) {
        resolved.push(item);
    } else {
        rejected.push(rejected_path(item.path, RejectedPathReason::Duplicate));
    }
}

fn audio_files_in_tree(root: &Path, rejected: &mut Vec<RejectedPath>) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut pending = vec![root.to_path_buf()];

    while let Some(path) = pending.pop() {
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(_) => {
                rejected.push(rejected_path(path, RejectedPathReason::Unreadable));
                continue;
            }
        };
        if is_link_like(&metadata) {
            rejected.push(rejected_path(path, RejectedPathReason::LinkedPath));
            continue;
        }
        if metadata.is_file() {
            if is_supported_audio(&path) {
                paths.push(normalize_audio_path(path));
            }
            continue;
        }
        if !metadata.is_dir() {
            continue;
        }

        let entries = match fs::read_dir(&path) {
            Ok(entries) => entries,
            Err(_) => {
                rejected.push(rejected_path(path, RejectedPathReason::Unreadable));
                continue;
            }
        };
        let mut children = Vec::new();
        for entry in entries {
            match entry {
                Ok(entry) => children.push(entry.path()),
                Err(_) => {
                    rejected.push(rejected_path(path.clone(), RejectedPathReason::Unreadable))
                }
            }
        }
        children.sort_by(|left, right| compare_paths(left, right));
        pending.extend(children.into_iter().rev());
    }

    paths
}

fn audio_files_in_directory(path: &Path) -> Result<Vec<PathBuf>, PlaybackFailure> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(path)
        .map_err(|error| PlaybackFailure::task_failed(format!("Cannot read folder: {error}")))?
    {
        let entry = entry
            .map_err(|error| PlaybackFailure::task_failed(format!("Cannot read entry: {error}")))?;
        let file_type = entry.file_type().map_err(|error| {
            PlaybackFailure::task_failed(format!("Cannot inspect entry: {error}"))
        })?;
        let path = entry.path();
        if file_type.is_file() && is_supported_audio(&path) {
            paths.push(path);
        }
    }
    paths.sort_by(|left, right| compare_paths(left, right));
    Ok(paths)
}

fn rejected_path(path: PathBuf, reason: RejectedPathReason) -> RejectedPath {
    RejectedPath {
        path: path.to_string_lossy().into_owned(),
        reason,
    }
}

fn compare_paths(left: &Path, right: &Path) -> std::cmp::Ordering {
    let left_name = left
        .file_name()
        .map(|name| name.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let right_name = right
        .file_name()
        .map(|name| name.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    left_name
        .cmp(&right_name)
        .then_with(|| left.as_os_str().cmp(right.as_os_str()))
}

fn normalize_audio_path(path: PathBuf) -> PathBuf {
    path
}

pub(crate) fn path_identity_key(path: &Path) -> String {
    let normalized = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let value = normalized.to_string_lossy().into_owned();
    #[cfg(target_os = "windows")]
    {
        value.to_lowercase()
    }
    #[cfg(not(target_os = "windows"))]
    {
        value
    }
}

pub(crate) fn is_link_like(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

pub(crate) fn is_supported_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "mp3" | "wav" | "flac"
            )
        })
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn builds_a_sorted_context_with_the_selected_file_index() {
        let root = test_directory();
        let selected = root.join("B-track.flac");
        File::create(root.join("a-track.wav")).expect("create first audio file");
        File::create(&selected).expect("create selected audio file");
        File::create(root.join("c-track.mp3")).expect("create last audio file");

        let context = audio_file_context(&selected).expect("create audio context");
        assert_eq!(context.paths.len(), 3);
        assert_eq!(context.selected_index, 1);
        assert_eq!(context.paths[context.selected_index], selected);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn recursively_expands_audio_and_reports_rejected_inputs() {
        let root = test_directory();
        File::create(root.join("one.wav")).expect("create audio file");
        File::create(root.join("notes.txt")).expect("create unsupported file");
        let nested = root.join("nested");
        fs::create_dir(&nested).expect("create nested directory");
        File::create(nested.join("two.flac")).expect("create nested audio file");
        let missing = root.join("missing.flac");

        let resolved = resolve_audio_paths(vec![root.clone(), missing]);
        assert_eq!(
            resolved
                .items
                .iter()
                .map(|item| item.path.clone())
                .collect::<Vec<_>>(),
            [nested.join("two.flac"), root.join("one.wav")]
        );
        assert!(resolved
            .items
            .iter()
            .all(|item| { item.folder_root.as_deref() == Some(root.as_path()) }));
        assert_eq!(resolved.rejected.len(), 1);
        assert_eq!(resolved.rejected[0].reason, RejectedPathReason::Missing);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn deduplicates_overlapping_files_and_folders() {
        let root = test_directory();
        let audio = root.join("one.wav");
        File::create(&audio).expect("create audio file");

        let resolved = resolve_audio_paths(vec![audio.clone(), root.clone(), audio.clone()]);
        assert_eq!(
            resolved
                .items
                .iter()
                .map(|item| item.path.clone())
                .collect::<Vec<_>>(),
            [audio]
        );
        assert_eq!(resolved.rejected.len(), 2);
        assert!(resolved
            .rejected
            .iter()
            .all(|rejected| rejected.reason == RejectedPathReason::Duplicate));

        let _ = fs::remove_dir_all(root);
    }

    fn test_directory() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("resona-folder-test-{nonce}"));
        fs::create_dir_all(&path).expect("create test directory");
        path
    }
}
