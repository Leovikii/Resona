# FFmpeg sidecars

Resona 0.0.16 bundles the static GPLv3 FFmpeg 8.1.2 essentials build from:

`https://www.gyan.dev/ffmpeg/builds/packages/ffmpeg-8.1.2-essentials_build.zip`

- Retrieved: 2026-07-20
- Archive SHA-256: `DB580001CAA24AC104C8CB856CD113A87B0A443F7BDF47D8C12B1D740584A2EC`
- Build flags include `--enable-gpl --enable-version3 --enable-static`.
- `ffmpeg-x86_64-pc-windows-msvc.exe` SHA-256: `1326DDE4C84FF1F96FE6B8916C5BED29E163E9B5DCCF995F6F3DB069D143EC5E`
- `ffprobe-x86_64-pc-windows-msvc.exe` SHA-256: `B49CCC7C6547B141AD5A2F6EC69CC04323D7133D7704D70B331B904C63EECB07`

The target-triple suffix is required by Tauri. At runtime Tauri distributes these as `ffmpeg.exe` and `ffprobe.exe` beside the application executable. These binaries are used only by the WAV-to-FLAC compression service and never by playback.

The executables are intentionally excluded from Git: each file is about 97 MiB and exceeds GitHub's 100 MB decimal per-file limit. Prepare a fresh checkout with:

```powershell
npm run prepare:sidecars
```

[`scripts/prepare-ffmpeg-sidecars.ps1`](../../scripts/prepare-ffmpeg-sidecars.ps1) downloads only the pinned archive above and verifies the archive plus both extracted executables before placing them under this directory. `npm run tauri dev`, `npm run tauri build` and `npm run release:windows` run the preparation step automatically. Do not commit locally downloaded sidecars or replace the pinned build without updating and reviewing all recorded hashes and license information.
