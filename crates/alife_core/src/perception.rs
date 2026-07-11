//! Contract-only same-tick perception and unscored action-candidate records.

use core::fmt;

use serde::de::Error as _;
use serde::ser::{
    Error as _, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant,
    SerializeTuple, SerializeTupleStruct, SerializeTupleVariant,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    ensure_current_version, ActionCommand, ActionId, ActionKind, ActionTarget, Confidence,
    DurationTicks, HomeostaticSnapshot, Intensity, NormalizedScalar, OrganismId, Pose,
    ScaffoldContractError, SchemaKind, SchemaVersions, SensorySnapshot, Tick, Validate, Velocity,
};

pub const CANDIDATE_FEATURE_COUNT: usize = 24;
pub const MAX_ACTION_CANDIDATES: usize = 32;

const PERCEPTION_BASE_DOMAIN: &[u8] = b"alife.perception.base.v1";
const PERCEPTION_CONTEXT_DOMAIN: &[u8] = b"alife.perception.context.v1";
const PERCEPTION_FRAME_DOMAIN: &[u8] = b"alife.perception.frame.v1";
const CANDIDATE_FEATURE_DOMAIN: &[u8] = b"alife.candidate.features.v1";

#[repr(u16)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SensorProfile {
    #[default]
    PrivilegedAffordanceV1 = 1,
    GroundedObjectSlotsV1 = 2,
}

impl SensorProfile {
    pub const fn raw(self) -> u16 {
        self as u16
    }

    pub fn try_from_raw(raw: u16) -> Result<Self, ScaffoldContractError> {
        match raw {
            1 => Ok(Self::PrivilegedAffordanceV1),
            2 => Ok(Self::GroundedObjectSlotsV1),
            _ => Err(ScaffoldContractError::SensorProfileMismatch),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PolicyBackend {
    #[default]
    NeuralClosedLoopGpu,
    HeuristicBaseline,
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
                | (Self::Inspect, ActionKind::Inspect)
                | (Self::Approach | Self::Avoid, ActionKind::Move)
                | (Self::Contact | Self::Ingest, ActionKind::Interact)
                | (
                    Self::Other,
                    ActionKind::Hold
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
            ActionKind::Inspect => Self::Inspect,
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
    pub const fn zero() -> Self {
        Self([0.0; CANDIDATE_FEATURE_COUNT])
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

    pub fn feature_digest(self) -> CandidateFeatureDigest {
        #[derive(Serialize)]
        struct CandidateFeatureCanonical {
            kind: ActionKind,
            family: CandidateActionFamily,
            observation: CandidateObservationRef,
            features: CandidateFeatureVector,
            sensor_confidence: Confidence,
            required_effort: NormalizedScalar,
            min_duration: DurationTicks,
            max_duration: DurationTicks,
        }

        let canonical = CandidateFeatureCanonical {
            kind: self.kind,
            family: self.family,
            observation: self.observation,
            features: self.features,
            sensor_confidence: self.sensor_confidence,
            required_effort: self.required_effort,
            min_duration: self.min_duration,
            max_duration: self.max_duration,
        };
        CandidateFeatureDigest(hash_canonical::<2, _>(CANDIDATE_FEATURE_DOMAIN, &canonical))
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
    pub fn new(
        organism_id: OrganismId,
        tick: Tick,
        sensor_profile: SensorProfile,
        sensory: SensorySnapshot,
        body: BodySnapshot,
        homeostasis: HomeostaticSnapshot,
        candidates: Vec<ActionCandidate>,
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
            base_digest,
        })
    }

    pub const fn schema_version(&self) -> u16 {
        self.schema_version
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
        self.validate_contract()?;
        context.validate_contract()?;
        let frame_digest = compute_frame_digest(self.base_digest, &context);
        Ok(PerceptionFrame {
            base: self,
            context,
            frame_digest,
        })
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
    pub fn new(
        organism_id: OrganismId,
        tick: Tick,
        sensor_profile: SensorProfile,
        sensory: SensorySnapshot,
        body: BodySnapshot,
        homeostasis: HomeostaticSnapshot,
        candidates: Vec<ActionCandidate>,
    ) -> Result<Self, ScaffoldContractError> {
        PerceptionFrameDraft::new(
            organism_id,
            tick,
            sensor_profile,
            sensory,
            body,
            homeostasis,
            candidates,
        )?
        .finalize(PerceptionContextBlock::empty())
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

    pub fn homeostasis(&self) -> &HomeostaticSnapshot {
        &self.base.homeostasis
    }

    pub fn candidates(&self) -> &[ActionCandidate] {
        &self.base.candidates
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
) -> Result<(), ScaffoldContractError> {
    ensure_current_version(SchemaKind::Perception, schema_version)
        .map_err(|_| ScaffoldContractError::InvalidPerceptionFrame)?;
    SensorProfile::try_from_raw(sensor_profile.raw())?;
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
    if sensory.organism_id != organism_id
        || sensory.tick != tick
        || homeostasis.tick != tick
        || candidates.is_empty()
        || candidates.len() > MAX_ACTION_CANDIDATES
    {
        return Err(ScaffoldContractError::InvalidPerceptionFrame);
    }
    for (expected_index, candidate) in candidates.iter().enumerate() {
        candidate.validate_contract()?;
        if usize::from(candidate.candidate_index) != expected_index {
            return Err(ScaffoldContractError::InvalidActionCandidate);
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
    if context_kind == PerceptionContextKind::None && !values.is_empty() {
        return Err(ScaffoldContractError::InvalidPerceptionFrame);
    }
    if values
        .iter()
        .any(|value| !value.is_finite() || !(-1.0..=1.0).contains(value))
    {
        return Err(ScaffoldContractError::InvalidPerceptionFrame);
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
) -> PerceptionBaseDigest {
    #[derive(Serialize)]
    struct Canonical<'a> {
        schema_version: u16,
        organism_id: OrganismId,
        tick: Tick,
        sensor_profile: SensorProfile,
        sensory: &'a SensorySnapshot,
        body: BodySnapshot,
        homeostasis: &'a HomeostaticSnapshot,
        candidates: &'a [ActionCandidate],
    }

    PerceptionBaseDigest(hash_canonical::<4, _>(
        PERCEPTION_BASE_DOMAIN,
        &Canonical {
            schema_version,
            organism_id,
            tick,
            sensor_profile,
            sensory,
            body,
            homeostasis,
            candidates,
        },
    ))
}

fn compute_context_digest(
    schema_version: u16,
    context_kind: PerceptionContextKind,
    values: &[f32],
) -> PerceptionContextDigest {
    #[derive(Serialize)]
    struct Canonical<'a> {
        schema_version: u16,
        context_kind: PerceptionContextKind,
        values: &'a [f32],
    }

    PerceptionContextDigest(hash_canonical::<4, _>(
        PERCEPTION_CONTEXT_DOMAIN,
        &Canonical {
            schema_version,
            context_kind,
            values,
        },
    ))
}

fn compute_frame_digest(
    base_digest: PerceptionBaseDigest,
    context: &PerceptionContextBlock,
) -> PerceptionFrameDigest {
    #[derive(Serialize)]
    struct Canonical<'a> {
        base_digest: PerceptionBaseDigest,
        context_schema_version: u16,
        context_kind: PerceptionContextKind,
        context_values: &'a [f32],
    }

    PerceptionFrameDigest(hash_canonical::<4, _>(
        PERCEPTION_FRAME_DOMAIN,
        &Canonical {
            base_digest,
            context_schema_version: context.schema_version,
            context_kind: context.context_kind,
            context_values: &context.values,
        },
    ))
}

fn hash_canonical<const N: usize, T: Serialize + ?Sized>(domain: &[u8], value: &T) -> [u64; N] {
    let mut encoder = CanonicalEncoder::default();
    value
        .serialize(&mut encoder)
        .expect("validated core contracts have canonical serde representations");
    core::array::from_fn(|lane| {
        let mut hash = 0xcbf2_9ce4_8422_2325u64 ^ (lane as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        for byte in domain.iter().chain(encoder.bytes.iter()) {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            hash ^= hash.rotate_right(29);
        }
        hash ^ ((domain.len() as u64) << 32) ^ encoder.bytes.len() as u64
    })
}

#[derive(Default)]
struct CanonicalEncoder {
    bytes: Vec<u8>,
}

#[derive(Debug)]
struct CanonicalEncodeError(String);

impl fmt::Display for CanonicalEncodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for CanonicalEncodeError {}

impl serde::ser::Error for CanonicalEncodeError {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self(message.to_string())
    }
}

struct CanonicalCompound<'a> {
    encoder: &'a mut CanonicalEncoder,
}

impl CanonicalEncoder {
    fn tag(&mut self, tag: u8) {
        self.bytes.push(tag);
    }

    fn len(&mut self, len: usize) -> Result<(), CanonicalEncodeError> {
        let len = u64::try_from(len)
            .map_err(|_| CanonicalEncodeError("canonical length overflow".to_string()))?;
        self.bytes.extend_from_slice(&len.to_le_bytes());
        Ok(())
    }

    fn name(&mut self, name: &str) -> Result<(), CanonicalEncodeError> {
        self.len(name.len())?;
        self.bytes.extend_from_slice(name.as_bytes());
        Ok(())
    }
}

impl<'a> Serializer for &'a mut CanonicalEncoder {
    type Ok = ();
    type Error = CanonicalEncodeError;
    type SerializeSeq = CanonicalCompound<'a>;
    type SerializeTuple = CanonicalCompound<'a>;
    type SerializeTupleStruct = CanonicalCompound<'a>;
    type SerializeTupleVariant = CanonicalCompound<'a>;
    type SerializeMap = CanonicalCompound<'a>;
    type SerializeStruct = CanonicalCompound<'a>;
    type SerializeStructVariant = CanonicalCompound<'a>;

    fn serialize_bool(self, value: bool) -> Result<Self::Ok, Self::Error> {
        self.tag(1);
        self.bytes.push(u8::from(value));
        Ok(())
    }

    fn serialize_i8(self, value: i8) -> Result<Self::Ok, Self::Error> {
        self.tag(2);
        self.bytes.push(value as u8);
        Ok(())
    }

    fn serialize_i16(self, value: i16) -> Result<Self::Ok, Self::Error> {
        self.tag(3);
        self.bytes.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn serialize_i32(self, value: i32) -> Result<Self::Ok, Self::Error> {
        self.tag(4);
        self.bytes.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn serialize_i64(self, value: i64) -> Result<Self::Ok, Self::Error> {
        self.tag(5);
        self.bytes.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn serialize_i128(self, value: i128) -> Result<Self::Ok, Self::Error> {
        self.tag(6);
        self.bytes.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn serialize_u8(self, value: u8) -> Result<Self::Ok, Self::Error> {
        self.tag(7);
        self.bytes.push(value);
        Ok(())
    }

    fn serialize_u16(self, value: u16) -> Result<Self::Ok, Self::Error> {
        self.tag(8);
        self.bytes.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn serialize_u32(self, value: u32) -> Result<Self::Ok, Self::Error> {
        self.tag(9);
        self.bytes.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn serialize_u64(self, value: u64) -> Result<Self::Ok, Self::Error> {
        self.tag(10);
        self.bytes.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn serialize_u128(self, value: u128) -> Result<Self::Ok, Self::Error> {
        self.tag(11);
        self.bytes.extend_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn serialize_f32(self, value: f32) -> Result<Self::Ok, Self::Error> {
        self.tag(12);
        self.bytes.extend_from_slice(&value.to_bits().to_le_bytes());
        Ok(())
    }

    fn serialize_f64(self, value: f64) -> Result<Self::Ok, Self::Error> {
        self.tag(13);
        self.bytes.extend_from_slice(&value.to_bits().to_le_bytes());
        Ok(())
    }

    fn serialize_char(self, value: char) -> Result<Self::Ok, Self::Error> {
        self.tag(14);
        self.bytes
            .extend_from_slice(&u32::from(value).to_le_bytes());
        Ok(())
    }

    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        self.tag(15);
        self.name(value)
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.tag(16);
        self.len(value.len())?;
        self.bytes.extend_from_slice(value);
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.tag(17);
        Ok(())
    }

    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        self.tag(18);
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.tag(19);
        Ok(())
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.tag(20);
        self.name(name)
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.tag(21);
        self.name(name)?;
        self.bytes.extend_from_slice(&variant_index.to_le_bytes());
        self.name(variant)
    }

    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        self.tag(22);
        self.name(name)?;
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        self.tag(23);
        self.name(name)?;
        self.bytes.extend_from_slice(&variant_index.to_le_bytes());
        self.name(variant)?;
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.tag(24);
        self.len(len.ok_or_else(|| Self::Error::custom("unknown sequence length"))?)?;
        Ok(CanonicalCompound { encoder: self })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.tag(25);
        self.len(len)?;
        Ok(CanonicalCompound { encoder: self })
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.tag(26);
        self.name(name)?;
        self.len(len)?;
        Ok(CanonicalCompound { encoder: self })
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.tag(27);
        self.name(name)?;
        self.bytes.extend_from_slice(&variant_index.to_le_bytes());
        self.name(variant)?;
        self.len(len)?;
        Ok(CanonicalCompound { encoder: self })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.tag(28);
        self.len(len.ok_or_else(|| Self::Error::custom("unknown map length"))?)?;
        Ok(CanonicalCompound { encoder: self })
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.tag(29);
        self.name(name)?;
        self.len(len)?;
        Ok(CanonicalCompound { encoder: self })
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.tag(30);
        self.name(name)?;
        self.bytes.extend_from_slice(&variant_index.to_le_bytes());
        self.name(variant)?;
        self.len(len)?;
        Ok(CanonicalCompound { encoder: self })
    }
}

impl SerializeSeq for CanonicalCompound<'_> {
    type Ok = ();
    type Error = CanonicalEncodeError;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(&mut *self.encoder)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl SerializeTuple for CanonicalCompound<'_> {
    type Ok = ();
    type Error = CanonicalEncodeError;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(&mut *self.encoder)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl SerializeTupleStruct for CanonicalCompound<'_> {
    type Ok = ();
    type Error = CanonicalEncodeError;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(&mut *self.encoder)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl SerializeTupleVariant for CanonicalCompound<'_> {
    type Ok = ();
    type Error = CanonicalEncodeError;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(&mut *self.encoder)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl SerializeMap for CanonicalCompound<'_> {
    type Ok = ();
    type Error = CanonicalEncodeError;

    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Self::Error> {
        key.serialize(&mut *self.encoder)
    }

    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(&mut *self.encoder)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl SerializeStruct for CanonicalCompound<'_> {
    type Ok = ();
    type Error = CanonicalEncodeError;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.encoder.name(key)?;
        value.serialize(&mut *self.encoder)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl SerializeStructVariant for CanonicalCompound<'_> {
    type Ok = ();
    type Error = CanonicalEncodeError;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.encoder.name(key)?;
        value.serialize(&mut *self.encoder)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}
