//! Opt-in grounded grip interface. No target value or world desirability lives here.
use crate::{
    ActionId, CandidateActionFamily, Confidence, OrganismId, PerceptionBaseDigest,
    ScaffoldContractError, Tick, TrackedObjectId, Validate, WorldEntityId,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GripOperationV1 {
    Close,
    Release,
}
impl GripOperationV1 {
    pub const fn action_id(self) -> ActionId {
        match self {
            Self::Close => ActionId(2201),
            Self::Release => ActionId(2202),
        }
    }
    pub const fn family(self) -> CandidateActionFamily {
        match self {
            Self::Close => CandidateActionFamily::Contact,
            Self::Release => CandidateActionFamily::Other,
        }
    }
    /// The explicit cross-operation memory route is restricted to this ABI.
    pub fn for_candidate(
        profile: crate::SensorProfile,
        action: ActionId,
        kind: crate::ActionKind,
        family: CandidateActionFamily,
    ) -> Option<Self> {
        if profile != crate::SensorProfile::GroundedGripObjectSlotsV1
            || kind != crate::ActionKind::Interact
        {
            return None;
        }
        Self::from_action_id(action).filter(|operation| operation.family() == family)
    }
    pub fn from_action_id(action: ActionId) -> Option<Self> {
        match action.raw() {
            2201 => Some(Self::Close),
            2202 => Some(Self::Release),
            _ => None,
        }
    }
}

/// A physical constraint relation, never an object's kind, valence or safety.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GripOwnershipV1 {
    /// Body proprioception establishes only absence of one's own constraint.
    /// This does not assert that another organism or the player is not holding it.
    NotSelfHeld = 0,
    SelfHeld = 1,
}
impl GripOwnershipV1 {
    pub const fn operation(self) -> Option<GripOperationV1> {
        match self {
            Self::NotSelfHeld => Some(GripOperationV1::Close),
            Self::SelfHeld => Some(GripOperationV1::Release),
        }
    }
}

/// Kept outside the frozen 24 object lanes; serialized only in the new profile.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroundedGripSlotV1 {
    pub slot_index: u16,
    pub ownership: GripOwnershipV1,
    pub confidence: Confidence,
}
impl Validate for GroundedGripSlotV1 {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if usize::from(self.slot_index) >= crate::MAX_GROUNDED_OBJECT_SLOTS {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        Confidence::new(self.confidence.raw())?;
        Ok(())
    }
}

/// Transient, exact-frame transport for explicit physical execution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GripObservationV1 {
    pub organism_id: OrganismId,
    pub tick: Tick,
    pub target: WorldEntityId,
    pub tracked_object_id: TrackedObjectId,
    pub slot_index: u16,
    pub base_frame_digest: PerceptionBaseDigest,
    pub ownership: GripOwnershipV1,
    pub confidence: Confidence,
}
impl GripObservationV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        organism_id: OrganismId,
        tick: Tick,
        target: WorldEntityId,
        tracked_object_id: TrackedObjectId,
        slot_index: u16,
        base_frame_digest: PerceptionBaseDigest,
        ownership: GripOwnershipV1,
        confidence: Confidence,
    ) -> Result<Self, ScaffoldContractError> {
        let value = Self {
            organism_id,
            tick,
            target,
            tracked_object_id,
            slot_index,
            base_frame_digest,
            ownership,
            confidence,
        };
        value.validate_contract()?;
        Ok(value)
    }
}
impl Validate for GripObservationV1 {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        self.target.validate()?;
        self.tracked_object_id.validate()?;
        GroundedGripSlotV1 {
            slot_index: self.slot_index,
            ownership: self.ownership,
            confidence: self.confidence,
        }
        .validate_contract()
    }
}

/// Explicitly versioned neural input. Non-grip Other actions, including Play and
/// Vocalize, receive exact zero even when their own family expectation is bad.
/// Callers must already validate the finalized recall against this exact frame.
pub fn operation_value_input(
    frame: &crate::PerceptionFrame,
    candidate: &crate::ActionCandidate,
    context: &crate::CandidateMemoryContextV1,
) -> Result<f32, ScaffoldContractError> {
    if GripOperationV1::for_candidate(
        frame.sensor_profile(),
        candidate.action_id,
        candidate.kind,
        candidate.family,
    )
    .is_none()
    {
        return Ok(0.0);
    }
    let crate::CandidateObservationRef::ObjectSlot(index) = candidate.observation else {
        return Err(ScaffoldContractError::InvalidPerceptionFrame);
    };
    let grip = frame
        .grip_observations()
        .get(usize::from(index))
        .ok_or(ScaffoldContractError::InvalidPerceptionFrame)?;
    if grip.slot_index != index
        || candidate.target.entity.is_none()
        || context.candidate_index != candidate.candidate_index
        || frame
            .candidates()
            .get(usize::from(candidate.candidate_index))
            != Some(candidate)
        || grip.ownership.operation() != GripOperationV1::from_action_id(candidate.action_id)
    {
        return Err(ScaffoldContractError::InvalidPerceptionFrame);
    }
    context.validate_contract()?;
    Ok(context.family_value[0] * context.family_confidence.raw().clamp(0.0, 1.0))
}
