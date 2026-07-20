//! Real-hardware acceptance for GPU replay and exactly-once sleep consolidation.
#![cfg(feature = "gpu-tests")]

mod support;

use alife_core::{
    BrainGenome, Confidence, ConsolidationIntent, DecisionSnapshot, DevelopmentState,
    EndocrineDelta, ExperiencePatch, ExperiencePatchBuilder, ExperienceSequenceId,
    HomeostaticDelta, NeuralActionSelection, NormalizedScalar, PhysicalActionOutcome,
    PhysicalContactKind, PostActionOutcome, PreActionSnapshot, SignedValence, Tick, Vec3f,
};
use alife_gpu_backend::{
    GpuClosedLoopBackend, GpuConsolidationRequestRecord, GpuReplayEventRecord,
    GpuReplaySynapseSpanRecord, GpuSleepHeader,
};

fn sealed_reward(
    handle: alife_gpu_backend::GpuBrainHandle,
    frame: &alife_core::PerceptionFrame,
    tick: &alife_gpu_backend::GpuClosedLoopTick,
    sequence_raw: u64,
    reward: f32,
) -> ExperiencePatch {
    let sequence_id = ExperienceSequenceId(sequence_raw);
    let genome = BrainGenome::scaffold(13, handle.class_id());
    let development = DevelopmentState::new(
        genome.id,
        frame.tick(),
        NormalizedScalar::new(0.35).unwrap(),
    );
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
    let outcome = PostActionOutcome::new(
        handle.organism_id(),
        sequence_id,
        Tick::new(frame.tick().raw() + 1),
        true,
        PhysicalActionOutcome {
            contact: PhysicalContactKind::None,
            target_entity: None,
            displacement: Vec3f::ZERO,
            collision_normal: None,
            energy_cost: NormalizedScalar::new(0.0).unwrap(),
        },
        HomeostaticDelta {
            drives: alife_core::DriveDelta::zero(),
            hormones: EndocrineDelta::zero(),
        },
        SignedValence::new(reward).unwrap(),
        NormalizedScalar::new(0.0).unwrap(),
        NormalizedScalar::new(0.0).unwrap(),
        SignedValence::new(0.0).unwrap(),
        NormalizedScalar::new(0.0).unwrap(),
    )
    .unwrap();
    ExperiencePatchBuilder::new(sequence_id)
        .record_pre_action(pre_action)
        .unwrap()
        .record_decision(decision)
        .unwrap()
        .record_outcome(outcome)
        .unwrap()
        .seal()
        .unwrap()
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
    let frame = support::perception_frame_for_profile_at_tick(
        organism_raw,
        2_000,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let tick = backend
        .tick_batch(&[(handle, frame.clone())])
        .unwrap()
        .remove(0);
    let patch = sealed_reward(handle, &frame, &tick, 1, 0.8);
    let receipt = backend.apply_sealed_outcome(handle, &patch).unwrap();
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
    assert_eq!(std::mem::size_of::<GpuReplayEventRecord>(), 96);
    assert_eq!(std::mem::align_of::<GpuReplayEventRecord>(), 16);
    assert_eq!(
        std::mem::offset_of!(GpuReplayEventRecord, modulator_value),
        92
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
        replay
            .events
            .iter()
            .any(|event| event.modulator.value() != 0.0),
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
        .insert_brain(alife_core::OrganismId(5_006), phenotype)
        .unwrap();
    let frame_a = support::perception_frame_for_profile_at_tick(
        5_005,
        3_000,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let frame_b = support::perception_frame_for_profile_at_tick(
        5_006,
        3_000,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let ticks = backend
        .tick_batch(&[(handle_a, frame_a.clone()), (handle_b, frame_b.clone())])
        .unwrap();
    let patch_a = sealed_reward(handle_a, &frame_a, &ticks[0], 1, 0.8);
    let patch_b = sealed_reward(handle_b, &frame_b, &ticks[1], 1, 0.8);
    backend
        .apply_sealed_outcome_batch(&[(handle_a, &patch_a), (handle_b, &patch_b)])
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

    for exposure in 0_u64..8 {
        let tick_raw = 5_000 + exposure * 2;
        let frames = organisms.map(|organism| {
            support::perception_frame_for_profile_at_tick(
                organism.raw(),
                tick_raw,
                alife_core::SensorProfile::PrivilegedAffordanceV1,
                true,
                2,
            )
        });
        let ticks = backend
            .tick_batch(&[
                (handles[0], frames[0].clone()),
                (handles[1], frames[1].clone()),
            ])
            .unwrap();
        let patches = [
            sealed_reward(handles[0], &frames[0], &ticks[0], exposure + 1, 0.8),
            sealed_reward(handles[1], &frames[1], &ticks[1], exposure + 1, 0.8),
        ];
        backend
            .apply_sealed_outcome_batch(&[(handles[0], &patches[0]), (handles[1], &patches[1])])
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
        let frames = organisms.map(|organism| {
            support::perception_frame_for_profile_at_tick(
                organism.raw(),
                5_100 + offset,
                alife_core::SensorProfile::PrivilegedAffordanceV1,
                true,
                2,
            )
        });
        let ticks = backend
            .tick_batch(&[
                (handles[0], frames[0].clone()),
                (handles[1], frames[1].clone()),
            ])
            .unwrap();
        max_post_wake_delta =
            max_post_wake_delta.max((ticks[0].selection.logit - ticks[1].selection.logit).abs());
        for tick in &ticks {
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
    let frame = support::perception_frame_for_profile_at_tick(
        5_007,
        4_000,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let tick = brain.tick(&frame).unwrap();
    let patch = sealed_reward(brain.handle, &frame, &tick, 1, 0.8);
    brain.apply_sealed_outcome(&patch).unwrap();

    let receipt = brain
        .submit_and_complete_sleep_consolidation(ConsolidationIntent { cycle_id: 1 })
        .unwrap();

    assert_eq!(receipt.generation_swaps, 1);
    assert_eq!(receipt.staged.cycle_id, 1);
    assert!(receipt.promoted_fast_l1 > 0.0);
}
