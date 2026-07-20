// SPDX-License-Identifier: GPL-3.0-only

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
    EmptyFolder,
}

pub struct AudioFileContext {
    pub paths: Vec<PathBuf>,
    pub selected_index: usize,
}

pub struct ResolvedAudioPaths {
    pub paths: Vec<PathBuf>,
    pub rejected: Vec<RejectedPath>,
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

    for path in paths {
        match fs::metadata(&path) {
            Ok(metadata) if metadata.is_file() => {
                if is_supported_audio(&path) {
                    resolved.push(path);
                } else {
                    rejected.push(rejected_path(path, RejectedPathReason::Unsupported));
                }
            }
            Ok(metadata) if metadata.is_dir() => match audio_files_in_directory(&path) {
                Ok(audio) if audio.is_empty() => {
                    rejected.push(rejected_path(path, RejectedPathReason::EmptyFolder));
                }
                Ok(mut audio) => resolved.append(&mut audio),
                Err(_) => rejected.push(rejected_path(path, RejectedPathReason::Unreadable)),
            },
            Ok(_) => rejected.push(rejected_path(path, RejectedPathReason::Unsupported)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                rejected.push(rejected_path(path, RejectedPathReason::Missing));
            }
            Err(_) => rejected.push(rejected_path(path, RejectedPathReason::Unreadable)),
        }
    }

    ResolvedAudioPaths {
        paths: resolved,
        rejected,
    }
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
    fn expands_direct_child_audio_and_reports_rejected_inputs() {
        let root = test_directory();
        File::create(root.join("one.wav")).expect("create audio file");
        File::create(root.join("notes.txt")).expect("create unsupported file");
        let missing = root.join("missing.flac");

        let resolved = resolve_audio_paths(vec![root.clone(), missing]);
        assert_eq!(resolved.paths, [root.join("one.wav")]);
        assert_eq!(resolved.rejected.len(), 1);
        assert_eq!(resolved.rejected[0].reason, RejectedPathReason::Missing);

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
