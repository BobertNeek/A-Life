use alife_core::{
    ActionId, ActionKind, AffordanceBits, CandidateActionFamily, CandidateObservationRef,
    HomeostaticSnapshot, OrganismId, PerceptionContextBlock, ScaffoldContractError, SensorProfile,
    TeacherPerceptionChannel, Tick, Vec3f, WorldEntityId, CANDIDATE_FEATURE_COUNT,
    MAX_ACTION_CANDIDATES,
};
use alife_world::{
    CandidateEnumerator, HeadlessActionIds, HeadlessCandidateEnumerator, HeadlessScenarioBuilder,
    HeadlessWorld, CANDIDATE_FEATURE_AFFORDANCE_COUNT, CANDIDATE_FEATURE_AFFORDANCE_START_LANE,
    CANDIDATE_FEATURE_BEARING_COS_LANE, CANDIDATE_FEATURE_BEARING_SIN_LANE,
    CANDIDATE_FEATURE_CONTACT_LANE, CANDIDATE_FEATURE_DISTANCE_LANE,
    CANDIDATE_FEATURE_EVIDENCE_LANE, CANDIDATE_FEATURE_RELATIVE_VELOCITY_X_LANE,
    CANDIDATE_FEATURE_RELATIVE_VELOCITY_Y_LANE, CANDIDATE_FEATURE_RELATIVE_VELOCITY_Z_LANE,
    CANDIDATE_FEATURE_RESERVED_START_LANE, HEADLESS_VISION_RADIUS,
};

const ORGANISM: OrganismId = OrganismId(17);

fn pos(x: f32, y: f32) -> Vec3f {
    Vec3f::new(x, y, 0.0)
}

fn frame_world(kind: SemanticFixtureKind) -> HeadlessWorld {
    let builder = HeadlessScenarioBuilder::new(41).agent("agent", ORGANISM, pos(0.0, 0.0));
    match kind {
        SemanticFixtureKind::Food => builder.food("object", pos(1.0, 0.0), 0.7),
        SemanticFixtureKind::Hazard => builder.hazard("object", pos(1.0, 0.0), 0.7),
    }
    .build()
    .unwrap()
}

#[derive(Clone, Copy)]
enum SemanticFixtureKind {
    Food,
    Hazard,
}

type TransportSignatureRow = (
    u16,
    ActionId,
    ActionKind,
    CandidateActionFamily,
    CandidateObservationRef,
    Option<WorldEntityId>,
    Option<Vec3f>,
);

fn transport_signature(frame: &alife_core::PerceptionFrame) -> Vec<TransportSignatureRow> {
    frame
        .candidates()
        .iter()
        .map(|candidate| {
            (
                candidate.candidate_index,
                candidate.action_id,
                candidate.kind,
                candidate.family,
                candidate.observation,
                candidate.target.entity,
                candidate.target.position,
            )
        })
        .collect()
}

fn perception_frame(
    world: &HeadlessWorld,
    tick: Tick,
    profile: SensorProfile,
) -> alife_core::PerceptionFrame {
    world
        .perception_frame(ORGANISM, tick, profile, HomeostaticSnapshot::baseline(tick))
        .unwrap()
}

#[test]
fn perception_and_candidates_share_one_authoritative_tick_and_empty_context() {
    let world = frame_world(SemanticFixtureKind::Food);
    let tick = Tick::new(3);
    let homeostasis = HomeostaticSnapshot::baseline(tick);
    let report = world.sensory_report(ORGANISM, tick).unwrap();
    let enumerated = HeadlessCandidateEnumerator
        .enumerate_candidates(&report, SensorProfile::PrivilegedAffordanceV1)
        .unwrap();

    let draft = world
        .perception_frame_draft(
            ORGANISM,
            tick,
            SensorProfile::PrivilegedAffordanceV1,
            homeostasis,
        )
        .unwrap();
    let frame = world
        .perception_frame(
            ORGANISM,
            tick,
            SensorProfile::PrivilegedAffordanceV1,
            homeostasis,
        )
        .unwrap();

    assert_eq!(draft.tick(), tick);
    assert_eq!(
        draft.body().pose.translation,
        report.core_snapshot.observer_position
    );
    assert_eq!(draft.sensory().tick, tick);
    assert_eq!(draft.sensory(), &report.core_snapshot);
    assert_eq!(draft.homeostasis().tick, tick);
    assert_eq!(frame.tick(), frame.sensory().tick);
    assert_eq!(frame.tick(), frame.homeostasis().tick);
    assert_eq!(draft.base_digest(), frame.base_digest());
    assert_eq!(draft.candidates(), enumerated);
    assert_eq!(draft.candidates(), frame.candidates());
    let visible = &report.visible_entities[0];
    for candidate in &draft.candidates()[1..6] {
        assert_eq!(candidate.target.entity, Some(visible.id));
        assert_eq!(
            candidate.observation,
            CandidateObservationRef::ObjectSlot(0)
        );
        assert!(candidate.features.0[CANDIDATE_FEATURE_BEARING_SIN_LANE].abs() < 1e-6);
        assert!((candidate.features.0[CANDIDATE_FEATURE_BEARING_COS_LANE] - 1.0).abs() < 1e-6);
        assert!(
            (candidate.features.0[CANDIDATE_FEATURE_DISTANCE_LANE]
                - visible.distance / HEADLESS_VISION_RADIUS)
                .abs()
                < 1e-6
        );
    }
    assert_eq!(
        draft.finalize(PerceptionContextBlock::empty()).unwrap(),
        frame
    );
}

#[test]
fn idle_is_index_zero_and_all_candidate_indices_are_contiguous() {
    let world = frame_world(SemanticFixtureKind::Food);
    let frame = perception_frame(&world, Tick::new(5), SensorProfile::PrivilegedAffordanceV1);

    assert_eq!(frame.candidates()[0].candidate_index, 0);
    assert_eq!(
        frame.candidates()[0].action_id,
        ActionKind::Idle.canonical_id()
    );
    assert_eq!(frame.candidates()[0].kind, ActionKind::Idle);
    assert_eq!(frame.candidates()[0].family, CandidateActionFamily::Idle);
    assert_eq!(
        frame.candidates()[0].observation,
        CandidateObservationRef::None
    );
    assert_eq!(frame.candidates()[0].target.entity, None);
    assert_eq!(frame.candidates()[0].target.position, None);
    assert_eq!(
        frame
            .candidates()
            .iter()
            .map(|candidate| candidate.candidate_index)
            .collect::<Vec<_>>(),
        (0..frame.candidates().len() as u16).collect::<Vec<_>>()
    );
}

#[test]
fn candidate_enumerator_source_contains_observations_not_privileged_scores() {
    let source = include_str!("../src/candidate_enumerator.rs");
    for forbidden in [
        "ActionProposal",
        "CreatureMind",
        "WorldObjectKind",
        "food_score",
        "hazard_score",
        "proposal_salience",
        "proximity_salience",
        "score_candidate",
        "candidate_utility",
        "static_utility",
        "utility_score",
        ".nutrition",
        ".hazard_pain",
        "0.72",
        "0.66",
        "0.38",
        "0.42",
        "0.28",
    ] {
        assert!(
            !source.contains(forbidden),
            "candidate enumeration contains forbidden utility source {forbidden:?}"
        );
    }
}

#[test]
fn privileged_feature_lanes_have_exact_geometry_affordance_evidence_and_reserved_values() {
    let world = HeadlessScenarioBuilder::new(73)
        .agent("agent", ORGANISM, pos(0.0, 0.0))
        .food("three_four_five", pos(3.0, 4.0), 0.9)
        .build()
        .unwrap();
    let tick = Tick::new(6);
    let report = world.sensory_report(ORGANISM, tick).unwrap();
    let visible = &report.visible_entities[0];
    let candidates = HeadlessCandidateEnumerator
        .enumerate_candidates(&report, SensorProfile::PrivilegedAffordanceV1)
        .unwrap();

    assert_eq!(visible.relative_position, pos(3.0, 4.0));
    assert!((visible.distance - 5.0).abs() < 1e-6);
    assert_eq!(CANDIDATE_FEATURE_BEARING_SIN_LANE, 0);
    assert_eq!(CANDIDATE_FEATURE_BEARING_COS_LANE, 1);
    assert_eq!(CANDIDATE_FEATURE_DISTANCE_LANE, 2);
    assert_eq!(CANDIDATE_FEATURE_RELATIVE_VELOCITY_X_LANE, 3);
    assert_eq!(CANDIDATE_FEATURE_RELATIVE_VELOCITY_Y_LANE, 4);
    assert_eq!(CANDIDATE_FEATURE_RELATIVE_VELOCITY_Z_LANE, 5);
    assert_eq!(CANDIDATE_FEATURE_AFFORDANCE_START_LANE, 6);
    assert_eq!(CANDIDATE_FEATURE_AFFORDANCE_COUNT, 10);
    assert_eq!(CANDIDATE_FEATURE_CONTACT_LANE, 16);
    assert_eq!(CANDIDATE_FEATURE_EVIDENCE_LANE, 17);
    assert_eq!(CANDIDATE_FEATURE_RESERVED_START_LANE, 18);
    assert_eq!(CANDIDATE_FEATURE_COUNT, 24);

    let affordances = [
        AffordanceBits::FOOD,
        AffordanceBits::WATER,
        AffordanceBits::HAZARD,
        AffordanceBits::MATE,
        AffordanceBits::SOCIAL_AGENT,
        AffordanceBits::SHELTER,
        AffordanceBits::TOOL,
        AffordanceBits::GLYPH_OR_WRITING,
        AffordanceBits::TEACHER_OBJECT,
        AffordanceBits::RESOURCE,
    ];
    let mut golden = [0.0_f32; CANDIDATE_FEATURE_COUNT];
    golden[CANDIDATE_FEATURE_BEARING_SIN_LANE] = 4.0 / 5.0;
    golden[CANDIDATE_FEATURE_BEARING_COS_LANE] = 3.0 / 5.0;
    golden[CANDIDATE_FEATURE_DISTANCE_LANE] = 5.0 / HEADLESS_VISION_RADIUS;
    golden[CANDIDATE_FEATURE_RELATIVE_VELOCITY_X_LANE] = 0.0;
    golden[CANDIDATE_FEATURE_RELATIVE_VELOCITY_Y_LANE] = 0.0;
    golden[CANDIDATE_FEATURE_RELATIVE_VELOCITY_Z_LANE] = 0.0;
    for (offset, affordance) in affordances.into_iter().enumerate() {
        golden[CANDIDATE_FEATURE_AFFORDANCE_START_LANE + offset] =
            if visible.affordances.contains(affordance) {
                1.0
            } else {
                0.0
            };
    }
    golden[CANDIDATE_FEATURE_CONTACT_LANE] = 0.0;
    golden[CANDIDATE_FEATURE_EVIDENCE_LANE] = 1.0;

    for candidate in &candidates[1..6] {
        assert_eq!(candidate.features, candidates[1].features);
        candidate.features.validate().unwrap();
        for (lane, (actual, expected)) in candidate.features.0.iter().zip(golden).enumerate() {
            assert!(
                (*actual - expected).abs() < 1e-6,
                "feature lane {lane}: expected {expected}, got {actual}"
            );
            assert!(actual.is_finite() && (-1.0..=1.0).contains(actual));
        }
        assert!(
            candidate.features.0[CANDIDATE_FEATURE_RESERVED_START_LANE..]
                .iter()
                .all(|value| *value == 0.0)
        );
    }
}

#[test]
fn distance_then_entity_order_is_stable_and_caps_at_six_objects() {
    let world = HeadlessScenarioBuilder::new(99)
        .agent("agent", ORGANISM, pos(0.0, 0.0))
        .food("id2_d3", pos(3.0, 0.0), 0.5)
        .food("id3_d1", pos(1.0, 0.0), 0.5)
        .food("id4_d1", pos(0.0, 1.0), 0.5)
        .food("id5_d2", pos(2.0, 0.0), 0.5)
        .food("id6_d4", pos(4.0, 0.0), 0.5)
        .food("id7_d2", pos(0.0, 2.0), 0.5)
        .food("id8_d0_5", pos(0.5, 0.0), 0.5)
        .food("id9_d5", pos(5.0, 0.0), 0.5)
        .build()
        .unwrap();
    let frame = perception_frame(&world, Tick::new(7), SensorProfile::PrivilegedAffordanceV1);

    assert_eq!(MAX_ACTION_CANDIDATES, 32);
    assert_eq!(frame.candidates().len(), 1 + 6 * 5);
    let retained_targets = frame.candidates()[1..]
        .chunks_exact(5)
        .enumerate()
        .map(|(object_slot, family_group)| {
            assert!(family_group.iter().all(|candidate| {
                candidate.observation == CandidateObservationRef::ObjectSlot(object_slot as u16)
            }));
            family_group[0].target.entity.unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        retained_targets,
        ["id8_d0_5", "id3_d1", "id4_d1", "id5_d2", "id7_d2", "id2_d3",]
            .map(|label| world.entity_id(label).unwrap())
    );
    assert!(!retained_targets.contains(&world.entity_id("id6_d4").unwrap()));
    assert!(!retained_targets.contains(&world.entity_id("id9_d5").unwrap()));
}

#[test]
fn every_retained_object_gets_the_same_five_mechanical_families() {
    let world = HeadlessScenarioBuilder::new(13)
        .agent("agent", ORGANISM, pos(0.0, 0.0))
        .food("food", pos(1.0, 0.0), 0.5)
        .hazard("hazard", pos(0.0, 2.0), 0.5)
        .obstacle("obstacle", pos(3.0, 0.0), 0.4)
        .token("token", pos(0.0, 4.0), 77)
        .build()
        .unwrap();
    let frame = perception_frame(&world, Tick::new(11), SensorProfile::PrivilegedAffordanceV1);
    let expected = [
        (
            ActionKind::Inspect.canonical_id(),
            ActionKind::Inspect,
            CandidateActionFamily::Inspect,
        ),
        (
            HeadlessActionIds::APPROACH,
            ActionKind::Move,
            CandidateActionFamily::Approach,
        ),
        (
            HeadlessActionIds::FLEE,
            ActionKind::Move,
            CandidateActionFamily::Avoid,
        ),
        (
            HeadlessActionIds::EAT,
            ActionKind::Interact,
            CandidateActionFamily::Ingest,
        ),
        (
            HeadlessActionIds::GRAB,
            ActionKind::Interact,
            CandidateActionFamily::Contact,
        ),
    ];

    assert_eq!(frame.candidates().len(), 1 + 4 * expected.len());
    for (object_slot, family_group) in frame.candidates()[1..].chunks_exact(5).enumerate() {
        let target_entity = family_group[0].target.entity;
        assert!(target_entity.is_some());
        assert!(family_group
            .iter()
            .all(|candidate| candidate.target.entity == target_entity));
        assert!(family_group.iter().all(|candidate| {
            candidate.observation == CandidateObservationRef::ObjectSlot(object_slot as u16)
        }));
        assert_eq!(
            family_group
                .iter()
                .map(|candidate| (candidate.action_id, candidate.kind, candidate.family))
                .collect::<Vec<_>>(),
            expected
        );
    }
}

#[test]
fn semantic_relabelling_changes_features_not_candidate_transport() {
    let food = frame_world(SemanticFixtureKind::Food);
    let hazard = frame_world(SemanticFixtureKind::Hazard);
    let tick = Tick::new(17);
    let food_frame = perception_frame(&food, tick, SensorProfile::PrivilegedAffordanceV1);
    let hazard_frame = perception_frame(&hazard, tick, SensorProfile::PrivilegedAffordanceV1);

    assert_eq!(
        transport_signature(&food_frame),
        transport_signature(&hazard_frame)
    );
    assert_ne!(
        food_frame.candidates()[1].features,
        hazard_frame.candidates()[1].features
    );
    assert_ne!(food_frame.base_digest(), hazard_frame.base_digest());
    let food_features = food_frame.candidates()[1].features.0;
    let hazard_features = hazard_frame.candidates()[1].features.0;
    for lane in 0..CANDIDATE_FEATURE_COUNT {
        let may_differ = lane == CANDIDATE_FEATURE_AFFORDANCE_START_LANE
            || lane == CANDIDATE_FEATURE_AFFORDANCE_START_LANE + 2;
        if may_differ {
            assert_ne!(food_features[lane], hazard_features[lane]);
        } else {
            assert_eq!(food_features[lane], hazard_features[lane], "lane {lane}");
        }
    }
}

#[test]
fn teacher_token_adds_teacher_affordance_without_changing_candidate_transport() {
    let ordinary = HeadlessScenarioBuilder::new(101)
        .agent("agent", ORGANISM, pos(0.0, 0.0))
        .token("token", pos(3.0, 4.0), 77)
        .build()
        .unwrap();
    let teacher = HeadlessScenarioBuilder::new(101)
        .agent("agent", ORGANISM, pos(0.0, 0.0))
        .teacher_token("token", pos(3.0, 4.0), 77, TeacherPerceptionChannel::Object)
        .build()
        .unwrap();
    let tick = Tick::new(18);
    let ordinary_report = ordinary.sensory_report(ORGANISM, tick).unwrap();
    let teacher_report = teacher.sensory_report(ORGANISM, tick).unwrap();
    let ordinary_frame = perception_frame(&ordinary, tick, SensorProfile::PrivilegedAffordanceV1);
    let teacher_frame = perception_frame(&teacher, tick, SensorProfile::PrivilegedAffordanceV1);

    assert_eq!(
        transport_signature(&ordinary_frame),
        transport_signature(&teacher_frame)
    );
    assert!(ordinary_report
        .core_snapshot
        .channels
        .nearby_affordances
        .contains(AffordanceBits::GLYPH_OR_WRITING));
    assert!(!ordinary_report
        .core_snapshot
        .channels
        .nearby_affordances
        .contains(AffordanceBits::TEACHER_OBJECT));
    assert!(teacher_report
        .core_snapshot
        .channels
        .nearby_affordances
        .contains(AffordanceBits::GLYPH_OR_WRITING));
    assert!(teacher_report
        .core_snapshot
        .channels
        .nearby_affordances
        .contains(AffordanceBits::TEACHER_OBJECT));

    let glyph_lane = CANDIDATE_FEATURE_AFFORDANCE_START_LANE + 7;
    let teacher_lane = CANDIDATE_FEATURE_AFFORDANCE_START_LANE + 8;
    for candidate in &ordinary_frame.candidates()[1..6] {
        assert_eq!(candidate.features.0[glyph_lane], 1.0);
        assert_eq!(candidate.features.0[teacher_lane], 0.0);
    }
    for candidate in &teacher_frame.candidates()[1..6] {
        assert_eq!(candidate.features.0[glyph_lane], 1.0);
        assert_eq!(candidate.features.0[teacher_lane], 1.0);
    }
}

fn contact_validation_report() -> (alife_world::HeadlessSensoryReport, WorldEntityId) {
    let world = HeadlessScenarioBuilder::new(103)
        .agent("agent", ORGANISM, pos(0.0, 0.0))
        .food("near_but_not_contacting", pos(1.0, 0.0), 0.5)
        .build()
        .unwrap();
    let report = world.sensory_report(ORGANISM, Tick::new(20)).unwrap();
    let visible_id = report.visible_entities[0].id;
    assert!(report.contact_entities.is_empty());
    (report, visible_id)
}

fn assert_contacts_rejected(
    mut report: alife_world::HeadlessSensoryReport,
    contact_entities: Vec<WorldEntityId>,
) {
    report.contact_entities = contact_entities;
    assert_eq!(
        HeadlessCandidateEnumerator
            .enumerate_candidates(&report, SensorProfile::PrivilegedAffordanceV1)
            .unwrap_err(),
        ScaffoldContractError::InvalidPerceptionFrame
    );
}

#[test]
fn candidate_enumerator_rejects_invalid_contact_entity_id() {
    let (report, _) = contact_validation_report();
    assert_contacts_rejected(report, vec![WorldEntityId(0)]);
}

#[test]
fn candidate_enumerator_rejects_duplicate_contact_entity_ids() {
    let (report, visible_id) = contact_validation_report();
    assert_contacts_rejected(report, vec![visible_id, visible_id]);
}

#[test]
fn candidate_enumerator_rejects_non_visible_contact_entity_id() {
    let (report, _) = contact_validation_report();
    assert_contacts_rejected(report, vec![WorldEntityId(999)]);
}

#[test]
fn candidate_enumerator_rejects_visible_entity_beyond_contact_radius() {
    let (report, visible_id) = contact_validation_report();
    assert_contacts_rejected(report, vec![visible_id]);
}

#[test]
fn candidate_enumerator_rejects_missing_genuine_visible_contact() {
    let world = HeadlessScenarioBuilder::new(107)
        .agent("agent", ORGANISM, pos(0.0, 0.0))
        .food("contacting", pos(0.5, 0.0), 0.5)
        .build()
        .unwrap();
    let mut report = world.sensory_report(ORGANISM, Tick::new(21)).unwrap();
    assert_eq!(report.visible_entities.len(), 1);
    assert_eq!(report.contact_entities, vec![report.visible_entities[0].id]);

    report.contact_entities.clear();
    assert_eq!(
        HeadlessCandidateEnumerator
            .enumerate_candidates(&report, SensorProfile::PrivilegedAffordanceV1)
            .unwrap_err(),
        ScaffoldContractError::InvalidPerceptionFrame
    );
}

#[test]
fn grounded_profile_fails_closed_until_slice_c_provides_grounded_features() {
    let world = frame_world(SemanticFixtureKind::Food);
    let tick = Tick::new(19);
    let report = world.sensory_report(ORGANISM, tick).unwrap();

    assert_eq!(
        HeadlessCandidateEnumerator
            .enumerate_candidates(&report, SensorProfile::GroundedObjectSlotsV1)
            .unwrap_err(),
        ScaffoldContractError::SensorProfileMismatch
    );
    assert_eq!(
        world
            .perception_frame(
                ORGANISM,
                tick,
                SensorProfile::GroundedObjectSlotsV1,
                HomeostaticSnapshot::baseline(tick),
            )
            .unwrap_err(),
        ScaffoldContractError::SensorProfileMismatch
    );
}

#[test]
fn same_seed_snapshot_and_inputs_replay_identical_frames() {
    let first = frame_world(SemanticFixtureKind::Food);
    let second = frame_world(SemanticFixtureKind::Food);
    let tick = Tick::new(23);

    assert_eq!(
        perception_frame(&first, tick, SensorProfile::PrivilegedAffordanceV1),
        perception_frame(&second, tick, SensorProfile::PrivilegedAffordanceV1)
    );
}

#[test]
fn invalid_organism_and_mismatched_homeostasis_tick_are_rejected() {
    let world = frame_world(SemanticFixtureKind::Food);
    let tick = Tick::new(29);

    assert_eq!(
        world
            .perception_frame(
                OrganismId(999),
                tick,
                SensorProfile::PrivilegedAffordanceV1,
                HomeostaticSnapshot::baseline(tick),
            )
            .unwrap_err(),
        ScaffoldContractError::InvalidId
    );
    assert_eq!(
        world
            .perception_frame(
                ORGANISM,
                tick,
                SensorProfile::PrivilegedAffordanceV1,
                HomeostaticSnapshot::baseline(Tick::new(30)),
            )
            .unwrap_err(),
        ScaffoldContractError::InvalidPerceptionFrame
    );
}
