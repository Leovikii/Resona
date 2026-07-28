// SPDX-License-Identifier: GPL-3.0-only

use std::borrow::Cow;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::Duration;

use encoding_rs::{GBK, UTF_16BE, UTF_16LE};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CueTrackSource {
    pub cue_path: PathBuf,
    pub track_number: u32,
    pub title: Option<String>,
    pub performer: Option<String>,
    pub album: Option<String>,
    pub start_ms: u64,
    pub end_ms: Option<u64>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaSource {
    pub path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cue: Option<CueTrackSource>,
}

impl MediaSource {
    pub fn file(path: PathBuf) -> Self {
        Self { path, cue: None }
    }

    pub fn display_name(&self) -> String {
        self.cue
            .as_ref()
            .and_then(|cue| cue.title.clone())
            .filter(|title| !title.trim().is_empty())
            .unwrap_or_else(|| {
                self.path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| self.path.to_string_lossy().into_owned())
            })
    }

    pub fn duration_ms(&self, physical_duration_ms: Option<u64>) -> Option<u64> {
        let Some(cue) = &self.cue else {
            return physical_duration_ms;
        };
        cue.end_ms
            .or(physical_duration_ms)
            .map(|end| end.saturating_sub(cue.start_ms))
    }
}

pub fn expand_same_name_cue(path: &Path) -> Vec<MediaSource> {
    let fallback = || vec![MediaSource::file(path.to_path_buf())];
    let Some(cue_path) = same_name_cue(path) else {
        return fallback();
    };
    match parse_cue_for_audio(&cue_path, path) {
        Ok(tracks) if !tracks.is_empty() => tracks,
        Ok(_) => fallback(),
        Err(error) => {
            log::warn!("CUE ignored for {}: {error}", path.display());
            fallback()
        }
    }
}

fn same_name_cue(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    let stem = path.file_stem()?.to_string_lossy();
    fs::read_dir(parent)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|candidate| {
            candidate.is_file()
                && candidate
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.eq_ignore_ascii_case("cue"))
                && candidate
                    .file_stem()
                    .is_some_and(|value| value.to_string_lossy().eq_ignore_ascii_case(&stem))
        })
}

fn parse_cue_for_audio(cue_path: &Path, audio_path: &Path) -> Result<Vec<MediaSource>, String> {
    let bytes = fs::read(cue_path).map_err(|error| error.to_string())?;
    let text = decode_text(&bytes)?;
    let mut reader = Cursor::new(text.as_bytes());
    let cue = rcue::parser::parse(&mut reader, false).map_err(|error| error.to_string())?;
    let cue_parent = cue_path.parent().unwrap_or_else(|| Path::new(""));
    let audio_identity = crate::filesystem::path_identity_key(audio_path);
    let mut sources = Vec::new();

    for cue_file in cue.files {
        let referenced = cue_parent.join(&cue_file.file);
        if crate::filesystem::path_identity_key(&referenced) != audio_identity {
            continue;
        }
        let starts = cue_file.tracks.iter().map(track_start).collect::<Vec<_>>();
        for (index, track) in cue_file.tracks.into_iter().enumerate() {
            let Some(start_ms) = starts[index] else {
                continue;
            };
            let end_ms = starts.iter().skip(index + 1).flatten().next().copied();
            sources.push(MediaSource {
                path: audio_path.to_path_buf(),
                cue: Some(CueTrackSource {
                    cue_path: cue_path.to_path_buf(),
                    track_number: track.no.parse().unwrap_or((index + 1) as u32),
                    title: clean(track.title),
                    performer: clean(track.performer.or_else(|| cue.performer.clone())),
                    album: clean(cue.title.clone()),
                    start_ms,
                    end_ms,
                }),
            });
        }
    }
    Ok(sources)
}

fn track_start(track: &rcue::cue::Track) -> Option<u64> {
    track
        .indices
        .iter()
        .find(|(index, _)| index == "01")
        .or_else(|| track.indices.first())
        .map(|(_, duration)| duration_ms(*duration))
}

fn duration_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
}

fn decode_text(bytes: &[u8]) -> Result<Cow<'_, str>, String> {
    if let Some(bytes) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return std::str::from_utf8(bytes)
            .map(Cow::Borrowed)
            .map_err(|error| error.to_string());
    }
    if let Some(bytes) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        let (text, errors) = UTF_16LE.decode_without_bom_handling(bytes);
        return (!errors)
            .then_some(text)
            .ok_or_else(|| "invalid UTF-16LE CUE".to_owned());
    }
    if let Some(bytes) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        let (text, errors) = UTF_16BE.decode_without_bom_handling(bytes);
        return (!errors)
            .then_some(text)
            .ok_or_else(|| "invalid UTF-16BE CUE".to_owned());
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return Ok(Cow::Borrowed(text));
    }
    let (text, errors) = GBK.decode_without_bom_handling(bytes);
    (!errors)
        .then_some(text)
        .ok_or_else(|| "unsupported CUE encoding".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_matching_cue_into_bounded_tracks() {
        let root = std::env::temp_dir().join(format!("resona-cue-{}", fastrand::u64(..)));
        fs::create_dir_all(&root).expect("create fixture directory");
        let audio = root.join("album.flac");
        fs::write(&audio, []).expect("create audio");
        fs::write(
            root.join("ALBUM.CUE"),
            concat!(
                "PERFORMER \"Artist\"\n",
                "TITLE \"Album\"\n",
                "FILE \"album.flac\" WAVE\n",
                "  TRACK 01 AUDIO\n",
                "    TITLE \"First\"\n",
                "    INDEX 01 00:00:00\n",
                "  TRACK 02 AUDIO\n",
                "    TITLE \"Second\"\n",
                "    INDEX 01 03:10:15\n",
            ),
        )
        .expect("write CUE");

        let tracks = expand_same_name_cue(&audio);
        assert_eq!(tracks.len(), 2);
        assert_eq!(
            tracks[0].cue.as_ref().and_then(|cue| cue.end_ms),
            Some(190_200)
        );
        assert_eq!(tracks[1].display_name(), "Second");
        let _ = fs::remove_dir_all(root);
    }
}
