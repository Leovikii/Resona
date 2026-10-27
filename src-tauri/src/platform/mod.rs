// SPDX-License-Identifier: GPL-3.0-only

pub mod compression_tools;
pub mod desktop_lyrics;
pub mod distribution;
pub mod file_commit;
pub mod media_session;
pub mod playback_projection;
pub mod taskbar;
pub mod tray;
pub mod window_controls;
pub mod window_geometry;
pub mod window_material;

#[cfg(target_os = "windows")]
pub fn initialize_process() {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;

    let app_id: Vec<u16> = "io.github.vki.resona\0".encode_utf16().collect();
    // Windows uses this identity for the media flyout and taskbar grouping.
    if let Err(error) = unsafe { SetCurrentProcessExplicitAppUserModelID(PCWSTR(app_id.as_ptr())) }
    {
        eprintln!("Windows AppUserModelID unavailable: {error}");
    }
}

#[cfg(target_os = "linux")]
pub fn initialize_process() {
    // Run before GTK and worker threads: native Wayland is required for layer-shell lyrics.
    std::env::set_var("GDK_BACKEND", "wayland");
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn initialize_process() {}
