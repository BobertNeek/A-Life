#![cfg(feature = "gpu-tests")]

use alife_core::{
    BrainCapacityClass, BrainGenome, CandidateActionFamily, CandidateFeatureVector,
    CompiledSynapseKind, DecoderHeadKind, DevelopmentState, FoundationWeightAsset,
    NormalizedScalar, PhenotypeCompiler, SensorProfile, Tick, TrainingStageManifest,
};
use alife_training::{
    AdamWConfig, CandidateTrainingTarget, FoundationTrainer, StageTrainableMask,
    TrainingSequence32, TrainingTick, TRAINING_SEQUENCE_TICKS,
};

#[test]
fn n2048_exact_graph_adamw_step_changes_only_the_masked_weight_and_exports() {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(0x7A11_2048, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.8).unwrap());
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
    let (synapse_index, source_neuron, input_lane) = phenotype
        .synapses()
        .iter()
        .enumerate()
        .find_map(|(index, synapse)| match synapse.kind() {
            CompiledSynapseKind::Decoder(coordinate)
                if coordinate.head() == DecoderHeadKind::ActionCandidate
                    && coordinate.family() == CandidateActionFamily::Approach =>
            {
                Some((index as u32, synapse.source(), coordinate.input_lane()))
            }
            _ => None,
        })
        .unwrap();
    let mask = StageTrainableMask::from_synapse_indices(&phenotype, &[synapse_index]).unwrap();
    let neuron_count = phenotype.neuron_count() as usize;
    let mut features = [0.0_f32; 24];
    features[usize::from(input_lane)] = 1.0;
    let candidate = CandidateTrainingTarget::try_new(
        CandidateActionFamily::Approach,
        CandidateFeatureVector(features),
        1.0,
        1.0,
    )
    .unwrap();
    let mut inputs = vec![0.0; neuron_count];
    inputs[source_neuron as usize] = 1.0;
    let tick = TrainingTick::try_new(
        inputs,
        vec![0.0; neuron_count],
        vec![0.0; neuron_count],
        Some(candidate),
    )
    .unwrap();
    let sequence = TrainingSequence32::try_new(vec![tick; TRAINING_SEQUENCE_TICKS]).unwrap();
    let before = source.weights().to_vec();
    let mut trainer =
        FoundationTrainer::new_required(phenotype.clone(), source, mask, AdamWConfig::default())
            .unwrap();
    let receipt = trainer.train_step(&sequence).unwrap();
    assert_eq!(receipt.optimizer_step, 1);
    assert_eq!(receipt.trained_weight_count, 1);
    assert!(receipt.unclipped_gradient_norm > 0.0);
    assert!(receipt.loss_after < receipt.loss_before);

    let trained = trainer.read_weights().unwrap();
    let changed = before
        .iter()
        .zip(&trained)
        .enumerate()
        .filter(|(_, (left, right))| left.to_bits() != right.to_bits())
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    assert_eq!(changed, vec![synapse_index as usize]);

    let asset = trainer
        .export_candidate(TrainingStageManifest::new(1, 1, 1))
        .unwrap();
    let encoded = asset.encode_canonical().unwrap();
    let decoded = FoundationWeightAsset::decode_canonical(&encoded).unwrap();
    assert_eq!(decoded.weights(), trained);
    let rebuilt = PhenotypeCompiler::compile_from_foundation_asset(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
        &decoded,
    )
    .unwrap();
    decoded.validate_against(&rebuilt).unwrap();
}
