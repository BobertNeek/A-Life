//! Future World-only opt-in receipt. No App/HUD, rendered art or animal automation.
use alife_core::{BrainScaleTier, Vec3f, WorldEntityId};
use alife_world::orchard::{OrchardBed, OrchardReceipt};
use alife_world::{
    AssetManifest, HeadlessWorld, PortableSaveFile, RuntimeConfig, VoxelTileCoord, WorldObjectKind,
};

const ROOT: VoxelTileCoord = VoxelTileCoord::new(79, 250);
const NEW: VoxelTileCoord = VoxelTileCoord::new(82, 252);
const TRAY: VoxelTileCoord = VoxelTileCoord::new(81, 250);

fn ground(world: &HeadlessWorld, x: f32, z: f32) -> Vec3f {
    Vec3f::new(x, world.terrain().unwrap().surface().height(x, z).unwrap(), z)
}

fn advance(world: &mut HeadlessWorld, ticks: u64) {
    for _ in 0..ticks {
        world.try_advance_tick().unwrap();
    }
}

fn place(world: &mut HeadlessWorld, item: WorldEntityId, at: Vec3f) {
    world.begin_player_hold(item).unwrap();
    world.move_player_hold(at).unwrap();
    assert_eq!(world.release_player_hold().unwrap(), Some(item));
    assert_eq!(world.entity(item).unwrap().position, at);
}

#[test]
fn signed_wick_moves_one_watered_root_only_after_its_original_pear_clears() {
    let mut world = HeadlessWorld::new(1643);
    world.enable_island_for_new_game().unwrap();
    assert_eq!(world.tend_orchard(ROOT).unwrap(), OrchardReceipt::Sown);
    let origin = ground(&world, 80.0, 251.5);
    let fit = ground(&world, 83.5, 250.5);
    let other_side = ground(&world, 86.5, 252.5);
    let parking = ground(&world, 77.5, 246.5);
    // This former Wick placement is signed and available before and after a
    // nearby crop; its actual legality is not the reason the source refuses.
    let dry = ground(&world, 82.5, 252.5);
    let actual_tray = ground(&world, 81.5, 250.5);
    let parked_in_another_footprint = VoxelTileCoord::new(77, 248);
    let nursery = world.movable_orchard_planter(ROOT).unwrap();
    assert_eq!(world.survey_orchard_planter(nursery, NEW), Some(dry));
    assert_eq!(world.survey_orchard_planter(nursery, parked_in_another_footprint),
        Some(ground(&world, 77.5, 248.5)));
    let bookend = world.spawn_toy("player-activity-toy", origin, false).unwrap();
    place(&mut world, bookend, fit);
    let first = world.orchard_court().unwrap();
    assert_eq!((first.bed, first.landing, first.landing_ground), (ROOT, TRAY, actual_tray));

    advance(&mut world, 80);
    assert_eq!(world.tend_orchard(ROOT).unwrap(), OrchardReceipt::Watered);
    advance(&mut world, 220);
    let OrchardReceipt::Picked(pear) = world.tend_orchard(ROOT).unwrap() else {
        panic!("expected native crop");
    };
    assert_eq!(world.entity(pear).unwrap().position, actual_tray);
    assert!(!world.orchard_court().unwrap().open);
    assert_eq!(world.movable_orchard_planter(ROOT), None);

    // Still tending is a valid choice while the pear is left at the actual
    // tray. The next already growing root is watered, not destroyed/refunded.
    advance(&mut world, 80);
    assert_eq!(world.tend_orchard(ROOT).unwrap(), OrchardReceipt::Watered);
    let root = world.ecology().orchard_beds[0];
    assert!(root.watered && root.last_fruit == Some(pear) && !root.offered);
    assert_eq!(world.survey_orchard_planter(root, NEW), None);

    // Making the court disappear is not a shortcut around its unfinished
    // harvest. The previous original physical tray still has that real pear.
    world.begin_player_hold(bookend).unwrap();
    world.move_player_hold(other_side).unwrap();
    assert_eq!(world.release_player_hold().unwrap(), Some(bookend));
    assert!(world.orchard_court().is_none());
    assert_eq!(world.movable_orchard_planter(ROOT), None);
    // Return the real structure without moving/replacing the original crop.
    place(&mut world, bookend, fit);
    assert_eq!(world.entity(pear).unwrap().position, actual_tray);
    assert!(!world.orchard_court().unwrap().open);

    // A real carriage away lets the exact wet planter show its original Wick
    // affordance. Target refusal remains conservative if an older pear
    // would immediately be reattached to the next site's serving footprint.
    place(&mut world, pear, parking);
    assert_eq!(world.movable_orchard_planter(ROOT), Some(root));
    assert_eq!(world.entity(pear).unwrap().radius, 0.5);
    assert_eq!(world.survey_orchard_planter(root, parked_in_another_footprint), None);
    assert_eq!(world.survey_orchard_planter(root, NEW), Some(dry));

    // The exact source captured by the earlier fit becomes invalid if the
    // same previous crop physically comes back before placement.
    place(&mut world, pear, actual_tray);
    let before_refusal = world.canonical_signature_digest().unwrap();
    assert!(world.reseat_orchard_planter(root, NEW).is_err());
    assert_eq!(world.canonical_signature_digest().unwrap(), before_refusal);
    assert_eq!(world.ecology().orchard_beds.as_slice(), &[root]);

    place(&mut world, pear, parking);
    assert!(world.orchard_court().unwrap().open);
    let moved = world.reseat_orchard_planter(root, NEW).unwrap();
    assert_eq!(moved, OrchardBed { tile: NEW, ..root });
    assert!(world.orchard_court().is_none());
    assert_eq!(world.entity(bookend).unwrap().position, fit);
    let original_pear = world.entity(pear).unwrap();
    assert_eq!((original_pear.kind, original_pear.position), (WorldObjectKind::Food, parking));
    assert!(!original_pear.consumed && original_pear.carried_by.is_none());
    assert_eq!(world.object_count(), 2);

    let saved = PortableSaveFile::from_headless_world(
        "signed-wick-after-orchard-collection", &world,
        RuntimeConfig::deterministic_default(world.seed(), BrainScaleTier::Nano512),
        AssetManifest::empty(), vec![],
    ).unwrap();
    let mut reopened = PortableSaveFile::from_json_str(&saved.to_json_string_pretty().unwrap())
        .unwrap().restore_headless_world().unwrap();
    assert_eq!(reopened.movable_orchard_planter(NEW), Some(moved));
    assert_eq!(reopened.entity(pear).unwrap().position, parking);
    assert_eq!(reopened.entity(bookend).unwrap().position, fit);
    assert_eq!(reopened.reseat_orchard_planter(moved, ROOT).unwrap(), root);
    assert_eq!(reopened.orchard_court(), Some(first));
    assert_eq!(reopened.entity(pear).unwrap().position, parking);
    assert_eq!(reopened.object_count(), 2);
}
