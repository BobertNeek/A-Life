//! Grounded successor prediction with explicit semantic and motor contracts.

use serde::{Deserialize, Serialize};

use crate::{
    ActionId, CanonicalDigestBuilder, ChannelCommand, ExperienceSequenceId, MotorChannel,
    MotorCommandBundle, ScaffoldContractError, Tick, Validate, Vec3f, MAX_MOTOR_CHANNELS,
    MAX_MOTOR_PAYLOAD_VALUES,
};

pub use crate::predictor_contract::{
    CategoricalMotorPrimitive, GroundedOutcomeFeatures, GroundedOutcomeMeaning, MotorFamily,
    MotorPrimitiveEmbedding, SemanticStateMeaning, SemanticStateSchemaV2,
    SemanticStateTransition, SemanticStateVector, GROUNDED_OUTCOME_ABI_V1,
    GROUNDED_OUTCOME_FEATURE_COUNT, GROUNDED_OUTCOME_MEANINGS_V1, GROUNDED_OUTCOME_SCHEMA_VERSION,
    MAX_PRIMITIVE_EMBEDDINGS, MAX_SEMANTIC_STATE_VALUES, MOTOR_CATEGORY_ABI_V1,
    MOTOR_CATEGORY_SCHEMA_VERSION, MOTOR_PRIMITIVE_EMBEDDING_DIM, SEMANTIC_STATE_MEANINGS_V2,
    SEMANTIC_STATE_VECTOR_ABI_V1, SEMANTIC_STATE_VECTOR_ABI_V2,
    SEMANTIC_STATE_VECTOR_SCHEMA_VERSION,
};

pub const PREDICTION_TARGET_SCHEMA_VERSION: u16 = 3;
pub const PREDICTOR_STATE_SCHEMA_VERSION: u16 = 2;
pub const PREDICTOR_STATE_ABI_VERSION: u16 = 1;
pub const JOINT_MOTOR_CONDITION_SCHEMA_VERSION: u16 = 2;
pub const JOINT_MOTOR_CONDITION_ABI_V1: u16 = 2;
pub const MAX_SUCCESSOR_FEATURES: usize = MAX_SEMANTIC_STATE_VALUES;
pub const SUCCESSOR_FEATURE_ABI_V1: u16 = 2;
pub const DEFAULT_PREDICTOR_LEARNING_RATE: f32 = 0.25;
pub const DEFAULT_INTERACTION_RANK: usize = 4;
pub const MAX_INTERACTION_RANK: usize = 8;
pub const MAX_PREDICTION_SHORTLIST: usize = 8;
pub const MIN_MATERIAL_MOTOR_DISTANCE: f32 = 0.125;
pub const MIN_MATERIAL_SUCCESSOR_DISTANCE: f32 = 0.125;

const MOTOR_FACTOR_FEATURES: usize = 1 + MotorFamily::COUNT + MOTOR_PRIMITIVE_EMBEDDING_DIM + 9;
const MAX_MODEL_PARAMETERS: usize = 65_536;

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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MotorChannelFactor {
    pub channel: MotorChannel,
    /// Action identity is retained for world binding and categorical lookup.
    /// It is never fed to the predictor as a numeric feature.
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

    pub fn family(self) -> MotorFamily {
        MotorFamily::from_channel(self.channel)
    }

    pub fn categorical_primitive(self) -> CategoricalMotorPrimitive {
        CategoricalMotorPrimitive::from_action(self.family(), self.primitive)
    }

    fn feature_values(
        self,
        embedding: [f32; MOTOR_PRIMITIVE_EMBEDDING_DIM],
    ) -> [f32; MOTOR_FACTOR_FEATURES] {
        let mut features = [0.0; MOTOR_FACTOR_FEATURES];
        features[0] = 1.0;
        let family = self.family().one_hot();
        features[1..1 + MotorFamily::COUNT].copy_from_slice(&family);
        let embedding_start = 1 + MotorFamily::COUNT;
        features[embedding_start..embedding_start + MOTOR_PRIMITIVE_EMBEDDING_DIM]
            .copy_from_slice(&embedding);
        let start = embedding_start + MOTOR_PRIMITIVE_EMBEDDING_DIM;
        features[start] = self.intensity;
        features[start + 1] = self.duration_ticks as f32 / (self.duration_ticks as f32 + 1.0);
        features[start + 2] = signed_unit(self.direction.x);
        features[start + 3] = signed_unit(self.direction.y);
        features[start + 4] = signed_unit(self.direction.z);
        features[start + 5] = bounded_unit(self.stand_off_distance);
        features[start + 6] = self.confidence;
        features[start + 7] = f32::from(self.payload_len) / MAX_MOTOR_PAYLOAD_VALUES as f32;
        features[start + 8] = f32::from(self.coordination_group) / f32::from(u8::MAX);
        features
    }
}

impl Validate for MotorChannelFactor {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.primitive.validate()?;
        if !self.intensity.is_finite()
            || !(0.0..=1.0).contains(&self.intensity)
            || self.duration_ticks == 0
            || !self.stand_off_distance.is_finite()
            || self.stand_off_distance < 0.0
            || !(0.0..=1.0).contains(&self.confidence)
            || usize::from(self.payload_len) > MAX_MOTOR_PAYLOAD_VALUES
        {
            return Err(ScaffoldContractError::InvalidActionDecision);
        }
        self.direction.validate()?;
        Ok(())
    }
}

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
        self.feature_vector_with(|factor| seeded_embedding(factor.categorical_primitive()))
    }

    fn feature_vector_with<F>(&self, mut embedding: F) -> Vec<f32>
    where
        F: FnMut(MotorChannelFactor) -> [f32; MOTOR_PRIMITIVE_EMBEDDING_DIM],
    {
        let mut features = vec![0.0; MAX_MOTOR_CHANNELS * MOTOR_FACTOR_FEATURES];
        for (index, factor) in self.channels.iter().enumerate() {
            let start = index * MOTOR_FACTOR_FEATURES;
            features[start..start + MOTOR_FACTOR_FEATURES]
                .copy_from_slice(&factor.feature_values(embedding(*factor)));
        }
        features
    }

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

    /// Categorical mismatches are identity changes, not distances between IDs.
    pub fn material_distance(&self, other: &Self) -> Result<f32, ScaffoldContractError> {
        self.validate_contract()?;
        other.validate_contract()?;
        let mut difference = 0.0;
        let mut terms: f32 = 0.0;
        for index in 0..MAX_MOTOR_CHANNELS {
            let left = self.channels.get(index);
            let right = other.channels.get(index);
            match (left, right) {
                (None, None) => {}
                (Some(_), None) | (None, Some(_)) => {
                    difference += 1.0;
                    terms += 1.0;
                }
                (Some(left), Some(right)) => {
                    terms += 2.0;
                    if left.family() != right.family() {
                        difference += 1.0;
                    }
                    if left.categorical_primitive() != right.categorical_primitive() {
                        difference += 1.0;
                    }
                    let continuous_left = left.feature_values([0.0; MOTOR_PRIMITIVE_EMBEDDING_DIM]);
                    let continuous_right = right.feature_values([0.0; MOTOR_PRIMITIVE_EMBEDDING_DIM]);
                    difference += continuous_left
                        .iter()
                        .zip(continuous_right.iter())
                        .skip(1 + MotorFamily::COUNT + MOTOR_PRIMITIVE_EMBEDDING_DIM)
                        .map(|(a, b)| (*a - *b).abs())
                        .sum::<f32>();
                    terms += 9.0;
                }
            }
        }
        Ok((difference / terms.max(1.0)).clamp(0.0, 1.0))
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
    pub organism_id: crate::OrganismId,
    pub experience_sequence: ExperienceSequenceId,
    pub decision: ActionId,
    pub world_tick: Tick,
    /// Identity evidence only. It is never a predictor feature.
    pub source_digest: [u64; 4],
    pub source_state: SemanticStateVector,
    pub motor_condition: JointMotorCondition,
    pub target_digest: [u64; 4],
    pub target_state: SemanticStateVector,
    #[serde(default)]
    pub outcome_features: GroundedOutcomeFeatures,
    /// Zero means that no fixed-state comparison was supplied for this event.
    pub action_sensitivity_score: f32,
    /// Zero means that no pairwise successor comparison was supplied here.
    pub successor_separability_score: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuccessorPrediction {
    pub source_digest: [u64; 4],
    pub source_state: SemanticStateVector,
    pub motor_condition: JointMotorCondition,
    pub semantic_state_abi: u16,
    pub predicted_successor: Vec<f32>,
    pub uncertainty: f32,
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
pub struct ActionSensitivityEvidence {
    pub schema_version: u16,
    pub abi_version: u16,
    pub source_digest: [u64; 4],
    pub first_motor_digest: [u64; 4],
    pub second_motor_digest: [u64; 4],
    pub motor_bundle_distance: f32,
    pub predicted_successor_distance: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuccessorSeparabilityEvidence {
    pub schema_version: u16,
    pub abi_version: u16,
    pub first_successor_digest: [u64; 4],
    pub second_successor_digest: [u64; 4],
    pub successor_distance: f32,
    pub materially_different: bool,
}

impl Validate for ActionSensitivityEvidence {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != PREDICTOR_STATE_SCHEMA_VERSION
            || self.abi_version != PREDICTOR_STATE_ABI_VERSION
            || self.source_digest == [0; 4]
            || self.first_motor_digest == [0; 4]
            || self.second_motor_digest == [0; 4]
            || self.first_motor_digest == self.second_motor_digest
            || !self.motor_bundle_distance.is_finite()
            || !self.predicted_successor_distance.is_finite()
            || !(0.0..=1.0).contains(&self.motor_bundle_distance)
            || !(0.0..=1.0).contains(&self.predicted_successor_distance)
            || self.motor_bundle_distance < MIN_MATERIAL_MOTOR_DISTANCE
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
}

impl Validate for SuccessorSeparabilityEvidence {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != PREDICTOR_STATE_SCHEMA_VERSION
            || self.abi_version != PREDICTOR_STATE_ABI_VERSION
            || self.first_successor_digest == [0; 4]
            || self.second_successor_digest == [0; 4]
            || !self.successor_distance.is_finite()
            || !(0.0..=1.0).contains(&self.successor_distance)
            || self.materially_different
                != (self.successor_distance >= MIN_MATERIAL_SUCCESSOR_DISTANCE)
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroundedSuccessorPredictor {
    pub schema_version: u16,
    pub abi_version: u16,
    semantic_state_abi: u16,
    semantic_state_count: usize,
    motor_condition_abi: u16,
    input_feature_count: usize,
    interaction_rank: usize,
    learning_rate: f32,
    weights: Vec<f32>,
    source_projection: Vec<f32>,
    motor_projection: Vec<f32>,
    interaction_weights: Vec<f32>,
    gate_bias: Vec<f32>,
    primitive_embeddings: Vec<MotorPrimitiveEmbedding>,
    observation_count: u32,
    last_update: Option<PredictionUpdate>,
}

impl Default for GroundedSuccessorPredictor {
    fn default() -> Self {
        Self {
            schema_version: PREDICTOR_STATE_SCHEMA_VERSION,
            abi_version: PREDICTOR_STATE_ABI_VERSION,
            semantic_state_abi: 0,
            semantic_state_count: 0,
            motor_condition_abi: 0,
            input_feature_count: 0,
            interaction_rank: 0,
            learning_rate: DEFAULT_PREDICTOR_LEARNING_RATE,
            weights: Vec::new(),
            source_projection: Vec::new(),
            motor_projection: Vec::new(),
            interaction_weights: Vec::new(),
            gate_bias: Vec::new(),
            primitive_embeddings: Vec::new(),
            observation_count: 0,
            last_update: None,
        }
    }
}

impl GroundedSuccessorPredictor {
    pub fn with_learning_rate(learning_rate: f32) -> Result<Self, ScaffoldContractError> {
        if !learning_rate.is_finite() || !(0.0..=1.0).contains(&learning_rate) || learning_rate == 0.0 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(Self { learning_rate, ..Self::default() })
    }

    pub fn predict(
        &self,
        source_state: &SemanticStateVector,
        motor_condition: &JointMotorCondition,
    ) -> Result<SuccessorPrediction, ScaffoldContractError> {
        source_state.validate_contract()?;
        motor_condition.validate_contract()?;
        self.validate_runtime_shape(source_state, motor_condition)?;
        let inputs = self.predictor_inputs(source_state, motor_condition);
        let predicted_successor = if self.semantic_state_abi == 0 {
            cold_start_prediction(source_state, motor_condition)
        } else {
            let interaction = self.interaction_features(source_state, &inputs);
            (0..source_state.len())
                .map(|feature_index| {
                    let offset = feature_index * inputs.len();
                    let direct = self.weights[offset..offset + inputs.len()]
                        .iter()
                        .zip(&inputs)
                        .map(|(weight, input)| weight * input)
                        .sum::<f32>();
                    let interaction_value = self.interaction_weights
                        [feature_index * self.interaction_rank..(feature_index + 1) * self.interaction_rank]
                        .iter()
                        .zip(&interaction)
                        .map(|(weight, value)| weight * value)
                        .sum::<f32>();
                    sigmoid(direct + interaction_value)
                })
                .collect()
        };
        Ok(SuccessorPrediction {
            source_digest: [0; 4],
            source_state: source_state.clone(),
            motor_condition: motor_condition.clone(),
            semantic_state_abi: if self.semantic_state_abi == 0 {
                source_state.abi_version
            } else {
                self.semantic_state_abi
            },
            predicted_successor,
            uncertainty: if self.observation_count == 0 {
                1.0
            } else {
                (1.0 / (self.observation_count as f32 + 1.0)).clamp(0.05, 1.0)
            },
        })
    }

    /// Candidate-specific predecision consequence predictions.  This returns
    /// facts and uncertainty in caller order.  It never returns desirability
    /// or selects an action.
    pub fn predict_candidates(
        &self,
        source_state: &SemanticStateVector,
        candidates: &[JointMotorCondition],
    ) -> Result<Vec<SuccessorPrediction>, ScaffoldContractError> {
        if candidates.is_empty() || candidates.len() > MAX_PREDICTION_SHORTLIST {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        candidates
            .iter()
            .map(|candidate| self.predict(source_state, candidate))
            .collect()
    }

    pub fn observe(
        &mut self,
        receipt: &PredictionTargetReceipt,
    ) -> Result<PredictionUpdate, ScaffoldContractError> {
        receipt.validate_contract()?;
        self.configure_for(receipt)?;
        let prediction = self.predict(&receipt.source_state, &receipt.motor_condition)?;
        let inputs = self.predictor_inputs(&receipt.source_state, &receipt.motor_condition);
        let interaction = self.interaction_features(&receipt.source_state, &inputs);
        let derivative_step = self.learning_rate / inputs.len().max(1) as f32;
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
                *weight = (*weight + derivative_step * feature_error * input).clamp(-4.0, 4.0);
            }
            let interaction_offset = feature_index * self.interaction_rank;
            for (weight, value) in self.interaction_weights
                [interaction_offset..interaction_offset + self.interaction_rank]
                .iter_mut()
                .zip(&interaction)
            {
                *weight = (*weight + derivative_step * feature_error * value).clamp(-4.0, 4.0);
            }
        }
        self.update_embeddings(
            &receipt.motor_condition,
            error.iter().copied().sum::<f32>() / error.len() as f32,
        );
        self.observation_count = self.observation_count.saturating_add(1);
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

    /// Compares predictions at one fixed state for materially different
    /// bundles.  The result is an information diagnostic, never a policy
    /// score.
    pub fn action_sensitivity(
        &self,
        source_state: &SemanticStateVector,
        first: &JointMotorCondition,
        second: &JointMotorCondition,
    ) -> Result<ActionSensitivityEvidence, ScaffoldContractError> {
        source_state.validate_contract()?;
        let motor_bundle_distance = first.material_distance(second)?;
        if motor_bundle_distance < MIN_MATERIAL_MOTOR_DISTANCE {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let first_prediction = self.predict(source_state, first)?;
        let second_prediction = self.predict(source_state, second)?;
        let predicted_successor_distance = mean_absolute_values(
            &first_prediction.predicted_successor,
            &second_prediction.predicted_successor,
        )?;
        let evidence = ActionSensitivityEvidence {
            schema_version: PREDICTOR_STATE_SCHEMA_VERSION,
            abi_version: PREDICTOR_STATE_ABI_VERSION,
            source_digest: source_state.canonical_digest()?,
            first_motor_digest: first.canonical_digest()?,
            second_motor_digest: second.canonical_digest()?,
            motor_bundle_distance,
            predicted_successor_distance,
        };
        evidence.validate_contract()?;
        Ok(evidence)
    }

    pub fn fixed_state_action_sensitivity(
        &self,
        source_state: &SemanticStateVector,
        first: &JointMotorCondition,
        second: &JointMotorCondition,
    ) -> Result<f32, ScaffoldContractError> {
        Ok(self
            .action_sensitivity(source_state, first, second)?
            .predicted_successor_distance)
    }

    pub fn successor_separability(
        &self,
        first: &SemanticStateVector,
        second: &SemanticStateVector,
    ) -> Result<SuccessorSeparabilityEvidence, ScaffoldContractError> {
        compare_successor_states(first, second)
    }

    fn configure_for(
        &mut self,
        receipt: &PredictionTargetReceipt,
    ) -> Result<(), ScaffoldContractError> {
        let state_count = receipt.source_state.len();
        let input_feature_count = 1 + state_count + MAX_MOTOR_CHANNELS * MOTOR_FACTOR_FEATURES;
        if self.semantic_state_abi == 0 {
            self.semantic_state_abi = receipt.source_state.abi_version;
            self.semantic_state_count = state_count;
            self.motor_condition_abi = receipt.motor_condition.abi_version;
            self.input_feature_count = input_feature_count;
            self.interaction_rank = DEFAULT_INTERACTION_RANK;
            self.weights = vec![0.0; state_count * input_feature_count];
            self.source_projection = vec![0.0; self.interaction_rank * state_count];
            self.motor_projection =
                vec![0.0; self.interaction_rank * MAX_MOTOR_CHANNELS * MOTOR_FACTOR_FEATURES];
            self.interaction_weights = vec![0.0; state_count * self.interaction_rank];
            self.gate_bias = vec![0.0; self.interaction_rank];
            seed_model_parameters(self);
        }
        self.ensure_embeddings(&receipt.motor_condition)?;
        self.validate_runtime_shape(&receipt.source_state, &receipt.motor_condition)
    }

    fn ensure_embeddings(
        &mut self,
        condition: &JointMotorCondition,
    ) -> Result<(), ScaffoldContractError> {
        for factor in &condition.channels {
            let primitive = factor.categorical_primitive();
            if !self
                .primitive_embeddings
                .iter()
                .any(|embedding| embedding.primitive == primitive)
            {
                if self.primitive_embeddings.len() >= MAX_PRIMITIVE_EMBEDDINGS {
                    return Err(ScaffoldContractError::InvalidDecisionEvidence);
                }
                self.primitive_embeddings.push(MotorPrimitiveEmbedding::new(
                    primitive,
                    seeded_embedding(primitive),
                )?);
            }
        }
        self.primitive_embeddings
            .sort_by_key(|embedding| embedding.primitive.identity);
        Ok(())
    }

    fn update_embeddings(&mut self, condition: &JointMotorCondition, error: f32) {
        for factor in &condition.channels {
            let primitive = factor.categorical_primitive();
            if let Some(embedding) = self
                .primitive_embeddings
                .iter_mut()
                .find(|embedding| embedding.primitive == primitive)
            {
                for value in &mut embedding.values {
                    *value = (*value + self.learning_rate * error * 0.02).clamp(-1.0, 1.0);
                }
            }
        }
    }

    fn predictor_inputs(
        &self,
        source_state: &SemanticStateVector,
        condition: &JointMotorCondition,
    ) -> Vec<f32> {
        let mut inputs = Vec::with_capacity(
            1 + source_state.len() + MAX_MOTOR_CHANNELS * MOTOR_FACTOR_FEATURES,
        );
        inputs.push(1.0);
        inputs.extend(source_state.values.iter().copied());
        inputs.extend(condition.feature_vector_with(|factor| self.embedding_for(factor)));
        inputs
    }

    fn embedding_for(&self, factor: MotorChannelFactor) -> [f32; MOTOR_PRIMITIVE_EMBEDDING_DIM] {
        let primitive = factor.categorical_primitive();
        self.primitive_embeddings
            .iter()
            .find(|embedding| embedding.primitive == primitive)
            .map(|embedding| embedding.values)
            .unwrap_or_else(|| seeded_embedding(primitive))
    }

    fn interaction_features(
        &self,
        source_state: &SemanticStateVector,
        inputs: &[f32],
    ) -> Vec<f32> {
        if self.interaction_rank == 0 {
            return Vec::new();
        }
        let motor_offset = 1 + source_state.len();
        let motor = &inputs[motor_offset..];
        (0..self.interaction_rank)
            .map(|rank| {
                let source_projection = dot(
                    &self.source_projection
                        [rank * source_state.len()..(rank + 1) * source_state.len()],
                    &source_state.values,
                )
                .tanh();
                let motor_projection = dot(
                    &self.motor_projection[rank * motor.len()..(rank + 1) * motor.len()],
                    motor,
                )
                .tanh();
                let gate = sigmoid(self.gate_bias[rank] + source_projection * motor_projection);
                source_projection * motor_projection * gate
            })
            .collect()
    }

    fn validate_runtime_shape(
        &self,
        source_state: &SemanticStateVector,
        motor_condition: &JointMotorCondition,
    ) -> Result<(), ScaffoldContractError> {
        if self.semantic_state_abi == 0 {
            return Ok(());
        }
        if self.semantic_state_abi != source_state.abi_version
            || self.semantic_state_count != source_state.len()
            || self.motor_condition_abi != motor_condition.abi_version
            || self.input_feature_count
                != 1 + source_state.len() + MAX_MOTOR_CHANNELS * MOTOR_FACTOR_FEATURES
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
}

impl Validate for GroundedSuccessorPredictor {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != PREDICTOR_STATE_SCHEMA_VERSION
            || self.abi_version != PREDICTOR_STATE_ABI_VERSION
            || !self.learning_rate.is_finite()
            || !(0.0..=1.0).contains(&self.learning_rate)
            || self.learning_rate == 0.0
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        if self.semantic_state_abi == 0 {
            if self.semantic_state_count != 0
                || self.motor_condition_abi != 0
                || self.input_feature_count != 0
                || self.interaction_rank != 0
                || !self.weights.is_empty()
                || !self.source_projection.is_empty()
                || !self.motor_projection.is_empty()
                || !self.interaction_weights.is_empty()
                || !self.gate_bias.is_empty()
                || !self.primitive_embeddings.is_empty()
                || self.observation_count != 0
                || self.last_update.is_some()
            {
                return Err(ScaffoldContractError::InvalidDecisionEvidence);
            }
            return Ok(());
        }
        let expected_input = 1 + self.semantic_state_count + MAX_MOTOR_CHANNELS * MOTOR_FACTOR_FEATURES;
        let parameter_count = self.weights.len()
            + self.source_projection.len()
            + self.motor_projection.len()
            + self.interaction_weights.len()
            + self.gate_bias.len();
        if self.semantic_state_abi != SEMANTIC_STATE_VECTOR_ABI_V2
            || !(2..=MAX_SEMANTIC_STATE_VALUES).contains(&self.semantic_state_count)
            || self.motor_condition_abi != JOINT_MOTOR_CONDITION_ABI_V1
            || self.interaction_rank == 0
            || self.interaction_rank > MAX_INTERACTION_RANK
            || self.input_feature_count != expected_input
            || self.weights.len() != self.semantic_state_count * expected_input
            || self.source_projection.len() != self.interaction_rank * self.semantic_state_count
            || self.motor_projection.len()
                != self.interaction_rank * MAX_MOTOR_CHANNELS * MOTOR_FACTOR_FEATURES
            || self.interaction_weights.len() != self.semantic_state_count * self.interaction_rank
            || self.gate_bias.len() != self.interaction_rank
            || parameter_count > MAX_MODEL_PARAMETERS
            || self.primitive_embeddings.len() > MAX_PRIMITIVE_EMBEDDINGS
            || self
                .primitive_embeddings
                .windows(2)
                .any(|pair| pair[0].primitive.identity >= pair[1].primitive.identity)
            || self
                .weights
                .iter()
                .chain(self.source_projection.iter())
                .chain(self.motor_projection.iter())
                .chain(self.interaction_weights.iter())
                .chain(self.gate_bias.iter())
                .any(|value| !value.is_finite() || value.abs() > 4.0)
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        for embedding in &self.primitive_embeddings {
            embedding.validate_contract()?;
        }
        if let Some(update) = &self.last_update {
            validate_prediction_update(update, self.semantic_state_count)?;
        }
        Ok(())
    }
}

impl PredictionTargetReceipt {
    pub fn for_successor(
        organism_id: crate::OrganismId,
        experience_sequence: ExperienceSequenceId,
        decision: ActionId,
        world_tick: Tick,
        source_digest: [u64; 4],
        source_state: SemanticStateVector,
        motor_condition: JointMotorCondition,
        target_state: SemanticStateVector,
    ) -> Result<Self, ScaffoldContractError> {
        Self::for_successor_with_outcome(
            organism_id,
            experience_sequence,
            decision,
            world_tick,
            source_digest,
            source_state,
            motor_condition,
            target_state,
            GroundedOutcomeFeatures::unknown(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn for_successor_with_outcome(
        organism_id: crate::OrganismId,
        experience_sequence: ExperienceSequenceId,
        decision: ActionId,
        world_tick: Tick,
        source_digest: [u64; 4],
        source_state: SemanticStateVector,
        motor_condition: JointMotorCondition,
        target_state: SemanticStateVector,
        outcome_features: GroundedOutcomeFeatures,
    ) -> Result<Self, ScaffoldContractError> {
        let transition = SemanticStateTransition::new(source_state, target_state)?;
        if source_digest == [0; 4] {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        outcome_features.validate_contract()?;
        let target_digest = transition.post_state.canonical_digest()?;
        let receipt = Self {
            schema_version: PREDICTION_TARGET_SCHEMA_VERSION,
            organism_id,
            experience_sequence,
            decision,
            world_tick,
            source_digest,
            source_state: transition.pre_state,
            motor_condition,
            target_digest,
            target_state: transition.post_state,
            outcome_features,
            action_sensitivity_score: 0.0,
            successor_separability_score: 0.0,
        };
        receipt.validate_contract()?;
        Ok(receipt)
    }

    pub fn with_information_diagnostics(
        mut self,
        action_sensitivity_score: f32,
        successor_separability_score: f32,
    ) -> Result<Self, ScaffoldContractError> {
        self.action_sensitivity_score = action_sensitivity_score;
        self.successor_separability_score = successor_separability_score;
        self.validate_contract()?;
        Ok(self)
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
        for word in self.outcome_features.canonical_digest()? {
            builder.write_u64(word);
        }
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
            || !self.action_sensitivity_score.is_finite()
            || !self.successor_separability_score.is_finite()
            || !(0.0..=1.0).contains(&self.action_sensitivity_score)
            || !(0.0..=1.0).contains(&self.successor_separability_score)
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        self.organism_id.validate()?;
        self.experience_sequence.validate()?;
        self.decision.validate()?;
        let transition = SemanticStateTransition::new(
            self.source_state.clone(),
            self.target_state.clone(),
        )?;
        self.motor_condition.validate_contract()?;
        self.outcome_features.validate_contract()?;
        if self.target_state.canonical_digest()? != self.target_digest {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        transition.validate_contract()
    }
}

pub fn compare_successor_states(
    first: &SemanticStateVector,
    second: &SemanticStateVector,
) -> Result<SuccessorSeparabilityEvidence, ScaffoldContractError> {
    first.validate_contract()?;
    second.validate_contract()?;
    let successor_distance = first.mean_absolute_distance(second)?;
    let evidence = SuccessorSeparabilityEvidence {
        schema_version: PREDICTOR_STATE_SCHEMA_VERSION,
        abi_version: PREDICTOR_STATE_ABI_VERSION,
        first_successor_digest: first.canonical_digest()?,
        second_successor_digest: second.canonical_digest()?,
        successor_distance,
        materially_different: successor_distance >= MIN_MATERIAL_SUCCESSOR_DISTANCE,
    };
    evidence.validate_contract()?;
    Ok(evidence)
}

fn seed_model_parameters(predictor: &mut GroundedSuccessorPredictor) {
    for (index, value) in predictor.source_projection.iter_mut().enumerate() {
        *value = deterministic_weight(index, 0.12);
    }
    for (index, value) in predictor.motor_projection.iter_mut().enumerate() {
        *value = deterministic_weight(index + 97, 0.12);
    }
    for (index, value) in predictor.interaction_weights.iter_mut().enumerate() {
        *value = deterministic_weight(index + 193, 0.08);
    }
}

fn seeded_embedding(
    primitive: CategoricalMotorPrimitive,
) -> [f32; MOTOR_PRIMITIVE_EMBEDDING_DIM] {
    let mut values = [0.0; MOTOR_PRIMITIVE_EMBEDDING_DIM];
    for (index, value) in values.iter_mut().enumerate() {
        *value = deterministic_weight(primitive.identity[index] as usize, 0.8);
    }
    values
}

fn deterministic_weight(index: usize, amplitude: f32) -> f32 {
    (((index as f32 + 1.0) * 0.618_034).sin() * amplitude).clamp(-amplitude, amplitude)
}

fn cold_start_prediction(
    source: &SemanticStateVector,
    motor: &JointMotorCondition,
) -> Vec<f32> {
    let motor_signal = motor
        .feature_vector()
        .iter()
        .enumerate()
        .map(|(index, value)| *value * deterministic_weight(index + 31, 0.08))
        .sum::<f32>();
    source
        .values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            (value * 0.55 + motor_signal * deterministic_weight(index + 11, 0.35) + 0.225)
                .clamp(0.0, 1.0)
        })
        .collect()
}

fn validate_prediction_update(
    update: &PredictionUpdate,
    semantic_state_count: usize,
) -> Result<(), ScaffoldContractError> {
    if update.target_digest == [0; 4]
        || update.error.len() != semantic_state_count
        || update.prediction.predicted_successor.len() != semantic_state_count
        || !update.mean_squared_error.is_finite()
        || !update.mean_absolute_error.is_finite()
        || update.error.iter().any(|value| !value.is_finite())
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

fn mean_absolute_values(first: &[f32], second: &[f32]) -> Result<f32, ScaffoldContractError> {
    if first.len() != second.len() || first.is_empty() {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    let distance = first
        .iter()
        .zip(second)
        .map(|(left, right)| (*left - *right).abs())
        .sum::<f32>()
        / first.len() as f32;
    if distance.is_finite() {
        Ok(distance.clamp(0.0, 1.0))
    } else {
        Err(ScaffoldContractError::NonFiniteFloat)
    }
}

fn dot(left: &[f32], right: &[f32]) -> f32 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

fn sigmoid(value: f32) -> f32 {
    if value >= 0.0 {
        1.0 / (1.0 + (-value).exp())
    } else {
        let exp = value.exp();
        exp / (1.0 + exp)
    }
}

fn bounded_unit(value: f32) -> f32 {
    (value / (1.0 + value.abs())).clamp(0.0, 1.0)
}

fn signed_unit(value: f32) -> f32 {
    0.5 + 0.5 * (value / (1.0 + value.abs()))
}

fn write_words(builder: &mut CanonicalDigestBuilder, words: [u64; 4]) {
    builder.write_sequence_len(words.len());
    for word in words {
        builder.write_u64(word);
    }
}
