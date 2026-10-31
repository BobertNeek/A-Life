//! Direct-device synthetic assays. Software adapters are allowed only here;
//! normal runtime admission and real checkpoint handling are unchanged.

use super::*;
use crate::{
    TrainingFrozenSynapse, TrainingInitialState, TrainingReplayCandidate, TrainingReplayTick,
};
use alife_core::{
    BrainCapacityClass, BrainGenome, CandidateActionFamily, DendriticBranch, DendriticBranchSet,
    DendriticInputRef, DevelopmentState, NormalizedScalar, PhenotypeCompiler, SensorProfile, Tick,
};
use std::time::Instant;

fn seed_age(index: usize) -> u32 {
    if index.is_multiple_of(11) {
        (1 << 24) + (index % 7) as u32
    } else {
        (index % 7) as u32
    }
}

fn device() -> (wgpu::Device, wgpu::Queue) {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = wgpu::Backends::VULKAN;
    let instance = wgpu::Instance::new(descriptor);
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .expect("direct-device dispatch assay requires a Vulkan adapter");
    eprintln!("dispatch assay adapter: {:?}", adapter.get_info());
    if std::env::var("ALIFE_DISPATCH_REQUIRE_HARDWARE").as_deref() == Ok("1") {
        assert_ne!(
            adapter.get_info().device_type,
            wgpu::DeviceType::Cpu,
            "physical-device qualification may not use a software adapter"
        );
    }
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("test-only-dispatch-assay"),
        required_features: wgpu::Features::empty(),
        required_limits: adapter.limits(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::MemoryUsage,
        trace: wgpu::Trace::Off,
    }))
    .unwrap()
}

fn phenotype() -> BrainPhenotype {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(0xD15A_2048, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.8).unwrap());
    PhenotypeCompiler::compile_testing_procedural_baseline(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap()
}

fn replay(phenotype: &BrainPhenotype, ticks: usize, burn_in: usize) -> TrainingSequence {
    let neurons = phenotype.neuron_count() as usize;
    let sequence = TrainingSequence {
        phenotype_hash: phenotype.phenotype_hash(),
        initial: TrainingInitialState {
            activations: (0..neurons).map(|n| (n % 13) as f32 / 64.0).collect(),
            activity_ema: vec![0.125; neurons],
            metabolic_load: vec![0.25; neurons],
            dendrites: DendriticBranchSet::new(vec![DendriticBranch::new(
                4,
                0.01,
                0.2,
                vec![
                    DendriticInputRef::new(1, 0.125).unwrap(),
                    DendriticInputRef::new(3, -0.25).unwrap(),
                ],
            )
            .unwrap()])
            .unwrap(),
        },
        ticks: (0..ticks)
            .map(|tick| TrainingReplayTick {
                encoded_inputs: (0..neurons)
                    .map(|n| ((n + tick) % 17) as f32 / 128.0)
                    .collect(),
                projection_gain: 0.75,
                local_threshold_shift: tick as f32 / 1024.0,
                // Include inactive and partially active microsteps, as well as
                // a complete active tick. Header offsets still span all steps.
                microstep_count: if tick == ticks - 1 {
                    u32::from(phenotype.microstep_count())
                } else {
                    (tick % (usize::from(phenotype.microstep_count()) + 1)) as u32
                },
                enabled_routes: vec![true; phenotype.projections().len()],
                effective_weight_offsets: vec![0.001; phenotype.synapses().len()],
                structural_synapses: vec![TrainingFrozenSynapse {
                    source: 1,
                    target: 2,
                    route: 0,
                    cadence: u32::from(phenotype.projections()[0].update_cadence().raw()),
                    effective_weight: 0.125,
                }],
                candidates: [CandidateActionFamily::Approach, CandidateActionFamily::Rest]
                    .into_iter()
                    .map(|family| TrainingReplayCandidate {
                        family,
                        decoder_inputs: std::array::from_fn(|lane| (lane % 7) as f32 / 16.0),
                        innate_bias: -0.125,
                    })
                    .collect(),
            })
            .collect(),
        burn_in_ticks: burn_in,
        memory_candidate_gain: phenotype
            .candidate_decoder()
            .memory_channel()
            .map_or(0.0, |p| p.max_candidate_gain()),
    };
    sequence.validate_for(phenotype).unwrap();
    sequence
}

fn gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    phenotype: &BrainPhenotype,
    replay: &TrainingSequence,
) -> TrainerGpuState {
    let mask = StageTrainableMask::from_synapse_indices(
        phenotype,
        &(0..phenotype.synapses().len() as u32)
            .step_by(3)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let weights = phenotype
        .synapses()
        .iter()
        .map(|s| s.genetic_weight())
        .collect::<Vec<_>>();
    let state =
        TrainerGpuState::new_for_replay(device, phenotype, &weights, &mask, Some(replay), None)
            .unwrap();
    let words = pack_replay_sequence(replay, state.layout, phenotype.neuron_count());
    let headers = build_step_headers(
        phenotype,
        state.layout,
        AdamWConfig::default(),
        (1 << 24) + 32,
    )
    .unwrap();
    queue.write_buffer(&state.training, 0, bytemuck::cast_slice(&words));
    queue.write_buffer(&state.step_headers, 0, bytemuck::cast_slice(&headers));
    // Continue nonzero moments and heterogeneous integer ages. Frozen rows must
    // retain every bit; trainable rows must increment their own previous age.
    let n = weights.len();
    let mut optimizer = vec![0_u32; n * 3];
    for index in 0..n {
        optimizer[index] = if seed_age(index) == 0 {
            -0.0_f32
        } else {
            0.01_f32
        }
        .to_bits();
        optimizer[n + index] = if seed_age(index) == 0 {
            0.0_f32
        } else {
            0.02_f32
        }
        .to_bits();
        optimizer[n * 2 + index] = seed_age(index);
    }
    queue.write_buffer(&state._optimizer, 0, bytemuck::cast_slice(&optimizer));
    // A real six-step speech decoder target on every loss tick; burn-in labels
    // remain absent. This exercises speech adjoints entering the recurrent graph.
    let mut speech = vec![0_u32; replay.ticks.len() * 384];
    for tick in replay.burn_in_ticks..replay.ticks.len() {
        for step in 0..6 {
            for output in 0..32 {
                let at = tick * 384 + step * 64 + output * 2;
                speech[at] = (output as f32 / 128.0).to_bits();
                speech[at + 1] = 0.01_f32.to_bits();
            }
        }
    }
    queue.write_buffer(
        &state.training,
        u64::from(state.layout.replay_speech_targets) * 4,
        bytemuck::cast_slice(&speech),
    );
    state
}

/// Original fixed-header binding and pass-per-dispatch algorithm, deliberately
/// independent of the production dynamic binding layout and sweep encoders.
struct StaticOracle {
    groups: Vec<wgpu::BindGroup>,
    forward: wgpu::ComputePipeline,
    local: wgpu::ComputePipeline,
    sources: wgpu::ComputePipeline,
}

impl StaticOracle {
    fn new(device: &wgpu::Device, gpu: &TrainerGpuState, total_steps: u32) -> Self {
        let entries = (0..=8)
            .map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: if binding == 0 {
                        wgpu::BufferBindingType::Uniform
                    } else {
                        wgpu::BufferBindingType::Storage {
                            read_only: matches!(binding, 1 | 4 | 8),
                        }
                    },
                    has_dynamic_offset: false,
                    min_binding_size: if binding == 0 {
                        NonZeroU64::new(HEADER_BYTES)
                    } else {
                        None
                    },
                },
                count: None,
            })
            .collect::<Vec<_>>();
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("static-header-oracle-layout"),
            entries: &entries,
        });
        let groups = (0..total_steps)
            .map(|step| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("static-header-oracle-group"),
                    layout: &layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer: &gpu.step_headers,
                                offset: u64::from(step) * HEADER_BYTES,
                                size: NonZeroU64::new(HEADER_BYTES),
                            }),
                        },
                        entry(1, &gpu._meta),
                        entry(2, &gpu.weights),
                        entry(3, &gpu._optimizer),
                        entry(4, &gpu.training),
                        entry(5, &gpu.state),
                        entry(6, &gpu.gradients),
                        entry(7, &gpu.outputs),
                        entry(8, &gpu.mask),
                    ],
                })
            })
            .collect();
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("static-header-oracle-pipeline-layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("static-header-oracle-wgsl"),
            source: wgpu::ShaderSource::Wgsl(TRAINER_WGSL.into()),
        });
        Self {
            groups,
            forward: pipeline(device, &pipeline_layout, &shader, "forward_microstep"),
            local: pipeline(device, &pipeline_layout, &shader, "backward_local"),
            sources: pipeline(
                device,
                &pipeline_layout,
                &shader,
                "backward_recurrent_sources",
            ),
        }
    }

    fn dispatch(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        pipeline: &wgpu::ComputePipeline,
        step: u32,
        groups: u32,
    ) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &self.groups[step as usize], &[]);
        pass.dispatch_workgroups(groups.max(1), 1, 1);
    }

    fn forward(&self, encoder: &mut wgpu::CommandEncoder, total: u32, groups: u32) {
        for step in 0..total {
            self.dispatch(encoder, &self.forward, step, groups);
        }
    }

    fn backward(&self, encoder: &mut wgpu::CommandEncoder, burn_in: u32, total: u32, groups: u32) {
        for step in (burn_in..total).rev() {
            self.dispatch(encoder, &self.local, step, groups);
            self.dispatch(encoder, &self.sources, step, groups);
        }
    }
}

fn encode_update(
    device: &wgpu::Device,
    gpu: &TrainerGpuState,
    phenotype: &BrainPhenotype,
    oracle: Option<&StaticOracle>,
) -> wgpu::CommandBuffer {
    let groups = phenotype.neuron_count().div_ceil(WORKGROUP_SIZE);
    let weights = (phenotype.synapses().len() as u32).div_ceil(WORKGROUP_SIZE);
    let total = gpu.layout.ticks * u32::from(phenotype.microstep_count());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.clear_buffer(&gpu.state, 0, None);
    encoder.clear_buffer(&gpu.gradients, 0, None);
    encoder.clear_buffer(&gpu.outputs, 0, None);
    dispatch(
        &mut encoder,
        &gpu.pipelines.initialize,
        &gpu.bind_group,
        groups,
        1,
        "assay-init",
    );
    match oracle {
        Some(oracle) => oracle.forward(&mut encoder, total, groups),
        None => gpu.record_forward(&mut encoder, total, groups),
    }
    dispatch(
        &mut encoder,
        &gpu.pipelines.candidate_forward,
        &gpu.bind_group,
        1,
        gpu.layout.ticks,
        "assay-candidates",
    );
    dispatch(
        &mut encoder,
        &gpu.pipelines.speech_forward,
        &gpu.bind_group,
        gpu.layout.ticks,
        1,
        "assay-speech",
    );
    let mut values = vec![0_f32; (gpu.layout.ticks * gpu.layout.candidate_capacity) as usize];
    for tick in gpu.layout.burn_in_steps / u32::from(phenotype.microstep_count())..gpu.layout.ticks
    {
        values[(tick * gpu.layout.candidate_capacity) as usize] = 0.125;
        values[(tick * gpu.layout.candidate_capacity + 1) as usize] = -0.25;
    }
    let adjoints = buffer_init(
        device,
        "assay-adjoints",
        &values,
        wgpu::BufferUsages::COPY_SRC,
    );
    encoder.copy_buffer_to_buffer(
        &adjoints,
        0,
        &gpu.gradients,
        u64::from(gpu.layout.adjoints) * 4,
        values.len() as u64 * 4,
    );
    dispatch(
        &mut encoder,
        &gpu.pipelines.seed_candidate,
        &gpu.bind_group,
        groups,
        gpu.layout.ticks,
        "assay-seed-candidates",
    );
    dispatch(
        &mut encoder,
        &gpu.pipelines.seed_speech,
        &gpu.bind_group,
        groups,
        gpu.layout.ticks,
        "assay-seed-speech",
    );
    match oracle {
        Some(oracle) => oracle.backward(&mut encoder, gpu.layout.burn_in_steps, total, groups),
        None => gpu.record_backward(&mut encoder, total, groups),
    }
    for (kernel, count) in [
        (&gpu.pipelines.recurrent_gradients, weights),
        (&gpu.pipelines.candidate_gradients, weights),
        (&gpu.pipelines.speech_gradients, weights),
        (&gpu.pipelines.gradient_norm, 1),
        (&gpu.pipelines.validate_adamw, 1),
        (&gpu.pipelines.adamw, weights),
    ] {
        dispatch(
            &mut encoder,
            kernel,
            &gpu.bind_group,
            count,
            1,
            "assay-update",
        );
    }
    encoder.finish()
}

fn read(device: &wgpu::Device, queue: &wgpu::Queue, buffer: &wgpu::Buffer) -> Vec<u32> {
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("dispatch-assay-readback"),
        size: buffer.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, buffer.size());
    let commands = encoder.finish();
    let (send, receive) = mpsc::channel();
    commands.map_buffer_on_submit(&staging, wgpu::MapMode::Read, .., move |r| {
        send.send(r).unwrap();
    });
    let submission = queue.submit(Some(commands));
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .unwrap();
    receive.recv().unwrap().unwrap();
    let words = bytemuck::cast_slice(&staging.slice(..).get_mapped_range()).to_vec();
    staging.unmap();
    words
}

#[test]
#[ignore = "requires an explicitly selected Vulkan device; accepts software only in this test fixture"]
fn dynamic_dispatch_matches_static_headers_through_adam() {
    let (device, queue) = device();
    let phenotype = phenotype();
    for (ticks, burn_in) in [(3, 0), (5, 2)] {
        let replay = replay(&phenotype, ticks, burn_in);
        let candidate = gpu(&device, &queue, &phenotype, &replay);
        let baseline = gpu(&device, &queue, &phenotype, &replay);
        let total = ticks as u32 * u32::from(phenotype.microstep_count());
        let oracle = StaticOracle::new(&device, &baseline, total);
        queue.submit([
            encode_update(&device, &baseline, &phenotype, Some(&oracle)),
            encode_update(&device, &candidate, &phenotype, None),
        ]);
        for (label, actual, expected) in [
            ("state", &candidate.state, &baseline.state),
            ("outputs", &candidate.outputs, &baseline.outputs),
            ("gradients", &candidate.gradients, &baseline.gradients),
            ("weights", &candidate.weights, &baseline.weights),
            ("optimizer", &candidate._optimizer, &baseline._optimizer),
        ] {
            let actual = read(&device, &queue, actual);
            let expected = read(&device, &queue, expected);
            assert_eq!(actual.len(), expected.len());
            assert_eq!(
                actual.iter().zip(&expected).position(|(a, b)| a != b),
                None,
                "{label}: first differing u32, ticks={ticks}, burn_in={burn_in}"
            );
        }
        let optimizer = read(&device, &queue, &candidate._optimizer);
        let weights = read(&device, &queue, &candidate.weights);
        let count = phenotype.synapses().len();
        for index in 0..count {
            assert_eq!(
                optimizer[count * 2 + index],
                seed_age(index) + u32::from(index % 3 == 0)
            );
            if index % 3 != 0 {
                assert_eq!(
                    optimizer[index],
                    if seed_age(index) == 0 {
                        -0.0_f32
                    } else {
                        0.01_f32
                    }
                    .to_bits()
                );
                assert_eq!(
                    optimizer[count + index],
                    if seed_age(index) == 0 {
                        0.0_f32
                    } else {
                        0.02_f32
                    }
                    .to_bits()
                );
                assert_eq!(
                    weights[index],
                    phenotype.synapses()[index].genetic_weight().to_bits()
                );
            }
        }
        let outputs = read(&device, &queue, &candidate.outputs);
        assert_eq!(
            f32::from_bits(outputs[candidate.layout.metrics as usize + 3]),
            1.0
        );
        assert!(f32::from_bits(outputs[candidate.layout.metrics as usize + 2]) > 0.0);
        eprintln!("dispatch equivalence passed: ticks={ticks}, burn_in={burn_in}, microsteps={}, synapses={count}", phenotype.microstep_count());
    }
}

#[test]
#[ignore = "host encoding assay, no submitted GPU work or training throughput claim"]
fn dynamic_dispatch_host_encoding_assay() {
    let (device, queue) = device();
    let phenotype = phenotype();
    let mut receipts = Vec::new();
    for (ticks, burn_in) in [(256, 0), (384, 128), (512, 128)] {
        let replay = replay(&phenotype, ticks, burn_in);
        let gpu = gpu(&device, &queue, &phenotype, &replay);
        // Retire fixture uploads before timing command recording. No neural
        // dispatches are submitted by this host-only assay.
        queue.submit([]);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .unwrap();
        let total = ticks as u32 * u32::from(phenotype.microstep_count());
        let started = Instant::now();
        let oracle = StaticOracle::new(&device, &gpu, total);
        let oracle_setup = started.elapsed().as_secs_f64();
        let groups = phenotype.neuron_count().div_ceil(WORKGROUP_SIZE);
        // Alternate order in seven paired trials; report raw samples, including
        // the first cold pair. No dispatch is submitted in this host-only assay.
        for trial in 0..7 {
            for legacy in if trial % 2 == 0 {
                [true, false]
            } else {
                [false, true]
            } {
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                let started = Instant::now();
                if legacy {
                    oracle.forward(&mut encoder, total, groups);
                    oracle.backward(&mut encoder, gpu.layout.burn_in_steps, total, groups);
                } else {
                    gpu.record_forward(&mut encoder, total, groups);
                    gpu.record_backward(&mut encoder, total, groups);
                }
                let encode_seconds = started.elapsed().as_secs_f64();
                let started = Instant::now();
                let commands = encoder.finish();
                let finish_seconds = started.elapsed().as_secs_f64();
                drop(commands);
                let receipt = serde_json::json!({"ticks":ticks,"burn_in":burn_in,
                    "microsteps":phenotype.microstep_count(),"trial":trial,"legacy":legacy,
                    "encode_seconds":encode_seconds,"finish_seconds":finish_seconds,
                    "oracle_setup_seconds_including_pipelines":oracle_setup});
                eprintln!("{receipt}");
                receipts.push(receipt);
            }
        }
    }
    if let Ok(path) = std::env::var("ALIFE_DISPATCH_ASSAY_PATH") {
        std::fs::write(path, serde_json::to_vec_pretty(&receipts).unwrap()).unwrap();
    }
}
