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
    #[cfg(feature = "test-support")]
    pub fn training_capture_packet_plan_for_cpu_test(
        sizes: &[u64],
        limit: u64,
    ) -> Result<Vec<Range<usize>>, ScaffoldContractError> {
        packets(sizes, limit)
    }

    /// Synthetic CPU-only replay fixture. This is not device authority or a
    /// captured execution; it exists only under the explicit test-support flag.
    #[cfg(feature = "test-support")]
    pub fn synthetic_training_snapshot_for_cpu_test(
        phenotype: std::sync::Arc<alife_core::BrainPhenotype>,
        organism_id: alife_core::OrganismId,
        tick: Tick,
    ) -> Result<Snapshot, ScaffoldContractError> {
        let capacity = BrainCapacityClass::production_for_id(phenotype.brain_class_id())?;
        let plan =
            crate::closed_loop_buffers::GpuFixedClassArenaPlan::new(capacity, 1, 512 * 1024 * 1024)
                .map_err(map_gpu_contract_error)?;
        let upload = plan
            .prepare_slot_upload(0, 1, &phenotype)
            .map_err(map_gpu_contract_error)?;
        let ranges = upload.ranges();
        Ok(Snapshot {
            handle: GpuBrainHandle {
                backend_instance_id: NonZeroU64::new(u64::MAX).expect("synthetic nonzero id"),
                class_id: phenotype.brain_class_id(),
                slot: 0,
                generation: 1,
                organism_id,
                phenotype_hash: phenotype.phenotype_hash(),
            },
            tick: tick.raw(),
            logical_dispatch_generation: 1,
            active_activation_side: 0,
            active_weight_generation: 13,
            active_weight_bank: 0,
            v11: crate::GpuV11Checkpoint::canonical_for_phenotype(&phenotype)?,
            brain_slot: upload.brain_slot().clone(),
            mutable_word_base: ranges.mutable_state_words.start,
            mutable_words: vec![
                0;
                (ranges.mutable_state_words.end - ranges.mutable_state_words.start)
                    as usize
            ],
            structural_synapses: Vec::new(),
            phenotype,
        })
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
}
