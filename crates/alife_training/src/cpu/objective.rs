use super::{adam_coordinate, finite, invalid, CpuFoundationTrainer};
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
fn categorical(logits: &[f32], mask: u32, temperature: f32) -> Result<Distribution, TrainingError> {
    let mut d = Distribution::default();
    if mask == 0 {
        return Ok(d);
    }
    let max = logits
        .iter()
        .enumerate()
        .filter(|(i, _)| mask & (1u32 << i) != 0)
        .map(|(_, v)| v / temperature)
        .fold(f32::NEG_INFINITY, f32::max);
    let mut z = 0.;
    for (i, v) in logits.iter().enumerate() {
        if mask & (1u32 << i) != 0 {
            d.p[i] = (v / temperature - max).exp();
            z += d.p[i];
        }
    }
    if !z.is_finite() || z <= 0. {
        return Err(invalid());
    }
    for (i, v) in logits.iter().enumerate() {
        if mask & (1u32 << i) != 0 {
            d.lp[i] = v / temperature - max - z.ln();
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
    fn proposal(
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
        for (i, g) in gradients.iter().enumerate() {
            let (w, m, v) = adam_coordinate(
                self.checkpoint.parameters[i],
                self.checkpoint.parameters[width + i],
                self.checkpoint.parameters[2 * width + i],
                g * scale,
                age,
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
        let next = self.proposal(&gradients, config, policy)?;
        *self = next;
        Ok(loss)
    }
}

type WindowEvaluation = (Vec<Vec<f32>>, Vec<CpuPpoObjective>, Vec<Vec<f32>>);
fn evaluate_window(
    trainer: &CpuFoundationTrainer,
    window: &PpoTrainingWindow,
    config: PpoConfig,
    version: u64,
) -> Result<WindowEvaluation, TrainingError> {
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
    let replay = trainer.evaluate_replay(&window.sequence)?;
    let mut adjoints = window.sequence.ticks[window.sequence.burn_in_ticks..]
        .iter()
        .map(|t| vec![0.; t.candidates.len()])
        .collect::<Vec<_>>();
    let mut objectives = Vec::new();
    let mut features = Vec::new();
    for (i, t) in window.batch.transitions().iter().enumerate() {
        let tick = i + window.sequence.burn_in_ticks;
        let objective = cpu_ppo_objective(
            &replay.candidate_logits[tick],
            &t.action,
            window.batch.actor_advantage(i).ok_or_else(invalid)?,
            config,
        )?;
        adjoints[i] = objective
            .logit_gradients
            .iter()
            .map(|g| g / window.batch.len() as f32)
            .collect();
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
            for (g, extra) in adjoints[i].iter_mut().zip(imitation.logit_gradients) {
                *g += extra / window.batch.len() as f32;
            }
        }
        objectives.push(objective);
        features.push(replay.final_activations[tick].clone());
    }
    Ok((adjoints, objectives, features))
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
    let mut counts = Vec::new();
    for i in 0..window_count {
        let w = load_window(i)?;
        let (_, objectives, features) = evaluate_window(trainer, &w, config, version)?;
        for (r, f) in features.iter().enumerate() {
            let error = value.predict(f)? - w.batch.returns()[r];
            finite(&[error, 0.5 * config.value_coefficient * error * error])?;
        }
        if objectives.iter().zip(w.batch.transitions()).any(|(o, t)| {
            (o.joint_log_probability - t.action.old_joint_log_probability).abs() > 1e-3
        }) {
            return Err(invalid());
        }
        counts.push(w.batch.len());
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
            for (i, &count) in counts.iter().enumerate().take(end).skip(start) {
                let w = load_window(i)?;
                if w.batch.len() != count {
                    return Err(invalid());
                }
                let (adjoints, objectives, features) =
                    evaluate_window(trainer, &w, config, version)?;
                let probe = trainer.probe_replay_gradients_with_speech(
                    &w.sequence,
                    &adjoints,
                    &w.speech_targets,
                )?;
                let scale = count as f32 / total as f32;
                for (g, part) in gradients.iter_mut().zip(probe.gradients) {
                    *g += part * scale;
                }
                for (r, (o, f)) in objectives.iter().zip(&features).enumerate() {
                    kl += o.approximate_kl / total as f32;
                    let error = value.predict(f)? - w.batch.returns()[r];
                    finite(&[error, 0.5 * config.value_coefficient * error * error])?;
                    for (j, g) in vg.iter_mut().enumerate() {
                        *g +=
                            config.value_coefficient * error * if j < f.len() { f[j] } else { 1. }
                                / total as f32;
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
