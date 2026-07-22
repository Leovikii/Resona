// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};
use tauri::{
    window::{Effect, EffectsBuilder},
    AppHandle, Manager, Runtime, Theme, WebviewWindow, WebviewWindowBuilder,
};

const MAIN_LABEL: &str = "main";
const COMPRESSION_LABEL: &str = "audio-compression";
const WINDOWS_11_FIRST_BUILD: u32 = 22_000;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowMaterial {
    Mica,
    #[default]
    Solid,
}

impl WindowMaterial {
    pub fn query_value(self) -> &'static str {
        match self {
            Self::Mica => "mica",
            Self::Solid => "solid",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WindowTheme {
    Auto,
    Light,
    Dark,
}

#[derive(Debug, Serialize)]
pub struct WindowMaterialFailure {
    code: String,
    message: String,
}

pub fn preferred_for(label: &str) -> WindowMaterial {
    if !matches!(label, MAIN_LABEL | COMPRESSION_LABEL) {
        return WindowMaterial::Solid;
    }
    preferred_platform_material()
}

pub fn apply<R: Runtime>(
    window: &WebviewWindow<R>,
) -> Result<WindowMaterial, WindowMaterialFailure> {
    let material = preferred_for(window.label());
    apply_effect(window, material, None)?;
    Ok(material)
}

pub fn configure_builder<'a, R: Runtime, M: Manager<R>>(
    builder: WebviewWindowBuilder<'a, R, M>,
    material: WindowMaterial,
) -> WebviewWindowBuilder<'a, R, M> {
    match material {
        WindowMaterial::Mica => builder
            .transparent(true)
            .background_color((0, 0, 0, 0).into())
            .effects(mica_effect(None)),
        WindowMaterial::Solid => builder,
    }
}

pub fn sync_theme<R: Runtime>(
    app: &AppHandle<R>,
    label: &str,
    theme: WindowTheme,
) -> Result<WindowMaterial, WindowMaterialFailure> {
    let window = managed_window(app, label)?;
    let native_theme = match theme {
        WindowTheme::Auto => None,
        WindowTheme::Light => Some(Theme::Light),
        WindowTheme::Dark => Some(Theme::Dark),
    };
    window
        .set_theme(native_theme)
        .map_err(|error| failure("window_theme_failed", error))?;
    let material = preferred_for(label);
    apply_effect(
        &window,
        material,
        match theme {
            WindowTheme::Auto => None,
            explicit => Some(explicit),
        },
    )?;
    Ok(material)
}

fn managed_window<R: Runtime>(
    app: &AppHandle<R>,
    label: &str,
) -> Result<WebviewWindow<R>, WindowMaterialFailure> {
    if !matches!(label, MAIN_LABEL | COMPRESSION_LABEL) {
        return Err(failure(
            "window_material_unsupported",
            format!("window material is not available for {label}"),
        ));
    }
    app.get_webview_window(label).ok_or_else(|| {
        failure(
            "window_material_unavailable",
            format!("window {label} is unavailable"),
        )
    })
}

fn apply_effect<R: Runtime>(
    window: &WebviewWindow<R>,
    material: WindowMaterial,
    theme: Option<WindowTheme>,
) -> Result<(), WindowMaterialFailure> {
    match material {
        WindowMaterial::Mica => window
            .set_effects(mica_effect(theme))
            .map_err(|error| failure("window_material_failed", error)),
        WindowMaterial::Solid => Ok(()),
    }
}

fn mica_effect(theme: Option<WindowTheme>) -> tauri::utils::config::WindowEffectsConfig {
    let effect = match theme {
        Some(WindowTheme::Light) => Effect::MicaLight,
        Some(WindowTheme::Dark) => Effect::MicaDark,
        Some(WindowTheme::Auto) | None => Effect::Mica,
    };
    EffectsBuilder::new().effect(effect).build()
}

#[cfg(target_os = "windows")]
fn preferred_platform_material() -> WindowMaterial {
    let version = windows_version::OsVersion::current();
    if supports_mica_version(version.major, version.build) && !windows_version::is_server() {
        WindowMaterial::Mica
    } else {
        WindowMaterial::Solid
    }
}

#[cfg(not(target_os = "windows"))]
fn preferred_platform_material() -> WindowMaterial {
    WindowMaterial::Solid
}

fn supports_mica_version(major: u32, build: u32) -> bool {
    major >= 10 && build >= WINDOWS_11_FIRST_BUILD
}

fn failure(code: &str, error: impl std::fmt::Display) -> WindowMaterialFailure {
    WindowMaterialFailure {
        code: code.to_owned(),
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mica_requires_windows_11_build() {
        assert!(!supports_mica_version(10, 19_045));
        assert!(supports_mica_version(10, 22_000));
        assert!(supports_mica_version(10, 26_100));
        assert!(!supports_mica_version(9, 30_000));
    }

    #[test]
    fn specialized_windows_keep_solid_material() {
        assert_eq!(preferred_for("desktop-lyrics"), WindowMaterial::Solid);
        assert_eq!(preferred_for("unknown"), WindowMaterial::Solid);
    }

    #[test]
    fn auto_theme_is_available_at_the_typed_boundary() {
        let theme = serde_json::from_str::<WindowTheme>("\"auto\"").expect("deserialize auto");
        assert_eq!(theme, WindowTheme::Auto);
    }
}
