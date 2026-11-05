use alife_core::{
    ActionCommand, ActionKind, ActionTarget, BrainCapacityClass, BrainScaleTier, Confidence,
    CreatureGenome, DurationTicks, ExperienceSequenceId, FoundationGeneticIdentity,
    HomeostaticSnapshot, Intensity, MotorChannel, MotorCommandBundle, OrganismId,
    PhysicalContactKind, ReferenceActionFailure, SensorProfile, Tick, Vec3f, WorldEntityId,
};
use alife_world::{
    AssetManifest, HeadlessActionIds, HeadlessScenarioBuilder, HeadlessWorld, LocomotionLimits,
    PortableSaveFile, RuntimeConfig, TerrainData, WorldOrganismRecord, WorldTerrain,
};

fn command(kind: ActionKind, target: WorldEntityId) -> ActionCommand {
    ActionCommand::structured(
        OrganismId(1),
        if kind == ActionKind::Interact {
            HeadlessActionIds::EAT
        } else {
            kind.canonical_id()
        },
        kind,
        ActionTarget::new(Some(target), None),
        Intensity::new(1.0).unwrap(),
        DurationTicks::new(1),
        Confidence::new(1.0).unwrap(),
        0,
        None,
        None,
        None,
    )
    .unwrap()
}

fn rock_world(x: f32) -> HeadlessWorld {
    HeadlessScenarioBuilder::new(4252)
        .agent("creature", OrganismId(1), Vec3f::ZERO)
        .obstacle("rock", Vec3f::new(x, 0.0, 0.0), 0.75)
        .build()
        .unwrap()
}

#[test]
fn reachable_solid_surfaces_keep_contact_in_actions_and_grounded_perception_after_restore() {
    for restored in [false, true] {
        for profile in [
            SensorProfile::GroundedObjectSlotsV1,
            SensorProfile::GroundedTerrainVisionV1,
        ] {
            let world = rock_world(0.75);
            let objects = world.object_snapshots();
            let mut world = if restored {
                let save = PortableSaveFile::from_headless_world(
                    "solid-surface",
                    &world,
                    RuntimeConfig::deterministic_default(world.seed(), BrainScaleTier::Nano512),
                    AssetManifest::empty(),
                    vec![],
                )
                .unwrap();
                PortableSaveFile::from_json_str(&save.to_json_string_pretty().unwrap())
                    .unwrap()
                    .restore_headless_world()
                    .unwrap()
            } else {
                world
            };
            assert_eq!(world.object_snapshots(), objects);
            let rock = world.entity_id("rock").unwrap();
            let observed = world
                .physical_observation_snapshot(OrganismId(1), Tick::ZERO)
                .unwrap();
            assert!(
                observed
                    .visible
                    .iter()
                    .find(|o| o.transport_entity == rock)
                    .unwrap()
                    .contact
            );
            let mut indexed = world.clone();
            let index = indexed.build_perception_batch_index().unwrap();
            let frame = world
                .perception_frame_draft(
                    OrganismId(1),
                    Tick::ZERO,
                    profile,
                    HomeostaticSnapshot::baseline(Tick::ZERO),
                )
                .unwrap();
            let indexed_frame = indexed
                .perception_frame_draft_indexed(
                    OrganismId(1),
                    Tick::ZERO,
                    profile,
                    HomeostaticSnapshot::baseline(Tick::ZERO),
                    &index,
                )
                .unwrap();
            assert_eq!(frame, indexed_frame);
            if profile == SensorProfile::GroundedTerrainVisionV1 {
                assert_eq!(frame.sensory().channels.tactile_contact[0], 1.0);
            }
            assert_eq!(frame.grounded_object_slots().len(), 1);
            assert!(frame.grounded_object_slots()[0].contact > 0.0);
            let result = world
                .apply_command(&command(ActionKind::Inspect, rock))
                .unwrap();
            assert!(result.execution.succeeded);
            assert_eq!(
                result.execution.physical.contact,
                PhysicalContactKind::Touch
            );
            assert_eq!(result.execution.physical.target_entity, Some(rock));
            assert_eq!(result.touched_entities, vec![rock]);
            assert_eq!(result.body_event.damage, 0.0);
            assert_eq!(world.object_snapshots(), objects);
        }
    }
}

#[test]
fn inspecting_a_solid_keeps_the_existing_reach_limit_and_rear_contact() {
    for (x, reachable) in [(-0.75, true), (1.25, true), (1.2501, false)] {
        let mut world = rock_world(x);
        let rock = world.entity_id("rock").unwrap();
        let result = world
            .apply_command(&command(ActionKind::Inspect, rock))
            .unwrap();
        assert_eq!(
            result.execution.physical.contact,
            if reachable {
                PhysicalContactKind::Touch
            } else {
                PhysicalContactKind::None
            }
        );
        assert_eq!(
            result.touched_entities,
            if reachable { vec![rock] } else { vec![] }
        );
    }
}

#[test]
fn other_solids_and_terrain_still_block_contact_with_a_rock() {
    for terrain in [false, true] {
        let mut world = if terrain {
            rock_world(0.75)
        } else {
            HeadlessScenarioBuilder::new(4252)
                .agent("creature", OrganismId(1), Vec3f::ZERO)
                .obstacle("rock", Vec3f::new(0.75, 0.0, 0.0), 0.75)
                // Neither center endpoint is inside this intervening sphere.
                .obstacle("barrier", Vec3f::new(0.375, 0.0, 0.749), 0.75)
                .build()
                .unwrap()
        };
        if terrain {
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
                            obstacles: vec![[0.37, -1.0, 0.38, 1.0, -0.5, 2.0]],
                            water_level: None,
                        },
                        LocomotionLimits::default(),
                    )
                    .unwrap(),
                    Vec3f::ZERO,
                )
                .unwrap();
        } else {
            let barrier = world.entity_id("barrier").unwrap();
            world.editor_set_optical_opacity(barrier, 0.0).unwrap();
        }
        let rock = world.entity_id("rock").unwrap();
        assert_eq!(
            world.entity(rock).unwrap().position,
            Vec3f::new(0.75, 0.0, 0.0)
        );
        let observed = world
            .physical_observation_snapshot(OrganismId(1), Tick::ZERO)
            .unwrap();
        assert!(
            !observed
                .visible
                .iter()
                .find(|o| o.transport_entity == rock)
                .unwrap()
                .contact
        );
        let result = world
            .apply_command(&command(ActionKind::Inspect, rock))
            .unwrap();
        assert_eq!(result.execution.physical.contact, PhysicalContactKind::None);
        assert!(result.touched_entities.is_empty());
    }
}

#[test]
fn attempting_to_eat_a_reachable_rock_reports_touch_without_ingestion_or_possession() {
    let mut world = rock_world(0.75);
    let objects = world.object_snapshots();
    let rock = world.entity_id("rock").unwrap();
    let result = world
        .apply_command(&command(ActionKind::Interact, rock))
        .unwrap();
    assert!(!result.execution.succeeded);
    assert_eq!(
        result.execution.failure,
        Some(ReferenceActionFailure::MissingAffordance)
    );
    assert_eq!(
        result.execution.physical.contact,
        PhysicalContactKind::Touch
    );
    assert_eq!(result.touched_entities, vec![rock]);
    assert_eq!(result.body_event.nutrition, 0.0);
    assert_eq!(result.body_event.damage, 0.0);
    assert_eq!(world.object_snapshots(), objects);
}

#[test]
fn a_registered_motor_investigation_reports_the_same_surface_touch() {
    let mut world = rock_world(0.75);
    let rock = world.entity_id("rock").unwrap();
    let agent = world.entity_id("creature").unwrap();
    let genome = CreatureGenome::early_mammal_founder(
        4252,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    let phenotype = genome.express().unwrap();
    world
        .register_organism_record(
            WorldOrganismRecord::newborn(OrganismId(1), agent, genome, phenotype, Tick::ZERO)
                .unwrap(),
        )
        .unwrap();
    let channel = alife_core::channel_command_for_action(
        MotorChannel::Posture,
        &command(ActionKind::Inspect, rock),
    )
    .unwrap();
    let bundle = MotorCommandBundle::new(
        OrganismId(1),
        ExperienceSequenceId::new(1).unwrap(),
        Tick::ZERO,
        vec![channel],
    )
    .unwrap();
    let receipt = world.apply_registered_motor_bundle(&bundle, agent).unwrap();
    assert!(receipt.succeeded);
    assert_eq!(
        receipt.channel_receipts[0].physical.contact,
        PhysicalContactKind::Touch
    );
    assert_eq!(receipt.joint.execution.contact, PhysicalContactKind::Touch);
    assert_eq!(
        receipt.channel_receipts[0].physical.target_entity,
        Some(rock)
    );
    assert_eq!(receipt.body_event.damage, 0.0);
    assert_eq!(receipt.body_event.nutrition, 0.0);
    assert_eq!(world.entity(agent).unwrap().position, Vec3f::ZERO);
    assert!(!world.entity(rock).unwrap().consumed);
    assert_eq!(world.entity(rock).unwrap().carried_by, None);
}
