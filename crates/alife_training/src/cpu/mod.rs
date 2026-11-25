//! Offline CPU execution of the compiled sparse production graph.
//! The collector owns organism biology and outcomes; the trainer owns genetic
//! parameters, Adam state and detached replay. CPU execution is opt-in.

mod activity;
mod cohort;
mod collector_output;
mod collector_shard;
mod continuation;
mod cooperative_round;
mod cooperative_source;
mod host;
mod input;
mod objective;
mod organism;
mod private_checkpoint;
mod replay;
mod round_output;

pub use activity::*;
pub use cohort::*;
pub use collector_output::*;
pub use collector_shard::*;
pub use continuation::*;
pub use cooperative_round::*;
pub use cooperative_source::*;
pub use host::*;
pub use input::*;
pub use objective::*;
pub use organism::*;
pub use private_checkpoint::*;
pub use round_output::{read_cpu_cooperative_round, CPU_COOPERATIVE_ROUND_RESULT_FILE};

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

pub(super) fn parallel_map_indexed<T, R, F>(items: &[T], f: F) -> Result<Vec<R>, TrainingError>
where
    T: Sync,
    R: Send,
    F: Fn(usize, &T) -> Result<R, TrainingError> + Sync,
{
    parallel_map_indexed_with_limit(items, usize::MAX, f)
}

pub(super) fn parallel_map_indexed_with_limit<T, R, F>(
    items: &[T],
    max_threads: usize,
    f: F,
) -> Result<Vec<R>, TrainingError>
where
    T: Sync,
    R: Send,
    F: Fn(usize, &T) -> Result<R, TrainingError> + Sync,
{
    if items.is_empty() {
        return Ok(Vec::new());
    }
    let threads = if items.len() <= 1 || max_threads <= 1 {
        1
    } else {
        std::thread::available_parallelism()
            .map_or(1, usize::from)
            .min(items.len())
            .min(max_threads)
            .max(1)
    };
    if threads == 1 {
        return items
            .iter()
            .enumerate()
            .map(|(i, item)| f(i, item))
            .collect();
    }
    let cursor = std::sync::atomic::AtomicUsize::new(0);
    let mut slots: Vec<Option<Result<R, TrainingError>>> = (0..items.len()).map(|_| None).collect();
    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(threads);
        for _ in 0..threads {
            let cursor = &cursor;
            let f = &f;
            handles.push(scope.spawn(move || {
                let mut local = Vec::new();
                loop {
                    let idx = cursor.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if idx >= items.len() {
                        break;
                    }
                    local.push((idx, f(idx, &items[idx])));
                }
                local
            }));
        }
        for handle in handles {
            for (idx, res) in handle.join().expect("CPU training worker panicked") {
                slots[idx] = Some(res);
            }
        }
    });
    let mut out = Vec::with_capacity(items.len());
    for slot in slots {
        out.push(slot.expect("CPU training slot missing")?);
    }
    Ok(out)
}

/// CPU hardware is reported explicitly and never represented as a GPU receipt.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CpuHardwareReceipt {
    pub backend: &'static str,
    pub architecture: &'static str,
    pub logical_parallelism: usize,
}

fn compute_weight_digest(weights: &[f32]) -> Result<[u64; 4], TrainingError> {
    let mut d = alife_core::CanonicalDigestBuilder::new(b"ALIFE-CPU-FOUNDER-WEIGHTS-V1");
    for &w in weights {
        d.write_f32(w)?;
    }
    Ok(d.finish256())
}
fn compute_weights_match_genetic(weights: &[f32], p: &BrainPhenotype) -> bool {
    weights
        .iter()
        .zip(p.synapses())
        .all(|(w, s)| w.to_bits() == s.genetic_weight().to_bits())
}

/// Offline founder policy/optimizer. Cloning an organism never clones this.
pub struct CpuFoundationTrainer {
    phenotype: BrainPhenotype,
    source_foundation: FoundationWeightAsset,
    checkpoint: FoundationTrainerCheckpoint,
    graph: replay::SparseGraph,
    receptor_phenotype: alife_core::NeuralReceptorPhenotype,
    weight_digest: [u64; 4],
    weights_match_genetic: bool,
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
        let receptor_phenotype = alife_core::NeuralReceptorPhenotype::compile(&phenotype)?;
        let weights = phenotype
            .synapses()
            .iter()
            .map(|s| s.genetic_weight())
            .collect::<Vec<_>>();
        let weight_digest = compute_weight_digest(&weights)?;
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
            receptor_phenotype,
            weight_digest,
            weights_match_genetic: true,
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
        result.weight_digest = compute_weight_digest(&checkpoint.weights)?;
        result.weights_match_genetic =
            compute_weights_match_genetic(&checkpoint.weights, &result.phenotype);
        result.checkpoint = checkpoint;
        Ok(result)
    }

    pub fn restore_checkpoint(
        &mut self,
        checkpoint: FoundationTrainerCheckpoint,
    ) -> Result<(), TrainingError> {
        checkpoint.validate_for(&self.phenotype, self.source_foundation.digest())?;
        let weight_digest = compute_weight_digest(&checkpoint.weights)?;
        let weights_match_genetic =
            compute_weights_match_genetic(&checkpoint.weights, &self.phenotype);
        self.checkpoint = checkpoint;
        self.weight_digest = weight_digest;
        self.weights_match_genetic = weights_match_genetic;
        Ok(())
    }
    pub fn phenotype(&self) -> &BrainPhenotype {
        &self.phenotype
    }
    pub(super) fn receptor_phenotype(&self) -> &alife_core::NeuralReceptorPhenotype {
        &self.receptor_phenotype
    }
    pub(super) fn weight_digest(&self) -> [u64; 4] {
        self.weight_digest
    }
    pub(super) fn weights_match_genetic(&self) -> bool {
        self.weights_match_genetic
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
        let receptor_phenotype = alife_core::NeuralReceptorPhenotype::compile(&next)?;
        self.checkpoint.phenotype_hash = next.phenotype_hash();
        self.checkpoint.source_foundation_digest = asset.digest();
        self.graph = replay::SparseGraph::new(&next);
        self.phenotype = next;
        self.source_foundation = asset;
        self.receptor_phenotype = receptor_phenotype;
        self.weights_match_genetic = true;
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
        let tape = self.forward(sequence)?;
        self.backward_with_tape(sequence, &tape, adjoints, targets)
    }
    fn backward_with_tape(
        &self,
        sequence: &TrainingSequence,
        tape: &replay::Tape,
        adjoints: &[Vec<f32>],
        targets: &[Option<crate::ReplaySpeechTarget>],
    ) -> Result<TrainingGradientProbe, TrainingError> {
        self.graph.backward(
            &self.phenotype,
            self.weights(),
            sequence,
            tape,
            adjoints,
            targets,
        )
    }
    fn forward(&self, sequence: &TrainingSequence) -> Result<replay::Tape, TrainingError> {
        sequence.validate_for(&self.phenotype)?;
        self.forward_validated(sequence)
    }
    fn forward_validated(
        &self,
        sequence: &TrainingSequence,
    ) -> Result<replay::Tape, TrainingError> {
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
        let mut cached_denoms: Option<(u32, f32, f32)> = None;
        for (i, g) in gradients.iter().enumerate() {
            if !next.stage_mask.is_trainable(i) {
                continue;
            }
            let age = next.update_ages[i].checked_add(1).ok_or_else(invalid)?;
            let (b1_denom, b2_denom) = match cached_denoms {
                Some((prev_age, d1, d2)) if prev_age == age => (d1, d2),
                _ => {
                    let (d1, d2) = adam_denoms(age, c)?;
                    cached_denoms = Some((age, d1, d2));
                    (d1, d2)
                }
            };
            let (w, m, v) = adam_coordinate_with_denoms(
                next.weights[i],
                next.first_moment[i],
                next.second_moment[i],
                g * scale,
                b1_denom,
                b2_denom,
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
        let weight_digest = compute_weight_digest(&next.weights)?;
        let weights_match_genetic = compute_weights_match_genetic(&next.weights, &self.phenotype);
        self.checkpoint = next;
        self.weight_digest = weight_digest;
        self.weights_match_genetic = weights_match_genetic;
        Ok(norm)
    }
}

pub(super) fn adam_denoms(age: u32, c: AdamWConfig) -> Result<(f32, f32), TrainingError> {
    if age == 0 {
        return Err(invalid());
    }
    Ok((1. - c.beta1.powf(age as f32), 1. - c.beta2.powf(age as f32)))
}

pub(super) fn adam_coordinate_with_denoms(
    w: f32,
    m: f32,
    v: f32,
    g: f32,
    b1_denom: f32,
    b2_denom: f32,
    c: AdamWConfig,
) -> Result<(f32, f32, f32), TrainingError> {
    let m = c.beta1 * m + (1. - c.beta1) * g;
    let v = c.beta2 * v + (1. - c.beta2) * g * g;
    let next = w - c.learning_rate
        * ((m / b1_denom) / ((v / b2_denom).sqrt() + c.epsilon) + c.weight_decay * w);
    finite(&[next, m, v])?;
    Ok((next, m, v))
}

#[allow(dead_code)]
fn adam_coordinate(
    w: f32,
    m: f32,
    v: f32,
    g: f32,
    age: u32,
    c: AdamWConfig,
) -> Result<(f32, f32, f32), TrainingError> {
    let (b1_denom, b2_denom) = adam_denoms(age, c)?;
    adam_coordinate_with_denoms(w, m, v, g, b1_denom, b2_denom, c)
}
