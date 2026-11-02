//! Synthetic compiler discriminators. Barrier removal is never production code.
use super::kernel_diagnostics::{check, scopes};
use super::*;
use std::time::Duration;
use wgpu::util::DeviceExt;

const ABI: &str = include_str!("../../shaders/closed_loop_abi.wgsl");
const SCALAR: &str = include_str!("../../tests/fixtures/sleep_initializer_scalar_1bba259e.wgsl");

#[derive(Clone, Copy, Debug)]
enum Variant {
    MinimalBarrier,
    MinimalEarlyReturn,
    ScalarNoBarrierDiagnosticOnly,
    Converged,
}

fn variant_source(variant: Variant) -> String {
    match variant {
        Variant::Converged => CLOSED_LOOP_CONSOLIDATE_WGSL.to_owned(),
        Variant::MinimalBarrier => format!("{ABI}\n@compute @workgroup_size(1) fn initialize_sleep_transaction(@builtin(global_invocation_id) gid:vec3<u32>) {{ storageBarrier(); }}"),
        Variant::MinimalEarlyReturn => format!("{ABI}\n@compute @workgroup_size(1) fn initialize_sleep_transaction(@builtin(global_invocation_id) gid:vec3<u32>) {{ if (dispatch_header_words[0] != 0u) {{ return; }} storageBarrier(); }}"),
        Variant::ScalarNoBarrierDiagnosticOnly => {
            let start = SCALAR.find("@compute @workgroup_size(1)\nfn initialize_sleep_transaction").unwrap();
            let end = SCALAR[start..].find("\n@compute @workgroup_size(64)").unwrap()+start;
            let body = &SCALAR[start..end];
            assert_eq!(body.matches("  storageBarrier();\n").count(),1);
            let changed = body.replacen("  storageBarrier();\n", "", 1);
            format!("{ABI}{}{}{}", &SCALAR[..start], changed, &SCALAR[end..])
        }
    }
}

#[test]
fn discriminator_sources_validate_and_zero_init_is_not_a_new_program() {
    for variant in [
        Variant::MinimalBarrier,
        Variant::MinimalEarlyReturn,
        Variant::ScalarNoBarrierDiagnosticOnly,
        Variant::Converged,
    ] {
        let source = runtime_shader_source(&variant_source(variant)).into_owned();
        validate_production_shader_contract(&source, &["initialize_sleep_transaction"]).unwrap();
    }
    let source = runtime_shader_source(&format!("{ABI}{SCALAR}")).into_owned();
    let module = naga::front::wgsl::parse_str(&source).unwrap();
    assert!(
        module.overrides.is_empty(),
        "initializer has no pipeline constants to change"
    );
    assert!(module
        .global_variables
        .iter()
        .all(|(_, v)| v.space != naga::AddressSpace::WorkGroup));
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
    let pipeline = naga::back::spv::PipelineOptions {
        shader_stage: naga::ShaderStage::Compute,
        entry_point: "initialize_sleep_transaction".into(),
    };
    let words: Vec<_> = [
        naga::back::spv::ZeroInitializeWorkgroupMemoryMode::None,
        naga::back::spv::ZeroInitializeWorkgroupMemoryMode::Polyfill,
        naga::back::spv::ZeroInitializeWorkgroupMemoryMode::Native,
    ]
    .into_iter()
    .map(|mode| {
        let options = naga::back::spv::Options {
            zero_initialize_workgroup_memory: mode,
            ..Default::default()
        };
        naga::back::spv::write_vec(&module, &info, &options, Some(&pipeline)).unwrap()
    })
    .collect();
    eprintln!(
        "zero_init_spirv_words none={} polyfill={} native={} none_eq_polyfill={} none_eq_native={}",
        words[0].len(),
        words[1].len(),
        words[2].len(),
        words[0] == words[1],
        words[0] == words[2]
    );
    assert!(
        words[0] == words[1],
        "do not assume workgroup-zeroing is a no-op without checking emitted code"
    );
    assert!(
        words[0] == words[2],
        "native zeroing changes code; evaluate separately rather than assume"
    );
}

fn run(variant: Variant, software: bool, backend: wgpu::Backends) {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = backend;
    if !software && backend == wgpu::Backends::DX12 {
        crate::closed_loop_runtime::configure_gpu_test_dx12_compiler(&mut descriptor).unwrap();
    }
    let instance = wgpu::Instance::new(descriptor);
    let adapter = pollster::block_on(instance.enumerate_adapters(backend))
        .into_iter()
        .find(|a| {
            let i = a.get_info();
            if software {
                i.device_type == wgpu::DeviceType::Cpu
            } else {
                i.vendor == 0x10de && i.device_type == wgpu::DeviceType::DiscreteGpu
            }
        })
        .expect("requested backend/device absent; no fallback");
    eprintln!("barrier_discriminator variant={variant:?} adapter={:?} software={software} diagnostic_only=true",adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("isolated-initializer-discriminator"),
        required_features: crate::closed_loop_runtime::required_device_features(),
        required_limits: crate::closed_loop_runtime::required_device_limits(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::MemoryUsage,
        trace: wgpu::Trace::Off,
    }))
    .unwrap();
    let layout = create_neural_bind_group_layout(&device);
    let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("unchanged-seven-buffer-layout"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let zero = [0u32; 1024];
    let buffers: Vec<_> = (0..7)
        .map(|_| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("inactive-synthetic-records"),
                contents: bytemuck::cast_slice(&zero),
                usage: wgpu::BufferUsages::STORAGE,
            })
        })
        .collect();
    let bindings: Vec<_> = buffers
        .iter()
        .enumerate()
        .map(|(binding, buffer)| wgpu::BindGroupEntry {
            binding: binding as u32,
            resource: buffer.as_entire_binding(),
        })
        .collect();
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("synthetic-only"),
        layout: &layout,
        entries: &bindings,
    });
    let source = runtime_shader_source(&variant_source(variant)).into_owned();
    eprintln!("barrier_discriminator_create_begin variant={variant:?}");
    let scope = scopes(&device);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("initialize_sleep_transaction"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pipeline =
        create_compute_pipeline(&device, &pl, &shader, "initialize_sleep_transaction", &[]);
    check(
        scope,
        "discriminator_create",
        "initialize_sleep_transaction",
        None,
    );
    eprintln!("barrier_discriminator_create_ok variant={variant:?}");
    let scope = scopes(&device);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("one-discriminator-submit"),
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("inactive-discriminator"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    eprintln!("barrier_discriminator_submit_begin variant={variant:?}");
    let submission = queue.submit(Some(encoder.finish()));
    let polled = device.poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: Some(Duration::from_secs(30)),
    });
    check(
        scope,
        "discriminator_submit_poll",
        "initialize_sleep_transaction",
        None,
    );
    assert!(polled.is_ok(), "variant={variant:?} poll={polled:?}");
    eprintln!("barrier_discriminator_submit_ok variant={variant:?} software={software} diagnostic_only=true");
}

#[test]
#[ignore = "explicit CPU Vulkan discriminator harness; no hardware claim"]
fn software_vulkan_discriminators() {
    for variant in [
        Variant::MinimalBarrier,
        Variant::MinimalEarlyReturn,
        Variant::ScalarNoBarrierDiagnosticOnly,
        Variant::Converged,
    ] {
        run(variant, true, wgpu::Backends::VULKAN);
    }
}

#[test]
#[ignore = "explicit NVIDIA Vulkan minimal barrier discriminator only"]
fn nvidia_vulkan_minimal_barrier() {
    run(Variant::MinimalBarrier, false, wgpu::Backends::VULKAN);
}
#[test]
#[ignore = "explicit NVIDIA Vulkan minimal early-return discriminator only"]
fn nvidia_vulkan_minimal_early_return() {
    run(Variant::MinimalEarlyReturn, false, wgpu::Backends::VULKAN);
}
#[test]
#[ignore = "DIAGNOSTIC ONLY: removed barrier may never be used for cognition or Current"]
fn nvidia_vulkan_scalar_no_barrier_diagnostic_only() {
    run(
        Variant::ScalarNoBarrierDiagnosticOnly,
        false,
        wgpu::Backends::VULKAN,
    );
}
#[test]
#[ignore = "explicit NVIDIA Vulkan converged candidate, synthetic inactive records"]
fn nvidia_vulkan_converged() {
    run(Variant::Converged, false, wgpu::Backends::VULKAN);
}
#[test]
#[ignore = "explicit NVIDIA DX12 converged candidate, synthetic inactive records"]
fn nvidia_dx12_converged() {
    run(Variant::Converged, false, wgpu::Backends::DX12);
}
