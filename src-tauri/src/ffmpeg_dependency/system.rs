// SPDX-License-Identifier: GPL-3.0-only

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

use super::{FfmpegDependencyFailure, FfmpegDependencySnapshot};

pub struct FfmpegDependencyService {
    snapshot: Mutex<FfmpegDependencySnapshot>,
}

impl FfmpegDependencyService {
    pub fn new(_data_dir: &Path) -> Arc<Self> {
        let service = Arc::new(Self {
            snapshot: Mutex::new(FfmpegDependencySnapshot {
                status: "checking".to_owned(),
                managed_by_system: true,
                version: String::new(),
                source_url: "https://archlinux.org/packages/extra/x86_64/ffmpeg/".to_owned(),
                license: "GPL-3.0-or-later".to_owned(),
                downloaded_bytes: 0,
                total_bytes: None,
                installed_bytes: 0,
                error: None,
            }),
        });
        service.refresh();
        service
    }

    pub fn binary_paths(&self) -> (PathBuf, PathBuf) {
        ("/usr/bin/ffmpeg".into(), "/usr/bin/ffprobe".into())
    }

    pub fn snapshot(&self) -> FfmpegDependencySnapshot {
        self.snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    pub fn require_ready(&self) -> Result<(), FfmpegDependencyFailure> {
        let snapshot = self.snapshot();
        if snapshot.status == "ready" {
            Ok(())
        } else {
            Err(snapshot.error.unwrap_or_else(|| {
                FfmpegDependencyFailure::new(
                    "ffmpeg_dependency_not_ready",
                    "Install FFmpeg with sudo pacman -S ffmpeg, then recheck.",
                )
            }))
        }
    }

    pub fn install(self: &Arc<Self>) -> Result<FfmpegDependencySnapshot, FfmpegDependencyFailure> {
        self.refresh();
        Ok(self.snapshot())
    }

    pub fn cancel(&self) -> FfmpegDependencySnapshot {
        self.snapshot()
    }

    fn refresh(&self) {
        let (ffmpeg, ffprobe) = self.binary_paths();
        let result = check_binary(&ffmpeg, "ffmpeg").and_then(|version| {
            check_binary(&ffprobe, "ffprobe")?;
            Ok(version)
        });
        let mut snapshot = self
            .snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match result {
            Ok(version) => {
                snapshot.status = "ready".to_owned();
                snapshot.version = version;
                snapshot.error = None;
            }
            Err(error) => {
                eprintln!("system FFmpeg check failed: {}", error.message);
                snapshot.status = "missing".to_owned();
                snapshot.version.clear();
                snapshot.error = Some(error);
            }
        }
    }
}

fn check_binary(path: &Path, name: &str) -> Result<String, FfmpegDependencyFailure> {
    let output = Command::new(path)
        .arg("-version")
        .output()
        .map_err(|error| {
            FfmpegDependencyFailure::new(
                "system_ffmpeg_unavailable",
                format!("{}: {error}; sudo pacman -S ffmpeg", path.display()),
            )
        })?;
    parse_version(name, output.status.success(), &output.stdout)
}

fn parse_version(
    name: &str,
    success: bool,
    stdout: &[u8],
) -> Result<String, FfmpegDependencyFailure> {
    let text = String::from_utf8_lossy(stdout);
    if success {
        if let Some(version) = text
            .lines()
            .next()
            .and_then(|line| line.strip_prefix(&format!("{name} version ")))
            .and_then(|tail| tail.split_whitespace().next())
        {
            return Ok(version.to_owned());
        }
    }
    Err(FfmpegDependencyFailure::new(
        "system_ffmpeg_invalid",
        format!("{name} -version failed or returned an invalid version; sudo pacman -S ffmpeg"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_version_requires_success_and_matching_tool() {
        assert_eq!(
            parse_version("ffmpeg", true, b"ffmpeg version 8.0.1 Copyright\n").unwrap(),
            "8.0.1"
        );
        assert!(parse_version("ffprobe", true, b"ffmpeg version 8.0.1").is_err());
        assert!(parse_version("ffmpeg", false, b"ffmpeg version 8.0.1").is_err());
        assert!(parse_version("ffmpeg", true, b"ffmpeg version ").is_err());
    }

    #[test]
    #[ignore = "requires the Arch system ffmpeg package"]
    fn system_tools_are_ready_without_a_download() {
        let service = FfmpegDependencyService::new(Path::new("/unused"));
        service.require_ready().expect("system FFmpeg ready");
        let snapshot = service.install().unwrap();
        assert!(snapshot.managed_by_system);
        assert!(!snapshot.version.is_empty());
        assert_eq!(snapshot.downloaded_bytes, 0);
    }
}
