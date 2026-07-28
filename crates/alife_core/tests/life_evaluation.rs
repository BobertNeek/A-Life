use alife_core::{
    ActiveChallengeKind, EnvironmentalRegime, MetricReading, OrganismId, PassiveLifeEvent,
    PassiveLifeStatistics, PassiveMetricKind, Tick, ACTIVE_CHALLENGE_COUNT,
};

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
