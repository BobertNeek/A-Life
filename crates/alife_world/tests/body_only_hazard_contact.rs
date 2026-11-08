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

#[test]
fn body_only_candidate_withdraws_sleeping_contact_and_releases_after_removal() {
    let (mut world, body) = fixture();
    world
        .try_advance_tick_with_body_events(&event(1.0, 0.0))
        .unwrap();
    let before = world.organism_registry().get(ORGANISM).unwrap().clone();
    assert!(before.biochemistry().body.sleeping);
    hazard(&mut world, 0.4);
    let yaw = world.entity(body).unwrap().body_yaw;
    let receipt = world
        .try_advance_tick_with_body_contact_candidate(&event(1.0, 0.0))
        .unwrap();
    assert_eq!(receipt.responses.len(), 1);
    let response = &receipt.responses[0];
    assert_eq!(response.schema_version, 1);
    assert!(response.sleep_recovery_interrupted);
    assert_eq!(response.requested_sleep_recovery, 1.0);
    let withdrawal = response.withdrawal.as_ref().unwrap();
    assert_eq!(
        withdrawal.direction_basis,
        WithdrawalDirectionBasis::LocalizedContact
    );
    assert!(!withdrawal.blocked);
    assert!(withdrawal.displacement.x < 0.0);
    assert_eq!(world.entity(body).unwrap().body_yaw, yaw);
    let after = world
        .organism_registry()
        .get(ORGANISM)
        .unwrap()
        .biochemistry();
    assert!(!after.body.sleeping);
    assert_eq!(
        *after,
        before
            .biochemistry()
            .advance(
                receipt.outcome_tick,
                response.applied_body_event,
                before.phenotype()
            )
            .unwrap()
    );
    world
        .editor_remove_object(world.entity_id("stationary-contact").unwrap())
        .unwrap();
    let position = world.entity(body).unwrap().position;
    let next = world
        .try_advance_tick_with_body_contact_candidate(&event(1.0, 0.0))
        .unwrap();
    assert!(next.responses.is_empty());
    assert_eq!(world.entity(body).unwrap().position, position);
    assert!(
        world
            .organism_registry()
            .get(ORGANISM)
            .unwrap()
            .biochemistry()
            .body
            .sleeping
    );
}

#[test]
fn body_only_candidate_blocked_damage_still_interrupts_physical_sleep() {
    let (mut world, body) = fixture();
    world
        .try_advance_tick_with_body_events(&event(1.0, 0.0))
        .unwrap();
    hazard(&mut world, 0.4);
    world
        .editor_spawn_object(WorldEditorSpawnSpec {
            label: "withdrawal-blocker".into(),
            kind: WorldObjectKind::Obstacle,
            organism_id: None,
            position: Vec3f::new(-0.05, 0.0, 0.0),
            nutrition: 0.0,
            hazard_pain: 0.0,
            radius: 0.1,
            token_id: None,
        })
        .unwrap();
    let before = world.organism_registry().get(ORGANISM).unwrap().clone();
    let receipt = world
        .try_advance_tick_with_body_contact_candidate(&event(1.0, 0.0))
        .unwrap();
    let response = &receipt.responses[0];
    let reflex = response.withdrawal.as_ref().unwrap();
    assert!(reflex.blocked);
    assert_eq!(reflex.displacement, Vec3f::ZERO);
    assert_eq!(world.entity(body).unwrap().position, Vec3f::ZERO);
    assert!(response.sleep_recovery_interrupted);
    assert_eq!(response.applied_body_event.sleep_recovery, 0.0);
    let after = world
        .organism_registry()
        .get(ORGANISM)
        .unwrap()
        .biochemistry();
    assert!(!after.body.sleeping);
    assert!(after.body.injury > before.biochemistry().body.injury);
}

#[test]
fn body_only_candidate_coincident_or_unlocalized_dose_uses_bounded_heading_fallback() {
    for contact in [false, true] {
        let (mut world, body) = fixture();
        if contact {
            hazard(&mut world, 0.4);
            world
                .editor_move_object(world.entity_id("stationary-contact").unwrap(), Vec3f::ZERO)
                .unwrap();
        }
        let yaw = world.entity(body).unwrap().body_yaw;
        let response = world
            .try_advance_tick_with_body_contact_candidate(&event(
                1.0,
                if contact { 0.0 } else { 0.4 },
            ))
            .unwrap();
        let reflex = response.responses[0].withdrawal.as_ref().unwrap();
        assert_eq!(
            reflex.direction_basis,
            WithdrawalDirectionBasis::AcuteDamageHeadingFallback
        );
        assert!(!reflex.blocked);
        assert!(reflex.displacement.x < 0.0 && reflex.displacement.x >= -0.1);
        assert_eq!(world.entity(body).unwrap().body_yaw, yaw);
    }
}

#[test]
fn body_only_candidate_quiet_sleep_and_chronic_injury_are_exact_noops() {
    for chronic in [false, true] {
        let (mut candidate, _) = fixture();
        if chronic {
            candidate
                .try_advance_tick_with_body_events(&event(0.0, 0.4))
                .unwrap();
        }
        let mut baseline = candidate.clone();
        baseline
            .try_advance_tick_with_body_events(&event(1.0, 0.0))
            .unwrap();
        let receipt = candidate
            .try_advance_tick_with_body_contact_candidate(&event(1.0, 0.0))
            .unwrap();
        assert!(receipt.responses.is_empty());
        assert_eq!(snapshot(&candidate), snapshot(&baseline));
    }
}

#[test]
fn body_only_candidate_restore_is_exact_and_never_repeats_an_action_interval() {
    let (mut world, body) = fixture();
    hazard(&mut world, 0.4);
    let mut restored = PortableSaveFile::from_json_str(&snapshot(&world))
        .unwrap()
        .restore_headless_world()
        .unwrap();
    let a = world
        .try_advance_tick_with_body_contact_candidate(&event(1.0, 0.0))
        .unwrap();
    let b = restored
        .try_advance_tick_with_body_contact_candidate(&event(1.0, 0.0))
        .unwrap();
    assert_eq!(a, b);
    assert_eq!(snapshot(&world), snapshot(&restored));
    let (mut world, _) = fixture();
    hazard(&mut world, 0.4);
    let command = ChannelCommand::new(
        MotorChannel::Posture,
        ActionId(1),
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
    let action = world.apply_registered_motor_bundle(&bundle, body).unwrap();
    let position = world.entity(body).unwrap().position;
    let interval = world
        .try_advance_tick_with_body_contact_candidate(&BTreeMap::new())
        .unwrap();
    assert!(interval.responses.is_empty());
    assert_eq!(world.entity(body).unwrap().position, position);
    assert_eq!(
        *world
            .organism_registry()
            .get(ORGANISM)
            .unwrap()
            .biochemistry(),
        action.biology_after
    );
}

#[test]
fn body_only_candidate_opposed_contacts_cancel_without_suppressing_damage() {
    let (mut world, body) = fixture();
    hazard(&mut world, 0.4);
    world
        .editor_spawn_object(WorldEditorSpawnSpec {
            label: "opposing-contact".into(),
            kind: WorldObjectKind::Hazard,
            organism_id: None,
            position: Vec3f::new(-0.2, 0.0, 0.0),
            nutrition: 0.0,
            hazard_pain: 0.4,
            radius: 0.6,
            token_id: None,
        })
        .unwrap();
    let receipt = world
        .try_advance_tick_with_body_contact_candidate(&event(1.0, 0.0))
        .unwrap();
    let response = &receipt.responses[0];
    let reflex = response.withdrawal.as_ref().unwrap();
    assert_eq!(
        reflex.direction_basis,
        WithdrawalDirectionBasis::LocalizedContact
    );
    assert_eq!(reflex.sources.len(), 2);
    assert!(reflex.blocked);
    assert_eq!(reflex.displacement, Vec3f::ZERO);
    assert_eq!(world.entity(body).unwrap().position, Vec3f::ZERO);
    assert_eq!(response.applied_body_event.damage, 0.4);
    assert!(response.sleep_recovery_interrupted);
}

#[test]
fn body_only_candidate_awake_and_sleeping_intervals_share_physical_reflex_geometry() {
    let (mut sleeping, body) = fixture();
    hazard(&mut sleeping, 0.4);
    let mut awake = sleeping.clone();
    let awake_interval = awake
        .try_advance_tick_with_body_contact_candidate(&event(0.0, 0.0))
        .unwrap();
    let sleeping_interval = sleeping
        .try_advance_tick_with_body_contact_candidate(&event(1.0, 0.0))
        .unwrap();
    assert_eq!(
        sleeping_interval.responses[0].withdrawal,
        awake_interval.responses[0].withdrawal
    );
    assert_eq!(
        sleeping_interval.responses[0].applied_body_event.damage,
        awake_interval.responses[0].applied_body_event.damage
    );
    assert_eq!(
        sleeping.entity(body).unwrap().position,
        awake.entity(body).unwrap().position
    );
}

#[test]
fn body_only_candidate_source_bound_accepts_256_and_rejects_257_atomically() {
    assert_eq!(ContactWithdrawalReceipt::MAX_SOURCES, 256);
    for count in [255, 256, 257] {
        let (mut world, body) = fixture();
        for id in 0..count {
            world
                .editor_spawn_object(WorldEditorSpawnSpec {
                    label: format!("source-{id}"),
                    kind: WorldObjectKind::Hazard,
                    organism_id: None,
                    position: Vec3f::new(0.2, 0.0, 0.0),
                    nutrition: 0.0,
                    hazard_pain: 0.4,
                    radius: 0.6,
                    token_id: None,
                })
                .unwrap();
        }
        let before = snapshot(&world);
        let outcome = world.try_advance_tick_with_body_contact_candidate(&event(1.0, 0.0));
        if count > ContactWithdrawalReceipt::MAX_SOURCES {
            assert_eq!(outcome, Err(ScaffoldContractError::InvalidDecisionEvidence));
            assert_eq!(snapshot(&world), before);
        } else {
            let interval = outcome.unwrap();
            let response = &interval.responses[0];
            let reflex = response.withdrawal.as_ref().unwrap();
            assert_eq!(reflex.sources.len(), count);
            assert!(!reflex.blocked);
            assert!(world.entity(body).unwrap().position.x < 0.0);
            assert_eq!(response.applied_body_event.damage, 0.4);
            assert_eq!(response.applied_body_event.sleep_recovery, 0.0);
        }
    }
}
