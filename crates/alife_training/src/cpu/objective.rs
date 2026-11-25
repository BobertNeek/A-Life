use super::{
    adam_coordinate_with_denoms, adam_denoms, finite, invalid, parallel_map_indexed_with_limit,
    CpuFoundationTrainer,
};
use crate::{
    AdamWConfig, CriticResidualRow, ImitationExample, PpoConfig, PpoJointAction, PpoTrainingWindow,
    PpoUpdateReceipt, PpoValueHeadCheckpoint, TrainingError,
};

#[derive(Debug, Clone)]
pub struct CpuPpoObjective {
    pub joint_log_probability: f32,
    pub entropy: f32,
    pub policy_loss: f32,
    pub approximate_kl: f32,
    pub ratio: f32,
    pub clipped: bool,
    /// Includes division by temperature, but no batch reduction.
    pub logit_gradients: Vec<f32>,
}
#[derive(Default)]
struct Distribution {
    p: [f32; 32],
    lp: [f32; 32],
    entropy: f32,
}
fn shifted_logit(value: f32, maximum: f32, temperature: f32) -> f32 {
    // Opposite signs cannot suffer cancellation. Scale first above temperature one so
    // their difference does not overflow before a representable division.
    if temperature > 1. && value < 0. && maximum > 0. {
        value / temperature - maximum / temperature
    } else {
        (value - maximum) / temperature
    }
}
fn categorical(logits: &[f32], mask: u32, temperature: f32) -> Result<Distribution, TrainingError> {
    let mut d = Distribution::default();
    if mask == 0 {
        return Ok(d);
    }
    let max = logits
        .iter()
        .enumerate()
        .filter(|(i, _)| mask & (1u32 << i) != 0)
        .map(|(_, v)| *v)
        .fold(f32::NEG_INFINITY, f32::max);
    let mut z = 0.;
    for (i, v) in logits.iter().enumerate() {
        if mask & (1u32 << i) != 0 {
            d.p[i] = shifted_logit(*v, max, temperature).exp();
            z += d.p[i];
        }
    }
    if !z.is_finite() || z <= 0. {
        return Err(invalid());
    }
    for (i, v) in logits.iter().enumerate() {
        if mask & (1u32 << i) != 0 {
            d.lp[i] = shifted_logit(*v, max, temperature) - z.ln();
            d.p[i] /= z;
            if d.p[i] > 0. {
                d.entropy -= d.p[i] * d.lp[i];
            }
        }
    }
    Ok(d)
}

#[derive(Debug, Clone)]
pub struct CpuImitationObjective {
    pub loss: f32,
    pub logit_gradients: Vec<f32>,
}
/// Marginalize labelled commands over every compatible representative, including
/// its forced motor channel. Labels remain loss targets, never encoded inputs.
pub fn cpu_imitation_objective(
    logits: &[f32],
    example: &ImitationExample,
    temperature: f32,
    coefficient: f32,
) -> Result<CpuImitationObjective, TrainingError> {
    example.validate()?;
    finite(logits)?;
    if logits.len() != example.candidate_count as usize
        || !temperature.is_finite()
        || temperature <= 0.
        || !coefficient.is_finite()
        || coefficient < 0.
    {
        return Err(invalid());
    }
    let mut d = vec![categorical(
        logits,
        example.representative_mask,
        temperature,
    )?];
    for mask in example.motor_masks {
        d.push(categorical(logits, mask, temperature)?);
    }
    let mut accepted = Vec::new();
    let mut log_mass = [0.; 6];
    for s in 0..6 {
        let mask = example.target.motors[s].map_or(0, |m| m & example.motor_masks[s]);
        let a = categorical(logits, mask, temperature)?;
        if mask != 0 {
            let pivot = (0..logits.len())
                .find(|i| mask & (1u32 << i) != 0)
                .ok_or_else(invalid)?;
            log_mass[s] = d[s + 1].lp[pivot] - a.lp[pivot];
        }
        accepted.push(a);
    }
    let mut terms = vec![f32::NEG_INFINITY; logits.len()];
    for (r, term) in terms.iter_mut().enumerate() {
        if example
            .target
            .representative
            .unwrap_or(example.representative_mask)
            & (1 << r)
            == 0
        {
            continue;
        }
        let mut value = d[0].lp[r];
        let mut valid = true;
        for (s, mass) in log_mass.iter().enumerate() {
            if let Some(mask) = example.target.motors[s] {
                if example.forced_slots[r] == Some(s as u8) {
                    valid &= mask & (1 << r) != 0;
                } else {
                    valid &= mask & example.motor_masks[s] != 0;
                    value += mass;
                }
            }
        }
        if valid {
            *term = value;
        }
    }
    let max = terms.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let z = terms.iter().map(|t| (t - max).exp()).sum::<f32>();
    if !z.is_finite() || z <= 0. {
        return Err(invalid());
    }
    let logp = max + z.ln();
    let posterior = terms
        .iter()
        .map(|t| (t - max).exp() / z)
        .collect::<Vec<_>>();
    let mut forced = [0.; 6];
    for (r, &mass) in posterior.iter().enumerate() {
        if let Some(s) = example.forced_slots[r] {
            forced[s as usize] += mass;
        }
    }
    let gradients = (0..logits.len())
        .map(|i| {
            let mut gradient = d[0].p[i] - posterior[i];
            for s in 0..6 {
                if example.target.motors[s].is_some() {
                    gradient += (1. - forced[s]) * (d[s + 1].p[i] - accepted[s].p[i]);
                }
            }
            coefficient * gradient / temperature
        })
        .collect::<Vec<_>>();
    finite(&gradients)?;
    let loss = -coefficient * logp;
    finite(&[logp, loss])?;
    Ok(CpuImitationObjective {
        loss,
        logit_gradients: gradients,
    })
}
/// Exact joint representative/conditional motor objective from foundation_ppo.wgsl.
/// Legal attempt masks and receipt temperature are supplied, never inferred from
/// physical success. A representative's forced motor slot is counted only once.
pub fn cpu_ppo_objective(
    logits: &[f32],
    action: &PpoJointAction,
    advantage: f32,
    config: PpoConfig,
) -> Result<CpuPpoObjective, TrainingError> {
    config.validate()?;
    action.validate(config.temperature)?;
    finite(logits)?;
    if logits.len() != usize::from(action.candidate_count) || !advantage.is_finite() {
        return Err(invalid());
    }
    let mut d = Vec::new();
    d.push(categorical(
        logits,
        action.representative_mask,
        config.temperature,
    )?);
    for mask in action.motor_masks {
        d.push(categorical(logits, mask, config.temperature)?);
    }
    let representative = action.representative as usize;
    let forced = action.forced_slots[representative];
    let mut logp = d[0].lp[representative];
    for s in 0..6 {
        if forced != Some(s as u8) && action.motor_masks[s] != 0 {
            logp += d[s + 1].lp[action.motor_candidates[s].ok_or_else(invalid)? as usize];
        }
    }
    let mut forced_mass = [0.; 6];
    for (i, slot) in action.forced_slots.iter().enumerate() {
        if let Some(s) = slot {
            forced_mass[*s as usize] += d[0].p[i];
        }
    }
    let entropy = d[0].entropy
        + (0..6)
            .map(|s| (1. - forced_mass[s]) * d[s + 1].entropy)
            .sum::<f32>();
    let expected_forced = (0..6)
        .map(|s| forced_mass[s] * d[s + 1].entropy)
        .sum::<f32>();
    let log_ratio = logp - action.old_joint_log_probability;
    let ratio = log_ratio.exp();
    let clipped_ratio = ratio.clamp(1. - config.clip_ratio, 1. + config.clip_ratio);
    let policy_loss = -(ratio * advantage).min(clipped_ratio * advantage);
    let clipped = (advantage >= 0. && ratio > 1. + config.clip_ratio)
        || (advantage < 0. && ratio < 1. - config.clip_ratio);
    let logp_gradient = if clipped { 0. } else { -advantage * ratio };
    let mut gradients = vec![0.; logits.len()];
    for (i, g) in gradients.iter_mut().enumerate() {
        let p = d[0].p[i];
        let mut lp = f32::from((i == representative) as u8) - p;
        let mut ent = if p > 0. {
            -p * (d[0].lp[i]
                + d[0].entropy
                + action.forced_slots[i].map_or(0., |s| d[s as usize + 1].entropy)
                - expected_forced)
        } else {
            0.
        };
        for s in 0..6 {
            if action.motor_masks[s] != 0 {
                let p = d[s + 1].p[i];
                if forced != Some(s as u8) {
                    lp += f32::from((action.motor_candidates[s] == Some(i as u16)) as u8) - p;
                }
                if p > 0. {
                    ent -= (1. - forced_mass[s]) * p * (d[s + 1].lp[i] + d[s + 1].entropy);
                }
            }
        }
        *g = (logp_gradient * lp - config.entropy_coefficient * ent) / config.temperature;
    }
    let kl = ((ratio - 1.) - log_ratio).max(0.);
    finite(&[logp, entropy, policy_loss, ratio, kl])?;
    finite(&gradients)?;
    Ok(CpuPpoObjective {
        joint_log_probability: logp,
        entropy,
        policy_loss,
        approximate_kl: kl,
        ratio,
        clipped,
        logit_gradients: gradients,
    })
}

/// Native linear critic coefficients, bias and Adam state, detached from actor.
#[derive(Debug, Clone)]
pub struct CpuValueHead {
    checkpoint: PpoValueHeadCheckpoint,
}
impl CpuValueHead {
    pub fn from_checkpoint(checkpoint: PpoValueHeadCheckpoint) -> Result<Self, TrainingError> {
        let width = checkpoint.feature_count as usize + 1;
        if checkpoint.feature_count == 0
            || checkpoint.parameters.len() != width * 3
            || checkpoint.parameters[2 * width..].iter().any(|v| *v < 0.)
        {
            return Err(invalid());
        }
        finite(&checkpoint.parameters)?;
        Ok(Self { checkpoint })
    }
    pub fn checkpoint(&self) -> PpoValueHeadCheckpoint {
        self.checkpoint.clone()
    }
    pub fn predict(&self, features: &[f32]) -> Result<f32, TrainingError> {
        let n = self.checkpoint.feature_count as usize;
        if features.len() != n {
            return Err(invalid());
        }
        finite(features)?;
        let value = self.checkpoint.parameters[..n]
            .iter()
            .zip(features)
            .fold(self.checkpoint.parameters[n], |v, (w, x)| v + w * x);
        finite(&[value])?;
        Ok(value)
    }
    pub(super) fn proposal(
        &self,
        gradients: &[f32],
        config: AdamWConfig,
        policy: u64,
    ) -> Result<Self, TrainingError> {
        config.validate()?;
        finite(gradients)?;
        let width = self.checkpoint.feature_count as usize + 1;
        if gradients.len() != width {
            return Err(invalid());
        }
        let mut next = self.clone();
        let age = next
            .checkpoint
            .optimizer_step
            .checked_add(1)
            .ok_or_else(invalid)?;
        let norm = gradients.iter().map(|g| g * g).sum::<f32>().sqrt();
        if !norm.is_finite() {
            return Err(invalid());
        }
        let scale = (config.gradient_clip / norm.max(1e-12)).min(1.);
        let (b1_denom, b2_denom) = adam_denoms(age, config)?;
        for (i, g) in gradients.iter().enumerate() {
            let (w, m, v) = adam_coordinate_with_denoms(
                self.checkpoint.parameters[i],
                self.checkpoint.parameters[width + i],
                self.checkpoint.parameters[2 * width + i],
                g * scale,
                b1_denom,
                b2_denom,
                config,
            )?;
            next.checkpoint.parameters[i] = w;
            next.checkpoint.parameters[width + i] = m;
            next.checkpoint.parameters[2 * width + i] = v;
        }
        next.checkpoint.optimizer_step = age;
        next.checkpoint.last_updated_policy_version = Some(policy);
        Ok(next)
    }
    /// Full Bellman residual derivative through current and actual next features,
    /// including bias*(1-discount), as in critic_residual.wgsl.
    pub fn residual_step(
        &mut self,
        features: &[Vec<f32>],
        rows: &[CriticResidualRow],
        config: AdamWConfig,
        coefficient: f32,
        policy: u64,
    ) -> Result<f32, TrainingError> {
        let (loss, gradients) = self.residual_gradients(features, rows, coefficient)?;
        let next = self.proposal(&gradients, config, policy)?;
        *self = next;
        Ok(loss)
    }
    pub(super) fn residual_gradients(
        &self,
        features: &[Vec<f32>],
        rows: &[CriticResidualRow],
        coefficient: f32,
    ) -> Result<(f32, Vec<f32>), TrainingError> {
        if rows.is_empty() || !coefficient.is_finite() || coefficient <= 0. {
            return Err(invalid());
        }
        let values = features
            .iter()
            .map(|f| self.predict(f))
            .collect::<Result<Vec<_>, _>>()?;
        let width = self.checkpoint.feature_count as usize + 1;
        let mut gradients = vec![0.; width];
        let mut loss = 0.;
        for r in rows {
            if r.current as usize >= features.len()
                || r.next as usize >= features.len()
                || !r.reward.is_finite()
                || !r.discount.is_finite()
                || !(0.0..=1.0).contains(&r.discount)
            {
                return Err(invalid());
            }
            let current = r.current as usize;
            let next = r.next as usize;
            let residual = values[current] - r.reward - r.discount * values[next];
            loss += 0.5 * residual * residual / rows.len() as f32;
            for (i, g) in gradients.iter_mut().enumerate() {
                let delta = if i + 1 == width {
                    1. - r.discount
                } else {
                    features[current][i] - r.discount * features[next][i]
                };
                *g += coefficient * residual * delta / rows.len() as f32;
            }
        }
        finite(&[loss])?;
        finite(&gradients)?;
        Ok((loss, gradients))
    }
}

pub(super) type WindowEvaluation = (Vec<Vec<f32>>, Vec<CpuPpoObjective>, Vec<Vec<f32>>);
const PARALLEL_WINDOW_CHUNK: usize = 32;

pub(super) fn evaluate_window_with_tape(
    trainer: &CpuFoundationTrainer,
    window: &PpoTrainingWindow,
    config: PpoConfig,
    version: u64,
) -> Result<(WindowEvaluation, super::replay::Tape), TrainingError> {
    window.batch.validate_policy_version(version)?;
    window.sequence.validate_for(trainer.phenotype())?;
    if window.sequence.ticks.len() - window.sequence.burn_in_ticks != window.batch.len()
        || window.speech_targets.len() != window.sequence.ticks.len()
        || window
            .auxiliary
            .as_ref()
            .is_some_and(|(t, c)| t.len() != window.batch.len() || !c.is_finite() || *c < 0.)
    {
        return Err(invalid());
    }
    for (tick, target) in window.speech_targets.iter().enumerate() {
        if let Some(target) = target {
            target.validate()?;
            if tick < window.sequence.burn_in_ticks {
                return Err(invalid());
            }
        }
    }
    let tape = trainer.forward_validated(&window.sequence)?;
    let replay = &tape.evaluation;
    let mut adjoints = Vec::with_capacity(window.batch.len());
    let mut objectives = Vec::with_capacity(window.batch.len());
    let mut features = Vec::with_capacity(window.batch.len());
    for (i, t) in window.batch.transitions().iter().enumerate() {
        let tick = i + window.sequence.burn_in_ticks;
        let objective = cpu_ppo_objective(
            &replay.candidate_logits[tick],
            &t.action,
            window.batch.actor_advantage(i).ok_or_else(invalid)?,
            config,
        )?;
        let mut grad = objective
            .logit_gradients
            .iter()
            .map(|g| g / window.batch.len() as f32)
            .collect::<Vec<_>>();
        if let Some((targets, coefficient)) = &window.auxiliary {
            let example = ImitationExample {
                candidate_count: t.action.candidate_count,
                representative_mask: t.action.representative_mask,
                motor_masks: t.action.motor_masks,
                forced_slots: t.action.forced_slots.clone(),
                target: targets[i].clone(),
            };
            let imitation = cpu_imitation_objective(
                &replay.candidate_logits[tick],
                &example,
                config.temperature,
                *coefficient,
            )?;
            for (g, extra) in grad.iter_mut().zip(imitation.logit_gradients) {
                *g += extra / window.batch.len() as f32;
            }
        }
        adjoints.push(grad);
        objectives.push(objective);
        features.push(replay.final_activations[tick].clone());
    }
    Ok(((adjoints, objectives, features), tape))
}

pub(super) fn evaluate_window(
    trainer: &CpuFoundationTrainer,
    window: &PpoTrainingWindow,
    config: PpoConfig,
    version: u64,
) -> Result<WindowEvaluation, TrainingError> {
    evaluate_window_with_tape(trainer, window, config, version).map(|(eval, _)| eval)
}

fn validate_ppo_window(
    trainer: &CpuFoundationTrainer,
    value: &CpuValueHead,
    w: &PpoTrainingWindow,
    config: PpoConfig,
    version: u64,
) -> Result<usize, TrainingError> {
    let (_, objectives, features) = evaluate_window(trainer, w, config, version)?;
    for (r, f) in features.iter().enumerate() {
        let error = value.predict(f)? - w.batch.returns()[r];
        finite(&[error, 0.5 * config.value_coefficient * error * error])?;
    }
    if objectives
        .iter()
        .zip(w.batch.transitions())
        .any(|(o, t)| (o.joint_log_probability - t.action.old_joint_log_probability).abs() > 1e-3)
    {
        return Err(invalid());
    }
    Ok(w.batch.len())
}

type PpoWindowStepItem = (f32, f32, Vec<f32>);
fn evaluate_and_backward_ppo_window(
    trainer: &CpuFoundationTrainer,
    value: &CpuValueHead,
    w: &PpoTrainingWindow,
    config: PpoConfig,
    version: u64,
) -> Result<(Vec<f32>, Vec<PpoWindowStepItem>), TrainingError> {
    let ((adjoints, objectives, features), tape) =
        evaluate_window_with_tape(trainer, w, config, version)?;
    let probe = trainer.backward_with_tape(&w.sequence, &tape, &adjoints, &w.speech_targets)?;
    drop(tape);
    let mut step_items = Vec::with_capacity(objectives.len());
    for (r, (o, f)) in objectives.into_iter().zip(features).enumerate() {
        let error = value.predict(&f)? - w.batch.returns()[r];
        finite(&[error, 0.5 * config.value_coefficient * error * error])?;
        step_items.push((o.approximate_kl, error, f));
    }
    Ok((probe.gradients, step_items))
}

fn validate_ppo_cohort_inputs(
    trainer: &CpuFoundationTrainer,
    value: &CpuValueHead,
    window_count: usize,
    effective_batch_size: usize,
    config: PpoConfig,
    version: u64,
) -> Result<(), TrainingError> {
    config.validate()?;
    if window_count == 0
        || effective_batch_size == 0
        || value.checkpoint.feature_count != trainer.phenotype().neuron_count()
        || value
            .checkpoint
            .last_updated_policy_version
            .is_some_and(|p| version <= p)
    {
        return Err(invalid());
    }
    Ok(())
}

#[allow(dead_code)]
pub(super) fn train_cpu_ppo_cohort_slice(
    trainer: &mut CpuFoundationTrainer,
    value: &mut CpuValueHead,
    windows: &[PpoTrainingWindow],
    effective_batch_size: usize,
    config: PpoConfig,
    version: u64,
) -> Result<PpoUpdateReceipt, TrainingError> {
    train_cpu_ppo_cohort_slice_with_threads(
        trainer,
        value,
        windows,
        effective_batch_size,
        config,
        version,
        usize::MAX,
    )
}

pub(super) fn train_cpu_ppo_cohort_slice_with_threads(
    trainer: &mut CpuFoundationTrainer,
    value: &mut CpuValueHead,
    windows: &[PpoTrainingWindow],
    effective_batch_size: usize,
    config: PpoConfig,
    version: u64,
    max_threads: usize,
) -> Result<PpoUpdateReceipt, TrainingError> {
    let window_count = windows.len();
    validate_ppo_cohort_inputs(
        trainer,
        value,
        window_count,
        effective_batch_size,
        config,
        version,
    )?;
    let counts = parallel_map_indexed_with_limit(windows, max_threads, |_, w| {
        validate_ppo_window(trainer, value, w, config, version)
    })?;
    let mut receipt = PpoUpdateReceipt {
        collection_policy_version: version,
        completed_epochs: 0,
        stopped_for_kl: false,
        mean_kl_before_updates: Vec::new(),
        actor_optimizer_step: trainer.optimizer_step(),
        value_optimizer_step: value.checkpoint.optimizer_step,
    };
    for _ in 0..config.epochs {
        for start in (0..window_count).step_by(effective_batch_size) {
            let end = (start + effective_batch_size).min(window_count);
            let total = counts[start..end].iter().sum::<usize>();
            let mut gradients = vec![0.; trainer.weights().len()];
            let mut vg = vec![0.; value.checkpoint.feature_count as usize + 1];
            let mut kl = 0.;
            for chunk_start in (start..end).step_by(PARALLEL_WINDOW_CHUNK) {
                let chunk_end = (chunk_start + PARALLEL_WINDOW_CHUNK).min(end);
                let chunk = &windows[chunk_start..chunk_end];
                let results = parallel_map_indexed_with_limit(chunk, max_threads, |_, w| {
                    evaluate_and_backward_ppo_window(trainer, value, w, config, version)
                })?;
                for (w, (part_grads, step_items)) in chunk.iter().zip(results) {
                    let scale = w.batch.len() as f32 / total as f32;
                    for (g, part) in gradients.iter_mut().zip(part_grads) {
                        *g += part * scale;
                    }
                    for (approx_kl, error, f) in step_items {
                        kl += approx_kl / total as f32;
                        for (j, g) in vg.iter_mut().enumerate() {
                            *g += config.value_coefficient
                                * error
                                * if j < f.len() { f[j] } else { 1. }
                                / total as f32;
                        }
                    }
                }
            }
            if !kl.is_finite() {
                return Err(invalid());
            }
            receipt.mean_kl_before_updates.push(kl);
            if kl > config.target_kl {
                receipt.stopped_for_kl = true;
                return Ok(receipt);
            }
            let next_value = if config.value_coefficient > 0. {
                Some(value.proposal(&vg, config.value_optimizer, version)?)
            } else {
                None
            };
            // Actor is itself atomic; commit the already-validated critic last.
            trainer.apply_gradients(&gradients)?;
            if let Some(next) = next_value {
                *value = next;
            } else {
                value.checkpoint.last_updated_policy_version = Some(version);
            }
            receipt.actor_optimizer_step = trainer.optimizer_step();
            receipt.value_optimizer_step = value.checkpoint.optimizer_step;
        }
        receipt.completed_epochs += 1;
    }
    Ok(receipt)
}

/// Bounded-memory microbatch BPTT. Every window is freshly collected under one
/// founder version; frozen old log probabilities are checked before any update.
/// Only summed genetic gradients cross organism boundaries. Each optimizer steps
/// once per effective batch, independent of microbatch size.
#[allow(clippy::too_many_arguments)]
pub fn train_cpu_ppo_cohort<F>(
    trainer: &mut CpuFoundationTrainer,
    value: &mut CpuValueHead,
    window_count: usize,
    effective_batch_size: usize,
    mut load_window: F,
    config: PpoConfig,
    version: u64,
) -> Result<PpoUpdateReceipt, TrainingError>
where
    F: FnMut(usize) -> Result<PpoTrainingWindow, TrainingError>,
{
    validate_ppo_cohort_inputs(
        trainer,
        value,
        window_count,
        effective_batch_size,
        config,
        version,
    )?;
    let mut counts = Vec::with_capacity(window_count);
    for chunk_start in (0..window_count).step_by(PARALLEL_WINDOW_CHUNK) {
        let chunk_end = (chunk_start + PARALLEL_WINDOW_CHUNK).min(window_count);
        let mut chunk = Vec::with_capacity(chunk_end - chunk_start);
        for i in chunk_start..chunk_end {
            chunk.push(load_window(i)?);
        }
        let chunk_counts = parallel_map_indexed_with_limit(&chunk, usize::MAX, |_, w| {
            validate_ppo_window(trainer, value, w, config, version)
        })?;
        counts.extend(chunk_counts);
    }
    let mut receipt = PpoUpdateReceipt {
        collection_policy_version: version,
        completed_epochs: 0,
        stopped_for_kl: false,
        mean_kl_before_updates: Vec::new(),
        actor_optimizer_step: trainer.optimizer_step(),
        value_optimizer_step: value.checkpoint.optimizer_step,
    };
    for _ in 0..config.epochs {
        for start in (0..window_count).step_by(effective_batch_size) {
            let end = (start + effective_batch_size).min(window_count);
            let total = counts[start..end].iter().sum::<usize>();
            let mut gradients = vec![0.; trainer.weights().len()];
            let mut vg = vec![0.; value.checkpoint.feature_count as usize + 1];
            let mut kl = 0.;
            for chunk_start in (start..end).step_by(PARALLEL_WINDOW_CHUNK) {
                let chunk_end = (chunk_start + PARALLEL_WINDOW_CHUNK).min(end);
                let mut chunk = Vec::with_capacity(chunk_end - chunk_start);
                for (i, &count) in counts.iter().enumerate().take(chunk_end).skip(chunk_start) {
                    let w = load_window(i)?;
                    if w.batch.len() != count {
                        return Err(invalid());
                    }
                    chunk.push(w);
                }
                let results = parallel_map_indexed_with_limit(&chunk, usize::MAX, |_, w| {
                    evaluate_and_backward_ppo_window(trainer, value, w, config, version)
                })?;
                for (w, (part_grads, step_items)) in chunk.iter().zip(results) {
                    let scale = w.batch.len() as f32 / total as f32;
                    for (g, part) in gradients.iter_mut().zip(part_grads) {
                        *g += part * scale;
                    }
                    for (approx_kl, error, f) in step_items {
                        kl += approx_kl / total as f32;
                        for (j, g) in vg.iter_mut().enumerate() {
                            *g += config.value_coefficient
                                * error
                                * if j < f.len() { f[j] } else { 1. }
                                / total as f32;
                        }
                    }
                }
            }
            if !kl.is_finite() {
                return Err(invalid());
            }
            receipt.mean_kl_before_updates.push(kl);
            if kl > config.target_kl {
                receipt.stopped_for_kl = true;
                return Ok(receipt);
            }
            let next_value = if config.value_coefficient > 0. {
                Some(value.proposal(&vg, config.value_optimizer, version)?)
            } else {
                None
            };
            // Actor is itself atomic; commit the already-validated critic last.
            trainer.apply_gradients(&gradients)?;
            if let Some(next) = next_value {
                *value = next;
            } else {
                value.checkpoint.last_updated_policy_version = Some(version);
            }
            receipt.actor_optimizer_step = trainer.optimizer_step();
            receipt.value_optimizer_step = value.checkpoint.optimizer_step;
        }
        receipt.completed_epochs += 1;
    }
    Ok(receipt)
}

#[cfg(test)]
mod distribution_tests {
    use super::*;
    #[test]
    fn categorical_preserves_common_logit_shifts_at_nonunit_temperature() {
        let small = categorical(&[0., 0.125, 0.25], 7, 0.7).unwrap();
        let shifted = categorical(&[0., 0.125, 0.25].map(|v| v + 1_000_000.), 7, 0.7).unwrap();
        for i in 0..3 {
            assert!((small.p[i] - shifted.p[i]).abs() < 1e-6);
            assert!((small.lp[i] - shifted.lp[i]).abs() < 1e-6);
        }
    }
    #[test]
    fn equal_finite_logits_do_not_overflow_when_temperature_scaled() {
        let d = categorical(&[f32::MAX, f32::MAX], 3, 0.5).unwrap();
        assert_eq!(&d.p[..2], &[0.5, 0.5]);
        assert!(d.entropy.is_finite());
    }
    #[test]
    fn opposite_finite_extremes_remain_finite_at_larger_temperatures() {
        let d = categorical(&[-f32::MAX, f32::MAX], 3, 2.).unwrap();
        assert_eq!(d.lp[0], -f32::MAX);
        assert_eq!(&d.p[..2], &[0., 1.]);
        assert_eq!(d.entropy, 0.);
        let softer = categorical(&[-f32::MAX, f32::MAX], 3, 4.).unwrap();
        assert_eq!(softer.lp[0], -f32::MAX / 2.);
    }

    #[test]
    fn parallel_ppo_cohort_matches_single_threaded_bit_for_bit() {
        use crate::{
            PpoBatch, PpoBoundary, PpoTransition, StageTrainableMask, TrainingInitialState,
            TrainingReplayCandidate, TrainingReplayTick, TrainingSequence,
        };
        use alife_core::{
            BrainCapacityClass, BrainGenome, CandidateActionFamily, DendriticBranch,
            DendriticBranchSet, DendriticInputRef, DevelopmentState, FoundationWeightAsset,
            NormalizedScalar, PhenotypeCompiler, SensorProfile, Tick,
        };

        let capacity = BrainCapacityClass::n2048();
        let genome = BrainGenome::scaffold(0x7A11, capacity.id());
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.8).unwrap());
        let source =
            FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
        let p = PhenotypeCompiler::compile_from_foundation_asset(
            &genome,
            &capacity,
            &development,
            SensorProfile::GroundedObjectSlotsV1,
            &source,
        )
        .unwrap();
        let mask = StageTrainableMask::from_synapse_indices(
            &p,
            &(0..p.synapses().len() as u32).collect::<Vec<_>>(),
        )
        .unwrap();
        let make_trainer = || {
            CpuFoundationTrainer::new(
                p.clone(),
                source.clone(),
                mask.clone(),
                AdamWConfig::default(),
            )
            .unwrap()
        };
        let make_value = || {
            CpuValueHead::from_checkpoint(PpoValueHeadCheckpoint {
                feature_count: p.neuron_count(),
                optimizer_step: 42,
                parameters: vec![0.; (p.neuron_count() as usize + 1) * 3],
                last_updated_policy_version: Some(7399),
            })
            .unwrap()
        };
        let mut t_seq = make_trainer();
        let mut v_seq = make_value();
        let mut t_par = make_trainer();
        let mut v_par = make_value();

        let n = p.neuron_count() as usize;
        let config = PpoConfig {
            epochs: 2,
            ..Default::default()
        };
        let mut inputs = [0.; 54];
        for (i, v) in inputs.iter_mut().enumerate() {
            *v = 0.03 * (i % 5) as f32;
        }
        let candidates = [
            CandidateActionFamily::Approach,
            CandidateActionFamily::Ingest,
            CandidateActionFamily::Idle,
        ]
        .into_iter()
        .map(|family| TrainingReplayCandidate {
            family,
            decoder_inputs: inputs,
            innate_bias: 0.04,
        })
        .collect::<Vec<_>>();

        let windows: Vec<PpoTrainingWindow> = (0..5)
            .map(|w_idx| {
                let initial = TrainingInitialState {
                    activations: (0..n)
                        .map(|i| 0.05 + ((i + w_idx) % 7) as f32 * 0.01)
                        .collect(),
                    activity_ema: vec![0.2; n],
                    metabolic_load: vec![0.03; n],
                    dendrites: DendriticBranchSet::new(vec![DendriticBranch::new(
                        35,
                        0.02,
                        0.4,
                        vec![DendriticInputRef::new(6, 0.8).unwrap()],
                    )
                    .unwrap()])
                    .unwrap(),
                };
                let tick = TrainingReplayTick {
                    encoded_inputs: (0..n)
                        .map(|i| ((i + w_idx * 3) % 11) as f32 * 0.015)
                        .collect(),
                    projection_gain: 0.85,
                    local_threshold_shift: 0.017,
                    microstep_count: u32::from(p.microstep_count()),
                    enabled_routes: vec![true; p.projections().len()],
                    effective_weight_offsets: (0..p.synapses().len())
                        .map(|i| (i % 7) as f32 * 0.002)
                        .collect(),
                    structural_synapses: vec![],
                    candidates: candidates.clone(),
                };
                let seq = TrainingSequence {
                    phenotype_hash: p.phenotype_hash(),
                    initial,
                    ticks: vec![tick; 2],
                    burn_in_ticks: 0,
                    memory_candidate_gain: p
                        .candidate_decoder()
                        .memory_channel()
                        .map_or(0., |m| m.max_candidate_gain()),
                };
                let eval = t_seq.evaluate_replay(&seq).unwrap();
                let transitions = (0..2)
                    .map(|i| {
                        let mut a = PpoJointAction {
                            candidate_count: 3,
                            representative_mask: 7,
                            motor_masks: [0; 6],
                            representative: ((w_idx + i) % 3) as u16,
                            forced_slots: vec![None; 3],
                            motor_candidates: [None; 6],
                            old_joint_log_probability: -1.,
                            temperature: config.temperature,
                        };
                        a.old_joint_log_probability =
                            cpu_ppo_objective(&eval.candidate_logits[i], &a, 0., config)
                                .unwrap()
                                .joint_log_probability;
                        PpoTransition {
                            policy_version: 7400,
                            trajectory_id: w_idx as u64 + 1,
                            step: i as u64,
                            action: a,
                            reward: 0.1 + 0.07 * w_idx as f32 + 0.05 * i as f32,
                            old_value: 0.,
                            next_value: 0.,
                            elapsed_seconds: 0.05,
                            boundary: if i == 1 {
                                PpoBoundary::Terminated
                            } else {
                                PpoBoundary::Continuing
                            },
                        }
                    })
                    .collect();
                PpoTrainingWindow {
                    sequence: seq,
                    batch: PpoBatch::from_rollout(7400, transitions, config).unwrap(),
                    auxiliary: None,
                    speech_targets: vec![None; 2],
                }
            })
            .collect();

        let receipt_seq = train_cpu_ppo_cohort_slice_with_threads(
            &mut t_seq, &mut v_seq, &windows, 4, config, 7400, 1,
        )
        .unwrap();
        let receipt_par = train_cpu_ppo_cohort_slice_with_threads(
            &mut t_par,
            &mut v_par,
            &windows,
            4,
            config,
            7400,
            usize::MAX,
        )
        .unwrap();

        assert_eq!(receipt_seq, receipt_par);
        let c_seq = t_seq.checkpoint();
        let c_par = t_par.checkpoint();
        assert_eq!(c_seq.optimizer_step, c_par.optimizer_step);
        assert_eq!(c_seq.update_ages, c_par.update_ages);
        assert!(c_seq
            .weights
            .iter()
            .zip(&c_par.weights)
            .all(|(a, b)| a.to_bits() == b.to_bits()));
        assert!(c_seq
            .first_moment
            .iter()
            .zip(&c_par.first_moment)
            .all(|(a, b)| a.to_bits() == b.to_bits()));
        assert!(c_seq
            .second_moment
            .iter()
            .zip(&c_par.second_moment)
            .all(|(a, b)| a.to_bits() == b.to_bits()));
        let vc_seq = v_seq.checkpoint();
        let vc_par = v_par.checkpoint();
        assert_eq!(vc_seq.optimizer_step, vc_par.optimizer_step);
        assert_eq!(
            vc_seq.last_updated_policy_version,
            vc_par.last_updated_policy_version
        );
        assert!(vc_seq
            .parameters
            .iter()
            .zip(&vc_par.parameters)
            .all(|(a, b)| a.to_bits() == b.to_bits()));
    }
}
