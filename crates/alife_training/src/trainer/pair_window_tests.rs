//! Experimental two-window replay, never referenced by production dispatch.
//! Fixed adjoints isolate actor replay and ordered Adam equivalence. The real
//! PPO objective, KL gate, value accumulation/rollback are NOT integrated here.
use super::dynamic_dispatch_tests as fixture;
use super::*;
use std::time::Instant;

const PAIR_HEADER_BYTES: u64 = HEADER_BYTES * 2;
const PAIR_WGSL: &str = include_str!("../../shaders/foundation_train_pair.wgsl");

struct PairGpu {
    _headers: wgpu::Buffer,
    _meta: wgpu::Buffer,
    weights: wgpu::Buffer,
    optimizer: wgpu::Buffer,
    _training: wgpu::Buffer,
    state: wgpu::Buffer,
    gradients: wgpu::Buffer,
    outputs: wgpu::Buffer,
    _mask: wgpu::Buffer,
    bind: wgpu::BindGroup,
    views: [wgpu::BindGroup; 2],
    initialize: wgpu::ComputePipeline,
    forward: wgpu::ComputePipeline,
    candidates: wgpu::ComputePipeline,
    speech: wgpu::ComputePipeline,
    seed: wgpu::ComputePipeline,
    seed_speech: wgpu::ComputePipeline,
    local: wgpu::ComputePipeline,
    sources: wgpu::ComputePipeline,
    recurrent: wgpu::ComputePipeline,
    decoder: wgpu::ComputePipeline,
    speech_gradients: wgpu::ComputePipeline,
    norm: wgpu::ComputePipeline,
    layouts: [PackedLayout; 2],
    bases: [[u64; 4]; 2], // state, gradient, output, training word offsets
    forward_steps: u32,
    backward_steps: u32,
    backward_header: u32,
}

fn shift(
    layout: &mut PackedLayout,
    meta: u32,
    training: u32,
    state: u32,
    gradient: u32,
    output: u32,
) {
    for word in [
        &mut layout.speech_offsets,
        &mut layout.target_offsets,
        &mut layout.incoming_ids,
        &mut layout.source_offsets,
        &mut layout.outgoing_ids,
        &mut layout.synapse_records,
        &mut layout.dynamics,
        &mut layout.family_biases,
        &mut layout.dendritic_offsets,
        &mut layout.dendritic_branches,
        &mut layout.dendritic_inputs,
        &mut layout.dendritic_source_offsets,
        &mut layout.dendritic_source_branches,
        &mut layout.decoder_offsets,
        &mut layout.decoder_ids,
    ] {
        *word = word.checked_add(meta).unwrap();
    }
    for word in [
        &mut layout.inputs,
        &mut layout.targets,
        &mut layout.target_weights,
        &mut layout.candidate_records,
        &mut layout.context,
        &mut layout.initial,
        &mut layout.replay_speech_targets,
    ] {
        *word = word.checked_add(training).unwrap();
    }
    for word in [
        &mut layout.activations,
        &mut layout.metabolic,
        &mut layout.activity_ema,
    ] {
        *word = word.checked_add(state).unwrap();
    }
    for word in [
        &mut layout.activation_gradients,
        &mut layout.metabolic_gradients,
        &mut layout.deltas,
        &mut layout.weight_gradients,
        &mut layout.adjoints,
        &mut layout.accumulated_weights,
    ] {
        *word = word.checked_add(gradient).unwrap();
    }
    for word in [
        &mut layout.candidate_logits,
        &mut layout.speech_logits,
        &mut layout.metrics,
        &mut layout.memory_raw,
        &mut layout.cognitive_raw,
    ] {
        *word = word.checked_add(output).unwrap();
    }
}

fn speech_targets(sequence: &TrainingSequence, enabled: bool) -> Vec<u32> {
    let mut words = vec![0; sequence.ticks.len() * 384];
    if enabled {
        for tick in sequence.burn_in_ticks..sequence.ticks.len() {
            for step in 0..6 {
                for output in 0..32 {
                    let at = tick * 384 + step * 64 + output * 2;
                    words[at] = (output as f32 / 128.0).to_bits();
                    words[at + 1] = 0.01_f32.to_bits();
                }
            }
        }
    }
    words
}

fn sample_scales(sequences: &[TrainingSequence; 2]) -> [f32; 2] {
    let counts = sequences
        .each_ref()
        .map(|s| s.ticks.len() - s.burn_in_ticks);
    let total = (counts[0] + counts[1]) as f32;
    counts.map(|count| count as f32 / total)
}

impl PairGpu {
    fn new(
        device: &wgpu::Device,
        phenotype: &BrainPhenotype,
        sequences: &[TrainingSequence; 2],
        original: &TrainerGpuState,
        speech: bool,
    ) -> Self {
        let scales = sample_scales(sequences);
        let packed = sequences
            .each_ref()
            .map(|s| pack_metadata_and_layout(phenotype, Some(s)).unwrap());
        let mut metadata = Vec::new();
        let mut training = Vec::new();
        let mut layouts = [packed[0].1, packed[1].1];
        let mut bases = [[0; 4]; 2];
        let mut state_words = 0;
        let mut gradient_words = 0;
        let mut output_words = 0;
        for index in 0..2 {
            let layout = packed[index].1;
            bases[index] = [
                state_words,
                gradient_words,
                output_words,
                training.len() as u64,
            ];
            let mut row = pack_replay_sequence(&sequences[index], layout, phenotype.neuron_count());
            let speech_words = speech_targets(&sequences[index], speech);
            let at = layout.replay_speech_targets as usize;
            row[at..at + speech_words.len()].copy_from_slice(&speech_words);
            shift(
                &mut layouts[index],
                metadata.len() as u32,
                training.len() as u32,
                state_words as u32,
                gradient_words as u32,
                output_words as u32,
            );
            layouts[index].gradient_scale = scales[index];
            metadata.extend_from_slice(&packed[index].0);
            training.extend(row);
            state_words += layout.state_words;
            gradient_words += layout.gradient_words;
            output_words += layout.output_words;
        }
        let limits = device.limits();
        for words in [
            state_words,
            gradient_words,
            output_words,
            training.len() as u64,
            metadata.len() as u64,
        ] {
            assert!(
                words * 4 <= limits.max_storage_buffer_binding_size,
                "two-window storage binding exceeds adapter limit"
            );
            assert!(words * 4 <= limits.max_buffer_size);
            assert!(words <= u64::from(u32::MAX));
        }
        let microsteps = u32::from(phenotype.microstep_count());
        let forward_steps = layouts.iter().map(|l| l.ticks * microsteps).max().unwrap();
        let backward_steps = layouts
            .iter()
            .map(|l| l.ticks * microsteps - l.burn_in_steps)
            .max()
            .unwrap();
        let originals = layouts.map(|layout| {
            build_step_headers(phenotype, layout, AdamWConfig::default(), (1 << 24) + 32).unwrap()
        });
        let mut headers = Vec::new();
        for header in &originals {
            headers.extend_from_slice(&header[..HEADER_WORDS]);
        }
        for step in 0..forward_steps {
            for index in 0..2 {
                let total = layouts[index].ticks * microsteps;
                let at = step.min(total - 1) as usize * HEADER_WORDS;
                let mut header = originals[index][at..at + HEADER_WORDS].to_vec();
                if step >= total {
                    header[7] = total;
                }
                headers.extend(header);
            }
        }
        let backward_header = 1 + forward_steps;
        for ordinal in 0..backward_steps {
            for index in 0..2 {
                let total = layouts[index].ticks * microsteps;
                let step = layouts[index].burn_in_steps + ordinal;
                let at = step.min(total - 1) as usize * HEADER_WORDS;
                let mut header = originals[index][at..at + HEADER_WORDS].to_vec();
                if step >= total {
                    header[7] = total;
                }
                headers.extend(header);
            }
        }
        assert!(
            PAIR_HEADER_BYTES.is_multiple_of(u64::from(limits.min_uniform_buffer_offset_alignment))
        );
        assert!((headers.len() * 4) as u64 <= limits.max_buffer_size);
        assert!((headers.len() * 4) as u64 <= u64::from(u32::MAX));
        let headers = buffer_init(
            device,
            "pair-headers",
            &headers,
            wgpu::BufferUsages::UNIFORM,
        );
        let meta = buffer_init(device, "pair-meta", &metadata, wgpu::BufferUsages::STORAGE);
        let training = buffer_init(
            device,
            "pair-training",
            &training,
            wgpu::BufferUsages::STORAGE,
        );
        let state = zero_buffer(
            device,
            "pair-state",
            state_words * 4,
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        );
        let gradients = zero_buffer(
            device,
            "pair-gradients",
            gradient_words * 4,
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        );
        let outputs = zero_buffer(
            device,
            "pair-outputs",
            output_words * 4,
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        );
        let initial_weights = phenotype
            .synapses()
            .iter()
            .map(|s| s.genetic_weight())
            .collect::<Vec<_>>();
        let weights = buffer_init(
            device,
            "pair-weights",
            &initial_weights,
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        );
        let count = initial_weights.len();
        let mut optimizer = vec![0; count * 3];
        for index in 0..count {
            optimizer[index] = if fixture::seed_age(index) == 0 {
                -0.0_f32
            } else {
                0.01_f32
            }
            .to_bits();
            optimizer[count + index] = if fixture::seed_age(index) == 0 {
                0.0_f32
            } else {
                0.02_f32
            }
            .to_bits();
            optimizer[count * 2 + index] = fixture::seed_age(index);
        }
        let optimizer = buffer_init(
            device,
            "pair-optimizer",
            &optimizer,
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        );
        let stage_mask = StageTrainableMask::from_synapse_indices(
            phenotype,
            &(0..count as u32).step_by(3).collect::<Vec<_>>(),
        )
        .unwrap();
        let mask = buffer_init(
            device,
            "pair-mask",
            stage_mask.words(),
            wgpu::BufferUsages::STORAGE,
        );
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
                    has_dynamic_offset: binding == 0,
                    min_binding_size: if binding == 0 {
                        NonZeroU64::new(PAIR_HEADER_BYTES)
                    } else {
                        None
                    },
                },
                count: None,
            })
            .collect::<Vec<_>>();
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pair-test-layout"),
            entries: &entries,
        });
        let resources = [
            &meta, &weights, &optimizer, &training, &state, &gradients, &outputs, &mask,
        ];
        let bindings = |window: Option<usize>| {
            let mut bindings = vec![wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &headers,
                    offset: window.map_or(0, |i| i as u64 * HEADER_BYTES),
                    size: NonZeroU64::new(if window.is_some() {
                        HEADER_BYTES
                    } else {
                        PAIR_HEADER_BYTES
                    }),
                }),
            }];
            bindings.extend(
                resources
                    .iter()
                    .enumerate()
                    .map(|(i, buffer)| entry(i as u32 + 1, buffer)),
            );
            bindings
        };
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pair-test-bind"),
            layout: &layout,
            entries: &bindings(None),
        });
        let views = std::array::from_fn(|index| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("pair-ordered-original-view"),
                layout: &original.pipelines.bind_group_layout,
                entries: &bindings(Some(index)),
            })
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pair-test-pipelines"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("test-only-pair-shader"),
            source: wgpu::ShaderSource::Wgsl(PAIR_WGSL.into()),
        });
        let make = |entry_point| pipeline(device, &pipeline_layout, &shader, entry_point);
        Self {
            _headers: headers,
            _meta: meta,
            weights,
            optimizer,
            _training: training,
            state,
            gradients,
            outputs,
            _mask: mask,
            bind,
            views,
            initialize: make("initialize_replay_state"),
            forward: make("forward_microstep"),
            candidates: make("forward_candidate_logits"),
            speech: make("forward_speech_logits"),
            seed: make("seed_candidate_activation_gradients"),
            seed_speech: make("seed_speech_activation_gradients"),
            local: make("backward_local"),
            sources: make("backward_recurrent_sources"),
            recurrent: make("recurrent_weight_gradients"),
            decoder: make("candidate_weight_gradients"),
            speech_gradients: make("speech_weight_gradients"),
            norm: make("reduce_gradient_norm"),
            layouts,
            bases,
            forward_steps,
            backward_steps,
            backward_header,
        }
    }

    fn dispatch(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        pipeline: &wgpu::ComputePipeline,
        block: u32,
        x: u32,
        y: u32,
    ) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(pipeline);
        pass.set_bind_group(
            0,
            &self.bind,
            &[(u64::from(block) * PAIR_HEADER_BYTES) as u32],
        );
        pass.dispatch_workgroups(x.max(1), y.max(1), 2);
    }

    fn forward(&self, encoder: &mut wgpu::CommandEncoder, phenotype: &BrainPhenotype) {
        encoder.clear_buffer(&self.state, 0, None);
        encoder.clear_buffer(&self.outputs, 0, None);
        let groups = phenotype.neuron_count().div_ceil(WORKGROUP_SIZE);
        self.dispatch(encoder, &self.initialize, 0, groups, 1);
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&self.forward);
        for step in 0..self.forward_steps {
            pass.set_bind_group(
                0,
                &self.bind,
                &[((step + 1) as u64 * PAIR_HEADER_BYTES) as u32],
            );
            pass.dispatch_workgroups(groups, 1, 2);
        }
        drop(pass);
        self.dispatch(
            encoder,
            &self.candidates,
            0,
            1,
            self.layouts.iter().map(|l| l.ticks).max().unwrap(),
        );
    }

    fn gradients(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        phenotype: &BrainPhenotype,
        adjoints: &[wgpu::Buffer; 2],
        speech: bool,
    ) {
        encoder.clear_buffer(&self.gradients, 0, None);
        for (layout, adjoint) in self.layouts.iter().zip(adjoints) {
            encoder.copy_buffer_to_buffer(
                adjoint,
                0,
                &self.gradients,
                u64::from(layout.adjoints) * 4,
                adjoint.size(),
            );
        }
        let groups = phenotype.neuron_count().div_ceil(WORKGROUP_SIZE);
        let ticks = self.layouts.iter().map(|l| l.ticks).max().unwrap();
        self.dispatch(encoder, &self.seed, 0, groups, ticks);
        if speech {
            self.dispatch(encoder, &self.speech, 0, ticks, 1);
            self.dispatch(encoder, &self.seed_speech, 0, groups, ticks);
        }
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        for ordinal in (0..self.backward_steps).rev() {
            pass.set_bind_group(
                0,
                &self.bind,
                &[((self.backward_header + ordinal) as u64 * PAIR_HEADER_BYTES) as u32],
            );
            pass.set_pipeline(&self.local);
            pass.dispatch_workgroups(groups, 1, 2);
            pass.set_pipeline(&self.sources);
            pass.dispatch_workgroups(groups, 1, 2);
        }
        drop(pass);
        let weights = (phenotype.synapses().len() as u32).div_ceil(WORKGROUP_SIZE);
        self.dispatch(encoder, &self.recurrent, 0, weights, 1);
        self.dispatch(encoder, &self.decoder, 0, weights, 1);
        if speech {
            self.dispatch(encoder, &self.speech_gradients, 0, weights, 1);
        }
        self.dispatch(encoder, &self.norm, 0, 1, 1);
    }
}

fn original_forward(
    encoder: &mut wgpu::CommandEncoder,
    gpu: &TrainerGpuState,
    phenotype: &BrainPhenotype,
) {
    encoder.clear_buffer(&gpu.state, 0, None);
    encoder.clear_buffer(&gpu.outputs, 0, None);
    let groups = phenotype.neuron_count().div_ceil(WORKGROUP_SIZE);
    dispatch(
        encoder,
        &gpu.pipelines.initialize,
        &gpu.bind_group,
        groups,
        1,
        "serial-init",
    );
    gpu.record_forward(
        encoder,
        gpu.layout.ticks * u32::from(phenotype.microstep_count()),
        groups,
    );
    dispatch(
        encoder,
        &gpu.pipelines.candidate_forward,
        &gpu.bind_group,
        1,
        gpu.layout.ticks,
        "serial-candidates",
    );
}

fn original_gradients(
    encoder: &mut wgpu::CommandEncoder,
    gpu: &TrainerGpuState,
    phenotype: &BrainPhenotype,
    adjoint: &wgpu::Buffer,
    speech: bool,
) {
    encoder.clear_buffer(&gpu.gradients, 0, None);
    encoder.copy_buffer_to_buffer(
        adjoint,
        0,
        &gpu.gradients,
        u64::from(gpu.layout.adjoints) * 4,
        adjoint.size(),
    );
    let groups = phenotype.neuron_count().div_ceil(WORKGROUP_SIZE);
    dispatch(
        encoder,
        &gpu.pipelines.seed_candidate,
        &gpu.bind_group,
        groups,
        gpu.layout.ticks,
        "serial-seed",
    );
    if speech {
        dispatch(
            encoder,
            &gpu.pipelines.speech_forward,
            &gpu.bind_group,
            gpu.layout.ticks,
            1,
            "serial-speech",
        );
        dispatch(
            encoder,
            &gpu.pipelines.seed_speech,
            &gpu.bind_group,
            groups,
            gpu.layout.ticks,
            "serial-speech-seed",
        );
    }
    gpu.record_backward(
        encoder,
        gpu.layout.ticks * u32::from(phenotype.microstep_count()),
        groups,
    );
    let weights = (phenotype.synapses().len() as u32).div_ceil(WORKGROUP_SIZE);
    dispatch(
        encoder,
        &gpu.pipelines.recurrent_gradients,
        &gpu.bind_group,
        weights,
        1,
        "serial-recurrent-gradients",
    );
    dispatch(
        encoder,
        &gpu.pipelines.candidate_gradients,
        &gpu.bind_group,
        weights,
        1,
        "serial-candidate-gradients",
    );
    if speech {
        dispatch(
            encoder,
            &gpu.pipelines.speech_gradients,
            &gpu.bind_group,
            weights,
            1,
            "serial-speech-gradients",
        );
    }
    dispatch(
        encoder,
        &gpu.pipelines.gradient_norm,
        &gpu.bind_group,
        1,
        1,
        "serial-norm",
    );
}

fn ordered_add(
    encoder: &mut wgpu::CommandEncoder,
    pipelines: &TrainerPipelines,
    view: &wgpu::BindGroup,
    gradients: &wgpu::Buffer,
    layout: PackedLayout,
    accumulated: &wgpu::Buffer,
    count: usize,
) {
    encoder.copy_buffer_to_buffer(
        accumulated,
        0,
        gradients,
        u64::from(layout.accumulated_weights) * 4,
        count as u64 * 4,
    );
    dispatch(
        encoder,
        &pipelines.validate_accumulation,
        view,
        1,
        1,
        "ordered-validate",
    );
    dispatch(
        encoder,
        &pipelines.accumulate,
        view,
        (count as u32).div_ceil(WORKGROUP_SIZE),
        1,
        "ordered-add",
    );
    encoder.copy_buffer_to_buffer(
        gradients,
        u64::from(layout.accumulated_weights) * 4,
        accumulated,
        0,
        count as u64 * 4,
    );
}

fn adam(
    encoder: &mut wgpu::CommandEncoder,
    pipelines: &TrainerPipelines,
    view: &wgpu::BindGroup,
    gradients: &wgpu::Buffer,
    layout: PackedLayout,
    accumulated: &wgpu::Buffer,
    count: usize,
) {
    encoder.copy_buffer_to_buffer(
        accumulated,
        0,
        gradients,
        u64::from(layout.weight_gradients) * 4,
        count as u64 * 4,
    );
    dispatch(
        encoder,
        &pipelines.gradient_norm,
        view,
        1,
        1,
        "effective-batch-norm",
    );
    dispatch(
        encoder,
        &pipelines.validate_adamw,
        view,
        1,
        1,
        "effective-batch-preflight",
    );
    dispatch(
        encoder,
        &pipelines.adamw,
        view,
        (count as u32).div_ceil(WORKGROUP_SIZE),
        1,
        "effective-batch-adam",
    );
}

fn compact_read(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    source: &wgpu::Buffer,
    ranges: &[(u64, u64)],
) -> Vec<u32> {
    let bytes: u64 = ranges.iter().map(|(_, bytes)| bytes).sum();
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("pair-compact-readback"),
        size: bytes,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let mut to = 0;
    for &(offset, bytes) in ranges {
        encoder.copy_buffer_to_buffer(source, offset, &staging, to, bytes);
        to += bytes;
    }
    let commands = encoder.finish();
    let (send, receive) = mpsc::channel();
    commands.map_buffer_on_submit(&staging, wgpu::MapMode::Read, .., move |result| {
        let _ = send.send(result);
    });
    let submission = queue.submit(Some(commands));
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .unwrap();
    receive.recv().unwrap().unwrap();
    let mapped = staging.slice(..).get_mapped_range();
    let words = bytemuck::cast_slice(&mapped).to_vec();
    drop(mapped);
    staging.unmap();
    words
}

fn run_serial(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    gpus: &[TrainerGpuState; 2],
    phenotype: &BrainPhenotype,
    adjoints: &[wgpu::Buffer; 2],
    accumulated: &wgpu::Buffer,
    speech: bool,
) {
    let mut clear = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    clear.clear_buffer(accumulated, 0, None);
    queue.submit(Some(clear.finish()));
    for index in 0..2 {
        let gpu = &gpus[index];
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        original_forward(&mut encoder, gpu, phenotype);
        queue.submit(Some(encoder.finish()));
        // Forward availability boundary only: the full PPO objective is absent
        // from this fixed-adjoint fixture. This copies actual neural logits.
        compact_read(
            device,
            queue,
            &gpu.outputs,
            &[(u64::from(gpu.layout.candidate_logits) * 4, 8)],
        );
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        original_gradients(&mut encoder, gpu, phenotype, &adjoints[index], speech);
        ordered_add(
            &mut encoder,
            &gpu.pipelines,
            &gpu.bind_group,
            &gpu.gradients,
            gpu.layout,
            accumulated,
            phenotype.synapses().len(),
        );
        queue.submit(Some(encoder.finish()));
        let receipt = compact_read(
            device,
            queue,
            &gpu.outputs,
            &[(u64::from(gpu.layout.metrics + 2) * 4, 8)],
        );
        assert!(f32::from_bits(receipt[0]).is_finite());
        assert_eq!(f32::from_bits(receipt[1]), 1.0);
    }
    let gpu = &gpus[0];
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    adam(
        &mut encoder,
        &gpu.pipelines,
        &gpu.bind_group,
        &gpu.gradients,
        gpu.layout,
        accumulated,
        phenotype.synapses().len(),
    );
    queue.submit(Some(encoder.finish()));
    let receipt = compact_read(
        device,
        queue,
        &gpu.outputs,
        &[(u64::from(gpu.layout.metrics + 2) * 4, 8)],
    );
    assert_eq!(f32::from_bits(receipt[1]), 1.0);
}

#[allow(clippy::too_many_arguments)]
fn run_pair(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pair: &PairGpu,
    original: &TrainerGpuState,
    phenotype: &BrainPhenotype,
    adjoints: &[wgpu::Buffer; 2],
    accumulated: &wgpu::Buffer,
    speech: bool,
) {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.clear_buffer(accumulated, 0, None);
    pair.forward(&mut encoder, phenotype);
    queue.submit(Some(encoder.finish()));
    compact_read(
        device,
        queue,
        &pair.outputs,
        &pair.layouts.map(|l| (u64::from(l.candidate_logits) * 4, 8)),
    );
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    pair.gradients(&mut encoder, phenotype, adjoints, speech);
    for index in 0..2 {
        ordered_add(
            &mut encoder,
            &original.pipelines,
            &pair.views[index],
            &pair.gradients,
            pair.layouts[index],
            accumulated,
            phenotype.synapses().len(),
        );
    }
    queue.submit(Some(encoder.finish()));
    let receipt = compact_read(
        device,
        queue,
        &pair.outputs,
        &pair.layouts.map(|l| (u64::from(l.metrics + 2) * 4, 8)),
    );
    for row in receipt.as_chunks::<2>().0 {
        assert!(f32::from_bits(row[0]).is_finite());
        assert_eq!(f32::from_bits(row[1]), 1.0);
    }
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    adam(
        &mut encoder,
        &original.pipelines,
        &pair.views[0],
        &pair.gradients,
        pair.layouts[0],
        accumulated,
        phenotype.synapses().len(),
    );
    queue.submit(Some(encoder.finish()));
    let receipt = compact_read(
        device,
        queue,
        &pair.outputs,
        &[(u64::from(pair.layouts[0].metrics + 2) * 4, 8)],
    );
    assert_eq!(f32::from_bits(receipt[1]), 1.0);
}

struct Case {
    serial: [TrainerGpuState; 2],
    pair: PairGpu,
    adjoints: [wgpu::Buffer; 2],
    serial_accumulated: wgpu::Buffer,
    pair_accumulated: wgpu::Buffer,
}

impl Case {
    fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        phenotype: &BrainPhenotype,
        shapes: [(usize, usize); 2],
        speech: bool,
    ) -> Self {
        let mut sequences = shapes.map(|(ticks, burn)| fixture::replay(phenotype, ticks, burn));
        if shapes.iter().any(|(ticks, _)| *ticks >= 256) {
            // Representative throughput fixture: four active microsteps per
            // tick. Small correctness cases retain partial/inactive ticks.
            for sequence in &mut sequences {
                for tick in &mut sequence.ticks {
                    tick.microstep_count = u32::from(phenotype.microstep_count());
                }
            }
        }
        // Distinct valid recurrent boundaries and local plastic context ensure
        // this cannot pass by accidentally executing the first slot twice.
        sequences[1].initial.activations[1] = -0.25;
        sequences[1].ticks[0].effective_weight_offsets[0] = -0.003;
        let scales = sample_scales(&sequences);
        let serial = sequences
            .each_ref()
            .map(|s| fixture::gpu(device, queue, phenotype, s));
        for index in 0..2 {
            let mut layout = serial[index].layout;
            layout.gradient_scale = scales[index];
            let headers =
                build_step_headers(phenotype, layout, AdamWConfig::default(), (1 << 24) + 32)
                    .unwrap();
            queue.write_buffer(
                &serial[index].step_headers,
                0,
                bytemuck::cast_slice(&headers),
            );
            let words = speech_targets(&sequences[index], speech);
            queue.write_buffer(
                &serial[index].training,
                u64::from(layout.replay_speech_targets) * 4,
                bytemuck::cast_slice(&words),
            );
        }
        let pair = PairGpu::new(device, phenotype, &sequences, &serial[0], speech);
        let adjoints = std::array::from_fn(|index| {
            let s = &sequences[index];
            let mut values = vec![0.0_f32; s.ticks.len() * alife_core::MAX_ACTION_CANDIDATES];
            for tick in s.burn_in_ticks..s.ticks.len() {
                values[tick * alife_core::MAX_ACTION_CANDIDATES] = 0.125 / (index + 1) as f32;
                values[tick * alife_core::MAX_ACTION_CANDIDATES + 1] = -0.25 / (index + 1) as f32;
            }
            buffer_init(
                device,
                "pair-fixture-fixed-adjoints",
                &values,
                wgpu::BufferUsages::COPY_SRC,
            )
        });
        let make_accum = || {
            zero_buffer(
                device,
                "pair-ordered-accumulation",
                phenotype.synapses().len() as u64 * 4,
                wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            )
        };
        let case = Self {
            serial,
            pair,
            adjoints,
            serial_accumulated: make_accum(),
            pair_accumulated: make_accum(),
        };
        // queue.write_buffer copies become submitted only at the next submit.
        // Flush every setup upload before either side's measured interval.
        let setup = queue.submit(std::iter::empty::<wgpu::CommandBuffer>());
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(setup),
                timeout: None,
            })
            .unwrap();
        case
    }

    fn check(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        for (label, actual, expected) in [
            ("weights", &self.pair.weights, &self.serial[0].weights),
            (
                "optimizer",
                &self.pair.optimizer,
                &self.serial[0]._optimizer,
            ),
            (
                "ordered accumulated gradients",
                &self.pair_accumulated,
                &self.serial_accumulated,
            ),
        ] {
            assert_eq!(
                fixture::read(device, queue, actual),
                fixture::read(device, queue, expected),
                "{label}"
            );
        }
        for (which, actual, column) in [
            ("state", &self.pair.state, 0),
            ("gradients", &self.pair.gradients, 1),
            ("outputs", &self.pair.outputs, 2),
        ] {
            let all = fixture::read(device, queue, actual);
            for index in 0..2 {
                let expected = match column {
                    0 => &self.serial[index].state,
                    1 => &self.serial[index].gradients,
                    _ => &self.serial[index].outputs,
                };
                let expected = fixture::read(device, queue, expected);
                let at = self.pair.bases[index][column] as usize;
                let found = &all[at..at + expected.len()];
                assert_eq!(
                    found.iter().zip(&expected).position(|(a, b)| a != b),
                    None,
                    "{which}, window={index}"
                );
            }
            let optimizer = fixture::read(device, queue, &self.pair.optimizer);
            let count = optimizer.len() / 3;
            for index in 0..count {
                assert_eq!(
                    optimizer[count * 2 + index],
                    fixture::seed_age(index) + u32::from(index % 3 == 0)
                );
                if index % 3 != 0 {
                    assert_eq!(
                        optimizer[index],
                        if fixture::seed_age(index) == 0 {
                            -0.0_f32
                        } else {
                            0.01_f32
                        }
                        .to_bits()
                    );
                    assert_eq!(
                        optimizer[count + index],
                        if fixture::seed_age(index) == 0 {
                            0.0_f32
                        } else {
                            0.02_f32
                        }
                        .to_bits()
                    );
                }
            }
        }
    }
}

#[test]
#[ignore = "test-only direct Vulkan device; accepts software, never a trained checkpoint"]
fn paired_ragged_windows_match_serial_ordered_gradients_and_adam() {
    let (device, queue) = fixture::device();
    let phenotype = fixture::phenotype();
    for (shapes, speech) in [
        ([(3, 0), (5, 2)], false),
        ([(3, 1), (5, 2)], false),
        ([(3, 0), (5, 2)], true),
    ] {
        let case = Case::new(&device, &queue, &phenotype, shapes, speech);
        run_serial(
            &device,
            &queue,
            &case.serial,
            &phenotype,
            &case.adjoints,
            &case.serial_accumulated,
            speech,
        );
        run_pair(
            &device,
            &queue,
            &case.pair,
            &case.serial[0],
            &phenotype,
            &case.adjoints,
            &case.pair_accumulated,
            speech,
        );
        case.check(&device, &queue);
        eprintln!("pair exactness passed: shapes={shapes:?}, speech={speech}");
    }
}

#[test]
#[ignore = "test-only Vulkan fault injection; no production GPU admission"]
fn paired_invalid_second_gradient_cannot_apply_adam() {
    let (device, queue) = fixture::device();
    let phenotype = fixture::phenotype();
    let mut case = Case::new(&device, &queue, &phenotype, [(3, 0), (5, 2)], false);
    let before_weights = fixture::read(&device, &queue, &case.pair.weights);
    let before_optimizer = fixture::read(&device, &queue, &case.pair.optimizer);
    let invalid = vec![f32::NAN; 5 * alife_core::MAX_ACTION_CANDIDATES];
    case.adjoints[1] = buffer_init(
        &device,
        "invalid-second-window-adjoints",
        &invalid,
        wgpu::BufferUsages::COPY_SRC,
    );
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_pair(
            &device,
            &queue,
            &case.pair,
            &case.serial[0],
            &phenotype,
            &case.adjoints,
            &case.pair_accumulated,
            false,
        );
    }));
    assert!(
        result.is_err(),
        "the invalid ordered accumulation must reject before Adam"
    );
    assert_eq!(
        fixture::read(&device, &queue, &case.pair.weights),
        before_weights
    );
    assert_eq!(
        fixture::read(&device, &queue, &case.pair.optimizer),
        before_optimizer
    );
}

#[test]
#[ignore = "complete unprofiled fixed-adjoint actor assay; PPO objective/KL/value not integrated"]
fn paired_ragged_windows_complete_actor_assay() {
    let (device, queue) = fixture::device();
    let phenotype = fixture::phenotype();
    let shapes = [(256, 0), (384, 128)];
    let trials = std::env::var("ALIFE_PAIR_TRIALS")
        .ok()
        .map(|v| v.parse::<usize>().unwrap())
        .unwrap_or(3);
    assert!((1..=7).contains(&trials));
    for trial in 0..trials {
        let case = Case::new(&device, &queue, &phenotype, shapes, false);
        // Setup/uploads excluded for both sides; everything from command
        // encoding through final compact Adam receipt is included, no timestamps.
        let mut seconds = [0.0; 2];
        for pair in if trial % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        } {
            let started = Instant::now();
            if pair {
                run_pair(
                    &device,
                    &queue,
                    &case.pair,
                    &case.serial[0],
                    &phenotype,
                    &case.adjoints,
                    &case.pair_accumulated,
                    false,
                );
            } else {
                run_serial(
                    &device,
                    &queue,
                    &case.serial,
                    &phenotype,
                    &case.adjoints,
                    &case.serial_accumulated,
                    false,
                );
            }
            seconds[usize::from(pair)] = started.elapsed().as_secs_f64();
        }
        case.check(&device, &queue);
        eprintln!(
            "PAIR_ACTOR_TRIAL {}",
            serde_json::json!({"trial":trial,"shapes":shapes,"speech":false,"active_microsteps_per_tick":phenotype.microstep_count(),"serial_seconds":seconds[0],"pair_seconds":seconds[1],"ratio":seconds[0]/seconds[1],"scope":"fixed-adjoint actor encode+submit+map/wait+ordered accumulation+Adam; setup/uploads/full oracle reads excluded; full PPO objective/KL/value/rollback absent","serial_submissions":11,"pair_submissions":6,"serial_waits":5,"pair_waits":3,"exact_bits":true})
        );
    }
}
