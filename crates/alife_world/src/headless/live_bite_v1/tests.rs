use super::*;
use alife_core::{
    channel_command_for_action, BrainCapacityClass, BrainScaleTier, CreatureGenome,
    ExperienceSequenceId, FoundationGeneticIdentity, OrganKind,
};

const ACTOR: OrganismId = OrganismId(1);
const RECIPIENT: OrganismId = OrganismId(2);
const BYSTANDER: OrganismId = OrganismId(3);

// Component biology fixtures, not a brain trial or a replacement foundation.
fn world(seed: u64) -> HeadlessWorld {
    let mut world = HeadlessScenarioBuilder::new(seed)
        .agent("actor", ACTOR, Vec3f::ZERO)
        .agent("recipient", RECIPIENT, Vec3f::new(0.5, 0.0, 0.0))
        .agent("bystander", BYSTANDER, Vec3f::new(4.0, 0.0, 0.0))
        .food("food-control", Vec3f::new(0.0, 0.5, 0.0), 0.4)
        .build()
        .unwrap();
    for (id, label) in [
        (ACTOR, "actor"),
        (RECIPIENT, "recipient"),
        (BYSTANDER, "bystander"),
    ] {
        let genome = CreatureGenome::early_mammal_founder(
            seed + id.raw(),
            FoundationGeneticIdentity::new(100 + id.raw(), 1, 7, BrainCapacityClass::N512_ID)
                .unwrap(),
        )
        .unwrap();
        world
            .register_organism_record(
                WorldOrganismRecord::newborn(
                    id,
                    world.entity_id(label).unwrap(),
                    genome.clone(),
                    genome.express().unwrap(),
                    Tick::ZERO,
                )
                .unwrap(),
            )
            .unwrap();
        world
            .organism_registry
            .with_biology_mut(id, |b| Ok(b.body.set_energy(0.5)?))
            .unwrap();
    }
    world
}

fn entity(world: &HeadlessWorld, id: OrganismId) -> WorldEntityId {
    world.organism_registry.get(id).unwrap().world_entity_id()
}

fn bundle(
    world: &HeadlessWorld,
    id: OrganismId,
    channel: MotorChannel,
    action: ActionCommand,
) -> MotorCommandBundle {
    MotorCommandBundle::new(
        id,
        ExperienceSequenceId::new(world.tick.raw() + 1).unwrap(),
        world.tick,
        vec![channel_command_for_action(channel, &action).unwrap()],
    )
    .unwrap()
}

fn still(world: &HeadlessWorld, id: OrganismId) -> MotorCommandBundle {
    bundle(
        world,
        id,
        MotorChannel::Manipulation,
        HeadlessWorldCommand::structured(
            id,
            HeadlessActionIds::NO_MANIPULATION,
            ActionKind::Interact,
            None,
            None,
        )
        .unwrap(),
    )
}

fn eat(world: &HeadlessWorld) -> MotorCommandBundle {
    bundle(
        world,
        ACTOR,
        MotorChannel::Manipulation,
        HeadlessWorldCommand::eat(ACTOR, entity(world, RECIPIENT)).unwrap(),
    )
}

fn pair(world: &mut HeadlessWorld) -> LiveBitePairReceiptV1 {
    world
        .apply_registered_live_bite_pair_v1(
            &eat(world),
            entity(world, ACTOR),
            &still(world, RECIPIENT),
            entity(world, RECIPIENT),
        )
        .unwrap()
}

fn reserve_sum(biology: &BiochemistryState) -> f32 {
    biology.body.organs().iter().map(|organ| organ.energy).sum()
}

fn ordinary_pair(
    world: &mut HeadlessWorld,
    actor: &MotorCommandBundle,
    recipient: &MotorCommandBundle,
) {
    world
        .apply_registered_motor_bundle(actor, entity(world, ACTOR))
        .unwrap();
    world
        .apply_registered_motor_bundle(recipient, entity(world, RECIPIENT))
        .unwrap();
}

#[test]
fn default_eat_on_living_agent_keeps_original_touch_and_no_recipient_effect() {
    let mut world = world(91_700);
    let before = world.organism_registry.get(RECIPIENT).unwrap().clone();
    let receipt = world
        .apply_registered_motor_bundle(&eat(&world), entity(&world, ACTOR))
        .unwrap();
    assert!(!receipt.succeeded);
    assert_eq!(receipt.body_event.nutrition, 0.0);
    assert_eq!(receipt.joint.execution.contact, PhysicalContactKind::Touch);
    assert_eq!(world.organism_registry.get(RECIPIENT).unwrap(), &before);
    assert!(world.ingestion_observations.is_empty());
    assert!(world.live_bite_pair_v1.is_none());
}

#[test]
fn registered_live_bite_has_target_bound_donor_loss_injury_and_nutrition_once() {
    let mut world = world(91_701);
    // Express the ordinary nociceptive chemistry; no neural development claim.
    for _ in 0..600 {
        world.try_advance_tick().unwrap();
    }
    for id in [ACTOR, RECIPIENT] {
        world
            .organism_registry
            .with_biology_mut(id, |b| Ok(b.body.set_energy(0.5)?))
            .unwrap();
    }
    let bystander = world.organism_registry.get(BYSTANDER).unwrap().clone();
    let result = pair(&mut world);
    let transfer = result.transfer.unwrap();
    assert_eq!(
        (transfer.actor, transfer.recipient, transfer.target),
        (ACTOR, RECIPIENT, entity(&world, RECIPIENT))
    );
    assert!(transfer.reserve_debit > 0.0 && transfer.reserve_debit <= MAX_RESERVE_DEBIT);
    assert_eq!(transfer.nutrition, transfer.reserve_debit);
    assert_eq!(result.actor.body_event.nutrition, transfer.nutrition);
    assert_eq!(result.actor.body_event.energy, -BITE_EFFORT);
    assert_eq!(result.recipient.body_event.damage, transfer.damage);
    assert!(result.actor.succeeded && result.recipient.succeeded);
    assert_eq!(world.tick.raw(), 600);
    assert_eq!(result.actor.biology_after.tick, Tick(601));
    assert_eq!(result.recipient.biology_after.tick, Tick(601));
    let donor_removed = reserve_sum(&result.recipient_biology_before)
        - reserve_sum(&result.recipient.biology_before);
    assert!(
        (donor_removed - transfer.reserve_debit * alife_core::ORGAN_KIND_COUNT as f32).abs() < 1e-5
    );
    assert!(
        result.recipient.biology_after.body.injury > result.recipient_biology_before.body.injury
    );
    assert!(
        result.recipient.biology_after.body.health < result.recipient_biology_before.body.health
    );
    assert!(
        result.recipient.biology_after.homeostasis.drives.pain
            > result.recipient_biology_before.homeostasis.drives.pain
    );
    assert!(result.actor.biology_after.body.energy > result.actor.biology_before.body.energy);
    eprintln!("live-bite component: debit={:.8} nutrition={:.8} actor_energy={:.8}->{:.8} recipient_health={:.8}->{:.8} recipient_pain={:.8}->{:.8}",
        transfer.reserve_debit, transfer.nutrition,
        result.actor.biology_before.body.energy, result.actor.biology_after.body.energy,
        result.recipient_biology_before.body.health, result.recipient.biology_after.body.health,
        result.recipient_biology_before.homeostasis.drives.pain, result.recipient.biology_after.homeostasis.drives.pain);
    for (id, receipt) in [(ACTOR, &result.actor), (RECIPIENT, &result.recipient)] {
        let phenotype = world.organism_registry.get(id).unwrap().phenotype();
        assert_eq!(
            receipt.biology_after,
            receipt
                .biology_before
                .advance(receipt.outcome_tick, receipt.body_event, phenotype)
                .unwrap()
        );
    }
    assert_eq!(world.organism_registry.get(BYSTANDER).unwrap(), &bystander);
    assert!(!world.entity(transfer.target).unwrap().consumed);
    assert!(world.live_bite_pair_v1.is_none());
    let after = world.save_state();
    assert!(world
        .apply_registered_live_bite_pair_v1(
            &eat(&world),
            entity(&world, ACTOR),
            &still(&world, RECIPIENT),
            entity(&world, RECIPIENT)
        )
        .is_err());
    assert_eq!(world.save_state(), after);
    world.try_advance_tick().unwrap();
    assert_eq!(
        *world.organism_registry.get(ACTOR).unwrap().biochemistry(),
        result.actor.biology_after
    );
    assert_eq!(
        *world
            .organism_registry
            .get(RECIPIENT)
            .unwrap()
            .biochemistry(),
        result.recipient.biology_after
    );
}

#[test]
fn range_blocked_contact_and_empty_reserves_match_the_ordinary_failed_route() {
    for fault in 0..4 {
        let mut actual = world(91_702 + fault);
        let recipient_entity = entity(&actual, RECIPIENT);
        match fault {
            0 => actual
                .editor_move_object(recipient_entity, Vec3f::new(EAT_RADIUS + 0.1, 0.0, 0.0))
                .unwrap(),
            1 => {
                actual = HeadlessScenarioBuilder {
                    world: actual,
                    rng: DeterministicRng::new(91_703),
                    error: None,
                }
                .obstacle("solid", Vec3f::new(0.25, 0.0, 0.0), 0.1)
                .build()
                .unwrap();
                let solid = actual.entity_id("solid").unwrap();
                actual.editor_set_optical_opacity(solid, 0.0).unwrap();
            }
            2 => {
                actual.terrain = Some(
                    crate::WorldTerrain::new(
                        crate::TerrainData {
                            width: 2,
                            depth: 2,
                            origin_x: 0.0,
                            origin_z: 0.0,
                            spacing: 10.0,
                            heights: vec![0.0; 4],
                            obstacles: vec![[0.2, -0.1, 0.3, 0.1, -1.0, 1.0]],
                            water_level: None,
                        },
                        crate::LocomotionLimits::default(),
                    )
                    .unwrap(),
                )
            }
            _ => actual
                .organism_registry
                .with_biology_mut(RECIPIENT, |b| Ok(b.body.set_energy(0.0)?))
                .unwrap(),
        }
        let mut expected = actual.clone();
        let actor = eat(&actual);
        let recipient = still(&actual, RECIPIENT);
        ordinary_pair(&mut expected, &actor, &recipient);
        let receipt = pair(&mut actual);
        assert!(receipt.transfer.is_none());
        assert!(!receipt.actor.succeeded);
        assert_eq!(receipt.actor.body_event.nutrition, 0.0);
        assert_eq!(actual.save_state(), expected.save_state());
        assert_eq!(
            actual.canonical_signature_digest().unwrap(),
            expected.canonical_signature_digest().unwrap()
        );
    }
}

#[test]
fn wrong_identity_self_unregistered_dead_and_already_advanced_pairs_reject_atomically() {
    for fault in 0..6 {
        let mut world = world(91_710 + fault);
        let mut actor = eat(&world);
        let mut recipient = still(&world, RECIPIENT);
        let mut target = entity(&world, RECIPIENT);
        match fault {
            0 => {
                actor.channels[0].target = Some(alife_core::ActionTarget::new(
                    Some(entity(&world, BYSTANDER)),
                    None,
                ))
            }
            1 => {
                recipient = still(&world, ACTOR);
                target = entity(&world, ACTOR);
            }
            2 => recipient.organism_id = OrganismId(999),
            3 => world
                .organism_registry
                .mark_dead(RECIPIENT, Tick::ZERO)
                .unwrap(),
            4 => {
                world
                    .apply_registered_motor_bundle(&recipient, target)
                    .unwrap();
            }
            _ => world.objects.get_mut(&target.raw()).unwrap().organism_id = Some(BYSTANDER),
        }
        let before = world.save_state();
        assert!(
            world
                .apply_registered_live_bite_pair_v1(
                    &actor,
                    entity(&world, ACTOR),
                    &recipient,
                    target
                )
                .is_err(),
            "fault {fault}"
        );
        assert_eq!(world.save_state(), before, "fault {fault}");
    }
}

#[test]
fn actor_and_recipient_late_failures_restore_both_organisms_and_contact_state() {
    for after_recipient in [false, true] {
        let mut world = world(91_720);
        if after_recipient {
            world.injected_live_bite_late_failure = true;
        } else {
            world.injected_post_action_failure = true;
        }
        let before = world.save_state();
        let before_signature = world.canonical_signature_digest().unwrap();
        assert!(world
            .apply_registered_live_bite_pair_v1(
                &eat(&world),
                entity(&world, ACTOR),
                &still(&world, RECIPIENT),
                entity(&world, RECIPIENT)
            )
            .is_err());
        assert_eq!(world.save_state(), before);
        assert_eq!(
            world.canonical_signature_digest().unwrap(),
            before_signature
        );
        assert!(world.ingestion_observations.is_empty());
        assert!(world.live_bite_pair_v1.is_none());
    }
}

#[test]
fn finite_reserves_bound_total_nutrition_and_stop_further_extraction() {
    for seed in [91_730, 91_731, 91_732] {
        let mut world = world(seed);
        let initial = world
            .organism_registry
            .get(RECIPIENT)
            .unwrap()
            .biochemistry()
            .body
            .energy;
        let mut nutrition = 0.0;
        let mut donor_loss = 0.0;
        let mut empty = false;
        for _ in 0..32 {
            let result = pair(&mut world);
            if let Some(transfer) = result.transfer {
                assert!(!empty);
                nutrition += transfer.nutrition;
                donor_loss += transfer.reserve_debit;
                // Reserve conversion, clipping, repair and upkeep can only lose
                // energy in this fixture. Include every organ of both bodies.
                let before = reserve_sum(&result.actor.biology_before)
                    + reserve_sum(&result.recipient_biology_before);
                let after = reserve_sum(&result.actor.biology_after)
                    + reserve_sum(&result.recipient.biology_after);
                assert!(after <= before + 1e-5);
            } else {
                empty = true;
                assert_eq!(result.actor.body_event.nutrition, 0.0);
            }
            assert!(nutrition <= initial + 1e-5);
            assert!((nutrition - donor_loss).abs() < 1e-6);
            let depleted = result.recipient.biology_after.body.energy == 0.0;
            world.try_advance_tick().unwrap();
            if depleted {
                empty = true;
                assert!(!world
                    .organism_registry
                    .get(RECIPIENT)
                    .unwrap()
                    .lifecycle()
                    .is_alive());
                let before = world.save_state();
                assert!(world
                    .apply_registered_live_bite_pair_v1(
                        &eat(&world),
                        entity(&world, ACTOR),
                        &still(&world, RECIPIENT),
                        entity(&world, RECIPIENT)
                    )
                    .is_err());
                assert_eq!(world.save_state(), before);
                break;
            }
        }
        assert!(empty);
    }
}

#[test]
fn cold_portable_save_restores_pair_effects_without_replaying_nutrition() {
    let mut world = world(91_740);
    pair(&mut world);
    // Save before the world boundary, with both biology clocks already +1.
    let save = crate::PortableSaveFile::from_headless_world(
        "live-bite-component",
        &world,
        crate::RuntimeConfig::deterministic_default(world.seed(), BrainScaleTier::Nano512),
        crate::AssetManifest::empty(),
        Vec::new(),
    )
    .unwrap();
    let json = serde_json::to_string(&save).unwrap();
    let mut restored = crate::PortableSaveFile::from_json_str(&json)
        .unwrap()
        .restore_headless_world()
        .unwrap();
    assert_eq!(world.save_state(), restored.save_state());
    assert!(restored.live_bite_pair_v1.is_none());
    for current in [&mut world, &mut restored] {
        let before = current.save_state();
        assert!(current
            .apply_registered_live_bite_pair_v1(
                &eat(current),
                entity(current, ACTOR),
                &still(current, RECIPIENT),
                entity(current, RECIPIENT)
            )
            .is_err());
        assert_eq!(current.save_state(), before);
        current.try_advance_tick().unwrap();
    }
    assert_eq!(world.save_state(), restored.save_state());
    let actor_before = *restored
        .organism_registry
        .get(ACTOR)
        .unwrap()
        .biochemistry();
    ordinary_pair(&mut restored, &eat(&world), &still(&world, RECIPIENT));
    assert_eq!(
        restored
            .last_action_result
            .as_ref()
            .unwrap()
            .body_event
            .nutrition,
        0.0
    );
    assert!(
        restored
            .organism_registry
            .get(ACTOR)
            .unwrap()
            .biochemistry()
            .body
            .energy
            <= actor_before.body.energy
    );
    let actor = eat(&world);
    let recipient = still(&world, RECIPIENT);
    ordinary_pair(&mut world, &actor, &recipient);
    assert_eq!(world.save_state(), restored.save_state());
}

#[test]
fn moving_biological_agent_keeps_its_submitted_movement_and_one_biology_interval() {
    for seed in [91_750, 91_751, 91_752] {
        let mut world = world(seed);
        let before_position = world.entity(entity(&world, RECIPIENT)).unwrap().position;
        let movement = bundle(
            &world,
            RECIPIENT,
            MotorChannel::Locomotion,
            HeadlessWorldCommand::structured(
                RECIPIENT,
                HeadlessActionIds::STEP_FORWARD,
                ActionKind::Move,
                None,
                None,
            )
            .unwrap(),
        );
        let actor_still = still(&world, ACTOR);
        ordinary_pair(&mut world, &actor_still, &movement);
        world.try_advance_tick().unwrap();
        let position = world.entity(entity(&world, RECIPIENT)).unwrap().position;
        assert!(position.x > before_position.x);
        assert!(
            world
                .entity(entity(&world, RECIPIENT))
                .unwrap()
                .grounded_physical
                .velocity
                .x
                > 0.0
        );
        let mut moving = movement;
        moving.tick = world.tick();
        let actor = eat(&world);
        let result = world
            .apply_registered_live_bite_pair_v1(
                &actor,
                entity(&world, ACTOR),
                &moving,
                entity(&world, RECIPIENT),
            )
            .unwrap();
        assert!(result.transfer.is_some());
        assert_eq!(result.recipient.bundle, moving);
        assert!(result.recipient.joint.execution.displacement.x > 0.0);
        assert!(world.entity(entity(&world, RECIPIENT)).unwrap().position.x > position.x);
        assert_eq!(result.actor.outcome_tick, result.recipient.outcome_tick);
        assert!(
            result
                .recipient
                .biology_after
                .body
                .organ(OrganKind::Circulatory)
                .damage
                > 0.0
        );
        assert_eq!(
            world.entity(entity(&world, RECIPIENT)).unwrap().kind,
            WorldObjectKind::Agent
        );
    }
}

#[test]
fn incoming_wound_interrupts_rest_without_turning_into_a_social_signal() {
    let mut world = world(91_760);
    let rest = bundle(
        &world,
        RECIPIENT,
        MotorChannel::Posture,
        HeadlessWorldCommand::rest(RECIPIENT).unwrap(),
    );
    let result = world
        .apply_registered_live_bite_pair_v1(
            &eat(&world),
            entity(&world, ACTOR),
            &rest,
            entity(&world, RECIPIENT),
        )
        .unwrap();
    assert!(result.recipient.body_event.damage > 0.0);
    assert_eq!(result.recipient.body_event.sleep_recovery, 0.0);
    for body in [result.actor.body_event, result.recipient.body_event] {
        assert_eq!(body.player_reward, 0.0);
        assert_eq!(body.social_disapproval_v1, 0.0);
    }
}

#[test]
fn bounded_dose_uses_actual_available_material() {
    for (reserve, intensity) in [(0.01, 1.0), (0.5, 0.0), (0.5, 0.5), (1e-6, 1.0)] {
        let mut world = world(91_770);
        world
            .organism_registry
            .with_biology_mut(RECIPIENT, |b| {
                b.body.set_energy(reserve)?;
                // Retain an already depleted finite source rather than a fixed meal.
                b.body.debit_energy_pro_rata(reserve * 0.1)?;
                Ok(())
            })
            .unwrap();
        let mut actor = eat(&world);
        actor.channels[0].intensity = Intensity::new(intensity).unwrap();
        let result = world
            .apply_registered_live_bite_pair_v1(
                &actor,
                entity(&world, ACTOR),
                &still(&world, RECIPIENT),
                entity(&world, RECIPIENT),
            )
            .unwrap();
        if intensity == 0.0 {
            assert!(result.transfer.is_none());
            assert_eq!(result.actor.body_event.nutrition, 0.0);
        } else {
            let transfer = result.transfer.unwrap();
            assert!(transfer.reserve_debit <= result.recipient_biology_before.body.energy);
            let embodied_intensity = result.actor.channel_receipts[0].command.intensity.raw();
            assert!(transfer.reserve_debit <= MAX_RESERVE_DEBIT * embodied_intensity + 1e-6);
            assert!(transfer.reserve_debit <= MAX_RESERVE_DEBIT);
            assert!(result.recipient.biology_before.body.energy >= 0.0);
        }
    }
}

#[test]
fn paired_bite_requires_the_registered_ingestion_effector() {
    let mut world = world(91_780);
    let mut record = world.organism_registry.get(ACTOR).unwrap().clone();
    let mut value = serde_json::to_value(record.embodiment()).unwrap();
    let ports = value["effectors"].as_array_mut().unwrap();
    let index = ports
        .iter()
        .position(|port| port["capability"] == "Ingestion")
        .unwrap();
    let start: usize = ports[..index]
        .iter()
        .map(|port| port["command_lanes"].as_u64().unwrap() as usize)
        .sum();
    let width = ports[index]["command_lanes"].as_u64().unwrap() as usize;
    ports.remove(index);
    value["effector_controllability"]
        .as_array_mut()
        .unwrap()
        .drain(start..start + width);
    value["revision"] = serde_json::json!(record.embodiment().revision() + 1);
    record
        .replace_embodiment_state(serde_json::from_value(value).unwrap())
        .unwrap();
    world.replace_organism_record_exact(record).unwrap();
    let before = world.save_state();
    assert_eq!(
        world.apply_registered_live_bite_pair_v1(
            &eat(&world),
            entity(&world, ACTOR),
            &still(&world, RECIPIENT),
            entity(&world, RECIPIENT)
        ),
        Err(HeadlessMotorTransactionError::UnsupportedChannel(
            MotorChannel::Manipulation
        ))
    );
    assert_eq!(world.save_state(), before);
}
