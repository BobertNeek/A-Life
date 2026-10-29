use std::{cell::RefCell, rc::Rc};

use super::*;

#[derive(Debug)]
struct NativeError(&'static str, Option<Box<NativeError>>);

impl fmt::Display for NativeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl Error for NativeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.1.as_deref().map(|error| error as &dyn Error)
    }
}

fn native_error() -> NativeError {
    NativeError(
        "device initialization failed",
        Some(Box::new(NativeError(
            "Out of memory",
            Some(Box::new(NativeError("HRESULT 0x8007000E", None))),
        ))),
    )
}

fn info(backend: wgpu::Backend, name: &str) -> wgpu::AdapterInfo {
    wgpu::AdapterInfo {
        name: name.into(),
        backend,
        vendor: 0x10de,
        device: 0x1234,
        device_type: wgpu::DeviceType::DiscreteGpu,
        device_pci_bus_id: "not-reported".into(),
        driver: "not-reported".into(),
        driver_info: "not-reported".into(),
        subgroup_min_size: 32,
        subgroup_max_size: 32,
        transient_saves_memory: false,
    }
}

#[test]
fn request_failure_retains_backend_adapter_stage_and_native_chain() {
    let mut records = Vec::new();
    let result = request_device_result::<()>(
        // Mirrors request-device -> timestamp-normalizer -> compute-pipeline
        // -> native compiler cause without creating a hardware device.
        Err(NativeError(
            "timestamp normalizer initialization failed",
            Some(Box::new(native_error())),
        )),
        &info(wgpu::Backend::Dx12, "Fixture GPU"),
        |record| records.push(record.to_owned()),
    );
    assert_eq!(result, Err(ScaffoldContractError::NeuralBackendUnavailable));
    let record = &records[0];
    for value in [
        "backend=Dx12",
        "adapter=\"Fixture GPU\"",
        "vendor=0x10de",
        "device=0x1234",
        "stage=\"request_device\"",
        "alife-required-closed-loop-device",
        "device initialization failed",
        "Out of memory",
        "0x8007000E",
    ] {
        assert!(record.contains(value), "missing {value}: {record}");
    }
    assert!(!record.contains("not-reported"));
    assert_eq!(records.len(), 1);
}

fn scripted_selection(success: Option<usize>) -> (Option<usize>, Vec<usize>, Vec<String>) {
    let records = RefCell::new(Vec::new());
    let mut visited = Vec::new();
    let adapters = [
        info(wgpu::Backend::Vulkan, "first"),
        info(wgpu::Backend::Dx12, "second"),
        info(wgpu::Backend::Gl, "third"),
    ];
    let selected = select_adapter(
        adapters.into_iter().enumerate(),
        |(index, info)| {
            visited.push(index);
            let result = if success == Some(index) {
                Ok(index)
            } else {
                Err(native_error())
            };
            match request_device_result(result, &info, |record| {
                records.borrow_mut().push(record.to_owned())
            }) {
                Ok(device) => Ok(AdapterAttempt::Ready(device, info)),
                Err(_) => Ok(AdapterAttempt::RequestFailed),
            }
        },
        |record| records.borrow_mut().push(record.to_owned()),
    )
    .unwrap();
    (selected, visited, records.into_inner())
}

#[test]
fn ordered_fallback_preserves_selection_and_distinguishes_recovery_from_fatal_failure() {
    let (selected, visited, records) = scripted_selection(Some(1));
    assert_eq!(selected, Some(1));
    assert_eq!(visited, [0, 1]);
    assert_eq!(records.len(), 2);
    assert!(records[0].contains("backend=Vulkan"));
    assert!(records[0].contains("0x8007000E"));
    assert!(records[1].contains("outcome=fallback_selected backend=Dx12"));
    assert!(records[1].contains("failed_requests=1"));
    assert!(records
        .iter()
        .all(|record| !record.contains("outcome=unavailable")));

    let (selected, visited, records) = scripted_selection(None);
    assert_eq!(selected, None);
    assert_eq!(visited, [0, 1, 2]);
    assert_eq!(records.len(), 4);
    for (record, backend) in records[..3].iter().zip(["Vulkan", "Dx12", "Gl"]) {
        assert!(record.contains(backend));
        assert!(record.contains("0x8007000E"));
    }
    assert!(records[3].contains("outcome=unavailable"));
    assert!(records[3].contains("failed_requests=3"));

    // A post-request receipt/contract error was always terminal, unlike a
    // request-device failure. Never continue to the next adapter on that path.
    let mut visited = Vec::new();
    let result: Result<Option<usize>, _> = select_adapter(
        [0, 1, 2],
        |candidate| {
            visited.push(candidate);
            match candidate {
                0 => Ok(AdapterAttempt::RequestFailed),
                1 => Err(ScaffoldContractError::GpuLayoutMismatch),
                _ => panic!("terminal contract failure must stop adapter iteration"),
            }
        },
        |_| panic!("terminal attempt error must not report fallback selection"),
    );
    assert_eq!(result, Err(ScaffoldContractError::GpuLayoutMismatch));
    assert_eq!(visited, [0, 1]);
}

#[test]
fn first_adapter_success_is_silent_and_skipped_candidates_are_not_failed_requests() {
    let (selected, visited, records) = scripted_selection(Some(0));
    assert_eq!(selected, Some(0));
    assert_eq!(visited, [0]);
    assert!(records.is_empty());

    let mut records = Vec::new();
    let selected = select_adapter(
        [false, true],
        |usable| {
            Ok(if usable {
                AdapterAttempt::Ready(7, info(wgpu::Backend::Vulkan, "usable"))
            } else {
                AdapterAttempt::Skipped
            })
        },
        |record| records.push(record.to_owned()),
    )
    .unwrap();
    assert_eq!(selected, Some(7));
    assert!(records.is_empty());
}

#[test]
fn scoped_pipeline_failures_propagate_existing_types_and_keep_oom_native_cause() {
    for (error, expected) in [
        (
            wgpu::Error::Validation {
                source: Box::new(native_error()),
                description: "pipeline validation failed\nraw shader must not be logged".into(),
            },
            GpuClosedLoopError::LayoutMismatch,
        ),
        (
            wgpu::Error::Internal {
                source: Box::new(native_error()),
                description: "native compiler failed".into(),
            },
            GpuClosedLoopError::SubmissionFailed,
        ),
        (
            wgpu::Error::OutOfMemory {
                source: Box::new(native_error()),
            },
            GpuClosedLoopError::SubmissionFailed,
        ),
    ] {
        let mut records = Vec::new();
        let result = scoped_result(
            17,
            &info(wgpu::Backend::Dx12, "Fixture GPU"),
            "compute_pipeline",
            "initialize_sleep_transaction",
            [Some(error), None, None],
            |record| records.push(record.to_owned()),
        );
        assert_eq!(result, Err(expected));
        assert_eq!(records.len(), 1);
        let record = &records[0];
        for value in [
            "backend=Dx12",
            "Fixture GPU",
            "compute_pipeline",
            "initialize_sleep_transaction",
            "0x8007000E",
        ] {
            assert!(record.contains(value), "missing {value}: {record}");
        }
        assert!(!record.contains("raw shader"));
    }
}

#[test]
fn scoped_shader_failure_drops_failed_resource_and_reports_every_scope() {
    struct Resource(Rc<RefCell<u32>>);
    impl Drop for Resource {
        fn drop(&mut self) {
            *self.0.borrow_mut() += 1;
        }
    }
    let drops = Rc::new(RefCell::new(0));
    let mut records = Vec::new();
    let result = scoped_result(
        Resource(Rc::clone(&drops)),
        &info(wgpu::Backend::Vulkan, "GPU"),
        "shader_module",
        "closed-loop-consolidate-wgsl",
        [
            Some(wgpu::Error::Validation {
                source: Box::new(native_error()),
                description: "shader validation failed".into(),
            }),
            Some(wgpu::Error::Internal {
                source: Box::new(native_error()),
                description: "shader compiler failed".into(),
            }),
            Some(wgpu::Error::OutOfMemory {
                source: Box::new(native_error()),
            }),
        ],
        |record| records.push(record.to_owned()),
    );
    assert!(matches!(result, Err(GpuClosedLoopError::SubmissionFailed)));
    assert_eq!(*drops.borrow(), 1);
    assert_eq!(records.len(), 3);
    assert!(records.iter().all(|record| record.contains("shader_module")
        && record.contains("closed-loop-consolidate-wgsl")));

    let result = scoped_result(
        37,
        &info(wgpu::Backend::Vulkan, "GPU"),
        "compute_pipeline",
        "encode_perception",
        [None, None, None],
        |_| panic!("successful initialization must be silent"),
    );
    assert_eq!(result, Ok(37));
}

#[test]
fn diagnostics_bound_utf8_escape_controls_and_omit_paths_and_shader_dumps() {
    let adapter = info(
        wgpu::Backend::Vulkan,
        &format!("GPU\u{1b} {}", "🦀".repeat(400)),
    );
    let error = NativeError("compiler failed C:\\Users\\private\\shader /private/compiler user@example.invalid https://example.invalid\nRAW_SHADER_CONTENT", Some(Box::new(NativeError("NVVM 3: HRESULT 0x8007000E", None))));
    let record = failure_record(
        &adapter,
        "compute_pipeline",
        "initialize_sleep_transaction",
        &error,
    );
    assert!(record.len() <= MAX_RECORD_BYTES);
    assert!(!record.chars().any(char::is_control));
    for omitted in [
        "Users",
        "/private",
        "user@example",
        "https:",
        "RAW_SHADER_CONTENT",
    ] {
        assert!(!record.contains(omitted), "leaked {omitted}: {record}");
    }
    assert!(record.contains("[redacted]"));
    assert!(record.contains("0x8007000E"));
}

#[test]
fn device_loss_marks_failure_before_reporting_and_normal_destruction_is_quiet() {
    use std::sync::atomic::{AtomicBool, Ordering};

    let lost = AtomicBool::new(false);
    let mut records = Vec::new();
    device_lost(
        &lost,
        &info(wgpu::Backend::Dx12, "Fixture GPU"),
        wgpu::DeviceLostReason::Unknown,
        "native device loss 0x8007000E\nprivate raw shader excerpt",
        |record| {
            assert!(
                lost.load(Ordering::Acquire),
                "failure flag precedes emission"
            );
            records.push(record.to_owned());
        },
    );
    assert_eq!(records.len(), 1);
    for value in [
        "backend=Dx12",
        "Fixture GPU",
        "stage=\"device_lost\"",
        "label=\"device_lost_callback\"",
        "reason=Unknown",
        "0x8007000E",
    ] {
        assert!(
            records[0].contains(value),
            "missing {value}: {}",
            records[0]
        );
    }
    assert!(!records[0].contains("raw shader"));
    assert!(!records[0].contains("compute_pipeline"));

    let destroyed = AtomicBool::new(false);
    device_lost(
        &destroyed,
        &info(wgpu::Backend::Dx12, "Fixture GPU"),
        wgpu::DeviceLostReason::Destroyed,
        "normal destruction",
        |_| panic!("normal device destruction must remain quiet"),
    );
    assert!(destroyed.load(Ordering::Acquire));
}
