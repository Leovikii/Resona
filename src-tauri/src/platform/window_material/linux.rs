// SPDX-License-Identifier: GPL-3.0-only

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

use gtk::{gio, glib, prelude::*};
use tauri::{Runtime, WebviewWindow};

use super::{failure, WindowMaterialFailure, WindowTheme};

thread_local! {
    static APPEARANCE: RefCell<Option<Rc<Appearance>>> = const { RefCell::new(None) };
}

struct Appearance {
    settings: gtk::Settings,
    preference: Cell<WindowTheme>,
    system_dark: Cell<bool>,
    // Keep the subscription alive; no polling or per-window portal connections.
    _portal: Option<gio::DBusProxy>,
}

impl Appearance {
    fn new() -> Result<Rc<Self>, String> {
        let settings = gtk::Settings::default().ok_or("GTK settings unavailable")?;
        let portal = gio::DBusProxy::for_bus_sync(
            gio::BusType::Session,
            gio::DBusProxyFlags::DO_NOT_AUTO_START | gio::DBusProxyFlags::DO_NOT_LOAD_PROPERTIES,
            None,
            "org.freedesktop.portal.Desktop",
            "/org/freedesktop/portal/desktop",
            "org.freedesktop.portal.Settings",
            gio::Cancellable::NONE,
        )
        .map_err(|error| error.to_string());
        let desktop_dark = settings.is_gtk_application_prefer_dark_theme();
        let system_dark = portal
            .as_ref()
            .ok()
            .and_then(|proxy| {
                proxy.set_default_timeout(500);
                match proxy.call_sync(
                    "ReadOne",
                    Some(&("org.freedesktop.appearance", "color-scheme").to_variant()),
                    gio::DBusCallFlags::NO_AUTO_START,
                    500,
                    gio::Cancellable::NONE,
                ) {
                    Ok(value) => value
                        .get::<(glib::Variant,)>()
                        .and_then(|(value,)| portal_dark(&value)),
                    Err(error) => {
                        log::warn!("desktop color-scheme unavailable; using GTK: {error}");
                        None
                    }
                }
            })
            .unwrap_or(desktop_dark);
        let portal = portal
            .map_err(|error| {
                log::warn!("desktop appearance portal unavailable; using GTK: {error}")
            })
            .ok();
        let appearance = Rc::new(Self {
            settings,
            preference: Cell::new(WindowTheme::Auto),
            system_dark: Cell::new(system_dark),
            _portal: portal,
        });
        if let Some(proxy) = &appearance._portal {
            let weak = Rc::downgrade(&appearance);
            proxy.connect_local("g-signal", false, move |values| {
                let signal = values[2].get::<String>().ok()?;
                if signal != "SettingChanged" {
                    return None;
                }
                let parameters = values[3].get::<glib::Variant>().ok()?;
                if let Some((namespace, key, value)) =
                    parameters.get::<(String, String, glib::Variant)>()
                {
                    if namespace == "org.freedesktop.appearance" && key == "color-scheme" {
                        if let (Some(appearance), Some(dark)) =
                            (weak.upgrade(), portal_dark(&value))
                        {
                            appearance.system_dark.set(dark);
                            appearance.apply();
                        }
                    }
                }
                None
            });
        }
        let weak = Rc::downgrade(&appearance);
        appearance
            .settings
            .connect_gtk_application_prefer_dark_theme_notify(move |_| {
                // Tao also writes process-wide GTK settings on portal events. Keep manual
                // overrides stable without confusing those writes with desktop preferences.
                if let Some(appearance) = weak.upgrade() {
                    appearance.apply();
                }
            });
        Ok(appearance)
    }

    fn apply(&self) {
        let dark = resolved_dark(self.preference.get(), self.system_dark.get());
        if self.settings.is_gtk_application_prefer_dark_theme() != dark {
            self.settings.set_gtk_application_prefer_dark_theme(dark);
        }
    }
}

fn portal_dark(value: &glib::Variant) -> Option<bool> {
    match value.get::<u32>()? {
        1 => Some(true),
        2 => Some(false),
        // No preference follows the desktop's default light scheme.
        0 => Some(false),
        _ => None,
    }
}

fn resolved_dark(preference: WindowTheme, system_dark: bool) -> bool {
    match preference {
        WindowTheme::Auto => system_dark,
        WindowTheme::Dark => true,
        WindowTheme::Light => false,
    }
}

#[cfg(test)]
pub(crate) fn verify_native_theme() {
    let appearance = Appearance::new().unwrap();
    let original = appearance.system_dark.get();
    let proxy = appearance._portal.as_ref().expect("KDE appearance portal");
    for preference in [WindowTheme::Auto, WindowTheme::Light, WindowTheme::Dark] {
        appearance.preference.set(preference);
        for scheme in [2u32, 1, 2] {
            let parameters = (
                "org.freedesktop.appearance",
                "color-scheme",
                scheme.to_variant(),
            )
                .to_variant();
            proxy.emit_by_name::<()>("g-signal", &[&":1.test", &"SettingChanged", &parameters]);
            let expected = resolved_dark(preference, scheme == 1);
            assert_eq!(
                appearance.settings.is_gtk_application_prefer_dark_theme(),
                expected
            );
            // Exercise competing Tao writes without changing the user's desktop.
            appearance
                .settings
                .set_gtk_application_prefer_dark_theme(!expected);
            assert_eq!(
                appearance.settings.is_gtk_application_prefer_dark_theme(),
                expected
            );
        }
    }
    appearance.system_dark.set(original);
    appearance.preference.set(WindowTheme::Auto);
    appearance.apply();
    APPEARANCE.with(|slot| *slot.borrow_mut() = Some(appearance));
    println!("KDE portal light/dark/auto and manual overrides passed");
}

pub(super) fn sync<R: Runtime>(
    window: &WebviewWindow<R>,
    theme: WindowTheme,
) -> Result<(), WindowMaterialFailure> {
    let (sender, receiver) = mpsc::sync_channel(1);
    window
        .run_on_main_thread(move || {
            let result = APPEARANCE.with(|slot| {
                let mut slot = slot.borrow_mut();
                if slot.is_none() {
                    *slot = Some(Appearance::new()?);
                }
                let appearance = slot.as_ref().unwrap();
                appearance.preference.set(theme);
                appearance.apply();
                Ok::<_, String>(())
            });
            let _ = sender.send(result);
        })
        .map_err(|error| failure("window_theme_failed", error))?;
    receiver
        .recv_timeout(Duration::from_secs(3))
        .map_err(|error| failure("window_theme_failed", error))?
        .map_err(|error| failure("window_theme_failed", error))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_scheme_and_manual_override_are_independent() {
        for dark in [false, true] {
            assert_eq!(resolved_dark(WindowTheme::Auto, dark), dark);
            assert!(resolved_dark(WindowTheme::Dark, dark));
            assert!(!resolved_dark(WindowTheme::Light, dark));
        }
        for (value, expected) in [
            (0u32, Some(false)),
            (1, Some(true)),
            (2, Some(false)),
            (9, None),
        ] {
            assert_eq!(portal_dark(&value.to_variant()), expected);
        }
        assert_eq!(portal_dark(&"invalid".to_variant()), None);
    }
}
