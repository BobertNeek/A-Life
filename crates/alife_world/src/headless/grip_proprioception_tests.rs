//! Synthetic world-native sensing tests, not efficacy bodies.
use super::*;
use alife_core::{ActionTarget, DurationTicks};

const OBSERVER: OrganismId = OrganismId(1);

fn fixture(enabled: bool) -> (HeadlessWorld, WorldEntityId) {
    let mut world = HeadlessScenarioBuilder::new(2306)
        .agent("observer", OBSERVER, Vec3f::ZERO)
        .food("destination", Vec3f::new(3.0, 0.0, 0.0), 0.1)
        .build()
        .unwrap();
    let target = world
        .spawn_toy("carry", Vec3f::new(0.7, 0.0, 0.0), true)
        .unwrap();
    let mut genome = alife_core::CreatureGenome::early_mammal_founder(
        2306,
        alife_core::FoundationGeneticIdentity::new(
            10,
            1,
            7,
            alife_core::BrainCapacityClass::N512_ID,
        )
        .unwrap(),
    )
    .unwrap();
    if enabled {
        genome.brain.construction.maternal_architecture = genome
            .brain
            .construction
            .maternal_architecture
            .with_grip_proprioception_v5()
            .unwrap();
        genome.brain.construction.paternal_architecture = genome
            .brain
            .construction
            .paternal_architecture
            .with_grip_proprioception_v5()
            .unwrap();
    }
    let phenotype = genome.express().unwrap();
    assert_eq!(
        phenotype
            .brain_genome
            .cognitive_architecture()
            .grip_proprioception(),
        enabled
    );
    let biology = BiochemistryState::new(&phenotype, world.tick()).unwrap();
    let record = WorldOrganismRecord::new(
        OBSERVER,
        world.agent_entity_id(OBSERVER).unwrap(),
        genome,
        phenotype,
        biology,
        world.tick(),
    )
    .unwrap();
    world.register_organism_record(record).unwrap();
    (world, target)
}

fn command(action: ActionId, target: WorldEntityId) -> ActionCommand {
    ActionCommand::structured(
        OBSERVER,
        action,
        if action == HeadlessActionIds::APPROACH {
            ActionKind::Move
        } else {
            ActionKind::Interact
        },
        ActionTarget::new(Some(target), None),
        Intensity::new(1.0).unwrap(),
        DurationTicks::new(1),
        Confidence::new(1.0).unwrap(),
        0,
        None,
        None,
        None,
    )
    .unwrap()
}

fn packet(world: &HeadlessWorld, profile: SensorProfile) -> PerceptionFrameDraft {
    let tick = world.tick();
    let index = world.build_perception_batch_index().unwrap();
    let ordinary = world
        .clone()
        .perception_frame_draft(OBSERVER, tick, profile, HomeostaticSnapshot::baseline(tick))
        .unwrap();
    let indexed = world
        .clone()
        .perception_frame_draft_indexed(
            OBSERVER,
            tick,
            profile,
            HomeostaticSnapshot::baseline(tick),
            &index,
        )
        .unwrap();
    assert_eq!(
        ordinary, indexed,
        "indexed sensing must preserve the entire packet"
    );
    let bytes = serde_json::to_vec(&ordinary).unwrap();
    assert_eq!(
        serde_json::from_slice::<PerceptionFrameDraft>(&bytes).unwrap(),
        ordinary
    );
    if let Some(receipt) = ordinary.grip_proprioception() {
        assert_eq!(receipt.inspected_objects(), world.objects.len() as u64);
        assert_eq!(receipt.work_units(), world.objects.len() as u64 + 3);
        assert_eq!(
            ordinary.sensory().channels.tactile_contact[2].to_bits(),
            receipt.calibrated_output().to_bits()
        );
    }
    ordinary
}

#[test]
fn grip_actual_grab_carry_and_release_report_truthful_occupancy() {
    let (mut world, target) = fixture(true);
    assert!(!packet(&world, SensorProfile::GroundedObjectSlotsV1)
        .grip_proprioception()
        .unwrap()
        .held());
    let acquired = world
        .apply_command(&command(HeadlessActionIds::GRAB, target))
        .unwrap();
    assert!(acquired.execution.succeeded);
    assert_eq!(world.entity(target).unwrap().carried_by, Some(OBSERVER));
    assert!(packet(&world, SensorProfile::GroundedObjectSlotsV1)
        .grip_proprioception()
        .unwrap()
        .held());
    let destination = world.entity_id("destination").unwrap();
    let moved = world
        .apply_command(&command(HeadlessActionIds::APPROACH, destination))
        .unwrap();
    assert!(moved.execution.succeeded);
    assert_eq!(world.entity(target).unwrap().carried_by, Some(OBSERVER));
    assert!(packet(&world, SensorProfile::GroundedObjectSlotsV1)
        .grip_proprioception()
        .unwrap()
        .held());
    let released = world
        .apply_command(&command(HeadlessActionIds::GRAB, target))
        .unwrap();
    assert!(released.execution.succeeded);
    assert_eq!(world.entity(target).unwrap().carried_by, None);
    assert!(!packet(&world, SensorProfile::GroundedObjectSlotsV1)
        .grip_proprioception()
        .unwrap()
        .held());
}

#[test]
fn grip_touch_is_not_possession_and_other_owner_is_not_self() {
    let (mut world, target) = fixture(true);
    // Deliberate fixture geometry isolates physical touch from actual possession.
    world.objects.get_mut(&target.raw()).unwrap().position = Vec3f::new(0.01, 0.0, 0.0);
    let touched = packet(&world, SensorProfile::GroundedObjectSlotsV1);
    assert!(touched
        .grounded_object_slots()
        .iter()
        .any(|slot| slot.contact > 0.0));
    assert!(!touched.grip_proprioception().unwrap().held());
    world.objects.get_mut(&target.raw()).unwrap().carried_by = Some(OrganismId(2));
    assert!(!packet(&world, SensorProfile::GroundedObjectSlotsV1)
        .grip_proprioception()
        .unwrap()
        .held());
}

#[test]
fn grip_full_scan_reports_offscreen_ownership_without_target_features() {
    let (mut world, target) = fixture(true);
    // Deliberately offscreen ownership tests the sensor independently of hand motion.
    let object = world.objects.get_mut(&target.raw()).unwrap();
    object.position = Vec3f::new(1000.0, 0.0, 1000.0);
    object.carried_by = Some(OBSERVER);
    let draft = packet(&world, SensorProfile::GroundedObjectSlotsV1);
    assert!(draft.grip_proprioception().unwrap().held());
    assert!(draft
        .candidates()
        .iter()
        .all(|c| c.target.entity != Some(target)));
}

#[test]
fn grip_uses_current_touch_calibration_exactly_once() {
    let (mut world, target) = fixture(true);
    world
        .apply_command(&command(HeadlessActionIds::GRAB, target))
        .unwrap();
    for calibration in [-1.0, 0.0, 1.0] {
        let mut record = world.organism_registry().get(OBSERVER).unwrap().clone();
        let mut embodiment = record.embodiment().clone();
        embodiment
            .replace_calibration(
                world.tick(),
                vec![calibration; embodiment.sensor_calibration().len()],
                embodiment.effector_controllability().to_vec(),
                embodiment.body_schema().to_vec(),
            )
            .unwrap();
        let gain = embodiment.sensor_gain(SensorCapability::Touch);
        record.replace_embodiment_state(embodiment).unwrap();
        world.replace_organism_record_exact(record).unwrap();
        let draft = packet(&world, SensorProfile::GroundedObjectSlotsV1);
        assert_eq!(
            draft.grip_proprioception().unwrap().touch_gain().to_bits(),
            gain.to_bits()
        );
        assert_eq!(
            draft.sensory().channels.tactile_contact[2].to_bits(),
            gain.clamp(0.0, 1.0).to_bits()
        );
    }
}

#[test]
fn grip_opt_in_leaves_legacy_slots_and_terrain_sensing_unchanged() {
    let (mut old, old_target) = fixture(false);
    let (mut new, new_target) = fixture(true);
    for held in [false, true] {
        if held {
            old.apply_command(&command(HeadlessActionIds::GRAB, old_target))
                .unwrap();
            new.apply_command(&command(HeadlessActionIds::GRAB, new_target))
                .unwrap();
        }
        let legacy = packet(&old, SensorProfile::GroundedObjectSlotsV1);
        assert!(legacy.grip_proprioception().is_none());
        assert_eq!(legacy.sensory().channels.tactile_contact[2], 0.0);
        assert!(serde_json::to_value(&legacy)
            .unwrap()
            .get("grip_proprioception")
            .is_none());
        let terrain_old = packet(&old, SensorProfile::GroundedTerrainVisionV1);
        let terrain_new = packet(&new, SensorProfile::GroundedTerrainVisionV1);
        assert!(terrain_new.grip_proprioception().is_none());
        assert_eq!(terrain_old, terrain_new);
    }
}
