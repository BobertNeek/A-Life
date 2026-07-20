//! Exact activity-cost, deterministic-throttle, and replay contracts.

mod support;

use alife_core::{
    BrainActivityPolicyV1, BrainCapacityClass, BrainDispatchIdentity, BrainWorkCounters,
    BrainWorkReceipt, GpuPressureSample, GpuPressureSampleInput, NeuralThrottleDecision,
    NeuralThrottleLevel, OrganismId, SensorProfile, BRAIN_ATP_BASAL_DEBIT_Q16, BRAIN_ATP_Q16_MAX,
};
use alife_gpu_backend::{
    derive_executed_work, GpuActivityDispatchHeader, GpuClassBucketPlan,
    CLOSED_LOOP_RECURRENT_WGSL, GPU_ACTIVITY_DISPATCH_HEADER_WORDS,
};

fn identity(
    phenotype: &alife_core::BrainPhenotype,
    slot: u32,
    generation: u32,
    cursor: u64,
) -> BrainDispatchIdentity {
    BrainDispatchIdentity {
        organism_id_raw: u64::from(slot) + 1,
        tick: cursor + 100,
        class_id_raw: phenotype.brain_class_id().raw(),
        handle_slot: slot,
        handle_generation: generation,
        sequence_cursor: cursor,
        dispatch_generation: cursor + 1,
        frame_digest: [cursor + 11, cursor + 12, cursor + 13, cursor + 14],
    }
}

#[allow(clippy::too_many_arguments)]
fn pressure(
    policy: &BrainActivityPolicyV1,
    identity: BrainDispatchIdentity,
    completed_gpu_time_ns: u64,
    queue_depth: u32,
    logical_heap_pressure_q16: u32,
    brain_atp_fraction_q16: u32,
) -> GpuPressureSample {
    GpuPressureSample::try_new(
        policy,
        GpuPressureSampleInput {
            identity,
            source_dispatch_generation: identity.dispatch_generation.saturating_sub(1),
            source_frame_digest: if identity.dispatch_generation == 1 {
                [0; 4]
            } else {
                identity.frame_digest.map(|word| word - 1)
            },
            completed_gpu_time_ns,
            queue_depth,
            logical_heap_used: u64::from(logical_heap_pressure_q16),
            logical_heap_capacity: 65_535,
            brain_atp_remaining_q16: brain_atp_fraction_q16,
            brain_atp_capacity_q16: 65_535,
        },
    )
    .unwrap()
}

fn decision_for(
    phenotype: &alife_core::BrainPhenotype,
    slot: u32,
    generation: u32,
    cursor: u64,
    pressure_inputs: (u64, u32, u32, u32),
) -> NeuralThrottleDecision {
    let (completed_gpu_time_ns, queue_depth, logical_heap_pressure_q16, brain_atp_fraction_q16) =
        pressure_inputs;
    let capacity = BrainCapacityClass::production_for_id(phenotype.brain_class_id()).unwrap();
    let policy = BrainActivityPolicyV1::production_v1();
    let identity = identity(phenotype, slot, generation, cursor);
    let sample = pressure(
        &policy,
        identity,
        completed_gpu_time_ns,
        queue_depth,
        logical_heap_pressure_q16,
        brain_atp_fraction_q16,
    );
    NeuralThrottleDecision::derive(&policy, phenotype, capacity.execution(), identity, sample)
        .unwrap()
}

#[test]
fn repeated_microsteps_charge_repeated_executed_work() {
    let phenotype = support::phenotype_for_capacity_at_maturation(
        BrainCapacityClass::n512(),
        4_501,
        0.35,
        SensorProfile::GroundedObjectSlotsV1,
    );
    let hot_route = phenotype
        .projections()
        .iter()
        .find(|route| route.update_cadence() == alife_core::UpdateCadence::Hot60Hz)
        .unwrap()
        .route_index();
    let one = derive_executed_work(&phenotype, 1, &[hot_route], 3, 3).unwrap();
    let three = derive_executed_work(&phenotype, 3, &[hot_route], 3, 3).unwrap();

    assert_eq!(three.neuron_updates, one.neuron_updates * 3);
    assert_eq!(three.tile_visits, one.tile_visits * 3);
    assert_eq!(three.synapse_ops, one.synapse_ops * 3);
    assert_eq!(three.decoder_candidate_ops, one.decoder_candidate_ops);
    assert_eq!(three.memory_context_ops, one.memory_context_ops);
    assert!(
        BrainActivityPolicyV1::production_v1()
            .cost
            .neural_cost_q24(&three)
            .unwrap()
            > BrainActivityPolicyV1::production_v1()
                .cost
                .neural_cost_q24(&one)
                .unwrap()
    );
}

#[test]
fn throttle_never_violates_microstep_floor_or_drops_essential_routes() {
    let phenotype = support::phenotype_for_capacity_at_maturation(
        BrainCapacityClass::n1024(),
        4_502,
        0.35,
        SensorProfile::GroundedObjectSlotsV1,
    );
    let capacity = BrainCapacityClass::production_for_id(phenotype.brain_class_id()).unwrap();
    for (index, (time, queue, heap, atp)) in [
        (0, 0, 0, 65_535),
        (2_000_000, 0, 0, 65_535),
        (0, 2, 0, 65_535),
        (0, 0, 49_152, 65_535),
        (0, 0, 0, 16_383),
        (8_000_000, 4, 58_982, 0),
    ]
    .into_iter()
    .enumerate()
    {
        let decision = decision_for(&phenotype, 0, 1, index as u64 + 1, (time, queue, heap, atp));
        let (min_microsteps, max_microsteps) = capacity.execution().microstep_range();
        assert!((min_microsteps..=max_microsteps).contains(&decision.microsteps));
        for route in phenotype.projections().iter().filter(|route| {
            route.priority() == alife_core::BiologicalPriority::Essential
                || NeuralThrottleDecision::route_is_mandatory(route)
        }) {
            assert!(decision.enabled_route_ids.contains(&route.route_index()));
        }
        decision
            .validate_for(&phenotype, capacity.execution())
            .unwrap();
    }
}

#[test]
fn replayed_pressure_reproduces_decisions_and_work_receipts() {
    let phenotype = support::phenotype_for_capacity_at_maturation(
        BrainCapacityClass::n512(),
        4_503,
        0.35,
        SensorProfile::GroundedObjectSlotsV1,
    );
    let capacity = BrainCapacityClass::production_for_id(phenotype.brain_class_id()).unwrap();
    let policy = BrainActivityPolicyV1::production_v1();
    let inputs = [
        (0, 0, 0, 65_535),
        (2_000_000, 1, 32_768, 49_151),
        (4_000_000, 2, 49_152, 32_767),
        (8_000_000, 4, 58_982, 16_383),
    ];

    let recorded = inputs
        .into_iter()
        .enumerate()
        .map(|(index, (time, queue, heap, atp))| {
            let identity = identity(&phenotype, 0, 1, index as u64 + 1);
            let sample = pressure(&policy, identity, time, queue, heap, atp);
            let decision = NeuralThrottleDecision::derive(
                &policy,
                &phenotype,
                capacity.execution(),
                identity,
                sample,
            )
            .unwrap();
            let work = derive_executed_work(
                &phenotype,
                decision.microsteps,
                &decision.enabled_route_ids,
                4,
                4,
            )
            .unwrap();
            let receipt = BrainWorkReceipt::try_new(&policy, &decision, work, 65_535).unwrap();
            (sample, decision, receipt)
        })
        .collect::<Vec<_>>();

    let replayed = recorded
        .iter()
        .map(|(sample, _, _)| {
            let identity = sample.dispatch_identity();
            let decision = NeuralThrottleDecision::derive(
                &policy,
                &phenotype,
                capacity.execution(),
                identity,
                *sample,
            )
            .unwrap();
            let work = derive_executed_work(
                &phenotype,
                decision.microsteps,
                &decision.enabled_route_ids,
                4,
                4,
            )
            .unwrap();
            let receipt = BrainWorkReceipt::try_new(&policy, &decision, work, 65_535).unwrap();
            (*sample, decision, receipt)
        })
        .collect::<Vec<_>>();

    assert_eq!(recorded, replayed);
}

#[test]
fn throttle_and_work_receipts_cannot_cross_apply_between_same_class_slots() {
    let phenotype = support::phenotype_for_capacity_at_maturation(
        BrainCapacityClass::n512(),
        4_504,
        0.35,
        SensorProfile::GroundedObjectSlotsV1,
    );
    let a = decision_for(&phenotype, 0, 1, 1, (0, 0, 0, 65_535));
    let b = decision_for(&phenotype, 1, 1, 1, (0, 0, 0, 65_535));
    assert_ne!(a.handle_slot, b.handle_slot);
    assert!(a
        .validate_runtime_binding(b.handle_slot, b.handle_generation)
        .is_err());

    let policy = BrainActivityPolicyV1::production_v1();
    let work = derive_executed_work(&phenotype, a.microsteps, &a.enabled_route_ids, 3, 3).unwrap();
    let receipt = BrainWorkReceipt::try_new(&policy, &a, work, 65_535).unwrap();
    assert!(receipt
        .validate_runtime_binding(b.handle_slot, b.handle_generation)
        .is_err());
}

#[test]
fn every_pressure_bucket_truth_table_row_and_q24_rounding_is_exact() {
    let phenotype = support::phenotype_for_capacity_at_maturation(
        BrainCapacityClass::n512(),
        4_505,
        0.35,
        SensorProfile::GroundedObjectSlotsV1,
    );
    let policy = BrainActivityPolicyV1::production_v1();
    let time_rows = [(0, 0), (2_000_000, 1), (4_000_000, 2), (8_000_000, 3)];
    let queue_rows = [(0, 0), (1, 1), (2, 2), (4, 3)];
    let heap_rows = [(0, 0), (32_768, 1), (49_152, 2), (58_982, 3)];
    let atp_rows = [(65_535, 0), (49_151, 1), (32_767, 2), (16_383, 3)];
    let mut cursor = 1_u64;

    for (time, time_bucket) in time_rows {
        for (queue, queue_bucket) in queue_rows {
            for (heap, heap_bucket) in heap_rows {
                for (atp, atp_bucket) in atp_rows {
                    let sample = pressure(
                        &policy,
                        identity(&phenotype, 0, 1, cursor),
                        time,
                        queue,
                        heap,
                        atp,
                    );
                    assert_eq!(sample.completed_gpu_time_bucket, time_bucket);
                    assert_eq!(sample.queue_depth_bucket, queue_bucket);
                    assert_eq!(sample.neural_heap_pressure_bucket, heap_bucket);
                    assert_eq!(sample.brain_atp_bucket, atp_bucket);
                    let severity = time_bucket
                        .max(u16::from(queue_bucket))
                        .max(u16::from(heap_bucket))
                        .max(u16::from(atp_bucket));
                    assert_eq!(
                        sample.throttle_level(),
                        match severity {
                            0 => NeuralThrottleLevel::Full,
                            1 => NeuralThrottleLevel::Reduced,
                            _ => NeuralThrottleLevel::EssentialOnly,
                        }
                    );
                    cursor += 1;
                }
            }
        }
    }

    assert_eq!(
        policy
            .cost
            .neural_cost_q24(&BrainWorkCounters::default())
            .unwrap(),
        0
    );
    assert_eq!(policy.cost.q24_to_atp_q16_round_half_up(0x80).unwrap(), 1);
    assert!(policy
        .cost
        .neural_cost_q24(&BrainWorkCounters {
            microsteps: u32::MAX,
            neuron_updates: u64::MAX,
            tile_visits: u64::MAX,
            synapse_ops: u64::MAX,
            decoder_candidate_ops: u64::MAX,
            memory_context_ops: u64::MAX,
        })
        .is_err());

    let identity = identity(&phenotype, 0, 1, cursor);
    let exact_half = GpuPressureSample::try_new(
        &policy,
        GpuPressureSampleInput {
            identity,
            source_dispatch_generation: identity.dispatch_generation - 1,
            source_frame_digest: [91, 92, 93, 94],
            completed_gpu_time_ns: 0,
            queue_depth: 0,
            logical_heap_used: 1,
            logical_heap_capacity: 2,
            brain_atp_remaining_q16: 32_768,
            brain_atp_capacity_q16: 65_536,
        },
    )
    .unwrap();
    assert_eq!(exact_half.logical_heap_pressure_q16, 32_768);
    assert_eq!(exact_half.brain_atp_fraction_q16, 32_768);
    assert_eq!(exact_half.organism_id_raw, OrganismId(1).raw());
}

#[test]
fn activity_dispatch_header_binds_the_validated_route_schedule_exactly() {
    let capacity = BrainCapacityClass::n512();
    let phenotype = support::phenotype_for_capacity_at_maturation(
        capacity,
        4_506,
        0.35,
        SensorProfile::GroundedObjectSlotsV1,
    );
    let mut plan = GpuClassBucketPlan::new(capacity, 1).unwrap();
    let slot = plan.insert_phenotype(0, 7, &phenotype).unwrap();
    let decision = decision_for(&phenotype, 0, 7, 1, (0, 0, 0, 65_535));

    let header = GpuActivityDispatchHeader::try_from_decision(
        &decision,
        &phenotype,
        capacity.execution(),
        &slot,
    )
    .unwrap();

    assert_eq!(
        std::mem::size_of::<GpuActivityDispatchHeader>() / 4,
        GPU_ACTIVITY_DISPATCH_HEADER_WORDS
    );
    assert_eq!(
        header.enabled_route_count(),
        decision.enabled_route_ids.len()
    );
    assert_eq!(header.microsteps(), decision.microsteps);
    assert_eq!(
        header.route_schedule_digest_words(),
        decision.route_schedule_digest_words()
    );
    for route in phenotype.projections() {
        assert_eq!(
            header.route_is_enabled(route.route_index()),
            decision.enabled_route_ids.contains(&route.route_index())
        );
    }
    header
        .validate_for(&decision, &phenotype, capacity.execution(), &slot)
        .unwrap();

    let mut wrong_digest = decision.clone();
    wrong_digest.route_schedule_digest[0] ^= 1;
    assert!(GpuActivityDispatchHeader::try_from_decision(
        &wrong_digest,
        &phenotype,
        capacity.execution(),
        &slot,
    )
    .is_err());
}

#[test]
fn recurrent_shader_validates_and_applies_the_activity_route_mask() {
    assert!(CLOSED_LOOP_RECURRENT_WGSL.contains("fn validate_activity_header"));
    assert!(CLOSED_LOOP_RECURRENT_WGSL.contains("fn route_enabled"));
    assert!(CLOSED_LOOP_RECURRENT_WGSL
        .contains("if (!route_enabled(activity, route_index)) { continue; }"));
    assert!(CLOSED_LOOP_RECURRENT_WGSL
        .contains("if (!route_enabled(activity, projection.route_index)) { continue; }"));
}

#[cfg(feature = "gpu-tests")]
#[test]
fn real_gpu_executes_only_the_validated_throttle_schedule() {
    pollster::block_on(async {
        let phenotype = support::phenotype_for_capacity_at_maturation(
            BrainCapacityClass::n512(),
            4_507,
            0.35,
            SensorProfile::GroundedObjectSlotsV1,
        );
        let frame =
            support::perception_frame_for_profile(1, SensorProfile::GroundedObjectSlotsV1, true, 2);
        let mut fixture = support::GpuPipelineFixture::new(&phenotype).await;

        let (full, full_decision) = fixture
            .run_slot_with_pressure(0, &frame, 0, 0, 0, 65_535)
            .await;
        fixture.restore_mutable_checkpoint();
        let (essential, essential_decision) = fixture
            .run_slot_with_pressure(0, &frame, 8_000_000, 4, 58_982, 0)
            .await;

        assert_eq!(full_decision.level, NeuralThrottleLevel::Full);
        assert_eq!(essential_decision.level, NeuralThrottleLevel::EssentialOnly);
        let full_work = derive_executed_work(
            &phenotype,
            full_decision.microsteps,
            &full_decision.enabled_route_ids,
            frame.candidates().len() as u32,
            frame.candidates().len() as u32,
        )
        .unwrap();
        let essential_work = derive_executed_work(
            &phenotype,
            essential_decision.microsteps,
            &essential_decision.enabled_route_ids,
            frame.candidates().len() as u32,
            frame.candidates().len() as u32,
        )
        .unwrap();
        assert_eq!(u64::from(full.record.active_tiles), full_work.tile_visits);
        assert_eq!(
            u64::from(full.record.active_synapses),
            full_work.synapse_ops
        );
        assert_eq!(
            u64::from(essential.record.active_tiles),
            essential_work.tile_visits
        );
        assert_eq!(
            u64::from(essential.record.active_synapses),
            essential_work.synapse_ops
        );
        assert!(essential.record.active_tiles < full.record.active_tiles);
        assert!(essential.record.active_synapses < full.record.active_synapses);
    });
}

#[cfg(feature = "gpu-tests")]
#[test]
fn real_gpu_rejects_a_tampered_route_schedule_digest() {
    pollster::block_on(async {
        let phenotype = support::phenotype_for_capacity_at_maturation(
            BrainCapacityClass::n512(),
            4_508,
            0.35,
            SensorProfile::GroundedObjectSlotsV1,
        );
        let frame =
            support::perception_frame_for_profile(1, SensorProfile::GroundedObjectSlotsV1, true, 2);
        let mut fixture = support::GpuPipelineFixture::new(&phenotype).await;

        assert_eq!(
            fixture
                .run_slot_with_tampered_activity_digest(0, &frame)
                .await,
            alife_gpu_backend::GpuClosedLoopError::SubmissionFailed
        );
    });
}

#[cfg(feature = "gpu-tests")]
#[test]
fn runtime_uses_prior_gpu_timestamps_and_debits_exact_atp_once() {
    let phenotype = support::phenotype_for_capacity_at_maturation(
        BrainCapacityClass::n512(),
        4_509,
        0.35,
        SensorProfile::GroundedObjectSlotsV1,
    );
    let mut brain = support::GpuTestBrain::from_phenotype(OrganismId(1), phenotype).unwrap();
    let first_world_atp = brain
        .backend
        .charge_world_brain_atp_tick(brain.handle, 500, false)
        .unwrap();
    assert_eq!(
        first_world_atp,
        BRAIN_ATP_Q16_MAX - BRAIN_ATP_BASAL_DEBIT_Q16
    );
    assert_eq!(
        brain
            .backend
            .charge_world_brain_atp_tick(brain.handle, 500, false),
        Ok(first_world_atp)
    );
    let first_frame = support::perception_frame_for_profile_at_tick(
        1,
        500,
        SensorProfile::GroundedObjectSlotsV1,
        true,
        2,
    );
    let first = brain.tick(&first_frame).unwrap();
    assert_eq!(first.pressure.source_dispatch_generation, 0);
    assert_eq!(first.pressure.completed_gpu_time_ns, 0);
    assert_eq!(first.work.atp_before_q16, first_world_atp);
    assert_eq!(
        first.work.atp_after_q16,
        first
            .work
            .atp_before_q16
            .checked_sub(first.work.atp_debit_q16)
            .unwrap()
    );
    brain
        .backend
        .discard_pending_eligibility(first.handle, first.pending_eligibility.identity())
        .unwrap();
    let second_world_atp = brain
        .backend
        .charge_world_brain_atp_tick(brain.handle, 501, false)
        .unwrap();
    assert_eq!(
        second_world_atp,
        first
            .work
            .atp_after_q16
            .saturating_sub(BRAIN_ATP_BASAL_DEBIT_Q16)
    );

    let second_frame = support::perception_frame_for_profile_at_tick(
        1,
        501,
        SensorProfile::GroundedObjectSlotsV1,
        true,
        2,
    );
    let second = brain.tick(&second_frame).unwrap();
    assert_eq!(
        second.pressure.source_dispatch_generation,
        first.dispatch_generation
    );
    assert_eq!(second.pressure.source_frame_digest, first.frame_digest.0);
    assert!(second.pressure.completed_gpu_time_ns > 0);
    assert_eq!(second.work.atp_before_q16, second_world_atp);
    assert_eq!(
        brain.backend.brain_atp_q16(brain.handle).unwrap(),
        second.work.atp_after_q16
    );
}

#[cfg(feature = "gpu-tests")]
#[test]
fn exhausted_activity_cursor_rejects_before_gpu_submission_without_panicking() {
    let phenotype = support::phenotype_for_capacity_at_maturation(
        BrainCapacityClass::n512(),
        4_510,
        0.35,
        SensorProfile::GroundedObjectSlotsV1,
    );
    let mut brain = support::GpuTestBrain::from_phenotype(OrganismId(1), phenotype).unwrap();
    brain
        .backend
        .force_activity_sequence_cursor_for_test(brain.handle, u64::MAX)
        .unwrap();
    let frame = support::perception_frame_for_profile_at_tick(
        1,
        600,
        SensorProfile::GroundedObjectSlotsV1,
        true,
        2,
    );
    assert_eq!(
        brain.tick(&frame),
        Err(alife_core::ScaffoldContractError::BrainActivitySequenceMismatch)
    );
    assert_eq!(brain.backend.completed_dispatch_count(), 0);
}
