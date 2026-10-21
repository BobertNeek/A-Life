use crate::prelude::*;

use super::*;

#[path = "test_fixtures.rs"]
pub(crate) mod fixtures;

struct CurrentAppFixtures {
    root: PathBuf,
}

impl CurrentAppFixtures {
    fn new() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let root = ca12_workspace_root().join("target").join(format!(
            "app-test-fixtures-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        for name in ["p34", "production_voxel"] {
            let relative = PathBuf::from("crates/alife_world/tests/fixtures").join(name);
            let source = ca12_workspace_root().join(&relative);
            let destination = root.join(&relative);
            copy_test_fixture_tree(&source, &destination);
            let population = if name == "p34" { 1 } else { 3 };
            let save = fixtures::current_scene_save(&source, population);
            std::fs::write(
                destination.join("tiny_save.json"),
                serde_json::to_vec(&save).unwrap(),
            )
            .unwrap();
        }
        let mut environment: serde_json::Value =
            serde_json::from_slice(&std::fs::read(default_environment_manifest_path()).unwrap())
                .unwrap();
        environment["scenarios"][0]["fixture_root"] =
            serde_json::json!(root.join("crates/alife_world/tests/fixtures/production_voxel"));
        std::fs::write(
            root.join("environment_manifest.json"),
            serde_json::to_vec_pretty(&environment).unwrap(),
        )
        .unwrap();
        let mut bundle: serde_json::Value =
            serde_json::from_slice(&std::fs::read(default_app_bundle_manifest_path()).unwrap())
                .unwrap();
        bundle["environment_manifest"] = serde_json::json!(root
            .join("environment_manifest.json")
            .strip_prefix(ca12_workspace_root())
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/"));
        for entry in bundle["entries"].as_array_mut().unwrap() {
            let relative = entry["relative_path"].as_str().unwrap();
            entry["relative_path"] = serde_json::json!(root
                .join(relative)
                .strip_prefix(ca12_workspace_root())
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/"));
        }
        std::fs::write(
            root.join("app_bundle_manifest.json"),
            serde_json::to_vec_pretty(&bundle).unwrap(),
        )
        .unwrap();
        Self { root }
    }

    fn p34_launch(&self) -> AppShellLaunchConfig {
        AppShellLaunchConfig::from_p34_fixture_root(
            self.root.join("crates/alife_world/tests/fixtures/p34"),
        )
    }

    fn environment_manifest(&self) -> PathBuf {
        self.root.join("environment_manifest.json")
    }

    fn bundle_manifest(&self) -> PathBuf {
        self.root.join("app_bundle_manifest.json")
    }
}

impl Drop for CurrentAppFixtures {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}

fn copy_test_fixture_tree(source: &Path, destination: &Path) {
    std::fs::create_dir_all(destination).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let destination = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_test_fixture_tree(&entry.path(), &destination);
        } else {
            std::fs::copy(entry.path(), destination).unwrap();
        }
    }
}

fn p34_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../alife_world/tests/fixtures/p34")
}

fn path_ends_with(path: &Path, suffix: &str) -> bool {
    path.to_string_lossy().replace('\\', "/").ends_with(suffix)
}

#[test]
fn ca10_environment_manifest_validates_and_selects_default_production_voxel() {
    let manifest_path = default_environment_manifest_path();
    let manifest = EnvironmentManifest::from_json_file(&manifest_path).unwrap();
    manifest.validate(&manifest_path).unwrap();
    assert_eq!(manifest.schema, CA10_ENVIRONMENT_MANIFEST_SCHEMA);
    assert_eq!(
        manifest.schema_version,
        CA10_ENVIRONMENT_MANIFEST_SCHEMA_VERSION
    );
    assert_eq!(manifest.default_scenario_id, "production-voxel");
    assert_eq!(manifest.scenario_ids(), vec!["production-voxel"]);

    let fixtures = CurrentAppFixtures::new();
    let selection = select_environment_scenario(fixtures.environment_manifest(), None).unwrap();
    assert_eq!(selection.entry.id, "production-voxel");
    assert!(path_ends_with(
        &selection.launch.fixture_root,
        "crates/alife_world/tests/fixtures/production_voxel"
    ));
    let visible = load_visible_world_from_p34_save(&selection.launch).unwrap();
    assert_eq!(visible.seed, 4242);
    assert_eq!(visible.object_count, 12);
    assert_eq!(visible.kind_count(WorldObjectKind::Agent), 3);
    assert_eq!(visible.kind_count(WorldObjectKind::Food), 3);
    assert_eq!(visible.kind_count(WorldObjectKind::Hazard), 3);
    assert_eq!(visible.kind_count(WorldObjectKind::Obstacle), 3);
}

#[test]
fn ca10_environment_manifest_reports_known_scenarios_for_bad_selection() {
    let manifest_path = default_environment_manifest_path();
    let err = select_environment_scenario(&manifest_path, Some("missing-arena"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("unknown environment scenario 'missing-arena'"));
    assert!(err.contains("Known scenarios: production-voxel"));
}

#[test]
fn ca12_app_bundle_manifest_discovers_production_assets_and_shaders() {
    let fixtures = CurrentAppFixtures::new();
    let summary = validate_app_bundle_manifest(fixtures.bundle_manifest()).unwrap();
    assert_eq!(summary.schema, CA12_APP_BUNDLE_MANIFEST_SCHEMA);
    assert_eq!(
        summary.schema_version,
        CA12_APP_BUNDLE_MANIFEST_SCHEMA_VERSION
    );
    assert_eq!(summary.environment_scenarios, 1);
    assert_eq!(summary.shader_assets, summary.discovered_shader_assets);
    assert!(summary.production_voxel_asset_entries > 0);
    assert!(summary.production_voxel_generated_assets > 0);
    assert!(summary.shader_discovery_complete);
    summary.validate().unwrap();
}

#[test]
fn ca12_app_bundle_manifest_rejects_missing_required_entries() {
    let source = std::fs::read_to_string(default_app_bundle_manifest_path()).unwrap();
    let broken = source.replace(
        "crates/alife_world/tests/fixtures/production_voxel/tiny_config.json",
        "crates/alife_world/tests/fixtures/production_voxel/missing_config.json",
    );
    let path = std::env::temp_dir().join("alife_ca12_broken_bundle_manifest.json");
    std::fs::write(&path, broken).unwrap();
    let err = validate_app_bundle_manifest(&path).unwrap_err().to_string();
    assert!(err.contains("required app bundle entry is missing"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn ca12_app_bundle_manifest_rejects_missing_shader_assets() {
    let fixtures = CurrentAppFixtures::new();
    let mut broken: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixtures.bundle_manifest()).unwrap()).unwrap();
    // A valid config/asset bundle isolates the missing-shader contract. The
    // full-bundle test separately covers authoritative portable-save entries.
    broken["entries"]
        .as_array_mut()
        .unwrap()
        .retain(|entry| entry["kind"] != "portable-save");
    let shader = broken["shader_assets"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|shader| {
            shader["relative_path"] == "crates/alife_gpu_backend/shaders/closed_loop_recurrent.wgsl"
        })
        .unwrap();
    shader["relative_path"] =
        serde_json::json!("crates/alife_gpu_backend/shaders/missing_recurrent.wgsl");
    let path = std::env::temp_dir().join("alife_ca12_broken_shader_manifest.json");
    std::fs::write(&path, serde_json::to_vec_pretty(&broken).unwrap()).unwrap();
    let err = validate_app_bundle_manifest(&path).unwrap_err().to_string();
    assert!(err.contains("required shader asset is missing"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn invalid_config_rejects_with_p34_diagnostics() {
    let mut launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    launch.config_path = launch.fixture_root.join("missing_config.json");
    let err = validate_app_shell_config(&launch).unwrap_err().to_string();
    assert!(err.contains("persistence/config error") || err.contains("io error"));
}

#[test]
fn visible_world_loads_actual_saved_objects_without_bevy() {
    let fixtures = CurrentAppFixtures::new();
    let launch = fixtures.p34_launch();
    let presentation = load_visible_world_from_p34_save(&launch).unwrap();
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
        .objects
        .iter()
        .any(|object| object.kind == WorldObjectKind::Food && object.label == "berry"));
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
