//! A deliberately synthetic, checkpoint-admitted Adam fault, never a retained
//! user checkpoint or a claim that ordinary learning produced this history.
use super::*;

#[test]
#[ignore = "explicit synthetic GPU actor rejection after positive value apply"]
fn actor_gpu_rejection_restores_value_update_for_original_and_cache() {
    let (mut trainer, weight, neuron, lane) = fixture();
    assert!(trainer.read_replay_update_status_for_test().is_err());
    let mut state = PpoTrainingState::default();
    let config = PpoConfig {
        epochs: 1,
        target_kl: 100.0,
        entropy_coefficient: 0.0,
        value_coefficient: 0.5,
        ..Default::default()
    };
    let (before_actor, before_value) =
        warm_pair(&mut trainer, &mut state, weight, neuron, lane, config);
    let collected = windows_with_count(&mut trainer, &mut state, neuron, lane, config, 6, 0.0, 2);
    assert_eq!(
        collected
            .iter()
            .map(|window| window.batch.len())
            .collect::<Vec<_>>(),
        [3, 4]
    );
    assert_eq!(before_actor.update_ages[weight], 1);
    // Decoder readouts are unconstrained (trainer's sign policy 0). A recurrent
    // sign clamp could otherwise turn a nonfinite proposal into a finite value.
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
    let mut expected_norm = None;
    for cached in [false, true] {
        trainer.restore_checkpoint(&before_actor).unwrap();
        state = PpoTrainingState::from_checkpoint(before_value.clone()).unwrap();
        let (positive, stats, positive_loads) =
            run_owned(&mut trainer, &mut state, &collected, config, 6, cached);
        let positive = positive.unwrap();
        assert_eq!(positive_loads, [0, 1, 0, 1]);
        if cached {
            assert_active_cache(stats);
        }
        assert_eq!(positive.completed_epochs, 1);
        assert!(!positive.stopped_for_kl);
        progress_steps(&before_actor, &trainer.checkpoint().unwrap(), weight, 1);
        let accepted_value = state.checkpoint(trainer.session()).unwrap();
        assert_eq!(
            accepted_value.optimizer_step,
            before_value.optimizer_step + 1
        );
        assert_eq!(accepted_value.last_updated_policy_version, Some(6));
        let width = before_value.feature_count as usize + 1;
        for plane in 0..3 {
            let range = plane * width..(plane + 1) * width;
            assert_ne!(
                bits(&before_value.parameters[range.clone()]),
                bits(&accepted_value.parameters[range]),
                "positive control must change value parameter/m/v plane {plane}"
            );
        }
        let positive_status = trainer.read_replay_update_status_for_test().unwrap();
        assert!(positive_status[0].is_finite() && positive_status[0] > 0.0);
        assert_eq!(positive_status[1], 1.0);
        let positive_capture = capture(&trainer, &mut state);
        if let Some(expected) = &positive_expected {
            assert_semantics_equal(expected, &positive_capture);
        } else {
            positive_expected = Some(positive_capture.clone());
        }
        artifact.insert(format!("positive/cached={cached}"), positive_capture);

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
        assert_eq!(status[1], 0.0, "final GPU Adam preflight rejected");
        if let Some(norm) = expected_norm {
            assert_eq!(status[0].to_bits(), norm);
        }
        expected_norm = Some(status[0].to_bits());
        assert_persistent_equal(&fault_actor, &before_value, &trainer, &mut state);
        let failed = capture(&trainer, &mut state);
        assert_eq!(failed["objective_preflight"], bits(&[1.0]));
        // Compare all five post-error planes across execution paths. The two
        // gradient scratch planes need not equal their pre-cohort contents.
        if let Some(expected) = &fault_expected {
            assert_semantics_equal(expected, &failed);
        } else {
            fault_expected = Some(failed.clone());
        }
        artifact.insert(format!("fault/cached={cached}"), failed);
        eprintln!(
            "actor_gpu_rollback cached={cached}; error=MalformedReadback; loads={loads:?}; norm={}; actor_valid={}; value_preflight=1; positive_value_planes_changed=3; actor_and_persistent_value_unchanged=true",
            status[0], status[1]
        );
    }
    if let Some(directory) = std::env::var_os("ALIFE_SOFTWARE_PPO_CAPTURE_DIR") {
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            std::path::Path::new(&directory).join("rollback-semantic-words.json"),
            serde_json::to_vec(&artifact).unwrap(),
        )
        .unwrap();
    }
    // The matched positive controls observe accepted value updates. For faults,
    // source dispatch ordering establishes that value apply precedes actor
    // preflight in the submitted encoder; transient value writes are not sampled.
}
