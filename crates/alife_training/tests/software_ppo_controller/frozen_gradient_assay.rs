//! Bounded release-only actual-controller assay. All data is disposable synthetic
//! replay; execution mode and actual adapter identity are recorded explicitly.

use super::*;
use std::{path::Path, time::Instant};

const POLICY: u64 = 6;
const CACHE_BYTES: u64 = 278_544;
const LOADS: [usize; 6] = [0, 1, 0, 1, 0, 1];

struct Run {
    report: serde_json::Value,
    receipt: Vec<u32>,
    words: BTreeMap<String, Vec<u32>>,
}

fn wide_progress(
    before: &FoundationTrainerCheckpoint,
    after: &FoundationTrainerCheckpoint,
    updates: u32,
) -> serde_json::Value {
    assert_eq!(after.optimizer_step, before.optimizer_step + updates);
    assert_eq!(before.schema_version, after.schema_version);
    assert_eq!(before.phenotype_hash, after.phenotype_hash);
    assert_eq!(
        before.source_foundation_digest,
        after.source_foundation_digest
    );
    assert_eq!(before.stage_mask, after.stage_mask);
    let config_words = |config: AdamWConfig| {
        bits(&[
            config.learning_rate,
            config.beta1,
            config.beta2,
            config.epsilon,
            config.weight_decay,
            config.gradient_clip,
        ])
    };
    assert_eq!(config_words(before.config), config_words(after.config));
    let mut changed_weights = 0;
    let mut nonzero_first_moments = 0;
    let mut positive_second_moments = 0;
    let mut frozen = 0;
    for index in 0..before.weights.len() {
        assert!(after.weights[index].is_finite());
        assert!(after.first_moment[index].is_finite());
        assert!(after.second_moment[index].is_finite());
        assert!(after.second_moment[index] >= 0.0);
        if before.stage_mask.is_trainable(index) {
            changed_weights +=
                usize::from(before.weights[index].to_bits() != after.weights[index].to_bits());
            nonzero_first_moments += usize::from(after.first_moment[index] != 0.0);
            positive_second_moments += usize::from(after.second_moment[index] > 0.0);
            assert_eq!(
                after.update_ages[index],
                before.update_ages[index] + updates
            );
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
    assert!(changed_weights > 0);
    assert!(nonzero_first_moments > 0);
    assert!(positive_second_moments > 0);
    assert!(frozen > 0);
    serde_json::json!({
        "trainable_weights": before.stage_mask.trainable_count(),
        "changed_trainable_weights": changed_weights,
        "nonzero_trainable_first_moments": nonzero_first_moments,
        "positive_trainable_second_moments": positive_second_moments,
        "frozen_coordinates_exact": frozen,
        "actor_optimizer_step": after.optimizer_step,
        "all_trainable_integer_ages_incremented_by": updates,
    })
}

fn flush(trainer: &FoundationTrainer) {
    let (device, queue) = trainer
        .session()
        .backend()
        .offline_training_device_queue()
        .unwrap();
    let submission = queue.submit(std::iter::empty());
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .unwrap();
}

fn run_mode(
    trainer: &mut FoundationTrainer,
    state: &mut PpoTrainingState,
    boundary_actor: &FoundationTrainerCheckpoint,
    boundary_value: &PpoValueHeadCheckpoint,
    windows: &[Arc<PpoTrainingWindow>],
    config: PpoConfig,
    cached: bool,
) -> Run {
    // Restore persistent history and precreate the largest replay/objective.
    // The shared controller retains each mode's intended per-window preparation.
    trainer.restore_checkpoint(boundary_actor).unwrap();
    *state = PpoTrainingState::from_checkpoint(boundary_value.clone()).unwrap();
    trainer.prepare_replay(&windows[1].sequence).unwrap();
    state.prepare_for_replay(trainer).unwrap();
    assert_persistent_equal(boundary_actor, boundary_value, trainer, state);
    let mut stats = FrozenGradientCacheStats::default();
    let mut loads = [usize::MAX; LOADS.len()];
    let mut load_count = 0;
    let load = |index: usize| {
        // Fixed-size order bookkeeping and an Arc increment only; no replay
        // cloning, disk reads, collection, or buffer capture occurs in loaders.
        loads[load_count] = index;
        load_count += 1;
        Ok(Arc::clone(&windows[index]))
    };
    flush(trainer);
    let (result, elapsed) = if cached {
        let start = Instant::now();
        let result = train_recurrent_ppo_cohort_with_frozen_gradients_bounded_for_test(
            trainer,
            state,
            windows.len(),
            8,
            load,
            config,
            POLICY,
            replay_tick_bound(windows),
            &mut stats,
        );
        (result, start.elapsed())
    } else {
        let start = Instant::now();
        let result = train_recurrent_ppo_cohort_with_windows(
            trainer,
            state,
            windows.len(),
            8,
            load,
            config,
            POLICY,
        );
        (result, start.elapsed())
    };
    // Additional proof readbacks, checkpoints, comparisons and reports are outside the clock.
    if cached {
        use std::io::Write as _;
        let _ = writeln!(
            std::io::stderr().lock(),
            "frozen_gradient_admission_snapshot={:?}",
            stats.admission
        );
    }
    let receipt = result.unwrap();
    assert_eq!(load_count, LOADS.len());
    assert_eq!(loads, LOADS);
    assert_eq!(receipt.completed_epochs, 2);
    assert!(!receipt.stopped_for_kl);
    assert_eq!(receipt.mean_kl_before_updates.len(), 2);
    assert_eq!(
        receipt.actor_optimizer_step,
        boundary_actor.optimizer_step + 2
    );
    assert_eq!(receipt.value_optimizer_step, boundary_value.optimizer_step);
    if cached {
        assert_active_cache(stats);
        assert_eq!(stats.requested_gpu_bytes, CACHE_BYTES);
    } else {
        assert_eq!(stats, FrozenGradientCacheStats::default());
    }
    let actor = trainer.checkpoint().unwrap();
    let value = state.checkpoint(trainer.session()).unwrap();
    let progress = wide_progress(boundary_actor, &actor, 2);
    assert_eq!(value.feature_count, boundary_value.feature_count);
    assert_eq!(value.optimizer_step, boundary_value.optimizer_step);
    assert_eq!(value.last_updated_policy_version, Some(POLICY));
    assert_eq!(bits(&value.parameters), bits(&boundary_value.parameters));
    let words = capture(trainer, state);
    let word_count = words.values().map(|words| words.len() as u64).sum::<u64>();
    let receipt = receipt_words(&receipt);
    Run {
        report: serde_json::json!({
            "mode": if cached { "frozen_cache" } else { "original" },
            "seconds": elapsed.as_secs_f64(),
            "receipt_words": receipt,
            "loader_indices": loads,
            "capture_sections": words.len(),
            "capture_u32_words": word_count,
            "cache": {
                "eligible": stats.eligible,
                "requested_gpu_bytes": stats.requested_gpu_bytes,
                "raw_cached_windows": stats.raw_cached_windows,
                "reused_windows": stats.reused_windows,
                "admission": stats.admission,
            },
            "accepted_actor_progress": progress,
            "value_parameters_and_moments_frozen_bit_exact": true,
            "value_optimizer_step": value.optimizer_step,
        }),
        receipt,
        words,
    }
}

fn save_report(report: &serde_json::Value) {
    if let Some(directory) = std::env::var_os("ALIFE_SOFTWARE_PPO_CAPTURE_DIR") {
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            Path::new(&directory).join("frozen-gradient-release-assay.json"),
            serde_json::to_vec_pretty(report).unwrap(),
        )
        .unwrap();
    }
}

#[test]
#[ignore = "one bounded actual-PPO release assay with explicit test-adapter selection"]
#[allow(clippy::assertions_on_constants)]
fn representative_actual_ppo_frozen_gradient_release_assay() {
    // Keep ordinary debug correctness targets compilable; enforce release at execution.
    assert!(
        !cfg!(debug_assertions),
        "run this bounded assay with all-dependency --release"
    );
    let (mut trainer, selected_weight, neuron, lane) = fixture_with_wide_mask(true);
    assert_eq!(selected_weight, 25_728);
    let initial_actor = trainer.checkpoint().unwrap();
    assert_eq!(initial_actor.stage_mask.trainable_count(), 10_923);
    let mut state = PpoTrainingState::default();
    let warm_config = PpoConfig {
        epochs: 1,
        target_kl: 100.0,
        entropy_coefficient: 0.0,
        value_coefficient: 0.5,
        ..Default::default()
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
    let boundary_actor = trainer.checkpoint().unwrap();
    let warm_progress = wide_progress(&initial_actor, &boundary_actor, 1);
    let boundary_value = state.checkpoint(trainer.session()).unwrap();
    assert_eq!(boundary_value.optimizer_step, 1);
    assert_eq!(boundary_value.last_updated_policy_version, Some(5));
    let width = boundary_value.feature_count as usize + 1;
    assert!(boundary_value.parameters[..width].iter().any(|v| *v != 0.0));
    assert!(boundary_value.parameters[width..2 * width]
        .iter()
        .any(|v| *v != 0.0));
    assert!(boundary_value.parameters[2 * width..]
        .iter()
        .any(|v| *v > 0.0));
    let config = PpoConfig {
        epochs: 2,
        value_coefficient: 0.0,
        ..warm_config
    };
    let windows = windows_with_shapes(
        &mut trainer,
        &mut state,
        neuron,
        lane,
        config,
        POLICY,
        0.0,
        &[(256, 0), (384, 128)],
        true,
    )
    .into_iter()
    .map(Arc::new)
    .collect::<Vec<_>>();
    assert_eq!(
        windows.iter().map(|w| w.batch.len()).collect::<Vec<_>>(),
        [256, 256]
    );
    for window in &windows {
        assert!(window
            .sequence
            .ticks
            .iter()
            .all(|tick| tick.microstep_count == 4));
        assert!(window.speech_targets.iter().all(Option::is_none));
        assert!(window.auxiliary.is_none());
    }
    assert_persistent_equal(&boundary_actor, &boundary_value, &trainer, &mut state);
    let stop_file = std::env::var_os("ALIFE_FROZEN_ASSAY_STOP_FILE");
    let mut report = serde_json::json!({
        "scope": "actual full PPO controller, disposable synthetic terminal replay only",
        "execution": execution::metadata(&trainer),
        "ppo_config": config,
        "warm_ppo_config": warm_config,
        "actor_adam_config": boundary_actor.config,
        "configuration": {
            "neurons": trainer.phenotype().neuron_count(),
            "synapses": trainer.phenotype().synapses().len(),
            "window_ticks": [256, 384], "burn_in_ticks": [0, 128],
            "loss_rows": [256, 256], "microsteps_every_tick": 4,
            "speech_labels": false, "auxiliary_objective": false,
            "epochs": 2, "effective_batch_size": 8, "value_coefficient": 0.0,
            "entropy_coefficient": 0.0, "collection_policy_version": POLICY,
            "target_kl": config.target_kl, "loader_indices": LOADS,
            "trainable_weights": 10_923, "stage_mask": "every third synapse, including 25728",
            "cache_requested_gpu_bytes": CACHE_BYTES,
            "cache_raw_windows": 2, "cache_reused_windows": 2,
            "later_epoch": "fresh original evaluation and backpropagation",
            "warm_value_optimizer_step": boundary_value.optimizer_step,
            "warmed_nonzero_value_coefficients_and_both_moment_planes": true,
            "all_dependencies_release_required": true,
            "planned_warmup_pairs": 1, "planned_measured_pairs": 5,
        },
        "timing_scope": "only the actual controller call, including Arc loader callbacks with fixed-size order bookkeeping; restore, largest replay and objective allocation, setup submission/wait, collection, checkpoints, captures, comparisons, and JSON excluded",
        "warm_actor_progress": warm_progress,
        "pairs": [], "completed_measured_pairs": 0, "stopped": false,
        "performance_claim": false,
        "rss_scope": "external driver may report whole-process RSS; not VRAM or live allocator residency",
    });
    let mut completed_measured_pairs = 0;
    for pair in 0..6 {
        if stop_file
            .as_ref()
            .is_some_and(|path| Path::new(path).exists())
        {
            report["stopped"] = true.into();
            break;
        }
        let order = if pair % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        };
        let first = run_mode(
            &mut trainer,
            &mut state,
            &boundary_actor,
            &boundary_value,
            &windows,
            config,
            order[0],
        );
        let second = run_mode(
            &mut trainer,
            &mut state,
            &boundary_actor,
            &boundary_value,
            &windows,
            config,
            order[1],
        );
        assert_eq!(first.receipt, second.receipt);
        assert_semantics_equal(&first.words, &second.words);
        let compared_words = first
            .words
            .values()
            .map(|words| words.len() as u64)
            .sum::<u64>();
        if pair > 0 {
            completed_measured_pairs += 1;
        }
        let pair_report = serde_json::json!({
            "pair": pair, "warmup": pair == 0,
            "execution_order": [first.report["mode"].clone(), second.report["mode"].clone()],
            "runs": [first.report, second.report],
            "all_receipt_and_capture_words_exact": true,
            "compared_capture_u32_words": compared_words,
        });
        eprintln!("frozen_gradient_release_pair={pair_report}");
        report["pairs"].as_array_mut().unwrap().push(pair_report);
        report["completed_measured_pairs"] = completed_measured_pairs.into();
        save_report(&report);
    }
    if !report["stopped"].as_bool().unwrap() {
        assert_eq!(report["pairs"].as_array().unwrap().len(), 6);
        assert_eq!(completed_measured_pairs, 5);
    }
    save_report(&report);
    eprintln!("frozen_gradient_release_assay={report}");
}
