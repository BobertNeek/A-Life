use super::*;
use crate::{create_canonical_new_game, CanonicalNewGameConfig, PlayerInteractionV1};
use alife_core::{DurationTicks, ExperienceSequenceId, FoundationWeightAsset};

fn world() -> HeadlessWorld {
    let foundation =
        FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    create_canonical_new_game(
        &CanonicalNewGameConfig::phase3(2116, 1).unwrap(),
        &foundation,
    )
    .unwrap()
    .world
}

fn queued() -> (HeadlessWorld, OrganismId, WorldEntityId) {
    let mut world = world();
    let (id, entity) = world.organism_entity_ids()[0];
    let p = world.entity(entity).unwrap().position;
    for kind in [
        PlayerInteractionV1::Praise,
        PlayerInteractionV1::Scold,
        PlayerInteractionV1::Pet,
        PlayerInteractionV1::Slap,
    ] {
        world.queue_player_interaction_v1(id, p, kind).unwrap();
    }
    (world, id, entity)
}

fn bundle(world: &HeadlessWorld, id: OrganismId) -> MotorCommandBundle {
    let command = ChannelCommand::new(
        MotorChannel::Posture,
        ActionKind::Idle.canonical_id(),
        None,
        Vec3f::ZERO,
        Intensity::new(1.0).unwrap(),
        DurationTicks::new(1),
        0.0,
        Confidence::new(0.9).unwrap(),
        0,
    )
    .unwrap();
    MotorCommandBundle::new(
        id,
        ExperienceSequenceId::new(1).unwrap(),
        world.tick(),
        vec![command],
    )
    .unwrap()
}

#[test]
fn four_player_doses_apply_once_through_motor_bundle() {
    let (mut world, id, entity) = queued();
    let receipt = world
        .apply_registered_motor_bundle(&bundle(&world, id), entity)
        .unwrap();
    assert_eq!(receipt.body_event.player_reward, 1.0);
    assert!(receipt.body_event.social_disapproval_v1 > 0.0);
    assert_eq!(receipt.body_event.social_contact, 0.25);
    assert_eq!(receipt.body_event.damage, 0.02);
    assert!(!world.pending_player_care.contains_key(&id.raw()));
}

#[test]
fn late_failures_restore_pending_doses_and_full_world() {
    let (world, id, entity) = queued();
    let mut action = world.clone();
    action.inject_post_action_failure();
    let before = action.save_state();
    assert!(action
        .apply_registered_command(&HeadlessWorldCommand::idle(id).unwrap(), entity, Tick(1))
        .is_err());
    assert_eq!(action.save_state(), before);
    let mut motor = world.clone();
    motor.inject_post_action_failure();
    let before = motor.save_state();
    assert!(motor
        .apply_registered_motor_bundle(&bundle(&motor, id), entity)
        .is_err());
    assert_eq!(motor.save_state(), before);
    let mut idle = world;
    idle.inject_tick_late_failure_after_first_organism_for_test();
    let before = idle.save_state();
    assert!(idle.try_advance_tick().is_err());
    assert_eq!(idle.save_state(), before);
}
