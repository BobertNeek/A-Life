//! Disposable fresh-state audit; no acquired creature or bank state is loaded.
use alife_core::{
    BiochemistryState, BodyEventDelta, BrainCapacityClass, CreatureGenome,
    FoundationGeneticIdentity, Tick,
};

fn phenotype() -> alife_core::CreaturePhenotype {
    CreatureGenome::early_mammal_founder(
        0x7151_2026,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap()
    .express()
    .unwrap()
}

#[test]
fn supplied_thermal_exposure_reaches_drive_and_recovery_reduces_body_load() {
    let p = phenotype();
    let fresh = BiochemistryState::new(&p, Tick(600)).unwrap();
    let exposed = fresh
        .advance(
            Tick(606),
            BodyEventDelta {
                temperature_stress: 1.0,
                ..BodyEventDelta::zero()
            },
            &p,
        )
        .unwrap();
    let control = fresh
        .advance(Tick(606), BodyEventDelta::zero(), &p)
        .unwrap();
    assert!(exposed.body.temperature_stress > control.body.temperature_stress);
    assert!(
        exposed.homeostasis.drives.temperature_stress
            > control.homeostasis.drives.temperature_stress
    );
    let passive = exposed
        .advance(Tick(607), BodyEventDelta::zero(), &p)
        .unwrap();
    let rested = exposed
        .advance(
            Tick(607),
            BodyEventDelta {
                sleep_recovery: 1.0,
                ..BodyEventDelta::zero()
            },
            &p,
        )
        .unwrap();
    assert!(rested.body.temperature_stress < passive.body.temperature_stress);
    println!(
        "thermal body passive={} recovered={}; drive passive={} recovered={}",
        passive.body.temperature_stress,
        rested.body.temperature_stress,
        passive.homeostasis.drives.temperature_stress,
        rested.homeostasis.drives.temperature_stress
    );
}

#[test]
fn adult_mating_readiness_rechecks_live_atp_and_juvenile_gate() {
    let p = phenotype();
    let encounter = BodyEventDelta {
        mating_opportunity: 1.0,
        ..BodyEventDelta::zero()
    };
    let juvenile = BiochemistryState::new(&p, Tick::ZERO)
        .unwrap()
        .advance(Tick(4), encounter, &p)
        .unwrap();
    assert!(!juvenile.reproduction.ready);
    let adult = BiochemistryState::new(&p, Tick(4076))
        .unwrap()
        .advance(Tick(4080), encounter, &p)
        .unwrap();
    assert!(adult.is_reproduction_ready_at(Tick(4080), Tick(4080), &p));
    let mut depleted = adult;
    depleted.homeostasis.drives.brain_atp = 0.249;
    assert!(!depleted.is_reproduction_ready_at(Tick(4080), Tick(4080), &p));
    depleted.homeostasis.drives.brain_atp = 0.25;
    assert!(depleted.is_reproduction_ready_at(Tick(4080), Tick(4080), &p));
    println!(
        "reproductive juvenile={} adult={}; adult ATP={}",
        juvenile.homeostasis.drives.reproductive_drive,
        adult.homeostasis.drives.reproductive_drive,
        adult.homeostasis.drives.brain_atp
    );
}
