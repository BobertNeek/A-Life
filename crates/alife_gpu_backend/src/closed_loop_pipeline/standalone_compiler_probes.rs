//! Test-only compiler reproducers. Never called by production cognition.
use super::kernel_diagnostics::{check, scopes};
use super::*;
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, Once,
    },
    time::Duration,
};
use wgpu::util::DeviceExt;

const ABI: &str = include_str!("../../shaders/closed_loop_abi.wgsl");
const ENTRY: &str = "initialize_sleep_transaction";
const RAW: &str = "@group(0) @binding(4) var<storage,read> dispatch_header_words:array<u32>;\n@group(0) @binding(6) var<storage,read_write> mutable_state_words:array<atomic<u32>>;\n";
const WORDS: usize = 1024;
#[path = "native_stage_trace.rs"]
mod native_stage_trace;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Body {
    Empty,
    Barrier,
    ConditionalAtomic,
    ConvergedAtomic,
    ProductionInitializer,
}
#[derive(Clone, Copy, Debug)]
struct Probe {
    body: Body,
    abi: bool,
}
impl Probe {
    fn name(self) -> String {
        if self.body == Body::ProductionInitializer {
            return "production-initialize-sleep-transaction".into();
        }
        format!(
            "{}-{}",
            if self.abi { "abi" } else { "standalone" },
            match self.body {
                Body::Empty => "empty",
                Body::Barrier => "barrier",
                Body::ConditionalAtomic => "conditional-atomic",
                Body::ConvergedAtomic => "converged-atomic",
                Body::ProductionInitializer => unreachable!(),
            }
        )
    }
    fn source(self) -> String {
        if self.body == Body::ProductionInitializer {
            return runtime_shader_source(CLOSED_LOOP_CONSOLIDATE_WGSL).into_owned();
        }
        let prefix = if self.abi {
            runtime_shader_source(ABI).into_owned()
        } else if self.atomic() {
            RAW.to_owned()
        } else {
            String::new()
        };
        let body = match self.body {
            Body::ProductionInitializer => unreachable!(),
            Body::Empty => "",
            Body::Barrier => "storageBarrier();",
            Body::ConditionalAtomic => "if (dispatch_header_words[0] != 0u) { return; }\natomicStore(&mutable_state_words[0], dispatch_header_words[1]);\nstorageBarrier();\natomicStore(&mutable_state_words[3], 256u);",
            Body::ConvergedAtomic => "let prepared = dispatch_header_words[0] == 0u;\nif (prepared) { atomicStore(&mutable_state_words[0], dispatch_header_words[1]); }\nstorageBarrier();\nif (prepared) { atomicStore(&mutable_state_words[3], 256u); }",
        };
        format!("{prefix}\n@compute @workgroup_size(1) fn {ENTRY}() {{\n{body}\n}}\n")
    }
    fn atomic(self) -> bool {
        matches!(self.body, Body::ConditionalAtomic | Body::ConvergedAtomic)
    }
}
fn probes() -> impl Iterator<Item = Probe> {
    [false, true].into_iter().flat_map(|abi| {
        [
            Body::Empty,
            Body::Barrier,
            Body::ConditionalAtomic,
            Body::ConvergedAtomic,
        ]
        .into_iter()
        .map(move |body| Probe { body, abi })
    })
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn write_new(dir: &Path, name: &str, bytes: &[u8]) {
    use std::io::Write;
    let mut out = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.join(name))
        .unwrap();
    out.write_all(bytes).unwrap();
    eprintln!(
        "compiler_probe_artifact file={name} bytes={} sha256={}",
        bytes.len(),
        digest(bytes)
    );
}
fn module(source: &str) -> (naga::Module, naga::valid::ModuleInfo) {
    let module = naga::front::wgsl::parse_str(source).unwrap();
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
    (module, info)
}

fn offline_hlsl(
    module: &naga::Module,
    info: &naga::valid::ModuleInfo,
    shader_model: naga::back::hlsl::ShaderModel,
) -> (String, naga::back::hlsl::Options, String) {
    let options = naga::back::hlsl::Options {
        shader_model,
        fake_missing_bindings: false,
        binding_map: (0..7)
            .map(|binding| {
                (
                    naga::ResourceBinding { group: 0, binding },
                    naga::back::hlsl::BindTarget {
                        space: 0,
                        register: binding,
                        restrict_indexing: true,
                        ..Default::default()
                    },
                )
            })
            .collect(),
        ..Default::default()
    };
    let pipeline = naga::back::hlsl::PipelineOptions {
        entry_point: Some((naga::ShaderStage::Compute, ENTRY.into())),
    };
    let mut hlsl = String::new();
    let reflection = naga::back::hlsl::Writer::new(&mut hlsl, &options, &pipeline)
        .write(module, info, None)
        .unwrap();
    assert_eq!(reflection.entry_point_names.len(), 1);
    let translated_entry = reflection.entry_point_names[0].as_ref().unwrap().clone();
    (hlsl, options, translated_entry)
}

// These are fully specified offline emitter inputs, not a claim about an
// unknown adapter's HAL register allocation, shader model or SPIR-V options.
fn export(probe: Probe, dir: &Path) {
    let name = probe.name();
    let source = probe.source();
    write_new(dir, &format!("{name}.wgsl"), source.as_bytes());
    let (module, info) = module(&source);
    if probe.body != Body::ProductionInitializer {
        assert_eq!(module.entry_points.len(), 1);
    }
    let constants = naga::back::PipelineConstants::default();
    let (module, info) = naga::back::pipeline_constants::process_overrides(
        &module,
        &info,
        Some((naga::ShaderStage::Compute, ENTRY)),
        &constants,
    )
    .unwrap();
    let spv_options = naga::back::spv::Options {
        fake_missing_bindings: false,
        bounds_check_policies: naga::proc::BoundsCheckPolicies {
            index: naga::proc::BoundsCheckPolicy::ReadZeroSkipWrite,
            buffer: naga::proc::BoundsCheckPolicy::ReadZeroSkipWrite,
            image_load: naga::proc::BoundsCheckPolicy::ReadZeroSkipWrite,
            binding_array: naga::proc::BoundsCheckPolicy::ReadZeroSkipWrite,
        },
        binding_map: (0..7)
            .map(|binding| {
                (
                    naga::ResourceBinding { group: 0, binding },
                    naga::back::spv::BindingInfo {
                        descriptor_set: 0,
                        binding,
                        binding_array_size: None,
                    },
                )
            })
            .collect(),
        ..Default::default()
    };
    let spv = naga::back::spv::write_vec(
        &module,
        &info,
        &spv_options,
        Some(&naga::back::spv::PipelineOptions {
            shader_stage: naga::ShaderStage::Compute,
            entry_point: ENTRY.into(),
        }),
    );
    match spv {
        Ok(words) => {
            let bytes: Vec<_> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
            write_new(dir, &format!("{name}.offline.spv"), &bytes);
        }
        Err(error) if probe.body == Body::ProductionInitializer => {
            write_new(dir,&format!("{name}.offline-spv-unavailable.txt"),format!("Checked offline SPIR-V was not emitted: {error:?}. No unchecked fallback. Runtime robustness unchanged.\n").as_bytes());
        }
        Err(error) => panic!("offline probe SPIR-V: {error:?}"),
    }
    write_new(
        dir,
        &format!("{name}.offline-spv-options.txt"),
        format!("{spv_options:#?}\n").as_bytes(),
    );
    for shader_model in [
        naga::back::hlsl::ShaderModel::V5_1,
        naga::back::hlsl::ShaderModel::V6_0,
    ] {
        let (hlsl, options, translated_entry) = offline_hlsl(&module, &info, shader_model);
        let suffix = shader_model.to_str();
        write_new(
            dir,
            &format!("{name}.offline-{suffix}.hlsl"),
            hlsl.as_bytes(),
        );
        write_new(
            dir,
            &format!("{name}.offline-{suffix}-options.txt"),
            format!("{options:#?}\nentry={translated_entry}\ntarget=cs_{suffix}\n").as_bytes(),
        );
    }
}

#[test]
fn standalone_sources_validate_and_emit_with_explicit_options() {
    // Temp output is disposable; durable artifact export is a separate opt-in test.
    let dir = std::env::temp_dir().join(format!("alife-probe-emit-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    for probe in probes() {
        export(probe, &dir);
    }
    let small = std::fs::read(dir.join("standalone-barrier.offline-5_1.hlsl")).unwrap();
    let large = std::fs::read(dir.join("abi-barrier.offline-5_1.hlsl")).unwrap();
    assert!(
        large.len() > small.len() * 10,
        "ABI-included HLSL must preserve the intended source-volume confound"
    );
    assert_ne!(digest(&small), digest(&large));
    eprintln!(
        "compiler_probe_offline_barrier_hlsl_bytes standalone={} abi={}",
        small.len(),
        large.len()
    );
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
#[ignore = "CPU-only durable source/HLSL/SPIR-V export; output directory must not exist"]
fn export_standalone_probe_artifacts() {
    let dir = std::env::var_os("ALIFE_COMPILER_PROBE_ARTIFACT_DIR")
        .expect("explicit new artifact directory required");
    let dir = Path::new(&dir);
    std::fs::create_dir(dir).unwrap();
    write_new(dir,"OFFLINE-SCOPE.txt",b"Naga29.0.3 fully specified offline emission. Not driver-extracted SPIR-V or actual HAL HLSL. HLSL uses explicit register=binding/space0 and SM5.1/6.0; native compiler not invoked. See per-file option records. Runtime DX12 logger separately captures actual HAL HLSL and compiler selection. Production sleep is untouched.\n");
    for probe in probes() {
        export(probe, dir);
    }
}

// wgpu-hal already logs the actual DX12 compiler choice and complete generated
// HLSL. Capture them without changing compiler/backend code or shader semantics.
struct HalLog;
static HAL_LOG: HalLog = HalLog;
static LOG_ONCE: Once = Once::new();
static HLSL_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static HLSL_CAPTURE_LOCK: Mutex<()> = Mutex::new(());

fn capture_hal_hlsl(dir: &Path, message: &str, sequence: &AtomicU64) -> Option<String> {
    use std::io::Write;
    let message = message.strip_prefix("Naga generated shader for ")?;
    let (label, source) = message.split_once(":\n")?;
    // Serialize numbering and the index append even if internal shaders compile
    // concurrently. A repeated label is a new emission, never an overwrite.
    let _capture = HLSL_CAPTURE_LOCK.lock().unwrap();
    let id = sequence.fetch_add(1, Ordering::Relaxed);
    let file_label: String = label
        .chars()
        .filter(|ch| *ch != '"')
        .take(64)
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    let filename = format!("runtime-hal-{id:06}-{file_label}.hlsl");
    write_new(dir, &filename, source.as_bytes());
    let record = serde_json::json!({
        "sequence": id, "label": label, "file": filename,
        "bytes": source.len(), "sha256": digest(source.as_bytes()),
    });
    let mut index = std::fs::OpenOptions::new()
        .append(true)
        .open(dir.join("runtime-hlsl-index.jsonl"))
        .unwrap();
    writeln!(index, "{record}").unwrap();
    native_stage_trace::set_shader(record);
    Some(filename)
}

#[test]
fn multiple_hal_shader_emissions_keep_distinct_labeled_artifacts() {
    let dir = std::env::temp_dir().join(format!("alife-multiple-hal-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    write_new(&dir, "runtime-hlsl-index.jsonl", b"");
    let sequence = AtomicU64::new(0);
    let inputs = [
        "Naga generated shader for \"initialize_sleep_transaction\" at Compute:\nfirst input\n",
        "Naga generated shader for \"initialize_sleep_transaction\" at Compute:\nsecond input\n",
        "Naga generated shader for \"internal/../helper\" at Compute:\ninternal input\n",
    ];
    let mut files = std::collections::BTreeSet::new();
    for (id, message) in inputs.iter().enumerate() {
        let filename = capture_hal_hlsl(&dir, message, &sequence).unwrap();
        assert!(
            files.insert(filename.clone()),
            "emissions must never share a filename"
        );
        assert_eq!(Path::new(&filename).components().count(), 1);
        assert!(
            filename.contains("Compute"),
            "stage label must be preserved"
        );
        let source = message.split_once(":\n").unwrap().1;
        assert_eq!(
            std::fs::read(dir.join(filename)).unwrap(),
            source.as_bytes()
        );
        assert_eq!(sequence.load(Ordering::Relaxed), id as u64 + 1);
    }
    assert!(
        capture_hal_hlsl(&dir, "Using dynamic DXC for shader compilation", &sequence).is_none()
    );
    let records = std::fs::read_to_string(dir.join("runtime-hlsl-index.jsonl")).unwrap();
    let records: Vec<serde_json::Value> = records
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(records.len(), inputs.len());
    for (id, record) in records.iter().enumerate() {
        assert_eq!(record["sequence"], id);
        let filename = record["file"].as_str().unwrap();
        let bytes = std::fs::read(dir.join(filename)).unwrap();
        assert_eq!(record["bytes"], bytes.len());
        assert_eq!(record["sha256"], digest(&bytes));
    }
    std::fs::remove_dir_all(dir).unwrap();
}
impl log::Log for HalLog {
    fn enabled(&self, m: &log::Metadata<'_>) -> bool {
        m.target().starts_with("wgpu_hal::dx12")
            || m.target().starts_with("wgpu_hal::auxil::dxgi::result")
    }
    fn log(&self, r: &log::Record<'_>) {
        if !self.enabled(r.metadata()) {
            return;
        }
        let message = format!("{}", r.args());
        eprintln!(
            "compiler_probe_hal level={} target={} thread={:?} active_native_stage={:?} shader={:?} message={message}",
            r.level(),
            r.target(),
            std::thread::current().id(),
            native_stage_trace::active(),
            native_stage_trace::shader()
        );
        append_trace(
            "runtime-hal.log",
            &format!(
                "level={} target={} thread={:?} active_native_stage={:?} shader={:?} message={message}",
                r.level(),
                r.target(),
                std::thread::current().id(),
                native_stage_trace::active(),
                native_stage_trace::shader()
            ),
        );
        if message.starts_with("Naga generated shader for ") {
            if let Some((_, source)) = message.split_once(":\n") {
                eprintln!(
                    "compiler_probe_actual_hlsl bytes={} sha256={}",
                    source.len(),
                    digest(source.as_bytes())
                );
                if let Some(dir) = std::env::var_os("ALIFE_COMPILER_PROBE_ARTIFACT_DIR") {
                    let filename =
                        capture_hal_hlsl(Path::new(&dir), &message, &HLSL_SEQUENCE).unwrap();
                    stage(&format!("actual_hal_hlsl_emitted file={filename}"));
                }
            }
        }
    }
    fn flush(&self) {}
}
static TRACE_OUTPUT_LOCK: Mutex<()> = Mutex::new(());
fn append_trace_file(path: &Path, message: &str) {
    use std::io::Write;
    let _output = TRACE_OUTPUT_LOCK.lock().unwrap();
    let mut out = std::fs::OpenOptions::new().append(true).open(path).unwrap();
    writeln!(out, "{message}").unwrap();
}
fn append_trace(name: &str, message: &str) {
    if let Some(dir) = std::env::var_os("ALIFE_COMPILER_PROBE_ARTIFACT_DIR") {
        append_trace_file(&Path::new(&dir).join(name), message);
    }
}
fn stage(message: &str) {
    eprintln!("compiler_probe_stage={message}");
    append_trace("runtime-stages.log", message);
}
fn install_hal_log() {
    LOG_ONCE.call_once(|| {
        log::set_logger(&HAL_LOG)
            .expect("exact HAL capture requires ownership of the test process logger");
        log::set_max_level(log::LevelFilter::Debug);
    });
}

fn selected_probe() -> Probe {
    let selected =
        std::env::var("ALIFE_COMPILER_PROBE_VARIANT").expect("explicit probe variant required");
    probes()
        .find(|probe| probe.name() == selected)
        .expect("unknown probe; use the exact eight exported names")
}
fn run(probe: Probe, software: bool, backends: wgpu::Backends, submit: bool) {
    run_with_pipeline_naming(probe, software, backends, submit, true);
}
fn run_with_pipeline_naming(
    probe: Probe,
    software: bool,
    backends: wgpu::Backends,
    submit: bool,
    name_pipeline: bool,
) {
    install_hal_log();
    native_stage_trace::reset();
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = backends;
    if probe.body == Body::ProductionInitializer && backends == wgpu::Backends::DX12 {
        assert_eq!(std::env::var("ALIFE_GPU_BACKEND").as_deref(), Ok("dx12"));
        assert_eq!(
            std::env::var("ALIFE_GPU_DX12_COMPILER").as_deref(),
            Ok("dxc")
        );
        crate::closed_loop_runtime::configure_gpu_test_dx12_compiler(&mut descriptor).unwrap();
    } else if backends == wgpu::Backends::DX12 {
        descriptor.backend_options.dx12.shader_compiler =
            match std::env::var("ALIFE_COMPILER_PROBE_DX12_COMPILER").as_deref() {
                Ok("fxc") => wgpu::Dx12Compiler::Fxc,
                Ok("dxc") => wgpu::Dx12Compiler::DynamicDxc {
                    dxc_path: "dxcompiler.dll".into(),
                },
                _ => panic!(
                    "DX12 diagnostic requires explicit fxc or dxc; Auto/fallback not allowed"
                ),
            };
    }
    if let Some(dir) = std::env::var_os("ALIFE_COMPILER_PROBE_ARTIFACT_DIR") {
        let dir = Path::new(&dir);
        std::fs::create_dir(dir).unwrap();
        write_new(dir, "runtime.wgsl", probe.source().as_bytes());
        write_new(dir, "runtime-hal.log", b"");
        write_new(dir, "runtime-hlsl-index.jsonl", b"");
        write_new(dir, "runtime-stages.log", b"");
        write_new(dir, "runtime-native-stages.jsonl", b"");
        write_new(
            dir,
            "runtime-pipeline-naming.txt",
            format!(
                "pipeline_label={}\nshader_module_label=standalone-test-only (unchanged)\n",
                if name_pipeline {
                    "initialize_sleep_transaction"
                } else {
                    "None; target HAL SetName skipped"
                }
            )
            .as_bytes(),
        );
        write_new(dir,"runtime-request.txt",format!("variant={} backend={backends:?} compiler={:?} flags={:?}\nconstants=[] zero_initialize_workgroup_memory=true cache=None\nrequired_features={:?} required_limits={:?}\nsubmit={submit} software={software}\ncompiler DLL path/version/hash not resolved by this API; record from actual process separately. Actual HAL HLSL is captured if DX12 emits it. Offline SPIR-V is not driver-extracted. DXC debug/optimization flags depend on actual device workarounds; no compiler stage success is inferred from HLSL emission.\n",probe.name(),descriptor.backend_options.dx12.shader_compiler,descriptor.flags,crate::closed_loop_runtime::required_device_features(),crate::closed_loop_runtime::required_device_limits()).as_bytes());
    }
    eprintln!("compiler_probe_begin variant={} wgsl_sha256={} backend={backends:?} compiler={:?} flags={:?} compilation_options=constants_empty,zero_init_true cache=None submit={submit}",probe.name(),digest(probe.source().as_bytes()),descriptor.backend_options.dx12.shader_compiler,descriptor.flags);
    native_stage_trace::install_once();
    stage("adapter_request_begin");
    let instance = wgpu::Instance::new(descriptor);
    let adapter = pollster::block_on(instance.enumerate_adapters(backends))
        .into_iter()
        .find(|adapter| {
            let info = adapter.get_info();
            backends.contains(wgpu::Backends::from(info.backend))
                && if software {
                    info.device_type == wgpu::DeviceType::Cpu
                } else {
                    info.vendor == 0x10de && info.device_type == wgpu::DeviceType::DiscreteGpu
                }
        })
        .expect("explicit device/backend absent; no fallback");
    let adapter_info = adapter.get_info();
    eprintln!("compiler_probe_adapter={adapter_info:?}");
    if let Some(dir) = std::env::var_os("ALIFE_COMPILER_PROBE_ARTIFACT_DIR") {
        write_new(
            Path::new(&dir),
            "runtime-adapter.txt",
            format!("{adapter_info:#?}\n").as_bytes(),
        );
    }
    stage("device_request_begin");
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("standalone-compiler-probe"),
        required_features: crate::closed_loop_runtime::required_device_features(),
        required_limits: crate::closed_loop_runtime::required_device_limits(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::MemoryUsage,
        trace: wgpu::Trace::Off,
    }))
    .unwrap();
    stage("layout_begin");
    let scope = scopes(&device);
    let layout = create_neural_bind_group_layout(&device);
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("unchanged-production-seven-bindings"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    check(scope, "layout", ENTRY, None);
    stage("shader_module_begin");
    let scope = scopes(&device);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("standalone-test-only"),
        source: wgpu::ShaderSource::Wgsl(probe.source().into()),
    });
    check(scope, "shader_module", ENTRY, None);
    stage("pipeline_begin");
    let scope = scopes(&device);
    let pipeline = if name_pipeline {
        create_compute_pipeline(&device, &pipeline_layout, &shader, ENTRY, &[])
    } else {
        stage("target_pipeline_label_none_setname_skipped");
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some(ENTRY),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &[],
                zero_initialize_workgroup_memory: true,
            },
            cache: None,
        })
    };
    check(scope, "pipeline", ENTRY, None);
    stage("pipeline_ok");
    if !submit {
        return;
    }
    stage("resources_begin");
    let scope = scopes(&device);
    let initial: Vec<u32> = (0..WORDS as u32).map(|i| 0xa5a5_0000 ^ i).collect();
    let buffers: Vec<_> = (0..7)
        .map(|_| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("synthetic-only-probe"),
                contents: bytemuck::cast_slice(&initial),
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            })
        })
        .collect();
    let entries: Vec<_> = buffers
        .iter()
        .enumerate()
        .map(|(binding, buffer)| wgpu::BindGroupEntry {
            binding: binding as u32,
            resource: buffer.as_entire_binding(),
        })
        .collect();
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("fresh-seven-buffer-probe"),
        layout: &layout,
        entries: &entries,
    });
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("all-state-output"),
        size: (WORDS * 4) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    check(scope, "resources", ENTRY, None);
    let mut cases = 0;
    for flag in [0u32, 1, u32::MAX] {
        for payload in [0u32, 0x1234_5678, u32::MAX] {
            let scope = scopes(&device);
            stage(&format!("encode_begin case={cases}"));
            queue.write_buffer(&buffers[4], 0, bytemuck::cast_slice(&[flag, payload]));
            queue.write_buffer(&buffers[6], 0, bytemuck::cast_slice(&initial));
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("one-synthetic-probe"),
            });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("standalone-probe"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &group, &[]);
                pass.dispatch_workgroups(1, 1, 1);
            }
            encoder.copy_buffer_to_buffer(&buffers[6], 0, &output, 0, (WORDS * 4) as u64);
            stage(&format!("submit_begin case={cases}"));
            let submission = queue.submit(Some(encoder.finish()));
            let polled = device.poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(Duration::from_secs(30)),
            });
            check(scope, "submit_poll", ENTRY, None);
            assert!(polled.is_ok(), "poll={polled:?}");
            let (tx, rx) = std::sync::mpsc::channel();
            output
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |result| {
                    tx.send(result).unwrap();
                });
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(30)),
                })
                .unwrap();
            rx.recv_timeout(Duration::from_secs(30)).unwrap().unwrap();
            let mapped = output.slice(..).get_mapped_range();
            let words: &[u32] = bytemuck::cast_slice(&mapped);
            let mut expected = initial.clone();
            if probe.atomic() && flag == 0 {
                expected[0] = payload;
                expected[3] = 256;
            }
            assert_eq!(
                words,
                expected,
                "variant={} flag={flag} payload={payload}",
                probe.name()
            );
            drop(mapped);
            output.unmap();
            cases += 1;
        }
    }
    eprintln!("compiler_probe_complete variant={} cases={cases} words_per_case={WORDS} software={software}",probe.name());
}
#[test]
#[ignore = "explicit CPU Vulkan only; no NVIDIA/PC qualification"]
fn software_vulkan_standalone_probes_match_cpu_oracle() {
    for probe in probes() {
        run(probe, true, wgpu::Backends::VULKAN, true);
    }
}
fn selected_submit() -> bool {
    match std::env::var("ALIFE_COMPILER_PROBE_PHASE").as_deref() {
        Ok("create") => false,
        Ok("submit") => true,
        _ => panic!("explicit phase create or submit required"),
    }
}
#[test]
#[ignore = "NVIDIA Vulkan synthetic diagnostic; one explicit variant and phase per process"]
fn nvidia_vulkan_selected_standalone_probe() {
    run(
        selected_probe(),
        false,
        wgpu::Backends::VULKAN,
        selected_submit(),
    );
}
#[test]
#[ignore = "NVIDIA DX12 synthetic diagnostic; explicit fxc/dxc, variant and phase, no Auto"]
fn nvidia_dx12_selected_standalone_probe() {
    run(
        selected_probe(),
        false,
        wgpu::Backends::DX12,
        selected_submit(),
    );
}

#[test]
#[ignore = "exact production DX12 initializer, explicit DXC; capture compiler versus native PSO stages"]
fn nvidia_dx12_production_initializer_native_stages() {
    assert!(
        std::env::var_os("ALIFE_COMPILER_PROBE_ARTIFACT_DIR").is_some(),
        "fresh artifact output required"
    );
    run(
        Probe {
            body: Body::ProductionInitializer,
            abi: true,
        },
        false,
        wgpu::Backends::DX12,
        false,
    );
}

#[test]
#[ignore = "exact production DX12/DXC initializer with only target pipeline label omitted; no target SetName"]
fn nvidia_dx12_production_initializer_without_pipeline_setname() {
    assert!(
        std::env::var_os("ALIFE_COMPILER_PROBE_ARTIFACT_DIR").is_some(),
        "fresh artifact output required"
    );
    run_with_pipeline_naming(
        Probe {
            body: Body::ProductionInitializer,
            abi: true,
        },
        false,
        wgpu::Backends::DX12,
        false,
        false,
    );
}

#[test]
#[ignore = "explicit CPU Vulkan check of exact production diagnostic without pipeline name"]
fn software_vulkan_production_initializer_without_pipeline_name() {
    run_with_pipeline_naming(
        Probe {
            body: Body::ProductionInitializer,
            abi: true,
        },
        true,
        wgpu::Backends::VULKAN,
        false,
        false,
    );
}

#[test]
fn production_initializer_is_already_compacted_before_hlsl_compilation() {
    let probe = Probe {
        body: Body::ProductionInitializer,
        abi: true,
    };
    let source = probe.source();
    assert_eq!(source, runtime_shader_source(CLOSED_LOOP_CONSOLIDATE_WGSL));
    let (module, info) = module(&source);
    let before = module.functions.len();
    let entries_before = module.entry_points.len();
    let constants = naga::back::PipelineConstants::default();
    let (prepared, prepared_info) = naga::back::pipeline_constants::process_overrides(
        &module,
        &info,
        Some((naga::ShaderStage::Compute, ENTRY)),
        &constants,
    )
    .unwrap();
    assert_eq!(prepared.entry_points.len(), 1);
    assert!(prepared.functions.len() < before);
    assert!(!prepared
        .functions
        .iter()
        .any(|(_, f)| f.name.as_deref() == Some("sleep_transaction_valid")));
    let mut again = prepared.clone().into_owned();
    naga::compact::compact(&mut again, naga::compact::KeepUnused::No);
    let again_info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&again)
    .unwrap();
    for model in [
        naga::back::hlsl::ShaderModel::V5_1,
        naga::back::hlsl::ShaderModel::V6_0,
    ] {
        let (before_hlsl, _, _) = offline_hlsl(&prepared, &prepared_info, model);
        let (after_hlsl, _, _) = offline_hlsl(&again, &again_info, model);
        assert_eq!(
            before_hlsl, after_hlsl,
            "another compaction is not a new compiler input"
        );
        eprintln!("production_compaction before_entries={entries_before} after_entries={} before_helpers={before} after_helpers={} model={model:?} hlsl_bytes={} sha256={} second_compaction_identical=true wgsl_sha256={}",prepared.entry_points.len(),prepared.functions.len(),before_hlsl.len(),digest(before_hlsl.as_bytes()),digest(source.as_bytes()));
    }
    if let Some(dir) = std::env::var_os("ALIFE_COMPILER_PROBE_ANALYSIS_DIR") {
        let dir = Path::new(&dir);
        std::fs::create_dir(dir).unwrap();
        export(probe, dir);
    }
}

#[test]
#[ignore = "explicit CPU Vulkan create-only check of exact production diagnostic source"]
fn software_vulkan_production_initializer_native_stage_harness() {
    run(
        Probe {
            body: Body::ProductionInitializer,
            abi: true,
        },
        true,
        wgpu::Backends::VULKAN,
        false,
    );
}
