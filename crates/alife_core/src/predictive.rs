//! Grounded, versioned prediction targets and factorized motor conditioning.

use serde::{Deserialize, Serialize};

use crate::{
    ActionId, CanonicalDigestBuilder, ChannelCommand, ExperienceSequenceId, MotorChannel,
    MotorCommandBundle, NormalizedScalar, OrganismId, ScaffoldContractError, Tick, Validate, Vec3f,
    MAX_MOTOR_CHANNELS, MAX_MOTOR_PAYLOAD_VALUES,
};

pub const PREDICTION_TARGET_SCHEMA_VERSION: u16 = 2;
pub const SEMANTIC_STATE_VECTOR_SCHEMA_VERSION: u16 = 1;
pub const SEMANTIC_STATE_VECTOR_ABI_V1: u16 = 1;
pub const JOINT_MOTOR_CONDITION_SCHEMA_VERSION: u16 = 1;
pub const JOINT_MOTOR_CONDITION_ABI_V1: u16 = 1;
pub const MAX_SEMANTIC_STATE_VALUES: usize = 32;
pub const MAX_SUCCESSOR_FEATURES: usize = MAX_SEMANTIC_STATE_VALUES;
pub const SUCCESSOR_FEATURE_ABI_V1: u16 = 1;
pub const DEFAULT_PREDICTOR_LEARNING_RATE: f32 = 0.25;

const MOTOR_FACTOR_BASE_FEATURES: usize = 15;
const MOTOR_PRIMITIVE_BIT_FEATURES: usize = u32::BITS as usize;
const MAX_MOTOR_FACTOR_FEATURES: usize = MOTOR_FACTOR_BASE_FEATURES + MOTOR_PRIMITIVE_BIT_FEATURES;

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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MotorChannelFactor {
    pub channel: MotorChannel,
    pub primitive: ActionId,
    pub intensity: f32,
    pub duration_ticks: u32,
    pub direction: Vec3f,
    pub stand_off_distance: f32,
    pub confidence: f32,
    pub payload_len: u8,
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
            payload_len: u8::try_from(command.payload.values.len())
                .map_err(|_| ScaffoldContractError::InvalidActionDecision)?,
            coordination_group: command.coordination_group,
        };
        factor.validate_contract()?;
        Ok(factor)
    }

    fn feature_values(self) -> [f32; MAX_MOTOR_FACTOR_FEATURES] {
        let primitive = self.primitive.raw();
        let base_features = [
            1.0,
            f32::from(self.channel.canonical_key()) / f32::from(u16::MAX),
            (primitive & 0xff) as f32 / f32::from(u8::MAX),
            ((primitive >> 8) & 0xff) as f32 / f32::from(u8::MAX),
            ((primitive >> 16) & 0xff) as f32 / f32::from(u8::MAX),
            ((primitive >> 24) & 0xff) as f32 / f32::from(u8::MAX),
            self.intensity,
            self.duration_ticks as f32 / (self.duration_ticks as f32 + 1.0),
            signed_unit(self.direction.x),
            signed_unit(self.direction.y),
            signed_unit(self.direction.z),
            bounded_unit(self.stand_off_distance),
            self.confidence,
            f32::from(self.payload_len) / MAX_MOTOR_PAYLOAD_VALUES as f32,
            f32::from(self.coordination_group) / f32::from(u8::MAX),
        ];
        let mut features = [0.0; MAX_MOTOR_FACTOR_FEATURES];
        features[..MOTOR_FACTOR_BASE_FEATURES].copy_from_slice(&base_features);
        // Primitive IDs are logical categories, not ordinal magnitudes. Signed
        // bits give the shared predictor a centered condition signal.
        for bit in 0..u32::BITS {
            let feature_index = MOTOR_FACTOR_BASE_FEATURES + bit as usize;
            features[feature_index] = if primitive & (1_u32 << bit) != 0 {
                1.0
            } else {
                -1.0
            };
        }
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
            || usize::from(self.payload_len) > MAX_MOTOR_PAYLOAD_VALUES
        {
            return Err(ScaffoldContractError::InvalidActionDecision);
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
    pub channels: Vec<MotorChannelFactor>,
}

impl JointMotorCondition {
    pub fn new(mut channels: Vec<MotorChannelFactor>) -> Result<Self, ScaffoldContractError> {
        channels.sort_by_key(|factor| factor.channel.canonical_key());
        let condition = Self {
            schema_version: JOINT_MOTOR_CONDITION_SCHEMA_VERSION,
            abi_version: JOINT_MOTOR_CONDITION_ABI_V1,
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

    pub fn feature_vector(&self) -> Vec<f32> {
        let mut features = vec![0.0; MAX_MOTOR_CHANNELS * MAX_MOTOR_FACTOR_FEATURES];
        for (index, factor) in self.channels.iter().enumerate() {
            let start = index * MAX_MOTOR_FACTOR_FEATURES;
            features[start..start + MAX_MOTOR_FACTOR_FEATURES]
                .copy_from_slice(&factor.feature_values());
        }
        features
    }

    /// A bounded signal that reports whether the condition carries usable
    /// motor information. It is diagnostic, not a reward or credit value.
    pub fn information_score(&self) -> Result<f32, ScaffoldContractError> {
        self.validate_contract()?;
        let features = self.feature_vector();
        let score = features.iter().map(|value| value.abs()).sum::<f32>() / features.len() as f32;
        if score.is_finite() {
            Ok(score.clamp(0.0, 1.0))
        } else {
            Err(ScaffoldContractError::NonFiniteFloat)
        }
    }

    pub fn canonical_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        self.validate_contract()?;
        let mut builder = CanonicalDigestBuilder::new(b"ALIFE-V11-JOINT-MOTOR-CONDITION-V1");
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
            builder.write_u8(factor.payload_len);
            builder.write_u8(factor.coordination_group);
        }
        Ok(builder.finish256())
    }
}

impl Validate for JointMotorCondition {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != JOINT_MOTOR_CONDITION_SCHEMA_VERSION
            || self.abi_version != JOINT_MOTOR_CONDITION_ABI_V1
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
    pub representation_variance: f32,
    pub action_sensitivity_score: f32,
    pub successor_separability_score: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuccessorPrediction {
    pub source_digest: [u64; 4],
    pub source_state: SemanticStateVector,
    pub motor_condition: JointMotorCondition,
    pub semantic_state_abi: u16,
    pub predicted_successor: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PredictionUpdate {
    pub prediction: SuccessorPrediction,
    pub target_digest: [u64; 4],
    pub error: Vec<f32>,
    pub mean_squared_error: f32,
    pub mean_absolute_error: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroundedSuccessorPredictor {
    semantic_state_abi: u16,
    semantic_state_count: usize,
    motor_condition_abi: u16,
    input_feature_count: usize,
    learning_rate: f32,
    weights: Vec<f32>,
    last_update: Option<PredictionUpdate>,
}

impl Default for GroundedSuccessorPredictor {
    fn default() -> Self {
        Self {
            semantic_state_abi: 0,
            semantic_state_count: 0,
            motor_condition_abi: 0,
            input_feature_count: 0,
            learning_rate: DEFAULT_PREDICTOR_LEARNING_RATE,
            weights: Vec::new(),
            last_update: None,
        }
    }
}

impl GroundedSuccessorPredictor {
    pub fn with_learning_rate(learning_rate: f32) -> Result<Self, ScaffoldContractError> {
        if !learning_rate.is_finite() || !(0.0..=1.0).contains(&learning_rate) {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if learning_rate == 0.0 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(Self {
            learning_rate,
            ..Self::default()
        })
    }

    pub fn predict(
        &self,
        source_state: &SemanticStateVector,
        motor_condition: &JointMotorCondition,
    ) -> Result<SuccessorPrediction, ScaffoldContractError> {
        source_state.validate_contract()?;
        motor_condition.validate_contract()?;
        self.validate_runtime_shape(source_state, motor_condition)?;
        let semantic_state_abi = if self.semantic_state_abi == 0 {
            source_state.abi_version
        } else {
            self.semantic_state_abi
        };
        let inputs = predictor_inputs(source_state, motor_condition);
        let mut predicted_successor = vec![0.0; source_state.len()];
        if !self.weights.is_empty() {
            for (feature_index, predicted) in predicted_successor.iter_mut().enumerate() {
                let offset = feature_index * inputs.len();
                let raw_prediction = self.weights[offset..offset + inputs.len()]
                    .iter()
                    .zip(&inputs)
                    .map(|(weight, input)| weight * input)
                    .sum::<f32>();
                *predicted = raw_prediction.clamp(0.0, 1.0);
            }
        }
        Ok(SuccessorPrediction {
            source_digest: [0; 4],
            source_state: source_state.clone(),
            motor_condition: motor_condition.clone(),
            semantic_state_abi,
            predicted_successor,
        })
    }

    pub fn observe(
        &mut self,
        receipt: &PredictionTargetReceipt,
    ) -> Result<PredictionUpdate, ScaffoldContractError> {
        receipt.validate_contract()?;
        self.configure_for(receipt)?;
        let prediction = self.predict(&receipt.source_state, &receipt.motor_condition)?;
        let inputs = predictor_inputs(&receipt.source_state, &receipt.motor_condition);
        let input_energy = inputs.iter().map(|input| input * input).sum::<f32>();
        let normalized_step = self.learning_rate / input_energy.max(f32::EPSILON);
        let mut error = Vec::with_capacity(receipt.target_state.len());
        let mut squared_error = 0.0;
        let mut absolute_error = 0.0;
        for (feature_index, (predicted, target)) in prediction
            .predicted_successor
            .iter()
            .zip(&receipt.target_state.values)
            .enumerate()
        {
            let feature_error = *target - *predicted;
            error.push(feature_error);
            squared_error += feature_error * feature_error;
            absolute_error += feature_error.abs();

            let offset = feature_index * inputs.len();
            for (weight, input) in self.weights[offset..offset + inputs.len()]
                .iter_mut()
                .zip(&inputs)
            {
                *weight += normalized_step * feature_error * input;
            }
        }

        let feature_count = receipt.target_state.len() as f32;
        let update = PredictionUpdate {
            prediction: SuccessorPrediction {
                source_digest: receipt.source_digest,
                ..prediction
            },
            target_digest: receipt.target_digest,
            error,
            mean_squared_error: squared_error / feature_count,
            mean_absolute_error: absolute_error / feature_count,
        };
        self.last_update = Some(update.clone());
        Ok(update)
    }

    pub fn last_update(&self) -> Option<&PredictionUpdate> {
        self.last_update.as_ref()
    }

    fn configure_for(
        &mut self,
        receipt: &PredictionTargetReceipt,
    ) -> Result<(), ScaffoldContractError> {
        let state_count = receipt.source_state.len();
        let input_feature_count = 1 + state_count + MAX_MOTOR_CHANNELS * MAX_MOTOR_FACTOR_FEATURES;
        if self.semantic_state_abi == 0 {
            self.semantic_state_abi = receipt.source_state.abi_version;
            self.semantic_state_count = state_count;
            self.motor_condition_abi = receipt.motor_condition.abi_version;
            self.input_feature_count = input_feature_count;
            self.weights = vec![0.0; state_count * input_feature_count];
        }
        self.validate_runtime_shape(&receipt.source_state, &receipt.motor_condition)
    }

    fn validate_runtime_shape(
        &self,
        source_state: &SemanticStateVector,
        motor_condition: &JointMotorCondition,
    ) -> Result<(), ScaffoldContractError> {
        if self.semantic_state_abi != 0
            && (self.semantic_state_abi != source_state.abi_version
                || self.semantic_state_count != source_state.len()
                || self.motor_condition_abi != motor_condition.abi_version
                || self.input_feature_count
                    != 1 + source_state.len() + MAX_MOTOR_CHANNELS * MAX_MOTOR_FACTOR_FEATURES)
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
}

impl Validate for GroundedSuccessorPredictor {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if !self.learning_rate.is_finite()
            || !(0.0..=1.0).contains(&self.learning_rate)
            || self.learning_rate == 0.0
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if self.semantic_state_abi == 0 {
            if self.semantic_state_count != 0
                || self.motor_condition_abi != 0
                || self.input_feature_count != 0
                || !self.weights.is_empty()
                || self.last_update.is_some()
            {
                return Err(ScaffoldContractError::InvalidDecisionEvidence);
            }
            return Ok(());
        }
        if self.semantic_state_abi != SEMANTIC_STATE_VECTOR_ABI_V1
            || self.semantic_state_count < 2
            || self.semantic_state_count > MAX_SEMANTIC_STATE_VALUES
            || self.motor_condition_abi != JOINT_MOTOR_CONDITION_ABI_V1
            || self.input_feature_count
                != 1 + self.semantic_state_count + MAX_MOTOR_CHANNELS * MAX_MOTOR_FACTOR_FEATURES
            || self.weights.len() != self.semantic_state_count * self.input_feature_count
            || self.weights.iter().any(|weight| !weight.is_finite())
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        if let Some(update) = &self.last_update {
            validate_prediction_update(update, self.semantic_state_count)?;
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
            representation_variance: target_state.variance()?,
            action_sensitivity_score: motor_condition.information_score()?,
            successor_separability_score: source_state.mean_absolute_distance(&target_state)?,
        };
        receipt.validate_contract()?;
        Ok(receipt)
    }

    pub fn canonical_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        self.validate_contract()?;
        let mut builder = CanonicalDigestBuilder::new(b"ALIFE-V11-PREDICTION-TARGET-V2");
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
        builder.write_f32(self.representation_variance)?;
        builder.write_f32(self.action_sensitivity_score)?;
        builder.write_f32(self.successor_separability_score)?;
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
            || !self.representation_variance.is_finite()
            || !self.action_sensitivity_score.is_finite()
            || !self.successor_separability_score.is_finite()
            || !(0.0..=1.0).contains(&self.representation_variance)
            || !(0.0..=1.0).contains(&self.action_sensitivity_score)
            || !(0.0..=1.0).contains(&self.successor_separability_score)
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
            || self.representation_variance != self.target_state.variance()?
            || self.action_sensitivity_score != self.motor_condition.information_score()?
            || self.successor_separability_score
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
) -> Vec<f32> {
    let mut inputs =
        Vec::with_capacity(1 + source_state.len() + MAX_MOTOR_CHANNELS * MAX_MOTOR_FACTOR_FEATURES);
    inputs.push(1.0);
    inputs.extend(source_state.values.iter().copied());
    inputs.extend(motor_condition.feature_vector());
    inputs
}

fn validate_prediction_update(
    update: &PredictionUpdate,
    semantic_state_count: usize,
) -> Result<(), ScaffoldContractError> {
    if update.target_digest == [0; 4]
        || update.error.len() != semantic_state_count
        || !update.mean_squared_error.is_finite()
        || !update.mean_absolute_error.is_finite()
        || update.error.iter().any(|value| !value.is_finite())
        || update.prediction.predicted_successor.len() != semantic_state_count
        || update
            .prediction
            .predicted_successor
            .iter()
            .any(|value| !value.is_finite())
    {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    update.prediction.source_state.validate_contract()?;
    update.prediction.motor_condition.validate_contract()?;
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
