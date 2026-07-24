// SPDX-License-Identifier: GPL-3.0-only

#[cfg(not(target_os = "windows"))]
use std::sync::Arc;

#[cfg(not(target_os = "windows"))]
use crate::playback::RodioPlaybackEngine;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub use windows::MediaSessionAdapter;

#[cfg(not(target_os = "windows"))]
pub struct MediaSessionAdapter;

#[cfg(not(target_os = "windows"))]
impl MediaSessionAdapter {
    pub fn start(
        _hwnd: isize,
        _engine: Arc<RodioPlaybackEngine>,
        _app_logo_path: &std::path::Path,
        _projection: std::sync::mpsc::Receiver<
            crate::platform::playback_projection::NativePlaybackSnapshot,
        >,
    ) -> Result<Self, String> {
        Err("SMTC is only available on Windows in 0.0.6".to_owned())
    }
}
