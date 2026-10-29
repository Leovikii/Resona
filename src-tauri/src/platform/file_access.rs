// SPDX-License-Identifier: GPL-3.0-only

use crate::playback::{PlaybackErrorCode, PlaybackFailure};
use std::path::{Path, PathBuf};

pub enum DirectoryEntries {
    Local(std::fs::ReadDir),
    #[cfg(target_os = "linux")]
    Network(std::vec::IntoIter<PathBuf>),
}

impl Iterator for DirectoryEntries {
    type Item = std::io::Result<PathBuf>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Local(entries) => entries.next().map(|entry| entry.map(|entry| entry.path())),
            #[cfg(target_os = "linux")]
            Self::Network(entries) => entries.next().map(Ok),
        }
    }
}

pub fn read_directory(path: &Path) -> std::io::Result<DirectoryEntries> {
    #[cfg(target_os = "linux")]
    if is_kio_fuse_path(path) {
        let uri = mapped_location(path)
            .map_err(|_| std::io::Error::other("Cannot resolve network directory"))?;
        return network_directory(path, &uri)
            .map(|entries| DirectoryEntries::Network(entries.into_iter()));
    }
    std::fs::read_dir(path).map(DirectoryEntries::Local)
}

pub fn resolve_path(path: PathBuf) -> Result<PathBuf, PlaybackFailure> {
    match path.to_str() {
        Some(value) => resolve_location(value),
        None => refresh_mapped_path(path),
    }
}

pub fn resolve_location(value: &str) -> Result<PathBuf, PlaybackFailure> {
    if !value.contains("://") {
        return refresh_mapped_path(PathBuf::from(value));
    }
    let uri = tauri::Url::parse(value).map_err(|_| failure(PlaybackErrorCode::InvalidLocation))?;
    match uri.scheme() {
        "file" => uri
            .to_file_path()
            .map_err(|_| failure(PlaybackErrorCode::InvalidLocation))
            .and_then(refresh_mapped_path),
        "nfs"
            if uri.host_str().is_some()
                && uri.username().is_empty()
                && uri.password().is_none() =>
        {
            map_nfs(uri.as_str())
        }
        _ => Err(failure(PlaybackErrorCode::UnsupportedLocation)),
    }
}

fn failure(code: PlaybackErrorCode) -> PlaybackFailure {
    PlaybackFailure {
        code,
        message: "File location could not be resolved".into(),
    }
}

#[cfg(target_os = "linux")]
fn map_nfs(uri: &str) -> Result<PathBuf, PlaybackFailure> {
    use glib::variant::ToVariant;
    use gtk::{gio, glib};
    let parent = tauri::Url::parse(uri)
        .and_then(|uri| uri.join("."))
        .map_err(|_| failure(PlaybackErrorCode::InvalidLocation))?;
    let result = (|| {
        let connection = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE)?;
        // Resolve the parent first: a prior failed FUSE enumeration may have invalidated it.
        connection.call_sync(
            Some("org.kde.KIOFuse"),
            "/org/kde/KIOFuse",
            "org.kde.KIOFuse.VFS",
            "mountUrl",
            Some(&(parent.as_str(),).to_variant()),
            None,
            gio::DBusCallFlags::NONE,
            15_000,
            gio::Cancellable::NONE,
        )?;
        connection.call_sync(
            Some("org.kde.KIOFuse"),
            "/org/kde/KIOFuse",
            "org.kde.KIOFuse.VFS",
            "mountUrl",
            Some(&(uri,).to_variant()),
            None,
            gio::DBusCallFlags::NONE,
            15_000,
            gio::Cancellable::NONE,
        )
    })();
    let value = result.map_err(|_| failure(PlaybackErrorCode::NetworkLocationUnavailable))?;
    let (path,) = value
        .get::<(String,)>()
        .ok_or_else(|| failure(PlaybackErrorCode::NetworkLocationUnavailable))?;
    let path = PathBuf::from(path);
    if !path.is_absolute() {
        return Err(failure(PlaybackErrorCode::NetworkLocationUnavailable));
    }
    Ok(path)
}

#[cfg(not(target_os = "linux"))]
fn map_nfs(_: &str) -> Result<PathBuf, PlaybackFailure> {
    Err(failure(PlaybackErrorCode::UnsupportedLocation))
}

#[cfg(target_os = "linux")]
fn is_kio_fuse_path(path: &Path) -> bool {
    if !path.is_absolute() {
        return false;
    }
    gtk::gio::UnixMountEntry::mounts().0.iter().any(|mount| {
        mount.fs_type() == "fuse.kio-fuse" && path.starts_with(mount.unix_mount_get_mount_path())
    })
}

#[cfg(target_os = "linux")]
fn refresh_mapped_path(path: PathBuf) -> Result<PathBuf, PlaybackFailure> {
    if !is_kio_fuse_path(&path) {
        return Ok(path);
    }
    let remote = mapped_location(&path)?;
    if remote.starts_with("nfs://") {
        return resolve_location(&remote);
    }
    Ok(path)
}

#[cfg(target_os = "linux")]
fn mapped_location(path: &Path) -> Result<String, PlaybackFailure> {
    use gtk::{gio, glib::variant::ToVariant};
    let connection = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE)
        .map_err(|_| failure(PlaybackErrorCode::NetworkLocationUnavailable))?;
    let value = connection
        .call_sync(
            Some("org.kde.KIOFuse"),
            "/org/kde/KIOFuse",
            "org.kde.KIOFuse.VFS",
            "remoteUrl",
            Some(&(path.to_string_lossy().as_ref(),).to_variant()),
            None,
            gio::DBusCallFlags::NONE,
            15_000,
            gio::Cancellable::NONE,
        )
        .map_err(|_| failure(PlaybackErrorCode::NetworkLocationUnavailable))?;
    let (remote,) = value
        .get::<(String,)>()
        .ok_or_else(|| failure(PlaybackErrorCode::NetworkLocationUnavailable))?;
    Ok(remote)
}

#[cfg(target_os = "linux")]
pub fn run_directory_worker() -> i32 {
    extern "C" {
        fn resona_kio_directory() -> std::ffi::c_int;
    }
    // This entry runs before GTK/threads; C++ owns its process-local Qt event loop.
    unsafe { resona_kio_directory() }
}

#[cfg(target_os = "linux")]
struct DirectoryWorker {
    child: std::process::Child,
    output: std::io::BufReader<std::os::unix::net::UnixStream>,
}

#[cfg(target_os = "linux")]
impl DirectoryWorker {
    fn start() -> std::io::Result<Self> {
        use std::os::fd::OwnedFd;
        use std::os::unix::net::UnixStream;
        use std::process::{Command, Stdio};
        let executable = std::env::current_exe()?;
        #[cfg(test)]
        let executable = std::env::var_os("RESONA_TEST_BINARY")
            .map(PathBuf::from)
            .unwrap_or(executable);
        let (output, child_output) = UnixStream::pair()?;
        output.set_read_timeout(Some(std::time::Duration::from_secs(20)))?;
        let child = Command::new(executable)
            .arg("--resona-list-directory")
            .stdin(Stdio::piped())
            .stdout(Stdio::from(OwnedFd::from(child_output)))
            .stderr(Stdio::null())
            .spawn()?;
        Ok(Self {
            child,
            output: std::io::BufReader::new(output),
        })
    }

    fn list(&mut self, path: &Path, request: &[u8]) -> std::io::Result<Vec<PathBuf>> {
        use std::io::{BufRead, Read, Write};
        let input = self.child.stdin.as_mut().expect("worker stdin is piped");
        input.write_all(request)?;
        input.write_all(b"\n")?;
        let mut bytes = Vec::new();
        self.output
            .by_ref()
            .take(16 * 1024 * 1024 + 1)
            .read_until(b'\n', &mut bytes)?;
        if bytes.len() > 16 * 1024 * 1024 || bytes.last() != Some(&b'\n') {
            return Err(std::io::Error::other("Invalid network directory response"));
        }
        directory_paths(path, &bytes)
    }
}

#[cfg(target_os = "linux")]
impl Drop for DirectoryWorker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(target_os = "linux")]
fn network_directory(path: &Path, uri: &str) -> std::io::Result<Vec<PathBuf>> {
    // One serialized session preserves KIO connections/handles; EOF ends the child with its parent.
    static WORKER: std::sync::Mutex<Option<DirectoryWorker>> = std::sync::Mutex::new(None);
    let request = serde_json::to_vec(&serde_json::json!({"uri": uri}))?;
    if request.len() > 65536 {
        return Err(std::io::Error::other("Network directory URI is too long"));
    }
    let mut worker = WORKER
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if worker.is_none() {
        *worker = Some(DirectoryWorker::start()?);
    }
    let result = worker.as_mut().unwrap().list(path, &request);
    if let Err(error) = &result {
        *worker = None;
        log::warn!("Network directory listing failed: {error}");
    }
    result
}

#[cfg(target_os = "linux")]
fn directory_paths(path: &Path, bytes: &[u8]) -> std::io::Result<Vec<PathBuf>> {
    let names: Vec<String> = serde_json::from_slice(bytes)?;
    if names.len() > 100000
        || names.iter().any(|name| {
            name.is_empty() || name == "." || name == ".." || name.contains(['/', '\0'])
        })
    {
        return Err(std::io::Error::other("Invalid network directory entries"));
    }
    Ok(names.into_iter().map(|name| path.join(name)).collect())
}

#[cfg(not(target_os = "linux"))]
fn refresh_mapped_path(path: PathBuf) -> Result<PathBuf, PlaybackFailure> {
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directory_iteration_preserves_local_paths_and_entry_errors() {
        let root = tempfile::tempdir().unwrap();
        let child = root.path().join("中文 # %.wav");
        std::fs::write(&child, b"audio").unwrap();
        let paths = read_directory(root.path())
            .unwrap()
            .collect::<std::io::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(paths, [child]);
        assert!(read_directory(&root.path().join("missing")).is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn network_names_are_literal_children_not_paths_or_uris() {
        let root = Path::new("/mapped/music");
        let names = vec!["中文 # %\\\n.wav", ".hidden", "album"];
        let bytes = serde_json::to_vec(&names).unwrap();
        assert_eq!(
            directory_paths(root, &bytes).unwrap(),
            names.iter().map(|name| root.join(name)).collect::<Vec<_>>()
        );
        for name in [
            "",
            ".",
            "..",
            "../outside",
            "/absolute",
            "nfs://host/path",
            "null\0byte",
        ] {
            assert!(directory_paths(root, &serde_json::to_vec(&vec![name]).unwrap()).is_err());
        }
        assert!(directory_paths(root, b"not JSON").is_err());
        assert!(directory_paths(root, b"[null]").is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "requires RESONA_TEST_BINARY and a desktop session; contacts a reserved test address"]
    fn unreachable_network_directory_returns_within_the_worker_deadline() {
        let start = std::time::Instant::now();
        let error =
            network_directory(Path::new("/unused"), "nfs://192.0.2.1/unavailable/").unwrap_err();
        assert!(start.elapsed() < std::time::Duration::from_secs(22));
        assert!(!error.to_string().is_empty());
        eprintln!("Unreachable directory returned after {:?}", start.elapsed());
    }
    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "requires RESONA_TEST_NFS_LOCATIONS and RESONA_TEST_BINARY in a KDE session"]
    fn network_worker_reuses_process_and_recovers_after_protocol_failure() {
        let locations: Vec<String> =
            serde_json::from_str(&std::env::var("RESONA_TEST_NFS_LOCATIONS").unwrap()).unwrap();
        let root = resolve_location(&locations[0])
            .unwrap()
            .parent()
            .unwrap()
            .to_owned();
        let uri = mapped_location(&root).unwrap();
        let request = serde_json::to_vec(&serde_json::json!({"uri": uri})).unwrap();
        let mut worker = DirectoryWorker::start().unwrap();
        let pid = worker.child.id();
        let mut first = worker.list(&root, &request).unwrap();
        first.sort();
        assert!(!first.is_empty());
        for _ in 0..3 {
            let mut next = worker.list(&root, &request).unwrap();
            next.sort();
            assert_eq!(next, first);
            assert_eq!(worker.child.id(), pid);
            assert!(worker.child.try_wait().unwrap().is_none());
        }
        assert!(network_directory(&root, "https://invalid.example/").is_err());
        let mut recovered = network_directory(&root, &uri).unwrap();
        recovered.sort();
        assert_eq!(recovered, first);
        assert!(network_directory(&root, &"x".repeat(65537)).is_err());
        worker.child.stdin.take();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while worker.child.try_wait().unwrap().is_none() {
            assert!(
                std::time::Instant::now() < deadline,
                "worker exits on parent EOF"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    #[test]
    fn paths_and_file_uris_are_not_confused_with_network_protocols() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("中文 # space.flac");
        let uri = tauri::Url::from_file_path(&path).unwrap();
        assert_eq!(resolve_location(uri.as_str()).unwrap(), path);
        assert_eq!(
            resolve_location("C:\\Music\\a.flac").unwrap(),
            PathBuf::from("C:\\Music\\a.flac")
        );
        assert_eq!(
            resolve_location("relative.flac").unwrap(),
            PathBuf::from("relative.flac")
        );
        assert_eq!(
            resolve_location("https://example.com/a.flac")
                .unwrap_err()
                .code,
            PlaybackErrorCode::UnsupportedLocation
        );
        assert!(resolve_location("nfs://user:secret@host/export/a.flac").is_err());
    }
}
