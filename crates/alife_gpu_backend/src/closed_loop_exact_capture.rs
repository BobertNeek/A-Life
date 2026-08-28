//! Nonblocking exact-population GPU checkpoint capture.
//!
//! This is a persistence transaction boundary. It copies one immutable view of
//! every admitted resident into one staging arena and maps that arena once.
//! It never keeps a continuously synchronized CPU neural mirror.

use std::{
    ops::Range,
    sync::mpsc::{self, TryRecvError},
};

use alife_core::{
    BrainClassId, BrainWorkReceipt, CanonicalDigestBuilder, GpuConsolidationRequest,
    NeuralThrottleDecision, OrganismId, OutcomeCreditReplayKey, PhenotypeHash,
    ScaffoldContractError, Tick,
};

use crate::{
    closed_loop_buffers::{GpuBrainSlot, GpuFixedSlotRanges, GpuFixedSlotUpload},
    GpuActivityRuntimeSnapshot, GpuBrainHandle, GpuClosedLoopBackend,
    GpuCompactCheckpointAuthorityV1, GpuPendingEligibilityRecord, GpuSleepStagingReceipt,
    GpuSlotLearningStateRecord, GpuV11Checkpoint, PendingEligibilityReceipt,
    PendingEligibilityRestoreParts,
};

pub const GPU_EXACT_POPULATION_CAPTURE_SCHEMA_VERSION: u16 = 1;
const GPU_EXACT_POPULATION_SET_DIGEST_DOMAIN: &[u8] = b"alife.gpu.exact-population-set.v1";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GpuExactPopulationCaptureMetricsV1 {
    pub gpu_copy_submissions: u64,
    pub map_operations: u64,
    pub poll_calls: u64,
    pub bytes_copied: u64,
    pub completed_captures: u64,
    pub released_staging_bytes: u64,
}

/// Backend-owned authority bound to one captured fixed-slot row.
#[derive(Debug, Clone, PartialEq)]
pub struct GpuExactPopulationCaptureIdentityV1 {
    pub schema_version: u16,
    pub organism_id: OrganismId,
    pub class_id: BrainClassId,
    pub phenotype_hash: PhenotypeHash,
    pub slot: u32,
    pub slot_generation: u32,
    /// Monotonic resident transaction epoch that also fences graph mutation.
    pub graph_epoch: u64,
    pub active_activation_side: u8,
    pub logical_dispatch_generation: u64,
    pub active_weight_generation: u64,
    pub active_weight_bank: u8,
    pub active_eligibility_generation: u64,
    pub inactive_eligibility_generation: u64,
    pub active_eligibility_bank: u8,
    pub replay_journal_generation: u64,
    pub replay_journal_cursor: u32,
    pub replay_journal_event_count: u32,
    pub transaction_generation: u64,
    pub activity_sequence_cursor: u64,
    pub brain_atp_q16: u32,
    pub last_world_atp_tick: Option<u64>,
    pub last_activity_dispatch_generation: u64,
    pub last_activity_frame_digest: [u64; 4],
    pub last_completed_gpu_time_ns: u64,
    pub last_throttle: Option<NeuralThrottleDecision>,
    pub last_work: Option<BrainWorkReceipt>,
    pub v11: GpuV11Checkpoint,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuExactPopulationCaptureRowV1 {
    identity: GpuExactPopulationCaptureIdentityV1,
    pub(crate) brain_slot: GpuBrainSlot,
    pub(crate) ranges: GpuFixedSlotRanges,
    pub(crate) last_learning_replay_key: Option<OutcomeCreditReplayKey>,
    pub(crate) pending_eligibility: Option<PendingEligibilityReceipt>,
    pub(crate) pending_eligibility_record: Option<GpuPendingEligibilityRecord>,
    activity_snapshot: GpuActivityRuntimeSnapshot,
    completed_sleep: Option<GpuExactCompletedSleepCaptureV1>,
    brain_slot_bytes: Vec<u8>,
    phenotype_identity_bytes: Vec<u8>,
    immutable_plan_bytes: Vec<u8>,
    immutable_weight_bytes: Vec<u8>,
    mutable_state_bytes: Vec<u8>,
}

impl GpuExactPopulationCaptureRowV1 {
    pub const fn identity(&self) -> &GpuExactPopulationCaptureIdentityV1 {
        &self.identity
    }

    pub fn brain_slot_bytes(&self) -> &[u8] {
        &self.brain_slot_bytes
    }

    pub fn phenotype_identity_bytes(&self) -> &[u8] {
        &self.phenotype_identity_bytes
    }

    pub fn immutable_plan_bytes(&self) -> &[u8] {
        &self.immutable_plan_bytes
    }

    pub fn immutable_weight_bytes(&self) -> &[u8] {
        &self.immutable_weight_bytes
    }

    pub fn mutable_state_bytes(&self) -> &[u8] {
        &self.mutable_state_bytes
    }

    pub const fn activity_snapshot(&self) -> &GpuActivityRuntimeSnapshot {
        &self.activity_snapshot
    }

    pub const fn completed_sleep(&self) -> Option<&GpuExactCompletedSleepCaptureV1> {
        self.completed_sleep.as_ref()
    }

    pub fn compact_checkpoint_authority(
        &self,
    ) -> Result<GpuCompactCheckpointAuthorityV1, ScaffoldContractError> {
        let pending = self
            .pending_eligibility
            .map(|receipt| {
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
            })
            .transpose()?;
        GpuCompactCheckpointAuthorityV1::try_new(
            self.identity.active_activation_side,
            self.identity.logical_dispatch_generation,
            self.identity.active_weight_generation,
            self.identity.active_weight_bank,
            self.identity.active_eligibility_generation,
            self.identity.inactive_eligibility_generation,
            self.identity.active_eligibility_bank,
            self.identity.replay_journal_generation,
            self.identity.replay_journal_cursor,
            self.identity.replay_journal_event_count,
            self.identity.transaction_generation,
            self.last_learning_replay_key,
            pending,
            self.identity.v11.clone(),
        )
    }

    pub(crate) fn fixed_slot_upload(&self) -> Result<GpuFixedSlotUpload, ScaffoldContractError> {
        let words = |bytes: &[u8]| -> Result<Vec<u32>, ScaffoldContractError> {
            bytemuck::try_cast_slice::<u8, u32>(bytes)
                .map(<[u32]>::to_vec)
                .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)
        };
        Ok(GpuFixedSlotUpload::from_existing_slot(
            self.brain_slot.clone(),
            self.ranges.clone(),
            words(&self.immutable_plan_bytes)?,
            words(&self.immutable_weight_bytes)?,
            words(&self.mutable_state_bytes)?,
        ))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuExactCompletedSleepCaptureV1 {
    pub request: GpuConsolidationRequest,
    pub receipt: GpuSleepStagingReceipt,
    pub restored_completed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuExactPopulationCaptureV1 {
    schema_version: u16,
    capture_transaction_generation: u64,
    population_set_digest: [u64; 4],
    checkpoint_tick: Tick,
    rows: Vec<GpuExactPopulationCaptureRowV1>,
    bytes_copied: u64,
}

impl GpuExactPopulationCaptureV1 {
    pub const fn checkpoint_tick(&self) -> Tick {
        self.checkpoint_tick
    }

    pub const fn capture_transaction_generation(&self) -> u64 {
        self.capture_transaction_generation
    }

    pub const fn population_set_digest(&self) -> [u64; 4] {
        self.population_set_digest
    }

    pub fn rows(&self) -> &[GpuExactPopulationCaptureRowV1] {
        &self.rows
    }

    pub const fn bytes_copied(&self) -> u64 {
        self.bytes_copied
    }

    #[allow(dead_code)]
    pub(crate) fn into_rows(self) -> Vec<GpuExactPopulationCaptureRowV1> {
        self.rows
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuExactPopulationCaptureFailureStageV1 {
    DevicePoll,
    MapCallback,
    DecodeIdentity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuExactPopulationCaptureFailureV1 {
    pub capture_transaction_generation: u64,
    pub stage: GpuExactPopulationCaptureFailureStageV1,
}

pub enum GpuExactPopulationCapturePollV1 {
    Pending,
    Ready(GpuExactPopulationCaptureV1),
    Failed(GpuExactPopulationCaptureFailureV1),
}

#[derive(Debug, Clone)]
struct CapturedRowLayout {
    identity: GpuExactPopulationCaptureIdentityV1,
    brain_slot_model: GpuBrainSlot,
    ranges: GpuFixedSlotRanges,
    last_learning_replay_key: Option<OutcomeCreditReplayKey>,
    pending_eligibility: Option<PendingEligibilityReceipt>,
    pending_eligibility_record: Option<GpuPendingEligibilityRecord>,
    activity_snapshot: GpuActivityRuntimeSnapshot,
    completed_sleep: Option<GpuExactCompletedSleepCaptureV1>,
    expected_brain_slot_bytes: Vec<u8>,
    expected_phenotype_identity_bytes: Vec<u8>,
    brain_slot: Range<u64>,
    phenotype_identity: Range<u64>,
    immutable_plan: Range<u64>,
    immutable_weight: Range<u64>,
    mutable_state: Range<u64>,
    learning_state: Range<u64>,
}

pub struct GpuExactPopulationCaptureTicketV1 {
    backend_instance_id: std::num::NonZeroU64,
    capture_transaction_generation: u64,
    population_set_digest: [u64; 4],
    checkpoint_tick: Tick,
    staging: Option<wgpu::Buffer>,
    receiver: mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>,
    rows: Vec<CapturedRowLayout>,
    staging_bytes: u64,
    completed: bool,
    failure: Option<GpuExactPopulationCaptureFailureV1>,
}

impl GpuExactPopulationCaptureTicketV1 {
    pub const fn gpu_copy_submissions(&self) -> u64 {
        1
    }

    pub const fn map_operations(&self) -> u64 {
        1
    }

    pub const fn staging_bytes(&self) -> u64 {
        self.staging_bytes
    }

    pub const fn capture_transaction_generation(&self) -> u64 {
        self.capture_transaction_generation
    }

    pub const fn population_set_digest(&self) -> [u64; 4] {
        self.population_set_digest
    }

    pub fn retained_staging_bytes(&self) -> u64 {
        if self.staging.is_some() {
            self.staging_bytes
        } else {
            0
        }
    }

    #[cfg(feature = "gpu-tests")]
    pub fn force_decode_identity_failure_for_test(&mut self) {
        if let Some(row) = self.rows.first_mut() {
            row.identity.transaction_generation =
                row.identity.transaction_generation.saturating_add(1);
        }
    }
}

impl GpuClosedLoopBackend {
    pub fn exact_population_capture_metrics(&self) -> GpuExactPopulationCaptureMetricsV1 {
        self.exact_population_capture_metrics
    }

    pub fn submit_exact_population_capture(
        &mut self,
        checkpoint_tick: Tick,
        capture_transaction_generation: u64,
        handles: &[GpuBrainHandle],
    ) -> Result<GpuExactPopulationCaptureTicketV1, ScaffoldContractError> {
        self.ensure_ready()?;
        if capture_transaction_generation == 0
            || capture_transaction_generation != self.next_exact_population_capture_generation
            || handles.is_empty()
            || handles
                .windows(2)
                .any(|pair| pair[0].organism_id().raw() >= pair[1].organism_id().raw())
        {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        let next_capture_transaction_generation = capture_transaction_generation
            .checked_add(1)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;

        // Complete preflight precedes encoder creation. A stale or foreign row
        // therefore cannot submit a partial population capture.
        for handle in handles {
            self.validate_handle_backend(*handle)?;
            self.class_buckets
                .get(&handle.class_id().raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                .resident(*handle)?;
        }
        let resident_handles = self.organisms.values().copied().collect::<Vec<_>>();
        if resident_handles.len() != handles.len()
            || resident_handles
                .iter()
                .zip(handles)
                .any(|(resident, requested)| resident != requested)
        {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        let population_set_digest = population_set_digest(handles);

        let mut destination_cursor = 0_u64;
        let mut rows = Vec::with_capacity(handles.len());
        for handle in handles {
            let bucket = self
                .class_buckets
                .get(&handle.class_id().raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                .bucket_for_handle(*handle)?;
            let resident = bucket
                .slots
                .get(handle.slot() as usize)
                .and_then(Option::as_ref)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let active_activation_side = bucket
                .pipelines
                .slot_active_side(handle.slot(), handle.generation())
                .map_err(super::closed_loop_runtime::map_gpu_contract_error)?;
            let activity_snapshot = self.snapshot_activity_state(*handle)?;
            let mut matching_sleep_jobs =
                self.sleep_jobs.values().filter(|job| job.handle == *handle);
            let completed_sleep =
                matching_sleep_jobs
                    .next()
                    .map(|job| GpuExactCompletedSleepCaptureV1 {
                        request: job.request,
                        receipt: job.receipt,
                        restored_completed: job.restored_completed,
                    });
            if matching_sleep_jobs.next().is_some() {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
            }
            let identity = GpuExactPopulationCaptureIdentityV1 {
                schema_version: GPU_EXACT_POPULATION_CAPTURE_SCHEMA_VERSION,
                organism_id: handle.organism_id(),
                class_id: handle.class_id(),
                phenotype_hash: handle.phenotype_hash(),
                slot: handle.slot(),
                slot_generation: handle.generation(),
                graph_epoch: resident.transaction_generation,
                active_activation_side,
                logical_dispatch_generation: resident.logical_dispatch_generation,
                active_weight_generation: resident.active_weight_generation,
                active_weight_bank: resident.active_weight_bank,
                active_eligibility_generation: resident.active_eligibility_generation,
                inactive_eligibility_generation: resident.inactive_eligibility_generation,
                active_eligibility_bank: resident.active_eligibility_bank,
                replay_journal_generation: resident.replay_journal_generation,
                replay_journal_cursor: resident.replay_journal_cursor,
                replay_journal_event_count: resident.replay_journal_event_count,
                transaction_generation: resident.transaction_generation,
                activity_sequence_cursor: resident.activity_sequence_cursor,
                brain_atp_q16: resident.brain_atp_q16,
                last_world_atp_tick: resident.last_world_atp_tick,
                last_activity_dispatch_generation: resident.last_activity_dispatch_generation,
                last_activity_frame_digest: resident.last_activity_frame_digest,
                last_completed_gpu_time_ns: resident.last_completed_gpu_time_ns,
                last_throttle: resident.last_throttle.clone(),
                last_work: resident.last_work.clone(),
                v11: resident.v11.checkpoint(),
            };
            validate_capture_identity(&identity)?;
            let ranges = &resident.ranges;
            let brain_slot =
                reserve_destination(&ranges.brain_slot_bytes, &mut destination_cursor)?;
            let phenotype_identity =
                reserve_destination(&ranges.identity_bytes, &mut destination_cursor)?;
            let immutable_plan = reserve_destination(
                &words_to_bytes(ranges.immutable_plan_words.clone())?,
                &mut destination_cursor,
            )?;
            let immutable_weight = reserve_destination(
                &words_to_bytes(ranges.immutable_weight_words.clone())?,
                &mut destination_cursor,
            )?;
            let mutable_state = reserve_destination(
                &words_to_bytes(ranges.mutable_state_words.clone())?,
                &mut destination_cursor,
            )?;
            let learning_state = destination_subrange(
                &mutable_state,
                ranges.mutable_state_words.start,
                &ranges.layout.learning_state_words,
            )?;
            rows.push(CapturedRowLayout {
                identity,
                brain_slot_model: resident.brain_slot.clone(),
                ranges: resident.ranges.clone(),
                last_learning_replay_key: resident.learning_sequence_guard.last_committed(),
                pending_eligibility: resident.pending_eligibility,
                pending_eligibility_record: resident.pending_eligibility_record,
                activity_snapshot,
                completed_sleep,
                expected_brain_slot_bytes: bytemuck::bytes_of(resident.brain_slot.record())
                    .to_vec(),
                expected_phenotype_identity_bytes: bytemuck::bytes_of(
                    resident.brain_slot.identity(),
                )
                .to_vec(),
                brain_slot,
                phenotype_identity,
                immutable_plan,
                immutable_weight,
                mutable_state,
                learning_state,
            });
        }
        if destination_cursor == 0 || destination_cursor > self.device.limits().max_buffer_size {
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("alife-exact-population-checkpoint-staging-v1"),
            size: destination_cursor,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Copies were recorded with destination offsets but need the final
        // staging buffer. Re-record now that its bounded size is known.
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("alife-exact-population-checkpoint-copy-v1"),
            });
        for (handle, layout) in handles.iter().zip(&rows) {
            let bucket = self
                .class_buckets
                .get(&handle.class_id().raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                .bucket_for_handle(*handle)?;
            let resident = bucket
                .slots
                .get(handle.slot() as usize)
                .and_then(Option::as_ref)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            record_row_copies(
                &mut encoder,
                &bucket.buffers.neural_buffers(),
                &resident.ranges,
                &staging,
                layout,
            )?;
        }
        let command = encoder.finish();
        let (sender, receiver) = mpsc::channel();
        command.map_buffer_on_submit(
            &staging,
            wgpu::MapMode::Read,
            0..destination_cursor,
            move |result| {
                let _ = sender.send(result);
            },
        );
        self.queue.submit(Some(command));
        self.next_exact_population_capture_generation = next_capture_transaction_generation;
        self.exact_population_capture_metrics.gpu_copy_submissions = self
            .exact_population_capture_metrics
            .gpu_copy_submissions
            .saturating_add(1);
        self.exact_population_capture_metrics.map_operations = self
            .exact_population_capture_metrics
            .map_operations
            .saturating_add(1);
        self.exact_population_capture_metrics.bytes_copied = self
            .exact_population_capture_metrics
            .bytes_copied
            .saturating_add(destination_cursor);
        Ok(GpuExactPopulationCaptureTicketV1 {
            backend_instance_id: self.backend_instance_id,
            capture_transaction_generation,
            population_set_digest,
            checkpoint_tick,
            staging: Some(staging),
            receiver,
            rows,
            staging_bytes: destination_cursor,
            completed: false,
            failure: None,
        })
    }

    pub fn poll_exact_population_capture(
        &mut self,
        ticket: &mut GpuExactPopulationCaptureTicketV1,
    ) -> Result<GpuExactPopulationCapturePollV1, ScaffoldContractError> {
        self.ensure_ready()?;
        if ticket.backend_instance_id != self.backend_instance_id {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        if let Some(failure) = ticket.failure {
            return Ok(GpuExactPopulationCapturePollV1::Failed(failure));
        }
        if ticket.completed {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        self.exact_population_capture_metrics.poll_calls = self
            .exact_population_capture_metrics
            .poll_calls
            .saturating_add(1);
        if self.device.poll(wgpu::PollType::Poll).is_err() {
            return Ok(self.fail_exact_population_capture(
                ticket,
                GpuExactPopulationCaptureFailureStageV1::DevicePoll,
            ));
        }
        match ticket.receiver.try_recv() {
            Err(TryRecvError::Empty) => return Ok(GpuExactPopulationCapturePollV1::Pending),
            Err(TryRecvError::Disconnected) | Ok(Err(_)) => {
                return Ok(self.fail_exact_population_capture(
                    ticket,
                    GpuExactPopulationCaptureFailureStageV1::MapCallback,
                ));
            }
            Ok(Ok(())) => {}
        }
        let staging = ticket
            .staging
            .take()
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let mapped = staging.slice(..ticket.staging_bytes).get_mapped_range();
        let decoded = ticket
            .rows
            .iter()
            .map(|layout| decode_row(&mapped, layout))
            .collect::<Result<Vec<_>, _>>();
        drop(mapped);
        staging.unmap();
        self.record_exact_population_staging_release(ticket.staging_bytes);
        let rows = match decoded {
            Ok(rows) => rows,
            Err(_) => {
                let failure = GpuExactPopulationCaptureFailureV1 {
                    capture_transaction_generation: ticket.capture_transaction_generation,
                    stage: GpuExactPopulationCaptureFailureStageV1::DecodeIdentity,
                };
                ticket.completed = true;
                ticket.failure = Some(failure);
                return Ok(GpuExactPopulationCapturePollV1::Failed(failure));
            }
        };
        ticket.completed = true;
        self.exact_population_capture_metrics.completed_captures = self
            .exact_population_capture_metrics
            .completed_captures
            .saturating_add(1);
        Ok(GpuExactPopulationCapturePollV1::Ready(
            GpuExactPopulationCaptureV1 {
                schema_version: GPU_EXACT_POPULATION_CAPTURE_SCHEMA_VERSION,
                capture_transaction_generation: ticket.capture_transaction_generation,
                population_set_digest: ticket.population_set_digest,
                checkpoint_tick: ticket.checkpoint_tick,
                rows,
                bytes_copied: ticket.staging_bytes,
            },
        ))
    }

    fn fail_exact_population_capture(
        &mut self,
        ticket: &mut GpuExactPopulationCaptureTicketV1,
        stage: GpuExactPopulationCaptureFailureStageV1,
    ) -> GpuExactPopulationCapturePollV1 {
        if let Some(staging) = ticket.staging.take() {
            staging.unmap();
            self.record_exact_population_staging_release(ticket.staging_bytes);
        }
        let failure = GpuExactPopulationCaptureFailureV1 {
            capture_transaction_generation: ticket.capture_transaction_generation,
            stage,
        };
        ticket.completed = true;
        ticket.failure = Some(failure);
        GpuExactPopulationCapturePollV1::Failed(failure)
    }

    fn record_exact_population_staging_release(&mut self, bytes: u64) {
        self.exact_population_capture_metrics.released_staging_bytes = self
            .exact_population_capture_metrics
            .released_staging_bytes
            .saturating_add(bytes);
    }
}

fn validate_capture_identity(
    identity: &GpuExactPopulationCaptureIdentityV1,
) -> Result<(), ScaffoldContractError> {
    identity.organism_id.validate()?;
    if identity.schema_version != GPU_EXACT_POPULATION_CAPTURE_SCHEMA_VERSION
        || identity.slot_generation == 0
        || identity.graph_epoch == 0
        || identity.active_activation_side > 1
        || identity.logical_dispatch_generation == 0
        || identity.active_weight_generation == 0
        || identity.active_weight_bank > 1
        || identity.active_eligibility_generation == 0
        || identity.active_eligibility_bank > 1
        || identity.replay_journal_generation == 0
        || identity.transaction_generation == 0
        || identity.graph_epoch != identity.transaction_generation
    {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    Ok(())
}

fn words_to_bytes(words: Range<u32>) -> Result<Range<u64>, ScaffoldContractError> {
    Ok(u64::from(words.start)
        .checked_mul(4)
        .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?
        ..u64::from(words.end)
            .checked_mul(4)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?)
}

fn destination_subrange(
    destination: &Range<u64>,
    source_base_words: u32,
    source_subrange_words: &Range<u32>,
) -> Result<Range<u64>, ScaffoldContractError> {
    let local_start = source_subrange_words
        .start
        .checked_sub(source_base_words)
        .and_then(|value| u64::from(value).checked_mul(4))
        .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
    let local_end = source_subrange_words
        .end
        .checked_sub(source_base_words)
        .and_then(|value| u64::from(value).checked_mul(4))
        .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
    let start = destination
        .start
        .checked_add(local_start)
        .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
    let end = destination
        .start
        .checked_add(local_end)
        .filter(|end| *end <= destination.end)
        .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
    Ok(start..end)
}

fn population_set_digest(handles: &[GpuBrainHandle]) -> [u64; 4] {
    let mut digest = CanonicalDigestBuilder::new(GPU_EXACT_POPULATION_SET_DIGEST_DOMAIN);
    digest.write_sequence_len(handles.len());
    for handle in handles {
        digest.write_u64(handle.organism_id().raw());
        digest.write_u16(handle.class_id().raw());
        digest.write_u32(handle.slot());
        digest.write_u32(handle.generation());
        for word in handle.phenotype_hash().0 {
            digest.write_u64(word);
        }
    }
    digest.finish256()
}

fn reserve_destination(
    source: &Range<u64>,
    destination_cursor: &mut u64,
) -> Result<Range<u64>, ScaffoldContractError> {
    let size = source
        .end
        .checked_sub(source.start)
        .filter(|size| *size > 0 && *size % 4 == 0)
        .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
    let start = *destination_cursor;
    let end = start
        .checked_add(size)
        .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
    *destination_cursor = end;
    Ok(start..end)
}

fn record_row_copies(
    encoder: &mut wgpu::CommandEncoder,
    buffers: &[&wgpu::Buffer; 7],
    ranges: &GpuFixedSlotRanges,
    staging: &wgpu::Buffer,
    layout: &CapturedRowLayout,
) -> Result<(), ScaffoldContractError> {
    let sources = [
        (buffers[0], ranges.brain_slot_bytes.clone()),
        (buffers[1], ranges.identity_bytes.clone()),
        (
            buffers[2],
            words_to_bytes(ranges.immutable_plan_words.clone())?,
        ),
        (
            buffers[3],
            words_to_bytes(ranges.immutable_weight_words.clone())?,
        ),
        (
            buffers[6],
            words_to_bytes(ranges.mutable_state_words.clone())?,
        ),
    ];
    let destinations = [
        &layout.brain_slot,
        &layout.phenotype_identity,
        &layout.immutable_plan,
        &layout.immutable_weight,
        &layout.mutable_state,
    ];
    for ((source, source_range), destination) in sources.into_iter().zip(destinations) {
        let source_size = source_range
            .end
            .checked_sub(source_range.start)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let destination_size = destination
            .end
            .checked_sub(destination.start)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        if source_size != destination_size {
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        encoder.copy_buffer_to_buffer(
            source,
            source_range.start,
            staging,
            destination.start,
            source_size,
        );
    }
    Ok(())
}

fn decode_row(
    mapped: &[u8],
    layout: &CapturedRowLayout,
) -> Result<GpuExactPopulationCaptureRowV1, ScaffoldContractError> {
    let take = |range: &Range<u64>| -> Result<Vec<u8>, ScaffoldContractError> {
        let start = usize::try_from(range.start)
            .map_err(|_| ScaffoldContractError::NeuralBackendUnavailable)?;
        let end = usize::try_from(range.end)
            .map_err(|_| ScaffoldContractError::NeuralBackendUnavailable)?;
        mapped
            .get(start..end)
            .map(<[u8]>::to_vec)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)
    };
    let brain_slot_bytes = take(&layout.brain_slot)?;
    let phenotype_identity_bytes = take(&layout.phenotype_identity)?;
    if brain_slot_bytes != layout.expected_brain_slot_bytes
        || phenotype_identity_bytes != layout.expected_phenotype_identity_bytes
    {
        return Err(ScaffoldContractError::BrainOwnershipMismatch);
    }
    let learning_state_bytes = take(&layout.learning_state)?;
    if learning_state_bytes.len() != std::mem::size_of::<GpuSlotLearningStateRecord>() {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    let learning_state =
        bytemuck::pod_read_unaligned::<GpuSlotLearningStateRecord>(&learning_state_bytes);
    let identity = &layout.identity;
    if learning_state.active_weight_bank != u32::from(identity.active_weight_bank)
        || join_u32_pair(
            learning_state.active_weight_generation_lo,
            learning_state.active_weight_generation_hi,
        ) != identity.active_weight_generation
        || learning_state.active_eligibility_bank != u32::from(identity.active_eligibility_bank)
        || join_u32_pair(
            learning_state.active_eligibility_generation_lo,
            learning_state.active_eligibility_generation_hi,
        ) != identity.active_eligibility_generation
        || join_u32_pair(
            learning_state.inactive_eligibility_generation_lo,
            learning_state.inactive_eligibility_generation_hi,
        ) != identity.inactive_eligibility_generation
        || join_u32_pair(
            learning_state.replay_generation_lo,
            learning_state.replay_generation_hi,
        ) != identity.replay_journal_generation
        || learning_state.replay_cursor != identity.replay_journal_cursor
        || learning_state.replay_event_count != identity.replay_journal_event_count
        || join_u32_pair(
            learning_state.transaction_generation_lo,
            learning_state.transaction_generation_hi,
        ) != identity.transaction_generation
    {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    Ok(GpuExactPopulationCaptureRowV1 {
        identity: layout.identity.clone(),
        brain_slot: layout.brain_slot_model.clone(),
        ranges: layout.ranges.clone(),
        last_learning_replay_key: layout.last_learning_replay_key,
        pending_eligibility: layout.pending_eligibility,
        pending_eligibility_record: layout.pending_eligibility_record,
        activity_snapshot: layout.activity_snapshot.clone(),
        completed_sleep: layout.completed_sleep.clone(),
        brain_slot_bytes,
        phenotype_identity_bytes,
        immutable_plan_bytes: take(&layout.immutable_plan)?,
        immutable_weight_bytes: take(&layout.immutable_weight)?,
        mutable_state_bytes: take(&layout.mutable_state)?,
    })
}

fn join_u32_pair(lo: u32, hi: u32) -> u64 {
    u64::from(lo) | (u64::from(hi) << 32)
}
