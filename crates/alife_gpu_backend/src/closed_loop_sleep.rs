//! GPU-authoritative sleep replay, staging, and exactly-once bank commit.
//!
//! Host code validates identities, canonicalizes bounded replay, hashes complete
//! banks, and persists compact receipts. All replay-credit and weight-update
//! mathematics execute in WGSL.

use std::ops::Range;

use alife_core::{
    compute_gpu_sleep_commit_digest, compute_gpu_sleep_input_weight_digest,
    compute_gpu_sleep_mutable_state_digest, compute_gpu_sleep_output_weight_digest, ActionId,
    BoundedReplayBatch, CandidateActionFamily, CandidateFeatureDigest, CanonicalDigestBuilder,
    ConsolidationIntent, ConsolidationJobId, ConsolidationStagedOutput, ExperienceSequenceId,
    GpuConsolidationRequest, NeuromodulatorSample, PerceptionFrameDigest, PhenotypeHash,
    ReplayEligibilitySample, ReplaySynapseSpan, ScaffoldContractError, SleepReplayEvent, Tick,
    Validate, BOUNDED_REPLAY_BATCH_SCHEMA_VERSION, GPU_CONSOLIDATION_REQUEST_SCHEMA_VERSION,
};
use bytemuck::{Pod, Zeroable};

use crate::{
    pack_replay_eligibility_sample, unpack_replay_eligibility_sample, GpuBrainHandle, GpuBrainSlot,
    GpuClosedLoopBackend, GpuConsolidationRequestRecord, GpuFixedSlotRanges, GpuReplayEventRecord,
    GpuReplaySynapseSpanRecord, GpuSleepHeader, GpuSlotLearningStateRecord,
};

pub type GpuSleepJobId = ConsolidationJobId;

const SLEEP_HEADER_WORDS: usize = 20;
const CONSOLIDATION_REQUEST_WORDS: usize = 44;
const REPLAY_EVENT_WORDS: usize = 24;
const REPLAY_SPAN_WORDS: usize = 4;
const SLEEP_COMPLETION_WORDS: usize = 16;
const SLEEP_STATUS_STAGED: u32 = 1;
const SLEEP_STATUS_COMMITTED: u32 = 2;
const SLEEP_DIAGNOSTIC_Q12: f32 = 4096.0;
const ELIGIBILITY_RESET_DIGEST_DOMAIN: &[u8] = b"ALIFE-GPU-SLEEP-ELIGIBILITY-RESET-V1";
const REPLAY_RESET_DIGEST_DOMAIN: &[u8] = b"ALIFE-GPU-SLEEP-REPLAY-RESET-V1";

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub(crate) struct GpuSleepCompletionRecord {
    pub(crate) schema_version: u32,
    pub(crate) slot: u32,
    pub(crate) slot_generation: u32,
    pub(crate) status: u32,
    pub(crate) input_generation: [u32; 2],
    pub(crate) output_generation: [u32; 2],
    pub(crate) output_weight_bank: u32,
    pub(crate) replay_span_count: u32,
    pub(crate) promoted_fast_l1_q12: u32,
    pub(crate) replay_induced_fast_l1_q12: u32,
    pub(crate) job_id: [u32; 2],
    pub(crate) reserved: [u32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuSleepStagingReceipt {
    pub handle: GpuBrainHandle,
    pub cycle_id: u64,
    pub phenotype_hash: PhenotypeHash,
    pub input_generation: u64,
    pub input_digest: [u64; 4],
    pub replay_digest: [u64; 4],
    pub staged: ConsolidationStagedOutput,
    pub hardware_receipt_generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuSleepConsolidationReceipt {
    pub staged: GpuSleepStagingReceipt,
    pub output_generation: u64,
    pub output_digest: [u64; 4],
    pub promoted_fast_l1: f32,
    pub replay_induced_fast_l1: f32,
    pub generation_swaps: u32,
    pub active_weight_bank: u8,
    pub eligibility_reset_generation: u64,
    pub active_eligibility_bank: u8,
    pub replay_journal_generation: u64,
    pub replay_journal_cursor: u32,
    pub replay_journal_event_count: u32,
    pub commit_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuLearningStateSnapshot {
    pub active_weight_bank: u8,
    pub active_weight_generation: u64,
    pub active_eligibility_bank: u8,
    pub active_eligibility_generation: u64,
    pub replay_journal_generation: u64,
    pub replay_journal_cursor: u32,
    pub replay_journal_event_count: u32,
    pub transaction_generation: u64,
    pub recurrent_eligibility_nonzero: usize,
    pub decoder_eligibility_nonzero: usize,
    pub replay_event_nonzero_words: usize,
    pub replay_sample_nonzero_words: usize,
    pub pending_eligibility_nonzero_words: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct GpuSleepJobState {
    pub(crate) handle: GpuBrainHandle,
    pub(crate) request: GpuConsolidationRequest,
    pub(crate) replay: BoundedReplayBatch,
    pub(crate) header: GpuSleepHeader,
    pub(crate) payload_words: Vec<u32>,
    pub(crate) reset_word_count: u32,
    pub(crate) receipt: GpuSleepStagingReceipt,
    pub(crate) restored_completed: bool,
}

#[derive(Clone)]
pub(crate) struct SleepSlotSnapshot {
    pub(crate) brain_slot: GpuBrainSlot,
    pub(crate) ranges: GpuFixedSlotRanges,
    pub(crate) active_weight_bank: u8,
    pub(crate) active_weight_generation: u64,
    pub(crate) active_eligibility_bank: u8,
    pub(crate) active_eligibility_generation: u64,
    pub(crate) replay_journal_generation: u64,
    pub(crate) transaction_generation: u64,
    pub(crate) sleep_plan: alife_core::SleepConsolidationPlan,
}

impl From<&GpuConsolidationRequest> for GpuConsolidationRequestRecord {
    fn from(request: &GpuConsolidationRequest) -> Self {
        Self {
            schema_version: u32::from(request.schema_version),
            request_flags: u32::from(request.request_flags),
            cycle_id_lo: request.cycle_id as u32,
            cycle_id_hi: (request.cycle_id >> 32) as u32,
            phenotype_hash: split_digest(request.phenotype_hash.0),
            input_generation_lo: request.input_generation as u32,
            input_generation_hi: (request.input_generation >> 32) as u32,
            expected_output_generation_lo: request.expected_output_generation as u32,
            expected_output_generation_hi: (request.expected_output_generation >> 32) as u32,
            input_digest: split_digest(request.input_digest),
            replay_digest: split_digest(request.replay_digest),
            max_replay_events: request.max_replay_events,
            max_replay_eligibility_samples: request.max_replay_eligibility_samples,
            request_digest: split_digest(request.request_digest),
            reserved_tail: [0; 2],
        }
    }
}

impl GpuClosedLoopBackend {
    pub fn build_sleep_replay_batch(
        &mut self,
        handle: GpuBrainHandle,
    ) -> Result<BoundedReplayBatch, ScaffoldContractError> {
        self.ensure_ready()?;
        let snapshot = self.sleep_slot_snapshot(handle)?;
        let words = self.read_slot_mutable_words(handle, &snapshot.ranges)?;
        let state = learning_state_from_slot_words(&words, &snapshot.ranges)?;
        validate_learning_state(&snapshot, &state)?;

        let capacity = state.replay_event_capacity;
        let event_count = state.replay_event_count;
        let oldest = if event_count == capacity {
            state.replay_cursor
        } else {
            0
        };
        let physical_order = (0..event_count)
            .map(|offset| (oldest + offset) % capacity)
            .collect::<Vec<_>>();
        let event_words = absolute_slice(
            &words,
            snapshot.ranges.mutable_state_words.start,
            &snapshot.ranges.layout.replay_event_words,
        )?;
        let events = physical_order
            .iter()
            .map(|physical| {
                let start = usize::try_from(*physical)
                    .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?
                    .checked_mul(REPLAY_EVENT_WORDS)
                    .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
                let end = start
                    .checked_add(REPLAY_EVENT_WORDS)
                    .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
                let row = GpuReplayEventRecord::from_words(
                    event_words
                        .get(start..end)
                        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?,
                )
                .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?;
                decode_replay_event(row)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let all_physical_spans = pod_slice_from_absolute::<GpuReplaySynapseSpanRecord>(
            &words,
            snapshot.ranges.mutable_state_words.start,
            &snapshot.ranges.layout.replay_span_words,
        )?;
        let physical_spans = all_physical_spans
            .get(..state.replay_span_count as usize)
            .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
        let sample_words = absolute_slice(
            &words,
            snapshot.ranges.mutable_state_words.start,
            &snapshot.ranges.layout.replay_sample_words,
        )?;
        let mut synapse_spans = Vec::with_capacity(physical_spans.len());
        let mut eligibility_samples = Vec::with_capacity(
            usize::try_from(event_count)
                .ok()
                .and_then(|count| count.checked_mul(physical_spans.len()))
                .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?,
        );
        for (capture_index, physical_span) in physical_spans.iter().enumerate() {
            let expected_physical_start = u32::try_from(capture_index)
                .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?
                .checked_mul(capacity)
                .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
            if physical_span.sample_start != expected_physical_start
                || physical_span.sample_count != event_count
                || physical_span.reserved != 0
            {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
            }
            let compact_start = u32::try_from(eligibility_samples.len())
                .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?;
            for (logical_index, physical_event) in physical_order.iter().copied().enumerate() {
                let index = physical_span
                    .sample_start
                    .checked_add(physical_event)
                    .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
                let packed = *sample_words
                    .get(index as usize)
                    .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
                let (captured_physical, eligibility_q15) = unpack_replay_eligibility_sample(packed);
                if u32::from(captured_physical) != physical_event || eligibility_q15 == i16::MIN {
                    return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
                }
                eligibility_samples.push(ReplayEligibilitySample {
                    event_index: u16::try_from(logical_index)
                        .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?,
                    eligibility_q15,
                });
            }
            synapse_spans.push(ReplaySynapseSpan {
                local_synapse_id: physical_span.local_synapse_id,
                sample_start: compact_start,
                sample_count: event_count,
                reserved: 0,
            });
        }
        let mut batch = BoundedReplayBatch {
            schema_version: BOUNDED_REPLAY_BATCH_SCHEMA_VERSION,
            events,
            synapse_spans,
            eligibility_samples,
            canonical_digest: [0; 4],
        };
        batch.canonical_digest = batch.recompute_canonical_digest()?;
        batch.validate_contract(
            state.replay_event_capacity,
            state.replay_sample_capacity,
            snapshot.brain_slot.record().synapse_count,
        )?;
        Ok(batch)
    }

    pub fn prepare_sleep_consolidation(
        &self,
        handle: GpuBrainHandle,
        intent: ConsolidationIntent,
        replay: &BoundedReplayBatch,
    ) -> Result<GpuConsolidationRequest, ScaffoldContractError> {
        intent.validate_contract()?;
        let snapshot = self.sleep_slot_snapshot(handle)?;
        let words = self.read_slot_mutable_words(handle, &snapshot.ranges)?;
        let state = learning_state_from_slot_words(&words, &snapshot.ranges)?;
        validate_learning_state(&snapshot, &state)?;
        replay.validate_contract(
            state.replay_event_capacity,
            state.replay_sample_capacity,
            snapshot.brain_slot.record().synapse_count,
        )?;
        if replay.events.len() != state.replay_event_count as usize {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        let (lifetime, fast) = resolved_weight_banks(&words, &snapshot, true)?;
        let input_digest = compute_gpu_sleep_input_weight_digest(
            handle.phenotype_hash(),
            handle.class_id(),
            snapshot.active_weight_generation,
            snapshot.active_weight_bank,
            &lifetime,
            &fast,
        )?;
        let mut request = GpuConsolidationRequest {
            schema_version: GPU_CONSOLIDATION_REQUEST_SCHEMA_VERSION,
            request_flags: 0,
            cycle_id: intent.cycle_id,
            phenotype_hash: handle.phenotype_hash(),
            input_generation: snapshot.active_weight_generation,
            expected_output_generation: snapshot
                .active_weight_generation
                .checked_add(1)
                .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?,
            input_digest,
            replay_digest: replay.canonical_digest,
            max_replay_events: state.replay_event_capacity,
            max_replay_eligibility_samples: state.replay_sample_capacity,
            request_digest: [0; 4],
        };
        request.request_digest = request.recompute_request_digest()?;
        request.validate_contract()?;
        Ok(request)
    }

    pub fn submit_sleep_consolidation(
        &mut self,
        handle: GpuBrainHandle,
        request: &GpuConsolidationRequest,
        replay: &BoundedReplayBatch,
    ) -> Result<GpuSleepJobId, ScaffoldContractError> {
        self.submit_sleep_consolidation_inner(handle, request, replay, None)
    }

    pub fn recover_submitted_sleep_consolidation(
        &mut self,
        handle: GpuBrainHandle,
        request: &GpuConsolidationRequest,
        replay: &BoundedReplayBatch,
        lost_process_job_id: GpuSleepJobId,
    ) -> Result<GpuSleepJobId, ScaffoldContractError> {
        if self.sleep_jobs.contains_key(&lost_process_job_id.raw())
            || self
                .committed_sleep
                .contains_key(&sleep_commit_key(handle, request.cycle_id))
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        if self.next_sleep_job_id == lost_process_job_id.raw() {
            self.next_sleep_job_id = self
                .next_sleep_job_id
                .checked_add(1)
                .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
        }
        self.submit_sleep_consolidation_inner(handle, request, replay, Some(lost_process_job_id))
    }

    pub fn poll_sleep_consolidation(
        &mut self,
        handle: GpuBrainHandle,
        job_id: GpuSleepJobId,
    ) -> Result<Option<GpuSleepStagingReceipt>, ScaffoldContractError> {
        self.ensure_ready()?;
        self.validate_handle_backend(handle)?;
        let job = self
            .sleep_jobs
            .get(&job_id.raw())
            .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
        if job.handle != handle || job.receipt.staged.job_id != job_id {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        Ok(Some(job.receipt))
    }

    pub fn commit_sleep_consolidation(
        &mut self,
        handle: GpuBrainHandle,
        request: &GpuConsolidationRequest,
        staged: &ConsolidationStagedOutput,
    ) -> Result<GpuSleepConsolidationReceipt, ScaffoldContractError> {
        self.ensure_ready()?;
        self.validate_handle_backend(handle)?;
        request.validate_contract()?;
        let key = sleep_commit_key(handle, request.cycle_id);
        if let Some(receipt) = self.committed_sleep.get(&key).copied() {
            if receipt.staged.staged == *staged
                && receipt.staged.input_digest == request.input_digest
                && receipt.staged.replay_digest == request.replay_digest
            {
                return Ok(receipt);
            }
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        let job = self
            .sleep_jobs
            .get(&staged.job_id.raw())
            .cloned()
            .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
        if job.handle != handle
            || job.request != *request
            || job.receipt.staged != *staged
            || job.replay.canonical_digest != request.replay_digest
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        let snapshot = self.sleep_slot_snapshot(handle)?;
        staged.validate_against(
            request,
            snapshot.sleep_plan.eligibility_reset_policy_raw(),
            snapshot.sleep_plan.replay_consume_policy_raw(),
        )?;
        if !job.restored_completed {
            let exact_request = self.prepare_sleep_consolidation(
                handle,
                ConsolidationIntent {
                    cycle_id: request.cycle_id,
                },
                &job.replay,
            )?;
            if exact_request != *request {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
            }
        }

        let dispatch_result = {
            let bucket = self
                .class_buckets
                .get_mut(&handle.class_id().raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            bucket.pipelines.dispatch_sleep_commit(
                &self.device,
                &self.queue,
                &bucket.buffers,
                &job.header,
                &job.payload_words,
                job.reset_word_count,
            )
        };
        if dispatch_result.is_err() || self.device_lost.load(std::sync::atomic::Ordering::Acquire) {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }

        let words = self.read_slot_mutable_words(handle, &snapshot.ranges)?;
        let completion = completion_from_slot_words(&words, &snapshot.ranges)?;
        let state = learning_state_from_slot_words(&words, &snapshot.ranges)?;
        if completion.status != SLEEP_STATUS_COMMITTED
            || completion.slot != handle.slot()
            || completion.slot_generation != handle.generation()
            || join_pair(completion.output_generation) != staged.output_generation
            || state.active_weight_bank != u32::from(staged.output_weight_bank)
            || join_pair([
                state.active_weight_generation_lo,
                state.active_weight_generation_hi,
            ]) != staged.output_generation
            || state.active_eligibility_bank != 0
            || join_pair([
                state.active_eligibility_generation_lo,
                state.active_eligibility_generation_hi,
            ]) != staged.eligibility_reset_generation
            || join_pair([state.replay_generation_lo, state.replay_generation_hi])
                != staged.replay_journal_generation
            || state.replay_cursor != 0
            || state.replay_event_count != 0
            || state.pending_valid != 0
        {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        let (lifetime, fast) = resolved_weight_banks(&words, &snapshot, false)?;
        let output_digest = compute_gpu_sleep_output_weight_digest(
            handle.phenotype_hash(),
            handle.class_id(),
            staged.output_generation,
            staged.output_weight_bank,
            &lifetime,
            &fast,
        )?;
        let eligibility_digest = eligibility_reset_digest_from_words(
            &words,
            &snapshot,
            staged.eligibility_reset_generation,
            staged.output_eligibility_bank,
        )?;
        let replay_digest = replay_reset_digest_from_words(
            &words,
            &snapshot.ranges,
            &state,
            staged.replay_journal_generation,
        )?;
        if output_digest != staged.output_digest
            || eligibility_digest != staged.eligibility_output_digest
            || replay_digest != staged.replay_journal_output_digest
        {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        let mutable_state_digest = compute_gpu_sleep_mutable_state_digest(&words);
        let commit_digest = compute_gpu_sleep_commit_digest(
            staged.staging_digest,
            handle.phenotype_hash(),
            handle.organism_id(),
            request.cycle_id,
            staged.output_generation,
            staged.output_weight_bank,
            staged.eligibility_reset_generation,
            staged.output_eligibility_bank,
            staged.replay_journal_generation,
            staged.replay_journal_cursor,
            staged.replay_journal_event_count,
            mutable_state_digest,
        )?;
        let receipt = GpuSleepConsolidationReceipt {
            staged: job.receipt,
            output_generation: staged.output_generation,
            output_digest,
            promoted_fast_l1: staged.promoted_fast_l1(),
            replay_induced_fast_l1: staged.replay_induced_fast_l1(),
            generation_swaps: 1,
            active_weight_bank: staged.output_weight_bank,
            eligibility_reset_generation: staged.eligibility_reset_generation,
            active_eligibility_bank: staged.output_eligibility_bank,
            replay_journal_generation: staged.replay_journal_generation,
            replay_journal_cursor: staged.replay_journal_cursor,
            replay_journal_event_count: staged.replay_journal_event_count,
            commit_digest,
        };
        let resident = self
            .class_buckets
            .get_mut(&handle.class_id().raw())
            .and_then(|bucket| bucket.slots.get_mut(handle.slot() as usize))
            .and_then(Option::as_mut)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        resident.active_weight_bank = staged.output_weight_bank;
        resident.active_weight_generation = staged.output_generation;
        resident.active_eligibility_bank = staged.output_eligibility_bank;
        resident.active_eligibility_generation = staged.eligibility_reset_generation;
        resident.replay_journal_generation = staged.replay_journal_generation;
        resident.transaction_generation = join_pair([
            state.transaction_generation_lo,
            state.transaction_generation_hi,
        ]);
        resident.pending_eligibility = None;
        resident.pending_eligibility_record = None;
        if self.sleep_jobs.remove(&staged.job_id.raw()).is_none() {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        self.committed_sleep.insert(key, receipt);
        Ok(receipt)
    }

    fn submit_sleep_consolidation_inner(
        &mut self,
        handle: GpuBrainHandle,
        request: &GpuConsolidationRequest,
        replay: &BoundedReplayBatch,
        recovery_job: Option<GpuSleepJobId>,
    ) -> Result<GpuSleepJobId, ScaffoldContractError> {
        self.ensure_ready()?;
        self.validate_handle_backend(handle)?;
        if self
            .sleep_jobs
            .values()
            .any(|job| job.handle == handle && job.request.request_digest == request.request_digest)
            || self
                .committed_sleep
                .contains_key(&sleep_commit_key(handle, request.cycle_id))
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        let exact_replay = self.build_sleep_replay_batch(handle)?;
        if exact_replay != *replay {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        let exact_request = self.prepare_sleep_consolidation(
            handle,
            ConsolidationIntent {
                cycle_id: request.cycle_id,
            },
            replay,
        )?;
        if exact_request != *request || recovery_job.is_some_and(|lost| lost.raw() == 0) {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        let snapshot = self.sleep_slot_snapshot(handle)?;
        let before_words = self.read_slot_mutable_words(handle, &snapshot.ranges)?;
        let state = learning_state_from_slot_words(&before_words, &snapshot.ranges)?;
        let job_id = ConsolidationJobId::try_from_raw(self.next_sleep_job_id)?;
        self.next_sleep_job_id = self
            .next_sleep_job_id
            .checked_add(1)
            .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
        let (header, payload_words) =
            build_sleep_upload(handle, &snapshot, request, replay, job_id)?;
        let reset_word_count = reset_word_count(&snapshot, &state)?;
        let dispatch_result = {
            let bucket = self
                .class_buckets
                .get_mut(&handle.class_id().raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            bucket.pipelines.dispatch_sleep_staging(
                &self.device,
                &self.queue,
                &bucket.buffers,
                &header,
                &payload_words,
            )
        };
        if dispatch_result.is_err() || self.device_lost.load(std::sync::atomic::Ordering::Acquire) {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        let staged_words = self.read_slot_mutable_words(handle, &snapshot.ranges)?;
        let completion = completion_from_slot_words(&staged_words, &snapshot.ranges)?;
        if completion.schema_version != 1
            || completion.slot != handle.slot()
            || completion.slot_generation != handle.generation()
            || completion.status != SLEEP_STATUS_STAGED
            || join_pair(completion.input_generation) != request.input_generation
            || join_pair(completion.output_generation) != request.expected_output_generation
            || completion.output_weight_bank != u32::from(snapshot.active_weight_bank ^ 1)
            || completion.replay_span_count != replay.synapse_spans.len() as u32
            || join_pair(completion.job_id) != job_id.raw()
            || completion.reserved != [0; 2]
        {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        let (lifetime, fast) = resolved_weight_banks(&staged_words, &snapshot, false)?;
        let output_bank = snapshot.active_weight_bank ^ 1;
        let output_digest = compute_gpu_sleep_output_weight_digest(
            handle.phenotype_hash(),
            handle.class_id(),
            request.expected_output_generation,
            output_bank,
            &lifetime,
            &fast,
        )?;
        let eligibility_reset_generation = snapshot
            .active_eligibility_generation
            .checked_add(1)
            .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
        let replay_journal_generation = snapshot
            .replay_journal_generation
            .checked_add(1)
            .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
        let eligibility_output_digest =
            expected_eligibility_reset_digest(&snapshot, eligibility_reset_generation, 0)?;
        let replay_journal_output_digest =
            expected_replay_reset_digest(&state, replay_journal_generation, replay)?;
        let promoted_fast_l1 = completion.promoted_fast_l1_q12 as f32 / SLEEP_DIAGNOSTIC_Q12;
        let replay_induced_fast_l1 =
            completion.replay_induced_fast_l1_q12 as f32 / SLEEP_DIAGNOSTIC_Q12;
        if !promoted_fast_l1.is_finite() || !replay_induced_fast_l1.is_finite() {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        let mut staged = ConsolidationStagedOutput {
            job_id,
            output_generation: request.expected_output_generation,
            output_weight_bank: output_bank,
            output_digest,
            eligibility_reset_generation,
            output_eligibility_bank: 0,
            eligibility_output_digest,
            replay_journal_generation,
            replay_journal_cursor: 0,
            replay_journal_event_count: 0,
            replay_journal_output_digest,
            staging_digest: [0; 4],
            promoted_fast_l1_bits: promoted_fast_l1.to_bits(),
            replay_induced_fast_l1_bits: replay_induced_fast_l1.to_bits(),
        };
        staged.staging_digest = staged.recompute_staging_digest(
            request,
            snapshot.sleep_plan.eligibility_reset_policy_raw(),
            snapshot.sleep_plan.replay_consume_policy_raw(),
        )?;
        let receipt = GpuSleepStagingReceipt {
            handle,
            cycle_id: request.cycle_id,
            phenotype_hash: handle.phenotype_hash(),
            input_generation: request.input_generation,
            input_digest: request.input_digest,
            replay_digest: request.replay_digest,
            staged,
            hardware_receipt_generation: self.hardware.generation,
        };
        self.sleep_jobs.insert(
            job_id.raw(),
            GpuSleepJobState {
                handle,
                request: *request,
                replay: replay.clone(),
                header,
                payload_words,
                reset_word_count,
                receipt,
                restored_completed: false,
            },
        );
        Ok(job_id)
    }

    pub(crate) fn sleep_slot_snapshot(
        &self,
        handle: GpuBrainHandle,
    ) -> Result<SleepSlotSnapshot, ScaffoldContractError> {
        if self.device_lost.load(std::sync::atomic::Ordering::Acquire)
            || !matches!(self.state, crate::GpuBackendState::Ready)
        {
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        self.validate_handle_backend(handle)?;
        let bucket = self
            .class_buckets
            .get(&handle.class_id().raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let resident = bucket
            .slots
            .get(handle.slot() as usize)
            .and_then(Option::as_ref)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        if !bucket.contains(handle)
            || resident.pending_eligibility.is_some()
            || resident.pending_eligibility_record.is_some()
        {
            return Err(ScaffoldContractError::LearningReplayRejected);
        }
        resident.sleep_plan.validate_contract()?;
        Ok(SleepSlotSnapshot {
            brain_slot: resident.brain_slot.clone(),
            ranges: resident.ranges.clone(),
            active_weight_bank: resident.active_weight_bank,
            active_weight_generation: resident.active_weight_generation,
            active_eligibility_bank: resident.active_eligibility_bank,
            active_eligibility_generation: resident.active_eligibility_generation,
            replay_journal_generation: resident.replay_journal_generation,
            transaction_generation: resident.transaction_generation,
            sleep_plan: resident.sleep_plan,
        })
    }

    pub(crate) fn read_slot_mutable_words(
        &self,
        handle: GpuBrainHandle,
        ranges: &GpuFixedSlotRanges,
    ) -> Result<Vec<u32>, ScaffoldContractError> {
        let bucket = self
            .class_buckets
            .get(&handle.class_id().raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        read_gpu_words(
            &self.device,
            &self.queue,
            bucket.buffers.neural_buffers()[6],
            ranges.mutable_state_words.clone(),
            "closed-loop-sleep-mutable-readback",
        )
    }

    #[cfg(feature = "gpu-tests")]
    pub fn read_immutable_genetic_weights_for_test(
        &mut self,
        handle: GpuBrainHandle,
    ) -> Result<Vec<f32>, ScaffoldContractError> {
        self.ensure_ready()?;
        let snapshot = self.sleep_slot_snapshot(handle)?;
        let bucket = self
            .class_buckets
            .get(&handle.class_id().raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let words = read_gpu_words(
            &self.device,
            &self.queue,
            bucket.buffers.neural_buffers()[3],
            snapshot.ranges.layout.genetic_weight_words.clone(),
            "closed-loop-test-genetic-readback",
        )?;
        let active_words = words
            .get(..snapshot.brain_slot.record().synapse_count as usize)
            .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
        finite_f32_words(active_words)
    }

    #[cfg(feature = "gpu-tests")]
    pub fn read_active_lifetime_weights_for_test(
        &mut self,
        handle: GpuBrainHandle,
    ) -> Result<Vec<f32>, ScaffoldContractError> {
        self.ensure_ready()?;
        let snapshot = self.sleep_slot_snapshot(handle)?;
        let words = self.read_slot_mutable_words(handle, &snapshot.ranges)?;
        resolved_weight_banks(&words, &snapshot, true).map(|(lifetime, _)| lifetime)
    }

    #[cfg(feature = "gpu-tests")]
    pub fn learning_state_snapshot_for_test(
        &mut self,
        handle: GpuBrainHandle,
    ) -> Result<GpuLearningStateSnapshot, ScaffoldContractError> {
        self.ensure_ready()?;
        let snapshot = self.sleep_slot_snapshot(handle)?;
        let words = self.read_slot_mutable_words(handle, &snapshot.ranges)?;
        let state = learning_state_from_slot_words(&words, &snapshot.ranges)?;
        validate_learning_state(&snapshot, &state)?;
        let recurrent_count = snapshot.brain_slot.record().recurrent_synapse_count as usize;
        let decoder_count = snapshot
            .brain_slot
            .record()
            .synapse_count
            .checked_sub(snapshot.brain_slot.record().recurrent_synapse_count)
            .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?
            as usize;
        let recurrent_eligibility_nonzero = [
            &snapshot.ranges.layout.recurrent_eligibility_words,
            &snapshot.ranges.layout.recurrent_eligibility_bank_1_words,
        ]
        .into_iter()
        .try_fold(0_usize, |total, range| {
            let values = absolute_slice(&words, snapshot.ranges.mutable_state_words.start, range)?
                .get(..recurrent_count)
                .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
            Ok::<_, ScaffoldContractError>(
                total + values.iter().filter(|value| **value != 0).count(),
            )
        })?;
        let decoder_eligibility_nonzero = [
            &snapshot.ranges.layout.decoder_eligibility_words,
            &snapshot.ranges.layout.decoder_eligibility_bank_1_words,
        ]
        .into_iter()
        .try_fold(0_usize, |total, range| {
            let values = absolute_slice(&words, snapshot.ranges.mutable_state_words.start, range)?
                .get(..decoder_count)
                .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
            Ok::<_, ScaffoldContractError>(
                total + values.iter().filter(|value| **value != 0).count(),
            )
        })?;
        let replay_event_words = usize::try_from(state.replay_event_capacity)
            .ok()
            .and_then(|count| count.checked_mul(REPLAY_EVENT_WORDS))
            .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
        let replay_event_nonzero_words = absolute_slice(
            &words,
            snapshot.ranges.mutable_state_words.start,
            &snapshot.ranges.layout.replay_event_words,
        )?
        .get(..replay_event_words)
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?
        .iter()
        .filter(|value| **value != 0)
        .count();
        let replay_sample_nonzero_words = absolute_slice(
            &words,
            snapshot.ranges.mutable_state_words.start,
            &snapshot.ranges.layout.replay_sample_words,
        )?
        .get(..state.replay_sample_capacity as usize)
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?
        .iter()
        .filter(|value| **value != 0)
        .count();
        let pending_eligibility_nonzero_words = absolute_slice(
            &words,
            snapshot.ranges.mutable_state_words.start,
            &snapshot.ranges.layout.pending_eligibility_words,
        )?
        .iter()
        .filter(|value| **value != 0)
        .count();
        Ok(GpuLearningStateSnapshot {
            active_weight_bank: state.active_weight_bank as u8,
            active_weight_generation: join_pair([
                state.active_weight_generation_lo,
                state.active_weight_generation_hi,
            ]),
            active_eligibility_bank: state.active_eligibility_bank as u8,
            active_eligibility_generation: join_pair([
                state.active_eligibility_generation_lo,
                state.active_eligibility_generation_hi,
            ]),
            replay_journal_generation: join_pair([
                state.replay_generation_lo,
                state.replay_generation_hi,
            ]),
            replay_journal_cursor: state.replay_cursor,
            replay_journal_event_count: state.replay_event_count,
            transaction_generation: join_pair([
                state.transaction_generation_lo,
                state.transaction_generation_hi,
            ]),
            recurrent_eligibility_nonzero,
            decoder_eligibility_nonzero,
            replay_event_nonzero_words,
            replay_sample_nonzero_words,
            pending_eligibility_nonzero_words,
        })
    }

    /// Test-only replay ablation that preserves event and synapse identities
    /// while replacing every live captured eligibility value with zero.
    #[cfg(feature = "gpu-tests")]
    pub fn zero_replay_eligibility_samples_for_test(
        &mut self,
        handle: GpuBrainHandle,
    ) -> Result<(), ScaffoldContractError> {
        self.ensure_ready()?;
        let snapshot = self.sleep_slot_snapshot(handle)?;
        let words = self.read_slot_mutable_words(handle, &snapshot.ranges)?;
        let state = learning_state_from_slot_words(&words, &snapshot.ranges)?;
        validate_learning_state(&snapshot, &state)?;

        let range = snapshot.ranges.layout.replay_sample_words.clone();
        let mut replacement =
            absolute_slice(&words, snapshot.ranges.mutable_state_words.start, &range)?.to_vec();
        let capacity = usize::try_from(state.replay_event_capacity)
            .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?;
        let span_count = usize::try_from(state.replay_span_count)
            .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?;
        let oldest = if state.replay_event_count == state.replay_event_capacity {
            state.replay_cursor
        } else {
            0
        };
        for capture_index in 0..span_count {
            for logical_offset in 0..state.replay_event_count {
                let physical_event = (oldest + logical_offset) % state.replay_event_capacity;
                let index = capture_index
                    .checked_mul(capacity)
                    .and_then(|base| base.checked_add(physical_event as usize))
                    .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
                let physical_event = u16::try_from(physical_event)
                    .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?;
                *replacement
                    .get_mut(index)
                    .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)? =
                    pack_replay_eligibility_sample(physical_event, 0);
            }
        }

        let bucket = self
            .class_buckets
            .get(&handle.class_id().raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        self.queue.write_buffer(
            bucket.buffers.neural_buffers()[6],
            u64::from(range.start) * 4,
            bytemuck::cast_slice(&replacement),
        );
        Ok(())
    }

    #[cfg(feature = "gpu-tests")]
    pub fn slot_full_digest_for_test(
        &mut self,
        handle: GpuBrainHandle,
    ) -> Result<[u64; 4], ScaffoldContractError> {
        self.ensure_ready()?;
        let snapshot = self.sleep_slot_snapshot(handle)?;
        let bucket = self
            .class_buckets
            .get(&handle.class_id().raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let plan_words = read_gpu_words(
            &self.device,
            &self.queue,
            bucket.buffers.neural_buffers()[2],
            snapshot.ranges.immutable_plan_words.clone(),
            "closed-loop-test-slot-plan-digest",
        )?;
        let weight_words = read_gpu_words(
            &self.device,
            &self.queue,
            bucket.buffers.neural_buffers()[3],
            snapshot.ranges.immutable_weight_words.clone(),
            "closed-loop-test-slot-weight-digest",
        )?;
        let mutable_words = self.read_slot_mutable_words(handle, &snapshot.ranges)?;
        let mut digest = CanonicalDigestBuilder::new(b"ALIFE-GPU-SLEEP-SLOT-FULL-TEST-V1");
        digest.write_u16(handle.class_id().raw());
        digest.write_u32(handle.slot());
        digest.write_u32(handle.generation());
        digest.write_u64(handle.organism_id().raw());
        for words in [&plan_words, &weight_words, &mutable_words] {
            digest.write_sequence_len(words.len());
            for word in words {
                digest.write_u32(*word);
            }
        }
        Ok(digest.finish256())
    }
}

pub(crate) fn sleep_commit_key(handle: GpuBrainHandle, cycle_id: u64) -> (u16, u32, u32, u64) {
    (
        handle.class_id().raw(),
        handle.slot(),
        handle.generation(),
        cycle_id,
    )
}

fn validate_learning_state(
    snapshot: &SleepSlotSnapshot,
    state: &GpuSlotLearningStateRecord,
) -> Result<(), ScaffoldContractError> {
    if state.schema_version != u32::from(alife_core::SchemaVersions::CURRENT.learning.raw())
        || state.active_weight_bank != u32::from(snapshot.active_weight_bank)
        || state.active_eligibility_bank != u32::from(snapshot.active_eligibility_bank)
        || state.pending_valid != 0
        || join_pair([
            state.active_weight_generation_lo,
            state.active_weight_generation_hi,
        ]) != snapshot.active_weight_generation
        || join_pair([
            state.active_eligibility_generation_lo,
            state.active_eligibility_generation_hi,
        ]) != snapshot.active_eligibility_generation
        || join_pair([state.replay_generation_lo, state.replay_generation_hi])
            != snapshot.replay_journal_generation
        || join_pair([
            state.transaction_generation_lo,
            state.transaction_generation_hi,
        ]) != snapshot.transaction_generation
        || state.replay_event_capacity == 0
        || state.replay_event_capacity > 65_536
        || state.replay_cursor >= state.replay_event_capacity
        || state.replay_event_count > state.replay_event_capacity
        || state.replay_span_count == 0
        || state.replay_sample_capacity
            != state
                .replay_event_capacity
                .checked_mul(state.replay_span_count)
                .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?
    {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    Ok(())
}

pub(crate) fn build_sleep_upload(
    handle: GpuBrainHandle,
    snapshot: &SleepSlotSnapshot,
    request: &GpuConsolidationRequest,
    replay: &BoundedReplayBatch,
    job_id: GpuSleepJobId,
) -> Result<(GpuSleepHeader, Vec<u32>), ScaffoldContractError> {
    let request_offset = 0_u32;
    let replay_event_offset = CONSOLIDATION_REQUEST_WORDS as u32;
    let event_words = u32::try_from(replay.events.len())
        .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?
        .checked_mul(REPLAY_EVENT_WORDS as u32)
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
    let replay_span_offset = replay_event_offset
        .checked_add(event_words)
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
    let span_words = u32::try_from(replay.synapse_spans.len())
        .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?
        .checked_mul(REPLAY_SPAN_WORDS as u32)
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
    let replay_sample_offset = replay_span_offset
        .checked_add(span_words)
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
    let mut payload = Vec::new();
    payload.extend_from_slice(bytemuck::cast_slice(std::slice::from_ref(
        &GpuConsolidationRequestRecord::from(request),
    )));
    for event in &replay.events {
        let row = encode_replay_event(*event);
        payload.extend_from_slice(bytemuck::cast_slice(std::slice::from_ref(&row)));
    }
    for span in &replay.synapse_spans {
        let row = GpuReplaySynapseSpanRecord {
            local_synapse_id: span.local_synapse_id,
            sample_start: span.sample_start,
            sample_count: span.sample_count,
            reserved: span.reserved,
        };
        payload.extend_from_slice(row.words());
    }
    payload.extend(
        replay.eligibility_samples.iter().map(|sample| {
            pack_replay_eligibility_sample(sample.event_index, sample.eligibility_q15)
        }),
    );
    if payload.len()
        != usize::try_from(replay_sample_offset)
            .ok()
            .and_then(|start| start.checked_add(replay.eligibility_samples.len()))
            .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?
    {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    let header = GpuSleepHeader {
        schema_version: u32::from(GPU_CONSOLIDATION_REQUEST_SCHEMA_VERSION),
        class_id: u32::from(handle.class_id().raw()),
        slot: handle.slot(),
        slot_generation: handle.generation(),
        brain_slot_index: snapshot.brain_slot.brain_slot_index(),
        request_offset,
        replay_event_offset,
        replay_event_count: replay.events.len() as u32,
        replay_span_offset,
        replay_span_count: replay.synapse_spans.len() as u32,
        replay_sample_offset,
        replay_sample_count: replay.eligibility_samples.len() as u32,
        synapse_count: snapshot.brain_slot.record().synapse_count,
        completion_offset: snapshot.brain_slot.record().diagnostic_offset,
        job_id_lo: job_id.raw() as u32,
        job_id_hi: (job_id.raw() >> 32) as u32,
        cycle_id_lo: request.cycle_id as u32,
        cycle_id_hi: (request.cycle_id >> 32) as u32,
        flags: 0,
        reserved: 0,
    };
    debug_assert_eq!(
        std::mem::size_of::<GpuSleepHeader>() / 4,
        SLEEP_HEADER_WORDS
    );
    Ok((header, payload))
}

pub(crate) fn reset_word_count(
    snapshot: &SleepSlotSnapshot,
    state: &GpuSlotLearningStateRecord,
) -> Result<u32, ScaffoldContractError> {
    let decoder_count = snapshot
        .brain_slot
        .record()
        .synapse_count
        .checked_sub(snapshot.brain_slot.record().recurrent_synapse_count)
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
    let event_words = state
        .replay_event_capacity
        .checked_mul(REPLAY_EVENT_WORDS as u32)
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
    Ok([
        snapshot.brain_slot.record().recurrent_synapse_count,
        decoder_count,
        event_words,
        state.replay_sample_capacity,
        state.replay_span_count,
        36,
    ]
    .into_iter()
    .max()
    .unwrap_or(0))
}

fn resolved_weight_banks(
    words: &[u32],
    snapshot: &SleepSlotSnapshot,
    active: bool,
) -> Result<(Vec<f32>, Vec<f32>), ScaffoldContractError> {
    let use_bank_one = if active {
        snapshot.active_weight_bank == 1
    } else {
        snapshot.active_weight_bank == 0
    };
    let lifetime_range = if use_bank_one {
        &snapshot.ranges.layout.lifetime_weight_bank_1_words
    } else {
        &snapshot.ranges.layout.lifetime_weight_words
    };
    let fast_range = if use_bank_one {
        &snapshot.ranges.layout.fast_weight_bank_1_words
    } else {
        &snapshot.ranges.layout.fast_weight_words
    };
    let synapse_count = snapshot.brain_slot.record().synapse_count as usize;
    let lifetime = finite_f32_words(
        absolute_slice(
            words,
            snapshot.ranges.mutable_state_words.start,
            lifetime_range,
        )?
        .get(..synapse_count)
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?,
    )?;
    let fast = finite_f32_words(
        absolute_slice(words, snapshot.ranges.mutable_state_words.start, fast_range)?
            .get(..synapse_count)
            .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?,
    )?;
    Ok((lifetime, fast))
}

fn expected_eligibility_reset_digest(
    snapshot: &SleepSlotSnapshot,
    generation: u64,
    bank: u8,
) -> Result<[u64; 4], ScaffoldContractError> {
    let recurrent = snapshot.brain_slot.record().recurrent_synapse_count as usize;
    let decoder = snapshot
        .brain_slot
        .record()
        .synapse_count
        .checked_sub(snapshot.brain_slot.record().recurrent_synapse_count)
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)? as usize;
    eligibility_reset_digest(
        generation, bank, &[0; 0], recurrent, &[0; 0], recurrent, &[0; 0], decoder, &[0; 0],
        decoder,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn eligibility_reset_digest(
    generation: u64,
    bank: u8,
    recurrent_0: &[u32],
    recurrent_0_len: usize,
    recurrent_1: &[u32],
    recurrent_1_len: usize,
    decoder_0: &[u32],
    decoder_0_len: usize,
    decoder_1: &[u32],
    decoder_1_len: usize,
) -> Result<[u64; 4], ScaffoldContractError> {
    if generation == 0 || bank > 1 {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    let mut digest = CanonicalDigestBuilder::new(ELIGIBILITY_RESET_DIGEST_DOMAIN);
    digest.write_u64(generation);
    digest.write_u8(bank);
    for (values, len) in [
        (recurrent_0, recurrent_0_len),
        (recurrent_1, recurrent_1_len),
        (decoder_0, decoder_0_len),
        (decoder_1, decoder_1_len),
    ] {
        if !values.is_empty() && values.len() != len {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        digest.write_sequence_len(len);
        if values.is_empty() {
            for _ in 0..len {
                digest.write_u32(0);
            }
        } else {
            for value in values {
                digest.write_u32(*value);
            }
        }
    }
    Ok(digest.finish256())
}

fn eligibility_reset_digest_from_words(
    words: &[u32],
    snapshot: &SleepSlotSnapshot,
    generation: u64,
    bank: u8,
) -> Result<[u64; 4], ScaffoldContractError> {
    let ranges = &snapshot.ranges;
    let recurrent_count = snapshot.brain_slot.record().recurrent_synapse_count as usize;
    let decoder_count = snapshot
        .brain_slot
        .record()
        .synapse_count
        .checked_sub(snapshot.brain_slot.record().recurrent_synapse_count)
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?
        as usize;
    let r0 = absolute_slice(
        words,
        ranges.mutable_state_words.start,
        &ranges.layout.recurrent_eligibility_words,
    )?
    .get(..recurrent_count)
    .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
    let r1 = absolute_slice(
        words,
        ranges.mutable_state_words.start,
        &ranges.layout.recurrent_eligibility_bank_1_words,
    )?
    .get(..recurrent_count)
    .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
    let d0 = absolute_slice(
        words,
        ranges.mutable_state_words.start,
        &ranges.layout.decoder_eligibility_words,
    )?
    .get(..decoder_count)
    .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
    let d1 = absolute_slice(
        words,
        ranges.mutable_state_words.start,
        &ranges.layout.decoder_eligibility_bank_1_words,
    )?
    .get(..decoder_count)
    .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
    eligibility_reset_digest(
        generation,
        bank,
        r0,
        r0.len(),
        r1,
        r1.len(),
        d0,
        d0.len(),
        d1,
        d1.len(),
    )
}

fn expected_replay_reset_digest(
    state: &GpuSlotLearningStateRecord,
    generation: u64,
    replay: &BoundedReplayBatch,
) -> Result<[u64; 4], ScaffoldContractError> {
    let spans = replay
        .synapse_spans
        .iter()
        .enumerate()
        .map(|(index, span)| {
            Ok(GpuReplaySynapseSpanRecord {
                local_synapse_id: span.local_synapse_id,
                sample_start: u32::try_from(index)
                    .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?
                    .checked_mul(state.replay_event_capacity)
                    .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?,
                sample_count: 0,
                reserved: 0,
            })
        })
        .collect::<Result<Vec<_>, ScaffoldContractError>>()?;
    replay_reset_digest(
        generation,
        0,
        0,
        state.replay_event_capacity,
        state.replay_sample_capacity,
        &[],
        &[],
        &spans,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn replay_reset_digest(
    generation: u64,
    cursor: u32,
    event_count: u32,
    event_capacity: u32,
    sample_capacity: u32,
    event_words: &[u32],
    sample_words: &[u32],
    spans: &[GpuReplaySynapseSpanRecord],
) -> Result<[u64; 4], ScaffoldContractError> {
    if generation == 0 || cursor >= event_capacity || event_count > event_capacity {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    let expected_event_words = event_capacity as usize * REPLAY_EVENT_WORDS;
    if (!event_words.is_empty() && event_words.len() != expected_event_words)
        || (!sample_words.is_empty() && sample_words.len() != sample_capacity as usize)
    {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    let mut digest = CanonicalDigestBuilder::new(REPLAY_RESET_DIGEST_DOMAIN);
    digest.write_u64(generation);
    digest.write_u32(cursor);
    digest.write_u32(event_count);
    digest.write_u32(event_capacity);
    digest.write_u32(sample_capacity);
    digest.write_sequence_len(expected_event_words);
    if event_words.is_empty() {
        for _ in 0..expected_event_words {
            digest.write_u32(0);
        }
    } else {
        for value in event_words {
            digest.write_u32(*value);
        }
    }
    digest.write_sequence_len(sample_capacity as usize);
    if sample_words.is_empty() {
        for _ in 0..sample_capacity {
            digest.write_u32(0);
        }
    } else {
        for value in sample_words {
            digest.write_u32(*value);
        }
    }
    digest.write_sequence_len(spans.len());
    for span in spans {
        digest.write_u32(span.local_synapse_id);
        digest.write_u32(span.sample_start);
        digest.write_u32(span.sample_count);
        digest.write_u32(span.reserved);
    }
    Ok(digest.finish256())
}

fn replay_reset_digest_from_words(
    words: &[u32],
    ranges: &GpuFixedSlotRanges,
    state: &GpuSlotLearningStateRecord,
    generation: u64,
) -> Result<[u64; 4], ScaffoldContractError> {
    let events = absolute_slice(
        words,
        ranges.mutable_state_words.start,
        &ranges.layout.replay_event_words,
    )?;
    let samples = absolute_slice(
        words,
        ranges.mutable_state_words.start,
        &ranges.layout.replay_sample_words,
    )?;
    let all_spans = pod_slice_from_absolute::<GpuReplaySynapseSpanRecord>(
        words,
        ranges.mutable_state_words.start,
        &ranges.layout.replay_span_words,
    )?;
    let spans = all_spans
        .get(..state.replay_span_count as usize)
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
    replay_reset_digest(
        generation,
        state.replay_cursor,
        state.replay_event_count,
        state.replay_event_capacity,
        state.replay_sample_capacity,
        events,
        samples,
        spans,
    )
}

fn learning_state_from_slot_words(
    words: &[u32],
    ranges: &GpuFixedSlotRanges,
) -> Result<GpuSlotLearningStateRecord, ScaffoldContractError> {
    pod_from_absolute(
        words,
        ranges.mutable_state_words.start,
        ranges.layout.learning_state_words.start,
    )
}

fn completion_from_slot_words(
    words: &[u32],
    ranges: &GpuFixedSlotRanges,
) -> Result<GpuSleepCompletionRecord, ScaffoldContractError> {
    debug_assert_eq!(
        std::mem::size_of::<GpuSleepCompletionRecord>() / 4,
        SLEEP_COMPLETION_WORDS
    );
    pod_from_absolute(
        words,
        ranges.mutable_state_words.start,
        ranges.layout.diagnostic_words.start,
    )
}

fn pod_from_absolute<T: Pod + Copy>(
    words: &[u32],
    base: u32,
    absolute_start: u32,
) -> Result<T, ScaffoldContractError> {
    let local = absolute_start
        .checked_sub(base)
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)? as usize;
    let count = std::mem::size_of::<T>() / 4;
    let slice = words
        .get(local..local + count)
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
    Ok(bytemuck::pod_read_unaligned(bytemuck::cast_slice(slice)))
}

fn pod_slice_from_absolute<T: Pod + Copy>(
    words: &[u32],
    base: u32,
    range: &Range<u32>,
) -> Result<Vec<T>, ScaffoldContractError> {
    let slice = absolute_slice(words, base, range)?;
    if slice.len() % (std::mem::size_of::<T>() / 4) != 0 {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    Ok(bytemuck::cast_slice::<u32, T>(slice).to_vec())
}

fn absolute_slice<'a>(
    words: &'a [u32],
    base: u32,
    range: &Range<u32>,
) -> Result<&'a [u32], ScaffoldContractError> {
    let start = range
        .start
        .checked_sub(base)
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)? as usize;
    let end = range
        .end
        .checked_sub(base)
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)? as usize;
    words
        .get(start..end)
        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)
}

fn read_gpu_words(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    source: &wgpu::Buffer,
    words: Range<u32>,
    label: &'static str,
) -> Result<Vec<u32>, ScaffoldContractError> {
    let count = words
        .end
        .checked_sub(words.start)
        .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
    if count == 0 {
        return Err(ScaffoldContractError::NeuralBackendUnavailable);
    }
    let size = u64::from(count) * 4;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) });
    encoder.copy_buffer_to_buffer(source, u64::from(words.start) * 4, &readback, 0, size);
    let command = encoder.finish();
    let (sender, receiver) = std::sync::mpsc::channel();
    command.map_buffer_on_submit(&readback, wgpu::MapMode::Read, 0..size, move |result| {
        let _ = sender.send(result);
    });
    let submission = queue.submit(Some(command));
    if device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .is_err()
        || receiver.recv().ok().and_then(Result::ok).is_none()
    {
        readback.unmap();
        return Err(ScaffoldContractError::NeuralBackendUnavailable);
    }
    let mapped = readback.slice(..size).get_mapped_range();
    let result = bytemuck::cast_slice::<u8, u32>(&mapped).to_vec();
    drop(mapped);
    readback.unmap();
    Ok(result)
}

fn finite_f32_words(words: &[u32]) -> Result<Vec<f32>, ScaffoldContractError> {
    let values = words
        .iter()
        .map(|word| f32::from_bits(*word))
        .collect::<Vec<_>>();
    if values.iter().any(|value| !value.is_finite()) {
        return Err(ScaffoldContractError::NonFiniteFloat);
    }
    Ok(values)
}

fn encode_replay_event(event: SleepReplayEvent) -> GpuReplayEventRecord {
    GpuReplayEventRecord {
        sequence_id: split_pair(event.sequence_id.raw()),
        originating_tick: split_pair(event.originating_tick.raw()),
        frame_digest: split_digest(event.frame_digest.0),
        candidate_feature_digest: split_digest2(event.candidate_feature_digest.0),
        action_id: event.action_id.raw(),
        family: u32::from(event.family.raw()),
        reward_prediction_error: event.modulator.reward_prediction_error(),
        pain: event.modulator.pain(),
        homeostatic_improvement: event.modulator.homeostatic_improvement(),
        frustration: event.modulator.frustration(),
        novelty: event.modulator.novelty(),
        modulator_value: event.modulator.value(),
    }
}

fn decode_replay_event(
    row: GpuReplayEventRecord,
) -> Result<SleepReplayEvent, ScaffoldContractError> {
    let modulator = NeuromodulatorSample::from_components(
        row.reward_prediction_error,
        row.pain,
        row.homeostatic_improvement,
        row.frustration,
        row.novelty,
    )?;
    if modulator.value().to_bits() != row.modulator_value.to_bits() {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    Ok(SleepReplayEvent {
        sequence_id: ExperienceSequenceId(join_pair(row.sequence_id)),
        originating_tick: Tick::new(join_pair(row.originating_tick)),
        frame_digest: PerceptionFrameDigest(join_digest(row.frame_digest)),
        candidate_feature_digest: CandidateFeatureDigest(join_digest2(
            row.candidate_feature_digest,
        )),
        action_id: ActionId(row.action_id),
        family: CandidateActionFamily::try_from_raw(
            u8::try_from(row.family)
                .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?,
        )?,
        modulator,
    })
}

const fn split_pair(value: u64) -> [u32; 2] {
    [value as u32, (value >> 32) as u32]
}

const fn join_pair(value: [u32; 2]) -> u64 {
    value[0] as u64 | ((value[1] as u64) << 32)
}

fn split_digest(values: [u64; 4]) -> [u32; 8] {
    let mut output = [0; 8];
    for (index, value) in values.into_iter().enumerate() {
        output[index * 2] = value as u32;
        output[index * 2 + 1] = (value >> 32) as u32;
    }
    output
}

fn split_digest2(values: [u64; 2]) -> [u32; 4] {
    let mut output = [0; 4];
    for (index, value) in values.into_iter().enumerate() {
        output[index * 2] = value as u32;
        output[index * 2 + 1] = (value >> 32) as u32;
    }
    output
}

fn join_digest(values: [u32; 8]) -> [u64; 4] {
    std::array::from_fn(|index| join_pair([values[index * 2], values[index * 2 + 1]]))
}

fn join_digest2(values: [u32; 4]) -> [u64; 2] {
    std::array::from_fn(|index| join_pair([values[index * 2], values[index * 2 + 1]]))
}
