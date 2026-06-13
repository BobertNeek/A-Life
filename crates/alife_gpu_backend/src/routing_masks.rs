//! v0 runtime milestone: P27 supertile culling and routing-mask contracts.
//!
//! This module is a CPU-side parity and packing contract for hierarchical
//! culling. It does not implement P28 recompaction or P29 runtime scheduling.

use std::collections::{BTreeMap, BTreeSet};

use alife_core::{BrainClassSpec, ScaffoldContractError};

use crate::buffers::{
    GpuPackedSynapseIndexRecord, GpuRoutingDescriptorRecord, GpuSupertileMaskRecord,
    GpuTileMetadataRecord, GpuUploadBuffers,
};

pub const P27_MICROTILE_EDGE: u32 = 16;
pub const P27_SUPERTILE_MICROTILES: u32 = 8;
pub const P27_SUPERTILE_EDGE: u32 = P27_MICROTILE_EDGE * P27_SUPERTILE_MICROTILES;
pub const P27_SUPERTILE_MASK_WORDS: u32 = 2;
pub const P27_STATIC_FORWARD_STORAGE_BINDINGS: u32 = 9;
pub const P27_PLASTICITY_STORAGE_BINDINGS: u32 = 10;
pub const P27_WGSL_SUPERTILE_ROUTING: &str = include_str!("../shaders/p27_supertile_routing.wgsl");

const PROJECTION_RECURRENT_CODE: u32 = 3;
const POLICY_ESSENTIAL_RESERVATION: u32 = 1;
const POLICY_SALIENCE_GATED: u32 = 2;
const POLICY_DECIMATED: u32 = 3;
const POLICY_SLEEP_QUEUED: u32 = 4;
const CADENCE_HOT_60HZ: u32 = 1;
const CADENCE_HOT_15_TO_60HZ: u32 = 2;
const CADENCE_HOT_10_TO_30HZ: u32 = 3;
const CADENCE_HOT_5_TO_15HZ: u32 = 4;
const CADENCE_HOT_1_TO_5HZ: u32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuSupertileIndex {
    pub supertile_row: u32,
    pub supertile_col: u32,
    pub local_row: u32,
    pub local_col: u32,
    pub local_bit: u32,
    pub mask_word_index: u32,
    pub mask_word_bit: u32,
}

impl GpuSupertileIndex {
    pub fn from_microtile(
        microtile_row: u32,
        microtile_col: u32,
    ) -> Result<Self, ScaffoldContractError> {
        let local_row = microtile_row % P27_SUPERTILE_MICROTILES;
        let local_col = microtile_col % P27_SUPERTILE_MICROTILES;
        let local_bit = local_row
            .checked_mul(P27_SUPERTILE_MICROTILES)
            .and_then(|value| value.checked_add(local_col))
            .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;
        Self::from_parts(
            microtile_row,
            microtile_col,
            local_row,
            local_col,
            local_bit,
        )
    }

    fn from_parts(
        microtile_row: u32,
        microtile_col: u32,
        local_row: u32,
        local_col: u32,
        local_bit: u32,
    ) -> Result<Self, ScaffoldContractError> {
        if local_row >= P27_SUPERTILE_MICROTILES
            || local_col >= P27_SUPERTILE_MICROTILES
            || local_bit >= 64
        {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        Ok(Self {
            supertile_row: microtile_row / P27_SUPERTILE_MICROTILES,
            supertile_col: microtile_col / P27_SUPERTILE_MICROTILES,
            local_row,
            local_col,
            local_bit,
            mask_word_index: local_bit / 32,
            mask_word_bit: local_bit % 32,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuSupertileMaskWords {
    pub projection_index: u32,
    pub supertile_row: u32,
    pub supertile_col: u32,
    pub low_word: u32,
    pub high_word: u32,
}

impl GpuSupertileMaskWords {
    pub const fn empty(projection_index: u32, supertile_row: u32, supertile_col: u32) -> Self {
        Self {
            projection_index,
            supertile_row,
            supertile_col,
            low_word: 0,
            high_word: 0,
        }
    }

    pub const fn from_record(record: GpuSupertileMaskRecord) -> Self {
        Self {
            projection_index: record.projection_index,
            supertile_row: record.supertile_row,
            supertile_col: record.supertile_col,
            low_word: record.active_microtile_mask_lo,
            high_word: record.active_microtile_mask_hi,
        }
    }

    pub const fn to_record(self) -> GpuSupertileMaskRecord {
        GpuSupertileMaskRecord {
            projection_index: self.projection_index,
            supertile_row: self.supertile_row,
            supertile_col: self.supertile_col,
            active_microtile_mask_lo: self.low_word,
            active_microtile_mask_hi: self.high_word,
            flags: 0,
        }
    }

    pub fn insert_local_bit(&mut self, local_bit: u32) -> Result<(), ScaffoldContractError> {
        let (word, bit) = mask_word_and_bit(local_bit)?;
        if word == 0 {
            self.low_word |= 1_u32 << bit;
        } else {
            self.high_word |= 1_u32 << bit;
        }
        Ok(())
    }

    pub fn insert_microtile(
        &mut self,
        microtile_row: u32,
        microtile_col: u32,
    ) -> Result<(), ScaffoldContractError> {
        let index = GpuSupertileIndex::from_microtile(microtile_row, microtile_col)?;
        if index.supertile_row != self.supertile_row || index.supertile_col != self.supertile_col {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        self.insert_local_bit(index.local_bit)
    }

    pub fn contains_local_bit(self, local_bit: u32) -> Result<bool, ScaffoldContractError> {
        let (word, bit) = mask_word_and_bit(local_bit)?;
        if word == 0 {
            Ok((self.low_word & (1_u32 << bit)) != 0)
        } else {
            Ok((self.high_word & (1_u32 << bit)) != 0)
        }
    }

    pub fn contains_microtile(
        self,
        microtile_row: u32,
        microtile_col: u32,
    ) -> Result<bool, ScaffoldContractError> {
        let index = GpuSupertileIndex::from_microtile(microtile_row, microtile_col)?;
        if index.supertile_row != self.supertile_row || index.supertile_col != self.supertile_col {
            return Ok(false);
        }
        self.contains_local_bit(index.local_bit)
    }

    pub const fn is_zero(self) -> bool {
        self.low_word == 0 && self.high_word == 0
    }

    pub fn active_microtile_count(self) -> u32 {
        self.low_word.count_ones() + self.high_word.count_ones()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuActiveTileMaskConfig {
    pub tick_index: u64,
    pub sensory_activity_present: bool,
    pub biological_tile_budget: u32,
    pub force_static_fixture_tiles: bool,
}

impl GpuActiveTileMaskConfig {
    pub const fn for_deterministic_fixture(
        tick_index: u64,
        sensory_activity_present: bool,
    ) -> Self {
        Self {
            tick_index,
            sensory_activity_present,
            biological_tile_budget: u32::MAX,
            force_static_fixture_tiles: true,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct GpuRoutingCounters {
    pub skipped_supertiles: u32,
    pub skipped_microtiles: u32,
    pub active_tiles: u32,
    pub active_synapses: u32,
    pub routing_descriptors_evaluated: u32,
    pub mask_boundary_failures: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuRoutingMaskPlan {
    pub brain_class_id: u32,
    pub active_masks: Vec<GpuSupertileMaskRecord>,
    pub routing_descriptors: Vec<GpuRoutingDescriptorRecord>,
    pub routing_descriptors_evaluated: u32,
    pub active_tiles: u32,
    pub skipped_microtiles: u32,
    pub active_synapses: u32,
}

impl GpuRoutingMaskPlan {
    pub fn from_upload_and_brain_class(
        upload: &GpuUploadBuffers,
        spec: &BrainClassSpec,
        config: GpuActiveTileMaskConfig,
    ) -> Result<Self, ScaffoldContractError> {
        spec.validate()?;
        if upload.header.brain_class_id != u32::from(spec.id.raw())
            || upload.header.neuron_count != spec.neuron_count
        {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        validate_routing_descriptors_against_spec(&upload.routing_descriptors, spec)?;

        let descriptor_by_projection = upload
            .routing_descriptors
            .iter()
            .map(|descriptor| (descriptor.projection_index, *descriptor))
            .collect::<BTreeMap<_, _>>();
        let active_tile_set = select_active_tiles(upload, &descriptor_by_projection, spec, config)?;
        let mut masks = BTreeMap::<(u32, u32, u32), GpuSupertileMaskWords>::new();
        let mut active_synapses = 0_u32;

        for tile in &upload.tile_metadata {
            if active_tile_set.contains(&tile_key(tile)) {
                let index =
                    GpuSupertileIndex::from_microtile(tile.microtile_row, tile.microtile_col)?;
                let mask = masks
                    .entry((
                        tile.projection_index,
                        index.supertile_row,
                        index.supertile_col,
                    ))
                    .or_insert_with(|| {
                        GpuSupertileMaskWords::empty(
                            tile.projection_index,
                            index.supertile_row,
                            index.supertile_col,
                        )
                    });
                mask.insert_local_bit(index.local_bit)?;
                active_synapses = active_synapses.saturating_add(tile.synapse_count);
            }
        }

        let active_masks = masks
            .into_values()
            .map(GpuSupertileMaskWords::to_record)
            .collect();
        let active_tiles = active_tile_set.len() as u32;
        Ok(Self {
            brain_class_id: upload.header.brain_class_id,
            active_masks,
            routing_descriptors: upload.routing_descriptors.clone(),
            routing_descriptors_evaluated: checked_u32(upload.routing_descriptors.len())?,
            active_tiles,
            skipped_microtiles: checked_u32(upload.tile_metadata.len())?
                .saturating_sub(active_tiles),
            active_synapses,
        })
    }
}

pub fn p27_tile_is_active(
    tile: GpuTileMetadataRecord,
    masks: &[GpuSupertileMaskRecord],
) -> Result<bool, ScaffoldContractError> {
    if masks.is_empty() {
        return Ok(true);
    }
    let index = GpuSupertileIndex::from_microtile(tile.microtile_row, tile.microtile_col)?;
    for mask in masks {
        if mask.projection_index != tile.projection_index
            || mask.supertile_row != index.supertile_row
            || mask.supertile_col != index.supertile_col
        {
            continue;
        }
        return GpuSupertileMaskWords::from_record(*mask).contains_local_bit(index.local_bit);
    }
    Ok(false)
}

pub fn p27_routing_counters(
    tiles: &[GpuTileMetadataRecord],
    _packed_indices: &[GpuPackedSynapseIndexRecord],
    masks: &[GpuSupertileMaskRecord],
    routing_descriptor_count: u32,
) -> GpuRoutingCounters {
    let mut counters = GpuRoutingCounters {
        routing_descriptors_evaluated: routing_descriptor_count,
        ..GpuRoutingCounters::default()
    };
    let mut skipped_supertiles = BTreeSet::new();

    for tile in tiles {
        match p27_tile_is_active(*tile, masks) {
            Ok(true) => {
                counters.active_tiles = counters.active_tiles.saturating_add(1);
                counters.active_synapses =
                    counters.active_synapses.saturating_add(tile.synapse_count);
            }
            Ok(false) => {
                counters.skipped_microtiles = counters.skipped_microtiles.saturating_add(1);
                if let Ok(index) =
                    GpuSupertileIndex::from_microtile(tile.microtile_row, tile.microtile_col)
                {
                    skipped_supertiles.insert((
                        tile.projection_index,
                        index.supertile_row,
                        index.supertile_col,
                    ));
                } else {
                    counters.mask_boundary_failures =
                        counters.mask_boundary_failures.saturating_add(1);
                }
            }
            Err(_) => {
                counters.mask_boundary_failures = counters.mask_boundary_failures.saturating_add(1);
            }
        }
    }
    counters.skipped_supertiles = skipped_supertiles.len() as u32;
    counters
}

fn validate_routing_descriptors_against_spec(
    descriptors: &[GpuRoutingDescriptorRecord],
    spec: &BrainClassSpec,
) -> Result<(), ScaffoldContractError> {
    for descriptor in descriptors {
        let source = spec
            .lobe_regions()
            .find(|region| {
                region.enabled
                    && u32::from(region.kind.stable_id().raw()) == descriptor.source_lobe_id
            })
            .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;
        let target = spec
            .lobe_regions()
            .find(|region| {
                region.enabled
                    && u32::from(region.kind.stable_id().raw()) == descriptor.target_lobe_id
            })
            .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;

        let range_matches_lobes = descriptor.source_start == source.start
            && descriptor.source_len == source.len
            && descriptor.target_start == target.start
            && descriptor.target_len == target.len;
        let full_brain_reference = descriptor.source_start == 0
            && descriptor.source_len == spec.neuron_count
            && descriptor.target_start == 0
            && descriptor.target_len == spec.neuron_count
            && descriptor.source_lobe_id == descriptor.target_lobe_id
            && descriptor.projection_type == PROJECTION_RECURRENT_CODE;
        if !range_matches_lobes && !full_brain_reference {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }

        let route_exists = spec.routing_masks().iter().any(|mask| {
            u32::from(mask.source_lobe.stable_id().raw()) == descriptor.source_lobe_id
                && u32::from(mask.target_lobe.stable_id().raw()) == descriptor.target_lobe_id
                && projection_type_code_matches(mask.projection_type, descriptor.projection_type)
        });
        if !route_exists && !full_brain_reference {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
    }
    Ok(())
}

fn select_active_tiles(
    upload: &GpuUploadBuffers,
    descriptor_by_projection: &BTreeMap<u32, GpuRoutingDescriptorRecord>,
    spec: &BrainClassSpec,
    config: GpuActiveTileMaskConfig,
) -> Result<BTreeSet<(u32, u32, u32)>, ScaffoldContractError> {
    let effective_budget = if config.force_static_fixture_tiles {
        u32::MAX
    } else {
        config
            .biological_tile_budget
            .min(spec.compute_budget.max_active_tiles)
    };

    let mut candidates = Vec::new();
    for tile in &upload.tile_metadata {
        let descriptor = descriptor_by_projection
            .get(&tile.projection_index)
            .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;
        if descriptor_allows_tick(*descriptor, config) {
            candidates.push((
                policy_priority(descriptor.active_tile_policy),
                descriptor.projection_index,
                tile.microtile_row,
                tile.microtile_col,
                tile.synapse_count,
            ));
        }
    }
    candidates.sort_by_key(|candidate| (candidate.0, candidate.1, candidate.2, candidate.3));

    let mut selected = BTreeSet::new();
    for (_, projection_index, microtile_row, microtile_col, _) in candidates {
        if checked_u32(selected.len())? >= effective_budget {
            break;
        }
        selected.insert((projection_index, microtile_row, microtile_col));
    }
    Ok(selected)
}

fn descriptor_allows_tick(
    descriptor: GpuRoutingDescriptorRecord,
    config: GpuActiveTileMaskConfig,
) -> bool {
    if config.force_static_fixture_tiles {
        return true;
    }
    if !cadence_due(descriptor.update_cadence, config.tick_index) {
        return false;
    }
    match descriptor.active_tile_policy {
        POLICY_ESSENTIAL_RESERVATION => true,
        POLICY_SALIENCE_GATED => config.sensory_activity_present,
        POLICY_DECIMATED => config.tick_index.is_multiple_of(2),
        POLICY_SLEEP_QUEUED => false,
        _ => false,
    }
}

fn cadence_due(cadence: u32, tick_index: u64) -> bool {
    match cadence {
        CADENCE_HOT_60HZ => true,
        CADENCE_HOT_15_TO_60HZ => tick_index.is_multiple_of(2),
        CADENCE_HOT_10_TO_30HZ => tick_index.is_multiple_of(4),
        CADENCE_HOT_5_TO_15HZ => tick_index.is_multiple_of(8),
        CADENCE_HOT_1_TO_5HZ => tick_index.is_multiple_of(16),
        _ => false,
    }
}

fn policy_priority(policy: u32) -> u8 {
    match policy {
        POLICY_ESSENTIAL_RESERVATION => 0,
        POLICY_SALIENCE_GATED => 1,
        POLICY_DECIMATED => 2,
        POLICY_SLEEP_QUEUED => 3,
        _ => 4,
    }
}

fn projection_type_code_matches(projection_type: alife_core::ProjectionType, code: u32) -> bool {
    matches!(
        (projection_type, code),
        (alife_core::ProjectionType::FeedForward, 1)
            | (alife_core::ProjectionType::Feedback, 2)
            | (alife_core::ProjectionType::Recurrent, 3)
            | (alife_core::ProjectionType::Modulatory, 4)
            | (alife_core::ProjectionType::MotorProposal, 5)
            | (alife_core::ProjectionType::Homeostatic, 6)
            | (alife_core::ProjectionType::LateralInhibition, 7)
    )
}

fn tile_key(tile: &GpuTileMetadataRecord) -> (u32, u32, u32) {
    (
        tile.projection_index,
        tile.microtile_row,
        tile.microtile_col,
    )
}

fn mask_word_and_bit(local_bit: u32) -> Result<(u32, u32), ScaffoldContractError> {
    if local_bit >= 64 {
        return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
    }
    Ok((local_bit / 32, local_bit % 32))
}

fn checked_u32(value: usize) -> Result<u32, ScaffoldContractError> {
    u32::try_from(value).map_err(|_| ScaffoldContractError::InvalidSparseProjectionSchema)
}
