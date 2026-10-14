//! Injury/repair ordering through canonical production motor biology.
use alife_core::*;
use alife_world::persistence::{AssetManifest, PortableSaveFile, RuntimeConfig};
use alife_world::*;

fn canonical_game() -> CanonicalNewGame {
    let foundation =
        FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    create_canonical_new_game(
        &CanonicalNewGameConfig::phase3(240_824, 1).unwrap(),
        &foundation,
    )
    .unwrap()
}

fn idle(world: &mut HeadlessWorld, id: OrganismId, entity: WorldEntityId) {
    let command = ChannelCommand::new(
        MotorChannel::Posture,
        ActionKind::Idle.canonical_id(),
        None,
        Vec3f::ZERO,
        Intensity::new(1.0).unwrap(),
        DurationTicks::new(1),
        0.0,
        Confidence::new(0.9).unwrap(),
        0,
    )
    .unwrap();
    let bundle = MotorCommandBundle::new(
        id,
        ExperienceSequenceId::new(world.tick().raw() + 1).unwrap(),
        world.tick(),
        vec![command],
    )
    .unwrap();
    let receipt = world
        .apply_registered_motor_bundle(&bundle, entity)
        .unwrap();
    assert!(receipt.succeeded);
    assert!(receipt.body_event.damage > 0.0);
    world.try_advance_tick().unwrap();
}

#[test]
fn sustained_hazard_reaches_health_death_before_energy_exhaustion() {
    let mut world = canonical_game().world;
    let (id, entity) = world.organism_entity_ids()[0];
    let hazard = world.entity_id("hazard-01").unwrap();
    let position = world.entity(entity).unwrap().position;
    world.editor_move_object(hazard, position).unwrap();
    for _ in 0..300 {
        idle(&mut world, id, entity);
        if !world
            .organism_registry()
            .get(id)
            .unwrap()
            .lifecycle()
            .is_alive()
        {
            break;
        }
    }
    let record = world.organism_registry().get(id).unwrap();
    println!(
        "tick={} health={} energy={} alive={}",
        world.tick().raw(),
        record.biochemistry().body.health,
        record.biochemistry().body.energy,
        record.lifecycle().is_alive(),
    );
    assert!(record.biochemistry().body.energy > 0.0);
    assert_eq!(record.biochemistry().body.health, 0.0);
    assert!(!record.lifecycle().is_alive());
}

fn roundtrip(world: &HeadlessWorld, creatures: &[CreatureSaveState]) -> HeadlessWorld {
    let mut creatures = creatures.to_vec();
    for creature in &mut creatures {
        let record = world.organism_registry().get(creature.organism_id).unwrap();
        creature.mind.tick = record.biochemistry().tick;
        creature.mind.homeostasis = record.biochemistry().homeostasis;
        creature.development_tick = record.biochemistry().development.last_update_tick;
    }
    let save = PortableSaveFile::from_headless_world(
        "injury-terminal-ordering",
        world,
        RuntimeConfig::deterministic_default(world.seed(), BrainScaleTier::Nano512),
        AssetManifest::empty(),
        creatures,
    )
    .unwrap();
    PortableSaveFile::from_json_str(&serde_json::to_string(&save).unwrap())
        .unwrap()
        .restore_headless_world()
        .unwrap()
}

#[test]
fn injury_save_reload_reaches_the_same_terminal_state_and_stays_dead() {
    let game = canonical_game();
    let mut uninterrupted = game.world;
    let (id, entity) = uninterrupted.organism_entity_ids()[0];
    let hazard = uninterrupted.entity_id("hazard-01").unwrap();
    let position = uninterrupted.entity(entity).unwrap().position;
    uninterrupted.editor_move_object(hazard, position).unwrap();
    for _ in 0..80 {
        idle(&mut uninterrupted, id, entity);
    }
    let mut restored = roundtrip(&uninterrupted, &game.creatures);
    for _ in 80..300 {
        assert_eq!(
            restored.canonical_signature_digest().unwrap(),
            uninterrupted.canonical_signature_digest().unwrap()
        );
        if !uninterrupted
            .organism_registry()
            .get(id)
            .unwrap()
            .lifecycle()
            .is_alive()
        {
            break;
        }
        idle(&mut uninterrupted, id, entity);
        idle(&mut restored, id, entity);
    }
    assert!(!restored
        .organism_registry()
        .get(id)
        .unwrap()
        .lifecycle()
        .is_alive());
    assert_eq!(
        restored.canonical_signature_digest().unwrap(),
        uninterrupted.canonical_signature_digest().unwrap()
    );
    let mut dead_restored = roundtrip(&restored, &game.creatures);
    assert_eq!(
        dead_restored.canonical_signature_digest().unwrap(),
        restored.canonical_signature_digest().unwrap()
    );
    dead_restored.try_advance_tick().unwrap();
    let record = dead_restored.organism_registry().get(id).unwrap();
    assert!(!record.lifecycle().is_alive());
    assert_eq!(record.biochemistry().body.health, 0.0);
}
