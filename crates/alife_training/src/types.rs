//! Offline-only foundation-training inputs and receipts.

use alife_core::{
    BrainPhenotype, CandidateActionFamily, CandidateFeatureVector, CompiledSynapseKind,
    ScaffoldContractError, Validate, CANDIDATE_FEATURE_COUNT,
};

pub const TRAINING_SEQUENCE_TICKS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdamWConfig {
    pub learning_rate: f32,
    pub beta1: f32,
    pub beta2: f32,
    pub epsilon: f32,
    pub weight_decay: f32,
    pub gradient_clip: f32,
}

impl Default for AdamWConfig {
    fn default() -> Self {
        Self {
            learning_rate: 3.0e-4,
            beta1: 0.9,
            beta2: 0.999,
            epsilon: 1.0e-8,
            weight_decay: 1.0e-4,
            gradient_clip: 1.0,
        }
    }
}

impl AdamWConfig {
    pub fn validate(self) -> Result<(), ScaffoldContractError> {
        if [
            self.learning_rate,
            self.beta1,
            self.beta2,
            self.epsilon,
            self.weight_decay,
            self.gradient_clip,
        ]
        .into_iter()
        .all(f32::is_finite)
            && self.learning_rate > 0.0
            && (0.0..1.0).contains(&self.beta1)
            && (0.0..1.0).contains(&self.beta2)
            && self.epsilon > 0.0
            && self.weight_decay >= 0.0
            && self.gradient_clip > 0.0
        {
            Ok(())
        } else {
            Err(ScaffoldContractError::PhenotypeCompile)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CandidateTrainingTarget {
    pub family: CandidateActionFamily,
    pub features: CandidateFeatureVector,
    pub target_logit: f32,
    pub loss_weight: f32,
}

impl CandidateTrainingTarget {
    pub fn try_new(
        family: CandidateActionFamily,
        features: CandidateFeatureVector,
        target_logit: f32,
        loss_weight: f32,
    ) -> Result<Self, ScaffoldContractError> {
        let value = Self {
            family,
            features,
            target_logit,
            loss_weight,
        };
        value.validate()?;
        Ok(value)
    }

    fn validate(self) -> Result<(), ScaffoldContractError> {
        self.features.validate_contract()?;
        if self.target_logit.is_finite() && self.loss_weight.is_finite() && self.loss_weight > 0.0 {
            Ok(())
        } else {
            Err(ScaffoldContractError::PhenotypeCompile)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrainingTick {
    encoded_inputs: Vec<f32>,
    target_activations: Vec<f32>,
    target_weights: Vec<f32>,
    candidate_target: Option<CandidateTrainingTarget>,
}

impl TrainingTick {
    pub fn try_new(
        encoded_inputs: Vec<f32>,
        target_activations: Vec<f32>,
        target_weights: Vec<f32>,
        candidate_target: Option<CandidateTrainingTarget>,
    ) -> Result<Self, ScaffoldContractError> {
        let value = Self {
            encoded_inputs,
            target_activations,
            target_weights,
            candidate_target,
        };
        value.validate_finite()?;
        Ok(value)
    }

    pub fn encoded_inputs(&self) -> &[f32] {
        &self.encoded_inputs
    }

    pub fn target_activations(&self) -> &[f32] {
        &self.target_activations
    }

    pub fn target_weights(&self) -> &[f32] {
        &self.target_weights
    }

    pub const fn candidate_target(&self) -> Option<CandidateTrainingTarget> {
        self.candidate_target
    }

    fn validate_finite(&self) -> Result<(), ScaffoldContractError> {
        if self.encoded_inputs.is_empty()
            || self.encoded_inputs.len() != self.target_activations.len()
            || self.encoded_inputs.len() != self.target_weights.len()
            || self
                .encoded_inputs
                .iter()
                .chain(&self.target_activations)
                .chain(&self.target_weights)
                .any(|value| !value.is_finite())
            || self.target_weights.iter().any(|weight| *weight < 0.0)
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        if let Some(candidate) = self.candidate_target {
            candidate.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrainingSequence32 {
    ticks: Vec<TrainingTick>,
}

impl TrainingSequence32 {
    pub fn try_new(ticks: Vec<TrainingTick>) -> Result<Self, ScaffoldContractError> {
        if ticks.len() != TRAINING_SEQUENCE_TICKS {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        for tick in &ticks {
            tick.validate_finite()?;
        }
        Ok(Self { ticks })
    }

    pub fn ticks(&self) -> &[TrainingTick] {
        &self.ticks
    }

    pub fn validate_for(&self, phenotype: &BrainPhenotype) -> Result<(), ScaffoldContractError> {
        let neurons = phenotype.neuron_count() as usize;
        if self.ticks.iter().all(|tick| {
            tick.encoded_inputs.len() == neurons
                && tick.target_activations.len() == neurons
                && tick.target_weights.len() == neurons
        }) {
            Ok(())
        } else {
            Err(ScaffoldContractError::PhenotypeCompile)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageTrainableMask {
    words: Vec<u32>,
}

impl StageTrainableMask {
    pub fn from_synapse_indices(
        phenotype: &BrainPhenotype,
        indices: &[u32],
    ) -> Result<Self, ScaffoldContractError> {
        if indices.is_empty() {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        let mut words = vec![0; phenotype.synapses().len()];
        for index in indices {
            let word = words
                .get_mut(*index as usize)
                .ok_or(ScaffoldContractError::PhenotypeCompile)?;
            if *word != 0 {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
            *word = 1;
        }
        Ok(Self { words })
    }

    pub fn from_route_indices(
        phenotype: &BrainPhenotype,
        route_indices: &[u16],
    ) -> Result<Self, ScaffoldContractError> {
        let selected = phenotype
            .synapses()
            .iter()
            .enumerate()
            .filter(|(_, synapse)| route_indices.contains(&synapse.route_index()))
            .map(|(index, _)| index as u32)
            .collect::<Vec<_>>();
        Self::from_synapse_indices(phenotype, &selected)
    }

    pub fn recurrent_only(phenotype: &BrainPhenotype) -> Result<Self, ScaffoldContractError> {
        let selected = phenotype
            .synapses()
            .iter()
            .enumerate()
            .filter(|(_, synapse)| matches!(synapse.kind(), CompiledSynapseKind::Recurrent))
            .map(|(index, _)| index as u32)
            .collect::<Vec<_>>();
        Self::from_synapse_indices(phenotype, &selected)
    }

    pub const fn len(&self) -> usize {
        self.words.len()
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    pub fn trainable_count(&self) -> usize {
        self.words.iter().filter(|word| **word != 0).count()
    }

    pub fn is_trainable(&self, index: usize) -> bool {
        self.words.get(index).copied() == Some(1)
    }

    pub(crate) fn words(&self) -> &[u32] {
        &self.words
    }

    pub(crate) fn validate_for(
        &self,
        phenotype: &BrainPhenotype,
    ) -> Result<(), ScaffoldContractError> {
        if self.words.len() == phenotype.synapses().len()
            && self.words.contains(&1)
            && self.words.iter().all(|word| *word <= 1)
        {
            Ok(())
        } else {
            Err(ScaffoldContractError::PhenotypeCompile)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrainingStepReceipt {
    pub optimizer_step: u32,
    pub loss_before: f32,
    pub loss_after: f32,
    pub unclipped_gradient_norm: f32,
    pub trained_weight_count: u32,
}

pub(crate) const CANDIDATE_RECORD_WORDS: usize = 4 + CANDIDATE_FEATURE_COUNT + 4;
