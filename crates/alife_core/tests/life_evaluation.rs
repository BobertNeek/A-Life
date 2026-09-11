use alife_core::{
    heuristic_baseline_arbitrate, ActionArbitrationConfig, ActionCandidate, ActionId, ActionKind,
    ActionProposal, ActionTarget, ActiveChallengeKind, BodySnapshot, BrainClassSpec, BrainGenome,
    BrainScaleTier, CandidateActionFamily, CandidateFeatureVector, CandidateObservationRef,
    Confidence, DevelopmentState, DurationTicks, EnvironmentalRegime, ExperiencePatch,
    ExperiencePatchBuilder, ExperienceSequenceId, HeardToken, HomeostaticDelta,
    HomeostaticSnapshot, LobeKind, MetricReading, NormalizedScalar, OrganismId, PassiveLifeEvent,
    PassiveLifeStatistics, PassiveMetricKind, PerceptionFrame, PhysicalActionOutcome,
    PhysicalContactKind, Pose, PostActionOutcome, SensorProfile, SensorProfileProvenance,
    SensoryAbiVersion, SensoryChannels, SensorySnapshot, SignedValence, Tick, UtteranceId,
    UtteranceSourceKind, Velocity, WeightSplitContract, WorldEntityId, ACTIVE_CHALLENGE_COUNT,
};

fn finalized_patch(
    contact: PhysicalContactKind,
    reward: f32,
    pain: f32,
    action_kind: ActionKind,
) -> ExperiencePatch {
    let organism_id = OrganismId(7);
    let sequence_id = ExperienceSequenceId(99);
    let tick = Tick::new(10);
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Standard2048);
    let genome = BrainGenome::scaffold(42, spec.id);
    let (action_id, candidate_family, target) = match action_kind {
        ActionKind::Vocalize => (
            ActionKind::Vocalize.canonical_id(),
            CandidateActionFamily::Other,
            ActionTarget::NONE,
        ),
        _ => (
            ActionId(300),
            CandidateActionFamily::Contact,
            ActionTarget::new(
                Some(WorldEntityId(1)),
                Some(alife_core::Vec3f::new(0.0, 0.0, 1.0)),
            ),
        ),
    };
    let development = DevelopmentState::new(
        genome.id,
        Tick::new(120),
        NormalizedScalar::new(0.35).unwrap(),
    )
    .with_enabled_lobes([
        LobeKind::PerceptualIntegration,
        LobeKind::TemporalPredictive,
        LobeKind::ActionPlanning,
    ]);
    let weight_split = WeightSplitContract::for_brain_class(
        spec.id,
        spec.max_active_synapses,
        spec.max_active_microtiles,
        genome.genetic_prior_seed,
    )
    .unwrap();
    let mut sensory = SensorySnapshot::new(
        organism_id,
        tick,
        alife_core::Vec3f::ZERO,
        SensoryChannels::default(),
        Default::default(),
    )
    .unwrap();
    sensory.language_context.heard_tokens[0] = Some(HeardToken {
        utterance_id: UtteranceId::new(70).unwrap(),
        sequence_position: 0,
        source_kind: UtteranceSourceKind::Creature,
        speaker_id: Some(OrganismId(8)),
        addressee: None,
        source_entity: Some(WorldEntityId(70)),
        token_id: 101,
        source_position: alife_core::Vec3f::new(0.5, 0.0, 1.0),
        confidence: Confidence::new(0.8).unwrap(),
        teacher_channel: None,
    });
    let perception = PerceptionFrame::new(
        organism_id,
        tick,
        SensorProfile::PrivilegedAffordanceV1,
        sensory,
        BodySnapshot {
            pose: Pose::IDENTITY,
            velocity: Velocity::ZERO,
        },
        HomeostaticSnapshot::baseline(tick),
        vec![ActionCandidate::new(
            0,
            action_id,
            action_kind,
            candidate_family,
            CandidateObservationRef::None,
            target,
            CandidateFeatureVector::zero(),
            Confidence::new(0.8).unwrap(),
            NormalizedScalar::new(0.0).unwrap(),
            DurationTicks::new(4),
            DurationTicks::new(4),
        )
        .unwrap()],
        SensorProfileProvenance::new(
            SensorProfile::PrivilegedAffordanceV1,
            SensoryAbiVersion::CURRENT,
            tick,
        )
        .unwrap(),
        Vec::new(),
    )
    .unwrap();
    let pre_action = alife_core::PreActionSnapshot::from_heuristic_frame(
        sequence_id,
        perception,
        spec.clone(),
        genome.clone(),
        development,
        weight_split,
        alife_core::MemoryExpectancySnapshot::neutral(),
    )
    .unwrap();
    let proposal = ActionProposal::new(
        action_id,
        action_kind,
        0.75,
        Confidence::new(0.8).unwrap(),
        None,
        0b101,
        target,
        NormalizedScalar::new(0.5).unwrap(),
    )
    .unwrap();
    let action_decision = heuristic_baseline_arbitrate(
        organism_id,
        std::slice::from_ref(&proposal),
        ActionArbitrationConfig {
            default_duration_ticks: DurationTicks::new(4),
            ..ActionArbitrationConfig::default()
        },
    )
    .unwrap();
    let decision = alife_core::DecisionSnapshot::from_action_decision(
        sequence_id,
        tick,
        vec![proposal],
        action_decision,
    )
    .unwrap();
    let outcome = PostActionOutcome::new(
        organism_id,
        sequence_id,
        Tick::new(11),
        true,
        PhysicalActionOutcome {
            contact,
            target_entity: Some(WorldEntityId(1)),
            displacement: alife_core::Vec3f::ZERO,
            collision_normal: None,
            energy_cost: NormalizedScalar::new(0.1).unwrap(),
        },
        HomeostaticDelta::zero(),
        SignedValence::new(reward).unwrap(),
        NormalizedScalar::new(0.0).unwrap(),
        NormalizedScalar::new(pain).unwrap(),
        SignedValence::ZERO,
        NormalizedScalar::new(0.0).unwrap(),
    )
    .unwrap();
    ExperiencePatchBuilder::new(sequence_id)
        .record_pre_action(pre_action)
        .unwrap()
        .record_decision(decision)
        .unwrap()
        .record_outcome(outcome)
        .unwrap()
        .seal()
        .unwrap()
}

#[test]
fn unexposed_passive_metrics_are_unknown_and_updates_are_constant_state() {
    let mut statistics = PassiveLifeStatistics::new(OrganismId(7), Tick(10)).unwrap();
    assert_eq!(
        statistics.metric(PassiveMetricKind::FoodSuccess),
        MetricReading::Unknown
    );
    assert_eq!(
        statistics.metric(PassiveMetricKind::UnaidedComprehension),
        MetricReading::Unknown
    );

    statistics
        .observe(PassiveLifeEvent::SurvivalTick {
            tick: Tick(11),
            regime: EnvironmentalRegime::Temperate,
            energy_q16: 49_152,
            movement_distance_q16: 32_768,
            gpu_dispatched: true,
            gpu_throttled: false,
        })
        .unwrap();
    statistics
        .observe(PassiveLifeEvent::FoodOutcome { beneficial: true })
        .unwrap();
    statistics
        .observe(PassiveLifeEvent::Comprehension {
            assisted: false,
            correct: true,
        })
        .unwrap();

    assert_eq!(statistics.survival_ticks(), 1);
    assert_eq!(statistics.environmental_regime_ticks()[0], 1);
    assert_eq!(
        statistics.metric(PassiveMetricKind::FoodSuccess),
        MetricReading::Measured {
            value_q16: 65_535,
            exposures: 1,
        }
    );
    assert_eq!(statistics.gpu_dispatches(), 1);
    assert_eq!(statistics.gpu_throttled_dispatches(), 0);
    statistics.finalize(Tick(12), "starvation").unwrap();
    statistics.validate_contract().unwrap();

    let encoded = serde_json::to_vec(&statistics).unwrap();
    let restored: PassiveLifeStatistics = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(restored, statistics);
}

#[test]
fn active_battery_contains_the_exact_fifteen_challenges() {
    assert_eq!(ACTIVE_CHALLENGE_COUNT, 15);
    assert_eq!(ActiveChallengeKind::ALL.len(), ACTIVE_CHALLENGE_COUNT);
    let mut unique = ActiveChallengeKind::ALL.to_vec();
    unique.sort_by_key(|challenge| challenge.raw());
    unique.dedup();
    assert_eq!(unique.len(), ACTIVE_CHALLENGE_COUNT);
    assert_eq!(ActiveChallengeKind::VisibleRewardNavigation.raw(), 1);
    assert_eq!(ActiveChallengeKind::SlmDisabledDialectTransfer.raw(), 15);
}

#[test]
fn narration_frequency_and_dialect_divergence_are_recorded_without_history() {
    let mut statistics = PassiveLifeStatistics::new(OrganismId(9), Tick::ZERO).unwrap();
    statistics
        .observe(PassiveLifeEvent::NarrationUtterance)
        .unwrap();
    statistics
        .observe(PassiveLifeEvent::Narration { faithful: true })
        .unwrap();
    statistics
        .observe(PassiveLifeEvent::DialectDivergence {
            distance_q16: 12_345,
        })
        .unwrap();
    assert_eq!(statistics.narration_utterances(), 1);
    assert_eq!(
        statistics.metric(PassiveMetricKind::DialectDivergence),
        MetricReading::Measured {
            value_q16: 12_345,
            exposures: 1,
        }
    );
}

#[test]
fn finalized_statistics_reject_idle_and_food_hazard_patches_atomically() {
    for (contact, reward, pain) in [
        (PhysicalContactKind::Touch, 0.0, 0.0),
        (PhysicalContactKind::Consumed, -0.25, 0.2),
    ] {
        let patch = finalized_patch(contact, reward, pain, ActionKind::Interact);
        let mut statistics = PassiveLifeStatistics::new(OrganismId(7), Tick::ZERO).unwrap();
        statistics.finalize(Tick::new(12), "completed").unwrap();
        let before = statistics.clone();

        assert_eq!(
            statistics.observe_sealed_patch(&patch),
            Err(alife_core::ScaffoldContractError::InvalidId)
        );
        assert_eq!(statistics, before);
    }
}

#[test]
fn sealed_harm_does_not_measure_avoidance_without_explicit_opportunities() {
    let patch = finalized_patch(
        PhysicalContactKind::Consumed,
        -0.25,
        0.2,
        ActionKind::Interact,
    );
    let mut statistics = PassiveLifeStatistics::new(OrganismId(7), Tick::ZERO).unwrap();

    statistics.observe_sealed_patch(&patch).unwrap();

    assert_eq!(
        statistics.metric(PassiveMetricKind::FoodSuccess),
        MetricReading::Measured {
            value_q16: 0,
            exposures: 1,
        }
    );
    for metric in [
        PassiveMetricKind::PoisonAvoidance,
        PassiveMetricKind::HazardAvoidance,
    ] {
        assert_eq!(statistics.metric(metric), MetricReading::Unknown);
    }

    for event in [
        PassiveLifeEvent::PoisonEncounter { avoided: true },
        PassiveLifeEvent::PoisonEncounter { avoided: false },
        PassiveLifeEvent::HazardEncounter { avoided: true },
        PassiveLifeEvent::HazardEncounter { avoided: false },
    ] {
        statistics.observe(event).unwrap();
    }
    for metric in [
        PassiveMetricKind::PoisonAvoidance,
        PassiveMetricKind::HazardAvoidance,
    ] {
        assert_eq!(
            statistics.metric(metric),
            MetricReading::Measured {
                value_q16: 32_768,
                exposures: 2,
            }
        );
    }
}

#[test]
fn heard_creature_vocalization_does_not_measure_peer_communication() {
    let patch = finalized_patch(PhysicalContactKind::Touch, 0.0, 0.0, ActionKind::Vocalize);
    let mut statistics = PassiveLifeStatistics::new(OrganismId(7), Tick::ZERO).unwrap();

    statistics.observe_sealed_patch(&patch).unwrap();

    assert_eq!(statistics.heard_token_exposures(), 1);
    assert_eq!(statistics.narration_utterances(), 1);
    assert_eq!(
        statistics.metric(PassiveMetricKind::PeerCommunication),
        MetricReading::Unknown
    );
    assert_eq!(
        statistics.metric(PassiveMetricKind::UnaidedComprehension),
        MetricReading::Unknown
    );

    statistics
        .observe(PassiveLifeEvent::PeerCommunication { successful: true })
        .unwrap();
    assert_eq!(
        statistics.metric(PassiveMetricKind::PeerCommunication),
        MetricReading::Measured {
            value_q16: 65_535,
            exposures: 1,
        }
    );
}
