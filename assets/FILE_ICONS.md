# Resona Brand and File Icons

All SVG files in this directory are the canonical source artwork for the confirmed Resona visual identity.

| Purpose | Asset | Design |
| --- | --- | --- |
| Application icon | `resona-icon.svg` | Violet turntable, black record, rose-red hairpin stylus |
| Default artwork | `resona-default-artwork.svg` | Standalone black record with the canonical violet rim and rose-red center label |
| Standalone mark | `resona-turntable-mark.svg` | Same transparent 48 × 48 turntable mark |
| Wordmark | `resona-resonance-wordmark.svg` | `Resona` as a window onto nested violet, plum, and rose record-label fields |
| MP3 file association | `resona-file-mp3.svg` | Blue turntable with dark lower band and white `MP3` label |
| WAV file association | `resona-file-wav.svg` | Yellow turntable with dark lower band and white `WAV` label |
| FLAC file association | `resona-file-flac.svg` | Canonical violet turntable with dark lower band and white `FLAC` label |

## Format-icon rules

- The turntable structure, black vinyl, rose-red label, and lower information band are shared by all file types.
- Only the upper purple surfaces vary by format: MP3 uses Mantine blue, WAV uses Mantine yellow, and FLAC keeps the canonical violet palette.
- The lower band is always deep gray `#29272f`; labels are always near-white `#f8f8fa` and use the same 14 px master size for readability.
- File icons are deliberately more descriptive than the application icon: at small sizes, the format label has priority over the lower portion of the turntable.
- Source artwork scales the 38 × 36 turntable body proportionally to a 48 × 45.5 effective footprint inside its 48 × 48 SVG viewport. This removes left/right transparent padding while preserving the designed proportions; the remaining vertical breathing room follows naturally from the non-square body.

## Windows output

`npm run icons:windows` generates `src-tauri/icons/file-mp3.ico`, `file-wav.ico`, and `file-flac.ico` from the three format SVG files. The installer registers those ICO files for the corresponding Windows file associations.

Changes require Windows install, upgrade, uninstall, Explorer icon-cache refresh, and 16/24/32/48/256 px display checks.
