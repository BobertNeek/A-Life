use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use alife_core::predictive::GroundedSuccessorPredictor;
use alife_core::sleep::{SleepWorkReceipt, SleepWorkStatus};
use alife_core::structural_plasticity::CoactivationEvidence;
use alife_core::{
    select_focal_targets, ActionId, AttentionSelectionPolicy, ChannelCommand,
    CognitiveContextFrame, CognitiveWorkReceipt, DendriticBranch, DendriticBranchSet,
    DendriticInputRef, DurationTicks, ExperienceSequenceId, HysteresisState, Intensity,
    MotorChannel, MotorCommandBundle, NormalizedScalar, OrganismId, PredictionTargetReceipt,
    SleepState, SleepTrigger, StructuralPlasticityConfig, StructuralPlasticityState,
    StableFocusIdentity, Validate, Vec3f, SLEEP_CONSOLIDATION_SCHEMA_VERSION,
    SUCCESSOR_FEATURE_ABI_V1,
};
use alife_game_app::{
    merge_gpu_checkpoint_manifest_entries, GpuBrainCheckpointWrite, GpuCheckpointAssetStore,
};
use alife_world::persistence::{
    AssetManifest, ExactCognitiveCheckpointState, GpuBrainSaveState, PortableSaveFile,
    V11_EXACT_COGNITIVE_STATE_SCHEMA_VERSION,
};

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
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../alife_world/tests/fixtures/p34/tiny_save.json");
    let save = PortableSaveFile::from_json_file(fixture).expect("checkpoint fixture");
    let mut save_state = save
        .creatures
        .into_iter()
        .find_map(|creature| creature.gpu_brain)
        .expect("fixture GPU brain checkpoint");
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
        SUCCESSOR_FEATURE_ABI_V1,
        vec![0.25, 0.75],
    )
    .expect("grounded prediction target");
    predictor.observe(&target).expect("non-default predictor update");

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

    let cognitive_work = CognitiveWorkReceipt::from_counters(
        2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37,
    )
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

    let dendritic_branches = DendriticBranchSet::new(vec![
        DendriticBranch::new(
            0,
            0.5,
            1.0,
            vec![DendriticInputRef::new(1, 0.75).expect("dendritic input")],
        )
        .expect("dendritic branch"),
    ])
    .expect("dendritic branch set");
    let mut structural_plasticity = StructuralPlasticityState::new(
        512,
        StructuralPlasticityConfig::default(),
    )
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
    assert_eq!(roundtrip_save.exact_cognitive_state, Some(asset_ref.clone()));

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
        .predict(source_digest, action, 2)
        .expect("unsaved control prediction");
    let restored_prediction = restored
        .predictor
        .predict(source_digest, action, 2)
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
