use alife_core::{
    CandidateObservationRef, PerceptionFrame, CANDIDATE_FEATURE_COUNT, MAX_ACTION_CANDIDATES,
};

use super::{
    GpuBrainSlot, GpuBrainSlotRecord, GpuCandidateRecord, GpuClosedLoopError, GpuPerceptionHeader,
    GPU_CLOSED_LOOP_LAYOUT_VERSION,
};

const FIXED_FRAME_LANES: u32 = 77;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuOffsetDomain {
    Local,
    Rebased { dispatch_base: u32, frame_base: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuPerceptionUpload {
    pub header: GpuPerceptionHeader,
    pub candidates: Vec<GpuCandidateRecord>,
    pub dispatch_header_words: Vec<u32>,
    pub frame_payload_words: Vec<u32>,
    offset_domain: GpuOffsetDomain,
}

impl GpuPerceptionUpload {
    pub fn validate_candidate_count(count: usize) -> Result<(), GpuClosedLoopError> {
        if count == 0 || count > MAX_ACTION_CANDIDATES {
            Err(GpuClosedLoopError::CapacityExceeded)
        } else {
            Ok(())
        }
    }
    pub fn try_from_frame(
        frame: &PerceptionFrame,
        slot: &GpuBrainSlot,
        active_activation_side: u32,
    ) -> Result<Self, GpuClosedLoopError> {
        Self::validate_candidate_count(frame.candidates().len())?;
        if active_activation_side > 1 {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        frame
            .validate()
            .map_err(|_| GpuClosedLoopError::MalformedUpload)?;
        let record = slot.record();
        record.validate_slice_a()?;
        let tick = frame.tick().raw();
        let header = GpuPerceptionHeader {
            // Dynamic dispatches inherit the slot's GPU layout ABI version.
            schema_version: record.schema_version,
            class_id: record.class_id,
            slot: record.slot,
            slot_generation: record.slot_generation,
            neuron_count: record.neuron_count,
            candidate_count: frame.candidates().len() as u32,
            microstep_count: record.microstep_count,
            active_activation_side,
            tick_lo: tick as u32,
            tick_hi: (tick >> 32) as u32,
            sensory_offset: 0,
            candidate_offset: 16,
            brain_slot_index: slot.brain_slot_index(),
            dispatch_generation_lo: 0,
            dispatch_generation_hi: 0,
            reserved: 0,
        };
        let mut candidates = Vec::with_capacity(frame.candidates().len());
        for (index, candidate) in frame.candidates().iter().enumerate() {
            if candidate.features.0.iter().any(|v| !v.is_finite()) {
                return Err(GpuClosedLoopError::NonFinitePayload);
            }
            candidates.push(GpuCandidateRecord {
                action_id: candidate.action_id.raw(),
                kind: candidate.kind.raw() as u32,
                family: candidate.family.raw() as u32,
                candidate_index: index as u32,
                feature_offset: FIXED_FRAME_LANES + (index * CANDIDATE_FEATURE_COUNT) as u32,
                observation_slot_or_max: match candidate.observation {
                    CandidateObservationRef::None => u32::MAX,
                    CandidateObservationRef::ObjectSlot(slot) => slot as u32,
                },
                confidence_q16: (candidate.sensor_confidence.raw() * 65535.0).round() as u32,
                effort_q16: (candidate.required_effort.raw() * 65535.0).round() as u32,
            });
        }
        let mut dispatch_header_words = header.words().to_vec();
        for candidate in &candidates {
            dispatch_header_words.extend_from_slice(candidate.words());
        }
        let mut frame_payload_words = Vec::with_capacity(
            FIXED_FRAME_LANES as usize + frame.candidates().len() * CANDIDATE_FEATURE_COUNT,
        );
        frame_payload_words.extend(frame.sensory().channels.as_flat_array().map(f32::to_bits));
        let body = frame.body();
        frame_payload_words.extend(
            [
                body.pose.translation.x,
                body.pose.translation.y,
                body.pose.translation.z,
                body.pose.rotation.x,
                body.pose.rotation.y,
                body.pose.rotation.z,
                body.pose.rotation.w,
                body.velocity.linear.x,
                body.velocity.linear.y,
                body.velocity.linear.z,
                body.velocity.angular.x,
                body.velocity.angular.y,
                body.velocity.angular.z,
            ]
            .map(f32::to_bits),
        );
        frame_payload_words.extend(frame.homeostasis().drives.to_array().map(f32::to_bits));
        frame_payload_words.extend(frame.homeostasis().hormones.to_array().map(f32::to_bits));
        for candidate in frame.candidates() {
            frame_payload_words.extend(candidate.features.0.map(f32::to_bits));
        }
        if frame_payload_words.len() < FIXED_FRAME_LANES as usize
            || frame_payload_words
                .iter()
                .any(|bits| !f32::from_bits(*bits).is_finite())
        {
            return Err(GpuClosedLoopError::NonFinitePayload);
        }
        Ok(Self {
            header,
            candidates,
            dispatch_header_words,
            frame_payload_words,
            offset_domain: GpuOffsetDomain::Local,
        })
    }
    pub fn validate_against(
        &self,
        frame: &PerceptionFrame,
        slot: &GpuBrainSlot,
    ) -> Result<(), GpuClosedLoopError> {
        self.header.validate_layout_for_slot(slot.record())?;
        if self.header.active_activation_side > 1 || self.header.reserved != 0 {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        let mut expected = Self::try_from_frame(frame, slot, self.header.active_activation_side)?;
        if let GpuOffsetDomain::Rebased {
            dispatch_base,
            frame_base,
        } = self.offset_domain
        {
            expected.rebase(dispatch_base, frame_base)?;
        }
        if self != &expected {
            Err(GpuClosedLoopError::MalformedUpload)
        } else {
            Ok(())
        }
    }
    /// Rebase local dispatch and frame offsets exactly once when appending to shared heaps.
    pub fn rebase(
        &mut self,
        dispatch_word_base: u32,
        frame_word_base: u32,
    ) -> Result<(), GpuClosedLoopError> {
        if self.offset_domain != GpuOffsetDomain::Local {
            return Err(GpuClosedLoopError::InvalidOffsetDomain);
        }
        let candidate_offset = self
            .header
            .candidate_offset
            .checked_add(dispatch_word_base)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let sensory_offset = self
            .header
            .sensory_offset
            .checked_add(frame_word_base)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let feature_offsets = self
            .candidates
            .iter()
            .map(|row| {
                row.feature_offset
                    .checked_add(frame_word_base)
                    .ok_or(GpuClosedLoopError::ArithmeticOverflow)
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.header.candidate_offset = candidate_offset;
        self.header.sensory_offset = sensory_offset;
        for (row, feature_offset) in self.candidates.iter_mut().zip(feature_offsets) {
            row.feature_offset = feature_offset;
        }
        self.dispatch_header_words.clear();
        self.dispatch_header_words
            .extend_from_slice(self.header.words());
        for row in &self.candidates {
            self.dispatch_header_words.extend_from_slice(row.words());
        }
        self.offset_domain = GpuOffsetDomain::Rebased {
            dispatch_base: dispatch_word_base,
            frame_base: frame_word_base,
        };
        Ok(())
    }

    pub const fn offset_domain(&self) -> GpuOffsetDomain {
        self.offset_domain
    }
}

impl GpuPerceptionHeader {
    pub fn validate_layout_for_slot(
        &self,
        slot: &GpuBrainSlotRecord,
    ) -> Result<(), GpuClosedLoopError> {
        if self.schema_version != GPU_CLOSED_LOOP_LAYOUT_VERSION
            || slot.schema_version != GPU_CLOSED_LOOP_LAYOUT_VERSION
        {
            return Err(GpuClosedLoopError::LayoutMismatch);
        }
        Ok(())
    }
}
