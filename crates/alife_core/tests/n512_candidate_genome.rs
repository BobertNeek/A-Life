use alife_core::*;

#[test]
fn legacy_genome_wire_identity_is_unchanged() {
    let identity = FoundationGeneticIdentity::new(
        FoundationId::N512_V1.raw(),
        1,
        FoundationCompatibilityFamilyId::N512_FOUNDATION.raw(),
        BrainCapacityClass::N512_ID,
    )
    .unwrap();
    let genome = CreatureGenome::early_mammal_founder(901, identity).unwrap();
    let bytes = serde_json::to_vec(&genome).unwrap();
    assert_eq!(
        blake3::hash(&bytes).to_hex().as_str(),
        "752e4e27febcec3958f3f1e4ebf22584ad547a1adaafb3918cdb88494aab8784"
    );
    let restored: CreatureGenome = serde_json::from_slice(&bytes).unwrap();
    assert!(restored.nano512_readout_candidate.is_none());
    assert_eq!(serde_json::to_vec(&restored).unwrap(), bytes);
}

#[test]
fn same_prior_is_inherited_and_mixed_priors_fail_closed() {
    let profile = SensorProfile::GroundedObjectSlotsV1;
    let builtin = FoundationWeightAsset::builtin_nano512_v1(profile).unwrap();
    let (baseline, _, _) =
        PhenotypeCompiler::compile_fixed_legacy_nano512_compatibility_asset(profile, &builtin)
            .unwrap()
            .into_runtime_parts();
    let candidate = FoundationWeightAsset::from_trained_weights(
        &baseline,
        builtin.weights().to_vec(),
        TrainingStageManifest::new(1, 1, 1),
    )
    .unwrap();
    let old = |seed| {
        CreatureGenome::early_mammal_founder(
            seed,
            FoundationGeneticIdentity::new(
                FoundationId::N512_V1.raw(),
                1,
                FoundationCompatibilityFamilyId::N512_FOUNDATION.raw(),
                BrainCapacityClass::N512_ID,
            )
            .unwrap(),
        )
        .unwrap()
    };
    let maternal = old(901).with_nano512_readout_candidate(&candidate).unwrap();
    let paternal = old(902).with_nano512_readout_candidate(&candidate).unwrap();
    let child = CreatureGenome::reproduce(&maternal, &paternal, 903).unwrap();
    assert_eq!(
        child.nano512_readout_candidate,
        maternal.nano512_readout_candidate
    );
    assert_eq!(child.foundation, maternal.foundation);
    let restored: CreatureGenome =
        serde_json::from_slice(&serde_json::to_vec(&child).unwrap()).unwrap();
    restored.validate_contract().unwrap();
    assert_eq!(restored, child);
    assert!(CreatureGenome::reproduce(&maternal, &old(902), 903).is_err());
    let mut changed_weights = builtin.weights().to_vec();
    let readout = baseline.synapses().iter().position(|synapse| matches!(synapse.kind(), CompiledSynapseKind::Decoder(coordinate) if coordinate.head() == DecoderHeadKind::ActionCandidate)).unwrap();
    changed_weights[readout] += 0.125;
    let other_candidate = FoundationWeightAsset::from_trained_weights(
        &baseline,
        changed_weights,
        TrainingStageManifest::new(1, 1, 1),
    )
    .unwrap();
    let other_parent = old(902)
        .with_nano512_readout_candidate(&other_candidate)
        .unwrap();
    assert_eq!(other_parent.foundation, maternal.foundation);
    assert!(CreatureGenome::reproduce(&maternal, &other_parent, 903).is_err());
    let mut forged = maternal.clone();
    forged.foundation = old(901).foundation;
    assert!(forged.validate_contract().is_err());
    forged = maternal;
    forged.nano512_readout_candidate = None;
    assert!(forged.validate_contract().is_err());
}

#[test]
fn candidate_foundation_identity_requires_its_exact_payload() {
    let identity = FoundationGeneticIdentity::new(
        FoundationId::N512_READOUT_CANDIDATE_V1.raw(),
        1,
        FoundationCompatibilityFamilyId::N512_FOUNDATION.raw(),
        BrainCapacityClass::N512_ID,
    )
    .unwrap();
    assert!(
        CreatureGenome::early_mammal_founder(901, identity).is_err(),
        "candidate identity without its genetic payload must fail closed"
    );
}
