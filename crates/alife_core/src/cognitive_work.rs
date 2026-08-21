//! Versioned, hardware-independent cognitive work receipts.

use serde::{Deserialize, Serialize};

use crate::{CanonicalDigestBuilder, ScaffoldContractError, Validate};

pub const COGNITIVE_WORK_SCHEMA_VERSION: u16 = 1;
pub const COGNITIVE_WORK_POLICY_VERSION: u16 = 1;
pub const COGNITIVE_WORK_COST_POLICY_VERSION: u16 = 1;
pub const MAX_COGNITIVE_WORK_COUNTER: u64 = 1_000_000_000;
pub const MAX_COGNITIVE_ENERGY_PER_WORK_UNIT: f32 = 1.0;

fn default_cognitive_work_cost_policy_version() -> u16 {
    COGNITIVE_WORK_COST_POLICY_VERSION
}

/// Bounded semantic operation counts collected by the cognitive runtime.
///
/// These counts describe work performed, not elapsed time or a particular
/// execution backend. Runtime emitters can accumulate them independently and
/// seal one deterministic receipt at the end of a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitiveWorkCounters {
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
    #[serde(default)]
    pub motor_ops: u64,
}

impl CognitiveWorkCounters {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
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
        let counters = Self {
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
            motor_ops: 0,
        };
        counters.validate_contract()?;
        Ok(counters)
    }

    pub const fn zero() -> Self {
        Self {
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
            motor_ops: 0,
        }
    }

    pub fn into_receipt(self) -> Result<CognitiveWorkReceipt, ScaffoldContractError> {
        CognitiveWorkReceipt::aggregate(self)
    }

    pub fn with_motor_ops(mut self, motor_ops: u64) -> Result<Self, ScaffoldContractError> {
        self.motor_ops = motor_ops;
        self.validate_contract()?;
        Ok(self)
    }

    fn values(self) -> [u64; 13] {
        [
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
            self.motor_ops,
        ]
    }

    fn validate_counter_bounds(self) -> Result<(), ScaffoldContractError> {
        if self
            .values()
            .into_iter()
            .any(|value| value > MAX_COGNITIVE_WORK_COUNTER)
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

impl Default for CognitiveWorkCounters {
    fn default() -> Self {
        Self::zero()
    }
}

impl Validate for CognitiveWorkCounters {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        (*self).validate_counter_bounds()
    }
}

/// Optional world/species conversion from semantic work units into body
/// energy expenditure. A disabled policy still permits the receipt to be
/// recorded while charging no energy.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CognitiveWorkCostPolicy {
    pub enabled: bool,
    #[serde(default = "default_cognitive_work_cost_policy_version")]
    pub schema_version: u16,
    pub energy_per_work_unit: f32,
    #[serde(default)]
    pub fatigue_per_work_unit: f32,
    #[serde(default)]
    pub heat_per_work_unit: f32,
}

impl CognitiveWorkCostPolicy {
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            schema_version: COGNITIVE_WORK_COST_POLICY_VERSION,
            energy_per_work_unit: 0.0,
            fatigue_per_work_unit: 0.0,
            heat_per_work_unit: 0.0,
        }
    }

    pub const fn production_default() -> Self {
        Self {
            enabled: true,
            schema_version: COGNITIVE_WORK_COST_POLICY_VERSION,
            energy_per_work_unit: 0.000_001,
            fatigue_per_work_unit: 0.000_000_5,
            heat_per_work_unit: 0.000_000_25,
        }
    }

    pub fn enabled(energy_per_work_unit: f32) -> Result<Self, ScaffoldContractError> {
        let policy = Self {
            enabled: true,
            schema_version: COGNITIVE_WORK_COST_POLICY_VERSION,
            energy_per_work_unit,
            fatigue_per_work_unit: 0.0,
            heat_per_work_unit: 0.0,
        };
        policy.validate_contract()?;
        Ok(policy)
    }

    pub const fn is_enabled(self) -> bool {
        self.enabled
    }

    pub fn fatigue_debit(
        &self,
        receipt: &CognitiveWorkReceipt,
    ) -> Result<f32, ScaffoldContractError> {
        self.validate_contract()?;
        receipt.validate_contract()?;
        Ok(if self.enabled {
            (receipt.weighted_total as f64 * f64::from(self.fatigue_per_work_unit)) as f32
        } else {
            0.0
        })
    }

    pub fn heat_debit(&self, receipt: &CognitiveWorkReceipt) -> Result<f32, ScaffoldContractError> {
        self.validate_contract()?;
        receipt.validate_contract()?;
        Ok(if self.enabled {
            (receipt.weighted_total as f64 * f64::from(self.heat_per_work_unit)) as f32
        } else {
            0.0
        })
    }

    pub fn energy_debit(
        &self,
        receipt: &CognitiveWorkReceipt,
    ) -> Result<f32, ScaffoldContractError> {
        self.validate_contract()?;
        receipt.validate_contract()?;
        if !self.enabled {
            return Ok(0.0);
        }
        let debit = (receipt.weighted_total as f64 * f64::from(self.energy_per_work_unit)) as f32;
        if !debit.is_finite() {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        Ok(debit)
    }
}

impl Default for CognitiveWorkCostPolicy {
    fn default() -> Self {
        Self::disabled()
    }
}

impl Validate for CognitiveWorkCostPolicy {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != COGNITIVE_WORK_COST_POLICY_VERSION
            || !self.energy_per_work_unit.is_finite()
            || self.energy_per_work_unit < 0.0
            || self.energy_per_work_unit > MAX_COGNITIVE_ENERGY_PER_WORK_UNIT
            || !self.fatigue_per_work_unit.is_finite()
            || self.fatigue_per_work_unit < 0.0
            || self.fatigue_per_work_unit > MAX_COGNITIVE_ENERGY_PER_WORK_UNIT
            || !self.heat_per_work_unit.is_finite()
            || self.heat_per_work_unit < 0.0
            || self.heat_per_work_unit > MAX_COGNITIVE_ENERGY_PER_WORK_UNIT
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

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
    #[serde(default)]
    pub motor_ops: u64,
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
        CognitiveWorkCounters::new(
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
        )
        .and_then(Self::aggregate)
    }

    pub fn aggregate(counters: CognitiveWorkCounters) -> Result<Self, ScaffoldContractError> {
        counters.validate_contract()?;
        let [neural_updates, synapses_evaluated, dendritic_ops, focal_target_ops, memory_ops, concept_ops, gap_ops, prediction_ops, replay_ops, structural_ops, learning_ops, sleep_ops, motor_ops] =
            counters.values();
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
            motor_ops,
        };
        receipt.with_computed_total()
    }

    pub fn with_motor_ops(mut self, motor_ops: u64) -> Result<Self, ScaffoldContractError> {
        self.motor_ops = motor_ops;
        self.validate_contract()?;
        Ok(self)
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
            motor_ops: 0,
            weighted_total: 0,
            policy_version: COGNITIVE_WORK_POLICY_VERSION,
        }
    }

    pub const fn attention_ops(&self) -> u64 {
        self.focal_target_ops
    }

    pub fn recompute_total(&self) -> Result<u64, ScaffoldContractError> {
        CognitiveWorkCounters {
            neural_updates: self.neural_updates,
            synapses_evaluated: self.synapses_evaluated,
            dendritic_ops: self.dendritic_ops,
            focal_target_ops: self.focal_target_ops,
            memory_ops: self.memory_ops,
            concept_ops: self.concept_ops,
            gap_ops: self.gap_ops,
            prediction_ops: self.prediction_ops,
            replay_ops: self.replay_ops,
            structural_ops: self.structural_ops,
            learning_ops: self.learning_ops,
            sleep_ops: self.sleep_ops,
            motor_ops: self.motor_ops,
        }
        .values()
        .into_iter()
        .try_fold(0u64, |total, value| {
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
            self.motor_ops,
        ] {
            builder.write_u64(value);
        }
        builder.write_u64(self.weighted_total);
        builder.write_u16(self.policy_version);
        Ok(builder.finish256())
    }

    fn validate_counter_bounds(&self) -> Result<(), ScaffoldContractError> {
        CognitiveWorkCounters {
            neural_updates: self.neural_updates,
            synapses_evaluated: self.synapses_evaluated,
            dendritic_ops: self.dendritic_ops,
            focal_target_ops: self.focal_target_ops,
            memory_ops: self.memory_ops,
            concept_ops: self.concept_ops,
            gap_ops: self.gap_ops,
            prediction_ops: self.prediction_ops,
            replay_ops: self.replay_ops,
            structural_ops: self.structural_ops,
            learning_ops: self.learning_ops,
            sleep_ops: self.sleep_ops,
            motor_ops: self.motor_ops,
        }
        .validate_counter_bounds()
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
