//! Admission qualification only: real declared profiles, shared immutable
//! windows, and actual accepted updates. No physical-device or timing claim.
use super::*;
use alife_gpu_backend::GpuRuntimeProfile;

fn run_shared(
    trainer: &mut FoundationTrainer,
    state: &mut PpoTrainingState,
    windows: &[Arc<PpoTrainingWindow>],
    config: PpoConfig,
    max_ticks: Option<usize>,
) -> (PpoUpdateReceipt, FrozenGradientCacheStats, Vec<usize>) {
    let mut stats = FrozenGradientCacheStats::default();
    let mut loads = Vec::new();
    let load = |index: usize| {
        loads.push(index);
        Ok(Arc::clone(&windows[index]))
    };
    let receipt = if let Some(max_ticks) = max_ticks {
        train_recurrent_ppo_cohort_with_frozen_gradients_bounded_for_test(
            trainer,
            state,
            windows.len(),
            8,
            load,
            config,
            6,
            max_ticks,
            &mut stats,
        )
    } else {
        train_recurrent_ppo_cohort_with_windows(trainer, state, windows.len(), 8, load, config, 6)
    }
    .unwrap();
    assert_eq!(loads, [0, 1, 0, 1]);
    assert_eq!(receipt.completed_epochs, 1);
    assert!(!receipt.stopped_for_kl);
    (receipt, stats, loads)
}

fn observation(trainer: &FoundationTrainer) -> serde_json::Value {
    let backend = trainer.session().backend();
    serde_json::json!({
        "execution": execution::metadata(trainer),
        "declared_profile": backend.runtime_profile(),
        "scope": "the exact admission snapshot is recorded once in experimental stats; not free VRAM, a reservation, or future reserved peak",
    })
}

fn summary(
    trainer: &FoundationTrainer,
    state: &mut PpoTrainingState,
    weight: usize,
    receipt: &PpoUpdateReceipt,
    stats: FrozenGradientCacheStats,
    loads: &[usize],
) -> serde_json::Value {
    let actor = trainer.checkpoint().unwrap();
    let value = state.checkpoint(trainer.session()).unwrap();
    let width = value.feature_count as usize + 1;
    serde_json::json!({
        "receipt_words": receipt_words(receipt),
        "loads": loads,
        "stats": {"eligible": stats.eligible, "requested_gpu_bytes": stats.requested_gpu_bytes,
            "raw_cached_windows": stats.raw_cached_windows, "reused_windows": stats.reused_windows,
            "admission": stats.admission},
        "actor_optimizer_step": actor.optimizer_step,
        "actor_coordinate_count": actor.weights.len(),
        "selected_actor_coordinate": weight,
        "selected_actor_w_m_v_age_words": [actor.weights[weight].to_bits(),
            actor.first_moment[weight].to_bits(), actor.second_moment[weight].to_bits(), actor.update_ages[weight]],
        "value_optimizer_step": value.optimizer_step,
        "last_policy_version": value.last_updated_policy_version,
        "value_bias_parameter_m_v_words": [value.parameters[width-1].to_bits(),
            value.parameters[2*width-1].to_bits(), value.parameters[3*width-1].to_bits()],
        "full_actor_and_value_equality": "asserted in the fixture; this small summary does not retain every word",
    })
}

#[test]
#[ignore = "explicit software Vulkan requested-byte admission qualification"]
fn requested_admission_accepts_refuses_and_drops_before_larger_replay() {
    // This constructor rejects a hardware selector before requesting a device.
    // It never relabels a physical adapter as this software policy fixture.
    let (mut trainer, weight, neuron, lane) =
        fixture_with_profile(GpuRuntimeProfile::production_v1());
    let mut state = PpoTrainingState::default();
    let config = PpoConfig {
        epochs: 1,
        value_coefficient: 0.5,
        target_kl: 100.0,
        entropy_coefficient: 0.0,
        ..Default::default()
    };
    let (warm_actor, warm_value) =
        warm_pair(&mut trainer, &mut state, weight, neuron, lane, config);
    let windows = windows_with_shapes(
        &mut trainer,
        &mut state,
        neuron,
        lane,
        config,
        6,
        0.0,
        &[(256, 0), (384, 128)],
        true,
    )
    .into_iter()
    .map(Arc::new)
    .collect::<Vec<_>>();
    assert_eq!(
        windows
            .iter()
            .map(|window| window.batch.len())
            .collect::<Vec<_>>(),
        [256, 256]
    );
    let mut artifacts = serde_json::Map::new();
    artifacts.insert(
        "production_profile_before_controller".into(),
        observation(&trainer),
    );

    let (fresh, stats, loads) = run_shared(&mut trainer, &mut state, &windows, config, None);
    let accepted_actor = trainer.checkpoint().unwrap();
    let accepted_value = state.checkpoint(trainer.session()).unwrap();
    progress_steps(&warm_actor, &accepted_actor, weight, 1);
    assert_eq!(accepted_value.optimizer_step, warm_value.optimizer_step + 1);
    assert_eq!(accepted_value.last_updated_policy_version, Some(6));
    assert_ne!(
        bits(&warm_value.parameters),
        bits(&accepted_value.parameters)
    );
    artifacts.insert(
        "fresh".into(),
        summary(&trainer, &mut state, weight, &fresh, stats, &loads),
    );

    for (name, max_ticks) in [("admitted", 384), ("geometry_fallback", 256)] {
        trainer.restore_checkpoint(&warm_actor).unwrap();
        state = PpoTrainingState::from_checkpoint(warm_value.clone()).unwrap();
        let (receipt, stats, loads) =
            run_shared(&mut trainer, &mut state, &windows, config, Some(max_ticks));
        if max_ticks == 384 {
            assert_active_cache(stats);
        } else {
            assert!(!stats.eligible);
            assert!(stats.requested_gpu_bytes > 0);
            assert_eq!(stats.raw_cached_windows, 1);
            assert_eq!(stats.reused_windows, 0);
        }
        let admission = stats
            .admission
            .expect("actual first-wait admission snapshot");
        assert!(admission.admitted);
        let baseline = admission.allocator_reserved_baseline_bytes.unwrap();
        let additional = admission.additional_requested_upper_bytes.unwrap();
        let ceiling = admission.declared_profile_ceiling_bytes.unwrap();
        assert!(baseline > 0 && additional > 0);
        assert!(baseline.checked_add(additional).unwrap() <= ceiling);
        assert_eq!(receipt_words(&receipt), receipt_words(&fresh));
        assert_actor_equal(&accepted_actor, &trainer.checkpoint().unwrap());
        assert_value_equal(
            &accepted_value,
            &state.checkpoint(trainer.session()).unwrap(),
        );
        eprintln!(
            "requested_admission mode={name}; max_replay_ticks={max_ticks}; stats={stats:?}; loads={loads:?}; accepted_actor_and_value_exact=true"
        );
        artifacts.insert(
            name.into(),
            summary(&trainer, &mut state, weight, &receipt, stats, &loads),
        );
    }

    let tiny_profile = GpuRuntimeProfile {
        physical_allocation_ceiling_bytes: 1,
        ..GpuRuntimeProfile::production_v1()
    };
    tiny_profile.validate_contract().unwrap();
    let (mut tiny, tiny_weight, tiny_neuron, tiny_lane) = fixture_with_profile(tiny_profile);
    assert_eq!(
        (tiny_weight, tiny_neuron, tiny_lane),
        (weight, neuron, lane)
    );
    assert_eq!(
        tiny.session()
            .backend()
            .runtime_budget()
            .physical_allocation_ceiling_bytes,
        1
    );
    tiny.restore_checkpoint(&warm_actor).unwrap();
    assert_actor_equal(&warm_actor, &tiny.checkpoint().unwrap());
    let mut tiny_state = PpoTrainingState::from_checkpoint(warm_value.clone()).unwrap();
    artifacts.insert("tiny_profile_before_controller".into(), observation(&tiny));
    let (tiny_fresh, stats, loads) = run_shared(&mut tiny, &mut tiny_state, &windows, config, None);
    let tiny_actor = tiny.checkpoint().unwrap();
    let tiny_value = tiny_state.checkpoint(tiny.session()).unwrap();
    progress_steps(&warm_actor, &tiny_actor, weight, 1);
    assert_eq!(tiny_value.optimizer_step, warm_value.optimizer_step + 1);
    assert_eq!(tiny_value.last_updated_policy_version, Some(6));
    assert_ne!(bits(&warm_value.parameters), bits(&tiny_value.parameters));
    artifacts.insert(
        "tiny_fresh".into(),
        summary(&tiny, &mut tiny_state, weight, &tiny_fresh, stats, &loads),
    );

    tiny.restore_checkpoint(&warm_actor).unwrap();
    tiny_state = PpoTrainingState::from_checkpoint(warm_value).unwrap();
    let (refused, stats, loads) =
        run_shared(&mut tiny, &mut tiny_state, &windows, config, Some(384));
    assert!(!stats.eligible);
    assert_eq!(stats.requested_gpu_bytes, 0);
    assert_eq!(stats.raw_cached_windows, 0);
    assert_eq!(stats.reused_windows, 0);
    let admission = stats
        .admission
        .expect("actual tiny-profile refusal snapshot");
    assert!(!admission.admitted);
    assert_eq!(admission.declared_profile_ceiling_bytes, Some(1));
    assert!(admission.allocator_reserved_baseline_bytes.unwrap() > 1);
    assert!(admission.additional_requested_upper_bytes.unwrap() > 0);
    assert_eq!(receipt_words(&refused), receipt_words(&tiny_fresh));
    assert_actor_equal(&tiny_actor, &tiny.checkpoint().unwrap());
    assert_value_equal(&tiny_value, &tiny_state.checkpoint(tiny.session()).unwrap());
    eprintln!(
        "requested_admission mode=tiny_profile_refusal; actual_physical_ceiling=1; stats={stats:?}; loads={loads:?}; accepted_actor_and_value_exact=true"
    );
    artifacts.insert(
        "tiny_refused".into(),
        summary(&tiny, &mut tiny_state, weight, &refused, stats, &loads),
    );
    eprintln!(
        "requested_admission_observations={}",
        serde_json::Value::Object(artifacts.clone())
    );
    if let Some(directory) = std::env::var_os("ALIFE_SOFTWARE_PPO_CAPTURE_DIR") {
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            std::path::Path::new(&directory).join("admission-summary.json"),
            serde_json::to_vec_pretty(&artifacts).unwrap(),
        )
        .unwrap();
    }
}
