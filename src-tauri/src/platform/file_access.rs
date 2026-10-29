// SPDX-License-Identifier: GPL-3.0-only

use crate::playback::{PlaybackErrorCode, PlaybackFailure};
use std::path::PathBuf;

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

// KIO-FUSE directory enumeration can invalidate otherwise readable NFS children.
// Use GIO mount metadata, never a runtime-directory name heuristic.
#[cfg(target_os = "linux")]
pub fn supports_directory_discovery(path: &std::path::Path) -> bool {
    if !path.is_absolute() {
        return true;
    }
    !gtk::gio::UnixMountEntry::mounts().0.iter().any(|mount| {
        mount.fs_type() == "fuse.kio-fuse" && path.starts_with(mount.unix_mount_get_mount_path())
    })
}

#[cfg(not(target_os = "linux"))]
pub fn supports_directory_discovery(_: &std::path::Path) -> bool {
    true
}

#[cfg(target_os = "linux")]
fn refresh_mapped_path(path: PathBuf) -> Result<PathBuf, PlaybackFailure> {
    if supports_directory_discovery(&path) {
        return Ok(path);
    }
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
    if remote.starts_with("nfs://") {
        return resolve_location(&remote);
    }
    Ok(path)
}

#[cfg(not(target_os = "linux"))]
fn refresh_mapped_path(path: PathBuf) -> Result<PathBuf, PlaybackFailure> {
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
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
