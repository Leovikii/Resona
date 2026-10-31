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

Download the [rc.1 Arch package](https://github.com/Leovikii/Resona/releases/tag/v0.3.0-rc.1) and its `SHA256SUMS` from the same release. Verify the package hash before installing:

```bash
sha256sum --ignore-missing -c SHA256SUMS
sudo pacman -U ./resona-0.3.0rc1-1-x86_64.pkg.tar.zst
```

The package registers MP3/WAV/FLAC opening. Future in-app updates download and verify the package, show the transaction, and request Polkit authorization for Pacman. The application restarts automatically after a successful installation. Remove with `sudo pacman -R resona`; user data and audio remain. Preview and stable releases share the `resona` package name. If an old test package is installed, remove it before installing this package; no automatic package-name migration is provided.

Pacman keeps your system signature policy; `.tauri.sig` is a Tauri signature, not OpenPGP. A graphical Polkit agent is required. Linux uses system FFmpeg for compression. KDE Plasma is the tested desktop; other distributions, X11 and ARM are outside the current scope. See the [Linux guide](docs/LINUX.md).

## Development

```bash
npm ci
npm run tauri dev
```

For Arch Linux dependencies and build commands, see the [Linux guide](docs/LINUX.md).

See [the development guide](docs/DEVELOPMENT.md). Resona is licensed under [GPL-3.0-only](LICENSE).
