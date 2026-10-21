//! P29 runtime integration contracts for required GPU execution.

use alife_core::ScaffoldContractError;

use crate::GPU_CLOSED_LOOP_STORAGE_BINDINGS;

pub const P29_RUNTIME_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuRuntimeBackendKind {
    GpuAuthoritative,
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
    pub authoritative: bool,
    pub unavailable_reason: Option<&'static str>,
    pub no_active_gameplay_readback: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuRuntimeBackendConfig {
    pub requested: GpuRuntimeBackendKind,
    pub gpu_feature_enabled: bool,
    pub hardware_available: bool,
    pub validation_passed: bool,
}

impl Default for GpuRuntimeBackendConfig {
    fn default() -> Self {
        Self {
            requested: GpuRuntimeBackendKind::GpuAuthoritative,
            gpu_feature_enabled: true,
            hardware_available: false,
            validation_passed: true,
        }
    }
}

impl GpuRuntimeBackendConfig {
    pub fn request(requested: GpuRuntimeBackendKind) -> Self {
        Self {
            requested,
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

    pub fn select_backend(self) -> Result<GpuRuntimeBackendStatus, ScaffoldContractError> {
        if !self.gpu_feature_enabled {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        if !self.hardware_available {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        if !self.validation_passed {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }

        Ok(GpuRuntimeBackendStatus {
            schema_version: P29_RUNTIME_SCHEMA_VERSION,
            requested: self.requested,
            selected: self.requested,
            authoritative: true,
            unavailable_reason: None,
            no_active_gameplay_readback: true,
        })
    }
}

pub fn probe_local_wgpu_runtime(
    requested_backend: GpuRuntimeBackendKind,
) -> GpuRuntimeHardwareProbe {
    probe_local_wgpu_runtime_with_backends(requested_backend, None)
}

pub fn probe_local_wgpu_runtime_for_graphics_backend(
    requested_backend: GpuRuntimeBackendKind,
    graphics_backend: &str,
) -> GpuRuntimeHardwareProbe {
    let backends = match graphics_backend.trim().to_ascii_lowercase().as_str() {
        "dx12" | "d3d12" => Some(wgpu::Backends::DX12),
        "vulkan" | "vk" => Some(wgpu::Backends::VULKAN),
        "auto" | "existing" | "" => None,
        _ => None,
    };
    probe_local_wgpu_runtime_with_backends(requested_backend, backends)
}

pub fn probe_local_wgpu_runtime_with_backends(
    requested_backend: GpuRuntimeBackendKind,
    backends: Option<wgpu::Backends>,
) -> GpuRuntimeHardwareProbe {
    pollster::block_on(probe_local_wgpu_runtime_async(requested_backend, backends))
}

async fn probe_local_wgpu_runtime_async(
    requested_backend: GpuRuntimeBackendKind,
    backends: Option<wgpu::Backends>,
) -> GpuRuntimeHardwareProbe {
    let instance = if let Some(backends) = backends {
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = backends;
        wgpu::Instance::new(descriptor)
    } else {
        wgpu::Instance::default()
    };
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
        GpuRuntimeBackendKind::GpuAuthoritative => GPU_CLOSED_LOOP_STORAGE_BINDINGS,
    }
}
