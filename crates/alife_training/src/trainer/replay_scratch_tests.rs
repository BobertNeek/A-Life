//! Actual device preparation versus the original fresh-allocation shape path.
use super::*;

fn mask(phenotype: &BrainPhenotype) -> StageTrainableMask {
    StageTrainableMask::from_synapse_indices(
        phenotype,
        &(0..phenotype.synapses().len() as u32)
            .step_by(3)
            .collect::<Vec<_>>(),
    )
    .unwrap()
}

fn fresh_prepare(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    gpu: &mut TrainerGpuState,
    phenotype: &BrainPhenotype,
    replay: &TrainingSequence,
    optimizer_step: u32,
) {
    // This reference deliberately always takes the original changed-shape path:
    // fresh storage, complete packet/header upload and exact weights/Adam copy.
    replay.validate_for(phenotype).unwrap();
    let zeros = vec![0_f32; phenotype.synapses().len()];
    let next = TrainerGpuState::new_for_replay(
        device,
        phenotype,
        &zeros,
        &mask(phenotype),
        Some(replay),
        Some(&gpu.pipelines),
    )
    .unwrap();
    let words = pack_replay_sequence(replay, next.layout, phenotype.neuron_count());
    let headers = build_step_headers(
        phenotype,
        next.layout,
        AdamWConfig::default(),
        optimizer_step + 1,
    )
    .unwrap();
    queue.write_buffer(&next.training, 0, bytemuck::cast_slice(&words));
    queue.write_buffer(&next.step_headers, 0, bytemuck::cast_slice(&headers));
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(&gpu.weights, 0, &next.weights, 0, zeros.len() as u64 * 4);
    encoder.copy_buffer_to_buffer(
        &gpu._optimizer,
        0,
        &next._optimizer,
        0,
        zeros.len() as u64 * 12,
    );
    queue.submit(Some(encoder.finish()));
    *gpu = next;
}

fn sequence(
    phenotype: &BrainPhenotype,
    ticks: usize,
    burn_in: usize,
    variant: usize,
) -> TrainingSequence {
    let mut value = replay(phenotype, ticks, burn_in);
    value.initial.dendrites = if variant == 0 {
        DendriticBranchSet::default()
    } else {
        DendriticBranchSet::new(
            (0..if variant == 3 { 3 } else { 1 })
                .map(|branch| {
                    DendriticBranch::new(
                        4 + branch,
                        if variant == 2 { -0.0 } else { 0.01 },
                        0.2,
                        vec![
                            DendriticInputRef::new(
                                1 + branch,
                                if variant == 2 { -0.0 } else { 0.125 },
                            )
                            .unwrap(),
                            DendriticInputRef::new(3, -0.25).unwrap(),
                        ],
                    )
                    .unwrap()
                })
                .collect(),
        )
        .unwrap()
    };
    for (index, tick) in value.ticks.iter_mut().enumerate() {
        if index % 2 == 0 {
            tick.structural_synapses.clear();
        }
        if index % 3 == 0 {
            tick.enabled_routes[0] = false;
        }
        if index % 2 != 0 {
            tick.candidates.truncate(1);
        }
    }
    value.validate_for(phenotype).unwrap();
    value
}

fn speech(queue: &wgpu::Queue, gpu: &TrainerGpuState, sequence: &TrainingSequence, mode: usize) {
    let mut words = vec![0_u32; sequence.ticks.len() * 384];
    for tick in sequence.burn_in_ticks..sequence.ticks.len() {
        if mode == 0 || (mode == 1 && tick % 3 != 0) {
            continue;
        }
        for step in 0..6 {
            for output in 0..32 {
                let at = tick * 384 + step * 64 + output * 2;
                words[at] = (output as f32 / 128.0).to_bits();
                words[at + 1] = 0.01_f32.to_bits();
            }
        }
    }
    queue.write_buffer(
        &gpu.training,
        u64::from(gpu.layout.replay_speech_targets) * 4,
        bytemuck::cast_slice(&words),
    );
}

// Diagnostic readback through the resource's real usage flags. Metadata,
// packets, masks and headers intentionally do not need COPY_SRC in production.
fn read_any(device: &wgpu::Device, queue: &wgpu::Queue, buffer: &wgpu::Buffer) -> Vec<u32> {
    if buffer.usage().contains(wgpu::BufferUsages::COPY_SRC) {
        return read(device, queue, buffer);
    }
    let uniform = buffer.usage().contains(wgpu::BufferUsages::UNIFORM);
    let (declaration, expression) = if uniform {
        assert!(buffer.size() <= u64::from(device.limits().max_uniform_buffer_binding_size));
        (format!("struct Source {{ words:array<vec4<u32>,{}>, }} @group(0) @binding(0) var<uniform> source:Source;", buffer.size()/16),
         "source.words[i/4u][i%4u]")
    } else {
        (
            "@group(0) @binding(0) var<storage,read> source:array<u32>;".into(),
            "source[i]",
        )
    };
    let code = format!("{declaration} @group(0) @binding(1) var<storage,read_write> output:array<u32>; @compute @workgroup_size(64) fn copy(@builtin(global_invocation_id) gid:vec3<u32>) {{ let i=gid.x; if (i<{}u) {{ output[i]={expression}; }} }}", buffer.size()/4);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("scratch-test-read-only-probe"),
        source: wgpu::ShaderSource::Wgsl(code.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: None,
        module: &shader,
        entry_point: Some("copy"),
        compilation_options: Default::default(),
        cache: None,
    });
    let staging = zero_buffer(
        device,
        "scratch-test-probe-output",
        buffer.size(),
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    );
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[entry(0, buffer), entry(1, &staging)],
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind, &[]);
        pass.dispatch_workgroups((buffer.size() / 4).div_ceil(64) as u32, 1, 1);
    }
    queue.submit(Some(encoder.finish()));
    read(device, queue, &staging)
}

fn assert_prefix(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    actual: &wgpu::Buffer,
    expected: &wgpu::Buffer,
    words: u64,
    label: &str,
) {
    let actual = read_any(device, queue, actual);
    let expected = read_any(device, queue, expected);
    let words = words as usize;
    assert_eq!(
        actual[..words]
            .iter()
            .zip(&expected[..words])
            .position(|(a, b)| a != b),
        None,
        "{label}: first differing logical word"
    );
}

#[test]
#[ignore = "explicit direct Vulkan device; software is admitted only by this synthetic fixture"]
fn reused_scratch_matches_fresh_shapes_and_preserves_failed_preparation() {
    let (device, queue) = device();
    let phenotype = phenotype();
    let mask = mask(&phenotype);
    let weights = phenotype
        .synapses()
        .iter()
        .map(|s| s.genetic_weight())
        .collect::<Vec<_>>();
    let mut candidate = TrainerGpuState::new(&device, &phenotype, &weights, &mask).unwrap();
    let mut baseline = TrainerGpuState::new(&device, &phenotype, &weights, &mask).unwrap();
    // Preserve nonzero moments and exact heterogeneous integer ages through
    // first replay preparation, growth, shrink, metadata changes and updates.
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
    for gpu in [&candidate, &baseline] {
        queue.write_buffer(&gpu._optimizer, 0, bytemuck::cast_slice(&optimizer));
    }
    let mut shape = None;
    let mut step = (1 << 24) + 32;
    let cases = [
        (1, 0, 0),
        (9, 2, 1),
        (3, 1, 3),
        (9, 0, 2),
        (9, 5, 2),
        (5, 4, 2),
        (2, 0, 0),
        (13, 3, 1),
        (3, 0, 3),
        (13, 4, 3),
    ];
    let mut reused = 0;
    let mut metadata_only = 0;
    for (index, (ticks, burn_in, variant)) in cases.into_iter().enumerate() {
        let sequence = sequence(&phenotype, ticks, burn_in, variant);
        let prior_training = candidate.training.clone();
        let prior_meta = candidate._meta.clone();
        shape = Some(
            candidate
                .prepare_replay(
                    &device,
                    &queue,
                    &phenotype,
                    &mask,
                    &sequence,
                    shape.as_ref(),
                    AdamWConfig::default(),
                    step,
                )
                .unwrap(),
        );
        fresh_prepare(&device, &queue, &mut baseline, &phenotype, &sequence, step);
        if candidate.training == prior_training {
            reused += 1;
            metadata_only += usize::from(candidate._meta != prior_meta);
        }
        speech(&queue, &candidate, &sequence, index % 3);
        speech(&queue, &baseline, &sequence, index % 3);
        assert_eq!(
            format!("{:?}", candidate.layout),
            format!("{:?}", baseline.layout)
        );
        let (meta, _) = pack_metadata_and_layout(&phenotype, Some(&sequence)).unwrap();
        assert_prefix(
            &device,
            &queue,
            &candidate._meta,
            &baseline._meta,
            meta.len() as u64,
            "metadata",
        );
        assert_prefix(
            &device,
            &queue,
            &candidate.training,
            &baseline.training,
            candidate.layout.training_words,
            "upload",
        );
        let before_weights = read(&device, &queue, &candidate.weights);
        queue.submit([
            encode_update(&device, &baseline, &phenotype, None),
            encode_update(&device, &candidate, &phenotype, None),
        ]);
        for (label, actual, expected, words) in [
            (
                "state",
                &candidate.state,
                &baseline.state,
                candidate.layout.state_words,
            ),
            (
                "outputs",
                &candidate.outputs,
                &baseline.outputs,
                candidate.layout.output_words,
            ),
            (
                "gradients",
                &candidate.gradients,
                &baseline.gradients,
                candidate.layout.gradient_words,
            ),
            ("weights", &candidate.weights, &baseline.weights, n as u64),
            (
                "optimizer",
                &candidate._optimizer,
                &baseline._optimizer,
                n as u64 * 3,
            ),
            ("mask", &candidate.mask, &baseline.mask, n as u64),
        ] {
            assert_prefix(&device, &queue, actual, expected, words, label);
        }
        let output = read(&device, &queue, &candidate.outputs);
        assert_eq!(
            f32::from_bits(output[candidate.layout.metrics as usize + 3]),
            1.0,
            "Adam must accept each update"
        );
        let norm = f32::from_bits(output[candidate.layout.metrics as usize + 2]);
        assert!(norm.is_finite() && norm > 0.0);
        let actual_optimizer = read(&device, &queue, &candidate._optimizer);
        let after_weights = read(&device, &queue, &candidate.weights);
        let changed_trainable = (0..n)
            .filter(|coordinate| {
                coordinate % 3 == 0 && before_weights[*coordinate] != after_weights[*coordinate]
            })
            .count();
        assert!(
            changed_trainable > 0,
            "an accepted update must change at least one trainable weight"
        );
        for coordinate in 0..n {
            assert_eq!(
                actual_optimizer[n * 2 + coordinate],
                seed_age(coordinate)
                    + if coordinate % 3 == 0 {
                        index as u32 + 1
                    } else {
                        0
                    }
            );
            if coordinate % 3 != 0 {
                assert_eq!(after_weights[coordinate], before_weights[coordinate]);
                assert_eq!(actual_optimizer[coordinate], optimizer[coordinate]);
                assert_eq!(actual_optimizer[n + coordinate], optimizer[n + coordinate]);
            }
        }
        step += 1;
        eprintln!("scratch exact accepted device update case {index}: ticks={ticks}, burn={burn_in}, dendrite_variant={variant}, changed_trainable_weights={changed_trainable}");
    }
    assert!(
        reused >= 6,
        "expected bounded reuse across shorter logical shapes"
    );
    assert!(
        metadata_only >= 1,
        "growing dendrites should replace metadata alone"
    );
    let prepared = sequence(&phenotype, 3, 0, 3);
    let ids = [
        candidate.training.clone(),
        candidate.state.clone(),
        candidate.outputs.clone(),
        candidate.gradients.clone(),
        candidate._meta.clone(),
        candidate.step_headers.clone(),
        candidate.weights.clone(),
        candidate._optimizer.clone(),
    ];
    let before = ids
        .iter()
        .map(|buffer| read_any(&device, &queue, buffer))
        .collect::<Vec<_>>();
    let before_layout = format!("{:?}", candidate.layout);
    for invalid in 0..3 {
        let mut sequence = prepared.clone();
        let mut age = step;
        match invalid {
            0 => sequence.ticks[0].encoded_inputs[0] = f32::NAN,
            1 => sequence.burn_in_ticks = sequence.ticks.len(),
            _ => age = u32::MAX,
        }
        assert!(candidate
            .prepare_replay(
                &device,
                &queue,
                &phenotype,
                &mask,
                &sequence,
                shape.as_ref(),
                AdamWConfig::default(),
                age
            )
            .is_err());
        assert_eq!(format!("{:?}", candidate.layout), before_layout);
        for (buffer, words) in ids.iter().zip(&before) {
            assert_eq!(&read_any(&device, &queue, buffer), words);
        }
        assert!(candidate.training == ids[0] && candidate._meta == ids[4]);
    }
    // A cohort invalidation deliberately drops the old shape identity even if
    // its backing buffers remain large enough for a fresh validated replay.
    candidate
        .prepare_replay(
            &device,
            &queue,
            &phenotype,
            &mask,
            &prepared,
            None,
            AdamWConfig::default(),
            step,
        )
        .unwrap();
    eprintln!("scratch reuse cases={reused}, metadata-only growth cases={metadata_only}; invalid preparations preserved bytes and layout");
}

fn capacities(gpu: &TrainerGpuState) -> serde_json::Value {
    let fields = [
        ("training", gpu.training.size()),
        ("state", gpu.state.size()),
        ("gradients", gpu.gradients.size()),
        ("outputs", gpu.outputs.size()),
        ("headers", gpu.step_headers.size()),
        ("metadata", gpu._meta.size()),
        ("weights", gpu.weights.size()),
        ("optimizer", gpu._optimizer.size()),
        ("mask", gpu.mask.size()),
        ("readback", gpu.readback.size()),
    ];
    let mut object = serde_json::Map::new();
    object.insert(
        "total".into(),
        fields.iter().map(|(_, bytes)| bytes).sum::<u64>().into(),
    );
    for (name, bytes) in fields {
        object.insert(name.into(), bytes.into());
    }
    object.into()
}

fn wait_submission(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    command: Option<wgpu::CommandBuffer>,
) {
    let submission = queue.submit(command);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .unwrap();
}

#[test]
#[ignore = "bounded explicit release preparation/forward/BPTT/Adam assay; software device is not training throughput"]
fn replay_scratch_capacity_assay() {
    assert!(
        !cfg!(debug_assertions),
        "run this matched assay with cargo test --release"
    );
    let output = std::env::var_os("ALIFE_REPLAY_SCRATCH_ASSAY_OUTPUT")
        .expect("explicit writable assay output required");
    let (device, queue) = device();
    let phenotype = phenotype();
    let mask = mask(&phenotype);
    let mut sequences = [
        sequence(&phenotype, 256, 0, 1),
        sequence(&phenotype, 384, 128, 1),
        sequence(&phenotype, 256, 0, 1),
    ];
    for sequence in &mut sequences {
        for tick in &mut sequence.ticks {
            tick.microstep_count = u32::from(phenotype.microstep_count());
        }
    }
    let initial = &sequences[1];
    let fresh_bounds = sequences
        .iter()
        .map(|sequence| FoundationTrainer::replay_allocation_bytes(&phenotype, sequence).unwrap())
        .collect::<Vec<_>>();
    let mut trials = Vec::new();
    for full_update in [false, true] {
        // Two warm-up pairs precede seven alternating timed pairs. Preparation
        // includes validation/packing/uploads/allocation plus queue submit/wait.
        // The second scope also runs actual frozen graph/BPTT/Adam, no PPO head.
        for trial in 0_usize..9 {
            let mut baseline = gpu(&device, &queue, &phenotype, initial);
            let mut candidate = gpu(&device, &queue, &phenotype, initial);
            wait_submission(&device, &queue, None);
            let order = if trial % 2 == 0 {
                [false, true]
            } else {
                [true, false]
            };
            for reuse in order {
                let gpu = if reuse { &mut candidate } else { &mut baseline };
                let mut shape = Some((initial.ticks.len(), initial.initial.dendrites.clone()));
                let mut step = (1 << 24) + 32;
                let starting_capacity = capacities(gpu);
                let mut rows = Vec::new();
                let started = Instant::now();
                let mut measured_seconds = 0.0;
                for (sequence_index, sequence) in sequences.iter().enumerate() {
                    let before = capacities(gpu);
                    let clock = Instant::now();
                    if reuse {
                        shape = Some(
                            gpu.prepare_replay(
                                &device,
                                &queue,
                                &phenotype,
                                &mask,
                                sequence,
                                shape.as_ref(),
                                AdamWConfig::default(),
                                step,
                            )
                            .unwrap(),
                        );
                    } else {
                        fresh_prepare(&device, &queue, gpu, &phenotype, sequence, step);
                    }
                    let command = if full_update {
                        Some(encode_update_with_speech(
                            &device, gpu, &phenotype, None, false,
                        ))
                    } else {
                        None
                    };
                    wait_submission(&device, &queue, command);
                    let seconds = clock.elapsed().as_secs_f64();
                    measured_seconds += seconds;
                    rows.push(serde_json::json!({"ticks":sequence.ticks.len(),"burn_in":sequence.burn_in_ticks,
                        "seconds":seconds,"before_capacity_bytes":before,"after_capacity_bytes":capacities(gpu),
                        "fresh_logical_allocation_bound_bytes":fresh_bounds[sequence_index]}));
                    step += u32::from(full_update);
                }
                let sequence_wall_seconds = started.elapsed().as_secs_f64();
                trials.push(serde_json::json!({"scope":if full_update {"prepare_forward_BPTT_Adam_submit_wait"} else {"prepare_submit_wait"},
                        "mode":if reuse {"capacity_reuse"} else {"fresh_shape_allocation"},
                        "phase":if trial<2 {"warmup"} else {"measured"},"pair_index":trial,"trial":trial.saturating_sub(2),
                        "seconds":measured_seconds,"sequence_wall_seconds_including_receipt_bookkeeping":sequence_wall_seconds,
                        "starting_capacity_bytes":starting_capacity,"rows":rows}));
            }
            // Exactness checks and destruction are outside the timed scopes.
            for (label, actual, expected, words) in [
                (
                    "weights",
                    &candidate.weights,
                    &baseline.weights,
                    phenotype.synapses().len() as u64,
                ),
                (
                    "optimizer",
                    &candidate._optimizer,
                    &baseline._optimizer,
                    phenotype.synapses().len() as u64 * 3,
                ),
            ] {
                assert_prefix(&device, &queue, actual, expected, words, label);
            }
            if full_update {
                for (label, actual, expected, words) in [
                    (
                        "state",
                        &candidate.state,
                        &baseline.state,
                        candidate.layout.state_words,
                    ),
                    (
                        "outputs",
                        &candidate.outputs,
                        &baseline.outputs,
                        candidate.layout.output_words,
                    ),
                    (
                        "gradients",
                        &candidate.gradients,
                        &baseline.gradients,
                        candidate.layout.gradient_words,
                    ),
                ] {
                    assert_prefix(&device, &queue, actual, expected, words, label);
                }
                let status = read(&device, &queue, &candidate.outputs);
                assert_eq!(
                    f32::from_bits(status[candidate.layout.metrics as usize + 3]),
                    1.0
                );
                let trained_weights = read(&device, &queue, &candidate.weights);
                assert!(trained_weights
                    .iter()
                    .zip(phenotype.synapses())
                    .enumerate()
                    .any(|(index, (word, synapse))| index % 3 == 0
                        && *word != synapse.genetic_weight().to_bits()));
                let ages = read(&device, &queue, &candidate._optimizer);
                for index in 0..phenotype.synapses().len() {
                    assert_eq!(
                        ages[phenotype.synapses().len() * 2 + index],
                        seed_age(index) + if index % 3 == 0 { 3 } else { 0 }
                    );
                }
            }
            eprintln!(
                "scratch assay scope={} pair={trial} exact",
                if full_update {
                    "prepare+graph+BPTT+Adam"
                } else {
                    "prepare"
                }
            );
        }
    }
    let receipt = serde_json::json!({"schema":1,"kind":"synthetic-replay-scratch-capacity-assay",
        "debug_assertions":cfg!(debug_assertions),"neuron_count":phenotype.neuron_count(),
        "synapse_count":phenotype.synapses().len(),"route_count":phenotype.projections().len(),
        "microsteps":phenotype.microstep_count(),"active_microstep_pattern":"all four microsteps active on every tick",
        "max_candidate_count":2,"candidate_count_pattern":"two on even ticks; one on odd ticks",
        "route_pattern":"all routes enabled except route zero disabled every third tick",
        "structural_pattern":"no edge on even ticks; one source1-target2 route0 edge on odd ticks",
        "dendritic_branches":1,"dendritic_inputs":2,
        "warmup_pairs_per_scope":2,"timed_pairs_per_scope":7,"trials":trials,
        "speech_objective_enabled":false,"value_or_PPO_objective_executed":false,
        "scope":"actual production preparation and submitted/waited frozen forward/BPTT/Adam only; excludes world collection, PPO objective/value state, replay IO, checkpoint admission/persistence and temporary correctness readbacks",
        "resource_byte_scope":"requested Buffer.size backing capacities, not physical VRAM; one current set per trainer; baseline/reference coexist only in this fixture",
        "timing_scope":"seconds is sum of three individually timed prepare/submit/wait spans; per-row receipt construction, capacity sampling, precomputed allocation estimates, correctness readbacks and destruction are outside those spans; inclusive sequence wall is retained separately"});
    std::fs::write(output, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
}
