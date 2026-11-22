//! One continuous signed World receipt. No App/clicks/graphics or creature actions.
use alife_core::{BrainScaleTier, Tick, Vec3f, WorldEntityId};
use alife_world::orchard::{
    OrchardBed, OrchardCourt, OrchardGesture, OrchardReceipt, ORCHARD_COURT_REVIEW,
};
use alife_world::{
    AssetManifest, HeadlessWorld, PortableSaveFile, RuntimeConfig, VoxelTileCoord, WorldObjectKind,
};

const ROOT: VoxelTileCoord = VoxelTileCoord::new(79, 250);
const OLD_AISLE: VoxelTileCoord = VoxelTileCoord::new(80, 250);
const OLD_TRAY: VoxelTileCoord = VoxelTileCoord::new(81, 250);
const OTHER: VoxelTileCoord = VoxelTileCoord::new(79, 252);
const MOVED: VoxelTileCoord = VoxelTileCoord::new(82, 252);
const NEW_AISLE: VoxelTileCoord = VoxelTileCoord::new(83, 252);
const NEW_TRAY: VoxelTileCoord = VoxelTileCoord::new(84, 252);

fn ground(world: &HeadlessWorld, x: f32, z: f32) -> Vec3f {
    Vec3f::new(
        x,
        world.terrain().unwrap().surface().height(x, z).unwrap(),
        z,
    )
}

fn advance(world: &mut HeadlessWorld, ticks: u64) {
    for _ in 0..ticks {
        world.try_advance_tick().unwrap();
    }
}

fn settle(world: &mut HeadlessWorld, chime: WorldEntityId, at: Vec3f) -> OrchardCourt {
    let ready = world.orchard_court_preview(chime, at).unwrap();
    assert_eq!(ready.0, at);
    world.begin_player_hold(chime).unwrap();
    assert_eq!(world.orchard_court(), None);
    world.move_player_hold(at).unwrap();
    assert_eq!(world.orchard_court_preview(chime, at), Some(ready));
    assert_eq!(world.release_player_hold().unwrap(), Some(chime));
    assert_eq!(world.entity(chime).unwrap().position, at);
    assert_eq!(world.orchard_court(), Some(ready.1));
    assert!(ready.1.open);
    ready.1
}

fn return_bed(world: &HeadlessWorld, apron: VoxelTileCoord) -> Option<VoxelTileCoord> {
    // Literal World-side projection used by existing App Garden Return. This
    // follows the physical court; no click, replacement focus state or HUD.
    world
        .orchard_court()
        .filter(|c| c.reserves(apron))
        .map(|c| c.bed)
}

fn assert_no_claim(world: &HeadlessWorld, soils: [VoxelTileCoord; 2]) {
    for cell in soils {
        assert_eq!(return_bed(world, cell), None);
        assert_eq!(world.orchard_gesture(cell), Some(OrchardGesture::Sow));
    }
}

#[test]
fn signed_garden_root_and_its_single_chime_can_make_a_second_court_without_losing_care() {
    assert!(ORCHARD_COURT_REVIEW);
    let mut world = HeadlessWorld::new(1643);
    world.enable_island_for_new_game().unwrap();
    let original = ground(&world, 80.0, 251.5);
    let first = ground(&world, 83.5, 250.5);
    let second = ground(&world, 86.5, 252.5);
    let chime = world
        .spawn_toy("player-activity-toy", original, false)
        .unwrap();

    assert_eq!(world.tend_orchard(ROOT).unwrap(), OrchardReceipt::Sown);
    advance(&mut world, 1);
    assert_eq!(world.tend_orchard(OTHER).unwrap(), OrchardReceipt::Sown);
    advance(&mut world, 79);
    assert_eq!(world.tend_orchard(ROOT).unwrap(), OrchardReceipt::Watered);
    let rooted = world.movable_orchard_planter(ROOT).unwrap();
    let other = world.movable_orchard_planter(OTHER).unwrap();
    assert_eq!(
        (rooted.cycle_started, other.cycle_started),
        (Tick::ZERO, Tick::new(1))
    );
    assert!(rooted.watered && !other.watered);
    for target in [OLD_AISLE, OLD_TRAY] {
        // Both are genuine pot fits before the Chime physically arrives.
        assert!(world.survey_orchard_planter(other, target).is_some());
        assert!(world.survey_orchard_planter(rooted, target).is_some());
    }

    let old = settle(&mut world, chime, first);
    assert_eq!(
        (old.bed, old.aisle, old.landing),
        (ROOT, OLD_AISLE, OLD_TRAY)
    );
    let at_capacity = world.canonical_signature_digest().unwrap();
    for target in [OLD_AISLE, OLD_TRAY] {
        assert_eq!(return_bed(&world, target), Some(ROOT));
        assert_eq!(world.orchard_gesture(target), None);
        assert!(world.tend_orchard(target).is_err());
        // Covers both an unrelated planter and the court's own rooted plant.
        for source in [rooted, other] {
            assert_eq!(world.survey_orchard_planter(source, target), None);
            assert!(world.reseat_orchard_planter(source, target).is_err());
        }
    }
    assert_eq!(world.canonical_signature_digest().unwrap(), at_capacity);
    assert_eq!(world.orchard_court(), Some(old));

    let moved = world.reseat_orchard_planter(rooted, MOVED).unwrap();
    assert_eq!(
        moved,
        OrchardBed {
            tile: MOVED,
            ..rooted
        }
    );
    assert_eq!(world.orchard_court(), None);
    assert_no_claim(&world, [OLD_AISLE, OLD_TRAY]);
    assert_eq!(world.orchard_gesture(ROOT), Some(OrchardGesture::Sow));
    assert_eq!(world.movable_orchard_planter(OTHER), Some(other));
    assert_eq!(world.object_count(), 1);

    // An actual Wick inverse restores that original court, not just some
    // unrelated current bed. Neither operation touched the original Chime.
    assert_eq!(world.reseat_orchard_planter(moved, ROOT).unwrap(), rooted);
    assert_eq!(world.orchard_court(), Some(old));
    assert_eq!(world.entity(chime).unwrap().position, first);
    assert_eq!(world.reseat_orchard_planter(rooted, MOVED).unwrap(), moved);
    assert_no_claim(&world, [OLD_AISLE, OLD_TRAY]);

    let new = settle(&mut world, chime, second);
    assert_eq!(
        (new.chime, new.bed, new.aisle, new.landing),
        (chime, MOVED, NEW_AISLE, NEW_TRAY)
    );
    assert_eq!(new.landing_ground, ground(&world, 84.5, 252.5));
    assert_no_claim(&world, [OLD_AISLE, OLD_TRAY]);
    for target in [NEW_AISLE, NEW_TRAY] {
        assert_eq!(return_bed(&world, target), Some(MOVED));
        assert_eq!(world.orchard_gesture(target), None);
        assert!(world.tend_orchard(target).is_err());
        assert!(world.survey_orchard_planter(other, target).is_none());
    }

    let s = PortableSaveFile::from_headless_world(
        "signed-court-and-wick-single-loop",
        &world,
        RuntimeConfig::deterministic_default(world.seed(), BrainScaleTier::Nano512),
        AssetManifest::empty(),
        vec![],
    )
    .unwrap();
    let mut reopened = PortableSaveFile::from_json_str(&s.to_json_string_pretty().unwrap())
        .unwrap()
        .restore_headless_world()
        .unwrap();
    assert_eq!(reopened.orchard_court(), Some(new));
    assert_eq!(reopened.movable_orchard_planter(MOVED), Some(moved));
    assert_eq!(reopened.movable_orchard_planter(OTHER), Some(other));
    assert_eq!(reopened.entity_id("player-activity-toy"), Some(chime));

    advance(&mut reopened, 219);
    assert_eq!(reopened.orchard_gesture(MOVED), Some(OrchardGesture::Watch));
    advance(&mut reopened, 1);
    assert_eq!(reopened.orchard_gesture(MOVED), Some(OrchardGesture::Pick));
    assert_eq!(
        reopened.movable_orchard_planter(MOVED),
        None,
        "cannot move a ripe bed"
    );
    let return_to = return_bed(&reopened, NEW_TRAY).unwrap();
    assert_eq!(return_to, MOVED);
    let OrchardReceipt::Picked(pear) = reopened.tend_orchard(return_to).unwrap() else {
        panic!("the same watered root did not make an ordinary pear");
    };
    let crop = reopened.entity(pear).unwrap();
    assert_eq!(
        (crop.kind, crop.position),
        (WorldObjectKind::Food, new.landing_ground)
    );
    assert!(!crop.consumed && crop.carried_by.is_none());
    assert_eq!(reopened.entity(chime).unwrap().position, second);
    assert_eq!(
        reopened.object_count(),
        2,
        "one original Chime plus one ordinary pear"
    );
    assert_eq!(
        reopened.orchard_court(),
        Some(OrchardCourt { open: false, ..new })
    );
    assert_eq!(
        reopened.tend_orchard(MOVED).unwrap(),
        OrchardReceipt::Watching
    );
    let harvested = reopened
        .ecology()
        .orchard_beds
        .iter()
        .find(|b| b.tile == MOVED)
        .unwrap();
    assert_eq!(harvested.last_fruit, Some(pear));
    assert_eq!(reopened.movable_orchard_planter(OTHER), Some(other));
}
