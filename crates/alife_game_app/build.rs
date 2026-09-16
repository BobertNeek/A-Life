fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        // Loading the production world exceeds the default Windows main-thread stack.
        println!("cargo:rustc-link-arg-bin=alife_game_app=/STACK:16777216");
    }
}
