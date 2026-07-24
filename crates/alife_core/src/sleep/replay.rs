//! Portable bounded sleep replay batches and deterministic physical-ring packing.

use serde::{Deserialize, Serialize};

use crate::{
    ActionId, CandidateActionFamily, CandidateFeatureDigest, CanonicalDigestBuilder,
    ExperienceSequenceId, NeuromodulatorSample, PerceptionFrameDigest, ReplayCapturePlan,
    ScaffoldContractError, Tick,
};

pub const BOUNDED_REPLAY_BATCH_SCHEMA_VERSION: u16 = 1;
const REPLAY_BATCH_DIGEST_DOMAIN: &[u8] = b"ALIFE-GPU-SLEEP-REPLAY-BATCH-V1";

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SleepReplayEvent {
    pub sequence_id: ExperienceSequenceId,
    pub originating_tick: Tick,
    pub frame_digest: PerceptionFrameDigest,
    pub candidate_feature_digest: CandidateFeatureDigest,
    pub action_id: ActionId,
    pub family: CandidateActionFamily,
    pub modulator: NeuromodulatorSample,
}

impl SleepReplayEvent {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.sequence_id.validate()?;
        self.action_id.validate()?;
        if self.frame_digest == PerceptionFrameDigest([0; 4])
            || self.candidate_feature_digest == CandidateFeatureDigest([0; 2])
            || !self.modulator.value().is_finite()
            || !(-1.0..=1.0).contains(&self.modulator.value())
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayEligibilitySample {
    pub event_index: u16,
    pub eligibility_q15: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplaySynapseSpan {
    pub local_synapse_id: u32,
    pub sample_start: u32,
    pub sample_count: u32,
    pub reserved: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundedReplayBatch {
    pub schema_version: u16,
    pub events: Vec<SleepReplayEvent>,
    pub synapse_spans: Vec<ReplaySynapseSpan>,
    pub eligibility_samples: Vec<ReplayEligibilitySample>,
    pub canonical_digest: [u64; 4],
}

impl BoundedReplayBatch {
    pub fn recompute_canonical_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        let mut digest = CanonicalDigestBuilder::new(REPLAY_BATCH_DIGEST_DOMAIN);
        digest.write_u16(self.schema_version);
        digest.write_sequence_len(self.events.len());
        for event in &self.events {
            event.validate_contract()?;
            digest.write_u64(event.sequence_id.raw());
            digest.write_u64(event.originating_tick.raw());
            write_digest4(&mut digest, event.frame_digest.0);
            digest.write_u64(event.candidate_feature_digest.0[0]);
            digest.write_u64(event.candidate_feature_digest.0[1]);
            digest.write_u32(event.action_id.0);
            digest.write_u8(event.family.raw());
            digest.write_f32(event.modulator.reward_prediction_error())?;
            digest.write_f32(event.modulator.pain())?;
            digest.write_f32(event.modulator.homeostatic_improvement())?;
            digest.write_f32(event.modulator.frustration())?;
            digest.write_f32(event.modulator.novelty())?;
            digest.write_f32(event.modulator.value())?;
        }
        digest.write_sequence_len(self.synapse_spans.len());
        for span in &self.synapse_spans {
            digest.write_u32(span.local_synapse_id);
            digest.write_u32(span.sample_start);
            digest.write_u32(span.sample_count);
            digest.write_u32(span.reserved);
        }
        digest.write_sequence_len(self.eligibility_samples.len());
        for sample in &self.eligibility_samples {
            digest.write_u16(sample.event_index);
            digest.write_i16(sample.eligibility_q15);
        }
        Ok(digest.finish256())
    }

    pub fn validate_contract(
        &self,
        max_events: u32,
        max_samples: u32,
        synapse_count: u32,
    ) -> Result<(), ScaffoldContractError> {
        if self.schema_version != BOUNDED_REPLAY_BATCH_SCHEMA_VERSION
            || max_events == 0
            || max_events > 65_536
            || max_samples == 0
            || synapse_count == 0
            || self.events.len() > max_events as usize
            || self.events.len() > 65_536
            || self.eligibility_samples.len() > max_samples as usize
            || self
                .events
                .windows(2)
                .any(|pair| pair[0].sequence_id.raw() >= pair[1].sequence_id.raw())
            || self
                .events
                .iter()
                .any(|event| event.validate_contract().is_err())
            || self
                .synapse_spans
                .windows(2)
                .any(|pair| pair[0].local_synapse_id >= pair[1].local_synapse_id)
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }

        let event_count = u32::try_from(self.events.len())
            .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?;
        let mut expected_sample_start = 0_u32;
        for span in &self.synapse_spans {
            if span.local_synapse_id >= synapse_count
                || span.reserved != 0
                || span.sample_start != expected_sample_start
                || span.sample_count != event_count
            {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
            }
            let start = usize::try_from(span.sample_start)
                .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?;
            let end_u32 = span
                .sample_start
                .checked_add(span.sample_count)
                .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
            let end = usize::try_from(end_u32)
                .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?;
            let samples = self
                .eligibility_samples
                .get(start..end)
                .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
            for (event_index, sample) in samples.iter().enumerate() {
                if sample.event_index as usize != event_index || sample.eligibility_q15 == i16::MIN
                {
                    return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
                }
            }
            expected_sample_start = end_u32;
        }
        if usize::try_from(expected_sample_start).ok() != Some(self.eligibility_samples.len())
            || self.canonical_digest != self.recompute_canonical_digest()?
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SleepReplayJournal {
    plan: ReplayCapturePlan,
    cursor: u32,
    event_count: u32,
    events: Vec<Option<SleepReplayEvent>>,
    physical_samples: Vec<i16>,
}

impl SleepReplayJournal {
    pub fn new(plan: ReplayCapturePlan) -> Result<Self, ScaffoldContractError> {
        plan.validate_contract()?;
        let event_capacity = usize::try_from(plan.event_capacity())
            .map_err(|_| ScaffoldContractError::PhenotypeCompile)?;
        let sample_capacity = usize::try_from(plan.sample_capacity())
            .map_err(|_| ScaffoldContractError::PhenotypeCompile)?;
        Ok(Self {
            plan,
            cursor: 0,
            event_count: 0,
            events: vec![None; event_capacity],
            physical_samples: vec![0; sample_capacity],
        })
    }

    pub const fn cursor(&self) -> u32 {
        self.cursor
    }

    pub const fn event_count(&self) -> u32 {
        self.event_count
    }

    pub fn push(
        &mut self,
        event: SleepReplayEvent,
        eligibility: &[f32],
    ) -> Result<(), ScaffoldContractError> {
        event.validate_contract()?;
        if eligibility.len() != usize::from(self.plan.samples_per_event()) {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        if self.event_count != 0 {
            let capacity = self.plan.event_capacity();
            let previous = (self.cursor + capacity - 1) % capacity;
            let previous = self.events[previous as usize]
                .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
            if event.sequence_id.raw() <= previous.sequence_id.raw() {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
            }
        }
        let encoded = eligibility
            .iter()
            .copied()
            .map(encode_replay_eligibility_q15)
            .collect::<Result<Vec<_>, _>>()?;
        let capacity = self.plan.event_capacity();
        let physical_event = self.cursor;
        self.events[physical_event as usize] = Some(event);
        for (capture_index, sample) in encoded.into_iter().enumerate() {
            let start = u32::try_from(capture_index)
                .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?
                .checked_mul(capacity)
                .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
            let sample_index = start
                .checked_add(physical_event)
                .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
            self.physical_samples[sample_index as usize] = sample;
        }
        self.cursor = (self.cursor + 1) % capacity;
        self.event_count = self.event_count.saturating_add(1).min(capacity);
        Ok(())
    }

    pub fn build_bounded_batch(
        &self,
        max_events: u32,
        max_samples: u32,
        synapse_count: u32,
    ) -> Result<BoundedReplayBatch, ScaffoldContractError> {
        self.plan.validate_contract()?;
        if self.event_count > max_events {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        let capacity = self.plan.event_capacity();
        let oldest = if self.event_count == capacity {
            self.cursor
        } else {
            0
        };
        let physical_order = (0..self.event_count)
            .map(|offset| (oldest + offset) % capacity)
            .collect::<Vec<_>>();
        let events = physical_order
            .iter()
            .map(|physical| {
                self.events[*physical as usize]
                    .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut synapse_spans = Vec::with_capacity(self.plan.global_synapse_ids().len());
        let mut eligibility_samples = Vec::with_capacity(
            usize::try_from(self.event_count)
                .ok()
                .and_then(|count| count.checked_mul(self.plan.global_synapse_ids().len()))
                .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?,
        );
        for (capture_index, local_synapse_id) in
            self.plan.global_synapse_ids().iter().copied().enumerate()
        {
            let sample_start = u32::try_from(eligibility_samples.len())
                .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?;
            let physical_start = u32::try_from(capture_index)
                .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?
                .checked_mul(capacity)
                .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
            for (event_index, physical) in physical_order.iter().copied().enumerate() {
                eligibility_samples.push(ReplayEligibilitySample {
                    event_index: u16::try_from(event_index)
                        .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?,
                    eligibility_q15: self.physical_samples[(physical_start + physical) as usize],
                });
            }
            synapse_spans.push(ReplaySynapseSpan {
                local_synapse_id,
                sample_start,
                sample_count: self.event_count,
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
        batch.validate_contract(max_events, max_samples, synapse_count)?;
        Ok(batch)
    }
}

pub fn encode_replay_eligibility_q15(value: f32) -> Result<i16, ScaffoldContractError> {
    if !value.is_finite() {
        return Err(ScaffoldContractError::NonFiniteFloat);
    }
    let scaled = (value.clamp(-1.0, 1.0) * 32_767.0).round();
    Ok((scaled as i32).clamp(-32_767, 32_767) as i16)
}

pub fn decode_replay_eligibility_q15(value: i16) -> Result<f32, ScaffoldContractError> {
    if value == i16::MIN {
        return Err(ScaffoldContractError::ScalarOutOfRange);
    }
    Ok(f32::from(value) / 32_767.0)
}

fn write_digest4(builder: &mut CanonicalDigestBuilder, words: [u64; 4]) {
    for word in words {
        builder.write_u64(word);
    }
}
