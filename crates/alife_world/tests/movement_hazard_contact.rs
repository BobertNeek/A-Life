use alife_core::{
    ActionCommand, ActionKind, ActionTarget, BrainCapacityClass, BrainScaleTier, Confidence,
    CreatureGenome, DurationTicks, ExperienceSequenceId, FoundationGeneticIdentity, Intensity,
    MotorChannel, MotorCommandBundle, OrganismId, PhysicalContactKind, Tick, Vec3f, WorldEntityId,
};
use alife_world::{
    persistence::{AssetManifest, PortableSaveFile, RuntimeConfig},
    HeadlessActionIds, HeadlessScenarioBuilder, HeadlessWorld, WorldEditorSpawnSpec,
    WorldObjectKind, WorldOrganismRecord,
};

fn step(intensity: f32) -> ActionCommand {
    ActionCommand::structured(
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
    .unwrap()
}

fn overlapping(reverse: bool, near_pain: f32, far_pain: f32) -> HeadlessWorld {
    let mut builder =
        HeadlessScenarioBuilder::new(4248).agent("creature", OrganismId(1), Vec3f::ZERO);
    let mut hazards = [("far", 0.7, far_pain), ("near", 0.15, near_pain)];
    if reverse {
        hazards.reverse();
    }
    for (label, x, pain) in hazards {
        builder = builder.hazard(label, Vec3f::new(x, 0.0, 0.0), pain);
    }
    builder.build().unwrap()
}

fn register(world: &mut HeadlessWorld) -> WorldEntityId {
    let agent = world.entity_id("creature").unwrap();
    let genome = CreatureGenome::early_mammal_founder(
        4248,
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
    agent
}

#[test]
fn movement_hazard_feedback_and_injury_are_independent_of_spawn_order() {
    for (near_pain, far_pain) in [(0.8, 0.1), (0.1, 0.8)] {
        for intensity in [0.0, 1.0] {
            for registered in [false, true] {
                let mut previous_injury = None;
                for reverse in [false, true] {
                    let mut world = overlapping(reverse, near_pain, far_pain);
                    let agent = world.entity_id("creature").unwrap();
                    let near = world.entity_id("near").unwrap();
                    let hazards = ["far", "near"].map(|label| {
                        world
                            .entity(world.entity_id(label).unwrap())
                            .unwrap()
                            .clone()
                    });
                    if registered {
                        register(&mut world);
                    }
                    let result = if registered {
                        world
                            .apply_registered_neural_command(
                                &step(intensity),
                                agent,
                                Tick::new(1),
                                None,
                                false,
                            )
                            .unwrap()
                            .action_result
                    } else {
                        world.apply_command(&step(intensity)).unwrap()
                    };
                    assert!(result.execution.succeeded);
                    assert_eq!(
                        result.execution.physical.contact,
                        PhysicalContactKind::Collision
                    );
                    assert_eq!(result.execution.physical.target_entity, Some(near));
                    assert_eq!(result.observation.pain_delta.raw(), near_pain);
                    assert_eq!(result.body_event.damage, near_pain);
                    assert_eq!(
                        result.execution.physical.displacement,
                        Vec3f::new(0.1 * intensity, 0.0, 0.0)
                    );
                    assert_eq!(
                        world.entity(agent).unwrap().position,
                        result.execution.physical.displacement
                    );
                    let mut expected = hazards.iter().map(|object| object.id).collect::<Vec<_>>();
                    expected.sort_by_key(|id| id.raw());
                    assert_eq!(result.touched_entities, expected);
                    for object in hazards {
                        assert_eq!(world.entity(object.id), Some(&object));
                    }
                    if registered {
                        let injury = world
                            .organism_registry()
                            .get(OrganismId(1))
                            .unwrap()
                            .biochemistry()
                            .body
                            .injury;
                        if let Some(previous) = previous_injury {
                            assert_eq!(injury, previous);
                        }
                        previous_injury = Some(injury);
                    } else {
                        assert_eq!(
                            world
                                .sensory_report(OrganismId(1), world.tick())
                                .unwrap()
                                .touched_entities,
                            result.touched_entities
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn equal_distance_hazards_keep_the_lowest_entity_id_tie_break() {
    for reverse in [false, true] {
        let mut builder =
            HeadlessScenarioBuilder::new(4248).agent("creature", OrganismId(1), Vec3f::ZERO);
        let mut hazards = [("left", -0.25, 0.2), ("right", 0.25, 0.7)];
        if reverse {
            hazards.reverse();
        }
        for (label, x, pain) in hazards {
            builder = builder.hazard(label, Vec3f::new(x, 0.0, 0.0), pain);
        }
        let mut world = builder.build().unwrap();
        let expected = [
            world.entity_id("left").unwrap(),
            world.entity_id("right").unwrap(),
        ]
        .into_iter()
        .min_by_key(|id| id.raw())
        .unwrap();
        let pain = world.entity(expected).unwrap().hazard_pain;
        let result = world.apply_command(&step(0.0)).unwrap();
        assert_eq!(result.execution.physical.target_entity, Some(expected));
        assert_eq!(result.body_event.damage, pain);
    }
}

#[test]
fn stationary_contact_boundary_and_clear_movement_are_preserved() {
    for x in [0.75, 0.7501, 2.0] {
        let mut world = HeadlessScenarioBuilder::new(4248)
            .agent("creature", OrganismId(1), Vec3f::ZERO)
            .hazard("hazard", Vec3f::new(x, 0.0, 0.0), 0.6)
            .build()
            .unwrap();
        let result = world.apply_command(&step(0.0)).unwrap();
        let expected = if x == 0.75 { 0.6 } else { 0.0 };
        assert_eq!(result.body_event.damage, expected);
        assert_eq!(result.observation.pain_delta.raw(), expected);
        assert_eq!(result.execution.physical.displacement, Vec3f::ZERO);
        assert_eq!(result.touched_entities.len(), usize::from(x == 0.75));
    }
    let mut clear = HeadlessScenarioBuilder::new(4248)
        .agent("creature", OrganismId(1), Vec3f::ZERO)
        .build()
        .unwrap();
    let result = clear.apply_command(&step(1.0)).unwrap();
    assert_eq!(
        result.execution.physical.contact,
        PhysicalContactKind::Moved
    );
    assert_eq!(
        result.execution.physical.displacement,
        Vec3f::new(0.1, 0.0, 0.0)
    );
    assert_eq!(result.body_event.damage, 0.0);
}

#[test]
fn a_swept_only_hazard_still_causes_contact_after_the_body_leaves_it() {
    let mut world = HeadlessScenarioBuilder::new(4248)
        .agent("creature", OrganismId(1), Vec3f::ZERO)
        .build()
        .unwrap();
    let target = world
        .editor_spawn_object(WorldEditorSpawnSpec {
            label: "swept".into(),
            kind: WorldObjectKind::Hazard,
            organism_id: None,
            position: Vec3f::new(0.05, 0.0, 0.099),
            radius: 0.1,
            hazard_pain: 0.6,
            nutrition: 0.0,
            token_id: None,
        })
        .unwrap();
    assert!(0.05_f32.hypot(0.099) > world.entity(target).unwrap().radius);
    let result = world.apply_command(&step(1.0)).unwrap();
    assert_eq!(
        result.execution.physical.contact,
        PhysicalContactKind::Collision
    );
    assert_eq!(result.execution.physical.target_entity, Some(target));
    assert_eq!(result.body_event.damage, 0.6);
    assert_eq!(result.touched_entities, vec![target]);
}

#[test]
fn motor_bundle_reports_the_same_nearest_hazard_as_its_body_contact() {
    for reverse in [false, true] {
        for intensity in [0.0, 1.0] {
            let mut world = overlapping(reverse, 0.8, 0.1);
            let agent = register(&mut world);
            let near = world.entity_id("near").unwrap();
            let channel =
                alife_core::channel_command_for_action(MotorChannel::Locomotion, &step(intensity))
                    .unwrap();
            let bundle = MotorCommandBundle::new(
                OrganismId(1),
                ExperienceSequenceId::new(1).unwrap(),
                Tick::ZERO,
                vec![channel],
            )
            .unwrap();
            let receipt = world.apply_registered_motor_bundle(&bundle, agent).unwrap();
            assert_eq!(receipt.joint.execution.target_entity, Some(near));
            assert_eq!(
                receipt.channel_receipts[0].physical.target_entity,
                Some(near)
            );
            assert_eq!(receipt.body_event.damage, 0.8);
        }
    }
}

#[test]
fn portable_restore_keeps_hazard_identity_and_geometric_selection() {
    let world = overlapping(false, 0.8, 0.1);
    let before = world.object_snapshots();
    let near = world.entity_id("near").unwrap();
    let save = PortableSaveFile::from_headless_world(
        "hazard-contact",
        &world,
        RuntimeConfig::deterministic_default(world.seed(), BrainScaleTier::Nano512),
        AssetManifest::empty(),
        vec![],
    )
    .unwrap();
    let mut restored = PortableSaveFile::from_json_str(&save.to_json_string_pretty().unwrap())
        .unwrap()
        .restore_headless_world()
        .unwrap();
    assert_eq!(restored.object_snapshots(), before);
    let result = restored.apply_command(&step(1.0)).unwrap();
    assert_eq!(result.execution.physical.target_entity, Some(near));
    assert_eq!(result.body_event.damage, 0.8);
}

#[test]
fn completed_pose_contact_takes_priority_over_a_nearer_swept_only_hazard() {
    for reverse in [false, true] {
        let mut world = HeadlessScenarioBuilder::new(4248)
            .agent("creature", OrganismId(1), Vec3f::ZERO)
            .build()
            .unwrap();
        let mut hazards = [
            ("endpoint", Vec3f::new(0.7, 0.0, 0.0), 0.75, 0.8),
            ("swept", Vec3f::new(0.05, 0.0, 0.099), 0.1, 0.1),
        ];
        if reverse {
            hazards.reverse();
        }
        for (label, position, radius, pain) in hazards {
            world
                .editor_spawn_object(WorldEditorSpawnSpec {
                    label: label.into(),
                    kind: WorldObjectKind::Hazard,
                    organism_id: None,
                    position,
                    radius,
                    hazard_pain: pain,
                    nutrition: 0.0,
                    token_id: None,
                })
                .unwrap();
        }
        let endpoint = world.entity_id("endpoint").unwrap();
        let mut registered_world = world.clone();
        let agent = register(&mut registered_world);
        let channel =
            alife_core::channel_command_for_action(MotorChannel::Locomotion, &step(1.0)).unwrap();
        let bundle = MotorCommandBundle::new(
            OrganismId(1),
            ExperienceSequenceId::new(1).unwrap(),
            Tick::ZERO,
            vec![channel],
        )
        .unwrap();
        let receipt = registered_world
            .apply_registered_motor_bundle(&bundle, agent)
            .unwrap();
        let result = world.apply_command(&step(1.0)).unwrap();
        println!("reverse={reverse} endpoint={} action_target={:?} action_damage={} joint_target={:?} joint_damage={}",
            endpoint.raw(), result.execution.physical.target_entity, result.body_event.damage,
            receipt.joint.execution.target_entity, receipt.body_event.damage);
        assert_eq!(result.execution.physical.target_entity, Some(endpoint));
        assert_eq!(result.body_event.damage, 0.8);
        assert_eq!(receipt.joint.execution.target_entity, Some(endpoint));
        assert_eq!(receipt.body_event.damage, 0.8);
    }
}

#[test]
fn multiple_swept_only_contacts_use_completed_pose_proximity_in_both_spawn_orders() {
    for reverse in [false, true] {
        let mut world = HeadlessScenarioBuilder::new(4248)
            .agent("creature", OrganismId(1), Vec3f::ZERO)
            .build()
            .unwrap();
        let mut hazards = [("earlier", 0.03_f32, 0.2), ("later", 0.07, 0.6)];
        if reverse {
            hazards.reverse();
        }
        for (label, x, pain) in hazards {
            assert!(x.hypot(0.099) > 0.1 && (0.1 - x).hypot(0.099) > 0.1);
            world
                .editor_spawn_object(WorldEditorSpawnSpec {
                    label: label.into(),
                    kind: WorldObjectKind::Hazard,
                    organism_id: None,
                    position: Vec3f::new(x, 0.0, 0.099),
                    radius: 0.1,
                    hazard_pain: pain,
                    nutrition: 0.0,
                    token_id: None,
                })
                .unwrap();
        }
        let result = world.apply_command(&step(1.0)).unwrap();
        assert_eq!(
            result.execution.physical.target_entity,
            world.entity_id("later")
        );
        assert_eq!(result.body_event.damage, 0.6);
        assert_eq!(result.touched_entities.len(), 2);
    }
}
