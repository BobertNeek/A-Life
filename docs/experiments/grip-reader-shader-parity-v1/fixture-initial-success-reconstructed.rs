//! Fixed synthetic ABI fixtures, not learned behavior or hardware qualification.
//! Runs the unchanged production memory and decoder-eligibility entry points on
//! CPU Vulkan. Selection is explicitly seeded; this is not a runtime admission,
//! recurrent-network, memory retrieval, or outcome/publication test.
use super::*;
use alife_core::*;
use alife_gpu_backend::*;
use bytemuck::Zeroable;
use wgpu::util::DeviceExt;

struct Device {
    device: wgpu::Device,
    queue: wgpu::Queue,
    layout: wgpu::BindGroupLayout,
    memory: wgpu::ComputePipeline,
    eligibility: wgpu::ComputePipeline,
}
impl Device {
    fn new() -> Self {
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        desc.backends = wgpu::Backends::VULKAN;
        let instance = wgpu::Instance::new(desc);
        let adapter = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::VULKAN))
            .into_iter()
            .find(|a| a.get_info().device_type == wgpu::DeviceType::Cpu)
            .expect("explicit CPU Vulkan required; no hardware/Noop fallback");
        let info = adapter.get_info();
        assert_eq!(info.backend, wgpu::Backend::Vulkan);
        eprintln!("grip_reader_adapter={info:?}; synthetic=true; hardware_qualification=false");
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("synthetic-grip-reader-parity"),
            required_limits: adapter.limits(),
            ..Default::default()
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
            label: Some("unchanged-seven-buffer-ABI"),
            entries: &entries,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |source: &'static str, entry: &'static str| {
            // Compile the exported production bytes, without test shader rewrites.
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(entry),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                module: &module,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let memory = pipeline(
            CLOSED_LOOP_MEMORY_CONTEXT_WGSL,
            "add_candidate_memory_context",
        );
        let eligibility = pipeline(
            CLOSED_LOOP_ELIGIBILITY_WGSL,
            "accumulate_decoder_eligibility",
        );
        Self {
            device,
            queue,
            layout,
            memory,
            eligibility,
        }
    }
    fn execute(
        &self,
        words: &[Vec<u32>; 7],
        decoder_count: u32,
        eligibility: bool,
    ) -> (Vec<u32>, Vec<u32>) {
        let buffers: Vec<_> = words
            .iter()
            .map(|words| {
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("synthetic-production-ABI"),
                        contents: bytemuck::cast_slice(words),
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
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
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.layout,
            entries: &entries,
        });
        let readbacks: Vec<_> = [5, 6]
            .iter()
            .map(|&i| {
                self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("fixture-readback"),
                    size: buffers[i].size(),
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            })
            .collect();
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_bind_group(0, &group, &[]);
            pass.set_pipeline(&self.memory);
            pass.dispatch_workgroups(1, 1, 1);
        }
        if eligibility {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_bind_group(0, &group, &[]);
            pass.set_pipeline(&self.eligibility);
            pass.dispatch_workgroups(decoder_count.div_ceil(64), 1, 1);
        }
        for (i, readback) in readbacks.iter().enumerate() {
            encoder.copy_buffer_to_buffer(&buffers[i + 5], 0, readback, 0, readback.size());
        }
        let submission = self.queue.submit(Some(encoder.finish()));
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(std::time::Duration::from_secs(60)),
            })
            .unwrap();
        let values: Vec<_> = readbacks
            .iter()
            .map(|buffer| {
                let (tx, rx) = std::sync::mpsc::channel();
                buffer
                    .slice(..)
                    .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
                self.device
                    .poll(wgpu::PollType::wait_indefinitely())
                    .unwrap();
                rx.recv().unwrap().unwrap();
                let mapped = buffer.slice(..).get_mapped_range();
                let words = bytemuck::cast_slice::<u8, u32>(&mapped).to_vec();
                drop(mapped);
                buffer.unmap();
                words
            })
            .collect();
        (values[0].clone(), values[1].clone())
    }
}

fn graph(schema: u32) -> (BrainPhenotype, FoundationWeightAsset) {
    let genome = BrainGenome::scaffold(0x2048_2026, BrainCapacityClass::N2048_ID);
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.).unwrap());
    let (p, _) = match schema {
        1 => PhenotypeCompiler::compile_n2048_experimental_graph(
            genome.clone(),
            development.clone(),
            SensorProfile::GroundedObjectSlotsV1,
        ),
        2 => PhenotypeCompiler::compile_n2048_memory_gain_graph(
            genome.clone(),
            development.clone(),
            SensorProfile::GroundedObjectSlotsV1,
        ),
        3 => PhenotypeCompiler::compile_n2048_grip_reader_graph(
            genome.clone(),
            development.clone(),
            SensorProfile::GroundedGripObjectSlotsV1,
        ),
        _ => unreachable!(),
    }
    .unwrap();
    assert_eq!(
        p.candidate_decoder()
            .memory_channel()
            .unwrap()
            .schema_version(),
        schema as u16
    );
    assert!(p.cognitive_channel_plan().is_none());
    assert!(!p.synapses().iter().any(|s| matches!(s.kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::CognitiveContext)));
    let source = FoundationWeightAsset::from_phenotype_for_genetic_birth(&p).unwrap();
    let (bound, _) = match schema {
        1 => PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
            genome,
            development,
            source.clone(),
        ),
        2 => PhenotypeCompiler::compile_n2048_memory_gain_foundation_candidate(
            genome,
            development,
            source.clone(),
        ),
        3 => PhenotypeCompiler::compile_n2048_grip_reader_foundation_candidate(
            genome,
            development,
            source.clone(),
        ),
        _ => unreachable!(),
    }
    .unwrap();
    assert_eq!(bound.synapses(), p.synapses());
    (bound, source)
}

fn make_frame(profile: SensorProfile, action: u32) -> PerceptionFrame {
    let tick = Tick::new(8);
    let slot = GroundedObjectSlotV1 {
        slot_index: 0,
        tracked_object_id: TrackedObjectId(77),
        bearing: [0., 1.],
        distance: 0.02,
        relative_velocity: [0.; 3],
        color: [0.2; 3],
        material: [0.1; 3],
        shape: [0.3; 3],
        chemical: [0.; 3],
        contact: 1.,
        proprioception: [0.; 2],
        temperature: 0.,
        terrain: [0.; 2],
        confidence: Confidence::new(1.).unwrap(),
    };
    let family = if action == 2201 {
        CandidateActionFamily::Contact
    } else {
        CandidateActionFamily::Other
    };
    let candidate = ActionCandidate::new(
        0,
        ActionId(action),
        ActionKind::Interact,
        family,
        CandidateObservationRef::ObjectSlot(0),
        ActionTarget::new(Some(WorldEntityId(9)), None),
        slot.candidate_features().unwrap(),
        Confidence::new(1.).unwrap(),
        NormalizedScalar::new(0.2).unwrap(),
        DurationTicks::new(1),
        DurationTicks::new(1),
    )
    .unwrap();
    let grips = if profile == SensorProfile::GroundedGripObjectSlotsV1 {
        vec![GroundedGripSlotV1 {
            slot_index: 0,
            ownership: if action == 2201 {
                GripOwnershipV1::NotSelfHeld
            } else {
                GripOwnershipV1::SelfHeld
            },
            confidence: Confidence::new(1.).unwrap(),
        }]
    } else {
        vec![]
    };
    PerceptionFrameDraft::new_with_grip_observations(
        OrganismId(1),
        tick,
        profile,
        SensorySnapshot::new(
            OrganismId(1),
            tick,
            Vec3f::ZERO,
            SensoryChannels::ZERO,
            Default::default(),
        )
        .unwrap(),
        BodySnapshot {
            pose: Pose::IDENTITY,
            velocity: Velocity::ZERO,
        },
        HomeostaticSnapshot::baseline(tick),
        vec![candidate],
        SensorProfileProvenance::new(profile, SensoryAbiVersion::CURRENT, tick).unwrap(),
        vec![slot],
        grips,
    )
    .unwrap()
    .finalize(PerceptionContextBlock::empty())
    .unwrap()
}

fn evidence(sign: f32, present: bool) -> CandidateMemoryContextV1 {
    let count = u16::from(present);
    let m = CandidateMemoryContextV1 {
        candidate_index: 0,
        target_latent: if present {
            [0.3, -0.2, 0.7, -0.4, 0.1, -0.6, 0.5, -0.8]
        } else {
            [0.; 8]
        },
        family_value: if present {
            [sign * 0.8, -0.4, 0.2, -0.1]
        } else {
            [0.; 4]
        },
        target_confidence: Confidence::new(if present { 0.5 } else { 0. }).unwrap(),
        family_confidence: Confidence::new(if present { 0.75 } else { 0. }).unwrap(),
        target_source_count: count,
        family_source_count: count,
        best_target_source: present.then_some(MemoryId(1)),
        best_family_source: present.then_some(MemoryId(2)),
    };
    m.validate_contract().unwrap();
    m
}

const INPUT: usize = 64;
const MEMORY: usize = 160;
const CANDIDATE: usize = GPU_ACTIVE_DISPATCH_ROW_WORDS;
const LEARNING_HEADER: usize = GPU_PERCEPTION_DISPATCH_ROW_WORDS;
const MEMORY_HEADER: usize = LEARNING_HEADER + GPU_LEARNING_HEADER_WORDS;
fn run_case(
    device: &Device,
    schema: u32,
    action: u32,
    sign: f32,
    present: bool,
    saturated: bool,
) -> f32 {
    let (p, source) = graph(schema);
    let frame = make_frame(p.sensor_profile(), action);
    let m = evidence(sign, present);
    let c = &frame.candidates()[0];
    let mut inputs = [0.; 54];
    inputs[..24].copy_from_slice(&c.features.0);
    for i in 0..8 {
        inputs[24 + i] = m.target_latent[i] * m.target_confidence.raw();
    }
    for i in 0..4 {
        inputs[32 + i] = m.family_value[i] * m.family_confidence.raw();
    }
    let mut cognitive: [f32; 18] = std::array::from_fn(|i| (i as f32 - 9.) / 16.);
    cognitive[16] = 1.; // The production forecast-available lane is Boolean.
    cognitive[17] = 0.; // Production cognitive upload leaves the grip port unused.
    if schema == 3 {
        inputs[36..54].copy_from_slice(&cognitive);
    }
    // Real CPU operation reader, including typed profile/ownership checks.
    inputs[53] = grip_research::operation_value_input(&frame, c, &m).unwrap();
    assert_eq!(
        inputs[53],
        if schema == 3 && action != 212 && present {
            sign * 0.6
        } else {
            0.
        }
    );
    let mask = crate::StageTrainableMask::recurrent_only(&p).unwrap();
    let trainer =
        CpuFoundationTrainer::new(p.clone(), source, mask, crate::AdamWConfig::default()).unwrap();
    let initial = TrainingInitialState {
        activations: vec![0.; p.neuron_count() as usize],
        activity_ema: vec![0.; p.neuron_count() as usize],
        metabolic_load: vec![0.; p.neuron_count() as usize],
        dendrites: Default::default(),
    };
    let mut cpu = CpuOrganismState::new(&p, frame.organism_id(), initial).unwrap();
    for (i, s) in p.synapses().iter().enumerate() {
        if let CompiledSynapseKind::Decoder(coord) = s.kind() {
            if coord.head() == DecoderHeadKind::MemoryContext {
                cpu.checkpoint.lifetime[i] = if saturated {
                    // Only family-value lane 32 contributes; its sign is controlled.
                    if coord.input_lane() == 32 {
                        1.
                    } else {
                        0.
                    }
                } else {
                    ((i % 13) as f32 - 6.) / 64.
                };
                cpu.checkpoint.fast[i] = if saturated {
                    0.
                } else {
                    ((i % 7) as f32 - 3.) / 32.
                };
            }
        }
    }
    let lifetime = cpu.checkpoint.lifetime.clone();
    let fast = cpu.checkpoint.fast.clone();
    let replay = TrainingReplayTick {
        encoded_inputs: vec![0.; p.neuron_count() as usize],
        projection_gain: 1.,
        local_threshold_shift: 0.,
        microstep_count: 0,
        enabled_routes: vec![true; p.projections().len()],
        effective_weight_offsets: vec![0.; p.synapses().len()],
        structural_synapses: vec![],
        candidates: vec![crate::TrainingReplayCandidate {
            family: c.family,
            decoder_inputs: inputs,
            innate_bias: 0.,
        }],
    };
    let effects = NeuralReceptorEffects {
        source_tick: frame.tick(),
        source_chemistry_version: 1,
        interoceptive_gain: 1.,
        regional_excitability: 1.,
        projection_gain: 1.,
        local_threshold_shift: 0.,
        attention_gain: 1.,
        plasticity_appetitive: 0.,
        plasticity_aversive: 0.,
        structural_growth_gate: 0.,
        sleep_gate: 0.,
        consolidation_gate: 0.,
    };
    let result = cpu.advance(&trainer, replay, &effects).unwrap();
    let joint = crate::PpoJointAction {
        candidate_count: 1,
        representative_mask: 1,
        motor_masks: [0, 0, 1, 0, 0, 0],
        representative: 0,
        forced_slots: vec![Some(2)],
        motor_candidates: [None, None, Some(0), None, None, None],
        old_joint_log_probability: 0.,
        temperature: 1.,
    };
    let bundle = MotorCommandBundle::new(
        frame.organism_id(),
        ExperienceSequenceId(9),
        frame.tick(),
        vec![],
    )
    .unwrap();
    let binding = CpuDecisionBinding {
        sequence_id: ExperienceSequenceId(9),
        frame_digest: frame.frame_digest(),
        action_id: c.action_id,
        candidate_feature_digest: c.feature_digest().unwrap(),
        dispatch_generation: result.dispatch_generation,
        active_activation_side: result.active_activation_side,
        joint_selection: JointActionSelectionV1::new(
            joint.motor_candidates.map(|i| i.map_or(0, |i| i + 1)),
            &bundle,
        )
        .unwrap(),
    };
    cpu.stage_selection(&p, &joint, binding, false).unwrap();
    let expected_eligibility = cpu.pending.as_ref().unwrap().staged.as_ref().unwrap();
    let mut plan = GpuClassBucketPlan::new(BrainCapacityClass::n2048(), 1).unwrap();
    let slot = plan.insert_phenotype(0, 1, &p).unwrap();
    let upload = GpuPhenotypeUpload::try_from(&p).unwrap();
    let perception = GpuPerceptionUpload::try_from_frame(&frame, &slot, 0).unwrap();
    let old = make_frame(SensorProfile::GroundedObjectSlotsV1, 2202);
    let new = make_frame(SensorProfile::GroundedGripObjectSlotsV1, 2202);
    assert!(
        GpuPerceptionUpload::try_from_frame(if schema == 3 { &old } else { &new }, &slot, 0)
            .is_err()
    );
    let ranges = slot.word_ranges();
    let mut state = plan.mutable_state_words().to_vec();
    for (canonical, &local) in upload.canonical_to_local_synapse.iter().enumerate() {
        state[ranges.lifetime_weight_words.start as usize + local as usize] =
            lifetime[canonical].to_bits();
        state[ranges.fast_weight_words.start as usize + local as usize] = fast[canonical].to_bits();
    }
    let base = p
        .candidate_decoder()
        .families()
        .iter()
        .find(|f| f.family() == c.family)
        .unwrap()
        .bias();
    state[ranges.candidate_logit_words.start as usize] = base.to_bits();
    // The earlier selector/prevalidation passes are deliberately outside this
    // numerical fixture. Seed their successful selection status explicitly.
    let selection = GpuSelectionRecord {
        slot_generation: 1,
        dispatch_generation_lo: 1,
        status: 2,
        candidate_index: 0,
        ..GpuSelectionRecord::zeroed()
    };
    state[ranges.selection_words.start as usize..ranges.selection_words.end as usize]
        .copy_from_slice(selection.words());
    let stride = slot.decoder_input_stride() as usize;
    let mut payload = vec![0u32; 256];
    for i in 0..stride {
        payload[INPUT + i] = if i < 24 {
            inputs[i].to_bits()
        } else {
            123.0f32.to_bits()
        };
    }
    let record = GpuCandidateMemoryRecord {
        candidate_index: 0,
        target_confidence: m.target_confidence.raw(),
        family_confidence: m.family_confidence.raw(),
        source_counts_packed: u32::from(m.target_source_count)
            | (u32::from(m.family_source_count) << 16),
        target_latent: m.target_latent,
        family_value: m.family_value,
    };
    payload[MEMORY..MEMORY + 16].copy_from_slice(record.words());
    if schema == 3 {
        let cognitive = GpuCognitiveProjectionRecord {
            schema_version: 1,
            candidate_index: 0,
            forecast_available: 1,
            reserved: 0,
            values: cognitive,
            reserved_tail: [0; 2],
        };
        payload[MEMORY + 16..MEMORY + 40].copy_from_slice(cognitive.words());
    }
    let mut headers = vec![0; CANDIDATE + 8];
    headers[CANDIDATE..].copy_from_slice(perception.candidates[0].words());
    let memory_header = GpuMemoryContextHeader {
        schema_version: 1,
        class_id: slot.record().class_id,
        slot: 0,
        slot_generation: 1,
        tick_lo: 8,
        tick_hi: 0,
        candidate_count: 1,
        memory_context_offset: MEMORY as u32,
        candidate_offset: CANDIDATE as u32,
        profile_id: slot.sensor_profile_raw(),
        profile_schema_version: 1,
        sensory_abi_version: 1,
        brain_slot_index: 0,
        decoder_learning_input_offset: INPUT as u32,
        perception_header_index: 0,
        neural_receptor_effects_offset: 1,
    };
    headers[MEMORY_HEADER..MEMORY_HEADER + GPU_MEMORY_CONTEXT_HEADER_WORDS]
        .copy_from_slice(memory_header.words());
    let learning = GpuLearningHeader {
        schema_version: 3,
        class_id: slot.record().class_id,
        slot_generation: 1,
        candidate_count: 1,
        candidate_offset: CANDIDATE as u32,
        decoder_learning_input_offset: INPUT as u32,
        selection_offset: ranges.selection_words.start,
        decoder_input_stride: stride as u32,
        decoder_synapse_count: slot.record().synapse_count - slot.record().recurrent_synapse_count,
        ..GpuLearningHeader::zeroed()
    };
    headers[LEARNING_HEADER..MEMORY_HEADER].copy_from_slice(learning.words());
    let words = [
        bytemuck::cast_slice(plan.brain_slot_records()).to_vec(),
        bytemuck::cast_slice(plan.phenotype_identities()).to_vec(),
        plan.immutable_plan_words().to_vec(),
        plan.immutable_weight_words().to_vec(),
        headers,
        payload,
        state,
    ];
    let (gpu_inputs, gpu_state) = device.execute(&words, learning.decoder_synapse_count, true);
    let actual = f32::from_bits(gpu_state[ranges.candidate_logit_words.start as usize]);
    assert!(
        (actual - result.candidate_logits[0]).abs() <= 2e-5,
        "schema={schema} action={action} actual={actual} CPU={:?}",
        result.candidate_logits
    );
    for i in 0..stride {
        assert_eq!(
            f32::from_bits(gpu_inputs[INPUT + i]),
            inputs[i],
            "schema={schema} action={action} feature={i}"
        );
    }
    let mut memory_rows = 0;
    let mut max_eligibility_error = 0.0_f32;
    let mut checked_decoder_rows = 0;
    for (canonical, &local) in upload.canonical_to_local_synapse.iter().enumerate() {
        if let CompiledSynapseKind::Decoder(coord) = p.synapses()[canonical].kind() {
            let metadata = &upload.synapse_learning_metadata[local as usize];
            let actual = f32::from_bits(
                gpu_state[ranges.decoder_eligibility_bank_1_words.start as usize
                    + metadata.eligibility_local_index as usize],
            );
            checked_decoder_rows += 1;
            max_eligibility_error =
                max_eligibility_error.max((actual - expected_eligibility[canonical]).abs());
            assert!(
                (actual - expected_eligibility[canonical]).abs() <= 1e-6,
                "eligibility schema={schema} action={action} canonical={canonical}"
            );
            if coord.head() == DecoderHeadKind::MemoryContext {
                memory_rows += 1;
            }
        }
    }
    assert_eq!(memory_rows, 4096);
    assert!(gpu_state
        [ranges.diagnostic_words.start as usize..ranges.diagnostic_words.end as usize]
        .iter()
        .all(|w| *w == 0));
    if schema == 3 {
        // Deliberately corrupt only the low-level header. Host admission already
        // rejected this above; independently check the shader's schema3 guard.
        let mut wrong = words.clone();
        wrong[4][MEMORY_HEADER + 9] = 3;
        let (rejected_inputs, rejected_state) =
            device.execute(&wrong, learning.decoder_synapse_count, false);
        assert_eq!(rejected_inputs, wrong[5]);
        assert_eq!(
            rejected_state[ranges.candidate_logit_words.start as usize],
            base.to_bits()
        );
        assert_ne!(
            rejected_state[ranges.diagnostic_words.start as usize + 2],
            0
        );
    }
    eprintln!("grip_reader_case schema={schema} action={action} sign={sign} evidence={present} saturated={saturated} logit={actual} features={stride} memory_eligibility_rows={memory_rows} decoder_rows={checked_decoder_rows} max_eligibility_error={max_eligibility_error} logit_error={}", (actual-result.candidate_logits[0]).abs());
    actual - base
}

#[test]
#[ignore = "explicit synthetic CPU Vulkan parity; not hardware or learned-behavior evidence"]
fn software_vulkan_matches_cpu_grip_reader_logits_features_and_eligibility() {
    let device = Device::new();
    for action in [2201, 2202, 212] {
        for sign in [-1., 1.] {
            run_case(&device, 3, action, sign, true, false);
        }
        assert_eq!(run_case(&device, 3, action, -1., false, false), 0.);
    }
    for sign in [-1., 1.] {
        assert!((run_case(&device, 1, 2202, sign, true, true) - sign * 0.5).abs() < 1e-6);
        assert!((run_case(&device, 2, 2202, sign, true, true) - sign * 12.).abs() < 1e-6);
        assert!((run_case(&device, 3, 2202, sign, true, true) - sign * 12.).abs() < 1e-6);
    }
    eprintln!("grip_reader_parity_complete cases=15; native_CPU_forward_and_staging=true; unchanged_WGSL_execution=true; cognitive_decoder_absent=true; synthetic=true");
}
