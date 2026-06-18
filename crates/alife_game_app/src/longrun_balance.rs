//! G19 long-run balance smoke and report aggregation.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub struct LongRunBalanceConfig {
    pub schema: &'static str,
    pub schema_version: u16,
    pub seed: u64,
    pub cycles: u32,
    pub population_cap: usize,
    pub resource_cap: usize,
    pub extended_manual: bool,
}

impl LongRunBalanceConfig {
    pub const fn fast_ci() -> Self {
        Self {
            schema: G19_LONG_RUN_BALANCE_SCHEMA,
            schema_version: G19_LONG_RUN_BALANCE_SCHEMA_VERSION,
            seed: 19_190,
            cycles: G19_FAST_BALANCE_CYCLES,
            population_cap: G08_MAX_POPULATION_CAP,
            resource_cap: 64,
            extended_manual: false,
        }
    }

    pub const fn extended_manual() -> Self {
        Self {
            schema: G19_LONG_RUN_BALANCE_SCHEMA,
            schema_version: G19_LONG_RUN_BALANCE_SCHEMA_VERSION,
            seed: 19_191,
            cycles: G19_EXTENDED_BALANCE_CYCLES,
            population_cap: G08_MAX_POPULATION_CAP,
            resource_cap: 64,
            extended_manual: true,
        }
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G19_LONG_RUN_BALANCE_SCHEMA
            || self.schema_version != G19_LONG_RUN_BALANCE_SCHEMA_VERSION
            || self.cycles == 0
            || self.cycles > G19_EXTENDED_BALANCE_CYCLES
            || self.population_cap == 0
            || self.population_cap > G08_MAX_POPULATION_CAP
            || self.resource_cap == 0
            || self.resource_cap > 256
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LongRunBalanceMetrics {
    pub survival_score: f32,
    pub energy_stability: f32,
    pub food_success_rate: f32,
    pub hazard_avoidance_score: f32,
    pub sleep_cycle_count: u32,
    pub reproduction_births: u32,
    pub reproduction_blocked: u32,
    pub social_diversity_score: f32,
    pub sealed_patch_count: usize,
    pub packed_record_count: usize,
    pub memory_record_count: usize,
    pub topology_concept_count: usize,
    pub unresolved_gap_count: usize,
    pub max_population_observed: usize,
    pub max_resources_observed: usize,
    pub no_unsealed_learning: bool,
    pub invalid_id_rejected: bool,
    pub finite_values: bool,
    pub population_bounds_enforced: bool,
    pub resource_bounds_enforced: bool,
}

impl LongRunBalanceMetrics {
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
        if self.sleep_cycle_count == 0
            || self.sealed_patch_count == 0
            || self.packed_record_count == 0
            || self.memory_record_count == 0
            || self.topology_concept_count == 0
            || self.max_population_observed == 0
            || self.max_population_observed > G08_MAX_POPULATION_CAP
            || self.max_resources_observed == 0
            || self.max_resources_observed > 64
            || !self.no_unsealed_learning
            || !self.invalid_id_rejected
            || !self.finite_values
            || !self.population_bounds_enforced
            || !self.resource_bounds_enforced
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LongRunBalanceSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub config: LongRunBalanceConfig,
    pub survival_signature: String,
    pub ecology_signature: String,
    pub population_signature: String,
    pub lifecycle_signature: String,
    pub performance_signature: String,
    pub metrics: LongRunBalanceMetrics,
    pub degenerate_behaviors: Vec<String>,
    pub constraints: Vec<String>,
    pub manual_extended_command: String,
    pub report_markdown: String,
}

impl LongRunBalanceSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G19_LONG_RUN_BALANCE_SCHEMA
            || self.schema_version != G19_LONG_RUN_BALANCE_SCHEMA_VERSION
            || self.survival_signature.is_empty()
            || self.ecology_signature.is_empty()
            || self.population_signature.is_empty()
            || self.lifecycle_signature.is_empty()
            || self.performance_signature.is_empty()
            || self.degenerate_behaviors.is_empty()
            || self.constraints.is_empty()
            || !self.manual_extended_command.contains("--ignored")
            || !self
                .manual_extended_command
                .contains("g19_manual_extended_balance_run")
            || !self.report_markdown.contains("Known degenerate behaviors")
            || !self.report_markdown.contains("manual extended")
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        if self
            .degenerate_behaviors
            .iter()
            .chain(self.constraints.iter())
            .any(|line| line.is_empty())
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        self.config.validate()?;
        self.metrics.validate()?;
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{:.3}:{:.3}:{:.3}:{:.3}:{}:{}",
            self.schema_version,
            self.config.seed,
            self.config.cycles,
            self.metrics.sealed_patch_count,
            self.metrics.survival_score,
            self.metrics.energy_stability,
            self.metrics.food_success_rate,
            self.metrics.hazard_avoidance_score,
            self.metrics.sleep_cycle_count,
            self.metrics.social_diversity_score
        )
    }
}

pub fn run_longrun_balance_smoke() -> Result<LongRunBalanceSummary, GameAppShellError> {
    run_longrun_balance_with_config(LongRunBalanceConfig::fast_ci())
}

pub fn run_longrun_balance_with_config(
    config: LongRunBalanceConfig,
) -> Result<LongRunBalanceSummary, GameAppShellError> {
    config.validate()?;
    let survival = run_playable_survival_loop_smoke()?;
    let ecology = run_world_ecology_loop_smoke()?;
    let population = run_population_social_loop_smoke()?;
    let lifecycle = run_lifecycle_lineage_smoke()?;
    let performance =
        run_population_performance_lod_smoke(&AppShellLaunchConfig::from_p34_fixture_root(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../alife_world/tests/fixtures/p34"),
        ))?;

    survival.validate()?;
    ecology.validate()?;
    population.validate()?;
    lifecycle.validate()?;
    performance.validate()?;

    let metrics = compute_balance_metrics(
        &config,
        &survival,
        &ecology,
        &population,
        &lifecycle,
        &performance,
    )?;
    let degenerate_behaviors = vec![
        "hazard contact is still scripted in the smoke loop; G19 keeps it visible instead of hiding the pain metric".to_string(),
        "manual upper population tiers remain expected-slow and are not normal CI gates".to_string(),
        "current fast balance smoke proves bounded deterministic loops, not full player fun across every ecology".to_string(),
    ];
    let constraints = vec![
        "CPU/headless path remains the correctness oracle".to_string(),
        "G18 LOD policy preserves sensory, motor, homeostasis, and action arbitration priority"
            .to_string(),
        "GPU fallback reports are not converted into product GPU performance claims".to_string(),
        "population and resource caps are finite and validated".to_string(),
    ];
    let manual_extended_command = "cargo test -p alife_game_app --test app_shell g19_manual_extended_balance_run -- --ignored --nocapture".to_string();
    let mut summary = LongRunBalanceSummary {
        schema: G19_LONG_RUN_BALANCE_SCHEMA,
        schema_version: G19_LONG_RUN_BALANCE_SCHEMA_VERSION,
        config,
        survival_signature: survival.signature_line(),
        ecology_signature: ecology.signature_line(),
        population_signature: population.signature_line(),
        lifecycle_signature: lifecycle.signature_line(),
        performance_signature: performance.signature_line(),
        metrics,
        degenerate_behaviors,
        constraints,
        manual_extended_command,
        report_markdown: String::new(),
    };
    summary.report_markdown = balance_report_markdown(&summary);
    summary.validate()?;
    Ok(summary)
}

fn compute_balance_metrics(
    config: &LongRunBalanceConfig,
    survival: &PlayableSurvivalLoopSummary,
    ecology: &PlayableEcologyLoopSummary,
    population: &PopulationSocialLoopSummary,
    lifecycle: &LifecycleLineageSummary,
    performance: &PopulationPerformanceOverlaySummary,
) -> Result<LongRunBalanceMetrics, ScaffoldContractError> {
    let survival_success = survival.events.iter().filter(|event| event.success).count() as f32
        / survival.events.len() as f32;
    let brain_atp_values = survival
        .events
        .iter()
        .map(|event| event.brain_atp_after)
        .collect::<Vec<_>>();
    let mean_energy = brain_atp_values.iter().sum::<f32>() / brain_atp_values.len() as f32;
    let energy_variance = brain_atp_values
        .iter()
        .map(|value| {
            let delta = value - mean_energy;
            delta * delta
        })
        .sum::<f32>()
        / brain_atp_values.len() as f32;
    let energy_stability = (1.0 - energy_variance.sqrt()).clamp(0.0, 1.0);
    let food_attempts = survival
        .events
        .iter()
        .filter(|event| event.kind == PlayableSurvivalEventKind::FoodConsumed)
        .count();
    let food_success_rate = if food_attempts == 0 {
        0.0
    } else {
        survival
            .events
            .iter()
            .filter(|event| event.kind == PlayableSurvivalEventKind::FoodConsumed && event.success)
            .count() as f32
            / food_attempts as f32
    };
    let hazard_pressure = survival
        .events
        .iter()
        .map(|event| event.pain_after)
        .fold(ecology.hazard_pain, f32::max);
    let hazard_avoidance_score = (1.0 - hazard_pressure).clamp(0.0, 1.0);
    let sleep_cycle_count = survival
        .events
        .iter()
        .filter(|event| event.kind == PlayableSurvivalEventKind::RestSleep && event.success)
        .count() as u32;
    let reproduction_births = lifecycle.metrics.births as u32;
    let reproduction_blocked = lifecycle.metrics.reproduction_blocked_count as u32;
    let social_diversity_score = social_diversity(population);
    let sealed_patch_count = survival.sealed_patch_count
        + ecology.sealed_patch_count
        + population.metrics.sealed_patch_count
        + lifecycle.metrics.sealed_patch_count
        + performance.sealed_patch_count;
    let packed_record_count = survival.packed_record_count
        + ecology.packed_record_count
        + population.metrics.packed_record_count
        + lifecycle.metrics.packed_record_count
        + performance.packed_record_count;
    let max_population_observed = population
        .creature_count
        .max(lifecycle.metrics.living_population)
        .max(performance.population_creatures);
    let max_resources_observed = ecology.metrics.active_resources.max(
        survival
            .world_signature
            .iter()
            .filter(|line| line.contains("Food"))
            .count(),
    );
    let no_unsealed_learning = survival.tick_summaries.iter().all(|tick| tick.patch_sealed)
        && ecology.tick_summaries.iter().all(|tick| tick.patch_sealed)
        && population
            .tick_records
            .iter()
            .all(|record| record.tick_summary.patch_sealed);
    let invalid_id_rejected = PopulationLoopConfig {
        population_cap: 1,
        ..PopulationLoopConfig::two_creature_smoke()
            .map_err(|_| ScaffoldContractError::MissingPhaseData)?
    }
    .validate()
    .is_err();
    let metrics = LongRunBalanceMetrics {
        survival_score: survival_success,
        energy_stability,
        food_success_rate,
        hazard_avoidance_score,
        sleep_cycle_count,
        reproduction_births,
        reproduction_blocked,
        social_diversity_score,
        sealed_patch_count,
        packed_record_count,
        memory_record_count: survival.memory_record_count,
        topology_concept_count: survival.topology_concept_count,
        unresolved_gap_count: survival.unresolved_gap_count,
        max_population_observed,
        max_resources_observed,
        no_unsealed_learning,
        invalid_id_rejected,
        finite_values: [
            survival_success,
            energy_stability,
            food_success_rate,
            hazard_avoidance_score,
            social_diversity_score,
        ]
        .into_iter()
        .all(f32::is_finite),
        population_bounds_enforced: max_population_observed <= config.population_cap,
        resource_bounds_enforced: max_resources_observed <= config.resource_cap,
    };
    metrics.validate()?;
    Ok(metrics)
}

fn social_diversity(population: &PopulationSocialLoopSummary) -> f32 {
    let vocal = usize::from(population.metrics.vocal_tokens_heard > 0);
    let trust = usize::from(
        population
            .tick_records
            .iter()
            .any(|record| record.trust_cues_seen > 0),
    );
    let fear = usize::from(
        population
            .tick_records
            .iter()
            .any(|record| record.fear_cues_seen > 0),
    );
    let contact = usize::from(population.metrics.collision_feedback_count > 0);
    (vocal + trust + fear + contact) as f32 / 4.0
}

pub fn balance_report_markdown(summary: &LongRunBalanceSummary) -> String {
    let mut report = String::new();
    report.push_str("# G19 Long-run Balance Report\n\n");
    report.push_str("| Metric | Value |\n|---|---:|\n");
    report.push_str(&format!(
        "| Survival score | {:.3} |\n| Energy stability | {:.3} |\n| Food success rate | {:.3} |\n| Hazard avoidance score | {:.3} |\n| Sleep cycles | {} |\n| Reproduction births | {} |\n| Reproduction blocked | {} |\n| Social diversity | {:.3} |\n| Sealed patches | {} |\n| Max population observed | {} |\n| Max resources observed | {} |\n",
        summary.metrics.survival_score,
        summary.metrics.energy_stability,
        summary.metrics.food_success_rate,
        summary.metrics.hazard_avoidance_score,
        summary.metrics.sleep_cycle_count,
        summary.metrics.reproduction_births,
        summary.metrics.reproduction_blocked,
        summary.metrics.social_diversity_score,
        summary.metrics.sealed_patch_count,
        summary.metrics.max_population_observed,
        summary.metrics.max_resources_observed
    ));
    report.push_str("\n## Known degenerate behaviors\n\n");
    for behavior in &summary.degenerate_behaviors {
        report.push_str(&format!("- {behavior}\n"));
    }
    report.push_str("\n## Constraints\n\n");
    for constraint in &summary.constraints {
        report.push_str(&format!("- {constraint}\n"));
    }
    report.push_str("\n## Manual extended command\n\n");
    report.push_str(&format!("`{}`\n", summary.manual_extended_command));
    report.push_str("\nThe manual extended command is intentionally ignored by default; normal CI keeps only the fast balance smoke.\n");
    report
}
