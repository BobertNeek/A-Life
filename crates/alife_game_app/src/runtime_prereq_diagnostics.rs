//! CA42 runtime prerequisite diagnostics for the Windows alpha launcher.
//!
//! This module is app/tooling policy. It probes optional GPU/windowing
//! prerequisites and reports clear fallback/blocking state without changing
//! simulation authority or making GPU mandatory for CI.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePrereqDiagnosticsOptions {
    pub gpu_mode: GraphicalGpuRuntimeMode,
    pub require_gpu: bool,
    pub graphics_backend: String,
    pub log_path: PathBuf,
}

impl RuntimePrereqDiagnosticsOptions {
    pub fn new(
        gpu_mode: GraphicalGpuRuntimeMode,
        require_gpu: bool,
        graphics_backend: impl Into<String>,
        log_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            gpu_mode,
            require_gpu,
            graphics_backend: graphics_backend.into(),
            log_path: log_path.into(),
        }
    }
}

impl Default for RuntimePrereqDiagnosticsOptions {
    fn default() -> Self {
        Self {
            gpu_mode: GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded,
            require_gpu: false,
            graphics_backend: "dx12".to_string(),
            log_path: PathBuf::from("target/artifacts/ca42_runtime_prereq/runtime_prereq.log"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePrereqDiagnosticsSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub requested_gpu_mode: GraphicalGpuRuntimeMode,
    pub requested_backend: String,
    pub selected_backend: String,
    pub fallback_reason: Option<String>,
    pub require_gpu: bool,
    pub would_block_launch: bool,
    pub cpu_fallback_available: bool,
    pub cpu_fallback_degraded_visible: bool,
    pub gpu_probe_attempted: bool,
    pub adapter_available: bool,
    pub device_request_succeeded: bool,
    pub adapter_name: Option<String>,
    pub backend_api: Option<String>,
    pub adapter_type: Option<String>,
    pub driver: Option<String>,
    pub driver_info: Option<String>,
    pub graphics_backend: String,
    pub log_path: PathBuf,
    pub missing_driver_guidance: String,
    pub no_full_action_authoritative_claim: bool,
    pub cpu_shadow_gate_preserved: bool,
}

impl RuntimePrereqDiagnosticsSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != CA42_RUNTIME_PREREQ_SCHEMA
            || self.schema_version != CA42_RUNTIME_PREREQ_SCHEMA_VERSION
            || self.requested_backend.is_empty()
            || self.selected_backend.is_empty()
            || self.graphics_backend.trim().is_empty()
            || self.log_path.as_os_str().is_empty()
            || self.missing_driver_guidance.trim().is_empty()
            || !self.cpu_shadow_gate_preserved
            || !self.no_full_action_authoritative_claim
        {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        if self.require_gpu && self.requested_gpu_mode == GraphicalGpuRuntimeMode::CpuReference {
            return Err(GameAppShellError::InvalidGraphicalLaunch {
                message: "RequireGpu cannot be paired with cpu-reference mode",
            });
        }
        if self.would_block_launch && !self.require_gpu {
            return Err(GameAppShellError::InvalidGraphicalLaunch {
                message: "runtime preflight can only block when RequireGpu is enabled",
            });
        }
        if self.selected_backend == "CpuReference"
            && self.requested_gpu_mode.requests_gpu()
            && self.fallback_reason.is_none()
        {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        if self.fallback_reason.is_some() && !self.cpu_fallback_degraded_visible {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        Ok(())
    }

    pub fn hardware_line(&self) -> String {
        match (&self.adapter_name, &self.backend_api) {
            (Some(name), Some(api)) => {
                let driver = self
                    .driver_info
                    .as_deref()
                    .filter(|value| !value.is_empty())
                    .or(self.driver.as_deref().filter(|value| !value.is_empty()))
                    .unwrap_or("driver-unknown");
                format!("{name} api={api} driver={driver}")
            }
            (Some(name), None) => format!("{name} api=unknown driver=unknown"),
            _ => "unavailable".to_string(),
        }
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{:?}:probe={}:adapter={}:device={}:require={}:block={}:graphics={}:log={}",
            self.schema_version,
            self.requested_gpu_mode.label(),
            self.selected_backend,
            self.fallback_reason,
            self.gpu_probe_attempted,
            self.adapter_available,
            self.device_request_succeeded,
            self.require_gpu,
            self.would_block_launch,
            self.graphics_backend,
            self.log_path.display()
        )
    }
}

pub fn run_runtime_prereq_diagnostics(
    options: &RuntimePrereqDiagnosticsOptions,
) -> Result<RuntimePrereqDiagnosticsSummary, GameAppShellError> {
    if options.require_gpu && !options.gpu_mode.requests_gpu() {
        return Err(GameAppShellError::InvalidGraphicalLaunch {
            message: "RequireGpu needs a GPU runtime mode, not cpu-reference",
        });
    }

    let summary = runtime_prereq_diagnostics_impl(options);
    summary.validate()?;
    Ok(summary)
}

#[cfg(feature = "gpu-runtime")]
fn runtime_prereq_diagnostics_impl(
    options: &RuntimePrereqDiagnosticsOptions,
) -> RuntimePrereqDiagnosticsSummary {
    use alife_gpu_backend::{
        probe_local_wgpu_runtime_for_graphics_backend, GpuRuntimeBackendConfig,
        GpuRuntimeBackendKind, GpuRuntimeFallbackReason,
    };

    let requested_backend = gpu_mode_to_backend(options.gpu_mode);
    if requested_backend == GpuRuntimeBackendKind::CpuReference {
        return build_runtime_prereq_summary(
            options,
            "CpuReference".to_string(),
            "CpuReference".to_string(),
            None,
            false,
            false,
            false,
            None,
            None,
            None,
            None,
            None,
        );
    }

    if std::env::var("ALIFE_GPU_RUNTIME_AVAILABLE").ok().as_deref() == Some("0") {
        return build_runtime_prereq_summary(
            options,
            format!("{requested_backend:?}"),
            "CpuReference".to_string(),
            Some(format!(
                "{:?}",
                GpuRuntimeFallbackReason::HardwareUnavailable
            )),
            false,
            false,
            false,
            None,
            None,
            None,
            None,
            Some("ALIFE_GPU_RUNTIME_AVAILABLE=0 forced GPU unavailable".to_string()),
        );
    }

    let probe =
        probe_local_wgpu_runtime_for_graphics_backend(requested_backend, &options.graphics_backend);
    let status = GpuRuntimeBackendConfig::request(requested_backend)
        .with_gpu_feature_enabled(true)
        .with_hardware_available(probe.hardware_available())
        .with_validation_passed(probe.error.is_none())
        .with_full_runtime_available(requested_backend == GpuRuntimeBackendKind::GpuFull)
        .select_backend();
    let (selected_backend, fallback_reason) = match status {
        Ok(status) => (
            format!("{:?}", status.selected),
            status.fallback_reason.map(|reason| format!("{reason:?}")),
        ),
        Err(_) => (
            "CpuReference".to_string(),
            Some(format!("{:?}", GpuRuntimeFallbackReason::ValidationFailed)),
        ),
    };
    build_runtime_prereq_summary(
        options,
        format!("{requested_backend:?}"),
        selected_backend,
        fallback_reason,
        true,
        probe.adapter_available,
        probe.device_request_succeeded,
        probe.adapter_name,
        probe.backend_api,
        probe.adapter_type,
        probe.driver,
        probe.driver_info.or(probe.error),
    )
}

#[cfg(not(feature = "gpu-runtime"))]
fn runtime_prereq_diagnostics_impl(
    options: &RuntimePrereqDiagnosticsOptions,
) -> RuntimePrereqDiagnosticsSummary {
    let requested_backend = match options.gpu_mode {
        GraphicalGpuRuntimeMode::CpuReference => "CpuReference",
        GraphicalGpuRuntimeMode::StaticCpuShadowGuarded => "GpuStatic",
        GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded => "GpuPlastic",
        GraphicalGpuRuntimeMode::FullCpuShadowGuarded
        | GraphicalGpuRuntimeMode::AutoWithCpuFallback => "GpuFull",
    };
    let fallback_reason = options
        .gpu_mode
        .requests_gpu()
        .then(|| "FeatureDisabled".to_string());
    let selected_backend = if fallback_reason.is_some() {
        "CpuReference".to_string()
    } else {
        requested_backend.to_string()
    };
    build_runtime_prereq_summary(
        options,
        requested_backend.to_string(),
        selected_backend,
        fallback_reason,
        false,
        false,
        false,
        None,
        None,
        None,
        None,
        Some("gpu-runtime feature disabled".to_string()),
    )
}

#[cfg(feature = "gpu-runtime")]
const fn gpu_mode_to_backend(
    mode: GraphicalGpuRuntimeMode,
) -> alife_gpu_backend::GpuRuntimeBackendKind {
    match mode {
        GraphicalGpuRuntimeMode::CpuReference => {
            alife_gpu_backend::GpuRuntimeBackendKind::CpuReference
        }
        GraphicalGpuRuntimeMode::StaticCpuShadowGuarded => {
            alife_gpu_backend::GpuRuntimeBackendKind::GpuStatic
        }
        GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded => {
            alife_gpu_backend::GpuRuntimeBackendKind::GpuPlastic
        }
        GraphicalGpuRuntimeMode::FullCpuShadowGuarded
        | GraphicalGpuRuntimeMode::AutoWithCpuFallback => {
            alife_gpu_backend::GpuRuntimeBackendKind::GpuFull
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_runtime_prereq_summary(
    options: &RuntimePrereqDiagnosticsOptions,
    requested_backend: String,
    selected_backend: String,
    fallback_reason: Option<String>,
    gpu_probe_attempted: bool,
    adapter_available: bool,
    device_request_succeeded: bool,
    adapter_name: Option<String>,
    backend_api: Option<String>,
    adapter_type: Option<String>,
    driver: Option<String>,
    driver_info: Option<String>,
) -> RuntimePrereqDiagnosticsSummary {
    let fallback_active = fallback_reason.is_some();
    RuntimePrereqDiagnosticsSummary {
        schema: CA42_RUNTIME_PREREQ_SCHEMA,
        schema_version: CA42_RUNTIME_PREREQ_SCHEMA_VERSION,
        requested_gpu_mode: options.gpu_mode,
        requested_backend,
        selected_backend,
        fallback_reason,
        require_gpu: options.require_gpu,
        would_block_launch: options.require_gpu && fallback_active,
        cpu_fallback_available: true,
        cpu_fallback_degraded_visible: fallback_active,
        gpu_probe_attempted,
        adapter_available,
        device_request_succeeded,
        adapter_name,
        backend_api,
        adapter_type,
        driver,
        driver_info,
        graphics_backend: options.graphics_backend.clone(),
        log_path: options.log_path.clone(),
        missing_driver_guidance:
            "If GPU is unavailable, update NVIDIA/AMD/Intel drivers, verify DirectX 12 or Vulkan support, try -GraphicsBackend dx12 or vulkan, and rerun with -RequireGpu only when testing GPU hardware."
                .to_string(),
        no_full_action_authoritative_claim: true,
        cpu_shadow_gate_preserved: true,
    }
}
