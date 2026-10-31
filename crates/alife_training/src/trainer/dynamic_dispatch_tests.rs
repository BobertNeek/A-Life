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
    device_with_features(wgpu::Features::empty())
}

fn device_with_features(features: wgpu::Features) -> (wgpu::Device, wgpu::Queue) {
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
        required_features: features,
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
                    DendriticInputRef::new(3, 0.25).unwrap(),
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
    sweep: Sweep<'_>,
) -> wgpu::CommandBuffer {
    encode_profiled_update(device, gpu, phenotype, sweep, None, None)
}

fn encode_profiled_update(
    device: &wgpu::Device,
    gpu: &TrainerGpuState,
    phenotype: &BrainPhenotype,
    sweep: Sweep<'_>,
    profile: Option<&StageProfile>,
    speech: Option<&ParallelSpeech>,
) -> wgpu::CommandBuffer {
    let groups = phenotype.neuron_count().div_ceil(WORKGROUP_SIZE);
    let weights = (phenotype.synapses().len() as u32).div_ceil(WORKGROUP_SIZE);
    let total = gpu.layout.ticks * u32::from(phenotype.microstep_count());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    timestamp_marker(&mut encoder, profile, 0);
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
    timestamp_marker(&mut encoder, profile, 1);
    match sweep {
        Sweep::Static(oracle) => oracle.forward(&mut encoder, total, groups),
        Sweep::Dynamic => gpu.record_forward(&mut encoder, total, groups),
        Sweep::Persistent(persistent) => persistent.forward(&mut encoder, gpu, total),
    }
    timestamp_marker(&mut encoder, profile, 2);
    dispatch(
        &mut encoder,
        &gpu.pipelines.candidate_forward,
        &gpu.bind_group,
        1,
        gpu.layout.ticks,
        "assay-candidates",
    );
    timestamp_marker(&mut encoder, profile, 3);
    dispatch(
        &mut encoder,
        speech
            .filter(|kernels| kernels.use_forward)
            .map_or(&gpu.pipelines.speech_forward, |kernels| &kernels.forward),
        &gpu.bind_group,
        gpu.layout.ticks,
        1,
        "assay-speech",
    );
    timestamp_marker(&mut encoder, profile, 4);
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
    timestamp_marker(&mut encoder, profile, 5);
    if let Some(kernels) = speech.filter(|kernels| kernels.use_seed) {
        dispatch(
            &mut encoder,
            &kernels.seed,
            &gpu.bind_group,
            gpu.layout.ticks,
            1,
            "assay-parallel-seed-speech",
        );
    } else {
        dispatch(
            &mut encoder,
            &gpu.pipelines.seed_speech,
            &gpu.bind_group,
            groups,
            gpu.layout.ticks,
            "assay-seed-speech",
        );
    }
    timestamp_marker(&mut encoder, profile, 6);
    match sweep {
        Sweep::Static(oracle) => {
            oracle.backward(&mut encoder, gpu.layout.burn_in_steps, total, groups)
        }
        Sweep::Dynamic => gpu.record_backward(&mut encoder, total, groups),
        Sweep::Persistent(persistent) => persistent.backward(&mut encoder, gpu, total),
    }
    timestamp_marker(&mut encoder, profile, 7);
    for (stage, (kernel, count)) in [
        (&gpu.pipelines.recurrent_gradients, weights),
        (&gpu.pipelines.candidate_gradients, weights),
        (&gpu.pipelines.speech_gradients, weights),
        (&gpu.pipelines.gradient_norm, 1),
        (&gpu.pipelines.validate_adamw, 1),
        (&gpu.pipelines.adamw, weights),
    ]
    .into_iter()
    .enumerate()
    {
        dispatch(
            &mut encoder,
            kernel,
            &gpu.bind_group,
            count,
            1,
            "assay-update",
        );
        timestamp_marker(&mut encoder, profile, 8 + stage as u32);
    }
    if let Some(profile) = profile {
        encoder.resolve_query_set(&profile.queries, 0..14, &profile.resolved, 0);
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
            encode_update(&device, &baseline, &phenotype, Sweep::Static(&oracle)),
            encode_update(&device, &candidate, &phenotype, Sweep::Dynamic),
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

#[derive(Clone, Copy)]
enum Sweep<'a> {
    Static(&'a StaticOracle),
    Dynamic,
    Persistent(&'a PersistentSweep),
}

/// Experimental one-workgroup kernels, reached only by these explicit tests.
/// Positive chunks keep dispatch duration bounded; production uses Dynamic.
struct PersistentSweep {
    forward: wgpu::ComputePipeline,
    backward: wgpu::ComputePipeline,
    chunk_steps: u32,
}

impl PersistentSweep {
    fn new(device: &wgpu::Device, gpu: &TrainerGpuState, width: u32, chunk_steps: u32) -> Self {
        assert!(chunk_steps > 0);
        let (forward, backward) = match width {
            64 => ("forward_persistent_64", "backward_persistent_64"),
            128 => ("forward_persistent_128", "backward_persistent_128"),
            256 => ("forward_persistent_256", "backward_persistent_256"),
            _ => panic!("unsupported persistent width"),
        };
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("experimental-persistent-layout"),
            bind_group_layouts: &[Some(&gpu.pipelines.bind_group_layout)],
            immediate_size: 0,
        });
        let source = include_str!("../../shaders/foundation_train_persistent.wgsl");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("experimental-persistent-shader"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline = |entry| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &[("SWEEP_STEPS", f64::from(chunk_steps))],
                    ..Default::default()
                },
                cache: None,
            })
        };
        Self {
            forward: pipeline(forward),
            backward: pipeline(backward),
            chunk_steps,
        }
    }

    fn forward(&self, encoder: &mut wgpu::CommandEncoder, gpu: &TrainerGpuState, total: u32) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&self.forward);
        // Header first, first+chunk, ...; the shader clamps its last partial
        // chunk to total. Every original microstep is executed exactly once.
        for first in (0..total).step_by(self.chunk_steps as usize) {
            pass.set_bind_group(0, &gpu.bind_group, &[first * HEADER_BYTES as u32]);
            pass.dispatch_workgroups(1, 1, 1);
        }
    }

    fn backward(&self, encoder: &mut wgpu::CommandEncoder, gpu: &TrainerGpuState, total: u32) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&self.backward);
        // Each header names the inclusive last step. Subtract the dispatched
        // chunk from the exclusive end; stop at the detached burn-in boundary.
        let mut end = total;
        while end > gpu.layout.burn_in_steps {
            pass.set_bind_group(0, &gpu.bind_group, &[(end - 1) * HEADER_BYTES as u32]);
            pass.dispatch_workgroups(1, 1, 1);
            end = end
                .saturating_sub(self.chunk_steps)
                .max(gpu.layout.burn_in_steps);
        }
    }
}

#[test]
#[ignore = "experimental persistent WGSL; selected Vulkan device, no production admission change"]
fn persistent_dispatch_matches_static_headers_through_adam() {
    let (device, queue) = device();
    let phenotype = phenotype();
    for (ticks, burn_in) in [(3, 0), (5, 2), (9, 1)] {
        let replay = replay(&phenotype, ticks, burn_in);
        let baseline = gpu(&device, &queue, &phenotype, &replay);
        let total = ticks as u32 * u32::from(phenotype.microstep_count());
        let oracle = StaticOracle::new(&device, &baseline, total);
        queue.submit([encode_update(
            &device,
            &baseline,
            &phenotype,
            Sweep::Static(&oracle),
        )]);
        let expected = [
            &baseline.state,
            &baseline.outputs,
            &baseline.gradients,
            &baseline.weights,
            &baseline._optimizer,
        ]
        .map(|buffer| read(&device, &queue, buffer));
        for (width, chunk) in [
            (64, 8),
            (64, 32),
            (128, 8),
            (128, 32),
            (256, 8),
            (256, 32),
            (128, 3),
        ] {
            let candidate = gpu(&device, &queue, &phenotype, &replay);
            let persistent = PersistentSweep::new(&device, &candidate, width, chunk);
            queue.submit([encode_update(
                &device,
                &candidate,
                &phenotype,
                Sweep::Persistent(&persistent),
            )]);
            for ((label, actual), expected) in [
                ("state", &candidate.state),
                ("outputs", &candidate.outputs),
                ("gradients", &candidate.gradients),
                ("weights", &candidate.weights),
                ("optimizer", &candidate._optimizer),
            ]
            .into_iter()
            .zip(&expected)
            {
                let actual = read(&device, &queue, actual);
                assert_eq!(actual.len(), expected.len());
                assert_eq!(
                    actual.iter().zip(expected).position(|(a, b)| a != b),
                    None,
                    "{label}: ticks={ticks},burn_in={burn_in},width={width},chunk={chunk}"
                );
            }
            eprintln!("persistent equivalence passed: ticks={ticks},burn_in={burn_in},width={width},chunk={chunk}");
        }
    }
}

#[test]
#[ignore = "experimental paired execution assay; synthetic replay, no whole-training throughput claim"]
fn persistent_dispatch_execution_assay() {
    let (device, queue) = device();
    let phenotype = phenotype();
    let ticks = std::env::var("ALIFE_PERSISTENT_ASSAY_TICKS")
        .ok()
        .map(|v| v.parse::<usize>().unwrap())
        .unwrap_or(64);
    assert!(ticks > 0 && ticks <= crate::MAX_TRAINING_SEQUENCE_TICKS);
    let burn_in = (ticks / 4).min(128);
    let mut receipts = Vec::new();
    for width in [128, 256] {
        let replay = replay(&phenotype, ticks, burn_in);
        let baseline = gpu(&device, &queue, &phenotype, &replay);
        let candidate = gpu(&device, &queue, &phenotype, &replay);
        let persistent = PersistentSweep::new(&device, &candidate, width, 32);
        queue.submit([]);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .unwrap();
        // Both sides continue their own matched optimizer history for seven
        // paired updates. Only ephemeral replay scratch is cleared by the same
        // common update encoder. Fixture setup and validation reads are outside
        // the reported elapsed; no real creature/checkpoint is involved.
        for trial in 0..7 {
            for legacy in if trial % 2 == 0 {
                [true, false]
            } else {
                [false, true]
            } {
                let (gpu, sweep) = if legacy {
                    (&baseline, Sweep::Dynamic)
                } else {
                    (&candidate, Sweep::Persistent(&persistent))
                };
                let started = Instant::now();
                let commands = encode_update(&device, gpu, &phenotype, sweep);
                let encode_finish_seconds = started.elapsed().as_secs_f64();
                let started = Instant::now();
                let submission = queue.submit([commands]);
                let submit_seconds = started.elapsed().as_secs_f64();
                let started = Instant::now();
                device
                    .poll(wgpu::PollType::Wait {
                        submission_index: Some(submission),
                        timeout: None,
                    })
                    .unwrap();
                let wait_seconds = started.elapsed().as_secs_f64();
                let receipt = serde_json::json!({"ticks":ticks,"burn_in":burn_in,
                    "microsteps":phenotype.microstep_count(),"width":width,"chunk_steps":32,
                    "trial":trial,"dynamic_baseline":legacy,
                    "encode_finish_seconds":encode_finish_seconds,"submit_seconds":submit_seconds,
                    "wait_seconds":wait_seconds,
                    "total_seconds":encode_finish_seconds+submit_seconds+wait_seconds});
                eprintln!("{receipt}");
                receipts.push(receipt);
            }
        }
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
                "{label} differs after seven paired actual updates at width={width}"
            );
        }
    }
    if let Ok(path) = std::env::var("ALIFE_PERSISTENT_ASSAY_PATH") {
        std::fs::write(path, serde_json::to_vec_pretty(&receipts).unwrap()).unwrap();
    }
}

struct StageProfile {
    queries: wgpu::QuerySet,
    resolved: wgpu::Buffer,
}

impl StageProfile {
    fn new(device: &wgpu::Device) -> Self {
        Self {
            queries: device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("test-only-replay-stage-timestamps"),
                ty: wgpu::QueryType::Timestamp,
                count: 14,
            }),
            resolved: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("test-only-replay-stage-timestamp-results"),
                size: 14 * 8,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
        }
    }

    fn read(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<f64> {
        let words = read(device, queue, &self.resolved);
        let stamps = words
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u64::from(pair[0]) | (u64::from(pair[1]) << 32))
            .collect::<Vec<_>>();
        stamps
            .windows(2)
            .map(|pair| {
                pair[1]
                    .checked_sub(pair[0])
                    .expect("non-monotonic device timestamps") as f64
                    * f64::from(queue.get_timestamp_period())
            })
            .collect()
    }
}

fn timestamp_marker(
    encoder: &mut wgpu::CommandEncoder,
    profile: Option<&StageProfile>,
    index: u32,
) {
    if let Some(profile) = profile {
        // An empty timestamp pass avoids optional inside-encoder timestamp
        // features. Its cost belongs to this measured instrumented cohort;
        // matched uninstrumented updates quantify the observer effect.
        let pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("test-only-stage-timestamp-marker"),
            timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                query_set: &profile.queries,
                beginning_of_pass_write_index: Some(index),
                end_of_pass_write_index: None,
            }),
        });
        drop(pass);
    }
}

#[test]
#[ignore = "software/selected Vulkan stage-share assay with measured timestamp observer effect"]
fn replay_speech_stage_share_assay() {
    let (device, queue) = device_with_features(wgpu::Features::TIMESTAMP_QUERY);
    let phenotype = phenotype();
    let replay = replay(&phenotype, 64, 16);
    let baseline = gpu(&device, &queue, &phenotype, &replay);
    let instrumented = gpu(&device, &queue, &phenotype, &replay);
    let profile = StageProfile::new(&device);
    let names = [
        "initialize",
        "recurrent_forward",
        "candidate_forward",
        "speech_forward",
        "adjoints_and_candidate_seed",
        "speech_seed",
        "recurrent_backward",
        "recurrent_weight_gradients",
        "candidate_weight_gradients",
        "speech_weight_gradients",
        "gradient_norm",
        "validate_adam",
        "adam",
    ];
    queue.submit([]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .unwrap();
    let mut receipts = Vec::new();
    for trial in 0..7 {
        for profiled in if trial % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        } {
            let state = if profiled { &instrumented } else { &baseline };
            let started = Instant::now();
            let commands = encode_profiled_update(
                &device,
                state,
                &phenotype,
                Sweep::Dynamic,
                profiled.then_some(&profile),
                None,
            );
            let submission = queue.submit([commands]);
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: Some(submission),
                    timeout: None,
                })
                .unwrap();
            let total_seconds = started.elapsed().as_secs_f64();
            let mut receipt = serde_json::json!({"trial":trial,"profiled":profiled,"total_seconds":total_seconds,
                "ticks":64,"burn_in":16,"microsteps":phenotype.microstep_count()});
            if profiled {
                let times = profile.read(&device, &queue);
                let stages = names
                    .iter()
                    .zip(&times)
                    .map(|(name, value)| ((*name).to_owned(), serde_json::json!(value)))
                    .collect::<serde_json::Map<_, _>>();
                receipt["device_stages_nanoseconds"] = serde_json::Value::Object(stages);
                receipt["device_total_nanoseconds"] = serde_json::json!(times.iter().sum::<f64>());
            }
            eprintln!("{receipt}");
            receipts.push(receipt);
        }
    }
    for (actual, expected) in [
        (&instrumented.state, &baseline.state),
        (&instrumented.outputs, &baseline.outputs),
        (&instrumented.gradients, &baseline.gradients),
        (&instrumented.weights, &baseline.weights),
        (&instrumented._optimizer, &baseline._optimizer),
    ] {
        assert_eq!(
            read(&device, &queue, actual),
            read(&device, &queue, expected),
            "profiling changed replay/update state"
        );
    }
    if let Ok(path) = std::env::var("ALIFE_SPEECH_STAGE_ASSAY_PATH") {
        std::fs::write(path, serde_json::to_vec_pretty(&receipts).unwrap()).unwrap();
    }
}

struct ParallelSpeech {
    forward: wgpu::ComputePipeline,
    seed: wgpu::ComputePipeline,
    use_forward: bool,
    use_seed: bool,
}

impl ParallelSpeech {
    fn new(
        device: &wgpu::Device,
        gpu: &TrainerGpuState,
        use_forward: bool,
        use_seed: bool,
    ) -> Self {
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("experimental-parallel-speech-layout"),
            bind_group_layouts: &[Some(&gpu.pipelines.bind_group_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("experimental-parallel-speech-source"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../shaders/foundation_train_parallel_speech.wgsl").into(),
            ),
        });
        Self {
            forward: pipeline(device, &layout, &shader, "forward_speech_parallel_32"),
            seed: pipeline(device, &layout, &shader, "seed_speech_parallel_32"),
            use_forward,
            use_seed,
        }
    }
}

#[test]
#[ignore = "experimental parallel replay speech; selected Vulkan adapter, no production enable"]
fn parallel_speech_matches_serial_through_adam() {
    let (device, queue) = device();
    let phenotype = phenotype();
    for (ticks, burn_in, labels) in [
        (3, 0, "dense"),
        (5, 2, "dense"),
        (9, 1, "dense"),
        (3, 1, "absent"),
        (3, 1, "sparse"),
    ] {
        let replay = replay(&phenotype, ticks, burn_in);
        let baseline = gpu(&device, &queue, &phenotype, &replay);
        set_speech_label_fixture(&queue, &baseline, labels);
        let total = ticks as u32 * u32::from(phenotype.microstep_count());
        let oracle = StaticOracle::new(&device, &baseline, total);
        queue.submit([encode_update(
            &device,
            &baseline,
            &phenotype,
            Sweep::Static(&oracle),
        )]);
        let expected = [
            &baseline.state,
            &baseline.outputs,
            &baseline.gradients,
            &baseline.weights,
            &baseline._optimizer,
        ]
        .map(|buffer| read(&device, &queue, buffer));
        for (forward, seed) in [(true, false), (false, true), (true, true)] {
            let candidate = gpu(&device, &queue, &phenotype, &replay);
            set_speech_label_fixture(&queue, &candidate, labels);
            let kernels = ParallelSpeech::new(&device, &candidate, forward, seed);
            queue.submit([encode_profiled_update(
                &device,
                &candidate,
                &phenotype,
                Sweep::Dynamic,
                None,
                Some(&kernels),
            )]);
            for ((label, buffer), expected) in [
                ("state", &candidate.state),
                ("outputs", &candidate.outputs),
                ("gradients", &candidate.gradients),
                ("weights", &candidate.weights),
                ("optimizer", &candidate._optimizer),
            ]
            .into_iter()
            .zip(&expected)
            {
                let actual = read(&device, &queue, buffer);
                assert_eq!(actual.len(), expected.len());
                assert_eq!(
                    actual.iter().zip(expected).position(|(a, b)| a != b),
                    None,
                    "{label}:ticks={ticks},burn_in={burn_in},labels={labels},forward={forward},seed={seed}"
                );
            }
            eprintln!("parallel speech exact: ticks={ticks},burn_in={burn_in},labels={labels},forward={forward},seed={seed}");
        }
    }
}

fn set_speech_label_fixture(queue: &wgpu::Queue, gpu: &TrainerGpuState, labels: &str) {
    if labels == "dense" {
        return;
    }
    let mut words = vec![0_u32; gpu.layout.ticks as usize * 384];
    if labels == "sparse" {
        // One late hidden-lane target propagates through earlier unlabelled
        // autoregressive steps; all other target weights remain zero.
        let at = (gpu.layout.ticks as usize - 1) * 384 + 3 * 64 + 19 * 2;
        words[at] = 0.25_f32.to_bits();
        words[at + 1] = 0.5_f32.to_bits();
    } else {
        assert_eq!(labels, "absent");
    }
    queue.write_buffer(
        &gpu.training,
        u64::from(gpu.layout.replay_speech_targets) * 4,
        bytemuck::cast_slice(&words),
    );
}

#[test]
#[ignore = "paired actual-update speech assay; synthetic replay and optional measured profiling only"]
fn parallel_speech_execution_assay() {
    let profiled = std::env::var("ALIFE_SPEECH_PROFILE").as_deref() == Ok("1");
    let (device, queue) = device_with_features(if profiled {
        wgpu::Features::TIMESTAMP_QUERY
    } else {
        wgpu::Features::empty()
    });
    let phenotype = phenotype();
    let replay = replay(&phenotype, 64, 16);
    let baseline = gpu(&device, &queue, &phenotype, &replay);
    let candidate = gpu(&device, &queue, &phenotype, &replay);
    let kernels = ParallelSpeech::new(&device, &candidate, true, true);
    let profile = profiled.then(|| StageProfile::new(&device));
    queue.submit([]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .unwrap();
    let mut receipts = Vec::new();
    for trial in 0..7 {
        for parallel in if trial % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        } {
            let state = if parallel { &candidate } else { &baseline };
            let started = Instant::now();
            let commands = encode_profiled_update(
                &device,
                state,
                &phenotype,
                Sweep::Dynamic,
                profile.as_ref(),
                parallel.then_some(&kernels),
            );
            let encode_finish_seconds = started.elapsed().as_secs_f64();
            let started = Instant::now();
            let submission = queue.submit([commands]);
            let submit_seconds = started.elapsed().as_secs_f64();
            let started = Instant::now();
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: Some(submission),
                    timeout: None,
                })
                .unwrap();
            let wait_seconds = started.elapsed().as_secs_f64();
            let mut receipt = serde_json::json!({"trial":trial,"parallel":parallel,"profiled":profiled,"ticks":64,"burn_in":16,
                "encode_finish_seconds":encode_finish_seconds,"submit_seconds":submit_seconds,"wait_seconds":wait_seconds,
                "total_seconds":encode_finish_seconds+submit_seconds+wait_seconds});
            if let Some(profile) = &profile {
                let times = profile.read(&device, &queue);
                receipt["speech_forward_nanoseconds"] = serde_json::json!(times[3]);
                receipt["speech_seed_nanoseconds"] = serde_json::json!(times[5]);
                receipt["device_total_nanoseconds"] = serde_json::json!(times.iter().sum::<f64>());
            }
            eprintln!("{receipt}");
            receipts.push(receipt);
        }
    }
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
            "{label}:seven-update continuation"
        );
    }
    if let Ok(path) = std::env::var("ALIFE_PARALLEL_SPEECH_ASSAY_PATH") {
        std::fs::write(path, serde_json::to_vec_pretty(&receipts).unwrap()).unwrap();
    }
}
