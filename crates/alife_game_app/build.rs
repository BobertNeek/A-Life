fn main() {
    // Cargo-reported settings for this package, not an effective compiler-flags
    // fingerprint. Keep the values compiled into diagnostics, never read them
    // from the executable's runtime environment.
    for (source, embedded) in [
        ("PROFILE", "ALIFE_GAME_APP_CARGO_PROFILE"),
        ("OPT_LEVEL", "ALIFE_GAME_APP_CARGO_OPT_LEVEL"),
        ("TARGET", "ALIFE_GAME_APP_CARGO_TARGET"),
    ] {
        let value = std::env::var(source).expect("Cargo build setting missing");
        println!("cargo:rustc-env={embedded}={value}");
    }
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        // Loading the production world exceeds the default Windows main-thread stack.
        println!("cargo:rustc-link-arg-bin=alife_game_app=/STACK:16777216");
    }
}
