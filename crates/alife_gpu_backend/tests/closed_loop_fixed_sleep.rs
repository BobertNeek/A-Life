//! Disabled-receptor encoding and consolidation arithmetic checks.
use alife_core::{
    BrainCapacityClass, BrainGenome, BrainPhenotype, CompiledSynapseKind, DevelopmentState,
    NormalizedScalar, PhenotypeCompiler, PlasticityReceptorPlan, PlasticityReceptorProfile,
    ProjectionKey, ProjectionPlasticityMask, SensorProfile, Tick,
};

const ABI: &str = include_str!("../shaders/closed_loop_abi.wgsl");
const CONSOLIDATE: &str = include_str!("../shaders/closed_loop_consolidate.wgsl");
const REPLAY: &str = include_str!("../shaders/closed_loop_replay_learning.wgsl");

fn receptor(rates: [f32; 3]) -> PlasticityReceptorPlan {
    PlasticityReceptorPlan::try_new(
        0.95,
        rates[0],
        rates[1],
        rates[2],
        PlasticityReceptorProfile::try_new([0.0; 8]).unwrap(),
        -2.0,
        2.0,
    )
    .unwrap()
}

// Arithmetic reference for the shader's disabled-receptor behavior.
fn staged_weights(receptor: PlasticityReceptorPlan, lifetime: f32, fast: f32) -> (f32, f32) {
    if receptor.is_delta_enabled() {
        ((lifetime + 0.5 * fast).clamp(-4.0, 4.0), fast * 0.5)
    } else {
        (lifetime, fast)
    }
}

fn no_replay_fast(receptor: PlasticityReceptorPlan, fast: f32) -> f32 {
    if receptor.is_delta_enabled() {
        fast * 0.5
    } else {
        fast
    }
}

fn compile(genome: &BrainGenome, capacity: &BrainCapacityClass) -> BrainPhenotype {
    let state = DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
    PhenotypeCompiler::compile(
        genome,
        capacity,
        &state,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap()
}

#[test]
fn sleep_shaders_validate_without_hardware() {
    for source in [CONSOLIDATE, REPLAY] {
        let module = naga::front::wgsl::parse_str(&format!("{ABI}\n{source}")).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap();
    }
}

#[test]
fn sleep_source_uses_immutable_delta_semantics_before_weight_arithmetic() {
    let source: String = CONSOLIDATE.split_whitespace().collect();
    let helper = source
        .split("fnconsolidate_synapse_delta_mode")
        .nth(1)
        .unwrap()
        .split("fnsleep_transaction_valid")
        .next()
        .unwrap();
    assert!(helper.contains("load_synapse_learning_metadata(metadata_base)"));
    assert!(helper.contains("load_plasticity_receptor(receptor_base)"));
    assert!(helper.contains("receptor.learning_rate!=0.0||receptor.sleep_replay_rate!=0.0||receptor.normalization_rate!=0.0"));
    assert!(!helper.contains("developmental_plasticity_rate"));
    assert!(!helper.contains("alpha"));
    for check in [
        "consolidate_immutable_plan_span_within(metadata_base,8u)",
        "consolidate_immutable_plan_span_within(receptor_base,16u)",
        "metadata.global_synapse_id!=synapse",
        "consolidate_finite(receptor.learning_rate)",
        "consolidate_finite(receptor.sleep_replay_rate)",
        "consolidate_finite(receptor.normalization_rate)",
    ] {
        assert!(helper.contains(check), "missing {check}");
    }
    let entry = source
        .split("fnconsolidate_fast_weights")
        .nth(1)
        .unwrap()
        .split("fnfinalize_sleep_staging")
        .next()
        .unwrap();
    let disabled_return = entry.find("if(delta_mode==0u){return;}").unwrap();
    assert!(disabled_return < entry.find("letpromoted=").unwrap());
    assert!(disabled_return < entry.find("store_state_f32(inactive.lifetime").unwrap());
    assert!(
        entry.contains("if(delta_mode==2u){consolidate_reject(header.completion_offset);return;}")
    );
    assert!(source.contains("letno_replay_fast=select(active_fast*(1.0-parameters.fast_decay_rate),active_fast,delta_mode==0u);"));
    // Enabled receptors use ordinary consolidation arithmetic.
    assert!(entry.contains("letpromoted=parameters.staging_rate*replayed_fast;"));
    assert!(entry.contains("letnext_lifetime=clamp(active_lifetime+promoted,-parameters.weight_limit,parameters.weight_limit);"));
    assert!(entry.contains("letnext_fast=replayed_fast*(1.0-parameters.fast_decay_rate);"));
}

#[test]
fn disabled_receptors_preserve_finite_coefficients_and_effective_weight() {
    let disabled = receptor([0.0; 3]);
    for (lifetime, fast) in [(0.5_f32, 0.3_f32), (-0.5, -0.3), (4.5, 1.5), (0.0, 0.0)] {
        let next = staged_weights(disabled, lifetime, fast);
        assert_eq!(next.0.to_bits(), lifetime.to_bits());
        assert_eq!(next.1.to_bits(), fast.to_bits());
        let (genetic, alpha) = (0.125, 0.75);
        assert_eq!(
            genetic + next.0 + alpha * next.1,
            genetic + lifetime + alpha * fast
        );
    }
}

#[test]
fn disabled_sleep_receipt_has_no_false_replay_or_promotion() {
    let disabled = receptor([0.0; 3]);
    let (lifetime, fast) = (0.5, 0.3);
    let staged = staged_weights(disabled, lifetime, fast);
    assert_eq!((staged.0 - lifetime).abs(), 0.0);
    assert_eq!((staged.1 - no_replay_fast(disabled, fast)).abs(), 0.0);
    assert!(
        (staged.1 - fast * 0.5).abs() > 0.0,
        "old baseline falsely reports replay"
    );
}

#[test]
fn any_enabled_rate_keeps_sleep_plastic_even_without_modulator_signal() {
    for rates in [[0.01, 0.0, 0.0], [0.0, 0.01, 0.0], [0.0, 0.0, 0.01]] {
        let enabled = receptor(rates); // Zero receptor weights are not a fixed identity.
        assert!(enabled.is_delta_enabled());
        let staged = staged_weights(enabled, 0.5, 0.3);
        assert!((staged.0 - 0.65).abs() < 1e-6);
        assert!((staged.1 - 0.15).abs() < 1e-6);
        assert_eq!(staged.1, no_replay_fast(enabled, 0.3));
    }
}

#[test]
fn promoted_defaults_preserve_enabled_arithmetic_and_zero_acquired_fixed_state() {
    for capacity in [
        BrainCapacityClass::n512(),
        BrainCapacityClass::n1024(),
        BrainCapacityClass::n2048(),
    ] {
        let genome = BrainGenome::scaffold(0xF1_2026, capacity.id());
        let phenotype = compile(&genome, &capacity);
        let mut enabled_count = 0;
        let mut fixed_count = 0;
        for synapse in phenotype.synapses() {
            let plan = phenotype.plasticity_receptors()[usize::from(synapse.receptor_index())];
            if plan.is_delta_enabled() {
                enabled_count += 1;
                let old = ((0.5_f32 + 0.5 * 0.3).clamp(-4.0, 4.0), 0.3 * 0.5);
                assert_eq!(staged_weights(plan, 0.5, 0.3), old);
            } else {
                fixed_count += 1;
                // Newly born fixed rows have no acquired coefficients to transfer.
                assert_eq!(staged_weights(plan, 0.0, 0.0), (0.0, 0.0));
            }
        }
        assert!(enabled_count > 0);
        println!(
            "{:?}: {enabled_count} enabled, {fixed_count} disabled synapses",
            capacity.id()
        );
        if capacity.id() == BrainCapacityClass::n2048().id() {
            assert_eq!(fixed_count, 0);
        }
    }
}

#[test]
fn inherited_zero_route_is_fixed_while_association_readouts_remain_plastic() {
    for capacity in [
        BrainCapacityClass::n512(),
        BrainCapacityClass::n1024(),
        BrainCapacityClass::n2048(),
    ] {
        let mut genome = BrainGenome::scaffold(0xF1_2026, capacity.id());
        let initial = compile(&genome, &capacity);
        let route = initial
            .projections()
            .iter()
            .find(|route| {
                let (start, len) = route.synapse_range();
                initial.synapses()[start as usize..(start + len) as usize]
                    .iter()
                    .all(|synapse| matches!(synapse.kind(), CompiledSynapseKind::Recurrent))
            })
            .unwrap();
        let key = ProjectionKey::new(route.source_lobe(), route.target_lobe());
        genome
            .plasticity_mask
            .projection_masks
            .retain(|mask| mask.projection != key);
        genome
            .plasticity_mask
            .projection_masks
            .push(ProjectionPlasticityMask {
                projection: key,
                learning_rate_scale: NormalizedScalar::ZERO,
                plasticity_enabled: true,
            });
        let phenotype = compile(&genome, &capacity);
        let fixed_route = phenotype
            .projections()
            .iter()
            .find(|route| {
                route.source_lobe() == key.source_lobe && route.target_lobe() == key.target_lobe
            })
            .unwrap();
        let (start, len) = fixed_route.synapse_range();
        assert!(len > 0);
        for synapse in &phenotype.synapses()[start as usize..(start + len) as usize] {
            assert!(
                !phenotype.plasticity_receptors()[usize::from(synapse.receptor_index())]
                    .is_delta_enabled()
            );
        }
        assert!(phenotype
            .replay_capture_plan()
            .global_synapse_ids()
            .iter()
            .all(|id| *id < start || *id >= start + len));
        assert!(phenotype
            .synapses()
            .iter()
            .filter(|synapse| matches!(synapse.kind(), CompiledSynapseKind::Decoder(_)))
            .all(|synapse| phenotype.plasticity_receptors()
                [usize::from(synapse.receptor_index())]
            .is_delta_enabled()));
    }
}
