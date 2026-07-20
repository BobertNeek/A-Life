//! v0 tooling: CPU-only benchmark tiers, budget policy data, and reports.
//!
//! This module measures deterministic headless scenarios as an early
//! population-tier harness. It intentionally does not require Bevy, wgpu, or
//! GPU devices, and it does not optimize runtime internals.

pub mod gpu_closed_loop;

use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use alife_core::{BrainClassSpec, BrainScaleTier, BrainTickStatus, LobeKind, Validate};
use alife_gpu_backend::{
    GpuRuntimeBackendStatus, GpuTierMeasurement, GpuTierPerformanceReport, GpuTierPopulation,
    P29_RUNTIME_SCHEMA_VERSION,
};
use alife_world::{ScenarioFixture, ScenarioName};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BenchmarkTier {
    pub population: u16,
    pub expected_slow_cpu_only: bool,
}

impl BenchmarkTier {
    pub const fn new(population: u16) -> Self {
        Self {
            population,
            expected_slow_cpu_only: population >= 50,
        }
    }

    pub const fn required_tiers() -> [Self; 7] {
        [
            Self::new(1),
            Self::new(10),
            Self::new(30),
            Self::new(50),
            Self::new(100),
            Self::new(250),
            Self::new(500),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BenchmarkMetricKind {
    TickTime,
    MemoryUsageEstimate,
    PatchThroughput,
    MemoryTopologyUpdateTime,
    NeuralProjectionTime,
    SleepConsolidationTime,
    ScenarioSuccessRate,
}

impl BenchmarkMetricKind {
    pub const ALL: [Self; 7] = [
        Self::TickTime,
        Self::MemoryUsageEstimate,
        Self::PatchThroughput,
        Self::MemoryTopologyUpdateTime,
        Self::NeuralProjectionTime,
        Self::SleepConsolidationTime,
        Self::ScenarioSuccessRate,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidencyClass {
    Hot,
    Warm,
    Cold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetProfile {
    SensoryMotor,
    EndocrineHomeostasis,
    ActionArbitration,
    OnlinePlasticity,
    MemoryExpectancy,
    TopologyConcepts,
    SleepConsolidation,
    LoggingExport,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpdateRateBand {
    pub residency: ResidencyClass,
    pub target: TargetProfile,
    pub hz: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpdateRatePolicy {
    bands: Vec<UpdateRateBand>,
}

impl UpdateRatePolicy {
    pub fn v1_defaults() -> Self {
        let mut bands = Vec::new();
        for (residency, values) in [
            (
                ResidencyClass::Hot,
                [60.0, 20.0, 60.0, 30.0, 10.0, 3.0, 0.0, 60.0],
            ),
            (
                ResidencyClass::Warm,
                [20.0, 5.0, 20.0, 5.0, 2.0, 0.5, 0.0, 10.0],
            ),
            (
                ResidencyClass::Cold,
                [0.0, 0.5, 0.0, 0.0, 0.1, 0.0, 0.0, 1.0],
            ),
        ] {
            for (target, hz) in TargetProfile::ALL.into_iter().zip(values) {
                bands.push(UpdateRateBand {
                    residency,
                    target,
                    hz,
                });
            }
        }
        Self { bands }
    }

    pub fn with_rate_hz(
        mut self,
        residency: ResidencyClass,
        target: TargetProfile,
        hz: f32,
    ) -> Self {
        let hz = hz.max(0.0);
        if let Some(band) = self
            .bands
            .iter_mut()
            .find(|band| band.residency == residency && band.target == target)
        {
            band.hz = hz;
        } else {
            self.bands.push(UpdateRateBand {
                residency,
                target,
                hz,
            });
        }
        self
    }

    pub fn rate_hz(&self, residency: ResidencyClass, target: TargetProfile) -> f32 {
        self.bands
            .iter()
            .find(|band| band.residency == residency && band.target == target)
            .map_or(0.0, |band| band.hz)
    }

    pub fn bands(&self) -> &[UpdateRateBand] {
        &self.bands
    }
}

impl TargetProfile {
    pub const ALL: [Self; 8] = [
        Self::SensoryMotor,
        Self::EndocrineHomeostasis,
        Self::ActionArbitration,
        Self::OnlinePlasticity,
        Self::MemoryExpectancy,
        Self::TopologyConcepts,
        Self::SleepConsolidation,
        Self::LoggingExport,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThrottlingThresholds {
    pub nonessential_decimation_threshold: f32,
    pub warm_cadence_threshold: f32,
    pub sleep_only_threshold: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComputeBudgetPolicy {
    pub tier: BrainScaleTier,
    pub active_synapse_budget: u32,
    pub active_tile_budget: u32,
    pub essential_lobes: Vec<LobeKind>,
    pub nonessential_lobes: Vec<LobeKind>,
    pub throttling: ThrottlingThresholds,
    pub fallback_update_frequency_hz: f32,
}

impl ComputeBudgetPolicy {
    pub fn for_tier(tier: BrainScaleTier) -> Result<Self, alife_core::ScaffoldContractError> {
        let spec = BrainClassSpec::try_for_tier(tier)?;
        let essential_lobes = spec.compute_budget.essential_lobes.clone();
        let nonessential_lobes = spec
            .lobe_regions()
            .filter(|region| region.enabled && !essential_lobes.contains(&region.kind))
            .map(|region| region.kind)
            .collect();
        Ok(Self {
            tier,
            active_synapse_budget: spec.compute_budget.max_active_synapses,
            active_tile_budget: spec.compute_budget.max_active_tiles,
            essential_lobes,
            nonessential_lobes,
            throttling: ThrottlingThresholds {
                nonessential_decimation_threshold: 0.70,
                warm_cadence_threshold: 0.85,
                sleep_only_threshold: 0.95,
            },
            fallback_update_frequency_hz: 5.0,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkHarnessConfig {
    pub tiers: Vec<BenchmarkTier>,
    pub brain_tier: BrainScaleTier,
    pub iteration_count: u16,
    pub update_rates: UpdateRatePolicy,
}

impl BenchmarkHarnessConfig {
    pub fn smoke() -> Self {
        Self {
            tiers: vec![BenchmarkTier::new(1), BenchmarkTier::new(10)],
            brain_tier: BrainScaleTier::Nano512,
            iteration_count: 1,
            update_rates: UpdateRatePolicy::v1_defaults(),
        }
    }

    pub fn manual_full() -> Self {
        Self {
            tiers: BenchmarkTier::required_tiers().to_vec(),
            ..Self::smoke()
        }
    }

    pub fn with_tiers<const N: usize>(mut self, tiers: [BenchmarkTier; N]) -> Self {
        self.tiers = tiers.to_vec();
        self
    }

    pub const fn with_iteration_count(mut self, iteration_count: u16) -> Self {
        self.iteration_count = iteration_count;
        self
    }

    pub const fn with_brain_tier(mut self, brain_tier: BrainScaleTier) -> Self {
        self.brain_tier = brain_tier;
        self
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BenchmarkMetrics {
    pub tick_time: Duration,
    pub memory_usage_estimate_bytes: u64,
    pub patch_throughput_per_second: f64,
    pub memory_topology_update_time: Duration,
    pub neural_projection_time: Duration,
    pub sleep_consolidation_time: Duration,
    pub scenario_attempts: u32,
    pub scenario_successes: u32,
    pub sealed_patches: u32,
    pub memory_updates: u32,
    pub topology_updates: u32,
    pub active_synapses: u32,
    pub active_tiles: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkRun {
    pub tier: BenchmarkTier,
    pub brain_tier: BrainScaleTier,
    pub metrics: BenchmarkMetrics,
    pub manual_expected_slow: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkReport {
    pub runs: Vec<BenchmarkRun>,
    pub update_rates: UpdateRatePolicy,
    pub budget_policy: ComputeBudgetPolicy,
    pub requires_bevy: bool,
    pub requires_gpu: bool,
}

impl BenchmarkReport {
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("# A-Life benchmark tiers report\n\n");
        out.push_str("Mode: CPU reference smoke; Bevy/GPU required: no/no.\n\n");
        out.push_str("| Population | Brain tier | Manual expected-slow | Tick time ms | Patches/sec | Memory bytes | Success |\n");
        out.push_str("|---:|---|---|---:|---:|---:|---:|\n");
        for run in &self.runs {
            let success_rate = if run.metrics.scenario_attempts == 0 {
                0.0
            } else {
                run.metrics.scenario_successes as f64 / run.metrics.scenario_attempts as f64
            };
            out.push_str(&format!(
                "| {} | {:?} | {} | {:.3} | {:.3} | {} | {:.3} |\n",
                run.tier.population,
                run.brain_tier,
                run.manual_expected_slow,
                run.metrics.tick_time.as_secs_f64() * 1000.0,
                run.metrics.patch_throughput_per_second,
                run.metrics.memory_usage_estimate_bytes,
                success_rate,
            ));
        }
        out.push_str("\n## Metric fields\n\n");
        for metric in BenchmarkMetricKind::ALL {
            out.push_str(&format!("- {:?}\n", metric));
        }
        out.push_str("\n## Biological compute budget\n\n");
        out.push_str(&format!(
            "- Tier: {:?}\n- Active synapses: {}\n- Active tiles: {}\n- Essential lobes: {:?}\n- Non-essential lobes: {:?}\n- Fallback update Hz: {:.1}\n",
            self.budget_policy.tier,
            self.budget_policy.active_synapse_budget,
            self.budget_policy.active_tile_budget,
            self.budget_policy.essential_lobes,
            self.budget_policy.nonessential_lobes,
            self.budget_policy.fallback_update_frequency_hz,
        ));
        out.push_str("\n## Manual expected-slow tiers\n\n");
        out.push_str("Run `cargo test -p alife_tools --test benchmark_tiers -- --ignored --nocapture` for CPU-only tiers 50/100/250/500. These are measurement targets, not CI gates.\n");
        out
    }

    pub fn write_markdown(&self, output_dir: &Path) -> std::io::Result<PathBuf> {
        fs::create_dir_all(output_dir)?;
        let path = output_dir.join("benchmark_tiers.md");
        fs::write(&path, self.to_markdown())?;
        Ok(path)
    }
}

pub struct BenchmarkHarness;

impl BenchmarkHarness {
    pub fn run(
        config: BenchmarkHarnessConfig,
    ) -> Result<BenchmarkReport, alife_core::ScaffoldContractError> {
        let budget_policy = ComputeBudgetPolicy::for_tier(config.brain_tier)?;
        let runs = config
            .tiers
            .iter()
            .copied()
            .map(|tier| run_tier(tier, config.brain_tier, config.iteration_count))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(BenchmarkReport {
            runs,
            update_rates: config.update_rates,
            budget_policy,
            requires_bevy: false,
            requires_gpu: false,
        })
    }
}

pub struct GpuRuntimeBenchmarkBridge;

impl GpuRuntimeBenchmarkBridge {
    pub fn from_cpu_smoke(
        cpu_report: &BenchmarkReport,
        backend: GpuRuntimeBackendStatus,
        notes: impl Into<String>,
    ) -> Result<GpuTierPerformanceReport, alife_core::ScaffoldContractError> {
        let notes = notes.into();
        let mut report = GpuTierMeasurement::unavailable_report(backend, notes.clone());
        report.schema_version = P29_RUNTIME_SCHEMA_VERSION;
        report
            .feature_flags
            .retain(|flag| flag != "gpu-unavailable");
        report.feature_flags.push("cpu-smoke-metrics".to_string());
        report.feature_flags.push("p20-smoke".to_string());

        for run in &cpu_report.runs {
            let Some(population) = tier_population(run.tier.population) else {
                return Err(alife_core::ScaffoldContractError::ScalarOutOfRange);
            };
            if let Some(measurement) = report
                .measurements
                .iter_mut()
                .find(|measurement| measurement.population == population)
            {
                measurement.tick_time_ms = Some(run.metrics.tick_time.as_secs_f32() * 1000.0);
                measurement.patch_throughput_per_second =
                    Some(run.metrics.patch_throughput_per_second as f32);
                measurement.memory_topology_update_ms =
                    Some(run.metrics.memory_topology_update_time.as_secs_f32() * 1000.0);
                measurement.sleep_recompaction_ms =
                    Some(run.metrics.sleep_consolidation_time.as_secs_f32() * 1000.0);
                measurement.active_synapses = run.metrics.active_synapses;
                measurement.active_tiles = run.metrics.active_tiles;
                measurement.notes = format!("{notes}; P20 CPU smoke measurement copied");
            }
        }

        report.validate()?;
        Ok(report)
    }
}

fn run_tier(
    tier: BenchmarkTier,
    brain_tier: BrainScaleTier,
    iteration_count: u16,
) -> Result<BenchmarkRun, alife_core::ScaffoldContractError> {
    let mut metrics = BenchmarkMetrics {
        memory_usage_estimate_bytes: sparse_population_total_bytes(brain_tier, tier.population),
        ..BenchmarkMetrics::default()
    };
    let start = Instant::now();
    let iterations = iteration_count.max(1);
    for iteration in 0..iterations {
        for agent_index in 0..tier.population {
            let scenario = scenario_for_agent(agent_index);
            let seed = 20_000 + u64::from(iteration) * 1_000 + u64::from(agent_index);
            let scenario_start = Instant::now();
            let fixture = ScenarioFixture::with_seed(scenario, seed)?;
            let run = fixture.run()?;
            let elapsed = scenario_start.elapsed();

            metrics.scenario_attempts = metrics.scenario_attempts.saturating_add(1);
            metrics.sealed_patches = metrics
                .sealed_patches
                .saturating_add(run.patches.len() as u32);
            if run
                .patches
                .iter()
                .all(|patch| patch.validate_contract().is_ok())
                && run
                    .statuses
                    .iter()
                    .all(|status| *status != BrainTickStatus::TerminalInvalidState)
            {
                metrics.scenario_successes = metrics.scenario_successes.saturating_add(1);
            }
            metrics.memory_updates = metrics
                .memory_updates
                .saturating_add(run.memory_record_count as u32);
            metrics.topology_updates = metrics
                .topology_updates
                .saturating_add(run.topology_concept_count as u32);
            if run.memory_record_count > 0 || run.topology_concept_count > 0 {
                metrics.memory_topology_update_time += elapsed;
            }
            if run.sleep_report.is_some() {
                metrics.sleep_consolidation_time += elapsed;
            }
        }
    }
    metrics.tick_time = start.elapsed();
    let seconds = metrics.tick_time.as_secs_f64().max(f64::EPSILON);
    metrics.patch_throughput_per_second = f64::from(metrics.sealed_patches) / seconds;

    Ok(BenchmarkRun {
        tier,
        brain_tier,
        metrics,
        manual_expected_slow: tier.expected_slow_cpu_only,
    })
}

fn scenario_for_agent(agent_index: u16) -> ScenarioName {
    let scenarios = ScenarioName::ALL;
    scenarios[usize::from(agent_index) % scenarios.len()]
}

fn sparse_population_total_bytes(tier: BrainScaleTier, population: u16) -> u64 {
    let Ok(spec) = BrainClassSpec::try_for_tier(tier) else {
        return 0;
    };
    let active_synapses = u64::from(spec.compute_budget.max_active_synapses);
    let active_tiles = u64::from(spec.compute_budget.max_active_tiles);
    let neurons = u64::from(spec.neuron_count);
    let shared_species_template = active_synapses * 2;
    let sparse_per_creature_live = active_synapses * 2
        + active_tiles
        + active_synapses * 2
        + active_synapses
        + neurons * 4
        + neurons * 4
        + active_synapses
        + active_tiles * 16
        + 2 * 64
        + 2 * 256;
    shared_species_template + sparse_per_creature_live * u64::from(population)
}

fn tier_population(population: u16) -> Option<GpuTierPopulation> {
    match population {
        1 => Some(GpuTierPopulation::One),
        10 => Some(GpuTierPopulation::Ten),
        30 => Some(GpuTierPopulation::Thirty),
        50 => Some(GpuTierPopulation::Fifty),
        100 => Some(GpuTierPopulation::OneHundred),
        250 => Some(GpuTierPopulation::TwoHundredFifty),
        500 => Some(GpuTierPopulation::FiveHundred),
        _ => None,
    }
}
