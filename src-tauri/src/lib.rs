// SPDX-License-Identifier: GPL-3.0-only

mod commands;
mod compression;
mod compression_window;
mod filesystem;
mod lyrics;
mod main_window;
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
    add_default_playlist_items, add_playlist_items, cancel_audio_compression,
    cancel_audio_compression_scan, clear_audio_compression_inputs, clear_default_playlist,
    clear_playlist_items, create_playlist, delete_playlist, desktop_lyrics_window_ready,
    fit_desktop_lyrics_window, get_audio_compression_scan_state, get_audio_compression_state,
    get_default_playlist, get_desktop_lyrics_window_state, get_main_window_state,
    get_now_playing_state, get_playback_state, get_track_details, hide_desktop_lyrics_window,
    list_playlist_items, list_playlists, list_recent_play, lock_desktop_lyrics_window,
    main_window_ready, move_default_playlist_item, move_playlist, move_playlist_item,
    next_playback, open_main_settings, open_media_context, pause_playback,
    play_default_playlist_item, play_queue_item, play_user_playlist_item, previous_playback,
    refresh_output_devices, remove_audio_compression_inputs, remove_default_playlist_items,
    remove_playlist_item, remove_playlist_items, rename_playlist, resume_playback,
    scan_audio_compression_inputs, seek_playback, select_output_device,
    set_main_window_layout_mode, set_playback_mode, set_playback_volume,
    show_audio_compression_window, show_desktop_lyrics_window, start_audio_compression,
    start_desktop_lyrics_drag, sync_window_theme, unlock_desktop_lyrics_window,
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
            if webview.label() == compression_window::LABEL
                && payload.event() == PageLoadEvent::Finished {
                if let Err(error) = webview.window().show() {
                    eprintln!("{} window show after page load failed: {error}", webview.label());
                }
            } else if webview.label() == "main" && payload.event() == PageLoadEvent::Finished {
                let app = webview.app_handle().clone();
                let _ = std::thread::Builder::new()
                    .name("resona-main-window-ready-fallback".to_owned())
                    .spawn(move || {
                        std::thread::sleep(std::time::Duration::from_secs(3));
                        let Some(window) = app.get_webview_window("main") else {
                            return;
                        };
                        if window.is_visible().unwrap_or(false) {
                            return;
                        }
                        eprintln!("main window ready timed out; showing the restored fallback frame");
                        if let Err(error) = window.show() {
                            eprintln!("main window fallback show failed: {error}");
                        }
                    });
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
            restore_playback_preferences(&playback_for_restore, &database);
            app.manage(Arc::new(database));
            let main_window = Arc::new(main_window::MainWindowService::load(&data_dir));
            if let Err(error) = main_window.restore(app.handle()) {
                eprintln!("main window restore failed: {error:?}");
            }
            if let Some(window) = app.get_webview_window("main") {
                if let Err(error) = platform::window_material::apply(&window) {
                    eprintln!("main window material unavailable; using solid fallback: {error:?}");
                }
            }
            app.manage(main_window);
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
            add_default_playlist_items,
            remove_default_playlist_items,
            clear_default_playlist,
            move_default_playlist_item,
            play_default_playlist_item,
            play_user_playlist_item,
            play_queue_item,
            previous_playback,
            next_playback,
            set_playback_mode,
            refresh_output_devices,
            select_output_device,
            pause_playback,
            resume_playback,
            seek_playback,
            set_playback_volume,
            get_playback_state,
            get_main_window_state,
            set_main_window_layout_mode,
            main_window_ready,
            sync_window_theme,
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
            fit_desktop_lyrics_window,
            open_main_settings,
            list_playlists,
            create_playlist,
            rename_playlist,
            delete_playlist,
            move_playlist,
            list_playlist_items,
            add_playlist_items,
            remove_playlist_item,
            remove_playlist_items,
            clear_playlist_items,
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
                    app_handle
                        .state::<Arc<main_window::MainWindowService>>()
                        .capture(app_handle);
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
            if matches!(
                &event,
                tauri::RunEvent::WindowEvent {
                    label,
                    event: WindowEvent::Moved(_) | WindowEvent::Resized(_),
                    ..
                } if label == "main"
            ) {
                app_handle
                    .state::<Arc<main_window::MainWindowService>>()
                    .observe(app_handle);
            }
            if matches!(
                &event,
                tauri::RunEvent::WindowEvent {
                    label,
                    event: WindowEvent::Resized(_),
                    ..
                } if label == "desktop-lyrics"
            ) {
                app_handle
                    .state::<Arc<DesktopLyricsWindowService>>()
                    .enforce_height();
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

fn restore_playback_preferences(engine: &Arc<RodioPlaybackEngine>, database: &PersistenceService) {
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
        paths: Vec::new(),
        current_path: None,
        position_ms: 0,
        volume: session.volume,
        playback_mode: PlaybackMode::from_storage_key(&session.playback_mode),
    };
    if let Err(error) = engine.restore_session(restored) {
        eprintln!("playback preferences restore failed: {error}");
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
        queue_paths: Vec::new(),
        current_path: None,
        position_ms: 0,
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
