use super::*;

#[test]
fn residual_scaling_uses_train_only_and_initial_predictions_are_persistence() {
    let (trainer, mut data, _) = super::super::tests::fixture();
    let scale = fit_training_scales(&data, 0.001).unwrap();
    data.episodes[1].observations[1].fill(99.);
    assert_eq!(scale, fit_training_scales(&data, 0.001).unwrap());
    let neurons = trainer
        .phenotype()
        .synapses()
        .iter()
        .filter(|s| s.kind() == CompiledSynapseKind::Recurrent)
        .map(|s| s.target() as usize)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(4)
        .collect();
    let head = PredictiveHead::new_residual(
        trainer.phenotype(),
        data.schema.clone(),
        neurons,
        8,
        71,
        scale.scales,
        1,
    )
    .unwrap();
    let probe = head.probe(&trainer, &data, 0).unwrap();
    assert_eq!(
        probe.predictions,
        data.episodes[0].observations[..data.episodes[0].replay.ticks.len()]
    );
    assert!(probe.recurrent.gradients.iter().all(|&g| g == 0.));
}

#[test]
fn shuffling_never_crosses_splits_and_does_not_move_targets_or_replay() {
    let (_, data, _) = super::super::tests::fixture();
    let p = action_permutation(&data, 71);
    for (i, &j) in p.iter().enumerate() {
        assert_eq!(data.episodes[i].split, data.episodes[j].split);
        let changed = conditioned(&data, i, PredictiveInputs::ShuffledAction, &p);
        assert_eq!(changed.replay, data.episodes[i].replay);
        assert_eq!(changed.observations, data.episodes[i].observations);
    }
}

#[test]
fn residual_normalized_loss_reaches_native_recurrence_after_zero_output_warmup() {
    let (mut trainer, mut data, _) = super::super::tests::fixture();
    data.episodes[0].replay.burn_in_ticks = 0;
    let scaling = fit_training_scales(&data, 0.001).unwrap();
    let neurons = trainer
        .phenotype()
        .synapses()
        .iter()
        .filter(|s| s.kind() == CompiledSynapseKind::Recurrent)
        .map(|s| s.target() as usize)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(8)
        .collect();
    let mut head = PredictiveHead::new_residual(
        trainer.phenotype(),
        data.schema.clone(),
        neurons,
        8,
        71,
        scaling.scales,
        1,
    )
    .unwrap();
    head.train_step(&mut trainer, &data, 0, 0.03).unwrap();
    let probe = head.probe(&trainer, &data, 0).unwrap();
    let (id, &analytic) = probe
        .recurrent
        .gradients
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.abs().total_cmp(&b.abs()))
        .unwrap();
    assert!(analytic.abs() > 1e-7, "missing native residual gradient");
    let epsilon = 0.002;
    let mut loss = Vec::new();
    for direction in [-1., 1.] {
        let mut cp = trainer.checkpoint();
        cp.weights[id] += direction * epsilon;
        let perturbed = CpuFoundationTrainer::from_checkpoint(
            trainer.phenotype().clone(),
            trainer.source_foundation.clone(),
            cp,
        )
        .unwrap();
        loss.push(head.probe(&perturbed, &data, 0).unwrap().metrics.mse);
    }
    let numeric = ((loss[1] - loss[0]) / (2. * f64::from(epsilon))) as f32;
    assert!(
        (numeric - analytic).abs() < 1e-5 + 0.05 * analytic.abs(),
        "native residual derivative {analytic} != {numeric}"
    );
}

#[test]
fn multi_horizon_targets_are_real_future_observations_with_whole_world_splits() {
    let config = ResidualConfig {
        world_ticks: 6,
        epochs: 1,
        ..ResidualConfig::default()
    };
    let captures = capture_diverse(&config).unwrap();
    let data = horizon_dataset(&captures, 4).unwrap();
    assert_eq!(data.episodes.len(), 18);
    let scaling = fit_training_scales(&data, 0.001).unwrap();
    assert_eq!(scaling.train_pairs, 6);
    for e in &data.episodes {
        let evidence = e.evidence.as_ref().unwrap();
        assert_eq!(evidence.horizon_ticks, 4);
        assert_eq!(evidence.source_tick + 4, evidence.target_tick);
        let group = captures
            .iter()
            .flat_map(|r| &r.groups)
            .find(|g| g.collection_id == e.split_group)
            .unwrap();
        let actual = group
            .decisions
            .iter()
            .find(|d| d.patch.pre_action().sequence_id.raw() == evidence.target_sequence)
            .unwrap();
        assert_eq!(e.observations[1], observation(actual));
        assert_eq!(group.split, e.split);
    }
    let p = captures[0].source.clone().admit().unwrap().phenotype;
    let neurons = p
        .synapses()
        .iter()
        .filter(|s| s.kind() == CompiledSynapseKind::Recurrent)
        .map(|s| s.target() as usize)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(4)
        .collect();
    let wrong =
        PredictiveHead::new_residual(&p, data.schema.clone(), neurons, 8, 71, scaling.scales, 1)
            .unwrap();
    let s = captures[0].source.clone().admit().unwrap();
    let trainer =
        CpuFoundationTrainer::from_checkpoint(s.phenotype, s.foundation, s.actor).unwrap();
    assert!(wrong
        .evaluate(&trainer, &data, PredictiveSplit::Validation)
        .is_err());
}
