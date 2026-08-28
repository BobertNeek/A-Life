//! Real-hardware exact checkpoint/restore acceptance for the GPU-authoritative brain.
#![cfg(feature = "gpu-tests")]

mod support;

use alife_core::{
    BrainCapacityClass, BrainGenome, Confidence, ConsolidationIntent, DecisionSnapshot,
    DevelopmentState, EndocrineDelta, ExperiencePatch, ExperiencePatchBuilder,
    ExperienceSequenceId, HomeostaticDelta, NeuralActionSelection, NormalizedScalar,
    PhenotypeCompiler, PhenotypeCompilerInputs, PhenotypeGrowthMigration, PhysicalActionOutcome,
    PhysicalContactKind, PostActionOutcome, PreActionSnapshot, SensorProfile, SignedValence, Tick,
    Vec3f,
};
use alife_gpu_backend::{
    verify_research_growth_equivalence, GpuBrainRestoreRequest, GpuClosedLoopBackend,
    GpuExactPopulationCapturePollV1, GpuResearchGrowthHandoffOutcome,
};

fn sealed_reward(
    handle: alife_gpu_backend::GpuBrainHandle,
    frame: &alife_core::PerceptionFrame,
    tick: &alife_gpu_backend::GpuClosedLoopTick,
    sequence_raw: u64,
    reward: f32,
) -> ExperiencePatch {
    let sequence_id = ExperienceSequenceId(sequence_raw);
    let genome = BrainGenome::scaffold(42, handle.class_id());
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
    let source_handle = source.insert_brain(organism, phenotype.clone()).unwrap();
    source
        .tick_batch(&[(source_handle, frame.clone())])
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
            phenotype,
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
    assert_eq!(
        restored
            .tick_batch(&[(receipt.handle, next_frame)])
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn learned_checkpoint_roundtrip_preserves_logits_and_replay_guard() {
    let organism = alife_core::OrganismId(71_002);
    let phenotype = support::controlled_learning_n512_phenotype(1.0);
    let mut source =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let source_handle = source.insert_brain(organism, phenotype.clone()).unwrap();
    let learning_frame = support::perception_frame_for_profile_at_tick(
        organism.raw(),
        5_000,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let learning_tick = source
        .tick_batch(&[(source_handle, learning_frame.clone())])
        .unwrap()
        .remove(0);
    let learning_patch = sealed_reward(source_handle, &learning_frame, &learning_tick, 1, 0.8);
    let learning_receptors = support::test_receptor_frame(&learning_patch);
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
            phenotype,
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
    let source_tick = source
        .tick_batch(&[(source_handle, probe.clone())])
        .unwrap()
        .remove(0);
    let restored_tick = restored
        .tick_batch(&[(restore.handle, probe.clone())])
        .unwrap()
        .remove(0);
    assert_eq!(
        source_tick.selection.candidate_index,
        restored_tick.selection.candidate_index
    );
    assert_eq!(
        source_tick.selection.logit.to_bits(),
        restored_tick.selection.logit.to_bits()
    );

    let duplicate = sealed_reward(restore.handle, &probe, &restored_tick, 1, 0.8);
    let duplicate_receptors = support::test_receptor_frame(&duplicate);
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
    let handle = source.insert_brain(organism, phenotype.clone()).unwrap();
    let frame = support::perception_frame_for_profile_at_tick(
        organism.raw(),
        6_000,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let tick = source
        .tick_batch(&[(handle, frame.clone())])
        .unwrap()
        .remove(0);
    let patch = sealed_reward(handle, &frame, &tick, 1, 0.8);
    let receptors = support::test_receptor_frame(&patch);
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
            phenotype,
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
    let warmup_tick = source
        .tick_batch(&[(source_handle, warmup)])
        .unwrap()
        .remove(0);
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
    let source_tick = source
        .tick_batch(&[(source_handle, probe.clone())])
        .unwrap()
        .remove(0);
    let target_tick = target
        .tick_batch(&[(target_restore.handle, probe.clone())])
        .unwrap()
        .remove(0);
    let source_logits = source
        .candidate_logits_for_evidence(
            source_handle,
            &probe,
            source_tick.pending_eligibility.identity(),
        )
        .unwrap();
    let target_logits = target
        .candidate_logits_for_evidence(
            target_restore.handle,
            &probe,
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
