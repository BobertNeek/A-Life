//! Real-hardware tests for sealed-outcome GPU fast plasticity.
//!
//! These tests build ordinary sealed neural experience patches and never run
//! CPU neural or plasticity math.
#![cfg(feature = "gpu-tests")]

mod support;

use alife_core::sleep::SleepReplayEvidence;
use alife_core::{
    AlphaMask, BiochemistryState, BodyEventDelta, BrainCapacityClass, BrainGenome, Confidence,
    CreatureGenome, DecisionSnapshot, DevelopmentState, EndocrineDelta, ExperiencePatch,
    ExperiencePatchBuilder, ExperienceSequenceId, FoundationGeneticIdentity, HomeostaticDelta,
    JointMotorCondition, JointPhysicalOutcome, MeasuredPhysiologyTransition, MotorChannel,
    MotorChannelFactor, NeuralActionSelection, NormalizedScalar, OutcomeCreditPacket,
    PhenotypeCompiler, PhysicalActionOutcome, PhysicalContactKind, PlasticityGenomeParameters,
    PostActionOutcome, PreActionSnapshot, PredictionTargetReceipt, SemanticStateVector,
    SensorProfile, SignedValence, Tick, Vec3f,
};
use alife_gpu_backend::{GpuClosedLoopBackend, GpuOutcomeCreditRecord};

const GPU_LEARNING_TOLERANCE: f32 = 1.0e-5;

fn r12_pending_fixture() -> (
    GpuClosedLoopBackend,
    alife_gpu_backend::GpuBrainHandle,
    ExperiencePatch,
    alife_core::NeuralReceptorFrame,
) {
    let phenotype = support::controlled_learning_n512_phenotype(1.0);
    let organism = alife_core::OrganismId(12_004);
    let mut backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    println!("R12 hardware {:?}", backend.hardware_receipt());
    let handle = backend.insert_brain(organism, phenotype.clone()).unwrap();
    let source = support::perception_frame_for_profile_at_tick(
        organism.raw(),
        900,
        SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let physiology = CreatureGenome::early_mammal_founder(
        42,
        FoundationGeneticIdentity::new(42, 1, 1, handle.class_id()).unwrap(),
    )
    .unwrap()
    .express()
    .unwrap();
    let chemistry = BiochemistryState::new(&physiology, source.tick()).unwrap();
    let receptors = chemistry.neural_receptor_frame(&physiology).unwrap();
    assert!(!receptors.activations.is_empty());
    let expression = alife_core::NeuralReceptorPhenotype::compile(&phenotype).unwrap();
    let effects = alife_core::NeuralReceptorEffects::from_frame(&receptors, &expression).unwrap();
    let (frame, recall) = support::empty_recall(&source);
    let upload = backend
        .prepare_memory_context_upload(handle, &frame, &recall)
        .unwrap()
        .bind_neural_receptor_effects(effects)
        .unwrap();
    let batch = alife_gpu_backend::GpuClosedLoopMemoryBatchInput::try_new(vec![
        alife_gpu_backend::GpuClosedLoopMemoryTickInput::try_new(handle, &frame, &upload).unwrap(),
    ])
    .unwrap();
    let tick = backend.tick_memory_batch(&batch).unwrap().remove(0);
    let measured = sealed_measured_outcome(handle, &frame, &tick);
    let command = frame.candidates()[usize::from(tick.selection.candidate_index)]
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
    tick.pending_eligibility
        .identity()
        .joint_selection()
        .unwrap()
        .validate_bundle(&bundle)
        .unwrap();
    // Declared physical/prediction fixtures exercise transaction mechanics only.
    // Chemistry and the selected joint command come from their real owners.
    let work = alife_core::CognitiveWorkReceipt::zero();
    let outcome = measured
        .outcome()
        .clone()
        .with_v11_joint(
            alife_core::JointPhysicalOutcome::new(measured.outcome().physical, Vec::new()).unwrap(),
            work,
        )
        .unwrap();
    let target = alife_core::PredictionTargetReceipt::for_successor(
        organism,
        ExperienceSequenceId(1),
        command.action_id,
        frame.tick(),
        frame.frame_digest().0,
        SemanticStateVector::new(vec![0.5, 0.25]).unwrap(),
        JointMotorCondition::from_bundle(&bundle).unwrap(),
        SemanticStateVector::new(vec![0.4, 0.3]).unwrap(),
    )
    .unwrap();
    let patch = ExperiencePatch::new_v11_with_decision(
        measured.pre_action().clone(),
        measured.decision().clone(),
        bundle,
        outcome,
        target,
        work,
        alife_core::CognitiveContextFrame::empty(organism, ExperienceSequenceId(1), frame.tick())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        backend
            .sealed_outcome_credit_mismatch_receipt(handle, &patch)
            .unwrap(),
        None
    );
    (backend, handle, patch, receptors)
}

fn r12_assert_no_promotion(
    before: &alife_gpu_backend::GpuPlasticityProbeForTest,
    after: &alife_gpu_backend::GpuPlasticityProbeForTest,
) {
    assert_eq!(
        after.learning, before.learning,
        "device banks, generations, cursor and pending flag"
    );
    assert_eq!(
        after.pending, before.pending,
        "exact pending transaction remains owned"
    );
    assert_eq!(after.host_generations, before.host_generations);
    assert_eq!(after.active_lifetime, before.active_lifetime);
    assert_eq!(after.active_fast, before.active_fast);
    assert_eq!(after.replay_events, before.replay_events);
}

fn r12_reject(
    backend: &mut GpuClosedLoopBackend,
    handle: alife_gpu_backend::GpuBrainHandle,
    patch: &ExperiencePatch,
    receptors: &alife_core::NeuralReceptorFrame,
) -> alife_gpu_backend::GpuPlasticityProbeForTest {
    assert_eq!(
        backend
            .apply_sealed_outcome(handle, patch, receptors)
            .unwrap_err(),
        alife_core::ScaffoldContractError::LearningEvidenceMismatch
    );
    let receipt = backend
        .take_apply_fast_plasticity_failure_receipt()
        .unwrap();
    assert_eq!(
        receipt.malformed_field,
        Some(alife_gpu_backend::GpuRuntimeApplyFastPlasticityMalformedField::CommitGuardRejected)
    );
    let probe = backend.plasticity_probe_for_test(handle).unwrap();
    assert_eq!(probe.receipt[3], 512);
    assert_eq!(probe.receipt[8], 0);
    assert_eq!(
        receipt.actual,
        Some(
            probe.receipt[6..10]
                .iter()
                .map(|word| u64::from(*word))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap()
        )
    );
    let source = support::perception_frame_for_profile_at_tick(
        handle.organism_id().raw(),
        patch.outcome().outcome_tick.raw(),
        SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let physiology = CreatureGenome::early_mammal_founder(
        42,
        FoundationGeneticIdentity::new(42, 1, 1, handle.class_id()).unwrap(),
    )
    .unwrap()
    .express()
    .unwrap();
    let receptors = BiochemistryState::new(&physiology, source.tick())
        .unwrap()
        .neural_receptor_frame(&physiology)
        .unwrap();
    let expression = alife_core::NeuralReceptorPhenotype::compile(
        &support::controlled_learning_n512_phenotype(1.0),
    )
    .unwrap();
    let effects = alife_core::NeuralReceptorEffects::from_frame(&receptors, &expression).unwrap();
    let (frame, recall) = support::empty_recall(&source);
    let upload = backend
        .prepare_memory_context_upload(handle, &frame, &recall)
        .unwrap()
        .bind_neural_receptor_effects(effects)
        .unwrap();
    let batch = alife_gpu_backend::GpuClosedLoopMemoryBatchInput::try_new(vec![
        alife_gpu_backend::GpuClosedLoopMemoryTickInput::try_new(handle, &frame, &upload).unwrap(),
    ])
    .unwrap();
    assert_eq!(
        backend.tick_memory_batch(&batch).unwrap_err(),
        alife_core::ScaffoldContractError::LearningReplayRejected
    );
    assert_eq!(
        backend.plasticity_probe_for_test(handle).unwrap(),
        probe,
        "rejected next tick leaves raw device and host authority unchanged"
    );
    println!(
        "R12 rejected receipt={:?}, host={:?}, synapses={}, replay_spans={}",
        probe.receipt, probe.host_generations, probe.synapse_count, probe.learning[16]
    );
    probe
}

#[test]
fn r12_native_synapse_rejection_blocks_replay_and_retains_authority() {
    for last_only in [true, false] {
        let (mut backend, handle, patch, receptors) = r12_pending_fixture();
        let before = backend.plasticity_probe_for_test(handle).unwrap();
        assert!(before.synapse_count > 128);
        let last = before.synapse_count - 1;
        let faults = if last_only {
            vec![last]
        } else {
            vec![0, 64, last]
        };
        println!(
            "R12 apply tail lanes={}, faults={faults:?}",
            before.synapse_count % 64
        );
        backend
            .inject_plasticity_guard_faults_for_test(handle, &faults, &[], &[])
            .unwrap();
        let after = r12_reject(&mut backend, handle, &patch, &receptors);
        assert_eq!(after.receipt[6], 4);
        assert_eq!(after.receipt[9], after.receipt[10]);
        assert!(faults.contains(&after.receipt[9]));
        assert_ne!(after.receipt[7] & 1, 0, "seeded nonfinite eligibility bit");
        assert_eq!(&after.receipt[11..13], &[u32::MAX, u32::MAX]);
        r12_assert_no_promotion(&before, &after);
        assert_eq!(
            after.replay_samples, before.replay_samples,
            "later replay pass did not capture"
        );
        assert_eq!(after.replay_spans, before.replay_spans);
    }
}

#[test]
fn r12_native_replay_guards_publish_coherent_actual_nominees() {
    // Valid production replay plans contain at most 64 spans. Mixed guards therefore
    // share one production workgroup; this is not cross-workgroup replay evidence.
    for (spans, eligibility) in [
        (vec![1, 7], vec![]),
        (vec![], vec![2, 9]),
        (vec![1, 7], vec![2, 9]),
        (vec![2, 9], vec![1, 7]),
    ] {
        let (mut backend, handle, patch, receptors) = r12_pending_fixture();
        let before = backend.plasticity_probe_for_test(handle).unwrap();
        assert!((10..=64).contains(&before.learning[16]));
        backend
            .inject_plasticity_guard_faults_for_test(handle, &[], &spans, &eligibility)
            .unwrap();
        let after = r12_reject(&mut backend, handle, &patch, &receptors);
        r12_assert_no_promotion(&before, &after);
        assert_eq!(
            after.receipt[10],
            u32::MAX,
            "apply completed before replay fault injection"
        );
        let (guard, context, payload) = if after.receipt[11] != u32::MAX {
            assert!(spans.contains(&after.receipt[11]));
            (5, after.receipt[11], 2)
        } else {
            assert_ne!(after.receipt[12], u32::MAX, "at least one actual nominee");
            assert!(eligibility.contains(&after.receipt[12]));
            (6, after.receipt[12], f32::INFINITY.to_bits())
        };
        if after.receipt[12] != u32::MAX {
            assert!(eligibility.contains(&after.receipt[12]));
        }
        assert_eq!(&after.receipt[6..10], &[guard, payload, 0, context]);
    }
}

#[test]
fn r12_native_success_promotes_exactly_one_transaction() {
    let (mut backend, handle, patch, receptors) = r12_pending_fixture();
    let before = backend.plasticity_probe_for_test(handle).unwrap();
    let receipt = backend
        .apply_sealed_outcome(handle, &patch, &receptors)
        .unwrap();
    let after = backend.plasticity_probe_for_test(handle).unwrap();
    assert_eq!(after.receipt[3], 1);
    assert_eq!(
        receipt.output_fast_generation,
        receipt.input_fast_generation + 1
    );
    assert_eq!(after.learning[1], 1 - before.learning[1]);
    assert_eq!(after.learning[2], 1 - before.learning[2]);
    assert_eq!(after.learning[3], 0);
    assert_eq!(after.learning[13], before.learning[13] + 1);
    assert_eq!(after.host_generations[2], before.host_generations[2] + 1);
    assert_eq!(after.host_generations[4], before.host_generations[4] + 1);
    assert_eq!(after.host_generations[5], before.host_generations[5] + 1);
    assert!(after
        .replay_spans
        .chunks_exact(4)
        .take(after.learning[16] as usize)
        .all(|span| span[2] == 1));
    println!("R12 success receipt={:?}", after.receipt);
}

#[test]
fn superseded_p26_oja_product_runtime_is_absent() {
    let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for relative in [
        "src/plasticity.rs",
        "shaders/p26_plasticity.wgsl",
        "tests/plasticity_oja_parity.rs",
    ] {
        assert!(
            !crate_root.join(relative).exists(),
            "superseded product plasticity surface still exists: {relative}"
        );
    }

    let exports = std::fs::read_to_string(crate_root.join("src/lib.rs")).unwrap();
    for forbidden in [
        "pub mod plasticity;",
        "run_plasticity_gpu_diagnostic",
        "GpuOjaFixedPointConfig",
        "GpuPlasticityPlan",
        "P26_WGSL_PLASTICITY",
    ] {
        assert!(
            !exports.contains(forbidden),
            "superseded product export remains: {forbidden}"
        );
    }
}

fn sealed_measured_outcome(
    handle: alife_gpu_backend::GpuBrainHandle,
    frame: &alife_core::PerceptionFrame,
    tick: &alife_gpu_backend::GpuClosedLoopTick,
) -> ExperiencePatch {
    let sequence_id = ExperienceSequenceId(1);
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
    let physiology = CreatureGenome::early_mammal_founder(
        42,
        FoundationGeneticIdentity::new(42, 1, 1, handle.class_id()).unwrap(),
    )
    .unwrap()
    .express()
    .unwrap();
    let before = BiochemistryState::new(&physiology, frame.tick()).unwrap();
    let after = before
        .advance(
            Tick::new(frame.tick().raw() + 1),
            BodyEventDelta::zero(),
            &physiology,
        )
        .unwrap();
    let measured = MeasuredPhysiologyTransition::new(before, after).unwrap();
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
        SignedValence::ZERO,
        NormalizedScalar::new(0.0).unwrap(),
        NormalizedScalar::new(0.0).unwrap(),
        SignedValence::ZERO,
        NormalizedScalar::new(0.0).unwrap(),
    )
    .unwrap()
    .with_measured_physiology(measured)
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

fn controlled_learning_fixture(
    modulator_sign: f32,
) -> (BrainGenome, DevelopmentState, alife_core::BrainPhenotype) {
    let capacity = BrainCapacityClass::n512();
    let parameters = PlasticityGenomeParameters::try_new_v1(
        0.95,
        1.0,
        0.0,
        0.25,
        modulator_sign,
        -2.0,
        2.0,
        0.5,
        4.0,
        0.5,
    )
    .unwrap();
    let mut genome = BrainGenome::scaffold(42, capacity.id())
        .with_plasticity_parameters(parameters)
        .unwrap();
    genome.alpha_mask = AlphaMask::default_for_projection(NormalizedScalar::new(1.0).unwrap());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
    let phenotype = PhenotypeCompiler::compile(
        &genome,
        &capacity,
        &development,
        SensorProfile::PrivilegedAffordanceV1,
    )
    .unwrap();
    (genome, development, phenotype)
}

fn sealed_measured_event(
    handle: alife_gpu_backend::GpuBrainHandle,
    brain: &alife_core::BrainPhenotype,
    genome: &BrainGenome,
    development: &DevelopmentState,
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
    assert_eq!(genome.id, development.genome_id);
    let capacity = BrainCapacityClass::n512();
    assert_eq!(capacity.id(), handle.class_id());
    let compiled =
        PhenotypeCompiler::compile(genome, &capacity, development, brain.sensor_profile()).unwrap();
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
        development.clone(),
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
    let receptors = support::physiology_receptor_frame(physiology, frame.tick()).unwrap();
    assert_eq!(receptors, before.neural_receptor_frame(physiology).unwrap());
    assert!(!receptors.activations.is_empty());
    let outcome_tick = Tick::new(frame.tick().raw() + 1);
    let after = before
        .advance(outcome_tick, body_event, physiology)
        .unwrap();
    let measured = MeasuredPhysiologyTransition::new(before, after).unwrap();
    assert_ne!(measured.homeostatic_delta, HomeostaticDelta::zero());
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
    OutcomeCreditPacket::from_sealed_patch(&patch)
        .unwrap()
        .with_biochemical_receptors(&receptors)
        .unwrap();
    (patch, receptors)
}

fn nutrition_event() -> BodyEventDelta {
    BodyEventDelta {
        nutrition: 1.0,
        ..BodyEventDelta::zero()
    }
}

fn harm_event() -> BodyEventDelta {
    BodyEventDelta {
        damage: 0.8,
        energy: -0.2,
        ..BodyEventDelta::zero()
    }
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

fn matched_chemistry_tick(
    backend: &mut GpuClosedLoopBackend,
    rows: &[(
        alife_gpu_backend::GpuBrainHandle,
        &alife_core::BrainPhenotype,
        &alife_core::CreaturePhenotype,
        &alife_core::PerceptionFrame,
    )],
) -> Result<
    Vec<(
        alife_core::PerceptionFrame,
        alife_gpu_backend::GpuClosedLoopTick,
    )>,
    alife_core::ScaffoldContractError,
> {
    if rows.is_empty() {
        return Err(alife_core::ScaffoldContractError::InvalidDecisionEvidence);
    }
    let final_frames = rows
        .iter()
        .map(|(_, _, _, source)| support::empty_recall(source).0)
        .collect::<Vec<_>>();
    if final_frames
        .iter()
        .any(|frame| frame.tick() != final_frames[0].tick())
    {
        return Err(alife_core::ScaffoldContractError::InvalidDecisionEvidence);
    }
    let dispatch_generation = backend.runtime_counters_for_test().0 + 1;
    let pressure_rows = rows
        .iter()
        .zip(&final_frames)
        .map(|((handle, _, _, _), frame)| (*handle, frame))
        .collect::<Vec<_>>();
    install_full_recorded_pressures(
        backend,
        &pressure_rows,
        final_frames[0].tick(),
        dispatch_generation,
    );
    let result = support::tick_chemistry_batch(backend, rows)?;
    if result.len() != rows.len() {
        return Err(alife_core::ScaffoldContractError::InvalidDecisionEvidence);
    }
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
        assert_eq!(tick.throttle.microsteps, result[0].1.throttle.microsteps);
        assert_eq!(
            tick.throttle.enabled_route_ids,
            result[0].1.throttle.enabled_route_ids
        );
    }
    Ok(result)
}

fn assert_pending_matches_patch(
    handle: alife_gpu_backend::GpuBrainHandle,
    tick: &alife_gpu_backend::GpuClosedLoopTick,
    patch: &ExperiencePatch,
) {
    let packet = OutcomeCreditPacket::from_sealed_patch(patch).unwrap();
    let identity = tick.pending_eligibility.identity();
    assert_eq!(packet.organism_id(), handle.organism_id());
    assert_eq!(packet.phenotype_hash(), handle.phenotype_hash());
    assert_eq!(identity.handle_generation(), handle.generation());
    assert_eq!(identity.phenotype_hash(), packet.phenotype_hash());
    assert_eq!(identity.dispatch_generation(), packet.dispatch_generation());
    assert_eq!(identity.originating_tick(), packet.originating_tick());
    assert_eq!(identity.frame_digest(), packet.frame_digest());
    assert_eq!(
        identity.active_activation_side(),
        packet.active_activation_side()
    );
    assert_eq!(identity.candidate_index(), packet.selected_candidate());
    assert_eq!(identity.action_id(), packet.selected_action());
    assert_eq!(identity.action_family(), packet.selected_family());
    assert_eq!(
        identity.candidate_feature_digest(),
        packet.candidate_feature_digest()
    );
    GpuOutcomeCreditRecord::try_from(&packet).unwrap();
}

#[test]
fn outcome_credit_record_matches_the_frozen_abi() {
    assert_eq!(std::mem::align_of::<GpuOutcomeCreditRecord>(), 16);
    assert_eq!(std::mem::size_of::<GpuOutcomeCreditRecord>(), 176);
    assert_eq!(
        std::mem::offset_of!(GpuOutcomeCreditRecord, active_activation_side),
        76
    );
    assert_eq!(
        std::mem::offset_of!(GpuOutcomeCreditRecord, candidate_feature_digest),
        80
    );
    assert_eq!(
        std::mem::offset_of!(GpuOutcomeCreditRecord, frame_digest),
        96
    );
    assert_eq!(
        std::mem::offset_of!(GpuOutcomeCreditRecord, dispatch_generation),
        128
    );
    assert_eq!(
        std::mem::offset_of!(GpuOutcomeCreditRecord, biochemical_aversive),
        160
    );
}

#[test]
fn plasticity_wgsl_parses_with_four_passes_and_exactly_seven_heap_bindings() {
    assert!(
        alife_gpu_backend::CLOSED_LOOP_PLASTICITY_WGSL.contains(&format!(
            "const PLASTICITY_DISPATCH_ROW_WORDS:u32 = {}u;",
            alife_gpu_backend::GPU_ACTIVE_DISPATCH_ROW_WORDS
        )),
        "plasticity row stepping must match the complete host dispatch row"
    );
    let module = naga::front::wgsl::parse_str(alife_gpu_backend::CLOSED_LOOP_PLASTICITY_WGSL)
        .expect("plasticity WGSL must parse");
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .expect("plasticity WGSL must validate");
    let entries = module
        .entry_points
        .iter()
        .map(|entry| (entry.name.as_str(), entry.stage, entry.workgroup_size))
        .collect::<Vec<_>>();
    for expected in [
        (
            "initialize_fast_plasticity",
            naga::ShaderStage::Compute,
            [1, 1, 1],
        ),
        (
            "apply_fast_plasticity",
            naga::ShaderStage::Compute,
            [64, 1, 1],
        ),
        (
            "capture_fast_plasticity_replay",
            naga::ShaderStage::Compute,
            [64, 1, 1],
        ),
        (
            "finalize_fast_plasticity",
            naga::ShaderStage::Compute,
            [1, 1, 1],
        ),
    ] {
        assert!(
            entries.contains(&expected),
            "missing WGSL pass {expected:?}"
        );
    }
    let bindings = module
        .global_variables
        .iter()
        .filter_map(|(_, global)| global.binding.as_ref())
        .map(|binding| (binding.group, binding.binding))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        bindings,
        (0_u32..7).map(|binding| (0, binding)).collect(),
        "plasticity must reuse the exact seven production heaps"
    );
}

#[test]
fn rewarding_outcome_changes_next_encounter_before_sleep() {
    let (genome, development, phenotype) = controlled_learning_fixture(1.0);
    let organism = alife_core::OrganismId(4_101);
    let mut backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let handle = backend.insert_brain(organism, phenotype.clone()).unwrap();
    let physiology = support::test_physiology(organism.raw(), &phenotype).unwrap();
    let first_frame = support::perception_frame_for_profile_at_tick(
        organism.raw(),
        900,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let (first_frame, first_tick) =
        support::tick_with_receptors(&mut backend, handle, &phenotype, &physiology, &first_frame)
            .unwrap();
    let (patch, receptors) = sealed_measured_event(
        handle,
        &phenotype,
        &genome,
        &development,
        &physiology,
        &first_frame,
        &first_tick,
        1,
        nutrition_event(),
    );
    let receipt = backend
        .apply_sealed_outcome(handle, &patch, &receptors)
        .unwrap();

    assert_eq!(receipt.handle, handle);
    assert_eq!(receipt.sequence_id, ExperienceSequenceId(1));
    assert_eq!(receipt.dispatch_generation, first_tick.dispatch_generation);
    assert_eq!(
        receipt.active_activation_side,
        first_tick.active_activation_side
    );
    assert_eq!(
        receipt.output_fast_generation,
        receipt.input_fast_generation + 1
    );
    assert_eq!(
        receipt.output_eligibility_generation,
        first_tick
            .pending_eligibility
            .identity()
            .staging_eligibility_generation()
    );
    assert!(receipt.fast_weights_changed > 0);
    assert!(receipt.max_abs_delta > 0.0);

    let second_frame = support::perception_frame_for_profile_at_tick(
        organism.raw(),
        902,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    support::tick_with_receptors(&mut backend, handle, &phenotype, &physiology, &second_frame)
        .expect("learning must unblock the next waking tick");
}

#[test]
fn compact_authority_receipt_is_backend_owned_and_rejects_foreign_or_stale_state() {
    let profile = SensorProfile::GroundedObjectSlotsV1;
    let capacity = BrainCapacityClass::n512();
    let genome = BrainGenome::scaffold(0x4E35_3132_5F00_0001, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    let phenotype = PhenotypeCompiler::compile(&genome, &capacity, &development, profile).unwrap();
    let organism = alife_core::OrganismId(4_111);
    let mut backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let handle = backend.insert_brain(organism, phenotype.clone()).unwrap();
    let physiology = support::test_physiology(organism.raw(), &phenotype).unwrap();
    let frame =
        support::perception_frame_for_profile_at_tick(organism.raw(), 900, profile, true, 2);
    let (frame, tick) =
        support::tick_with_receptors(&mut backend, handle, &phenotype, &physiology, &frame)
            .unwrap();
    let (patch, receptors) = sealed_measured_event(
        handle,
        &phenotype,
        &genome,
        &development,
        &physiology,
        &frame,
        &tick,
        1,
        nutrition_event(),
    );

    let retained = backend
        .authority_receipt_for_sealed_outcome(handle, &tick.pending_eligibility, None, &patch)
        .unwrap();
    retained.validate().unwrap();
    assert!(matches!(
        retained.evidence(),
        alife_gpu_backend::GpuAuthorityCommitEvidenceV1::LearningRetained {
            transaction_generation: 2,
            hardware_receipt_generation,
            ..
        } if hardware_receipt_generation > 0
    ));

    let foreign =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    assert_eq!(
        foreign.authority_receipt_for_sealed_outcome(
            handle,
            &tick.pending_eligibility,
            None,
            &patch,
        ),
        Err(alife_core::ScaffoldContractError::BrainOwnershipMismatch)
    );

    let learning = backend
        .apply_sealed_outcome(handle, &patch, &receptors)
        .unwrap();
    let replay = backend.build_sleep_replay_batch(handle).unwrap();
    assert_eq!(replay.events.len(), 1);
    let base_pairs = phenotype
        .synapses()
        .iter()
        .map(|synapse| (synapse.source(), synapse.target()))
        .collect::<std::collections::BTreeSet<_>>();
    let route_synapses = phenotype
        .synapses()
        .iter()
        .enumerate()
        .filter(|(_, synapse)| {
            (1..=8_u32).any(|offset| {
                let target = (synapse.target() % phenotype.neuron_count()).wrapping_add(offset)
                    % phenotype.neuron_count();
                target != synapse.source() && !base_pairs.contains(&(synapse.source(), target))
            })
        })
        .fold(
            std::collections::BTreeMap::new(),
            |mut by_route, (index, synapse)| {
                by_route
                    .entry(synapse.route_index())
                    .or_insert(index as u32);
                by_route
            },
        )
        .into_values()
        .take(2)
        .collect::<Vec<_>>();
    assert_eq!(route_synapses.len(), 2);
    let targets = replay
        .events
        .iter()
        .map(|event| {
            alife_core::PredictionTargetReceipt::for_successor(
                organism,
                event.sequence_id,
                event.action_id,
                Tick::new(event.originating_tick.raw() + 1),
                event.frame_digest.0,
                SemanticStateVector::new(vec![0.5, 0.25]).unwrap(),
                JointMotorCondition::new(vec![MotorChannelFactor {
                    channel: MotorChannel::Locomotion,
                    primitive: event.action_id,
                    intensity: 0.8,
                    duration_ticks: 1,
                    direction: Vec3f::new(1.0, 0.0, 0.0),
                    stand_off_distance: 0.0,
                    confidence: 0.9,
                    target: None,
                    payload: Vec::new(),
                    coordination_group: 0,
                }])
                .unwrap(),
                SemanticStateVector::new(vec![0.9, 0.1]).unwrap(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let evidence_for_route = |local_synapse_id: u32| {
        let mut route_replay = replay.clone();
        route_replay.synapse_spans = vec![alife_core::sleep::ReplaySynapseSpan {
            local_synapse_id,
            sample_start: 0,
            sample_count: 1,
            reserved: 0,
        }];
        route_replay.eligibility_samples = vec![alife_core::sleep::ReplayEligibilitySample {
            event_index: 0,
            eligibility_q15: 1,
        }];
        route_replay.canonical_digest = route_replay.recompute_canonical_digest().unwrap();
        SleepReplayEvidence::new(route_replay, targets.clone()).unwrap()
    };
    let first_evidence = evidence_for_route(route_synapses[0]);
    let second_evidence = evidence_for_route(route_synapses[1]);
    backend
        .apply_v11_sleep_structural_phase(handle, &first_evidence)
        .expect("first identity-bound replay stays within the structural region budget");
    backend
        .apply_v11_sleep_structural_phase(handle, &second_evidence)
        .expect("a later disjoint replay retains the resident's canonical structural region");
    assert_eq!(
        backend
            .checkpoint_v11(handle)
            .unwrap()
            .sparse_spans
            .iter()
            .flat_map(|span| span.edges.iter().map(|edge| edge.route))
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from([0]),
        "all replay-derived candidates use the resident's canonical structural region",
    );
    let mut full_replay = replay.clone();
    full_replay.synapse_spans = (0..64_u32)
        .map(|local_synapse_id| alife_core::sleep::ReplaySynapseSpan {
            local_synapse_id,
            sample_start: local_synapse_id,
            sample_count: 1,
            reserved: 0,
        })
        .collect();
    full_replay.eligibility_samples = (0..64)
        .map(|_| alife_core::sleep::ReplayEligibilitySample {
            event_index: 0,
            eligibility_q15: 1,
        })
        .collect();
    full_replay.canonical_digest = full_replay.recompute_canonical_digest().unwrap();
    let full_evidence = SleepReplayEvidence::new(full_replay, targets.clone()).unwrap();
    for _ in 0..3 {
        backend
            .apply_v11_sleep_structural_phase(handle, &full_evidence)
            .expect("five bounded structural cycles retain valid resident authority");
    }
    let foreign_organism = alife_core::OrganismId(4_112);
    let foreign_handle = backend
        .insert_brain(foreign_organism, phenotype.clone())
        .unwrap();
    let foreign_physiology = support::test_physiology(foreign_organism.raw(), &phenotype).unwrap();
    assert_eq!(
        backend.apply_v11_sleep_structural_phase(foreign_handle, &second_evidence),
        Err(alife_core::ScaffoldContractError::BrainOwnershipMismatch)
    );
    let replay_before_foreign_tick = backend.build_sleep_replay_batch(handle).unwrap();
    let foreign_frame = support::perception_frame_for_profile_at_tick(
        foreign_handle.organism_id().raw(),
        901,
        profile,
        true,
        2,
    );
    let (foreign_frame, foreign_tick) = support::tick_with_receptors(
        &mut backend,
        foreign_handle,
        &phenotype,
        &foreign_physiology,
        &foreign_frame,
    )
    .unwrap();
    let (foreign_patch, foreign_receptors) = sealed_measured_event(
        foreign_handle,
        &phenotype,
        &genome,
        &development,
        &foreign_physiology,
        &foreign_frame,
        &foreign_tick,
        1,
        nutrition_event(),
    );
    backend
        .apply_sealed_outcome(foreign_handle, &foreign_patch, &foreign_receptors)
        .unwrap();
    assert_eq!(
        backend.build_sleep_replay_batch(handle).unwrap(),
        replay_before_foreign_tick,
        "a same-class foreign tick cannot alter this resident's replay authority",
    );
    let committed = backend
        .authority_receipt_for_sealed_outcome(
            handle,
            &tick.pending_eligibility,
            Some(&learning),
            &patch,
        )
        .unwrap();
    committed.validate().unwrap();
    assert!(matches!(
        committed.evidence(),
        alife_gpu_backend::GpuAuthorityCommitEvidenceV1::LearningCommitted {
            transaction_generation,
            hardware_receipt_generation,
            ..
        } if transaction_generation == learning.transaction_generation
            && hardware_receipt_generation == learning.hardware_receipt_generation
    ));
    let mut tampered_learning = learning;
    tampered_learning.transaction_generation += 1;
    assert!(backend
        .authority_receipt_for_sealed_outcome(
            handle,
            &tick.pending_eligibility,
            Some(&tampered_learning),
            &patch,
        )
        .is_err());

    let next_frame =
        support::perception_frame_for_profile_at_tick(organism.raw(), 902, profile, true, 2);
    let (_, next_tick) =
        support::tick_with_receptors(&mut backend, handle, &phenotype, &physiology, &next_frame)
            .unwrap();
    assert!(backend
        .authority_receipt_for_sealed_outcome(
            handle,
            &tick.pending_eligibility,
            Some(&learning),
            &patch,
        )
        .is_err());
    backend
        .discard_pending_eligibility(handle, next_tick.pending_eligibility.identity())
        .unwrap();
}

#[test]
fn eligibility_contract_is_validated_once_per_row_before_synapse_work() {
    let source = alife_gpu_backend::CLOSED_LOOP_ELIGIBILITY_WGSL;
    let module = naga::front::wgsl::parse_str(source).expect("eligibility WGSL must parse");
    let entry = module
        .entry_points
        .iter()
        .find(|entry| entry.name == "prevalidate_eligibility")
        .expect("eligibility requires a once-per-row prevalidation pass");
    assert_eq!(entry.stage, naga::ShaderStage::Compute);
    assert_eq!(entry.workgroup_size, [1, 1, 1]);
    assert_eq!(
        source.matches("learning_contract_is_valid(").count(),
        2,
        "the full contract may appear only in its definition and row prepass"
    );
    assert_eq!(
        source.matches("pending_row_is_zero(").count(),
        2,
        "the 36-word pending scan may run only in the row prepass"
    );
    assert_eq!(
        source.matches("learning_contract_prevalidated(").count(),
        0,
        "parallel synapse work must consume the prevalidated selection sentinel directly"
    );
}

#[test]
fn parallel_learning_loops_trust_the_validated_row_and_immutable_upload() {
    let eligibility = alife_gpu_backend::CLOSED_LOOP_ELIGIBILITY_WGSL;
    for (entry, next) in [
        (
            "fn accumulate_recurrent_eligibility",
            "fn accumulate_decoder_eligibility",
        ),
        (
            "fn accumulate_decoder_eligibility",
            "fn finalize_pending_eligibility",
        ),
    ] {
        let body = eligibility
            .split_once(entry)
            .expect("parallel eligibility entry exists")
            .1
            .split_once(next)
            .expect("parallel eligibility entry is bounded")
            .0;
        for repeated_guard in [
            "immutable_span_within(",
            "receptor_is_valid(",
            "state_span_within(",
        ] {
            assert!(
                !body.contains(repeated_guard),
                "{entry} repeated immutable upload guard {repeated_guard}"
            );
        }
    }

    let plasticity = alife_gpu_backend::CLOSED_LOOP_PLASTICITY_WGSL;
    let apply_body = plasticity
        .split_once("fn apply_fast_plasticity")
        .expect("fast plasticity entry exists")
        .1
        .split_once("fn capture_fast_plasticity_replay")
        .expect("fast plasticity entry is bounded")
        .0;
    for repeated_guard in [
        "immutable_plan_span_within(",
        "receptor_valid_for_plasticity(",
        "state_span_within(",
    ] {
        assert!(
            !apply_body.contains(repeated_guard),
            "fast plasticity repeated immutable upload guard {repeated_guard}"
        );
    }
}

#[test]
fn fast_plasticity_canonicalizes_zero_before_gpu_storage() {
    let source = alife_gpu_backend::CLOSED_LOOP_PLASTICITY_WGSL;
    let body = source
        .split_once("fn apply_fast_plasticity")
        .expect("fast plasticity entry exists")
        .1
        .split_once("fn capture_fast_plasticity_replay")
        .expect("fast plasticity entry is bounded")
        .0;
    assert!(source.contains("fn canonicalize_state_zero(value:f32) -> f32"));
    assert!(
        body.contains("store_state_f32(inactive_fast_index,canonicalize_state_zero(next_fast))")
    );
}

#[test]
fn recurrent_eligibility_obeys_the_validated_activity_route_mask() {
    let source = alife_gpu_backend::CLOSED_LOOP_ELIGIBILITY_WGSL;
    let body = source
        .split_once("fn accumulate_recurrent_eligibility")
        .expect("recurrent eligibility entry exists")
        .1
        .split_once("fn accumulate_decoder_eligibility")
        .expect("recurrent eligibility entry is bounded")
        .0;
    assert!(body.contains(
        "let route_index = immutable_plan_words[brain.route_indices_offset + local_synapse]"
    ));
    assert!(body.contains("if (!route_enabled_at(route_mask_base, route_index))"));
    assert!(body.contains("store_state_f32(staging_bases.recurrent + local_synapse, previous)"));
}

#[test]
fn same_class_learning_batch_spans_fixed_arenas_without_aliasing_slots() {
    let (genome, development, phenotype) = controlled_learning_fixture(1.0);
    let slot_bytes = alife_gpu_backend::GpuClassBucketPlan::for_phenotype(&phenotype)
        .unwrap()
        .slot_allocation_receipt()
        .unwrap()
        .logical_slot_commit_bytes;
    let profile = support::scaling::bounded_profile(slot_bytes * 5, 512 * 1024 * 1024, 5, 2);
    let mut backend = GpuClosedLoopBackend::new_required(profile).unwrap();
    let handles = (0_u64..5)
        .map(|index| {
            backend
                .insert_brain(alife_core::OrganismId(4_200 + index), phenotype.clone())
                .unwrap()
        })
        .collect::<Vec<_>>();
    let physiologies = handles
        .iter()
        .map(|handle| support::test_physiology(handle.organism_id().raw(), &phenotype).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(backend.allocated_class_arena_count_for_test(), 3);

    let frames = handles
        .iter()
        .map(|handle| {
            support::perception_frame_for_profile_at_tick(
                handle.organism_id().raw(),
                1_200,
                alife_core::SensorProfile::PrivilegedAffordanceV1,
                true,
                2,
            )
        })
        .collect::<Vec<_>>();
    let rows = handles
        .iter()
        .copied()
        .enumerate()
        .map(|(index, handle)| (handle, &phenotype, &physiologies[index], &frames[index]))
        .collect::<Vec<_>>();
    let (frames, ticks): (Vec<_>, Vec<_>) = support::tick_chemistry_batch(&mut backend, &rows)
        .unwrap()
        .into_iter()
        .unzip();
    assert_eq!(
        ticks.iter().map(|tick| tick.handle).collect::<Vec<_>>(),
        handles
    );
    assert!(ticks
        .iter()
        .all(|tick| tick.dispatch_generation == ticks[0].dispatch_generation));

    let outcomes = handles
        .iter()
        .zip(&frames)
        .zip(&ticks)
        .enumerate()
        .map(|(index, ((handle, frame), tick))| {
            sealed_measured_event(
                *handle,
                &phenotype,
                &genome,
                &development,
                &physiologies[index],
                frame,
                tick,
                1,
                nutrition_event(),
            )
        })
        .collect::<Vec<_>>();
    let learning_batch = handles
        .iter()
        .copied()
        .zip(outcomes.iter())
        .map(|(handle, (patch, receptors))| (handle, patch, receptors))
        .collect::<Vec<_>>();
    let receipts = backend.apply_sealed_outcome_batch(&learning_batch).unwrap();
    assert_eq!(receipts.len(), handles.len());
    assert!(receipts
        .iter()
        .all(|receipt| receipt.fast_weights_changed > 0));
    for handle in handles {
        assert!(backend
            .read_active_fast_weights_for_test(handle)
            .unwrap()
            .iter()
            .any(|weight| *weight != 0.0));
    }
}

#[test]
fn modulator_credit_changes_the_next_encounter_relative_to_an_inherited_no_learning_control() {
    let (learning_genome, learning_development, learning_phenotype) =
        controlled_learning_fixture(1.0);
    let (control_genome, control_development, control_phenotype) = controlled_learning_fixture(0.0);
    assert_eq!(
        learning_phenotype.neuron_count(),
        control_phenotype.neuron_count()
    );
    assert_eq!(learning_phenotype.synapses(), control_phenotype.synapses());
    let organism_a = alife_core::OrganismId(4_111);
    let organism_b = alife_core::OrganismId(4_112);
    let mut backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let handle_a = backend
        .insert_brain(organism_a, learning_phenotype.clone())
        .unwrap();
    let handle_b = backend
        .insert_brain(organism_b, control_phenotype.clone())
        .unwrap();
    let physiology_a = support::test_physiology(42, &learning_phenotype).unwrap();
    let physiology_b = support::test_physiology(42, &control_phenotype).unwrap();
    for exposure in 0_u64..8 {
        let tick_raw = 1_000 + exposure * 2;
        let frame_a = support::perception_frame_for_profile_at_tick(
            organism_a.raw(),
            tick_raw,
            alife_core::SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        let frame_b = support::perception_frame_for_profile_at_tick(
            organism_b.raw(),
            tick_raw,
            alife_core::SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        let (_, ticks): (Vec<_>, Vec<_>) = matched_chemistry_tick(
            &mut backend,
            &[
                (handle_a, &learning_phenotype, &physiology_a, &frame_a),
                (handle_b, &control_phenotype, &physiology_b, &frame_b),
            ],
        )
        .unwrap()
        .into_iter()
        .unzip();
        assert_eq!(
            ticks[0].selection.candidate_index,
            ticks[1].selection.candidate_index
        );
        assert!((ticks[0].selection.logit - ticks[1].selection.logit).abs() <= f32::EPSILON);
        assert_eq!(
            backend.pending_eligibility(handle_a).unwrap(),
            Some(ticks[0].pending_eligibility)
        );
        assert_eq!(
            backend.pending_eligibility(handle_b).unwrap(),
            Some(ticks[1].pending_eligibility)
        );
        for tick in &ticks {
            backend
                .discard_pending_eligibility(tick.handle, tick.pending_eligibility.identity())
                .unwrap_or_else(|error| {
                    panic!("activation-only warmup exposure {exposure} failed: {error:?}")
                });
        }
        for handle in [handle_a, handle_b] {
            assert!(backend
                .read_active_fast_weights_for_test(handle)
                .unwrap()
                .iter()
                .all(|value| *value == 0.0));
        }
    }
    let frame_a = support::perception_frame_for_profile_at_tick(
        organism_a.raw(),
        1_100,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let frame_b = support::perception_frame_for_profile_at_tick(
        organism_b.raw(),
        1_100,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let (first_frames, first): (Vec<_>, Vec<_>) = matched_chemistry_tick(
        &mut backend,
        &[
            (handle_a, &learning_phenotype, &physiology_a, &frame_a),
            (handle_b, &control_phenotype, &physiology_b, &frame_b),
        ],
    )
    .unwrap()
    .into_iter()
    .unzip();
    assert_eq!(
        first[0].selection.candidate_index,
        first[1].selection.candidate_index
    );
    assert!((first[0].selection.logit - first[1].selection.logit).abs() <= f32::EPSILON);
    let (rewarded, rewarded_receptors) = sealed_measured_event(
        handle_a,
        &learning_phenotype,
        &learning_genome,
        &learning_development,
        &physiology_a,
        &first_frames[0],
        &first[0],
        9,
        nutrition_event(),
    );
    let (no_learning_control, control_receptors) = sealed_measured_event(
        handle_b,
        &control_phenotype,
        &control_genome,
        &control_development,
        &physiology_b,
        &first_frames[1],
        &first[1],
        9,
        nutrition_event(),
    );
    assert_eq!(rewarded_receptors, control_receptors);
    let rewarded_credit = OutcomeCreditPacket::from_sealed_patch(&rewarded)
        .unwrap()
        .with_biochemical_receptors(&rewarded_receptors)
        .unwrap();
    let control_credit = OutcomeCreditPacket::from_sealed_patch(&no_learning_control)
        .unwrap()
        .with_biochemical_receptors(&control_receptors)
        .unwrap();
    assert!(rewarded_credit.modulator().homeostatic_improvement() > 0.0);
    assert_eq!(
        rewarded_credit.modulator().homeostatic_improvement(),
        control_credit.modulator().homeostatic_improvement()
    );
    assert_pending_matches_patch(handle_a, &first[0], &rewarded);
    assert_pending_matches_patch(handle_b, &first[1], &no_learning_control);
    let receipts = backend
        .apply_sealed_outcome_batch(&[
            (handle_a, &rewarded, &rewarded_receptors),
            (handle_b, &no_learning_control, &control_receptors),
        ])
        .unwrap();
    assert_eq!(receipts.len(), 2);
    assert!(receipts[0].fast_weights_changed > 0);
    assert!(receipts[0].max_abs_delta > GPU_LEARNING_TOLERANCE);
    assert_eq!(receipts[1].fast_weights_changed, 0);
    assert_eq!(receipts[1].max_abs_delta, 0.0);
    let rewarded_fast = backend.read_active_fast_weights_for_test(handle_a).unwrap();
    let control_fast = backend.read_active_fast_weights_for_test(handle_b).unwrap();
    assert!(rewarded_fast.iter().any(|value| *value != 0.0));
    assert!(control_fast.iter().all(|value| *value == 0.0));

    let next_a = support::perception_frame_for_profile_at_tick(
        organism_a.raw(),
        1_102,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let next_b = support::perception_frame_for_profile_at_tick(
        organism_b.raw(),
        1_102,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let after = matched_chemistry_tick(
        &mut backend,
        &[
            (handle_a, &learning_phenotype, &physiology_a, &next_a),
            (handle_b, &control_phenotype, &physiology_b, &next_b),
        ],
    )
    .unwrap()
    .into_iter()
    .map(|(_, tick)| tick)
    .collect::<Vec<_>>();
    assert!(
        (after[0].selection.logit - after[1].selection.logit).abs() > GPU_LEARNING_TOLERANCE,
        "immediately active fast weights must causally alter the next decision: rewarded={}, no_learning_control={}, delta={}, receipt_delta={}",
        after[0].selection.logit,
        after[1].selection.logit,
        (after[0].selection.logit - after[1].selection.logit).abs(),
        receipts[0].max_abs_delta,
    );
    for tick in &after {
        backend
            .discard_pending_eligibility(tick.handle, tick.pending_eligibility.identity())
            .unwrap();
    }
}

#[test]
fn reward_and_pain_change_the_next_decision_in_opposite_directions() {
    let (learning_genome, learning_development, learning_phenotype) =
        controlled_learning_fixture(1.0);
    let (control_genome, control_development, control_phenotype) = controlled_learning_fixture(0.0);
    assert_eq!(
        learning_phenotype.neuron_count(),
        control_phenotype.neuron_count()
    );
    assert_eq!(learning_phenotype.synapses(), control_phenotype.synapses());
    let organisms = [
        alife_core::OrganismId(4_131),
        alife_core::OrganismId(4_132),
        alife_core::OrganismId(4_133),
        alife_core::OrganismId(4_134),
    ];
    let mut backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let handles = [
        backend
            .insert_brain(organisms[0], learning_phenotype.clone())
            .unwrap(),
        backend
            .insert_brain(organisms[1], control_phenotype.clone())
            .unwrap(),
        backend
            .insert_brain(organisms[2], learning_phenotype.clone())
            .unwrap(),
        backend
            .insert_brain(organisms[3], control_phenotype.clone())
            .unwrap(),
    ];
    let physiologies = [
        support::test_physiology(42, &learning_phenotype).unwrap(),
        support::test_physiology(42, &control_phenotype).unwrap(),
        support::test_physiology(42, &learning_phenotype).unwrap(),
        support::test_physiology(42, &control_phenotype).unwrap(),
    ];
    let phenotype_for = |index: usize| {
        if index % 2 == 0 {
            &learning_phenotype
        } else {
            &control_phenotype
        }
    };
    for exposure in 0_u64..8 {
        let tick_raw = 1_200 + exposure * 2;
        let frames = organisms.map(|organism| {
            support::perception_frame_for_profile_at_tick(
                organism.raw(),
                tick_raw,
                alife_core::SensorProfile::PrivilegedAffordanceV1,
                true,
                2,
            )
        });
        let rows = std::array::from_fn::<_, 4, _>(|index| {
            (
                handles[index],
                phenotype_for(index),
                &physiologies[index],
                &frames[index],
            )
        });
        let (_, ticks): (Vec<_>, Vec<_>) = matched_chemistry_tick(&mut backend, &rows)
            .unwrap()
            .into_iter()
            .unzip();
        assert_eq!(
            ticks[0].selection.candidate_index,
            ticks[1].selection.candidate_index
        );
        assert_eq!(
            ticks[2].selection.candidate_index,
            ticks[3].selection.candidate_index
        );
        assert!((ticks[0].selection.logit - ticks[1].selection.logit).abs() <= f32::EPSILON);
        assert!((ticks[2].selection.logit - ticks[3].selection.logit).abs() <= f32::EPSILON);
        for tick in &ticks {
            assert_eq!(
                backend.pending_eligibility(tick.handle).unwrap(),
                Some(tick.pending_eligibility)
            );
            backend
                .discard_pending_eligibility(tick.handle, tick.pending_eligibility.identity())
                .unwrap_or_else(|error| {
                    panic!("activation-only warmup exposure {exposure} failed: {error:?}")
                });
            assert!(backend
                .read_active_fast_weights_for_test(tick.handle)
                .unwrap()
                .iter()
                .all(|value| *value == 0.0));
        }
    }
    let frames = organisms.map(|organism| {
        support::perception_frame_for_profile_at_tick(
            organism.raw(),
            1_300,
            alife_core::SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        )
    });
    let rows = std::array::from_fn::<_, 4, _>(|index| {
        (
            handles[index],
            phenotype_for(index),
            &physiologies[index],
            &frames[index],
        )
    });
    let (frames, before): (Vec<_>, Vec<_>) = matched_chemistry_tick(&mut backend, &rows)
        .unwrap()
        .into_iter()
        .unzip();
    assert_eq!(
        before[0].selection.candidate_index,
        before[1].selection.candidate_index
    );
    assert_eq!(
        before[2].selection.candidate_index,
        before[3].selection.candidate_index
    );
    assert!((before[0].selection.logit - before[1].selection.logit).abs() <= f32::EPSILON);
    assert!((before[2].selection.logit - before[3].selection.logit).abs() <= f32::EPSILON);
    let reward = sealed_measured_event(
        handles[0],
        &learning_phenotype,
        &learning_genome,
        &learning_development,
        &physiologies[0],
        &frames[0],
        &before[0],
        9,
        nutrition_event(),
    );
    let reward_control = sealed_measured_event(
        handles[1],
        &control_phenotype,
        &control_genome,
        &control_development,
        &physiologies[1],
        &frames[1],
        &before[1],
        9,
        nutrition_event(),
    );
    let pain = sealed_measured_event(
        handles[2],
        &learning_phenotype,
        &learning_genome,
        &learning_development,
        &physiologies[2],
        &frames[2],
        &before[2],
        9,
        harm_event(),
    );
    let pain_control = sealed_measured_event(
        handles[3],
        &control_phenotype,
        &control_genome,
        &control_development,
        &physiologies[3],
        &frames[3],
        &before[3],
        9,
        harm_event(),
    );
    assert_eq!(reward.1, reward_control.1);
    assert_eq!(pain.1, pain_control.1);
    let reward_credit = OutcomeCreditPacket::from_sealed_patch(&reward.0)
        .unwrap()
        .with_biochemical_receptors(&reward.1)
        .unwrap();
    let pain_credit = OutcomeCreditPacket::from_sealed_patch(&pain.0)
        .unwrap()
        .with_biochemical_receptors(&pain.1)
        .unwrap();
    assert!(reward_credit.modulator().homeostatic_improvement() > 0.0);
    assert!(pain_credit.modulator().homeostatic_improvement() < 0.0);
    let receipts = backend
        .apply_sealed_outcome_batch(&[
            (handles[0], &reward.0, &reward.1),
            (handles[1], &reward_control.0, &reward_control.1),
            (handles[2], &pain.0, &pain.1),
            (handles[3], &pain_control.0, &pain_control.1),
        ])
        .unwrap();
    assert!(receipts[0].fast_weights_changed > 0);
    assert_eq!(receipts[1].fast_weights_changed, 0);
    assert!(receipts[2].fast_weights_changed > 0);
    assert_eq!(receipts[3].fast_weights_changed, 0);
    assert_eq!(receipts[1].max_abs_delta, 0.0);
    assert_eq!(receipts[3].max_abs_delta, 0.0);

    let next_frames = organisms.map(|organism| {
        support::perception_frame_for_profile_at_tick(
            organism.raw(),
            1_302,
            alife_core::SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        )
    });
    let rows = std::array::from_fn::<_, 4, _>(|index| {
        (
            handles[index],
            phenotype_for(index),
            &physiologies[index],
            &next_frames[index],
        )
    });
    let after = matched_chemistry_tick(&mut backend, &rows)
        .unwrap()
        .into_iter()
        .map(|(_, tick)| tick)
        .collect::<Vec<_>>();
    let reward_delta = after[0].selection.logit - after[1].selection.logit;
    let pain_delta = after[2].selection.logit - after[3].selection.logit;
    assert!(reward_delta.abs() > GPU_LEARNING_TOLERANCE);
    assert!(pain_delta.abs() > GPU_LEARNING_TOLERANCE);
    assert!(
        reward_delta * pain_delta < 0.0,
        "reward={reward_delta}, pain={pain_delta} must oppose their inherited no-learning controls"
    );
    for tick in &after {
        backend
            .discard_pending_eligibility(tick.handle, tick.pending_eligibility.identity())
            .unwrap();
    }
}

#[test]
fn decoder_credit_is_selected_family_and_selected_feature_specific() {
    let (learning_genome, learning_development, learning_phenotype) =
        controlled_learning_fixture(1.0);
    let (control_genome, control_development, control_phenotype) = controlled_learning_fixture(0.0);
    assert_eq!(
        learning_phenotype.neuron_count(),
        control_phenotype.neuron_count()
    );
    assert_eq!(learning_phenotype.synapses(), control_phenotype.synapses());
    let organisms = [alife_core::OrganismId(4_141), alife_core::OrganismId(4_142)];
    let mut backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let handles = [
        backend
            .insert_brain(organisms[0], learning_phenotype.clone())
            .unwrap(),
        backend
            .insert_brain(organisms[1], control_phenotype.clone())
            .unwrap(),
    ];
    let physiologies = [
        support::test_physiology(42, &learning_phenotype).unwrap(),
        support::test_physiology(42, &control_phenotype).unwrap(),
    ];
    for exposure in 0_u64..8 {
        let frames = organisms.map(|organism| {
            support::perception_frame_for_profile_at_tick(
                organism.raw(),
                1_400 + exposure * 2,
                alife_core::SensorProfile::PrivilegedAffordanceV1,
                true,
                2,
            )
        });
        let (_, ticks): (Vec<_>, Vec<_>) = matched_chemistry_tick(
            &mut backend,
            &[
                (
                    handles[0],
                    &learning_phenotype,
                    &physiologies[0],
                    &frames[0],
                ),
                (handles[1], &control_phenotype, &physiologies[1], &frames[1]),
            ],
        )
        .unwrap()
        .into_iter()
        .unzip();
        assert_eq!(
            ticks[0].selection.candidate_index,
            ticks[1].selection.candidate_index
        );
        assert!((ticks[0].selection.logit - ticks[1].selection.logit).abs() <= f32::EPSILON);
        for tick in &ticks {
            assert_eq!(
                backend.pending_eligibility(tick.handle).unwrap(),
                Some(tick.pending_eligibility)
            );
            backend
                .discard_pending_eligibility(tick.handle, tick.pending_eligibility.identity())
                .unwrap_or_else(|error| {
                    panic!("activation-only warmup exposure {exposure} failed: {error:?}")
                });
            assert!(backend
                .read_active_fast_weights_for_test(tick.handle)
                .unwrap()
                .iter()
                .all(|value| *value == 0.0));
        }
    }
    let frames = organisms.map(|organism| {
        support::perception_frame_for_profile_at_tick(
            organism.raw(),
            1_500,
            alife_core::SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        )
    });
    let (frames, ticks): (Vec<_>, Vec<_>) = matched_chemistry_tick(
        &mut backend,
        &[
            (
                handles[0],
                &learning_phenotype,
                &physiologies[0],
                &frames[0],
            ),
            (handles[1], &control_phenotype, &physiologies[1], &frames[1]),
        ],
    )
    .unwrap()
    .into_iter()
    .unzip();
    assert_eq!(
        ticks[0].selection.candidate_index,
        ticks[1].selection.candidate_index
    );
    assert!((ticks[0].selection.logit - ticks[1].selection.logit).abs() <= f32::EPSILON);
    let selected = frames[0].candidates()[usize::from(ticks[0].selection.candidate_index)];
    let reward = sealed_measured_event(
        handles[0],
        &learning_phenotype,
        &learning_genome,
        &learning_development,
        &physiologies[0],
        &frames[0],
        &ticks[0],
        9,
        nutrition_event(),
    );
    let no_learning_control = sealed_measured_event(
        handles[1],
        &control_phenotype,
        &control_genome,
        &control_development,
        &physiologies[1],
        &frames[1],
        &ticks[1],
        9,
        nutrition_event(),
    );
    assert_eq!(reward.1, no_learning_control.1);
    let receipts = backend
        .apply_sealed_outcome_batch(&[
            (handles[0], &reward.0, &reward.1),
            (handles[1], &no_learning_control.0, &no_learning_control.1),
        ])
        .unwrap();
    assert!(receipts[0].fast_weights_changed > 0);
    assert_eq!(receipts[1].fast_weights_changed, 0);
    assert_eq!(receipts[1].max_abs_delta, 0.0);
    let fast = backend
        .read_active_fast_weights_for_test(handles[0])
        .unwrap();
    let control_fast = backend
        .read_active_fast_weights_for_test(handles[1])
        .unwrap();
    assert_eq!(fast.len(), learning_phenotype.synapses().len());
    assert!(control_fast.iter().all(|value| *value == 0.0));

    let mut credited_decoder_weights = 0_usize;
    for (index, synapse) in learning_phenotype.synapses().iter().enumerate() {
        let alife_core::CompiledSynapseKind::Decoder(coordinate) = synapse.kind() else {
            continue;
        };
        let should_receive_credit = if coordinate.head()
            == alife_core::DecoderHeadKind::ActionCandidate
            && coordinate.family() == selected.family
        {
            *selected
                .features
                .0
                .get(usize::from(coordinate.input_lane()))
                .expect("action decoder lanes must stay inside the action feature ABI")
                != 0.0
        } else {
            false
        };
        if should_receive_credit {
            credited_decoder_weights += usize::from(fast[index] != 0.0);
        } else {
            assert_eq!(
                fast[index],
                0.0,
                "credit leaked to decoder {:?} lane {}",
                coordinate.family(),
                coordinate.input_lane()
            );
        }
    }
    assert!(credited_decoder_weights > 0);
}

#[test]
fn replayed_sealed_credit_is_rejected_without_blocking_a_later_tick() {
    let (genome, development, phenotype) = controlled_learning_fixture(1.0);
    let organism = alife_core::OrganismId(4_121);
    let mut backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let handle = backend.insert_brain(organism, phenotype.clone()).unwrap();
    let physiology = support::test_physiology(organism.raw(), &phenotype).unwrap();
    let frame = support::perception_frame_for_profile_at_tick(
        organism.raw(),
        1_100,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let (frame, tick) =
        support::tick_with_receptors(&mut backend, handle, &phenotype, &physiology, &frame)
            .unwrap();
    let (patch, receptors) = sealed_measured_event(
        handle,
        &phenotype,
        &genome,
        &development,
        &physiology,
        &frame,
        &tick,
        1,
        nutrition_event(),
    );
    let committed = backend
        .apply_sealed_outcome(handle, &patch, &receptors)
        .unwrap();
    assert_eq!(
        backend.apply_sealed_outcome(handle, &patch, &receptors),
        Err(alife_core::ScaffoldContractError::LearningReplayRejected)
    );

    let next_frame = support::perception_frame_for_profile_at_tick(
        organism.raw(),
        1_102,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        2,
    );
    let (_, next_tick) =
        support::tick_with_receptors(&mut backend, handle, &phenotype, &physiology, &next_frame)
            .unwrap();
    assert_eq!(committed.output_fast_generation, 2);
    backend
        .discard_pending_eligibility(handle, next_tick.pending_eligibility.identity())
        .unwrap();
}

#[test]
fn foreign_outcome_is_rejected_before_gpu_mutation_and_preserves_both_pending_rows() {
    let (genome, development, phenotype) = controlled_learning_fixture(1.0);
    let organisms = [alife_core::OrganismId(4_151), alife_core::OrganismId(4_152)];
    let mut backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let handles =
        organisms.map(|organism| backend.insert_brain(organism, phenotype.clone()).unwrap());
    let physiologies =
        organisms.map(|organism| support::test_physiology(organism.raw(), &phenotype).unwrap());
    let frames = organisms.map(|organism| {
        support::perception_frame_for_profile_at_tick(
            organism.raw(),
            1_600,
            alife_core::SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        )
    });
    let rows = std::array::from_fn::<_, 2, _>(|index| {
        (
            handles[index],
            &phenotype,
            &physiologies[index],
            &frames[index],
        )
    });
    let (frames, ticks): (Vec<_>, Vec<_>) = support::tick_chemistry_batch(&mut backend, &rows)
        .unwrap()
        .into_iter()
        .unzip();
    let pending_before = [
        backend.pending_eligibility(handles[0]).unwrap().unwrap(),
        backend.pending_eligibility(handles[1]).unwrap().unwrap(),
    ];
    let (patch_a, receptors) = sealed_measured_event(
        handles[0],
        &phenotype,
        &genome,
        &development,
        &physiologies[0],
        &frames[0],
        &ticks[0],
        1,
        nutrition_event(),
    );
    assert_eq!(
        backend.apply_sealed_outcome(handles[1], &patch_a, &receptors),
        Err(alife_core::ScaffoldContractError::LearningEvidenceMismatch)
    );
    assert_eq!(
        backend.pending_eligibility(handles[0]).unwrap(),
        Some(pending_before[0])
    );
    assert_eq!(
        backend.pending_eligibility(handles[1]).unwrap(),
        Some(pending_before[1])
    );
    assert!(backend
        .read_active_fast_weights_for_test(handles[0])
        .unwrap()
        .iter()
        .all(|value| *value == 0.0));
    assert!(backend
        .read_active_fast_weights_for_test(handles[1])
        .unwrap()
        .iter()
        .all(|value| *value == 0.0));
    for index in 0..2 {
        backend
            .discard_pending_eligibility(
                handles[index],
                ticks[index].pending_eligibility.identity(),
            )
            .unwrap();
    }
}

#[test]
fn abi_conversion_uses_only_validated_sealed_credit() {
    let capacity = BrainCapacityClass::n512();
    let genome = BrainGenome::scaffold(13, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
    let phenotype = PhenotypeCompiler::compile(
        &genome,
        &capacity,
        &development,
        SensorProfile::PrivilegedAffordanceV1,
    )
    .unwrap();
    let organism = alife_core::OrganismId(4_102);
    let mut backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .unwrap();
    let handle = backend.insert_brain(organism, phenotype.clone()).unwrap();
    let physiology = support::test_physiology(organism.raw(), &phenotype).unwrap();
    let frame = support::perception_frame_for_profile_at_tick(
        organism.raw(),
        910,
        alife_core::SensorProfile::PrivilegedAffordanceV1,
        true,
        1,
    );
    let (frame, tick) =
        support::tick_with_receptors(&mut backend, handle, &phenotype, &physiology, &frame)
            .unwrap();
    let (patch, _) = sealed_measured_event(
        handle,
        &phenotype,
        &genome,
        &development,
        &physiology,
        &frame,
        &tick,
        1,
        nutrition_event(),
    );
    let packet = OutcomeCreditPacket::from_sealed_patch(&patch).unwrap();
    let record = GpuOutcomeCreditRecord::try_from(&packet).unwrap();

    assert_eq!(record.schema_version, u32::from(packet.schema_version()));
    assert_eq!(
        record.active_activation_side,
        u32::from(tick.active_activation_side)
    );
    assert_eq!(
        record.dispatch_generation[0] as u64,
        tick.dispatch_generation & 0xffff_ffff
    );
    assert_eq!(
        record.biochemical_appetitive,
        packet.modulator().frame().lanes()[6]
    );
    assert_eq!(
        record.biochemical_aversive,
        packet.modulator().frame().lanes()[7]
    );

    backend
        .discard_pending_eligibility(handle, tick.pending_eligibility.identity())
        .unwrap();
}
