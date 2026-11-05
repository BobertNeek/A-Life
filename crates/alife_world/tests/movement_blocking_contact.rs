use alife_core::{
    ActionCommand, ActionKind, ActionTarget, Confidence, DurationTicks, Intensity, OrganismId,
    PhysicalContactKind, Vec3f,
};
use alife_world::{HeadlessActionIds, HeadlessScenarioBuilder, HeadlessWorld};

fn step(world: &mut HeadlessWorld, intensity: f32) -> alife_world::HeadlessActionResult {
    let command = ActionCommand::structured(
        OrganismId(1),
        HeadlessActionIds::STEP_FORWARD,
        ActionKind::Move,
        ActionTarget::new(None, None),
        Intensity::new(intensity).unwrap(),
        DurationTicks::new(1),
        Confidence::new(1.0).unwrap(),
        0,
        None,
        None,
        None,
    )
    .unwrap();
    world.apply_command(&command).unwrap()
}

#[test]
fn blocked_step_reports_the_first_rock_surface_independently_of_spawn_order() {
    for reverse in [false, true] {
        let mut builder =
            HeadlessScenarioBuilder::new(4242).agent("creature", OrganismId(1), Vec3f::ZERO);
        let mut rocks = [("far", 0.82), ("near", 0.77)];
        if reverse {
            rocks.reverse();
        }
        for (name, x) in rocks {
            builder = builder.obstacle(name, Vec3f::new(x, 0.0, 0.0), 0.75);
        }
        let mut world = builder.build().unwrap();
        let result = step(&mut world, 1.0);
        assert_eq!(
            result.execution.physical.contact,
            PhysicalContactKind::Blocked
        );
        assert_eq!(
            result.execution.physical.target_entity,
            world.entity_id("near")
        );
        assert_eq!(
            result.touched_entities,
            vec![world.entity_id("near").unwrap()]
        );
        assert_eq!(
            world
                .sensory_report(OrganismId(1), world.tick())
                .unwrap()
                .touched_entities,
            result.touched_entities,
        );
        assert_eq!(
            world
                .entity(world.entity_id("creature").unwrap())
                .unwrap()
                .position,
            Vec3f::ZERO
        );
    }
}

#[test]
fn first_surface_can_belong_to_the_rock_with_the_more_distant_center() {
    let mut world = HeadlessScenarioBuilder::new(4242)
        .agent("creature", OrganismId(1), Vec3f::ZERO)
        .obstacle("small", Vec3f::new(0.83, 0.0, 0.0), 0.75)
        .obstacle("large", Vec3f::new(1.20, 0.0, 0.0), 1.15)
        .build()
        .unwrap();
    let result = step(&mut world, 1.0);
    assert_eq!(
        result.execution.physical.contact,
        PhysicalContactKind::Blocked
    );
    assert_eq!(
        result.execution.physical.target_entity,
        world.entity_id("large")
    );
}

#[test]
fn coincident_contacts_keep_id_order_even_without_displacement() {
    for (center, intensity) in [(Vec3f::ZERO, 0.0), (Vec3f::new(0.8, 0.0, 0.0), 1.0)] {
        let mut world = HeadlessScenarioBuilder::new(4242)
            .agent("creature", OrganismId(1), Vec3f::ZERO)
            .obstacle("first", center, 0.75)
            .obstacle("second", center, 0.75)
            .build()
            .unwrap();
        let result = step(&mut world, intensity);
        assert_eq!(
            result.execution.physical.contact,
            PhysicalContactKind::Blocked
        );
        assert_eq!(
            result.execution.physical.target_entity,
            world.entity_id("first")
        );
        assert_eq!(result.execution.physical.displacement, Vec3f::ZERO);
    }
}

#[test]
fn tangent_and_clear_steps_keep_the_existing_collision_boundary() {
    for (height, expected) in [
        (0.75, PhysicalContactKind::Blocked),
        (0.76, PhysicalContactKind::Moved),
    ] {
        let mut world = HeadlessScenarioBuilder::new(4242)
            .agent("creature", OrganismId(1), Vec3f::ZERO)
            .obstacle("rock", Vec3f::new(0.05, height, 0.0), 0.75)
            .build()
            .unwrap();
        let result = step(&mut world, 1.0);
        assert_eq!(result.execution.physical.contact, expected);
        if expected == PhysicalContactKind::Blocked {
            assert_eq!(
                result.execution.physical.target_entity,
                world.entity_id("rock")
            );
        } else {
            assert!(result.execution.physical.displacement.x > 0.0);
        }
    }
}
