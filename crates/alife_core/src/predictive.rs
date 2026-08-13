//! Grounded, versioned prediction-target receipts.

use serde::{Deserialize, Serialize};

use crate::{
    ActionId, CanonicalDigestBuilder, ExperienceSequenceId, OrganismId, ScaffoldContractError,
    Tick, Validate,
};

pub const PREDICTION_TARGET_SCHEMA_VERSION: u16 = 1;
pub const MAX_SUCCESSOR_FEATURES: usize = 64;
pub const SUCCESSOR_FEATURE_ABI_V1: u16 = 1;

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
            || self.successor_feature_abi == 0
            || self.successor_features.is_empty()
            || self.successor_features.len() > MAX_SUCCESSOR_FEATURES
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
        for value in &self.successor_features {
            if !value.is_finite() {
                return Err(ScaffoldContractError::NonFiniteFloat);
            }
        }
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
    if successor_feature_abi == 0 || features.is_empty() || features.len() > MAX_SUCCESSOR_FEATURES
    {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    let mut builder = CanonicalDigestBuilder::new(b"ALIFE-V11-SUCCESSOR-FEATURES");
    builder.write_u16(successor_feature_abi);
    builder.write_sequence_len(features.len());
    for value in features {
        builder.write_f32(*value)?;
    }
    Ok(builder.finish256())
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
