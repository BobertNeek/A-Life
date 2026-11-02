use alife_core::Vec3f;
use alife_world::{LocomotionLimits, TerrainData, WorldTerrain};

#[test]
fn movement_does_not_skip_a_valid_fine_grid_water_strip() {
    let mut heights = vec![1.0; 62];
    heights[5] = -1.0;
    heights[31 + 5] = -1.0;
    let terrain = WorldTerrain::new(
        TerrainData {
            width: 31,
            depth: 2,
            origin_x: 0.0,
            origin_z: 0.0,
            spacing: 0.01,
            heights,
            obstacles: vec![],
            water_level: Some(0.0),
        },
        LocomotionLimits::default(),
    )
    .unwrap();
    let start = Vec3f::new(0.0, 1.0, 0.005);
    let end = Vec3f::new(0.28, 1.0, 0.005);
    assert!(terrain.walkable(start.x, start.z));
    assert!(terrain.walkable(end.x, end.z));
    assert!(!terrain.walkable(0.05, 0.005));
    // Custom terrain survives the same checked save/restore path as the island.
    let restored = WorldTerrain::restore(Some(terrain.binding()), Some(&terrain.state()))
        .unwrap()
        .unwrap();
    assert_eq!(restored.binding(), terrain.binding());
    assert!(restored.resolve_move(start, end).is_none());
}

fn surface(data: TerrainData, limits: LocomotionLimits) -> WorldTerrain {
    WorldTerrain::new(data, limits).unwrap()
}

fn flat(width: usize, depth: usize, spacing: f32) -> TerrainData {
    TerrainData {
        width,
        depth,
        origin_x: 0.0,
        origin_z: 0.0,
        spacing,
        heights: vec![1.0; width * depth],
        obstacles: vec![],
        water_level: None,
    }
}

#[test]
fn water_and_slope_barriers_are_independently_swept_in_both_directions() {
    for water_only in [false, true] {
        let mut data = flat(31, 2, 0.01);
        data.heights[5] = -1.0;
        data.heights[36] = -1.0;
        let mut limits = LocomotionLimits::default();
        if water_only {
            data.water_level = Some(0.0);
            limits.max_slope = f32::MAX;
        }
        let terrain = surface(data, limits);
        let a = Vec3f::new(0.0, 1.0, 0.005);
        let b = Vec3f::new(0.28, 1.0, 0.005);
        assert!(terrain.walkable(a.x, a.z) && terrain.walkable(b.x, b.z));
        assert!(terrain.resolve_move(a, b).is_none());
        assert!(terrain.resolve_move(b, a).is_none());
    }
}

#[test]
fn diagonal_sweep_catches_a_narrow_valley_at_positive_and_negative_origins() {
    for origin in [0.0, -0.15] {
        for water_only in [false, true] {
            let mut data = flat(31, 31, 0.01);
            data.origin_x = origin;
            data.origin_z = origin;
            data.heights[5 * 31 + 5] = -1.0;
            let mut limits = LocomotionLimits::default();
            if water_only {
                data.water_level = Some(0.0);
                limits.max_slope = f32::MAX;
            }
            let terrain = surface(data, limits);
            let a = Vec3f::new(origin, 1.0, origin);
            let b = Vec3f::new(origin + 0.28, 1.0, origin + 0.28);
            assert!(terrain.walkable(a.x, a.z) && terrain.walkable(b.x, b.z));
            assert!(terrain.resolve_move(a, b).is_none());
            assert!(terrain.resolve_move(b, a).is_none());
        }
    }
}

#[test]
fn diagonal_and_grid_boundary_ownership_and_start_escape_are_preserved() {
    let terrain = surface(
        TerrainData {
            heights: vec![1.0, 5.0, 1.0, 1.0],
            ..flat(2, 2, 1.0)
        },
        Default::default(),
    );
    for (a, b) in [(0.1, 0.9), (0.9, 0.1)] {
        // Along the diagonal, the v >= u (flat) triangle owns the path.
        assert!(terrain
            .resolve_move(Vec3f::new(a, 1.0, a), Vec3f::new(b, 1.0, b))
            .is_some());
        // Nearby distinct intervals must not be merged with that diagonal.
        assert!(terrain
            .resolve_move(Vec3f::new(a, 1.0, a - 1e-7), Vec3f::new(b, 1.0, b - 1e-7))
            .is_none());
    }
    let terrain = surface(
        TerrainData {
            heights: vec![1.0, 1.0, 4.0, 1.0, 1.0, 4.0],
            ..flat(3, 2, 1.0)
        },
        Default::default(),
    );
    let edge = Vec3f::new(1.0, 1.0, 0.25);
    let left = Vec3f::new(0.5, 1.0, 0.25);
    assert!(!terrain.walkable(edge.x, edge.z));
    // Do not add a start-point slope gate: the outgoing left triangle is flat.
    assert_eq!(terrain.resolve_move(edge, left), Some(left));
    // The existing endpoint gradient check is retained.
    assert!(terrain.resolve_move(left, edge).is_none());
    let terrain = surface(
        TerrainData {
            heights: vec![4.0, 1.0, 1.0, 4.0, 1.0, 1.0],
            ..flat(3, 2, 1.0)
        },
        Default::default(),
    );
    assert!(terrain
        .resolve_move(Vec3f::new(1.0, 1.0, 0.0), Vec3f::new(1.0, 1.0, 1.0))
        .is_some());
}

#[test]
fn decimal_spacing_keeps_the_existing_rounded_grid_boundary_owner() {
    let terrain = surface(
        TerrainData {
            heights: [4.0, 4.0, 4.0, 4.0, 4.0, 1.0, 1.0].repeat(2),
            ..flat(7, 2, 0.1)
        },
        Default::default(),
    );
    let a = Vec3f::new(0.5, 1.0, 0.01);
    let b = Vec3f::new(0.5, 1.0, 0.09);
    assert!(terrain.walkable(a.x, a.z) && terrain.walkable(b.x, b.z));
    assert_eq!(terrain.resolve_move(a, b), Some(b));
    assert_eq!(terrain.resolve_move(b, a), Some(a));
}

#[test]
fn translated_diagonal_keeps_its_flat_triangle_owner() {
    let mut data = flat(5, 4, 0.03125);
    data.heights[5 + 3] = 5.0;
    let terrain = surface(data, Default::default());
    // u-v is exactly 1 along the whole path. Computing two local fractions
    // separately can spuriously select the steep triangle below the diagonal.
    let a = Vec3f::new(0.03125, 1.0, 0.0);
    let b = Vec3f::new(0.09765625, 1.0, 0.06640625);
    assert!(terrain.walkable(a.x, a.z) && terrain.walkable(b.x, b.z));
    assert_eq!(terrain.resolve_move(a, b), Some(b));
    assert_eq!(terrain.resolve_move(b, a), Some(a));
}

#[test]
fn ordinary_flat_paths_corners_zero_distance_and_rounded_edges_still_work() {
    let data = TerrainData {
        origin_x: -1.0,
        origin_z: -1.0,
        water_level: Some(1.6),
        ..flat(5, 5, 0.5)
    };
    let terrain = surface(data, Default::default());
    for a in [
        Vec3f::new(-1.0, 9.0, -1.0),
        Vec3f::new(0.0, 9.0, 0.0),
        Vec3f::new(1.0, 9.0, 1.0),
    ] {
        for b in [
            Vec3f::new(-1.0, -5.0, 1.0),
            Vec3f::new(0.0, -5.0, 0.0),
            Vec3f::new(1.0, -5.0, -1.0),
        ] {
            assert_eq!(terrain.resolve_move(a, b), Some(Vec3f::new(b.x, 1.0, b.z)));
        }
    }
    let lo = 650.5266_f32;
    let spacing = 4.2522902_f32;
    let hi = lo + spacing;
    let terrain = surface(
        TerrainData {
            origin_x: lo,
            origin_z: lo,
            ..flat(2, 2, spacing)
        },
        Default::default(),
    );
    assert_eq!(
        terrain.resolve_move(Vec3f::new(lo, 1.0, lo), Vec3f::new(hi, 2.0, hi)),
        Some(Vec3f::new(hi, 1.0, hi))
    );
}

#[test]
fn sweep_rejects_excessive_traversal_and_malformed_surface_without_scanning_map() {
    let terrain = surface(flat(100_001, 2, 0.001), Default::default());
    // A short move on a large map only visits the local triangles.
    assert!(terrain
        .resolve_move(Vec3f::new(0.0, 1.0, 0.0), Vec3f::new(0.01, 1.0, 0.0))
        .is_some());
    // The work budget fails closed instead of skipping the remaining triangles.
    assert!(terrain
        .resolve_move(Vec3f::new(0.0, 1.0, 0.0), Vec3f::new(64.0, 1.0, 0.0))
        .is_none());
    let original = surface(flat(2, 2, 1.0), Default::default());
    let a = Vec3f::new(0.0, 1.0, 0.0);
    let b = Vec3f::new(0.5, 1.0, 0.5);
    for field in 0..8 {
        let mut malformed = original.surface().as_ref().clone();
        match field {
            0 => malformed.width = 0,
            1 => malformed.depth = usize::MAX,
            2 => malformed.spacing = f32::NAN,
            3 => malformed.spacing = 0.0,
            4 => malformed.origin_x = f32::NAN,
            5 => malformed.origin_z = f32::INFINITY,
            6 => malformed.origin_x = 1e20,
            _ => malformed.origin_z = 1e20,
        }
        assert!(malformed.resolve_move(a, b).is_none());
        let origin = Vec3f::new(malformed.origin_x, 1.0, malformed.origin_z);
        assert!(malformed.resolve_move(origin, origin).is_none());
    }
    for invalid in [f32::NAN, f32::INFINITY, -1.0, 65.0] {
        assert!(original
            .resolve_move(a, Vec3f::new(invalid, 1.0, 0.0))
            .is_none());
    }
}

#[test]
fn authoritative_world_blocks_the_strip_without_displacing_the_creature() {
    use alife_core::{
        ActionCommand, ActionKind, ActionTarget, Confidence, DurationTicks, Intensity, OrganismId,
        PhysicalContactKind,
    };
    use alife_world::HeadlessScenarioBuilder;
    let mut data = flat(31, 2, 0.01);
    data.heights[5] = -1.0;
    data.heights[36] = -1.0;
    data.water_level = Some(0.0);
    let mut world = HeadlessScenarioBuilder::new(17)
        .agent("walker", OrganismId(1), Vec3f::ZERO)
        .build()
        .unwrap();
    world
        .enable_terrain_for_new_game(
            surface(data, Default::default()),
            Vec3f::new(0.0, 1.0, 0.005),
        )
        .unwrap();
    let id = world.entity_id("walker").unwrap();
    let start = world.entity(id).unwrap().position;
    let command = ActionCommand::structured(
        OrganismId(1),
        ActionKind::Move.canonical_id(),
        ActionKind::Move,
        ActionTarget::new(None, Some(Vec3f::new(0.28, 1.0, 0.005))),
        Intensity::new(1.0).unwrap(),
        DurationTicks::new(1),
        Confidence::new(0.9).unwrap(),
        0,
        None,
        None,
        None,
    )
    .unwrap();
    let result = world.apply_command(&command).unwrap();
    assert!(!result.execution.succeeded);
    assert_eq!(
        result.execution.physical.contact,
        PhysicalContactKind::Blocked
    );
    assert_eq!(result.execution.physical.displacement, Vec3f::ZERO);
    assert_eq!(world.entity(id).unwrap().position, start);
}
