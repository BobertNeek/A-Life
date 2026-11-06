use std::{
    io,
    sync::atomic::{AtomicBool, Ordering},
};

use super::*;

#[derive(Debug, Clone)]
struct NativeError(String, Option<Box<NativeError>>);

impl fmt::Display for NativeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for NativeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.1.as_deref().map(|error| error as &dyn Error)
    }
}

fn native_error() -> NativeError {
    NativeError(
        "device initialization failed".into(),
        Some(Box::new(NativeError("HRESULT 0x8007000E".into(), None))),
    )
}

fn info(backend: wgpu::Backend, name: &str) -> wgpu::AdapterInfo {
    wgpu::AdapterInfo {
        name: name.into(),
        backend,
        vendor: 0x10de,
        device: 0x1234,
        device_type: wgpu::DeviceType::DiscreteGpu,
        device_pci_bus_id: "private pci".into(),
        driver: "private driver".into(),
        driver_info: "private driver info".into(),
        subgroup_min_size: 32,
        subgroup_max_size: 32,
        transient_saves_memory: false,
    }
}

#[test]
fn request_failure_keeps_the_legacy_error_and_reports_native_context() {
    for backend in [
        wgpu::Backend::Vulkan,
        wgpu::Backend::Dx12,
        wgpu::Backend::Metal,
    ] {
        let error = native_error();
        let legacy: Result<(), _> =
            Err(error.clone()).map_err(|_| ScaffoldContractError::NeuralBackendUnavailable);
        let mut records = Vec::new();
        let actual = request_device_result::<()>(
            Err(error),
            || info(backend, "Fixture GPU"),
            "actual requested device",
            |record| records.push(record.to_owned()),
        );
        assert_eq!(actual, legacy);
        assert_eq!(records.len(), 1);
        for expected in [
            format!("backend={backend:?}"),
            "adapter=\"Fixture GPU\"".into(),
            "vendor=0x10de".into(),
            "device=0x1234".into(),
            "stage=\"request_device\"".into(),
            "label=\"actual requested device\"".into(),
            "device initialization failed".into(),
            "0x8007000E".into(),
        ] {
            assert!(
                records[0].contains(&expected),
                "missing {expected}: {}",
                records[0]
            );
        }
        assert!(!records[0].contains("private"));
    }
}

#[test]
fn request_success_preserves_the_resource_without_reading_info_or_reporting() {
    let resource = Box::new(17_u32);
    let address = std::ptr::from_ref(resource.as_ref());
    let result = request_device_result::<Box<u32>>(
        Ok::<_, NativeError>(resource),
        || panic!("success must not read or format adapter metadata"),
        "actual requested device",
        |_| panic!("success must remain silent"),
    )
    .unwrap();
    assert_eq!(std::ptr::from_ref(result.as_ref()), address);
    assert_eq!(*result, 17);
}

#[test]
fn device_loss_retains_fail_stop_before_reporting_and_destruction_is_quiet() {
    let lost = AtomicBool::new(false);
    let mut records = Vec::new();
    device_lost(
        &lost,
        &info(wgpu::Backend::Dx12, "Fixture GPU"),
        wgpu::DeviceLostReason::Unknown,
        "native device loss 0x8007000E\nprivate raw shader excerpt",
        |record| {
            assert!(lost.load(Ordering::Acquire));
            records.push(record.to_owned());
        },
    );
    assert_eq!(records.len(), 1);
    for expected in ["device_lost_callback", "reason=Unknown", "0x8007000E"] {
        assert!(records[0].contains(expected));
    }
    assert!(!records[0].contains("private"));
    assert!(!records[0].contains("compute_pipeline"));

    let destroyed = AtomicBool::new(false);
    device_lost(
        &destroyed,
        &info(wgpu::Backend::Vulkan, "Fixture GPU"),
        wgpu::DeviceLostReason::Destroyed,
        "normal destruction",
        |_| panic!("normal destruction must remain silent"),
    );
    assert!(destroyed.load(Ordering::Acquire));
}

#[test]
fn records_bound_causes_utf8_and_controls_and_redact_entire_path_fields() {
    let mut short_chain = NativeError("fifth cause must be omitted".into(), None);
    for index in (1..=MAX_CAUSES).rev() {
        short_chain = NativeError(format!("cause {index}"), Some(Box::new(short_chain)));
    }
    let short_record = failure_record(
        &info(wgpu::Backend::Vulkan, "Fixture GPU"),
        "request_device",
        "label",
        &short_chain,
    );
    for index in 1..=MAX_CAUSES {
        assert!(short_record.contains(&format!("cause {index}")));
    }
    assert!(!short_record.contains("fifth cause"));
    assert!(!short_record.contains("truncated"));

    let mut error = NativeError("fifth cause must be omitted".into(), None);
    for _ in 0..MAX_CAUSES {
        error = NativeError("\u{7f}".repeat(400), Some(Box::new(error)));
    }
    let record = failure_record(
        &info(wgpu::Backend::Vulkan, &"\u{7f}".repeat(400)),
        "request_device",
        "label",
        &error,
    );
    assert!(record.len() <= MAX_RECORD_BYTES);
    assert!(record.ends_with("...[truncated]"));
    assert!(!record.contains("fifth cause"));
    assert!(!record.chars().any(char::is_control));
    assert_eq!(
        safe_field("🦀".repeat(400))
            .chars()
            .filter(|c| *c == '🦀')
            .count(),
        64
    );
    for text in [
        "compiler C:\\Users\\private person\\file failed",
        "compiler /private/user name with spaces/file failed",
        "email user@example.invalid failed",
        "download https://example.invalid/private failed",
    ] {
        assert_eq!(safe_field(text), "\"[redacted]\"");
    }
    assert_eq!(safe_field("first line\nprivate shader"), "\"first line\"");
    assert_eq!(safe_field("first line\rprivate shader"), "\"first line\"");
    assert_eq!(
        safe_field("tab\tand\u{1b}control"),
        "\"tab\\tand\\u{1b}control\""
    );
}

#[test]
fn first_line_limit_stays_closed_if_display_ignores_format_errors() {
    struct IgnoresErrors;
    impl fmt::Display for IgnoresErrors {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            let _ = formatter.write_str("first\nprivate shader");
            let _ = formatter.write_str("more private shader");
            Ok(())
        }
    }
    assert_eq!(safe_field(IgnoresErrors), "\"first\"");
}

#[test]
fn diagnostic_write_failures_do_not_replace_typed_errors_or_fail_stop() {
    struct Broken;
    impl io::Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed sink"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let result = request_device_result::<()>(
        Err(native_error()),
        || info(wgpu::Backend::Dx12, "Fixture GPU"),
        "actual requested device",
        |record| emit_to(Broken, record),
    );
    assert_eq!(result, Err(ScaffoldContractError::NeuralBackendUnavailable));
    let lost = AtomicBool::new(false);
    device_lost(
        &lost,
        &info(wgpu::Backend::Dx12, "Fixture GPU"),
        wgpu::DeviceLostReason::Unknown,
        "lost",
        |record| emit_to(Broken, record),
    );
    assert!(lost.load(Ordering::Acquire));
}

#[test]
fn diagnostic_sink_closed_pipe_does_not_change_exit_status() {
    use std::{
        io::{Read, Write},
        process::{Command, Stdio},
    };
    const CHILD: &str = "ALIFE_STARTUP_DIAGNOSTICS_CLOSED_STDERR_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let mut ready = [0];
        std::io::stdin().read_exact(&mut ready).unwrap();
        let result = request_device_result::<()>(
            Err(native_error()),
            || info(wgpu::Backend::Dx12, "Fixture GPU"),
            "actual requested device",
            emit,
        );
        assert_eq!(result, Err(ScaffoldContractError::NeuralBackendUnavailable));
        let lost = AtomicBool::new(false);
        device_lost(
            &lost,
            &info(wgpu::Backend::Dx12, "Fixture GPU"),
            wgpu::DeviceLostReason::Unknown,
            "lost",
            emit,
        );
        assert!(lost.load(Ordering::Acquire));
        println!("NORMAL_RESULT");
        return;
    }
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "startup_diagnostics::tests::diagnostic_sink_closed_pipe_does_not_change_exit_status",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // Close the reader before releasing the child; the writes must see a
    // genuinely broken pipe, including during process teardown.
    drop(child.stderr.take().unwrap());
    child.stdin.take().unwrap().write_all(&[1]).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "child exit: {:?}", output.status);
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .contains("NORMAL_RESULT"));
}

#[cfg(feature = "gpu-tests")]
#[test]
#[ignore = "explicit CPU Vulkan request-failure fixture; not GPU hardware qualification"]
fn software_vulkan_request_failure_retains_the_actual_native_cause() {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = wgpu::Backends::VULKAN;
    let instance = wgpu::Instance::new(descriptor);
    let adapter = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::VULKAN))
        .into_iter()
        .find(|a| a.get_info().device_type == wgpu::DeviceType::Cpu)
        .expect("this explicit fixture requires a CPU Vulkan adapter");
    let mut limits = wgpu::Limits::downlevel_defaults();
    limits.max_bind_groups = adapter.limits().max_bind_groups.checked_add(1).unwrap();
    let descriptor = wgpu::DeviceDescriptor {
        label: Some("alife-diagnostic-invalid-limit-fixture"),
        required_limits: limits,
        ..Default::default()
    };
    let legacy = pollster::block_on(adapter.request_device(&descriptor))
        .map_err(|_| ScaffoldContractError::NeuralBackendUnavailable);
    assert!(matches!(
        legacy,
        Err(ScaffoldContractError::NeuralBackendUnavailable)
    ));
    let error = pollster::block_on(adapter.request_device(&descriptor)).unwrap_err();
    assert!(error.to_string().contains("max_bind_groups"));
    let mut records = Vec::new();
    let result = request_device_result::<()>(
        Err(error),
        || adapter.get_info(),
        descriptor.label.unwrap(),
        |record| records.push(record.to_owned()),
    );
    assert_eq!(result, Err(ScaffoldContractError::NeuralBackendUnavailable));
    assert_eq!(records.len(), 1);
    assert!(records[0].contains("backend=Vulkan"));
    assert!(records[0].contains("request_device"));
    assert!(records[0].contains("alife-diagnostic-invalid-limit-fixture"));
    assert!(records[0].contains("max_bind_groups"));
    eprintln!("software_vulkan_request_failure={}", records[0]);
}
