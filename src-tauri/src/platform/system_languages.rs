// SPDX-License-Identifier: GPL-3.0-only

#[cfg(target_os = "linux")]
pub fn preferred() -> Result<Vec<String>, String> {
    Ok(gtk::glib::language_names()
        .iter()
        .map(ToString::to_string)
        .collect())
}

#[cfg(target_os = "windows")]
pub fn preferred() -> Result<Vec<String>, String> {
    use windows::core::PWSTR;
    use windows::Win32::Globalization::{GetUserPreferredUILanguages, MUI_LANGUAGE_NAME};
    let mut count = 0;
    let mut length = 0;
    unsafe {
        if !GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut count, PWSTR::null(), &mut length)
            .as_bool()
        {
            return Err(windows::core::Error::from_win32().to_string());
        }
        if length == 0 || length > 65_536 {
            return Err("Invalid system language buffer size".into());
        }
        let mut buffer = vec![0u16; length as usize];
        if !GetUserPreferredUILanguages(
            MUI_LANGUAGE_NAME,
            &mut count,
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )
        .as_bool()
        {
            return Err(windows::core::Error::from_win32().to_string());
        }
        Ok(buffer
            .split(|unit| *unit == 0)
            .take_while(|part| !part.is_empty())
            .map(String::from_utf16_lossy)
            .collect())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn preferred() -> Result<Vec<String>, String> {
    Err("Native system languages are unavailable on this platform".into())
}

#[cfg(all(test, any(target_os = "linux", target_os = "windows")))]
mod tests {
    #[test]
    fn native_system_language_list_is_available() {
        let languages = super::preferred().unwrap();
        assert!(!languages.is_empty());
        assert!(languages
            .iter()
            .all(|value| !value.is_empty() && !value.contains('\0')));
    }
}
