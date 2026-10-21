#![cfg(feature = "gpu-tests")]

use alife_core::{
    BrainCapacityClass, BrainGenome, DevelopmentState, FoundationWeightAsset, NormalizedScalar,
    PhenotypeCompiler, SensorProfile, Tick,
};
use alife_training::{
    AdamWConfig, FoundationCurriculumStage, FoundationTrainer, N2048CurriculumV1,
    N2048FoundationProgram, N2048_FOUNDATION_TRAINING_SEED,
};

#[test]
fn shipped_foundation_still_passes_every_curated_gpu_regression_gate() {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(N2048_FOUNDATION_TRAINING_SEED, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    let source =
        FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    let phenotype = PhenotypeCompiler::compile_from_foundation_asset(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
        &source,
    )
    .unwrap();
    let mask = N2048CurriculumV1::new()
        .stage_mask(&phenotype, FoundationCurriculumStage::HeldOutGeneralization)
        .unwrap();
    let trainer =
        FoundationTrainer::new_required(phenotype, source, mask, AdamWConfig::default()).unwrap();
    let program = N2048FoundationProgram::resume(trainer).unwrap();
    assert_eq!(program.completed_stage_count(), 9);
}
