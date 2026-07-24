//! Candidate-local episodic-context ABI and production WGSL entry point.

use alife_core::{
    FinalizedMemoryRecall, MemoryChannelPlan, PerceptionBaseDigest, PerceptionContextDigest,
    PerceptionFrame, PerceptionFrameDigest, Tick,
};
use bytemuck::{Pod, Zeroable};

use crate::{GpuBrainSlot, GpuClosedLoopError};

pub const GPU_MEMORY_CONTEXT_HEADER_WORDS: usize = 16;
pub const GPU_CANDIDATE_MEMORY_RECORD_WORDS: usize = 16;
pub const GPU_MEMORY_CHANNEL_PLAN_WORDS: usize = 8;

pub const CLOSED_LOOP_MEMORY_CONTEXT_WGSL: &str = concat!(
    include_str!("../shaders/closed_loop_abi.wgsl"),
    include_str!("../shaders/closed_loop_memory_context.wgsl")
);

/// Host-only identity binding for one exact dynamic perception header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuPerceptionFrameBinding {
    pub perception_header_index: u32,
    pub slot: u32,
    pub slot_generation: u32,
    pub tick: Tick,
    pub candidate_count: u16,
    pub base_frame_digest: PerceptionBaseDigest,
    pub context_digest: PerceptionContextDigest,
    pub final_frame_digest: PerceptionFrameDigest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GpuMemoryOffsetDomain {
    Local,
    Rebased,
}

/// Host receipt proving that one finalized memory context was bound to the
/// exact perception row consumed by the GPU dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuMemoryContextDispatchReceipt {
    pub slot: u32,
    pub slot_generation: u32,
    pub perception_header_index: u32,
    pub base_frame_digest: PerceptionBaseDigest,
    pub context_digest: PerceptionContextDigest,
    pub final_frame_digest: PerceptionFrameDigest,
    pub candidate_count: u16,
}

/// Finalized candidate memory rows plus the exact frame identities they encode.
#[derive(Debug, Clone, PartialEq)]
pub struct GpuMemoryContextUpload {
    pub header: GpuMemoryContextHeader,
    pub records: Vec<GpuCandidateMemoryRecord>,
    pub base_frame_digest: PerceptionBaseDigest,
    pub context_digest: PerceptionContextDigest,
    pub final_frame_digest: PerceptionFrameDigest,
    pub perception_binding: GpuPerceptionFrameBinding,
    offset_domain: GpuMemoryOffsetDomain,
}

impl GpuMemoryContextUpload {
    pub fn try_from_finalized(
        frame: &PerceptionFrame,
        recall: &FinalizedMemoryRecall,
        perception_binding: GpuPerceptionFrameBinding,
        slot: &GpuBrainSlot,
    ) -> Result<Self, GpuClosedLoopError> {
        recall
            .validate_for_frame(frame)
            .map_err(|_| GpuClosedLoopError::MalformedUpload)?;
        let upload = Self::build_local(frame, recall, perception_binding, slot)?;
        upload.validate_against(frame, recall, slot)?;
        Ok(upload)
    }

    pub fn validate_against(
        &self,
        frame: &PerceptionFrame,
        recall: &FinalizedMemoryRecall,
        slot: &GpuBrainSlot,
    ) -> Result<(), GpuClosedLoopError> {
        recall
            .validate_for_frame(frame)
            .map_err(|_| GpuClosedLoopError::MalformedUpload)?;
        let expected = Self::build_local(frame, recall, self.perception_binding, slot)?;
        if self == &expected {
            Ok(())
        } else {
            Err(GpuClosedLoopError::MalformedUpload)
        }
    }

    pub(crate) fn validate_for_frame_and_slot(
        &self,
        frame: &PerceptionFrame,
        slot: &GpuBrainSlot,
    ) -> Result<(), GpuClosedLoopError> {
        if self.offset_domain != GpuMemoryOffsetDomain::Local {
            return Err(GpuClosedLoopError::InvalidOffsetDomain);
        }
        let record = slot.record();
        let profile = frame.profile_provenance().identity();
        let candidate_count = u16::try_from(frame.candidates().len())
            .map_err(|_| GpuClosedLoopError::CapacityExceeded)?;
        if self.header.schema_version == 0
            || self.header.class_id != record.class_id
            || self.header.slot != record.slot
            || self.header.slot_generation != record.slot_generation
            || self.header.tick() != frame.tick().raw()
            || self.header.candidate_count != u32::from(candidate_count)
            || self.header.memory_context_offset != 0
            || self.header.candidate_offset != 16
            || self.header.profile_id != u32::from(profile.profile_id.raw())
            || self.header.profile_schema_version != u32::from(profile.profile_schema_version)
            || self.header.sensory_abi_version != u32::from(profile.sensory_abi_version)
            || self.header.brain_slot_index != slot.brain_slot_index()
            || self.header.decoder_learning_input_offset != 77
            || self.header.perception_header_index
                != self.perception_binding.perception_header_index
            || self.header.reserved != 0
            || self.perception_binding.slot != record.slot
            || self.perception_binding.slot_generation != record.slot_generation
            || self.perception_binding.tick != frame.tick()
            || self.perception_binding.candidate_count != candidate_count
            || self.base_frame_digest != frame.base_digest()
            || self.context_digest != frame.context().canonical_digest()
            || self.final_frame_digest != frame.frame_digest()
            || self.perception_binding.base_frame_digest != self.base_frame_digest
            || self.perception_binding.context_digest != self.context_digest
            || self.perception_binding.final_frame_digest != self.final_frame_digest
            || self.records.len() != frame.candidates().len()
        {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        let mut reencoded = Vec::with_capacity(self.records.len() * 16);
        for (index, row) in self.records.iter().enumerate() {
            if row.candidate_index
                != u32::try_from(index).map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?
                || row
                    .target_latent
                    .iter()
                    .chain(row.family_value.iter())
                    .chain([&row.target_confidence, &row.family_confidence])
                    .any(|value| !value.is_finite())
            {
                return Err(GpuClosedLoopError::NonFinitePayload);
            }
            reencoded.extend_from_slice(&row.target_latent);
            reencoded.extend_from_slice(&row.family_value);
            reencoded.push(row.target_confidence);
            reencoded.push(row.family_confidence);
            reencoded.push((row.source_counts_packed & 0xffff) as f32);
            reencoded.push((row.source_counts_packed >> 16) as f32);
        }
        if reencoded != frame.context().values() {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        Ok(())
    }

    pub(crate) fn rebase_for_batch(
        &mut self,
        frame: &PerceptionFrame,
        slot: &GpuBrainSlot,
        perception_binding: GpuPerceptionFrameBinding,
        memory_context_offset: u32,
        candidate_offset: u32,
        decoder_learning_input_offset: u32,
    ) -> Result<GpuMemoryContextDispatchReceipt, GpuClosedLoopError> {
        self.validate_for_frame_and_slot(frame, slot)?;
        if perception_binding.slot != self.perception_binding.slot
            || perception_binding.slot_generation != self.perception_binding.slot_generation
            || perception_binding.tick != self.perception_binding.tick
            || perception_binding.candidate_count != self.perception_binding.candidate_count
            || perception_binding.base_frame_digest != self.base_frame_digest
            || perception_binding.context_digest != self.context_digest
            || perception_binding.final_frame_digest != self.final_frame_digest
            || memory_context_offset == 0
            || candidate_offset == 0
            || decoder_learning_input_offset == 0
        {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        self.header.memory_context_offset = memory_context_offset;
        self.header.candidate_offset = candidate_offset;
        self.header.decoder_learning_input_offset = decoder_learning_input_offset;
        self.header.perception_header_index = perception_binding.perception_header_index;
        self.perception_binding = perception_binding;
        self.offset_domain = GpuMemoryOffsetDomain::Rebased;
        Ok(GpuMemoryContextDispatchReceipt {
            slot: self.header.slot,
            slot_generation: self.header.slot_generation,
            perception_header_index: self.header.perception_header_index,
            base_frame_digest: self.base_frame_digest,
            context_digest: self.context_digest,
            final_frame_digest: self.final_frame_digest,
            candidate_count: u16::try_from(self.header.candidate_count)
                .map_err(|_| GpuClosedLoopError::CapacityExceeded)?,
        })
    }

    fn build_local(
        frame: &PerceptionFrame,
        recall: &FinalizedMemoryRecall,
        perception_binding: GpuPerceptionFrameBinding,
        slot: &GpuBrainSlot,
    ) -> Result<Self, GpuClosedLoopError> {
        let record = slot.record();
        let profile = frame.profile_provenance().identity();
        let candidate_count = u16::try_from(frame.candidates().len())
            .map_err(|_| GpuClosedLoopError::CapacityExceeded)?;
        if perception_binding.slot != record.slot
            || perception_binding.slot_generation != record.slot_generation
            || perception_binding.tick != frame.tick()
            || perception_binding.candidate_count != candidate_count
            || perception_binding.base_frame_digest != frame.base_digest()
            || perception_binding.context_digest != frame.context().canonical_digest()
            || perception_binding.final_frame_digest != frame.frame_digest()
        {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        let records = recall
            .context()
            .candidates
            .iter()
            .enumerate()
            .map(|(index, context)| {
                if usize::from(context.candidate_index) != index {
                    return Err(GpuClosedLoopError::MalformedUpload);
                }
                Ok(GpuCandidateMemoryRecord {
                    candidate_index: u32::try_from(index)
                        .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?,
                    target_confidence: context.target_confidence.raw(),
                    family_confidence: context.family_confidence.raw(),
                    source_counts_packed: u32::from(context.target_source_count)
                        | (u32::from(context.family_source_count) << 16),
                    target_latent: context.target_latent,
                    family_value: context.family_value,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut reencoded = Vec::with_capacity(records.len() * 16);
        for row in &records {
            reencoded.extend_from_slice(&row.target_latent);
            reencoded.extend_from_slice(&row.family_value);
            reencoded.push(row.target_confidence);
            reencoded.push(row.family_confidence);
            reencoded.push((row.source_counts_packed & 0xffff) as f32);
            reencoded.push((row.source_counts_packed >> 16) as f32);
        }
        if reencoded != frame.context().values() {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        let tick = frame.tick().raw();
        Ok(Self {
            header: GpuMemoryContextHeader {
                schema_version: u32::from(recall.context().schema_version),
                class_id: record.class_id,
                slot: record.slot,
                slot_generation: record.slot_generation,
                tick_lo: tick as u32,
                tick_hi: (tick >> 32) as u32,
                candidate_count: u32::from(candidate_count),
                memory_context_offset: 0,
                candidate_offset: 16,
                profile_id: u32::from(profile.profile_id.raw()),
                profile_schema_version: u32::from(profile.profile_schema_version),
                sensory_abi_version: u32::from(profile.sensory_abi_version),
                brain_slot_index: slot.brain_slot_index(),
                decoder_learning_input_offset: 77,
                perception_header_index: perception_binding.perception_header_index,
                reserved: 0,
            },
            records,
            base_frame_digest: frame.base_digest(),
            context_digest: frame.context().canonical_digest(),
            final_frame_digest: frame.frame_digest(),
            perception_binding,
            offset_domain: GpuMemoryOffsetDomain::Local,
        })
    }
}

/// One finalized candidate-local retrieval row. It contains no raw entity ID.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct GpuCandidateMemoryRecord {
    pub candidate_index: u32,
    pub target_confidence: f32,
    pub family_confidence: f32,
    pub source_counts_packed: u32,
    pub target_latent: [f32; 8],
    pub family_value: [f32; 4],
}

impl GpuCandidateMemoryRecord {
    pub fn words(&self) -> &[u32; GPU_CANDIDATE_MEMORY_RECORD_WORDS] {
        bytemuck::cast_ref(self)
    }

    pub fn from_words(words: &[u32]) -> Result<Self, GpuClosedLoopError> {
        if words.len() != GPU_CANDIDATE_MEMORY_RECORD_WORDS {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        Ok(bytemuck::pod_read_unaligned(bytemuck::cast_slice(words)))
    }
}

/// Dynamic dispatch identity for one frame's finalized memory context.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct GpuMemoryContextHeader {
    pub schema_version: u32,
    pub class_id: u32,
    pub slot: u32,
    pub slot_generation: u32,
    pub tick_lo: u32,
    pub tick_hi: u32,
    pub candidate_count: u32,
    pub memory_context_offset: u32,
    pub candidate_offset: u32,
    pub profile_id: u32,
    pub profile_schema_version: u32,
    pub sensory_abi_version: u32,
    pub brain_slot_index: u32,
    pub decoder_learning_input_offset: u32,
    pub perception_header_index: u32,
    pub reserved: u32,
}

impl GpuMemoryContextHeader {
    pub fn words(&self) -> &[u32; GPU_MEMORY_CONTEXT_HEADER_WORDS] {
        bytemuck::cast_ref(self)
    }

    pub fn tick(&self) -> u64 {
        u64::from(self.tick_lo) | (u64::from(self.tick_hi) << 32)
    }

    pub fn from_words(words: &[u32]) -> Result<Self, GpuClosedLoopError> {
        if words.len() != GPU_MEMORY_CONTEXT_HEADER_WORDS {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        Ok(bytemuck::pod_read_unaligned(bytemuck::cast_slice(words)))
    }
}

/// Immutable per-phenotype memory-channel plan uploaded once with the phenotype.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct GpuMemoryChannelPlan {
    pub schema_version: u32,
    pub target_latent_lane_start: u32,
    pub family_value_lane_start: u32,
    pub decoder_input_stride: u32,
    pub max_candidate_gain: f32,
    pub memory_decoder_synapse_count: u32,
    pub reserved: [u32; 2],
}

impl GpuMemoryChannelPlan {
    pub fn words(&self) -> &[u32; GPU_MEMORY_CHANNEL_PLAN_WORDS] {
        bytemuck::cast_ref(self)
    }

    pub fn from_words(words: &[u32]) -> Result<Self, GpuClosedLoopError> {
        if words.len() != GPU_MEMORY_CHANNEL_PLAN_WORDS {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        Ok(bytemuck::pod_read_unaligned(bytemuck::cast_slice(words)))
    }
}

impl TryFrom<&MemoryChannelPlan> for GpuMemoryChannelPlan {
    type Error = GpuClosedLoopError;

    fn try_from(plan: &MemoryChannelPlan) -> Result<Self, Self::Error> {
        plan.validate_contract()
            .map_err(|_| GpuClosedLoopError::MalformedUpload)?;
        if !plan.max_candidate_gain().is_finite() {
            return Err(GpuClosedLoopError::NonFinitePayload);
        }
        Ok(Self {
            schema_version: u32::from(plan.schema_version()),
            target_latent_lane_start: plan.target_latent_lane_start(),
            family_value_lane_start: plan.family_value_lane_start(),
            decoder_input_stride: plan.decoder_input_stride(),
            max_candidate_gain: plan.max_candidate_gain(),
            memory_decoder_synapse_count: plan.memory_decoder_synapse_count(),
            reserved: [0; 2],
        })
    }
}
