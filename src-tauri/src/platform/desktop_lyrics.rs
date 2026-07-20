// SPDX-License-Identifier: GPL-3.0-only

use serde::Serialize;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub use windows::DesktopLyricsWindowService;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopLyricsWindowSnapshot {
    pub supported: bool,
    pub visible: bool,
    pub locked: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DesktopLyricsWindowFailure {
    pub code: String,
    pub message: String,
}

impl DesktopLyricsWindowFailure {
    pub(crate) fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
        }
    }
}

#[cfg(not(target_os = "windows"))]
#[derive(Default)]
pub struct DesktopLyricsWindowService;

#[cfg(not(target_os = "windows"))]
impl DesktopLyricsWindowService {
    pub fn snapshot(&self) -> DesktopLyricsWindowSnapshot {
        DesktopLyricsWindowSnapshot::default()
    }

    pub fn show(
        self: &std::sync::Arc<Self>,
        _app: &tauri::AppHandle,
    ) -> Result<DesktopLyricsWindowSnapshot, DesktopLyricsWindowFailure> {
        Err(DesktopLyricsWindowFailure::new(
            "desktop_lyrics_unsupported",
            "desktop lyrics are not available on this platform",
        ))
    }

    pub fn hide(
        &self,
        _app: &tauri::AppHandle,
    ) -> Result<DesktopLyricsWindowSnapshot, DesktopLyricsWindowFailure> {
        Ok(self.snapshot())
    }

    pub fn window_ready(&self) -> Result<DesktopLyricsWindowSnapshot, DesktopLyricsWindowFailure> {
        Ok(self.snapshot())
    }

    pub fn persist_geometry(&self, _app: &tauri::AppHandle) {}

    pub fn lock(&self) -> Result<DesktopLyricsWindowSnapshot, DesktopLyricsWindowFailure> {
        Err(DesktopLyricsWindowFailure::new(
            "desktop_lyrics_unsupported",
            "desktop lyrics are not available on this platform",
        ))
    }

    pub fn unlock(&self) -> Result<DesktopLyricsWindowSnapshot, DesktopLyricsWindowFailure> {
        Ok(self.snapshot())
    }

    pub fn start_dragging(&self) -> Result<(), DesktopLyricsWindowFailure> {
        Err(DesktopLyricsWindowFailure::new(
            "desktop_lyrics_unsupported",
            "desktop lyrics are not available on this platform",
        ))
    }
}
