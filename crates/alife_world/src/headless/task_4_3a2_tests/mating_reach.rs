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
fn local_candidates_follow_queried_parent_and_do_not_survive_world_changes() {
    let (mut world, next_tick) = benchmark_world(3, true, 0, 0);
    let ids = world.alive_organism_ids();
    for (id, x) in ids.iter().zip([-0.9, 0.0, 0.9]) {
        let entity = world.agent_entity_id(*id).unwrap();
        world
            .editor_move_object(entity, Vec3f::new(x, 0.0, 0.0))
            .unwrap();
    }
    let mut rocks = Vec::new();
    for (label, x) in [("left-rock", -0.45), ("right-rock", 0.45)] {
        rocks.push(
            world
                .editor_spawn_object(WorldEditorSpawnSpec {
                    label: label.into(),
                    kind: WorldObjectKind::Obstacle,
                    organism_id: None,
                    position: Vec3f::new(x, 0.7, 0.0),
                    nutrition: 0.0,
                    hazard_pain: 0.0,
                    radius: 0.75,
                    token_id: None,
                })
                .unwrap(),
        );
    }
    // The queried middle ID is paternal for the left mate, maternal for the
    // right. A blocked earlier mate must not suppress a later clear mate.
    for (left, right) in [(true, false), (false, true), (true, true), (false, false)] {
        for (rock, active) in rocks.iter().zip([left, right]) {
            world.objects.get_mut(&rock.raw()).unwrap().consumed = !active;
        }
        let mut expected_pairs = Vec::new();
        for (index, a) in ids.iter().enumerate() {
            for b in ids.iter().skip(index + 1) {
                if let Some(pair) = world.eligible_mating_pair(*a, *b, next_tick).unwrap() {
                    expected_pairs.push((pair.maternal_id, pair.paternal_id));
                }
            }
        }
        let report = world.collect_mating_opportunities(next_tick).unwrap();
        assert_eq!(
            report
                .eligible_pairs
                .iter()
                .map(|pair| (pair.maternal_id, pair.paternal_id))
                .collect::<Vec<_>>(),
            expected_pairs
        );
        for id in &ids {
            let own = world.organism_registry().get(*id).unwrap();
            let mut expected = 0.0_f32;
            for (a, b) in &expected_pairs {
                let partner = if a == id {
                    Some(*b)
                } else if b == id {
                    Some(*a)
                } else {
                    None
                };
                if let Some(partner) = partner {
                    let other = world.organism_registry().get(partner).unwrap();
                    expected = expected.max(own.phenotype().reproduction.mate_response(
                        own.phenotype(),
                        other.phenotype(),
                        other.biochemistry().body.health,
                    ));
                }
            }
            assert_eq!(world.mating_response(*id, next_tick).unwrap(), expected);
        }
        assert_eq!(
            world.mating_response(ids[1], next_tick).unwrap() > 0.0,
            !(left && right)
        );
    }
    // Move an already queried obstacle into contact, then replace terrain.
    world.objects.get_mut(&rocks[0].raw()).unwrap().consumed = false;
    world
        .editor_move_object(rocks[0], Vec3f::new(10.0, 0.0, 0.0))
        .unwrap();
    assert!(world.mating_response(ids[0], next_tick).unwrap() > 0.0);
    world
        .editor_move_object(rocks[0], Vec3f::new(-0.45, 0.7, 0.0))
        .unwrap();
    assert_eq!(world.mating_response(ids[0], next_tick).unwrap(), 0.0);
    world.terrain = Some(terrain(vec![[0.35, -0.2, 0.55, 0.2, -1.0, 1.0]]));
    assert_eq!(world.mating_response(ids[1], next_tick).unwrap(), 0.0);
    world.terrain = None;
    assert!(world.mating_response(ids[1], next_tick).unwrap() > 0.0);
}

fn island_benchmark_world(count: usize, clustered: bool) -> (HeadlessWorld, Tick) {
    let (mut world, next_tick) = benchmark_world(count, clustered, 6, 0);
    let terrain = crate::island_terrain();
    let mut agent_index = 0;
    let mut clutter_index = 0;
    for object in world.objects.values_mut() {
        let (x, z) = if object.kind == WorldObjectKind::Agent {
            let index = agent_index;
            agent_index += 1;
            if clustered {
                (80.0 + index as f32 * 0.01, 250.0)
            } else {
                (
                    80.0 + (index % 10) as f32 * 2.0,
                    250.0 + (index / 10) as f32 * 2.0,
                )
            }
        } else {
            let x = 120.0 + clutter_index as f32 * 2.0;
            clutter_index += 1;
            (x, 250.0)
        };
        assert!(terrain.walkable(x, z));
        object.position = Vec3f::new(x, terrain.surface().height(x, z).unwrap(), z);
    }
    assert_eq!(agent_index, count);
    assert_eq!(terrain.surface().obstacles.len(), 1716);
    world.terrain = Some(terrain);
    (world, next_tick)
}

fn benchmark_cycle(world: &mut HeadlessWorld) {
    for id in world.alive_organism_ids() {
        let command = idle_bundle(world, id);
        let entity = world.agent_entity_id(id).unwrap();
        world
            .apply_registered_motor_bundle(&command, entity)
            .unwrap();
    }
    world.try_advance_tick().unwrap();
}

fn measure_mating_queries(world: HeadlessWorld, next_tick: Tick, clustered: bool, scene: &str) {
    use std::{hint::black_box, time::Instant};
    let ids = world.alive_organism_ids();
    let count = ids.len();
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
    for id in &ids {
        assert_eq!(world.mating_response(*id, next_tick).unwrap() > 0.0, clustered);
    }
    let before = world.canonical_signature_digest().unwrap();
    let collect = || {
        black_box(
            black_box(&world)
                .collect_mating_opportunities(black_box(next_tick))
                .unwrap(),
        );
    };
    let responses = || {
        for id in &ids {
            black_box(
                black_box(&world)
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

    // A full CPU world cycle includes all idle bundles, their response queries,
    // world collection and conception. Fixture cloning/setup/signatures stay
    // outside the timer; production rollback clones remain inside. Each sample
    // is one fresh-state cycle, not consecutive ticks. This does not include a
    // neural policy, rendering or GPU execution.
    let mut warm = world.clone();
    benchmark_cycle(&mut warm);
    let cycle_signature = warm.canonical_signature_digest().unwrap();
    let mut cycle_ns = 0;
    for _ in 0..3 {
        let mut copy = world.clone();
        let started = Instant::now();
        benchmark_cycle(black_box(&mut copy));
        cycle_ns += started.elapsed().as_nanos();
        assert_eq!(copy.canonical_signature_digest().unwrap(), cycle_signature);
    }
    let obstacles = world
        .objects
        .values()
        .filter(|object| object.kind == WorldObjectKind::Obstacle && !object.consumed)
        .count();
    println!(
        "MATING_TIMING {}",
        serde_json::json!({
            "scene": scene, "organisms": count, "clustered": clustered,
            "objects": world.object_count(),
            "obstacles": obstacles,
            "terrain_boxes": world.terrain().map_or(0, |terrain| terrain.surface().obstacles.len()),
            "eligible_pairs": expected, "collect_ns": collect_ns, "responses_ns": responses_ns,
            "bundle_tick_ns": cycle_ns / 3, "cycle_signature": cycle_signature,
        })
    );
    assert_eq!(world.canonical_signature_digest().unwrap(), before);
}

#[test]
#[ignore = "bounded CPU component timing, not a gameplay throughput benchmark"]
fn mating_query_cpu_cost() {
    for count in [2, 50] {
        for clustered in [false, true] {
            for (objects, boxes) in [(0, 0), (192, 0), (192, 64), (768, 256)] {
                let (world, next_tick) = benchmark_world(count, clustered, objects, boxes);
                measure_mating_queries(world, next_tick, clustered, "synthetic");
            }
        }
    }
    for count in [2, 6, 50] {
        for clustered in [false, true] {
            let (world, next_tick) = island_benchmark_world(count, clustered);
            measure_mating_queries(world, next_tick, clustered, "island");
        }
    }
}
