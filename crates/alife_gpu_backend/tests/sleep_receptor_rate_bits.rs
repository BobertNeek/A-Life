//! Encoded receptor-rate admission, using the actual production delta-mode helper.
#[cfg(feature = "gpu-tests")]
use sha2::{Digest, Sha256};
const ABI: &str = include_str!("../shaders/closed_loop_abi.wgsl");
const BODY: &str = include_str!("../shaders/closed_loop_consolidate.wgsl");
const OLD: &str = include_str!("fixtures/barrier_original_d139e44c_consolidate.wgsl");
const PROBE: &str = r#"
fn legacy_rate_mode(base:u32) -> u32 {
  let a = bitcast<f32>(immutable_plan_words[base+1u]);
  let b = bitcast<f32>(immutable_plan_words[base+2u]);
  let c = bitcast<f32>(immutable_plan_words[base+3u]);
  if (!consolidate_finite(a) || !consolidate_finite(b) || !consolidate_finite(c)
      || a < 0.0 || a > 1.0 || b < 0.0 || b > 1.0 || c < 0.0 || c > 1.0) { return 2u; }
  return select(0u,1u,a != 0.0 || b != 0.0 || c != 0.0);
}
@compute @workgroup_size(1)
fn probe_receptor_rate_mode(@builtin(global_invocation_id) gid:vec3<u32>) {
  let row=gid.x;
  if (row >= dispatch_header_words[0]) { return; }
  let extension=load_slot_extension(brain_slots[row]);
  let mode=consolidate_synapse_delta_mode(extension,frame_payload_words[row*2u]);
  atomicStore(&mutable_state_words[row*2u],mode);
  var legacy=0xffffffffu;
  if (frame_payload_words[row*2u+1u] != 0u) { legacy=legacy_rate_mode(extension.receptor_offset); }
  atomicStore(&mutable_state_words[row*2u+1u],legacy);
}
"#;
#[cfg(feature = "gpu-tests")]
#[derive(Clone)]
struct Case {
    name: String,
    rates: [u32; 3],
    guard: u32,
}
fn cpu_mode(bits: [u32; 3]) -> u32 {
    let rates = bits.map(|b| f32::from_bits(std::hint::black_box(b)));
    if rates
        .iter()
        .any(|&r| !r.is_finite() || !(0.0..=1.0).contains(&r))
    {
        2
    } else {
        u32::from(rates.iter().any(|&r| r != 0.0))
    }
}
#[cfg(feature = "gpu-tests")]
fn cases() -> Vec<Case> {
    let mut patterns = std::collections::BTreeSet::from([
        0, 0x80000000, 1, 2, 0x007ffffe, 0x007fffff, 0x80000001, 0x807fffff, 0x00800000,
        0x80800000, 0x3f7fffff, 0x3f800000, 0x3f800001, 0xbf7fffff, 0xbf800000, 0xbf800001,
        0x40000000, 0x7f7fffff, 0xff7fffff, 0x7f800000, 0xff800000, 0x7fc00000, 0xffc00000,
        0x7f800001, 0xff800001,
    ]);
    for seed in [
        0u32, 0x80000000, 1, 0x007fffff, 0x00800000, 0x3f800000, 0x7f800000, 0x7fc00000,
    ] {
        for bit in 0..32 {
            patterns.insert(seed ^ (1 << bit));
        }
    }
    let mut random = 0x13579bdfu32;
    for _ in 0..512 {
        random ^= random << 13;
        random ^= random >> 17;
        random ^= random << 5;
        patterns.insert(random);
    }
    let mut cases = Vec::new();
    for lane in 0..3 {
        for &bits in &patterns {
            let mut rates = [0; 3];
            rates[lane] = bits;
            cases.push(Case {
                name: format!("lane{lane}-{bits:08x}"),
                rates,
                guard: 0,
            });
        }
    }
    for rates in [
        [0x80000000; 3],
        [0x80000000, 1, 0x80000000],
        [1, 0x80000001, 0],
        [1, 0x7fc00000, 0],
        [0x3f800000; 3],
        [1, 1, 1],
    ] {
        cases.push(Case {
            name: format!("mixed-{rates:08x?}"),
            rates,
            guard: 0,
        });
    }
    for guard in 1..=8 {
        cases.push(Case {
            name: format!("bounds-guard-{guard}"),
            rates: [1, 0, 0],
            guard,
        });
    }
    cases
}
#[test]
fn ieee_edges_and_production_source_bounds_are_preserved() {
    for lane in 0..3 {
        for (bits, mode) in [
            (0, 0),
            (0x80000000, 0),
            (1, 1),
            (0x007fffff, 1),
            (0x00800000, 1),
            (0x80000001, 2),
            (0x807fffff, 2),
            (0x80800000, 2),
            (0x3f800000, 1),
            (0x3f800001, 2),
            (0x7f800000, 2),
            (0xff800000, 2),
            (0x7fc00000, 2),
            (0x7f800001, 2),
        ] {
            let mut b = [0; 3];
            b[lane] = bits;
            assert_eq!(cpu_mode(b), mode);
        }
    }
    let start = OLD.find("fn consolidate_synapse_delta_mode(").unwrap();
    let old_end = start
        + OLD[start..]
            .find("  let receptor = load_plasticity_receptor(receptor_base);")
            .unwrap();
    let new_start = BODY.find("fn consolidate_synapse_delta_mode(").unwrap();
    assert!(BODY[new_start..].starts_with(&OLD[start..old_end]));
    let source = format!("{ABI}{BODY}{PROBE}");
    let module = naga::front::wgsl::parse_str(&source).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
}
#[cfg(feature = "gpu-tests")]
mod device {
    use super::*;
    use wgpu::util::DeviceExt;
    fn run(software: bool) {
        let cases = cases();
        // Capture IEEE truth before requesting driver/compiler services (including FTZ state).
        let expected: Vec<u32> = cases
            .iter()
            .map(|c| {
                if c.guard == 0 {
                    cpu_mode(c.rates)
                } else if c.guard >= 7 {
                    1
                } else {
                    2
                }
            })
            .collect();
        assert!(expected.contains(&0) && expected.contains(&1) && expected.contains(&2));
        let n = cases.len();
        let mut brains = vec![0u32; n * 36];
        let mut plan = vec![0u32; n * 64 + 32];
        let mut state = vec![0xa5a54321u32; n * 22];
        let mut frame = vec![0u32; n * 2];
        for (i, c) in cases.iter().enumerate() {
            let base = i * 64;
            let ext = n * 2 + i * 20;
            brains[i * 36 + 32] = ext as u32;
            state[ext..ext + 20].fill(0);
            state[ext] = 4;
            state[ext + 4] = (base + 16) as u32;
            state[ext + 7] = base as u32;
            plan[base] = 0;
            plan[base + 4] = 0;
            plan[base + 17..base + 20].copy_from_slice(&c.rates);
            frame[i * 2 + 1] = u32::from(c.guard == 0);
            match c.guard {
                0 => {}
                1 => {
                    state[ext + 7] = u32::MAX - 3;
                    frame[i * 2] = 1;
                }
                2 => state[ext + 7] = (plan.len() - 7) as u32,
                3 => plan[base] = 7,
                4 => {
                    state[ext + 4] = u32::MAX - 7;
                    plan[base + 4] = 1;
                }
                5 => state[ext + 4] = (plan.len() - 15) as u32,
                6 => state[ext + 4] = u32::MAX,
                7 => {
                    let b = plan.len() - 16;
                    state[ext + 4] = b as u32;
                    plan[b + 1..b + 4].copy_from_slice(&c.rates);
                }
                8 => {
                    let b = plan.len() - 8;
                    state[ext + 7] = b as u32;
                    plan[b..b + 8].fill(0);
                }
                _ => unreachable!(),
            }
        }
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
            alife_gpu_backend::configure_gpu_test_dx12_compiler(&mut descriptor).unwrap();
        }
        let instance = wgpu::Instance::new(descriptor);
        let adapter = pollster::block_on(instance.enumerate_adapters(backends))
            .into_iter()
            .find(|a| {
                let i = a.get_info();
                i.backend
                    == if software {
                        wgpu::Backend::Vulkan
                    } else {
                        wgpu::Backend::Dx12
                    }
                    && if software {
                        i.device_type == wgpu::DeviceType::Cpu
                    } else {
                        i.vendor == 0x10de && i.device_type == wgpu::DeviceType::DiscreteGpu
                    }
            })
            .expect("strict CPU Vulkan or NVIDIA DX12, no fallback");
        let info = adapter.get_info();
        eprintln!("receptor_mode_adapter={info:?} software={software}");
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("synthetic-receptor-admission"),
            required_features: wgpu::Features::empty(),
            required_limits: adapter.limits(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
        }))
        .unwrap();
        let entries: Vec<_> = (0..7)
            .map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage {
                        read_only: binding < 5,
                    },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            })
            .collect();
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &entries,
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let source = format!("{ABI}{BODY}{PROBE}");
        let source_hash = format!("{:x}", Sha256::digest(source.as_bytes()));
        eprintln!("receptor_mode_source_sha256={source_hash}");
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(source.clone().into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pl),
            module: &module,
            entry_point: Some("probe_receptor_rate_mode"),
            compilation_options: Default::default(),
            cache: None,
        });
        let arrays: [&[u32]; 7] = [
            &brains,
            &[0; 8],
            &plan,
            &[0; 8],
            &[n as u32],
            &frame,
            &state,
        ];
        let buffers: Vec<_> = arrays
            .iter()
            .map(|a| {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(a),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                })
            })
            .collect();
        let entries: Vec<_> = buffers
            .iter()
            .enumerate()
            .map(|(i, b)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: b.as_entire_binding(),
            })
            .collect();
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &entries,
        });
        let bytes = (n * 2 * 4) as u64;
        let out = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(n as u32, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&buffers[6], 0, &out, 0, bytes);
        let submitted = queue.submit(Some(encoder.finish()));
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submitted),
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        out.slice(..)
            .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        rx.recv().unwrap().unwrap();
        let mapped = out.slice(..).get_mapped_range();
        let words: &[u32] = bytemuck::cast_slice(&mapped);
        let mut legacy_differences = 0;
        let mut observations = Vec::new();
        for (i, c) in cases.iter().enumerate() {
            assert_eq!(
                words[i * 2],
                expected[i],
                "production mode mismatch {}",
                c.name
            );
            let legacy = (c.guard == 0).then_some(words[i * 2 + 1]);
            if let Some(v) = legacy {
                legacy_differences += usize::from(v != expected[i]);
            }
            let observation = serde_json::json!({"case":c.name,"rates_hex":c.rates.map(|v|format!("{v:08x}")),"expected":expected[i],"actual":words[i*2],"legacy_observed":legacy});
            if c.guard == 0
                && ["00000000", "00000001", "007fffff", "00800000"]
                    .iter()
                    .any(|b| c.name.ends_with(b))
            {
                eprintln!("receptor_mode_discriminator={observation}");
            }
            observations.push(observation);
        }
        if software {
            assert_eq!(legacy_differences, 0, "CPU original comparison changed");
        }
        if let Some(p) = std::env::var_os("ALIFE_RECEPTOR_RATE_ARTIFACT_DIR") {
            let p = std::path::Path::new(&p);
            std::fs::create_dir(p).unwrap();
            std::fs::write(p.join("probe.wgsl"), source).unwrap();
            std::fs::write(p.join("observations.json"),serde_json::to_vec_pretty(&serde_json::json!({"source_sha256":source_hash,"adapter":format!("{info:?}"),"software":software,"legacy_differences":legacy_differences,"cases":observations})).unwrap()).unwrap();
        }
        drop(mapped);
        out.unmap();
        eprintln!("receptor_mode_complete cases={n} lanes=3 bounds_controls=8 production_matches_CPU=true legacy_differences={legacy_differences} software={software} scope=encoded-admission-only");
    }
    #[test]
    #[ignore = "explicit CPU Vulkan actual-helper predicate and bounds regression"]
    fn software_vulkan_matches_ieee_admission() {
        run(true);
    }
    #[test]
    #[ignore = "NVIDIA DX12 FXC raw-bit admission with legacy float observations; synthetic only"]
    fn nvidia_dx12_fxc_matches_ieee_admission() {
        run(false);
    }
}
