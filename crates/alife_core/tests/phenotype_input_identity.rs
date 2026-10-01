use alife_core::{
    BrainCapacityClass, BrainGenome, DevelopmentState, InnatePriorityGenes, NormalizedScalar,
    PhenotypeCompiler, PhenotypeCompilerInputs, SensorProfile, Tick,
};

fn inputs_for(genome: BrainGenome) -> PhenotypeCompilerInputs {
    let capacity = BrainCapacityClass::n512();
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    PhenotypeCompilerInputs::try_new(
        genome,
        &capacity,
        development,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap()
}

fn founder_inputs() -> PhenotypeCompilerInputs {
    inputs_for(BrainGenome::scaffold(96_001, BrainCapacityClass::N512_ID))
}

#[test]
fn each_innate_gene_binds_input_and_expressed_identity_without_changing_synapses() {
    let capacity = BrainCapacityClass::n512();
    let baseline_inputs = founder_inputs();
    let baseline = PhenotypeCompiler::compile_validated(&baseline_inputs, &capacity).unwrap();
    for field in ["reflex_strength", "food_attraction", "hazard_aversion"] {
        let mut genome_wire = serde_json::to_value(baseline_inputs.genome()).unwrap();
        genome_wire["innate_priority"][field] = serde_json::json!(0.1);
        let changed_inputs = inputs_for(serde_json::from_value(genome_wire).unwrap());
        let changed = PhenotypeCompiler::compile_validated(&changed_inputs, &capacity).unwrap();
        assert_ne!(
            baseline_inputs.canonical_digest(),
            changed_inputs.canonical_digest(),
            "{field} was omitted from input identity"
        );
        assert_ne!(baseline.phenotype_hash(), changed.phenotype_hash());
        assert_ne!(
            baseline.candidate_decoder().canonical_digest(),
            changed.candidate_decoder().canonical_digest()
        );
        assert_eq!(baseline.synapses(), changed.synapses());
    }
}

#[test]
fn innate_gene_tampering_or_removal_with_the_original_digest_is_rejected() {
    let baseline = serde_json::to_value(founder_inputs()).unwrap();
    for field in ["reflex_strength", "food_attraction", "hazard_aversion"] {
        let mut tampered = baseline.clone();
        tampered["genome"]["innate_priority"][field] = serde_json::json!(0.1);
        assert!(
            serde_json::from_value::<PhenotypeCompilerInputs>(tampered).is_err(),
            "accepted altered {field} with an unchanged digest"
        );
    }
    let mut omitted = baseline;
    omitted["genome"]
        .as_object_mut()
        .unwrap()
        .remove("innate_priority");
    assert!(serde_json::from_value::<PhenotypeCompilerInputs>(omitted).is_err());
}

#[test]
fn obsolete_or_unknown_input_schemas_are_rejected_explicitly() {
    let baseline = serde_json::to_value(founder_inputs()).unwrap();
    for version in [5, 7] {
        let mut obsolete = baseline.clone();
        obsolete["schema_version"] = serde_json::json!(version);
        let error = serde_json::from_value::<PhenotypeCompilerInputs>(obsolete).unwrap_err();
        assert!(
            error.to_string().contains(&format!(
                "unsupported compiler-input schema version {version}; expected 6"
            )),
            "schema rejection was not explicit: {error}"
        );
    }
}

#[test]
fn current_inputs_roundtrip_with_founder_or_zero_innate_genes() {
    let capacity = BrainCapacityClass::n512();
    let founder = founder_inputs();
    let mut zero_genome = founder.genome().clone();
    zero_genome.innate_priority = InnatePriorityGenes::default();
    for original in [founder, inputs_for(zero_genome)] {
        let wire = serde_json::to_value(&original).unwrap();
        assert_eq!(wire["schema_version"], serde_json::json!(6));
        let restored: PhenotypeCompilerInputs = serde_json::from_value(wire).unwrap();
        assert_eq!(restored, original);
        let before = PhenotypeCompiler::compile_validated(&original, &capacity).unwrap();
        let after = PhenotypeCompiler::compile_validated(&restored, &capacity).unwrap();
        assert_eq!(after, before);
    }
}
