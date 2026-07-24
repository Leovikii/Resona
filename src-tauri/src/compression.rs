// SPDX-License-Identifier: GPL-3.0-only

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use lofty::file::{AudioFile, FileType, TaggedFileExt};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompressionPreset {
    Fast,
    Balanced,
    Smallest,
}

impl CompressionPreset {
    fn level(self) -> &'static str {
        match self {
            Self::Fast => "0",
            Self::Balanced => "5",
            Self::Smallest => "12",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressionItem {
    pub source: String,
    pub output: String,
    pub status: String,
    pub message: Option<String>,
    pub source_deleted: bool,
    pub progress: f32,
    pub source_bytes: u64,
    pub output_bytes: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressionSnapshot {
    pub task_id: u64,
    pub status: String,
    pub completed: usize,
    pub total: usize,
    pub items: Vec<CompressionItem>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressionScanNode {
    pub path: String,
    pub name: String,
    pub kind: String,
    pub ready: bool,
    pub issue_code: Option<String>,
    pub source_bytes: u64,
    pub children: Vec<CompressionScanNode>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressionScanWarning {
    pub path: String,
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressionScanSnapshot {
    pub scan_id: u64,
    pub status: String,
    pub input_roots: Vec<String>,
    pub scanned_entries: usize,
    pub candidate_files: usize,
    pub validated_files: usize,
    pub ready_files: usize,
    pub roots: Vec<CompressionScanNode>,
    pub warnings: Vec<CompressionScanWarning>,
}

impl CompressionScanSnapshot {
    fn idle() -> Self {
        Self {
            scan_id: 0,
            status: "idle".to_owned(),
            input_roots: Vec::new(),
            scanned_entries: 0,
            candidate_files: 0,
            validated_files: 0,
            ready_files: 0,
            roots: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct CompressionFailure {
    pub code: String,
    pub message: String,
}

impl CompressionFailure {
    pub(crate) fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
        }
    }
}

pub struct CompressionService {
    ffmpeg: PathBuf,
    next_task_id: AtomicU64,
    next_scan_id: AtomicU64,
    cancel: Arc<AtomicBool>,
    scan_cancel: Arc<AtomicBool>,
    snapshot: Arc<Mutex<CompressionSnapshot>>,
    scan_snapshot: Arc<Mutex<CompressionScanSnapshot>>,
    scan_workspace: Arc<Mutex<ScanWorkspace>>,
}

#[derive(Clone, Debug, Default)]
struct ScanWorkspace {
    roots: Vec<PathBuf>,
    exclusions: Vec<PathBuf>,
}

impl Default for CompressionService {
    fn default() -> Self {
        let ffmpeg = resolve_ffmpeg_binary();
        Self {
            ffmpeg,
            next_task_id: AtomicU64::new(1),
            next_scan_id: AtomicU64::new(1),
            cancel: Arc::new(AtomicBool::new(false)),
            scan_cancel: Arc::new(AtomicBool::new(false)),
            snapshot: Arc::new(Mutex::new(CompressionSnapshot {
                task_id: 0,
                status: "idle".to_owned(),
                completed: 0,
                total: 0,
                items: Vec::new(),
            })),
            scan_snapshot: Arc::new(Mutex::new(CompressionScanSnapshot::idle())),
            scan_workspace: Arc::new(Mutex::new(ScanWorkspace::default())),
        }
    }
}

impl CompressionService {
    pub fn with_binaries(ffmpeg: PathBuf, _ffprobe: PathBuf) -> Self {
        Self {
            ffmpeg,
            ..Self::default()
        }
    }

    pub fn shutdown(&self) {
        self.cancel();
        self.cancel_scan();
        for _ in 0..50 {
            let task_active = matches!(self.snapshot().status.as_str(), "running" | "cancelling");
            let scan_active = matches!(
                self.scan_snapshot().status.as_str(),
                "scanning" | "cancelling"
            );
            if !task_active && !scan_active {
                return;
            }
            thread::sleep(Duration::from_millis(40));
        }
        eprintln!("compression shutdown timed out; process exit will release remaining resources");
    }

    pub fn has_active_work(&self) -> bool {
        matches!(self.snapshot().status.as_str(), "running" | "cancelling")
            || matches!(
                self.scan_snapshot().status.as_str(),
                "scanning" | "cancelling"
            )
    }

    pub fn scan_snapshot(&self) -> CompressionScanSnapshot {
        self.scan_snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    pub fn add_scan_inputs(
        self: &Arc<Self>,
        paths: Vec<PathBuf>,
    ) -> Result<CompressionScanSnapshot, CompressionFailure> {
        if paths.is_empty() {
            return Err(CompressionFailure::new(
                "empty_compression_scan",
                "no files or folders were selected",
            ));
        }
        self.ensure_scan_idle()?;
        {
            let mut workspace = self
                .scan_workspace
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            for path in paths {
                let path = normalize_scan_path(path);
                workspace
                    .exclusions
                    .retain(|excluded| !paths_overlap(excluded, &path));
                workspace.roots.push(path);
            }
            normalize_scan_roots(&mut workspace.roots);
        }
        self.start_scan()
    }

    pub fn remove_scan_inputs(
        self: &Arc<Self>,
        paths: Vec<PathBuf>,
    ) -> Result<CompressionScanSnapshot, CompressionFailure> {
        self.ensure_scan_idle()?;
        {
            let mut workspace = self
                .scan_workspace
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            for path in paths.into_iter().map(normalize_scan_path) {
                let root_count = workspace.roots.len();
                workspace.roots.retain(|root| root != &path);
                if workspace.roots.len() == root_count
                    && !workspace.exclusions.iter().any(|value| value == &path)
                {
                    workspace.exclusions.push(path);
                }
            }
            if workspace.roots.is_empty() {
                workspace.exclusions.clear();
            }
        }
        self.start_scan()
    }

    pub fn clear_scan_inputs(&self) -> Result<CompressionScanSnapshot, CompressionFailure> {
        self.ensure_scan_idle()?;
        let mut workspace = self
            .scan_workspace
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *workspace = ScanWorkspace::default();
        let mut snapshot = self
            .scan_snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *snapshot = CompressionScanSnapshot::idle();
        Ok(snapshot.clone())
    }

    pub fn cancel_scan(&self) -> CompressionScanSnapshot {
        self.scan_cancel.store(true, Ordering::Release);
        let mut snapshot = self
            .scan_snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if snapshot.status == "scanning" {
            snapshot.status = "cancelling".to_owned();
        }
        snapshot.clone()
    }

    fn ensure_scan_idle(&self) -> Result<(), CompressionFailure> {
        let status = self
            .scan_snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .status
            .clone();
        if status == "scanning" || status == "cancelling" {
            Err(CompressionFailure::new(
                "compression_scan_busy",
                "compression inputs are still being scanned",
            ))
        } else {
            Ok(())
        }
    }

    fn start_scan(self: &Arc<Self>) -> Result<CompressionScanSnapshot, CompressionFailure> {
        let workspace = self
            .scan_workspace
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        if workspace.roots.is_empty() {
            let mut snapshot = self
                .scan_snapshot
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *snapshot = CompressionScanSnapshot::idle();
            return Ok(snapshot.clone());
        }
        let scan_id = self.next_scan_id.fetch_add(1, Ordering::Relaxed);
        self.scan_cancel.store(false, Ordering::Release);
        let initial = CompressionScanSnapshot {
            scan_id,
            status: "scanning".to_owned(),
            input_roots: workspace
                .roots
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect(),
            ..CompressionScanSnapshot::idle()
        };
        *self
            .scan_snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = initial.clone();
        let service = Arc::clone(self);
        if let Err(error) = thread::Builder::new()
            .name("resona-compression-scan".to_owned())
            .spawn(move || service.run_scan(scan_id, workspace))
        {
            let mut snapshot = self
                .scan_snapshot
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            snapshot.status = "failed".to_owned();
            snapshot.warnings.push(CompressionScanWarning {
                path: String::new(),
                code: "scan_task_failed".to_owned(),
                message: error.to_string(),
            });
            return Err(CompressionFailure::new(
                "scan_task_failed",
                error.to_string(),
            ));
        }
        Ok(initial)
    }

    fn run_scan(&self, scan_id: u64, workspace: ScanWorkspace) {
        let result = scan_workspace(&workspace, &self.scan_cancel, &self.scan_snapshot);
        let mut snapshot = self
            .scan_snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if snapshot.scan_id != scan_id {
            return;
        }
        match result {
            Ok(result) => {
                snapshot.status = "ready".to_owned();
                snapshot.scanned_entries = result.scanned_entries;
                snapshot.candidate_files = result.candidate_files;
                snapshot.validated_files = result.validated_files;
                snapshot.ready_files = result.ready_files;
                snapshot.roots = result.roots;
                snapshot.warnings = result.warnings;
            }
            Err(ScanInterrupted) => snapshot.status = "cancelled".to_owned(),
        }
    }

    pub fn snapshot(&self) -> CompressionSnapshot {
        self.snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    pub fn cancel(&self) -> CompressionSnapshot {
        self.cancel.store(true, Ordering::Release);
        let mut state = self
            .snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.status == "running" {
            state.status = "cancelling".to_owned();
        }
        state.clone()
    }

    pub fn start(
        self: &Arc<Self>,
        paths: Vec<PathBuf>,
        preset: CompressionPreset,
        delete_source: bool,
        deletion_confirmed: bool,
    ) -> Result<CompressionSnapshot, CompressionFailure> {
        if delete_source && !deletion_confirmed {
            return Err(CompressionFailure::new(
                "source_deletion_confirmation_required",
                "source deletion must be explicitly confirmed for this batch",
            ));
        }
        if paths.is_empty() {
            return Err(CompressionFailure::new(
                "empty_compression_batch",
                "no WAV files selected",
            ));
        }
        if !self.ffmpeg.is_file() {
            return Err(CompressionFailure::new(
                "compression_binary_missing",
                "bundled FFmpeg is unavailable",
            ));
        }
        let mut state = self
            .snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.status == "running" || state.status == "cancelling" {
            return Err(CompressionFailure::new(
                "compression_busy",
                "a compression batch is already running",
            ));
        }
        let task_id = self.next_task_id.fetch_add(1, Ordering::Relaxed);
        let items = paths
            .iter()
            .map(|source| CompressionItem {
                source: source.to_string_lossy().into_owned(),
                output: source.with_extension("flac").to_string_lossy().into_owned(),
                status: "pending".to_owned(),
                message: None,
                source_deleted: false,
                progress: 0.0,
                source_bytes: fs::metadata(source).map_or(0, |value| value.len()),
                output_bytes: 0,
            })
            .collect::<Vec<_>>();
        *state = CompressionSnapshot {
            task_id,
            status: "running".to_owned(),
            completed: 0,
            total: items.len(),
            items,
        };
        let initial = state.clone();
        drop(state);
        self.cancel.store(false, Ordering::Release);
        let service = Arc::clone(self);
        if let Err(error) = thread::Builder::new()
            .name("resona-compression".to_owned())
            .spawn(move || service.run_batch(task_id, paths, preset, delete_source))
        {
            let mut state = self
                .snapshot
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.status = "completed_with_errors".to_owned();
            return Err(CompressionFailure::new(
                "compression_task_failed",
                error.to_string(),
            ));
        }
        Ok(initial)
    }

    fn run_batch(
        &self,
        task_id: u64,
        paths: Vec<PathBuf>,
        preset: CompressionPreset,
        delete_source: bool,
    ) {
        self.run_batch_with(
            task_id,
            paths,
            preset,
            delete_source,
            |task_id, index, source, preset| self.convert_one(task_id, index, source, preset),
        );
    }

    fn run_batch_with<F>(
        &self,
        task_id: u64,
        paths: Vec<PathBuf>,
        preset: CompressionPreset,
        delete_source: bool,
        convert: F,
    ) where
        F: Fn(u64, usize, &Path, CompressionPreset) -> Result<u64, String> + Sync,
    {
        let next = AtomicUsize::new(0);
        let workers = compression_worker_count(paths.len());
        thread::scope(|scope| {
            for _ in 0..workers {
                scope.spawn(|| loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    let Some(source) = paths.get(index) else {
                        return;
                    };
                    if self.cancel.load(Ordering::Acquire) {
                        self.update_item(index, "cancelled", None, false, None);
                        continue;
                    }
                    self.update_item(index, "running", None, false, None);
                    match convert(task_id, index, source, preset) {
                        Ok(output_bytes) => {
                            let mut message = None;
                            let mut deleted = false;
                            if delete_source {
                                match fs::remove_file(source) {
                                    Ok(()) => deleted = true,
                                    Err(error) => {
                                        message = Some(format!(
                                            "FLAC created, but source deletion failed: {error}"
                                        ))
                                    }
                                }
                            }
                            self.update_item(
                                index,
                                "completed",
                                message,
                                deleted,
                                Some(output_bytes),
                            );
                        }
                        Err(_) if self.cancel.load(Ordering::Acquire) => {
                            self.update_item(index, "cancelled", None, false, None);
                        }
                        Err(message) => {
                            self.update_item(index, "failed", Some(message), false, None)
                        }
                    }
                });
            }
        });
        let mut state = self
            .snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.cancel.load(Ordering::Acquire) {
            state.status = "cancelled".to_owned();
            for item in &mut state.items {
                if item.status == "pending" {
                    item.status = "cancelled".to_owned();
                }
            }
            state.completed = state
                .items
                .iter()
                .filter(|item| item.status != "pending")
                .count();
            return;
        }
        state.status = if state
            .items
            .iter()
            .any(|item| item.status == "failed" || item.message.is_some())
        {
            "completed_with_errors"
        } else {
            "completed"
        }
        .to_owned();
    }

    fn convert_one(
        &self,
        task_id: u64,
        index: usize,
        source: &Path,
        preset: CompressionPreset,
    ) -> Result<u64, String> {
        if source
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
            != Some("wav")
        {
            return Err("only PCM WAV input is supported".to_owned());
        }
        let output = source.with_extension("flac");
        if output.exists() {
            return Err("output FLAC already exists".to_owned());
        }
        let input = inspect_wav(source)?;
        let temp = source.with_file_name(format!(
            ".{}.resona-{task_id}-{index}.tmp.flac",
            source
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("audio")
        ));
        let _ = fs::remove_file(&temp);
        let target_bits = if input.bits_per_sample <= 16 { 16 } else { 24 };
        let mut command = background_command(&self.ffmpeg);
        command
            .args(["-hide_banner", "-nostdin", "-v", "error", "-y", "-i"])
            .arg(source)
            .args([
                "-map",
                "0:a:0",
                "-map_metadata",
                "0",
                "-c:a",
                "flac",
                "-compression_level",
                preset.level(),
            ]);
        if target_bits == 16 {
            command.args(["-sample_fmt", "s16"]);
        } else {
            command.args(["-sample_fmt", "s32", "-bits_per_raw_sample", "24"]);
        }
        command
            .args(["-progress", "pipe:1", "-nostats"])
            .arg(&temp)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command
            .spawn()
            .map_err(|error| format!("failed to start FFmpeg: {error}"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "FFmpeg progress stream unavailable".to_owned())?;
        let progress_state = Arc::clone(&self.snapshot);
        let duration_us = input.duration_seconds * 1_000_000.0;
        let progress_reader = thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Some(value) = line
                    .strip_prefix("out_time_us=")
                    .and_then(|value| value.parse::<f32>().ok())
                {
                    if duration_us > 0.0 {
                        let mut state = progress_state
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        if let Some(item) = state.items.get_mut(index) {
                            item.progress =
                                item.progress.max((value / duration_us).clamp(0.0, 1.0));
                        }
                    }
                }
            }
        });
        loop {
            if self.cancel.load(Ordering::Acquire) {
                let _ = child.kill();
                let _ = child.wait();
                let _ = progress_reader.join();
                let _ = fs::remove_file(&temp);
                return Err("cancelled".to_owned());
            }
            match child.try_wait() {
                Ok(Some(status)) => {
                    let stderr = child
                        .stderr
                        .take()
                        .map(|stream| {
                            BufReader::new(stream)
                                .lines()
                                .map_while(Result::ok)
                                .collect::<Vec<_>>()
                                .join("\n")
                        })
                        .unwrap_or_default();
                    let _ = progress_reader.join();
                    if !status.success() {
                        let _ = fs::remove_file(&temp);
                        return Err(if stderr.is_empty() {
                            "FFmpeg conversion failed".to_owned()
                        } else {
                            stderr
                        });
                    }
                    break;
                }
                Ok(None) => thread::sleep(Duration::from_millis(80)),
                Err(error) => {
                    let _ = child.kill();
                    let _ = progress_reader.join();
                    let _ = fs::remove_file(&temp);
                    return Err(format!("FFmpeg process failed: {error}"));
                }
            }
        }
        let verified = inspect_flac(&temp).inspect_err(|_| {
            let _ = fs::remove_file(&temp);
        })?;
        if verified.sample_rate != input.sample_rate
            || verified.channels != input.channels
            || verified.bits_per_sample != target_bits
        {
            let _ = fs::remove_file(&temp);
            return Err(
                "FLAC validation did not preserve the required audio properties".to_owned(),
            );
        }
        fs::rename(&temp, &output).map_err(|error| {
            let _ = fs::remove_file(&temp);
            format!("failed to commit FLAC output: {error}")
        })?;
        let output_bytes = fs::metadata(&output)
            .map_err(|error| format!("failed to read FLAC output size: {error}"))?
            .len();
        self.update_progress(index, 1.0);
        Ok(output_bytes)
    }

    fn update_item(
        &self,
        index: usize,
        status: &str,
        message: Option<String>,
        source_deleted: bool,
        output_bytes: Option<u64>,
    ) {
        let mut state = self
            .snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(item) = state.items.get_mut(index) {
            item.status = status.to_owned();
            item.message = message;
            item.source_deleted = source_deleted;
            if let Some(output_bytes) = output_bytes {
                item.output_bytes = output_bytes;
                item.progress = 1.0;
            }
        }
        state.completed = state
            .items
            .iter()
            .filter(|item| matches!(item.status.as_str(), "completed" | "failed" | "cancelled"))
            .count();
    }

    fn update_progress(&self, index: usize, progress: f32) {
        let mut state = self
            .snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(item) = state.items.get_mut(index) {
            item.progress = item.progress.max(progress.clamp(0.0, 1.0));
        }
    }
}

fn compression_worker_count(item_count: usize) -> usize {
    let available = thread::available_parallelism().map_or(1, usize::from);
    available
        .saturating_sub(1)
        .clamp(1, 4)
        .min(item_count.max(1))
}

#[derive(Debug)]
struct AudioProperties {
    sample_rate: u32,
    channels: u8,
    bits_per_sample: u8,
    duration_seconds: f32,
}

fn inspect_wav(path: &Path) -> Result<AudioProperties, String> {
    let reader =
        hound::WavReader::open(path).map_err(|error| format!("invalid WAV header: {error}"))?;
    let spec = reader.spec();
    validate_wav_spec(spec)?;
    Ok(AudioProperties {
        sample_rate: spec.sample_rate,
        channels: spec.channels as u8,
        bits_per_sample: spec.bits_per_sample as u8,
        duration_seconds: reader.duration() as f32 / spec.sample_rate as f32,
    })
}

fn inspect_flac(path: &Path) -> Result<AudioProperties, String> {
    let tagged =
        lofty::read_from_path(path).map_err(|error| format!("invalid FLAC output: {error}"))?;
    if tagged.file_type() != FileType::Flac {
        return Err("output is not a FLAC file".to_owned());
    }
    let properties = tagged.properties();
    Ok(AudioProperties {
        sample_rate: properties
            .sample_rate()
            .ok_or("FLAC sample rate is unavailable")?,
        channels: properties
            .channels()
            .ok_or("FLAC channel count is unavailable")?,
        bits_per_sample: properties
            .bit_depth()
            .ok_or("FLAC bit depth is unavailable")?,
        duration_seconds: properties.duration().as_secs_f32(),
    })
}

#[derive(Debug)]
struct ScanInterrupted;

struct ScanResult {
    scanned_entries: usize,
    candidate_files: usize,
    validated_files: usize,
    ready_files: usize,
    roots: Vec<CompressionScanNode>,
    warnings: Vec<CompressionScanWarning>,
}

#[derive(Clone)]
struct ScanCandidate {
    path: PathBuf,
    root: PathBuf,
}

#[derive(Clone)]
struct ValidatedCandidate {
    candidate: ScanCandidate,
    ready: bool,
    issue_code: Option<String>,
}

fn scan_workspace(
    workspace: &ScanWorkspace,
    cancel: &AtomicBool,
    progress: &Arc<Mutex<CompressionScanSnapshot>>,
) -> Result<ScanResult, ScanInterrupted> {
    let mut warnings = Vec::new();
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    let mut scanned_entries = 0usize;
    for root in &workspace.roots {
        discover_scan_candidates(
            root,
            root,
            &workspace.exclusions,
            cancel,
            &mut candidates,
            &mut seen,
            &mut warnings,
            &mut scanned_entries,
            progress,
        )?;
    }
    candidates.sort_by_key(|candidate| path_sort_key(&candidate.path));
    {
        let mut snapshot = progress
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        snapshot.scanned_entries = scanned_entries;
        snapshot.candidate_files = candidates.len();
    }

    let candidate_files = candidates.len();
    let results = validate_scan_candidates(&candidates, cancel, progress)?;
    let mut validated = Vec::new();
    for (candidate, result) in candidates.into_iter().zip(results) {
        match result {
            Ok(()) => {
                let output_exists = candidate.path.with_extension("flac").exists();
                validated.push(ValidatedCandidate {
                    candidate,
                    ready: !output_exists,
                    issue_code: output_exists.then(|| "output_exists".to_owned()),
                });
            }
            Err(message) => warnings.push(CompressionScanWarning {
                path: candidate.path.to_string_lossy().into_owned(),
                code: "invalid_wav".to_owned(),
                message,
            }),
        }
    }
    let ready_files = validated.iter().filter(|file| file.ready).count();
    let roots = build_scan_tree(&workspace.roots, validated);
    Ok(ScanResult {
        scanned_entries,
        candidate_files,
        validated_files: candidate_files,
        ready_files,
        roots,
        warnings,
    })
}

#[allow(clippy::too_many_arguments)]
fn discover_scan_candidates(
    root: &Path,
    path: &Path,
    exclusions: &[PathBuf],
    cancel: &AtomicBool,
    candidates: &mut Vec<ScanCandidate>,
    seen: &mut HashSet<PathBuf>,
    warnings: &mut Vec<CompressionScanWarning>,
    scanned_entries: &mut usize,
    progress: &Arc<Mutex<CompressionScanSnapshot>>,
) -> Result<(), ScanInterrupted> {
    if cancel.load(Ordering::Acquire) {
        return Err(ScanInterrupted);
    }
    if exclusions
        .iter()
        .any(|excluded| path == excluded || path.starts_with(excluded))
    {
        return Ok(());
    }
    *scanned_entries += 1;
    if (*scanned_entries).is_multiple_of(32) {
        let mut snapshot = progress
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        snapshot.scanned_entries = *scanned_entries;
        snapshot.candidate_files = candidates.len();
    }
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            warnings.push(scan_warning(path, "path_unavailable", error));
            return Ok(());
        }
    };
    if is_link_like(&metadata) {
        warnings.push(CompressionScanWarning {
            path: path.to_string_lossy().into_owned(),
            code: "linked_path_skipped".to_owned(),
            message: "linked or reparse-point paths are not followed".to_owned(),
        });
        return Ok(());
    }
    if metadata.is_file() {
        if has_wav_extension(path) {
            let normalized = normalize_scan_path(path.to_path_buf());
            if seen.insert(normalized.clone()) {
                candidates.push(ScanCandidate {
                    path: normalized,
                    root: root.to_path_buf(),
                });
            }
        } else if path == root {
            warnings.push(CompressionScanWarning {
                path: path.to_string_lossy().into_owned(),
                code: "unsupported_input".to_owned(),
                message: "only PCM WAV input is supported".to_owned(),
            });
        }
        return Ok(());
    }
    if !metadata.is_dir() {
        return Ok(());
    }
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) => {
            warnings.push(scan_warning(path, "directory_unreadable", error));
            return Ok(());
        }
    };
    let mut collected = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) => collected.push(entry),
            Err(error) => warnings.push(scan_warning(path, "directory_entry_unreadable", error)),
        }
    }
    let mut entries = collected;
    entries.sort_by_key(|entry| path_sort_key(&entry.path()));
    for entry in entries {
        discover_scan_candidates(
            root,
            &entry.path(),
            exclusions,
            cancel,
            candidates,
            seen,
            warnings,
            scanned_entries,
            progress,
        )?;
    }
    Ok(())
}

fn validate_scan_candidates(
    candidates: &[ScanCandidate],
    cancel: &AtomicBool,
    progress: &Arc<Mutex<CompressionScanSnapshot>>,
) -> Result<Vec<Result<(), String>>, ScanInterrupted> {
    let mut results = Vec::with_capacity(candidates.len());
    for (index, candidate) in candidates.iter().enumerate() {
        if cancel.load(Ordering::Acquire) {
            return Err(ScanInterrupted);
        }
        results.push(validate_wav_header(&candidate.path));
        progress
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .validated_files = index + 1;
    }
    Ok(results)
}

fn validate_wav_header(path: &Path) -> Result<(), String> {
    let reader =
        hound::WavReader::open(path).map_err(|error| format!("invalid WAV header: {error}"))?;
    validate_wav_spec(reader.spec())
}

fn validate_wav_spec(spec: hound::WavSpec) -> Result<(), String> {
    if spec.channels == 0 || spec.sample_rate == 0 {
        return Err("WAV channel count or sample rate is invalid".to_owned());
    }
    let supported_bits = match spec.sample_format {
        hound::SampleFormat::Int => matches!(spec.bits_per_sample, 8 | 16 | 24 | 32),
        hound::SampleFormat::Float => spec.bits_per_sample == 32,
    };
    if !supported_bits {
        return Err(format!(
            "unsupported WAV sample format: {:?} {}-bit",
            spec.sample_format, spec.bits_per_sample
        ));
    }
    Ok(())
}

fn build_scan_tree(
    roots: &[PathBuf],
    validated: Vec<ValidatedCandidate>,
) -> Vec<CompressionScanNode> {
    let mut grouped = BTreeMap::<String, Vec<ValidatedCandidate>>::new();
    for candidate in validated {
        grouped
            .entry(path_sort_key(&candidate.candidate.root))
            .or_default()
            .push(candidate);
    }
    roots
        .iter()
        .filter_map(|root| {
            let root_name = display_file_name(root);
            if root.is_file() {
                let candidate = grouped
                    .remove(&path_sort_key(root))
                    .and_then(|mut files| files.pop());
                return candidate.map(|candidate| {
                    file_scan_node(root, root_name, candidate.ready, candidate.issue_code)
                });
            }
            let mut builder = ScanTreeBuilder::directory(root.to_path_buf(), root_name);
            for candidate in grouped.remove(&path_sort_key(root)).unwrap_or_default() {
                if let Ok(relative) = candidate.candidate.path.strip_prefix(root) {
                    builder.insert_file(
                        relative,
                        &candidate.candidate.path,
                        candidate.ready,
                        candidate.issue_code,
                    );
                }
            }
            Some(builder.finish("root"))
        })
        .collect()
}

struct ScanTreeBuilder {
    path: PathBuf,
    name: String,
    children: BTreeMap<String, ScanTreeBuilder>,
    file: Option<(bool, Option<String>)>,
}

impl ScanTreeBuilder {
    fn directory(path: PathBuf, name: String) -> Self {
        Self {
            path,
            name,
            children: BTreeMap::new(),
            file: None,
        }
    }

    fn insert_file(
        &mut self,
        relative: &Path,
        full_path: &Path,
        ready: bool,
        issue_code: Option<String>,
    ) {
        let parts = relative.components().collect::<Vec<_>>();
        self.insert_parts(&parts, full_path, ready, issue_code);
    }

    fn insert_parts(
        &mut self,
        parts: &[std::path::Component<'_>],
        full_path: &Path,
        ready: bool,
        issue_code: Option<String>,
    ) {
        let Some((head, tail)) = parts.split_first() else {
            return;
        };
        let name = head.as_os_str().to_string_lossy().into_owned();
        let key = name_sort_key(&name);
        if tail.is_empty() {
            self.children.insert(
                key,
                Self {
                    path: full_path.to_path_buf(),
                    name,
                    children: BTreeMap::new(),
                    file: Some((ready, issue_code)),
                },
            );
            return;
        }
        let directory_path = self.path.join(head.as_os_str());
        self.children
            .entry(key)
            .or_insert_with(|| Self::directory(directory_path, name))
            .insert_parts(tail, full_path, ready, issue_code);
    }

    fn finish(self, directory_kind: &str) -> CompressionScanNode {
        let is_file = self.file.is_some();
        let (ready, issue_code) = self.file.unwrap_or((false, None));
        let source_bytes = if is_file {
            fs::metadata(&self.path).map_or(0, |value| value.len())
        } else {
            0
        };
        CompressionScanNode {
            path: self.path.to_string_lossy().into_owned(),
            name: self.name,
            kind: if is_file { "file" } else { directory_kind }.to_owned(),
            ready,
            issue_code,
            source_bytes,
            children: self
                .children
                .into_values()
                .map(|child| child.finish("directory"))
                .collect(),
        }
    }
}

fn file_scan_node(
    path: &Path,
    name: String,
    ready: bool,
    issue_code: Option<String>,
) -> CompressionScanNode {
    CompressionScanNode {
        path: path.to_string_lossy().into_owned(),
        name,
        kind: "file".to_owned(),
        ready,
        issue_code,
        source_bytes: fs::metadata(path).map_or(0, |value| value.len()),
        children: Vec::new(),
    }
}

fn normalize_scan_path(path: PathBuf) -> PathBuf {
    if fs::symlink_metadata(&path).is_ok_and(|metadata| is_link_like(&metadata)) {
        path
    } else {
        fs::canonicalize(&path).unwrap_or(path)
    }
}

fn normalize_scan_roots(roots: &mut Vec<PathBuf>) {
    roots.sort_by_key(|path| path_sort_key(path));
    roots.dedup_by(|left, right| paths_equal(left, right));
    let snapshot = roots.clone();
    roots.retain(|candidate| {
        !snapshot.iter().any(|other| {
            !paths_equal(candidate, other) && other.is_dir() && candidate.starts_with(other)
        })
    });
}

fn paths_overlap(left: &Path, right: &Path) -> bool {
    left.starts_with(right) || right.starts_with(left)
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    path_sort_key(left) == path_sort_key(right)
}

fn path_sort_key(path: &Path) -> String {
    let value = path.to_string_lossy().into_owned();
    #[cfg(target_os = "windows")]
    {
        value.to_lowercase()
    }
    #[cfg(not(target_os = "windows"))]
    {
        value
    }
}

fn name_sort_key(name: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        name.to_lowercase()
    }
    #[cfg(not(target_os = "windows"))]
    {
        name.to_owned()
    }
}

fn display_file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

fn has_wav_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("wav"))
}

fn scan_warning(path: &Path, code: &str, error: impl std::fmt::Display) -> CompressionScanWarning {
    CompressionScanWarning {
        path: path.to_string_lossy().into_owned(),
        code: code.to_owned(),
        message: error.to_string(),
    }
}

fn is_link_like(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

fn background_command(program: &Path) -> Command {
    let mut command = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

fn resolve_ffmpeg_binary() -> PathBuf {
    if let Ok(ffmpeg) = std::env::var("RESONA_FFMPEG_PATH") {
        return ffmpeg.into();
    }
    let release_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf));
    if let Some(directory) = release_dir {
        let ffmpeg = directory.join("ffmpeg.exe");
        if ffmpeg.is_file() {
            return ffmpeg;
        }
    }
    let binaries = Path::new(env!("CARGO_MANIFEST_DIR")).join("binaries");
    binaries.join("ffmpeg-x86_64-pc-windows-msvc.exe")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    #[test]
    fn presets_map_to_lossless_compression_levels() {
        assert_eq!(CompressionPreset::Fast.level(), "0");
        assert_eq!(CompressionPreset::Balanced.level(), "5");
        assert_eq!(CompressionPreset::Smallest.level(), "12");
    }
    #[test]
    fn requires_explicit_confirmation_before_source_deletion() {
        let service = Arc::new(CompressionService::default());
        let result = service.start(
            vec![PathBuf::from("example.wav")],
            CompressionPreset::Balanced,
            true,
            false,
        );
        assert!(
            matches!(result, Err(CompressionFailure { code, .. }) if code == "source_deletion_confirmation_required")
        );
    }

    #[test]
    fn active_work_is_reported_for_exit_coordination() {
        let service = CompressionService::default();
        assert!(!service.has_active_work());
        service
            .snapshot
            .lock()
            .expect("compression snapshot")
            .status = "running".to_owned();
        assert!(service.has_active_work());
        service
            .snapshot
            .lock()
            .expect("compression snapshot")
            .status = "completed".to_owned();
        service.scan_snapshot.lock().expect("scan snapshot").status = "scanning".to_owned();
        assert!(service.has_active_work());
    }

    #[test]
    #[ignore = "requires pinned FFmpeg test tools prepared by npm run prepare:test-tools"]
    fn pinned_ffmpeg_test_tools_preserve_matrix_and_quantize_32_bit_to_24() {
        let service = CompressionService::default();
        assert!(service.ffmpeg.is_file(), "bundled ffmpeg is required");
        let directory = test_directory("matrix");
        fs::create_dir_all(&directory).expect("create compression test directory");
        let fixtures = [
            ("wav_44100_16_stereo.wav", 44_100, 16),
            ("wav_48000_24_stereo.wav", 48_000, 24),
            ("wav_96000_32_stereo.wav", 96_000, 24),
            ("wav_192000_f32_stereo.wav", 192_000, 24),
        ];
        for (index, (name, expected_rate, expected_bits)) in fixtures.iter().enumerate() {
            let source = directory.join(name);
            fs::copy(fixture_directory().join(name), &source).expect("copy WAV fixture");
            let preset = match index % 3 {
                0 => CompressionPreset::Fast,
                1 => CompressionPreset::Balanced,
                _ => CompressionPreset::Smallest,
            };
            service
                .convert_one(100 + index as u64, index, &source, preset)
                .expect("convert fixture");
            let output = source.with_extension("flac");
            let probe = inspect_flac(&output).expect("inspect converted FLAC");
            assert_eq!(probe.sample_rate, *expected_rate);
            assert_eq!(probe.channels, 2);
            assert_eq!(probe.bits_per_sample, *expected_bits);
        }
        fs::remove_dir_all(directory).expect("remove compression test directory");
    }

    #[test]
    fn successful_confirmed_batch_deletes_source_after_commit() {
        let service = CompressionService::default();
        let directory = test_directory("delete");
        fs::create_dir_all(&directory).expect("create compression test directory");
        let source = directory.join("source.wav");
        fs::write(&source, b"source audio").expect("create source fixture");
        prime_batch_snapshot(&service, 7, &source);
        service.run_batch_with(
            7,
            vec![source.clone()],
            CompressionPreset::Balanced,
            true,
            |_, _, input, _| {
                let output = input.with_extension("flac");
                fs::write(&output, b"committed FLAC").map_err(|error| error.to_string())?;
                Ok(fs::metadata(output)
                    .map_err(|error| error.to_string())?
                    .len())
            },
        );
        let snapshot = service.snapshot();
        assert_eq!(snapshot.status, "completed");
        assert!(!source.exists());
        assert!(source.with_extension("flac").is_file());
        assert!(snapshot.items[0].source_deleted);
        fs::remove_dir_all(directory).expect("remove compression test directory");
    }

    #[test]
    fn batch_tracks_partial_results_and_output_sizes() {
        let service = CompressionService::default();
        let sources = [PathBuf::from("first.wav"), PathBuf::from("second.wav")];
        prime_batch_snapshots(&service, 8, &sources);
        service.run_batch_with(
            8,
            sources.to_vec(),
            CompressionPreset::Fast,
            false,
            |_, index, _, _| {
                if index == 0 {
                    Ok(240)
                } else {
                    Err("conversion failed".to_owned())
                }
            },
        );
        let snapshot = service.snapshot();
        assert_eq!(snapshot.status, "completed_with_errors");
        assert_eq!(snapshot.completed, 2);
        assert_eq!(snapshot.items[0].output_bytes, 240);
        assert_eq!(snapshot.items[0].progress, 1.0);
        assert_eq!(snapshot.items[1].status, "failed");
        assert_eq!(snapshot.items[1].output_bytes, 0);
    }

    #[test]
    fn item_progress_never_moves_backwards() {
        let service = CompressionService::default();
        prime_batch_snapshot(&service, 9, Path::new("progress.wav"));
        service.update_progress(0, 0.75);
        service.update_progress(0, 0.25);
        assert_eq!(service.snapshot().items[0].progress, 0.75);
    }

    #[test]
    fn batch_uses_bounded_parallel_workers_when_available() {
        let workers = compression_worker_count(8);
        assert!((1..=4).contains(&workers));
        if workers == 1 {
            return;
        }
        let service = CompressionService::default();
        let sources = (0..8)
            .map(|index| PathBuf::from(format!("parallel-{index}.wav")))
            .collect::<Vec<_>>();
        prime_batch_snapshots(&service, 10, &sources);
        let active = AtomicUsize::new(0);
        let maximum = AtomicUsize::new(0);
        service.run_batch_with(10, sources, CompressionPreset::Fast, false, |_, _, _, _| {
            let now = active.fetch_add(1, Ordering::SeqCst) + 1;
            maximum.fetch_max(now, Ordering::SeqCst);
            thread::sleep(Duration::from_millis(30));
            active.fetch_sub(1, Ordering::SeqCst);
            Ok(1)
        });
        assert!(maximum.load(Ordering::SeqCst) > 1);
        assert!(maximum.load(Ordering::SeqCst) <= workers);
        assert_eq!(service.snapshot().completed, 8);
    }

    #[test]
    fn output_conflict_retains_source_and_existing_output() {
        let directory = test_directory("conflict");
        fs::create_dir_all(&directory).expect("create compression test directory");
        let source = directory.join("source.wav");
        let output = source.with_extension("flac");
        let fake_ffmpeg = directory.join("ffmpeg.exe");
        let fake_ffprobe = directory.join("ffprobe.exe");
        fs::copy(fixture_directory().join("wav_44100_16_stereo.wav"), &source)
            .expect("copy WAV fixture");
        fs::write(&output, b"existing output").expect("create existing output");
        fs::write(&fake_ffmpeg, b"not executed").expect("create fake ffmpeg");
        fs::write(&fake_ffprobe, b"not executed").expect("create fake ffprobe");
        let service = Arc::new(CompressionService::with_binaries(fake_ffmpeg, fake_ffprobe));
        service
            .start(vec![source.clone()], CompressionPreset::Fast, true, true)
            .expect("start conversion");
        let snapshot = wait_for_completion(&service);
        assert_eq!(snapshot.status, "completed_with_errors");
        assert!(source.is_file());
        assert_eq!(fs::read(&output).expect("read output"), b"existing output");
        assert!(!snapshot.items[0].source_deleted);
        fs::remove_dir_all(directory).expect("remove compression test directory");
    }

    #[test]
    fn cancellation_before_work_retains_all_sources() {
        let service = CompressionService::default();
        let source = PathBuf::from("source.wav");
        *service
            .snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = CompressionSnapshot {
            task_id: 7,
            status: "running".to_owned(),
            completed: 0,
            total: 1,
            items: vec![CompressionItem {
                source: source.to_string_lossy().into_owned(),
                output: source.with_extension("flac").to_string_lossy().into_owned(),
                status: "pending".to_owned(),
                message: None,
                source_deleted: false,
                progress: 0.0,
                source_bytes: 0,
                output_bytes: 0,
            }],
        };
        service.cancel.store(true, Ordering::Release);
        service.run_batch(7, vec![source], CompressionPreset::Balanced, true);
        let snapshot = service.snapshot();
        assert_eq!(snapshot.status, "cancelled");
        assert_eq!(snapshot.items[0].status, "cancelled");
        assert!(!snapshot.items[0].source_deleted);
    }

    #[test]
    fn recursive_scan_builds_a_deduplicated_tree_and_marks_conflicts() {
        let service = Arc::new(CompressionService::default());
        let directory = test_directory("scan-tree");
        let nested = directory.join("disc-one").join("live");
        fs::create_dir_all(&nested).expect("create nested scan directory");
        let first = directory.join("disc-one").join("Track 01.WAV");
        let second = nested.join("Track 02.wav");
        fs::copy(fixture_directory().join("wav_44100_16_stereo.wav"), &first)
            .expect("copy first scan fixture");
        fs::copy(fixture_directory().join("wav_48000_24_stereo.wav"), &second)
            .expect("copy second scan fixture");
        fs::write(second.with_extension("flac"), b"existing").expect("create conflict");
        fs::write(nested.join("notes.txt"), b"ignored").expect("create ignored file");

        service
            .add_scan_inputs(vec![directory.clone(), nested.clone()])
            .expect("start recursive scan");
        let snapshot = wait_for_scan(&service);
        assert_eq!(snapshot.status, "ready");
        assert_eq!(snapshot.input_roots.len(), 1, "nested roots must collapse");
        assert_eq!(snapshot.candidate_files, 2);
        assert_eq!(snapshot.validated_files, 2);
        assert_eq!(snapshot.ready_files, 1);
        assert_eq!(snapshot.roots.len(), 1);
        assert_eq!(file_nodes(&snapshot.roots[0]).len(), 2);
        assert!(file_nodes(&snapshot.roots[0])
            .iter()
            .any(|node| node.issue_code.as_deref() == Some("output_exists")));

        service
            .remove_scan_inputs(vec![first])
            .expect("remove one scanned file");
        let snapshot = wait_for_scan(&service);
        assert_eq!(snapshot.ready_files, 0);
        assert_eq!(file_nodes(&snapshot.roots[0]).len(), 1);
        fs::remove_dir_all(directory).expect("remove scan test directory");
    }

    #[test]
    fn recursive_scan_isolates_invalid_and_missing_inputs() {
        let service = Arc::new(CompressionService::default());
        let directory = test_directory("scan-errors");
        fs::create_dir_all(&directory).expect("create scan error directory");
        let invalid = directory.join("broken.wav");
        fs::write(&invalid, b"not a wave file").expect("create invalid WAV");
        let missing = test_directory("missing-root");
        service
            .add_scan_inputs(vec![directory.clone(), missing])
            .expect("start scan with errors");
        let snapshot = wait_for_scan(&service);
        assert_eq!(snapshot.status, "ready");
        assert_eq!(snapshot.candidate_files, 1);
        assert_eq!(snapshot.ready_files, 0);
        assert!(snapshot
            .warnings
            .iter()
            .any(|warning| warning.code == "invalid_wav"));
        assert!(snapshot
            .warnings
            .iter()
            .any(|warning| warning.code == "path_unavailable"));
        fs::remove_dir_all(directory).expect("remove scan error directory");
    }

    #[test]
    fn cancelled_scan_preserves_a_recoverable_snapshot() {
        let service = CompressionService::default();
        let workspace = ScanWorkspace {
            roots: vec![fixture_directory()],
            exclusions: Vec::new(),
        };
        service.scan_cancel.store(true, Ordering::Release);
        let result = scan_workspace(&workspace, &service.scan_cancel, &service.scan_snapshot);
        assert!(matches!(result, Err(ScanInterrupted)));
        assert_eq!(service.scan_snapshot().status, "idle");
    }

    #[test]
    fn scan_many_wavs_does_not_depend_on_ffprobe_processes() {
        let service = Arc::new(CompressionService::default());
        let directory = test_directory("scan-many");
        fs::create_dir_all(&directory).expect("create bulk scan directory");
        let fixture = fixture_directory().join("wav_44100_16_stereo.wav");
        for index in 0..45 {
            fs::copy(&fixture, directory.join(format!("track-{index:02}.wav")))
                .expect("copy bulk scan fixture");
        }

        service
            .add_scan_inputs(vec![directory.clone()])
            .expect("start bulk WAV scan");
        let snapshot = wait_for_scan(&service);
        assert_eq!(snapshot.status, "ready");
        assert_eq!(snapshot.candidate_files, 45);
        assert_eq!(snapshot.validated_files, 45);
        assert_eq!(snapshot.ready_files, 45);
        assert!(snapshot.warnings.is_empty());
        fs::remove_dir_all(directory).expect("remove bulk scan directory");
    }

    #[test]
    fn in_process_wav_header_validation_accepts_supported_sample_formats() {
        for fixture in [
            "wav_44100_16_stereo.wav",
            "wav_48000_24_stereo.wav",
            "wav_96000_32_stereo.wav",
            "wav_192000_f32_stereo.wav",
        ] {
            validate_wav_header(&fixture_directory().join(fixture))
                .unwrap_or_else(|error| panic!("{fixture} header rejected: {error}"));
        }
    }

    #[test]
    fn linked_directories_are_not_followed_when_the_platform_can_create_one() {
        let service = Arc::new(CompressionService::default());
        let directory = test_directory("scan-link");
        let target = directory.join("target");
        let linked = directory.join("linked");
        fs::create_dir_all(&target).expect("create symlink target");
        fs::copy(
            fixture_directory().join("wav_44100_16_stereo.wav"),
            target.join("linked.wav"),
        )
        .expect("copy linked fixture");
        if !create_directory_link(&target, &linked) {
            fs::remove_dir_all(directory).expect("remove unsupported link test directory");
            return;
        }
        service
            .add_scan_inputs(vec![linked])
            .expect("scan linked directory");
        let snapshot = wait_for_scan(&service);
        assert_eq!(snapshot.ready_files, 0);
        assert!(snapshot
            .warnings
            .iter()
            .any(|warning| warning.code == "linked_path_skipped"));
        fs::remove_dir_all(directory).expect("remove link test directory");
    }

    fn wait_for_completion(service: &CompressionService) -> CompressionSnapshot {
        for _ in 0..200 {
            let snapshot = service.snapshot();
            if snapshot.status != "running" && snapshot.status != "cancelling" {
                return snapshot;
            }
            thread::sleep(Duration::from_millis(25));
        }
        panic!("compression task did not finish");
    }

    fn prime_batch_snapshot(service: &CompressionService, task_id: u64, source: &Path) {
        prime_batch_snapshots(service, task_id, &[source.to_path_buf()]);
    }

    fn prime_batch_snapshots(service: &CompressionService, task_id: u64, sources: &[PathBuf]) {
        *service
            .snapshot
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = CompressionSnapshot {
            task_id,
            status: "running".to_owned(),
            completed: 0,
            total: sources.len(),
            items: sources
                .iter()
                .map(|source| CompressionItem {
                    source: source.to_string_lossy().into_owned(),
                    output: source.with_extension("flac").to_string_lossy().into_owned(),
                    status: "pending".to_owned(),
                    message: None,
                    source_deleted: false,
                    progress: 0.0,
                    source_bytes: fs::metadata(source).map_or(0, |value| value.len()),
                    output_bytes: 0,
                })
                .collect(),
        };
    }

    fn wait_for_scan(service: &CompressionService) -> CompressionScanSnapshot {
        for _ in 0..400 {
            let snapshot = service.scan_snapshot();
            if snapshot.status != "scanning" && snapshot.status != "cancelling" {
                return snapshot;
            }
            thread::sleep(Duration::from_millis(25));
        }
        panic!("compression scan did not finish");
    }

    fn file_nodes(node: &CompressionScanNode) -> Vec<&CompressionScanNode> {
        let mut files = Vec::new();
        if node.kind == "file" {
            files.push(node);
        }
        for child in &node.children {
            files.extend(file_nodes(child));
        }
        files
    }

    #[cfg(target_os = "windows")]
    fn create_directory_link(target: &Path, link: &Path) -> bool {
        std::os::windows::fs::symlink_dir(target, link).is_ok()
    }

    #[cfg(unix)]
    fn create_directory_link(target: &Path, link: &Path) -> bool {
        std::os::unix::fs::symlink(target, link).is_ok()
    }

    fn fixture_directory() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("tests")
            .join("fixtures")
            .join("audio")
    }

    fn test_directory(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("resona-compression-{label}-{nonce}"))
    }
}
