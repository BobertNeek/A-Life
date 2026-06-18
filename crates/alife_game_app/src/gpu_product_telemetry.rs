//! Split from the original playable-sim app shell during R13 remediation.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub struct GpuProductTelemetryOverlay {
    pub schema: &'static str,
    pub schema_version: u16,
    pub requested_backend: String,
    pub selected_backend: String,
    pub fallback_reason: Option<String>,
    pub gpu_runtime_feature_compiled: bool,
    pub cpu_oracle_authoritative: bool,
    pub no_active_gameplay_readback: bool,
    pub telemetry_boundary: String,
    pub tick_time_ms: Option<f32>,
    pub gpu_neural_time_ms: Option<f32>,
    pub skipped_supertiles: u32,
    pub skipped_tiles: u32,
    pub measured_gpu_performance: bool,
    pub report_notes: String,
}

impl GpuProductTelemetryOverlay {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G12_GPU_PRODUCT_TELEMETRY_SCHEMA
            || self.schema_version != G12_GPU_PRODUCT_TELEMETRY_SCHEMA_VERSION
            || self.requested_backend.is_empty()
            || self.selected_backend.is_empty()
            || !self.cpu_oracle_authoritative
            || !self.no_active_gameplay_readback
            || self.telemetry_boundary.is_empty()
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        for value in [self.tick_time_ms, self.gpu_neural_time_ms]
            .into_iter()
            .flatten()
        {
            if !value.is_finite() || value < 0.0 {
                return Err(ScaffoldContractError::ScalarOutOfRange);
            }
        }
        if self.measured_gpu_performance && self.gpu_neural_time_ms.is_none() {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{:?}:{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.requested_backend,
            self.selected_backend,
            self.fallback_reason,
            self.gpu_runtime_feature_compiled,
            self.no_active_gameplay_readback,
            self.skipped_supertiles,
            self.skipped_tiles,
            self.measured_gpu_performance,
            self.report_notes
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuProductHardeningSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub cpu_fallback_default: bool,
    pub invalid_gpu_config_falls_back: bool,
    pub active_readback_blocked: bool,
    pub diagnostic_export_boundary_allowed: bool,
    pub telemetry_overlay: GpuProductTelemetryOverlay,
    pub report_markdown_preview: String,
    pub manual_hardware_command: String,
    pub performance_claim_status: String,
}

impl GpuProductHardeningSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G12_GPU_PRODUCT_TELEMETRY_SCHEMA
            || self.schema_version != G12_GPU_PRODUCT_TELEMETRY_SCHEMA_VERSION
            || !self.cpu_fallback_default
            || !self.invalid_gpu_config_falls_back
            || !self.active_readback_blocked
            || !self.diagnostic_export_boundary_allowed
            || self.manual_hardware_command.is_empty()
            || self.performance_claim_status != "unknown-unless-measured"
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        self.telemetry_overlay.validate()?;
        if !self
            .report_markdown_preview
            .contains("CPU fallback is not GPU performance")
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.cpu_fallback_default,
            self.invalid_gpu_config_falls_back,
            self.active_readback_blocked,
            self.diagnostic_export_boundary_allowed,
            self.telemetry_overlay.signature_line()
        )
    }
}

pub fn run_gpu_product_hardening_smoke() -> Result<GpuProductHardeningSummary, GameAppShellError> {
    run_gpu_product_hardening_smoke_impl()
}

#[cfg(feature = "gpu-runtime")]
fn run_gpu_product_hardening_smoke_impl() -> Result<GpuProductHardeningSummary, GameAppShellError> {
    use alife_gpu_backend::{
        GpuRuntimeBackendConfig, GpuRuntimeBackendKind, GpuRuntimeBoundary,
        GpuRuntimeReadbackGuard, GpuRuntimeTimingSample, GpuTierMeasurement,
    };

    let cpu_status = GpuRuntimeBackendConfig::default().select_backend()?;
    let invalid_gpu_status = GpuRuntimeBackendConfig::request(GpuRuntimeBackendKind::GpuStatic)
        .with_gpu_feature_enabled(true)
        .with_hardware_available(true)
        .with_validation_passed(false)
        .select_backend()?;
    let active_guard = GpuRuntimeReadbackGuard::active_tick();
    let active_readback_blocked = !active_guard.permits_bulk_neural_readback()
        && !active_guard.permits_per_synapse_readback()
        && !active_guard.permits_per_lobe_readback()
        && !active_guard.permits_weight_readback()
        && active_guard
            .validate_export_request(GpuRuntimeBoundary::DiagnosticExport)
            .is_err();
    let diagnostic_export_boundary_allowed = GpuRuntimeReadbackGuard::after_frame_boundary()
        .validate_export_request(GpuRuntimeBoundary::DiagnosticExport)
        .is_ok();
    let report = GpuTierMeasurement::cpu_fallback_report(
        invalid_gpu_status,
        "G12 product smoke: CPU fallback is not GPU performance; run manual hardware command for measured GPU data",
    );
    report.validate()?;
    let report_markdown_preview = report.to_markdown();
    let telemetry_overlay = GpuProductTelemetryOverlay {
        schema: G12_GPU_PRODUCT_TELEMETRY_SCHEMA,
        schema_version: G12_GPU_PRODUCT_TELEMETRY_SCHEMA_VERSION,
        requested_backend: format!("{:?}", invalid_gpu_status.requested),
        selected_backend: format!("{:?}", invalid_gpu_status.selected),
        fallback_reason: invalid_gpu_status
            .fallback_reason
            .map(|reason| format!("{reason:?}")),
        gpu_runtime_feature_compiled: true,
        cpu_oracle_authoritative: invalid_gpu_status.cpu_oracle_authoritative,
        no_active_gameplay_readback: invalid_gpu_status.no_active_gameplay_readback,
        telemetry_boundary: "frame-boundary-diagnostic-export".to_string(),
        tick_time_ms: Some(
            GpuRuntimeTimingSample {
                measured_gpu_neural_ms: 0.0,
                measured_frame_ms: 0.0,
            }
            .measured_frame_ms,
        ),
        gpu_neural_time_ms: None,
        skipped_supertiles: 0,
        skipped_tiles: 0,
        measured_gpu_performance: false,
        report_notes: "CPU fallback; no GPU hardware timing claimed".to_string(),
    };
    let summary = GpuProductHardeningSummary {
        schema: G12_GPU_PRODUCT_TELEMETRY_SCHEMA,
        schema_version: G12_GPU_PRODUCT_TELEMETRY_SCHEMA_VERSION,
        cpu_fallback_default: cpu_status.selected == GpuRuntimeBackendKind::CpuReference
            && cpu_status.fallback_reason.is_none(),
        invalid_gpu_config_falls_back: invalid_gpu_status.selected
            == GpuRuntimeBackendKind::CpuReference
            && invalid_gpu_status.fallback_reason.is_some(),
        active_readback_blocked,
        diagnostic_export_boundary_allowed,
        telemetry_overlay,
        report_markdown_preview,
        manual_hardware_command: g12_manual_gpu_hardware_command().to_string(),
        performance_claim_status: "unknown-unless-measured".to_string(),
    };
    summary.validate()?;
    Ok(summary)
}

#[cfg(not(feature = "gpu-runtime"))]
fn run_gpu_product_hardening_smoke_impl() -> Result<GpuProductHardeningSummary, GameAppShellError> {
    let telemetry_overlay = GpuProductTelemetryOverlay {
        schema: G12_GPU_PRODUCT_TELEMETRY_SCHEMA,
        schema_version: G12_GPU_PRODUCT_TELEMETRY_SCHEMA_VERSION,
        requested_backend: "GpuStatic".to_string(),
        selected_backend: "CpuReference".to_string(),
        fallback_reason: Some("FeatureDisabled".to_string()),
        gpu_runtime_feature_compiled: false,
        cpu_oracle_authoritative: true,
        no_active_gameplay_readback: true,
        telemetry_boundary: "frame-boundary-diagnostic-export".to_string(),
        tick_time_ms: None,
        gpu_neural_time_ms: None,
        skipped_supertiles: 0,
        skipped_tiles: 0,
        measured_gpu_performance: false,
        report_notes: "GPU runtime feature disabled; CPU fallback is not GPU performance"
            .to_string(),
    };
    let summary = GpuProductHardeningSummary {
        schema: G12_GPU_PRODUCT_TELEMETRY_SCHEMA,
        schema_version: G12_GPU_PRODUCT_TELEMETRY_SCHEMA_VERSION,
        cpu_fallback_default: true,
        invalid_gpu_config_falls_back: true,
        active_readback_blocked: true,
        diagnostic_export_boundary_allowed: true,
        telemetry_overlay,
        report_markdown_preview:
            "# G12 GPU product telemetry\n\nCPU fallback is not GPU performance.\n".to_string(),
        manual_hardware_command: g12_manual_gpu_hardware_command().to_string(),
        performance_claim_status: "unknown-unless-measured".to_string(),
    };
    summary.validate()?;
    Ok(summary)
}

pub const fn g12_manual_gpu_hardware_command() -> &'static str {
    "ALIFE_GPU_RUNTIME_BACKEND=static ALIFE_GPU_RUNTIME_FEATURE=1 ALIFE_GPU_RUNTIME_AVAILABLE=1 ALIFE_GPU_RUNTIME_VALIDATED=1 cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime"
}
