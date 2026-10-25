// SPDX-License-Identifier: GPL-3.0-only
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::MediaSessionAdapter;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::MediaSessionAdapter;
