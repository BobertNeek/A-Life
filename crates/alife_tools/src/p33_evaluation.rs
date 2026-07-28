//! EI0 deterministic intelligence-battery evaluation over packed experience logs.
//!
//! This module is offline tooling. It scores recorded outcomes and selection
//! evidence without issuing actions, injecting rewards, or becoming runtime
//! policy authority.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use alife_core::{
    BrainGenome, GenomeId, LineageId, PackedExperienceRecord, ScaffoldContractError, Validate,
    PACKED_FLAG_SUCCESS,
};
use alife_world::{ScenarioFixture, ScenarioName};
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
    #[error("failed to read battery fixture: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse battery fixture: {0}")]
    Json(#[from] serde_json::Error),
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
    ObservedBehavior,
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
    ObservedOutcome,
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
    pub evidence_scope: EvidenceScope,
    pub measures: CognitiveMeasures,
    pub objectives: ObjectiveVector,
    pub flags: Vec<EvaluationFlag>,
    pub promotion_eligible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceScope {
    pub promotion_backend_eligible: bool,
    pub unsupported_measures: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioBatteryFixture {
    pub schema_version: u16,
    pub suite_id: String,
    pub cases: Vec<ScenarioBatteryCase>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioBatteryCase {
    pub test_id: String,
    pub layer: BatteryLayer,
    pub domain: TrialDomain,
    pub team_mode: TeamMode,
    pub variant_id: String,
    pub answer_fingerprint: Option<String>,
    pub hidden_set_id: Option<String>,
    pub foundation_id: String,
    pub foundation_version: u32,
    pub exposure_count: u32,
    pub assistance: Vec<AssistanceKind>,
    pub lineage_id: u64,
    pub ancestor_genome_ids: Vec<GenomeId>,
    pub population_share: f32,
    pub genome_novelty: f32,
    pub compute: ScenarioComputeProvenance,
    pub sources: Vec<ScenarioTraceSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioComputeProvenance {
    pub adapter: String,
    pub backend: String,
    pub elapsed_micros: u64,
    pub energy_milliunits: u64,
    pub budget_units: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioTraceSource {
    pub scenario: ScenarioSource,
    pub seed: u64,
    pub phase: TrialPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioSource {
    FoodSeeking,
    PoisonPainAvoidance,
    ObstacleFrustration,
    FatigueSleep,
    CuriosityContradiction,
    WordTokenGrounding,
    SimpleSocialTrustFear,
    TeacherPerceptionEvent,
}

impl ScenarioSource {
    fn scenario_name(self) -> ScenarioName {
        match self {
            Self::FoodSeeking => ScenarioName::FoodSeeking,
            Self::PoisonPainAvoidance => ScenarioName::PoisonPainAvoidance,
            Self::ObstacleFrustration => ScenarioName::ObstacleFrustration,
            Self::FatigueSleep => ScenarioName::FatigueSleep,
            Self::CuriosityContradiction => ScenarioName::CuriosityContradiction,
            Self::WordTokenGrounding => ScenarioName::WordTokenGrounding,
            Self::SimpleSocialTrustFear => ScenarioName::SimpleSocialTrustFear,
            Self::TeacherPerceptionEvent => ScenarioName::TeacherPerceptionEvent,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::FoodSeeking => "food-seeking",
            Self::PoisonPainAvoidance => "poison-pain-avoidance",
            Self::ObstacleFrustration => "obstacle-frustration",
            Self::FatigueSleep => "fatigue-sleep",
            Self::CuriosityContradiction => "curiosity-contradiction",
            Self::WordTokenGrounding => "word-token-grounding",
            Self::SimpleSocialTrustFear => "simple-social-trust-fear",
            Self::TeacherPerceptionEvent => "teacher-perception-event",
        }
    }
}

impl ScenarioBatteryFixture {
    pub fn from_json_file(path: impl AsRef<Path>) -> Result<Self, EvaluationError> {
        let fixture: Self = serde_json::from_str(&fs::read_to_string(path)?)?;
        if fixture.schema_version != EI0_EVALUATION_SCHEMA_VERSION {
            return Err(EvaluationError::InvalidSuite(
                "scenario fixture schema version mismatch",
            ));
        }
        if fixture.suite_id.trim().is_empty() || fixture.cases.is_empty() {
            return Err(EvaluationError::InvalidSuite(
                "scenario fixture needs a suite id and cases",
            ));
        }
        Ok(fixture)
    }

    pub fn run(&self) -> Result<BatteryReport, EvaluationError> {
        if self.schema_version != EI0_EVALUATION_SCHEMA_VERSION
            || self.suite_id.trim().is_empty()
            || self.cases.is_empty()
        {
            return Err(EvaluationError::InvalidSuite(
                "scenario fixture header is invalid",
            ));
        }
        let trials = self
            .cases
            .iter()
            .map(ScenarioBatteryCase::run)
            .collect::<Result<Vec<_>, _>>()?;
        evaluate_battery(&BatterySuite {
            schema_version: EI0_EVALUATION_SCHEMA_VERSION,
            suite_id: self.suite_id.clone(),
            trials,
        })
    }
}

impl ScenarioBatteryCase {
    fn run(&self) -> Result<BatteryTrial, EvaluationError> {
        if self.sources.is_empty() {
            return Err(EvaluationError::InvalidTrial {
                test_id: self.test_id.clone(),
                message: "scenario case needs at least one source",
            });
        }
        let first_seed = self.sources[0].seed;
        if first_seed == 0 || self.sources.iter().any(|source| source.seed != first_seed) {
            return Err(EvaluationError::InvalidTrial {
                test_id: self.test_id.clone(),
                message: "scenario phases must use one nonzero candidate seed",
            });
        }

        let mut phase_records: BTreeMap<TrialPhase, Vec<PackedExperienceRecord>> = BTreeMap::new();
        let mut source_run_ids = Vec::with_capacity(self.sources.len());
        let mut focal_organism_id = None;
        let mut genome_id = None;
        let mut packed_record_count = 0_u64;

        for source in &self.sources {
            let fixture = ScenarioFixture::with_seed(source.scenario.scenario_name(), source.seed)?;
            let candidate_genome = BrainGenome::scaffold(
                fixture.creature.genome_seed,
                fixture.creature.brain_tier.default_class_id(),
            );
            if genome_id
                .replace(candidate_genome.id)
                .is_some_and(|id| id != candidate_genome.id)
            {
                return Err(EvaluationError::InvalidTrial {
                    test_id: self.test_id.clone(),
                    message: "scenario phases resolved different candidate genomes",
                });
            }
            let run = fixture.run()?;
            let records = run
                .ticks
                .into_iter()
                .filter_map(|tick| tick.brain.packed_record)
                .collect::<Vec<_>>();
            if records.is_empty() {
                return Err(EvaluationError::InvalidTrial {
                    test_id: self.test_id.clone(),
                    message: "scenario source emitted no packed records",
                });
            }
            let organism_id = records[0].frame.organism_id;
            if focal_organism_id
                .replace(organism_id)
                .is_some_and(|id| id != organism_id)
            {
                return Err(EvaluationError::InvalidTrial {
                    test_id: self.test_id.clone(),
                    message: "scenario phases resolved different focal organisms",
                });
            }
            packed_record_count += records.len() as u64;
            phase_records
                .entry(source.phase)
                .or_default()
                .extend(records);
            source_run_ids.push(format!("{}@{}", source.scenario.label(), source.seed));
        }

        Ok(BatteryTrial {
            test_id: self.test_id.clone(),
            layer: self.layer,
            domain: self.domain,
            team_mode: self.team_mode,
            seed: first_seed,
            variant_id: self.variant_id.clone(),
            answer_fingerprint: self.answer_fingerprint.clone(),
            hidden_set_id: self.hidden_set_id.clone(),
            focal_organism_id: focal_organism_id.expect("nonempty source records set organism"),
            provenance: EvaluationProvenance {
                source_run_id: source_run_ids.join("+"),
                foundation_id: self.foundation_id.clone(),
                foundation_version: self.foundation_version,
                exposure_count: self.exposure_count,
                assistance: self.assistance.clone(),
                compute: ComputeProvenance {
                    adapter: self.compute.adapter.clone(),
                    backend: self.compute.backend.clone(),
                    dispatches: packed_record_count,
                    neural_ticks: packed_record_count,
                    elapsed_micros: self.compute.elapsed_micros,
                    energy_milliunits: self.compute.energy_milliunits,
                    budget_units: self.compute.budget_units,
                },
                lineage: LineageProvenance {
                    lineage_id: LineageId(self.lineage_id),
                    genome_id: genome_id.expect("nonempty source records set genome"),
                    ancestor_genome_ids: self.ancestor_genome_ids.clone(),
                    population_share: self.population_share,
                    genome_novelty: self.genome_novelty,
                },
            },
            traces: phase_records
                .into_iter()
                .map(|(phase, records)| TrialTrace { phase, records })
                .collect(),
        })
    }
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

    let layer_counts = BatteryLayerCounts {
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
    };
    let evidence_scope = evidence_scope(&source_backends, layer_counts, &measures, &objectives);

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
        layer_counts,
        evidence_scope,
        measures,
        objectives,
        flags,
        promotion_eligible,
    })
}

fn evidence_scope(
    backends: &[String],
    layers: BatteryLayerCounts,
    measures: &CognitiveMeasures,
    objectives: &ObjectiveVector,
) -> EvidenceScope {
    let promotion_backend_eligible = !backends.is_empty()
        && backends
            .iter()
            .all(|backend| backend != "HeuristicBaseline");
    let mut unsupported_measures = Vec::new();
    for (name, value) in [
        ("learning", measures.learning.value),
        ("transfer", measures.transfer.value),
        ("reversal", measures.reversal.value),
        ("delayed_memory", measures.delayed_memory.value),
        ("abstraction", measures.abstraction.value),
        ("social_contribution", measures.social_contribution.value),
        ("cognitive_objective", objectives.cognitive.value),
        ("social_objective", objectives.social.value),
        ("group_objective", objectives.group.value),
    ] {
        if value.is_none() {
            unsupported_measures.push(name.to_string());
        }
    }
    let mut notes = Vec::new();
    if !promotion_backend_eligible {
        notes.push(
            "HeuristicBaseline traces are deterministic tooling evidence, not GPU-authoritative promotion evidence."
                .to_string(),
        );
    }
    if !unsupported_measures.is_empty() {
        notes.push(
            "UNKNOWN means the fixture lacks a genuine phase or exposure; it is not a zero score."
                .to_string(),
        );
    }
    if layers.hidden_promotion == 0 {
        notes.push("No hidden promotion trials were supplied.".to_string());
    }
    EvidenceScope {
        promotion_backend_eligible,
        unsupported_measures,
        notes,
    }
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
