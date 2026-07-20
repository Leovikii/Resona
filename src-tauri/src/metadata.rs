// SPDX-License-Identifier: GPL-3.0-only

use std::path::{Path, PathBuf};

use base64::Engine;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::tag::Accessor;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackDetails {
    pub path: String,
    pub file_name: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub sample_rate: Option<u32>,
    pub bit_depth: Option<u8>,
    pub channels: Option<u8>,
    pub audio_bitrate: Option<u32>,
    pub codec: String,
    pub file_size: Option<u64>,
    pub artwork_data_url: Option<String>,
    pub metadata_warning: Option<String>,
}

impl TrackDetails {
    fn fallback(path: &Path, warning: impl Into<String>) -> Self {
        Self {
            path: path.to_string_lossy().into_owned(),
            file_name: file_name(path),
            title: None,
            artist: None,
            album: None,
            duration_ms: None,
            sample_rate: None,
            bit_depth: None,
            channels: None,
            audio_bitrate: None,
            codec: codec_name(path),
            file_size: std::fs::metadata(path).ok().map(|value| value.len()),
            artwork_data_url: None,
            metadata_warning: Some(warning.into()),
        }
    }
}

pub fn read_track_details(path: &Path) -> TrackDetails {
    let tagged_file = match lofty::read_from_path(path) {
        Ok(value) => value,
        Err(error) => return TrackDetails::fallback(path, error.to_string()),
    };
    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.tags().first());
    let properties = tagged_file.properties();
    let artwork_data_url = tag
        .and_then(|value| value.pictures().first())
        .map(|picture| {
            let mime = picture
                .mime_type()
                .map_or("application/octet-stream", |value| value.as_str());
            format!(
                "data:{mime};base64,{}",
                base64::engine::general_purpose::STANDARD.encode(picture.data())
            )
        });
    TrackDetails {
        path: path.to_string_lossy().into_owned(),
        file_name: file_name(path),
        title: tag.and_then(|value| value.title().map(|text| text.into_owned())),
        artist: tag.and_then(|value| value.artist().map(|text| text.into_owned())),
        album: tag.and_then(|value| value.album().map(|text| text.into_owned())),
        duration_ms: Some(properties.duration().as_millis().min(u128::from(u64::MAX)) as u64),
        sample_rate: properties.sample_rate(),
        bit_depth: properties.bit_depth(),
        channels: properties.channels(),
        audio_bitrate: properties.audio_bitrate(),
        codec: codec_name(path),
        file_size: std::fs::metadata(path).ok().map(|value| value.len()),
        artwork_data_url,
        metadata_warning: None,
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

fn codec_name(path: &Path) -> String {
    path.extension()
        .and_then(|value| value.to_str())
        .map(str::to_uppercase)
        .unwrap_or_else(|| "Unknown".to_owned())
}

pub fn normalized_path(path: String) -> PathBuf {
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_supported_fixture_properties() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("tests/fixtures/audio/flac_44100_16_stereo.flac");
        let metadata = read_track_details(&path);
        assert_eq!(metadata.sample_rate, Some(44_100));
        assert_eq!(metadata.bit_depth, Some(16));
        assert!(metadata.duration_ms.is_some());
        assert!(metadata.metadata_warning.is_none());
    }

    #[test]
    fn unreadable_file_returns_diagnostic_fallback() {
        let metadata = read_track_details(Path::new("missing.wav"));
        assert_eq!(metadata.file_name, "missing.wav");
        assert!(metadata.metadata_warning.is_some());
    }
}
