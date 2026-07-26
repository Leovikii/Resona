# Resona Icon System

This directory is the canonical source of Resona's brand artwork. The system is
built on two foundation assets:

- `resona-icon.svg`: the 48 × 48 turntable geometry and canonical violet palette.
- `resona-resonance-wordmark.svg`: the continuous `Resona` wordmark, filled by
  the same resonance color field.

Associated icons must inherit geometry or color semantics from these foundations
instead of introducing an unrelated visual language.

## Canonical assets

| Purpose | Asset | Derivation |
| --- | --- | --- |
| Application icon | `resona-icon.svg` | Foundation turntable |
| Standalone mark | `resona-turntable-mark.svg` | Transparent use of the same turntable |
| Wordmark | `resona-resonance-wordmark.svg` | Foundation wordmark with resonance color field |
| Default artwork | `resona-default-artwork.svg` | Record extracted from the turntable |
| MP3 association | `resona-file-mp3.svg` | Generated blue mapping plus format band |
| WAV association | `resona-file-wav.svg` | Generated yellow mapping plus format band |
| FLAC association | `resona-file-flac.svg` | Generated canonical-violet mapping plus format band |
| Visual reference | `icon-lab.html` | Direct references to the canonical SVG files |

## Semantic color hierarchy

The canonical violet icon uses a small role-based palette. New related artwork
must reuse these roles before adding colors.

| Role | Canonical violet | Use |
| --- | --- | --- |
| Body | `#6f00d9` | Main housing and dominant mass |
| Highlight | `#a000ff` | Upward-facing or illuminated surface |
| Structure | `#440080` | Record rim, pivot housing, and structural depth |
| Metal | `#e2c6ff` | Tonearm and small reflective mechanical details |
| Record | `#10091a` | Near-black vinyl surface |
| Focus | `#ff4081` | Record label and stylus head only |

Hierarchy rules:

- Body and highlight establish the icon's family color.
- Structure must be darker than the body while retaining the same hue.
- Metal is the lightest family tone and is reserved for the tonearm or equivalent
  reflective detail.
- Record and focus are shared anchors; do not recolor them for file formats.
- Prefer reusing a semantic role over creating another visually similar shade.

## Wordmark rules

The wordmark is a viewport onto a record, not a conventional text gradient:

- the `Resona` glyph outlines form a single mask;
- concentric color fields behind that mask simulate the record surface and grooves;
- the color-field center follows the actual `o` position of the outlined typeface;
- the `o` reveals the record label and therefore uses the same `#ff4081` focus
  color as the application icon;
- only three related violet values are used, with the main violet repeated to
  create an inner ring without adding another hue.

The SVG view box removes transparent outer canvas only. Letter geometry, record
circles, and their proportions remain independent of application layout CSS.

## File-format mappings

File icons are generated from `resona-icon.svg`; they do not own independent
turntable geometry. Every purple role is mapped to the corresponding format
family, then the shared dark information band and outlined label are appended.

| Role | Violet / FLAC | Blue / MP3 | Yellow / WAV |
| --- | --- | --- | --- |
| Body | `#6f00d9` | `#2979ff` | `#ffab00` |
| Highlight | `#a000ff` | `#82b1ff` | `#ffd740` |
| Structure | `#440080` | `#0d47a1` | `#9a5a00` |
| Metal | `#e2c6ff` | `#e3f2fd` | `#fff3c4` |

The information band is always `#212121`; its label is always `#fafafa`.
Labels use fixed SVG outlines derived from M PLUS 1p Black, so rendering and ICO
generation do not depend on Segoe UI or another installed system font.

## Generation and verification

Run:

```text
npm run icons:sources
npm run icons:windows
```

`icons:sources` applies the palette mappings and regenerates the three file SVGs.
`icons:windows` runs that source generation first, then produces:

- application PNG and ICO sizes from `resona-icon.svg`;
- `default-artwork.png` from `resona-default-artwork.svg`;
- MP3, WAV, and FLAC association ICO files;
- native taskbar control ICO files.

Use `node scripts/generate-file-icon-sources.mjs --check` and
`node scripts/generate-windows-shell-icons.mjs --check` in release validation to
detect generated files that no longer match their sources.

With the Vite development server running, open
`http://127.0.0.1:1420/assets/icon-lab.html`. The page contains no inline copies
of the artwork and therefore refreshes directly from the SVG sources.

Before release, inspect the application and format icons at 16, 20, 24, 32, 48,
64, 128, and 256 px on both dark and light backgrounds. Windows association
changes additionally require install, upgrade, uninstall, and Explorer cache
checks.
