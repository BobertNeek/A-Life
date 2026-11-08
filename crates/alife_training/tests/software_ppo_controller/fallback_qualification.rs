//! Valid supervised objectives force the experimental cache to the ordinary
//! controller. Disposable software Vulkan fixtures, not creature evidence.
use super::*;
use alife_training::{ImitationTarget, ReplaySpeechTarget};

#[derive(Clone, Copy, Debug)]
enum Unsupported {
    Speech,
    Auxiliary,
}

pub(super) fn assert_actor_progress(
    before: &FoundationTrainerCheckpoint,
    after: &FoundationTrainerCheckpoint,
    updates: u32,
) {
    assert_eq!(after.optimizer_step, before.optimizer_step + updates);
    assert_eq!(before.stage_mask, after.stage_mask);
    let mut changed = 0;
    let mut frozen = 0;
    for index in 0..before.weights.len() {
        assert!(after.weights[index].is_finite());
        assert!(after.first_moment[index].is_finite());
        assert!(after.second_moment[index].is_finite());
        if before.stage_mask.is_trainable(index) {
            assert_eq!(
                after.update_ages[index],
                before.update_ages[index] + updates
            );
            changed +=
                usize::from(before.weights[index].to_bits() != after.weights[index].to_bits());
        } else {
            frozen += 1;
            assert_eq!(
                before.weights[index].to_bits(),
                after.weights[index].to_bits()
            );
            assert_eq!(
                before.first_moment[index].to_bits(),
                after.first_moment[index].to_bits()
            );
            assert_eq!(
                before.second_moment[index].to_bits(),
                after.second_moment[index].to_bits()
            );
            assert_eq!(before.update_ages[index], after.update_ages[index]);
        }
    }
    assert!(changed > 0 && frozen > 0);
}

fn restore(
    trainer: &mut FoundationTrainer,
    state: &mut PpoTrainingState,
    actor: &FoundationTrainerCheckpoint,
    value: &PpoValueHeadCheckpoint,
) {
    trainer.restore_checkpoint(actor).unwrap();
    *state = PpoTrainingState::from_checkpoint(value.clone()).unwrap();
}

fn assert_accepted(
    receipt: &PpoUpdateReceipt,
    loads: &[usize],
    before_actor: &FoundationTrainerCheckpoint,
    before_value: &PpoValueHeadCheckpoint,
    trainer: &FoundationTrainer,
    state: &mut PpoTrainingState,
) {
    assert_eq!(loads, [0, 1, 0, 1, 0, 1]);
    assert_eq!(receipt.completed_epochs, 2);
    assert!(!receipt.stopped_for_kl);
    assert_eq!(receipt.mean_kl_before_updates.len(), 2);
    assert_eq!(
        receipt.actor_optimizer_step,
        before_actor.optimizer_step + 2
    );
    assert_eq!(
        receipt.value_optimizer_step,
        before_value.optimizer_step + 2
    );
    assert_actor_progress(before_actor, &trainer.checkpoint().unwrap(), 2);
    let after_value = state.checkpoint(trainer.session()).unwrap();
    assert_eq!(after_value.optimizer_step, before_value.optimizer_step + 2);
    assert_eq!(after_value.last_updated_policy_version, Some(6));
    let width = before_value.feature_count as usize + 1;
    for plane in 0..3 {
        let range = plane * width..(plane + 1) * width;
        assert_ne!(
            bits(&before_value.parameters[range.clone()]),
            bits(&after_value.parameters[range]),
            "value parameter/moment plane {plane} must advance"
        );
    }
}

fn captured_objectives(
    trainer: &FoundationTrainer,
    state: &mut PpoTrainingState,
    kind: Unsupported,
) -> BTreeMap<String, Vec<u32>> {
    let mut words = capture(trainer, state);
    if matches!(kind, Unsupported::Auxiliary) {
        let objective = state.objective_mut().unwrap();
        let metrics = read_words(
            trainer,
            objective.output_buffer(),
            objective.imitation_metric_bytes(),
        );
        assert!(!metrics.is_empty());
        for row in metrics.as_chunks::<4>().0 {
            assert!(row.iter().all(|word| f32::from_bits(*word).is_finite()));
            assert!(f32::from_bits(row[1]) > 0.0);
            assert_eq!(row[2], 0.2_f32.to_bits());
            assert_eq!(row[3], 1.0_f32.to_bits());
        }
        words.insert("actual_auxiliary_imitation_metrics".into(), metrics);
    }
    words
}

fn qualify(kind: Unsupported) {
    let (mut trainer, _, neuron, lane) = fixture_with_wide_mask(true);
    let mut state = PpoTrainingState::default();
    let config = PpoConfig {
        epochs: 2,
        target_kl: 100.0,
        entropy_coefficient: 0.0,
        value_coefficient: 0.5,
        ..Default::default()
    };
    let warm_config = PpoConfig {
        epochs: 1,
        ..config
    };
    let initial = trainer.checkpoint().unwrap();
    assert_eq!(initial.stage_mask.trainable_count(), 10_923);
    let warm = windows_with_count(
        &mut trainer,
        &mut state,
        neuron,
        lane,
        warm_config,
        5,
        0.0,
        2,
    );
    let (warm_result, warm_stats, warm_loads) =
        run_owned(&mut trainer, &mut state, &warm, warm_config, 5, false);
    assert_eq!(warm_result.unwrap().completed_epochs, 1);
    assert_eq!(warm_loads, [0, 1, 0, 1]);
    assert_eq!(warm_stats, FrozenGradientCacheStats::default());
    let before_actor = trainer.checkpoint().unwrap();
    assert_actor_progress(&initial, &before_actor, 1);
    assert!(before_actor
        .first_moment
        .iter()
        .any(|moment| *moment != 0.0));
    let before_value = state.checkpoint(trainer.session()).unwrap();
    assert_eq!(before_value.optimizer_step, 1);
    let width = before_value.feature_count as usize + 1;
    assert!(before_value.parameters[..width].iter().any(|v| *v != 0.0));
    assert!(before_value.parameters[width..2 * width]
        .iter()
        .any(|v| *v != 0.0));
    assert!(before_value.parameters[2 * width..]
        .iter()
        .any(|v| *v > 0.0));
    let ordinary = windows_with_count(&mut trainer, &mut state, neuron, lane, config, 6, 0.0, 2);
    let mut labeled = ordinary.clone();
    let late = &mut labeled[1];
    match kind {
        Unsupported::Speech => {
            let positive = ReplaySpeechTarget {
                token: Some(8),
                continuation: [1, 0, 0, 0, 0],
                act: alife_core::SpeechActKind::Declare,
                weight: 0.5,
            };
            let silence = ReplaySpeechTarget {
                token: None,
                continuation: [0; 5],
                weight: 0.2,
                ..positive
            };
            positive.validate().unwrap();
            silence.validate().unwrap();
            assert_eq!(positive.len(), 2);
            let loss_start = late.sequence.burn_in_ticks;
            late.speech_targets[loss_start] = Some(positive);
            late.speech_targets[loss_start + 1] = Some(silence);
        }
        Unsupported::Auxiliary => {
            // The actual PPO representative is Idle (candidate2). Supervision
            // instead labels the legal Approach candidate0, with positive mass.
            assert!(late
                .batch
                .transitions()
                .iter()
                .all(|t| t.action.representative == 2));
            late.auxiliary = Some((
                vec![
                    ImitationTarget {
                        representative: Some(1),
                        motors: [None; 6],
                    };
                    late.batch.len()
                ],
                0.2,
            ));
        }
    }
    // A normal no-label control distinguishes actual supervised gradient effects
    // from ordinary PPO, weight decay, and the already-warmed optimizer history.
    restore(&mut trainer, &mut state, &before_actor, &before_value);
    let (control, stats, loads) = run_owned(&mut trainer, &mut state, &ordinary, config, 6, false);
    let control = control.unwrap();
    assert_eq!(stats, FrozenGradientCacheStats::default());
    assert_accepted(
        &control,
        &loads,
        &before_actor,
        &before_value,
        &trainer,
        &mut state,
    );
    let control_actor = trainer.checkpoint().unwrap();

    restore(&mut trainer, &mut state, &before_actor, &before_value);
    let (normal, stats, normal_loads) =
        run_owned(&mut trainer, &mut state, &labeled, config, 6, false);
    let normal = normal.unwrap();
    assert_eq!(stats, FrozenGradientCacheStats::default());
    assert_accepted(
        &normal,
        &normal_loads,
        &before_actor,
        &before_value,
        &trainer,
        &mut state,
    );
    let expected_actor = trainer.checkpoint().unwrap();
    let expected_value = state.checkpoint(trainer.session()).unwrap();
    let causal = trainer.phenotype().synapses().iter().enumerate().any(|(index, synapse)| {
        before_actor.stage_mask.is_trainable(index)
            && (!matches!(kind, Unsupported::Speech)
                || matches!(synapse.kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::SpeechPayload))
            && (expected_actor.first_moment[index].to_bits() != control_actor.first_moment[index].to_bits()
                || expected_actor.second_moment[index].to_bits() != control_actor.second_moment[index].to_bits())
    });
    assert!(
        causal,
        "{kind:?} must affect actual trainable optimizer moments beyond the no-label control"
    );
    let expected_words = captured_objectives(&trainer, &mut state, kind);

    restore(&mut trainer, &mut state, &before_actor, &before_value);
    let (fallback, stats, fallback_loads) =
        run_owned(&mut trainer, &mut state, &labeled, config, 6, true);
    let fallback = fallback.unwrap();
    assert_accepted(
        &fallback,
        &fallback_loads,
        &before_actor,
        &before_value,
        &trainer,
        &mut state,
    );
    assert!(!stats.eligible);
    assert_eq!(stats.requested_gpu_bytes, 278_544);
    assert_eq!(stats.raw_cached_windows, 1);
    assert_eq!(stats.reused_windows, 0);
    assert_eq!(normal_loads, fallback_loads);
    assert_eq!(receipt_words(&normal), receipt_words(&fallback));
    assert_actor_equal(&expected_actor, &trainer.checkpoint().unwrap());
    assert_value_equal(
        &expected_value,
        &state.checkpoint(trainer.session()).unwrap(),
    );
    assert_semantics_equal(
        &expected_words,
        &captured_objectives(&trainer, &mut state, kind),
    );
    eprintln!("actual {kind:?} fallback: valid late labels discard one raw cached window, reuse0; two accepted actor/value updates, causal supervised moments, frozen bits/ages and every captured word match original controller");
}

#[test]
#[ignore = "explicit synthetic software Vulkan actual speech fallback and causal control"]
fn actual_speech_labels_force_late_frozen_cache_fallback() {
    qualify(Unsupported::Speech);
}

#[test]
#[ignore = "explicit synthetic software Vulkan actual auxiliary fallback and causal control"]
fn actual_auxiliary_targets_force_late_frozen_cache_fallback() {
    qualify(Unsupported::Auxiliary);
}
