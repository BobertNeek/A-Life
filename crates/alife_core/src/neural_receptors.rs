//! Neural-owned projection of targeted biochemical receptor activations.

use serde::{Deserialize, Serialize};

use crate::{
    BrainPhenotype, LobeKind, NeuralReceptorClass, NeuralReceptorFrame, ScaffoldContractError,
    Tick, Validate,
};

/// Phenotype-compiled neural expression for each biochemical receptor class.
/// Chemistry cannot set these gains.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NeuralReceptorPhenotype {
    expression: [f32; 10],
}

impl NeuralReceptorPhenotype {
    pub fn compile(phenotype: &BrainPhenotype) -> Result<Self, ScaffoldContractError> {
        if phenotype.neuron_count() == 0
            || phenotype.recompute_phenotype_hash()? != phenotype.phenotype_hash()
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        let neurons = phenotype.neuron_count().max(1) as f32;
        let share = |kind: LobeKind| {
            phenotype
                .lobe_layout()
                .region(kind)
                .map_or(0.0, |region| region.len as f32 / neurons)
        };
        let receptor_count = phenotype.plasticity_receptors().len().max(1) as f32;
        let appetitive = phenotype
            .plasticity_receptors()
            .iter()
            .map(|receptor| receptor.receptor_profile().weights()[6].abs())
            .sum::<f32>()
            / receptor_count;
        let aversive = phenotype
            .plasticity_receptors()
            .iter()
            .map(|receptor| receptor.receptor_profile().weights()[7].abs())
            .sum::<f32>()
            / receptor_count;
        let bounded = |value: f32| value.clamp(0.25, 1.0);
        let value = Self {
            expression: [
                bounded(0.25 + share(LobeKind::InteroceptiveMotivational) * 3.0),
                bounded(0.25 + share(LobeKind::FlexibleReserve) * 3.0),
                bounded(0.25 + share(LobeKind::MultimodalAssociation) * 3.0),
                bounded(0.25 + share(LobeKind::WorkingContextExecutive) * 3.0),
                bounded(0.25 + share(LobeKind::PerceptualIntegration) * 3.0),
                bounded(0.25 + appetitive * 0.375),
                bounded(0.25 + aversive * 0.375),
                bounded(0.25 + share(LobeKind::FlexibleReserve) * 4.0),
                bounded(0.25 + share(LobeKind::InteroceptiveMotivational) * 4.0),
                bounded(0.25 + share(LobeKind::MemoryInterface) * 4.0),
            ],
        };
        value.validate_contract()?;
        Ok(value)
    }

    pub const fn expression(&self) -> &[f32; 10] {
        &self.expression
    }
}

impl Validate for NeuralReceptorPhenotype {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self
            .expression
            .iter()
            .any(|value| !value.is_finite() || !(0.25..=1.0).contains(value))
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

/// Bounded neural modulation derived from one authoritative chemistry tick.
///
/// Biochemistry owns receptor activation. The neural subsystem owns this
/// projection. No field is a selected action, policy score, or reward.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NeuralReceptorEffects {
    pub source_tick: Tick,
    pub source_chemistry_version: u16,
    pub interoceptive_gain: f32,
    pub regional_excitability: f32,
    pub projection_gain: f32,
    pub local_threshold_shift: f32,
    pub attention_gain: f32,
    pub plasticity_appetitive: f32,
    pub plasticity_aversive: f32,
    pub structural_growth_gate: f32,
    pub sleep_gate: f32,
    pub consolidation_gate: f32,
}

impl NeuralReceptorEffects {
    pub fn from_frame(
        frame: &NeuralReceptorFrame,
        phenotype: &NeuralReceptorPhenotype,
    ) -> Result<Self, ScaffoldContractError> {
        frame.validate_contract()?;
        phenotype.validate_contract()?;
        let signal =
            |class, index: usize| frame.activation_for(class) * phenotype.expression[index];
        let value = Self {
            source_tick: frame.source_tick,
            source_chemistry_version: frame.source_chemistry_version,
            interoceptive_gain: 0.5 + signal(NeuralReceptorClass::InteroceptiveInput, 0),
            regional_excitability: 0.5 + signal(NeuralReceptorClass::RegionalExcitability, 1),
            projection_gain: 0.5 + signal(NeuralReceptorClass::ProjectionGain, 2),
            local_threshold_shift: 0.25 - 0.5 * signal(NeuralReceptorClass::LocalThreshold, 3),
            attention_gain: 0.5 + signal(NeuralReceptorClass::AttentionGate, 4),
            plasticity_appetitive: signal(NeuralReceptorClass::PlasticityAppetitive, 5),
            plasticity_aversive: signal(NeuralReceptorClass::PlasticityAversive, 6),
            structural_growth_gate: signal(NeuralReceptorClass::StructuralGrowth, 7),
            sleep_gate: signal(NeuralReceptorClass::Sleep, 8),
            consolidation_gate: signal(NeuralReceptorClass::Consolidation, 9),
        };
        value.validate_contract()?;
        Ok(value)
    }
}

impl Validate for NeuralReceptorEffects {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        let gains = [
            self.interoceptive_gain,
            self.regional_excitability,
            self.projection_gain,
            self.attention_gain,
        ];
        let unit = [
            self.plasticity_appetitive,
            self.plasticity_aversive,
            self.structural_growth_gate,
            self.sleep_gate,
            self.consolidation_gate,
        ];
        if gains
            .iter()
            .any(|value| !value.is_finite() || !(0.5..=1.5).contains(value))
            || unit
                .iter()
                .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
            || !self.local_threshold_shift.is_finite()
            || !(-0.25..=0.25).contains(&self.local_threshold_shift)
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}
