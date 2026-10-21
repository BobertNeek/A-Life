//! Isolated tests of production eligibility and plasticity kernels on compiled
//! arena layouts. These tests permit software Vulkan adapters.
use super::*;
use alife_gpu_backend::{GpuClassBucketPlan, CLOSED_LOOP_ELIGIBILITY_WGSL};
use wgpu::util::DeviceExt;

const PROBE: &str = r#"
@compute @workgroup_size(1)
fn probe_causal_plasticity() {
  let brain = brain_slots[0];
  let header = load_learning_header(272u);
  let synapse = frame_payload_words[48u];
  let evaluation = evaluate_fast_plasticity_synapse(brain,header,synapse);
  let extension = load_slot_extension(brain);
  let metadata = load_synapse_learning_metadata(extension.synapse_metadata_offset+synapse*8u);
  let receptor = load_plasticity_receptor(extension.receptor_offset+metadata.receptor_index*16u);
  let outcome = load_outcome_credit(header.outcome_offset);
  let staged = load_staging_eligibility_value_direct(brain,metadata);
  let e = eligibility_at_outcome(staged,receptor.eligibility_decay,outcome);
  let m = project_third_factor(receptor,outcome);
  let alpha = bitcast<f32>(immutable_weight_words[brain.alpha_offset+synapse]);
  let genetic = bitcast<f32>(immutable_weight_words[brain.genetic_weight_offset+synapse]);
  let post = load_state_f32(brain.activation_b_offset+metadata.target_neuron);
  let effective = genetic+evaluation.lifetime+alpha*evaluation.fast;
  let old_delta = receptor.learning_rate*alpha*m*e
    - receptor.normalization_rate*abs(m)*post*post*effective;
  frame_payload_words[100u] = bitcast<u32>(evaluation.next_fast);
  frame_payload_words[101u] = bitcast<u32>(clamp(evaluation.fast+old_delta,receptor.fast_min,receptor.fast_max));
  frame_payload_words[102u] = bitcast<u32>(e);
  frame_payload_words[103u] = bitcast<u32>(m);
  frame_payload_words[104u] = bitcast<u32>(staged);
  frame_payload_words[105u] = bitcast<u32>(effective);
  frame_payload_words[106u] = bitcast<u32>(evaluation.lifetime);
  frame_payload_words[107u] = evaluation.guard;
  frame_payload_words[108u] = bitcast<u32>(post);
  frame_payload_words[109u] = bitcast<u32>(receptor.normalization_rate);
  frame_payload_words[110u] = bitcast<u32>(receptor.learning_rate);
  frame_payload_words[111u] = bitcast<u32>(alpha);
}
"#;

fn probe_source() -> String {
    format!("{CLOSED_LOOP_PLASTICITY_WGSL}\n{PROBE}")
}

#[test]
fn causal_probe_uses_complete_validated_production_modules() {
    for source in [CLOSED_LOOP_ELIGIBILITY_WGSL, &probe_source()] {
        let module = naga::front::wgsl::parse_str(source).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap();
    }
}

fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    source: &str,
    entry: &str,
) -> wgpu::ComputePipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(entry),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(entry),
        layout: Some(layout),
        module: &module,
        entry_point: Some(entry),
        compilation_options: Default::default(),
        cache: None,
    })
}

fn dispatch(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline: &wgpu::ComputePipeline,
    bindings: &wgpu::BindGroup,
    count: u32,
) {
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, bindings, &[]);
        pass.dispatch_workgroups(count, 1, 1);
    }
    queue.submit([encoder.finish()]);
}

#[test]
#[ignore = "explicit isolated production-kernel test; software adapter allowed, not runtime acceptance"]
fn actual_route_masks_decoder_selection_and_persisted_eligibility_gate_updates() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = instance
            .enumerate_adapters(wgpu::Backends::VULKAN)
            .await
            .into_iter()
            .next()
            .expect("isolated Vulkan test requires an adapter");
        println!(
            "isolated production-kernel adapter: {:?}",
            adapter.get_info()
        );
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_limits: adapter.limits(),
                ..Default::default()
            })
            .await
            .unwrap();
        let entries = (0..7)
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
            .collect::<Vec<_>>();
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("isolated-production-layout"),
            entries: &entries,
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("isolated-production-layout"),
            bind_group_layouts: &[Some(&bind_layout)],
            immediate_size: 0,
        });
        let recurrent = pipeline(
            &device,
            &layout,
            CLOSED_LOOP_ELIGIBILITY_WGSL,
            "accumulate_recurrent_eligibility",
        );
        let decoder = pipeline(
            &device,
            &layout,
            CLOSED_LOOP_ELIGIBILITY_WGSL,
            "accumulate_decoder_eligibility",
        );
        let evaluate = pipeline(&device, &layout, &probe_source(), "probe_causal_plasticity");

        let capacity = BrainCapacityClass::n512();
        let genome = BrainGenome::scaffold(42, capacity.id());
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
        let phenotype = PhenotypeCompiler::compile(
            &genome,
            &capacity,
            &development,
            SensorProfile::GroundedObjectSlotsV1,
        )
        .unwrap();
        let mut plan = GpuClassBucketPlan::new(capacity, 1).unwrap();
        let slot = plan.insert_phenotype(0, 7, &phenotype).unwrap();
        let brain = *slot.record();
        // Device indices follow the uploaded CSR layout, not phenotype-vector
        // order. Select through the actual uploaded metadata and receptor row.
        let initial = plan.mutable_state_words();
        let extension = brain.extension_record_offset as usize;
        let metadata_base = initial[extension + 7] as usize;
        let receptor_base = initial[extension + 4] as usize;
        let words = plan.immutable_plan_words();
        let rec = (0..brain.recurrent_synapse_count as usize)
            .find(|i| {
                let receptor = words[metadata_base + i * 8 + 4] as usize;
                f32::from_bits(words[receptor_base + receptor * 16 + 3]) > 0.0
            })
            .unwrap();
        let dec = (brain.recurrent_synapse_count as usize..brain.synapse_count as usize)
            .find(|i| {
                let decoder_local = words[metadata_base + i * 8 + 6] as usize;
                words[initial[extension + 6] as usize + decoder_local * 8 + 1] == 1
            })
            .unwrap();
        let mut weights = plan.immutable_weight_words().to_vec();
        for synapse in [rec, dec] {
            weights[brain.genetic_weight_offset as usize + synapse] = 0.5_f32.to_bits();
        }
        let make_buffer = |label, bytes: &[u8]| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytes,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
            })
        };
        let buffers = [
            make_buffer("slots", bytemuck::cast_slice(plan.brain_slot_records())),
            make_buffer(
                "identities",
                bytemuck::cast_slice(plan.phenotype_identities()),
            ),
            make_buffer("plans", bytemuck::cast_slice(plan.immutable_plan_words())),
            make_buffer("weights", bytemuck::cast_slice(&weights)),
            make_buffer("dispatch", bytemuck::cast_slice(&[0_u32; 332])),
            make_buffer("frame", bytemuck::cast_slice(&vec![0_u32; 16_384])),
            make_buffer("state", bytemuck::cast_slice(plan.mutable_state_words())),
        ];
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("isolated-production-bindings"),
            layout: &bind_layout,
            entries: &buffers
                .iter()
                .enumerate()
                .map(|(binding, buffer)| wgpu::BindGroupEntry {
                    binding: binding as u32,
                    resource: buffer.as_entire_binding(),
                })
                .collect::<Vec<_>>(),
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("causal-probe-result"),
            size: 48,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut checked = 0;
        for (event, lanes) in [
            ("none", [0.0_f32; 8]),
            ("novelty", [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0]),
            ("pain", [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
            ("frustration", [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0]),
            ("mixed", [0.0, 0.1, 0.0, 0.2, 0.5, 0.0, 0.0, 0.0]),
        ] {
            for (name, is_decoder, enabled, previous, pre) in [
                ("active-zero", false, true, 0.0_f32, 0.0_f32),
                ("masked-zero", false, false, 0.0, 0.8),
                ("active-tiny", false, true, 0.0, 0.000002),
                ("active-tiny-negative", false, true, 0.0, -0.000002),
                ("active-positive", false, true, 0.0, 0.02),
                ("active-negative", false, true, 0.0, -0.02),
                ("masked-persisted-positive", false, false, 0.4, 0.8),
                ("masked-persisted-negative", false, false, -0.4, 0.8),
                ("active-canceled", false, true, 0.4, -0.76),
                ("active-trace-reversed-negative", false, true, 0.4, -0.8),
                ("active-trace-reversed-positive", false, true, -0.4, 0.8),
                ("masked-expired", false, false, 0.4, 0.8),
                ("unselected-decoder-zero", true, true, 0.0, 0.8),
                ("unselected-decoder-persisted", true, true, 0.4, 0.8),
            ] {
                let synapse = if is_decoder { dec } else { rec };
                let mut state = plan.mutable_state_words().to_vec();
                let ext = brain.extension_record_offset as usize;
                let learning = state[ext + 15] as usize;
                let metadata = state[ext + 7] as usize + synapse * 8;
                let metadata = &plan.immutable_plan_words()[metadata..metadata + 8];
                let source = metadata[2] as usize;
                let target = metadata[3] as usize;
                let local = metadata[5] as usize;
                state[brain.activation_a_offset as usize + source] = pre.to_bits();
                state[brain.activation_b_offset as usize + target] = 0.5_f32.to_bits();
                state[brain.lifetime_weight_offset as usize + synapse] = 0.25_f32.to_bits();
                state[brain.fast_weight_offset as usize + synapse] = 0.2_f32.to_bits();
                state[if is_decoder {
                    brain.decoder_eligibility_offset
                } else {
                    brain.recurrent_eligibility_offset
                } as usize
                    + local] = previous.to_bits();
                state[learning + 1] = 0;
                state[learning + 2] = 0;
                state[learning + 12] = 1;
                state[learning + 13] = 1;
                let event_base = state[learning + 17] as usize;
                state[event_base + 2] = 10;
                state[brain.selection_offset as usize + 5] = 2;
                state[brain.selection_offset as usize + 11] = 1;

                let mut headers = [0_u32; 332];
                headers[272 + 5] = 1;
                headers[272 + 8] = 1;
                headers[272 + 9] = 64;
                headers[272 + 10] = 256;
                headers[272 + 11] = brain.selection_offset;
                headers[272 + 13] = if is_decoder { 0 } else { synapse as u32 + 1 };
                headers[272 + 14] = if is_decoder {
                    synapse as u32 - brain.recurrent_synapse_count + 1
                } else {
                    0
                };
                headers[272 + 15] = 1024;
                if enabled && !is_decoder {
                    let route =
                        plan.immutable_plan_words()[brain.route_indices_offset as usize + synapse];
                    headers[308 + 8 + (route / 32) as usize] |= 1 << (route % 32);
                }
                let mut frame = vec![0_u32; 16_384];
                frame[16] = 11; // pending template's actual origin tick
                frame[48] = synapse as u32;
                if is_decoder {
                    let dec_meta = state[ext + 6] as usize
                        + (synapse - brain.recurrent_synapse_count as usize) * 8;
                    frame[64 + 2] = (plan.immutable_plan_words()[dec_meta + 2] + 1) % 8;
                }
                queue.write_buffer(&buffers[4], 0, bytemuck::cast_slice(&headers));
                queue.write_buffer(&buffers[5], 0, bytemuck::cast_slice(&frame));
                queue.write_buffer(&buffers[6], 0, bytemuck::cast_slice(&state));
                let count = if is_decoder {
                    headers[272 + 14]
                } else {
                    headers[272 + 13]
                }
                .div_ceil(64);
                dispatch(
                    &device,
                    &queue,
                    if is_decoder { &decoder } else { &recurrent },
                    &bindings,
                    count,
                );

                let delay = if name.contains("reversed") {
                    20
                } else if name == "masked-expired" {
                    10_000
                } else {
                    3
                };
                frame[14] = 11;
                frame[16] = 11 + delay;
                for (index, lane) in lanes.iter().copied().enumerate() {
                    frame[34 + index] = lane.to_bits();
                }
                queue.write_buffer(&buffers[5], 0, bytemuck::cast_slice(&frame));
                dispatch(&device, &queue, &evaluate, &bindings, 1);
                let mut encoder = device.create_command_encoder(&Default::default());
                encoder.copy_buffer_to_buffer(&buffers[5], 400, &readback, 0, 48);
                queue.submit([encoder.finish()]);
                let (sender, receiver) = std::sync::mpsc::channel();
                readback
                    .slice(..)
                    .map_async(wgpu::MapMode::Read, move |result| {
                        sender.send(result).unwrap()
                    });
                device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                receiver.recv().unwrap().unwrap();
                let mapped = readback.slice(..).get_mapped_range();
                let result: &[u32] = bytemuck::cast_slice(&mapped);
                let new_fast = f32::from_bits(result[0]);
                let prior_fast = f32::from_bits(result[1]);
                let e = f32::from_bits(result[2]);
                let m = f32::from_bits(result[3]);
                assert_eq!(f32::from_bits(result[8]), 0.5);
                assert!(f32::from_bits(result[9]) > 0.0);
                println!("kernel-input {event}/{name} new={new_fast} prior={prior_fast} p={} lambda={} eta={} alpha={} metadata_receptor={}", f32::from_bits(result[8]), f32::from_bits(result[9]), f32::from_bits(result[10]), f32::from_bits(result[11]), metadata[4]);
                let staged = (0.95 * previous
                    + if enabled && !is_decoder {
                        pre * 0.5
                    } else {
                        0.0
                    })
                .clamp(-1.0, 1.0);
                assert!(
                    (f32::from_bits(result[4]) - staged).abs() < 1e-7,
                    "{event}/{name} staged"
                );
                assert!(
                    (e - staged * 0.95_f32.powi(delay as i32)).abs() < 1e-7,
                    "{event}/{name} outcome eligibility"
                );
                let profile =
                    phenotype.plasticity_receptors()[metadata[4] as usize].receptor_profile();
                assert!(
                    (m - profile
                        .project(&alife_core::NeuromodulatoryFrame::try_new(lanes).unwrap())
                        .unwrap())
                    .abs()
                        < 1e-7
                );
                assert_eq!(result[7], 0, "finite guard {event}/{name}");
                assert_eq!(f32::from_bits(result[6]), 0.25, "lifetime unchanged");
                if e == 0.0 || m == 0.0 {
                    assert_eq!(new_fast, 0.2, "uncredited update {event}/{name}");
                }
                if e == 0.0 && m != 0.0 {
                    assert_ne!(prior_fast, 0.2, "prior leak {event}/{name}");
                }
                if previous != 0.0 && !enabled && m != 0.0 && delay == 3 {
                    assert_ne!(new_fast, 0.2, "persisted credit was lost {event}/{name}");
                }
                if name.contains("reversed") {
                    assert!(
                        e * previous < 0.0,
                        "new coactivation reverses persisted trace"
                    );
                    if m != 0.0 {
                        assert!(
                            (new_fast - 0.2) * m * e > 0.0,
                            "delayed signed learning {event}/{name}"
                        );
                    }
                }
                println!("production-kernel {event}/{name} staged={} e={e} m={m} prior_fast={prior_fast} new_fast={new_fast}", f32::from_bits(result[4]));
                drop(mapped);
                readback.unmap();
                checked += 1;
            }
        }
        println!("{checked} complete production eligibility/evaluator kernel cases passed");
    });
}
