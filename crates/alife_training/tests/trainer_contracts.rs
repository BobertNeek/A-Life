use alife_core::{
    BrainCapacityClass, BrainGenome, CandidateActionFamily, CandidateFeatureVector,
    DevelopmentState, FoundationWeightAsset, NormalizedScalar, PhenotypeCompiler, SensorProfile,
    Tick,
};
use alife_training::{
    AdamWConfig, CandidateTrainingTarget, StageTrainableMask, TrainingSequence32, TrainingTick,
    TRAINING_SEQUENCE_TICKS,
};

fn phenotype_and_foundation() -> (alife_core::BrainPhenotype, FoundationWeightAsset) {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(0x7A11, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.8).unwrap());
    let foundation =
        FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    let phenotype = PhenotypeCompiler::compile_from_foundation_asset(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
        &foundation,
    )
    .unwrap();
    (phenotype, foundation)
}

#[test]
fn adamw_defaults_match_the_approved_foundation_program() {
    let config = AdamWConfig::default();
    assert_eq!(config.learning_rate, 3.0e-4);
    assert_eq!(config.weight_decay, 1.0e-4);
    assert_eq!(config.gradient_clip, 1.0);
    config.validate().unwrap();
}

#[test]
fn stage_masks_are_exact_canonical_weight_masks() {
    let (phenotype, foundation) = phenotype_and_foundation();
    foundation.validate_against(&phenotype).unwrap();
    let chosen = 17_u32;
    let mask = StageTrainableMask::from_synapse_indices(&phenotype, &[chosen]).unwrap();
    assert_eq!(mask.len(), phenotype.synapses().len());
    assert_eq!(mask.trainable_count(), 1);
    assert!(mask.is_trainable(chosen as usize));
    assert!(!mask.is_trainable(chosen as usize + 1));
}

#[test]
fn training_sequences_are_exactly_32_ticks_and_auxiliary_taps_are_ephemeral() {
    let (phenotype, _) = phenotype_and_foundation();
    let neuron_count = phenotype.neuron_count() as usize;
    let candidate = CandidateTrainingTarget::try_new(
        CandidateActionFamily::Approach,
        CandidateFeatureVector::zero(),
        0.75,
        1.0,
    )
    .unwrap();
    let tick = TrainingTick::try_new(
        vec![0.0; neuron_count],
        vec![0.0; neuron_count],
        vec![0.0; neuron_count],
        Some(candidate),
    )
    .unwrap();
    assert!(TrainingSequence32::try_new(vec![tick.clone(); TRAINING_SEQUENCE_TICKS - 1]).is_err());
    let sequence = TrainingSequence32::try_new(vec![tick; TRAINING_SEQUENCE_TICKS]).unwrap();
    sequence.validate_for(&phenotype).unwrap();
    assert_eq!(sequence.ticks().len(), TRAINING_SEQUENCE_TICKS);
}
