// SPDX-License-Identifier: GPL-3.0-only

use std::ffi::c_void;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::thread::{self, JoinHandle};

use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};
use windows::core::{Error as WindowsError, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    Arc as DrawArc, BeginPaint, CreatePen, CreateRoundRectRgn, CreateSolidBrush, DeleteObject,
    EndPaint, FillRect, InvalidateRect, LineTo, MoveToEx, RoundRect, SelectObject, SetWindowRgn,
    PAINTSTRUCT, PS_SOLID,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::WM_MOUSELEAVE;
use windows::Win32::UI::Input::KeyboardAndMouse::{TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetWindowLongPtrW, IsWindow,
    LoadCursorW, RegisterClassExW, SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos,
    ShowWindow, CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, GWLP_USERDATA, HWND_TOPMOST, IDC_ARROW,
    LWA_ALPHA, MA_NOACTIVATE, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SW_HIDE, SW_SHOWNOACTIVATE,
    WM_ERASEBKGND, WM_LBUTTONUP, WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_NCCREATE, WM_NCDESTROY,
    WM_PAINT, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_POPUP,
};

use super::{DesktopLyricsWindowFailure, DesktopLyricsWindowSnapshot};

const LYRICS_WINDOW_LABEL: &str = "desktop-lyrics";
const LYRICS_WIDTH: f64 = 760.0;
const LYRICS_HEIGHT: f64 = 188.0;
const MIN_LYRICS_WIDTH: u32 = 480;
const MIN_LYRICS_HEIGHT: u32 = 130;
const GEOMETRY_FILE: &str = "desktop-lyrics-window.json";
const HELPER_LOGICAL_SIZE: f64 = 38.0;
const HELPER_LOGICAL_INSET: f64 = 8.0;
// Fully transparent layered windows can be skipped by Windows hit testing.
// Keep a near-transparent pixel so the unlock hotspot remains discoverable.
const HELPER_IDLE_ALPHA: u8 = 1;
const HELPER_HOVER_ALPHA: u8 = 230;
const _: () = assert!(HELPER_IDLE_ALPHA > 0 && HELPER_HOVER_ALPHA > HELPER_IDLE_ALPHA);
const HELPER_CLASS_NAME: PCWSTR = windows::w!("ResonaDesktopLyricsUnlock");

#[derive(Default)]
struct Lifecycle {
    visible: bool,
    locked: bool,
}

impl Lifecycle {
    fn snapshot(&self) -> DesktopLyricsWindowSnapshot {
        DesktopLyricsWindowSnapshot {
            supported: true,
            visible: self.visible,
            locked: self.locked,
        }
    }

    fn mark_visible(&mut self) {
        self.visible = true;
        self.locked = false;
    }

    fn mark_hidden(&mut self) {
        self.visible = false;
        self.locked = false;
    }

    fn ensure_lockable(&self) -> Result<(), DesktopLyricsWindowFailure> {
        if self.visible {
            Ok(())
        } else {
            Err(DesktopLyricsWindowFailure::new(
                "desktop_lyrics_not_visible",
                "desktop lyrics must be visible before locking",
            ))
        }
    }

    fn mark_locked(&mut self) {
        self.locked = true;
    }

    fn mark_unlocked(&mut self) {
        self.locked = false;
    }
}

struct WindowResources {
    native_helper: NativeUnlockHelper,
    lyrics_window: WebviewWindow,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct WindowGeometry {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

#[derive(Default)]
struct ServiceState {
    lifecycle: Lifecycle,
    resources: Option<WindowResources>,
}

#[derive(Default)]
pub struct DesktopLyricsWindowService {
    state: Mutex<ServiceState>,
}

impl DesktopLyricsWindowService {
    pub fn snapshot(&self) -> DesktopLyricsWindowSnapshot {
        self.lock_state().lifecycle.snapshot()
    }

    pub fn show(
        self: &Arc<Self>,
        app: &AppHandle,
    ) -> Result<DesktopLyricsWindowSnapshot, DesktopLyricsWindowFailure> {
        let mut state = self.lock_state();
        if state.resources.is_none() {
            state.resources = Some(create_windows(app, Arc::downgrade(self))?);
        }
        let resources = state.resources.as_ref().expect("desktop lyrics resources");
        resources
            .lyrics_window
            .set_ignore_cursor_events(false)
            .map_err(|error| window_failure("desktop_lyrics_unlock_failed", error))?;
        resources.native_helper.hide();
        state.lifecycle.mark_visible();
        Ok(state.lifecycle.snapshot())
    }

    pub fn window_ready(&self) -> Result<DesktopLyricsWindowSnapshot, DesktopLyricsWindowFailure> {
        let state = self.lock_state();
        if state.lifecycle.visible {
            let resources = state.resources.as_ref().ok_or_else(|| {
                DesktopLyricsWindowFailure::new(
                    "desktop_lyrics_unavailable",
                    "desktop lyrics windows are not initialized",
                )
            })?;
            resources
                .lyrics_window
                .show()
                .map_err(|error| window_failure("desktop_lyrics_show_failed", error))?;
        }
        Ok(state.lifecycle.snapshot())
    }

    pub fn hide(
        &self,
        app: &AppHandle,
    ) -> Result<DesktopLyricsWindowSnapshot, DesktopLyricsWindowFailure> {
        self.persist_geometry(app);
        let resources = {
            let mut state = self.lock_state();
            state.lifecycle.mark_hidden();
            state.resources.take()
        };
        if let Some(resources) = resources {
            resources
                .lyrics_window
                .set_ignore_cursor_events(false)
                .map_err(|error| window_failure("desktop_lyrics_unlock_failed", error))?;
            resources.native_helper.hide();
            resources
                .lyrics_window
                .close()
                .map_err(|error| window_failure("desktop_lyrics_hide_failed", error))?;
        }
        Ok(self.snapshot())
    }

    pub fn persist_geometry(&self, app: &AppHandle) {
        let state = self.lock_state();
        let Some(resources) = state.resources.as_ref() else {
            return;
        };
        let Ok(position) = resources.lyrics_window.outer_position() else {
            return;
        };
        let Ok(size) = resources.lyrics_window.outer_size() else {
            return;
        };
        let geometry = WindowGeometry {
            x: position.x,
            y: position.y,
            width: size.width,
            height: size.height,
        };
        let Ok(data_dir) = app.path().app_data_dir() else {
            return;
        };
        if let Err(error) = std::fs::create_dir_all(&data_dir).and_then(|_| {
            let bytes = serde_json::to_vec(&geometry).map_err(std::io::Error::other)?;
            std::fs::write(data_dir.join(GEOMETRY_FILE), bytes)
        }) {
            eprintln!("desktop lyrics geometry save failed: {error}");
        }
    }

    pub fn lock(&self) -> Result<DesktopLyricsWindowSnapshot, DesktopLyricsWindowFailure> {
        let mut state = self.lock_state();
        state.lifecycle.ensure_lockable()?;
        if state.lifecycle.locked {
            return Ok(state.lifecycle.snapshot());
        }
        let resources = state.resources.as_ref().ok_or_else(|| {
            DesktopLyricsWindowFailure::new(
                "desktop_lyrics_unavailable",
                "desktop lyrics windows are not initialized",
            )
        })?;

        position_helper(resources)?;
        resources.native_helper.show()?;
        if let Err(error) = resources.lyrics_window.set_ignore_cursor_events(true) {
            resources.native_helper.hide();
            return Err(window_failure("desktop_lyrics_lock_failed", error));
        }
        state.lifecycle.mark_locked();
        Ok(state.lifecycle.snapshot())
    }

    pub fn unlock(&self) -> Result<DesktopLyricsWindowSnapshot, DesktopLyricsWindowFailure> {
        self.unlock_internal()
    }

    pub fn start_dragging(&self) -> Result<(), DesktopLyricsWindowFailure> {
        let state = self.lock_state();
        if !state.lifecycle.visible || state.lifecycle.locked {
            return Err(DesktopLyricsWindowFailure::new(
                "desktop_lyrics_drag_unavailable",
                "desktop lyrics cannot be dragged in the current state",
            ));
        }
        state
            .resources
            .as_ref()
            .ok_or_else(|| {
                DesktopLyricsWindowFailure::new(
                    "desktop_lyrics_unavailable",
                    "desktop lyrics window is not initialized",
                )
            })?
            .lyrics_window
            .start_dragging()
            .map_err(|error| window_failure("desktop_lyrics_drag_failed", error))
    }

    fn unlock_internal(&self) -> Result<DesktopLyricsWindowSnapshot, DesktopLyricsWindowFailure> {
        let mut state = self.lock_state();
        if let Some(resources) = state.resources.as_ref() {
            resources
                .lyrics_window
                .set_ignore_cursor_events(false)
                .map_err(|error| window_failure("desktop_lyrics_unlock_failed", error))?;
            resources.native_helper.hide();
        }
        state.lifecycle.mark_unlocked();
        Ok(state.lifecycle.snapshot())
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, ServiceState> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

fn create_windows(
    app: &AppHandle,
    service: Weak<DesktopLyricsWindowService>,
) -> Result<WindowResources, DesktopLyricsWindowFailure> {
    let lyrics_window = WebviewWindowBuilder::new(
        app,
        LYRICS_WINDOW_LABEL,
        WebviewUrl::App("index.html?window=desktop-lyrics".into()),
    )
    .title("Resona Desktop Lyrics")
    .inner_size(LYRICS_WIDTH, LYRICS_HEIGHT)
    .min_inner_size(MIN_LYRICS_WIDTH as f64, MIN_LYRICS_HEIGHT as f64)
    .resizable(true)
    .closable(false)
    .decorations(false)
    .transparent(true)
    .background_color((0, 0, 0, 0).into())
    .always_on_top(true)
    .skip_taskbar(true)
    .shadow(false)
    .focused(false)
    .visible(false)
    .center()
    .build()
    .map_err(|error| window_failure("desktop_lyrics_create_failed", error))?;

    restore_geometry(app, &lyrics_window);

    let lyrics_hwnd = lyrics_window
        .hwnd()
        .map_err(|error| window_failure("desktop_lyrics_handle_failed", error))?;
    let owner = lyrics_hwnd.0 as isize;
    let native_helper = match NativeUnlockHelper::create(app, owner, service) {
        Ok(helper) => helper,
        Err(error) => {
            let _ = lyrics_window.close();
            return Err(error);
        }
    };

    Ok(WindowResources {
        native_helper,
        lyrics_window,
    })
}

fn restore_geometry(app: &AppHandle, window: &WebviewWindow) {
    let Ok(data_dir) = app.path().app_data_dir() else {
        return;
    };
    let Ok(bytes) = std::fs::read(data_dir.join(GEOMETRY_FILE)) else {
        return;
    };
    let Ok(saved) = serde_json::from_slice::<WindowGeometry>(&bytes) else {
        eprintln!("desktop lyrics geometry file is invalid; using centered defaults");
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
        eprintln!("desktop lyrics size restore failed: {error}");
    }
    if let Err(error) = window.set_position(PhysicalPosition::new(restored.x, restored.y)) {
        eprintln!("desktop lyrics position restore failed: {error}");
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
    let width = saved.width.clamp(MIN_LYRICS_WIDTH, area.size.width);
    let height = saved.height.clamp(MIN_LYRICS_HEIGHT, area.size.height);
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

fn position_helper(resources: &WindowResources) -> Result<(), DesktopLyricsWindowFailure> {
    let position = resources
        .lyrics_window
        .outer_position()
        .map_err(|error| window_failure("desktop_lyrics_position_failed", error))?;
    let size = resources
        .lyrics_window
        .outer_size()
        .map_err(|error| window_failure("desktop_lyrics_size_failed", error))?;
    let scale = resources
        .lyrics_window
        .scale_factor()
        .map_err(|error| window_failure("desktop_lyrics_scale_failed", error))?;
    let helper = helper_bounds(position, size, scale);
    resources.native_helper.set_bounds(helper)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct HelperBounds {
    x: i32,
    y: i32,
    size: u32,
}

fn helper_bounds(
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
    scale: f64,
) -> HelperBounds {
    let helper_size = (HELPER_LOGICAL_SIZE * scale).round().max(1.0) as u32;
    let inset = (HELPER_LOGICAL_INSET * scale).round() as i32;
    HelperBounds {
        x: position.x + size.width as i32 - helper_size as i32 - inset,
        y: position.y + inset,
        size: helper_size,
    }
}

fn window_failure(code: &str, error: impl std::fmt::Display) -> DesktopLyricsWindowFailure {
    DesktopLyricsWindowFailure::new(
        code,
        format!("desktop lyrics window operation failed: {error}"),
    )
}

enum NativeHelperCommand {
    Unlock,
    Shutdown,
}

struct NativeHelperData {
    commands: Sender<NativeHelperCommand>,
    hovered: bool,
    tracking_leave: bool,
}

struct NativeUnlockHelper {
    hwnd: HWND,
    commands: Sender<NativeHelperCommand>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl NativeUnlockHelper {
    fn create(
        app: &AppHandle,
        owner: isize,
        service: Weak<DesktopLyricsWindowService>,
    ) -> Result<Self, DesktopLyricsWindowFailure> {
        let (commands, receiver) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("resona-desktop-lyrics-unlock".to_owned())
            .spawn(move || {
                while let Ok(command) = receiver.recv() {
                    match command {
                        NativeHelperCommand::Unlock => {
                            if let Some(service) = service.upgrade() {
                                if let Err(error) = service.unlock_internal() {
                                    eprintln!(
                                        "desktop lyrics native unlock failed: {}",
                                        error.message
                                    );
                                }
                            } else {
                                break;
                            }
                        }
                        NativeHelperCommand::Shutdown => break,
                    }
                }
            })
            .map_err(|error| {
                DesktopLyricsWindowFailure::new(
                    "desktop_lyrics_helper_worker_failed",
                    format!("failed to start desktop lyrics helper worker: {error}"),
                )
            })?;

        let data_slot = Arc::new(Mutex::new(Some(Box::new(NativeHelperData {
            commands: commands.clone(),
            hovered: false,
            tracking_leave: false,
        }))));
        let data_for_main = Arc::clone(&data_slot);
        let (result_sender, result_receiver) = mpsc::sync_channel(1);
        if let Err(error) = app.run_on_main_thread(move || {
            let result = create_native_helper_window(owner, &data_for_main);
            let _ = result_sender.send(result);
        }) {
            let _ = commands.send(NativeHelperCommand::Shutdown);
            let _ = worker.join();
            return Err(window_failure(
                "desktop_lyrics_helper_dispatch_failed",
                error,
            ));
        }

        let hwnd = match result_receiver.recv() {
            Ok(Ok(hwnd)) => HWND(hwnd),
            Ok(Err(error)) => {
                let _ = commands.send(NativeHelperCommand::Shutdown);
                let _ = worker.join();
                return Err(error);
            }
            Err(error) => {
                let _ = commands.send(NativeHelperCommand::Shutdown);
                let _ = worker.join();
                return Err(DesktopLyricsWindowFailure::new(
                    "desktop_lyrics_helper_dispatch_failed",
                    format!("desktop lyrics helper creation did not respond: {error}"),
                ));
            }
        };

        Ok(Self {
            hwnd,
            commands,
            worker: Mutex::new(Some(worker)),
        })
    }

    fn set_bounds(&self, bounds: HelperBounds) -> Result<(), DesktopLyricsWindowFailure> {
        let size = i32::try_from(bounds.size).unwrap_or(i32::MAX);
        let radius = (size / 3).max(2);
        let region = unsafe { CreateRoundRectRgn(0, 0, size + 1, size + 1, radius, radius) };
        if region.0 == 0 {
            return Err(DesktopLyricsWindowFailure::new(
                "desktop_lyrics_helper_shape_failed",
                "failed to create desktop lyrics helper window region",
            ));
        }
        if unsafe { SetWindowRgn(self.hwnd, region, true) } == 0 {
            unsafe { DeleteObject(region) };
            return Err(DesktopLyricsWindowFailure::new(
                "desktop_lyrics_helper_shape_failed",
                "failed to apply desktop lyrics helper window region",
            ));
        }
        let positioned = unsafe {
            SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                bounds.x,
                bounds.y,
                size,
                size,
                SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            )
        };
        if !positioned.as_bool() {
            return Err(window_failure(
                "desktop_lyrics_helper_position_failed",
                WindowsError::from_win32(),
            ));
        }
        Ok(())
    }

    fn show(&self) -> Result<(), DesktopLyricsWindowFailure> {
        if !unsafe { IsWindow(self.hwnd) }.as_bool() {
            return Err(DesktopLyricsWindowFailure::new(
                "desktop_lyrics_helper_unavailable",
                "desktop lyrics helper window is no longer available",
            ));
        }
        set_helper_alpha(self.hwnd, HELPER_IDLE_ALPHA);
        unsafe { ShowWindow(self.hwnd, SW_SHOWNOACTIVATE) };
        Ok(())
    }

    fn hide(&self) {
        unsafe { ShowWindow(self.hwnd, SW_HIDE) };
    }
}

impl Drop for NativeUnlockHelper {
    fn drop(&mut self) {
        let _ = self.commands.send(NativeHelperCommand::Shutdown);
        if let Ok(mut worker) = self.worker.lock() {
            if let Some(worker) = worker.take() {
                let _ = worker.join();
            }
        }
    }
}

fn create_native_helper_window(
    owner: isize,
    data_slot: &Mutex<Option<Box<NativeHelperData>>>,
) -> Result<isize, DesktopLyricsWindowFailure> {
    register_helper_class()?;
    let data = data_slot
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take()
        .expect("native helper data");
    let data = Box::into_raw(data);
    let instance = unsafe { GetModuleHandleW(None) }
        .map_err(|error| window_failure("desktop_lyrics_helper_module_handle_failed", error))?;
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
            HELPER_CLASS_NAME,
            windows::w!("Resona Unlock Desktop Lyrics"),
            WS_POPUP,
            0,
            0,
            HELPER_LOGICAL_SIZE as i32,
            HELPER_LOGICAL_SIZE as i32,
            HWND(owner),
            None,
            instance,
            Some(data.cast::<c_void>()),
        )
    };
    if hwnd.0 == 0 {
        unsafe { drop(Box::from_raw(data)) };
        return Err(window_failure(
            "desktop_lyrics_helper_create_failed",
            WindowsError::from_win32(),
        ));
    }
    if !unsafe { SetLayeredWindowAttributes(hwnd, COLORREF(0), 0, LWA_ALPHA) }.as_bool() {
        unsafe { DestroyWindow(hwnd) };
        return Err(window_failure(
            "desktop_lyrics_helper_alpha_failed",
            WindowsError::from_win32(),
        ));
    }
    Ok(hwnd.0)
}

fn register_helper_class() -> Result<(), DesktopLyricsWindowFailure> {
    static REGISTRATION: OnceLock<Result<(), String>> = OnceLock::new();
    let registration = REGISTRATION.get_or_init(|| {
        let instance = unsafe { GetModuleHandleW(None) }.map_err(|error| error.to_string())?;
        let cursor = unsafe { LoadCursorW(None, IDC_ARROW) }.map_err(|error| error.to_string())?;
        let class = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(helper_window_proc),
            hInstance: HINSTANCE(instance.0),
            hCursor: cursor,
            lpszClassName: HELPER_CLASS_NAME,
            ..Default::default()
        };
        if unsafe { RegisterClassExW(&class) } == 0 {
            Err(WindowsError::from_win32().to_string())
        } else {
            Ok(())
        }
    });
    registration.clone().map_err(|error| {
        DesktopLyricsWindowFailure::new(
            "desktop_lyrics_helper_class_failed",
            format!("failed to register desktop lyrics helper class: {error}"),
        )
    })
}

unsafe extern "system" fn helper_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        let create = &*(lparam.0 as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
    }
    let reference_data = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut NativeHelperData;
    if reference_data.is_null() {
        return DefWindowProcW(hwnd, message, wparam, lparam);
    }
    let data = &mut *reference_data;
    match message {
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_MOUSEMOVE => {
            set_helper_alpha(hwnd, HELPER_HOVER_ALPHA);
            if !data.tracking_leave {
                let mut tracking = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                if TrackMouseEvent(&mut tracking).as_bool() {
                    data.tracking_leave = true;
                }
            }
            if !data.hovered {
                data.hovered = true;
                InvalidateRect(hwnd, None, false);
            }
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            set_helper_alpha(hwnd, HELPER_IDLE_ALPHA);
            data.hovered = false;
            data.tracking_leave = false;
            InvalidateRect(hwnd, None, false);
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let _ = data.commands.send(NativeHelperCommand::Unlock);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            paint_helper(hwnd, data.hovered);
            LRESULT(0)
        }
        WM_NCDESTROY => {
            let result = DefWindowProcW(hwnd, message, wparam, lparam);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            drop(Box::from_raw(reference_data));
            result
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

fn set_helper_alpha(hwnd: HWND, alpha: u8) {
    unsafe {
        SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA);
    }
}

unsafe fn paint_helper(hwnd: HWND, hovered: bool) {
    let mut paint = PAINTSTRUCT::default();
    let hdc = BeginPaint(hwnd, &mut paint);
    let mut rect = RECT::default();
    GetClientRect(hwnd, &mut rect);

    let background = if hovered {
        COLORREF(0x004C4C52)
    } else {
        COLORREF(0x00323236)
    };
    let brush = CreateSolidBrush(background);
    FillRect(hdc, &rect, brush);
    DeleteObject(brush);

    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    let scale = (width.min(height) as f32 / 38.0).max(0.5);
    let stroke = (2.0 * scale).round().max(1.0) as i32;
    let pen = CreatePen(PS_SOLID, stroke, COLORREF(0x00FFFFFF));
    let old_pen = SelectObject(hdc, pen);
    let hollow =
        windows::Win32::Graphics::Gdi::GetStockObject(windows::Win32::Graphics::Gdi::HOLLOW_BRUSH);
    let old_brush = SelectObject(hdc, hollow);

    let cx = width / 2;
    let cy = height / 2;
    let half = (7.0 * scale).round() as i32;
    let body_top = cy - (1.0 * scale).round() as i32;
    let body_bottom = cy + (9.0 * scale).round() as i32;
    RoundRect(
        hdc,
        cx - half,
        body_top,
        cx + half,
        body_bottom,
        stroke * 2,
        stroke * 2,
    );
    DrawArc(
        hdc,
        cx - half + stroke,
        cy - (10.0 * scale).round() as i32,
        cx + half - stroke,
        cy + (3.0 * scale).round() as i32,
        cx + half - stroke,
        cy - (4.0 * scale).round() as i32,
        cx - half + stroke,
        cy - (4.0 * scale).round() as i32,
    );
    MoveToEx(
        hdc,
        cx + half - stroke,
        cy - (4.0 * scale).round() as i32,
        None,
    );
    LineTo(
        hdc,
        cx + half + (2.0 * scale).round() as i32,
        cy - (4.0 * scale).round() as i32,
    );

    SelectObject(hdc, old_brush);
    SelectObject(hdc, old_pen);
    DeleteObject(pen);
    EndPaint(hwnd, &paint);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_bounds_follow_window_and_scale() {
        assert_eq!(
            helper_bounds(
                PhysicalPosition::new(100, 200),
                PhysicalSize::new(760, 148),
                1.0,
            ),
            HelperBounds {
                x: 814,
                y: 208,
                size: 38,
            }
        );
        assert_eq!(
            helper_bounds(
                PhysicalPosition::new(-1200, 80),
                PhysicalSize::new(1140, 222),
                1.5,
            ),
            HelperBounds {
                x: -129,
                y: 92,
                size: 57,
            }
        );
    }

    #[test]
    fn restored_geometry_is_constrained_to_a_visible_work_area() {
        let work_area = tauri::PhysicalRect {
            position: PhysicalPosition::new(-1920, 0),
            size: PhysicalSize::new(1920, 1040),
        };
        let restored = constrain_geometry(
            WindowGeometry {
                x: 5000,
                y: 4000,
                width: 320,
                height: 80,
            },
            &[&work_area],
        )
        .expect("constrain geometry");
        assert_eq!(restored.width, MIN_LYRICS_WIDTH);
        assert_eq!(restored.height, MIN_LYRICS_HEIGHT);
        assert!(restored.x >= work_area.position.x);
        assert!(
            restored.x + restored.width as i32
                <= work_area.position.x + work_area.size.width as i32
        );
        assert!(restored.y >= work_area.position.y);
        assert!(
            restored.y + restored.height as i32
                <= work_area.position.y + work_area.size.height as i32
        );
    }

    #[test]
    fn lifecycle_snapshot_preserves_valid_visible_and_locked_state() {
        let mut lifecycle = Lifecycle::default();
        assert_eq!(
            lifecycle.ensure_lockable().unwrap_err().code,
            "desktop_lyrics_not_visible"
        );
        lifecycle.mark_visible();
        lifecycle.ensure_lockable().unwrap();
        lifecycle.mark_locked();
        assert_eq!(
            lifecycle.snapshot(),
            DesktopLyricsWindowSnapshot {
                supported: true,
                visible: true,
                locked: true,
            }
        );
        lifecycle.mark_unlocked();
        assert!(!lifecycle.snapshot().locked);
        lifecycle.mark_hidden();
        assert!(!lifecycle.snapshot().visible);
    }
}
