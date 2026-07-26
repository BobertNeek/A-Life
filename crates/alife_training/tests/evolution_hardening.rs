#![cfg(feature = "gpu-tests")]

use alife_core::{
    BrainCapacityClass, BrainGenome, DevelopmentState, FoundationWeightAsset, NormalizedScalar,
    PhenotypeCompiler, SensorProfile, Tick,
};
use alife_runtime::GpuSessionConsumerKind;
use alife_training::{
    AdamWConfig, FoundationCurriculumStage, FoundationTrainer, HardeningMutationKind,
    N2048CurriculumV1, N2048EvolutionHardener, N2048FoundationProgram,
    HARDENING_NEWBORNS_PER_GENOME, HARDENING_TICKS_PER_WORLD, HARDENING_WORLD_COUNT,
    N2048_FOUNDATION_TRAINING_SEED,
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

#[test]
fn production_gpu_evolution_evaluates_four_memory_empty_newborns_across_four_worlds() {
    let source =
        FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    let mut hardener = N2048EvolutionHardener::new_required(source).unwrap();
    assert_eq!(hardener.consumer_kind(), GpuSessionConsumerKind::Evolution);
    let genome =
        BrainGenome::scaffold(N2048_FOUNDATION_TRAINING_SEED, BrainCapacityClass::N2048_ID);
    let evaluation = hardener
        .evaluate_genome(genome, HardeningMutationKind::Baseline)
        .unwrap();
    assert!(evaluation.viable, "{:?}", evaluation.nonviable_reason);
    assert_eq!(evaluation.newborn_count, HARDENING_NEWBORNS_PER_GENOME);
    assert_eq!(evaluation.world_count, HARDENING_WORLD_COUNT);
    assert_eq!(
        evaluation.neural_ticks,
        HARDENING_NEWBORNS_PER_GENOME * HARDENING_TICKS_PER_WORLD
    );
    evaluation.fitness.unwrap().validate().unwrap();
}
