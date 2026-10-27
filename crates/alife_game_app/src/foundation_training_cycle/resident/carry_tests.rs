//! Synthetic, opt-in discrimination of retained GPU scratch versus a restored
//! learner. Both arms consume the SAME immutable production-recorded replay.
use super::*;
use alife_training::{FoundationTrainerCheckpoint, PpoValueHeadCheckpoint};

fn float_bits(values: &[f32]) -> Vec<u32> {
    values.iter().map(|value| value.to_bits()).collect()
}

#[derive(Debug, serde::Serialize)]
struct FloatDifference {
    coordinates: usize,
    different_bits: usize,
    first_difference: Option<(usize, u32, u32)>,
    max_absolute: f64,
    mean_absolute: f64,
    rms: f64,
}

fn differences(a: &[f32], b: &[f32]) -> FloatDifference {
    assert_eq!(a.len(), b.len(), "numeric comparison length mismatch");
    assert!(a.iter().chain(b).all(|value| value.is_finite()));
    let mut count = 0usize;
    let mut first = None;
    let mut max = 0.0_f64;
    let mut sum = 0.0_f64;
    let mut squares = 0.0_f64;
    for (index, (left, right)) in a.iter().zip(b).enumerate() {
        if left.to_bits() != right.to_bits() {
            count += 1;
            first.get_or_insert((index, left.to_bits(), right.to_bits()));
        }
        let difference = (f64::from(*left) - f64::from(*right)).abs();
        max = max.max(difference);
        sum += difference;
        squares += difference * difference;
    }
    let divisor = a.len().max(1) as f64;
    FloatDifference {
        coordinates: a.len(),
        different_bits: count,
        first_difference: first,
        max_absolute: max,
        mean_absolute: sum / divisor,
        rms: (squares / divisor).sqrt(),
    }
}

fn float_difference(label: &str, a: &[f32], b: &[f32]) -> bool {
    let stats = differences(a, b);
    let equal = stats.different_bits == 0;
    eprintln!(
        "{}",
        serde_json::json!({"kind":"same-recorded-replay-float-difference-v1", "component":label,"statistics":stats})
    );
    equal
}

fn compare_heads(
    label: &str,
    a: &FoundationTrainerCheckpoint,
    b: &FoundationTrainerCheckpoint,
    av: &PpoValueHeadCheckpoint,
    bv: &PpoValueHeadCheckpoint,
) {
    assert_eq!(a.schema_version, b.schema_version);
    let weights = float_difference(&format!("{label}.actor.weights"), &a.weights, &b.weights);
    let first = float_difference(
        &format!("{label}.actor.first_moment"),
        &a.first_moment,
        &b.first_moment,
    );
    let second = float_difference(
        &format!("{label}.actor.second_moment"),
        &a.second_moment,
        &b.second_moment,
    );
    let value = float_difference(
        &format!("{label}.value.parameters"),
        &av.parameters,
        &bv.parameters,
    );
    assert_eq!(a.update_ages, b.update_ages, "{label} coordinate ages");
    assert_eq!(a.config, b.config, "{label} Adam config");
    assert_eq!(a.stage_mask, b.stage_mask, "{label} trainability mask");
    assert_eq!(a.optimizer_step, b.optimizer_step, "{label} actor age");
    assert_eq!(a.source_foundation_digest, b.source_foundation_digest);
    assert_eq!(a.phenotype_hash, b.phenotype_hash);
    assert_eq!(av.optimizer_step, bv.optimizer_step, "{label} value age");
    assert_eq!(av.feature_count, bv.feature_count);
    assert_eq!(
        av.last_updated_policy_version,
        bv.last_updated_policy_version
    );
    assert!(
        weights && first && second && value,
        "{label}: numeric difference recorded above; investigate before assigning a cause"
    );
}

#[test]
fn resident_difference_statistics_preserve_signed_zero_and_subnormal_evidence() {
    assert!(float_difference(
        "equal",
        &[0.0, f32::from_bits(1)],
        &[0.0, f32::from_bits(1)]
    ));
    assert!(!float_difference("signed-zero", &[0.0], &[-0.0]));
    assert!(!float_difference(
        "adjacent",
        &[1.0],
        &[f32::from_bits(1.0_f32.to_bits() + 1)]
    ));
    assert_eq!(float_bits(&[0.0, -0.0]), vec![0, 0x8000_0000]);
    let stats = differences(&[0.0, 4.0], &[0.0, 0.0]);
    assert_eq!(stats.different_bits, 1);
    assert_eq!(stats.first_difference, Some((1, 4.0_f32.to_bits(), 0)));
    assert_eq!(stats.max_absolute, 4.0);
    assert_eq!(stats.mean_absolute, 2.0);
    assert_eq!(stats.rms, 8.0_f64.sqrt());
    let signed = differences(&[0.0], &[-0.0]);
    assert_eq!(signed.different_bits, 1);
    assert_eq!(signed.max_absolute, 0.0);
    let extreme = differences(&[f32::MAX], &[-f32::MAX]);
    assert!(extreme.max_absolute.is_finite() && extreme.rms.is_finite());
}

#[test]
#[ignore = "requires an exclusively admitted Windows hardware GPU; no live creature or production seed"]
fn resident_same_recorded_replay_warm_rebind_matches_checkpoint_restore() {
    std::thread::Builder::new()
        .name("same-recorded-replay-carry".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(same_recorded_replay_assay)
        .unwrap()
        .join()
        .unwrap();
}

fn same_recorded_replay_assay() {
    let root = std::env::temp_dir().join(format!(
        "alife-same-recorded-carry-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    eprintln!(
        "same-recorded-replay disposable fixture (retained on failure): {}",
        root.display()
    );
    let initial = root.join("initial");
    run_foundation_training_cycle(&initial, 91301, 2).unwrap();
    let source = root.join("source-policy-one");
    resume_foundation_training_cycle_with_final_private_life(
        &initial,
        &source,
        91302,
        2,
        FoundationTeacherLesson::Feeding,
        1.0,
        Default::default(),
    )
    .unwrap();
    let admitted = load_cycle_continuation(&source, None).unwrap();
    let state = AdmittedCycleState {
        asset: admitted.asset,
        policy_version: admitted.policy_version,
        restored_actor: Some(admitted.actor),
        restored_value: Some(admitted.value),
        founder_seed_base: admitted.founder_seed_base,
        objective_state_reset: false,
        objective_transition_from: None,
        resident: None,
    };
    let warm_output = root.join("actual-resident-warmup");
    let timing = CycleStageTiming::default();
    let outcome = crate::foundation_training_output::in_new_directory(
        &warm_output,
        || {
            run_foundation_training_cycle_in_owned_output(
                &warm_output,
                91303,
                16,
                None,
                Some(FoundationTeacherLesson::Feeding),
                state,
                1.0,
                0,
                None,
                false,
                &timing,
                None,
                true,
                Default::default(),
                true,
            )
        },
        |error| {
            record_cycle_failure(
                &warm_output,
                error,
                91303,
                16,
                None,
                Some(FoundationTeacherLesson::Feeding),
            )
        },
    )
    .unwrap();
    let FoundationCycleOutcome::Resident(warm) = outcome else {
        panic!("warm-up did not retain the learner")
    };
    let ResidentEpisode {
        learner: mut warm,
        receipt,
    } = *warm;
    // These are actual returned native heads/receipt, not a fabricated cycle.
    fs::write(
        warm_output.join("actor-checkpoint.json"),
        serde_json::to_vec(&warm.trainer.checkpoint().unwrap()).unwrap(),
    )
    .unwrap();
    fs::write(
        warm_output.join("value-checkpoint.json"),
        serde_json::to_vec(&warm.value.checkpoint(warm.trainer.session()).unwrap()).unwrap(),
    )
    .unwrap();
    fs::write(
        warm_output.join("cycle.json"),
        serde_json::to_vec(&receipt).unwrap(),
    )
    .unwrap();
    inspect_native_training_frontier(&warm_output, receipt.policy_version).unwrap();
    let capture_dir = root.join("one-immutable-recording");
    let captured =
        capture_foundation_critic_trajectory(&warm_output, &capture_dir, 91304, 16, 1.0).unwrap();
    let policy = captured["collection_policy_version"].as_u64().unwrap();
    let (phenotype, _) = crate::foundation_archived_success_replay::compile_source_with_inputs(
        warm.asset.clone(),
        91304,
        warm.founder_seed_base,
    )
    .unwrap();
    let replay_source: crate::FoundationReplaySource =
        serde_json::from_value(captured["source"].clone()).unwrap();
    let references: Vec<crate::FoundationReplayRecordRef> =
        serde_json::from_slice(&fs::read(capture_dir.join("replay-manifest.json")).unwrap())
            .unwrap();
    assert!(
        references
            .iter()
            .all(|row| row.segment == references[0].segment),
        "assay requires one actual waking segment"
    );
    let all = load_foundation_replay_window(
        &capture_dir.join("replay"),
        &references,
        0,
        &replay_source,
        &phenotype,
        FoundationReplayBudget::default(),
    )
    .unwrap();
    let rows = captured["rows"].as_array().unwrap();
    let loss = load_foundation_replay_window(
        &capture_dir.join("replay"),
        &references[..rows.len()],
        0,
        &replay_source,
        &phenotype,
        FoundationReplayBudget::default(),
    )
    .unwrap();
    let mut expected_actor = warm.trainer.checkpoint().unwrap();
    let expected_value = warm.value.checkpoint(warm.trainer.session()).unwrap();
    warm.trainer
        .rebind_for_next_cohort(phenotype.clone(), warm.asset.clone())
        .unwrap();
    let actor = warm.trainer.checkpoint().unwrap();
    let value = warm.value.checkpoint(warm.trainer.session()).unwrap();
    expected_actor.phenotype_hash = phenotype.phenotype_hash();
    expected_actor.source_foundation_digest = warm.asset.digest();
    compare_heads(
        "warm-rebind-preserves-heads",
        &expected_actor,
        &actor,
        &expected_value,
        &value,
    );
    let staging = warm
        .trainer
        .session()
        .backend()
        .new_staging_like_live()
        .unwrap();
    let mut restored = FoundationTrainer::from_session(
        alife_runtime::GpuAuthoritativeSession::new(
            staging,
            alife_runtime::GpuSessionConsumerKind::Training,
        ),
        phenotype.clone(),
        warm.asset.clone(),
        actor.stage_mask.clone(),
        actor.config,
    )
    .unwrap();
    restored.restore_checkpoint(&actor).unwrap();
    let mut restored_value = PpoTrainingState::from_checkpoint(value.clone()).unwrap();
    eprintln!(
        "same-device hardware receipt: {:?}",
        warm.trainer.hardware_receipt()
    );
    compare_heads(
        "before-replay",
        &actor,
        &restored.checkpoint().unwrap(),
        &value,
        &restored_value.checkpoint(restored.session()).unwrap(),
    );
    let wp = warm
        .value
        .predict_values(&mut warm.trainer, &all.sequence)
        .unwrap();
    let rp = restored_value
        .predict_values(&mut restored, &all.sequence)
        .unwrap();
    let predicted_equal = float_difference("frozen-predictions", &wp, &rp);
    compare_heads(
        "after-frozen-prediction",
        &warm.trainer.checkpoint().unwrap(),
        &restored.checkpoint().unwrap(),
        &warm.value.checkpoint(warm.trainer.session()).unwrap(),
        &restored_value.checkpoint(restored.session()).unwrap(),
    );
    assert!(
        predicted_equal,
        "same-recorded replay predictions differ; magnitude evidence printed above"
    );
    let mut transitions = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        transitions.push(PpoTransition {
            policy_version: policy,
            trajectory_id: 91304,
            step: i as u64,
            action: joint_action(&all.behavior[i]).unwrap(),
            reward: row["reward"].as_f64().unwrap() as f32,
            old_value: row["current_value"].as_f64().unwrap() as f32,
            next_value: row["next_value"].as_f64().unwrap() as f32,
            elapsed_seconds: row["elapsed_seconds"].as_f64().unwrap() as f32,
            boundary: match row["boundary"].as_str().unwrap() {
                "Terminated" => PpoBoundary::Terminated,
                "Truncated" => PpoBoundary::Truncated,
                "Continuing" => PpoBoundary::Continuing,
                _ => panic!("unknown recorded boundary"),
            },
        });
    }
    let config = cycle_ppo_config(1.0).unwrap();
    let batch = PpoBatch::from_rollout(policy, transitions, config).unwrap();
    let feature_bytes = fs::read(capture_dir.join("critic-features.f32le")).unwrap();
    let floats = feature_bytes
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
        .collect::<Vec<_>>();
    assert_eq!(feature_bytes.len() % 4, 0);
    let features = floats
        .chunks_exact(phenotype.neuron_count() as usize)
        .map(|r| r.to_vec())
        .collect::<Vec<_>>();
    assert_eq!(floats.len() % phenotype.neuron_count() as usize, 0);
    assert_eq!(features.len(), references.len());
    assert!(features.iter().flatten().all(|value| value.is_finite()));
    apply_identical_recording(
        &mut warm.trainer,
        &mut warm.value,
        &loss.sequence,
        &batch,
        &features,
        config,
        policy,
    );
    assert!(warm.trainer.optimizer_step() > actor.optimizer_step);
    assert!(
        warm.value
            .checkpoint(warm.trainer.session())
            .unwrap()
            .optimizer_step
            > value.optimizer_step
    );
    apply_identical_recording(
        &mut restored,
        &mut restored_value,
        &loss.sequence,
        &batch,
        &features,
        config,
        policy,
    );
    compare_heads(
        "after-identical-update",
        &warm.trainer.checkpoint().unwrap(),
        &restored.checkpoint().unwrap(),
        &warm.value.checkpoint(warm.trainer.session()).unwrap(),
        &restored_value.checkpoint(restored.session()).unwrap(),
    );
    fs::remove_dir_all(root).unwrap();
}

fn apply_identical_recording(
    trainer: &mut FoundationTrainer,
    value: &mut PpoTrainingState,
    sequence: &alife_training::TrainingSequence,
    batch: &PpoBatch,
    features: &[Vec<f32>],
    config: PpoConfig,
    policy: u64,
) {
    let before = trainer.optimizer_step();
    // Deliberately the same recorded actions/rewards/targets for both arms. No
    // new world actions, teacher forcing or synthetic nourishment are involved.
    let actor_config = PpoConfig {
        value_coefficient: 0.0,
        ..config
    };
    let update = train_recurrent_ppo_cohort(
        trainer,
        value,
        1,
        8,
        |_| {
            Ok(PpoTrainingWindow {
                sequence: sequence.clone(),
                batch: batch.clone(),
                speech_targets: vec![None; sequence.ticks.len()],
                auxiliary: None,
            })
        },
        actor_config,
        policy,
    )
    .unwrap();
    let count = update.actor_optimizer_step.checked_sub(before).unwrap();
    let (checkpoint, _) = critic_residual::train(
        trainer.session(),
        value.checkpoint(trainer.session()).unwrap(),
        features,
        batch.transitions(),
        config,
        count,
        policy,
    )
    .unwrap();
    *value = PpoTrainingState::from_checkpoint(checkpoint).unwrap();
}
