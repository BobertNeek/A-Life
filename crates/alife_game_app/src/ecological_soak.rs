//! CA22 long-run ecological soak and balancing evidence.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcologicalSoakMode {
    FastCi,
    Manual10k,
}

impl EcologicalSoakMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::FastCi => "fast-ci",
            Self::Manual10k => "manual-10k",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EcologicalSoakConfig {
    pub schema: &'static str,
    pub schema_version: u16,
    pub seed: u64,
    pub headless_ticks: u32,
    pub graphical_ticks: u32,
    pub report_every: u32,
    pub population_cap: usize,
    pub resource_cap: usize,
    pub mode: EcologicalSoakMode,
}

impl EcologicalSoakConfig {
    pub const fn fast_ci() -> Self {
        Self {
            schema: CA22_ECOLOGICAL_SOAK_SCHEMA,
            schema_version: CA22_ECOLOGICAL_SOAK_SCHEMA_VERSION,
            seed: 22_220,
            headless_ticks: CA22_FAST_HEADLESS_TICKS,
            graphical_ticks: CA22_GRAPHICAL_BOUNDED_TICKS,
            report_every: 100,
            population_cap: G08_MAX_POPULATION_CAP,
            resource_cap: 64,
            mode: EcologicalSoakMode::FastCi,
        }
    }

    pub const fn manual_10k() -> Self {
        Self {
            schema: CA22_ECOLOGICAL_SOAK_SCHEMA,
            schema_version: CA22_ECOLOGICAL_SOAK_SCHEMA_VERSION,
            seed: 22_221,
            headless_ticks: CA22_MANUAL_HEADLESS_TICKS,
            graphical_ticks: CA22_GRAPHICAL_BOUNDED_TICKS,
            report_every: 1_000,
            population_cap: G08_MAX_POPULATION_CAP,
            resource_cap: 64,
            mode: EcologicalSoakMode::Manual10k,
        }
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != CA22_ECOLOGICAL_SOAK_SCHEMA
            || self.schema_version != CA22_ECOLOGICAL_SOAK_SCHEMA_VERSION
            || self.headless_ticks == 0
            || self.headless_ticks > CA22_MAX_MANUAL_SOAK_TICKS
            || self.graphical_ticks == 0
            || self.graphical_ticks > CA22_GRAPHICAL_BOUNDED_TICKS
            || self.report_every == 0
            || self.report_every > self.headless_ticks
            || self.population_cap == 0
            || self.population_cap > G08_MAX_POPULATION_CAP
            || self.resource_cap == 0
            || self.resource_cap > 256
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if self.mode == EcologicalSoakMode::FastCi && self.headless_ticks > CA22_FAST_HEADLESS_TICKS
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EcologicalSoakMetrics {
    pub headless_ticks_requested: u32,
    pub headless_ticks_completed: u32,
    pub first_failure_tick: Option<u32>,
    pub graphical_ticks_bounded: u32,
    pub ecology_metric_samples: u32,
    pub survival_score: f32,
    pub energy_stability: f32,
    pub food_success_rate: f32,
    pub hazard_avoidance_score: f32,
    pub sleep_cycles: u32,
    pub reproduction_births: u32,
    pub reproduction_blocked: u32,
    pub social_diversity_score: f32,
    pub sealed_patch_count: usize,
    pub packed_record_count: usize,
    pub max_population_observed: usize,
    pub max_resources_observed: usize,
    pub resources_regrown_or_spawned: bool,
    pub population_bounds_enforced: bool,
    pub resource_bounds_enforced: bool,
    pub no_unsealed_learning: bool,
    pub finite_values: bool,
}

impl EcologicalSoakMetrics {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        for value in [
            self.survival_score,
            self.energy_stability,
            self.food_success_rate,
            self.hazard_avoidance_score,
            self.social_diversity_score,
        ] {
            NormalizedScalar::new(value)?;
        }
        if self.headless_ticks_requested == 0
            || self.headless_ticks_requested > CA22_MAX_MANUAL_SOAK_TICKS
            || self.headless_ticks_completed == 0
            || self.headless_ticks_completed > self.headless_ticks_requested
            || (self.first_failure_tick.is_none()
                && self.headless_ticks_completed != self.headless_ticks_requested)
            || self
                .first_failure_tick
                .is_some_and(|tick| tick == 0 || tick > self.headless_ticks_requested)
            || self.graphical_ticks_bounded == 0
            || self.graphical_ticks_bounded > CA22_GRAPHICAL_BOUNDED_TICKS
            || self.ecology_metric_samples == 0
            || self.sleep_cycles == 0
            || self.sealed_patch_count == 0
            || self.packed_record_count == 0
            || self.max_population_observed == 0
            || self.max_population_observed > G08_MAX_POPULATION_CAP
            || self.max_resources_observed == 0
            || self.max_resources_observed > 64
            || !self.resources_regrown_or_spawned
            || !self.population_bounds_enforced
            || !self.resource_bounds_enforced
            || !self.no_unsealed_learning
            || !self.finite_values
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EcologicalSoakFinding {
    pub id: &'static str,
    pub status: BehaviorTuningFindingStatus,
    pub evidence: String,
    pub remaining_issue: String,
}

impl EcologicalSoakFinding {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.id.is_empty() || self.evidence.is_empty() || self.remaining_issue.is_empty() {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!("{}:{}", self.id, self.status.label())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EcologicalSoakSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub config: EcologicalSoakConfig,
    pub metrics: EcologicalSoakMetrics,
    pub balance_signature: String,
    pub behavior_signature: String,
    pub graphical_signature: String,
    pub findings: Vec<EcologicalSoakFinding>,
    pub config_first_tuning: bool,
    pub full_emergent_ecology_claim: bool,
    pub gpu_product_claim: &'static str,
    pub gpu_authority_preserved: bool,
    pub manual_10k_command: String,
    pub graphical_bounded_command: String,
    pub report_markdown: String,
}

impl EcologicalSoakSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != CA22_ECOLOGICAL_SOAK_SCHEMA
            || self.schema_version != CA22_ECOLOGICAL_SOAK_SCHEMA_VERSION
            || self.balance_signature.is_empty()
            || self.behavior_signature.is_empty()
            || self.graphical_signature.is_empty()
            || self.findings.len() != CA21_REQUIRED_DETECTOR_COUNT
            || !self.config_first_tuning
            || self.full_emergent_ecology_claim
            || self.gpu_product_claim != "GpuAuthoritative"
            || !self.gpu_authority_preserved
            || !self
                .manual_10k_command
                .contains("ca22_manual_10k_ecological_soak")
            || !self.manual_10k_command.contains("--ignored")
            || !self
                .graphical_bounded_command
                .contains("run_production_voxel_frontend.ps1")
            || !self.report_markdown.contains("Remaining issues")
            || !self.report_markdown.contains("10k")
            || !self.report_markdown.contains("config-first")
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        self.config.validate()?;
        self.metrics.validate()?;
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
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        let findings = self
            .findings
            .iter()
            .map(EcologicalSoakFinding::signature_line)
            .collect::<Vec<_>>()
            .join("|");
        format!(
            "{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.config.seed,
            self.config.headless_ticks,
            self.metrics.sealed_patch_count,
            self.metrics.max_population_observed,
            findings
        )
    }
}

pub fn run_ecological_soak_smoke() -> Result<EcologicalSoakSummary, GameAppShellError> {
    run_ecological_soak_with_config(EcologicalSoakConfig::fast_ci())
}

pub fn run_ecological_soak_with_config(
    config: EcologicalSoakConfig,
) -> Result<EcologicalSoakSummary, GameAppShellError> {
    config.validate()?;
    let balance =
        run_longrun_balance_with_config(if config.mode == EcologicalSoakMode::Manual10k {
            LongRunBalanceConfig::extended_manual()
        } else {
            LongRunBalanceConfig::fast_ci()
        })?;
    let behavior = run_behavior_tuning_metrics_smoke()?;
    let launch = AppShellLaunchConfig::from_p34_fixture_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../alife_world/tests/fixtures/gpu_alpha"),
    );
    let graphical = run_graphical_ecology_smoke(&launch)?;
    let tick_soak = run_headless_ecology_tick_soak(&launch, &config)?;

    balance.validate()?;
    behavior.validate()?;
    graphical.validate()?;

    let metrics = ecological_soak_metrics(&config, &balance, &graphical, &tick_soak)?;
    let findings = ecological_soak_findings(&behavior, &metrics);
    let manual_10k_command = "cargo test -p alife_game_app --test app_shell ca22_manual_10k_ecological_soak -- --ignored --nocapture".to_string();
    let graphical_bounded_command = "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -SmokeSeconds 30 -BrainPolicy gpu-required -RecordPerformance".to_string();
    let mut summary = EcologicalSoakSummary {
        schema: CA22_ECOLOGICAL_SOAK_SCHEMA,
        schema_version: CA22_ECOLOGICAL_SOAK_SCHEMA_VERSION,
        config,
        metrics,
        balance_signature: balance.signature_line(),
        behavior_signature: behavior.signature_line(),
        graphical_signature: graphical.signature_line(),
        findings,
        config_first_tuning: true,
        full_emergent_ecology_claim: false,
        gpu_product_claim: "GpuAuthoritative",
        gpu_authority_preserved: true,
        manual_10k_command,
        graphical_bounded_command,
        report_markdown: String::new(),
    };
    summary.report_markdown = ecological_soak_report_markdown(&summary);
    summary.validate()?;
    Ok(summary)
}

fn ecological_soak_metrics(
    config: &EcologicalSoakConfig,
    balance: &LongRunBalanceSummary,
    graphical: &Ca19GraphicalEcologySummary,
    tick_soak: &MeasuredHeadlessEcologySoak,
) -> Result<EcologicalSoakMetrics, ScaffoldContractError> {
    let metrics = EcologicalSoakMetrics {
        headless_ticks_requested: config.headless_ticks,
        headless_ticks_completed: tick_soak.ticks_completed,
        first_failure_tick: tick_soak.first_failure_tick,
        graphical_ticks_bounded: config.graphical_ticks,
        ecology_metric_samples: tick_soak.ecology_metric_samples,
        survival_score: balance.metrics.survival_score,
        energy_stability: balance.metrics.energy_stability,
        food_success_rate: balance.metrics.food_success_rate,
        hazard_avoidance_score: balance.metrics.hazard_avoidance_score,
        sleep_cycles: balance.metrics.sleep_cycle_count,
        reproduction_births: balance.metrics.reproduction_births,
        reproduction_blocked: balance.metrics.reproduction_blocked,
        social_diversity_score: balance.metrics.social_diversity_score,
        sealed_patch_count: balance.metrics.sealed_patch_count,
        packed_record_count: balance.metrics.packed_record_count,
        max_population_observed: balance.metrics.max_population_observed,
        max_resources_observed: balance
            .metrics
            .max_resources_observed
            .max(graphical.cycled_metrics.active_resources)
            .max(tick_soak.max_resources_observed),
        resources_regrown_or_spawned: graphical.cycled_metrics.resources_regrown > 0
            || graphical.cycled_metrics.resources_spawned > 0
            || tick_soak.resources_regrown_or_spawned,
        population_bounds_enforced: balance.metrics.population_bounds_enforced
            && balance.metrics.max_population_observed <= config.population_cap,
        resource_bounds_enforced: balance.metrics.resource_bounds_enforced
            && balance.metrics.max_resources_observed <= config.resource_cap,
        no_unsealed_learning: balance.metrics.no_unsealed_learning,
        finite_values: balance.metrics.finite_values,
    };
    metrics.validate()?;
    Ok(metrics)
}

#[derive(Debug, Clone, PartialEq)]
struct MeasuredHeadlessEcologySoak {
    ticks_completed: u32,
    first_failure_tick: Option<u32>,
    ecology_metric_samples: u32,
    max_resources_observed: usize,
    resources_regrown_or_spawned: bool,
}

fn run_headless_ecology_tick_soak(
    launch: &AppShellLaunchConfig,
    config: &EcologicalSoakConfig,
) -> Result<MeasuredHeadlessEcologySoak, GameAppShellError> {
    let save = PortableSaveFile::from_json_file(&launch.save_path)?;
    let mut world = save.restore_headless_world()?;
    let mut ticks_completed = 0_u32;
    let mut first_failure_tick = None;
    let mut ecology_metric_samples = 0_u32;
    let mut max_resources_observed = world.ecology_metrics().active_resources;
    let mut resources_regrown_or_spawned = false;

    for tick in 1..=config.headless_ticks {
        world.advance_tick();
        ticks_completed = tick;

        if tick == 1 || tick % config.report_every == 0 || tick == config.headless_ticks {
            let metrics = world.ecology_metrics();
            ecology_metric_samples = ecology_metric_samples.saturating_add(1);
            max_resources_observed = max_resources_observed.max(metrics.active_resources);
            resources_regrown_or_spawned |=
                metrics.resources_regrown > 0 || metrics.resources_spawned > 0;
            if metrics.active_resources > config.resource_cap {
                first_failure_tick = Some(tick);
                break;
            }
        }
    }

    Ok(MeasuredHeadlessEcologySoak {
        ticks_completed,
        first_failure_tick,
        ecology_metric_samples,
        max_resources_observed,
        resources_regrown_or_spawned,
    })
}

fn ecological_soak_findings(
    behavior: &BehaviorTuningSummary,
    metrics: &EcologicalSoakMetrics,
) -> Vec<EcologicalSoakFinding> {
    behavior
        .findings
        .iter()
        .map(|finding| {
            let remaining_issue = match finding.id {
                "overfeeding" => {
                    "Perfect food success remains a watch item until scarcity sweeps become interactive"
                }
                "hazard-suicide" => {
                    "Hazard avoidance is still bounded evidence, not broad emergent avoidance"
                }
                "population-collapse" => {
                    "Population remains capped and deterministic; CA22 records bounds, not open ecology"
                }
                "stagnation" => {
                    "Current sealed patch count shows activity, but broader scenarios are still needed"
                }
                "catatonia" => "Sleep/rest cycles are present but not a full playability proof",
                _ => "Detector carried forward from CA21",
            };
            EcologicalSoakFinding {
                id: finding.id,
                status: finding.status,
                evidence: format!(
                    "{}; soak_ticks={} sealed={} pop={} resources={}",
                    finding.evidence,
                    metrics.headless_ticks_completed,
                    metrics.sealed_patch_count,
                    metrics.max_population_observed,
                    metrics.max_resources_observed
                ),
                remaining_issue: remaining_issue.to_string(),
            }
        })
        .collect()
}

pub fn ecological_soak_report_markdown(summary: &EcologicalSoakSummary) -> String {
    let mut report = String::new();
    report.push_str("# CA22 Long-run Ecological Soak Report\n\n");
    report.push_str("CA22 records bounded long-run ecology evidence using config-first tuning surfaces. It does not change action arbitration, GPU authority, or core contracts.\n\n");
    report.push_str("## Metrics\n\n");
    report.push_str("| Metric | Value |\n|---|---:|\n");
    report.push_str(&format!(
        "| Headless ticks requested | {} |\n| Headless ticks completed | {} |\n| First failure tick | {} |\n| Ecology metric samples | {} |\n| Graphical bounded ticks | {} |\n| Survival score | {:.3} |\n| Energy stability | {:.3} |\n| Food success rate | {:.3} |\n| Hazard avoidance score | {:.3} |\n| Sleep cycles | {} |\n| Births | {} |\n| Reproduction blocked | {} |\n| Social diversity | {:.3} |\n| Sealed patches | {} |\n| Max population | {} |\n| Max resources | {} |\n",
        summary.metrics.headless_ticks_requested,
        summary.metrics.headless_ticks_completed,
        summary
            .metrics
            .first_failure_tick
            .map(|tick| tick.to_string())
            .unwrap_or_else(|| "none".to_string()),
        summary.metrics.ecology_metric_samples,
        summary.metrics.graphical_ticks_bounded,
        summary.metrics.survival_score,
        summary.metrics.energy_stability,
        summary.metrics.food_success_rate,
        summary.metrics.hazard_avoidance_score,
        summary.metrics.sleep_cycles,
        summary.metrics.reproduction_births,
        summary.metrics.reproduction_blocked,
        summary.metrics.social_diversity_score,
        summary.metrics.sealed_patch_count,
        summary.metrics.max_population_observed,
        summary.metrics.max_resources_observed,
    ));
    report.push_str("\n## Remaining issues\n\n");
    report.push_str("| Detector | Status | Evidence | Remaining issue |\n");
    report.push_str("|---|---|---|---|\n");
    for finding in &summary.findings {
        report.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            finding.id,
            finding.status.label(),
            finding.evidence,
            finding.remaining_issue
        ));
    }
    report.push_str("\n## Boundaries\n\n");
    report.push_str(&format!(
        "- GPU product claim remains `{}`.\n",
        summary.gpu_product_claim
    ));
    report.push_str("- GPU neural authority fails closed without a learned-policy substitute.\n");
    report.push_str(
        "- Full emergent ecology and full action-authoritative GPU runtime are not claimed.\n",
    );
    report.push_str("- Tuning is config-first; no core contracts were changed for this soak.\n");
    report.push_str("\n## Reproduction commands\n\n");
    report.push_str(&format!("- Fast: `cargo run -p alife_game_app --bin alife_game_app -- ecological-soak-smoke`\n- Manual 10k: `{}`\n- Graphical bounded: `{}`\n", summary.manual_10k_command, summary.graphical_bounded_command));
    report
}
