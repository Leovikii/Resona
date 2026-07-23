# Audio fixtures

These files are deterministic synthetic 997 Hz sine waves generated for Resona tests. They contain no third-party recordings and are distributed under the repository's GPL-3.0-only license.

The matrix covers:

- integer WAV: 44.1/48/96/192 kHz at 16/24/32-bit;
- float WAV: 44.1/48/96/192 kHz at 32-bit float;
- integer FLAC: 44.1/48/96/192 kHz at 16/24/32-bit;
- MP3: 44.1/48 kHz at 128/320 kbps CBR and VBR quality 0;
- mono, seek, empty, truncated, and mislabeled-content boundary cases.

Regenerate from the repository root with FFmpeg 8.1.2 or later and Xiph FLAC 1.5.0 or later. The reference FLAC encoder is required because FFmpeg 8.1.2 silently limits FLAC output to 24-bit.

```bash
node scripts/generate-audio-fixtures.mjs
npm run verify:fixtures
```

`SHA256SUMS.txt` records the expected generated files. Lossless samples are 0.35 seconds long; `seek_48000_24_stereo.flac` is 4 seconds long.
