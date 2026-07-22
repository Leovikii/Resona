// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, Runtime, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use crate::compression::CompressionFailure;

pub const LABEL: &str = "audio-compression";
const GEOMETRY_FILE: &str = "audio-compression-window.json";
const DEFAULT_WIDTH: f64 = 720.0;
const DEFAULT_HEIGHT: f64 = 500.0;
const MIN_WIDTH: u32 = 640;
const MIN_HEIGHT: u32 = 440;
const MAX_WIDTH: u32 = 960;
const MAX_HEIGHT: u32 = 720;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct WindowGeometry {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

pub fn show<R: Runtime>(app: &AppHandle<R>) -> Result<(), CompressionFailure> {
    if let Some(window) = app.get_webview_window(LABEL) {
        window
            .show()
            .map_err(|error| window_error("show_failed", error))?;
        window
            .unminimize()
            .map_err(|error| window_error("restore_failed", error))?;
        window
            .set_focus()
            .map_err(|error| window_error("focus_failed", error))?;
        return Ok(());
    }
    let window = WebviewWindowBuilder::new(
        app,
        LABEL,
        WebviewUrl::App("index.html?window=audio-compression".into()),
    )
    .title("Resona - Audio Compression")
    .inner_size(DEFAULT_WIDTH, DEFAULT_HEIGHT)
    .min_inner_size(MIN_WIDTH as f64, MIN_HEIGHT as f64)
    .resizable(true)
    .decorations(true)
    .background_color((16, 17, 19, 255).into())
    .visible(false)
    .center()
    .build()
    .or_else(|error| app.get_webview_window(LABEL).ok_or(error))
    .map_err(|error| window_error("create_failed", error))?;
    restore_geometry(app, &window);
    Ok(())
}

pub fn persist_geometry<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    let Ok(position) = window.outer_position() else {
        return;
    };
    let Ok(size) = window.outer_size() else {
        return;
    };
    let geometry = WindowGeometry {
        x: position.x,
        y: position.y,
        width: size.width.clamp(MIN_WIDTH, MAX_WIDTH),
        height: size.height.clamp(MIN_HEIGHT, MAX_HEIGHT),
    };
    let Ok(data_dir) = app.path().app_data_dir() else {
        return;
    };
    if let Err(error) = std::fs::create_dir_all(&data_dir) {
        eprintln!("audio compression geometry directory failed: {error}");
        return;
    }
    match serde_json::to_vec(&geometry) {
        Ok(bytes) => {
            if let Err(error) = std::fs::write(data_dir.join(GEOMETRY_FILE), bytes) {
                eprintln!("audio compression geometry save failed: {error}");
            }
        }
        Err(error) => eprintln!("audio compression geometry serialization failed: {error}"),
    }
}

fn restore_geometry<R: Runtime>(app: &AppHandle<R>, window: &WebviewWindow<R>) {
    let Ok(data_dir) = app.path().app_data_dir() else {
        return;
    };
    let Ok(bytes) = std::fs::read(data_dir.join(GEOMETRY_FILE)) else {
        return;
    };
    let Ok(saved) = serde_json::from_slice::<WindowGeometry>(&bytes) else {
        eprintln!("audio compression geometry file is invalid; using centered defaults");
        return;
    };
    let Ok(monitors) = app.available_monitors() else {
        return;
    };
    let work_areas = monitors
        .iter()
        .map(|monitor| monitor.work_area())
        .collect::<Vec<_>>();
    let Some(restored) = constrain_geometry(saved, &work_areas) else {
        return;
    };
    if let Err(error) = window.set_size(PhysicalSize::new(restored.width, restored.height)) {
        eprintln!("audio compression size restore failed: {error}");
    }
    if let Err(error) = window.set_position(PhysicalPosition::new(restored.x, restored.y)) {
        eprintln!("audio compression position restore failed: {error}");
    }
}

fn constrain_geometry(
    saved: WindowGeometry,
    work_areas: &[&tauri::PhysicalRect<i32, u32>],
) -> Option<WindowGeometry> {
    let area = work_areas
        .iter()
        .find(|area| {
            saved.x < area.position.x + area.size.width as i32
                && saved.x + saved.width as i32 > area.position.x
                && saved.y < area.position.y + area.size.height as i32
                && saved.y + saved.height as i32 > area.position.y
        })
        .copied()
        .or_else(|| work_areas.first().copied())?;
    let width = saved.width.clamp(MIN_WIDTH, MAX_WIDTH).min(area.size.width);
    let height = saved
        .height
        .clamp(MIN_HEIGHT, MAX_HEIGHT)
        .min(area.size.height);
    let x = saved.x.clamp(
        area.position.x,
        area.position.x + area.size.width as i32 - width as i32,
    );
    let y = saved.y.clamp(
        area.position.y,
        area.position.y + area.size.height as i32 - height as i32,
    );
    Some(WindowGeometry {
        x,
        y,
        width,
        height,
    })
}

fn window_error(code: &str, error: impl std::fmt::Display) -> CompressionFailure {
    CompressionFailure::new(
        &format!("audio_compression_window_{code}"),
        format!("audio compression window operation failed: {error}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restored_geometry_is_constrained_to_a_visible_work_area() {
        let area = tauri::PhysicalRect {
            position: PhysicalPosition::new(100, 80),
            size: PhysicalSize::new(1600, 900),
        };
        let restored = constrain_geometry(
            WindowGeometry {
                x: 4000,
                y: -2000,
                width: 3000,
                height: 200,
            },
            &[&area],
        )
        .expect("constrain geometry");
        assert_eq!(restored.width, MAX_WIDTH);
        assert_eq!(restored.height, MIN_HEIGHT);
        assert_eq!(restored.x, 740);
        assert_eq!(restored.y, 80);
    }

    #[test]
    fn restored_geometry_caps_historical_large_dimensions() {
        let area = tauri::PhysicalRect {
            position: PhysicalPosition::new(0, 0),
            size: PhysicalSize::new(2560, 1440),
        };
        let restored = constrain_geometry(
            WindowGeometry {
                x: 100,
                y: 120,
                width: 1710,
                height: 1860,
            },
            &[&area],
        )
        .expect("constrain oversized geometry");
        assert_eq!(restored.width, MAX_WIDTH);
        assert_eq!(restored.height, MAX_HEIGHT);
        assert_eq!(restored.x, 100);
        assert_eq!(restored.y, 120);
    }
}
