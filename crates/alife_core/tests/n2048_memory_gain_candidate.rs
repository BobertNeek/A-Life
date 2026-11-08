use alife_core::*;
fn inputs() -> (BrainGenome, DevelopmentState) {
    let g = BrainGenome::scaffold(0x2048_2026, BrainCapacityClass::N2048_ID);
    let d = DevelopmentState::new(g.id, Tick::ZERO, NormalizedScalar::new(1.).unwrap());
    (g, d)
}
#[test]
fn memory_gain_has_distinct_identity_without_changing_graph_or_plasticity() {
    let (g, d) = inputs();
    let profile = SensorProfile::GroundedObjectSlotsV1;
    let (old, _) =
        PhenotypeCompiler::compile_n2048_experimental_graph(g.clone(), d.clone(), profile).unwrap();
    let old_bytes = serde_json::to_vec(&old).unwrap();
    let (new, inputs) =
        PhenotypeCompiler::compile_n2048_memory_gain_graph(g.clone(), d.clone(), profile).unwrap();
    assert_eq!(old.synapses(), new.synapses());
    assert_eq!(old.projections(), new.projections());
    assert_eq!(old.neuron_dynamics(), new.neuron_dynamics());
    assert_eq!(old.plasticity_receptors(), new.plasticity_receptors());
    // The ABI cap is shared by memory and cognitive scoring in general. This
    // N2048 experiment has no cognitive decoder, so its cap factor cannot
    // change cognitive clipping as an unreported second intervention.
    assert!(new.cognitive_channel_plan().is_none());
    assert!(new.synapses().iter().all(|synapse| !matches!(
        synapse.kind(),
        CompiledSynapseKind::Decoder(coordinate)
            if coordinate.head() == DecoderHeadKind::CognitiveContext
    )));
    assert_eq!(old.budgets().global.active_tiles, 137);
    assert_eq!(new.budgets().global.active_tiles, 137);
    assert_eq!(
        old.candidate_decoder()
            .memory_channel()
            .unwrap()
            .max_candidate_gain(),
        0.5
    );
    assert_eq!(
        new.candidate_decoder()
            .memory_channel()
            .unwrap()
            .max_candidate_gain(),
        12.
    );
    assert_ne!(old.phenotype_hash(), new.phenotype_hash());
    let old_selector_bytes =
        bincode::serde::encode_to_vec(old.foundation_abi(), bincode::config::standard()).unwrap();
    let new_selector_bytes =
        bincode::serde::encode_to_vec(new.foundation_abi(), bincode::config::standard()).unwrap();
    assert_eq!(
        old_selector_bytes.first(),
        Some(&0),
        "legacy experimental enum tag must not move"
    );
    assert_eq!(
        new_selector_bytes.first(),
        Some(&6),
        "new selector is appended, never inserted"
    );

    assert_ne!(
        old.foundation_abi().selector_digest(),
        new.foundation_abi().selector_digest()
    );
    let asset = FoundationWeightAsset::from_phenotype_for_genetic_birth(&new).unwrap();
    assert!(asset.is_n2048_memory_gain_candidate());
    assert!(!asset.is_n2048_experimental_graph());
    assert!(N2048ExperimentalGraphV1::for_asset(&asset).is_err());
    let old_asset = FoundationWeightAsset::from_phenotype_for_genetic_birth(&old).unwrap();
    assert!(N2048MemoryGainCandidateV1::for_asset(&old_asset).is_err());
    assert!(FoundationAbiBinding::canonical_for_foundation_asset(
        &BrainCapacityClass::n2048(),
        &asset
    )
    .is_err());
    assert!(
        PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
            g.clone(),
            d.clone(),
            asset.clone()
        )
        .is_err()
    );
    assert!(
        PhenotypeCompiler::compile_n2048_memory_gain_foundation_candidate(
            g.clone(),
            d.clone(),
            old_asset
        )
        .is_err()
    );
    let bytes = asset.encode_canonical().unwrap();
    assert_eq!(
        FoundationWeightAsset::decode_canonical(&bytes).unwrap(),
        asset
    );
    let (bound, _) = PhenotypeCompiler::compile_n2048_memory_gain_foundation_candidate(
        g.clone(),
        d.clone(),
        asset,
    )
    .unwrap();
    assert_eq!(new.synapses(), bound.synapses());
    let wire = serde_json::to_vec(&inputs).unwrap();
    let restored: PhenotypeCompilerInputs = serde_json::from_slice(&wire).unwrap();
    assert_eq!(
        PhenotypeCompiler::compile_validated(&restored, &BrainCapacityClass::n2048()).unwrap(),
        new
    );
    assert_eq!(
        serde_json::to_vec(
            &PhenotypeCompiler::compile_n2048_experimental_graph(g, d, profile)
                .unwrap()
                .0
        )
        .unwrap(),
        old_bytes
    );
}
#[test]
fn plan_rejects_unversioned_gain_and_selector_substitution() {
    let (g, d) = inputs();
    let (old, _) = PhenotypeCompiler::compile_n2048_experimental_graph(
        g.clone(),
        d.clone(),
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let (new, _) = PhenotypeCompiler::compile_n2048_memory_gain_graph(
        g,
        d,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let mut plan = serde_json::to_value(old.candidate_decoder().memory_channel().unwrap()).unwrap();
    plan["max_candidate_gain"] = serde_json::json!(12.);
    assert!(serde_json::from_value::<MemoryChannelPlan>(plan).is_err());
    let mut wire = serde_json::to_value(&new).unwrap();
    wire["foundation_abi_selection"] = serde_json::to_value(old.foundation_abi()).unwrap();
    assert!(serde_json::from_value::<BrainPhenotype>(wire).is_err());
}
