// SPDX-License-Identifier: GPL-3.0-only

use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use encoding_rs::{GBK, UTF_16BE, UTF_16LE};
use lrc::Lyrics as ParsedLrc;
use serde::Serialize;
use subtp::srt::{SrtTimestamp, SubRip};
use subtp::vtt::{VttBlock, VttTimestamp, WebVtt};
use thiserror::Error;

use crate::playback::{PlaybackSnapshot, PlaybackStatus};

const FORMAT_PRIORITY: [(LyricsFormat, &str); 3] = [
    (LyricsFormat::Lrc, "lrc"),
    (LyricsFormat::Srt, "srt"),
    (LyricsFormat::Vtt, "vtt"),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LyricsFormat {
    Lrc,
    Srt,
    Vtt,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LyricsStatus {
    #[default]
    Idle,
    Missing,
    Empty,
    Ready,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsLine {
    pub start_ms: u64,
    pub end_ms: Option<u64>,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsDocument {
    pub source_path: String,
    pub format: LyricsFormat,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub warning_count: u32,
    pub lines: Vec<LyricsLine>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LyricsErrorCode {
    Read,
    DecodeText,
    Parse,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LyricsFailure {
    pub code: LyricsErrorCode,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsSnapshot {
    pub revision: u64,
    pub audio_path: Option<String>,
    pub status: LyricsStatus,
    pub document: Option<LyricsDocument>,
    pub active_line_index: Option<usize>,
    pub error: Option<LyricsFailure>,
}

#[derive(Debug, Error)]
enum LyricsError {
    #[error("无法读取歌词文件：{0}")]
    Read(String),
    #[error("无法识别歌词文件编码")]
    DecodeText,
    #[error("无法解析歌词文件：{0}")]
    Parse(String),
}

impl LyricsError {
    fn failure(&self) -> LyricsFailure {
        let code = match self {
            Self::Read(_) => LyricsErrorCode::Read,
            Self::DecodeText => LyricsErrorCode::DecodeText,
            Self::Parse(_) => LyricsErrorCode::Parse,
        };
        LyricsFailure {
            code,
            message: self.to_string(),
        }
    }
}

#[derive(Default)]
pub struct LyricsService {
    cache: Mutex<LyricsCache>,
}

#[derive(Default)]
struct LyricsCache {
    audio_path: Option<PathBuf>,
    revision: u64,
    loaded: LoadedLyrics,
}

#[derive(Default)]
enum LoadedLyrics {
    #[default]
    Idle,
    Missing,
    Document(LyricsDocument),
    Failed(LyricsFailure),
}

impl LyricsService {
    pub fn snapshot(
        &self,
        playback: &PlaybackSnapshot,
        known_revision: Option<u64>,
    ) -> LyricsSnapshot {
        let audio_path = playback.path.as_deref().map(PathBuf::from);
        let mut cache = self
            .cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        if cache.audio_path != audio_path {
            cache.audio_path = audio_path.clone();
            cache.revision = cache.revision.wrapping_add(1).max(1);
            cache.loaded = audio_path
                .as_deref()
                .map(load_for_audio)
                .unwrap_or(LoadedLyrics::Idle);
        }

        let include_document = known_revision != Some(cache.revision);
        let can_have_active_line = matches!(
            playback.status,
            PlaybackStatus::Playing | PlaybackStatus::Paused
        );
        let (status, document, active_line_index, error) = match &cache.loaded {
            LoadedLyrics::Idle => (LyricsStatus::Idle, None, None, None),
            LoadedLyrics::Missing => (LyricsStatus::Missing, None, None, None),
            LoadedLyrics::Failed(failure) => {
                (LyricsStatus::Failed, None, None, Some(failure.clone()))
            }
            LoadedLyrics::Document(document) if document.lines.is_empty() => (
                LyricsStatus::Empty,
                include_document.then(|| document.clone()),
                None,
                None,
            ),
            LoadedLyrics::Document(document) => {
                let active = can_have_active_line
                    .then(|| active_line_index(&document.lines, playback.position_ms))
                    .flatten();
                (
                    LyricsStatus::Ready,
                    include_document.then(|| document.clone()),
                    active,
                    None,
                )
            }
        };

        LyricsSnapshot {
            revision: cache.revision,
            audio_path: cache
                .audio_path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            status,
            document,
            active_line_index,
            error,
        }
    }
}

fn load_for_audio(audio_path: &Path) -> LoadedLyrics {
    let Some((lyrics_path, format)) = discover_sidecar(audio_path) else {
        return LoadedLyrics::Missing;
    };
    match read_document(&lyrics_path, format) {
        Ok(document) => LoadedLyrics::Document(document),
        Err(error) => LoadedLyrics::Failed(error.failure()),
    }
}

fn discover_sidecar(audio_path: &Path) -> Option<(PathBuf, LyricsFormat)> {
    let parent = audio_path.parent()?;
    let audio_stem = audio_path.file_stem()?.to_string_lossy();
    let audio_file_name = audio_path.file_name()?.to_string_lossy();
    let mut entries = fs::read_dir(parent)
        .ok()?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name().to_string_lossy().to_ascii_lowercase());

    for (format, extension) in FORMAT_PRIORITY {
        let path = entries
            .iter()
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file()
                    && path.extension().is_some_and(|value| {
                        value.to_string_lossy().eq_ignore_ascii_case(extension)
                    })
            })
            .filter_map(|path| {
                let lyrics_stem = path.file_stem()?.to_string_lossy();
                sidecar_match_rank(&audio_stem, &audio_file_name, &lyrics_stem)
                    .map(|rank| (rank, path))
            })
            .min_by_key(|(rank, _)| *rank)
            .map(|(_, path)| path);
        if let Some(path) = path {
            return Some((path, format));
        }
    }
    None
}

fn sidecar_match_rank(audio_stem: &str, audio_file_name: &str, lyrics_stem: &str) -> Option<u8> {
    if lyrics_stem.eq_ignore_ascii_case(audio_stem) {
        return Some(0);
    }
    if lyrics_stem.eq_ignore_ascii_case(audio_file_name) {
        return Some(1);
    }

    let qualifier = lyrics_stem.get(audio_stem.len()..)?;
    (lyrics_stem
        .get(..audio_stem.len())?
        .eq_ignore_ascii_case(audio_stem)
        && qualifier.starts_with('.')
        && qualifier.len() > 1)
        .then_some(2)
}

fn read_document(path: &Path, format: LyricsFormat) -> Result<LyricsDocument, LyricsError> {
    let bytes = fs::read(path).map_err(|error| LyricsError::Read(error.to_string()))?;
    let text = decode_text(&bytes)?;
    let mut document = match format {
        LyricsFormat::Lrc => parse_lrc(&text)?,
        LyricsFormat::Srt => parse_srt(&text)?,
        LyricsFormat::Vtt => parse_vtt(&text)?,
    };
    document.source_path = path.to_string_lossy().into_owned();
    document.format = format;
    Ok(document)
}

fn decode_text(bytes: &[u8]) -> Result<Cow<'_, str>, LyricsError> {
    if let Some(bytes) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return std::str::from_utf8(bytes)
            .map(Cow::Borrowed)
            .map_err(|_| LyricsError::DecodeText);
    }
    if let Some(bytes) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        let (text, had_errors) = UTF_16LE.decode_without_bom_handling(bytes);
        return (!had_errors).then_some(text).ok_or(LyricsError::DecodeText);
    }
    if let Some(bytes) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        let (text, had_errors) = UTF_16BE.decode_without_bom_handling(bytes);
        return (!had_errors).then_some(text).ok_or(LyricsError::DecodeText);
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return Ok(Cow::Borrowed(text));
    }
    let (text, had_errors) = GBK.decode_without_bom_handling(bytes);
    (!had_errors).then_some(text).ok_or(LyricsError::DecodeText)
}

fn parse_lrc(text: &str) -> Result<LyricsDocument, LyricsError> {
    let mut pending_lines = Vec::new();
    let mut title = None;
    let mut artist = None;
    let mut album = None;
    let mut offset_ms = 0_i64;
    let mut warning_count = 0_u32;

    for source_line in normalized_lines(text) {
        let source_line = source_line.trim();
        if source_line.is_empty() {
            continue;
        }
        let normalized = normalize_lrc_timestamp_precision(source_line);
        match ParsedLrc::from_str(normalized.as_ref()) {
            Ok(parsed) => {
                title = parsed.metadata.title().map(str::to_owned).or(title);
                artist = parsed.metadata.artist().map(str::to_owned).or(artist);
                album = parsed.metadata.album().map(str::to_owned).or(album);
                if let Some(offset) = parsed.metadata.get_text("offset") {
                    match offset.trim().parse::<i64>() {
                        Ok(value) => offset_ms = value,
                        Err(_) => warning_count = warning_count.saturating_add(1),
                    }
                }
                let timed_lines = parsed.get_timed_lines();
                pending_lines.extend(
                    timed_lines
                        .iter()
                        .map(|(timestamp, text)| (timestamp.get_timestamp(), text.to_string())),
                );
                if timed_lines.is_empty() && parsed.metadata.is_empty() {
                    warning_count = warning_count.saturating_add(1);
                }
            }
            Err(_) => warning_count = warning_count.saturating_add(1),
        }
    }

    let raw_lines = pending_lines
        .into_iter()
        .map(|(timestamp, text)| RawLine {
            start_ms: timestamp.saturating_add(offset_ms).max(0) as u64,
            end_ms: None,
            text,
        })
        .collect::<Vec<_>>();

    if raw_lines.is_empty() && warning_count > 0 {
        return Err(LyricsError::Parse("没有可用的 LRC 时间行".to_owned()));
    }
    Ok(LyricsDocument {
        source_path: String::new(),
        format: LyricsFormat::Lrc,
        title,
        artist,
        album,
        warning_count,
        lines: normalize_lines(raw_lines, true),
    })
}

fn parse_srt(text: &str) -> Result<LyricsDocument, LyricsError> {
    let normalized = normalize_newlines(text);
    let (subtitles, warning_count) =
        match SubRip::parse(ensure_trailing_newline(&normalized).as_ref()) {
            Ok(subrip) => (subrip.subtitles, 0),
            Err(_) => {
                let mut subtitles = Vec::new();
                let mut warnings = 0_u32;
                for block in split_blocks(&normalized) {
                    match SubRip::parse(ensure_trailing_newline(block).as_ref()) {
                        Ok(parsed) => subtitles.extend(parsed.subtitles),
                        Err(_) => warnings = warnings.saturating_add(1),
                    }
                }
                (subtitles, warnings)
            }
        };
    let raw_lines = subtitles
        .into_iter()
        .filter_map(|subtitle| {
            let start_ms = srt_timestamp_ms(subtitle.start);
            let end_ms = srt_timestamp_ms(subtitle.end);
            (end_ms > start_ms).then(|| RawLine {
                start_ms,
                end_ms: Some(end_ms),
                text: normalize_timed_text(&subtitle.text),
            })
        })
        .collect::<Vec<_>>();
    if raw_lines.is_empty() && warning_count > 0 {
        return Err(LyricsError::Parse("没有可用的 SRT cue".to_owned()));
    }
    Ok(document_for_subtitles(
        LyricsFormat::Srt,
        raw_lines,
        warning_count,
    ))
}

fn parse_vtt(text: &str) -> Result<LyricsDocument, LyricsError> {
    let normalized = normalize_newlines(text);
    let (blocks, warning_count) = match WebVtt::parse(ensure_trailing_newline(&normalized).as_ref())
    {
        Ok(webvtt) => (webvtt.blocks, 0),
        Err(_) => {
            let body = normalized
                .split_once("\n\n")
                .map_or(normalized.as_str(), |(_, body)| body);
            let mut blocks = Vec::new();
            let mut warnings = 0_u32;
            for block in split_blocks(body) {
                let candidate = format!("WEBVTT\n\n{}\n", block.trim());
                match WebVtt::parse(&candidate) {
                    Ok(parsed) => blocks.extend(parsed.blocks),
                    Err(_) => warnings = warnings.saturating_add(1),
                }
            }
            (blocks, warnings)
        }
    };
    let raw_lines = blocks
        .into_iter()
        .filter_map(|block| match block {
            VttBlock::Que(cue) => {
                let start_ms = vtt_timestamp_ms(cue.timings.start);
                let end_ms = vtt_timestamp_ms(cue.timings.end);
                (end_ms > start_ms).then(|| RawLine {
                    start_ms,
                    end_ms: Some(end_ms),
                    text: normalize_timed_text(&cue.payload),
                })
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if raw_lines.is_empty() && warning_count > 0 {
        return Err(LyricsError::Parse("没有可用的 WebVTT cue".to_owned()));
    }
    Ok(document_for_subtitles(
        LyricsFormat::Vtt,
        raw_lines,
        warning_count,
    ))
}

fn document_for_subtitles(
    format: LyricsFormat,
    raw_lines: Vec<RawLine>,
    warning_count: u32,
) -> LyricsDocument {
    LyricsDocument {
        source_path: String::new(),
        format,
        title: None,
        artist: None,
        album: None,
        warning_count,
        lines: normalize_lines(raw_lines, false),
    }
}

#[derive(Debug)]
struct RawLine {
    start_ms: u64,
    end_ms: Option<u64>,
    text: String,
}

fn normalize_lines(mut raw_lines: Vec<RawLine>, derive_end: bool) -> Vec<LyricsLine> {
    raw_lines.sort_by_key(|line| (line.start_ms, line.end_ms.unwrap_or(u64::MAX)));
    let mut lines: Vec<LyricsLine> = Vec::with_capacity(raw_lines.len());
    for raw in raw_lines {
        if let Some(existing) = lines
            .last_mut()
            .filter(|line| line.start_ms == raw.start_ms && line.end_ms == raw.end_ms)
        {
            if existing.text != raw.text && !raw.text.is_empty() {
                if !existing.text.is_empty() {
                    existing.text.push('\n');
                }
                existing.text.push_str(&raw.text);
            }
        } else {
            lines.push(LyricsLine {
                start_ms: raw.start_ms,
                end_ms: raw.end_ms,
                text: raw.text,
            });
        }
    }
    if derive_end {
        for index in 0..lines.len().saturating_sub(1) {
            lines[index].end_ms = Some(lines[index + 1].start_ms);
        }
    }
    lines
}

fn active_line_index(lines: &[LyricsLine], position_ms: u64) -> Option<usize> {
    lines
        .partition_point(|line| line.start_ms <= position_ms)
        .checked_sub(1)
}

fn normalized_lines(text: &str) -> impl Iterator<Item = &str> {
    text.split('\n').map(|line| line.trim_end_matches('\r'))
}

fn normalize_lrc_timestamp_precision(line: &str) -> Cow<'_, str> {
    let mut cursor = 0;
    let mut output = String::with_capacity(line.len());
    let mut changed = false;
    while let Some(relative_open) = line[cursor..].find('[') {
        let open = cursor + relative_open;
        let Some(relative_close) = line[open + 1..].find(']') else {
            break;
        };
        let close = open + 1 + relative_close;
        output.push_str(&line[cursor..=open]);
        let content = &line[open + 1..close];
        if let Some(normalized) = normalize_lrc_time_tag(content) {
            output.push_str(&normalized);
            changed = true;
        } else {
            output.push_str(content);
        }
        output.push(']');
        cursor = close + 1;
    }
    if !changed {
        return Cow::Borrowed(line);
    }
    output.push_str(&line[cursor..]);
    Cow::Owned(output)
}

fn normalize_lrc_time_tag(content: &str) -> Option<String> {
    let (minutes, remainder) = content.split_once(':')?;
    let (seconds, fraction) = remainder.split_once('.')?;
    if !minutes.chars().all(|value| value.is_ascii_digit())
        || !seconds.chars().all(|value| value.is_ascii_digit())
        || fraction.len() != 3
        || !fraction.chars().all(|value| value.is_ascii_digit())
    {
        return None;
    }
    Some(format!("{minutes}:{seconds}.{}", &fraction[..2]))
}

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn ensure_trailing_newline(text: &str) -> Cow<'_, str> {
    if text.ends_with('\n') {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(format!("{text}\n"))
    }
}

fn split_blocks(text: &str) -> impl Iterator<Item = &str> {
    text.split("\n\n")
        .map(str::trim)
        .filter(|block| !block.is_empty())
}

fn normalize_timed_text(lines: &[String]) -> String {
    let joined = lines.join("\n");
    let mut result = String::with_capacity(joined.len());
    let mut inside_tag = false;
    for character in joined.chars() {
        match character {
            '<' => inside_tag = true,
            '>' if inside_tag => inside_tag = false,
            _ if !inside_tag => result.push(character),
            _ => {}
        }
    }
    result
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .trim()
        .to_owned()
}

fn srt_timestamp_ms(timestamp: SrtTimestamp) -> u64 {
    timestamp.hours as u64 * 3_600_000
        + timestamp.minutes as u64 * 60_000
        + timestamp.seconds as u64 * 1_000
        + timestamp.milliseconds as u64
}

fn vtt_timestamp_ms(timestamp: VttTimestamp) -> u64 {
    timestamp.hours as u64 * 3_600_000
        + timestamp.minutes as u64 * 60_000
        + timestamp.seconds as u64 * 1_000
        + timestamp.milliseconds as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ID: AtomicU64 = AtomicU64::new(1);

    #[test]
    fn parses_lrc_metadata_multiple_timestamps_offset_and_bad_lines() {
        let document =
            parse_lrc(include_str!("../../tests/fixtures/lyrics/sample.lrc")).expect("parse LRC");
        assert_eq!(document.title.as_deref(), Some("Fixture Song"));
        assert_eq!(document.artist.as_deref(), Some("Resona Tests"));
        assert_eq!(document.warning_count, 0);
        assert_eq!(document.lines.len(), 3);
        assert_eq!(document.lines[0].start_ms, 1_100);
        assert_eq!(document.lines[0].end_ms, Some(3_350));
        assert_eq!(document.lines[2].start_ms, 5_100);

        let tolerant = parse_lrc("[00:01.00]Valid\n[not a tag").expect("keep valid LRC lines");
        assert_eq!(tolerant.warning_count, 1);
        assert_eq!(tolerant.lines.len(), 1);
    }

    #[test]
    fn parses_srt_multiline_and_keeps_source_cue_boundaries() {
        let document =
            parse_srt(include_str!("../../tests/fixtures/lyrics/sample.srt")).expect("parse SRT");
        assert_eq!(document.lines.len(), 2);
        assert_eq!(
            document.lines[0].text,
            "First subtitle line\ncontinues here"
        );
        assert_eq!(document.lines[0].end_ms, Some(2_500));
        assert_eq!(active_line_index(&document.lines, 500), None);
        assert_eq!(active_line_index(&document.lines, 1_500), Some(0));
        assert_eq!(active_line_index(&document.lines, 2_750), Some(0));
        assert_eq!(active_line_index(&document.lines, 20_000), Some(1));

        let tolerant = parse_srt(
            "1\n00:00:01,000 --> 00:00:02,000\nValid\n\nbad cue\n\n2\n00:00:03,000 --> 00:00:04,000\nStill valid\n",
        )
        .expect("keep valid SRT cues");
        assert_eq!(tolerant.lines.len(), 2);
        assert_eq!(tolerant.warning_count, 1);
    }

    #[test]
    fn parses_webvtt_and_reduces_markup_to_plain_text() {
        let document = parse_vtt(include_str!("../../tests/fixtures/lyrics/sample.vtt"))
            .expect("parse WebVTT");
        assert_eq!(document.lines.len(), 2);
        assert_eq!(document.lines[0].text, "First cue & voice");

        let tolerant = parse_vtt(
            "WEBVTT\n\n00:01.000 --> 00:02.000\nValid\n\nnot a cue\n\n00:03.000 --> 00:04.000\nStill valid\n",
        )
        .expect("keep valid WebVTT cues");
        assert_eq!(tolerant.lines.len(), 2);
        assert_eq!(tolerant.warning_count, 1);
    }

    #[test]
    fn decodes_gbk_when_utf8_is_invalid() {
        let (encoded, _, had_errors) = GBK.encode("[00:01.00]中文歌词");
        assert!(!had_errors);
        assert_eq!(
            decode_text(&encoded).expect("decode GBK"),
            "[00:01.00]中文歌词"
        );

        let mut with_bom = vec![0xFF, 0xFE];
        with_bom.extend(
            "[00:01.00]UTF-16 lyrics"
                .encode_utf16()
                .flat_map(u16::to_le_bytes),
        );
        assert_eq!(
            decode_text(&with_bom).expect("decode UTF-16 LE"),
            "[00:01.00]UTF-16 lyrics"
        );
    }

    #[test]
    fn discovery_prefers_lrc_then_srt_then_vtt_case_insensitively() {
        let root = temp_root();
        let audio = root.join("Track.flac");
        fs::write(&audio, []).expect("write audio marker");
        fs::write(root.join("Track.vtt"), "WEBVTT\n").expect("write VTT");
        assert_eq!(discover_sidecar(&audio).unwrap().1, LyricsFormat::Vtt);
        fs::write(root.join("Track.SRT"), "").expect("write SRT");
        let (path, format) = discover_sidecar(&audio).expect("discover SRT");
        assert_eq!(format, LyricsFormat::Srt);
        assert!(path
            .extension()
            .is_some_and(|value| value.to_string_lossy().eq_ignore_ascii_case("srt")));
        fs::write(root.join("Track.lrc"), "").expect("write LRC");
        assert_eq!(discover_sidecar(&audio).unwrap().1, LyricsFormat::Lrc);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn discovery_accepts_audio_format_and_language_qualifiers_without_prefix_collisions() {
        let root = temp_root();
        let flac = root.join("1.flac");
        let wav = root.join("1.wav");
        fs::write(&flac, []).expect("write FLAC marker");
        fs::write(&wav, []).expect("write WAV marker");
        fs::write(root.join("10.wav.vtt"), "WEBVTT\n").expect("write unrelated VTT");
        assert!(discover_sidecar(&flac).is_none());

        fs::write(root.join("1.wav.vtt"), "WEBVTT\n").expect("write format VTT");
        let (flac_sidecar, format) = discover_sidecar(&flac).expect("match qualified FLAC lyrics");
        assert_eq!(format, LyricsFormat::Vtt);
        assert_eq!(flac_sidecar.file_name().unwrap(), "1.wav.vtt");
        assert_eq!(
            discover_sidecar(&wav).unwrap().0.file_name().unwrap(),
            "1.wav.vtt"
        );

        fs::write(root.join("1.zh-CN.vtt"), "WEBVTT\n").expect("write language VTT");
        fs::write(root.join("1.vtt"), "WEBVTT\n").expect("write exact VTT");
        assert_eq!(
            discover_sidecar(&flac).unwrap().0.file_name().unwrap(),
            "1.vtt"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn service_sends_document_once_and_repositions_after_seek() {
        let root = temp_root();
        let audio = root.join("track.flac");
        fs::write(&audio, []).expect("write audio marker");
        fs::write(root.join("track.lrc"), "[00:01.00]One\n[00:03.00]Two").expect("write LRC");
        let service = LyricsService::default();
        let mut playback = PlaybackSnapshot {
            status: PlaybackStatus::Playing,
            path: Some(audio.to_string_lossy().into_owned()),
            position_ms: 1_500,
            ..PlaybackSnapshot::default()
        };
        let first = service.snapshot(&playback, None);
        assert_eq!(first.status, LyricsStatus::Ready);
        assert_eq!(first.active_line_index, Some(0));
        assert!(first.document.is_some());
        let second = service.snapshot(&playback, Some(first.revision));
        assert!(second.document.is_none());
        playback.position_ms = 3_500;
        assert_eq!(
            service
                .snapshot(&playback, Some(first.revision))
                .active_line_index,
            Some(1)
        );
        playback.status = PlaybackStatus::Stopped;
        assert_eq!(
            service
                .snapshot(&playback, Some(first.revision))
                .active_line_index,
            None
        );
        let _ = fs::remove_dir_all(root);
    }

    fn temp_root() -> PathBuf {
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("resona-lyrics-{}-{id}", std::process::id()));
        fs::create_dir_all(&path).expect("create temp directory");
        path
    }
}
