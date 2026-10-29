// SPDX-License-Identifier: GPL-3.0-only

use crate::playback::{PlaybackErrorCode, PlaybackFailure};
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaSelection {
    Audio,
    Wav,
    Folders,
}

static OPEN: AtomicBool = AtomicBool::new(false);

pub fn select(
    window: tauri::WebviewWindow,
    kind: MediaSelection,
    multiple: bool,
) -> Result<Vec<String>, PlaybackFailure> {
    if OPEN.swap(true, Ordering::AcqRel) {
        return Ok(Vec::new());
    }
    struct Pending;
    impl Drop for Pending {
        fn drop(&mut self) {
            OPEN.store(false, Ordering::Release);
        }
    }
    let _pending = Pending;
    select_native(&window, kind, multiple)
}

fn failure() -> PlaybackFailure {
    PlaybackFailure {
        code: PlaybackErrorCode::DialogOpenFailed,
        message: "Native file selection failed; check the desktop portal service".into(),
    }
}

#[cfg(target_os = "linux")]
fn select_native(
    window: &tauri::WebviewWindow,
    kind: MediaSelection,
    multiple: bool,
) -> Result<Vec<String>, PlaybackFailure> {
    use ashpd::desktop::{
        file_chooser::{FileFilter, OpenFileRequest},
        ResponseError,
    };
    let parent = portal_parent(window)?;
    tauri::async_runtime::block_on(async {
        let mut request = OpenFileRequest::default()
            .identifier(Some(parent))
            .title("Resona")
            .multiple(multiple)
            .modal(true)
            .directory(matches!(kind, MediaSelection::Folders));
        request = match kind {
            MediaSelection::Audio => request.filter(
                FileFilter::new("MP3 / WAV / FLAC")
                    .glob("*.[mM][pP]3")
                    .glob("*.[wW][aA][vV]")
                    .glob("*.[fF][lL][aA][cC]"),
            ),
            MediaSelection::Wav => request.filter(FileFilter::new("WAV").glob("*.[wW][aA][vV]")),
            MediaSelection::Folders => request,
        };
        let response = request.send().await.map_err(|error| {
            log::warn!("File chooser request failed: {error}");
            failure()
        })?;
        match response.response() {
            Ok(selection) => selection
                .uris()
                .iter()
                .map(|uri| {
                    super::file_access::resolve_location(uri.as_str())
                        .map(|path| path.to_string_lossy().into_owned())
                })
                .collect(),
            Err(ashpd::Error::Response(ResponseError::Cancelled)) => Ok(Vec::new()),
            Err(error) => {
                log::warn!("File chooser response failed: {error}");
                Err(failure())
            }
        }
    })
}

#[cfg(not(target_os = "linux"))]
fn select_native(
    window: &tauri::WebviewWindow,
    kind: MediaSelection,
    multiple: bool,
) -> Result<Vec<String>, PlaybackFailure> {
    use tauri_plugin_dialog::DialogExt;
    let mut dialog = window
        .dialog()
        .file()
        .set_parent(window)
        .set_title("Resona");
    dialog = match kind {
        MediaSelection::Audio => dialog.add_filter("MP3 / WAV / FLAC", &["mp3", "wav", "flac"]),
        MediaSelection::Wav => dialog.add_filter("WAV", &["wav"]),
        MediaSelection::Folders => dialog,
    };
    let paths = if matches!(kind, MediaSelection::Folders) {
        dialog.blocking_pick_folders()
    } else if multiple {
        dialog.blocking_pick_files()
    } else {
        dialog.blocking_pick_file().map(|path| vec![path])
    };
    paths
        .unwrap_or_default()
        .into_iter()
        .map(|path| {
            path.into_path()
                .map(|path| path.to_string_lossy().into_owned())
                .map_err(|_| failure())
        })
        .collect()
}

#[cfg(target_os = "linux")]
fn portal_parent(
    window: &tauri::WebviewWindow,
) -> Result<ashpd::WindowIdentifier, PlaybackFailure> {
    use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
    let handle = window.window_handle().map_err(|_| failure())?.as_raw();
    let display = window.display_handle().map_err(|_| failure())?.as_raw();
    // The owning Tauri window outlives the exported parent and the portal request.
    tauri::async_runtime::block_on(ashpd::WindowIdentifier::from_raw_handle(
        &handle,
        Some(&display),
    ))
    .ok_or_else(failure)
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires KDE Wayland; run alone with isolated XDG data/cache"]
    fn native_portal_parent_exports_from_tauri_window() {
        let (sent, received) = std::sync::mpsc::sync_channel(1);
        let mut context = tauri::generate_context!();
        context.config_mut().app.windows.clear();
        let app = tauri::Builder::default()
            .any_thread()
            .setup(move |app| {
                let window = tauri::WebviewWindowBuilder::new(
                    app,
                    "portal-test",
                    tauri::WebviewUrl::External("about:blank".parse().unwrap()),
                )
                .title("Resona portal integration test")
                .inner_size(500.0, 200.0)
                .build()?;
                let app = app.handle().clone();
                std::thread::spawn(move || {
                    let checked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        std::thread::sleep(std::time::Duration::from_millis(300));
                        let parent = portal_parent(&window).expect("export native parent");
                        assert!(parent.to_string().starts_with("wayland:"));
                        use gtk::{gio, glib::variant::ToVariant};
                        let bus = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE)
                            .unwrap();
                        let version = bus
                            .call_sync(
                                Some("org.freedesktop.portal.Desktop"),
                                "/org/freedesktop/portal/desktop",
                                "org.freedesktop.DBus.Properties",
                                "Get",
                                Some(
                                    &("org.freedesktop.portal.FileChooser", "version").to_variant(),
                                ),
                                None,
                                gio::DBusCallFlags::NONE,
                                5_000,
                                gio::Cancellable::NONE,
                            )
                            .expect("native portal FileChooser interface");
                        assert!(
                            version
                                .get::<(gtk::glib::Variant,)>()
                                .unwrap()
                                .0
                                .get::<u32>()
                                .unwrap()
                                > 0
                        );
                        if std::env::var_os("RESONA_TEST_SELECT_NFS").is_some() {
                            let paths = select_native(&window, MediaSelection::Audio, true)
                                .expect("select NFS audio in the native chooser");
                            assert!(!paths.is_empty(), "select at least one NFS audio file");
                            for path in paths {
                                use std::io::Read;
                                let path = std::path::Path::new(&path);
                                assert!(!super::super::file_access::supports_directory_discovery(
                                    path
                                ));
                                let mut header = [0; 4];
                                std::fs::File::open(path)
                                    .unwrap()
                                    .read_exact(&mut header)
                                    .unwrap();
                                let context = crate::filesystem::audio_file_context(path).unwrap();
                                assert_eq!(context.sources.len(), 1);
                            }
                            assert!(select_native(&window, MediaSelection::Audio, false)
                                .expect("cancel the second native chooser")
                                .is_empty());
                        }
                    }));
                    sent.send(checked.is_ok()).unwrap();
                    app.exit(0);
                });
                Ok(())
            })
            .build(context)
            .unwrap();
        assert_eq!(app.run_return(|_, _| {}), 0);
        assert!(received
            .recv_timeout(std::time::Duration::from_secs(30))
            .unwrap());
    }
}
