use alife_core::{
    ExperienceSequenceId, LanguageTokenId, OrganismId, SpeechTranslationInput,
    SpeechTranslationRequest, SurfaceTokenBinding, Tick, UtteranceId,
};
use alife_semantic::{
    BoundedSpeechTranslator, DevelopmentalPriorController, LanguageEvaluationScores,
    TranslationAssistance,
};

#[test]
fn developmental_prior_fades_after_three_passing_unaided_probes() {
    let mut controller = DevelopmentalPriorController::default();

    for exposure in 1..=192 {
        controller.record_relevant_exposure(false);
        if exposure % 64 == 0 {
            assert!(controller.probe_due());
            controller.record_unaided_probe(0.80).unwrap();
        }
    }

    assert_eq!(controller.unaided_exposures(), 192);
    assert_eq!(controller.consecutive_passing_probes(), 3);
    assert_eq!(controller.developmental_gain(), 0.0);

    let packet = controller
        .issue_packet(
            alife_core::SemanticPriorRequest::new(OrganismId(7), ExperienceSequenceId(11)).unwrap(),
            Tick(300),
            vec![4, 9],
            false,
        )
        .unwrap();
    assert_eq!(packet.plasticity_modulation, 0.0);
    assert_eq!(
        packet.expires_at_tick.raw() - packet.issued_at_tick.raw(),
        32
    );
}

#[test]
fn novelty_reactivation_is_bounded_and_obeys_cooldown() {
    let mut controller = DevelopmentalPriorController::default();
    for _ in 0..192 {
        controller.record_relevant_exposure(false);
    }
    for _ in 0..3 {
        controller.record_unaided_probe(0.80).unwrap();
    }

    let request =
        alife_core::SemanticPriorRequest::new(OrganismId(7), ExperienceSequenceId(11)).unwrap();
    let first = controller
        .issue_packet(request, Tick(2_000), vec![3], true)
        .unwrap();
    assert_eq!(first.plasticity_modulation, 0.05);
    assert_eq!(controller.active_reactivation_until(), Some(Tick(2_128)));

    let boundary = controller
        .issue_packet(request, Tick(2_128), vec![3], false)
        .unwrap();
    assert_eq!(boundary.plasticity_modulation, 0.0);

    let expired = controller
        .issue_packet(request, Tick(2_129), vec![3], false)
        .unwrap();
    assert_eq!(expired.plasticity_modulation, 0.0);
    let cooling_down = controller
        .issue_packet(request, Tick(2_500), vec![3], true)
        .unwrap();
    assert_eq!(cooling_down.plasticity_modulation, 0.0);
    let reactivated = controller
        .issue_packet(request, Tick(3_024), vec![3], true)
        .unwrap();
    assert_eq!(reactivated.plasticity_modulation, 0.05);
}

#[test]
fn player_translation_is_bounded_preserves_addressee_and_marks_unknowns() {
    let known = SurfaceTokenBinding::try_new("come", LanguageTokenId::new(1).unwrap()).unwrap();
    let request = SpeechTranslationRequest::try_new(
        UtteranceId::new(99).unwrap(),
        Some(OrganismId(42)),
        SpeechTranslationInput::PlayerText {
            text: "Come glimmer glimmer beyond extra words are deliberately supplied to exceed the bounded sixteen token hearing interface one two three four five six".to_string(),
        },
        vec![known],
    )
    .unwrap();
    let translator =
        BoundedSpeechTranslator::new("local-test-model", TranslationAssistance::SlmAssisted)
            .unwrap();
    let receipt = translator.translate(&request).unwrap();

    assert_eq!(receipt.addressee, Some(OrganismId(42)));
    assert_eq!(receipt.literal_tokens.len(), 16);
    assert_eq!(receipt.literal_tokens[0], LanguageTokenId::new(1).unwrap());
    assert_eq!(receipt.literal_tokens[1], receipt.literal_tokens[2]);
    assert_eq!(
        receipt
            .novel_tokens
            .iter()
            .filter(|token| token.surface == "glimmer")
            .count(),
        1
    );
    assert!(receipt
        .novel_tokens
        .iter()
        .any(|token| token.surface == "glimmer"));
    assert!(receipt.assisted);
    assert_eq!(receipt.model_identity, "local-test-model");
}

#[test]
fn creature_translation_renders_only_supplied_raw_tokens_and_exposes_uncertainty() {
    let request = SpeechTranslationRequest::try_new(
        UtteranceId::new(100).unwrap(),
        None,
        SpeechTranslationInput::CreatureTokens {
            tokens: vec![
                LanguageTokenId::new(8).unwrap(),
                LanguageTokenId::new(77).unwrap(),
            ],
        },
        vec![SurfaceTokenBinding::try_new("rest", LanguageTokenId::new(8).unwrap()).unwrap()],
    )
    .unwrap();
    let translator =
        BoundedSpeechTranslator::new("local-test-model", TranslationAssistance::SlmAssisted)
            .unwrap();
    let receipt = translator.translate(&request).unwrap();

    assert_eq!(receipt.literal_tokens, request.raw_tokens());
    assert!(receipt.uncertain);
    assert!(receipt.rendered_text.starts_with("[uncertain] rest "));
    assert!(!receipt.rendered_text.contains("eat"));
}

#[test]
fn unaided_and_assisted_language_scores_never_mix() {
    let mut scores = LanguageEvaluationScores::default();
    scores.record(false, true);
    scores.record(false, false);
    scores.record(true, true);

    assert_eq!(scores.unaided_trials, 2);
    assert_eq!(scores.unaided_successes, 1);
    assert_eq!(scores.assisted_trials, 1);
    assert_eq!(scores.assisted_successes, 1);
}
