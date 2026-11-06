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
    expected
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
