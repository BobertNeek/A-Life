//! P29 runtime integration contracts for optional GPU execution.
//!
//! This module does not replace the CPU reference oracle and does not perform
//! active gameplay readback. It records selectable backend modes, fallback
//! reasons, boundary-scoped diagnostics, throttling decisions, and honest
//! performance-tier report shells for hardware/manual runs.

use alife_core::{validate_finite, LobeKind, ScaffoldContractError};

use crate::{
    GPU_BUFFER_CONTRACT_SCHEMA_VERSION, P27_PLASTICITY_STORAGE_BINDINGS,
    P27_STATIC_FORWARD_STORAGE_BINDINGS,
};

pub const P29_RUNTIME_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuRuntimeBackendKind {
    CpuReference,
    GpuStatic,
    GpuPlastic,
    GpuFull,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuRuntimeFallbackReason {
    FeatureDisabled,
    HardwareUnavailable,
    ValidationFailed,
    UnsupportedBackend,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuRuntimeHardwareProbe {
    pub schema_version: u16,
    pub requested_backend: GpuRuntimeBackendKind,
    pub adapter_available: bool,
    pub device_request_succeeded: bool,
    pub adapter_name: Option<String>,
    pub backend_api: Option<String>,
    pub adapter_type: Option<String>,
    pub vendor_id: Option<u32>,
    pub device_id: Option<u32>,
    pub driver: Option<String>,
    pub driver_info: Option<String>,
    pub required_storage_buffers_per_shader_stage: u32,
    pub adapter_storage_buffers_per_shader_stage: Option<u32>,
    pub error: Option<String>,
}

impl GpuRuntimeHardwareProbe {
    pub fn unavailable(requested_backend: GpuRuntimeBackendKind, error: impl Into<String>) -> Self {
        Self {
            schema_version: P29_RUNTIME_SCHEMA_VERSION,
            requested_backend,
            adapter_available: false,
            device_request_succeeded: false,
            adapter_name: None,
            backend_api: None,
            adapter_type: None,
            vendor_id: None,
            device_id: None,
            driver: None,
            driver_info: None,
            required_storage_buffers_per_shader_stage: required_storage_buffers(requested_backend),
            adapter_storage_buffers_per_shader_stage: None,
            error: Some(error.into()),
        }
    }

    pub const fn hardware_available(&self) -> bool {
        self.adapter_available && self.device_request_succeeded
    }

    pub fn hardware_identifier(&self) -> Option<String> {
        self.adapter_name.as_ref().map(|name| {
            let backend = self.backend_api.as_deref().unwrap_or("unknown-backend");
            let adapter_type = self.adapter_type.as_deref().unwrap_or("unknown-type");
            let driver = self.driver_info.as_deref().or(self.driver.as_deref());
            match driver {
                Some(driver) if !driver.is_empty() => {
                    format!("{name} ({backend}, {adapter_type}, {driver})")
                }
                _ => format!("{name} ({backend}, {adapter_type})"),
            }
        })
    }

    fn from_adapter(
        requested_backend: GpuRuntimeBackendKind,
        info: wgpu::AdapterInfo,
        limits: wgpu::Limits,
        device_request_succeeded: bool,
        error: Option<String>,
    ) -> Self {
        Self {
            schema_version: P29_RUNTIME_SCHEMA_VERSION,
            requested_backend,
            adapter_available: true,
            device_request_succeeded,
            adapter_name: Some(info.name),
            backend_api: Some(format!("{:?}", info.backend)),
            adapter_type: Some(format!("{:?}", info.device_type)),
            vendor_id: Some(info.vendor),
            device_id: Some(info.device),
            driver: Some(info.driver),
            driver_info: Some(info.driver_info),
            required_storage_buffers_per_shader_stage: required_storage_buffers(requested_backend),
            adapter_storage_buffers_per_shader_stage: Some(
                limits.max_storage_buffers_per_shader_stage,
            ),
            error,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuRuntimeBackendStatus {
    pub schema_version: u16,
    pub requested: GpuRuntimeBackendKind,
    pub selected: GpuRuntimeBackendKind,
    pub fallback_reason: Option<GpuRuntimeFallbackReason>,
    pub cpu_oracle_authoritative: bool,
    pub static_forward_parity_checked: bool,
    pub plasticity_parity_checked: bool,
    pub no_active_gameplay_readback: bool,
}

impl GpuRuntimeBackendStatus {
    fn cpu_fallback(
        requested: GpuRuntimeBackendKind,
        fallback_reason: GpuRuntimeFallbackReason,
    ) -> Self {
        Self {
            schema_version: P29_RUNTIME_SCHEMA_VERSION,
            requested,
            selected: GpuRuntimeBackendKind::CpuReference,
            fallback_reason: Some(fallback_reason),
            cpu_oracle_authoritative: true,
            static_forward_parity_checked: false,
            plasticity_parity_checked: false,
            no_active_gameplay_readback: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuRuntimeBackendConfig {
    pub requested: GpuRuntimeBackendKind,
    pub gpu_feature_enabled: bool,
    pub hardware_available: bool,
    pub validation_passed: bool,
    pub static_forward_available: bool,
    pub plasticity_available: bool,
    pub routing_masks_available: bool,
    pub sleep_recompaction_available: bool,
    pub full_runtime_available: bool,
}

impl Default for GpuRuntimeBackendConfig {
    fn default() -> Self {
        Self {
            requested: GpuRuntimeBackendKind::CpuReference,
            gpu_feature_enabled: false,
            hardware_available: false,
            validation_passed: true,
            static_forward_available: true,
            plasticity_available: true,
            routing_masks_available: true,
            sleep_recompaction_available: true,
            full_runtime_available: false,
        }
    }
}

impl GpuRuntimeBackendConfig {
    pub fn request(requested: GpuRuntimeBackendKind) -> Self {
        Self {
            requested,
            gpu_feature_enabled: requested != GpuRuntimeBackendKind::CpuReference,
            ..Self::default()
        }
    }

    pub const fn with_gpu_feature_enabled(mut self, gpu_feature_enabled: bool) -> Self {
        self.gpu_feature_enabled = gpu_feature_enabled;
        self
    }

    pub const fn with_hardware_available(mut self, hardware_available: bool) -> Self {
        self.hardware_available = hardware_available;
        self
    }

    pub const fn with_validation_passed(mut self, validation_passed: bool) -> Self {
        self.validation_passed = validation_passed;
        self
    }

    pub const fn with_full_runtime_available(mut self, full_runtime_available: bool) -> Self {
        self.full_runtime_available = full_runtime_available;
        self
    }

    pub fn select_backend(self) -> Result<GpuRuntimeBackendStatus, ScaffoldContractError> {
        if self.requested == GpuRuntimeBackendKind::CpuReference {
            return Ok(GpuRuntimeBackendStatus {
                schema_version: P29_RUNTIME_SCHEMA_VERSION,
                requested: self.requested,
                selected: GpuRuntimeBackendKind::CpuReference,
                fallback_reason: None,
                cpu_oracle_authoritative: true,
                static_forward_parity_checked: false,
                plasticity_parity_checked: false,
                no_active_gameplay_readback: true,
            });
        }
        if !self.gpu_feature_enabled {
            return Ok(GpuRuntimeBackendStatus::cpu_fallback(
                self.requested,
                GpuRuntimeFallbackReason::FeatureDisabled,
            ));
        }
        if !self.hardware_available {
            return Ok(GpuRuntimeBackendStatus::cpu_fallback(
                self.requested,
                GpuRuntimeFallbackReason::HardwareUnavailable,
            ));
        }
        if !self.validation_passed {
            return Ok(GpuRuntimeBackendStatus::cpu_fallback(
                self.requested,
                GpuRuntimeFallbackReason::ValidationFailed,
            ));
        }

        let supported = match self.requested {
            GpuRuntimeBackendKind::CpuReference => true,
            GpuRuntimeBackendKind::GpuStatic => self.static_forward_available,
            GpuRuntimeBackendKind::GpuPlastic => {
                self.static_forward_available && self.plasticity_available
            }
            GpuRuntimeBackendKind::GpuFull => {
                self.static_forward_available
                    && self.plasticity_available
                    && self.routing_masks_available
                    && self.sleep_recompaction_available
                    && self.full_runtime_available
            }
        };
        if !supported {
            return Ok(GpuRuntimeBackendStatus::cpu_fallback(
                self.requested,
                GpuRuntimeFallbackReason::UnsupportedBackend,
            ));
        }

        Ok(GpuRuntimeBackendStatus {
            schema_version: P29_RUNTIME_SCHEMA_VERSION,
            requested: self.requested,
            selected: self.requested,
            fallback_reason: None,
            cpu_oracle_authoritative: true,
            static_forward_parity_checked: true,
            plasticity_parity_checked: matches!(
                self.requested,
                GpuRuntimeBackendKind::GpuPlastic | GpuRuntimeBackendKind::GpuFull
            ),
            no_active_gameplay_readback: true,
        })
    }
}

pub fn probe_local_wgpu_runtime(
    requested_backend: GpuRuntimeBackendKind,
) -> GpuRuntimeHardwareProbe {
    pollster::block_on(probe_local_wgpu_runtime_async(requested_backend))
}

async fn probe_local_wgpu_runtime_async(
    requested_backend: GpuRuntimeBackendKind,
) -> GpuRuntimeHardwareProbe {
    if requested_backend == GpuRuntimeBackendKind::CpuReference {
        return GpuRuntimeHardwareProbe {
            schema_version: P29_RUNTIME_SCHEMA_VERSION,
            requested_backend,
            adapter_available: false,
            device_request_succeeded: false,
            adapter_name: None,
            backend_api: None,
            adapter_type: None,
            vendor_id: None,
            device_id: None,
            driver: None,
            driver_info: None,
            required_storage_buffers_per_shader_stage: 0,
            adapter_storage_buffers_per_shader_stage: None,
            error: None,
        };
    }

    let instance = wgpu::Instance::default();
    let adapter = match instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
    {
        Ok(adapter) => adapter,
        Err(error) => {
            return GpuRuntimeHardwareProbe::unavailable(
                requested_backend,
                format!("wgpu adapter request failed: {error}"),
            );
        }
    };

    let info = adapter.get_info();
    let limits = adapter.limits();
    let required_storage_buffers = required_storage_buffers(requested_backend);
    if limits.max_storage_buffers_per_shader_stage < required_storage_buffers {
        let exposed_storage_buffers = limits.max_storage_buffers_per_shader_stage;
        return GpuRuntimeHardwareProbe::from_adapter(
            requested_backend,
            info,
            limits,
            false,
            Some(format!(
                "adapter exposes {} storage buffers per shader stage, but {:?} requires {}",
                exposed_storage_buffers, requested_backend, required_storage_buffers
            )),
        );
    }

    let mut required_limits = wgpu::Limits::downlevel_defaults();
    required_limits.max_storage_buffers_per_shader_stage = required_limits
        .max_storage_buffers_per_shader_stage
        .max(required_storage_buffers);

    match adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("alife-local-gpu-runtime-probe-device"),
            required_features: wgpu::Features::empty(),
            required_limits,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
        })
        .await
    {
        Ok((_device, _queue)) => {
            GpuRuntimeHardwareProbe::from_adapter(requested_backend, info, limits, true, None)
        }
        Err(error) => GpuRuntimeHardwareProbe::from_adapter(
            requested_backend,
            info,
            limits,
            false,
            Some(format!("wgpu device request failed: {error}")),
        ),
    }
}

pub const fn required_storage_buffers(backend: GpuRuntimeBackendKind) -> u32 {
    match backend {
        GpuRuntimeBackendKind::CpuReference => 0,
        GpuRuntimeBackendKind::GpuStatic => P27_STATIC_FORWARD_STORAGE_BINDINGS,
        GpuRuntimeBackendKind::GpuPlastic | GpuRuntimeBackendKind::GpuFull => {
            if P27_STATIC_FORWARD_STORAGE_BINDINGS > P27_PLASTICITY_STORAGE_BINDINGS {
                P27_STATIC_FORWARD_STORAGE_BINDINGS
            } else {
                P27_PLASTICITY_STORAGE_BINDINGS
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuRuntimeBoundary {
    ActiveTick,
    TickActionSummary,
    FrameBoundary,
    SleepBoundary,
    DiagnosticExport,
    ManualValidation,
    PerformanceReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuRuntimeReadbackGuard {
    pub boundary: GpuRuntimeBoundary,
}

impl GpuRuntimeReadbackGuard {
    pub const fn active_tick() -> Self {
        Self {
            boundary: GpuRuntimeBoundary::ActiveTick,
        }
    }

    pub const fn after_frame_boundary() -> Self {
        Self {
            boundary: GpuRuntimeBoundary::FrameBoundary,
        }
    }

    pub const fn permits_boundary(self, requested: GpuRuntimeBoundary) -> bool {
        match self.boundary {
            GpuRuntimeBoundary::ActiveTick => {
                matches!(requested, GpuRuntimeBoundary::TickActionSummary)
            }
            GpuRuntimeBoundary::FrameBoundary
            | GpuRuntimeBoundary::SleepBoundary
            | GpuRuntimeBoundary::ManualValidation
            | GpuRuntimeBoundary::PerformanceReport
            | GpuRuntimeBoundary::DiagnosticExport => matches!(
                requested,
                GpuRuntimeBoundary::TickActionSummary
                    | GpuRuntimeBoundary::FrameBoundary
                    | GpuRuntimeBoundary::SleepBoundary
                    | GpuRuntimeBoundary::DiagnosticExport
                    | GpuRuntimeBoundary::ManualValidation
                    | GpuRuntimeBoundary::PerformanceReport
            ),
            GpuRuntimeBoundary::TickActionSummary => {
                matches!(requested, GpuRuntimeBoundary::TickActionSummary)
            }
        }
    }

    pub const fn permits_bulk_neural_readback(self) -> bool {
        false
    }

    pub const fn permits_per_synapse_readback(self) -> bool {
        false
    }

    pub const fn permits_per_lobe_readback(self) -> bool {
        false
    }

    pub const fn permits_weight_readback(self) -> bool {
        false
    }

    pub fn validate_export_request(
        self,
        requested: GpuRuntimeBoundary,
    ) -> Result<(), ScaffoldContractError> {
        if !self.permits_boundary(requested)
            || matches!(
                requested,
                GpuRuntimeBoundary::ActiveTick | GpuRuntimeBoundary::TickActionSummary
            )
        {
            return Err(ScaffoldContractError::BackendParity);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuRuntimeTimingBudget {
    pub target_frame_budget_ms: f32,
    pub gpu_neural_budget_ms: f32,
    pub fallback_update_frequency_hz: f32,
}

impl GpuRuntimeTimingBudget {
    pub fn validate(self) -> Result<(), ScaffoldContractError> {
        validate_finite(self.target_frame_budget_ms)?;
        validate_finite(self.gpu_neural_budget_ms)?;
        validate_finite(self.fallback_update_frequency_hz)?;
        if self.target_frame_budget_ms <= 0.0
            || self.gpu_neural_budget_ms <= 0.0
            || self.fallback_update_frequency_hz <= 0.0
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuRuntimeTimingSample {
    pub measured_gpu_neural_ms: f32,
    pub measured_frame_ms: f32,
}

impl GpuRuntimeTimingSample {
    pub fn validate(self) -> Result<(), ScaffoldContractError> {
        validate_finite(self.measured_gpu_neural_ms)?;
        validate_finite(self.measured_frame_ms)?;
        if self.measured_gpu_neural_ms < 0.0 || self.measured_frame_ms < 0.0 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuThrottleLevel {
    None,
    DecimateNonEssential,
    WarmCadenceFallback,
    SleepOnlyNonEssential,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuThrottleReason {
    WithinBudget,
    GpuNeuralOverBudget,
    FrameOverBudget,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuRuntimeThrottleDecision {
    pub level: GpuThrottleLevel,
    pub reason: GpuThrottleReason,
    pub nonessential_decimation_factor: u32,
    pub sensory_motor_protected: bool,
    pub fallback_update_frequency_hz: f32,
    pub protected_lobes: Vec<LobeKind>,
    pub decimated_lobes: Vec<LobeKind>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuRuntimeThrottlingPolicy {
    pub decimate_ratio: f32,
    pub warm_ratio: f32,
    pub sleep_only_ratio: f32,
}

impl GpuRuntimeThrottlingPolicy {
    pub const fn reference() -> Self {
        Self {
            decimate_ratio: 1.0,
            warm_ratio: 2.0,
            sleep_only_ratio: 3.0,
        }
    }

    pub fn decide(
        self,
        budget: GpuRuntimeTimingBudget,
        sample: GpuRuntimeTimingSample,
    ) -> Result<GpuRuntimeThrottleDecision, ScaffoldContractError> {
        budget.validate()?;
        sample.validate()?;
        validate_finite(self.decimate_ratio)?;
        validate_finite(self.warm_ratio)?;
        validate_finite(self.sleep_only_ratio)?;
        if self.decimate_ratio <= 0.0
            || self.decimate_ratio > self.warm_ratio
            || self.warm_ratio > self.sleep_only_ratio
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }

        let gpu_ratio = sample.measured_gpu_neural_ms / budget.gpu_neural_budget_ms;
        let frame_ratio = sample.measured_frame_ms / budget.target_frame_budget_ms;
        let reason = if frame_ratio > 1.0 && frame_ratio >= gpu_ratio {
            GpuThrottleReason::FrameOverBudget
        } else if gpu_ratio > self.decimate_ratio {
            GpuThrottleReason::GpuNeuralOverBudget
        } else {
            GpuThrottleReason::WithinBudget
        };
        let level = if gpu_ratio <= self.decimate_ratio && frame_ratio <= 1.0 {
            GpuThrottleLevel::None
        } else if gpu_ratio >= self.sleep_only_ratio {
            GpuThrottleLevel::SleepOnlyNonEssential
        } else if gpu_ratio >= self.warm_ratio {
            GpuThrottleLevel::WarmCadenceFallback
        } else {
            GpuThrottleLevel::DecimateNonEssential
        };
        let factor = match level {
            GpuThrottleLevel::None => 1,
            GpuThrottleLevel::DecimateNonEssential => 2,
            GpuThrottleLevel::WarmCadenceFallback => 4,
            GpuThrottleLevel::SleepOnlyNonEssential => 8,
        };
        Ok(GpuRuntimeThrottleDecision {
            level,
            reason,
            nonessential_decimation_factor: factor,
            sensory_motor_protected: true,
            fallback_update_frequency_hz: budget.fallback_update_frequency_hz,
            protected_lobes: protected_lobes(),
            decimated_lobes: if level == GpuThrottleLevel::None {
                Vec::new()
            } else {
                nonessential_lobes()
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuRuntimeDiagnosticExport {
    pub schema_version: u16,
    pub boundary: GpuRuntimeBoundary,
    pub backend: GpuRuntimeBackendStatus,
    pub timing: GpuRuntimeTimingSample,
    pub run_notes: String,
}

impl GpuRuntimeDiagnosticExport {
    pub fn new(
        schema_version: u16,
        boundary: GpuRuntimeBoundary,
        backend: GpuRuntimeBackendStatus,
        timing: GpuRuntimeTimingSample,
        run_notes: impl Into<String>,
    ) -> Result<Self, ScaffoldContractError> {
        if schema_version != P29_RUNTIME_SCHEMA_VERSION
            || backend.schema_version != P29_RUNTIME_SCHEMA_VERSION
        {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        GpuRuntimeReadbackGuard { boundary }
            .validate_export_request(GpuRuntimeBoundary::DiagnosticExport)?;
        timing.validate()?;
        Ok(Self {
            schema_version,
            boundary,
            backend,
            timing,
            run_notes: run_notes.into(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuRuntimeCapabilityManifest {
    pub schema_version: u16,
    pub gpu_buffer_schema_version: u16,
    pub static_forward_parity_available: bool,
    pub plasticity_parity_available: bool,
    pub routing_masks_available: bool,
    pub sleep_recompaction_available: bool,
    pub product_gpu_full_runtime_default: bool,
    pub static_forward_storage_bindings: u32,
    pub plasticity_storage_bindings: u32,
    pub no_active_gameplay_neural_readback: bool,
}

impl GpuRuntimeCapabilityManifest {
    pub const fn current_contract() -> Self {
        Self {
            schema_version: P29_RUNTIME_SCHEMA_VERSION,
            gpu_buffer_schema_version: GPU_BUFFER_CONTRACT_SCHEMA_VERSION,
            static_forward_parity_available: true,
            plasticity_parity_available: true,
            routing_masks_available: true,
            sleep_recompaction_available: true,
            product_gpu_full_runtime_default: false,
            static_forward_storage_bindings: P27_STATIC_FORWARD_STORAGE_BINDINGS,
            plasticity_storage_bindings: P27_PLASTICITY_STORAGE_BINDINGS,
            no_active_gameplay_neural_readback: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuTierPopulation {
    One,
    Ten,
    Fifty,
    OneHundred,
    TwoHundredFifty,
    FiveHundred,
}

impl GpuTierPopulation {
    pub const fn required() -> [Self; 6] {
        [
            Self::One,
            Self::Ten,
            Self::Fifty,
            Self::OneHundred,
            Self::TwoHundredFifty,
            Self::FiveHundred,
        ]
    }

    pub const fn count(self) -> u16 {
        match self {
            Self::One => 1,
            Self::Ten => 10,
            Self::Fifty => 50,
            Self::OneHundred => 100,
            Self::TwoHundredFifty => 250,
            Self::FiveHundred => 500,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuPerformanceTargetStatus {
    Met,
    Missed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuTierMeasurement {
    pub population: GpuTierPopulation,
    pub backend: GpuRuntimeBackendKind,
    pub tick_time_ms: Option<f32>,
    pub gpu_neural_time_ms: Option<f32>,
    pub patch_throughput_per_second: Option<f32>,
    pub memory_topology_update_ms: Option<f32>,
    pub sleep_recompaction_ms: Option<f32>,
    pub active_synapses: u32,
    pub active_tiles: u32,
    pub skipped_supertiles: u32,
    pub skipped_tiles: u32,
    pub target_60_fps: GpuPerformanceTargetStatus,
    pub notes: String,
}

impl GpuTierMeasurement {
    pub fn cpu_fallback_report(
        backend: GpuRuntimeBackendStatus,
        notes: impl Into<String>,
    ) -> GpuTierPerformanceReport {
        let notes = notes.into();
        GpuTierPerformanceReport {
            schema_version: P29_RUNTIME_SCHEMA_VERSION,
            backend,
            hardware_identifier: None,
            feature_flags: vec!["cpu-fallback".to_string()],
            measurements: GpuTierPopulation::required()
                .into_iter()
                .map(|population| Self {
                    population,
                    backend: backend.selected,
                    tick_time_ms: None,
                    gpu_neural_time_ms: None,
                    patch_throughput_per_second: None,
                    memory_topology_update_ms: None,
                    sleep_recompaction_ms: None,
                    active_synapses: 0,
                    active_tiles: 0,
                    skipped_supertiles: 0,
                    skipped_tiles: 0,
                    target_60_fps: GpuPerformanceTargetStatus::Unknown,
                    notes: notes.clone(),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuTierPerformanceReport {
    pub schema_version: u16,
    pub backend: GpuRuntimeBackendStatus,
    pub hardware_identifier: Option<String>,
    pub feature_flags: Vec<String>,
    pub measurements: Vec<GpuTierMeasurement>,
}

impl GpuTierPerformanceReport {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != P29_RUNTIME_SCHEMA_VERSION
            || self.backend.schema_version != P29_RUNTIME_SCHEMA_VERSION
            || self.measurements.len() != GpuTierPopulation::required().len()
        {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        for (actual, expected) in self.measurements.iter().zip(GpuTierPopulation::required()) {
            if actual.population != expected {
                return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
            }
            for value in [
                actual.tick_time_ms,
                actual.gpu_neural_time_ms,
                actual.patch_throughput_per_second,
                actual.memory_topology_update_ms,
                actual.sleep_recompaction_ms,
            ]
            .into_iter()
            .flatten()
            {
                validate_finite(value)?;
                if value < 0.0 {
                    return Err(ScaffoldContractError::ScalarOutOfRange);
                }
            }
        }
        Ok(())
    }

    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("# P29 GPU runtime performance report\n\n");
        out.push_str(&format!(
            "- Backend requested: {:?}\n- Backend selected: {:?}\n- Fallback reason: {:?}\n- Hardware: {}\n- Feature flags/evidence: {}\n- No active gameplay neural readback: {}\n\n",
            self.backend.requested,
            self.backend.selected,
            self.backend.fallback_reason,
            self.hardware_identifier.as_deref().unwrap_or("unknown"),
            if self.feature_flags.is_empty() {
                "none".to_string()
            } else {
                self.feature_flags.join(", ")
            },
            self.backend.no_active_gameplay_readback,
        ));
        out.push_str(
            "| Population | Backend | Tick ms | GPU neural ms | 60 FPS target | Notes |\n",
        );
        out.push_str("|---:|---|---:|---:|---|---|\n");
        for measurement in &self.measurements {
            out.push_str(&format!(
                "| {} | {:?} | {} | {} | {:?} | {} |\n",
                measurement.population.count(),
                measurement.backend,
                optional_ms(measurement.tick_time_ms),
                optional_ms(measurement.gpu_neural_time_ms),
                measurement.target_60_fps,
                measurement.notes,
            ));
        }
        out.push_str("\n## Boundary policy\n\n");
        out.push_str("- No active gameplay neural readback.\n");
        out.push_str("- Diagnostics/export snapshots are frame, sleep, manual, or performance-report boundary scoped.\n");
        out
    }
}

fn protected_lobes() -> Vec<LobeKind> {
    vec![
        LobeKind::SensoryGrounding,
        LobeKind::MetabolicDrive,
        LobeKind::MotorArbitration,
        LobeKind::HomeostaticRegulation,
    ]
}

fn nonessential_lobes() -> Vec<LobeKind> {
    vec![
        LobeKind::AuditorySpeech,
        LobeKind::GlyphVision,
        LobeKind::LexiconConcept,
        LobeKind::CoreAssociation,
        LobeKind::EpisodicMemory,
        LobeKind::WorkingMemory,
    ]
}

fn optional_ms(value: Option<f32>) -> String {
    value.map_or_else(|| "unknown".to_string(), |value| format!("{value:.3}"))
}
