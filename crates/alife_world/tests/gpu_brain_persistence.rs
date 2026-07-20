//! Portable GPU brain save-state and sleep-transaction persistence contracts.

use alife_core::{
    ActionId, BrainCapacityClass, CandidateActionFamily, CandidateFeatureDigest, Confidence,
    ConsolidationIntent, ConsolidationJobId, ConsolidationStagedOutput, ConsolidationState,
    ExperienceSequenceId, GpuConsolidationRequest, MemoryBankConfig, MemorySidecarState,
    NeuromodulatorSample, OrganismId, PerceptionFrameDigest, PhenotypeHash,
    ReplayEligibilitySample, ReplaySynapseSpan, SensorProfile, SensorProfileIdentity,
    SensoryAbiVersion, SleepPhase, SleepReplayEvent, SleepState, SleepTrigger, Tick,
    TopologicalMapConfig, TopologySidecar, Validate, GPU_CONSOLIDATION_REQUEST_SCHEMA_VERSION,
    SLEEP_CONSOLIDATION_SCHEMA_VERSION,
};
use alife_world::persistence::{
    AssetKind, AssetManifest, AssetManifestEntry, AssetPresence, GpuBrainAssetRef,
    GpuBrainSaveState, GpuSleepAssetState, MemorySidecarSaveState, PortableActivationBanksV1,
    PortableAssetDigest, PortableDualWeightBankV1, PortableEligibilityBanksV1,
    PortableNeuronHomeostasisV1, PortableReplayJournalV1, TopologySidecarSaveSummary,
    GPU_BRAIN_HOMEOSTASIS_LANES_PER_NEURON, GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION,
    GPU_BRAIN_WEIGHT_LAYER_FAST, GPU_BRAIN_WEIGHT_LAYER_LIFETIME,
};
use alife_world::TrackedObjectRegistry;

fn asset(label: &str) -> GpuBrainAssetRef {
    GpuBrainAssetRef {
        asset_id: label.to_string(),
        digest: PortableAssetDigest::for_bytes(label.as_bytes()),
    }
}

fn request() -> GpuConsolidationRequest {
    let mut request = GpuConsolidationRequest {
        schema_version: GPU_CONSOLIDATION_REQUEST_SCHEMA_VERSION,
        request_flags: 0,
        cycle_id: 1,
        phenotype_hash: PhenotypeHash([1, 2, 3, 4]),
        input_generation: 9,
        expected_output_generation: 10,
        input_digest: [5, 6, 7, 8],
        replay_digest: [9, 10, 11, 12],
        max_replay_events: 8,
        max_replay_eligibility_samples: 64,
        request_digest: [0; 4],
    };
    request.request_digest = request.recompute_request_digest().unwrap();
    request
}

fn staged(request: &GpuConsolidationRequest) -> ConsolidationStagedOutput {
    let mut staged = ConsolidationStagedOutput {
        job_id: ConsolidationJobId::try_from_raw(7).unwrap(),
        output_generation: request.expected_output_generation,
        output_weight_bank: 1,
        output_digest: [13, 14, 15, 16],
        eligibility_reset_generation: 11,
        output_eligibility_bank: 0,
        eligibility_output_digest: [17, 18, 19, 20],
        replay_journal_generation: 12,
        replay_journal_cursor: 0,
        replay_journal_event_count: 0,
        replay_journal_output_digest: [21, 22, 23, 24],
        staging_digest: [0; 4],
        promoted_fast_l1_bits: 0.5_f32.to_bits(),
        replay_induced_fast_l1_bits: 0.25_f32.to_bits(),
    };
    staged.staging_digest = staged.recompute_staging_digest(request, 1, 1).unwrap();
    staged
}

fn sleep_state(phase: SleepPhase, consolidation: ConsolidationState) -> SleepState {
    let state = if phase == SleepPhase::Awake {
        SleepState::awake_at(Tick::new(40))
    } else {
        SleepState {
            schema_version: SLEEP_CONSOLIDATION_SCHEMA_VERSION,
            phase,
            phase_started_tick: Tick::new(40),
            entered_sleep_tick: Some(Tick::new(32)),
            cycles_completed: 0,
            last_trigger: Some(SleepTrigger::FatigueThreshold),
            active_cycle_id: 1,
            last_consolidated_cycle_id: 0,
            consolidation,
        }
    };
    state.validate_contract().unwrap();
    state
}

fn save_for_sleep(sleep: SleepState) -> GpuBrainSaveState {
    let organism_id = OrganismId(44);
    let sensor_profile = SensorProfileIdentity {
        profile_id: SensorProfile::PrivilegedAffordanceV1.into(),
        profile_schema_version: 1,
        sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
    };
    let memory_sidecar = MemorySidecarState::new_profiled(
        organism_id,
        sensor_profile,
        MemoryBankConfig::new(64, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap(),
    )
    .unwrap();
    let memory =
        MemorySidecarSaveState::from_sidecar(&memory_sidecar, asset("memory-active"), None, None)
            .unwrap();
    let topology_sidecar =
        TopologySidecar::new_profiled(organism_id, sensor_profile, TopologicalMapConfig::default())
            .unwrap();
    let topology =
        TopologySidecarSaveSummary::from_sidecar(&topology_sidecar, asset("topology")).unwrap();
    let tracked_objects = TrackedObjectRegistry::new(0xA11F_E, 1_024)
        .unwrap()
        .save_state(organism_id)
        .unwrap();
    let committed = matches!(sleep.consolidation, ConsolidationState::Committed { .. });
    let completed = matches!(sleep.consolidation, ConsolidationState::Completed { .. });
    let replay_required = matches!(
        sleep.consolidation,
        ConsolidationState::Pending { .. }
            | ConsolidationState::Prepared { .. }
            | ConsolidationState::Submitted { .. }
            | ConsolidationState::Completed { .. }
    );
    GpuBrainSaveState {
        schema_version: 2,
        organism_id,
        phenotype_hash: PhenotypeHash([1, 2, 3, 4]),
        capacity_class_id: BrainCapacityClass::n512().id(),
        sensor_profile,
        immutable_phenotype: asset("immutable-phenotype"),
        phenotype_compiler_inputs: asset("compiler-inputs"),
        active_weight_generation: if committed { 10 } else { 9 },
        active_weight_bank: if committed { 1 } else { 0 },
        active_eligibility_bank: 0,
        learning_transaction_generation: 6,
        lifetime_weights: asset(if committed {
            "staged-lifetime"
        } else {
            "lifetime"
        }),
        fast_weights: asset(if committed { "staged-fast" } else { "fast" }),
        eligibility: asset(if committed {
            "staged-eligibility"
        } else {
            "eligibility"
        }),
        replay_journal: asset(if committed { "staged-replay" } else { "replay" }),
        replay_journal_generation: if committed { 12 } else { 5 },
        replay_journal_cursor: 0,
        replay_journal_event_count: if committed { 0 } else { 1 },
        activation_state: asset("activations"),
        neuron_homeostasis: asset("neuron-homeostasis"),
        checkpoint_tick: Tick::new(40),
        last_learning_replay_key: None,
        pending_eligibility: None,
        pending_experience_transaction: None,
        memory,
        topology,
        tracked_objects,
        sleep,
        sleep_assets: GpuSleepAssetState {
            replay_batch: replay_required.then(|| asset("sleep-replay")),
            lifetime_staging: completed.then(|| asset("staged-lifetime")),
            fast_staging: completed.then(|| asset("staged-fast")),
            eligibility_staging: completed.then(|| asset("staged-eligibility")),
            replay_journal_staging: completed.then(|| asset("staged-replay")),
        },
    }
}

#[test]
fn every_sleep_phase_roundtrips_without_duplicate_consolidation() {
    let request = request();
    let cases = [
        sleep_state(SleepPhase::Awake, ConsolidationState::None),
        sleep_state(SleepPhase::EnteringSleep, ConsolidationState::None),
        sleep_state(
            SleepPhase::Consolidating,
            ConsolidationState::Pending {
                intent: ConsolidationIntent { cycle_id: 1 },
                replay_digest: request.replay_digest,
                replay_event_count: 1,
                replay_eligibility_sample_count: 1,
            },
        ),
        sleep_state(
            SleepPhase::Waking,
            ConsolidationState::Committed {
                cycle_id: 1,
                output_generation: 10,
                output_digest: [13, 14, 15, 16],
            },
        ),
        sleep_state(SleepPhase::ForcedRecoverySleep, ConsolidationState::None),
    ];

    for sleep in cases {
        let save = save_for_sleep(sleep);
        save.validate().unwrap();
        let json = serde_json::to_string(&save).unwrap();
        let loaded: GpuBrainSaveState = serde_json::from_str(&json).unwrap();
        loaded.validate().unwrap();
        assert_eq!(loaded.sleep.phase, sleep.phase);
        assert_eq!(
            loaded.sleep.last_consolidated_cycle_id,
            sleep.last_consolidated_cycle_id
        );
        assert_eq!(loaded.sleep.consolidation, sleep.consolidation);
    }
}

#[test]
fn every_consolidation_state_roundtrips_with_exact_required_assets() {
    let request = request();
    let staged = staged(&request);
    let states = [
        ConsolidationState::None,
        ConsolidationState::Pending {
            intent: ConsolidationIntent { cycle_id: 1 },
            replay_digest: request.replay_digest,
            replay_event_count: 1,
            replay_eligibility_sample_count: 1,
        },
        ConsolidationState::Prepared { request },
        ConsolidationState::Submitted {
            request,
            job_id: staged.job_id,
        },
        ConsolidationState::Completed { request, staged },
        ConsolidationState::Committed {
            cycle_id: 1,
            output_generation: staged.output_generation,
            output_digest: staged.output_digest,
        },
    ];

    for consolidation in states {
        let save = save_for_sleep(sleep_state(SleepPhase::Consolidating, consolidation));
        save.validate().unwrap();
        let decoded: GpuBrainSaveState =
            serde_json::from_str(&serde_json::to_string(&save).unwrap()).unwrap();
        decoded.validate().unwrap();
        assert_eq!(decoded.sleep.consolidation, consolidation);
    }
}

#[test]
fn completed_sleep_promotion_moves_exact_staging_refs_into_committed_main_state() {
    let request = request();
    let staged = staged(&request);
    let completed = save_for_sleep(sleep_state(
        SleepPhase::Consolidating,
        ConsolidationState::Completed { request, staged },
    ));
    let old_lifetime = completed.lifetime_weights.clone();
    let old_fast = completed.fast_weights.clone();
    let staging_lifetime = completed.sleep_assets.lifetime_staging.clone().unwrap();
    let staging_fast = completed.sleep_assets.fast_staging.clone().unwrap();
    let staging_eligibility = completed.sleep_assets.eligibility_staging.clone().unwrap();
    let staging_replay = completed
        .sleep_assets
        .replay_journal_staging
        .clone()
        .unwrap();

    let committed = completed.promoted_completed_sleep_state().unwrap();

    assert_eq!(completed.lifetime_weights, old_lifetime);
    assert_eq!(completed.fast_weights, old_fast);
    assert_eq!(committed.lifetime_weights, staging_lifetime);
    assert_eq!(committed.fast_weights, staging_fast);
    assert_eq!(committed.eligibility, staging_eligibility);
    assert_eq!(committed.replay_journal, staging_replay);
    assert_eq!(committed.active_weight_generation, staged.output_generation);
    assert_eq!(committed.active_weight_bank, staged.output_weight_bank);
    assert_eq!(
        committed.active_eligibility_bank,
        staged.output_eligibility_bank
    );
    assert_eq!(
        committed.learning_transaction_generation,
        completed.learning_transaction_generation + 1
    );
    assert_eq!(
        committed.replay_journal_generation,
        staged.replay_journal_generation
    );
    assert_eq!(
        committed.replay_journal_cursor,
        staged.replay_journal_cursor
    );
    assert_eq!(
        committed.replay_journal_event_count,
        staged.replay_journal_event_count
    );
    assert_eq!(
        committed.sleep.consolidation,
        ConsolidationState::Committed {
            cycle_id: request.cycle_id,
            output_generation: staged.output_generation,
            output_digest: staged.output_digest,
        }
    );
    assert_eq!(committed.sleep_assets, GpuSleepAssetState::default());
    committed.validate().unwrap();
}

#[test]
fn gpu_checkpoint_requires_exact_enclosing_manifest_references() {
    let save = save_for_sleep(sleep_state(SleepPhase::Awake, ConsolidationState::None));
    let refs = [
        &save.immutable_phenotype,
        &save.phenotype_compiler_inputs,
        &save.lifetime_weights,
        &save.fast_weights,
        &save.eligibility,
        &save.replay_journal,
        &save.activation_state,
        &save.neuron_homeostasis,
        &save.memory.compaction.active_bank_asset,
        &save.topology.summary_asset,
    ];
    let mut manifest = AssetManifest::empty();
    manifest.entries = refs
        .into_iter()
        .map(|asset| AssetManifestEntry {
            asset_id: asset.asset_id.clone(),
            kind: AssetKind::Other,
            relative_path: format!("gpu-brain/{}.json", asset.asset_id),
            digest: asset.digest.clone(),
            presence: AssetPresence::Required,
            schema_version: 1,
            size_bytes: None,
            provenance: Some("gpu-checkpoint-test".to_string()),
        })
        .collect();

    save.validate_asset_manifest(&manifest).unwrap();

    let removed = manifest.entries.pop().unwrap();
    assert!(save.validate_asset_manifest(&manifest).is_err());
    manifest.entries.push(removed);
    manifest.entries[0].digest = PortableAssetDigest::for_bytes(b"wrong payload");
    assert!(save.validate_asset_manifest(&manifest).is_err());
}

#[test]
fn impossible_phase_asset_and_pending_transaction_combinations_are_rejected() {
    let request = request();
    let pending = ConsolidationState::Pending {
        intent: ConsolidationIntent { cycle_id: 1 },
        replay_digest: request.replay_digest,
        replay_event_count: 1,
        replay_eligibility_sample_count: 1,
    };
    let mut missing_replay = save_for_sleep(sleep_state(SleepPhase::Consolidating, pending));
    missing_replay.sleep_assets.replay_batch = None;
    assert!(missing_replay.validate().is_err());

    let mut missing_completed_asset = save_for_sleep(sleep_state(
        SleepPhase::Consolidating,
        ConsolidationState::Completed {
            request,
            staged: staged(&request),
        },
    ));
    missing_completed_asset.sleep_assets.eligibility_staging = None;
    assert!(missing_completed_asset.validate().is_err());

    let mut orphaned_pending_transaction =
        save_for_sleep(sleep_state(SleepPhase::Awake, ConsolidationState::None));
    orphaned_pending_transaction.pending_experience_transaction = Some(asset("pending-patch"));
    assert!(orphaned_pending_transaction.validate().is_err());

    let mut invalid_selector =
        save_for_sleep(sleep_state(SleepPhase::Awake, ConsolidationState::None));
    invalid_selector.active_weight_bank = 2;
    assert!(invalid_selector.validate().is_err());

    let json = serde_json::to_string(&missing_replay).unwrap();
    assert!(serde_json::from_str::<GpuBrainSaveState>(&json).is_err());
}

#[test]
fn portable_float_assets_bind_exact_shapes_generations_and_canonical_bits() {
    let phenotype_hash = PhenotypeHash([31, 32, 33, 34]);
    let mut activations = PortableActivationBanksV1 {
        schema_version: GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION,
        phenotype_hash,
        neuron_count: 2,
        active_side: 1,
        logical_dispatch_generation: 7,
        activation_a_bits: vec![0.0_f32.to_bits(), 0.25_f32.to_bits()],
        activation_b_bits: vec![(-0.5_f32).to_bits(), 1.0_f32.to_bits()],
        canonical_digest: [0; 4],
    };
    activations.canonical_digest = activations.recompute_canonical_digest().unwrap();
    activations.validate().unwrap();

    let mut wrong_length = activations.clone();
    wrong_length.activation_b_bits.pop();
    assert!(wrong_length.validate().is_err());

    let mut negative_zero = activations.clone();
    negative_zero.activation_a_bits[0] = 0x8000_0000;
    assert!(negative_zero.recompute_canonical_digest().is_err());

    let mut homeostasis = PortableNeuronHomeostasisV1 {
        schema_version: GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION,
        phenotype_hash,
        neuron_count: 2,
        lanes_per_neuron: GPU_BRAIN_HOMEOSTASIS_LANES_PER_NEURON,
        value_bits: vec![
            0.1_f32.to_bits(),
            0.2_f32.to_bits(),
            0.3_f32.to_bits(),
            0.4_f32.to_bits(),
        ],
        canonical_digest: [0; 4],
    };
    homeostasis.canonical_digest = homeostasis.recompute_canonical_digest().unwrap();
    homeostasis.validate().unwrap();

    let mut lifetime = PortableDualWeightBankV1 {
        schema_version: GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION,
        layer_raw: GPU_BRAIN_WEIGHT_LAYER_LIFETIME,
        phenotype_hash,
        synapse_count: 2,
        active_generation: 9,
        active_bank: 0,
        bank_0_bits: vec![0.1_f32.to_bits(), 0.2_f32.to_bits()],
        bank_1_bits: vec![0.3_f32.to_bits(), 0.4_f32.to_bits()],
        canonical_digest: [0; 4],
    };
    lifetime.canonical_digest = lifetime.recompute_canonical_digest().unwrap();
    lifetime.validate().unwrap();
    let mut fast = lifetime.clone();
    fast.layer_raw = GPU_BRAIN_WEIGHT_LAYER_FAST;
    fast.canonical_digest = fast.recompute_canonical_digest().unwrap();
    fast.validate().unwrap();
    fast.layer_raw = 99;
    assert!(fast.validate().is_err());

    let mut eligibility = PortableEligibilityBanksV1 {
        schema_version: GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION,
        phenotype_hash,
        recurrent_count: 2,
        decoder_count: 1,
        active_generation: 4,
        inactive_generation: 5,
        active_bank: 1,
        recurrent_bank_0_bits: vec![0.0_f32.to_bits(); 2],
        recurrent_bank_1_bits: vec![0.5_f32.to_bits(); 2],
        decoder_bank_0_bits: vec![0.0_f32.to_bits()],
        decoder_bank_1_bits: vec![(-0.25_f32).to_bits()],
        canonical_digest: [0; 4],
    };
    eligibility.canonical_digest = eligibility.recompute_canonical_digest().unwrap();
    eligibility.validate().unwrap();
    eligibility.inactive_generation = 8;
    assert!(eligibility.validate().is_err());
}

#[test]
fn portable_replay_journal_preserves_bounded_ring_identity_and_empty_reset_state() {
    let event = SleepReplayEvent {
        sequence_id: ExperienceSequenceId(1),
        originating_tick: Tick::new(10),
        frame_digest: PerceptionFrameDigest([1, 2, 3, 4]),
        candidate_feature_digest: CandidateFeatureDigest([5, 6]),
        action_id: ActionId(7),
        family: CandidateActionFamily::Approach,
        modulator: NeuromodulatorSample::from_components(0.5, 0.0, 0.2, 0.0, 0.1).unwrap(),
    };
    let mut journal = PortableReplayJournalV1 {
        schema_version: GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION,
        phenotype_hash: PhenotypeHash([11, 12, 13, 14]),
        replay_capture_plan_digest: [21, 22, 23, 24],
        generation: 3,
        cursor: 1,
        event_count: 1,
        event_capacity: 4,
        sample_capacity: 4,
        events: vec![event],
        synapse_spans: vec![ReplaySynapseSpan {
            local_synapse_id: 9,
            sample_start: 0,
            sample_count: 1,
            reserved: 0,
        }],
        eligibility_samples: vec![ReplayEligibilitySample {
            event_index: 0,
            eligibility_q15: 123,
        }],
        canonical_digest: [0; 4],
    };
    journal.canonical_digest = journal.recompute_canonical_digest().unwrap();
    journal.validate().unwrap();

    let mut wrong_cursor = journal.clone();
    wrong_cursor.cursor = 2;
    assert!(wrong_cursor.validate().is_err());

    let mut reset = PortableReplayJournalV1 {
        schema_version: GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION,
        phenotype_hash: journal.phenotype_hash,
        replay_capture_plan_digest: journal.replay_capture_plan_digest,
        generation: 4,
        cursor: 0,
        event_count: 0,
        event_capacity: 4,
        sample_capacity: 4,
        events: Vec::new(),
        synapse_spans: vec![ReplaySynapseSpan {
            local_synapse_id: 9,
            sample_start: 0,
            sample_count: 0,
            reserved: 0,
        }],
        eligibility_samples: Vec::new(),
        canonical_digest: [0; 4],
    };
    reset.canonical_digest = reset.recompute_canonical_digest().unwrap();
    reset.validate().unwrap();
}
