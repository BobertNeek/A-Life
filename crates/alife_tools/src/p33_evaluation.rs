//! EI0 deterministic intelligence-battery evaluation over packed experience logs.
//!
//! This module is offline tooling. It scores recorded outcomes and selection
//! evidence without issuing actions, injecting rewards, or becoming runtime
//! policy authority.

use std::collections::{BTreeMap, BTreeSet};

use alife_core::{
    GenomeId, LineageId, PackedExperienceRecord, ScaffoldContractError, Validate,
    PACKED_FLAG_SUCCESS,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const EI0_EVALUATION_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Error)]
pub enum EvaluationError {
    #[error("packed-log contract failed: {0}")]
    Contract(#[from] ScaffoldContractError),
    #[error("invalid battery suite: {0}")]
    InvalidSuite(&'static str),
    #[error("invalid battery trial `{test_id}`: {message}")]
    InvalidTrial {
        test_id: String,
        message: &'static str,
    },
    #[error("hidden promotion trial `{test_id}` is missing required provenance")]
    MissingPromotionProvenance { test_id: String },
    #[error("hidden promotion trial `{test_id}` has assistance or prior exposure")]
    ContaminatedPromotionEvidence { test_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatteryLayer {
    PermanentAnchor,
    ProceduralBreeding,
    HiddenPromotion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrialDomain {
    Ecology,
    Learning,
    Transfer,
    Reversal,
    DelayedMemory,
    Abstraction,
    SocialContribution,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrialPhase {
    Baseline,
    Acquisition,
    Transfer,
    Reversal,
    DelayRecall,
    ActiveGroup,
    MemberRemoved,
    Replacement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamMode {
    Individual,
    PersistentPack,
    RandomizedTeam,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssistanceKind {
    Teacher,
    PlayerPossession,
    SemanticPrior,
    HiddenReward,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeProvenance {
    pub adapter: String,
    pub backend: String,
    pub dispatches: u64,
    pub neural_ticks: u64,
    pub elapsed_micros: u64,
    pub energy_milliunits: u64,
    pub budget_units: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineageProvenance {
    pub lineage_id: LineageId,
    pub genome_id: GenomeId,
    pub ancestor_genome_ids: Vec<GenomeId>,
    pub population_share: f32,
    pub genome_novelty: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationProvenance {
    pub source_run_id: String,
    pub foundation_id: String,
    pub foundation_version: u32,
    pub exposure_count: u32,
    pub assistance: Vec<AssistanceKind>,
    pub compute: ComputeProvenance,
    pub lineage: LineageProvenance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrialTrace {
    pub phase: TrialPhase,
    pub records: Vec<PackedExperienceRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatteryTrial {
    pub test_id: String,
    pub layer: BatteryLayer,
    pub domain: TrialDomain,
    pub team_mode: TeamMode,
    pub seed: u64,
    pub variant_id: String,
    pub answer_fingerprint: Option<String>,
    pub hidden_set_id: Option<String>,
    pub focal_organism_id: u64,
    pub provenance: EvaluationProvenance,
    pub traces: Vec<TrialTrace>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatterySuite {
    pub schema_version: u16,
    pub suite_id: String,
    pub trials: Vec<BatteryTrial>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScoreEstimate {
    pub value: Option<f32>,
    pub samples: u32,
}

impl ScoreEstimate {
    pub const UNKNOWN: Self = Self {
        value: None,
        samples: 0,
    };

    pub fn known(value: f32, samples: u32) -> Self {
        Self {
            value: Some(value.clamp(0.0, 1.0)),
            samples,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CognitiveMeasures {
    pub learning: ScoreEstimate,
    pub transfer: ScoreEstimate,
    pub reversal: ScoreEstimate,
    pub delayed_memory: ScoreEstimate,
    pub abstraction: ScoreEstimate,
    pub social_contribution: ScoreEstimate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectiveVector {
    pub ecological: ScoreEstimate,
    pub cognitive: ScoreEstimate,
    pub social: ScoreEstimate,
    pub group: ScoreEstimate,
    pub stability: ScoreEstimate,
    pub efficiency: ScoreEstimate,
    pub diversity: ScoreEstimate,
}

impl ObjectiveVector {
    pub fn all_known(&self) -> bool {
        [
            self.ecological.value,
            self.cognitive.value,
            self.social.value,
            self.group.value,
            self.stability.value,
            self.efficiency.value,
            self.diversity.value,
        ]
        .into_iter()
        .all(|value| value.is_some())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationFlag {
    AnchorProceduralGap,
    FixedAnswerOverfit,
    GroupFreeRider,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatteryLayerCounts {
    pub permanent_anchor: usize,
    pub procedural_breeding: usize,
    pub hidden_promotion: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatteryReport {
    pub schema_version: u16,
    pub suite_id: String,
    pub packed_record_count: usize,
    pub source_run_ids: Vec<String>,
    pub source_backends: Vec<String>,
    pub layer_counts: BatteryLayerCounts,
    pub measures: CognitiveMeasures,
    pub objectives: ObjectiveVector,
    pub flags: Vec<EvaluationFlag>,
    pub promotion_eligible: bool,
}

pub fn evaluate_battery(suite: &BatterySuite) -> Result<BatteryReport, EvaluationError> {
    validate_suite(suite)?;

    let measures = CognitiveMeasures {
        learning: learning_measure(&suite.trials),
        transfer: phase_domain_measure(&suite.trials, TrialDomain::Transfer, TrialPhase::Transfer),
        reversal: reversal_measure(&suite.trials),
        delayed_memory: phase_domain_measure(
            &suite.trials,
            TrialDomain::DelayedMemory,
            TrialPhase::DelayRecall,
        ),
        abstraction: abstraction_measure(&suite.trials),
        social_contribution: social_contribution_measure(&suite.trials),
    };

    let trial_scores = suite
        .trials
        .iter()
        .map(trial_performance)
        .collect::<Vec<_>>();
    let cognitive = mean_if_all(&[
        measures.learning,
        measures.transfer,
        measures.reversal,
        measures.delayed_memory,
        measures.abstraction,
    ]);
    let objectives =
        ObjectiveVector {
            ecological: estimate(suite.trials.iter().zip(&trial_scores).filter_map(
                |(trial, score)| (trial.domain == TrialDomain::Ecology).then_some(*score),
            )),
            cognitive,
            social: social_objective(&suite.trials),
            group: group_objective(&suite.trials),
            stability: stability_objective(&trial_scores),
            efficiency: efficiency_objective(&suite.trials, &trial_scores),
            diversity: diversity_objective(&suite.trials),
        };
    let flags = evaluation_flags(&suite.trials, &trial_scores);
    let promotion_eligible = promotion_ready(&suite.trials, &measures, &objectives, &flags);
    let source_run_ids = sorted_unique(
        suite
            .trials
            .iter()
            .map(|trial| trial.provenance.source_run_id.clone()),
    );
    let source_backends = sorted_unique(
        suite
            .trials
            .iter()
            .map(|trial| trial.provenance.compute.backend.clone()),
    );

    Ok(BatteryReport {
        schema_version: EI0_EVALUATION_SCHEMA_VERSION,
        suite_id: suite.suite_id.clone(),
        packed_record_count: suite
            .trials
            .iter()
            .flat_map(|trial| &trial.traces)
            .map(|trace| trace.records.len())
            .sum(),
        source_run_ids,
        source_backends,
        layer_counts: BatteryLayerCounts {
            permanent_anchor: suite
                .trials
                .iter()
                .filter(|trial| trial.layer == BatteryLayer::PermanentAnchor)
                .count(),
            procedural_breeding: suite
                .trials
                .iter()
                .filter(|trial| trial.layer == BatteryLayer::ProceduralBreeding)
                .count(),
            hidden_promotion: suite
                .trials
                .iter()
                .filter(|trial| trial.layer == BatteryLayer::HiddenPromotion)
                .count(),
        },
        measures,
        objectives,
        flags,
        promotion_eligible,
    })
}

fn validate_suite(suite: &BatterySuite) -> Result<(), EvaluationError> {
    if suite.schema_version != EI0_EVALUATION_SCHEMA_VERSION {
        return Err(EvaluationError::InvalidSuite("schema version mismatch"));
    }
    if suite.suite_id.trim().is_empty() || suite.trials.is_empty() {
        return Err(EvaluationError::InvalidSuite(
            "suite id and at least one trial are required",
        ));
    }
    for trial in &suite.trials {
        validate_trial(trial)?;
    }
    Ok(())
}

fn validate_trial(trial: &BatteryTrial) -> Result<(), EvaluationError> {
    let invalid = |message| EvaluationError::InvalidTrial {
        test_id: trial.test_id.clone(),
        message,
    };
    if trial.test_id.trim().is_empty()
        || trial.variant_id.trim().is_empty()
        || trial.seed == 0
        || trial.focal_organism_id == 0
        || trial.traces.is_empty()
    {
        return Err(invalid(
            "identity, seed, focal organism, and traces are required",
        ));
    }
    let provenance = &trial.provenance;
    if provenance.source_run_id.trim().is_empty()
        || provenance.foundation_id.trim().is_empty()
        || provenance.foundation_version == 0
        || provenance.compute.adapter.trim().is_empty()
        || provenance.compute.backend.trim().is_empty()
        || provenance.compute.budget_units == 0
    {
        if trial.layer == BatteryLayer::HiddenPromotion {
            return Err(EvaluationError::MissingPromotionProvenance {
                test_id: trial.test_id.clone(),
            });
        }
        return Err(invalid("complete evaluation provenance is required"));
    }
    provenance.lineage.lineage_id.validate()?;
    provenance.lineage.genome_id.validate()?;
    for ancestor in &provenance.lineage.ancestor_genome_ids {
        ancestor.validate()?;
    }
    if !unit_interval(provenance.lineage.population_share)
        || !unit_interval(provenance.lineage.genome_novelty)
    {
        return Err(invalid(
            "lineage share and novelty must be finite unit values",
        ));
    }
    if trial.layer == BatteryLayer::HiddenPromotion {
        if trial
            .hidden_set_id
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        {
            return Err(EvaluationError::MissingPromotionProvenance {
                test_id: trial.test_id.clone(),
            });
        }
        if provenance.exposure_count != 0 || !provenance.assistance.is_empty() {
            return Err(EvaluationError::ContaminatedPromotionEvidence {
                test_id: trial.test_id.clone(),
            });
        }
    }
    for trace in &trial.traces {
        if trace.records.is_empty() {
            return Err(invalid("each phase needs at least one packed record"));
        }
        for record in &trace.records {
            record.validate_contract()?;
            if record.frame.organism_id != trial.focal_organism_id {
                return Err(invalid(
                    "packed record organism does not match focal organism",
                ));
            }
        }
    }
    Ok(())
}

fn unit_interval(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn trace_performance(records: &[PackedExperienceRecord]) -> f32 {
    let count = records.len() as f32;
    let success = records
        .iter()
        .filter(|record| record.frame.flags & PACKED_FLAG_SUCCESS != 0)
        .count() as f32
        / count;
    let reward = records
        .iter()
        .map(|record| record.frame.reward_valence.max(0.0))
        .sum::<f32>()
        / count;
    let prediction = records
        .iter()
        .map(|record| 1.0 - record.frame.prediction_error)
        .sum::<f32>()
        / count;
    let energy = records
        .iter()
        .map(|record| 1.0 - record.frame.energy_delta.abs())
        .sum::<f32>()
        / count;
    (0.45 * success + 0.25 * reward + 0.20 * prediction + 0.10 * energy).clamp(0.0, 1.0)
}

fn trial_performance(trial: &BatteryTrial) -> f32 {
    trial
        .traces
        .iter()
        .map(|trace| trace_performance(&trace.records))
        .sum::<f32>()
        / trial.traces.len() as f32
}

fn trace_score(trial: &BatteryTrial, phase: TrialPhase) -> Option<f32> {
    let values = trial
        .traces
        .iter()
        .filter(|trace| trace.phase == phase)
        .map(|trace| trace_performance(&trace.records))
        .collect::<Vec<_>>();
    (!values.is_empty()).then(|| values.iter().sum::<f32>() / values.len() as f32)
}

fn learning_measure(trials: &[BatteryTrial]) -> ScoreEstimate {
    estimate(trials.iter().filter_map(|trial| {
        if trial.domain != TrialDomain::Learning {
            return None;
        }
        let baseline = trace_score(trial, TrialPhase::Baseline)?;
        let acquisition = trial
            .traces
            .iter()
            .find(|trace| trace.phase == TrialPhase::Acquisition)?;
        let acquired = trace_performance(&acquisition.records);
        let first_success = acquisition
            .records
            .iter()
            .position(|record| record.frame.flags & PACKED_FLAG_SUCCESS != 0);
        let speed = first_success.map_or(0.0, |index| {
            1.0 - index as f32 / acquisition.records.len() as f32
        });
        let improvement = (0.5 + 0.5 * (acquired - baseline)).clamp(0.0, 1.0);
        Some(0.7 * improvement + 0.3 * speed)
    }))
}

fn phase_domain_measure(
    trials: &[BatteryTrial],
    domain: TrialDomain,
    phase: TrialPhase,
) -> ScoreEstimate {
    estimate(trials.iter().filter_map(|trial| {
        (trial.domain == domain)
            .then(|| trace_score(trial, phase))
            .flatten()
    }))
}

fn reversal_measure(trials: &[BatteryTrial]) -> ScoreEstimate {
    estimate(trials.iter().filter_map(|trial| {
        if trial.domain != TrialDomain::Reversal {
            return None;
        }
        let trace = trial
            .traces
            .iter()
            .find(|trace| trace.phase == TrialPhase::Reversal)?;
        let split = trace.records.len() / 2;
        (split < trace.records.len()).then(|| trace_performance(&trace.records[split..]))
    }))
}

fn abstraction_measure(trials: &[BatteryTrial]) -> ScoreEstimate {
    let abstraction = trials
        .iter()
        .filter(|trial| trial.domain == TrialDomain::Abstraction)
        .collect::<Vec<_>>();
    let variants = abstraction
        .iter()
        .map(|trial| trial.variant_id.as_str())
        .collect::<BTreeSet<_>>();
    if variants.len() < 2 {
        return ScoreEstimate::UNKNOWN;
    }
    estimate(abstraction.into_iter().map(trial_performance))
}

fn social_contribution(trial: &BatteryTrial) -> Option<f32> {
    if trial.domain != TrialDomain::SocialContribution {
        return None;
    }
    let active = trace_score(trial, TrialPhase::ActiveGroup)?;
    let removed = trace_score(trial, TrialPhase::MemberRemoved)?;
    let replacement = trace_score(trial, TrialPhase::Replacement).unwrap_or(removed);
    Some((active - 0.5 * (removed + replacement)).clamp(0.0, 1.0))
}

fn social_contribution_measure(trials: &[BatteryTrial]) -> ScoreEstimate {
    estimate(trials.iter().filter_map(social_contribution))
}

fn social_objective(trials: &[BatteryTrial]) -> ScoreEstimate {
    estimate(trials.iter().filter_map(|trial| {
        let contribution = social_contribution(trial)?;
        let active = trace_score(trial, TrialPhase::ActiveGroup)?;
        Some(0.5 * active + 0.5 * contribution)
    }))
}

fn group_objective(trials: &[BatteryTrial]) -> ScoreEstimate {
    let persistent = estimate(trials.iter().filter_map(|trial| {
        (trial.team_mode == TeamMode::PersistentPack)
            .then(|| social_contribution(trial))
            .flatten()
    }));
    let randomized = estimate(trials.iter().filter_map(|trial| {
        (trial.team_mode == TeamMode::RandomizedTeam)
            .then(|| social_contribution(trial))
            .flatten()
    }));
    mean_if_all(&[persistent, randomized])
}

fn stability_objective(scores: &[f32]) -> ScoreEstimate {
    if scores.len() < 2 {
        return ScoreEstimate::UNKNOWN;
    }
    let min = scores.iter().copied().fold(f32::INFINITY, f32::min);
    let max = scores.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    ScoreEstimate::known(1.0 - (max - min), scores.len() as u32)
}

fn efficiency_objective(trials: &[BatteryTrial], scores: &[f32]) -> ScoreEstimate {
    estimate(trials.iter().zip(scores).map(|(trial, score)| {
        let compute = &trial.provenance.compute;
        let cost = compute.energy_milliunits as f32 / compute.budget_units as f32;
        score / (1.0 + cost)
    }))
}

fn diversity_objective(trials: &[BatteryTrial]) -> ScoreEstimate {
    estimate(trials.iter().map(|trial| {
        let lineage = &trial.provenance.lineage;
        0.5 * lineage.genome_novelty + 0.5 * (1.0 - lineage.population_share)
    }))
}

fn estimate(values: impl IntoIterator<Item = f32>) -> ScoreEstimate {
    let values = values.into_iter().collect::<Vec<_>>();
    if values.is_empty() {
        ScoreEstimate::UNKNOWN
    } else {
        ScoreEstimate::known(
            values.iter().copied().sum::<f32>() / values.len() as f32,
            values.len() as u32,
        )
    }
}

fn mean_if_all(values: &[ScoreEstimate]) -> ScoreEstimate {
    if values.iter().any(|score| score.value.is_none()) {
        return ScoreEstimate::UNKNOWN;
    }
    ScoreEstimate::known(
        values.iter().filter_map(|score| score.value).sum::<f32>() / values.len() as f32,
        values.iter().map(|score| score.samples).sum(),
    )
}

fn evaluation_flags(trials: &[BatteryTrial], scores: &[f32]) -> Vec<EvaluationFlag> {
    let mut flags = BTreeSet::new();
    let mut fingerprints: BTreeMap<&str, (BTreeSet<u64>, BTreeSet<&str>)> = BTreeMap::new();
    for trial in trials {
        if let Some(fingerprint) = trial
            .answer_fingerprint
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            let entry = fingerprints.entry(fingerprint).or_default();
            entry.0.insert(trial.seed);
            entry.1.insert(trial.variant_id.as_str());
        }
        if trial.domain == TrialDomain::SocialContribution {
            if let (Some(active), Some(removed)) = (
                trace_score(trial, TrialPhase::ActiveGroup),
                trace_score(trial, TrialPhase::MemberRemoved),
            ) {
                if active <= removed {
                    flags.insert(EvaluationFlag::GroupFreeRider);
                }
            }
        }
    }
    if fingerprints
        .values()
        .any(|(seeds, variants)| seeds.len() > 1 && variants.len() > 1)
    {
        flags.insert(EvaluationFlag::FixedAnswerOverfit);
    }

    let anchors = estimate(trials.iter().zip(scores).filter_map(|(trial, score)| {
        (trial.layer == BatteryLayer::PermanentAnchor).then_some(*score)
    }));
    let procedural = estimate(trials.iter().zip(scores).filter_map(|(trial, score)| {
        (trial.layer == BatteryLayer::ProceduralBreeding).then_some(*score)
    }));
    if let (Some(anchor), Some(procedural)) = (anchors.value, procedural.value) {
        if anchor - procedural >= 0.35 {
            flags.insert(EvaluationFlag::AnchorProceduralGap);
        }
    }
    flags.into_iter().collect()
}

fn promotion_ready(
    trials: &[BatteryTrial],
    measures: &CognitiveMeasures,
    objectives: &ObjectiveVector,
    flags: &[EvaluationFlag],
) -> bool {
    if !flags.is_empty() || !objectives.all_known() {
        return false;
    }
    if [
        measures.learning.value,
        measures.transfer.value,
        measures.reversal.value,
        measures.delayed_memory.value,
        measures.abstraction.value,
        measures.social_contribution.value,
    ]
    .into_iter()
    .any(|value| value.is_none())
    {
        return false;
    }
    let hidden = trials
        .iter()
        .filter(|trial| trial.layer == BatteryLayer::HiddenPromotion)
        .collect::<Vec<_>>();
    if hidden.is_empty() {
        return false;
    }
    let domains = hidden
        .iter()
        .map(|trial| trial.domain)
        .collect::<BTreeSet<_>>();
    let teams = hidden
        .iter()
        .map(|trial| trial.team_mode)
        .collect::<BTreeSet<_>>();
    [
        TrialDomain::Learning,
        TrialDomain::Transfer,
        TrialDomain::Reversal,
        TrialDomain::DelayedMemory,
        TrialDomain::Abstraction,
        TrialDomain::SocialContribution,
    ]
    .into_iter()
    .all(|domain| domains.contains(&domain))
        && teams.contains(&TeamMode::PersistentPack)
        && teams.contains(&TeamMode::RandomizedTeam)
}

fn sorted_unique(values: impl IntoIterator<Item = String>) -> Vec<String> {
    values
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
