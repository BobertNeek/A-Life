use alife_core::{
    ActiveBatteryReceipt, Blake3Digest, GenomeId, LineageId, MetricReading, OrganismId,
    PassiveLifeEvent, PassiveLifeStatistics, PassiveMetricKind, Tick,
};
use alife_training::{
    CrossRunRanker, RankingCandidate, MAX_ACTIVE_BATTERY_CANDIDATES,
    MAX_SCREENED_CANDIDATES_PER_RUN,
};

fn candidate(
    run: &str,
    ordinal: u64,
    food_successes: u32,
    food_exposures: u32,
    pinned: bool,
    genome_distance_q16: Option<u32>,
) -> RankingCandidate {
    let organism_id = OrganismId(ordinal + 1);
    let mut passive = PassiveLifeStatistics::new(organism_id, Tick::ZERO).unwrap();
    for exposure in 0..food_exposures {
        passive
            .observe(PassiveLifeEvent::FoodOutcome {
                beneficial: exposure < food_successes,
            })
            .unwrap();
    }
    RankingCandidate {
        manifest_digest: Blake3Digest::from_bytes([ordinal as u8; 32]),
        source_run_id: run.to_string(),
        organism_id,
        genome_id: GenomeId(ordinal + 100),
        lineage_id: LineageId(1),
        parent_genome_ids: vec![GenomeId(1)],
        genome_distance_q16,
        passive,
        active: ActiveBatteryReceipt::empty(organism_id),
        novelty_q16: (ordinal as u32).saturating_mul(100),
        mutation_representative: ordinal.is_multiple_of(17),
        pinned,
    }
}

#[test]
fn screening_is_bounded_and_keeps_unknown_distinct_from_zero() {
    let mut candidates = (0..70)
        .map(|index| {
            candidate(
                "run-a",
                index,
                (index % 5) as u32,
                4,
                index == 69,
                Some(index as u32),
            )
        })
        .collect::<Vec<_>>();
    candidates.push(candidate("run-b", 100, 0, 0, false, None));
    candidates.push(candidate("run-b", 101, 0, 1, false, None));

    assert_eq!(
        candidates[70]
            .passive
            .metric(PassiveMetricKind::FoodSuccess),
        MetricReading::Unknown
    );
    assert!(matches!(
        candidates[71]
            .passive
            .metric(PassiveMetricKind::FoodSuccess),
        MetricReading::Measured { value_q16: 0, .. }
    ));

    let screening = CrossRunRanker::screen(&candidates).unwrap();
    let run_a = screening.run("run-a").unwrap();
    assert!(run_a.screened.len() <= MAX_SCREENED_CANDIDATES_PER_RUN);
    assert!(run_a.full_battery.len() <= MAX_ACTIVE_BATTERY_CANDIDATES);
    assert!(run_a.screened.contains(&candidates[69].manifest_digest));
    let run_b = screening.run("run-b").unwrap();
    assert!(run_b.screened.contains(&candidates[70].manifest_digest));
    assert!(run_b.screened.contains(&candidates[71].manifest_digest));
}

#[test]
fn ancestry_and_genome_distance_are_visible_but_never_a_penalty() {
    let near = candidate("run-a", 1, 3, 4, false, Some(1));
    let far = candidate("run-a", 2, 3, 4, false, Some(60_000));
    let front = CrossRunRanker::pareto_front(&[near.clone(), far.clone()]).unwrap();
    assert_eq!(front.len(), 2);
    assert!(front.contains(&near.manifest_digest));
    assert!(front.contains(&far.manifest_digest));
    assert_ne!(near.genome_distance_q16, far.genome_distance_q16);
}
