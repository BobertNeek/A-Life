use alife_game_app::{
    run_headless_app_shell_smoke, validate_app_shell_config, AppShellLaunchConfig,
};
use alife_world::persistence::BackendSelection;
use std::path::PathBuf;

fn p34_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates/alife_world/tests/fixtures/p34")
}

#[test]
fn ci_headless_app_shell_smoke_uses_p34_config_without_graphics() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let summary = run_headless_app_shell_smoke(&launch).unwrap();
    assert_eq!(summary.seed, 4242);
    assert_eq!(summary.requested_backend, BackendSelection::CpuReference);
    assert!(!summary.gpu_backend_enabled);
    assert!(!summary.semantic_enabled);
    assert!(!summary.school_enabled);
    assert!(summary.logging_enabled);
    assert!(!summary.graphics_required_for_default_path);
}

#[test]
fn explicit_config_validation_rejects_missing_required_manifest() {
    let mut launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    launch.asset_manifest_path = launch.fixture_root.join("missing_asset_manifest.json");
    assert!(validate_app_shell_config(&launch).is_err());
}

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_can_construct_shell_without_visible_world_content() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let summary = run_headless_app_shell_smoke(&launch).unwrap();
    let mut app = alife_game_app::bevy_shell::build_minimal_bevy_app_shell(summary);
    app.update();
    assert!(app
        .world()
        .get_resource::<alife_bevy_adapter::AdapterWorldTick>()
        .is_some());
}
