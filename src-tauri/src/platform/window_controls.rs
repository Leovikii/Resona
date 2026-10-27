// SPDX-License-Identifier: GPL-3.0-only

use tauri::{Runtime, WebviewWindow};

pub fn apply<R: Runtime>(window: &WebviewWindow<R>) {
    #[cfg(target_os = "linux")]
    {
        let native_window = window.clone();
        if let Err(error) = window.run_on_main_thread(move || match native_window.gtk_window() {
            Ok(window) => configure(&window),
            Err(error) => log::warn!("native titlebar controls unavailable: {error}"),
        }) {
            log::warn!("native titlebar controls dispatch failed: {error}");
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = window;
}

#[cfg(target_os = "linux")]
fn configure(window: &gtk::ApplicationWindow) {
    use gtk::prelude::*;

    if let Some(wrapper) = window
        .titlebar()
        .and_then(|titlebar| titlebar.downcast::<gtk::EventBox>().ok())
    {
        // Tao's Wayland wrapper otherwise intercepts input above the native buttons.
        wrapper.set_above_child(false);
    }
    let Some(screen) = gtk::prelude::WidgetExt::screen(window) else {
        log::warn!("native titlebar controls have no GDK screen");
        return;
    };
    static INSTALLED: std::sync::Once = std::sync::Once::new();
    INSTALLED.call_once(|| {
        let css = gtk::CssProvider::new();
        if let Err(error) = css.load_from_data(include_bytes!("window_controls.css")) {
            log::warn!("native titlebar controls stylesheet failed: {error}");
            return;
        }
        gtk::StyleContext::add_provider_for_screen(
            &screen,
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });
    window.style_context().add_class("resona-native-controls");
}
