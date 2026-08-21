//! Stable, engine-neutral contracts consumed by the grounded predictor.
//!
//! The predictor must not infer meaning from a position in an arbitrary vector
//! or from the numeric distance between action identifiers.  This module keeps
//! those meanings explicit and gives pre-state, post-state, motor, and
//! outcome payloads one bounded validation surface.

use serde::{Deserialize, Serialize};

use crate::{
    ActionId, CanonicalDigestBuilder, MotorChannel, ScaffoldContractError, Validate, Vec3f,
    MAX_MOTOR_CHANNELS,
};

pub const SEMANTIC_STATE_VECTOR_SCHEMA_VERSION: u16 = 2;
pub const SEMANTIC_STATE_VECTOR_ABI_V1: u16 = 2;
pub const SEMANTIC_STATE_VECTOR_ABI_V2: u16 = SEMANTIC_STATE_VECTOR_ABI_V1;
pub const MAX_SEMANTIC_STATE_VALUES: usize = 32;
pub const MIN_SEMANTIC_STATE_VALUES: usize = 2;

pub const GROUNDED_OUTCOME_SCHEMA_VERSION: u16 = 1;
pub const GROUNDED_OUTCOME_ABI_V1: u16 = 1;
pub const GROUNDED_OUTCOME_FEATURE_COUNT: usize = 8;

pub const MOTOR_CATEGORY_SCHEMA_VERSION: u16 = 1;
pub const MOTOR_CATEGORY_ABI_V1: u16 = 1;
pub const MOTOR_PRIMITIVE_EMBEDDING_DIM: usize = 4;
pub const MAX_PRIMITIVE_EMBEDDINGS: usize = 64;

/// Stable meanings for the bounded semantic state lanes.
///
/// The state may use a prefix of this table, but both sides of a transition
/// must use the same prefix.  The table is append-only for this ABI.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SemanticStateMeaning {
    BodyPositionX,
    BodyPositionY,
    BodyPositionZ,
    BodyVelocityX,
    BodyVelocityY,
    BodyVelocityZ,
    DriveHunger,
    DriveThirst,
    DriveFatigue,
    DrivePain,
    HomeostasisEnergy,
    HomeostasisHealth,
    HomeostasisTemperature,
    HomeostasisInjury,
    AttentionPersistence,
    AttentionNovelty,
    InteroceptiveArousal,
    InteroceptiveUncertainty,
    FocalDistance,
    FocalRelativeX,
    FocalRelativeY,
    FocalRelativeZ,
    FocalMotion,
    FocalContact,
    SocialProximity,
    SocialMotion,
    MemoryEvidence,
    ConceptEvidence,
    GapEvidence,
    PredictionUncertainty,
    ReservedMeaning30,
    ReservedMeaning31,
}

pub const SEMANTIC_STATE_MEANINGS_V2: [SemanticStateMeaning; MAX_SEMANTIC_STATE_VALUES] = [
    SemanticStateMeaning::BodyPositionX,
    SemanticStateMeaning::BodyPositionY,
    SemanticStateMeaning::BodyPositionZ,
    SemanticStateMeaning::BodyVelocityX,
    SemanticStateMeaning::BodyVelocityY,
    SemanticStateMeaning::BodyVelocityZ,
    SemanticStateMeaning::DriveHunger,
    SemanticStateMeaning::DriveThirst,
    SemanticStateMeaning::DriveFatigue,
    SemanticStateMeaning::DrivePain,
    SemanticStateMeaning::HomeostasisEnergy,
    SemanticStateMeaning::HomeostasisHealth,
    SemanticStateMeaning::HomeostasisTemperature,
    SemanticStateMeaning::HomeostasisInjury,
    SemanticStateMeaning::AttentionPersistence,
    SemanticStateMeaning::AttentionNovelty,
    SemanticStateMeaning::InteroceptiveArousal,
    SemanticStateMeaning::InteroceptiveUncertainty,
    SemanticStateMeaning::FocalDistance,
    SemanticStateMeaning::FocalRelativeX,
    SemanticStateMeaning::FocalRelativeY,
    SemanticStateMeaning::FocalRelativeZ,
    SemanticStateMeaning::FocalMotion,
    SemanticStateMeaning::FocalContact,
    SemanticStateMeaning::SocialProximity,
    SemanticStateMeaning::SocialMotion,
    SemanticStateMeaning::MemoryEvidence,
    SemanticStateMeaning::ConceptEvidence,
    SemanticStateMeaning::GapEvidence,
    SemanticStateMeaning::PredictionUncertainty,
    SemanticStateMeaning::ReservedMeaning30,
    SemanticStateMeaning::ReservedMeaning31,
];

/// The state schema is shared by pre-state and post-state.  There is no
/// outcome-only lane in this table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticStateSchemaV2;

impl SemanticStateSchemaV2 {
    pub const SCHEMA_VERSION: u16 = SEMANTIC_STATE_VECTOR_SCHEMA_VERSION;
    pub const ABI_VERSION: u16 = SEMANTIC_STATE_VECTOR_ABI_V2;
    pub const MEANINGS: [SemanticStateMeaning; MAX_SEMANTIC_STATE_VALUES] =
        SEMANTIC_STATE_MEANINGS_V2;

    pub const fn meaning_at(index: usize) -> Option<SemanticStateMeaning> {
        if index < MAX_SEMANTIC_STATE_VALUES {
            Some(SEMANTIC_STATE_MEANINGS_V2[index])
        } else {
            None
        }
    }

    pub fn validate_len(len: usize) -> Result<(), ScaffoldContractError> {
        if (MIN_SEMANTIC_STATE_VALUES..=MAX_SEMANTIC_STATE_VALUES).contains(&len) {
            Ok(())
        } else {
            Err(ScaffoldContractError::InvalidDecisionEvidence)
        }
    }
}

/// A bounded normalized state with stable index meanings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticStateVector {
    pub schema_version: u16,
    pub abi_version: u16,
    pub values: Vec<f32>,
}

impl SemanticStateVector {
    pub fn new(values: Vec<f32>) -> Result<Self, ScaffoldContractError> {
        let state = Self {
            schema_version: SEMANTIC_STATE_VECTOR_SCHEMA_VERSION,
            abi_version: SEMANTIC_STATE_VECTOR_ABI_V2,
            values,
        };
        state.validate_contract()?;
        Ok(state)
    }

    pub fn from_slice(values: &[f32]) -> Result<Self, ScaffoldContractError> {
        Self::new(values.to_vec())
    }

    pub const fn len(&self) -> usize {
        self.values.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub const fn meaning_at(index: usize) -> Option<SemanticStateMeaning> {
        SemanticStateSchemaV2::meaning_at(index)
    }

    pub fn meanings(&self) -> Result<&'static [SemanticStateMeaning], ScaffoldContractError> {
        self.validate_contract()?;
        Ok(&SEMANTIC_STATE_MEANINGS_V2[..self.values.len()])
    }

    pub fn variance(&self) -> Result<f32, ScaffoldContractError> {
        self.validate_contract()?;
        feature_variance(&self.values)
    }

    pub fn mean_absolute_distance(&self, other: &Self) -> Result<f32, ScaffoldContractError> {
        self.validate_contract()?;
        other.validate_contract()?;
        if self.schema_version != other.schema_version
            || self.abi_version != other.abi_version
            || self.values.len() != other.values.len()
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let distance = self
            .values
            .iter()
            .zip(&other.values)
            .map(|(left, right)| (*left - *right).abs())
            .sum::<f32>()
            / self.values.len() as f32;
        if distance.is_finite() {
            Ok(distance.clamp(0.0, 1.0))
        } else {
            Err(ScaffoldContractError::NonFiniteFloat)
        }
    }

    pub fn canonical_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        self.validate_contract()?;
        let mut builder = CanonicalDigestBuilder::new(b"ALIFE-V11-SEMANTIC-STATE-V2");
        builder.write_u16(self.schema_version);
        builder.write_u16(self.abi_version);
        builder.write_sequence_len(self.values.len());
        for (meaning, value) in self.meanings()?.iter().zip(&self.values) {
            builder.write_u8(*meaning as u8);
            builder.write_f32(*value)?;
        }
        Ok(builder.finish256())
    }
}

impl Validate for SemanticStateVector {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != SEMANTIC_STATE_VECTOR_SCHEMA_VERSION
            || self.abi_version != SEMANTIC_STATE_VECTOR_ABI_V2
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        SemanticStateSchemaV2::validate_len(self.values.len())?;
        for value in &self.values {
            if !value.is_finite() || !(0.0..=1.0).contains(value) {
                return Err(if value.is_finite() {
                    ScaffoldContractError::ScalarOutOfRange
                } else {
                    ScaffoldContractError::NonFiniteFloat
                });
            }
        }
        Ok(())
    }
}

/// Explicit pre/post pairing.  Both states use the same schema prefix and
/// therefore retain identical meanings at every index.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticStateTransition {
    pub schema_version: u16,
    pub abi_version: u16,
    pub pre_state: SemanticStateVector,
    pub post_state: SemanticStateVector,
}

impl SemanticStateTransition {
    pub fn new(
        pre_state: SemanticStateVector,
        post_state: SemanticStateVector,
    ) -> Result<Self, ScaffoldContractError> {
        let transition = Self {
            schema_version: SEMANTIC_STATE_VECTOR_SCHEMA_VERSION,
            abi_version: SEMANTIC_STATE_VECTOR_ABI_V2,
            pre_state,
            post_state,
        };
        transition.validate_contract()?;
        Ok(transition)
    }
}

impl Validate for SemanticStateTransition {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != SEMANTIC_STATE_VECTOR_SCHEMA_VERSION
            || self.abi_version != SEMANTIC_STATE_VECTOR_ABI_V2
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        self.pre_state.validate_contract()?;
        self.post_state.validate_contract()?;
        if self.pre_state.schema_version != self.post_state.schema_version
            || self.pre_state.abi_version != self.post_state.abi_version
            || self.pre_state.values.len() != self.post_state.values.len()
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GroundedOutcomeMeaning {
    DisplacementX,
    DisplacementY,
    DisplacementZ,
    ContactIntensity,
    ActionSucceeded,
    PainDelta,
    EnergyDelta,
    HealthDelta,
}

pub const GROUNDED_OUTCOME_MEANINGS_V1: [GroundedOutcomeMeaning; GROUNDED_OUTCOME_FEATURE_COUNT] = [
    GroundedOutcomeMeaning::DisplacementX,
    GroundedOutcomeMeaning::DisplacementY,
    GroundedOutcomeMeaning::DisplacementZ,
    GroundedOutcomeMeaning::ContactIntensity,
    GroundedOutcomeMeaning::ActionSucceeded,
    GroundedOutcomeMeaning::PainDelta,
    GroundedOutcomeMeaning::EnergyDelta,
    GroundedOutcomeMeaning::HealthDelta,
];

/// Facts caused by the action and measured after execution.  They are kept
/// out of `SemanticStateVector` so a target cannot quietly redefine a
/// pre-action lane as an outcome label.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroundedOutcomeFeatures {
    pub schema_version: u16,
    pub abi_version: u16,
    pub observed: bool,
    pub values: [f32; GROUNDED_OUTCOME_FEATURE_COUNT],
}

impl Default for GroundedOutcomeFeatures {
    fn default() -> Self {
        Self::unknown()
    }
}

impl GroundedOutcomeFeatures {
    pub fn unknown() -> Self {
        Self {
            schema_version: GROUNDED_OUTCOME_SCHEMA_VERSION,
            abi_version: GROUNDED_OUTCOME_ABI_V1,
            observed: false,
            values: [0.0; GROUNDED_OUTCOME_FEATURE_COUNT],
        }
    }

    pub fn new(values: [f32; GROUNDED_OUTCOME_FEATURE_COUNT]) -> Result<Self, ScaffoldContractError> {
        let features = Self {
            schema_version: GROUNDED_OUTCOME_SCHEMA_VERSION,
            abi_version: GROUNDED_OUTCOME_ABI_V1,
            observed: true,
            values,
        };
        features.validate_contract()?;
        Ok(features)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        displacement: Vec3f,
        contact_intensity: f32,
        action_succeeded: bool,
        pain_delta: f32,
        energy_delta: f32,
        health_delta: f32,
    ) -> Result<Self, ScaffoldContractError> {
        displacement.validate()?;
        Self::new([
            bounded_signed(displacement.x),
            bounded_signed(displacement.y),
            bounded_signed(displacement.z),
            contact_intensity.clamp(0.0, 1.0),
            if action_succeeded { 1.0 } else { 0.0 },
            bounded_signed(pain_delta),
            bounded_signed(energy_delta),
            bounded_signed(health_delta),
        ])
    }

    pub const fn meaning_at(index: usize) -> Option<GroundedOutcomeMeaning> {
        if index < GROUNDED_OUTCOME_FEATURE_COUNT {
            Some(GROUNDED_OUTCOME_MEANINGS_V1[index])
        } else {
            None
        }
    }

    pub fn mean_absolute_distance(&self, other: &Self) -> Result<f32, ScaffoldContractError> {
        self.validate_contract()?;
        other.validate_contract()?;
        if !self.observed || !other.observed {
            return Ok(0.0);
        }
        let distance = self
            .values
            .iter()
            .zip(&other.values)
            .map(|(left, right)| (*left - *right).abs())
            .sum::<f32>()
            / GROUNDED_OUTCOME_FEATURE_COUNT as f32;
        Ok(distance.clamp(0.0, 1.0))
    }

    pub fn canonical_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        self.validate_contract()?;
        let mut builder = CanonicalDigestBuilder::new(b"ALIFE-V11-GROUNDED-OUTCOME-V1");
        builder.write_u16(self.schema_version);
        builder.write_u16(self.abi_version);
        builder.write_bool(self.observed);
        for (meaning, value) in GROUNDED_OUTCOME_MEANINGS_V1.iter().zip(self.values) {
            builder.write_u8(*meaning as u8);
            builder.write_f32(value)?;
        }
        Ok(builder.finish256())
    }
}

impl Validate for GroundedOutcomeFeatures {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != GROUNDED_OUTCOME_SCHEMA_VERSION
            || self.abi_version != GROUNDED_OUTCOME_ABI_V1
            || self.values.iter().any(|value| {
                !value.is_finite() || !(-1.0..=1.0).contains(value)
            })
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        if !self.observed && self.values.iter().any(|value| *value != 0.0) {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        if self.values[3] < 0.0 || self.values[3] > 1.0 || self.values[4] < 0.0 || self.values[4] > 1.0 {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
}

/// A motor family is a categorical identity, not a scalar axis.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MotorFamily {
    Locomotion,
    Orientation,
    Manipulation,
    Vocal,
    Posture,
    SpeciesSpecific,
}

impl MotorFamily {
    pub const COUNT: usize = 6;

    pub const fn from_channel(channel: MotorChannel) -> Self {
        match channel {
            MotorChannel::Locomotion => Self::Locomotion,
            MotorChannel::Orientation => Self::Orientation,
            MotorChannel::Manipulation => Self::Manipulation,
            MotorChannel::Vocal => Self::Vocal,
            MotorChannel::Posture => Self::Posture,
            MotorChannel::SpeciesSpecific(_) => Self::SpeciesSpecific,
        }
    }

    pub const fn one_hot(self) -> [f32; Self::COUNT] {
        let mut values = [0.0; Self::COUNT];
        values[self.index()] = 1.0;
        values
    }

    const fn index(self) -> usize {
        match self {
            Self::Locomotion => 0,
            Self::Orientation => 1,
            Self::Manipulation => 2,
            Self::Vocal => 3,
            Self::Posture => 4,
            Self::SpeciesSpecific => 5,
        }
    }
}

/// Opaque primitive identity.  The digest is a lookup key only.  It is never
/// converted to a scalar, byte lane, bit lane, ordinal, or distance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CategoricalMotorPrimitive {
    pub schema_version: u16,
    pub abi_version: u16,
    pub family: MotorFamily,
    pub identity: [u64; 4],
}

impl CategoricalMotorPrimitive {
    pub fn from_action(family: MotorFamily, primitive: ActionId) -> Self {
        let mut builder = CanonicalDigestBuilder::new(b"ALIFE-V11-CATEGORICAL-MOTOR-PRIMITIVE-V1");
        builder.write_u8(family as u8);
        builder.write_u32(primitive.raw());
        Self {
            schema_version: MOTOR_CATEGORY_SCHEMA_VERSION,
            abi_version: MOTOR_CATEGORY_ABI_V1,
            family,
            identity: builder.finish256(),
        }
    }

    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != MOTOR_CATEGORY_SCHEMA_VERSION
            || self.abi_version != MOTOR_CATEGORY_ABI_V1
            || self.identity == [0; 4]
        {
            return Err(ScaffoldContractError::InvalidActionDecision);
        }
        Ok(())
    }
}

/// Persistable learned embedding for one opaque primitive category.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MotorPrimitiveEmbedding {
    pub schema_version: u16,
    pub abi_version: u16,
    pub primitive: CategoricalMotorPrimitive,
    pub values: [f32; MOTOR_PRIMITIVE_EMBEDDING_DIM],
}

impl MotorPrimitiveEmbedding {
    pub fn new(
        primitive: CategoricalMotorPrimitive,
        values: [f32; MOTOR_PRIMITIVE_EMBEDDING_DIM],
    ) -> Result<Self, ScaffoldContractError> {
        let embedding = Self {
            schema_version: MOTOR_CATEGORY_SCHEMA_VERSION,
            abi_version: MOTOR_CATEGORY_ABI_V1,
            primitive,
            values,
        };
        embedding.validate_contract()?;
        Ok(embedding)
    }
}

impl Validate for MotorPrimitiveEmbedding {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != MOTOR_CATEGORY_SCHEMA_VERSION
            || self.abi_version != MOTOR_CATEGORY_ABI_V1
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        self.primitive.validate_contract()?;
        if self.values.iter().any(|value| !value.is_finite() || !(-1.0..=1.0).contains(value)) {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
}

fn bounded_signed(value: f32) -> f32 {
    (value / (1.0 + value.abs())).clamp(-1.0, 1.0)
}

fn feature_variance(features: &[f32]) -> Result<f32, ScaffoldContractError> {
    if features.is_empty() {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    let mean = features.iter().sum::<f32>() / features.len() as f32;
    let variance = features
        .iter()
        .map(|value| {
            let delta = *value - mean;
            delta * delta
        })
        .sum::<f32>()
        / features.len() as f32;
    if variance.is_finite() {
        Ok(variance.clamp(0.0, 1.0))
    } else {
        Err(ScaffoldContractError::NonFiniteFloat)
    }
}

#[allow(dead_code)]
const _: usize = MAX_MOTOR_CHANNELS;
