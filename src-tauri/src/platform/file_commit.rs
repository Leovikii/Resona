// SPDX-License-Identifier: GPL-3.0-only

use std::{io, path::Path};

pub fn commit_new_file(temporary: &Path, destination: &Path) -> io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows::{
            core::PCWSTR,
            Win32::Storage::FileSystem::{MoveFileExW, MOVE_FILE_FLAGS},
        };

        let wide_path = |path: &Path| -> io::Result<Vec<u16>> {
            let mut value: Vec<u16> = path.as_os_str().encode_wide().collect();
            if value.contains(&0) {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "NUL in path"));
            }
            value.push(0);
            Ok(value)
        };
        let temporary = wide_path(temporary)?;
        let destination = wide_path(destination)?;
        // std::fs::rename replaces existing files on Windows too; omit REPLACE_EXISTING.
        if unsafe {
            MoveFileExW(
                PCWSTR(temporary.as_ptr()),
                PCWSTR(destination.as_ptr()),
                MOVE_FILE_FLAGS(0),
            )
        }
        .as_bool()
        {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        // POSIX rename replaces existing files; linking publishes without that race.
        std::fs::hard_link(temporary, destination)?;
        std::fs::remove_file(temporary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn commit_never_replaces_an_output_created_after_validation() {
        let root = std::env::temp_dir().join(format!("resona-commit-{}", fastrand::u64(..)));
        fs::create_dir_all(&root).unwrap();
        let temporary = root.join("临时 音频.flac");
        let destination = root.join("输出 音频.flac");
        fs::write(&temporary, b"new").unwrap();
        fs::write(&destination, b"existing").unwrap();
        assert!(commit_new_file(&temporary, &destination).is_err());
        assert_eq!(fs::read(&destination).unwrap(), b"existing");
        assert_eq!(fs::read(&temporary).unwrap(), b"new");
        fs::remove_file(&destination).unwrap();
        commit_new_file(&temporary, &destination).unwrap();
        assert_eq!(fs::read(&destination).unwrap(), b"new");
        assert!(!temporary.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_paths_cannot_publish_a_truncated_filename() {
        let root = std::env::temp_dir().join(format!("resona-commit-{}", fastrand::u64(..)));
        fs::create_dir_all(&root).unwrap();
        let temporary = root.join("temporary");
        let destination = root.join("output");
        fs::write(&temporary, b"new").unwrap();
        for (source, target) in [
            (temporary.clone(), root.join("output\0ignored")),
            (root.join("temporary\0ignored"), destination.clone()),
        ] {
            assert!(commit_new_file(&source, &target).is_err());
            assert_eq!(fs::read(&temporary).unwrap(), b"new");
            assert!(!destination.exists());
        }
        fs::remove_dir_all(root).unwrap();
    }
}
