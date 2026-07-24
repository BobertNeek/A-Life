use alife_core::{
    ActionId, BoundedReplayBatch, CandidateActionFamily, CandidateFeatureDigest,
    ConsolidationDriverEvent, ConsolidationIntent, ConsolidationJobId, ConsolidationStagedOutput,
    ConsolidationState, DurationTicks, GpuConsolidationRequest, HomeostaticParameters,
    HomeostaticSnapshot, NeuromodulatorSample, NormalizedScalar, PerceptionFrameDigest,
    PhenotypeHash, ReplayCapturePlan, ScaffoldContractError, SleepConsolidationConfig,
    SleepController, SleepPhase, SleepReplayEvent, SleepReplayJournal, SleepState, Tick, Validate,
    GPU_CONSOLIDATION_REQUEST_SCHEMA_VERSION, SLEEP_CONSOLIDATION_SCHEMA_VERSION,
};

fn replay_event(sequence: u64) -> SleepReplayEvent {
    SleepReplayEvent {
        sequence_id: alife_core::ExperienceSequenceId(sequence),
        originating_tick: Tick::new(sequence - 1),
        frame_digest: PerceptionFrameDigest([sequence, 2, 3, 4]),
        candidate_feature_digest: CandidateFeatureDigest([sequence, 9]),
        action_id: ActionId(200 + sequence as u32),
        family: CandidateActionFamily::Contact,
        modulator: NeuromodulatorSample::from_components(0.4, 0.1, 0.2, 0.0, 0.3).unwrap(),
    }
}

#[test]
fn sleep_phase_and_consolidation_state_raw_values_are_frozen() {
    for (phase, raw) in [
        (SleepPhase::Awake, 1),
        (SleepPhase::EnteringSleep, 2),
        (SleepPhase::Consolidating, 3),
        (SleepPhase::Waking, 4),
        (SleepPhase::ForcedRecoverySleep, 5),
    ] {
        assert_eq!(phase.raw(), raw);
        assert_eq!(SleepPhase::try_from_raw(raw).unwrap(), phase);
    }
    assert!(SleepPhase::try_from_raw(0).is_err());
    assert!(SleepPhase::try_from_raw(6).is_err());
    assert_eq!(ConsolidationState::None.kind_raw(), 0);
}

#[test]
fn consolidation_job_id_rejects_zero_through_constructor_and_serde() {
    assert_eq!(
        ConsolidationJobId::try_from_raw(0),
        Err(ScaffoldContractError::InvalidId)
    );
    let decoded = serde_json::from_str::<ConsolidationJobId>("0");
    assert!(decoded.is_err());
    let valid = ConsolidationJobId::try_from_raw(7).unwrap();
    assert_eq!(serde_json::to_string(&valid).unwrap(), "7");
    assert_eq!(
        serde_json::from_str::<ConsolidationJobId>("7").unwrap(),
        valid
    );

    let config = bincode::config::standard();
    let zero = bincode::serde::encode_to_vec(0_u64, config).unwrap();
    assert!(bincode::serde::decode_from_slice::<ConsolidationJobId, _>(&zero, config).is_err());
    let encoded = bincode::serde::encode_to_vec(valid, config).unwrap();
    let (decoded, consumed) =
        bincode::serde::decode_from_slice::<ConsolidationJobId, _>(&encoded, config).unwrap();
    assert_eq!(decoded, valid);
    assert_eq!(consumed, encoded.len());
}

fn valid_request() -> GpuConsolidationRequest {
    valid_request_for(4, [9, 10, 11, 12])
}

fn valid_request_for(cycle_id: u64, replay_digest: [u64; 4]) -> GpuConsolidationRequest {
    let mut request = GpuConsolidationRequest {
        schema_version: GPU_CONSOLIDATION_REQUEST_SCHEMA_VERSION,
        request_flags: 0,
        cycle_id,
        phenotype_hash: PhenotypeHash([1, 2, 3, 4]),
        input_generation: 9,
        expected_output_generation: 10,
        input_digest: [5, 6, 7, 8],
        replay_digest,
        max_replay_events: 8,
        max_replay_eligibility_samples: 32,
        request_digest: [0; 4],
    };
    request.request_digest = request.recompute_request_digest().unwrap();
    request
}

fn automatic_test_controller() -> SleepController {
    SleepController::new(SleepConsolidationConfig {
        fatigue_threshold: NormalizedScalar::new(0.8).unwrap(),
        sleep_pressure_threshold: NormalizedScalar::new(0.8).unwrap(),
        entering_duration: DurationTicks::new(1),
        waking_duration: DurationTicks::new(1),
        ..SleepConsolidationConfig::reference()
    })
    .unwrap()
}

fn fatigued_homeostasis(tick: Tick) -> HomeostaticSnapshot {
    let mut drives = alife_core::DriveSnapshot::baseline();
    drives.fatigue = 0.99;
    let mut hormones = alife_core::EndocrineSnapshot::baseline();
    hormones.sleep_pressure = 0.99;
    HomeostaticSnapshot::new(tick, drives, hormones).unwrap()
}

fn assert_sleep_state_round_trip(state: SleepState) {
    state.validate_contract().unwrap();
    let json = serde_json::to_vec(&state).unwrap();
    let from_json: SleepState = serde_json::from_slice(&json).unwrap();
    assert_eq!(from_json, state);

    let config = bincode::config::standard();
    let bytes = bincode::serde::encode_to_vec(state, config).unwrap();
    let (from_binary, consumed) =
        bincode::serde::decode_from_slice::<SleepState, _>(&bytes, config).unwrap();
    assert_eq!(from_binary, state);
    assert_eq!(consumed, bytes.len());
}

#[test]
fn every_sleep_phase_and_consolidation_stage_round_trips() {
    let mut controller = automatic_test_controller();
    assert_sleep_state_round_trip(controller.state());

    controller
        .evaluate_homeostasis(
            &fatigued_homeostasis(Tick::ZERO),
            HomeostaticParameters::reference(),
            Tick::ZERO,
        )
        .unwrap()
        .unwrap();
    assert_eq!(controller.state().phase, SleepPhase::EnteringSleep);
    assert_sleep_state_round_trip(controller.state());

    controller.advance(Tick::new(1)).unwrap().unwrap();
    assert_eq!(controller.state().phase, SleepPhase::Consolidating);
    assert_sleep_state_round_trip(controller.state());

    let intent = ConsolidationIntent { cycle_id: 1 };
    let replay_digest = [41, 42, 43, 44];
    controller
        .apply_consolidation_driver_event(ConsolidationDriverEvent::ReplayAssetPersisted {
            intent,
            replay_digest,
            replay_event_count: 1,
            replay_eligibility_sample_count: 1,
        })
        .unwrap();
    assert_sleep_state_round_trip(controller.state());

    let request = valid_request_for(intent.cycle_id, replay_digest);
    controller
        .apply_consolidation_driver_event(ConsolidationDriverEvent::Prepared { request })
        .unwrap();
    assert_sleep_state_round_trip(controller.state());

    let job_id = ConsolidationJobId::try_from_raw(7).unwrap();
    controller
        .apply_consolidation_driver_event(ConsolidationDriverEvent::Submitted { request, job_id })
        .unwrap();
    assert_sleep_state_round_trip(controller.state());

    let mut staged = valid_staged(&request);
    staged.job_id = job_id;
    staged.staging_digest = staged.recompute_staging_digest(&request, 1, 1).unwrap();
    controller
        .apply_consolidation_driver_event(ConsolidationDriverEvent::Completed { request, staged })
        .unwrap();
    assert_sleep_state_round_trip(controller.state());

    controller
        .apply_consolidation_driver_event(ConsolidationDriverEvent::Committed {
            cycle_id: request.cycle_id,
            output_generation: staged.output_generation,
            output_digest: staged.output_digest,
        })
        .unwrap();
    assert_sleep_state_round_trip(controller.state());

    controller.advance(Tick::new(2)).unwrap().unwrap();
    assert_eq!(controller.state().phase, SleepPhase::Waking);
    assert_sleep_state_round_trip(controller.state());

    controller.advance(Tick::new(3)).unwrap().unwrap();
    assert_eq!(controller.state().phase, SleepPhase::Awake);
    assert_sleep_state_round_trip(controller.state());
}

#[test]
fn driver_events_are_ordered_replay_protected_and_commit_exactly_once() {
    let mut controller = automatic_test_controller();
    controller
        .evaluate_homeostasis(
            &fatigued_homeostasis(Tick::ZERO),
            HomeostaticParameters::reference(),
            Tick::ZERO,
        )
        .unwrap();
    controller.advance(Tick::new(1)).unwrap();
    let intent = ConsolidationIntent { cycle_id: 1 };
    let replay_digest = [61, 62, 63, 64];
    let request = valid_request_for(intent.cycle_id, replay_digest);
    let job_id = ConsolidationJobId::try_from_raw(9).unwrap();

    let before = controller.state();
    assert!(controller
        .apply_consolidation_driver_event(ConsolidationDriverEvent::Submitted { request, job_id })
        .is_err());
    assert_eq!(controller.state(), before);

    let replay_persisted = ConsolidationDriverEvent::ReplayAssetPersisted {
        intent,
        replay_digest,
        replay_event_count: 1,
        replay_eligibility_sample_count: 1,
    };
    controller
        .apply_consolidation_driver_event(replay_persisted)
        .unwrap();
    let pending = controller.state();
    assert!(controller
        .apply_consolidation_driver_event(replay_persisted)
        .is_err());
    assert_eq!(controller.state(), pending);

    let prepared = ConsolidationDriverEvent::Prepared { request };
    controller
        .apply_consolidation_driver_event(prepared)
        .unwrap();
    let prepared_state = controller.state();
    assert!(controller
        .apply_consolidation_driver_event(prepared)
        .is_err());
    assert_eq!(controller.state(), prepared_state);

    controller
        .apply_consolidation_driver_event(ConsolidationDriverEvent::Submitted { request, job_id })
        .unwrap();
    let mut staged = valid_staged(&request);
    staged.job_id = job_id;
    staged.staging_digest = staged.recompute_staging_digest(&request, 1, 1).unwrap();
    controller
        .apply_consolidation_driver_event(ConsolidationDriverEvent::Completed { request, staged })
        .unwrap();
    let committed = ConsolidationDriverEvent::Committed {
        cycle_id: request.cycle_id,
        output_generation: staged.output_generation,
        output_digest: staged.output_digest,
    };
    controller
        .apply_consolidation_driver_event(committed)
        .unwrap();
    let committed_state = controller.state();
    assert!(controller
        .apply_consolidation_driver_event(committed)
        .is_err());
    assert_eq!(controller.state(), committed_state);

    controller.advance(Tick::new(2)).unwrap();
    controller.advance(Tick::new(3)).unwrap();
    let awake = controller.state();
    assert_eq!(awake.phase, SleepPhase::Awake);
    assert_eq!(awake.cycles_completed, 1);
    assert_eq!(awake.last_consolidated_cycle_id, 1);
    assert!(controller
        .apply_consolidation_driver_event(committed)
        .is_err());
    assert_eq!(controller.state(), awake);
}

#[test]
fn restored_submitted_cycle_rebinds_only_its_process_local_job() {
    let config = SleepConsolidationConfig {
        fatigue_threshold: NormalizedScalar::new(0.8).unwrap(),
        sleep_pressure_threshold: NormalizedScalar::new(0.8).unwrap(),
        entering_duration: DurationTicks::new(1),
        waking_duration: DurationTicks::new(1),
        ..SleepConsolidationConfig::reference()
    };
    let mut original = SleepController::new(config).unwrap();
    original
        .evaluate_homeostasis(
            &fatigued_homeostasis(Tick::ZERO),
            HomeostaticParameters::reference(),
            Tick::ZERO,
        )
        .unwrap();
    original.advance(Tick::new(1)).unwrap();
    let intent = ConsolidationIntent { cycle_id: 1 };
    let replay_digest = [81, 82, 83, 84];
    original
        .apply_consolidation_driver_event(ConsolidationDriverEvent::ReplayAssetPersisted {
            intent,
            replay_digest,
            replay_event_count: 1,
            replay_eligibility_sample_count: 1,
        })
        .unwrap();
    let request = valid_request_for(intent.cycle_id, replay_digest);
    original
        .apply_consolidation_driver_event(ConsolidationDriverEvent::Prepared { request })
        .unwrap();
    let lost_job_id = ConsolidationJobId::try_from_raw(17).unwrap();
    original
        .apply_consolidation_driver_event(ConsolidationDriverEvent::Submitted {
            request,
            job_id: lost_job_id,
        })
        .unwrap();

    let mut restored = SleepController::restore(config, original.state()).unwrap();
    assert_eq!(restored.state(), original.state());
    let recovered_job_id = ConsolidationJobId::try_from_raw(23).unwrap();
    restored
        .apply_consolidation_driver_event(ConsolidationDriverEvent::RecoveredSubmitted {
            request,
            lost_job_id,
            recovered_job_id,
        })
        .unwrap();
    assert_eq!(
        restored.state().consolidation,
        ConsolidationState::Submitted {
            request,
            job_id: recovered_job_id,
        }
    );

    let state_after_rebind = restored.state();
    assert!(restored
        .apply_consolidation_driver_event(ConsolidationDriverEvent::RecoveredSubmitted {
            request,
            lost_job_id,
            recovered_job_id: ConsolidationJobId::try_from_raw(29).unwrap(),
        })
        .is_err());
    assert_eq!(restored.state(), state_after_rebind);
}

fn valid_staged(request: &GpuConsolidationRequest) -> ConsolidationStagedOutput {
    let mut staged = ConsolidationStagedOutput {
        job_id: ConsolidationJobId::try_from_raw(3).unwrap(),
        output_generation: request.expected_output_generation,
        output_weight_bank: 1,
        output_digest: [13, 14, 15, 16],
        eligibility_reset_generation: 8,
        output_eligibility_bank: 0,
        eligibility_output_digest: [17, 18, 19, 20],
        replay_journal_generation: 7,
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

#[test]
fn request_and_staging_digests_reject_every_identity_tamper() {
    let request = valid_request();
    request.validate_contract().unwrap();
    let staged = valid_staged(&request);
    staged.validate_against(&request, 1, 1).unwrap();

    let mut request_tamper = request;
    request_tamper.input_generation += 1;
    assert!(request_tamper.validate_contract().is_err());

    let mut staged_tamper = staged;
    staged_tamper.output_weight_bank ^= 1;
    assert!(staged_tamper.validate_against(&request, 1, 1).is_err());

    let mut non_finite = staged;
    non_finite.promoted_fast_l1_bits = f32::NAN.to_bits();
    assert!(non_finite.recompute_staging_digest(&request, 1, 1).is_err());
}

#[test]
fn phase_only_sleep_state_migrates_to_durable_cycle_identity() {
    let legacy = serde_json::json!({
        "schema_version": SLEEP_CONSOLIDATION_SCHEMA_VERSION,
        "phase": "EnteringSleep",
        "phase_started_tick": 10,
        "entered_sleep_tick": 10,
        "cycles_completed": 3,
        "last_trigger": "FatigueThreshold"
    });
    let migrated: SleepState = serde_json::from_value(legacy).unwrap();
    assert_eq!(migrated.active_cycle_id, 4);
    assert_eq!(migrated.last_consolidated_cycle_id, 3);
    assert_eq!(migrated.consolidation, ConsolidationState::None);

    let round_trip: SleepState =
        serde_json::from_slice(&serde_json::to_vec(&migrated).unwrap()).unwrap();
    assert_eq!(round_trip, migrated);
}

#[test]
fn replay_q15_encoding_is_exact_bounded_and_canonical() {
    for (value, expected) in [
        (-1.0, -32_767),
        (-0.5, -16_384),
        (-0.0, 0),
        (0.0, 0),
        (0.5, 16_384),
        (1.0, 32_767),
    ] {
        let encoded = alife_core::encode_replay_eligibility_q15(value).unwrap();
        assert_eq!(encoded, expected);
        let decoded = alife_core::decode_replay_eligibility_q15(encoded).unwrap();
        assert!((decoded - value).abs() <= 1.0 / 32_767.0);
    }
    assert!(alife_core::encode_replay_eligibility_q15(f32::NAN).is_err());
    assert!(alife_core::decode_replay_eligibility_q15(i16::MIN).is_err());
}

#[test]
fn replay_capture_plan_accepts_65536_events_and_rejects_overflow() {
    let boundary = ReplayCapturePlan::try_new(vec![3], 1, 65_536, 65_536).unwrap();
    assert_eq!(boundary.event_capacity(), 65_536);
    assert!(ReplayCapturePlan::try_new(vec![3], 1, 65_537, 65_537).is_err());
    assert!(ReplayCapturePlan::try_new(vec![3, 7], 2, u32::MAX, u32::MAX).is_err());
}

#[test]
fn replay_ring_wraps_chronologically_and_replaces_every_span_sample() {
    let plan = ReplayCapturePlan::try_new(vec![3, 7], 2, 3, 6).unwrap();
    let mut journal = SleepReplayJournal::new(plan).unwrap();
    journal.push(replay_event(1), &[0.1, -0.1]).unwrap();
    journal.push(replay_event(2), &[0.2, -0.2]).unwrap();
    journal.push(replay_event(3), &[0.3, -0.3]).unwrap();
    journal.push(replay_event(4), &[0.4, -0.4]).unwrap();

    let batch = journal.build_bounded_batch(3, 6, 8).unwrap();
    assert_eq!(
        batch
            .events
            .iter()
            .map(|event| event.sequence_id.raw())
            .collect::<Vec<_>>(),
        vec![2, 3, 4]
    );
    assert_eq!(batch.synapse_spans.len(), 2);
    assert_eq!(batch.synapse_spans[0].local_synapse_id, 3);
    assert_eq!(batch.synapse_spans[1].local_synapse_id, 7);
    assert!(batch
        .synapse_spans
        .iter()
        .all(|span| span.sample_count == 3));
    assert_eq!(
        batch.eligibility_samples[0..3]
            .iter()
            .map(|sample| sample.event_index)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(
        batch.eligibility_samples[3..6]
            .iter()
            .map(|sample| sample.event_index)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(
        batch.eligibility_samples[0].eligibility_q15,
        alife_core::encode_replay_eligibility_q15(0.2).unwrap()
    );
    assert_eq!(
        batch.eligibility_samples[3].eligibility_q15,
        alife_core::encode_replay_eligibility_q15(-0.2).unwrap()
    );
    batch.validate_contract(3, 6, 8).unwrap();

    journal.push(replay_event(5), &[0.5, -0.5]).unwrap();
    journal.push(replay_event(6), &[0.6, -0.6]).unwrap();
    journal.push(replay_event(7), &[0.7, -0.7]).unwrap();
    let twice_wrapped = journal.build_bounded_batch(3, 6, 8).unwrap();
    assert_eq!(
        twice_wrapped
            .events
            .iter()
            .map(|event| event.sequence_id.raw())
            .collect::<Vec<_>>(),
        vec![5, 6, 7]
    );
    assert_eq!(
        twice_wrapped.eligibility_samples[0].eligibility_q15,
        alife_core::encode_replay_eligibility_q15(0.5).unwrap()
    );
    assert_eq!(
        twice_wrapped.eligibility_samples[3].eligibility_q15,
        alife_core::encode_replay_eligibility_q15(-0.5).unwrap()
    );
}

#[test]
fn bounded_replay_batch_rejects_digest_span_and_event_index_tampering() {
    let plan = ReplayCapturePlan::try_new(vec![1], 1, 2, 2).unwrap();
    let mut journal = SleepReplayJournal::new(plan).unwrap();
    journal.push(replay_event(1), &[0.25]).unwrap();
    let batch = journal.build_bounded_batch(2, 2, 2).unwrap();

    let mut digest_tamper = batch.clone();
    digest_tamper.canonical_digest[0] ^= 1;
    assert!(digest_tamper.validate_contract(2, 2, 2).is_err());

    let mut index_tamper = batch.clone();
    index_tamper.eligibility_samples[0].event_index = 1;
    index_tamper.canonical_digest = index_tamper.recompute_canonical_digest().unwrap();
    assert!(index_tamper.validate_contract(2, 2, 2).is_err());

    let mut span_tamper: BoundedReplayBatch = batch;
    span_tamper.synapse_spans[0].reserved = 1;
    span_tamper.canonical_digest = span_tamper.recompute_canonical_digest().unwrap();
    assert!(span_tamper.validate_contract(2, 2, 2).is_err());
}

#[test]
fn consolidation_request_schema_constant_is_frozen() {
    assert_eq!(GPU_CONSOLIDATION_REQUEST_SCHEMA_VERSION, 1);
}
