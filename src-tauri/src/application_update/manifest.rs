// SPDX-License-Identifier: GPL-3.0-only
use base64::Engine;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::ApplicationUpdateFailure;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildIdentity {
    pub run_number: u64,
    pub run_attempt: u64,
    pub sha: String,
}

impl BuildIdentity {
    pub fn current() -> Option<Self> {
        Some(Self {
            run_number: option_env!("RESONA_BUILD_NUMBER")?.parse().ok()?,
            run_attempt: option_env!("RESONA_BUILD_ATTEMPT")?.parse().ok()?,
            sha: option_env!("RESONA_BUILD_SHA")?.to_owned(),
        })
    }

    pub fn id(&self) -> String {
        format!(
            "b{}.{}-g{}",
            self.run_number,
            self.run_attempt,
            &self.sha[..7]
        )
    }

    pub fn order(&self) -> (u64, u64) {
        (self.run_number, self.run_attempt)
    }

    fn valid(&self) -> bool {
        self.run_number > 0
            && self.run_attempt > 0
            && self.sha.len() == 40
            && self
                .sha
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    }
}

pub fn parse_tag(tag: &str) -> Option<(Version, Option<BuildIdentity>)> {
    if let Some(value) = tag.strip_prefix("preview-v") {
        let (version, build) = value.rsplit_once("-b")?;
        let version = Version::parse(version).ok()?;
        if version.pre.is_empty() {
            return None;
        }
        let (numbers, sha) = build.split_once("-g")?;
        if sha.len() != 7
            || !sha
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return None;
        }
        let (number, attempt) = numbers.split_once('.')?;
        let identity = BuildIdentity {
            run_number: number.parse().ok()?,
            run_attempt: attempt.parse().ok()?,
            // Full SHA is only trusted after signature verification.
            sha: format!("{sha}{}", "0".repeat(33)),
        };
        if !identity.valid() || format!("preview-v{}-{}", version, identity.id()) != tag {
            return None;
        }
        Some((version, Some(identity)))
    } else {
        Some((Version::parse(tag.strip_prefix('v')?).ok()?, None))
    }
}

pub fn is_newer(
    version: &Version,
    build: Option<&BuildIdentity>,
    current: &Version,
    current_build: Option<&BuildIdentity>,
    preview: bool,
) -> bool {
    match version.cmp_precedence(current) {
        std::cmp::Ordering::Greater => preview || version.pre.is_empty(),
        std::cmp::Ordering::Equal => {
            version == current
                && !version.pre.is_empty()
                && preview
                && build
                    .zip(current_build)
                    .is_some_and(|(next, installed)| next.order() > installed.order())
        }
        std::cmp::Ordering::Less => false,
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformAsset {
    pub url: String,
    pub signature: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    schema: u32,
    version: Version,
    tag: String,
    channel: String,
    build: BuildIdentity,
    platforms: HashMap<String, PlatformAsset>,
}

pub fn verify_signature(
    bytes: &[u8],
    signature: &str,
    public_key: &str,
) -> Result<(), ApplicationUpdateFailure> {
    let decode = |value: &str| -> Result<String, ApplicationUpdateFailure> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(value.trim())
            .map_err(invalid)?;
        String::from_utf8(bytes).map_err(invalid)
    };
    let key = minisign_verify::PublicKey::decode(&decode(public_key)?).map_err(invalid)?;
    let signature = minisign_verify::Signature::decode(&decode(signature)?).map_err(invalid)?;
    key.verify(bytes, &signature, true).map_err(invalid)
}

pub fn verified_asset(
    bytes: &[u8],
    signature: &str,
    public_key: &str,
    tag: &str,
    version: &Version,
) -> Result<(BuildIdentity, PlatformAsset), ApplicationUpdateFailure> {
    verify_signature(bytes, signature, public_key)?;
    validate_manifest(bytes, tag, version)
}

fn validate_manifest(
    bytes: &[u8],
    tag: &str,
    version: &Version,
) -> Result<(BuildIdentity, PlatformAsset), ApplicationUpdateFailure> {
    let manifest: Manifest = serde_json::from_slice(bytes).map_err(invalid)?;
    if !manifest.build.valid() {
        return Err(invalid("invalid build identity"));
    }
    let expected_tag = if version.pre.is_empty() {
        format!("v{version}")
    } else {
        format!("preview-v{version}-{}", manifest.build.id())
    };
    if manifest.schema != 1
        || manifest.version != *version
        || manifest.tag != tag
        || expected_tag != tag
        || manifest.channel
            != if version.pre.is_empty() {
                "stable"
            } else {
                "preview"
            }
        || !manifest.build.valid()
        || manifest.platforms.len() != 1
    {
        return Err(invalid("manifest identity does not match selected release"));
    }
    let asset = manifest
        .platforms
        .get("windows-x86_64")
        .ok_or_else(|| invalid("missing Windows NSIS asset"))?;
    let prefix = format!("https://github.com/Leovikii/Resona/releases/download/{tag}/");
    let name = asset
        .url
        .strip_prefix(&prefix)
        .ok_or_else(|| invalid("installer URL is outside selected release"))?;
    if name.contains('/')
        || !name.ends_with("_windows_x86_64-setup.exe")
        || asset.signature.trim().is_empty()
    {
        return Err(invalid("invalid NSIS asset or signature"));
    }
    Ok((manifest.build, asset.clone()))
}

fn invalid(error: impl std::fmt::Display) -> ApplicationUpdateFailure {
    ApplicationUpdateFailure::new(
        "update_manifest_invalid",
        format!("update manifest verification failed: {error}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn build(number: u64, attempt: u64) -> BuildIdentity {
        BuildIdentity {
            run_number: number,
            run_attempt: attempt,
            sha: "a".repeat(40),
        }
    }

    #[test]
    fn preview_order_is_numeric_and_never_overrides_application_version() {
        let version = Version::parse("0.3.0-alpha.1").unwrap();
        assert!(is_newer(
            &version,
            Some(&build(10, 1)),
            &version,
            Some(&build(9, 9)),
            true
        ));
        assert!(is_newer(
            &version,
            Some(&build(10, 2)),
            &version,
            Some(&build(10, 1)),
            true
        ));
        for (next, installed) in [(build(10, 1), build(10, 1)), (build(9, 9), build(10, 1))] {
            assert!(!is_newer(
                &version,
                Some(&next),
                &version,
                Some(&installed),
                true
            ));
        }
        assert!(!is_newer(
            &version,
            Some(&build(999, 1)),
            &Version::parse("0.3.0").unwrap(),
            Some(&build(1, 1)),
            true
        ));
        assert!(!is_newer(
            &version,
            Some(&build(10, 1)),
            &version,
            Some(&build(9, 1)),
            false
        ));
        assert!(!is_newer(
            &version,
            Some(&build(10, 1)),
            &version,
            None,
            true
        ));
        assert!(is_newer(
            &Version::parse("0.3.0").unwrap(),
            Some(&build(1, 1)),
            &version,
            Some(&build(999, 1)),
            false
        ));
    }

    #[test]
    fn tag_parser_rejects_ambiguous_builds() {
        assert!(parse_tag("preview-v0.3.0-alpha.1-b42.2-gaaaaaaa").is_some());
        for tag in [
            "preview-v0.3.0-b42.1-gaaaaaaa",
            "preview-v0.3.0-alpha.1-b0.1-gaaaaaaa",
            "preview-v0.3.0-alpha.1-b01.1-gaaaaaaa",
            "preview-v0.3.0-alpha.1-b42.1-gAAAAAAA",
            "0.3.0",
        ] {
            assert!(parse_tag(tag).is_none(), "{tag}");
        }
    }

    #[test]
    fn tauri_signature_fixture_verifies_and_modified_bytes_are_rejected() {
        let bytes = include_bytes!("../../../tests/fixtures/update/latest.json");
        let signature = include_str!("../../../tests/fixtures/update/latest.json.sig");
        let public_key = include_str!("../../../tests/fixtures/update/public-key.txt");
        let version = Version::parse("0.3.0-alpha.1").unwrap();
        let tag = "preview-v0.3.0-alpha.1-b42.1-gaaaaaaa";
        verified_asset(bytes, signature, public_key, tag, &version)
            .expect("the fixture must retain its exact signed bytes after checkout");
        let crlf = String::from_utf8(bytes.to_vec())
            .unwrap()
            .replace('\n', "\r\n");
        assert!(verified_asset(crlf.as_bytes(), signature, public_key, tag, &version).is_err());
        let mut changed = bytes.to_vec();
        changed.push(b' ');
        assert!(verified_asset(&changed, signature, public_key, tag, &version).is_err());
        assert!(verified_asset(bytes, "", public_key, tag, &version).is_err());
        assert!(verified_asset(bytes, signature, public_key, "other-tag", &version).is_err());
    }

    #[test]
    fn manifest_is_bound_to_tag_build_and_installer() {
        let tag = "preview-v0.3.0-alpha.1-b42.1-gaaaaaaa";
        let version = Version::parse("0.3.0-alpha.1").unwrap();
        let mut json = serde_json::json!({"schema": 1, "version": version, "tag": tag, "channel": "preview", "build": build(42,1), "platforms": {"windows-x86_64": {"url": format!("https://github.com/Leovikii/Resona/releases/download/{tag}/Resona_windows_x86_64-setup.exe"), "signature":"installer-signature"}}});
        let bytes = serde_json::to_vec(&json).unwrap();
        assert!(validate_manifest(&bytes, tag, &version).is_ok());
        assert!(validate_manifest(&bytes, "other-tag", &version).is_err());
        assert!(verified_asset(&bytes, "", "", tag, &version).is_err());
        json["build"]["runNumber"] = 41.into();
        assert!(validate_manifest(&serde_json::to_vec(&json).unwrap(), tag, &version).is_err());
        json["build"] = serde_json::to_value(build(42, 1)).unwrap();
        json["platforms"]["windows-x86_64"]["url"] = "https://example.com/setup.exe".into();
        assert!(validate_manifest(&serde_json::to_vec(&json).unwrap(), tag, &version).is_err());
    }
}
