// SPDX-License-Identifier: GPL-3.0-only

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::platform::window_geometry;
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
    #[serde(default)]
    geometry_version: u8,
    #[serde(skip)]
    revision: u64,
    #[serde(skip)]
    pending: Option<PendingGeometry>,
    layout_mode: MainWindowLayoutMode,
    wide: Option<WindowGeometry>,
    compact: Option<WindowGeometry>,
}

#[derive(Clone, Debug)]
struct PendingGeometry {
    geometry: WindowGeometry,
    requested_at: Instant,
}

#[derive(Debug, Serialize)]
pub struct MainWindowFailure {
    code: String,
    message: String,
}

pub struct MainWindowService {
    path: PathBuf,
    state: Arc<Mutex<StoredMainWindowState>>,
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
            state: Arc::new(Mutex::new(state)),
        }
    }

    pub fn snapshot(&self) -> Result<MainWindowSnapshot, MainWindowFailure> {
        Ok(MainWindowSnapshot {
            layout_mode: self.lock_state()?.layout_mode,
        })
    }

    pub fn restore<R: Runtime>(&self, app: &AppHandle<R>) -> Result<(), MainWindowFailure> {
        let window = main_window(app)?;
        let scale = window.scale_factor().map_err(window_operation_failure)?;
        {
            let mut state = self.lock_state()?;
            state.migrate_geometry(scale, window_geometry::supports_absolute_position());
        }
        let observed = Arc::clone(&self.state);
        window_geometry::observe_size(&window, move |size, maximized| {
            match observed.lock() {
                Ok(mut state) => {
                    // Invalidate samples read before GTK allocated this content size.
                    state.revision += 1;
                    let revision = state.revision;
                    state.observe_geometry(
                        revision,
                        WindowGeometry {
                            x: 0,
                            y: 0,
                            width: size.width,
                            height: size.height,
                            maximized,
                        },
                    );
                }
                Err(error) => log::warn!("native content size observation failed: {error}"),
            }
        })
        .map_err(window_operation_failure)?;
        let mode = self.lock_state()?.layout_mode;
        self.apply_mode(app, &window, mode)?;
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
        if self.snapshot()?.layout_mode == mode {
            return self.snapshot();
        }
        let window = main_window(app)?;
        self.capture_window(&window)?;
        self.apply_mode(app, &window, mode)?;
        self.persist()?;
        Ok(MainWindowSnapshot { layout_mode: mode })
    }

    fn apply_mode<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        window: &WebviewWindow<R>,
        mode: MainWindowLayoutMode,
    ) -> Result<(), MainWindowFailure> {
        let saved = self.lock_state()?.geometry(mode).cloned();
        let geometry = planned_geometry(app, window, mode, saved)?;
        let previous = {
            let mut state = self.lock_state()?;
            let previous = state.clone();
            state.layout_mode = mode;
            state.revision += 1;
            *state.geometry_slot_mut(mode) = Some(geometry.clone());
            state.pending = Some(PendingGeometry {
                geometry: geometry.clone(),
                requested_at: Instant::now(),
            });
            previous
        };
        if let Err(error) = apply_layout(window, mode, &geometry) {
            let mut state = self.lock_state()?;
            let revision = state.revision + 1;
            *state = previous;
            state.revision = revision;
            return Err(error);
        }
        Ok(())
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
        let revision = self.lock_state()?.revision;
        if window.is_minimized().map_err(window_operation_failure)? {
            return Ok(());
        }
        // Window getters may wait for the UI thread, which also captures geometry.
        let maximized = window.is_maximized().map_err(window_operation_failure)?;
        let position = window_geometry::position(window).map_err(window_operation_failure)?;
        let size = window_geometry::inner_size(window).map_err(window_operation_failure)?;
        self.lock_state()?.observe_geometry(
            revision,
            WindowGeometry {
                x: position.x,
                y: position.y,
                width: size.width,
                height: size.height,
                maximized,
            },
        );
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
    fn migrate_geometry(&mut self, scale: f64, legacy_is_client_size: bool) {
        if self.geometry_version == 0 {
            for slot in [&mut self.wide, &mut self.compact] {
                if legacy_is_client_size {
                    if let Some(geometry) = slot {
                        let logical = PhysicalSize::new(geometry.width, geometry.height)
                            .to_logical::<u32>(scale);
                        geometry.width = logical.width;
                        geometry.height = logical.height;
                    }
                } else {
                    // Old GTK records include titlebars/shadows and may mix both modes.
                    *slot = None;
                }
            }
            self.geometry_version = 1;
        }
    }

    fn observe_geometry(&mut self, revision: u64, geometry: WindowGeometry) {
        if revision != self.revision {
            return;
        }
        if let Some(pending) = &self.pending {
            let target = &pending.geometry;
            let matches = geometry.maximized == target.maximized
                && (target.maximized
                    || (geometry.width.abs_diff(target.width) <= 1
                        && geometry.height.abs_diff(target.height) <= 1));
            if !matches {
                if pending.requested_at.elapsed() >= Duration::from_secs(2) {
                    log::warn!("window geometry adjusted by compositor: requested {target:?}, observed {geometry:?}");
                    self.pending = None;
                }
                return;
            }
            self.pending = None;
        }
        if geometry.maximized {
            if let Some(saved) = self.geometry_mut(self.layout_mode) {
                saved.maximized = true;
            }
        } else {
            *self.geometry_slot_mut(self.layout_mode) = Some(geometry);
        }
    }

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

fn planned_geometry<R: Runtime>(
    app: &AppHandle<R>,
    window: &WebviewWindow<R>,
    mode: MainWindowLayoutMode,
    saved: Option<WindowGeometry>,
) -> Result<WindowGeometry, MainWindowFailure> {
    let metrics = mode.metrics();
    let has_saved = saved.is_some();
    let mut geometry = saved.unwrap_or(WindowGeometry {
        x: 0,
        y: 0,
        width: metrics.default_width as u32,
        height: metrics.default_height as u32,
        maximized: false,
    });
    let max = window_geometry::max_inner_size(window).map_err(window_operation_failure)?;
    let max = max.unwrap_or(LogicalSize::new(u32::MAX, u32::MAX));
    geometry.width = geometry
        .width
        .clamp((metrics.min_width as u32).min(max.width), max.width);
    geometry.height = geometry
        .height
        .clamp((metrics.min_height as u32).min(max.height), max.height);
    if window_geometry::supports_absolute_position() {
        let areas = app
            .available_monitors()
            .map_err(window_operation_failure)?
            .into_iter()
            .map(|monitor| *monitor.work_area())
            .collect::<Vec<_>>();
        let scale = window.scale_factor().map_err(window_operation_failure)?;
        if !has_saved {
            let monitor = window.current_monitor().map_err(window_operation_failure)?;
            if let Some(area) = monitor
                .as_ref()
                .map(|monitor| monitor.work_area())
                .or(areas.first())
            {
                let size =
                    LogicalSize::new(geometry.width, geometry.height).to_physical::<u32>(scale);
                geometry.x =
                    area.position.x + area.size.width.saturating_sub(size.width) as i32 / 2;
                geometry.y =
                    area.position.y + area.size.height.saturating_sub(size.height) as i32 / 2;
            }
        }
        if let Some(constrained) = constrain_geometry(geometry.clone(), &areas, scale) {
            geometry = constrained;
        }
    }
    Ok(geometry)
}

fn apply_layout<R: Runtime>(
    window: &WebviewWindow<R>,
    mode: MainWindowLayoutMode,
    geometry: &WindowGeometry,
) -> Result<(), MainWindowFailure> {
    let metrics = mode.metrics();
    window_geometry::resize(
        window,
        LogicalSize::new(geometry.width, geometry.height),
        LogicalSize::new(
            (metrics.min_width as u32).min(geometry.width),
            (metrics.min_height as u32).min(geometry.height),
        ),
    )
    .map_err(window_operation_failure)?;
    if window_geometry::supports_absolute_position() {
        window
            .set_position(PhysicalPosition::new(geometry.x, geometry.y))
            .map_err(window_operation_failure)?;
    }
    if geometry.maximized {
        window.maximize().map_err(window_operation_failure)?;
    }
    Ok(())
}

fn constrain_geometry(
    saved: WindowGeometry,
    work_areas: &[tauri::PhysicalRect<i32, u32>],
    scale: f64,
) -> Option<WindowGeometry> {
    let area = work_areas
        .iter()
        .find(|area| intersects(&saved, area, scale))
        .or_else(|| work_areas.first())?;
    let physical = LogicalSize::new(saved.width, saved.height).to_physical::<u32>(scale);
    let width = physical.width.min(area.size.width).max(1);
    let height = physical.height.min(area.size.height).max(1);
    let logical = PhysicalSize::new(width, height).to_logical::<u32>(scale);
    let max_x = area.position.x + area.size.width as i32 - width as i32;
    let max_y = area.position.y + area.size.height as i32 - height as i32;
    Some(WindowGeometry {
        x: saved.x.clamp(area.position.x, max_x),
        y: saved.y.clamp(area.position.y, max_y),
        width: logical.width,
        height: logical.height,
        maximized: saved.maximized,
    })
}

fn intersects(geometry: &WindowGeometry, area: &tauri::PhysicalRect<i32, u32>, scale: f64) -> bool {
    let size = LogicalSize::new(geometry.width, geometry.height).to_physical::<u32>(scale);
    geometry.x < area.position.x + area.size.width as i32
        && geometry.x + size.width as i32 > area.position.x
        && geometry.y < area.position.y + area.size.height as i32
        && geometry.y + size.height as i32 > area.position.y
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

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "requires KDE Wayland, qdbus6 and maximize/squash effects; run alone with fresh XDG data"]
    fn native_window_controls_and_geometry() {
        crate::platform::initialize_process();
        fn wait_for_effect(effect: &str) {
            let deadline = std::time::Instant::now() + Duration::from_secs(1);
            loop {
                let output = std::process::Command::new("qdbus6")
                    .args([
                        "org.kde.KWin",
                        "/Effects",
                        "org.kde.kwin.Effects.activeEffects",
                    ])
                    .output()
                    .expect("KDE qdbus6");
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                if String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .any(|name| name == effect)
                {
                    println!("KWin {effect} animation active");
                    return;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "KWin {effect} animation did not run"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        use gtk::prelude::*;
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            mpsc, Arc,
        };
        use std::time::Duration;

        fn wait_for_size(app: &AppHandle, window: &WebviewWindow, expected: (i32, i32)) {
            let deadline = Instant::now() + Duration::from_secs(2);
            let mut first_frame = None;
            window.set_focus().unwrap();
            loop {
                let native = window.clone();
                let (done, received) = mpsc::sync_channel(1);
                app.run_on_main_thread(move || {
                    let bounds = native.gtk_window().map(|native| {
                        let bounds = native.child().unwrap().allocation();
                        let frame = native
                            .frame_clock()
                            .map_or(0, |clock| clock.frame_counter());
                        native.queue_draw();
                        ((bounds.width(), bounds.height()), frame)
                    });
                    done.send(bounds).unwrap();
                })
                .unwrap();
                let actual = received
                    .recv_timeout(Duration::from_secs(5))
                    .unwrap()
                    .unwrap_or_else(|error| {
                        panic!("{} expected {expected:?}: {error}", window.label())
                    });
                if actual.0 == expected {
                    if first_frame.is_some_and(|frame| actual.1 > frame) {
                        return;
                    }
                    first_frame.get_or_insert(actual.1);
                } else {
                    first_frame = None;
                }
                assert!(
                    Instant::now() < deadline,
                    "{} expected client {expected:?}, actual {actual:?}",
                    window.label()
                );
                std::thread::sleep(Duration::from_millis(10));
            }
        }

        let directory =
            std::env::temp_dir().join(format!("resona-window-test-{}", fastrand::u64(..)));
        let service = Arc::new(MainWindowService::load(&directory));
        let events = Arc::clone(&service);
        let closes = Arc::new(AtomicUsize::new(0));
        let closed_events = Arc::clone(&closes);
        let (finished, result) = mpsc::sync_channel(1);
        let test_directory = directory.clone();
        let app = tauri::Builder::default()
            .any_thread()
            .setup(move |app| {
                crate::platform::window_material::verify_native_theme();
                let main = app.get_webview_window("main").unwrap();
                assert_eq!(main.gtk_window().unwrap().application().unwrap().application_id().as_deref(), Some("io.github.vki.resona"));
                crate::platform::window_controls::apply(&main);
                service.restore(app.handle()).unwrap();
                let shown = std::time::Instant::now();
                main.show().unwrap();
                let app = app.handle().clone();
                let directory = test_directory.clone();
                std::thread::spawn(move || {
                    let checked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        for label in ["main", crate::compression_window::LABEL] {
                            let shown = if label == "main" {
                                shown
                            } else {
                                crate::compression_window::show(&app).unwrap();
                                let window = app.get_webview_window(label).unwrap();
                                let shown = std::time::Instant::now();
                                window.show().unwrap();
                                shown
                            };
                            let window = app.get_webview_window(label).unwrap();
                            let native = window.clone();
                            let (done, received) = mpsc::sync_channel(1);
                            app.run_on_main_thread(move || {
                                let native = native.gtk_window().unwrap();
                                gtk::glib::timeout_add_local(
                                    Duration::from_millis(10),
                                    move || {
                                        let ready = !native.is_decorated() && native.child().is_some_and(|child| child.is_mapped());
                                        let elapsed = shown.elapsed();
                                        if ready || elapsed >= Duration::from_secs(1) {
                                            done.send((ready, elapsed)).unwrap();
                                            gtk::glib::ControlFlow::Break
                                        } else {
                                            gtk::glib::ControlFlow::Continue
                                        }
                                    },
                                );
                            })
                            .unwrap();
                            let (ready, elapsed) =
                                received.recv_timeout(Duration::from_secs(5)).unwrap();
                            assert!(
                                ready && elapsed < Duration::from_secs(1),
                                "{label} controls not ready within one second: {elapsed:?}"
                            );
                            println!("{label} frameless GTK content ready after {elapsed:?}");
                            let expected = if label == "main" {
                                let state = service.lock_state().unwrap();
                                let geometry = state.geometry(state.layout_mode).unwrap();
                                (geometry.width as i32, geometry.height as i32)
                            } else { (720, 500) };
                            wait_for_size(&app, &window, expected);
                            let minimum = if label == "main" { (760, 520) } else { (640, 440) };
                            window.set_size(LogicalSize::new(100, 100)).unwrap();
                            wait_for_size(&app, &window, minimum);
                            window.set_size(LogicalSize::new(expected.0, expected.1)).unwrap();
                            wait_for_size(&app, &window, expected);

                        }
                        // UI resize callbacks capture geometry while this worker switches layouts.
                        for index in 0..4 {
                            service
                                .set_layout_mode(
                                    &app,
                                    if index % 2 == 0 {
                                        MainWindowLayoutMode::Compact
                                    } else {
                                        MainWindowLayoutMode::Wide
                                    },
                                )
                                .unwrap();
                            let window = app.get_webview_window("main").unwrap();
                            let expected = {
                                let state = service.lock_state().unwrap();
                                let geometry = state.geometry(state.layout_mode).unwrap();
                                (geometry.width as i32, geometry.height as i32)
                            };
                            wait_for_size(&app, &window, expected);
                            service.capture(&app);
                            println!("layout {index}: {expected:?}");
                        }
                        let main = app.get_webview_window("main").unwrap();
                        main.set_size(LogicalSize::new(900, 640)).unwrap();
                        wait_for_size(&app, &main, (900, 640));
                        service.capture(&app);
                        service.set_layout_mode(&app, MainWindowLayoutMode::Compact).unwrap();
                        let compact = service.lock_state().unwrap().compact.clone().unwrap();
                        wait_for_size(&app, &main, (compact.width as i32, compact.height as i32));
                        main.set_size(LogicalSize::new(440, 640)).unwrap();
                        wait_for_size(&app, &main, (440, 640));
                        service.capture(&app);
                        for _ in 0..3 {
                            service.set_layout_mode(&app, MainWindowLayoutMode::Wide).unwrap();
                            wait_for_size(&app, &main, (900, 640));
                            service.set_layout_mode(&app, MainWindowLayoutMode::Compact).unwrap();
                            wait_for_size(&app, &main, (440, 640));
                            service.capture(&app);
                            let reloaded = MainWindowService::load(&directory);
                            reloaded.restore(&app).unwrap();
                            wait_for_size(&app, &main, (440, 640));
                        }
                        let saved = MainWindowService::load(&directory);
                        let state = saved.lock_state().unwrap();
                        assert_eq!(state.geometry_version, 1);
                        assert_eq!((state.wide.as_ref().unwrap().width, state.wide.as_ref().unwrap().height), (900, 640));
                        assert_eq!((state.compact.as_ref().unwrap().width, state.compact.as_ref().unwrap().height), (440, 640));
                        drop(state);
                        let compression = app.get_webview_window(crate::compression_window::LABEL).unwrap();
                        compression.set_size(LogicalSize::new(800, 560)).unwrap();
                        wait_for_size(&app, &compression, (800, 560));
                        for _ in 0..3 {
                            crate::compression_window::persist_geometry(&app);
                            app.get_webview_window(crate::compression_window::LABEL).unwrap().destroy().unwrap();
                            let deadline = Instant::now() + Duration::from_secs(2);
                            while app.get_webview_window(crate::compression_window::LABEL).is_some() {
                                assert!(Instant::now() < deadline, "compression window destruction timed out");
                                std::thread::sleep(Duration::from_millis(10));
                            }
                            crate::compression_window::show(&app).unwrap();
                            let compression = app.get_webview_window(crate::compression_window::LABEL).unwrap();
                            compression.show().unwrap();
                            wait_for_size(&app, &compression, (800, 560));
                        }
                        service.set_layout_mode(&app, MainWindowLayoutMode::Wide).unwrap();
                        wait_for_size(&app, &main, (900, 640));
                        main.set_size(LogicalSize::new(100, 100)).unwrap();
                        wait_for_size(&app, &main, (760, 520));
                        service.set_layout_mode(&app, MainWindowLayoutMode::Compact).unwrap();
                        wait_for_size(&app, &main, (440, 640));
                        main.set_size(LogicalSize::new(100, 100)).unwrap();
                        wait_for_size(&app, &main, (360, 600));
                        service.capture(&app);
                        service.set_layout_mode(&app, MainWindowLayoutMode::Wide).unwrap();
                        wait_for_size(&app, &main, (760, 520));
                        main.maximize().unwrap();
                        let deadline = Instant::now() + Duration::from_secs(2);
                        while !main.is_maximized().unwrap() {
                            assert!(Instant::now() < deadline);
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        service.capture(&app);
                        service.set_layout_mode(&app, MainWindowLayoutMode::Compact).unwrap();
                        wait_for_size(&app, &main, (360, 600));
                        assert!(!main.is_maximized().unwrap());
                        service.set_layout_mode(&app, MainWindowLayoutMode::Wide).unwrap();
                        let deadline = Instant::now() + Duration::from_secs(2);
                        while !main.is_maximized().unwrap() {
                            assert!(Instant::now() < deadline);
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        main.unmaximize().unwrap();
                        wait_for_size(&app, &main, (760, 520));
                        println!("custom sizes, reload and compression recreation preserved client dimensions");
                        // Wayland cannot acknowledge deiconify: test geometry before minimizing.
                        for label in ["main", crate::compression_window::LABEL] {
                            let window = app.get_webview_window(label).unwrap();
                            for action in ["maximize", "minimize", "close"] {
                                match action {
                                    "maximize" => window.maximize().unwrap(),
                                    "minimize" => window.minimize().unwrap(),
                                    "close" => window.close().unwrap(),
                                    _ => unreachable!(),
                                }
                                if action == "maximize" {
                                    wait_for_effect("maximize");
                                    let deadline =
                                        std::time::Instant::now() + Duration::from_secs(5);
                                    while !window.is_maximized().unwrap() {
                                        assert!(
                                            std::time::Instant::now() < deadline,
                                            "native {action} signal was not handled"
                                        );
                                        std::thread::sleep(Duration::from_millis(10));
                                    }
                                    window.unmaximize().unwrap();
                                    std::thread::sleep(Duration::from_millis(200));
                                } else if action == "minimize" {
                                    wait_for_effect("squash");
                                    // Wayland has no minimized-state acknowledgement; check the protocol trace.
                                    std::thread::sleep(Duration::from_millis(200));
                                    window.unminimize().unwrap();
                                    window.set_focus().unwrap();
                                    std::thread::sleep(Duration::from_millis(200));
                                }
                                println!("{} native {action} delivered", window.label());
                            }
                        }
                        let deadline = std::time::Instant::now() + Duration::from_secs(5);
                        while closes.load(Ordering::Acquire) < 2 {
                            assert!(
                                std::time::Instant::now() < deadline,
                                "native Close signals were not handled"
                            );
                            std::thread::sleep(Duration::from_millis(10));
                        }
                    }));
                    finished.send(checked.is_ok()).unwrap();
                    app.exit(0);
                });
                Ok(())
            })
            .build(tauri::generate_context!())
            .unwrap();
        assert_eq!(
            app.run_return(move |app, event| {
                if let tauri::RunEvent::WindowEvent { label, event, .. } = event {
                    match event {
                        tauri::WindowEvent::Resized(_) if label == "main" => events.observe(app),
                        tauri::WindowEvent::CloseRequested { api, .. } => {
                            api.prevent_close();
                            closed_events.fetch_add(1, Ordering::AcqRel);
                        }
                        _ => {}
                    }
                }
            }),
            0
        );
        assert!(result.recv_timeout(Duration::from_secs(5)).unwrap());
        std::fs::remove_dir_all(directory).unwrap();
    }

    fn geometry(width: u32, height: u32) -> WindowGeometry {
        WindowGeometry {
            x: 0,
            y: 0,
            width,
            height,
            maximized: false,
        }
    }

    #[test]
    fn legacy_geometry_migration_preserves_layout_and_converts_windows_pixels_once() {
        let bytes = r#"{"layoutMode":"compact","wide":{"x":100,"y":50,"width":2160,"height":1400,"maximized":false},"compact":{"x":100,"y":50,"width":840,"height":1440,"maximized":false}}"#;
        let mut windows: StoredMainWindowState = serde_json::from_str(bytes).unwrap();
        windows.migrate_geometry(2.0, true);
        assert_eq!(windows.layout_mode, MainWindowLayoutMode::Compact);
        assert_eq!(windows.geometry_version, 1);
        assert_eq!(windows.wide.as_ref().unwrap().width, 1080);
        assert_eq!(windows.compact.as_ref().unwrap().height, 720);
        assert_eq!(windows.wide.as_ref().unwrap().x, 100);
        windows.migrate_geometry(1.5, true);
        assert_eq!(windows.wide.as_ref().unwrap().width, 1080);
        let mut linux: StoredMainWindowState = serde_json::from_str(bytes).unwrap();
        linux.migrate_geometry(2.0, false);
        assert_eq!(linux.layout_mode, MainWindowLayoutMode::Compact);
        assert!(linux.wide.is_none() && linux.compact.is_none());
    }

    #[test]
    fn resizing_does_not_mix_modes_or_accept_an_old_query() {
        let mut state = StoredMainWindowState {
            geometry_version: 1,
            layout_mode: MainWindowLayoutMode::Compact,
            revision: 2,
            wide: Some(geometry(1080, 700)),
            compact: Some(geometry(420, 720)),
            pending: Some(PendingGeometry {
                geometry: geometry(420, 720),
                requested_at: Instant::now(),
            }),
        };
        state.observe_geometry(1, geometry(1080, 700));
        state.observe_geometry(2, geometry(760, 720));
        assert!(state.pending.is_some());
        assert_eq!(state.compact.as_ref().unwrap().width, 420);
        state.observe_geometry(2, geometry(420, 720));
        assert!(state.pending.is_none());
        state.observe_geometry(2, geometry(440, 760));
        assert_eq!(state.compact.as_ref().unwrap().width, 440);
        assert_eq!(state.wide.as_ref().unwrap().width, 1080);
    }

    #[test]
    fn maximization_preserves_normal_geometry_and_compositor_timeout_releases_capture() {
        let mut state = StoredMainWindowState {
            geometry_version: 1,
            wide: Some(geometry(980, 640)),
            ..StoredMainWindowState::default()
        };
        let mut maximized = geometry(1920, 1080);
        maximized.maximized = true;
        state.observe_geometry(0, maximized);
        assert!(state.wide.as_ref().unwrap().maximized);
        assert_eq!(state.wide.as_ref().unwrap().width, 980);
        state.pending = Some(PendingGeometry {
            geometry: geometry(1080, 700),
            requested_at: Instant::now() - Duration::from_secs(3),
        });
        state.observe_geometry(0, geometry(900, 600));
        assert!(state.pending.is_none());
        assert_eq!(state.wide.as_ref().unwrap().width, 980);
        state.observe_geometry(0, geometry(900, 600));
        assert_eq!(state.wide.as_ref().unwrap().width, 900);
    }

    #[test]
    fn logical_geometry_is_constrained_using_physical_monitor_coordinates() {
        let area = tauri::PhysicalRect {
            position: PhysicalPosition::new(100, 80),
            size: PhysicalSize::new(2400, 1600),
        };
        let mut saved = geometry(1080, 700);
        saved.x = 4000;
        let restored = constrain_geometry(saved, &[area], 2.0).unwrap();
        assert_eq!((restored.width, restored.height), (1080, 700));
        assert_eq!((restored.x, restored.y), (340, 80));
    }

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
            1.0,
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
