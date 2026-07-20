#![cfg(feature = "gpu-tests")]

use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use alife_core::{
    BrainCapacityClass, BrainGenome, BrainScaleTier, BrainTickStatus, Confidence,
    ConsolidationState, DevelopmentState, DriveSnapshot, EndocrineSnapshot, HomeostaticSnapshot,
    MemoryBankConfig, MemorySidecarState, NormalizedScalar, OrganismId, PhenotypeCompiler,
    PhenotypeCompilerInputs, PolicyBackend, ScaffoldContractError, SensorProfile,
    SensorProfileIdentity, SensoryAbiVersion, SleepPhase, SleepState, SleepTrigger, Tick,
    TopologicalMapConfig, TopologySidecar, Validate, Vec3f, SLEEP_CONSOLIDATION_SCHEMA_VERSION,
};
use alife_game_app::{
    merge_gpu_checkpoint_manifest_entries, AppShellLaunchConfig, GameAppShellError,
    GpuBrainSidecarCapture, GpuCheckpointAssetStore, GpuDurableSaveManifest, GpuLiveBrainRuntime,
};
use alife_gpu_backend::GpuClosedLoopBackend;
use alife_world::persistence::{AssetManifest, GpuBrainSaveState, PortableSaveFile};
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
    let mut source =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter");
    let handle = source.insert_brain(organism_id, phenotype.clone()).unwrap();
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
                retained_learning: None,
            },
        )
        .unwrap();
    let mut manifest = AssetManifest::empty();
    merge_gpu_checkpoint_manifest_entries(&mut manifest, write.manifest_entries).unwrap();
    manifest.validate_with_root(&asset_root).unwrap();

    let mut target =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter");
    let restored = store
        .restore_brain(&mut target, &manifest, &write.save_state)
        .unwrap();
    assert_eq!(restored.sleep, SleepState::awake_at(Tick::ZERO));
    assert_eq!(restored.phenotype, phenotype);
    assert_eq!(restored.compiler_inputs, inputs);
    let restored_snapshot = target
        .snapshot_brain(restored.receipt.handle, Tick::ZERO)
        .unwrap();
    assert_eq!(
        restored_snapshot.canonical_digest(),
        write.checkpoint_digest
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
