use alife_core::*;

#[test]
fn retained_nano_founder_asset_remains_canonically_admissible_without_rewriting() {
    let bytes = include_bytes!(
        "../../../assets/founders/scaled-choice-nociceptive-v1/candidate.alife-foundation"
    );
    let asset = FoundationWeightAsset::decode_canonical(bytes)
        .expect("retained candidate must remain admissible");
    assert_eq!(asset.encode_canonical().unwrap(), bytes);
    let (compiled, _) = PhenotypeCompiler::compile_nano512_readout_candidate(&asset).unwrap();
    assert!(compiled
        .synapses()
        .iter()
        .zip(asset.weights())
        .all(|(s, w)| s.genetic_weight().to_bits() == w.to_bits()));
}

#[test]
fn legacy_nano_candidate_keeps_signed_readouts_and_every_frozen_weight() {
    let profile = SensorProfile::GroundedObjectSlotsV1;
    let source = FoundationWeightAsset::builtin_nano512_v1(profile).unwrap();
    let (baseline, _, _) =
        PhenotypeCompiler::compile_fixed_legacy_nano512_compatibility_asset(profile, &source)
            .unwrap()
            .into_runtime_parts();
    let mut weights = source.weights().to_vec();
    let index = baseline.synapses().iter().position(|s| matches!(s.kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::ActionCandidate && c.input_lane() == 2)).unwrap();
    weights[index] = -0.375;
    let positive_distance = baseline.synapses().iter().enumerate().find(|(i, s)| *i != index && matches!(s.kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::ActionCandidate && c.input_lane() == 2)).unwrap().0;
    weights[positive_distance] = 0.625;
    let asset = FoundationWeightAsset::from_trained_weights(
        &baseline,
        weights.clone(),
        TrainingStageManifest::new(1, 1, 1),
    )
    .unwrap();
    let encoded = asset.encode_canonical().unwrap();
    let (candidate, inputs) = PhenotypeCompiler::compile_nano512_readout_candidate(&asset).unwrap();
    assert!(candidate
        .synapses()
        .iter()
        .zip(&weights)
        .all(|(s, w)| s.genetic_weight().to_bits() == w.to_bits()));
    assert_eq!(asset.encode_canonical().unwrap(), encoded);
    let restored: PhenotypeCompilerInputs =
        serde_json::from_slice(&serde_json::to_vec(&inputs).unwrap()).unwrap();
    assert_eq!(
        PhenotypeCompiler::compile_validated(&restored, &BrainCapacityClass::n512()).unwrap(),
        candidate
    );
}

#[test]
fn exact_n2048_foundation_preserves_negative_decoder_weights_and_rejects_negative_excitation() {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(96001, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    let procedural = PhenotypeCompiler::compile(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let mut weights: Vec<_> = procedural
        .synapses()
        .iter()
        .map(|s| s.genetic_weight())
        .collect();
    let index = procedural.synapses().iter().position(|s| matches!(s.kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::ActionCandidate && c.input_lane() == 2)).unwrap();
    weights[index] = -0.375;
    let positive_distance = procedural.synapses().iter().enumerate().find(|(i, s)| *i != index && matches!(s.kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::ActionCandidate && c.input_lane() == 2)).unwrap().0;
    weights[positive_distance] = 0.625;
    let speech = procedural
        .synapses()
        .iter()
        .position(|s| {
            matches!(s.kind(),
        CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::SpeechPayload)
        })
        .unwrap();
    weights[speech] = -0.25;
    weights[speech + 1] = 0.5;

    let asset = FoundationWeightAsset::from_trained_weights(
        &procedural,
        weights.clone(),
        TrainingStageManifest::new(1, 1, 1),
    )
    .unwrap();
    let bytes = asset.encode_canonical().unwrap();
    let (compiled, inputs) = PhenotypeCompiler::compile_n2048_foundation_candidate(
        genome.clone(),
        development.clone(),
        asset.clone(),
    )
    .expect("signed mathematical readouts must survive exact foundation replacement");
    assert_eq!(
        compiled.synapses()[index].genetic_weight().to_bits(),
        (-0.375_f32).to_bits()
    );
    assert_eq!(
        compiled.synapses()[speech].genetic_weight().to_bits(),
        (-0.25_f32).to_bits()
    );
    assert!(compiled
        .synapses()
        .iter()
        .zip(&weights)
        .all(|(s, w)| s.genetic_weight().to_bits() == w.to_bits()));
    let restored: PhenotypeCompilerInputs =
        serde_json::from_slice(&serde_json::to_vec(&inputs).unwrap()).unwrap();
    assert_eq!(
        PhenotypeCompiler::compile_validated(&restored, &capacity).unwrap(),
        compiled
    );
    assert_eq!(asset.encode_canonical().unwrap(), bytes);

    let recurrent = procedural
        .synapses()
        .iter()
        .position(|s| {
            s.kind() == CompiledSynapseKind::Recurrent
                && matches!(
                    procedural.projections()[usize::from(s.route_index())].projection_type(),
                    ProjectionType::MotorProposal | ProjectionType::Homeostatic
                )
        })
        .unwrap();
    weights[recurrent] = -0.375;
    let invalid = FoundationWeightAsset::from_trained_weights(
        &procedural,
        weights,
        TrainingStageManifest::new(1, 1, 1),
    )
    .unwrap();
    assert!(
        PhenotypeCompiler::compile_n2048_foundation_candidate(genome, development, invalid)
            .is_err()
    );
}

#[test]
fn private_n2048_foundation_replacement_does_not_apply_builtin_birth_signs() {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(96001, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    let profile = SensorProfile::GroundedObjectSlotsV1;
    let procedural = PhenotypeCompiler::compile(&genome, &capacity, &development, profile).unwrap();
    let mut weights: Vec<_> = procedural
        .synapses()
        .iter()
        .map(|s| s.genetic_weight())
        .collect();
    let index = procedural.synapses().iter().position(|s| matches!(s.kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::ActionCandidate && c.input_lane() == 2)).unwrap();
    weights[index] = 0.375;
    let private = FoundationWeightAsset::from_trained_weights(
        &procedural,
        weights,
        TrainingStageManifest::new(1, 1, 1),
    )
    .unwrap();
    let bytes = private.encode_canonical().unwrap();
    let compiled = PhenotypeCompiler::compile_from_foundation_asset(
        &genome,
        &capacity,
        &development,
        profile,
        &private,
    )
    .unwrap();
    assert!(compiled.migrated_n2048_foundation_v1().is_none());
    assert!((compiled.synapses()[index].genetic_weight() - 0.375).abs() < 0.0051);
    assert_eq!(private.encode_canonical().unwrap(), bytes);
}
