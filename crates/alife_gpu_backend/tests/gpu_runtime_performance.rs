use alife_core::LobeKind;
use alife_gpu_backend::{
    required_storage_buffers, GpuPerformanceTargetStatus, GpuRuntimeBackendConfig,
    GpuRuntimeBackendKind, GpuRuntimeBoundary, GpuRuntimeCapabilityManifest,
    GpuRuntimeDiagnosticExport, GpuRuntimeFallbackReason, GpuRuntimeHardwareProbe,
    GpuRuntimeReadbackGuard, GpuRuntimeThrottlingPolicy, GpuRuntimeTimingBudget,
    GpuRuntimeTimingSample, GpuThrottleLevel, GpuThrottleReason, GpuTierMeasurement,
    GpuTierPopulation, P27_PLASTICITY_STORAGE_BINDINGS, P27_STATIC_FORWARD_STORAGE_BINDINGS,
    P29_RUNTIME_SCHEMA_VERSION,
};

#[test]
fn backend_selection_defaults_to_cpu_and_falls_back_cleanly() {
    let cpu = GpuRuntimeBackendConfig::default().select_backend().unwrap();
    assert_eq!(cpu.selected, GpuRuntimeBackendKind::CpuReference);
    assert_eq!(cpu.requested, GpuRuntimeBackendKind::CpuReference);
    assert_eq!(cpu.fallback_reason, None);
    assert!(cpu.cpu_oracle_authoritative);

    let unavailable = GpuRuntimeBackendConfig::request(GpuRuntimeBackendKind::GpuStatic)
        .with_hardware_available(false)
        .select_backend()
        .unwrap();
    assert_eq!(unavailable.selected, GpuRuntimeBackendKind::CpuReference);
    assert_eq!(
        unavailable.fallback_reason,
        Some(GpuRuntimeFallbackReason::HardwareUnavailable)
    );

    let failed_validation = GpuRuntimeBackendConfig::request(GpuRuntimeBackendKind::GpuPlastic)
        .with_hardware_available(true)
        .with_gpu_feature_enabled(true)
        .with_validation_passed(false)
        .select_backend()
        .unwrap();
    assert_eq!(
        failed_validation.selected,
        GpuRuntimeBackendKind::CpuReference
    );
    assert_eq!(
        failed_validation.fallback_reason,
        Some(GpuRuntimeFallbackReason::ValidationFailed)
    );

    let plastic = GpuRuntimeBackendConfig::request(GpuRuntimeBackendKind::GpuPlastic)
        .with_hardware_available(true)
        .with_gpu_feature_enabled(true)
        .with_validation_passed(true)
        .select_backend()
        .unwrap();
    assert_eq!(plastic.selected, GpuRuntimeBackendKind::GpuPlastic);
    assert_eq!(plastic.fallback_reason, None);
}

#[test]
fn local_gpu_probe_contract_records_requirements_without_claiming_hardware_in_ci() {
    assert_eq!(
        required_storage_buffers(GpuRuntimeBackendKind::GpuStatic),
        P27_STATIC_FORWARD_STORAGE_BINDINGS
    );
    assert_eq!(
        required_storage_buffers(GpuRuntimeBackendKind::GpuPlastic),
        P27_PLASTICITY_STORAGE_BINDINGS
    );
    assert_eq!(
        required_storage_buffers(GpuRuntimeBackendKind::GpuFull),
        P27_PLASTICITY_STORAGE_BINDINGS
    );

    let unavailable =
        GpuRuntimeHardwareProbe::unavailable(GpuRuntimeBackendKind::GpuStatic, "adapter missing");
    assert!(!unavailable.hardware_available());
    assert_eq!(
        unavailable.required_storage_buffers_per_shader_stage,
        P27_STATIC_FORWARD_STORAGE_BINDINGS
    );
    assert_eq!(unavailable.hardware_identifier(), None);
    assert!(unavailable
        .error
        .as_deref()
        .unwrap()
        .contains("adapter missing"));
}

#[test]
fn no_readback_guard_blocks_active_bulk_and_diagnostic_readbacks() {
    let active = GpuRuntimeReadbackGuard::active_tick();
    assert!(active.permits_boundary(GpuRuntimeBoundary::TickActionSummary));
    assert!(!active.permits_boundary(GpuRuntimeBoundary::DiagnosticExport));
    assert!(!active.permits_bulk_neural_readback());
    assert!(!active.permits_per_synapse_readback());
    assert!(!active.permits_per_lobe_readback());
    assert!(!active.permits_weight_readback());
    assert!(active
        .validate_export_request(GpuRuntimeBoundary::DiagnosticExport)
        .is_err());

    let frame = GpuRuntimeReadbackGuard::after_frame_boundary();
    assert!(frame
        .validate_export_request(GpuRuntimeBoundary::DiagnosticExport)
        .is_ok());
    assert!(!frame.permits_bulk_neural_readback());
}

#[test]
fn throttling_decimates_nonessential_lobes_before_sensory_motor() {
    let policy = GpuRuntimeThrottlingPolicy::reference();
    let budget = GpuRuntimeTimingBudget {
        target_frame_budget_ms: 16.667,
        gpu_neural_budget_ms: 4.0,
        fallback_update_frequency_hz: 5.0,
    };

    let under_budget = policy
        .decide(
            budget,
            GpuRuntimeTimingSample {
                measured_gpu_neural_ms: 2.0,
                measured_frame_ms: 10.0,
            },
        )
        .unwrap();
    assert_eq!(under_budget.level, GpuThrottleLevel::None);
    assert_eq!(under_budget.reason, GpuThrottleReason::WithinBudget);
    assert_eq!(under_budget.nonessential_decimation_factor, 1);
    assert!(under_budget.sensory_motor_protected);

    let over_budget = policy
        .decide(
            budget,
            GpuRuntimeTimingSample {
                measured_gpu_neural_ms: 7.0,
                measured_frame_ms: 17.0,
            },
        )
        .unwrap();
    assert_eq!(over_budget.level, GpuThrottleLevel::DecimateNonEssential);
    assert_eq!(over_budget.reason, GpuThrottleReason::GpuNeuralOverBudget);
    assert!(over_budget.nonessential_decimation_factor > 1);
    assert!(over_budget
        .protected_lobes
        .contains(&LobeKind::SensoryGrounding));
    assert!(over_budget
        .protected_lobes
        .contains(&LobeKind::MotorArbitration));
    assert!(!over_budget
        .decimated_lobes
        .contains(&LobeKind::MotorArbitration));
}

#[test]
fn diagnostics_export_is_boundary_scoped_and_lossless_about_fallback_status() {
    let status = GpuRuntimeBackendConfig::request(GpuRuntimeBackendKind::GpuFull)
        .with_hardware_available(false)
        .select_backend()
        .unwrap();
    let snapshot = GpuRuntimeDiagnosticExport::new(
        P29_RUNTIME_SCHEMA_VERSION,
        GpuRuntimeBoundary::FrameBoundary,
        status,
        GpuRuntimeTimingSample {
            measured_gpu_neural_ms: 0.0,
            measured_frame_ms: 0.0,
        },
        "wgpu adapter unavailable in CI",
    )
    .unwrap();

    assert_eq!(snapshot.boundary, GpuRuntimeBoundary::FrameBoundary);
    assert_eq!(
        snapshot.backend.selected,
        GpuRuntimeBackendKind::CpuReference
    );
    assert_eq!(
        snapshot.backend.fallback_reason,
        Some(GpuRuntimeFallbackReason::HardwareUnavailable)
    );

    let active_attempt = GpuRuntimeDiagnosticExport::new(
        P29_RUNTIME_SCHEMA_VERSION,
        GpuRuntimeBoundary::ActiveTick,
        status,
        GpuRuntimeTimingSample {
            measured_gpu_neural_ms: 0.0,
            measured_frame_ms: 0.0,
        },
        "should reject",
    );
    assert!(active_attempt.is_err());
}

#[test]
fn performance_tier_report_covers_all_required_populations_without_fabricating_gpu_results() {
    let status = GpuRuntimeBackendConfig::request(GpuRuntimeBackendKind::GpuStatic)
        .with_hardware_available(false)
        .select_backend()
        .unwrap();
    let report = GpuTierMeasurement::cpu_fallback_report(
        status,
        "no GPU performance run in CI; smoke CPU data only",
    );

    assert_eq!(
        report
            .measurements
            .iter()
            .map(|tier| tier.population)
            .collect::<Vec<_>>(),
        [
            GpuTierPopulation::One,
            GpuTierPopulation::Ten,
            GpuTierPopulation::Fifty,
            GpuTierPopulation::OneHundred,
            GpuTierPopulation::TwoHundredFifty,
            GpuTierPopulation::FiveHundred,
        ]
    );
    assert!(report
        .measurements
        .iter()
        .all(|tier| tier.target_60_fps == GpuPerformanceTargetStatus::Unknown));
    assert!(report.hardware_identifier.is_none());
    assert!(report
        .measurements
        .iter()
        .all(|tier| tier.notes.contains("no GPU performance run")));
}

#[test]
fn performance_tier_report_markdown_records_unknown_targets_and_manual_notes() {
    let status = GpuRuntimeBackendConfig::request(GpuRuntimeBackendKind::GpuStatic)
        .with_hardware_available(false)
        .select_backend()
        .unwrap();
    let report = GpuTierMeasurement::cpu_fallback_report(
        status,
        "manual GPU performance unavailable on this runner",
    );
    let markdown = report.to_markdown();

    assert!(markdown.contains("# P29 GPU runtime performance report"));
    assert!(markdown.contains("| 500 | CpuReference | unknown | unknown | Unknown |"));
    assert!(markdown.contains("manual GPU performance unavailable"));
    assert!(markdown.contains("No active gameplay neural readback"));
}

#[test]
fn runtime_capability_manifest_preserves_p25_p26_p27_p28_boundaries() {
    let manifest = GpuRuntimeCapabilityManifest::current_contract();
    assert!(manifest.static_forward_parity_available);
    assert!(manifest.plasticity_parity_available);
    assert!(manifest.routing_masks_available);
    assert!(manifest.sleep_recompaction_available);
    assert!(!manifest.product_gpu_full_runtime_default);
    assert_eq!(manifest.static_forward_storage_bindings, 9);
    assert_eq!(manifest.plasticity_storage_bindings, 10);
    assert!(manifest.no_active_gameplay_neural_readback);
}
