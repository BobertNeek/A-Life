//! v0 runtime milestone: P28 sleep/offline structural recompaction contracts.
//!
//! This module compiles P16 sleep-phase structural edit batches into validated
//! GPU upload buffer rebuilds. It is deliberately host-side and double-buffered:
//! active gameplay buffers remain valid until a validated scratch upload is
//! swapped at a safe sleep/offline boundary. No active tick readback or in-place
//! structural mutation is exposed here.

use std::collections::BTreeSet;

use alife_core::{
    require_current_version, NeuralProjectionSchema, ScaffoldContractError, SchemaKind,
    StructuralEditBatch, StructuralEditKind, StructuralEditReason, Tick, Validate,
};

use crate::{
    GpuPackedSynapseIndexRecord, GpuTileMetadataRecord, GpuUploadBuffers,
    GPU_BUFFER_CONTRACT_SCHEMA_VERSION,
};

pub const GPU_RECOMPACTION_SCHEMA_VERSION: u16 = 1;
pub const P28_WGSL_RECOMPACTION_AUTOPHAGY: &str =
    include_str!("../shaders/p28_recompaction_autophagy.wgsl");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuAutophagyPolicy {
    pub schema_version: u16,
    pub max_edit_candidates: usize,
    pub prune_zero_effective_synapses: bool,
    pub prune_zero_h_shadow_only: bool,
    pub h_shadow_abs_prune_threshold_q: i16,
    pub byproduct_decay_quorum: u32,
    pub brain_atp_recovery_per_pruned_q16: u32,
}

impl GpuAutophagyPolicy {
    pub const fn reference() -> Self {
        Self {
            schema_version: GPU_RECOMPACTION_SCHEMA_VERSION,
            max_edit_candidates: 64,
            prune_zero_effective_synapses: true,
            prune_zero_h_shadow_only: true,
            h_shadow_abs_prune_threshold_q: 0,
            byproduct_decay_quorum: 1,
            brain_atp_recovery_per_pruned_q16: 256,
        }
    }

    pub fn validate(self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != GPU_RECOMPACTION_SCHEMA_VERSION || self.max_edit_candidates == 0 {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        if self.byproduct_decay_quorum == 0 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuStructuralEditStatus {
    Accepted,
    RejectedInvalidReference,
    UnsupportedDeferred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuAutophagyMarkerKind {
    LowSalienceZeroEffective,
    DecayedTrace,
    InactiveZeroContribution,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuAutophagyMarker {
    pub old_weight_index: u32,
    pub projection_index: u32,
    pub tile_metadata_index: u32,
    pub reason: StructuralEditReason,
    pub kind: GpuAutophagyMarkerKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuStructuralEditPlanEntry {
    pub candidate_id: u64,
    pub projection_index: u32,
    pub kind: StructuralEditKind,
    pub reason: StructuralEditReason,
    pub status: GpuStructuralEditStatus,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct GpuRecompactionDiagnostics {
    pub edit_candidates_received: u32,
    pub edits_accepted: u32,
    pub edits_rejected: u32,
    pub pruned_entries: u32,
    pub remapped_entries: u32,
    pub preserved_entries: u32,
    pub swap_validation_failures: u32,
    pub unsupported_edit_kinds: u32,
    pub routing_mask_coherence_failures: u32,
    pub byproduct_decay_events: u32,
    pub brain_atp_recovery_signal_q16: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuLogicalBufferRef {
    pub generation: u64,
    pub tile_count: u32,
    pub synapse_count: u32,
    pub encoded_len_bytes: u64,
}

impl GpuLogicalBufferRef {
    fn from_upload(generation: u64, upload: &GpuUploadBuffers) -> Self {
        Self {
            generation,
            tile_count: upload.header.tile_count,
            synapse_count: upload.header.synapse_count,
            encoded_len_bytes: upload.encoded_bytes().len() as u64,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GpuAffectedTileRef {
    pub projection_index: u32,
    pub tile_metadata_index: u32,
    pub microtile_row: u32,
    pub microtile_col: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuRecompactionRemapTable {
    pub old_to_new: Vec<Option<u32>>,
}

impl GpuRecompactionRemapTable {
    pub fn validate(&self, new_synapse_count: u32) -> Result<(), ScaffoldContractError> {
        let mut seen = BTreeSet::new();
        for maybe_new in &self.old_to_new {
            let Some(new_index) = maybe_new else {
                continue;
            };
            if *new_index >= new_synapse_count || !seen.insert(*new_index) {
                return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
            }
        }
        if seen.len() != new_synapse_count as usize {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        Ok(())
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct GpuRoutingMaskPreservation {
    pub masks_before: u32,
    pub masks_after: u32,
    pub routing_descriptors_before: u32,
    pub routing_descriptors_after: u32,
    pub failures: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuRecompactionValidationStatus {
    Validated,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuRecompactionOutput {
    pub old_buffer_ref: GpuLogicalBufferRef,
    pub new_buffer_ref: GpuLogicalBufferRef,
    pub remap: GpuRecompactionRemapTable,
    pub affected_projections: Vec<u32>,
    pub affected_tiles: Vec<GpuAffectedTileRef>,
    pub routing_mask_preservation: GpuRoutingMaskPreservation,
    pub diagnostics: GpuRecompactionDiagnostics,
    pub validation_status: GpuRecompactionValidationStatus,
    pub autophagy_markers: Vec<GpuAutophagyMarker>,
    pub compacted_upload: GpuUploadBuffers,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuRecompactionPlan {
    pub schema_version: u16,
    pub sleep_batch_tick: Tick,
    pub expected_gpu_schema_version: u16,
    pub expected_neural_schema_version: u16,
    pub brain_class_id: u32,
    pub neuron_count: u32,
    pub policy: GpuAutophagyPolicy,
    pub edit_entries: Vec<GpuStructuralEditPlanEntry>,
    pub diagnostics: GpuRecompactionDiagnostics,
}

impl GpuRecompactionPlan {
    pub fn from_sleep_batch(
        schema: &NeuralProjectionSchema,
        batch: &StructuralEditBatch,
        policy: GpuAutophagyPolicy,
    ) -> Result<Self, ScaffoldContractError> {
        policy.validate()?;
        schema.validate()?;
        batch.validate_contract()?;
        if batch.candidates().len() > policy.max_edit_candidates {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }

        let mut diagnostics = GpuRecompactionDiagnostics {
            edit_candidates_received: batch.candidates().len() as u32,
            ..GpuRecompactionDiagnostics::default()
        };
        let mut edit_entries = Vec::with_capacity(batch.candidates().len());
        for candidate in batch.candidates() {
            let Some(projection_index) = schema
                .projections
                .iter()
                .find(|projection| projection.routing_ref == candidate.projection)
                .map(|projection| projection.projection_index)
            else {
                diagnostics.edits_rejected = diagnostics.edits_rejected.saturating_add(1);
                edit_entries.push(GpuStructuralEditPlanEntry {
                    candidate_id: candidate.candidate_id,
                    projection_index: u32::MAX,
                    kind: candidate.kind,
                    reason: candidate.reason,
                    status: GpuStructuralEditStatus::RejectedInvalidReference,
                });
                continue;
            };

            let status = match candidate.kind {
                StructuralEditKind::PruneMarker
                | StructuralEditKind::Consolidate
                | StructuralEditKind::RecompactionHint => {
                    diagnostics.edits_accepted = diagnostics.edits_accepted.saturating_add(1);
                    GpuStructuralEditStatus::Accepted
                }
                StructuralEditKind::Strengthen
                | StructuralEditKind::Weaken
                | StructuralEditKind::SynaptogenesisCandidate => {
                    diagnostics.edits_rejected = diagnostics.edits_rejected.saturating_add(1);
                    diagnostics.unsupported_edit_kinds =
                        diagnostics.unsupported_edit_kinds.saturating_add(1);
                    GpuStructuralEditStatus::UnsupportedDeferred
                }
            };

            edit_entries.push(GpuStructuralEditPlanEntry {
                candidate_id: candidate.candidate_id,
                projection_index,
                kind: candidate.kind,
                reason: candidate.reason,
                status,
            });
        }
        edit_entries.sort_by_key(|entry| {
            (
                entry.candidate_id,
                entry.projection_index,
                edit_kind_sort_key(entry.kind),
                edit_reason_sort_key(entry.reason),
            )
        });

        Ok(Self {
            schema_version: GPU_RECOMPACTION_SCHEMA_VERSION,
            sleep_batch_tick: batch.tick,
            expected_gpu_schema_version: GPU_BUFFER_CONTRACT_SCHEMA_VERSION,
            expected_neural_schema_version: schema.schema_version,
            brain_class_id: u32::from(schema.brain_class_id.raw()),
            neuron_count: schema.neuron_count,
            policy,
            edit_entries,
            diagnostics,
        })
    }

    pub fn rebuild_scratch_upload(
        &self,
        active_upload: &GpuUploadBuffers,
    ) -> Result<GpuRecompactionOutput, ScaffoldContractError> {
        self.validate_against_active_upload(active_upload)?;
        let mut diagnostics = self.diagnostics;
        let prune_enabled = self.policy.prune_zero_effective_synapses
            && self.edit_entries.iter().any(|entry| {
                entry.status == GpuStructuralEditStatus::Accepted
                    && entry.kind == StructuralEditKind::PruneMarker
            });

        let mut compacted = active_upload.clone();
        let mut old_to_new = vec![None; active_upload.packed_indices.len()];
        let mut autophagy_markers = Vec::new();
        let mut packed_indices = Vec::new();
        let mut genetic_fixed_q = Vec::new();
        let mut lifetime_consolidated_q = Vec::new();
        let mut alpha_q16 = Vec::new();
        let mut h_operational_q = Vec::new();
        let mut h_shadow_q = Vec::new();
        let mut tile_metadata = Vec::with_capacity(active_upload.tile_metadata.len());

        for (tile_index, tile) in active_upload.tile_metadata.iter().copied().enumerate() {
            let synapse_offset = packed_indices.len() as u32;
            let mut tile_synapse_count = 0_u32;
            for (old_index, synapse) in active_upload
                .packed_indices
                .iter()
                .copied()
                .enumerate()
                .filter(|(_, synapse)| synapse.tile_metadata_index as usize == tile_index)
            {
                if prune_enabled && self.synapse_is_autophagic_prune(active_upload, old_index)? {
                    diagnostics.pruned_entries = diagnostics.pruned_entries.saturating_add(1);
                    autophagy_markers.push(GpuAutophagyMarker {
                        old_weight_index: synapse.weight_index,
                        projection_index: synapse_for_projection(tile),
                        tile_metadata_index: tile_index as u32,
                        reason: StructuralEditReason::LowSalience,
                        kind: GpuAutophagyMarkerKind::LowSalienceZeroEffective,
                    });
                    continue;
                }

                let new_index = packed_indices.len() as u32;
                old_to_new[old_index] = Some(new_index);
                packed_indices.push(GpuPackedSynapseIndexRecord {
                    weight_index: new_index,
                    tile_metadata_index: tile_index as u32,
                    ..synapse
                });
                copy_weight_slot(
                    active_upload,
                    synapse.weight_index as usize,
                    &mut genetic_fixed_q,
                    &mut lifetime_consolidated_q,
                    &mut alpha_q16,
                    &mut h_operational_q,
                    &mut h_shadow_q,
                )?;
                tile_synapse_count = tile_synapse_count.saturating_add(1);
                diagnostics.preserved_entries = diagnostics.preserved_entries.saturating_add(1);
            }

            tile_metadata.push(GpuTileMetadataRecord {
                synapse_offset,
                synapse_count: tile_synapse_count,
                nonzero_count: tile_synapse_count,
                ..tile
            });
        }

        if diagnostics.pruned_entries > 0 {
            diagnostics.remapped_entries = diagnostics.preserved_entries;
            if diagnostics.pruned_entries >= self.policy.byproduct_decay_quorum {
                diagnostics.byproduct_decay_events = diagnostics.pruned_entries;
                diagnostics.brain_atp_recovery_signal_q16 = diagnostics
                    .pruned_entries
                    .saturating_mul(self.policy.brain_atp_recovery_per_pruned_q16);
            }
        }

        compacted.tile_metadata = tile_metadata;
        compacted.packed_indices = packed_indices;
        compacted.genetic_fixed_q = genetic_fixed_q;
        compacted.lifetime_consolidated_q = lifetime_consolidated_q;
        compacted.alpha_q16 = alpha_q16;
        compacted.h_operational_q = h_operational_q;
        compacted.h_shadow_q = h_shadow_q;
        compacted.header.synapse_count = compacted.packed_indices.len() as u32;
        compacted.header.tile_count = compacted.tile_metadata.len() as u32;

        let remap = GpuRecompactionRemapTable { old_to_new };
        remap.validate(compacted.header.synapse_count)?;
        validate_upload_internal_shape(&compacted)?;

        let routing_mask_preservation = routing_mask_preservation(active_upload, &compacted);
        diagnostics.routing_mask_coherence_failures = routing_mask_preservation.failures;

        Ok(GpuRecompactionOutput {
            old_buffer_ref: GpuLogicalBufferRef::from_upload(
                self.sleep_batch_tick.raw(),
                active_upload,
            ),
            new_buffer_ref: GpuLogicalBufferRef::from_upload(
                self.sleep_batch_tick.raw().saturating_add(1),
                &compacted,
            ),
            remap,
            affected_projections: affected_projections(&self.edit_entries),
            affected_tiles: affected_tiles(&compacted.tile_metadata, &self.edit_entries),
            routing_mask_preservation,
            diagnostics,
            validation_status: GpuRecompactionValidationStatus::Validated,
            autophagy_markers,
            compacted_upload: compacted,
        })
    }

    fn validate_against_active_upload(
        &self,
        active_upload: &GpuUploadBuffers,
    ) -> Result<(), ScaffoldContractError> {
        if self.schema_version != GPU_RECOMPACTION_SCHEMA_VERSION
            || active_upload.header.gpu_schema_version != self.expected_gpu_schema_version
            || active_upload.header.neural_projection_schema_version
                != self.expected_neural_schema_version
            || active_upload.header.brain_class_id != self.brain_class_id
            || active_upload.header.neuron_count != self.neuron_count
        {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        validate_upload_internal_shape(active_upload)
    }

    fn synapse_is_autophagic_prune(
        &self,
        upload: &GpuUploadBuffers,
        old_index: usize,
    ) -> Result<bool, ScaffoldContractError> {
        let genetic = *upload
            .genetic_fixed_q
            .get(old_index)
            .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;
        let lifetime = *upload
            .lifetime_consolidated_q
            .get(old_index)
            .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;
        let alpha = *upload
            .alpha_q16
            .get(old_index)
            .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;
        let h_operational = *upload
            .h_operational_q
            .get(old_index)
            .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;
        let h_shadow = *upload
            .h_shadow_q
            .get(old_index)
            .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;
        let alpha_h = round_div_i64(
            i64::from(alpha) * i64::from(h_operational),
            i64::from(u16::MAX),
        )?;
        let effective = i32::from(genetic)
            .checked_add(i32::from(lifetime))
            .and_then(|value| value.checked_add(alpha_h))
            .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
        let h_shadow_abs = i32::from(h_shadow).abs();
        Ok(effective == 0
            && (!self.policy.prune_zero_h_shadow_only
                || h_shadow_abs <= i32::from(self.policy.h_shadow_abs_prune_threshold_q)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuRecompactionSwapState {
    Idle,
    Uploading,
    ReadyToSwap,
    Active,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuBufferReplacement {
    active_upload: GpuUploadBuffers,
    scratch_upload: Option<GpuUploadBuffers>,
    state: GpuRecompactionSwapState,
    diagnostics: GpuRecompactionDiagnostics,
}

impl GpuBufferReplacement {
    pub fn stage(
        active_upload: GpuUploadBuffers,
        output: GpuRecompactionOutput,
    ) -> Result<Self, ScaffoldContractError> {
        if output.validation_status != GpuRecompactionValidationStatus::Validated
            || output.old_buffer_ref.synapse_count != active_upload.header.synapse_count
        {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        validate_upload_internal_shape(&active_upload)?;
        validate_upload_internal_shape(&output.compacted_upload)?;
        Ok(Self {
            active_upload,
            scratch_upload: Some(output.compacted_upload),
            state: GpuRecompactionSwapState::ReadyToSwap,
            diagnostics: output.diagnostics,
        })
    }

    pub const fn state(&self) -> GpuRecompactionSwapState {
        self.state
    }

    pub const fn diagnostics(&self) -> GpuRecompactionDiagnostics {
        self.diagnostics
    }

    pub fn active_upload(&self) -> &GpuUploadBuffers {
        &self.active_upload
    }

    pub fn reject_for_failed_validation(mut self) -> Self {
        self.diagnostics.swap_validation_failures =
            self.diagnostics.swap_validation_failures.saturating_add(1);
        self.scratch_upload = None;
        self.state = GpuRecompactionSwapState::Failed;
        self
    }

    pub fn swap_at_sleep_boundary(mut self) -> Result<Self, ScaffoldContractError> {
        if self.state != GpuRecompactionSwapState::ReadyToSwap {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        let scratch = self
            .scratch_upload
            .take()
            .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;
        validate_upload_internal_shape(&scratch)?;
        self.active_upload = scratch;
        self.state = GpuRecompactionSwapState::Active;
        Ok(self)
    }
}

fn validate_upload_internal_shape(upload: &GpuUploadBuffers) -> Result<(), ScaffoldContractError> {
    require_current_version(
        SchemaKind::NeuralProjection,
        upload.header.neural_projection_schema_version,
    )?;
    let synapse_count = upload.packed_indices.len();
    if upload.header.gpu_schema_version != GPU_BUFFER_CONTRACT_SCHEMA_VERSION
        || upload.header.tile_count != upload.tile_metadata.len() as u32
        || upload.header.synapse_count != synapse_count as u32
        || upload.header.routing_descriptor_count != upload.routing_descriptors.len() as u32
        || upload.genetic_fixed_q.len() != synapse_count
        || upload.lifetime_consolidated_q.len() != synapse_count
        || upload.alpha_q16.len() != synapse_count
        || upload.h_operational_q.len() != synapse_count
        || upload.h_shadow_q.len() != synapse_count
    {
        return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
    }
    for tile in &upload.tile_metadata {
        let start = tile.synapse_offset as usize;
        let end = start
            .checked_add(tile.synapse_count as usize)
            .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?;
        if end > synapse_count {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
    }
    for synapse in &upload.packed_indices {
        if synapse.weight_index as usize >= synapse_count
            || synapse.tile_metadata_index as usize >= upload.tile_metadata.len()
        {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
    }
    Ok(())
}

fn copy_weight_slot(
    upload: &GpuUploadBuffers,
    old_weight_index: usize,
    genetic_fixed_q: &mut Vec<i16>,
    lifetime_consolidated_q: &mut Vec<i16>,
    alpha_q16: &mut Vec<u16>,
    h_operational_q: &mut Vec<i16>,
    h_shadow_q: &mut Vec<i16>,
) -> Result<(), ScaffoldContractError> {
    genetic_fixed_q.push(
        *upload
            .genetic_fixed_q
            .get(old_weight_index)
            .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?,
    );
    lifetime_consolidated_q.push(
        *upload
            .lifetime_consolidated_q
            .get(old_weight_index)
            .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?,
    );
    alpha_q16.push(
        *upload
            .alpha_q16
            .get(old_weight_index)
            .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?,
    );
    h_operational_q.push(
        *upload
            .h_operational_q
            .get(old_weight_index)
            .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?,
    );
    h_shadow_q.push(
        *upload
            .h_shadow_q
            .get(old_weight_index)
            .ok_or(ScaffoldContractError::InvalidSparseProjectionSchema)?,
    );
    Ok(())
}

fn routing_mask_preservation(
    old: &GpuUploadBuffers,
    new: &GpuUploadBuffers,
) -> GpuRoutingMaskPreservation {
    let failures = u32::from(old.supertile_masks != new.supertile_masks)
        + u32::from(old.routing_descriptors != new.routing_descriptors);
    GpuRoutingMaskPreservation {
        masks_before: old.supertile_masks.len() as u32,
        masks_after: new.supertile_masks.len() as u32,
        routing_descriptors_before: old.routing_descriptors.len() as u32,
        routing_descriptors_after: new.routing_descriptors.len() as u32,
        failures,
    }
}

fn affected_projections(entries: &[GpuStructuralEditPlanEntry]) -> Vec<u32> {
    entries
        .iter()
        .filter(|entry| entry.status == GpuStructuralEditStatus::Accepted)
        .map(|entry| entry.projection_index)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn affected_tiles(
    tiles: &[GpuTileMetadataRecord],
    entries: &[GpuStructuralEditPlanEntry],
) -> Vec<GpuAffectedTileRef> {
    let affected_projection_set = affected_projections(entries)
        .into_iter()
        .collect::<BTreeSet<_>>();
    tiles
        .iter()
        .enumerate()
        .filter(|(_, tile)| affected_projection_set.contains(&tile.projection_index))
        .map(|(tile_metadata_index, tile)| GpuAffectedTileRef {
            projection_index: tile.projection_index,
            tile_metadata_index: tile_metadata_index as u32,
            microtile_row: tile.microtile_row,
            microtile_col: tile.microtile_col,
        })
        .collect()
}

fn synapse_for_projection(tile: GpuTileMetadataRecord) -> u32 {
    tile.projection_index
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

fn edit_kind_sort_key(kind: StructuralEditKind) -> u8 {
    match kind {
        StructuralEditKind::PruneMarker => 0,
        StructuralEditKind::Strengthen => 1,
        StructuralEditKind::Weaken => 2,
        StructuralEditKind::SynaptogenesisCandidate => 3,
        StructuralEditKind::Consolidate => 4,
        StructuralEditKind::RecompactionHint => 5,
    }
}

fn edit_reason_sort_key(reason: StructuralEditReason) -> u8 {
    match reason {
        StructuralEditReason::MemoryCorrelation => 0,
        StructuralEditReason::TopologyCorrelation => 1,
        StructuralEditReason::LowSalience => 2,
        StructuralEditReason::Recovery => 3,
        StructuralEditReason::Fatigue => 4,
    }
}
