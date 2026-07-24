// SPDX-License-Identifier: GPL-3.0-only

mod application_lifetime;
mod application_update;
mod commands;
mod compression;
mod compression_window;
mod diagnostics;
mod ffmpeg_dependency;
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

use application_lifetime::{ApplicationLifetimeService, CloseDisposition};
use application_update::ApplicationUpdateService;
use commands::{
    add_default_playlist_items, add_playlist_items, cancel_application_update,
    cancel_audio_compression, cancel_audio_compression_scan, cancel_ffmpeg_dependency_install,
    check_application_update, clear_audio_compression_inputs, clear_default_playlist,
    clear_playlist_items, confirm_application_exit, create_playlist, delete_playlist,
    desktop_lyrics_window_ready, fit_desktop_lyrics_window, get_application_lifetime_state,
    get_application_update_state, get_audio_compression_scan_state, get_audio_compression_state,
    get_default_playlist, get_desktop_lyrics_window_state, get_ffmpeg_dependency_state,
    get_main_window_state, get_now_playing_state, get_playback_state, get_track_details,
    hide_desktop_lyrics_window, install_application_update, install_ffmpeg_dependency,
    list_playlist_items, list_playlists, lock_desktop_lyrics_window, main_window_ready,
    move_default_playlist_item, move_playlist, move_playlist_item, next_playback,
    open_external_url, open_main_settings, open_media_context, open_project_page, pause_playback,
    play_default_playlist_item, play_queue_item, play_user_playlist_item, previous_playback,
    refresh_output_devices, remove_audio_compression_inputs, remove_default_playlist_items,
    remove_playlist_item, remove_playlist_items, rename_playlist, resolve_main_window_close,
    resume_playback, scan_audio_compression_inputs, seek_playback, select_output_device,
    set_close_behavior, set_main_window_layout_mode, set_playback_mode, set_playback_volume,
    set_receive_prerelease_updates, show_audio_compression_window, show_desktop_lyrics_window,
    start_audio_compression, start_desktop_lyrics_drag, sync_window_theme,
    unlock_desktop_lyrics_window,
};
use compression::CompressionService;
use ffmpeg_dependency::FfmpegDependencyService;
use lyrics::LyricsService;
use media_import::MediaImportService;
use metadata::MetadataService;
use persistence::{PersistenceService, PlaybackSessionRecord};
use platform::desktop_lyrics::DesktopLyricsWindowService;
use platform::playback_projection::NativePlaybackProjection;
use playback::{PlaybackEngine, PlaybackMode, RestoredPlaybackSession, RodioPlaybackEngine};

pub fn run() {
    platform::initialize_process();
    let playback_engine = Arc::new(RodioPlaybackEngine::new());
    let media_import_service = Arc::new(MediaImportService::new(Arc::clone(&playback_engine)));
    let playback_for_restore = Arc::clone(&playback_engine);
    let lyrics_service = Arc::new(LyricsService::default());
    let desktop_lyrics_service = Arc::new(DesktopLyricsWindowService::default());
    let metadata_service = Arc::new(MetadataService::default());
    let metadata_for_projection = Arc::clone(&metadata_service);
    #[cfg(target_os = "windows")]
    let playback_for_smtc = Arc::clone(&playback_engine);
    #[cfg(target_os = "windows")]
    let smtc_attempted = Arc::new(AtomicBool::new(false));
    tauri::Builder::default()
        .plugin(diagnostics::plugin())
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            log::info!("second instance activation received");
            if let Err(error) = application_lifetime::show_main_window(app) {
                log::warn!(
                    "second instance main window restore failed: code={}",
                    error.code
                );
                eprintln!("second instance main window restore failed: {}", error.message);
            }
            if let Some(path) =
                media_import::external_media_path(argv, std::path::Path::new(&cwd))
            {
                dispatch_external_media(app.clone(), path);
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
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
        .manage(metadata_service)
        .setup(move |app| {
            log::info!(
                "application setup started: version={}",
                env!("CARGO_PKG_VERSION")
            );
            let legacy_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| std::io::Error::other(format!("无法定位应用数据目录：{error}")))?;
            let data_dir = app
                .path()
                .app_local_data_dir()
                .map_err(|error| std::io::Error::other(format!("无法定位本地应用数据目录：{error}")))?;
            migrate_legacy_data_directory(&legacy_data_dir, &data_dir)?;
            std::fs::create_dir_all(&data_dir)?;
            let ffmpeg_dependency = FfmpegDependencyService::new(&data_dir);
            let (ffmpeg, ffprobe) = ffmpeg_dependency.binary_paths();
            app.manage(Arc::new(CompressionService::with_binaries(ffmpeg, ffprobe)));
            app.manage(ffmpeg_dependency);
            app.manage(Arc::new(ApplicationLifetimeService::load(&data_dir)));
            app.manage(Arc::new(ApplicationUpdateService::load(&data_dir)));
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
            let native_projection = Arc::new(
                NativePlaybackProjection::start(
                    Arc::clone(app.state::<Arc<RodioPlaybackEngine>>().inner()),
                    Arc::clone(&metadata_for_projection),
                )
                .map_err(std::io::Error::other)?,
            );
            let tray = platform::tray::TrayService::create(
                app.handle(),
                native_projection.subscribe(),
            )
            .map_err(|error| std::io::Error::other(format!("无法创建系统托盘：{error}")))?;
            app.manage(native_projection);
            app.manage(tray);
            log::info!("application services initialized");
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
            get_application_lifetime_state,
            set_close_behavior,
            resolve_main_window_close,
            confirm_application_exit,
            get_application_update_state,
            set_receive_prerelease_updates,
            check_application_update,
            install_application_update,
            cancel_application_update,
            open_project_page,
            open_external_url,
            get_main_window_state,
            set_main_window_layout_mode,
            main_window_ready,
            sync_window_theme,
            get_now_playing_state,
            get_track_details,
            start_audio_compression,
            get_audio_compression_state,
            get_ffmpeg_dependency_state,
            install_ffmpeg_dependency,
            cancel_ffmpeg_dependency_install,
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
            move_playlist_item
        ])
        .build(tauri::generate_context!())
        .expect("failed to build Resona")
        .run(move |app_handle, event| {
            if let tauri::RunEvent::WindowEvent {
                label,
                event: WindowEvent::CloseRequested { api, .. },
                ..
            } = &event
            {
                if label == compression_window::LABEL {
                    compression_window::persist_geometry(app_handle);
                } else if label == "main" {
                    api.prevent_close();
                    app_handle
                        .state::<Arc<main_window::MainWindowService>>()
                        .capture(app_handle);
                    match app_handle
                        .state::<Arc<ApplicationLifetimeService>>()
                        .handle_close(app_handle)
                    {
                        Ok(CloseDisposition::Exit) => request_application_exit(app_handle),
                        Ok(CloseDisposition::KeepRunning) => {}
                        Err(error) => {
                            eprintln!("main window close handling failed: {}", error.message);
                        }
                    }
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
                                let resource_dir = app_handle.path().resource_dir();
                                let logo_path = resource_dir
                                    .as_ref()
                                    .map(|directory| directory.join("icons").join("128x128.png"))
                                    .unwrap_or_else(|_| {
                                        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                                            .join("icons")
                                            .join("128x128.png")
                                    });
                                match platform::media_session::MediaSessionAdapter::start(
                                    hwnd.0 as isize,
                                    Arc::clone(&playback),
                                    &logo_path,
                                    app_handle
                                        .state::<Arc<NativePlaybackProjection>>()
                                        .subscribe(),
                                ) {
                                    Ok(adapter) => {
                                        app_handle.manage(adapter);
                                        log::info!("Windows SMTC initialized");
                                        eprintln!("Windows SMTC initialized");
                                    }
                                    Err(error) => {
                                        log::warn!("Windows SMTC unavailable");
                                        eprintln!("Windows SMTC unavailable; continuing without it: {error}");
                                    }
                                }
                                match resource_dir {
                                    Ok(resource_dir) => {
                                        match platform::taskbar::TaskbarAdapter::start(
                                            &app_handle,
                                            hwnd.0 as isize,
                                            playback,
                                            resource_dir,
                                            app_handle
                                                .state::<Arc<NativePlaybackProjection>>()
                                                .subscribe(),
                                        ) {
                                            Ok(adapter) => {
                                                app_handle.manage(adapter);
                                                log::info!(
                                                    "Windows taskbar media controls initialized"
                                                );
                                                eprintln!("Windows taskbar media controls initialized");
                                            }
                                            Err(error) => {
                                                log::warn!(
                                                    "Windows taskbar media controls unavailable"
                                                );
                                                eprintln!(
                                                    "Windows taskbar media controls unavailable; continuing without them: {error}"
                                                );
                                            }
                                        }
                                    }
                                    Err(error) => {
                                        log::warn!("Windows taskbar resources unavailable");
                                        eprintln!(
                                            "Windows taskbar resources unavailable; continuing without media controls: {error}"
                                        );
                                    }
                                }
                                return;
                            }
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                        log::warn!("Windows media integration timed out waiting for main HWND");
                        eprintln!("Windows SMTC unavailable; main window HWND was not ready");
                    });
            }
        });
}

pub(crate) fn request_application_exit(app: &tauri::AppHandle) {
    if app.state::<Arc<CompressionService>>().has_active_work() {
        log::info!("application exit deferred for active compression");
        if let Err(error) = application_lifetime::show_main_window(app) {
            eprintln!(
                "main window restore for active compression confirmation failed: {}",
                error.message
            );
        }
        if let Err(error) = app.emit_to("main", "resona://exit-confirmation-requested", ()) {
            eprintln!("active compression exit confirmation event failed: {error}");
        }
        return;
    }
    perform_application_exit(app);
}

pub(crate) fn perform_application_exit(app: &tauri::AppHandle) {
    let lifetime = app.state::<Arc<ApplicationLifetimeService>>();
    if !lifetime.begin_exit() {
        return;
    }
    log::info!("application exit started");
    app.state::<Arc<main_window::MainWindowService>>()
        .capture(app);
    compression_window::persist_geometry(app);
    app.state::<Arc<DesktopLyricsWindowService>>()
        .persist_geometry(app);
    app.state::<Arc<CompressionService>>().shutdown();
    persist_playback_session(app);
    app.exit(0);
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

fn migrate_legacy_data_directory(
    legacy_data_dir: &std::path::Path,
    local_data_dir: &std::path::Path,
) -> std::io::Result<()> {
    if legacy_data_dir == local_data_dir || !legacy_data_dir.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(local_data_dir)?;
    const APP_OWNED_FILES: &[&str] = &[
        "resona.sqlite3",
        "resona.sqlite3-wal",
        "resona.sqlite3-shm",
        "main-window.json",
        "audio-compression-window.json",
        "desktop-lyrics-window.json",
        "application-lifetime.json",
    ];
    for name in APP_OWNED_FILES {
        let source = legacy_data_dir.join(name);
        let target = local_data_dir.join(name);
        if !source.is_file() || target.exists() {
            continue;
        }
        if let Err(rename_error) = std::fs::rename(&source, &target) {
            std::fs::copy(&source, &target).map_err(|copy_error| {
                std::io::Error::other(format!(
                    "failed to migrate {} from roaming to local app data (rename: {rename_error}; copy: {copy_error})",
                    source.display()
                ))
            })?;
            std::fs::remove_file(&source)?;
        }
    }
    if legacy_data_dir
        .read_dir()
        .is_ok_and(|mut entries| entries.next().is_none())
    {
        let _ = std::fs::remove_dir(legacy_data_dir);
    }
    Ok(())
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
    tauri::async_runtime::spawn(async move {
        let result =
            tauri::async_runtime::spawn_blocking(move || media_import.open_media_context(&path))
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

#[cfg(test)]
mod tests {
    use super::migrate_legacy_data_directory;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn legacy_roaming_data_moves_to_local_without_overwriting_newer_state() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "resona-data-migration-{}-{unique}",
            std::process::id()
        ));
        let roaming = root.join("roaming");
        let local = root.join("local");
        fs::create_dir_all(&roaming).expect("roaming directory");
        fs::create_dir_all(&local).expect("local directory");
        fs::write(roaming.join("resona.sqlite3"), b"legacy").expect("legacy database");
        fs::write(roaming.join("main-window.json"), b"legacy-window").expect("legacy window");
        fs::write(local.join("main-window.json"), b"local-window").expect("local window");

        migrate_legacy_data_directory(&roaming, &local).expect("migration");

        assert_eq!(
            fs::read(local.join("resona.sqlite3")).expect("migrated database"),
            b"legacy"
        );
        assert_eq!(
            fs::read(local.join("main-window.json")).expect("preserved local window"),
            b"local-window"
        );
        assert!(roaming.join("main-window.json").is_file());
        fs::remove_dir_all(root).expect("cleanup");
    }
}
