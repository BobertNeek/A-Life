//! One invalid Adam proposal must prevent writes to all other trainable lanes.
//! Fault input is synthetic, checkpoint-admitted, and never a user checkpoint.
use super::*;

fn assert_general_progress(
    trainer: &FoundationTrainer,
    before: &FoundationTrainerCheckpoint,
    after: &FoundationTrainerCheckpoint,
    decoder: usize,
) -> usize {
    super::fallback_qualification::assert_actor_progress(before, after, 1);
    assert_ne!(
        before.weights[decoder].to_bits(),
        after.weights[decoder].to_bits()
    );
    let recurrent_changed = trainer
        .phenotype()
        .synapses()
        .iter()
        .enumerate()
        .filter(|(index, synapse)| {
            matches!(synapse.kind(), CompiledSynapseKind::Recurrent)
                && before.stage_mask.is_trainable(*index)
                && before.weights[*index].to_bits() != after.weights[*index].to_bits()
                && before.first_moment[*index].to_bits() != after.first_moment[*index].to_bits()
                && before.second_moment[*index].to_bits() != after.second_moment[*index].to_bits()
        })
        .count();
    assert!(
        recurrent_changed > 0,
        "accepted update must change recurrent w/m/v"
    );
    recurrent_changed
}

#[test]
#[ignore = "explicit synthetic many-coordinate atomic GPU Adam rejection"]
fn one_invalid_decoder_proposal_prevents_all_wide_mask_actor_writes() {
    let (mut trainer, weight, neuron, lane) = fixture_with_wide_mask(true);
    let mut state = PpoTrainingState::default();
    let config = PpoConfig {
        epochs: 1,
        target_kl: 100.0,
        entropy_coefficient: 0.0,
        value_coefficient: 0.5,
        ..Default::default()
    };
    let initial = trainer.checkpoint().unwrap();
    let warm = windows_with_shapes(
        &mut trainer,
        &mut state,
        neuron,
        lane,
        config,
        5,
        0.0,
        &[(3, 0), (5, 1)],
        true,
    );
    let (warmed, _, _) = run_owned(&mut trainer, &mut state, &warm, config, 5, false);
    assert_eq!(warmed.unwrap().completed_epochs, 1);
    let before_actor = trainer.checkpoint().unwrap();
    let warm_recurrent = assert_general_progress(&trainer, &initial, &before_actor, weight);
    assert_eq!(before_actor.stage_mask.trainable_count(), 10_923);
    let before_value = state.checkpoint(trainer.session()).unwrap();
    assert_eq!(before_value.optimizer_step, 1);
    assert_eq!(before_value.last_updated_policy_version, Some(5));
    let collected = windows_with_shapes(
        &mut trainer,
        &mut state,
        neuron,
        lane,
        config,
        6,
        0.0,
        &[(3, 0), (5, 1)],
        true,
    );
    assert_eq!(
        collected
            .iter()
            .map(|window| window.batch.len())
            .collect::<Vec<_>>(),
        [3, 4]
    );
    assert_eq!(before_actor.update_ages[weight], 1);
    assert!(matches!(
        trainer.phenotype().synapses()[weight].kind(),
        CompiledSynapseKind::Decoder(_)
    ));
    let mut fault_actor = before_actor.clone();
    fault_actor.first_moment[weight] = f32::MAX;
    fault_actor
        .validate_for(trainer.phenotype(), before_actor.source_foundation_digest)
        .unwrap();

    let mut positive_expected = None;
    let mut fault_expected = None;
    let mut artifact = BTreeMap::new();
    let mut expected_recurrent = None;
    for cached in [false, true] {
        trainer.restore_checkpoint(&before_actor).unwrap();
        state = PpoTrainingState::from_checkpoint(before_value.clone()).unwrap();
        let (positive, stats, positive_loads) =
            run_owned(&mut trainer, &mut state, &collected, config, 6, cached);
        let positive = positive.unwrap();
        assert_eq!(positive_loads, [0, 1, 0, 1]);
        assert_eq!(positive.completed_epochs, 1);
        assert!(!positive.stopped_for_kl);
        if cached {
            assert_active_cache(stats);
        }
        let accepted_actor = trainer.checkpoint().unwrap();
        let recurrent_changed =
            assert_general_progress(&trainer, &before_actor, &accepted_actor, weight);
        if let Some(expected) = expected_recurrent {
            assert_eq!(recurrent_changed, expected);
        }
        expected_recurrent = Some(recurrent_changed);
        let accepted_value = state.checkpoint(trainer.session()).unwrap();
        assert_eq!(accepted_value.optimizer_step, 2);
        assert_eq!(accepted_value.last_updated_policy_version, Some(6));
        let width = before_value.feature_count as usize + 1;
        for plane in 0..3 {
            let range = plane * width..(plane + 1) * width;
            assert_ne!(
                bits(&before_value.parameters[range.clone()]),
                bits(&accepted_value.parameters[range])
            );
        }
        let positive_status = trainer.read_replay_update_status_for_test().unwrap();
        assert!(positive_status[0].is_finite() && positive_status[0] > 0.0);
        assert_eq!(positive_status[1], 1.0);
        let accepted = capture(&trainer, &mut state);
        if let Some(expected) = &positive_expected {
            assert_semantics_equal(expected, &accepted);
        } else {
            positive_expected = Some(accepted.clone());
        }
        artifact.insert(format!("positive/cached={cached}"), accepted);

        // Only one coordinate is faulty; other valid trainable recurrent lanes
        // demonstrably wrote w/m/v in the matched control above.
        trainer.restore_checkpoint(&fault_actor).unwrap();
        assert_actor_equal(&fault_actor, &trainer.checkpoint().unwrap());
        state = PpoTrainingState::from_checkpoint(before_value.clone()).unwrap();
        let (failure, stats, loads) =
            run_owned(&mut trainer, &mut state, &collected, config, 6, cached);
        assert!(matches!(failure, Err(TrainingError::MalformedReadback)));
        assert_eq!(loads, positive_loads);
        if cached {
            assert_active_cache(stats);
        }
        let status = trainer.read_replay_update_status_for_test().unwrap();
        assert!(status[0].is_finite() && status[0] > 0.0);
        assert_eq!(status[0].to_bits(), positive_status[0].to_bits());
        assert_eq!(status[1], 0.0);
        // This checks EVERY coordinate, including all valid trainable lanes,
        // and the durable value parameter/m/v planes plus policy/host counters.
        assert_persistent_equal(&fault_actor, &before_value, &trainer, &mut state);
        let failed = capture(&trainer, &mut state);
        assert_eq!(failed["objective_preflight"], bits(&[1.0]));
        if let Some(expected) = &fault_expected {
            assert_semantics_equal(expected, &failed);
        } else {
            fault_expected = Some(failed.clone());
        }
        artifact.insert(format!("fault/cached={cached}"), failed);
        eprintln!("wide_actor_gpu_rollback cached={cached}; trainable=10923; warm_recurrent_w_m_v_changed={warm_recurrent}; positive_recurrent_w_m_v_changed={recurrent_changed}; error=MalformedReadback; loads={loads:?}; norm={}; actor_valid={}; value_preflight=1; all_actor_coordinates_unchanged=true; persistent_value_unchanged=true", status[0], status[1]);
    }
    if let Some(directory) = std::env::var_os("ALIFE_SOFTWARE_PPO_CAPTURE_DIR") {
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            std::path::Path::new(&directory).join("rollback-wide-semantic-words.json"),
            serde_json::to_vec(&artifact).unwrap(),
        )
        .unwrap();
    }
    // As in the narrow case, transient value writes are inferred from source
    // ordering and matched accepted controls, not sampled during execution.
}
