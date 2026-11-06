use super::*;
use std::{
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

#[derive(Debug, Clone)]
struct NativeError(String, Option<Box<NativeError>>);
impl fmt::Display for NativeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
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
        Some(Box::new(NativeError(
            "HRESULT 0x8007000E private driver text".into(),
            None,
        ))),
    )
}
fn context(backend: wgpu::Backend) -> AdapterContext {
    AdapterContext::from_info(&wgpu::AdapterInfo {
        name: "private@example.invalid secret-name 192.168.1.1".into(),
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
    })
}
fn record(error: impl Error, label: &str) -> DiagnosticRecord {
    let mut records = Vec::new();
    let result = request_device_result::<()>(
        Err(error),
        || context(wgpu::Backend::Dx12),
        label,
        |record| records.push(record),
    );
    assert_eq!(result, Err(ScaffoldContractError::NeuralBackendUnavailable));
    assert_eq!(records.len(), 1);
    records[0]
}

#[test]
fn request_failure_preserves_the_legacy_error_and_structured_context() {
    for backend in [
        wgpu::Backend::Vulkan,
        wgpu::Backend::Dx12,
        wgpu::Backend::Metal,
    ] {
        let error = native_error();
        let legacy: Result<(), _> =
            Err(error.clone()).map_err(|_| ScaffoldContractError::NeuralBackendUnavailable);
        let mut records = Vec::new();
        let result = request_device_result::<()>(
            Err(error),
            || context(backend),
            "alife-required-closed-loop-device",
            |record| records.push(record),
        );
        assert_eq!(result, legacy);
        assert_eq!(
            records,
            [DiagnosticRecord {
                context: context(backend),
                event: Event::Request {
                    role: DeviceRole::RequiredClosedLoop,
                    detail: RequestDetail::Other,
                }
            }]
        );
        assert_eq!(records[0].line(), format!("gpu_device_failure backend={backend:?} vendor=0x10de device=0x1234 stage=request_device role=required_closed_loop kind=other"));
    }
}

#[test]
fn request_success_preserves_resource_and_avoids_metadata_and_reporting() {
    let resource = Box::new(17_u32);
    let pointer = std::ptr::from_ref(resource.as_ref());
    let actual = request_device_result(
        Ok::<_, NativeError>(resource),
        || panic!("success must not query context"),
        "private label",
        |_| panic!("success must not report"),
    )
    .unwrap();
    assert_eq!(std::ptr::from_ref(actual.as_ref()), pointer);
}

#[test]
fn callback_sets_fail_stop_before_emission_and_destruction_stays_quiet() {
    let lost = AtomicBool::new(false);
    let mut records = Vec::new();
    device_lost(
        &lost,
        context(wgpu::Backend::Dx12),
        wgpu::DeviceLostReason::Unknown,
        |record| {
            assert!(lost.load(Ordering::Acquire));
            records.push(record);
        },
    );
    assert_eq!(records[0].line(), "gpu_device_failure backend=Dx12 vendor=0x10de device=0x1234 stage=device_lost reason=Unknown");
    let destroyed = AtomicBool::new(false);
    device_lost(
        &destroyed,
        context(wgpu::Backend::Vulkan),
        wgpu::DeviceLostReason::Destroyed,
        |_| panic!("normal destruction stays quiet"),
    );
    assert!(destroyed.load(Ordering::Acquire));
}

#[test]
fn arbitrary_text_and_truncated_prefixes_never_reach_output() {
    let texts = [
        "personal@example.invalid".into(),
        "/home/person/private.bin".into(),
        "C:\\Users\\private person\\file".into(),
        "private-filename.brain".into(),
        "192.168.0.1".into(),
        "secret token content".into(),
        "HRESULT 0x8007000E".into(),
        "🦀".repeat(400),
        format!("{}@private.invalid", "personal".repeat(40)),
        format!("{}/private/file", "prefix".repeat(50)),
        format!(
            "Limit 'max_bind_groups' value 9 is better than allowed 8{}@private",
            " ".repeat(256)
        ),
        "Limit 'max_bind_groups' value 9 is better than allowed 8\nsecret".into(),
    ];
    for text in texts {
        let result = record(NativeError(text, None), "private/path label");
        assert_eq!(result.line(), "gpu_device_failure backend=Dx12 vendor=0x10de device=0x1234 stage=request_device role=other kind=other");
    }
    assert_eq!(
        DeviceRole::from_label("alife-explicit-software-offline-training-test"),
        DeviceRole::OfflineSoftwareTest
    );
    assert_eq!(
        DeviceRole::from_label("alife-required-closed-loop-device secret"),
        DeviceRole::Other
    );
}

#[test]
fn classifier_accepts_only_exact_known_messages_and_validated_numeric_limits() {
    for (text, expected) in [
        ("Not enough memory left.", RequestDetail::OutOfMemory),
        ("Parent device is lost", RequestDetail::DeviceLost),
        (
            "Failed to initialize Timestamp Normalizer",
            RequestDetail::TimestampNormalizer,
        ),
    ] {
        assert_eq!(classify_request(&NativeError(text.into(), None)), expected);
    }
    for name in [
        "max_bind_groups",
        "max_storage_buffers_per_shader_stage",
        "max_buffer_size",
        "max_storage_buffer_binding_size",
    ] {
        assert_eq!(
            classify_request(&NativeError(
                format!("Limit '{name}' value 9 is better than allowed 8"),
                None
            )),
            RequestDetail::Limit {
                name,
                requested: 9,
                allowed: 8
            }
        );
    }
    for (name, requested, allowed) in [
        ("private-file.bin", "9", "8"),
        ("max_bind_groups", "09", "8"),
        ("max_bind_groups", "+9", "8"),
        ("max_bind_groups", "9", "-1"),
        ("max_bind_groups", "18446744073709551616", "8"),
        ("max_bind_groups", "9", "8 private"),
        ("max_bind_groups", "8", "9"),
        ("max_bind_groups", "8", "8"),
        ("max_bind_groups", "1.2.3.4", "0"),
    ] {
        assert_eq!(
            classify_request(&NativeError(
                format!("Limit '{name}' value {requested} is better than allowed {allowed}"),
                None
            )),
            RequestDetail::Other
        );
    }
    struct IgnoresErrors;
    impl fmt::Display for IgnoresErrors {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let _ = f.write_str(&"x".repeat(MAX_MESSAGE_BYTES + 1));
            let _ = f.write_str("Not enough memory left.");
            Ok(())
        }
    }
    impl fmt::Debug for IgnoresErrors {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("fixture")
        }
    }
    impl Error for IgnoresErrors {}
    assert_eq!(classify_request(&IgnoresErrors), RequestDetail::Other);
}

#[test]
fn records_have_fixed_storage_and_bounded_structured_output() {
    assert!(std::mem::size_of::<DiagnosticRecord>() <= 128);
    let record = DiagnosticRecord {
        context: AdapterContext {
            backend: wgpu::Backend::BrowserWebGpu,
            vendor: u32::MAX,
            device: u32::MAX,
        },
        event: Event::Request {
            role: DeviceRole::OfflineSoftwareTest,
            detail: RequestDetail::Limit {
                name: "max_storage_buffers_per_shader_stage",
                requested: u64::MAX,
                allowed: u64::MAX - 1,
            },
        },
    };
    let line = record.line();
    assert!(line.len() <= 512);
    assert!(!line.chars().any(char::is_control));
}

#[test]
fn sink_drops_on_unprepared_full_failed_or_disconnected_worker() {
    let sink = DiagnosticSink::default();
    let first = record(native_error(), "alife-required-closed-loop-device");
    let second = record(native_error(), "other");
    sink.submit(first);
    let mut receiver = None;
    sink.prepare_with(|rx| {
        receiver = Some(rx);
        Ok(())
    });
    let receiver = receiver.unwrap();
    assert!(receiver.try_recv().is_err());
    sink.submit(first);
    sink.submit(second);
    assert_eq!(receiver.try_recv().unwrap(), first);
    assert!(receiver.try_recv().is_err());
    sink.submit(second);
    assert_eq!(receiver.try_recv().unwrap(), second);
    drop(receiver);
    sink.submit(first);
    sink.prepare_with(|_| panic!("worker must never be restarted"));
    let failed = DiagnosticSink::default();
    failed.prepare_with(|_| Err(io::Error::other("fixture spawn failure")));
    failed.prepare_with(|_| panic!("failed worker must not be retried"));
    assert_eq!(
        request_device_result::<()>(
            Err(native_error()),
            || context(wgpu::Backend::Metal),
            "other",
            |record| failed.submit(record)
        ),
        Err(ScaffoldContractError::NeuralBackendUnavailable)
    );
    let lost = AtomicBool::new(false);
    device_lost(
        &lost,
        context(wgpu::Backend::Metal),
        wgpu::DeviceLostReason::Unknown,
        |record| failed.submit(record),
    );
    assert!(lost.load(Ordering::Acquire));
}

#[test]
fn callback_does_not_wait_for_worker_initialization() {
    let sink = Arc::new(DiagnosticSink::default());
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let initializing = Arc::clone(&sink);
    let initializer = std::thread::spawn(move || {
        initializing.prepare_with(|_| {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            Err(io::Error::other("fixture"))
        })
    });
    started_rx
        .recv_timeout(std::time::Duration::from_secs(2))
        .unwrap();
    let submitting = Arc::clone(&sink);
    let (done_tx, done_rx) = mpsc::channel();
    let producer = std::thread::spawn(move || {
        let lost = AtomicBool::new(false);
        device_lost(
            &lost,
            context(wgpu::Backend::Metal),
            wgpu::DeviceLostReason::Unknown,
            |record| submitting.submit(record),
        );
        done_tx.send(lost.load(Ordering::Acquire)).unwrap();
    });
    let done = done_rx.recv_timeout(std::time::Duration::from_secs(2));
    release_tx.send(()).unwrap();
    initializer.join().unwrap();
    producer.join().unwrap();
    assert!(done.unwrap());
}

#[test]
fn diagnostic_sink_closed_pipe_does_not_change_exit_status() {
    use std::{
        io::{Read, Write},
        process::{Command, Stdio},
    };
    const CHILD: &str = "ALIFE_STARTUP_DIAGNOSTICS_CLOSED_STDERR_CHILD";
    if let Some(mode) = std::env::var_os(CHILD) {
        let mode = mode.to_str().unwrap();
        prepare();
        let mut ready = [0];
        std::io::stdin().read_exact(&mut ready).unwrap();
        let count = if mode == "burst" { 128 } else { 1 };
        if mode != "loss" {
            for _ in 0..count {
                assert_eq!(
                    request_device_result::<()>(
                        Err(native_error()),
                        || context(wgpu::Backend::Dx12),
                        "other",
                        emit
                    ),
                    Err(ScaffoldContractError::NeuralBackendUnavailable)
                );
            }
            println!("REQUEST_RETURNED");
        }
        if mode != "request" {
            let lock = std::sync::Mutex::new(());
            let guard = lock.lock().unwrap();
            let lost = AtomicBool::new(false);
            for _ in 0..count {
                device_lost(
                    &lost,
                    context(wgpu::Backend::Dx12),
                    wgpu::DeviceLostReason::Unknown,
                    emit,
                );
            }
            assert!(lost.load(Ordering::Acquire));
            drop(guard);
            assert!(lock.try_lock().is_ok());
            println!("LOSS_FLAG_SET_CALLBACK_LOCK_RELEASED");
        }
        println!("NORMAL_RESULT");
        std::io::stdout().flush().unwrap();
        if mode == "capture" {
            // Keep the child alive until the parent has read a real worker
            // record. Shutdown intentionally does not wait for diagnostics.
            std::io::stdin().read_exact(&mut ready).unwrap();
        }
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
        .expect("explicit fixture requires CPU Vulkan");
    let mut limits = wgpu::Limits::downlevel_defaults();
    limits.max_bind_groups = adapter.limits().max_bind_groups.checked_add(1).unwrap();
    let descriptor = wgpu::DeviceDescriptor {
        label: Some("private fixture label"),
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
    let mut records = Vec::new();
    assert_eq!(
        request_device_result::<()>(
            Err(error),
            || AdapterContext::from_info(&adapter.get_info()),
            descriptor.label.unwrap(),
            |record| records.push(record),
        ),
        Err(ScaffoldContractError::NeuralBackendUnavailable)
    );
    assert_eq!(records.len(), 1);
    let record = records[0];
    assert_eq!(record.context.backend, wgpu::Backend::Vulkan);
    assert_eq!(
        record.event,
        Event::Request {
            role: DeviceRole::Other,
            detail: RequestDetail::Limit {
                name: "max_bind_groups",
                requested: u64::from(adapter.limits().max_bind_groups) + 1,
                allowed: u64::from(adapter.limits().max_bind_groups),
            }
        }
    );
    eprintln!("software_vulkan_verified_limit={}", record.line());
}
