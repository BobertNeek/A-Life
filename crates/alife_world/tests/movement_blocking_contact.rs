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
        let before = ["creature", "near", "far"].map(|label| {
            world
                .entity(world.entity_id(label).unwrap())
                .unwrap()
                .clone()
        });
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
        for object in before {
            assert_eq!(world.entity(object.id), Some(&object));
        }
    }
}

#[test]
fn starting_on_a_surface_keeps_contact_at_the_start_of_the_step() {
    for intensity in [0.0, 1.0] {
        let mut world = HeadlessScenarioBuilder::new(4242)
            .agent("creature", OrganismId(1), Vec3f::ZERO)
            .obstacle("far", Vec3f::new(0.82, 0.0, 0.0), 0.75)
            .obstacle("touching", Vec3f::new(0.75, 0.0, 0.0), 0.75)
            .build()
            .unwrap();
        let result = step(&mut world, intensity);
        let touching = world.entity_id("touching").unwrap();
        assert_eq!(
            result.execution.physical.contact,
            PhysicalContactKind::Blocked
        );
        assert_eq!(result.execution.physical.target_entity, Some(touching));
        assert_eq!(result.execution.physical.displacement, Vec3f::ZERO);
        assert_eq!(result.touched_entities, vec![touching]);
        assert_eq!(
            world
                .sensory_report(OrganismId(1), world.tick())
                .unwrap()
                .touched_entities,
            vec![touching]
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

fn assert_tangent_order(earlier_x: f32, radius: f32) {
    // Establish through the public API that both surfaces are independently
    // admitted by the existing movement collision predicate.
    for (name, center, obstacle_radius) in [
        ("earlier", Vec3f::new(earlier_x, 0.0, 0.0), 0.75),
        ("tangent", Vec3f::new(0.05, radius, 0.0), radius),
    ] {
        let mut world = HeadlessScenarioBuilder::new(4242)
            .agent("creature", OrganismId(1), Vec3f::ZERO)
            .obstacle(name, center, obstacle_radius)
            .build()
            .unwrap();
        let result = step(&mut world, 1.0);
        assert_eq!(
            result.execution.physical.contact,
            PhysicalContactKind::Blocked
        );
        assert_eq!(
            result.execution.physical.target_entity,
            world.entity_id(name)
        );
    }
    for reverse in [false, true] {
        let mut builder =
            HeadlessScenarioBuilder::new(4242).agent("creature", OrganismId(1), Vec3f::ZERO);
        let mut rocks = [
            ("earlier", Vec3f::new(earlier_x, 0.0, 0.0)),
            ("tangent", Vec3f::new(0.05, radius, 0.0)),
        ];
        if reverse {
            rocks.reverse();
        }
        for (name, center) in rocks {
            let obstacle_radius = if name == "tangent" { radius } else { 0.75 };
            builder = builder.obstacle(name, center, obstacle_radius);
        }
        let mut world = builder.build().unwrap();
        let result = step(&mut world, 1.0);
        let earlier = world.entity_id("earlier").unwrap();
        assert_eq!(
            result.execution.physical.contact,
            PhysicalContactKind::Blocked
        );
        assert_eq!(result.execution.physical.target_entity, Some(earlier));
        assert_eq!(result.touched_entities, vec![earlier]);
        assert_eq!(result.execution.physical.displacement, Vec3f::ZERO);
    }
}

#[test]
fn near_tangent_rounding_does_not_move_contact_ahead_of_an_earlier_surface() {
    assert_tangent_order(0.799975, 0.75);
}

#[test]
fn large_tangent_radius_does_not_move_contact_ahead_of_an_earlier_surface() {
    assert_tangent_order(0.795, 32.0);
}

#[test]
fn coincident_contacts_keep_id_order_even_without_displacement() {
    let inside = Vec3f::ZERO;
    let surface = Vec3f::new(0.75, 0.0, 0.0);
    let ahead = Vec3f::new(0.8, 0.0, 0.0);
    for (centers, intensity) in [
        ([inside, inside], 0.0),
        ([ahead, ahead], 1.0),
        ([inside, surface], 1.0),
        ([surface, inside], 1.0),
    ] {
        let mut world = HeadlessScenarioBuilder::new(4242)
            .agent("creature", OrganismId(1), Vec3f::ZERO)
            .obstacle("first", centers[0], 0.75)
            .obstacle("second", centers[1], 0.75)
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
