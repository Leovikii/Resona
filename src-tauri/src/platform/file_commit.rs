// SPDX-License-Identifier: GPL-3.0-only

use std::{fs, io, path::Path};

pub fn commit_new_file(temporary: &Path, destination: &Path) -> io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        fs::rename(temporary, destination)
    }
    #[cfg(not(target_os = "windows"))]
    {
        // POSIX rename replaces existing files; linking publishes without that race.
        fs::hard_link(temporary, destination)?;
        fs::remove_file(temporary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_never_replaces_an_output_created_after_validation() {
        let root = std::env::temp_dir().join(format!("resona-commit-{}", fastrand::u64(..)));
        fs::create_dir_all(&root).unwrap();
        let temporary = root.join("temporary");
        let destination = root.join("output");
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
}
