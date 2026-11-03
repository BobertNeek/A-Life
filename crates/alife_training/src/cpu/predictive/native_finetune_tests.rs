use super::*;
use crate::cpu::{cpu_training_windows, train_cpu_residual_cohort, CpuCanonicalHost, CpuValueHead};
use crate::PpoConfig;
use alife_core::{BrainScaleTier, SensorProfile};
use alife_world::{create_canonical_new_game_with_n2048_candidate, CanonicalNewGameConfig};

#[test]
fn exported_predictive_founder_completes_actual_native_disposable_ppo_update() {
    let recording = recording::capture_public_simulation(recording::PublicCaptureConfig {
        founder_seed: 71,
        world_seeds: [72, 73, 74],
        sampling_seeds: [101, 202, 303],
        world_ticks: 5,
    })
    .unwrap();
    let original_source = serde_json::to_vec(&recording.source).unwrap();
    let (data, _) = recording::convert_recording(&recording).unwrap();
    let source = recording.source.clone().admit().unwrap();
    let mut trainer =
        CpuFoundationTrainer::from_checkpoint(source.phenotype, source.foundation, source.actor)
            .unwrap();
    let neurons = trainer
        .phenotype()
        .synapses()
        .iter()
        .filter(|s| s.kind() == CompiledSynapseKind::Recurrent)
        .map(|s| s.target() as usize)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(16)
        .collect();
    let mut head =
        PredictiveHead::new(trainer.phenotype(), data.schema.clone(), neurons, 8, 71).unwrap();
    head.train_step(&mut trainer, &data, 0, 0.03).unwrap();
    let run = artifact::PredictiveRun::from_dataset(
        71,
        &data,
        artifact::identity(&data).unwrap(),
        1,
        0.03,
        PredictiveInputs::Full,
    )
    .unwrap();
    let state = artifact::ExperimentalPredictiveState::capture(
        &trainer,
        &head,
        recording.source.clone(),
        run,
        None,
    )
    .unwrap();
    let original_state = serde_json::to_vec(&state).unwrap();
    let before_rebind = trainer.checkpoint();
    let (mut founder, inputs) = state.into_disposable_founder().unwrap();
    let before = founder.checkpoint();
    assert_eq!(before.weights, before_rebind.weights);
    assert_eq!(before.first_moment, before_rebind.first_moment);
    assert_eq!(before.second_moment, before_rebind.second_moment);
    assert_eq!(before.update_ages, before_rebind.update_ages);
    assert_eq!(before.optimizer_step, 1);
    assert_ne!(
        before.source_foundation_digest,
        before_rebind.source_foundation_digest
    );

    let mut config = CanonicalNewGameConfig::phase3(99, 1).unwrap();
    config.founder_seed_base = 71;
    config.brain_class = BrainScaleTier::Standard2048;
    config.sensor_profile = SensorProfile::GroundedObjectSlotsV1;
    let game = create_canonical_new_game_with_n2048_candidate(
        &config,
        inputs.n2048_candidate_asset().unwrap(),
    )
    .unwrap();
    let mut host = CpuCanonicalHost::new_cohort(game, &founder, 404).unwrap();
    let mut decisions = Vec::new();
    for _ in 0..5 {
        decisions.extend(host.step(&founder, 0.9).unwrap().decisions);
    }
    assert!(decisions.len() >= 3);
    assert_eq!(
        founder.checkpoint(),
        before,
        "collection must not mutate founder"
    );
    let phenotype = host
        .world()
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .phenotype()
        .clone();
    let mut value = CpuValueHead::from_checkpoint(recording.source.value.clone()).unwrap();
    let ppo = PpoConfig {
        epochs: 1,
        temperature: 0.9,
        ..PpoConfig::default()
    };
    let windows =
        cpu_training_windows(&decisions, None, &founder, &value, &phenotype, 2, ppo, 8, 0).unwrap();
    let receipt = train_cpu_residual_cohort(
        &mut founder,
        &mut value,
        windows.len(),
        windows.len(),
        |i| Ok(windows[i].clone()),
        ppo,
        2,
    )
    .unwrap();
    let after = founder.checkpoint();
    assert_eq!(receipt.completed_epochs, 1);
    assert!(!receipt.stopped_for_kl);
    assert_eq!(after.optimizer_step, before.optimizer_step + 1);
    assert_eq!(value.checkpoint().optimizer_step, 1);
    assert_eq!(
        after.source_foundation_digest,
        before.source_foundation_digest
    );
    assert_eq!(after.phenotype_hash, before.phenotype_hash);
    assert_eq!(after.stage_mask, before.stage_mask);
    assert!(after
        .weights
        .iter()
        .zip(&before.weights)
        .any(|(a, b)| a.to_bits() != b.to_bits()));
    assert!(after
        .first_moment
        .iter()
        .zip(&before.first_moment)
        .any(|(a, b)| a.to_bits() != b.to_bits()));
    for i in 0..before.weights.len() {
        if before.stage_mask.is_trainable(i) {
            assert_eq!(after.update_ages[i], before.update_ages[i] + 1);
        } else {
            assert_eq!(after.weights[i].to_bits(), before.weights[i].to_bits());
            assert_eq!(after.first_moment[i], before.first_moment[i]);
            assert_eq!(after.second_moment[i], before.second_moment[i]);
            assert_eq!(after.update_ages[i], before.update_ages[i]);
        }
    }
    assert_eq!(serde_json::to_vec(&state).unwrap(), original_state);
    assert_eq!(
        serde_json::to_vec(&recording.source).unwrap(),
        original_source
    );
    assert!(!serde_json::to_string(&after).unwrap().contains("head"));
    println!(
        "{}",
        serde_json::json!({"evidence":"actual fresh public native simulation; disposable update only","source_identity_sha256":artifact::identity(&recording.source).unwrap(),"experimental_payload_sha256":state.payload_sha256,"source_optimizer_step":recording.source.actor.optimizer_step,"predictive_optimizer_step":before_rebind.optimizer_step,"rebound_optimizer_step":before.optimizer_step,"native_optimizer_step":after.optimizer_step,"critic_optimizer_step":value.checkpoint().optimizer_step,"native_decisions":decisions.len(),"native_train_transitions":decisions.len()-1,"completed_epochs":receipt.completed_epochs,"original_source_unchanged":true,"original_experimental_state_unchanged":true,"auxiliary_head_in_native_state":false,"accepted_state_written":false,"behavior_gain_measured":false})
    );
}
