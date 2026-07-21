// SPDX-License-Identifier: GPL-3.0-only

use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, LogicalSize, Manager, PhysicalPosition, PhysicalSize, Runtime, WebviewWindow,
};

const LABEL: &str = "main";
const STATE_FILE: &str = "main-window.json";

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MainWindowLayoutMode {
    #[default]
    Wide,
    Compact,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MainWindowSnapshot {
    pub layout_mode: MainWindowLayoutMode,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct WindowGeometry {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    maximized: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredMainWindowState {
    layout_mode: MainWindowLayoutMode,
    wide: Option<WindowGeometry>,
    compact: Option<WindowGeometry>,
}

#[derive(Debug, Serialize)]
pub struct MainWindowFailure {
    code: String,
    message: String,
}

pub struct MainWindowService {
    path: PathBuf,
    state: Mutex<StoredMainWindowState>,
}

#[derive(Clone, Copy)]
struct LayoutMetrics {
    default_width: f64,
    default_height: f64,
    min_width: f64,
    min_height: f64,
}

impl MainWindowLayoutMode {
    fn metrics(self) -> LayoutMetrics {
        match self {
            Self::Wide => LayoutMetrics {
                default_width: 1080.0,
                default_height: 700.0,
                min_width: 760.0,
                min_height: 520.0,
            },
            Self::Compact => LayoutMetrics {
                default_width: 420.0,
                default_height: 720.0,
                min_width: 360.0,
                min_height: 600.0,
            },
        }
    }
}

impl MainWindowService {
    pub fn load(data_dir: &Path) -> Self {
        let path = data_dir.join(STATE_FILE);
        let state = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|error| {
                eprintln!("main window state is invalid; using defaults: {error}");
                StoredMainWindowState::default()
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                StoredMainWindowState::default()
            }
            Err(error) => {
                eprintln!("main window state could not be read; using defaults: {error}");
                StoredMainWindowState::default()
            }
        };
        Self {
            path,
            state: Mutex::new(state),
        }
    }

    pub fn snapshot(&self) -> Result<MainWindowSnapshot, MainWindowFailure> {
        Ok(MainWindowSnapshot {
            layout_mode: self.lock_state()?.layout_mode,
        })
    }

    pub fn restore<R: Runtime>(&self, app: &AppHandle<R>) -> Result<(), MainWindowFailure> {
        let window = main_window(app)?;
        let state = self.lock_state()?.clone();
        apply_layout(
            app,
            &window,
            state.layout_mode,
            state.geometry(state.layout_mode),
        )?;
        self.capture_window(&window)?;
        self.persist()
    }

    pub fn show<R: Runtime>(
        &self,
        app: &AppHandle<R>,
    ) -> Result<MainWindowSnapshot, MainWindowFailure> {
        let window = main_window(app)?;
        window.show().map_err(window_operation_failure)?;
        self.snapshot()
    }

    pub fn set_layout_mode<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        mode: MainWindowLayoutMode,
    ) -> Result<MainWindowSnapshot, MainWindowFailure> {
        let window = main_window(app)?;
        self.capture_window(&window)?;

        let geometry = {
            let mut state = self.lock_state()?;
            state.layout_mode = mode;
            state.geometry(mode).cloned()
        };
        apply_layout(app, &window, mode, geometry.as_ref())?;
        self.persist()?;
        Ok(MainWindowSnapshot { layout_mode: mode })
    }

    pub fn capture<R: Runtime>(&self, app: &AppHandle<R>) {
        let Some(window) = app.get_webview_window(LABEL) else {
            return;
        };
        if let Err(error) = self.capture_window(&window).and_then(|_| self.persist()) {
            eprintln!("main window geometry save failed: {}", error.message);
        }
    }

    pub fn observe<R: Runtime>(&self, app: &AppHandle<R>) {
        let Some(window) = app.get_webview_window(LABEL) else {
            return;
        };
        if let Err(error) = self.capture_window(&window) {
            eprintln!("main window geometry observation failed: {}", error.message);
        }
    }

    fn capture_window<R: Runtime>(
        &self,
        window: &WebviewWindow<R>,
    ) -> Result<(), MainWindowFailure> {
        if window.is_minimized().map_err(window_operation_failure)? {
            return Ok(());
        }
        let maximized = window.is_maximized().map_err(window_operation_failure)?;
        let mut state = self.lock_state()?;
        let mode = state.layout_mode;
        if maximized {
            if let Some(geometry) = state.geometry_mut(mode) {
                geometry.maximized = true;
            }
            return Ok(());
        }
        let position = window.outer_position().map_err(window_operation_failure)?;
        let size = window.inner_size().map_err(window_operation_failure)?;
        *state.geometry_slot_mut(mode) = Some(WindowGeometry {
            x: position.x,
            y: position.y,
            width: size.width,
            height: size.height,
            maximized: false,
        });
        Ok(())
    }

    fn persist(&self) -> Result<(), MainWindowFailure> {
        let state = self.lock_state()?.clone();
        let parent = self.path.parent().ok_or_else(|| {
            failure(
                "main_window_state_path",
                "main window state path has no parent directory",
            )
        })?;
        std::fs::create_dir_all(parent).map_err(state_io_failure)?;
        let bytes = serde_json::to_vec(&state).map_err(state_io_failure)?;
        std::fs::write(&self.path, bytes).map_err(state_io_failure)
    }

    fn lock_state(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, StoredMainWindowState>, MainWindowFailure> {
        self.state.lock().map_err(|_| {
            failure(
                "main_window_state_poisoned",
                "main window state lock is unavailable",
            )
        })
    }
}

impl StoredMainWindowState {
    fn geometry(&self, mode: MainWindowLayoutMode) -> Option<&WindowGeometry> {
        match mode {
            MainWindowLayoutMode::Wide => self.wide.as_ref(),
            MainWindowLayoutMode::Compact => self.compact.as_ref(),
        }
    }

    fn geometry_mut(&mut self, mode: MainWindowLayoutMode) -> Option<&mut WindowGeometry> {
        match mode {
            MainWindowLayoutMode::Wide => self.wide.as_mut(),
            MainWindowLayoutMode::Compact => self.compact.as_mut(),
        }
    }

    fn geometry_slot_mut(&mut self, mode: MainWindowLayoutMode) -> &mut Option<WindowGeometry> {
        match mode {
            MainWindowLayoutMode::Wide => &mut self.wide,
            MainWindowLayoutMode::Compact => &mut self.compact,
        }
    }
}

fn apply_layout<R: Runtime>(
    app: &AppHandle<R>,
    window: &WebviewWindow<R>,
    mode: MainWindowLayoutMode,
    saved: Option<&WindowGeometry>,
) -> Result<(), MainWindowFailure> {
    let metrics = mode.metrics();
    window.unmaximize().map_err(window_operation_failure)?;
    window
        .set_min_size(Some(LogicalSize::new(
            metrics.min_width,
            metrics.min_height,
        )))
        .map_err(window_operation_failure)?;

    if let Some(saved) = saved {
        let work_areas = app
            .available_monitors()
            .map_err(window_operation_failure)?
            .into_iter()
            .map(|monitor| *monitor.work_area())
            .collect::<Vec<_>>();
        if let Some(geometry) = constrain_geometry(saved.clone(), &work_areas) {
            window
                .set_size(PhysicalSize::new(geometry.width, geometry.height))
                .map_err(window_operation_failure)?;
            window
                .set_position(PhysicalPosition::new(geometry.x, geometry.y))
                .map_err(window_operation_failure)?;
            if geometry.maximized {
                window.maximize().map_err(window_operation_failure)?;
            }
            return Ok(());
        }
    }

    window
        .set_size(LogicalSize::new(
            metrics.default_width,
            metrics.default_height,
        ))
        .map_err(window_operation_failure)?;
    window.center().map_err(window_operation_failure)
}

fn constrain_geometry(
    saved: WindowGeometry,
    work_areas: &[tauri::PhysicalRect<i32, u32>],
) -> Option<WindowGeometry> {
    let area = work_areas
        .iter()
        .find(|area| intersects(&saved, area))
        .or_else(|| work_areas.first())?;
    let width = saved.width.min(area.size.width).max(1);
    let height = saved.height.min(area.size.height).max(1);
    let max_x = area.position.x + area.size.width as i32 - width as i32;
    let max_y = area.position.y + area.size.height as i32 - height as i32;
    Some(WindowGeometry {
        x: saved.x.clamp(area.position.x, max_x),
        y: saved.y.clamp(area.position.y, max_y),
        width,
        height,
        maximized: saved.maximized,
    })
}

fn intersects(geometry: &WindowGeometry, area: &tauri::PhysicalRect<i32, u32>) -> bool {
    geometry.x < area.position.x + area.size.width as i32
        && geometry.x + geometry.width as i32 > area.position.x
        && geometry.y < area.position.y + area.size.height as i32
        && geometry.y + geometry.height as i32 > area.position.y
}

fn main_window<R: Runtime>(app: &AppHandle<R>) -> Result<WebviewWindow<R>, MainWindowFailure> {
    app.get_webview_window(LABEL).ok_or_else(|| {
        failure(
            "main_window_unavailable",
            "the main application window is unavailable",
        )
    })
}

fn state_io_failure(error: impl std::fmt::Display) -> MainWindowFailure {
    failure("main_window_state_io", error.to_string())
}

fn window_operation_failure(error: impl std::fmt::Display) -> MainWindowFailure {
    failure("main_window_operation_failed", error.to_string())
}

fn failure(code: &str, message: impl Into<String>) -> MainWindowFailure {
    MainWindowFailure {
        code: code.to_owned(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offscreen_geometry_is_constrained_to_primary_work_area() {
        let area = tauri::PhysicalRect {
            position: PhysicalPosition::new(100, 80),
            size: PhysicalSize::new(1600, 900),
        };
        let restored = constrain_geometry(
            WindowGeometry {
                x: 4000,
                y: -2000,
                width: 2000,
                height: 1200,
                maximized: false,
            },
            &[area],
        )
        .expect("constrained geometry");
        assert_eq!(restored.x, 100);
        assert_eq!(restored.y, 80);
        assert_eq!(restored.width, 1600);
        assert_eq!(restored.height, 900);
    }

    #[test]
    fn layout_modes_keep_independent_geometry_slots() {
        let mut state = StoredMainWindowState::default();
        *state.geometry_slot_mut(MainWindowLayoutMode::Wide) = Some(WindowGeometry {
            x: 20,
            y: 30,
            width: 1000,
            height: 700,
            maximized: true,
        });
        *state.geometry_slot_mut(MainWindowLayoutMode::Compact) = Some(WindowGeometry {
            x: 50,
            y: 60,
            width: 420,
            height: 720,
            maximized: false,
        });
        assert_eq!(state.wide.as_ref().map(|value| value.width), Some(1000));
        assert_eq!(state.compact.as_ref().map(|value| value.width), Some(420));
    }
}
