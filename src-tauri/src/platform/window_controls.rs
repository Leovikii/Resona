// SPDX-License-Identifier: GPL-3.0-only

use tauri::{Runtime, WebviewWindow};

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
thread_local! {
    static MENU_EVENTS: std::cell::RefCell<std::collections::HashMap<String, gtk::gdk::Event>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

pub fn apply<R: Runtime>(window: &WebviewWindow<R>) {
    #[cfg(target_os = "windows")]
    windows::install(window);
    #[cfg(target_os = "linux")]
    {
        use gtk::prelude::*;
        let native = window.clone();
        if let Err(error) = window.with_webview(move |webview| {
            let label = native.label().to_owned();
            let Ok(native) = native.gtk_window() else {
                return;
            };
            let event_label = label.clone();
            native.connect_destroy(move |_| {
                MENU_EVENTS.with(|events| {
                    events.borrow_mut().remove(&label);
                })
            });
            // Retain the actual GDK event/Wayland serial before WebKit consumes it.
            // The frontend requests the menu only from its drag region.
            webview.inner().connect_button_press_event(move |_, event| {
                if event.button() == 3 {
                    MENU_EVENTS.with(|events| {
                        events
                            .borrow_mut()
                            .insert(event_label.clone(), (**event).clone());
                    });
                }
                gtk::glib::Propagation::Proceed
            });
        }) {
            log::warn!("native window menu unavailable: {error}");
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    let _ = window;
}

pub fn show_menu<R: Runtime>(window: &WebviewWindow<R>) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    return windows::show_menu(window);
    #[cfg(target_os = "linux")]
    {
        let window_copy = window.clone();
        return window
            .run_on_main_thread(move || {
                if let Ok(native) = window_copy.gtk_window() {
                    MENU_EVENTS.with(|events| {
                        if let (Some(mut event), Some(surface)) = (
                            events.borrow_mut().remove(window_copy.label()),
                            gtk::prelude::WidgetExt::window(&native),
                        ) {
                            if !surface.show_window_menu(&mut event) {
                                log::warn!("compositor window menu unavailable");
                            }
                        }
                    });
                }
            })
            .map_err(|error| error.to_string());
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        // GTK handles the real pointer event. Keyboard menus belong to KWin.
        let _ = window;
        Ok(())
    }
}
