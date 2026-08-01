use alife_core::{
    BrainCapacityClass, BrainGenome, CandidateActionFamily, Confidence, DecisionSnapshot,
    DevelopmentState, ExperiencePatch, ExperiencePatchBuilder, ExperienceSequenceId,
    LanguageGroundingLedger, LanguageTokenId, LobeKind, NeuralActionSelection, NormalizedScalar,
    OrganismId, PerceptionContextBlock, PhenotypeHash, PostActionOutcome, PreActionSnapshot,
    ScaffoldContractError, SensorProfile, SpeechActKind, SpeechMotorPayload, Tick,
    UtteranceGroundingReceiptV2, UtteranceId, UtteranceSourceKind, Vec3f, WorldEntityId,
};
use alife_world::HeadlessScenarioBuilder;

const SUBJECT: OrganismId = OrganismId(501);
const PEER: OrganismId = OrganismId(502);

fn grounded_patch(source: UtteranceSourceKind) -> (ExperiencePatch, UtteranceId, WorldEntityId) {
    let mut world = HeadlessScenarioBuilder::new(77_001)
        .agent("subject", SUBJECT, Vec3f::ZERO)
        .social_agent("peer", PEER, Vec3f::new(0.25, 0.0, 0.0), 0.6)
        .food("fruit", Vec3f::new(0.75, 0.0, 0.0), 0.8)
        .build()
        .unwrap();
    let target = world.entity_id("fruit").unwrap();
    let utterance = match source {
        UtteranceSourceKind::Creature => world
            .emit_creature_utterance(
                UtteranceId::new(1).unwrap(),
                PEER,
                Some(SUBJECT),
                SpeechMotorPayload::try_new(
                    SpeechActKind::Declare,
                    vec![LanguageTokenId::new(41).unwrap()],
                    Confidence::new(1.0).unwrap(),
                )
                .unwrap(),
            )
            .unwrap(),
        UtteranceSourceKind::Player => world
            .emit_player_tokens(
                Some(SUBJECT),
                Vec3f::new(0.25, 0.0, 0.0),
                vec![LanguageTokenId::new(41).unwrap()],
            )
            .unwrap(),
        UtteranceSourceKind::Teacher => unreachable!(),
    };
    let draft = world
        .perception_frame_draft(
            SUBJECT,
            Tick::new(0),
            SensorProfile::GroundedObjectSlotsV1,
            alife_core::HomeostaticSnapshot::baseline(Tick::new(0)),
        )
        .unwrap();
    let frame = draft.finalize(PerceptionContextBlock::empty()).unwrap();
    let candidate = *frame
        .candidates()
        .iter()
        .find(|candidate| {
            candidate.family == CandidateActionFamily::Ingest
                && candidate.target.entity == Some(target)
        })
        .unwrap();
    let sequence = ExperienceSequenceId(1);
    let phenotype = PhenotypeHash([11, 12, 13, 14]);
    let decision = DecisionSnapshot::from_neural_selection(
        sequence,
        phenotype,
        1,
        0,
        &frame,
        NeuralActionSelection {
            candidate_index: candidate.candidate_index,
            logit: 0.9,
            confidence: Confidence::new(0.9).unwrap(),
            active_tiles: 8,
            active_synapses: 64,
        },
        candidate
            .to_command(SUBJECT, Confidence::new(0.9).unwrap())
            .unwrap(),
    )
    .unwrap();
    let genome = BrainGenome::scaffold(9101, BrainCapacityClass::N2048_ID);
    let development = DevelopmentState::new(
        genome.id,
        Tick::new(20),
        NormalizedScalar::new(1.0).unwrap(),
    )
    .with_enabled_lobes([
        LobeKind::SensoryGrounding,
        LobeKind::CoreAssociation,
        LobeKind::MotorArbitration,
    ]);
    let pre_action = PreActionSnapshot::from_neural_frame(
        sequence,
        BrainCapacityClass::N2048_ID,
        phenotype,
        genome.id,
        genome.schema_version,
        development,
        frame,
    )
    .unwrap();
    let action = world.apply_command(&decision.selected_action).unwrap();
    let mut outcome = PostActionOutcome::new(
        SUBJECT,
        sequence,
        Tick::new(1),
        action.observation.success && action.execution.succeeded,
        action.execution.physical,
        action.observation.homeostatic_delta,
        action.observation.reward_valence,
        action.observation.frustration_delta,
        action.observation.pain_delta,
        action.observation.energy_delta,
        action.observation.prediction_error,
    )
    .unwrap();
    outcome.contradiction_observed = action.observation.contradiction_observed;
    let patch = ExperiencePatchBuilder::new(sequence)
        .record_pre_action(pre_action)
        .unwrap()
        .record_decision(decision)
        .unwrap()
        .record_outcome(outcome)
        .unwrap()
        .seal()
        .unwrap();
    (patch, utterance.utterance_id, target)
}

#[test]
fn exact_utterance_grounding_requires_matching_neural_target_and_sealed_success() {
    let (patch, utterance, target) = grounded_patch(UtteranceSourceKind::Creature);
    let receipt =
        UtteranceGroundingReceiptV2::try_from_sealed(&patch, utterance, 0, target).unwrap();
    receipt.validate_contract().unwrap();
    assert_eq!(receipt.speaker_id, Some(PEER));
    assert_eq!(receipt.target_entity, target);
    assert_ne!(receipt.tracked_target.raw(), 0);

    assert!(
        UtteranceGroundingReceiptV2::try_from_sealed(&patch, utterance, 0, WorldEntityId(999))
            .is_err()
    );
    assert!(UtteranceGroundingReceiptV2::try_from_sealed(
        &patch,
        UtteranceId::new(999).unwrap(),
        0,
        target
    )
    .is_err());

    let mut ledger = LanguageGroundingLedger::default();
    ledger.observe_grounding_v2(receipt.clone()).unwrap();
    assert_eq!(ledger.utterance_receipts_v2(), &[receipt.clone()]);
    assert_eq!(
        ledger.observe_grounding_v2(receipt).unwrap_err(),
        ScaffoldContractError::LearningReplayRejected
    );
}

#[test]
fn host_authored_speech_cannot_become_grounding_evidence() {
    let (patch, utterance, target) = grounded_patch(UtteranceSourceKind::Player);
    assert!(UtteranceGroundingReceiptV2::try_from_sealed(&patch, utterance, 0, target).is_err());
}

#[cfg(feature = "gpu-tests")]
#[test]
fn gpu_episode_reports_honest_phase_learning_and_grounding_evidence() {
    use alife_core::{
        BrainCapacityClass, CreatureGenome, Era1Ability, Era1Control, Era1EvidencePartition,
        FoundationGeneticIdentity, MetricReading,
    };
    use alife_training::{Era1TrialRunRequest, Era1TrialRunner};
    use alife_world::{Era1TrialManifest, Era1WorldFamily};

    let foundation = FoundationGeneticIdentity::new(
        0x4E32_3034_385F_5631,
        1,
        0x4E32_3034_385F_FA11,
        BrainCapacityClass::N2048_ID,
    )
    .unwrap();
    let genome = CreatureGenome::early_mammal_founder(0xE11_041, foundation).unwrap();
    let manifest = Era1TrialManifest::new(
        55_041,
        Era1WorldFamily::GroundedVocabulary,
        SUBJECT,
        PEER,
        OrganismId(503),
        1,
        false,
        41,
    )
    .unwrap();
    let request = Era1TrialRunRequest::new(
        SUBJECT,
        0,
        &genome,
        &manifest,
        Era1Ability::GroundedLanguage,
        Era1Control::Intact,
        Era1EvidencePartition::Acquisition,
        "0123456789abcdef0123456789abcdef01234567",
        "89abcdef0123456789abcdef0123456789abcdef",
    )
    .unwrap();
    let mut runner = Era1TrialRunner::new_required().unwrap();
    let evidence = runner.run(request).unwrap();
    evidence.validate_contract().unwrap();

    for reading in [
        evidence.learning_assessment.early_acquisition,
        evidence.learning_assessment.late_acquisition,
        evidence.learning_assessment.delay,
        evidence.learning_assessment.probe,
    ] {
        assert!(matches!(reading, MetricReading::Measured { exposures, .. } if exposures > 0));
    }
    assert_eq!(
        evidence.receipt.score,
        evidence.learning_assessment.late_acquisition
    );
    assert!(evidence
        .learning_assessment
        .grounding_receipts
        .iter()
        .all(|receipt| receipt.source_kind != UtteranceSourceKind::Player));
    if evidence.learning_assessment.demonstrated {
        assert!(evidence.learning_assessment.probe_change_from_early_q16 > 0);
        assert!(!evidence.learning_assessment.grounding_receipts.is_empty());
    }

    let mut relabelled = evidence.clone();
    relabelled.steps[0].behavior_success = !relabelled.steps[0].behavior_success;
    assert!(relabelled.validate_contract().is_err());
}
