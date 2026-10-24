//! Offline CPU execution of the compiled sparse production graph.
//! The collector owns organism biology and outcomes; the trainer owns genetic
//! parameters, Adam state and detached replay. CPU execution is opt-in.

mod activity;
mod cohort;
mod collector_output;
mod collector_shard;
mod continuation;
mod host;
mod input;
mod objective;
mod organism;
mod replay;

pub use activity::*;
pub use cohort::*;
pub use collector_output::*;
pub use collector_shard::*;
pub use continuation::*;
pub use host::*;
pub use input::*;
pub use objective::*;
pub use organism::*;

use crate::{
    AdamWConfig, FoundationTrainerCheckpoint, StageTrainableMask, TrainingError,
    TrainingGradientProbe, TrainingReplayEvaluation, TrainingSequence,
};
use alife_core::{
    BrainPhenotype, FoundationWeightAsset, ScaffoldContractError, TrainingStageManifest,
};

fn invalid() -> TrainingError {
    ScaffoldContractError::InvalidDecisionEvidence.into()
}
fn finite(xs: &[f32]) -> Result<(), TrainingError> {
    if xs.iter().all(|x| x.is_finite()) {
        Ok(())
    } else {
        Err(invalid())
    }
}

/// CPU hardware is reported explicitly and never represented as a GPU receipt.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CpuHardwareReceipt {
    pub backend: &'static str,
    pub architecture: &'static str,
    pub logical_parallelism: usize,
}

/// Offline founder policy/optimizer. Cloning an organism never clones this.
pub struct CpuFoundationTrainer {
    phenotype: BrainPhenotype,
    source_foundation: FoundationWeightAsset,
    checkpoint: FoundationTrainerCheckpoint,
    graph: replay::SparseGraph,
}

impl CpuFoundationTrainer {
    pub fn new(
        phenotype: BrainPhenotype,
        source: FoundationWeightAsset,
        mask: StageTrainableMask,
        config: AdamWConfig,
    ) -> Result<Self, TrainingError> {
        config.validate()?;
        source.validate_against(&phenotype)?;
        mask.validate_for(&phenotype)?;
        if phenotype
            .projections()
            .iter()
            .any(|p| p.delay_microsteps() != 0)
        {
            return Err(invalid());
        }
        let weights = phenotype
            .synapses()
            .iter()
            .map(|s| s.genetic_weight())
            .collect::<Vec<_>>();
        let checkpoint = FoundationTrainerCheckpoint {
            schema_version: 2,
            phenotype_hash: phenotype.phenotype_hash(),
            source_foundation_digest: source.digest(),
            optimizer_step: 0,
            config,
            stage_mask: mask,
            first_moment: vec![0.; weights.len()],
            second_moment: vec![0.; weights.len()],
            update_ages: vec![0; weights.len()],
            weights,
        };
        let graph = replay::SparseGraph::new(&phenotype);
        Ok(Self {
            phenotype,
            source_foundation: source,
            checkpoint,
            graph,
        })
    }

    pub fn from_checkpoint(
        phenotype: BrainPhenotype,
        source: FoundationWeightAsset,
        checkpoint: FoundationTrainerCheckpoint,
    ) -> Result<Self, TrainingError> {
        checkpoint.validate_for(&phenotype, source.digest())?;
        let mut result = Self::new(
            phenotype,
            source,
            checkpoint.stage_mask.clone(),
            checkpoint.config,
        )?;
        result.checkpoint = checkpoint;
        Ok(result)
    }

    pub fn restore_checkpoint(
        &mut self,
        checkpoint: FoundationTrainerCheckpoint,
    ) -> Result<(), TrainingError> {
        checkpoint.validate_for(&self.phenotype, self.source_foundation.digest())?;
        self.checkpoint = checkpoint;
        Ok(())
    }
    pub fn phenotype(&self) -> &BrainPhenotype {
        &self.phenotype
    }
    pub fn weights(&self) -> &[f32] {
        &self.checkpoint.weights
    }
    pub fn checkpoint(&self) -> FoundationTrainerCheckpoint {
        self.checkpoint.clone()
    }
    pub fn optimizer_step(&self) -> u32 {
        self.checkpoint.optimizer_step
    }
    pub fn hardware_receipt(&self) -> CpuHardwareReceipt {
        CpuHardwareReceipt {
            backend: "native-cpu",
            architecture: std::env::consts::ARCH,
            logical_parallelism: std::thread::available_parallelism().map_or(1, usize::from),
        }
    }
    pub fn set_stage_mask(&mut self, mask: StageTrainableMask) -> Result<(), TrainingError> {
        mask.validate_for(&self.phenotype)?;
        self.checkpoint.stage_mask = mask;
        Ok(())
    }
    /// Admit an exported founder for the next fresh cohort without resetting
    /// either Adam moment, any per-coordinate age, the mask or global step.
    /// Every non-genetic coordinate and causal plan must remain identical.
    pub fn rebind_for_next_cohort(
        &mut self,
        next: BrainPhenotype,
        asset: FoundationWeightAsset,
    ) -> Result<(), TrainingError> {
        asset.validate_against(&next)?;
        self.checkpoint.stage_mask.validate_for(&next)?;
        let p = &self.phenotype;
        let binding = next
            .foundation_abi_selection()
            .canonical_v2()
            .ok_or_else(invalid)?;
        if p.foundation_abi_selection().canonical_v2().is_none()
            || binding.foundation_payload_digest() != Some(asset.digest())
            || p.schema_version() != next.schema_version()
            || p.brain_class_id() != next.brain_class_id()
            || p.neuron_count() != next.neuron_count()
            || p.microstep_count() != next.microstep_count()
            || p.sensor_profile() != next.sensor_profile()
            || p.lobe_layout() != next.lobe_layout()
            || p.language_codebook() != next.language_codebook()
            || p.cognitive_architecture() != next.cognitive_architecture()
            || p.projections() != next.projections()
            || p.neuron_dynamics() != next.neuron_dynamics()
            || p.sensor_encoder() != next.sensor_encoder()
            || p.candidate_decoder() != next.candidate_decoder()
            || p.speech_decoder() != next.speech_decoder()
            || p.memory_decoder() != next.memory_decoder()
            || p.cognitive_decoder() != next.cognitive_decoder()
            || p.cognitive_channel_plan() != next.cognitive_channel_plan()
            || p.plasticity_receptors() != next.plasticity_receptors()
            || p.replay_capture_plan() != next.replay_capture_plan()
            || p.sleep_consolidation_plan() != next.sleep_consolidation_plan()
            || p.plasticity_plan_digest() != next.plasticity_plan_digest()
            || p.persistent_address_map() != next.persistent_address_map()
            || p.route_abi_digest() != next.route_abi_digest()
            || p.plasticity_abi_digest() != next.plasticity_abi_digest()
            || p.budgets() != next.budgets()
            || p.synapses().len() != next.synapses().len()
            || p.synapses().iter().zip(next.synapses()).any(|(a, b)| {
                a.source() != b.source()
                    || a.target() != b.target()
                    || a.alpha().to_bits() != b.alpha().to_bits()
                    || a.route_index() != b.route_index()
                    || a.receptor_index() != b.receptor_index()
                    || a.kind() != b.kind()
            })
            || self
                .weights()
                .iter()
                .zip(asset.weights())
                .zip(next.synapses())
                .any(|((a, b), s)| {
                    a.to_bits() != b.to_bits() || a.to_bits() != s.genetic_weight().to_bits()
                })
        {
            return Err(invalid());
        }
        self.checkpoint.phenotype_hash = next.phenotype_hash();
        self.checkpoint.source_foundation_digest = asset.digest();
        self.graph = replay::SparseGraph::new(&next);
        self.phenotype = next;
        self.source_foundation = asset;
        Ok(())
    }
    pub fn export_candidate(
        &self,
        stage: TrainingStageManifest,
    ) -> Result<FoundationWeightAsset, TrainingError> {
        Ok(FoundationWeightAsset::from_trained_weights(
            &self.phenotype,
            self.weights().to_vec(),
            stage,
        )?)
    }
    pub fn evaluate_replay(
        &self,
        sequence: &TrainingSequence,
    ) -> Result<TrainingReplayEvaluation, TrainingError> {
        Ok(self.forward(sequence)?.evaluation)
    }
    pub fn probe_replay_gradients(
        &self,
        sequence: &TrainingSequence,
        adjoints: &[Vec<f32>],
    ) -> Result<TrainingGradientProbe, TrainingError> {
        self.probe_replay_gradients_with_speech(
            sequence,
            adjoints,
            &vec![None; sequence.ticks.len()],
        )
    }
    pub fn probe_replay_gradients_with_speech(
        &self,
        sequence: &TrainingSequence,
        adjoints: &[Vec<f32>],
        targets: &[Option<crate::ReplaySpeechTarget>],
    ) -> Result<TrainingGradientProbe, TrainingError> {
        sequence.validate_for(&self.phenotype)?;
        if adjoints.len() != sequence.ticks.len() - sequence.burn_in_ticks {
            return Err(invalid());
        }
        let mut full = sequence.ticks[..sequence.burn_in_ticks]
            .iter()
            .map(|tick| vec![0.; tick.candidates.len()])
            .collect::<Vec<_>>();
        full.extend_from_slice(adjoints);
        let tape = self.forward(sequence)?;
        self.graph.backward(
            &self.phenotype,
            self.weights(),
            sequence,
            &tape,
            &full,
            targets,
        )
    }
    fn forward(&self, sequence: &TrainingSequence) -> Result<replay::Tape, TrainingError> {
        sequence.validate_for(&self.phenotype)?;
        self.graph
            .forward(&self.phenotype, self.weights(), sequence)
    }
    /// One atomic AdamW commit after complete finite/age validation. Ages remain
    /// integers throughout serialization and restoration, including >2^24.
    pub fn apply_gradients(&mut self, gradients: &[f32]) -> Result<f32, TrainingError> {
        if gradients.len() != self.weights().len() {
            return Err(invalid());
        }
        finite(gradients)?;
        let c = self.checkpoint.config;
        let mut next = self.checkpoint.clone();
        next.optimizer_step = next.optimizer_step.checked_add(1).ok_or_else(invalid)?;
        let norm = gradients
            .iter()
            .enumerate()
            .filter(|(i, _)| next.stage_mask.is_trainable(*i))
            .map(|(_, g)| g * g)
            .sum::<f32>()
            .sqrt();
        if !norm.is_finite() {
            return Err(invalid());
        }
        let scale = if norm > c.gradient_clip {
            c.gradient_clip / norm
        } else {
            1.
        };
        for (i, g) in gradients.iter().enumerate() {
            if !next.stage_mask.is_trainable(i) {
                continue;
            }
            let age = next.update_ages[i].checked_add(1).ok_or_else(invalid)?;
            let (w, m, v) = adam_coordinate(
                next.weights[i],
                next.first_moment[i],
                next.second_moment[i],
                g * scale,
                age,
                c,
            )?;
            let projection = self.phenotype.projections()
                [self.phenotype.synapses()[i].route_index() as usize]
                .projection_type();
            next.weights[i] = match (self.phenotype.synapses()[i].kind(), projection) {
                (
                    alife_core::CompiledSynapseKind::Recurrent,
                    alife_core::ProjectionType::LateralInhibition,
                ) if w >= 0. => -0.0001,
                (
                    alife_core::CompiledSynapseKind::Recurrent,
                    alife_core::ProjectionType::Homeostatic
                    | alife_core::ProjectionType::MotorProposal,
                ) if w < 0. => 0.0001,
                _ => w,
            };
            next.first_moment[i] = m;
            next.second_moment[i] = v;
            next.update_ages[i] = age;
        }
        self.checkpoint = next;
        Ok(norm)
    }
}

fn adam_coordinate(
    w: f32,
    m: f32,
    v: f32,
    g: f32,
    age: u32,
    c: AdamWConfig,
) -> Result<(f32, f32, f32), TrainingError> {
    if age == 0 {
        return Err(invalid());
    }
    let m = c.beta1 * m + (1. - c.beta1) * g;
    let v = c.beta2 * v + (1. - c.beta2) * g * g;
    let next = w - c.learning_rate
        * ((m / (1. - c.beta1.powf(age as f32)))
            / ((v / (1. - c.beta2.powf(age as f32))).sqrt() + c.epsilon)
            + c.weight_decay * w);
    finite(&[next, m, v])?;
    Ok((next, m, v))
}
