use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use alife_core::predictive::GroundedSuccessorPredictor;
use alife_core::sleep::{SleepWorkReceipt, SleepWorkStatus};
use alife_core::structural_plasticity::CoactivationEvidence;
use alife_core::{
    select_focal_targets, ActionId, AttentionSelectionPolicy, BrainActivityPolicyV1,
    BrainCapacityClass, ChannelCommand, CognitiveContextFrame, CognitiveWorkReceipt, Confidence,
    DendriticBranch, DendriticBranchSet, DendriticInputRef, DurationTicks, ExperienceSequenceId,
    HysteresisState, Intensity, JointMotorCondition, MemoryBankConfig, MemorySidecarState,
    MotorChannel, MotorChannelFactor, MotorCommandBundle, NormalizedScalar, OrganismId,
    PredictionTargetReceipt, SemanticStateVector, SensorProfile, SensorProfileIdentity,
    SensoryAbiVersion, SleepState, SleepTrigger, StableFocusIdentity, StructuralPlasticityConfig,
    StructuralPlasticityState, TopologicalMapConfig, TopologySidecar, Validate, Vec3f,
    SLEEP_CONSOLIDATION_SCHEMA_VERSION,
};
use alife_game_app::{
    merge_gpu_checkpoint_manifest_entries, GpuBrainCheckpointWrite, GpuCheckpointAssetStore,
};
use alife_world::persistence::{
    AssetManifest, ExactCognitiveCheckpointState, GpuBrainAssetRef, GpuBrainSaveState,
    GpuSleepAssetState, MemorySidecarSaveState, PortableAssetDigest, PortableSaveFile,
    ThrottleReplaySaveState, TopologySidecarSaveSummary, GPU_BRAIN_SAVE_STATE_SCHEMA_VERSION,
    V11_EXACT_COGNITIVE_STATE_SCHEMA_VERSION,
};
use alife_world::TrackedObjectRegistry;

#[path = "../src/test_fixtures.rs"]
mod test_fixtures;

// Only metadata and the exact cognitive sidecar are exercised in this CPU test.
// Bulk asset references/provenance come from the historical fixture; sidecar
// contracts and world organism authority are constructed at current versions.
fn checkpoint_metadata(save: &PortableSaveFile, fixture: &std::path::Path) -> GpuBrainSaveState {
    let source: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.join("tiny_save.json")).unwrap()).unwrap();
    let brain = &source["creatures"][0]["gpu_brain"];
    let asset =
        |field: &str| -> GpuBrainAssetRef { serde_json::from_value(brain[field].clone()).unwrap() };
    let organism_id = save.creatures[0].organism_id;
    let tick = save.world.tick;
    let profile = SensorProfileIdentity {
        profile_id: SensorProfile::GroundedObjectSlotsV1.into(),
        profile_schema_version: 1,
        sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
    };
    let memory = MemorySidecarState::new_profiled(
        organism_id,
        profile,
        MemoryBankConfig::new(64, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap(),
    )
    .unwrap();
    let topology =
        TopologySidecar::new_profiled(organism_id, profile, TopologicalMapConfig::default())
            .unwrap();
    let reference = |label: &str| GpuBrainAssetRef {
        asset_id: label.to_string(),
        digest: PortableAssetDigest::for_bytes(label.as_bytes()),
    };
    let policy = BrainActivityPolicyV1::production_v1();
    let state = GpuBrainSaveState {
        schema_version: GPU_BRAIN_SAVE_STATE_SCHEMA_VERSION,
        organism_id,
        phenotype_hash: serde_json::from_value(brain["phenotype_hash"].clone()).unwrap(),
        capacity_class_id: BrainCapacityClass::N512_ID,
        sensor_profile: profile,
        immutable_phenotype: asset("immutable_phenotype"),
        phenotype_compiler_inputs: asset("phenotype_compiler_inputs"),
        live_structural_topology: Some(reference("metadata-only-live-topology")),
        legacy_nano512_compatibility_receipt: None,
        active_weight_generation: 1,
        active_weight_bank: 0,
        active_eligibility_bank: 0,
        learning_transaction_generation: 1,
        lifetime_weights: asset("lifetime_weights"),
        fast_weights: asset("fast_weights"),
        eligibility: asset("eligibility"),
        replay_journal: asset("replay_journal"),
        replay_journal_generation: 1,
        replay_journal_cursor: 0,
        replay_journal_event_count: 0,
        activation_state: asset("activation_state"),
        neuron_homeostasis: asset("neuron_homeostasis"),
        checkpoint_tick: tick,
        exact_cognitive_state: None,
        last_learning_replay_key: None,
        pending_eligibility: None,
        pending_experience_transaction: None,
        memory: MemorySidecarSaveState::from_sidecar(
            &memory,
            reference("metadata-only-memory"),
            None,
            None,
        )
        .unwrap(),
        topology: TopologySidecarSaveSummary::from_sidecar(
            &topology,
            reference("metadata-only-topology"),
        )
        .unwrap(),
        tracked_objects: TrackedObjectRegistry::new(save.world.seed, 1_024)
            .unwrap()
            .save_state(organism_id)
            .unwrap(),
        language_grounding: Default::default(),
        life_statistics: None,
        sleep: SleepState::awake_at(tick),
        sleep_assets: GpuSleepAssetState::default(),
        backend_provenance: serde_json::from_value(brain["backend_provenance"].clone()).unwrap(),
        runtime_profile_id: 1,
        runtime_profile_digest: [31, 32, 33, 34],
        activity_policy_version: policy.policy_version,
        activity_policy_digest: policy.policy_digest,
        throttle_replay: ThrottleReplaySaveState::bootstrap(reference("metadata-only-throttle"))
            .unwrap(),
    };
    state.validate().unwrap();
    state
}

fn unique_asset_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "alife-v11-checkpoint-{}-{nonce}",
        std::process::id()
    ))
}

#[test]
fn exact_checkpoint_manifest_restore_preserves_control_path() {
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../alife_world/tests/fixtures/p34");
    let save = test_fixtures::current_scene_save(&fixture, 1);
    let mut save_state = checkpoint_metadata(&save, &fixture);
    let organism_id = save_state.organism_id;
    let checkpoint_tick = save_state.checkpoint_tick;
    let sequence_id = ExperienceSequenceId(3);
    let action = ActionId::new(7).expect("valid action id");
    let source_digest = [11, 22, 33, 44];

    let mut cognitive_context =
        CognitiveContextFrame::empty(organism_id, sequence_id, checkpoint_tick)
            .expect("empty cognitive context");
    cognitive_context.attention.hysteresis = HysteresisState {
        previous_identity: Some(StableFocusIdentity::Organism(OrganismId(19))),
        retained_ticks: 3,
        switch_margin: NormalizedScalar::new(0.4).expect("bounded switch margin"),
    };
    cognitive_context
        .validate_contract()
        .expect("non-default attention remains valid");

    let mut predictor = GroundedSuccessorPredictor::with_learning_rate(0.5)
        .expect("bounded predictor learning rate");
    let target = PredictionTargetReceipt::for_successor(
        organism_id,
        sequence_id,
        action,
        checkpoint_tick,
        source_digest,
        SemanticStateVector::new(vec![0.5, 0.25]).expect("source semantic state"),
        JointMotorCondition::new(vec![MotorChannelFactor {
            channel: MotorChannel::Locomotion,
            primitive: action,
            intensity: 0.8,
            duration_ticks: 2,
            direction: Vec3f::new(1.0, 0.0, 0.0),
            stand_off_distance: 0.5,
            confidence: 0.9,
            target: None,
            payload: Vec::new(),
            coordination_group: 0,
        }])
        .expect("joint motor condition"),
        SemanticStateVector::new(vec![0.25, 0.75]).expect("target semantic state"),
    )
    .expect("grounded prediction target");
    predictor
        .observe(&target)
        .expect("non-default predictor update");

    let motor_command = ChannelCommand::new(
        MotorChannel::Locomotion,
        action,
        None,
        Vec3f::new(1.0, 0.0, 0.0),
        Intensity::new(0.8).expect("bounded intensity"),
        DurationTicks::new(2),
        0.5,
        alife_core::Confidence::new(0.9).expect("bounded confidence"),
        0,
    )
    .expect("motor command");
    let motor_bundle = MotorCommandBundle::new(
        organism_id,
        sequence_id,
        checkpoint_tick,
        vec![motor_command],
    )
    .expect("non-default motor intent");

    let cognitive_work =
        CognitiveWorkReceipt::from_counters(2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37)
            .expect("non-default cognitive work");
    let mut sleep_work = SleepWorkReceipt {
        schema_version: SLEEP_CONSOLIDATION_SCHEMA_VERSION,
        tick: checkpoint_tick,
        status: SleepWorkStatus::SkippedLowPressure,
        fatigue: NormalizedScalar::new(0.2).expect("bounded fatigue"),
        sleep_pressure: NormalizedScalar::new(0.8).expect("bounded sleep pressure"),
        replay_digest: [0; 4],
        replay_event_count: 0,
        replay_eligibility_sample_count: 0,
        promoted_memory_ids: Vec::new(),
        predictor_update_count: 0,
        concept: None,
        work_units: 0,
        canonical_digest: [0; 4],
    };
    sleep_work.canonical_digest = sleep_work
        .recompute_canonical_digest()
        .expect("sleep work digest");
    sleep_work
        .validate_contract()
        .expect("non-default sleep work remains valid");

    let mut sleep_state = SleepState::awake_at(checkpoint_tick);
    sleep_state.cycles_completed = 2;
    sleep_state.last_consolidated_cycle_id = 2;
    sleep_state.last_trigger = Some(SleepTrigger::FatigueThreshold);
    sleep_state
        .validate_contract()
        .expect("non-default sleep state remains valid");

    let dendritic_branches = DendriticBranchSet::new(vec![DendriticBranch::new(
        0,
        0.5,
        1.0,
        vec![DendriticInputRef::new(1, 0.75).expect("dendritic input")],
    )
    .expect("dendritic branch")])
    .expect("dendritic branch set");
    let mut structural_plasticity =
        StructuralPlasticityState::new(512, StructuralPlasticityConfig::default())
            .expect("bounded structural state");
    structural_plasticity
        .discover_candidates(&[CoactivationEvidence {
            region: 0,
            source: 1,
            target: 2,
            coactivation: 10,
            eligibility: 10,
            concept_gap_support: 1,
        }])
        .expect("non-default structural evidence");

    let checkpoint = ExactCognitiveCheckpointState {
        schema_version: V11_EXACT_COGNITIVE_STATE_SCHEMA_VERSION,
        organism_id,
        checkpoint_tick,
        cognitive_context,
        predictor,
        selected_motor_bundle: Some(motor_bundle),
        cognitive_work,
        sleep_state,
        last_sleep_work: Some(sleep_work),
        dendritic_branches,
        structural_plasticity,
        structural_edit_receipts: Vec::new(),
        last_sleep_report: None,
        private_semantic_prior: None,
    };
    checkpoint.validate().expect("valid exact checkpoint");
    save_state.sleep = checkpoint.sleep_state;

    let asset_root = unique_asset_root();
    fs::create_dir_all(&asset_root).expect("asset root");
    let store = GpuCheckpointAssetStore::new(&asset_root).expect("asset store");
    let mut write = GpuBrainCheckpointWrite {
        save_state,
        manifest_entries: Vec::new(),
        checkpoint_digest: [0; 4],
    };
    write
        .attach_exact_cognitive_state(&store, &checkpoint)
        .expect("attach exact checkpoint asset");
    let asset_ref = write
        .save_state
        .exact_cognitive_state
        .clone()
        .expect("persisted exact asset reference");
    let roundtrip_save: GpuBrainSaveState = serde_json::from_str(
        &serde_json::to_string(&write.save_state).expect("serialize save state"),
    )
    .expect("deserialize save state");
    assert_eq!(
        roundtrip_save.exact_cognitive_state,
        Some(asset_ref.clone())
    );

    let mut manifest = AssetManifest::empty();
    merge_gpu_checkpoint_manifest_entries(&mut manifest, write.manifest_entries)
        .expect("attach manifest entry");
    manifest
        .validate_with_root(&asset_root)
        .expect("manifest and digest validate");
    let restored = store
        .read_exact_cognitive_state(&manifest, &asset_ref)
        .expect("restore exact checkpoint asset");
    assert_eq!(restored, checkpoint);
    assert_ne!(restored.cognitive_work, CognitiveWorkReceipt::zero());
    assert!(restored.selected_motor_bundle.is_some());
    assert_ne!(restored.sleep_state, SleepState::awake_at(checkpoint_tick));
    assert!(restored.last_sleep_work.is_some());
    assert!(!restored.dendritic_branches.is_empty());
    assert_ne!(
        restored.structural_plasticity,
        StructuralPlasticityState::new(512, StructuralPlasticityConfig::default())
            .expect("default structural state")
    );

    let control_prediction = checkpoint
        .predictor
        .predict(target.source_state(), target.motor_condition())
        .expect("unsaved control prediction");
    let restored_prediction = restored
        .predictor
        .predict(target.source_state(), target.motor_condition())
        .expect("restored prediction");
    assert_eq!(restored_prediction, control_prediction);
    assert_eq!(
        restored.selected_motor_bundle,
        checkpoint.selected_motor_bundle
    );
    assert_eq!(
        restored
            .selected_motor_bundle
            .as_ref()
            .and_then(|bundle| bundle.channels.first())
            .map(|command| command.primitive),
        Some(action)
    );

    let control_attention = select_focal_targets(
        organism_id,
        sequence_id,
        checkpoint_tick,
        &[],
        checkpoint.cognitive_context.attention.hysteresis,
        AttentionSelectionPolicy::default(),
    )
    .expect("unsaved control attention");
    let restored_attention = select_focal_targets(
        organism_id,
        sequence_id,
        checkpoint_tick,
        &[],
        restored.attention().hysteresis,
        AttentionSelectionPolicy::default(),
    )
    .expect("restored attention");
    assert_eq!(restored_attention, control_attention);

    fs::remove_dir_all(asset_root).expect("remove test asset root");
}
