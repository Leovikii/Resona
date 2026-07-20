# Timed text fixtures

These small LRC, SRT and WebVTT files are authored for Resona automated tests and distributed under the repository's GPL-3.0-only license.

- `sample.lrc`: metadata, offset, multiple timestamps and millisecond precision.
- `sample.srt`: multi-line cues and a cue gap.
- `sample.vtt`: header description, cue identifier, settings, voice/italic markup and entities.

Legacy GBK input is generated in memory with `encoding_rs` so the repository does not need an opaque binary text fixture.
