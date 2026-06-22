//! v0 runtime milestone: P26 fixed-point Oja plasticity diagnostics.
//!
//! This module implements pass 3 as a diagnostic/parity path only. It consumes
//! finalized pass-2 activations plus the previous activation buffer and writes a
//! new `H_shadow` trace buffer. Genetic, lifetime, and operational trace layers
//! are carried for contract checks but are not writable by this pass. P27
//! supplies the shared supertile mask early-exit contract.

use std::{sync::mpsc, time::Instant};

use alife_core::{OjaUpdateConfig, ScaffoldContractError};

use crate::routing_masks::p27_tile_is_active;
use crate::{
    GpuBufferContractHeader, GpuFixedPointPolicy, GpuPackedSynapseIndexRecord,
    GpuSupertileMaskRecord, GpuTileMetadataRecord, GpuUploadBuffers,
};

pub const P26_PLASTICITY_WORKGROUP_SIZE: u32 = 64;
pub const P26_PLASTICITY_DIAGNOSTIC_WORDS: u32 = 8;
pub const P26_PLASTICITY_TOLERANCE_Q: i32 = 2;
pub const P26_WGSL_PLASTICITY: &str = include_str!("../shaders/p26_plasticity.wgsl");

const Q16_DENOMINATOR: i64 = u16::MAX as i64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuPlasticityDispatch {
    pub workgroup_size: u32,
    pub pass3_workgroups: u32,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct GpuPlasticityDiagnostics {
    pub overflow_flags: u32,
    pub overflow_count: u32,
    pub saturation_count: u32,
    pub alpha_zero_skips: u32,
    pub active_tiles: u32,
    pub active_synapses: u32,
    pub mask_skipped_tiles: u32,
    pub unsupported_tiles: u32,
}

impl GpuPlasticityDiagnostics {
    fn from_words(words: &[u32]) -> Result<Self, ScaffoldContractError> {
        if words.len() != P26_PLASTICITY_DIAGNOSTIC_WORDS as usize {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        Ok(Self {
            overflow_flags: words[0],
            overflow_count: words[1],
            saturation_count: words[2],
            alpha_zero_skips: words[3],
            active_tiles: words[4],
            active_synapses: words[5],
            mask_skipped_tiles: words[6],
            unsupported_tiles: words[7],
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuOjaFixedPointConfig {
    pub learning_rate_q16: u32,
    pub decay_q16: u32,
    pub shadow_min_q: i16,
    pub shadow_max_q: i16,
    pub stochastic_seed: u32,
}

impl GpuOjaFixedPointConfig {
    pub fn from_oja_config(
        config: OjaUpdateConfig,
        policy: GpuFixedPointPolicy,
        stochastic_seed: u32,
    ) -> Result<Self, ScaffoldContractError> {
        policy.validate()?;
        config.validate_for_gpu()?;
        let learning_rate = config.learning_rate * config.learning_rate_scale;
        let learning_rate_q16 = quantize_nonnegative_q16(learning_rate)?;
        let decay_q16 = quantize_nonnegative_q16(config.decay)?;
        let shadow_min_q = policy.quantize_weight(config.shadow_min)?;
        let shadow_max_q = policy.quantize_weight(config.shadow_max)?;
        if shadow_min_q > shadow_max_q {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(Self {
            learning_rate_q16,
            decay_q16,
            shadow_min_q,
            shadow_max_q,
            stochastic_seed,
        })
    }

    pub fn to_oja_config(self, policy: GpuFixedPointPolicy) -> OjaUpdateConfig {
        OjaUpdateConfig {
            learning_rate: self.learning_rate_q16 as f32 / u16::MAX as f32,
            learning_rate_scale: 1.0,
            decay: self.decay_q16 as f32 / u16::MAX as f32,
            shadow_min: f32::from(self.shadow_min_q) / policy.weight_scale as f32,
            shadow_max: f32::from(self.shadow_max_q) / policy.weight_scale as f32,
        }
    }

    pub fn stochastic_round_div_signed(
        numerator: i64,
        denominator: i64,
        seed: u32,
    ) -> Result<i32, ScaffoldContractError> {
        stochastic_round_div_signed(numerator, denominator, seed)
    }
}

trait ValidateOjaForGpu {
    fn validate_for_gpu(self) -> Result<(), ScaffoldContractError>;
}

impl ValidateOjaForGpu for OjaUpdateConfig {
    fn validate_for_gpu(self) -> Result<(), ScaffoldContractError> {
        alife_core::validate_finite(self.learning_rate)?;
        alife_core::validate_finite(self.learning_rate_scale)?;
        alife_core::validate_finite(self.decay)?;
        alife_core::validate_finite(self.shadow_min)?;
        alife_core::validate_finite(self.shadow_max)?;
        if self.learning_rate < 0.0
            || self.learning_rate_scale < 0.0
            || self.decay < 0.0
            || self.shadow_min > self.shadow_max
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuPlasticityResult {
    pub h_shadow_q: Vec<i16>,
    pub genetic_fixed_q: Vec<i16>,
    pub lifetime_consolidated_q: Vec<i16>,
    pub h_operational_q: Vec<i16>,
    pub diagnostics: GpuPlasticityDiagnostics,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuPlasticityTiming {
    pub submit_poll_wall_ms: f32,
    pub readback_wall_ms: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuPlasticityTimedResult {
    pub result: GpuPlasticityResult,
    pub timing: GpuPlasticityTiming,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuPlasticityPlan {
    pub header: GpuBufferContractHeader,
    pub tile_metadata: Vec<GpuTileMetadataRecord>,
    pub supertile_masks: Vec<GpuSupertileMaskRecord>,
    pub packed_indices: Vec<GpuPackedSynapseIndexRecord>,
    pub genetic_fixed_q: Vec<i16>,
    pub lifetime_consolidated_q: Vec<i16>,
    pub alpha_q16: Vec<u16>,
    pub h_operational_q: Vec<i16>,
    pub h_shadow_initial_q: Vec<i16>,
    pub policy: GpuFixedPointPolicy,
    pub oja: GpuOjaFixedPointConfig,
    pub dispatch: GpuPlasticityDispatch,
}

impl GpuPlasticityPlan {
    pub fn from_upload(
        upload: &GpuUploadBuffers,
        policy: GpuFixedPointPolicy,
        oja: GpuOjaFixedPointConfig,
    ) -> Result<Self, ScaffoldContractError> {
        policy.validate()?;
        validate_oja_against_policy(oja, policy)?;
        let synapse_count = upload.packed_indices.len();
        if upload.genetic_fixed_q.len() != synapse_count
            || upload.lifetime_consolidated_q.len() != synapse_count
            || upload.alpha_q16.len() != synapse_count
            || upload.h_operational_q.len() != synapse_count
            || upload.h_shadow_q.len() != synapse_count
        {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        for tile in &upload.tile_metadata {
            if !matches!(tile.tile_type, 1 | 2) {
                return Err(ScaffoldContractError::UnsupportedSparseTileFormat);
            }
        }

        Ok(Self {
            header: upload.header,
            tile_metadata: upload.tile_metadata.clone(),
            supertile_masks: upload.supertile_masks.clone(),
            packed_indices: upload.packed_indices.clone(),
            genetic_fixed_q: upload.genetic_fixed_q.clone(),
            lifetime_consolidated_q: upload.lifetime_consolidated_q.clone(),
            alpha_q16: upload.alpha_q16.clone(),
            h_operational_q: upload.h_operational_q.clone(),
            h_shadow_initial_q: upload.h_shadow_q.clone(),
            policy,
            oja,
            dispatch: GpuPlasticityDispatch {
                workgroup_size: P26_PLASTICITY_WORKGROUP_SIZE,
                pass3_workgroups: div_ceil_u32(
                    checked_u32(synapse_count)?,
                    P26_PLASTICITY_WORKGROUP_SIZE,
                ),
            },
        })
    }

    pub fn quantize_activations(
        &self,
        activations: &[f32],
    ) -> Result<Vec<i32>, ScaffoldContractError> {
        if activations.len() != self.header.neuron_count as usize {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        activations
            .iter()
            .copied()
            .map(|value| quantize_activation(value, self.policy))
            .collect()
    }

    pub fn execute_cpu_diagnostic(
        &self,
        previous_activation_q: &[i32],
        finalized_activation_q: &[i32],
    ) -> Result<GpuPlasticityResult, ScaffoldContractError> {
        self.validate_activation_shapes(previous_activation_q, finalized_activation_q)?;

        let mut diagnostics = GpuPlasticityDiagnostics::default();
        let mut h_shadow_q = self.h_shadow_initial_q.clone();
        for (tile_index, tile) in self.tile_metadata.iter().enumerate() {
            if !matches!(tile.tile_type, 1 | 2) {
                return Err(ScaffoldContractError::UnsupportedSparseTileFormat);
            }
            if !p27_tile_is_active(*tile, &self.supertile_masks)? {
                diagnostics.mask_skipped_tiles = diagnostics.mask_skipped_tiles.saturating_add(1);
                continue;
            }
            diagnostics.active_tiles = diagnostics.active_tiles.saturating_add(1);
            for synapse in self
                .packed_indices
                .iter()
                .filter(|record| record.tile_metadata_index as usize == tile_index)
            {
                diagnostics.active_synapses = diagnostics.active_synapses.saturating_add(1);
                let weight_index = synapse.weight_index as usize;
                let alpha = *self
                    .alpha_q16
                    .get(weight_index)
                    .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;
                if alpha == 0 {
                    diagnostics.alpha_zero_skips = diagnostics.alpha_zero_skips.saturating_add(1);
                    continue;
                }
                let pre_q = *previous_activation_q
                    .get(synapse.source_index as usize)
                    .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;
                let post_q = *finalized_activation_q
                    .get(synapse.target_index as usize)
                    .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;
                let current_q = *h_shadow_q
                    .get(weight_index)
                    .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;
                let seed =
                    self.oja.stochastic_seed ^ synapse.weight_index.wrapping_mul(0x9E37_79B9);
                let delta_q =
                    compute_oja_delta_q(pre_q, post_q, current_q, self.policy, self.oja, seed)?;
                let unclamped = i32::from(current_q)
                    .checked_add(delta_q)
                    .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
                if unclamped < i32::from(self.oja.shadow_min_q)
                    || unclamped > i32::from(self.oja.shadow_max_q)
                {
                    diagnostics.saturation_count = diagnostics.saturation_count.saturating_add(1);
                }
                if unclamped < i32::from(i16::MIN) || unclamped > i32::from(i16::MAX) {
                    diagnostics.overflow_flags |= 1;
                    diagnostics.overflow_count = diagnostics.overflow_count.saturating_add(1);
                }
                h_shadow_q[weight_index] = unclamped
                    .clamp(
                        i32::from(self.oja.shadow_min_q),
                        i32::from(self.oja.shadow_max_q),
                    )
                    .clamp(i32::from(i16::MIN), i32::from(i16::MAX))
                    as i16;
            }
        }

        Ok(GpuPlasticityResult {
            h_shadow_q,
            genetic_fixed_q: self.genetic_fixed_q.clone(),
            lifetime_consolidated_q: self.lifetime_consolidated_q.clone(),
            h_operational_q: self.h_operational_q.clone(),
            diagnostics,
        })
    }

    fn validate_activation_shapes(
        &self,
        previous_activation_q: &[i32],
        finalized_activation_q: &[i32],
    ) -> Result<(), ScaffoldContractError> {
        let expected = self.header.neuron_count as usize;
        if previous_activation_q.len() != expected || finalized_activation_q.len() != expected {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        Ok(())
    }

    fn params_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(48);
        push_u32(&mut bytes, self.header.neuron_count);
        push_u32(&mut bytes, self.packed_indices.len() as u32);
        push_u32(&mut bytes, self.tile_metadata.len() as u32);
        push_u32(&mut bytes, self.supertile_masks.len() as u32);
        push_i32(&mut bytes, self.policy.activation_scale);
        push_i32(&mut bytes, self.policy.weight_scale);
        push_u32(&mut bytes, self.oja.learning_rate_q16);
        push_u32(&mut bytes, self.oja.decay_q16);
        push_i32(&mut bytes, i32::from(self.oja.shadow_min_q));
        push_i32(&mut bytes, i32::from(self.oja.shadow_max_q));
        push_u32(&mut bytes, self.oja.stochastic_seed);
        push_u32(&mut bytes, 0);
        bytes
    }
}

pub async fn run_plasticity_gpu_diagnostic(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    plan: &GpuPlasticityPlan,
    previous_activation_q: &[i32],
    finalized_activation_q: &[i32],
) -> Result<GpuPlasticityResult, ScaffoldContractError> {
    Ok(run_plasticity_gpu_diagnostic_timed(
        device,
        queue,
        plan,
        previous_activation_q,
        finalized_activation_q,
    )
    .await?
    .result)
}

pub async fn run_plasticity_gpu_diagnostic_timed(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    plan: &GpuPlasticityPlan,
    previous_activation_q: &[i32],
    finalized_activation_q: &[i32],
) -> Result<GpuPlasticityTimedResult, ScaffoldContractError> {
    plan.validate_activation_shapes(previous_activation_q, finalized_activation_q)?;

    let buffers = GpuPlasticityDeviceBuffers::new(
        device,
        plan,
        previous_activation_q,
        finalized_activation_q,
    )?;
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("p26-plasticity"),
        source: wgpu::ShaderSource::Wgsl(P26_WGSL_PLASTICITY.into()),
    });
    let bind_group_layout = create_bind_group_layout(device);
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("p26-plasticity-layout"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });
    let pipeline = create_pipeline(device, &pipeline_layout, &shader, "plasticity_update");
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("p26-plasticity-bind-group"),
        layout: &bind_group_layout,
        entries: &buffers.bind_group_entries(),
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("p26-plasticity-encoder"),
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("p26-plasticity-update"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(plan.dispatch.pass3_workgroups, 1, 1);
    }
    let submit_poll_start = Instant::now();
    queue.submit(Some(encoder.finish()));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|_| ScaffoldContractError::BackendParity)?;
    let submit_poll_wall_ms = elapsed_ms(submit_poll_start);

    let readback_start = Instant::now();
    let h_shadow_q = read_i32_buffer(device, queue, &buffers.h_shadow_write_buffer)?
        .into_iter()
        .map(|value| i16::try_from(value).map_err(|_| ScaffoldContractError::BackendParity))
        .collect::<Result<Vec<_>, _>>()?;
    let diagnostics_words = read_u32_buffer(device, queue, &buffers.diagnostics_buffer)?;
    let readback_wall_ms = elapsed_ms(readback_start);
    Ok(GpuPlasticityTimedResult {
        result: GpuPlasticityResult {
            h_shadow_q,
            genetic_fixed_q: plan.genetic_fixed_q.clone(),
            lifetime_consolidated_q: plan.lifetime_consolidated_q.clone(),
            h_operational_q: plan.h_operational_q.clone(),
            diagnostics: GpuPlasticityDiagnostics::from_words(&diagnostics_words)?,
        },
        timing: GpuPlasticityTiming {
            submit_poll_wall_ms,
            readback_wall_ms,
        },
    })
}

struct GpuPlasticityDeviceBuffers {
    params_buffer: wgpu::Buffer,
    tile_metadata_buffer: wgpu::Buffer,
    supertile_mask_buffer: wgpu::Buffer,
    packed_index_buffer: wgpu::Buffer,
    alpha_buffer: wgpu::Buffer,
    previous_activation_buffer: wgpu::Buffer,
    finalized_activation_buffer: wgpu::Buffer,
    h_shadow_read_buffer: wgpu::Buffer,
    h_shadow_write_buffer: wgpu::Buffer,
    diagnostics_buffer: wgpu::Buffer,
}

impl GpuPlasticityDeviceBuffers {
    fn new(
        device: &wgpu::Device,
        plan: &GpuPlasticityPlan,
        previous_activation_q: &[i32],
        finalized_activation_q: &[i32],
    ) -> Result<Self, ScaffoldContractError> {
        let diagnostics_bytes = u64::from(P26_PLASTICITY_DIAGNOSTIC_WORDS) * 4;
        Ok(Self {
            params_buffer: create_init_buffer(
                device,
                "p26-params",
                &plan.params_bytes(),
                wgpu::BufferUsages::STORAGE,
            ),
            tile_metadata_buffer: create_init_buffer(
                device,
                "p26-tile-metadata",
                &tile_metadata_bytes(&plan.tile_metadata),
                wgpu::BufferUsages::STORAGE,
            ),
            supertile_mask_buffer: create_init_buffer(
                device,
                "p26-supertile-masks",
                &supertile_mask_bytes(&plan.supertile_masks),
                wgpu::BufferUsages::STORAGE,
            ),
            packed_index_buffer: create_init_buffer(
                device,
                "p26-packed-indices",
                &packed_index_bytes(&plan.packed_indices),
                wgpu::BufferUsages::STORAGE,
            ),
            alpha_buffer: create_init_buffer(
                device,
                "p26-alpha",
                &u32_bytes(
                    &plan
                        .alpha_q16
                        .iter()
                        .map(|value| u32::from(*value))
                        .collect::<Vec<_>>(),
                ),
                wgpu::BufferUsages::STORAGE,
            ),
            previous_activation_buffer: create_init_buffer(
                device,
                "p26-previous-activation",
                &i32_bytes(previous_activation_q),
                wgpu::BufferUsages::STORAGE,
            ),
            finalized_activation_buffer: create_init_buffer(
                device,
                "p26-finalized-activation",
                &i32_bytes(finalized_activation_q),
                wgpu::BufferUsages::STORAGE,
            ),
            h_shadow_read_buffer: create_init_buffer(
                device,
                "p26-h-shadow-read",
                &i32_bytes(
                    &plan
                        .h_shadow_initial_q
                        .iter()
                        .map(|value| i32::from(*value))
                        .collect::<Vec<_>>(),
                ),
                wgpu::BufferUsages::STORAGE,
            ),
            h_shadow_write_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("p26-h-shadow-write"),
                size: (plan.h_shadow_initial_q.len() as u64).saturating_mul(4),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            diagnostics_buffer: create_init_buffer(
                device,
                "p26-diagnostics",
                &vec![0_u8; diagnostics_bytes as usize],
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            ),
        })
    }

    fn bind_group_entries(&self) -> [wgpu::BindGroupEntry<'_>; 10] {
        [
            wgpu::BindGroupEntry {
                binding: 0,
                resource: self.params_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: self.tile_metadata_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: self.supertile_mask_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: self.packed_index_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: self.alpha_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: self.previous_activation_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: self.finalized_activation_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: self.h_shadow_read_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 8,
                resource: self.h_shadow_write_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 9,
                resource: self.diagnostics_buffer.as_entire_binding(),
            },
        ]
    }
}

fn compute_oja_delta_q(
    pre_q: i32,
    post_q: i32,
    current_q: i16,
    policy: GpuFixedPointPolicy,
    oja: GpuOjaFixedPointConfig,
    seed: u32,
) -> Result<i32, ScaffoldContractError> {
    let activation_scale = i64::from(policy.activation_scale);
    let weight_scale = i64::from(policy.weight_scale);
    let pre_post_activation_q = stochastic_round_div_signed(
        i64::from(pre_q) * i64::from(post_q),
        activation_scale,
        seed ^ 0x1001,
    )?;
    let pre_post_weight_q = stochastic_round_div_signed(
        i64::from(pre_post_activation_q) * weight_scale,
        activation_scale,
        seed ^ 0x1002,
    )?;
    let post_sq_activation_q = stochastic_round_div_signed(
        i64::from(post_q) * i64::from(post_q),
        activation_scale,
        seed ^ 0x1003,
    )?;
    let post_sq_current_q = stochastic_round_div_signed(
        i64::from(post_sq_activation_q) * i64::from(current_q),
        activation_scale,
        seed ^ 0x1004,
    )?;
    let decayed_current_q = stochastic_round_div_signed(
        i64::from(post_sq_current_q) * i64::from(oja.decay_q16),
        Q16_DENOMINATOR,
        seed ^ 0x1005,
    )?;
    let signal_q = pre_post_weight_q
        .checked_sub(decayed_current_q)
        .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
    stochastic_round_div_signed(
        i64::from(signal_q) * i64::from(oja.learning_rate_q16),
        Q16_DENOMINATOR,
        seed ^ 0x1006,
    )
}

fn stochastic_round_div_signed(
    numerator: i64,
    denominator: i64,
    seed: u32,
) -> Result<i32, ScaffoldContractError> {
    if denominator <= 0 {
        return Err(ScaffoldContractError::ScalarOutOfRange);
    }
    let sign = if numerator < 0 { -1_i64 } else { 1_i64 };
    let magnitude = numerator.unsigned_abs();
    let denominator = denominator as u64;
    let base = magnitude / denominator;
    let remainder = magnitude % denominator;
    let threshold = u64::from(lfsr32(seed)) % denominator;
    let rounded = base + u64::from(threshold < remainder);
    i32::try_from(sign * rounded as i64).map_err(|_| ScaffoldContractError::ScalarOutOfRange)
}

fn lfsr32(seed: u32) -> u32 {
    let mut value = if seed == 0 { 0xA341_316C } else { seed };
    value ^= value << 13;
    value ^= value >> 17;
    value ^= value << 5;
    value
}

fn quantize_nonnegative_q16(value: f32) -> Result<u32, ScaffoldContractError> {
    alife_core::validate_finite(value)?;
    if value < 0.0 {
        return Err(ScaffoldContractError::ScalarOutOfRange);
    }
    let scaled = (value * u16::MAX as f32).round();
    if scaled > u32::MAX as f32 {
        return Err(ScaffoldContractError::ScalarOutOfRange);
    }
    Ok(scaled as u32)
}

fn quantize_activation(
    value: f32,
    policy: GpuFixedPointPolicy,
) -> Result<i32, ScaffoldContractError> {
    alife_core::validate_finite(value)?;
    let min = policy.activation_clamp_min_q as f32 / policy.activation_scale as f32;
    let max = policy.activation_clamp_max_q as f32 / policy.activation_scale as f32;
    if value < min || value > max {
        return Err(ScaffoldContractError::ScalarOutOfRange);
    }
    Ok((value * policy.activation_scale as f32).round() as i32)
}

fn validate_oja_against_policy(
    oja: GpuOjaFixedPointConfig,
    policy: GpuFixedPointPolicy,
) -> Result<(), ScaffoldContractError> {
    policy.validate()?;
    if oja.shadow_min_q > oja.shadow_max_q {
        return Err(ScaffoldContractError::ScalarOutOfRange);
    }
    Ok(())
}

fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let read_only = |binding| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    };
    let read_write = |binding| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    };

    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("p26-plasticity-bind-group-layout"),
        entries: &[
            read_only(0),
            read_only(1),
            read_only(2),
            read_only(3),
            read_only(4),
            read_only(5),
            read_only(6),
            read_only(7),
            read_write(8),
            read_write(9),
        ],
    })
}

fn create_pipeline(
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

fn create_init_buffer(
    device: &wgpu::Device,
    label: &'static str,
    contents: &[u8],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    use wgpu::util::DeviceExt;

    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: nonempty_buffer_contents(contents),
        usage,
    })
}

fn nonempty_buffer_contents(contents: &[u8]) -> &[u8] {
    if contents.is_empty() {
        &[0, 0, 0, 0]
    } else {
        contents
    }
}

fn read_i32_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
) -> Result<Vec<i32>, ScaffoldContractError> {
    let bytes = read_buffer_bytes(device, queue, buffer)?;
    if bytes.len() % 4 != 0 {
        return Err(ScaffoldContractError::BackendParity);
    }
    Ok(bytes
        .chunks_exact(4)
        .map(|chunk| i32::from_le_bytes(chunk.try_into().unwrap()))
        .collect())
}

fn read_u32_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
) -> Result<Vec<u32>, ScaffoldContractError> {
    let bytes = read_buffer_bytes(device, queue, buffer)?;
    if bytes.len() % 4 != 0 {
        return Err(ScaffoldContractError::BackendParity);
    }
    Ok(bytes
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()))
        .collect())
}

fn read_buffer_bytes(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
) -> Result<Vec<u8>, ScaffoldContractError> {
    let size = buffer.size();
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("p26-readback"),
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("p26-readback-encoder"),
    });
    encoder.copy_buffer_to_buffer(buffer, 0, &readback, 0, size);
    queue.submit(Some(encoder.finish()));

    let (sender, receiver) = mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|_| ScaffoldContractError::BackendParity)?;
    receiver
        .recv()
        .map_err(|_| ScaffoldContractError::BackendParity)?
        .map_err(|_| ScaffoldContractError::BackendParity)?;

    let mapped = readback.slice(..).get_mapped_range();
    let bytes = mapped.to_vec();
    drop(mapped);
    readback.unmap();
    Ok(bytes)
}

fn elapsed_ms(start: Instant) -> f32 {
    start.elapsed().as_secs_f64().mul_add(1000.0, 0.0) as f32
}

fn div_ceil_u32(value: u32, divisor: u32) -> u32 {
    value.div_ceil(divisor)
}

fn checked_u32(value: usize) -> Result<u32, ScaffoldContractError> {
    u32::try_from(value).map_err(|_| ScaffoldContractError::InvalidSparseProjectionSchema)
}

fn tile_metadata_bytes(records: &[GpuTileMetadataRecord]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(records.len() * 32);
    for record in records {
        bytes.extend_from_slice(&record.to_le_bytes());
    }
    bytes
}

fn supertile_mask_bytes(records: &[GpuSupertileMaskRecord]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(records.len() * 24);
    for record in records {
        bytes.extend_from_slice(&record.to_le_bytes());
    }
    bytes
}

fn packed_index_bytes(records: &[GpuPackedSynapseIndexRecord]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(records.len() * 16);
    for record in records {
        bytes.extend_from_slice(&record.to_le_bytes());
    }
    bytes
}

fn i32_bytes(values: &[i32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * 4);
    for value in values {
        push_i32(&mut bytes, *value);
    }
    bytes
}

fn u32_bytes(values: &[u32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(values.len() * 4);
    for value in values {
        push_u32(&mut bytes, *value);
    }
    bytes
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_i32(bytes: &mut Vec<u8>, value: i32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}
