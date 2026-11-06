//! Manual CPU cost qualification; no inference, training, or live saves.
use alife_core::{
    ActionCommand, ActionKind, ActionTarget, BrainCapacityClass, Confidence, CreatureGenome,
    DurationTicks, ExperienceSequenceId, FoundationGeneticIdentity, Intensity, MotorChannel,
    MotorCommandBundle, OrganismId, Tick, Vec3f,
};
use alife_world::{
    HeadlessActionIds, HeadlessScenarioBuilder, HeadlessWorld, LocomotionLimits, TerrainData,
    WorldEditorSpawnSpec, WorldObjectKind, WorldOrganismRecord, WorldTerrain,
};
use std::{hint::black_box, time::Instant};

fn step() -> ActionCommand {
    ActionCommand::structured(
        OrganismId(1),
        HeadlessActionIds::STEP_FORWARD,
        ActionKind::Move,
        ActionTarget::new(None, None),
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
fn fixture(scene: &str) -> HeadlessWorld {
    let mut world = HeadlessScenarioBuilder::new(4251)
        .agent("creature", OrganismId(1), Vec3f::ZERO)
        .build()
        .unwrap();
    let (source, radius, body) = if scene.starts_with("island") {
        let terrain = alife_world::island_terrain();
        let surface = terrain.surface();
        let source = Vec3f::new(89.0, surface.height(89.0, 256.0).unwrap(), 256.0);
        let x = source.x + 0.75 * 0.85;
        let body = Vec3f::new(x, surface.height(x, 256.0).unwrap(), 256.0);
        let mut data = surface.data();
        if scene == "island_wall" {
            let x = (body.x + source.x) * 0.5;
            data.obstacles.push([
                x - 0.005,
                255.0,
                x + 0.005,
                257.0,
                source.y - 0.5,
                source.y + 2.0,
            ]);
        }
        world
            .enable_terrain_for_new_game(
                WorldTerrain::new(data, LocomotionLimits::default()).unwrap(),
                Vec3f::new(80.0, 0.0, 250.0),
            )
            .unwrap();
        (source, 0.75, body)
    } else if scene == "slats64" {
        let project = |t: f32| 0.015625 + 0.06875 * t;
        let z1 = f32::from_bits(0.25_f32.to_bits() + 1);
        let obstacles = (0..64)
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
                        obstacles,
                        water_level: None,
                    },
                    LocomotionLimits::default(),
                )
                .unwrap(),
                Vec3f::ZERO,
            )
            .unwrap();
        (Vec3f::new(0.05, 0.0, 0.8), 0.805, Vec3f::ZERO)
    } else {
        (Vec3f::new(0.75, 0.0, 0.0), 0.75, Vec3f::ZERO)
    };
    world
        .editor_spawn_object(WorldEditorSpawnSpec {
            label: "thorn".into(),
            kind: WorldObjectKind::Hazard,
            organism_id: None,
            position: source,
            nutrition: 0.0,
            hazard_pain: 0.12,
            radius,
            token_id: None,
        })
        .unwrap();
    let agent = world.entity_id("creature").unwrap();
    world.editor_move_object(agent, body).unwrap();
    assert_eq!(world.entity(agent).unwrap().position, body);
    let genome = CreatureGenome::early_mammal_founder(
        4251,
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
    world
}

#[test]
#[ignore = "manual CPU action/motor comparison; run with --ignored --nocapture"]
fn public_hazard_action_and_motor_cost() {
    let shielding = std::env::var_os("ALIFE_EXPECT_HAZARD_SHIELDING").is_some();
    for scene in ["flat_open", "island_open", "island_wall", "slats64"] {
        let world = fixture(scene);
        let agent = world.entity_id("creature").unwrap();
        let hazard = world.entity_id("thorn").unwrap();
        let object = world.entity(hazard).unwrap().clone();
        let moving = MotorCommandBundle::new(
            OrganismId(1),
            ExperienceSequenceId::new(1).unwrap(),
            Tick::ZERO,
            vec![
                alife_core::channel_command_for_action(MotorChannel::Locomotion, &step()).unwrap(),
            ],
        )
        .unwrap();
        let idle = MotorCommandBundle::new(
            OrganismId(1),
            ExperienceSequenceId::new(1).unwrap(),
            Tick::ZERO,
            vec![],
        )
        .unwrap();
        let boxes = world
            .terrain()
            .map_or(0, |t| t.surface().data().obstacles.len());
        for mode in ["direct", "motor_move", "motor_idle"] {
            let expected = if shielding
                && (scene == "island_wall" || (scene == "slats64" && mode == "motor_idle"))
            {
                0.0
            } else {
                0.12
            };
            for round in 0..7 {
                let mut elapsed = 0;
                let repeats = 128;
                for _ in 0..repeats {
                    let mut trial = world.clone(); // Setup is excluded from the timed action.
                    let started = Instant::now();
                    let damage = match mode {
                        "direct" => {
                            trial
                                .apply_command(black_box(&step()))
                                .unwrap()
                                .body_event
                                .damage
                        }
                        "motor_move" => {
                            trial
                                .apply_registered_motor_bundle(black_box(&moving), agent)
                                .unwrap()
                                .body_event
                                .damage
                        }
                        _ => {
                            trial
                                .apply_registered_motor_bundle(black_box(&idle), agent)
                                .unwrap()
                                .body_event
                                .damage
                        }
                    };
                    elapsed += started.elapsed().as_nanos();
                    assert_eq!(damage, expected, "scene={scene} mode={mode}");
                    assert_eq!(trial.entity(hazard), Some(&object));
                    black_box(trial);
                }
                println!("hazard_public_cpu scene={scene} mode={mode} round={round} boxes={boxes} shielding={shielding} damage={expected} ns={}",elapsed/repeats);
            }
        }
    }
}
