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
fn network_directory(path: &Path, uri: &str) -> std::io::Result<Vec<PathBuf>> {
    use std::io::{Read, Seek, Write};
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    let executable = std::env::current_exe()?;
    #[cfg(test)]
    let executable = std::env::var_os("RESONA_TEST_BINARY")
        .map(PathBuf::from)
        .unwrap_or(executable);
    let mut output = tempfile::tempfile()?;
    let mut child = Command::new(executable)
        .arg("--resona-list-directory")
        .stdin(Stdio::piped())
        .stdout(output.try_clone()?)
        .stderr(Stdio::null())
        .spawn()?;
    let result = (|| {
        let request = serde_json::to_vec(&serde_json::json!({"uri": uri}))?;
        if request.len() > 65536 {
            return Err(std::io::Error::other("Network directory URI is too long"));
        }
        child.stdin.take().unwrap().write_all(&request)?;
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            if let Some(status) = child.try_wait()? {
                if !status.success() {
                    return Err(std::io::Error::other(format!(
                        "KIO directory request failed: {status}"
                    )));
                }
                break;
            }
            if Instant::now() >= deadline {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "KIO directory request timed out",
                ));
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        output.rewind()?;
        let mut bytes = Vec::new();
        output.take(16 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(std::io::Error::other(
                "Network directory response is too large",
            ));
        }
        directory_paths(path, &bytes)
    })();
    if let Err(error) = &result {
        let _ = child.kill();
        let _ = child.wait();
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
