fn main() {
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
