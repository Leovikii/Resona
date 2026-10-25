// SPDX-License-Identifier: GPL-3.0-only

pub fn supports_updater() -> bool {
    cfg!(target_os = "windows")
}
