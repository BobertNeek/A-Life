use alife_bevy_adapter::{
    bevy_quat_to_core, bevy_transform_to_core_pose, bevy_vec3_to_core, core_pose_to_bevy_transform,
    core_vec3_to_bevy, execute_action_command, gather_sensory_from_observed, ActionAdapterContext,
    AdapterScheduleTrace, AdapterWorldTick, AlifeBevyAdapterPlugin, AlifeBevyAdapterSet,
    BevyActionFailure, BevyActionKind, BevyEntityMap, ObservedBevyEntity, TargetAdapterState,
    ACTION_APPROACH, ACTION_EAT, ACTION_FLEE,
};
use alife_core::{
    ActionCommand, ActionId, ActionKind, ActionTarget, AffordanceBits, Confidence, DurationTicks,
    Intensity, OrganismId, PhysicalContactKind, ReferenceActionFailure, Tick, Vec3f, WorldEntityId,
};
use bevy::prelude::{App, Entity, MinimalPlugins, Quat, Transform, Vec3};

fn organism() -> OrganismId {
    OrganismId(21)
}

fn target_id(raw: u64) -> WorldEntityId {
    WorldEntityId(raw)
}

fn command(
    action_id: ActionId,
    kind: ActionKind,
    target_entity: Option<WorldEntityId>,
    target_position: Option<Vec3f>,
) -> ActionCommand {
    ActionCommand::structured(
        organism(),
        action_id,
        kind,
        ActionTarget::new(target_entity, target_position),
        Intensity::new(1.0).unwrap(),
        DurationTicks::new(1),
        Confidence::new(0.9).unwrap(),
        0,
        None,
        None,
        None,
    )
    .unwrap()
}

fn spawn_entity(app: &mut App) -> Entity {
    app.world_mut().spawn_empty().id()
}

#[test]
fn entity_map_allocates_bidirectional_stable_ids_without_core_engine_leakage() {
    let mut app = App::new();
    let first = spawn_entity(&mut app);
    let second = spawn_entity(&mut app);
    let mut map = BevyEntityMap::with_next_id(40);

    let first_id = map.get_or_allocate(first).unwrap();
    let second_id = map.get_or_allocate(second).unwrap();

    assert_eq!(first_id, WorldEntityId(40));
    assert_eq!(second_id, WorldEntityId(41));
    assert_eq!(map.world_id(first), Some(first_id));
    assert_eq!(map.bevy_entity(second_id), Some(second));
    assert_eq!(map.get_or_allocate(first).unwrap(), first_id);

    assert_eq!(map.remove_by_entity(first), Some(first_id));
    assert_eq!(map.world_id(first), None);
    assert_eq!(map.bevy_entity(first_id), None);

    let mut exhausted = BevyEntityMap::with_next_id(u64::MAX);
    let max_entity = spawn_entity(&mut app);
    let overflow_entity = spawn_entity(&mut app);
    assert_eq!(
        exhausted.get_or_allocate(max_entity).unwrap(),
        WorldEntityId(u64::MAX)
    );
    assert!(exhausted.get_or_allocate(overflow_entity).is_err());
}

#[test]
fn core_math_conversions_are_explicit_and_round_trip() {
    let bevy_position = Vec3::new(1.25, -2.5, 3.75);
    let core_position = bevy_vec3_to_core(bevy_position).unwrap();
    assert_eq!(core_position, Vec3f::new(1.25, -2.5, 3.75));
    assert_eq!(core_vec3_to_bevy(core_position).unwrap(), bevy_position);

    let bevy_rotation = Quat::from_xyzw(0.0, 0.0, 0.70710677, 0.70710677);
    let core_rotation = bevy_quat_to_core(bevy_rotation).unwrap();
    assert_eq!(core_rotation.z, bevy_rotation.z);

    let transform = Transform::from_translation(bevy_position).with_rotation(bevy_rotation);
    let pose = bevy_transform_to_core_pose(transform).unwrap();
    let round_trip = core_pose_to_bevy_transform(pose).unwrap();
    assert_eq!(round_trip.translation, bevy_position);
    assert_eq!(round_trip.rotation, bevy_rotation);
}

#[test]
fn sensory_conversion_maps_affordances_tokens_and_social_context_by_stable_id() {
    let mut app = App::new();
    let food_entity = spawn_entity(&mut app);
    let hazard_entity = spawn_entity(&mut app);
    let social_entity = spawn_entity(&mut app);
    let token_entity = spawn_entity(&mut app);

    let observed = vec![
        ObservedBevyEntity::new(
            food_entity,
            target_id(2),
            Vec3::new(1.0, 0.0, 0.0),
            AffordanceBits::FOOD,
        )
        .with_nutrition(0.7),
        ObservedBevyEntity::new(
            hazard_entity,
            target_id(3),
            Vec3::new(0.0, 0.5, 0.0),
            AffordanceBits::HAZARD,
        )
        .with_hazard_pain(0.8),
        ObservedBevyEntity::new(
            social_entity,
            target_id(4),
            Vec3::new(2.0, 0.0, 0.0),
            AffordanceBits::SOCIAL_AGENT,
        )
        .with_organism(OrganismId(77)),
        ObservedBevyEntity::new(
            token_entity,
            target_id(5),
            Vec3::new(1.5, 0.0, 0.0),
            AffordanceBits::GLYPH_OR_WRITING,
        )
        .with_token(42),
    ];

    let snapshot =
        gather_sensory_from_observed(organism(), Tick::ZERO, target_id(1), Vec3::ZERO, &observed)
            .unwrap();

    assert!(snapshot
        .channels
        .nearby_affordances
        .contains(AffordanceBits::FOOD));
    assert!(snapshot
        .channels
        .nearby_affordances
        .contains(AffordanceBits::HAZARD));
    assert!(snapshot.channels.visual_affordance[0] > 0.0);
    assert!(snapshot.channels.pain_signal.raw() > 0.0);
    assert_eq!(
        snapshot.context_streams.vocal_tokens[0]
            .unwrap()
            .source_entity,
        Some(target_id(5))
    );
    assert_eq!(
        snapshot.context_streams.vocal_tokens[0].unwrap().token_id,
        42
    );
    assert_eq!(
        snapshot.social_context.nearest_agents[0].unwrap().agent_id,
        OrganismId(77)
    );
}

#[test]
fn action_adapter_translates_move_approach_flee_and_rest() {
    let mut app = App::new();
    let actor = spawn_entity(&mut app);
    let target = spawn_entity(&mut app);
    let context = ActionAdapterContext::new(actor, target_id(1), Vec3::ZERO).with_target(
        TargetAdapterState::new(
            target,
            target_id(2),
            Vec3::new(4.0, 0.0, 0.0),
            AffordanceBits::FOOD,
        ),
    );

    let moved = execute_action_command(
        &command(
            ActionKind::Move.canonical_id(),
            ActionKind::Move,
            None,
            Some(Vec3f::new(0.5, 0.0, 0.0)),
        ),
        &context,
    )
    .unwrap();
    assert_eq!(moved.plan.kind, BevyActionKind::Move);
    assert!(moved.plan.displacement.x > 0.0);

    let approached = execute_action_command(
        &command(ACTION_APPROACH, ActionKind::Move, Some(target_id(2)), None),
        &context,
    )
    .unwrap();
    assert_eq!(approached.plan.kind, BevyActionKind::Approach);
    assert!(approached.plan.displacement.x > 0.0);

    let fled = execute_action_command(
        &command(ACTION_FLEE, ActionKind::Move, Some(target_id(2)), None),
        &context,
    )
    .unwrap();
    assert_eq!(fled.plan.kind, BevyActionKind::Flee);
    assert!(fled.plan.displacement.x < 0.0);

    let rested = execute_action_command(
        &command(
            ActionKind::Rest.canonical_id(),
            ActionKind::Rest,
            None,
            None,
        ),
        &context,
    )
    .unwrap();
    assert_eq!(rested.plan.kind, BevyActionKind::Rest);
    assert!(rested.plan.rest_requested);
}

#[test]
fn action_adapter_returns_core_failures_for_missing_targets_and_affordances() {
    let mut app = App::new();
    let actor = spawn_entity(&mut app);
    let target = spawn_entity(&mut app);
    let context = ActionAdapterContext::new(actor, target_id(1), Vec3::ZERO).with_target(
        TargetAdapterState::new(
            target,
            target_id(2),
            Vec3::new(1.0, 0.0, 0.0),
            AffordanceBits::RESOURCE,
        ),
    );

    let missing = execute_action_command(
        &command(
            ActionKind::Inspect.canonical_id(),
            ActionKind::Inspect,
            Some(target_id(999)),
            None,
        ),
        &context,
    )
    .unwrap();
    assert_eq!(
        missing.failure,
        Some(BevyActionFailure::MissingTarget(target_id(999)))
    );
    assert_eq!(
        missing.execution.failure,
        Some(ReferenceActionFailure::ActionRejected)
    );
    assert_eq!(
        missing.execution.physical.contact,
        PhysicalContactKind::Blocked
    );

    let missing_food = execute_action_command(
        &command(ACTION_EAT, ActionKind::Interact, Some(target_id(2)), None),
        &context,
    )
    .unwrap();
    assert_eq!(
        missing_food.failure,
        Some(BevyActionFailure::MissingAffordance {
            target: target_id(2),
            required: AffordanceBits::FOOD,
        })
    );
    assert_eq!(
        missing_food.execution.failure,
        Some(ReferenceActionFailure::MissingAffordance)
    );
}

#[test]
fn plugin_registers_ordered_causal_system_sets() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AlifeBevyAdapterPlugin);

    app.update();

    let trace = app.world().resource::<AdapterScheduleTrace>();
    assert_eq!(
        trace.stages(),
        &[
            AlifeBevyAdapterSet::GatherSensory,
            AlifeBevyAdapterSet::CpuBrainTick,
            AlifeBevyAdapterSet::ExecuteAction,
            AlifeBevyAdapterSet::MeasureOutcome,
            AlifeBevyAdapterSet::SealPatch,
        ]
    );

    assert_eq!(
        app.world().resource::<AdapterWorldTick>().current(),
        Tick::new(1)
    );
}

#[cfg(feature = "avian3d")]
#[test]
fn avian3d_feature_converts_adapter_motion_to_linear_velocity() {
    let plan = alife_bevy_adapter::BevyActionPlan {
        actor: Entity::PLACEHOLDER,
        organism_id: organism(),
        action_id: ActionKind::Move.canonical_id(),
        kind: BevyActionKind::Move,
        target: None,
        target_world_id: None,
        displacement: Vec3::new(0.25, 0.0, 0.0),
        rest_requested: false,
    };

    let velocity = alife_bevy_adapter::avian3d::plan_to_linear_velocity(&plan, 4.0);
    assert_eq!(velocity.0, Vec3::new(1.0, 0.0, 0.0));
}
