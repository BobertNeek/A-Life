//! CPU-only bundle admission against current organism authority and approved art.

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use alife_core::ScaffoldContractError;
use alife_game_app::{
    ca12_workspace_root, default_app_bundle_manifest_path, default_environment_manifest_path,
    validate_app_bundle_manifest, GameAppShellError, CA12_MAX_BUNDLE_FILE_BYTES,
    CA12_MAX_REFERENCED_MANIFEST_BYTES, FVR07_MAX_COMMITTED_ASSET_BYTES,
};
use alife_world::persistence::{PersistenceError, PortableSaveFile, P34_MAX_INLINE_SAVE_BYTES};
use serde_json::Value;

#[path = "../src/test_fixtures.rs"]
mod fixtures;

// Historical bundle-wide threshold: retain the regression without treating it
// as current portable-save or approved-production-asset admission policy.
const LEGACY_BUNDLE_ASSET_LIMIT_BYTES: u64 = 768 * 1024;

struct CurrentBundleFixture {
    root: PathBuf,
    manifest: Value,
}

impl CurrentBundleFixture {
    fn new() -> Self {
        Self::with_production_population(3)
    }

    fn with_production_population(population: usize) -> Self {
        let workspace = ca12_workspace_root();
        let root = workspace.join("target").join(format!(
            "current-bundle-admission-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        for (name, population) in [("p34", 1), ("production_voxel", population)] {
            let relative = PathBuf::from("crates/alife_world/tests/fixtures").join(name);
            let source = workspace.join(&relative);
            let destination = root.join(&relative);
            copy_tree(&source, &destination);
            let save = fixtures::current_scene_save(&source, population);
            // Pretty file serialization is valid even when the inline-string
            // serializer's small transport bound cannot carry this authority.
            save.to_json_file(destination.join("tiny_save.json"))
                .unwrap();
        }
        let mut environment: Value =
            serde_json::from_slice(&fs::read(default_environment_manifest_path()).unwrap())
                .unwrap();
        environment["scenarios"][0]["fixture_root"] =
            serde_json::json!(root.join("crates/alife_world/tests/fixtures/production_voxel"));
        fs::write(
            root.join("environment_manifest.json"),
            serde_json::to_vec(&environment).unwrap(),
        )
        .unwrap();
        let mut manifest: Value =
            serde_json::from_slice(&fs::read(default_app_bundle_manifest_path()).unwrap()).unwrap();
        manifest["environment_manifest"] =
            serde_json::json!(workspace_relative(&root.join("environment_manifest.json")));
        for entry in manifest["entries"].as_array_mut().unwrap() {
            entry["relative_path"] = serde_json::json!(workspace_relative(
                &root.join(entry["relative_path"].as_str().unwrap())
            ));
        }
        Self { root, manifest }
    }

    fn write_manifest(&self) -> PathBuf {
        let path = self.root.join("app_bundle_manifest.json");
        fs::write(&path, serde_json::to_vec(&self.manifest).unwrap()).unwrap();
        path
    }

    fn entry_path(&self, id: &str) -> PathBuf {
        let entry = self.manifest["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["id"] == id)
            .unwrap();
        ca12_workspace_root().join(entry["relative_path"].as_str().unwrap())
    }
}

impl Drop for CurrentBundleFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn workspace_relative(path: &Path) -> String {
    path.strip_prefix(ca12_workspace_root())
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/")
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let destination = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), destination).unwrap();
        }
    }
}

#[test]
fn full_current_bundle_accepts_pretty_saves_and_canonical_production_assets() {
    let fixture = CurrentBundleFixture::new();
    for id in ["p34-save", "production-voxel-save"] {
        let bytes = fs::metadata(fixture.entry_path(id)).unwrap().len();
        assert!(bytes > CA12_MAX_BUNDLE_FILE_BYTES);
        assert!(bytes > P34_MAX_INLINE_SAVE_BYTES);
    }
    let summary = validate_app_bundle_manifest(fixture.write_manifest()).unwrap();
    assert_eq!(summary.config_entries, 6);
    assert_eq!(summary.shader_assets, 14);
    assert_eq!(summary.discovered_shader_assets, 14);
    assert!(summary.shader_discovery_complete);
    assert!(summary.production_voxel_asset_manifest_validated);
    assert!(summary.production_voxel_generated_assets > 0);
    assert!(summary.largest_production_asset_bytes > LEGACY_BUNDLE_ASSET_LIMIT_BYTES);
    assert!(summary.largest_production_asset_bytes <= FVR07_MAX_COMMITTED_ASSET_BYTES);
    assert!(summary.largest_file_bytes >= summary.largest_production_asset_bytes);
    assert!(summary.missing_required_rejected);
    summary.validate().unwrap();

    // A validated bundle summary cannot omit its approved asset size or claim
    // that an admitted asset is larger than every admitted file.
    let mut corrupted = summary.clone();
    corrupted.largest_production_asset_bytes = 0;
    assert!(corrupted.validate().is_err());
    corrupted = summary.clone();
    corrupted.largest_file_bytes = summary.largest_production_asset_bytes - 1;
    assert!(corrupted.validate().is_err());
    corrupted = summary.clone();
    corrupted.largest_production_asset_bytes = FVR07_MAX_COMMITTED_ASSET_BYTES + 1;
    corrupted.largest_file_bytes = corrupted.largest_production_asset_bytes;
    assert!(corrupted.validate().is_err());
}

#[test]
fn canonical_file_save_admission_preserves_larger_populations_and_exact_authority() {
    let fixture = CurrentBundleFixture::with_production_population(10);
    let path = fixture.entry_path("production-voxel-save");
    assert!(fs::metadata(&path).unwrap().len() > LEGACY_BUNDLE_ASSET_LIMIT_BYTES);
    let save = PortableSaveFile::from_json_file(&path).unwrap();
    save.validate_with_asset_root(path.parent().unwrap())
        .unwrap();
    assert!(matches!(
        save.to_json_string_pretty(),
        Err(PersistenceError::HugeInlinePayload { bytes }) if bytes > P34_MAX_INLINE_SAVE_BYTES
    ));
    let before = save.restore_headless_world().unwrap();
    assert_eq!(before.organism_registry().len(), 10);
    let roundtrip_path = path.with_file_name("current-file-roundtrip.json");
    save.to_json_file(&roundtrip_path).unwrap();
    let roundtrip = PortableSaveFile::from_json_file(roundtrip_path).unwrap();
    roundtrip
        .validate_with_asset_root(path.parent().unwrap())
        .unwrap();
    assert_eq!(roundtrip, save);
    let after = roundtrip.restore_headless_world().unwrap();
    assert_eq!(
        after.canonical_signature_digest().unwrap(),
        before.canonical_signature_digest().unwrap()
    );
    assert_eq!(after.organism_registry().len(), 10);
    let summary = validate_app_bundle_manifest(fixture.write_manifest()).unwrap();
    assert_eq!(summary.config_entries, 6);
    assert_eq!(summary.discovered_shader_assets, 14);
    summary.validate().unwrap();
}

#[test]
fn oversized_metadata_and_shader_entries_reject_before_parsing_or_inventory_comparison() {
    let mut fixture = CurrentBundleFixture::new();
    for (id, bound) in [
        ("p34-config", CA12_MAX_BUNDLE_FILE_BYTES),
        ("p34-assets", CA12_MAX_REFERENCED_MANIFEST_BYTES),
    ] {
        let path = fixture.entry_path(id);
        let original = fs::read(&path).unwrap();
        // Invalid JSON makes the error distinguish size admission from decoding.
        fs::write(&path, vec![b'x'; usize::try_from(bound + 1).unwrap()]).unwrap();
        assert!(matches!(
            validate_app_bundle_manifest(fixture.write_manifest()),
            Err(GameAppShellError::Core(
                ScaffoldContractError::MissingPhaseData
            ))
        ));
        fs::write(path, original).unwrap();
    }
    let oversized_shader = fixture.root.join("oversized.wgsl");
    fs::write(
        &oversized_shader,
        vec![b'x'; usize::try_from(CA12_MAX_BUNDLE_FILE_BYTES + 1).unwrap()],
    )
    .unwrap();
    let original_path = fixture.manifest["shader_assets"][0]["relative_path"].clone();
    fixture.manifest["shader_assets"][0]["relative_path"] =
        serde_json::json!(workspace_relative(&oversized_shader));
    assert!(matches!(
        validate_app_bundle_manifest(fixture.write_manifest()),
        Err(GameAppShellError::Core(
            ScaffoldContractError::MissingPhaseData
        ))
    ));
    fixture.manifest["shader_assets"][0]["relative_path"] = original_path;

    // Rejection never alters the remaining positive bundle inputs.
    validate_app_bundle_manifest(fixture.write_manifest()).unwrap();
}

#[test]
fn full_current_bundle_rejects_missing_save_and_incomplete_runtime_shader_inventory() {
    let mut fixture = CurrentBundleFixture::new();
    let path = fixture.entry_path("p34-save");
    let save = fs::read(&path).unwrap();
    fs::remove_file(&path).unwrap();
    assert!(matches!(
        validate_app_bundle_manifest(fixture.write_manifest()),
        Err(GameAppShellError::VisibleWorldMismatch {
            message: "required app bundle entry is missing"
        })
    ));
    fs::write(path, save).unwrap();

    let shaders = fixture.manifest["shader_assets"].clone();
    fixture.manifest["shader_assets"]
        .as_array_mut()
        .unwrap()
        .retain(|shader| shader["id"] != "closed-loop-recurrent");
    assert!(validate_app_bundle_manifest(fixture.write_manifest()).is_err());
    fixture.manifest["shader_assets"] = shaders;
    fixture.manifest["shader_assets"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "id": "offline-training-rollout",
            "relative_path": "crates/alife_gpu_backend/shaders/training_rollout.wgsl",
            "required": true
        }));
    assert!(validate_app_bundle_manifest(fixture.write_manifest()).is_err());
}
