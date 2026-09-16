//! Real-hardware acceptance for GPU replay and exactly-once sleep consolidation.
#![cfg(feature = "gpu-tests")]

mod support;

use alife_core::{
    AlphaMask, BiochemistryState, BodyEventDelta, BrainCapacityClass, BrainGenome, Confidence,
    ConsolidationIntent, DecisionSnapshot, DevelopmentState, ExperiencePatch, ExperienceSequenceId,
    JointMotorCondition, JointPhysicalOutcome, MeasuredPhysiologyTransition, NeuralActionSelection,
    NormalizedScalar, OutcomeCreditPacket, PhenotypeCompiler, PhysicalActionOutcome,
    PhysicalContactKind, PlasticityGenomeParameters, PostActionOutcome, PreActionSnapshot,
    PredictionTargetReceipt, SemanticStateVector, SignedValence, Tick, Vec3f,
};
use alife_gpu_backend::{
    GpuClosedLoopBackend, GpuConsolidationRequestRecord, GpuReplayEventRecord,
    GpuReplaySynapseSpanRecord, GpuSleepHeader,
};

fn sealed_reward(
    handle: alife_gpu_backend::GpuBrainHandle,
    brain: &alife_core::BrainPhenotype,
    physiology: &alife_core::CreaturePhenotype,
    frame: &alife_core::PerceptionFrame,
    tick: &alife_gpu_backend::GpuClosedLoopTick,
    sequence_raw: u64,
    body_event: BodyEventDelta,
) -> (ExperiencePatch, alife_core::NeuralReceptorFrame) {
    let sequence_id = ExperienceSequenceId(sequence_raw);
    assert_eq!(handle.organism_id(), frame.organism_id());
    assert_eq!(handle.class_id(), brain.brain_class_id());
    assert_eq!(handle.phenotype_hash(), brain.phenotype_hash());

    let capacity = BrainCapacityClass::n512();
    assert_eq!(capacity.id(), handle.class_id());
    let parameters =
        PlasticityGenomeParameters::try_new_v1(0.95, 1.0, 0.0, 0.25, 1.0, -2.0, 2.0, 0.5, 4.0, 0.5)
            .unwrap();
    let mut genome = BrainGenome::scaffold(42, handle.class_id())
        .with_plasticity_parameters(parameters)
        .unwrap();
    genome.alpha_mask = AlphaMask::default_for_projection(NormalizedScalar::new(1.0).unwrap());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
    let compiled =
        PhenotypeCompiler::compile(&genome, &capacity, &development, brain.sensor_profile())
            .unwrap();
    assert_eq!(compiled.phenotype_hash(), brain.phenotype_hash());

    let selection = NeuralActionSelection {
        candidate_index: tick.selection.candidate_index,
        logit: tick.selection.logit,
        confidence: tick.selection.confidence,
        active_tiles: tick.selection.active_tiles,
        active_synapses: tick.selection.active_synapses,
    };
    let command = frame.candidates()[usize::from(selection.candidate_index)]
        .to_command(
            handle.organism_id(),
            Confidence::new(selection.confidence.raw()).unwrap(),
        )
        .unwrap();
    let selected_action_id = command.action_id;
    let prompted = frame
        .sensory()
        .language_context
        .heard_tokens
        .iter()
        .flatten()
        .any(|token| token.source_kind == alife_core::UtteranceSourceKind::Player);
    let bundle = alife_core::factorized_motor_bundle_for_candidates(
        handle.organism_id(),
        sequence_id,
        frame.tick(),
        frame,
        tick.factorized_motor_candidates,
        &brain
            .candidate_decoder()
            .factorized_motor_channels(brain)
            .unwrap(),
        &command,
        selection.candidate_index,
        tick.speech_payload.as_ref(),
        prompted,
    )
    .unwrap();
    tick.pending_eligibility
        .identity()
        .joint_selection()
        .unwrap()
        .validate_bundle(&bundle)
        .unwrap();
    let pre_action = PreActionSnapshot::from_neural_frame(
        sequence_id,
        handle.class_id(),
        handle.phenotype_hash(),
        genome.id,
        genome.schema_version,
        development,
        frame.clone(),
    )
    .unwrap();
    let decision = DecisionSnapshot::from_neural_selection(
        sequence_id,
        handle.phenotype_hash(),
        tick.dispatch_generation,
        tick.active_activation_side,
        frame,
        selection,
        command,
    )
    .unwrap();
    let before = BiochemistryState::new(physiology, frame.tick()).unwrap();
    let receptors = before.neural_receptor_frame(physiology).unwrap();
    assert!(!receptors.activations.is_empty());
    let outcome_tick = Tick::new(frame.tick().raw() + 1);
    let after = before
        .advance(outcome_tick, body_event, physiology)
        .unwrap();
    let measured = MeasuredPhysiologyTransition::new(before, after).unwrap();
    assert_ne!(
        measured.homeostatic_delta,
        alife_core::HomeostaticDelta::zero()
    );
    let physical = PhysicalActionOutcome {
        contact: PhysicalContactKind::None,
        target_entity: None,
        displacement: Vec3f::ZERO,
        collision_normal: None,
        energy_cost: NormalizedScalar::ZERO,
    };
    let work = alife_core::CognitiveWorkReceipt::zero();
    let outcome = PostActionOutcome::new(
        handle.organism_id(),
        sequence_id,
        outcome_tick,
        true,
        physical,
        measured.homeostatic_delta,
        SignedValence::ZERO,
        NormalizedScalar::ZERO,
        NormalizedScalar::new(measured.aversive_harm()).unwrap(),
        measured.energy_delta,
        NormalizedScalar::ZERO,
    )
    .unwrap()
    .with_measured_physiology(measured)
    .unwrap()
    .with_v11_joint(
        JointPhysicalOutcome::new(physical, Vec::new()).unwrap(),
        work,
    )
    .unwrap();
    let target = PredictionTargetReceipt::for_successor(
        handle.organism_id(),
        sequence_id,
        selected_action_id,
        frame.tick(),
        frame.frame_digest().0,
        SemanticStateVector::new(vec![0.5, 0.25]).unwrap(),
        JointMotorCondition::from_bundle(&bundle).unwrap(),
        SemanticStateVector::new(vec![0.4, 0.3]).unwrap(),
    )
    .unwrap();
    let patch = ExperiencePatch::new_v11_with_decision(
        pre_action,
        decision,
        bundle,
        outcome,
        target,
        work,
        alife_core::CognitiveContextFrame::empty(handle.organism_id(), sequence_id, frame.tick())
            .unwrap(),
    )
    .unwrap();
    let credit = OutcomeCreditPacket::from_sealed_patch(&patch)
        .unwrap()
        .with_biochemical_receptors(&receptors)
        .unwrap();
    assert!(credit.modulator().homeostatic_improvement() > 0.0);
    (patch, receptors)
}

fn install_full_recorded_pressures(
    backend: &mut GpuClosedLoopBackend,
    rows: &[(
        alife_gpu_backend::GpuBrainHandle,
        &alife_core::PerceptionFrame,
    )],
    snapshot_tick: Tick,
    dispatch_generation: u64,
) {
    let completed_dispatches = backend.runtime_counters_for_test().0;
    let expected_resident_dispatch = if completed_dispatches == 0 {
        dispatch_generation
    } else {
        dispatch_generation - 1
    };
    let logical_heap_used = backend.admission_receipt().logical_committed_bytes;
    let logical_heap_capacity = backend.runtime_budget().logical_neural_heap_budget_bytes;
    let pressures = rows
        .iter()
        .map(|(handle, final_frame)| {
            let activity = backend.snapshot_activity_state(*handle).unwrap();
            let checkpoint = backend
                .snapshot_brain(*handle, snapshot_tick)
                .unwrap()
                .into_parts();
            assert_eq!(
                checkpoint.logical_dispatch_generation, expected_resident_dispatch,
                "pressure fixture must use the resident logical dispatch identity"
            );
            let prior = activity.pressure;
            alife_core::GpuPressureSample::try_new(
                backend.activity_policy(),
                alife_core::GpuPressureSampleInput {
                    identity: alife_core::BrainDispatchIdentity {
                        organism_id_raw: handle.organism_id().raw(),
                        tick: final_frame.tick().raw(),
                        class_id_raw: handle.class_id().raw(),
                        handle_slot: handle.slot(),
                        handle_generation: handle.generation(),
                        sequence_cursor: activity.next_sequence_cursor,
                        dispatch_generation,
                        frame_digest: final_frame.frame_digest().0,
                    },
                    source_dispatch_generation: prior
                        .map_or(0, |pressure| pressure.dispatch_generation),
                    source_frame_digest: prior.map_or([0; 4], |pressure| pressure.frame_digest),
                    completed_gpu_time_ns: 0,
                    queue_depth: 0,
                    logical_heap_used,
                    logical_heap_capacity,
                    brain_atp_remaining_q16: activity.brain_atp_q16,
                    brain_atp_capacity_q16: alife_core::BRAIN_ATP_Q16_MAX,
                },
            )
            .unwrap()
        })
        .collect();
    backend.install_recorded_pressure_replay(pressures).unwrap();
}

fn paired_chemistry_tick(
    backend: &mut GpuClosedLoopBackend,
    handles: [alife_gpu_backend::GpuBrainHandle; 2],
    brain: &alife_core::BrainPhenotype,
    physiologies: [&alife_core::CreaturePhenotype; 2],
    sources: [&alife_core::PerceptionFrame; 2],
) -> Result<
    [(
        alife_core::PerceptionFrame,
        alife_gpu_backend::GpuClosedLoopTick,
    ); 2],
    alife_core::ScaffoldContractError,
> {
    let final_frames = [
        support::empty_recall(sources[0]).0,
        support::empty_recall(sources[1]).0,
    ];
    let dispatch_generation = backend.runtime_counters_for_test().0 + 1;
    install_full_recorded_pressures(
        backend,
        &[
            (handles[0], &final_frames[0]),
            (handles[1], &final_frames[1]),
        ],
        final_frames[0].tick(),
        dispatch_generation,
    );
    let mut ticks = support::tick_chemistry_batch(
        backend,
        &[
            (handles[0], brain, physiologies[0], sources[0]),
            (handles[1], brain, physiologies[1], sources[1]),
        ],
    )?;
    if ticks.len() != 2 {
        return Err(alife_core::ScaffoldContractError::InvalidDecisionEvidence);
    }
    let first = ticks.remove(0);
    let second = ticks.remove(0);
    let result = [first, second];
    for (index, (frame, tick)) in result.iter().enumerate() {
        assert_eq!(frame.frame_digest(), final_frames[index].frame_digest());
        assert_eq!(tick.pressure.dispatch_generation, dispatch_generation);
        assert_eq!(
            tick.pressure.frame_digest,
            final_frames[index].frame_digest().0
        );
        assert_eq!(tick.throttle.level, alife_core::NeuralThrottleLevel::Full);
        assert_eq!(tick.pressure.completed_gpu_time_ns, 0);
        assert_eq!(tick.pressure.queue_depth, 0);
    }
    assert_eq!(
        result[0].1.throttle.microsteps,
        result[1].1.throttle.microsteps
    );
    assert_eq!(
        result[0].1.throttle.enabled_route_ids,
        result[1].1.throttle.enabled_route_ids
    );
    Ok(result)
}

fn learned_backend(
    organism_raw: u64,
) -> (
    GpuClosedLoopBackend,
    alife_gpu_backend::GpuBrainHandle,
    alife_core::BrainPhenotype,
) {
    let phenotype = support::controlled_learning_n512_phenotype(1.0);
    let mut backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let handle = backend
        .insert_brain(alife_core::OrganismId(organism_raw), phenotype.clone())
        .unwrap();
    let source = support::perception_frame_for_profile_at_tick(
        organism_raw,
        2_000,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let physiology = support::test_physiology(organism_raw, &phenotype).unwrap();
    let (frame, tick) =
        support::tick_with_receptors(&mut backend, handle, &phenotype, &physiology, &source)
            .unwrap();
    let (patch, receptors) = sealed_reward(
        handle,
        &phenotype,
        &physiology,
        &frame,
        &tick,
        1,
        BodyEventDelta {
            nutrition: 1.0,
            ..BodyEventDelta::zero()
        },
    );
    let receipt = backend
        .apply_sealed_outcome(handle, &patch, &receptors)
        .unwrap();
    assert!(receipt.fast_weights_changed > 0);
    (backend, handle, phenotype)
}

fn stage_sleep(
    backend: &mut GpuClosedLoopBackend,
    handle: alife_gpu_backend::GpuBrainHandle,
    cycle_id: u64,
) -> (
    alife_core::BoundedReplayBatch,
    alife_core::GpuConsolidationRequest,
    alife_gpu_backend::GpuSleepStagingReceipt,
) {
    let replay = backend.build_sleep_replay_batch(handle).unwrap();
    let request = backend
        .prepare_sleep_consolidation(handle, ConsolidationIntent { cycle_id }, &replay)
        .unwrap();
    let job = backend
        .submit_sleep_consolidation(handle, &request, &replay)
        .unwrap();
    let staged = backend
        .poll_sleep_consolidation(handle, job)
        .unwrap()
        .expect("synchronous real-GPU staging must be pollable");
    (replay, request, staged)
}

#[test]
fn sleep_header_layout_matches_wgsl() {
    assert_eq!(std::mem::size_of::<GpuSleepHeader>(), 80);
    assert_eq!(std::mem::align_of::<GpuSleepHeader>(), 16);
    assert_eq!(std::mem::offset_of!(GpuSleepHeader, brain_slot_index), 16);
    assert_eq!(std::mem::offset_of!(GpuSleepHeader, request_offset), 20);
    assert_eq!(std::mem::offset_of!(GpuSleepHeader, replay_span_offset), 32);
    assert_eq!(
        std::mem::offset_of!(GpuSleepHeader, replay_sample_count),
        44
    );
    assert_eq!(std::mem::offset_of!(GpuSleepHeader, job_id_lo), 56);
    assert_eq!(std::mem::offset_of!(GpuSleepHeader, flags), 72);
    assert_eq!(std::mem::size_of::<GpuConsolidationRequestRecord>(), 176);
    assert_eq!(std::mem::align_of::<GpuConsolidationRequestRecord>(), 16);
    assert_eq!(
        std::mem::offset_of!(GpuConsolidationRequestRecord, phenotype_hash),
        16
    );
    assert_eq!(
        std::mem::offset_of!(GpuConsolidationRequestRecord, request_digest),
        136
    );
    assert_eq!(std::mem::size_of::<GpuReplayEventRecord>(), 112);
    assert_eq!(std::mem::align_of::<GpuReplayEventRecord>(), 16);
    assert_eq!(
        std::mem::offset_of!(GpuReplayEventRecord, biochemical_aversive),
        100
    );
    assert_eq!(std::mem::size_of::<GpuReplaySynapseSpanRecord>(), 16);
    assert_eq!(std::mem::align_of::<GpuReplaySynapseSpanRecord>(), 16);

    for source in [
        alife_gpu_backend::CLOSED_LOOP_CONSOLIDATE_WGSL,
        alife_gpu_backend::CLOSED_LOOP_REPLAY_LEARNING_WGSL,
    ] {
        naga::front::wgsl::parse_str(source).expect("sleep WGSL must parse");
    }
}

#[test]
fn sleep_weight_writes_canonicalize_zero_before_gpu_storage() {
    let consolidate = alife_gpu_backend::CLOSED_LOOP_CONSOLIDATE_WGSL;
    assert!(consolidate.contains("fn canonicalize_state_zero(value:f32) -> f32"));
    assert!(
        consolidate.contains("store_state_f32(inactive.fast+gid.x,canonicalize_state_zero(fast))")
    );
    assert!(consolidate.contains(
        "store_state_f32(inactive.lifetime+gid.x,canonicalize_state_zero(next_lifetime))"
    ));
    assert!(consolidate
        .contains("store_state_f32(inactive.fast+gid.x,canonicalize_state_zero(next_fast))"));

    let replay = alife_gpu_backend::CLOSED_LOOP_REPLAY_LEARNING_WGSL;
    assert!(replay.contains("store_state_f32(fast_index,canonicalize_state_zero(next))"));
}

#[test]
fn consolidation_promotes_fast_preserves_genetic_and_commits_once() {
    let (mut backend, handle, phenotype) = learned_backend(5_001);
    let genetic_before = backend
        .read_immutable_genetic_weights_for_test(handle)
        .unwrap();
    let fast_before = backend.read_active_fast_weights_for_test(handle).unwrap();
    assert!(fast_before.iter().any(|value| *value != 0.0));
    let lifetime_before = backend
        .read_active_lifetime_weights_for_test(handle)
        .unwrap();
    let (replay, request, staged) = stage_sleep(&mut backend, handle, 1);
    assert!(
        replay
            .eligibility_samples
            .iter()
            .any(|sample| sample.eligibility_q15 != 0),
        "sleep replay must carry nonzero captured eligibility: {replay:?}"
    );
    assert!(
        replay.events.iter().any(|event| event
            .modulator
            .frame()
            .lanes()
            .iter()
            .any(|value| *value != 0.0)),
        "sleep replay must carry a nonzero outcome modulator: {replay:?}"
    );
    staged.staged.validate_against(&request, 1, 1).unwrap();
    assert_eq!(
        backend
            .prepare_sleep_consolidation(handle, ConsolidationIntent { cycle_id: 1 }, &replay)
            .unwrap(),
        request
    );

    let receipt = backend
        .commit_sleep_consolidation(handle, &request, &staged.staged)
        .unwrap_or_else(|error| {
            panic!(
                "commit failed after staging: {error:?}; state={:?}",
                backend.learning_state_snapshot_for_test(handle)
            )
        });
    assert_eq!(receipt.generation_swaps, 1);
    assert_eq!(
        receipt.output_generation,
        request.expected_output_generation
    );
    let lifetime_after = backend
        .read_active_lifetime_weights_for_test(handle)
        .unwrap();
    let measured_promoted_l1 = lifetime_before
        .iter()
        .zip(&lifetime_after)
        .map(|(before, after)| (after - before).abs())
        .sum::<f32>();
    assert!(
        receipt.promoted_fast_l1 > 0.0,
        "receipt={receipt:?}; measured_promoted_l1={measured_promoted_l1}"
    );
    assert!(
        receipt.replay_induced_fast_l1 > 0.0,
        "receipt={receipt:?}; nonzero_replay_samples={:?}",
        replay
            .eligibility_samples
            .iter()
            .filter(|sample| sample.eligibility_q15 != 0)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        backend
            .read_immutable_genetic_weights_for_test(handle)
            .unwrap(),
        genetic_before
    );
    assert_eq!(genetic_before.len(), phenotype.synapses().len());
    assert!(lifetime_after.iter().any(|value| *value != 0.0));

    let retry = backend
        .commit_sleep_consolidation(handle, &request, &staged.staged)
        .unwrap();
    assert_eq!(retry, receipt);
}

#[test]
fn committed_sleep_retires_staging_payload_but_remains_idempotent() {
    let (mut backend, handle, _) = learned_backend(5_010);
    let (_replay, request, staged) = stage_sleep(&mut backend, handle, 1);

    let receipt = backend
        .commit_sleep_consolidation(handle, &request, &staged.staged)
        .unwrap();

    assert!(matches!(
        backend.poll_sleep_consolidation(handle, staged.staged.job_id),
        Err(alife_core::ScaffoldContractError::ConsolidationGenerationMismatch)
    ));
    assert_eq!(
        backend
            .commit_sleep_consolidation(handle, &request, &staged.staged)
            .unwrap(),
        receipt
    );
}

#[test]
fn invalid_staging_rolls_back_and_valid_staging_still_commits() {
    let (mut backend, handle, _) = learned_backend(5_002);
    let before = backend.learning_state_snapshot_for_test(handle).unwrap();
    let (_replay, request, staged) = stage_sleep(&mut backend, handle, 2);
    let mut tampered = staged.staged;
    tampered.output_digest[0] ^= 1;

    assert!(backend
        .commit_sleep_consolidation(handle, &request, &tampered)
        .is_err());
    assert_eq!(
        backend.learning_state_snapshot_for_test(handle).unwrap(),
        before
    );
    let receipt = backend
        .commit_sleep_consolidation(handle, &request, &staged.staged)
        .unwrap();
    assert_eq!(receipt.generation_swaps, 1);
}

#[test]
fn committed_sleep_consumes_replay_and_a_second_empty_cycle_cannot_replay_it() {
    let (mut backend, handle, _) = learned_backend(5_003);
    let (first_replay, first_request, first_staged) = stage_sleep(&mut backend, handle, 1);
    assert!(!first_replay.events.is_empty());
    let first = backend
        .commit_sleep_consolidation(handle, &first_request, &first_staged.staged)
        .unwrap();
    assert!(first.replay_induced_fast_l1 > 0.0);

    let second_replay = backend.build_sleep_replay_batch(handle).unwrap();
    assert!(second_replay.events.is_empty());
    assert!(second_replay.eligibility_samples.is_empty());
    let second_request = backend
        .prepare_sleep_consolidation(handle, ConsolidationIntent { cycle_id: 2 }, &second_replay)
        .unwrap();
    let second_job = backend
        .submit_sleep_consolidation(handle, &second_request, &second_replay)
        .unwrap();
    let second_staged = backend
        .poll_sleep_consolidation(handle, second_job)
        .unwrap()
        .unwrap();
    let second = backend
        .commit_sleep_consolidation(handle, &second_request, &second_staged.staged)
        .unwrap();
    assert_eq!(second.replay_induced_fast_l1, 0.0);
    assert_ne!(first.staged.cycle_id, second.staged.cycle_id);
}

#[test]
fn wake_starts_with_no_pre_sleep_eligibility_or_replay_rows() {
    let (mut backend, handle, _) = learned_backend(5_004);
    let before = backend.learning_state_snapshot_for_test(handle).unwrap();
    assert!(before.recurrent_eligibility_nonzero + before.decoder_eligibility_nonzero > 0);
    assert!(before.replay_event_nonzero_words > 0);
    assert!(before.replay_sample_nonzero_words > 0);

    let (_replay, request, staged) = stage_sleep(&mut backend, handle, 1);
    let receipt = backend
        .commit_sleep_consolidation(handle, &request, &staged.staged)
        .unwrap();
    let after = backend.learning_state_snapshot_for_test(handle).unwrap();

    assert_eq!(after.recurrent_eligibility_nonzero, 0);
    assert_eq!(after.decoder_eligibility_nonzero, 0);
    assert_eq!(after.replay_event_nonzero_words, 0);
    assert_eq!(after.replay_sample_nonzero_words, 0);
    assert_eq!(after.pending_eligibility_nonzero_words, 0);
    assert_eq!(after.replay_journal_cursor, 0);
    assert_eq!(after.replay_journal_event_count, 0);
    assert_eq!(after.active_eligibility_bank, 0);
    assert!(after.active_eligibility_generation > before.active_eligibility_generation);
    assert_eq!(
        after.active_eligibility_generation,
        receipt.eligibility_reset_generation
    );
}

#[test]
fn two_same_class_sleep_jobs_do_not_cross_write_slots() {
    let phenotype = support::controlled_learning_n512_phenotype(1.0);
    let mut backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let handle_a = backend
        .insert_brain(alife_core::OrganismId(5_005), phenotype.clone())
        .unwrap();
    let handle_b = backend
        .insert_brain(alife_core::OrganismId(5_006), phenotype.clone())
        .unwrap();
    let source_a = support::perception_frame_for_profile_at_tick(
        5_005,
        3_000,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let source_b = support::perception_frame_for_profile_at_tick(
        5_006,
        3_000,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let physiology_a = support::test_physiology(5_005, &phenotype).unwrap();
    let physiology_b = support::test_physiology(5_006, &phenotype).unwrap();
    let [(frame_a, tick_a), (frame_b, tick_b)] = paired_chemistry_tick(
        &mut backend,
        [handle_a, handle_b],
        &phenotype,
        [&physiology_a, &physiology_b],
        [&source_a, &source_b],
    )
    .unwrap();
    let (patch_a, receptors_a) = sealed_reward(
        handle_a,
        &phenotype,
        &physiology_a,
        &frame_a,
        &tick_a,
        1,
        BodyEventDelta {
            nutrition: 1.0,
            ..BodyEventDelta::zero()
        },
    );
    let (patch_b, receptors_b) = sealed_reward(
        handle_b,
        &phenotype,
        &physiology_b,
        &frame_b,
        &tick_b,
        1,
        BodyEventDelta {
            nutrition: 1.0,
            ..BodyEventDelta::zero()
        },
    );
    backend
        .apply_sealed_outcome_batch(&[
            (handle_a, &patch_a, &receptors_a),
            (handle_b, &patch_b, &receptors_b),
        ])
        .unwrap();

    let slot_b_before = backend.slot_full_digest_for_test(handle_b).unwrap();
    let (_replay_a, request_a, staged_a) = stage_sleep(&mut backend, handle_a, 1);
    backend
        .commit_sleep_consolidation(handle_a, &request_a, &staged_a.staged)
        .unwrap();
    assert_eq!(
        backend.slot_full_digest_for_test(handle_b).unwrap(),
        slot_b_before
    );

    let slot_a_after = backend.slot_full_digest_for_test(handle_a).unwrap();
    let (_replay_b, request_b, staged_b) = stage_sleep(&mut backend, handle_b, 1);
    backend
        .commit_sleep_consolidation(handle_b, &request_b, &staged_b.staged)
        .unwrap();
    assert_eq!(
        backend.slot_full_digest_for_test(handle_a).unwrap(),
        slot_a_after
    );
}

#[test]
fn replay_learning_payload_changes_behavior_within_post_wake_probe_window() {
    const LOGIT_TOLERANCE: f32 = 1.0e-5;
    const POST_WAKE_PROBE_TICKS: u64 = 32;

    let phenotype = support::controlled_learning_n512_phenotype(1.0);
    let organisms = [alife_core::OrganismId(5_008), alife_core::OrganismId(5_009)];
    let mut backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let handles =
        organisms.map(|organism| backend.insert_brain(organism, phenotype.clone()).unwrap());
    let physiology_fixture = support::test_physiology(5_008, &phenotype).unwrap();

    for exposure in 0_u64..8 {
        let tick_raw = 5_000 + exposure * 2;
        let sources = organisms.map(|organism| {
            support::perception_frame_for_profile_at_tick(
                organism.raw(),
                tick_raw,
                alife_core::SensorProfile::PrivilegedAffordanceV1,
                true,
                2,
            )
        });
        let [(frame_a, tick_a), (frame_b, tick_b)] = paired_chemistry_tick(
            &mut backend,
            handles,
            &phenotype,
            [&physiology_fixture, &physiology_fixture],
            [&sources[0], &sources[1]],
        )
        .unwrap();
        let (patch_a, receptors_a) = sealed_reward(
            handles[0],
            &phenotype,
            &physiology_fixture,
            &frame_a,
            &tick_a,
            exposure + 1,
            BodyEventDelta {
                nutrition: 1.0,
                ..BodyEventDelta::zero()
            },
        );
        let (patch_b, receptors_b) = sealed_reward(
            handles[1],
            &phenotype,
            &physiology_fixture,
            &frame_b,
            &tick_b,
            exposure + 1,
            BodyEventDelta {
                nutrition: 1.0,
                ..BodyEventDelta::zero()
            },
        );
        backend
            .apply_sealed_outcome_batch(&[
                (handles[0], &patch_a, &receptors_a),
                (handles[1], &patch_b, &receptors_b),
            ])
            .unwrap();
    }

    assert_eq!(
        backend
            .read_active_fast_weights_for_test(handles[0])
            .unwrap(),
        backend
            .read_active_fast_weights_for_test(handles[1])
            .unwrap(),
        "identically trained slots must enter sleep with identical fast weights"
    );

    backend
        .zero_replay_eligibility_samples_for_test(handles[1])
        .unwrap();
    let replayed_batch = backend.build_sleep_replay_batch(handles[0]).unwrap();
    let ablated_batch = backend.build_sleep_replay_batch(handles[1]).unwrap();
    assert!(replayed_batch
        .eligibility_samples
        .iter()
        .any(|sample| sample.eligibility_q15 != 0));
    assert!(ablated_batch
        .eligibility_samples
        .iter()
        .all(|sample| sample.eligibility_q15 == 0));

    let (_replayed_batch, replayed_request, replayed_staged) =
        stage_sleep(&mut backend, handles[0], 1);
    let (_ablated_batch, ablated_request, ablated_staged) =
        stage_sleep(&mut backend, handles[1], 1);
    let replayed_receipt = backend
        .commit_sleep_consolidation(handles[0], &replayed_request, &replayed_staged.staged)
        .unwrap();
    let ablated_receipt = backend
        .commit_sleep_consolidation(handles[1], &ablated_request, &ablated_staged.staged)
        .unwrap();
    assert!(replayed_receipt.replay_induced_fast_l1 > 0.0);
    assert_eq!(ablated_receipt.replay_induced_fast_l1, 0.0);

    let mut max_post_wake_delta = 0.0_f32;
    for offset in 0..POST_WAKE_PROBE_TICKS {
        let sources = organisms.map(|organism| {
            support::perception_frame_for_profile_at_tick(
                organism.raw(),
                5_100 + offset,
                alife_core::SensorProfile::PrivilegedAffordanceV1,
                true,
                2,
            )
        });
        let ticks = paired_chemistry_tick(
            &mut backend,
            handles,
            &phenotype,
            [&physiology_fixture, &physiology_fixture],
            [&sources[0], &sources[1]],
        )
        .unwrap();
        max_post_wake_delta = max_post_wake_delta
            .max((ticks[0].1.selection.logit - ticks[1].1.selection.logit).abs());
        for (_, tick) in &ticks {
            backend
                .discard_pending_eligibility(tick.handle, tick.pending_eligibility.identity())
                .unwrap();
        }
    }
    assert!(
        max_post_wake_delta > LOGIT_TOLERANCE,
        "GPU replay payload must causally alter the selected logit within the bounded post-wake probe window: max_delta={max_post_wake_delta}",
    );
}

#[test]
fn gpu_test_brain_sleep_delegate_runs_the_full_request_transaction() {
    let phenotype = support::controlled_learning_n512_phenotype(1.0);
    let mut brain =
        support::GpuTestBrain::from_phenotype(alife_core::OrganismId(5_007), phenotype).unwrap();
    let source = support::perception_frame_for_profile_at_tick(
        5_007,
        4_000,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let (frame, tick) = brain.tick_with_frame(&source).unwrap();
    let (patch, _) = sealed_reward(
        brain.handle,
        brain.brain_phenotype(),
        brain.physiology(),
        &frame,
        &tick,
        1,
        BodyEventDelta {
            nutrition: 1.0,
            ..BodyEventDelta::zero()
        },
    );
    brain.apply_sealed_outcome(&patch).unwrap();

    let receipt = brain
        .submit_and_complete_sleep_consolidation(ConsolidationIntent { cycle_id: 1 })
        .unwrap();

    assert_eq!(receipt.generation_swaps, 1);
    assert_eq!(receipt.staged.cycle_id, 1);
    assert!(receipt.promoted_fast_l1 > 0.0);
}
