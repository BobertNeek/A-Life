#![cfg(feature = "gpu-tests")]

use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use alife_archive::{GeneticArchiveInput, LineageLibrary, LineageLibraryConfig};
use alife_core::predictive::GroundedSuccessorPredictor;
use alife_core::{
    BrainCapacityClass, BrainGenome, BrainScaleTier, CoactivationEvidence, CognitiveContextFrame,
    CognitiveWorkReceipt, Confidence, ConsolidationIntent, ConsolidationState, DevelopmentState,
    ExperienceSequenceId, FounderMode, FounderSelection, HomeostaticSnapshot, MemoryBankConfig,
    MemorySidecarState, NormalizedScalar, OrganismId, PhenotypeCompiler, PhenotypeCompilerInputs,
    PolicyBackend, ScaffoldContractError, SensorProfile, SensorProfileIdentity, SensoryAbiVersion,
    SleepPhase, SleepState, SleepTrigger, Tick, TopologicalMapConfig, TopologySidecar, Validate,
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
    AssetManifest, ExactCognitiveCheckpointState, PortableSaveFile,
    GPU_BRAIN_SAVE_STATE_LEGACY_SCHEMA_VERSION, V11_EXACT_COGNITIVE_STATE_SCHEMA_VERSION,
};
use alife_world::TrackedObjectRegistry;

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
        private_semantic_prior: None,
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
        private_semantic_prior: None,
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

fn drain_persistence(runtime: &mut GpuLiveBrainRuntime) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
    while !runtime.persistence_idle_for_shutdown_for_test() {
        assert!(
            std::time::Instant::now() < deadline,
            "checkpoint publication did not drain"
        );
        runtime.poll_persistence_for_shutdown_for_test().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

fn canonical_runtime(
    root: &std::path::Path,
    seed: u64,
    sensor_profile: SensorProfile,
) -> GpuLiveBrainRuntime {
    let mut world_config = alife_world::CanonicalNewGameConfig::phase3(seed, 4).unwrap();
    world_config.sensor_profile = sensor_profile;
    let asset = alife_core::FoundationWeightAsset::builtin_nano512_v1(sensor_profile).unwrap();
    let game = alife_world::create_canonical_new_game(&world_config, &asset).unwrap();
    let mut config =
        alife_world::RuntimeConfig::deterministic_default(seed, BrainScaleTier::Nano512);
    config.features.gpu_backend_enabled = true;
    config.brain_policy.policy = PolicyBackend::NeuralClosedLoopGpu;
    fs::write(
        root.join("tiny_config.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    fs::write(
        root.join("tiny_asset_manifest.json"),
        serde_json::to_vec(&AssetManifest::empty()).unwrap(),
    )
    .unwrap();
    let save = PortableSaveFile::from_headless_world(
        format!("gpu-restore-{seed}"),
        &game.world,
        config,
        AssetManifest::empty(),
        game.creatures,
    )
    .unwrap();
    let mut runtime = GpuLiveBrainRuntime::new_profiled(
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter"),
        game.world,
        seed,
        BrainScaleTier::Nano512,
        sensor_profile,
    )
    .unwrap();
    runtime
        .attach_durable_checkpoint_boundary(root.join("live.json"), root, save)
        .unwrap();
    runtime
}

fn assert_learned_awake_profile_roundtrip(sensor_profile: SensorProfile, label: &str, seed: u64) {
    let root = unique_asset_root(label);
    fs::create_dir_all(&root).unwrap();
    let organism_id = OrganismId(1);
    let mut source = canonical_runtime(&root, seed, sensor_profile);
    source.tick().unwrap();
    assert!(source
        .last_learning_receipts()
        .iter()
        .any(|receipt| receipt.fast_weights_changed > 0));
    let fast_before = source.active_fast_weights_for_test(organism_id).unwrap();
    let checkpoint = source.capture_portable_checkpoint().unwrap();
    let brain = checkpoint
        .creatures
        .iter()
        .find(|creature| creature.organism_id == organism_id)
        .unwrap()
        .gpu_brain
        .as_ref()
        .unwrap();
    assert_eq!(brain.sensor_profile.profile().unwrap(), sensor_profile);
    let path = root.join("learned.json");
    GpuDurableSaveManifest::publish_snapshot(&path, &root, &checkpoint).unwrap();
    let mut launch = AppShellLaunchConfig::from_p34_fixture_root(&root)
        .with_brain_policy(PolicyBackend::NeuralClosedLoopGpu);
    launch.save_path = path;
    let mut restored = GpuLiveBrainRuntime::from_p34_launch(
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter"),
        &launch,
    )
    .unwrap();
    assert_eq!(
        restored.active_fast_weights_for_test(organism_id).unwrap(),
        fast_before
    );
    assert_eq!(
        restored
            .world_snapshot()
            .canonical_signature_digest()
            .unwrap(),
        source
            .world_snapshot()
            .canonical_signature_digest()
            .unwrap()
    );
    let recaptured = restored.capture_portable_checkpoint().unwrap();
    assert_eq!(recaptured.creatures, checkpoint.creatures);
    let source_summary = source.tick().unwrap();
    let restored_summary = restored.tick().unwrap();
    assert_eq!(source_summary.len(), restored_summary.len());
    for (original, resumed) in source_summary.iter().zip(&restored_summary) {
        assert!(original.patch_sealed && resumed.patch_sealed);
        assert_eq!(original.organism_id, resumed.organism_id);
        assert_eq!(original.selected_action_id, resumed.selected_action_id);
    }
    drop(restored);
    drop(source);
    fs::remove_dir_all(root).unwrap();
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

#[test]
fn durable_mind_clone_keeps_consolidated_learning_and_clears_transient_world_state() {
    let asset_root = unique_asset_root("durable-founder-clone");
    fs::create_dir_all(&asset_root).unwrap();
    let store = GpuCheckpointAssetStore::new(&asset_root).unwrap();
    let organism_id = OrganismId(1);
    let target_organism_id = OrganismId(91);
    let mut source = canonical_runtime(&asset_root, 7_701, SensorProfile::GroundedObjectSlotsV1);
    source.tick().unwrap();
    source.request_recovery_sleep(organism_id).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
    loop {
        assert!(
            std::time::Instant::now() < deadline,
            "native recovery sleep did not complete"
        );
        source.tick_outcome().unwrap();
        let sleep = source.sleep_state_for_test(organism_id).unwrap();
        if sleep.phase == SleepPhase::Awake && sleep.cycles_completed == 1 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
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
    fs::create_dir_all(&root).unwrap();
    let organism_id = OrganismId(1);
    let mut runtime = canonical_runtime(&root, 7_703, SensorProfile::GroundedObjectSlotsV1);
    let launch = AppShellLaunchConfig::from_p34_fixture_root(&root)
        .with_brain_policy(PolicyBackend::NeuralClosedLoopGpu);
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

    drop(restored);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn production_save_persists_recovered_submission_and_atomically_promotes_completed_assets() {
    let root = unique_asset_root("durable-production");
    copy_tree(
        std::path::Path::new("../alife_world/tests/fixtures/p34"),
        &root,
    );
    let mut launch = AppShellLaunchConfig::from_p34_fixture_root(&root)
        .with_brain_policy(PolicyBackend::NeuralClosedLoopGpu);
    let organism_id = OrganismId(1);
    let mut runtime = GpuLiveBrainRuntime::from_p34_launch(
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter"),
        &launch,
    )
    .unwrap();
    runtime.tick().unwrap();
    runtime.request_recovery_sleep(organism_id).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    loop {
        assert!(
            std::time::Instant::now() < deadline,
            "native submission did not complete"
        );
        runtime.tick_outcome().unwrap();
        if matches!(
            runtime
                .sleep_state_for_test(organism_id)
                .unwrap()
                .consolidation,
            ConsolidationState::Submitted { .. }
        ) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    drain_persistence(&mut runtime);
    let submitted = runtime.capture_portable_checkpoint().unwrap();
    let input_generation = submitted
        .creatures
        .iter()
        .find(|creature| creature.organism_id == organism_id)
        .unwrap()
        .gpu_brain
        .as_ref()
        .unwrap()
        .active_weight_generation;
    launch.save_path = root.join("submitted.json");
    GpuDurableSaveManifest::publish_snapshot(&launch.save_path, &root, &submitted).unwrap();
    drop(runtime);
    // Device job handles are session-local. Restore must resubmit the retained
    // replay and finish one logical generation, irrespective of the new job ID.
    let mut recovered = GpuLiveBrainRuntime::from_p34_launch(
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter"),
        &launch,
    )
    .unwrap();
    let store = GpuCheckpointAssetStore::new(&root).unwrap();
    let mut completed = None;
    loop {
        assert!(
            std::time::Instant::now() < deadline,
            "restored native sleep did not wake"
        );
        recovered.tick_outcome().unwrap();
        let state = recovered.sleep_state_for_test(organism_id).unwrap();
        if matches!(state.consolidation, ConsolidationState::Completed { .. }) {
            completed = Some(
                recovered
                    .checkpoint_brain(organism_id, &store)
                    .unwrap()
                    .save_state,
            );
        }
        if state.phase == SleepPhase::Awake && state.cycles_completed == 1 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let completed = completed.expect("restored native job reached Completed before commit");
    let committed = recovered
        .checkpoint_brain(organism_id, &store)
        .unwrap()
        .save_state;
    assert_eq!(
        committed.lifetime_weights,
        completed.sleep_assets.lifetime_staging.unwrap()
    );
    assert_eq!(
        committed.fast_weights,
        completed.sleep_assets.fast_staging.unwrap()
    );
    assert_eq!(
        committed.eligibility,
        completed.sleep_assets.eligibility_staging.unwrap()
    );
    assert_eq!(
        committed.replay_journal,
        completed.sleep_assets.replay_journal_staging.unwrap()
    );
    assert!(committed.sleep_assets.replay_batch.is_none());
    assert!(committed.sleep_assets.lifetime_staging.is_none());
    assert!(committed.sleep_assets.fast_staging.is_none());
    assert!(committed.sleep_assets.eligibility_staging.is_none());
    assert!(committed.sleep_assets.replay_journal_staging.is_none());
    assert_eq!(committed.active_weight_generation, input_generation + 1);
    drain_persistence(&mut recovered);
    let post = recovered.capture_portable_checkpoint().unwrap();
    launch.save_path = root.join("committed.json");
    GpuDurableSaveManifest::publish_snapshot(&launch.save_path, &root, &post).unwrap();
    drop(recovered);
    let mut after_cas = GpuLiveBrainRuntime::from_p34_launch(
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter"),
        &launch,
    )
    .unwrap();
    after_cas.tick_outcome().unwrap();
    assert_eq!(
        after_cas
            .learning_state_for_test(organism_id)
            .unwrap()
            .active_weight_generation,
        committed.active_weight_generation,
        "reopen cannot promote the completed generation twice"
    );
    drop(after_cas);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn device_loss_fail_stops_live_actions_and_retains_the_last_durable_reference() {
    let root = unique_asset_root("device-loss-fail-stop");
    copy_tree(
        std::path::Path::new("../alife_world/tests/fixtures/p34"),
        &root,
    );
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
