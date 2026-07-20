//! Production GPU checkpoint validation, exact slot snapshot, and atomic restore.
//!
//! This module is an explicit save/restore boundary. Active neural ticks never
//! call it, and no CPU neural execution or fallback is implemented here.

use std::mem::size_of;

use alife_core::{
    compute_gpu_sleep_output_weight_digest, ActionId, BoundedReplayBatch, BrainPhenotype,
    CandidateActionFamily, CandidateFeatureDigest, CanonicalDigestBuilder, ConsolidationIntent,
    ConsolidationStagedOutput, GpuConsolidationRequest, LearningSequenceGuard, OrganismId,
    OutcomeCreditReplayKey, PerceptionFrameDigest, PhenotypeHash, ScaffoldContractError,
    SchemaVersions, Tick, Validate,
};
use bytemuck::Zeroable;

use crate::{
    build_sleep_upload, eligibility_reset_digest, map_gpu_contract_error,
    pack_candidate_index_and_family, replay_reset_digest, reset_word_count, sleep_commit_key,
    GpuBrainHandle, GpuClosedLoopBackend, GpuPendingEligibilityRecord, GpuReplayEventRecord,
    GpuReplaySynapseSpanRecord, GpuSleepCompletionRecord, GpuSleepJobState, GpuSleepStagingReceipt,
    GpuSlotLearningStateRecord, PendingEligibilityIdentity, PendingEligibilityReceipt,
};

pub const GPU_BRAIN_CHECKPOINT_SCHEMA_VERSION: u16 = 1;

const CHECKPOINT_DIGEST_DOMAIN: &[u8] = b"alife.gpu.brain-checkpoint.v1";
const COMPLETED_STAGING_DIGEST_DOMAIN: &[u8] = b"alife.gpu.completed-sleep-staging.v1";
const REPLAY_EVENT_WORDS: usize = size_of::<GpuReplayEventRecord>() / 4;
const LEARNING_STATE_WORDS: usize = size_of::<GpuSlotLearningStateRecord>() / 4;
const SLEEP_DIAGNOSTIC_Q12: f32 = 4096.0;

/// Bounded post-dispatch readback used only by causal evidence collection.
///
/// Gameplay consumes the compact winning-selection record instead. This
/// snapshot exists so offline acceptance can prove how one finalized memory
/// context changed every candidate without creating host-side policy
/// authority.
#[derive(Debug, Clone, PartialEq)]
pub struct GpuCandidateLogitEvidenceSnapshot {
    pub handle: GpuBrainHandle,
    pub dispatch_generation: u64,
    pub originating_tick: Tick,
    pub frame_digest: PerceptionFrameDigest,
    pub logits: Vec<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingEligibilityRestoreParts {
    dispatch_generation: u64,
    originating_tick: Tick,
    frame_digest: PerceptionFrameDigest,
    active_activation_side: u8,
    candidate_index: u16,
    action_id: ActionId,
    action_family: CandidateActionFamily,
    candidate_feature_digest: CandidateFeatureDigest,
    active_eligibility_generation: u64,
    staging_eligibility_generation: u64,
}

impl PendingEligibilityRestoreParts {
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        dispatch_generation: u64,
        originating_tick: Tick,
        frame_digest: PerceptionFrameDigest,
        active_activation_side: u8,
        candidate_index: u16,
        action_id: ActionId,
        action_family: CandidateActionFamily,
        candidate_feature_digest: CandidateFeatureDigest,
        active_eligibility_generation: u64,
        staging_eligibility_generation: u64,
    ) -> Result<Self, ScaffoldContractError> {
        action_id.validate()?;
        if dispatch_generation == 0
            || frame_digest == PerceptionFrameDigest([0; 4])
            || active_activation_side > 1
            || candidate_feature_digest == CandidateFeatureDigest([0; 2])
            || active_eligibility_generation == 0
            || active_eligibility_generation.checked_add(1) != Some(staging_eligibility_generation)
        {
            return Err(ScaffoldContractError::LearningEvidenceMismatch);
        }
        Ok(Self {
            dispatch_generation,
            originating_tick,
            frame_digest,
            active_activation_side,
            candidate_index,
            action_id,
            action_family,
            candidate_feature_digest,
            active_eligibility_generation,
            staging_eligibility_generation,
        })
    }

    pub const fn dispatch_generation(self) -> u64 {
        self.dispatch_generation
    }

    pub const fn originating_tick(self) -> Tick {
        self.originating_tick
    }

    pub const fn frame_digest(self) -> PerceptionFrameDigest {
        self.frame_digest
    }

    pub const fn active_activation_side(self) -> u8 {
        self.active_activation_side
    }

    pub const fn candidate_index(self) -> u16 {
        self.candidate_index
    }

    pub const fn action_id(self) -> ActionId {
        self.action_id
    }

    pub const fn action_family(self) -> CandidateActionFamily {
        self.action_family
    }

    pub const fn candidate_feature_digest(self) -> CandidateFeatureDigest {
        self.candidate_feature_digest
    }

    pub const fn active_eligibility_generation(self) -> u64 {
        self.active_eligibility_generation
    }

    pub const fn staging_eligibility_generation(self) -> u64 {
        self.staging_eligibility_generation
    }

    fn write_canonical(self, digest: &mut CanonicalDigestBuilder) {
        digest.write_u64(self.dispatch_generation);
        digest.write_u64(self.originating_tick.raw());
        write_digest4(digest, self.frame_digest.0);
        digest.write_u8(self.active_activation_side);
        digest.write_u16(self.candidate_index);
        digest.write_u32(self.action_id.raw());
        digest.write_u8(self.action_family.raw());
        write_digest2(digest, self.candidate_feature_digest.0);
        digest.write_u64(self.active_eligibility_generation);
        digest.write_u64(self.staging_eligibility_generation);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuBrainCheckpointParts {
    pub schema_version: u16,
    pub organism_id: OrganismId,
    pub phenotype_hash: PhenotypeHash,
    pub checkpoint_tick: Tick,
    pub active_activation_side: u8,
    pub logical_dispatch_generation: u64,
    pub activation_a_bits: Vec<u32>,
    pub activation_b_bits: Vec<u32>,
    pub neuron_homeostasis_bits: Vec<u32>,
    pub active_weight_generation: u64,
    pub active_weight_bank: u8,
    pub lifetime_bank_0_bits: Vec<u32>,
    pub lifetime_bank_1_bits: Vec<u32>,
    pub fast_bank_0_bits: Vec<u32>,
    pub fast_bank_1_bits: Vec<u32>,
    pub active_eligibility_generation: u64,
    pub inactive_eligibility_generation: u64,
    pub active_eligibility_bank: u8,
    pub learning_transaction_generation: u64,
    pub recurrent_eligibility_bank_0_bits: Vec<u32>,
    pub recurrent_eligibility_bank_1_bits: Vec<u32>,
    pub decoder_eligibility_bank_0_bits: Vec<u32>,
    pub decoder_eligibility_bank_1_bits: Vec<u32>,
    pub replay_journal_generation: u64,
    pub replay_journal_cursor: u32,
    pub replay_journal_event_count: u32,
    pub replay_events: Vec<GpuReplayEventRecord>,
    pub replay_spans: Vec<GpuReplaySynapseSpanRecord>,
    pub replay_samples: Vec<u32>,
    pub last_learning_replay_key: Option<OutcomeCreditReplayKey>,
    pub pending_eligibility: Option<PendingEligibilityRestoreParts>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuBrainCheckpointSnapshot {
    parts: GpuBrainCheckpointParts,
    canonical_digest: [u64; 4],
}

impl GpuBrainCheckpointSnapshot {
    pub fn try_from_parts(parts: GpuBrainCheckpointParts) -> Result<Self, ScaffoldContractError> {
        validate_checkpoint_parts(&parts)?;
        let canonical_digest = checkpoint_digest(&parts)?;
        Ok(Self {
            parts,
            canonical_digest,
        })
    }

    pub const fn canonical_digest(&self) -> [u64; 4] {
        self.canonical_digest
    }

    pub fn into_parts(self) -> GpuBrainCheckpointParts {
        self.parts
    }

    fn validate(&self) -> Result<(), ScaffoldContractError> {
        validate_checkpoint_parts(&self.parts)?;
        if self.canonical_digest != checkpoint_digest(&self.parts)? {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuBrainRestoreRequest {
    snapshot: GpuBrainCheckpointSnapshot,
}

impl GpuBrainRestoreRequest {
    pub fn try_new(snapshot: GpuBrainCheckpointSnapshot) -> Result<Self, ScaffoldContractError> {
        snapshot.validate()?;
        Ok(Self { snapshot })
    }

    fn into_snapshot(self) -> GpuBrainCheckpointSnapshot {
        self.snapshot
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuBrainRestoreReceipt {
    pub handle: GpuBrainHandle,
    pub pending_eligibility: Option<PendingEligibilityReceipt>,
    pub active_weight_generation: u64,
    pub active_weight_bank: u8,
    pub active_eligibility_generation: u64,
    pub active_eligibility_bank: u8,
    pub learning_transaction_generation: u64,
    pub replay_journal_generation: u64,
    pub replay_journal_cursor: u32,
    pub replay_journal_event_count: u32,
    pub checkpoint_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuCompletedSleepStagingInputParts {
    pub output_weight_generation: u64,
    pub output_weight_bank: u8,
    pub lifetime_bank_0_bits: Vec<u32>,
    pub lifetime_bank_1_bits: Vec<u32>,
    pub fast_bank_0_bits: Vec<u32>,
    pub fast_bank_1_bits: Vec<u32>,
    pub eligibility_reset_generation: u64,
    pub output_eligibility_bank: u8,
    pub recurrent_eligibility_bank_0_bits: Vec<u32>,
    pub recurrent_eligibility_bank_1_bits: Vec<u32>,
    pub decoder_eligibility_bank_0_bits: Vec<u32>,
    pub decoder_eligibility_bank_1_bits: Vec<u32>,
    pub replay_journal_generation: u64,
    pub replay_journal_cursor: u32,
    pub replay_journal_event_count: u32,
    pub replay_events: Vec<GpuReplayEventRecord>,
    pub replay_spans: Vec<GpuReplaySynapseSpanRecord>,
    pub replay_samples: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuCompletedSleepStagingParts {
    parts: GpuCompletedSleepStagingInputParts,
    canonical_digest: [u64; 4],
}

impl GpuCompletedSleepStagingParts {
    pub fn try_from_parts(
        parts: GpuCompletedSleepStagingInputParts,
    ) -> Result<Self, ScaffoldContractError> {
        validate_completed_staging_parts(&parts)?;
        let canonical_digest = completed_staging_digest(&parts);
        Ok(Self {
            parts,
            canonical_digest,
        })
    }

    pub const fn canonical_digest(&self) -> [u64; 4] {
        self.canonical_digest
    }

    pub fn into_parts(self) -> GpuCompletedSleepStagingInputParts {
        self.parts
    }

    fn validate(&self) -> Result<(), ScaffoldContractError> {
        validate_completed_staging_parts(&self.parts)?;
        if self.canonical_digest != completed_staging_digest(&self.parts) {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        Ok(())
    }
}

fn validate_completed_staging_parts(
    parts: &GpuCompletedSleepStagingInputParts,
) -> Result<(), ScaffoldContractError> {
    let synapse_count = parts.lifetime_bank_0_bits.len();
    let recurrent_count = parts.recurrent_eligibility_bank_0_bits.len();
    let decoder_count = parts.decoder_eligibility_bank_0_bits.len();
    let event_capacity = parts.replay_events.len();
    let span_count = parts.replay_spans.len();
    if parts.output_weight_generation == 0
        || parts.output_weight_bank > 1
        || synapse_count == 0
        || parts.lifetime_bank_1_bits.len() != synapse_count
        || parts.fast_bank_0_bits.len() != synapse_count
        || parts.fast_bank_1_bits.len() != synapse_count
        || parts.eligibility_reset_generation == 0
        || parts.output_eligibility_bank > 1
        || recurrent_count == 0
        || decoder_count == 0
        || recurrent_count.checked_add(decoder_count) != Some(synapse_count)
        || parts.recurrent_eligibility_bank_1_bits.len() != recurrent_count
        || parts.decoder_eligibility_bank_1_bits.len() != decoder_count
        || parts.replay_journal_generation == 0
        || parts.replay_journal_cursor != 0
        || parts.replay_journal_event_count != 0
        || event_capacity == 0
        || event_capacity > 65_536
        || span_count == 0
        || parts.replay_samples.len() != event_capacity.saturating_mul(span_count)
    {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    for bits in [
        &parts.lifetime_bank_0_bits,
        &parts.lifetime_bank_1_bits,
        &parts.fast_bank_0_bits,
        &parts.fast_bank_1_bits,
        &parts.recurrent_eligibility_bank_0_bits,
        &parts.recurrent_eligibility_bank_1_bits,
        &parts.decoder_eligibility_bank_0_bits,
        &parts.decoder_eligibility_bank_1_bits,
    ] {
        validate_float_bits(bits)?;
    }
    if [
        &parts.recurrent_eligibility_bank_0_bits,
        &parts.recurrent_eligibility_bank_1_bits,
        &parts.decoder_eligibility_bank_0_bits,
        &parts.decoder_eligibility_bank_1_bits,
    ]
    .into_iter()
    .any(|words| words.iter().any(|word| *word != 0))
        || parts
            .replay_events
            .iter()
            .flat_map(|event| {
                bytemuck::cast_slice::<GpuReplayEventRecord, u32>(std::slice::from_ref(event))
            })
            .any(|word| *word != 0)
        || parts.replay_samples.iter().any(|word| *word != 0)
    {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    let mut previous_synapse = None;
    for (index, span) in parts.replay_spans.iter().enumerate() {
        let expected_start = index
            .checked_mul(event_capacity)
            .and_then(|value| u32::try_from(value).ok())
            .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
        if span.sample_start != expected_start
            || span.sample_count != 0
            || span.reserved != 0
            || usize::try_from(span.local_synapse_id)
                .ok()
                .is_none_or(|synapse| synapse >= synapse_count)
            || previous_synapse.is_some_and(|previous| previous >= span.local_synapse_id)
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        previous_synapse = Some(span.local_synapse_id);
    }
    Ok(())
}

fn completed_staging_digest(parts: &GpuCompletedSleepStagingInputParts) -> [u64; 4] {
    let mut digest = CanonicalDigestBuilder::new(COMPLETED_STAGING_DIGEST_DOMAIN);
    digest.write_u64(parts.output_weight_generation);
    digest.write_u8(parts.output_weight_bank);
    for words in [
        &parts.lifetime_bank_0_bits,
        &parts.lifetime_bank_1_bits,
        &parts.fast_bank_0_bits,
        &parts.fast_bank_1_bits,
    ] {
        write_word_sequence(&mut digest, words);
    }
    digest.write_u64(parts.eligibility_reset_generation);
    digest.write_u8(parts.output_eligibility_bank);
    for words in [
        &parts.recurrent_eligibility_bank_0_bits,
        &parts.recurrent_eligibility_bank_1_bits,
        &parts.decoder_eligibility_bank_0_bits,
        &parts.decoder_eligibility_bank_1_bits,
    ] {
        write_word_sequence(&mut digest, words);
    }
    digest.write_u64(parts.replay_journal_generation);
    digest.write_u32(parts.replay_journal_cursor);
    digest.write_u32(parts.replay_journal_event_count);
    digest.write_sequence_len(parts.replay_events.len());
    for event in &parts.replay_events {
        write_word_sequence(
            &mut digest,
            bytemuck::cast_slice(std::slice::from_ref(event)),
        );
    }
    digest.write_sequence_len(parts.replay_spans.len());
    for span in &parts.replay_spans {
        write_word_sequence(&mut digest, span.words());
    }
    write_word_sequence(&mut digest, &parts.replay_samples);
    digest.finish256()
}

fn validate_checkpoint_parts(parts: &GpuBrainCheckpointParts) -> Result<(), ScaffoldContractError> {
    parts.organism_id.validate()?;
    let neuron_count = parts.activation_a_bits.len();
    let synapse_count = parts.lifetime_bank_0_bits.len();
    let recurrent_count = parts.recurrent_eligibility_bank_0_bits.len();
    let decoder_count = parts.decoder_eligibility_bank_0_bits.len();
    let event_capacity = parts.replay_events.len();
    let span_count = parts.replay_spans.len();
    let expected_sample_capacity = event_capacity
        .checked_mul(span_count)
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
    if parts.schema_version != GPU_BRAIN_CHECKPOINT_SCHEMA_VERSION
        || parts.phenotype_hash == PhenotypeHash([0; 4])
        || neuron_count == 0
        || parts.activation_b_bits.len() != neuron_count
        || parts.neuron_homeostasis_bits.len() != neuron_count.saturating_mul(2)
        || parts.active_activation_side > 1
        || parts.logical_dispatch_generation == 0
        || synapse_count == 0
        || parts.lifetime_bank_1_bits.len() != synapse_count
        || parts.fast_bank_0_bits.len() != synapse_count
        || parts.fast_bank_1_bits.len() != synapse_count
        || recurrent_count == 0
        || decoder_count == 0
        || recurrent_count.checked_add(decoder_count) != Some(synapse_count)
        || parts.recurrent_eligibility_bank_1_bits.len() != recurrent_count
        || parts.decoder_eligibility_bank_1_bits.len() != decoder_count
        || parts.active_weight_generation == 0
        || parts.active_weight_bank > 1
        || parts.active_eligibility_generation == 0
        || parts.active_eligibility_bank > 1
        || (parts.inactive_eligibility_generation != 0
            && parts.active_eligibility_generation.checked_add(1)
                != Some(parts.inactive_eligibility_generation))
        || parts.learning_transaction_generation == 0
        || parts.replay_journal_generation == 0
        || event_capacity == 0
        || event_capacity > 65_536
        || span_count == 0
        || parts.replay_samples.len() != expected_sample_capacity
        || usize::try_from(parts.replay_journal_event_count)
            .ok()
            .is_none_or(|count| count > event_capacity)
        || usize::try_from(parts.replay_journal_cursor)
            .ok()
            .is_none_or(|cursor| cursor >= event_capacity)
        || (usize::try_from(parts.replay_journal_event_count).ok() != Some(event_capacity)
            && parts.replay_journal_cursor != parts.replay_journal_event_count)
    {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    for bits in [
        &parts.activation_a_bits,
        &parts.activation_b_bits,
        &parts.neuron_homeostasis_bits,
        &parts.lifetime_bank_0_bits,
        &parts.lifetime_bank_1_bits,
        &parts.fast_bank_0_bits,
        &parts.fast_bank_1_bits,
        &parts.recurrent_eligibility_bank_0_bits,
        &parts.recurrent_eligibility_bank_1_bits,
        &parts.decoder_eligibility_bank_0_bits,
        &parts.decoder_eligibility_bank_1_bits,
    ] {
        validate_float_bits(bits)?;
    }
    validate_physical_replay(parts, synapse_count)?;
    if let Some(key) = parts.last_learning_replay_key {
        key.organism_id.validate()?;
        key.sequence_id.validate()?;
        if key.organism_id != parts.organism_id || key.phenotype_hash != parts.phenotype_hash {
            return Err(ScaffoldContractError::LearningEvidenceMismatch);
        }
    }
    match parts.pending_eligibility {
        Some(pending)
            if pending.dispatch_generation() == parts.logical_dispatch_generation
                && pending.active_activation_side() == parts.active_activation_side
                && pending.active_eligibility_generation()
                    == parts.active_eligibility_generation
                && pending.staging_eligibility_generation()
                    == parts.inactive_eligibility_generation => {}
        Some(_) => return Err(ScaffoldContractError::LearningEvidenceMismatch),
        None if parts.inactive_eligibility_generation == 0 => {}
        None => return Err(ScaffoldContractError::LearningEvidenceMismatch),
    }
    Ok(())
}

fn validate_physical_replay(
    parts: &GpuBrainCheckpointParts,
    synapse_count: usize,
) -> Result<(), ScaffoldContractError> {
    let capacity = parts.replay_events.len();
    let event_count = usize::try_from(parts.replay_journal_event_count)
        .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?;
    let cursor = usize::try_from(parts.replay_journal_cursor)
        .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?;
    let oldest = if event_count == capacity { cursor } else { 0 };
    let physical_order = (0..event_count)
        .map(|offset| (oldest + offset) % capacity)
        .collect::<Vec<_>>();
    let mut previous_sequence = None;
    for physical in &physical_order {
        let event = parts
            .replay_events
            .get(*physical)
            .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
        let sequence = validate_replay_event(event)?;
        if previous_sequence.is_some_and(|previous| previous >= sequence) {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        previous_sequence = Some(sequence);
    }
    for event in &parts.replay_events {
        validate_float_values(&[
            event.reward_prediction_error,
            event.pain,
            event.homeostatic_improvement,
            event.frustration,
            event.novelty,
            event.modulator_value,
        ])?;
    }
    let mut previous_synapse = None;
    for (capture_index, span) in parts.replay_spans.iter().enumerate() {
        let expected_start = capture_index
            .checked_mul(capacity)
            .and_then(|value| u32::try_from(value).ok())
            .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
        if span.reserved != 0
            || span.sample_start != expected_start
            || usize::try_from(span.sample_count).ok() != Some(event_count)
            || usize::try_from(span.local_synapse_id)
                .ok()
                .is_none_or(|synapse| synapse >= synapse_count)
            || previous_synapse.is_some_and(|previous| previous >= span.local_synapse_id)
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        previous_synapse = Some(span.local_synapse_id);
        for physical in &physical_order {
            let sample_index = usize::try_from(span.sample_start)
                .ok()
                .and_then(|start| start.checked_add(*physical))
                .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
            let packed = *parts
                .replay_samples
                .get(sample_index)
                .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
            let (sample_event, eligibility) = crate::unpack_replay_eligibility_sample(packed);
            if usize::from(sample_event) != *physical || eligibility == i16::MIN {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
            }
        }
    }
    Ok(())
}

fn validate_replay_event(event: &GpuReplayEventRecord) -> Result<u64, ScaffoldContractError> {
    let sequence = join_pair(event.sequence_id);
    let frame_digest = join_u32x8(event.frame_digest);
    let feature_digest = join_u32x4(event.candidate_feature_digest);
    ActionId(event.action_id).validate()?;
    let family = u8::try_from(event.family)
        .ok()
        .and_then(|raw| CandidateActionFamily::try_from_raw(raw).ok());
    validate_float_values(&[
        event.reward_prediction_error,
        event.pain,
        event.homeostatic_improvement,
        event.frustration,
        event.novelty,
        event.modulator_value,
    ])?;
    if sequence == 0
        || frame_digest == [0; 4]
        || feature_digest == [0; 2]
        || family.is_none()
        || [
            event.reward_prediction_error,
            event.pain,
            event.homeostatic_improvement,
            event.frustration,
            event.novelty,
            event.modulator_value,
        ]
        .iter()
        .any(|value| !(-1.0..=1.0).contains(value))
    {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    Ok(sequence)
}

fn checkpoint_digest(parts: &GpuBrainCheckpointParts) -> Result<[u64; 4], ScaffoldContractError> {
    let mut digest = CanonicalDigestBuilder::new(CHECKPOINT_DIGEST_DOMAIN);
    digest.write_u16(parts.schema_version);
    digest.write_u64(parts.organism_id.raw());
    write_digest4(&mut digest, parts.phenotype_hash.0);
    digest.write_u64(parts.checkpoint_tick.raw());
    digest.write_u8(parts.active_activation_side);
    digest.write_u64(parts.logical_dispatch_generation);
    for bits in [
        &parts.activation_a_bits,
        &parts.activation_b_bits,
        &parts.neuron_homeostasis_bits,
    ] {
        write_word_sequence(&mut digest, bits);
    }
    digest.write_u64(parts.active_weight_generation);
    digest.write_u8(parts.active_weight_bank);
    for bits in [
        &parts.lifetime_bank_0_bits,
        &parts.lifetime_bank_1_bits,
        &parts.fast_bank_0_bits,
        &parts.fast_bank_1_bits,
    ] {
        write_word_sequence(&mut digest, bits);
    }
    digest.write_u64(parts.active_eligibility_generation);
    digest.write_u64(parts.inactive_eligibility_generation);
    digest.write_u8(parts.active_eligibility_bank);
    digest.write_u64(parts.learning_transaction_generation);
    for bits in [
        &parts.recurrent_eligibility_bank_0_bits,
        &parts.recurrent_eligibility_bank_1_bits,
        &parts.decoder_eligibility_bank_0_bits,
        &parts.decoder_eligibility_bank_1_bits,
    ] {
        write_word_sequence(&mut digest, bits);
    }
    digest.write_u64(parts.replay_journal_generation);
    digest.write_u32(parts.replay_journal_cursor);
    digest.write_u32(parts.replay_journal_event_count);
    digest.write_sequence_len(parts.replay_events.len());
    for event in &parts.replay_events {
        write_word_sequence(
            &mut digest,
            bytemuck::cast_slice(std::slice::from_ref(event)),
        );
    }
    digest.write_sequence_len(parts.replay_spans.len());
    for span in &parts.replay_spans {
        write_word_sequence(&mut digest, span.words());
    }
    write_word_sequence(&mut digest, &parts.replay_samples);
    match parts.last_learning_replay_key {
        Some(key) => {
            digest.write_u8(1);
            digest.write_u64(key.organism_id.raw());
            write_digest4(&mut digest, key.phenotype_hash.0);
            digest.write_u64(key.sequence_id.raw());
        }
        None => digest.write_u8(0),
    }
    match parts.pending_eligibility {
        Some(pending) => {
            digest.write_u8(1);
            pending.write_canonical(&mut digest);
        }
        None => digest.write_u8(0),
    }
    Ok(digest.finish256())
}

fn validate_float_bits(bits: &[u32]) -> Result<(), ScaffoldContractError> {
    if bits
        .iter()
        .any(|bits| *bits == 0x8000_0000 || !f32::from_bits(*bits).is_finite())
    {
        return Err(ScaffoldContractError::NonFiniteFloat);
    }
    Ok(())
}

fn validate_float_values(values: &[f32]) -> Result<(), ScaffoldContractError> {
    if values
        .iter()
        .any(|value| !value.is_finite() || value.to_bits() == 0x8000_0000)
    {
        return Err(ScaffoldContractError::NonFiniteFloat);
    }
    Ok(())
}

fn write_word_sequence(digest: &mut CanonicalDigestBuilder, words: &[u32]) {
    digest.write_sequence_len(words.len());
    for word in words {
        digest.write_u32(*word);
    }
}

fn write_digest4(digest: &mut CanonicalDigestBuilder, value: [u64; 4]) {
    for word in value {
        digest.write_u64(word);
    }
}

fn write_digest2(digest: &mut CanonicalDigestBuilder, value: [u64; 2]) {
    for word in value {
        digest.write_u64(word);
    }
}

const fn join_pair(value: [u32; 2]) -> u64 {
    value[0] as u64 | ((value[1] as u64) << 32)
}

const fn split_pair(value: u64) -> [u32; 2] {
    [value as u32, (value >> 32) as u32]
}

const fn join_u32x8(value: [u32; 8]) -> [u64; 4] {
    [
        join_pair([value[0], value[1]]),
        join_pair([value[2], value[3]]),
        join_pair([value[4], value[5]]),
        join_pair([value[6], value[7]]),
    ]
}

const fn join_u32x4(value: [u32; 4]) -> [u64; 2] {
    [
        join_pair([value[0], value[1]]),
        join_pair([value[2], value[3]]),
    ]
}

const fn split_u64x2(value: [u64; 2]) -> [u32; 4] {
    let a = split_pair(value[0]);
    let b = split_pair(value[1]);
    [a[0], a[1], b[0], b[1]]
}

fn local_slice<'a>(
    words: &'a [u32],
    base: u32,
    range: &std::ops::Range<u32>,
) -> Result<&'a [u32], ScaffoldContractError> {
    let start = range
        .start
        .checked_sub(base)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
    let end = range
        .end
        .checked_sub(base)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
    words
        .get(start..end)
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)
}

fn exact_bits(
    words: &[u32],
    base: u32,
    range: &std::ops::Range<u32>,
    count: usize,
) -> Result<Vec<u32>, ScaffoldContractError> {
    Ok(local_slice(words, base, range)?
        .get(..count)
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?
        .to_vec())
}

fn record_from_range<T: bytemuck::Pod + Copy>(
    words: &[u32],
    base: u32,
    range: &std::ops::Range<u32>,
) -> Result<T, ScaffoldContractError> {
    let expected = size_of::<T>() / 4;
    let slice = local_slice(words, base, range)?
        .get(..expected)
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
    Ok(bytemuck::pod_read_unaligned(bytemuck::cast_slice(slice)))
}

fn record_from_absolute_start<T: bytemuck::Pod + Copy>(
    words: &[u32],
    base: u32,
    absolute_start: u32,
) -> Result<T, ScaffoldContractError> {
    let start = absolute_start
        .checked_sub(base)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
    let end = start
        .checked_add(size_of::<T>() / 4)
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
    let slice = words
        .get(start..end)
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
    Ok(bytemuck::pod_read_unaligned(bytemuck::cast_slice(slice)))
}

fn records_from_range<T: bytemuck::Pod + Copy>(
    words: &[u32],
    base: u32,
    range: &std::ops::Range<u32>,
    count: usize,
) -> Result<Vec<T>, ScaffoldContractError> {
    let word_count = count
        .checked_mul(size_of::<T>() / 4)
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
    let slice = local_slice(words, base, range)?
        .get(..word_count)
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
    Ok(bytemuck::cast_slice(slice).to_vec())
}

fn pending_parts_from_receipt(
    receipt: PendingEligibilityReceipt,
) -> Result<PendingEligibilityRestoreParts, ScaffoldContractError> {
    let identity = receipt.identity();
    PendingEligibilityRestoreParts::try_new(
        identity.dispatch_generation(),
        identity.originating_tick(),
        identity.frame_digest(),
        identity.active_activation_side(),
        identity.candidate_index(),
        identity.action_id(),
        identity.action_family(),
        identity.candidate_feature_digest(),
        identity.active_eligibility_generation(),
        identity.staging_eligibility_generation(),
    )
}

fn restored_pending_record(
    handle: GpuBrainHandle,
    pending: PendingEligibilityRestoreParts,
) -> Result<(GpuPendingEligibilityRecord, PendingEligibilityReceipt), ScaffoldContractError> {
    let mut record = GpuPendingEligibilityRecord::template(
        handle.slot(),
        handle.generation(),
        pending.active_activation_side(),
        handle.phenotype_hash(),
        handle.organism_id(),
        pending.dispatch_generation(),
        pending.originating_tick(),
        pending.frame_digest(),
        pending.active_eligibility_generation(),
        pending.staging_eligibility_generation(),
    )?;
    record.candidate_index_and_family =
        pack_candidate_index_and_family(pending.candidate_index(), pending.action_family());
    record.action_id = pending.action_id().raw();
    record.candidate_feature_digest = split_u64x2(pending.candidate_feature_digest().0);
    let receipt = PendingEligibilityReceipt::from_gpu_record(
        record,
        handle.slot(),
        handle.organism_id(),
        handle.phenotype_hash(),
    )?;
    Ok((record, receipt))
}

impl GpuClosedLoopBackend {
    /// Reads the bounded candidate-logit row for an exact pending GPU
    /// transaction. This is an offline evidence/checkpoint boundary, never a
    /// gameplay arbitration path.
    pub fn candidate_logits_for_evidence(
        &mut self,
        handle: GpuBrainHandle,
        frame: &alife_core::PerceptionFrame,
        pending: &PendingEligibilityIdentity,
    ) -> Result<GpuCandidateLogitEvidenceSnapshot, ScaffoldContractError> {
        self.ensure_ready()?;
        self.validate_handle_backend(handle)?;
        frame.validate()?;
        let ranges = {
            let bucket = self
                .class_buckets
                .get(&handle.class_id().raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            if !bucket.contains(handle) {
                return Err(ScaffoldContractError::BrainOwnershipMismatch);
            }
            let resident = bucket.slots[handle.slot() as usize]
                .as_ref()
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let resident_pending = resident
                .pending_eligibility
                .ok_or(ScaffoldContractError::LearningEvidenceMismatch)?;
            if resident_pending.identity() != pending {
                return Err(ScaffoldContractError::LearningEvidenceMismatch);
            }
            resident.ranges.clone()
        };
        if frame.organism_id() != handle.organism_id()
            || pending.handle_generation() != handle.generation()
            || pending.phenotype_hash() != handle.phenotype_hash()
            || pending.originating_tick() != frame.tick()
            || pending.frame_digest() != frame.frame_digest()
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let selected = frame
            .candidates()
            .get(usize::from(pending.candidate_index()))
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        if selected.action_id != pending.action_id()
            || selected.family != pending.action_family()
            || selected.feature_digest()? != pending.candidate_feature_digest()
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let candidate_count = frame.candidates().len();
        let available = usize::try_from(
            ranges
                .layout
                .candidate_logit_words
                .end
                .saturating_sub(ranges.layout.candidate_logit_words.start),
        )
        .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
        if candidate_count == 0 || candidate_count > available {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let words = self.read_slot_mutable_words(handle, &ranges)?;
        let raw = local_slice(
            &words,
            ranges.mutable_state_words.start,
            &ranges.layout.candidate_logit_words,
        )?
        .get(..candidate_count)
        .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        let logits = raw
            .iter()
            .map(|bits| {
                let value = f32::from_bits(*bits);
                if !value.is_finite() || *bits == (-0.0_f32).to_bits() {
                    Err(ScaffoldContractError::NonFiniteFloat)
                } else {
                    Ok(value)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(GpuCandidateLogitEvidenceSnapshot {
            handle,
            dispatch_generation: pending.dispatch_generation(),
            originating_tick: pending.originating_tick(),
            frame_digest: pending.frame_digest(),
            logits,
        })
    }

    pub fn snapshot_brain(
        &mut self,
        handle: GpuBrainHandle,
        checkpoint_tick: Tick,
    ) -> Result<GpuBrainCheckpointSnapshot, ScaffoldContractError> {
        self.ensure_ready()?;
        self.validate_handle_backend(handle)?;
        let (brain_slot, ranges, resident_state, active_side) = {
            let bucket = self
                .class_buckets
                .get(&handle.class_id().raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            if !bucket.contains(handle) {
                return Err(ScaffoldContractError::BrainOwnershipMismatch);
            }
            let resident = bucket.slots[handle.slot() as usize]
                .as_ref()
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let side = bucket
                .pipelines
                .slot_active_side(handle.slot(), handle.generation())
                .map_err(map_gpu_contract_error)?;
            (
                resident.brain_slot.clone(),
                resident.ranges.clone(),
                (
                    resident.active_weight_generation,
                    resident.active_weight_bank,
                    resident.active_eligibility_generation,
                    resident.active_eligibility_bank,
                    resident.replay_journal_generation,
                    resident.transaction_generation,
                    resident.logical_dispatch_generation,
                    resident.learning_sequence_guard.last_committed(),
                    resident.pending_eligibility,
                    resident.pending_eligibility_record,
                ),
                side,
            )
        };
        let words = self.read_slot_mutable_words(handle, &ranges)?;
        let base = ranges.mutable_state_words.start;
        let state: GpuSlotLearningStateRecord =
            record_from_range(&words, base, &ranges.layout.learning_state_words)?;
        let active_weight_generation = join_pair([
            state.active_weight_generation_lo,
            state.active_weight_generation_hi,
        ]);
        let active_eligibility_generation = join_pair([
            state.active_eligibility_generation_lo,
            state.active_eligibility_generation_hi,
        ]);
        let inactive_eligibility_generation = join_pair([
            state.inactive_eligibility_generation_lo,
            state.inactive_eligibility_generation_hi,
        ]);
        let replay_generation = join_pair([state.replay_generation_lo, state.replay_generation_hi]);
        let transaction_generation = join_pair([
            state.transaction_generation_lo,
            state.transaction_generation_hi,
        ]);
        if state.schema_version != u32::from(SchemaVersions::CURRENT.learning.raw())
            || state.active_weight_bank > 1
            || state.active_eligibility_bank > 1
            || active_weight_generation != resident_state.0
            || state.active_weight_bank != u32::from(resident_state.1)
            || active_eligibility_generation != resident_state.2
            || state.active_eligibility_bank != u32::from(resident_state.3)
            || replay_generation != resident_state.4
            || transaction_generation != resident_state.5
            || state.replay_event_capacity == 0
            || state.replay_event_capacity > 65_536
            || state.replay_span_count == 0
            || state.replay_sample_capacity
                != state
                    .replay_event_capacity
                    .checked_mul(state.replay_span_count)
                    .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        let pending = match (resident_state.8, resident_state.9, state.pending_valid) {
            (Some(receipt), Some(record), 1) => {
                let gpu_record: GpuPendingEligibilityRecord =
                    record_from_range(&words, base, &ranges.layout.pending_eligibility_words)?;
                if gpu_record != record
                    || PendingEligibilityReceipt::from_gpu_record(
                        gpu_record,
                        handle.slot(),
                        handle.organism_id(),
                        handle.phenotype_hash(),
                    )? != receipt
                {
                    return Err(ScaffoldContractError::LearningEvidenceMismatch);
                }
                Some(pending_parts_from_receipt(receipt)?)
            }
            (None, None, 0) => None,
            _ => return Err(ScaffoldContractError::LearningEvidenceMismatch),
        };
        let neuron_count = brain_slot.record().neuron_count as usize;
        let recurrent_count = brain_slot.record().recurrent_synapse_count as usize;
        let synapse_count = brain_slot.record().synapse_count as usize;
        let decoder_count = synapse_count
            .checked_sub(recurrent_count)
            .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
        let parts = GpuBrainCheckpointParts {
            schema_version: GPU_BRAIN_CHECKPOINT_SCHEMA_VERSION,
            organism_id: handle.organism_id(),
            phenotype_hash: handle.phenotype_hash(),
            checkpoint_tick,
            active_activation_side: active_side,
            logical_dispatch_generation: resident_state.6,
            activation_a_bits: exact_bits(
                &words,
                base,
                &ranges.layout.activation_a_words,
                neuron_count,
            )?,
            activation_b_bits: exact_bits(
                &words,
                base,
                &ranges.layout.activation_b_words,
                neuron_count,
            )?,
            neuron_homeostasis_bits: exact_bits(
                &words,
                base,
                &ranges.layout.homeostasis_words,
                neuron_count * 2,
            )?,
            active_weight_generation,
            active_weight_bank: state.active_weight_bank as u8,
            lifetime_bank_0_bits: exact_bits(
                &words,
                base,
                &ranges.layout.lifetime_weight_words,
                synapse_count,
            )?,
            lifetime_bank_1_bits: exact_bits(
                &words,
                base,
                &ranges.layout.lifetime_weight_bank_1_words,
                synapse_count,
            )?,
            fast_bank_0_bits: exact_bits(
                &words,
                base,
                &ranges.layout.fast_weight_words,
                synapse_count,
            )?,
            fast_bank_1_bits: exact_bits(
                &words,
                base,
                &ranges.layout.fast_weight_bank_1_words,
                synapse_count,
            )?,
            active_eligibility_generation,
            inactive_eligibility_generation,
            active_eligibility_bank: state.active_eligibility_bank as u8,
            learning_transaction_generation: transaction_generation,
            recurrent_eligibility_bank_0_bits: exact_bits(
                &words,
                base,
                &ranges.layout.recurrent_eligibility_words,
                recurrent_count,
            )?,
            recurrent_eligibility_bank_1_bits: exact_bits(
                &words,
                base,
                &ranges.layout.recurrent_eligibility_bank_1_words,
                recurrent_count,
            )?,
            decoder_eligibility_bank_0_bits: exact_bits(
                &words,
                base,
                &ranges.layout.decoder_eligibility_words,
                decoder_count,
            )?,
            decoder_eligibility_bank_1_bits: exact_bits(
                &words,
                base,
                &ranges.layout.decoder_eligibility_bank_1_words,
                decoder_count,
            )?,
            replay_journal_generation: replay_generation,
            replay_journal_cursor: state.replay_cursor,
            replay_journal_event_count: state.replay_event_count,
            replay_events: records_from_range(
                &words,
                base,
                &ranges.layout.replay_event_words,
                state.replay_event_capacity as usize,
            )?,
            replay_spans: records_from_range(
                &words,
                base,
                &ranges.layout.replay_span_words,
                state.replay_span_count as usize,
            )?,
            replay_samples: exact_bits(
                &words,
                base,
                &ranges.layout.replay_sample_words,
                state.replay_sample_capacity as usize,
            )?,
            last_learning_replay_key: resident_state.7,
            pending_eligibility: pending,
        };
        GpuBrainCheckpointSnapshot::try_from_parts(parts)
    }

    pub fn restore_brain(
        &mut self,
        organism_id: OrganismId,
        phenotype: BrainPhenotype,
        request: GpuBrainRestoreRequest,
    ) -> Result<GpuBrainRestoreReceipt, ScaffoldContractError> {
        self.ensure_ready()?;
        let snapshot = request.into_snapshot();
        snapshot.validate()?;
        let checkpoint_digest = snapshot.canonical_digest();
        let parts = snapshot.into_parts();
        if organism_id != parts.organism_id
            || phenotype.phenotype_hash() != parts.phenotype_hash
            || phenotype.neuron_count() as usize != parts.activation_a_bits.len()
        {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        let handle = self.insert_brain(organism_id, phenotype)?;
        let restore = self.restore_brain_inner(handle, parts, checkpoint_digest);
        if restore.is_err() {
            let _ = self.remove_brain(handle);
        }
        restore
    }

    fn restore_brain_inner(
        &mut self,
        handle: GpuBrainHandle,
        parts: GpuBrainCheckpointParts,
        checkpoint_digest: [u64; 4],
    ) -> Result<GpuBrainRestoreReceipt, ScaffoldContractError> {
        validate_checkpoint_parts(&parts)?;
        let (brain_slot, ranges, initialized_state) = {
            let bucket = self
                .class_buckets
                .get(&handle.class_id().raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let resident = bucket.slots[handle.slot() as usize]
                .as_ref()
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let words = self.read_slot_mutable_words(handle, &resident.ranges)?;
            let state = record_from_range::<GpuSlotLearningStateRecord>(
                &words,
                resident.ranges.mutable_state_words.start,
                &resident.ranges.layout.learning_state_words,
            )?;
            (resident.brain_slot.clone(), resident.ranges.clone(), state)
        };
        let neuron_count = brain_slot.record().neuron_count as usize;
        let recurrent_count = brain_slot.record().recurrent_synapse_count as usize;
        let synapse_count = brain_slot.record().synapse_count as usize;
        let decoder_count = synapse_count
            .checked_sub(recurrent_count)
            .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
        if parts.activation_a_bits.len() != neuron_count
            || parts.neuron_homeostasis_bits.len() != neuron_count * 2
            || parts.lifetime_bank_0_bits.len() != synapse_count
            || parts.recurrent_eligibility_bank_0_bits.len() != recurrent_count
            || parts.decoder_eligibility_bank_0_bits.len() != decoder_count
            || parts.replay_events.len() != initialized_state.replay_event_capacity as usize
            || parts.replay_spans.len() != initialized_state.replay_span_count as usize
            || parts.replay_samples.len() != initialized_state.replay_sample_capacity as usize
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        let (pending_record, pending_receipt) = match parts.pending_eligibility {
            Some(pending) => {
                let (record, receipt) = restored_pending_record(handle, pending)?;
                (record, Some(receipt))
            }
            None => (GpuPendingEligibilityRecord::zeroed(), None),
        };
        let mut state = initialized_state;
        state.active_weight_bank = u32::from(parts.active_weight_bank);
        state.active_eligibility_bank = u32::from(parts.active_eligibility_bank);
        state.pending_valid = u32::from(pending_receipt.is_some());
        [
            state.active_weight_generation_lo,
            state.active_weight_generation_hi,
        ] = split_pair(parts.active_weight_generation);
        [
            state.active_eligibility_generation_lo,
            state.active_eligibility_generation_hi,
        ] = split_pair(parts.active_eligibility_generation);
        [
            state.inactive_eligibility_generation_lo,
            state.inactive_eligibility_generation_hi,
        ] = split_pair(parts.inactive_eligibility_generation);
        [state.replay_generation_lo, state.replay_generation_hi] =
            split_pair(parts.replay_journal_generation);
        state.replay_cursor = parts.replay_journal_cursor;
        state.replay_event_count = parts.replay_journal_event_count;
        [
            state.transaction_generation_lo,
            state.transaction_generation_hi,
        ] = split_pair(parts.learning_transaction_generation);
        let buffer = self
            .class_buckets
            .get(&handle.class_id().raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
            .buffers
            .neural_buffers()[6];
        for (range, words) in [
            (
                &ranges.layout.activation_a_words,
                parts.activation_a_bits.as_slice(),
            ),
            (
                &ranges.layout.activation_b_words,
                parts.activation_b_bits.as_slice(),
            ),
            (
                &ranges.layout.homeostasis_words,
                parts.neuron_homeostasis_bits.as_slice(),
            ),
            (
                &ranges.layout.lifetime_weight_words,
                parts.lifetime_bank_0_bits.as_slice(),
            ),
            (
                &ranges.layout.lifetime_weight_bank_1_words,
                parts.lifetime_bank_1_bits.as_slice(),
            ),
            (
                &ranges.layout.fast_weight_words,
                parts.fast_bank_0_bits.as_slice(),
            ),
            (
                &ranges.layout.fast_weight_bank_1_words,
                parts.fast_bank_1_bits.as_slice(),
            ),
            (
                &ranges.layout.recurrent_eligibility_words,
                parts.recurrent_eligibility_bank_0_bits.as_slice(),
            ),
            (
                &ranges.layout.recurrent_eligibility_bank_1_words,
                parts.recurrent_eligibility_bank_1_bits.as_slice(),
            ),
            (
                &ranges.layout.decoder_eligibility_words,
                parts.decoder_eligibility_bank_0_bits.as_slice(),
            ),
            (
                &ranges.layout.decoder_eligibility_bank_1_words,
                parts.decoder_eligibility_bank_1_bits.as_slice(),
            ),
            (
                &ranges.layout.replay_event_words,
                bytemuck::cast_slice(parts.replay_events.as_slice()),
            ),
            (
                &ranges.layout.replay_span_words,
                bytemuck::cast_slice(parts.replay_spans.as_slice()),
            ),
            (
                &ranges.layout.replay_sample_words,
                parts.replay_samples.as_slice(),
            ),
            (&ranges.layout.learning_state_words, state.words()),
            (
                &ranges.layout.pending_eligibility_words,
                pending_record.words(),
            ),
        ] {
            write_exact_prefix(&self.queue, buffer, range, words)?;
        }
        {
            let bucket = self
                .class_buckets
                .get_mut(&handle.class_id().raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            bucket
                .pipelines
                .restore_slot_active_side(
                    handle.slot(),
                    handle.generation(),
                    parts.active_activation_side,
                )
                .map_err(map_gpu_contract_error)?;
            let resident = bucket.slots[handle.slot() as usize]
                .as_mut()
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            resident.active_weight_generation = parts.active_weight_generation;
            resident.active_weight_bank = parts.active_weight_bank;
            resident.active_eligibility_generation = parts.active_eligibility_generation;
            resident.active_eligibility_bank = parts.active_eligibility_bank;
            resident.replay_journal_generation = parts.replay_journal_generation;
            resident.transaction_generation = parts.learning_transaction_generation;
            resident.logical_dispatch_generation = parts.logical_dispatch_generation;
            resident.learning_sequence_guard = LearningSequenceGuard::restore_validated(
                handle.organism_id(),
                handle.phenotype_hash(),
                parts.last_learning_replay_key,
            )?;
            resident.pending_eligibility = pending_receipt;
            resident.pending_eligibility_record = pending_receipt.map(|_| pending_record);
        }
        self.next_dispatch_generation = self.next_dispatch_generation.max(
            parts
                .logical_dispatch_generation
                .checked_add(1)
                .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?,
        );
        let submission = self.queue.submit(std::iter::empty());
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .map_err(|_| ScaffoldContractError::NeuralBackendUnavailable)?;
        Ok(GpuBrainRestoreReceipt {
            handle,
            pending_eligibility: pending_receipt,
            active_weight_generation: parts.active_weight_generation,
            active_weight_bank: parts.active_weight_bank,
            active_eligibility_generation: parts.active_eligibility_generation,
            active_eligibility_bank: parts.active_eligibility_bank,
            learning_transaction_generation: parts.learning_transaction_generation,
            replay_journal_generation: parts.replay_journal_generation,
            replay_journal_cursor: parts.replay_journal_cursor,
            replay_journal_event_count: parts.replay_journal_event_count,
            checkpoint_digest,
        })
    }

    pub fn snapshot_completed_sleep_staging(
        &mut self,
        handle: GpuBrainHandle,
        request: &GpuConsolidationRequest,
        staged: &ConsolidationStagedOutput,
    ) -> Result<GpuCompletedSleepStagingParts, ScaffoldContractError> {
        self.ensure_ready()?;
        request.validate_contract()?;
        let job = self
            .sleep_jobs
            .get(&staged.job_id.raw())
            .cloned()
            .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
        if job.handle != handle
            || job.request != *request
            || job.receipt.staged != *staged
            || job.restored_completed
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        let snapshot = self.sleep_slot_snapshot(handle)?;
        staged.validate_against(
            request,
            snapshot.sleep_plan.eligibility_reset_policy_raw(),
            snapshot.sleep_plan.replay_consume_policy_raw(),
        )?;
        let words = self.read_slot_mutable_words(handle, &snapshot.ranges)?;
        let base = snapshot.ranges.mutable_state_words.start;
        let state: GpuSlotLearningStateRecord =
            record_from_range(&words, base, &snapshot.ranges.layout.learning_state_words)?;
        let completion: GpuSleepCompletionRecord = record_from_absolute_start(
            &words,
            base,
            snapshot.ranges.layout.diagnostic_words.start,
        )?;
        if state.active_weight_bank != u32::from(snapshot.active_weight_bank)
            || join_pair([
                state.active_weight_generation_lo,
                state.active_weight_generation_hi,
            ]) != snapshot.active_weight_generation
            || state.replay_event_capacity != request.max_replay_events
            || state.replay_sample_capacity != request.max_replay_eligibility_samples
            || completion.schema_version != 1
            || completion.slot != handle.slot()
            || completion.slot_generation != handle.generation()
            || completion.status != 1
            || join_pair(completion.input_generation) != request.input_generation
            || join_pair(completion.output_generation) != staged.output_generation
            || completion.output_weight_bank != u32::from(staged.output_weight_bank)
            || completion.replay_span_count != state.replay_span_count
            || join_pair(completion.job_id) != staged.job_id.raw()
            || completion.reserved != [0; 2]
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        let synapse_count = snapshot.brain_slot.record().synapse_count as usize;
        let recurrent_count = snapshot.brain_slot.record().recurrent_synapse_count as usize;
        let decoder_count = synapse_count
            .checked_sub(recurrent_count)
            .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
        let event_capacity = state.replay_event_capacity as usize;
        let sample_capacity = state.replay_sample_capacity as usize;
        let physical_spans = records_from_range::<GpuReplaySynapseSpanRecord>(
            &words,
            base,
            &snapshot.ranges.layout.replay_span_words,
            state.replay_span_count as usize,
        )?;
        let replay_spans = physical_spans
            .into_iter()
            .enumerate()
            .map(|(index, span)| {
                let sample_start = index
                    .checked_mul(event_capacity)
                    .and_then(|value| u32::try_from(value).ok())
                    .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
                if span.sample_start != sample_start || span.reserved != 0 {
                    return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
                }
                Ok(GpuReplaySynapseSpanRecord {
                    local_synapse_id: span.local_synapse_id,
                    sample_start,
                    sample_count: 0,
                    reserved: 0,
                })
            })
            .collect::<Result<Vec<_>, ScaffoldContractError>>()?;
        let parts = GpuCompletedSleepStagingInputParts {
            output_weight_generation: staged.output_generation,
            output_weight_bank: staged.output_weight_bank,
            lifetime_bank_0_bits: exact_bits(
                &words,
                base,
                &snapshot.ranges.layout.lifetime_weight_words,
                synapse_count,
            )?,
            lifetime_bank_1_bits: exact_bits(
                &words,
                base,
                &snapshot.ranges.layout.lifetime_weight_bank_1_words,
                synapse_count,
            )?,
            fast_bank_0_bits: exact_bits(
                &words,
                base,
                &snapshot.ranges.layout.fast_weight_words,
                synapse_count,
            )?,
            fast_bank_1_bits: exact_bits(
                &words,
                base,
                &snapshot.ranges.layout.fast_weight_bank_1_words,
                synapse_count,
            )?,
            eligibility_reset_generation: staged.eligibility_reset_generation,
            output_eligibility_bank: staged.output_eligibility_bank,
            recurrent_eligibility_bank_0_bits: vec![0; recurrent_count],
            recurrent_eligibility_bank_1_bits: vec![0; recurrent_count],
            decoder_eligibility_bank_0_bits: vec![0; decoder_count],
            decoder_eligibility_bank_1_bits: vec![0; decoder_count],
            replay_journal_generation: staged.replay_journal_generation,
            replay_journal_cursor: staged.replay_journal_cursor,
            replay_journal_event_count: staged.replay_journal_event_count,
            replay_events: vec![GpuReplayEventRecord::zeroed(); event_capacity],
            replay_spans,
            replay_samples: vec![0; sample_capacity],
        };
        validate_completed_staging_against(
            handle,
            request,
            staged,
            &parts,
            synapse_count,
            recurrent_count,
            decoder_count,
        )?;
        GpuCompletedSleepStagingParts::try_from_parts(parts)
    }

    pub fn restore_completed_sleep_staging(
        &mut self,
        handle: GpuBrainHandle,
        request: &GpuConsolidationRequest,
        replay: &BoundedReplayBatch,
        staged: &ConsolidationStagedOutput,
        parts: GpuCompletedSleepStagingParts,
    ) -> Result<GpuSleepStagingReceipt, ScaffoldContractError> {
        self.ensure_ready()?;
        self.validate_handle_backend(handle)?;
        request.validate_contract()?;
        parts.validate()?;
        if self.sleep_jobs.contains_key(&staged.job_id.raw())
            || self
                .committed_sleep
                .contains_key(&sleep_commit_key(handle, request.cycle_id))
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        let snapshot = self.sleep_slot_snapshot(handle)?;
        staged.validate_against(
            request,
            snapshot.sleep_plan.eligibility_reset_policy_raw(),
            snapshot.sleep_plan.replay_consume_policy_raw(),
        )?;
        replay.validate_contract(
            request.max_replay_events,
            request.max_replay_eligibility_samples,
            snapshot.brain_slot.record().synapse_count,
        )?;
        if replay.canonical_digest != request.replay_digest
            || self.build_sleep_replay_batch(handle)? != *replay
            || self.prepare_sleep_consolidation(
                handle,
                ConsolidationIntent {
                    cycle_id: request.cycle_id,
                },
                replay,
            )? != *request
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        let parts = parts.into_parts();
        let synapse_count = snapshot.brain_slot.record().synapse_count as usize;
        let recurrent_count = snapshot.brain_slot.record().recurrent_synapse_count as usize;
        let decoder_count = synapse_count
            .checked_sub(recurrent_count)
            .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
        validate_completed_staging_against(
            handle,
            request,
            staged,
            &parts,
            synapse_count,
            recurrent_count,
            decoder_count,
        )?;
        let before_words = self.read_slot_mutable_words(handle, &snapshot.ranges)?;
        let base = snapshot.ranges.mutable_state_words.start;
        let state: GpuSlotLearningStateRecord = record_from_range(
            &before_words,
            base,
            &snapshot.ranges.layout.learning_state_words,
        )?;
        let (saved_active_lifetime, saved_active_fast) = if snapshot.active_weight_bank == 0 {
            (&parts.lifetime_bank_0_bits, &parts.fast_bank_0_bits)
        } else {
            (&parts.lifetime_bank_1_bits, &parts.fast_bank_1_bits)
        };
        let (current_active_lifetime, current_active_fast) = if snapshot.active_weight_bank == 0 {
            (
                exact_bits(
                    &before_words,
                    base,
                    &snapshot.ranges.layout.lifetime_weight_words,
                    synapse_count,
                )?,
                exact_bits(
                    &before_words,
                    base,
                    &snapshot.ranges.layout.fast_weight_words,
                    synapse_count,
                )?,
            )
        } else {
            (
                exact_bits(
                    &before_words,
                    base,
                    &snapshot.ranges.layout.lifetime_weight_bank_1_words,
                    synapse_count,
                )?,
                exact_bits(
                    &before_words,
                    base,
                    &snapshot.ranges.layout.fast_weight_bank_1_words,
                    synapse_count,
                )?,
            )
        };
        if current_active_lifetime != *saved_active_lifetime
            || current_active_fast != *saved_active_fast
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        let buffer = self
            .class_buckets
            .get(&handle.class_id().raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
            .buffers
            .neural_buffers()[6];
        let (output_lifetime_range, output_fast_range, output_lifetime, output_fast) =
            if staged.output_weight_bank == 0 {
                (
                    &snapshot.ranges.layout.lifetime_weight_words,
                    &snapshot.ranges.layout.fast_weight_words,
                    parts.lifetime_bank_0_bits.as_slice(),
                    parts.fast_bank_0_bits.as_slice(),
                )
            } else {
                (
                    &snapshot.ranges.layout.lifetime_weight_bank_1_words,
                    &snapshot.ranges.layout.fast_weight_bank_1_words,
                    parts.lifetime_bank_1_bits.as_slice(),
                    parts.fast_bank_1_bits.as_slice(),
                )
            };
        for (range, words) in [
            (output_lifetime_range, output_lifetime),
            (output_fast_range, output_fast),
            (
                &snapshot.ranges.layout.recurrent_eligibility_words,
                parts.recurrent_eligibility_bank_0_bits.as_slice(),
            ),
            (
                &snapshot.ranges.layout.recurrent_eligibility_bank_1_words,
                parts.recurrent_eligibility_bank_1_bits.as_slice(),
            ),
            (
                &snapshot.ranges.layout.decoder_eligibility_words,
                parts.decoder_eligibility_bank_0_bits.as_slice(),
            ),
            (
                &snapshot.ranges.layout.decoder_eligibility_bank_1_words,
                parts.decoder_eligibility_bank_1_bits.as_slice(),
            ),
            (
                &snapshot.ranges.layout.replay_event_words,
                bytemuck::cast_slice(parts.replay_events.as_slice()),
            ),
            (
                &snapshot.ranges.layout.replay_span_words,
                bytemuck::cast_slice(parts.replay_spans.as_slice()),
            ),
            (
                &snapshot.ranges.layout.replay_sample_words,
                parts.replay_samples.as_slice(),
            ),
        ] {
            write_exact_prefix(&self.queue, buffer, range, words)?;
        }
        let completion = GpuSleepCompletionRecord {
            schema_version: 1,
            slot: handle.slot(),
            slot_generation: handle.generation(),
            status: 1,
            input_generation: split_pair(request.input_generation),
            output_generation: split_pair(staged.output_generation),
            output_weight_bank: u32::from(staged.output_weight_bank),
            replay_span_count: replay.synapse_spans.len() as u32,
            promoted_fast_l1_q12: diagnostic_q12(staged.promoted_fast_l1())?,
            replay_induced_fast_l1_q12: diagnostic_q12(staged.replay_induced_fast_l1())?,
            job_id: split_pair(staged.job_id.raw()),
            reserved: [0; 2],
        };
        write_words_at_offset(
            &self.queue,
            buffer,
            snapshot.ranges.layout.diagnostic_words.start,
            snapshot.ranges.mutable_state_words.end,
            bytemuck::cast_slice(std::slice::from_ref(&completion)),
        )?;
        let (header, payload_words) =
            build_sleep_upload(handle, &snapshot, request, replay, staged.job_id)?;
        let reset_word_count = reset_word_count(&snapshot, &state)?;
        let receipt = GpuSleepStagingReceipt {
            handle,
            cycle_id: request.cycle_id,
            phenotype_hash: handle.phenotype_hash(),
            input_generation: request.input_generation,
            input_digest: request.input_digest,
            replay_digest: request.replay_digest,
            staged: *staged,
            hardware_receipt_generation: self.hardware.generation,
        };
        let submission = self.queue.submit(std::iter::empty());
        if self
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .is_err()
        {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        self.next_sleep_job_id = self.next_sleep_job_id.max(
            staged
                .job_id
                .raw()
                .checked_add(1)
                .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?,
        );
        self.sleep_jobs.insert(
            staged.job_id.raw(),
            GpuSleepJobState {
                handle,
                request: *request,
                replay: replay.clone(),
                header,
                payload_words,
                reset_word_count,
                receipt,
                restored_completed: true,
            },
        );
        Ok(receipt)
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_completed_staging_against(
    handle: GpuBrainHandle,
    request: &GpuConsolidationRequest,
    staged: &ConsolidationStagedOutput,
    parts: &GpuCompletedSleepStagingInputParts,
    synapse_count: usize,
    recurrent_count: usize,
    decoder_count: usize,
) -> Result<(), ScaffoldContractError> {
    validate_completed_staging_parts(parts)?;
    if request.phenotype_hash != handle.phenotype_hash()
        || parts.output_weight_generation != staged.output_generation
        || parts.output_weight_bank != staged.output_weight_bank
        || parts.lifetime_bank_0_bits.len() != synapse_count
        || parts.recurrent_eligibility_bank_0_bits.len() != recurrent_count
        || parts.decoder_eligibility_bank_0_bits.len() != decoder_count
        || parts.eligibility_reset_generation != staged.eligibility_reset_generation
        || parts.output_eligibility_bank != staged.output_eligibility_bank
        || parts.replay_journal_generation != staged.replay_journal_generation
        || parts.replay_journal_cursor != staged.replay_journal_cursor
        || parts.replay_journal_event_count != staged.replay_journal_event_count
        || parts.replay_events.len() != request.max_replay_events as usize
        || parts.replay_samples.len() != request.max_replay_eligibility_samples as usize
    {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    let (lifetime, fast) = if staged.output_weight_bank == 0 {
        (
            f32_from_bits(&parts.lifetime_bank_0_bits)?,
            f32_from_bits(&parts.fast_bank_0_bits)?,
        )
    } else {
        (
            f32_from_bits(&parts.lifetime_bank_1_bits)?,
            f32_from_bits(&parts.fast_bank_1_bits)?,
        )
    };
    let output_digest = compute_gpu_sleep_output_weight_digest(
        handle.phenotype_hash(),
        handle.class_id(),
        staged.output_generation,
        staged.output_weight_bank,
        &lifetime,
        &fast,
    )?;
    let eligibility_digest = eligibility_reset_digest(
        staged.eligibility_reset_generation,
        staged.output_eligibility_bank,
        &parts.recurrent_eligibility_bank_0_bits,
        recurrent_count,
        &parts.recurrent_eligibility_bank_1_bits,
        recurrent_count,
        &parts.decoder_eligibility_bank_0_bits,
        decoder_count,
        &parts.decoder_eligibility_bank_1_bits,
        decoder_count,
    )?;
    let replay_digest = replay_reset_digest(
        staged.replay_journal_generation,
        staged.replay_journal_cursor,
        staged.replay_journal_event_count,
        request.max_replay_events,
        request.max_replay_eligibility_samples,
        bytemuck::cast_slice(parts.replay_events.as_slice()),
        &parts.replay_samples,
        &parts.replay_spans,
    )?;
    if output_digest != staged.output_digest
        || eligibility_digest != staged.eligibility_output_digest
        || replay_digest != staged.replay_journal_output_digest
    {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    Ok(())
}

fn f32_from_bits(bits: &[u32]) -> Result<Vec<f32>, ScaffoldContractError> {
    validate_float_bits(bits)?;
    Ok(bits.iter().map(|bits| f32::from_bits(*bits)).collect())
}

fn diagnostic_q12(value: f32) -> Result<u32, ScaffoldContractError> {
    if !value.is_finite()
        || value.is_sign_negative()
        || value > u32::MAX as f32 / SLEEP_DIAGNOSTIC_Q12
    {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    Ok((value * SLEEP_DIAGNOSTIC_Q12).round() as u32)
}

fn write_exact_prefix(
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    range: &std::ops::Range<u32>,
    words: &[u32],
) -> Result<(), ScaffoldContractError> {
    let capacity = range
        .end
        .checked_sub(range.start)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
    if words.is_empty() || words.len() > capacity {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    queue.write_buffer(
        buffer,
        u64::from(range.start) * 4,
        bytemuck::cast_slice(words),
    );
    Ok(())
}

fn write_words_at_offset(
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    start: u32,
    heap_end: u32,
    words: &[u32],
) -> Result<(), ScaffoldContractError> {
    let word_count = u32::try_from(words.len())
        .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?;
    if words.is_empty()
        || start
            .checked_add(word_count)
            .is_none_or(|end| end > heap_end)
    {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    queue.write_buffer(buffer, u64::from(start) * 4, bytemuck::cast_slice(words));
    Ok(())
}

const _: () = assert!(LEARNING_STATE_WORDS == 24);
const _: () = assert!(REPLAY_EVENT_WORDS == 24);
