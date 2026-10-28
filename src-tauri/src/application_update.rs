// SPDX-License-Identifier: GPL-3.0-only

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::future::{AbortHandle, Abortable};
use semver::Version;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

mod manifest;
use manifest::{BuildIdentity, PlatformAsset};

const STATE_FILE: &str = "application-update.json";
const GITHUB_RELEASES_URL: &str =
    "https://api.github.com/repos/Leovikii/Resona/releases?per_page=100&page=";
const UPDATE_PROGRESS_EVENT: &str = "resona://application-update-progress";
const UPDATE_MANIFEST_ASSET: &str = "latest.json";
const UPDATER_PUBLIC_KEY: &str = match option_env!("RESONA_UPDATER_PUBLIC_KEY") {
    Some(value) => value,
    None => "",
};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationUpdateSnapshot {
    pub supported: bool,
    pub current_version: String,
    pub current_build: Option<String>,
    pub current_is_prerelease: bool,
    pub receive_prerelease_updates: bool,
    pub updater_configured: bool,
    pub distribution: &'static str,
    pub stage: UpdateStage,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStage {
    pub phase: String,
    pub changes: Vec<String>,
}

struct UpdateGuard<'a>(&'a AtomicBool);
impl Drop for UpdateGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationUpdateRelease {
    pub tag: String,
    pub version: String,
    pub title: String,
    pub notes: String,
    pub published_at: Option<String>,
    pub release_url: String,
    pub installer_size: Option<u64>,
    pub prerelease: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationUpdateCheckResult {
    pub current_version: String,
    pub update: Option<ApplicationUpdateRelease>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationUpdateProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ApplicationUpdateFailure {
    pub code: String,
    pub message: String,
}

impl ApplicationUpdateFailure {
    pub(crate) fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredApplicationUpdate {
    receive_prerelease_updates: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    html_url: String,
    draft: bool,
    prerelease: bool,
    published_at: Option<String>,
    assets: Vec<GithubReleaseAsset>,
}

#[derive(Clone, Debug, Deserialize)]
struct GithubReleaseAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

#[derive(Clone, Debug)]
struct SelectedRelease {
    version: Version,
    release: GithubRelease,
    manifest_url: String,
}

#[derive(Clone)]
struct VerifiedRelease {
    selected: SelectedRelease,
    asset: PlatformAsset,
}

pub struct ApplicationUpdateService {
    path: PathBuf,
    state: Mutex<StoredApplicationUpdate>,
    install_cancel: Mutex<Option<AbortHandle>>,
    selected: Mutex<Option<VerifiedRelease>>,
    stage: Mutex<UpdateStage>,
    operation_active: AtomicBool,
    cancel_requested: AtomicBool,
    prepared: Mutex<Option<(String, bool, crate::platform::distribution::PreparedUpdate)>>,
}

impl ApplicationUpdateService {
    pub fn load(data_dir: &Path) -> Self {
        let path = data_dir.join(STATE_FILE);
        let current_is_prerelease = current_version()
            .map(|version| receive_prerelease_updates_default(&version))
            .unwrap_or(false);
        let state = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|error| {
                eprintln!("application update state is invalid; using defaults: {error}");
                StoredApplicationUpdate {
                    receive_prerelease_updates: current_is_prerelease,
                }
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => StoredApplicationUpdate {
                receive_prerelease_updates: current_is_prerelease,
            },
            Err(error) => {
                eprintln!("application update state could not be read; using defaults: {error}");
                StoredApplicationUpdate {
                    receive_prerelease_updates: current_is_prerelease,
                }
            }
        };
        Self {
            path,
            state: Mutex::new(state),
            install_cancel: Mutex::new(None),
            selected: Mutex::new(None),
            stage: Mutex::new(UpdateStage::default()),
            operation_active: AtomicBool::new(false),
            cancel_requested: AtomicBool::new(false),
            prepared: Mutex::new(None),
        }
    }

    pub fn snapshot(&self) -> Result<ApplicationUpdateSnapshot, ApplicationUpdateFailure> {
        let current = current_version()?;
        Ok(ApplicationUpdateSnapshot {
            distribution: if cfg!(target_os = "linux") {
                "pacman"
            } else {
                "nsis"
            },
            stage: self.stage.lock().map_err(updater_failure)?.clone(),
            supported: crate::platform::distribution::supports_updater(),
            current_version: current.to_string(),
            current_build: BuildIdentity::current().map(|build| build.id()),
            current_is_prerelease: !current.pre.is_empty(),
            receive_prerelease_updates: self.lock_state()?.receive_prerelease_updates,
            updater_configured: crate::platform::distribution::supports_updater()
                && !UPDATER_PUBLIC_KEY.trim().is_empty(),
        })
    }

    pub fn set_receive_prerelease_updates(
        &self,
        enabled: bool,
    ) -> Result<ApplicationUpdateSnapshot, ApplicationUpdateFailure> {
        let _guard = self.begin_operation("update_install_busy")?;
        self.prepared.lock().map_err(updater_failure)?.take();
        self.selected.lock().map_err(updater_failure)?.take();
        *self.stage.lock().map_err(updater_failure)? = UpdateStage::default();
        let previous = {
            let mut state = self.lock_state()?;
            let previous = state.receive_prerelease_updates;
            state.receive_prerelease_updates = enabled;
            previous
        };
        if let Err(error) = self.persist() {
            self.lock_state()?.receive_prerelease_updates = previous;
            return Err(error);
        }
        self.snapshot()
    }

    pub async fn check(
        self: &Arc<Self>,
    ) -> Result<ApplicationUpdateCheckResult, ApplicationUpdateFailure> {
        let _guard = self.begin_operation("update_check_busy")?;
        self.prepared.lock().map_err(updater_failure)?.take();
        *self.stage.lock().map_err(updater_failure)? = UpdateStage::default();
        let service = Arc::clone(self);
        let result = tauri::async_runtime::spawn_blocking(move || service.discover_update())
            .await
            .map_err(|error| {
                ApplicationUpdateFailure::new(
                    "update_check_task_failed",
                    format!("update check task failed: {error}"),
                )
            });
        result?
    }

    pub async fn install(
        self: &Arc<Self>,
        app: AppHandle,
        expected_tag: String,
        confirm: bool,
    ) -> Result<(), ApplicationUpdateFailure> {
        let _guard = self.begin_operation("update_install_busy")?;
        self.cancel_requested.store(false, Ordering::Release);
        let result = self.install_inner(app, expected_tag, confirm).await;
        if result.is_err() {
            *self.stage.lock().map_err(updater_failure)? = UpdateStage::default();
            self.prepared.lock().map_err(updater_failure)?.take();
        }
        result
    }

    fn begin_operation(&self, code: &str) -> Result<UpdateGuard<'_>, ApplicationUpdateFailure> {
        self.operation_active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| {
                ApplicationUpdateFailure::new(code, "Another update operation is running")
            })?;
        Ok(UpdateGuard(&self.operation_active))
    }

    pub fn is_busy(&self) -> bool {
        self.operation_active.load(Ordering::Acquire)
            && self
                .stage
                .lock()
                .map(|stage| {
                    matches!(
                        stage.phase.as_str(),
                        "downloading" | "verifying" | "preparing" | "installing"
                    )
                })
                .unwrap_or(true)
    }

    fn phase(&self, phase: &str) -> Result<(), ApplicationUpdateFailure> {
        self.stage.lock().map_err(updater_failure)?.phase = phase.to_owned();
        Ok(())
    }

    async fn install_inner(
        self: &Arc<Self>,
        app: AppHandle,
        expected_tag: String,
        confirm: bool,
    ) -> Result<(), ApplicationUpdateFailure> {
        if !crate::platform::distribution::supports_updater()
            || UPDATER_PUBLIC_KEY.trim().is_empty()
        {
            return Err(ApplicationUpdateFailure::new(
                "updater_not_configured",
                "this build does not contain an updater verification public key",
            ));
        }
        let verified = self
            .selected
            .lock()
            .map_err(updater_failure)?
            .clone()
            .filter(|release| release.selected.release.tag_name == expected_tag)
            .ok_or_else(|| {
                ApplicationUpdateFailure::new(
                    "update_selection_expired",
                    "check for updates again before installing",
                )
            })?;
        let selected = verified.selected;
        let current = current_version()?;
        let enabled = self.lock_state()?.receive_prerelease_updates;
        if !manifest::is_newer(&selected.version, &current, enabled) {
            return Err(ApplicationUpdateFailure::new(
                "update_selection_expired",
                "selected version is not an allowed update",
            ));
        }
        if confirm {
            let (tag, preview, plan) = self
                .prepared
                .lock()
                .map_err(updater_failure)?
                .take()
                .ok_or_else(|| {
                    ApplicationUpdateFailure::new(
                        "update_selection_expired",
                        "Download the update again",
                    )
                })?;
            if tag != expected_tag || preview != enabled {
                return Err(ApplicationUpdateFailure::new(
                    "update_selection_expired",
                    "Update preference changed",
                ));
            }
            self.phase("installing")?;
            let compression = Arc::clone(
                app.state::<Arc<crate::compression::CompressionService>>()
                    .inner(),
            );
            tauri::async_runtime::spawn_blocking(move || {
                let _guard = compression
                    .system_install_guard()
                    .map_err(|e| ApplicationUpdateFailure::new(&e.code, e.message))?;
                plan.install()
            })
            .await
            .map_err(updater_failure)??;
            self.phase("complete")?;
            return Ok(());
        }
        self.phase("downloading")?;
        let endpoint: tauri::Url = selected.manifest_url.parse().map_err(|error| {
            ApplicationUpdateFailure::new(
                "update_manifest_url_invalid",
                format!("invalid updater manifest URL: {error}"),
            )
        })?;
        let updater = app
            .updater_builder()
            .endpoints(vec![endpoint])
            .map_err(updater_failure)?
            .pubkey(UPDATER_PUBLIC_KEY)
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(updater_failure)?;
        let mut update = updater
            .check()
            .await
            .map_err(updater_failure)?
            .ok_or_else(|| {
                ApplicationUpdateFailure::new(
                    "update_no_longer_available",
                    "the selected update is no longer newer than this installation",
                )
            })?;
        if update.version != selected.version.to_string()
            || update.download_url.as_str() != verified.asset.url
            || update.signature != verified.asset.signature
        {
            return Err(ApplicationUpdateFailure::new(
                "update_manifest_version_mismatch",
                format!(
                    "release version {} does not match updater manifest version {}",
                    selected.version, update.version
                ),
            ));
        }

        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        {
            let mut active = self.lock_install_cancel()?;
            if self.cancel_requested.load(Ordering::Acquire) {
                return Err(ApplicationUpdateFailure::new(
                    "update_cancelled",
                    "Download cancelled",
                ));
            }
            if active.is_some() {
                return Err(ApplicationUpdateFailure::new(
                    "update_install_busy",
                    "an application update is already being downloaded",
                ));
            }
            *active = Some(abort_handle);
        }

        // Updater 2.10.1 does not carry the check timeout into Update::download.
        update.timeout = Some(Duration::from_secs(600));
        let mut downloaded_bytes = 0_u64;
        let progress_app = app.clone();
        let finish_service = Arc::clone(self);
        let download = update.download(
            move |chunk_size, total_bytes| {
                downloaded_bytes = downloaded_bytes.saturating_add(chunk_size as u64);
                if let Err(error) = progress_app.emit_to(
                    "main",
                    UPDATE_PROGRESS_EVENT,
                    ApplicationUpdateProgress {
                        downloaded_bytes,
                        total_bytes,
                    },
                ) {
                    eprintln!("application update progress event failed: {error}");
                }
            },
            move || {
                if let Err(error) = finish_service.phase("verifying") {
                    log::warn!("{}", error.message);
                }
            },
        );
        let bytes = Abortable::new(download, abort_registration)
            .await
            .map_err(|_| {
                ApplicationUpdateFailure::new(
                    "update_cancelled",
                    "application update download was cancelled",
                )
            });
        self.lock_install_cancel()?.take();
        let bytes = bytes?.map_err(updater_failure)?;
        if cfg!(target_os = "linux") {
            self.phase("preparing")?;
            let version = selected.version.to_string();
            let plan = tauri::async_runtime::spawn_blocking(move || {
                crate::platform::distribution::prepare(&version, &bytes)
            })
            .await
            .map_err(updater_failure)??;
            let changes = plan.changes();
            *self.prepared.lock().map_err(updater_failure)? = Some((expected_tag, enabled, plan));
            *self.stage.lock().map_err(updater_failure)? = UpdateStage {
                phase: "prepared".into(),
                changes,
            };
            Ok(())
        } else {
            self.phase("installing")?;
            update.install(bytes).map_err(updater_failure)?;
            self.phase("")
        }
    }

    pub fn cancel_install(&self) -> Result<(), ApplicationUpdateFailure> {
        if self.stage.lock().map_err(updater_failure)?.phase != "downloading" {
            return Err(ApplicationUpdateFailure::new(
                "update_cannot_cancel",
                "This installation stage cannot be cancelled",
            ));
        }
        self.cancel_requested.store(true, Ordering::Release);
        if let Some(cancel) = self.lock_install_cancel()?.take() {
            cancel.abort();
        }
        Ok(())
    }

    fn discover_update(&self) -> Result<ApplicationUpdateCheckResult, ApplicationUpdateFailure> {
        let current = current_version()?;
        if !crate::platform::distribution::supports_updater() {
            return Ok(ApplicationUpdateCheckResult {
                current_version: current.to_string(),
                update: None,
            });
        }
        if UPDATER_PUBLIC_KEY.trim().is_empty() {
            return Ok(ApplicationUpdateCheckResult {
                current_version: current.to_string(),
                update: None,
            });
        }
        let include_prerelease = self.lock_state()?.receive_prerelease_updates;
        let candidate = select_release(fetch_github_releases()?, include_prerelease)
            .filter(|release| manifest::is_newer(&release.version, &current, include_prerelease));
        let verified = candidate
            .map(|selected| {
                let bytes = fetch_asset(&selected.manifest_url, 256 * 1024)?;
                let signature_url = selected
                    .release
                    .assets
                    .iter()
                    .find(|asset| asset.name == "latest.json.sig")
                    .ok_or_else(|| updater_failure("missing manifest signature"))?
                    .browser_download_url
                    .clone();
                let signature = String::from_utf8(fetch_asset(&signature_url, 16 * 1024)?)
                    .map_err(updater_failure)?;
                let asset = manifest::verified_asset(
                    &bytes,
                    &signature,
                    UPDATER_PUBLIC_KEY,
                    &selected.release.tag_name,
                    &selected.version,
                    crate::platform::distribution::platform_key(),
                )?;
                if !selected
                    .release
                    .assets
                    .iter()
                    .any(|entry| entry.browser_download_url == asset.url)
                {
                    return Err(updater_failure(
                        "signed installer is missing from selected release",
                    ));
                }
                Ok(VerifiedRelease { selected, asset })
            })
            .transpose()?;
        let update = verified
            .as_ref()
            .map(|release| release_snapshot(&release.selected));
        *self.selected.lock().map_err(updater_failure)? = verified;
        Ok(ApplicationUpdateCheckResult {
            current_version: current.to_string(),
            update,
        })
    }

    fn persist(&self) -> Result<(), ApplicationUpdateFailure> {
        let state = self.lock_state()?.clone();
        let parent = self.path.parent().ok_or_else(|| {
            ApplicationUpdateFailure::new(
                "update_state_path_invalid",
                "application update state path has no parent directory",
            )
        })?;
        std::fs::create_dir_all(parent).map_err(|error| {
            ApplicationUpdateFailure::new("update_state_directory_failed", error.to_string())
        })?;
        let bytes = serde_json::to_vec(&state).map_err(|error| {
            ApplicationUpdateFailure::new("update_state_serialize_failed", error.to_string())
        })?;
        std::fs::write(&self.path, bytes).map_err(|error| {
            ApplicationUpdateFailure::new("update_state_write_failed", error.to_string())
        })
    }

    fn lock_state(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, StoredApplicationUpdate>, ApplicationUpdateFailure> {
        self.state.lock().map_err(|_| {
            ApplicationUpdateFailure::new(
                "update_state_poisoned",
                "application update state lock is unavailable",
            )
        })
    }

    fn lock_install_cancel(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, Option<AbortHandle>>, ApplicationUpdateFailure> {
        self.install_cancel.lock().map_err(|_| {
            ApplicationUpdateFailure::new(
                "update_install_state_poisoned",
                "application update install state lock is unavailable",
            )
        })
    }
}

fn current_version() -> Result<Version, ApplicationUpdateFailure> {
    Version::parse(env!("CARGO_PKG_VERSION")).map_err(|error| {
        ApplicationUpdateFailure::new(
            "current_version_invalid",
            format!("application package version is invalid: {error}"),
        )
    })
}

fn receive_prerelease_updates_default(version: &Version) -> bool {
    !version.pre.is_empty()
}

fn fetch_github_releases() -> Result<Vec<GithubRelease>, ApplicationUpdateFailure> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(5)))
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .into();
    let mut releases = Vec::new();
    for page in 1..=10 {
        let url = format!("{GITHUB_RELEASES_URL}{page}");
        let mut response = agent
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header(
                "User-Agent",
                &format!("Resona/{}", env!("CARGO_PKG_VERSION")),
            )
            .call()
            .map_err(|error| {
                ApplicationUpdateFailure::new(
                    "update_network_failed",
                    format!("GitHub Releases request failed: {error}"),
                )
            })?;
        let body = response.body_mut().read_to_string().map_err(|error| {
            ApplicationUpdateFailure::new(
                "update_response_read_failed",
                format!("GitHub Releases response could not be read: {error}"),
            )
        })?;
        let mut page_releases: Vec<GithubRelease> =
            serde_json::from_str(&body).map_err(|error| {
                ApplicationUpdateFailure::new(
                    "update_response_invalid",
                    format!("GitHub Releases response is invalid: {error}"),
                )
            })?;
        let page_is_full = page_releases.len() == 100;
        if page == 10 && page_is_full {
            return Err(updater_failure("GitHub pagination limit exceeded"));
        }
        releases.append(&mut page_releases);
        if !page_is_full {
            break;
        }
    }
    Ok(releases)
}

fn fetch_asset(url: &str, limit: u64) -> Result<Vec<u8>, ApplicationUpdateFailure> {
    use std::io::Read;
    if !url.starts_with("https://github.com/Leovikii/Resona/releases/download/") {
        return Err(updater_failure(
            "update asset URL is outside the release repository",
        ));
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(5)))
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .into();
    let mut response = agent.get(url).call().map_err(updater_failure)?;
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(updater_failure)?;
    if bytes.len() as u64 > limit {
        return Err(updater_failure("update asset exceeds size limit"));
    }
    Ok(bytes)
}

fn select_release(
    releases: Vec<GithubRelease>,
    include_prerelease: bool,
) -> Option<SelectedRelease> {
    releases
        .into_iter()
        .filter_map(|release| {
            if release.draft {
                return None;
            }
            let version = manifest::parse_tag(&release.tag_name)?;
            let version_is_prerelease = !version.pre.is_empty();
            if version_is_prerelease != release.prerelease {
                return None;
            }
            if version_is_prerelease && !include_prerelease {
                return None;
            }
            let manifest_url = release
                .assets
                .iter()
                .find(|asset| asset.name == UPDATE_MANIFEST_ASSET)?
                .browser_download_url
                .clone();
            Some(SelectedRelease {
                version,
                release,
                manifest_url,
            })
        })
        .max_by(|left, right| left.version.cmp_precedence(&right.version))
}

fn release_snapshot(selected: &SelectedRelease) -> ApplicationUpdateRelease {
    let release = &selected.release;
    let installer_size = release
        .assets
        .iter()
        .find(|asset| {
            (cfg!(target_os = "linux") && asset.name.ends_with(".pkg.tar.zst"))
                || (!cfg!(target_os = "linux")
                    && (asset.name.ends_with("_windows_x86_64-setup.exe")
                        || asset.name.ends_with("_windows_x64-setup.exe")
                        || asset.name.ends_with("_windows_arm64-setup.exe")
                        || asset.name.ends_with("_windows_x86-setup.exe")))
        })
        .map(|asset| asset.size);
    ApplicationUpdateRelease {
        tag: release.tag_name.clone(),
        version: selected.version.to_string(),
        title: release
            .name
            .clone()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| format!("Resona {}", selected.version)),
        notes: release.body.clone().unwrap_or_default(),
        published_at: release.published_at.clone(),
        release_url: release.html_url.clone(),
        installer_size,
        prerelease: release.prerelease,
    }
}

fn updater_failure(error: impl std::fmt::Display) -> ApplicationUpdateFailure {
    ApplicationUpdateFailure::new("updater_failed", error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_operations_exclude_preference_changes_and_release_the_guard() {
        let service = ApplicationUpdateService::load(Path::new("/nonexistent-resona-test"));
        let guard = service.begin_operation("busy").unwrap();
        assert!(!service.is_busy());
        service.phase("installing").unwrap();
        assert!(service.is_busy());
        assert!(service.begin_operation("busy").is_err());
        assert_eq!(
            service
                .set_receive_prerelease_updates(true)
                .unwrap_err()
                .code,
            "update_install_busy"
        );
        drop(guard);
        assert!(!service.is_busy());
        assert!(service.begin_operation("busy").is_ok());
    }
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn unsupported_platforms_do_not_offer_windows_releases() {
        if crate::platform::distribution::supports_updater() {
            return;
        }
        let service = ApplicationUpdateService::load(Path::new("/nonexistent-resona-test"));
        assert!(!service.snapshot().unwrap().updater_configured);
        assert!(service.discover_update().unwrap().update.is_none());
    }

    #[test]
    fn preview_channel_uses_full_semver_ordering_across_alpha_beta_rc_and_final() {
        let selected = select_release(
            vec![
                release("v0.1.0-alpha.2", true),
                release("v0.1.0-beta.1", true),
                release("v0.1.0-rc.3", true),
                release("v0.1.0", false),
            ],
            true,
        )
        .expect("selected release");
        assert_eq!(selected.version, Version::parse("0.1.0").unwrap());
    }

    #[test]
    fn stable_channel_excludes_every_prerelease_identifier() {
        let selected = select_release(
            vec![
                release("v0.2.0-alpha.1", true),
                release("v0.2.0-beta.1", true),
                release("v0.2.0-rc.1", true),
                release("v0.1.0", false),
            ],
            false,
        )
        .expect("stable release");
        assert_eq!(selected.version, Version::parse("0.1.0").unwrap());
    }

    #[test]
    fn inconsistent_github_prerelease_flag_and_drafts_are_rejected() {
        let mut draft = release("v1.0.0", false);
        draft.draft = true;
        assert!(select_release(vec![release("v1.1.0-beta.1", false), draft], true).is_none());
    }

    #[test]
    fn an_empty_or_incompatible_release_list_means_no_update() {
        assert!(select_release(Vec::new(), true).is_none());
        assert!(select_release(vec![release("v1.0.0-beta.1", true)], false).is_none());
    }

    #[test]
    fn build_tags_are_ignored_and_version_tags_are_selected() {
        let selected = select_release(
            vec![
                release("preview-v0.3.0-alpha.1-b30.1-gaaaaaaa", true),
                release("v0.3.0-alpha.1", true),
                release("v0.3.0-alpha.2", true),
            ],
            true,
        )
        .unwrap();
        assert_eq!(selected.version, Version::parse("0.3.0-alpha.2").unwrap());
    }

    #[test]
    fn preference_default_matches_build_channel_and_round_trips() {
        assert!(!receive_prerelease_updates_default(
            &Version::parse("0.1.0").unwrap()
        ));
        assert!(receive_prerelease_updates_default(
            &Version::parse("0.1.1-rc.1").unwrap()
        ));

        let directory = temporary_directory();
        let service = ApplicationUpdateService::load(&directory);
        let expected_default =
            receive_prerelease_updates_default(&Version::parse(env!("CARGO_PKG_VERSION")).unwrap());
        assert_eq!(
            service
                .snapshot()
                .expect("default update snapshot")
                .receive_prerelease_updates,
            expected_default
        );
        service
            .set_receive_prerelease_updates(!expected_default)
            .expect("persist update preference");
        let restored = ApplicationUpdateService::load(&directory);
        assert_eq!(
            restored
                .snapshot()
                .expect("restored update snapshot")
                .receive_prerelease_updates,
            !expected_default
        );
        std::fs::remove_dir_all(directory).expect("remove update fixture");
    }

    fn release(version: &str, prerelease: bool) -> GithubRelease {
        GithubRelease {
            tag_name: version.to_owned(),
            name: None,
            body: Some("notes".to_owned()),
            html_url: format!("https://github.com/Leovikii/Resona/releases/tag/{version}"),
            draft: false,
            prerelease,
            published_at: None,
            assets: vec![
                GithubReleaseAsset {
                    name: UPDATE_MANIFEST_ASSET.to_owned(),
                    browser_download_url: format!(
                        "https://github.com/Leovikii/Resona/releases/download/{version}/latest.json"
                    ),
                    size: 500,
                },
                GithubReleaseAsset {
                    name: format!("Resona_{version}_windows_x64-setup.exe"),
                    browser_download_url: "https://example.invalid/setup.exe".to_owned(),
                    size: 1024,
                },
            ],
        }
    }

    fn temporary_directory() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("resona-update-{nonce}"));
        std::fs::create_dir_all(&path).expect("create update fixture");
        path
    }
}
