//! S08 GPU, graphics, and performance evidence aggregation.
//!
//! This module does not execute GPU work. It collects existing CPU benchmark,
//! graphical launcher, GPU fallback, no-readback, and population-performance
//! policy evidence into a player/tester-facing status surface.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum S08EvidenceStatus {
    Measured,
    ManualUnknown,
    FallbackOnly,
}

impl S08EvidenceStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Measured => "measured",
            Self::ManualUnknown => "manual-unknown",
            Self::FallbackOnly => "fallback-only",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuGraphicsPerformanceSettingsPanel {
    pub schema: &'static str,
    pub schema_version: u16,
    pub target_fps: u16,
    pub target_frame_ms: f32,
    pub requested_backend: String,
    pub selected_backend: String,
    pub fallback_reason: Option<String>,
    pub gpu_runtime_feature_compiled: bool,
    pub cpu_oracle_authoritative: bool,
    pub no_active_gameplay_readback: bool,
    pub measured_gpu_performance: bool,
    pub gpu_neural_time_ms: Option<f32>,
    pub fps_target_status: S08EvidenceStatus,
    pub gpu_evidence_status: S08EvidenceStatus,
    pub graphics_evidence_status: S08EvidenceStatus,
    pub status_line: String,
}

impl GpuGraphicsPerformanceSettingsPanel {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != S08_GPU_GRAPHICS_PERFORMANCE_SCHEMA
            || self.schema_version != S08_GPU_GRAPHICS_PERFORMANCE_SCHEMA_VERSION
            || self.target_fps != S08_TARGET_FPS
            || !self.target_frame_ms.is_finite()
            || self.target_frame_ms <= 0.0
            || self.requested_backend.is_empty()
            || self.selected_backend.is_empty()
            || !self.cpu_oracle_authoritative
            || !self.no_active_gameplay_readback
            || self.status_line.is_empty()
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        if let Some(value) = self.gpu_neural_time_ms {
            if !value.is_finite() || value < 0.0 {
                return Err(ScaffoldContractError::ScalarOutOfRange);
            }
        }
        if self.measured_gpu_performance {
            if self.gpu_neural_time_ms.is_none()
                || self.gpu_evidence_status != S08EvidenceStatus::Measured
            {
                return Err(ScaffoldContractError::MissingPhaseData);
            }
        } else if self.gpu_evidence_status == S08EvidenceStatus::Measured
            || self.fps_target_status == S08EvidenceStatus::Measured
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        if !self
            .status_line
            .contains("CPU fallback is not GPU performance")
            || !self.status_line.contains("60 FPS target")
            || !self.status_line.contains("no active neural readback")
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{:?}:{}:{}:{}:{}",
            self.schema_version,
            self.requested_backend,
            self.selected_backend,
            self.fallback_reason,
            self.gpu_evidence_status.label(),
            self.graphics_evidence_status.label(),
            self.fps_target_status.label(),
            self.measured_gpu_performance
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuGraphicsPerformanceEvidenceSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub settings_panel: GpuGraphicsPerformanceSettingsPanel,
    pub benchmark_smoke_command: String,
    pub benchmark_gpu_runtime_command: String,
    pub benchmark_hardware_command: String,
    pub graphical_dry_run_command: String,
    pub graphical_smoke_command: String,
    pub cpu_benchmark_evidence: String,
    pub gpu_runtime_evidence: String,
    pub graphics_smoke_evidence: String,
    pub launch_window_smoke_status: String,
    pub report_markdown: String,
    pub no_false_gpu_claims: bool,
    pub no_active_readback: bool,
    pub cpu_fallback_works: bool,
}

impl GpuGraphicsPerformanceEvidenceSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != S08_GPU_GRAPHICS_PERFORMANCE_SCHEMA
            || self.schema_version != S08_GPU_GRAPHICS_PERFORMANCE_SCHEMA_VERSION
            || self.benchmark_smoke_command.is_empty()
            || !self.benchmark_gpu_runtime_command.contains("--gpu-runtime")
            || !self
                .benchmark_hardware_command
                .contains("ALIFE_GPU_RUNTIME_BACKEND=static")
            || !self.graphical_dry_run_command.contains("-DryRun")
            || !self.graphical_smoke_command.contains("-SmokeSeconds 5")
            || self.cpu_benchmark_evidence.is_empty()
            || self.gpu_runtime_evidence.is_empty()
            || self.graphics_smoke_evidence.is_empty()
            || self.launch_window_smoke_status.is_empty()
            || !self.no_false_gpu_claims
            || !self.no_active_readback
            || !self.cpu_fallback_works
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        if self.report_markdown.contains("gpu-report")
            || self.report_markdown.contains("ALIFE_GPU_BACKEND")
            || self.report_markdown.contains("bash scripts/check.sh")
            || !self
                .report_markdown
                .contains("CPU fallback is not GPU performance")
            || !self.report_markdown.contains("manual/unknown")
            || !self.report_markdown.contains("60 FPS")
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        self.settings_panel.validate()?;
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}",
            self.schema_version,
            self.settings_panel.signature_line(),
            self.cpu_fallback_works,
            self.no_active_readback,
            self.launch_window_smoke_status
        )
    }
}

pub fn run_gpu_graphics_performance_evidence_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<GpuGraphicsPerformanceEvidenceSummary, GameAppShellError> {
    let gpu = run_gpu_product_hardening_smoke()?;
    let population = run_population_performance_lod_smoke(launch)?;
    let graphical = validate_graphical_playground_launch(&GraphicalPlaygroundLaunchConfig::smoke(
        &launch.fixture_root,
        5,
    ))?;
    let settings_panel = gpu_graphics_performance_settings_panel(&gpu, &population)?;
    let benchmark_smoke_command = "cargo run -p alife_tools --bin benchmark_tiers".to_string();
    let benchmark_gpu_runtime_command =
        "cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime".to_string();
    let graphical_dry_run_command =
        "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -DryRun"
            .to_string();
    let graphical_smoke_command =
        "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -SmokeSeconds 5 -RecordPerformance"
            .to_string();
    let launch_window_smoke_status = if graphical.smoke_seconds == Some(5)
        && graphical.cpu_fallback_visible
        && graphical.player_view_acceptance.dev_overlay_hidden
        && graphical
            .player_view_acceptance
            .stable_id_labels_hidden_except_selected
    {
        "configured-ci-safe-smoke; real window timing remains manual unless captured".to_string()
    } else {
        "not-configured".to_string()
    };
    let report_markdown = s08_gpu_graphics_performance_report_markdown(
        &settings_panel,
        &benchmark_smoke_command,
        &benchmark_gpu_runtime_command,
        &gpu.manual_hardware_command,
        &graphical_dry_run_command,
        &graphical_smoke_command,
        &launch_window_smoke_status,
    );

    let summary = GpuGraphicsPerformanceEvidenceSummary {
        schema: S08_GPU_GRAPHICS_PERFORMANCE_SCHEMA,
        schema_version: S08_GPU_GRAPHICS_PERFORMANCE_SCHEMA_VERSION,
        settings_panel,
        benchmark_smoke_command,
        benchmark_gpu_runtime_command,
        benchmark_hardware_command: gpu.manual_hardware_command,
        graphical_dry_run_command,
        graphical_smoke_command,
        cpu_benchmark_evidence:
            "benchmark_tiers smoke measures CPU reference tiers 1 and 10 in target/artifacts"
                .to_string(),
        gpu_runtime_evidence:
            "gpu-runtime command may select CPU fallback; GPU timing stays manual/unknown unless hardware flags and validation are set"
                .to_string(),
        graphics_smoke_evidence:
            "graphical smoke command opens the feature-gated Bevy window when local graphics are available; dry-run is not graphical proof"
                .to_string(),
        launch_window_smoke_status,
        report_markdown,
        no_false_gpu_claims: !gpu.telemetry_overlay.measured_gpu_performance
            && gpu.performance_claim_status == "unknown-unless-measured",
        no_active_readback: gpu.active_readback_blocked
            && gpu.telemetry_overlay.no_active_gameplay_readback,
        cpu_fallback_works: gpu.cpu_fallback_default
            && gpu.telemetry_overlay.selected_backend == "CpuReference",
    };
    summary.validate()?;
    Ok(summary)
}

pub fn gpu_graphics_performance_settings_panel(
    gpu: &GpuProductHardeningSummary,
    population: &PopulationPerformanceOverlaySummary,
) -> Result<GpuGraphicsPerformanceSettingsPanel, ScaffoldContractError> {
    gpu.validate()?;
    population.validate()?;
    let gpu_evidence_status = if gpu.telemetry_overlay.measured_gpu_performance {
        S08EvidenceStatus::Measured
    } else if gpu.telemetry_overlay.selected_backend == "CpuReference" {
        S08EvidenceStatus::FallbackOnly
    } else {
        S08EvidenceStatus::ManualUnknown
    };
    let fps_target_status = if gpu.telemetry_overlay.measured_gpu_performance {
        S08EvidenceStatus::Measured
    } else {
        S08EvidenceStatus::ManualUnknown
    };
    let graphics_evidence_status = S08EvidenceStatus::ManualUnknown;
    let status_line = s08_settings_status_line(
        &gpu.telemetry_overlay.selected_backend,
        gpu.telemetry_overlay.fallback_reason.as_deref(),
        gpu_evidence_status,
        fps_target_status,
    );
    let panel = GpuGraphicsPerformanceSettingsPanel {
        schema: S08_GPU_GRAPHICS_PERFORMANCE_SCHEMA,
        schema_version: S08_GPU_GRAPHICS_PERFORMANCE_SCHEMA_VERSION,
        target_fps: S08_TARGET_FPS,
        target_frame_ms: population.policy.target_frame_ms,
        requested_backend: gpu.telemetry_overlay.requested_backend.clone(),
        selected_backend: gpu.telemetry_overlay.selected_backend.clone(),
        fallback_reason: gpu.telemetry_overlay.fallback_reason.clone(),
        gpu_runtime_feature_compiled: gpu.telemetry_overlay.gpu_runtime_feature_compiled,
        cpu_oracle_authoritative: gpu.telemetry_overlay.cpu_oracle_authoritative,
        no_active_gameplay_readback: gpu.telemetry_overlay.no_active_gameplay_readback,
        measured_gpu_performance: gpu.telemetry_overlay.measured_gpu_performance,
        gpu_neural_time_ms: gpu.telemetry_overlay.gpu_neural_time_ms,
        fps_target_status,
        gpu_evidence_status,
        graphics_evidence_status,
        status_line,
    };
    panel.validate()?;
    Ok(panel)
}

pub fn s08_settings_status_line(
    selected_backend: &str,
    fallback_reason: Option<&str>,
    gpu_evidence_status: S08EvidenceStatus,
    fps_target_status: S08EvidenceStatus,
) -> String {
    format!(
        "S08 GPU/Graphics: backend={} fallback={} gpu_evidence={} 60 FPS target={} | CPU fallback is not GPU performance | no active neural readback",
        selected_backend,
        fallback_reason.unwrap_or("none"),
        gpu_evidence_status.label(),
        fps_target_status.label(),
    )
}

pub fn s08_runtime_overlay_status_line() -> String {
    s08_settings_status_line(
        "CpuReference",
        Some("default-safe-path"),
        S08EvidenceStatus::FallbackOnly,
        S08EvidenceStatus::ManualUnknown,
    )
}

pub fn s08_gpu_graphics_performance_report_markdown(
    settings: &GpuGraphicsPerformanceSettingsPanel,
    benchmark_smoke_command: &str,
    benchmark_gpu_runtime_command: &str,
    benchmark_hardware_command: &str,
    graphical_dry_run_command: &str,
    graphical_smoke_command: &str,
    launch_window_smoke_status: &str,
) -> String {
    format!(
        concat!(
            "# S08 GPU, Graphics, and Performance Evidence\n\n",
            "Status: product evidence is measured where commands ran, otherwise manual/unknown.\n\n",
            "## Current Settings Surface\n\n",
            "- Requested backend: `{}`\n",
            "- Selected backend: `{}`\n",
            "- Fallback reason: `{:?}`\n",
            "- GPU runtime feature compiled: `{}`\n",
            "- CPU oracle authoritative: `{}`\n",
            "- No active neural readback: `{}`\n",
            "- GPU performance evidence: `{}`\n",
            "- Graphics evidence: `{}`\n",
            "- 60 FPS target: `{}` at {:.3} ms/frame target\n\n",
            "{}\n\n",
            "## Commands\n\n",
            "- CPU benchmark smoke: `{}`\n",
            "- GPU runtime fallback report: `{}`\n",
            "- Manual GPU hardware report: `{}`\n",
            "- Graphical dry-run: `{}`\n",
            "- Graphical smoke: `{}`\n\n",
            "## Evidence Boundary\n\n",
            "- CPU fallback is not GPU performance.\n",
            "- The `--gpu-runtime` command may honestly record CPU fallback when hardware or validation flags are unset.\n",
            "- Real GPU timing requires the manual hardware command and validated local hardware.\n",
            "- Real graphical FPS/window evidence requires a local graphical smoke run and screenshot/log capture.\n",
            "- Launch/window smoke status: `{}`.\n"
        ),
        settings.requested_backend,
        settings.selected_backend,
        settings.fallback_reason,
        settings.gpu_runtime_feature_compiled,
        settings.cpu_oracle_authoritative,
        settings.no_active_gameplay_readback,
        settings.gpu_evidence_status.label(),
        settings.graphics_evidence_status.label(),
        settings.fps_target_status.label(),
        settings.target_frame_ms,
        settings.status_line,
        benchmark_smoke_command,
        benchmark_gpu_runtime_command,
        benchmark_hardware_command,
        graphical_dry_run_command,
        graphical_smoke_command,
        launch_window_smoke_status
    )
}
