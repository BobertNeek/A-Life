use std::{collections::BTreeSet, path::PathBuf};

use alife_core::BrainScaleTier;
#[cfg(feature = "gpu-tests")]
use alife_core::{
    BrainCapacityClass, CreatureGenome, FoundationGeneticIdentity, LegacyFoundationAbiId,
    OrganismId, PolicyBackend, ProductionRuntimeAbiId, ProductionRuntimePath,
    ScaffoldContractError, SensorProfile, Tick, WorldEntityId,
};
#[cfg(feature = "gpu-tests")]
use alife_game_app::{
    create_canonical_new_game_runtime,
    create_canonical_new_game_runtime_with_forced_late_failure_for_test,
    legacy_nano512_compatibility_receipt_for_record_for_test, AppShellLaunchConfig,
    GpuLiveBrainRuntime,
};
use alife_game_app::{stage_phase3_new_game, CanonicalNewGameLaunchRequest};
#[cfg(feature = "gpu-tests")]
use alife_gpu_backend::{GpuClosedLoopBackend, GpuRuntimeProfile};
use alife_world::{AssetManifest, RuntimeConfig};
#[cfg(feature = "gpu-tests")]
use alife_world::{PortableSaveFile, WorldOrganismRecord};

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

#[cfg(feature = "gpu-tests")]
#[test]
fn n512_record_with_unknown_foundation_metadata_is_not_compatibility_admitted() {
    let foundation = FoundationGeneticIdentity::new(
        0x554E_4B4E_4F57_4E01,
        99,
        0x554E_4B4E_4F57_FA11,
        BrainCapacityClass::N512_ID,
    )
    .unwrap();
    let genome = CreatureGenome::early_mammal_founder(0xBAD5_EED1, foundation).unwrap();
    let phenotype = genome.express().unwrap();
    let record = WorldOrganismRecord::newborn(
        OrganismId(91),
        WorldEntityId(191),
        genome,
        phenotype,
        Tick::ZERO,
    )
    .unwrap();

    assert_eq!(
        legacy_nano512_compatibility_receipt_for_record_for_test(
            &record,
            Tick::ZERO,
            SensorProfile::GroundedObjectSlotsV1,
        ),
        Err(ScaffoldContractError::PhenotypeCompile)
    );
}

#[cfg(feature = "gpu-tests")]
#[test]
fn gpu_new_game_publishes_only_exact_resident_state() {
    let root =
        std::env::temp_dir().join(format!("alife-phase3-gpu-new-game-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let mut request = phase3_request(4);
    request.save_path = root.join("phase3-save.json");
    request.asset_root = root.join("assets");
    std::fs::create_dir_all(&request.asset_root).unwrap();

    let result = create_canonical_new_game_runtime(request).unwrap();
    assert_eq!(result.runtime.world_snapshot().organism_registry().len(), 4);
    let residency = result.runtime.residency_summary();
    assert_eq!(residency.handle_count, 4);
    assert_eq!(residency.resident_count, 4);
    assert_eq!(residency.memory_sidecar_count, 4);
    assert_eq!(residency.topology_sidecar_count, 4);
    assert_eq!(
        result.runtime.lineage_archive_manifest_count().unwrap(),
        Some(4)
    );
    assert!(result.exact_save.creatures.iter().all(|creature| {
        let brain = creature.gpu_brain.as_ref().unwrap();
        let receipt = brain.legacy_nano512_compatibility_receipt.as_ref().unwrap();
        receipt.source_abi_id() == LegacyFoundationAbiId::NANO512_V1
            && receipt.runtime_abi_id() == ProductionRuntimeAbiId::V2
            && receipt.runtime_path() == ProductionRuntimePath::ORDINARY_GPU_ORGANISM_V2
    }));
    let organism_genomes = result
        .exact_save
        .creatures
        .iter()
        .map(|creature| creature.genome_id.0)
        .collect::<BTreeSet<_>>();
    let immutable_phenotypes = result
        .exact_save
        .creatures
        .iter()
        .map(|creature| {
            creature
                .gpu_brain
                .as_ref()
                .unwrap()
                .immutable_phenotype
                .asset_id
                .clone()
        })
        .collect::<BTreeSet<_>>();
    let coordinate_inputs = result
        .exact_save
        .creatures
        .iter()
        .map(|creature| {
            creature
                .gpu_brain
                .as_ref()
                .unwrap()
                .phenotype_compiler_inputs
                .asset_id
                .clone()
        })
        .collect::<BTreeSet<_>>();
    let receipts = result
        .exact_save
        .creatures
        .iter()
        .map(|creature| {
            creature
                .gpu_brain
                .as_ref()
                .unwrap()
                .legacy_nano512_compatibility_receipt
                .as_ref()
                .unwrap()
                .canonical_digest()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(organism_genomes.len(), 4);
    assert_eq!(immutable_phenotypes.len(), 1);
    assert_eq!(coordinate_inputs.len(), 1);
    assert_eq!(receipts.len(), 1);
    let reopened = PortableSaveFile::from_json_file(&result.save_path).unwrap();
    reopened
        .validate_with_asset_root(&result.asset_root)
        .unwrap();
    assert_eq!(reopened, result.exact_save);

    let config_path = root.join("phase3-config.json");
    let asset_manifest_path = root.join("phase3-assets.json");
    std::fs::write(
        &config_path,
        serde_json::to_vec_pretty(&result.exact_save.config).unwrap(),
    )
    .unwrap();
    std::fs::write(
        &asset_manifest_path,
        serde_json::to_vec_pretty(&result.exact_save.assets).unwrap(),
    )
    .unwrap();
    let launch = AppShellLaunchConfig {
        fixture_root: root.clone(),
        config_path,
        asset_manifest_path,
        save_path: result.save_path.clone(),
        asset_root: result.asset_root.clone(),
        start_paused: false,
        brain_policy: PolicyBackend::NeuralClosedLoopGpu,
    };

    drop(result);
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap();
    let mut restored = GpuLiveBrainRuntime::from_p34_launch(backend, &launch).unwrap();
    assert_eq!(restored.residency_summary().resident_count, 4);
    assert!(restored
        .capture_portable_checkpoint()
        .unwrap()
        .creatures
        .iter()
        .all(|creature| creature
            .gpu_brain
            .as_ref()
            .unwrap()
            .legacy_nano512_compatibility_receipt
            .is_some()));
    drop(restored);
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(feature = "gpu-tests")]
#[test]
fn late_new_game_failure_rolls_back_every_artifact_and_allows_identical_retry() {
    let root = std::env::temp_dir().join(format!(
        "alife-phase3-gpu-new-game-retry-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let mut request = phase3_request(4);
    request.save_path = root.join("phase3-save.json");
    request.asset_root = root.join("assets");
    std::fs::create_dir_all(&request.asset_root).unwrap();
    let preexisting_gpu_root = request.asset_root.join("gpu-brain");
    std::fs::create_dir_all(&preexisting_gpu_root).unwrap();
    let sentinel_path = preexisting_gpu_root.join("pre-existing-foundation.bin");
    let sentinel_bytes = b"immutable-pre-existing-production-asset";
    std::fs::write(&sentinel_path, sentinel_bytes).unwrap();
    let staging_path = root.join(".phase3-save.json.phase3-staging");
    let lineage_root = root.join(".phase3-save.lineage");

    let failure = match create_canonical_new_game_runtime_with_forced_late_failure_for_test(
        request.clone(),
    ) {
        Err(error) => error,
        Ok(_) => panic!("forced late failure unexpectedly committed"),
    };
    assert!(failure
        .to_string()
        .contains("test-forced late canonical New Game failure"));
    assert!(!staging_path.exists());
    assert!(!request.save_path.exists());
    assert!(!lineage_root.exists());
    assert_eq!(std::fs::read(&sentinel_path).unwrap(), sentinel_bytes);
    assert!(preexisting_gpu_root.exists());
    assert_eq!(std::fs::read_dir(&preexisting_gpu_root).unwrap().count(), 1);

    let retry = create_canonical_new_game_runtime(request).unwrap();
    assert_eq!(retry.runtime.residency_summary().resident_count, 4);
    assert!(retry.save_path.exists());
    assert!(lineage_root.exists());
    assert_eq!(std::fs::read(&sentinel_path).unwrap(), sentinel_bytes);

    drop(retry);
    std::fs::remove_dir_all(root).unwrap();
}
