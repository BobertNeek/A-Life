use alife_core::{
    ActionCommand, ActionKind, ActionTarget, BrainCapacityClass, BrainScaleTier, Confidence,
    CreatureGenome, DurationTicks, ExperienceSequenceId, FoundationGeneticIdentity,
    HomeostaticSnapshot, Intensity, MotorChannel, MotorCommandBundle, OrganismId,
    PhysicalContactKind, SensorProfile, Tick, Vec3f, WorldEntityId,
};
use alife_world::{
    AssetManifest, EcologyZoneId, HeadlessActionIds, HeadlessScenarioBuilder, HeadlessWorld,
    LocomotionLimits, PortableSaveFile, RuntimeConfig, TerrainData, TerrainZone, TerrainZoneKind,
    WorldEditorSpawnSpec, WorldObjectKind, WorldOrganismRecord, WorldTerrain,
};

type Source = (&'static str, Vec3f, f32, f32);
fn world(
    boxes: Vec<[f32; 6]>,
    sources: &[Source],
    spheres: &[(&str, Vec3f, f32)],
) -> HeadlessWorld {
    let mut world = HeadlessScenarioBuilder::new(4261)
        .agent("creature", OrganismId(1), Vec3f::ZERO)
        .build()
        .unwrap();
    if !boxes.is_empty() {
        world
            .enable_terrain_for_new_game(
                WorldTerrain::new(
                    TerrainData {
                        width: 9,
                        depth: 9,
                        origin_x: -2.0,
                        origin_z: -2.0,
                        spacing: 0.5,
                        heights: vec![0.0; 81],
                        obstacles: boxes,
                        water_level: None,
                    },
                    LocomotionLimits::default(),
                )
                .unwrap(),
                Vec3f::ZERO,
            )
            .unwrap();
    }
    for (label, position, radius, pain) in sources {
        let id = world
            .editor_spawn_object(WorldEditorSpawnSpec {
                label: (*label).into(),
                kind: WorldObjectKind::Hazard,
                organism_id: None,
                position: *position,
                radius: *radius,
                nutrition: 0.0,
                hazard_pain: *pain,
                token_id: None,
            })
            .unwrap();
        let mut properties = world.entity(id).unwrap().grounded_physical;
        properties.surface_temperature = 0.8;
        world
            .set_grounded_physical_properties(id, properties)
            .unwrap();
        assert_eq!(world.entity(id).unwrap().position, *position);
    }
    for (label, position, radius) in spheres {
        let id = world
            .editor_spawn_object(WorldEditorSpawnSpec {
                label: (*label).into(),
                kind: WorldObjectKind::Obstacle,
                organism_id: None,
                position: *position,
                radius: *radius,
                nutrition: 0.0,
                hazard_pain: 0.0,
                token_id: None,
            })
            .unwrap();
        world.editor_set_optical_opacity(id, 0.0).unwrap();
    }
    world
}
fn source() -> Source {
    ("thorn", Vec3f::new(0.75, 0.0, 0.0), 0.75, 0.6)
}
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
fn inspect(id: WorldEntityId) -> ActionCommand {
    ActionCommand::structured(
        OrganismId(1),
        ActionKind::Inspect.canonical_id(),
        ActionKind::Inspect,
        ActionTarget::new(Some(id), None),
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
fn register(world: &mut HeadlessWorld) -> WorldEntityId {
    let agent = world.entity_id("creature").unwrap();
    let genome = CreatureGenome::early_mammal_founder(
        4261,
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
#[derive(Clone, Copy, Debug)]
enum Path {
    Direct,
    Registered,
    MotorMove,
    MotorIdle,
}
const PATHS: [Path; 4] = [
    Path::Direct,
    Path::Registered,
    Path::MotorMove,
    Path::MotorIdle,
];
fn run(world: &mut HeadlessWorld, path: Path) -> f32 {
    match path {
        Path::Direct => world.apply_command(&step(1.0)).unwrap().body_event.damage,
        Path::Registered => {
            let agent = register(world);
            let receipt = world
                .apply_registered_neural_command(&step(1.0), agent, Tick(1), None, false)
                .unwrap();
            assert_eq!(
                receipt.biology_after.body.injury > receipt.biology_before.body.injury,
                receipt.action_result.body_event.damage > 0.0
            );
            receipt.action_result.body_event.damage
        }
        Path::MotorMove | Path::MotorIdle => {
            let agent = register(world);
            let channels = if matches!(path, Path::MotorMove) {
                vec![
                    alife_core::channel_command_for_action(MotorChannel::Locomotion, &step(1.0))
                        .unwrap(),
                ]
            } else {
                vec![]
            };
            let bundle = MotorCommandBundle::new(
                OrganismId(1),
                ExperienceSequenceId::new(1).unwrap(),
                Tick::ZERO,
                channels,
            )
            .unwrap();
            let receipt = world.apply_registered_motor_bundle(&bundle, agent).unwrap();
            assert_eq!(
                receipt.biology_after.body.injury > receipt.biology_before.body.injury,
                receipt.body_event.damage > 0.0
            );
            receipt.body_event.damage
        }
    }
}

#[test]
fn box_and_transparent_sphere_shields_agree_with_touch_inspect_and_all_damage_paths() {
    for barrier in 0..3 {
        let boxes = if barrier == 1 {
            vec![[0.37, -1.0, 0.38, 1.0, -0.5, 2.0]]
        } else {
            vec![]
        };
        let spheres = if barrier == 2 {
            vec![("barrier", Vec3f::new(0.375, 0.0, 0.749), 0.75)]
        } else {
            vec![]
        };
        let initial = world(boxes, &[source()], &spheres);
        let hazard = initial.entity_id("thorn").unwrap();
        let object = initial.entity(hazard).unwrap().clone();
        let mut sensed = initial.clone();
        let frame = sensed
            .perception_frame(
                OrganismId(1),
                Tick::ZERO,
                SensorProfile::GroundedTerrainVisionV1,
                HomeostaticSnapshot::baseline(Tick::ZERO),
            )
            .unwrap();
        assert_eq!(
            frame.sensory().channels.tactile_contact[0] > 0.0,
            barrier == 0
        );
        assert_eq!(
            frame.sensory().channels.tactile_contact[3] > 0.0,
            barrier == 0
        );
        let result = sensed.apply_command(&inspect(hazard)).unwrap();
        assert_eq!(
            result.execution.physical.contact,
            if barrier == 0 {
                PhysicalContactKind::Touch
            } else {
                PhysicalContactKind::None
            }
        );
        for path in PATHS {
            let mut trial = initial.clone();
            assert_eq!(
                run(&mut trial, path),
                if barrier == 0 { 0.6 } else { 0.0 },
                "barrier={barrier} path={path:?}"
            );
            assert_eq!(trial.entity(hazard), Some(&object));
        }
        let mut direct = initial.clone();
        let result = direct.apply_command(&step(1.0)).unwrap();
        assert_eq!(
            result.execution.physical.contact,
            if barrier == 0 {
                PhysicalContactKind::Collision
            } else {
                PhysicalContactKind::Moved
            }
        );
        assert_eq!(
            result.touched_entities,
            if barrier == 0 { vec![hazard] } else { vec![] }
        );
        assert_eq!(
            direct
                .entity(direct.entity_id("creature").unwrap())
                .unwrap()
                .position,
            Vec3f::new(0.1, 0.0, 0.0)
        );
    }
}

#[test]
fn swept_damage_keeps_exposed_endpoints_and_an_exposed_interior_interval() {
    let setups = [
        (
            Vec3f::new(0.05, 0.0, 0.4),
            0.405,
            vec![[0.045, 0.25, 0.055, 0.35, -0.5, 2.0]],
            false,
        ),
        (
            Vec3f::new(0.05, 0.0, 0.8),
            0.805,
            [0.015625, 0.05, 0.084375]
                .map(|x| [x - 0.001, 0.249, x + 0.001, 0.251, -0.5, 2.0])
                .to_vec(),
            true,
        ),
    ];
    for (position, radius, boxes, hidden_endpoints) in setups {
        let initial = world(boxes, &[("thorn", position, radius, 0.6)], &[]);
        for path in PATHS {
            let mut trial = initial.clone();
            let damage = run(&mut trial, path);
            assert_eq!(
                damage,
                if hidden_endpoints && matches!(path, Path::MotorIdle) {
                    0.0
                } else {
                    0.6
                },
                "{path:?}"
            );
        }
        if hidden_endpoints {
            let hazard = initial.entity_id("thorn").unwrap();
            let agent = initial.entity_id("creature").unwrap();
            for (x, damage) in [(0.0, 0.0), (0.025, 0.6), (0.05, 0.0), (0.1, 0.0)] {
                let mut trial = initial.clone();
                trial
                    .editor_move_object(agent, Vec3f::new(x, 0.0, 0.0))
                    .unwrap();
                assert_eq!(
                    trial
                        .apply_command(&inspect(hazard))
                        .unwrap()
                        .body_event
                        .damage,
                    damage
                );
            }
        }
    }
}

#[test]
fn numerical_admissions_preserve_f32_radius_contact_and_conservative_graze_fallback() {
    let position = Vec3f::new(0.05, 0.45, 0.6);
    assert!(f64::from(position.y).powi(2) + f64::from(position.z).powi(2) > 0.75_f64.powi(2));
    let initial = world(vec![], &[("thorn", position, 0.75, 0.6)], &[]);
    for path in PATHS {
        assert_eq!(
            run(&mut initial.clone(), path),
            if matches!(path, Path::MotorIdle) {
                0.0
            } else {
                0.6
            }
        );
    }
    let initial = world(
        vec![],
        &[source()],
        &[("graze", Vec3f::new(0.375, 0.45, 0.6), 0.75)],
    );
    let hazard = initial.entity_id("thorn").unwrap();
    assert_eq!(
        initial
            .clone()
            .apply_command(&inspect(hazard))
            .unwrap()
            .body_event
            .damage,
        0.0
    );
    // The bounded sweep is unresolved here; it keeps the old damage. An idle
    // point query is exact and remains shielded. This is not universal shielding.
    for path in PATHS {
        assert_eq!(
            run(&mut initial.clone(), path),
            if matches!(path, Path::MotorIdle) {
                0.0
            } else {
                0.6
            }
        );
    }
}

#[test]
fn stored_radius_boundaries_initial_final_and_zero_movement_contacts_are_preserved() {
    for (x, radius, expected) in [
        (0.75, 0.75, 0.6),
        (0.75, f32::from_bits(0.75_f32.to_bits() - 1), 0.0),
        (2.0, 2.5, 0.6),
    ] {
        let initial = world(
            vec![],
            &[("thorn", Vec3f::new(x, 0.0, 0.0), radius, 0.6)],
            &[],
        );
        assert_eq!(run(&mut initial.clone(), Path::MotorIdle), expected);
        assert_eq!(
            initial
                .clone()
                .apply_command(&step(0.0))
                .unwrap()
                .body_event
                .damage,
            expected
        );
    }
    for x in [-0.7, 0.8] {
        let initial = world(
            vec![],
            &[("thorn", Vec3f::new(x, 0.0, 0.0), 0.75, 0.6)],
            &[],
        );
        for path in [Path::Direct, Path::Registered, Path::MotorMove] {
            assert_eq!(run(&mut initial.clone(), path), 0.6);
        }
    }
}

#[test]
fn overlapping_hazards_keep_spatial_ranking_and_ties_among_retained_contacts() {
    for reverse in [false, true] {
        let mut sources = [
            ("behind", Vec3f::new(0.75, 0.0, 0.0), 0.75, 0.8),
            ("near-side", Vec3f::new(-0.6, 0.0, 0.0), 0.75, 0.1),
        ];
        if reverse {
            sources.reverse();
        }
        let initial = world(vec![[0.37, -1.0, 0.38, 1.0, -0.5, 2.0]], &sources, &[]);
        let near = initial.entity_id("near-side").unwrap();
        let result = initial.clone().apply_command(&step(1.0)).unwrap();
        assert_eq!(result.execution.physical.target_entity, Some(near));
        assert_eq!(result.touched_entities, vec![near]);
        for path in PATHS {
            assert_eq!(run(&mut initial.clone(), path), 0.1);
        }
        let initial = world(
            vec![],
            &sources,
            &[("graze", Vec3f::new(0.375, 0.45, 0.6), 0.75)],
        );
        let behind = initial.entity_id("behind").unwrap();
        let result = initial.clone().apply_command(&step(1.0)).unwrap();
        assert_eq!(result.execution.physical.target_entity, Some(behind));
        assert_eq!(result.body_event.damage, 0.8);
        assert_eq!(result.touched_entities.len(), 2);
    }
    let initial = world(
        vec![],
        &[
            ("first", Vec3f::new(0.2, 0.0, 0.0), 0.75, 0.2),
            ("second", Vec3f::new(0.2, 0.0, 0.0), 0.75, 0.8),
        ],
        &[],
    );
    let result = initial.clone().apply_command(&step(1.0)).unwrap();
    assert_eq!(
        result.execution.physical.target_entity,
        initial.entity_id("first")
    );
    assert_eq!(result.body_event.damage, 0.2);
}

#[test]
fn ecological_pressure_is_unchanged_when_an_object_hazard_is_shielded() {
    let mut initial = world(vec![[0.37, -1.0, 0.38, 1.0, -0.5, 2.0]], &[source()], &[]);
    initial
        .add_terrain_zone(
            TerrainZone::new(
                EcologyZoneId(1),
                "field",
                TerrainZoneKind::HazardField,
                Vec3f::new(0.75, 0.0, 0.0),
                1.0,
                0.0,
                0.4,
            )
            .unwrap(),
        )
        .unwrap();
    for path in [Path::Direct, Path::Registered, Path::MotorMove] {
        assert_eq!(run(&mut initial.clone(), path), 0.4);
    }
    assert_eq!(
        initial
            .apply_command(&step(1.0))
            .unwrap()
            .execution
            .physical
            .contact,
        PhysicalContactKind::Moved
    );
}

#[test]
fn portable_restore_preserves_identity_biology_and_the_same_shielding_and_fallback() {
    for graze in [false, true] {
        let mut original = if graze {
            world(
                vec![],
                &[source()],
                &[("graze", Vec3f::new(0.375, 0.45, 0.6), 0.75)],
            )
        } else {
            world(vec![[0.37, -1.0, 0.38, 1.0, -0.5, 2.0]], &[source()], &[])
        };
        let agent = register(&mut original);
        let hazard = original.entity_id("thorn").unwrap();
        let file = PortableSaveFile::from_headless_world(
            "hazard-occlusion",
            &original,
            RuntimeConfig::deterministic_default(original.seed(), BrainScaleTier::Nano512),
            AssetManifest::empty(),
            vec![],
        )
        .unwrap();
        let loaded =
            PortableSaveFile::from_json_str(&file.to_json_string_pretty().unwrap()).unwrap();
        let mut restored = loaded.restore_headless_world().unwrap();
        assert_eq!(restored.object_snapshots(), original.object_snapshots());
        assert_eq!(
            restored.organism_registry().get(OrganismId(1)),
            original.organism_registry().get(OrganismId(1))
        );
        let before = restored.entity(hazard).unwrap().clone();
        let receipt = restored
            .apply_registered_neural_command(&step(1.0), agent, Tick(1), None, false)
            .unwrap();
        assert_eq!(
            receipt.action_result.body_event.damage,
            if graze { 0.6 } else { 0.0 }
        );
        assert_eq!(restored.entity(hazard), Some(&before));
    }
}

#[test]
fn dense_search_fallback_keeps_damage_and_is_independent_of_box_order() {
    let project = |t: f32| 0.015625 + 0.06875 * t;
    let z1 = f32::from_bits(0.25_f32.to_bits() + 1);
    let boxes: Vec<_> = (0..64)
        .map(|i| {
            [
                project(i as f32 / 64.0) - 0.000001,
                0.25,
                project((i + 1) as f32 / 64.0) + 0.000001,
                z1,
                -0.5,
                2.0,
            ]
        })
        .collect();
    for reverse in [false, true] {
        let mut boxes = boxes.clone();
        if reverse {
            boxes.reverse();
        }
        let initial = world(
            boxes,
            &[("thorn", Vec3f::new(0.05, 0.0, 0.8), 0.805, 0.6)],
            &[],
        );
        let hazard = initial.entity_id("thorn").unwrap();
        for path in PATHS {
            let mut trial = initial.clone();
            assert_eq!(
                run(&mut trial, path),
                if matches!(path, Path::MotorIdle) {
                    0.0
                } else {
                    0.6
                }
            );
        }
        let result = initial.clone().apply_command(&step(1.0)).unwrap();
        assert_eq!(result.touched_entities, vec![hazard]);
        assert_eq!(result.execution.physical.target_entity, Some(hazard));
    }
}
