//! EI0 body, homeostasis, development, sleep, and reproduction integration.

use std::collections::BTreeSet;

use alife_core::{
    BiochemistryState, BodyEventDelta, BodyState, BrainCapacityClass, CreatureGenome,
    FoundationGeneticIdentity, OrganKind, Tick, Validate,
};

fn phenotype() -> alife_core::CreaturePhenotype {
    CreatureGenome::early_mammal_founder(
        0xE10_8001,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap()
    .express()
    .unwrap()
}

#[test]
fn inherited_waking_fatigue_is_separate_from_food_and_recovery() {
    let mut genome = CreatureGenome::early_mammal_founder(
        0xE10_8001,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    let legacy = genome.express().unwrap();
    let legacy_state = BiochemistryState::new(&legacy, Tick(600)).unwrap();
    assert_eq!(
        legacy_state
            .graph_state()
            .locomotor_capacity(&legacy.chemistry.biochemical)
            .unwrap(),
        None
    );
    for side in [
        alife_core::AlleleSide::Maternal,
        alife_core::AlleleSide::Paternal,
    ] {
        genome.chemistry.graph = genome
            .chemistry
            .graph
            .clone()
            .with_waking_recovery(side)
            .unwrap();
    }
    let genome: CreatureGenome =
        serde_json::from_str(&serde_json::to_string(&genome).unwrap()).unwrap();
    let phenotype = genome.express().unwrap();
    let young = BiochemistryState::new(&phenotype, Tick::ZERO)
        .unwrap()
        .advance(
            Tick(1),
            BodyEventDelta {
                sleep_recovery: 1.0,
                ..BodyEventDelta::zero()
            },
            &phenotype,
        )
        .unwrap();
    assert!(
        young
            .graph_state()
            .sleep_metabolic_rate(&phenotype.chemistry.biochemical)
            .unwrap()
            .unwrap()
            < 0.2
    );
    let tick = mature_tick(&phenotype);
    let start = BiochemistryState::new(&phenotype, tick).unwrap();
    let mut fed = start;
    let mut hungry = start;
    fed.body.set_energy(1.0).unwrap();
    hungry.body.set_energy(0.1).unwrap();
    for offset in 1..=120 {
        fed = fed
            .advance(
                Tick(tick.raw() + offset),
                BodyEventDelta::zero(),
                &phenotype,
            )
            .unwrap();
        hungry = hungry
            .advance(
                Tick(tick.raw() + offset),
                BodyEventDelta::zero(),
                &phenotype,
            )
            .unwrap();
    }
    assert_eq!(
        fed.homeostasis.drives.fatigue,
        hungry.homeostasis.drives.fatigue
    );
    assert!(fed.homeostasis.drives.fatigue > start.homeostasis.drives.fatigue);
    assert!(hungry.homeostasis.drives.hunger > fed.homeostasis.drives.hunger);
    let mut ordinary_reserve = start;
    ordinary_reserve.body.set_energy(0.5).unwrap();
    for offset in 1..=120 {
        ordinary_reserve = ordinary_reserve
            .advance(
                Tick(tick.raw() + offset),
                BodyEventDelta::zero(),
                &phenotype,
            )
            .unwrap();
    }
    assert!(ordinary_reserve.homeostasis.drives.brain_atp > 0.25);
    let meal = hungry
        .advance(
            Tick(tick.raw() + 121),
            BodyEventDelta {
                nutrition: 0.5,
                ..BodyEventDelta::zero()
            },
            &phenotype,
        )
        .unwrap();
    assert!(meal.body.energy > hungry.body.energy);
    assert!(meal.homeostasis.drives.hunger < hungry.homeostasis.drives.hunger);
    assert!(meal.homeostasis.drives.fatigue >= hungry.homeostasis.drives.fatigue);
    let rested = hungry
        .advance(
            Tick(tick.raw() + 121),
            BodyEventDelta {
                sleep_recovery: 1.0,
                ..BodyEventDelta::zero()
            },
            &phenotype,
        )
        .unwrap();
    assert!(rested.homeostasis.drives.fatigue < hungry.homeostasis.drives.fatigue);
    assert!(rested.body.energy <= hungry.body.energy);
    let mut asleep = hungry;
    let mut awake = hungry;
    for offset in 121..=132 {
        asleep = asleep
            .advance(
                Tick(tick.raw() + offset),
                BodyEventDelta {
                    sleep_recovery: 1.0,
                    ..BodyEventDelta::zero()
                },
                &phenotype,
            )
            .unwrap();
        awake = awake
            .advance(
                Tick(tick.raw() + offset),
                BodyEventDelta::zero(),
                &phenotype,
            )
            .unwrap();
    }
    assert!(asleep.body.energy <= hungry.body.energy);
    assert!(asleep.body.energy > awake.body.energy);
    let sleeping_rate = asleep
        .graph_state()
        .sleep_metabolic_rate(&phenotype.chemistry.biochemical)
        .unwrap()
        .unwrap();
    assert!(sleeping_rate > 0.0 && sleeping_rate < 0.2);
    assert!(!phenotype
        .chemistry
        .biochemical
        .emitters()
        .iter()
        .any(
            |row| row.source == alife_core::BiochemicalSourceLocus::SleepRecovery
                && row.target == alife_core::ChemicalSpeciesId(7)
        ));
    assert!(
        rested
            .graph_state()
            .locomotor_capacity(&phenotype.chemistry.biochemical)
            .unwrap()
            .unwrap()
            > hungry
                .graph_state()
                .locomotor_capacity(&phenotype.chemistry.biochemical)
                .unwrap()
                .unwrap()
    );
}

fn mature_tick(phenotype: &alife_core::CreaturePhenotype) -> Tick {
    let maturation = u64::from(phenotype.development.maturation_duration_ticks);
    Tick(((maturation + 119) / 120) * 120)
}

#[test]
fn body_damage_and_energy_loss_change_drives_hormones_and_neural_modulation() {
    let phenotype = phenotype();
    let tick = mature_tick(&phenotype);
    let state = BiochemistryState::new(&phenotype, tick).unwrap();
    let next = state
        .advance(
            Tick(tick.raw() + 12),
            BodyEventDelta {
                energy: -0.30,
                damage: 0.40,
                ..BodyEventDelta::zero()
            },
            &phenotype,
        )
        .unwrap();

    assert!(next.homeostasis.drives.fatigue > state.homeostasis.drives.fatigue);
    assert!(next.homeostasis.drives.pain > state.homeostasis.drives.pain);
    assert!(next.homeostasis.hormones.cortisol > state.homeostasis.hormones.cortisol);
    assert!(
        next.neural_receptor_frame(&phenotype)
            .unwrap()
            .activation_for(alife_core::NeuralReceptorClass::PlasticityAversive)
            > state
                .neural_receptor_frame(&phenotype)
                .unwrap()
                .activation_for(alife_core::NeuralReceptorClass::PlasticityAversive)
    );
}

#[test]
fn fast_hormones_respond_before_development_and_reproduction_boundaries() {
    let phenotype = phenotype();
    let tick = mature_tick(&phenotype);
    let state = BiochemistryState::new(&phenotype, tick).unwrap();
    let next = state
        .advance(
            Tick(tick.raw() + 1),
            BodyEventDelta {
                damage: 0.30,
                ..BodyEventDelta::zero()
            },
            &phenotype,
        )
        .unwrap();

    assert!(next.homeostasis.hormones.cortisol > state.homeostasis.hormones.cortisol);
    assert_eq!(
        next.development.last_update_tick,
        state.development.last_update_tick
    );
    assert_eq!(
        next.reproduction.last_update_tick,
        state.reproduction.last_update_tick
    );
}

#[test]
fn metabolic_development_and_reproduction_update_only_on_their_boundaries() {
    let phenotype = phenotype();
    let tick = mature_tick(&phenotype);
    let state = BiochemistryState::new(&phenotype, tick).unwrap();
    let before_metabolism = state
        .advance(Tick(tick.raw() + 5), BodyEventDelta::zero(), &phenotype)
        .unwrap();
    assert_eq!(
        before_metabolism.development.last_update_tick,
        state.development.last_update_tick
    );
    assert_eq!(
        before_metabolism.reproduction.last_update_tick,
        state.reproduction.last_update_tick
    );

    let metabolism = before_metabolism
        .advance(Tick(tick.raw() + 6), BodyEventDelta::zero(), &phenotype)
        .unwrap();
    assert!(metabolism.body.energy < before_metabolism.body.energy);
    assert!(metabolism.homeostasis.drives.fatigue > before_metabolism.homeostasis.drives.fatigue);
    assert_eq!(
        metabolism.development.last_update_tick,
        state.development.last_update_tick
    );
    assert_eq!(
        metabolism.reproduction.last_update_tick,
        state.reproduction.last_update_tick
    );

    let development = metabolism
        .advance(Tick(tick.raw() + 60), BodyEventDelta::zero(), &phenotype)
        .unwrap();
    assert_eq!(
        development.development.last_update_tick,
        Tick(tick.raw() + 60)
    );
    assert_eq!(
        development.reproduction.last_update_tick,
        state.reproduction.last_update_tick
    );

    let reproduction = development
        .advance(Tick(tick.raw() + 120), BodyEventDelta::zero(), &phenotype)
        .unwrap();
    assert_eq!(
        reproduction.reproduction.last_update_tick,
        Tick(tick.raw() + 120)
    );
}

#[test]
fn organ_upkeep_follows_elapsed_cadence_crossings() {
    let phenotype = phenotype();
    let event = BodyEventDelta::zero();
    let start = BiochemistryState::new(&phenotype, Tick(5)).unwrap();
    let jumped = start.advance(Tick(7), event, &phenotype).unwrap();
    let partitioned = start
        .advance(Tick(6), event, &phenotype)
        .unwrap()
        .advance(Tick(7), event, &phenotype)
        .unwrap();

    assert_eq!(
        jumped.body.organ(OrganKind::Locomotor).energy.to_bits(),
        partitioned
            .body
            .organ(OrganKind::Locomotor)
            .energy
            .to_bits()
    );

    let at_boundary = start.advance(Tick(6), event, &phenotype).unwrap();
    let repeated = at_boundary.advance(Tick(6), event, &phenotype).unwrap();
    assert_eq!(
        repeated.body.organ(OrganKind::Locomotor).energy.to_bits(),
        at_boundary
            .body
            .organ(OrganKind::Locomotor)
            .energy
            .to_bits()
    );
}

#[test]
fn sleep_recovery_lowers_fatigue_and_sleep_pressure() {
    let phenotype = phenotype();
    let tick = mature_tick(&phenotype);
    let strained = BiochemistryState::new(&phenotype, tick)
        .unwrap()
        .advance(
            Tick(tick.raw() + 12),
            BodyEventDelta {
                energy: -0.40,
                damage: 0.20,
                ..BodyEventDelta::zero()
            },
            &phenotype,
        )
        .unwrap();
    let recovered = strained
        .advance(
            Tick(tick.raw() + 13),
            BodyEventDelta {
                sleep_recovery: 0.80,
                ..BodyEventDelta::zero()
            },
            &phenotype,
        )
        .unwrap();

    assert!(recovered.homeostasis.drives.fatigue < strained.homeostasis.drives.fatigue);
    assert!(
        recovered.homeostasis.hormones.sleep_pressure
            < strained.homeostasis.hormones.sleep_pressure
    );
    // Sleep relieves fatigue and permits reserve-paid repair; it is not food.
    assert!(recovered.body.energy <= strained.body.energy);
    assert!(recovered.body.health >= strained.body.health);
}

#[test]
fn puberty_health_and_mating_opportunity_gate_reproduction() {
    let phenotype = phenotype();
    let juvenile = BiochemistryState::new(&phenotype, Tick::ZERO)
        .unwrap()
        .advance(
            Tick(120),
            BodyEventDelta {
                mating_opportunity: 1.0,
                ..BodyEventDelta::zero()
            },
            &phenotype,
        )
        .unwrap();
    assert!(!juvenile.reproduction.puberty_reached);
    assert!(!juvenile.reproduction.ready);

    let adult_tick = Tick(4_080);
    // Keep the encounter short enough that this fixture remains ATP-sufficient.
    // A long unfed batch also exercises metabolic exhaustion, a different gate.
    let adult = BiochemistryState::new(&phenotype, Tick(4_076))
        .unwrap()
        .advance(
            adult_tick,
            BodyEventDelta {
                mating_opportunity: 1.0,
                ..BodyEventDelta::zero()
            },
            &phenotype,
        )
        .unwrap();
    assert!(adult.reproduction.puberty_reached);
    assert!(adult.reproduction.ready);

    // Sustained damage outruns reserve-paid repair. Two isolated hits separated
    // by a long recovery batch are not necessarily disabling injury.
    let mut injured = adult;
    for tick in (4090..=4200).step_by(10) {
        injured = injured
            .advance(
                Tick(tick),
                BodyEventDelta {
                    damage: 1.0,
                    mating_opportunity: 1.0,
                    ..BodyEventDelta::zero()
                },
                &phenotype,
            )
            .unwrap();
    }
    assert!(!injured.reproduction.healthy_enough);
    assert!(!injured.reproduction.ready);
}

#[test]
fn critical_period_raises_plasticity_then_closes() {
    let phenotype = phenotype();
    let period = phenotype.development.critical_period;
    let before = BiochemistryState::new(&phenotype, Tick(period.opens_at.raw() - 1)).unwrap();
    let active = BiochemistryState::new(&phenotype, period.opens_at).unwrap();
    let closed = BiochemistryState::new(&phenotype, Tick(period.closes_at.raw() + 1)).unwrap();

    assert!(!before.development.critical_period_active);
    assert!(active.development.critical_period_active);
    assert!(!closed.development.critical_period_active);
    assert!(active.development.critical_period_plasticity_bias > 0.0);
    assert_eq!(
        closed.development.critical_period_plasticity_bias,
        before.development.critical_period_plasticity_bias
    );
}

#[test]
fn long_multi_rate_run_stays_bounded() {
    let phenotype = phenotype();
    let mut state = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
    for tick in 1..=2_000_u64 {
        let signed = if tick % 3 == 0 { -0.04 } else { 0.02 };
        state = state
            .advance(
                Tick(tick),
                BodyEventDelta {
                    energy: signed,
                    damage: if tick % 17 == 0 { 0.03 } else { 0.0 },
                    nutrition: if tick % 11 == 0 { 0.05 } else { 0.0 },
                    social_contact: if tick % 13 == 0 { 0.08 } else { 0.0 },
                    sleep_recovery: if tick % 23 == 0 { 0.1 } else { 0.0 },
                    ..BodyEventDelta::zero()
                },
                &phenotype,
            )
            .unwrap();
        state.validate_contract().unwrap();
        assert!(state
            .homeostasis
            .drives
            .to_array()
            .into_iter()
            .chain(state.homeostasis.hormones.to_array())
            .all(|value| (0.0..=1.0).contains(&value)));
    }
}

#[test]
fn invalid_body_bounds_are_rejected() {
    let phenotype = phenotype();
    let state = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
    assert!(state
        .advance(
            Tick(1),
            BodyEventDelta {
                damage: 1.01,
                ..BodyEventDelta::zero()
            },
            &phenotype,
        )
        .is_err());

    let mut invalid_state = state;
    invalid_state.body.energy = f32::NAN;
    assert!(invalid_state.validate_contract().is_err());
}

#[test]
fn neural_receptor_frame_has_no_hidden_action_authority() {
    let phenotype = phenotype();
    let state = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
    let value = serde_json::to_value(state.neural_receptor_frame(&phenotype).unwrap()).unwrap();
    let keys = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        keys,
        BTreeSet::from([
            "activations",
            "schema_version",
            "source_chemistry_version",
            "source_tick",
        ])
    );
    for forbidden in ["action", "candidate", "target", "reward", "command"] {
        assert!(!keys.iter().any(|key| key.contains(forbidden)));
    }
}

#[test]
fn body_compatibility_projections_validate_for_newborns_and_legacy_migrations() {
    for efficiency in [0.0_f32, 0.2, 0.8, 1.0] {
        let mut phenotype = phenotype();
        phenotype.body.metabolic_efficiency = efficiency;

        let state = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
        state.validate_contract().unwrap();
    }

    for (energy, health, injury, temperature_stress, sleeping) in [
        (0.9_f32, 1.0, 0.0, 0.0, false),
        (0.2_f32, 0.7, 0.3, 0.4, true),
    ] {
        let body =
            BodyState::migrate_legacy_v1(energy, health, injury, temperature_stress, sleeping)
                .unwrap();
        body.validate_contract().unwrap();

        let round_trip: BodyState =
            serde_json::from_slice(&serde_json::to_vec(&body).unwrap()).unwrap();
        assert_eq!(round_trip, body);
        round_trip.validate_contract().unwrap();
    }
}
