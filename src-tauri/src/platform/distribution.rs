// SPDX-License-Identifier: GPL-3.0-only

pub fn supports_updater() -> bool {
    cfg!(any(
        target_os = "windows",
        all(target_os = "linux", target_arch = "x86_64")
    ))
}

pub fn platform_key() -> &'static str {
    if cfg!(target_os = "linux") {
        "linux-x86_64"
    } else {
        "windows-x86_64"
    }
}

mod arch;
pub use arch::{prepare, PreparedUpdate};
