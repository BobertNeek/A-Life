//! Offline cross-run screening and Pareto ranking over archived creature evidence.

use std::{cmp::Ordering, collections::BTreeMap};

use alife_core::{
    ActiveBatteryReceipt, Blake3Digest, GenomeId, LineageId, MetricReading, OrganismId,
    PassiveLifeStatistics, PassiveMetricKind, ScaffoldContractError,
};

pub const MAX_SCREENED_CANDIDATES_PER_RUN: usize = 64;
pub const MAX_ACTIVE_BATTERY_CANDIDATES: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankingCandidate {
    pub manifest_digest: Blake3Digest,
    pub source_run_id: String,
    pub organism_id: OrganismId,
    pub genome_id: GenomeId,
    pub lineage_id: LineageId,
    pub parent_genome_ids: Vec<GenomeId>,
    pub genome_distance_q16: Option<u32>,
    pub passive: PassiveLifeStatistics,
    pub active: ActiveBatteryReceipt,
    pub novelty_q16: u32,
    pub mutation_representative: bool,
    pub pinned: bool,
}

impl RankingCandidate {
    fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.source_run_id.trim().is_empty()
            || self.source_run_id.chars().count() > 96
            || !self
                .source_run_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
            || self.novelty_q16 > 65_535
            || self
                .genome_distance_q16
                .is_some_and(|distance| distance > 65_535)
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        self.organism_id.validate()?;
        self.genome_id.validate()?;
        self.lineage_id.validate()?;
        for parent in &self.parent_genome_ids {
            parent.validate()?;
        }
        self.passive.validate_contract()?;
        self.active.validate_contract()?;
        if self.passive.organism_id() != self.organism_id
            || self.active.organism_id != self.organism_id
        {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunScreening {
    pub source_run_id: String,
    pub screened: Vec<Blake3Digest>,
    pub full_battery: Vec<Blake3Digest>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossRunScreening {
    pub runs: Vec<RunScreening>,
}

impl CrossRunScreening {
    pub fn run(&self, source_run_id: &str) -> Option<&RunScreening> {
        self.runs
            .iter()
            .find(|run| run.source_run_id == source_run_id)
    }
}

pub struct CrossRunRanker;

impl CrossRunRanker {
    pub fn screen(
        candidates: &[RankingCandidate],
    ) -> Result<CrossRunScreening, ScaffoldContractError> {
        let mut grouped = BTreeMap::<&str, Vec<&RankingCandidate>>::new();
        for candidate in candidates {
            candidate.validate()?;
            grouped
                .entry(candidate.source_run_id.as_str())
                .or_default()
                .push(candidate);
        }
        let mut runs = Vec::with_capacity(grouped.len());
        for (source_run_id, mut run_candidates) in grouped {
            run_candidates.sort_by_key(|candidate| candidate.manifest_digest.bytes());
            if run_candidates
                .windows(2)
                .any(|pair| pair[0].manifest_digest == pair[1].manifest_digest)
                || run_candidates
                    .iter()
                    .filter(|candidate| candidate.pinned)
                    .count()
                    > MAX_SCREENED_CANDIDATES_PER_RUN
            {
                return Err(ScaffoldContractError::InvalidId);
            }

            let mut selected = vec![false; run_candidates.len()];
            if run_candidates.len() <= MAX_SCREENED_CANDIDATES_PER_RUN {
                selected.fill(true);
            } else {
                select_where(&run_candidates, &mut selected, |candidate| candidate.pinned);
                select_where(&run_candidates, &mut selected, |candidate| {
                    candidate.mutation_representative
                });
                select_top_n(
                    &run_candidates,
                    &mut selected,
                    percentile_count(run_candidates.len()),
                    |candidate| Some(u64::from(candidate.novelty_q16)),
                );
                for metric in PassiveMetricKind::ALL {
                    let measured = run_candidates
                        .iter()
                        .filter(|candidate| candidate.passive.metric(metric).value_q16().is_some())
                        .count();
                    select_top_n(
                        &run_candidates,
                        &mut selected,
                        percentile_count(measured),
                        |candidate| candidate.passive.metric(metric).value_q16().map(u64::from),
                    );
                }
                let owned = run_candidates
                    .iter()
                    .map(|candidate| (*candidate).clone())
                    .collect::<Vec<_>>();
                let pareto = Self::pareto_front_indices(&owned)?;
                for index in pareto {
                    if selected.iter().filter(|selected| **selected).count()
                        >= MAX_SCREENED_CANDIDATES_PER_RUN
                    {
                        break;
                    }
                    selected[index] = true;
                }
                let mut fill = (0..run_candidates.len()).collect::<Vec<_>>();
                fill.sort_by(|left, right| {
                    compare_quality(run_candidates[*right], run_candidates[*left])
                        .then_with(|| left.cmp(right))
                });
                for index in fill {
                    if selected.iter().filter(|selected| **selected).count()
                        >= MAX_SCREENED_CANDIDATES_PER_RUN
                    {
                        break;
                    }
                    selected[index] = true;
                }
            }

            let mut screened_indices = selected
                .iter()
                .enumerate()
                .filter_map(|(index, selected)| selected.then_some(index))
                .collect::<Vec<_>>();
            if screened_indices.len() > MAX_SCREENED_CANDIDATES_PER_RUN {
                screened_indices.sort_by(|left, right| {
                    run_candidates[*right]
                        .pinned
                        .cmp(&run_candidates[*left].pinned)
                        .then_with(|| {
                            compare_quality(run_candidates[*right], run_candidates[*left])
                        })
                        .then_with(|| left.cmp(right))
                });
                screened_indices.truncate(MAX_SCREENED_CANDIDATES_PER_RUN);
            }
            screened_indices.sort_unstable();
            let screened = screened_indices
                .iter()
                .map(|index| run_candidates[*index].manifest_digest)
                .collect::<Vec<_>>();
            let mut battery_indices = screened_indices;
            battery_indices.sort_by(|left, right| {
                compare_quality(run_candidates[*right], run_candidates[*left])
                    .then_with(|| left.cmp(right))
            });
            battery_indices.truncate(MAX_ACTIVE_BATTERY_CANDIDATES);
            let full_battery = battery_indices
                .into_iter()
                .map(|index| run_candidates[index].manifest_digest)
                .collect();
            runs.push(RunScreening {
                source_run_id: source_run_id.to_string(),
                screened,
                full_battery,
            });
        }
        Ok(CrossRunScreening { runs })
    }

    pub fn pareto_front(
        candidates: &[RankingCandidate],
    ) -> Result<Vec<Blake3Digest>, ScaffoldContractError> {
        Ok(Self::pareto_front_indices(candidates)?
            .into_iter()
            .map(|index| candidates[index].manifest_digest)
            .collect())
    }

    fn pareto_front_indices(
        candidates: &[RankingCandidate],
    ) -> Result<Vec<usize>, ScaffoldContractError> {
        for candidate in candidates {
            candidate.validate()?;
        }
        Ok(candidates
            .iter()
            .enumerate()
            .filter(|(index, candidate)| {
                !candidates.iter().enumerate().any(|(other_index, other)| {
                    other_index != *index && dominates(other, candidate)
                })
            })
            .map(|(index, _)| index)
            .collect())
    }
}

fn select_where(
    candidates: &[&RankingCandidate],
    selected: &mut [bool],
    predicate: impl Fn(&RankingCandidate) -> bool,
) {
    for (index, candidate) in candidates.iter().enumerate() {
        if selected.iter().filter(|selected| **selected).count() >= MAX_SCREENED_CANDIDATES_PER_RUN
        {
            return;
        }
        if predicate(candidate) {
            selected[index] = true;
        }
    }
}

fn select_top_n(
    candidates: &[&RankingCandidate],
    selected: &mut [bool],
    count: usize,
    value: impl Fn(&RankingCandidate) -> Option<u64>,
) {
    if count == 0 {
        return;
    }
    let mut measured = candidates
        .iter()
        .enumerate()
        .filter_map(|(index, candidate)| value(candidate).map(|value| (index, value)))
        .collect::<Vec<_>>();
    measured.sort_by(|(left_index, left), (right_index, right)| {
        right.cmp(left).then_with(|| left_index.cmp(right_index))
    });
    for (index, _) in measured.into_iter().take(count) {
        if selected.iter().filter(|selected| **selected).count() >= MAX_SCREENED_CANDIDATES_PER_RUN
        {
            break;
        }
        selected[index] = true;
    }
}

fn percentile_count(population: usize) -> usize {
    if population == 0 {
        0
    } else {
        population.saturating_mul(5).div_ceil(100).max(1)
    }
}

fn dominates(left: &RankingCandidate, right: &RankingCandidate) -> bool {
    let mut shared = false;
    let mut strictly_better = false;
    for metric in PassiveMetricKind::ALL {
        if let (Some(left), Some(right)) = (
            left.passive.metric(metric).value_q16(),
            right.passive.metric(metric).value_q16(),
        ) {
            shared = true;
            if left < right {
                return false;
            }
            strictly_better |= left > right;
        }
    }
    for (left, right) in left.active.results.iter().zip(&right.active.results) {
        if let (
            MetricReading::Measured {
                value_q16: left, ..
            },
            MetricReading::Measured {
                value_q16: right, ..
            },
        ) = (left.score, right.score)
        {
            shared = true;
            if left < right {
                return false;
            }
            strictly_better |= left > right;
        }
    }
    shared && strictly_better
}

fn compare_quality(left: &RankingCandidate, right: &RankingCandidate) -> Ordering {
    quality(left).cmp(&quality(right))
}

fn quality(candidate: &RankingCandidate) -> (u64, u32, u32) {
    let mut total = 0_u64;
    let mut known = 0_u32;
    for metric in PassiveMetricKind::ALL {
        if let Some(value) = candidate.passive.metric(metric).value_q16() {
            total += u64::from(value);
            known += 1;
        }
    }
    for result in &candidate.active.results {
        if let Some(value) = result.score.value_q16() {
            total += u64::from(value);
            known += 1;
        }
    }
    let mean = if known == 0 {
        0
    } else {
        total / u64::from(known)
    };
    (mean, known, candidate.novelty_q16)
}
