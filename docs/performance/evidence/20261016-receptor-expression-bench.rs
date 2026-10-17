//! CPU component benchmark. Compile with the optimized alife_core rlib and -O.
//! It imports the production resident cache source directly; no device is made.
#[path = "../../../crates/alife_game_app/src/gpu_live_runtime/neural_receptor_cache.rs"]
mod neural_receptor_cache;
use alife_core::{
    BrainCapacityClass, BrainGenome, DevelopmentState, FoundationWeightAsset,
    NeuralReceptorActivation, NeuralReceptorClass, NeuralReceptorEffects, NeuralReceptorFrame,
    NeuralReceptorPhenotype, NormalizedScalar, PhenotypeCompiler, SensorProfile, Tick,
    BIOCHEMICAL_GRAPH_SCHEMA_VERSION,
};
use neural_receptor_cache::ResidentNeuralReceptors;
use std::{hint::black_box, time::Instant};
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

fn main() {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(77_112, capacity.id());
    let development = DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar(1.0));
    let profile = SensorProfile::GroundedObjectSlotsV1;
    let foundation = FoundationWeightAsset::builtin_n2048_v1(profile).unwrap();
    let source = PhenotypeCompiler::compile_from_foundation_asset(
        &genome,
        &capacity,
        &development,
        profile,
        &foundation,
    )
    .unwrap();
    assert_eq!(source.neuron_count(), 2048);
    assert_eq!(source.synapses().len(), 32768);
    let frames = (0..20).map(frame).collect::<Vec<_>>();
    println!(
        "META neurons={} synapses={} ticks={} debug_assertions={}",
        source.neuron_count(),
        source.synapses().len(),
        frames.len(),
        cfg!(debug_assertions)
    );
    for population in [8, 50] {
        let phenotypes = (0..population).map(|_| source.clone()).collect::<Vec<_>>();
        let caches = phenotypes
            .iter()
            .map(|p| ResidentNeuralReceptors::compile(p).unwrap())
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
        for _ in 0..2 {
            let a = run(false);
            let b = run(true);
            assert_eq!(a.1, b.1);
            black_box((a, b));
        }
        for sample in 0..9 {
            let (baseline, cached) = if sample % 2 == 0 {
                (run(false), run(true))
            } else {
                let cached = run(true);
                (run(false), cached)
            };
            assert_eq!(baseline.1, cached.1);
            println!("SAMPLE population={} sample={} baseline_first={} baseline_ns={} cached_ns={} exact_effect_rows={}",
                population, sample, sample % 2 == 0, baseline.0, cached.0, baseline.1.len());
            black_box((baseline, cached));
        }
    }
}
