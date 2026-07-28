//! Lossless conversion between physical GPU replay rings and compact portable rows.

use alife_core::{
    ActionId, CandidateActionFamily, CandidateFeatureDigest, ExperienceSequenceId,
    NeuromodulatorSample, PerceptionFrameDigest, ReplayEligibilitySample, ReplaySynapseSpan,
    ScaffoldContractError, SleepReplayEvent, Tick,
};
use alife_gpu_backend::{
    pack_replay_eligibility_sample, unpack_replay_eligibility_sample, GpuReplayEventRecord,
    GpuReplaySynapseSpanRecord,
};
use alife_world::persistence::{PortableReplayJournalV1, GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION};

use super::state_codec::PhysicalReplayParts;

pub(crate) fn encode_portable_replay(
    phenotype_hash: alife_core::PhenotypeHash,
    replay_capture_plan_digest: [u64; 4],
    generation: u64,
    cursor: u32,
    event_count: u32,
    physical: &PhysicalReplayParts,
) -> Result<PortableReplayJournalV1, ScaffoldContractError> {
    let capacity = physical.events.len();
    let count = usize::try_from(event_count)
        .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?;
    let cursor_usize = usize::try_from(cursor)
        .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?;
    if capacity == 0
        || count > capacity
        || cursor_usize >= capacity
        || physical.samples.len() != capacity.saturating_mul(physical.spans.len())
    {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    let oldest = if count == capacity { cursor_usize } else { 0 };
    let physical_order = (0..count)
        .map(|offset| (oldest + offset) % capacity)
        .collect::<Vec<_>>();
    let mut events = Vec::with_capacity(count);
    for physical_index in &physical_order {
        events.push(decode_event(physical.events[*physical_index])?);
    }
    let active = physical_order
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    for (index, event) in physical.events.iter().enumerate() {
        if !active.contains(&index) && !event_is_zero(event) {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
    }

    let mut compact_spans = Vec::with_capacity(physical.spans.len());
    let mut compact_samples = Vec::with_capacity(count.saturating_mul(physical.spans.len()));
    let mut previous_synapse = None;
    for (capture_index, span) in physical.spans.iter().enumerate() {
        let physical_start = capture_index
            .checked_mul(capacity)
            .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
        if usize::try_from(span.sample_start).ok() != Some(physical_start)
            || usize::try_from(span.sample_count).ok() != Some(count)
            || span.reserved != 0
            || previous_synapse.is_some_and(|previous| previous >= span.local_synapse_id)
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        previous_synapse = Some(span.local_synapse_id);
        let compact_start = u32::try_from(compact_samples.len())
            .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?;
        for (logical_event, physical_event) in physical_order.iter().copied().enumerate() {
            let packed = physical.samples[physical_start + physical_event];
            let (sample_event, eligibility_q15) = unpack_replay_eligibility_sample(packed);
            if usize::from(sample_event) != physical_event || eligibility_q15 == i16::MIN {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
            }
            compact_samples.push(ReplayEligibilitySample {
                event_index: u16::try_from(logical_event)
                    .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?,
                eligibility_q15,
            });
        }
        for physical_event in 0..capacity {
            if !active.contains(&physical_event)
                && physical.samples[physical_start + physical_event] != 0
            {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
            }
        }
        compact_spans.push(ReplaySynapseSpan {
            local_synapse_id: span.local_synapse_id,
            sample_start: compact_start,
            sample_count: event_count,
            reserved: 0,
        });
    }

    let event_capacity = u32::try_from(capacity)
        .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?;
    let sample_capacity = u32::try_from(physical.samples.len())
        .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?;
    let mut portable = PortableReplayJournalV1 {
        schema_version: GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION,
        phenotype_hash,
        replay_capture_plan_digest,
        generation,
        cursor,
        event_count,
        event_capacity,
        sample_capacity,
        events,
        synapse_spans: compact_spans,
        eligibility_samples: compact_samples,
        canonical_digest: [0; 4],
    };
    portable.canonical_digest = portable.recompute_canonical_digest()?;
    portable.validate()?;
    Ok(portable)
}

pub(crate) fn decode_physical_replay(
    portable: &PortableReplayJournalV1,
) -> Result<PhysicalReplayParts, ScaffoldContractError> {
    portable.validate()?;
    let capacity = usize::try_from(portable.event_capacity)
        .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?;
    let count = usize::try_from(portable.event_count)
        .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?;
    let cursor = usize::try_from(portable.cursor)
        .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?;
    let oldest = if count == capacity { cursor } else { 0 };
    let physical_order = (0..count)
        .map(|offset| (oldest + offset) % capacity)
        .collect::<Vec<_>>();
    let mut events = vec![zero_event(); capacity];
    for (event, physical_index) in portable.events.iter().zip(&physical_order) {
        events[*physical_index] = encode_event(*event)?;
    }

    let mut spans = Vec::with_capacity(portable.synapse_spans.len());
    let mut samples = vec![
        0_u32;
        usize::try_from(portable.sample_capacity).map_err(|_| {
            ScaffoldContractError::ConsolidationGenerationMismatch
        })?
    ];
    for (capture_index, compact_span) in portable.synapse_spans.iter().enumerate() {
        let physical_start = capture_index
            .checked_mul(capacity)
            .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
        let compact_start = usize::try_from(compact_span.sample_start)
            .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?;
        let compact_end = compact_start
            .checked_add(count)
            .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
        let compact_samples = portable
            .eligibility_samples
            .get(compact_start..compact_end)
            .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
        for ((logical_event, sample), physical_event) in
            compact_samples.iter().enumerate().zip(&physical_order)
        {
            if usize::from(sample.event_index) != logical_event
                || sample.eligibility_q15 == i16::MIN
            {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
            }
            samples[physical_start + *physical_event] = pack_replay_eligibility_sample(
                u16::try_from(*physical_event)
                    .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?,
                sample.eligibility_q15,
            );
        }
        spans.push(GpuReplaySynapseSpanRecord {
            local_synapse_id: compact_span.local_synapse_id,
            sample_start: u32::try_from(physical_start)
                .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?,
            sample_count: portable.event_count,
            reserved: 0,
        });
    }
    Ok(PhysicalReplayParts {
        events,
        spans,
        samples,
    })
}

fn decode_event(row: GpuReplayEventRecord) -> Result<SleepReplayEvent, ScaffoldContractError> {
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
        frame_digest: PerceptionFrameDigest(join_digest4(row.frame_digest)),
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

fn encode_event(event: SleepReplayEvent) -> Result<GpuReplayEventRecord, ScaffoldContractError> {
    event.sequence_id.validate()?;
    event.action_id.validate()?;
    Ok(GpuReplayEventRecord {
        sequence_id: split_pair(event.sequence_id.raw()),
        originating_tick: split_pair(event.originating_tick.raw()),
        frame_digest: split_digest4(event.frame_digest.0),
        candidate_feature_digest: split_digest2(event.candidate_feature_digest.0),
        action_id: event.action_id.raw(),
        family: u32::from(event.family.raw()),
        reward_prediction_error: event.modulator.reward_prediction_error(),
        pain: event.modulator.pain(),
        homeostatic_improvement: event.modulator.homeostatic_improvement(),
        frustration: event.modulator.frustration(),
        novelty: event.modulator.novelty(),
        modulator_value: event.modulator.value(),
    })
}

const fn zero_event() -> GpuReplayEventRecord {
    GpuReplayEventRecord {
        sequence_id: [0; 2],
        originating_tick: [0; 2],
        frame_digest: [0; 8],
        candidate_feature_digest: [0; 4],
        action_id: 0,
        family: 0,
        reward_prediction_error: 0.0,
        pain: 0.0,
        homeostatic_improvement: 0.0,
        frustration: 0.0,
        novelty: 0.0,
        modulator_value: 0.0,
    }
}

fn event_is_zero(event: &GpuReplayEventRecord) -> bool {
    *event == zero_event()
}

const fn split_pair(value: u64) -> [u32; 2] {
    [value as u32, (value >> 32) as u32]
}

const fn join_pair(value: [u32; 2]) -> u64 {
    value[0] as u64 | ((value[1] as u64) << 32)
}

fn split_digest4(values: [u64; 4]) -> [u32; 8] {
    let mut result = [0; 8];
    for (index, value) in values.into_iter().enumerate() {
        result[index * 2] = value as u32;
        result[index * 2 + 1] = (value >> 32) as u32;
    }
    result
}

fn join_digest4(values: [u32; 8]) -> [u64; 4] {
    let mut result = [0; 4];
    for index in 0..4 {
        result[index] = join_pair([values[index * 2], values[index * 2 + 1]]);
    }
    result
}

fn split_digest2(values: [u64; 2]) -> [u32; 4] {
    [
        values[0] as u32,
        (values[0] >> 32) as u32,
        values[1] as u32,
        (values[1] >> 32) as u32,
    ]
}

fn join_digest2(values: [u32; 4]) -> [u64; 2] {
    [
        join_pair([values[0], values[1]]),
        join_pair([values[2], values[3]]),
    ]
}
