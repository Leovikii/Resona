# Audio File Icons

These transparent SVG assets are the format-specific source artwork for the future Windows and Linux file associations:

| Format | Asset | Body color | Label color |
| --- | --- | --- | --- |
| FLAC | `resona-file-flac.svg` | Violet `#845ef7` | White text with dark outline `#17151c` |
| WAV | `resona-file-wav.svg` | Amber `#f59f00` | White text with dark outline `#17151c` |
| MP3 | `resona-file-mp3.svg` | Blue `#228be6` | White text with dark outline `#17151c` |

All three assets reuse the canonical `R` letterform from the application icon. The format label is large outlined text layered over the lower half of the `R`; there is no extra border, pill, or background band. Labels use the same white fill and deep outline across formats so the amber WAV variant does not introduce an unrelated inverted treatment. Each label is inset from the foreground `R` bounds, including its stroke, so it cannot extend past the silhouette at any edge. The label is part of the artwork so rasterized Windows `.ico` sizes remain self-contained and do not depend on a runtime font. The `Arial Black`/`Segoe UI` fallback in the SVG is only used when previewing the source directly; the next packaging stage should rasterize each source into a multi-size `.ico` without changing the artwork.

The assets are intentionally not registered as file associations yet. Registration belongs to the Windows installer/platform adapter stage and must be tested with install, upgrade, uninstall, and Explorer icon-cache refresh behavior.
