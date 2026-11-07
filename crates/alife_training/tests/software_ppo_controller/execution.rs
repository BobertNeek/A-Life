//! Explicit fixture selection; ordinary hardware admission is never changed.
use super::*;
use alife_gpu_backend::{GpuClosedLoopBackend, GpuRuntimeProfile};
use std::ffi::OsStr;

const SELECTOR: &str = "ALIFE_DISPATCH_REQUIRE_HARDWARE";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExecutionMode {
    SoftwareVulkan,
    RequiredHardware,
}

impl ExecutionMode {
    fn parse(value: Option<&OsStr>) -> Result<Self, &'static str> {
        match value {
            None => Ok(Self::SoftwareVulkan),
            Some(value) => match value.to_str() {
                Some("0") => Ok(Self::SoftwareVulkan),
                Some("1") => Ok(Self::RequiredHardware),
                _ => Err("ALIFE_DISPATCH_REQUIRE_HARDWARE must be unset, 0, or 1"),
            },
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::SoftwareVulkan => "software_vulkan_test",
            Self::RequiredHardware => "required_hardware",
        }
    }
}

pub(super) fn new_session() -> Result<GpuAuthoritativeSession, ScaffoldContractError> {
    // Parse before either constructor can enumerate/request a device. No
    // permissive truthiness, trimming, invalid-Unicode default or fallback.
    let value = std::env::var_os(SELECTOR);
    let mode = ExecutionMode::parse(value.as_deref()).unwrap_or_else(|message| panic!("{message}"));
    let session = match mode {
        ExecutionMode::SoftwareVulkan => {
            GpuAuthoritativeSession::new_software_vulkan_training_test()?
        }
        ExecutionMode::RequiredHardware => GpuAuthoritativeSession::new(
            GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())?,
            alife_runtime::GpuSessionConsumerKind::Training,
        ),
    };
    let backend = session.backend();
    let info = backend.adapter_info_for_test();
    match mode {
        ExecutionMode::SoftwareVulkan => {
            assert!(backend.is_software_training_test());
            assert_eq!(info.device_type, wgpu::DeviceType::Cpu);
            assert_eq!(info.backend, wgpu::Backend::Vulkan);
        }
        ExecutionMode::RequiredHardware => {
            assert!(!backend.is_software_training_test());
            assert_ne!(info.device_type, wgpu::DeviceType::Cpu);
            assert_ne!(info.backend, wgpu::Backend::Noop);
        }
    }
    eprintln!(
        "actual_controller_execution={} adapter={info:?}",
        mode.label()
    );
    Ok(session)
}

pub(super) fn metadata(trainer: &FoundationTrainer) -> serde_json::Value {
    let backend = trainer.session().backend();
    let info = backend.adapter_info_for_test();
    let mode = if backend.is_software_training_test() {
        ExecutionMode::SoftwareVulkan
    } else {
        ExecutionMode::RequiredHardware
    };
    serde_json::json!({
        "execution_mode": mode.label(),
        "selector_variable": SELECTOR,
        "adapter_info": {
            "name": info.name, "device_type": format!("{:?}", info.device_type),
            "backend": format!("{:?}", info.backend), "vendor": info.vendor,
            "device": info.device, "driver": info.driver, "driver_info": info.driver_info,
        },
    })
}

#[test]
fn selector_accepts_only_unset_zero_or_one_without_environment_mutation() {
    assert_eq!(
        ExecutionMode::parse(None),
        Ok(ExecutionMode::SoftwareVulkan)
    );
    assert_eq!(
        ExecutionMode::parse(Some(OsStr::new("0"))),
        Ok(ExecutionMode::SoftwareVulkan)
    );
    assert_eq!(
        ExecutionMode::parse(Some(OsStr::new("1"))),
        Ok(ExecutionMode::RequiredHardware)
    );
    for value in ["", "2", "01", "-1", "true", "false", " 1", "1\n"] {
        assert!(ExecutionMode::parse(Some(OsStr::new(value))).is_err());
    }
}

#[test]
#[cfg(unix)]
fn selector_rejects_non_unicode_without_environment_mutation() {
    use std::os::unix::ffi::OsStrExt;
    assert!(ExecutionMode::parse(Some(OsStr::from_bytes(&[0xff]))).is_err());
}
