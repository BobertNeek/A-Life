//! Opt-in action-conditioned successor pilot. This head exists only in offline
//! training memory; neither foundation assets nor gameplay saves contain it.
//! Observations and full categorical motor bundles are caller-supplied evidence,
//! not generated rewards or proof of provenance. Lifetime plasticity is frozen
//! replay context; only native genetic recurrent weights receive BPTT/Adam.

use super::{finite, invalid, CpuFoundationTrainer};
use crate::{TrainingError, TrainingGradientProbe, TrainingSequence};
use alife_core::{BrainPhenotype, CompiledSynapseKind, PhenotypeHash};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PredictiveSplit {
    Train,
    Validation,
    Test,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotorChannel {
    pub name: String,
    /// Include an explicit no-op choice when the channel can be inactive.
    pub choices: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PredictiveSchema {
    pub version: String,
    /// Each observation is sensors followed by body features, in these orders.
    /// Units/scaling must be fixed before collecting the evaluation split.
    pub sensors: Vec<String>,
    pub body: Vec<String>,
    /// Every simultaneously executable categorical channel, in canonical order.
    pub motors: Vec<MotorChannel>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PredictiveEpisode {
    pub episode_id: String,
    /// All windows from the same life/environment/collection group share a key.
    /// A group cannot appear in more than one split, even for disjoint windows.
    pub split_group: String,
    pub split: PredictiveSplit,
    pub replay: TrainingSequence,
    /// N+1 observations for N replay ticks; terminal observation is target only.
    pub observations: Vec<Vec<f32>>,
    /// N full motor bundles; each choice index is encoded one-hot, never scalar.
    pub actions: Vec<Vec<usize>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PredictiveDataset {
    pub schema_version: u16,
    /// State whether this is an authored contract fixture or recorded evidence.
    /// This description is not a provenance attestation.
    pub source_description: String,
    pub schema: PredictiveSchema,
    pub episodes: Vec<PredictiveEpisode>,
}

fn names(values: &[String], max: usize) -> bool {
    !values.is_empty()
        && values.len() <= max
        && values
            .iter()
            .all(|v| !v.trim().is_empty() && v.len() <= 256)
        && values.iter().collect::<BTreeSet<_>>().len() == values.len()
}
impl PredictiveSchema {
    fn feature_count(&self) -> usize {
        self.sensors.len() + self.body.len()
    }
    fn action_width(&self) -> usize {
        self.motors.iter().map(|m| m.choices.len()).sum()
    }
    fn validate(&self) -> Result<(), TrainingError> {
        if self.version.trim().is_empty()
            || self.version.len() > 256
            || !names(&self.sensors, 128)
            || !names(&self.body, 128)
            || self.sensors.iter().any(|s| self.body.contains(s))
            || !names(
                &self
                    .motors
                    .iter()
                    .map(|m| m.name.clone())
                    .collect::<Vec<_>>(),
                32,
            )
            || self.motors.iter().any(|m| !names(&m.choices, 64))
        {
            return Err(invalid());
        }
        Ok(())
    }
}
impl PredictiveDataset {
    pub fn validate_for(&self, p: &BrainPhenotype) -> Result<(), TrainingError> {
        self.schema.validate()?;
        if self.schema_version != 1
            || self.source_description.trim().is_empty()
            || self.source_description.len() > 4096
            || self.episodes.is_empty()
            || self.episodes.len() > 4096
            || !self
                .episodes
                .iter()
                .any(|e| e.split == PredictiveSplit::Train)
            || !self
                .episodes
                .iter()
                .any(|e| e.split == PredictiveSplit::Validation)
        {
            return Err(invalid());
        }
        let mut ids = BTreeSet::new();
        let mut groups = BTreeMap::new();
        let mut contents = BTreeMap::new();
        for e in &self.episodes {
            e.replay.validate_for(p)?;
            if e.episode_id.trim().is_empty()
                || e.episode_id.len() > 256
                || e.split_group.trim().is_empty()
                || e.split_group.len() > 256
                || !ids.insert(&e.episode_id)
                || groups
                    .insert(&e.split_group, e.split)
                    .is_some_and(|s| s != e.split)
                || e.observations.len() != e.replay.ticks.len() + 1
                || e.actions.len() != e.replay.ticks.len()
                || e.observations
                    .iter()
                    .any(|o| o.len() != self.schema.feature_count())
                || e.actions.iter().any(|a| {
                    a.len() != self.schema.motors.len()
                        || a.iter()
                            .zip(&self.schema.motors)
                            .any(|(&i, m)| i >= m.choices.len())
                })
            {
                return Err(invalid());
            }
            for o in &e.observations {
                finite(o)?;
            }
            // Catch exact copied evidence across splits even if IDs were changed.
            // Group membership remains caller-owned; this is no general near-
            // duplicate or collection-provenance detector.
            let bytes = serde_json::to_vec(&(&e.replay, &e.observations, &e.actions))
                .map_err(|_| invalid())?;
            use sha2::{Digest, Sha256};
            let digest: [u8; 32] = Sha256::digest(bytes).into();
            if contents
                .insert(digest, e.split)
                .is_some_and(|s| s != e.split)
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
}

/// Small nonlinear state/action interaction model; deterministic initialization.
/// No serialization implementation: this experimental head is disposable.
#[derive(Debug, Clone, PartialEq)]
pub struct PredictiveHead {
    phenotype_hash: PhenotypeHash,
    schema: PredictiveSchema,
    neurons: Vec<usize>,
    hidden: usize,
    input_weights: Vec<f32>,
    hidden_bias: Vec<f32>,
    output_weights: Vec<f32>,
    output_bias: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PredictiveMetrics {
    /// Mean over scored ticks and sensor/body coordinates; burn-in is excluded.
    pub mse: f64,
    pub scored_ticks: usize,
}

pub struct PredictiveProbe {
    pub metrics: PredictiveMetrics,
    pub predictions: Vec<Vec<f32>>,
    pub recurrent: TrainingGradientProbe,
}

struct HeadPass {
    metrics: PredictiveMetrics,
    predictions: Vec<Vec<f32>>,
    activation_adjoints: Vec<Vec<f32>>,
    gradients: [Vec<f32>; 4],
}

impl PredictiveHead {
    pub fn new(
        p: &BrainPhenotype,
        schema: PredictiveSchema,
        neurons: Vec<usize>,
        hidden: usize,
        seed: u64,
    ) -> Result<Self, TrainingError> {
        schema.validate()?;
        if neurons.is_empty()
            || neurons.len() > 256
            || neurons.iter().any(|&i| i >= p.neuron_count() as usize)
            || neurons.iter().collect::<BTreeSet<_>>().len() != neurons.len()
            || !(1..=64).contains(&hidden)
        {
            return Err(invalid());
        }
        let input = neurons.len() + schema.feature_count() + schema.action_width();
        let output = schema.feature_count();
        let mut state = seed;
        let mut weights = |count| {
            (0..count)
                .map(|_| {
                    // SplitMix64; fixed integer operations, no process RNG state.
                    state = state.wrapping_add(0x9e3779b97f4a7c15);
                    let mut z = state;
                    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
                    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
                    z ^= z >> 31;
                    ((z >> 40) as f32 / 16777216. - 0.5) * 0.2
                })
                .collect()
        };
        Ok(Self {
            phenotype_hash: p.phenotype_hash(),
            schema,
            neurons,
            hidden,
            input_weights: weights(input * hidden),
            hidden_bias: vec![0.; hidden],
            output_weights: weights(output * hidden),
            output_bias: vec![0.; output],
        })
    }

    fn validate(
        &self,
        trainer: &CpuFoundationTrainer,
        data: &PredictiveDataset,
    ) -> Result<(), TrainingError> {
        data.validate_for(trainer.phenotype())?;
        if self.phenotype_hash != trainer.phenotype().phenotype_hash() || self.schema != data.schema
        {
            return Err(invalid());
        }
        Ok(())
    }

    fn pass(
        &self,
        e: &PredictiveEpisode,
        activations: &[Vec<f32>],
    ) -> Result<HeadPass, TrainingError> {
        let ticks = e.replay.ticks.len();
        let scored_ticks = ticks - e.replay.burn_in_ticks;
        let outputs = self.schema.feature_count();
        let inputs = self.input_weights.len() / self.hidden;
        let normalizer = (scored_ticks * outputs) as f32;
        let mut result = HeadPass {
            metrics: PredictiveMetrics {
                mse: 0.,
                scored_ticks,
            },
            predictions: Vec::with_capacity(ticks),
            activation_adjoints: activations.iter().map(|a| vec![0.; a.len()]).collect(),
            gradients: [
                vec![0.; self.input_weights.len()],
                vec![0.; self.hidden],
                vec![0.; self.output_weights.len()],
                vec![0.; outputs],
            ],
        };
        for (t, activation) in activations.iter().enumerate().take(ticks) {
            // Only current observation/action and causal recurrent state enter x.
            let mut x = self
                .neurons
                .iter()
                .map(|&i| activation[i])
                .collect::<Vec<_>>();
            x.extend_from_slice(&e.observations[t]);
            for (&choice, channel) in e.actions[t].iter().zip(&self.schema.motors) {
                x.extend((0..channel.choices.len()).map(|i| f32::from(i == choice)));
            }
            let h = self
                .input_weights
                .chunks_exact(inputs)
                .zip(&self.hidden_bias)
                .map(|(w, b)| (b + w.iter().zip(&x).map(|(w, x)| w * x).sum::<f32>()).tanh())
                .collect::<Vec<_>>();
            let prediction = self
                .output_weights
                .chunks_exact(self.hidden)
                .zip(&self.output_bias)
                .map(|(w, b)| b + w.iter().zip(&h).map(|(w, h)| w * h).sum::<f32>())
                .collect::<Vec<_>>();
            finite(&prediction)?;
            if t >= e.replay.burn_in_ticks {
                let mut dh = vec![0.; self.hidden];
                for (o, &value) in prediction.iter().enumerate() {
                    let error = value - e.observations[t + 1][o];
                    result.metrics.mse += f64::from(error).powi(2) / f64::from(normalizer);
                    let dy = 2. * error / normalizer;
                    result.gradients[3][o] += dy;
                    for j in 0..self.hidden {
                        result.gradients[2][o * self.hidden + j] += dy * h[j];
                        dh[j] += dy * self.output_weights[o * self.hidden + j];
                    }
                }
                for j in 0..self.hidden {
                    let dz = dh[j] * (1. - h[j] * h[j]);
                    result.gradients[1][j] += dz;
                    for (i, &value) in x.iter().enumerate() {
                        result.gradients[0][j * inputs + i] += dz * value;
                        if i < self.neurons.len() {
                            result.activation_adjoints[t][self.neurons[i]] +=
                                dz * self.input_weights[j * inputs + i];
                        }
                    }
                }
            }
            result.predictions.push(prediction);
        }
        for g in &result.gradients {
            finite(g)?;
        }
        if !result.metrics.mse.is_finite() {
            return Err(invalid());
        }
        Ok(result)
    }

    /// Read-only probe uses actual native recurrent reverse mode, not finite
    /// differences or surrogate gradients. Burn-in is a detached boundary.
    pub fn probe(
        &self,
        trainer: &CpuFoundationTrainer,
        data: &PredictiveDataset,
        index: usize,
    ) -> Result<PredictiveProbe, TrainingError> {
        self.validate(trainer, data)?;
        let e = data.episodes.get(index).ok_or_else(invalid)?;
        let tape = trainer.forward(&e.replay)?;
        let pass = self.pass(e, &tape.evaluation.final_activations)?;
        let recurrent = trainer.graph.backward_from_activations(
            trainer.phenotype(),
            trainer.weights(),
            &e.replay,
            &tape,
            &pass.activation_adjoints,
        )?;
        Ok(PredictiveProbe {
            metrics: pass.metrics,
            predictions: pass.predictions,
            recurrent,
        })
    }

    pub fn evaluate(
        &self,
        trainer: &CpuFoundationTrainer,
        data: &PredictiveDataset,
        split: PredictiveSplit,
    ) -> Result<PredictiveMetrics, TrainingError> {
        self.validate(trainer, data)?;
        let mut metrics = PredictiveMetrics {
            mse: 0.,
            scored_ticks: 0,
        };
        for e in data.episodes.iter().filter(|e| e.split == split) {
            let evaluation = trainer.evaluate_replay(&e.replay)?;
            let pass = self.pass(e, &evaluation.final_activations)?;
            metrics.mse += pass.metrics.mse * pass.metrics.scored_ticks as f64;
            metrics.scored_ticks += pass.metrics.scored_ticks;
        }
        if metrics.scored_ticks == 0 {
            return Err(invalid());
        }
        metrics.mse /= metrics.scored_ticks as f64;
        Ok(metrics)
    }

    /// One episode: mean-MSE SGD for this head, existing clipped AdamW for native
    /// recurrent weights. Both updates are checked before either is committed.
    /// The caller must explicitly freeze decoder coordinates in the stage mask.
    pub fn train_step(
        &mut self,
        trainer: &mut CpuFoundationTrainer,
        data: &PredictiveDataset,
        index: usize,
        head_learning_rate: f32,
    ) -> Result<PredictiveMetrics, TrainingError> {
        self.validate(trainer, data)?;
        let e = data.episodes.get(index).ok_or_else(invalid)?;
        if e.split != PredictiveSplit::Train
            || !head_learning_rate.is_finite()
            || head_learning_rate <= 0.
            || trainer
                .phenotype()
                .synapses()
                .iter()
                .enumerate()
                .any(|(i, s)| {
                    trainer.checkpoint.stage_mask.is_trainable(i)
                        && s.kind() != CompiledSynapseKind::Recurrent
                })
        {
            return Err(invalid());
        }
        let tape = trainer.forward(&e.replay)?;
        let pass = self.pass(e, &tape.evaluation.final_activations)?;
        let probe = trainer.graph.backward_from_activations(
            trainer.phenotype(),
            trainer.weights(),
            &e.replay,
            &tape,
            &pass.activation_adjoints,
        )?;
        let mut next = self.clone();
        for (weights, gradient) in [
            &mut next.input_weights,
            &mut next.hidden_bias,
            &mut next.output_weights,
            &mut next.output_bias,
        ]
        .into_iter()
        .zip(&pass.gradients)
        {
            for (w, g) in weights.iter_mut().zip(gradient) {
                *w -= head_learning_rate * g;
            }
            finite(weights)?;
        }
        trainer.apply_gradients(&probe.gradients)?;
        *self = next;
        Ok(pass.metrics)
    }
}

#[cfg(test)]
mod tests;
