//! Metadata-only native references for source-binding tests. Placeholder assets
//! intentionally do not qualify full native recovery or hardware admission.
use super::*;
use alife_world::*;

pub(super) fn attach(
    current: &mut CurrentTrainingCheckpointV1,
    retained: Option<&CpuRetainedNeuralInheritance>,
) {
    let admitted = current.handoff.clone().admit().unwrap();
    let p = &admitted.phenotype;
    let mut config = CanonicalNewGameConfig::phase3(current.seed, 1).unwrap();
    config.brain_class = BrainScaleTier::Standard2048;
    config.founder_seed_base = current.founder_seed_base;
    config.sensor_profile = p.sensor_profile();
    let game =
        create_canonical_new_game_with_n2048_candidate(&config, &admitted.foundation).unwrap();
    let id = game.creatures[0].organism_id;
    let mut life = CurrentLifeCheckpointV1 {
        world_json: String::new(),
        sleep_journal_json: "{}".into(),
        assets_base64: Default::default(),
    };
    let mut manifest = AssetManifest::empty();
    let mut put = |name: &str, bytes: Vec<u8>| {
        let digest = PortableAssetDigest::for_bytes(&bytes);
        manifest.entries.push(AssetManifestEntry {
            asset_id: name.into(),
            kind: AssetKind::Other,
            relative_path: format!("{name}.json"),
            digest: digest.clone(),
            presence: AssetPresence::Required,
            schema_version: GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION,
            size_bytes: Some(bytes.len() as u64),
            provenance: Some(
                "synthetic metadata source-binding fixture; not hardware state".into(),
            ),
        });
        life.insert_asset(format!("{name}.json"), &bytes);
        GpuBrainAssetRef {
            asset_id: name.into(),
            digest,
        }
    };
    let phenotype = put("phenotype", serde_json::to_vec(p).unwrap());
    let inputs = put(
        "compiler_inputs",
        serde_json::to_vec(&current.handoff.compiler_inputs).unwrap(),
    );
    let canonical = GpuLiveTopologyCheckpointV1::recompile_for_phenotype(
        p,
        alife_gpu_backend::GpuV11Checkpoint::canonical_for_phenotype(p).unwrap(),
    )
    .unwrap();
    let top = retained.map_or(&canonical, |r| &r.topology);
    let topology_ref = put("live_topology", serde_json::to_vec(top).unwrap());
    let exact = ExactCognitiveCheckpointState {
        schema_version: V11_EXACT_COGNITIVE_STATE_SCHEMA_VERSION,
        organism_id: id,
        checkpoint_tick: Tick::ZERO,
        cognitive_context: CognitiveContextFrame::empty(id, ExperienceSequenceId(1), Tick::ZERO)
            .unwrap(),
        predictor: retained.map_or_else(Default::default, |r| r.predictor.clone()),
        selected_motor_bundle: None,
        cognitive_work: CognitiveWorkReceipt::zero(),
        sleep_state: SleepState::awake_at(Tick::ZERO),
        last_sleep_work: None,
        dendritic_branches: top.v11_checkpoint.dendritic_branches.clone(),
        structural_plasticity: top.v11_checkpoint.structural.clone(),
        structural_edit_receipts: vec![],
        last_sleep_report: None,
        private_semantic_prior: None,
        private_training_sampling: None,
    };
    let exact_ref = put("exact", exact.encode().unwrap());
    let bank = |layer_raw| {
        let mut b = PortableDualWeightBankV1 {
            schema_version: GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION,
            layer_raw,
            phenotype_hash: p.phenotype_hash(),
            synapse_count: top.total_synapse_count,
            active_generation: 1,
            active_bank: 0,
            bank_0_bits: vec![0; top.total_synapse_count as usize],
            bank_1_bits: vec![0; top.total_synapse_count as usize],
            canonical_digest: [0; 4],
        };
        b.canonical_digest = b.recompute_canonical_digest().unwrap();
        b
    };
    let lifetime = retained.map_or_else(
        || bank(GPU_BRAIN_WEIGHT_LAYER_LIFETIME),
        |r| r.lifetime.clone(),
    );
    let fast = retained.map_or_else(|| bank(GPU_BRAIN_WEIGHT_LAYER_FAST), |r| r.fast.clone());
    let lifetime_ref = put("lifetime", serde_json::to_vec(&lifetime).unwrap());
    let fast_ref = put("fast", serde_json::to_vec(&fast).unwrap());
    let placeholder = put("placeholder", b"{}".to_vec());
    let profile = SensorProfileIdentity {
        profile_id: p.sensor_profile().into(),
        profile_schema_version: 1,
        sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
    };
    let memory = MemorySidecarState::new_profiled(
        id,
        profile,
        MemoryBankConfig::new(256, 64, 4, 0.72, Confidence::new(0.).unwrap()).unwrap(),
    )
    .unwrap();
    let topo_sidecar =
        TopologySidecar::new_profiled(id, profile, TopologicalMapConfig::default()).unwrap();
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../alife_world/tests/fixtures/p34/tiny_save.json"
    ))
    .unwrap();
    let policy = BrainActivityPolicyV1::production_v1();
    let brain = GpuBrainSaveState {
        schema_version: GPU_BRAIN_SAVE_STATE_SCHEMA_VERSION,
        organism_id: id,
        phenotype_hash: p.phenotype_hash(),
        capacity_class_id: p.brain_class_id(),
        sensor_profile: profile,
        immutable_phenotype: phenotype,
        phenotype_compiler_inputs: inputs,
        live_structural_topology: Some(topology_ref),
        legacy_nano512_compatibility_receipt: None,
        active_weight_generation: lifetime.active_generation,
        active_weight_bank: lifetime.active_bank,
        active_eligibility_bank: 0,
        learning_transaction_generation: 1,
        lifetime_weights: lifetime_ref,
        fast_weights: fast_ref,
        eligibility: placeholder.clone(),
        replay_journal: placeholder.clone(),
        replay_journal_generation: 1,
        replay_journal_cursor: 0,
        replay_journal_event_count: 0,
        activation_state: placeholder.clone(),
        neuron_homeostasis: placeholder.clone(),
        checkpoint_tick: Tick::ZERO,
        exact_cognitive_state: Some(exact_ref),
        last_learning_replay_key: None,
        pending_eligibility: None,
        pending_experience_transaction: None,
        memory: MemorySidecarSaveState::from_sidecar(&memory, placeholder.clone(), None, None)
            .unwrap(),
        topology: TopologySidecarSaveSummary::from_sidecar(&topo_sidecar, placeholder.clone())
            .unwrap(),
        tracked_objects: TrackedObjectRegistry::new(game.world.seed(), 1024)
            .unwrap()
            .save_state(id)
            .unwrap(),
        language_grounding: Default::default(),
        life_statistics: None,
        sleep: SleepState::awake_at(Tick::ZERO),
        sleep_assets: GpuSleepAssetState::default(),
        backend_provenance: serde_json::from_value(
            fixture["creatures"][0]["gpu_brain"]["backend_provenance"].clone(),
        )
        .unwrap(),
        runtime_profile_id: 1,
        runtime_profile_digest: [31, 32, 33, 34],
        activity_policy_version: policy.policy_version,
        activity_policy_digest: policy.policy_digest,
        throttle_replay: ThrottleReplaySaveState::bootstrap(placeholder).unwrap(),
    };
    brain.validate().unwrap();
    let mut save = PortableSaveFile::from_headless_world(
        "source-binding-metadata-fixture",
        &game.world,
        RuntimeConfig::deterministic_default(game.world.seed(), BrainScaleTier::Standard2048),
        manifest,
        game.creatures.clone(),
    )
    .unwrap();
    save.creatures[0].gpu_brain = Some(brain);
    life.world_json = serde_json::to_string(&save).unwrap();
    current.life = Some(life);
}

pub(super) fn repin(inputs: &mut InterpolationExperimentInputs) {
    inputs.expected_pc_current_sha256 = native_receipt::sha256(&inputs.pc_current);
    if let InterpolationCloudParent::Current {
        bytes,
        expected_sha256,
    } = &mut inputs.cloud_parent
    {
        *expected_sha256 = native_receipt::sha256(bytes);
    }
}

pub(super) fn replace_selected_asset(
    life: &mut CurrentLifeCheckpointV1,
    save: &mut PortableSaveFile,
    id: &str,
    bytes: Vec<u8>,
) {
    let entry = save
        .assets
        .entries
        .iter_mut()
        .find(|e| e.asset_id == id)
        .unwrap();
    entry.digest = PortableAssetDigest::for_bytes(&bytes);
    entry.size_bytes = Some(bytes.len() as u64);
    life.insert_asset(entry.relative_path.clone(), &bytes);
    let digest = entry.digest.clone();
    let brain = save.creatures[0].gpu_brain.as_mut().unwrap();
    match id {
        "exact" => brain.exact_cognitive_state.as_mut().unwrap().digest = digest,
        "compiler_inputs" => brain.phenotype_compiler_inputs.digest = digest,
        _ => panic!("unsupported synthetic asset replacement"),
    }
}

/// Deliberately reseal an adversarial synthetic container so tests reach private
/// closure validation instead of stopping at the outer transport checksum.
pub(super) fn rewrite(
    original: &InterpolationExperimentInputs,
    change: impl FnOnce(&mut CurrentLifeCheckpointV1, &mut PortableSaveFile),
) -> InterpolationExperimentInputs {
    use sha2::{Digest, Sha256};
    let mut changed = original.clone();
    let mut cp = CurrentTrainingCheckpointV1::decode(&changed.pc_current).unwrap();
    let life = cp.life.as_mut().unwrap();
    let mut save = PortableSaveFile::from_json_str(&life.world_json).unwrap();
    change(life, &mut save);
    life.world_json = serde_json::to_string(&save).unwrap();
    let payload = serde_json::to_vec(&cp).unwrap();
    let mut bytes = original.pc_current[..56].to_vec();
    bytes[16..24].copy_from_slice(&(payload.len() as u64).to_le_bytes());
    bytes[24..56].copy_from_slice(&Sha256::digest(&payload));
    bytes.extend_from_slice(&payload);
    changed.pc_current = bytes;
    repin(&mut changed);
    changed
}
