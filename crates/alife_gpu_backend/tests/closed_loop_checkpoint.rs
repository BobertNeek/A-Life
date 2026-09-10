//! Real-hardware exact checkpoint/restore acceptance for the GPU-authoritative brain.
#![cfg(feature = "gpu-tests")]

mod support;

use alife_core::{
    AlphaMask, BiochemistryState, BodyEventDelta, BrainCapacityClass, BrainGenome, Confidence,
    ConsolidationIntent, DecisionSnapshot, DevelopmentState, ExperiencePatch, ExperienceSequenceId,
    JointMotorCondition, JointPhysicalOutcome, MeasuredPhysiologyTransition, NeuralActionSelection,
    NormalizedScalar, OutcomeCreditPacket, PhenotypeCompiler, PhenotypeCompilerInputs,
    PhenotypeGrowthMigration, PhysicalActionOutcome, PhysicalContactKind,
    PlasticityGenomeParameters, PostActionOutcome, PreActionSnapshot, PredictionTargetReceipt,
    SemanticStateVector, SensorProfile, SignedValence, Tick, Vec3f,
};
use alife_gpu_backend::{
    verify_research_growth_equivalence, GpuBrainRestoreRequest, GpuClosedLoopBackend,
    GpuExactPopulationCapturePollV1, GpuResearchGrowthHandoffOutcome,
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

fn install_full_recorded_pressure(
    backend: &mut GpuClosedLoopBackend,
    handle: alife_gpu_backend::GpuBrainHandle,
    final_frame: &alife_core::PerceptionFrame,
    snapshot_tick: Tick,
    prior: Option<&alife_gpu_backend::GpuClosedLoopTick>,
) {
    let activity = backend.snapshot_activity_state(handle).unwrap();
    let checkpoint = backend
        .snapshot_brain(handle, snapshot_tick)
        .unwrap()
        .into_parts();
    let pressure = alife_core::GpuPressureSample::try_new(
        backend.activity_policy(),
        alife_core::GpuPressureSampleInput {
            identity: alife_core::BrainDispatchIdentity {
                organism_id_raw: handle.organism_id().raw(),
                tick: final_frame.tick().raw(),
                class_id_raw: handle.class_id().raw(),
                handle_slot: handle.slot(),
                handle_generation: handle.generation(),
                sequence_cursor: activity.next_sequence_cursor,
                dispatch_generation: checkpoint.logical_dispatch_generation + 1,
                frame_digest: final_frame.frame_digest().0,
            },
            source_dispatch_generation: prior.map_or(0, |tick| tick.dispatch_generation),
            source_frame_digest: prior.map_or([0; 4], |tick| tick.frame_digest.0),
            completed_gpu_time_ns: 0,
            queue_depth: 0,
            logical_heap_used: backend.admission_receipt().logical_committed_bytes,
            logical_heap_capacity: backend.runtime_budget().logical_neural_heap_budget_bytes,
            brain_atp_remaining_q16: activity.brain_atp_q16,
            brain_atp_capacity_q16: alife_core::BRAIN_ATP_Q16_MAX,
        },
    )
    .unwrap();
    backend
        .install_recorded_pressure_replay(vec![pressure])
        .unwrap();
}

#[test]
fn pending_checkpoint_roundtrip_rebinds_private_receipt_and_resolves_exactly_once() {
    let organism = alife_core::OrganismId(71_001);
    let phenotype = support::controlled_learning_n512_phenotype(1.0);
    let frame = support::perception_frame_for_profile_at_tick(
        organism.raw(),
        4_000,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let mut source =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let physiology = support::test_physiology(71_001, &phenotype).unwrap();
    let source_handle = source.insert_brain(organism, phenotype.clone()).unwrap();
    support::tick_with_receptors(&mut source, source_handle, &phenotype, &physiology, &frame)
        .unwrap();
    let snapshot = source
        .snapshot_brain(source_handle, Tick::new(4_000))
        .unwrap();
    let checkpoint_digest = snapshot.canonical_digest();

    let mut restored =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let receipt = restored
        .restore_brain(
            organism,
            phenotype.clone(),
            GpuBrainRestoreRequest::try_new(snapshot).unwrap(),
        )
        .unwrap();
    assert_eq!(receipt.checkpoint_digest, checkpoint_digest);
    let restored_snapshot = restored
        .snapshot_brain(receipt.handle, Tick::new(4_000))
        .unwrap();
    assert_eq!(restored_snapshot.canonical_digest(), checkpoint_digest);

    let pending = receipt
        .pending_eligibility
        .expect("pending checkpoint must mint a new-process receipt");
    let identity = *pending.identity();
    restored
        .discard_pending_eligibility(receipt.handle, &identity)
        .unwrap();
    assert!(restored
        .discard_pending_eligibility(receipt.handle, &identity)
        .is_err());

    let next_frame = support::perception_frame_for_profile_at_tick(
        organism.raw(),
        4_001,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let (_, next_tick) = support::tick_with_receptors(
        &mut restored,
        receipt.handle,
        &phenotype,
        &physiology,
        &next_frame,
    )
    .unwrap();
    assert_eq!(next_tick.handle, receipt.handle);
}

#[test]
fn learned_checkpoint_roundtrip_preserves_logits_and_replay_guard() {
    let organism = alife_core::OrganismId(71_002);
    let phenotype = support::controlled_learning_n512_phenotype(1.0);
    let mut source =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let physiology = support::test_physiology(71_002, &phenotype).unwrap();
    let source_handle = source.insert_brain(organism, phenotype.clone()).unwrap();
    let learning_frame = support::perception_frame_for_profile_at_tick(
        organism.raw(),
        5_000,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let (learning_frame, learning_tick) = support::tick_with_receptors(
        &mut source,
        source_handle,
        &phenotype,
        &physiology,
        &learning_frame,
    )
    .unwrap();
    let (learning_patch, learning_receptors) = sealed_reward(
        source_handle,
        &phenotype,
        &physiology,
        &learning_frame,
        &learning_tick,
        1,
        BodyEventDelta {
            nutrition: 1.0,
            ..BodyEventDelta::zero()
        },
    );
    source
        .apply_sealed_outcome(source_handle, &learning_patch, &learning_receptors)
        .unwrap();

    let snapshot = source
        .snapshot_brain(source_handle, Tick::new(5_001))
        .unwrap();
    let digest = snapshot.canonical_digest();
    let mut restored =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let restore = restored
        .restore_brain(
            organism,
            phenotype.clone(),
            GpuBrainRestoreRequest::try_new(snapshot).unwrap(),
        )
        .unwrap();
    assert_eq!(restore.checkpoint_digest, digest);
    assert!(restore.pending_eligibility.is_none());

    let probe = support::perception_frame_for_profile_at_tick(
        organism.raw(),
        5_002,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let (final_probe, _) = support::empty_recall(&probe);
    install_full_recorded_pressure(
        &mut source,
        source_handle,
        &final_probe,
        Tick::new(5_001),
        Some(&learning_tick),
    );
    install_full_recorded_pressure(
        &mut restored,
        restore.handle,
        &final_probe,
        Tick::new(5_001),
        None,
    );
    let (source_probe, source_tick) =
        support::tick_with_receptors(&mut source, source_handle, &phenotype, &physiology, &probe)
            .unwrap();
    let (probe, restored_tick) = support::tick_with_receptors(
        &mut restored,
        restore.handle,
        &phenotype,
        &physiology,
        &probe,
    )
    .unwrap();
    assert_eq!(source_probe.frame_digest(), final_probe.frame_digest());
    assert_eq!(probe.frame_digest(), final_probe.frame_digest());
    assert_eq!(
        source_tick.throttle.level,
        alife_core::NeuralThrottleLevel::Full
    );
    assert_eq!(restored_tick.throttle.level, source_tick.throttle.level);
    assert_eq!(
        restored_tick.throttle.microsteps,
        source_tick.throttle.microsteps
    );
    assert_eq!(
        restored_tick.throttle.enabled_route_ids,
        source_tick.throttle.enabled_route_ids
    );
    for tick in [&source_tick, &restored_tick] {
        assert_eq!(tick.pressure.completed_gpu_time_ns, 0);
        assert_eq!(tick.pressure.queue_depth, 0);
    }
    assert_eq!(
        source_tick.selection.candidate_index,
        restored_tick.selection.candidate_index
    );
    assert_eq!(
        source_tick.selection.logit.to_bits(),
        restored_tick.selection.logit.to_bits()
    );

    let (duplicate, duplicate_receptors) = sealed_reward(
        restore.handle,
        &phenotype,
        &physiology,
        &probe,
        &restored_tick,
        1,
        BodyEventDelta {
            nutrition: 1.0,
            ..BodyEventDelta::zero()
        },
    );
    assert!(restored
        .apply_sealed_outcome(restore.handle, &duplicate, &duplicate_receptors)
        .is_err());
}

#[test]
fn completed_sleep_staging_restores_and_commits_one_physical_swap() {
    let organism = alife_core::OrganismId(71_003);
    let phenotype = support::controlled_learning_n512_phenotype(1.0);
    let mut source =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let physiology = support::test_physiology(71_003, &phenotype).unwrap();
    let handle = source.insert_brain(organism, phenotype.clone()).unwrap();
    let frame = support::perception_frame_for_profile_at_tick(
        organism.raw(),
        6_000,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let (frame, tick) =
        support::tick_with_receptors(&mut source, handle, &phenotype, &physiology, &frame).unwrap();
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
    source
        .apply_sealed_outcome(handle, &patch, &receptors)
        .unwrap();
    let replay = source.build_sleep_replay_batch(handle).unwrap();
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
    let completed_parts = source
        .snapshot_completed_sleep_staging(handle, &request, &staged.staged)
        .unwrap();
    let snapshot = source.snapshot_brain(handle, Tick::new(6_001)).unwrap();

    let mut restored =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let restore = restored
        .restore_brain(
            organism,
            phenotype.clone(),
            GpuBrainRestoreRequest::try_new(snapshot).unwrap(),
        )
        .unwrap();
    let restored_staging = restored
        .restore_completed_sleep_staging(
            restore.handle,
            &request,
            &replay,
            &staged.staged,
            completed_parts,
        )
        .unwrap();
    assert_eq!(restored_staging.staged, staged.staged);

    let first = restored
        .commit_sleep_consolidation(restore.handle, &request, &restored_staging.staged)
        .unwrap();
    let second = restored
        .commit_sleep_consolidation(restore.handle, &request, &restored_staging.staged)
        .unwrap();
    assert_eq!(first.commit_digest, second.commit_digest);
    assert_eq!(first.output_generation, request.expected_output_generation);
    assert_eq!(first.generation_swaps, 1);
}

#[test]
fn n2048_to_n4096_growth_is_same_adapter_equivalent_and_atomic() {
    let growth_profile = alife_gpu_backend::GpuRuntimeProfile {
        profile_id: 4_096,
        max_hot_brains: 2,
        growth_chunk_slots: 1,
        ..alife_gpu_backend::GpuRuntimeProfile::production_v1()
    };
    let organism = alife_core::OrganismId(71_4096);
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(0x2048_4096, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    let inputs = PhenotypeCompilerInputs::try_new(
        genome,
        &capacity,
        development,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let source_phenotype = PhenotypeCompiler::compile_validated(&inputs, &capacity).unwrap();
    let migration =
        PhenotypeGrowthMigration::compile_n2048_to_n4096(&source_phenotype, &inputs).unwrap();

    let mut source = GpuClosedLoopBackend::new_required(growth_profile).unwrap();
    let source_physiology = support::test_physiology(0x2048_4096, &source_phenotype).unwrap();
    let source_handle = source
        .insert_brain(organism, source_phenotype.clone())
        .unwrap();
    let warmup = support::perception_frame_for_profile_at_tick(
        organism.raw(),
        9_000,
        SensorProfile::GroundedObjectSlotsV1,
        true,
        2,
    );
    let warmup_tick = support::tick_with_receptors(
        &mut source,
        source_handle,
        &source_phenotype,
        &source_physiology,
        &warmup,
    )
    .unwrap()
    .1;
    source
        .discard_pending_eligibility(source_handle, warmup_tick.pending_eligibility.identity())
        .unwrap();
    let rollback = source
        .snapshot_brain(source_handle, Tick::new(9_000))
        .unwrap();
    let target_snapshot = rollback.clone().migrate_n2048_to_n4096(&migration).unwrap();

    let mut target = GpuClosedLoopBackend::new_required(growth_profile).unwrap();
    alife_gpu_backend::GpuClassBucketPlan::for_phenotype(&migration.phenotype).unwrap();
    target
        .runtime_budget()
        .validate_for(BrainCapacityClass::n4096_research().execution())
        .unwrap();
    let probe_handle = target
        .insert_research_brain(alife_core::OrganismId(71_4097), migration.phenotype.clone())
        .unwrap();
    target.remove_brain(probe_handle).unwrap();
    let target_restore = target
        .restore_research_brain(
            organism,
            migration.phenotype.clone(),
            GpuBrainRestoreRequest::try_new(target_snapshot.clone()).unwrap(),
        )
        .unwrap();
    let source_hw = source.hardware_receipt();
    let target_hw = target.hardware_receipt();
    assert_eq!(source_hw.backend_api, target_hw.backend_api);
    assert_eq!(source_hw.adapter_name, target_hw.adapter_name);
    assert_eq!(source_hw.vendor_id, target_hw.vendor_id);
    assert_eq!(source_hw.device_id, target_hw.device_id);
    assert_eq!(source_hw.driver_digest, target_hw.driver_digest);
    assert_eq!(source_hw.feature_digest, target_hw.feature_digest);
    assert_eq!(source_hw.limits_digest, target_hw.limits_digest);

    let probe = support::perception_frame_for_profile_at_tick(
        organism.raw(),
        9_001,
        SensorProfile::GroundedObjectSlotsV1,
        true,
        2,
    );
    // Compare neural migration under matched scheduling. A cognitive snapshot
    // does not migrate live governor history or prove equal runtime performance.
    let (final_probe, _) = support::empty_recall(&probe);
    install_full_recorded_pressure(
        &mut source,
        source_handle,
        &final_probe,
        Tick::new(9_000),
        Some(&warmup_tick),
    );
    install_full_recorded_pressure(
        &mut target,
        target_restore.handle,
        &final_probe,
        Tick::new(9_000),
        None,
    );
    let (source_probe, source_tick) = support::tick_with_receptors(
        &mut source,
        source_handle,
        &source_phenotype,
        &source_physiology,
        &probe,
    )
    .unwrap();
    // Brain growth does not rewrite the organism's inherited body physiology.
    let (target_probe, target_tick) = support::tick_with_receptors(
        &mut target,
        target_restore.handle,
        &migration.phenotype,
        &source_physiology,
        &probe,
    )
    .unwrap();
    assert_eq!(source_probe.frame_digest(), final_probe.frame_digest());
    assert_eq!(target_probe.frame_digest(), final_probe.frame_digest());
    assert_eq!(
        source_tick.throttle.level,
        alife_core::NeuralThrottleLevel::Full
    );
    assert_eq!(target_tick.throttle.level, source_tick.throttle.level);
    assert_eq!(
        target_tick.throttle.microsteps,
        source_tick.throttle.microsteps
    );
    assert_eq!(
        target_tick.throttle.enabled_route_ids,
        source_tick.throttle.enabled_route_ids
    );
    for tick in [&source_tick, &target_tick] {
        assert_eq!(tick.pressure.completed_gpu_time_ns, 0);
        assert_eq!(tick.pressure.queue_depth, 0);
    }
    let source_logits = source
        .candidate_logits_for_evidence(
            source_handle,
            &source_probe,
            source_tick.pending_eligibility.identity(),
        )
        .unwrap();
    let target_logits = target
        .candidate_logits_for_evidence(
            target_restore.handle,
            &target_probe,
            target_tick.pending_eligibility.identity(),
        )
        .unwrap();
    assert_eq!(
        source_tick.selection.candidate_index,
        target_tick.selection.candidate_index
    );
    assert_eq!(source_logits.frame_digest, target_logits.frame_digest);
    assert_eq!(source_logits.logits.len(), target_logits.logits.len());
    let observed_max_delta = source_logits
        .logits
        .iter()
        .zip(&target_logits.logits)
        .map(|(source, target)| (source - target).abs())
        .fold(0.0_f32, f32::max);
    eprintln!(
        "GROWTH logits source={:?} target={:?} max_delta={observed_max_delta:?}",
        source_logits.logits, target_logits.logits
    );
    assert!(observed_max_delta <= 1.0e-6);
    let equivalence = verify_research_growth_equivalence(
        source.hardware_receipt(),
        target.hardware_receipt(),
        &migration,
        &rollback,
        &target_snapshot,
        &source_tick,
        &target_tick,
        &source_logits,
        &target_logits,
    )
    .unwrap();
    assert!(equivalence.max_logit_delta() <= 1.0e-6);
    source
        .discard_pending_eligibility(source_handle, source_tick.pending_eligibility.identity())
        .unwrap();
    target
        .discard_pending_eligibility(
            target_restore.handle,
            target_tick.pending_eligibility.identity(),
        )
        .unwrap();

    let mut handoff = GpuClosedLoopBackend::new_required(growth_profile).unwrap();
    let handoff_source = handoff
        .restore_brain(
            organism,
            source_phenotype.clone(),
            GpuBrainRestoreRequest::try_new(rollback.clone()).unwrap(),
        )
        .unwrap();
    let handoff_receipt = match handoff
        .replace_brain_with_research_growth(
            handoff_source.handle,
            &migration,
            rollback.clone(),
            target_snapshot.clone(),
            &equivalence,
        )
        .unwrap()
    {
        GpuResearchGrowthHandoffOutcome::Committed(receipt) => receipt,
        GpuResearchGrowthHandoffOutcome::RolledBack(_) => panic!("valid growth rolled back"),
    };
    assert_eq!(
        handoff_receipt.target_handle.class_id(),
        BrainCapacityClass::N4096_RESEARCH_ID
    );
    assert!(handoff
        .snapshot_brain(handoff_source.handle, Tick::new(9_000))
        .is_err());
    assert_eq!(
        handoff
            .snapshot_brain(handoff_receipt.target_handle, Tick::new(9_000))
            .unwrap()
            .canonical_digest(),
        target_snapshot.canonical_digest(),
    );

    let constrained_profile = alife_gpu_backend::GpuRuntimeProfile {
        profile_id: 4_097,
        physical_allocation_ceiling_bytes: handoff
            .admission_receipt()
            .physical_allocated_bytes
            .saturating_sub(1),
        ..growth_profile
    };
    let mut rollback_backend = GpuClosedLoopBackend::new_required(constrained_profile).unwrap();
    let rollback_source = rollback_backend
        .restore_brain(
            organism,
            source_phenotype,
            GpuBrainRestoreRequest::try_new(rollback.clone()).unwrap(),
        )
        .unwrap();
    let rollback_receipt = match rollback_backend
        .replace_brain_with_research_growth(
            rollback_source.handle,
            &migration,
            rollback.clone(),
            target_snapshot,
            &equivalence,
        )
        .unwrap()
    {
        GpuResearchGrowthHandoffOutcome::RolledBack(receipt) => receipt,
        GpuResearchGrowthHandoffOutcome::Committed(_) => {
            panic!("constrained growth unexpectedly committed")
        }
    };
    assert_eq!(
        rollback_backend
            .snapshot_brain(rollback_receipt.source_handle, Tick::new(9_000))
            .unwrap()
            .canonical_digest(),
        rollback.canonical_digest(),
    );
}

#[test]
fn exact_population_capture_is_one_nonblocking_identity_bound_gpu_transaction() {
    let phenotype = support::controlled_learning_n512_phenotype(1.0);
    let organisms = [alife_core::OrganismId(5_105), alife_core::OrganismId(5_106)];
    let mut backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    println!(
        "PHASE31_GPU_ADAPTER={} BACKEND={:?}",
        backend.hardware_receipt().adapter_name,
        backend.hardware_receipt().backend_api
    );
    let handles =
        organisms.map(|organism| backend.insert_brain(organism, phenotype.clone()).unwrap());
    let submissions_before_rejection = backend
        .exact_population_capture_metrics()
        .gpu_copy_submissions;
    let mut foreign_backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let foreign = foreign_backend
        .insert_brain(alife_core::OrganismId(71_103), phenotype.clone())
        .unwrap();
    assert!(backend
        .submit_exact_population_capture(Tick::new(100), 1, &[foreign])
        .is_err());
    let stale = backend
        .insert_brain(alife_core::OrganismId(71_104), phenotype)
        .unwrap();
    backend.remove_brain(stale).unwrap();
    assert!(backend
        .submit_exact_population_capture(Tick::new(100), 1, &[stale])
        .is_err());
    assert!(backend
        .submit_exact_population_capture(Tick::new(100), 1, &handles[..1])
        .is_err());
    assert_eq!(
        backend
            .exact_population_capture_metrics()
            .gpu_copy_submissions,
        submissions_before_rejection,
        "foreign and stale identity must fail before GPU submission"
    );

    let mut tick_t = backend
        .submit_exact_population_capture(Tick::new(100), 1, &handles)
        .unwrap();
    assert_eq!(tick_t.capture_transaction_generation(), 1);
    assert_ne!(tick_t.population_set_digest(), [0; 4]);
    assert_eq!(tick_t.gpu_copy_submissions(), 1);
    assert_eq!(tick_t.map_operations(), 1);
    assert!(tick_t.staging_bytes() > 0);

    // The later topology upload is queued after the capture copy. It may
    // advance the live resident before mapping completes, but cannot change
    // the already ordered tick-T bytes.
    backend
        .set_v11_dendritic_branches(handles[0], alife_core::DendriticBranchSet::default())
        .unwrap();

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let captured_t = loop {
        match backend.poll_exact_population_capture(&mut tick_t).unwrap() {
            GpuExactPopulationCapturePollV1::Pending if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            GpuExactPopulationCapturePollV1::Pending => {
                panic!("tick-T exact population capture exceeded the bounded poll deadline")
            }
            GpuExactPopulationCapturePollV1::Ready(capture) => break capture,
            GpuExactPopulationCapturePollV1::Failed(failure) => {
                panic!("tick-T exact population capture failed: {failure:?}")
            }
        }
    };
    assert_eq!(captured_t.checkpoint_tick(), Tick::new(100));
    assert_eq!(captured_t.capture_transaction_generation(), 1);
    assert_eq!(
        captured_t.population_set_digest(),
        tick_t.population_set_digest()
    );
    assert_eq!(captured_t.rows().len(), handles.len());
    for (row, handle) in captured_t.rows().iter().zip(handles) {
        assert_eq!(row.identity().organism_id, handle.organism_id());
        assert_eq!(row.identity().class_id, handle.class_id());
        assert_eq!(row.identity().slot, handle.slot());
        assert_eq!(row.identity().slot_generation, handle.generation());
        assert_eq!(row.identity().phenotype_hash, handle.phenotype_hash());
        assert!(row.identity().graph_epoch > 0);
        assert!(row.identity().logical_dispatch_generation > 0);
        assert!(row.identity().active_activation_side <= 1);
        assert!(row.identity().active_weight_generation > 0);
        assert!(row.identity().active_weight_bank <= 1);
        assert!(row.identity().active_eligibility_generation > 0);
        assert!(row.identity().active_eligibility_bank <= 1);
        assert!(row.identity().replay_journal_generation > 0);
        assert!(row.identity().transaction_generation > 0);
        assert_eq!(row.identity().activity_sequence_cursor, 1);
        assert!(row.identity().last_throttle.is_none());
        assert!(row.identity().last_work.is_none());
        assert!(!row.immutable_plan_bytes().is_empty());
        assert!(!row.immutable_weight_bytes().is_empty());
        assert!(!row.mutable_state_bytes().is_empty());
    }

    let mut after_advance = backend
        .submit_exact_population_capture(Tick::new(101), 2, &handles)
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let captured_after = loop {
        match backend
            .poll_exact_population_capture(&mut after_advance)
            .unwrap()
        {
            GpuExactPopulationCapturePollV1::Pending if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            GpuExactPopulationCapturePollV1::Pending => {
                panic!("post-advance exact population capture exceeded the bounded poll deadline")
            }
            GpuExactPopulationCapturePollV1::Ready(capture) => break capture,
            GpuExactPopulationCapturePollV1::Failed(failure) => {
                panic!("post-advance exact population capture failed: {failure:?}")
            }
        }
    };
    assert_ne!(
        captured_t.rows()[0].immutable_plan_bytes(),
        captured_after.rows()[0].immutable_plan_bytes(),
        "the first capture must retain immutable tick-T bytes after live advance"
    );
    assert_ne!(
        captured_t.rows()[0].identity().v11,
        captured_after.rows()[0].identity().v11,
        "the capture identity must bind the graph state paired with the copied plan"
    );
    assert_eq!(captured_t.rows()[1], captured_after.rows()[1]);
    assert_eq!(tick_t.retained_staging_bytes(), 0);
    assert_eq!(after_advance.retained_staging_bytes(), 0);

    let released_before_failure = backend
        .exact_population_capture_metrics()
        .released_staging_bytes;
    let mut forced_failure = backend
        .submit_exact_population_capture(Tick::new(101), 3, &handles)
        .unwrap();
    let forced_bytes = forced_failure.staging_bytes();
    forced_failure.force_decode_identity_failure_for_test();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let failure = loop {
        match backend
            .poll_exact_population_capture(&mut forced_failure)
            .unwrap()
        {
            GpuExactPopulationCapturePollV1::Pending if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            GpuExactPopulationCapturePollV1::Pending => {
                panic!("forced-failure capture exceeded the bounded poll deadline")
            }
            GpuExactPopulationCapturePollV1::Ready(_) => {
                panic!("forced identity mismatch unexpectedly completed")
            }
            GpuExactPopulationCapturePollV1::Failed(failure) => break failure,
        }
    };
    assert_eq!(
        failure.stage,
        alife_gpu_backend::GpuExactPopulationCaptureFailureStageV1::DecodeIdentity
    );
    assert_eq!(forced_failure.retained_staging_bytes(), 0);
    assert_eq!(
        backend
            .exact_population_capture_metrics()
            .released_staging_bytes,
        released_before_failure + forced_bytes
    );
    assert!(matches!(
        backend
            .poll_exact_population_capture(&mut forced_failure)
            .unwrap(),
        GpuExactPopulationCapturePollV1::Failed(repeated) if repeated == failure
    ));
    assert_eq!(
        backend
            .exact_population_capture_metrics()
            .released_staging_bytes,
        released_before_failure + forced_bytes,
        "terminal failure polling must not release or account staging twice"
    );
}

#[test]
fn joint_pending_checkpoint_preserves_exact_confidence_and_legacy_pending_banks() {
    let organism = alife_core::OrganismId(71_019);
    let capacity = BrainCapacityClass::n512();
    let genome = support::two_head_n512_genome();
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
    let phenotype = PhenotypeCompiler::compile(
        &genome,
        &capacity,
        &development,
        SensorProfile::PrivilegedAffordanceV1,
    )
    .unwrap();
    let template = support::perception_frame_for_profile_at_tick(
        organism.raw(),
        4100,
        SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let mut candidates = template.candidates().to_vec();
    for (index, candidate) in candidates.iter_mut().enumerate() {
        candidate.kind = if index == 0 {
            alife_core::ActionKind::Move
        } else {
            alife_core::ActionKind::Interact
        };
        candidate.family = if index == 0 {
            alife_core::CandidateActionFamily::Approach
        } else {
            alife_core::CandidateActionFamily::Ingest
        };
        candidate.action_id = alife_core::ActionId(if index == 0 { 102 } else { 210 });
        candidate.sensor_confidence = Confidence::new(0.731234).unwrap();
        candidate.target =
            alife_core::ActionTarget::new(None, Some(Vec3f::new(index as f32 + 2.0, 0.0, 1.0)));
    }
    let frame = alife_core::PerceptionFrame::new(
        organism,
        template.tick(),
        template.sensor_profile(),
        template.sensory().clone(),
        template.body(),
        template.homeostasis().clone(),
        candidates,
        template.profile_provenance(),
        template.grounded_object_slots().to_vec(),
    )
    .unwrap();
    let mut source =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let physiology = support::test_physiology(71_019, &phenotype).unwrap();
    let handle = source.insert_brain(organism, phenotype.clone()).unwrap();
    let (frame, tick) =
        support::tick_with_receptors(&mut source, handle, &phenotype, &physiology, &frame).unwrap();
    let proof = tick
        .pending_eligibility
        .identity()
        .joint_selection()
        .unwrap();
    assert_eq!(
        proof.candidate_slots().iter().filter(|v| **v != 0).count(),
        2
    );
    assert_eq!(
        tick.selection.confidence.raw().to_bits(),
        0.731234_f32.to_bits()
    );
    let command = frame.candidates()[tick.selection.candidate_index as usize]
        .to_command(organism, tick.selection.confidence)
        .unwrap();
    let bundle = alife_core::factorized_motor_bundle_for_candidates(
        organism,
        ExperienceSequenceId(1),
        frame.tick(),
        &frame,
        tick.factorized_motor_candidates,
        &phenotype
            .candidate_decoder()
            .factorized_motor_channels(&phenotype)
            .unwrap(),
        &command,
        tick.selection.candidate_index,
        tick.speech_payload.as_ref(),
        false,
    )
    .unwrap();
    proof.validate_bundle(&bundle).unwrap();
    let snapshot = source.snapshot_brain(handle, frame.tick()).unwrap();
    let mut restored =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let restored_receipt = restored
        .restore_brain(
            organism,
            phenotype.clone(),
            GpuBrainRestoreRequest::try_new(snapshot.clone()).unwrap(),
        )
        .unwrap();
    let restored_pending = restored_receipt.pending_eligibility.unwrap();
    assert_eq!(restored_pending.identity().joint_selection(), Some(proof));
    assert_eq!(
        restored
            .snapshot_brain(restored_receipt.handle, frame.tick())
            .unwrap()
            .canonical_digest(),
        snapshot.canonical_digest()
    );
    restored
        .discard_pending_eligibility(restored_receipt.handle, restored_pending.identity())
        .unwrap();
    restored.remove_brain(restored_receipt.handle).unwrap();
    source
        .discard_pending_eligibility(handle, tick.pending_eligibility.identity())
        .unwrap();

    // A single Inspect candidate has the identical eligibility interpretation
    // under the historical global-only path. Remove only the optional metadata,
    // then prove a live legacy pending restore keeps every acquired bank bit.
    let legacy_frame = support::perception_frame_for_profile_at_tick(
        organism.raw(),
        4101,
        SensorProfile::PrivilegedAffordanceV1,
        true,
        1,
    );
    let (legacy_frame, _) =
        support::tick_with_receptors(&mut source, handle, &phenotype, &physiology, &legacy_frame)
            .unwrap();
    let mut parts = source
        .snapshot_brain(handle, legacy_frame.tick())
        .unwrap()
        .into_parts();
    let pending = parts.pending_eligibility.unwrap();
    parts.pending_eligibility = Some(pending.with_joint_selection(None).unwrap());
    let legacy =
        alife_gpu_backend::GpuBrainCheckpointSnapshot::try_from_parts(parts.clone()).unwrap();
    let legacy_receipt = restored
        .restore_brain(
            organism,
            phenotype.clone(),
            GpuBrainRestoreRequest::try_new(legacy.clone()).unwrap(),
        )
        .unwrap();
    let pending = legacy_receipt.pending_eligibility.unwrap();
    assert_eq!(pending.identity().joint_selection(), None);
    let after = restored
        .snapshot_brain(legacy_receipt.handle, legacy_frame.tick())
        .unwrap();
    assert_eq!(after.canonical_digest(), legacy.canonical_digest());
    assert_eq!(after.into_parts(), parts);
    restored
        .discard_pending_eligibility(legacy_receipt.handle, pending.identity())
        .unwrap();
    let new_frame = support::perception_frame_for_profile_at_tick(
        organism.raw(),
        4102,
        SensorProfile::PrivilegedAffordanceV1,
        true,
        1,
    );
    let (_, new_tick) = support::tick_with_receptors(
        &mut restored,
        legacy_receipt.handle,
        &phenotype,
        &physiology,
        &new_frame,
    )
    .unwrap();
    assert!(new_tick
        .pending_eligibility
        .identity()
        .joint_selection()
        .is_some());
    restored
        .discard_pending_eligibility(
            legacy_receipt.handle,
            new_tick.pending_eligibility.identity(),
        )
        .unwrap();
}
