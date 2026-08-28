#![cfg(feature = "gpu-tests")]

use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use alife_archive::{GeneticArchiveInput, LineageLibrary, LineageLibraryConfig};
use alife_core::predictive::GroundedSuccessorPredictor;
use alife_core::{
    BrainCapacityClass, BrainGenome, BrainScaleTier, BrainTickStatus, CoactivationEvidence,
    CognitiveContextFrame, CognitiveWorkReceipt, Confidence, ConsolidationIntent,
    ConsolidationState, DevelopmentState, DriveSnapshot, EndocrineSnapshot, ExperienceSequenceId,
    FounderMode, FounderSelection, HomeostaticSnapshot, MemoryBankConfig, MemorySidecarState,
    NormalizedScalar, OrganismId, PhenotypeCompiler, PhenotypeCompilerInputs, PolicyBackend,
    ScaffoldContractError, SensorProfile, SensorProfileIdentity, SensoryAbiVersion, SleepPhase,
    SleepState, SleepTrigger, Tick, TopologicalMapConfig, TopologySidecar, Validate, Vec3f,
    SLEEP_CONSOLIDATION_SCHEMA_VERSION,
};
use alife_game_app::{
    materialize_founder_gpu_states, merge_gpu_checkpoint_manifest_entries, AppShellLaunchConfig,
    GameAppShellError, GpuBrainSidecarCapture, GpuCheckpointAssetStore, GpuDurableSaveManifest,
    GpuLiveBrainRuntime,
};
use alife_gpu_backend::{GpuClosedLoopBackend, GpuExactPopulationCapturePollV1};
use alife_runtime::{
    GpuAuthoritativeSession, GpuExactCheckpointTransactionContextV1, GpuSessionConsumerKind,
};
use alife_world::persistence::{
    AssetManifest, ExactCognitiveCheckpointState, GpuBrainSaveState, PortableSaveFile,
    GPU_BRAIN_SAVE_STATE_LEGACY_SCHEMA_VERSION, V11_EXACT_COGNITIVE_STATE_SCHEMA_VERSION,
};
use alife_world::{HeadlessScenarioBuilder, TrackedObjectRegistry};

fn unique_asset_root(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "alife-gpu-sleep-restore-{label}-{}-{nonce}",
        std::process::id()
    ))
}

fn copy_tree(source: &std::path::Path, destination: &std::path::Path) {
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

fn durable_gpu_state(root: &std::path::Path) -> GpuBrainSaveState {
    PortableSaveFile::from_json_file(root.join("tiny_save.json"))
        .unwrap()
        .creatures
        .into_iter()
        .find(|creature| creature.organism_id == OrganismId(1))
        .and_then(|creature| creature.gpu_brain)
        .expect("durable GPU checkpoint for organism 1")
}

#[test]
fn exact_population_capture_codec_matches_synchronous_completed_sleep_assets() {
    let asset_root = unique_asset_root("exact-population-completed-codec");
    fs::create_dir_all(&asset_root).unwrap();
    let store = GpuCheckpointAssetStore::new(&asset_root).unwrap();
    let capacity = BrainCapacityClass::production_for_id(BrainCapacityClass::N512_ID).unwrap();
    let genome = BrainGenome::scaffold(50_019, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
    let inputs = PhenotypeCompilerInputs::try_new(
        genome,
        &capacity,
        development,
        SensorProfile::PrivilegedAffordanceV1,
    )
    .unwrap();
    let phenotype = PhenotypeCompiler::compile_validated(&inputs, &capacity).unwrap();
    let organism_id = OrganismId(50_019);
    let checkpoint_tick = Tick::new(6_001);
    let sensor_profile = SensorProfileIdentity {
        profile_id: SensorProfile::PrivilegedAffordanceV1.into(),
        profile_schema_version: 1,
        sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
    };
    let memory = MemorySidecarState::new_profiled(
        organism_id,
        sensor_profile,
        MemoryBankConfig::new(64, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap(),
    )
    .unwrap();
    let topology =
        TopologySidecar::new_profiled(organism_id, sensor_profile, TopologicalMapConfig::default())
            .unwrap();
    let tracked_objects = TrackedObjectRegistry::new(50_019, 1_024)
        .unwrap()
        .save_state(organism_id)
        .unwrap();
    let language_grounding = alife_core::LanguageGroundingLedger::default();
    let life_statistics =
        alife_core::PassiveLifeStatistics::new(organism_id, checkpoint_tick).unwrap();
    let mut source = GpuAuthoritativeSession::new(
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter"),
        GpuSessionConsumerKind::Challenge,
    );
    let handle = source.insert_brain(organism_id, phenotype.clone()).unwrap();
    let replay = source.build_sleep_replay_batch(handle).unwrap();
    assert!(replay.events.is_empty());
    let request = source
        .prepare_sleep_consolidation(handle, ConsolidationIntent { cycle_id: 1 }, &replay)
        .unwrap();
    let job = source
        .submit_sleep_consolidation(handle, &request, &replay)
        .unwrap();
    let staged = source
        .poll_sleep_consolidation(handle, job)
        .unwrap()
        .unwrap();
    let sleep = SleepState {
        schema_version: SLEEP_CONSOLIDATION_SCHEMA_VERSION,
        phase: SleepPhase::Consolidating,
        phase_started_tick: checkpoint_tick,
        entered_sleep_tick: Some(checkpoint_tick),
        cycles_completed: 0,
        last_trigger: Some(SleepTrigger::FatigueThreshold),
        active_cycle_id: 1,
        last_consolidated_cycle_id: 0,
        consolidation: ConsolidationState::Completed {
            request,
            staged: staged.staged,
        },
    };
    sleep.validate_contract().unwrap();

    let context =
        GpuExactCheckpointTransactionContextV1::capture(source.backend(), &capacity).unwrap();
    let mut ticket = source
        .submit_exact_population_capture(checkpoint_tick, 1, &[handle])
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let capture = loop {
        match source.poll_exact_population_capture(&mut ticket).unwrap() {
            GpuExactPopulationCapturePollV1::Pending if std::time::Instant::now() < deadline => {
                std::thread::yield_now();
            }
            GpuExactPopulationCapturePollV1::Pending => {
                panic!("Completed codec capture exceeded the bounded poll deadline")
            }
            GpuExactPopulationCapturePollV1::Ready(capture) => break capture,
            GpuExactPopulationCapturePollV1::Failed(failure) => {
                panic!("Completed codec capture failed: {failure:?}")
            }
        }
    };
    let row = capture.rows().first().unwrap();
    assert_eq!(row.completed_sleep().unwrap().request, request);
    let sidecars = || GpuBrainSidecarCapture {
        sensor_profile,
        memory: &memory,
        topology: &topology,
        tracked_objects: tracked_objects.clone(),
        language_grounding: &language_grounding,
        life_statistics: &life_statistics,
        legacy_nano512_compatibility_receipt: None,
        retained_learning: None,
    };
    let mut captured_write = store
        .capture_brain_from_exact_population_capture(
            handle,
            &phenotype,
            &inputs,
            sleep,
            checkpoint_tick,
            None,
            &[],
            sidecars(),
            row,
            &context,
        )
        .unwrap();
    let mut synchronous_write = store
        .capture_brain_with_runtime_replay_state(
            &mut source,
            handle,
            &phenotype,
            &inputs,
            sleep,
            checkpoint_tick,
            None,
            &[],
            sidecars(),
        )
        .unwrap();
    let exact_state = |v11: alife_gpu_backend::GpuV11Checkpoint| ExactCognitiveCheckpointState {
        schema_version: V11_EXACT_COGNITIVE_STATE_SCHEMA_VERSION,
        organism_id,
        checkpoint_tick,
        cognitive_context: CognitiveContextFrame::empty(
            organism_id,
            ExperienceSequenceId(1),
            checkpoint_tick,
        )
        .unwrap(),
        predictor: GroundedSuccessorPredictor::default(),
        selected_motor_bundle: None,
        cognitive_work: CognitiveWorkReceipt::zero(),
        sleep_state: sleep,
        last_sleep_work: None,
        dendritic_branches: v11.dendritic_branches,
        structural_plasticity: v11.structural,
        structural_edit_receipts: Vec::new(),
        last_sleep_report: None,
    };
    let captured_exact = exact_state(row.identity().v11.clone());
    let synchronous_exact = exact_state(source.checkpoint_v11(handle).unwrap());
    captured_write
        .attach_exact_cognitive_state(&store, &captured_exact)
        .unwrap();
    synchronous_write
        .attach_exact_cognitive_state(&store, &synchronous_exact)
        .unwrap();

    assert_eq!(captured_exact, synchronous_exact);
    assert_eq!(captured_write, synchronous_write);
    assert!(captured_write
        .save_state
        .sleep_assets
        .replay_batch
        .is_some());
    assert!(captured_write
        .save_state
        .sleep_assets
        .lifetime_staging
        .is_some());
    assert!(captured_write
        .save_state
        .sleep_assets
        .fast_staging
        .is_some());
    assert!(captured_write
        .save_state
        .sleep_assets
        .eligibility_staging
        .is_some());
    assert!(captured_write
        .save_state
        .sleep_assets
        .replay_journal_staging
        .is_some());

    fs::remove_dir_all(asset_root).unwrap();
}

#[test]
fn awake_checkpoint_restores_every_mutable_gpu_bank_exactly() {
    let asset_root = unique_asset_root("awake");
    fs::create_dir_all(&asset_root).unwrap();
    let store = GpuCheckpointAssetStore::new(&asset_root).unwrap();
    let capacity = BrainCapacityClass::production_for_id(BrainCapacityClass::N512_ID).unwrap();
    let genome = BrainGenome::scaffold(50_001, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
    let inputs = PhenotypeCompilerInputs::try_new(
        genome,
        &capacity,
        development,
        SensorProfile::PrivilegedAffordanceV1,
    )
    .unwrap();
    let phenotype = PhenotypeCompiler::compile_validated(&inputs, &capacity).unwrap();
    let organism_id = OrganismId(1);
    let sensor_profile = SensorProfileIdentity {
        profile_id: SensorProfile::PrivilegedAffordanceV1.into(),
        profile_schema_version: 1,
        sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
    };
    let memory = MemorySidecarState::new_profiled(
        organism_id,
        sensor_profile,
        MemoryBankConfig::new(64, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap(),
    )
    .unwrap();
    let topology =
        TopologySidecar::new_profiled(organism_id, sensor_profile, TopologicalMapConfig::default())
            .unwrap();
    let tracked_objects = TrackedObjectRegistry::new(50_001, 1_024)
        .unwrap()
        .save_state(organism_id)
        .unwrap();
    let language_grounding = alife_core::LanguageGroundingLedger::default();
    let life_statistics = alife_core::PassiveLifeStatistics::new(organism_id, Tick::ZERO).unwrap();
    let mut source = GpuAuthoritativeSession::new(
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter"),
        GpuSessionConsumerKind::Challenge,
    );
    let handle = source.insert_brain(organism_id, phenotype.clone()).unwrap();
    let canonical_v11 = source.checkpoint_v11(handle).unwrap();
    assert_eq!(canonical_v11.structural.connection_count(), 0);
    assert!(canonical_v11.sparse_spans.is_empty());
    let mut canonical_write = store
        .capture_brain(
            &mut source,
            handle,
            &phenotype,
            &inputs,
            SleepState::awake_at(Tick::ZERO),
            Tick::ZERO,
            None,
            GpuBrainSidecarCapture {
                sensor_profile,
                memory: &memory,
                topology: &topology,
                tracked_objects: tracked_objects.clone(),
                language_grounding: &language_grounding,
                life_statistics: &life_statistics,
                legacy_nano512_compatibility_receipt: None,
                retained_learning: None,
            },
        )
        .unwrap();
    let canonical_exact = ExactCognitiveCheckpointState {
        schema_version: V11_EXACT_COGNITIVE_STATE_SCHEMA_VERSION,
        organism_id,
        checkpoint_tick: Tick::ZERO,
        cognitive_context: CognitiveContextFrame::empty(
            organism_id,
            ExperienceSequenceId(1),
            Tick::ZERO,
        )
        .unwrap(),
        predictor: GroundedSuccessorPredictor::default(),
        selected_motor_bundle: None,
        cognitive_work: CognitiveWorkReceipt::zero(),
        sleep_state: SleepState::awake_at(Tick::ZERO),
        last_sleep_work: None,
        dendritic_branches: canonical_v11.dendritic_branches,
        structural_plasticity: canonical_v11.structural,
        structural_edit_receipts: Vec::new(),
        last_sleep_report: None,
    };
    canonical_write
        .attach_exact_cognitive_state(&store, &canonical_exact)
        .unwrap();
    let mut canonical_manifest = AssetManifest::empty();
    merge_gpu_checkpoint_manifest_entries(
        &mut canonical_manifest,
        canonical_write.manifest_entries.clone(),
    )
    .unwrap();
    canonical_manifest.validate_with_root(&asset_root).unwrap();

    let mut missing_exact_v5 = canonical_write.save_state.clone();
    missing_exact_v5.schema_version = GPU_BRAIN_SAVE_STATE_LEGACY_SCHEMA_VERSION;
    missing_exact_v5.live_structural_topology = None;
    missing_exact_v5.exact_cognitive_state = None;
    let mut missing_exact_target = GpuAuthoritativeSession::new(
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter"),
        GpuSessionConsumerKind::Challenge,
    );
    assert!(
        store
            .restore_brain(
                &mut missing_exact_target,
                &canonical_manifest,
                &missing_exact_v5,
            )
            .is_err(),
        "shape-valid v5 without exact cognitive topology authority must fail closed"
    );

    let mut explicit_canonical_v5 = canonical_write.save_state.clone();
    explicit_canonical_v5.schema_version = GPU_BRAIN_SAVE_STATE_LEGACY_SCHEMA_VERSION;
    explicit_canonical_v5.live_structural_topology = None;
    let mut canonical_v5_target = GpuAuthoritativeSession::new(
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter"),
        GpuSessionConsumerKind::Challenge,
    );
    store
        .restore_brain(
            &mut canonical_v5_target,
            &canonical_manifest,
            &explicit_canonical_v5,
        )
        .expect("explicitly canonical v5 exact cognitive state remains readable");

    let structural_target = phenotype.candidate_decoder().motor_start();
    let structural_source = (structural_target + 1) % phenotype.neuron_count();
    let structural_work = source
        .apply_v11_structural_phase(
            handle,
            &[CoactivationEvidence {
                region: 0,
                source: structural_source,
                target: structural_target,
                coactivation: 100,
                eligibility: 0,
                concept_gap_support: 0,
            }],
        )
        .unwrap();
    assert_eq!(structural_work.structural.accepted_edges, 1);
    let source_topology = source.checkpoint_v11(handle).unwrap();
    assert_eq!(source_topology.sparse_spans.len(), 1);
    let source_snapshot = source.snapshot_brain(handle, Tick::ZERO).unwrap();
    assert_eq!(
        source_snapshot
            .clone()
            .into_parts()
            .lifetime_bank_0_bits
            .len(),
        phenotype.budgets().global.total_synapses as usize + 1
    );
    let write = store
        .capture_brain(
            &mut source,
            handle,
            &phenotype,
            &inputs,
            SleepState::awake_at(Tick::ZERO),
            Tick::ZERO,
            None,
            GpuBrainSidecarCapture {
                sensor_profile,
                memory: &memory,
                topology: &topology,
                tracked_objects,
                language_grounding: &language_grounding,
                life_statistics: &life_statistics,
                legacy_nano512_compatibility_receipt: None,
                retained_learning: None,
            },
        )
        .unwrap();
    let mut manifest = AssetManifest::empty();
    merge_gpu_checkpoint_manifest_entries(&mut manifest, write.manifest_entries).unwrap();
    manifest.validate_with_root(&asset_root).unwrap();

    let mut target = GpuAuthoritativeSession::new(
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter"),
        GpuSessionConsumerKind::Challenge,
    );
    let mut forged_legacy = write.save_state.clone();
    forged_legacy.schema_version = GPU_BRAIN_SAVE_STATE_LEGACY_SCHEMA_VERSION;
    forged_legacy.live_structural_topology = None;
    assert!(
        store
            .restore_brain(&mut target, &manifest, &forged_legacy)
            .is_err(),
        "a structurally divergent v6 checkpoint must not become implicit v5 state"
    );
    let restored = store
        .restore_brain(&mut target, &manifest, &write.save_state)
        .unwrap();
    assert_eq!(restored.sleep, SleepState::awake_at(Tick::ZERO));
    assert_eq!(restored.phenotype, phenotype);
    assert_eq!(restored.compiler_inputs, inputs);
    assert_eq!(restored.language_grounding, language_grounding);
    let restored_snapshot = target
        .snapshot_brain(restored.receipt.handle, Tick::ZERO)
        .unwrap();
    assert_eq!(
        restored_snapshot.canonical_digest(),
        write.checkpoint_digest
    );
    assert_eq!(
        restored_snapshot.canonical_digest(),
        source_snapshot.canonical_digest()
    );
    assert_eq!(
        target.checkpoint_v11(restored.receipt.handle).unwrap(),
        source_topology
    );

    fs::remove_dir_all(asset_root).unwrap();
}

fn assert_learned_awake_profile_roundtrip(
    sensor_profile: SensorProfile,
    label: &str,
    learning_seed: u64,
) {
    let asset_root = unique_asset_root(label);
    fs::create_dir_all(&asset_root).unwrap();
    let store = GpuCheckpointAssetStore::new(&asset_root).unwrap();
    let organism_id = OrganismId(1);
    let world = HeadlessScenarioBuilder::new(learning_seed)
        .agent("learner", organism_id, Vec3f::ZERO)
        .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.9)
        .hazard("hazard", Vec3f::new(-2.0, 0.0, 0.0), 0.7)
        .build()
        .unwrap();
    let mut source = GpuLiveBrainRuntime::new_profiled(
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter"),
        world,
        learning_seed,
        BrainScaleTier::Nano512,
        sensor_profile,
    )
    .unwrap();
    source.tick().unwrap();
    if sensor_profile == SensorProfile::PrivilegedAffordanceV1 {
        assert!(source
            .last_learning_receipts()
            .iter()
            .any(|receipt| receipt.fast_weights_changed > 0));
    }
    assert_eq!(
        source
            .sealed_patches()
            .last()
            .unwrap()
            .header()
            .sensor_profile
            .identity()
            .profile()
            .unwrap(),
        sensor_profile
    );
    let fast_before = source.active_fast_weights_for_test(organism_id).unwrap();
    let body_homeostasis = source.homeostasis_for_test(organism_id).unwrap();
    let world_at_checkpoint = source.world_snapshot();
    source
        .memory_sidecar_for_test(organism_id)
        .unwrap()
        .export_active_bank()
        .unwrap();
    let topology = source.topology_sidecar_for_test(organism_id).unwrap();
    topology.validate_contract().unwrap();
    topology.export_portable().unwrap();
    let write = source.checkpoint_brain(organism_id, &store).unwrap();
    let mut manifest = AssetManifest::empty();
    merge_gpu_checkpoint_manifest_entries(&mut manifest, write.manifest_entries).unwrap();
    assert_eq!(
        write.save_state.sensor_profile.profile().unwrap(),
        sensor_profile
    );

    let mut mismatched = write.save_state.clone();
    mismatched.sensor_profile.profile_id = match sensor_profile {
        SensorProfile::PrivilegedAffordanceV1 => SensorProfile::GroundedObjectSlotsV1.into(),
        SensorProfile::GroundedObjectSlotsV1 => SensorProfile::PrivilegedAffordanceV1.into(),
    };
    let mismatch = GpuLiveBrainRuntime::restore_with_checkpoints(
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter"),
        world_at_checkpoint.clone(),
        learning_seed,
        BrainScaleTier::Nano512,
        &store,
        &manifest,
        std::slice::from_ref(&mismatched),
    )
    .err()
    .expect("cross-profile restore must fail before allocation");
    assert!(
        matches!(
            mismatch,
            GameAppShellError::Core(ScaffoldContractError::SensorProfileMismatch)
        ),
        "unexpected cross-profile restore error: {mismatch:?}"
    );

    let mut restored = GpuLiveBrainRuntime::restore_with_checkpoints(
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter"),
        world_at_checkpoint,
        learning_seed,
        BrainScaleTier::Nano512,
        &store,
        &manifest,
        std::slice::from_ref(&write.save_state),
    )
    .unwrap();
    restored
        .set_homeostasis_for_test(organism_id, body_homeostasis)
        .unwrap();
    assert_eq!(
        restored.active_fast_weights_for_test(organism_id).unwrap(),
        fast_before
    );
    let restored_write = restored.checkpoint_brain(organism_id, &store).unwrap();
    assert_eq!(
        restored_write.save_state.sensor_profile,
        write.save_state.sensor_profile
    );
    assert_eq!(restored_write.save_state.memory, write.save_state.memory);
    assert_eq!(
        restored_write.save_state.topology,
        write.save_state.topology
    );
    assert_eq!(
        restored_write.save_state.tracked_objects,
        write.save_state.tracked_objects
    );

    let source_summary = source.tick().unwrap();
    let restored_summary = restored.tick().unwrap();
    assert_eq!(
        source_summary[0].selected_action_id,
        restored_summary[0].selected_action_id
    );
    let source_evidence = source
        .sealed_patches()
        .last()
        .unwrap()
        .decision()
        .neural_evidence()
        .unwrap();
    let restored_evidence = restored
        .sealed_patches()
        .last()
        .unwrap()
        .decision()
        .neural_evidence()
        .unwrap();
    assert_eq!(
        source_evidence.candidate_index,
        restored_evidence.candidate_index
    );
    assert_eq!(
        source_evidence.logit.to_bits(),
        restored_evidence.logit.to_bits()
    );

    fs::remove_dir_all(asset_root).unwrap();
}

#[test]
fn learned_awake_runtime_retains_fast_weights_and_next_decision() {
    assert_learned_awake_profile_roundtrip(
        SensorProfile::PrivilegedAffordanceV1,
        "learned-awake-privileged",
        7_701,
    );
}

#[test]
fn grounded_awake_runtime_restores_exact_profile_sidecars_and_next_decision() {
    assert_learned_awake_profile_roundtrip(
        SensorProfile::GroundedObjectSlotsV1,
        "learned-awake-grounded",
        7_702,
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RestoreCase {
    Awake,
    EnteringSleep,
    ConsolidatingNone,
    Pending,
    Prepared,
    Submitted,
    Completed,
    Committed,
    Waking,
    ForcedRecovery,
}

impl RestoreCase {
    const ALL: [Self; 10] = [
        Self::Awake,
        Self::EnteringSleep,
        Self::ConsolidatingNone,
        Self::Pending,
        Self::Prepared,
        Self::Submitted,
        Self::Completed,
        Self::Committed,
        Self::Waking,
        Self::ForcedRecovery,
    ];

    const fn expects_remaining_swap(self) -> bool {
        matches!(
            self,
            Self::EnteringSleep
                | Self::ConsolidatingNone
                | Self::Pending
                | Self::Prepared
                | Self::Submitted
                | Self::Completed
                | Self::ForcedRecovery
        )
    }

    fn matches(self, state: SleepState) -> bool {
        match self {
            Self::Awake => state.phase == SleepPhase::Awake,
            Self::EnteringSleep => state.phase == SleepPhase::EnteringSleep,
            Self::ConsolidatingNone => {
                state.phase == SleepPhase::Consolidating
                    && state.consolidation == ConsolidationState::None
            }
            Self::Pending => matches!(state.consolidation, ConsolidationState::Pending { .. }),
            Self::Prepared => matches!(state.consolidation, ConsolidationState::Prepared { .. }),
            Self::Submitted => {
                matches!(state.consolidation, ConsolidationState::Submitted { .. })
            }
            Self::Completed => {
                matches!(state.consolidation, ConsolidationState::Completed { .. })
            }
            Self::Committed => {
                state.phase == SleepPhase::Consolidating
                    && matches!(state.consolidation, ConsolidationState::Committed { .. })
            }
            Self::Waking => state.phase == SleepPhase::Waking,
            Self::ForcedRecovery => state.phase == SleepPhase::ForcedRecoverySleep,
        }
    }
}

fn learned_runtime(tier: BrainScaleTier) -> GpuLiveBrainRuntime {
    const LEARNING_SEED: u64 = 7_701;
    let organism_id = OrganismId(1);
    let world = HeadlessScenarioBuilder::new(LEARNING_SEED)
        .agent("learner", organism_id, Vec3f::ZERO)
        .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.9)
        .hazard("hazard", Vec3f::new(-2.0, 0.0, 0.0), 0.7)
        .build()
        .unwrap();
    let mut runtime = GpuLiveBrainRuntime::new(
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter"),
        world,
        LEARNING_SEED,
        tier,
    )
    .unwrap();
    runtime.tick().unwrap();
    assert!(runtime
        .last_learning_receipts()
        .iter()
        .any(|receipt| receipt.fast_weights_changed > 0));
    runtime
}

fn normal_sleep_homeostasis(tick: Tick) -> HomeostaticSnapshot {
    let mut drives = DriveSnapshot::baseline();
    drives.fatigue = 0.99;
    let mut hormones = EndocrineSnapshot::baseline();
    hormones.sleep_pressure = 0.99;
    HomeostaticSnapshot::new(tick, drives, hormones).unwrap()
}

fn forced_recovery_homeostasis(tick: Tick) -> HomeostaticSnapshot {
    let mut drives = DriveSnapshot::baseline();
    drives.fear = 0.95;
    let mut hormones = EndocrineSnapshot::baseline();
    hormones.adrenaline = 0.98;
    hormones.cortisol = 0.96;
    HomeostaticSnapshot::new(tick, drives, hormones).unwrap()
}

fn advance_runtime_to_case(
    runtime: &mut GpuLiveBrainRuntime,
    tier: BrainScaleTier,
    case: RestoreCase,
) {
    if case == RestoreCase::Awake || case == RestoreCase::ConsolidatingNone {
        return;
    }
    let organism_id = OrganismId(1);
    let tick = runtime.world_tick_for_test();
    let homeostasis = if case == RestoreCase::ForcedRecovery {
        forced_recovery_homeostasis(tick)
    } else {
        normal_sleep_homeostasis(tick)
    };
    runtime
        .set_homeostasis_for_test(organism_id, homeostasis)
        .unwrap();

    for _ in 0..96 {
        let state_before = runtime.sleep_state_for_test(organism_id).unwrap();
        let learning_before = runtime.learning_state_for_test(organism_id).unwrap();
        runtime.sleep_replay_for_test(organism_id).unwrap_or_else(|error| {
            panic!(
                "invalid replay before advancing {tier:?} {case:?} from {state_before:?}; learning={learning_before:?}: {error:?}"
            )
        });
        let summaries = runtime.tick().unwrap_or_else(|error| {
            panic!("failed to advance {tier:?} {case:?} from {state_before:?}: {error:?}")
        });
        assert_eq!(summaries[0].status, BrainTickStatus::SafeIdle);
        assert_eq!(summaries[0].selected_action_id, None);
        assert!(!summaries[0].patch_sealed);
        let state = runtime.sleep_state_for_test(organism_id).unwrap();
        if case.matches(state) {
            return;
        }
    }
    panic!("failed to reach restore case {case:?}");
}

fn synthetic_consolidating_none(checkpoint_tick: Tick) -> SleepState {
    let state = SleepState {
        schema_version: SLEEP_CONSOLIDATION_SCHEMA_VERSION,
        phase: SleepPhase::Consolidating,
        phase_started_tick: checkpoint_tick,
        entered_sleep_tick: Some(checkpoint_tick),
        cycles_completed: 0,
        last_trigger: Some(SleepTrigger::FatigueThreshold),
        active_cycle_id: 1,
        last_consolidated_cycle_id: 0,
        consolidation: ConsolidationState::None,
    };
    state.validate_contract().unwrap();
    state
}

fn assert_restore_case(tier: BrainScaleTier, case: RestoreCase) {
    let organism_id = OrganismId(1);
    let asset_root = unique_asset_root(&format!("{tier:?}-{case:?}"));
    fs::create_dir_all(&asset_root).unwrap();
    let store = GpuCheckpointAssetStore::new(&asset_root).unwrap();
    let mut source = learned_runtime(tier);
    advance_runtime_to_case(&mut source, tier, case);
    let homeostasis = source.homeostasis_for_test(organism_id).unwrap();
    let world = source.world_snapshot();
    let mut write = source.checkpoint_brain(organism_id, &store).unwrap();
    if case == RestoreCase::ConsolidatingNone {
        write.save_state.sleep = synthetic_consolidating_none(write.save_state.checkpoint_tick);
        write.save_state.validate().unwrap();
    }
    assert!(
        case.matches(write.save_state.sleep),
        "checkpoint case {case:?}"
    );
    let genetic_identity = write.save_state.immutable_phenotype.clone();
    let input_generation = write.save_state.active_weight_generation;
    let dispatches_before = source.completed_dispatch_count_for_test();
    let mut manifest = AssetManifest::empty();
    merge_gpu_checkpoint_manifest_entries(&mut manifest, write.manifest_entries).unwrap();

    let mut restored = GpuLiveBrainRuntime::restore_with_checkpoints(
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter"),
        world,
        7_701,
        tier,
        &store,
        &manifest,
        std::slice::from_ref(&write.save_state),
    )
    .unwrap();
    restored
        .set_homeostasis_for_test(organism_id, homeostasis)
        .unwrap();

    if case != RestoreCase::Awake {
        let restored_dispatches = restored.completed_dispatch_count_for_test();
        let mut woke = false;
        for _ in 0..128 {
            let state_before = restored.sleep_state_for_test(organism_id).unwrap();
            if state_before.phase == SleepPhase::Awake {
                woke = true;
                break;
            }
            let summaries = restored.tick().unwrap();
            assert_eq!(
                summaries[0].status,
                BrainTickStatus::SafeIdle,
                "{tier:?} {case:?}"
            );
            assert_eq!(summaries[0].selected_action_id, None, "{tier:?} {case:?}");
            assert!(!summaries[0].patch_sealed, "{tier:?} {case:?}");
            assert_eq!(
                restored.completed_dispatch_count_for_test(),
                restored_dispatches,
                "{tier:?} {case:?}"
            );
        }
        assert!(woke, "restore case {tier:?} {case:?} did not wake");
        assert_eq!(
            restored
                .sleep_state_for_test(organism_id)
                .unwrap()
                .last_consolidated_cycle_id,
            1,
            "{tier:?} {case:?}"
        );
    }

    let expected_generation = input_generation + u64::from(case.expects_remaining_swap());
    assert_eq!(
        restored
            .learning_state_for_test(organism_id)
            .unwrap()
            .active_weight_generation,
        expected_generation,
        "{tier:?} {case:?}"
    );
    let post = restored.checkpoint_brain(organism_id, &store).unwrap();
    assert_eq!(
        post.save_state.immutable_phenotype, genetic_identity,
        "{tier:?} {case:?}"
    );
    if case != RestoreCase::Awake {
        assert!(restored
            .active_lifetime_weights_for_test(organism_id)
            .unwrap()
            .iter()
            .any(|value| *value != 0.0));
    }

    let tick = restored.world_tick_for_test();
    restored
        .set_homeostasis_for_test(organism_id, HomeostaticSnapshot::baseline(tick))
        .unwrap();
    let resumed_dispatches = restored.completed_dispatch_count_for_test();
    let resumed = restored.tick().unwrap();
    assert_eq!(
        resumed[0].status,
        BrainTickStatus::Normal,
        "{tier:?} {case:?}"
    );
    assert!(resumed[0].patch_sealed, "{tier:?} {case:?}");
    assert_eq!(
        restored.completed_dispatch_count_for_test(),
        resumed_dispatches + 1,
        "{tier:?} {case:?}"
    );
    assert!(dispatches_before >= 1);

    fs::remove_dir_all(asset_root).unwrap();
}

#[test]
fn n512_every_sleep_phase_restores_with_exact_remaining_gpu_work() {
    for case in RestoreCase::ALL {
        assert_restore_case(BrainScaleTier::Nano512, case);
    }
}

#[test]
fn n1024_and_n2048_restore_submitted_lost_jobs_and_completed_staging() {
    for tier in [BrainScaleTier::Small1024, BrainScaleTier::Standard2048] {
        for case in [RestoreCase::Submitted, RestoreCase::Completed] {
            assert_restore_case(tier, case);
        }
    }
}

#[test]
fn durable_mind_clone_keeps_consolidated_learning_and_clears_transient_world_state() {
    let asset_root = unique_asset_root("durable-founder-clone");
    fs::create_dir_all(&asset_root).unwrap();
    let store = GpuCheckpointAssetStore::new(&asset_root).unwrap();
    let organism_id = OrganismId(1);
    let target_organism_id = OrganismId(91);
    let mut source = learned_runtime(BrainScaleTier::Nano512);
    advance_runtime_to_case(&mut source, BrainScaleTier::Nano512, RestoreCase::Waking);
    assert!(source
        .active_lifetime_weights_for_test(organism_id)
        .unwrap()
        .iter()
        .any(|weight| *weight != 0.0));
    let source_write = source.checkpoint_brain(organism_id, &store).unwrap();
    let source_memory_count = source_write.save_state.memory.summary.record_count;
    let source_topology_counts = source_write.save_state.topology.counts;
    let mut manifest = AssetManifest::empty();
    merge_gpu_checkpoint_manifest_entries(&mut manifest, source_write.manifest_entries).unwrap();

    let mut clone_session = GpuAuthoritativeSession::new(
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter"),
        GpuSessionConsumerKind::Challenge,
    );
    let cloned = store
        .clone_durable_founder(
            &mut clone_session,
            &manifest,
            &source_write.save_state,
            target_organism_id,
            88_001,
            Tick::ZERO,
        )
        .unwrap();
    assert_eq!(cloned.checkpoint.save_state.organism_id, target_organism_id);
    assert_eq!(
        cloned.checkpoint.save_state.sleep,
        SleepState::awake_at(Tick::ZERO)
    );
    assert_eq!(
        cloned.checkpoint.save_state.memory.summary.record_count,
        source_memory_count
    );
    assert_eq!(
        cloned.checkpoint.save_state.topology.counts,
        source_topology_counts
    );
    assert_eq!(
        cloned.checkpoint.save_state.memory.summary.organism_id_raw,
        target_organism_id.raw()
    );
    assert_eq!(
        cloned.checkpoint.save_state.topology.organism_id_raw,
        target_organism_id.raw()
    );
    assert_eq!(
        cloned.checkpoint.save_state.tracked_objects.world_seed,
        88_001
    );
    assert!(cloned
        .checkpoint
        .save_state
        .tracked_objects
        .records
        .is_empty());
    assert!(cloned.checkpoint.save_state.pending_eligibility.is_none());
    assert!(cloned
        .checkpoint
        .save_state
        .pending_experience_transaction
        .is_none());

    merge_gpu_checkpoint_manifest_entries(
        &mut manifest,
        cloned.checkpoint.manifest_entries.clone(),
    )
    .unwrap();
    let mut restored_session = GpuAuthoritativeSession::new(
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter"),
        GpuSessionConsumerKind::Challenge,
    );
    let restored = store
        .restore_brain(
            &mut restored_session,
            &manifest,
            &cloned.checkpoint.save_state,
        )
        .unwrap();
    let parts = restored_session
        .snapshot_brain(restored.receipt.handle, Tick::ZERO)
        .unwrap()
        .into_parts();
    let lifetime = if parts.active_weight_bank == 0 {
        &parts.lifetime_bank_0_bits
    } else {
        &parts.lifetime_bank_1_bits
    };
    assert!(lifetime.iter().any(|bits| *bits != 0));
    assert!(parts.fast_bank_0_bits.iter().all(|bits| *bits == 0));
    assert!(parts.fast_bank_1_bits.iter().all(|bits| *bits == 0));
    assert!(parts.activation_a_bits.iter().all(|bits| *bits == 0));
    assert!(parts.activation_b_bits.iter().all(|bits| *bits == 0));
    assert!(parts
        .recurrent_eligibility_bank_0_bits
        .iter()
        .chain(&parts.recurrent_eligibility_bank_1_bits)
        .chain(&parts.decoder_eligibility_bank_0_bits)
        .chain(&parts.decoder_eligibility_bank_1_bits)
        .all(|bits| *bits == 0));

    fs::remove_dir_all(asset_root).unwrap();
}

#[test]
fn archived_genetic_founder_builds_a_launch_ready_gpu_save() {
    let archive_root = unique_asset_root("genetic-founder-archive");
    let save_root = unique_asset_root("genetic-founder-save");
    fs::create_dir_all(&archive_root).unwrap();
    copy_tree(
        std::path::Path::new("../alife_world/tests/fixtures/p34"),
        &save_root,
    );
    let capacity = BrainCapacityClass::n512();
    let genome = BrainGenome::scaffold(92_001, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.25).unwrap());
    let phenotype = PhenotypeCompiler::compile(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let mut library =
        LineageLibrary::open(LineageLibraryConfig::profile_default(&archive_root)).unwrap();
    let source_manifest = library
        .archive_birth(GeneticArchiveInput {
            source_run_id: "gpu-founder-source",
            organism_id: OrganismId(82),
            birth_tick: Tick::ZERO,
            genome: &genome,
            phenotype: &phenotype,
            foundation_asset_bytes: None,
        })
        .unwrap();
    let cohort = library
        .resolve_founder_cohort(
            "founder-world",
            4242,
            &[FounderSelection {
                source_manifest_digest: source_manifest,
                mode: FounderMode::GeneticFounder,
            }],
        )
        .unwrap();

    let mut base = PortableSaveFile::from_json_file(save_root.join("tiny_save.json")).unwrap();
    let mut empty_world = base.restore_headless_world().unwrap();
    empty_world.remove_organism(OrganismId(1)).unwrap();
    base.replace_headless_world_snapshot(&empty_world).unwrap();
    base.save_id = "founder-world".to_string();
    base.gpu_runtime = None;
    base.creatures.clear();
    let skeleton = library
        .create_new_save_from_founders(base, &save_root, &cohort)
        .unwrap();
    assert!(skeleton.creatures[0].gpu_brain.is_none());

    let save = materialize_founder_gpu_states(
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter"),
        skeleton,
        &save_root,
        &cohort,
    )
    .unwrap();
    save.validate_with_asset_root(&save_root).unwrap();
    let state = save.creatures[0]
        .gpu_brain
        .as_ref()
        .expect("genetic founder was captured on GPU");
    assert_eq!(state.phenotype_hash, phenotype.phenotype_hash());
    assert_eq!(state.memory.summary.record_count, 0);
    assert_eq!(state.topology.counts, alife_core::TopologyCounts::default());
    assert_eq!(
        state.language_grounding,
        alife_core::LanguageGroundingLedger::default()
    );

    let store = GpuCheckpointAssetStore::new(&save_root).unwrap();
    let restored = GpuLiveBrainRuntime::restore_with_checkpoints(
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter"),
        save.restore_headless_world().unwrap(),
        save.deterministic_seed,
        BrainScaleTier::Nano512,
        &store,
        &save.assets,
        std::slice::from_ref(state),
    )
    .unwrap();
    assert_eq!(
        restored
            .homeostasis_for_test(save.creatures[0].organism_id)
            .unwrap(),
        HomeostaticSnapshot::baseline(save.world.tick)
    );

    drop(library);
    fs::remove_dir_all(archive_root).unwrap();
    fs::remove_dir_all(save_root).unwrap();
}

#[test]
fn manual_portable_checkpoint_atomically_restores_awake_fast_learning() {
    let root = unique_asset_root("manual-awake");
    copy_tree(
        std::path::Path::new("../alife_world/tests/fixtures/p34"),
        &root,
    );
    let save_path = root.join("tiny_save.json");
    let mut source_save = PortableSaveFile::from_json_file(&save_path).unwrap();
    let seed = source_save.deterministic_seed;
    let learning_world = HeadlessScenarioBuilder::new(seed)
        .agent("learner", OrganismId(1), Vec3f::ZERO)
        .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.9)
        .hazard("hazard", Vec3f::new(-2.0, 0.0, 0.0), 0.7)
        .build()
        .unwrap();
    source_save
        .replace_headless_world_snapshot(&learning_world)
        .unwrap();
    // This case deliberately starts a new neural life over the replacement
    // world. The canonical P34 fixture now carries an exact-resume checkpoint,
    // so retaining it here would combine a tick-zero world with the fixture's
    // older GPU generation instead of exercising fresh birth -> manual save.
    source_save.gpu_runtime = None;
    for creature in &mut source_save.creatures {
        creature.gpu_brain = None;
    }
    source_save
        .assets
        .entries
        .retain(|entry| !entry.asset_id.starts_with("gpu-brain."));
    let stale_gpu_assets = root.join("gpu-brain");
    if stale_gpu_assets.exists() {
        fs::remove_dir_all(stale_gpu_assets).unwrap();
    }
    source_save.to_json_file(&save_path).unwrap();
    let launch = AppShellLaunchConfig::from_p34_fixture_root(&root)
        .with_brain_policy(PolicyBackend::NeuralClosedLoopGpu);
    let organism_id = OrganismId(1);
    let mut runtime = GpuLiveBrainRuntime::from_p34_launch(
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter"),
        &launch,
    )
    .unwrap();
    assert!(runtime
        .session_authority()
        .latest_durable_checkpoint()
        .is_some());
    runtime.tick().unwrap();
    let fast_before = runtime.active_fast_weights_for_test(organism_id).unwrap();
    assert!(fast_before.iter().any(|value| *value != 0.0));

    let checkpointed = runtime.capture_portable_checkpoint().unwrap();
    assert_eq!(checkpointed.world.tick, runtime.world_tick_for_test());
    let state = checkpointed
        .creatures
        .iter()
        .find(|creature| creature.organism_id == organism_id)
        .and_then(|creature| creature.gpu_brain.as_ref())
        .expect("manual save carries the exact GPU brain checkpoint");
    assert_eq!(state.checkpoint_tick, checkpointed.world.tick);
    assert!(state.pending_eligibility.is_none());
    assert!(state.pending_experience_transaction.is_none());

    let manual_path = root.join("manual_awake.json");
    GpuDurableSaveManifest::publish_snapshot(&manual_path, &root, &checkpointed).unwrap();
    drop(runtime);

    let mut restore_launch = launch.clone();
    restore_launch.save_path = manual_path;
    let mut restored = GpuLiveBrainRuntime::from_p34_launch(
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter"),
        &restore_launch,
    )
    .unwrap();
    assert_eq!(
        restored
            .session_authority()
            .latest_durable_checkpoint()
            .expect("restored session retains its durable manifest")
            .checkpoint_tick,
        state.checkpoint_tick
    );
    assert_eq!(
        restored.active_fast_weights_for_test(organism_id).unwrap(),
        fast_before
    );
    let telemetry = restored.authority_telemetry();
    assert_eq!(telemetry.checkpoint_tick, Some(state.checkpoint_tick.raw()));
    assert_eq!(telemetry.checkpoint_sleep_phase, "Awake");
    assert_eq!(telemetry.checkpoint_consolidation_state, "None");
    assert_eq!(telemetry.recovery_status, "GPU required");

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn production_save_persists_recovered_submission_and_atomically_promotes_completed_assets() {
    let root = unique_asset_root("durable-production");
    copy_tree(
        std::path::Path::new("../alife_world/tests/fixtures/p34"),
        &root,
    );
    let launch = AppShellLaunchConfig::from_p34_fixture_root(&root)
        .with_brain_policy(PolicyBackend::NeuralClosedLoopGpu);
    let organism_id = OrganismId(1);
    let mut runtime = GpuLiveBrainRuntime::from_p34_launch(
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter"),
        &launch,
    )
    .unwrap();
    let initial = durable_gpu_state(&root);
    assert_eq!(initial.checkpoint_tick, runtime.world_tick_for_test());
    assert_eq!(initial.sleep.phase, SleepPhase::Awake);
    assert_eq!(initial.sleep.consolidation, ConsolidationState::None);

    runtime.tick().unwrap();
    let tick = runtime.world_tick_for_test();
    runtime
        .set_homeostasis_for_test(organism_id, normal_sleep_homeostasis(tick))
        .unwrap();

    let submitted = loop {
        runtime.tick().unwrap();
        let state = runtime.sleep_state_for_test(organism_id).unwrap();
        if matches!(state.consolidation, ConsolidationState::Submitted { .. }) {
            break state;
        }
    };
    let durable_submitted = durable_gpu_state(&root);
    assert_eq!(durable_submitted.sleep, submitted);
    let lost_job_id = match durable_submitted.sleep.consolidation {
        ConsolidationState::Submitted { job_id, .. } => job_id,
        other => panic!("expected durable Submitted state, got {other:?}"),
    };
    drop(runtime);

    let mut recovered = GpuLiveBrainRuntime::from_p34_launch(
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter"),
        &launch,
    )
    .unwrap();
    recovered.tick().unwrap();
    let recovered_state = recovered.sleep_state_for_test(organism_id).unwrap();
    let recovered_job_id = match recovered_state.consolidation {
        ConsolidationState::Submitted { job_id, .. } => job_id,
        other => panic!("lost job was not recovered as Submitted: {other:?}"),
    };
    assert_ne!(recovered_job_id, lost_job_id);
    assert_eq!(durable_gpu_state(&root).sleep, recovered_state);

    let completed = loop {
        recovered.tick().unwrap();
        let state = recovered.sleep_state_for_test(organism_id).unwrap();
        if matches!(state.consolidation, ConsolidationState::Completed { .. }) {
            break durable_gpu_state(&root);
        }
    };
    assert_eq!(
        completed.sleep,
        recovered.sleep_state_for_test(organism_id).unwrap()
    );
    let lifetime_staging = completed.sleep_assets.lifetime_staging.clone().unwrap();
    let fast_staging = completed.sleep_assets.fast_staging.clone().unwrap();
    let eligibility_staging = completed.sleep_assets.eligibility_staging.clone().unwrap();
    let replay_staging = completed
        .sleep_assets
        .replay_journal_staging
        .clone()
        .unwrap();
    let completed_generation = completed.active_weight_generation;

    recovered.tick().unwrap();
    let committed = durable_gpu_state(&root);
    assert!(matches!(
        committed.sleep.consolidation,
        ConsolidationState::Committed { .. }
    ));
    assert_eq!(committed.lifetime_weights, lifetime_staging);
    assert_eq!(committed.fast_weights, fast_staging);
    assert_eq!(committed.eligibility, eligibility_staging);
    assert_eq!(committed.replay_journal, replay_staging);
    assert!(committed.sleep_assets.lifetime_staging.is_none());
    assert!(committed.sleep_assets.fast_staging.is_none());
    assert!(committed.sleep_assets.eligibility_staging.is_none());
    assert!(committed.sleep_assets.replay_journal_staging.is_none());
    assert!(committed.sleep_assets.replay_batch.is_none());
    assert_eq!(committed.active_weight_generation, completed_generation + 1);
    drop(recovered);

    let mut after_cas = GpuLiveBrainRuntime::from_p34_launch(
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter"),
        &launch,
    )
    .unwrap();
    for _ in 0..16 {
        if after_cas.sleep_state_for_test(organism_id).unwrap().phase == SleepPhase::Awake {
            break;
        }
        after_cas.tick().unwrap();
    }
    assert_eq!(
        after_cas.sleep_state_for_test(organism_id).unwrap().phase,
        SleepPhase::Awake
    );
    assert_eq!(
        after_cas
            .learning_state_for_test(organism_id)
            .unwrap()
            .active_weight_generation,
        committed.active_weight_generation,
        "restart after manifest CAS must not promote a second time",
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn device_loss_fail_stops_live_actions_and_retains_the_last_durable_reference() {
    let root = unique_asset_root("device-loss-fail-stop");
    copy_tree(
        std::path::Path::new("../alife_world/tests/fixtures/p34"),
        &root,
    );
    let save_path = root.join("tiny_save.json");
    let mut source_save = PortableSaveFile::from_json_file(&save_path).unwrap();
    source_save.gpu_runtime = None;
    for creature in &mut source_save.creatures {
        creature.gpu_brain = None;
    }
    source_save
        .assets
        .entries
        .retain(|entry| !entry.asset_id.starts_with("gpu-brain."));
    let stale_gpu_assets = root.join("gpu-brain");
    if stale_gpu_assets.exists() {
        fs::remove_dir_all(stale_gpu_assets).unwrap();
    }
    source_save.to_json_file(&save_path).unwrap();
    let launch = AppShellLaunchConfig::from_p34_fixture_root(&root)
        .with_brain_policy(PolicyBackend::NeuralClosedLoopGpu);
    let mut runtime = GpuLiveBrainRuntime::from_p34_launch(
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter"),
        &launch,
    )
    .unwrap();
    let durable_before = runtime
        .session_authority()
        .latest_durable_checkpoint()
        .cloned()
        .expect("production launch has a durable checkpoint reference");

    runtime.force_device_lost_after_next_submit_for_test();
    assert!(matches!(
        runtime.tick(),
        Err(GameAppShellError::Core(
            ScaffoldContractError::NeuralBackendUnavailable
        ))
    ));
    assert_eq!(
        runtime.session_authority().latest_durable_checkpoint(),
        Some(&durable_before)
    );
    assert!(matches!(
        runtime.tick(),
        Err(GameAppShellError::Core(
            ScaffoldContractError::NeuralBackendUnavailable
        ))
    ));

    fs::remove_dir_all(root).unwrap();
}
