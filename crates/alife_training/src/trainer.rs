//! Offline-only production-graph WGSL trainer implementation.

use std::{num::NonZeroU64, sync::mpsc};

use alife_core::{
    BrainPhenotype, CompiledSynapseKind, FoundationWeightAsset, ProjectionType,
    ScaffoldContractError, TrainingStageManifest, CANDIDATE_FEATURE_COUNT,
};
use alife_gpu_backend::{GpuClosedLoopBackend, GpuRuntimeProfile};
use alife_runtime::{GpuAuthoritativeSession, GpuSessionConsumerKind};
use wgpu::util::DeviceExt;

use crate::{
    AdamWConfig, StageTrainableMask, TrainingError, TrainingSequence32, TrainingStepReceipt,
    CANDIDATE_RECORD_WORDS, TRAINING_SEQUENCE_TICKS,
};

const TRAINER_SCHEMA_VERSION: u32 = 1;
const HEADER_WORDS: usize = 64;
const HEADER_BYTES: u64 = (HEADER_WORDS * 4) as u64;
const READBACK_BYTES: u64 = 16;
const WORKGROUP_SIZE: u32 = 64;
const TRAINER_WGSL: &str = include_str!("../shaders/foundation_train.wgsl");

#[derive(Debug, Clone, Copy)]
struct PackedLayout {
    target_offsets: u32,
    incoming_ids: u32,
    source_offsets: u32,
    outgoing_ids: u32,
    synapse_records: u32,
    dynamics: u32,
    family_biases: u32,
    inputs: u32,
    targets: u32,
    target_weights: u32,
    candidate_records: u32,
    activations: u32,
    metabolic: u32,
    activation_gradients: u32,
    metabolic_gradients: u32,
    deltas: u32,
    weight_gradients: u32,
    candidate_logits: u32,
    metrics: u32,
    optimizer_v: u32,
    state_words: u64,
    gradient_words: u64,
    output_words: u64,
    training_words: u64,
}

struct TrainerPipelines {
    forward: wgpu::ComputePipeline,
    candidate_forward: wgpu::ComputePipeline,
    loss: wgpu::ComputePipeline,
    seed_candidate: wgpu::ComputePipeline,
    backward_local: wgpu::ComputePipeline,
    backward_sources: wgpu::ComputePipeline,
    recurrent_gradients: wgpu::ComputePipeline,
    candidate_gradients: wgpu::ComputePipeline,
    gradient_norm: wgpu::ComputePipeline,
    adamw: wgpu::ComputePipeline,
}

struct TrainerGpuState {
    active_header: wgpu::Buffer,
    step_headers: wgpu::Buffer,
    _meta: wgpu::Buffer,
    weights: wgpu::Buffer,
    _optimizer: wgpu::Buffer,
    training: wgpu::Buffer,
    state: wgpu::Buffer,
    gradients: wgpu::Buffer,
    outputs: wgpu::Buffer,
    mask: wgpu::Buffer,
    readback: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pipelines: TrainerPipelines,
    layout: PackedLayout,
}

/// Owns offline optimizer state beside, but never inside, the shared runtime.
pub struct FoundationTrainer {
    session: GpuAuthoritativeSession,
    phenotype: BrainPhenotype,
    source_foundation: FoundationWeightAsset,
    config: AdamWConfig,
    stage_mask: StageTrainableMask,
    optimizer_step: u32,
    gpu: TrainerGpuState,
}

impl FoundationTrainer {
    pub fn new_required(
        phenotype: BrainPhenotype,
        source_foundation: FoundationWeightAsset,
        stage_mask: StageTrainableMask,
        config: AdamWConfig,
    ) -> Result<Self, TrainingError> {
        let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())?;
        let session = GpuAuthoritativeSession::new(backend, GpuSessionConsumerKind::Training);
        Self::from_session(session, phenotype, source_foundation, stage_mask, config)
    }

    pub fn from_session(
        session: GpuAuthoritativeSession,
        phenotype: BrainPhenotype,
        source_foundation: FoundationWeightAsset,
        stage_mask: StageTrainableMask,
        config: AdamWConfig,
    ) -> Result<Self, TrainingError> {
        if session.authority().consumer() != GpuSessionConsumerKind::Training {
            return Err(ScaffoldContractError::NeuralBackendUnavailable.into());
        }
        config.validate()?;
        source_foundation.validate_against(&phenotype)?;
        stage_mask.validate_for(&phenotype)?;
        if phenotype.brain_class_id() != alife_core::BrainCapacityClass::N2048_ID {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        let gpu = {
            let (device, _) = session.backend().offline_training_device_queue()?;
            TrainerGpuState::new(device, &phenotype, source_foundation.weights(), &stage_mask)?
        };
        Ok(Self {
            session,
            phenotype,
            source_foundation,
            config,
            stage_mask,
            optimizer_step: 0,
            gpu,
        })
    }

    pub const fn phenotype(&self) -> &BrainPhenotype {
        &self.phenotype
    }

    pub const fn source_foundation(&self) -> &FoundationWeightAsset {
        &self.source_foundation
    }

    pub const fn optimizer_step(&self) -> u32 {
        self.optimizer_step
    }

    pub fn set_stage_mask(&mut self, mask: StageTrainableMask) -> Result<(), TrainingError> {
        mask.validate_for(&self.phenotype)?;
        let (_, queue) = self.session.backend().offline_training_device_queue()?;
        queue.write_buffer(&self.gpu.mask, 0, bytemuck::cast_slice(mask.words()));
        self.stage_mask = mask;
        Ok(())
    }

    pub fn train_step(
        &mut self,
        sequence: &TrainingSequence32,
    ) -> Result<TrainingStepReceipt, TrainingError> {
        sequence.validate_for(&self.phenotype)?;
        let next_step = self
            .optimizer_step
            .checked_add(1)
            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
        let training_words = pack_training_sequence(sequence, self.phenotype.neuron_count())?;
        if training_words.len() as u64 != self.gpu.layout.training_words {
            return Err(TrainingError::MalformedReadback);
        }
        let headers = build_step_headers(&self.phenotype, self.gpu.layout, self.config, next_step)?;
        let (device, queue) = self.session.backend().offline_training_device_queue()?;
        queue.write_buffer(&self.gpu.training, 0, bytemuck::cast_slice(&training_words));
        queue.write_buffer(&self.gpu.step_headers, 0, bytemuck::cast_slice(&headers));

        let neurons = self.phenotype.neuron_count();
        let synapses = self.phenotype.synapses().len() as u32;
        let total_steps =
            u32::from(self.phenotype.microstep_count()) * TRAINING_SEQUENCE_TICKS as u32;
        let neuron_groups = neurons.div_ceil(WORKGROUP_SIZE);
        let synapse_groups = synapses.div_ceil(WORKGROUP_SIZE);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("foundation-trainer-step"),
        });
        encoder.clear_buffer(&self.gpu.state, 0, None);
        encoder.clear_buffer(&self.gpu.gradients, 0, None);
        encoder.clear_buffer(&self.gpu.outputs, 0, None);
        self.record_forward(&mut encoder, total_steps, neuron_groups);
        dispatch(
            &mut encoder,
            &self.gpu.pipelines.candidate_forward,
            &self.gpu.bind_group,
            TRAINING_SEQUENCE_TICKS as u32,
            1,
            "foundation-trainer-candidate-forward",
        );
        dispatch(
            &mut encoder,
            &self.gpu.pipelines.loss,
            &self.gpu.bind_group,
            1,
            1,
            "foundation-trainer-loss-before",
        );
        encoder.copy_buffer_to_buffer(
            &self.gpu.outputs,
            u64::from(self.gpu.layout.metrics) * 4,
            &self.gpu.readback,
            0,
            4,
        );
        dispatch(
            &mut encoder,
            &self.gpu.pipelines.seed_candidate,
            &self.gpu.bind_group,
            neuron_groups,
            TRAINING_SEQUENCE_TICKS as u32,
            "foundation-trainer-seed-candidate-gradients",
        );
        self.record_backward(&mut encoder, total_steps, neuron_groups);
        dispatch(
            &mut encoder,
            &self.gpu.pipelines.recurrent_gradients,
            &self.gpu.bind_group,
            synapse_groups,
            1,
            "foundation-trainer-recurrent-weight-gradients",
        );
        dispatch(
            &mut encoder,
            &self.gpu.pipelines.candidate_gradients,
            &self.gpu.bind_group,
            synapse_groups,
            1,
            "foundation-trainer-candidate-weight-gradients",
        );
        dispatch(
            &mut encoder,
            &self.gpu.pipelines.gradient_norm,
            &self.gpu.bind_group,
            1,
            1,
            "foundation-trainer-gradient-norm",
        );
        encoder.copy_buffer_to_buffer(
            &self.gpu.outputs,
            u64::from(self.gpu.layout.metrics + 2) * 4,
            &self.gpu.readback,
            8,
            4,
        );
        dispatch(
            &mut encoder,
            &self.gpu.pipelines.adamw,
            &self.gpu.bind_group,
            synapse_groups,
            1,
            "foundation-trainer-adamw",
        );

        encoder.clear_buffer(&self.gpu.state, 0, None);
        encoder.clear_buffer(&self.gpu.outputs, 0, None);
        self.record_forward(&mut encoder, total_steps, neuron_groups);
        dispatch(
            &mut encoder,
            &self.gpu.pipelines.candidate_forward,
            &self.gpu.bind_group,
            TRAINING_SEQUENCE_TICKS as u32,
            1,
            "foundation-trainer-candidate-forward-after",
        );
        dispatch(
            &mut encoder,
            &self.gpu.pipelines.loss,
            &self.gpu.bind_group,
            1,
            1,
            "foundation-trainer-loss-after",
        );
        encoder.copy_buffer_to_buffer(
            &self.gpu.outputs,
            u64::from(self.gpu.layout.metrics) * 4,
            &self.gpu.readback,
            4,
            4,
        );

        let command_buffer = encoder.finish();
        let (sender, receiver) = mpsc::channel();
        command_buffer.map_buffer_on_submit(
            &self.gpu.readback,
            wgpu::MapMode::Read,
            0..READBACK_BYTES,
            move |result| {
                let _ = sender.send(result);
            },
        );
        let submission = queue.submit(Some(command_buffer));
        if device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .is_err()
            || receiver.recv().ok().and_then(Result::ok).is_none()
        {
            return Err(TrainingError::GpuSubmission);
        }
        let mapped = self.gpu.readback.slice(..READBACK_BYTES).get_mapped_range();
        let values = bytemuck::cast_slice::<u8, f32>(&mapped).to_vec();
        drop(mapped);
        self.gpu.readback.unmap();
        let [loss_before, loss_after, gradient_norm, _] = values.as_slice() else {
            return Err(TrainingError::MalformedReadback);
        };
        if !loss_before.is_finite()
            || !loss_after.is_finite()
            || !gradient_norm.is_finite()
            || *gradient_norm < 0.0
        {
            return Err(TrainingError::MalformedReadback);
        }
        self.optimizer_step = next_step;
        Ok(TrainingStepReceipt {
            optimizer_step: next_step,
            loss_before: *loss_before,
            loss_after: *loss_after,
            unclipped_gradient_norm: *gradient_norm,
            trained_weight_count: self.stage_mask.trainable_count() as u32,
        })
    }

    pub fn export_candidate(
        &self,
        training_stage: TrainingStageManifest,
    ) -> Result<FoundationWeightAsset, TrainingError> {
        let weights = self.read_weights()?;
        Ok(FoundationWeightAsset::from_trained_weights(
            &self.phenotype,
            weights,
            training_stage,
        )?)
    }

    pub fn read_weights(&self) -> Result<Vec<f32>, TrainingError> {
        let (device, queue) = self.session.backend().offline_training_device_queue()?;
        let bytes = (self.phenotype.synapses().len() * 4) as u64;
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("foundation-trainer-weight-readback"),
            size: bytes,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("foundation-trainer-weight-export"),
        });
        encoder.copy_buffer_to_buffer(&self.gpu.weights, 0, &staging, 0, bytes);
        let command_buffer = encoder.finish();
        let (sender, receiver) = mpsc::channel();
        command_buffer.map_buffer_on_submit(
            &staging,
            wgpu::MapMode::Read,
            0..bytes,
            move |result| {
                let _ = sender.send(result);
            },
        );
        let submission = queue.submit(Some(command_buffer));
        if device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .is_err()
            || receiver.recv().ok().and_then(Result::ok).is_none()
        {
            return Err(TrainingError::GpuSubmission);
        }
        let mapped = staging.slice(..bytes).get_mapped_range();
        let weights = bytemuck::cast_slice::<u8, f32>(&mapped).to_vec();
        drop(mapped);
        staging.unmap();
        if weights.len() != self.phenotype.synapses().len()
            || weights.iter().any(|weight| !weight.is_finite())
        {
            return Err(TrainingError::MalformedReadback);
        }
        Ok(weights)
    }

    fn record_forward(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        total_steps: u32,
        neuron_groups: u32,
    ) {
        for step in 0..total_steps {
            encoder.copy_buffer_to_buffer(
                &self.gpu.step_headers,
                u64::from(step) * HEADER_BYTES,
                &self.gpu.active_header,
                0,
                HEADER_BYTES,
            );
            dispatch(
                encoder,
                &self.gpu.pipelines.forward,
                &self.gpu.bind_group,
                neuron_groups,
                1,
                "foundation-trainer-forward-microstep",
            );
        }
    }

    fn record_backward(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        total_steps: u32,
        neuron_groups: u32,
    ) {
        for step in (0..total_steps).rev() {
            encoder.copy_buffer_to_buffer(
                &self.gpu.step_headers,
                u64::from(step) * HEADER_BYTES,
                &self.gpu.active_header,
                0,
                HEADER_BYTES,
            );
            dispatch(
                encoder,
                &self.gpu.pipelines.backward_local,
                &self.gpu.bind_group,
                neuron_groups,
                1,
                "foundation-trainer-backward-local",
            );
            dispatch(
                encoder,
                &self.gpu.pipelines.backward_sources,
                &self.gpu.bind_group,
                neuron_groups,
                1,
                "foundation-trainer-backward-sources",
            );
        }
    }
}

impl TrainerGpuState {
    fn new(
        device: &wgpu::Device,
        phenotype: &BrainPhenotype,
        initial_weights: &[f32],
        mask: &StageTrainableMask,
    ) -> Result<Self, TrainingError> {
        let (meta, layout) = pack_metadata_and_layout(phenotype)?;
        let total_steps = u32::from(phenotype.microstep_count()) * TRAINING_SEQUENCE_TICKS as u32;
        let header_capacity_words = usize::try_from(total_steps)
            .ok()
            .and_then(|steps| steps.checked_mul(HEADER_WORDS))
            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
        let zero_headers = vec![0_u32; header_capacity_words];
        let active_header = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("foundation-trainer-active-header"),
            size: HEADER_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let step_headers = buffer_init(
            device,
            "foundation-trainer-step-headers",
            &zero_headers,
            wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        );
        let meta_buffer = buffer_init(
            device,
            "foundation-trainer-meta",
            &meta,
            wgpu::BufferUsages::STORAGE,
        );
        let weights = buffer_init(
            device,
            "foundation-trainer-weights",
            initial_weights,
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        );
        let optimizer = zero_buffer(
            device,
            "foundation-trainer-optimizer",
            initial_weights.len() as u64 * 8,
            wgpu::BufferUsages::STORAGE,
        );
        let training = zero_buffer(
            device,
            "foundation-trainer-sequence",
            layout.training_words * 4,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let state = zero_buffer(
            device,
            "foundation-trainer-state",
            layout.state_words * 4,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let gradients = zero_buffer(
            device,
            "foundation-trainer-gradients",
            layout.gradient_words * 4,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let outputs = zero_buffer(
            device,
            "foundation-trainer-outputs",
            layout.output_words * 4,
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        );
        let mask_buffer = buffer_init(
            device,
            "foundation-trainer-stage-mask",
            mask.words(),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("foundation-trainer-step-readback"),
            size: READBACK_BYTES,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let bind_group_layout = create_bind_group_layout(device);
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("foundation-trainer-bind-group"),
            layout: &bind_group_layout,
            entries: &[
                entry(0, &active_header),
                entry(1, &meta_buffer),
                entry(2, &weights),
                entry(3, &optimizer),
                entry(4, &training),
                entry(5, &state),
                entry(6, &gradients),
                entry(7, &outputs),
                entry(8, &mask_buffer),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("foundation-trainer-pipeline-layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("foundation-trainer-wgsl"),
            source: wgpu::ShaderSource::Wgsl(TRAINER_WGSL.into()),
        });
        let pipelines = TrainerPipelines {
            forward: pipeline(device, &pipeline_layout, &shader, "forward_microstep"),
            candidate_forward: pipeline(
                device,
                &pipeline_layout,
                &shader,
                "forward_candidate_logits",
            ),
            loss: pipeline(device, &pipeline_layout, &shader, "reduce_loss"),
            seed_candidate: pipeline(
                device,
                &pipeline_layout,
                &shader,
                "seed_candidate_activation_gradients",
            ),
            backward_local: pipeline(device, &pipeline_layout, &shader, "backward_local"),
            backward_sources: pipeline(
                device,
                &pipeline_layout,
                &shader,
                "backward_recurrent_sources",
            ),
            recurrent_gradients: pipeline(
                device,
                &pipeline_layout,
                &shader,
                "recurrent_weight_gradients",
            ),
            candidate_gradients: pipeline(
                device,
                &pipeline_layout,
                &shader,
                "candidate_weight_gradients",
            ),
            gradient_norm: pipeline(device, &pipeline_layout, &shader, "reduce_gradient_norm"),
            adamw: pipeline(device, &pipeline_layout, &shader, "apply_adamw"),
        };
        Ok(Self {
            active_header,
            step_headers,
            _meta: meta_buffer,
            weights,
            _optimizer: optimizer,
            training,
            state,
            gradients,
            outputs,
            mask: mask_buffer,
            readback,
            bind_group,
            pipelines,
            layout,
        })
    }
}

fn pack_metadata_and_layout(
    phenotype: &BrainPhenotype,
) -> Result<(Vec<u32>, PackedLayout), TrainingError> {
    let neuron_count = phenotype.neuron_count() as usize;
    let synapse_count = phenotype.synapses().len();
    let mut incoming = phenotype
        .synapses()
        .iter()
        .enumerate()
        .filter(|(_, synapse)| matches!(synapse.kind(), CompiledSynapseKind::Recurrent))
        .map(|(index, synapse)| (synapse.target(), synapse.source(), index as u32))
        .collect::<Vec<_>>();
    incoming.sort_unstable();
    let mut outgoing = phenotype
        .synapses()
        .iter()
        .enumerate()
        .map(|(index, synapse)| (synapse.source(), synapse.target(), index as u32))
        .collect::<Vec<_>>();
    outgoing.sort_unstable();
    let mut meta = Vec::new();
    let target_offsets = push_offsets(
        &mut meta,
        neuron_count,
        incoming.iter().map(|entry| entry.0),
    )?;
    let incoming_ids = as_u32(meta.len())?;
    meta.extend(incoming.iter().map(|entry| entry.2));
    let source_offsets = push_offsets(
        &mut meta,
        neuron_count,
        outgoing.iter().map(|entry| entry.0),
    )?;
    let outgoing_ids = as_u32(meta.len())?;
    meta.extend(outgoing.iter().map(|entry| entry.2));
    let synapse_records = as_u32(meta.len())?;
    for synapse in phenotype.synapses() {
        let projection = phenotype
            .projections()
            .get(usize::from(synapse.route_index()))
            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
        let (kind, head, family, lane) = match synapse.kind() {
            CompiledSynapseKind::Recurrent => (1, 0, 0, 0),
            CompiledSynapseKind::Decoder(coordinate) => (
                2,
                coordinate.head().raw(),
                u32::from(coordinate.family().raw()),
                u32::from(coordinate.input_lane()),
            ),
        };
        let sign_policy = match projection.projection_type() {
            ProjectionType::LateralInhibition => 1,
            ProjectionType::Homeostatic | ProjectionType::MotorProposal => 2,
            _ => 0,
        };
        meta.extend_from_slice(&[
            synapse.source(),
            synapse.target(),
            kind,
            u32::from(projection.update_cadence().raw()),
            head,
            family,
            lane,
            sign_policy,
        ]);
    }
    let dynamics = as_u32(meta.len())?;
    for row in phenotype.neuron_dynamics() {
        meta.extend_from_slice(&[
            row.bias().to_bits(),
            row.leak().to_bits(),
            u32::from(row.activation().raw()),
            row.homeostatic_gain().to_bits(),
            row.metabolic_decay().to_bits(),
            0,
            0,
            0,
        ]);
    }
    let family_biases = as_u32(meta.len())?;
    let mut biases = [0.0_f32; 8];
    for family in phenotype.candidate_decoder().families() {
        biases[usize::from(family.family().raw())] = family.bias();
    }
    meta.extend(biases.into_iter().map(f32::to_bits));

    let ticks = TRAINING_SEQUENCE_TICKS as u64;
    let neurons = neuron_count as u64;
    let total_steps = ticks * u64::from(phenotype.microstep_count());
    let state_count = (total_steps + 1)
        .checked_mul(neurons)
        .ok_or(ScaffoldContractError::PhenotypeCompile)?;
    let inputs = 0;
    let targets = as_u32_u64(ticks * neurons)?;
    let target_weights = as_u32_u64(ticks * neurons * 2)?;
    let candidate_records = as_u32_u64(ticks * neurons * 3)?;
    let training_words = ticks
        .checked_mul(neurons * 3 + CANDIDATE_RECORD_WORDS as u64)
        .ok_or(ScaffoldContractError::PhenotypeCompile)?;
    let activations = 0;
    let metabolic = as_u32_u64(state_count)?;
    let state_words = state_count * 2;
    let activation_gradients = 0;
    let metabolic_gradients = as_u32_u64(state_count)?;
    let deltas = as_u32_u64(state_count * 2)?;
    let weight_gradients = as_u32_u64(state_count * 2 + total_steps * neurons)?;
    let gradient_words = state_count
        .checked_mul(2)
        .and_then(|value| value.checked_add(total_steps * neurons))
        .and_then(|value| value.checked_add(synapse_count as u64))
        .ok_or(ScaffoldContractError::PhenotypeCompile)?;
    let candidate_logits = 0;
    let metrics = TRAINING_SEQUENCE_TICKS as u32;
    let output_words = TRAINING_SEQUENCE_TICKS as u64 + 4;
    Ok((
        meta,
        PackedLayout {
            target_offsets,
            incoming_ids,
            source_offsets,
            outgoing_ids,
            synapse_records,
            dynamics,
            family_biases,
            inputs,
            targets,
            target_weights,
            candidate_records,
            activations,
            metabolic,
            activation_gradients,
            metabolic_gradients,
            deltas,
            weight_gradients,
            candidate_logits,
            metrics,
            optimizer_v: synapse_count as u32,
            state_words,
            gradient_words,
            output_words,
            training_words,
        },
    ))
}

fn push_offsets(
    out: &mut Vec<u32>,
    neuron_count: usize,
    keys: impl Iterator<Item = u32>,
) -> Result<u32, TrainingError> {
    let start = as_u32(out.len())?;
    let mut counts = vec![0_u32; neuron_count + 1];
    for key in keys {
        let slot = counts
            .get_mut(key as usize + 1)
            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
        *slot = slot
            .checked_add(1)
            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
    }
    for index in 1..counts.len() {
        counts[index] = counts[index]
            .checked_add(counts[index - 1])
            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
    }
    out.extend(counts);
    Ok(start)
}

fn pack_training_sequence(
    sequence: &TrainingSequence32,
    neuron_count: u32,
) -> Result<Vec<u32>, TrainingError> {
    let mut words = Vec::new();
    for tick in sequence.ticks() {
        words.extend(tick.encoded_inputs().iter().map(|value| value.to_bits()));
    }
    for tick in sequence.ticks() {
        words.extend(
            tick.target_activations()
                .iter()
                .map(|value| value.to_bits()),
        );
    }
    for tick in sequence.ticks() {
        words.extend(tick.target_weights().iter().map(|value| value.to_bits()));
    }
    for tick in sequence.ticks() {
        let start = words.len();
        if let Some(candidate) = tick.candidate_target() {
            words.extend_from_slice(&[
                1,
                u32::from(candidate.family.raw()),
                candidate.target_logit.to_bits(),
                candidate.loss_weight.to_bits(),
            ]);
            words.extend(candidate.features.0.into_iter().map(f32::to_bits));
        } else {
            words.extend(std::iter::repeat_n(0, 4 + CANDIDATE_FEATURE_COUNT));
        }
        words.extend(std::iter::repeat_n(
            0,
            CANDIDATE_RECORD_WORDS - (4 + CANDIDATE_FEATURE_COUNT),
        ));
        if words.len() - start != CANDIDATE_RECORD_WORDS {
            return Err(TrainingError::MalformedReadback);
        }
    }
    let expected = TRAINING_SEQUENCE_TICKS
        .checked_mul(neuron_count as usize * 3 + CANDIDATE_RECORD_WORDS)
        .ok_or(ScaffoldContractError::PhenotypeCompile)?;
    if words.len() != expected {
        return Err(TrainingError::MalformedReadback);
    }
    Ok(words)
}

fn build_step_headers(
    phenotype: &BrainPhenotype,
    layout: PackedLayout,
    config: AdamWConfig,
    optimizer_step: u32,
) -> Result<Vec<u32>, TrainingError> {
    let total_steps = u32::from(phenotype.microstep_count())
        .checked_mul(TRAINING_SEQUENCE_TICKS as u32)
        .ok_or(ScaffoldContractError::PhenotypeCompile)?;
    let recurrent_count = phenotype
        .synapses()
        .iter()
        .filter(|synapse| matches!(synapse.kind(), CompiledSynapseKind::Recurrent))
        .count() as u32;
    let mut headers = Vec::with_capacity(total_steps as usize * HEADER_WORDS);
    for step in 0..total_steps {
        let mut header = [0_u32; HEADER_WORDS];
        header[0] = TRAINER_SCHEMA_VERSION;
        header[1] = phenotype.neuron_count();
        header[2] = phenotype.synapses().len() as u32;
        header[3] = recurrent_count;
        header[4] = u32::from(phenotype.microstep_count());
        header[5] = TRAINING_SEQUENCE_TICKS as u32;
        header[6] = total_steps;
        header[7] = step;
        header[8] = step % u32::from(phenotype.microstep_count());
        header[9] = step / u32::from(phenotype.microstep_count());
        header[10] = optimizer_step;
        header[12] = layout.target_offsets;
        header[13] = layout.incoming_ids;
        header[14] = layout.source_offsets;
        header[15] = layout.outgoing_ids;
        header[16] = layout.synapse_records;
        header[17] = layout.dynamics;
        header[18] = layout.inputs;
        header[19] = layout.targets;
        header[20] = layout.target_weights;
        header[21] = layout.candidate_records;
        header[22] = layout.activations;
        header[23] = layout.metabolic;
        header[24] = layout.activation_gradients;
        header[25] = layout.metabolic_gradients;
        header[26] = layout.deltas;
        header[27] = layout.weight_gradients;
        header[28] = layout.candidate_logits;
        header[29] = layout.metrics;
        header[30] = layout.optimizer_v;
        header[31] = config.learning_rate.to_bits();
        header[32] = config.beta1.to_bits();
        header[33] = config.beta2.to_bits();
        header[34] = config.epsilon.to_bits();
        header[35] = config.weight_decay.to_bits();
        header[36] = config.gradient_clip.to_bits();
        header[38] = layout.family_biases;
        header[39] = CANDIDATE_FEATURE_COUNT as u32;
        headers.extend_from_slice(&header);
    }
    Ok(headers)
}

fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let mut entries = Vec::with_capacity(9);
    entries.push(wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: NonZeroU64::new(HEADER_BYTES),
        },
        count: None,
    });
    for binding in 1..=8 {
        entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage {
                    read_only: matches!(binding, 1 | 4 | 8),
                },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
    }
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("foundation-trainer-bind-group-layout"),
        entries: &entries,
    })
}

fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    entry_point: &'static str,
) -> wgpu::ComputePipeline {
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(entry_point),
        layout: Some(layout),
        module: shader,
        entry_point: Some(entry_point),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}

fn dispatch(
    encoder: &mut wgpu::CommandEncoder,
    pipeline: &wgpu::ComputePipeline,
    bind_group: &wgpu::BindGroup,
    x: u32,
    y: u32,
    label: &'static str,
) {
    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: Some(label),
        timestamp_writes: None,
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, bind_group, &[]);
    pass.dispatch_workgroups(x.max(1), y.max(1), 1);
}

fn entry(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buffer.as_entire_binding(),
    }
}

fn buffer_init<T: bytemuck::Pod>(
    device: &wgpu::Device,
    label: &'static str,
    contents: &[T],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytemuck::cast_slice(contents),
        usage,
    })
}

fn zero_buffer(
    device: &wgpu::Device,
    label: &'static str,
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

fn as_u32(value: usize) -> Result<u32, TrainingError> {
    u32::try_from(value)
        .map_err(|_| ScaffoldContractError::PhenotypeCompile)
        .map_err(Into::into)
}

fn as_u32_u64(value: u64) -> Result<u32, TrainingError> {
    u32::try_from(value)
        .map_err(|_| ScaffoldContractError::PhenotypeCompile)
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn training_shader_validates_and_contains_every_gradient_stage() {
        let module = naga::front::wgsl::parse_str(TRAINER_WGSL).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap();
        let names = module
            .entry_points
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>();
        for required in [
            "forward_microstep",
            "forward_candidate_logits",
            "reduce_loss",
            "seed_candidate_activation_gradients",
            "backward_local",
            "backward_recurrent_sources",
            "recurrent_weight_gradients",
            "candidate_weight_gradients",
            "reduce_gradient_norm",
            "apply_adamw",
        ] {
            assert!(names.contains(&required), "missing {required}");
        }
    }

    #[test]
    fn production_crates_do_not_embed_the_training_shader() {
        let game_manifest = include_str!("../../alife_game_app/Cargo.toml");
        let backend_manifest = include_str!("../../alife_gpu_backend/Cargo.toml");
        assert!(!game_manifest.contains("alife_training"));
        assert!(!backend_manifest.contains("alife_training"));
    }
}
