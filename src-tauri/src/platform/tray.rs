// SPDX-License-Identifier: GPL-3.0-only

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use crate::application_lifetime::show_main_window;
use crate::platform::desktop_lyrics::DesktopLyricsWindowService;
use crate::playback::{PlaybackEngine, PlaybackSnapshot, PlaybackStatus, RodioPlaybackEngine};

const SHOW_ID: &str = "tray_show";
const TITLE_ID: &str = "tray_title";
const PREVIOUS_ID: &str = "tray_previous";
const PLAY_PAUSE_ID: &str = "tray_play_pause";
const NEXT_ID: &str = "tray_next";
const LYRICS_ID: &str = "tray_desktop_lyrics";
const EXIT_ID: &str = "tray_exit";
const MAX_TITLE_CHARACTERS: usize = 10;

pub struct TrayService {
    _tray: TrayIcon,
    _title: MenuItem<tauri::Wry>,
    _previous: MenuItem<tauri::Wry>,
    _play_pause: MenuItem<tauri::Wry>,
    _next: MenuItem<tauri::Wry>,
    _lyrics: CheckMenuItem<tauri::Wry>,
    projection_commands: Sender<TrayProjectionCommand>,
    projection_worker: Mutex<Option<JoinHandle<()>>>,
}

impl TrayService {
    pub fn create(app: &AppHandle, projection: Receiver<PlaybackSnapshot>) -> tauri::Result<Self> {
        let show = MenuItem::with_id(app, SHOW_ID, "打开 Resona", true, None::<&str>)?;
        let title = MenuItem::with_id(app, TITLE_ID, "未在播放", false, None::<&str>)?;
        let previous = MenuItem::with_id(app, PREVIOUS_ID, "上一曲", false, None::<&str>)?;
        let play_pause = MenuItem::with_id(app, PLAY_PAUSE_ID, "播放", false, None::<&str>)?;
        let next = MenuItem::with_id(app, NEXT_ID, "下一曲", false, None::<&str>)?;
        let lyrics = CheckMenuItem::with_id(app, LYRICS_ID, "桌面歌词", true, false, None::<&str>)?;
        let exit = MenuItem::with_id(app, EXIT_ID, "退出 Resona", true, None::<&str>)?;
        let separator_one = PredefinedMenuItem::separator(app)?;
        let separator_two = PredefinedMenuItem::separator(app)?;
        let menu = Menu::with_items(
            app,
            &[
                &show,
                &title,
                &separator_one,
                &previous,
                &play_pause,
                &next,
                &separator_two,
                &lyrics,
                &exit,
            ],
        )?;

        let mut builder = TrayIconBuilder::with_id("resona-main")
            .menu(&menu)
            .show_menu_on_left_click(false)
            .tooltip("Resona")
            .on_menu_event(handle_menu_event)
            .on_tray_icon_event(|tray, event| {
                if matches!(
                    event,
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } | TrayIconEvent::DoubleClick {
                        button: MouseButton::Left,
                        ..
                    }
                ) {
                    if let Err(error) = show_main_window(tray.app_handle()) {
                        eprintln!("tray main window restore failed: {}", error.message);
                    }
                }
            });
        if let Some(icon) = app.default_window_icon() {
            builder = builder.icon(icon.clone());
        }
        let tray = builder.build(app)?;

        let (projection_commands, projection_worker) = spawn_projection_refresh(
            projection,
            title.clone(),
            previous.clone(),
            play_pause.clone(),
            next.clone(),
            lyrics.clone(),
            Arc::clone(app.state::<Arc<DesktopLyricsWindowService>>().inner()),
        )?;
        let service = Self {
            _tray: tray,
            _title: title.clone(),
            _previous: previous.clone(),
            _play_pause: play_pause.clone(),
            _next: next.clone(),
            _lyrics: lyrics.clone(),
            projection_commands,
            projection_worker: Mutex::new(Some(projection_worker)),
        };
        Ok(service)
    }
}

impl Drop for TrayService {
    fn drop(&mut self) {
        let _ = self
            .projection_commands
            .send(TrayProjectionCommand::Shutdown);
        if let Ok(mut worker) = self.projection_worker.lock() {
            if let Some(worker) = worker.take() {
                let _ = worker.join();
            }
        }
    }
}

fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    let id = event.id().as_ref();
    if id == SHOW_ID {
        if let Err(error) = show_main_window(app) {
            eprintln!("tray main window restore failed: {}", error.message);
        }
        return;
    }
    if id == EXIT_ID {
        crate::request_application_exit(app);
        return;
    }
    if id == LYRICS_ID {
        let service = Arc::clone(app.state::<Arc<DesktopLyricsWindowService>>().inner());
        let result = if service.snapshot().visible {
            service.hide(app).map(|_| ())
        } else {
            service.show(app, None).map(|_| ())
        };
        if let Err(error) = result {
            eprintln!("tray desktop lyrics action failed: {}", error.message);
        }
        return;
    }

    let playback = Arc::clone(app.state::<Arc<RodioPlaybackEngine>>().inner());
    let result = match id {
        PREVIOUS_ID => playback.previous().map(|_| ()),
        NEXT_ID => playback.next().map(|_| ()),
        PLAY_PAUSE_ID => playback
            .snapshot()
            .and_then(|snapshot| match snapshot.status {
                PlaybackStatus::Playing => playback.pause().map(|_| ()),
                PlaybackStatus::Paused => playback.resume().map(|_| ()),
                _ => snapshot
                    .current_item_id
                    .ok_or(crate::playback::PlaybackError::NothingLoaded)
                    .and_then(|item_id| playback.play_queue_item(item_id))
                    .map(|_| ()),
            }),
        _ => return,
    };
    if let Err(error) = result {
        eprintln!("tray action {id} failed: {error}");
    }
}

enum TrayProjectionCommand {
    Shutdown,
}

fn spawn_projection_refresh(
    projection: Receiver<PlaybackSnapshot>,
    title: MenuItem<tauri::Wry>,
    previous: MenuItem<tauri::Wry>,
    play_pause: MenuItem<tauri::Wry>,
    next: MenuItem<tauri::Wry>,
    lyrics: CheckMenuItem<tauri::Wry>,
    desktop_lyrics: Arc<DesktopLyricsWindowService>,
) -> std::io::Result<(Sender<TrayProjectionCommand>, JoinHandle<()>)> {
    let (commands, command_receiver) = mpsc::channel();
    let worker = thread::Builder::new()
        .name("resona-tray-projection".to_owned())
        .spawn(move || loop {
            if matches!(
                command_receiver.try_recv(),
                Ok(TrayProjectionCommand::Shutdown) | Err(mpsc::TryRecvError::Disconnected)
            ) {
                break;
            }
            match projection.recv_timeout(Duration::from_millis(500)) {
                Ok(snapshot) => {
                    let current = snapshot
                        .queue
                        .iter()
                        .find(|item| Some(item.id) == snapshot.current_item_id);
                    let label = current
                        .map(|item| compact_menu_title(&item.display_name))
                        .unwrap_or_else(|| "未在播放".to_owned());
                    let controllable = matches!(
                        snapshot.status,
                        PlaybackStatus::Playing | PlaybackStatus::Paused
                    );
                    let has_item = snapshot.current_item_id.is_some();
                    let _ = title.set_text(label);
                    let _ = previous.set_enabled(controllable);
                    let _ = next.set_enabled(controllable);
                    let _ = play_pause.set_enabled(has_item);
                    let _ = play_pause.set_text(if snapshot.status == PlaybackStatus::Playing {
                        "暂停"
                    } else {
                        "播放"
                    });
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
            let _ = lyrics.set_checked(desktop_lyrics.snapshot().visible);
        })?;
    Ok((commands, worker))
}

fn escape_menu_text(value: &str) -> String {
    value.replace('&', "&&")
}

fn compact_menu_title(value: &str) -> String {
    let mut characters = value.chars();
    let prefix = characters
        .by_ref()
        .take(MAX_TITLE_CHARACTERS)
        .collect::<String>();
    let title = if characters.next().is_some() {
        format!("{prefix}…")
    } else {
        prefix
    };
    escape_menu_text(&title)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_native_menu_mnemonics_in_track_titles() {
        assert_eq!(escape_menu_text("R&B & Soul"), "R&&B && Soul");
    }

    #[test]
    fn compacts_long_track_titles_without_breaking_unicode() {
        let title = "这是一首名称非常非常长而且会撑宽系统托盘菜单的测试歌曲.flac";
        let compact = compact_menu_title(title);
        assert!(compact.ends_with('…'));
        assert_eq!(compact.chars().count(), MAX_TITLE_CHARACTERS + 1);
    }
}
