<p align="center">
  <img src="assets/resona-resonance-wordmark.svg" width="420" alt="Resona">
</p>

<p align="center">A local-first Windows audio player built for focused listening.</p>

<p align="center">
  <a href="README.zh-CN.md">简体中文</a> ·
  <a href="https://github.com/Leovikii/Resona/releases">Download</a> ·
  <a href="docs/README.md">Documentation</a>
</p>

## Highlights

- Local MP3, WAV and FLAC playback with playlists and timed lyrics
- Windows SMTC, taskbar controls, tray controls and desktop lyrics
- Lossless WAV-to-FLAC compression through an optional verified FFmpeg download
- Local SQLite data; no account, telemetry or cloud library

## Install

Download the Windows x64 NSIS installer from [GitHub Releases](https://github.com/Leovikii/Resona/releases). The 0.1.0 installer is not Authenticode-signed, so Windows may show a SmartScreen prompt; use only the official release page and verify its published SHA-256. In-app updates remain cryptographically signed.

## Development

```bash
npm ci
npm run tauri dev
```

See [the development guide](docs/DEVELOPMENT.md). Resona is licensed under [GPL-3.0-only](LICENSE).
