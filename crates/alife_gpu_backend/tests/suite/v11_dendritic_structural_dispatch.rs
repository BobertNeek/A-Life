#[allow(unused_imports)]
use crate::support;

use alife_core::sleep::SleepReplayEvidence;
use alife_core::{
    BiochemistryState, BodyEventDelta, BrainCapacityClass, BrainGenome, CandidateActionFamily,
    CoactivationEvidence, CognitiveContextFrame, CompiledSynapseKind, Confidence, DecisionSnapshot,
    DendriticBranch, DendriticBranchSet, DendriticInputRef, DevelopmentState, ExperiencePatch,
    ExperienceSequenceId, JointMotorCondition, JointPhysicalOutcome, MeasuredPhysiologyTransition,
    NeuralActionSelection, NormalizedScalar, OrganismId, PhysicalActionOutcome,
    PhysicalContactKind, PostActionOutcome, PreActionSnapshot, PredictionTargetReceipt,
    SemanticStateVector, SensorProfile, SignedValence, StructuralPlasticityConfig,
    StructuralPlasticityState, Tick, Vec3f,
};
use alife_gpu_backend::{
    GpuBrainHandle, GpuClosedLoopBackend, GpuClosedLoopTick, GpuRuntimeProfile,
    GpuV11MutableStateProbe,
};

fn discard(
    backend: &mut GpuClosedLoopBackend,
    tick: &GpuClosedLoopTick,
) -> Result<(), alife_core::ScaffoldContractError> {
    backend
        .discard_pending_eligibility(tick.handle, tick.pending_eligibility.identity())
        .map(|_| ())
}

fn seal_anchor_outcome(
    backend: &mut GpuClosedLoopBackend,
    handle: GpuBrainHandle,
    phenotype: &alife_core::BrainPhenotype,
    physiology: &alife_core::CreaturePhenotype,
    frame: &alife_core::PerceptionFrame,
    tick: &GpuClosedLoopTick,
) -> Result<PredictionTargetReceipt, alife_core::ScaffoldContractError> {
    let sequence_id = ExperienceSequenceId(1);
    let genome = BrainGenome::scaffold(27, BrainCapacityClass::n512().id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
    let selection = NeuralActionSelection {
        candidate_index: tick.selection.candidate_index,
        logit: tick.selection.logit,
        confidence: tick.selection.confidence,
        active_tiles: tick.selection.active_tiles,
        active_synapses: tick.selection.active_synapses,
    };
    let command = frame.candidates()[usize::from(selection.candidate_index)].to_command(
        handle.organism_id(),
        Confidence::new(selection.confidence.raw()).unwrap(),
    )?;
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
        &phenotype
            .candidate_decoder()
            .factorized_motor_channels(phenotype)?,
        &command,
        selection.candidate_index,
        tick.speech_payload.as_ref(),
        prompted,
    )?;
    let pre_action = PreActionSnapshot::from_neural_frame(
        sequence_id,
        handle.class_id(),
        handle.phenotype_hash(),
        genome.id,
        genome.schema_version,
        development,
        frame.clone(),
    )?;
    let decision = DecisionSnapshot::from_neural_selection(
        sequence_id,
        handle.phenotype_hash(),
        tick.dispatch_generation,
        tick.active_activation_side,
        frame,
        selection,
        command,
    )?;
    let before = BiochemistryState::new(physiology, frame.tick())?;
    let receptors = before.neural_receptor_frame(physiology)?;
    let outcome_tick = Tick::new(frame.tick().raw() + 1);
    let after = before.advance(
        outcome_tick,
        BodyEventDelta {
            nutrition: 1.0,
            ..BodyEventDelta::zero()
        },
        physiology,
    )?;
    let measured = MeasuredPhysiologyTransition::new(before, after)?;
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
        NormalizedScalar::new(measured.aversive_harm())?,
        measured.energy_delta,
        NormalizedScalar::ZERO,
    )?
    .with_measured_physiology(measured)?
    .with_v11_joint(JointPhysicalOutcome::new(physical, Vec::new())?, work)?;
    let target = PredictionTargetReceipt::for_successor(
        handle.organism_id(),
        sequence_id,
        selected_action_id,
        frame.tick(),
        frame.frame_digest().0,
        SemanticStateVector::new(vec![0.5, 0.25])?,
        JointMotorCondition::from_bundle(&bundle)?,
        SemanticStateVector::new(vec![0.4, 0.3])?,
    )?;
    let patch = ExperiencePatch::new_v11_with_decision(
        pre_action,
        decision,
        bundle,
        outcome,
        target.clone(),
        work,
        CognitiveContextFrame::empty(handle.organism_id(), sequence_id, frame.tick())?,
    )?;
    backend.apply_sealed_outcome(handle, &patch, &receptors)?;
    Ok(target)
}

#[test]
fn target_indexed_dendrites_visit_only_the_target_span(
) -> Result<(), alife_core::ScaffoldContractError> {
    let phenotype = support::controlled_sensory_n512_phenotype();
    let microsteps = u32::from(phenotype.microstep_count());
    let target = phenotype.candidate_decoder().motor_start();
    let second_source = if target + 1 < phenotype.neuron_count() {
        target + 1
    } else {
        target
    };
    let branches = DendriticBranchSet::new(vec![DendriticBranch::new(
        target,
        -1.0,
        1.0,
        vec![
            DendriticInputRef::new(target, 1.0).unwrap(),
            DendriticInputRef::new(second_source, 1.0).unwrap(),
        ],
    )?])?;

    let physiology = support::test_physiology(10, &phenotype)?;
    let mut backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())?;
    let handle = backend.insert_brain(OrganismId(10), phenotype.clone())?;
    backend.set_v11_dendritic_branches(handle, branches)?;
    let source = support::perception_frame_for_profile_at_tick(
        10,
        80,
        SensorProfile::PrivilegedAffordanceV1,
        true,
        1,
    );
    let (_frame, tick) =
        support::tick_with_receptors(&mut backend, handle, &phenotype, &physiology, &source)?;
    assert_eq!(tick.v11_work.dendritic.branches_evaluated, microsteps);
    assert_eq!(tick.v11_work.dendritic.inputs_evaluated, 2 * microsteps);
    assert_eq!(tick.v11_work.dendritic.work_units, 3 * microsteps);
    discard(&mut backend, &tick)?;
    Ok(())
}

fn prune_config(max_regions: u16) -> StructuralPlasticityConfig {
    StructuralPlasticityConfig {
        max_candidates_per_region: 8,
        max_regions,
        max_accepted_per_phase: 1,
        max_structural_edges: 1,
        min_candidate_score: 2,
        prune_score_below: 10,
    }
}

#[test]
fn normal_tick_joins_dendrites_growth_pruning_and_bounded_work(
) -> Result<(), alife_core::ScaffoldContractError> {
    let phenotype = support::controlled_sensory_n512_phenotype();
    let motor_start = phenotype.candidate_decoder().motor_start();
    let inspect_family = phenotype
        .candidate_decoder()
        .families()
        .iter()
        .find(|family| family.family() == CandidateActionFamily::Inspect)
        .expect("controlled phenotype has an Inspect decoder family");
    let structural_target =
        phenotype.synapses()[inspect_family.decoder_synapse_start() as usize].source();
    let recurrent_count = phenotype
        .synapses()
        .iter()
        .filter(|synapse| synapse.kind() == CompiledSynapseKind::Recurrent)
        .count() as u32;
    let inspect_decoder_local = inspect_family
        .decoder_synapse_start()
        .checked_sub(recurrent_count)
        .expect("Inspect family is in the decoder span");
    let structural_route = phenotype
        .projections()
        .iter()
        .map(|projection| projection.route_index())
        .find(|route| *route > 0 && *route < 16)
        .expect("controlled phenotype has a nonzero bounded route");
    let source = motor_start + 1;
    assert!(source < phenotype.neuron_count());
    let branches = DendriticBranchSet::new(
        (0..usize::from(phenotype.candidate_decoder().motor_width()))
            .map(|index| {
                let target = motor_start + index as u32;
                let second_source = if target + 1 < phenotype.neuron_count() {
                    target + 1
                } else {
                    target
                };
                DendriticBranch::new(
                    target,
                    -1.0,
                    6.0,
                    vec![
                        DendriticInputRef::new(target, 1.0).unwrap(),
                        DendriticInputRef::new(second_source, 1.0).unwrap(),
                    ],
                )
                .unwrap()
            })
            .collect(),
    )
    .unwrap();

    let mut backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())?;
    let physiology = support::test_physiology(1, &phenotype)?;
    let control = backend.insert_brain(OrganismId(1), phenotype.clone())?;
    let branch_subject = backend.insert_brain(OrganismId(2), phenotype.clone())?;
    let growth_subject = backend.insert_brain(OrganismId(3), phenotype.clone())?;
    let prune_subject = backend.insert_brain(OrganismId(4), phenotype.clone())?;
    let fresh_checkpoint = backend.checkpoint_v11(control)?;
    assert!(!fresh_checkpoint.dendritic_branches.is_empty());
    backend.set_v11_dendritic_branches(control, DendriticBranchSet::default())?;

    let frame = |organism: u64, tick: u64| {
        support::perception_frame_for_profile_at_tick(
            organism,
            tick,
            SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        )
    };

    let baseline_sources = [frame(1, 77), frame(2, 77), frame(3, 77), frame(4, 77)];
    let (_, baseline_ticks): (Vec<_>, Vec<_>) = support::tick_chemistry_batch(
        &mut backend,
        &[
            (control, &phenotype, &physiology, &baseline_sources[0]),
            (
                branch_subject,
                &phenotype,
                &physiology,
                &baseline_sources[1],
            ),
            (
                growth_subject,
                &phenotype,
                &physiology,
                &baseline_sources[2],
            ),
            (prune_subject, &phenotype, &physiology, &baseline_sources[3]),
        ],
    )?
    .into_iter()
    .unzip();
    for tick in &baseline_ticks {
        discard(&mut backend, tick)?;
    }

    backend.set_v11_dendritic_branches(growth_subject, branches.clone())?;
    backend.set_v11_dendritic_branches(prune_subject, branches)?;
    let branch_sources = [frame(1, 78), frame(2, 78), frame(3, 78), frame(4, 78)];
    let (branch_frames, branch_ticks): (Vec<_>, Vec<_>) = support::tick_chemistry_batch(
        &mut backend,
        &[
            (control, &phenotype, &physiology, &branch_sources[0]),
            (branch_subject, &phenotype, &physiology, &branch_sources[1]),
            (growth_subject, &phenotype, &physiology, &branch_sources[2]),
            (prune_subject, &phenotype, &physiology, &branch_sources[3]),
        ],
    )?
    .into_iter()
    .unzip();
    assert_ne!(
        branch_ticks[0].selection.logit.to_bits(),
        branch_ticks[1].selection.logit.to_bits()
    );
    assert!(branch_ticks[1].v11_work.dendritic.branches_evaluated > 0);
    assert!(branch_ticks[1].v11_work.dendritic.inputs_evaluated >= 2);
    assert!(branch_ticks[1].v11_work.dendritic.gated_branches > 0);
    assert_eq!(
        branch_frames[1].candidates()[branch_ticks[1].selection.candidate_index as usize].family,
        CandidateActionFamily::Inspect
    );
    discard(&mut backend, &branch_ticks[0])?;
    discard(&mut backend, &branch_ticks[1])?;
    seal_anchor_outcome(
        &mut backend,
        growth_subject,
        &phenotype,
        &physiology,
        &branch_frames[2],
        &branch_ticks[2],
    )?;
    discard(&mut backend, &branch_ticks[3])?;

    for handle in [growth_subject, prune_subject] {
        let mut checkpoint = backend.checkpoint_v11(handle)?;
        checkpoint.structural = StructuralPlasticityState::new(
            phenotype.neuron_count(),
            prune_config(structural_route + 1),
        )
        .map_err(|_| alife_core::ScaffoldContractError::InvalidSparseProjectionSchema)?;
        backend.restore_v11(handle, checkpoint)?;
    }

    let evidence = CoactivationEvidence {
        region: structural_route,
        source,
        target: structural_target,
        coactivation: 100,
        eligibility: 0,
        concept_gap_support: 0,
    };
    let learned_state = GpuV11MutableStateProbe {
        lifetime_weight_banks: [0.125_f32.to_bits(), (-0.25_f32).to_bits()],
        fast_weight_banks: [0.375_f32.to_bits(), (-0.5_f32).to_bits()],
        decoder_eligibility_banks: [0.625_f32.to_bits(), (-0.75_f32).to_bits()],
        activation_sides: [0.875_f32.to_bits(), (-1.0_f32).to_bits()],
    };
    for handle in [growth_subject, prune_subject] {
        backend.seed_v11_mutable_state_for_test(
            handle,
            inspect_decoder_local,
            structural_target,
            learned_state,
        )?;
    }
    let rollback_checkpoint = backend.checkpoint_v11(growth_subject)?;
    let invalid_evidence = CoactivationEvidence {
        target: phenotype.neuron_count(),
        ..evidence
    };
    assert!(backend
        .apply_v11_structural_phase(growth_subject, &[invalid_evidence])
        .is_err());
    assert_eq!(backend.checkpoint_v11(growth_subject)?, rollback_checkpoint);
    assert_eq!(
        backend.read_v11_mutable_state_for_test(
            growth_subject,
            inspect_decoder_local,
            structural_target,
        )?,
        learned_state
    );
    let growth_work = backend.apply_v11_structural_phase(growth_subject, &[evidence])?;
    assert_eq!(growth_work.structural.accepted_edges, 1);
    assert!(growth_work.structural.candidate_comparisons <= 8);
    assert!(growth_work.structural.active_edges > 0);
    let growth_checkpoint = backend.checkpoint_v11(growth_subject)?;
    assert_eq!(
        growth_checkpoint.sparse_spans[0].edges[0].route,
        u32::from(structural_route)
    );
    let topology_readback_before = backend.mutable_slot_readback_metrics();
    let growth_live_topology = backend.checkpoint_live_topology(growth_subject)?;
    let topology_readback_after = backend.mutable_slot_readback_metrics();
    assert_eq!(
        topology_readback_after.calls - topology_readback_before.calls,
        1,
        "live topology capture must batch immutable plan and weight ranges into one readback"
    );
    assert!(topology_readback_after.bytes > topology_readback_before.bytes);
    let mut tampered_topology = growth_live_topology.clone();
    tampered_topology.canonical_digest[0] ^= 1;
    assert!(tampered_topology
        .validate_for_capacity(&BrainCapacityClass::n512())
        .is_err());
    assert_eq!(
        growth_live_topology.total_synapse_count,
        phenotype.budgets().global.total_synapses + 1
    );
    assert_eq!(
        growth_live_topology.recurrent_synapse_count,
        phenotype.budgets().global.recurrent_synapses + 1
    );
    assert_eq!(
        backend.read_v11_mutable_state_for_test(
            growth_subject,
            inspect_decoder_local,
            structural_target,
        )?,
        learned_state
    );

    let _ = backend.apply_v11_structural_phase(prune_subject, &[evidence])?;
    let mut prune_checkpoint = backend.checkpoint_v11(prune_subject)?;
    prune_checkpoint
        .structural
        .record_edge_support(source, structural_target, 0)
        .map_err(|_| alife_core::ScaffoldContractError::InvalidSparseProjectionSchema)?;
    backend.restore_v11(prune_subject, prune_checkpoint)?;
    let prune_work = backend.apply_v11_structural_phase(prune_subject, &[])?;
    assert_eq!(prune_work.structural.pruned_edges, 1);
    assert_eq!(prune_work.structural.active_edges, 0);
    let pruned_live_topology = backend.checkpoint_live_topology(prune_subject)?;
    assert_eq!(
        pruned_live_topology.total_synapse_count,
        phenotype.budgets().global.total_synapses
    );
    assert_eq!(
        pruned_live_topology.recurrent_synapse_count,
        phenotype.budgets().global.recurrent_synapses
    );
    assert_eq!(
        backend.read_v11_mutable_state_for_test(
            prune_subject,
            inspect_decoder_local,
            structural_target,
        )?,
        learned_state
    );
    // `prune_subject` still has zero committed replay events (`trace_has_anchor == false`),
    // so clear the synthetic non-zero decoder eligibility probe before the next GPU tick.
    backend.seed_v11_mutable_state_for_test(
        prune_subject,
        inspect_decoder_local,
        structural_target,
        GpuV11MutableStateProbe {
            decoder_eligibility_banks: [0, 0],
            ..learned_state
        },
    )?;
    let replay = backend.build_sleep_replay_batch(prune_subject)?;
    let evidence = SleepReplayEvidence::new(replay, Vec::new())?;
    backend.apply_v11_sleep_structural_phase(prune_subject, &evidence)?;

    let structural_sources = [frame(1, 79), frame(2, 79), frame(3, 79), frame(4, 79)];
    let (structural_frames, structural_ticks): (Vec<_>, Vec<_>) = support::tick_chemistry_batch(
        &mut backend,
        &[
            (control, &phenotype, &physiology, &structural_sources[0]),
            (
                branch_subject,
                &phenotype,
                &physiology,
                &structural_sources[1],
            ),
            (
                growth_subject,
                &phenotype,
                &physiology,
                &structural_sources[2],
            ),
            (
                prune_subject,
                &phenotype,
                &physiology,
                &structural_sources[3],
            ),
        ],
    )?
    .into_iter()
    .unzip();
    assert_ne!(
        structural_ticks[2].selection.logit.to_bits(),
        structural_ticks[3].selection.logit.to_bits()
    );
    assert!(structural_ticks[2].v11_work.dendritic.work_units > 0);
    assert!(structural_ticks[2].v11_work.dendritic.gated_branches > 0);
    assert!(structural_ticks[2].v11_work.cognitive.structural_ops > 0);
    assert!(structural_ticks[2].v11_work.structural.active_edges > 0);
    assert_eq!(structural_ticks[3].v11_work.structural.active_edges, 0);
    assert_eq!(
        structural_frames[2].candidates()[structural_ticks[2].selection.candidate_index as usize]
            .family,
        CandidateActionFamily::Inspect
    );
    for tick in &structural_ticks {
        discard(&mut backend, tick)?;
    }
    Ok(())
}
