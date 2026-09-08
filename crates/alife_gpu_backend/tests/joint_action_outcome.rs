//! Transaction validation with real GPU decisions and explicit outcome fixtures.
#![cfg(feature = "gpu-tests")]

mod support;

use alife_core::*;
use alife_gpu_backend::{
    GpuClosedLoopBackend, GpuLearningEvidenceMismatchField, GpuRuntimeProfile,
};

#[test]
fn joint_non_global_target_tampering_rejects_before_learning_and_preserves_pending_state() {
    let organism = OrganismId(71_020);
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
            ActionKind::Move
        } else {
            ActionKind::Interact
        };
        candidate.family = if index == 0 {
            CandidateActionFamily::Approach
        } else {
            CandidateActionFamily::Ingest
        };
        candidate.action_id = ActionId(if index == 0 { 102 } else { 210 });
        candidate.sensor_confidence = Confidence::new(0.731234).unwrap();
        candidate.target = ActionTarget::new(None, Some(Vec3f::new(index as f32 + 2.0, 0.0, 1.0)));
    }
    let frame = PerceptionFrame::new(
        organism,
        template.tick(),
        template.sensor_profile(),
        template.sensory().clone(),
        template.body(),
        *template.homeostasis(),
        candidates,
        template.profile_provenance(),
        template.grounded_object_slots().to_vec(),
    )
    .unwrap();
    let mut backend =
        GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap();
    let handle = backend.insert_brain(organism, phenotype.clone()).unwrap();
    let (frame, gpu_tick) = support::tick_with_receptors(&mut backend, handle, &frame);
    let sequence = ExperienceSequenceId(1);
    let command = frame.candidates()[usize::from(gpu_tick.selection.candidate_index)]
        .to_command(organism, gpu_tick.selection.confidence)
        .unwrap();
    let bundle = factorized_motor_bundle_for_candidates(
        organism,
        sequence,
        frame.tick(),
        &frame,
        gpu_tick.factorized_motor_candidates,
        &phenotype
            .candidate_decoder()
            .factorized_motor_channels(&phenotype)
            .unwrap(),
        &command,
        gpu_tick.selection.candidate_index,
        gpu_tick.speech_payload.as_ref(),
        false,
    )
    .unwrap();
    assert_eq!(bundle.channels.len(), 2);
    gpu_tick
        .pending_eligibility
        .identity()
        .joint_selection()
        .unwrap()
        .validate_bundle(&bundle)
        .unwrap();

    let pre_action = PreActionSnapshot::from_neural_frame(
        sequence,
        phenotype.brain_class_id(),
        phenotype.phenotype_hash(),
        genome.id,
        genome.schema_version,
        development,
        frame.clone(),
    )
    .unwrap();
    let decision = DecisionSnapshot::from_neural_selection(
        sequence,
        phenotype.phenotype_hash(),
        gpu_tick.dispatch_generation,
        gpu_tick.active_activation_side,
        &frame,
        gpu_tick.selection,
        command,
    )
    .unwrap();
    // Only the transaction guard is under test. These are declared physiology
    // fixtures, not evidence of useful behavior or reward supplied to training.
    let body_phenotype = CreatureGenome::early_mammal_founder(
        42,
        FoundationGeneticIdentity::new(22, 1, 1, capacity.id()).unwrap(),
    )
    .unwrap()
    .express()
    .unwrap();
    let before = BiochemistryState::new(&body_phenotype, frame.tick()).unwrap();
    let outcome_tick = Tick::new(frame.tick().raw() + 1);
    let after = before
        .advance(outcome_tick, BodyEventDelta::zero(), &body_phenotype)
        .unwrap();
    let physiology = MeasuredPhysiologyTransition::new(before, after).unwrap();
    let physical = PhysicalActionOutcome {
        contact: PhysicalContactKind::None,
        target_entity: None,
        displacement: Vec3f::ZERO,
        collision_normal: None,
        energy_cost: NormalizedScalar::ZERO,
    };
    let work = CognitiveWorkReceipt::zero();
    let outcome = PostActionOutcome::new(
        organism,
        sequence,
        outcome_tick,
        true,
        physical,
        physiology.homeostatic_delta,
        SignedValence::ZERO,
        NormalizedScalar::ZERO,
        NormalizedScalar::ZERO,
        physiology.energy_delta,
        NormalizedScalar::ZERO,
    )
    .unwrap()
    .with_measured_physiology(physiology)
    .unwrap()
    .with_v11_joint(
        JointPhysicalOutcome::new(physical, Vec::new()).unwrap(),
        work,
    )
    .unwrap();
    let seal = |selected_bundle: MotorCommandBundle| {
        let target = PredictionTargetReceipt::for_successor(
            organism,
            sequence,
            command.action_id,
            frame.tick(),
            frame.frame_digest().0,
            SemanticStateVector::new(vec![0.5, 0.25]).unwrap(),
            JointMotorCondition::from_bundle(&selected_bundle).unwrap(),
            SemanticStateVector::new(vec![0.4, 0.3]).unwrap(),
        )
        .unwrap();
        ExperiencePatch::new_v11_with_decision(
            pre_action.clone(),
            decision.clone(),
            selected_bundle,
            outcome.clone(),
            target,
            work,
            CognitiveContextFrame::empty(organism, sequence, frame.tick()).unwrap(),
        )
        .unwrap()
    };
    let original = seal(bundle.clone());
    assert_eq!(
        backend
            .sealed_outcome_credit_mismatch_receipt(handle, &original)
            .unwrap(),
        None
    );
    let before_rejection = backend.snapshot_brain(handle, frame.tick()).unwrap();
    let mut changed = bundle;
    let non_global = changed
        .channels
        .iter_mut()
        .find(|channel| channel.primitive != command.action_id)
        .unwrap();
    non_global.target.as_mut().unwrap().entity = Some(WorldEntityId(999));
    let tampered = seal(changed);
    OutcomeCreditPacket::from_sealed_patch(&tampered).unwrap();
    let mismatch = backend
        .sealed_outcome_credit_mismatch_receipt(handle, &tampered)
        .unwrap()
        .unwrap();
    assert_eq!(
        mismatch.field,
        GpuLearningEvidenceMismatchField::JointActionSelection
    );
    let receptors = support::test_receptor_frame(&original);
    assert!(backend
        .apply_sealed_outcome(handle, &tampered, &receptors)
        .is_err());
    assert_eq!(
        backend
            .snapshot_brain(handle, frame.tick())
            .unwrap()
            .canonical_digest(),
        before_rejection.canonical_digest()
    );
    assert_eq!(
        backend.pending_eligibility(handle).unwrap(),
        Some(gpu_tick.pending_eligibility)
    );
    backend
        .apply_sealed_outcome(handle, &original, &receptors)
        .unwrap();
    assert_eq!(backend.pending_eligibility(handle).unwrap(), None);
}
