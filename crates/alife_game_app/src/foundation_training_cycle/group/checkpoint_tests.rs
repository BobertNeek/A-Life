//! Native group admission is checked without constructing a device.
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Source(PathBuf);
impl Drop for Source {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
pub(super) fn accepted_asset() -> (FoundationWeightAsset, u64) {
    static ASSET: std::sync::OnceLock<FoundationWeightAsset> = std::sync::OnceLock::new();
    let asset = ASSET.get_or_init(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/founders/terrain-care-n2048-v2");
        let source = FoundationWeightAsset::decode_canonical(
            &std::fs::read(root.join("trained.alife-foundation")).unwrap(),
        )
        .unwrap();
        let receipt: FoundationAdaptationReceipt =
            serde_json::from_slice(&std::fs::read(root.join("adaptation.json")).unwrap()).unwrap();
        let (current, _, _) =
            super::super::prepare_terrain_founder_refresh(&root, &receipt, &source).unwrap();
        assert!(current
            .weights()
            .iter()
            .zip(source.weights())
            .all(|(a, b)| a.to_bits() == b.to_bits()));
        current
    });
    (asset.clone(), 539_363_617)
}

fn fixture() -> (
    Source,
    alife_training::FoundationTrainerCheckpoint,
    alife_training::PpoValueHeadCheckpoint,
) {
    let (asset, seed) = accepted_asset();
    let mut config = alife_world::CanonicalNewGameConfig::phase3(872_991, 1).unwrap();
    config.founder_seed_base = seed;
    config.brain_class = BrainScaleTier::Standard2048;
    config.sensor_profile = asset.manifest().sensor_profile();
    let game =
        alife_world::create_canonical_new_game_with_n2048_candidate(&config, &asset).unwrap();
    let record = game.world.organism_registry().iter().next().unwrap();
    let genome = record.phenotype().brain_genome.clone();
    let development = crate::gpu_live_runtime::foundation_construction_development(
        &genome,
        &alife_core::BrainCapacityClass::n2048(),
        &record
            .phenotype()
            .development_state_at(alife_core::Tick::ZERO)
            .unwrap(),
    )
    .unwrap();
    let (phenotype, _) = alife_core::PhenotypeCompiler::compile_n2048_foundation_candidate(
        genome,
        development,
        asset.clone(),
    )
    .unwrap();
    let count = phenotype.synapses().len();
    let age = (1 << 24) + 17;
    let actor = alife_training::FoundationTrainerCheckpoint {
        schema_version: 2,
        phenotype_hash: phenotype.phenotype_hash(),
        source_foundation_digest: asset.digest(),
        optimizer_step: age,
        config: AdamWConfig::default(),
        stage_mask: StageTrainableMask::from_synapse_indices(
            &phenotype,
            &(0..count as u32).collect::<Vec<_>>(),
        )
        .unwrap(),
        weights: asset.weights().to_vec(),
        first_moment: vec![0.125; count],
        second_moment: vec![0.25; count],
        update_ages: vec![age; count],
    };
    let value = alife_training::PpoValueHeadCheckpoint {
        feature_count: phenotype.neuron_count(),
        optimizer_step: age + 2,
        parameters: vec![0.125; (phenotype.neuron_count() as usize + 1) * 3],
        last_updated_policy_version: Some(7),
    };
    let path = std::env::temp_dir().join(format!(
        "alife-native-group-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&path).unwrap();
    std::fs::write(
        path.join("trained.alife-foundation"),
        asset.encode_canonical().unwrap(),
    )
    .unwrap();
    let actor_bytes = serde_json::to_vec(&actor).unwrap();
    let value_bytes = serde_json::to_vec(&value).unwrap();
    std::fs::write(path.join("actor-checkpoint.json"), &actor_bytes).unwrap();
    std::fs::write(path.join("value-checkpoint.json"), &value_bytes).unwrap();
    let receipt = serde_json::json!({"schema":1,"complete":true,"mode":"group50_shared_habitat","population":50,"seed":872991,"founder_seed_base":seed,"policy_version":7,"biological_objective_version":2,
        "critic_objective_version":1,"next_cohort_optimizer_rebound":true,"new_asset_digest":digest(&asset),"actor_optimizer_step":actor.optimizer_step,"value_optimizer_step":value.optimizer_step,
        "actor_checkpoint_sha256":admission::hash(&actor_bytes),"value_checkpoint_sha256":admission::hash(&value_bytes)});
    std::fs::write(
        path.join("group50.json"),
        serde_json::to_vec(&receipt).unwrap(),
    )
    .unwrap();
    (Source(path), actor, value)
}
#[test]
fn group_native_admission_preserves_complete_learned_state_and_exact_large_ages() {
    let (source, actor, value) = fixture();
    let loaded = group_source(&source.0).unwrap();
    assert_eq!(loaded.actor, actor);
    assert_eq!(loaded.value, value);
    assert_eq!(loaded.policy_version, 8);
    assert!(loaded
        .actor
        .update_ages
        .iter()
        .all(|age| *age == (1 << 24) + 17));
    assert!(loaded.actor.first_moment.iter().all(|m| *m == 0.125));
    let validation = validate_foundation_training_group_source(&source.0).unwrap();
    assert_eq!(validation["actor_optimizer_step"], (1 << 24) + 17);
    assert_eq!(validation["actor_update_age_max"], (1 << 24) + 17);
    assert_eq!(validation["seed_reserved"], false);
    assert!(!source.0.join(".eat-held-food-owner.lock").exists());
}
#[test]
fn group_native_admission_rejects_corrupted_moments_and_age_before_gpu() {
    let (source, mut actor, _) = fixture();
    actor.update_ages[0] = actor.optimizer_step + 1;
    let bytes = serde_json::to_vec(&actor).unwrap();
    std::fs::write(source.0.join("actor-checkpoint.json"), &bytes).unwrap();
    let receipt = source.0.join("group50.json");
    let mut json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&receipt).unwrap()).unwrap();
    json["actor_checkpoint_sha256"] = serde_json::json!(admission::hash(&bytes));
    std::fs::write(&receipt, serde_json::to_vec(&json).unwrap()).unwrap();
    assert!(group_source(&source.0).is_err());
}
#[test]
fn partial_group_handoffs_and_changed_value_bytes_are_rejected() {
    let (source, _, _) = fixture();
    let receipt = source.0.join("group50.json");
    let bytes = std::fs::read(&receipt).unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    json["next_cohort_optimizer_rebound"] = serde_json::json!(false);
    std::fs::write(&receipt, serde_json::to_vec(&json).unwrap()).unwrap();
    assert!(group_source(&source.0).is_err());
    std::fs::write(&receipt, bytes).unwrap();
    std::fs::write(source.0.join("value-checkpoint.json"), b"{}").unwrap();
    assert!(group_source(&source.0).is_err());
}
