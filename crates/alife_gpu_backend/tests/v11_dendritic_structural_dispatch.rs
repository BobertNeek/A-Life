#![cfg(feature = "gpu-tests")]

mod support;

use alife_core::{
    CandidateActionFamily, CoactivationEvidence, CompiledSynapseKind, DendriticBranch,
    DendriticBranchSet, DendriticInputRef, OrganismId, SensorProfile, StructuralPlasticityConfig,
    StructuralPlasticityState,
};
use alife_gpu_backend::{
    GpuClosedLoopBackend, GpuClosedLoopTick, GpuRuntimeProfile, GpuV11MutableStateProbe,
};

fn discard(
    backend: &mut GpuClosedLoopBackend,
    tick: &GpuClosedLoopTick,
) -> Result<(), alife_core::ScaffoldContractError> {
    backend
        .discard_pending_eligibility(tick.handle, &tick.pending_eligibility.identity())
        .map(|_| ())
}

#[test]
fn target_indexed_dendrites_visit_only_the_target_span(
) -> Result<(), alife_core::ScaffoldContractError> {
    let phenotype = support::controlled_sensory_n512_phenotype();
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

    let mut backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())?;
    let handle = backend.insert_brain(OrganismId(10), phenotype)?;
    backend.set_v11_dendritic_branches(handle, branches)?;
    let frame = support::perception_frame_for_profile_at_tick(
        10,
        80,
        SensorProfile::PrivilegedAffordanceV1,
        true,
        1,
    );
    let mut ticks = backend.tick_batch(&[(handle, frame)])?;
    let tick = ticks.pop().expect("one dendritic tick");
    assert_eq!(tick.v11_work.dendritic.branches_evaluated, 1);
    assert_eq!(tick.v11_work.dendritic.inputs_evaluated, 2);
    assert_eq!(tick.v11_work.dendritic.work_units, 3);
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

    let baseline = backend.tick_batch(&[
        (control, frame(1, 77)),
        (branch_subject, frame(2, 77)),
        (growth_subject, frame(3, 77)),
        (prune_subject, frame(4, 77)),
    ])?;
    for tick in &baseline {
        discard(&mut backend, tick)?;
    }

    backend.set_v11_dendritic_branches(growth_subject, branches.clone())?;
    backend.set_v11_dendritic_branches(prune_subject, branches)?;
    let branch_ticks = backend.tick_batch(&[
        (control, frame(1, 78)),
        (branch_subject, frame(2, 78)),
        (growth_subject, frame(3, 78)),
        (prune_subject, frame(4, 78)),
    ])?;
    assert_ne!(
        branch_ticks[0].selection.logit.to_bits(),
        branch_ticks[1].selection.logit.to_bits()
    );
    assert!(branch_ticks[1].v11_work.dendritic.branches_evaluated > 0);
    assert!(branch_ticks[1].v11_work.dendritic.inputs_evaluated >= 2);
    assert!(branch_ticks[1].v11_work.dendritic.gated_branches > 0);
    assert_eq!(
        frame(2, 78).candidates()[branch_ticks[1].selection.candidate_index as usize].family,
        CandidateActionFamily::Inspect
    );
    for tick in &branch_ticks {
        discard(&mut backend, tick)?;
    }

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
    assert_eq!(
        backend.read_v11_mutable_state_for_test(
            prune_subject,
            inspect_decoder_local,
            structural_target,
        )?,
        learned_state
    );
    backend.apply_v11_sleep_structural_phase(prune_subject)?;

    let structural_ticks = backend.tick_batch(&[
        (control, frame(1, 79)),
        (branch_subject, frame(2, 79)),
        (growth_subject, frame(3, 79)),
        (prune_subject, frame(4, 79)),
    ])?;
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
        frame(3, 79).candidates()[structural_ticks[2].selection.candidate_index as usize].family,
        CandidateActionFamily::Inspect
    );
    for tick in &structural_ticks {
        discard(&mut backend, tick)?;
    }
    Ok(())
}
