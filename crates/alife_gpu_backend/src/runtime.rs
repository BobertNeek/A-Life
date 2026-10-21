//! GPU adapter admission and device probing.

use crate::closed_loop_runtime::{required_device_features, required_device_limits};

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
    if let Some(error) =
        adapter_admission_error(info.device_type, info.backend, adapter.features(), &limits)
    {
        return GpuRuntimeHardwareProbe::from_adapter(
            requested_backend,
            info,
            limits,
            false,
            Some(error),
        );
    }

    match adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("alife-local-gpu-runtime-probe-device"),
            required_features: required_device_features(),
            required_limits: required_device_limits(),
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

pub fn required_storage_buffers(_backend: GpuRuntimeBackendKind) -> u32 {
    required_device_limits().max_storage_buffers_per_shader_stage
}

fn adapter_admission_error(
    device_type: wgpu::DeviceType,
    backend: wgpu::Backend,
    features: wgpu::Features,
    limits: &wgpu::Limits,
) -> Option<String> {
    if device_type == wgpu::DeviceType::Cpu || backend == wgpu::Backend::Noop {
        return Some("gameplay requires a hardware GPU adapter".into());
    }
    if !features.contains(required_device_features()) {
        return Some("adapter does not support the timestamp queries required by gameplay".into());
    }
    let required = required_device_limits();
    if !required.check_limits(limits) {
        return Some(format!(
            "adapter limits do not meet gameplay requirements (storage buffers per stage: {} required, {} available)",
            required.max_storage_buffers_per_shader_stage,
            limits.max_storage_buffers_per_shader_stage,
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_admits_only_devices_that_meet_gameplay_requirements() {
        let limits = required_device_limits();
        let hardware = wgpu::DeviceType::DiscreteGpu;
        let backend = wgpu::Backend::Vulkan;
        let features = required_device_features();
        assert!(adapter_admission_error(hardware, backend, features, &limits).is_none());
        assert!(
            adapter_admission_error(wgpu::DeviceType::Cpu, backend, features, &limits).is_some()
        );
        assert!(
            adapter_admission_error(hardware, wgpu::Backend::Noop, features, &limits).is_some()
        );
        assert!(
            adapter_admission_error(hardware, backend, wgpu::Features::empty(), &limits).is_some()
        );
        let mut insufficient = limits;
        insufficient.max_storage_buffers_per_shader_stage -= 1;
        assert!(adapter_admission_error(hardware, backend, features, &insufficient).is_some());
    }
}
