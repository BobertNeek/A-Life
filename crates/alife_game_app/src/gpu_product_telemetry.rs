//! GPU-authority product telemetry and bounded readback policy.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub struct GpuProductTelemetryOverlay {
    pub schema: &'static str,
    pub schema_version: u16,
    pub requested_backend: String,
    pub selected_backend: String,
    pub unavailable_reason: Option<String>,
    pub gpu_runtime_feature_compiled: bool,
    pub authoritative: bool,
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
            || !self.no_active_gameplay_readback
            || self.telemetry_boundary.is_empty()
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{:?}:{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.requested_backend,
            self.selected_backend,
            self.unavailable_reason,
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
    pub gpu_required_default: bool,
    pub invalid_gpu_config_stops_actions: bool,
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
            || !self.gpu_required_default
            || !self.invalid_gpu_config_stops_actions
            || !self.active_readback_blocked
            || !self.diagnostic_export_boundary_allowed
            || self.manual_hardware_command.is_empty()
            || self.performance_claim_status != "unknown-unless-measured"
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        self.telemetry_overlay.validate()
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.gpu_required_default,
            self.invalid_gpu_config_stops_actions,
            self.active_readback_blocked,
            self.diagnostic_export_boundary_allowed,
            self.telemetry_overlay.signature_line()
        )
    }
}

pub fn run_gpu_product_hardening_smoke() -> Result<GpuProductHardeningSummary, GameAppShellError> {
    #[cfg(feature = "gpu-runtime")]
    let compiled = true;
    #[cfg(not(feature = "gpu-runtime"))]
    let compiled = false;
    let overlay = GpuProductTelemetryOverlay {
        schema: G12_GPU_PRODUCT_TELEMETRY_SCHEMA,
        schema_version: G12_GPU_PRODUCT_TELEMETRY_SCHEMA_VERSION,
        requested_backend: "GpuAuthoritative".to_string(),
        selected_backend: if compiled {
            "GpuAuthoritative"
        } else {
            "Unavailable"
        }
        .to_string(),
        unavailable_reason: (!compiled).then(|| "gpu-runtime feature disabled".to_string()),
        gpu_runtime_feature_compiled: compiled,
        authoritative: compiled,
        no_active_gameplay_readback: true,
        telemetry_boundary: "frame-boundary-diagnostic-export".to_string(),
        tick_time_ms: None,
        gpu_neural_time_ms: None,
        skipped_supertiles: 0,
        skipped_tiles: 0,
        measured_gpu_performance: false,
        report_notes: "GPU measurements remain unknown until measured on hardware".to_string(),
    };
    let summary = GpuProductHardeningSummary {
        schema: G12_GPU_PRODUCT_TELEMETRY_SCHEMA,
        schema_version: G12_GPU_PRODUCT_TELEMETRY_SCHEMA_VERSION,
        gpu_required_default: true,
        invalid_gpu_config_stops_actions: true,
        active_readback_blocked: true,
        diagnostic_export_boundary_allowed: true,
        telemetry_overlay: overlay,
        report_markdown_preview: "# GPU authority telemetry\n\nFailure stops learned actions.\n"
            .to_string(),
        manual_hardware_command: g12_manual_gpu_hardware_command().to_string(),
        performance_claim_status: "unknown-unless-measured".to_string(),
    };
    summary.validate()?;
    Ok(summary)
}

pub const fn g12_manual_gpu_hardware_command() -> &'static str {
    "cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime"
}
