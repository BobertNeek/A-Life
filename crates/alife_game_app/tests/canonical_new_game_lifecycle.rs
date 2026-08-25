use std::path::PathBuf;

use alife_core::BrainScaleTier;
use alife_game_app::{stage_phase3_new_game, CanonicalNewGameLaunchRequest};
use alife_world::{AssetManifest, RuntimeConfig};

fn phase3_request(population: u16) -> CanonicalNewGameLaunchRequest {
    let root = std::env::temp_dir().join(format!(
        "alife-phase3-stage-{}-{population}",
        std::process::id()
    ));
    let mut config = RuntimeConfig::deterministic_default(240_824, BrainScaleTier::Nano512);
    config.features.gpu_backend_enabled = true;
    CanonicalNewGameLaunchRequest {
        world_seed: 240_824,
        population,
        save_path: root.join("phase3-save.json"),
        asset_root: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        config,
        assets: AssetManifest::empty(),
    }
}

#[test]
fn new_game_base_save_matches_every_canonical_founder() {
    let staged = stage_phase3_new_game(phase3_request(6)).unwrap();

    assert_eq!(staged.save.creatures.len(), 6);
    assert_eq!(
        staged.save.world.organism_records.as_ref().unwrap().len(),
        6
    );
    assert!(staged
        .save
        .creatures
        .iter()
        .all(|creature| creature.gpu_brain.is_none()));
    assert_eq!(staged.receipt.requested_population, 6);
    assert_eq!(staged.save_path, phase3_request(6).save_path);
    staged
        .save
        .validate_with_asset_root(&staged.asset_root)
        .unwrap();
}
