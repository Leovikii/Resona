// SPDX-License-Identifier: GPL-3.0-only

mod commands;
mod playback;

use std::sync::Arc;

use commands::{
    get_playback_state, pause_playback, play_file, resume_playback, seek_playback,
    set_playback_volume, stop_playback,
};
use playback::RodioPlaybackEngine;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Arc::new(RodioPlaybackEngine::new()))
        .invoke_handler(tauri::generate_handler![
            play_file,
            pause_playback,
            resume_playback,
            stop_playback,
            seek_playback,
            set_playback_volume,
            get_playback_state
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Resona");
}
