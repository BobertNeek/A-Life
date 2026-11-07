use super::*;

const ORGANISM: OrganismId = OrganismId(1);

fn world(radius: f32, body: Option<f32>, center: Vec3f) -> HeadlessWorld {
    let mut world = HeadlessScenarioBuilder::new(701)
        .agent("body", ORGANISM, Vec3f::ZERO)
        .obstacle("solid", center, radius)
        .build()
        .unwrap();
    if let Some(body_radius) = body {
        world.terrain = Some(
            crate::WorldTerrain::new(
                crate::TerrainData {
                    width: 3,
                    depth: 3,
                    origin_x: -10.0,
                    origin_z: -10.0,
                    spacing: 10.0,
                    heights: vec![0.0; 9],
                    obstacles: vec![],
                    water_level: None,
                },
                crate::LocomotionLimits {
                    body_radius,
                    ..Default::default()
                },
            )
            .unwrap(),
        );
    }
    world
}

fn step() -> ActionCommand {
    HeadlessWorldCommand::structured(
        ORGANISM,
        HeadlessActionIds::STEP_FORWARD,
        ActionKind::Move,
        None,
        None,
    )
    .unwrap()
}

#[test]
fn authored_small_obstacle_does_not_overblock_default_body() {
    let mut world = world(0.35, None, Vec3f::new(0.5, 0.0, 0.6));
    for _ in 0..10 {
        assert!(world.apply_command(&step()).unwrap().execution.succeeded);
    }
}

#[test]
fn declared_large_body_cannot_pass_through_small_obstacle() {
    let mut world = world(0.35, Some(0.8), Vec3f::new(0.5, 0.0, 1.0));
    assert!(!world.apply_command(&step()).unwrap().execution.succeeded);
}

#[test]
fn authored_radius_and_body_limits_define_closed_swept_sphere() {
    for radius in [0.25_f32, 0.75, 1.25] {
        for body in [0.0_f32, 0.2, 0.5, 0.8] {
            let effective = radius + body;
            for clearance in [effective.next_down(), effective, effective.next_up()] {
                let world = world(radius, Some(body), Vec3f::new(1.0, clearance, 0.0));
                assert_eq!(
                    world
                        .first_motion_blocker(Vec3f::ZERO, Vec3f::new(2.0, 0.0, 0.0))
                        .is_some(),
                    clearance <= effective,
                    "radius {radius}, body {body}, clearance {clearance}"
                );
            }
        }
    }
}

#[test]
fn overlap_stationary_and_fast_crossings_use_same_radius() {
    let world = world(0.25, Some(0.5), Vec3f::ZERO);
    for x in [0.0_f32, 0.75_f32.next_down(), 0.75, 0.75_f32.next_up()] {
        let start = Vec3f::new(x, 0.0, 0.0);
        assert_eq!(
            world.first_motion_blocker(start, start).is_some(),
            x <= 0.75
        );
        assert!(world
            .first_motion_blocker(start, Vec3f::new(4.0, 0.0, 0.0))
            .is_none());
    }
    assert!(world
        .first_motion_blocker(Vec3f::new(-1000.0, 0.0, 0.0), Vec3f::new(1000.0, 0.0, 0.0))
        .is_some());
    assert!(world
        .first_motion_blocker(
            Vec3f::new(-1000.0, 0.76, 0.0),
            Vec3f::new(1000.0, 0.76, 0.0)
        )
        .is_none());
}

#[test]
fn expanded_sphere_preserves_three_dimensional_sloping_trajectories() {
    let start = Vec3f::new(-1.0, -1.0, 0.0);
    let end = Vec3f::new(1.0, 1.0, 0.0);
    for z in [0.75_f32.next_down(), 0.75, 0.75_f32.next_up()] {
        let world = world(0.25, Some(0.5), Vec3f::new(0.0, 0.0, z));
        assert_eq!(world.first_motion_blocker(start, end).is_some(), z <= 0.75);
    }
    let low = world(0.25, Some(0.0), Vec3f::new(0.5, 0.6, 0.0));
    let wide = world(0.25, Some(0.5), Vec3f::new(0.5, 0.6, 0.0));
    assert!(low
        .first_motion_blocker(Vec3f::ZERO, Vec3f::new(1.0, 0.0, 0.0))
        .is_none());
    assert!(wide
        .first_motion_blocker(Vec3f::ZERO, Vec3f::new(1.0, 0.0, 0.0))
        .is_some());
}

#[test]
fn flat_terrain_default_and_no_terrain_agree() {
    for z in [0.5, 0.6, 1.0] {
        let mut bare = world(0.35, None, Vec3f::new(0.5, 0.0, z));
        let mut flat = world(0.35, Some(0.2), Vec3f::new(0.5, 0.0, z));
        for _ in 0..15 {
            assert_eq!(
                bare.apply_command(&step()).unwrap(),
                flat.apply_command(&step()).unwrap()
            );
        }
    }
}

#[test]
fn portable_roundtrip_keeps_authored_fields_and_corrected_continuation() {
    for body in [None, Some(0.0), Some(0.2), Some(0.5), Some(0.8)] {
        for radius in [0.35, 0.75, 1.25] {
            let mut original = world(radius, body, Vec3f::new(0.5, 0.0, 1.0));
            let save = crate::PortableSaveFile::from_headless_world(
                "body-clearance",
                &original,
                crate::RuntimeConfig::deterministic_default(
                    original.seed(),
                    alife_core::BrainScaleTier::Nano512,
                ),
                crate::AssetManifest::empty(),
                Vec::new(),
            )
            .unwrap();
            let json = serde_json::to_string(&save).unwrap();
            let save = crate::PortableSaveFile::from_json_str(&json).unwrap();
            let mut restored = save.restore_headless_world().unwrap();
            assert_eq!(original.object_snapshots(), restored.object_snapshots());
            assert_eq!(original.save_state().terrain, restored.save_state().terrain);
            for _ in 0..20 {
                assert_eq!(
                    original.apply_command(&step()).unwrap(),
                    restored.apply_command(&step()).unwrap()
                );
            }
            assert_eq!(original.object_snapshots(), restored.object_snapshots());
        }
    }
}

#[test]
fn body_limits_do_not_change_fixed_pose_sensing_or_hand_contact_queries() {
    let narrow = world(0.35, Some(0.0), Vec3f::new(0.5, 0.0, 0.6));
    let wide = world(0.35, Some(0.8), Vec3f::new(0.5, 0.0, 0.6));
    let snapshot = |world: &HeadlessWorld| {
        world
            .physical_observation_snapshot_from_objects(
                ORGANISM,
                world.tick(),
                world.agent_for(ORGANISM).unwrap(),
                world.objects.values(),
                true,
            )
            .unwrap()
    };
    assert_eq!(snapshot(&narrow), snapshot(&wide));
    assert_eq!(
        narrow.sensory_report(ORGANISM, narrow.tick()).unwrap(),
        wide.sensory_report(ORGANISM, wide.tick()).unwrap()
    );
    for end in [Vec3f::new(1.0, 0.0, 0.0), Vec3f::new(0.0, 0.0, -1.0)] {
        assert_eq!(
            narrow.physical_contact_reachable(Vec3f::ZERO, end),
            wide.physical_contact_reachable(Vec3f::ZERO, end)
        );
        assert_eq!(
            narrow.solid_segment_clear(Vec3f::ZERO, end),
            wide.solid_segment_clear(Vec3f::ZERO, end)
        );
    }
}

#[test]
fn carrying_still_checks_body_and_unchanged_grip_sweep() {
    for (body, center, body_blocked) in [
        (0.8, Vec3f::new(0.5, 0.0, 1.0), true),
        (0.0, Vec3f::new(0.9, 0.0, 0.0), false),
    ] {
        let mut world = world(0.35, Some(body), Vec3f::new(8.0, 0.0, 8.0));
        let food = world
            .editor_spawn_object(WorldEditorSpawnSpec {
                label: "held".into(),
                kind: WorldObjectKind::Food,
                organism_id: None,
                position: Vec3f::new(-0.6, 0.0, 0.0),
                nutrition: 0.6,
                hazard_pain: 0.0,
                radius: 0.1,
                token_id: None,
            })
            .unwrap();
        let grab = HeadlessWorldCommand::structured(
            ORGANISM,
            HeadlessActionIds::GRAB,
            ActionKind::Hold,
            Some(food),
            None,
        )
        .unwrap();
        assert!(world.apply_command(&grab).unwrap().execution.succeeded);
        let solid = world.entity_id("solid").unwrap();
        world.editor_move_object(solid, center).unwrap();
        let before = world.object_snapshots();
        assert_eq!(
            world
                .first_motion_blocker(Vec3f::ZERO, Vec3f::new(MOVE_STEP, 0.0, 0.0))
                .is_some(),
            body_blocked
        );
        let result = world.apply_command(&step()).unwrap();
        assert!(!result.execution.succeeded);
        assert_eq!(
            result.execution.physical.target_entity,
            body_blocked.then_some(solid)
        );
        for previous in before {
            let current = world.entity(previous.id).unwrap();
            assert_eq!(previous.position, current.position);
            assert_eq!(previous.body_yaw, current.body_yaw);
            assert_eq!(previous.carried_by, current.carried_by);
        }
    }
}

fn portable(world: &HeadlessWorld) -> HeadlessWorld {
    let save = crate::PortableSaveFile::from_headless_world(
        "legacy-clearance-pose",
        world,
        crate::RuntimeConfig::deterministic_default(
            world.seed(),
            alife_core::BrainScaleTier::Nano512,
        ),
        crate::AssetManifest::empty(),
        Vec::new(),
    )
    .unwrap();
    crate::PortableSaveFile::from_json_str(&serde_json::to_string(&save).unwrap())
        .unwrap()
        .restore_headless_world()
        .unwrap()
}

#[test]
fn compatibility_old_legal_pose_restores_and_escapes_monotonically() {
    // Explicit old-world legal pose: .85 > legacy .75, but .85 < .75 + .2.
    // Construct the persisted pose without executing corrected movement first.
    let old = world(0.75, None, Vec3f::new(-0.85, 0.0, 0.0));
    assert!(!old
        .entity(old.entity_id("solid").unwrap())
        .unwrap()
        .blocks_position(Vec3f::ZERO));
    let mut restored = portable(&old);
    for _ in 0..4 {
        let result = restored.apply_command(&step()).unwrap();
        assert!(result.execution.succeeded);
        assert!(result.execution.physical.displacement.x > 0.0);
    }
    let mut edited = old.clone();
    let actor = edited.agent_entity_id(ORGANISM).unwrap();
    edited
        .editor_move_object(actor, Vec3f::new(-0.02, 0.0, 0.0))
        .unwrap();
    assert!(edited.apply_command(&step()).unwrap().execution.succeeded);
}

#[test]
fn compatibility_visible_approach_can_leave_old_overlap() {
    let mut world = world(0.75, None, Vec3f::new(-0.85, 0.0, 0.0));
    let target = world
        .spawn_toy("goal", Vec3f::new(2.0, 0.0, 0.0), true)
        .unwrap();
    let approach = HeadlessWorldCommand::structured(
        ORGANISM,
        HeadlessActionIds::APPROACH,
        ActionKind::Move,
        Some(target),
        None,
    )
    .unwrap();
    assert!(world
        .visible_motion_target(ORGANISM, target)
        .unwrap()
        .is_some());
    assert!(world.apply_command(&approach).unwrap().execution.succeeded);
}

#[test]
fn compatibility_agent_drop_cannot_create_expanded_overlap_but_items_keep_legacy_rule() {
    for terrain in [None, Some(0.2)] {
        let mut actor_world = world(0.75, terrain, Vec3f::new(2.0, 0.0, 0.0));
        let actor = actor_world.agent_entity_id(ORGANISM).unwrap();
        actor_world.begin_player_hold(actor).unwrap();
        let before = actor_world.canonical_signature_digest().unwrap();
        assert!(actor_world
            .move_player_hold(Vec3f::new(1.15, 0.0, 0.0))
            .is_err());
        assert_eq!(before, actor_world.canonical_signature_digest().unwrap());
        let mut item_world = world(0.75, terrain, Vec3f::new(2.0, 0.0, 0.0));
        let item = item_world.spawn_toy("item", Vec3f::ZERO, true).unwrap();
        item_world.begin_player_hold(item).unwrap();
        item_world
            .move_player_hold(Vec3f::new(1.15, 0.0, 0.0))
            .unwrap();
        item_world.release_player_hold().unwrap();
    }
}

#[test]
fn compatibility_escape_is_monotonic_with_explicit_boundary_rules() {
    let r = 0.75_f32;
    for x in [0.0, r.next_down(), r, r.next_up()] {
        let start = Vec3f::new(x, 0.0, 0.0);
        let fixture = world(0.25, Some(0.5), Vec3f::ZERO);
        assert_eq!(fixture.first_motion_blocker(start, start).is_some(), x <= r);
        assert!(fixture
            .first_motion_blocker(start, Vec3f::new(x + 0.1, 0.0, 0.0))
            .is_none());
        assert_eq!(
            fixture
                .first_motion_blocker(start, Vec3f::new(x, 0.1, 0.0))
                .is_some(),
            x == r
        );
        if x > 0.0 {
            assert!(fixture.first_motion_blocker(start, Vec3f::ZERO).is_some());
            // A farther endpoint on the other side is not an escape.
            assert!(fixture
                .first_motion_blocker(start, Vec3f::new(-2.0, 0.0, 0.0))
                .is_some());
        }
    }
    let elevated = world(0.75, None, Vec3f::new(0.0, 2.0, 0.0));
    assert!(elevated
        .first_motion_blocker(Vec3f::new(0.0, 2.85, 0.0), Vec3f::new(0.1, 2.9, 0.0))
        .is_none());
    assert!(elevated
        .first_motion_blocker(Vec3f::new(0.0, 2.85, 0.0), Vec3f::new(2.0, 2.8, 0.0))
        .is_some());
}

#[test]
fn compatibility_escape_still_checks_every_other_obstacle() {
    let mut world = world(0.75, None, Vec3f::new(-0.85, 0.0, 0.0));
    let other = world
        .editor_spawn_object(WorldEditorSpawnSpec {
            label: "other".into(),
            kind: WorldObjectKind::Obstacle,
            organism_id: None,
            position: Vec3f::new(0.3, 0.0, 0.0),
            nutrition: 0.0,
            hazard_pain: 0.0,
            radius: 0.1,
            token_id: None,
        })
        .unwrap();
    assert_eq!(
        world.first_motion_blocker(Vec3f::ZERO, Vec3f::new(1.0, 0.0, 0.0)),
        Some(other)
    );
    world
        .editor_move_object(other, Vec3f::new(0.2, 0.0, 0.0))
        .unwrap();
    assert_eq!(
        world.first_motion_blocker(Vec3f::ZERO, Vec3f::new(0.1, 0.0, 0.0)),
        Some(other)
    );
    // Orthogonal displacement strictly improves both existing overlaps.
    assert!(world
        .first_motion_blocker(Vec3f::ZERO, Vec3f::new(0.0, 0.1, 0.0))
        .is_none());
}

#[test]
fn compatibility_selected_visible_step_never_deepens_any_overlap() {
    for goal in [
        Vec3f::new(2.0, 0.0, 0.0),
        Vec3f::new(-2.0, 0.0, 0.0),
        Vec3f::new(0.0, 0.0, 2.0),
        Vec3f::new(0.0, 0.0, -2.0),
    ] {
        let mut world = world(0.75, None, Vec3f::new(-0.85, 0.0, 0.0));
        let solid = world.entity_id("solid").unwrap();
        world.editor_set_optical_opacity(solid, 0.0).unwrap();
        let other = world
            .editor_spawn_object(WorldEditorSpawnSpec {
                label: "second-overlap".into(),
                kind: WorldObjectKind::Obstacle,
                organism_id: None,
                position: Vec3f::new(-0.6, 0.0, 0.5),
                nutrition: 0.0,
                hazard_pain: 0.0,
                radius: 0.75,
                token_id: None,
            })
            .unwrap();
        world.editor_set_optical_opacity(other, 0.0).unwrap();
        let target = world.spawn_toy("goal", goal, true).unwrap();
        let actor = world.agent_entity_id(ORGANISM).unwrap();
        world.objects.get_mut(&actor.raw()).unwrap().body_yaw = goal.z.atan2(goal.x);
        let command = HeadlessWorldCommand::structured(
            ORGANISM,
            HeadlessActionIds::APPROACH,
            ActionKind::Move,
            Some(target),
            None,
        )
        .unwrap();
        assert!(world
            .visible_motion_target(ORGANISM, target)
            .unwrap()
            .is_some());
        let start = world.entity(actor).unwrap().position;
        let result = world.apply_command(&command).unwrap();
        let end = world.entity(actor).unwrap().position;
        if result.execution.succeeded {
            let delta = [
                f64::from(end.x) - f64::from(start.x),
                f64::from(end.y) - f64::from(start.y),
                f64::from(end.z) - f64::from(start.z),
            ];
            for obstacle in world
                .objects
                .values()
                .filter(|object| object.kind == WorldObjectKind::Obstacle)
            {
                assert_eq!(
                    sphere_start_escape(
                        start,
                        delta,
                        obstacle.position,
                        obstacle.radius + world.locomotion_body_radius()
                    ),
                    Some(true)
                );
                for t in [0.0_f32, 0.25, 0.5, 0.75, 1.0] {
                    let point = add(start, scale(subtract(end, start), t));
                    assert!(
                        sphere_distance_squared(point, obstacle.position)
                            >= sphere_distance_squared(start, obstacle.position)
                    );
                }
            }
        } else {
            assert_eq!(start, end);
        }
    }
}

#[test]
fn compatibility_player_held_old_pose_allows_only_nonworsening_recovery() {
    for terrain in [None, Some(0.2)] {
        let mut old = world(0.75, terrain, Vec3f::new(-0.85, 0.0, 0.0));
        let actor = old.agent_entity_id(ORGANISM).unwrap();
        old.begin_player_hold(actor).unwrap();
        let mut restored = portable(&old);
        restored.move_player_hold(Vec3f::ZERO).unwrap();
        let before = restored.canonical_signature_digest().unwrap();
        assert!(restored
            .move_player_hold(Vec3f::new(-0.01, 0.0, 0.0))
            .is_err());
        assert_eq!(before, restored.canonical_signature_digest().unwrap());
        // Air transport may cross the obstacle at equal separation; no new overlap.
        restored
            .move_player_hold(Vec3f::new(-1.7, 0.0, 0.0))
            .unwrap();
        restored
            .move_player_hold(Vec3f::new(-2.0, 0.0, 0.0))
            .unwrap();
        restored.release_player_hold().unwrap();
        assert_eq!(
            restored.entity(actor).unwrap().position,
            Vec3f::new(-2.0, 0.0, 0.0)
        );
        let mut restored = portable(&old);
        restored.release_player_hold().unwrap();
        assert!(restored.apply_command(&step()).unwrap().execution.succeeded);
    }
}

#[test]
fn compatibility_geometry_arriving_during_hold_does_not_trap_release() {
    let mut world = world(0.75, None, Vec3f::new(-2.0, 0.0, 0.0));
    let actor = world.agent_entity_id(ORGANISM).unwrap();
    let solid = world.entity_id("solid").unwrap();
    world.begin_player_hold(actor).unwrap();
    world
        .editor_move_object(solid, Vec3f::new(-0.85, 0.0, 0.0))
        .unwrap();
    let mut restored = portable(&world);
    restored.move_player_hold(Vec3f::ZERO).unwrap();
    restored.release_player_hold().unwrap();
    assert!(restored.apply_command(&step()).unwrap().execution.succeeded);
}

#[test]
fn compatibility_agent_placement_respects_limits_and_closed_static_boundary() {
    for body in [None, Some(0.0), Some(0.2), Some(0.8)] {
        let radius: f32 = 0.75 + body.unwrap_or(0.2);
        for destination in [radius.next_down(), radius, radius.next_up()] {
            let mut world = world(0.75, body, Vec3f::ZERO);
            let actor = world.agent_entity_id(ORGANISM).unwrap();
            world
                .editor_move_object(actor, Vec3f::new(-3.0, 0.0, 0.0))
                .unwrap();
            world.begin_player_hold(actor).unwrap();
            assert_eq!(
                world
                    .move_player_hold(Vec3f::new(destination, 0.0, 0.0))
                    .is_ok(),
                destination > radius
            );
        }
    }
    let mut world = world(0.75, Some(0.8), Vec3f::new(-5.0, 0.0, 0.0));
    let actor = world.agent_entity_id(ORGANISM).unwrap();
    let mut terrain = world.terrain.as_ref().unwrap().state();
    terrain.data.obstacles = vec![[2.0, -1.0, 2.1, 1.0, 0.0, 2.0]];
    world.terrain = Some(crate::WorldTerrain::new(terrain.data, terrain.locomotion).unwrap());
    world.begin_player_hold(actor).unwrap();
    assert!(world.move_player_hold(Vec3f::new(1.5, 0.0, 0.0)).is_err());
    world.move_player_hold(Vec3f::new(1.0, 0.0, 0.0)).unwrap();
}

#[test]
fn compatibility_player_rescue_cannot_create_overlap_with_another_obstacle() {
    let mut world = world(0.75, None, Vec3f::new(-0.85, 0.0, 0.0));
    let actor = world.agent_entity_id(ORGANISM).unwrap();
    world
        .editor_spawn_object(WorldEditorSpawnSpec {
            label: "other".into(),
            kind: WorldObjectKind::Obstacle,
            organism_id: None,
            position: Vec3f::new(0.4, 0.0, 0.0),
            nutrition: 0.0,
            hazard_pain: 0.0,
            radius: 0.1,
            token_id: None,
        })
        .unwrap();
    world.begin_player_hold(actor).unwrap();
    let before = world.canonical_signature_digest().unwrap();
    assert!(world.move_player_hold(Vec3f::new(0.2, 0.0, 0.0)).is_err());
    assert_eq!(before, world.canonical_signature_digest().unwrap());
}

#[test]
fn precision_visible_approach_escapes_symmetric_overlap_on_exact_perpendicular() {
    for sign in [-1.0, 1.0] {
        let mut world = world(0.75, Some(0.2), Vec3f::new(-0.85, 0.0, 0.0));
        world
            .editor_spawn_object(WorldEditorSpawnSpec {
                label: "opposite".into(),
                kind: WorldObjectKind::Obstacle,
                organism_id: None,
                position: Vec3f::new(0.85, 0.0, 0.0),
                nutrition: 0.0,
                hazard_pain: 0.0,
                radius: 0.75,
                token_id: None,
            })
            .unwrap();
        let actor = world.agent_entity_id(ORGANISM).unwrap();
        world.objects.get_mut(&actor.raw()).unwrap().body_yaw = sign * std::f32::consts::FRAC_PI_2;
        let target = world
            .spawn_toy("goal", Vec3f::new(0.0, 0.0, sign * 2.0), true)
            .unwrap();
        let command = HeadlessWorldCommand::structured(
            ORGANISM,
            HeadlessActionIds::APPROACH,
            ActionKind::Move,
            Some(target),
            None,
        )
        .unwrap();
        for _ in 0..5 {
            assert!(world
                .visible_motion_target(ORGANISM, target)
                .unwrap()
                .is_some());
            let before = world.entity(actor).unwrap().position;
            let result = world.apply_command(&command).unwrap();
            assert!(result.execution.succeeded);
            let after = world.entity(actor).unwrap().position;
            assert_eq!(after.x, 0.0);
            assert!(sign * after.z > sign * before.z);
            for obstacle in world
                .objects
                .values()
                .filter(|object| object.kind == WorldObjectKind::Obstacle)
            {
                assert!(
                    sphere_distance_squared(after, obstacle.position)
                        > sphere_distance_squared(before, obstacle.position)
                );
            }
        }
    }
}

#[test]
fn precision_visible_approach_can_escape_center_and_boundary_outward() {
    for center in [Vec3f::ZERO, Vec3f::new(0.0, 0.0, -0.95)] {
        let mut world = world(0.75, Some(0.2), center);
        let solid = world.entity_id("solid").unwrap();
        world.editor_set_optical_opacity(solid, 0.0).unwrap();
        let actor = world.agent_entity_id(ORGANISM).unwrap();
        world.objects.get_mut(&actor.raw()).unwrap().body_yaw = std::f32::consts::FRAC_PI_2;
        let target = world
            .spawn_toy("goal", Vec3f::new(0.0, 0.0, 2.0), true)
            .unwrap();
        assert!(world
            .visible_motion_target(ORGANISM, target)
            .unwrap()
            .is_some());
        let command = HeadlessWorldCommand::structured(
            ORGANISM,
            HeadlessActionIds::APPROACH,
            ActionKind::Move,
            Some(target),
            None,
        )
        .unwrap();
        assert!(world.apply_command(&command).unwrap().execution.succeeded);
        assert_eq!(world.entity(actor).unwrap().position.x, 0.0);
        assert!(world.entity(actor).unwrap().position.z > 0.0);
    }
}

fn trough_with_raised_goal(incline: f32) -> (HeadlessWorld, WorldEntityId) {
    let mut world = HeadlessScenarioBuilder::new(803)
        .agent("actor", ORGANISM, Vec3f::ZERO)
        .build()
        .unwrap();
    world
        .enable_terrain_for_new_game(
            crate::WorldTerrain::new(
                crate::TerrainData {
                    width: 7,
                    depth: 3,
                    origin_x: -2.4,
                    origin_z: -0.8,
                    spacing: 0.8,
                    heights: (0..3)
                        .flat_map(|row| {
                            (0..7).map(move |column| {
                                incline * (-2.4 + column as f32 * 0.8)
                                    + if row == 1 { 0.0 } else { 0.6 }
                            })
                        })
                        .collect(),
                    obstacles: vec![],
                    water_level: None,
                },
                crate::LocomotionLimits::default(),
            )
            .unwrap(),
            Vec3f::ZERO,
        )
        .unwrap();
    for (label, z) in [("north", 0.8), ("south", -0.8)] {
        let id = world
            .editor_spawn_object(WorldEditorSpawnSpec {
                label: label.into(),
                kind: WorldObjectKind::Obstacle,
                organism_id: None,
                position: Vec3f::new(0.0, 0.6, z),
                nutrition: 0.0,
                hazard_pain: 0.0,
                radius: 0.9,
                token_id: None,
            })
            .unwrap();
        world.editor_set_optical_opacity(id, 0.0).unwrap();
    }
    let target = world
        .spawn_toy("raised-goal", Vec3f::new(2.0, 0.0, 0.0), true)
        .unwrap();
    world.begin_player_hold(target).unwrap();
    assert_eq!(
        world.entity(target).unwrap().position,
        Vec3f::new(
            2.0,
            world
                .terrain
                .as_ref()
                .unwrap()
                .surface()
                .height(2.0, 0.0)
                .unwrap()
                + 1.3,
            0.0
        )
    );
    (world, target)
}

#[test]
fn grounded_candidate_approach_escapes_trough_toward_player_held_goal() {
    let (mut world, target) = trough_with_raised_goal(0.0);
    let actor = world.agent_entity_id(ORGANISM).unwrap();
    let relative = world
        .visible_motion_target(ORGANISM, target)
        .unwrap()
        .expect("raised goal must be visually admitted");
    assert!(relative.y > 0.0);
    let mut absolute = world.clone();
    let absolute_command = HeadlessWorldCommand::structured(
        ORGANISM,
        ActionKind::Move.canonical_id(),
        ActionKind::Move,
        None,
        Some(Vec3f::new(2.0, 0.0, 0.0)),
    )
    .unwrap();
    assert!(
        absolute
            .apply_command(&absolute_command)
            .unwrap()
            .execution
            .succeeded
    );
    assert_eq!(absolute.entity(actor).unwrap().position.y, 0.0);
    let approach = HeadlessWorldCommand::approach(ORGANISM, target).unwrap();
    for _ in 0..5 {
        assert!(world
            .visible_motion_target(ORGANISM, target)
            .unwrap()
            .is_some());
        let before = world.entity(actor).unwrap().position;
        let result = world.apply_command(&approach).unwrap();
        assert!(
            result.execution.succeeded,
            "grounded perpendicular escape must not inherit raised-target dy"
        );
        let after = world.entity(actor).unwrap().position;
        assert!(after.x > before.x);
        assert_eq!(after.y, 0.0);
        assert_eq!(after.z, 0.0);
        for obstacle in world
            .objects
            .values()
            .filter(|object| object.kind == WorldObjectKind::Obstacle)
        {
            assert!(
                sphere_distance_squared(after, obstacle.position)
                    > sphere_distance_squared(before, obstacle.position)
            );
        }
    }
}

#[test]
fn grounded_candidate_slope_projection_preserves_monotonic_escape() {
    for incline in [-0.25, 0.25] {
        let (mut world, target) = trough_with_raised_goal(incline);
        let actor = world.agent_entity_id(ORGANISM).unwrap();
        let start = world.entity(actor).unwrap().position;
        assert!(
            world
                .visible_motion_target(ORGANISM, target)
                .unwrap()
                .unwrap()
                .y
                > start.y
        );
        let result = world
            .apply_command(&HeadlessWorldCommand::approach(ORGANISM, target).unwrap())
            .unwrap();
        let end = world.entity(actor).unwrap().position;
        if incline < 0.0 {
            assert!(
                result.execution.succeeded,
                "downhill escape must use decreasing grounded Y despite raised target"
            );
            assert!(end.x > start.x && end.y < start.y);
        }
        if result.execution.succeeded {
            assert_eq!(
                end.y,
                world
                    .terrain
                    .as_ref()
                    .unwrap()
                    .surface()
                    .height(end.x, end.z)
                    .unwrap()
            );
            let delta = [
                f64::from(end.x) - f64::from(start.x),
                f64::from(end.y) - f64::from(start.y),
                f64::from(end.z) - f64::from(start.z),
            ];
            for obstacle in world
                .objects
                .values()
                .filter(|object| object.kind == WorldObjectKind::Obstacle)
            {
                assert_eq!(
                    sphere_start_escape(
                        start,
                        delta,
                        obstacle.position,
                        obstacle.radius + world.locomotion_body_radius()
                    ),
                    Some(true)
                );
            }
        } else {
            assert_eq!(start, end);
        }
    }
}

#[test]
fn grounded_candidate_projection_does_not_authorize_unsupported_steps() {
    for barrier in ["water", "slope", "outside"] {
        let mut world = HeadlessScenarioBuilder::new(804)
            .agent("actor", ORGANISM, Vec3f::ZERO)
            .toy("goal", Vec3f::new(2.0, 5.0, 0.0), true)
            .build()
            .unwrap();
        let target = world.entity_id("goal").unwrap();
        let actor = world.agent_entity_id(ORGANISM).unwrap();
        // Bind synthetic support directly, retaining a visible elevated goal.
        // Height availability alone must never substitute for full move validation.
        let heights = if barrier == "slope" {
            vec![0.0, 2.0, 0.0, 2.0]
        } else {
            vec![0.0; 4]
        };
        world.terrain = Some(
            crate::WorldTerrain::new(
                crate::TerrainData {
                    width: 2,
                    depth: 2,
                    origin_x: if barrier == "outside" { -1.0 } else { 0.0 },
                    origin_z: -0.5,
                    spacing: 1.0,
                    heights,
                    obstacles: vec![],
                    water_level: (barrier == "water").then_some(1.0),
                },
                crate::LocomotionLimits::default(),
            )
            .unwrap(),
        );
        assert!(world
            .visible_motion_target(ORGANISM, target)
            .unwrap()
            .is_some());
        let before = world.entity(actor).unwrap().position;
        let relative = world
            .visible_motion_target(ORGANISM, target)
            .unwrap()
            .unwrap();
        let original = step_toward(before, add(before, relative), MOVE_STEP);
        // Selection still returns one candidate. The authoritative terrain check
        // rejects unsupported/out-of-bounds, too-steep and too-deep endpoints.
        let selected = world
            .steer_visible_step(ORGANISM, target, relative, original)
            .unwrap()
            .unwrap()
            .0;
        assert!(
            world
                .terrain
                .as_ref()
                .unwrap()
                .resolve_move(before, selected)
                .is_none(),
            "{barrier}"
        );
        let result = world
            .apply_command(&HeadlessWorldCommand::approach(ORGANISM, target).unwrap())
            .unwrap();
        assert!(!result.execution.succeeded, "{barrier}");
        assert_eq!(world.entity(actor).unwrap().position, before);
    }
}
