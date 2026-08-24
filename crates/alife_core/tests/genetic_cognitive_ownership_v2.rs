use alife_core::{
    ActivationPolicy, BrainCapacityClass, BrainGenome, FoundationGeneticIdentity, LobeKind,
};

#[test]
fn portable_brain_genome_does_not_duplicate_biochemical_state_or_thresholds() {
    let genome = BrainGenome::scaffold(0xA0A_2002, BrainCapacityClass::N512_ID);
    let value = serde_json::to_value(genome).unwrap();
    let object = value.as_object().unwrap();

    assert!(!object.contains_key("endocrine_constants"));
    assert!(!object.contains_key("drive_thresholds"));
}

#[test]
fn region_metadata_selects_only_generic_neural_execution_modes() {
    for kind in LobeKind::ALL {
        assert!(matches!(
            kind.default_activation_policy(),
            ActivationPolicy::InputCoupled
                | ActivationPolicy::Recurrent
                | ActivationPolicy::OutputCoupled
                | ActivationPolicy::Disabled
        ));
    }
}

#[test]
fn chemistry_is_expressed_once_outside_the_portable_brain_genome() {
    let phenotype = alife_core::CreatureGenome::early_mammal_founder(
        0xA0A_2003,
        FoundationGeneticIdentity::new(21, 1, 1, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap()
    .express()
    .unwrap();
    let brain = serde_json::to_string(&phenotype.brain_genome).unwrap();
    let chemistry = serde_json::to_string(&phenotype.chemistry.biochemical).unwrap();

    assert!(!brain.contains("dopamine"));
    assert!(chemistry.contains("species"));
}
