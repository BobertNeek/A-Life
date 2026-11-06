use std::{fs, path::PathBuf};

use alife_core::{BrainScaleTier, OrganismId, Vec3f};
use alife_world::{
    persistence::{AssetManifest, PortableSaveFile, RuntimeConfig},
    HeadlessScenarioBuilder,
};

fn fixture(name: &str) -> PortableSaveFile {
    let world = HeadlessScenarioBuilder::new(4242)
        .agent("creature", OrganismId(1), Vec3f::ZERO)
        .food("berry", Vec3f::new(1.0, 0.0, 0.0), 0.75)
        .build()
        .unwrap();
    PortableSaveFile::from_headless_world(
        name,
        &world,
        RuntimeConfig::deterministic_default(4242, BrainScaleTier::Nano512),
        AssetManifest::empty(),
        vec![],
    )
    .unwrap()
}

fn temp_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "alife-atomic-portable-save-{name}-{}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    root
}

#[test]
fn save_creation_and_replacement_preserve_exact_world_state_and_budget() {
    let root = temp_root("replace");
    let budget = alife_storage::OutputBudget::register(&root, 8 * 1024 * 1024).unwrap();
    let path = root.join("creature.alife.json");
    let mut save = fixture("first");
    save.to_json_file(&path).unwrap();
    save.save_id = "replacement".to_string();
    save.to_json_file(&path).unwrap();
    let bytes = fs::read(&path).unwrap();
    assert_eq!(bytes, save.to_json_string_pretty().unwrap().as_bytes());
    let restored = PortableSaveFile::from_json_file(&path).unwrap();
    assert_eq!(restored.save_id, save.save_id);
    assert_eq!(restored.world, save.world);
    assert_eq!(
        restored
            .restore_headless_world()
            .unwrap()
            .canonical_signature_digest()
            .unwrap(),
        save.restore_headless_world()
            .unwrap()
            .canonical_signature_digest()
            .unwrap()
    );
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    assert_eq!(budget.usage().unwrap().used, bytes.len() as u64);
    alife_storage::remove_dir_all(&root).unwrap();
    assert_eq!(budget.usage().unwrap().used, 0);
}

#[test]
fn failed_save_write_keeps_previous_save_readable_and_releases_staging_budget() {
    let root = temp_root("write-failure");
    let original = fixture("previous");
    let bytes = original.to_json_string_pretty().unwrap().into_bytes();
    let budget = alife_storage::OutputBudget::register(&root, bytes.len() as u64 + 1).unwrap();
    let path = root.join("creature.alife.json");
    alife_storage::write(&path, &bytes).unwrap();
    let mut replacement = original.clone();
    replacement.save_id = "larger replacement".repeat(1024);
    assert!(replacement.to_json_file(&path).is_err());
    assert!(fs::read(&path).unwrap() == bytes, "previous save changed");
    let restored = PortableSaveFile::from_json_file(&path).unwrap();
    assert_eq!(restored.save_id, original.save_id);
    assert_eq!(restored.world, original.world);
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    let usage = budget.usage().unwrap();
    assert_eq!(usage.used, bytes.len() as u64);
    assert_eq!(usage.reserved, 0);
    assert!(usage.high_water <= usage.limit);
    alife_storage::remove_dir_all(&root).unwrap();
}

#[test]
fn failed_save_publication_preserves_destination_and_cleans_its_staged_file() {
    let root = temp_root("publication-failure");
    let budget = alife_storage::OutputBudget::register(&root, 8 * 1024 * 1024).unwrap();
    let path = root.join("creature.alife.json");
    fs::create_dir(&path).unwrap();
    let sentinel = path.join("recoverable-state");
    alife_storage::write(&sentinel, b"preserve this").unwrap();
    assert!(fixture("replacement").to_json_file(&path).is_err());
    assert_eq!(fs::read(&sentinel).unwrap(), b"preserve this");
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    assert_eq!(budget.usage().unwrap().used, 13);
    alife_storage::remove_dir_all(&root).unwrap();
}
