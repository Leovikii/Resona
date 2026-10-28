// SPDX-License-Identifier: GPL-3.0-only

use std::cell::Cell;
use tauri::{Emitter, Runtime, WebviewWindow};
use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::Gdi::ScreenToClient,
    UI::{
        HiDpi::GetDpiForWindow,
        Input::KeyboardAndMouse::{TrackMouseEvent, TME_LEAVE, TME_NONCLIENT, TRACKMOUSEEVENT},
        Shell::{DefSubclassProc, GetWindowSubclass, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::*,
    },
};

const SUBCLASS: usize = 0x52534e41;

struct CaptionState {
    hovered: Cell<bool>,
    notify: Box<dyn Fn(bool)>,
}
impl CaptionState {
    fn set_hovered(&self, hovered: bool) {
        if self.hovered.replace(hovered) != hovered {
            (self.notify)(hovered);
        }
    }
}

pub fn install<R: Runtime>(window: &WebviewWindow<R>) {
    let window = window.clone();
    let dispatch = window.clone();
    if let Err(error) = dispatch.run_on_main_thread(move || {
        let Ok(raw) = window.hwnd() else { return };
        let hwnd = HWND(raw.0 as isize);
        unsafe {
            let mut existing = 0;
            if !GetWindowSubclass(hwnd, Some(caption_proc), SUBCLASS, Some(&mut existing)).as_bool()
            {
                let notify = window.clone();
                let state = Box::into_raw(Box::new(CaptionState {
                    hovered: Cell::new(false),
                    notify: Box::new(move |hovered| {
                        if let Err(error) = notify.emit_to(
                            notify.label(),
                            "resona://caption-maximize-hover",
                            hovered,
                        ) {
                            log::warn!("caption hover notification failed: {error}");
                        }
                    }),
                }));
                if !SetWindowSubclass(hwnd, Some(caption_proc), SUBCLASS, state as usize).as_bool()
                {
                    drop(Box::from_raw(state));
                    log::warn!(
                        "caption subclass unavailable: {}",
                        windows::core::Error::from_win32()
                    );
                }
            }
            // WebView2 child windows otherwise receive hit tests before the top-level HWND.
            EnumChildWindows(hwnd, Some(install_child), LPARAM(0));
        }
    }) {
        log::warn!("caption subclass dispatch failed: {error}");
    }
}

unsafe extern "system" fn install_child(hwnd: HWND, _: LPARAM) -> BOOL {
    SetWindowSubclass(hwnd, Some(child_proc), SUBCLASS, 0);
    BOOL(1)
}

// Same logical 44×48 slots as WindowTitleBar; query DPI rather than caching it.
unsafe fn over_maximize(hwnd: HWND, position: LPARAM) -> bool {
    let mut point = POINT {
        x: (position.0 as i16) as i32,
        y: ((position.0 >> 16) as i16) as i32,
    };
    let mut rect = RECT::default();
    if !ScreenToClient(hwnd, &mut point).as_bool() || !GetClientRect(hwnd, &mut rect).as_bool() {
        return false;
    }
    let scale = f64::from(GetDpiForWindow(hwnd)) / 96.0;
    let top_edge = if IsZoomed(hwnd).as_bool() {
        0
    } else {
        (5.0 * scale).round() as i32
    };
    maximize_hit(rect.right, point.x, point.y, scale, top_edge)
}

fn maximize_hit(width: i32, x: i32, y: i32, scale: f64, top_edge: i32) -> bool {
    let slot = (44.0 * scale).round() as i32;
    y >= top_edge && y < (48.0 * scale).round() as i32 && x >= width - slot * 2 && x < width - slot
}

unsafe extern "system" fn child_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _: usize,
    _: usize,
) -> LRESULT {
    if message == WM_NCHITTEST && over_maximize(GetAncestor(hwnd, GA_ROOT), lparam) {
        return LRESULT(HTTRANSPARENT as isize);
    }
    if message == WM_NCDESTROY {
        RemoveWindowSubclass(hwnd, Some(child_proc), SUBCLASS);
    }
    DefSubclassProc(hwnd, message, wparam, lparam)
}

unsafe extern "system" fn caption_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    let state = &*(data as *const CaptionState);
    match message {
        WM_NCMOUSEMOVE => {
            state.set_hovered(wparam.0 == HTMAXBUTTON as usize);
            let mut tracking = TRACKMOUSEEVENT {
                cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                dwFlags: TME_LEAVE | TME_NONCLIENT,
                hwndTrack: hwnd,
                ..Default::default()
            };
            TrackMouseEvent(&mut tracking);
        }
        WM_NCMOUSELEAVE => state.set_hovered(false),
        WM_NCHITTEST if over_maximize(hwnd, lparam) => return LRESULT(HTMAXBUTTON as isize),
        WM_NCLBUTTONDOWN if wparam.0 == HTMAXBUTTON as usize => return LRESULT(0),
        WM_NCLBUTTONUP if wparam.0 == HTMAXBUTTON as usize && over_maximize(hwnd, lparam) => {
            let action = if IsZoomed(hwnd).as_bool() {
                SC_RESTORE
            } else {
                SC_MAXIMIZE
            };
            PostMessageW(hwnd, WM_SYSCOMMAND, WPARAM(action as usize), LPARAM(0));
            return LRESULT(0);
        }
        WM_NCDESTROY => {
            RemoveWindowSubclass(hwnd, Some(caption_proc), SUBCLASS);
            let result = DefSubclassProc(hwnd, message, wparam, lparam);
            drop(Box::from_raw(data as *mut CaptionState));
            return result;
        }
        _ => {}
    }
    // Keep DefWindowProc's non-client hover path for Windows 11 Snap Layouts.
    DefSubclassProc(hwnd, message, wparam, lparam)
}

pub fn show_menu<R: Runtime>(window: &WebviewWindow<R>) -> Result<(), String> {
    let native = window.clone();
    window
        .run_on_main_thread(move || unsafe {
            let Ok(raw) = native.hwnd() else { return };
            let hwnd = HWND(raw.0 as isize);
            let mut point = POINT::default();
            if !GetCursorPos(&mut point).as_bool() {
                return;
            }
            let menu = GetSystemMenu(hwnd, false);
            let maximized = IsZoomed(hwnd).as_bool();
            EnableMenuItem(
                menu,
                SC_RESTORE,
                if maximized { MF_ENABLED } else { MF_GRAYED },
            );
            EnableMenuItem(
                menu,
                SC_MAXIMIZE,
                if maximized { MF_GRAYED } else { MF_ENABLED },
            );
            EnableMenuItem(
                menu,
                SC_SIZE,
                if maximized { MF_GRAYED } else { MF_ENABLED },
            );
            let action = TrackPopupMenu(
                menu,
                TPM_RETURNCMD | TPM_RIGHTBUTTON,
                point.x,
                point.y,
                0,
                hwnd,
                None,
            );
            if action.0 != 0 {
                PostMessageW(hwnd, WM_SYSCOMMAND, WPARAM(action.0 as usize), LPARAM(0));
            }
        })
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maximize_hit_tracks_dpi_and_leaves_resize_edges_and_neighbors_alone() {
        for scale in [1.0_f64, 1.25, 1.5, 2.0] {
            let width = (360.0 * scale) as i32;
            let slot = (44.0 * scale).round() as i32;
            let top = (5.0 * scale).round() as i32;
            assert!(maximize_hit(width, width - slot - 1, top, scale, top));
            assert!(!maximize_hit(width, width - slot, top, scale, top));
            assert!(!maximize_hit(width, width - slot * 2 - 1, top, scale, top));
            assert!(!maximize_hit(width, width - slot - 1, 0, scale, top));
            assert!(maximize_hit(width, width - slot - 1, 0, scale, 0));
            assert!(!maximize_hit(
                width,
                width - slot - 1,
                (48.0 * scale) as i32,
                scale,
                top
            ));
        }
    }
}
