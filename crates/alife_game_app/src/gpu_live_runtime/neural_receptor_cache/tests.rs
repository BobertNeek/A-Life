use super::*;
use crate::gpu_live_runtime::{
    compile_gpu_birth_components, compile_gpu_components_from_genome, ResidentAuthorityPlan,
};
use alife_core::{
    BiochemistryState, BrainCapacityClass, BrainGenome, BrainScaleTier, DevelopmentState,
    NeuralReceptorActivation, NeuralReceptorClass, NormalizedScalar, OrganismId, SensorProfile,
    Tick, BIOCHEMICAL_GRAPH_SCHEMA_VERSION,
};
use std::{hint::black_box, time::Instant};

fn phenotype(tier: BrainScaleTier, seed: u64, tick: Tick) -> BrainPhenotype {
    compile_gpu_birth_components(
        seed,
        tier,
        OrganismId(1),
        tick,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap()
    .0
}

fn frame(tick: u64) -> NeuralReceptorFrame {
    use NeuralReceptorClass::*;
    NeuralReceptorFrame {
        schema_version: BIOCHEMICAL_GRAPH_SCHEMA_VERSION,
        source_chemistry_version: BIOCHEMICAL_GRAPH_SCHEMA_VERSION,
        source_tick: Tick(tick),
        activations: [
            InteroceptiveInput,
            RegionalExcitability,
            ProjectionGain,
            LocalThreshold,
            AttentionGate,
            PlasticityAppetitive,
            PlasticityAversive,
            StructuralGrowth,
            Sleep,
            Consolidation,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, class)| NeuralReceptorActivation {
            class,
            signal: ((tick + index as u64) % 11) as f32 / 10.0,
        })
        .collect(),
    }
}

#[test]
fn cached_expression_matches_full_compile_for_promoted_brains_and_fresh_chemistry() {
    for tier in [
        BrainScaleTier::Nano512,
        BrainScaleTier::Small1024,
        BrainScaleTier::Standard2048,
    ] {
        let phenotype = phenotype(tier, 77_112, Tick::ZERO);
        let cached = ResidentNeuralReceptors::compile(&phenotype).unwrap();
        let mut previous = None;
        for tick in [0, 1, 7, 10, 1_000] {
            let frame = frame(tick);
            let expected = NeuralReceptorEffects::from_frame(
                &frame,
                &NeuralReceptorPhenotype::compile(&phenotype).unwrap(),
            )
            .unwrap();
            let actual = cached.effects_for(&phenotype, &frame).unwrap();
            assert_eq!(actual, expected);
            assert_eq!(actual.source_tick, Tick(tick));
            if let Some(previous) = previous {
                assert_ne!(actual, previous);
            }
            previous = Some(actual);
        }
    }
}

#[test]
fn cache_rejects_another_valid_phenotype_and_is_rebuilt_on_replacement() {
    let first = phenotype(BrainScaleTier::Small1024, 77_112, Tick::ZERO);
    let second = phenotype(BrainScaleTier::Small1024, 88_112, Tick::ZERO);
    assert_ne!(first.phenotype_hash(), second.phenotype_hash());
    let cached = ResidentNeuralReceptors::compile(&first).unwrap();
    assert_eq!(
        cached.effects_for(&second, &frame(12)),
        Err(ScaffoldContractError::PhenotypeCompile)
    );
    assert_eq!(
        ResidentNeuralReceptors::compile(&second)
            .unwrap()
            .effects_for(&second, &frame(12))
            .unwrap(),
        NeuralReceptorEffects::from_frame(
            &frame(12),
            &NeuralReceptorPhenotype::compile(&second).unwrap()
        )
        .unwrap()
    );
}

#[test]
fn restored_phenotype_rebuilds_expression_and_rejects_corrupt_source() {
    let original = phenotype(BrainScaleTier::Standard2048, 77_112, Tick::ZERO);
    let bytes = serde_json::to_vec(&original).unwrap();
    let restored: BrainPhenotype = serde_json::from_slice(&bytes).unwrap();
    let expected = ResidentNeuralReceptors::compile(&original)
        .unwrap()
        .effects_for(&original, &frame(1_001))
        .unwrap();
    assert_eq!(
        ResidentNeuralReceptors::compile(&restored)
            .unwrap()
            .effects_for(&restored, &frame(1_001))
            .unwrap(),
        expected
    );
    let mut corrupt: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    corrupt["phenotype_hash"][0] = serde_json::json!(0);
    assert!(serde_json::from_value::<BrainPhenotype>(corrupt).is_err());
}

#[test]
fn cache_keeps_dynamic_chemistry_validation() {
    let phenotype = phenotype(BrainScaleTier::Nano512, 77_112, Tick::ZERO);
    let cached = ResidentNeuralReceptors::compile(&phenotype).unwrap();
    let mut invalid = frame(1);
    invalid.activations[0].signal = f32::NAN;
    assert!(cached.effects_for(&phenotype, &invalid).is_err());
    let mut invalid = frame(2);
    invalid.activations[1].class = invalid.activations[0].class;
    assert!(cached.effects_for(&phenotype, &invalid).is_err());
    let mut invalid = frame(3);
    invalid.source_chemistry_version += 1;
    assert!(cached.effects_for(&phenotype, &invalid).is_err());
}

#[test]
fn fresh_resident_compiles_cache_and_does_not_serialize_it() {
    let capacity = BrainCapacityClass::n512();
    let genome =
        BrainGenome::scaffold(alife_core::LEGACY_NANO512_V1_COORDINATE_SEED, capacity.id());
    let development = DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar(0.35));
    let (phenotype, compiler_inputs) = compile_gpu_components_from_genome(
        genome.clone(),
        development.clone(),
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let biology = alife_core::CreatureGenome::early_mammal_founder(
        77_112,
        alife_core::FoundationGeneticIdentity::new(
            alife_core::FoundationId::N512_V1.raw(),
            alife_core::FoundationVersion::V1.raw() as u16,
            alife_core::FoundationCompatibilityFamilyId::N512_FOUNDATION.raw(),
            capacity.id(),
        )
        .unwrap(),
    )
    .unwrap()
    .express()
    .unwrap();
    let resident = ResidentAuthorityPlan {
        organism_id: OrganismId(1),
        world_tick: Tick::ZERO,
        phenotype,
        compiler_inputs,
        legacy_nano512_compatibility_receipt: None,
        genome,
        development,
        biochemistry: BiochemistryState::new(&biology, Tick::ZERO).unwrap(),
    }
    .into_fresh_resident()
    .unwrap();
    assert_eq!(
        resident
            .neural_receptors
            .effects_for(&resident.phenotype, &frame(7))
            .unwrap(),
        NeuralReceptorEffects::from_frame(
            &frame(7),
            &NeuralReceptorPhenotype::compile(&resident.phenotype).unwrap()
        )
        .unwrap()
    );
    let json = serde_json::to_value(&resident).unwrap();
    assert!(json.get("neural_receptors").is_none());
    let clone = resident.clone();
    assert_eq!(serde_json::to_value(&clone).unwrap(), json);
    assert_eq!(
        clone
            .neural_receptors
            .effects_for(&clone.phenotype, &frame(8))
            .unwrap(),
        resident
            .neural_receptors
            .effects_for(&resident.phenotype, &frame(8))
            .unwrap()
    );
}

#[test]
fn developmental_time_does_not_change_the_immutable_expression_source() {
    for tier in [BrainScaleTier::Small1024, BrainScaleTier::Standard2048] {
        let newborn = phenotype(tier, 77_112, Tick::ZERO);
        let later = phenotype(tier, 77_112, Tick(100_000));
        assert_eq!(newborn, later);
        assert_eq!(
            ResidentNeuralReceptors::compile(&newborn)
                .unwrap()
                .effects_for(&later, &frame(100_000))
                .unwrap(),
            NeuralReceptorEffects::from_frame(
                &frame(100_000),
                &NeuralReceptorPhenotype::compile(&later).unwrap()
            )
            .unwrap()
        );
    }
}

#[test]
#[ignore = "CPU-only timing receipt; run explicitly without concurrent builds"]
fn receptor_expression_cpu_timing_evidence() {
    let source = phenotype(BrainScaleTier::Standard2048, 77_112, Tick::ZERO);
    assert_eq!(source.neuron_count(), 2_048);
    assert_eq!(source.synapses().len(), 32_768);
    let frames = (0..20).map(frame).collect::<Vec<_>>();
    for population in [8, 50] {
        let phenotypes = (0..population).map(|_| source.clone()).collect::<Vec<_>>();
        let caches = phenotypes
            .iter()
            .map(|phenotype| ResidentNeuralReceptors::compile(phenotype).unwrap())
            .collect::<Vec<_>>();
        let run = |cached: bool| {
            let mut outputs = Vec::with_capacity(population * frames.len());
            let started = Instant::now();
            for frame in &frames {
                for (phenotype, cache) in phenotypes.iter().zip(&caches) {
                    let phenotype = black_box(phenotype);
                    let frame = black_box(frame);
                    outputs.push(if cached {
                        black_box(cache).effects_for(phenotype, frame).unwrap()
                    } else {
                        NeuralReceptorEffects::from_frame(
                            frame,
                            &NeuralReceptorPhenotype::compile(phenotype).unwrap(),
                        )
                        .unwrap()
                    });
                }
            }
            (started.elapsed().as_nanos(), outputs)
        };
        let (baseline, _) = run(false);
        let (cached, _) = run(true);
        black_box((baseline, cached));
        let mut samples = Vec::new();
        for sample in 0..9 {
            let (baseline, cached) = if sample % 2 == 0 {
                (run(false), run(true))
            } else {
                let cached = run(true);
                (run(false), cached)
            };
            assert_eq!(baseline.1, cached.1);
            samples.push(serde_json::json!({
                "baseline_ns": baseline.0,
                "cached_ns": cached.0,
                "baseline_first": sample % 2 == 0,
                "exact_effect_rows": baseline.1.len(),
            }));
        }
        println!(
            "RECEPTOR_CPU_RECEIPT {}",
            serde_json::json!({
                "population": population,
                "neuron_count": source.neuron_count(),
                "synapse_count": source.synapses().len(),
                "ticks_per_sample": frames.len(),
                "debug_assertions": cfg!(debug_assertions),
                "samples": samples,
                "scope": "receptor expression compilation/source binding and fresh-frame projection only; excludes admission, world, cognition preparation, GPU, persistence and rendering",
            })
        );
    }
}
