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
    pub readback_submissions: u64,
    pub bytes_copied: u64,
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

impl GpuClosedLoopBackend {
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
                Some(&self.readback_scratch),
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
                Some(&self.readback_scratch),
            )?;
            let weights = crate::closed_loop_sleep::read_gpu_words(
                &self.device,
                &self.queue,
                bucket.buffers.neural_buffers()[3],
                ranges.layout.genetic_weight_words.start
                    ..ranges.layout.alpha_words.start + snapshot.brain_slot.record().synapse_count,
                "training-structural-weight-readback",
                None,
                Some(&self.readback_scratch),
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
        attach_training_structural_into_cache(
            &mut self.training_structural_cache,
            snapshot,
            ranges,
            plan,
            weights,
        )
    }

    pub(super) fn prepare_fused_training_captures(
        &mut self,
        batch: &[GpuRuntimeTickInput<'_>],
    ) -> Result<Option<FusedTrainingCapturePlan>, ScaffoldContractError> {
        let limit = self
            .device
            .limits()
            .max_buffer_size
            .min(TRAINING_CAPTURE_STAGING_BYTES);
        let mut cursor = 0_u64;
        let mut rows = Vec::with_capacity(batch.len());
        for input in batch {
            let (mut before, ranges) =
                self.training_snapshot_metadata(input.handle, input.frame.tick())?;
            let mutable_bytes =
                u64::from(ranges.mutable_state_words.end - ranges.mutable_state_words.start) * 4;
            if mutable_bytes == 0 {
                return Err(ScaffoldContractError::NeuralBackendUnavailable);
            }
            let Some(before_end) = cursor.checked_add(mutable_bytes) else {
                return Ok(None);
            };
            let before_mutable_dst = cursor..before_end;
            cursor = before_end;

            let structural_dst = if self.training_structural_needed(&mut before) {
                let plan_words = ranges.layout.target_offset_words.start
                    ..ranges.layout.route_index_words.start
                        + before.brain_slot.record().recurrent_synapse_count;
                let weight_words = ranges.layout.genetic_weight_words.start
                    ..ranges.layout.alpha_words.start + before.brain_slot.record().synapse_count;
                let plan_bytes = u64::from(plan_words.end - plan_words.start) * 4;
                let weight_bytes = u64::from(weight_words.end - weight_words.start) * 4;
                let Some(plan_end) = cursor.checked_add(plan_bytes) else {
                    return Ok(None);
                };
                let Some(weight_end) = plan_end.checked_add(weight_bytes) else {
                    return Ok(None);
                };
                let plan_dst = cursor..plan_end;
                let weight_dst = plan_end..weight_end;
                cursor = weight_end;
                Some(StructuralCaptureSpans {
                    plan_dst,
                    weight_dst,
                    plan_words,
                    weight_words,
                })
            } else {
                None
            };

            let Some(after_end) = cursor.checked_add(mutable_bytes) else {
                return Ok(None);
            };
            let after_mutable_dst = cursor..after_end;
            cursor = after_end;

            if cursor > limit {
                return Ok(None);
            }
            rows.push(FusedTrainingCaptureRow {
                before,
                ranges,
                handle: input.handle,
                before_mutable_dst,
                structural_dst,
                after_mutable_dst,
            });
        }
        if cursor == 0 {
            return Ok(None);
        }
        let max_buffer_size = self.device.limits().max_buffer_size;
        let needs_alloc = self
            .training_capture_readback
            .as_ref()
            .is_none_or(|buffer| buffer.size() < cursor);
        if needs_alloc {
            let capacity = cursor
                .checked_next_power_of_two()
                .unwrap_or(cursor)
                .max(4096)
                .min(max_buffer_size)
                .max(cursor);
            self.training_capture_readback =
                Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("training-fused-capture-readback"),
                    size: capacity,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                }));
        }
        Ok(Some(FusedTrainingCapturePlan {
            rows,
            total_bytes: cursor,
        }))
    }

    pub(super) fn encode_fused_training_capture_before(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        plan: &FusedTrainingCapturePlan,
    ) -> Result<(), ScaffoldContractError> {
        let readback = self
            .training_capture_readback
            .as_ref()
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        for row in &plan.rows {
            let bucket = self
                .class_buckets
                .get(&row.handle.class_id.raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                .bucket_for_handle(row.handle)?;
            let neural = bucket.buffers.neural_buffers();
            encoder.copy_buffer_to_buffer(
                neural[6],
                u64::from(row.ranges.mutable_state_words.start) * 4,
                readback,
                row.before_mutable_dst.start,
                row.before_mutable_dst.end - row.before_mutable_dst.start,
            );
            if let Some(spans) = &row.structural_dst {
                encoder.copy_buffer_to_buffer(
                    neural[2],
                    u64::from(spans.plan_words.start) * 4,
                    readback,
                    spans.plan_dst.start,
                    spans.plan_dst.end - spans.plan_dst.start,
                );
                encoder.copy_buffer_to_buffer(
                    neural[3],
                    u64::from(spans.weight_words.start) * 4,
                    readback,
                    spans.weight_dst.start,
                    spans.weight_dst.end - spans.weight_dst.start,
                );
            }
        }
        Ok(())
    }

    pub(super) fn encode_fused_training_capture_after(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        plan: &FusedTrainingCapturePlan,
    ) -> Result<(), ScaffoldContractError> {
        let readback = self
            .training_capture_readback
            .as_ref()
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        for row in &plan.rows {
            let bucket = self
                .class_buckets
                .get(&row.handle.class_id.raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                .bucket_for_handle(row.handle)?;
            let neural = bucket.buffers.neural_buffers();
            encoder.copy_buffer_to_buffer(
                neural[6],
                u64::from(row.ranges.mutable_state_words.start) * 4,
                readback,
                row.after_mutable_dst.start,
                row.after_mutable_dst.end - row.after_mutable_dst.start,
            );
        }
        Ok(())
    }

    pub(super) fn finish_fused_training_captures(
        &mut self,
        plan: FusedTrainingCapturePlan,
        dispatch_generation: u64,
        ticks: &[crate::GpuClosedLoopTick],
        poll_wait_ns: u64,
        map_receive_wait_ns: u64,
    ) -> Result<(Vec<Snapshot>, Vec<Snapshot>), ScaffoldContractError> {
        if plan.rows.len() != ticks.len() {
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        let readback = self
            .training_capture_readback
            .as_ref()
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let cache = &mut self.training_structural_cache;
        let decode_result = {
            let mapped = readback.slice(..plan.total_bytes).get_mapped_range();
            let result = (|| -> Result<(Vec<Snapshot>, Vec<Snapshot>), ScaffoldContractError> {
                let mut before_snapshots = Vec::with_capacity(plan.rows.len());
                let mut after_snapshots = Vec::with_capacity(plan.rows.len());
                for (row, tick) in plan.rows.into_iter().zip(ticks) {
                    let mut before = row.before;
                    before.mutable_words = bytemuck::cast_slice::<u8, u32>(
                        &mapped[row.before_mutable_dst.start as usize
                            ..row.before_mutable_dst.end as usize],
                    )
                    .to_vec();
                    if let Some(spans) = &row.structural_dst {
                        let plan_slice = bytemuck::cast_slice::<u8, u32>(
                            &mapped[spans.plan_dst.start as usize..spans.plan_dst.end as usize],
                        );
                        let weight_slice = bytemuck::cast_slice::<u8, u32>(
                            &mapped[spans.weight_dst.start as usize..spans.weight_dst.end as usize],
                        );
                        attach_training_structural_into_cache(
                            cache,
                            &mut before,
                            &row.ranges,
                            plan_slice,
                            weight_slice,
                        )?;
                    }
                    let mut after = before.clone();
                    after.logical_dispatch_generation = dispatch_generation;
                    after.active_activation_side = tick.active_activation_side;
                    after.v11.work = tick.v11_work;
                    after.mutable_words = bytemuck::cast_slice::<u8, u32>(
                        &mapped[row.after_mutable_dst.start as usize
                            ..row.after_mutable_dst.end as usize],
                    )
                    .to_vec();
                    before_snapshots.push(before);
                    after_snapshots.push(after);
                }
                Ok((before_snapshots, after_snapshots))
            })();
            drop(mapped);
            readback.unmap();
            result
        };
        let (before_snapshots, after_snapshots) = decode_result?;
        self.mutable_slot_readback_counters.record(
            plan.total_bytes,
            poll_wait_ns,
            map_receive_wait_ns,
        );
        self.training_capture_metrics.batch_captures += 2;
        self.training_capture_metrics.captured_rows += (ticks.len() * 2) as u64;
        self.training_capture_metrics.readback_submissions += 1;
        self.training_capture_metrics.bytes_copied += plan.total_bytes;
        Ok((before_snapshots, after_snapshots))
    }
}

struct StructuralCaptureSpans {
    plan_dst: Range<u64>,
    weight_dst: Range<u64>,
    plan_words: Range<u32>,
    weight_words: Range<u32>,
}

pub(super) struct FusedTrainingCaptureRow {
    before: Snapshot,
    ranges: GpuFixedSlotRanges,
    handle: GpuBrainHandle,
    before_mutable_dst: Range<u64>,
    structural_dst: Option<StructuralCaptureSpans>,
    after_mutable_dst: Range<u64>,
}

pub(super) struct FusedTrainingCapturePlan {
    rows: Vec<FusedTrainingCaptureRow>,
    pub(super) total_bytes: u64,
}

fn attach_training_structural_into_cache(
    cache: &mut BTreeMap<(u16, u32, u32, u64), TrainingStructuralCache>,
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
    cache.insert(
        key,
        TrainingStructuralCache {
            slot: snapshot.brain_slot.clone(),
            structural: snapshot.v11.structural.clone(),
            synapses: snapshot.structural_synapses.clone(),
        },
    );
    Ok(())
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
}
