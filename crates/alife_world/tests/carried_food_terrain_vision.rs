use alife_core::{
    ActionCommand, ActionId, ActionKind, CandidateObservationRef, Confidence, DurationTicks,
    GroundedObjectSlotV1, HomeostaticSnapshot, OrganismId, ReferenceActionFailure, SensorProfile,
    Vec3f, WorldEntityId,
};
use alife_world::{
    HeadlessActionIds, HeadlessScenarioBuilder, HeadlessWorld, WorldEditorSpawnSpec,
    WorldObjectKind,
};

fn command(
    organism: OrganismId,
    action: ActionId,
    kind: ActionKind,
    target: Option<WorldEntityId>,
) -> ActionCommand {
    let mut command = ActionCommand::new(
        organism,
        kind,
        target,
        Confidence::new(0.9).unwrap(),
        DurationTicks::new(1),
    )
    .unwrap();
    command.action_id = action;
    command
}

fn observed_food(
    world: &mut HeadlessWorld,
    observer: OrganismId,
    food: WorldEntityId,
) -> (GroundedObjectSlotV1, Vec3f) {
    let tick = world.tick();
    let frame = world
        .perception_frame_draft(
            observer,
            tick,
            SensorProfile::GroundedTerrainVisionV1,
            HomeostaticSnapshot::baseline(tick),
        )
        .unwrap();
    let candidate = frame
        .candidates()
        .iter()
        .find(|candidate| {
            candidate.action_id == HeadlessActionIds::APPROACH
                && candidate.target.entity == Some(food)
        })
        .expect("food must survive real TerrainVision filtering");
    let CandidateObservationRef::ObjectSlot(index) = candidate.observation else {
        panic!("food candidate must reference its grounded observation");
    };
    (
        frame.grounded_object_slots()[usize::from(index)],
        candidate.target.position.unwrap(),
    )
}

#[test]
fn other_creature_tracks_carried_food_and_cannot_grab_or_eat_it() {
    let holder = OrganismId(1);
    let observer = OrganismId(2);
    let observer_position = Vec3f::new(0.0, 0.0, -1.05);
    let ground_position = Vec3f::new(0.5, 0.0, 0.25);
    let mut world = HeadlessScenarioBuilder::new(60_030)
        .agent("observer", observer, observer_position)
        .food("food", ground_position, 0.6)
        .build()
        .unwrap();
    // Opaque A has a clear 0.5 grip; B is farther than their radius sum (1.0).
    let holder_entity = world
        .editor_spawn_object(WorldEditorSpawnSpec {
            label: "holder".into(),
            kind: WorldObjectKind::Agent,
            organism_id: Some(holder),
            position: Vec3f::ZERO,
            nutrition: 0.0,
            hazard_pain: 0.0,
            radius: 0.25,
            token_id: None,
        })
        .unwrap();
    let food = world.entity_id("food").unwrap();
    assert_eq!(world.entity(holder_entity).unwrap().optical_opacity, 1.0);
    let (ground, seen_ground) = observed_food(&mut world, observer, food);
    assert_eq!(seen_ground, ground_position);

    let grab = command(
        holder,
        HeadlessActionIds::GRAB,
        ActionKind::Hold,
        Some(food),
    );
    assert!(world.apply_command(&grab).unwrap().execution.succeeded);
    world.advance_tick();
    let (held, seen_held) = observed_food(&mut world, observer, food);
    assert_eq!(seen_held, Vec3f::new(0.5, 0.0, 0.0));
    assert_eq!(held.tracked_object_id, ground.tracked_object_id);

    let step = command(
        holder,
        HeadlessActionIds::STEP_FORWARD,
        ActionKind::Move,
        None,
    );
    assert!(world.apply_command(&step).unwrap().execution.succeeded);
    let body = world.entity(holder_entity).unwrap();
    assert_eq!(body.position, Vec3f::new(0.1, 0.0, 0.0));
    world.advance_tick();
    let (moved, seen_moved) = observed_food(&mut world, observer, food);
    assert_eq!(seen_moved, Vec3f::new(0.6, 0.0, 0.0));
    assert_eq!(seen_moved, world.entity(food).unwrap().position);
    assert_eq!(moved.tracked_object_id, ground.tracked_object_id);
    assert_ne!(moved.bearing, ground.bearing);
    let dx = seen_moved.x - observer_position.x;
    let dz = seen_moved.z - observer_position.z;
    for (actual, expected) in moved
        .bearing
        .into_iter()
        .zip([dz / dx.hypot(dz), dx / dx.hypot(dz)])
    {
        assert!((actual - expected).abs() < 1e-6);
    }

    for action in [HeadlessActionIds::GRAB, HeadlessActionIds::EAT] {
        let attempt = command(observer, action, ActionKind::Interact, Some(food));
        let denied = world.apply_command(&attempt).unwrap();
        assert!(!denied.execution.succeeded);
        assert_eq!(
            denied.execution.failure,
            Some(ReferenceActionFailure::MissingAffordance)
        );
        assert_eq!(denied.body_event.nutrition, 0.0);
        assert_eq!(world.entity(food).unwrap().carried_by, Some(holder));
        assert_eq!(world.entity(food).unwrap().position, seen_moved);
        assert!(!world.entity(food).unwrap().consumed);
    }
    // Release only ownership; independent retries start at the same food pose.
    assert!(world.apply_command(&grab).unwrap().execution.succeeded);
    assert_eq!(world.entity(food).unwrap().carried_by, None);
    assert_eq!(world.entity(food).unwrap().position, seen_moved);
    for action in [HeadlessActionIds::GRAB, HeadlessActionIds::EAT] {
        let mut released = world.clone();
        let attempt = command(observer, action, ActionKind::Interact, Some(food));
        let allowed = released.apply_command(&attempt).unwrap();
        assert!(allowed.execution.succeeded);
        assert_eq!(released.entity(food).unwrap().carried_by, Some(observer));
        assert!(!released.entity(food).unwrap().consumed);
    }
}
