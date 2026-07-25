// SPDX-License-Identifier: GPL-3.0-only

use std::collections::VecDeque;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use base64::Engine;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::tag::Accessor;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioQuality {
    HiRes,
    Sq,
    Hq,
}

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
    pub quality: Option<AudioQuality>,
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
            quality: None,
        }
    }
}

const MAX_CACHE_ENTRIES: usize = 4;
const MAX_ARTWORK_BYTES: usize = 16 * 1024 * 1024;
const MAX_ARTWORK_DIMENSION: u32 = 8_192;
const MAX_DECODED_ARTWORK_BYTES: u64 = 64 * 1024 * 1024;
const CACHED_ARTWORK_DIMENSION: u32 = 512;

#[derive(Clone, Debug)]
pub struct Artwork {
    pub mime_type: String,
    pub encoded: Arc<[u8]>,
    pub bgra: Arc<[u8]>,
    pub width: u32,
    pub height: u32,
}

impl Artwork {
    pub fn data_url(&self) -> String {
        format!(
            "data:{};base64,{}",
            self.mime_type,
            base64::engine::general_purpose::STANDARD.encode(&self.encoded)
        )
    }
}

#[derive(Clone)]
pub struct CachedTrackMetadata {
    pub details: TrackDetails,
    pub artwork: Option<Arc<Artwork>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FileRevision {
    path: PathBuf,
    size: Option<u64>,
    modified: Option<SystemTime>,
}

#[derive(Clone)]
struct CacheEntry {
    revision: FileRevision,
    metadata: CachedTrackMetadata,
}

#[derive(Default)]
pub struct MetadataService {
    cache: Mutex<VecDeque<CacheEntry>>,
}

impl MetadataService {
    pub fn load(&self, path: &Path) -> CachedTrackMetadata {
        let revision = file_revision(path);
        let mut cache = self
            .cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(index) = cache.iter().position(|entry| entry.revision == revision) {
            let entry = cache.remove(index).expect("cache index exists");
            let metadata = entry.metadata.clone();
            cache.push_back(entry);
            return metadata;
        }
        drop(cache);

        let metadata = read_uncached_track_metadata(path);
        let mut cache = self
            .cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        cache.retain(|entry| entry.revision.path != revision.path);
        cache.push_back(CacheEntry {
            revision,
            metadata: metadata.clone(),
        });
        while cache.len() > MAX_CACHE_ENTRIES {
            cache.pop_front();
        }
        metadata
    }
}

#[cfg(test)]
fn read_track_details(path: &Path) -> TrackDetails {
    MetadataService::default().load(path).details
}

pub fn read_artwork_file(path: &Path) -> Result<Arc<Artwork>, String> {
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    decode_artwork("image/png", &bytes)
}

fn read_uncached_track_metadata(path: &Path) -> CachedTrackMetadata {
    let tagged_file = match lofty::read_from_path(path) {
        Ok(value) => value,
        Err(error) => {
            return CachedTrackMetadata {
                details: TrackDetails::fallback(path, error.to_string()),
                artwork: None,
            }
        }
    };
    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.tags().first());
    let properties = tagged_file.properties();
    let artwork = tag
        .and_then(|value| value.pictures().first())
        .and_then(|picture| {
            if picture.data().len() > MAX_ARTWORK_BYTES {
                eprintln!(
                    "embedded artwork exceeds the {} byte limit: {}",
                    MAX_ARTWORK_BYTES,
                    path.display()
                );
                return None;
            }
            let mime = picture
                .mime_type()
                .map_or("application/octet-stream", |value| value.as_str());
            match decode_artwork(mime, picture.data()) {
                Ok(artwork) => Some(artwork),
                Err(error) => {
                    eprintln!(
                        "embedded artwork decode failed for {}: {error}",
                        path.display()
                    );
                    None
                }
            }
        });
    let sample_rate = properties.sample_rate();
    let bit_depth = properties.bit_depth();
    let audio_bitrate = properties.audio_bitrate();
    let codec = codec_name(path);
    let details = TrackDetails {
        path: path.to_string_lossy().into_owned(),
        file_name: file_name(path),
        title: tag.and_then(|value| value.title().map(|text| text.into_owned())),
        artist: tag.and_then(|value| value.artist().map(|text| text.into_owned())),
        album: tag.and_then(|value| value.album().map(|text| text.into_owned())),
        duration_ms: Some(properties.duration().as_millis().min(u128::from(u64::MAX)) as u64),
        sample_rate,
        bit_depth,
        channels: properties.channels(),
        audio_bitrate,
        quality: classify_quality(&codec, sample_rate, bit_depth, audio_bitrate),
        codec,
        file_size: std::fs::metadata(path).ok().map(|value| value.len()),
        artwork_data_url: artwork.as_ref().map(|value| value.data_url()),
        metadata_warning: None,
    };
    CachedTrackMetadata { details, artwork }
}

fn decode_artwork(_mime_type: &str, bytes: &[u8]) -> Result<Arc<Artwork>, String> {
    if bytes.len() > MAX_ARTWORK_BYTES {
        return Err(format!("artwork exceeds {MAX_ARTWORK_BYTES} bytes"));
    }
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|error| error.to_string())?;
    let decoder = reader.into_decoder().map_err(|error| error.to_string())?;
    let (width, height) = decoder.dimensions();
    if width == 0
        || height == 0
        || width > MAX_ARTWORK_DIMENSION
        || height > MAX_ARTWORK_DIMENSION
        || u64::from(width) * u64::from(height) * 4 > MAX_DECODED_ARTWORK_BYTES
    {
        return Err(format!("unsupported artwork dimensions {width}x{height}"));
    }
    let decoded = DynamicImage::from_decoder(decoder).map_err(|error| error.to_string())?;
    let decoded = if width > CACHED_ARTWORK_DIMENSION || height > CACHED_ARTWORK_DIMENSION {
        decoded.thumbnail(CACHED_ARTWORK_DIMENSION, CACHED_ARTWORK_DIMENSION)
    } else {
        decoded
    };
    let rgba = decoded.into_rgba8();
    let (width, height) = rgba.dimensions();
    let mut encoded = Vec::new();
    DynamicImage::ImageRgba8(rgba.clone())
        .write_to(&mut Cursor::new(&mut encoded), ImageFormat::Png)
        .map_err(|error| error.to_string())?;
    let mut bgra = rgba.into_raw();
    for pixel in bgra.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    Ok(Arc::new(Artwork {
        mime_type: "image/png".to_owned(),
        encoded: Arc::from(encoded),
        bgra: Arc::from(bgra),
        width,
        height,
    }))
}

fn file_revision(path: &Path) -> FileRevision {
    let metadata = std::fs::metadata(path).ok();
    FileRevision {
        path: path.to_path_buf(),
        size: metadata.as_ref().map(std::fs::Metadata::len),
        modified: metadata.and_then(|value| value.modified().ok()),
    }
}

fn classify_quality(
    codec: &str,
    sample_rate: Option<u32>,
    bit_depth: Option<u8>,
    bitrate: Option<u32>,
) -> Option<AudioQuality> {
    match codec {
        "WAV" | "FLAC"
            if sample_rate.unwrap_or_default() >= 88_200 && bit_depth.unwrap_or_default() >= 24 =>
        {
            Some(AudioQuality::HiRes)
        }
        "WAV" | "FLAC" => Some(AudioQuality::Sq),
        "MP3" if bitrate.unwrap_or_default() >= 256 => Some(AudioQuality::Hq),
        _ => None,
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

    #[test]
    fn classifies_quality_with_explicit_precedence() {
        assert_eq!(
            classify_quality("FLAC", Some(96_000), Some(24), None),
            Some(AudioQuality::HiRes)
        );
        assert_eq!(
            classify_quality("WAV", Some(44_100), Some(16), None),
            Some(AudioQuality::Sq)
        );
        assert_eq!(
            classify_quality("MP3", Some(44_100), None, Some(320)),
            Some(AudioQuality::Hq)
        );
        assert_eq!(classify_quality("MP3", Some(44_100), None, Some(192)), None);
    }

    #[test]
    fn artwork_is_normalized_to_a_bounded_png() {
        let source = DynamicImage::new_rgb8(1_024, 768);
        let mut bytes = Vec::new();
        source
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .expect("encode source image");

        let artwork = decode_artwork("image/jpeg", &bytes).expect("decode artwork");

        assert_eq!(artwork.mime_type, "image/png");
        assert!(artwork.encoded.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(artwork.width <= CACHED_ARTWORK_DIMENSION);
        assert!(artwork.height <= CACHED_ARTWORK_DIMENSION);
        assert_eq!(
            artwork.bgra.len(),
            (artwork.width * artwork.height * 4) as usize
        );
    }
}
