// SPDX-License-Identifier: GPL-3.0-only

use serde::Serialize;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::FfmpegDependencyService;

#[cfg(not(target_os = "windows"))]
mod system;
#[cfg(not(target_os = "windows"))]
pub use system::FfmpegDependencyService;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegDependencySnapshot {
    pub status: String,
    pub managed_by_system: bool,
    pub version: String,
    pub source_url: String,
    pub license: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub installed_bytes: u64,
    pub error: Option<FfmpegDependencyFailure>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FfmpegDependencyFailure {
    pub code: String,
    pub message: String,
}

impl FfmpegDependencyFailure {
    pub(super) fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
        }
    }
}
