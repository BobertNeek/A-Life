use std::{fs, path::Path};

use alife_core::{
    BrainCapacityClass, CreatureGenome, FoundationGeneticIdentity, OrganismId, Tick, Vec3f,
};
use alife_world::{
    persistence::{AssetManifest, PortableSaveFile, RuntimeConfig},
    HeadlessScenarioBuilder, WorldOrganismRecord,
};

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

// Preserve the committed legacy fixture. Positive validators use a fresh
// owned-organism save, not an invented migration of missing historical state.
pub fn current_fixture_workspace() -> tempfile::TempDir {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap();
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("Cargo.toml"), "[workspace]\n").unwrap();
    let p34_relative = "crates/alife_world/tests/fixtures/p34";
    let p34 = root.path().join(p34_relative);
    copy_tree(&source.join(p34_relative), &p34);
    copy_tree(
        &source.join("content/fixtures/g16"),
        &root.path().join("content/fixtures/g16"),
    );

    let config = RuntimeConfig::from_json_file(p34.join("tiny_config.json")).unwrap();
    let organism_id = OrganismId(1);
    let mut world = HeadlessScenarioBuilder::new(config.deterministic_seed)
        .agent("p34-current-agent", organism_id, Vec3f::ZERO)
        .food("p34-current-food", Vec3f::new(1.0, 0.0, 0.0), 0.25)
        .build()
        .unwrap();
    let genome = CreatureGenome::early_mammal_founder(
        73_128,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    let phenotype = genome.express().unwrap();
    assert!(!phenotype.chemistry.biochemical.reactions().is_empty());
    let record = WorldOrganismRecord::newborn(
        organism_id,
        world.entity_id("p34-current-agent").unwrap(),
        genome,
        phenotype,
        Tick::ZERO,
    )
    .unwrap();
    world.register_organism_record(record).unwrap();
    let assets = AssetManifest::from_json_file(p34.join("tiny_asset_manifest.json")).unwrap();
    let save = PortableSaveFile::from_headless_world(
        "tools-current-owned-organism",
        &world,
        config,
        assets,
        Vec::new(),
    )
    .unwrap();
    assert_eq!(save.world.organism_records.as_ref().unwrap().len(), 1);
    save.validate_with_asset_root(&p34).unwrap();
    save.to_json_file(p34.join("tiny_save.json")).unwrap();
    let loaded = PortableSaveFile::from_json_file(p34.join("tiny_save.json")).unwrap();
    assert_eq!(loaded, save);
    root
}
