use crate::application_update::ApplicationUpdateFailure;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, ApplicationUpdateFailure>;
fn error(code: &str, message: impl std::fmt::Display) -> ApplicationUpdateFailure {
    ApplicationUpdateFailure::new(code, message.to_string())
}
pub struct PreparedUpdate {
    _directory: tempfile::TempDir,
    file: PathBuf,
    digest: Vec<u8>,
    package: String,
    version: String,
    installed: String,
    transaction: String,
    changes: Vec<String>,
    created: Instant,
}

fn query(args: &[&str]) -> Result<String> {
    let mut log = tempfile::tempfile().map_err(|e| error("update_package_query", e))?;
    let mut child = Command::new("/usr/bin/pacman")
        .env("LC_ALL", "C")
        .args(args)
        .stdin(Stdio::null())
        .stdout(
            log.try_clone()
                .map_err(|e| error("update_package_query", e))?,
        )
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| error("update_package_query", e))?;
    // Read-only probes may time out; the installing Pacman below must never be killed.
    let started = Instant::now();
    loop {
        if child
            .try_wait()
            .map_err(|e| error("update_package_query", e))?
            .is_some()
        {
            break;
        }
        if started.elapsed() > Duration::from_secs(30) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error("update_package_query", "Pacman query timed out"));
        }
        thread::sleep(Duration::from_millis(50));
    }
    let output = child
        .wait_with_output()
        .map_err(|e| error("update_package_query", e))?;
    if !output.status.success() || !output.stderr.is_empty() {
        return Err(error(
            "update_package_query",
            String::from_utf8_lossy(&output.stderr),
        ));
    }
    log.seek(SeekFrom::Start(0))
        .map_err(|e| error("update_package_query", e))?;
    let mut text = String::new();
    log.take(256 * 1024)
        .read_to_string(&mut text)
        .map_err(|e| error("update_package_query", e))?;
    if text.len() >= 256 * 1024 {
        return Err(error("update_package_query", "Pacman response too large"));
    }
    Ok(text.trim().to_owned())
}
fn package_version(version: &str) -> String {
    format!(
        "{}-1",
        version
            .replace("-alpha.", "alpha")
            .replace("-beta.", "beta")
            .replace("-rc.", "rc")
    )
}
fn installed() -> Result<(String, String)> {
    let os = fs::read_to_string("/etc/os-release")
        .map_err(|e| error("update_package_unsupported", e))?;
    if !os.lines().filter_map(|l| l.split_once('=')).any(|(k, v)| {
        (k == "ID" || k == "ID_LIKE")
            && v.trim_matches('"')
                .split_whitespace()
                .any(|s| matches!(s, "arch" | "cachyos"))
    }) {
        return Err(error(
            "update_package_unsupported",
            "Arch Linux is required",
        ));
    }
    if std::env::current_exe().ok().as_deref() != Some(Path::new("/usr/bin/resona")) {
        return Err(error(
            "update_package_unmanaged",
            "Install the official system package first",
        ));
    }
    let package = query(&["-Qoq", "--", "/usr/bin/resona"])?;
    if !["resona-preview-bin", "resona-bin"].contains(&package.as_str())
        || !Path::new("/usr/bin/pkexec").is_file()
    {
        return Err(error(
            "update_package_unmanaged",
            "Resona package or Polkit is unavailable",
        ));
    }
    let installed = query(&["-Q", "--", &package])?;
    if installed != format!("{package} {}", package_version(env!("CARGO_PKG_VERSION"))) {
        return Err(error(
            "update_selection_expired",
            "Restart the installed version first",
        ));
    }
    Ok((package, installed))
}
fn parse_transaction(text: &str, package: &str, version: &str) -> Result<Vec<String>> {
    let mut target = false;
    let mut changes = Vec::new();
    for line in text.lines() {
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 4
            || fields.iter().any(|s| s.is_empty())
            || !["x86_64", "any"].contains(&fields[2])
        {
            return Err(error(
                "update_package_query",
                "Unexpected package transaction",
            ));
        }
        if fields[0] == package {
            if target || fields[1] != version || fields[2] != "x86_64" {
                return Err(error(
                    "update_manifest_invalid",
                    "Wrong package version or architecture",
                ));
            }
            target = true;
        }
        changes.push(format!("{} {}", fields[0], fields[1]));
    }
    if !target {
        return Err(error(
            "update_manifest_invalid",
            "Resona is missing from the transaction",
        ));
    }
    Ok(changes)
}
fn transaction(file: &Path, package: &str, version: &str) -> Result<(String, Vec<String>)> {
    let text = query(&[
        "-Up",
        "--noconfirm",
        "--print-format",
        "%n\t%v\t%a\t%l",
        "--",
        file.to_str()
            .ok_or_else(|| error("update_package_temp", "Invalid package path"))?,
    ])?;
    let changes = parse_transaction(&text, package, version)?;
    Ok((text, changes))
}
pub fn prepare(version: &str, bytes: &[u8]) -> Result<PreparedUpdate> {
    let (package, installed) = installed()?;
    let expected_package = if version.contains('-') {
        "resona-preview-bin"
    } else {
        "resona-bin"
    };
    if package != expected_package {
        return Err(error(
            "update_package_unmanaged",
            "Install the selected release package manually to switch channels",
        ));
    }
    let directory = tempfile::Builder::new()
        .prefix("resona-update-")
        .tempdir()
        .map_err(|e| error("update_package_temp", e))?;
    let file = directory.path().join("resona.pkg.tar.zst");
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&file)
        .map_err(|e| error("update_package_temp", e))?;
    output
        .write_all(bytes)
        .and_then(|_| output.sync_all())
        .map_err(|e| error("update_package_temp", e))?;
    let version = package_version(version);
    if query(&[
        "-Qp",
        "--",
        file.to_str()
            .ok_or_else(|| error("update_package_temp", "Invalid path"))?,
    ])? != format!("{package} {version}")
    {
        return Err(error(
            "update_manifest_invalid",
            "Downloaded package identity differs",
        ));
    }
    let (transaction, changes) = transaction(&file, &package, &version)?;
    Ok(PreparedUpdate {
        _directory: directory,
        file,
        digest: Sha256::digest(bytes).to_vec(),
        package,
        version,
        installed,
        transaction,
        changes,
        created: Instant::now(),
    })
}
impl PreparedUpdate {
    pub fn changes(&self) -> Vec<String> {
        self.changes.clone()
    }
    pub fn install(self) -> Result<()> {
        if self.created.elapsed() > Duration::from_secs(1800) || installed()?.1 != self.installed {
            return Err(error(
                "update_selection_expired",
                "Check and download the update again",
            ));
        }
        if fs::symlink_metadata(&self.file)
            .map_err(|e| error("update_package_temp", e))?
            .file_type()
            .is_symlink()
            || Sha256::digest(fs::read(&self.file).map_err(|e| error("update_package_temp", e))?)
                .as_slice()
                != self.digest
            || transaction(&self.file, &self.package, &self.version)?.0 != self.transaction
        {
            return Err(error(
                "update_selection_expired",
                "The installation preview changed",
            ));
        }
        let mut log = tempfile::tempfile().map_err(|e| error("update_package_temp", e))?;
        let status = Command::new("/usr/bin/pkexec")
            .args([
                "--disable-internal-agent",
                "/usr/bin/pacman",
                "-U",
                "--noconfirm",
                "--",
            ])
            .arg(&self.file)
            .stdin(Stdio::null())
            .stdout(
                log.try_clone()
                    .map_err(|e| error("update_package_temp", e))?,
            )
            .stderr(
                log.try_clone()
                    .map_err(|e| error("update_package_temp", e))?,
            )
            .status()
            .map_err(|e| error("update_package_install", e))?;
        match status.code() {
            Some(126) => {
                return Err(error(
                    "update_authorization_cancelled",
                    "Authorization cancelled",
                ))
            }
            Some(127) => {
                return Err(error(
                    "update_authorization_failed",
                    "System authorization is unavailable",
                ))
            }
            _ => {}
        }
        if !status.success() {
            log.seek(SeekFrom::Start(0))
                .map_err(|e| error("update_package_install", e))?;
            let mut diagnostic = String::new();
            log.take(16 * 1024)
                .read_to_string(&mut diagnostic)
                .map_err(|e| error("update_package_install", e))?;
            return Err(error(
                "update_package_install",
                format!("Pacman failed: {}", diagnostic.trim()),
            ));
        }
        if query(&["-Q", "--", &self.package])? != format!("{} {}", self.package, self.version)
            || query(&["-Qoq", "--", "/usr/bin/resona"])? != self.package
        {
            return Err(error(
                "update_package_incomplete",
                "Installed version could not be confirmed",
            ));
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires the CI-built Arch package and Pacman"]
    fn packaged_update_metadata() {
        let file = std::env::var("RESONA_TEST_PACKAGE").expect("select a locally built package");
        let version = package_version(env!("CARGO_PKG_VERSION"));
        let package = if env!("CARGO_PKG_VERSION").contains('-') {
            "resona-preview-bin"
        } else {
            "resona-bin"
        };
        assert_eq!(
            query(&["-Qp", "--", &file]).unwrap(),
            format!("{package} {version}")
        );
        assert!(!transaction(Path::new(&file), package, &version)
            .unwrap()
            .1
            .is_empty());
    }
    #[test]
    fn transaction_requires_the_exact_resona_package_and_architecture() {
        let line = "resona-preview-bin\t0.3.0alpha3-1\tx86_64\t/tmp/resona.pkg.tar.zst";
        assert_eq!(
            parse_transaction(line, "resona-preview-bin", "0.3.0alpha3-1").unwrap(),
            vec!["resona-preview-bin 0.3.0alpha3-1"]
        );
        for bad in [
            line.replace("alpha3", "alpha4"),
            line.replace("x86_64", "aarch64"),
            line.replace("resona-preview-bin", "other"),
            format!("{line}\n{line}"),
            "prompt".into(),
        ] {
            assert!(parse_transaction(&bad, "resona-preview-bin", "0.3.0alpha3-1").is_err());
        }
        assert_eq!(package_version("0.3.0-rc.2"), "0.3.0rc2-1");
    }
}
