<p align="center">
  <img src="assets/resona-resonance-wordmark.svg" width="420" alt="Resona">
</p>

<p align="center">A local-first desktop audio player built for focused listening.</p>

<p align="center">
  <a href="README.zh-CN.md">简体中文</a> ·
  <a href="https://github.com/Leovikii/Resona/releases">Download</a> ·
  <a href="docs/README.md">Documentation</a>
</p>

## Highlights

- Local MP3, WAV and FLAC playback with playlists and timed lyrics
- Windows SMTC, taskbar controls, tray controls and desktop lyrics
- Arch / KDE Wayland always-on-top desktop lyrics and MPRIS media controls
- Lossless WAV-to-FLAC compression with FFmpeg (verified download on Windows, system package on Linux)
- Local SQLite data; no account, telemetry or cloud library

## Install

### Windows x64

Download the `.exe` NSIS installer from [GitHub Releases](https://github.com/Leovikii/Resona/releases). Preview versions are marked **Pre-release**. The installer is not Authenticode-signed, so Windows may show a SmartScreen prompt. Use the official release page and verify `SHA256SUMS`; in-app updates use Tauri signature verification.

### Arch Linux x86_64 · Wayland

The recommended package is AUR `resona-preview-bin` for previews; stable releases will use `resona-bin`. The first AUR sync is pending. Once available, install with either helper:

```bash
yay -S resona-preview-bin
# or
paru -S resona-preview-bin
```

The current preview package will be available after main Actions publishes it. While AUR is pending, download the `.pkg.tar.zst` package from [GitHub Releases](https://github.com/Leovikii/Resona/releases) and install it:

```bash
sudo pacman -U ./resona-preview-bin-0.3.0alpha1-1-x86_64.pkg.tar.zst
```

The package registers MP3/WAV/FLAC opening in your desktop. Updates use `yay -Syu` / `paru -Syu`; manual package installs require downloading and installing the next package. Linux provides package-manager guidance in the app and uses system FFmpeg for compression. KDE Plasma is the tested desktop; other distributions, X11 and ARM are outside the current scope. See the [Linux guide](docs/LINUX.md).

## Development

```bash
npm ci
npm run tauri dev
```

For Arch Linux dependencies and build commands, see the [Linux guide](docs/LINUX.md).

See [the development guide](docs/DEVELOPMENT.md). Resona is licensed under [GPL-3.0-only](LICENSE).
