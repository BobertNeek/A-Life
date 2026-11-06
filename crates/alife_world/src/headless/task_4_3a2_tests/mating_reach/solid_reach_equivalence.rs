use super::*;

// Frozen pre-filter implementation from main de709823. Keep the existing
// narrow-phase predicates, ordering, radius floor and vertical bounds exact.
fn original(
    world: &HeadlessWorld,
    start: Vec3f,
    end: Vec3f,
    ignored: Option<WorldEntityId>,
) -> bool {
    let length = distance(start, end);
    if world
        .objects
        .values()
        .any(|object| Some(object.id) != ignored && object.blocks_segment(start, end))
    {
        return false;
    }
    let Some(terrain) = world.terrain.as_ref() else {
        return true;
    };
    if length <= f32::EPSILON {
        return !terrain
            .surface()
            .obstacles
            .iter()
            .any(|bounds| sight_box_hit(start, Vec3f::ZERO, *bounds, 0.0).is_some());
    }
    let direction = scale(subtract(end, start), 1.0 / length);
    !terrain
        .surface()
        .obstacles
        .iter()
        .any(|bounds| sight_box_hit(start, direction, *bounds, length).is_some())
}

fn fixture() -> (HeadlessWorld, WorldEntityId) {
    let mut world = HeadlessScenarioBuilder::new(43_281)
        .agent("agent", OrganismId(1), Vec3f::new(5.0, 0.0, 0.0))
        .food("food", Vec3f::ZERO, 0.6)
        .obstacle("rock", Vec3f::ZERO, 0.75)
        .build()
        .unwrap();
    let rock = world.entity_id("rock").unwrap();
    world.editor_set_optical_opacity(rock, 0.0).unwrap();
    (world, rock)
}

fn compare(
    world: &HeadlessWorld,
    start: Vec3f,
    end: Vec3f,
    ignored: Option<WorldEntityId>,
) -> bool {
    let expected = original(world, start, end, ignored);
    assert_eq!(
        world.solid_segment_clear_from(start, end, ignored, world.objects.values()),
        expected,
        "start={start:?} end={end:?} ignored={ignored:?}"
    );
    assert_eq!(
        world.solid_segment_clear_from(start, end, ignored, world.objects.values().rev()),
        expected
    );
    for center in [start, end] {
        assert_eq!(
            world.mating_segment_clear(
                center,
                start,
                end,
                ignored,
                &std::cell::OnceCell::new(),
            ),
            expected,
            "candidate center={center:?} start={start:?} end={end:?}"
        );
    }
    expected
}

#[test]
fn local_candidates_preserve_domain_edges_large_spheres_and_cell_faces() {
    let (mut world, rock) = fixture();
    for center_x in [
        -4096.0,
        (-4096.0_f32).next_down(),
        -10.0,
        0.0,
        10.0,
        4096.0,
        4096.0_f32.next_up(),
    ] {
        let start = Vec3f::new(center_x, 0.0, 0.0);
        let end = Vec3f::new(center_x + 0.5, 0.0, 0.0);
        for radius in [0.1, 0.75, 32.0, 65536.0, 65536.0_f32.next_up(), 1.0e18] {
            for tangent_y in [radius, radius.next_down(), radius.next_up()] {
                let object = world.objects.get_mut(&rock.raw()).unwrap();
                object.position = Vec3f::new(center_x + 0.25, tangent_y, 0.0);
                object.radius = radius;
                compare(&world, start, end, None);
                compare(&world, end, start, None);
            }
        }
        world.objects.get_mut(&rock.raw()).unwrap().consumed = true;
        for face in [center_x, center_x.next_down(), center_x.next_up()] {
            world.terrain = Some(terrain(vec![
                [face, -0.1, face + 0.25, 0.1, -1.0, 1.0]; 32
            ]));
            compare(&world, start, end, None);
            compare(&world, end, start, None);
            compare(&world, start, start, None);
        }
        world.terrain = None;
        world.objects.get_mut(&rock.raw()).unwrap().consumed = false;
    }
}

#[test]
fn sparse_terrain_cell_faces_use_pruned_index_candidates() {
    let (mut world, rock) = fixture();
    world.objects.get_mut(&rock.raw()).unwrap().consumed = true;
    for face in [-4090.0_f32, -10.0, 0.0, 10.0, 4090.0] {
        let mut boxes = vec![[face, -0.1, face + 0.1, 0.1, -1.0, 1.0]];
        boxes.extend((0..64).map(|i| {
            let x = 10000.0 + i as f32 * 20.0;
            [x, 100.0, x + 1.0, 101.0, -1.0, 1.0]
        }));
        world.terrain = Some(terrain(boxes));
        for end_x in [face.next_down(), face, face.next_up()] {
            let start = Vec3f::new(face - 0.5, 0.0, 0.0);
            let end = Vec3f::new(end_x, 0.0, 0.0);
            for (a, b) in [(start, end), (end, start)] {
                let candidates = std::cell::OnceCell::new();
                assert_eq!(
                    world.mating_segment_clear(a, a, b, None, &candidates),
                    original(&world, a, b, None),
                );
                assert_eq!(
                    candidates.get().unwrap().as_ref().unwrap().candidate_counts(),
                    (0, 1)
                );
            }
        }
    }
}

#[test]
fn cached_candidates_fall_back_for_later_unsupported_pairs() {
    let (mut world, rock) = fixture();
    world.objects.get_mut(&rock.raw()).unwrap().position = Vec3f::new(10.0, 0.0, 0.0);
    world.terrain = Some(terrain(vec![[19.5, -0.2, 20.5, 0.2, -1.0, 1.0]]));
    let candidates = std::cell::OnceCell::new();
    assert!(world.mating_segment_clear(
        Vec3f::ZERO,
        Vec3f::ZERO,
        Vec3f::new(0.5, 0.0, 0.0),
        None,
        &candidates,
    ));
    for (start, end) in [
        (Vec3f::new(9.5, 0.0, 0.0), Vec3f::new(10.0, 0.0, 0.0)),
        (Vec3f::new(19.0, 0.0, 0.0), Vec3f::new(20.0, 0.0, 0.0)),
        (
            Vec3f::new(1.0e18, 0.0, 0.0),
            Vec3f::new(1.0e18, 0.0, 0.0),
        ),
        (Vec3f::new(f32::NAN, 0.0, 0.0), Vec3f::ZERO),
    ] {
        assert_eq!(
            world.mating_segment_clear(Vec3f::ZERO, start, end, None, &candidates),
            original(&world, start, end, None),
        );
    }
}

#[test]
fn local_candidates_match_original_over_bounded_nearby_geometry() {
    let (mut world, rock) = fixture();
    let mut state = 0x281_4096_u64;
    let mut clear = 0;
    let mut blocked = 0;
    for iteration in 0..4096 {
        let start = position(&mut state, 4096.0);
        let end = add(start, position(&mut state, 0.55));
        let object = world.objects.get_mut(&rock.raw()).unwrap();
        object.radius = [0.1, 0.75, 32.0, 65536.0][iteration % 4];
        object.position = add(start, position(&mut state, object.radius * 1.2));
        let x = start.x + (draw(&mut state) - 0.5);
        let z = start.z + (draw(&mut state) - 0.5);
        world.terrain = Some(terrain(vec![
            [x, z, x + 0.1, z + 0.1, start.y - 0.1, start.y + 0.1],
        ]));
        for (a, b) in [(start, end), (end, start)] {
            if compare(&world, a, b, None) {
                clear += 1;
            } else {
                blocked += 1;
            }
        }
    }
    assert!(clear > 100 && blocked > 100);
}

#[test]
fn early_filter_preserves_endpoints_tangency_radius_floor_and_consumption() {
    let (mut world, rock) = fixture();
    for (position, radius, start, end) in [
        (
            Vec3f::new(1.75, 0.0, 0.0),
            0.75,
            Vec3f::ZERO,
            Vec3f::new(1.0, 0.0, 0.0),
        ),
        (
            Vec3f::new(0.5, 0.75, 0.0),
            0.1,
            Vec3f::ZERO,
            Vec3f::new(1.0, 0.0, 0.0),
        ),
        (
            Vec3f::new(0.5, 0.75_f32.next_up(), 0.0),
            0.75,
            Vec3f::ZERO,
            Vec3f::new(1.0, 0.0, 0.0),
        ),
        (
            Vec3f::new(0.5, 32.0, 0.0),
            32.0,
            Vec3f::ZERO,
            Vec3f::new(1.0, 0.0, 0.0),
        ),
        (
            Vec3f::new(0.5, 32.0_f32.next_down(), 0.0),
            32.0,
            Vec3f::ZERO,
            Vec3f::new(1.0, 0.0, 0.0),
        ),
        (Vec3f::ZERO, 0.75, Vec3f::ZERO, Vec3f::ZERO),
        (
            Vec3f::new(0.0, 1.75, 0.0),
            0.75,
            Vec3f::ZERO,
            Vec3f::new(0.0, 1.0, 0.0),
        ),
    ] {
        for consumed in [false, true] {
            let object = world.objects.get_mut(&rock.raw()).unwrap();
            object.position = position;
            object.radius = radius;
            object.consumed = consumed;
            for ignored in [None, Some(rock)] {
                compare(&world, start, end, ignored);
                compare(&world, end, start, ignored);
            }
        }
    }
    let object = world.objects.get_mut(&rock.raw()).unwrap();
    object.position = Vec3f::ZERO;
    object.radius = 32.0;
    object.consumed = false;
    assert!(!compare(&world, Vec3f::ZERO, Vec3f::ZERO, None));
    world.objects.get_mut(&rock.raw()).unwrap().consumed = true;
    assert!(compare(&world, Vec3f::ZERO, Vec3f::ZERO, None));
}

#[test]
fn early_filter_preserves_terrain_faces_vertical_bounds_and_zero_length() {
    let (mut world, rock) = fixture();
    world.objects.get_mut(&rock.raw()).unwrap().consumed = true;
    world.terrain = Some(terrain(vec![
        [0.4, -0.2, 0.6, 0.2, 1.0, 2.0],
        [4.0, 4.0, 6.0, 6.0, 0.0, 2.0],
    ]));
    for height in [0.0, 1.0_f32.next_down(), 1.0, 2.0, 2.0_f32.next_up()] {
        let start = Vec3f::new(0.0, height, 0.0);
        let end = Vec3f::new(1.0, height, 0.0);
        let clear = compare(&world, start, end, None);
        assert_eq!(clear, !(1.0..=2.0).contains(&height));
        compare(&world, end, start, None);
        compare(
            &world,
            Vec3f::new(0.4, height, 0.0),
            Vec3f::new(0.4, height, 0.0),
            None,
        );
    }
    compare(
        &world,
        Vec3f::new(3.79, 0.0, 4.25),
        Vec3f::new(4.25, 0.0, 3.79),
        None,
    );
    compare(
        &world,
        Vec3f::new(3.79, 0.0, 4.10),
        Vec3f::new(4.10, 0.0, 3.79),
        None,
    );
}

fn draw(state: &mut u64) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state >> 40) as f32 / (1_u32 << 24) as f32
}

fn position(state: &mut u64, scale: f32) -> Vec3f {
    Vec3f::new(
        (draw(state) * 2.0 - 1.0) * scale,
        (draw(state) * 2.0 - 1.0) * scale,
        (draw(state) * 2.0 - 1.0) * scale,
    )
}

#[test]
fn early_filter_matches_original_over_deterministic_mixed_geometry() {
    let (mut world, rock) = fixture();
    world.terrain = Some(terrain(vec![[-2.0, -2.0, 2.0, 2.0, -0.5, 0.5]]));
    let mut state = 0x0432_8106_u64;
    let mut clear = 0;
    let mut blocked = 0;
    for scale in [0.0001, 1.0, 1000.0, 1.0e8, 1.0e18] {
        for iteration in 0..1024 {
            let start = position(&mut state, scale);
            let end = if iteration % 8 == 0 {
                start
            } else {
                position(&mut state, scale)
            };
            for object in world.objects.values_mut() {
                object.position = position(&mut state, scale);
                object.radius = (draw(&mut state) * scale * 2.0).max(0.01);
                object.consumed = iteration % 3 == 0;
            }
            let ignored = (iteration % 4 == 0).then_some(rock);
            for (a, b) in [(start, end), (end, start)] {
                if compare(&world, a, b, ignored) {
                    clear += 1;
                } else {
                    blocked += 1;
                }
            }
        }
    }
    assert!(clear > 100 && blocked > 100);
}
