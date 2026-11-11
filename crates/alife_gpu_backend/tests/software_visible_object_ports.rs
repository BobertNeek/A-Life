//! Offline CPU/Vulkan WGSL parity, never physical-GPU qualification.
#![cfg(all(feature = "gpu-tests", feature = "training-rollout"))]

mod support;

use alife_gpu_backend::GpuClosedLoopBackend;

#[test]
#[ignore = "explicit offline software-Vulkan parity fixture; not hardware qualification"]
fn visible_object_ports_match_cpu_encoding_on_software_vulkan() {
    // Missing CPU Vulkan is a failure, never a skip or hardware fallback.
    let profile = support::scaling::bounded_profile(128 * 1024 * 1024, 128 * 1024 * 1024, 8, 2);
    let mut backend = GpuClosedLoopBackend::new_software_vulkan_runtime_test(profile)
        .expect("explicit offline CPU Vulkan runtime fixture");
    assert!(backend.is_software_runtime_test());
    assert!(!backend.is_software_training_test());
    let info = backend.software_runtime_test_adapter_info().unwrap();
    assert_eq!(info.device_type, wgpu::DeviceType::Cpu);
    assert_eq!(info.backend, wgpu::Backend::Vulkan);
    assert_eq!(backend.hardware_receipt().backend_api, "vulkan");
    assert_eq!(backend.hardware_receipt().vendor_id, info.vendor);
    assert_eq!(backend.hardware_receipt().device_id, info.device);
    eprintln!("visible_object_ports_offline_software_adapter={info:?}");

    support::visible_object_parity::assert_visible_object_ports_match_cpu_encoding(&mut backend);
    assert_eq!(backend.admission_receipt().live_brains, 0);
}
