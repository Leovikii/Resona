// SPDX-License-Identifier: GPL-3.0-only

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

const STATE_FILE: &str = "application-lifetime.json";
const CLOSE_REQUESTED_EVENT: &str = "resona://close-requested";

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseBehavior {
    #[default]
    Ask,
    HideToTray,
    Exit,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CloseDecision {
    HideToTray,
    Exit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloseDisposition {
    KeepRunning,
    Exit,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationLifetimeSnapshot {
    pub close_behavior: CloseBehavior,
}

#[derive(Clone, Debug, Serialize)]
pub struct ApplicationLifetimeFailure {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredApplicationLifetime {
    close_behavior: CloseBehavior,
}

pub struct ApplicationLifetimeService {
    path: PathBuf,
    state: Mutex<StoredApplicationLifetime>,
    exiting: AtomicBool,
}

impl ApplicationLifetimeService {
    pub fn load(data_dir: &Path) -> Self {
        let path = data_dir.join(STATE_FILE);
        let state = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|error| {
                eprintln!("application lifetime state is invalid; using defaults: {error}");
                StoredApplicationLifetime::default()
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                StoredApplicationLifetime::default()
            }
            Err(error) => {
                eprintln!("application lifetime state could not be read; using defaults: {error}");
                StoredApplicationLifetime::default()
            }
        };
        Self {
            path,
            state: Mutex::new(state),
            exiting: AtomicBool::new(false),
        }
    }

    pub fn snapshot(&self) -> Result<ApplicationLifetimeSnapshot, ApplicationLifetimeFailure> {
        Ok(ApplicationLifetimeSnapshot {
            close_behavior: self.lock_state()?.close_behavior,
        })
    }

    pub fn set_close_behavior(
        &self,
        behavior: CloseBehavior,
    ) -> Result<ApplicationLifetimeSnapshot, ApplicationLifetimeFailure> {
        self.lock_state()?.close_behavior = behavior;
        self.persist()?;
        self.snapshot()
    }

    pub fn handle_close(
        &self,
        app: &AppHandle,
    ) -> Result<CloseDisposition, ApplicationLifetimeFailure> {
        match self.snapshot()?.close_behavior {
            CloseBehavior::Ask => {
                show_main_window(app)?;
                app.emit_to("main", CLOSE_REQUESTED_EVENT, ())
                    .map_err(|error| failure("close_prompt_emit_failed", error))?;
                Ok(CloseDisposition::KeepRunning)
            }
            CloseBehavior::HideToTray => {
                hide_main_window(app)?;
                Ok(CloseDisposition::KeepRunning)
            }
            CloseBehavior::Exit => Ok(CloseDisposition::Exit),
        }
    }

    pub fn resolve_close(
        &self,
        app: &AppHandle,
        decision: CloseDecision,
        remember: bool,
    ) -> Result<CloseDisposition, ApplicationLifetimeFailure> {
        if remember {
            self.set_close_behavior(match decision {
                CloseDecision::HideToTray => CloseBehavior::HideToTray,
                CloseDecision::Exit => CloseBehavior::Exit,
            })?;
        }
        match decision {
            CloseDecision::HideToTray => {
                hide_main_window(app)?;
                Ok(CloseDisposition::KeepRunning)
            }
            CloseDecision::Exit => Ok(CloseDisposition::Exit),
        }
    }

    pub fn begin_exit(&self) -> bool {
        !self.exiting.swap(true, Ordering::AcqRel)
    }

    fn persist(&self) -> Result<(), ApplicationLifetimeFailure> {
        let state = self.lock_state()?.clone();
        let parent = self.path.parent().ok_or_else(|| {
            failure(
                "application_lifetime_path_invalid",
                "application lifetime state path has no parent directory",
            )
        })?;
        std::fs::create_dir_all(parent)
            .map_err(|error| failure("application_lifetime_directory_failed", error))?;
        let bytes = serde_json::to_vec(&state)
            .map_err(|error| failure("application_lifetime_serialize_failed", error))?;
        std::fs::write(&self.path, bytes)
            .map_err(|error| failure("application_lifetime_write_failed", error))
    }

    fn lock_state(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, StoredApplicationLifetime>, ApplicationLifetimeFailure>
    {
        self.state.lock().map_err(|_| {
            failure(
                "application_lifetime_state_poisoned",
                "application lifetime state lock is unavailable",
            )
        })
    }
}

pub fn show_main_window(app: &AppHandle) -> Result<(), ApplicationLifetimeFailure> {
    let window = app.get_webview_window("main").ok_or_else(|| {
        failure(
            "main_window_unavailable",
            "main window is unavailable for application lifetime operation",
        )
    })?;
    window
        .unminimize()
        .map_err(|error| failure("main_window_restore_failed", error))?;
    window
        .show()
        .map_err(|error| failure("main_window_show_failed", error))?;
    window
        .set_focus()
        .map_err(|error| failure("main_window_focus_failed", error))
}

fn hide_main_window(app: &AppHandle) -> Result<(), ApplicationLifetimeFailure> {
    let window = app.get_webview_window("main").ok_or_else(|| {
        failure(
            "main_window_unavailable",
            "main window is unavailable for application lifetime operation",
        )
    })?;
    window
        .hide()
        .map_err(|error| failure("main_window_hide_failed", error))
}

fn failure(code: &str, error: impl std::fmt::Display) -> ApplicationLifetimeFailure {
    ApplicationLifetimeFailure {
        code: code.to_owned(),
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn close_behavior_defaults_to_ask_and_round_trips() {
        let directory = temporary_directory();
        let service = ApplicationLifetimeService::load(&directory);
        assert_eq!(
            service.snapshot().expect("default snapshot").close_behavior,
            CloseBehavior::Ask
        );
        service
            .set_close_behavior(CloseBehavior::HideToTray)
            .expect("persist close behavior");

        let restored = ApplicationLifetimeService::load(&directory);
        assert_eq!(
            restored
                .snapshot()
                .expect("restored snapshot")
                .close_behavior,
            CloseBehavior::HideToTray
        );
        std::fs::remove_dir_all(directory).expect("remove lifetime fixture");
    }

    fn temporary_directory() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("resona-lifetime-{nonce}"));
        std::fs::create_dir_all(&path).expect("create lifetime fixture");
        path
    }
}
