use alife_core::{
    BrainCapacityClass, CompiledSynapseKind, DecoderHeadKind, FoundationWeightAsset,
    PhenotypeCompiler, PhenotypeCompilerInputs, SensorProfile, TrainingStageManifest,
};
#[test]
fn readout_candidate_reloads_exactly_without_claiming_builtin_identity() {
    let profile = SensorProfile::GroundedObjectSlotsV1;
    let source = FoundationWeightAsset::builtin_nano512_v1(profile).unwrap();
    let (baseline, _, _) =
        PhenotypeCompiler::compile_fixed_legacy_nano512_compatibility_asset(profile, &source)
            .unwrap()
            .into_runtime_parts();
    let index = baseline
        .synapses()
        .iter()
        .position(|s| {
            matches!(s.kind(),
                CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::ActionCandidate
            )
        })
        .unwrap();
    let mut weights = source.weights().to_vec();
    weights[index] += 0.125;
    let candidate = FoundationWeightAsset::from_nano512_readout_candidate(
        &baseline,
        weights.clone(),
        TrainingStageManifest::new(1, 1, 1),
    )
    .unwrap();
    assert_ne!(
        candidate.manifest().foundation_id(),
        source.manifest().foundation_id()
    );
    assert!(!candidate.manifest().promotion_receipt().is_promoted());
    let bytes = candidate.encode_canonical().unwrap();
    let decoded = FoundationWeightAsset::decode_canonical(&bytes).unwrap();
    assert_eq!(decoded.encode_canonical().unwrap(), bytes);
    let (phenotype, inputs) =
        PhenotypeCompiler::compile_nano512_readout_candidate(&decoded).unwrap();
    assert_eq!(
        phenotype
            .synapses()
            .iter()
            .map(|s| s.genetic_weight().to_bits())
            .collect::<Vec<_>>(),
        weights.iter().map(|w| w.to_bits()).collect::<Vec<_>>()
    );
    assert_eq!(phenotype.lobe_layout(), baseline.lobe_layout());
    assert_eq!(phenotype.route_abi_digest(), baseline.route_abi_digest());
    assert_eq!(
        phenotype.plasticity_abi_digest(),
        baseline.plasticity_abi_digest()
    );
    let saved = serde_json::to_vec(&inputs).unwrap();
    let restored: PhenotypeCompilerInputs = serde_json::from_slice(&saved).unwrap();
    assert_eq!(
        serde_json::from_slice::<alife_core::BrainPhenotype>(
            &serde_json::to_vec(&phenotype).unwrap()
        )
        .unwrap(),
        phenotype
    );
    assert_eq!(
        PhenotypeCompiler::compile_validated(&restored, &BrainCapacityClass::n512()).unwrap(),
        phenotype
    );
    assert!(
        PhenotypeCompiler::compile_fixed_legacy_nano512_compatibility_asset(profile, &decoded)
            .is_err()
    );
    let mut damaged = bytes;
    let end = damaged.len() - 33;
    damaged[end] ^= 1;
    assert!(FoundationWeightAsset::decode_canonical(&damaged).is_err());
    let mut forged = serde_json::to_value(&inputs).unwrap();
    forged["sensor_profile"] = serde_json::to_value(SensorProfile::PrivilegedAffordanceV1).unwrap();
    assert!(serde_json::from_value::<PhenotypeCompilerInputs>(forged).is_err());
    let mut forged = serde_json::to_value(&inputs).unwrap();
    forged["foundation_abi_selection"]["contract"]["canonical_asset"] =
        serde_json::to_value(source.encode_canonical().unwrap()).unwrap();
    assert!(serde_json::from_value::<PhenotypeCompilerInputs>(forged).is_err());
}

#[test]
fn readout_candidate_rejects_upstream_changes_and_another_source_graph() {
    let profile = SensorProfile::GroundedObjectSlotsV1;
    let source = FoundationWeightAsset::builtin_nano512_v1(profile).unwrap();
    let (baseline, _, _) =
        PhenotypeCompiler::compile_fixed_legacy_nano512_compatibility_asset(profile, &source)
            .unwrap()
            .into_runtime_parts();
    let mut weights = source.weights().to_vec();
    weights[0] += 0.125;
    assert!(FoundationWeightAsset::from_nano512_readout_candidate(
        &baseline,
        weights,
        TrainingStageManifest::new(1, 1, 1),
    )
    .is_err());
    assert!(PhenotypeCompiler::compile_nano512_readout_candidate(&source).is_err());
}

#[test]
fn unchanged_legacy_inputs_still_reconstruct_the_builtin() {
    let profile = SensorProfile::GroundedObjectSlotsV1;
    let source = FoundationWeightAsset::builtin_nano512_v1(profile).unwrap();
    let (phenotype, inputs, _) =
        PhenotypeCompiler::compile_fixed_legacy_nano512_compatibility_asset(profile, &source)
            .unwrap()
            .into_runtime_parts();
    let restored: alife_core::PhenotypeCompilerInputs =
        serde_json::from_slice(&serde_json::to_vec(&inputs).unwrap()).unwrap();
    assert_eq!(
        PhenotypeCompiler::compile_validated(&restored, &alife_core::BrainCapacityClass::n512())
            .unwrap(),
        phenotype
    );
    assert_eq!(
        phenotype.foundation_abi().foundation_payload_digest(),
        Some(source.digest())
    );
}

#[test]
fn playable_readout_candidate_can_be_exported() {
    let profile = SensorProfile::GroundedObjectSlotsV1;
    let source = FoundationWeightAsset::builtin_nano512_v1(profile).unwrap();
    let (baseline, _, _) =
        PhenotypeCompiler::compile_fixed_legacy_nano512_compatibility_asset(profile, &source)
            .unwrap()
            .into_runtime_parts();
    let mut weights = source.weights().to_vec();
    let index = baseline.synapses().iter().position(|s| matches!(s.kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::ActionCandidate)).unwrap();
    weights[index] += 0.125;
    let result = FoundationWeightAsset::from_trained_weights(
        &baseline,
        weights,
        TrainingStageManifest::new(1, 1, 1),
    );
    assert!(
        result.is_ok(),
        "playable legacy graph must export a separately identified readout candidate: {result:?}"
    );
}
