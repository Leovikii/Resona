// SPDX-License-Identifier: GPL-3.0-only

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use base64::Engine;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader};
use lofty::config::ParseOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use serde::Serialize;
use sha2::{Digest, Sha256};

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
    pub genre: Option<String>,
    pub track_number: Option<u32>,
    pub track_total: Option<u32>,
    pub disc_number: Option<u32>,
    pub disc_total: Option<u32>,
    pub date: Option<String>,
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
            genre: None,
            track_number: None,
            track_total: None,
            disc_number: None,
            disc_total: None,
            date: None,
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

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackSummary {
    pub path: String,
    pub title: Option<String>,
    pub track_number: Option<u32>,
    pub duration_ms: Option<u64>,
    pub metadata_warning: Option<String>,
}

const MAX_CACHE_ENTRIES: usize = 4;
const MAX_SUMMARY_CACHE_ENTRIES: usize = 256;
pub const MAX_SUMMARY_BATCH_SIZE: usize = 64;
const MAX_ARTWORK_CACHE_ENTRIES: usize = 8;
const MAX_ARTWORK_BYTES: usize = 16 * 1024 * 1024;
const MAX_ARTWORK_DIMENSION: u32 = 8_192;
const MAX_DECODED_ARTWORK_BYTES: u64 = 64 * 1024 * 1024;
const CACHED_ARTWORK_DIMENSION: u32 = 512;
const MAX_ARTWORK_DIRECTORY_ENTRIES: usize = 128;

#[derive(Clone, Debug)]
pub struct Artwork {
    pub fingerprint: [u8; 32],
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

    pub fn fingerprint_hex(&self) -> String {
        let mut value = String::with_capacity(self.fingerprint.len() * 2);
        for byte in self.fingerprint {
            write!(&mut value, "{byte:02x}").expect("writing to a string cannot fail");
        }
        value
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
    fallback_artwork: Option<(PathBuf, u64, Option<SystemTime>)>,
}

#[derive(Clone)]
struct CacheEntry {
    revision: FileRevision,
    metadata: CachedTrackMetadata,
}

#[derive(Clone)]
struct SummaryCacheEntry {
    revision: FileRevision,
    summary: TrackSummary,
}

#[derive(Clone)]
struct ArtworkCacheEntry {
    source_fingerprint: [u8; 32],
    artwork: Arc<Artwork>,
}

#[derive(Default)]
pub struct MetadataService {
    cache: Mutex<VecDeque<CacheEntry>>,
    summary_cache: Mutex<VecDeque<SummaryCacheEntry>>,
    artwork_cache: Mutex<VecDeque<ArtworkCacheEntry>>,
}

impl MetadataService {
    pub fn load(&self, path: &Path) -> CachedTrackMetadata {
        let revision = file_revision(path, true);
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

        let metadata = read_uncached_track_metadata(self, path, true);
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

    pub fn load_info(&self, path: &Path) -> TrackDetails {
        read_uncached_track_metadata(self, path, false).details
    }

    pub fn load_summaries(&self, paths: &[PathBuf]) -> Vec<TrackSummary> {
        paths.iter().map(|path| self.load_summary(path)).collect()
    }

    fn load_summary(&self, path: &Path) -> TrackSummary {
        let revision = file_revision(path, false);
        let mut cache = self
            .summary_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(index) = cache.iter().position(|entry| entry.revision == revision) {
            let entry = cache.remove(index).expect("summary cache index exists");
            let summary = entry.summary.clone();
            cache.push_back(entry);
            return summary;
        }
        drop(cache);

        let summary = read_track_summary(path);
        let mut cache = self
            .summary_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        cache.retain(|entry| entry.revision.path != revision.path);
        cache.push_back(SummaryCacheEntry {
            revision,
            summary: summary.clone(),
        });
        while cache.len() > MAX_SUMMARY_CACHE_ENTRIES {
            cache.pop_front();
        }
        summary
    }

    fn load_artwork(&self, mime_type: &str, bytes: &[u8]) -> Result<Arc<Artwork>, String> {
        if bytes.len() > MAX_ARTWORK_BYTES {
            return Err(format!("artwork exceeds {MAX_ARTWORK_BYTES} bytes"));
        }
        let source_fingerprint = fingerprint(bytes);
        let mut cache = self
            .artwork_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(artwork) = cached_artwork(&mut cache, source_fingerprint) {
            return Ok(artwork);
        }
        drop(cache);

        let artwork = decode_artwork(mime_type, bytes)?;
        let mut cache = self
            .artwork_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(shared) = cached_artwork(&mut cache, source_fingerprint) {
            return Ok(shared);
        }
        cache.push_back(ArtworkCacheEntry {
            source_fingerprint,
            artwork: Arc::clone(&artwork),
        });
        while cache.len() > MAX_ARTWORK_CACHE_ENTRIES {
            cache.pop_front();
        }
        Ok(artwork)
    }
}

fn cached_artwork(
    cache: &mut VecDeque<ArtworkCacheEntry>,
    source_fingerprint: [u8; 32],
) -> Option<Arc<Artwork>> {
    let index = cache
        .iter()
        .position(|entry| entry.source_fingerprint == source_fingerprint)?;
    let entry = cache.remove(index).expect("artwork cache index exists");
    let artwork = Arc::clone(&entry.artwork);
    cache.push_back(entry);
    Some(artwork)
}

#[cfg(test)]
fn read_track_details(path: &Path) -> TrackDetails {
    MetadataService::default().load(path).details
}

pub fn read_artwork_file(path: &Path) -> Result<Arc<Artwork>, String> {
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    decode_artwork("image/png", &bytes)
}

fn read_uncached_track_metadata(
    service: &MetadataService,
    path: &Path,
    include_artwork: bool,
) -> CachedTrackMetadata {
    let tagged_file = match Probe::open(path).and_then(|probe| {
        probe
            .options(ParseOptions::new().read_cover_art(include_artwork))
            .read()
    }) {
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
    let embedded_artwork = include_artwork
        .then_some(tag)
        .flatten()
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
            match service.load_artwork(mime, picture.data()) {
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
    let artwork = if include_artwork && embedded_artwork.is_none() {
        fallback_artwork_candidate(path).and_then(|candidate| {
            match std::fs::read(&candidate.path) {
                Ok(bytes) => match service.load_artwork("application/octet-stream", &bytes) {
                    Ok(artwork) => Some(artwork),
                    Err(error) => {
                        log::warn!(
                            "fallback artwork decode failed for {}: {error}",
                            candidate.path.display()
                        );
                        None
                    }
                },
                Err(error) => {
                    log::warn!(
                        "fallback artwork read failed for {}: {error}",
                        candidate.path.display()
                    );
                    None
                }
            }
        })
    } else {
        embedded_artwork
    };
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
        genre: tag.and_then(|value| value.genre().map(|text| text.into_owned())),
        track_number: tag.and_then(Accessor::track),
        track_total: tag.and_then(Accessor::track_total),
        disc_number: tag.and_then(Accessor::disk),
        disc_total: tag.and_then(Accessor::disk_total),
        date: tag.and_then(|value| value.date().map(|date| date.to_string())),
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

fn read_track_summary(path: &Path) -> TrackSummary {
    let tagged_file = Probe::open(path).and_then(|probe| {
        probe
            .options(
                ParseOptions::new()
                    .read_properties(true)
                    .read_cover_art(false),
            )
            .read()
    });
    let metadata_warning = tagged_file.as_ref().err().map(ToString::to_string);
    let tag = tagged_file
        .as_ref()
        .ok()
        .and_then(|file| file.primary_tag().or_else(|| file.tags().first()));
    let duration_ms = tagged_file.as_ref().ok().map(|file| {
        file.properties()
            .duration()
            .as_millis()
            .min(u128::from(u64::MAX)) as u64
    });
    TrackSummary {
        path: path.to_string_lossy().into_owned(),
        title: tag.and_then(|value| value.title().map(|text| text.into_owned())),
        track_number: tag.and_then(Accessor::track),
        duration_ms,
        metadata_warning,
    }
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
        fingerprint: fingerprint(&encoded),
        mime_type: "image/png".to_owned(),
        encoded: Arc::from(encoded),
        bgra: Arc::from(bgra),
        width,
        height,
    }))
}

fn fingerprint(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn file_revision(path: &Path, include_artwork: bool) -> FileRevision {
    let metadata = std::fs::metadata(path).ok();
    let fallback_artwork = include_artwork
        .then(|| fallback_artwork_candidate(path))
        .flatten()
        .and_then(|candidate| {
            let metadata = std::fs::metadata(&candidate.path).ok()?;
            Some((candidate.path, metadata.len(), metadata.modified().ok()))
        });
    FileRevision {
        path: path.to_path_buf(),
        size: metadata.as_ref().map(std::fs::Metadata::len),
        modified: metadata.and_then(|value| value.modified().ok()),
        fallback_artwork,
    }
}

#[derive(Clone, Debug)]
struct ArtworkCandidate {
    path: PathBuf,
    width: u32,
    height: u32,
    size: u64,
}

fn fallback_artwork_candidate(audio_path: &Path) -> Option<ArtworkCandidate> {
    let directory = audio_path.parent()?;
    let mut candidates = std::fs::read_dir(directory)
        .ok()?
        .take(MAX_ARTWORK_DIRECTORY_ENTRIES)
        .filter_map(Result::ok)
        .filter_map(|entry| inspect_artwork_candidate(entry.path()))
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        left.path
            .file_name()
            .map(|value| value.to_string_lossy().to_lowercase())
            .cmp(
                &right
                    .path
                    .file_name()
                    .map(|value| value.to_string_lossy().to_lowercase()),
            )
            .then_with(|| left.path.cmp(&right.path))
    });
    if let Some(candidate) = candidates.iter().find(|candidate| {
        candidate
            .path
            .file_stem()
            .is_some_and(|stem| stem.to_string_lossy().eq_ignore_ascii_case("cover"))
    }) {
        return Some(candidate.clone());
    }
    if candidates.len() == 1 {
        return candidates.pop();
    }
    candidates.into_iter().min_by_key(|candidate| {
        let longest = u64::from(candidate.width.max(candidate.height).max(1));
        let aspect_error = u64::from(candidate.width.abs_diff(candidate.height)) * 10_000 / longest;
        (
            aspect_error,
            u64::from(candidate.width) * u64::from(candidate.height),
            candidate.size,
        )
    })
}

fn inspect_artwork_candidate(path: PathBuf) -> Option<ArtworkCandidate> {
    let extension = path.extension()?.to_str()?;
    if !matches!(
        extension.to_ascii_lowercase().as_str(),
        "jpg" | "jpeg" | "png"
    ) {
        return None;
    }
    let metadata = std::fs::metadata(&path).ok()?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_ARTWORK_BYTES as u64 {
        return None;
    }
    let reader = ImageReader::open(&path).ok()?.with_guessed_format().ok()?;
    let (width, height) = reader.into_dimensions().ok()?;
    if width == 0
        || height == 0
        || width > MAX_ARTWORK_DIMENSION
        || height > MAX_ARTWORK_DIMENSION
        || u64::from(width) * u64::from(height) * 4 > MAX_DECODED_ARTWORK_BYTES
    {
        return None;
    }
    Some(ArtworkCandidate {
        path,
        width,
        height,
        size: metadata.len(),
    })
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

    fn png_with_color(color: u8) -> Vec<u8> {
        let source = image::RgbaImage::from_pixel(2, 2, image::Rgba([color, 0, 0, 255]));
        let mut bytes = Vec::new();
        DynamicImage::ImageRgba8(source)
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .expect("encode test artwork");
        bytes
    }

    fn write_png(path: &Path, width: u32, height: u32, color: u8) {
        let source = image::RgbaImage::from_pixel(width, height, image::Rgba([color, 0, 0, 255]));
        source.save(path).expect("write artwork fixture");
    }

    #[test]
    fn fallback_artwork_prefers_cover_then_unique_then_small_square() {
        let root = std::env::temp_dir().join(format!("resona-artwork-{}", fastrand::u64(..)));
        std::fs::create_dir_all(&root).expect("create artwork fixture directory");
        let audio = root.join("album.flac");
        std::fs::write(&audio, []).expect("create audio fixture");

        write_png(&root.join("only.png"), 320, 200, 1);
        assert_eq!(
            fallback_artwork_candidate(&audio)
                .and_then(|candidate| candidate.path.file_name().map(|name| name.to_owned())),
            Some("only.png".into())
        );

        write_png(&root.join("folder.png"), 100, 100, 2);
        write_png(&root.join("large-square.png"), 500, 500, 3);
        assert_eq!(
            fallback_artwork_candidate(&audio)
                .and_then(|candidate| candidate.path.file_name().map(|name| name.to_owned())),
            Some("folder.png".into())
        );

        write_png(&root.join("CoVeR.PNG"), 600, 400, 4);
        assert_eq!(
            fallback_artwork_candidate(&audio)
                .and_then(|candidate| candidate.path.file_name().map(|name| name.to_owned())),
            Some("CoVeR.PNG".into())
        );
        let _ = std::fs::remove_dir_all(root);
    }

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
    fn lightweight_summary_reads_duration_without_artwork() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("tests/fixtures/audio/flac_44100_16_stereo.flac");
        let summary = MetadataService::default().load_summary(&path);
        assert!(summary.duration_ms.is_some_and(|duration| duration > 0));
        assert!(summary.metadata_warning.is_none());
    }

    #[test]
    fn unreadable_file_returns_diagnostic_fallback() {
        let metadata = read_track_details(Path::new("missing.wav"));
        assert_eq!(metadata.file_name, "missing.wav");
        assert!(metadata.metadata_warning.is_some());
    }

    #[test]
    fn lightweight_summary_cache_is_bounded_and_omits_artwork_work() {
        let service = MetadataService::default();
        for index in 0..=MAX_SUMMARY_CACHE_ENTRIES {
            let path = PathBuf::from(format!("missing-summary-{index}.flac"));
            let summary = service.load_summary(&path);
            assert_eq!(summary.path, path.to_string_lossy());
            assert_eq!(summary.title, None);
            assert!(summary.metadata_warning.is_some());
        }
        assert_eq!(
            service.summary_cache.lock().expect("summary cache").len(),
            MAX_SUMMARY_CACHE_ENTRIES
        );
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
        assert_eq!(artwork.fingerprint, fingerprint(&artwork.encoded));
    }

    #[test]
    fn identical_source_artwork_reuses_the_normalized_result() {
        let service = MetadataService::default();
        let bytes = png_with_color(42);

        let first = service
            .load_artwork("image/png", &bytes)
            .expect("decode first artwork");
        let second = service
            .load_artwork("image/png", &bytes)
            .expect("reuse artwork");

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(service.artwork_cache.lock().expect("cache").len(), 1);
    }

    #[test]
    fn source_artwork_cache_is_bounded_and_evicts_least_recently_used() {
        let service = MetadataService::default();
        let first_bytes = png_with_color(0);
        let first = service
            .load_artwork("image/png", &first_bytes)
            .expect("decode first artwork");
        for color in 1..=MAX_ARTWORK_CACHE_ENTRIES as u8 {
            service
                .load_artwork("image/png", &png_with_color(color))
                .expect("decode unique artwork");
        }

        assert_eq!(
            service.artwork_cache.lock().expect("cache").len(),
            MAX_ARTWORK_CACHE_ENTRIES
        );
        let reloaded = service
            .load_artwork("image/png", &first_bytes)
            .expect("reload evicted artwork");
        assert!(!Arc::ptr_eq(&first, &reloaded));
    }
}
