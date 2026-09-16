use alife_gpu_backend::{
    GpuDiagnosticProductRuntimeClaim, GpuDiagnosticTimingKind, GpuDiagnosticTimingReport,
    GpuDiagnosticWorkloadTiming, GpuRuntimeBackendKind, GpuTimingTargetStatus,
    GPU_DIAGNOSTIC_TIMING_SCHEMA_VERSION,
};

#[test]
fn gpu_timing_report_marks_diagnostic_evidence_not_product_runtime() {
    let report = GpuDiagnosticTimingReport {
        schema_version: GPU_DIAGNOSTIC_TIMING_SCHEMA_VERSION,
        adapter_identifier: "test-adapter (Vulkan, DiscreteGpu, test-driver)".to_string(),
        adapter_name: "test-adapter".to_string(),
        backend_api: "Vulkan".to_string(),
        driver_info: "test-driver".to_string(),
        timestamp_query_supported: false,
        requested_backend: GpuRuntimeBackendKind::GpuAuthoritative,
        product_gameplay_timing_claim: GpuDiagnosticProductRuntimeClaim::None,
        workloads: vec![GpuDiagnosticWorkloadTiming {
            schema_version: GPU_DIAGNOSTIC_TIMING_SCHEMA_VERSION,
            workload_name: "P25 static forward diagnostic fixture".to_string(),
            fixture_dimensions: "neurons=512, tiles=2, synapses=258".to_string(),
            warmup_iterations: 1,
            measured_iterations: 2,
            host_fixture_mean_ms: Some(0.01),
            gpu_submit_poll_mean_ms: Some(0.25),
            readback_mean_ms: Some(0.4),
            gpu_total_mean_ms: Some(0.65),
            parity_passed: true,
            no_active_gameplay_readback: true,
            timing_kind: GpuDiagnosticTimingKind::HostObservedDiagnostic,
            product_runtime_claim: GpuDiagnosticProductRuntimeClaim::DiagnosticOnly,
            target_60_fps: GpuTimingTargetStatus::NotApplicable,
            notes: "host-observed diagnostic timing".to_string(),
        }],
    };

    report.validate().unwrap();
    let markdown = report.to_markdown();
    assert!(markdown.contains("Host fixture mean ms"));
    assert!(!markdown.contains("CPU mean ms"));
    assert!(markdown.contains("Product gameplay timing claim: None"));
    assert!(markdown.contains("HostObservedDiagnostic"));
    assert!(markdown.contains("DiagnosticOnly"));
    assert!(markdown.contains("not active gameplay runtime timing"));
    assert!(markdown.contains("Required GPU unavailability is typed"));
}

#[test]
fn gpu_timing_report_rejects_fake_timing_claims_without_gpu_measurements() {
    let workload = GpuDiagnosticWorkloadTiming {
        schema_version: GPU_DIAGNOSTIC_TIMING_SCHEMA_VERSION,
        workload_name: "P25 static forward diagnostic fixture".to_string(),
        fixture_dimensions: "neurons=512, tiles=2, synapses=258".to_string(),
        warmup_iterations: 1,
        measured_iterations: 2,
        host_fixture_mean_ms: Some(0.01),
        gpu_submit_poll_mean_ms: None,
        readback_mean_ms: None,
        gpu_total_mean_ms: None,
        parity_passed: true,
        no_active_gameplay_readback: true,
        timing_kind: GpuDiagnosticTimingKind::HostObservedDiagnostic,
        product_runtime_claim: GpuDiagnosticProductRuntimeClaim::DiagnosticOnly,
        target_60_fps: GpuTimingTargetStatus::NotApplicable,
        notes: "missing GPU timing should reject".to_string(),
    };

    assert!(workload.validate().is_err());
}

#[cfg(feature = "gpu-tests")]
#[test]
#[ignore = "requires a local wgpu adapter; run with `cargo test -p alife_gpu_backend --features gpu-tests --test gpu_timing_report -- --ignored --nocapture`"]
fn local_gpu_diagnostic_timing_report_measures_real_adapter() {
    let report = alife_gpu_backend::run_local_gpu_diagnostic_timing(1, 2).unwrap();
    println!("{}", report.to_markdown());
    report.validate().unwrap();
    assert_eq!(report.workloads.len(), 1);
    assert!(report
        .workloads
        .iter()
        .all(|workload| workload.parity_passed));
}
