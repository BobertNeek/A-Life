//! Same-tick perception and unscored action-candidate records.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    ensure_current_version, ActionCommand, ActionId, ActionKind, ActionTarget,
    CanonicalDigestBuilder, CompressedSemanticCode, Confidence, ContextStreams, DriveSnapshot,
    DurationTicks, EndocrineSnapshot, EnvironmentStreamEntry, GaussianContextRef,
    GaussianSalienceEntry, GroundedObjectSlotV1, HeardToken, HomeostaticSnapshot, Intensity,
    LanguageContextSnapshot, NormalizedScalar, OrganismId, Pose, Quatf, ScaffoldContractError,
    SchemaKind, SchemaVersions, SemanticContextRef, SemanticSalienceEntry, SensorProfileProvenance,
    SensoryChannels, SensorySnapshot, SocialAgentSnapshot, SocialContextSnapshot,
    SocialProximityEntry, TeacherPerceptionChannel, Tick, Validate, Vec3f, Velocity,
    VocalizedToken, MAX_GROUNDED_OBJECT_SLOTS,
};

pub const CANDIDATE_FEATURE_COUNT: usize = 24;
pub const MAX_ACTION_CANDIDATES: usize = 32;

const PERCEPTION_BASE_DOMAIN: &[u8] = b"alife.perception.base.v1";
const PERCEPTION_CONTEXT_DOMAIN: &[u8] = b"alife.perception.context.v1";
const PERCEPTION_FRAME_DOMAIN: &[u8] = b"alife.perception.frame.v1";
const CANDIDATE_FEATURE_DOMAIN: &[u8] = b"alife.candidate.features.v1";

/// Evidence of a bounded, score-free candidate opportunity selection.
/// This receipt authenticates measured work and observed slots, never action scores.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CandidateOpportunityCoverageV1 {
    observed_slots: u16,
    capacity: u8,
    contact_slot: Option<u16>,
    focal_slot: Option<u16>,
    phase: u64,
    selected_slots: Vec<u16>,
    work_units: u64,
}
impl CandidateOpportunityCoverageV1 {
    pub fn new(
        observed_slots: u16,
        capacity: u8,
        contact_slot: Option<u16>,
        focal_slot: Option<u16>,
        phase: u64,
        selected_slots: Vec<u16>,
        work_units: u64,
    ) -> Result<Self, ScaffoldContractError> {
        let value = Self {
            observed_slots,
            capacity,
            contact_slot,
            focal_slot,
            phase,
            selected_slots,
            work_units,
        };
        value.validate_contract()?;
        Ok(value)
    }
    pub const fn observed_slots(&self) -> u16 {
        self.observed_slots
    }
    pub const fn capacity(&self) -> u8 {
        self.capacity
    }
    pub const fn contact_slot(&self) -> Option<u16> {
        self.contact_slot
    }
    pub const fn focal_slot(&self) -> Option<u16> {
        self.focal_slot
    }
    pub const fn phase(&self) -> u64 {
        self.phase
    }
    pub fn selected_slots(&self) -> &[u16] {
        &self.selected_slots
    }
    pub const fn work_units(&self) -> u64 {
        self.work_units
    }
    fn validate_candidates(
        &self,
        slots: &[GroundedObjectSlotV1],
        candidates: &[ActionCandidate],
    ) -> Result<(), ScaffoldContractError> {
        self.validate_contract()?;
        if usize::from(self.observed_slots) != slots.len() {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        let mut masks = [0u8; 16];
        for c in candidates {
            if let CandidateObservationRef::ObjectSlot(slot) = c.observation {
                if !self.selected_slots.contains(&slot) {
                    return Err(ScaffoldContractError::InvalidPerceptionFrame);
                }
                let bit = match c.family {
                    CandidateActionFamily::Inspect => 1,
                    CandidateActionFamily::Approach => 2,
                    CandidateActionFamily::Avoid => 4,
                    CandidateActionFamily::Ingest => 8,
                    CandidateActionFamily::Contact => 16,
                    CandidateActionFamily::Other => 32,
                    _ => return Err(ScaffoldContractError::InvalidPerceptionFrame),
                };
                let mask = &mut masks[usize::from(slot)];
                if *mask & bit != 0 {
                    return Err(ScaffoldContractError::InvalidPerceptionFrame);
                }
                *mask |= bit;
            }
        }
        if self
            .selected_slots
            .iter()
            .any(|&slot| masks[usize::from(slot)] != 63)
        {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        Ok(())
    }
}
impl Validate for CandidateOpportunityCoverageV1 {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        let mut seen = [false; 16];
        if self.observed_slots > 16
            || self.capacity > 16
            || self.selected_slots.len() > usize::from(self.capacity)
            || self.work_units > crate::cognitive_work::MAX_COGNITIVE_WORK_COUNTER
        {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        for &slot in &self.selected_slots {
            if slot >= self.observed_slots || seen[usize::from(slot)] {
                return Err(ScaffoldContractError::InvalidPerceptionFrame);
            }
            seen[usize::from(slot)] = true;
        }
        if [self.contact_slot, self.focal_slot]
            .into_iter()
            .flatten()
            .any(|slot| !self.selected_slots.contains(&slot))
        {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        Ok(())
    }
}
impl<'de> Deserialize<'de> for CandidateOpportunityCoverageV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            observed_slots: u16,
            capacity: u8,
            contact_slot: Option<u16>,
            focal_slot: Option<u16>,
            phase: u64,
            selected_slots: Vec<u16>,
            work_units: u64,
        }
        let w = Wire::deserialize(deserializer)?;
        Self::new(
            w.observed_slots,
            w.capacity,
            w.contact_slot,
            w.focal_slot,
            w.phase,
            w.selected_slots,
            w.work_units,
        )
        .map_err(D::Error::custom)
    }
}

/// World-measured global own-grip observation. This is not target ownership or a policy.
/// Logical tariff: one occupancy check per visited object, one output/reduction,
/// and two calibration operations (multiply and clamp). No runtime-speed claim.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct GripProprioceptionV1 {
    inspected_objects: u64,
    held: bool,
    touch_gain: f32,
}

impl GripProprioceptionV1 {
    pub fn new(
        inspected_objects: u64,
        held: bool,
        touch_gain: f32,
    ) -> Result<Self, ScaffoldContractError> {
        let value = Self {
            inspected_objects,
            held,
            touch_gain,
        };
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.inspected_objects > crate::cognitive_work::MAX_COGNITIVE_WORK_COUNTER - 3
            || (self.held && self.inspected_objects == 0)
            || !self.touch_gain.is_finite()
            || self.touch_gain < 0.0
        {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        Ok(())
    }
    pub const fn inspected_objects(&self) -> u64 {
        self.inspected_objects
    }
    pub const fn held(&self) -> bool {
        self.held
    }
    pub const fn touch_gain(&self) -> f32 {
        self.touch_gain
    }
    pub fn calibrated_output(&self) -> f32 {
        (f32::from(self.held) * self.touch_gain).clamp(0.0, 1.0)
    }
    pub const fn work_units(&self) -> u64 {
        self.inspected_objects + 3
    }
}

impl<'de> Deserialize<'de> for GripProprioceptionV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            inspected_objects: u64,
            held: bool,
            touch_gain: f32,
        }
        let wire = Wire::deserialize(deserializer)?;
        Self::new(wire.inspected_objects, wire.held, wire.touch_gain).map_err(D::Error::custom)
    }
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SensorProfile {
    #[default]
    PrivilegedAffordanceV1 = 1,
    GroundedObjectSlotsV1 = 2,
    GroundedTerrainVisionV1 = 3,
    /// Terrain vision plus explicitly versioned grounded visible-object neural ports.
    GroundedTerrainObjectPortsV2 = 4,
}

impl SensorProfile {
    pub const fn raw(self) -> u16 {
        self as u16
    }

    pub const fn has_terrain_vision(self) -> bool {
        matches!(
            self,
            Self::GroundedTerrainVisionV1 | Self::GroundedTerrainObjectPortsV2
        )
    }

    pub fn try_from_raw(raw: u16) -> Result<Self, ScaffoldContractError> {
        match raw {
            1 => Ok(Self::PrivilegedAffordanceV1),
            2 => Ok(Self::GroundedObjectSlotsV1),
            3 => Ok(Self::GroundedTerrainVisionV1),
            4 => Ok(Self::GroundedTerrainObjectPortsV2),
            _ => Err(ScaffoldContractError::SensorProfileMismatch),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PolicyBackend {
    #[default]
    NeuralClosedLoopGpu,
    HeuristicBaseline,
    /// Explicit offline neural execution; never an automatic gameplay fallback.
    NeuralClosedLoopCpu,
}

impl PolicyBackend {
    pub const fn requires_gpu(self) -> bool {
        matches!(self, Self::NeuralClosedLoopGpu)
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CandidateActionFamily {
    Idle = 0,
    Rest = 1,
    Inspect = 2,
    Approach = 3,
    Avoid = 4,
    Contact = 5,
    Ingest = 6,
    Other = 7,
}

impl CandidateActionFamily {
    pub const fn raw(self) -> u8 {
        self as u8
    }

    pub fn try_from_raw(raw: u8) -> Result<Self, ScaffoldContractError> {
        match raw {
            0 => Ok(Self::Idle),
            1 => Ok(Self::Rest),
            2 => Ok(Self::Inspect),
            3 => Ok(Self::Approach),
            4 => Ok(Self::Avoid),
            5 => Ok(Self::Contact),
            6 => Ok(Self::Ingest),
            7 => Ok(Self::Other),
            _ => Err(ScaffoldContractError::InvalidActionCandidate),
        }
    }

    pub const fn is_compatible_with(self, kind: ActionKind) -> bool {
        matches!(
            (self, kind),
            (Self::Idle, ActionKind::Idle)
                | (Self::Rest, ActionKind::Rest)
                | (Self::Inspect, ActionKind::Inspect | ActionKind::Look)
                | (Self::Approach | Self::Avoid, ActionKind::Move)
                | (Self::Contact | Self::Ingest, ActionKind::Interact)
                | (
                    Self::Other,
                    ActionKind::Interact
                        | ActionKind::Hold
                        | ActionKind::Gesture
                        | ActionKind::Vocalize
                        | ActionKind::Write
                )
        )
    }

    pub const fn baseline_for_kind(kind: ActionKind) -> Self {
        match kind {
            ActionKind::Idle => Self::Idle,
            ActionKind::Rest => Self::Rest,
            ActionKind::Inspect | ActionKind::Look => Self::Inspect,
            ActionKind::Move => Self::Approach,
            ActionKind::Interact => Self::Contact,
            ActionKind::Hold | ActionKind::Gesture | ActionKind::Vocalize | ActionKind::Write => {
                Self::Other
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CandidateObservationRef {
    None,
    ObjectSlot(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BodySnapshot {
    pub pose: Pose,
    pub velocity: Velocity,
}

impl BodySnapshot {
    /// Neural observation only; authoritative poses and causal receipts keep
    /// their real coordinates. Terrain vision does not include an innate GPS.
    pub fn neural_projection(mut self, profile: SensorProfile) -> Self {
        if profile.has_terrain_vision() {
            self.pose.translation = Vec3f::ZERO;
        }
        self
    }
}

impl Validate for BodySnapshot {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.pose.validate()?;
        self.velocity.validate()?;
        Ok(())
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CandidateFeatureVector(pub [f32; CANDIDATE_FEATURE_COUNT]);

impl CandidateFeatureVector {
    pub const INTRINSIC_ACTION_BASIS_LANE: usize = 23;

    pub const fn zero() -> Self {
        Self([0.0; CANDIDATE_FEATURE_COUNT])
    }

    /// Describe a targetless action opportunity without assigning a score.
    /// Object candidates retain the physical meaning of these same lanes.
    pub fn with_intrinsic_action_basis(mut self, kind: ActionKind) -> Self {
        self.0[Self::INTRINSIC_ACTION_BASIS_LANE] = 1.0;
        self.0[21] = f32::from(matches!(kind, ActionKind::Rest | ActionKind::Hold));
        self.0[22] = f32::from(kind == ActionKind::Vocalize);
        self
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.validate_contract()
    }
}

impl Default for CandidateFeatureVector {
    fn default() -> Self {
        Self::zero()
    }
}

impl Validate for CandidateFeatureVector {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self
            .0
            .iter()
            .all(|value| value.is_finite() && (-1.0..=1.0).contains(value))
        {
            Ok(())
        } else {
            Err(ScaffoldContractError::InvalidActionCandidate)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PerceptionBaseDigest(pub [u64; 4]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PerceptionContextDigest(pub [u64; 4]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PerceptionFrameDigest(pub [u64; 4]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CandidateFeatureDigest(pub [u64; 2]);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActionCandidate {
    pub candidate_index: u16,
    pub action_id: ActionId,
    pub kind: ActionKind,
    pub family: CandidateActionFamily,
    pub observation: CandidateObservationRef,
    pub target: ActionTarget,
    pub features: CandidateFeatureVector,
    pub sensor_confidence: Confidence,
    pub required_effort: NormalizedScalar,
    pub min_duration: DurationTicks,
    pub max_duration: DurationTicks,
}

impl ActionCandidate {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        candidate_index: u16,
        action_id: ActionId,
        kind: ActionKind,
        family: CandidateActionFamily,
        observation: CandidateObservationRef,
        target: ActionTarget,
        features: CandidateFeatureVector,
        sensor_confidence: Confidence,
        required_effort: NormalizedScalar,
        min_duration: DurationTicks,
        max_duration: DurationTicks,
    ) -> Result<Self, ScaffoldContractError> {
        let candidate = Self {
            candidate_index,
            action_id,
            kind,
            family,
            observation,
            target,
            features,
            sensor_confidence,
            required_effort,
            min_duration,
            max_duration,
        };
        candidate.validate_contract()?;
        Ok(candidate)
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.validate_contract()
    }

    pub fn feature_digest(self) -> Result<CandidateFeatureDigest, ScaffoldContractError> {
        self.validate_contract()?;
        let mut builder = CanonicalDigestBuilder::new(CANDIDATE_FEATURE_DOMAIN);
        encode_candidate_feature(&mut builder, &self)?;
        Ok(CandidateFeatureDigest(builder.finish128()))
    }

    pub fn to_command(
        self,
        organism_id: OrganismId,
        neural_confidence: Confidence,
    ) -> Result<ActionCommand, ScaffoldContractError> {
        self.validate_contract()?;
        ActionCommand::structured(
            organism_id,
            self.action_id,
            self.kind,
            self.target,
            Intensity::new(1.0)?,
            self.min_duration,
            neural_confidence,
            0,
            None,
            None,
            None,
        )
    }
}

impl Validate for ActionCandidate {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.action_id
            .validate()
            .map_err(|_| ScaffoldContractError::InvalidActionCandidate)?;
        self.target
            .validate()
            .map_err(|_| ScaffoldContractError::InvalidActionCandidate)?;
        self.features.validate_contract()?;
        Confidence::new(self.sensor_confidence.raw())
            .map_err(|_| ScaffoldContractError::InvalidActionCandidate)?;
        NormalizedScalar::new(self.required_effort.raw())
            .map_err(|_| ScaffoldContractError::InvalidActionCandidate)?;
        if self.min_duration.raw() == 0
            || self.max_duration.raw() == 0
            || self.min_duration.raw() > self.max_duration.raw()
            || !self.family.is_compatible_with(self.kind)
        {
            return Err(ScaffoldContractError::InvalidActionCandidate);
        }
        Ok(())
    }
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PerceptionContextKind {
    None = 0,
    EpisodicCandidateV1 = 1,
}

impl PerceptionContextKind {
    pub const fn raw(self) -> u16 {
        self as u16
    }

    pub fn try_from_raw(raw: u16) -> Result<Self, ScaffoldContractError> {
        match raw {
            0 => Ok(Self::None),
            1 => Ok(Self::EpisodicCandidateV1),
            _ => Err(ScaffoldContractError::InvalidPerceptionFrame),
        }
    }
}

/// World-authenticated self-capability, not a sensor profile or a scored action.
/// The World binds the constructor's architecture to the organism's genome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrientationRepertoireProvenance {
    GroundedOrientationV3,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PerceptionFrameDraft {
    schema_version: u16,
    organism_id: OrganismId,
    tick: Tick,
    sensor_profile: SensorProfile,
    sensory: SensorySnapshot,
    body: BodySnapshot,
    homeostasis: HomeostaticSnapshot,
    candidates: Vec<ActionCandidate>,
    profile_provenance: SensorProfileProvenance,
    grounded_object_slots: Vec<GroundedObjectSlotV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    orientation_repertoire: Option<OrientationRepertoireProvenance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    grip_proprioception: Option<GripProprioceptionV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    candidate_opportunity_coverage: Option<CandidateOpportunityCoverageV1>,
    base_digest: PerceptionBaseDigest,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PerceptionContextBlock {
    schema_version: u16,
    context_kind: PerceptionContextKind,
    values: Vec<f32>,
    canonical_digest: PerceptionContextDigest,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PerceptionFrame {
    base: PerceptionFrameDraft,
    context: PerceptionContextBlock,
    frame_digest: PerceptionFrameDigest,
}

impl PerceptionFrameDraft {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        organism_id: OrganismId,
        tick: Tick,
        sensor_profile: SensorProfile,
        sensory: SensorySnapshot,
        body: BodySnapshot,
        homeostasis: HomeostaticSnapshot,
        candidates: Vec<ActionCandidate>,
        profile_provenance: SensorProfileProvenance,
        grounded_object_slots: Vec<GroundedObjectSlotV1>,
    ) -> Result<Self, ScaffoldContractError> {
        Self::new_with_orientation_repertoire(
            organism_id,
            tick,
            sensor_profile,
            sensory,
            body,
            homeostasis,
            candidates,
            profile_provenance,
            grounded_object_slots,
            None,
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_grounded_orientation_v3(
        organism_id: OrganismId,
        tick: Tick,
        sensor_profile: SensorProfile,
        sensory: SensorySnapshot,
        body: BodySnapshot,
        homeostasis: HomeostaticSnapshot,
        candidates: Vec<ActionCandidate>,
        profile_provenance: SensorProfileProvenance,
        grounded_object_slots: Vec<GroundedObjectSlotV1>,
        architecture: &crate::genome::CognitiveArchitectureGenomeParameters,
    ) -> Result<Self, ScaffoldContractError> {
        architecture.validate_contract()?;
        if !architecture.grounded_orientation_repertoire() {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        Self::new_with_orientation_repertoire(
            organism_id,
            tick,
            sensor_profile,
            sensory,
            body,
            homeostasis,
            candidates,
            profile_provenance,
            grounded_object_slots,
            Some(OrientationRepertoireProvenance::GroundedOrientationV3),
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new_with_orientation_repertoire(
        organism_id: OrganismId,
        tick: Tick,
        sensor_profile: SensorProfile,
        sensory: SensorySnapshot,
        body: BodySnapshot,
        homeostasis: HomeostaticSnapshot,
        candidates: Vec<ActionCandidate>,
        profile_provenance: SensorProfileProvenance,
        grounded_object_slots: Vec<GroundedObjectSlotV1>,
        orientation_repertoire: Option<OrientationRepertoireProvenance>,
        grip_proprioception: Option<GripProprioceptionV1>,
        candidate_opportunity_coverage: Option<CandidateOpportunityCoverageV1>,
    ) -> Result<Self, ScaffoldContractError> {
        let schema_version = SchemaVersions::CURRENT.perception.raw();
        validate_frame_base(
            schema_version,
            organism_id,
            tick,
            sensor_profile,
            &sensory,
            body,
            &homeostasis,
            &candidates,
            profile_provenance,
            &grounded_object_slots,
            orientation_repertoire,
            grip_proprioception,
            candidate_opportunity_coverage.as_ref(),
        )?;
        let base_digest = compute_base_digest(
            schema_version,
            organism_id,
            tick,
            sensor_profile,
            &sensory,
            body,
            &homeostasis,
            &candidates,
            profile_provenance,
            &grounded_object_slots,
            orientation_repertoire,
            grip_proprioception,
            candidate_opportunity_coverage.as_ref(),
        );
        Ok(Self {
            schema_version,
            organism_id,
            tick,
            sensor_profile,
            sensory,
            body,
            homeostasis,
            candidates,
            profile_provenance,
            grounded_object_slots,
            orientation_repertoire,
            grip_proprioception,
            candidate_opportunity_coverage,
            base_digest,
        })
    }

    pub fn candidate_opportunity_coverage(&self) -> Option<&CandidateOpportunityCoverageV1> {
        self.candidate_opportunity_coverage.as_ref()
    }
    pub fn with_opportunity_coverage(
        mut self,
        receipt: CandidateOpportunityCoverageV1,
    ) -> Result<Self, ScaffoldContractError> {
        if self.candidate_opportunity_coverage.is_some() {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        self.candidate_opportunity_coverage = Some(receipt);
        let candidates = std::mem::take(&mut self.candidates);
        self.with_candidates(candidates)
    }
    /// Replace the opportunity set and its measured receipt as one validated value.
    pub fn with_candidates_and_opportunity_coverage(
        mut self,
        candidates: Vec<ActionCandidate>,
        receipt: CandidateOpportunityCoverageV1,
    ) -> Result<Self, ScaffoldContractError> {
        self.candidate_opportunity_coverage = Some(receipt);
        self.with_candidates(candidates)
    }

    /// Bind a measured observation without altering any input or candidate.
    pub fn with_grip_proprioception(
        mut self,
        receipt: GripProprioceptionV1,
    ) -> Result<Self, ScaffoldContractError> {
        self.grip_proprioception = Some(receipt);
        Self::new_with_orientation_repertoire(
            self.organism_id,
            self.tick,
            self.sensor_profile,
            self.sensory,
            self.body,
            self.homeostasis,
            self.candidates,
            self.profile_provenance,
            self.grounded_object_slots,
            self.orientation_repertoire,
            self.grip_proprioception,
            self.candidate_opportunity_coverage,
        )
    }
    pub const fn grip_proprioception(&self) -> Option<GripProprioceptionV1> {
        self.grip_proprioception
    }

    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    pub const fn grounded_orientation_repertoire(&self) -> bool {
        self.orientation_repertoire.is_some()
    }

    pub const fn organism_id(&self) -> OrganismId {
        self.organism_id
    }

    pub const fn tick(&self) -> Tick {
        self.tick
    }

    pub const fn sensor_profile(&self) -> SensorProfile {
        self.sensor_profile
    }

    pub fn sensory(&self) -> &SensorySnapshot {
        &self.sensory
    }

    pub const fn body(&self) -> BodySnapshot {
        self.body
    }

    pub fn homeostasis(&self) -> &HomeostaticSnapshot {
        &self.homeostasis
    }

    pub fn candidates(&self) -> &[ActionCandidate] {
        &self.candidates
    }

    pub const fn profile_provenance(&self) -> SensorProfileProvenance {
        self.profile_provenance
    }

    pub fn grounded_object_slots(&self) -> &[GroundedObjectSlotV1] {
        &self.grounded_object_slots
    }

    pub const fn base_digest(&self) -> PerceptionBaseDigest {
        self.base_digest
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.validate_contract()
    }

    pub fn finalize(
        self,
        context: PerceptionContextBlock,
    ) -> Result<PerceptionFrame, ScaffoldContractError> {
        let frame_digest = compute_frame_digest(self.base_digest, &context);
        Ok(PerceptionFrame {
            base: self,
            context,
            frame_digest,
        })
    }

    /// Rebind candidates without dropping the authenticated self-capability.
    pub fn with_candidates(
        self,
        candidates: Vec<ActionCandidate>,
    ) -> Result<Self, ScaffoldContractError> {
        Self::new_with_orientation_repertoire(
            self.organism_id,
            self.tick,
            self.sensor_profile,
            self.sensory,
            self.body,
            self.homeostasis,
            candidates,
            self.profile_provenance,
            self.grounded_object_slots,
            self.orientation_repertoire,
            self.grip_proprioception,
            self.candidate_opportunity_coverage,
        )
    }

    /// Attach private, bounded semantic input before recall/finalization. Ordinary
    /// sight, hearing, candidates and world object slots remain unchanged.
    pub fn with_semantic_context(
        mut self,
        context: Option<crate::SemanticContextRef>,
    ) -> Result<Self, ScaffoldContractError> {
        self.sensory.semantic_context = context;
        Self::new_with_orientation_repertoire(
            self.organism_id,
            self.tick,
            self.sensor_profile,
            self.sensory,
            self.body,
            self.homeostasis,
            self.candidates,
            self.profile_provenance,
            self.grounded_object_slots,
            self.orientation_repertoire,
            self.grip_proprioception,
            self.candidate_opportunity_coverage,
        )
    }

    pub fn with_remembered_novelty(mut self, novelty: f32) -> Result<Self, ScaffoldContractError> {
        self.sensory.channels.novelty_signal = crate::NormalizedScalar::new(novelty)?;
        Self::new_with_orientation_repertoire(
            self.organism_id,
            self.tick,
            self.sensor_profile,
            self.sensory,
            self.body,
            self.homeostasis,
            self.candidates,
            self.profile_provenance,
            self.grounded_object_slots,
            self.orientation_repertoire,
            self.grip_proprioception,
            self.candidate_opportunity_coverage,
        )
    }
}

impl Validate for PerceptionFrameDraft {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_frame_base(
            self.schema_version,
            self.organism_id,
            self.tick,
            self.sensor_profile,
            &self.sensory,
            self.body,
            &self.homeostasis,
            &self.candidates,
            self.profile_provenance,
            &self.grounded_object_slots,
            self.orientation_repertoire,
            self.grip_proprioception,
            self.candidate_opportunity_coverage.as_ref(),
        )?;
        let expected = compute_base_digest(
            self.schema_version,
            self.organism_id,
            self.tick,
            self.sensor_profile,
            &self.sensory,
            self.body,
            &self.homeostasis,
            &self.candidates,
            self.profile_provenance,
            &self.grounded_object_slots,
            self.orientation_repertoire,
            self.grip_proprioception,
            self.candidate_opportunity_coverage.as_ref(),
        );
        if expected == self.base_digest {
            Ok(())
        } else {
            Err(ScaffoldContractError::InvalidPerceptionFrame)
        }
    }
}

impl<'de> Deserialize<'de> for PerceptionFrameDraft {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            schema_version: u16,
            organism_id: OrganismId,
            tick: Tick,
            sensor_profile: SensorProfile,
            sensory: SensorySnapshot,
            body: BodySnapshot,
            homeostasis: HomeostaticSnapshot,
            candidates: Vec<ActionCandidate>,
            profile_provenance: SensorProfileProvenance,
            grounded_object_slots: Vec<GroundedObjectSlotV1>,
            #[serde(default)]
            orientation_repertoire: Option<OrientationRepertoireProvenance>,
            #[serde(default)]
            grip_proprioception: Option<GripProprioceptionV1>,
            #[serde(default)]
            candidate_opportunity_coverage: Option<CandidateOpportunityCoverageV1>,
            base_digest: PerceptionBaseDigest,
        }

        let wire = Wire::deserialize(deserializer)?;
        validate_frame_base(
            wire.schema_version,
            wire.organism_id,
            wire.tick,
            wire.sensor_profile,
            &wire.sensory,
            wire.body,
            &wire.homeostasis,
            &wire.candidates,
            wire.profile_provenance,
            &wire.grounded_object_slots,
            wire.orientation_repertoire,
            wire.grip_proprioception,
            wire.candidate_opportunity_coverage.as_ref(),
        )
        .map_err(D::Error::custom)?;
        let expected = compute_base_digest(
            wire.schema_version,
            wire.organism_id,
            wire.tick,
            wire.sensor_profile,
            &wire.sensory,
            wire.body,
            &wire.homeostasis,
            &wire.candidates,
            wire.profile_provenance,
            &wire.grounded_object_slots,
            wire.orientation_repertoire,
            wire.grip_proprioception,
            wire.candidate_opportunity_coverage.as_ref(),
        );
        if expected != wire.base_digest {
            return Err(D::Error::custom("perception base digest mismatch"));
        }
        Ok(Self {
            schema_version: wire.schema_version,
            organism_id: wire.organism_id,
            tick: wire.tick,
            sensor_profile: wire.sensor_profile,
            sensory: wire.sensory,
            body: wire.body,
            homeostasis: wire.homeostasis,
            candidates: wire.candidates,
            profile_provenance: wire.profile_provenance,
            grounded_object_slots: wire.grounded_object_slots,
            orientation_repertoire: wire.orientation_repertoire,
            grip_proprioception: wire.grip_proprioception,
            candidate_opportunity_coverage: wire.candidate_opportunity_coverage,
            base_digest: wire.base_digest,
        })
    }
}

impl PerceptionContextBlock {
    pub fn empty() -> Self {
        Self::try_new(
            SchemaVersions::CURRENT.perception.raw(),
            PerceptionContextKind::None,
            Vec::new(),
        )
        .expect("the versioned empty perception context is valid")
    }

    pub fn try_new(
        schema_version: u16,
        context_kind: PerceptionContextKind,
        values: Vec<f32>,
    ) -> Result<Self, ScaffoldContractError> {
        validate_context(schema_version, context_kind, &values)?;
        let canonical_digest = compute_context_digest(schema_version, context_kind, &values);
        Ok(Self {
            schema_version,
            context_kind,
            values,
            canonical_digest,
        })
    }

    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    pub const fn context_kind(&self) -> PerceptionContextKind {
        self.context_kind
    }

    pub const fn canonical_digest(&self) -> PerceptionContextDigest {
        self.canonical_digest
    }

    pub fn values(&self) -> &[f32] {
        &self.values
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.validate_contract()
    }
}

impl Validate for PerceptionContextBlock {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_context(self.schema_version, self.context_kind, &self.values)?;
        if compute_context_digest(self.schema_version, self.context_kind, &self.values)
            == self.canonical_digest
        {
            Ok(())
        } else {
            Err(ScaffoldContractError::InvalidPerceptionFrame)
        }
    }
}

impl<'de> Deserialize<'de> for PerceptionContextBlock {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            schema_version: u16,
            context_kind: PerceptionContextKind,
            values: Vec<f32>,
            canonical_digest: PerceptionContextDigest,
        }

        let wire = Wire::deserialize(deserializer)?;
        validate_context(wire.schema_version, wire.context_kind, &wire.values)
            .map_err(D::Error::custom)?;
        let expected = compute_context_digest(wire.schema_version, wire.context_kind, &wire.values);
        if expected != wire.canonical_digest {
            return Err(D::Error::custom("perception context digest mismatch"));
        }
        Ok(Self {
            schema_version: wire.schema_version,
            context_kind: wire.context_kind,
            values: wire.values,
            canonical_digest: wire.canonical_digest,
        })
    }
}

impl PerceptionFrame {
    pub fn candidate_opportunity_coverage(&self) -> Option<&CandidateOpportunityCoverageV1> {
        self.base.candidate_opportunity_coverage()
    }
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        organism_id: OrganismId,
        tick: Tick,
        sensor_profile: SensorProfile,
        sensory: SensorySnapshot,
        body: BodySnapshot,
        homeostasis: HomeostaticSnapshot,
        candidates: Vec<ActionCandidate>,
        profile_provenance: SensorProfileProvenance,
        grounded_object_slots: Vec<GroundedObjectSlotV1>,
    ) -> Result<Self, ScaffoldContractError> {
        PerceptionFrameDraft::new(
            organism_id,
            tick,
            sensor_profile,
            sensory,
            body,
            homeostasis,
            candidates,
            profile_provenance,
            grounded_object_slots,
        )?
        .finalize(PerceptionContextBlock::empty())
    }

    pub const fn grip_proprioception(&self) -> Option<GripProprioceptionV1> {
        self.base.grip_proprioception()
    }

    pub const fn grounded_orientation_repertoire(&self) -> bool {
        self.base.grounded_orientation_repertoire()
    }

    pub const fn organism_id(&self) -> OrganismId {
        self.base.organism_id
    }

    pub const fn tick(&self) -> Tick {
        self.base.tick
    }

    pub const fn sensor_profile(&self) -> SensorProfile {
        self.base.sensor_profile
    }

    pub fn sensory(&self) -> &SensorySnapshot {
        &self.base.sensory
    }

    pub const fn body(&self) -> BodySnapshot {
        self.base.body
    }

    pub const fn homeostasis(&self) -> &HomeostaticSnapshot {
        &self.base.homeostasis
    }

    pub fn candidates(&self) -> &[ActionCandidate] {
        &self.base.candidates
    }

    pub const fn profile_provenance(&self) -> SensorProfileProvenance {
        self.base.profile_provenance
    }

    pub fn grounded_object_slots(&self) -> &[GroundedObjectSlotV1] {
        &self.base.grounded_object_slots
    }

    pub fn context(&self) -> &PerceptionContextBlock {
        &self.context
    }

    pub const fn base_digest(&self) -> PerceptionBaseDigest {
        self.base.base_digest
    }

    pub const fn frame_digest(&self) -> PerceptionFrameDigest {
        self.frame_digest
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.validate_contract()
    }
}

impl Validate for PerceptionFrame {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.base.validate_contract()?;
        self.context.validate_contract()?;
        if compute_frame_digest(self.base.base_digest, &self.context) == self.frame_digest {
            Ok(())
        } else {
            Err(ScaffoldContractError::InvalidPerceptionFrame)
        }
    }
}

impl<'de> Deserialize<'de> for PerceptionFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            base: PerceptionFrameDraft,
            context: PerceptionContextBlock,
            frame_digest: PerceptionFrameDigest,
        }

        let wire = Wire::deserialize(deserializer)?;
        let expected = compute_frame_digest(wire.base.base_digest, &wire.context);
        if expected != wire.frame_digest {
            return Err(D::Error::custom("perception frame digest mismatch"));
        }
        Ok(Self {
            base: wire.base,
            context: wire.context,
            frame_digest: wire.frame_digest,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NeuralActionSelection {
    pub candidate_index: u16,
    pub logit: f32,
    pub confidence: Confidence,
    pub active_tiles: u32,
    pub active_synapses: u32,
}

impl Validate for NeuralActionSelection {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if !self.logit.is_finite() {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Confidence::new(self.confidence.raw())
            .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_frame_base(
    schema_version: u16,
    organism_id: OrganismId,
    tick: Tick,
    sensor_profile: SensorProfile,
    sensory: &SensorySnapshot,
    body: BodySnapshot,
    homeostasis: &HomeostaticSnapshot,
    candidates: &[ActionCandidate],
    profile_provenance: SensorProfileProvenance,
    grounded_object_slots: &[GroundedObjectSlotV1],
    orientation_repertoire: Option<OrientationRepertoireProvenance>,
    grip_proprioception: Option<GripProprioceptionV1>,
    candidate_opportunity_coverage: Option<&CandidateOpportunityCoverageV1>,
) -> Result<(), ScaffoldContractError> {
    if let Some(receipt) = candidate_opportunity_coverage {
        receipt.validate_candidates(grounded_object_slots, candidates)?;
    }
    if let Some(receipt) = grip_proprioception {
        receipt.validate()?;
        if sensor_profile != SensorProfile::GroundedObjectSlotsV1
            || sensory.channels.tactile_contact[2].to_bits()
                != receipt.calibrated_output().to_bits()
        {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
    }
    if orientation_repertoire == Some(OrientationRepertoireProvenance::GroundedOrientationV3)
        && sensor_profile != SensorProfile::GroundedObjectSlotsV1
    {
        return Err(ScaffoldContractError::InvalidPerceptionFrame);
    }
    ensure_current_version(SchemaKind::Perception, schema_version)
        .map_err(|_| ScaffoldContractError::InvalidPerceptionFrame)?;
    organism_id
        .validate()
        .map_err(|_| ScaffoldContractError::InvalidPerceptionFrame)?;
    sensory
        .validate_contract()
        .map_err(|_| ScaffoldContractError::InvalidPerceptionFrame)?;
    body.validate_contract()
        .map_err(|_| ScaffoldContractError::InvalidPerceptionFrame)?;
    homeostasis
        .validate_contract()
        .map_err(|_| ScaffoldContractError::InvalidPerceptionFrame)?;
    profile_provenance
        .validate_contract()
        .map_err(|_| ScaffoldContractError::InvalidPerceptionFrame)?;
    if sensory.organism_id != organism_id
        || sensory.tick != tick
        || homeostasis.tick != tick
        || profile_provenance.profile != sensor_profile
        || profile_provenance.source_tick != tick
        || profile_provenance.sensory_abi_version != sensory.abi_version
        || candidates.is_empty()
        || candidates.len() > MAX_ACTION_CANDIDATES
        || grounded_object_slots.len() > MAX_GROUNDED_OBJECT_SLOTS
    {
        return Err(ScaffoldContractError::InvalidPerceptionFrame);
    }
    for (expected_index, slot) in grounded_object_slots.iter().enumerate() {
        slot.validate_contract()?;
        if usize::from(slot.slot_index) != expected_index {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
    }
    for (expected_index, candidate) in candidates.iter().enumerate() {
        candidate.validate_contract()?;
        if usize::from(candidate.candidate_index) != expected_index {
            return Err(ScaffoldContractError::InvalidActionCandidate);
        }
    }
    match sensor_profile {
        SensorProfile::PrivilegedAffordanceV1 => {
            if !grounded_object_slots.is_empty()
                || candidates
                    .iter()
                    .any(|candidate| candidate.observation != CandidateObservationRef::None)
            {
                return Err(ScaffoldContractError::InvalidPerceptionFrame);
            }
        }
        SensorProfile::GroundedObjectSlotsV1
        | SensorProfile::GroundedTerrainVisionV1
        | SensorProfile::GroundedTerrainObjectPortsV2 => {
            if (sensor_profile == SensorProfile::GroundedObjectSlotsV1
                && sensory
                    .channels
                    .visual_affordance
                    .iter()
                    .any(|value| *value != 0.0))
                || sensory.channels.nearby_affordances.raw() != 0
            {
                return Err(ScaffoldContractError::InvalidPerceptionFrame);
            }
            for candidate in candidates {
                let mut intrinsic_grounding = candidate.features;
                if candidate.observation == CandidateObservationRef::None
                    && intrinsic_grounding.0[CandidateFeatureVector::INTRINSIC_ACTION_BASIS_LANE]
                        == 1.0
                {
                    let basis =
                        CandidateFeatureVector::zero().with_intrinsic_action_basis(candidate.kind);
                    if intrinsic_grounding.0[21..] != basis.0[21..] {
                        return Err(ScaffoldContractError::InvalidPerceptionFrame);
                    }
                    // Validate the unchanged physical primitive separately from
                    // its exact, unscored action-opportunity descriptor.
                    intrinsic_grounding.0[21..].fill(0.0);
                }
                match (candidate.family, candidate.observation) {
                    (CandidateActionFamily::Idle, CandidateObservationRef::None) => {}
                    (CandidateActionFamily::Rest, CandidateObservationRef::None)
                        if candidate.kind == ActionKind::Rest
                            && candidate.target == ActionTarget::NONE
                            && intrinsic_grounding == CandidateFeatureVector::zero() => {}
                    (CandidateActionFamily::Other, CandidateObservationRef::None)
                        if candidate.kind == ActionKind::Vocalize
                            && candidate.target == ActionTarget::NONE
                            && intrinsic_grounding == CandidateFeatureVector::zero() => {}
                    (CandidateActionFamily::Contact, CandidateObservationRef::None)
                        if candidate.kind == ActionKind::Interact
                            && candidate.target == ActionTarget::NONE
                            && intrinsic_grounding == CandidateFeatureVector::zero() => {}
                    (CandidateActionFamily::Approach, CandidateObservationRef::None)
                        if candidate.kind == ActionKind::Move
                            && candidate.target == ActionTarget::NONE
                            && intrinsic_grounding == CandidateFeatureVector::zero() => {}
                    (CandidateActionFamily::Approach, CandidateObservationRef::None)
                        if (sensor_profile.has_terrain_vision()
                            || orientation_repertoire
                                == Some(
                                    OrientationRepertoireProvenance::GroundedOrientationV3,
                                ))
                            && candidate.kind == ActionKind::Move
                            && candidate.target == ActionTarget::NONE
                            && intrinsic_grounding.0[0] == 0.0
                            && intrinsic_grounding.0[1] == 1.0
                            && intrinsic_grounding.0[2..19]
                                .iter()
                                .all(|value| *value == 0.0)
                            && intrinsic_grounding.0[19] == 1.0
                            && intrinsic_grounding.0[20..]
                                .iter()
                                .all(|value| *value == 0.0) => {}
                    (CandidateActionFamily::Other, CandidateObservationRef::None)
                        if candidate.kind == ActionKind::Hold
                            && candidate.target == ActionTarget::NONE
                            && intrinsic_grounding == CandidateFeatureVector::zero() => {}
                    (CandidateActionFamily::Inspect, CandidateObservationRef::None)
                        if (sensor_profile.has_terrain_vision()
                            || orientation_repertoire
                                == Some(
                                    OrientationRepertoireProvenance::GroundedOrientationV3,
                                ))
                            && candidate.kind == ActionKind::Look
                            && candidate.target == ActionTarget::NONE
                            && intrinsic_grounding.0[18] == 1.0
                            && intrinsic_grounding.0[2..18]
                                .iter()
                                .all(|value| *value == 0.0)
                            && intrinsic_grounding.0[19..]
                                .iter()
                                .all(|value| *value == 0.0)
                            && matches!(
                                (intrinsic_grounding.0[0], intrinsic_grounding.0[1]),
                                (1.0, 0.0) | (-1.0, 0.0) | (0.0, 1.0) | (0.0, 0.0)
                            ) => {}
                    (CandidateActionFamily::Inspect, CandidateObservationRef::None)
                        if (sensor_profile.has_terrain_vision()
                            || orientation_repertoire
                                == Some(
                                    OrientationRepertoireProvenance::GroundedOrientationV3,
                                ))
                            && candidate.kind == ActionKind::Look
                            && candidate.target == ActionTarget::NONE
                            && intrinsic_grounding.0[2..19]
                                .iter()
                                .all(|value| *value == 0.0)
                            && intrinsic_grounding.0[19] == 1.0
                            && intrinsic_grounding.0[20..]
                                .iter()
                                .all(|value| *value == 0.0)
                            && matches!(
                                (intrinsic_grounding.0[0], intrinsic_grounding.0[1]),
                                (1.0, 0.0) | (-1.0, 0.0)
                            ) => {}
                    (CandidateActionFamily::Idle, CandidateObservationRef::ObjectSlot(_))
                    | (_, CandidateObservationRef::None) => {
                        return Err(ScaffoldContractError::InvalidPerceptionFrame);
                    }
                    (_, CandidateObservationRef::ObjectSlot(slot_index)) => {
                        let slot = grounded_object_slots
                            .get(usize::from(slot_index))
                            .ok_or(ScaffoldContractError::InvalidPerceptionFrame)?;
                        if candidate.features != slot.candidate_features()? {
                            return Err(ScaffoldContractError::InvalidPerceptionFrame);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_context(
    schema_version: u16,
    context_kind: PerceptionContextKind,
    values: &[f32],
) -> Result<(), ScaffoldContractError> {
    ensure_current_version(SchemaKind::Perception, schema_version)
        .map_err(|_| ScaffoldContractError::InvalidPerceptionFrame)?;
    match context_kind {
        PerceptionContextKind::None => {
            if !values.is_empty() {
                return Err(ScaffoldContractError::InvalidPerceptionFrame);
            }
        }
        PerceptionContextKind::EpisodicCandidateV1 => {
            let (rows, remainder) =
                values.as_chunks::<{ crate::MEMORY_CONTEXT_V1_LANES_PER_CANDIDATE }>();
            if rows.is_empty() || !remainder.is_empty() {
                return Err(ScaffoldContractError::InvalidPerceptionFrame);
            }
            for row in rows {
                if row[..14]
                    .iter()
                    .any(|value| !value.is_finite() || !(-1.0..=1.0).contains(value))
                    || row[14..]
                        .iter()
                        .any(|value| !value.is_finite() || !(0.0..=u16::MAX as f32).contains(value))
                {
                    return Err(ScaffoldContractError::InvalidPerceptionFrame);
                }
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn compute_base_digest(
    schema_version: u16,
    organism_id: OrganismId,
    tick: Tick,
    sensor_profile: SensorProfile,
    sensory: &SensorySnapshot,
    body: BodySnapshot,
    homeostasis: &HomeostaticSnapshot,
    candidates: &[ActionCandidate],
    profile_provenance: SensorProfileProvenance,
    grounded_object_slots: &[GroundedObjectSlotV1],
    orientation_repertoire: Option<OrientationRepertoireProvenance>,
    grip_proprioception: Option<GripProprioceptionV1>,
    candidate_opportunity_coverage: Option<&CandidateOpportunityCoverageV1>,
) -> PerceptionBaseDigest {
    let mut builder = CanonicalDigestBuilder::new(PERCEPTION_BASE_DOMAIN);
    builder.write_u16(schema_version);
    builder.write_u64(organism_id.raw());
    builder.write_u64(tick.raw());
    builder.write_u16(sensor_profile.raw());
    encode_profile_provenance(&mut builder, profile_provenance);
    encode_sensory_snapshot(&mut builder, sensory)
        .expect("validated sensory snapshots have canonical finite fields");
    encode_body_snapshot(&mut builder, body)
        .expect("validated body snapshots have canonical finite fields");
    encode_homeostatic_snapshot(&mut builder, homeostasis)
        .expect("validated homeostatic snapshots have canonical finite fields");
    builder.write_sequence_len(candidates.len());
    for candidate in candidates {
        encode_action_candidate(&mut builder, candidate)
            .expect("validated action candidates have canonical finite fields");
    }
    builder.write_sequence_len(grounded_object_slots.len());
    for slot in grounded_object_slots {
        encode_grounded_object_slot(&mut builder, slot)
            .expect("validated grounded object slots have canonical finite fields");
    }
    if orientation_repertoire == Some(OrientationRepertoireProvenance::GroundedOrientationV3) {
        builder.write_u64(0x6d6f746f725f7633);
        builder.write_u16(3);
    }
    // Absence writes nothing, preserving every legacy canonical digest.
    if let Some(receipt) = grip_proprioception {
        builder.write_u64(0x677269705f763031);
        builder.write_u64(receipt.inspected_objects);
        builder.write_u16(u16::from(receipt.held));
        builder
            .write_f32(receipt.touch_gain)
            .expect("validated grip calibration");
    }
    if let Some(receipt) = candidate_opportunity_coverage {
        builder.write_u64(0x6f7070636f763031);
        builder.write_u16(receipt.observed_slots);
        builder.write_u8(receipt.capacity);
        for slot in [receipt.contact_slot, receipt.focal_slot] {
            builder.write_bool(slot.is_some());
            if let Some(slot) = slot {
                builder.write_u16(slot);
            }
        }
        builder.write_u64(receipt.phase);
        builder.write_sequence_len(receipt.selected_slots.len());
        for &slot in &receipt.selected_slots {
            builder.write_u16(slot);
        }
        builder.write_u64(receipt.work_units);
    }
    PerceptionBaseDigest(builder.finish256())
}

fn encode_profile_provenance(
    builder: &mut CanonicalDigestBuilder,
    provenance: SensorProfileProvenance,
) {
    builder.write_u16(provenance.schema_version);
    builder.write_u16(provenance.profile.raw());
    builder.write_u16(provenance.sensory_abi_version.raw());
    builder.write_u64(provenance.source_tick.raw());
}

fn encode_grounded_object_slot(
    builder: &mut CanonicalDigestBuilder,
    slot: &GroundedObjectSlotV1,
) -> Result<(), ScaffoldContractError> {
    builder.write_u16(slot.slot_index);
    builder.write_u64(slot.tracked_object_id.raw());
    encode_f32_values(builder, &slot.bearing)?;
    builder.write_f32(slot.distance)?;
    encode_f32_values(builder, &slot.relative_velocity)?;
    encode_f32_values(builder, &slot.color)?;
    encode_f32_values(builder, &slot.material)?;
    encode_f32_values(builder, &slot.shape)?;
    encode_f32_values(builder, &slot.chemical)?;
    builder.write_f32(slot.contact)?;
    encode_f32_values(builder, &slot.proprioception)?;
    builder.write_f32(slot.temperature)?;
    encode_f32_values(builder, &slot.terrain)?;
    builder.write_f32(slot.confidence.raw())?;
    Ok(())
}

fn compute_context_digest(
    schema_version: u16,
    context_kind: PerceptionContextKind,
    values: &[f32],
) -> PerceptionContextDigest {
    let mut builder = CanonicalDigestBuilder::new(PERCEPTION_CONTEXT_DOMAIN);
    encode_context_fields(&mut builder, schema_version, context_kind, values)
        .expect("validated perception contexts have canonical finite fields");
    PerceptionContextDigest(builder.finish256())
}

fn compute_frame_digest(
    base_digest: PerceptionBaseDigest,
    context: &PerceptionContextBlock,
) -> PerceptionFrameDigest {
    let mut builder = CanonicalDigestBuilder::new(PERCEPTION_FRAME_DOMAIN);
    encode_u64_words(&mut builder, &base_digest.0);
    encode_context_fields(
        &mut builder,
        context.schema_version,
        context.context_kind,
        &context.values,
    )
    .expect("validated perception contexts have canonical finite fields");
    PerceptionFrameDigest(builder.finish256())
}

fn encode_candidate_feature(
    builder: &mut CanonicalDigestBuilder,
    candidate: &ActionCandidate,
) -> Result<(), ScaffoldContractError> {
    builder.write_u8(candidate.kind.raw());
    builder.write_u8(candidate.family.raw());
    encode_candidate_observation(builder, candidate.observation);
    encode_f32_values(builder, &candidate.features.0)?;
    builder.write_f32(candidate.sensor_confidence.raw())?;
    builder.write_f32(candidate.required_effort.raw())?;
    builder.write_u32(candidate.min_duration.raw());
    builder.write_u32(candidate.max_duration.raw());
    Ok(())
}

fn encode_action_candidate(
    builder: &mut CanonicalDigestBuilder,
    candidate: &ActionCandidate,
) -> Result<(), ScaffoldContractError> {
    builder.write_u16(candidate.candidate_index);
    builder.write_u32(candidate.action_id.raw());
    builder.write_u8(candidate.kind.raw());
    builder.write_u8(candidate.family.raw());
    encode_candidate_observation(builder, candidate.observation);
    encode_action_target(builder, candidate.target)?;
    encode_f32_values(builder, &candidate.features.0)?;
    builder.write_f32(candidate.sensor_confidence.raw())?;
    builder.write_f32(candidate.required_effort.raw())?;
    builder.write_u32(candidate.min_duration.raw());
    builder.write_u32(candidate.max_duration.raw());
    Ok(())
}

fn encode_candidate_observation(
    builder: &mut CanonicalDigestBuilder,
    observation: CandidateObservationRef,
) {
    match observation {
        CandidateObservationRef::None => builder.write_none(),
        CandidateObservationRef::ObjectSlot(slot) => {
            builder.write_some();
            builder.write_u16(slot);
        }
    }
}

fn encode_action_target(
    builder: &mut CanonicalDigestBuilder,
    target: ActionTarget,
) -> Result<(), ScaffoldContractError> {
    encode_optional_world_entity(builder, target.entity);
    encode_optional_vec3(builder, target.position)
}

fn encode_sensory_snapshot(
    builder: &mut CanonicalDigestBuilder,
    sensory: &SensorySnapshot,
) -> Result<(), ScaffoldContractError> {
    builder.write_u16(sensory.abi_version.raw());
    builder.write_u64(sensory.organism_id.raw());
    builder.write_u64(sensory.tick.raw());
    encode_vec3(builder, sensory.observer_position)?;
    encode_sensory_channels(builder, &sensory.channels)?;
    encode_context_streams(builder, &sensory.context_streams)?;
    encode_social_context(builder, &sensory.social_context)?;
    encode_language_context(builder, &sensory.language_context)?;
    encode_optional_semantic_context(builder, sensory.semantic_context.as_ref())?;
    encode_optional_gaussian_context(builder, sensory.gaussian_context.as_ref())?;
    Ok(())
}

fn encode_sensory_channels(
    builder: &mut CanonicalDigestBuilder,
    channels: &SensoryChannels,
) -> Result<(), ScaffoldContractError> {
    encode_f32_values(builder, &channels.visual_affordance)?;
    encode_f32_values(builder, &channels.auditory_acoustic)?;
    encode_f32_values(builder, &channels.smell_chemistry)?;
    encode_f32_values(builder, &channels.tactile_contact)?;
    builder.write_f32(channels.pain_signal.raw())?;
    builder.write_f32(channels.novelty_signal.raw())?;
    builder.write_u32(channels.nearby_affordances.raw());
    Ok(())
}

fn encode_context_streams(
    builder: &mut CanonicalDigestBuilder,
    streams: &ContextStreams,
) -> Result<(), ScaffoldContractError> {
    builder.write_u16(streams.abi_version.raw());
    builder.write_f32(streams.atmospheric_temperature_celsius)?;
    builder.write_f32(streams.ambient_light.raw())?;
    builder.write_f32(streams.energy_intake_trend.raw())?;
    builder.write_f32(streams.blood_sugar_trend.raw())?;

    builder.write_sequence_len(streams.vocal_tokens.len());
    for token in &streams.vocal_tokens {
        encode_optional_heard_token(builder, token.as_ref())?;
    }
    builder.write_sequence_len(streams.social_proximity.len());
    for entry in &streams.social_proximity {
        encode_optional_social_proximity(builder, entry.as_ref())?;
    }
    builder.write_sequence_len(streams.optional_environment.len());
    for entry in &streams.optional_environment {
        encode_optional_environment_entry(builder, entry.as_ref())?;
    }
    Ok(())
}

fn encode_optional_heard_token(
    builder: &mut CanonicalDigestBuilder,
    token: Option<&HeardToken>,
) -> Result<(), ScaffoldContractError> {
    let Some(token) = token else {
        builder.write_none();
        return Ok(());
    };
    builder.write_some();
    builder.write_u64(token.utterance_id.raw());
    builder.write_u8(token.sequence_position);
    builder.write_u8(token.source_kind as u8);
    encode_optional_organism(builder, token.speaker_id);
    encode_optional_organism(builder, token.addressee);
    encode_optional_world_entity(builder, token.source_entity);
    builder.write_u32(token.token_id);
    encode_vec3(builder, token.source_position)?;
    builder.write_f32(token.confidence.raw())?;
    encode_optional_teacher_channel(builder, token.teacher_channel);
    Ok(())
}

fn encode_optional_social_proximity(
    builder: &mut CanonicalDigestBuilder,
    entry: Option<&SocialProximityEntry>,
) -> Result<(), ScaffoldContractError> {
    let Some(entry) = entry else {
        builder.write_none();
        return Ok(());
    };
    builder.write_some();
    builder.write_u64(entry.agent_id.raw());
    builder.write_f32(entry.proximity.raw())?;
    builder.write_f32(entry.confidence.raw())?;
    Ok(())
}

fn encode_optional_environment_entry(
    builder: &mut CanonicalDigestBuilder,
    entry: Option<&EnvironmentStreamEntry>,
) -> Result<(), ScaffoldContractError> {
    let Some(entry) = entry else {
        builder.write_none();
        return Ok(());
    };
    builder.write_some();
    builder.write_u16(entry.stream_id);
    builder.write_f32(entry.value.raw())?;
    builder.write_f32(entry.confidence.raw())?;
    Ok(())
}

fn encode_social_context(
    builder: &mut CanonicalDigestBuilder,
    context: &SocialContextSnapshot,
) -> Result<(), ScaffoldContractError> {
    builder.write_sequence_len(context.nearest_agents.len());
    for agent in &context.nearest_agents {
        encode_optional_social_agent(builder, agent.as_ref())?;
    }
    Ok(())
}

fn encode_optional_social_agent(
    builder: &mut CanonicalDigestBuilder,
    agent: Option<&SocialAgentSnapshot>,
) -> Result<(), ScaffoldContractError> {
    let Some(agent) = agent else {
        builder.write_none();
        return Ok(());
    };
    builder.write_some();
    builder.write_u64(agent.agent_id.raw());
    encode_optional_world_entity(builder, agent.body_entity);
    encode_vec3(builder, agent.relative_position)?;
    encode_vec3(builder, agent.gaze_direction)?;
    encode_vec3(builder, agent.orientation_forward)?;
    builder.write_f32(agent.affinity.raw())?;
    builder.write_f32(agent.proximity.raw())?;
    Ok(())
}

fn encode_language_context(
    builder: &mut CanonicalDigestBuilder,
    context: &LanguageContextSnapshot,
) -> Result<(), ScaffoldContractError> {
    builder.write_sequence_len(context.heard_tokens.len());
    for token in &context.heard_tokens {
        encode_optional_heard_token(builder, token.as_ref())?;
    }
    encode_optional_vocalized_token(builder, context.vocalized_token.as_ref())?;
    builder.write_f32(context.word_confidence.raw())?;
    encode_optional_teacher_channel(builder, context.teacher_channel_marker);
    Ok(())
}

fn encode_optional_vocalized_token(
    builder: &mut CanonicalDigestBuilder,
    token: Option<&VocalizedToken>,
) -> Result<(), ScaffoldContractError> {
    let Some(token) = token else {
        builder.write_none();
        return Ok(());
    };
    builder.write_some();
    builder.write_u32(token.token_id);
    builder.write_f32(token.confidence.raw())?;
    Ok(())
}

fn encode_optional_semantic_context(
    builder: &mut CanonicalDigestBuilder,
    context: Option<&SemanticContextRef>,
) -> Result<(), ScaffoldContractError> {
    let Some(context) = context else {
        builder.write_none();
        return Ok(());
    };
    builder.write_some();
    builder.write_u32(context.feature_flags.raw());
    builder.write_f32(context.confidence.raw())?;
    builder.write_sequence_len(context.compressed_codes.len());
    for code in &context.compressed_codes {
        encode_compressed_semantic_code(builder, code)?;
    }
    builder.write_sequence_len(context.salience.len());
    for entry in &context.salience {
        encode_semantic_salience_entry(builder, entry)?;
    }
    Ok(())
}

fn encode_compressed_semantic_code(
    builder: &mut CanonicalDigestBuilder,
    code: &CompressedSemanticCode,
) -> Result<(), ScaffoldContractError> {
    builder.write_u16(code.codebook_id);
    builder.write_u32(code.code);
    builder.write_f32(code.salience.raw())?;
    Ok(())
}

fn encode_semantic_salience_entry(
    builder: &mut CanonicalDigestBuilder,
    entry: &SemanticSalienceEntry,
) -> Result<(), ScaffoldContractError> {
    builder.write_u64(entry.concept_id.raw());
    builder.write_f32(entry.salience.raw())?;
    Ok(())
}

fn encode_optional_gaussian_context(
    builder: &mut CanonicalDigestBuilder,
    context: Option<&GaussianContextRef>,
) -> Result<(), ScaffoldContractError> {
    let Some(context) = context else {
        builder.write_none();
        return Ok(());
    };
    builder.write_some();
    builder.write_u64(context.egocentric_bin_hash);
    builder.write_u32(context.feature_flags.raw());
    builder.write_f32(context.confidence.raw())?;
    builder.write_sequence_len(context.clusters.len());
    for cluster in &context.clusters {
        encode_gaussian_salience_entry(builder, cluster)?;
    }
    Ok(())
}

fn encode_gaussian_salience_entry(
    builder: &mut CanonicalDigestBuilder,
    cluster: &GaussianSalienceEntry,
) -> Result<(), ScaffoldContractError> {
    builder.write_u64(cluster.cluster_id.raw());
    builder.write_f32(cluster.salience.raw())?;
    builder.write_f32(cluster.distance_meters)?;
    Ok(())
}

fn encode_body_snapshot(
    builder: &mut CanonicalDigestBuilder,
    body: BodySnapshot,
) -> Result<(), ScaffoldContractError> {
    encode_pose(builder, body.pose)?;
    encode_velocity(builder, body.velocity)
}

fn encode_pose(
    builder: &mut CanonicalDigestBuilder,
    pose: Pose,
) -> Result<(), ScaffoldContractError> {
    encode_vec3(builder, pose.translation)?;
    encode_quat(builder, pose.rotation)
}

fn encode_velocity(
    builder: &mut CanonicalDigestBuilder,
    velocity: Velocity,
) -> Result<(), ScaffoldContractError> {
    encode_vec3(builder, velocity.linear)?;
    encode_vec3(builder, velocity.angular)
}

fn encode_homeostatic_snapshot(
    builder: &mut CanonicalDigestBuilder,
    homeostasis: &HomeostaticSnapshot,
) -> Result<(), ScaffoldContractError> {
    builder.write_u16(homeostasis.schema_version);
    builder.write_u64(homeostasis.tick.raw());
    encode_drive_snapshot(builder, &homeostasis.drives)?;
    encode_endocrine_snapshot(builder, &homeostasis.hormones)
}

fn encode_drive_snapshot(
    builder: &mut CanonicalDigestBuilder,
    drives: &DriveSnapshot,
) -> Result<(), ScaffoldContractError> {
    builder.write_f32(drives.hunger)?;
    builder.write_f32(drives.fatigue)?;
    builder.write_f32(drives.fear)?;
    builder.write_f32(drives.pain)?;
    builder.write_f32(drives.loneliness)?;
    builder.write_f32(drives.curiosity)?;
    builder.write_f32(drives.brain_atp)?;
    builder.write_f32(drives.temperature_stress)?;
    builder.write_f32(drives.reproductive_drive)?;
    encode_f32_values(builder, &drives.extension)
}

fn encode_endocrine_snapshot(
    builder: &mut CanonicalDigestBuilder,
    hormones: &EndocrineSnapshot,
) -> Result<(), ScaffoldContractError> {
    builder.write_f32(hormones.adrenaline)?;
    builder.write_f32(hormones.cortisol)?;
    builder.write_f32(hormones.dopamine)?;
    builder.write_f32(hormones.oxytocin)?;
    builder.write_f32(hormones.serotonin)?;
    builder.write_f32(hormones.acetylcholine)?;
    builder.write_f32(hormones.learning_modulator)?;
    builder.write_f32(hormones.developmental_hormone)?;
    builder.write_f32(hormones.sleep_pressure)?;
    encode_f32_values(builder, &hormones.extension)
}

fn encode_context_fields(
    builder: &mut CanonicalDigestBuilder,
    schema_version: u16,
    context_kind: PerceptionContextKind,
    values: &[f32],
) -> Result<(), ScaffoldContractError> {
    builder.write_u16(schema_version);
    builder.write_u16(context_kind.raw());
    encode_f32_values(builder, values)
}

fn encode_optional_organism(builder: &mut CanonicalDigestBuilder, organism: Option<OrganismId>) {
    match organism {
        Some(organism) => {
            builder.write_some();
            builder.write_u64(organism.raw());
        }
        None => builder.write_none(),
    }
}

fn encode_optional_world_entity(
    builder: &mut CanonicalDigestBuilder,
    entity: Option<crate::WorldEntityId>,
) {
    match entity {
        Some(entity) => {
            builder.write_some();
            builder.write_u64(entity.raw());
        }
        None => builder.write_none(),
    }
}

fn encode_optional_vec3(
    builder: &mut CanonicalDigestBuilder,
    value: Option<Vec3f>,
) -> Result<(), ScaffoldContractError> {
    let Some(value) = value else {
        builder.write_none();
        return Ok(());
    };
    builder.write_some();
    encode_vec3(builder, value)
}

fn encode_optional_teacher_channel(
    builder: &mut CanonicalDigestBuilder,
    channel: Option<TeacherPerceptionChannel>,
) {
    match channel {
        Some(channel) => {
            builder.write_some();
            builder.write_u8(channel.raw());
        }
        None => builder.write_none(),
    }
}

fn encode_vec3(
    builder: &mut CanonicalDigestBuilder,
    value: Vec3f,
) -> Result<(), ScaffoldContractError> {
    builder.write_f32(value.x)?;
    builder.write_f32(value.y)?;
    builder.write_f32(value.z)
}

fn encode_quat(
    builder: &mut CanonicalDigestBuilder,
    value: Quatf,
) -> Result<(), ScaffoldContractError> {
    builder.write_f32(value.x)?;
    builder.write_f32(value.y)?;
    builder.write_f32(value.z)?;
    builder.write_f32(value.w)
}

fn encode_f32_values(
    builder: &mut CanonicalDigestBuilder,
    values: &[f32],
) -> Result<(), ScaffoldContractError> {
    builder.write_sequence_len(values.len());
    for value in values {
        builder.write_f32(*value)?;
    }
    Ok(())
}

fn encode_u64_words(builder: &mut CanonicalDigestBuilder, values: &[u64]) {
    builder.write_sequence_len(values.len());
    for value in values {
        builder.write_u64(*value);
    }
}

#[cfg(test)]
mod grip_proprioception_tests {
    use super::*;
    fn fixture(output: f32) -> PerceptionFrameDraft {
        let owner = OrganismId(2306);
        let tick = Tick::new(1);
        let profile = SensorProfile::GroundedObjectSlotsV1;
        let mut channels = SensoryChannels::ZERO;
        channels.tactile_contact[2] = output;
        let candidate = ActionCandidate::new(
            0,
            ActionId(1),
            ActionKind::Idle,
            CandidateActionFamily::Idle,
            CandidateObservationRef::None,
            ActionTarget::NONE,
            CandidateFeatureVector::zero(),
            Confidence::new(1.0).unwrap(),
            NormalizedScalar::new(0.0).unwrap(),
            DurationTicks::new(1),
            DurationTicks::new(1),
        )
        .unwrap();
        PerceptionFrameDraft::new(
            owner,
            tick,
            profile,
            SensorySnapshot::new(owner, tick, Vec3f::ZERO, channels, Default::default()).unwrap(),
            BodySnapshot {
                pose: Pose::IDENTITY,
                velocity: Velocity::ZERO,
            },
            HomeostaticSnapshot::baseline(tick),
            vec![candidate],
            SensorProfileProvenance::new(profile, crate::SensoryAbiVersion::CURRENT, tick).unwrap(),
            vec![],
        )
        .unwrap()
    }
    #[test]
    fn grip_receipt_binds_count_and_survives_all_rebinding() {
        let r = GripProprioceptionV1::new(7, true, 0.5).unwrap();
        assert_eq!(r.work_units(), 10);
        let d = fixture(0.5).with_grip_proprioception(r).unwrap();
        for rebuilt in [
            d.clone().with_candidates(d.candidates().to_vec()).unwrap(),
            d.clone().with_semantic_context(None).unwrap(),
            d.clone().with_remembered_novelty(0.0).unwrap(),
        ] {
            assert_eq!(rebuilt.grip_proprioception(), Some(r));
            rebuilt.validate().unwrap();
        }
        let bytes = serde_json::to_vec(&d).unwrap();
        assert_eq!(
            serde_json::from_slice::<PerceptionFrameDraft>(&bytes).unwrap(),
            d
        );
        assert_eq!(
            d.clone()
                .finalize(PerceptionContextBlock::empty())
                .unwrap()
                .grip_proprioception(),
            Some(r)
        );
        let mut v = serde_json::to_value(&d).unwrap();
        v["grip_proprioception"]["inspected_objects"] = 8.into();
        assert!(serde_json::from_value::<PerceptionFrameDraft>(v).is_err());
        let mut v = serde_json::to_value(&d).unwrap();
        v["sensory"]["channels"]["tactile_contact"][2] = serde_json::json!(0.0);
        assert!(serde_json::from_value::<PerceptionFrameDraft>(v).is_err());

        assert_ne!(
            d.base_digest(),
            fixture(0.5)
                .with_grip_proprioception(GripProprioceptionV1::new(8, true, 0.5).unwrap())
                .unwrap()
                .base_digest()
        );
    }
    #[test]
    fn grip_receipt_checks_calibration_and_bounded_work() {
        assert!(fixture(0.0)
            .with_grip_proprioception(GripProprioceptionV1::new(1, true, 1.0).unwrap())
            .is_err());
        assert!(GripProprioceptionV1::new(0, true, 1.0).is_err());
        assert!(GripProprioceptionV1::new(u64::MAX, false, 1.0).is_err());
        assert!(GripProprioceptionV1::new(1, false, f32::NAN).is_err());
        assert_eq!(
            GripProprioceptionV1::new(3, true, 2.0)
                .unwrap()
                .calibrated_output(),
            1.0
        );
        assert_eq!(
            GripProprioceptionV1::new(3, true, 0.0)
                .unwrap()
                .calibrated_output(),
            0.0
        );
    }
    #[test]
    fn legacy_absence_does_not_add_wire_field_or_digest_data() {
        let d = fixture(0.0);
        let bytes = serde_json::to_vec(&d).unwrap();
        assert!(!std::str::from_utf8(&bytes)
            .unwrap()
            .contains("grip_proprioception"));
        assert_eq!(
            serde_json::to_vec(&serde_json::from_slice::<PerceptionFrameDraft>(&bytes).unwrap())
                .unwrap(),
            bytes
        );
        let mut v = serde_json::to_value(&d).unwrap();
        v["grip_proprioception"] = serde_json::Value::Null;
        assert_eq!(
            serde_json::from_value::<PerceptionFrameDraft>(v).unwrap(),
            d
        );
    }
}

#[cfg(test)]
mod candidate_opportunity_coverage_tests {
    use super::*;
    fn fixture() -> PerceptionFrameDraft {
        let owner = OrganismId(2313);
        let tick = Tick::new(1);
        let profile = SensorProfile::GroundedObjectSlotsV1;
        let slots = (0..2)
            .map(|i| GroundedObjectSlotV1 {
                slot_index: i,
                tracked_object_id: crate::TrackedObjectId(u64::from(i) + 1),
                bearing: [0.0, 1.0],
                distance: 0.2,
                relative_velocity: [0.; 3],
                color: [0.1; 3],
                material: [0.; 3],
                shape: [0.2; 3],
                chemical: [0.; 3],
                contact: 0.0,
                proprioception: [0.; 2],
                temperature: 0.0,
                terrain: [0.; 2],
                confidence: Confidence::new(1.0).unwrap(),
            })
            .collect::<Vec<_>>();
        let mut candidates = Vec::new();
        for slot in &slots {
            for (kind, family) in [
                (ActionKind::Inspect, CandidateActionFamily::Inspect),
                (ActionKind::Move, CandidateActionFamily::Approach),
                (ActionKind::Move, CandidateActionFamily::Avoid),
                (ActionKind::Interact, CandidateActionFamily::Ingest),
                (ActionKind::Interact, CandidateActionFamily::Contact),
                (ActionKind::Interact, CandidateActionFamily::Other),
            ] {
                candidates.push(
                    ActionCandidate::new(
                        candidates.len() as u16,
                        ActionId(100 + candidates.len() as u32),
                        kind,
                        family,
                        CandidateObservationRef::ObjectSlot(slot.slot_index),
                        ActionTarget::NONE,
                        slot.candidate_features().unwrap(),
                        Confidence::new(1.).unwrap(),
                        NormalizedScalar::new(0.).unwrap(),
                        DurationTicks::new(1),
                        DurationTicks::new(1),
                    )
                    .unwrap(),
                );
            }
        }
        let channels = SensoryChannels::ZERO;
        PerceptionFrameDraft::new(
            owner,
            tick,
            profile,
            SensorySnapshot::new(owner, tick, Vec3f::ZERO, channels, Default::default()).unwrap(),
            BodySnapshot {
                pose: Pose::IDENTITY,
                velocity: Velocity::ZERO,
            },
            HomeostaticSnapshot::baseline(tick),
            candidates,
            SensorProfileProvenance::new(profile, crate::SensoryAbiVersion::CURRENT, tick).unwrap(),
            slots,
        )
        .unwrap()
        .with_grip_proprioception(GripProprioceptionV1::new(3, false, 1.).unwrap())
        .unwrap()
    }
    fn receipt() -> CandidateOpportunityCoverageV1 {
        CandidateOpportunityCoverageV1::new(2, 2, Some(0), Some(1), 7, vec![0, 1], 11).unwrap()
    }
    fn reindex(candidates: &mut [ActionCandidate]) {
        for (i, c) in candidates.iter_mut().enumerate() {
            c.candidate_index = i as u16;
        }
    }
    #[test]
    fn coverage_validates_bounds_duplicates_protected_slots_and_work() {
        for (n, cap, contact, focal, selected, work) in [
            (17, 2, None, None, vec![0], 0),
            (2, 17, None, None, vec![0], 0),
            (2, 1, None, None, vec![0, 1], 0),
            (2, 2, None, None, vec![0, 0], 0),
            (2, 2, None, None, vec![2], 0),
            (2, 2, Some(1), None, vec![0], 0),
            (2, 2, None, Some(1), vec![0], 0),
            (2, 2, None, None, vec![0], 1_000_000_001),
        ] {
            assert!(
                CandidateOpportunityCoverageV1::new(n, cap, contact, focal, 0, selected, work)
                    .is_err()
            );
        }
        assert!(CandidateOpportunityCoverageV1::new(0, 0, None, None, 0, vec![], 0).is_ok());
        let mut v = serde_json::to_value(receipt()).unwrap();
        v["selected_slots"] = serde_json::json!([0, 0]);
        assert!(serde_json::from_value::<CandidateOpportunityCoverageV1>(v).is_err());
    }
    #[test]
    fn coverage_retained_through_reorder_semantics_grip_and_finalization() {
        let d = fixture().with_opportunity_coverage(receipt()).unwrap();
        let mut candidates = d.candidates().to_vec();
        candidates.reverse();
        reindex(&mut candidates);
        let reordered = d.clone().with_candidates(candidates).unwrap();
        for rebuilt in [
            reordered,
            d.clone().with_semantic_context(None).unwrap(),
            d.clone().with_remembered_novelty(0.2).unwrap(),
            d.clone()
                .with_grip_proprioception(d.grip_proprioception().unwrap())
                .unwrap(),
        ] {
            assert_eq!(rebuilt.candidate_opportunity_coverage(), Some(&receipt()));
            assert_eq!(rebuilt.grip_proprioception(), d.grip_proprioception());
            rebuilt.validate_contract().unwrap();
            let frame = rebuilt.finalize(PerceptionContextBlock::empty()).unwrap();
            assert_eq!(frame.candidate_opportunity_coverage(), Some(&receipt()));
            let bytes = serde_json::to_vec(&frame).unwrap();
            assert_eq!(
                serde_json::from_slice::<PerceptionFrame>(&bytes).unwrap(),
                frame
            );
        }
    }
    #[test]
    fn coverage_requires_exact_six_family_set_and_atomic_replacement() {
        let base = fixture();
        let d = base.clone().with_opportunity_coverage(receipt()).unwrap();
        let mut missing = d.candidates().to_vec();
        missing.remove(0);
        reindex(&mut missing);
        assert!(d.clone().with_candidates(missing).is_err());
        let mut duplicate = d.candidates().to_vec();
        duplicate[1] = duplicate[0];
        reindex(&mut duplicate);
        assert!(d.clone().with_candidates(duplicate).is_err());
        let mut one = d
            .candidates()
            .iter()
            .filter(|c| c.observation == CandidateObservationRef::ObjectSlot(0))
            .copied()
            .collect::<Vec<_>>();
        reindex(&mut one);
        assert!(d.clone().with_candidates(one.clone()).is_err());
        let new = CandidateOpportunityCoverageV1::new(2, 2, Some(0), None, 8, vec![0], 9).unwrap();
        let changed = d
            .with_candidates_and_opportunity_coverage(one, new.clone())
            .unwrap();
        assert_eq!(changed.candidate_opportunity_coverage(), Some(&new));
        assert_eq!(changed.grip_proprioception(), base.grip_proprioception());
        assert!(base
            .with_opportunity_coverage(
                CandidateOpportunityCoverageV1::new(1, 1, None, None, 0, vec![0], 0).unwrap()
            )
            .is_err());
    }
    #[test]
    fn coverage_digest_and_wire_reject_tampering_or_stripping() {
        let d = fixture().with_opportunity_coverage(receipt()).unwrap();
        for field in ["phase", "work_units", "capacity"] {
            let mut v = serde_json::to_value(&d).unwrap();
            v["candidate_opportunity_coverage"][field] = 13.into();
            assert!(serde_json::from_value::<PerceptionFrameDraft>(v).is_err());
        }
        let mut v = serde_json::to_value(&d).unwrap();
        v.as_object_mut()
            .unwrap()
            .remove("candidate_opportunity_coverage");
        assert!(serde_json::from_value::<PerceptionFrameDraft>(v).is_err());
        let mut v = serde_json::to_value(&d).unwrap();
        v["candidate_opportunity_coverage"]["selected_slots"] = serde_json::json!([1, 0]);
        assert!(serde_json::from_value::<PerceptionFrameDraft>(v).is_err());
        assert_ne!(d.base_digest(), fixture().base_digest());
    }
    #[test]
    fn coverage_absence_preserves_legacy_serialized_fields_and_digest() {
        let d = fixture();
        let bytes = serde_json::to_vec(&d).unwrap();
        assert!(!std::str::from_utf8(&bytes)
            .unwrap()
            .contains("candidate_opportunity_coverage"));
        let mut v = serde_json::to_value(&d).unwrap();
        v["candidate_opportunity_coverage"] = serde_json::Value::Null;
        let decoded = serde_json::from_value::<PerceptionFrameDraft>(v).unwrap();
        assert_eq!(decoded, d);
        assert_eq!(serde_json::to_vec(&decoded).unwrap(), bytes);
        assert_eq!(
            d.clone()
                .with_candidates(d.candidates().to_vec())
                .unwrap()
                .base_digest(),
            d.base_digest()
        );
        #[derive(Serialize)]
        struct Legacy<'a> {
            schema_version: u16,
            organism_id: OrganismId,
            tick: Tick,
            sensor_profile: SensorProfile,
            sensory: &'a SensorySnapshot,
            body: BodySnapshot,
            homeostasis: &'a HomeostaticSnapshot,
            candidates: &'a [ActionCandidate],
            profile_provenance: SensorProfileProvenance,
            grounded_object_slots: &'a [GroundedObjectSlotV1],
            #[serde(skip_serializing_if = "Option::is_none")]
            orientation_repertoire: Option<OrientationRepertoireProvenance>,
            #[serde(skip_serializing_if = "Option::is_none")]
            grip_proprioception: Option<GripProprioceptionV1>,
            base_digest: PerceptionBaseDigest,
        }
        let old = Legacy {
            schema_version: d.schema_version,
            organism_id: d.organism_id,
            tick: d.tick,
            sensor_profile: d.sensor_profile,
            sensory: &d.sensory,
            body: d.body,
            homeostasis: &d.homeostasis,
            candidates: &d.candidates,
            profile_provenance: d.profile_provenance,
            grounded_object_slots: &d.grounded_object_slots,
            orientation_repertoire: d.orientation_repertoire,
            grip_proprioception: d.grip_proprioception,
            base_digest: d.base_digest,
        };
        assert_eq!(serde_json::to_vec(&old).unwrap(), bytes);
        assert_eq!(
            bincode::serde::encode_to_vec(&old, bincode::config::standard()).unwrap(),
            bincode::serde::encode_to_vec(&d, bincode::config::standard()).unwrap()
        );
    }
}
