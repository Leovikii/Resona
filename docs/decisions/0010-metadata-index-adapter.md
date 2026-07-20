# ADR 0010: read-only metadata index adapter

- Status: Superseded by [ADR 0014](0014-player-only-scope-and-transient-default-playlist.md)
- Date: 2026-07-18

## Decision

Resona uses `lofty 0.24.0` as the read-only metadata adapter for the Windows-first media index. The adapter reads MP3, WAV and FLAC properties and common title, artist, album and embedded-picture presence into Resona-owned DTOs. Lofty types and errors terminate inside `src-tauri/src/metadata.rs`.

The first scan covers only direct files in user-selected managed folders. Recursive scanning, file watching and metadata writing are separate future decisions.

## Rationale

Lofty provides the required tag and audio-property parsing without adding a second metadata implementation or exposing format-specific types to the application boundary. Its MIT OR Apache-2.0 license is compatible with the GPL-3.0-only application.

## Consequences

- Corrupt metadata is reported per file and does not abort the rest of a directory scan.
- Media records keep `missing`, modified time and indexed time so later scans can remain incremental and diagnosable.
- Embedded artwork is indexed as presence only in 0.0.8; extracting and caching image bytes is deferred until the artwork view requires it.
