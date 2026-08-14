//! Grounded, versioned prediction-target receipts.

use serde::{Deserialize, Serialize};

use crate::{
    ActionId, CanonicalDigestBuilder, ExperienceSequenceId, NormalizedScalar, OrganismId,
    ScaffoldContractError, Tick, Validate,
};

pub const PREDICTION_TARGET_SCHEMA_VERSION: u16 = 1;
pub const MAX_SUCCESSOR_FEATURES: usize = 64;
pub const SUCCESSOR_FEATURE_ABI_V1: u16 = 1;
pub const DEFAULT_PREDICTOR_LEARNING_RATE: f32 = 0.25;

const MAX_PREDICTOR_ACTIONS: usize = 32;
const PREDICTOR_CONTEXT_FEATURES: usize = 8;
const PREDICTOR_INPUT_FEATURES: usize = PREDICTOR_CONTEXT_FEATURES + 1;

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PredictionTargetReceipt {
    pub schema_version: u16,
    pub organism_id: OrganismId,
    pub experience_sequence: ExperienceSequenceId,
    pub decision: ActionId,
    pub world_tick: Tick,
    pub source_digest: [u64; 4],
    pub successor_feature_abi: u16,
    pub family: PredictionTargetFamily,
    pub target_digest: [u64; 4],
    pub successor_features: Vec<f32>,
    pub representation_variance: f32,
    pub action_sensitivity_score: f32,
    pub successor_separability_score: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuccessorPrediction {
    pub source_digest: [u64; 4],
    pub decision: ActionId,
    pub successor_feature_abi: u16,
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
pub struct GroundedSuccessorPredictor {
    successor_feature_abi: u16,
    successor_feature_count: usize,
    learning_rate: f32,
    action_heads: Vec<ActionPredictionHead>,
    last_update: Option<PredictionUpdate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ActionPredictionHead {
    action: ActionId,
    weights: Vec<f32>,
}

impl Default for GroundedSuccessorPredictor {
    fn default() -> Self {
        Self {
            successor_feature_abi: 0,
            successor_feature_count: 0,
            learning_rate: DEFAULT_PREDICTOR_LEARNING_RATE,
            action_heads: Vec::new(),
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
        source_digest: [u64; 4],
        decision: ActionId,
        successor_feature_count: usize,
    ) -> Result<SuccessorPrediction, ScaffoldContractError> {
        validate_prediction_input(source_digest, decision, successor_feature_count)?;
        if self.successor_feature_count != 0
            && self.successor_feature_count != successor_feature_count
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }

        let successor_feature_abi = if self.successor_feature_abi == 0 {
            SUCCESSOR_FEATURE_ABI_V1
        } else {
            self.successor_feature_abi
        };
        let inputs = predictor_inputs(source_digest);
        let mut predicted_successor = vec![0.0; successor_feature_count];
        if let Some(head) = self
            .action_heads
            .iter()
            .find(|head| head.action == decision)
        {
            for (feature_index, predicted) in predicted_successor.iter_mut().enumerate() {
                let offset = feature_index * PREDICTOR_INPUT_FEATURES;
                let raw_prediction = head.weights[offset..offset + PREDICTOR_INPUT_FEATURES]
                    .iter()
                    .zip(inputs)
                    .map(|(weight, input)| weight * input)
                    .sum::<f32>();
                *predicted = raw_prediction.clamp(0.0, 1.0);
            }
        }

        Ok(SuccessorPrediction {
            source_digest,
            decision,
            successor_feature_abi,
            predicted_successor,
        })
    }

    pub fn observe(
        &mut self,
        receipt: &PredictionTargetReceipt,
    ) -> Result<PredictionUpdate, ScaffoldContractError> {
        receipt.validate_contract()?;
        if self.successor_feature_abi == 0 {
            self.successor_feature_abi = receipt.successor_feature_abi;
        } else if self.successor_feature_abi != receipt.successor_feature_abi {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        if self.successor_feature_count == 0 {
            self.successor_feature_count = receipt.successor_features.len();
        } else if self.successor_feature_count != receipt.successor_features.len() {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }

        let prediction = self.predict(
            receipt.source_digest,
            receipt.decision,
            receipt.successor_features.len(),
        )?;
        let inputs = predictor_inputs(receipt.source_digest);
        let input_energy = inputs.iter().map(|input| input * input).sum::<f32>();
        let normalized_step = self.learning_rate / input_energy.max(f32::EPSILON);
        let mut error = Vec::with_capacity(receipt.successor_features.len());
        let mut squared_error = 0.0;
        let mut absolute_error = 0.0;
        let head = self.action_head_mut(receipt.decision)?;
        for (feature_index, (predicted, target)) in prediction
            .predicted_successor
            .iter()
            .zip(&receipt.successor_features)
            .enumerate()
        {
            let feature_error = *target - *predicted;
            error.push(feature_error);
            squared_error += feature_error * feature_error;
            absolute_error += feature_error.abs();

            let offset = feature_index * PREDICTOR_INPUT_FEATURES;
            for (weight, input) in head.weights[offset..offset + PREDICTOR_INPUT_FEATURES]
                .iter_mut()
                .zip(inputs)
            {
                *weight += normalized_step * feature_error * input;
            }
        }

        let feature_count = receipt.successor_features.len() as f32;
        let update = PredictionUpdate {
            prediction,
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

    fn action_head_mut(
        &mut self,
        action: ActionId,
    ) -> Result<&mut ActionPredictionHead, ScaffoldContractError> {
        if let Some(index) = self
            .action_heads
            .iter()
            .position(|head| head.action == action)
        {
            return Ok(&mut self.action_heads[index]);
        }
        if self.action_heads.len() >= MAX_PREDICTOR_ACTIONS {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        self.action_heads.push(ActionPredictionHead {
            action,
            weights: vec![0.0; self.successor_feature_count * PREDICTOR_INPUT_FEATURES],
        });
        self.action_heads
            .last_mut()
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)
    }
}

fn validate_prediction_input(
    source_digest: [u64; 4],
    decision: ActionId,
    successor_feature_count: usize,
) -> Result<(), ScaffoldContractError> {
    if source_digest == [0; 4] {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    decision.validate()?;
    if successor_feature_count == 0 || successor_feature_count > MAX_SUCCESSOR_FEATURES {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    Ok(())
}

fn predictor_inputs(source_digest: [u64; 4]) -> [f32; PREDICTOR_INPUT_FEATURES] {
    let mut inputs = [0.0; PREDICTOR_INPUT_FEATURES];
    inputs[0] = 1.0;
    for (index, word) in source_digest.into_iter().enumerate() {
        inputs[1 + index * 2] = (word as u32 as f32) / u32::MAX as f32;
        inputs[2 + index * 2] = ((word >> 32) as u32 as f32) / u32::MAX as f32;
    }
    inputs
}

impl PredictionTargetReceipt {
    pub fn for_successor(
        organism_id: OrganismId,
        experience_sequence: ExperienceSequenceId,
        decision: ActionId,
        world_tick: Tick,
        source_digest: [u64; 4],
        successor_feature_abi: u16,
        successor_features: Vec<f32>,
    ) -> Result<Self, ScaffoldContractError> {
        let target_digest = digest_successor_features(successor_feature_abi, &successor_features)?;
        let receipt = Self {
            schema_version: PREDICTION_TARGET_SCHEMA_VERSION,
            organism_id,
            experience_sequence,
            decision,
            world_tick,
            source_digest,
            successor_feature_abi,
            family: PredictionTargetFamily::GroundedObservables,
            target_digest,
            representation_variance: feature_variance(&successor_features)?,
            action_sensitivity_score: if successor_features.len() > 1 {
                1.0
            } else {
                0.0
            },
            successor_separability_score: if successor_features.is_empty() {
                0.0
            } else {
                1.0
            },
            successor_features,
        };
        receipt.validate_contract()?;
        Ok(receipt)
    }

    pub fn canonical_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        self.validate_contract()?;
        let mut builder = CanonicalDigestBuilder::new(b"ALIFE-V11-PREDICTION-TARGET");
        builder.write_u16(self.schema_version);
        builder.write_u64(self.organism_id.raw());
        builder.write_u64(self.experience_sequence.raw());
        builder.write_u32(self.decision.raw());
        builder.write_u64(self.world_tick.raw());
        write_words(&mut builder, self.source_digest);
        builder.write_u16(self.successor_feature_abi);
        builder.write_u8(self.family as u8);
        write_words(&mut builder, self.target_digest);
        builder.write_sequence_len(self.successor_features.len());
        for value in &self.successor_features {
            builder.write_f32(*value)?;
        }
        builder.write_f32(self.representation_variance)?;
        builder.write_f32(self.action_sensitivity_score)?;
        builder.write_f32(self.successor_separability_score)?;
        Ok(builder.finish256())
    }

    pub const fn target_digest(&self) -> [u64; 4] {
        self.target_digest
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
            || self.representation_variance < 0.0
            || self.action_sensitivity_score < 0.0
            || self.successor_separability_score < 0.0
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        self.organism_id.validate()?;
        self.experience_sequence.validate()?;
        self.decision.validate()?;
        validate_successor_features(self.successor_feature_abi, &self.successor_features)?;
        if digest_successor_features(self.successor_feature_abi, &self.successor_features)?
            != self.target_digest
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
}

fn digest_successor_features(
    successor_feature_abi: u16,
    features: &[f32],
) -> Result<[u64; 4], ScaffoldContractError> {
    validate_successor_features(successor_feature_abi, features)?;
    let mut builder = CanonicalDigestBuilder::new(b"ALIFE-V11-SUCCESSOR-FEATURES");
    builder.write_u16(successor_feature_abi);
    builder.write_sequence_len(features.len());
    for value in features {
        builder.write_f32(*value)?;
    }
    Ok(builder.finish256())
}

fn validate_successor_features(
    successor_feature_abi: u16,
    features: &[f32],
) -> Result<(), ScaffoldContractError> {
    if successor_feature_abi != SUCCESSOR_FEATURE_ABI_V1
        || features.is_empty()
        || features.len() > MAX_SUCCESSOR_FEATURES
    {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    for value in features {
        NormalizedScalar::new(*value)?;
    }
    Ok(())
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
        Ok(variance)
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
