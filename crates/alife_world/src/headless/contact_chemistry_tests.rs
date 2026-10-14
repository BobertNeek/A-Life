use super::*;
use crate::{create_canonical_new_game, CanonicalNewGameConfig, PortableSaveFile, RuntimeConfig};
use alife_core::{
    channel_command_for_action, BrainCapacityClass, BrainScaleTier, CreatureGenome,
    ExperienceSequenceId, FoundationWeightAsset, MeasuredPhysiologyTransition,
};

const OWN: OrganismId = OrganismId(1);
const PEER: OrganismId = OrganismId(2);

fn founders() -> HeadlessWorld {
    let asset =
        FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    let game =
        create_canonical_new_game(&CanonicalNewGameConfig::phase3(92_108, 2).unwrap(), &asset)
            .unwrap();
    let mut world = game.world;
    place(&mut world, OWN, Vec3f::new(-100.0, 0.0, 0.0));
    place(&mut world, PEER, Vec3f::new(-98.0, 0.0, 0.0));
    assert!(world
        .objects
        .values()
        .filter(|o| o.kind == WorldObjectKind::Agent)
        .all(|o| o.social_affinity == 0.0));
    world
}

fn entity(world: &HeadlessWorld, organism: OrganismId) -> WorldEntityId {
    world
        .organism_registry
        .get(organism)
        .unwrap()
        .world_entity_id()
}

fn place(world: &mut HeadlessWorld, organism: OrganismId, position: Vec3f) {
    let id = entity(world, organism);
    world.editor_move_object(id, position).unwrap();
}

fn touch(world: &mut HeadlessWorld) {
    let position = world.objects[&entity(world, OWN).raw()].position;
    place(world, PEER, position);
}

fn biology(world: &HeadlessWorld) -> BiochemistryState {
    *world.organism_registry.get(OWN).unwrap().biochemistry()
}

#[test]
fn same_tick_death_preserves_contact_exposure_across_order_and_action_paths() {
    for expiring in [OWN, PEER] {
        let survivor = if expiring == OWN { PEER } else { OWN };
        let mut base = founders();
        touch(&mut base);
        let next = PassiveBodyUpkeepPolicy::maximum_lifespan_ticks(
            base.organism_registry.get(expiring).unwrap().phenotype(),
        );
        base.tick = Tick(next - 1);
        for id in [OWN, PEER] {
            let record = base.organism_registry.get(id).unwrap();
            let age = if id == expiring { next - 1 } else { 600 };
            let replacement = crate::WorldOrganismRecord::new(
                id,
                record.world_entity_id(),
                record.genome().clone(),
                record.phenotype().clone(),
                BiochemistryState::new_with_age(record.phenotype(), base.tick, Tick(age)).unwrap(),
                Tick(base.tick.raw() - age),
            )
            .unwrap();
            base.organism_registry
                .replace_existing_exact(replacement)
                .unwrap();
        }
        let mut passive = base.clone();
        passive.try_advance_tick().unwrap();
        let mut idle = base.clone();
        let receipt = idle
            .apply_registered_command(
                &HeadlessWorldCommand::idle(survivor).unwrap(),
                entity(&idle, survivor),
                Tick(next),
            )
            .unwrap();
        assert_eq!(
            receipt.action_result.body_event.social_contact,
            PEER_TOUCH_PER_INTERVAL
        );
        idle.try_advance_tick().unwrap();
        let observed = |world: &HeadlessWorld| {
            let h = world
                .organism_registry
                .get(survivor)
                .unwrap()
                .biochemistry()
                .homeostasis;
            (
                h.drives.loneliness,
                h.hormones.oxytocin,
                h.hormones.serotonin,
            )
        };
        assert_eq!(observed(&passive), observed(&idle), "expiring {expiring:?}");
        let mut staged = base.clone();
        staged
            .try_advance_tick_with_body_events_in_staged_tick(&BTreeMap::new())
            .unwrap();
        assert_eq!(
            staged.canonical_signature_digest().unwrap(),
            passive.canonical_signature_digest().unwrap()
        );
        assert!(!passive
            .organism_registry
            .get(expiring)
            .unwrap()
            .lifecycle()
            .is_alive());

        // A corpse contributes no exposure in the following biological interval.
        let mut separated = passive.clone();
        place(&mut separated, expiring, Vec3f::new(100.0, 0.0, 0.0));
        passive.try_advance_tick().unwrap();
        separated.try_advance_tick().unwrap();
        assert_eq!(observed(&passive), observed(&separated));

        let mut failed = base.clone();
        let before = failed.canonical_signature_digest().unwrap();
        failed.injected_tick_late_failure_after_first_organism = true;
        assert!(failed.try_advance_tick().is_err());
        failed.injected_tick_late_failure_after_first_organism = false;
        assert_eq!(failed.canonical_signature_digest().unwrap(), before);
        failed.try_advance_tick().unwrap();
        assert_eq!(observed(&failed), observed(&idle));
    }
}

fn bundle(world: &HeadlessWorld, duplicate_channels: bool, duration: u32) -> MotorCommandBundle {
    let peer = entity(world, PEER);
    let command = HeadlessWorldCommand::approach(OWN, peer).unwrap();
    let mut channel = channel_command_for_action(MotorChannel::Locomotion, &command).unwrap();
    channel.duration_ticks = alife_core::DurationTicks::new(duration);
    let mut channels = vec![channel];
    if duplicate_channels {
        let inspect = HeadlessWorldCommand::structured(
            OWN,
            ActionKind::Inspect.canonical_id(),
            ActionKind::Inspect,
            Some(peer),
            None,
        )
        .unwrap();
        channels.push(channel_command_for_action(MotorChannel::Posture, &inspect).unwrap());
    }
    MotorCommandBundle::new(OWN, ExperienceSequenceId(1), world.tick(), channels).unwrap()
}

#[test]
fn grounded_peer_identity_uses_observed_slots_without_static_affinity() {
    for profile in [
        SensorProfile::GroundedObjectSlotsV1,
        SensorProfile::GroundedTerrainVisionV1,
    ] {
        let mut world = founders();
        let own_position = world.objects[&entity(&world, OWN).raw()].position;
        place(
            &mut world,
            PEER,
            Vec3f::new(own_position.x + 0.4, own_position.y, own_position.z),
        );
        let peer = entity(&world, PEER);
        world.objects.get_mut(&peer.raw()).unwrap().social_affinity = 1.0;
        let homeostasis = biology(&world).homeostasis;
        let draft = world
            .perception_frame_draft(OWN, world.tick(), profile, homeostasis)
            .unwrap();
        let observed = draft.sensory().social_context.nearest_agents[0].unwrap();
        assert_eq!(observed.agent_id, PEER);
        assert_eq!(observed.body_entity, Some(peer));
        assert_eq!(observed.affinity, SignedValence::ZERO);
        assert!(draft
            .candidates()
            .iter()
            .any(|c| c.target.entity == Some(peer)));
        assert!(draft.sensory().social_context.nearest_agents[1..]
            .iter()
            .all(Option::is_none));
        place(&mut world, PEER, Vec3f::new(100.0, 0.0, 0.0));
        let draft = world
            .perception_frame_draft(OWN, world.tick(), profile, homeostasis)
            .unwrap();
        assert!(draft
            .sensory()
            .social_context
            .nearest_agents
            .iter()
            .all(Option::is_none));
    }
}

#[test]
fn zero_affinity_founders_receive_stationary_touch_and_natural_loneliness_relief() {
    let mut separated = founders();
    let newborn = biology(&separated);
    for _ in 0..600 {
        separated.try_advance_tick().unwrap();
    }
    let before = biology(&separated);
    assert!(before.homeostasis.drives.loneliness > newborn.homeostasis.drives.loneliness);
    let mut touching = separated.clone();
    touch(&mut touching);
    touching.try_advance_tick().unwrap();
    separated.try_advance_tick().unwrap();
    let after = biology(&touching);
    let control = biology(&separated);
    let measured = MeasuredPhysiologyTransition::new(before, after).unwrap();
    assert!(measured.homeostatic_delta.drives.loneliness < 0.0);
    assert!(measured.homeostatic_improvement() > 0.0, "{measured:?}");
    assert!(after.homeostasis.drives.loneliness < control.homeostasis.drives.loneliness);
    assert!(after.homeostasis.hormones.oxytocin > control.homeostasis.hormones.oxytocin);
    assert_eq!(
        after.biochemical_work(),
        control.biochemical_work(),
        "contact adds no biochemical graph work"
    );
    assert_eq!(before.value_profile().hormones[..9], [0.0; 9]);

    // The ordinary New Game graph can receive care at birth, but a drive at its
    // floor cannot supply relief credit. Chemistry by itself is not affection.
    let mut fresh = founders();
    touch(&mut fresh);
    fresh.try_advance_tick().unwrap();
    assert_eq!(biology(&fresh).homeostasis.drives.loneliness, 0.0);
    assert!(biology(&fresh).homeostasis.hormones.oxytocin > newborn.homeostasis.hormones.oxytocin);
}

#[test]
fn touch_is_independent_of_static_affinity_and_bounded_per_canonical_interval() {
    let mut base = founders();
    touch(&mut base);
    let mut expected = None;
    for affinity in [-1.0, 0.0, 1.0] {
        for (channels, duration) in [(false, 1), (true, 1), (true, 40)] {
            let mut world = base.clone();
            let peer = entity(&world, PEER);
            world.objects.get_mut(&peer.raw()).unwrap().social_affinity = affinity;
            let mut dose = 0.0;
            for _ in 0..WORLD_TICKS_PER_SECOND {
                let commands = bundle(&world, channels, duration);
                let own = entity(&world, OWN);
                let receipt = world.apply_registered_motor_bundle(&commands, own).unwrap();
                assert_eq!(receipt.body_event.social_contact, PEER_TOUCH_PER_INTERVAL);
                assert_eq!(receipt.joint.execution.displacement, Vec3f::ZERO);
                dose += receipt.body_event.social_contact;
                // Tick finalization must not dose an organism that already acted.
                let after = biology(&world);
                world.try_advance_tick().unwrap();
                assert_eq!(biology(&world), after);
            }
            assert!((dose - 0.25).abs() < 1e-6);
            let state = biology(&world).homeostasis;
            let signal = (
                state.drives.loneliness,
                state.hormones.oxytocin,
                state.hormones.serotonin,
            );
            if let Some(expected) = expected {
                assert_eq!(signal, expected);
            } else {
                expected = Some(signal);
            }
        }
    }
    let mut passive = base.clone();
    for _ in 0..WORLD_TICKS_PER_SECOND {
        passive.try_advance_tick().unwrap();
    }
    let h = biology(&passive).homeostasis;
    assert_eq!(
        Some((
            h.drives.loneliness,
            h.hormones.oxytocin,
            h.hormones.serotonin
        )),
        expected
    );
    // Legacy registered commands reach the same biology source.
    let mut legacy = base;
    for _ in 0..WORLD_TICKS_PER_SECOND {
        let own = entity(&legacy, OWN);
        let next = Tick(legacy.tick().raw() + 1);
        let receipt = legacy
            .apply_registered_command(&HeadlessWorldCommand::idle(OWN).unwrap(), own, next)
            .unwrap();
        assert_eq!(
            receipt.action_result.body_event.social_contact,
            PEER_TOUCH_PER_INTERVAL
        );
        legacy.try_advance_tick().unwrap();
    }
    let h = biology(&legacy).homeostasis;
    assert_eq!(
        Some((
            h.drives.loneliness,
            h.hormones.oxytocin,
            h.hormones.serotonin
        )),
        expected
    );
}

#[test]
fn separation_solids_and_dead_bodies_stop_contact_and_unpleasant_events_remain_negative() {
    let mut world = founders();
    for _ in 0..600 {
        world.try_advance_tick().unwrap();
    }
    touch(&mut world);
    let own = entity(&world, OWN);
    let position = world.objects[&own.raw()].position;
    let peer = entity(&world, PEER);
    world
        .editor_spawn_object(WorldEditorSpawnSpec {
            label: "contact-injury".into(),
            kind: WorldObjectKind::Hazard,
            organism_id: None,
            position,
            nutrition: 0.0,
            hazard_pain: 0.4,
            radius: 0.75,
            token_id: None,
        })
        .unwrap();
    let receipt = world
        .apply_registered_motor_bundle(&bundle(&world, false, 1), own)
        .unwrap();
    assert_eq!(receipt.body_event.social_contact, PEER_TOUCH_PER_INTERVAL);
    assert!(
        MeasuredPhysiologyTransition::new(receipt.biology_before, receipt.biology_after)
            .unwrap()
            .aversive_value()
            > 0.0
    );
    assert!(
        MeasuredPhysiologyTransition::new(receipt.biology_before, receipt.biology_after)
            .unwrap()
            .homeostatic_improvement()
            < 0.0
    );
    world.try_advance_tick().unwrap();
    place(&mut world, PEER, Vec3f::new(-98.0, 0.0, 0.0));
    let receipt = world
        .apply_registered_motor_bundle(&bundle(&world, false, 1), own)
        .unwrap();
    assert_eq!(receipt.body_event.social_contact, 0.0);
    world.try_advance_tick().unwrap();
    touch(&mut world);
    world
        .organism_registry
        .mark_dead(PEER, world.tick())
        .unwrap();
    assert!(!world.body_contacts_at(own, position).1);
    world
        .organism_registry
        .get(PEER)
        .unwrap()
        .validate_contract()
        .unwrap();
    // Physical obstruction prevents contact even when positions are close.
    let mut blocked = founders();
    place(&mut blocked, PEER, Vec3f::new(-99.5, 0.0, 0.0));
    blocked
        .editor_spawn_object(WorldEditorSpawnSpec {
            label: "solid".into(),
            kind: WorldObjectKind::Obstacle,
            organism_id: None,
            position: Vec3f::new(-99.75, 0.0, 0.0),
            nutrition: 0.0,
            hazard_pain: 0.0,
            radius: 0.1,
            token_id: None,
        })
        .unwrap();
    assert!(!blocked.body_contacts_at(entity(&blocked, OWN), position).1);
    assert_eq!(peer, entity(&world, PEER));
}

#[test]
fn current_save_restore_and_rollback_preserve_contact_continuation_without_new_state() {
    let mut world = founders();
    touch(&mut world);
    for _ in 0..7 {
        world.try_advance_tick().unwrap();
    }
    let mut game = create_canonical_new_game(
        &CanonicalNewGameConfig::phase3(world.seed(), 2).unwrap(),
        &FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1).unwrap(),
    )
    .unwrap();
    for creature in &mut game.creatures {
        let record = world.organism_registry.get(creature.organism_id).unwrap();
        creature.mind.tick = world.tick();
        creature.mind.homeostasis = record.biochemistry().homeostasis;
        creature.development_tick = record.biochemistry().development.last_update_tick;
    }
    let save = PortableSaveFile::from_headless_world(
        "contact",
        &world,
        RuntimeConfig::deterministic_default(world.seed(), BrainScaleTier::Nano512),
        crate::AssetManifest::empty(),
        game.creatures,
    )
    .unwrap();
    let mut restored = PortableSaveFile::from_json_str(&serde_json::to_string(&save).unwrap())
        .unwrap()
        .restore_headless_world()
        .unwrap();
    assert_eq!(
        world.canonical_signature_digest().unwrap(),
        restored.canonical_signature_digest().unwrap()
    );
    for _ in 0..3 {
        world.try_advance_tick().unwrap();
        restored.try_advance_tick().unwrap();
        assert_eq!(
            world.canonical_signature_digest().unwrap(),
            restored.canonical_signature_digest().unwrap()
        );
    }
    let before = world.canonical_signature_digest().unwrap();
    let mut failed = world.clone();
    failed.inject_post_action_failure();
    assert!(failed
        .apply_registered_motor_bundle(&bundle(&failed, true, 1), entity(&failed, OWN))
        .is_err());
    failed.injected_post_action_failure = false;
    assert_eq!(failed.canonical_signature_digest().unwrap(), before);
    let commands = bundle(&world, true, 1);
    assert_eq!(
        world
            .apply_registered_motor_bundle(&commands, entity(&world, OWN))
            .unwrap(),
        failed
            .apply_registered_motor_bundle(&commands, entity(&failed, OWN))
            .unwrap()
    );
}

#[test]
fn genetic_newborn_gets_touch_without_inheriting_personal_affinity_or_parent_chemistry() {
    let mut world = founders();
    let first = world.organism_registry.get(OWN).unwrap();
    let second = world.organism_registry.get(PEER).unwrap();
    let genome = CreatureGenome::reproduce(first.genome(), second.genome(), 0xCAFE).unwrap();
    assert_eq!(
        genome.foundation.brain_class_id,
        BrainCapacityClass::N512_ID
    );
    let phenotype = genome.express().unwrap();
    let child = OrganismId(3);
    let position = world.objects[&entity(&world, OWN).raw()].position;
    let body = world
        .spawn_social_agent("newborn", child, position, 0.0)
        .unwrap();
    world
        .register_organism_record(
            WorldOrganismRecord::newborn(child, body, genome, phenotype, world.tick()).unwrap(),
        )
        .unwrap();
    let idle = HeadlessWorldCommand::idle(child).unwrap();
    let receipt = world
        .apply_registered_command(&idle, body, Tick(1))
        .unwrap();
    assert_eq!(world.entity(body).unwrap().social_affinity, 0.0);
    assert_eq!(
        receipt.action_result.body_event.social_contact,
        PEER_TOUCH_PER_INTERVAL
    );
}

#[test]
#[ignore = "manual CPU endpoint cost comparison; no biology or neural execution"]
fn contact_endpoint_cost_against_previous_hazard_scan() {
    use std::{hint::black_box, time::Instant};
    // Exact previous hazard-only endpoint implementation, retained only here
    // for a same-process comparison. This is not a population simulation gate.
    fn previous(world: &HeadlessWorld, position: Vec3f) -> Option<(WorldEntityId, f32)> {
        world
            .objects
            .values()
            .filter(|object| {
                object.kind == WorldObjectKind::Hazard
                    && !object.consumed
                    && distance(object.position, position) <= object.radius
            })
            .min_by(|left, right| {
                distance(left.position, position)
                    .total_cmp(&distance(right.position, position))
                    .then_with(|| left.id.raw().cmp(&right.id.raw()))
            })
            .map(|object| (object.id, object.hazard_pain))
    }
    for peers in [1, 10, 100, 500] {
        let mut world = HeadlessWorld::new(92_108);
        let own = world
            .spawn_social_agent("own", OWN, Vec3f::ZERO, 0.0)
            .unwrap();
        for index in 0..peers {
            world
                .spawn_social_agent(
                    &format!("peer-{index}"),
                    OrganismId(index + 2),
                    Vec3f::new(index as f32 * 2.0 + 2.0, 0.0, 0.0),
                    0.0,
                )
                .unwrap();
        }
        let started = Instant::now();
        for _ in 0..20_000 {
            black_box(previous(black_box(&world), black_box(Vec3f::ZERO)));
        }
        let old = started.elapsed().as_nanos() / 20_000;
        let started = Instant::now();
        for _ in 0..20_000 {
            black_box(world.body_contacts_at(black_box(own), black_box(Vec3f::ZERO)));
        }
        let new = started.elapsed().as_nanos() / 20_000;
        println!("endpoint_cpu peers={peers} samples=20000 previous_ns={old} contact_ns={new}");
    }
}
