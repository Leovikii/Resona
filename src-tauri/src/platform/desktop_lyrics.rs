// SPDX-License-Identifier: GPL-3.0-only

use serde::Serialize;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub use windows::DesktopLyricsWindowService;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::{DesktopLyricsWindowService, GestureKind};

pub fn create_service() -> DesktopLyricsWindowService {
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        DesktopLyricsWindowService::default()
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        DesktopLyricsWindowService
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopLyricsWindowSnapshot {
    pub supported: bool,
    pub layer_shell: bool,
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

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
#[derive(Default)]
pub struct DesktopLyricsWindowService;

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
impl DesktopLyricsWindowService {
    pub fn snapshot(&self) -> DesktopLyricsWindowSnapshot {
        DesktopLyricsWindowSnapshot::default()
    }

    pub fn show(
        self: &std::sync::Arc<Self>,
        _app: &tauri::AppHandle,
        _font_size: Option<u32>,
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

    pub fn fit_height(
        &self,
        _font_size: u32,
    ) -> Result<DesktopLyricsWindowSnapshot, DesktopLyricsWindowFailure> {
        Err(DesktopLyricsWindowFailure::new(
            "desktop_lyrics_unsupported",
            "desktop lyrics are not available on this platform",
        ))
    }

    pub fn maintain_topmost(&self) {}

    pub fn enforce_height(&self) {}
}

pub fn pointer(
    service: &DesktopLyricsWindowService,
    phase: String,
    kind: String,
    x: f64,
    y: f64,
) -> Result<(), DesktopLyricsWindowFailure> {
    #[cfg(target_os = "linux")]
    {
        let kind = match kind.as_str() {
            "move" => GestureKind::Move,
            "east" => GestureKind::East,
            "west" => GestureKind::West,
            _ => {
                return Err(DesktopLyricsWindowFailure::new(
                    "desktop_lyrics_invalid_gesture",
                    "invalid gesture kind",
                ))
            }
        };
        service.pointer(phase, kind, x, y)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (service, phase, kind, x, y);
        Err(DesktopLyricsWindowFailure::new(
            "desktop_lyrics_unsupported_gesture",
            "layer-shell gestures are unavailable",
        ))
    }
}
