use std::{
    path::{Path, PathBuf},
    process::Command,
};

use alife_game_app::{
    default_environment_manifest_path, select_environment_scenario, ProductionFrontendProfileId,
};

#[cfg(feature = "gpu-runtime")]
use alife_game_app::{
    run_production_voxel_frontend_dry_run, ProductionAppState, ProductionVoxelLaunchConfig,
    FVR01_PRODUCTION_FRONTEND_SCHEMA,
};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("alife_game_app should live under crates/")
        .to_path_buf()
}

#[test]
fn fvr01_profile_registry_exposes_minimum_default_and_scale_up_profiles() {
    let labels = ProductionFrontendProfileId::all()
        .iter()
        .map(|profile| profile.label())
        .collect::<Vec<_>>();
    assert_eq!(
        labels,
        vec![
            "MinimumSettings30x30",
            "MinSpecComfort1080p",
            "Balanced1080p",
            "HighSpecScaleUp",
            "ResearchScale"
        ]
    );
    assert_eq!(
        ProductionFrontendProfileId::default(),
        ProductionFrontendProfileId::MinSpecComfort1080p
    );

    let minimum = ProductionFrontendProfileId::MinimumSettings30x30.budget();
    assert_eq!(minimum.default_population, 30);
    assert_eq!(minimum.target_fps, 30);
    assert_eq!(minimum.output_resolution, (1920, 1080));
    assert_eq!(minimum.chunk_activation_radius, 2);
    assert_eq!(minimum.active_chunk_cap, 128);
    assert_eq!(minimum.hot_brain_slots, 4);
    assert_eq!(minimum.warm_brain_slots, 12);
    assert_eq!(minimum.cold_brain_slots, 14);
    assert!((minimum.internal_render_scale_floor - 0.67).abs() < f32::EPSILON);
    assert!(minimum.hard_floor);
    assert_eq!(minimum.renderer_profile, "voxel-backend");

    let comfort = ProductionFrontendProfileId::MinSpecComfort1080p.budget();
    assert_eq!(comfort.default_population, 30);
    assert_eq!(comfort.target_fps, 60);
    assert_eq!(comfort.output_resolution, (1920, 1080));
    assert_eq!(comfort.chunk_activation_radius, 4);
    assert_eq!(comfort.active_chunk_cap, 256);
    assert_eq!(comfort.hot_brain_slots, 8);
    assert_eq!(comfort.warm_brain_slots, 16);
    assert!(comfort.comfort_default);

    let balanced = ProductionFrontendProfileId::Balanced1080p.budget();
    assert_eq!(balanced.default_population, 50);
    assert_eq!(balanced.chunk_activation_radius, 5);
    assert_eq!(balanced.active_chunk_cap, 384);

    let high = ProductionFrontendProfileId::HighSpecScaleUp.budget();
    assert_eq!(high.default_population, 100);
    assert!(high.maximum_profile_population >= 500);
    assert!(high.default_population > minimum.default_population);

    let research = ProductionFrontendProfileId::ResearchScale.budget();
    assert!(research.default_population >= 250);
    assert!(research.maximum_profile_population >= 500);
    assert!(!research.comfort_default);
}

#[test]
fn fvr01_default_environment_selects_production_voxel_not_alpha() {
    let manifest_path = default_environment_manifest_path();
    let selection = select_environment_scenario(&manifest_path, None).unwrap();
    assert_eq!(selection.entry.id, "production-voxel");
    assert_eq!(selection.entry.title, "A-Life Voxel Frontend");
    assert!(selection.entry.player_visible);
    assert!(selection.entry.tags.iter().any(|tag| tag == "production"));
    assert!(selection.entry.tags.iter().any(|tag| tag == "voxel"));
    assert!(!selection.entry.tags.iter().any(|tag| tag == "alpha"));
}

#[test]
#[cfg(feature = "gpu-runtime")]
fn fvr01_dry_run_uses_real_save_and_production_state_pipeline() {
    let launch =
        ProductionVoxelLaunchConfig::default_from_manifest(default_environment_manifest_path())
            .unwrap();
    let summary = run_production_voxel_frontend_dry_run(&launch).unwrap();
    assert_eq!(summary.schema, FVR01_PRODUCTION_FRONTEND_SCHEMA);
    assert_eq!(
        summary.profile_id,
        ProductionFrontendProfileId::MinSpecComfort1080p
    );
    assert_eq!(
        summary.state_trace,
        vec![
            ProductionAppState::Boot,
            ProductionAppState::ValidateRuntime,
            ProductionAppState::LoadAssets,
            ProductionAppState::LoadOrCreateWorld,
            ProductionAppState::Running,
            ProductionAppState::Shutdown,
        ]
    );
    assert_eq!(
        summary.state_labels(),
        vec![
            "Boot",
            "ValidateRuntime",
            "LoadAssets",
            "LoadOrCreateWorld",
            "Running",
            "Shutdown"
        ]
    );
    assert_eq!(summary.window_title, "A-Life Voxel Frontend");
    assert_eq!(summary.renderer_profile, "voxel-backend");
    assert!(summary.real_save_loaded);
    assert!(!summary.mock_data_source);
    assert_eq!(
        summary.save_metadata.selected_profile,
        "MinSpecComfort1080p"
    );
    assert_eq!(summary.save_metadata.profile_budget_version, 1);
    assert_eq!(
        summary.save_path.file_name().and_then(|name| name.to_str()),
        Some("tiny_save.json")
    );
    assert_eq!(
        summary
            .asset_manifest_path
            .file_name()
            .and_then(|name| name.to_str()),
        Some("tiny_asset_manifest.json")
    );
    assert!(!summary.diagnostics.selected_backend.is_empty());
    assert!(!summary.diagnostics.renderer_profile.is_empty());
    assert!(!summary.diagnostics.save_path.as_os_str().is_empty());
    assert!(!summary
        .diagnostics
        .asset_manifest_path
        .as_os_str()
        .is_empty());
}

#[test]
#[cfg(feature = "gpu-runtime")]
fn fvr01_minimum_profile_is_available_as_hard_fallback_floor() {
    let mut launch =
        ProductionVoxelLaunchConfig::default_from_manifest(default_environment_manifest_path())
            .unwrap();
    launch.profile_id = ProductionFrontendProfileId::MinimumSettings30x30;
    launch.population = Some(30);
    let summary = run_production_voxel_frontend_dry_run(&launch).unwrap();
    assert_eq!(
        summary.profile_id,
        ProductionFrontendProfileId::MinimumSettings30x30
    );
    assert_eq!(summary.effective_population, 30);
    assert_eq!(summary.profile_budget.target_fps, 30);
    assert_eq!(summary.profile_budget.active_chunk_cap, 128);
    assert_eq!(
        summary.save_metadata.selected_profile,
        "MinimumSettings30x30"
    );
    assert!(summary.profile_budget.hard_floor);
}

#[test]
fn fvr01_cargo_manifest_pins_bevy_018_voxel_stack() {
    let cargo =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).unwrap();
    for required in [
        "bevy_voxel_world = { version = \"=0.16.0\"",
        "block-mesh = { version = \"=0.2.0\"",
        "bevy_sprite3d = { version = \"=8.0.0\"",
        "bevy_asset_loader = { version = \"=0.26.0\"",
        "bevy_hanabi = { version = \"=0.18.0\"",
        "bevy_egui = { version = \"=0.39.0\"",
        "bevy-inspector-egui = { version = \"=0.36.0\"",
        "voxel-backend =",
        "production-voxel-frontend =",
        "debug-tools =",
        "licensed-assets =",
        "vfx-hanabi =",
        "presentation-physics =",
        "creature-sprites =",
        "bevy/bevy_picking",
    ] {
        assert!(cargo.contains(required), "Cargo.toml missing {required}");
    }
    for rejected in [
        "bevy_voxel_world = { version = \"0.17",
        "bevy_sprite3d = { version = \"9.",
        "bevy_asset_loader = { version = \"0.27",
        "bevy_hanabi = { version = \"0.19",
        "bevy_egui = { version = \"0.40",
        "bevy-inspector-egui = { version = \"0.37",
    ] {
        assert!(
            !cargo.contains(rejected),
            "Cargo.toml includes rejected {rejected}"
        );
    }
}

#[test]
fn fvr01_cli_help_names_production_command_and_profiles() {
    let output = Command::new(env!("CARGO_BIN_EXE_alife_game_app"))
        .args(["production-voxel", "--help"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "production-voxel --help failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("production-voxel"));
    assert!(stdout.contains("MinimumSettings30x30"));
    assert!(stdout.contains("MinSpecComfort1080p"));
    assert!(stdout.contains("Balanced1080p"));
    assert!(stdout.contains("HighSpecScaleUp"));
    assert!(stdout.contains("ResearchScale"));
    assert!(!stdout.contains("A-Life GPU Alpha Playground"));
}

#[test]
#[cfg(feature = "gpu-runtime")]
fn fvr01_legacy_graphical_command_is_alias_not_product_path() {
    let output = Command::new(env!("CARGO_BIN_EXE_alife_game_app"))
        .args(["graphical-playground", "--dry-run"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "legacy graphical alias failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("legacy_alias=true"));
    assert!(stdout.contains("routed_to=production-voxel"));
    assert!(stdout.contains("profile=MinSpecComfort1080p"));
    assert!(!stdout.contains("requires feature `bevy-app`"));
}

#[test]
fn fvr01_windows_scripts_default_to_production_voxel_frontend() {
    let root = workspace_root();
    let production =
        std::fs::read_to_string(root.join("scripts/run_production_voxel_frontend.ps1")).unwrap();
    assert!(production.contains("production-voxel"));
    assert!(production.contains("MinSpecComfort1080p"));
    assert!(production.contains("bevy-app gpu-runtime voxel-backend"));
    assert!(production.contains("A-Life Voxel Frontend"));
    assert!(production.contains("-DryRun"));

    let legacy =
        std::fs::read_to_string(root.join("scripts/run_graphical_playground.ps1")).unwrap();
    assert!(legacy.contains("FVR01 compatibility alias"));
    assert!(legacy.contains("run_production_voxel_frontend.ps1"));
    assert!(!legacy.contains("Starting A-Life GPU Alpha Playground"));
}
