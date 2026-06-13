//! v0 scaffold: GPU upload buffer contracts translated from the CPU sparse schema.
//!
//! These records are host-side contract structs with explicit little-endian
//! encoders. They intentionally do not use bytemuck/Pod or create wgpu devices.
//! Shader-facing offsets are page-relative byte offsets, never host pointers.

use std::collections::BTreeMap;

use alife_core::{
    require_current_version, validate_finite, ActiveTilePolicy, NeuralProjectionSchema,
    ProjectionType, ScaffoldContractError, SchemaKind, SparseTileType, SupertileMask,
    SynapseWeightSplit, UpdateCadence,
};

pub const GPU_BUFFER_CONTRACT_SCHEMA_VERSION: u16 = 1;
pub const GPU_SERIALIZATION_ENDIANNESS: &str = "little-endian";

pub const GPU_HEADER_BYTES: usize = 48;
pub const GPU_TILE_METADATA_BYTES: usize = 32;
pub const GPU_SUPERTILE_MASK_BYTES: usize = 24;
pub const GPU_PACKED_SYNAPSE_INDEX_BYTES: usize = 16;
pub const GPU_ROUTING_DESCRIPTOR_BYTES: usize = 64;
pub const GPU_DIAGNOSTIC_COUNTER_BYTES: usize = 32;
pub const GPU_ACTION_SUMMARY_RECORD_BYTES: usize = 64;

const I16_BYTES: u64 = 2;
const U16_BYTES: u64 = 2;
const I32_BYTES: u64 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeightBufferFormat {
    I16Fixed,
    U16Normalized,
    I32Fixed,
    I32AtomicAccumulator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuBufferView {
    pub offset_bytes: u64,
    pub len_bytes: u64,
    pub stride_bytes: u32,
    pub format: WeightBufferFormat,
}

impl GpuBufferView {
    pub const fn empty(format: WeightBufferFormat) -> Self {
        Self {
            offset_bytes: 0,
            len_bytes: 0,
            stride_bytes: 0,
            format,
        }
    }

    pub fn is_aligned(self) -> bool {
        if self.stride_bytes == 0 {
            return self.len_bytes == 0;
        }
        let stride = u64::from(self.stride_bytes);
        self.offset_bytes.is_multiple_of(stride) && self.len_bytes.is_multiple_of(stride)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuWeightBufferViews {
    pub genetic_fixed: GpuBufferView,
    pub lifetime_consolidated: GpuBufferView,
    pub alpha: GpuBufferView,
    pub h_operational: GpuBufferView,
    pub h_shadow: GpuBufferView,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuActivationPingPongViews {
    pub activation_read: GpuBufferView,
    pub activation_write: GpuBufferView,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuAccumulatorLayout {
    pub accumulators: GpuBufferView,
    pub diagnostics: GpuBufferView,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuBufferContractHeader {
    pub gpu_schema_version: u16,
    pub neural_projection_schema_version: u16,
    pub brain_class_id: u32,
    pub neuron_count: u32,
    pub microtile_edge: u32,
    pub supertile_edge: u32,
    pub projection_count: u32,
    pub tile_count: u32,
    pub synapse_count: u32,
    pub routing_descriptor_count: u32,
    pub flags: u32,
}

impl GpuBufferContractHeader {
    pub fn to_le_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(GPU_HEADER_BYTES);
        push_u16(&mut bytes, self.gpu_schema_version);
        push_u16(&mut bytes, self.neural_projection_schema_version);
        push_u32(&mut bytes, self.brain_class_id);
        push_u32(&mut bytes, self.neuron_count);
        push_u32(&mut bytes, self.microtile_edge);
        push_u32(&mut bytes, self.supertile_edge);
        push_u32(&mut bytes, self.projection_count);
        push_u32(&mut bytes, self.tile_count);
        push_u32(&mut bytes, self.synapse_count);
        push_u32(&mut bytes, self.routing_descriptor_count);
        push_u32(&mut bytes, self.flags);
        push_u32(&mut bytes, 0);
        push_u32(&mut bytes, 0);
        debug_assert_eq!(bytes.len(), GPU_HEADER_BYTES);
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuTileMetadataRecord {
    pub projection_index: u32,
    pub microtile_row: u32,
    pub microtile_col: u32,
    pub tile_type: u32,
    pub nonzero_count: u32,
    pub synapse_offset: u32,
    pub synapse_count: u32,
    pub flags: u32,
}

impl GpuTileMetadataRecord {
    pub fn to_le_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(GPU_TILE_METADATA_BYTES);
        push_u32(&mut bytes, self.projection_index);
        push_u32(&mut bytes, self.microtile_row);
        push_u32(&mut bytes, self.microtile_col);
        push_u32(&mut bytes, self.tile_type);
        push_u32(&mut bytes, self.nonzero_count);
        push_u32(&mut bytes, self.synapse_offset);
        push_u32(&mut bytes, self.synapse_count);
        push_u32(&mut bytes, self.flags);
        debug_assert_eq!(bytes.len(), GPU_TILE_METADATA_BYTES);
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuSupertileMaskRecord {
    pub projection_index: u32,
    pub supertile_row: u32,
    pub supertile_col: u32,
    pub active_microtile_mask_lo: u32,
    pub active_microtile_mask_hi: u32,
    pub flags: u32,
}

impl GpuSupertileMaskRecord {
    pub fn to_le_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(GPU_SUPERTILE_MASK_BYTES);
        push_u32(&mut bytes, self.projection_index);
        push_u32(&mut bytes, self.supertile_row);
        push_u32(&mut bytes, self.supertile_col);
        push_u32(&mut bytes, self.active_microtile_mask_lo);
        push_u32(&mut bytes, self.active_microtile_mask_hi);
        push_u32(&mut bytes, self.flags);
        debug_assert_eq!(bytes.len(), GPU_SUPERTILE_MASK_BYTES);
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuPackedSynapseIndexRecord {
    pub target_index: u32,
    pub source_index: u32,
    pub weight_index: u32,
    pub tile_metadata_index: u32,
}

impl GpuPackedSynapseIndexRecord {
    pub fn to_le_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(GPU_PACKED_SYNAPSE_INDEX_BYTES);
        push_u32(&mut bytes, self.target_index);
        push_u32(&mut bytes, self.source_index);
        push_u32(&mut bytes, self.weight_index);
        push_u32(&mut bytes, self.tile_metadata_index);
        debug_assert_eq!(bytes.len(), GPU_PACKED_SYNAPSE_INDEX_BYTES);
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuRoutingDescriptorRecord {
    pub projection_index: u32,
    pub source_start: u32,
    pub source_len: u32,
    pub target_start: u32,
    pub target_len: u32,
    pub source_lobe_id: u32,
    pub target_lobe_id: u32,
    pub projection_type: u32,
    pub active_tile_policy: u32,
    pub update_cadence: u32,
    pub tile_metadata_offset: u32,
    pub tile_count: u32,
    pub supertile_mask_offset: u32,
    pub supertile_mask_count: u32,
    pub reserved0: u32,
    pub reserved1: u32,
}

impl GpuRoutingDescriptorRecord {
    pub fn to_le_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(GPU_ROUTING_DESCRIPTOR_BYTES);
        push_u32(&mut bytes, self.projection_index);
        push_u32(&mut bytes, self.source_start);
        push_u32(&mut bytes, self.source_len);
        push_u32(&mut bytes, self.target_start);
        push_u32(&mut bytes, self.target_len);
        push_u32(&mut bytes, self.source_lobe_id);
        push_u32(&mut bytes, self.target_lobe_id);
        push_u32(&mut bytes, self.projection_type);
        push_u32(&mut bytes, self.active_tile_policy);
        push_u32(&mut bytes, self.update_cadence);
        push_u32(&mut bytes, self.tile_metadata_offset);
        push_u32(&mut bytes, self.tile_count);
        push_u32(&mut bytes, self.supertile_mask_offset);
        push_u32(&mut bytes, self.supertile_mask_count);
        push_u32(&mut bytes, self.reserved0);
        push_u32(&mut bytes, self.reserved1);
        debug_assert_eq!(bytes.len(), GPU_ROUTING_DESCRIPTOR_BYTES);
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuDiagnosticCountersRecord {
    pub overflow_flags: u32,
    pub overflow_count: u32,
    pub range_rejections: u32,
    pub nan_rejections: u32,
    pub active_tiles: u32,
    pub active_synapses: u32,
    pub mask_skipped_tiles: u32,
    pub unsupported_tiles: u32,
}

impl GpuDiagnosticCountersRecord {
    pub fn to_le_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(GPU_DIAGNOSTIC_COUNTER_BYTES);
        push_u32(&mut bytes, self.overflow_flags);
        push_u32(&mut bytes, self.overflow_count);
        push_u32(&mut bytes, self.range_rejections);
        push_u32(&mut bytes, self.nan_rejections);
        push_u32(&mut bytes, self.active_tiles);
        push_u32(&mut bytes, self.active_synapses);
        push_u32(&mut bytes, self.mask_skipped_tiles);
        push_u32(&mut bytes, self.unsupported_tiles);
        debug_assert_eq!(bytes.len(), GPU_DIAGNOSTIC_COUNTER_BYTES);
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuActionSummaryStagingRecord {
    pub brain_slot: u32,
    pub winning_action_id: u32,
    pub confidence_q16: u32,
    pub drive_source_mask: u32,
    pub motor_payload_ref: u32,
    pub flags: u32,
    pub reserved: [u32; 10],
}

impl GpuActionSummaryStagingRecord {
    pub fn to_le_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(GPU_ACTION_SUMMARY_RECORD_BYTES);
        push_u32(&mut bytes, self.brain_slot);
        push_u32(&mut bytes, self.winning_action_id);
        push_u32(&mut bytes, self.confidence_q16);
        push_u32(&mut bytes, self.drive_source_mask);
        push_u32(&mut bytes, self.motor_payload_ref);
        push_u32(&mut bytes, self.flags);
        for value in self.reserved {
            push_u32(&mut bytes, value);
        }
        debug_assert_eq!(bytes.len(), GPU_ACTION_SUMMARY_RECORD_BYTES);
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuFixedPointPolicy {
    pub weight_scale: i32,
    pub activation_scale: i32,
    pub accumulator_scale: i32,
    pub activation_clamp_min_q: i32,
    pub activation_clamp_max_q: i32,
    pub accumulator_abs_limit_q: i32,
    pub tolerance_abs: f32,
}

impl GpuFixedPointPolicy {
    pub const fn reference() -> Self {
        Self {
            weight_scale: 4096,
            activation_scale: 32767,
            accumulator_scale: 4096,
            activation_clamp_min_q: -32767,
            activation_clamp_max_q: 32767,
            accumulator_abs_limit_q: 1 << 28,
            tolerance_abs: 1.0 / 4096.0,
        }
    }

    pub fn validate(self) -> Result<(), ScaffoldContractError> {
        validate_finite(self.tolerance_abs)?;
        if self.weight_scale <= 0
            || self.activation_scale <= 0
            || self.accumulator_scale <= 0
            || self.activation_clamp_min_q > self.activation_clamp_max_q
            || self.accumulator_abs_limit_q <= 0
            || self.tolerance_abs <= 0.0
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }

    pub fn quantize_weight(self, value: f32) -> Result<i16, ScaffoldContractError> {
        self.validate()?;
        quantize_i16(value, self.weight_scale)
    }

    pub fn quantize_alpha(self, value: f32) -> Result<u16, ScaffoldContractError> {
        self.validate()?;
        validate_finite(value)?;
        if !(0.0..=1.0).contains(&value) {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok((value * f32::from(u16::MAX)).round() as u16)
    }

    pub fn clamp_activation_q(self, value: i32) -> i32 {
        value.clamp(self.activation_clamp_min_q, self.activation_clamp_max_q)
    }

    pub fn accumulator_overflows(self, value: i32) -> bool {
        i64::from(value).abs() > i64::from(self.accumulator_abs_limit_q)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuReadbackClass {
    ActionSummaryStaging,
    DiagnosticExportStaging,
    BulkActivation,
    PerSynapse,
    PerLobeSlice,
    WeightBuffer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuReadbackPolicy {
    pub action_summary_staging_allowed: bool,
    pub diagnostic_export_staging_allowed: bool,
    pub bulk_activation_allowed: bool,
    pub per_synapse_allowed: bool,
    pub per_lobe_slice_allowed: bool,
    pub weight_buffer_allowed: bool,
}

impl GpuReadbackPolicy {
    pub const fn active_gameplay() -> Self {
        Self {
            action_summary_staging_allowed: true,
            diagnostic_export_staging_allowed: true,
            bulk_activation_allowed: false,
            per_synapse_allowed: false,
            per_lobe_slice_allowed: false,
            weight_buffer_allowed: false,
        }
    }

    pub const fn allows(self, class: GpuReadbackClass) -> bool {
        match class {
            GpuReadbackClass::ActionSummaryStaging => self.action_summary_staging_allowed,
            GpuReadbackClass::DiagnosticExportStaging => self.diagnostic_export_staging_allowed,
            GpuReadbackClass::BulkActivation => self.bulk_activation_allowed,
            GpuReadbackClass::PerSynapse => self.per_synapse_allowed,
            GpuReadbackClass::PerLobeSlice => self.per_lobe_slice_allowed,
            GpuReadbackClass::WeightBuffer => self.weight_buffer_allowed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuUploadBuffers {
    pub header: GpuBufferContractHeader,
    pub tile_metadata: Vec<GpuTileMetadataRecord>,
    pub supertile_masks: Vec<GpuSupertileMaskRecord>,
    pub packed_indices: Vec<GpuPackedSynapseIndexRecord>,
    pub routing_descriptors: Vec<GpuRoutingDescriptorRecord>,
    pub genetic_fixed_q: Vec<i16>,
    pub lifetime_consolidated_q: Vec<i16>,
    pub alpha_q16: Vec<u16>,
    pub h_operational_q: Vec<i16>,
    pub h_shadow_q: Vec<i16>,
}

impl GpuUploadBuffers {
    pub fn from_cpu_schema(
        schema: &NeuralProjectionSchema,
        policy: GpuFixedPointPolicy,
    ) -> Result<Self, ScaffoldContractError> {
        require_current_version(SchemaKind::NeuralProjection, schema.schema_version)?;
        schema.validate()?;
        policy.validate()?;

        let mut tile_metadata = Vec::new();
        let mut supertile_masks = Vec::new();
        let mut packed_indices = Vec::new();
        let mut routing_descriptors = Vec::new();
        let mut genetic_fixed_q = Vec::new();
        let mut lifetime_consolidated_q = Vec::new();
        let mut alpha_q16 = Vec::new();
        let mut h_operational_q = Vec::new();
        let mut h_shadow_q = Vec::new();

        for projection in &schema.projections {
            let tile_metadata_offset = checked_u32(tile_metadata.len())?;
            let supertile_mask_offset = checked_u32(supertile_masks.len())?;

            for tile in &projection.tiles {
                let synapse_offset = checked_u32(packed_indices.len())?;
                let tile_metadata_index = checked_u32(tile_metadata.len())?;
                let decoded = tile.decode_synapses()?;
                for synapse in &decoded {
                    let weight_index = checked_u32(genetic_fixed_q.len())?;
                    packed_indices.push(GpuPackedSynapseIndexRecord {
                        target_index: synapse.target,
                        source_index: synapse.source,
                        weight_index,
                        tile_metadata_index,
                    });
                    encode_weights(
                        synapse.weights,
                        policy,
                        &mut genetic_fixed_q,
                        &mut lifetime_consolidated_q,
                        &mut alpha_q16,
                        &mut h_operational_q,
                        &mut h_shadow_q,
                    )?;
                }
                tile_metadata.push(GpuTileMetadataRecord {
                    projection_index: tile.metadata.projection_index,
                    microtile_row: tile.metadata.coord.microtile_row,
                    microtile_col: tile.metadata.coord.microtile_col,
                    tile_type: tile_type_code(tile.metadata.tile_type),
                    nonzero_count: u32::from(tile.metadata.nonzero_count),
                    synapse_offset,
                    synapse_count: checked_u32(decoded.len())?,
                    flags: 0,
                });
            }

            for mask in explicit_masks_for_projection(projection) {
                supertile_masks.push(GpuSupertileMaskRecord {
                    projection_index: projection.projection_index,
                    supertile_row: mask.supertile_row,
                    supertile_col: mask.supertile_col,
                    active_microtile_mask_lo: mask.active_microtile_mask as u32,
                    active_microtile_mask_hi: (mask.active_microtile_mask >> 32) as u32,
                    flags: 0,
                });
            }

            routing_descriptors.push(GpuRoutingDescriptorRecord {
                projection_index: projection.projection_index,
                source_start: projection.source_range.start,
                source_len: projection.source_range.len,
                target_start: projection.target_range.start,
                target_len: projection.target_range.len,
                source_lobe_id: u32::from(projection.routing_ref.source_lobe.stable_id().raw()),
                target_lobe_id: u32::from(projection.routing_ref.target_lobe.stable_id().raw()),
                projection_type: projection_type_code(projection.routing_ref.projection_type),
                active_tile_policy: active_tile_policy_code(projection.active_tile_policy),
                update_cadence: update_cadence_code(projection.update_cadence),
                tile_metadata_offset,
                tile_count: checked_u32(projection.tiles.len())?,
                supertile_mask_offset,
                supertile_mask_count: checked_u32(supertile_masks.len())?
                    .saturating_sub(supertile_mask_offset),
                reserved0: 0,
                reserved1: 0,
            });
        }

        let header = GpuBufferContractHeader {
            gpu_schema_version: GPU_BUFFER_CONTRACT_SCHEMA_VERSION,
            neural_projection_schema_version: schema.schema_version,
            brain_class_id: u32::from(schema.brain_class_id.raw()),
            neuron_count: schema.neuron_count,
            microtile_edge: schema.microtile_edge,
            supertile_edge: schema.supertile_edge,
            projection_count: checked_u32(schema.projections.len())?,
            tile_count: checked_u32(tile_metadata.len())?,
            synapse_count: checked_u32(packed_indices.len())?,
            routing_descriptor_count: checked_u32(routing_descriptors.len())?,
            flags: 0,
        };

        Ok(Self {
            header,
            tile_metadata,
            supertile_masks,
            packed_indices,
            routing_descriptors,
            genetic_fixed_q,
            lifetime_consolidated_q,
            alpha_q16,
            h_operational_q,
            h_shadow_q,
        })
    }

    pub fn weight_views(&self) -> GpuWeightBufferViews {
        let genetic_len = self.genetic_fixed_q.len() as u64 * I16_BYTES;
        let lifetime_len = self.lifetime_consolidated_q.len() as u64 * I16_BYTES;
        let alpha_len = self.alpha_q16.len() as u64 * U16_BYTES;
        let h_operational_len = self.h_operational_q.len() as u64 * I16_BYTES;
        let h_shadow_len = self.h_shadow_q.len() as u64 * I16_BYTES;

        let genetic_fixed = GpuBufferView {
            offset_bytes: 0,
            len_bytes: genetic_len,
            stride_bytes: I16_BYTES as u32,
            format: WeightBufferFormat::I16Fixed,
        };
        let lifetime_consolidated = GpuBufferView {
            offset_bytes: genetic_fixed.offset_bytes + genetic_len,
            len_bytes: lifetime_len,
            stride_bytes: I16_BYTES as u32,
            format: WeightBufferFormat::I16Fixed,
        };
        let alpha = GpuBufferView {
            offset_bytes: lifetime_consolidated.offset_bytes + lifetime_len,
            len_bytes: alpha_len,
            stride_bytes: U16_BYTES as u32,
            format: WeightBufferFormat::U16Normalized,
        };
        let h_operational = GpuBufferView {
            offset_bytes: alpha.offset_bytes + alpha_len,
            len_bytes: h_operational_len,
            stride_bytes: I16_BYTES as u32,
            format: WeightBufferFormat::I16Fixed,
        };
        let h_shadow = GpuBufferView {
            offset_bytes: h_operational.offset_bytes + h_operational_len,
            len_bytes: h_shadow_len,
            stride_bytes: I16_BYTES as u32,
            format: WeightBufferFormat::I16Fixed,
        };

        GpuWeightBufferViews {
            genetic_fixed,
            lifetime_consolidated,
            alpha,
            h_operational,
            h_shadow,
        }
    }

    pub fn activation_ping_pong_views(&self) -> GpuActivationPingPongViews {
        let len_bytes = u64::from(self.header.neuron_count) * I32_BYTES;
        GpuActivationPingPongViews {
            activation_read: GpuBufferView {
                offset_bytes: 0,
                len_bytes,
                stride_bytes: I32_BYTES as u32,
                format: WeightBufferFormat::I32Fixed,
            },
            activation_write: GpuBufferView {
                offset_bytes: len_bytes,
                len_bytes,
                stride_bytes: I32_BYTES as u32,
                format: WeightBufferFormat::I32Fixed,
            },
        }
    }

    pub fn accumulator_layout(&self) -> GpuAccumulatorLayout {
        let accumulator_len = u64::from(self.header.neuron_count) * I32_BYTES;
        GpuAccumulatorLayout {
            accumulators: GpuBufferView {
                offset_bytes: 0,
                len_bytes: accumulator_len,
                stride_bytes: I32_BYTES as u32,
                format: WeightBufferFormat::I32AtomicAccumulator,
            },
            diagnostics: GpuBufferView {
                offset_bytes: accumulator_len,
                len_bytes: GPU_DIAGNOSTIC_COUNTER_BYTES as u64,
                stride_bytes: I32_BYTES as u32,
                format: WeightBufferFormat::I32AtomicAccumulator,
            },
        }
    }

    pub fn encoded_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_le_bytes();
        extend_records(
            &mut bytes,
            &self.tile_metadata,
            GpuTileMetadataRecord::to_le_bytes,
        );
        extend_records(
            &mut bytes,
            &self.supertile_masks,
            GpuSupertileMaskRecord::to_le_bytes,
        );
        extend_records(
            &mut bytes,
            &self.packed_indices,
            GpuPackedSynapseIndexRecord::to_le_bytes,
        );
        extend_records(
            &mut bytes,
            &self.routing_descriptors,
            GpuRoutingDescriptorRecord::to_le_bytes,
        );
        extend_i16s(&mut bytes, &self.genetic_fixed_q);
        extend_i16s(&mut bytes, &self.lifetime_consolidated_q);
        extend_u16s(&mut bytes, &self.alpha_q16);
        extend_i16s(&mut bytes, &self.h_operational_q);
        extend_i16s(&mut bytes, &self.h_shadow_q);
        bytes
    }
}

fn encode_weights(
    weights: SynapseWeightSplit,
    policy: GpuFixedPointPolicy,
    genetic_fixed_q: &mut Vec<i16>,
    lifetime_consolidated_q: &mut Vec<i16>,
    alpha_q16: &mut Vec<u16>,
    h_operational_q: &mut Vec<i16>,
    h_shadow_q: &mut Vec<i16>,
) -> Result<(), ScaffoldContractError> {
    genetic_fixed_q.push(policy.quantize_weight(weights.genetic_fixed)?);
    lifetime_consolidated_q.push(policy.quantize_weight(weights.lifetime_consolidated)?);
    alpha_q16.push(policy.quantize_alpha(weights.alpha)?);
    h_operational_q.push(policy.quantize_weight(weights.h_operational)?);
    h_shadow_q.push(policy.quantize_weight(weights.h_shadow)?);
    Ok(())
}

fn explicit_masks_for_projection(projection: &alife_core::SparseProjection) -> Vec<SupertileMask> {
    if !projection.supertile_masks.is_empty() {
        return projection.supertile_masks.clone();
    }

    let mut masks: BTreeMap<(u32, u32), u64> = BTreeMap::new();
    for tile in &projection.tiles {
        let coord = tile.metadata.coord;
        let entry = masks
            .entry((coord.supertile_row(), coord.supertile_col()))
            .or_insert(0);
        *entry |= 1_u64 << coord.supertile_local_bit();
    }
    masks
        .into_iter()
        .map(
            |((supertile_row, supertile_col), active_microtile_mask)| SupertileMask {
                supertile_row,
                supertile_col,
                active_microtile_mask,
            },
        )
        .collect()
}

fn tile_type_code(tile_type: SparseTileType) -> u32 {
    match tile_type {
        SparseTileType::Dense16x16 => 1,
        SparseTileType::Coo => 2,
        SparseTileType::RowRun => 3,
        SparseTileType::ColumnRun => 4,
    }
}

fn projection_type_code(projection_type: ProjectionType) -> u32 {
    match projection_type {
        ProjectionType::FeedForward => 1,
        ProjectionType::Feedback => 2,
        ProjectionType::Recurrent => 3,
        ProjectionType::Modulatory => 4,
        ProjectionType::MotorProposal => 5,
        ProjectionType::Homeostatic => 6,
        ProjectionType::LateralInhibition => 7,
    }
}

fn active_tile_policy_code(policy: ActiveTilePolicy) -> u32 {
    match policy {
        ActiveTilePolicy::EssentialReservation => 1,
        ActiveTilePolicy::SalienceGated => 2,
        ActiveTilePolicy::Decimated => 3,
        ActiveTilePolicy::SleepQueued => 4,
    }
}

fn update_cadence_code(cadence: UpdateCadence) -> u32 {
    match cadence {
        UpdateCadence::Hot60Hz => 1,
        UpdateCadence::Hot15To60Hz => 2,
        UpdateCadence::Hot10To30Hz => 3,
        UpdateCadence::Hot5To15Hz => 4,
        UpdateCadence::Hot1To5Hz => 5,
        UpdateCadence::SleepOrOffline => 6,
        UpdateCadence::Disabled => 7,
    }
}

fn quantize_i16(value: f32, scale: i32) -> Result<i16, ScaffoldContractError> {
    validate_finite(value)?;
    let scaled = (value * scale as f32).round();
    if scaled < f32::from(i16::MIN) || scaled > f32::from(i16::MAX) {
        return Err(ScaffoldContractError::ScalarOutOfRange);
    }
    Ok(scaled as i16)
}

fn checked_u32(value: usize) -> Result<u32, ScaffoldContractError> {
    u32::try_from(value).map_err(|_| ScaffoldContractError::InvalidSparseProjectionSchema)
}

fn push_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn extend_records<T: Copy>(bytes: &mut Vec<u8>, records: &[T], encode: fn(T) -> Vec<u8>) {
    for record in records {
        bytes.extend_from_slice(&encode(*record));
    }
}

fn extend_i16s(bytes: &mut Vec<u8>, values: &[i16]) {
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
}

fn extend_u16s(bytes: &mut Vec<u8>, values: &[u16]) {
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
}
