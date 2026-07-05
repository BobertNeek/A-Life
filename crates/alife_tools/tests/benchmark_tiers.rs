use std::time::Duration;

use alife_core::{BrainScaleTier, LobeKind};
use alife_gpu_backend::{
    GpuPerformanceTargetStatus, GpuRuntimeBackendConfig, GpuRuntimeBackendKind,
    GpuRuntimeFallbackReason, GpuTierPopulation,
};
use alife_tools::benchmark::{
    BenchmarkHarness, BenchmarkHarnessConfig, BenchmarkMetricKind, BenchmarkTier,
    ComputeBudgetPolicy, GpuRuntimeBenchmarkBridge, ResidencyClass, TargetProfile,
    UpdateRatePolicy,
};

#[test]
fn benchmark_tiers_cover_required_population_counts_and_manual_upper_tiers() {
    let tiers = BenchmarkTier::required_tiers();
    assert_eq!(
        tiers.iter().map(|tier| tier.population).collect::<Vec<_>>(),
        [1, 10, 30, 50, 100, 250, 500]
    );
    assert!(tiers
        .iter()
        .filter(|tier| tier.expected_slow_cpu_only)
        .all(|tier| tier.population >= 50));
    assert!(tiers
        .iter()
        .filter(|tier| !tier.expected_slow_cpu_only)
        .all(|tier| tier.population <= 30));
}

#[test]
fn compute_budget_policy_keeps_essential_lobes_and_decimates_nonessential_first() {
    let policy = ComputeBudgetPolicy::for_tier(BrainScaleTier::Standard2048).unwrap();
    assert!(policy.essential_lobes.contains(&LobeKind::SensoryGrounding));
    assert!(policy.essential_lobes.contains(&LobeKind::MotorArbitration));
    assert!(policy
        .nonessential_lobes
        .contains(&LobeKind::LexiconConcept));
    assert!(policy.throttling.nonessential_decimation_threshold > 0.0);
    assert!(
        policy.throttling.nonessential_decimation_threshold
            < policy.throttling.warm_cadence_threshold
    );
    assert!(policy.fallback_update_frequency_hz > 0.0);
    assert!(policy.active_synapse_budget > 0);
    assert!(policy.active_tile_budget > 0);
}

#[test]
fn update_rates_are_configurable_by_residency_and_keep_hot_motor_at_60_hz() {
    let rates = UpdateRatePolicy::v1_defaults();
    assert_eq!(
        rates.rate_hz(ResidencyClass::Hot, TargetProfile::SensoryMotor),
        60.0
    );
    assert_eq!(
        rates.rate_hz(ResidencyClass::Hot, TargetProfile::ActionArbitration),
        60.0
    );
    assert!(
        rates.rate_hz(ResidencyClass::Warm, TargetProfile::MemoryExpectancy)
            < rates.rate_hz(ResidencyClass::Hot, TargetProfile::MemoryExpectancy)
    );

    let adjusted = rates.with_rate_hz(ResidencyClass::Warm, TargetProfile::OnlinePlasticity, 3.0);
    assert_eq!(
        adjusted.rate_hz(ResidencyClass::Warm, TargetProfile::OnlinePlasticity),
        3.0
    );
}

#[test]
fn benchmark_metric_manifest_names_all_p20_metrics() {
    assert_eq!(
        BenchmarkMetricKind::ALL,
        [
            BenchmarkMetricKind::TickTime,
            BenchmarkMetricKind::MemoryUsageEstimate,
            BenchmarkMetricKind::PatchThroughput,
            BenchmarkMetricKind::MemoryTopologyUpdateTime,
            BenchmarkMetricKind::NeuralProjectionTime,
            BenchmarkMetricKind::SleepConsolidationTime,
            BenchmarkMetricKind::ScenarioSuccessRate,
        ]
    );
}

#[test]
fn benchmark_tiers_smoke_runs_tier_1_and_10_without_bevy_or_gpu() {
    let report = BenchmarkHarness::run(
        BenchmarkHarnessConfig::smoke()
            .with_tiers([BenchmarkTier::new(1), BenchmarkTier::new(10)])
            .with_iteration_count(1),
    )
    .unwrap();

    assert_eq!(report.runs.len(), 2);
    assert!(report
        .runs
        .iter()
        .all(|run| run.metrics.scenario_attempts == run.metrics.scenario_successes));
    assert!(report
        .runs
        .iter()
        .all(|run| run.metrics.patch_throughput_per_second > 0.0));
    assert!(report
        .runs
        .iter()
        .all(|run| run.metrics.tick_time > Duration::ZERO));
    assert!(report
        .runs
        .iter()
        .all(|run| run.metrics.memory_usage_estimate_bytes > 0));
    assert!(!report.requires_bevy);
    assert!(!report.requires_gpu);
}

#[test]
fn benchmark_report_generator_writes_markdown_under_target_artifacts() {
    let report = BenchmarkHarness::run(
        BenchmarkHarnessConfig::smoke()
            .with_tiers([BenchmarkTier::new(1)])
            .with_iteration_count(1),
    )
    .unwrap();
    let output_dir = std::env::temp_dir().join("alife_p20_benchmark_report_test");
    let path = report.write_markdown(&output_dir).unwrap();
    let markdown = std::fs::read_to_string(&path).unwrap();

    assert!(path.ends_with("benchmark_tiers.md"));
    assert!(markdown.contains("# A-Life benchmark tiers report"));
    assert!(markdown.contains("| 1 |"));
    assert!(markdown.contains("CPU reference smoke"));
    assert!(markdown.contains("Manual expected-slow tiers"));
}

#[test]
fn gpu_runtime_bridge_reuses_p20_smoke_without_fabricating_gpu_results() {
    let cpu_report = BenchmarkHarness::run(
        BenchmarkHarnessConfig::smoke()
            .with_tiers([BenchmarkTier::new(1), BenchmarkTier::new(10)])
            .with_iteration_count(1),
    )
    .unwrap();
    let backend = GpuRuntimeBackendConfig::request(GpuRuntimeBackendKind::GpuStatic)
        .with_hardware_available(false)
        .select_backend()
        .unwrap();
    let gpu_report = GpuRuntimeBenchmarkBridge::from_cpu_smoke(
        &cpu_report,
        backend,
        "P29 CI smoke: no GPU hardware performance run",
    )
    .unwrap();

    assert_eq!(
        gpu_report.backend.selected,
        GpuRuntimeBackendKind::CpuReference
    );
    assert_eq!(
        gpu_report.backend.fallback_reason,
        Some(GpuRuntimeFallbackReason::HardwareUnavailable)
    );
    assert!(gpu_report.feature_flags.contains(&"p20-smoke".to_string()));
    assert_eq!(
        gpu_report
            .measurements
            .iter()
            .map(|measurement| measurement.population)
            .collect::<Vec<_>>(),
        [
            GpuTierPopulation::One,
            GpuTierPopulation::Ten,
            GpuTierPopulation::Thirty,
            GpuTierPopulation::Fifty,
            GpuTierPopulation::OneHundred,
            GpuTierPopulation::TwoHundredFifty,
            GpuTierPopulation::FiveHundred,
        ]
    );
    assert!(gpu_report.measurements[0].tick_time_ms.is_some());
    assert!(gpu_report.measurements[1].tick_time_ms.is_some());
    assert!(gpu_report.measurements[2].tick_time_ms.is_none());
    assert!(gpu_report
        .measurements
        .iter()
        .all(|measurement| measurement.gpu_neural_time_ms.is_none()));
    assert!(gpu_report
        .measurements
        .iter()
        .all(|measurement| measurement.target_60_fps == GpuPerformanceTargetStatus::Unknown));
}

#[test]
#[ignore = "manual CPU-only expected-slow benchmark tiers; use --ignored --nocapture"]
fn manual_expected_slow_cpu_tiers_run_without_bevy_or_gpu() {
    let report = BenchmarkHarness::run(
        BenchmarkHarnessConfig::smoke()
            .with_tiers([
                BenchmarkTier::new(50),
                BenchmarkTier::new(100),
                BenchmarkTier::new(250),
                BenchmarkTier::new(500),
            ])
            .with_iteration_count(1),
    )
    .unwrap();

    assert_eq!(report.runs.len(), 4);
    assert!(report.runs.iter().all(|run| run.manual_expected_slow));
    assert!(report
        .runs
        .iter()
        .all(|run| run.metrics.scenario_attempts == u32::from(run.tier.population)));
    assert!(!report.requires_bevy);
    assert!(!report.requires_gpu);
}
