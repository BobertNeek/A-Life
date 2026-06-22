//! v0 runtime milestone: P25 static GPU forward-pass parity support.
//!
//! This module implements only passes 0-2 for deterministic static forward
//! projection diagnostics: accumulator clear, fixed-point SpMV, and activation
//! clamp/finalize. P27 supplies the shared supertile mask early-exit contract.
//! This module does not implement plasticity, structural editing, or
//! active-tick readback APIs.

use std::{sync::mpsc, time::Instant};

use alife_core::{validate_finite, ScaffoldContractError};

use crate::routing_masks::{p27_routing_counters, p27_tile_is_active, GpuRoutingCounters};
use crate::{
    GpuBufferContractHeader, GpuFixedPointPolicy, GpuPackedSynapseIndexRecord,
    GpuSupertileMaskRecord, GpuTileMetadataRecord, GpuUploadBuffers,
};

pub const P25_STATIC_FORWARD_WORKGROUP_SIZE: u32 = 64;
pub const P25_DIAGNOSTIC_COUNTER_WORDS: u32 = 8;
pub const P25_STATIC_FORWARD_TOLERANCE_ABS: f32 = 1.0 / 4096.0;
pub const P25_WGSL_STATIC_FORWARD: &str = include_str!("../shaders/p25_static_forward.wgsl");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuStaticForwardDispatch {
    pub workgroup_size: u32,
    pub pass0_workgroups: u32,
    pub pass1_workgroups: u32,
    pub pass2_workgroups: u32,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct GpuStaticForwardDiagnostics {
    pub overflow_flags: u32,
    pub overflow_count: u32,
    pub range_rejections: u32,
    pub nan_rejections: u32,
    pub active_tiles: u32,
    pub active_synapses: u32,
    pub mask_skipped_tiles: u32,
    pub unsupported_tiles: u32,
}

impl GpuStaticForwardDiagnostics {
    fn from_words(words: &[u32]) -> Result<Self, ScaffoldContractError> {
        if words.len() != P25_DIAGNOSTIC_COUNTER_WORDS as usize {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        Ok(Self {
            overflow_flags: words[0],
            overflow_count: words[1],
            range_rejections: words[2],
            nan_rejections: words[3],
            active_tiles: words[4],
            active_synapses: words[5],
            mask_skipped_tiles: words[6],
            unsupported_tiles: words[7],
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuStaticForwardResult {
    pub activations_q: Vec<i32>,
    pub accumulators_q: Vec<i32>,
    pub diagnostics: GpuStaticForwardDiagnostics,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuStaticForwardTiming {
    pub submit_poll_wall_ms: f32,
    pub readback_wall_ms: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuStaticForwardTimedResult {
    pub result: GpuStaticForwardResult,
    pub timing: GpuStaticForwardTiming,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuStaticForwardPlan {
    pub header: GpuBufferContractHeader,
    pub tile_metadata: Vec<GpuTileMetadataRecord>,
    pub supertile_masks: Vec<GpuSupertileMaskRecord>,
    pub packed_indices: Vec<GpuPackedSynapseIndexRecord>,
    pub effective_weight_q: Vec<i32>,
    pub policy: GpuFixedPointPolicy,
    pub dispatch: GpuStaticForwardDispatch,
}

impl GpuStaticForwardPlan {
    pub fn from_upload(
        upload: &GpuUploadBuffers,
        policy: GpuFixedPointPolicy,
    ) -> Result<Self, ScaffoldContractError> {
        policy.validate()?;
        let synapse_count = upload.packed_indices.len();
        if upload.genetic_fixed_q.len() != synapse_count
            || upload.lifetime_consolidated_q.len() != synapse_count
            || upload.alpha_q16.len() != synapse_count
            || upload.h_operational_q.len() != synapse_count
        {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        for tile in &upload.tile_metadata {
            if !matches!(tile.tile_type, 1 | 2) {
                return Err(ScaffoldContractError::UnsupportedSparseTileFormat);
            }
        }

        let mut effective_weight_q = Vec::with_capacity(synapse_count);
        for index in 0..synapse_count {
            let genetic = i32::from(upload.genetic_fixed_q[index]);
            let lifetime = i32::from(upload.lifetime_consolidated_q[index]);
            let h = i64::from(upload.h_operational_q[index]);
            let alpha = i64::from(upload.alpha_q16[index]);
            let alpha_h = round_div_i64(alpha * h, i64::from(u16::MAX))?;
            let effective = genetic
                .checked_add(lifetime)
                .and_then(|sum| sum.checked_add(alpha_h))
                .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
            effective_weight_q.push(effective);
        }

        let clear_words = upload
            .header
            .neuron_count
            .checked_add(P25_DIAGNOSTIC_COUNTER_WORDS)
            .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;
        let dispatch = GpuStaticForwardDispatch {
            workgroup_size: P25_STATIC_FORWARD_WORKGROUP_SIZE,
            pass0_workgroups: div_ceil_u32(clear_words, P25_STATIC_FORWARD_WORKGROUP_SIZE),
            pass1_workgroups: div_ceil_u32(
                checked_u32(synapse_count)?,
                P25_STATIC_FORWARD_WORKGROUP_SIZE,
            ),
            pass2_workgroups: div_ceil_u32(
                upload.header.neuron_count,
                P25_STATIC_FORWARD_WORKGROUP_SIZE,
            ),
        };

        Ok(Self {
            header: upload.header,
            tile_metadata: upload.tile_metadata.clone(),
            supertile_masks: upload.supertile_masks.clone(),
            packed_indices: upload.packed_indices.clone(),
            effective_weight_q,
            policy,
            dispatch,
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

    pub fn dequantize_activations(
        &self,
        activations_q: &[i32],
    ) -> Result<Vec<f32>, ScaffoldContractError> {
        if activations_q.len() != self.header.neuron_count as usize {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        Ok(activations_q
            .iter()
            .map(|value| *value as f32 / self.policy.activation_scale as f32)
            .collect())
    }

    pub fn execute_cpu_diagnostic(
        &self,
        activation_read_q: &[i32],
    ) -> Result<GpuStaticForwardResult, ScaffoldContractError> {
        if activation_read_q.len() != self.header.neuron_count as usize {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        let mut diagnostics = GpuStaticForwardDiagnostics::default();
        let mut accumulators_q = vec![0_i32; self.header.neuron_count as usize];

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
                let source = synapse.source_index as usize;
                let target = synapse.target_index as usize;
                let weight = synapse.weight_index as usize;
                let activation_q = *activation_read_q
                    .get(source)
                    .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;
                let weight_q = *self
                    .effective_weight_q
                    .get(weight)
                    .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;
                let delta_q = round_div_i64(
                    i64::from(activation_q) * i64::from(weight_q),
                    i64::from(self.policy.weight_scale),
                )?;
                let next = accumulators_q
                    .get(target)
                    .copied()
                    .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?
                    .checked_add(delta_q)
                    .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
                if self.policy.accumulator_overflows(next) {
                    diagnostics.overflow_flags |= 1;
                    diagnostics.overflow_count = diagnostics.overflow_count.saturating_add(1);
                }
                accumulators_q[target] = next;
            }
        }

        let mut finalized =
            finalize_static_forward_accumulators_for_diagnostics(&accumulators_q, self.policy)?;
        finalized.diagnostics.overflow_flags |= diagnostics.overflow_flags;
        finalized.diagnostics.overflow_count = finalized
            .diagnostics
            .overflow_count
            .saturating_add(diagnostics.overflow_count);
        finalized.diagnostics.active_tiles = diagnostics.active_tiles;
        finalized.diagnostics.active_synapses = diagnostics.active_synapses;
        finalized.diagnostics.mask_skipped_tiles = diagnostics.mask_skipped_tiles;
        finalized.diagnostics.unsupported_tiles = diagnostics.unsupported_tiles;
        Ok(finalized)
    }

    pub fn routing_counters(&self) -> GpuRoutingCounters {
        p27_routing_counters(
            &self.tile_metadata,
            &self.packed_indices,
            &self.supertile_masks,
            self.header.routing_descriptor_count,
        )
    }

    fn params_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(32);
        push_u32(&mut bytes, self.header.neuron_count);
        push_u32(&mut bytes, self.packed_indices.len() as u32);
        push_u32(&mut bytes, self.tile_metadata.len() as u32);
        push_u32(&mut bytes, self.supertile_masks.len() as u32);
        push_i32(&mut bytes, self.policy.weight_scale);
        push_i32(&mut bytes, self.policy.activation_clamp_min_q);
        push_i32(&mut bytes, self.policy.activation_clamp_max_q);
        push_i32(&mut bytes, self.policy.accumulator_abs_limit_q);
        bytes
    }
}

pub fn finalize_static_forward_accumulators_for_diagnostics(
    accumulators_q: &[i32],
    policy: GpuFixedPointPolicy,
) -> Result<GpuStaticForwardResult, ScaffoldContractError> {
    policy.validate()?;
    let mut diagnostics = GpuStaticForwardDiagnostics::default();
    let mut activations_q = Vec::with_capacity(accumulators_q.len());
    for raw in accumulators_q.iter().copied() {
        if policy.accumulator_overflows(raw) {
            diagnostics.overflow_flags |= 1;
            diagnostics.overflow_count = diagnostics.overflow_count.saturating_add(1);
        }
        let clamped = policy.clamp_activation_q(raw);
        if clamped != raw {
            diagnostics.range_rejections = diagnostics.range_rejections.saturating_add(1);
        }
        activations_q.push(clamped);
    }
    Ok(GpuStaticForwardResult {
        activations_q,
        accumulators_q: accumulators_q.to_vec(),
        diagnostics,
    })
}

pub async fn run_static_forward_gpu_diagnostic(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    plan: &GpuStaticForwardPlan,
    activation_read_q: &[i32],
) -> Result<GpuStaticForwardResult, ScaffoldContractError> {
    Ok(
        run_static_forward_gpu_diagnostic_timed(device, queue, plan, activation_read_q)
            .await?
            .result,
    )
}

pub async fn run_static_forward_gpu_diagnostic_timed(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    plan: &GpuStaticForwardPlan,
    activation_read_q: &[i32],
) -> Result<GpuStaticForwardTimedResult, ScaffoldContractError> {
    if activation_read_q.len() != plan.header.neuron_count as usize {
        return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
    }

    let buffers = GpuStaticForwardDeviceBuffers::new(device, plan, activation_read_q)?;
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("p25-static-forward"),
        source: wgpu::ShaderSource::Wgsl(P25_WGSL_STATIC_FORWARD.into()),
    });
    let bind_group_layout = create_bind_group_layout(device);
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("p25-static-forward-layout"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });
    let pass0 = create_pipeline(device, &pipeline_layout, &shader, "clear_accumulators");
    let pass1 = create_pipeline(device, &pipeline_layout, &shader, "sparse_projection_spmv");
    let pass2 = create_pipeline(device, &pipeline_layout, &shader, "activation_finalize");
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("p25-static-forward-bind-group"),
        layout: &bind_group_layout,
        entries: &buffers.bind_group_entries(),
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("p25-static-forward-encoder"),
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("p25-clear-accumulators"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pass0);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(plan.dispatch.pass0_workgroups, 1, 1);
    }
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("p25-sparse-projection-spmv"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pass1);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(plan.dispatch.pass1_workgroups, 1, 1);
    }
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("p25-activation-finalize"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pass2);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(plan.dispatch.pass2_workgroups, 1, 1);
    }
    let submit_poll_start = Instant::now();
    queue.submit(Some(encoder.finish()));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|_| ScaffoldContractError::BackendParity)?;
    let submit_poll_wall_ms = elapsed_ms(submit_poll_start);

    let readback_start = Instant::now();
    let activations_q = read_i32_buffer(device, queue, &buffers.activation_write_buffer)?;
    let accumulators_q = read_i32_buffer(device, queue, &buffers.accumulator_buffer)?;
    let diagnostics_words = read_u32_buffer(device, queue, &buffers.diagnostics_buffer)?;
    let readback_wall_ms = elapsed_ms(readback_start);
    Ok(GpuStaticForwardTimedResult {
        result: GpuStaticForwardResult {
            activations_q,
            accumulators_q,
            diagnostics: GpuStaticForwardDiagnostics::from_words(&diagnostics_words)?,
        },
        timing: GpuStaticForwardTiming {
            submit_poll_wall_ms,
            readback_wall_ms,
        },
    })
}

struct GpuStaticForwardDeviceBuffers {
    params_buffer: wgpu::Buffer,
    tile_metadata_buffer: wgpu::Buffer,
    supertile_mask_buffer: wgpu::Buffer,
    packed_index_buffer: wgpu::Buffer,
    effective_weight_buffer: wgpu::Buffer,
    activation_read_buffer: wgpu::Buffer,
    accumulator_buffer: wgpu::Buffer,
    activation_write_buffer: wgpu::Buffer,
    diagnostics_buffer: wgpu::Buffer,
}

impl GpuStaticForwardDeviceBuffers {
    fn new(
        device: &wgpu::Device,
        plan: &GpuStaticForwardPlan,
        activation_read_q: &[i32],
    ) -> Result<Self, ScaffoldContractError> {
        let neuron_bytes = u64::from(plan.header.neuron_count) * 4;
        let diagnostics_bytes = u64::from(P25_DIAGNOSTIC_COUNTER_WORDS) * 4;
        Ok(Self {
            params_buffer: create_init_buffer(
                device,
                "p25-params",
                &plan.params_bytes(),
                wgpu::BufferUsages::STORAGE,
            ),
            tile_metadata_buffer: create_init_buffer(
                device,
                "p25-tile-metadata",
                &tile_metadata_bytes(&plan.tile_metadata),
                wgpu::BufferUsages::STORAGE,
            ),
            supertile_mask_buffer: create_init_buffer(
                device,
                "p25-supertile-masks",
                &supertile_mask_bytes(&plan.supertile_masks),
                wgpu::BufferUsages::STORAGE,
            ),
            packed_index_buffer: create_init_buffer(
                device,
                "p25-packed-indices",
                &packed_index_bytes(&plan.packed_indices),
                wgpu::BufferUsages::STORAGE,
            ),
            effective_weight_buffer: create_init_buffer(
                device,
                "p25-effective-weights",
                &i32_bytes(&plan.effective_weight_q),
                wgpu::BufferUsages::STORAGE,
            ),
            activation_read_buffer: create_init_buffer(
                device,
                "p25-activation-read",
                &i32_bytes(activation_read_q),
                wgpu::BufferUsages::STORAGE,
            ),
            accumulator_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("p25-accumulators"),
                size: neuron_bytes,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            activation_write_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("p25-activation-write"),
                size: neuron_bytes,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            diagnostics_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("p25-diagnostics"),
                size: diagnostics_bytes,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
        })
    }

    fn bind_group_entries(&self) -> [wgpu::BindGroupEntry<'_>; 9] {
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
                resource: self.effective_weight_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: self.activation_read_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: self.accumulator_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: self.activation_write_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 8,
                resource: self.diagnostics_buffer.as_entire_binding(),
            },
        ]
    }
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
        label: Some("p25-static-forward-bind-group-layout"),
        entries: &[
            read_only(0),
            read_only(1),
            read_only(2),
            read_only(3),
            read_only(4),
            read_only(5),
            read_write(6),
            read_write(7),
            read_write(8),
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
        label: Some("p25-readback"),
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("p25-readback-encoder"),
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

fn quantize_activation(
    value: f32,
    policy: GpuFixedPointPolicy,
) -> Result<i32, ScaffoldContractError> {
    validate_finite(value)?;
    let min = policy.activation_clamp_min_q as f32 / policy.activation_scale as f32;
    let max = policy.activation_clamp_max_q as f32 / policy.activation_scale as f32;
    if value < min || value > max {
        return Err(ScaffoldContractError::ScalarOutOfRange);
    }
    Ok((value * policy.activation_scale as f32).round() as i32)
}

fn round_div_i64(numerator: i64, denominator: i64) -> Result<i32, ScaffoldContractError> {
    if denominator <= 0 {
        return Err(ScaffoldContractError::ScalarOutOfRange);
    }
    let half = denominator / 2;
    let rounded = if numerator >= 0 {
        (numerator + half) / denominator
    } else {
        (numerator - half) / denominator
    };
    i32::try_from(rounded).map_err(|_| ScaffoldContractError::ScalarOutOfRange)
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

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_i32(bytes: &mut Vec<u8>, value: i32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}
