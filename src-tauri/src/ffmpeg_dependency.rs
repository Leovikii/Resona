// SPDX-License-Identifier: GPL-3.0-only

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use sha2::{Digest, Sha256};

pub const FFMPEG_VERSION: &str = "8.1.2";
pub const FFMPEG_SOURCE_URL: &str =
    "https://github.com/GyanD/codexffmpeg/releases/download/8.1.2/ffmpeg-8.1.2-essentials_build.zip";
const ARCHIVE_SHA256: &str = "DB580001CAA24AC104C8CB856CD113A87B0A443F7BDF47D8C12B1D740584A2EC";
const FFMPEG_SHA256: &str = "1326DDE4C84FF1F96FE6B8916C5BED29E163E9B5DCCF995F6F3DB069D143EC5E";
const FFPROBE_SHA256: &str = "B49CCC7C6547B141AD5A2F6EC69CC04323D7133D7704D70B331B904C63EECB07";
const MAX_ARCHIVE_BYTES: u64 = 300 * 1024 * 1024;
const MAX_BINARY_BYTES: u64 = 300 * 1024 * 1024;

#[derive(Clone, Debug)]
struct DependencySpec {
    version: String,
    source_url: String,
    archive_sha256: String,
    ffmpeg_sha256: String,
    ffprobe_sha256: String,
}

impl Default for DependencySpec {
    fn default() -> Self {
        Self {
            version: FFMPEG_VERSION.to_owned(),
            source_url: FFMPEG_SOURCE_URL.to_owned(),
            archive_sha256: ARCHIVE_SHA256.to_owned(),
            ffmpeg_sha256: FFMPEG_SHA256.to_owned(),
            ffprobe_sha256: FFPROBE_SHA256.to_owned(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegDependencySnapshot {
    pub status: String,
    pub version: String,
    pub source_url: String,
    pub license: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub installed_bytes: u64,
    pub error: Option<FfmpegDependencyFailure>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FfmpegDependencyFailure {
    pub code: String,
    pub message: String,
}

impl FfmpegDependencyFailure {
    fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
        }
    }
}

pub struct FfmpegDependencyService {
    spec: DependencySpec,
    root: PathBuf,
    snapshot: Arc<Mutex<FfmpegDependencySnapshot>>,
    cancel: Arc<AtomicBool>,
}

impl FfmpegDependencyService {
    pub fn new(local_data_dir: &Path) -> Arc<Self> {
        let spec = DependencySpec::default();
        let service = Arc::new(Self {
            root: local_data_dir.join("dependencies").join("ffmpeg"),
            snapshot: Arc::new(Mutex::new(FfmpegDependencySnapshot {
                status: "checking".to_owned(),
                version: spec.version.to_owned(),
                source_url: spec.source_url.to_owned(),
                license: "GPL-3.0-or-later".to_owned(),
                downloaded_bytes: 0,
                total_bytes: None,
                installed_bytes: 0,
                error: None,
            })),
            cancel: Arc::new(AtomicBool::new(false)),
            spec,
        });
        let checker = Arc::clone(&service);
        if let Err(error) = thread::Builder::new()
            .name("resona-ffmpeg-check".to_owned())
            .spawn(move || checker.finish_initial_check())
        {
            service.fail(FfmpegDependencyFailure::new(
                "dependency_check_task_failed",
                format!("unable to start FFmpeg dependency check: {error}"),
            ));
        }
        service
    }

    pub fn snapshot(&self) -> FfmpegDependencySnapshot {
        self.snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    pub fn binary_paths(&self) -> (PathBuf, PathBuf) {
        let directory = self.install_directory();
        (directory.join("ffmpeg.exe"), directory.join("ffprobe.exe"))
    }

    pub fn require_ready(&self) -> Result<(), FfmpegDependencyFailure> {
        let snapshot = self.snapshot();
        if snapshot.status == "ready" {
            Ok(())
        } else {
            Err(FfmpegDependencyFailure::new(
                "ffmpeg_dependency_not_ready",
                format!("FFmpeg dependency is {}", snapshot.status),
            ))
        }
    }

    pub fn install(self: &Arc<Self>) -> Result<FfmpegDependencySnapshot, FfmpegDependencyFailure> {
        #[cfg(not(target_os = "windows"))]
        {
            return Err(FfmpegDependencyFailure::new(
                "ffmpeg_dependency_unsupported",
                "FFmpeg dependency download is not configured for this platform",
            ));
        }

        #[cfg(target_os = "windows")]
        {
            let mut snapshot = self
                .snapshot
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if snapshot.status == "ready" {
                return Ok(snapshot.clone());
            }
            if matches!(
                snapshot.status.as_str(),
                "checking" | "downloading" | "installing" | "cancelling"
            ) {
                return Err(FfmpegDependencyFailure::new(
                    "ffmpeg_dependency_busy",
                    "FFmpeg dependency is already being prepared",
                ));
            }
            self.cancel.store(false, Ordering::Release);
            snapshot.status = "downloading".to_owned();
            snapshot.downloaded_bytes = 0;
            snapshot.total_bytes = None;
            snapshot.error = None;
            let started = snapshot.clone();
            drop(snapshot);

            let service = Arc::clone(self);
            thread::Builder::new()
                .name("resona-ffmpeg-download".to_owned())
                .spawn(move || service.run_install())
                .map_err(|error| {
                    let failure = FfmpegDependencyFailure::new(
                        "dependency_download_task_failed",
                        format!("unable to start FFmpeg download: {error}"),
                    );
                    self.fail(failure.clone());
                    failure
                })?;
            Ok(started)
        }
    }

    pub fn cancel(&self) -> FfmpegDependencySnapshot {
        let mut snapshot = self
            .snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if matches!(snapshot.status.as_str(), "downloading" | "installing") {
            self.cancel.store(true, Ordering::Release);
            snapshot.status = "cancelling".to_owned();
        }
        snapshot.clone()
    }

    fn finish_initial_check(&self) {
        match self.validate_installed() {
            Ok(installed_bytes) => {
                let mut snapshot = self
                    .snapshot
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                snapshot.status = "ready".to_owned();
                snapshot.installed_bytes = installed_bytes;
                snapshot.error = None;
            }
            Err(error) => {
                eprintln!("FFmpeg dependency check: {}", error.message);
                let mut snapshot = self
                    .snapshot
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                snapshot.status = "missing".to_owned();
                snapshot.installed_bytes = 0;
                snapshot.error = None;
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn run_install(&self) {
        let archive_path = self
            .root
            .join(format!(".{}-download.zip", self.spec.version));
        let mut result = self.download_and_install();
        if let Err(cleanup) = remove_file_if_present(&archive_path) {
            if result.is_ok() {
                result = Err(cleanup);
            } else {
                eprintln!(
                    "FFmpeg temporary archive cleanup failed: {}",
                    cleanup.message
                );
            }
        }
        match result {
            Ok(installed_bytes) => {
                let mut snapshot = self
                    .snapshot
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                snapshot.status = "ready".to_owned();
                snapshot.installed_bytes = installed_bytes;
                snapshot.error = None;
            }
            Err(failure) if failure.code == "ffmpeg_dependency_cancelled" => {
                let mut snapshot = self
                    .snapshot
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                snapshot.status = "cancelled".to_owned();
                snapshot.error = None;
            }
            Err(failure) => self.fail(failure),
        }
    }

    #[cfg(target_os = "windows")]
    fn download_and_install(&self) -> Result<u64, FfmpegDependencyFailure> {
        fs::create_dir_all(&self.root).map_err(|error| {
            io_failure(
                "dependency_directory_failed",
                "unable to create FFmpeg dependency directory",
                error,
            )
        })?;
        let archive_path = self
            .root
            .join(format!(".{}-download.zip", self.spec.version));
        remove_file_if_present(&archive_path)?;

        let user_agent = concat!("Resona/", env!("CARGO_PKG_VERSION"));
        let mut response = ureq::get(&self.spec.source_url)
            .header("User-Agent", user_agent)
            .call()
            .map_err(|error| {
                FfmpegDependencyFailure::new(
                    "dependency_download_failed",
                    format!("FFmpeg download failed: {error}"),
                )
            })?;
        let total = response.body().content_length();
        if total.is_some_and(|length| length > MAX_ARCHIVE_BYTES) {
            return Err(FfmpegDependencyFailure::new(
                "dependency_archive_too_large",
                "FFmpeg archive is larger than the configured safety limit",
            ));
        }
        self.update_download_progress(0, total);

        let mut output = File::create(&archive_path).map_err(|error| {
            io_failure(
                "dependency_archive_create_failed",
                "unable to create the temporary FFmpeg archive",
                error,
            )
        })?;
        let mut reader = response.body_mut().as_reader();
        let mut hasher = Sha256::new();
        let mut downloaded = 0_u64;
        let mut buffer = [0_u8; 256 * 1024];
        loop {
            self.check_cancelled()?;
            let count = reader.read(&mut buffer).map_err(|error| {
                io_failure(
                    "dependency_download_read_failed",
                    "unable to read the FFmpeg download",
                    error,
                )
            })?;
            if count == 0 {
                break;
            }
            downloaded = downloaded.saturating_add(count as u64);
            if downloaded > MAX_ARCHIVE_BYTES {
                return Err(FfmpegDependencyFailure::new(
                    "dependency_archive_too_large",
                    "FFmpeg archive exceeded the configured safety limit",
                ));
            }
            output.write_all(&buffer[..count]).map_err(|error| {
                io_failure(
                    "dependency_archive_write_failed",
                    "unable to write the temporary FFmpeg archive",
                    error,
                )
            })?;
            hasher.update(&buffer[..count]);
            self.update_download_progress(downloaded, total);
        }
        output.sync_all().map_err(|error| {
            io_failure(
                "dependency_archive_flush_failed",
                "unable to flush the temporary FFmpeg archive",
                error,
            )
        })?;
        let archive_hash = upper_hex(hasher.finalize());
        if archive_hash != self.spec.archive_sha256 {
            remove_file_if_present(&archive_path)?;
            return Err(FfmpegDependencyFailure::new(
                "dependency_archive_hash_mismatch",
                "FFmpeg archive SHA-256 verification failed",
            ));
        }

        {
            let mut snapshot = self
                .snapshot
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            snapshot.status = "installing".to_owned();
        }
        self.check_cancelled()?;
        let installed = self.install_from_archive(&archive_path);
        let cleanup = remove_file_if_present(&archive_path);
        match (installed, cleanup) {
            (Ok(bytes), Ok(())) => Ok(bytes),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
        }
    }

    fn install_from_archive(&self, archive_path: &Path) -> Result<u64, FfmpegDependencyFailure> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let staging = self
            .root
            .join(format!(".{}-install-{nonce}", self.spec.version));
        fs::create_dir_all(&staging).map_err(|error| {
            io_failure(
                "dependency_staging_create_failed",
                "unable to create the FFmpeg staging directory",
                error,
            )
        })?;

        let result = (|| {
            let archive = File::open(archive_path).map_err(|error| {
                io_failure(
                    "dependency_archive_open_failed",
                    "unable to open the verified FFmpeg archive",
                    error,
                )
            })?;
            let mut archive = zip::ZipArchive::new(archive).map_err(|error| {
                FfmpegDependencyFailure::new(
                    "dependency_archive_invalid",
                    format!("unable to read FFmpeg archive: {error}"),
                )
            })?;
            let mut ffmpeg_found = false;
            let mut ffprobe_found = false;
            for index in 0..archive.len() {
                self.check_cancelled()?;
                let mut entry = archive.by_index(index).map_err(|error| {
                    FfmpegDependencyFailure::new(
                        "dependency_archive_entry_invalid",
                        format!("unable to read FFmpeg archive entry: {error}"),
                    )
                })?;
                if entry.is_symlink() || entry.size() > MAX_BINARY_BYTES {
                    continue;
                }
                let Some(path) = entry.enclosed_name() else {
                    continue;
                };
                let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                    continue;
                };
                let expected_hash = match name.to_ascii_lowercase().as_str() {
                    "ffmpeg.exe" => {
                        if ffmpeg_found {
                            return Err(FfmpegDependencyFailure::new(
                                "dependency_archive_duplicate_binary",
                                "FFmpeg archive contains duplicate ffmpeg.exe entries",
                            ));
                        }
                        ffmpeg_found = true;
                        &self.spec.ffmpeg_sha256
                    }
                    "ffprobe.exe" => {
                        if ffprobe_found {
                            return Err(FfmpegDependencyFailure::new(
                                "dependency_archive_duplicate_binary",
                                "FFmpeg archive contains duplicate ffprobe.exe entries",
                            ));
                        }
                        ffprobe_found = true;
                        &self.spec.ffprobe_sha256
                    }
                    _ => continue,
                };
                let destination = staging.join(name.to_ascii_lowercase());
                copy_and_verify(&mut entry, &destination, expected_hash)?;
            }
            if !ffmpeg_found || !ffprobe_found {
                return Err(FfmpegDependencyFailure::new(
                    "dependency_archive_missing_binary",
                    "FFmpeg archive does not contain both ffmpeg.exe and ffprobe.exe",
                ));
            }

            let target = self.install_directory();
            if target.exists() {
                fs::remove_dir_all(&target).map_err(|error| {
                    io_failure(
                        "dependency_old_version_remove_failed",
                        "unable to replace the existing FFmpeg dependency",
                        error,
                    )
                })?;
            }
            fs::rename(&staging, &target).map_err(|error| {
                io_failure(
                    "dependency_commit_failed",
                    "unable to activate the verified FFmpeg dependency",
                    error,
                )
            })?;
            self.validate_installed()
        })();

        if staging.exists() {
            if let Err(error) = fs::remove_dir_all(&staging) {
                eprintln!("FFmpeg staging cleanup failed: {error}");
            }
        }
        result
    }

    fn validate_installed(&self) -> Result<u64, FfmpegDependencyFailure> {
        let (ffmpeg, ffprobe) = self.binary_paths();
        let ffmpeg_bytes = verify_file(&ffmpeg, &self.spec.ffmpeg_sha256)?;
        let ffprobe_bytes = verify_file(&ffprobe, &self.spec.ffprobe_sha256)?;
        Ok(ffmpeg_bytes.saturating_add(ffprobe_bytes))
    }

    fn install_directory(&self) -> PathBuf {
        self.root.join(&self.spec.version)
    }

    fn check_cancelled(&self) -> Result<(), FfmpegDependencyFailure> {
        if self.cancel.load(Ordering::Acquire) {
            Err(FfmpegDependencyFailure::new(
                "ffmpeg_dependency_cancelled",
                "FFmpeg dependency download was cancelled",
            ))
        } else {
            Ok(())
        }
    }

    fn update_download_progress(&self, downloaded: u64, total: Option<u64>) {
        let mut snapshot = self
            .snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        snapshot.downloaded_bytes = downloaded;
        snapshot.total_bytes = total;
    }

    fn fail(&self, failure: FfmpegDependencyFailure) {
        eprintln!("FFmpeg dependency failed: {}", failure.message);
        let mut snapshot = self
            .snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        snapshot.status = "failed".to_owned();
        snapshot.error = Some(failure);
    }
}

fn copy_and_verify(
    reader: &mut impl Read,
    destination: &Path,
    expected_hash: &str,
) -> Result<u64, FfmpegDependencyFailure> {
    let mut output = File::create(destination).map_err(|error| {
        io_failure(
            "dependency_binary_create_failed",
            "unable to create an FFmpeg binary",
            error,
        )
    })?;
    let mut hasher = Sha256::new();
    let mut copied = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer).map_err(|error| {
            io_failure(
                "dependency_binary_read_failed",
                "unable to extract an FFmpeg binary",
                error,
            )
        })?;
        if count == 0 {
            break;
        }
        copied = copied.saturating_add(count as u64);
        if copied > MAX_BINARY_BYTES {
            return Err(FfmpegDependencyFailure::new(
                "dependency_binary_too_large",
                "extracted FFmpeg binary exceeded the configured safety limit",
            ));
        }
        output.write_all(&buffer[..count]).map_err(|error| {
            io_failure(
                "dependency_binary_write_failed",
                "unable to write an FFmpeg binary",
                error,
            )
        })?;
        hasher.update(&buffer[..count]);
    }
    output.sync_all().map_err(|error| {
        io_failure(
            "dependency_binary_flush_failed",
            "unable to flush an FFmpeg binary",
            error,
        )
    })?;
    let actual_hash = upper_hex(hasher.finalize());
    if actual_hash != expected_hash {
        return Err(FfmpegDependencyFailure::new(
            "dependency_binary_hash_mismatch",
            format!(
                "{} SHA-256 verification failed",
                destination
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("FFmpeg binary")
            ),
        ));
    }
    Ok(copied)
}

fn verify_file(path: &Path, expected_hash: &str) -> Result<u64, FfmpegDependencyFailure> {
    let mut file = File::open(path).map_err(|error| {
        io_failure(
            "dependency_binary_missing",
            &format!("{} is unavailable", path.display()),
            error,
        )
    })?;
    let mut hasher = Sha256::new();
    let bytes = io::copy(&mut file, &mut HashWriter(&mut hasher)).map_err(|error| {
        io_failure(
            "dependency_binary_check_failed",
            &format!("unable to verify {}", path.display()),
            error,
        )
    })?;
    if upper_hex(hasher.finalize()) != expected_hash {
        return Err(FfmpegDependencyFailure::new(
            "dependency_binary_hash_mismatch",
            format!("{} SHA-256 verification failed", path.display()),
        ));
    }
    Ok(bytes)
}

struct HashWriter<'a>(&'a mut Sha256);

impl Write for HashWriter<'_> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.0.update(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn remove_file_if_present(path: &Path) -> Result<(), FfmpegDependencyFailure> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_failure(
            "dependency_temporary_cleanup_failed",
            "unable to remove a temporary FFmpeg archive",
            error,
        )),
    }
}

fn io_failure(code: &str, context: &str, error: io::Error) -> FfmpegDependencyFailure {
    FfmpegDependencyFailure::new(code, format!("{context}: {error}"))
}

fn upper_hex(bytes: impl AsRef<[u8]>) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let bytes = bytes.as_ref();
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    #[test]
    fn verified_archive_installs_only_the_two_required_binaries() {
        let root = temporary_directory("valid");
        let archive = root.join("fixture.zip");
        let ffmpeg = b"test-ffmpeg";
        let ffprobe = b"test-ffprobe";
        write_fixture(&archive, ffmpeg, ffprobe);
        let service = fixture_service(&root, ffmpeg, ffprobe);

        let installed = service
            .install_from_archive(&archive)
            .expect("install verified dependency");

        let (ffmpeg_path, ffprobe_path) = service.binary_paths();
        assert_eq!(fs::read(ffmpeg_path).expect("read ffmpeg"), ffmpeg);
        assert_eq!(fs::read(ffprobe_path).expect("read ffprobe"), ffprobe);
        assert_eq!(installed, (ffmpeg.len() + ffprobe.len()) as u64);
        assert!(!service.install_directory().join("unrelated.txt").exists());
        fs::remove_dir_all(root).expect("remove dependency fixture");
    }

    #[test]
    fn binary_hash_mismatch_never_activates_staging_directory() {
        let root = temporary_directory("invalid");
        let archive = root.join("fixture.zip");
        write_fixture(&archive, b"wrong-ffmpeg", b"test-ffprobe");
        let service = fixture_service(&root, b"expected-ffmpeg", b"test-ffprobe");

        let failure = service
            .install_from_archive(&archive)
            .expect_err("reject mismatched dependency");

        assert_eq!(failure.code, "dependency_binary_hash_mismatch");
        assert!(!service.install_directory().exists());
        fs::remove_dir_all(root).expect("remove dependency fixture");
    }

    fn fixture_service(
        directory: &Path,
        ffmpeg: &'static [u8],
        ffprobe: &'static [u8],
    ) -> FfmpegDependencyService {
        FfmpegDependencyService {
            spec: DependencySpec {
                version: "test".to_owned(),
                source_url: "https://example.invalid/ffmpeg.zip".to_owned(),
                archive_sha256: String::new(),
                ffmpeg_sha256: hash(ffmpeg),
                ffprobe_sha256: hash(ffprobe),
            },
            root: directory.join("dependencies"),
            snapshot: Arc::new(Mutex::new(FfmpegDependencySnapshot {
                status: "missing".to_owned(),
                version: "test".to_owned(),
                source_url: "https://example.invalid/ffmpeg.zip".to_owned(),
                license: "GPL-3.0-or-later".to_owned(),
                downloaded_bytes: 0,
                total_bytes: None,
                installed_bytes: 0,
                error: None,
            })),
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    fn write_fixture(path: &Path, ffmpeg: &[u8], ffprobe: &[u8]) {
        let file = File::create(path).expect("create fixture archive");
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        writer
            .start_file("ffmpeg-test/bin/ffmpeg.exe", options)
            .expect("start ffmpeg");
        writer.write_all(ffmpeg).expect("write ffmpeg");
        writer
            .start_file("ffmpeg-test/bin/ffprobe.exe", options)
            .expect("start ffprobe");
        writer.write_all(ffprobe).expect("write ffprobe");
        writer
            .start_file("ffmpeg-test/unrelated.txt", options)
            .expect("start unrelated");
        writer.write_all(b"ignored").expect("write unrelated");
        writer.finish().expect("finish fixture archive");
    }

    fn hash(bytes: &[u8]) -> String {
        upper_hex(Sha256::digest(bytes))
    }

    fn temporary_directory(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("resona-ffmpeg-{label}-{nonce}"));
        fs::create_dir_all(&path).expect("create dependency fixture directory");
        path
    }
}
