use alife_core::{LanguageTokenId, OrganismId, UtteranceSourceKind, Vec3f};
use alife_school::{LanguageNursery, LanguageNurseryLesson, NurseryDemonstration, NurserySpeaker};
use alife_world::WorldObjectKind;

#[test]
fn nursery_exposure_is_spatial_perception_with_visible_object_and_no_authority_bypass() {
    let subject = OrganismId(7);
    let mut nursery = LanguageNursery::new(4404, subject).unwrap();
    let lesson = LanguageNurseryLesson::try_new(
        LanguageTokenId::new(41).unwrap(),
        "striped-fruit",
        WorldObjectKind::Food,
        Vec3f::new(2.0, 0.0, 0.0),
        NurseryDemonstration::Approach,
    )
    .unwrap();

    let exposure = nursery
        .present(
            NurserySpeaker::Teacher {
                source_position: Vec3f::new(0.5, 0.0, 0.0),
            },
            &lesson,
        )
        .unwrap();

    assert_eq!(exposure.subject, subject);
    assert_eq!(exposure.utterance.source_kind, UtteranceSourceKind::Teacher);
    assert_eq!(exposure.utterance.addressee, Some(subject));
    assert_eq!(exposure.utterance.tokens, vec![lesson.token]);
    assert!(!exposure.perception.grounded_object_slots().is_empty());
    assert!(exposure
        .perception
        .sensory()
        .language_context
        .heard_tokens
        .iter()
        .flatten()
        .any(|heard| {
            heard.utterance_id == exposure.utterance.utterance_id
                && heard.token_id == u32::from(lesson.token.raw())
                && heard.source_kind == UtteranceSourceKind::Teacher
        }));
    assert!(!exposure.can_issue_actions);
    assert!(!exposure.can_write_rewards);
    assert!(!exposure.can_inject_hidden_concepts);
}

#[test]
fn player_teacher_and_peer_use_the_same_nursery_perception_path() {
    let lesson = LanguageNurseryLesson::try_new(
        LanguageTokenId::new(42).unwrap(),
        "blue-object",
        WorldObjectKind::Token,
        Vec3f::new(2.0, 0.0, 0.0),
        NurseryDemonstration::Inspect,
    )
    .unwrap();
    for (speaker, expected_source) in [
        (
            NurserySpeaker::Player {
                source_position: Vec3f::new(0.5, 0.0, 0.0),
            },
            UtteranceSourceKind::Player,
        ),
        (
            NurserySpeaker::Teacher {
                source_position: Vec3f::new(0.5, 0.0, 0.0),
            },
            UtteranceSourceKind::Teacher,
        ),
        (
            NurserySpeaker::Peer {
                organism_id: OrganismId(8),
                source_position: Vec3f::new(0.5, 0.0, 0.0),
            },
            UtteranceSourceKind::Creature,
        ),
    ] {
        let mut nursery = LanguageNursery::new(4405, OrganismId(7)).unwrap();
        let exposure = nursery.present(speaker, &lesson).unwrap();
        assert_eq!(exposure.utterance.source_kind, expected_source);
        assert!(exposure
            .perception
            .sensory()
            .language_context
            .heard_tokens
            .iter()
            .flatten()
            .any(|heard| heard.source_kind == expected_source));
        assert_eq!(exposure.utterance.tokens, vec![lesson.token]);
        assert!(!exposure.perception.grounded_object_slots().is_empty());
    }
}
