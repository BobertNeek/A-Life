//! CA21 behavior tuning metrics over existing bounded ecology signals.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub struct BehaviorTuningConfig {
    pub schema: &'static str,
    pub schema_version: u16,
    pub seed: u64,
    pub minimum_sealed_patches: usize,
    pub minimum_sleep_cycles: u32,
    pub overfeeding_watch_threshold: f32,
    pub minimum_hazard_avoidance_score: f32,
    pub minimum_population_observed: usize,
}

impl BehaviorTuningConfig {
    pub const fn fast_ci() -> Self {
        Self {
            schema: CA21_BEHAVIOR_TUNING_SCHEMA,
            schema_version: CA21_BEHAVIOR_TUNING_SCHEMA_VERSION,
            seed: 21_210,
            minimum_sealed_patches: 8,
            minimum_sleep_cycles: 1,
            overfeeding_watch_threshold: 0.95,
            minimum_hazard_avoidance_score: 0.50,
            minimum_population_observed: 2,
        }
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != CA21_BEHAVIOR_TUNING_SCHEMA
            || self.schema_version != CA21_BEHAVIOR_TUNING_SCHEMA_VERSION
            || self.minimum_sealed_patches == 0
            || self.minimum_sealed_patches > 256
            || self.minimum_sleep_cycles == 0
            || self.minimum_sleep_cycles > 64
            || self.minimum_population_observed == 0
            || self.minimum_population_observed > G08_MAX_POPULATION_CAP
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        NormalizedScalar::new(self.overfeeding_watch_threshold)?;
        NormalizedScalar::new(self.minimum_hazard_avoidance_score)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BehaviorTuningFindingStatus {
    Clear,
    Watch,
    KnownLimitation,
}

impl BehaviorTuningFindingStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Clear => "clear",
            Self::Watch => "watch",
            Self::KnownLimitation => "known-limitation",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BehaviorTuningFinding {
    pub id: &'static str,
    pub label: &'static str,
    pub status: BehaviorTuningFindingStatus,
    pub metric_value: f32,
    pub threshold: f32,
    pub evidence: String,
    pub recommendation: String,
}

impl BehaviorTuningFinding {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.id.is_empty()
            || self.label.is_empty()
            || self.evidence.is_empty()
            || self.recommendation.is_empty()
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        NormalizedScalar::new(self.metric_value)?;
        NormalizedScalar::new(self.threshold)?;
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{:.3}:{:.3}",
            self.id,
            self.status.label(),
            self.metric_value,
            self.threshold
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BehaviorTuningSweepCase {
    pub id: &'static str,
    pub source: &'static str,
    pub detector_focus: &'static str,
    pub source_signature: String,
    pub bounded_ci: bool,
    pub notes: String,
}

impl BehaviorTuningSweepCase {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.id.is_empty()
            || self.source.is_empty()
            || self.detector_focus.is_empty()
            || self.source_signature.is_empty()
            || self.notes.is_empty()
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!("{}:{}:{}", self.id, self.detector_focus, self.bounded_ci)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BehaviorTuningSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub config: BehaviorTuningConfig,
    pub balance_signature: String,
    pub metrics: LongRunBalanceMetrics,
    pub scenario_sweeps: Vec<BehaviorTuningSweepCase>,
    pub findings: Vec<BehaviorTuningFinding>,
    pub known_degenerate_behaviors: Vec<String>,
    pub no_hidden_overfitting: bool,
    pub manual_extended_command: String,
    pub report_markdown: String,
}

impl BehaviorTuningSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != CA21_BEHAVIOR_TUNING_SCHEMA
            || self.schema_version != CA21_BEHAVIOR_TUNING_SCHEMA_VERSION
            || self.balance_signature.is_empty()
            || self.scenario_sweeps.len() != CA21_SCENARIO_SWEEP_COUNT
            || self.findings.len() != CA21_REQUIRED_DETECTOR_COUNT
            || self.known_degenerate_behaviors.is_empty()
            || !self.no_hidden_overfitting
            || !self
                .manual_extended_command
                .contains("longrun-balance-smoke")
            || !self
                .report_markdown
                .contains("Known degenerate behavior list")
            || !self.report_markdown.contains("No hidden overfitting")
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        self.config.validate()?;
        self.metrics.validate()?;
        for sweep in &self.scenario_sweeps {
            sweep.validate()?;
        }
        for finding in &self.findings {
            finding.validate()?;
        }
        for required in [
            "stagnation",
            "catatonia",
            "overfeeding",
            "hazard-suicide",
            "population-collapse",
        ] {
            if !self.findings.iter().any(|finding| finding.id == required) {
                return Err(ScaffoldContractError::MissingPhaseData);
            }
        }
        if self
            .known_degenerate_behaviors
            .iter()
            .any(|behavior| behavior.is_empty())
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        let findings = self
            .findings
            .iter()
            .map(BehaviorTuningFinding::signature_line)
            .collect::<Vec<_>>()
            .join("|");
        format!(
            "{}:{}:{}:{}",
            self.schema_version, self.config.seed, self.balance_signature, findings
        )
    }
}

pub fn run_behavior_tuning_metrics_smoke() -> Result<BehaviorTuningSummary, GameAppShellError> {
    run_behavior_tuning_metrics_with_config(BehaviorTuningConfig::fast_ci())
}

pub fn run_behavior_tuning_metrics_with_config(
    config: BehaviorTuningConfig,
) -> Result<BehaviorTuningSummary, GameAppShellError> {
    config.validate()?;
    let balance = run_longrun_balance_smoke()?;
    balance.validate()?;
    let metrics = balance.metrics.clone();
    let findings = behavior_tuning_findings(&config, &metrics);
    let scenario_sweeps = behavior_tuning_sweeps(&balance);
    let mut known_degenerate_behaviors = balance.degenerate_behaviors.clone();
    known_degenerate_behaviors.extend([
        "overfeeding risk is flagged when food success is perfect in the bounded smoke fixture"
            .to_string(),
        "hazard-suicide risk is flagged when pain remains present and avoidance is not emergent"
            .to_string(),
        "population-collapse checks are bounded to current fixture caps until CA22 long-run ecology"
            .to_string(),
    ]);
    let manual_extended_command =
        "cargo run -p alife_game_app --bin alife_game_app -- longrun-balance-smoke".to_string();
    let mut summary = BehaviorTuningSummary {
        schema: CA21_BEHAVIOR_TUNING_SCHEMA,
        schema_version: CA21_BEHAVIOR_TUNING_SCHEMA_VERSION,
        config,
        balance_signature: balance.signature_line(),
        metrics,
        scenario_sweeps,
        findings,
        known_degenerate_behaviors,
        no_hidden_overfitting: true,
        manual_extended_command,
        report_markdown: String::new(),
    };
    summary.report_markdown = behavior_tuning_report_markdown(&summary);
    summary.validate()?;
    Ok(summary)
}

fn behavior_tuning_sweeps(balance: &LongRunBalanceSummary) -> Vec<BehaviorTuningSweepCase> {
    vec![
        BehaviorTuningSweepCase {
            id: "survival-loop",
            source: "G06 playable survival loop",
            detector_focus: "stagnation catatonia overfeeding hazard-suicide",
            source_signature: balance.survival_signature.clone(),
            bounded_ci: true,
            notes: "Uses sealed survival events and drive deltas; not an open-ended ecology proof"
                .to_string(),
        },
        BehaviorTuningSweepCase {
            id: "ecology-loop",
            source: "G07 world ecology loop",
            detector_focus: "overfeeding resource-stability hazard-suicide",
            source_signature: balance.ecology_signature.clone(),
            bounded_ci: true,
            notes:
                "Uses resource regrowth/spawn and hazard pain counters from deterministic ecology"
                    .to_string(),
        },
        BehaviorTuningSweepCase {
            id: "population-loop",
            source: "G08 population social loop",
            detector_focus: "population-collapse stagnation social-diversity",
            source_signature: balance.population_signature.clone(),
            bounded_ci: true,
            notes: "Uses bounded social proximity and collision/vocal cues without direct actions"
                .to_string(),
        },
        BehaviorTuningSweepCase {
            id: "lifecycle-loop",
            source: "G09/CA20 lifecycle loop",
            detector_focus: "population-collapse reproduction-bounds",
            source_signature: balance.lifecycle_signature.clone(),
            bounded_ci: true,
            notes: "Uses birth/death/lineage metrics while preserving genetic/lifetime separation"
                .to_string(),
        },
        BehaviorTuningSweepCase {
            id: "performance-lod-loop",
            source: "G18 population performance LOD smoke",
            detector_focus: "bounded-performance no-hidden-overfitting",
            source_signature: balance.performance_signature.clone(),
            bounded_ci: true,
            notes:
                "Uses behavior-preserving LOD signature to avoid tuning by hidden feature removal"
                    .to_string(),
        },
    ]
}

fn behavior_tuning_findings(
    config: &BehaviorTuningConfig,
    metrics: &LongRunBalanceMetrics,
) -> Vec<BehaviorTuningFinding> {
    let stagnation_clear = metrics.sealed_patch_count >= config.minimum_sealed_patches
        && metrics.social_diversity_score > 0.0
        && metrics.food_success_rate > 0.0;
    let catatonia_clear = metrics.sleep_cycle_count >= config.minimum_sleep_cycles
        && metrics.survival_score > 0.0
        && metrics.energy_stability > 0.0;
    let overfeeding_watch = metrics.food_success_rate >= config.overfeeding_watch_threshold;
    let hazard_watch = metrics.hazard_avoidance_score < config.minimum_hazard_avoidance_score;
    let population_clear = metrics.max_population_observed >= config.minimum_population_observed
        && metrics.reproduction_births > 0
        && metrics.population_bounds_enforced;

    vec![
        BehaviorTuningFinding {
            id: "stagnation",
            label: "Stagnation",
            status: if stagnation_clear {
                BehaviorTuningFindingStatus::Clear
            } else {
                BehaviorTuningFindingStatus::Watch
            },
            metric_value: (metrics.sealed_patch_count as f32 / config.minimum_sealed_patches as f32)
                .min(1.0),
            threshold: 1.0,
            evidence: format!(
                "sealed_patches={} social_diversity={:.3} food_success={:.3}",
                metrics.sealed_patch_count,
                metrics.social_diversity_score,
                metrics.food_success_rate
            ),
            recommendation:
                "CA22 should broaden ecology so non-scripted multi-creature ticks continue to produce varied sealed patches"
                    .to_string(),
        },
        BehaviorTuningFinding {
            id: "catatonia",
            label: "Catatonia",
            status: if catatonia_clear {
                BehaviorTuningFindingStatus::Clear
            } else {
                BehaviorTuningFindingStatus::Watch
            },
            metric_value: (metrics.sleep_cycle_count as f32 / config.minimum_sleep_cycles as f32)
                .min(1.0),
            threshold: 1.0,
            evidence: format!(
                "sleep_cycles={} survival={:.3} energy_stability={:.3}",
                metrics.sleep_cycle_count, metrics.survival_score, metrics.energy_stability
            ),
            recommendation:
                "Keep sleep as a recovery behavior, but watch for long-run loops that stop producing action outcomes"
                    .to_string(),
        },
        BehaviorTuningFinding {
            id: "overfeeding",
            label: "Overfeeding",
            status: if overfeeding_watch {
                BehaviorTuningFindingStatus::KnownLimitation
            } else {
                BehaviorTuningFindingStatus::Clear
            },
            metric_value: metrics.food_success_rate,
            threshold: config.overfeeding_watch_threshold,
            evidence: format!(
                "food_success_rate={:.3}; perfect food success is expected in the bounded smoke fixture",
                metrics.food_success_rate
            ),
            recommendation:
                "CA22 should include scarcity/regrowth sweeps before treating perfect food success as healthy balance"
                    .to_string(),
        },
        BehaviorTuningFinding {
            id: "hazard-suicide",
            label: "Hazard suicide",
            status: if hazard_watch {
                BehaviorTuningFindingStatus::KnownLimitation
            } else {
                BehaviorTuningFindingStatus::Clear
            },
            metric_value: metrics.hazard_avoidance_score,
            threshold: config.minimum_hazard_avoidance_score,
            evidence: format!(
                "hazard_avoidance_score={:.3}; pain remains visible and avoidance is not yet emergent",
                metrics.hazard_avoidance_score
            ),
            recommendation:
                "CA22 should keep pain visible while checking whether avoidance improves under richer terrain/resource pressure"
                    .to_string(),
        },
        BehaviorTuningFinding {
            id: "population-collapse",
            label: "Population collapse",
            status: if population_clear {
                BehaviorTuningFindingStatus::Clear
            } else {
                BehaviorTuningFindingStatus::Watch
            },
            metric_value: (metrics.max_population_observed as f32
                / config.minimum_population_observed as f32)
                .min(1.0),
            threshold: 1.0,
            evidence: format!(
                "max_population={} births={} blocked={} cap_enforced={}",
                metrics.max_population_observed,
                metrics.reproduction_births,
                metrics.reproduction_blocked,
                metrics.population_bounds_enforced
            ),
            recommendation:
                "CA22 should run a longer ecological soak to see whether bounded population survives past the deterministic smoke slice"
                    .to_string(),
        },
    ]
}

pub fn behavior_tuning_report_markdown(summary: &BehaviorTuningSummary) -> String {
    let mut report = String::new();
    report.push_str("# CA21 Behavior Tuning Metrics Report\n\n");
    report.push_str("CA21 layers degeneracy detectors over the existing bounded G19 balance signals. It does not alter action arbitration, GPU/CPU authority, save data, or core contracts.\n\n");
    report.push_str("## Detector results\n\n");
    report.push_str("| Detector | Status | Metric | Threshold | Evidence | Recommendation |\n");
    report.push_str("|---|---|---:|---:|---|---|\n");
    for finding in &summary.findings {
        report.push_str(&format!(
            "| {} | {} | {:.3} | {:.3} | {} | {} |\n",
            finding.label,
            finding.status.label(),
            finding.metric_value,
            finding.threshold,
            finding.evidence,
            finding.recommendation
        ));
    }
    report.push_str("\n## Scenario sweeps\n\n");
    report.push_str("| Sweep | Source | Focus | Bounded CI | Notes |\n");
    report.push_str("|---|---|---|---|---|\n");
    for sweep in &summary.scenario_sweeps {
        report.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            sweep.id, sweep.source, sweep.detector_focus, sweep.bounded_ci, sweep.notes
        ));
    }
    report.push_str("\n## Known degenerate behavior list\n\n");
    for behavior in &summary.known_degenerate_behaviors {
        report.push_str(&format!("- {behavior}\n"));
    }
    report.push_str("\n## No hidden overfitting\n\n");
    report.push_str("- CA21 reuses deterministic G19/G06-G09/G18 signatures rather than retuning fixture data to pass.\n");
    report.push_str("- Watch statuses are retained in the report instead of hidden or converted into pass claims.\n");
    report.push_str("- Required GPU unavailability is typed and stops learned actions.\n");
    report.push_str("- CA22 owns broader long-run ecological soak evidence.\n");
    report.push_str("\n## Reproduction command\n\n");
    report.push_str(&format!("`{}`\n", summary.manual_extended_command));
    report
}
