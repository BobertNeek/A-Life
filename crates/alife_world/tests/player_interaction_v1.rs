use alife_core::*;
use alife_world::*;

fn game() -> CanonicalNewGame {
    let foundation =
        FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    create_canonical_new_game(
        &CanonicalNewGameConfig::phase3(2116, 1).unwrap(),
        &foundation,
    )
    .unwrap()
}

#[test]
fn all_four_are_distinct_capped_once_only_body_experiences() {
    let game = game();
    let base = game.world;
    let (id, entity) = base.organism_entity_ids()[0];
    let position = base.entity(entity).unwrap().position;
    let command = HeadlessWorldCommand::idle(id).unwrap();
    let mut control = base.clone();
    let none = control
        .apply_registered_command(&command, entity, Tick(1))
        .unwrap();
    for kind in [
        PlayerInteractionV1::Praise,
        PlayerInteractionV1::Scold,
        PlayerInteractionV1::Pet,
        PlayerInteractionV1::Slap,
    ] {
        let mut world = base.clone();
        for _ in 0..100 {
            world
                .queue_player_interaction_v1(id, position, kind)
                .unwrap();
        }
        let applied = world
            .apply_registered_command(&command, entity, Tick(1))
            .unwrap();
        let event = applied.action_result.body_event;
        assert_eq!(event.nutrition, 0.0);
        match kind {
            PlayerInteractionV1::Praise => {
                assert_eq!(event.player_reward, 1.0);
                assert!(
                    applied.biology_after.homeostasis.hormones.extension[0]
                        > none.biology_after.homeostasis.hormones.extension[0]
                );
            }
            PlayerInteractionV1::Scold => {
                assert_eq!(event.social_disapproval_v1, 1.0);
                assert_eq!(event.damage, 0.0);
                assert_eq!(applied.biology_after.body, none.biology_after.body);
                assert!(
                    applied.biology_after.homeostasis.drives.fear
                        > none.biology_after.homeostasis.drives.fear
                );
            }
            PlayerInteractionV1::Pet => {
                assert_eq!(event.social_contact, 0.25);
                assert_eq!(event.damage, 0.0);
            }
            PlayerInteractionV1::Slap => {
                assert_eq!(event.damage, 0.02);
                assert!(applied.biology_after.body.injury > none.biology_after.body.injury);
                assert!(
                    applied.biology_after.homeostasis.drives.pain
                        > none.biology_after.homeostasis.drives.pain
                );
            }
        }
        world.try_advance_tick().unwrap();
        let next = world
            .apply_registered_command(&command, entity, Tick(2))
            .unwrap();
        let cleared = next.action_result.body_event;
        assert_eq!(
            (
                cleared.player_reward,
                cleared.social_contact,
                cleared.social_disapproval_v1,
                cleared.damage
            ),
            (0.0, 0.0, 0.0, 0.0)
        );
    }
}

#[test]
fn rejection_and_pending_save_are_transactional_for_all_four() {
    let game = game();
    let mut world = game.world;
    let (id, entity) = world.organism_entity_ids()[0];
    let position = world.entity(entity).unwrap().position;
    for kind in [
        PlayerInteractionV1::Praise,
        PlayerInteractionV1::Scold,
        PlayerInteractionV1::Pet,
        PlayerInteractionV1::Slap,
    ] {
        let before = world.canonical_signature_digest().unwrap();
        assert!(world
            .queue_player_interaction_v1(id, Vec3f::new(100., 0., 0.), kind)
            .is_err());
        assert!(world
            .queue_player_interaction_v1(OrganismId::new(99999).unwrap(), position, kind)
            .is_err());
        assert_eq!(world.canonical_signature_digest().unwrap(), before);
        world
            .queue_player_interaction_v1(id, position, kind)
            .unwrap();
    }
    let save = PortableSaveFile::from_headless_world(
        "four",
        &world,
        RuntimeConfig::deterministic_default(world.seed(), BrainScaleTier::Nano512),
        AssetManifest::empty(),
        game.creatures,
    )
    .unwrap();
    // A new-format pending slap is bounded; serialized input is not a way to
    // smuggle arbitrary damage into the player event queue.
    let mut invalid = serde_json::to_value(&save).unwrap();
    fn corrupt_pending(value: &mut serde_json::Value) -> bool {
        match value {
            serde_json::Value::Object(map) => {
                if let Some(pending) = map
                    .get_mut("pending_player_care")
                    .and_then(serde_json::Value::as_object_mut)
                {
                    if let Some(event) = pending.values_mut().next() {
                        event["damage"] = serde_json::json!(0.5);
                        return true;
                    }
                }
                map.values_mut().any(corrupt_pending)
            }
            serde_json::Value::Array(values) => values.iter_mut().any(corrupt_pending),
            _ => false,
        }
    }
    assert!(corrupt_pending(&mut invalid));
    assert!(PortableSaveFile::from_json_str(&invalid.to_string())
        .and_then(|save| save.restore_headless_world())
        .is_err());
    let mut restored = PortableSaveFile::from_json_str(&serde_json::to_string(&save).unwrap())
        .unwrap()
        .restore_headless_world()
        .unwrap();
    assert_eq!(
        world.canonical_signature_digest().unwrap(),
        restored.canonical_signature_digest().unwrap()
    );
    world.try_advance_tick().unwrap();
    restored.try_advance_tick().unwrap();
    assert_eq!(
        world.canonical_signature_digest().unwrap(),
        restored.canonical_signature_digest().unwrap()
    );
    let before = world.canonical_signature_digest().unwrap();
    assert!(world
        .apply_registered_command(&HeadlessWorldCommand::idle(id).unwrap(), entity, Tick(0))
        .is_err());
    assert_eq!(world.canonical_signature_digest().unwrap(), before);
}

#[test]
fn scold_uses_received_hearing_and_max_gain_is_bounded() {
    for hearing in [false, true] {
        let mut world = game().world;
        let (id, entity) = world.organism_entity_ids()[0];
        let position = world.entity(entity).unwrap().position;
        let mut record = world.organism_registry().get(id).unwrap().clone();
        // Test-only construction of an alternate validated embodiment, not a
        // production creature/save migration or acquired-state edit.
        let mut value = serde_json::to_value(record.embodiment()).unwrap();
        value["revision"] = serde_json::json!(record.embodiment().revision() + 1);
        let ports = value["sensors"].as_array().unwrap().clone();
        let old = value["sensor_calibration"].as_array().unwrap();
        let mut offset = 0;
        let mut sensors = vec![];
        let mut calibration = vec![];
        for port in ports {
            let width = port["value_lanes"].as_u64().unwrap() as usize;
            let is_hearing = port["capability"] == "Hearing";
            if !is_hearing || hearing {
                sensors.push(port);
                calibration.extend((offset..offset + width).map(|i| {
                    if is_hearing {
                        serde_json::json!(1.0)
                    } else {
                        old[i].clone()
                    }
                }));
            }
            offset += width;
        }
        value["sensors"] = serde_json::json!(sensors);
        value["sensor_calibration"] = serde_json::json!(calibration);
        record
            .replace_embodiment_state(serde_json::from_value(value).unwrap())
            .unwrap();
        assert_eq!(
            record.embodiment().sensor_gain(SensorCapability::Hearing),
            if hearing { 1.5 } else { 0.0 }
        );
        world.replace_organism_record_exact(record).unwrap();
        let before = world.canonical_signature_digest().unwrap();
        let outcome = world
            .queue_player_interaction_v1(id, position, PlayerInteractionV1::Scold)
            .unwrap();
        assert_eq!(
            outcome,
            if hearing {
                PlayerInteractionQueueOutcomeV1::Queued
            } else {
                PlayerInteractionQueueOutcomeV1::NoReceivedDose
            }
        );
        if !hearing {
            assert_eq!(before, world.canonical_signature_digest().unwrap());
        }
        let homeostasis = world
            .organism_registry()
            .get(id)
            .unwrap()
            .biochemistry()
            .homeostasis;
        let frame = world
            .perception_frame(
                id,
                Tick(0),
                SensorProfile::GroundedObjectSlotsV1,
                homeostasis,
            )
            .unwrap();
        frame.validate_contract().unwrap();
        assert_eq!(
            frame.sensory().channels.auditory_acoustic[2],
            if hearing { 1.0 } else { 0.0 }
        );
        let result = world
            .apply_registered_command(&HeadlessWorldCommand::idle(id).unwrap(), entity, Tick(1))
            .unwrap();
        assert_eq!(
            result.action_result.body_event.social_disapproval_v1,
            if hearing { 1.0 } else { 0.0 }
        );
    }
}

#[test]
fn blocked_inputs_leave_no_pending_dose() {
    let mut world = game().world;
    let (id, entity) = world.organism_entity_ids()[0];
    let p = world.entity(entity).unwrap().position;
    world
        .editor_spawn_object(WorldEditorSpawnSpec {
            label: "feedback-wall".to_string(),
            kind: WorldObjectKind::Obstacle,
            organism_id: None,
            position: Vec3f::new(p.x + 0.3, p.y, p.z),
            nutrition: 0.0,
            hazard_pain: 0.0,
            radius: 0.15,
            token_id: None,
        })
        .unwrap();
    let source = Vec3f::new(p.x + 0.6, p.y, p.z);
    let before = world.canonical_signature_digest().unwrap();
    for kind in [
        PlayerInteractionV1::Praise,
        PlayerInteractionV1::Scold,
        PlayerInteractionV1::Pet,
        PlayerInteractionV1::Slap,
    ] {
        assert!(world.queue_player_interaction_v1(id, source, kind).is_err());
        assert_eq!(world.canonical_signature_digest().unwrap(), before);
    }
}

#[test]
fn new_physical_cue_is_present_in_all_grounded_profiles() {
    for profile in [
        SensorProfile::GroundedObjectSlotsV1,
        SensorProfile::GroundedTerrainVisionV1,
        SensorProfile::GroundedTerrainObjectPortsV2,
    ] {
        let mut world = game().world;
        let (id, entity) = world.organism_entity_ids()[0];
        let p = world.entity(entity).unwrap().position;
        let record = world.organism_registry().get(id).unwrap();
        let homeostasis = record.biochemistry().homeostasis;
        let touch_gain = record.embodiment().sensor_gain(SensorCapability::Touch);
        let mut control = world.clone();
        let baseline = control
            .perception_frame(id, Tick(0), profile, homeostasis)
            .unwrap();
        world
            .queue_player_interaction_v1(id, p, PlayerInteractionV1::Slap)
            .unwrap();
        let frame = world
            .perception_frame(id, Tick(0), profile, homeostasis)
            .unwrap();
        frame.validate_contract().unwrap();
        assert_eq!(
            frame.sensory().channels.tactile_contact[0],
            baseline.sensory().channels.tactile_contact[0].max((0.02 * touch_gain).clamp(0.0, 1.0))
        );
    }
}
