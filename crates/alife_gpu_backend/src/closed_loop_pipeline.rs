//! Production GPU-authoritative perception, recurrent, selection, and eligibility dispatch.
//!
//! This module exposes no CPU neural execution and obtains neural results only
//! from WGSL state plus bounded compact receipts.

use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroU64,
    sync::{mpsc, Arc},
};

use alife_core::{
    BrainCapacityClass, BrainPhenotype, NeuralThrottleDecision, PerceptionFrame, SchemaVersions,
    CANDIDATE_FEATURE_COUNT, MAX_ACTION_CANDIDATES,
};
use bytemuck::Zeroable;

use crate::{
    phenotype_hash_from_gpu_words, split_u64x2, GpuActivityDispatchHeader, GpuBrainSlot,
    GpuCandidateMemoryRecord, GpuCandidateRecord, GpuClassBucketBuffers, GpuClosedLoopError,
    GpuEligibilityDiscardRecord, GpuFastPlasticityCommitRecord, GpuFixedClassArenaBuffers,
    GpuLearningHeader, GpuMemoryContextDispatchReceipt, GpuMemoryContextHeader,
    GpuMemoryContextUpload, GpuOutcomeCreditRecord, GpuPendingEligibilityRecord,
    GpuPerceptionHeader, GpuPerceptionUpload, GpuSelectionRecord, CLOSED_LOOP_ELIGIBILITY_WGSL,
    CLOSED_LOOP_MEMORY_CONTEXT_WGSL, GPU_CLOSED_LOOP_TICK_READBACK_BYTES,
    GPU_FAST_PLASTICITY_COMMIT_BYTES, GPU_FAST_PLASTICITY_COMMIT_WORDS, GPU_LEARNING_HEADER_WORDS,
    GPU_OUTCOME_CREDIT_WORDS,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BatchLifecycleStage {
    Built,
    EncodeRecorded,
    RecurrentRecorded,
    SelectionRecorded,
    EligibilityRecorded,
}

pub(crate) struct GpuTimedFastPlasticityResult {
    pub records: Vec<GpuFastPlasticityCommitRecord>,
    pub timestamp_delta_ticks: u64,
}

pub(crate) struct GpuTimestampQueryResources<'a> {
    query_set: &'a wgpu::QuerySet,
    resolve_buffer: &'a wgpu::Buffer,
    readback_buffer: &'a wgpu::Buffer,
}

impl<'a> GpuTimestampQueryResources<'a> {
    pub(crate) const fn new(
        query_set: &'a wgpu::QuerySet,
        resolve_buffer: &'a wgpu::Buffer,
        readback_buffer: &'a wgpu::Buffer,
    ) -> Self {
        Self {
            query_set,
            resolve_buffer,
            readback_buffer,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingBatchAuthority {
    nonce: u64,
    stage: BatchLifecycleStage,
}

#[derive(Debug, Default)]
struct BatchAuthority {
    active_sides: BTreeMap<(u32, u32), u32>,
    pending: Option<PendingBatchAuthority>,
    poisoned_nonce: Option<u64>,
}

impl BatchAuthority {
    fn begin(&mut self, nonce: u64) -> Result<(), GpuClosedLoopError> {
        self.ensure_healthy()?;
        if self.pending.is_some() {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        self.pending = Some(PendingBatchAuthority {
            nonce,
            stage: BatchLifecycleStage::Built,
        });
        Ok(())
    }

    fn record_encode(&mut self, nonce: u64) -> Result<(), GpuClosedLoopError> {
        let pending = self.pending_mut(nonce)?;
        if pending.stage != BatchLifecycleStage::Built {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        pending.stage = BatchLifecycleStage::EncodeRecorded;
        Ok(())
    }

    fn record_recurrent(&mut self, nonce: u64) -> Result<(), GpuClosedLoopError> {
        let pending = self.pending_mut(nonce)?;
        if pending.stage != BatchLifecycleStage::EncodeRecorded {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        pending.stage = BatchLifecycleStage::RecurrentRecorded;
        Ok(())
    }

    fn recording_failed(&mut self, nonce: u64) -> Result<(), GpuClosedLoopError> {
        let pending = self.pending_mut(nonce)?;
        pending.stage = BatchLifecycleStage::Built;
        Ok(())
    }

    fn submission_indeterminate(&mut self, nonce: u64) -> Result<(), GpuClosedLoopError> {
        let pending = self.pending_mut(nonce)?;
        if !matches!(
            pending.stage,
            BatchLifecycleStage::RecurrentRecorded
                | BatchLifecycleStage::SelectionRecorded
                | BatchLifecycleStage::EligibilityRecorded
        ) {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        self.poisoned_nonce = Some(nonce);
        Ok(())
    }

    fn abandon_unsubmitted(&mut self, nonce: u64) -> Result<(), GpuClosedLoopError> {
        let pending = self.pending_mut(nonce)?;
        if pending.stage != BatchLifecycleStage::Built {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        self.pending = None;
        Ok(())
    }

    fn submission_succeeded(
        &mut self,
        nonce: u64,
        final_sides: &[(u32, u32, u32)],
    ) -> Result<(), GpuClosedLoopError> {
        let pending = self.pending_mut(nonce)?;
        if pending.stage != BatchLifecycleStage::EligibilityRecorded {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        for &(slot, generation, side) in final_sides {
            self.active_sides.insert((slot, generation), side);
        }
        self.pending = None;
        Ok(())
    }

    fn prevalidate_submission_succeeded(&self, nonce: u64) -> Result<(), GpuClosedLoopError> {
        self.require_stage(nonce, BatchLifecycleStage::EligibilityRecorded)
    }

    fn record_selection(&mut self, nonce: u64) -> Result<(), GpuClosedLoopError> {
        let pending = self.pending_mut(nonce)?;
        if pending.stage != BatchLifecycleStage::RecurrentRecorded {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        pending.stage = BatchLifecycleStage::SelectionRecorded;
        Ok(())
    }

    fn record_eligibility(&mut self, nonce: u64) -> Result<(), GpuClosedLoopError> {
        let pending = self.pending_mut(nonce)?;
        if pending.stage != BatchLifecycleStage::SelectionRecorded {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        pending.stage = BatchLifecycleStage::EligibilityRecorded;
        Ok(())
    }

    #[cfg(feature = "gpu-tests")]
    fn recurrent_diagnostic_succeeded(
        &mut self,
        nonce: u64,
        final_sides: &[(u32, u32, u32)],
    ) -> Result<(), GpuClosedLoopError> {
        let pending = self.pending_mut(nonce)?;
        if pending.stage != BatchLifecycleStage::RecurrentRecorded {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        for &(slot, generation, side) in final_sides {
            self.active_sides.insert((slot, generation), side);
        }
        self.pending = None;
        Ok(())
    }

    fn pending_mut(
        &mut self,
        nonce: u64,
    ) -> Result<&mut PendingBatchAuthority, GpuClosedLoopError> {
        self.ensure_healthy()?;
        self.pending
            .as_mut()
            .filter(|pending| pending.nonce == nonce)
            .ok_or(GpuClosedLoopError::StaleOrForeignHandle)
    }

    fn require_stage(
        &self,
        nonce: u64,
        stage: BatchLifecycleStage,
    ) -> Result<(), GpuClosedLoopError> {
        self.ensure_healthy()?;
        if self.pending == Some(PendingBatchAuthority { nonce, stage }) {
            Ok(())
        } else {
            Err(GpuClosedLoopError::MalformedUpload)
        }
    }

    fn ensure_healthy(&self) -> Result<(), GpuClosedLoopError> {
        if self.poisoned_nonce.is_some() {
            Err(GpuClosedLoopError::SubmissionFailed)
        } else {
            Ok(())
        }
    }

    fn retire_active_side(&mut self, slot: u32, generation: u32) -> Result<(), GpuClosedLoopError> {
        self.ensure_healthy()?;
        if self.pending.is_some() {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        self.active_sides.remove(&(slot, generation));
        Ok(())
    }
}

pub const GPU_PERCEPTION_DISPATCH_ROW_WORDS: usize = 272;
pub const GPU_ACTIVE_DISPATCH_ROW_WORDS: usize = GPU_PERCEPTION_DISPATCH_ROW_WORDS
    + GPU_LEARNING_HEADER_WORDS
    + crate::GPU_MEMORY_CONTEXT_HEADER_WORDS
    + crate::GPU_ACTIVITY_DISPATCH_HEADER_WORDS;
pub const GPU_ACTIVE_SIDE_DIAGNOSTIC_LANE: u32 = 3;
const GPU_PERCEPTION_HEADER_WORDS: usize = 16;
const GPU_CANDIDATE_RECORD_WORDS: usize = 8;
const WORKGROUP_SIZE: u32 = 64;
const GPU_REQUIRED_MAX_BUFFER_WORDS: usize = 268_435_456 / 4;

pub const CLOSED_LOOP_ENCODE_WGSL: &str = concat!(
    include_str!("../shaders/closed_loop_abi.wgsl"),
    include_str!("../shaders/closed_loop_activity_validation.wgsl"),
    include_str!("../shaders/closed_loop_encode.wgsl")
);
pub const CLOSED_LOOP_CLEAR_DIAGNOSTICS_WGSL: &str = concat!(
    include_str!("../shaders/closed_loop_abi.wgsl"),
    include_str!("../shaders/closed_loop_activity_validation.wgsl"),
    include_str!("../shaders/closed_loop_clear_diagnostics.wgsl")
);
pub const CLOSED_LOOP_RECURRENT_WGSL: &str = concat!(
    include_str!("../shaders/closed_loop_abi.wgsl"),
    include_str!("../shaders/closed_loop_activity_validation.wgsl"),
    include_str!("../shaders/closed_loop_recurrent.wgsl")
);
pub const CLOSED_LOOP_DECODE_WGSL: &str = concat!(
    include_str!("../shaders/closed_loop_abi.wgsl"),
    include_str!("../shaders/closed_loop_activity_validation.wgsl"),
    include_str!("../shaders/closed_loop_decode.wgsl")
);
pub const CLOSED_LOOP_PLASTICITY_WGSL: &str = concat!(
    include_str!("../shaders/closed_loop_abi.wgsl"),
    include_str!("../shaders/closed_loop_plasticity.wgsl")
);
pub const CLOSED_LOOP_CONSOLIDATE_WGSL: &str = concat!(
    include_str!("../shaders/closed_loop_abi.wgsl"),
    include_str!("../shaders/closed_loop_consolidate.wgsl")
);
pub const CLOSED_LOOP_REPLAY_LEARNING_WGSL: &str = concat!(
    include_str!("../shaders/closed_loop_abi.wgsl"),
    include_str!("../shaders/closed_loop_replay_learning.wgsl")
);

pub(crate) struct GpuFastPlasticityBatchEntry<'a> {
    pub slot: &'a GpuBrainSlot,
    pub pending: &'a GpuPendingEligibilityRecord,
    pub outcome: GpuOutcomeCreditRecord,
    pub active_weight_generation: u64,
    pub replay_generation: u64,
    pub transaction_generation: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct GpuActiveBatchEntry<'a> {
    frame: &'a PerceptionFrame,
    slot: &'a GpuBrainSlot,
    phenotype: &'a BrainPhenotype,
    activity: &'a NeuralThrottleDecision,
    memory_upload: Option<&'a GpuMemoryContextUpload>,
    active_eligibility_generation: u64,
}

impl<'a> GpuActiveBatchEntry<'a> {
    pub const fn new(
        frame: &'a PerceptionFrame,
        slot: &'a GpuBrainSlot,
        phenotype: &'a BrainPhenotype,
        activity: &'a NeuralThrottleDecision,
    ) -> Self {
        Self {
            frame,
            slot,
            phenotype,
            activity,
            memory_upload: None,
            active_eligibility_generation: 1,
        }
    }

    pub const fn with_memory(
        frame: &'a PerceptionFrame,
        slot: &'a GpuBrainSlot,
        phenotype: &'a BrainPhenotype,
        activity: &'a NeuralThrottleDecision,
        memory_upload: &'a GpuMemoryContextUpload,
    ) -> Self {
        Self {
            frame,
            slot,
            phenotype,
            activity,
            memory_upload: Some(memory_upload),
            active_eligibility_generation: 1,
        }
    }
}

/// Borrowed fixed-arena row used by the live runtime. It carries no packed
/// append-plan state and binds the physical arena index explicitly.
pub(crate) struct GpuFixedActiveBatchEntry<'a> {
    frame: &'a PerceptionFrame,
    slot: &'a GpuBrainSlot,
    phenotype: &'a BrainPhenotype,
    activity: &'a NeuralThrottleDecision,
    memory_upload: Option<&'a GpuMemoryContextUpload>,
    active_eligibility_generation: u64,
}

impl<'a> GpuFixedActiveBatchEntry<'a> {
    pub(crate) const fn new(
        frame: &'a PerceptionFrame,
        slot: &'a GpuBrainSlot,
        phenotype: &'a BrainPhenotype,
        activity: &'a NeuralThrottleDecision,
        active_eligibility_generation: u64,
    ) -> Self {
        Self {
            frame,
            slot,
            phenotype,
            activity,
            memory_upload: None,
            active_eligibility_generation,
        }
    }

    pub(crate) const fn with_memory(
        frame: &'a PerceptionFrame,
        slot: &'a GpuBrainSlot,
        phenotype: &'a BrainPhenotype,
        activity: &'a NeuralThrottleDecision,
        memory_upload: &'a GpuMemoryContextUpload,
        active_eligibility_generation: u64,
    ) -> Self {
        Self {
            frame,
            slot,
            phenotype,
            activity,
            memory_upload: Some(memory_upload),
            active_eligibility_generation,
        }
    }
}

#[derive(Clone, Copy)]
struct GpuBatchEntryView<'a> {
    frame: &'a PerceptionFrame,
    slot: &'a GpuBrainSlot,
    phenotype: &'a BrainPhenotype,
    activity: &'a NeuralThrottleDecision,
    memory_upload: Option<&'a GpuMemoryContextUpload>,
    active_eligibility_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuActiveBatchUpload {
    headers: Vec<GpuPerceptionHeader>,
    learning_headers: Vec<GpuLearningHeader>,
    activity_headers: Vec<GpuActivityDispatchHeader>,
    pending_templates: Vec<GpuPendingEligibilityRecord>,
    dispatch_header_words: Vec<u32>,
    frame_payload_words: Vec<u32>,
    bucket_ownership_token: u64,
    authority_nonce: u64,
    selection_offsets: Vec<u32>,
    memory_context_bindings: Vec<Option<GpuMemoryContextDispatchReceipt>>,
}

impl GpuActiveBatchUpload {
    #[allow(clippy::too_many_arguments)]
    fn try_from_views(
        entries: &[GpuBatchEntryView<'_>],
        frame_base_words: u32,
        bucket_ownership_token: u64,
        active_sides: &BTreeMap<(u32, u32), u32>,
        dispatch_capacity_words: usize,
        frame_payload_capacity_words: usize,
        dispatch_generation: NonZeroU64,
        authority_nonce: u64,
    ) -> Result<Self, GpuClosedLoopError> {
        if entries.is_empty() {
            return Err(GpuClosedLoopError::CapacityExceeded);
        }
        let dispatch_words = entries
            .len()
            .checked_mul(GPU_ACTIVE_DISPATCH_ROW_WORDS)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        if dispatch_words > GPU_REQUIRED_MAX_BUFFER_WORDS
            || dispatch_words > dispatch_capacity_words
            || frame_base_words as usize > GPU_REQUIRED_MAX_BUFFER_WORDS
            || frame_base_words as usize > frame_payload_capacity_words
        {
            return Err(GpuClosedLoopError::CapacityExceeded);
        }
        let mut dispatch_header_words = vec![0_u32; dispatch_words];
        let mut frame_payload_words = vec![0_u32; frame_base_words as usize];
        let mut headers = Vec::with_capacity(entries.len());
        let mut learning_headers = Vec::with_capacity(entries.len());
        let mut activity_headers = Vec::with_capacity(entries.len());
        let mut pending_templates = Vec::with_capacity(entries.len());
        let mut memory_context_bindings = Vec::with_capacity(entries.len());
        let mut seen_slots = BTreeSet::new();
        let class_id = entries[0].slot.record().class_id;

        for (row, entry) in entries.iter().enumerate() {
            if entry.slot.record().class_id != class_id
                || entry.slot.record().slot_generation == 0
                || !seen_slots.insert(entry.slot.brain_slot_index())
            {
                return Err(GpuClosedLoopError::StaleOrForeignHandle);
            }
            if entry.frame.candidates().len() > MAX_ACTION_CANDIDATES {
                return Err(GpuClosedLoopError::CapacityExceeded);
            }
            let mut upload = GpuPerceptionUpload::try_from_frame(
                entry.frame,
                entry.slot,
                active_sides
                    .get(&(
                        entry.slot.brain_slot_index(),
                        entry.slot.record().slot_generation,
                    ))
                    .copied()
                    .unwrap_or(0),
            )?;
            let row_base = row
                .checked_mul(GPU_ACTIVE_DISPATCH_ROW_WORDS)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
            let row_base_u32 =
                u32::try_from(row_base).map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?;
            let payload_base = u32::try_from(frame_payload_words.len())
                .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?;
            upload.rebase(row_base_u32, payload_base)?;
            upload.validate_against(entry.frame, entry.slot)?;
            let mut memory_upload = entry.memory_upload.cloned();
            if let Some(memory) = &memory_upload {
                memory.validate_for_frame_and_slot(entry.frame, entry.slot)?;
            }

            let candidate_words = upload
                .candidates
                .len()
                .checked_mul(GPU_CANDIDATE_RECORD_WORDS)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
            let used_words = GPU_PERCEPTION_HEADER_WORDS
                .checked_add(candidate_words)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
            if used_words > GPU_PERCEPTION_DISPATCH_ROW_WORDS {
                return Err(GpuClosedLoopError::CapacityExceeded);
            }
            dispatch_header_words[row_base..row_base + GPU_PERCEPTION_HEADER_WORDS]
                .copy_from_slice(upload.header.words());
            for (candidate_index, candidate) in upload.candidates.iter().enumerate() {
                let start = row_base
                    + GPU_PERCEPTION_HEADER_WORDS
                    + candidate_index * GPU_CANDIDATE_RECORD_WORDS;
                dispatch_header_words[start..start + GPU_CANDIDATE_RECORD_WORDS]
                    .copy_from_slice(candidate.words());
            }
            let candidate_digest_words = upload
                .candidates
                .len()
                .checked_mul(4)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
            let memory_record_words = memory_upload
                .as_ref()
                .map(|memory| {
                    memory
                        .records
                        .len()
                        .checked_mul(crate::GPU_CANDIDATE_MEMORY_RECORD_WORDS)
                        .ok_or(GpuClosedLoopError::ArithmeticOverflow)
                })
                .transpose()?
                .unwrap_or(0);
            let payload_end = frame_payload_words
                .len()
                .checked_add(upload.frame_payload_words.len())
                .and_then(|value| value.checked_add(candidate_digest_words))
                .and_then(|value| value.checked_add(crate::GPU_PENDING_ELIGIBILITY_WORDS))
                .and_then(|value| value.checked_add(memory_record_words))
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
            if payload_end > GPU_REQUIRED_MAX_BUFFER_WORDS
                || payload_end > frame_payload_capacity_words
            {
                return Err(GpuClosedLoopError::CapacityExceeded);
            }
            frame_payload_words.extend_from_slice(&upload.frame_payload_words);
            let decoder_learning_input_offset = upload
                .candidates
                .first()
                .map(|candidate| candidate.feature_offset)
                .ok_or(GpuClosedLoopError::MalformedUpload)?;
            let decoder_input_stride = entry.slot.decoder_input_stride();
            let expected_digest_base = decoder_learning_input_offset
                .checked_add(
                    u32::try_from(upload.candidates.len())
                        .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?
                        .checked_mul(decoder_input_stride)
                        .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
                )
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
            if usize::try_from(expected_digest_base)
                .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?
                != frame_payload_words.len()
            {
                return Err(GpuClosedLoopError::MalformedUpload);
            }
            for candidate in entry.frame.candidates() {
                let digest = candidate
                    .feature_digest()
                    .map_err(|_| GpuClosedLoopError::MalformedUpload)?;
                frame_payload_words.extend_from_slice(&split_u64x2(digest.0));
            }
            upload.header.microstep_count = u32::from(entry.activity.microsteps);
            let pending_template_offset = u32::try_from(frame_payload_words.len())
                .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?;
            let final_side =
                upload.header.active_activation_side ^ (upload.header.microstep_count & 1);
            let active_activation_side =
                u8::try_from(final_side).map_err(|_| GpuClosedLoopError::MalformedUpload)?;
            let staging_eligibility_generation = entry
                .active_eligibility_generation
                .checked_add(1)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
            let pending_template = GpuPendingEligibilityRecord::template(
                entry.slot.record().slot,
                entry.slot.record().slot_generation,
                active_activation_side,
                phenotype_hash_from_gpu_words(entry.slot.identity().phenotype_hash),
                entry.frame.organism_id(),
                dispatch_generation.get(),
                entry.frame.tick(),
                entry.frame.frame_digest(),
                entry.active_eligibility_generation,
                staging_eligibility_generation,
            )
            .map_err(|_| GpuClosedLoopError::MalformedUpload)?;
            frame_payload_words.extend_from_slice(pending_template.words());
            let memory_binding = if let Some(memory) = &mut memory_upload {
                let memory_context_offset = u32::try_from(frame_payload_words.len())
                    .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?;
                let receipt = memory.rebase_for_batch(
                    entry.frame,
                    entry.slot,
                    upload.frame_binding,
                    memory_context_offset,
                    upload.header.candidate_offset,
                    decoder_learning_input_offset,
                )?;
                for record in &memory.records {
                    frame_payload_words.extend_from_slice(record.words());
                }
                Some(receipt)
            } else {
                None
            };
            if frame_payload_words.len() != payload_end {
                return Err(GpuClosedLoopError::MalformedUpload);
            }
            upload.header.dispatch_generation_lo = dispatch_generation.get() as u32;
            upload.header.dispatch_generation_hi = (dispatch_generation.get() >> 32) as u32;
            let capacity = BrainCapacityClass::production_for_id(entry.phenotype.brain_class_id())
                .map_err(|_| GpuClosedLoopError::MalformedUpload)?;
            if entry.activity.organism_id_raw != entry.frame.organism_id().raw()
                || entry.activity.tick != entry.frame.tick().0
                || entry.activity.dispatch_generation != dispatch_generation.get()
                || entry.activity.frame_digest != entry.frame.frame_digest().0
                || phenotype_hash_from_gpu_words(entry.slot.identity().phenotype_hash)
                    != entry.phenotype.phenotype_hash()
            {
                return Err(GpuClosedLoopError::MalformedUpload);
            }
            let activity_header = GpuActivityDispatchHeader::try_from_decision(
                entry.activity,
                entry.phenotype,
                capacity.execution(),
                entry.slot,
            )
            .map_err(|_| GpuClosedLoopError::MalformedUpload)?;
            dispatch_header_words[row_base..row_base + GPU_PERCEPTION_HEADER_WORDS]
                .copy_from_slice(upload.header.words());
            let decoder_synapse_count = entry
                .slot
                .record()
                .synapse_count
                .checked_sub(entry.slot.record().recurrent_synapse_count)
                .ok_or(GpuClosedLoopError::MalformedUpload)?;
            let scheduled_work = crate::derive_executed_work(
                entry.phenotype,
                entry.activity.microsteps,
                &entry.activity.enabled_route_ids,
                upload.header.candidate_count,
                memory_upload
                    .as_ref()
                    .map_or(0, |memory| memory.header.candidate_count),
            )
            .map_err(|_| GpuClosedLoopError::MalformedUpload)?;
            let scheduled_tile_visits = u32::try_from(scheduled_work.tile_visits)
                .map_err(|_| GpuClosedLoopError::CapacityExceeded)?;
            let scheduled_synapse_ops = u32::try_from(scheduled_work.synapse_ops)
                .map_err(|_| GpuClosedLoopError::CapacityExceeded)?;
            let learning_header = GpuLearningHeader {
                schema_version: u32::from(SchemaVersions::CURRENT.learning.raw()),
                class_id: entry.slot.record().class_id,
                slot: entry.slot.record().slot,
                slot_generation: entry.slot.record().slot_generation,
                brain_slot_index: entry.slot.brain_slot_index(),
                active_activation_side: final_side,
                dispatch_generation_lo: dispatch_generation.get() as u32,
                dispatch_generation_hi: (dispatch_generation.get() >> 32) as u32,
                candidate_count: upload.header.candidate_count,
                candidate_offset: upload.header.candidate_offset,
                decoder_learning_input_offset,
                selection_offset: entry.slot.record().selection_offset,
                outcome_offset: pending_template_offset,
                recurrent_synapse_count: entry.slot.record().recurrent_synapse_count,
                decoder_synapse_count,
                decoder_input_stride,
                pending_eligibility_offset: entry
                    .slot
                    .word_ranges()
                    .pending_eligibility_words
                    .start,
                scheduled_tile_visits,
                scheduled_synapse_ops,
                scheduled_work_checksum: activity_header
                    .scheduled_work_checksum(scheduled_tile_visits, scheduled_synapse_ops),
            };
            let learning_start = row_base + GPU_PERCEPTION_DISPATCH_ROW_WORDS;
            dispatch_header_words[learning_start..learning_start + GPU_LEARNING_HEADER_WORDS]
                .copy_from_slice(learning_header.words());
            if let Some(memory) = &memory_upload {
                let memory_start = learning_start + GPU_LEARNING_HEADER_WORDS;
                dispatch_header_words
                    [memory_start..memory_start + crate::GPU_MEMORY_CONTEXT_HEADER_WORDS]
                    .copy_from_slice(memory.header.words());
            }
            let activity_start =
                learning_start + GPU_LEARNING_HEADER_WORDS + crate::GPU_MEMORY_CONTEXT_HEADER_WORDS;
            dispatch_header_words
                [activity_start..activity_start + crate::GPU_ACTIVITY_DISPATCH_HEADER_WORDS]
                .copy_from_slice(activity_header.words());
            headers.push(upload.header);
            learning_headers.push(learning_header);
            activity_headers.push(activity_header);
            pending_templates.push(pending_template);
            memory_context_bindings.push(memory_binding);
        }

        Ok(Self {
            headers,
            learning_headers,
            activity_headers,
            pending_templates,
            dispatch_header_words,
            frame_payload_words,
            bucket_ownership_token,
            authority_nonce,
            selection_offsets: entries
                .iter()
                .map(|entry| entry.slot.record().selection_offset)
                .collect(),
            memory_context_bindings,
        })
    }

    pub fn row_count(&self) -> usize {
        self.headers.len()
    }
    pub fn headers(&self) -> &[GpuPerceptionHeader] {
        &self.headers
    }
    pub fn learning_headers(&self) -> &[GpuLearningHeader] {
        &self.learning_headers
    }
    pub fn activity_headers(&self) -> &[GpuActivityDispatchHeader] {
        &self.activity_headers
    }
    pub fn dispatch_header_words(&self) -> &[u32] {
        &self.dispatch_header_words
    }
    pub fn frame_payload_words(&self) -> &[u32] {
        &self.frame_payload_words
    }
    pub fn memory_context_bindings(&self) -> &[Option<GpuMemoryContextDispatchReceipt>] {
        &self.memory_context_bindings
    }
    #[cfg(feature = "gpu-tests")]
    pub fn tamper_activity_digest_for_hardware_diagnostic(
        &mut self,
        row: usize,
    ) -> Result<(), GpuClosedLoopError> {
        let header = self
            .activity_headers
            .get_mut(row)
            .ok_or(GpuClosedLoopError::MalformedUpload)?;
        header.tamper_route_schedule_digest_for_hardware_diagnostic();
        let learning = self
            .learning_headers
            .get_mut(row)
            .ok_or(GpuClosedLoopError::MalformedUpload)?;
        learning.scheduled_work_checksum = header.scheduled_work_checksum(
            learning.scheduled_tile_visits,
            learning.scheduled_synapse_ops,
        );
        let learning_start = row
            .checked_mul(GPU_ACTIVE_DISPATCH_ROW_WORDS)
            .and_then(|base| base.checked_add(GPU_PERCEPTION_DISPATCH_ROW_WORDS))
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        self.dispatch_header_words[learning_start..learning_start + GPU_LEARNING_HEADER_WORDS]
            .copy_from_slice(learning.words());
        let start = row
            .checked_mul(GPU_ACTIVE_DISPATCH_ROW_WORDS)
            .and_then(|base| {
                base.checked_add(
                    GPU_PERCEPTION_DISPATCH_ROW_WORDS
                        + GPU_LEARNING_HEADER_WORDS
                        + crate::GPU_MEMORY_CONTEXT_HEADER_WORDS,
                )
            })
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        self.dispatch_header_words[start..start + crate::GPU_ACTIVITY_DISPATCH_HEADER_WORDS]
            .copy_from_slice(header.words());
        Ok(())
    }
    #[allow(dead_code)]
    pub(crate) fn dispatch_generation(&self) -> u64 {
        self.headers.first().map_or(0, |header| {
            u64::from(header.dispatch_generation_lo)
                | (u64::from(header.dispatch_generation_hi) << 32)
        })
    }
    #[cfg(test)]
    fn authority_nonce_for_test(&self) -> u64 {
        self.authority_nonce
    }
    #[cfg(any(test, feature = "gpu-tests"))]
    pub fn zero_frame_payload_for_hardware_diagnostic(&mut self) {
        self.frame_payload_words.fill(0);
    }
}

pub(crate) struct GpuPreparedActiveBatch {
    batch: GpuActiveBatchUpload,
}

pub(crate) struct GpuCompactMapTicket {
    receiver: mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>,
}

impl GpuCompactMapTicket {
    pub(crate) fn mapping_succeeded(self) -> bool {
        matches!(self.receiver.try_recv(), Ok(Ok(())))
    }
}

pub(crate) struct GpuValidatedClassBatch {
    bucket_ownership_token: u64,
    authority_nonce: u64,
    records: Vec<GpuSelectionRecord>,
    pending_records: Vec<GpuPendingEligibilityRecord>,
    final_sides: Vec<(u32, u32, u32)>,
    readback_bytes: u64,
}

impl GpuValidatedClassBatch {
    pub(crate) fn records(&self) -> &[GpuSelectionRecord] {
        &self.records
    }

    pub(crate) fn pending_records(&self) -> &[GpuPendingEligibilityRecord] {
        &self.pending_records
    }
}

pub(crate) struct GpuCommittedClassBatch {
    pub(crate) records: Vec<GpuSelectionRecord>,
    pub(crate) pending_records: Vec<GpuPendingEligibilityRecord>,
    pub(crate) readback_bytes: u64,
}

pub(crate) trait ClosedLoopBufferSet {
    fn neural_buffers(&self) -> [&wgpu::Buffer; 7];
    fn compact_readback(&self) -> &wgpu::Buffer;
    fn ownership_token(&self) -> u64;
    fn buffer_set_token(&self) -> u64;
    fn max_neurons(&self) -> u32;
    fn dispatch_capacity_words(&self) -> usize;
    fn frame_payload_capacity_words(&self) -> usize;
    fn compact_readback_capacity_bytes(&self) -> u64;
}

impl ClosedLoopBufferSet for GpuClassBucketBuffers {
    fn neural_buffers(&self) -> [&wgpu::Buffer; 7] {
        self.neural_buffers()
    }
    fn compact_readback(&self) -> &wgpu::Buffer {
        self.compact_readback()
    }
    fn ownership_token(&self) -> u64 {
        self.ownership_token()
    }
    fn buffer_set_token(&self) -> u64 {
        self.buffer_set_token()
    }
    fn max_neurons(&self) -> u32 {
        self.max_neurons()
    }
    fn dispatch_capacity_words(&self) -> usize {
        self.dispatch_capacity_words()
    }
    fn frame_payload_capacity_words(&self) -> usize {
        self.frame_payload_capacity_words()
    }
    fn compact_readback_capacity_bytes(&self) -> u64 {
        self.compact_readback_capacity_bytes()
    }
}

impl ClosedLoopBufferSet for GpuFixedClassArenaBuffers {
    fn neural_buffers(&self) -> [&wgpu::Buffer; 7] {
        self.neural_buffers()
    }
    fn compact_readback(&self) -> &wgpu::Buffer {
        self.compact_readback()
    }
    fn ownership_token(&self) -> u64 {
        self.ownership_token()
    }
    fn buffer_set_token(&self) -> u64 {
        self.buffer_set_token()
    }
    fn max_neurons(&self) -> u32 {
        self.max_neurons()
    }
    fn dispatch_capacity_words(&self) -> usize {
        self.dispatch_capacity_words()
    }
    fn frame_payload_capacity_words(&self) -> usize {
        self.frame_payload_capacity_words()
    }
    fn compact_readback_capacity_bytes(&self) -> u64 {
        self.compact_readback_capacity_bytes()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuRecurrentDispatchReceipt {
    pub max_microsteps_dispatched: u32,
    initial_activation_sides: Vec<u32>,
    row_microstep_counts: Vec<u32>,
}

impl GpuRecurrentDispatchReceipt {
    pub fn final_activation_side(&self, row: u32) -> Result<u32, GpuClosedLoopError> {
        let initial = *self
            .initial_activation_sides
            .get(row as usize)
            .ok_or(GpuClosedLoopError::MalformedUpload)?;
        let count = *self
            .row_microstep_counts
            .get(row as usize)
            .ok_or(GpuClosedLoopError::MalformedUpload)?;
        GpuClosedLoopPipelines::final_activation_side(initial, count)
    }
}

/// Device-owned immutable WGSL kernels shared by every class arena.
pub(crate) struct GpuClosedLoopKernelSet {
    bind_group_layout: wgpu::BindGroupLayout,
    encode_pipeline: wgpu::ComputePipeline,
    recurrent_pipelines: [wgpu::ComputePipeline; 4],
    clear_diagnostics_pipeline: wgpu::ComputePipeline,
    decode_pipeline: wgpu::ComputePipeline,
    memory_context_pipeline: wgpu::ComputePipeline,
    select_pipeline: wgpu::ComputePipeline,
    prevalidate_eligibility_pipeline: wgpu::ComputePipeline,
    recurrent_eligibility_pipeline: wgpu::ComputePipeline,
    decoder_eligibility_pipeline: wgpu::ComputePipeline,
    finalize_pending_eligibility_pipeline: wgpu::ComputePipeline,
    discard_pending_eligibility_arrays_pipeline: wgpu::ComputePipeline,
    finalize_discard_pending_eligibility_pipeline: wgpu::ComputePipeline,
    initialize_fast_plasticity_pipeline: wgpu::ComputePipeline,
    apply_fast_plasticity_pipeline: wgpu::ComputePipeline,
    capture_fast_plasticity_replay_pipeline: wgpu::ComputePipeline,
    finalize_fast_plasticity_pipeline: wgpu::ComputePipeline,
    initialize_sleep_transaction_pipeline: wgpu::ComputePipeline,
    copy_sleep_weight_banks_pipeline: wgpu::ComputePipeline,
    replay_sleep_learning_pipeline: wgpu::ComputePipeline,
    consolidate_fast_weights_pipeline: wgpu::ComputePipeline,
    finalize_sleep_staging_pipeline: wgpu::ComputePipeline,
    reset_sleep_mutable_state_pipeline: wgpu::ComputePipeline,
    finalize_sleep_commit_pipeline: wgpu::ComputePipeline,
}

impl GpuClosedLoopKernelSet {
    pub(crate) fn new(device: &wgpu::Device) -> Result<Arc<Self>, GpuClosedLoopError> {
        for (source, entries) in [
            (CLOSED_LOOP_ENCODE_WGSL, &["encode_perception"][..]),
            (CLOSED_LOOP_RECURRENT_WGSL, &["recurrent_microstep"][..]),
            (
                CLOSED_LOOP_CLEAR_DIAGNOSTICS_WGSL,
                &["clear_diagnostics"][..],
            ),
            (
                CLOSED_LOOP_DECODE_WGSL,
                &["decode_candidates", "select_candidate"][..],
            ),
            (
                CLOSED_LOOP_MEMORY_CONTEXT_WGSL,
                &["add_candidate_memory_context"][..],
            ),
            (
                CLOSED_LOOP_ELIGIBILITY_WGSL,
                &[
                    "prevalidate_eligibility",
                    "accumulate_recurrent_eligibility",
                    "accumulate_decoder_eligibility",
                    "finalize_pending_eligibility",
                    "discard_pending_eligibility_arrays",
                    "finalize_discard_pending_eligibility",
                ][..],
            ),
            (
                CLOSED_LOOP_PLASTICITY_WGSL,
                &[
                    "initialize_fast_plasticity",
                    "apply_fast_plasticity",
                    "capture_fast_plasticity_replay",
                    "finalize_fast_plasticity",
                ][..],
            ),
            (
                CLOSED_LOOP_CONSOLIDATE_WGSL,
                &[
                    "initialize_sleep_transaction",
                    "copy_sleep_weight_banks",
                    "consolidate_fast_weights",
                    "finalize_sleep_staging",
                    "reset_sleep_mutable_state",
                    "finalize_sleep_commit",
                ][..],
            ),
            (
                CLOSED_LOOP_REPLAY_LEARNING_WGSL,
                &["replay_sleep_learning"][..],
            ),
        ] {
            validate_production_shader_contract(source, entries)?;
        }
        let layout = create_neural_bind_group_layout(device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("closed-loop-neural-pipeline-layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let encode_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("closed-loop-encode-wgsl"),
            source: wgpu::ShaderSource::Wgsl(CLOSED_LOOP_ENCODE_WGSL.into()),
        });
        let recurrent_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("closed-loop-recurrent-wgsl"),
            source: wgpu::ShaderSource::Wgsl(CLOSED_LOOP_RECURRENT_WGSL.into()),
        });
        let clear_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("closed-loop-clear-diagnostics-wgsl"),
            source: wgpu::ShaderSource::Wgsl(CLOSED_LOOP_CLEAR_DIAGNOSTICS_WGSL.into()),
        });
        let decode_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("closed-loop-decode-wgsl"),
            source: wgpu::ShaderSource::Wgsl(CLOSED_LOOP_DECODE_WGSL.into()),
        });
        let memory_context_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("closed-loop-memory-context-wgsl"),
            source: wgpu::ShaderSource::Wgsl(CLOSED_LOOP_MEMORY_CONTEXT_WGSL.into()),
        });
        let eligibility_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("closed-loop-eligibility-wgsl"),
            source: wgpu::ShaderSource::Wgsl(CLOSED_LOOP_ELIGIBILITY_WGSL.into()),
        });
        let plasticity_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("closed-loop-plasticity-wgsl"),
            source: wgpu::ShaderSource::Wgsl(CLOSED_LOOP_PLASTICITY_WGSL.into()),
        });
        let consolidate_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("closed-loop-consolidate-wgsl"),
            source: wgpu::ShaderSource::Wgsl(CLOSED_LOOP_CONSOLIDATE_WGSL.into()),
        });
        let replay_sleep_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("closed-loop-replay-learning-wgsl"),
            source: wgpu::ShaderSource::Wgsl(CLOSED_LOOP_REPLAY_LEARNING_WGSL.into()),
        });
        let encode_pipeline = create_compute_pipeline(
            device,
            &pipeline_layout,
            &encode_shader,
            "encode_perception",
            &[],
        );
        let recurrent_pipelines = [0_u32, 1, 2, 3].map(|step| {
            create_compute_pipeline(
                device,
                &pipeline_layout,
                &recurrent_shader,
                "recurrent_microstep",
                &[("microstep_index", step as f64)],
            )
        });
        let clear_diagnostics_pipeline = create_compute_pipeline(
            device,
            &pipeline_layout,
            &clear_shader,
            "clear_diagnostics",
            &[],
        );
        let decode_pipeline = create_compute_pipeline(
            device,
            &pipeline_layout,
            &decode_shader,
            "decode_candidates",
            &[],
        );
        let memory_context_pipeline = create_compute_pipeline(
            device,
            &pipeline_layout,
            &memory_context_shader,
            "add_candidate_memory_context",
            &[],
        );
        let select_pipeline = create_compute_pipeline(
            device,
            &pipeline_layout,
            &decode_shader,
            "select_candidate",
            &[],
        );
        let prevalidate_eligibility_pipeline = create_compute_pipeline(
            device,
            &pipeline_layout,
            &eligibility_shader,
            "prevalidate_eligibility",
            &[],
        );
        let recurrent_eligibility_pipeline = create_compute_pipeline(
            device,
            &pipeline_layout,
            &eligibility_shader,
            "accumulate_recurrent_eligibility",
            &[],
        );
        let decoder_eligibility_pipeline = create_compute_pipeline(
            device,
            &pipeline_layout,
            &eligibility_shader,
            "accumulate_decoder_eligibility",
            &[],
        );
        let finalize_pending_eligibility_pipeline = create_compute_pipeline(
            device,
            &pipeline_layout,
            &eligibility_shader,
            "finalize_pending_eligibility",
            &[],
        );
        let discard_pending_eligibility_arrays_pipeline = create_compute_pipeline(
            device,
            &pipeline_layout,
            &eligibility_shader,
            "discard_pending_eligibility_arrays",
            &[],
        );
        let finalize_discard_pending_eligibility_pipeline = create_compute_pipeline(
            device,
            &pipeline_layout,
            &eligibility_shader,
            "finalize_discard_pending_eligibility",
            &[],
        );
        let initialize_fast_plasticity_pipeline = create_compute_pipeline(
            device,
            &pipeline_layout,
            &plasticity_shader,
            "initialize_fast_plasticity",
            &[],
        );
        let apply_fast_plasticity_pipeline = create_compute_pipeline(
            device,
            &pipeline_layout,
            &plasticity_shader,
            "apply_fast_plasticity",
            &[],
        );
        let capture_fast_plasticity_replay_pipeline = create_compute_pipeline(
            device,
            &pipeline_layout,
            &plasticity_shader,
            "capture_fast_plasticity_replay",
            &[],
        );
        let finalize_fast_plasticity_pipeline = create_compute_pipeline(
            device,
            &pipeline_layout,
            &plasticity_shader,
            "finalize_fast_plasticity",
            &[],
        );
        let initialize_sleep_transaction_pipeline = create_compute_pipeline(
            device,
            &pipeline_layout,
            &consolidate_shader,
            "initialize_sleep_transaction",
            &[],
        );
        let copy_sleep_weight_banks_pipeline = create_compute_pipeline(
            device,
            &pipeline_layout,
            &consolidate_shader,
            "copy_sleep_weight_banks",
            &[],
        );
        let replay_sleep_learning_pipeline = create_compute_pipeline(
            device,
            &pipeline_layout,
            &replay_sleep_shader,
            "replay_sleep_learning",
            &[],
        );
        let consolidate_fast_weights_pipeline = create_compute_pipeline(
            device,
            &pipeline_layout,
            &consolidate_shader,
            "consolidate_fast_weights",
            &[],
        );
        let finalize_sleep_staging_pipeline = create_compute_pipeline(
            device,
            &pipeline_layout,
            &consolidate_shader,
            "finalize_sleep_staging",
            &[],
        );
        let reset_sleep_mutable_state_pipeline = create_compute_pipeline(
            device,
            &pipeline_layout,
            &consolidate_shader,
            "reset_sleep_mutable_state",
            &[],
        );
        let finalize_sleep_commit_pipeline = create_compute_pipeline(
            device,
            &pipeline_layout,
            &consolidate_shader,
            "finalize_sleep_commit",
            &[],
        );
        Ok(Arc::new(Self {
            bind_group_layout: layout,
            encode_pipeline,
            recurrent_pipelines,
            clear_diagnostics_pipeline,
            decode_pipeline,
            memory_context_pipeline,
            select_pipeline,
            prevalidate_eligibility_pipeline,
            recurrent_eligibility_pipeline,
            decoder_eligibility_pipeline,
            finalize_pending_eligibility_pipeline,
            discard_pending_eligibility_arrays_pipeline,
            finalize_discard_pending_eligibility_pipeline,
            initialize_fast_plasticity_pipeline,
            apply_fast_plasticity_pipeline,
            capture_fast_plasticity_replay_pipeline,
            finalize_fast_plasticity_pipeline,
            initialize_sleep_transaction_pipeline,
            copy_sleep_weight_banks_pipeline,
            replay_sleep_learning_pipeline,
            consolidate_fast_weights_pipeline,
            finalize_sleep_staging_pipeline,
            reset_sleep_mutable_state_pipeline,
            finalize_sleep_commit_pipeline,
        }))
    }
}

pub struct GpuClosedLoopPipelines {
    kernels: Arc<GpuClosedLoopKernelSet>,
    bind_group: wgpu::BindGroup,
    bucket_ownership_token: u64,
    buffer_set_token: u64,
    max_neurons: u32,
    max_compute_workgroups_per_dimension: u32,
    authority: BatchAuthority,
    dispatch_capacity_words: usize,
    frame_payload_capacity_words: usize,
    next_authority_nonce: u64,
    #[cfg(feature = "gpu-tests")]
    force_all_invalid_slot: Option<(u32, u32)>,
    #[cfg(feature = "gpu-tests")]
    force_pending_identity_mismatch_slot: Option<(u32, u32)>,
}

impl GpuClosedLoopPipelines {
    /// Generation that the next successfully begun active batch must bind.
    pub const fn next_dispatch_generation(&self) -> u64 {
        self.next_authority_nonce
    }

    pub fn new(
        device: &wgpu::Device,
        buffers: &GpuClassBucketBuffers,
    ) -> Result<Self, GpuClosedLoopError> {
        let kernels = GpuClosedLoopKernelSet::new(device)?;
        Self::from_shared_kernel_set(device, buffers, kernels)
    }

    pub(crate) fn from_shared_kernel_set(
        device: &wgpu::Device,
        buffers: &GpuClassBucketBuffers,
        kernels: Arc<GpuClosedLoopKernelSet>,
    ) -> Result<Self, GpuClosedLoopError> {
        Self::from_buffer_set(device, buffers, kernels)
    }

    pub(crate) fn from_shared_kernel_set_for_fixed_arena(
        device: &wgpu::Device,
        buffers: &GpuFixedClassArenaBuffers,
        kernels: Arc<GpuClosedLoopKernelSet>,
    ) -> Result<Self, GpuClosedLoopError> {
        Self::from_buffer_set(device, buffers, kernels)
    }

    fn from_buffer_set(
        device: &wgpu::Device,
        buffers: &impl ClosedLoopBufferSet,
        kernels: Arc<GpuClosedLoopKernelSet>,
    ) -> Result<Self, GpuClosedLoopError> {
        let neural = buffers.neural_buffers();
        let entries = std::array::from_fn::<_, 7, _>(|index| wgpu::BindGroupEntry {
            binding: index as u32,
            resource: neural[index].as_entire_binding(),
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("closed-loop-neural-bind-group"),
            layout: &kernels.bind_group_layout,
            entries: &entries,
        });
        Ok(Self {
            kernels,
            bind_group,
            bucket_ownership_token: buffers.ownership_token(),
            buffer_set_token: buffers.buffer_set_token(),
            max_neurons: buffers.max_neurons(),
            max_compute_workgroups_per_dimension: device
                .limits()
                .max_compute_workgroups_per_dimension,
            authority: BatchAuthority::default(),
            dispatch_capacity_words: buffers.dispatch_capacity_words(),
            frame_payload_capacity_words: buffers.frame_payload_capacity_words(),
            next_authority_nonce: 1,
            #[cfg(feature = "gpu-tests")]
            force_all_invalid_slot: None,
            #[cfg(feature = "gpu-tests")]
            force_pending_identity_mismatch_slot: None,
        })
    }

    pub(crate) fn dispatch_sleep_staging(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        buffers: &GpuFixedClassArenaBuffers,
        header: &crate::GpuSleepHeader,
        payload_words: &[u32],
    ) -> Result<(), GpuClosedLoopError> {
        self.validate_sleep_dispatch(buffers, header, payload_words)?;
        queue.write_buffer(buffers.neural_buffers()[4], 0, bytemuck::bytes_of(header));
        queue.write_buffer(
            buffers.neural_buffers()[5],
            0,
            bytemuck::cast_slice(payload_words),
        );
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("closed-loop-sleep-staging"),
        });
        self.record_sleep_pass(
            &mut encoder,
            &self.kernels.initialize_sleep_transaction_pipeline,
            1,
        );
        self.record_sleep_pass(
            &mut encoder,
            &self.kernels.copy_sleep_weight_banks_pipeline,
            header.synapse_count.div_ceil(WORKGROUP_SIZE),
        );
        self.record_sleep_pass(
            &mut encoder,
            &self.kernels.replay_sleep_learning_pipeline,
            header.replay_span_count.div_ceil(WORKGROUP_SIZE).max(1),
        );
        self.record_sleep_pass(
            &mut encoder,
            &self.kernels.consolidate_fast_weights_pipeline,
            header.synapse_count.div_ceil(WORKGROUP_SIZE),
        );
        self.record_sleep_pass(
            &mut encoder,
            &self.kernels.finalize_sleep_staging_pipeline,
            1,
        );
        let submission = queue.submit(Some(encoder.finish()));
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .map_err(|_| GpuClosedLoopError::SubmissionFailed)?;
        Ok(())
    }

    pub(crate) fn dispatch_sleep_commit(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        buffers: &GpuFixedClassArenaBuffers,
        header: &crate::GpuSleepHeader,
        payload_words: &[u32],
        reset_word_count: u32,
    ) -> Result<(), GpuClosedLoopError> {
        self.validate_sleep_dispatch(buffers, header, payload_words)?;
        if reset_word_count == 0 {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        queue.write_buffer(buffers.neural_buffers()[4], 0, bytemuck::bytes_of(header));
        queue.write_buffer(
            buffers.neural_buffers()[5],
            0,
            bytemuck::cast_slice(payload_words),
        );
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("closed-loop-sleep-commit"),
        });
        self.record_sleep_pass(
            &mut encoder,
            &self.kernels.reset_sleep_mutable_state_pipeline,
            reset_word_count.div_ceil(WORKGROUP_SIZE),
        );
        self.record_sleep_pass(
            &mut encoder,
            &self.kernels.finalize_sleep_commit_pipeline,
            1,
        );
        let submission = queue.submit(Some(encoder.finish()));
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .map_err(|_| GpuClosedLoopError::SubmissionFailed)?;
        Ok(())
    }

    fn validate_sleep_dispatch(
        &self,
        buffers: &GpuFixedClassArenaBuffers,
        header: &crate::GpuSleepHeader,
        payload_words: &[u32],
    ) -> Result<(), GpuClosedLoopError> {
        self.authority.ensure_healthy()?;
        if self.authority.pending.is_some()
            || buffers.ownership_token() != self.bucket_ownership_token
            || buffers.buffer_set_token() != self.buffer_set_token
            || header.schema_version != 1
            || header.flags != 0
            || header.reserved != 0
            || header.synapse_count == 0
            || header.replay_span_count == 0
            || std::mem::size_of::<crate::GpuSleepHeader>() / 4 > self.dispatch_capacity_words
            || payload_words.is_empty()
            || payload_words.len() > self.frame_payload_capacity_words
            || header.brain_slot_index != header.slot
        {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        Ok(())
    }

    fn record_sleep_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        pipeline: &wgpu::ComputePipeline,
        x_groups: u32,
    ) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("closed-loop-sleep-pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.dispatch_workgroups(x_groups.max(1), 1, 1);
    }

    pub fn build_active_batch(
        &mut self,
        plan: &crate::GpuClassBucketPlan,
        entries: &[GpuActiveBatchEntry<'_>],
        frame_base_words: u32,
    ) -> Result<GpuActiveBatchUpload, GpuClosedLoopError> {
        let dispatch_generation = NonZeroU64::new(self.next_authority_nonce)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let prepared =
            self.preflight_active_batch(plan, entries, frame_base_words, dispatch_generation)?;
        self.begin_prepared_batch(prepared)
    }

    /// Performs the complete class-local host preflight without reserving a
    /// private nonce or mutating persistent active-side authority.
    pub(crate) fn preflight_active_batch(
        &self,
        plan: &crate::GpuClassBucketPlan,
        entries: &[GpuActiveBatchEntry<'_>],
        frame_base_words: u32,
        dispatch_generation: NonZeroU64,
    ) -> Result<GpuPreparedActiveBatch, GpuClosedLoopError> {
        self.authority.ensure_healthy()?;
        if self.authority.pending.is_some() {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        if plan.ownership_token() != self.bucket_ownership_token
            || plan.capacity().execution().max_neurons() != self.max_neurons
        {
            return Err(GpuClosedLoopError::StaleOrForeignHandle);
        }
        for entry in entries {
            plan.validate_slot_handle(entry.slot)?;
            let record = entry.slot.record();
            if record.neuron_count != self.max_neurons
                || record.microstep_count < 2
                || record.microstep_count > 4
            {
                return Err(GpuClosedLoopError::MalformedUpload);
            }
        }
        let views = entries
            .iter()
            .map(|entry| GpuBatchEntryView {
                frame: entry.frame,
                slot: entry.slot,
                phenotype: entry.phenotype,
                activity: entry.activity,
                memory_upload: entry.memory_upload,
                active_eligibility_generation: entry.active_eligibility_generation,
            })
            .collect::<Vec<_>>();
        let batch = GpuActiveBatchUpload::try_from_views(
            &views,
            frame_base_words,
            self.bucket_ownership_token,
            &self.authority.active_sides,
            self.dispatch_capacity_words,
            self.frame_payload_capacity_words,
            dispatch_generation,
            0,
        )?;
        Ok(GpuPreparedActiveBatch { batch })
    }

    pub(crate) fn preflight_fixed_active_batch(
        &self,
        entries: &[GpuFixedActiveBatchEntry<'_>],
        frame_base_words: u32,
        dispatch_generation: NonZeroU64,
    ) -> Result<GpuPreparedActiveBatch, GpuClosedLoopError> {
        self.authority.ensure_healthy()?;
        if self.authority.pending.is_some() || entries.is_empty() {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        let views = entries
            .iter()
            .map(|entry| GpuBatchEntryView {
                frame: entry.frame,
                slot: entry.slot,
                phenotype: entry.phenotype,
                activity: entry.activity,
                memory_upload: entry.memory_upload,
                active_eligibility_generation: entry.active_eligibility_generation,
            })
            .collect::<Vec<_>>();
        if views.iter().any(|entry| {
            entry.slot.record().slot != entry.slot.brain_slot_index()
                || entry.slot.record().neuron_count != self.max_neurons
                || !(2..=4).contains(&entry.slot.record().microstep_count)
        }) {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        let batch = GpuActiveBatchUpload::try_from_views(
            &views,
            frame_base_words,
            self.bucket_ownership_token,
            &self.authority.active_sides,
            self.dispatch_capacity_words,
            self.frame_payload_capacity_words,
            dispatch_generation,
            0,
        )?;
        Ok(GpuPreparedActiveBatch { batch })
    }

    /// Reserves the class-private authority nonce only after every class in a
    /// backend-global transaction has passed preflight.
    pub(crate) fn begin_prepared_batch(
        &mut self,
        mut prepared: GpuPreparedActiveBatch,
    ) -> Result<GpuActiveBatchUpload, GpuClosedLoopError> {
        self.authority.ensure_healthy()?;
        if self.authority.pending.is_some()
            || prepared.batch.bucket_ownership_token != self.bucket_ownership_token
        {
            return Err(GpuClosedLoopError::StaleOrForeignHandle);
        }
        for header in &prepared.batch.headers {
            let side = self
                .authority
                .active_sides
                .get(&(header.brain_slot_index, header.slot_generation))
                .copied()
                .unwrap_or(0);
            if side != header.active_activation_side {
                return Err(GpuClosedLoopError::StaleOrForeignHandle);
            }
        }
        let authority_nonce = self.next_authority_nonce;
        let next_authority_nonce = authority_nonce
            .checked_add(1)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        self.authority.begin(authority_nonce)?;
        for header in &prepared.batch.headers {
            self.authority
                .active_sides
                .entry((header.brain_slot_index, header.slot_generation))
                .or_insert(0);
        }
        prepared.batch.authority_nonce = authority_nonce;
        self.next_authority_nonce = next_authority_nonce;
        Ok(prepared.batch)
    }

    /// Explicitly releases a built batch that will not be submitted. The
    /// opaque batch is consumed, persistent activation sides are unchanged,
    /// and a later batch receives a fresh nonce.
    pub fn abandon_unsubmitted_batch(
        &mut self,
        batch: GpuActiveBatchUpload,
    ) -> Result<(), GpuClosedLoopError> {
        if batch.bucket_ownership_token != self.bucket_ownership_token {
            return Err(GpuClosedLoopError::StaleOrForeignHandle);
        }
        self.authority.abandon_unsubmitted(batch.authority_nonce)
    }

    pub(crate) fn rollback_recorded_batch(
        &mut self,
        batch: &GpuActiveBatchUpload,
    ) -> Result<(), GpuClosedLoopError> {
        self.validate_batch_identity(batch)?;
        self.authority.recording_failed(batch.authority_nonce)
    }

    pub(crate) fn mark_post_submit_poison(
        &mut self,
        batch: &GpuActiveBatchUpload,
    ) -> Result<(), GpuClosedLoopError> {
        self.validate_batch_identity(batch)?;
        self.authority
            .submission_indeterminate(batch.authority_nonce)
    }

    pub(crate) fn retire_slot_active_side(
        &mut self,
        slot: u32,
        generation: u32,
    ) -> Result<(), GpuClosedLoopError> {
        self.authority.retire_active_side(slot, generation)
    }

    pub(crate) fn slot_active_side(
        &self,
        slot: u32,
        generation: u32,
    ) -> Result<u8, GpuClosedLoopError> {
        self.authority.ensure_healthy()?;
        if self.authority.pending.is_some() {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        u8::try_from(
            self.authority
                .active_sides
                .get(&(slot, generation))
                .copied()
                .unwrap_or(0),
        )
        .ok()
        .filter(|side| *side <= 1)
        .ok_or(GpuClosedLoopError::MalformedUpload)
    }

    pub(crate) fn restore_slot_active_side(
        &mut self,
        slot: u32,
        generation: u32,
        side: u8,
    ) -> Result<(), GpuClosedLoopError> {
        self.authority.ensure_healthy()?;
        if self.authority.pending.is_some() || generation == 0 || side > 1 {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        self.authority
            .active_sides
            .insert((slot, generation), u32::from(side));
        Ok(())
    }

    #[cfg(feature = "gpu-tests")]
    pub fn reset_active_sides_for_hardware_diagnostic(&mut self) -> Result<(), GpuClosedLoopError> {
        self.authority.ensure_healthy()?;
        if self.authority.pending.is_some() {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        self.authority.active_sides.clear();
        Ok(())
    }

    pub const fn recurrent_variant_count() -> usize {
        4
    }

    pub const fn recurrent_variant_microstep_indices() -> [u32; 4] {
        [0, 1, 2, 3]
    }

    pub fn validate_microstep_count(microsteps: u32) -> Result<(), GpuClosedLoopError> {
        if (2..=4).contains(&microsteps) {
            Ok(())
        } else {
            Err(GpuClosedLoopError::MalformedUpload)
        }
    }

    pub fn final_activation_side(
        initial_side: u32,
        microsteps: u32,
    ) -> Result<u32, GpuClosedLoopError> {
        Self::validate_microstep_count(microsteps)?;
        if initial_side > 1 {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        Ok(initial_side ^ (microsteps & 1))
    }

    pub(crate) fn write_staged_uploads(
        &self,
        queue: &wgpu::Queue,
        buffers: &impl ClosedLoopBufferSet,
        batch: &GpuActiveBatchUpload,
    ) -> Result<(), GpuClosedLoopError> {
        self.validate_buffers_and_dispatch(buffers, batch)?;
        let neural = buffers.neural_buffers();
        queue.write_buffer(
            neural[4],
            0,
            bytemuck::cast_slice(batch.dispatch_header_words()),
        );
        queue.write_buffer(
            neural[5],
            0,
            bytemuck::cast_slice(batch.frame_payload_words()),
        );
        Ok(())
    }

    pub(crate) fn apply_fast_plasticity(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        buffers: &GpuFixedClassArenaBuffers,
        entries: &[GpuFastPlasticityBatchEntry<'_>],
        timestamp: GpuTimestampQueryResources<'_>,
    ) -> Result<GpuTimedFastPlasticityResult, GpuClosedLoopError> {
        self.authority.ensure_healthy()?;
        if self.authority.pending.is_some()
            || entries.is_empty()
            || buffers.ownership_token() != self.bucket_ownership_token
            || buffers.buffer_set_token() != self.buffer_set_token
        {
            return Err(GpuClosedLoopError::StaleOrForeignHandle);
        }
        let rows =
            u32::try_from(entries.len()).map_err(|_| GpuClosedLoopError::CapacityExceeded)?;
        if rows == 0 || rows > self.max_compute_workgroups_per_dimension {
            return Err(GpuClosedLoopError::CapacityExceeded);
        }
        let mut seen = BTreeSet::new();
        let mut dispatch_words = vec![0_u32; entries.len() * GPU_ACTIVE_DISPATCH_ROW_WORDS];
        let mut frame_words = Vec::with_capacity(entries.len() * GPU_OUTCOME_CREDIT_WORDS);
        let mut max_synapse_count = 0_u32;
        let mut max_replay_span_count = 0_u32;
        for (row, entry) in entries.iter().enumerate() {
            let record = entry.slot.record();
            let pending = entry.pending;
            let outcome = entry.outcome;
            if entry.slot.brain_slot_index() != record.slot
                || !seen.insert((record.slot, record.slot_generation))
                || record.slot != pending.slot
                || record.slot_generation != pending.slot_generation
                || record.schema_version != crate::GPU_CLOSED_LOOP_LAYOUT_VERSION
                || record.selection_offset != record.diagnostic_offset + 4
                || record.synapse_count == 0
                || record.recurrent_synapse_count == 0
                || record.recurrent_synapse_count >= record.synapse_count
                || outcome.schema_version != u32::from(SchemaVersions::CURRENT.learning.raw())
                || outcome.active_activation_side > 1
                || outcome.active_activation_side != pending.active_activation_side
                || outcome.organism_id != pending.organism_id
                || outcome.phenotype_hash != pending.phenotype_hash
                || outcome.dispatch_generation != pending.dispatch_generation
                || outcome.originating_tick != pending.originating_tick
                || outcome.frame_digest != pending.frame_digest
                || outcome.selected_candidate_and_family != pending.candidate_index_and_family
                || outcome.selected_action != pending.action_id
                || outcome.candidate_feature_digest != pending.candidate_feature_digest
                || pending.schema_version != u32::from(SchemaVersions::CURRENT.learning.raw())
                || (u64::from(pending.staging_eligibility_generation[0])
                    | (u64::from(pending.staging_eligibility_generation[1]) << 32))
                    != (u64::from(pending.active_eligibility_generation[0])
                        | (u64::from(pending.active_eligibility_generation[1]) << 32))
                        .checked_add(1)
                        .ok_or(GpuClosedLoopError::ArithmeticOverflow)?
                || entry.active_weight_generation == 0
                || entry.replay_generation == 0
                || entry.transaction_generation == 0
            {
                return Err(GpuClosedLoopError::MalformedUpload);
            }
            let decoder_synapse_count = record
                .synapse_count
                .checked_sub(record.recurrent_synapse_count)
                .ok_or(GpuClosedLoopError::MalformedUpload)?;
            let replay_span_words = entry
                .slot
                .word_ranges()
                .replay_span_words
                .end
                .checked_sub(entry.slot.word_ranges().replay_span_words.start)
                .ok_or(GpuClosedLoopError::MalformedUpload)?;
            if replay_span_words == 0 || replay_span_words % 4 != 0 {
                return Err(GpuClosedLoopError::MalformedUpload);
            }
            let replay_span_count = replay_span_words / 4;
            max_synapse_count = max_synapse_count.max(record.synapse_count);
            max_replay_span_count = max_replay_span_count.max(replay_span_count);
            let outcome_offset = u32::try_from(row)
                .map_err(|_| GpuClosedLoopError::CapacityExceeded)?
                .checked_mul(GPU_OUTCOME_CREDIT_WORDS as u32)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
            let header = GpuLearningHeader {
                schema_version: u32::from(SchemaVersions::CURRENT.learning.raw()),
                class_id: record.class_id,
                slot: record.slot,
                slot_generation: record.slot_generation,
                brain_slot_index: entry.slot.brain_slot_index(),
                active_activation_side: outcome.active_activation_side,
                dispatch_generation_lo: outcome.dispatch_generation[0],
                dispatch_generation_hi: outcome.dispatch_generation[1],
                candidate_count: 0,
                candidate_offset: 0,
                decoder_learning_input_offset: 0,
                selection_offset: record.diagnostic_offset,
                outcome_offset,
                recurrent_synapse_count: record.recurrent_synapse_count,
                decoder_synapse_count,
                decoder_input_stride: 0,
                pending_eligibility_offset: entry
                    .slot
                    .word_ranges()
                    .pending_eligibility_words
                    .start,
                scheduled_tile_visits: 0,
                scheduled_synapse_ops: 0,
                scheduled_work_checksum: 0,
            };
            let dispatch_base =
                row * GPU_ACTIVE_DISPATCH_ROW_WORDS + GPU_PERCEPTION_DISPATCH_ROW_WORDS;
            dispatch_words[dispatch_base..dispatch_base + GPU_LEARNING_HEADER_WORDS]
                .copy_from_slice(header.words());
            frame_words.extend_from_slice(outcome.words());
        }
        let readback_bytes = entries
            .len()
            .checked_mul(GPU_FAST_PLASTICITY_COMMIT_BYTES)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        if dispatch_words.len() > buffers.dispatch_capacity_words()
            || frame_words.len() > buffers.frame_payload_capacity_words()
            || readback_bytes as u64 > buffers.compact_readback_capacity_bytes()
        {
            return Err(GpuClosedLoopError::CapacityExceeded);
        }
        let synapse_groups = max_synapse_count.div_ceil(WORKGROUP_SIZE);
        let replay_groups = max_replay_span_count.div_ceil(WORKGROUP_SIZE);
        if synapse_groups == 0
            || replay_groups == 0
            || synapse_groups > self.max_compute_workgroups_per_dimension
            || replay_groups > self.max_compute_workgroups_per_dimension
        {
            return Err(GpuClosedLoopError::CapacityExceeded);
        }
        let neural = buffers.neural_buffers();
        queue.write_buffer(neural[4], 0, bytemuck::cast_slice(&dispatch_words));
        queue.write_buffer(neural[5], 0, bytemuck::cast_slice(&frame_words));
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("closed-loop-fast-plasticity-batch"),
        });
        {
            let _timestamp_start = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("closed-loop-fast-plasticity-timestamp-start"),
                timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                    query_set: timestamp.query_set,
                    beginning_of_pass_write_index: Some(0),
                    end_of_pass_write_index: None,
                }),
            });
        }
        for (label, pipeline, groups) in [
            (
                "closed-loop-initialize-fast-plasticity-pass",
                &self.kernels.initialize_fast_plasticity_pipeline,
                1,
            ),
            (
                "closed-loop-apply-fast-plasticity-pass",
                &self.kernels.apply_fast_plasticity_pipeline,
                synapse_groups,
            ),
            (
                "closed-loop-capture-fast-plasticity-replay-pass",
                &self.kernels.capture_fast_plasticity_replay_pipeline,
                replay_groups,
            ),
            (
                "closed-loop-finalize-fast-plasticity-pass",
                &self.kernels.finalize_fast_plasticity_pipeline,
                1,
            ),
        ] {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(label),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.dispatch_workgroups(groups, rows, 1);
        }
        {
            let _timestamp_end = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("closed-loop-fast-plasticity-timestamp-end"),
                timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                    query_set: timestamp.query_set,
                    beginning_of_pass_write_index: None,
                    end_of_pass_write_index: Some(1),
                }),
            });
        }
        encoder.resolve_query_set(timestamp.query_set, 0..2, timestamp.resolve_buffer, 0);
        encoder.copy_buffer_to_buffer(
            timestamp.resolve_buffer,
            0,
            timestamp.readback_buffer,
            0,
            16,
        );
        for (row, entry) in entries.iter().enumerate() {
            encoder.copy_buffer_to_buffer(
                neural[6],
                u64::from(entry.slot.record().diagnostic_offset) * 4,
                buffers.compact_readback(),
                (row * GPU_FAST_PLASTICITY_COMMIT_BYTES) as u64,
                GPU_FAST_PLASTICITY_COMMIT_BYTES as u64,
            );
        }
        let command_buffer = encoder.finish();
        let (sender, receiver) = mpsc::channel();
        command_buffer.map_buffer_on_submit(
            buffers.compact_readback(),
            wgpu::MapMode::Read,
            0..readback_bytes as u64,
            move |result| {
                let _ = sender.send(result);
            },
        );
        let (timestamp_sender, timestamp_receiver) = mpsc::channel();
        command_buffer.map_buffer_on_submit(
            timestamp.readback_buffer,
            wgpu::MapMode::Read,
            0..16,
            move |result| {
                let _ = timestamp_sender.send(result);
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
            || timestamp_receiver
                .recv()
                .ok()
                .and_then(Result::ok)
                .is_none()
        {
            buffers.compact_readback().unmap();
            timestamp.readback_buffer.unmap();
            return Err(GpuClosedLoopError::SubmissionFailed);
        }
        let timestamp_mapped = timestamp.readback_buffer.slice(..16).get_mapped_range();
        let timestamp_begin = u64::from_le_bytes(
            timestamp_mapped[0..8]
                .try_into()
                .map_err(|_| GpuClosedLoopError::SubmissionFailed)?,
        );
        let timestamp_end = u64::from_le_bytes(
            timestamp_mapped[8..16]
                .try_into()
                .map_err(|_| GpuClosedLoopError::SubmissionFailed)?,
        );
        drop(timestamp_mapped);
        timestamp.readback_buffer.unmap();
        let timestamp_delta_ticks = timestamp_end
            .checked_sub(timestamp_begin)
            .filter(|ticks| *ticks != 0)
            .ok_or(GpuClosedLoopError::SubmissionFailed)?;
        let mapped = buffers
            .compact_readback()
            .slice(..readback_bytes as u64)
            .get_mapped_range();
        let words = bytemuck::cast_slice::<u8, u32>(&mapped).to_vec();
        drop(mapped);
        buffers.compact_readback().unmap();
        if words.len() != entries.len() * GPU_FAST_PLASTICITY_COMMIT_WORDS {
            return Err(GpuClosedLoopError::SubmissionFailed);
        }
        let mut records = Vec::with_capacity(entries.len());
        for (row, entry) in words
            .chunks_exact(GPU_FAST_PLASTICITY_COMMIT_WORDS)
            .zip(entries)
        {
            let record = GpuFastPlasticityCommitRecord::from_words(row)?;
            let expected_fast = entry
                .active_weight_generation
                .checked_add(1)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
            let expected_replay = entry
                .replay_generation
                .checked_add(1)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
            let expected_transaction = entry
                .transaction_generation
                .checked_add(1)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
            let expected_eligibility = u64::from(entry.pending.staging_eligibility_generation[0])
                | (u64::from(entry.pending.staging_eligibility_generation[1]) << 32);
            let max_abs_delta = record.max_abs_delta();
            if record.schema_version != u32::from(SchemaVersions::CURRENT.learning.raw())
                || record.slot != entry.slot.record().slot
                || record.slot_generation != entry.slot.record().slot_generation
                || record.status != 1
                || record.input_fast_generation() != entry.active_weight_generation
                || record.output_fast_generation() != expected_fast
                || record.output_eligibility_generation() != expected_eligibility
                || record.replay_generation() != expected_replay
                || record.transaction_generation() != expected_transaction
                || record.fast_weights_changed > entry.slot.record().synapse_count
                || !max_abs_delta.is_finite()
                || max_abs_delta < 0.0
                || (record.fast_weights_changed == 0 && max_abs_delta != 0.0)
                || (record.fast_weights_changed > 0 && max_abs_delta <= 0.0)
            {
                return Err(GpuClosedLoopError::MalformedUpload);
            }
            records.push(record);
        }
        Ok(GpuTimedFastPlasticityResult {
            records,
            timestamp_delta_ticks,
        })
    }

    pub(crate) fn discard_pending_eligibility(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        buffers: &GpuFixedClassArenaBuffers,
        slot: &GpuBrainSlot,
        pending: &GpuPendingEligibilityRecord,
        expected_transaction_generation: u64,
    ) -> Result<GpuEligibilityDiscardRecord, GpuClosedLoopError> {
        self.discard_pending_eligibility_impl(
            device,
            queue,
            buffers,
            slot,
            pending,
            expected_transaction_generation,
        )
    }

    #[cfg(feature = "gpu-tests")]
    pub fn discard_pending_eligibility_for_hardware_diagnostic(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        buffers: &GpuClassBucketBuffers,
        slot: &GpuBrainSlot,
        pending: &GpuPendingEligibilityRecord,
        expected_transaction_generation: u64,
    ) -> Result<GpuEligibilityDiscardRecord, GpuClosedLoopError> {
        self.discard_pending_eligibility_impl(
            device,
            queue,
            buffers,
            slot,
            pending,
            expected_transaction_generation,
        )
    }

    fn discard_pending_eligibility_impl(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        buffers: &impl ClosedLoopBufferSet,
        slot: &GpuBrainSlot,
        pending: &GpuPendingEligibilityRecord,
        expected_transaction_generation: u64,
    ) -> Result<GpuEligibilityDiscardRecord, GpuClosedLoopError> {
        self.authority.ensure_healthy()?;
        if self.authority.pending.is_some()
            || buffers.ownership_token() != self.bucket_ownership_token
            || buffers.buffer_set_token() != self.buffer_set_token
            || slot.record().slot != slot.brain_slot_index()
            || slot.record().slot != pending.slot
            || slot.record().slot_generation != pending.slot_generation
            || slot.record().selection_offset.checked_add(12).is_none()
            || expected_transaction_generation == 0
        {
            return Err(GpuClosedLoopError::StaleOrForeignHandle);
        }
        let next_transaction_generation = expected_transaction_generation
            .checked_add(1)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let recurrent_synapse_count = slot.record().recurrent_synapse_count;
        let decoder_synapse_count = slot
            .record()
            .synapse_count
            .checked_sub(recurrent_synapse_count)
            .ok_or(GpuClosedLoopError::MalformedUpload)?;
        if recurrent_synapse_count == 0 || decoder_synapse_count == 0 {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        let header = GpuLearningHeader {
            schema_version: u32::from(SchemaVersions::CURRENT.learning.raw()),
            class_id: slot.record().class_id,
            slot: slot.record().slot,
            slot_generation: slot.record().slot_generation,
            brain_slot_index: slot.brain_slot_index(),
            active_activation_side: pending.active_activation_side,
            dispatch_generation_lo: pending.dispatch_generation[0],
            dispatch_generation_hi: pending.dispatch_generation[1],
            candidate_count: 0,
            candidate_offset: 0,
            decoder_learning_input_offset: 0,
            selection_offset: slot.record().selection_offset,
            outcome_offset: 0,
            recurrent_synapse_count,
            decoder_synapse_count,
            decoder_input_stride: 0,
            pending_eligibility_offset: slot.word_ranges().pending_eligibility_words.start,
            scheduled_tile_visits: 0,
            scheduled_synapse_ops: 0,
            scheduled_work_checksum: 0,
        };
        let mut dispatch_words = vec![0_u32; GPU_ACTIVE_DISPATCH_ROW_WORDS];
        dispatch_words[GPU_PERCEPTION_DISPATCH_ROW_WORDS
            ..GPU_PERCEPTION_DISPATCH_ROW_WORDS + GPU_LEARNING_HEADER_WORDS]
            .copy_from_slice(header.words());
        if dispatch_words.len() > buffers.dispatch_capacity_words()
            || crate::GPU_PENDING_ELIGIBILITY_WORDS > buffers.frame_payload_capacity_words()
            || buffers.compact_readback_capacity_bytes() < 48
        {
            return Err(GpuClosedLoopError::CapacityExceeded);
        }
        let neural = buffers.neural_buffers();
        queue.write_buffer(neural[4], 0, bytemuck::cast_slice(&dispatch_words));
        queue.write_buffer(neural[5], 0, bytemuck::cast_slice(pending.words()));
        let total_groups = slot.record().synapse_count.div_ceil(WORKGROUP_SIZE);
        if total_groups == 0 || total_groups > self.max_compute_workgroups_per_dimension {
            return Err(GpuClosedLoopError::CapacityExceeded);
        }
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("closed-loop-discard-pending-eligibility"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("closed-loop-discard-eligibility-arrays-pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.kernels.discard_pending_eligibility_arrays_pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.dispatch_workgroups(total_groups, 1, 1);
        }
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("closed-loop-finalize-discard-eligibility-pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.kernels.finalize_discard_pending_eligibility_pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        encoder.copy_buffer_to_buffer(
            neural[6],
            u64::from(slot.record().selection_offset) * 4,
            buffers.compact_readback(),
            0,
            48,
        );
        let command_buffer = encoder.finish();
        let (sender, receiver) = mpsc::channel();
        command_buffer.map_buffer_on_submit(
            buffers.compact_readback(),
            wgpu::MapMode::Read,
            0..48,
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
            return Err(GpuClosedLoopError::SubmissionFailed);
        }
        let mapped = buffers.compact_readback().slice(..48).get_mapped_range();
        let words = bytemuck::cast_slice::<u8, u32>(&mapped).to_vec();
        drop(mapped);
        buffers.compact_readback().unmap();
        let record = GpuEligibilityDiscardRecord::from_words(&words)?;
        if record.schema_version != u32::from(SchemaVersions::CURRENT.learning.raw())
            || record.slot != slot.record().slot
            || record.slot_generation != slot.record().slot_generation
            || record.status != 1
            || record.active_eligibility_bank > 1
            || record.reserved != 0
            || record.active_eligibility_generation()
                != (u64::from(pending.active_eligibility_generation[0])
                    | (u64::from(pending.active_eligibility_generation[1]) << 32))
            || record.discarded_staging_generation()
                != (u64::from(pending.staging_eligibility_generation[0])
                    | (u64::from(pending.staging_eligibility_generation[1]) << 32))
            || record.transaction_generation() != next_transaction_generation
        {
            return Err(GpuClosedLoopError::SubmissionFailed);
        }
        Ok(record)
    }

    /// Records this class's complete closed-loop work and copies only the
    /// 48-byte winner/eligibility-completion proof into caller-owned readback.
    pub(crate) fn record_staged_closed_loop(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        buffers: &impl ClosedLoopBufferSet,
        batch: &GpuActiveBatchUpload,
    ) -> Result<u64, GpuClosedLoopError> {
        self.validate_buffers_and_dispatch(buffers, batch)?;
        let readback_bytes = self.readback_bytes(buffers, batch)?;
        self.authority.record_encode(batch.authority_nonce)?;
        let result = (|| {
            self.record_staged_compute_pass(encoder, batch)?;
            self.authority.record_recurrent(batch.authority_nonce)?;
            self.authority.record_selection(batch.authority_nonce)?;
            self.authority.record_eligibility(batch.authority_nonce)?;
            let neural = buffers.neural_buffers();
            for (row, selection_offset) in batch.selection_offsets.iter().enumerate() {
                let readback_base = row as u64 * GPU_CLOSED_LOOP_TICK_READBACK_BYTES as u64;
                encoder.copy_buffer_to_buffer(
                    neural[6],
                    u64::from(*selection_offset) * 4,
                    buffers.compact_readback(),
                    readback_base,
                    crate::GPU_SELECTION_RECORD_BYTES as u64,
                );
            }
            Ok(readback_bytes)
        })();
        if result.is_err() {
            self.authority.recording_failed(batch.authority_nonce)?;
        }
        result
    }

    fn record_staged_compute_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        batch: &GpuActiveBatchUpload,
    ) -> Result<(), GpuClosedLoopError> {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("closed-loop-authoritative-compute-pass"),
            timestamp_writes: None,
        });
        self.record_encode(&mut pass, batch)?;
        self.record_microsteps(&mut pass, batch)?;
        self.record_decode_select(&mut pass, batch)?;
        self.record_eligibility(&mut pass, batch)?;
        Ok(())
    }

    pub(crate) fn register_compact_mapping(
        &self,
        command_buffer: &wgpu::CommandBuffer,
        buffers: &impl ClosedLoopBufferSet,
        batch: &GpuActiveBatchUpload,
    ) -> Result<GpuCompactMapTicket, GpuClosedLoopError> {
        self.validate_buffers_and_dispatch(buffers, batch)?;
        self.authority.require_stage(
            batch.authority_nonce,
            BatchLifecycleStage::EligibilityRecorded,
        )?;
        let readback_bytes = self.readback_bytes(buffers, batch)?;
        let (sender, receiver) = mpsc::channel();
        command_buffer.map_buffer_on_submit(
            buffers.compact_readback(),
            wgpu::MapMode::Read,
            0..readback_bytes,
            move |result| {
                let _ = sender.send(result);
            },
        );
        Ok(GpuCompactMapTicket { receiver })
    }

    /// Copies and validates compact GPU records but deliberately leaves the
    /// class active-side authority unchanged until `commit_validated_batch`.
    pub(crate) fn decode_validate_mapped_records(
        &mut self,
        buffers: &impl ClosedLoopBufferSet,
        batch: &GpuActiveBatchUpload,
    ) -> Result<GpuValidatedClassBatch, GpuClosedLoopError> {
        self.validate_buffers_and_dispatch(buffers, batch)?;
        self.authority.require_stage(
            batch.authority_nonce,
            BatchLifecycleStage::EligibilityRecorded,
        )?;
        let readback_bytes = self.readback_bytes(buffers, batch)?;
        let mapped = buffers
            .compact_readback()
            .slice(..readback_bytes)
            .get_mapped_range();
        let words: Vec<u32> = bytemuck::cast_slice(&mapped).to_vec();
        drop(mapped);
        buffers.compact_readback().unmap();
        let row_words = GPU_CLOSED_LOOP_TICK_READBACK_BYTES / 4;
        if words.len() != batch.row_count() * row_words {
            return Err(GpuClosedLoopError::SubmissionFailed);
        }
        let mut records = Vec::with_capacity(batch.row_count());
        for row in words.chunks_exact(row_words) {
            records.push(GpuSelectionRecord::from_words(row)?);
        }
        #[cfg(feature = "gpu-tests")]
        if let Some((slot, generation)) = self.force_all_invalid_slot.take() {
            let record = records
                .iter_mut()
                .find(|record| record.slot == slot && record.slot_generation == generation)
                .ok_or(GpuClosedLoopError::StaleOrForeignHandle)?;
            record.candidate_index = u32::MAX;
            record.logit_bits = 0;
            record.confidence_q16 = 0;
            record.status = 2;
        }
        if !self.validate_selection_records(batch, &records) {
            return Err(GpuClosedLoopError::SubmissionFailed);
        }
        let pending_records = self
            .build_pending_eligibility_records(batch, &records)
            .ok_or(GpuClosedLoopError::SubmissionFailed)?;
        #[cfg(feature = "gpu-tests")]
        let mut pending_records = pending_records;
        #[cfg(feature = "gpu-tests")]
        if let Some((slot, generation)) = self.force_pending_identity_mismatch_slot.take() {
            let pending = records
                .iter()
                .zip(&mut pending_records)
                .find_map(|(record, pending)| {
                    (record.slot == slot && record.slot_generation == generation).then_some(pending)
                })
                .ok_or(GpuClosedLoopError::StaleOrForeignHandle)?;
            pending.phenotype_hash[0] ^= 1;
        }
        let final_sides = batch
            .headers
            .iter()
            .zip(&records)
            .map(|(header, record)| {
                (
                    header.brain_slot_index,
                    header.slot_generation,
                    record.active_activation_side,
                )
            })
            .collect();
        for record in &mut records {
            if record.status == 3 {
                record.status = 1;
            }
        }
        Ok(GpuValidatedClassBatch {
            bucket_ownership_token: self.bucket_ownership_token,
            authority_nonce: batch.authority_nonce,
            records,
            pending_records,
            final_sides,
            readback_bytes,
        })
    }

    #[cfg(feature = "gpu-tests")]
    pub(crate) fn force_all_invalid_record_for_test(&mut self, slot: u32, generation: u32) {
        self.force_all_invalid_slot = Some((slot, generation));
    }

    #[cfg(feature = "gpu-tests")]
    pub(crate) fn force_pending_identity_mismatch_for_test(&mut self, slot: u32, generation: u32) {
        self.force_pending_identity_mismatch_slot = Some((slot, generation));
    }

    pub(crate) fn commit_validated_batch(
        &mut self,
        validated: GpuValidatedClassBatch,
    ) -> Result<GpuCommittedClassBatch, GpuClosedLoopError> {
        if validated.bucket_ownership_token != self.bucket_ownership_token {
            return Err(GpuClosedLoopError::StaleOrForeignHandle);
        }
        self.authority
            .submission_succeeded(validated.authority_nonce, &validated.final_sides)?;
        Ok(GpuCommittedClassBatch {
            records: validated.records,
            pending_records: validated.pending_records,
            readback_bytes: validated.readback_bytes,
        })
    }

    pub(crate) fn prevalidate_commit_validated_batch(
        &self,
        validated: &GpuValidatedClassBatch,
    ) -> Result<(), GpuClosedLoopError> {
        if validated.bucket_ownership_token != self.bucket_ownership_token {
            return Err(GpuClosedLoopError::StaleOrForeignHandle);
        }
        self.authority
            .prevalidate_submission_succeeded(validated.authority_nonce)
    }

    #[cfg(feature = "gpu-tests")]
    pub fn submit_encode_and_microsteps(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        batch: &GpuActiveBatchUpload,
    ) -> Result<GpuRecurrentDispatchReceipt, GpuClosedLoopError> {
        self.validate_recurrent_diagnostic_batch(batch)?;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("closed-loop-authoritative-batch"),
        });
        self.authority.record_encode(batch.authority_nonce)?;
        let recorded = (|| {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("closed-loop-recurrent-diagnostic-compute-pass"),
                timestamp_writes: None,
            });
            self.record_encode(&mut pass, batch)?;
            self.record_microsteps(&mut pass, batch)
        })();
        let receipt = match recorded {
            Ok(receipt) => receipt,
            Err(error) => {
                self.authority.recording_failed(batch.authority_nonce)?;
                return Err(error);
            }
        };
        self.authority.record_recurrent(batch.authority_nonce)?;
        let submission = queue.submit(Some(encoder.finish()));
        if device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .is_err()
        {
            self.authority
                .submission_indeterminate(batch.authority_nonce)?;
            return Err(GpuClosedLoopError::SubmissionFailed);
        }
        let final_sides = batch
            .headers
            .iter()
            .map(|header| {
                Ok((
                    header.brain_slot_index,
                    header.slot_generation,
                    Self::final_activation_side(
                        header.active_activation_side,
                        header.microstep_count,
                    )?,
                ))
            })
            .collect::<Result<Vec<_>, GpuClosedLoopError>>()?;
        self.authority
            .recurrent_diagnostic_succeeded(batch.authority_nonce, &final_sides)?;
        Ok(receipt)
    }

    pub async fn submit_closed_loop_frame(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        buffers: &GpuClassBucketBuffers,
        batch: &GpuActiveBatchUpload,
    ) -> Result<
        (
            Vec<GpuSelectionRecord>,
            Vec<GpuPendingEligibilityRecord>,
            u64,
        ),
        GpuClosedLoopError,
    > {
        self.write_staged_uploads(queue, buffers, batch)?;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("closed-loop-authoritative-frame"),
        });
        self.record_staged_closed_loop(&mut encoder, buffers, batch)?;
        let command_buffer = encoder.finish();
        let map_ticket = self.register_compact_mapping(&command_buffer, buffers, batch)?;
        let submission = queue.submit(Some(command_buffer));
        let poll_result = device.poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        });
        if poll_result.is_err() || !map_ticket.mapping_succeeded() {
            self.mark_post_submit_poison(batch)?;
            buffers.compact_readback().unmap();
            return Err(GpuClosedLoopError::SubmissionFailed);
        }
        let validated = match self.decode_validate_mapped_records(buffers, batch) {
            Ok(validated) => validated,
            Err(_) => {
                self.mark_post_submit_poison(batch)?;
                return Err(GpuClosedLoopError::SubmissionFailed);
            }
        };
        let committed = self.commit_validated_batch(validated)?;
        Ok((
            committed.records,
            committed.pending_records,
            committed.readback_bytes,
        ))
    }

    fn record_eligibility<'pass>(
        &'pass self,
        pass: &mut wgpu::ComputePass<'pass>,
        batch: &GpuActiveBatchUpload,
    ) -> Result<(), GpuClosedLoopError> {
        self.validate_dispatch(batch)?;
        let rows =
            u32::try_from(batch.row_count()).map_err(|_| GpuClosedLoopError::CapacityExceeded)?;
        let recurrent_synapses = batch
            .learning_headers
            .iter()
            .map(|header| header.recurrent_synapse_count)
            .max()
            .ok_or(GpuClosedLoopError::MalformedUpload)?;
        let decoder_synapses = batch
            .learning_headers
            .iter()
            .map(|header| header.decoder_synapse_count)
            .max()
            .ok_or(GpuClosedLoopError::MalformedUpload)?;
        let recurrent_groups = recurrent_synapses.div_ceil(WORKGROUP_SIZE);
        let decoder_groups = decoder_synapses.div_ceil(WORKGROUP_SIZE);
        if recurrent_groups == 0
            || decoder_groups == 0
            || recurrent_groups > self.max_compute_workgroups_per_dimension
            || decoder_groups > self.max_compute_workgroups_per_dimension
            || rows > self.max_compute_workgroups_per_dimension
        {
            return Err(GpuClosedLoopError::CapacityExceeded);
        }
        pass.set_pipeline(&self.kernels.prevalidate_eligibility_pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.dispatch_workgroups(1, rows, 1);
        pass.set_pipeline(&self.kernels.recurrent_eligibility_pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.dispatch_workgroups(recurrent_groups, rows, 1);
        pass.set_pipeline(&self.kernels.decoder_eligibility_pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.dispatch_workgroups(decoder_groups, rows, 1);
        pass.set_pipeline(&self.kernels.finalize_pending_eligibility_pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.dispatch_workgroups(1, rows, 1);
        Ok(())
    }

    fn record_decode_select<'pass>(
        &'pass self,
        pass: &mut wgpu::ComputePass<'pass>,
        batch: &GpuActiveBatchUpload,
    ) -> Result<(), GpuClosedLoopError> {
        self.validate_dispatch(batch)?;
        let rows =
            u32::try_from(batch.row_count()).map_err(|_| GpuClosedLoopError::CapacityExceeded)?;
        pass.set_pipeline(&self.kernels.decode_pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.dispatch_workgroups(1, rows, 1);
        pass.set_pipeline(&self.kernels.memory_context_pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.dispatch_workgroups(1, rows, 1);
        pass.set_pipeline(&self.kernels.select_pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.dispatch_workgroups(1, rows, 1);
        Ok(())
    }

    fn validate_selection_records(
        &self,
        batch: &GpuActiveBatchUpload,
        records: &[GpuSelectionRecord],
    ) -> bool {
        records.len() == batch.headers.len()
            && records.iter().zip(&batch.headers).all(|(record, header)| {
                let generation = u64::from(record.dispatch_generation_lo)
                    | (u64::from(record.dispatch_generation_hi) << 32);
                let expected_generation = u64::from(header.dispatch_generation_lo)
                    | (u64::from(header.dispatch_generation_hi) << 32);
                let expected_side = Self::final_activation_side(
                    header.active_activation_side,
                    header.microstep_count,
                )
                .ok();
                if record.slot != header.slot
                    || record.slot_generation != header.slot_generation
                    || generation == 0
                    || generation != expected_generation
                    || Some(record.active_activation_side) != expected_side
                    || record.active_tiles == 0
                    || record.active_synapses == 0
                {
                    return false;
                }
                match record.status {
                    3 => {
                        if record.candidate_index >= header.candidate_count
                            || !f32::from_bits(record.logit_bits).is_finite()
                        {
                            return false;
                        }
                        let base = header.candidate_offset as usize
                            + record.candidate_index as usize * GPU_CANDIDATE_RECORD_WORDS;
                        GpuCandidateRecord::from_words(
                            &batch.dispatch_header_words[base..base + GPU_CANDIDATE_RECORD_WORDS],
                        )
                        .is_ok_and(|candidate| candidate.confidence_q16 == record.confidence_q16)
                    }
                    2 => {
                        record.candidate_index == u32::MAX
                            && record.logit_bits == 0
                            && record.confidence_q16 == 0
                    }
                    _ => false,
                }
            })
    }

    fn build_pending_eligibility_records(
        &self,
        batch: &GpuActiveBatchUpload,
        selections: &[GpuSelectionRecord],
    ) -> Option<Vec<GpuPendingEligibilityRecord>> {
        if selections.len() != batch.row_count()
            || batch.pending_templates.len() != batch.row_count()
            || batch.learning_headers.len() != batch.row_count()
        {
            return None;
        }
        let mut pending_records = Vec::with_capacity(batch.row_count());
        for ((selection, template), learning_header) in selections
            .iter()
            .zip(&batch.pending_templates)
            .zip(&batch.learning_headers)
        {
            if selection.status == 2 {
                if selection.candidate_index != u32::MAX {
                    return None;
                }
                pending_records.push(GpuPendingEligibilityRecord::zeroed());
                continue;
            }
            if selection.status != 3 || selection.candidate_index >= learning_header.candidate_count
            {
                return None;
            }
            let candidate_base = usize::try_from(learning_header.candidate_offset)
                .ok()
                .and_then(|base| {
                    usize::try_from(selection.candidate_index)
                        .ok()
                        .and_then(|index| index.checked_mul(GPU_CANDIDATE_RECORD_WORDS))
                        .and_then(|offset| base.checked_add(offset))
                })?;
            let candidate_end = match candidate_base.checked_add(GPU_CANDIDATE_RECORD_WORDS) {
                Some(end) if end <= batch.dispatch_header_words.len() => end,
                _ => return None,
            };
            let candidate = match GpuCandidateRecord::from_words(
                &batch.dispatch_header_words[candidate_base..candidate_end],
            ) {
                Ok(candidate) => candidate,
                Err(_) => return None,
            };
            let digest_base = usize::try_from(learning_header.decoder_learning_input_offset)
                .ok()
                .and_then(|base| {
                    usize::try_from(learning_header.candidate_count)
                        .ok()
                        .and_then(|count| {
                            count.checked_mul(learning_header.decoder_input_stride as usize)
                        })
                        .and_then(|feature_words| base.checked_add(feature_words))
                })
                .and_then(|base| {
                    usize::try_from(selection.candidate_index)
                        .ok()
                        .and_then(|index| index.checked_mul(4))
                        .and_then(|offset| base.checked_add(offset))
                })?;
            let digest_end = match digest_base.checked_add(4) {
                Some(end) if end <= batch.frame_payload_words.len() => end,
                _ => return None,
            };
            let mut expected = *template;
            let candidate_index = match u16::try_from(selection.candidate_index) {
                Ok(index) => index,
                Err(_) => return None,
            };
            let family = u8::try_from(candidate.family)
                .ok()
                .and_then(|raw| alife_core::CandidateActionFamily::try_from_raw(raw).ok())?;
            expected.candidate_index_and_family =
                crate::pack_candidate_index_and_family(candidate_index, family);
            expected.action_id = candidate.action_id;
            expected
                .candidate_feature_digest
                .copy_from_slice(&batch.frame_payload_words[digest_base..digest_end]);
            pending_records.push(expected);
        }
        Some(pending_records)
    }

    fn record_encode<'pass>(
        &'pass self,
        pass: &mut wgpu::ComputePass<'pass>,
        batch: &GpuActiveBatchUpload,
    ) -> Result<(), GpuClosedLoopError> {
        pass.set_pipeline(&self.kernels.clear_diagnostics_pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.dispatch_workgroups(
            1,
            u32::try_from(batch.row_count()).map_err(|_| GpuClosedLoopError::CapacityExceeded)?,
            1,
        );
        pass.set_pipeline(&self.kernels.encode_pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.dispatch_workgroups(
            self.max_neurons.div_ceil(WORKGROUP_SIZE),
            u32::try_from(batch.row_count()).map_err(|_| GpuClosedLoopError::CapacityExceeded)?,
            1,
        );
        Ok(())
    }

    /// Dispatches recurrent WGSL only. Candidate decode consumes diagnostic
    /// lane 3 as the GPU-authored active-side receipt before selection.
    fn record_microsteps<'pass>(
        &'pass self,
        pass: &mut wgpu::ComputePass<'pass>,
        batch: &GpuActiveBatchUpload,
    ) -> Result<GpuRecurrentDispatchReceipt, GpuClosedLoopError> {
        let max_microsteps = batch
            .headers
            .iter()
            .map(|h| h.microstep_count)
            .max()
            .ok_or(GpuClosedLoopError::MalformedUpload)?;
        Self::validate_microstep_count(max_microsteps)?;
        for step in 0..max_microsteps as usize {
            pass.set_pipeline(&self.kernels.recurrent_pipelines[step]);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.dispatch_workgroups(
                self.max_neurons.div_ceil(WORKGROUP_SIZE),
                u32::try_from(batch.row_count())
                    .map_err(|_| GpuClosedLoopError::CapacityExceeded)?,
                1,
            );
        }
        let initial_activation_sides = batch
            .headers
            .iter()
            .map(|header| header.active_activation_side)
            .collect::<Vec<_>>();
        let row_microstep_counts = batch
            .headers
            .iter()
            .map(|header| header.microstep_count)
            .collect::<Vec<_>>();
        Ok(GpuRecurrentDispatchReceipt {
            max_microsteps_dispatched: max_microsteps,
            initial_activation_sides,
            row_microstep_counts,
        })
    }

    fn validate_batch_identity(
        &self,
        batch: &GpuActiveBatchUpload,
    ) -> Result<(), GpuClosedLoopError> {
        self.authority.ensure_healthy()?;
        if batch.bucket_ownership_token != self.bucket_ownership_token
            || self.authority.pending.map(|pending| pending.nonce) != Some(batch.authority_nonce)
        {
            return Err(GpuClosedLoopError::StaleOrForeignHandle);
        }
        Ok(())
    }

    fn validate_buffers_and_dispatch(
        &self,
        buffers: &impl ClosedLoopBufferSet,
        batch: &GpuActiveBatchUpload,
    ) -> Result<(), GpuClosedLoopError> {
        if buffers.ownership_token() != self.bucket_ownership_token
            || buffers.buffer_set_token() != self.buffer_set_token
        {
            return Err(GpuClosedLoopError::StaleOrForeignHandle);
        }
        self.validate_dispatch(batch)
    }

    fn readback_bytes(
        &self,
        buffers: &impl ClosedLoopBufferSet,
        batch: &GpuActiveBatchUpload,
    ) -> Result<u64, GpuClosedLoopError> {
        let bytes = batch
            .row_count()
            .checked_mul(GPU_CLOSED_LOOP_TICK_READBACK_BYTES)
            .and_then(|value| u64::try_from(value).ok())
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        if bytes == 0 || bytes > buffers.compact_readback_capacity_bytes() {
            return Err(GpuClosedLoopError::CapacityExceeded);
        }
        Ok(bytes)
    }

    fn validate_dispatch(&self, batch: &GpuActiveBatchUpload) -> Result<(), GpuClosedLoopError> {
        self.validate_batch_identity(batch)?;
        if batch.headers.iter().any(|header| {
            self.authority
                .active_sides
                .get(&(header.brain_slot_index, header.slot_generation))
                .copied()
                != Some(header.active_activation_side)
        }) {
            return Err(GpuClosedLoopError::StaleOrForeignHandle);
        }
        validate_dispatch_dimensions(
            self.max_neurons,
            batch.row_count(),
            self.max_compute_workgroups_per_dimension,
        )?;
        validate_dispatch(self.max_neurons, batch)
    }

    #[cfg(feature = "gpu-tests")]
    fn validate_recurrent_diagnostic_batch(
        &self,
        batch: &GpuActiveBatchUpload,
    ) -> Result<(), GpuClosedLoopError> {
        self.validate_batch_identity(batch)?;
        if self.max_neurons == 0
            || batch.headers.is_empty()
            || batch.headers.iter().any(|header| {
                header.neuron_count != self.max_neurons || header.active_activation_side > 1
            })
            || batch.dispatch_header_words.len()
                != batch.row_count() * GPU_ACTIVE_DISPATCH_ROW_WORDS
            || batch.headers.iter().any(|header| {
                self.authority
                    .active_sides
                    .get(&(header.brain_slot_index, header.slot_generation))
                    .copied()
                    != Some(header.active_activation_side)
            })
        {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        validate_dispatch_dimensions(
            self.max_neurons,
            batch.row_count(),
            self.max_compute_workgroups_per_dimension,
        )?;
        Ok(())
    }
}

pub fn validate_dispatch_dimensions(
    max_neurons: u32,
    row_count: usize,
    limit: u32,
) -> Result<[u32; 3], GpuClosedLoopError> {
    if max_neurons == 0 || row_count == 0 {
        return Err(GpuClosedLoopError::MalformedUpload);
    }
    let y = u32::try_from(row_count).map_err(|_| GpuClosedLoopError::CapacityExceeded)?;
    let dimensions = [max_neurons.div_ceil(WORKGROUP_SIZE), y, 1];
    if dimensions.into_iter().any(|value| value > limit) {
        return Err(GpuClosedLoopError::CapacityExceeded);
    }
    Ok(dimensions)
}

fn validate_dispatch(
    max_neurons: u32,
    batch: &GpuActiveBatchUpload,
) -> Result<(), GpuClosedLoopError> {
    if max_neurons == 0
        || batch.headers.is_empty()
        || batch.learning_headers.len() != batch.headers.len()
        || batch.activity_headers.len() != batch.headers.len()
        || batch.pending_templates.len() != batch.headers.len()
        || batch.selection_offsets.len() != batch.headers.len()
        || batch.memory_context_bindings.len() != batch.headers.len()
        || batch.headers.iter().any(|header| {
            header.neuron_count == 0
                || header.neuron_count != max_neurons
                || header.active_activation_side > 1
        })
        || batch.dispatch_header_words.len() != batch.row_count() * GPU_ACTIVE_DISPATCH_ROW_WORDS
    {
        return Err(GpuClosedLoopError::MalformedUpload);
    }
    let expected_learning_schema = u32::from(SchemaVersions::CURRENT.learning.raw());
    for (row, (((((header, learning), activity), pending), selection_offset), memory_binding)) in
        batch
            .headers
            .iter()
            .zip(&batch.learning_headers)
            .zip(&batch.activity_headers)
            .zip(&batch.pending_templates)
            .zip(&batch.selection_offsets)
            .zip(&batch.memory_context_bindings)
            .enumerate()
    {
        let expected_final_side = header.active_activation_side ^ (header.microstep_count & 1);
        if learning.schema_version != expected_learning_schema
            || activity.class_id() != header.class_id
            || activity.slot() != header.slot
            || activity.slot_generation() != header.slot_generation
            || activity.brain_slot_index() != header.brain_slot_index
            || u32::from(activity.microsteps()) != header.microstep_count
            || learning.class_id != header.class_id
            || learning.slot != header.slot
            || learning.slot_generation != header.slot_generation
            || learning.brain_slot_index != header.brain_slot_index
            || learning.active_activation_side != expected_final_side
            || learning.dispatch_generation_lo != header.dispatch_generation_lo
            || learning.dispatch_generation_hi != header.dispatch_generation_hi
            || learning.dispatch_generation() == 0
            || learning.candidate_count != header.candidate_count
            || learning.candidate_offset != header.candidate_offset
            || learning.selection_offset != *selection_offset
            || learning.recurrent_synapse_count == 0
            || learning.decoder_synapse_count == 0
            || learning.decoder_input_stride < CANDIDATE_FEATURE_COUNT as u32
            || learning.decoder_input_stride > 64
            || learning.scheduled_tile_visits == 0
            || learning.scheduled_synapse_ops == 0
            || learning.scheduled_work_checksum
                != activity.scheduled_work_checksum(
                    learning.scheduled_tile_visits,
                    learning.scheduled_synapse_ops,
                )
            || pending.schema_version != expected_learning_schema
            || pending.slot != header.slot
            || pending.slot_generation != header.slot_generation
            || pending.active_activation_side != expected_final_side
            || pending.dispatch_generation
                != [header.dispatch_generation_lo, header.dispatch_generation_hi]
        {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        let candidate_count = usize::try_from(learning.candidate_count)
            .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?;
        for candidate_index in 0..candidate_count {
            let candidate_start = usize::try_from(learning.candidate_offset)
                .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?
                .checked_add(
                    candidate_index
                        .checked_mul(GPU_CANDIDATE_RECORD_WORDS)
                        .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
                )
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
            let candidate_end = candidate_start
                .checked_add(GPU_CANDIDATE_RECORD_WORDS)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
            if candidate_end > batch.dispatch_header_words.len() {
                return Err(GpuClosedLoopError::MalformedUpload);
            }
            let candidate = GpuCandidateRecord::from_words(
                &batch.dispatch_header_words[candidate_start..candidate_end],
            )?;
            let expected_feature_offset = learning
                .decoder_learning_input_offset
                .checked_add(
                    u32::try_from(candidate_index)
                        .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?
                        .checked_mul(learning.decoder_input_stride)
                        .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
                )
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
            if candidate.candidate_index
                != u32::try_from(candidate_index)
                    .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?
                || candidate.feature_offset != expected_feature_offset
            {
                return Err(GpuClosedLoopError::MalformedUpload);
            }
        }
        let expected_outcome_offset = learning
            .decoder_learning_input_offset
            .checked_add(
                learning
                    .candidate_count
                    .checked_mul(learning.decoder_input_stride)
                    .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
            )
            .and_then(|value| value.checked_add(learning.candidate_count.checked_mul(4)?))
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        if learning.outcome_offset != expected_outcome_offset {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        let row_base = row
            .checked_mul(GPU_ACTIVE_DISPATCH_ROW_WORDS)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let learning_start = row_base + GPU_PERCEPTION_DISPATCH_ROW_WORDS;
        if batch.dispatch_header_words[learning_start..learning_start + GPU_LEARNING_HEADER_WORDS]
            != *learning.words()
        {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        let memory_header_start = learning_start + GPU_LEARNING_HEADER_WORDS;
        let memory_header_end = memory_header_start
            .checked_add(crate::GPU_MEMORY_CONTEXT_HEADER_WORDS)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        if memory_header_end > batch.dispatch_header_words.len() {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        let memory_header_words =
            &batch.dispatch_header_words[memory_header_start..memory_header_end];
        match memory_binding {
            None => {
                if memory_header_words.iter().any(|word| *word != 0) {
                    return Err(GpuClosedLoopError::MalformedUpload);
                }
            }
            Some(memory_binding) => {
                let memory_header = GpuMemoryContextHeader::from_words(memory_header_words)?;
                if memory_header.schema_version == 0
                    || memory_header.class_id != header.class_id
                    || memory_header.slot != header.slot
                    || memory_header.slot_generation != header.slot_generation
                    || memory_header.tick()
                        != (u64::from(header.tick_lo) | (u64::from(header.tick_hi) << 32))
                    || memory_header.candidate_count != header.candidate_count
                    || memory_header.candidate_offset != header.candidate_offset
                    || memory_header.brain_slot_index != header.brain_slot_index
                    || memory_header.decoder_learning_input_offset
                        != learning.decoder_learning_input_offset
                    || usize::try_from(memory_header.perception_header_index)
                        .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?
                        != row_base
                    || memory_header.profile_id == 0
                    || memory_header.profile_schema_version == 0
                    || memory_header.sensory_abi_version == 0
                    || memory_header.reserved != 0
                    || memory_binding.slot != memory_header.slot
                    || memory_binding.slot_generation != memory_header.slot_generation
                    || memory_binding.perception_header_index
                        != memory_header.perception_header_index
                    || u32::from(memory_binding.candidate_count) != memory_header.candidate_count
                    || memory_binding.base_frame_digest.0 == [0; 4]
                    || memory_binding.context_digest.0 == [0; 4]
                    || memory_binding.final_frame_digest.0 == [0; 4]
                {
                    return Err(GpuClosedLoopError::MalformedUpload);
                }
                let final_digest_words = memory_binding
                    .final_frame_digest
                    .0
                    .into_iter()
                    .flat_map(|word| [word as u32, (word >> 32) as u32])
                    .collect::<Vec<_>>();
                if pending.frame_digest != final_digest_words.as_slice() {
                    return Err(GpuClosedLoopError::MalformedUpload);
                }
                let memory_record_start = usize::try_from(memory_header.memory_context_offset)
                    .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?;
                let memory_record_words = usize::try_from(memory_header.candidate_count)
                    .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?
                    .checked_mul(crate::GPU_CANDIDATE_MEMORY_RECORD_WORDS)
                    .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
                let memory_record_end = memory_record_start
                    .checked_add(memory_record_words)
                    .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
                if memory_record_start == 0 || memory_record_end > batch.frame_payload_words.len() {
                    return Err(GpuClosedLoopError::MalformedUpload);
                }
                for candidate_index in 0..memory_header.candidate_count {
                    let record_start = memory_record_start
                        + usize::try_from(candidate_index)
                            .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?
                            * crate::GPU_CANDIDATE_MEMORY_RECORD_WORDS;
                    let record = GpuCandidateMemoryRecord::from_words(
                        &batch.frame_payload_words
                            [record_start..record_start + crate::GPU_CANDIDATE_MEMORY_RECORD_WORDS],
                    )?;
                    if record.candidate_index != candidate_index
                        || !record.target_confidence.is_finite()
                        || !(0.0..=1.0).contains(&record.target_confidence)
                        || !record.family_confidence.is_finite()
                        || !(0.0..=1.0).contains(&record.family_confidence)
                        || record
                            .target_latent
                            .iter()
                            .chain(record.family_value.iter())
                            .any(|value| !value.is_finite())
                    {
                        return Err(GpuClosedLoopError::NonFinitePayload);
                    }
                }
            }
        }
        let activity_start = memory_header_end;
        let activity_end = activity_start
            .checked_add(crate::GPU_ACTIVITY_DISPATCH_HEADER_WORDS)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        if activity_end > batch.dispatch_header_words.len()
            || batch.dispatch_header_words[activity_start..activity_end] != *activity.words()
        {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        let pending_start = usize::try_from(learning.outcome_offset)
            .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?;
        let pending_end = pending_start
            .checked_add(crate::GPU_PENDING_ELIGIBILITY_WORDS)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        if pending_end > batch.frame_payload_words.len()
            || batch.frame_payload_words[pending_start..pending_end] != *pending.words()
        {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
    }
    Ok(())
}

fn create_neural_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let entries = crate::GpuClassBucketBuffers::neural_binding_manifest().map(|manifest| {
        wgpu::BindGroupLayoutEntry {
            binding: manifest.binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage {
                    read_only: matches!(manifest.access, crate::GpuBufferAccess::ReadOnly),
                },
                has_dynamic_offset: false,
                min_binding_size: NonZeroU64::new(manifest.minimum_binding_size_bytes),
            },
            count: None,
        }
    });
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("closed-loop-neural-bind-group-layout"),
        entries: &entries,
    })
}

fn validate_production_shader_contract(
    source: &str,
    required_entries: &[&str],
) -> Result<(), GpuClosedLoopError> {
    let layout_anchor = format!(
        "GPU_CLOSED_LOOP_LAYOUT_VERSION:u32 = {}u",
        crate::GPU_CLOSED_LOOP_LAYOUT_VERSION
    );
    if !source.contains(&layout_anchor) {
        return Err(GpuClosedLoopError::LayoutMismatch);
    }
    let module =
        naga::front::wgsl::parse_str(source).map_err(|_| GpuClosedLoopError::LayoutMismatch)?;
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .map_err(|_| GpuClosedLoopError::LayoutMismatch)?;

    let mut reflected = BTreeMap::new();
    for (_, global) in module.global_variables.iter() {
        let Some(binding) = global.binding.as_ref() else {
            continue;
        };
        let naga::AddressSpace::Storage { access } = global.space else {
            return Err(GpuClosedLoopError::LayoutMismatch);
        };
        if reflected
            .insert((binding.group, binding.binding), access)
            .is_some()
        {
            return Err(GpuClosedLoopError::LayoutMismatch);
        }
    }

    let manifest = GpuClassBucketBuffers::neural_binding_manifest();
    if reflected.len() != manifest.len() {
        return Err(GpuClosedLoopError::LayoutMismatch);
    }
    for expected in manifest {
        if expected.minimum_binding_size_bytes == 0 {
            return Err(GpuClosedLoopError::LayoutMismatch);
        }
        let expected_access = match expected.access {
            crate::GpuBufferAccess::ReadOnly => naga::StorageAccess::LOAD,
            crate::GpuBufferAccess::ReadWrite => {
                naga::StorageAccess::LOAD | naga::StorageAccess::STORE
            }
        };
        if reflected.get(&(expected.group, expected.binding)) != Some(&expected_access) {
            return Err(GpuClosedLoopError::LayoutMismatch);
        }
    }

    for required in required_entries {
        let matches = module
            .entry_points
            .iter()
            .filter(|entry| entry.name == *required && entry.stage == naga::ShaderStage::Compute)
            .count();
        if matches != 1 {
            return Err(GpuClosedLoopError::LayoutMismatch);
        }
    }
    Ok(())
}

fn create_compute_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    entry_point: &'static str,
    constants: &[(&str, f64)],
) -> wgpu::ComputePipeline {
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(entry_point),
        layout: Some(layout),
        module: shader,
        entry_point: Some(entry_point),
        compilation_options: wgpu::PipelineCompilationOptions {
            constants,
            zero_initialize_workgroup_memory: true,
        },
        cache: None,
    })
}

#[cfg(test)]
mod lifecycle_tests {
    use bytemuck::Zeroable;

    use super::*;

    #[test]
    fn compact_mapping_status_does_not_wait_for_a_missing_callback() {
        let (callback_sender, callback_receiver) =
            mpsc::channel::<Result<(), wgpu::BufferAsyncError>>();
        let ticket = GpuCompactMapTicket {
            receiver: callback_receiver,
        };
        let (result_sender, result_receiver) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            result_sender.send(ticket.mapping_succeeded()).unwrap();
        });

        let observed = result_receiver.recv_timeout(std::time::Duration::from_millis(25));
        drop(callback_sender);
        worker.join().unwrap();
        assert_eq!(observed, Ok(false));
    }

    #[test]
    fn production_shader_contract_validation_rejects_missing_entry_and_eighth_binding() {
        let entries = [
            "initialize_fast_plasticity",
            "apply_fast_plasticity",
            "capture_fast_plasticity_replay",
            "finalize_fast_plasticity",
        ];
        assert_eq!(
            validate_production_shader_contract(CLOSED_LOOP_PLASTICITY_WGSL, &entries),
            Ok(())
        );

        let missing_entry = CLOSED_LOOP_PLASTICITY_WGSL.replacen(
            "fn apply_fast_plasticity",
            "fn retired_fast_plasticity",
            1,
        );
        assert_eq!(
            validate_production_shader_contract(&missing_entry, &entries),
            Err(GpuClosedLoopError::LayoutMismatch)
        );

        let eighth_binding = CLOSED_LOOP_PLASTICITY_WGSL.replacen(
            "@group(0) @binding(6)",
            "@group(0) @binding(7)",
            1,
        );
        assert_eq!(
            validate_production_shader_contract(&eighth_binding, &entries),
            Err(GpuClosedLoopError::LayoutMismatch)
        );
    }

    #[test]
    fn backend_dispatch_generation_is_distinct_from_private_class_nonce() {
        let mut header = GpuPerceptionHeader::zeroed();
        header.dispatch_generation_lo = 0x5566_7788;
        header.dispatch_generation_hi = 0x1122_3344;
        let batch = GpuActiveBatchUpload {
            headers: vec![header],
            learning_headers: Vec::new(),
            activity_headers: Vec::new(),
            pending_templates: Vec::new(),
            dispatch_header_words: header.words().to_vec(),
            frame_payload_words: Vec::new(),
            bucket_ownership_token: 1,
            authority_nonce: 7,
            selection_offsets: vec![0],
            memory_context_bindings: vec![None],
        };

        assert_eq!(batch.authority_nonce_for_test(), 7);
        assert_eq!(batch.dispatch_generation(), 0x1122_3344_5566_7788);
    }

    #[test]
    fn validated_sides_remain_staged_until_explicit_commit() {
        let mut authority = BatchAuthority::default();
        authority.active_sides.insert((3, 7), 0);
        authority.begin(17).unwrap();
        authority.record_encode(17).unwrap();
        authority.record_recurrent(17).unwrap();
        authority.record_selection(17).unwrap();
        let validated_sides = [(3, 7, 1)];

        assert_eq!(authority.active_sides.get(&(3, 7)), Some(&0));
        authority.record_eligibility(17).unwrap();
        authority
            .submission_succeeded(17, &validated_sides)
            .unwrap();
        assert_eq!(authority.active_sides.get(&(3, 7)), Some(&1));
    }

    #[test]
    fn retiring_slot_side_is_exact_and_forbidden_while_a_batch_is_pending() {
        let mut authority = BatchAuthority::default();
        authority.active_sides.insert((5, 11), 1);
        authority.active_sides.insert((5, 12), 0);
        authority.begin(23).unwrap();
        assert_eq!(
            authority.retire_active_side(5, 11),
            Err(GpuClosedLoopError::MalformedUpload)
        );
        authority.abandon_unsubmitted(23).unwrap();
        authority.retire_active_side(5, 11).unwrap();
        assert!(!authority.active_sides.contains_key(&(5, 11)));
        assert_eq!(authority.active_sides.get(&(5, 12)), Some(&0));
    }

    #[test]
    fn recurrent_recording_before_same_batch_encode_is_rejected() {
        let mut authority = BatchAuthority::default();
        authority.begin(41).unwrap();
        assert_eq!(
            authority.record_recurrent(41),
            Err(GpuClosedLoopError::MalformedUpload)
        );
        assert_eq!(authority.pending.unwrap().stage, BatchLifecycleStage::Built);
    }

    #[test]
    fn unsubmitted_or_pre_submit_failure_preserves_side_and_same_nonce_retry() {
        let mut authority = BatchAuthority::default();
        authority.active_sides.insert((3, 7), 1);
        authority.begin(52).unwrap();

        // A built batch that is dropped before recording has no authority to
        // mutate the persistent side and retains its exact retry nonce.
        assert_eq!(authority.active_sides.get(&(3, 7)), Some(&1));
        assert_eq!(authority.pending.unwrap().nonce, 52);

        authority.abandon_unsubmitted(52).unwrap();
        assert_eq!(authority.active_sides.get(&(3, 7)), Some(&1));
        assert_eq!(authority.pending, None);
        authority.begin(53).unwrap();

        authority.record_encode(53).unwrap();
        authority.record_recurrent(53).unwrap();
        assert_eq!(authority.active_sides.get(&(3, 7)), Some(&1));
        assert_eq!(authority.pending.unwrap().nonce, 53);
        authority.recording_failed(53).unwrap();
        assert_eq!(authority.active_sides.get(&(3, 7)), Some(&1));
        assert_eq!(
            authority.pending,
            Some(PendingBatchAuthority {
                nonce: 53,
                stage: BatchLifecycleStage::Built
            })
        );

        authority.record_encode(53).unwrap();
        authority.record_recurrent(53).unwrap();
        authority.record_selection(53).unwrap();
        authority.record_eligibility(53).unwrap();
        authority.submission_succeeded(53, &[(3, 7, 0)]).unwrap();
        assert_eq!(authority.active_sides.get(&(3, 7)), Some(&0));
        assert_eq!(authority.pending, None);
    }

    #[test]
    fn recording_failure_rolls_the_exact_nonce_back_to_built() {
        let mut authority = BatchAuthority::default();
        authority.active_sides.insert((5, 9), 0);
        authority.begin(77).unwrap();
        authority.record_encode(77).unwrap();

        // This is the rollback used if either private command-recording stage
        // returns an error before Queue::submit.
        authority.recording_failed(77).unwrap();
        assert_eq!(authority.active_sides.get(&(5, 9)), Some(&0));
        assert_eq!(
            authority.pending,
            Some(PendingBatchAuthority {
                nonce: 77,
                stage: BatchLifecycleStage::Built
            })
        );
        authority.record_encode(77).unwrap();
        authority.record_recurrent(77).unwrap();
    }

    #[test]
    fn post_submit_failure_poison_rejects_retry_abandon_and_new_batches() {
        let mut authority = BatchAuthority::default();
        authority.active_sides.insert((8, 11), 1);
        authority.begin(91).unwrap();
        authority.record_encode(91).unwrap();
        authority.record_recurrent(91).unwrap();
        authority.submission_indeterminate(91).unwrap();

        assert_eq!(authority.active_sides.get(&(8, 11)), Some(&1));
        assert_eq!(authority.poisoned_nonce, Some(91));
        assert_eq!(authority.pending.unwrap().nonce, 91);
        assert_eq!(
            authority.record_encode(91),
            Err(GpuClosedLoopError::SubmissionFailed)
        );
        assert_eq!(
            authority.abandon_unsubmitted(91),
            Err(GpuClosedLoopError::SubmissionFailed)
        );
        assert_eq!(
            authority.submission_succeeded(91, &[(8, 11, 0)]),
            Err(GpuClosedLoopError::SubmissionFailed)
        );
        assert_eq!(
            authority.begin(92),
            Err(GpuClosedLoopError::SubmissionFailed)
        );
        assert_eq!(authority.active_sides.get(&(8, 11)), Some(&1));
    }
}
