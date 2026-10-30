//! Explicit training-boundary captures. No neural state advances on the host.
use super::*;
use crate::training_rollout::GpuTrainingStateSnapshot as Snapshot;
use alife_core::Tick;
use std::ops::Range;

// Bound transient staging, independently of resident class arenas. Large cohorts
// use the measured device limit and this scratch ceiling, never serial sessions.
const TRAINING_CAPTURE_STAGING_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct GpuTrainingCaptureMetrics {
    pub batch_captures: u64,
    pub captured_rows: u64,
    /// Standalone capture submissions plus joint inference/capture submissions.
    pub readback_submissions: u64,
    pub bytes_copied: u64,
    /// Completed joint inference/capture packets; not standalone readback timing.
    #[serde(default)]
    pub fused_tick_packets: u64,
    #[serde(default)]
    pub structural_fallback_packets: u64,
    #[serde(default)]
    pub staging_fallback_packets: u64,
}

#[derive(Clone)]
struct CopyRow {
    handle: GpuBrainHandle,
    buffer: usize,
    words: Range<u32>,
}

fn packets(sizes: &[u64], limit: u64) -> Result<Vec<Range<usize>>, ScaffoldContractError> {
    let mut result = Vec::new();
    let (mut start, mut used) = (0, 0u64);
    for (index, bytes) in sizes.iter().copied().enumerate() {
        if bytes == 0 || bytes % 4 != 0 || bytes > limit {
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        if used.checked_add(bytes).is_none_or(|n| n > limit) {
            result.push(start..index);
            start = index;
            used = 0;
        }
        used += bytes;
    }
    if start < sizes.len() {
        result.push(start..sizes.len());
    }
    Ok(result)
}

/// One training tick and its exact queue-ordered boundaries. Fallback captures
/// retain the standalone admission and bounded staging policy.
pub struct GpuTrainingTickCapture {
    pub ticks: Vec<GpuClosedLoopTick>,
    pub before: Vec<Snapshot>,
    pub after: Vec<Snapshot>,
}

/// Single-use preparation at the original pre-capture boundary. No fused GPU
/// arena is allocated until execution consumes this token.
pub struct GpuTrainingCaptureBoundary {
    before: Vec<Snapshot>,
    ranges: Vec<Range<u32>>,
    offsets: Option<Vec<Range<u64>>>,
    generations: Vec<[u64; 11]>,
    fallback: Option<CaptureFallback>,
}

enum CaptureFallback {
    Structural,
    Staging,
}

fn capture_generations(resident: &ResidentBrainSlot) -> [u64; 11] {
    [
        resident.transaction_generation,
        resident.active_eligibility_generation,
        resident.inactive_eligibility_generation,
        u64::from(resident.active_eligibility_bank),
        resident.replay_journal_generation,
        u64::from(resident.replay_journal_cursor),
        u64::from(resident.replay_journal_event_count),
        resident.activity_sequence_cursor,
        u64::from(resident.brain_atp_q16),
        u64::from(resident.last_world_atp_tick.is_some()),
        resident.last_world_atp_tick.unwrap_or(0),
    ]
}

fn capture_metadata_matches(expected: &Snapshot, current: &Snapshot) -> bool {
    expected.handle == current.handle
        && expected.tick == current.tick
        && expected.logical_dispatch_generation == current.logical_dispatch_generation
        && expected.active_activation_side == current.active_activation_side
        && expected.active_weight_generation == current.active_weight_generation
        && expected.active_weight_bank == current.active_weight_bank
        && Arc::ptr_eq(&expected.phenotype, &current.phenotype)
        && expected.brain_slot == current.brain_slot
        && expected.v11 == current.v11
        && expected.mutable_word_base == current.mutable_word_base
        && expected.structural_synapses == current.structural_synapses
}

fn capture_boundary_matches(
    expected: &Snapshot,
    current: &Snapshot,
    expected_generations: &[u64; 11],
    current_generations: &[u64; 11],
) -> bool {
    capture_metadata_matches(expected, current) && expected_generations == current_generations
}

// The arena is unmapped on every exit, including failures elsewhere in tick
// decoding. No snapshot or host authority is published from mapped bytes.
pub(super) struct TrainingTickCapture {
    staging: wgpu::Buffer,
    before: Vec<Snapshot>,
    ranges: Vec<Range<u32>>,
    offsets: Vec<Range<u64>>,
    receiver: Option<std::sync::mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>>,
    decoded: Option<Vec<Vec<u32>>>,
}

impl Drop for TrainingTickCapture {
    fn drop(&mut self) {
        self.staging.unmap();
    }
}

// Both boundaries must fit together. Exceeding the unchanged ceiling selects
// the packetized standalone path, rather than increasing transient staging.
fn fused_offsets(
    sizes: &[u64],
    limit: u64,
) -> Result<Option<Vec<Range<u64>>>, ScaffoldContractError> {
    let mut offsets = Vec::with_capacity(sizes.len().saturating_mul(2));
    let mut used = 0_u64;
    for &bytes in sizes.iter().chain(sizes) {
        if bytes == 0 || !bytes.is_multiple_of(4) {
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        let Some(end) = used.checked_add(bytes).filter(|end| *end <= limit) else {
            return Ok(None);
        };
        offsets.push(used..end);
        used = end;
    }
    Ok(Some(offsets))
}

fn decode_capture_words(
    mapped: &[u8],
    offsets: &[Range<u64>],
) -> Result<Vec<Vec<u32>>, ScaffoldContractError> {
    let mut end = 0_u64;
    let mut result = Vec::with_capacity(offsets.len());
    for range in offsets {
        if range.start != end
            || range.start >= range.end
            || !range.end.is_multiple_of(4)
            || range.end > mapped.len() as u64
        {
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        // Copy bytes rather than assuming a host fixture's slice alignment.
        result.push(
            mapped[range.start as usize..range.end as usize]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|word| u32::from_ne_bytes(*word))
                .collect(),
        );
        end = range.end;
    }
    if end != mapped.len() as u64 {
        return Err(ScaffoldContractError::NeuralBackendUnavailable);
    }
    Ok(result)
}

impl TrainingTickCapture {
    pub(super) fn record(
        &self,
        backend: &GpuClosedLoopBackend,
        encoder: &mut wgpu::CommandEncoder,
        post: bool,
    ) {
        let start = if post { self.before.len() } else { 0 };
        for ((snapshot, words), destination) in self
            .before
            .iter()
            .zip(&self.ranges)
            .zip(&self.offsets[start..])
        {
            let bucket = backend
                .class_buckets
                .get(&snapshot.handle.class_id.raw())
                .expect("capture capability preflighted")
                .bucket_for_handle(snapshot.handle)
                .expect("capture slot retained until commit");
            encoder.copy_buffer_to_buffer(
                bucket.buffers.neural_buffers()[6],
                u64::from(words.start) * 4,
                &self.staging,
                destination.start,
                destination.end - destination.start,
            );
        }
    }

    pub(super) fn register(&mut self, command: &wgpu::CommandBuffer) {
        let (sender, receiver) = std::sync::mpsc::channel();
        command.map_buffer_on_submit(&self.staging, wgpu::MapMode::Read, .., move |result| {
            let _ = sender.send(result);
        });
        self.receiver = Some(receiver);
    }

    pub(super) fn mapping_succeeded(&mut self) -> bool {
        self.receiver
            .take()
            .is_some_and(|receiver| timestamp_mapping_completed(&receiver))
    }

    pub(super) fn decode(
        &mut self,
        ticks: &[GpuClosedLoopTick],
    ) -> Result<(), ScaffoldContractError> {
        if ticks.len() != self.before.len()
            || ticks.iter().zip(&self.before).any(|(tick, before)| {
                tick.handle != before.handle
                    || tick.throttle.tick != before.tick
                    || tick.active_activation_side > 1
                    || tick.training_rollout.as_ref().is_none_or(|rollout| {
                        rollout.dispatch_generation != tick.dispatch_generation
                            || rollout.active_weight_generation != before.active_weight_generation
                    })
            })
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let mapped = self.staging.slice(..).get_mapped_range();
        let words = decode_capture_words(&mapped, &self.offsets)?;
        drop(mapped);
        self.staging.unmap();
        self.decoded = Some(words);
        Ok(())
    }

    fn finish(
        mut self,
        backend: &GpuClosedLoopBackend,
        ticks: Vec<GpuClosedLoopTick>,
    ) -> GpuTrainingTickCapture {
        let mut words = self
            .decoded
            .take()
            .expect("both boundaries decoded before commit")
            .into_iter();
        let mut before = std::mem::take(&mut self.before);
        let mut after = before.clone();
        for snapshot in &mut before {
            snapshot.mutable_words = words.next().expect("pre boundary decoded");
        }
        for (snapshot, tick) in after.iter_mut().zip(&ticks) {
            let resident = backend
                .class_buckets
                .get(&snapshot.handle.class_id.raw())
                .expect("committed class exists")
                .resident(snapshot.handle)
                .expect("committed resident exists");
            snapshot.mutable_words = words.next().expect("post boundary decoded");
            // These fields come from the actual host commit, not a predicted
            // dispatch. The side is the validated, contents-compared tick side.
            snapshot.logical_dispatch_generation = resident.logical_dispatch_generation;
            snapshot.active_activation_side = tick.active_activation_side;
            snapshot.active_weight_generation = resident.active_weight_generation;
            snapshot.active_weight_bank = resident.active_weight_bank;
            snapshot.brain_slot = resident.brain_slot.clone();
            snapshot.v11 = resident.v11.checkpoint();
        }
        GpuTrainingTickCapture {
            ticks,
            before,
            after,
        }
    }
}

impl GpuClosedLoopBackend {
    /// Prepare before demonstrator callbacks and sampling-counter advancement.
    /// Uncached structure is synchronously admitted here, as in the legacy path.
    pub fn prepare_training_tick_capture(
        &mut self,
        requests: &[(GpuBrainHandle, Tick)],
    ) -> Result<GpuTrainingCaptureBoundary, ScaffoldContractError> {
        self.prepare_training_tick_capture_with_limit(
            requests,
            self.device
                .limits()
                .max_buffer_size
                .min(TRAINING_CAPTURE_STAGING_BYTES),
        )
    }

    fn prepare_training_tick_capture_with_limit(
        &mut self,
        requests: &[(GpuBrainHandle, Tick)],
        limit: u64,
    ) -> Result<GpuTrainingCaptureBoundary, ScaffoldContractError> {
        self.validate_training_capture_requests(requests)?;
        if requests.is_empty() {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        let mut before = Vec::with_capacity(requests.len());
        let mut ranges = Vec::with_capacity(requests.len());
        let mut structural = false;
        for &(handle, tick) in requests {
            let (mut snapshot, slot) = self.training_snapshot_metadata(handle, tick)?;
            structural |= self.training_structural_needed(&mut snapshot);
            ranges.push(slot.mutable_state_words);
            before.push(snapshot);
        }
        let sizes = ranges
            .iter()
            .map(|range| {
                range
                    .end
                    .checked_sub(range.start)
                    .filter(|count| *count > 0)
                    .map(|count| u64::from(count) * 4)
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut offsets = fused_offsets(&sizes, limit)?;
        let fallback = if structural || offsets.is_none() {
            let reason = if structural {
                CaptureFallback::Structural
            } else {
                CaptureFallback::Staging
            };
            before = self.capture_training_states(requests)?;
            offsets = None;
            Some(reason)
        } else {
            None
        };
        // Validate source ranges and usages before creating the staging arena.
        for (snapshot, range) in before.iter().zip(&ranges) {
            let bucket = self
                .class_buckets
                .get(&snapshot.handle.class_id.raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                .bucket_for_handle(snapshot.handle)?;
            let source = bucket.buffers.neural_buffers()[6];
            if u64::from(range.end) * 4 > source.size()
                || !source.usage().contains(wgpu::BufferUsages::COPY_SRC)
            {
                return Err(ScaffoldContractError::NeuralBackendUnavailable);
            }
        }
        let generations = requests
            .iter()
            .map(|(handle, _)| {
                self.class_buckets
                    .get(&handle.class_id.raw())
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                    .resident(*handle)
                    .map(capture_generations)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(GpuTrainingCaptureBoundary {
            before,
            ranges,
            offsets,
            generations,
            fallback,
        })
    }

    /// Consume the exact prepared boundary. A foreign/stale token cannot bind
    /// snapshots to a later private generation or a different ordered batch.
    pub fn tick_memory_batch_training_captured(
        &mut self,
        batch: &GpuClosedLoopMemoryBatchInput<'_>,
        sampling: &[crate::GpuTrainingSamplingConfig],
        boundary: GpuTrainingCaptureBoundary,
    ) -> Result<GpuTrainingTickCapture, ScaffoldContractError> {
        let requests = batch
            .members
            .iter()
            .map(|member| (member.handle, member.frame.tick()))
            .collect::<Vec<_>>();
        self.validate_training_capture_requests(&requests)?;
        if requests.len() != boundary.before.len() {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        for (index, &(handle, tick)) in requests.iter().enumerate() {
            let (mut current, slot) = self.training_snapshot_metadata(handle, tick)?;
            if self.training_structural_needed(&mut current)
                || slot.mutable_state_words != boundary.ranges[index]
                || !capture_boundary_matches(
                    &boundary.before[index],
                    &current,
                    &boundary.generations[index],
                    &capture_generations(
                        self.class_buckets
                            .get(&handle.class_id.raw())
                            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                            .resident(handle)?,
                    ),
                )
            {
                return Err(ScaffoldContractError::BrainOwnershipMismatch);
            }
        }
        let GpuTrainingCaptureBoundary {
            before,
            ranges,
            offsets,
            generations: _,
            fallback,
        } = boundary;
        let Some(offsets) = offsets else {
            let ticks = self.tick_memory_batch_training(batch, sampling)?;
            let after = match self.capture_training_states(&requests) {
                Ok(after) => after,
                // Legacy fallback has already committed the tick. Stop, never
                // claim that these GPU writes or host commits were undone.
                Err(error) => {
                    self.mark_device_lost();
                    return Err(error);
                }
            };
            match fallback.expect("legacy preparation retains its reason") {
                CaptureFallback::Structural => {
                    self.training_capture_metrics.structural_fallback_packets += 1
                }
                CaptureFallback::Staging => {
                    self.training_capture_metrics.staging_fallback_packets += 1
                }
            }
            return Ok(GpuTrainingTickCapture {
                ticks,
                before,
                after,
            });
        };
        let size = offsets
            .last()
            .ok_or(ScaffoldContractError::InvalidPerceptionFrame)?
            .end;
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("training-tick-boundaries"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut capture = TrainingTickCapture {
            staging,
            before,
            ranges,
            offsets,
            receiver: None,
            decoded: None,
        };
        let inputs = batch
            .members
            .iter()
            .map(|member| GpuRuntimeTickInput {
                handle: member.handle,
                frame: member.frame,
                memory_upload: Some(member.memory_upload),
            })
            .collect::<Vec<_>>();
        let ticks = self.tick_inputs_with_selector_diagnostic_capture(
            &inputs,
            None,
            None,
            Some(sampling),
            Some(&mut capture),
        )?;
        self.training_capture_metrics.fused_tick_packets += 1;
        self.training_capture_metrics.batch_captures += 2;
        self.training_capture_metrics.captured_rows += (requests.len() * 2) as u64;
        self.training_capture_metrics.bytes_copied += size;
        // This is a joint inference/capture submission, not a standalone
        // mutable-slot readback timing sample.
        self.training_capture_metrics.readback_submissions += 1;
        Ok(capture.finish(self, ticks))
    }

    #[cfg(feature = "gpu-tests")]
    pub fn prepare_training_tick_capture_with_limit_for_test(
        &mut self,
        requests: &[(GpuBrainHandle, Tick)],
        limit: u64,
    ) -> Result<GpuTrainingCaptureBoundary, ScaffoldContractError> {
        self.prepare_training_tick_capture_with_limit(
            requests,
            limit
                .min(TRAINING_CAPTURE_STAGING_BYTES)
                .min(self.device.limits().max_buffer_size),
        )
    }

    #[cfg(feature = "gpu-tests")]
    pub fn force_training_capture_failure_for_test(&mut self, mapping: bool) {
        self.forced_training_capture_failure = Some(mapping);
    }

    fn validate_training_capture_requests(
        &mut self,
        requests: &[(GpuBrainHandle, Tick)],
    ) -> Result<(), ScaffoldContractError> {
        self.ensure_ready()?;
        let mut identities = BTreeSet::new();
        for (handle, tick) in requests {
            if !identities.insert(handle.organism_id().raw())
                || requests.first().is_some_and(|r| r.1 != *tick)
            {
                return Err(ScaffoldContractError::BrainOwnershipMismatch);
            }
            self.validate_handle_backend(*handle)?;
            self.class_buckets
                .get(&handle.class_id.raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                .resident(*handle)?;
        }
        Ok(())
    }

    pub fn training_capture_metrics(&self) -> GpuTrainingCaptureMetrics {
        self.training_capture_metrics
    }

    /// Request-order snapshots at one immutable queue boundary. All capabilities
    /// are preflighted before any copy; one mapped staging arena spans all rows
    /// that fit the bounded packet, including rows in different fixed chunks.
    /// Tick is the caller's canonical frame label, as in the single-row API;
    /// this readback does not infer a separate world clock. The staged consumer
    /// binds pre/post snapshots to its validated frame and dispatch receipt.
    pub fn capture_training_states(
        &mut self,
        requests: &[(GpuBrainHandle, Tick)],
    ) -> Result<Vec<Snapshot>, ScaffoldContractError> {
        self.validate_training_capture_requests(requests)?;
        let mut rows = Vec::with_capacity(requests.len());
        let mut copies = Vec::new();
        for &(handle, tick) in requests {
            let (mut snapshot, ranges) = self.training_snapshot_metadata(handle, tick)?;
            let mutable = copies.len();
            copies.push(CopyRow {
                handle,
                buffer: 6,
                words: ranges.mutable_state_words.clone(),
            });
            let structural = if self.training_structural_needed(&mut snapshot) {
                let start = copies.len();
                copies.push(CopyRow {
                    handle,
                    buffer: 2,
                    words: ranges.layout.target_offset_words.start
                        ..ranges.layout.route_index_words.start
                            + snapshot.brain_slot.record().recurrent_synapse_count,
                });
                copies.push(CopyRow {
                    handle,
                    buffer: 3,
                    words: ranges.layout.genetic_weight_words.start
                        ..ranges.layout.alpha_words.start
                            + snapshot.brain_slot.record().synapse_count,
                });
                Some(start)
            } else {
                None
            };
            rows.push((snapshot, ranges, mutable, structural));
        }
        let sizes = copies
            .iter()
            .map(|c| u64::from(c.words.end - c.words.start) * 4)
            .collect::<Vec<_>>();
        let packets = packets(
            &sizes,
            self.device
                .limits()
                .max_buffer_size
                .min(TRAINING_CAPTURE_STAGING_BYTES),
        )?;
        let mut words = vec![None; copies.len()];
        let before = self.mutable_slot_readback_counters.snapshot();
        for packet in packets {
            let sources = copies[packet.clone()]
                .iter()
                .map(|copy| {
                    let bucket = self
                        .class_buckets
                        .get(&copy.handle.class_id.raw())
                        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                        .bucket_for_handle(copy.handle)?;
                    Ok((
                        bucket.buffers.neural_buffers()[copy.buffer],
                        copy.words.clone(),
                    ))
                })
                .collect::<Result<Vec<_>, ScaffoldContractError>>()?;
            let captured = crate::closed_loop_sleep::read_gpu_word_ranges(
                &self.device,
                &self.queue,
                &sources,
                "training-population-readback",
                Some(&self.mutable_slot_readback_counters),
            )?;
            for (index, row) in packet.zip(captured) {
                words[index] = Some(row);
            }
        }
        let mut result = Vec::with_capacity(rows.len());
        for (mut snapshot, ranges, mutable, structural) in rows {
            snapshot.mutable_words = words[mutable]
                .take()
                .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
            if let Some(index) = structural {
                self.attach_training_structural(
                    &mut snapshot,
                    &ranges,
                    words[index]
                        .as_ref()
                        .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?,
                    words[index + 1]
                        .as_ref()
                        .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?,
                )?;
            }
            result.push(snapshot);
        }
        let after = self.mutable_slot_readback_counters.snapshot();
        self.training_capture_metrics.batch_captures += u64::from(!requests.is_empty());
        self.training_capture_metrics.captured_rows += result.len() as u64;
        self.training_capture_metrics.readback_submissions += after.calls - before.calls;
        self.training_capture_metrics.bytes_copied += after.bytes - before.bytes;
        Ok(result)
    }
    pub fn capture_training_state(
        &mut self,
        handle: GpuBrainHandle,
        tick: alife_core::Tick,
    ) -> Result<Snapshot, ScaffoldContractError> {
        let (mut snapshot, ranges) = self.training_snapshot_metadata(handle, tick)?;
        snapshot.mutable_words = self.read_slot_mutable_words(handle, &ranges)?;
        if self.training_structural_needed(&mut snapshot) {
            let bucket = self
                .class_buckets
                .get(&handle.class_id.raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                .bucket_for_handle(handle)?;
            let plan = crate::closed_loop_sleep::read_gpu_words(
                &self.device,
                &self.queue,
                bucket.buffers.neural_buffers()[2],
                ranges.layout.target_offset_words.start
                    ..ranges.layout.route_index_words.start
                        + snapshot.brain_slot.record().recurrent_synapse_count,
                "training-structural-plan-readback",
                None,
            )?;
            let weights = crate::closed_loop_sleep::read_gpu_words(
                &self.device,
                &self.queue,
                bucket.buffers.neural_buffers()[3],
                ranges.layout.genetic_weight_words.start
                    ..ranges.layout.alpha_words.start + snapshot.brain_slot.record().synapse_count,
                "training-structural-weight-readback",
                None,
            )?;
            self.attach_training_structural(&mut snapshot, &ranges, &plan, &weights)?;
        }
        Ok(snapshot)
    }
    fn training_snapshot_metadata(
        &mut self,
        handle: GpuBrainHandle,
        tick: alife_core::Tick,
    ) -> Result<(Snapshot, GpuFixedSlotRanges), ScaffoldContractError> {
        self.ensure_ready()?;
        self.validate_handle_backend(handle)?;
        let (snapshot, ranges) = {
            let bucket = self
                .class_buckets
                .get_mut(&handle.class_id.raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                .bucket_for_handle_mut(handle)?;
            let side = bucket
                .pipelines
                .slot_active_side(handle.slot, handle.generation)
                .map_err(map_gpu_contract_error)?;
            let resident = bucket.slots[handle.slot as usize]
                .as_mut()
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            (
                crate::training_rollout::GpuTrainingStateSnapshot {
                    handle,
                    tick: tick.raw(),
                    logical_dispatch_generation: resident.logical_dispatch_generation,
                    active_activation_side: side,
                    active_weight_generation: resident.active_weight_generation,
                    active_weight_bank: resident.active_weight_bank,
                    phenotype: shared_training_phenotype(
                        &mut resident.training_phenotype,
                        handle,
                        &resident.phenotype,
                    ),
                    brain_slot: resident.brain_slot.clone(),
                    v11: resident.v11.checkpoint(),
                    mutable_word_base: resident.ranges.mutable_state_words.start,
                    mutable_words: Vec::new(),
                    structural_synapses: Vec::new(),
                },
                resident.ranges.clone(),
            )
        };
        Ok((snapshot, ranges))
    }
    fn training_structural_needed(&self, snapshot: &mut Snapshot) -> bool {
        if snapshot.brain_slot.record().synapse_count as usize
            <= snapshot.phenotype.synapses().len()
        {
            return false;
        }
        let handle = snapshot.handle;
        let key = (
            handle.class_id.raw(),
            handle.slot,
            handle.generation,
            handle.organism_id.raw(),
        );
        if let Some(cached) = self.training_structural_cache.get(&key) {
            if cached.slot == snapshot.brain_slot && cached.structural == snapshot.v11.structural {
                snapshot.structural_synapses = cached.synapses.clone();
                return false;
            }
        }
        true
    }
    fn attach_training_structural(
        &mut self,
        snapshot: &mut Snapshot,
        ranges: &GpuFixedSlotRanges,
        plan: &[u32],
        weights: &[u32],
    ) -> Result<(), ScaffoldContractError> {
        let handle = snapshot.handle;
        let inherited = snapshot.phenotype.synapses().len();
        let live = snapshot.brain_slot.record().synapse_count as usize;
        let key = (
            handle.class_id.raw(),
            handle.slot,
            handle.generation,
            handle.organism_id.raw(),
        );
        let plan_base = ranges.layout.target_offset_words.start;
        let weight_base = ranges.layout.genetic_weight_words.start;
        let recurrent = snapshot.brain_slot.record().recurrent_synapse_count;
        for target in 0..snapshot.brain_slot.record().neuron_count {
            let offset = |word: u32| -> Result<usize, ScaffoldContractError> {
                usize::try_from(word - plan_base)
                    .map_err(|_| ScaffoldContractError::NeuralBackendUnavailable)
            };
            let begin = *plan
                .get(offset(ranges.layout.target_offset_words.start + target)?)
                .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
            let end = *plan
                .get(offset(
                    ranges.layout.target_offset_words.start + target + 1,
                )?)
                .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
            if end > recurrent || begin > end {
                return Err(ScaffoldContractError::NeuralBackendUnavailable);
            }
            for cursor in begin..end {
                let alpha = *weights
                    .get((ranges.layout.alpha_words.start + cursor - weight_base) as usize)
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
                if alpha != (-0.0_f32).to_bits() {
                    continue;
                }
                let source = *plan
                    .get(offset(ranges.layout.source_index_words.start + cursor)?)
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
                let route = *plan
                    .get(offset(ranges.layout.route_index_words.start + cursor)?)
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
                let genetic = *weights
                    .get((ranges.layout.genetic_weight_words.start + cursor - weight_base) as usize)
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
                snapshot.structural_synapses.push(
                    crate::training_rollout::GpuTrainingStructuralSynapse {
                        slot_index: cursor,
                        source,
                        target,
                        route,
                        genetic_weight: f32::from_bits(genetic),
                        alpha: f32::from_bits(alpha),
                    },
                );
            }
        }
        if snapshot.structural_synapses.len() != live - inherited {
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        self.training_structural_cache.insert(
            key,
            TrainingStructuralCache {
                slot: snapshot.brain_slot.clone(),
                structural: snapshot.v11.structural.clone(),
                synapses: snapshot.structural_synapses.clone(),
            },
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packets_keep_all_fifty_rows_in_order_and_split_only_for_bytes() {
        assert_eq!(packets(&[4; 50], 256).unwrap(), vec![0..50]);
        assert_eq!(packets(&[4; 50], 128).unwrap(), vec![0..32, 32..50]);
        assert_eq!(packets(&[4, 8, 12, 4], 16).unwrap(), vec![0..2, 2..4]);
        assert!(packets(&[], 16).unwrap().is_empty());
        for bytes in [0, 3, 20, u64::MAX] {
            assert!(packets(&[bytes], 16).is_err());
        }
        assert!(packets(&[4], 0).is_err());
    }
    #[test]
    fn fused_boundaries_share_one_ceiling_and_preserve_request_order() {
        assert_eq!(
            fused_offsets(&[4, 8, 12], 48).unwrap(),
            Some(vec![0..4, 4..12, 12..24, 24..28, 28..36, 36..48])
        );
        assert!(fused_offsets(&[4, 8, 12], 47).unwrap().is_none());
        assert!(fused_offsets(
            &[TRAINING_CAPTURE_STAGING_BYTES / 2],
            TRAINING_CAPTURE_STAGING_BYTES
        )
        .unwrap()
        .is_some());
        assert!(fused_offsets(
            &[TRAINING_CAPTURE_STAGING_BYTES / 2 + 4],
            TRAINING_CAPTURE_STAGING_BYTES
        )
        .unwrap()
        .is_none());
        assert!(fused_offsets(&[u64::MAX - 3], u64::MAX).unwrap().is_none());
        for size in [0, 1, 3, 7] {
            assert!(fused_offsets(&[size], 100).is_err());
        }
    }

    #[test]
    fn capture_decoder_rejects_partial_layout_before_returning_any_rows() {
        let words = [0_u32, (-0.0_f32).to_bits(), 0x7fc01234, u32::MAX];
        let bytes = bytemuck::cast_slice(&words);
        assert_eq!(
            decode_capture_words(bytes, &[0..8, 8..16]).unwrap(),
            vec![words[..2].to_vec(), words[2..].to_vec()]
        );
        for offsets in [
            std::iter::once(0..8).collect(),
            vec![0..8, 8..20],
            vec![0..8, 9..16],
            vec![0..7, 7..16],
            vec![0..8, 8..8],
            std::iter::once(4..16).collect(),
        ] {
            assert!(decode_capture_words(bytes, &offsets).is_err());
        }
    }

    #[test]
    fn capture_mapping_failure_or_missing_callback_never_waits_for_a_sender() {
        let (sender, receiver) = std::sync::mpsc::channel();
        // A live but silent sender must not turn a failed poll into a hang.
        assert!(!timestamp_mapping_completed(&receiver));
        sender.send(Err(wgpu::BufferAsyncError)).unwrap();
        assert!(!timestamp_mapping_completed(&receiver));
        sender.send(Ok(())).unwrap();
        assert!(timestamp_mapping_completed(&receiver));
        drop(sender);
        assert!(!timestamp_mapping_completed(&receiver));
    }

    fn snapshot_fixture() -> Snapshot {
        use alife_core::{BrainGenome, DevelopmentState, NormalizedScalar, PhenotypeCompiler};
        let capacity = BrainCapacityClass::n512();
        let genome = BrainGenome::scaffold(17, capacity.id());
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
        let phenotype = Arc::new(
            PhenotypeCompiler::compile_testing_procedural_baseline(
                &genome,
                &capacity,
                &development,
                SensorProfile::PrivilegedAffordanceV1,
            )
            .unwrap(),
        );
        let mut plan = crate::GpuClassBucketPlan::new(capacity, 1).unwrap();
        let brain_slot = plan.insert_phenotype(0, 1, &phenotype).unwrap();
        Snapshot {
            handle: GpuBrainHandle::authority_receipt_test_fixture(
                phenotype.brain_class_id(),
                0,
                1,
                OrganismId(1),
                phenotype.phenotype_hash(),
            ),
            tick: 12,
            logical_dispatch_generation: 1,
            active_activation_side: 0,
            active_weight_generation: 1,
            active_weight_bank: 0,
            v11: GpuV11Checkpoint::canonical_for_phenotype(&phenotype).unwrap(),
            phenotype,
            brain_slot,
            mutable_word_base: 0,
            mutable_words: Vec::new(),
            structural_synapses: Vec::new(),
        }
    }

    #[test]
    fn prepared_capture_metadata_rejects_foreign_stale_tick_bank_and_structural_changes() {
        let expected = snapshot_fixture();
        assert!(capture_metadata_matches(&expected, &expected));
        for lane in 0..11 {
            let expected_generations = [1_u64; 11];
            let mut changed = expected_generations;
            changed[lane] += 1;
            assert!(!capture_boundary_matches(
                &expected,
                &expected,
                &expected_generations,
                &changed
            ));
        }
        let mut current = expected.clone();
        // Captured payload ownership is deliberately independent of metadata.
        current.mutable_words = vec![42];
        assert!(capture_metadata_matches(&expected, &current));
        current.handle.backend_instance_id = NonZeroU64::new(2).unwrap();
        assert!(!capture_metadata_matches(&expected, &current));
        for mutation in 0..7 {
            let mut current = expected.clone();
            match mutation {
                0 => current.handle.generation += 1,
                1 => current.tick += 1,
                2 => current.logical_dispatch_generation += 1,
                3 => current.active_activation_side ^= 1,
                4 => current.active_weight_generation += 1,
                5 => current.active_weight_bank ^= 1,
                _ => current.mutable_word_base += 1,
            }
            assert!(
                !capture_metadata_matches(&expected, &current),
                "mutation {mutation}"
            );
        }
        let mut current = expected.clone();
        current.v11.work = GpuV11WorkReceipt::default();
        current.v11.schema_version += 1;
        assert!(!capture_metadata_matches(&expected, &current));
    }
}
