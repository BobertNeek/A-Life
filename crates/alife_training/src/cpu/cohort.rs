//! Canonical rollout bookkeeping with full Bellman residual critic updates.
use super::*;
use crate::{
    CriticResidualRow, PpoBatch, PpoBoundary, PpoConfig, PpoTrainingWindow, PpoTransition,
    PpoUpdateReceipt, TrainingError, TrainingSequence,
};
use alife_core::{CreaturePhenotype, MeasuredPhysiologyTransition, NeuromodulatorSample};
#[derive(Clone)]
pub struct CpuResidualTrainingWindow {
    pub actor: PpoTrainingWindow,
    /// Captured current/actual-next states, fixed before actor/critic updates.
    pub critic_features: Vec<Vec<f32>>,
    pub critic_rows: Vec<CriticResidualRow>,
}
/// Build GAE once over a complete organism trajectory, including failures and
/// elapsed sleep gaps, then slice bounded tapes with detached burn-in. The final
/// actual decision is a bootstrap observation and has no actor loss here.
#[allow(clippy::too_many_arguments)]
pub fn cpu_training_windows(
    decisions: &[CpuCollectedDecision],
    terminal_record: Option<&alife_world::WorldOrganismRecord>,
    founder: &CpuFoundationTrainer,
    value: &CpuValueHead,
    phenotype: &CreaturePhenotype,
    version: u64,
    config: PpoConfig,
    window_ticks: usize,
    burn_in: usize,
) -> Result<Vec<CpuResidualTrainingWindow>, TrainingError> {
    config.validate()?;
    if let Some(record) = terminal_record {
        record.validate_contract()?;
        if record.lifecycle().is_alive()
            || record.organism_id() != decisions.first().ok_or_else(super::invalid)?.organism_id
            || record.phenotype() != phenotype
        {
            return Err(super::invalid());
        }
    }
    let terminal = terminal_record.map(|r| *r.biochemistry());
    if decisions.is_empty()
        || (terminal.is_none() && decisions.len() < 2)
        || window_ticks == 0
        || window_ticks + burn_in > crate::MAX_TRAINING_SEQUENCE_TICKS
    {
        return Err(super::invalid());
    }
    let organism = decisions[0].organism_id;
    let mut features = decisions
        .iter()
        .map(|d| d.final_features.clone())
        .collect::<Vec<_>>();
    if terminal.is_some() {
        features.push(features.last().ok_or_else(super::invalid)?.clone());
    }
    let values = features
        .iter()
        .map(|f| value.predict(f))
        .collect::<Result<Vec<_>, _>>()?;
    let parameters = phenotype.brain_genome.plasticity_parameters();
    let profile = parameters
        .action_candidate_credit_profile()
        .map_or_else(|| parameters.receptor_profile(), |p| p.receptor_profile());
    let mut transitions = Vec::new();
    let mut residual = Vec::new();
    let train_len = decisions.len() - usize::from(terminal.is_none());
    for i in 0..train_len {
        let current = &decisions[i];
        let next = decisions.get(i + 1);
        let is_terminal = next.is_none();
        if current.organism_id != organism
            || next.is_some_and(|n| {
                n.organism_id != organism
                    || current.patch.header().sequence_id.raw().checked_add(1)
                        != Some(n.patch.header().sequence_id.raw())
            })
            || current.patch.decision().policy_backend()
                != alife_core::PolicyBackend::NeuralClosedLoopCpu
        {
            return Err(super::invalid());
        }
        let end_tick = next
            .map_or_else(
                || terminal.as_ref().map(|t| t.tick),
                |n| Some(n.patch.pre_action().tick),
            )
            .ok_or_else(super::invalid)?;
        let dt = end_tick
            .raw()
            .checked_sub(current.patch.pre_action().tick.raw())
            .filter(|t| *t > 0)
            .ok_or_else(super::invalid)? as f32
            / 20.;
        let mut reward = current.biological_reward;
        let after = &current
            .patch
            .outcome()
            .measured_physiology
            .as_ref()
            .ok_or_else(super::invalid)?
            .after;
        let before = match next {
            Some(n) => {
                &n.patch
                    .outcome()
                    .measured_physiology
                    .as_ref()
                    .ok_or_else(super::invalid)?
                    .before
            }
            None => terminal.as_ref().ok_or_else(super::invalid)?,
        };
        if before.tick.raw() > after.tick.raw() {
            let gap = MeasuredPhysiologyTransition::new(*after, *before)?;
            let receptors = gap.before.neural_receptor_frame(phenotype)?;
            let sample = NeuromodulatorSample::from_components(
                0.,
                gap.aversive_value(),
                gap.homeostatic_improvement(),
                0.,
                0.,
            )?
            .with_biochemical_receptors(&receptors)?;
            let elapsed = (before.tick.raw() - after.tick.raw()) as f32 / 20.;
            reward = (reward
                + profile.project(&sample.frame())?
                    * (-std::f32::consts::LN_2 * elapsed / config.discount_half_life_seconds)
                        .exp())
            .clamp(-1., 1.);
        }
        let discount = if is_terminal {
            0.
        } else {
            (-std::f32::consts::LN_2 * dt / config.discount_half_life_seconds).exp()
        };
        transitions.push(PpoTransition {
            policy_version: version,
            trajectory_id: organism.raw(),
            step: i as u64,
            action: current.action.clone(),
            reward,
            old_value: values[i],
            next_value: if is_terminal { 0. } else { values[i + 1] },
            elapsed_seconds: dt,
            boundary: if is_terminal {
                PpoBoundary::Terminated
            } else if i + 1 == train_len {
                PpoBoundary::Truncated
            } else {
                PpoBoundary::Continuing
            },
        });
        residual.push(CriticResidualRow {
            current: i as u32,
            next: i as u32 + 1,
            reward,
            discount,
        });
    }
    let batch = PpoBatch::from_rollout(version, transitions, config)?;
    let mut windows = Vec::new();
    for start in (0..batch.len()).step_by(window_ticks) {
        let end = (start + window_ticks).min(batch.len());
        let prefix = start.saturating_sub(burn_in);
        let sequence = TrainingSequence {
            phenotype_hash: founder.phenotype().phenotype_hash(),
            initial: decisions[prefix].initial.clone(),
            ticks: decisions[prefix..end]
                .iter()
                .map(|d| d.replay_tick.clone())
                .collect(),
            burn_in_ticks: start - prefix,
            memory_candidate_gain: founder
                .phenotype()
                .candidate_decoder()
                .memory_channel()
                .map_or(0., |m| m.max_candidate_gain()),
        };
        let rows = residual[start..end]
            .iter()
            .enumerate()
            .map(|(i, r)| CriticResidualRow {
                current: i as u32,
                next: i as u32 + 1,
                reward: r.reward,
                discount: r.discount,
            })
            .collect();
        windows.push(CpuResidualTrainingWindow {
            actor: PpoTrainingWindow {
                speech_targets: vec![None; sequence.ticks.len()],
                sequence,
                batch: batch.window(start..end)?,
                auxiliary: None,
            },
            critic_features: features[start..=end].to_vec(),
            critic_rows: rows,
        });
    }
    Ok(windows)
}
/// One actor update and one full residual critic Adam update per effective batch.
/// Every window's frozen probabilities and actual features are checked before
/// the first mutation. Tape memory is bounded by one window.
#[allow(clippy::too_many_arguments)]
pub fn train_cpu_residual_cohort<F>(
    trainer: &mut CpuFoundationTrainer,
    value: &mut CpuValueHead,
    window_count: usize,
    effective_batch_size: usize,
    mut load: F,
    config: PpoConfig,
    version: u64,
) -> Result<PpoUpdateReceipt, TrainingError>
where
    F: FnMut(usize) -> Result<CpuResidualTrainingWindow, TrainingError>,
{
    config.validate()?;
    if window_count == 0
        || effective_batch_size == 0
        || config.value_coefficient <= 0.
        || value.checkpoint().feature_count != trainer.phenotype().neuron_count()
        || value
            .checkpoint()
            .last_updated_policy_version
            .is_some_and(|v| version <= v)
    {
        return Err(super::invalid());
    }
    let mut counts = Vec::new();
    for i in 0..window_count {
        let w = load(i)?;
        validate_frozen_residual_window(trainer, value, &w, config, version)?;
        counts.push(w.actor.batch.len());
    }
    let mut receipt = PpoUpdateReceipt {
        collection_policy_version: version,
        completed_epochs: 0,
        stopped_for_kl: false,
        mean_kl_before_updates: Vec::new(),
        actor_optimizer_step: trainer.optimizer_step(),
        value_optimizer_step: value.checkpoint().optimizer_step,
    };
    for _ in 0..config.epochs {
        for start in (0..window_count).step_by(effective_batch_size) {
            let end = (start + effective_batch_size).min(window_count);
            let total = counts[start..end].iter().sum::<usize>();
            let mut gradient = vec![0.; trainer.weights().len()];
            let mut vg = vec![0.; value.checkpoint().feature_count as usize + 1];
            let mut kl = 0.;
            for (i, &count) in counts.iter().enumerate().take(end).skip(start) {
                let w = load(i)?;
                if w.actor.batch.len() != count {
                    return Err(super::invalid());
                }
                let (adjoints, objectives, _replayed, tape) =
                    super::objective::evaluate_window(trainer, &w.actor, config, version)?;
                validate_residual_window(&w, None, config)?;
                let probe = trainer.replay_gradients_from_tape(
                    &w.actor.sequence,
                    &adjoints,
                    &w.actor.speech_targets,
                    &tape,
                )?;
                let scale = count as f32 / total as f32;
                for (g, part) in gradient.iter_mut().zip(probe.gradients) {
                    *g += part * scale;
                }
                let (_, parts) = value.residual_gradients(
                    &w.critic_features,
                    &w.critic_rows,
                    config.value_coefficient,
                )?;
                for (g, part) in vg.iter_mut().zip(parts) {
                    *g += part * scale;
                }
                for o in objectives {
                    kl += o.approximate_kl / total as f32;
                }
            }
            super::finite(&gradient)?;
            super::finite(&vg)?;
            super::finite(&[kl])?;
            receipt.mean_kl_before_updates.push(kl);
            if kl > config.target_kl {
                receipt.stopped_for_kl = true;
                return Ok(receipt);
            }
            let next = value.proposal(&vg, config.value_optimizer, version)?;
            trainer.apply_gradients(&gradient)?;
            *value = next;
            receipt.actor_optimizer_step = trainer.optimizer_step();
            receipt.value_optimizer_step = value.checkpoint().optimizer_step;
        }
        receipt.completed_epochs += 1;
    }
    Ok(receipt)
}
pub(super) fn validate_frozen_residual_window(
    trainer: &CpuFoundationTrainer,
    value: &CpuValueHead,
    window: &CpuResidualTrainingWindow,
    config: PpoConfig,
    version: u64,
) -> Result<(), TrainingError> {
    let (_, objectives, replayed, _) =
        super::objective::evaluate_window(trainer, &window.actor, config, version)?;
    validate_residual_window(window, Some(&replayed), config)?;
    value.residual_gradients(
        &window.critic_features,
        &window.critic_rows,
        config.value_coefficient,
    )?;
    if objectives
        .iter()
        .zip(window.actor.batch.transitions())
        .any(|(o, t)| (o.joint_log_probability - t.action.old_joint_log_probability).abs() > 1e-3)
    {
        return Err(super::invalid());
    }
    Ok(())
}
fn validate_residual_window(
    w: &CpuResidualTrainingWindow,
    replayed: Option<&[Vec<f32>]>,
    config: PpoConfig,
) -> Result<(), TrainingError> {
    if w.critic_rows.len() != w.actor.batch.len()
        || w.critic_features.len() != w.actor.batch.len() + 1
    {
        return Err(super::invalid());
    }
    for (actual, replay) in w.critic_features.iter().zip(replayed.unwrap_or(&[])) {
        if actual.len() != replay.len()
            || actual.iter().zip(replay).any(|(a, b)| {
                !a.is_finite() || (*a - *b).abs() > 2e-5 + 2e-4 * a.abs().max(b.abs())
            })
        {
            return Err(super::invalid());
        }
    }
    for (i, (row, t)) in w
        .critic_rows
        .iter()
        .zip(w.actor.batch.transitions())
        .enumerate()
    {
        let discount = if t.boundary == PpoBoundary::Terminated {
            0.
        } else {
            (-std::f32::consts::LN_2 * t.elapsed_seconds / config.discount_half_life_seconds).exp()
        };
        // Discount is validated for finiteness/range by residual_gradients;
        // row adjacency and captured rewards are sealed to the actor transition.
        if row.current != i as u32
            || row.next != i as u32 + 1
            || row.reward.to_bits() != t.reward.to_bits()
            || (row.discount - discount).abs() > 1e-6
        {
            return Err(super::invalid());
        }
    }
    Ok(())
}

#[cfg(test)]
mod equivalence_tests {
    use super::*;
    use crate::cpu::hotpath_bench;

    #[test]
    fn residual_tape_reuse_matches_recomputed_forward_for_unequal_windows() {
        let (phenotype, source) = hotpath_bench::fixture();
        let mut actual_actor = hotpath_bench::learner(&phenotype, &source);
        let mut expected_actor = hotpath_bench::learner(&phenotype, &source);
        let mut actual_value = hotpath_bench::value(&phenotype);
        let mut expected_value = hotpath_bench::value(&phenotype);
        let config = hotpath_bench::configuration();
        let windows = [
            hotpath_bench::window(&actual_actor, 3, 0),
            hotpath_bench::window(&actual_actor, 7, 2),
        ];
        let counts = windows.each_ref().map(|w| w.actor.batch.len());
        assert_eq!(counts, [3, 5]);
        let total = counts.iter().sum::<usize>();
        let mut expected_kl = Vec::new();
        // Reference deliberately discards loss evaluation's tape and uses the
        // public standalone probe, which computes forward again. Keep the
        // established residual derivative and full Adam proposal in both paths.
        for _ in 0..config.epochs {
            let mut actor_gradient = vec![0.; expected_actor.weights().len()];
            let mut value_gradient = vec![0.; phenotype.neuron_count() as usize + 1];
            let mut kl = 0.;
            for (window, count) in windows.iter().zip(counts) {
                let (adjoints, objectives, _, _) = super::super::objective::evaluate_window(
                    &expected_actor,
                    &window.actor,
                    config,
                    7400,
                )
                .unwrap();
                let probe = expected_actor
                    .probe_replay_gradients_with_speech(
                        &window.actor.sequence,
                        &adjoints,
                        &window.actor.speech_targets,
                    )
                    .unwrap();
                let scale = count as f32 / total as f32;
                for (g, part) in actor_gradient.iter_mut().zip(probe.gradients) {
                    *g += part * scale;
                }
                let (_, parts) = expected_value
                    .residual_gradients(
                        &window.critic_features,
                        &window.critic_rows,
                        config.value_coefficient,
                    )
                    .unwrap();
                for (g, part) in value_gradient.iter_mut().zip(parts) {
                    *g += part * scale;
                }
                for objective in objectives {
                    kl += objective.approximate_kl / total as f32;
                }
            }
            assert!(kl <= config.target_kl);
            expected_kl.push(kl);
            let next_value = expected_value
                .proposal(&value_gradient, config.value_optimizer, 7400)
                .unwrap();
            expected_actor.apply_gradients(&actor_gradient).unwrap();
            expected_value = next_value;
        }
        let receipt = train_cpu_residual_cohort(
            &mut actual_actor,
            &mut actual_value,
            windows.len(),
            2,
            |i| Ok(windows[i].clone()),
            config,
            7400,
        )
        .unwrap();
        assert_eq!(receipt.completed_epochs, config.epochs);
        assert!(!receipt.stopped_for_kl);
        assert_eq!(receipt.mean_kl_before_updates, expected_kl);
        assert_eq!(
            receipt.actor_optimizer_step,
            expected_actor.optimizer_step()
        );
        assert_eq!(
            receipt.value_optimizer_step,
            expected_value.checkpoint().optimizer_step
        );
        assert_eq!(
            serde_json::to_vec(&actual_actor.checkpoint()).unwrap(),
            serde_json::to_vec(&expected_actor.checkpoint()).unwrap()
        );
        assert_eq!(
            serde_json::to_vec(&actual_value.checkpoint()).unwrap(),
            serde_json::to_vec(&expected_value.checkpoint()).unwrap()
        );
    }
}
