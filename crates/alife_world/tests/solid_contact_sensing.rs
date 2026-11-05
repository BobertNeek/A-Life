use alife_core::{
    ActionCommand, ActionKind, ActionTarget, Confidence, DurationTicks, HomeostaticSnapshot,
    Intensity, OrganismId, PhysicalContactKind, SensorProfile, Tick, Vec3f,
};
use alife_world::{HeadlessScenarioBuilder, LocomotionLimits, TerrainData, WorldTerrain};

#[test]
fn a_solid_wall_blocks_surface_temperature_contact_but_not_scent() {
    let mut unobstructed_scent = None;
    for (wall, temperature) in [(false, 0.8_f32), (true, 0.8), (false, -0.7), (true, -0.7)] {
        let mut world = HeadlessScenarioBuilder::new(4243)
            .agent("creature", OrganismId(1), Vec3f::ZERO)
            .food("warm-object", Vec3f::new(0.75, 0.0, 0.0), 0.5)
            .build()
            .unwrap();
        let target = world.entity_id("warm-object").unwrap();
        let mut properties = world.entity(target).unwrap().grounded_physical;
        properties.surface_temperature = temperature;
        properties.chemical = [0.6, 0.0, 0.0];
        world
            .set_grounded_physical_properties(target, properties)
            .unwrap();
        world
            .enable_terrain_for_new_game(
                WorldTerrain::new(
                    TerrainData {
                        width: 3,
                        depth: 3,
                        origin_x: -2.0,
                        origin_z: -2.0,
                        spacing: 2.0,
                        heights: vec![0.0; 9],
                        obstacles: if wall {
                            vec![[0.37, -1.0, 0.38, 1.0, -0.5, 2.0]]
                        } else {
                            vec![]
                        },
                        water_level: None,
                    },
                    LocomotionLimits::default(),
                )
                .unwrap(),
                Vec3f::ZERO,
            )
            .unwrap();
        assert_eq!(
            world.entity(target).unwrap().position,
            Vec3f::new(0.75, 0.0, 0.0)
        );
        let frame = world
            .perception_frame(
                OrganismId(1),
                Tick::ZERO,
                SensorProfile::GroundedTerrainVisionV1,
                HomeostaticSnapshot::baseline(Tick::ZERO),
            )
            .unwrap();
        let channels = &frame.sensory().channels;
        assert!(channels.smell_chemistry[0] > 0.0);
        if wall {
            assert_eq!(Some(channels.smell_chemistry), unobstructed_scent);
        } else {
            unobstructed_scent = Some(channels.smell_chemistry);
        }
        assert_eq!(
            channels.tactile_contact[3],
            if wall { 0.0 } else { temperature.max(0.0) },
            "wall={wall}"
        );
        assert_eq!(
            channels.tactile_contact[4],
            if wall { 0.0 } else { (-temperature).max(0.0) },
            "wall={wall}"
        );
        assert_eq!(
            channels.tactile_contact[0],
            if wall { 0.0 } else { 1.0 },
            "wall={wall}"
        );
        let inspect = ActionCommand::structured(
            OrganismId(1),
            ActionKind::Inspect.canonical_id(),
            ActionKind::Inspect,
            ActionTarget::new(Some(target), None),
            Intensity::new(1.0).unwrap(),
            DurationTicks::new(1),
            Confidence::new(1.0).unwrap(),
            0,
            None,
            None,
            None,
        )
        .unwrap();
        let result = world.apply_command(&inspect).unwrap();
        assert_eq!(
            result.execution.physical.contact,
            if wall {
                PhysicalContactKind::None
            } else {
                PhysicalContactKind::Touch
            }
        );
    }
}

#[test]
fn an_obstacle_keeps_its_own_contact_but_blocks_another_objects_temperature() {
    let mut world = HeadlessScenarioBuilder::new(4243)
        .agent("creature", OrganismId(1), Vec3f::ZERO)
        .food("warm-object", Vec3f::new(0.75, 0.0, 0.0), 0.5)
        .obstacle("cold-rock", Vec3f::new(0.375, 0.0, 0.0), 0.1)
        .build()
        .unwrap();
    for (label, temperature) in [("warm-object", 0.8), ("cold-rock", -0.6)] {
        let id = world.entity_id(label).unwrap();
        let mut properties = world.entity(id).unwrap().grounded_physical;
        properties.surface_temperature = temperature;
        world
            .set_grounded_physical_properties(id, properties)
            .unwrap();
        // Optical transparency cannot authorize physical contact through a rock.
        world.editor_set_optical_opacity(id, 0.0).unwrap();
    }
    let frame = world
        .perception_frame(
            OrganismId(1),
            Tick::ZERO,
            SensorProfile::GroundedTerrainVisionV1,
            HomeostaticSnapshot::baseline(Tick::ZERO),
        )
        .unwrap();
    assert_eq!(frame.sensory().channels.tactile_contact[0], 1.0);
    assert_eq!(frame.sensory().channels.tactile_contact[3], 0.0);
    assert_eq!(frame.sensory().channels.tactile_contact[4], 0.6);
}

#[test]
fn unobstructed_rear_contact_and_the_existing_range_boundary_are_preserved() {
    for x in [-0.75, -0.7501] {
        let mut world = HeadlessScenarioBuilder::new(4243)
            .agent("creature", OrganismId(1), Vec3f::ZERO)
            .food("rear-object", Vec3f::new(x, 0.0, 0.0), 0.5)
            .build()
            .unwrap();
        let id = world.entity_id("rear-object").unwrap();
        let mut properties = world.entity(id).unwrap().grounded_physical;
        properties.surface_temperature = 0.8;
        world
            .set_grounded_physical_properties(id, properties)
            .unwrap();
        let frame = world
            .perception_frame(
                OrganismId(1),
                Tick::ZERO,
                SensorProfile::GroundedTerrainVisionV1,
                HomeostaticSnapshot::baseline(Tick::ZERO),
            )
            .unwrap();
        assert!(frame.grounded_object_slots().is_empty());
        assert_eq!(
            frame.sensory().channels.tactile_contact[0],
            if x == -0.75 { 1.0 } else { 0.0 }
        );
        assert_eq!(
            frame.sensory().channels.tactile_contact[3],
            if x == -0.75 { 0.8 } else { 0.0 }
        );
    }
}
