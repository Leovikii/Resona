# ADR 0011: local timed lyrics formats

- Status: Accepted
- Date: 2026-07-18
- Matching rule amended: 2026-07-19

## Decision

Resona supports sidecar LRC, SubRip (`.srt`) and WebVTT (`.vtt`) files for the first local lyrics implementation. Files must be in the audio directory and share its base stem. In addition to `song.vtt`, discovery accepts qualified names such as `song.flac.vtt`, `song.wav.vtt`, `song.zh-CN.vtt` and `song.wav.zh-CN.vtt`; therefore `1.wav.vtt` can match either `1.wav` or `1.flac`.

Discovery first applies the deterministic format priority LRC, SRT, then WebVTT. Within one format it prefers the exact base stem, then the complete current audio filename, then other qualified names in stable filename order. A qualifier must follow a dot boundary, so audio `1.flac` cannot match `10.wav.vtt`.

The Rust `LyricsService` owns discovery, decoding, parsing, normalization and active-line selection. It exposes Resona DTOs only and remains separate from the Rodio playback engine. The service consumes the authoritative playback snapshot through an application-level combined state command; it never reads files on the audio thread.

Active-line selection follows music-player semantics rather than video-subtitle visibility. Before the first timed line starts there is no active line; after a line starts it remains active until the next line starts, including SRT/WebVTT cue gaps and the interval after the final line. Parsed SRT/WebVTT end times remain in the document as source information but do not make lyrics disappear during playback.

`lrc 0.2.0` parses LRC, `subtp 0.2.0` parses SRT/WebVTT, and `encoding_rs 0.8.35` handles UTF-8, BOM-marked UTF-16 and a GBK fallback for common Chinese Windows files. The adapter normalizes three-digit LRC millisecond tags to the parser's centisecond precision before parsing. Third-party parser and encoding types terminate inside the lyrics adapter module.

## Rationale

LRC is the common music format, while SRT and WebVTT are widely available timed-text formats that map cleanly to the same start/end/text contract. Using focused parsers avoids maintaining three grammar implementations. Qualified matching accommodates subtitle tools that preserve the source audio extension or append a language tag, while the dot boundary and stable ranking keep discovery offline and predictable.

## Consequences

- SRT/WebVTT cue layout, styling and regions are intentionally reduced to timed plain text; Resona is not a video subtitle renderer.
- SRT/WebVTT cue end times do not control playback visibility. The main player keeps the last started line highlighted, while desktop lyrics changes its current/next slots only when a new line starts.
- LRC offset and metadata are retained in the normalized document; malformed files degrade the lyrics feature without blocking playback.
- The main window receives the full lyrics document only when its revision changes; periodic playback refreshes return only the active line and status.
- Independent desktop lyrics, source selection, online lookup, translation and word-level karaoke require later decisions.
