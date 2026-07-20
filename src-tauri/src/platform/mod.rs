// SPDX-License-Identifier: GPL-3.0-only

pub mod desktop_lyrics;
pub mod media_session;

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

#[cfg(not(target_os = "windows"))]
pub fn initialize_process() {}
