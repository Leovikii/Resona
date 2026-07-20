// SPDX-License-Identifier: GPL-3.0-only

mod commands;
mod compression;
mod compression_window;
mod filesystem;
mod lyrics;
mod media_import;
mod metadata;
mod persistence;
mod platform;
mod playback;
mod playlists;

#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::webview::PageLoadEvent;
use tauri::{Emitter, Manager, WindowEvent};

use commands::{
    add_playlist_items, append_to_queue, cancel_audio_compression, cancel_audio_compression_scan,
    clear_audio_compression_inputs, clear_queue, create_playlist, delete_playlist,
    desktop_lyrics_window_ready, get_audio_compression_scan_state, get_audio_compression_state,
    get_default_playlist, get_desktop_lyrics_window_state, get_now_playing_state,
    get_playback_state, get_track_details, hide_desktop_lyrics_window, list_playlist_items,
    list_playlists, list_recent_play, lock_desktop_lyrics_window, move_playlist,
    move_playlist_item, move_queue_item, next_playback, open_main_settings, open_media_context,
    pause_playback, play_queue_item, previous_playback, refresh_output_devices,
    remove_audio_compression_inputs, remove_playlist_item, remove_queue_item, rename_playlist,
    replace_queue, replace_queue_and_play, resume_playback, scan_audio_compression_inputs,
    seek_playback, select_output_device, set_playback_mode, set_playback_volume,
    show_audio_compression_window, show_desktop_lyrics_window, start_audio_compression,
    start_desktop_lyrics_drag, stop_playback, unlock_desktop_lyrics_window,
};
use compression::CompressionService;
use lyrics::LyricsService;
use media_import::MediaImportService;
use persistence::{PersistenceService, PlaybackSessionRecord};
use platform::desktop_lyrics::DesktopLyricsWindowService;
use playback::{PlaybackEngine, PlaybackMode, RestoredPlaybackSession, RodioPlaybackEngine};

pub fn run() {
    platform::initialize_process();
    let playback_engine = Arc::new(RodioPlaybackEngine::new());
    let media_import_service = Arc::new(MediaImportService::new(Arc::clone(&playback_engine)));
    let playback_for_restore = Arc::clone(&playback_engine);
    let lyrics_service = Arc::new(LyricsService::default());
    let desktop_lyrics_service = Arc::new(DesktopLyricsWindowService::default());
    let compression_service = Arc::new(CompressionService::default());
    #[cfg(target_os = "windows")]
    let playback_for_smtc = Arc::clone(&playback_engine);
    #[cfg(target_os = "windows")]
    let smtc_attempted = Arc::new(AtomicBool::new(false));
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
            if let Some(path) =
                media_import::external_media_path(argv, std::path::Path::new(&cwd))
            {
                dispatch_external_media(app.clone(), path);
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .on_page_load(|webview, payload| {
            if matches!(webview.label(), "main" | compression_window::LABEL)
                && payload.event() == PageLoadEvent::Finished
            {
                if let Err(error) = webview.window().show() {
                    eprintln!("{} window show after page load failed: {error}", webview.label());
                }
            }
        })
        .manage(playback_engine)
        .manage(media_import_service)
        .manage(lyrics_service)
        .manage(desktop_lyrics_service)
        .manage(compression_service)
        .setup(move |app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| std::io::Error::other(format!("无法定位应用数据目录：{error}")))?;
            std::fs::create_dir_all(&data_dir)?;
            let database = PersistenceService::open(&data_dir.join("resona.sqlite3"))
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            restore_playback_session(&playback_for_restore, &database);
            app.manage(Arc::new(database));
            let arguments = std::env::args().collect::<Vec<_>>();
            let current_directory = std::env::current_dir().unwrap_or_default();
            if let Some(path) =
                media_import::external_media_path(arguments, &current_directory)
            {
                dispatch_external_media(app.handle().clone(), path);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            open_media_context,
            get_default_playlist,
            replace_queue,
            replace_queue_and_play,
            append_to_queue,
            play_queue_item,
            remove_queue_item,
            move_queue_item,
            previous_playback,
            next_playback,
            set_playback_mode,
            clear_queue,
            refresh_output_devices,
            select_output_device,
            pause_playback,
            resume_playback,
            stop_playback,
            seek_playback,
            set_playback_volume,
            get_playback_state,
            get_now_playing_state,
            get_track_details,
            start_audio_compression,
            get_audio_compression_state,
            cancel_audio_compression,
            scan_audio_compression_inputs,
            remove_audio_compression_inputs,
            clear_audio_compression_inputs,
            get_audio_compression_scan_state,
            cancel_audio_compression_scan,
            show_audio_compression_window,
            get_desktop_lyrics_window_state,
            show_desktop_lyrics_window,
            desktop_lyrics_window_ready,
            hide_desktop_lyrics_window,
            lock_desktop_lyrics_window,
            unlock_desktop_lyrics_window,
            start_desktop_lyrics_drag,
            open_main_settings,
            list_playlists,
            create_playlist,
            rename_playlist,
            delete_playlist,
            move_playlist,
            list_playlist_items,
            add_playlist_items,
            remove_playlist_item,
            move_playlist_item,
            list_recent_play
        ])
        .build(tauri::generate_context!())
        .expect("failed to build Resona")
        .run(move |app_handle, event| {
            if let tauri::RunEvent::WindowEvent {
                label,
                event: WindowEvent::CloseRequested { .. },
                ..
            } = &event
            {
                if label == compression_window::LABEL {
                    compression_window::persist_geometry(app_handle);
                } else if label == "main" {
                    compression_window::persist_geometry(app_handle);
                    app_handle
                        .state::<Arc<DesktopLyricsWindowService>>()
                        .persist_geometry(app_handle);
                    app_handle
                        .state::<Arc<CompressionService>>()
                        .shutdown();
                    persist_playback_session(app_handle);
                    app_handle.exit(0);
                    return;
                }
            }
            #[cfg(target_os = "windows")]
            let should_start = matches!(
                &event,
                tauri::RunEvent::WindowEvent { label, .. } if label == "main"
            );
            #[cfg(target_os = "windows")]
            if should_start && !smtc_attempted.swap(true, Ordering::AcqRel) {
                let app_handle = app_handle.clone();
                let playback = Arc::clone(&playback_for_smtc);
                let _ = std::thread::Builder::new()
                    .name("resona-smtc-init".to_owned())
                    .spawn(move || {
                        for _ in 0..50 {
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let Ok(hwnd) = window.hwnd() else {
                                    std::thread::sleep(std::time::Duration::from_millis(100));
                                    continue;
                                };
                                match platform::media_session::MediaSessionAdapter::start(
                                    hwnd.0 as isize,
                                    playback,
                                ) {
                                    Ok(adapter) => {
                                        app_handle.manage(adapter);
                                        eprintln!("Windows SMTC initialized");
                                    }
                                    Err(error) => {
                                        eprintln!("Windows SMTC unavailable; continuing without it: {error}");
                                    }
                                }
                                return;
                            }
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                        eprintln!("Windows SMTC unavailable; main window HWND was not ready");
                    });
            }
        });
}

fn restore_playback_session(engine: &Arc<RodioPlaybackEngine>, database: &PersistenceService) {
    let session = match database.load_playback_session() {
        Ok(Some(session)) => session,
        Ok(None) => return,
        Err(error) => {
            eprintln!("playback session load failed: {error}");
            return;
        }
    };
    let selected_output = session.selected_output_device_id.clone();
    let restored = RestoredPlaybackSession {
        paths: session.queue_paths.into_iter().map(Into::into).collect(),
        current_path: session.current_path.map(Into::into),
        position_ms: session.position_ms,
        volume: session.volume,
        playback_mode: PlaybackMode::from_storage_key(&session.playback_mode),
    };
    if let Err(error) = engine.restore_session(restored) {
        eprintln!("playback session restore failed: {error}");
    }
    if let Some(device_id) = selected_output {
        if let Err(error) = engine.select_output_device(Some(device_id)) {
            eprintln!("saved output device unavailable; following system default: {error}");
            let _ = engine.select_output_device(None);
        }
    }
}

fn persist_playback_session(app: &tauri::AppHandle) {
    let engine = Arc::clone(app.state::<Arc<RodioPlaybackEngine>>().inner());
    let database = Arc::clone(app.state::<Arc<PersistenceService>>().inner());
    let snapshot = match engine.snapshot() {
        Ok(snapshot) => snapshot,
        Err(error) => {
            eprintln!("playback session snapshot failed: {error}");
            return;
        }
    };
    let record = PlaybackSessionRecord {
        queue_paths: snapshot.queue.into_iter().map(|item| item.path).collect(),
        current_path: snapshot.path,
        position_ms: snapshot.position_ms,
        volume: snapshot.volume,
        playback_mode: snapshot.playback_mode.storage_key().to_owned(),
        selected_output_device_id: (!snapshot.output.follow_system_default)
            .then_some(snapshot.output.selected_device_id)
            .flatten(),
    };
    if let Err(error) = database.save_playback_session(&record) {
        eprintln!("playback session save failed: {error}");
    }
}

fn dispatch_external_media(app: tauri::AppHandle, path: std::path::PathBuf) {
    let media_import = Arc::clone(app.state::<Arc<MediaImportService>>().inner());
    let persistence = Arc::clone(app.state::<Arc<PersistenceService>>().inner());
    tauri::async_runtime::spawn(async move {
        let result = tauri::async_runtime::spawn_blocking(move || {
            let result = media_import.open_media_context(&path)?;
            if let Some(path) = result.playback.path.as_deref() {
                if let Err(error) = persistence.record_recent(path) {
                    eprintln!("recent playback record failed: {error}");
                }
            }
            Ok::<_, crate::playback::PlaybackFailure>(result)
        })
        .await
        .map_err(|error| {
            crate::playback::PlaybackFailure::task_failed(format!(
                "external media task failed: {error}"
            ))
        })
        .and_then(|result| result);

        match result {
            Ok(result) => {
                if let Err(error) = app.emit_to("main", "resona://media-opened", result) {
                    eprintln!("external media event failed: {error}");
                }
            }
            Err(failure) => {
                if let Err(error) = app.emit_to("main", "resona://media-open-failed", failure) {
                    eprintln!("external media failure event failed: {error}");
                }
            }
        }
    });
}
