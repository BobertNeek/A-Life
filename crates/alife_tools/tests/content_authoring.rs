use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use alife_core::ArchitectureMigrationError;
use alife_tools::g16_content_authoring::{
    validate_content_pack, validate_creature_preset, validate_creature_preset_file,
    validate_lesson_pack, validate_lesson_pack_file, validate_world_preset,
    validate_world_preset_file, ContentAuthoringError, ContentPackManifest,
    G16_MAX_CONTENT_FILE_BYTES,
};
use alife_world::persistence::{AssetManifest, PersistenceError};

#[path = "common/current_persistence.rs"]
mod current_persistence;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn content_manifest() -> PathBuf {
    workspace_root().join("content/fixtures/g16/content_pack_manifest.json")
}

fn p34_asset_manifest() -> AssetManifest {
    AssetManifest::from_json_file(
        workspace_root().join("crates/alife_world/tests/fixtures/p34/tiny_asset_manifest.json"),
    )
    .expect("asset manifest fixture loads")
}

#[test]
fn content_pack_manifest_validates() {
    let bundle = current_persistence::current_fixture_workspace();
    let manifest_path = bundle
        .path()
        .join("content/fixtures/g16/content_pack_manifest.json");
    let report = validate_content_pack(&manifest_path)
        .expect("content pack validates with current authoritative save");
    assert_eq!(report.pack_id, "g16-tiny-authoring-pack");
    assert_eq!(report.world_presets, 1);
    assert_eq!(report.lesson_packs, 1);
    assert_eq!(report.creature_presets, 1);
    assert_eq!(report.generated_weight_refs, 1);
    assert_eq!(report.stable_id_worlds, 1);
    assert_eq!(report.perception_only_lessons, 2);
    assert_eq!(report.valid_creature_presets, 1);
    let manifest: ContentPackManifest =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    let authored_paths = manifest
        .worlds
        .iter()
        .chain(&manifest.lessons)
        .chain(&manifest.creatures)
        .chain(&manifest.scenario_packs)
        .map(|entry| &entry.relative_path)
        .chain(
            manifest
                .generated_weight_refs
                .iter()
                .chain(&manifest.semantic_assets)
                .map(|entry| &entry.relative_path),
        )
        .map(|relative| bundle.path().join(relative))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    for path in &authored_paths {
        assert!(fs::metadata(path).unwrap().len() < G16_MAX_CONTENT_FILE_BYTES);
    }

    // The report also measures P34 dependencies. A full authoritative organism
    // save is not a G16 authored-content file, so verify its accounting separately.
    let mut checked_paths = authored_paths;
    checked_paths.extend([
        manifest_path.clone(),
        bundle.path().join(&manifest.p34_runtime_config),
        bundle.path().join(&manifest.p34_asset_manifest),
        bundle.path().join(&manifest.p34_save),
    ]);
    assert_eq!(report.checked_files, checked_paths.len());
    assert_eq!(
        report.largest_file_bytes,
        checked_paths
            .iter()
            .map(|path| fs::metadata(path).unwrap().len())
            .max()
            .unwrap()
    );

    let world_path = bundle.path().join(&manifest.worlds[0].relative_path);
    let oversized_bytes = vec![b' '; G16_MAX_CONTENT_FILE_BYTES as usize + 1];
    fs::write(&world_path, &oversized_bytes).unwrap();
    assert!(matches!(
        validate_content_pack(&manifest_path).unwrap_err(),
        ContentAuthoringError::OversizedContent { path, bytes }
            if path == world_path && bytes == oversized_bytes.len() as u64
    ));
}

#[test]
fn legacy_content_pack_rejects_missing_authoritative_graph() {
    let error = validate_content_pack(content_manifest()).unwrap_err();
    assert!(matches!(
        error,
        ContentAuthoringError::Persistence(PersistenceError::ArchitectureMigration(
            ArchitectureMigrationError::MissingAuthoritativeState {
                field: "genetic_biochemical_graph",
            }
        ))
    ));
}

#[test]
fn missing_required_content_rejects() {
    let bundle = current_persistence::current_fixture_workspace();
    let manifest_path = bundle
        .path()
        .join("content/fixtures/g16/content_pack_manifest.json");
    let manifest = fs::read_to_string(&manifest_path).expect("manifest fixture");
    let broken = manifest.replace(
        "content/fixtures/g16/worlds/tiny_meadow_world.json",
        "content/fixtures/g16/worlds/missing_world.json",
    );
    let path = manifest_path;
    fs::write(&path, broken).expect("write broken manifest");

    let err = validate_content_pack(&path).expect_err("missing required content rejects");
    assert!(
        matches!(err, ContentAuthoringError::MissingContent { .. }),
        "unexpected error: {err}"
    );
    let _ = fs::remove_file(path);
}

#[test]
fn lesson_pack_remains_perception_only() {
    let lesson_path =
        workspace_root().join("content/fixtures/g16/lessons/grounded_food_lesson.json");
    let lesson = validate_lesson_pack_file(&lesson_path).expect("lesson pack validates");
    assert!(lesson.steps.iter().all(|step| step.perception_only));
    assert!(lesson.steps.iter().all(|step| !step.direct_motor_bypass));
    assert!(lesson
        .steps
        .iter()
        .all(|step| !step.hidden_vector_injection));

    let mut bad_lesson = lesson;
    bad_lesson.steps[0].direct_motor_bypass = true;
    let err = validate_lesson_pack(&bad_lesson).expect_err("motor bypass rejects");
    assert!(
        matches!(err, ContentAuthoringError::InvalidContent { field, .. } if field == "steps.perception_boundary"),
        "unexpected error: {err}"
    );
}

#[test]
fn creature_preset_genome_valid_and_birth_weight_only() {
    let asset_manifest = p34_asset_manifest();
    let creature_path = workspace_root().join("content/fixtures/g16/creatures/nano_forager.json");
    let creature =
        validate_creature_preset_file(&creature_path, &asset_manifest).expect("creature validates");
    assert_eq!(creature.preset_id, "nano-forager");
    assert!(creature.inherited_weight_only);
    assert!(!creature.lifetime_state_included);
    assert_eq!(creature.generated_weight_asset_id, "tiny-generated-weights");
}

#[test]
fn world_and_creature_identity_metadata_is_unambiguous() {
    let world_path = workspace_root().join("content/fixtures/g16/worlds/tiny_meadow_world.json");
    let mut world = validate_world_preset_file(&world_path).unwrap();
    world.objects[1].organism_id = Some(99);
    assert!(validate_world_preset(&world).is_err());

    let mut world = validate_world_preset_file(&world_path).unwrap();
    world.terrain_zones.push(world.terrain_zones[0].clone());
    assert!(validate_world_preset(&world).is_err());

    let asset_manifest = p34_asset_manifest();
    let creature_path = workspace_root().join("content/fixtures/g16/creatures/nano_forager.json");
    let mut creature = validate_creature_preset_file(&creature_path, &asset_manifest).unwrap();
    creature.role_tags.clear();
    assert!(validate_creature_preset(&creature, &asset_manifest).is_err());
}

#[test]
fn asset_reference_path_must_match_the_p34_manifest_entry() {
    let bundle = current_persistence::current_fixture_workspace();
    let manifest_path = bundle
        .path()
        .join("content/fixtures/g16/content_pack_manifest.json");
    let manifest = fs::read_to_string(&manifest_path).unwrap();
    let broken = manifest.replace(
        "crates/alife_world/tests/fixtures/p34/assets/tiny_generated_weights_ref.json",
        "content/fixtures/g16/semantic/fake_provider_context.json",
    );
    let path = manifest_path;
    fs::write(&path, broken).unwrap();
    let error = validate_content_pack(&path).unwrap_err();
    assert!(matches!(
        error,
        ContentAuthoringError::InvalidContent {
            field: "asset_ref.relative_path",
            ..
        }
    ));
    let _ = fs::remove_file(path);
}

#[test]
fn committed_content_fixtures_are_small() {
    let root = workspace_root().join("content/fixtures/g16");
    let mut largest = 0;
    let mut visited = 0;
    visit_files(&root, &mut |path| {
        let bytes = fs::metadata(path).expect("metadata").len();
        largest = largest.max(bytes);
        visited += 1;
        assert!(
            bytes < G16_MAX_CONTENT_FILE_BYTES,
            "{} is too large for committed G16 fixtures",
            path.display()
        );
    });
    assert!(visited >= 5);
    assert!(largest < G16_MAX_CONTENT_FILE_BYTES);
}

#[test]
fn content_authoring_cli_validates_pack() {
    let bundle = current_persistence::current_fixture_workspace();
    let binary =
        std::env::var("CARGO_BIN_EXE_g16_content_authoring").expect("g16 validator binary path");
    let output = Command::new(binary)
        .arg("validate-pack")
        .arg(
            bundle
                .path()
                .join("content/fixtures/g16/content_pack_manifest.json"),
        )
        .output()
        .expect("run g16 content authoring validator");
    assert!(
        output.status.success(),
        "validator failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("G16 content pack g16-tiny-authoring-pack"));
}

fn visit_files(root: &Path, on_file: &mut impl FnMut(&Path)) {
    for entry in fs::read_dir(root).expect("read fixture dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            visit_files(&path, on_file);
        } else {
            on_file(&path);
        }
    }
}
