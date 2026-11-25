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
const PARALLEL_WINDOW_CHUNK: usize = 32;

fn validate_residual_cohort_inputs(
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
        || config.value_coefficient <= 0.
        || value.checkpoint().feature_count != trainer.phenotype().neuron_count()
        || value
            .checkpoint()
            .last_updated_policy_version
            .is_some_and(|v| version <= v)
    {
        return Err(super::invalid());
    }
    Ok(())
}

type ResidualWindowGradients = (Vec<f32>, Vec<f32>, Vec<f32>);

fn evaluate_and_backward_residual_window(
    trainer: &CpuFoundationTrainer,
    value: &CpuValueHead,
    w: &CpuResidualTrainingWindow,
    config: PpoConfig,
    version: u64,
) -> Result<ResidualWindowGradients, TrainingError> {
    let ((adjoints, objectives, _replayed), tape) =
        super::objective::evaluate_window_with_tape(trainer, &w.actor, config, version)?;
    validate_residual_window(w, None, config)?;
    let probe =
        trainer.backward_with_tape(&w.actor.sequence, &tape, &adjoints, &w.actor.speech_targets)?;
    drop(tape);
    let (_, critic_parts) =
        value.residual_gradients(&w.critic_features, &w.critic_rows, config.value_coefficient)?;
    let kls = objectives.into_iter().map(|o| o.approximate_kl).collect();
    Ok((probe.gradients, critic_parts, kls))
}

pub(super) fn train_cpu_residual_cohort_slice(
    trainer: &mut CpuFoundationTrainer,
    value: &mut CpuValueHead,
    windows: &[CpuResidualTrainingWindow],
    effective_batch_size: usize,
    config: PpoConfig,
    version: u64,
) -> Result<PpoUpdateReceipt, TrainingError> {
    train_cpu_residual_cohort_slice_with_threads(
        trainer,
        value,
        windows,
        effective_batch_size,
        config,
        version,
        usize::MAX,
    )
}

pub(super) fn train_cpu_residual_cohort_slice_with_threads(
    trainer: &mut CpuFoundationTrainer,
    value: &mut CpuValueHead,
    windows: &[CpuResidualTrainingWindow],
    effective_batch_size: usize,
    config: PpoConfig,
    version: u64,
    max_threads: usize,
) -> Result<PpoUpdateReceipt, TrainingError> {
    let window_count = windows.len();
    validate_residual_cohort_inputs(
        trainer,
        value,
        window_count,
        effective_batch_size,
        config,
        version,
    )?;
    let counts = super::parallel_map_indexed_with_limit(windows, max_threads, |_, w| {
        validate_frozen_residual_window(trainer, value, w, config, version)?;
        Ok(w.actor.batch.len())
    })?;
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
            for chunk_start in (start..end).step_by(PARALLEL_WINDOW_CHUNK) {
                let chunk_end = (chunk_start + PARALLEL_WINDOW_CHUNK).min(end);
                let chunk = &windows[chunk_start..chunk_end];
                let results =
                    super::parallel_map_indexed_with_limit(chunk, max_threads, |_, w| {
                        evaluate_and_backward_residual_window(trainer, value, w, config, version)
                    })?;
                for (w, (actor_parts, critic_parts, kls)) in chunk.iter().zip(results) {
                    let scale = w.actor.batch.len() as f32 / total as f32;
                    for (g, part) in gradient.iter_mut().zip(actor_parts) {
                        *g += part * scale;
                    }
                    for (g, part) in vg.iter_mut().zip(critic_parts) {
                        *g += part * scale;
                    }
                    for o_kl in kls {
                        kl += o_kl / total as f32;
                    }
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
    validate_residual_cohort_inputs(
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
            chunk.push(load(i)?);
        }
        let chunk_counts = super::parallel_map_indexed_with_limit(&chunk, usize::MAX, |_, w| {
            validate_frozen_residual_window(trainer, value, w, config, version)?;
            Ok(w.actor.batch.len())
        })?;
        counts.extend(chunk_counts);
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
            for chunk_start in (start..end).step_by(PARALLEL_WINDOW_CHUNK) {
                let chunk_end = (chunk_start + PARALLEL_WINDOW_CHUNK).min(end);
                let mut chunk = Vec::with_capacity(chunk_end - chunk_start);
                for (i, &count) in counts.iter().enumerate().take(chunk_end).skip(chunk_start) {
                    let w = load(i)?;
                    if w.actor.batch.len() != count {
                        return Err(super::invalid());
                    }
                    chunk.push(w);
                }
                let results =
                    super::parallel_map_indexed_with_limit(&chunk, usize::MAX, |_, w| {
                        evaluate_and_backward_residual_window(trainer, value, w, config, version)
                    })?;
                for (w, (actor_parts, critic_parts, kls)) in chunk.iter().zip(results) {
                    let scale = w.actor.batch.len() as f32 / total as f32;
                    for (g, part) in gradient.iter_mut().zip(actor_parts) {
                        *g += part * scale;
                    }
                    for (g, part) in vg.iter_mut().zip(critic_parts) {
                        *g += part * scale;
                    }
                    for o_kl in kls {
                        kl += o_kl / total as f32;
                    }
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
    let (_, objectives, replayed) =
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
mod determinism_tests {
    use super::*;
    use crate::{
        PpoBatch, PpoBoundary, PpoTrainingWindow, PpoTransition, PpoValueHeadCheckpoint,
        StageTrainableMask, TrainingInitialState, TrainingReplayCandidate, TrainingReplayTick,
        TrainingSequence,
    };
    use alife_core::{
        BrainCapacityClass, BrainGenome, CandidateActionFamily, DendriticBranch,
        DendriticBranchSet, DendriticInputRef, DevelopmentState, FoundationWeightAsset,
        NormalizedScalar, PhenotypeCompiler, SensorProfile, Tick,
    };

    #[test]
    fn parallel_residual_cohort_matches_single_threaded_bit_for_bit() {
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
                crate::AdamWConfig::default(),
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

        let windows: Vec<CpuResidualTrainingWindow> = (0..5)
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
                let transitions: Vec<PpoTransition> = (0..2)
                    .map(|i| {
                        let mut a = crate::PpoJointAction {
                            candidate_count: 3,
                            representative_mask: 7,
                            motor_masks: [0; 6],
                            representative: ((w_idx + i) % 3) as u16,
                            forced_slots: vec![None; 3],
                            motor_candidates: [None; 6],
                            old_joint_log_probability: -1.,
                            temperature: config.temperature,
                        };
                        a.old_joint_log_probability = super::objective::cpu_ppo_objective(
                            &eval.candidate_logits[i],
                            &a,
                            0.,
                            config,
                        )
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
                let critic_rows = transitions
                    .iter()
                    .enumerate()
                    .map(|(i, t)| CriticResidualRow {
                        current: i as u32,
                        next: i as u32 + 1,
                        reward: t.reward,
                        discount: if t.boundary == PpoBoundary::Terminated {
                            0.
                        } else {
                            (-std::f32::consts::LN_2 * t.elapsed_seconds
                                / config.discount_half_life_seconds)
                                .exp()
                        },
                    })
                    .collect();
                let mut critic_features = eval.final_activations.clone();
                critic_features.push(eval.final_activations.last().unwrap().clone());
                CpuResidualTrainingWindow {
                    actor: PpoTrainingWindow {
                        sequence: seq,
                        batch: PpoBatch::from_rollout(7400, transitions, config).unwrap(),
                        auxiliary: None,
                        speech_targets: vec![None; 2],
                    },
                    critic_features,
                    critic_rows,
                }
            })
            .collect();

        let receipt_seq = train_cpu_residual_cohort_slice_with_threads(
            &mut t_seq, &mut v_seq, &windows, 4, config, 7400, 1,
        )
        .unwrap();
        let receipt_par = train_cpu_residual_cohort_slice_with_threads(
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
