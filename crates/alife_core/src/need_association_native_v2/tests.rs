use super::*;
use crate::{
    AlleleSide, BiochemicalDriveChannel as Drive, BiochemicalSourceLocus as Source,
    BiochemicalTargetLocus as Target, BrainCapacityClass, CreatureGenome,
    FoundationGeneticIdentity, NativeNeedAssociationCircuitV1, NativeNeedAssociationRecipeV1,
    NativeNeedReferenceKindV1, Tick,
};

// Independent reconstruction of the actual fresh phase3 founder's inherited
// calibration (world/new_game.rs). No world step or living state is edited.
fn phenotype(seed: u64) -> CreaturePhenotype {
    let mut genome = CreatureGenome::early_mammal_founder(
        seed,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N2048_ID).unwrap(),
    )
    .unwrap();
    let graph = genome.chemistry.graph.expressed();
    let hunger = graph
        .receptors()
        .iter()
        .find(|r| r.target == Target::Drive(Drive::Hunger))
        .unwrap()
        .source;
    let hunger_emitter = graph
        .emitters()
        .iter()
        .position(|e| e.source == Source::EnergyDeficit && e.target == hunger)
        .unwrap();
    let repair = graph
        .receptors()
        .iter()
        .position(|r| r.target == Target::OrganRepair)
        .unwrap();
    for allele in [AlleleSide::Maternal, AlleleSide::Paternal] {
        genome.chemistry.graph = genome
            .chemistry
            .graph
            .clone()
            .with_emitter_gain(allele, hunger_emitter, 0.03)
            .unwrap()
            .with_receptor_nominal(allele, repair, 0.02)
            .unwrap();
    }
    for allele in [AlleleSide::Maternal, AlleleSide::Paternal] {
        genome.chemistry.graph = genome
            .chemistry
            .graph
            .clone()
            .with_waking_recovery(allele)
            .unwrap()
            .with_health_distress(allele)
            .unwrap()
            .with_player_reward(allele)
            .unwrap()
            .with_play_stimulation(allele)
            .unwrap();
    }
    let graph = genome.chemistry.graph.expressed();
    let pain = graph
        .receptors()
        .iter()
        .find(|r| r.target == Target::Drive(Drive::Pain))
        .unwrap()
        .source;
    let damage = graph
        .emitters()
        .iter()
        .position(|e| e.source == Source::Damage && e.target == pain)
        .unwrap();
    for allele in [AlleleSide::Maternal, AlleleSide::Paternal] {
        genome.chemistry.graph = genome
            .chemistry
            .graph
            .clone()
            .with_emitter_expression_floor(allele, damage, 1.0)
            .unwrap();
    }
    genome.express().unwrap()
}
fn circuit(execution: NativeNeedExecutionV1) -> NativeNeedAssociationCircuitV2 {
    NativeNeedAssociationCircuitV2::compile_bound(
        &phenotype(2026111301 * 16 + 1),
        NativeNeedAssociationRecipeV2::phase3_species22(),
        execution,
    )
    .unwrap()
}
fn current() -> [f32; N] {
    [0.4, 0.35, 0.1, 0.1, 0.15, 0.5, 0.6, 0.1, 0.3, 0.2, 0.0]
}
fn learned(delta: [f32; N], confidence: f32) -> NativeNeedTransitionV1 {
    NativeNeedTransitionV1::learned(
        delta,
        NativeNeedConfidenceV1::new(confidence).unwrap(),
        NativeNeedSupportV1::OwnHistoryAssociation,
    )
    .unwrap()
}
fn reference(delta: [f32; N], confidence: f32) -> NativeNeedReferenceV1 {
    NativeNeedReferenceV1 {
        kind: NativeNeedReferenceKindV1::Rest,
        transition: learned(delta, confidence),
    }
}
fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 2e-5, "{a:?} != {b:?}");
}
// Independent scalar oracle is test-only. Production uses native operators.
fn verify(out: &NativeNeedAssociationOutputV2, recipe: NativeNeedAssociationRecipeV2) {
    let a = out.action.raw_delta().unwrap_or([0.; N]);
    let r = out.reference.transition.raw_delta().unwrap_or([0.; N]);
    let mut weighted = 0.;
    for i in 0..N {
        let a = (out.current[i] + a[i]).clamp(0., 1.);
        let r = (out.current[i] + r[i]).clamp(0., 1.);
        let pressure = |x: f32| match recipe.roles[i] {
            NativeNeedDriveRoleV2::Burden => x,
            NativeNeedDriveRoleV2::Availability => (recipe.atp_availability_target - x).max(0.),
            _ => 0.,
        };
        let burden = |x| {
            recipe
                .knots
                .into_iter()
                .zip(recipe.basis)
                .map(|(k, w)| w * (pressure(x) - k).max(0.).tanh())
                .sum::<f32>()
        };
        close(out.action_successor[i], a);
        close(out.reference_successor[i], r);
        close(out.action_pressure[i], pressure(a));
        close(out.reference_pressure[i], pressure(r));
        close(out.action_burden[i], burden(a));
        close(out.reference_burden[i], burden(r));
        weighted += (burden(r) - burden(a)) * recipe.magnitudes[i];
    }
    close(out.weighted_difference, weighted);
    let normalized = match recipe.normalization {
        NativeNeedNormalizationV1::Identity => weighted,
        NativeNeedNormalizationV1::PressureQuantumTanh { quantum } => (weighted / quantum).tanh(),
    };
    close(out.normalized_difference, normalized);
    let expected = (normalized * recipe.gain).clamp(-recipe.ceiling, recipe.ceiling)
        * out
            .action
            .confidence()
            .map_or(0., NativeNeedConfidenceV1::raw)
        * out
            .reference
            .transition
            .confidence()
            .map_or(0., NativeNeedConfidenceV1::raw)
        * support_coefficient(out.action)
        * support_coefficient(out.reference.transition);
    close(out.confidence_product_gated_readout, expected);
}

#[test]
fn native_need_v2_actual_phase3_binding_all_eleven_roles() {
    let p = phenotype(2026111301 * 16 + 1);
    let c = NativeNeedAssociationCircuitV2::compile_bound(
        &p,
        NativeNeedAssociationRecipeV2::phase3_species22(),
        NativeNeedExecutionV1::ActiveCoordinates,
    )
    .unwrap();
    let graph = &p.chemistry.biochemical;
    let channels = [
        Drive::Hunger,
        Drive::Fatigue,
        Drive::Fear,
        Drive::Pain,
        Drive::Loneliness,
        Drive::Curiosity,
        Drive::BrainAtp,
        Drive::TemperatureStress,
        Drive::Reproductive,
        Drive::Extension0,
        Drive::Extension1,
    ];
    for (i, channel) in channels.into_iter().enumerate() {
        let receptors = graph
            .receptors()
            .iter()
            .filter(|r| r.target == Target::Drive(channel))
            .collect::<Vec<_>>();
        if i == 10 {
            assert!(receptors.is_empty());
        } else {
            assert_eq!(receptors.len(), 1);
            let receptor = receptors[0];
            assert_eq!(receptor.source.0, if i == 9 { 22 } else { i as u16 + 1 });
            assert_eq!(
                (
                    receptor.gain,
                    receptor.threshold,
                    receptor.nominal,
                    receptor.digital
                ),
                (1., 0., 0., false)
            );
        }
        let expected_value = match i {
            0..=4 | 7 => -1. / 7.,
            6 => 1. / 7.,
            9 => -0.08,
            _ => 0.,
        };
        assert_eq!(graph.value_profile().drives[i], expected_value);
    }
    assert_eq!(c.recipe().roles, binding::PHASE3_ROLES);
    assert_eq!(graph.value_profile().hormones[9], 0.3);
    let praise = graph
        .receptors()
        .iter()
        .find(|r| r.target == Target::Endocrine(crate::EndocrineChannel::Extension0))
        .unwrap();
    assert_eq!(praise.source.0, 21);
    let state = BiochemistryState::new(&p, Tick(0)).unwrap();
    let bound = c.bind_current(&p, &state).unwrap();
    assert_eq!(bound.drives(), state.homeostasis.drives.to_array());
    assert_eq!(bound.source_genome_id(), p.source_genome_id);
    assert_eq!(bound.source_tick(), Tick(0));
    assert_eq!(bound.work().phenotype_hash_passes, 1);
    assert_eq!(bound.work().homeostatic_receptor_evaluations, 20);
    assert_eq!(bound.work().copied_drive_components, 11);
    assert!(bound.work().encoded_values > 11);
    println!(
        "need_v2_binding_work={:?} compile={:?}",
        bound.work(),
        c.compile_binding_work()
    );
    let output = c
        .run(
            &bound,
            NativeNeedTransitionV1::Unknown,
            reference([0.; N], 1.),
        )
        .unwrap();
    assert_eq!(output.binding_identity, c.binding_identity());
}

#[test]
fn native_need_v2_each_lane_relief_harm_satiation_and_atp_target() {
    let c = circuit(NativeNeedExecutionV1::ActiveCoordinates);
    for lane in 0..N {
        for sign in [-1., 1.] {
            let mut delta = [0.; N];
            delta[lane] = sign * 0.02;
            let out = c
                .run_native(current(), learned(delta, 1.), reference([0.; N], 1.))
                .unwrap();
            verify(&out, c.recipe());
            let value = out.confidence_product_gated_readout;
            match c.recipe().roles[lane] {
                NativeNeedDriveRoleV2::Burden => assert!(value * sign < 0., "lane={lane}"),
                NativeNeedDriveRoleV2::Availability => assert!(value * sign > 0.),
                _ => assert_eq!(value, 0.),
            }
        }
    }
    for lane in [0, 1, 2, 3, 4, 7, 9] {
        let mut satiated = current();
        satiated[lane] = 0.;
        let mut delta = [0.; N];
        delta[lane] = -0.4;
        let out = c
            .run_native(satiated, learned(delta, 1.), reference([0.; N], 1.))
            .unwrap();
        assert_eq!(out.confidence_product_gated_readout, 0.);
        assert!(out.action_successor_clipped[lane]);
    }
    let mut available = current();
    available[6] = 0.85;
    let mut delta = [0.; N];
    delta[6] = 0.1;
    assert_eq!(
        c.run_native(available, learned(delta, 1.), reference([0.; N], 1.))
            .unwrap()
            .confidence_product_gated_readout,
        0.
    );
    // Legitimate Rest relief is useful against an own NOOP reference; an
    // identical Rest source/reference remains zero, not a mandate to be active.
    let mut rest = [0.; N];
    rest[1] = -0.03;
    rest[3] = -0.02;
    let out = c
        .run_native(
            current(),
            learned(rest, 1.),
            NativeNeedReferenceV1 {
                kind: NativeNeedReferenceKindV1::Noop,
                transition: learned([0.; N], 1.),
            },
        )
        .unwrap();
    assert!(out.confidence_product_gated_readout > 0.);
    assert_eq!(
        c.run_native(current(), learned(rest, 1.), reference(rest, 1.))
            .unwrap()
            .confidence_product_gated_readout,
        0.
    );
}

#[test]
fn native_need_v2_common_mode_raw_vectors_and_full_vector_boredom() {
    let c = circuit(NativeNeedExecutionV1::ActiveCoordinates);
    let drift = [
        0.009, 0.006, -0.003, 0.005, 0.002, 0.001, -0.014, 0.003, 0.001, 0.0002, 0.,
    ];
    let zero = c
        .run_native(current(), learned(drift, 1.), reference(drift, 1.))
        .unwrap();
    assert_eq!(zero.confidence_product_gated_readout, 0.);
    let mut action = drift;
    action[9] -= 0.0075;
    let out = c
        .run_native(current(), learned(action, 1.), reference(drift, 1.))
        .unwrap();
    verify(&out, c.recipe());
    assert!(out.confidence_product_gated_readout > 0.);
    assert_eq!(out.action.raw_delta(), Some(action));
    assert_eq!(out.reference.transition.raw_delta(), Some(drift));
    for i in 0..N {
        if i != 9 {
            assert_eq!(out.signed_association[i], 0.);
        }
    }
    println!(
        "need_v2_full_vector_boredom weighted={} normalized={} native={}",
        out.weighted_difference, out.normalized_difference, out.confidence_product_gated_readout
    );
}

#[test]
fn native_need_v2_rejects_mutated_graph_roles_and_state_mismatch() {
    let p = phenotype(77);
    let recipe = NativeNeedAssociationRecipeV2::phase3_species22();
    let c = NativeNeedAssociationCircuitV2::compile_bound(
        &p,
        recipe,
        NativeNeedExecutionV1::ActiveCoordinates,
    )
    .unwrap();
    let original = serde_json::to_value(&p.chemistry.biochemical).unwrap();
    for (field, row, property, value) in [
        ("species", 21, "baseline", serde_json::json!(0.1)),
        ("species", 21, "decay_retention", serde_json::json!(0.9)),
        ("emitters", 0, "gain", serde_json::json!(0.04)),
    ] {
        let mut json = original.clone();
        json[field][row][property] = value;
        let mut changed = p.clone();
        changed.chemistry.biochemical = serde_json::from_value(json).unwrap();
        assert!(matches!(
            NativeNeedAssociationCircuitV2::compile_bound(
                &changed,
                recipe,
                NativeNeedExecutionV1::ActiveCoordinates
            ),
            Err(NativeNeedAssociationErrorV2::UnsupportedPhenotype(_))
        ));
    }
    let receptor_index = p
        .chemistry
        .biochemical
        .receptors()
        .iter()
        .position(|r| r.target == Target::Drive(Drive::Extension0))
        .unwrap();
    for (field, value) in [
        ("gain", serde_json::json!(-1.)),
        ("threshold", serde_json::json!(0.1)),
        ("source", serde_json::json!(21)),
    ] {
        let mut json = original.clone();
        json["receptors"][receptor_index][field] = value;
        let mut changed = p.clone();
        changed.chemistry.biochemical = serde_json::from_value(json).unwrap();
        assert!(NativeNeedAssociationCircuitV2::compile_bound(
            &changed,
            recipe,
            NativeNeedExecutionV1::ActiveCoordinates
        )
        .is_err());
    }
    let mut profile = p.chemistry.biochemical.value_profile();
    profile.drives[9] = 0.08;
    let mut changed = p.clone();
    changed.chemistry.biochemical = changed
        .chemistry
        .biochemical
        .with_value_profile(profile)
        .unwrap();
    assert!(NativeNeedAssociationCircuitV2::compile_bound(
        &changed,
        recipe,
        NativeNeedExecutionV1::ActiveCoordinates
    )
    .is_err());
    let mut bad_recipe = recipe;
    bad_recipe.roles[9] = NativeNeedDriveRoleV2::Regulatory;
    assert!(bad_recipe.validate().is_err());
    let mut bad_recipe = recipe;
    bad_recipe.magnitudes[10] = 0.08;
    assert!(bad_recipe.validate().is_err());
    let state = BiochemistryState::new(&p, Tick(0)).unwrap();
    let bound = c.bind_current(&p, &state).unwrap();
    let other = phenotype(78);
    assert!(c.bind_current(&other, &state).is_err());
    let other_circuit = NativeNeedAssociationCircuitV2::compile_bound(
        &other,
        recipe,
        NativeNeedExecutionV1::ActiveCoordinates,
    )
    .unwrap();
    assert!(other_circuit
        .run(
            &bound,
            NativeNeedTransitionV1::Unknown,
            reference([0.; N], 1.)
        )
        .is_err());
    // Disposable forged snapshot verifies rejection. Never a world/live edit.
    let mut forged = state;
    forged.homeostasis.drives.extension[0] = 0.5;
    assert!(matches!(
        c.bind_current(&p, &forged),
        Err(NativeNeedAssociationErrorV2::BindingMismatch)
    ));
    let mut changed_body = p.clone();
    changed_body.body.size_scale += 0.01;
    assert!(c.bind_current(&changed_body, &state).is_err());
    let c2 = NativeNeedAssociationCircuitV2::compile_bound(
        &changed_body,
        recipe,
        NativeNeedExecutionV1::ActiveCoordinates,
    )
    .unwrap();
    assert_ne!(c.binding_identity(), c2.binding_identity());
}

#[test]
fn native_need_v2_seven_axes_and_work_match_frozen_v1() {
    let p = phenotype(77);
    for normalization in [
        NativeNeedNormalizationV1::Identity,
        NativeNeedNormalizationV1::PressureQuantumTanh { quantum: 0.001 },
    ] {
        for execution in [
            NativeNeedExecutionV1::FullScratchReference,
            NativeNeedExecutionV1::ActiveCoordinates,
        ] {
            let mut old_recipe = NativeNeedAssociationRecipeV1::reference_species22();
            old_recipe.normalization = normalization;
            let old = NativeNeedAssociationCircuitV1::compile_with_execution(old_recipe, execution)
                .unwrap();
            let mut recipe = NativeNeedAssociationRecipeV2::phase3_species22();
            recipe.normalization = normalization;
            let new = NativeNeedAssociationCircuitV2::compile_bound(&p, recipe, execution).unwrap();
            assert_ne!(old.identity(), new.identity());
            for lane in [0, 1, 2, 3, 4, 6, 7] {
                for magnitude in [-0.03, 0.03] {
                    let mut delta = [0.; N];
                    delta[lane] = magnitude;
                    let a = old
                        .run(current(), learned(delta, 0.5), reference([0.; N], 0.25))
                        .unwrap();
                    let b = new
                        .run_native(current(), learned(delta, 0.5), reference([0.; N], 0.25))
                        .unwrap();
                    assert_eq!(a.action_successor, b.action_successor);
                    assert_eq!(a.reference_successor, b.reference_successor);
                    assert_eq!(a.signed_association, b.signed_association);
                    assert_eq!(a.weighted_difference, b.weighted_difference);
                    assert_eq!(a.normalized_difference, b.normalized_difference);
                    assert_eq!(
                        a.confidence_product_gated_readout,
                        b.confidence_product_gated_readout
                    );
                    assert_eq!(a.work, b.work);
                }
            }
            // V1 continues to suppress boredom; its old validator still forbids it.
            let mut boredom = [0.; N];
            boredom[9] = -0.02;
            assert_eq!(
                old.run(current(), learned(boredom, 1.), reference([0.; N], 1.))
                    .unwrap()
                    .confidence_product_gated_readout,
                0.
            );
            old_recipe.magnitudes[9] = 0.08;
            assert!(old_recipe.validate().is_err());
        }
    }
}

#[test]
fn native_need_v2_full_active_equivalence_gates_and_exact_work() {
    let p = phenotype(77);
    for gain in [0., 0.35] {
        for normalization in [
            NativeNeedNormalizationV1::Identity,
            NativeNeedNormalizationV1::PressureQuantumTanh { quantum: 0.001 },
        ] {
            let mut recipe = NativeNeedAssociationRecipeV2::phase3_species22();
            recipe.gain = gain;
            recipe.normalization = normalization;
            let full = NativeNeedAssociationCircuitV2::compile_bound(
                &p,
                recipe,
                NativeNeedExecutionV1::FullScratchReference,
            )
            .unwrap();
            let active = NativeNeedAssociationCircuitV2::compile_bound(
                &p,
                recipe,
                NativeNeedExecutionV1::ActiveCoordinates,
            )
            .unwrap();
            assert_eq!(full.recipe_identity(), active.recipe_identity());
            assert_ne!(full.identity(), active.identity());
            let mut delta = [0.; N];
            delta[9] = -0.01;
            let unsupported = NativeNeedTransitionV1::learned(
                delta,
                NativeNeedConfidenceV1::new(1.).unwrap(),
                NativeNeedSupportV1::Unsupported,
            )
            .unwrap();
            let transitions = [
                learned(delta, 1.),
                learned(delta, 0.5),
                learned(delta, 0.),
                NativeNeedTransitionV1::Unknown,
                unsupported,
            ];
            for action in transitions {
                for reference_transition in transitions {
                    let r = NativeNeedReferenceV1 {
                        kind: NativeNeedReferenceKindV1::Noop,
                        transition: reference_transition,
                    };
                    let a = full.run_native(current(), action, r).unwrap();
                    let b = active.run_native(current(), action, r).unwrap();
                    let mut aj = serde_json::to_value(&a).unwrap();
                    let mut bj = serde_json::to_value(&b).unwrap();
                    for key in ["execution", "circuit_identity", "work"] {
                        aj.as_object_mut().unwrap().remove(key);
                        bj.as_object_mut().unwrap().remove(key);
                    }
                    assert_eq!(aj, bj);
                    verify(&b, recipe);
                    assert_eq!(a.work.activation_cells, 5632);
                    assert_eq!(b.work.activation_cells, 85);
                    assert_eq!(b.work.projections, 10);
                    assert_eq!(b.work.active_tiles, 19);
                    assert_eq!(b.work.active_synapses, 128);
                    assert_eq!(b.work.finalizations, 11);
                    assert_eq!(b.work.dendritic.branches_evaluated, 88);
                    assert_eq!(b.work.dendritic.inputs_evaluated, 88);
                    assert_eq!(b.work.previous_activation_copies, 118);
                    assert_eq!(b.work.cleared_accumulator_cells, 85);
                    assert_eq!(b.work.stale_activation_clears, 33);
                    assert_eq!(b.work.stale_previous_activation_clears, 33);
                    assert_eq!(b.work.initialized_scratch_float_cells, 1536);
                    assert_eq!(b.work.input_activation_writes, 34);
                    assert_eq!(b.work.initialized_coordinate_cells, 34);
                    assert_eq!(b.work.coordinate_tracking_copies, 203);
                    assert_eq!(b.work.coordinate_membership_probes, 235);
                    assert_eq!(b.work.finalizer_coordinate_checks, 320);
                    let brain = b.work.brain_work_counters(2);
                    assert_eq!(brain.microsteps, 0);
                    assert_eq!(brain.neuron_updates, 173);
                    assert_eq!(brain.synapse_ops, 216);
                }
            }
        }
    }
    let on = circuit(NativeNeedExecutionV1::ActiveCoordinates);
    let mut recipe = on.recipe();
    recipe.gain = 0.;
    let off = NativeNeedAssociationCircuitV2::compile_bound(
        &phenotype(2026111301 * 16 + 1),
        recipe,
        on.execution(),
    )
    .unwrap();
    let mut delta = [0.; N];
    delta[9] = -0.01;
    let a = on
        .run_native(current(), learned(delta, 1.), reference([0.; N], 1.))
        .unwrap();
    let b = off
        .run_native(current(), learned(delta, 1.), reference([0.; N], 1.))
        .unwrap();
    assert!(a.confidence_product_gated_readout > 0.);
    assert_eq!(b.confidence_product_gated_readout, 0.);
    assert_eq!(a.work, b.work);
    let fractional = on
        .run_native(current(), learned(delta, 0.5), reference([0.; N], 0.25))
        .unwrap();
    close(
        fractional.confidence_product_gated_readout,
        a.confidence_product_gated_readout * 0.125,
    );
}

#[test]
fn native_need_v2_engineered_normalization_and_ceiling_remain_bounded() {
    let p = phenotype(77);
    let c = NativeNeedAssociationCircuitV2::compile_bound(
        &p,
        NativeNeedAssociationRecipeV2::phase3_species22(),
        NativeNeedExecutionV1::ActiveCoordinates,
    )
    .unwrap();
    let mut last = 0.;
    for relief in [0.001, 0.00375, 0.0075, 0.015, 0.15] {
        let mut delta = [0.; N];
        delta[9] = -relief;
        let out = c
            .run_native(current(), learned(delta, 1.), reference([0.; N], 1.))
            .unwrap();
        verify(&out, c.recipe());
        assert!(out.confidence_product_gated_readout > last);
        assert!(out.confidence_product_gated_readout <= 0.35);
        println!("need_v2_boredom relief={relief} weighted={} normalized={} readout={} measured_secant={} clipped={}", out.weighted_difference, out.normalized_difference, out.confidence_product_gated_readout, out.confidence_product_gated_readout / relief, out.readout_clipped);
        last = out.confidence_product_gated_readout;
    }
    let mut recipe = c.recipe();
    recipe.ceiling = 0.001;
    let capped = NativeNeedAssociationCircuitV2::compile_bound(&p, recipe, c.execution()).unwrap();
    let mut delta = [0.; N];
    delta[9] = -0.1;
    let out = capped
        .run_native(current(), learned(delta, 1.), reference([0.; N], 1.))
        .unwrap();
    assert!(out.readout_clipped);
    assert_eq!(out.confidence_product_gated_readout, 0.001);
    assert_ne!(c.recipe_identity(), capped.recipe_identity());
}

#[test]
fn native_need_v2_active_finalizer_validates_and_clears_stale_coordinates() {
    let mut full = CpuNeuralState::for_brain_class(&brain_spec()).unwrap();
    for i in 0..=CONSTANT {
        full.activations[i] = i as f32 / 40.0;
    }
    let mut active = full.clone();
    let mut finalizer = NativeNeedFinalizerV2::new(NativeNeedExecutionV1::ActiveCoordinates);
    let mut active_work = NativeNeedAssociationWorkV1::default();
    let mut full_work = NativeNeedAssociationWorkV1::default();
    for (targets, function, min, max) in [
        (vec![0, 3, 33], ActivationFunction::Identity, -1.0, 1.0),
        (vec![2, 8], ActivationFunction::Relu, 0.0, 1.0),
        (vec![8, 33, 511], ActivationFunction::Tanh, -1.0, 1.0),
        (vec![], ActivationFunction::Identity, -1.0, 1.0),
        (vec![], ActivationFunction::Relu, 0.0, 1.0),
        (vec![0, 511], ActivationFunction::Identity, 0.0, 0.0),
    ] {
        for (ordinal, &i) in targets.iter().enumerate() {
            let value = if ordinal % 2 == 0 { -1.5 } else { 1.5 };
            full.accumulators[i] = value;
            active.accumulators[i] = value;
        }
        finalize_full_scratch(&mut full, function, min, max, &mut full_work).unwrap();
        finalizer
            .finalize(&targets, &mut active, function, min, max, &mut active_work)
            .unwrap();
        assert_eq!(active, full);
        assert!(active.accumulators.iter().all(|&v| v == 0.0));
    }
    assert!(active_work.stale_activation_clears > 0);
    assert!(active_work.stale_previous_activation_clears > 0);
    for targets in [vec![0, 0], vec![3, 1], vec![512], vec![usize::MAX]] {
        let before = active.clone();
        let work_before = active_work.clone();
        assert!(finalizer
            .finalize(
                &targets,
                &mut active,
                ActivationFunction::Identity,
                -1.0,
                1.0,
                &mut active_work
            )
            .is_err());
        assert_eq!(active, before);
        assert_eq!(active_work, work_before);
    }
    for (function, min, max) in [
        (ActivationFunction::Logistic, 0.0, 1.0),
        (ActivationFunction::Identity, 0.1, 1.0),
        (ActivationFunction::Identity, -1.0, -0.1),
        (ActivationFunction::Identity, f32::NAN, 1.0),
    ] {
        assert!(finalizer
            .finalize(&[0], &mut active, function, min, max, &mut active_work)
            .is_err());
    }
    active.accumulators[0] = f32::INFINITY;
    assert!(finalizer
        .finalize(
            &[0],
            &mut active,
            ActivationFunction::Identity,
            -1.0,
            1.0,
            &mut active_work
        )
        .is_err());
    assert_eq!(active.accumulators[0], f32::INFINITY);
}

#[test]
fn native_need_v2_bounds_and_frozen_v1_identity() {
    // Pinned from the untouched V1 typed canonical encoding, not V2's encoder.
    let legacy = NativeNeedAssociationCircuitV1::compile(
        NativeNeedAssociationRecipeV1::reference_species22(),
    )
    .unwrap();
    assert_eq!(
        legacy.identity(),
        [
            9732021349082251931,
            2601300632533825627,
            12710188986800041625,
            12777652414493001824
        ]
    );
    let base = NativeNeedAssociationRecipeV2::phase3_species22();
    for invalid in [0.350001, 2048., f32::NAN] {
        let mut r = base;
        r.gain = invalid;
        assert!(r.validate().is_err());
        let mut r = base;
        r.ceiling = invalid;
        assert!(r.validate().is_err());
    }
    for invalid in [0., 0.0009, 1.1, f32::NAN] {
        let mut r = base;
        r.normalization = NativeNeedNormalizationV1::PressureQuantumTanh { quantum: invalid };
        assert!(r.validate().is_err());
    }
    let mut r = base;
    r.atp_availability_target = 0.8;
    assert!(r.validate().is_err());
    let mut r = base;
    r.magnitudes[9] = 0.;
    assert!(r.validate().is_err());
    let mut r = base;
    r.magnitudes[5] = 0.08;
    assert!(r.validate().is_err());
    let mut r = base;
    r.knots[1] = 0.3;
    assert!(r.validate().is_err());
    let c = circuit(NativeNeedExecutionV1::ActiveCoordinates);
    let mut state = current();
    state[0] = f32::NAN;
    assert!(c
        .run_native(
            state,
            NativeNeedTransitionV1::Unknown,
            reference([0.; N], 1.)
        )
        .is_err());
}
