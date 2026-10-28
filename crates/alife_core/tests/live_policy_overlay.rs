//! Pure compiler/provenance checks. No device, world or trained founder is used.
use alife_core::{
    BrainCapacityClass, BrainGenome, DevelopmentState, FoundationWeightAsset,
    N2048LivePolicyTransition, NormalizedScalar, PhenotypeCompiler, PhenotypeCompilerInputs,
    SensorProfile, Tick, TrainingStageManifest,
};

fn fixture() -> (PhenotypeCompilerInputs, FoundationWeightAsset) {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(0x10_11_09, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    let initial = PhenotypeCompiler::compile_testing_procedural_baseline(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let asset = FoundationWeightAsset::from_phenotype_for_genetic_birth(&initial).unwrap();
    let (compiled, inputs) =
        PhenotypeCompiler::compile_n2048_foundation_candidate(genome, development, asset).unwrap();
    let mut weights = compiled
        .synapses()
        .iter()
        .map(|s| s.genetic_weight())
        .collect::<Vec<_>>();
    weights[0] += 0.001;
    let trained = FoundationWeightAsset::from_trained_weights(
        &compiled,
        weights,
        TrainingStageManifest::new(1, 1, 1),
    )
    .unwrap();
    (inputs, trained)
}

#[test]
fn overlay_changes_only_learned_weights_and_bound_policy_identity() {
    let (before, trained) = fixture();
    let transition = N2048LivePolicyTransition::prepare(&before, 7, trained.clone()).unwrap();
    assert_eq!(transition.previous_policy(), Some(7));
    assert_eq!(transition.next_policy(), 8);
    assert_eq!(transition.next_inputs().genome(), before.genome());
    assert_eq!(transition.next_inputs().development(), before.development());
    assert_eq!(
        transition.next_inputs().sensor_profile(),
        before.sensor_profile()
    );
    let provenance = transition.next_inputs().live_policy().unwrap();
    assert_eq!(provenance.policy_version, 8);
    assert_eq!(
        provenance.birth_foundation_digest,
        before.n2048_candidate_asset().unwrap().digest()
    );
    assert_ne!(
        transition.previous_phenotype().phenotype_hash(),
        transition.next_phenotype().phenotype_hash()
    );
    assert_eq!(
        transition.next_phenotype().synapses()[0]
            .genetic_weight()
            .to_bits(),
        trained.weights()[0].to_bits()
    );
    let encoded = serde_json::to_vec(transition.next_inputs()).unwrap();
    let restored: PhenotypeCompilerInputs = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(restored, *transition.next_inputs());
    assert_eq!(
        PhenotypeCompiler::compile_validated(&restored, &BrainCapacityClass::n2048()).unwrap(),
        *transition.next_phenotype()
    );
}

#[test]
fn existing_inputs_stay_unmarked_and_roundtrip_exactly() {
    let (before, _) = fixture();
    let encoded = serde_json::to_vec(&before).unwrap();
    assert!(!String::from_utf8(encoded.clone())
        .unwrap()
        .contains("live_policy"));
    let restored: PhenotypeCompilerInputs = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(before, restored);
    assert!(restored.live_policy().is_none());
}

#[test]
fn stale_policy_overflow_and_forged_provenance_are_rejected() {
    let (before, trained) = fixture();
    assert!(N2048LivePolicyTransition::prepare(&before, u64::MAX, trained.clone()).is_err());
    let transition = N2048LivePolicyTransition::prepare(&before, 7, trained.clone()).unwrap();
    assert!(
        N2048LivePolicyTransition::prepare(transition.next_inputs(), 7, trained.clone()).is_err()
    );
    let mut forged = serde_json::to_value(transition.next_inputs()).unwrap();
    forged["live_policy"]["policy_version"] = serde_json::json!(9);
    assert!(serde_json::from_value::<PhenotypeCompilerInputs>(forged).is_err());
    let again = N2048LivePolicyTransition::prepare(transition.next_inputs(), 8, trained).unwrap();
    assert_eq!(again.next_policy(), 9);
    assert_eq!(
        again
            .next_inputs()
            .live_policy()
            .unwrap()
            .birth_foundation_digest,
        transition
            .next_inputs()
            .live_policy()
            .unwrap()
            .birth_foundation_digest
    );
}

#[test]
fn legacy_install_preserves_unknown_private_policy_age_without_extra_global_update() {
    let (before, trained) = fixture();
    let transition =
        N2048LivePolicyTransition::adopt_existing_policy(&before, 7, trained.clone()).unwrap();
    assert_eq!(transition.previous_policy(), None);
    assert_eq!(transition.next_policy(), 7);
    assert_eq!(transition.next_inputs().genome(), before.genome());
    assert!(
        N2048LivePolicyTransition::adopt_existing_policy(transition.next_inputs(), 7, trained)
            .is_err()
    );
}

#[test]
fn activity_rebind_preserves_recorded_work_pressure_sequence_and_atp() {
    use alife_core::{
        BrainActivityPolicyV1, BrainDispatchIdentity, BrainWorkCounters, BrainWorkReceipt,
        GpuPressureSample, GpuPressureSampleInput, NeuralThrottleDecision, BRAIN_ATP_Q16_MAX,
    };
    let (inputs, trained) = fixture();
    let transition = N2048LivePolicyTransition::prepare(&inputs, 7, trained).unwrap();
    let capacity = BrainCapacityClass::n2048();
    let policy = BrainActivityPolicyV1::production_v1();
    let identity = BrainDispatchIdentity {
        organism_id_raw: 51,
        tick: 99,
        class_id_raw: capacity.id().raw(),
        handle_slot: 2,
        handle_generation: 3,
        sequence_cursor: 17,
        dispatch_generation: 23,
        frame_digest: [5; 4],
    };
    let pressure = GpuPressureSample::try_new(
        &policy,
        GpuPressureSampleInput {
            identity,
            source_dispatch_generation: 19,
            source_frame_digest: [4; 4],
            completed_gpu_time_ns: 1_000_000,
            queue_depth: 0,
            logical_heap_used: 128,
            logical_heap_capacity: 1024,
            brain_atp_remaining_q16: BRAIN_ATP_Q16_MAX,
            brain_atp_capacity_q16: BRAIN_ATP_Q16_MAX,
        },
    )
    .unwrap();
    let throttle = NeuralThrottleDecision::derive(
        &policy,
        transition.previous_phenotype(),
        capacity.execution(),
        identity,
        pressure,
    )
    .unwrap();
    let work = BrainWorkReceipt::try_new(
        &policy,
        &throttle,
        BrainWorkCounters {
            microsteps: u32::from(throttle.microsteps),
            neuron_updates: 27,
            tile_visits: 2,
            synapse_ops: 31,
            decoder_candidate_ops: 3,
            memory_context_ops: 4,
        },
        BRAIN_ATP_Q16_MAX,
    )
    .unwrap();
    assert!(
        throttle
            .validate_for(transition.next_phenotype(), capacity.execution())
            .is_err(),
        "the retained old route digest must fail against the new policy identity"
    );
    let (rebound, rebound_work) = throttle
        .rebind_weight_only_policy_authority(
            &work,
            &policy,
            transition.previous_phenotype(),
            transition.next_phenotype(),
            capacity.execution(),
        )
        .unwrap();
    rebound
        .validate_for(transition.next_phenotype(), capacity.execution())
        .unwrap();
    rebound_work.validate_for(&policy, &rebound).unwrap();
    assert_ne!(
        rebound.route_schedule_digest,
        throttle.route_schedule_digest
    );
    assert_ne!(rebound.decision_digest, throttle.decision_digest);
    assert_ne!(rebound_work.receipt_digest, work.receipt_digest);
    let mut same_history = rebound.clone();
    same_history.route_schedule_digest = throttle.route_schedule_digest;
    same_history.decision_digest = throttle.decision_digest;
    assert_eq!(
        same_history, throttle,
        "all recorded decision/pressure fields are retained"
    );
    let mut same_work = rebound_work;
    same_work.route_schedule_digest = work.route_schedule_digest;
    same_work.receipt_digest = work.receipt_digest;
    assert_eq!(
        same_work, work,
        "all recorded work/ATP/history fields are retained"
    );
    let mut corrupt = work.clone();
    corrupt.atp_after_q16 ^= 1;
    assert!(throttle
        .rebind_weight_only_policy_authority(
            &corrupt,
            &policy,
            transition.previous_phenotype(),
            transition.next_phenotype(),
            capacity.execution()
        )
        .is_err());
    let mut corrupt = throttle.clone();
    corrupt.sequence_cursor += 1;
    assert!(corrupt
        .rebind_weight_only_policy_authority(
            &work,
            &policy,
            transition.previous_phenotype(),
            transition.next_phenotype(),
            capacity.execution()
        )
        .is_err());
}
