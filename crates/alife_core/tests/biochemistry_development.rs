//! EI0 body, homeostasis, development, sleep, and reproduction integration.

use std::collections::BTreeSet;

use alife_core::{
    BiochemistryState, BodyEventDelta, BrainCapacityClass, CreatureGenome,
    FoundationGeneticIdentity, Tick, Validate,
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
fn body_damage_and_energy_loss_change_drives_hormones_and_neural_modulation() {
    let phenotype = phenotype();
    let state = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
    let next = state
        .advance(
            Tick(12),
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
    assert!(next.neural.learning_rate_scale < state.neural.learning_rate_scale);
}

#[test]
fn fast_hormones_respond_before_slower_cadence_boundaries() {
    let phenotype = phenotype();
    let state = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
    let next = state
        .advance(
            Tick(1),
            BodyEventDelta {
                damage: 0.30,
                ..BodyEventDelta::zero()
            },
            &phenotype,
        )
        .unwrap();

    assert!(next.homeostasis.hormones.cortisol > state.homeostasis.hormones.cortisol);
    assert_eq!(
        next.homeostasis.drives.fatigue,
        state.homeostasis.drives.fatigue
    );
    assert_eq!(next.development.last_update_tick, Tick::ZERO);
    assert_eq!(next.reproduction.last_update_tick, Tick::ZERO);
}

#[test]
fn metabolic_development_and_reproduction_update_only_on_their_boundaries() {
    let phenotype = phenotype();
    let state = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
    let before_metabolism = state
        .advance(
            Tick(5),
            BodyEventDelta {
                energy: -0.40,
                ..BodyEventDelta::zero()
            },
            &phenotype,
        )
        .unwrap();
    assert_eq!(
        before_metabolism.homeostasis.drives.fatigue,
        state.homeostasis.drives.fatigue
    );

    let metabolism = before_metabolism
        .advance(Tick(6), BodyEventDelta::zero(), &phenotype)
        .unwrap();
    assert!(metabolism.homeostasis.drives.fatigue > state.homeostasis.drives.fatigue);
    assert_eq!(metabolism.development.last_update_tick, Tick::ZERO);
    assert_eq!(metabolism.reproduction.last_update_tick, Tick::ZERO);

    let development = metabolism
        .advance(Tick(60), BodyEventDelta::zero(), &phenotype)
        .unwrap();
    assert_eq!(development.development.last_update_tick, Tick(60));
    assert_eq!(development.reproduction.last_update_tick, Tick::ZERO);

    let reproduction = development
        .advance(Tick(120), BodyEventDelta::zero(), &phenotype)
        .unwrap();
    assert_eq!(reproduction.reproduction.last_update_tick, Tick(120));
}

#[test]
fn sleep_recovery_lowers_fatigue_and_sleep_pressure() {
    let phenotype = phenotype();
    let strained = BiochemistryState::new(&phenotype, Tick::ZERO)
        .unwrap()
        .advance(
            Tick(12),
            BodyEventDelta {
                energy: -0.80,
                damage: 0.20,
                ..BodyEventDelta::zero()
            },
            &phenotype,
        )
        .unwrap();
    let recovered = strained
        .advance(
            Tick(13),
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
    assert!(recovered.body.energy > strained.body.energy);
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
    let adult = BiochemistryState::new(&phenotype, Tick(4_000))
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

    let injured = adult
        .advance(
            Tick(4_200),
            BodyEventDelta {
                damage: 1.0,
                mating_opportunity: 1.0,
                ..BodyEventDelta::zero()
            },
            &phenotype,
        )
        .unwrap();
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
    assert!(active.neural.plasticity_scale > before.neural.plasticity_scale);
    assert_eq!(
        closed.neural.plasticity_scale,
        before.neural.plasticity_scale
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
                    reward_outcome: if tick % 19 == 0 { 0.1 } else { -0.02 },
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
fn neural_modulation_has_no_hidden_action_authority() {
    let phenotype = phenotype();
    let state = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
    let value = serde_json::to_value(state.neural).unwrap();
    let keys = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        keys,
        BTreeSet::from([
            "attention_gain",
            "development_gate",
            "learning_rate_scale",
            "plasticity_scale",
            "salience_weight",
            "sleep_gate",
            "threshold_scale",
        ])
    );
    for forbidden in ["action", "candidate", "target", "reward", "command"] {
        assert!(!keys.iter().any(|key| key.contains(forbidden)));
    }
}
