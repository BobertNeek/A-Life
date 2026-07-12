//! Shared fixture construction and bounded real-GPU readback plumbing.
//!
//! Host code here only compiles/uploads contracts and derives structural counts. It never
//! executes neural math, supplies a neural oracle, or falls back from GPU authority.
#![allow(dead_code)] // Shared by integration targets that intentionally use disjoint helpers.

use alife_core::{
    ActionCandidate, ActionId, ActionKind, ActionTarget, BodySnapshot, BrainCapacityClass,
    BrainGenome, BrainPhenotype, CandidateActionFamily, CandidateFeatureVector,
    CandidateObservationRef, Confidence, DevelopmentState, DurationTicks, HomeostaticSnapshot,
    NormalizedScalar, OrganismId, PerceptionFrame, PhenotypeCompiler, Pose, SensorProfile,
    SensoryChannels, SensorySnapshot, Tick, Vec3f, Velocity,
};

pub fn n512_phenotype(seed: u64) -> BrainPhenotype {
    n512_phenotype_at_maturation(seed, 0.35)
}

pub fn n512_phenotype_at_maturation(seed: u64, maturation: f32) -> BrainPhenotype {
    let capacity = BrainCapacityClass::n512();
    let genome = BrainGenome::scaffold(seed, capacity.id());
    let development = DevelopmentState::new(
        genome.id,
        Tick::ZERO,
        NormalizedScalar::new(maturation).unwrap(),
    );
    PhenotypeCompiler::compile(
        &genome,
        &capacity,
        &development,
        SensorProfile::PrivilegedAffordanceV1,
    )
    .unwrap()
}

pub fn controlled_n512_phenotype_at_maturation(maturation: f32) -> BrainPhenotype {
    // Seed 13 is the checked-in deterministic fixture with the required
    // Idle/Inspect motor self-loop at every maturation used by these tests.
    n512_phenotype_at_maturation(13, maturation)
}

pub fn controlled_sensory_n512_phenotype() -> BrainPhenotype {
    // Seed 15 is the checked-in deterministic fixture with the required
    // encoded two-hop sensory-to-motor path.
    n512_phenotype_at_maturation(15, 0.35)
}

pub fn perception_frame(
    organism_raw: u64,
    nonzero: bool,
    candidate_count: usize,
) -> PerceptionFrame {
    assert!((1..=2).contains(&candidate_count));
    let organism_id = OrganismId(organism_raw);
    let tick = Tick::new(77 + organism_raw);
    let mut channels = SensoryChannels::ZERO;
    let translation = if nonzero {
        channels.visual_affordance[0] = 0.75;
        channels.visual_affordance[1] = 0.25;
        channels.auditory_acoustic[0] = 0.5;
        channels.novelty_signal = NormalizedScalar::new(0.6).unwrap();
        Vec3f::new(0.25, -0.5, 0.75)
    } else {
        Vec3f::ZERO
    };
    let sensory =
        SensorySnapshot::new(organism_id, tick, Vec3f::ZERO, channels, Default::default()).unwrap();
    let candidates = (0..candidate_count)
        .map(|index| {
            let mut features = CandidateFeatureVector::zero();
            if nonzero {
                features.0[index] = 0.5 + index as f32 * 0.1;
            }
            ActionCandidate::new(
                index as u16,
                ActionId(4 + index as u32),
                ActionKind::Inspect,
                CandidateActionFamily::Inspect,
                CandidateObservationRef::None,
                ActionTarget::NONE,
                features,
                Confidence::new(0.8).unwrap(),
                NormalizedScalar::new(0.1).unwrap(),
                DurationTicks::new(1),
                DurationTicks::new(1),
            )
            .unwrap()
        })
        .collect();
    PerceptionFrame::new(
        organism_id,
        tick,
        SensorProfile::PrivilegedAffordanceV1,
        sensory,
        BodySnapshot {
            pose: Pose {
                translation,
                ..Pose::IDENTITY
            },
            velocity: Velocity::ZERO,
        },
        HomeostaticSnapshot::baseline(tick),
        candidates,
    )
    .unwrap()
}

#[cfg(feature = "gpu-tests")]
mod hardware {
    use std::sync::mpsc;

    use alife_core::{BrainCapacityClass, BrainPhenotype, PerceptionFrame};
    use alife_gpu_backend::{
        GpuActiveBatchEntry, GpuBrainSlot, GpuClassBucketBuffers, GpuClassBucketPlan,
        GpuClosedLoopPipelines, GpuPhenotypeUpload, GpuSelectionRecord,
    };

    const SLOT_COUNT: usize = 3;
    const MAX_SAMPLE_WORDS_PER_BANK: usize = 8;
    const DIAGNOSTIC_WORDS: usize = 4;
    const WORDS_PER_SLOT: usize = MAX_SAMPLE_WORDS_PER_BANK * 2 + DIAGNOSTIC_WORDS;
    const READBACK_WORDS: usize = SLOT_COUNT * WORDS_PER_SLOT;
    const FRAME_BASE_WORDS: u32 = 64;

    #[derive(Debug, Clone, PartialEq)]
    pub struct SlotReadback {
        pub activation_a: Vec<f32>,
        pub activation_b: Vec<f32>,
        pub active_tiles: u32,
        pub active_synapses: u32,
        pub finite_rejections: u32,
        pub gpu_active_side: u32,
    }

    #[derive(Debug, Clone)]
    pub struct BatchReadback {
        pub adapter_name: String,
        pub slots: Vec<SlotReadback>,
        pub host_final_sides: Vec<u32>,
        pub sample_indices: Vec<u32>,
        pub recurrent_sample_positions: Vec<usize>,
        pub loop_sample_positions: Vec<usize>,
        pub readback_bytes: u64,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct CompactSelection {
        pub candidate_index: u32,
        pub logit: f32,
        pub confidence_q16: u32,
        pub status: u32,
        pub dispatch_generation: u64,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct GpuFrameResult {
        pub adapter_identity: String,
        pub selection: CompactSelection,
        pub record: GpuSelectionRecord,
        pub active_activation_side: u32,
        pub compact_readback_bytes: u64,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct DecoderLesionReceipt {
        pub adapter_identity: String,
        pub changed_ranges: Vec<std::ops::Range<u32>>,
        pub recurrent_prefixes_unchanged: bool,
    }

    pub struct GpuPipelineFixture {
        adapter_name: String,
        device: wgpu::Device,
        queue: wgpu::Queue,
        buffers: GpuClassBucketBuffers,
        manual_readback: wgpu::Buffer,
        pipelines: GpuClosedLoopPipelines,
        plan: GpuClassBucketPlan,
        slots: [GpuBrainSlot; SLOT_COUNT],
        immutable_weight_words: Vec<u32>,
        initial_mutable_state_words: Vec<u32>,
        sample_indices: Vec<u32>,
        recurrent_sample_positions: Vec<usize>,
        loop_sample_positions: Vec<usize>,
    }

    impl GpuPipelineFixture {
        pub async fn new(phenotype: &BrainPhenotype) -> Self {
            Self::new_with_phenotypes([phenotype, phenotype, phenotype]).await
        }

        pub async fn new_with_phenotypes(phenotypes: [&BrainPhenotype; SLOT_COUNT]) -> Self {
            assert!(phenotypes
                .iter()
                .all(|phenotype| phenotype.brain_class_id() == BrainCapacityClass::N512_ID));
            let capacity = BrainCapacityClass::n512();
            let mut plan = GpuClassBucketPlan::new(capacity, SLOT_COUNT as u32).unwrap();
            let slot0 = plan.insert_phenotype(0, 7, phenotypes[0]).unwrap();
            let slot1 = plan.insert_phenotype(1, 7, phenotypes[1]).unwrap();
            let slot2 = plan.insert_phenotype(2, 7, phenotypes[2]).unwrap();

            let instance = wgpu::Instance::default();
            let adapters = instance.enumerate_adapters(wgpu::Backends::all()).await;
            let inventory = adapters
                .iter()
                .map(|adapter| format!("{:?}", adapter.get_info()))
                .collect::<Vec<_>>()
                .join("\n");
            let adapter = adapters
                .into_iter()
                .find(|adapter| {
                    matches!(
                        adapter.get_info().device_type,
                        wgpu::DeviceType::IntegratedGpu | wgpu::DeviceType::DiscreteGpu
                    )
                })
                .unwrap_or_else(|| {
                    panic!("no integrated/discrete GPU adapter; inventory:\n{inventory}")
                });
            let info = adapter.get_info();
            preflight_n512_limits(&adapter, &capacity);
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor {
                    label: Some("closed-loop-wgsl-test-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: adapter.limits(),
                    experimental_features: wgpu::ExperimentalFeatures::disabled(),
                    memory_hints: wgpu::MemoryHints::MemoryUsage,
                    trace: wgpu::Trace::Off,
                })
                .await
                .expect("gpu-tests requires the selected GPU device");

            let immutable_weight_words = plan.immutable_weight_words().to_vec();
            let upload = GpuPhenotypeUpload::try_from(phenotypes[0]).unwrap();
            let (sample_indices, recurrent_sample_positions, loop_sample_positions) =
                derive_sample_indices(&upload, phenotypes[0].neuron_count());
            let mut initial_mutable_state_words = plan.mutable_state_words().to_vec();
            for slot in [&slot0, &slot1, &slot2] {
                let a = 0.125_f32.to_bits();
                let b = (-0.25_f32).to_bits();
                initial_mutable_state_words[slot.word_ranges().activation_a_words.start as usize
                    ..slot.word_ranges().activation_a_words.end as usize]
                    .fill(a);
                initial_mutable_state_words[slot.word_ranges().activation_b_words.start as usize
                    ..slot.word_ranges().activation_b_words.end as usize]
                    .fill(b);
            }
            let manual_readback = empty_buffer(
                &device,
                "bounded-manual-neural-readback",
                (READBACK_WORDS * 4) as u64,
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            );
            let buffers = GpuClassBucketBuffers::new(
                &plan,
                initialized_storage(
                    &device,
                    "brain-slots",
                    bytemuck::cast_slice(plan.brain_slot_records()),
                    true,
                ),
                initialized_storage(
                    &device,
                    "identities",
                    bytemuck::cast_slice(plan.phenotype_identities()),
                    false,
                ),
                initialized_storage(
                    &device,
                    "immutable-plan",
                    bytemuck::cast_slice(plan.immutable_plan_words()),
                    false,
                ),
                initialized_storage(
                    &device,
                    "immutable-weights",
                    bytemuck::cast_slice(plan.immutable_weight_words()),
                    true,
                ),
                empty_storage(
                    &device,
                    "dispatch-rows",
                    (SLOT_COUNT * alife_gpu_backend::GPU_ACTIVE_DISPATCH_ROW_WORDS * 4) as u64,
                    false,
                ),
                empty_storage(&device, "frame-payload", 4096, false),
                initialized_storage(
                    &device,
                    "mutable-state",
                    bytemuck::cast_slice(&initial_mutable_state_words),
                    true,
                ),
                empty_buffer(
                    &device,
                    "upload-staging",
                    4096,
                    wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
                ),
                empty_buffer(
                    &device,
                    "compact-readback",
                    (SLOT_COUNT * 48) as u64,
                    wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                ),
            )
            .unwrap();
            let pipelines = GpuClosedLoopPipelines::new(&device, &buffers).unwrap();
            let mut fixture = Self {
                adapter_name: format!("{} ({:?}, {:?})", info.name, info.backend, info.device_type),
                device,
                queue,
                buffers,
                manual_readback,
                pipelines,
                plan,
                slots: [slot0, slot1, slot2],
                immutable_weight_words,
                initial_mutable_state_words,
                sample_indices,
                recurrent_sample_positions,
                loop_sample_positions,
            };
            fixture.restore_mutable_checkpoint();
            fixture
        }

        pub fn restore_mutable_checkpoint(&mut self) {
            self.pipelines
                .reset_active_sides_for_hardware_diagnostic()
                .unwrap();
            let mutable = self.buffers.neural_buffers()[6];
            self.queue.write_buffer(
                mutable,
                0,
                bytemuck::cast_slice(&self.initial_mutable_state_words),
            );
        }

        pub fn set_recurrent_genetic_weights_zeroed(&self, zeroed: bool) {
            let weights = self.buffers.neural_buffers()[3];
            for slot in &self.slots {
                let start = slot.word_ranges().genetic_weight_words.start as usize;
                let count = slot.record().recurrent_synapse_count as usize;
                let words = if zeroed {
                    vec![0_u32; count]
                } else {
                    self.immutable_weight_words[start..start + count].to_vec()
                };
                self.queue
                    .write_buffer(weights, start as u64 * 4, bytemuck::cast_slice(&words));
            }
        }

        pub fn set_decoder_genetic_weights_zeroed(&self, zeroed: bool) {
            let weights = self.buffers.neural_buffers()[3];
            for slot in &self.slots {
                let start = slot.word_ranges().genetic_weight_words.start as usize
                    + slot.record().recurrent_synapse_count as usize;
                let end = slot.word_ranges().genetic_weight_words.end as usize;
                let words = if zeroed {
                    vec![0_u32; end - start]
                } else {
                    self.immutable_weight_words[start..end].to_vec()
                };
                self.queue
                    .write_buffer(weights, start as u64 * 4, bytemuck::cast_slice(&words));
            }
        }

        pub fn lesion_decoder_genetic_weights(&self) -> DecoderLesionReceipt {
            let mut changed_ranges = Vec::with_capacity(SLOT_COUNT);
            for slot in &self.slots {
                let start = slot.word_ranges().genetic_weight_words.start
                    + slot.record().recurrent_synapse_count;
                let end = slot.word_ranges().genetic_weight_words.end;
                assert!(self.immutable_weight_words[start as usize..end as usize]
                    .iter()
                    .any(|word| *word != 0));
                changed_ranges.push(start..end);
            }
            let recurrent_prefixes_unchanged =
                changed_ranges.iter().zip(&self.slots).all(|(range, slot)| {
                    range.start
                        == slot.word_ranges().genetic_weight_words.start
                            + slot.record().recurrent_synapse_count
                });
            self.set_decoder_genetic_weights_zeroed(true);
            DecoderLesionReceipt {
                adapter_identity: self.adapter_name.clone(),
                changed_ranges,
                recurrent_prefixes_unchanged,
            }
        }

        pub fn set_all_genetic_weights_zeroed(&self, zeroed: bool) {
            let weights = self.buffers.neural_buffers()[3];
            let words = if zeroed {
                vec![0_u32; self.immutable_weight_words.len()]
            } else {
                self.immutable_weight_words.clone()
            };
            self.queue
                .write_buffer(weights, 0, bytemuck::cast_slice(&words));
        }

        pub fn set_decoder_genetic_weights_non_finite(&self) {
            let weights = self.buffers.neural_buffers()[3];
            for slot in &self.slots {
                let start = slot.word_ranges().genetic_weight_words.start as usize
                    + slot.record().recurrent_synapse_count as usize;
                let end = slot.word_ranges().genetic_weight_words.end as usize;
                let words = vec![f32::NAN.to_bits(); end - start];
                self.queue
                    .write_buffer(weights, start as u64 * 4, bytemuck::cast_slice(&words));
            }
        }

        pub fn zero_all_mutable_layers_and_assert_biases(&self, phenotype: &BrainPhenotype) {
            let upload = GpuPhenotypeUpload::try_from(phenotype).unwrap();
            assert!(upload
                .neuron_dynamics
                .iter()
                .all(|row| row.bias_bits == 0.0_f32.to_bits()));
            assert!(upload
                .decoder_families
                .iter()
                .all(|row| row.bias_bits == 0.0_f32.to_bits()));
            let zero = vec![0_u32; self.initial_mutable_state_words.len()];
            self.queue.write_buffer(
                self.buffers.neural_buffers()[6],
                0,
                bytemuck::cast_slice(&zero),
            );
        }

        pub fn configure_controlled_motor_loop_and_decoder(
            &self,
            phenotype: &BrainPhenotype,
            inspect_on_positive: bool,
        ) {
            self.set_recurrent_genetic_weights_zeroed(true);
            self.set_decoder_genetic_weights_zeroed(true);
            let upload = GpuPhenotypeUpload::try_from(phenotype).unwrap();
            let decoder = upload.decoder_plans[0];
            let weights = self.buffers.neural_buffers()[3];
            let controlled = [0_u32, 2_u32]
                .into_iter()
                .find_map(|family_raw| {
                    let family = &upload.decoder_families[family_raw as usize];
                    assert_eq!(family.family_raw, family_raw);
                    let begin = ((family.weight_index_start
                        - upload.decoder_weight_index_word_base)
                        / 4) as usize;
                    let end = begin + family.weight_index_count as usize;
                    upload.decoder_weight_indices[begin..end]
                        .iter()
                        .filter(|map| map.input_lane == 0)
                        .find_map(|map| {
                            let target = decoder.motor_start + map.motor_index;
                            let recurrent_begin = upload.target_offsets[target as usize] as usize;
                            let recurrent_end = upload.target_offsets[target as usize + 1] as usize;
                            (recurrent_begin..recurrent_end)
                                .find(|cursor| upload.source_indices[*cursor] == target)
                                .map(|cursor| (family_raw, *map, target, cursor))
                        })
                })
                .expect("Idle or Inspect lane-zero motor has a recurrent self-loop");
            let (family_raw, map, target, self_cursor) = controlled;
            let dynamics = upload.neuron_dynamics[target as usize];
            assert_eq!(dynamics.bias_bits, 0.0_f32.to_bits());
            assert_eq!(dynamics.leak_bits, 0.25_f32.to_bits());
            assert_eq!(dynamics.activation_raw, 2);
            let recurrent_weight = -8.0_f32;
            let positive_winner_sign = if family_raw == 0 { 1.0_f32 } else { -1.0_f32 };
            let family_weight: f32 = if inspect_on_positive {
                -positive_winner_sign
            } else {
                positive_winner_sign
            };
            for slot in &self.slots {
                let recurrent_word =
                    slot.word_ranges().genetic_weight_words.start + self_cursor as u32;
                self.queue.write_buffer(
                    weights,
                    u64::from(recurrent_word) * 4,
                    bytemuck::bytes_of(&recurrent_weight.to_bits()),
                );
                let decoder_word =
                    slot.word_ranges().genetic_weight_words.start + map.global_synapse_id;
                self.queue.write_buffer(
                    weights,
                    u64::from(decoder_word) * 4,
                    bytemuck::bytes_of(&family_weight.to_bits()),
                );
            }
            let mutable = self.buffers.neural_buffers()[6];
            for slot in &self.slots {
                let positive = 1.0_f32;
                self.queue.write_buffer(
                    mutable,
                    u64::from(slot.word_ranges().activation_a_words.start + target) * 4,
                    bytemuck::bytes_of(&positive),
                );
                self.queue.write_buffer(
                    mutable,
                    u64::from(slot.word_ranges().activation_b_words.start + target) * 4,
                    bytemuck::bytes_of(&positive),
                );
            }
        }

        pub fn configure_controlled_sensory_path_and_decoder(&self, phenotype: &BrainPhenotype) {
            self.set_recurrent_genetic_weights_zeroed(true);
            self.set_decoder_genetic_weights_zeroed(true);
            self.zero_all_mutable_layers_and_assert_biases(phenotype);
            let upload = GpuPhenotypeUpload::try_from(phenotype).unwrap();
            let decoder = upload.decoder_plans[0];
            let sensory_targets = upload
                .encoder_assignments
                .iter()
                .filter(|row| row.source_group_raw == 1 && row.source_index == 0)
                .map(|row| row.target_neuron)
                .collect::<Vec<_>>();
            assert!(!sensory_targets.is_empty());
            let path = [0_u32, 2_u32]
                .into_iter()
                .find_map(|family_raw| {
                    let family = &upload.decoder_families[family_raw as usize];
                    let map_begin = ((family.weight_index_start
                        - upload.decoder_weight_index_word_base)
                        / 4) as usize;
                    let map_end = map_begin + family.weight_index_count as usize;
                    upload.decoder_weight_indices[map_begin..map_end]
                        .iter()
                        .filter(|map| map.input_lane == 0)
                        .find_map(|map| {
                            let motor = decoder.motor_start + map.motor_index;
                            let motor_begin = upload.target_offsets[motor as usize] as usize;
                            let motor_end = upload.target_offsets[motor as usize + 1] as usize;
                            (motor_begin..motor_end).find_map(|association_to_motor| {
                                let association = upload.source_indices[association_to_motor];
                                let association_begin =
                                    upload.target_offsets[association as usize] as usize;
                                let association_end =
                                    upload.target_offsets[association as usize + 1] as usize;
                                (association_begin..association_end)
                                    .find(|sensory_to_association| {
                                        sensory_targets.contains(
                                            &upload.source_indices[*sensory_to_association],
                                        )
                                    })
                                    .map(|sensory_to_association| {
                                        (*map, sensory_to_association, association_to_motor)
                                    })
                            })
                        })
                })
                .expect("compiled encoder and recurrent CSR expose a two-hop sensory-motor path");
            let (map, sensory_to_association, association_to_motor) = path;
            let weights = self.buffers.neural_buffers()[3];
            for slot in &self.slots {
                for cursor in [sensory_to_association, association_to_motor] {
                    let word = slot.word_ranges().genetic_weight_words.start + cursor as u32;
                    self.queue.write_buffer(
                        weights,
                        u64::from(word) * 4,
                        bytemuck::bytes_of(&4.0_f32.to_bits()),
                    );
                }
                let decoder_word =
                    slot.word_ranges().genetic_weight_words.start + map.global_synapse_id;
                self.queue.write_buffer(
                    weights,
                    u64::from(decoder_word) * 4,
                    bytemuck::bytes_of(&2.0_f32.to_bits()),
                );
            }
        }

        pub fn adapter_identity(&self) -> &str {
            &self.adapter_name
        }

        pub async fn run_frame(&mut self, frame: &PerceptionFrame) -> GpuFrameResult {
            let entries = [GpuActiveBatchEntry::new(frame, &self.slots[0])];
            let batch = self
                .pipelines
                .build_active_batch(&self.plan, &entries, FRAME_BASE_WORDS)
                .unwrap();
            // Task 6 production must add this atomic encode -> recurrent ->
            // decode -> select submission, including upload and exact compact
            // readback. The fixture intentionally has no CPU neural or
            // winner-selection implementation.
            let (records, actual_readback_bytes): (Vec<GpuSelectionRecord>, u64) = self
                .pipelines
                .submit_closed_loop_frame(&self.device, &self.queue, &self.buffers, &batch)
                .await
                .unwrap();
            assert_eq!(records.len(), 1);
            assert_eq!(actual_readback_bytes, 48);
            let record = records[0];
            assert_eq!(record.slot, self.slots[0].record().slot);
            assert_eq!(
                record.slot_generation,
                self.slots[0].record().slot_generation
            );
            assert!(record.active_activation_side <= 1);
            GpuFrameResult {
                adapter_identity: self.adapter_name.clone(),
                selection: CompactSelection {
                    candidate_index: record.candidate_index,
                    logit: f32::from_bits(record.logit_bits),
                    confidence_q16: record.confidence_q16,
                    status: record.status,
                    dispatch_generation: u64::from(record.dispatch_generation_lo)
                        | (u64::from(record.dispatch_generation_hi) << 32),
                },
                record,
                active_activation_side: record.active_activation_side,
                compact_readback_bytes: actual_readback_bytes,
            }
        }

        pub async fn run_frame_pair(
            &mut self,
            frames: [&PerceptionFrame; 2],
        ) -> [GpuFrameResult; 2] {
            let entries = [
                GpuActiveBatchEntry::new(frames[0], &self.slots[0]),
                GpuActiveBatchEntry::new(frames[1], &self.slots[1]),
            ];
            let batch = self
                .pipelines
                .build_active_batch(&self.plan, &entries, FRAME_BASE_WORDS)
                .unwrap();
            assert_eq!(batch.headers().len(), 2);
            for (row, header) in batch.headers().iter().enumerate() {
                let row_base = row * alife_gpu_backend::GPU_ACTIVE_DISPATCH_ROW_WORDS;
                assert_eq!(header.candidate_offset as usize, row_base + 16);
                assert!(header.sensory_offset >= FRAME_BASE_WORDS);
                assert_eq!(header.candidate_count, 2);
            }
            assert!(batch.headers()[1].sensory_offset > batch.headers()[0].sensory_offset);
            let first_candidate = alife_gpu_backend::GpuCandidateRecord::from_words(
                &batch.dispatch_header_words()[16..24],
            )
            .unwrap();
            let first_candidate_one = alife_gpu_backend::GpuCandidateRecord::from_words(
                &batch.dispatch_header_words()[24..32],
            )
            .unwrap();
            let second_row = alife_gpu_backend::GPU_ACTIVE_DISPATCH_ROW_WORDS;
            let second_row_candidate = alife_gpu_backend::GpuCandidateRecord::from_words(
                &batch.dispatch_header_words()[second_row + 16..second_row + 24],
            )
            .unwrap();
            let second_row_candidate_one = alife_gpu_backend::GpuCandidateRecord::from_words(
                &batch.dispatch_header_words()[second_row + 24..second_row + 32],
            )
            .unwrap();
            assert!(first_candidate.feature_offset > 0);
            assert_eq!(
                first_candidate_one.feature_offset,
                first_candidate.feature_offset + 24
            );
            assert!(second_row_candidate.feature_offset > first_candidate.feature_offset);
            assert_eq!(
                second_row_candidate_one.feature_offset,
                second_row_candidate.feature_offset + 24
            );
            let (records, actual_readback_bytes): (Vec<GpuSelectionRecord>, u64) = self
                .pipelines
                .submit_closed_loop_frame(&self.device, &self.queue, &self.buffers, &batch)
                .await
                .unwrap();
            assert_eq!(records.len(), 2);
            assert_eq!(actual_readback_bytes, 96);
            std::array::from_fn(|index| {
                let record = records[index];
                assert_eq!(record.slot, self.slots[index].record().slot);
                assert_eq!(
                    record.slot_generation,
                    self.slots[index].record().slot_generation
                );
                GpuFrameResult {
                    adapter_identity: self.adapter_name.clone(),
                    selection: CompactSelection {
                        candidate_index: record.candidate_index,
                        logit: f32::from_bits(record.logit_bits),
                        confidence_q16: record.confidence_q16,
                        status: record.status,
                        dispatch_generation: u64::from(record.dispatch_generation_lo)
                            | (u64::from(record.dispatch_generation_hi) << 32),
                    },
                    record,
                    active_activation_side: record.active_activation_side,
                    compact_readback_bytes: actual_readback_bytes / 2,
                }
            })
        }

        pub fn poison_activation_bank(&self, slot_index: usize, side: u32, value: f32) {
            assert!(slot_index < SLOT_COUNT);
            assert!(side <= 1);
            assert!(value.is_finite());
            let slot = &self.slots[slot_index];
            let range = if side == 0 {
                &slot.word_ranges().activation_a_words
            } else {
                &slot.word_ranges().activation_b_words
            };
            let poison = vec![value; slot.record().neuron_count as usize];
            self.queue.write_buffer(
                self.buffers.neural_buffers()[6],
                u64::from(range.start) * 4,
                bytemuck::cast_slice(&poison),
            );
        }

        pub fn foreign_same_tuple_batch_error(
            &mut self,
            phenotype: &BrainPhenotype,
            frame: &PerceptionFrame,
        ) -> alife_gpu_backend::GpuClosedLoopError {
            let mut foreign =
                GpuClassBucketPlan::new(BrainCapacityClass::n512(), SLOT_COUNT as u32).unwrap();
            let slot = foreign.insert_phenotype(0, 7, phenotype).unwrap();
            self.pipelines
                .build_active_batch(
                    &foreign,
                    &[GpuActiveBatchEntry::new(frame, &slot)],
                    FRAME_BASE_WORDS,
                )
                .unwrap_err()
        }

        pub fn oversized_frame_base_error(
            &mut self,
            frame: &PerceptionFrame,
        ) -> alife_gpu_backend::GpuClosedLoopError {
            self.pipelines
                .build_active_batch(
                    &self.plan,
                    &[GpuActiveBatchEntry::new(frame, &self.slots[0])],
                    1025,
                )
                .unwrap_err()
        }

        pub async fn run(&mut self, frames: [&PerceptionFrame; SLOT_COUNT]) -> BatchReadback {
            self.run_internal(frames, false).await
        }

        pub async fn run_with_complete_frame_payload_zeroed(
            &mut self,
            frames: [&PerceptionFrame; SLOT_COUNT],
        ) -> BatchReadback {
            self.run_internal(frames, true).await
        }

        async fn run_internal(
            &mut self,
            frames: [&PerceptionFrame; SLOT_COUNT],
            zero_complete_frame_payload: bool,
        ) -> BatchReadback {
            let entries = [
                GpuActiveBatchEntry::new(frames[0], &self.slots[0]),
                GpuActiveBatchEntry::new(frames[1], &self.slots[1]),
                GpuActiveBatchEntry::new(frames[2], &self.slots[2]),
            ];
            let mut batch = self
                .pipelines
                .build_active_batch(&self.plan, &entries, FRAME_BASE_WORDS)
                .unwrap();
            if zero_complete_frame_payload {
                batch.zero_frame_payload_for_hardware_diagnostic();
                assert!(batch.frame_payload_words().iter().all(|word| *word == 0));
            }
            assert_eq!(
                batch.dispatch_header_words().len(),
                SLOT_COUNT * alife_gpu_backend::GPU_ACTIVE_DISPATCH_ROW_WORDS
            );
            assert!(batch
                .headers()
                .iter()
                .all(|header| header.sensory_offset >= FRAME_BASE_WORDS));
            assert_eq!(
                [
                    batch.headers()[0].candidate_count,
                    batch.headers()[1].candidate_count
                ],
                [1, 2]
            );
            for (row, header) in batch.headers().iter().enumerate() {
                let row_base = row * alife_gpu_backend::GPU_ACTIVE_DISPATCH_ROW_WORDS;
                assert_eq!(header.candidate_offset as usize, row_base + 16);
                let used = header.candidate_count as usize * 8;
                assert!(batch.dispatch_header_words()[row_base + 16 + used
                    ..row_base + alife_gpu_backend::GPU_ACTIVE_DISPATCH_ROW_WORDS]
                    .iter()
                    .all(|word| *word == 0));
                assert_eq!(header.brain_slot_index as usize, row);
            }
            assert!(batch.headers()[1].sensory_offset > batch.headers()[0].sensory_offset);
            let neural = self.buffers.neural_buffers();
            self.queue.write_buffer(
                neural[4],
                0,
                bytemuck::cast_slice(batch.dispatch_header_words()),
            );
            self.queue.write_buffer(
                neural[5],
                0,
                bytemuck::cast_slice(batch.frame_payload_words()),
            );

            let receipt = self
                .pipelines
                .submit_encode_and_microsteps(&self.device, &self.queue, &batch)
                .unwrap();
            assert_eq!(
                receipt.max_microsteps_dispatched,
                self.slots
                    .iter()
                    .map(|slot| slot.record().microstep_count)
                    .max()
                    .unwrap()
            );
            let host_final_sides = (0..SLOT_COUNT)
                .map(|row| receipt.final_activation_side(row as u32).unwrap())
                .collect::<Vec<_>>();

            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("closed-loop-bounded-readback"),
                });
            let mut destination = 0_u64;
            for slot in &self.slots {
                for range in [
                    &slot.word_ranges().activation_a_words,
                    &slot.word_ranges().activation_b_words,
                ] {
                    for neuron in &self.sample_indices {
                        encoder.copy_buffer_to_buffer(
                            neural[6],
                            u64::from(range.start + *neuron) * 4,
                            &self.manual_readback,
                            destination,
                            4,
                        );
                        destination += 4;
                    }
                }
                encoder.copy_buffer_to_buffer(
                    neural[6],
                    u64::from(slot.word_ranges().diagnostic_words.start) * 4,
                    &self.manual_readback,
                    destination,
                    (DIAGNOSTIC_WORDS * 4) as u64,
                );
                destination += (DIAGNOSTIC_WORDS * 4) as u64;
            }
            self.queue.submit(Some(encoder.finish()));
            let used_words = SLOT_COUNT * (self.sample_indices.len() * 2 + DIAGNOSTIC_WORDS);
            let words = map_words(&self.device, &self.manual_readback, used_words).await;
            let mut cursor = 0;
            let mut slots = Vec::with_capacity(SLOT_COUNT);
            for _ in 0..SLOT_COUNT {
                let activation_a = words[cursor..cursor + self.sample_indices.len()]
                    .iter()
                    .map(|v| f32::from_bits(*v))
                    .collect();
                cursor += self.sample_indices.len();
                let activation_b = words[cursor..cursor + self.sample_indices.len()]
                    .iter()
                    .map(|v| f32::from_bits(*v))
                    .collect();
                cursor += self.sample_indices.len();
                let diagnostic = &words[cursor..cursor + DIAGNOSTIC_WORDS];
                cursor += DIAGNOSTIC_WORDS;
                slots.push(SlotReadback {
                    activation_a,
                    activation_b,
                    active_tiles: diagnostic[0],
                    active_synapses: diagnostic[1],
                    finite_rejections: diagnostic[2],
                    gpu_active_side: diagnostic[3],
                });
            }
            BatchReadback {
                adapter_name: self.adapter_name.clone(),
                slots,
                host_final_sides,
                sample_indices: self.sample_indices.clone(),
                recurrent_sample_positions: self.recurrent_sample_positions.clone(),
                loop_sample_positions: self.loop_sample_positions.clone(),
                readback_bytes: (used_words * 4) as u64,
            }
        }
    }

    pub fn expected_cadence_counts(phenotype: &BrainPhenotype, microsteps: u32) -> (u32, u32) {
        let upload = GpuPhenotypeUpload::try_from(phenotype).unwrap();
        let mut tiles = 0_u32;
        let mut synapses = 0_u32;
        for step in 0..microsteps {
            for route in &upload.route_metadata {
                if cadence_fires(route.update_cadence_raw, step) {
                    tiles += upload
                        .projections
                        .iter()
                        .find(|p| p.route_index == route.route_index)
                        .unwrap()
                        .active_tile_count;
                    synapses += upload
                        .route_indices
                        .iter()
                        .filter(|raw| **raw == route.route_index)
                        .count() as u32;
                }
            }
        }
        (tiles, synapses)
    }

    fn derive_sample_indices(
        upload: &GpuPhenotypeUpload,
        neuron_count: u32,
    ) -> (Vec<u32>, Vec<usize>, Vec<usize>) {
        let encoder_target = upload
            .encoder_assignments
            .first()
            .expect("N512 fixture needs an encoder assignment")
            .target_neuron;
        let sensory_association_routes = upload
            .projections
            .iter()
            .filter(|route| route.source_lobe_raw == 1 && route.target_lobe_raw == 6)
            .map(|route| route.route_index)
            .collect::<Vec<_>>();
        let motor_loop_routes = upload
            .projections
            .iter()
            .filter(|route| route.source_lobe_raw == 9 && route.target_lobe_raw == 9)
            .map(|route| route.route_index)
            .collect::<Vec<_>>();
        assert!(!motor_loop_routes.is_empty());

        let mut samples = vec![encoder_target];
        for target in 0..neuron_count {
            if samples.len() == MAX_SAMPLE_WORDS_PER_BANK {
                break;
            }
            let begin = upload.target_offsets[target as usize] as usize;
            let end = upload.target_offsets[target as usize + 1] as usize;
            if (begin..end).any(|row| motor_loop_routes.contains(&upload.route_indices[row]))
                && !samples.contains(&target)
            {
                samples.push(target);
                break;
            }
        }
        for target in 0..neuron_count {
            if samples.len() == MAX_SAMPLE_WORDS_PER_BANK {
                break;
            }
            let begin = upload.target_offsets[target as usize] as usize;
            let end = upload.target_offsets[target as usize + 1] as usize;
            if (begin..end)
                .any(|row| sensory_association_routes.contains(&upload.route_indices[row]))
                && !samples.contains(&target)
            {
                samples.push(target);
            }
        }
        for target in 0..neuron_count {
            if samples.len() == MAX_SAMPLE_WORDS_PER_BANK {
                break;
            }
            let begin = upload.target_offsets[target as usize];
            let end = upload.target_offsets[target as usize + 1];
            if begin < end && !samples.contains(&target) {
                samples.push(target);
            }
        }

        let recurrent_positions = samples
            .iter()
            .enumerate()
            .filter_map(|(position, target)| {
                (upload.target_offsets[*target as usize]
                    < upload.target_offsets[*target as usize + 1])
                    .then_some(position)
            })
            .collect::<Vec<_>>();
        let loop_positions = samples
            .iter()
            .enumerate()
            .filter_map(|(position, target)| {
                let begin = upload.target_offsets[*target as usize] as usize;
                let end = upload.target_offsets[*target as usize + 1] as usize;
                (begin < end
                    && (begin..end)
                        .any(|row| motor_loop_routes.contains(&upload.route_indices[row])))
                .then_some(position)
            })
            .collect::<Vec<_>>();
        assert!(!samples.is_empty());
        assert!(samples.len() <= MAX_SAMPLE_WORDS_PER_BANK);
        assert!(samples.iter().all(|index| *index < neuron_count));
        assert!(samples
            .iter()
            .enumerate()
            .all(|(index, value)| !samples[..index].contains(value)));
        assert!(samples.contains(&encoder_target));
        assert!(!recurrent_positions.is_empty());
        assert!(!loop_positions.is_empty());
        assert!(loop_positions.iter().all(|position| {
            let target = samples[*position] as usize;
            let begin = upload.target_offsets[target] as usize;
            let end = upload.target_offsets[target + 1] as usize;
            begin < end
                && (begin..end).any(|row| motor_loop_routes.contains(&upload.route_indices[row]))
        }));
        (samples, recurrent_positions, loop_positions)
    }

    fn cadence_fires(raw: u32, microstep: u32) -> bool {
        match raw {
            0 => true,
            1 | 2 => microstep.is_multiple_of(2),
            3 | 4 => microstep == 0,
            5 | 6 => false,
            _ => panic!("invalid cadence raw {raw}"),
        }
    }

    fn preflight_n512_limits(adapter: &wgpu::Adapter, capacity: &BrainCapacityClass) {
        let required = capacity.execution();
        let actual = adapter.limits();
        assert!(actual.max_buffer_size >= required.required_max_buffer_size());
        assert!(
            actual.max_storage_buffer_binding_size
                >= required.required_max_storage_buffer_binding_size()
        );
        assert!(actual.max_bind_groups >= required.required_max_bind_groups());
        assert!(
            actual.max_bindings_per_bind_group >= required.required_max_bindings_per_bind_group()
        );
        assert!(
            actual.max_storage_buffers_per_shader_stage
                >= required.required_max_storage_buffers_per_shader_stage()
        );
        assert!(
            actual.max_uniform_buffers_per_shader_stage
                >= required.required_max_uniform_buffers_per_shader_stage()
        );
        assert!(
            actual.max_dynamic_storage_buffers_per_pipeline_layout
                >= required.required_max_dynamic_storage_buffers_per_pipeline_layout()
        );
        assert!(
            actual.max_dynamic_uniform_buffers_per_pipeline_layout
                >= required.required_max_dynamic_uniform_buffers_per_pipeline_layout()
        );
        assert!(
            actual.max_compute_workgroup_storage_size
                >= required.required_max_compute_workgroup_storage_size()
        );
        assert!(
            actual.max_compute_workgroup_size_x >= required.required_max_compute_workgroup_size_x()
        );
        assert!(
            actual.max_compute_workgroup_size_y >= required.required_max_compute_workgroup_size_y()
        );
        assert!(
            actual.max_compute_workgroup_size_z >= required.required_max_compute_workgroup_size_z()
        );
        assert!(
            actual.max_compute_invocations_per_workgroup
                >= required.required_max_compute_invocations_per_workgroup()
        );
        assert!(
            actual.max_compute_workgroups_per_dimension
                >= required.required_max_compute_workgroups_per_dimension()
        );
        assert!(
            actual.min_storage_buffer_offset_alignment <= required.storage_offset_alignment_bytes()
        );
        assert!(
            actual.min_uniform_buffer_offset_alignment <= required.uniform_offset_alignment_bytes()
        );
        assert_eq!(required.required_feature_mask(), 0);
        assert_eq!(required.required_feature_mask_words(), 1);
    }

    fn initialized_storage(
        device: &wgpu::Device,
        label: &str,
        bytes: &[u8],
        copy_source: bool,
    ) -> wgpu::Buffer {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: bytes.len().max(4) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | if copy_source {
                    wgpu::BufferUsages::COPY_SRC
                } else {
                    wgpu::BufferUsages::empty()
                },
            mapped_at_creation: true,
        });
        {
            let mut mapped = buffer.slice(..).get_mapped_range_mut();
            let mut initial = vec![0_u8; mapped.len()];
            initial[..bytes.len()].copy_from_slice(bytes);
            mapped.copy_from_slice(&initial);
        }
        buffer.unmap();
        buffer
    }

    fn empty_storage(
        device: &wgpu::Device,
        label: &str,
        size: u64,
        copy_source: bool,
    ) -> wgpu::Buffer {
        empty_buffer(
            device,
            label,
            size,
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | if copy_source {
                    wgpu::BufferUsages::COPY_SRC
                } else {
                    wgpu::BufferUsages::empty()
                },
        )
    }

    fn empty_buffer(
        device: &wgpu::Device,
        label: &str,
        size: u64,
        usage: wgpu::BufferUsages,
    ) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: size.max(4),
            usage,
            mapped_at_creation: false,
        })
    }

    async fn map_words(device: &wgpu::Device, buffer: &wgpu::Buffer, count: usize) -> Vec<u32> {
        let slice = buffer.slice(..count as u64 * 4);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("device poll failed");
        receiver
            .recv()
            .expect("map callback dropped")
            .expect("readback map failed");
        let mapped = slice.get_mapped_range();
        let words = bytemuck::cast_slice(&mapped).to_vec();
        drop(mapped);
        buffer.unmap();
        words
    }
}

#[cfg(feature = "gpu-tests")]
#[allow(unused_imports)]
pub use hardware::{
    expected_cadence_counts, BatchReadback, CompactSelection, DecoderLesionReceipt, GpuFrameResult,
    GpuPipelineFixture, SlotReadback,
};
