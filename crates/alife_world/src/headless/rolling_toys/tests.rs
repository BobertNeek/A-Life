use super::*;

fn fixture() -> (HeadlessWorld, WorldEntityId, WorldEntityId) {
    let mut w = HeadlessScenarioBuilder::new(1643).build().unwrap();
    let ball = w.spawn_toy("rattle", Vec3f::ZERO, true).unwrap();
    let table = w.spawn_toy("table", Vec3f::new(3., 0., 0.), false).unwrap();
    (w, ball, table)
}
fn ticks(w: &mut HeadlessWorld, n: usize) {
    for _ in 0..n {
        w.try_advance_tick().unwrap();
    }
}

#[test]
fn table_stops_the_real_ball_and_does_not_receive_a_creature_action() {
    let (mut w, ball, table) = fixture();
    let station = w.entity(table).unwrap().clone();
    assert!(w.roll_player_toy(table, [1., 0.]).is_err());
    w.roll_player_toy(ball, [200., 0.]).unwrap();
    assert_eq!(w.entity(ball).unwrap().position, Vec3f::ZERO);
    assert!(w.last_chime_strike().is_none());
    ticks(&mut w, 50);
    let hit = w.last_chime_strike().unwrap();
    assert_eq!(hit.1, table);
    let parked = w.entity(ball).unwrap().position;
    assert!((parked.x - 2.2).abs() < 0.0001);
    assert_eq!(
        w.entity(ball).unwrap().grounded_physical.velocity,
        Vec3f::ZERO
    );
    assert_eq!(w.entity(table), Some(&station));
    assert_eq!(w.object_count(), 2);
    w.roll_player_toy(ball, [-1., 0.]).unwrap();
    ticks(&mut w, 70);
    assert!((w.entity(ball).unwrap().position.x - (parked.x - 1.)).abs() < 0.0001);
    assert_eq!(w.last_chime_strike(), Some(hit));
}

#[test]
fn mid_roll_save_resumes_exactly_and_invalid_motion_is_rejected() {
    let (mut w, ball, table) = fixture();
    let untouched = w.canonical_signature_digest().unwrap();
    w.roll_player_toy(ball, [0., 99.]).unwrap();
    assert_ne!(w.canonical_signature_digest().unwrap(), untouched);
    ticks(&mut w, 3);
    let json = serde_json::to_string(&w.save_state()).unwrap();
    let mut saved: crate::WorldSaveState = serde_json::from_str(&json).unwrap();
    let portable = crate::PortableSaveFile::from_headless_world(
        "rattle",
        &w,
        crate::RuntimeConfig::deterministic_default(w.seed(), alife_core::BrainScaleTier::Nano512),
        crate::AssetManifest::empty(),
        vec![],
    )
    .unwrap();
    let mut resumed =
        crate::PortableSaveFile::from_json_str(&portable.to_json_string_pretty().unwrap())
            .unwrap()
            .restore_headless_world()
            .unwrap();
    assert_eq!(
        w.canonical_signature_digest().unwrap(),
        resumed.canonical_signature_digest().unwrap()
    );
    for _ in 0..85 {
        ticks(&mut w, 1);
        ticks(&mut resumed, 1);
        assert_eq!(w.entity(ball), resumed.entity(ball));
    }
    assert!((w.entity(ball).unwrap().position.z - 6.).abs() < 0.0001);
    assert!(w.toy_rolls.is_empty());
    assert!(serde_json::to_value(w.save_state())
        .unwrap()
        .get("toy_rolls")
        .is_none());
    for bad in [[0., 0.], [f32::NAN, 1.], [7., 0.]] {
        saved.toy_rolls.insert(ball.raw(), bad);
        assert!(HeadlessWorld::from_save_state(&saved).is_err());
    }
    saved.toy_rolls.clear();
    saved.toy_rolls.insert(table.raw(), [1., 0.]);
    assert!(HeadlessWorld::from_save_state(&saved).is_err());
}

#[test]
fn catching_repositioning_and_removing_a_ball_cancel_its_roll() {
    let (mut w, ball, _) = fixture();
    w.roll_player_toy(ball, [3., 0.]).unwrap();
    w.begin_player_hold(ball).unwrap();
    assert!(w.toy_rolls.is_empty() && w.roll_player_toy(ball, [1., 0.]).is_err());
    w.release_player_hold().unwrap();
    let dropped = w.entity(ball).unwrap().position;
    ticks(&mut w, 1);
    assert_eq!(w.entity(ball).unwrap().position, dropped);
    w.roll_player_toy(ball, [3., 0.]).unwrap();
    w.editor_move_object(ball, Vec3f::ZERO).unwrap();
    assert!(w.toy_rolls.is_empty());
    w.roll_player_toy(ball, [3., 0.]).unwrap();
    w.editor_remove_object(ball).unwrap();
    assert!(w.toy_rolls.is_empty());
}

#[test]
fn a_solid_between_rattle_and_instrument_cannot_ring_the_table() {
    let (mut w, ball, _) = fixture();
    w.editor_spawn_object(WorldEditorSpawnSpec {
        label: "rock".into(),
        kind: WorldObjectKind::Obstacle,
        organism_id: None,
        position: Vec3f::new(1.4, 0., 0.),
        nutrition: 0.,
        radius: 0.15,
        hazard_pain: 0.,
        token_id: None,
    })
    .unwrap();
    w.roll_player_toy(ball, [6., 0.]).unwrap();
    ticks(&mut w, 55);
    assert!((w.entity(ball).unwrap().position.x - 0.95).abs() < 0.0001);
    assert_eq!(w.last_chime_strike(), None);
}

#[test]
fn bound_ball_follows_the_ground_height_and_stops_before_water() {
    let (mut w, ball, table) = fixture();
    w.editor_remove_object(table).unwrap();
    w.terrain = Some(
        crate::WorldTerrain::new(
            crate::TerrainData {
                width: 4,
                depth: 2,
                origin_x: 0.,
                origin_z: -1.,
                spacing: 2.,
                heights: vec![1., 1.5, 0.5, -1., 1., 1.5, 0.5, -1.],
                obstacles: vec![],
                water_level: Some(1.),
            },
            crate::LocomotionLimits {
                max_wading_depth: 0.,
                ..Default::default()
            },
        )
        .unwrap(),
    );
    w.editor_move_object(ball, Vec3f::new(0.5, 0., 0.)).unwrap();
    w.roll_player_toy(ball, [6., 0.]).unwrap();
    for _ in 0..65 {
        ticks(&mut w, 1);
        let p = w.entity(ball).unwrap().position;
        let h = w.terrain().unwrap().surface().height(p.x, p.z).unwrap();
        assert!((p.y - h).abs() < 0.0001 && h >= 1.);
    }
    let end = w.entity(ball).unwrap().position;
    assert!((2.5..=3.).contains(&end.x));
    assert!(w.toy_rolls.is_empty());
}

#[test]
fn resident_acquisition_catches_a_still_rolling_ball_without_releasing_or_duplicating_it() {
    let organism = OrganismId(1);
    let mut w = HeadlessScenarioBuilder::new(1643)
        .agent("resident", organism, Vec3f::ZERO)
        .build()
        .unwrap();
    let ball = w
        .spawn_toy("rattle", Vec3f::new(0.7, 0., 0.), true)
        .unwrap();
    w.roll_player_toy(ball, [4., 0.]).unwrap();
    let grab = ActionCommand::structured(
        organism,
        HeadlessActionIds::GRAB,
        ActionKind::Interact,
        alife_core::ActionTarget::new(Some(ball), None),
        Intensity::new(1.).unwrap(),
        alife_core::DurationTicks::new(1),
        Confidence::new(1.).unwrap(),
        0,
        None,
        None,
        None,
    )
    .unwrap();
    let result = w.apply_command(&grab).unwrap();
    assert!(result.execution.succeeded && w.toy_rolls.is_empty());
    assert_eq!(w.entity(ball).unwrap().carried_by, Some(organism));
    assert!(w.roll_player_toy(ball, [4., 0.]).is_err());
    assert!(w.last_chime_strike().is_none());
    assert_eq!(w.object_count(), 2);
}

mod original_chime_relocation {
    use super::*;
    use crate::orchard::{OrchardGesture, OrchardReceipt};
    use crate::{AssetManifest, PortableSaveFile, RuntimeConfig, VoxelTileCoord};

    const BALL: &str = "player-plaything";
    const TABLE: &str = "player-activity-toy";

    // This is the test caller's independently specified expectation, not
    // a query for the code path that happens to be active.
    fn want_review() -> bool {
        match option_env!("QUALIA_EXPECT_CHIME_PLACEMENT") {
            None | Some("stock") => false,
            Some("review") => true,
            Some(_) => panic!("expectation must be stock or review"),
        }
    }

    fn ground(world: &HeadlessWorld, x: f32, z: f32) -> Vec3f {
        let y = world.terrain().unwrap().surface().height(x, z).unwrap();
        Vec3f::new(x, y, z)
    }

    fn fixture() -> (HeadlessWorld, WorldEntityId, WorldEntityId) {
        let mut world = HeadlessWorld::new(1643);
        world.enable_island_for_new_game().unwrap();
        // Fixture only: use the unchanged App gift geometry at an illustrative
        // (80,250), yaw=PI. No resident, save import, authored action or forced
        // actor position. Same signed land, labels, recipe, radii and IDs.
        let yaw = std::f32::consts::PI;
        let launch = ground(&world, 80.0 + 1.5 * yaw.cos(), 250.0 + 1.5 * yaw.sin());
        let seat = ground(&world, launch.x + 1.5, launch.z + 1.5);
        assert_eq!((launch.x, launch.z), (78.5, 250.0));
        assert_eq!((seat.x, seat.z), (80.0, 251.5));
        let ball = world.spawn_toy(BALL, launch, true).unwrap();
        let table = world.spawn_toy(TABLE, seat, false).unwrap();
        assert_ne!(ball, table);
        assert_eq!(world.entity(ball).unwrap().radius, 0.3);
        assert_eq!(world.entity(table).unwrap().radius, 0.5);
        (world, ball, table)
    }

    fn roundtrip(world: &HeadlessWorld) -> HeadlessWorld {
        let save = PortableSaveFile::from_headless_world(
            "signed-original-chime-placement",
            world,
            RuntimeConfig::deterministic_default(world.seed(), alife_core::BrainScaleTier::Nano512),
            AssetManifest::empty(),
            vec![],
        )
        .unwrap();
        let json = save.to_json_string_pretty().unwrap();
        let fresh = PortableSaveFile::from_json_str(&json)
            .unwrap()
            .restore_headless_world()
            .expect("a fresh authority must re-open this exact hold/place state");
        assert_eq!(
            world.terrain().unwrap().binding(),
            fresh.terrain().unwrap().binding()
        );
        fresh
    }

    fn xz_to(a: Vec3f, b: Vec3f) -> [f32; 2] {
        [b.x - a.x, b.z - a.z]
    }

    fn snap_at(world: &HeadlessWorld, table: WorldEntityId, shape: &WorldObject, at: Vec3f) {
        let mut expected = shape.clone();
        expected.position = at;
        assert_eq!(world.entity_id(TABLE), Some(table));
        assert_eq!(world.entity(table), Some(&expected));
        assert_eq!(world.object_count(), 2);
    }

    fn settle(world: &mut HeadlessWorld, table: WorldEntityId, at: Vec3f) {
        world.begin_player_hold(table).unwrap();
        assert_eq!(
            world.move_player_hold(at).unwrap(),
            Vec3f::new(at.x, at.y + 1.3, at.z)
        );
        assert_eq!(world.release_player_hold().unwrap(), Some(table));
        assert_eq!(world.player_held_object(), None);
    }

    fn real_roll(
        world: &mut HeadlessWorld,
        ball: WorldEntityId,
        offset: [f32; 2],
    ) -> Option<(Tick, WorldEntityId)> {
        let before = world.entity(ball).unwrap().position;
        assert_eq!(world.last_chime_strike(), None);
        world.roll_player_toy(ball, offset).unwrap();
        assert_eq!(world.last_chime_strike(), None, "aim is not contact");
        assert_eq!(world.entity(ball).unwrap().position, before);
        let mut struck = None;
        for _ in 0..65 {
            let tick = world.try_advance_tick().unwrap();
            if let Some(receipt) = world.last_chime_strike() {
                match struck {
                    None => {
                        assert_eq!(receipt.0, tick);
                        struck = Some(receipt);
                    }
                    Some(first) => assert_eq!(receipt, first, "a stopped ball cannot re-strike"),
                }
            }
            if !world.toy_rolls.contains_key(&ball.raw()) {
                break;
            }
        }
        let after = world.entity(ball).unwrap();
        assert_ne!(after.position, before);
        assert_eq!(after.grounded_physical.velocity, Vec3f::ZERO);
        assert!(!world.toy_rolls.contains_key(&ball.raw()));
        let end = after.position;
        world.try_advance_tick().unwrap();
        assert_eq!(world.entity(ball).unwrap().position, end);
        assert_eq!(world.last_chime_strike(), struck);
        struck
    }

    fn contact_gap(ball: &WorldObject, table: &WorldObject) -> f32 {
        let p = ball.position;
        let q = table.position;
        (p.x - q.x).hypot(p.z - q.z).hypot(p.y - q.y) - (ball.radius + table.radius)
    }

    #[test]
    fn signed_original_chime_keeps_ids_across_refusal_inverse_reopen_and_actual_strike() {
        let (mut world, ball, table) = fixture();
        let original = world.entity(table).unwrap().clone();
        let rattle = world.entity(ball).unwrap().clone();
        let start = original.position;
        let next = ground(&world, 81.5, 251.5);
        let wet = ground(&world, 12.5, 435.0);
        let through_rattle = ground(&world, 77.0, 248.5);
        let old_aim = xz_to(rattle.position, start);
        let new_aim = xz_to(rattle.position, next);
        assert_eq!(old_aim, [1.5, 1.5]);
        assert_eq!(new_aim, [3.0, 1.5]);
        assert!(wet.y < world.terrain().unwrap().surface().water_level.unwrap());
        assert!(world.terrain().unwrap().walkable(next.x, next.z));
        let terrain = world.terrain().unwrap();
        let mut limits = terrain.limits();
        limits.body_radius = original.radius;
        limits.max_wading_depth = 0.0;
        assert_eq!(
            terrain.surface().resolve_move_with(start, through_rattle, limits),
            Some(through_rattle),
            "baked terrain alone allows this destination and line"
        );
        assert_eq!(world.can_begin_player_hold(table), want_review());

        // Same old ball, site and real rolling contact in both arms. An active
        // roll may not be interrupted by picking up its instrument.
        let mut old_site = world.clone();
        old_site.roll_player_toy(ball, old_aim).unwrap();
        assert!(!old_site.can_begin_player_hold(table));
        let queued = old_site.canonical_signature_digest().unwrap();
        assert!(old_site.begin_player_hold(table).is_err());
        assert_eq!(old_site.canonical_signature_digest().unwrap(), queued);
        assert_eq!(old_site.last_chime_strike(), None);
        // Use a separate fresh copy to share the bounded stock roll observer.
        let old_hit = real_roll(&mut roundtrip(&world), ball, old_aim);
        assert_eq!(old_hit.map(|(_, id)| id), Some(table));

        if !want_review() {
            let untouched = world.canonical_signature_digest().unwrap();
            assert!(world.begin_player_hold(table).is_err());
            assert_eq!(world.canonical_signature_digest().unwrap(), untouched);
            snap_at(&world, table, &original, start);
            return;
        }

        let raised = world.begin_player_hold(table).unwrap();
        assert_eq!(raised, Vec3f::new(start.x, start.y + 1.3, start.z));
        assert_eq!(world.player_held_object(), Some(table));
        assert!(!world.can_roll_toy(ball));
        let approved = world.canonical_signature_digest().unwrap();
        for refused in [rattle.position, through_rattle, wet] {
            assert!(world.move_player_hold(refused).is_err());
            assert_eq!(world.canonical_signature_digest().unwrap(), approved);
            assert_eq!(world.player_held_object(), Some(table));
            assert_eq!(world.entity(table).unwrap().position, raised);
            assert_eq!(world.entity(ball), Some(&rattle));
            assert_eq!(world.last_chime_strike(), None);
        }
        assert_eq!(world.release_player_hold().unwrap(), Some(table));
        snap_at(&world, table, &original, start);

        settle(&mut world, table, next);
        snap_at(&world, table, &original, next);
        settle(&mut world, table, start); // An actual same-identity inverse.
        snap_at(&world, table, &original, start);
        settle(&mut world, table, next);
        let mut fresh = roundtrip(&world);
        assert_eq!(
            world.canonical_signature_digest().unwrap(),
            fresh.canonical_signature_digest().unwrap()
        );
        assert_eq!(fresh.player_held_object(), None);
        assert_eq!(fresh.entity_id(BALL), Some(ball));
        assert_eq!(fresh.entity(ball), Some(&rattle));
        snap_at(&fresh, table, &original, next);

        // The old target really misses after the move; same rattle, actual
        // ticks, no fixture rebuild and no fabricated strike event.
        let mut old_aim_after_move = fresh.clone();
        assert_eq!(real_roll(&mut old_aim_after_move, ball, old_aim), None);
        let miss = old_aim_after_move.entity(ball).unwrap().position;
        assert!((miss.x - start.x).hypot(miss.z - start.z) < 0.004);
        assert!(
            contact_gap(old_aim_after_move.entity(ball).unwrap(), fresh.entity(table).unwrap()) > 0.6
        );

        let (hit_tick, hit_table) = real_roll(&mut fresh, ball, new_aim).unwrap();
        assert!(hit_tick.raw() > 0);
        assert_eq!(hit_table, table);
        snap_at(&fresh, table, &original, next);
        assert!(contact_gap(fresh.entity(ball).unwrap(), fresh.entity(table).unwrap()).abs() < 0.004);
        let settled_after_strike = roundtrip(&fresh);
        snap_at(&settled_after_strike, table, &original, next);
        assert_eq!(settled_after_strike.entity(ball), fresh.entity(ball));
        assert_eq!(settled_after_strike.last_chime_strike(), None);
    }

    #[test]
    fn signed_original_chime_failed_setdown_retains_same_hold_and_real_plant_through_reopen() {
        let (mut world, ball, table) = fixture();
        let original = world.entity(table).unwrap().clone();
        let untouched_rattle = world.entity(ball).unwrap().clone();
        assert_eq!(world.can_begin_player_hold(table), want_review());
        if !want_review() {
            return;
        }
        let next = ground(&world, 81.5, 251.5);
        let slot = VoxelTileCoord::new(81, 251);
        let bed_before = world.orchard_gesture(slot);
        assert_eq!(bed_before, Some(OrchardGesture::Sow));
        world.begin_player_hold(table).unwrap();
        let raised = world.move_player_hold(next).unwrap();

        // A real change of World authority between move and release, not
        // fabricated occupancy. This is a fault-injection order at World level;
        // it does not assert one App Hand can carry and tend simultaneously.
        assert_eq!(world.tend_orchard(slot).unwrap(), OrchardReceipt::Sown);
        let blocked = world.canonical_signature_digest().unwrap();
        assert!(world.release_player_hold().is_err());
        assert_eq!(world.canonical_signature_digest().unwrap(), blocked);
        assert_eq!(world.player_held_object(), Some(table));
        assert_eq!(world.entity(table).unwrap().position, raised);
        assert_eq!(world.entity(ball), Some(&untouched_rattle));

        let mut fresh = roundtrip(&world);
        assert_eq!(fresh.canonical_signature_digest().unwrap(), blocked);
        assert_eq!(fresh.player_held_object(), Some(table));
        assert_eq!(fresh.entity(table).unwrap().position, raised);
        assert_eq!(fresh.orchard_gesture(slot), Some(OrchardGesture::Watch));
        assert!(fresh.release_player_hold().is_err());
        assert_eq!(fresh.canonical_signature_digest().unwrap(), blocked);
        // World can recover to unoccupied signed land; never cuts out the bed.
        fresh.move_player_hold(original.position).unwrap();
        assert_eq!(fresh.release_player_hold().unwrap(), Some(table));
        snap_at(&fresh, table, &original, original.position);
        assert_eq!(fresh.entity(ball), Some(&untouched_rattle));
        assert_eq!(fresh.orchard_gesture(slot), Some(OrchardGesture::Watch));
        assert_eq!(fresh.last_chime_strike(), None);
    }

    #[test]
    fn signed_original_chime_saved_hold_admits_only_its_exact_identity_and_footing() {
        let (mut world, ball, table) = fixture();
        let original = world.entity(table).unwrap().clone();
        let alternate_ground = ground(&world, 84.0, 251.5);
        let unrelated = world.spawn_toy("unrelated-fixed-activity", alternate_ground, false).unwrap();
        assert!(!world.can_begin_player_hold(unrelated));
        assert_eq!(world.can_begin_player_hold(table), want_review());

        // Even a coherent raised original must fail closed in stock. The same
        // exact shape opens with the review flag and only when genuinely held.
        if !want_review() {
            let mut off = world.save_state();
            off.player_hold = Some(crate::headless::player_hand::PlayerHold {
                object_id: table,
                last_ground: original.position,
            });
            off.objects.iter_mut().find(|o| o.id == table).unwrap().position.y += 1.3;
            assert_rejected_chime_wire(&off);
            return;
        }

        let raised = world.begin_player_hold(table).unwrap();
        let restored = roundtrip(&world);
        assert_eq!(restored.player_held_object(), Some(table));
        assert_eq!(restored.entity(table).unwrap().position, raised);
        assert_eq!(restored.entity_id(BALL), Some(ball));
        assert_eq!(restored.entity_id(TABLE), Some(table));
        assert_eq!(restored.entity(unrelated), world.entity(unrelated));
        assert_eq!(restored.canonical_signature_digest().unwrap(), world.canonical_signature_digest().unwrap());

        let authentic = world.save_state();
        let mut wrong_id = authentic.clone();
        let hold = wrong_id.player_hold.as_mut().unwrap();
        hold.object_id = unrelated;
        hold.last_ground = alternate_ground;
        wrong_id.objects.iter_mut().find(|o| o.id == table).unwrap().position = original.position;
        wrong_id.objects.iter_mut().find(|o| o.id == unrelated).unwrap().position.y += 1.3;
        assert_rejected_chime_wire(&wrong_id);

        let mut wrong_radius = authentic.clone();
        wrong_radius.objects.iter_mut().find(|o| o.id == table).unwrap().radius = 0.51;
        assert_rejected_chime_wire(&wrong_radius);

        let mut busy = authentic.clone();
        busy.objects.iter_mut().find(|o| o.id == table).unwrap().activity_hold = true;
        assert_rejected_chime_wire(&busy);

        let mut duplicate_name = authentic.clone();
        duplicate_name.objects.iter_mut().find(|o| o.id == unrelated).unwrap().label = TABLE.into();
        assert_rejected_chime_wire(&duplicate_name);

        // Plausible relative lift off an invented support remains illegal:
        // the later geometry check must still compare with signed terrain.
        let mut wrong_ground = authentic;
        wrong_ground.player_hold.as_mut().unwrap().last_ground.y += 0.35;
        wrong_ground.objects.iter_mut().find(|o| o.id == table).unwrap().position.y += 0.35;
        assert_rejected_chime_wire(&wrong_ground);
    }

    fn assert_rejected_chime_wire(state: &crate::WorldSaveState) {
        let bytes = serde_json::to_vec(state).unwrap();
        let decoded: crate::WorldSaveState = serde_json::from_slice(&bytes).unwrap();
        assert!(matches!(
            HeadlessWorld::from_save_state(&decoded),
            Err(ScaffoldContractError::InvalidId)
        ));
    }
}
