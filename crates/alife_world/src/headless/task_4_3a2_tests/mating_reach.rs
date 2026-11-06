use super::*;

mod solid_reach_equivalence;

fn terrain(obstacles: Vec<[f32; 6]>) -> crate::WorldTerrain {
    crate::WorldTerrain::new(
        crate::TerrainData {
            width: 2,
            depth: 2,
            origin_x: 0.0,
            origin_z: 0.0,
            spacing: 10.0,
            heights: vec![0.0; 4],
            obstacles,
            water_level: None,
        },
        crate::LocomotionLimits::default(),
    )
    .unwrap()
}

fn blocked_world(kind: u8) -> (HeadlessWorld, Tick) {
    let (mut world, next_tick) = prepared_world(Vec3f::new(0.9, 0.0, 0.0), COMPATIBILITY_FAMILY_ID);
    if kind < 2 {
        let rock = world
            .editor_spawn_object(WorldEditorSpawnSpec {
                label: "blocking-rock".into(),
                kind: WorldObjectKind::Obstacle,
                organism_id: None,
                position: Vec3f::new(0.45, 0.0, 0.7),
                nutrition: 0.0,
                hazard_pain: 0.0,
                radius: 0.75,
                token_id: None,
            })
            .unwrap();
        world
            .editor_set_optical_opacity(rock, f32::from(kind))
            .unwrap();
        for id in [MATERNAL_ID, PATERNAL_ID] {
            assert!(
                !world.objects[&rock.raw()].blocks_position(world.agent_for(id).unwrap().position)
            );
        }
    } else if kind == 2 {
        world.terrain = Some(terrain(vec![[0.35, -0.2, 0.55, 0.2, 0.0, 2.0]]));
    } else {
        world.terrain = Some(terrain(vec![[4.0, 4.0, 6.0, 6.0, 0.0, 2.0]]));
        // Preserve #280's actual player-reachable solid-crossing fixture.
        for (id, position) in [
            (MATERNAL_ID, Vec3f::new(3.79, 0.0, 4.25)),
            (PATERNAL_ID, Vec3f::new(4.25, 0.0, 3.79)),
        ] {
            let entity = world.agent_entity_id(id).unwrap();
            world.begin_player_hold(entity).unwrap();
            world.move_player_hold(position).unwrap();
            assert_eq!(world.release_player_hold().unwrap(), Some(entity));
            assert_eq!(world.entity(entity).unwrap().position, position);
        }
    }
    let a = world.agent_for(MATERNAL_ID).unwrap().position;
    let b = world.agent_for(PATERNAL_ID).unwrap().position;
    assert!(distance(a, b) < HEADLESS_MATING_RADIUS);
    assert!(!world.solid_segment_clear(a, b));
    assert!(!world.solid_segment_clear(b, a));
    if let Some(terrain) = world.terrain() {
        assert!(terrain.walkable(a.x, a.z));
        assert!(terrain.walkable(b.x, b.z));
    }
    (world, next_tick)
}

#[test]
fn solids_remove_both_parents_opportunities_independently_of_opacity() {
    for kind in 0..4 {
        let (world, next_tick) = blocked_world(kind);
        let before = world.save_state();
        for (first, second) in [(MATERNAL_ID, PATERNAL_ID), (PATERNAL_ID, MATERNAL_ID)] {
            assert!(world
                .eligible_mating_pair(first, second, next_tick)
                .unwrap()
                .is_none());
            assert_eq!(world.mating_response(first, next_tick).unwrap(), 0.0);
        }
        let report = world.collect_mating_opportunities(next_tick).unwrap();
        assert_eq!(report.alive_organism_ids, vec![MATERNAL_ID, PATERNAL_ID]);
        assert!(report.eligible_pairs.is_empty());
        assert!(report.mating_organism_ids.is_empty());
        assert_eq!(world.save_state(), before);
    }
}

#[test]
fn solid_barriers_defer_birth_without_stalling_or_debiting_parental_investment() {
    for kind in 0..4 {
        let (mut world, next_tick) = blocked_world(kind);
        let before_objects = world.object_snapshots();
        let before_identity = (
            world.next_entity_id,
            world.next_organism_id,
            world.next_spawn_sequence,
        );
        let mut replay = world.clone();
        let mut separated = world.clone();
        let paternal = separated.agent_entity_id(PATERNAL_ID).unwrap();
        separated
            .editor_move_object(paternal, Vec3f::new(8.0, 0.0, 1.0))
            .unwrap();
        assert_eq!(world.try_advance_tick().unwrap(), next_tick);
        assert_eq!(replay.try_advance_tick().unwrap(), next_tick);
        assert_eq!(separated.try_advance_tick().unwrap(), next_tick);
        assert_eq!(world.organism_registry().iter().count(), 2);
        assert_eq!(world.object_snapshots(), before_objects);
        assert_eq!(
            (
                world.next_entity_id,
                world.next_organism_id,
                world.next_spawn_sequence
            ),
            before_identity
        );
        for id in [MATERNAL_ID, PATERNAL_ID] {
            let actual = world.organism_registry().get(id).unwrap();
            let control = separated.organism_registry().get(id).unwrap();
            assert_eq!(actual.biochemistry().tick, next_tick);
            assert_eq!(actual.biochemistry().reproduction.mating_opportunity, 0.0);
            assert!(!actual.biochemistry().reproduction.ready);
            // The distant control undergoes normal upkeep without birth.
            assert_eq!(
                actual.biochemistry().body.energy,
                control.biochemistry().body.energy
            );
            assert_eq!(actual.genome(), control.genome());
        }
        assert_eq!(
            world.canonical_signature_digest().unwrap(),
            replay.canonical_signature_digest().unwrap()
        );
        let serialized = serde_json::to_string(&world.save_state()).unwrap();
        let restored =
            HeadlessWorld::from_save_state(&serde_json::from_str(&serialized).unwrap()).unwrap();
        assert_eq!(restored.save_state(), world.save_state());
    }
}

fn idle_bundle(world: &HeadlessWorld, id: OrganismId) -> MotorCommandBundle {
    let idle = HeadlessWorldCommand::idle(id).unwrap();
    let channel = alife_core::channel_command_for_action(MotorChannel::Posture, &idle).unwrap();
    MotorCommandBundle::new(
        id,
        alife_core::ExperienceSequenceId(1),
        world.tick(),
        vec![channel],
    )
    .unwrap()
}

#[test]
fn motor_bundles_observe_barriers_and_clear_space_before_conception() {
    for blocked in [false, true] {
        let (mut world, next_tick) = if blocked {
            blocked_world(2)
        } else {
            prepared_world(Vec3f::new(0.9, 0.0, 0.0), COMPATIBILITY_FAMILY_ID)
        };
        let mut passive = world.clone();
        for id in [MATERNAL_ID, PATERNAL_ID] {
            let command = idle_bundle(&world, id);
            let entity = world.agent_entity_id(id).unwrap();
            let receipt = world
                .apply_registered_motor_bundle(&command, entity)
                .unwrap();
            assert_eq!(receipt.body_event.mating_opportunity > 0.0, !blocked);
            assert_eq!(receipt.biology_after.tick, next_tick);
        }
        world.try_advance_tick().unwrap();
        passive.try_advance_tick().unwrap();
        assert_eq!(
            world.organism_registry().iter().count(),
            if blocked { 2 } else { 3 }
        );
        assert_eq!(
            world.organism_registry().iter().count(),
            passive.organism_registry().iter().count()
        );
        for id in [MATERNAL_ID, PATERNAL_ID] {
            assert_eq!(
                world
                    .organism_registry()
                    .get(id)
                    .unwrap()
                    .biochemistry()
                    .reproduction,
                passive
                    .organism_registry()
                    .get(id)
                    .unwrap()
                    .biochemistry()
                    .reproduction
            );
        }
    }
}

#[test]
fn clear_mating_preserves_exact_radius_and_does_not_require_gaze() {
    for (separation, eligible) in [(0.0, true), (1.0, true), (1.0001, false)] {
        let (mut world, next_tick) =
            prepared_world(Vec3f::new(separation, 0.0, 0.0), COMPATIBILITY_FAMILY_ID);
        let maternal = world.agent_entity_id(MATERNAL_ID).unwrap();
        world.objects.get_mut(&maternal.raw()).unwrap().body_yaw = std::f32::consts::PI;
        assert_eq!(
            world
                .eligible_mating_pair(MATERNAL_ID, PATERNAL_ID, next_tick)
                .unwrap()
                .is_some(),
            eligible
        );
        world.try_advance_tick().unwrap();
        assert_eq!(
            world.organism_registry().iter().count(),
            if eligible { 3 } else { 2 }
        );
    }
}

#[test]
fn heightfield_slope_and_water_do_not_become_mating_path_requirements() {
    let (mut world, next_tick) = prepared_world(Vec3f::new(0.9, 0.0, 0.0), COMPATIBILITY_FAMILY_ID);
    let mut data = terrain(Vec::new()).surface().data();
    data.heights = vec![0.0, 20.0, 0.0, 20.0];
    data.water_level = Some(10.0);
    world.terrain =
        Some(crate::WorldTerrain::new(data, crate::LocomotionLimits::default()).unwrap());
    // This query-only imported geometry deliberately prevents walking. Contact
    // currently checks solid entities/AABBs, not slope, water, or ground height.
    assert!(!world.terrain().unwrap().walkable(0.45, 0.0));
    assert!(world.solid_segment_clear(Vec3f::ZERO, Vec3f::new(0.9, 0.0, 0.0)));
    assert!(world
        .eligible_mating_pair(MATERNAL_ID, PATERNAL_ID, next_tick)
        .unwrap()
        .is_some());
    assert!(world.mating_response(MATERNAL_ID, next_tick).unwrap() > 0.0);
}

fn benchmark_world(
    count: usize,
    clustered: bool,
    objects: usize,
    boxes: usize,
) -> (HeadlessWorld, Tick) {
    let (mut world, next_tick) = prepared_world(Vec3f::new(0.9, 0.0, 0.0), COMPATIBILITY_FAMILY_ID);
    let template = world.organism_registry().get(MATERNAL_ID).unwrap().clone();
    let mut habitats = world.habitat_authority().clone();
    for index in 2..count {
        let id = OrganismId(100 + index as u64);
        let entity = world
            .spawn_social_agent(&format!("bench-{index}"), id, Vec3f::ZERO, 0.0)
            .unwrap();
        world
            .register_organism_record(
                WorldOrganismRecord::new(
                    id,
                    entity,
                    template.genome().clone(),
                    template.phenotype().clone(),
                    *template.biochemistry(),
                    template.birth_tick(),
                )
                .unwrap(),
            )
            .unwrap();
        habitats
            .register_creature(id, crate::habitat::HabitatId::DEFAULT_WILD, Tick::ZERO)
            .unwrap();
    }
    world.replace_habitat_authority(habitats).unwrap();
    for (index, id) in world.alive_organism_ids().into_iter().enumerate() {
        let entity = world.agent_entity_id(id).unwrap();
        let spacing = if clustered { 0.01 } else { 2.0 };
        world
            .editor_move_object(entity, Vec3f::new(index as f32 * spacing, 0.0, 0.0))
            .unwrap();
    }
    if objects > 0 {
        for index in 0..objects {
            world
                .editor_spawn_object(WorldEditorSpawnSpec {
                    label: format!("clutter-{index}"),
                    kind: if index < objects / 3 {
                        WorldObjectKind::Obstacle
                    } else {
                        WorldObjectKind::Food
                    },
                    organism_id: None,
                    position: Vec3f::new(200.0 + index as f32 * 2.0, 0.0, 0.0),
                    nutrition: 0.0,
                    hazard_pain: 0.0,
                    radius: 0.75,
                    token_id: None,
                })
                .unwrap();
        }
    }
    if boxes > 0 {
        let obstacles = (0..boxes)
            .map(|index| {
                let x = 200.0 + index as f32 * 2.0;
                [x, 5.0, x + 1.0, 6.0, 0.0, 2.0]
            })
            .collect();
        let mut data = terrain(obstacles).surface().data();
        data.spacing = 2000.0;
        world.terrain =
            Some(crate::WorldTerrain::new(data, crate::LocomotionLimits::default()).unwrap());
    }
    (world, next_tick)
}

#[test]
#[ignore = "bounded CPU component timing, not a gameplay throughput benchmark"]
fn mating_query_cpu_cost() {
    use std::{hint::black_box, time::Instant};
    for count in [2, 50] {
        for clustered in [false, true] {
            for (objects, boxes) in [(0, 0), (192, 0), (192, 64), (768, 256)] {
                let (world, next_tick) = benchmark_world(count, clustered, objects, boxes);
                let expected = if clustered {
                    count * (count - 1) / 2
                } else {
                    0
                };
                assert_eq!(
                    world
                        .collect_mating_opportunities(next_tick)
                        .unwrap()
                        .eligible_pairs
                        .len(),
                    expected
                );
                let ids = world.alive_organism_ids();
                let before = world.canonical_signature_digest().unwrap();
                let collect = || {
                    black_box(
                        world
                            .collect_mating_opportunities(black_box(next_tick))
                            .unwrap(),
                    );
                };
                let responses = || {
                    for id in &ids {
                        black_box(
                            world
                                .mating_response(black_box(*id), black_box(next_tick))
                                .unwrap(),
                        );
                    }
                };
                for _ in 0..5 {
                    collect();
                    responses();
                }
                let started = Instant::now();
                for _ in 0..20 {
                    collect();
                }
                let collect_ns = started.elapsed().as_nanos() / 20;
                let started = Instant::now();
                for _ in 0..20 {
                    responses();
                }
                let responses_ns = started.elapsed().as_nanos() / 20;
                println!(
                    "MATING_TIMING {}",
                    serde_json::json!({
                        "organisms": count, "clustered": clustered,
                        "objects": world.object_count(), "obstacles": objects / 3,
                        "terrain_boxes": boxes, "eligible_pairs": expected,
                        "collect_ns": collect_ns, "responses_ns": responses_ns,
                    })
                );
                assert_eq!(world.canonical_signature_digest().unwrap(), before);
            }
        }
    }
}
