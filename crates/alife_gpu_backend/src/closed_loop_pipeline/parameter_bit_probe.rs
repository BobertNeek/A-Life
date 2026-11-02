//! Synthetic observation of serialized-float predicates, not a care qualifier.
use super::kernel_diagnostics::{check, scopes};
use super::*;
use sha2::{Digest, Sha256};
use wgpu::util::DeviceExt;

const INPUTS: &[u32] = &[
    0, 0x80000000, 1, 2, 0x007ffffe, 0x007fffff, 0x80000001, 0x807fffff, 0x00800000, 0x80800000,
    0x3f7fffff, 0x3f800000, 0x3f800001, 0x40ffffff, 0x41000000, 0x41000001, 0xbf800000, 0xc1000000,
    0x7f7fffff, 0xff7fffff, 0x7f800000, 0xff800000, 0x7fc00000, 0xffc00000, 0x7f800001, 0xff800001,
];
const BODY: &str = r#"
@compute @workgroup_size(1)
fn probe_sleep_parameter_bits(@builtin(global_invocation_id) gid:vec3<u32>) {
  let raw = immutable_plan_words[gid.x];
  let value = bitcast<f32>(raw);
  let base = gid.x*16u;
  atomicStore(&mutable_state_words[base+0u],raw);
  atomicStore(&mutable_state_words[base+1u],bitcast<u32>(value));
  atomicStore(&mutable_state_words[base+2u],select(0u,1u,consolidate_finite(value)));
  atomicStore(&mutable_state_words[base+3u],select(0u,1u,value > 0.0));
  atomicStore(&mutable_state_words[base+4u],select(0u,1u,value >= 0.0));
  atomicStore(&mutable_state_words[base+5u],select(0u,1u,value <= 1.0));
  atomicStore(&mutable_state_words[base+6u],select(0u,1u,value <= 8.0));
  atomicStore(&mutable_state_words[base+7u],select(0u,1u,sleep_parameter_finite_bits(raw)));
  atomicStore(&mutable_state_words[base+8u],select(0u,1u,sleep_parameter_gt_zero_bits(raw)));
  atomicStore(&mutable_state_words[base+9u],select(0u,1u,sleep_parameter_ge_zero_bits(raw)));
  atomicStore(&mutable_state_words[base+10u],select(0u,1u,sleep_parameter_le_positive_bits(raw,0x3f800000u)));
  atomicStore(&mutable_state_words[base+11u],select(0u,1u,sleep_parameter_le_positive_bits(raw,0x41000000u)));
  atomicStore(&mutable_state_words[base+12u],~raw);
  let prior = atomicMax(&mutable_state_words[base+14u],0x200u);
  atomicStore(&mutable_state_words[base+13u],prior);
  atomicStore(&mutable_state_words[base+15u],0x13579bdfu);
}
"#;

fn run(software: bool) {
    // Capture the CPU IEEE reference before requesting native/compiler services.
    let expected: Vec<[u32; 5]> = INPUTS
        .iter()
        .map(|&bits| {
            let value = f32::from_bits(std::hint::black_box(bits));
            [
                value.is_finite(),
                value > 0.0,
                value >= 0.0,
                value <= 1.0,
                value <= 8.0,
            ]
            .map(u32::from)
        })
        .collect();
    assert_eq!(
        expected[2],
        [1, 1, 1, 1, 1],
        "CPU reference must preserve positive subnormal"
    );
    assert_eq!(
        expected[6],
        [1, 0, 0, 1, 1],
        "CPU reference must preserve negative subnormal"
    );
    let backends = if software {
        wgpu::Backends::VULKAN
    } else {
        wgpu::Backends::DX12
    };
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = backends;
    if !software {
        assert_eq!(std::env::var("ALIFE_GPU_BACKEND").as_deref(), Ok("dx12"));
        assert_eq!(
            std::env::var("ALIFE_GPU_DX12_COMPILER").as_deref(),
            Ok("fxc")
        );
        crate::closed_loop_runtime::configure_gpu_test_dx12_compiler(&mut descriptor).unwrap();
    }
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
        .expect("required CPU Vulkan or NVIDIA DX12 adapter absent; no fallback");
    let info = adapter.get_info();
    eprintln!("parameter_bit_probe_adapter={info:?} software={software}");
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("sleep-parameter-bit-diagnostic"),
        required_features: crate::closed_loop_runtime::required_device_features(),
        required_limits: crate::closed_loop_runtime::required_device_limits(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::MemoryUsage,
        trace: wgpu::Trace::Off,
    }))
    .unwrap();
    let layout = create_neural_bind_group_layout(&device);
    let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("production-seven-buffer-layout"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let source = format!(
        "{}{BODY}",
        runtime_shader_source(CLOSED_LOOP_CONSOLIDATE_WGSL)
    );
    eprintln!(
        "parameter_bit_probe_source_sha256={:x}",
        Sha256::digest(source.as_bytes())
    );
    let scope = scopes(&device);
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("parameter-bit-probe"),
        source: wgpu::ShaderSource::Wgsl(source.clone().into()),
    });
    let pipeline =
        create_compute_pipeline(&device, &pl, &module, "probe_sleep_parameter_bits", &[]);
    check(
        scope,
        "parameter_probe_create",
        "probe_sleep_parameter_bits",
        None,
    );
    let zero = [0u32; 1024];
    let buffers: Vec<_> = (0..7)
        .map(|_| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("synthetic-only"),
                contents: bytemuck::cast_slice(&zero),
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            })
        })
        .collect();
    queue.write_buffer(&buffers[2], 0, bytemuck::cast_slice(INPUTS));
    let entries: Vec<_> = buffers
        .iter()
        .enumerate()
        .map(|(binding, buffer)| wgpu::BindGroupEntry {
            binding: binding as u32,
            resource: buffer.as_entire_binding(),
        })
        .collect();
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("synthetic-bindings"),
        layout: &layout,
        entries: &entries,
    });
    let size = (INPUTS.len() * 16 * 4) as u64;
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("parameter-observations"),
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let scope = scopes(&device);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("one-component-dispatch"),
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("observe-float-and-bits"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(INPUTS.len() as u32, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&buffers[6], 0, &output, 0, size);
    let submission = queue.submit(Some(encoder.finish()));
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    check(
        scope,
        "parameter_probe_submit",
        "probe_sleep_parameter_bits",
        None,
    );
    let (tx, rx) = std::sync::mpsc::channel();
    output
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    rx.recv().unwrap().unwrap();
    let mapped = output.slice(..).get_mapped_range();
    let words: &[u32] = bytemuck::cast_slice(&mapped);
    let mut observations = Vec::new();
    let mut float_differences = 0;
    for (index, (&bits, expected)) in INPUTS.iter().zip(&expected).enumerate() {
        let row = &words[index * 16..(index + 1) * 16];
        assert_eq!(row[0], bits, "raw integer input changed");
        assert_eq!(row[12], !bits);
        assert_eq!(row[13], 0);
        assert_eq!(row[14], 0x200);
        assert_eq!(row[15], 0x13579bdf);
        assert_eq!(
            &row[7..12],
            expected,
            "bit predicate differs from CPU IEEE truth for{bits:08x}"
        );
        float_differences += usize::from(&row[2..7] != expected);
        let record = serde_json::json!({"input_bits":format!("{bits:08x}"),"roundtrip_bits":format!("{:08x}",row[1]),"legacy_float_predicates":&row[2..7],"bit_predicates":&row[7..12],"CPU_IEEE_predicates":expected,"integer_atomic_controls_ok":true});
        eprintln!("parameter_bit_probe_observation={record}");
        observations.push(record);
    }
    if software {
        assert_eq!(float_differences, 0, "CPU software reference changed");
    }
    if let Some(dir) = std::env::var_os("ALIFE_PARAMETER_BITS_ARTIFACT_DIR") {
        let dir = std::path::Path::new(&dir);
        std::fs::create_dir(dir).unwrap();
        std::fs::write(dir.join("probe.wgsl"), source).unwrap();
        std::fs::write(dir.join("observations.json"),serde_json::to_vec_pretty(&serde_json::json!({"adapter":format!("{info:?}"),"software":software,"fields":["finite",">0",">=0","<=1","<=8"],"float_differences":float_differences,"observations":observations,"scope":"diagnostic only; not full state/care qualification"})).unwrap()).unwrap();
    }
    drop(mapped);
    output.unmap();
    eprintln!("parameter_bit_probe_complete cases={} integer_bit_predicates_match_CPU=true float_differences={float_differences} diagnostic_only=true",INPUTS.len());
}

#[test]
#[ignore = "explicit CPU-only Vulkan parameter predicate/reference check"]
fn software_vulkan_parameter_bits_match_ieee_reference() {
    run(true);
}
#[test]
#[ignore = "NVIDIA DX12/FXC focused float/subnormal versus raw integer/atomic diagnostic"]
fn nvidia_dx12_fxc_parameter_float_and_bit_observations() {
    run(false);
}
