// SPDX-License-Identifier: GPL-3.0-only

use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(not(target_os = "windows"))]
pub fn default_ffmpeg_binary() -> PathBuf {
    "/usr/bin/ffmpeg".into()
}

#[cfg(target_os = "windows")]
pub fn default_ffmpeg_binary() -> PathBuf {
    if let Some(directory) = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
    {
        let ffmpeg = directory.join("ffmpeg.exe");
        if ffmpeg.is_file() {
            return ffmpeg;
        }
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("binaries")
        .join("ffmpeg-x86_64-pc-windows-msvc.exe")
}

pub fn background_command(program: &Path) -> Command {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let mut command = Command::new(program);
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        command
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new(program)
    }
}
