use alife_core::{
    ActionCommand, ActionId, ActionKind, CandidateObservationRef, Confidence, DurationTicks,
    GroundedObjectSlotV1, HomeostaticSnapshot, OrganismId, PerceptionFrameDraft,
    PhysicalContactKind, ReferenceActionFailure, SensorProfile, Vec3f, WorldEntityId,
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

const HOLDER: OrganismId = OrganismId(1);
const OBSERVER: OrganismId = OrganismId(2);

fn draft_with_index_parity(
    world: &mut HeadlessWorld,
    organism: OrganismId,
) -> PerceptionFrameDraft {
    let mut indexed_world = world.clone();
    let index = indexed_world.build_perception_batch_index().unwrap();
    let tick = world.tick();
    let frame = world
        .perception_frame_draft(
            organism,
            tick,
            SensorProfile::GroundedTerrainVisionV1,
            HomeostaticSnapshot::baseline(tick),
        )
        .unwrap();
    let indexed = indexed_world
        .perception_frame_draft_indexed(
            organism,
            tick,
            SensorProfile::GroundedTerrainVisionV1,
            HomeostaticSnapshot::baseline(tick),
            &index,
        )
        .unwrap();
    assert_eq!(frame, indexed);
    frame
}

fn eat_candidate(
    frame: &PerceptionFrameDraft,
    food: WorldEntityId,
) -> Option<&alife_core::ActionCandidate> {
    frame.candidates().iter().find(|candidate| {
        candidate.action_id == HeadlessActionIds::EAT && candidate.target.entity == Some(food)
    })
}

fn assert_owner_target(world: &mut HeadlessWorld, food: WorldEntityId) -> ActionCommand {
    let frame = draft_with_index_parity(world, HOLDER);
    let candidate =
        eat_candidate(&frame, food).expect("owner must retain its held-food Eat target");
    let CandidateObservationRef::ObjectSlot(index) = candidate.observation else {
        panic!("held target must use its real grounded object slot");
    };
    assert_eq!(
        frame.grounded_object_slots()[usize::from(index)].contact,
        1.0
    );
    assert_eq!(frame.sensory().channels.tactile_contact[2], 1.0);
    assert_eq!(
        candidate.target.position,
        Some(world.entity(food).unwrap().position)
    );
    candidate
        .to_command(HOLDER, Confidence::new(1.0).unwrap())
        .unwrap()
}

#[test]
fn default_radius_owner_can_select_two_real_eat_candidates_and_consume() {
    let mut world = HeadlessScenarioBuilder::new(60_031)
        .agent("holder", HOLDER, Vec3f::ZERO)
        .food("food", Vec3f::new(0.5, 0.0, 0.25), 0.6)
        .build()
        .unwrap();
    let holder = world.entity_id("holder").unwrap();
    let food = world.entity_id("food").unwrap();
    assert_eq!(world.entity(holder).unwrap().radius, 0.75);
    assert_eq!(world.entity(holder).unwrap().optical_opacity, 1.0);
    let frame = draft_with_index_parity(&mut world, HOLDER);
    let first_eat = eat_candidate(&frame, food)
        .unwrap()
        .to_command(HOLDER, Confidence::new(1.0).unwrap())
        .unwrap();
    let pickup = world.apply_command(&first_eat).unwrap();
    assert!(pickup.execution.succeeded);
    assert_eq!(
        pickup.execution.physical.contact,
        PhysicalContactKind::Touch
    );
    assert_eq!(pickup.body_event.nutrition, 0.0);
    assert_eq!(world.entity(food).unwrap().carried_by, Some(HOLDER));
    world.advance_tick();
    let second_eat = assert_owner_target(&mut world, food);
    let meal = world.apply_command(&second_eat).unwrap();
    assert!(meal.execution.succeeded);
    assert_eq!(
        meal.execution.physical.contact,
        PhysicalContactKind::Consumed
    );
    assert_eq!(meal.body_event.nutrition, 0.6);
    assert!(world.entity(food).unwrap().consumed);
}

#[test]
fn default_radius_owner_retains_small_held_target_at_both_head_limits_and_after_moving() {
    for food_radius in [0.1, 0.75] {
        for head_action in [HeadlessActionIds::LOOK_LEFT, HeadlessActionIds::LOOK_RIGHT] {
            let mut world = HeadlessScenarioBuilder::new(60_032)
                .agent("holder", HOLDER, Vec3f::ZERO)
                .build()
                .unwrap();
            let food = world
                .editor_spawn_object(WorldEditorSpawnSpec {
                    label: "food".into(),
                    kind: WorldObjectKind::Food,
                    organism_id: None,
                    position: Vec3f::new(0.5, 0.0, 0.0),
                    nutrition: 0.6,
                    hazard_pain: 0.0,
                    radius: food_radius,
                    token_id: None,
                })
                .unwrap();
            let grab = command(
                HOLDER,
                HeadlessActionIds::GRAB,
                ActionKind::Hold,
                Some(food),
            );
            assert!(world.apply_command(&grab).unwrap().execution.succeeded);
            for _ in 0..4 {
                let look = command(HOLDER, head_action, ActionKind::Look, None);
                assert!(world.apply_command(&look).unwrap().execution.succeeded);
                world.advance_tick();
                assert_owner_target(&mut world, food);
            }
            let turn = command(HOLDER, HeadlessActionIds::TURN_LEFT, ActionKind::Look, None);
            assert!(world.apply_command(&turn).unwrap().execution.succeeded);
            world.advance_tick();
            assert_owner_target(&mut world, food);
            let step = command(
                HOLDER,
                HeadlessActionIds::STEP_FORWARD,
                ActionKind::Move,
                None,
            );
            assert!(world.apply_command(&step).unwrap().execution.succeeded);
            world.advance_tick();
            let eat = assert_owner_target(&mut world, food);
            assert!(world.apply_command(&eat).unwrap().execution.succeeded);
            assert!(world.entity(food).unwrap().consumed);
        }
    }
}

#[test]
fn default_radius_carrier_occludes_another_observer_but_its_owner_can_eat() {
    let mut world = HeadlessScenarioBuilder::new(60_033)
        .agent("holder", HOLDER, Vec3f::ZERO)
        .agent("observer", OBSERVER, Vec3f::new(0.0, 0.0, -1.05))
        .food("food", Vec3f::new(0.5, 0.0, 0.25), 0.6)
        .build()
        .unwrap();
    let holder = world.entity_id("holder").unwrap();
    let food = world.entity_id("food").unwrap();
    assert_eq!(world.entity(holder).unwrap().radius, 0.75);
    let grab = command(
        HOLDER,
        HeadlessActionIds::GRAB,
        ActionKind::Hold,
        Some(food),
    );
    assert!(world.apply_command(&grab).unwrap().execution.succeeded);
    world.advance_tick();
    let eat = assert_owner_target(&mut world, food);
    let opaque = draft_with_index_parity(&mut world, OBSERVER);
    assert!(eat_candidate(&opaque, food).is_none());
    assert_eq!(opaque.sensory().channels.tactile_contact[2], 0.0);
    // The causal intervention changes only optical opacity, not possession or geometry.
    world.editor_set_optical_opacity(holder, 0.0).unwrap();
    let transparent = draft_with_index_parity(&mut world, OBSERVER);
    assert!(eat_candidate(&transparent, food).is_some());
    assert_owner_target(&mut world, food);
    world.editor_set_optical_opacity(holder, 1.0).unwrap();
    assert!(eat_candidate(&draft_with_index_parity(&mut world, OBSERVER), food).is_none());
    for action in [HeadlessActionIds::GRAB, HeadlessActionIds::EAT] {
        let attempt = command(OBSERVER, action, ActionKind::Interact, Some(food));
        let denied = world.apply_command(&attempt).unwrap();
        assert!(!denied.execution.succeeded);
        assert_eq!(
            denied.execution.failure,
            Some(ReferenceActionFailure::MissingAffordance)
        );
        assert_eq!(denied.body_event.nutrition, 0.0);
        assert_eq!(world.entity(food).unwrap().carried_by, Some(HOLDER));
    }
    assert!(world.apply_command(&eat).unwrap().execution.succeeded);
    assert!(world.entity(food).unwrap().consumed);
}

#[test]
fn third_party_body_occlusion_also_holds_with_nonoverlapping_creature_spheres() {
    let mut world = HeadlessScenarioBuilder::new(60_034)
        .agent("holder", HOLDER, Vec3f::ZERO)
        .agent("observer", OBSERVER, Vec3f::new(-2.0, 0.0, 0.0))
        .food("food", Vec3f::new(0.5, 0.0, 0.0), 0.6)
        .build()
        .unwrap();
    let holder = world.entity_id("holder").unwrap();
    let observer = world.entity_id("observer").unwrap();
    let food = world.entity_id("food").unwrap();
    assert!(2.0 > world.entity(holder).unwrap().radius + world.entity(observer).unwrap().radius);
    let grab = command(
        HOLDER,
        HeadlessActionIds::GRAB,
        ActionKind::Hold,
        Some(food),
    );
    assert!(world.apply_command(&grab).unwrap().execution.succeeded);
    let frame = draft_with_index_parity(&mut world, OBSERVER);
    assert!(eat_candidate(&frame, food).is_none());
    assert_owner_target(&mut world, food);
    world.editor_set_optical_opacity(holder, 0.0).unwrap();
    let frame = draft_with_index_parity(&mut world, OBSERVER);
    assert!(eat_candidate(&frame, food).is_some());
    assert_eq!(world.entity(food).unwrap().carried_by, Some(HOLDER));
}
