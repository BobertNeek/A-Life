//! Grounded, versioned prediction targets and factorized motor conditioning.

use serde::{Deserialize, Serialize};

use crate::{
    ActionId, ActionTarget, CanonicalDigestBuilder, ChannelCommand, ExperienceSequenceId,
    MotorChannel, MotorCommandBundle, NormalizedScalar, OrganismId, ScaffoldContractError, Tick,
    Validate, Vec3f, MAX_MOTOR_CHANNELS, MAX_MOTOR_PAYLOAD_VALUES,
};

pub const PREDICTION_TARGET_SCHEMA_VERSION: u16 = 3;
pub const SEMANTIC_STATE_VECTOR_SCHEMA_VERSION: u16 = 1;
pub const SEMANTIC_STATE_VECTOR_ABI_V1: u16 = 1;
pub const JOINT_MOTOR_CONDITION_SCHEMA_VERSION: u16 = 2;
pub const JOINT_MOTOR_CONDITION_ABI_V1: u16 = 1;
pub const JOINT_MOTOR_CONDITION_ABI_V2: u16 = 2;
pub const GROUNDED_PREDICTOR_ABI_VERSION: u16 = 2;
pub const MAX_PREDICTOR_CATEGORIES: usize = 4096;
pub const MAX_SEMANTIC_STATE_VALUES: usize = 32;
pub const MAX_SUCCESSOR_FEATURES: usize = MAX_SEMANTIC_STATE_VALUES;
pub const SUCCESSOR_FEATURE_ABI_V1: u16 = 1;
pub const DEFAULT_PREDICTOR_LEARNING_RATE: f32 = 0.25;

const MOTOR_CONTINUOUS_COMPONENTS: usize = 14;
const MOTOR_CHANNEL_IDENTITIES: usize = 5 + 256;
const MAX_PREDICTOR_ROWS: usize = MAX_PREDICTOR_CATEGORIES
    + MOTOR_CHANNEL_IDENTITIES * MOTOR_CONTINUOUS_COMPONENTS
    + MAX_SEMANTIC_STATE_VALUES
    + 1;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredictionTargetFamily {
    EmaTeacher,
    StopGradientAsymmetric,
    FixedProjection,
    VarianceCovarianceConstrained,
    Contrastive,
    GroundedObservables,
    Composite,
}

/// A bounded semantic state. Its values are model inputs, unlike canonical
/// digests, which remain identity and integrity evidence only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticStateVector {
    pub schema_version: u16,
    pub abi_version: u16,
    #[serde(deserialize_with = "deserialize_prediction_weights")]
    pub values: Vec<f32>,
}

impl SemanticStateVector {
    pub fn new(values: Vec<f32>) -> Result<Self, ScaffoldContractError> {
        let state = Self {
            schema_version: SEMANTIC_STATE_VECTOR_SCHEMA_VERSION,
            abi_version: SEMANTIC_STATE_VECTOR_ABI_V1,
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

    pub fn variance(&self) -> Result<f32, ScaffoldContractError> {
        self.validate_contract()?;
        feature_variance(&self.values)
    }

    pub fn mean_absolute_distance(&self, other: &Self) -> Result<f32, ScaffoldContractError> {
        self.validate_contract()?;
        other.validate_contract()?;
        if self.abi_version != other.abi_version || self.values.len() != other.values.len() {
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
        let mut builder = CanonicalDigestBuilder::new(b"ALIFE-V11-SEMANTIC-STATE-V1");
        builder.write_u16(self.schema_version);
        builder.write_u16(self.abi_version);
        builder.write_sequence_len(self.values.len());
        for value in &self.values {
            builder.write_f32(*value)?;
        }
        Ok(builder.finish256())
    }
}

impl Validate for SemanticStateVector {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != SEMANTIC_STATE_VECTOR_SCHEMA_VERSION
            || self.abi_version != SEMANTIC_STATE_VECTOR_ABI_V1
            || self.values.len() < 2
            || self.values.len() > MAX_SEMANTIC_STATE_VALUES
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        for value in &self.values {
            NormalizedScalar::new(*value)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotorChannelFactor {
    pub channel: MotorChannel,
    pub primitive: ActionId,
    pub intensity: f32,
    pub duration_ticks: u32,
    pub direction: Vec3f,
    pub stand_off_distance: f32,
    pub confidence: f32,
    pub target: Option<ActionTarget>,
    #[serde(deserialize_with = "deserialize_motor_payload")]
    pub payload: Vec<u32>,
    pub coordination_group: u8,
}

impl MotorChannelFactor {
    pub fn from_command(command: &ChannelCommand) -> Result<Self, ScaffoldContractError> {
        command.validate_contract()?;
        let factor = Self {
            channel: command.channel,
            primitive: command.primitive,
            intensity: command.intensity.raw(),
            duration_ticks: command.duration_ticks.raw(),
            direction: command.direction,
            stand_off_distance: command.stand_off_distance,
            confidence: command.confidence.raw(),
            target: command.target,
            payload: command.payload.values.clone(),
            coordination_group: command.coordination_group,
        };
        factor.validate_contract()?;
        Ok(factor)
    }

    fn features(&self) -> Vec<(PredictionFeatureKey, f32)> {
        use MotorContinuousComponent::*;
        let channel = self.channel.canonical_key();
        let position = self.target.and_then(|target| target.position);
        let values = [
            (Presence, 1.0),
            (Intensity, self.intensity),
            (
                Duration,
                self.duration_ticks as f32 / (self.duration_ticks as f32 + 1.0),
            ),
            (DirectionX, signed_unit(self.direction.x)),
            (DirectionY, signed_unit(self.direction.y)),
            (DirectionZ, signed_unit(self.direction.z)),
            (StandOff, bounded_unit(self.stand_off_distance)),
            (Confidence, self.confidence),
            (TargetPresent, if self.target.is_some() { 1.0 } else { 0.0 }),
            (
                TargetPositionPresent,
                if position.is_some() { 1.0 } else { 0.0 },
            ),
            (
                TargetPositionX,
                position.map_or(0.0, |position| signed_unit(position.x)),
            ),
            (
                TargetPositionY,
                position.map_or(0.0, |position| signed_unit(position.y)),
            ),
            (
                TargetPositionZ,
                position.map_or(0.0, |position| signed_unit(position.z)),
            ),
            (
                PayloadLength,
                self.payload.len() as f32 / MAX_MOTOR_PAYLOAD_VALUES as f32,
            ),
        ];
        let mut features = values
            .into_iter()
            .map(|(component, value)| {
                (
                    PredictionFeatureKey::Continuous { channel, component },
                    value,
                )
            })
            .collect::<Vec<_>>();
        let mut categories = vec![
            MotorCategory::Primitive(self.primitive.raw()),
            MotorCategory::TargetEntity(
                self.target
                    .and_then(|target| target.entity)
                    .map(|id| id.raw()),
            ),
            MotorCategory::CoordinationGroup(self.coordination_group),
        ];
        categories.extend(self.payload.iter().enumerate().map(|(position, value)| {
            MotorCategory::Payload {
                position: position as u8,
                value: *value,
            }
        }));
        features.extend(
            categories
                .into_iter()
                .map(|category| (PredictionFeatureKey::Category { channel, category }, 1.0)),
        );
        features
    }
}

impl Validate for MotorChannelFactor {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.primitive.validate()?;
        NormalizedScalar::new(self.intensity)?;
        self.direction.validate()?;
        if self.duration_ticks == 0
            || !self.stand_off_distance.is_finite()
            || self.stand_off_distance < 0.0
            || !(0.0..=1.0).contains(&self.confidence)
            || self.payload.len() > MAX_MOTOR_PAYLOAD_VALUES
        {
            return Err(ScaffoldContractError::InvalidActionDecision);
        }
        if let Some(target) = self.target {
            target.validate()?;
        }
        Ok(())
    }
}

/// Bounded, deterministic factors for every selected motor channel. The
/// predictor sees these factors, not an action or frame digest chunk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JointMotorCondition {
    pub schema_version: u16,
    pub abi_version: u16,
    #[serde(deserialize_with = "deserialize_motor_channels")]
    pub channels: Vec<MotorChannelFactor>,
}

impl JointMotorCondition {
    pub fn new(mut channels: Vec<MotorChannelFactor>) -> Result<Self, ScaffoldContractError> {
        channels.sort_by_key(|factor| factor.channel.canonical_key());
        let condition = Self {
            schema_version: JOINT_MOTOR_CONDITION_SCHEMA_VERSION,
            abi_version: JOINT_MOTOR_CONDITION_ABI_V2,
            channels,
        };
        condition.validate_contract()?;
        Ok(condition)
    }

    pub fn from_bundle(bundle: &MotorCommandBundle) -> Result<Self, ScaffoldContractError> {
        bundle.validate_contract()?;
        bundle
            .channels
            .iter()
            .map(MotorChannelFactor::from_command)
            .collect::<Result<Vec<_>, _>>()
            .and_then(Self::new)
    }

    fn features(&self) -> Vec<(PredictionFeatureKey, f32)> {
        let mut features = self
            .channels
            .iter()
            .flat_map(MotorChannelFactor::features)
            .collect::<Vec<_>>();
        features.sort_by(|left, right| left.0.cmp(&right.0));
        features
    }

    /// A bounded signal that reports whether the condition carries usable
    /// motor information. It is diagnostic, not a reward or credit value.
    pub fn mean_feature_magnitude(&self) -> Result<f32, ScaffoldContractError> {
        self.validate_contract()?;
        let features = self.features();
        let score =
            features.iter().map(|(_, value)| value.abs()).sum::<f32>() / features.len() as f32;
        if score.is_finite() {
            Ok(score.clamp(0.0, 1.0))
        } else {
            Err(ScaffoldContractError::NonFiniteFloat)
        }
    }

    pub fn canonical_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        self.validate_contract()?;
        let mut builder = CanonicalDigestBuilder::new(b"ALIFE-V11-JOINT-MOTOR-CONDITION-V2");
        builder.write_u16(self.schema_version);
        builder.write_u16(self.abi_version);
        builder.write_sequence_len(self.channels.len());
        for factor in &self.channels {
            builder.write_u16(factor.channel.canonical_key());
            builder.write_u32(factor.primitive.raw());
            builder.write_f32(factor.intensity)?;
            builder.write_u32(factor.duration_ticks);
            builder.write_f32(factor.direction.x)?;
            builder.write_f32(factor.direction.y)?;
            builder.write_f32(factor.direction.z)?;
            builder.write_f32(factor.stand_off_distance)?;
            builder.write_f32(factor.confidence)?;
            match factor.target {
                Some(target) => {
                    builder.write_some();
                    match target.entity {
                        Some(entity) => {
                            builder.write_some();
                            builder.write_u64(entity.raw());
                        }
                        None => builder.write_none(),
                    }
                    match target.position {
                        Some(position) => {
                            builder.write_some();
                            builder.write_f32(position.x)?;
                            builder.write_f32(position.y)?;
                            builder.write_f32(position.z)?;
                        }
                        None => builder.write_none(),
                    }
                }
                None => builder.write_none(),
            }
            builder.write_sequence_len(factor.payload.len());
            for value in &factor.payload {
                builder.write_u32(*value);
            }
            builder.write_u8(factor.coordination_group);
        }
        Ok(builder.finish256())
    }
}

impl Validate for JointMotorCondition {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != JOINT_MOTOR_CONDITION_SCHEMA_VERSION
            || self.abi_version != JOINT_MOTOR_CONDITION_ABI_V2
            || self.channels.is_empty()
            || self.channels.len() > MAX_MOTOR_CHANNELS
            || self
                .channels
                .windows(2)
                .any(|pair| pair[0].channel.canonical_key() >= pair[1].channel.canonical_key())
        {
            return Err(ScaffoldContractError::InvalidActionDecision);
        }
        for factor in &self.channels {
            factor.validate_contract()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
enum MotorContinuousComponent {
    Presence,
    Intensity,
    Duration,
    DirectionX,
    DirectionY,
    DirectionZ,
    StandOff,
    Confidence,
    TargetPresent,
    TargetPositionPresent,
    TargetPositionX,
    TargetPositionY,
    TargetPositionZ,
    PayloadLength,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
enum MotorCategory {
    Primitive(u32),
    TargetEntity(Option<u64>),
    Payload { position: u8, value: u32 },
    CoordinationGroup(u8),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
enum PredictionFeatureKey {
    Bias,
    Semantic(u8),
    Continuous {
        channel: u16,
        component: MotorContinuousComponent,
    },
    Category {
        channel: u16,
        category: MotorCategory,
    },
}

impl PredictionFeatureKey {
    fn is_category(&self) -> bool {
        matches!(self, Self::Category { .. })
    }

    fn validate(&self, semantic_count: usize) -> Result<(), ScaffoldContractError> {
        let channel = match self {
            Self::Bias => return Ok(()),
            Self::Semantic(index) if usize::from(*index) < semantic_count => return Ok(()),
            Self::Semantic(_) => return Err(ScaffoldContractError::InvalidDecisionEvidence),
            Self::Continuous { channel, .. } => *channel,
            Self::Category { channel, category } => {
                match category {
                    MotorCategory::Primitive(id) => {
                        ActionId(*id).validate()?;
                    }
                    MotorCategory::TargetEntity(Some(id)) if *id == 0 => {
                        return Err(ScaffoldContractError::InvalidDecisionEvidence)
                    }
                    MotorCategory::Payload { position, .. }
                        if usize::from(*position) >= MAX_MOTOR_PAYLOAD_VALUES =>
                    {
                        return Err(ScaffoldContractError::InvalidDecisionEvidence)
                    }
                    _ => {}
                }
                *channel
            }
        };
        if channel <= 4 || (0x100..=0x1ff).contains(&channel) {
            Ok(())
        } else {
            Err(ScaffoldContractError::InvalidDecisionEvidence)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PredictorWeightRow {
    key: PredictionFeatureKey,
    #[serde(deserialize_with = "deserialize_prediction_weights")]
    weights: Vec<f32>,
}

fn deserialize_bounded_vec<'de, D, T, const LIMIT: usize>(
    deserializer: D,
) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Visitor<T, const LIMIT: usize>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>, const LIMIT: usize> serde::de::Visitor<'de> for Visitor<T, LIMIT> {
        type Value = Vec<T>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "at most {LIMIT} predictor entries")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut sequence: A,
        ) -> Result<Self::Value, A::Error> {
            if sequence.size_hint().is_some_and(|size| size > LIMIT) {
                return Err(serde::de::Error::custom("predictor entry limit exceeded"));
            }
            let mut values = Vec::with_capacity(sequence.size_hint().unwrap_or(0).min(LIMIT));
            while let Some(value) = sequence.next_element()? {
                if values.len() == LIMIT {
                    return Err(serde::de::Error::custom("predictor entry limit exceeded"));
                }
                values.push(value);
            }
            Ok(values)
        }
    }
    deserializer.deserialize_seq(Visitor::<T, LIMIT>(std::marker::PhantomData))
}

fn deserialize_prediction_weights<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<f32>, D::Error> {
    deserialize_bounded_vec::<D, f32, MAX_SEMANTIC_STATE_VALUES>(deserializer)
}

fn deserialize_motor_payload<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<u32>, D::Error> {
    deserialize_bounded_vec::<D, u32, MAX_MOTOR_PAYLOAD_VALUES>(deserializer)
}

fn deserialize_motor_channels<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<MotorChannelFactor>, D::Error> {
    deserialize_bounded_vec::<D, MotorChannelFactor, MAX_MOTOR_CHANNELS>(deserializer)
}

#[derive(Deserialize)]
struct BoundedPredictorRows(
    #[serde(deserialize_with = "deserialize_prediction_rows")] Vec<PredictorWeightRow>,
);

fn deserialize_prediction_rows<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<PredictorWeightRow>, D::Error> {
    deserialize_bounded_vec::<D, PredictorWeightRow, MAX_PREDICTOR_ROWS>(deserializer)
}

/// Coverage records exact categorical coefficients available to a forecast.
/// Occupancy is not evidence that a prediction is accurate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PredictionCategoryCoverage {
    pub stored_categorical_keys: u16,
    pub modelled_categories: u16,
    pub unmodelled_categories: u16,
}

impl Validate for PredictionCategoryCoverage {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if usize::from(self.stored_categorical_keys) > MAX_PREDICTOR_CATEGORIES
            || self.modelled_categories > self.stored_categorical_keys
            || usize::from(self.modelled_categories) + usize::from(self.unmodelled_categories)
                > MAX_MOTOR_CHANNELS * (MAX_MOTOR_PAYLOAD_VALUES + 3)
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PredictionTargetReceipt {
    pub schema_version: u16,
    pub organism_id: OrganismId,
    pub experience_sequence: ExperienceSequenceId,
    pub decision: ActionId,
    pub world_tick: Tick,
    /// Identity of the perception frame. Never used as predictor input.
    pub source_digest: [u64; 4],
    pub source_state: SemanticStateVector,
    pub motor_condition: JointMotorCondition,
    pub target_digest: [u64; 4],
    pub target_state: SemanticStateVector,
    pub target_component_variance: f32,
    pub motor_condition_magnitude: f32,
    pub successor_observed_distance: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuccessorPrediction {
    pub source_digest: [u64; 4],
    pub source_state: SemanticStateVector,
    pub motor_condition: JointMotorCondition,
    pub semantic_state_abi: u16,
    #[serde(deserialize_with = "deserialize_prediction_weights")]
    pub predicted_successor: Vec<f32>,
    pub predictor_abi_version: u16,
    pub category_coverage: PredictionCategoryCoverage,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PredictionUpdate {
    pub prediction: SuccessorPrediction,
    pub target_digest: [u64; 4],
    #[serde(deserialize_with = "deserialize_prediction_weights")]
    pub error: Vec<f32>,
    pub mean_squared_error: f32,
    pub mean_absolute_error: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GroundedSuccessorPredictor {
    schema_version: u16,
    semantic_state_abi: u16,
    semantic_state_count: usize,
    motor_condition_abi: u16,
    learning_rate: f32,
    rows: Vec<PredictorWeightRow>,
    last_update: Option<PredictionUpdate>,
}

#[derive(Deserialize)]
struct EmptyLegacyWeights(
    #[serde(deserialize_with = "deserialize_empty_legacy_weights")] Vec<serde::de::IgnoredAny>,
);

fn deserialize_empty_legacy_weights<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<serde::de::IgnoredAny>, D::Error> {
    deserialize_bounded_vec::<D, serde::de::IgnoredAny, 0>(deserializer)
}

impl<'de> Deserialize<'de> for GroundedSuccessorPredictor {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            schema_version: Option<u16>,
            semantic_state_abi: u16,
            semantic_state_count: usize,
            motor_condition_abi: u16,
            input_feature_count: Option<usize>,
            learning_rate: f32,
            rows: Option<BoundedPredictorRows>,
            weights: Option<EmptyLegacyWeights>,
            #[serde(deserialize_with = "deserialize_optional_prediction_update")]
            last_update: Option<PredictionUpdate>,
        }
        let wire = Wire::deserialize(deserializer)?;
        let value = match wire.schema_version {
            Some(version) if version == GROUNDED_PREDICTOR_ABI_VERSION
                && wire.weights.is_none() && wire.input_feature_count.is_none() => Self {
                    schema_version: version,
                    semantic_state_abi: wire.semantic_state_abi,
                    semantic_state_count: wire.semantic_state_count,
                    motor_condition_abi: wire.motor_condition_abi,
                    learning_rate: wire.learning_rate,
                    rows: wire.rows.ok_or_else(|| serde::de::Error::custom("missing predictor rows"))?.0,
                    last_update: wire.last_update,
                },
            None if wire.semantic_state_abi == 0 && wire.semantic_state_count == 0
                && wire.motor_condition_abi == 0 && wire.input_feature_count == Some(0)
                && wire.rows.is_none() && wire.weights.as_ref().is_some_and(|weights| weights.0.is_empty())
                && wire.last_update.is_none() => Self {
                    learning_rate: wire.learning_rate,
                    ..Self::default()
                },
            _ => return Err(serde::de::Error::custom(
                "unsupported predictor ABI: acquired legacy weights cannot be reset or reinterpreted",
            )),
        };
        value
            .validate_contract()
            .map_err(serde::de::Error::custom)?;
        Ok(value)
    }
}

fn deserialize_optional_prediction_update<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<PredictionUpdate>, D::Error> {
    Option::<PredictionUpdate>::deserialize(deserializer)
}

impl Default for GroundedSuccessorPredictor {
    fn default() -> Self {
        Self {
            schema_version: GROUNDED_PREDICTOR_ABI_VERSION,
            semantic_state_abi: 0,
            semantic_state_count: 0,
            motor_condition_abi: JOINT_MOTOR_CONDITION_ABI_V2,
            learning_rate: DEFAULT_PREDICTOR_LEARNING_RATE,
            rows: Vec::new(),
            last_update: None,
        }
    }
}

impl GroundedSuccessorPredictor {
    pub fn with_learning_rate(learning_rate: f32) -> Result<Self, ScaffoldContractError> {
        let value = Self {
            learning_rate,
            ..Self::default()
        };
        value.validate_contract()?;
        Ok(value)
    }

    pub fn predict(
        &self,
        source_state: &SemanticStateVector,
        motor_condition: &JointMotorCondition,
    ) -> Result<SuccessorPrediction, ScaffoldContractError> {
        source_state.validate_contract()?;
        motor_condition.validate_contract()?;
        self.validate_runtime_shape(source_state, motor_condition)?;
        let features = predictor_inputs(source_state, motor_condition);
        let mut predicted_successor = vec![0.0_f32; source_state.len()];
        let mut modelled_categories = 0;
        let mut unmodelled_categories = 0;
        for (key, value) in &features {
            match self.rows.binary_search_by(|row| row.key.cmp(key)) {
                Ok(index) => {
                    if key.is_category() {
                        modelled_categories += 1;
                    }
                    for (prediction, weight) in predicted_successor
                        .iter_mut()
                        .zip(&self.rows[index].weights)
                    {
                        *prediction += weight * value;
                    }
                }
                Err(_) if key.is_category() => unmodelled_categories += 1,
                Err(_) => {}
            }
        }
        for prediction in &mut predicted_successor {
            if !prediction.is_finite() {
                return Err(ScaffoldContractError::NonFiniteFloat);
            }
            *prediction = prediction.clamp(0.0, 1.0);
        }
        Ok(SuccessorPrediction {
            source_digest: [0; 4],
            source_state: source_state.clone(),
            motor_condition: motor_condition.clone(),
            semantic_state_abi: source_state.abi_version,
            predicted_successor,
            predictor_abi_version: GROUNDED_PREDICTOR_ABI_VERSION,
            category_coverage: PredictionCategoryCoverage {
                stored_categorical_keys: self.stored_category_count() as u16,
                modelled_categories,
                unmodelled_categories,
            },
        })
    }

    /// Returns whether the predictor has learned at least one grounded target.
    /// An uninitialized predictor may still produce a zero forecast, but that
    /// forecast is not exposed as acquired cognitive state.
    pub const fn has_acquired_state(&self) -> bool {
        self.semantic_state_abi != 0 && self.last_update.is_some()
    }

    pub fn observe(
        &mut self,
        receipt: &PredictionTargetReceipt,
    ) -> Result<PredictionUpdate, ScaffoldContractError> {
        receipt.validate_contract()?;
        self.validate_contract()?;
        let mut prediction = self.predict(&receipt.source_state, &receipt.motor_condition)?;
        prediction.source_digest = receipt.source_digest;
        self.observe_frozen(receipt, &prediction)
    }

    /// Apply a target to a forecast that was computed before the world
    /// mutation. The source identity and motor condition are checked so a
    /// forecast from another candidate or frame cannot be credited here.
    pub fn observe_frozen(
        &mut self,
        receipt: &PredictionTargetReceipt,
        frozen: &SuccessorPrediction,
    ) -> Result<PredictionUpdate, ScaffoldContractError> {
        receipt.validate_contract()?;
        self.validate_contract()?;
        frozen.source_state.validate_contract()?;
        frozen.motor_condition.validate_contract()?;
        if frozen.source_digest != receipt.source_digest
            || frozen.semantic_state_abi != receipt.source_state.abi_version
            || frozen.source_state != receipt.source_state
            || frozen.motor_condition != receipt.motor_condition
            || frozen.predictor_abi_version != GROUNDED_PREDICTOR_ABI_VERSION
            || frozen.predicted_successor.len() != receipt.target_state.len()
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let mut prediction = frozen.clone();
        prediction.source_digest = receipt.source_digest;
        let features = predictor_inputs(&receipt.source_state, &receipt.motor_condition);
        // Stage all admission and arithmetic. Invalid input or overflow cannot partly
        // allocate categories, change coefficients, or replace the last forecast.
        let mut staged = self.clone();
        if staged.semantic_state_abi == 0 {
            staged.semantic_state_abi = receipt.source_state.abi_version;
            staged.semantic_state_count = receipt.source_state.len();
        }
        let mut categories = staged.stored_category_count();
        for (key, _) in &features {
            if let Err(index) = staged.rows.binary_search_by(|row| row.key.cmp(key)) {
                if key.is_category() {
                    if categories == MAX_PREDICTOR_CATEGORIES {
                        continue;
                    }
                    categories += 1;
                }
                staged.rows.insert(
                    index,
                    PredictorWeightRow {
                        key: key.clone(),
                        weights: vec![0.0; staged.semantic_state_count],
                    },
                );
            }
        }
        let active = features
            .iter()
            .filter_map(|(key, value)| {
                staged
                    .rows
                    .binary_search_by(|row| row.key.cmp(key))
                    .ok()
                    .map(|index| (index, *value))
            })
            .collect::<Vec<_>>();
        let input_energy = active.iter().map(|(_, value)| value * value).sum::<f32>();
        let normalized_step = self.learning_rate / input_energy.max(f32::EPSILON);
        let mut error = Vec::with_capacity(receipt.target_state.len());
        let mut squared_error = 0.0;
        let mut absolute_error = 0.0;
        for (output, (predicted, target)) in prediction
            .predicted_successor
            .iter()
            .zip(&receipt.target_state.values)
            .enumerate()
        {
            let difference = *target - *predicted;
            error.push(difference);
            squared_error += difference * difference;
            absolute_error += difference.abs();
            for &(index, input) in &active {
                let weight = &mut staged.rows[index].weights[output];
                *weight += normalized_step * difference * input;
                if !weight.is_finite() {
                    return Err(ScaffoldContractError::NonFiniteFloat);
                }
            }
        }
        let count = receipt.target_state.len() as f32;
        let update = PredictionUpdate {
            prediction,
            target_digest: receipt.target_digest,
            error,
            mean_squared_error: squared_error / count,
            mean_absolute_error: absolute_error / count,
        };
        staged.last_update = Some(update.clone());
        staged.validate_contract()?;
        *self = staged;
        Ok(update)
    }

    pub fn last_update(&self) -> Option<&PredictionUpdate> {
        self.last_update.as_ref()
    }

    pub fn stored_category_count(&self) -> usize {
        self.rows.iter().filter(|row| row.key.is_category()).count()
    }

    fn validate_runtime_shape(
        &self,
        source_state: &SemanticStateVector,
        motor_condition: &JointMotorCondition,
    ) -> Result<(), ScaffoldContractError> {
        if self.schema_version != GROUNDED_PREDICTOR_ABI_VERSION
            || self.motor_condition_abi != motor_condition.abi_version
            || (self.semantic_state_abi != 0
                && (self.semantic_state_abi != source_state.abi_version
                    || self.semantic_state_count != source_state.len()))
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
}

impl Validate for GroundedSuccessorPredictor {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != GROUNDED_PREDICTOR_ABI_VERSION
            || self.motor_condition_abi != JOINT_MOTOR_CONDITION_ABI_V2
            || self.rows.len() > MAX_PREDICTOR_ROWS
            || self.stored_category_count() > MAX_PREDICTOR_CATEGORIES
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        if !self.learning_rate.is_finite()
            || !(0.0..=1.0).contains(&self.learning_rate)
            || self.learning_rate == 0.0
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if self.semantic_state_abi == 0 {
            if self.semantic_state_count != 0 || !self.rows.is_empty() || self.last_update.is_some()
            {
                return Err(ScaffoldContractError::InvalidDecisionEvidence);
            }
            return Ok(());
        }
        if self.semantic_state_abi != SEMANTIC_STATE_VECTOR_ABI_V1
            || !(2..=MAX_SEMANTIC_STATE_VALUES).contains(&self.semantic_state_count)
            || self.rows.windows(2).any(|rows| rows[0].key >= rows[1].key)
            || self.last_update.is_none()
            || self
                .rows
                .first()
                .is_none_or(|row| row.key != PredictionFeatureKey::Bias)
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        for index in 0..self.semantic_state_count {
            let key = PredictionFeatureKey::Semantic(index as u8);
            if self.rows.binary_search_by(|row| row.key.cmp(&key)).is_err() {
                return Err(ScaffoldContractError::InvalidDecisionEvidence);
            }
        }
        for row in &self.rows {
            row.key.validate(self.semantic_state_count)?;
            if row.weights.len() != self.semantic_state_count
                || row.weights.iter().any(|weight| !weight.is_finite())
            {
                return Err(ScaffoldContractError::InvalidDecisionEvidence);
            }
        }
        if let Some(update) = &self.last_update {
            validate_prediction_update(update, self.semantic_state_count)?;
            if usize::from(update.prediction.category_coverage.stored_categorical_keys)
                > self.stored_category_count()
            {
                return Err(ScaffoldContractError::InvalidDecisionEvidence);
            }
        }
        Ok(())
    }
}

impl PredictionTargetReceipt {
    pub fn for_successor(
        organism_id: OrganismId,
        experience_sequence: ExperienceSequenceId,
        decision: ActionId,
        world_tick: Tick,
        source_digest: [u64; 4],
        source_state: SemanticStateVector,
        motor_condition: JointMotorCondition,
        target_state: SemanticStateVector,
    ) -> Result<Self, ScaffoldContractError> {
        source_state.validate_contract()?;
        motor_condition.validate_contract()?;
        target_state.validate_contract()?;
        if source_digest == [0; 4]
            || source_state.abi_version != target_state.abi_version
            || source_state.len() != target_state.len()
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let target_digest = target_state.canonical_digest()?;
        let receipt = Self {
            schema_version: PREDICTION_TARGET_SCHEMA_VERSION,
            organism_id,
            experience_sequence,
            decision,
            world_tick,
            source_digest,
            source_state: source_state.clone(),
            motor_condition: motor_condition.clone(),
            target_digest,
            target_state: target_state.clone(),
            target_component_variance: target_state.variance()?,
            motor_condition_magnitude: motor_condition.mean_feature_magnitude()?,
            successor_observed_distance: source_state.mean_absolute_distance(&target_state)?,
        };
        receipt.validate_contract()?;
        Ok(receipt)
    }

    pub fn canonical_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        self.validate_contract()?;
        let mut builder = CanonicalDigestBuilder::new(b"ALIFE-V11-PREDICTION-TARGET-V3");
        builder.write_u16(self.schema_version);
        builder.write_u64(self.organism_id.raw());
        builder.write_u64(self.experience_sequence.raw());
        builder.write_u32(self.decision.raw());
        builder.write_u64(self.world_tick.raw());
        write_words(&mut builder, self.source_digest);
        for word in self.source_state.canonical_digest()? {
            builder.write_u64(word);
        }
        for word in self.motor_condition.canonical_digest()? {
            builder.write_u64(word);
        }
        write_words(&mut builder, self.target_digest);
        for word in self.target_state.canonical_digest()? {
            builder.write_u64(word);
        }
        builder.write_f32(self.target_component_variance)?;
        builder.write_f32(self.motor_condition_magnitude)?;
        builder.write_f32(self.successor_observed_distance)?;
        Ok(builder.finish256())
    }

    pub const fn target_digest(&self) -> [u64; 4] {
        self.target_digest
    }

    pub const fn source_state(&self) -> &SemanticStateVector {
        &self.source_state
    }

    pub const fn motor_condition(&self) -> &JointMotorCondition {
        &self.motor_condition
    }

    pub const fn target_state(&self) -> &SemanticStateVector {
        &self.target_state
    }
}

impl Validate for PredictionTargetReceipt {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != PREDICTION_TARGET_SCHEMA_VERSION
            || self.source_digest == [0; 4]
            || self.target_digest == [0; 4]
            || !self.target_component_variance.is_finite()
            || !self.motor_condition_magnitude.is_finite()
            || !self.successor_observed_distance.is_finite()
            || !(0.0..=1.0).contains(&self.target_component_variance)
            || !(0.0..=1.0).contains(&self.motor_condition_magnitude)
            || !(0.0..=1.0).contains(&self.successor_observed_distance)
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        self.organism_id.validate()?;
        self.experience_sequence.validate()?;
        self.decision.validate()?;
        self.source_state.validate_contract()?;
        self.motor_condition.validate_contract()?;
        self.target_state.validate_contract()?;
        if self.source_state.abi_version != self.target_state.abi_version
            || self.source_state.len() != self.target_state.len()
            || self.target_state.canonical_digest()? != self.target_digest
            || self.target_component_variance != self.target_state.variance()?
            || self.motor_condition_magnitude != self.motor_condition.mean_feature_magnitude()?
            || self.successor_observed_distance
                != self
                    .source_state
                    .mean_absolute_distance(&self.target_state)?
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
}

fn predictor_inputs(
    source_state: &SemanticStateVector,
    motor_condition: &JointMotorCondition,
) -> Vec<(PredictionFeatureKey, f32)> {
    // Bound each numeric block's energy by one without changing its relative
    // magnitudes. Each channel has its own fixed dimension, so adding another
    // channel never rescales previously learned inputs. Categories remain exact
    // unit indicators rather than acquiring a geometry from their logical IDs.
    let source_scale = ((1 + source_state.len()) as f32).sqrt().recip();
    let channel_scale = (MOTOR_CONTINUOUS_COMPONENTS as f32).sqrt().recip();
    let mut inputs = vec![(PredictionFeatureKey::Bias, source_scale)];
    inputs.extend(
        source_state
            .values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                (
                    PredictionFeatureKey::Semantic(index as u8),
                    *value * source_scale,
                )
            }),
    );
    inputs.extend(motor_condition.features().into_iter().map(|(key, value)| {
        let scale = if matches!(key, PredictionFeatureKey::Continuous { .. }) {
            channel_scale
        } else {
            1.0
        };
        (key, value * scale)
    }));
    inputs.sort_by(|left, right| left.0.cmp(&right.0));
    inputs
}

fn validate_prediction_update(
    update: &PredictionUpdate,
    semantic_state_count: usize,
) -> Result<(), ScaffoldContractError> {
    if update.target_digest == [0; 4]
        || update.error.len() != semantic_state_count
        || !(0.0..=1.0).contains(&update.mean_squared_error)
        || !(0.0..=1.0).contains(&update.mean_absolute_error)
        || update
            .error
            .iter()
            .any(|value| !(-1.0..=1.0).contains(value))
        || update.prediction.predicted_successor.len() != semantic_state_count
        || update
            .prediction
            .predicted_successor
            .iter()
            .any(|value| !(0.0..=1.0).contains(value))
    {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    update.prediction.source_state.validate_contract()?;
    update.prediction.motor_condition.validate_contract()?;
    update.prediction.category_coverage.validate_contract()?;
    let categories = update
        .prediction
        .motor_condition
        .features()
        .iter()
        .filter(|(key, _)| key.is_category())
        .count();
    if update.prediction.predictor_abi_version != GROUNDED_PREDICTOR_ABI_VERSION
        || update.prediction.semantic_state_abi != update.prediction.source_state.abi_version
        || update.prediction.source_state.len() != semantic_state_count
        || usize::from(update.prediction.category_coverage.modelled_categories)
            + usize::from(update.prediction.category_coverage.unmodelled_categories)
            != categories
    {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    Ok(())
}

fn bounded_unit(value: f32) -> f32 {
    (value / (1.0 + value.abs())).clamp(0.0, 1.0)
}

fn signed_unit(value: f32) -> f32 {
    0.5 + 0.5 * (value / (1.0 + value.abs()))
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

fn write_words(builder: &mut CanonicalDigestBuilder, words: [u64; 4]) {
    builder.write_sequence_len(words.len());
    for word in words {
        builder.write_u64(word);
    }
}
