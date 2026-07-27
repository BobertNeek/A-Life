use alife_core::{
    BrainScaleTier, Confidence, LanguageTokenId, OrganismId, PlayerUtterance, SpeechActKind,
    SpeechMotorPayload, UtteranceId, UtteranceSourceKind, Vec3f,
};
use alife_world::{
    AssetManifest, HeadlessScenarioBuilder, HeadlessWorldCommand, PortableSaveFile, RuntimeConfig,
};

fn token(raw: u16) -> LanguageTokenId {
    LanguageTokenId::new(raw).unwrap()
}

#[test]
fn named_player_speech_reaches_only_the_named_creature() {
    let addressed = OrganismId(1);
    let bystander = OrganismId(2);
    let mut world = HeadlessScenarioBuilder::new(71_001)
        .agent("addressed", addressed, Vec3f::ZERO)
        .agent("bystander", bystander, Vec3f::new(2.0, 0.0, 0.0))
        .build()
        .unwrap();
    world
        .emit_player_utterance(
            PlayerUtterance::try_new(
                UtteranceId::new(1).unwrap(),
                Some(addressed),
                Vec3f::new(1.0, 0.0, 0.0),
                vec![token(1), token(25)],
            )
            .unwrap(),
        )
        .unwrap();

    let addressed_tokens = world
        .sensory_report(addressed, world.tick())
        .unwrap()
        .core_snapshot
        .language_context
        .heard_tokens
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    assert_eq!(
        addressed_tokens
            .iter()
            .map(|heard| heard.token_id)
            .collect::<Vec<_>>(),
        vec![1, 25]
    );
    assert!(addressed_tokens.iter().all(|heard| {
        heard.utterance_id == UtteranceId::new(1).unwrap()
            && heard.source_kind == UtteranceSourceKind::Player
            && heard.addressee == Some(addressed)
    }));
    assert!(world
        .sensory_report(bystander, world.tick())
        .unwrap()
        .core_snapshot
        .language_context
        .heard_tokens
        .iter()
        .all(Option::is_none));
}

#[test]
fn world_allocates_collision_free_player_utterance_ids() {
    let listener = OrganismId(1);
    let mut world = HeadlessScenarioBuilder::new(71_005)
        .agent("listener", listener, Vec3f::ZERO)
        .build()
        .unwrap();
    let first = world
        .emit_player_tokens(None, Vec3f::ZERO, vec![token(1)])
        .unwrap();
    let second = world
        .emit_player_tokens(Some(listener), Vec3f::ZERO, vec![token(2)])
        .unwrap();
    assert_eq!(first.utterance_id.raw() + 1, second.utterance_id.raw());
    assert_eq!(world.audible_utterances(), vec![first, second]);
}

#[test]
fn broadcast_range_and_creature_raw_token_identity_are_preserved() {
    let speaker = OrganismId(1);
    let near = OrganismId(2);
    let far = OrganismId(3);
    let mut world = HeadlessScenarioBuilder::new(71_002)
        .agent("speaker", speaker, Vec3f::ZERO)
        .agent("near", near, Vec3f::new(2.0, 0.0, 0.0))
        .agent("far", far, Vec3f::new(20.0, 0.0, 0.0))
        .build()
        .unwrap();
    let payload = SpeechMotorPayload::try_new(
        SpeechActKind::ExpressState,
        vec![token(89), token(105)],
        Confidence::new(0.9).unwrap(),
    )
    .unwrap();
    world
        .emit_creature_utterance(UtteranceId::new(2).unwrap(), speaker, None, payload)
        .unwrap();

    let near_tokens = world
        .sensory_report(near, world.tick())
        .unwrap()
        .core_snapshot
        .language_context
        .heard_tokens
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    assert_eq!(
        near_tokens
            .iter()
            .map(|heard| heard.token_id)
            .collect::<Vec<_>>(),
        vec![89, 105]
    );
    assert!(near_tokens.iter().all(|heard| {
        heard.speaker_id == Some(speaker)
            && heard.source_kind == UtteranceSourceKind::Creature
            && heard.utterance_id == UtteranceId::new(2).unwrap()
    }));
    assert!(world
        .sensory_report(speaker, world.tick())
        .unwrap()
        .core_snapshot
        .language_context
        .heard_tokens
        .iter()
        .all(Option::is_none));
    assert!(world
        .sensory_report(far, world.tick())
        .unwrap()
        .core_snapshot
        .language_context
        .heard_tokens
        .iter()
        .all(Option::is_none));
}

#[test]
fn neural_vocalize_requires_gpu_payload_and_enforces_prompted_and_spontaneous_cooldowns() {
    let speaker = OrganismId(1);
    let listener = OrganismId(2);
    let mut world = HeadlessScenarioBuilder::new(71_003)
        .agent("speaker", speaker, Vec3f::ZERO)
        .agent("listener", listener, Vec3f::new(1.0, 0.0, 0.0))
        .build()
        .unwrap();
    let command = HeadlessWorldCommand::vocalize(speaker).unwrap();
    let payload = SpeechMotorPayload::try_new(
        SpeechActKind::Declare,
        vec![token(3), token(33), token(106)],
        Confidence::new(0.8).unwrap(),
    )
    .unwrap();

    let first = world
        .apply_neural_command(&command, Some(payload.clone()), false)
        .unwrap();
    assert!(first.execution.succeeded);
    assert!(first.observation.energy_delta.raw() < 0.0);
    assert_eq!(
        first
            .emitted_utterance
            .as_ref()
            .unwrap()
            .tokens
            .iter()
            .map(|token| token.raw())
            .collect::<Vec<_>>(),
        vec![3, 33, 106]
    );

    let cooldown = world
        .apply_neural_command(&command, Some(payload.clone()), false)
        .unwrap();
    assert!(!cooldown.execution.succeeded);
    assert!(cooldown.emitted_utterance.is_none());
    for _ in 0..8 {
        world.advance_tick();
    }
    let prompted = world
        .apply_neural_command(&command, Some(payload), true)
        .unwrap();
    assert!(prompted.execution.succeeded);

    let absent = world.apply_neural_command(&command, None, true).unwrap();
    assert!(!absent.execution.succeeded);
    assert!(absent.emitted_utterance.is_none());
}

#[test]
fn active_spatial_speech_and_cooldown_roundtrip_with_the_world_save() {
    let speaker = OrganismId(1);
    let listener = OrganismId(2);
    let mut world = HeadlessScenarioBuilder::new(71_004)
        .agent("speaker", speaker, Vec3f::ZERO)
        .agent("listener", listener, Vec3f::new(1.0, 0.0, 0.0))
        .build()
        .unwrap();
    let command = HeadlessWorldCommand::vocalize(speaker).unwrap();
    let payload = SpeechMotorPayload::try_new(
        SpeechActKind::ExpressState,
        vec![token(91), token(7)],
        Confidence::new(0.9).unwrap(),
    )
    .unwrap();
    world
        .apply_neural_command(&command, Some(payload.clone()), false)
        .unwrap();
    let save = PortableSaveFile::from_headless_world(
        "speech-roundtrip",
        &world,
        RuntimeConfig::deterministic_default(71_004, BrainScaleTier::Nano512),
        AssetManifest::empty(),
        Vec::new(),
    )
    .unwrap();
    let decoded = PortableSaveFile::from_json_str(&save.to_json_string_pretty().unwrap()).unwrap();
    let mut restored = decoded.restore_headless_world().unwrap();

    let heard = restored
        .sensory_report(listener, restored.tick())
        .unwrap()
        .core_snapshot
        .language_context
        .heard_tokens
        .into_iter()
        .flatten()
        .map(|heard| heard.token_id)
        .collect::<Vec<_>>();
    assert_eq!(heard, vec![91, 7]);
    assert!(
        !restored
            .apply_neural_command(&command, Some(payload), false)
            .unwrap()
            .execution
            .succeeded
    );
}
