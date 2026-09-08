use alife_core::*;

#[test]
fn inherited_action_credit_profile_changes_only_action_receptor_coefficients() {
    let capacity = BrainCapacityClass::n512();
    let genome = BrainGenome::scaffold(96001, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    let old_inputs = PhenotypeCompilerInputs::try_new(
        genome.clone(),
        &capacity,
        development.clone(),
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let old = PhenotypeCompiler::compile_validated(&old_inputs, &capacity).unwrap();
    assert_eq!(
        blake3::hash(&serde_json::to_vec(&genome).unwrap())
            .to_hex()
            .as_str(),
        "075e7047852e76ba1a4efbfc588c1666c1ed271b11b56535f0d4755e7ffe5478"
    );
    assert_eq!(
        old_inputs.canonical_digest(),
        [
            8198553176234550635,
            16078725702841470722,
            5411840204231279553,
            18095909579607225324
        ]
    );
    assert_eq!(
        old.phenotype_hash().0,
        [
            13512625940914146739,
            11450654150044100813,
            6684591199681414269,
            2884126178824629698
        ]
    );
    println!(
        "legacy_brain_json={} legacy_inputs={:?} legacy_phenotype={:?}",
        blake3::hash(&serde_json::to_vec(&genome).unwrap()).to_hex(),
        old_inputs.canonical_digest(),
        old.phenotype_hash()
    );
    // Existing code ignores this absent-by-default gene, so RED reaches the
    // compiler and fails on the unchanged unsigned prediction coefficient.
    let mut wire = serde_json::to_value(&genome).unwrap();
    wire["plasticity_parameters"]["action_candidate_credit_profile"] =
        serde_json::json!("SignedConsequences");
    let opted: BrainGenome = serde_json::from_value(wire).unwrap();
    let inputs = PhenotypeCompilerInputs::try_new(
        opted,
        &capacity,
        development,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let phenotype = PhenotypeCompiler::compile_validated(&inputs, &capacity).unwrap();
    let mut actions = 0;
    for (before, after) in old.synapses().iter().zip(phenotype.synapses()) {
        let a = &old.plasticity_receptors()[usize::from(before.receptor_index())];
        let b = &phenotype.plasticity_receptors()[usize::from(after.receptor_index())];
        let action = matches!(after.kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::ActionCandidate);
        if action && a.is_delta_enabled() {
            actions += 1;
            assert_eq!(
                b.receptor_profile().weights(),
                &[0.0, -1.0, 1.0, -0.5, 0.2, 0.0, 0.5, -0.5]
            );
        } else {
            assert_eq!(a, b, "non-action learning changed");
        }
    }
    assert!(actions > 0);
}

fn candidate() -> FoundationWeightAsset {
    let source =
        FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    let (baseline, _, _) = PhenotypeCompiler::compile_fixed_legacy_nano512_compatibility_asset(
        SensorProfile::GroundedObjectSlotsV1,
        &source,
    )
    .unwrap()
    .into_runtime_parts();
    let mut weights = source.weights().to_vec();
    let index = baseline.synapses().iter().position(|s| matches!(s.kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::ActionCandidate)).unwrap();
    weights[index] += 0.125;
    FoundationWeightAsset::from_nano512_readout_candidate(
        &baseline,
        weights,
        TrainingStageManifest::new(1, 1, 1),
    )
    .unwrap()
}

#[test]
fn v2_reuses_exact_v1_weights_and_changes_only_action_profiles_and_bound_identities() {
    let asset = candidate();
    let bytes = asset.encode_canonical().unwrap();
    let (v1, old_inputs) = PhenotypeCompiler::compile_nano512_readout_candidate(&asset).unwrap();
    let configured = Nano512ActionCreditCandidateV2::new(
        &asset,
        ActionCandidateCreditProfileV1::SignedConsequences,
    )
    .unwrap();
    let encoded = serde_json::to_vec(&configured).unwrap();
    let restored: Nano512ActionCreditCandidateV2 = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(configured, restored);
    assert_eq!(restored.asset().unwrap().encode_canonical().unwrap(), bytes);
    let (v2, inputs) =
        PhenotypeCompiler::compile_nano512_action_credit_candidate(&restored).unwrap();
    asset.validate_against(&v1).unwrap();
    asset.validate_against(&v2).unwrap();
    assert_ne!(v1.phenotype_hash(), v2.phenotype_hash());
    assert_ne!(v1.plasticity_plan_digest(), v2.plasticity_plan_digest());
    assert_ne!(old_inputs.canonical_digest(), inputs.canonical_digest());
    assert_ne!(
        old_inputs.foundation_abi().selector_digest(),
        inputs.foundation_abi().selector_digest()
    );
    assert_eq!(
        old_inputs.foundation_abi().foundation_weight_asset(),
        inputs.foundation_abi().foundation_weight_asset()
    );
    assert_eq!(v1.synapses().len(), v2.synapses().len());
    for (a, b) in v1.synapses().iter().zip(v2.synapses()) {
        let mut old_synapse = serde_json::to_value(a).unwrap();
        let mut new_synapse = serde_json::to_value(b).unwrap();
        old_synapse
            .as_object_mut()
            .unwrap()
            .remove("receptor_index");
        new_synapse
            .as_object_mut()
            .unwrap()
            .remove("receptor_index");
        assert_eq!(
            old_synapse, new_synapse,
            "topology, alpha, route or genetic weight changed"
        );
        assert_eq!(a.genetic_weight().to_bits(), b.genetic_weight().to_bits());
        let before = &v1.plasticity_receptors()[usize::from(a.receptor_index())];
        let after = &v2.plasticity_receptors()[usize::from(b.receptor_index())];
        let mut expected = serde_json::to_value(before).unwrap();
        if before.is_delta_enabled()
            && matches!(a.kind(), CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate)
        {
            expected["receptor_profile"] = serde_json::to_value(
                ActionCandidateCreditProfileV1::SignedConsequences.receptor_profile(),
            )
            .unwrap();
        }
        assert_eq!(
            expected,
            serde_json::to_value(after).unwrap(),
            "unintended learning rate, Oja, eligibility, bound or role change"
        );
    }
    let mut before = serde_json::to_value(&v1).unwrap();
    let mut after = serde_json::to_value(&v2).unwrap();
    for field in [
        "foundation_abi_selection",
        "compiler_inputs_digest",
        "plasticity_receptors",
        "plasticity_plan_digest",
        "phenotype_hash",
        "synapses",
    ] {
        before.as_object_mut().unwrap().remove(field);
        after.as_object_mut().unwrap().remove(field);
    }
    assert_eq!(before, after, "non-plasticity plans changed");
    let restored_inputs: PhenotypeCompilerInputs =
        serde_json::from_slice(&serde_json::to_vec(&inputs).unwrap()).unwrap();
    assert_eq!(
        PhenotypeCompiler::compile_validated(&restored_inputs, &BrainCapacityClass::n512())
            .unwrap(),
        v2
    );
    let restored_phenotype: BrainPhenotype =
        serde_json::from_slice(&serde_json::to_vec(&v2).unwrap()).unwrap();
    assert_eq!(restored_phenotype, v2);
    let mut tampered = serde_json::to_value(&configured).unwrap();
    tampered["action_profile"] = serde_json::json!("UnsignedReward");
    assert!(serde_json::from_value::<Nano512ActionCreditCandidateV2>(tampered).is_err());
    let mut tampered = serde_json::to_value(&configured).unwrap();
    tampered["source"]["canonical_asset"][0] = serde_json::json!(0);
    assert!(serde_json::from_value::<Nano512ActionCreditCandidateV2>(tampered).is_err());
    let mut tampered = serde_json::to_value(&inputs).unwrap();
    tampered["genome"]["plasticity_parameters"]
        .as_object_mut()
        .unwrap()
        .remove("action_candidate_credit_profile");
    assert!(serde_json::from_value::<PhenotypeCompilerInputs>(tampered).is_err());
    let builtin =
        FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    assert!(builtin.validate_against(&v2).is_err());
    assert_eq!(asset.encode_canonical().unwrap(), bytes);
    assert_eq!(
        PhenotypeCompiler::compile_validated(&old_inputs, &BrainCapacityClass::n512()).unwrap(),
        v1
    );
}

#[test]
fn v2_genetic_roundtrip_and_reproduction_preserve_profile_and_source_only() {
    let asset = candidate();
    let configured = Nano512ActionCreditCandidateV2::new(
        &asset,
        ActionCandidateCreditProfileV1::SignedConsequences,
    )
    .unwrap();
    let founder = |seed| {
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
    let old = founder(96001);
    assert!(!String::from_utf8(serde_json::to_vec(&old).unwrap())
        .unwrap()
        .contains("nano512_action_credit_candidate_v2"));
    let a = old
        .with_nano512_action_credit_candidate(configured.clone())
        .unwrap();
    let b = founder(96002)
        .with_nano512_action_credit_candidate(configured.clone())
        .unwrap();
    let restored: CreatureGenome =
        serde_json::from_slice(&serde_json::to_vec(&a).unwrap()).unwrap();
    restored.validate_contract().unwrap();
    assert_eq!(a, restored);
    let child = CreatureGenome::reproduce(&a, &b, 96003).unwrap();
    assert_eq!(child.nano512_action_credit_candidate_v2, Some(configured));
    assert!(child.nano512_readout_candidate.is_none());
    let expressed = child.express().unwrap();
    assert_eq!(
        expressed
            .brain_genome
            .plasticity_parameters()
            .action_candidate_credit_profile(),
        Some(ActionCandidateCreditProfileV1::SignedConsequences)
    );
    let (_, compiled_inputs) = PhenotypeCompiler::compile_nano512_action_credit_candidate(
        child.nano512_action_credit_candidate_v2.as_ref().unwrap(),
    )
    .unwrap();
    assert_eq!(
        expressed
            .brain_genome
            .plasticity_parameters()
            .action_candidate_credit_profile(),
        compiled_inputs
            .genome()
            .plasticity_parameters()
            .action_candidate_credit_profile()
    );
    let newborn =
        BiochemistryState::new_with_age(&child.express().unwrap(), Tick::ZERO, Tick::ZERO).unwrap();
    assert_eq!(newborn.homeostasis.drives.pain, 0.0);
    let v1 = founder(96004)
        .with_nano512_readout_candidate(&asset)
        .unwrap();
    assert!(CreatureGenome::reproduce(&a, &v1, 96005).is_err());
    let mut dual = a.clone();
    dual.nano512_readout_candidate = v1.nano512_readout_candidate;
    assert!(dual.validate_contract().is_err());
    let mut missing = a;
    missing.nano512_action_credit_candidate_v2 = None;
    assert!(missing.validate_contract().is_err());
}
