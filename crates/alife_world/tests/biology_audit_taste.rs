//! Regression coverage for bounded, organism-owned ingestion observations.
use alife_core::*;
use alife_world::*;

fn fixture() -> HeadlessWorld {
    let mut world = HeadlessScenarioBuilder::new(921)
        .agent("a", OrganismId(1), Vec3f::ZERO)
        .agent("b", OrganismId(2), Vec3f::new(3.0, 0.0, 0.0))
        .food("food", Vec3f::new(0.1, 0.0, 0.0), 0.6)
        .build()
        .unwrap();
    for (id, label) in [(OrganismId(1), "a"), (OrganismId(2), "b")] {
        let genome = CreatureGenome::early_mammal_founder(
            921 + id.raw(),
            FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
        )
        .unwrap();
        let phenotype = genome.express().unwrap();
        let record = WorldOrganismRecord::newborn(
            id,
            world.entity_id(label).unwrap(),
            genome,
            phenotype,
            Tick::ZERO,
        )
        .unwrap();
        world.register_organism_record(record).unwrap();
    }
    let food = world.entity_id("food").unwrap();
    let mut properties = world.entity(food).unwrap().grounded_physical;
    properties.chemical[2] = 0.8;
    world
        .set_grounded_physical_properties(food, properties)
        .unwrap();
    world
}

fn act(world: &mut HeadlessWorld, id: OrganismId, eat: bool) {
    let (channel, primitive, target) = if eat {
        (
            MotorChannel::Manipulation,
            HeadlessActionIds::EAT,
            Some(ActionTarget::new(
                Some(world.entity_id("food").unwrap()),
                None,
            )),
        )
    } else {
        (MotorChannel::Posture, ActionKind::Rest.canonical_id(), None)
    };
    let command = ChannelCommand::new(
        channel,
        primitive,
        target,
        Vec3f::ZERO,
        Intensity::new(1.0).unwrap(),
        DurationTicks::new(1),
        0.0,
        Confidence::new(0.9).unwrap(),
        0,
    )
    .unwrap();
    let bundle = MotorCommandBundle::new(
        id,
        ExperienceSequenceId::new(1).unwrap(),
        Tick::ZERO,
        vec![command],
    )
    .unwrap();
    let entity = world.organism_registry().get(id).unwrap().world_entity_id();
    let receipt = world
        .apply_registered_motor_bundle(&bundle, entity)
        .unwrap();
    assert!(receipt.succeeded);
}

fn taste(world: &mut HeadlessWorld) -> f32 {
    let bio = *world
        .organism_registry()
        .get(OrganismId(1))
        .unwrap()
        .biochemistry();
    let index = world.build_perception_batch_index().unwrap();
    world
        .perception_frame_draft_indexed(
            OrganismId(1),
            bio.tick,
            SensorProfile::GroundedTerrainVisionV1,
            bio.homeostasis,
            &index,
        )
        .unwrap()
        .sensory()
        .channels
        .tactile_contact[5]
}

#[test]
fn another_creatures_rest_preserves_eaters_taste() {
    let mut world = fixture();
    act(&mut world, OrganismId(1), true);
    let before = taste(&mut world);
    assert!(before > 0.0);
    act(&mut world, OrganismId(2), false);
    let after = taste(&mut world);
    println!("taste before other creature rests={before}; after={after}");
    assert_eq!(after, before);
}

#[test]
fn exact_world_roundtrip_preserves_eaters_taste() {
    let mut world = fixture();
    act(&mut world, OrganismId(2), false);
    act(&mut world, OrganismId(1), true);
    world.try_advance_tick().unwrap();
    let before = taste(&mut world);
    assert!(before > 0.0);
    let save = PortableSaveFile::from_headless_world(
        "taste-audit",
        &world,
        RuntimeConfig::deterministic_default(921, BrainScaleTier::Nano512),
        AssetManifest::empty(),
        vec![],
    )
    .unwrap();
    let json = serde_json::to_string(&save).unwrap();
    let decoded: PortableSaveFile = serde_json::from_str(&json).unwrap();
    let mut restored = decoded.restore_headless_world().unwrap();
    let after = taste(&mut restored);
    println!("taste before save={before}; after restore={after}");
    assert_eq!(after, before);
}

#[test]
fn simultaneous_neutral_posture_preserves_taste() {
    let mut control = fixture();
    act(&mut control, OrganismId(1), true);
    let expected = taste(&mut control);
    let mut world = fixture();
    let food = world.entity_id("food").unwrap();
    let channel = |kind, primitive, target| {
        ChannelCommand::new(
            kind,
            primitive,
            target,
            Vec3f::ZERO,
            Intensity::new(1.0).unwrap(),
            DurationTicks::new(1),
            0.0,
            Confidence::new(0.9).unwrap(),
            0,
        )
        .unwrap()
    };
    let bundle = MotorCommandBundle::new(
        OrganismId(1),
        ExperienceSequenceId::new(1).unwrap(),
        Tick::ZERO,
        vec![
            channel(
                MotorChannel::Manipulation,
                HeadlessActionIds::EAT,
                Some(ActionTarget::new(Some(food), None)),
            ),
            channel(MotorChannel::Posture, HeadlessActionIds::NO_POSTURE, None),
        ],
    )
    .unwrap();
    let entity = world.entity_id("a").unwrap();
    let receipt = world
        .apply_registered_motor_bundle(&bundle, entity)
        .unwrap();
    assert!(receipt.succeeded);
    assert!(receipt.body_event.nutrition > 0.0);
    let actual = taste(&mut world);
    println!("taste eating alone={expected}; with neutral posture={actual}");
    assert!(expected > 0.0);
    assert_eq!(actual, expected);
}

fn save(world: &HeadlessWorld) -> PortableSaveFile {
    PortableSaveFile::from_headless_world(
        "taste",
        world,
        RuntimeConfig::deterministic_default(921, BrainScaleTier::Nano512),
        AssetManifest::empty(),
        vec![],
    )
    .unwrap()
}

#[test]
fn taste_expires_after_next_perception_and_failed_ingestion_does_not_refresh_it() {
    let mut world = fixture();
    assert_eq!(taste(&mut world), 0.0);
    act(&mut world, OrganismId(1), true);
    world.try_advance_tick().unwrap();
    let at_next_tick = taste(&mut world);
    assert!(at_next_tick > 0.0);
    // Re-reading perception is observational; it must not consume or prolong taste.
    assert_eq!(taste(&mut world), at_next_tick);
    let food = world.entity_id("food").unwrap();
    let failed = world
        .apply_command(&HeadlessWorldCommand::eat(OrganismId(1), food).unwrap())
        .unwrap();
    assert!(!failed.execution.succeeded);
    world.try_advance_tick().unwrap();
    assert_eq!(taste(&mut world), 0.0);
    assert!(save(&world).world.ingestion_observations.is_empty());
    for _ in 0..3 {
        world.try_advance_tick().unwrap();
    }
    assert_eq!(taste(&mut world), 0.0);
}

#[test]
fn two_eaters_keep_their_own_opposite_tastes() {
    let mut world = fixture();
    let food_b = world
        .editor_spawn_object(WorldEditorSpawnSpec {
            label: "b-food".into(),
            kind: WorldObjectKind::Food,
            organism_id: None,
            position: Vec3f::new(3.1, 0.0, 0.0),
            radius: 0.2,
            nutrition: 0.6,
            hazard_pain: 0.0,
            token_id: None,
        })
        .unwrap();
    let mut properties = world.entity(food_b).unwrap().grounded_physical;
    properties.chemical[2] = -0.7;
    world
        .set_grounded_physical_properties(food_b, properties)
        .unwrap();
    act(&mut world, OrganismId(1), true);
    let entity_b = world.entity_id("b").unwrap();
    let receipt = world
        .apply_registered_command(
            &HeadlessWorldCommand::eat(OrganismId(2), food_b).unwrap(),
            entity_b,
            Tick(1),
        )
        .unwrap();
    assert!(receipt.action_result.execution.succeeded);
    world.try_advance_tick().unwrap();
    assert!(taste(&mut world) > 0.0);
    let bio = *world
        .organism_registry()
        .get(OrganismId(2))
        .unwrap()
        .biochemistry();
    let index = world.build_perception_batch_index().unwrap();
    let frame = world
        .perception_frame_draft_indexed(
            OrganismId(2),
            bio.tick,
            SensorProfile::GroundedTerrainVisionV1,
            bio.homeostasis,
            &index,
        )
        .unwrap();
    assert_eq!(frame.sensory().channels.tactile_contact[5], 0.0);
    assert!(frame.sensory().channels.tactile_contact[6] > 0.0);
    assert_eq!(save(&world).world.ingestion_observations.len(), 2);
}

#[test]
fn sampled_taste_survives_food_edit_and_removal() {
    let mut world = fixture();
    act(&mut world, OrganismId(1), true);
    let expected = taste(&mut world);
    let food = world.entity_id("food").unwrap();
    let mut properties = world.entity(food).unwrap().grounded_physical;
    properties.chemical[2] = -0.9;
    world
        .set_grounded_physical_properties(food, properties)
        .unwrap();
    assert_eq!(taste(&mut world), expected);
    world.editor_remove_object(food).unwrap();
    assert_eq!(taste(&mut world), expected);
}

#[test]
fn dead_creatures_drop_taste_before_retirement() {
    let mut world = fixture();
    act(&mut world, OrganismId(1), true);
    world.try_advance_tick().unwrap();
    assert!(taste(&mut world) > 0.0);
    let mut record = world
        .organism_registry()
        .get(OrganismId(1))
        .unwrap()
        .clone();
    record.mark_dead(Tick(1)).unwrap();
    world.replace_organism_record_exact(record).unwrap();
    assert!(save(&world).world.ingestion_observations.is_empty());
}

#[test]
fn both_external_actor_removal_paths_clear_taste() {
    for editor in [false, true] {
        let mut world = HeadlessScenarioBuilder::new(921)
            .agent("a", OrganismId(1), Vec3f::ZERO)
            .food("food", Vec3f::new(0.1, 0.0, 0.0), 0.6)
            .build()
            .unwrap();
        let food = world.entity_id("food").unwrap();
        world
            .apply_command(&HeadlessWorldCommand::eat(OrganismId(1), food).unwrap())
            .unwrap();
        assert_eq!(save(&world).world.ingestion_observations.len(), 1);
        if editor {
            world
                .editor_remove_object(world.entity_id("a").unwrap())
                .unwrap();
        } else {
            world.remove_organism(OrganismId(1)).unwrap();
        }
        assert!(save(&world).world.ingestion_observations.is_empty());
    }
}

#[test]
fn legacy_saves_without_observations_remain_loadable_and_new_invalid_samples_fail() {
    let mut world = fixture();
    let mut legacy = serde_json::to_value(save(&world)).unwrap();
    legacy["world"]
        .as_object_mut()
        .unwrap()
        .remove("ingestion_observations");
    let decoded: PortableSaveFile = serde_json::from_value(legacy).unwrap();
    assert_eq!(taste(&mut decoded.restore_headless_world().unwrap()), 0.0);
    act(&mut world, OrganismId(1), true);
    world.try_advance_tick().unwrap();
    for sample in [
        IngestionObservation {
            ingested_at: Tick(2),
            taste: 0.4,
        },
        IngestionObservation {
            ingested_at: Tick(0),
            taste: 1.1,
        },
        IngestionObservation {
            ingested_at: Tick(0),
            taste: f32::NAN,
        },
    ] {
        let mut invalid = save(&world);
        invalid.world.ingestion_observations.insert(1, sample);
        assert!(invalid.restore_headless_world().is_err());
    }
    let mut invalid = save(&world);
    let sample = invalid.world.ingestion_observations.remove(&1).unwrap();
    invalid.world.ingestion_observations.insert(999, sample);
    assert!(invalid.restore_headless_world().is_err());
}
