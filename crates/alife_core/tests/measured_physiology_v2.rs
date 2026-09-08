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

fn mature_tick(phenotype: &alife_core::CreaturePhenotype) -> Tick {
    let maturation = u64::from(phenotype.development.maturation_duration_ticks);
    Tick(((maturation + 119) / 120) * 120)
}

#[test]
fn physiology_transition_is_derived_from_authoritative_before_and_after_states() {
    let phenotype = phenotype();
    let tick = mature_tick(&phenotype);
    let before = BiochemistryState::new(&phenotype, tick).unwrap();
    let after = before
        .advance(
            Tick(tick.raw() + 1),
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

#[test]
fn harm_signal_counts_fresh_change_once_and_does_not_punish_unchanged_pain() {
    let phenotype = phenotype();
    let mut before = BiochemistryState::new(&phenotype, mature_tick(&phenotype)).unwrap();
    before.homeostasis.drives.pain = 1.0;
    assert_eq!(
        MeasuredPhysiologyTransition::new(before, before)
            .unwrap()
            .aversive_harm(),
        0.0
    );
    let mut recovering = before;
    recovering.homeostasis.drives.pain = 0.8;
    assert_eq!(
        MeasuredPhysiologyTransition::new(before, recovering)
            .unwrap()
            .aversive_harm(),
        0.0
    );

    before.homeostasis.drives.pain = 0.0;
    let mut injured = before;
    injured.homeostasis.drives.pain = 0.25;
    injured.body.set_health(0.75).unwrap();
    assert_eq!(
        MeasuredPhysiologyTransition::new(before, injured)
            .unwrap()
            .aversive_harm(),
        0.25
    );
}
