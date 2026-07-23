// SPDX-License-Identifier: GPL-3.0-only

use tauri::plugin::TauriPlugin;
use tauri::Runtime;
use tauri_plugin_log::{RotationStrategy, Target, TargetKind};

const LOG_FILE_NAME: &str = "resona";
const LOG_MAX_BYTES: u128 = 256 * 1024;

pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    tauri_plugin_log::Builder::new()
        .clear_targets()
        .target(Target::new(TargetKind::LogDir {
            file_name: Some(LOG_FILE_NAME.to_owned()),
        }))
        .level(log::LevelFilter::Info)
        .max_file_size(LOG_MAX_BYTES)
        .rotation_strategy(RotationStrategy::KeepOne)
        .filter(accepts_target)
        .build()
}

fn accepts_target(metadata: &log::Metadata<'_>) -> bool {
    matches!(metadata.target(), "resona" | "resona_lib")
        || metadata.target().starts_with("resona::")
        || metadata.target().starts_with("resona_lib::")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persisted_logs_only_accept_resona_targets() {
        let application = log::Metadata::builder()
            .level(log::Level::Info)
            .target("resona_lib::application_update")
            .build();
        let dependency = log::Metadata::builder()
            .level(log::Level::Warn)
            .target("hyper::client")
            .build();
        let similarly_named_dependency = log::Metadata::builder()
            .level(log::Level::Info)
            .target("resonance")
            .build();

        assert!(accepts_target(&application));
        assert!(!accepts_target(&dependency));
        assert!(!accepts_target(&similarly_named_dependency));
        assert_eq!(LOG_MAX_BYTES, 262_144);
    }
}
