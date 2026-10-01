//! CPU contract checks and a bounded real-GPU probe of the production time helpers.
//! This is not a CPU neural/plasticity implementation or a gameplay success test.

use alife_gpu_backend::{CLOSED_LOOP_ELIGIBILITY_WGSL, CLOSED_LOOP_PLASTICITY_WGSL};

const TIME_PROBE: &str = r#"
@group(0) @binding(7) var<storage,read_write> time_probe:array<f32>;
@compute @workgroup_size(1)
fn probe_simulation_time() {
  time_probe[0] = eligibility_decay_for_ticks(0.95,vec2<u32>(0u,0u));
  time_probe[1] = eligibility_decay_for_ticks(0.95,vec2<u32>(1u,0u));
  time_probe[2] = eligibility_decay_for_ticks(0.95,vec2<u32>(5u,0u));
  time_probe[3] = eligibility_decay_for_ticks(0.95,vec2<u32>(10u,0u));
  time_probe[4] = eligibility_decay_for_ticks(0.95,vec2<u32>(20u,0u));
  time_probe[5] = eligibility_decay_for_ticks(0.95,vec2<u32>(26u,0u));
  time_probe[6] = eligibility_decay_for_ticks(0.0,vec2<u32>(0u,0u));
  time_probe[7] = eligibility_decay_for_ticks(0.0,vec2<u32>(1u,0u));
  time_probe[8] = eligibility_decay_for_ticks(1.0,vec2<u32>(0xffffffffu,0xffffffffu));
  time_probe[9] = eligibility_decay_for_ticks(0.95,vec2<u32>(0u,1u));
  let borrowed = simulation_tick_delta(vec2<u32>(3u,1u),vec2<u32>(0xfffffffdu,0u));
  time_probe[10] = f32(borrowed.x);
  time_probe[11] = f32(borrowed.y);
  time_probe[12] = select(0.0,1.0,simulation_tick_less(vec2<u32>(0xffffffffu,0u),vec2<u32>(0u,1u)));
  var outcome:GpuOutcomeCreditRecord;
  outcome.originating_tick = vec2<u32>(900u,0u);
  outcome.outcome_tick = vec2<u32>(905u,0u);
  time_probe[13] = eligibility_at_outcome(0.5,0.95,outcome);
  outcome.outcome_tick = vec2<u32>(901u,0u);
  time_probe[14] = eligibility_at_outcome(0.5,0.95,outcome);
  time_probe[15] = canonicalize_state_zero(-0.5 * eligibility_decay_for_ticks(0.0,vec2<u32>(1u,0u)));
  time_probe[16] = canonicalize_state_zero(-0.5 * eligibility_decay_for_ticks(0.95,vec2<u32>(0u,1u)));
}
"#;

fn functions_called_from(source: &str, name: &str) -> Vec<String> {
    fn visit(block: &naga::Block, handles: &mut Vec<naga::Handle<naga::Function>>) {
        for statement in block.iter() {
            match statement {
                naga::Statement::Call { function, .. } => handles.push(*function),
                naga::Statement::Block(block) => visit(block, handles),
                naga::Statement::If { accept, reject, .. } => {
                    visit(accept, handles);
                    visit(reject, handles);
                }
                naga::Statement::Switch { cases, .. } => {
                    for case in cases {
                        visit(&case.body, handles);
                    }
                }
                naga::Statement::Loop {
                    body, continuing, ..
                } => {
                    visit(body, handles);
                    visit(continuing, handles);
                }
                _ => {}
            }
        }
    }
    let module = naga::front::wgsl::parse_str(source).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
    let body = module
        .entry_points
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.function.body)
        .or_else(|| {
            module
                .functions
                .iter()
                .find(|(_, function)| function.name.as_deref() == Some(name))
                .map(|(_, function)| &function.body)
        })
        .unwrap();
    let mut calls = Vec::new();
    visit(body, &mut calls);
    calls
        .into_iter()
        .map(|handle| module.functions[handle].name.clone().unwrap())
        .collect()
}

#[test]
fn simulation_time_credit_and_replay_use_the_same_outcome_helper() {
    for function in [
        "evaluate_fast_plasticity_synapse",
        "fast_plasticity_replay_eligibility",
    ] {
        assert!(functions_called_from(CLOSED_LOOP_PLASTICITY_WGSL, function)
            .iter()
            .any(|name| name == "eligibility_at_outcome"));
    }
    for entry in [
        "accumulate_recurrent_eligibility",
        "accumulate_decoder_eligibility",
    ] {
        let calls = functions_called_from(CLOSED_LOOP_ELIGIBILITY_WGSL, entry);
        for expected in ["trace_elapsed_ticks", "eligibility_decay_for_ticks"] {
            assert!(calls.iter().any(|name| name == expected));
        }
    }
    let calls = functions_called_from(CLOSED_LOOP_ELIGIBILITY_WGSL, "prevalidate_eligibility");
    assert!(calls
        .iter()
        .any(|name| name == "replay_trace_clock_is_valid"));
}

#[test]
fn simulation_time_probe_validates_with_the_exact_production_helpers() {
    let source = format!("{CLOSED_LOOP_PLASTICITY_WGSL}\n{TIME_PROBE}");
    let calls = functions_called_from(&source, "probe_simulation_time");
    for expected in [
        "eligibility_decay_for_ticks",
        "simulation_tick_delta",
        "simulation_tick_less",
        "eligibility_at_outcome",
    ] {
        assert!(calls.iter().any(|name| name == expected));
    }
}

#[cfg(feature = "gpu-tests")]
#[test]
fn simulation_time_real_gpu_probe_preserves_reference_decay_and_attenuates_delay() {
    pollster::block_on(async {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .enumerate_adapters(wgpu::Backends::all())
            .await
            .into_iter()
            .find(|adapter| {
                matches!(
                    adapter.get_info().device_type,
                    wgpu::DeviceType::IntegratedGpu | wgpu::DeviceType::DiscreteGpu
                )
            })
            .expect("gpu-tests requires integrated/discrete GPU hardware");
        println!("simulation-time helper adapter: {:?}", adapter.get_info());
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("simulation-time-helper-test"),
                required_features: wgpu::Features::empty(),
                required_limits: adapter.limits(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap();
        let source = format!("{CLOSED_LOOP_PLASTICITY_WGSL}\n{TIME_PROBE}");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("production-time-helper-probe"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("simulation-time-helper-probe"),
            layout: None,
            module: &shader,
            entry_point: Some("probe_simulation_time"),
            compilation_options: Default::default(),
            cache: None,
        });
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("simulation-time-values"),
            size: 17 * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("simulation-time-readback"),
            size: 17 * 4,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("simulation-time-helper-bindings"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 7,
                resource: output.as_entire_binding(),
            }],
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, 17 * 4);
        queue.submit([encoder.finish()]);
        let (sender, receiver) = std::sync::mpsc::channel();
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                sender.send(result).unwrap();
            });
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        receiver.recv().unwrap().unwrap();
        let bytes = readback.slice(..).get_mapped_range();
        let values: &[f32] = bytemuck::cast_slice(&bytes);
        // Analytic fixture values, not a CPU execution of the neural kernel.
        let expected = [
            1.0, 0.95, 0.77378094, 0.59873694, 0.35848592, 0.2635201, 1.0, 0.0, 1.0, 0.0, 6.0, 0.0,
            1.0, 0.38689047, 0.475, 0.0, 0.0,
        ];
        for (index, (&actual, expected)) in values.iter().zip(expected).enumerate() {
            assert!(
                (actual - expected).abs() <= 2.0e-6,
                "probe[{index}]: {actual} != {expected}"
            );
        }
        assert_eq!(values[1].to_bits(), 0.95_f32.to_bits());
        assert_eq!(values[15].to_bits(), 0);
        assert_eq!(values[16].to_bits(), 0);
    });
}
