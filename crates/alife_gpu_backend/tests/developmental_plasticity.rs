//! CPU coverage of transient current-age plasticity transport and its shader consumers.
//! Compiler output is the independent oracle; these tests do not execute GPU learning.
use alife_core::{
    BrainCapacityClass, BrainGenome, BrainPhenotype, CompiledSynapseKind, CriticalPeriod,
    DevelopmentState, LobeKind, NormalizedScalar, PhenotypeCompiler, ProjectionKey,
    ProjectionPlasticityMask, SensorProfile, Tick,
};
use alife_gpu_backend::{
    GpuDevelopmentalPlasticity, CLOSED_LOOP_PLASTICITY_WGSL, CLOSED_LOOP_REPLAY_LEARNING_WGSL,
};

fn fixture() -> (BrainGenome, DevelopmentState, BrainPhenotype) {
    let genome = BrainGenome::scaffold(0xD3_2026, BrainCapacityClass::n512().id());
    let state = DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
    let phenotype = compile(&genome, &state);
    (genome, state, phenotype)
}

fn compile(genome: &BrainGenome, state: &DevelopmentState) -> BrainPhenotype {
    PhenotypeCompiler::compile(
        genome,
        &BrainCapacityClass::n512(),
        state,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap()
}

fn period(bias: f32) -> CriticalPeriod {
    CriticalPeriod {
        lobe: LobeKind::TemporalPredictive,
        opens_at: Tick(10),
        closes_at: Tick(20),
        plasticity_bias: NormalizedScalar::new(bias).unwrap(),
    }
}

#[test]
fn transported_rates_match_existing_compiler_for_endpoints_overlap_fixed_and_caps() {
    for capacity in [
        BrainCapacityClass::n512(),
        BrainCapacityClass::n1024(),
        BrainCapacityClass::n2048(),
    ] {
        assert_compiler_oracle(capacity);
    }
}

fn assert_compiler_oracle(capacity: BrainCapacityClass) {
    let mut genome = BrainGenome::scaffold(0xD3_2026, capacity.id());
    let mut state =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
    let topology = PhenotypeCompiler::compile(
        &genome,
        &capacity,
        &state,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let targeted_routes: Vec<_> = topology
        .projections()
        .iter()
        .filter(|route| {
            let (start, len) = route.synapse_range();
            (route.source_lobe() == LobeKind::TemporalPredictive
                || route.target_lobe() == LobeKind::TemporalPredictive)
                && topology.synapses()[start as usize..(start + len) as usize]
                    .iter()
                    .any(|synapse| matches!(synapse.kind(), CompiledSynapseKind::Recurrent))
        })
        .take(2)
        .map(|route| ProjectionKey::new(route.source_lobe(), route.target_lobe()))
        .collect();
    assert_eq!(
        targeted_routes.len(),
        2,
        "fixture needs distinct targeted recurrent routes"
    );
    // Explicit inherited zero mask creates genuinely fixed synapses. Keep a
    // second target route at 0.5 for unsaturated boosts in every capacity.
    // Enabled zero is deliberate: N2048 treats an absent/disabled mask as its
    // default section scale, whereas an enabled zero is an actual zero scale.
    for (projection, scale) in targeted_routes.into_iter().zip([0.0, 0.5]) {
        genome
            .plasticity_mask
            .projection_masks
            .retain(|mask| mask.projection != projection);
        genome
            .plasticity_mask
            .projection_masks
            .push(ProjectionPlasticityMask {
                projection,
                learning_rate_scale: NormalizedScalar::new(scale).unwrap(),
                plasticity_enabled: true,
            });
    }
    let stable = PhenotypeCompiler::compile(
        &genome,
        &capacity,
        &state,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    state.age_ticks = Tick(15);
    state.open_critical_periods = vec![period(0.2), period(0.5)];
    let oracle = PhenotypeCompiler::compile(
        &genome,
        &capacity,
        &state,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let mut transport = GpuDevelopmentalPlasticity::default();
    assert!(transport
        .update(&stable, &state, genome.plasticity_parameters())
        .unwrap());
    let words = transport.words();
    assert_eq!(words[0] as usize, stable.projections().len());
    let mut source_match = false;
    let mut target_match = false;
    let mut unrelated = false;
    let mut boosted = 0;
    let mut fixed = 0;
    let mut capped = 0;
    let mut capped_decoder = 0;
    let mut boosted_recurrent = 0;
    for route in stable.projections() {
        let matches = state
            .open_critical_periods
            .iter()
            .any(|period| period.lobe == route.source_lobe() || period.lobe == route.target_lobe());
        source_match |= route.source_lobe() == LobeKind::TemporalPredictive;
        target_match |= route.target_lobe() == LobeKind::TemporalPredictive;
        unrelated |= !matches;
        let multiplier = f32::from_bits(words[4 + usize::from(route.route_index())]);
        assert_eq!(multiplier, if matches { 1.5 } else { 1.0 });
        let (start, len) = route.synapse_range();
        for index in start as usize..(start + len) as usize {
            let base = stable.plasticity_receptors()
                [usize::from(stable.synapses()[index].receptor_index())];
            let expected = oracle.plasticity_receptors()
                [usize::from(oracle.synapses()[index].receptor_index())];
            let rates = [
                base.learning_rate(),
                base.sleep_replay_rate(),
                base.normalization_rate(),
            ];
            let expected_rates = [
                expected.learning_rate(),
                expected.sleep_replay_rate(),
                expected.normalization_rate(),
            ];
            if matches && rates == [0.0; 3] {
                fixed += 1;
                assert_eq!(
                    expected_rates, [0.0; 3],
                    "a targeted fixed synapse must stay fixed"
                );
            }
            let kind = stable.synapses()[index].kind();
            if matches!(kind, CompiledSynapseKind::Decoder(_)) {
                assert_eq!(expected_rates, rates, "decoder rates are already capped");
                if base.learning_rate() == f32::from_bits(words[1]) && base.learning_rate() > 0.0 {
                    capped_decoder += 1;
                }
            } else if expected.learning_rate() > base.learning_rate() {
                boosted_recurrent += 1;
            }
            for lane in 0..3 {
                let cap = f32::from_bits(words[1 + lane]);
                let actual = (rates[lane] * multiplier).min(cap);
                assert!(
                    (actual - expected_rates[lane]).abs() < 1e-7,
                    "route {} synapse {index} lane {lane}: {actual} != {}",
                    route.route_index(),
                    expected_rates[lane]
                );
                if rates[lane] == 0.0 {
                    assert_eq!(actual, 0.0);
                }
                if rates[lane] == cap && cap > 0.0 {
                    capped += 1;
                    assert_eq!(actual, cap);
                }
                if actual > rates[lane] {
                    boosted += 1;
                }
            }
        }
    }
    assert!(source_match && target_match && unrelated);
    assert!(
        boosted > 0 && boosted_recurrent > 0,
        "{:?}: no boosted recurrent route",
        capacity.id()
    );
    assert!(fixed > 0, "{:?}: no targeted fixed synapse", capacity.id());
    assert!(
        capped > 0 && capped_decoder > 0,
        "{:?}: no capped decoder",
        capacity.id()
    );
}

#[test]
fn current_age_cache_closes_for_sleep_and_reconstructs_with_stable_compiler_identity() {
    let (genome, mut state, stable) = fixture();
    let genome_before = genome.clone();
    let phenotype_before = stable.clone();
    let mut transport = GpuDevelopmentalPlasticity::default();
    transport
        .update(&stable, &state, genome.plasticity_parameters())
        .unwrap();
    let closed = transport.words().to_vec();
    state.age_ticks = Tick(10);
    state.open_critical_periods = vec![period(0.5)];
    assert!(transport
        .update(&stable, &state, genome.plasticity_parameters())
        .unwrap());
    assert_ne!(transport.words(), closed);
    state.age_ticks = Tick(20);
    assert!(!transport
        .update(&stable, &state, genome.plasticity_parameters())
        .unwrap());
    state.age_ticks = Tick(21);
    state.open_critical_periods.clear();
    state.last_sleep_tick = Some(Tick(21));
    state.sleep_cycle_count = 1;
    assert!(transport
        .update(&stable, &state, genome.plasticity_parameters())
        .unwrap());
    assert_eq!(transport.words(), closed);
    // A fresh resident derives the same current-age metadata. No experience timestamp
    // is accepted by this transport, so old replay events cannot reopen the period.
    let mut restored = GpuDevelopmentalPlasticity::default();
    restored
        .update(&stable, &state, genome.plasticity_parameters())
        .unwrap();
    assert_eq!(restored.words(), transport.words());
    assert_eq!(stable, phenotype_before);
    assert_eq!(genome, genome_before);
}

fn calls(block: &naga::Block, output: &mut Vec<naga::Handle<naga::Function>>) {
    use naga::Statement;
    for statement in block.iter() {
        match statement {
            Statement::Call { function, .. } => {
                if !output.contains(function) {
                    output.push(*function);
                }
            }
            Statement::Block(body) => calls(body, output),
            Statement::If { accept, reject, .. } => {
                calls(accept, output);
                calls(reject, output);
            }
            Statement::Switch { cases, .. } => {
                for case in cases {
                    calls(&case.body, output);
                }
            }
            Statement::Loop {
                body, continuing, ..
            } => {
                calls(body, output);
                calls(continuing, output);
            }
            _ => {}
        }
    }
}

#[test]
fn validated_waking_and_sleep_entrypoints_reach_the_developmental_rate_helper() {
    for source in [
        CLOSED_LOOP_PLASTICITY_WGSL,
        CLOSED_LOOP_REPLAY_LEARNING_WGSL,
    ] {
        let compact: String = source.split_whitespace().collect();
        assert!(
            compact.contains(
                "letroute=immutable_plan_words[brain.route_indices_offset+local_synapse];"
            ),
            "route indices belong to immutable plan storage, never mutable learned-state storage"
        );
        assert!(!compact.contains("load_state_u32(brain.route_indices_offset+local_synapse)"));
        let module = naga::front::wgsl::parse_str(source).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap();
        let helper = module
            .functions
            .iter()
            .find(|(_, f)| f.name.as_deref() == Some("developmental_plasticity_rate"))
            .expect("production helper")
            .0;
        assert!(
            module.entry_points.iter().any(|entry| {
                let mut reachable = Vec::new();
                calls(&entry.function.body, &mut reachable);
                let mut cursor = 0;
                while cursor < reachable.len() {
                    calls(&module.functions[reachable[cursor]].body, &mut reachable);
                    cursor += 1;
                }
                reachable.contains(&helper)
            }),
            "helper must be reachable from a production entrypoint"
        );
    }
}
