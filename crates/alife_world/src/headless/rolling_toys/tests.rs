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
