# FFmpeg test tools

Resona uses the static GPLv3 FFmpeg 8.1.2 essentials build from an immutable GitHub Release asset:

`https://github.com/GyanD/codexffmpeg/releases/download/8.1.2/ffmpeg-8.1.2-essentials_build.zip`

- Retrieved: 2026-07-20
- Archive SHA-256: `DB580001CAA24AC104C8CB856CD113A87B0A443F7BDF47D8C12B1D740584A2EC`
- Build flags include `--enable-gpl --enable-version3 --enable-static`.
- `ffmpeg-x86_64-pc-windows-msvc.exe` SHA-256: `1326DDE4C84FF1F96FE6B8916C5BED29E163E9B5DCCF995F6F3DB069D143EC5E`
- `ffprobe-x86_64-pc-windows-msvc.exe` SHA-256: `B49CCC7C6547B141AD5A2F6EC69CC04323D7133D7704D70B331B904C63EECB07`

The executables in this directory are development-only inputs for the ignored real-conversion regression. They are never bundled with Resona. Prepare them explicitly with:

```powershell
npm run prepare:test-tools
```

[`scripts/prepare-ffmpeg-test-tools.ps1`](../../scripts/prepare-ffmpeg-test-tools.ps1) downloads the pinned archive, verifies the archive and both executables, and places them in this ignored directory. Normal development, Tauri builds and Windows releases do not run this script.

The installed application downloads the same pinned GitHub Release asset only after an explicit user action and stores it under Local AppData. FFmpeg.org publishes source code rather than official Windows executables; FFmpeg's download page links this Windows build provider. Do not commit these test executables or replace the pinned build without updating the runtime dependency specification, license review, hashes and conversion regression together.
