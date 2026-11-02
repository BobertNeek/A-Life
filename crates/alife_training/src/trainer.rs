//! Offline-only production-graph WGSL trainer implementation.

mod checkpoint_readback;

#[cfg(all(test, feature = "cpu-backend"))]
mod replay_packing_assay;

#[cfg(all(test, feature = "gpu-tests"))]
#[path = "trainer/packed_checkpoint_gpu_tests.rs"]
mod packed_checkpoint_gpu_tests;

#[cfg(all(test, feature = "gpu-tests"))]
#[path = "trainer/dynamic_dispatch_tests.rs"]
mod dynamic_dispatch_tests;

use std::{num::NonZeroU64, sync::mpsc};

use alife_core::{
    BrainPhenotype, CompiledSynapseKind, FoundationWeightAsset, ProjectionType,
    ScaffoldContractError, SpeechDecoderLayoutV1, TrainingStageManifest, CANDIDATE_FEATURE_COUNT,
};
use alife_gpu_backend::{GpuClosedLoopBackend, GpuRuntimeProfile};
use alife_runtime::{GpuAuthoritativeSession, GpuSessionConsumerKind};
use wgpu::util::DeviceExt;

use crate::{
    AdamWConfig, SequenceEvaluation, StageTrainableMask, TrainingError, TrainingSequence,
    TrainingSequence32, TrainingStepReceipt, TrainingStepStatistics, CANDIDATE_RECORD_WORDS,
    TRAINING_SEQUENCE_TICKS,
};

const TRAINER_SCHEMA_VERSION: u32 = 1;
const HEADER_WORDS: usize = 64;
const HEADER_BYTES: u64 = (HEADER_WORDS * 4) as u64;
const READBACK_BYTES: u64 = 16;
const WORKGROUP_SIZE: u32 = 64;
const TRAINER_WGSL: &str = include_str!("../shaders/foundation_train.wgsl");

#[derive(Debug, Clone, Copy)]
struct PackedLayout {
    ticks: u32,
    replay: bool,
    candidate_capacity: u32,
    context: u32,
    route_count: u32,
    dendritic_offsets: u32,
    dendritic_branches: u32,
    dendritic_inputs: u32,
    dendritic_source_offsets: u32,
    dendritic_source_branches: u32,
    initial: u32,
    activity_ema: u32,
    adjoints: u32,
    decoder_offsets: u32,
    decoder_ids: u32,
    memory_gain: f32,
    burn_in_steps: u32,
    memory_raw: u32,
    cognitive_raw: u32,
    accumulated_weights: u32,
    gradient_scale: f32,
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
    speech_logits: u32,
    metrics: u32,
    speech_target_start: u32,
    replay_speech_targets: u32,
    speech_offsets: u32,
    optimizer_v: u32,
    optimizer_age: u32,
    state_words: u64,
    gradient_words: u64,
    output_words: u64,
    training_words: u64,
}

impl PackedLayout {
    fn refresh_replay_parameters(&mut self, sequence: &TrainingSequence, microsteps: u8) {
        self.burn_in_steps = sequence.burn_in_ticks as u32 * u32::from(microsteps);
        self.memory_gain = sequence.memory_candidate_gain;
    }
}

type ReplayShape = (usize, alife_core::DendriticBranchSet);

fn replay_shapes_equal(previous: &ReplayShape, next: &ReplayShape) -> bool {
    let (previous_branches, next_branches) = (previous.1.branches(), next.1.branches());
    previous.0 == next.0
        && previous_branches.len() == next_branches.len()
        && previous_branches.iter().zip(next_branches).all(|(a, b)| {
            a.target == b.target
                && a.threshold.to_bits() == b.threshold.to_bits()
                && a.output_gain.to_bits() == b.output_gain.to_bits()
                && a.inputs.len() == b.inputs.len()
                && a.inputs
                    .iter()
                    .zip(&b.inputs)
                    .all(|(a, b)| a.source == b.source && a.weight.to_bits() == b.weight.to_bits())
        })
}

#[derive(Clone)]
struct TrainerPipelines {
    bind_group_layout: wgpu::BindGroupLayout,
    initialize: wgpu::ComputePipeline,
    forward: wgpu::ComputePipeline,
    candidate_forward: wgpu::ComputePipeline,
    speech_forward: wgpu::ComputePipeline,
    loss: wgpu::ComputePipeline,
    seed_candidate: wgpu::ComputePipeline,
    seed_speech: wgpu::ComputePipeline,
    backward_local: wgpu::ComputePipeline,
    backward_sources: wgpu::ComputePipeline,
    recurrent_gradients: wgpu::ComputePipeline,
    candidate_gradients: wgpu::ComputePipeline,
    speech_gradients: wgpu::ComputePipeline,
    gradient_norm: wgpu::ComputePipeline,
    validate_adamw: wgpu::ComputePipeline,
    validate_accumulation: wgpu::ComputePipeline,
    accumulate: wgpu::ComputePipeline,
    adamw: wgpu::ComputePipeline,
}

struct TrainerGpuState {
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
    replay_shape: Option<ReplayShape>,
    accumulated_gradients: wgpu::Buffer,
    accumulation: Option<(u32, u32)>,
    replay_speech_active: bool,
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
        if phenotype.brain_class_id() != alife_core::BrainCapacityClass::N2048_ID
            || phenotype
                .projections()
                .iter()
                .any(|p| p.delay_microsteps() != 0)
        {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        let gpu = {
            let (device, _) = session.backend().offline_training_device_queue()?;
            let genetic = phenotype
                .synapses()
                .iter()
                .map(|s| s.genetic_weight())
                .collect::<Vec<_>>();
            TrainerGpuState::new(device, &phenotype, &genetic, &stage_mask)?
        };
        let (device, _) = session.backend().offline_training_device_queue()?;
        let accumulated_gradients = zero_buffer(
            device,
            "foundation-batch-gradients",
            phenotype.synapses().len() as u64 * 4,
            wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        );
        Ok(Self {
            session,
            phenotype,
            source_foundation,
            config,
            stage_mask,
            optimizer_step: 0,
            gpu,
            replay_shape: None,
            accumulated_gradients,
            accumulation: None,
            replay_speech_active: false,
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

    pub fn hardware_receipt(&self) -> &alife_gpu_backend::GpuHardwareReceipt {
        self.session.backend().hardware_receipt()
    }

    pub fn session(&self) -> &GpuAuthoritativeSession {
        &self.session
    }

    /// Packed replay buffer bound, also covering the initial trainer allocation.
    /// The caller can reject a window before allocating its device scratch.
    /// Persistent objectives/accumulation and overlapping old/new allocations
    /// must be budgeted separately by the orchestration layer.
    pub fn replay_allocation_bytes(
        phenotype: &BrainPhenotype,
        sequence: &TrainingSequence,
    ) -> Result<u64, TrainingError> {
        sequence.validate_for(phenotype)?;
        let bytes = |meta_words: usize, layout: PackedLayout| {
            let words = layout.state_words
                + layout.gradient_words
                + layout.output_words
                + layout.training_words
                + meta_words as u64
                + phenotype.synapses().len() as u64 * 5;
            words * 4
                + u64::from(phenotype.microstep_count()) * u64::from(layout.ticks) * HEADER_BYTES
                + READBACK_BYTES
        };
        let (meta, replay) = pack_metadata_and_layout(phenotype, Some(sequence))?;
        let (initial_meta, initial) = pack_metadata_and_layout(phenotype, None)?;
        Ok(bytes(meta.len(), replay).max(bytes(initial_meta.len(), initial)))
    }

    /// Largest storage binding needed by a replay shape, checked before upload.
    pub fn replay_storage_binding_bytes(
        phenotype: &BrainPhenotype,
        sequence: &TrainingSequence,
    ) -> Result<u64, TrainingError> {
        sequence.validate_for(phenotype)?;
        let (meta, layout) = pack_metadata_and_layout(phenotype, Some(sequence))?;
        Ok([
            layout.state_words,
            layout.gradient_words,
            layout.output_words,
            layout.training_words,
            meta.len() as u64,
            phenotype.synapses().len() as u64 * 3,
        ]
        .into_iter()
        .max()
        .unwrap_or(0)
            * 4)
    }

    /// Starts an offline cohort under an exactly compiled export of the current
    /// GPU weights. This never replaces genetics in a living organism. Graph,
    /// dynamics and learning coordinates must be unchanged, so Adam moments,
    /// per-weight ages, stage mask and optimizer step remain meaningful.
    ///
    /// Prepared replay is invalidated: collect fresh organisms with this asset,
    /// then prepare their sequence and rebind the existing PpoTrainingState.
    /// Keep that value state to preserve its learned head and optimizer moments.
    pub fn rebind_for_next_cohort(
        &mut self,
        next_phenotype: BrainPhenotype,
        next_asset: FoundationWeightAsset,
    ) -> Result<(), TrainingError> {
        if self.accumulation.is_some() {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        next_phenotype.validate_against(&alife_core::BrainCapacityClass::n2048())?;
        next_asset.validate_against(&next_phenotype)?;
        self.stage_mask.validate_for(&next_phenotype)?;
        let previous = &self.phenotype;
        let next = &next_phenotype;
        let binding = next
            .foundation_abi_selection()
            .canonical_v2()
            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
        if previous.foundation_abi_selection().canonical_v2().is_none()
            || binding.foundation_payload_digest() != Some(next_asset.digest())
            || previous.schema_version() != next.schema_version()
            || previous.brain_class_id() != next.brain_class_id()
            || previous.neuron_count() != next.neuron_count()
            || previous.microstep_count() != next.microstep_count()
            || previous.sensor_profile() != next.sensor_profile()
            || previous.lobe_layout() != next.lobe_layout()
            || previous.language_codebook() != next.language_codebook()
            || previous.cognitive_architecture() != next.cognitive_architecture()
            || previous.projections() != next.projections()
            || previous.neuron_dynamics() != next.neuron_dynamics()
            || previous.sensor_encoder() != next.sensor_encoder()
            || previous.candidate_decoder() != next.candidate_decoder()
            || previous.speech_decoder() != next.speech_decoder()
            || previous.memory_decoder() != next.memory_decoder()
            || previous.cognitive_decoder() != next.cognitive_decoder()
            || previous.cognitive_channel_plan() != next.cognitive_channel_plan()
            || previous.plasticity_receptors() != next.plasticity_receptors()
            || previous.replay_capture_plan() != next.replay_capture_plan()
            || previous.sleep_consolidation_plan() != next.sleep_consolidation_plan()
            || previous.plasticity_plan_digest() != next.plasticity_plan_digest()
            || previous.persistent_address_map() != next.persistent_address_map()
            || previous.route_abi_digest() != next.route_abi_digest()
            || previous.plasticity_abi_digest() != next.plasticity_abi_digest()
            || previous.budgets() != next.budgets()
            || previous.synapses().len() != next.synapses().len()
            || previous
                .synapses()
                .iter()
                .zip(next.synapses())
                .any(|(a, b)| {
                    a.source() != b.source()
                        || a.target() != b.target()
                        || a.alpha().to_bits() != b.alpha().to_bits()
                        || a.route_index() != b.route_index()
                        || a.receptor_index() != b.receptor_index()
                        || a.kind() != b.kind()
                })
        {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        let weights = self.read_weights()?;
        if weights
            .iter()
            .zip(next_asset.weights())
            .zip(next.synapses())
            .any(|((actual, asset), compiled)| {
                actual.to_bits() != asset.to_bits()
                    || actual.to_bits() != compiled.genetic_weight().to_bits()
            })
        {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        self.phenotype = next_phenotype;
        self.source_foundation = next_asset;
        self.replay_shape = None;
        Ok(())
    }

    /// Replaces only replay scratch storage; genetic weights and both AdamW
    /// moments remain on GPU. Existing objective bindings must be recreated.
    pub fn prepare_replay(&mut self, sequence: &TrainingSequence) -> Result<(), TrainingError> {
        sequence.validate_for(&self.phenotype)?;
        self.replay_speech_active = false;
        let shape = (sequence.ticks.len(), sequence.initial.dendrites.clone());
        if self
            .replay_shape
            .as_ref()
            .is_some_and(|previous| replay_shapes_equal(previous, &shape))
        {
            self.gpu
                .layout
                .refresh_replay_parameters(sequence, self.phenotype.microstep_count());
            let words =
                pack_replay_sequence(sequence, self.gpu.layout, self.phenotype.neuron_count());
            let headers = build_step_headers(
                &self.phenotype,
                self.gpu.layout,
                self.config,
                self.optimizer_step
                    .checked_add(1)
                    .ok_or(ScaffoldContractError::PhenotypeCompile)?,
            )?;
            let (_, queue) = self.session.backend().offline_training_device_queue()?;
            queue.write_buffer(&self.gpu.training, 0, bytemuck::cast_slice(&words));
            queue.write_buffer(&self.gpu.step_headers, 0, bytemuck::cast_slice(&headers));
            return Ok(());
        }
        let (device, queue) = self.session.backend().offline_training_device_queue()?;
        let zeros = vec![0.0; self.phenotype.synapses().len()];
        let next = TrainerGpuState::new_for_replay(
            device,
            &self.phenotype,
            &zeros,
            &self.stage_mask,
            Some(sequence),
            Some(&self.gpu.pipelines),
        )?;
        let words = pack_replay_sequence(sequence, next.layout, self.phenotype.neuron_count());
        let headers = build_step_headers(
            &self.phenotype,
            next.layout,
            self.config,
            self.optimizer_step
                .checked_add(1)
                .ok_or(ScaffoldContractError::PhenotypeCompile)?,
        )?;
        queue.write_buffer(&next.training, 0, bytemuck::cast_slice(&words));
        queue.write_buffer(&next.step_headers, 0, bytemuck::cast_slice(&headers));
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("foundation-replay-reallocate"),
        });
        encoder.copy_buffer_to_buffer(
            &self.gpu.weights,
            0,
            &next.weights,
            0,
            zeros.len() as u64 * 4,
        );
        encoder.copy_buffer_to_buffer(
            &self.gpu._optimizer,
            0,
            &next._optimizer,
            0,
            zeros.len() as u64 * 12,
        );
        queue.submit(Some(encoder.finish()));
        self.gpu = next;
        self.replay_shape = Some(shape);
        Ok(())
    }

    pub fn replay_output_buffer(&self) -> &wgpu::Buffer {
        &self.gpu.outputs
    }

    pub fn upload_replay_speech_targets(
        &mut self,
        targets: &[Option<crate::ReplaySpeechTarget>],
    ) -> Result<(), TrainingError> {
        if !self.gpu.layout.replay || targets.len() != self.gpu.layout.ticks as usize {
            return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
        }
        let mut words = vec![0_u32; targets.len() * 384];
        let scales = crate::replay_speech_loss_scales(targets);
        for (tick, target) in targets.iter().enumerate() {
            if let Some(target) = target {
                target.validate()?;
                if tick
                    < (self.gpu.layout.burn_in_steps / u32::from(self.phenotype.microstep_count()))
                        as usize
                {
                    return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
                }
                for step in 0..target.len() {
                    for (output, value) in target.step_logits(step).into_iter().enumerate() {
                        words[tick * 384 + step * 64 + output * 2] = value.to_bits();
                        words[tick * 384 + step * 64 + output * 2 + 1] = (target
                            .output_weight(output)
                            * scales[usize::from(target.token.is_none())])
                        .to_bits();
                    }
                }
            }
        }
        let (_, queue) = self.session.backend().offline_training_device_queue()?;
        queue.write_buffer(
            &self.gpu.training,
            u64::from(self.gpu.layout.replay_speech_targets) * 4,
            bytemuck::cast_slice(&words),
        );
        self.replay_speech_active = targets.iter().any(Option::is_some);
        Ok(())
    }
    pub fn replay_state_buffer(&self) -> &wgpu::Buffer {
        &self.gpu.state
    }

    /// Current logical output/state extents in 32-bit words. Backing buffers
    /// may retain larger capacity. Stale or unprepared replay is rejected.
    pub fn replay_buffer_word_lengths(&self) -> Result<(u64, u64), TrainingError> {
        if !self.gpu.layout.replay || self.replay_shape.is_none() {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        Ok((self.gpu.layout.output_words, self.gpu.layout.state_words))
    }

    /// Diagnostic readback of the last replay, padded to the production
    /// candidate bound for each tick. Does not perform another forward pass.
    pub fn read_replay_logits(&self) -> Result<Vec<f32>, TrainingError> {
        if !self.gpu.layout.replay || self.replay_shape.is_none() {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        self.read_float_buffer(
            &self.gpu.outputs,
            u64::from(self.gpu.layout.candidate_logits) * 4,
            (self.gpu.layout.ticks * self.gpu.layout.candidate_capacity) as usize,
        )
    }

    /// Read only the speech outputs from the last forward pass for telemetry.
    pub(crate) fn read_replay_speech_diagnostics(
        &self,
        targets: &[Option<crate::ReplaySpeechTarget>],
    ) -> Result<crate::ReplaySpeechPayloadDiagnostics, TrainingError> {
        if !self.gpu.layout.replay || targets.len() != self.gpu.layout.ticks as usize {
            return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
        }
        if !self.replay_speech_active {
            return Ok(crate::ReplaySpeechPayloadDiagnostics::default());
        }
        let logits = self.read_float_buffer(
            &self.gpu.outputs,
            u64::from(self.gpu.layout.speech_logits) * 4,
            targets.len() * 192,
        )?;
        crate::replay_speech_payload_diagnostics(&logits, targets)
    }

    /// Runs the supplied frozen production contexts on the trainer GPU and
    /// returns numerical boundary receipts. This is conditional replay, not a
    /// claim that memory, chemistry or plasticity trajectories were generated here.
    pub fn evaluate_replay(
        &mut self,
        sequence: &TrainingSequence,
    ) -> Result<crate::TrainingReplayEvaluation, TrainingError> {
        self.prepare_replay(sequence)?;
        let ticks = sequence.ticks.len();
        let neurons = self.phenotype.neuron_count() as usize;
        let candidates = self.gpu.layout.candidate_capacity as usize;
        let logits_count = ticks * candidates;
        let state_count = ticks * neurons;
        let word_count = logits_count + state_count * 3;
        let (device, queue) = self.session.backend().offline_training_device_queue()?;
        let compact = zero_buffer(
            device,
            "replay-numerical-receipt",
            word_count as u64 * 4,
            wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        );
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("replay-numerical-evaluation"),
        });
        self.encode_replay_forward(&mut encoder)?;
        encoder.copy_buffer_to_buffer(
            &self.gpu.outputs,
            u64::from(self.gpu.layout.candidate_logits) * 4,
            &compact,
            0,
            logits_count as u64 * 4,
        );
        for (component, base) in [
            self.gpu.layout.activations,
            self.gpu.layout.activity_ema,
            self.gpu.layout.metabolic,
        ]
        .into_iter()
        .enumerate()
        {
            for tick in 0..ticks {
                let source = base as usize
                    + (tick + 1) * usize::from(self.phenotype.microstep_count()) * neurons;
                let target = logits_count + component * state_count + tick * neurons;
                encoder.copy_buffer_to_buffer(
                    &self.gpu.state,
                    source as u64 * 4,
                    &compact,
                    target as u64 * 4,
                    neurons as u64 * 4,
                );
            }
        }
        queue.submit(Some(encoder.finish()));
        let values = self.read_float_buffer(&compact, 0, word_count)?;
        let component = |index: usize| {
            values[logits_count + index * state_count..logits_count + (index + 1) * state_count]
                .chunks_exact(neurons)
                .map(<[f32]>::to_vec)
                .collect()
        };
        Ok(crate::TrainingReplayEvaluation {
            candidate_logits: sequence
                .ticks
                .iter()
                .enumerate()
                .map(|(tick, row)| {
                    values[tick * candidates..tick * candidates + row.candidates.len()].to_vec()
                })
                .collect(),
            final_activations: component(0),
            final_activity_ema: component(1),
            final_metabolic_load: component(2),
        })
    }

    /// Offsets for the objective exclude burn-in samples; features are detached
    /// final activations, while candidate adjoints enter the recurrent backbone.
    pub fn replay_rows(&self) -> Result<Vec<crate::PpoReplayRow>, TrainingError> {
        let mut rows = self.replay_all_rows()?;
        rows.drain(
            ..(self.gpu.layout.burn_in_steps / u32::from(self.phenotype.microstep_count()))
                as usize,
        );
        Ok(rows)
    }

    /// Boundary features for value prediction, including burn-in rows. This
    /// does not add an unobserved next-state/bootstrap row.
    pub fn replay_all_rows(&self) -> Result<Vec<crate::PpoReplayRow>, TrainingError> {
        if !self.gpu.layout.replay || self.replay_shape.is_none() {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        let steps = u32::from(self.phenotype.microstep_count());
        Ok((0..self.gpu.layout.ticks)
            .map(|tick| crate::PpoReplayRow {
                logits_word_offset: self.gpu.layout.candidate_logits
                    + tick * self.gpu.layout.candidate_capacity,
                features_word_offset: self.gpu.layout.activations
                    + (tick + 1) * steps * self.phenotype.neuron_count(),
            })
            .collect())
    }

    pub fn encode_replay_forward(
        &self,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), TrainingError> {
        if !self.gpu.layout.replay || self.replay_shape.is_none() {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        encoder.clear_buffer(&self.gpu.state, 0, None);
        encoder.clear_buffer(&self.gpu.outputs, 0, None);
        let groups = self.phenotype.neuron_count().div_ceil(WORKGROUP_SIZE);
        dispatch(
            encoder,
            &self.gpu.pipelines.initialize,
            &self.gpu.bind_group,
            groups,
            1,
            "replay-initial-state",
        );
        self.record_forward(
            encoder,
            self.gpu.layout.ticks * u32::from(self.phenotype.microstep_count()),
            groups,
        );
        dispatch(
            encoder,
            &self.gpu.pipelines.candidate_forward,
            &self.gpu.bind_group,
            1,
            self.gpu.layout.ticks,
            "replay-all-candidate-logits",
        );
        Ok(())
    }

    fn encode_replay_gradients(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        adjoints: &wgpu::Buffer,
        adjoint_offset: u64,
    ) -> Result<(), TrainingError> {
        let steps = u32::from(self.phenotype.microstep_count());
        let burn_in = self.gpu.layout.burn_in_steps / steps;
        let count = (self.gpu.layout.ticks - burn_in) * self.gpu.layout.candidate_capacity;
        if !adjoint_offset.is_multiple_of(4)
            || adjoint_offset
                .checked_add(u64::from(count) * 4)
                .is_none_or(|end| end > adjoints.size())
        {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        encoder.clear_buffer(&self.gpu.gradients, 0, None);
        encoder.copy_buffer_to_buffer(
            adjoints,
            adjoint_offset,
            &self.gpu.gradients,
            u64::from(self.gpu.layout.adjoints + burn_in * self.gpu.layout.candidate_capacity) * 4,
            u64::from(count) * 4,
        );
        let groups = self.phenotype.neuron_count().div_ceil(WORKGROUP_SIZE);
        let weight_groups = (self.phenotype.synapses().len() as u32).div_ceil(WORKGROUP_SIZE);
        dispatch(
            encoder,
            &self.gpu.pipelines.seed_candidate,
            &self.gpu.bind_group,
            groups,
            self.gpu.layout.ticks,
            "replay-seed-policy-adjoints",
        );
        if self.replay_speech_active {
            dispatch(
                encoder,
                &self.gpu.pipelines.speech_forward,
                &self.gpu.bind_group,
                self.gpu.layout.ticks,
                1,
                "replay-speech-forward",
            );
            dispatch(
                encoder,
                &self.gpu.pipelines.seed_speech,
                &self.gpu.bind_group,
                groups,
                self.gpu.layout.ticks,
                "replay-speech-adjoints",
            );
        }
        self.record_backward(encoder, self.gpu.layout.ticks * steps, groups);
        dispatch(
            encoder,
            &self.gpu.pipelines.recurrent_gradients,
            &self.gpu.bind_group,
            weight_groups,
            1,
            "replay-recurrent-gradients",
        );
        dispatch(
            encoder,
            &self.gpu.pipelines.candidate_gradients,
            &self.gpu.bind_group,
            weight_groups,
            1,
            "replay-decoder-gradients",
        );
        if self.replay_speech_active {
            dispatch(
                encoder,
                &self.gpu.pipelines.speech_gradients,
                &self.gpu.bind_group,
                weight_groups,
                1,
                "replay-speech-gradients",
            );
        }
        dispatch(
            encoder,
            &self.gpu.pipelines.gradient_norm,
            &self.gpu.bind_group,
            1,
            1,
            "replay-gradient-norm",
        );
        Ok(())
    }

    /// GPU-only forward/backward probe for sampled finite differences. The
    /// host reduces already-computed logits into a scalar; it never evaluates
    /// neurons. Neither genetic weights nor optimizer state is written.
    pub fn probe_replay_gradients(
        &mut self,
        sequence: &TrainingSequence,
        adjoints: &[Vec<f32>],
    ) -> Result<crate::TrainingGradientProbe, TrainingError> {
        self.probe_replay_gradients_with_speech(
            sequence,
            adjoints,
            &vec![None; sequence.ticks.len()],
        )
    }

    pub fn probe_replay_gradients_with_speech(
        &mut self,
        sequence: &TrainingSequence,
        adjoints: &[Vec<f32>],
        speech: &[Option<crate::ReplaySpeechTarget>],
    ) -> Result<crate::TrainingGradientProbe, TrainingError> {
        sequence.validate_for(&self.phenotype)?;
        if adjoints.len() != sequence.ticks.len() - sequence.burn_in_ticks
            || adjoints
                .iter()
                .zip(&sequence.ticks[sequence.burn_in_ticks..])
                .any(|(a, t)| a.len() != t.candidates.len() || a.iter().any(|v| !v.is_finite()))
        {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        self.prepare_replay(sequence)?;
        self.upload_replay_speech_targets(speech)?;
        let mut packed = vec![0.0; adjoints.len() * alife_core::MAX_ACTION_CANDIDATES];
        for (row, values) in adjoints.iter().enumerate() {
            packed[row * alife_core::MAX_ACTION_CANDIDATES
                ..row * alife_core::MAX_ACTION_CANDIDATES + values.len()]
                .copy_from_slice(values);
        }
        let (device, queue) = self.session.backend().offline_training_device_queue()?;
        let adjoint_buffer = buffer_init(
            device,
            "gradient-probe-adjoints",
            &packed,
            wgpu::BufferUsages::COPY_SRC,
        );
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("replay-gradient-probe"),
        });
        self.encode_replay_forward(&mut encoder)?;
        self.encode_replay_gradients(&mut encoder, &adjoint_buffer, 0)?;
        queue.submit(Some(encoder.finish()));
        let logits = self.read_replay_logits()?;
        let mut objective = 0.0f64;
        for (row, values) in adjoints.iter().enumerate() {
            for (candidate, adjoint) in values.iter().enumerate() {
                objective += f64::from(*adjoint)
                    * f64::from(
                        logits[(row + sequence.burn_in_ticks) * alife_core::MAX_ACTION_CANDIDATES
                            + candidate],
                    );
            }
        }
        if self.replay_speech_active {
            let logits = self.read_float_buffer(
                &self.gpu.outputs,
                u64::from(self.gpu.layout.speech_logits) * 4,
                sequence.ticks.len() * 192,
            )?;
            let scales = crate::replay_speech_loss_scales(speech);
            for (tick, label) in speech.iter().enumerate() {
                if let Some(label) = label {
                    for step in 0..label.len() {
                        for (output, target) in label.step_logits(step).iter().enumerate() {
                            let error = f64::from(logits[(tick * 6 + step) * 32 + output] - target);
                            objective += f64::from(
                                label.output_weight(output)
                                    * scales[usize::from(label.token.is_none())],
                            ) * error
                                * error;
                        }
                    }
                }
            }
        }
        let gradients = self.read_float_buffer(
            &self.gpu.gradients,
            u64::from(self.gpu.layout.weight_gradients) * 4,
            self.phenotype.synapses().len(),
        )?;
        if !objective.is_finite() {
            return Err(TrainingError::MalformedReadback);
        }
        Ok(crate::TrainingGradientProbe {
            objective,
            gradients,
        })
    }

    /// Called only after validating objective metrics and the KL gate. Adjoints
    /// cover replay_rows(), not burn-in. Value-head updates may share encoder.
    /// Submits and waits before advancing the optimizer counter.
    pub fn submit_replay_update(
        &mut self,
        mut encoder: wgpu::CommandEncoder,
        adjoints: &wgpu::Buffer,
        adjoint_offset: u64,
    ) -> Result<u32, TrainingError> {
        self.validate_objective_mask(true)?;
        if self.accumulation.is_some() || !self.gpu.layout.replay || self.replay_shape.is_none() {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        let next_step = self
            .optimizer_step
            .checked_add(1)
            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
        let headers = build_step_headers(&self.phenotype, self.gpu.layout, self.config, next_step)?;
        let (device, queue) = self.session.backend().offline_training_device_queue()?;
        queue.write_buffer(&self.gpu.step_headers, 0, bytemuck::cast_slice(&headers));
        self.encode_replay_gradients(&mut encoder, adjoints, adjoint_offset)?;
        let weight_groups = (self.phenotype.synapses().len() as u32).div_ceil(WORKGROUP_SIZE);
        dispatch(
            &mut encoder,
            &self.gpu.pipelines.validate_adamw,
            &self.gpu.bind_group,
            1,
            1,
            "validate-adamw-update",
        );
        dispatch(
            &mut encoder,
            &self.gpu.pipelines.adamw,
            &self.gpu.bind_group,
            weight_groups,
            1,
            "replay-adamw",
        );
        let submission = queue.submit(Some(encoder.finish()));
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .map_err(|_| TrainingError::GpuSubmission)?;
        let update = self.read_float_buffer(
            &self.gpu.outputs,
            u64::from(self.gpu.layout.metrics + 2) * 4,
            2,
        )?;
        if update[0] < 0.0 || update[1] != 1.0 {
            return Err(TrainingError::MalformedReadback);
        }
        self.optimizer_step = next_step;
        Ok(next_step)
    }

    fn validate_objective_mask(&self, replay: bool) -> Result<(), TrainingError> {
        for (index, synapse) in self.phenotype.synapses().iter().enumerate() {
            if !self.stage_mask.is_trainable(index) {
                continue;
            }
            if let CompiledSynapseKind::Decoder(coordinate) = synapse.kind() {
                let head = coordinate.head().raw();
                if !replay && (head == 2 || head == 4) {
                    return Err(ScaffoldContractError::PhenotypeCompile.into());
                }
            }
        }
        Ok(())
    }

    /// Starts a synchronous effective batch. No optimizer step occurs until
    /// exactly expected_sequences successful accumulations have completed.
    pub fn begin_gradient_accumulation(
        &mut self,
        expected_sequences: u32,
    ) -> Result<(), TrainingError> {
        self.validate_objective_mask(true)?;
        if expected_sequences == 0 || self.accumulation.is_some() {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        let (device, queue) = self.session.backend().offline_training_device_queue()?;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("begin-gradient-batch"),
        });
        encoder.clear_buffer(&self.accumulated_gradients, 0, None);
        queue.submit(Some(encoder.finish()));
        self.accumulation = Some((expected_sequences, 0));
        Ok(())
    }

    pub fn cancel_gradient_accumulation(&mut self) {
        self.accumulation = None;
    }

    /// Adds a GPU-computed sequence gradient with the caller's explicit batch
    /// normalization. Scratch reallocations preserve this persistent buffer.
    pub fn accumulate_replay_gradients(
        &mut self,
        mut encoder: wgpu::CommandEncoder,
        adjoints: &wgpu::Buffer,
        adjoint_offset: u64,
        scale: f32,
    ) -> Result<u32, TrainingError> {
        let (expected, count) = self
            .accumulation
            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
        if !self.gpu.layout.replay
            || self.replay_shape.is_none()
            || count >= expected
            || !scale.is_finite()
            || scale <= 0.0
        {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        let mut layout = self.gpu.layout;
        layout.gradient_scale = scale;
        let headers = build_step_headers(
            &self.phenotype,
            layout,
            self.config,
            self.optimizer_step
                .checked_add(1)
                .ok_or(ScaffoldContractError::PhenotypeCompile)?,
        )?;
        let (_, queue) = self.session.backend().offline_training_device_queue()?;
        queue.write_buffer(&self.gpu.step_headers, 0, bytemuck::cast_slice(&headers));
        self.encode_replay_gradients(&mut encoder, adjoints, adjoint_offset)?;
        let bytes = self.phenotype.synapses().len() as u64 * 4;
        encoder.copy_buffer_to_buffer(
            &self.accumulated_gradients,
            0,
            &self.gpu.gradients,
            u64::from(layout.accumulated_weights) * 4,
            bytes,
        );
        dispatch(
            &mut encoder,
            &self.gpu.pipelines.validate_accumulation,
            &self.gpu.bind_group,
            1,
            1,
            "validate-gradient-accumulation",
        );
        dispatch(
            &mut encoder,
            &self.gpu.pipelines.accumulate,
            &self.gpu.bind_group,
            (self.phenotype.synapses().len() as u32).div_ceil(WORKGROUP_SIZE),
            1,
            "accumulate-sequence-gradient",
        );
        encoder.copy_buffer_to_buffer(
            &self.gpu.gradients,
            u64::from(layout.accumulated_weights) * 4,
            &self.accumulated_gradients,
            0,
            bytes,
        );
        queue.submit(Some(encoder.finish()));
        let receipt =
            self.read_float_buffer(&self.gpu.outputs, u64::from(layout.metrics + 2) * 4, 2)?;
        if receipt[0] < 0.0 || receipt[1] != 1.0 {
            return Err(TrainingError::MalformedReadback);
        }
        self.accumulation = Some((expected, count + 1));
        Ok(count + 1)
    }

    /// Applies the whole effective batch once. The optional caller work in the
    /// encoder may stage the matching value-head update with its own rollback.
    pub fn apply_accumulated_gradients(
        &mut self,
        mut encoder: wgpu::CommandEncoder,
    ) -> Result<u32, TrainingError> {
        let (expected, count) = self
            .accumulation
            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
        if count != expected || !self.gpu.layout.replay || self.replay_shape.is_none() {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        let next = self
            .optimizer_step
            .checked_add(1)
            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
        let headers = build_step_headers(&self.phenotype, self.gpu.layout, self.config, next)?;
        let (_, queue) = self.session.backend().offline_training_device_queue()?;
        queue.write_buffer(&self.gpu.step_headers, 0, bytemuck::cast_slice(&headers));
        encoder.copy_buffer_to_buffer(
            &self.accumulated_gradients,
            0,
            &self.gpu.gradients,
            u64::from(self.gpu.layout.weight_gradients) * 4,
            self.phenotype.synapses().len() as u64 * 4,
        );
        dispatch(
            &mut encoder,
            &self.gpu.pipelines.gradient_norm,
            &self.gpu.bind_group,
            1,
            1,
            "batch-gradient-norm",
        );
        dispatch(
            &mut encoder,
            &self.gpu.pipelines.validate_adamw,
            &self.gpu.bind_group,
            1,
            1,
            "validate-batch-adamw",
        );
        dispatch(
            &mut encoder,
            &self.gpu.pipelines.adamw,
            &self.gpu.bind_group,
            (self.phenotype.synapses().len() as u32).div_ceil(WORKGROUP_SIZE),
            1,
            "batch-adamw",
        );
        queue.submit(Some(encoder.finish()));
        let receipt = self.read_float_buffer(
            &self.gpu.outputs,
            u64::from(self.gpu.layout.metrics + 2) * 4,
            2,
        )?;
        if receipt[0] < 0.0 || receipt[1] != 1.0 {
            return Err(TrainingError::MalformedReadback);
        }
        self.optimizer_step = next;
        self.accumulation = None;
        Ok(next)
    }

    pub fn set_stage_mask(&mut self, mask: StageTrainableMask) -> Result<(), TrainingError> {
        if self.accumulation.is_some() {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
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
        let stats = self.train_step_with_statistics(sequence, true)?;
        Ok(TrainingStepReceipt {
            optimizer_step: stats.optimizer_step,
            loss_before: stats.loss_before,
            loss_after: stats.loss_after.ok_or(TrainingError::MalformedReadback)?,
            unclipped_gradient_norm: stats.unclipped_gradient_norm,
            trained_weight_count: stats.trained_weight_count,
        })
    }

    /// Optionally skips the second forward pass for after-update loss.
    /// A skipped measurement is `None`.
    pub fn train_step_with_statistics(
        &mut self,
        sequence: &TrainingSequence32,
        measure_loss_after: bool,
    ) -> Result<TrainingStepStatistics, TrainingError> {
        self.validate_objective_mask(false)?;
        if self.accumulation.is_some() || self.gpu.layout.replay {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        sequence.validate_for(&self.phenotype)?;
        let next_step = self
            .optimizer_step
            .checked_add(1)
            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
        let training_words = pack_training_sequence(sequence, &self.phenotype, self.gpu.layout)?;
        if training_words.len() as u64 != self.gpu.layout.training_words {
            return Err(TrainingError::MalformedReadback);
        }
        let headers = build_step_headers(&self.phenotype, self.gpu.layout, self.config, next_step)?;
        let (device, queue) = self.session.backend().offline_training_device_queue()?;
        queue.write_buffer(&self.gpu.training, 0, bytemuck::cast_slice(&training_words));
        queue.write_buffer(&self.gpu.step_headers, 0, bytemuck::cast_slice(&headers));

        let has_candidate = sequence
            .ticks()
            .iter()
            .any(|tick| tick.candidate_target().is_some());
        let has_speech = sequence
            .ticks()
            .iter()
            .any(|tick| tick.speech_target().is_some());
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
        if has_candidate {
            dispatch(
                &mut encoder,
                &self.gpu.pipelines.candidate_forward,
                &self.gpu.bind_group,
                1,
                TRAINING_SEQUENCE_TICKS as u32,
                "foundation-trainer-candidate-forward",
            );
        }
        if has_speech {
            dispatch(
                &mut encoder,
                &self.gpu.pipelines.speech_forward,
                &self.gpu.bind_group,
                TRAINING_SEQUENCE_TICKS as u32,
                1,
                "foundation-trainer-speech-forward",
            );
        }
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
        if has_candidate {
            dispatch(
                &mut encoder,
                &self.gpu.pipelines.seed_candidate,
                &self.gpu.bind_group,
                neuron_groups,
                TRAINING_SEQUENCE_TICKS as u32,
                "foundation-trainer-seed-candidate-gradients",
            );
        }
        if has_speech {
            dispatch(
                &mut encoder,
                &self.gpu.pipelines.seed_speech,
                &self.gpu.bind_group,
                neuron_groups,
                TRAINING_SEQUENCE_TICKS as u32,
                "foundation-trainer-seed-speech-gradients",
            );
        }
        self.record_backward(&mut encoder, total_steps, neuron_groups);
        dispatch(
            &mut encoder,
            &self.gpu.pipelines.recurrent_gradients,
            &self.gpu.bind_group,
            synapse_groups,
            1,
            "foundation-trainer-recurrent-weight-gradients",
        );
        if has_candidate {
            dispatch(
                &mut encoder,
                &self.gpu.pipelines.candidate_gradients,
                &self.gpu.bind_group,
                synapse_groups,
                1,
                "foundation-trainer-candidate-weight-gradients",
            );
        }
        if has_speech {
            dispatch(
                &mut encoder,
                &self.gpu.pipelines.speech_gradients,
                &self.gpu.bind_group,
                synapse_groups,
                1,
                "foundation-trainer-speech-weight-gradients",
            );
        }
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
            &self.gpu.pipelines.validate_adamw,
            &self.gpu.bind_group,
            1,
            1,
            "validate-adamw-update",
        );
        dispatch(
            &mut encoder,
            &self.gpu.pipelines.adamw,
            &self.gpu.bind_group,
            synapse_groups,
            1,
            "foundation-trainer-adamw",
        );

        encoder.copy_buffer_to_buffer(
            &self.gpu.outputs,
            u64::from(self.gpu.layout.metrics + 3) * 4,
            &self.gpu.readback,
            12,
            4,
        );
        if measure_loss_after {
            encoder.clear_buffer(&self.gpu.state, 0, None);
            encoder.clear_buffer(&self.gpu.outputs, 0, None);
            self.record_forward(&mut encoder, total_steps, neuron_groups);
            if has_candidate {
                dispatch(
                    &mut encoder,
                    &self.gpu.pipelines.candidate_forward,
                    &self.gpu.bind_group,
                    1,
                    TRAINING_SEQUENCE_TICKS as u32,
                    "foundation-trainer-candidate-forward-after",
                );
            }
            if has_speech {
                dispatch(
                    &mut encoder,
                    &self.gpu.pipelines.speech_forward,
                    &self.gpu.bind_group,
                    TRAINING_SEQUENCE_TICKS as u32,
                    1,
                    "foundation-trainer-speech-forward-after",
                );
            }
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
        }
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
        let [loss_before, loss_after, gradient_norm, update_valid] = values.as_slice() else {
            return Err(TrainingError::MalformedReadback);
        };
        if !loss_before.is_finite()
            || (measure_loss_after && !loss_after.is_finite())
            || !gradient_norm.is_finite()
            || *gradient_norm < 0.0
            || *update_valid != 1.0
        {
            return Err(TrainingError::MalformedReadback);
        }
        self.optimizer_step = next_step;
        Ok(TrainingStepStatistics {
            optimizer_step: next_step,
            loss_before: *loss_before,
            loss_after: measure_loss_after.then_some(*loss_after),
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

    /// Neutral-context diagnostic forward. Grounded collection uses replay;
    /// synthetic targets do not demonstrate runtime parity.
    pub fn evaluate_sequence(
        &self,
        sequence: &TrainingSequence32,
    ) -> Result<SequenceEvaluation, TrainingError> {
        if self.gpu.layout.replay {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        sequence.validate_for(&self.phenotype)?;
        let training_words = pack_training_sequence(sequence, &self.phenotype, self.gpu.layout)?;
        let headers = build_step_headers(
            &self.phenotype,
            self.gpu.layout,
            self.config,
            self.optimizer_step.max(1),
        )?;
        let (device, queue) = self.session.backend().offline_training_device_queue()?;
        queue.write_buffer(&self.gpu.training, 0, bytemuck::cast_slice(&training_words));
        queue.write_buffer(&self.gpu.step_headers, 0, bytemuck::cast_slice(&headers));
        let total_steps =
            u32::from(self.phenotype.microstep_count()) * TRAINING_SEQUENCE_TICKS as u32;
        let neuron_groups = self.phenotype.neuron_count().div_ceil(WORKGROUP_SIZE);
        let readback_bytes = ((TRAINING_SEQUENCE_TICKS * 2 + 1) * 4) as u64;
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("foundation-trainer-evaluation-readback"),
            size: readback_bytes,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("foundation-trainer-evaluation"),
        });
        encoder.clear_buffer(&self.gpu.state, 0, None);
        encoder.clear_buffer(&self.gpu.outputs, 0, None);
        self.record_forward(&mut encoder, total_steps, neuron_groups);
        dispatch(
            &mut encoder,
            &self.gpu.pipelines.candidate_forward,
            &self.gpu.bind_group,
            1,
            TRAINING_SEQUENCE_TICKS as u32,
            "foundation-trainer-evaluation-candidate-forward",
        );
        dispatch(
            &mut encoder,
            &self.gpu.pipelines.speech_forward,
            &self.gpu.bind_group,
            TRAINING_SEQUENCE_TICKS as u32,
            1,
            "foundation-trainer-evaluation-speech-forward",
        );
        dispatch(
            &mut encoder,
            &self.gpu.pipelines.loss,
            &self.gpu.bind_group,
            1,
            1,
            "foundation-trainer-evaluation-loss",
        );
        encoder.copy_buffer_to_buffer(
            &self.gpu.outputs,
            u64::from(self.gpu.layout.candidate_logits) * 4,
            &staging,
            0,
            readback_bytes,
        );
        let command_buffer = encoder.finish();
        let (sender, receiver) = mpsc::channel();
        command_buffer.map_buffer_on_submit(
            &staging,
            wgpu::MapMode::Read,
            0..readback_bytes,
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
        let mapped = staging.slice(..readback_bytes).get_mapped_range();
        let values = bytemuck::cast_slice::<u8, f32>(&mapped).to_vec();
        drop(mapped);
        staging.unmap();
        let (candidate_logits, remainder) = values.split_at(TRAINING_SEQUENCE_TICKS);
        let (speech_logits, metric) = remainder.split_at(TRAINING_SEQUENCE_TICKS);
        let Some(mean_loss) = metric.first().copied() else {
            return Err(TrainingError::MalformedReadback);
        };
        let mut episode_count = 0_u32;
        let mut success_count = 0_u32;
        for ((tick, observed), speech_observed) in sequence
            .ticks()
            .iter()
            .zip(candidate_logits)
            .zip(speech_logits)
        {
            let Some(target) = tick.candidate_target() else {
                continue;
            };
            episode_count += 1;
            let candidate_correct = (*observed >= 0.0) == (target.target_logit >= 0.0);
            let speech_correct = tick
                .speech_target()
                .is_none_or(|speech| (*speech_observed >= 0.0) == (speech.target_logit >= 0.0));
            if candidate_correct && speech_correct {
                success_count += 1;
            }
        }
        Ok(SequenceEvaluation::new(
            mean_loss,
            candidate_logits.to_vec(),
            speech_logits.to_vec(),
            episode_count,
            success_count,
        )?)
    }

    pub fn read_weights(&self) -> Result<Vec<f32>, TrainingError> {
        self.read_float_buffer(&self.gpu.weights, 0, self.phenotype.synapses().len())
    }

    pub fn checkpoint(&self) -> Result<crate::FoundationTrainerCheckpoint, TrainingError> {
        if self.accumulation.is_some() {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        let n = self.phenotype.synapses().len();
        let state = self.read_checkpoint_state(n)?;
        Ok(crate::FoundationTrainerCheckpoint {
            schema_version: 2,
            phenotype_hash: self.phenotype.phenotype_hash(),
            source_foundation_digest: self.source_foundation.digest(),
            optimizer_step: self.optimizer_step,
            config: self.config,
            stage_mask: self.stage_mask.clone(),
            weights: state.weights,
            first_moment: state.first_moment,
            second_moment: state.second_moment,
            update_ages: state.update_ages,
        })
    }

    fn read_checkpoint_state(
        &self,
        count: usize,
    ) -> Result<checkpoint_readback::CheckpointReadback, TrainingError> {
        let (device, queue) = self.session.backend().offline_training_device_queue()?;
        let layout = checkpoint_readback::CheckpointReadbackLayout::new(
            count,
            device.limits().max_buffer_size,
        )?;
        layout.validate_source_sizes(self.gpu.weights.size(), self.gpu._optimizer.size())?;
        // Capture this boundary's actual device buffers in one submission. No
        // bytes are retained across training, replay shape changes, or rebind.
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("foundation-trainer-checkpoint-readback"),
            size: layout.total_bytes,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("foundation-trainer-checkpoint-export"),
        });
        encoder.copy_buffer_to_buffer(&self.gpu.weights, 0, &staging, 0, layout.lane_bytes);
        encoder.copy_buffer_to_buffer(
            &self.gpu._optimizer,
            0,
            &staging,
            layout.lane_bytes,
            layout.optimizer_bytes,
        );
        let command_buffer = encoder.finish();
        let (sender, receiver) = mpsc::channel();
        command_buffer.map_buffer_on_submit(
            &staging,
            wgpu::MapMode::Read,
            0..layout.total_bytes,
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
        let mapped = staging.slice(..layout.total_bytes).get_mapped_range();
        let state = layout.decode(&mapped);
        drop(mapped);
        staging.unmap();
        state
    }

    pub fn restore_checkpoint(
        &mut self,
        checkpoint: &crate::FoundationTrainerCheckpoint,
    ) -> Result<(), TrainingError> {
        if self.accumulation.is_some() {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        checkpoint.validate_for(&self.phenotype, self.source_foundation.digest())?;
        let n = self.phenotype.synapses().len();
        let (_, queue) = self.session.backend().offline_training_device_queue()?;
        queue.write_buffer(
            &self.gpu.weights,
            0,
            bytemuck::cast_slice(&checkpoint.weights),
        );
        queue.write_buffer(
            &self.gpu._optimizer,
            0,
            bytemuck::cast_slice(&checkpoint.first_moment),
        );
        queue.write_buffer(
            &self.gpu._optimizer,
            n as u64 * 4,
            bytemuck::cast_slice(&checkpoint.second_moment),
        );
        queue.write_buffer(
            &self.gpu._optimizer,
            n as u64 * 8,
            bytemuck::cast_slice(&checkpoint.update_ages),
        );
        queue.write_buffer(
            &self.gpu.mask,
            0,
            bytemuck::cast_slice(checkpoint.stage_mask.words()),
        );
        self.config = checkpoint.config;
        self.stage_mask = checkpoint.stage_mask.clone();
        self.optimizer_step = checkpoint.optimizer_step;
        Ok(())
    }

    fn read_float_buffer(
        &self,
        buffer: &wgpu::Buffer,
        offset: u64,
        count: usize,
    ) -> Result<Vec<f32>, TrainingError> {
        let values: Vec<f32> = self
            .read_word_buffer(buffer, offset, count)?
            .into_iter()
            .map(f32::from_bits)
            .collect();
        if values.iter().any(|v| !v.is_finite()) {
            return Err(TrainingError::MalformedReadback);
        }
        Ok(values)
    }

    fn read_word_buffer(
        &self,
        buffer: &wgpu::Buffer,
        offset: u64,
        count: usize,
    ) -> Result<Vec<u32>, TrainingError> {
        let (device, queue) = self.session.backend().offline_training_device_queue()?;
        let bytes = (count * 4) as u64;
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("foundation-trainer-weight-readback"),
            size: bytes,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("foundation-trainer-weight-export"),
        });
        encoder.copy_buffer_to_buffer(buffer, offset, &staging, 0, bytes);
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
        let weights = bytemuck::cast_slice::<u8, u32>(&mapped).to_vec();
        drop(mapped);
        staging.unmap();
        if weights.len() != count {
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
        self.gpu.record_forward(encoder, total_steps, neuron_groups);
    }

    fn record_backward(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        total_steps: u32,
        neuron_groups: u32,
    ) {
        self.gpu
            .record_backward(encoder, total_steps, neuron_groups);
    }
}

impl TrainerGpuState {
    fn record_forward(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        total_steps: u32,
        neuron_groups: u32,
    ) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("foundation-trainer-forward-microsteps"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipelines.forward);
        for step in 0..total_steps {
            pass.set_bind_group(0, &self.bind_group, &[step * HEADER_BYTES as u32]);
            pass.dispatch_workgroups(neuron_groups.max(1), 1, 1);
        }
    }

    fn record_backward(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        total_steps: u32,
        neuron_groups: u32,
    ) {
        // Each compute dispatch is a separate WebGPU usage scope. wgpu emits
        // the required storage barriers between dispatches within this pass,
        // preserving the local -> sources -> preceding-step dependency order.
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("foundation-trainer-backward-microsteps"),
            timestamp_writes: None,
        });
        for step in (self.layout.burn_in_steps..total_steps).rev() {
            pass.set_bind_group(0, &self.bind_group, &[step * HEADER_BYTES as u32]);
            pass.set_pipeline(&self.pipelines.backward_local);
            pass.dispatch_workgroups(neuron_groups.max(1), 1, 1);
            pass.set_pipeline(&self.pipelines.backward_sources);
            pass.dispatch_workgroups(neuron_groups.max(1), 1, 1);
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
        Self::new_for_replay(device, phenotype, initial_weights, mask, None, None)
    }

    fn new_for_replay(
        device: &wgpu::Device,
        phenotype: &BrainPhenotype,
        initial_weights: &[f32],
        mask: &StageTrainableMask,
        replay: Option<&TrainingSequence>,
        cached_pipelines: Option<&TrainerPipelines>,
    ) -> Result<Self, TrainingError> {
        let (meta, layout) = pack_metadata_and_layout(phenotype, replay)?;
        let limits = device.limits();
        for words in [
            layout.state_words,
            layout.gradient_words,
            layout.output_words,
            layout.training_words,
            meta.len() as u64,
        ] {
            if words * 4 > limits.max_storage_buffer_binding_size
                || words * 4 > limits.max_buffer_size
            {
                return Err(ScaffoldContractError::PhenotypeCompile.into());
            }
        }
        let total_steps = u32::from(phenotype.microstep_count()) * layout.ticks;
        // Uniform offsets are u32 even when buffer sizes are u64. Every header
        // is also one complete, independently aligned uniform binding.
        if !HEADER_BYTES.is_multiple_of(u64::from(limits.min_uniform_buffer_offset_alignment))
            || u64::from(total_steps.saturating_sub(1)) * HEADER_BYTES > u64::from(u32::MAX)
            || u64::from(total_steps) * HEADER_BYTES > limits.max_buffer_size
        {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        let header_capacity_words = usize::try_from(total_steps)
            .ok()
            .and_then(|steps| steps.checked_mul(HEADER_WORDS))
            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
        let zero_headers = vec![0_u32; header_capacity_words];
        let step_headers = buffer_init(
            device,
            "foundation-trainer-step-headers",
            &zero_headers,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
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
            initial_weights.len() as u64 * 12,
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
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
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        );
        let gradients = zero_buffer(
            device,
            "foundation-trainer-gradients",
            layout.gradient_words * 4,
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
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
        // Replay shape changes only buffers and bind groups. These immutable
        // device-local kernels use the same WGSL and binding ABI for every
        // window, including a new cohort after rebind_for_next_cohort.
        let bind_group_layout = cached_pipelines.map_or_else(
            || create_bind_group_layout(device),
            |pipelines| pipelines.bind_group_layout.clone(),
        );
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("foundation-trainer-bind-group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &step_headers,
                        offset: 0,
                        size: NonZeroU64::new(HEADER_BYTES),
                    }),
                },
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
        let pipelines = if let Some(pipelines) = cached_pipelines {
            pipelines.clone()
        } else {
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("foundation-trainer-pipeline-layout"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            });
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("foundation-trainer-wgsl"),
                source: wgpu::ShaderSource::Wgsl(TRAINER_WGSL.into()),
            });
            TrainerPipelines {
                bind_group_layout,
                initialize: pipeline(device, &pipeline_layout, &shader, "initialize_replay_state"),
                forward: pipeline(device, &pipeline_layout, &shader, "forward_microstep"),
                candidate_forward: pipeline(
                    device,
                    &pipeline_layout,
                    &shader,
                    "forward_candidate_logits",
                ),
                speech_forward: pipeline(
                    device,
                    &pipeline_layout,
                    &shader,
                    "forward_speech_logits",
                ),
                loss: pipeline(device, &pipeline_layout, &shader, "reduce_loss"),
                seed_candidate: pipeline(
                    device,
                    &pipeline_layout,
                    &shader,
                    "seed_candidate_activation_gradients",
                ),
                seed_speech: pipeline(
                    device,
                    &pipeline_layout,
                    &shader,
                    "seed_speech_activation_gradients",
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
                speech_gradients: pipeline(
                    device,
                    &pipeline_layout,
                    &shader,
                    "speech_weight_gradients",
                ),
                gradient_norm: pipeline(device, &pipeline_layout, &shader, "reduce_gradient_norm"),
                validate_adamw: pipeline(device, &pipeline_layout, &shader, "validate_adamw"),
                validate_accumulation: pipeline(
                    device,
                    &pipeline_layout,
                    &shader,
                    "validate_accumulation",
                ),
                accumulate: pipeline(
                    device,
                    &pipeline_layout,
                    &shader,
                    "accumulate_weight_gradients",
                ),
                adamw: pipeline(device, &pipeline_layout, &shader, "apply_adamw"),
            }
        };
        Ok(Self {
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

#[derive(Clone)]
struct CachedPhenotypeMetadata {
    prefix_meta: Vec<u32>,
    target_offsets: u32,
    incoming_ids: u32,
    source_offsets: u32,
    outgoing_ids: u32,
    synapse_records: u32,
    dynamics: u32,
    family_biases: u32,
    speech_offsets: u32,
    decoder_offset_counts: [u32; 9],
    decoder_ids_vec: Vec<u32>,
}

fn metadata_float_bits_digest(phenotype: &BrainPhenotype) -> [u64; 4] {
    // Canonical phenotype hashes normalize signed zero, but the GPU metadata
    // deliberately preserves every raw float bit. Keep those identities apart.
    let mut digest =
        alife_core::CanonicalDigestBuilder::new(b"alife.training.metadata-float-bits.v1");
    digest.write_sequence_len(phenotype.neuron_dynamics().len());
    for row in phenotype.neuron_dynamics() {
        for value in [
            row.bias(),
            row.leak(),
            row.homeostatic_gain(),
            row.metabolic_decay(),
            row.activity_ema_decay(),
        ] {
            digest.write_u32(value.to_bits());
        }
    }
    digest.write_sequence_len(phenotype.candidate_decoder().families().len());
    for family in phenotype.candidate_decoder().families() {
        digest.write_u32(family.bias().to_bits());
    }
    digest.finish256()
}

fn cached_phenotype_metadata(
    phenotype: &BrainPhenotype,
) -> Result<CachedPhenotypeMetadata, TrainingError> {
    use std::collections::VecDeque;
    use std::sync::{Mutex, OnceLock};

    type Key = (alife_core::PhenotypeHash, [u64; 4], [u64; 4]);
    static CACHE: OnceLock<Mutex<VecDeque<(Key, CachedPhenotypeMetadata)>>> = OnceLock::new();
    let key = (
        phenotype.phenotype_hash(),
        phenotype.compiler_inputs_digest(),
        metadata_float_bits_digest(phenotype),
    );
    let cache = CACHE.get_or_init(|| Mutex::new(VecDeque::with_capacity(8)));
    if let Ok(mut guard) = cache.lock() {
        if let Some(pos) = guard.iter().position(|(k, _)| *k == key) {
            let entry = guard.remove(pos).expect("checked cache index");
            let value = entry.1.clone();
            guard.push_back(entry);
            return Ok(value);
        }
    }

    let neuron_count = phenotype.neuron_count() as usize;
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
    let mut prefix_meta = Vec::new();
    let target_offsets = push_offsets(
        &mut prefix_meta,
        neuron_count,
        incoming.iter().map(|entry| entry.0),
    )?;
    let incoming_ids = as_u32(prefix_meta.len())?;
    prefix_meta.extend(incoming.iter().map(|entry| entry.2));
    let source_offsets = push_offsets(
        &mut prefix_meta,
        neuron_count,
        outgoing.iter().map(|entry| entry.0),
    )?;
    let outgoing_ids = as_u32(prefix_meta.len())?;
    prefix_meta.extend(outgoing.iter().map(|entry| entry.2));
    let synapse_records = as_u32(prefix_meta.len())?;
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
        // Decoder coefficients are signed mathematical readouts. Only recurrent
        // pathways obey the inherited excitatory/inhibitory route contract.
        let sign_policy = match (synapse.kind(), projection.projection_type()) {
            (CompiledSynapseKind::Recurrent, ProjectionType::LateralInhibition) => 1,
            (
                CompiledSynapseKind::Recurrent,
                ProjectionType::Homeostatic | ProjectionType::MotorProposal,
            ) => 2,
            _ => 0,
        };
        prefix_meta.extend_from_slice(&[
            synapse.source(),
            synapse.target(),
            kind | (u32::from(synapse.route_index()) << 8),
            u32::from(projection.update_cadence().raw()),
            head,
            family,
            lane,
            sign_policy,
        ]);
    }
    let dynamics = as_u32(prefix_meta.len())?;
    for row in phenotype.neuron_dynamics() {
        prefix_meta.extend_from_slice(&[
            row.bias().to_bits(),
            row.leak().to_bits(),
            u32::from(row.activation().raw()),
            row.homeostatic_gain().to_bits(),
            row.metabolic_decay().to_bits(),
            row.activity_ema_decay().to_bits(),
            0,
            0,
        ]);
    }
    let family_biases = as_u32(prefix_meta.len())?;
    let mut biases = [0.0_f32; 8];
    for family in phenotype.candidate_decoder().families() {
        biases[usize::from(family.family().raw())] = family.bias();
    }
    prefix_meta.extend(biases.into_iter().map(f32::to_bits));
    let speech_offsets = as_u32(prefix_meta.len())?;
    let speech_start =
        phenotype.candidate_decoder().motor_start() + SpeechDecoderLayoutV1::MOTOR_TARGET_OFFSET;
    let mut speech_ids = Vec::new();
    for output in 0..32_u32 {
        prefix_meta.push(as_u32(speech_ids.len())?);
        speech_ids.extend(phenotype.synapses().iter().enumerate().filter(|(_,s)| matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==alife_core::DecoderHeadKind::SpeechPayload) && s.target()==speech_start+output).map(|(i,_)|i as u32));
    }
    prefix_meta.push(as_u32(speech_ids.len())?);
    prefix_meta.extend(speech_ids);

    let mut decoder_offset_counts = [0_u32; 9];
    let mut decoder_ids_vec = Vec::new();
    for family in 0..8u8 {
        decoder_offset_counts[usize::from(family)] = as_u32(decoder_ids_vec.len())?;
        decoder_ids_vec.extend(
            phenotype
                .synapses()
                .iter()
                .enumerate()
                .filter_map(|(i, s)| match s.kind() {
                    CompiledSynapseKind::Decoder(c)
                        if c.family().raw() == family && c.head().raw() != 3 =>
                    {
                        Some(i as u32)
                    }
                    _ => None,
                }),
        );
    }
    decoder_offset_counts[8] = as_u32(decoder_ids_vec.len())?;

    let value = CachedPhenotypeMetadata {
        prefix_meta,
        target_offsets,
        incoming_ids,
        source_offsets,
        outgoing_ids,
        synapse_records,
        dynamics,
        family_biases,
        speech_offsets,
        decoder_offset_counts,
        decoder_ids_vec,
    };
    if let Ok(mut guard) = cache.lock() {
        if !guard.iter().any(|(k, _)| *k == key) {
            if guard.len() >= 8 {
                guard.pop_front();
            }
            guard.push_back((key, value.clone()));
        }
    }
    Ok(value)
}

fn pack_metadata_and_layout(
    phenotype: &BrainPhenotype,
    replay: Option<&TrainingSequence>,
) -> Result<(Vec<u32>, PackedLayout), TrainingError> {
    let neuron_count = phenotype.neuron_count() as usize;
    let synapse_count = phenotype.synapses().len();
    let cached = cached_phenotype_metadata(phenotype)?;
    let mut meta = cached.prefix_meta;
    let target_offsets = cached.target_offsets;
    let incoming_ids = cached.incoming_ids;
    let source_offsets = cached.source_offsets;
    let outgoing_ids = cached.outgoing_ids;
    let synapse_records = cached.synapse_records;
    let dynamics = cached.dynamics;
    let family_biases = cached.family_biases;
    let speech_offsets = cached.speech_offsets;

    let ticks = replay.map_or(TRAINING_SEQUENCE_TICKS, |r| r.ticks.len()) as u64;
    let candidate_capacity = if replay.is_some() {
        alife_core::MAX_ACTION_CANDIDATES as u64
    } else {
        1
    };
    let neurons = neuron_count as u64;
    let total_steps = ticks * u64::from(phenotype.microstep_count());
    let state_count = (total_steps + 1)
        .checked_mul(neurons)
        .ok_or(ScaffoldContractError::PhenotypeCompile)?;
    let inputs = 0;
    let targets = as_u32(ticks * neurons)?;
    let target_weights = as_u32(ticks * neurons * 2)?;
    let candidate_records = as_u32(ticks * neurons * 3)?;
    let base_training_words = ticks
        .checked_mul(neurons * 3 + CANDIDATE_RECORD_WORDS as u64 * candidate_capacity)
        .ok_or(ScaffoldContractError::PhenotypeCompile)?;
    let activations = 0;
    let metabolic = as_u32(state_count)?;
    let activity_ema = as_u32(state_count * 2)?;
    let state_words = state_count * 3;
    let activation_gradients = 0;
    let metabolic_gradients = as_u32(state_count)?;
    let deltas = as_u32(state_count * 2)?;
    let weight_gradients = as_u32(state_count * 2 + total_steps * neurons)?;
    let base_gradient_words = state_count
        .checked_mul(2)
        .and_then(|value| value.checked_add(total_steps * neurons))
        .and_then(|value| value.checked_add(synapse_count as u64))
        .ok_or(ScaffoldContractError::PhenotypeCompile)?;
    let adjoints = as_u32(base_gradient_words)?;
    let accumulated_weights = as_u32(base_gradient_words + ticks * candidate_capacity)?;
    let gradient_words = u64::from(accumulated_weights) + synapse_count as u64 + ticks * 192;
    let candidate_logits = 0;
    let speech_logits = as_u32(ticks * candidate_capacity)?;
    let metrics =
        as_u32(ticks * candidate_capacity + ticks * if replay.is_some() { 192 } else { 1 })?;
    let memory_raw = metrics + 4;
    let cognitive_raw = memory_raw + as_u32(ticks * candidate_capacity)?;
    let output_words = u64::from(cognitive_raw) + ticks * candidate_capacity;
    let context = as_u32(base_training_words)?;
    let route_count = as_u32(phenotype.projections().len())?;
    let initial = as_u32(
        base_training_words
            + ticks
                * (4 + u64::from(route_count)
                    + synapse_count as u64
                    + (alife_core::MAX_STRUCTURAL_EDGES as u64) * 5),
    )?;
    let replay_speech_targets = as_u32(u64::from(initial) + neurons * 3)?;
    let training_words = u64::from(replay_speech_targets) + ticks * 384;
    let (
        dendritic_offsets,
        dendritic_branches,
        dendritic_inputs,
        dendritic_source_offsets,
        dendritic_source_branches,
    ) = pack_dendrites(&mut meta, phenotype, replay)?;
    let decoder_offsets = as_u32(meta.len())?;
    meta.extend_from_slice(&cached.decoder_offset_counts);
    let decoder_ids = as_u32(meta.len())?;
    meta.extend(cached.decoder_ids_vec);
    let speech_target_start =
        phenotype.candidate_decoder().motor_start() + SpeechDecoderLayoutV1::MOTOR_TARGET_OFFSET;
    Ok((
        meta,
        PackedLayout {
            ticks: ticks as u32,
            replay: replay.is_some(),
            candidate_capacity: candidate_capacity as u32,
            context,
            route_count,
            dendritic_offsets,
            dendritic_branches,
            dendritic_inputs,
            dendritic_source_offsets,
            dendritic_source_branches,
            initial,
            activity_ema,
            adjoints,
            decoder_offsets,
            decoder_ids,
            memory_gain: replay.map_or(0.0, |r| r.memory_candidate_gain),
            burn_in_steps: replay.map_or(0, |r| {
                r.burn_in_ticks as u32 * u32::from(phenotype.microstep_count())
            }),
            memory_raw,
            cognitive_raw,
            accumulated_weights,
            gradient_scale: 1.0,
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
            speech_logits,
            metrics,
            speech_target_start,
            replay_speech_targets,
            speech_offsets,
            optimizer_v: synapse_count as u32,
            optimizer_age: as_u32(
                synapse_count
                    .checked_mul(2)
                    .ok_or(ScaffoldContractError::PhenotypeCompile)?,
            )?,
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

fn pack_dendrites(
    meta: &mut Vec<u32>,
    phenotype: &BrainPhenotype,
    replay: Option<&TrainingSequence>,
) -> Result<(u32, u32, u32, u32, u32), TrainingError> {
    let branches = replay.map_or(&[][..], |r| r.initial.dendrites.branches());
    let n = phenotype.neuron_count() as usize;
    let offsets = push_offsets(meta, n, branches.iter().map(|b| b.target))?;
    let descriptors = as_u32(meta.len())?;
    let mut inputs = Vec::new();
    let mut outgoing = Vec::new();
    for (index, b) in branches.iter().enumerate() {
        meta.extend_from_slice(&[
            b.target,
            b.threshold.to_bits(),
            b.output_gain.to_bits(),
            as_u32(inputs.len() / 2)?,
            as_u32(b.inputs.len())?,
        ]);
        for input in &b.inputs {
            inputs.extend_from_slice(&[input.source, input.weight.to_bits()]);
            outgoing.push((input.source, index as u32, input.weight.to_bits()));
        }
    }
    let input_base = as_u32(meta.len())?;
    meta.extend(inputs);
    outgoing.sort_unstable_by_key(|row| (row.0, row.1));
    let source_offsets = push_offsets(meta, n, outgoing.iter().map(|row| row.0))?;
    let source_branches = as_u32(meta.len())?;
    for (_, branch, weight) in outgoing {
        meta.extend_from_slice(&[branch, weight]);
    }
    Ok((
        offsets,
        descriptors,
        input_base,
        source_offsets,
        source_branches,
    ))
}

fn pack_replay_sequence(
    sequence: &TrainingSequence,
    layout: PackedLayout,
    neurons: u32,
) -> Vec<u32> {
    let mut words = vec![0; layout.training_words as usize];
    for (tick_index, tick) in sequence.ticks.iter().enumerate() {
        let input = layout.inputs as usize + tick_index * neurons as usize;
        for (slot, value) in words[input..input + neurons as usize]
            .iter_mut()
            .zip(&tick.encoded_inputs)
        {
            *slot = value.to_bits();
        }
        for (candidate_index, candidate) in tick.candidates.iter().enumerate() {
            let base = layout.candidate_records as usize
                + (tick_index * layout.candidate_capacity as usize + candidate_index)
                    * CANDIDATE_RECORD_WORDS;
            words[base] = 1;
            words[base + 1] = u32::from(candidate.family.raw());
            words[base + 2] = candidate.innate_bias.to_bits();
            for (lane, value) in candidate.decoder_inputs.iter().enumerate() {
                let field = if lane < 24 { 4 + lane } else { 32 + lane - 24 };
                words[base + field] = value.to_bits();
            }
        }
        let base = layout.context as usize
            + tick_index
                * (4 + tick.enabled_routes.len()
                    + tick.effective_weight_offsets.len()
                    + alife_core::MAX_STRUCTURAL_EDGES * 5);
        words[base] = tick.projection_gain.to_bits();
        words[base + 1] = tick.local_threshold_shift.to_bits();
        words[base + 2] = tick.microstep_count;
        words[base + 3] = tick.structural_synapses.len() as u32;
        for (index, enabled) in tick.enabled_routes.iter().enumerate() {
            words[base + 4 + index] = u32::from(*enabled);
        }
        let weight_start = base + 4 + tick.enabled_routes.len();
        let edge_base = weight_start + tick.effective_weight_offsets.len();
        words[weight_start..edge_base]
            .copy_from_slice(bytemuck::cast_slice(&tick.effective_weight_offsets));
        for (index, edge) in tick.structural_synapses.iter().enumerate() {
            let at = edge_base + index * 5;
            words[at..at + 5].copy_from_slice(&[
                edge.source,
                edge.target,
                edge.route,
                edge.cadence,
                edge.effective_weight.to_bits(),
            ]);
        }
    }
    for (index, value) in sequence
        .initial
        .activations
        .iter()
        .chain(&sequence.initial.metabolic_load)
        .chain(&sequence.initial.activity_ema)
        .enumerate()
    {
        words[layout.initial as usize + index] = value.to_bits();
    }
    words
}

fn pack_training_sequence(
    sequence: &TrainingSequence32,
    phenotype: &BrainPhenotype,
    layout: PackedLayout,
) -> Result<Vec<u32>, TrainingError> {
    let neuron_count = phenotype.neuron_count();
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
        if let Some(speech) = tick.speech_target() {
            words.extend_from_slice(&[
                1,
                u32::from(speech.output_index),
                speech.target_logit.to_bits(),
                speech.loss_weight.to_bits(),
            ]);
        } else {
            words.extend(std::iter::repeat_n(0, 4));
        }
        words.extend(std::iter::repeat_n(
            0,
            CANDIDATE_RECORD_WORDS - (8 + CANDIDATE_FEATURE_COUNT),
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
    words.resize(layout.training_words as usize, 0);
    Ok(words)
}

fn build_step_headers(
    phenotype: &BrainPhenotype,
    layout: PackedLayout,
    config: AdamWConfig,
    optimizer_step: u32,
) -> Result<Vec<u32>, TrainingError> {
    let total_steps = u32::from(phenotype.microstep_count())
        .checked_mul(layout.ticks)
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
        header[5] = layout.ticks;
        header[6] = total_steps;
        header[7] = step;
        header[8] = step % u32::from(phenotype.microstep_count());
        header[9] = step / u32::from(phenotype.microstep_count());
        header[10] = optimizer_step;
        header[11] = layout.speech_offsets;
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
        header[37] = layout.speech_target_start;
        header[38] = layout.family_biases;
        header[39] = CANDIDATE_FEATURE_COUNT as u32;
        header[40] = layout.speech_logits;
        header[41] = u32::from(layout.replay);
        header[42] = layout.context;
        header[43] = layout.route_count;
        header[44] = layout.dendritic_offsets;
        header[45] = layout.dendritic_branches;
        header[46] = layout.dendritic_inputs;
        header[47] = layout.dendritic_source_offsets;
        header[48] = layout.dendritic_source_branches;
        header[49] = layout.initial;
        header[50] = layout.activity_ema;
        header[51] = layout.candidate_capacity;
        header[52] = u32::from(layout.replay);
        header[53] = layout.adjoints;
        header[54] = layout.decoder_offsets;
        header[55] = layout.decoder_ids;
        header[56] = layout.memory_gain.to_bits();
        header[57] = layout.burn_in_steps;
        header[58] = layout.memory_raw;
        header[59] = layout.cognitive_raw;
        header[60] = layout.accumulated_weights;
        header[61] = layout.gradient_scale.to_bits();
        header[62] = layout.optimizer_age;
        header[63] = layout.replay_speech_targets;
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
            has_dynamic_offset: true,
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
    pass.set_bind_group(0, bind_group, &[0]);
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

fn as_u32(value: impl TryInto<u32>) -> Result<u32, TrainingError> {
    value
        .try_into()
        .map_err(|_| ScaffoldContractError::PhenotypeCompile)
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adam_metadata_constrains_recurrent_routes_but_preserves_signed_decoder_readouts() {
        use alife_core::{
            BrainCapacityClass, BrainGenome, DevelopmentState, NormalizedScalar, PhenotypeCompiler,
            SensorProfile, Tick,
        };
        let capacity = BrainCapacityClass::n2048();
        let genome = BrainGenome::scaffold(96001, capacity.id());
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
        let phenotype = PhenotypeCompiler::compile(
            &genome,
            &capacity,
            &development,
            SensorProfile::GroundedTerrainVisionV1,
        )
        .unwrap();
        let mut weights: Vec<_> = phenotype
            .synapses()
            .iter()
            .map(|s| s.genetic_weight())
            .collect();
        let speech = phenotype.synapses().iter().position(|s| matches!(s.kind(),
            CompiledSynapseKind::Decoder(c) if c.head() == alife_core::DecoderHeadKind::SpeechPayload)).unwrap();
        weights[speech] = -0.25;
        let asset = FoundationWeightAsset::from_trained_weights(
            &phenotype,
            weights,
            TrainingStageManifest::new(1, 1, 1),
        )
        .unwrap();
        let (phenotype, _) =
            PhenotypeCompiler::compile_n2048_foundation_candidate(genome, development, asset)
                .unwrap();
        let (metadata, layout) = pack_metadata_and_layout(&phenotype, None).unwrap();
        assert_eq!(
            metadata[layout.synapse_records as usize + speech * 8 + 7],
            0
        );
        assert_eq!(phenotype.synapses()[speech].genetic_weight(), -0.25);
        let mut signed_distance_readouts = 0;
        let mut excitatory_routes = 0;
        let mut inhibitory_routes = 0;
        for (index, synapse) in phenotype.synapses().iter().enumerate() {
            let policy = metadata[layout.synapse_records as usize + index * 8 + 7];
            let projection =
                phenotype.projections()[usize::from(synapse.route_index())].projection_type();
            match synapse.kind() {
                CompiledSynapseKind::Decoder(c) => {
                    assert_eq!(
                        policy, 0,
                        "Adam must permit either sign for decoder coordinates"
                    );
                    if c.head() == alife_core::DecoderHeadKind::ActionCandidate
                        && c.input_lane() == 2
                    {
                        assert!(synapse.genetic_weight() < 0.0);
                        signed_distance_readouts += 1;
                    }
                }
                CompiledSynapseKind::Recurrent => match projection {
                    ProjectionType::Homeostatic | ProjectionType::MotorProposal => {
                        assert_eq!(policy, 2);
                        excitatory_routes += 1;
                    }
                    ProjectionType::LateralInhibition => {
                        assert_eq!(policy, 1);
                        inhibitory_routes += 1;
                    }
                    _ => assert_eq!(policy, 0),
                },
            }
        }
        assert!(signed_distance_readouts > 0 && excitatory_routes > 0 && inhibitory_routes > 0);
    }

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
            "initialize_replay_state",
            "forward_microstep",
            "forward_candidate_logits",
            "forward_speech_logits",
            "reduce_loss",
            "seed_candidate_activation_gradients",
            "seed_speech_activation_gradients",
            "backward_local",
            "backward_recurrent_sources",
            "recurrent_weight_gradients",
            "candidate_weight_gradients",
            "speech_weight_gradients",
            "reduce_gradient_norm",
            "validate_adamw",
            "validate_accumulation",
            "accumulate_weight_gradients",
            "apply_adamw",
        ] {
            assert!(names.contains(&required), "missing {required}");
        }
    }

    fn metadata_test_phenotype() -> BrainPhenotype {
        use alife_core::{
            BrainCapacityClass, BrainGenome, DevelopmentState, NormalizedScalar, PhenotypeCompiler,
            SensorProfile, Tick,
        };
        let capacity = BrainCapacityClass::n2048();
        let genome = BrainGenome::scaffold(99, capacity.id());
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
        PhenotypeCompiler::compile(
            &genome,
            &capacity,
            &development,
            SensorProfile::GroundedTerrainVisionV1,
        )
        .unwrap()
    }

    #[test]
    fn cached_metadata_packing_preserves_layout() {
        let phenotype = metadata_test_phenotype();
        let (meta_first, layout_first) = pack_metadata_and_layout(&phenotype, None).unwrap();
        let (meta_second, layout_second) = pack_metadata_and_layout(&phenotype, None).unwrap();
        assert_eq!(meta_first, meta_second);
        assert_eq!(layout_first.decoder_offsets, layout_second.decoder_offsets);
        assert_eq!(layout_first.decoder_ids, layout_second.decoder_ids);
        assert_eq!(layout_first.speech_offsets, layout_second.speech_offsets);
    }

    #[test]
    fn metadata_cache_distinguishes_raw_signed_zero_dynamics() {
        let phenotype = metadata_test_phenotype();
        let index = phenotype
            .neuron_dynamics()
            .iter()
            .position(|row| row.bias().to_bits() == 0)
            .unwrap();
        let mut wire = serde_json::to_value(&phenotype).unwrap();
        wire["neuron_dynamics"][index]["bias"] = serde_json::json!(-0.0_f32);
        let negative: BrainPhenotype = serde_json::from_value(wire).unwrap();
        assert_eq!(phenotype.phenotype_hash(), negative.phenotype_hash());
        assert_eq!(
            phenotype.compiler_inputs_digest(),
            negative.compiler_inputs_digest()
        );
        let (positive_meta, positive_layout) = pack_metadata_and_layout(&phenotype, None).unwrap();
        let (negative_meta, negative_layout) = pack_metadata_and_layout(&negative, None).unwrap();
        let positive_bias = positive_layout.dynamics as usize + index * 8;
        let negative_bias = negative_layout.dynamics as usize + index * 8;
        assert_eq!(positive_meta[positive_bias], 0.0_f32.to_bits());
        assert_eq!(negative_meta[negative_bias], (-0.0_f32).to_bits());
        let (positive_again, _) = pack_metadata_and_layout(&phenotype, None).unwrap();
        assert_eq!(positive_again, positive_meta);
    }

    #[test]
    fn replay_shape_comparison_distinguishes_raw_dendritic_zero_bits() {
        use alife_core::{DendriticBranch, DendriticBranchSet, DendriticInputRef};
        let positive =
            DendriticBranch::new(0, 0.0, 0.0, vec![DendriticInputRef::new(1, 0.0).unwrap()])
                .unwrap();
        let shape = (4, DendriticBranchSet::new(vec![positive.clone()]).unwrap());
        assert!(replay_shapes_equal(&shape, &shape));
        assert!(!replay_shapes_equal(&shape, &(3, shape.1.clone())));
        for lane in 0..3 {
            let mut negative = positive.clone();
            match lane {
                0 => negative.threshold = -0.0,
                1 => negative.output_gain = -0.0,
                _ => negative.inputs[0].weight = -0.0,
            }
            let next = (4, DendriticBranchSet::new(vec![negative]).unwrap());
            assert_eq!(shape, next, "ordinary PartialEq normalizes signed zero");
            assert!(!replay_shapes_equal(&shape, &next));
        }
    }

    #[test]
    fn reused_replay_parameters_match_fresh_layout_and_headers_after_burn_in_changes() {
        use crate::{TrainingInitialState, TrainingReplayCandidate, TrainingReplayTick};
        let phenotype = metadata_test_phenotype();
        let neurons = phenotype.neuron_count() as usize;
        let tick = TrainingReplayTick {
            encoded_inputs: vec![0.0; neurons],
            projection_gain: 1.0,
            local_threshold_shift: 0.0,
            microstep_count: 0,
            enabled_routes: vec![false; phenotype.projections().len()],
            effective_weight_offsets: vec![0.0; phenotype.synapses().len()],
            structural_synapses: Vec::new(),
            candidates: vec![TrainingReplayCandidate {
                family: alife_core::CandidateActionFamily::Approach,
                decoder_inputs: [0.0; 54],
                innate_bias: 0.0,
            }],
        };
        let mut replay = TrainingSequence {
            phenotype_hash: phenotype.phenotype_hash(),
            initial: TrainingInitialState {
                activations: vec![0.0; neurons],
                activity_ema: vec![0.0; neurons],
                metabolic_load: vec![0.0; neurons],
                dendrites: Default::default(),
            },
            ticks: vec![tick; 4],
            burn_in_ticks: 0,
            memory_candidate_gain: phenotype
                .candidate_decoder()
                .memory_channel()
                .map_or(0.0, |p| p.max_candidate_gain()),
        };
        replay.validate_for(&phenotype).unwrap();
        let shape = (replay.ticks.len(), replay.initial.dendrites.clone());
        let (metadata, mut reused) = pack_metadata_and_layout(&phenotype, Some(&replay)).unwrap();
        let allocation_shape = (
            reused.state_words,
            reused.gradient_words,
            reused.output_words,
            reused.training_words,
        );
        for burn_in_ticks in [2, 1, 0, 3] {
            replay.burn_in_ticks = burn_in_ticks;
            replay.validate_for(&phenotype).unwrap();
            assert!(replay_shapes_equal(
                &shape,
                &(replay.ticks.len(), replay.initial.dendrites.clone())
            ));
            reused.refresh_replay_parameters(&replay, phenotype.microstep_count());
            let (fresh_metadata, fresh) =
                pack_metadata_and_layout(&phenotype, Some(&replay)).unwrap();
            assert_eq!(metadata, fresh_metadata);
            assert_eq!(
                allocation_shape,
                (
                    fresh.state_words,
                    fresh.gradient_words,
                    fresh.output_words,
                    fresh.training_words,
                )
            );
            assert_eq!(
                reused.burn_in_steps,
                burn_in_ticks as u32 * u32::from(phenotype.microstep_count())
            );
            assert_eq!(
                reused.memory_gain.to_bits(),
                replay.memory_candidate_gain.to_bits()
            );
            assert_eq!(
                build_step_headers(&phenotype, reused, AdamWConfig::default(), 17).unwrap(),
                build_step_headers(&phenotype, fresh, AdamWConfig::default(), 17).unwrap()
            );
            assert_eq!(
                pack_replay_sequence(&replay, reused, phenotype.neuron_count()),
                pack_replay_sequence(&replay, fresh, phenotype.neuron_count())
            );
        }
    }
}
