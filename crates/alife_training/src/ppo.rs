//! Offline PPO bookkeeping and GPU objective over the existing sparse-graph replay.
//! This module never runs a CPU neural forward pass or owns a second GPU session.

use alife_core::ScaffoldContractError;
use alife_runtime::{GpuAuthoritativeSession, GpuSessionConsumerKind};

use crate::{AdamWConfig, TrainingError};

pub const PPO_MAX_CANDIDATES: usize = 32;
pub const PPO_MOTOR_SLOTS: usize = 6;
pub const PPO_ROW_WORDS: usize = 64;
pub const PPO_METRIC_WORDS: usize = 8;
pub const PPO_WGSL: &str = include_str!("../shaders/foundation_ppo.wgsl");

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PpoConfig {
    pub clip_ratio: f32,
    pub epochs: u32,
    pub target_kl: f32,
    pub entropy_coefficient: f32,
    pub value_coefficient: f32,
    pub discount_half_life_seconds: f32,
    pub gae_half_life_seconds: f32,
    pub temperature: f32,
    pub value_optimizer: AdamWConfig,
}

impl Default for PpoConfig {
    fn default() -> Self {
        Self {
            clip_ratio: 0.2,
            epochs: 2,
            target_kl: 0.02,
            entropy_coefficient: 0.01,
            value_coefficient: 0.5,
            discount_half_life_seconds: 60.0,
            gae_half_life_seconds: 10.0,
            temperature: 1.0,
            value_optimizer: AdamWConfig::default(),
        }
    }
}

fn invalid() -> TrainingError {
    ScaffoldContractError::InvalidDecisionEvidence.into()
}

impl PpoConfig {
    pub fn validate(self) -> Result<(), TrainingError> {
        self.value_optimizer.validate()?;
        if self.epochs == 0
            || ![
                self.clip_ratio,
                self.target_kl,
                self.entropy_coefficient,
                self.value_coefficient,
                self.discount_half_life_seconds,
                self.gae_half_life_seconds,
                self.temperature,
            ]
            .into_iter()
            .all(f32::is_finite)
            || !(0.0..1.0).contains(&self.clip_ratio)
            || self.clip_ratio == 0.0
            || self.target_kl <= 0.0
            || self.entropy_coefficient < 0.0
            || self.value_coefficient < 0.0
            || self.discount_half_life_seconds <= 0.0
            || self.gae_half_life_seconds <= 0.0
            || self.temperature <= 0.0
        {
            return Err(invalid());
        }
        Ok(())
    }
}

/// Masks and indices must come from the same GPU sampling receipt, not inferred
/// from action success. A candidate can be legal to attempt and fail in the world.
#[derive(Debug, Clone, PartialEq)]
pub struct PpoJointAction {
    pub candidate_count: u16,
    pub representative_mask: u32,
    pub motor_masks: [u32; PPO_MOTOR_SLOTS],
    pub representative: u16,
    /// Candidate-indexed slot forced by representative arbitration. Idle and
    /// Gesture force posture even when absent from the posture sampling mask.
    pub forced_slots: Vec<Option<u8>>,
    /// Includes the representative's forced slot. That slot contributes no
    /// additional categorical log probability.
    pub motor_candidates: [Option<u16>; PPO_MOTOR_SLOTS],
    pub old_joint_log_probability: f32,
    pub temperature: f32,
}

impl PpoJointAction {
    pub fn validate(&self, temperature: f32) -> Result<(), TrainingError> {
        let count = usize::from(self.candidate_count);
        if count == 0
            || count > PPO_MAX_CANDIDATES
            || self.forced_slots.len() != count
            || usize::from(self.representative) >= count
            || !self.old_joint_log_probability.is_finite()
            || self.old_joint_log_probability > 1.0e-5
            || self.temperature.to_bits() != temperature.to_bits()
            || !self.temperature.is_finite()
            || self.temperature <= 0.0
        {
            return Err(invalid());
        }
        let valid_bits = u32::MAX >> (32 - count);
        if self.representative_mask == 0
            || self.representative_mask & !valid_bits != 0
            || self.representative_mask & (1 << self.representative) == 0
            || self
                .forced_slots
                .iter()
                .flatten()
                .any(|slot| usize::from(*slot) >= PPO_MOTOR_SLOTS)
        {
            return Err(invalid());
        }
        let forced = self.forced_slots[usize::from(self.representative)];
        for (slot, mask) in self.motor_masks.iter().copied().enumerate() {
            if mask & !self.representative_mask != 0 {
                return Err(invalid());
            }
            for candidate in 0..count {
                if mask & (1 << candidate) != 0 && self.forced_slots[candidate] != Some(slot as u8)
                {
                    return Err(invalid());
                }
            }
            match self.motor_candidates[slot] {
                Some(index) if forced == Some(slot as u8) && index == self.representative => {}
                Some(index)
                    if forced != Some(slot as u8)
                        && usize::from(index) < count
                        && mask & (1 << index) != 0 => {}
                None if forced != Some(slot as u8) && mask == 0 => {}
                _ => return Err(invalid()),
            }
        }
        Ok(())
    }
}

/// Supervision labels are sets within the actual production decision factors.
/// None leaves a factor unconstrained, including compatible simultaneous actions.
#[derive(Debug, Clone, PartialEq)]
pub struct ImitationTarget {
    pub representative: Option<u32>,
    pub motors: [Option<u32>; PPO_MOTOR_SLOTS],
}

/// Demonstrations have no PPO old-policy probability. Context is only the legal
/// decoder masks/arbitration, never a teacher label inserted into neural features.
#[derive(Debug, Clone, PartialEq)]
pub struct ImitationExample {
    pub candidate_count: u16,
    pub representative_mask: u32,
    pub motor_masks: [u32; PPO_MOTOR_SLOTS],
    pub forced_slots: Vec<Option<u8>>,
    pub target: ImitationTarget,
}

impl ImitationExample {
    pub fn validate(&self) -> Result<(), TrainingError> {
        let count = usize::from(self.candidate_count);
        if count == 0 || count > PPO_MAX_CANDIDATES || self.forced_slots.len() != count {
            return Err(invalid());
        }
        let bits = u32::MAX >> (32 - count);
        if self.representative_mask == 0
            || self.representative_mask & !bits != 0
            || self
                .forced_slots
                .iter()
                .flatten()
                .any(|slot| usize::from(*slot) >= PPO_MOTOR_SLOTS)
            || self
                .target
                .representative
                .is_some_and(|mask| mask == 0 || mask & !self.representative_mask != 0)
            || (self.target.representative.is_none()
                && self.target.motors.iter().all(Option::is_none))
        {
            return Err(invalid());
        }
        for slot in 0..PPO_MOTOR_SLOTS {
            let possible = (0..count)
                .filter(|candidate| self.forced_slots[*candidate] == Some(slot as u8))
                .fold(0, |mask, candidate| mask | (1u32 << candidate))
                & self.representative_mask;
            if self.motor_masks[slot] & !possible != 0
                || self.target.motors[slot].is_some_and(|mask| mask == 0 || mask & !possible != 0)
            {
                return Err(invalid());
            }
        }
        // At least one representative must permit the entire labelled command.
        let possible = self
            .target
            .representative
            .unwrap_or(self.representative_mask);
        if !(0..count).any(|candidate| {
            possible & (1 << candidate) != 0
                && (0..PPO_MOTOR_SLOTS).all(|slot| {
                    self.target.motors[slot].is_none_or(|mask| {
                        if self.forced_slots[candidate] == Some(slot as u8) {
                            mask & (1 << candidate) != 0
                        } else {
                            mask & self.motor_masks[slot] != 0
                        }
                    })
                })
        }) {
            return Err(invalid());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PpoBoundary {
    Continuing,
    Terminated,
    /// Time limit or rollout cut: bootstrap the final state, never the reset state.
    Truncated,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PpoTransition {
    pub policy_version: u64,
    pub trajectory_id: u64,
    pub step: u64,
    pub action: PpoJointAction,
    pub reward: f32,
    /// GPU value predictions made before any update to this rollout's policy/head.
    pub old_value: f32,
    pub next_value: f32,
    pub elapsed_seconds: f32,
    pub boundary: PpoBoundary,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PpoBatch {
    policy_version: u64,
    transitions: Vec<PpoTransition>,
    advantages: Vec<f32>,
    returns: Vec<f32>,
}

impl PpoBatch {
    /// Scalar outcome accounting only. Each trajectory is contiguous and ends in
    /// termination or explicit truncation; GAE never crosses a reset boundary.
    pub fn from_rollout(
        policy_version: u64,
        transitions: Vec<PpoTransition>,
        config: PpoConfig,
    ) -> Result<Self, TrainingError> {
        config.validate()?;
        if transitions.is_empty() {
            return Err(invalid());
        }
        for (index, transition) in transitions.iter().enumerate() {
            transition.action.validate(config.temperature)?;
            if transition.policy_version != policy_version
                || ![
                    transition.reward,
                    transition.old_value,
                    transition.next_value,
                    transition.elapsed_seconds,
                ]
                .into_iter()
                .all(f32::is_finite)
                || transition.elapsed_seconds <= 0.0
                || (transition.boundary == PpoBoundary::Terminated && transition.next_value != 0.0)
            {
                return Err(invalid());
            }
            if transition.boundary == PpoBoundary::Continuing {
                let next = transitions.get(index + 1).ok_or_else(invalid)?;
                if next.trajectory_id != transition.trajectory_id
                    || transition.step.checked_add(1) != Some(next.step)
                    || transition.next_value.to_bits() != next.old_value.to_bits()
                {
                    return Err(invalid());
                }
            }
        }
        let mut advantages = vec![0.0; transitions.len()];
        let mut returns = vec![0.0; transitions.len()];
        let mut tail = 0.0_f64;
        for (index, transition) in transitions.iter().enumerate().rev() {
            let dt = f64::from(transition.elapsed_seconds);
            let gamma =
                (-std::f64::consts::LN_2 * dt / f64::from(config.discount_half_life_seconds)).exp();
            let lambda =
                (-std::f64::consts::LN_2 * dt / f64::from(config.gae_half_life_seconds)).exp();
            let bootstrap = if transition.boundary == PpoBoundary::Terminated {
                0.0
            } else {
                f64::from(transition.next_value)
            };
            let delta =
                f64::from(transition.reward) + gamma * bootstrap - f64::from(transition.old_value);
            tail = delta
                + if transition.boundary == PpoBoundary::Continuing {
                    gamma * lambda * tail
                } else {
                    0.0
                };
            advantages[index] = tail as f32;
            returns[index] = (tail + f64::from(transition.old_value)) as f32;
            if !advantages[index].is_finite() || !returns[index].is_finite() {
                return Err(invalid());
            }
        }
        Ok(Self {
            policy_version,
            transitions,
            advantages,
            returns,
        })
    }

    pub fn len(&self) -> usize {
        self.transitions.len()
    }
    pub fn is_empty(&self) -> bool {
        self.transitions.is_empty()
    }
    pub fn policy_version(&self) -> u64 {
        self.policy_version
    }
    pub fn advantages(&self) -> &[f32] {
        &self.advantages
    }
    pub fn returns(&self) -> &[f32] {
        &self.returns
    }
    pub fn transitions(&self) -> &[PpoTransition] {
        &self.transitions
    }

    /// Validate once against the collection policy before starting its bounded
    /// optimization epochs. Epochs reuse these frozen old probabilities/values.
    /// Slice AFTER full-trajectory GAE. A replay-window edge is not a biological
    /// termination or a truncation of the original return calculation.
    pub fn window(&self, range: std::ops::Range<usize>) -> Result<Self, TrainingError> {
        if range.start >= range.end || range.end > self.len() {
            return Err(invalid());
        }
        Ok(Self {
            policy_version: self.policy_version,
            transitions: self.transitions[range.clone()].to_vec(),
            advantages: self.advantages[range.clone()].to_vec(),
            returns: self.returns[range].to_vec(),
        })
    }

    pub fn validate_policy_version(&self, collection_version: u64) -> Result<(), TrainingError> {
        if self.policy_version != collection_version {
            return Err(invalid());
        }
        Ok(())
    }
}

/// Word offsets into the trainer's existing replay logit and activation buffers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PpoReplayRow {
    pub logits_word_offset: u32,
    pub features_word_offset: u32,
}

/// Existing-session GPU kernels. The trainer owns recurrent replay and applies
/// the returned logit adjoints through its existing sparse BPTT/optimizer passes.
pub struct PpoGpuObjective {
    header: wgpu::Buffer,
    rows: wgpu::Buffer,
    output: wgpu::Buffer,
    value_head: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    value_forward: wgpu::ComputePipeline,
    objective: wgpu::ComputePipeline,
    value_gradients: wgpu::ComputePipeline,
    value_preflight: wgpu::ComputePipeline,
    value_update: wgpu::ComputePipeline,
    imitation_replace: wgpu::ComputePipeline,
    imitation_add: wgpu::ComputePipeline,
    value_accum_clear: wgpu::ComputePipeline,
    value_accum_add: wgpu::ComputePipeline,
    value_accum_finish: wgpu::ComputePipeline,
    imitation_ready: bool,
    capacity: u32,
    feature_count: u32,
    sample_count: u32,
    objective_ready: bool,
    logits_words: u64,
    feature_words: u64,
}

impl PpoGpuObjective {
    pub fn new(
        session: &GpuAuthoritativeSession,
        logits: &wgpu::Buffer,
        features: &wgpu::Buffer,
        feature_count: u32,
        sample_capacity: u32,
    ) -> Result<Self, TrainingError> {
        if session.authority().consumer() != GpuSessionConsumerKind::Training
            || feature_count == 0
            || sample_capacity == 0
            || sample_capacity > 4096
            || !logits.usage().contains(wgpu::BufferUsages::STORAGE)
            || !features.usage().contains(wgpu::BufferUsages::STORAGE)
        {
            return Err(invalid());
        }
        let (device, _) = session.backend().offline_training_device_queue()?;
        if (u64::from(feature_count) + 1) * 20
            > u64::from(device.limits().max_storage_buffer_binding_size)
        {
            return Err(invalid());
        }
        let make = |label, size, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        let header = make(
            "ppo-header",
            128,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );
        let rows = make(
            "ppo-rollout-rows",
            u64::from(sample_capacity) * PPO_ROW_WORDS as u64 * 4,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let output = make(
            "ppo-adjoints-metrics-values",
            (u64::from(sample_capacity) * 45 + 1) * 4,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        // Zero-initialized linear head: [weights+bias, Adam m, Adam v, gradients, batch accumulation].
        let value_head = make(
            "ppo-detached-value-head",
            (u64::from(feature_count) + 1) * 20,
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        );
        let entries = (0..6)
            .map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: if binding == 0 {
                        wgpu::BufferBindingType::Uniform
                    } else {
                        wgpu::BufferBindingType::Storage {
                            read_only: binding <= 3,
                        }
                    },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            })
            .collect::<Vec<_>>();
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ppo-objective-layout"),
            entries: &entries,
        });
        let buffers = [&header, &rows, logits, features, &output, &value_head];
        let bindings = buffers
            .iter()
            .enumerate()
            .map(|(binding, buffer)| wgpu::BindGroupEntry {
                binding: binding as u32,
                resource: buffer.as_entire_binding(),
            })
            .collect::<Vec<_>>();
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ppo-objective-bind-group"),
            layout: &layout,
            entries: &bindings,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ppo-objective-pipeline-layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("foundation-ppo"),
            source: wgpu::ShaderSource::Wgsl(PPO_WGSL.into()),
        });
        let pipeline = |entry| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        Ok(Self {
            value_forward: pipeline("ppo_value_forward"),
            objective: pipeline("ppo_objective"),
            value_gradients: pipeline("ppo_value_gradients"),
            value_preflight: pipeline("ppo_value_preflight"),
            value_update: pipeline("ppo_value_update"),
            imitation_replace: pipeline("imitation_replace"),
            imitation_add: pipeline("imitation_add"),
            value_accum_clear: pipeline("value_accum_clear"),
            value_accum_add: pipeline("value_accum_add"),
            value_accum_finish: pipeline("value_accum_finish"),
            imitation_ready: false,
            header,
            rows,
            output,
            value_head,
            bind_group_layout: layout,
            bind_group,
            capacity: sample_capacity,
            feature_count,
            sample_count: 0,
            objective_ready: false,
            logits_words: logits.size() / 4,
            feature_words: features.size() / 4,
        })
    }

    /// Replay scratch may be resized between collections. Preserve this learned
    /// value head and its optimizer moments while replacing only buffer views.
    pub fn rebind_buffers(
        &mut self,
        session: &GpuAuthoritativeSession,
        logits: &wgpu::Buffer,
        features: &wgpu::Buffer,
    ) -> Result<(), TrainingError> {
        if session.authority().consumer() != GpuSessionConsumerKind::Training
            || !logits.usage().contains(wgpu::BufferUsages::STORAGE)
            || !features.usage().contains(wgpu::BufferUsages::STORAGE)
        {
            return Err(invalid());
        }
        let (device, _) = session.backend().offline_training_device_queue()?;
        let buffers = [
            &self.header,
            &self.rows,
            logits,
            features,
            &self.output,
            &self.value_head,
        ];
        let entries = buffers
            .iter()
            .enumerate()
            .map(|(binding, buffer)| wgpu::BindGroupEntry {
                binding: binding as u32,
                resource: buffer.as_entire_binding(),
            })
            .collect::<Vec<_>>();
        self.bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ppo-objective-bind-group"),
            layout: &self.bind_group_layout,
            entries: &entries,
        });
        self.logits_words = logits.size() / 4;
        self.feature_words = features.size() / 4;
        self.sample_count = 0;
        self.objective_ready = false;
        self.imitation_ready = false;
        Ok(())
    }

    pub fn upload(
        &mut self,
        session: &GpuAuthoritativeSession,
        batch: &PpoBatch,
        replay: &[PpoReplayRow],
        config: PpoConfig,
        value_optimizer_step: u32,
        collection_policy_version: u64,
    ) -> Result<(), TrainingError> {
        config.validate()?;
        batch.validate_policy_version(collection_policy_version)?;
        if session.authority().consumer() != GpuSessionConsumerKind::Training
            || batch.len() > self.capacity as usize
            || replay.len() != batch.len()
            || value_optimizer_step == 0
        {
            return Err(invalid());
        }
        let mut rows = vec![0_u32; batch.len() * PPO_ROW_WORDS];
        for (index, (transition, replay)) in batch.transitions.iter().zip(replay).enumerate() {
            let action = &transition.action;
            action.validate(config.temperature)?;
            if u64::from(replay.logits_word_offset) + u64::from(action.candidate_count)
                > self.logits_words
                || u64::from(replay.features_word_offset) + u64::from(self.feature_count)
                    > self.feature_words
            {
                return Err(invalid());
            }
            let row = &mut rows[index * PPO_ROW_WORDS..(index + 1) * PPO_ROW_WORDS];
            row[0] = u32::from(action.candidate_count);
            row[1] = u32::from(action.representative);
            row[2] = action.old_joint_log_probability.to_bits();
            row[3] = batch.advantages[index].to_bits();
            row[4] = batch.returns[index].to_bits();
            row[5] = transition.old_value.to_bits();
            row[6] = action.representative_mask;
            row[7..13].copy_from_slice(&action.motor_masks);
            for slot in 0..PPO_MOTOR_SLOTS {
                row[13 + slot] = action.motor_candidates[slot].map_or(u32::MAX, u32::from);
            }
            row[19..51].fill(u32::MAX);
            for (candidate, slot) in action.forced_slots.iter().enumerate() {
                row[19 + candidate] = slot.map_or(u32::MAX, u32::from);
            }
            row[51] = replay.logits_word_offset;
            row[52] = replay.features_word_offset;
        }
        let adam = config.value_optimizer;
        let header_prefix = [
            batch.len() as u32,
            self.feature_count,
            self.capacity,
            value_optimizer_step,
            config.clip_ratio.to_bits(),
            config.entropy_coefficient.to_bits(),
            config.value_coefficient.to_bits(),
            config.temperature.to_bits(),
            adam.learning_rate.to_bits(),
            adam.beta1.to_bits(),
            adam.beta2.to_bits(),
            adam.epsilon.to_bits(),
            adam.weight_decay.to_bits(),
            adam.gradient_clip.to_bits(),
            config.target_kl.to_bits(),
            0,
        ];
        let mut header = [0_u32; 32];
        header[..16].copy_from_slice(&header_prefix);
        let (_, queue) = session.backend().offline_training_device_queue()?;
        queue.write_buffer(&self.header, 0, bytemuck::cast_slice(&header));
        queue.write_buffer(&self.rows, 0, bytemuck::cast_slice(&rows));
        self.imitation_ready = false;
        self.sample_count = batch.len() as u32;
        self.objective_ready = true;
        Ok(())
    }

    /// Evaluate old/bootstrap values without fabricating rewards or an action
    /// rollout. Features are existing GPU activations; head weights stay frozen.
    pub fn upload_value_features(
        &mut self,
        session: &GpuAuthoritativeSession,
        feature_word_offsets: &[u32],
    ) -> Result<(), TrainingError> {
        if session.authority().consumer() != GpuSessionConsumerKind::Training
            || feature_word_offsets.is_empty()
            || feature_word_offsets.len() > self.capacity as usize
            || feature_word_offsets.iter().any(|offset| {
                u64::from(*offset) + u64::from(self.feature_count) > self.feature_words
            })
        {
            return Err(invalid());
        }
        let mut rows = vec![0_u32; feature_word_offsets.len() * PPO_ROW_WORDS];
        for (index, offset) in feature_word_offsets.iter().enumerate() {
            rows[index * PPO_ROW_WORDS + 52] = *offset;
        }
        let mut header = [0_u32; 32];
        header[..3].copy_from_slice(&[
            feature_word_offsets.len() as u32,
            self.feature_count,
            self.capacity,
        ]);
        let (_, queue) = session.backend().offline_training_device_queue()?;
        queue.write_buffer(&self.header, 0, bytemuck::cast_slice(&header));
        queue.write_buffer(&self.rows, 0, bytemuck::cast_slice(&rows));
        self.sample_count = feature_word_offsets.len() as u32;
        self.objective_ready = false;
        self.imitation_ready = false;
        Ok(())
    }

    pub fn encode_value_predictions(
        &self,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), TrainingError> {
        if self.sample_count == 0 {
            return Err(invalid());
        }
        self.dispatch(encoder, &self.value_forward, self.sample_count);
        Ok(())
    }

    /// No update occurs here. Read/validate metrics and enforce the KL gate before
    /// dispatching either the backbone optimizer or encode_value_update.
    pub fn encode_evaluate(&self, encoder: &mut wgpu::CommandEncoder) -> Result<(), TrainingError> {
        if !self.objective_ready {
            return Err(invalid());
        }
        self.dispatch(encoder, &self.value_forward, self.sample_count);
        self.dispatch(encoder, &self.objective, self.sample_count);
        self.dispatch(
            encoder,
            &self.value_gradients,
            (self.feature_count + 1).div_ceil(64),
        );
        self.dispatch(encoder, &self.value_preflight, 1);
        Ok(())
    }

    pub fn encode_value_update(
        &self,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), TrainingError> {
        if !self.objective_ready {
            return Err(invalid());
        }
        self.dispatch(encoder, &self.value_update, 1);
        Ok(())
    }

    /// Warmup uses real replay masks without inventing an on-policy probability.
    pub fn upload_imitation(
        &mut self,
        session: &GpuAuthoritativeSession,
        examples: &[ImitationExample],
        replay: &[PpoReplayRow],
        temperature: f32,
        coefficient: f32,
    ) -> Result<(), TrainingError> {
        if examples.is_empty()
            || examples.len() > self.capacity as usize
            || examples.len() != replay.len()
            || !temperature.is_finite()
            || temperature <= 0.0
            || !coefficient.is_finite()
            || coefficient < 0.0
        {
            return Err(invalid());
        }
        let mut rows = vec![0u32; examples.len() * PPO_ROW_WORDS];
        for (index, (example, replay)) in examples.iter().zip(replay).enumerate() {
            example.validate()?;
            if u64::from(replay.logits_word_offset) + u64::from(example.candidate_count)
                > self.logits_words
            {
                return Err(invalid());
            }
            let row = &mut rows[index * PPO_ROW_WORDS..(index + 1) * PPO_ROW_WORDS];
            row[0] = u32::from(example.candidate_count);
            row[6] = example.representative_mask;
            row[7..13].copy_from_slice(&example.motor_masks);
            row[19..51].fill(u32::MAX);
            for (candidate, slot) in example.forced_slots.iter().enumerate() {
                row[19 + candidate] = slot.map_or(u32::MAX, u32::from);
            }
            row[51] = replay.logits_word_offset;
            Self::pack_target(row, &example.target);
        }
        let mut header = [0u32; 32];
        header[..3].copy_from_slice(&[examples.len() as u32, self.feature_count, self.capacity]);
        header[7] = temperature.to_bits();
        header[15] = coefficient.to_bits();
        let (_, queue) = session.backend().offline_training_device_queue()?;
        queue.write_buffer(&self.header, 0, bytemuck::cast_slice(&header));
        queue.write_buffer(&self.rows, 0, bytemuck::cast_slice(&rows));
        self.sample_count = examples.len() as u32;
        self.objective_ready = false;
        self.imitation_ready = true;
        Ok(())
    }

    /// Attach labels after PPO upload; exact context comes from that same batch.
    pub fn upload_auxiliary_targets(
        &mut self,
        session: &GpuAuthoritativeSession,
        batch: &PpoBatch,
        targets: &[ImitationTarget],
        coefficient: f32,
    ) -> Result<(), TrainingError> {
        if !self.objective_ready
            || targets.len() != batch.len()
            || batch.len() != self.sample_count as usize
            || !coefficient.is_finite()
            || coefficient < 0.0
        {
            return Err(invalid());
        }
        let mut packed = Vec::with_capacity(targets.len());
        for (transition, target) in batch.transitions.iter().zip(targets) {
            let action = &transition.action;
            ImitationExample {
                candidate_count: action.candidate_count,
                representative_mask: action.representative_mask,
                motor_masks: action.motor_masks,
                forced_slots: action.forced_slots.clone(),
                target: target.clone(),
            }
            .validate()?;
            let mut row = [0u32; PPO_ROW_WORDS];
            Self::pack_target(&mut row, target);
            packed.push(row);
        }
        let (_, queue) = session.backend().offline_training_device_queue()?;
        for (index, row) in packed.iter().enumerate() {
            queue.write_buffer(
                &self.rows,
                ((index * PPO_ROW_WORDS + 53) * 4) as u64,
                bytemuck::cast_slice(&row[53..61]),
            );
        }
        queue.write_buffer(&self.header, 15 * 4, bytemuck::bytes_of(&coefficient));
        self.imitation_ready = true;
        Ok(())
    }

    fn pack_target(row: &mut [u32], target: &ImitationTarget) {
        for (factor, mask) in std::iter::once(target.representative)
            .chain(target.motors)
            .enumerate()
        {
            if let Some(mask) = mask {
                row[53 + factor] = mask;
                row[60] |= 1 << factor;
            }
        }
    }

    pub fn encode_imitation(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        add_to_ppo: bool,
    ) -> Result<(), TrainingError> {
        if !self.imitation_ready || (add_to_ppo && !self.objective_ready) {
            return Err(invalid());
        }
        self.dispatch(
            encoder,
            if add_to_ppo {
                &self.imitation_add
            } else {
                &self.imitation_replace
            },
            self.sample_count,
        );
        Ok(())
    }
    pub fn imitation_metric_bytes(&self) -> std::ops::Range<u64> {
        let start = (u64::from(self.capacity) * 41 + 1) * 4;
        start..start + u64::from(self.sample_count) * 4 * 4
    }
    pub fn encode_clear_value_accumulation(&self, encoder: &mut wgpu::CommandEncoder) {
        self.dispatch(
            encoder,
            &self.value_accum_clear,
            (self.feature_count + 1).div_ceil(64),
        );
    }
    pub fn encode_accumulate_value_gradients(
        &self,
        session: &GpuAuthoritativeSession,
        encoder: &mut wgpu::CommandEncoder,
        scale: f32,
    ) -> Result<(), TrainingError> {
        if !self.objective_ready || !scale.is_finite() || scale <= 0.0 {
            return Err(invalid());
        }
        let (_, queue) = session.backend().offline_training_device_queue()?;
        queue.write_buffer(&self.header, 16 * 4, bytemuck::bytes_of(&scale));
        self.dispatch(
            encoder,
            &self.value_accum_add,
            (self.feature_count + 1).div_ceil(64),
        );
        Ok(())
    }
    pub fn encode_finish_value_accumulation(
        &self,
        session: &GpuAuthoritativeSession,
        encoder: &mut wgpu::CommandEncoder,
        mean_kl: f32,
    ) -> Result<(), TrainingError> {
        if !mean_kl.is_finite() || mean_kl < 0.0 {
            return Err(invalid());
        }
        let (_, queue) = session.backend().offline_training_device_queue()?;
        queue.write_buffer(
            &self.header,
            17 * 4,
            bytemuck::cast_slice(&[1u32, mean_kl.to_bits()]),
        );
        self.dispatch(
            encoder,
            &self.value_accum_finish,
            (self.feature_count + 1).div_ceil(64),
        );
        self.dispatch(encoder, &self.value_preflight, 1);
        Ok(())
    }

    pub fn output_buffer(&self) -> &wgpu::Buffer {
        &self.output
    }
    pub fn value_head_buffer(&self) -> &wgpu::Buffer {
        &self.value_head
    }
    pub fn adjoint_bytes(&self) -> std::ops::Range<u64> {
        0..u64::from(self.sample_count) * 32 * 4
    }
    pub fn metric_bytes(&self) -> std::ops::Range<u64> {
        let start = u64::from(self.capacity) * 32 * 4;
        start..start + u64::from(self.sample_count) * PPO_METRIC_WORDS as u64 * 4
    }
    pub fn value_bytes(&self) -> std::ops::Range<u64> {
        let start = u64::from(self.capacity) * 40 * 4;
        start..start + u64::from(self.sample_count) * 4
    }
    pub fn value_preflight_bytes(&self) -> std::ops::Range<u64> {
        let start = u64::from(self.capacity) * 41 * 4;
        start..start + 4
    }

    fn dispatch(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        pipeline: &wgpu::ComputePipeline,
        groups: u32,
    ) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("foundation-ppo"),
            timestamp_writes: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.dispatch_workgroups(groups, 1, 1);
    }
}

/// Metrics are per-row, not already averaged: new logp, approximate KL, exact
/// joint entropy, policy loss, weighted value loss, clip indicator, ratio, valid.
pub fn ppo_mean_kl(metrics: &[f32]) -> Result<f32, TrainingError> {
    if metrics.is_empty()
        || metrics.len() % PPO_METRIC_WORDS != 0
        || !metrics.iter().all(|value| value.is_finite())
        || metrics
            .chunks_exact(PPO_METRIC_WORDS)
            .any(|row| row[7] != 1.0)
    {
        return Err(TrainingError::MalformedReadback);
    }
    let mean = metrics
        .chunks_exact(PPO_METRIC_WORDS)
        .map(|row| row[1] as f64)
        .sum::<f64>()
        / (metrics.len() / PPO_METRIC_WORDS) as f64;
    if mean < 0.0 || !(mean as f32).is_finite() {
        return Err(TrainingError::MalformedReadback);
    }
    Ok(mean as f32)
}

/// Training checkpoint only. The campaign checkpoint also binds the actor asset,
/// compiler inputs, replay context and optimizer; this is never a creature save.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PpoValueHeadCheckpoint {
    pub feature_count: u32,
    pub optimizer_step: u32,
    /// Linear coefficients+bias, Adam first moments, Adam second moments.
    pub parameters: Vec<f32>,
    pub last_updated_policy_version: Option<u64>,
}

impl PpoValueHeadCheckpoint {
    fn validate(&self) -> Result<(), TrainingError> {
        let width = self.feature_count as usize + 1;
        if self.feature_count == 0
            || self.parameters.len() != width * 3
            || !self.parameters.iter().all(|value| value.is_finite())
            || self.parameters[width * 2..]
                .iter()
                .any(|value| *value < 0.0)
        {
            return Err(invalid());
        }
        Ok(())
    }
}

/// Persistent training-only value head. Replay scratch is rebound, never used to
/// create another neural engine. Policy versions are campaign genetic versions,
/// not the per-tick generation of acquired runtime weights.
#[derive(Default)]
pub struct PpoTrainingState {
    objective: Option<PpoGpuObjective>,
    pending_restore: Option<PpoValueHeadCheckpoint>,
    value_optimizer_step: u32,
    last_updated_policy_version: Option<u64>,
}

impl PpoTrainingState {
    pub fn from_checkpoint(checkpoint: PpoValueHeadCheckpoint) -> Result<Self, TrainingError> {
        checkpoint.validate()?;
        Ok(Self {
            value_optimizer_step: checkpoint.optimizer_step,
            last_updated_policy_version: checkpoint.last_updated_policy_version,
            pending_restore: Some(checkpoint),
            objective: None,
        })
    }

    /// Call after trainer.prepare_replay, including for pre-update GPU value
    /// collection. Rebinding preserves the learned head and its Adam moments.
    /// After rebind_for_next_cohort, keep this same state and prepare a fresh
    /// sequence from the newly admitted organisms before calling this method.
    pub fn prepare_for_replay(
        &mut self,
        trainer: &crate::FoundationTrainer,
    ) -> Result<(), TrainingError> {
        // Reject stale replay buffers invalidated by a cohort identity change.
        trainer.replay_all_rows()?;
        let feature_count = trainer.phenotype().neuron_count();
        if let Some(checkpoint) = &self.pending_restore {
            checkpoint.validate()?;
            if checkpoint.feature_count != feature_count {
                return Err(invalid());
            }
        }
        if let Some(objective) = &mut self.objective {
            if objective.feature_count != feature_count {
                return Err(invalid());
            }
            objective.rebind_buffers(
                trainer.session(),
                trainer.replay_output_buffer(),
                trainer.replay_state_buffer(),
            )?;
        } else {
            self.objective = Some(PpoGpuObjective::new(
                trainer.session(),
                trainer.replay_output_buffer(),
                trainer.replay_state_buffer(),
                feature_count,
                crate::MAX_TRAINING_SEQUENCE_TICKS as u32,
            )?);
        }
        if let Some(checkpoint) = self.pending_restore.take() {
            let (_, queue) = trainer
                .session()
                .backend()
                .offline_training_device_queue()?;
            queue.write_buffer(
                &self.objective.as_ref().ok_or_else(invalid)?.value_head,
                0,
                bytemuck::cast_slice(&checkpoint.parameters),
            );
        }
        Ok(())
    }

    /// Frozen scalar predictions for every captured decision state, including
    /// burn-in. To bootstrap a truncated trajectory, also capture its next state;
    /// this helper never invents that state or advances the authoritative world.
    pub fn predict_values(
        &mut self,
        trainer: &mut crate::FoundationTrainer,
        sequence: &crate::TrainingSequence,
    ) -> Result<Vec<f32>, TrainingError> {
        trainer.prepare_replay(sequence)?;
        self.prepare_for_replay(trainer)?;
        let offsets = trainer
            .replay_all_rows()?
            .iter()
            .map(|row| row.features_word_offset)
            .collect::<Vec<_>>();
        let objective = self.objective.as_mut().ok_or_else(invalid)?;
        objective.upload_value_features(trainer.session(), &offsets)?;
        let mut encoder = new_encoder(trainer.session(), "ppo-frozen-value-predictions")?;
        trainer.encode_replay_forward(&mut encoder)?;
        objective.encode_value_predictions(&mut encoder)?;
        submit(trainer.session(), encoder)?;
        let values = read_gpu_f32(
            trainer.session(),
            &objective.output,
            objective.value_bytes(),
        )?;
        if !values.iter().all(|value| value.is_finite()) {
            return Err(TrainingError::MalformedReadback);
        }
        Ok(values)
    }

    pub fn objective_mut(&mut self) -> Result<&mut PpoGpuObjective, TrainingError> {
        self.objective.as_mut().ok_or_else(invalid)
    }

    pub fn checkpoint(
        &self,
        session: &GpuAuthoritativeSession,
    ) -> Result<PpoValueHeadCheckpoint, TrainingError> {
        if let Some(checkpoint) = &self.pending_restore {
            return Ok(checkpoint.clone());
        }
        let objective = self.objective.as_ref().ok_or_else(invalid)?;
        let parameters = read_gpu_f32(
            session,
            &objective.value_head,
            0..(u64::from(objective.feature_count) + 1) * 3 * 4,
        )?;
        let checkpoint = PpoValueHeadCheckpoint {
            feature_count: objective.feature_count,
            optimizer_step: self.value_optimizer_step,
            parameters,
            last_updated_policy_version: self.last_updated_policy_version,
        };
        checkpoint.validate()?;
        Ok(checkpoint)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PpoUpdateReceipt {
    pub collection_policy_version: u64,
    pub completed_epochs: u32,
    pub stopped_for_kl: bool,
    /// Measured before each attempted update, not a bound on the final update.
    pub mean_kl_before_updates: Vec<f32>,
    pub actor_optimizer_step: u32,
    pub value_optimizer_step: u32,
}

/// One contiguous recurrent replay window. Compute GAE over the original
/// trajectory before slicing the batch; burn-in remains excluded from its loss.
#[derive(Debug, Clone)]
pub struct PpoTrainingWindow {
    pub sequence: crate::TrainingSequence,
    pub batch: PpoBatch,
    pub auxiliary: Option<(Vec<ImitationTarget>, f32)>,
}

/// Compatibility entry for an explicit single-window update. Campaigns should
/// use train_recurrent_ppo_cohort with an effective batch of eight windows.
pub fn train_recurrent_ppo(
    trainer: &mut crate::FoundationTrainer,
    state: &mut PpoTrainingState,
    sequence: &crate::TrainingSequence,
    batch: &PpoBatch,
    config: PpoConfig,
    collection_policy_version: u64,
) -> Result<PpoUpdateReceipt, TrainingError> {
    train_recurrent_ppo_cohort(
        trainer,
        state,
        1,
        1,
        |_| {
            Ok(PpoTrainingWindow {
                sequence: sequence.clone(),
                batch: batch.clone(),
                auxiliary: None,
            })
        },
        config,
        collection_policy_version,
    )
}

/// Stream a pinned-actor cohort from immutable recorded windows. Validate all
/// old probabilities before ANY updates, then run global epochs. Every batch
/// takes one actor and one value Adam step, sample-weighted across its windows.
/// The final partial batch deliberately uses its actual sample count.
///
/// The caller binds the genetic policy version to the exact frozen checkpoint
/// and must return identical data on repeated loads. Acquired bank generations
/// stay in replay context. After updates, finite burn-in is an approximation to
/// the changed recurrent trajectory, not an assertion of exact old-state replay.
/// Checkpoint at cohort boundaries; an interrupted cohort needs caller rollback
/// to its pre-update checkpoint rather than reuse of partially updated data.
pub fn train_recurrent_ppo_cohort<F>(
    trainer: &mut crate::FoundationTrainer,
    state: &mut PpoTrainingState,
    window_count: usize,
    effective_batch_size: usize,
    mut load_window: F,
    config: PpoConfig,
    collection_policy_version: u64,
) -> Result<PpoUpdateReceipt, TrainingError>
where
    F: FnMut(usize) -> Result<PpoTrainingWindow, TrainingError>,
{
    config.validate()?;
    if window_count == 0
        || effective_batch_size == 0
        || effective_batch_size > 64
        || state
            .last_updated_policy_version
            .is_some_and(|v| collection_policy_version <= v)
    {
        return Err(invalid());
    }
    let mut counts = Vec::with_capacity(window_count);
    // This separate frozen pass is necessary: checking ratio==1 after the first
    // minibatch would reject valid later windows from this same on-policy cohort.
    for index in 0..window_count {
        let window = load_window(index)?;
        let metrics =
            evaluate_ppo_window(trainer, state, &window, config, collection_policy_version)?;
        if metrics
            .chunks_exact(PPO_METRIC_WORDS)
            .zip(window.batch.transitions())
            .any(|(row, old)| (row[0] - old.action.old_joint_log_probability).abs() > 1.0e-3)
        {
            return Err(invalid());
        }
        counts.push(window.batch.len());
    }
    let mut receipt = PpoUpdateReceipt {
        collection_policy_version,
        completed_epochs: 0,
        stopped_for_kl: false,
        mean_kl_before_updates: Vec::new(),
        actor_optimizer_step: trainer.optimizer_step(),
        value_optimizer_step: state.value_optimizer_step,
    };
    for _epoch in 0..config.epochs {
        for start in (0..window_count).step_by(effective_batch_size) {
            let end = (start + effective_batch_size).min(window_count);
            let samples = counts[start..end].iter().sum::<usize>();
            trainer.begin_gradient_accumulation((end - start) as u32)?;
            let result = (|| -> Result<bool, TrainingError> {
                let mut mean_kl = 0.0f64;
                for index in start..end {
                    let window = load_window(index)?;
                    if window.batch.len() != counts[index] {
                        return Err(invalid());
                    }
                    let metrics = evaluate_ppo_window(
                        trainer,
                        state,
                        &window,
                        config,
                        collection_policy_version,
                    )?;
                    let scale = counts[index] as f32 / samples as f32;
                    mean_kl +=
                        f64::from(ppo_mean_kl(&metrics)?) * counts[index] as f64 / samples as f64;
                    let objective = state.objective.as_ref().ok_or_else(invalid)?;
                    let mut encoder = new_encoder(trainer.session(), "ppo-accumulate")?;
                    if index == start {
                        objective.encode_clear_value_accumulation(&mut encoder);
                    }
                    objective.encode_accumulate_value_gradients(
                        trainer.session(),
                        &mut encoder,
                        scale,
                    )?;
                    trainer.accumulate_replay_gradients(encoder, &objective.output, 0, scale)?;
                }
                let mean_kl = mean_kl as f32;
                if !mean_kl.is_finite() {
                    return Err(TrainingError::MalformedReadback);
                }
                receipt.mean_kl_before_updates.push(mean_kl);
                if mean_kl > config.target_kl {
                    receipt.stopped_for_kl = true;
                    return Ok(false);
                }
                let objective = state.objective.as_ref().ok_or_else(invalid)?;
                let mut encoder = new_encoder(trainer.session(), "ppo-value-batch-preflight")?;
                objective.encode_finish_value_accumulation(
                    trainer.session(),
                    &mut encoder,
                    mean_kl,
                )?;
                submit(trainer.session(), encoder)?;
                if read_gpu_f32(
                    trainer.session(),
                    &objective.output,
                    objective.value_preflight_bytes(),
                )?
                .as_slice()
                    != [1.0]
                {
                    return Err(TrainingError::MalformedReadback);
                }
                let backup = {
                    let (device, _) = trainer
                        .session()
                        .backend()
                        .offline_training_device_queue()?;
                    device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("ppo-value-rollback"),
                        size: objective.value_head.size(),
                        usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    })
                };
                let mut encoder = new_encoder(trainer.session(), "ppo-effective-batch-update")?;
                encoder.copy_buffer_to_buffer(&objective.value_head, 0, &backup, 0, backup.size());
                submit(trainer.session(), encoder)?;
                // Backup is submitted independently so even a host-side actor
                // precondition error cannot restore an uninitialized buffer.
                let mut encoder = new_encoder(trainer.session(), "ppo-effective-batch-apply")?;
                objective.encode_value_update(&mut encoder)?;
                match trainer.apply_accumulated_gradients(encoder) {
                    Ok(step) => receipt.actor_optimizer_step = step,
                    Err(error) => {
                        let mut rollback = new_encoder(trainer.session(), "ppo-value-rollback")?;
                        rollback.copy_buffer_to_buffer(
                            &backup,
                            0,
                            &objective.value_head,
                            0,
                            backup.size(),
                        );
                        submit(trainer.session(), rollback)?;
                        let (device, _) = trainer
                            .session()
                            .backend()
                            .offline_training_device_queue()?;
                        device
                            .poll(wgpu::PollType::Wait {
                                submission_index: None,
                                timeout: None,
                            })
                            .map_err(|_| TrainingError::GpuSubmission)?;
                        return Err(error);
                    }
                }
                if config.value_coefficient > 0.0 {
                    state.value_optimizer_step += 1;
                }
                state.last_updated_policy_version = Some(collection_policy_version);
                receipt.value_optimizer_step = state.value_optimizer_step;
                Ok(true)
            })();
            match result {
                Ok(true) => {}
                Ok(false) => {
                    trainer.cancel_gradient_accumulation();
                    return Ok(receipt);
                }
                Err(error) => {
                    trainer.cancel_gradient_accumulation();
                    return Err(error);
                }
            }
        }
        receipt.completed_epochs += 1;
    }
    Ok(receipt)
}

fn evaluate_ppo_window(
    trainer: &mut crate::FoundationTrainer,
    state: &mut PpoTrainingState,
    window: &PpoTrainingWindow,
    config: PpoConfig,
    version: u64,
) -> Result<Vec<f32>, TrainingError> {
    trainer.prepare_replay(&window.sequence)?;
    state.prepare_for_replay(trainer)?;
    let replay = trainer.replay_rows()?;
    let next_step = state
        .value_optimizer_step
        .checked_add(1)
        .ok_or_else(invalid)?;
    let objective = state.objective.as_mut().ok_or_else(invalid)?;
    objective.upload(
        trainer.session(),
        &window.batch,
        &replay,
        config,
        next_step,
        version,
    )?;
    if let Some((targets, coefficient)) = &window.auxiliary {
        objective.upload_auxiliary_targets(
            trainer.session(),
            &window.batch,
            targets,
            *coefficient,
        )?;
    }
    let mut encoder = new_encoder(trainer.session(), "ppo-replay-objective")?;
    trainer.encode_replay_forward(&mut encoder)?;
    objective.encode_evaluate(&mut encoder)?;
    if window.auxiliary.is_some() {
        objective.encode_imitation(&mut encoder, true)?;
    }
    submit(trainer.session(), encoder)?;
    // One blocking copy includes PPO metrics and, when present, imitation metrics.
    // Values in the intervening range are not neural state; they are head outputs.
    let range = objective.metric_bytes().start..if window.auxiliary.is_some() {
        objective.imitation_metric_bytes().end
    } else {
        objective.metric_bytes().end
    };
    let data = read_gpu_f32(trainer.session(), &objective.output, range)?;
    let metrics = data[..window.batch.len() * PPO_METRIC_WORDS].to_vec();
    ppo_mean_kl(&metrics)?;
    if window.auxiliary.is_some() {
        let offset = ((objective.imitation_metric_bytes().start - objective.metric_bytes().start)
            / 4) as usize;
        imitation_mean_loss(&data[offset..])?;
    }
    Ok(metrics)
}

#[derive(Debug, Clone)]
pub struct ImitationTrainingWindow {
    pub sequence: crate::TrainingSequence,
    pub examples: Vec<ImitationExample>,
}

/// Warmup on the same recurrent graph. Labels are only consumed by the GPU
/// objective. Adam steps once per effective batch; value parameters stay frozen.
pub fn train_recurrent_imitation<F>(
    trainer: &mut crate::FoundationTrainer,
    state: &mut PpoTrainingState,
    window_count: usize,
    effective_batch_size: usize,
    epochs: u32,
    temperature: f32,
    coefficient: f32,
    mut load_window: F,
) -> Result<Vec<f32>, TrainingError>
where
    F: FnMut(usize) -> Result<ImitationTrainingWindow, TrainingError>,
{
    if window_count == 0
        || effective_batch_size == 0
        || effective_batch_size > 64
        || epochs == 0
        || !temperature.is_finite()
        || temperature <= 0.0
        || !coefficient.is_finite()
        || coefficient <= 0.0
    {
        return Err(invalid());
    }
    let mut losses = Vec::new();
    for _ in 0..epochs {
        for start in (0..window_count).step_by(effective_batch_size) {
            let end = (start + effective_batch_size).min(window_count);
            let counts = (start..end)
                .map(|index| load_window(index).map(|window| window.examples.len()))
                .collect::<Result<Vec<_>, _>>()?;
            if counts.iter().any(|count| *count == 0) {
                return Err(invalid());
            }
            trainer.begin_gradient_accumulation((end - start) as u32)?;
            let result = (|| -> Result<(), TrainingError> {
                let mut loss = 0.0f64;
                for index in start..end {
                    let window = load_window(index)?;
                    if window.examples.len() != counts[index - start] {
                        return Err(invalid());
                    }
                    trainer.prepare_replay(&window.sequence)?;
                    state.prepare_for_replay(trainer)?;
                    let objective = state.objective.as_mut().ok_or_else(invalid)?;
                    objective.upload_imitation(
                        trainer.session(),
                        &window.examples,
                        &trainer.replay_rows()?,
                        temperature,
                        coefficient,
                    )?;
                    let mut encoder = new_encoder(trainer.session(), "imitation-replay-objective")?;
                    trainer.encode_replay_forward(&mut encoder)?;
                    objective.encode_imitation(&mut encoder, false)?;
                    submit(trainer.session(), encoder)?;
                    let metrics = read_gpu_f32(
                        trainer.session(),
                        &objective.output,
                        objective.imitation_metric_bytes(),
                    )?;
                    // Each demonstration is one curriculum episode. Equal
                    // episode weight keeps a short feeding lesson from being
                    // drowned by longer hazard or recovery trajectories.
                    let scale = 1.0 / (end - start) as f32;
                    loss += f64::from(imitation_mean_loss(&metrics)?) * f64::from(scale);
                    let encoder = new_encoder(trainer.session(), "imitation-accumulate")?;
                    trainer.accumulate_replay_gradients(encoder, &objective.output, 0, scale)?;
                }
                let encoder = new_encoder(trainer.session(), "imitation-effective-batch-update")?;
                trainer.apply_accumulated_gradients(encoder)?;
                losses.push(loss as f32);
                Ok(())
            })();
            if let Err(error) = result {
                trainer.cancel_gradient_accumulation();
                return Err(error);
            }
        }
    }
    Ok(losses)
}

pub fn imitation_mean_loss(metrics: &[f32]) -> Result<f32, TrainingError> {
    if metrics.is_empty()
        || metrics.len() % 4 != 0
        || !metrics.iter().all(|x| x.is_finite())
        || metrics.chunks_exact(4).any(|row| row[3] != 1.0)
    {
        return Err(TrainingError::MalformedReadback);
    }
    let loss = metrics
        .chunks_exact(4)
        .map(|row| f64::from(row[1]))
        .sum::<f64>()
        / (metrics.len() / 4) as f64;
    if !(loss as f32).is_finite() {
        return Err(TrainingError::MalformedReadback);
    }
    Ok(loss as f32)
}

fn new_encoder(
    session: &GpuAuthoritativeSession,
    label: &str,
) -> Result<wgpu::CommandEncoder, TrainingError> {
    let (device, _) = session.backend().offline_training_device_queue()?;
    Ok(device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) }))
}
fn submit(
    session: &GpuAuthoritativeSession,
    encoder: wgpu::CommandEncoder,
) -> Result<(), TrainingError> {
    let (_, queue) = session.backend().offline_training_device_queue()?;
    queue.submit(Some(encoder.finish()));
    Ok(())
}

fn read_gpu_f32(
    session: &GpuAuthoritativeSession,
    source: &wgpu::Buffer,
    range: std::ops::Range<u64>,
) -> Result<Vec<f32>, TrainingError> {
    if session.authority().consumer() != GpuSessionConsumerKind::Training
        || range.start >= range.end
        || range.end > source.size()
        || range.start % 4 != 0
        || range.end % 4 != 0
    {
        return Err(invalid());
    }
    let (device, queue) = session.backend().offline_training_device_queue()?;
    let size = range.end - range.start;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ppo-readback"),
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("ppo-readback"),
    });
    encoder.copy_buffer_to_buffer(source, range.start, &readback, 0, size);
    let commands = encoder.finish();
    let (sender, receiver) = std::sync::mpsc::channel();
    commands.map_buffer_on_submit(&readback, wgpu::MapMode::Read, 0..size, move |result| {
        let _ = sender.send(result);
    });
    let submission = queue.submit(Some(commands));
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
    let mapped = readback.slice(..size).get_mapped_range();
    let values = bytemuck::cast_slice::<u8, f32>(&mapped).to_vec();
    drop(mapped);
    readback.unmap();
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ppo_rollout_keeps_bootstrap_boundaries_masks_and_policy_version() {
        let shader = naga::front::wgsl::parse_str(PPO_WGSL).expect("PPO WGSL parses");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&shader)
        .expect("PPO WGSL validates");
        let config = PpoConfig::default();
        // Idle forces posture but is not an independently sampled posture factor.
        let action = PpoJointAction {
            candidate_count: 1,
            representative_mask: 1,
            motor_masks: [0; 6],
            representative: 0,
            forced_slots: vec![Some(4)],
            motor_candidates: [None, None, None, None, Some(0), None],
            old_joint_log_probability: 0.0,
            temperature: 1.0,
        };
        let example = ImitationExample {
            candidate_count: 3,
            representative_mask: 7,
            motor_masks: [1, 0, 2, 0, 0, 0],
            forced_slots: vec![Some(0), Some(2), Some(4)],
            target: ImitationTarget {
                representative: None,
                motors: [None, None, Some(2), None, None, None],
            },
        };
        example.validate().unwrap(); // Idle may override posture while Eat stays acceptable.
        let mut impossible = example.clone();
        impossible.target.representative = Some(1);
        impossible.target.motors[0] = Some(2); // Eat is not a move-slot action.
        assert!(impossible.validate().is_err());
        let mut idle = example;
        idle.target = ImitationTarget {
            representative: Some(4),
            motors: [None, None, None, None, Some(4), None],
        };
        idle.validate().unwrap(); // Forced posture is valid even with empty posture mask.
        let transitions = vec![
            PpoTransition {
                policy_version: 7,
                trajectory_id: 1,
                step: 0,
                action: action.clone(),
                reward: 1.0,
                old_value: 2.0,
                next_value: 4.0,
                elapsed_seconds: 60.0,
                boundary: PpoBoundary::Continuing,
            },
            PpoTransition {
                policy_version: 7,
                trajectory_id: 1,
                step: 1,
                action: action.clone(),
                reward: 3.0,
                old_value: 4.0,
                next_value: 8.0,
                elapsed_seconds: 60.0,
                boundary: PpoBoundary::Truncated,
            },
            PpoTransition {
                policy_version: 7,
                trajectory_id: 2,
                step: 0,
                action,
                reward: 5.0,
                old_value: 6.0,
                next_value: 0.0,
                elapsed_seconds: 60.0,
                boundary: PpoBoundary::Terminated,
            },
        ];
        let batch = PpoBatch::from_rollout(7, transitions.clone(), config).unwrap();
        assert_eq!(batch.advantages(), &[1.0234375, 3.0, -1.0]);
        assert_eq!(batch.returns(), &[3.0234375, 7.0, 5.0]);
        assert!(batch.validate_policy_version(8).is_err());
        assert!(PpoBatch::from_rollout(8, transitions.clone(), config).is_err());
        let mut invalid_rollout = transitions.clone();
        invalid_rollout[0].action.representative_mask = 0;
        assert!(PpoBatch::from_rollout(7, invalid_rollout, config).is_err());
        let mut invalid_rollout = transitions;
        invalid_rollout[2].boundary = PpoBoundary::Continuing;
        assert!(PpoBatch::from_rollout(7, invalid_rollout, config).is_err());
    }
}
