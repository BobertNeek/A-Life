use alife_core::{
    BiochemistryState, BodyEventDelta, BrainCapacityClass, FoundationGeneticIdentity,
    MeasuredPhysiologyTransition, Tick, Validate,
};

fn phenotype() -> alife_core::CreaturePhenotype {
    alife_core::CreatureGenome::early_mammal_founder(
        0xA0A_2004,
        FoundationGeneticIdentity::new(22, 1, 1, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap()
    .express()
    .unwrap()
}

#[test]
fn physiology_transition_is_derived_from_authoritative_before_and_after_states() {
    let phenotype = phenotype();
    let before = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
    let after = before
        .advance(
            Tick(1),
            BodyEventDelta {
                damage: 0.4,
                energy: -0.2,
                ..BodyEventDelta::zero()
            },
            &phenotype,
        )
        .unwrap();
    let transition = MeasuredPhysiologyTransition::new(before, after).unwrap();

    transition.validate_contract().unwrap();
    assert_eq!(transition.before, before);
    assert_eq!(transition.after, after);
    assert_eq!(
        transition.energy_delta.raw(),
        after.body.energy - before.body.energy
    );
    assert!(transition.pain_delta.raw() > 0.0);
    assert_ne!(
        transition.homeostatic_delta,
        alife_core::HomeostaticDelta::zero()
    );
}
