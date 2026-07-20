// SPDX-License-Identifier: GPL-3.0-only
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(payload) = std::panic::catch_unwind(resona_lib::run) {
        let message = panic_message(payload.as_ref());
        let _ = std::fs::write(
            std::env::temp_dir().join("resona-startup-error.log"),
            &message,
        );
        show_startup_error(&message);
        std::process::exit(1);
    }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| {
            payload
                .downcast_ref::<&str>()
                .map(|value| (*value).to_owned())
        })
        .unwrap_or_else(|| "Resona failed to start for an unknown reason.".to_owned())
}

#[cfg(target_os = "windows")]
fn show_startup_error(message: &str) {
    use windows::core::PCWSTR;
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

    let title = "Resona could not start\0"
        .encode_utf16()
        .collect::<Vec<_>>();
    let body = format!("Resona could not start.\n\n{message}\n\nDetails were written to the temporary file resona-startup-error.log.\0")
        .encode_utf16()
        .collect::<Vec<_>>();
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(body.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn show_startup_error(message: &str) {
    eprintln!("Resona could not start: {message}");
}
