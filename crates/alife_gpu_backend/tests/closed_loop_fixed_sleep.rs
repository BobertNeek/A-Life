//! Disabled-receptor compilation and sleep shader validation.
use alife_core::{
    BrainCapacityClass, BrainGenome, BrainPhenotype, CompiledSynapseKind, DevelopmentState,
    NormalizedScalar, PhenotypeCompiler, ProjectionKey, ProjectionPlasticityMask, SensorProfile,
    Tick,
};

const ABI: &str = include_str!("../shaders/closed_loop_abi.wgsl");
const CONSOLIDATE: &str = include_str!("../shaders/closed_loop_consolidate.wgsl");
const REPLAY: &str = include_str!("../shaders/closed_loop_replay_learning.wgsl");

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
