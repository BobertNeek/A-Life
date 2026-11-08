//! Ordinary stationary contact accounting, independent of reflex candidates.
use alife_core::*;
use alife_world::persistence::{AssetManifest, PortableSaveFile, RuntimeConfig};
use alife_world::*;
use std::collections::BTreeMap;

fn snapshot(world: &HeadlessWorld) -> String {
    PortableSaveFile::from_headless_world(
        "body-contact",
        world,
        RuntimeConfig::deterministic_default(world.seed(), BrainScaleTier::Nano512),
        AssetManifest::empty(),
        vec![],
    )
    .unwrap()
    .to_json_string_pretty()
    .unwrap()
}

const ORGANISM: OrganismId = OrganismId(1);
fn fixture() -> (HeadlessWorld, WorldEntityId) {
    let mut world = HeadlessScenarioBuilder::new(4249)
        .agent("creature", ORGANISM, Vec3f::ZERO)
        .build()
        .unwrap();
    let body = world.entity_id("creature").unwrap();
    let genome = CreatureGenome::early_mammal_founder(
        4249,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    let phenotype = genome.express().unwrap();
    world
        .register_organism_record(
            WorldOrganismRecord::newborn(ORGANISM, body, genome, phenotype, Tick::ZERO).unwrap(),
        )
        .unwrap();
    (world, body)
}
fn event(recovery: f32, damage: f32) -> BTreeMap<u64, BodyEventDelta> {
    BTreeMap::from([(
        ORGANISM.raw(),
        BodyEventDelta {
            sleep_recovery: recovery,
            damage,
            ..BodyEventDelta::zero()
        },
    )])
}
fn hazard(world: &mut HeadlessWorld, pain: f32) {
    world
        .editor_spawn_object(WorldEditorSpawnSpec {
            label: "stationary-contact".into(),
            kind: WorldObjectKind::Hazard,
            organism_id: None,
            position: Vec3f::new(0.2, 0.0, 0.0),
            nutrition: 0.0,
            hazard_pain: pain,
            radius: 0.6,
            token_id: None,
        })
        .unwrap();
}
#[test]
fn body_only_stationary_sleep_delivers_fresh_contact_once_without_movement() {
    let (mut world, body) = fixture();
    world
        .try_advance_tick_with_body_events(&event(1.0, 0.0))
        .unwrap();
    let record = world.organism_registry().get(ORGANISM).unwrap().clone();
    assert!(record.biochemistry().body.sleeping);
    let position = world.entity(body).unwrap().position;
    hazard(&mut world, 0.4);
    world
        .try_advance_tick_with_body_events(&event(1.0, 0.0))
        .unwrap();
    let expected = record
        .biochemistry()
        .advance(
            Tick(2),
            event(1.0, 0.4)[&ORGANISM.raw()],
            record.phenotype(),
        )
        .unwrap();
    assert_eq!(
        *world
            .organism_registry()
            .get(ORGANISM)
            .unwrap()
            .biochemistry(),
        expected
    );
    assert!(expected.body.injury > record.biochemistry().body.injury);
    assert_eq!(world.entity(body).unwrap().position, position);
}
#[test]
fn body_only_quiet_or_nonpainful_contact_keeps_exact_ordinary_sleep() {
    for contact in [false, true] {
        let (mut world, _) = fixture();
        if contact {
            hazard(&mut world, 0.0);
        }
        let record = world.organism_registry().get(ORGANISM).unwrap().clone();
        let expected = record
            .biochemistry()
            .advance(
                Tick(1),
                event(1.0, 0.0)[&ORGANISM.raw()],
                record.phenotype(),
            )
            .unwrap();
        world
            .try_advance_tick_with_body_events(&event(1.0, 0.0))
            .unwrap();
        assert_eq!(
            *world
                .organism_registry()
                .get(ORGANISM)
                .unwrap()
                .biochemistry(),
            expected
        );
    }
}
#[test]
fn body_only_contact_merges_interval_damage_without_counting_it_twice() {
    for supplied in [0.0_f32, 0.2, 0.4, 0.7] {
        let (mut world, _) = fixture();
        hazard(&mut world, 0.4);
        let record = world.organism_registry().get(ORGANISM).unwrap().clone();
        let expected = record
            .biochemistry()
            .advance(
                Tick(1),
                event(0.0, supplied.max(0.4))[&ORGANISM.raw()],
                record.phenotype(),
            )
            .unwrap();
        world
            .try_advance_tick_with_body_events(&event(0.0, supplied))
            .unwrap();
        assert_eq!(
            *world
                .organism_registry()
                .get(ORGANISM)
                .unwrap()
                .biochemistry(),
            expected
        );
    }
}
#[test]
fn body_only_contact_restore_continues_exactly_and_invalid_interval_rolls_back() {
    let (mut world, _) = fixture();
    hazard(&mut world, 0.4);
    let mut restored = PortableSaveFile::from_json_str(&snapshot(&world))
        .unwrap()
        .restore_headless_world()
        .unwrap();
    for candidate in [&mut world, &mut restored] {
        candidate
            .try_advance_tick_with_body_events(&event(1.0, 0.0))
            .unwrap();
    }
    assert_eq!(snapshot(&world), snapshot(&restored));
    let before = snapshot(&world);
    let invalid = BTreeMap::from([(404, BodyEventDelta::zero())]);
    assert!(world.try_advance_tick_with_body_events(&invalid).is_err());
    assert_eq!(snapshot(&world), before);
}

#[test]
fn body_only_boundary_does_not_redeliver_damage_after_an_action() {
    let (mut world, body) = fixture();
    hazard(&mut world, 0.4);
    let command = ChannelCommand::new(
        MotorChannel::Posture,
        ActionId(3),
        None,
        Vec3f::ZERO,
        Intensity::new(1.0).unwrap(),
        DurationTicks::new(1),
        0.0,
        Confidence::new(1.0).unwrap(),
        0,
    )
    .unwrap();
    let bundle = MotorCommandBundle::new(
        ORGANISM,
        ExperienceSequenceId::new(1).unwrap(),
        Tick::ZERO,
        vec![command],
    )
    .unwrap();
    let receipt = world.apply_registered_motor_bundle(&bundle, body).unwrap();
    assert_eq!(receipt.body_event.damage, 0.4);
    world.try_advance_tick().unwrap();
    assert_eq!(
        *world
            .organism_registry()
            .get(ORGANISM)
            .unwrap()
            .biochemistry(),
        receipt.biology_after
    );
}
