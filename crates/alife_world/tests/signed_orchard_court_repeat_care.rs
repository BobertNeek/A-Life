//! Signed World-only witness of two native harvests from the same court.
//! It does not execute the App Return input, renderer, or any resident action.
use alife_core::{BrainScaleTier, Tick, Vec3f};
use alife_world::{
    orchard::{OrchardCourt, OrchardCourtCare, OrchardGesture, OrchardReceipt},
    AssetManifest, HeadlessWorld, PortableSaveFile, RuntimeConfig, VoxelTileCoord, WorldObjectKind,
};

const ROOT: VoxelTileCoord = VoxelTileCoord::new(79, 250);
const OTHER: VoxelTileCoord = VoxelTileCoord::new(79, 252);

fn ground(world: &HeadlessWorld, x: f32, z: f32) -> Vec3f {
    Vec3f::new(x, world.terrain().unwrap().surface().height(x, z).unwrap(), z)
}

fn advance(world: &mut HeadlessWorld, n: u64) {
    for _ in 0..n {
        world.try_advance_tick().unwrap();
    }
}

fn read(
    world: &HeadlessWorld, court: OrchardCourt, care: OrchardCourtCare, open: bool,
) {
    let before = world.canonical_signature_digest().unwrap();
    assert_eq!(world.orchard_court_home(court.chime), Some(ROOT));
    assert_eq!(
        world.orchard_court_care(),
        Some((OrchardCourt { open, ..court }, care)),
    );
    assert_eq!(world.canonical_signature_digest().unwrap(), before);
}

#[test]
fn signed_court_brings_care_back_to_its_own_root_for_a_second_real_pear() {
    use OrchardCourtCare::{PickAtSide, ReceivePear, Waiting, WaterRoots};

    let mut world = HeadlessWorld::new(1643);
    world.enable_island_for_new_game().unwrap();
    assert_eq!(world.tend_orchard(ROOT).unwrap(), OrchardReceipt::Sown);
    advance(&mut world, 1);
    assert_eq!(world.tend_orchard(OTHER).unwrap(), OrchardReceipt::Sown);
    let original = ground(&world, 80.0, 251.5);
    let seat = ground(&world, 83.5, 250.5);
    let parking = ground(&world, 82.5, 252.5);
    let chime = world.spawn_toy("player-activity-toy", original, false).unwrap();
    let (_, court) = world.orchard_court_preview(chime, seat).unwrap();
    assert_eq!((court.bed, court.aisle, court.landing),
        (ROOT, VoxelTileCoord::new(80, 250), VoxelTileCoord::new(81, 250)));
    world.begin_player_hold(chime).unwrap();
    world.move_player_hold(seat).unwrap();
    assert_eq!(world.orchard_court_home(chime), None);
    assert_eq!(world.orchard_court_care(), None);
    assert_eq!(world.release_player_hold().unwrap(), Some(chime));
    read(&world, court, Waiting, true);

    let saved_newer_bed = world.ecology().orchard_beds.iter()
        .find(|bed| bed.tile == OTHER).copied().unwrap();
    assert_eq!(saved_newer_bed.cycle_started, Tick::new(1));
    advance(&mut world, 78);
    read(&world, court, Waiting, true);
    advance(&mut world, 1);
    read(&world, court, WaterRoots, true);
    assert_eq!(world.tend_orchard(ROOT).unwrap(), OrchardReceipt::Watered);
    read(&world, court, Waiting, true);
    advance(&mut world, 219);
    read(&world, court, Waiting, true);
    advance(&mut world, 1);
    read(&world, court, ReceivePear, true);
    let OrchardReceipt::Picked(first) = world.tend_orchard(ROOT).unwrap() else {
        panic!("the first ready receiving court did not get its ordinary pear");
    };
    assert_eq!(world.entity(first).unwrap().position, court.landing_ground);
    assert_eq!(world.orchard_court_home(first), None, "the pear is not a home key");
    assert_eq!(world.tend_orchard(ROOT).unwrap(), OrchardReceipt::Watching);
    read(&world, court, Waiting, false);
    assert_eq!(world.object_count(), 2);

    let save = PortableSaveFile::from_headless_world(
        "signed-court-repeat-care", &world,
        RuntimeConfig::deterministic_default(world.seed(), BrainScaleTier::Nano512),
        AssetManifest::empty(), vec![],
    ).unwrap();
    let mut reopened = PortableSaveFile::from_json_str(&save.to_json_string_pretty().unwrap())
        .unwrap().restore_headless_world().unwrap();
    read(&reopened, court, Waiting, false);
    advance(&mut reopened, 79);
    read(&reopened, court, Waiting, false);
    advance(&mut reopened, 1);
    read(&reopened, court, WaterRoots, false);
    assert_eq!(reopened.tend_orchard(ROOT).unwrap(), OrchardReceipt::Watered);
    advance(&mut reopened, 220);
    // The old real pear still occupies this tray. No promise of a court landing;
    // the body's ordinary side harvest is still available, and apron remains reserved.
    assert_eq!(reopened.orchard_gesture(ROOT), Some(OrchardGesture::Pick));
    read(&reopened, court, PickAtSide, false);
    assert!(reopened.tend_orchard(court.landing).is_err());
    assert_eq!(reopened.object_count(), 2);
    assert_eq!(reopened.entity(first).unwrap().position, court.landing_ground);

    // Taking the actual stock pear out of the tray is sufficient. We do not
    // claim it was eaten, replace it, or inject a simulated resident visit.
    reopened.begin_player_hold(first).unwrap();
    reopened.move_player_hold(parking).unwrap();
    assert_eq!(reopened.release_player_hold().unwrap(), Some(first));
    assert_eq!(reopened.entity(first).unwrap().position, parking);
    read(&reopened, court, ReceivePear, true);
    let OrchardReceipt::Picked(second) = reopened.tend_orchard(ROOT).unwrap() else {
        panic!("the same court did not receive the second real pear");
    };
    assert_ne!(second, first);
    let pear = reopened.entity(second).unwrap();
    assert_eq!((pear.position, pear.kind), (court.landing_ground, WorldObjectKind::Food));
    assert!(!pear.consumed && pear.carried_by.is_none());
    read(&reopened, court, Waiting, false);
    assert_eq!(reopened.object_count(), 3);
    assert_eq!(reopened.entity(chime).unwrap().position, seat);
    assert_eq!(reopened.entity(first).unwrap().position, parking);
    assert_eq!(reopened.ecology().orchard_beds.iter()
        .find(|bed| bed.tile == OTHER).copied(), Some(saved_newer_bed));
    let record = reopened.ecology().orchard_beds.iter().find(|bed| bed.tile == ROOT).unwrap();
    assert_eq!((record.cycle_started, record.watered, record.last_fruit),
        (Tick::new(600), false, Some(second)));
}
