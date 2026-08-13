//! Versioned, hardware-independent cognitive work receipts.

use serde::{Deserialize, Serialize};

use crate::{CanonicalDigestBuilder, ScaffoldContractError, Validate};

pub const COGNITIVE_WORK_SCHEMA_VERSION: u16 = 1;
pub const COGNITIVE_WORK_POLICY_VERSION: u16 = 1;
pub const MAX_COGNITIVE_WORK_COUNTER: u64 = 1_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitiveWorkReceipt {
    pub schema_version: u16,
    pub neural_updates: u64,
    pub synapses_evaluated: u64,
    pub dendritic_ops: u64,
    pub focal_target_ops: u64,
    pub memory_ops: u64,
    pub concept_ops: u64,
    pub gap_ops: u64,
    pub prediction_ops: u64,
    pub replay_ops: u64,
    pub structural_ops: u64,
    pub learning_ops: u64,
    pub sleep_ops: u64,
    pub weighted_total: u64,
    pub policy_version: u16,
}

impl CognitiveWorkReceipt {
    #[allow(clippy::too_many_arguments)]
    pub fn from_counters(
        neural_updates: u64,
        synapses_evaluated: u64,
        dendritic_ops: u64,
        focal_target_ops: u64,
        memory_ops: u64,
        concept_ops: u64,
        gap_ops: u64,
        prediction_ops: u64,
        replay_ops: u64,
        structural_ops: u64,
        learning_ops: u64,
        sleep_ops: u64,
    ) -> Result<Self, ScaffoldContractError> {
        let receipt = Self {
            schema_version: COGNITIVE_WORK_SCHEMA_VERSION,
            neural_updates,
            synapses_evaluated,
            dendritic_ops,
            focal_target_ops,
            memory_ops,
            concept_ops,
            gap_ops,
            prediction_ops,
            replay_ops,
            structural_ops,
            learning_ops,
            sleep_ops,
            weighted_total: 0,
            policy_version: COGNITIVE_WORK_POLICY_VERSION,
        };
        receipt.with_computed_total()
    }

    pub const fn zero() -> Self {
        Self {
            schema_version: COGNITIVE_WORK_SCHEMA_VERSION,
            neural_updates: 0,
            synapses_evaluated: 0,
            dendritic_ops: 0,
            focal_target_ops: 0,
            memory_ops: 0,
            concept_ops: 0,
            gap_ops: 0,
            prediction_ops: 0,
            replay_ops: 0,
            structural_ops: 0,
            learning_ops: 0,
            sleep_ops: 0,
            weighted_total: 0,
            policy_version: COGNITIVE_WORK_POLICY_VERSION,
        }
    }

    pub const fn attention_ops(&self) -> u64 {
        self.focal_target_ops
    }

    pub fn recompute_total(&self) -> Result<u64, ScaffoldContractError> {
        let counters = [
            self.neural_updates,
            self.synapses_evaluated,
            self.dendritic_ops,
            self.focal_target_ops,
            self.memory_ops,
            self.concept_ops,
            self.gap_ops,
            self.prediction_ops,
            self.replay_ops,
            self.structural_ops,
            self.learning_ops,
            self.sleep_ops,
        ];
        counters.into_iter().try_fold(0u64, |total, value| {
            total
                .checked_add(value)
                .ok_or(ScaffoldContractError::ScalarOutOfRange)
        })
    }

    pub fn with_computed_total(mut self) -> Result<Self, ScaffoldContractError> {
        self.validate_counter_bounds()?;
        self.weighted_total = self.recompute_total()?;
        Ok(self)
    }

    pub fn canonical_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        self.validate_contract()?;
        let mut builder = CanonicalDigestBuilder::new(b"ALIFE-V11-COGNITIVE-WORK");
        builder.write_u16(self.schema_version);
        for value in [
            self.neural_updates,
            self.synapses_evaluated,
            self.dendritic_ops,
            self.focal_target_ops,
            self.memory_ops,
            self.concept_ops,
            self.gap_ops,
            self.prediction_ops,
            self.replay_ops,
            self.structural_ops,
            self.learning_ops,
            self.sleep_ops,
        ] {
            builder.write_u64(value);
        }
        builder.write_u64(self.weighted_total);
        builder.write_u16(self.policy_version);
        Ok(builder.finish256())
    }

    fn validate_counter_bounds(&self) -> Result<(), ScaffoldContractError> {
        for value in [
            self.neural_updates,
            self.synapses_evaluated,
            self.dendritic_ops,
            self.focal_target_ops,
            self.memory_ops,
            self.concept_ops,
            self.gap_ops,
            self.prediction_ops,
            self.replay_ops,
            self.structural_ops,
            self.learning_ops,
            self.sleep_ops,
        ] {
            if value > MAX_COGNITIVE_WORK_COUNTER {
                return Err(ScaffoldContractError::ScalarOutOfRange);
            }
        }
        Ok(())
    }
}

impl Default for CognitiveWorkReceipt {
    fn default() -> Self {
        Self::zero()
    }
}

impl Validate for CognitiveWorkReceipt {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != COGNITIVE_WORK_SCHEMA_VERSION
            || self.policy_version != COGNITIVE_WORK_POLICY_VERSION
        {
            return Err(ScaffoldContractError::IncompatibleAbi {
                kind: crate::SchemaKind::Experience,
                expected: COGNITIVE_WORK_SCHEMA_VERSION,
                actual: self.schema_version,
            });
        }
        self.validate_counter_bounds()?;
        if self.weighted_total != self.recompute_total()? {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
}
