//! Offline-only foundation-training inputs and receipts.

use alife_core::{
    BrainPhenotype, CandidateActionFamily, CandidateFeatureVector, CompiledSynapseKind,
    ScaffoldContractError, SpeechDecoderLayoutV1, Validate,
};

pub const TRAINING_SEQUENCE_TICKS: usize = 32;
pub const MAX_TRAINING_SEQUENCE_TICKS: usize = 2048;

/// Detached production state at a replay boundary. These are real runtime
/// snapshots, not hidden teacher inputs. Gradients stop at this boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct TrainingInitialState {
    pub activations: Vec<f32>,
    pub activity_ema: Vec<f32>,
    pub metabolic_load: Vec<f32>,
    pub dendrites: alife_core::DendriticBranchSet,
}

/// One production decoder candidate, including confidence-weighted memory
/// lanes (24..36) and cognitive projection lanes (36..54).
#[derive(Debug, Clone, PartialEq)]
pub struct TrainingReplayCandidate {
    pub family: CandidateActionFamily,
    pub decoder_inputs: [f32; 54],
}

/// A frozen ordinary-runtime context. Lifetime/fast weights, chemistry,
/// memory and activity decisions are detached; their evolution is not BPTT.
#[derive(Debug, Clone, PartialEq)]
pub struct TrainingReplayTick {
    /// The final production sensor-encoder output, including receptor effects.
    pub encoded_inputs: Vec<f32>,
    pub projection_gain: f32,
    pub local_threshold_shift: f32,
    pub microstep_count: u32,
    pub enabled_routes: Vec<bool>,
    /// Per compiled synapse: lifetime + alpha * fast, in canonical synapse order.
    pub effective_weight_offsets: Vec<f32>,
    pub candidates: Vec<TrainingReplayCandidate>,
}

/// Fixed-topology, frozen-context replay with a detached burn-in boundary.
/// Production collection must supply every context; there is no neutral default.
#[derive(Debug, Clone, PartialEq)]
pub struct TrainingSequence {
    pub phenotype_hash: alife_core::PhenotypeHash,
    pub initial: TrainingInitialState,
    pub ticks: Vec<TrainingReplayTick>,
    pub burn_in_ticks: usize,
    pub memory_candidate_gain: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrainingReplayEvaluation {
    pub candidate_logits: Vec<Vec<f32>>,
    pub final_activations: Vec<Vec<f32>>,
    pub final_activity_ema: Vec<Vec<f32>>,
    pub final_metabolic_load: Vec<Vec<f32>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrainingGradientProbe {
    pub objective: f64,
    pub gradients: Vec<f32>,
}

impl TrainingSequence {
    pub fn validate_for(&self, phenotype: &BrainPhenotype) -> Result<(), ScaffoldContractError> {
        let n = phenotype.neuron_count() as usize;
        let finite = |xs: &[f32]| xs.iter().all(|x| x.is_finite());
        if self.ticks.is_empty()
            || self.ticks.len() > MAX_TRAINING_SEQUENCE_TICKS
            || self.phenotype_hash != phenotype.phenotype_hash()
            || self.burn_in_ticks >= self.ticks.len()
            || self.initial.activations.len() != n
            || self.initial.activity_ema.len() != n
            || self.initial.metabolic_load.len() != n
            || !finite(&self.initial.activations)
            || !self
                .initial
                .activity_ema
                .iter()
                .chain(&self.initial.metabolic_load)
                .all(|x| x.is_finite() && (0.0..=1.0).contains(x))
            || !self.memory_candidate_gain.is_finite()
            || self.memory_candidate_gain < 0.0
            || self.memory_candidate_gain
                != phenotype
                    .candidate_decoder()
                    .memory_channel()
                    .map_or(0.0, |p| p.max_candidate_gain())
            || phenotype
                .projections()
                .iter()
                .any(|p| p.delay_microsteps() != 0)
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        self.initial
            .dendrites
            .validate_for_neuron_count(phenotype.neuron_count())?;
        for tick in &self.ticks {
            if tick.encoded_inputs.len() != n
                || !finite(&tick.encoded_inputs)
                || !tick.projection_gain.is_finite()
                || !tick.local_threshold_shift.is_finite()
                || tick.microstep_count > u32::from(phenotype.microstep_count())
                || tick.enabled_routes.len() != phenotype.projections().len()
                || tick.effective_weight_offsets.len() != phenotype.synapses().len()
                || !finite(&tick.effective_weight_offsets)
                || tick.candidates.is_empty()
                || tick.candidates.len() > alife_core::MAX_ACTION_CANDIDATES
                || tick.candidates.iter().any(|c| !finite(&c.decoder_inputs))
            {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpeechTrainingTarget {
    pub output_index: u16,
    pub target_logit: f32,
    pub loss_weight: f32,
}

impl SpeechTrainingTarget {
    pub fn try_new(
        output_index: u16,
        target_logit: f32,
        loss_weight: f32,
    ) -> Result<Self, ScaffoldContractError> {
        let value = Self {
            output_index,
            target_logit,
            loss_weight,
        };
        value.validate()?;
        Ok(value)
    }

    fn validate(self) -> Result<(), ScaffoldContractError> {
        if self.output_index < SpeechDecoderLayoutV1::OUTPUT_WIDTH
            && self.target_logit.is_finite()
            && self.loss_weight.is_finite()
            && self.loss_weight > 0.0
        {
            Ok(())
        } else {
            Err(ScaffoldContractError::PhenotypeCompile)
        }
    }
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
    speech_target: Option<SpeechTrainingTarget>,
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
            speech_target: None,
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

    pub fn with_speech_target(
        mut self,
        speech_target: SpeechTrainingTarget,
    ) -> Result<Self, ScaffoldContractError> {
        speech_target.validate()?;
        self.speech_target = Some(speech_target);
        Ok(self)
    }

    pub const fn speech_target(&self) -> Option<SpeechTrainingTarget> {
        self.speech_target
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
        if let Some(speech) = self.speech_target {
            speech.validate()?;
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrainingStepStatistics {
    pub optimizer_step: u32,
    pub loss_before: f32,
    pub loss_after: Option<f32>,
    pub unclipped_gradient_norm: f32,
    pub trained_weight_count: u32,
}

/// Offline optimizer checkpoint. Organism saves never contain Adam state.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoundationTrainerCheckpoint {
    pub schema_version: u32,
    pub phenotype_hash: alife_core::PhenotypeHash,
    pub source_foundation_digest: alife_core::Blake3Digest,
    pub optimizer_step: u32,
    pub config: AdamWConfig,
    pub stage_mask: StageTrainableMask,
    pub weights: Vec<f32>,
    pub first_moment: Vec<f32>,
    pub second_moment: Vec<f32>,
    /// Schema 2: successful Adam updates per weight, unchanged while frozen.
    pub update_ages: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SequenceEvaluation {
    mean_loss: f32,
    candidate_logits: Vec<f32>,
    speech_logits: Vec<f32>,
    episode_count: u32,
    success_count: u32,
}

impl SequenceEvaluation {
    pub(crate) fn new(
        mean_loss: f32,
        candidate_logits: Vec<f32>,
        speech_logits: Vec<f32>,
        episode_count: u32,
        success_count: u32,
    ) -> Result<Self, ScaffoldContractError> {
        if !mean_loss.is_finite()
            || mean_loss < 0.0
            || candidate_logits.len() != TRAINING_SEQUENCE_TICKS
            || speech_logits.len() != TRAINING_SEQUENCE_TICKS
            || candidate_logits.iter().any(|value| !value.is_finite())
            || speech_logits.iter().any(|value| !value.is_finite())
            || success_count > episode_count
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(Self {
            mean_loss,
            candidate_logits,
            speech_logits,
            episode_count,
            success_count,
        })
    }

    pub const fn mean_loss(&self) -> f32 {
        self.mean_loss
    }

    pub fn candidate_logits(&self) -> &[f32] {
        &self.candidate_logits
    }

    pub fn speech_logits(&self) -> &[f32] {
        &self.speech_logits
    }

    pub const fn episode_count(&self) -> u32 {
        self.episode_count
    }

    pub const fn success_count(&self) -> u32 {
        self.success_count
    }
}

pub(crate) const CANDIDATE_RECORD_WORDS: usize = 64;
