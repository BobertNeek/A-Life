use crate::prelude::*;

use super::*;

fn p34_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../alife_world/tests/fixtures/p34")
}

fn path_ends_with(path: &Path, suffix: &str) -> bool {
    path.to_string_lossy().replace('\\', "/").ends_with(suffix)
}

#[test]
fn headless_app_shell_loads_p34_config_and_manifest() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let summary = run_headless_app_shell_smoke(&launch).unwrap();
    assert_eq!(summary.schema, G01_APP_SHELL_SCHEMA);
    assert_eq!(summary.schema_version, G01_APP_SHELL_SCHEMA_VERSION);
    assert_eq!(summary.seed, 4242);
    assert_eq!(summary.brain_class, "Nano512");
    assert_eq!(summary.requested_backend, BackendSelection::CpuReference);
    assert_eq!(summary.asset_count, 2);
    assert!(!summary.graphics_required_for_default_path);
    assert_eq!(
        summary.state_labels(),
        vec!["Boot", "LoadConfig", "DevMenu", "Running", "Shutdown"]
    );
}

#[test]
fn ca10_environment_manifest_validates_and_selects_default_gpu_alpha() {
    let manifest_path = default_environment_manifest_path();
    let manifest = EnvironmentManifest::from_json_file(&manifest_path).unwrap();
    manifest.validate(&manifest_path).unwrap();
    assert_eq!(manifest.schema, CA10_ENVIRONMENT_MANIFEST_SCHEMA);
    assert_eq!(
        manifest.schema_version,
        CA10_ENVIRONMENT_MANIFEST_SCHEMA_VERSION
    );
    assert_eq!(manifest.default_scenario_id, "gpu-alpha");
    assert_eq!(manifest.scenario_ids(), vec!["gpu-alpha", "p34"]);

    let summary = run_environment_launcher_smoke(&manifest_path, None).unwrap();
    assert_eq!(summary.schema, CA10_ENVIRONMENT_MANIFEST_SCHEMA);
    assert_eq!(summary.selected_scenario_id, "gpu-alpha");
    assert_eq!(summary.scenario_count, 2);
    assert!(path_ends_with(
        &summary.fixture_root,
        "crates/alife_world/tests/fixtures/gpu_alpha"
    ));
    assert_eq!(summary.seed, 4242);
    assert_eq!(summary.object_count, 4);
    assert_eq!(summary.creature_count, 1);
    assert_eq!(summary.food_count, 1);
    assert_eq!(summary.hazard_count, 1);
    assert_eq!(summary.obstacle_count, 1);
    assert!(summary
        .player_visible_error_sample
        .contains("Unknown scenario"));
}

#[test]
fn ca10_environment_manifest_can_select_legacy_p34_fixture() {
    let manifest_path = default_environment_manifest_path();
    let summary = run_environment_launcher_smoke(&manifest_path, Some("p34")).unwrap();
    assert_eq!(summary.selected_scenario_id, "p34");
    assert!(path_ends_with(
        &summary.fixture_root,
        "crates/alife_world/tests/fixtures/p34"
    ));
    assert_eq!(summary.object_count, 2);
    assert_eq!(summary.creature_count, 1);
    assert_eq!(summary.food_count, 1);
    assert_eq!(summary.hazard_count, 0);
    assert_eq!(summary.obstacle_count, 0);
}

#[test]
fn ca10_environment_manifest_reports_known_scenarios_for_bad_selection() {
    let manifest_path = default_environment_manifest_path();
    let err = select_environment_scenario(&manifest_path, Some("missing-arena"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("unknown environment scenario 'missing-arena'"));
    assert!(err.contains("Known scenarios: gpu-alpha, p34"));
}

#[test]
fn ca11_player_sandbox_editor_edits_default_manifest_scenario() {
    let manifest_path = default_environment_manifest_path();
    let summary = run_player_sandbox_editor_smoke(&manifest_path, None, None).unwrap();
    assert_eq!(summary.schema, CA11_PLAYER_SANDBOX_EDITOR_SCHEMA);
    assert_eq!(
        summary.schema_version,
        CA11_PLAYER_SANDBOX_EDITOR_SCHEMA_VERSION
    );
    assert_eq!(summary.scenario_id, "gpu-alpha");
    assert_eq!(summary.initial_object_count, 4);
    assert!(summary.final_object_count > summary.initial_object_count);
    assert!(summary.placed_food && summary.removed_food);
    assert!(summary.placed_hazard && summary.removed_hazard);
    assert!(summary.placed_obstacle && summary.removed_obstacle);
    assert!(summary.edit_mode_required);
    assert!(!summary.output_written);
    assert!(summary
        .player_status_lines
        .iter()
        .any(|line| line.contains("portable stable-ID save")));
    summary.validate().unwrap();
}

#[test]
fn ca11_player_sandbox_editor_can_select_legacy_p34_scenario() {
    let manifest_path = default_environment_manifest_path();
    let summary = run_player_sandbox_editor_smoke(&manifest_path, Some("p34"), None).unwrap();
    assert_eq!(summary.scenario_id, "p34");
    assert_eq!(summary.initial_object_count, 2);
    assert!(summary.final_object_count > summary.initial_object_count);
    assert!(summary.stable_ids.iter().all(|id| id.raw() > 0));
    summary.validate().unwrap();
}

#[test]
fn ca11_player_sandbox_editor_can_write_optional_save_output() {
    let manifest_path = default_environment_manifest_path();
    let output = std::env::temp_dir().join("alife_ca11_player_sandbox_editor_save.json");
    let _ = std::fs::remove_file(&output);
    let summary =
        run_player_sandbox_editor_smoke(&manifest_path, Some("gpu-alpha"), Some(&output)).unwrap();
    assert!(summary.output_written);
    assert!(output.exists());
    let saved = PortableSaveFile::from_json_file(&output).unwrap();
    let restored = saved.restore_headless_world().unwrap();
    assert_eq!(restored.object_count(), summary.final_object_count);
    let _ = std::fs::remove_file(&output);
}

#[test]
fn paused_state_path_is_explicit_and_deterministic() {
    let mut launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    launch.start_paused = true;
    let summary = run_headless_app_shell_smoke(&launch).unwrap();
    assert_eq!(
        summary.state_labels(),
        vec![
            "Boot",
            "LoadConfig",
            "DevMenu",
            "Running",
            "Paused",
            "Running",
            "Shutdown"
        ]
    );
}

#[test]
fn invalid_config_rejects_with_p34_diagnostics() {
    let mut launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    launch.config_path = launch.fixture_root.join("missing_config.json");
    let err = validate_app_shell_config(&launch).unwrap_err().to_string();
    assert!(err.contains("persistence/config error") || err.contains("io error"));
}

#[test]
fn invalid_state_transition_is_rejected() {
    let mut trace = AppShellStateTrace::default();
    let err = trace.transition(GameAppState::Running).unwrap_err();
    assert!(matches!(
        err,
        GameAppShellError::InvalidTransition {
            from: GameAppState::Boot,
            to: GameAppState::Running
        }
    ));
}

#[test]
fn visible_world_signature_loads_from_p34_save_without_bevy() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let presentation = load_visible_world_from_p34_save(&launch).unwrap();
    compare_visible_world_to_headless(&presentation).unwrap();
    assert_eq!(presentation.schema, G02_VISIBLE_WORLD_SCHEMA);
    assert_eq!(
        presentation.schema_version,
        G02_VISIBLE_WORLD_SCHEMA_VERSION
    );
    assert_eq!(presentation.seed, 4242);
    assert_eq!(presentation.object_count, 2);
    assert_eq!(presentation.kind_count(WorldObjectKind::Agent), 1);
    assert_eq!(presentation.kind_count(WorldObjectKind::Food), 1);
    assert!(presentation
        .visible_signature
        .iter()
        .any(|line| line.contains("Food:berry")));
}

#[test]
fn s01_graphical_playground_launch_plan_validates_without_graphics() {
    let launch = GraphicalPlaygroundLaunchConfig::interactive(p34_fixture_root());
    let summary = validate_graphical_playground_launch(&launch).unwrap();

    assert_eq!(summary.schema, S01_GRAPHICAL_PLAYGROUND_SCHEMA);
    assert_eq!(
        summary.schema_version,
        S01_GRAPHICAL_PLAYGROUND_SCHEMA_VERSION
    );
    assert_eq!(summary.window_title, S01_GRAPHICAL_WINDOW_TITLE);
    assert_eq!(summary.mode_label, "interactive");
    assert!(summary.persistent_window);
    assert_eq!(summary.seed, 4242);
    assert_eq!(summary.selected_backend, BackendSelection::CpuReference);
    assert_eq!(
        summary.requested_gpu_mode,
        GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded
    );
    assert!(!summary.require_gpu);
    assert!(summary.gpu_mode_visible);
    assert!(summary.cpu_fallback_visible);
    assert!(summary.stable_id_overlay_visible);
    assert_eq!(summary.object_count, 2);
    assert_eq!(summary.creature_marker_count, 1);
    assert_eq!(summary.food_marker_count, 1);
    assert!(summary.signature_line().contains("persistent=true"));
}

#[test]
fn s01_graphical_smoke_seconds_are_bounded() {
    let ok = GraphicalPlaygroundLaunchConfig::smoke(p34_fixture_root(), 5);
    let summary = validate_graphical_playground_launch(&ok).unwrap();
    assert_eq!(summary.mode_label, "smoke-timeout");
    assert_eq!(summary.smoke_seconds, Some(5));
    assert!(!summary.persistent_window);

    let zero = GraphicalPlaygroundLaunchConfig::smoke(p34_fixture_root(), 0);
    assert!(zero.validate().is_err());
    let too_long = GraphicalPlaygroundLaunchConfig::smoke(
        p34_fixture_root(),
        S01_MAX_GRAPHICAL_SMOKE_SECONDS + 1,
    );
    assert!(too_long.validate().is_err());
}

#[test]
fn s01_graphical_launcher_script_uses_persistent_window_commands() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script =
        std::fs::read_to_string(root.join("scripts/run_graphical_playground.ps1")).unwrap();

    assert!(script.contains("[switch]$DryRun"));
    assert!(script.contains("[int]$SmokeSeconds"));
    assert!(script.contains("[string]$GpuMode"));
    assert!(script.contains("[string]$Scenario"));
    assert!(script.contains("[string]$EnvironmentManifest"));
    assert!(script.contains("[string]$GraphicsBackend"));
    assert!(script.contains("graphical-playground"));
    assert!(script.contains("--scenario"));
    assert!(script.contains("gpu-alpha"));
    assert!(script.contains("Environment manifest:"));
    assert!(script.contains("--gpu-mode"));
    assert!(script.contains("--smoke-seconds"));
    assert!(script.contains("bevy-app gpu-runtime"));
    assert!(script.contains("Format-CommandArgument"));
    assert!(script.contains("$DisplayCommand"));
    assert!(script.contains("arrows/WASD pan"));
    assert!(script.contains("Inspector is read-only"));
    assert!(script.contains("Readability:"));
    assert!(script.contains("overriding inherited WGPU_BACKEND"));
    assert!(script.contains("-GraphicsBackend vulkan"));
    assert!(!script.contains("\"visible-world-smoke\""));
    assert!(!script.contains("$ModeArgs += \"crates/alife_world/tests/fixtures/gpu_alpha\""));
}

#[test]
fn placeholder_mapping_covers_g02_required_visual_kinds() {
    assert_eq!(
        placeholder_for_kind(WorldObjectKind::Agent),
        (
            VisiblePlaceholderShape::CreatureCapsule,
            VisibleMaterialKind::Creature
        )
    );
    assert_eq!(
        placeholder_for_kind(WorldObjectKind::Food),
        (
            VisiblePlaceholderShape::FoodSphere,
            VisibleMaterialKind::Food
        )
    );
    assert_eq!(
        placeholder_for_kind(WorldObjectKind::Hazard),
        (
            VisiblePlaceholderShape::HazardCone,
            VisibleMaterialKind::Hazard
        )
    );
    assert_eq!(
        placeholder_for_kind(WorldObjectKind::Obstacle),
        (
            VisiblePlaceholderShape::ObstacleCube,
            VisibleMaterialKind::Obstacle
        )
    );
    assert_eq!(
        placeholder_for_kind(WorldObjectKind::Token),
        (
            VisiblePlaceholderShape::TokenBillboard,
            VisibleMaterialKind::Token
        )
    );
}

#[test]
fn creature_visual_mapping_is_bounded_and_readable() {
    let mut homeostasis = HomeostaticSnapshot::baseline(Tick::new(9));
    homeostasis.drives.hunger = 0.82;
    homeostasis.drives.fear = 0.20;
    homeostasis.drives.pain = 0.10;
    homeostasis.drives.curiosity = 0.55;
    homeostasis.drives.brain_atp = 0.72;
    homeostasis.hormones.sleep_pressure = 0.25;
    let visual = creature_visual_snapshot_from_parts(
        OrganismId(1),
        WorldEntityId(1),
        Vec3f::new(0.0, 0.0, 0.0),
        Some(WorldEntityId(2)),
        Some(Vec3f::new(2.0, 0.0, 0.0)),
        &homeostasis,
        SleepPhase::Awake,
        Some(ActionKind::Interact),
    )
    .unwrap();

    assert_eq!(visual.schema, G04_CREATURE_VISUAL_SCHEMA);
    assert_eq!(visual.schema_version, G04_CREATURE_VISUAL_SCHEMA_VERSION);
    assert_eq!(visual.animation, CreatureAnimationState::Interacting);
    assert_eq!(visual.expression, CreatureExpressionState::Hungry);
    assert_eq!(visual.facing, Vec3f::new(1.0, 0.0, 0.0));
    assert_eq!(visual.cues.hunger.value, 0.82);
    assert!(visual
        .base_rgba
        .iter()
        .chain(visual.accent_rgba.iter())
        .chain(visual.intent_rgba.iter())
        .all(|channel| (0.0..=1.0).contains(channel)));
    visual.validate().unwrap();
}

#[test]
fn sleep_and_pain_override_action_visual_states_without_cognitive_mutation() {
    let mut homeostasis = HomeostaticSnapshot::baseline(Tick::new(11));
    homeostasis.drives.pain = 0.80;
    let pain_visual = creature_visual_snapshot_from_parts(
        OrganismId(1),
        WorldEntityId(1),
        Vec3f::ZERO,
        None,
        None,
        &homeostasis,
        SleepPhase::Awake,
        Some(ActionKind::Move),
    )
    .unwrap();
    assert_eq!(pain_visual.animation, CreatureAnimationState::Hurt);
    assert_eq!(pain_visual.expression, CreatureExpressionState::Pained);

    let sleep_visual = creature_visual_snapshot_from_parts(
        OrganismId(1),
        WorldEntityId(1),
        Vec3f::ZERO,
        None,
        None,
        &homeostasis,
        SleepPhase::Consolidating,
        Some(ActionKind::Move),
    )
    .unwrap();
    assert_eq!(sleep_visual.animation, CreatureAnimationState::Sleeping);
    assert_eq!(sleep_visual.expression, CreatureExpressionState::Tired);
}

#[test]
fn g04_creature_visual_smoke_derives_from_g03_tick_summary() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let visual = run_creature_visual_smoke(&launch).unwrap();
    assert_eq!(visual.organism_id, OrganismId(1));
    assert_eq!(visual.stable_id, WorldEntityId(1));
    assert_eq!(visual.selected_action_kind, Some(ActionKind::Interact));
    assert_eq!(visual.target_entity, Some(WorldEntityId(2)));
    assert_eq!(visual.animation, CreatureAnimationState::Interacting);
    assert!(visual.debug_summary.contains("organism=1"));
    visual.validate().unwrap();
}

#[test]
fn g05_camera_controls_are_bounded_and_deterministic() {
    let camera = CameraNavigationState::top_down_default()
        .pan_by(2.0, -3.5)
        .unwrap()
        .zoom_by(20.0)
        .unwrap()
        .orbit_by(-45.0)
        .unwrap()
        .with_follow_target(WorldEntityId(1))
        .unwrap();

    assert_eq!(camera.zoom, 8.0);
    assert_eq!(camera.yaw_degrees, 315.0);
    assert_eq!(camera.follow_target, Some(WorldEntityId(1)));
    assert!(camera.signature_line().contains("315.00"));
    camera.validate().unwrap();
}

#[test]
fn g05_selection_uses_stable_ids_from_visible_world() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let presentation = load_visible_world_from_p34_save(&launch).unwrap();
    let selection = select_visible_world_entity(&presentation, WorldEntityId(1)).unwrap();
    assert_eq!(selection.schema, G05_CAMERA_INSPECTOR_SCHEMA);
    assert_eq!(
        selection.schema_version,
        G05_CAMERA_INSPECTOR_SCHEMA_VERSION
    );
    assert_eq!(selection.stable_id, WorldEntityId(1));
    assert_eq!(selection.organism_id, Some(OrganismId(1)));
    assert_eq!(selection.kind, WorldObjectKind::Agent);
    assert!(selection.debug_label.contains("Agent"));
    selection.validate().unwrap();

    assert!(select_visible_world_entity(&presentation, WorldEntityId(99_999)).is_err());
}

#[test]
fn g05_inspector_snapshot_is_read_only_and_covers_expected_fields() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let inspector = run_creature_inspector_smoke(&launch).unwrap();
    assert_eq!(inspector.schema, G05_CAMERA_INSPECTOR_SCHEMA);
    assert_eq!(
        inspector.schema_version,
        G05_CAMERA_INSPECTOR_SCHEMA_VERSION
    );
    assert!(inspector.read_only);
    assert_eq!(inspector.selection.stable_id, WorldEntityId(1));
    assert_eq!(inspector.camera.follow_target, Some(WorldEntityId(1)));
    assert_eq!(
        inspector.visual.selected_action_kind,
        Some(ActionKind::Interact)
    );
    assert!(inspector.action_summary.contains("Interact"));
    assert!(inspector.patch_summary.contains("sealed=true"));
    assert!(inspector
        .memory_topology_summary
        .contains("memory_updates=1"));
    assert!(inspector
        .drive_lines
        .iter()
        .any(|line| line.starts_with("hunger=")));
    assert!(inspector
        .hormone_lines
        .iter()
        .any(|line| line.starts_with("sleep_pressure=")));
    assert!(inspector
        .troubleshooting_messages
        .iter()
        .any(|line| line.contains("gpu_runtime=optional")));
    inspector.validate().unwrap();
}

#[test]
fn g05_pause_step_run_controls_map_to_live_tick_controls() {
    let paused = InspectorControlPanel::paused();
    assert_eq!(
        paused.to_live_control().unwrap(),
        LiveBrainTickControl::paused()
    );
    let step = InspectorControlPanel::step_once();
    assert_eq!(
        step.to_live_control().unwrap(),
        LiveBrainTickControl::step_once()
    );
    let run = InspectorControlPanel::run_fixed(3, 150);
    assert_eq!(
        run.to_live_control().unwrap(),
        LiveBrainTickControl::run_fixed(3)
    );
    assert!(InspectorControlPanel::run_fixed(32, 100)
        .validate()
        .is_err());
}

#[cfg(feature = "bevy-app")]
#[test]
fn feature_gated_bevy_shell_builds_with_adapter_plugin() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let summary = run_headless_app_shell_smoke(&launch).unwrap();
    let mut app = crate::bevy_shell::build_minimal_bevy_app_shell(summary);
    app.update();
    assert!(app
        .world()
        .get_resource::<alife_bevy_adapter::AdapterScheduleTrace>()
        .is_some());
}

#[cfg(feature = "bevy-app")]
#[test]
fn feature_gated_visible_world_spawns_stable_mapped_entities() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let (mut app, summary) = crate::bevy_shell::build_visible_world_app_shell(&launch).unwrap();
    assert!(summary.ground_spawned);
    assert_eq!(summary.object_count, 2);
    assert_eq!(summary.stable_map_count, 2);
    let mut visible_query = app
        .world_mut()
        .query::<&crate::bevy_shell::VisibleWorldObject>();
    let visible = visible_query.iter(app.world()).collect::<Vec<_>>();
    assert_eq!(visible.len(), 2);
    let map = app.world().resource::<alife_bevy_adapter::BevyEntityMap>();
    for object in visible {
        assert!(map.bevy_entity(object.stable_id).is_some());
    }
    let mut ground_query = app
        .world_mut()
        .query::<&crate::bevy_shell::VisibleGroundPlane>();
    assert_eq!(ground_query.iter(app.world()).count(), 1);
}
