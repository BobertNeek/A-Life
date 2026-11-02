//! Coherent synthetic recurrent replay with full-trajectory GAE before slicing.
use super::*;

fn continuing_windows(
    trainer: &mut FoundationTrainer,
    state: &mut PpoTrainingState,
    neuron: usize,
    lane: usize,
    config: PpoConfig,
) -> Vec<PpoTrainingWindow> {
    // Eight actual GPU rows provide seven loss rows and an observed final
    // bootstrap state. The last observed row is not itself trained here.
    let mut collected = windows_with_shapes(
        trainer,
        state,
        neuron,
        lane,
        config,
        6,
        0.0,
        &[(8, 0)],
        true,
    );
    let whole = collected.pop().unwrap();
    let evaluation = trainer.evaluate_replay(&whole.sequence).unwrap();
    let mut transitions = whole.batch.transitions()[..7].to_vec();
    let elapsed = [0.04, 0.08, 0.05, 0.06, 0.03, 0.09, 0.07];
    for (index, row) in transitions.iter_mut().enumerate() {
        row.trajectory_id = 42;
        row.step = 100 + index as u64;
        row.elapsed_seconds = elapsed[index];
        row.reward = if index % 2 == 0 { 0.6 } else { -0.15 } + index as f32 * 0.04;
        row.next_value = whole.batch.transitions()[index + 1].old_value;
        row.boundary = if index == 6 {
            PpoBoundary::Truncated
        } else {
            PpoBoundary::Continuing
        };
    }
    assert_ne!(transitions[6].next_value, 0.0);
    let batch = PpoBatch::from_rollout(6, transitions.clone(), config).unwrap();
    // A replay window edge must not truncate the scalar trajectory's GAE tail.
    let mut cut = transitions[..3].to_vec();
    assert!(PpoBatch::from_rollout(6, cut.clone(), config).is_err());
    cut[2].boundary = PpoBoundary::Truncated;
    let cut_batch = PpoBatch::from_rollout(6, cut, config).unwrap();
    assert_ne!(bits(&batch.advantages()[..3]), bits(cut_batch.advantages()));
    assert_ne!(bits(&batch.returns()[..3]), bits(cut_batch.returns()));
    // The actual eighth GPU value must also contribute to final bootstrap.
    let mut zero_bootstrap = transitions;
    zero_bootstrap[6].next_value = 0.0;
    let zero_batch = PpoBatch::from_rollout(6, zero_bootstrap, config).unwrap();
    assert_ne!(bits(batch.returns()), bits(zero_batch.returns()));

    let mut first_sequence = whole.sequence.clone();
    first_sequence.ticks.truncate(3);
    let mut second_sequence = whole.sequence.clone();
    second_sequence.ticks = whole.sequence.ticks[1..7].to_vec();
    second_sequence.burn_in_ticks = 2;
    second_sequence.initial.activations = evaluation.final_activations[0].clone();
    second_sequence.initial.activity_ema = evaluation.final_activity_ema[0].clone();
    second_sequence.initial.metabolic_load = evaluation.final_metabolic_load[0].clone();
    // This fixture has no structural dendrites; no private branch state is omitted.
    assert!(second_sequence.initial.dendrites.branches().is_empty());
    let sliced = trainer.evaluate_replay(&second_sequence).unwrap();
    let sliced_values = state.predict_values(trainer, &second_sequence).unwrap();
    for (index, logits) in sliced.candidate_logits[2..].iter().enumerate() {
        assert_eq!(bits(logits), bits(&evaluation.candidate_logits[index + 3]));
        assert_eq!(
            sliced_values[index + 2].to_bits(),
            whole.batch.transitions()[index + 3].old_value.to_bits()
        );
    }
    let first_batch = batch.window(0..3).unwrap();
    let second_batch = batch.window(3..7).unwrap();
    assert_eq!(
        first_batch.transitions()[2].boundary,
        PpoBoundary::Continuing
    );
    assert_eq!(
        second_batch.transitions()[3].boundary,
        PpoBoundary::Truncated
    );
    assert_eq!(
        bits(first_batch.advantages()),
        bits(&batch.advantages()[..3])
    );
    assert_eq!(
        bits(second_batch.advantages()),
        bits(&batch.advantages()[3..])
    );
    vec![
        PpoTrainingWindow {
            sequence: first_sequence,
            batch: first_batch,
            speech_targets: vec![None; 3],
            auxiliary: None,
        },
        PpoTrainingWindow {
            sequence: second_sequence,
            batch: second_batch,
            speech_targets: vec![None; 6],
            auxiliary: None,
        },
    ]
}

#[test]
#[ignore = "explicit synthetic software qualification: coherent continuing GAE and raw-cache exactness"]
fn continuing_gae_matches_actual_controller_after_warm_restore() {
    let (mut trainer, weight, neuron, lane) = fixture();
    let mut state = PpoTrainingState::default();
    let config = PpoConfig {
        epochs: 2,
        target_kl: 100.0,
        entropy_coefficient: 0.0,
        value_coefficient: 0.5,
        ..Default::default()
    };
    let (before_actor, before_value) =
        warm_pair(&mut trainer, &mut state, weight, neuron, lane, config);
    let windows = continuing_windows(&mut trainer, &mut state, neuron, lane, config);
    assert_persistent_equal(&before_actor, &before_value, &trainer, &mut state);
    let expected_loads = [0, 1, 0, 1, 0, 1];
    let (normal, _, loads) = run_owned(&mut trainer, &mut state, &windows, config, 6, false);
    let normal = normal.unwrap();
    assert_eq!(loads, expected_loads);
    assert_eq!(normal.completed_epochs, 2);
    assert!(!normal.stopped_for_kl);
    let after_actor = trainer.checkpoint().unwrap();
    let after_value = state.checkpoint(trainer.session()).unwrap();
    progress_steps(&before_actor, &after_actor, weight, 2);
    assert_eq!(after_value.optimizer_step, before_value.optimizer_step + 2);
    let width = before_value.feature_count as usize + 1;
    for plane in 0..3 {
        assert_ne!(
            bits(&before_value.parameters[plane * width..(plane + 1) * width]),
            bits(&after_value.parameters[plane * width..(plane + 1) * width]),
            "value parameter/moment plane {plane} must advance",
        );
    }
    let expected = capture(&trainer, &mut state);
    for shared in [false, true] {
        trainer.restore_checkpoint(&before_actor).unwrap();
        state = PpoTrainingState::from_checkpoint(before_value.clone()).unwrap();
        let mut stats = FrozenGradientCacheStats::default();
        let mut loads = Vec::new();
        let receipt = if shared {
            let handles = windows.iter().cloned().map(Arc::new).collect::<Vec<_>>();
            train_recurrent_ppo_cohort_with_frozen_gradients_bounded_for_test(
                &mut trainer,
                &mut state,
                2,
                8,
                |index| {
                    loads.push(index);
                    Ok(Arc::clone(&handles[index]))
                },
                config,
                6,
                replay_tick_bound(&handles),
                &mut stats,
            )
        } else {
            train_recurrent_ppo_cohort_with_frozen_gradients_bounded_for_test(
                &mut trainer,
                &mut state,
                2,
                8,
                |index| {
                    loads.push(index);
                    Ok(windows[index].clone())
                },
                config,
                6,
                replay_tick_bound(&windows),
                &mut stats,
            )
        }
        .unwrap();
        assert_active_cache(stats);
        assert_eq!(loads, expected_loads);
        assert_eq!(receipt_words(&normal), receipt_words(&receipt));
        assert_actor_equal(&after_actor, &trainer.checkpoint().unwrap());
        assert_value_equal(&after_value, &state.checkpoint(trainer.session()).unwrap());
        assert_semantics_equal(&expected, &capture(&trainer, &mut state));
    }
    eprintln!("continuing GAE qualification passed: coherent8GPUrows,7trajectorylosses,actualnonzero finalbootstrap,globalGAE→3/4loss windows with2burn,epochs2/value.5,nonzero warmhistory,owned/shared bit-exact actualcontroller+freshlaterupdate");
}

fn changed_weight_and_moments(
    before: &FoundationTrainerCheckpoint,
    after: &FoundationTrainerCheckpoint,
) -> Vec<usize> {
    // Enumerated by the caller's actual compiled phenotype, not assumptions
    // about the canonical synapse index ranges.
    (0..before.weights.len())
        .filter(|&index| {
            before.stage_mask.is_trainable(index)
                && before.weights[index].to_bits() != after.weights[index].to_bits()
                && before.first_moment[index].to_bits() != after.first_moment[index].to_bits()
                && before.second_moment[index].to_bits() != after.second_moment[index].to_bits()
        })
        .collect()
}

#[test]
#[ignore = "explicit synthetic software qualification: continuing GAE with recurrent parameter changes"]
fn wide_continuing_gae_keeps_later_epochs_fresh_after_non_decoder_updates() {
    let (mut trainer, _, neuron, lane) = fixture_with_wide_mask(true);
    let mut state = PpoTrainingState::default();
    let config = PpoConfig {
        epochs: 2,
        target_kl: 100.0,
        entropy_coefficient: 0.0,
        value_coefficient: 0.5,
        ..Default::default()
    };
    let initial = trainer.checkpoint().unwrap();
    let warm_config = PpoConfig {
        epochs: 1,
        ..config
    };
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
    let (warm_receipt, _, _) = run_owned(&mut trainer, &mut state, &warm, warm_config, 5, false);
    assert_eq!(warm_receipt.unwrap().completed_epochs, 1);
    let before_actor = trainer.checkpoint().unwrap();
    super::fallback_qualification::assert_actor_progress(&initial, &before_actor, 1);
    assert_eq!(before_actor.stage_mask.trainable_count(), 10_923);
    let before_value = state.checkpoint(trainer.session()).unwrap();
    assert_eq!(before_value.optimizer_step, 1);
    let width = before_value.feature_count as usize + 1;
    for plane in 0..3 {
        assert!(before_value.parameters[plane * width..(plane + 1) * width]
            .iter()
            .any(|&v| v != 0.0));
    }
    let windows = continuing_windows(&mut trainer, &mut state, neuron, lane, config);
    assert_persistent_equal(&before_actor, &before_value, &trainer, &mut state);
    let non_decoder_indices = trainer
        .phenotype()
        .synapses()
        .iter()
        .enumerate()
        .filter(|(_, synapse)| !matches!(synapse.kind(), CompiledSynapseKind::Decoder(_)))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    // This separate one-epoch oracle establishes that the very first update
    // changes backbone parameters before the fresh second epoch is required.
    let (probe, _, loads) = run_owned(&mut trainer, &mut state, &windows, warm_config, 6, false);
    assert_eq!(probe.unwrap().completed_epochs, 1);
    assert_eq!(loads, [0, 1, 0, 1]);
    let probe_actor = trainer.checkpoint().unwrap();
    super::fallback_qualification::assert_actor_progress(&before_actor, &probe_actor, 1);
    let first_non_decoder = changed_weight_and_moments(&before_actor, &probe_actor)
        .into_iter()
        .filter(|index| non_decoder_indices.contains(index))
        .collect::<Vec<_>>();
    assert!(
        !first_non_decoder.is_empty(),
        "the first actual update must change a non-decoder weight and both moments"
    );
    trainer.restore_checkpoint(&before_actor).unwrap();
    state = PpoTrainingState::from_checkpoint(before_value.clone()).unwrap();
    let (normal, _, loads) = run_owned(&mut trainer, &mut state, &windows, config, 6, false);
    let normal = normal.unwrap();
    assert_eq!(normal.completed_epochs, 2);
    assert!(!normal.stopped_for_kl);
    assert_eq!(loads, [0, 1, 0, 1, 0, 1]);
    let after_actor = trainer.checkpoint().unwrap();
    let after_value = state.checkpoint(trainer.session()).unwrap();
    super::fallback_qualification::assert_actor_progress(&before_actor, &after_actor, 2);
    let final_non_decoder = changed_weight_and_moments(&before_actor, &after_actor)
        .into_iter()
        .filter(|index| non_decoder_indices.contains(index))
        .collect::<Vec<_>>();
    assert!(!final_non_decoder.is_empty());
    assert_eq!(after_value.optimizer_step, before_value.optimizer_step + 2);
    for plane in 0..3 {
        let range = plane * width..(plane + 1) * width;
        assert_ne!(
            bits(&before_value.parameters[range.clone()]),
            bits(&after_value.parameters[range])
        );
    }
    let expected = capture(&trainer, &mut state);
    trainer.restore_checkpoint(&before_actor).unwrap();
    state = PpoTrainingState::from_checkpoint(before_value).unwrap();
    let shared = windows.into_iter().map(Arc::new).collect::<Vec<_>>();
    let mut stats = FrozenGradientCacheStats::default();
    let mut loads = Vec::new();
    let receipt = train_recurrent_ppo_cohort_with_frozen_gradients_bounded_for_test(
        &mut trainer,
        &mut state,
        2,
        8,
        |index| {
            loads.push(index);
            Ok(Arc::clone(&shared[index]))
        },
        config,
        6,
        replay_tick_bound(&shared),
        &mut stats,
    )
    .unwrap();
    assert_active_cache(stats);
    assert_eq!(loads, [0, 1, 0, 1, 0, 1]);
    assert_eq!(receipt_words(&normal), receipt_words(&receipt));
    assert_actor_equal(&after_actor, &trainer.checkpoint().unwrap());
    assert_value_equal(&after_value, &state.checkpoint(trainer.session()).unwrap());
    assert_semantics_equal(&expected, &capture(&trainer, &mut state));
    eprintln!("wide continuing GAE passed:10923trainable, first-update nondecoder weight+m+v changed at{}coordinates; two-update nondecoder changed at{}; epochs2/value.5,reuse2,fresh second epoch,allmaskages/frozenbits and fullcapturedwords exact", first_non_decoder.len(), final_non_decoder.len());
}
