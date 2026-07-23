# Audio File Icons

These transparent SVG assets are the format-specific source artwork for the future Windows and Linux file associations:

| Format | Asset | Body color | Distinction |
| --- | --- | --- | --- |
| FLAC | `resona-file-flac.svg` | Violet `#845ef7` | Color only |
| WAV | `resona-file-wav.svg` | Amber `#f59f00` | Color only |
| MP3 | `resona-file-mp3.svg` | Blue `#228be6` | Color only |

All three assets reuse the canonical `R` letterform from the application icon. File types intentionally have no text label: Explorer's small-icon modes made every attempted label less legible than the mark itself. The saturated foreground color is the only format distinction.

The Windows installer registers the generated multi-size `.ico` files. Changes must be tested with install, upgrade, uninstall, Explorer icon-cache refresh, and 16/24/32/48/256 px display modes.
