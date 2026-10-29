fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        build_directory_worker();
    }
    println!("cargo:rerun-if-changed=icons/icon.ico");
    let keys = [
        "RESONA_BUILD_NUMBER",
        "RESONA_BUILD_ATTEMPT",
        "RESONA_BUILD_SHA",
        "RESONA_UPDATER_PUBLIC_KEY",
    ];
    for key in keys {
        println!("cargo:rerun-if-env-changed={key}");
    }
    let values: Vec<_> = keys[..3].iter().map(std::env::var).collect();
    if values.iter().any(Result::is_ok) {
        let values: Vec<_> = values
            .into_iter()
            .map(|value| value.expect("complete build identity required"))
            .collect();
        for value in &values[..2] {
            assert!(
                value.parse::<u64>().is_ok_and(|n| n > 0),
                "invalid build number"
            );
        }
        assert!(
            values[2].len() == 40
                && values[2]
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()),
            "full lowercase SHA required"
        );
    }
    tauri_build::build()
}

fn build_directory_worker() {
    println!("cargo:rerun-if-changed=src/platform/kio_directory.cpp");
    let qt = pkg_config::Config::new()
        .atleast_version("6.0")
        .probe("Qt6Core")
        .expect("Linux builds require qt6-base and kio development files");
    let prefix = pkg_config::get_variable("Qt6Core", "prefix").expect("Qt6 installation prefix");
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .file("src/platform/kio_directory.cpp");
    for path in qt.include_paths {
        build.include(path);
    }
    for library in ["KIO", "KIOCore", "KCoreAddons"] {
        build.include(
            std::path::Path::new(&prefix)
                .join("include/KF6")
                .join(library),
        );
    }
    build.compile("resona_kio_directory");
    println!("cargo:rustc-link-lib=KF6KIOCore");
    println!("cargo:rustc-link-lib=KF6CoreAddons");
}
