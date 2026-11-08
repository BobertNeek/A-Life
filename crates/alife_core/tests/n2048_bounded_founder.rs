use alife_core::*;
fn inputs() -> (BrainGenome, DevelopmentState) {
    let g = BrainGenome::scaffold(0x2048_2026, BrainCapacityClass::N2048_ID);
    let d = DevelopmentState::new(g.id, Tick::ZERO, NormalizedScalar::new(1.).unwrap());
    (g, d)
}
fn compile(bounded: bool) -> (BrainPhenotype, PhenotypeCompilerInputs) {
    let (g, d) = inputs();
    if bounded {
        PhenotypeCompiler::compile_n2048_bounded_founder(g, d, SensorProfile::GroundedObjectSlotsV1)
            .unwrap()
    } else {
        PhenotypeCompiler::compile_n2048_experimental_graph(
            g,
            d,
            SensorProfile::GroundedObjectSlotsV1,
        )
        .unwrap()
    }
}
#[test]
fn explicit_branch_abi_preserves_coordinates_and_separates_identity() {
    let (old, _) = compile(false);
    let before = serde_json::to_vec(&old).unwrap();
    let (new, _) = compile(true);
    assert_eq!(old.synapses(), new.synapses());
    assert_eq!(old.neuron_dynamics(), new.neuron_dynamics());
    assert_eq!(old.sensor_encoder(), new.sensor_encoder());
    assert_eq!(old.candidate_decoder(), new.candidate_decoder());
    assert_eq!(old.route_abi_digest(), new.route_abi_digest());
    assert_ne!(old.plasticity_abi_digest(), new.plasticity_abi_digest());
    assert_ne!(old.phenotype_hash(), new.phenotype_hash());
    assert_ne!(
        old.foundation_abi().selector_digest(),
        new.foundation_abi().selector_digest()
    );
    assert_eq!(old.foundation_abi().fresh_dendritic_parameters(), (6., -1.));
    assert_eq!(
        new.foundation_abi().fresh_dendritic_parameters(),
        (0.25, 0.1)
    );
    assert_eq!(before, serde_json::to_vec(&compile(false).0).unwrap());
    let (g, d) = inputs();
    let p = PhenotypeCompiler::compile(
        &g,
        &BrainCapacityClass::n2048(),
        &d,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    assert!(!p.foundation_abi().is_n2048_bounded_founder_v1());
    assert_eq!(p.foundation_abi().fresh_dendritic_parameters(), (6., -1.));
}
#[test]
fn exact_asset_admission_rejects_cross_branch_abi() {
    let old = FoundationWeightAsset::from_phenotype_for_genetic_birth(&compile(false).0).unwrap();
    let new = FoundationWeightAsset::from_phenotype_for_genetic_birth(&compile(true).0).unwrap();
    assert_ne!(old.digest(), new.digest());
    assert_eq!(old.weights(), new.weights());
    let (g, d) = inputs();
    assert!(
        PhenotypeCompiler::compile_n2048_bounded_founder_candidate(g.clone(), d.clone(), old)
            .is_err()
    );
    assert!(
        PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
            g.clone(),
            d.clone(),
            new.clone()
        )
        .is_err()
    );
    let (p, i) =
        PhenotypeCompiler::compile_n2048_bounded_founder_candidate(g, d, new.clone()).unwrap();
    new.validate_against(&p).unwrap();
    let round: PhenotypeCompilerInputs =
        serde_json::from_slice(&serde_json::to_vec(&i).unwrap()).unwrap();
    assert_eq!(
        p,
        PhenotypeCompiler::compile_validated(&round, &BrainCapacityClass::n2048()).unwrap()
    );
    assert_eq!(
        new,
        FoundationWeightAsset::decode_canonical(&new.encode_canonical().unwrap()).unwrap()
    );
}
#[test]
fn selector_and_recipe_tamper_cannot_reinterpret_phenotype() {
    let (p, _) = compile(true);
    let bytes = serde_json::to_string(&p).unwrap();
    let substituted = bytes.replace("N2048BoundedFounderV1", "N2048ExperimentalGraphV1");
    let altered = serde_json::from_str::<BrainPhenotype>(&substituted);
    assert!(
        altered.is_err()
            || altered
                .unwrap()
                .validate_against(&BrainCapacityClass::n2048())
                .is_err()
    );
    let mut recipe = serde_json::to_value(
        N2048ExperimentalGraphV1::fresh(SensorProfile::GroundedObjectSlotsV1).unwrap(),
    )
    .unwrap();
    recipe["recipe_digest"][0] = serde_json::json!(0);
    let bad: N2048ExperimentalGraphV1 = serde_json::from_value(recipe).unwrap();
    let (g, d) = inputs();
    assert!(PhenotypeCompilerInputs::try_new_with_foundation_selection(
        g,
        &BrainCapacityClass::n2048(),
        d,
        SensorProfile::GroundedObjectSlotsV1,
        FoundationAbiSelection::N2048BoundedFounderV1(bad)
    )
    .is_err());
}
