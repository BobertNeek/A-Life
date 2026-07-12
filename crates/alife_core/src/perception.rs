//! Contract-only same-tick perception and unscored action-candidate records.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    ensure_current_version, ActionCommand, ActionId, ActionKind, ActionTarget,
    CanonicalDigestBuilder, CompressedSemanticCode, Confidence, ContextStreams, DriveSnapshot,
    DurationTicks, EndocrineSnapshot, EnvironmentStreamEntry, GaussianContextRef,
    GaussianSalienceEntry, HeardToken, HomeostaticSnapshot, Intensity, LanguageContextSnapshot,
    NormalizedScalar, OrganismId, Pose, Quatf, ScaffoldContractError, SchemaKind, SchemaVersions,
    SemanticContextRef, SemanticSalienceEntry, SensoryChannels, SensorySnapshot,
    SocialAgentSnapshot, SocialContextSnapshot, SocialProximityEntry, TeacherPerceptionChannel,
    Tick, Validate, Vec3f, Velocity, VocalizedToken,
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

    pub const fn homeostasis(&self) -> &HomeostaticSnapshot {
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
    PerceptionContextKind::try_from_raw(context_kind.raw())?;
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
    let mut builder = CanonicalDigestBuilder::new(PERCEPTION_BASE_DOMAIN);
    builder.write_u16(schema_version);
    builder.write_u64(organism_id.raw());
    builder.write_u64(tick.raw());
    builder.write_u16(sensor_profile.raw());
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
    PerceptionBaseDigest(builder.finish256())
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
    encode_optional_organism(builder, token.speaker_id);
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
