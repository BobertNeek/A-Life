use alife_core::{
    BrainCapacityClass, BrainGenome, DevelopmentState, FoundationWeightAsset,
    NeuralReceptorPhenotype, NormalizedScalar, PhenotypeCompiler, SensorProfile, Tick,
    LEGACY_NANO512_V1_COORDINATE_SEED,
};
use std::hint::black_box;
use std::time::Instant;

fn main() {
    for profile in [SensorProfile::GroundedObjectSlotsV1] {
        let capacity = BrainCapacityClass::n512();
        let genome = BrainGenome::scaffold(LEGACY_NANO512_V1_COORDINATE_SEED, capacity.id());
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
        let foundation = FoundationWeightAsset::builtin_nano512_v1(profile).unwrap();
        let (phenotype, _, _) = PhenotypeCompiler::compile_from_legacy_nano512_compatibility_asset(
            &genome,
            &capacity,
            &development,
            profile,
            &foundation,
        )
        .unwrap()
        .into_runtime_parts();
        let cached = NeuralReceptorPhenotype::compile(&phenotype).unwrap();
        let mut compile_ns = Vec::new();
        let mut reuse_ns = Vec::new();
        for _ in 0..100 {
            black_box(NeuralReceptorPhenotype::compile(black_box(&phenotype)).unwrap());
        }
        for _ in 0..9 {
            let started = Instant::now();
            for _ in 0..1000 {
                for _ in 0..8 {
                    black_box(NeuralReceptorPhenotype::compile(black_box(&phenotype)).unwrap());
                }
            }
            compile_ns.push(started.elapsed().as_nanos() as u64 / 1000);
            let started = Instant::now();
            for _ in 0..1000 {
                for _ in 0..8 {
                    black_box(*black_box(&cached));
                }
            }
            reuse_ns.push(started.elapsed().as_nanos() as u64 / 1000);
        }
        println!("profile={profile:?} neuron_count={} synapse_count={} phenotype_hash={:?} expression={:?} compile_ns_per_eight_rows={compile_ns:?} reuse_ns_per_eight_rows={reuse_ns:?}",
            phenotype.neuron_count(), phenotype.synapses().len(), phenotype.phenotype_hash(), cached.expression());
    }
}
