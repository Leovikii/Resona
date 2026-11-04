// SPDX-License-Identifier: GPL-3.0-only

use std::{
    io::{self, Write},
    path::Path,
};

pub fn write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("state path has no parent"))?;
    std::fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_complete_state_and_cleans_up_failed_replacement() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.json");
        write(&path, br#"{"enabled":true}"#).unwrap();
        write(&path, br#"{"enabled":false}"#).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), br#"{"enabled":false}"#);
        let blocked = directory.path().join("blocked");
        std::fs::create_dir(&blocked).unwrap();
        assert!(write(&blocked, b"never committed").is_err());
        assert!(blocked.is_dir());
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 2);
    }
}
