use alife_game_app::{
    compare_visible_world_to_headless, load_visible_world_from_p34_save,
    run_headless_app_shell_smoke, validate_app_shell_config, AppShellLaunchConfig,
};
use alife_world::persistence::BackendSelection;
use alife_world::WorldObjectKind;
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

#[test]
fn visible_world_signature_matches_restored_headless_fixture_objects() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let presentation = load_visible_world_from_p34_save(&launch).unwrap();
    compare_visible_world_to_headless(&presentation).unwrap();
    assert_eq!(presentation.object_count, 2);
    assert_eq!(presentation.kind_count(WorldObjectKind::Agent), 1);
    assert_eq!(presentation.kind_count(WorldObjectKind::Food), 1);
    assert_eq!(presentation.stable_ids()[0].raw(), 1);
    assert_eq!(presentation.stable_ids()[1].raw(), 2);
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

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_spawns_visible_world_with_adapter_local_stable_mapping() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let (mut app, summary) = alife_game_app::bevy_shell::build_visible_world_app_shell(&launch)
        .expect("visible world should load from the committed P34 fixture");
    assert_eq!(summary.object_count, 2);
    assert_eq!(summary.stable_map_count, 2);

    let mut visible_query = app
        .world_mut()
        .query::<&alife_game_app::bevy_shell::VisibleWorldObject>();
    let visible = visible_query.iter(app.world()).collect::<Vec<_>>();
    assert_eq!(visible.len(), 2);

    let map = app.world().resource::<alife_bevy_adapter::BevyEntityMap>();
    for object in visible {
        assert!(map.bevy_entity(object.stable_id).is_some());
    }
}
