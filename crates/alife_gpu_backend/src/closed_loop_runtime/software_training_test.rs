//! Explicit software Vulkan for offline unit fixtures, never normal admission.
use super::*;

fn is_software_training_candidate(kind: wgpu::DeviceType, backend: wgpu::Backend) -> bool {
    kind == wgpu::DeviceType::Cpu && backend == wgpu::Backend::Vulkan
}

impl GpuClosedLoopBackend {
    /// Creates a device-backed OFFLINE TRAINING TEST fixture on a real CPU
    /// Vulkan adapter. Requires `gpu-tests`; never falls back to a physical
    /// adapter or Noop. Adapter/driver/features/limits remain truthful.
    ///
    /// The ordinary required-hardware factory is unchanged. Live brain
    /// admission/dispatch and world-restore staging reject this fixture.
    pub fn new_software_vulkan_training_test(
        profile: GpuRuntimeProfile,
    ) -> Result<Self, ScaffoldContractError> {
        profile.validate_contract()?;
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = wgpu::Backends::VULKAN;
        let instance = wgpu::Instance::new(descriptor);
        let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::VULKAN));
        let required_features = required_device_features();
        let required_limits = required_device_limits();
        for adapter in adapters {
            let info = adapter.get_info();
            if !is_software_training_candidate(info.device_type, info.backend)
                || !required_limits.check_limits(&adapter.limits())
            {
                continue;
            }
            validate_required_device_features(adapter.features())?;
            let (device, queue) = WgpuDeviceFactory.request_device(
                &adapter,
                &wgpu::DeviceDescriptor {
                    label: Some("alife-explicit-software-offline-training-test"),
                    required_features,
                    required_limits: required_limits.clone(),
                    experimental_features: wgpu::ExperimentalFeatures::disabled(),
                    memory_hints: wgpu::MemoryHints::MemoryUsage,
                    trace: wgpu::Trace::Off,
                },
            )?;
            let lost = Arc::new(AtomicBool::new(false));
            let callback_lost = Arc::clone(&lost);
            device.set_device_lost_callback(move |_reason, _message| {
                callback_lost.store(true, Ordering::Release);
            });
            let hardware = build_hardware_receipt(
                &info,
                required_features,
                device.features(),
                &device.limits(),
            )?;
            let mut backend = Self::from_required_gpu(
                RequiredGpuDevice {
                    adapter,
                    device,
                    queue,
                    hardware,
                    lost,
                },
                profile,
            )?;
            backend.offline_software_training_test = true;
            return Ok(backend);
        }
        Err(ScaffoldContractError::NeuralBackendUnavailable)
    }

    /// Marks test context, never a production-hardware qualification.
    pub const fn is_software_training_test(&self) -> bool {
        self.offline_software_training_test
    }

    /// Actual adapter identity, including `DeviceType::Cpu`; never relabelled.
    pub fn software_training_test_adapter_info(&self) -> Option<wgpu::AdapterInfo> {
        self.offline_software_training_test
            .then(|| self.adapter.get_info())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn software_training_selection_is_cpu_vulkan_only() {
        assert!(is_software_training_candidate(
            wgpu::DeviceType::Cpu,
            wgpu::Backend::Vulkan
        ));
        assert!(!is_software_training_candidate(
            wgpu::DeviceType::Cpu,
            wgpu::Backend::Noop
        ));
        assert!(!is_software_training_candidate(
            wgpu::DeviceType::DiscreteGpu,
            wgpu::Backend::Vulkan
        ));
        assert!(!is_software_training_candidate(
            wgpu::DeviceType::Cpu,
            wgpu::Backend::Gl
        ));
    }

    #[test]
    #[ignore = "explicit software Vulkan device fixture; not hardware qualification"]
    fn software_training_device_is_truthful_and_normal_factory_still_rejects_it() {
        let mut backend = GpuClosedLoopBackend::new_software_vulkan_training_test(
            GpuRuntimeProfile::production_v1(),
        )
        .unwrap();
        let info = backend.software_training_test_adapter_info().unwrap();
        assert_eq!(info.device_type, wgpu::DeviceType::Cpu);
        assert_eq!(info.backend, wgpu::Backend::Vulkan);
        assert_eq!(
            backend.hardware_receipt().adapter_name,
            truncate_utf8(&info.name, 256)
        );
        assert_eq!(backend.hardware_receipt().vendor_id, info.vendor);
        assert_eq!(backend.hardware_receipt().device_id, info.device);
        assert_eq!(backend.hardware_receipt().backend_api, "vulkan");
        assert_eq!(
            backend.hardware_receipt().feature_digest,
            canonical_feature_digest(required_device_features(), backend.device.features())
        );
        assert_eq!(
            backend.hardware_receipt().limits_digest,
            canonical_limits_digest(&backend.device.limits())
        );
        assert_eq!(backend.admission_receipt().live_brains, 0);
        assert_eq!(backend.admission_receipt().physical_allocated_bytes, 0);
        assert!(backend.offline_training_device_queue().is_ok());
        assert!(backend.ensure_ready().is_err());
        assert!(backend.new_staging_like_live().is_err());

        struct ActualCpuFactory(wgpu::Adapter);
        impl GpuDeviceFactory for ActualCpuFactory {
            fn request_adapters(&self) -> Result<Vec<GpuAdapterCandidate>, ScaffoldContractError> {
                Ok(vec![GpuAdapterCandidate::Hardware(self.0.clone())])
            }
            fn request_device(
                &self,
                _: &wgpu::Adapter,
                _: &wgpu::DeviceDescriptor<'_>,
            ) -> Result<(wgpu::Device, wgpu::Queue), ScaffoldContractError> {
                panic!("normal acquisition must reject the actual CPU before requesting a device")
            }
        }
        assert!(matches!(
            acquire_required_gpu(&ActualCpuFactory(backend.adapter.clone())),
            Err(ScaffoldContractError::NeuralBackendUnavailable)
        ));
        backend.device_lost.store(true, Ordering::Release);
        assert!(backend.offline_training_device_queue().is_err());
    }
}
