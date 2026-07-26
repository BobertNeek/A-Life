use alife_core::{
    Confidence, LanguageTokenId, OrganismId, PlayerUtterance, SpeechActKind, SpeechMotorPayload,
    UtteranceId, UtteranceSourceKind, Validate, Vec3f,
};

#[test]
fn player_utterance_is_bounded_spatial_and_optionally_addressed() {
    let utterance = PlayerUtterance::try_new(
        UtteranceId::new(7).unwrap(),
        Some(OrganismId(3)),
        Vec3f::new(1.0, 2.0, 0.0),
        vec![LanguageTokenId::new(1).unwrap(); 16],
    )
    .unwrap();
    utterance.validate_contract().unwrap();
    assert_eq!(utterance.source_kind, UtteranceSourceKind::Player);
    assert_eq!(utterance.addressee, Some(OrganismId(3)));
    assert!(PlayerUtterance::try_new(
        UtteranceId::new(8).unwrap(),
        None,
        Vec3f::ZERO,
        vec![LanguageTokenId::new(1).unwrap(); 17],
    )
    .is_err());
}

#[test]
fn neural_speech_payload_is_bounded_to_six_non_silence_tokens() {
    let payload = SpeechMotorPayload::try_new(
        SpeechActKind::Declare,
        vec![LanguageTokenId::new(25).unwrap(); 6],
        Confidence::new(0.75).unwrap(),
    )
    .unwrap();
    payload.validate_contract().unwrap();
    assert!(SpeechMotorPayload::try_new(
        SpeechActKind::Declare,
        vec![LanguageTokenId::new(25).unwrap(); 7],
        Confidence::new(0.75).unwrap(),
    )
    .is_err());
    assert!(SpeechMotorPayload::try_new(
        SpeechActKind::Declare,
        vec![LanguageTokenId::new(0).unwrap()],
        Confidence::new(0.75).unwrap(),
    )
    .is_err());
}
