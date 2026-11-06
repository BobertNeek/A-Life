use alife_core::*;
use alife_runtime::canonical_preparation::{
    route_focal_candidates, route_observed_focal_candidates,
};
use alife_world::*;

const OWN: OrganismId = OrganismId(1);
const PROFILE: SensorProfile = SensorProfile::GroundedTerrainVisionV1;

fn crowded_world() -> (HeadlessWorld, WorldEntityId) {
    let mut world = HeadlessScenarioBuilder::new(60_041)
        .agent("observer", OWN, Vec3f::ZERO)
        .build()
        .unwrap();
    let mut far = None;
    for i in 0..16 {
        let label = format!("object-{i}");
        world
            .spawn_toy(&label, Vec3f::new(1.0 + i as f32 * 0.3, 0.0, 0.2), true)
            .unwrap();
        let id = world.entity_id(&label).unwrap();
        world.editor_set_optical_opacity(id, 0.0).unwrap();
        far = Some(id);
    }
    (world, far.unwrap())
}

fn draft(world: &mut HeadlessWorld) -> (PerceptionFrameDraft, Option<GroundedCandidateBinding>) {
    world
        .perception_frame_draft_with_bindings(
            OWN,
            world.tick(),
            PROFILE,
            HomeostaticSnapshot::baseline(world.tick()),
        )
        .unwrap()
}

fn focus(draft: &PerceptionFrameDraft, identity: StableFocusIdentity) -> AttentionFrame {
    let summary = PeripheralSummary {
        identity,
        salience: SalienceComponents::default(),
        confidence: Confidence::new(1.0).unwrap(),
    };
    select_focal_targets(
        OWN,
        ExperienceSequenceId(1),
        draft.tick(),
        &[summary],
        HysteresisState::default(),
        AttentionSelectionPolicy::default(),
    )
    .unwrap()
}

#[test]
fn observed_far_focus_has_all_six_executable_opportunities() {
    let (mut world, far) = crowded_world();
    let (draft, binding) = draft(&mut world);
    assert_eq!(draft.grounded_object_slots().len(), 16);
    assert_eq!(draft.candidates().len(), 31);
    assert!(!draft
        .candidates()
        .iter()
        .any(|c| c.target.entity == Some(far)));
    let attention = focus(
        &draft,
        StableFocusIdentity::TrackedObject(draft.grounded_object_slots()[15].tracked_object_id),
    );
    let old = route_focal_candidates(&draft, &attention).unwrap();
    assert!(!old
        .candidates()
        .iter()
        .any(|c| c.target.entity == Some(far)));
    let routed = route_observed_focal_candidates(&draft, &attention, binding.as_ref()).unwrap();
    assert_eq!(routed.candidates().len(), 31);
    assert_eq!(
        routed.grounded_object_slots(),
        draft.grounded_object_slots()
    );
    assert_eq!(routed.body(), draft.body());
    assert_eq!(routed.sensory(), draft.sensory());
    assert_eq!(routed.profile_provenance(), draft.profile_provenance());
    assert_eq!(
        routed,
        route_observed_focal_candidates(&draft, &attention, binding.as_ref()).unwrap()
    );
    let actions = routed
        .candidates()
        .iter()
        .filter(|c| c.target.entity == Some(far))
        .collect::<Vec<_>>();
    assert_eq!(
        actions.len(),
        6,
        "an observed focal object lost all action access"
    );
    assert_eq!(
        actions.iter().map(|c| c.action_id).collect::<Vec<_>>(),
        vec![
            ActionKind::Inspect.canonical_id(),
            HeadlessActionIds::APPROACH,
            HeadlessActionIds::FLEE,
            HeadlessActionIds::EAT,
            HeadlessActionIds::GRAB,
            HeadlessActionIds::PLAY,
        ]
    );
    for (i, c) in actions.iter().enumerate() {
        assert_eq!(c.candidate_index, i as u16);
        assert_eq!(c.observation, CandidateObservationRef::ObjectSlot(15));
        assert_eq!(
            c.features,
            draft.grounded_object_slots()[15]
                .candidate_features()
                .unwrap()
        );
    }
    // All intrinsic/body/gaze/channel no-op descriptors keep their exact bits.
    let intrinsic = |d: &PerceptionFrameDraft| {
        d.candidates()
            .iter()
            .filter(|c| c.target == ActionTarget::NONE)
            .map(|c| {
                let mut c = *c;
                c.candidate_index = 0;
                c
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(intrinsic(&draft), intrinsic(&routed));
    assert_eq!(intrinsic(&routed).len(), 13);
    let approach = actions
        .iter()
        .find(|c| c.action_id == HeadlessActionIds::APPROACH)
        .unwrap();
    let start = world
        .entity(world.entity_id("observer").unwrap())
        .unwrap()
        .position;
    let outcome = world
        .apply_command(
            &approach
                .to_command(OWN, Confidence::new(1.0).unwrap())
                .unwrap(),
        )
        .unwrap();
    assert!(outcome.execution.succeeded);
    assert_ne!(
        world
            .entity(world.entity_id("observer").unwrap())
            .unwrap()
            .position,
        start
    );
    assert!(
        (0..16).all(|i| world.entity_id(&format!("object-{i}")).is_some()),
        "closer distractors must remain"
    );
}

#[test]
fn admitted_empty_stale_and_nonobject_focus_preserve_the_original_path() {
    let (mut world, _) = crowded_world();
    let (draft, binding) = draft(&mut world);
    for slot in &draft.grounded_object_slots()[..3] {
        let attention = focus(
            &draft,
            StableFocusIdentity::TrackedObject(slot.tracked_object_id),
        );
        assert_eq!(
            route_observed_focal_candidates(&draft, &attention, binding.as_ref()).unwrap(),
            route_focal_candidates(&draft, &attention).unwrap()
        );
    }
    let mut cases =
        vec![AttentionFrame::empty(OWN, ExperienceSequenceId(1), draft.tick()).unwrap()];
    for identity in [
        StableFocusIdentity::TrackedObject(TrackedObjectId(u64::MAX)),
        StableFocusIdentity::Organism(OrganismId(9)),
    ] {
        cases.push(focus(&draft, identity));
    }
    for attention in cases {
        assert_eq!(
            route_observed_focal_candidates(&draft, &attention, binding.as_ref()).unwrap(),
            draft
        );
    }
    let mut empty = HeadlessScenarioBuilder::new(60_042)
        .agent("observer", OWN, Vec3f::ZERO)
        .build()
        .unwrap();
    let (draft, binding) = self::draft(&mut empty);
    let attention = focus(
        &draft,
        StableFocusIdentity::TrackedObject(TrackedObjectId(1)),
    );
    assert!(draft.grounded_object_slots().is_empty());
    assert_eq!(
        route_observed_focal_candidates(&draft, &attention, binding.as_ref()).unwrap(),
        draft
    );
}

#[test]
fn disappeared_out_of_range_behind_gaze_and_occluded_focus_never_reenters() {
    let (mut source, far) = crowded_world();
    let (seen, _) = draft(&mut source);
    let id = seen.grounded_object_slots()[15].tracked_object_id;
    for mode in 0..4 {
        let mut world = source.clone();
        match mode {
            0 => {
                world.editor_remove_object(far).unwrap();
            }
            1 => world
                .editor_move_object(far, Vec3f::new(9.0, 0.0, 0.2))
                .unwrap(),
            2 => world
                .editor_move_object(far, Vec3f::new(-5.5, 0.0, 0.2))
                .unwrap(),
            _ => {
                world
                    .editor_spawn_object(WorldEditorSpawnSpec {
                        label: "occluder".into(),
                        kind: WorldObjectKind::Obstacle,
                        organism_id: None,
                        position: Vec3f::new(3.5, 0.0, 0.2),
                        nutrition: 0.0,
                        hazard_pain: 0.0,
                        radius: 0.7,
                        token_id: None,
                    })
                    .unwrap();
            }
        }
        let (draft, binding) = draft(&mut world);
        assert!(
            !draft
                .grounded_object_slots()
                .iter()
                .any(|s| s.tracked_object_id == id),
            "mode {mode}"
        );
        let attention = focus(&draft, StableFocusIdentity::TrackedObject(id));
        let routed = route_observed_focal_candidates(&draft, &attention, binding.as_ref()).unwrap();
        assert_eq!(routed, draft);
        assert!(!routed
            .candidates()
            .iter()
            .any(|c| c.target.entity == Some(far)));
    }
}

#[test]
fn contact_and_far_focus_compete_without_losing_possession_or_noops() {
    let mut world = HeadlessScenarioBuilder::new(60_043)
        .agent("observer", OWN, Vec3f::ZERO)
        .food("held", Vec3f::new(0.5, 0.0, 0.0), 0.6)
        .build()
        .unwrap();
    let held = world.entity_id("held").unwrap();
    world.editor_set_optical_opacity(held, 0.0).unwrap();
    let mut grab = ActionCommand::new(
        OWN,
        ActionKind::Interact,
        Some(held),
        Confidence::new(1.0).unwrap(),
        DurationTicks::new(1),
    )
    .unwrap();
    grab.action_id = HeadlessActionIds::GRAB;
    assert!(world.apply_command(&grab).unwrap().execution.succeeded);
    for i in 0..18 {
        let id = world
            .spawn_toy(
                &format!("contact-{i}"),
                Vec3f::new(0.1 + i as f32 * 0.015, 0.0, 0.0),
                true,
            )
            .unwrap();
        world.editor_set_optical_opacity(id, 0.0).unwrap();
    }
    let (draft, binding) = draft(&mut world);
    assert_eq!(draft.grounded_object_slots().len(), 16);
    assert!(draft
        .candidates()
        .iter()
        .any(|c| c.target.entity == Some(held)));
    let held_slot = draft
        .candidates()
        .iter()
        .find(|c| c.target.entity == Some(held))
        .unwrap()
        .observation;
    assert_eq!(held_slot, CandidateObservationRef::ObjectSlot(15));
    for i in 0..16 {
        let attention = focus(
            &draft,
            StableFocusIdentity::TrackedObject(draft.grounded_object_slots()[i].tracked_object_id),
        );
        let routed = route_observed_focal_candidates(&draft, &attention, binding.as_ref()).unwrap();
        assert_eq!(routed.candidates().len(), 31);
        assert_eq!(
            routed.candidates()[0].observation,
            CandidateObservationRef::ObjectSlot(i as u16)
        );
        assert_eq!(
            routed
                .candidates()
                .iter()
                .filter(|c| c.target.entity == Some(held))
                .count(),
            6
        );
        for (index, candidate) in routed.candidates().iter().enumerate() {
            assert_eq!(candidate.candidate_index, index as u16);
        }
        let slots = routed
            .candidates()
            .iter()
            .filter(|c| c.action_id == HeadlessActionIds::EAT)
            .map(|c| c.observation)
            .collect::<Vec<_>>();
        assert_eq!(slots.len(), 3);
        if (2..15).contains(&i) {
            assert_eq!(
                slots,
                vec![
                    CandidateObservationRef::ObjectSlot(i as u16),
                    CandidateObservationRef::ObjectSlot(0),
                    held_slot
                ]
            );
        }
        if i == 14 {
            let eat = routed
                .candidates()
                .iter()
                .find(|c| c.action_id == HeadlessActionIds::EAT && c.target.entity == Some(held))
                .unwrap();
            let mut copy = world.clone();
            copy.advance_tick();
            let meal = copy
                .apply_command(&eat.to_command(OWN, Confidence::new(1.0).unwrap()).unwrap())
                .unwrap();
            assert!(meal.execution.succeeded);
            assert!(meal.body_event.nutrition > 0.0);
        }
    }
}

#[test]
fn binding_rejects_other_snapshots_owners_and_clocks() {
    let (mut world, far) = crowded_world();
    let (seen, binding) = draft(&mut world);
    let attention = focus(
        &seen,
        StableFocusIdentity::TrackedObject(seen.grounded_object_slots()[15].tracked_object_id),
    );
    world
        .editor_move_object(far, Vec3f::new(6.0, 0.0, 1.0))
        .unwrap();
    let (moved, _) = draft(&mut world);
    assert!(route_observed_focal_candidates(&moved, &attention, binding.as_ref()).is_err());
    let mut stale = attention.clone();
    stale.world_tick = Tick::new(1);
    assert!(route_observed_focal_candidates(&seen, &stale, binding.as_ref()).is_err());
    let mut foreign = attention;
    foreign.organism_id = OrganismId(2);
    assert!(route_observed_focal_candidates(&seen, &foreign, binding.as_ref()).is_err());
    // Transport is transient; existing serialized perception remains unchanged.
    let roundtrip: PerceptionFrameDraft =
        serde_json::from_slice(&serde_json::to_vec(&seen).unwrap()).unwrap();
    assert_eq!(roundtrip, seen);
}

#[test]
fn indexed_and_scalar_capture_keep_both_profiles_bounded_and_deterministic() {
    for profile in [PROFILE, SensorProfile::GroundedObjectSlotsV1] {
        for count in 0..=16 {
            let (mut world, _) = crowded_world();
            for i in count..16 {
                let id = world.entity_id(&format!("object-{i}")).unwrap();
                world.editor_remove_object(id).unwrap();
            }
            let mut indexed = world.clone();
            let index = indexed.build_perception_batch_index().unwrap();
            let (draft, binding) = world
                .perception_frame_draft_with_bindings(
                    OWN,
                    Tick::ZERO,
                    profile,
                    HomeostaticSnapshot::baseline(Tick::ZERO),
                )
                .unwrap();
            let (other, other_binding) = indexed
                .perception_frame_draft_indexed_with_bindings(
                    OWN,
                    Tick::ZERO,
                    profile,
                    HomeostaticSnapshot::baseline(Tick::ZERO),
                    &index,
                )
                .unwrap();
            assert_eq!(draft, other);
            assert_eq!(binding, other_binding);
            let limit = if profile == PROFILE { 3 } else { 4 };
            let intrinsic = if profile == PROFILE { 13 } else { 6 };
            for slot in draft.grounded_object_slots() {
                let attention = focus(
                    &draft,
                    StableFocusIdentity::TrackedObject(slot.tracked_object_id),
                );
                let routed =
                    route_observed_focal_candidates(&draft, &attention, binding.as_ref()).unwrap();
                assert_eq!(routed.candidates().len(), intrinsic + 6 * count.min(limit));
                assert!(routed.candidates().len() <= MAX_ACTION_CANDIDATES);
                assert_eq!(
                    routed.candidates()[0].observation,
                    CandidateObservationRef::ObjectSlot(slot.slot_index)
                );
                for (i, c) in routed.candidates().iter().enumerate() {
                    assert_eq!(c.candidate_index, i as u16);
                }
            }
        }
    }
}
