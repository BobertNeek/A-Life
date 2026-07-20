//! Real-hardware exact checkpoint/restore acceptance for the GPU-authoritative brain.
#![cfg(feature = "gpu-tests")]

mod support;

use alife_core::{
    BrainGenome, Confidence, ConsolidationIntent, DecisionSnapshot, DevelopmentState,
    EndocrineDelta, ExperiencePatch, ExperiencePatchBuilder, ExperienceSequenceId,
    HomeostaticDelta, NeuralActionSelection, NormalizedScalar, PhysicalActionOutcome,
    PhysicalContactKind, PostActionOutcome, PreActionSnapshot, SignedValence, Tick, Vec3f,
};
use alife_gpu_backend::{GpuBrainRestoreRequest, GpuClosedLoopBackend};

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
    let mut source = GpuClosedLoopBackend::new_required().unwrap();
    let source_handle = source.insert_brain(organism, phenotype.clone()).unwrap();
    source
        .tick_batch(&[(source_handle, frame.clone())])
        .unwrap();
    let snapshot = source
        .snapshot_brain(source_handle, Tick::new(4_000))
        .unwrap();
    let checkpoint_digest = snapshot.canonical_digest();

    let mut restored = GpuClosedLoopBackend::new_required().unwrap();
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
    let mut source = GpuClosedLoopBackend::new_required().unwrap();
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
    source
        .apply_sealed_outcome(
            source_handle,
            &sealed_reward(source_handle, &learning_frame, &learning_tick, 1, 0.8),
        )
        .unwrap();

    let snapshot = source
        .snapshot_brain(source_handle, Tick::new(5_001))
        .unwrap();
    let digest = snapshot.canonical_digest();
    let mut restored = GpuClosedLoopBackend::new_required().unwrap();
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
    assert!(restored
        .apply_sealed_outcome(restore.handle, &duplicate)
        .is_err());
}

#[test]
fn completed_sleep_staging_restores_and_commits_one_physical_swap() {
    let organism = alife_core::OrganismId(71_003);
    let phenotype = support::controlled_learning_n512_phenotype(1.0);
    let mut source = GpuClosedLoopBackend::new_required().unwrap();
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
    source
        .apply_sealed_outcome(handle, &sealed_reward(handle, &frame, &tick, 1, 0.8))
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

    let mut restored = GpuClosedLoopBackend::new_required().unwrap();
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
