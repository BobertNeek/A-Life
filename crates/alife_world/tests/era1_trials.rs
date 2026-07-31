use alife_core::{HomeostaticSnapshot, OrganismId, SensorProfile, Tick};
use alife_world::{
    apply_era1_world_transition, build_era1_trial_world, Era1TrialManifest, Era1TrialPhase,
    Era1WorldFamily, ERA1_ACQUISITION_END_TICK, ERA1_PROBE_START_TICK, ERA1_WORLD_FAMILY_COUNT,
};

const SUBJECT: OrganismId = OrganismId(101);
const FAMILIAR: OrganismId = OrganismId(102);
const NOVEL: OrganismId = OrganismId(103);

fn manifest(family: Era1WorldFamily, held_out_transform: bool) -> Era1TrialManifest {
    Era1TrialManifest::new(
        44_001,
        family,
        SUBJECT,
        FAMILIAR,
        NOVEL,
        7,
        held_out_transform,
        41,
    )
    .unwrap()
}

#[test]
fn manifest_and_world_digest_are_byte_stable_but_held_out_layout_is_distinct() {
    let baseline = manifest(Era1WorldFamily::TransformedObjectsLayout, false);
    let replay = manifest(Era1WorldFamily::TransformedObjectsLayout, false);
    let held_out = manifest(Era1WorldFamily::TransformedObjectsLayout, true);

    assert_eq!(
        baseline.canonical_bytes().unwrap(),
        replay.canonical_bytes().unwrap()
    );
    assert_ne!(
        baseline.canonical_bytes().unwrap(),
        held_out.canonical_bytes().unwrap()
    );

    let baseline_world = build_era1_trial_world(&baseline).unwrap();
    let replay_world = build_era1_trial_world(&replay).unwrap();
    let held_out_world = build_era1_trial_world(&held_out).unwrap();
    assert_eq!(
        baseline_world.canonical_signature_digest().unwrap(),
        replay_world.canonical_signature_digest().unwrap()
    );
    assert_ne!(
        baseline_world.canonical_signature_digest().unwrap(),
        held_out_world.canonical_signature_digest().unwrap()
    );
}

#[test]
fn every_required_family_builds_a_real_unscored_world() {
    assert_eq!(ERA1_WORLD_FAMILY_COUNT, 8);
    assert_eq!(Era1WorldFamily::ALL.len(), ERA1_WORLD_FAMILY_COUNT);

    for family in Era1WorldFamily::ALL {
        let manifest = manifest(family, false);
        let encoded = String::from_utf8(manifest.canonical_bytes().unwrap()).unwrap();
        for forbidden in ["score", "answer", "selected_action", "reward_value"] {
            assert!(
                !encoded.contains(forbidden),
                "manifest leaked forbidden field {forbidden}: {encoded}"
            );
        }
        let world = build_era1_trial_world(&manifest).unwrap();
        assert!(
            world.object_count() > 1,
            "{family:?} produced an empty fixture"
        );
        assert_ne!(world.canonical_signature_digest().unwrap().words, [0; 4]);
    }
}

#[test]
fn grounded_frames_expose_only_physical_slots_and_stable_tracked_ids() {
    let manifest = manifest(Era1WorldFamily::ForagingHazardMaze, false);
    let mut world = build_era1_trial_world(&manifest).unwrap();

    let first = world
        .perception_frame_draft(
            SUBJECT,
            Tick::ZERO,
            SensorProfile::GroundedObjectSlotsV1,
            HomeostaticSnapshot::baseline(Tick::ZERO),
        )
        .unwrap();
    let second = world
        .perception_frame_draft(
            SUBJECT,
            Tick::new(1),
            SensorProfile::GroundedObjectSlotsV1,
            HomeostaticSnapshot::baseline(Tick::new(1)),
        )
        .unwrap();

    assert!(!first.grounded_object_slots().is_empty());
    assert!(first.sensory().semantic_context.is_none());
    assert_eq!(first.sensory().channels.nearby_affordances.raw(), 0);
    assert_eq!(
        first
            .grounded_object_slots()
            .iter()
            .map(|slot| slot.tracked_object_id)
            .collect::<Vec<_>>(),
        second
            .grounded_object_slots()
            .iter()
            .map(|slot| slot.tracked_object_id)
            .collect::<Vec<_>>()
    );
}

#[test]
fn phase_transitions_are_exact_and_apply_real_world_changes() {
    let manifest = manifest(Era1WorldFamily::DelayedLocation, false);
    let transitions = manifest.transitions();
    assert_eq!(transitions[0].at_tick, Tick::new(ERA1_ACQUISITION_END_TICK));
    assert_eq!(transitions[0].from, Era1TrialPhase::Acquisition);
    assert_eq!(transitions[0].to, Era1TrialPhase::Delay);
    assert_eq!(transitions[1].at_tick, Tick::new(ERA1_PROBE_START_TICK));
    assert_eq!(transitions[1].from, Era1TrialPhase::Delay);
    assert_eq!(transitions[1].to, Era1TrialPhase::Probe);

    assert_eq!(
        manifest
            .phase_at(Tick::new(ERA1_ACQUISITION_END_TICK - 1))
            .unwrap(),
        Era1TrialPhase::Acquisition
    );
    assert_eq!(
        manifest
            .phase_at(Tick::new(ERA1_ACQUISITION_END_TICK))
            .unwrap(),
        Era1TrialPhase::Delay
    );
    assert_eq!(
        manifest.phase_at(Tick::new(ERA1_PROBE_START_TICK)).unwrap(),
        Era1TrialPhase::Probe
    );

    let mut world = build_era1_trial_world(&manifest).unwrap();
    let cue = world.entity_id("era1-cue").unwrap();
    while world.tick().raw() < ERA1_ACQUISITION_END_TICK {
        world.advance_tick();
    }
    apply_era1_world_transition(&manifest, transitions[0], &mut world).unwrap();
    assert!(
        world.entity(cue).is_none(),
        "delay transition did not hide the cue"
    );

    assert!(apply_era1_world_transition(&manifest, transitions[1], &mut world).is_err());
    while world.tick().raw() < ERA1_PROBE_START_TICK {
        world.advance_tick();
    }
    apply_era1_world_transition(&manifest, transitions[1], &mut world).unwrap();
}

#[test]
fn familiar_and_novel_individuals_are_distinct_and_track_stably() {
    let manifest = manifest(Era1WorldFamily::FamiliarNovelIndividual, false);
    let mut world = build_era1_trial_world(&manifest).unwrap();
    let transitions = manifest.transitions();
    assert_eq!(world.organism_entity_ids().len(), 2);
    while world.tick().raw() < ERA1_ACQUISITION_END_TICK {
        world.advance_tick();
    }
    apply_era1_world_transition(&manifest, transitions[0], &mut world).unwrap();
    while world.tick().raw() < ERA1_PROBE_START_TICK {
        world.advance_tick();
    }
    apply_era1_world_transition(&manifest, transitions[1], &mut world).unwrap();

    let organisms = world.organism_entity_ids();
    assert_eq!(organisms.len(), 3);
    assert!(organisms.iter().any(|(organism, _)| *organism == FAMILIAR));
    assert!(organisms.iter().any(|(organism, _)| *organism == NOVEL));

    let first = world
        .perception_frame_draft(
            SUBJECT,
            Tick::new(ERA1_PROBE_START_TICK),
            SensorProfile::GroundedObjectSlotsV1,
            HomeostaticSnapshot::baseline(Tick::new(ERA1_PROBE_START_TICK)),
        )
        .unwrap();
    let second = world
        .perception_frame_draft(
            SUBJECT,
            Tick::new(ERA1_PROBE_START_TICK + 1),
            SensorProfile::GroundedObjectSlotsV1,
            HomeostaticSnapshot::baseline(Tick::new(ERA1_PROBE_START_TICK + 1)),
        )
        .unwrap();
    let first_ids = first
        .grounded_object_slots()
        .iter()
        .map(|slot| slot.tracked_object_id)
        .collect::<Vec<_>>();
    let second_ids = second
        .grounded_object_slots()
        .iter()
        .map(|slot| slot.tracked_object_id)
        .collect::<Vec<_>>();
    assert_eq!(first_ids.len(), 2);
    assert_ne!(first_ids[0], first_ids[1]);
    assert_eq!(first_ids, second_ids);
}

#[test]
fn invalid_identity_and_token_contracts_are_rejected() {
    assert!(Era1TrialManifest::new(
        0,
        Era1WorldFamily::GroundedVocabulary,
        SUBJECT,
        FAMILIAR,
        NOVEL,
        7,
        false,
        41,
    )
    .is_err());
    assert!(Era1TrialManifest::new(
        44_001,
        Era1WorldFamily::GroundedVocabulary,
        SUBJECT,
        SUBJECT,
        NOVEL,
        7,
        false,
        41,
    )
    .is_err());
    assert!(Era1TrialManifest::new(
        44_001,
        Era1WorldFamily::GroundedVocabulary,
        SUBJECT,
        FAMILIAR,
        NOVEL,
        7,
        false,
        0,
    )
    .is_err());
}
