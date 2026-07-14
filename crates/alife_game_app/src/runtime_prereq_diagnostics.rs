//! Required-GPU runtime prerequisite diagnostics for product launch.

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
        Self::new(
            GraphicalBrainPolicyMode::GpuRequired,
            true,
            "vulkan",
            "target/artifacts/ca42_runtime_prereq/runtime_prereq.log",
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePrereqDiagnosticsSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub requested_gpu_mode: GraphicalGpuRuntimeMode,
    pub requested_backend: String,
    pub selected_backend: String,
    pub unavailable_reason: Option<String>,
    pub require_gpu: bool,
    pub would_block_launch: bool,
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
    pub authoritative: bool,
    pub failure_stops_learned_actions: bool,
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
            || !self.failure_stops_learned_actions
            || (self.authoritative && !self.device_request_succeeded)
            || (self.would_block_launch && !self.require_gpu)
        {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        Ok(())
    }

    pub fn hardware_line(&self) -> String {
        match (&self.adapter_name, &self.backend_api) {
            (Some(name), Some(api)) => format!("{name} api={api}"),
            _ => "unavailable".to_string(),
        }
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{:?}:probe={}:adapter={}:device={}:require={}:block={}:graphics={}:log={}",
            self.schema_version,
            self.requested_gpu_mode.label(),
            self.selected_backend,
            self.unavailable_reason,
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
    let summary = runtime_prereq_diagnostics_impl(options);
    summary.validate()?;
    Ok(summary)
}

#[cfg(feature = "gpu-runtime")]
fn runtime_prereq_diagnostics_impl(
    options: &RuntimePrereqDiagnosticsOptions,
) -> RuntimePrereqDiagnosticsSummary {
    let probe = alife_gpu_backend::probe_local_wgpu_runtime_for_graphics_backend(
        alife_gpu_backend::GpuRuntimeBackendKind::GpuAuthoritative,
        &options.graphics_backend,
    );
    let available = probe.hardware_available() && probe.error.is_none();
    build_summary(
        options,
        available,
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
    build_summary(
        options,
        false,
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

#[allow(clippy::too_many_arguments)]
fn build_summary(
    options: &RuntimePrereqDiagnosticsOptions,
    available: bool,
    probe_attempted: bool,
    adapter_available: bool,
    device_request_succeeded: bool,
    adapter_name: Option<String>,
    backend_api: Option<String>,
    adapter_type: Option<String>,
    driver: Option<String>,
    driver_info: Option<String>,
) -> RuntimePrereqDiagnosticsSummary {
    RuntimePrereqDiagnosticsSummary {
        schema: CA42_RUNTIME_PREREQ_SCHEMA,
        schema_version: CA42_RUNTIME_PREREQ_SCHEMA_VERSION,
        requested_gpu_mode: options.gpu_mode,
        requested_backend: "GpuAuthoritative".to_string(),
        selected_backend: if available { "GpuAuthoritative" } else { "Unavailable" }.to_string(),
        unavailable_reason: (!available).then(|| {
            driver_info
                .clone()
                .unwrap_or_else(|| "required GPU unavailable".to_string())
        }),
        require_gpu: options.require_gpu,
        would_block_launch: options.require_gpu && !available,
        gpu_probe_attempted: probe_attempted,
        adapter_available,
        device_request_succeeded,
        adapter_name,
        backend_api,
        adapter_type,
        driver,
        driver_info,
        graphics_backend: options.graphics_backend.clone(),
        log_path: options.log_path.clone(),
        missing_driver_guidance: "Update the GPU driver, verify Vulkan support, and retry; learned actions remain stopped while unavailable.".to_string(),
        authoritative: available,
        failure_stops_learned_actions: true,
    }
}
