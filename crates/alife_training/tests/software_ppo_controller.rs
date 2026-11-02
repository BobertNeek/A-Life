//! Actual public PPO controller with explicit test-adapter selection.
#![cfg(feature = "gpu-tests")]

use std::{collections::BTreeMap, sync::Arc};

use alife_core::{
    BrainCapacityClass, BrainGenome, CandidateActionFamily, CompiledSynapseKind, DecoderHeadKind,
    DevelopmentState, FoundationWeightAsset, NormalizedScalar, PhenotypeCompiler,
    ScaffoldContractError, SensorProfile, Tick,
};
use alife_runtime::GpuAuthoritativeSession;
use alife_training::{
    train_recurrent_ppo_cohort, train_recurrent_ppo_cohort_with_frozen_gradients_bounded_for_test,
    train_recurrent_ppo_cohort_with_windows, AdamWConfig, FoundationTrainer,
    FoundationTrainerCheckpoint, FrozenGradientCacheStats, PpoBatch, PpoBoundary, PpoConfig,
    PpoJointAction, PpoTrainingState, PpoTrainingWindow, PpoTransition, PpoUpdateReceipt,
    PpoValueHeadCheckpoint, StageTrainableMask, TrainingError, TrainingInitialState,
    TrainingReplayCandidate, TrainingReplayTick, TrainingSequence,
};

#[path = "software_ppo_controller/execution.rs"]
mod execution;

fn fixture() -> (FoundationTrainer, usize, usize, usize) {
    fixture_with_wide_mask(false)
}

fn fixture_with_wide_mask(wide_mask: bool) -> (FoundationTrainer, usize, usize, usize) {
    fixture_with_session(wide_mask, None)
}

fn fixture_with_profile(
    profile: alife_gpu_backend::GpuRuntimeProfile,
) -> (FoundationTrainer, usize, usize, usize) {
    fixture_with_session(
        false,
        Some(execution::software_session_with_profile(profile).unwrap()),
    )
}

fn fixture_with_session(
    wide_mask: bool,
    session: Option<GpuAuthoritativeSession>,
) -> (FoundationTrainer, usize, usize, usize) {
    let genome = BrainGenome::scaffold(0x9900_2048, BrainCapacityClass::n2048().id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.8).unwrap());
    let native = PhenotypeCompiler::compile_testing_procedural_baseline(
        &genome,
        &BrainCapacityClass::n2048(),
        &development,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let source = FoundationWeightAsset::from_phenotype_for_genetic_birth(&native).unwrap();
    let (phenotype, _) =
        PhenotypeCompiler::compile_n2048_foundation_candidate(genome, development, source.clone())
            .unwrap();
    let (weight, neuron, lane) = phenotype
        .synapses()
        .iter()
        .enumerate()
        .find_map(|(index, synapse)| match synapse.kind() {
            CompiledSynapseKind::Decoder(coordinate)
                if coordinate.head() == DecoderHeadKind::ActionCandidate
                    && coordinate.family() == CandidateActionFamily::Approach =>
            {
                Some((
                    index,
                    synapse.source() as usize,
                    usize::from(coordinate.input_lane()),
                ))
            }
            _ => None,
        })
        .unwrap();
    let indices = if wide_mask {
        (0..phenotype.synapses().len())
            .step_by(3)
            .map(|index| index as u32)
            .collect::<Vec<_>>()
    } else {
        vec![weight as u32]
    };
    assert!(indices.contains(&(weight as u32)));
    let mask = StageTrainableMask::from_synapse_indices(&phenotype, &indices).unwrap();
    let session = session.unwrap_or_else(|| {
        execution::new_session().expect("explicit selected training fixture adapter unavailable")
    });
    eprintln!(
        "actual_controller_fixture neurons={}; synapses={}; trainable_weights={}; masked_weight={weight}",
        phenotype.neuron_count(),
        phenotype.synapses().len(),
        mask.trainable_count()
    );
    let trainer =
        FoundationTrainer::from_session(session, phenotype, source, mask, AdamWConfig::default())
            .unwrap();
    eprintln!(
        "actual_controller_execution_metadata={}",
        execution::metadata(&trainer)
    );
    assert!(trainer.replay_buffer_word_lengths().is_err());
    (trainer, weight, neuron, lane)
}

fn windows(
    trainer: &mut FoundationTrainer,
    state: &mut PpoTrainingState,
    neuron: usize,
    lane: usize,
    config: PpoConfig,
    policy: u64,
    old_logp_offset: f32,
) -> Vec<PpoTrainingWindow> {
    windows_with_count(
        trainer,
        state,
        neuron,
        lane,
        config,
        policy,
        old_logp_offset,
        9,
    )
}

#[allow(clippy::too_many_arguments)]
fn windows_with_count(
    trainer: &mut FoundationTrainer,
    state: &mut PpoTrainingState,
    neuron: usize,
    lane: usize,
    config: PpoConfig,
    policy: u64,
    old_logp_offset: f32,
    count: usize,
) -> Vec<PpoTrainingWindow> {
    let shapes = (0..count)
        .map(|index| if index % 2 == 0 { (3, 0) } else { (5, 1) })
        .collect::<Vec<_>>();
    windows_with_shapes(
        trainer,
        state,
        neuron,
        lane,
        config,
        policy,
        old_logp_offset,
        &shapes,
        false,
    )
}

#[allow(clippy::too_many_arguments)]
fn windows_with_shapes(
    trainer: &mut FoundationTrainer,
    state: &mut PpoTrainingState,
    neuron: usize,
    lane: usize,
    config: PpoConfig,
    policy: u64,
    old_logp_offset: f32,
    shapes: &[(usize, usize)],
    all_active: bool,
) -> Vec<PpoTrainingWindow> {
    shapes
        .iter()
        .copied()
        .enumerate()
        .map(|(index, (ticks, burn_in_ticks))| {
            let n = trainer.phenotype().neuron_count() as usize;
            let mut initial = vec![0.0; n];
            initial[neuron] = 0.4;
            let sequence = TrainingSequence {
                phenotype_hash: trainer.phenotype().phenotype_hash(),
                initial: TrainingInitialState {
                    activations: initial,
                    activity_ema: vec![0.0; n],
                    metabolic_load: vec![0.0; n],
                    dendrites: Default::default(),
                },
                ticks: (0..ticks)
                    .map(|tick| {
                        let mut inputs = vec![0.01; n];
                        inputs[neuron] = 0.5 + index as f32 * 0.01;
                        TrainingReplayTick {
                            encoded_inputs: inputs,
                            projection_gain: 0.9,
                            local_threshold_shift: 0.0,
                            microstep_count: if all_active || tick % 2 == 0 { 4 } else { 0 },
                            enabled_routes: vec![true; trainer.phenotype().projections().len()],
                            effective_weight_offsets: vec![
                                0.0;
                                trainer.phenotype().synapses().len()
                            ],
                            structural_synapses: Vec::new(),
                            candidates: [
                                (CandidateActionFamily::Approach, 1.0),
                                (CandidateActionFamily::Approach, -0.5),
                                (CandidateActionFamily::Idle, 0.0),
                                (CandidateActionFamily::Rest, 0.0),
                            ]
                            .into_iter()
                            .map(|(family, scale)| {
                                let mut decoder_inputs = [0.0; 54];
                                decoder_inputs[lane] = scale;
                                TrainingReplayCandidate {
                                    family,
                                    decoder_inputs,
                                    innate_bias: 0.0,
                                }
                            })
                            .collect(),
                        }
                    })
                    .collect(),
                burn_in_ticks,
                memory_candidate_gain: trainer
                    .phenotype()
                    .candidate_decoder()
                    .memory_channel()
                    .map_or(0.0, |p| p.max_candidate_gain()),
            };
            let evaluation = trainer.evaluate_replay(&sequence).unwrap();
            let values = state.predict_values(trainer, &sequence).unwrap();
            let transitions = evaluation.candidate_logits[burn_in_ticks..]
                .iter()
                .zip(&values[burn_in_ticks..])
                .enumerate()
                .map(|(step, (logits, value))| {
                    let maximum = logits[..3]
                        .iter()
                        .copied()
                        .fold(f32::NEG_INFINITY, f32::max);
                    let p = logits[..3]
                        .iter()
                        .map(|v| (f64::from(*v) - f64::from(maximum)).exp())
                        .collect::<Vec<_>>();
                    let z = p.iter().sum::<f64>();
                    let old_logp = ((p[2] / z) * p[0] / (p[0] + p[1])).ln() as f32;
                    PpoTransition {
                        policy_version: policy,
                        trajectory_id: (index * ticks.max(8) + step) as u64,
                        step: 0,
                        action: PpoJointAction {
                            candidate_count: 4,
                            representative_mask: 7,
                            motor_masks: [3, 0, 0, 0, 0, 0],
                            representative: 2,
                            forced_slots: vec![Some(0), Some(0), Some(4), Some(4)],
                            motor_candidates: [Some(0), None, None, None, Some(2), None],
                            old_joint_log_probability: old_logp + old_logp_offset,
                            temperature: 1.0,
                        },
                        reward: 0.8 + index as f32 * 0.03 + step as f32 * 0.01,
                        old_value: *value,
                        next_value: 0.0,
                        elapsed_seconds: 0.05,
                        boundary: PpoBoundary::Terminated,
                    }
                })
                .collect();
            PpoTrainingWindow {
                speech_targets: vec![None; ticks],
                sequence,
                batch: PpoBatch::from_rollout(policy, transitions, config).unwrap(),
                auxiliary: None,
            }
        })
        .collect()
}

#[path = "software_ppo_controller/admission_qualification.rs"]
mod admission_qualification;
#[path = "software_ppo_controller/continuing_qualification.rs"]
mod continuing_qualification;
#[path = "software_ppo_controller/fallback_qualification.rs"]
mod fallback_qualification;
#[path = "software_ppo_controller/frozen_gradient_assay.rs"]
mod frozen_gradient_assay;

fn bits(values: &[f32]) -> Vec<u32> {
    values.iter().map(|v| v.to_bits()).collect()
}

fn read_words(
    trainer: &FoundationTrainer,
    source: &wgpu::Buffer,
    range: std::ops::Range<u64>,
) -> Vec<u32> {
    let (device, queue) = trainer
        .session()
        .backend()
        .offline_training_device_queue()
        .unwrap();
    let bytes = range.end - range.start;
    assert!(bytes > 0 && range.end <= source.size());
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("software-controller-semantic-capture"),
        size: bytes,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(source, range.start, &staging, 0, bytes);
    let command = encoder.finish();
    let (sender, receiver) = std::sync::mpsc::channel();
    command.map_buffer_on_submit(&staging, wgpu::MapMode::Read, 0..bytes, move |result| {
        sender.send(result).unwrap();
    });
    let submission = queue.submit(Some(command));
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })
        .unwrap();
    receiver.recv().unwrap().unwrap();
    let mapped = staging.slice(..bytes).get_mapped_range();
    let result = bytemuck::cast_slice::<u8, u32>(&mapped).to_vec();
    drop(mapped);
    staging.unmap();
    result
}

fn capture(
    trainer: &FoundationTrainer,
    state: &mut PpoTrainingState,
) -> BTreeMap<String, Vec<u32>> {
    let actor = trainer.checkpoint().unwrap();
    let value = state.checkpoint(trainer.session()).unwrap();
    let (output_words, state_words) = trainer.replay_buffer_word_lengths().unwrap();
    let mut result = BTreeMap::from([
        ("weights".into(), bits(&actor.weights)),
        ("first_moment".into(), bits(&actor.first_moment)),
        ("second_moment".into(), bits(&actor.second_moment)),
        ("update_ages".into(), actor.update_ages),
        ("value_parameters_moments".into(), bits(&value.parameters)),
        (
            "optimizer_steps".into(),
            vec![actor.optimizer_step, value.optimizer_step],
        ),
        (
            "logical_outputs".into(),
            read_words(trainer, trainer.replay_output_buffer(), 0..output_words * 4),
        ),
        (
            "logical_state".into(),
            read_words(trainer, trainer.replay_state_buffer(), 0..state_words * 4),
        ),
    ]);
    let policy = value.last_updated_policy_version.unwrap();
    result.insert(
        "last_policy_version".into(),
        vec![policy as u32, (policy >> 32) as u32],
    );
    let objective = state.objective_mut().unwrap();
    result.insert(
        "value_all_five_planes".into(),
        read_words(
            trainer,
            objective.value_head_buffer(),
            0..objective.value_head_buffer().size(),
        ),
    );
    result.insert(
        "objective_adjoints".into(),
        read_words(
            trainer,
            objective.output_buffer(),
            objective.adjoint_bytes(),
        ),
    );
    result.insert(
        "objective_metrics".into(),
        read_words(trainer, objective.output_buffer(), objective.metric_bytes()),
    );
    result.insert(
        "objective_values".into(),
        read_words(trainer, objective.output_buffer(), objective.value_bytes()),
    );
    result.insert(
        "objective_preflight".into(),
        read_words(
            trainer,
            objective.output_buffer(),
            objective.value_preflight_bytes(),
        ),
    );
    result
}

fn progress(
    before: &FoundationTrainerCheckpoint,
    after: &FoundationTrainerCheckpoint,
    weight: usize,
) {
    progress_steps(before, after, weight, 2);
}

fn progress_steps(
    before: &FoundationTrainerCheckpoint,
    after: &FoundationTrainerCheckpoint,
    weight: usize,
    updates: u32,
) {
    assert_eq!(after.optimizer_step, before.optimizer_step + updates);
    let mut changed = 0;
    for index in 0..before.weights.len() {
        if index == weight {
            changed +=
                usize::from(before.weights[index].to_bits() != after.weights[index].to_bits());
            assert_eq!(
                after.update_ages[index],
                before.update_ages[index] + updates
            );
            assert!(after.first_moment[index].is_finite() && after.first_moment[index] != 0.0);
            assert!(after.second_moment[index].is_finite() && after.second_moment[index] > 0.0);
        } else {
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
    assert_eq!(changed, 1);
}

fn assert_actor_equal(
    expected: &FoundationTrainerCheckpoint,
    actual: &FoundationTrainerCheckpoint,
) {
    assert_eq!(expected.schema_version, actual.schema_version);
    assert_eq!(expected.phenotype_hash, actual.phenotype_hash);
    assert_eq!(
        expected.source_foundation_digest,
        actual.source_foundation_digest
    );
    assert_eq!(expected.optimizer_step, actual.optimizer_step);
    assert_eq!(expected.stage_mask, actual.stage_mask);
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
    assert_eq!(config_words(expected.config), config_words(actual.config));
    assert_eq!(bits(&expected.weights), bits(&actual.weights));
    assert_eq!(bits(&expected.first_moment), bits(&actual.first_moment));
    assert_eq!(bits(&expected.second_moment), bits(&actual.second_moment));
    assert_eq!(expected.update_ages, actual.update_ages);
}

fn assert_value_equal(expected: &PpoValueHeadCheckpoint, actual: &PpoValueHeadCheckpoint) {
    assert_eq!(expected.feature_count, actual.feature_count);
    assert_eq!(expected.optimizer_step, actual.optimizer_step);
    assert_eq!(
        expected.last_updated_policy_version,
        actual.last_updated_policy_version
    );
    assert_eq!(bits(&expected.parameters), bits(&actual.parameters));
}

fn assert_persistent_equal(
    actor: &FoundationTrainerCheckpoint,
    value: &PpoValueHeadCheckpoint,
    trainer: &FoundationTrainer,
    state: &mut PpoTrainingState,
) {
    assert_actor_equal(actor, &trainer.checkpoint().unwrap());
    assert_value_equal(value, &state.checkpoint(trainer.session()).unwrap());
}

fn receipt_words(receipt: &PpoUpdateReceipt) -> Vec<u32> {
    let mut words = vec![
        receipt.collection_policy_version as u32,
        (receipt.collection_policy_version >> 32) as u32,
        receipt.completed_epochs,
        u32::from(receipt.stopped_for_kl),
        receipt.actor_optimizer_step,
        receipt.value_optimizer_step,
    ];
    words.extend(bits(&receipt.mean_kl_before_updates));
    words
}

fn window_words(windows: &[PpoTrainingWindow]) -> Vec<u32> {
    let mut result = Vec::new();
    for window in windows {
        result.extend([
            window.sequence.ticks.len() as u32,
            window.sequence.burn_in_ticks as u32,
            window.batch.len() as u32,
        ]);
        for transition in window.batch.transitions() {
            result.extend([
                transition.action.old_joint_log_probability.to_bits(),
                transition.old_value.to_bits(),
                transition.reward.to_bits(),
            ]);
        }
        result.extend(bits(window.batch.advantages()));
        result.extend(bits(window.batch.returns()));
    }
    result
}

fn assert_semantics_equal(
    expected: &BTreeMap<String, Vec<u32>>,
    actual: &BTreeMap<String, Vec<u32>>,
) {
    assert_eq!(
        expected.keys().collect::<Vec<_>>(),
        actual.keys().collect::<Vec<_>>()
    );
    for (category, left) in expected {
        let right = &actual[category];
        assert_eq!(left.len(), right.len(), "{category} length");
        if let Some((index, (left, right))) = left
            .iter()
            .zip(right)
            .enumerate()
            .find(|(_, (a, b))| a != b)
        {
            panic!("{category} word {index}: {left:#010x} != {right:#010x}");
        }
    }
}

#[test]
#[ignore = "explicit synthetic full PPO controller oracle; inspect actual adapter identity"]
fn actual_controller_batches_restores_and_rejects_before_updates() {
    let mut artifact = BTreeMap::new();
    for value_coefficient in [0.0, 0.5] {
        let (mut trainer, weight, neuron, lane) = fixture();
        let mut state = PpoTrainingState::default();
        let config = PpoConfig {
            epochs: 1,
            target_kl: 100.0,
            entropy_coefficient: 0.0,
            value_coefficient,
            ..Default::default()
        };
        let initial_actor = trainer.checkpoint().unwrap();
        let first = windows(&mut trainer, &mut state, neuron, lane, config, 7, 0.0);
        assert_eq!(
            first
                .iter()
                .map(|window| window.batch.len())
                .collect::<Vec<_>>(),
            [3, 4, 3, 4, 3, 4, 3, 4, 3]
        );
        artifact.insert(
            format!("{value_coefficient}/first_windows"),
            BTreeMap::from([("immutable_collection".into(), window_words(&first))]),
        );
        let initial_value = state.checkpoint(trainer.session()).unwrap();
        let mut stale = first.clone();
        let mut transitions = stale[8].batch.transitions().to_vec();
        transitions[0].action.old_joint_log_probability -= 0.25;
        stale[8].batch = PpoBatch::from_rollout(7, transitions, config).unwrap();
        let mut loads = Vec::new();
        let rejected = train_recurrent_ppo_cohort(
            &mut trainer,
            &mut state,
            9,
            8,
            |index| {
                loads.push(index);
                Ok(stale[index].clone())
            },
            config,
            7,
        );
        assert!(matches!(
            rejected,
            Err(TrainingError::Contract(
                ScaffoldContractError::InvalidDecisionEvidence
            ))
        ));
        assert_eq!(loads, (0..9).collect::<Vec<_>>());
        assert_persistent_equal(&initial_actor, &initial_value, &trainer, &mut state);

        let receipt = train_recurrent_ppo_cohort(
            &mut trainer,
            &mut state,
            9,
            8,
            |index| Ok(first[index].clone()),
            config,
            7,
        )
        .unwrap();
        assert_eq!(
            (receipt.completed_epochs, receipt.actor_optimizer_step),
            (1, 2)
        );
        assert!(!receipt.stopped_for_kl);
        assert_eq!(receipt.mean_kl_before_updates.len(), 2);
        assert_eq!(
            receipt.value_optimizer_step,
            if value_coefficient > 0.0 { 2 } else { 0 }
        );
        let boundary_actor = trainer.checkpoint().unwrap();
        let boundary_value = state.checkpoint(trainer.session()).unwrap();
        progress(&initial_actor, &boundary_actor, weight);
        if value_coefficient == 0.0 {
            assert_eq!(
                bits(&initial_value.parameters),
                bits(&boundary_value.parameters)
            );
        } else {
            assert_ne!(
                bits(&initial_value.parameters),
                bits(&boundary_value.parameters)
            );
        }
        artifact.insert(
            format!("{value_coefficient}/first"),
            capture(&trainer, &mut state),
        );
        artifact.insert(
            format!("{value_coefficient}/first_receipt"),
            BTreeMap::from([("receipt".into(), receipt_words(&receipt))]),
        );

        let next = windows(&mut trainer, &mut state, neuron, lane, config, 8, 0.0);
        artifact.insert(
            format!("{value_coefficient}/continued_windows"),
            BTreeMap::from([("immutable_collection".into(), window_words(&next))]),
        );
        let natural = train_recurrent_ppo_cohort(
            &mut trainer,
            &mut state,
            9,
            8,
            |index| Ok(next[index].clone()),
            config,
            8,
        )
        .unwrap();
        progress(&boundary_actor, &trainer.checkpoint().unwrap(), weight);
        let continued = capture(&trainer, &mut state);
        trainer.restore_checkpoint(&boundary_actor).unwrap();
        state = PpoTrainingState::from_checkpoint(boundary_value).unwrap();
        let shared = next.into_iter().map(Arc::new).collect::<Vec<_>>();
        let restored = train_recurrent_ppo_cohort_with_windows(
            &mut trainer,
            &mut state,
            9,
            8,
            |index| Ok(Arc::clone(&shared[index])),
            config,
            8,
        )
        .unwrap();
        assert_eq!(receipt_words(&natural), receipt_words(&restored));
        assert_semantics_equal(&continued, &capture(&trainer, &mut state));
        artifact.insert(format!("{value_coefficient}/continued"), continued);
        artifact.insert(
            format!("{value_coefficient}/continued_receipt"),
            BTreeMap::from([("receipt".into(), receipt_words(&restored))]),
        );

        let stopped_actor = trainer.checkpoint().unwrap();
        let stopped_value = state.checkpoint(trainer.session()).unwrap();
        let gated_config = PpoConfig {
            target_kl: 1.0e-10,
            ..config
        };
        let gated = windows(
            &mut trainer,
            &mut state,
            neuron,
            lane,
            gated_config,
            9,
            -0.0005,
        );
        artifact.insert(
            format!("{value_coefficient}/gated_windows"),
            BTreeMap::from([("immutable_collection".into(), window_words(&gated))]),
        );
        let mut loads = Vec::new();
        let stopped = train_recurrent_ppo_cohort(
            &mut trainer,
            &mut state,
            9,
            8,
            |index| {
                loads.push(index);
                Ok(gated[index].clone())
            },
            gated_config,
            9,
        )
        .unwrap();
        assert!(stopped.stopped_for_kl);
        assert_eq!(stopped.completed_epochs, 0);
        assert_eq!(stopped.mean_kl_before_updates.len(), 1);
        assert!(stopped.mean_kl_before_updates[0] > gated_config.target_kl);
        assert_eq!(loads, (0..9).chain(0..8).collect::<Vec<_>>());
        assert_persistent_equal(&stopped_actor, &stopped_value, &trainer, &mut state);
        artifact.insert(
            format!("{value_coefficient}/kl_stop"),
            capture(&trainer, &mut state),
        );
        artifact.insert(
            format!("{value_coefficient}/kl_receipt"),
            BTreeMap::from([("receipt".into(), receipt_words(&stopped))]),
        );
        eprintln!("full PPO value_coefficient={value_coefficient}: 9 unequal windows (3/4 loss rows), effective batch8, final actor optimizer_step=4, changed masked weight, exact restore, late frozen rejection and KL cancellation passed");
    }
    if let Some(directory) = std::env::var_os("ALIFE_SOFTWARE_PPO_CAPTURE_DIR") {
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            std::path::Path::new(&directory).join("semantic-words.json"),
            serde_json::to_vec(&artifact).unwrap(),
        )
        .unwrap();
    }
}

fn replay_tick_bound<W: std::borrow::Borrow<PpoTrainingWindow>>(windows: &[W]) -> usize {
    windows
        .iter()
        .map(|window| window.borrow().sequence.ticks.len())
        .max()
        .unwrap_or(0)
}

fn run_owned(
    trainer: &mut FoundationTrainer,
    state: &mut PpoTrainingState,
    windows: &[PpoTrainingWindow],
    config: PpoConfig,
    policy: u64,
    cached: bool,
) -> (
    Result<PpoUpdateReceipt, TrainingError>,
    FrozenGradientCacheStats,
    Vec<usize>,
) {
    let mut stats = FrozenGradientCacheStats::default();
    let mut loads = Vec::new();
    let load = |index: usize| {
        loads.push(index);
        Ok(windows[index].clone())
    };
    let result = if cached {
        train_recurrent_ppo_cohort_with_frozen_gradients_bounded_for_test(
            trainer,
            state,
            windows.len(),
            8,
            load,
            config,
            policy,
            replay_tick_bound(windows),
            &mut stats,
        )
    } else {
        train_recurrent_ppo_cohort(trainer, state, windows.len(), 8, load, config, policy)
    };
    (result, stats, loads)
}

fn assert_active_cache(stats: FrozenGradientCacheStats) {
    assert!(stats.eligible);
    assert!(stats.requested_gpu_bytes > 0 && stats.requested_gpu_bytes <= 2 * 1024 * 1024);
    assert_eq!(stats.raw_cached_windows, 2);
    // Only the first frozen epoch is reused, even when later epochs run.
    assert_eq!(stats.reused_windows, 2);
}

fn warm_pair(
    trainer: &mut FoundationTrainer,
    state: &mut PpoTrainingState,
    weight: usize,
    neuron: usize,
    lane: usize,
    config: PpoConfig,
) -> (FoundationTrainerCheckpoint, PpoValueHeadCheckpoint) {
    // Seed a real residual critic even when the subsequent PPO update freezes it.
    let config = PpoConfig {
        epochs: 1,
        value_coefficient: 0.5,
        ..config
    };
    let initial = trainer.checkpoint().unwrap();
    let warm = windows_with_count(trainer, state, neuron, lane, config, 5, 0.0, 2);
    let (receipt, _, _) = run_owned(trainer, state, &warm, config, 5, false);
    assert_eq!(receipt.unwrap().completed_epochs, 1);
    let actor = trainer.checkpoint().unwrap();
    progress_steps(&initial, &actor, weight, 1);
    let value = state.checkpoint(trainer.session()).unwrap();
    assert_eq!(value.optimizer_step, 1);
    let width = value.feature_count as usize + 1;
    assert!(value.parameters[width..width * 2].iter().any(|&v| v != 0.0));
    assert!(value.parameters[width * 2..].iter().any(|&v| v > 0.0));
    (actor, value)
}

#[test]
#[ignore = "explicit synthetic frozen-gradient full PPO oracle; inspect actual adapter identity"]
fn frozen_gradients_match_actual_controller_epochs_and_shared_restore() {
    for value_coefficient in [0.0, 0.5] {
        for epochs in [1, 2] {
            let (mut trainer, weight, neuron, lane) = fixture();
            let mut state = PpoTrainingState::default();
            let config = PpoConfig {
                epochs,
                target_kl: 100.0,
                entropy_coefficient: 0.0,
                value_coefficient,
                ..Default::default()
            };
            let (before_actor, before_value) =
                warm_pair(&mut trainer, &mut state, weight, neuron, lane, config);
            let collected =
                windows_with_count(&mut trainer, &mut state, neuron, lane, config, 6, 0.0, 2);
            assert_eq!(
                collected.iter().map(|w| w.batch.len()).collect::<Vec<_>>(),
                [3, 4]
            );
            let (natural, _, natural_loads) =
                run_owned(&mut trainer, &mut state, &collected, config, 6, false);
            let natural = natural.unwrap();
            assert_eq!(
                natural_loads,
                (0..=epochs).flat_map(|_| 0..2).collect::<Vec<_>>()
            );
            assert_eq!(natural.completed_epochs, epochs);
            assert!(!natural.stopped_for_kl);
            assert_eq!(
                natural.actor_optimizer_step,
                before_actor.optimizer_step + epochs
            );
            assert_eq!(natural.mean_kl_before_updates.len(), epochs as usize);
            let after_actor = trainer.checkpoint().unwrap();
            let after_value = state.checkpoint(trainer.session()).unwrap();
            progress_steps(&before_actor, &after_actor, weight, epochs);
            assert_eq!(
                after_value.optimizer_step,
                before_value.optimizer_step + if value_coefficient > 0.0 { epochs } else { 0 }
            );
            if value_coefficient == 0.0 {
                assert_eq!(
                    bits(&before_value.parameters),
                    bits(&after_value.parameters)
                );
            } else {
                assert_ne!(
                    bits(&before_value.parameters),
                    bits(&after_value.parameters)
                );
            }
            let expected = capture(&trainer, &mut state);

            trainer.restore_checkpoint(&before_actor).unwrap();
            state = PpoTrainingState::from_checkpoint(before_value.clone()).unwrap();
            let (cached, stats, cached_loads) =
                run_owned(&mut trainer, &mut state, &collected, config, 6, true);
            assert_active_cache(stats);
            assert_eq!(natural_loads, cached_loads);
            assert_eq!(receipt_words(&natural), receipt_words(&cached.unwrap()));
            assert_actor_equal(&after_actor, &trainer.checkpoint().unwrap());
            assert_value_equal(&after_value, &state.checkpoint(trainer.session()).unwrap());
            assert_semantics_equal(&expected, &capture(&trainer, &mut state));

            trainer.restore_checkpoint(&before_actor).unwrap();
            state = PpoTrainingState::from_checkpoint(before_value).unwrap();
            let shared = collected.into_iter().map(Arc::new).collect::<Vec<_>>();
            let mut stats = FrozenGradientCacheStats::default();
            let mut shared_loads = Vec::new();
            let shared_receipt = train_recurrent_ppo_cohort_with_frozen_gradients_bounded_for_test(
                &mut trainer,
                &mut state,
                shared.len(),
                8,
                |index| {
                    shared_loads.push(index);
                    Ok(Arc::clone(&shared[index]))
                },
                config,
                6,
                replay_tick_bound(&shared),
                &mut stats,
            )
            .unwrap();
            assert_active_cache(stats);
            assert_eq!(natural_loads, shared_loads);
            assert_eq!(receipt_words(&natural), receipt_words(&shared_receipt));
            assert_actor_equal(&after_actor, &trainer.checkpoint().unwrap());
            assert_value_equal(&after_value, &state.checkpoint(trainer.session()).unwrap());
            assert_semantics_equal(&expected, &capture(&trainer, &mut state));
            eprintln!("frozen-gradient full PPO oracle: epochs={epochs}, value_coefficient={value_coefficient}, loss_rows=3+4, raw_cached=2, reused=2, later_epochs_fresh, owned/shared checkpoint restore and all captured words exact");
        }
    }
}

#[test]
#[ignore = "explicit synthetic KL cancellation and restart oracle"]
fn frozen_gradients_preserve_kl_cancellation_and_restart() {
    let (mut trainer, weight, neuron, lane) = fixture();
    let mut state = PpoTrainingState::default();
    let config = PpoConfig {
        epochs: 1,
        target_kl: 100.0,
        entropy_coefficient: 0.0,
        ..Default::default()
    };
    let (before_actor, before_value) =
        warm_pair(&mut trainer, &mut state, weight, neuron, lane, config);
    let gated_config = PpoConfig {
        target_kl: 1.0e-10,
        ..config
    };
    let gated = windows_with_count(
        &mut trainer,
        &mut state,
        neuron,
        lane,
        gated_config,
        6,
        -0.0005,
        2,
    );
    let restart = windows_with_count(&mut trainer, &mut state, neuron, lane, config, 6, 0.0, 2);
    let mut expected_stopped = None;
    let mut expected_restart = None;
    for cached in [false, true] {
        trainer.restore_checkpoint(&before_actor).unwrap();
        state = PpoTrainingState::from_checkpoint(before_value.clone()).unwrap();
        let (stopped, stats, loads) =
            run_owned(&mut trainer, &mut state, &gated, gated_config, 6, cached);
        let stopped = stopped.unwrap();
        assert!(stopped.stopped_for_kl);
        assert_eq!(stopped.completed_epochs, 0);
        assert_eq!(stopped.mean_kl_before_updates.len(), 1);
        assert!(stopped.mean_kl_before_updates[0] > gated_config.target_kl);
        assert_eq!(loads, [0, 1, 0, 1]);
        assert_persistent_equal(&before_actor, &before_value, &trainer, &mut state);
        if cached {
            assert_active_cache(stats);
        }
        let stopped_capture = capture(&trainer, &mut state);
        if let Some((receipt, words)) = &expected_stopped {
            assert_eq!(*receipt, receipt_words(&stopped));
            assert_semantics_equal(words, &stopped_capture);
        } else {
            expected_stopped = Some((receipt_words(&stopped), stopped_capture));
        }
        // Both the controller's cancellation and the public fresh begin/cancel
        // boundary must leave the next accepted update usable.
        trainer.begin_gradient_accumulation(1).unwrap();
        trainer.cancel_gradient_accumulation();
        let (receipt, stats, loads) =
            run_owned(&mut trainer, &mut state, &restart, config, 6, cached);
        let receipt = receipt.unwrap();
        assert_eq!(loads, [0, 1, 0, 1]);
        assert!(!receipt.stopped_for_kl);
        assert_eq!(receipt.completed_epochs, 1);
        progress_steps(&before_actor, &trainer.checkpoint().unwrap(), weight, 1);
        if cached {
            assert_active_cache(stats);
        }
        let restart_capture = capture(&trainer, &mut state);
        if let Some((expected_receipt, words)) = &expected_restart {
            assert_eq!(*expected_receipt, receipt_words(&receipt));
            assert_semantics_equal(words, &restart_capture);
        } else {
            expected_restart = Some((receipt_words(&receipt), restart_capture));
        }
    }
    eprintln!("frozen-gradient KL stop leaves persistent bits unchanged; explicit begin/cancel and accepted restart match original controller");
}

#[test]
#[ignore = "explicit synthetic frozen-validation and gradient-failure ordering oracle"]
fn frozen_gradients_defer_numeric_faults_and_preserve_loader_failures() {
    let (mut trainer, weight, neuron, lane) = fixture();
    let mut state = PpoTrainingState::default();
    let config = PpoConfig {
        epochs: 1,
        target_kl: 100.0,
        entropy_coefficient: 0.0,
        value_coefficient: 0.0,
        ..Default::default()
    };
    let (before_actor, before_value) =
        warm_pair(&mut trainer, &mut state, weight, neuron, lane, config);
    let collected = windows_with_count(&mut trainer, &mut state, neuron, lane, config, 6, 0.0, 2);
    let mut overflow = collected.clone();
    let mut transitions = overflow[0].batch.transitions().to_vec();
    for transition in &mut transitions {
        transition.reward = 1.0e30;
    }
    overflow[0].batch = PpoBatch::from_rollout(6, transitions, config).unwrap();
    let mut stale = overflow.clone();
    let mut transitions = stale[1].batch.transitions().to_vec();
    transitions[0].action.old_joint_log_probability -= 0.25;
    stale[1].batch = PpoBatch::from_rollout(6, transitions, config).unwrap();
    for cached in [false, true] {
        for late_stale in [true, false] {
            trainer.restore_checkpoint(&before_actor).unwrap();
            state = PpoTrainingState::from_checkpoint(before_value.clone()).unwrap();
            let (result, stats, loads) = run_owned(
                &mut trainer,
                &mut state,
                if late_stale { &stale } else { &overflow },
                config,
                6,
                cached,
            );
            if late_stale {
                assert!(matches!(
                    result,
                    Err(TrainingError::Contract(
                        ScaffoldContractError::InvalidDecisionEvidence
                    ))
                ));
                assert_eq!(loads, [0, 1]);
                if cached {
                    assert!(stats.eligible);
                    assert_eq!(stats.raw_cached_windows, 1);
                    assert_eq!(stats.reused_windows, 0);
                }
            } else {
                // This control proves the earlier raw gradient really is bad;
                // its finite scalar policy loss must survive the frozen pass.
                assert!(matches!(result, Err(TrainingError::MalformedReadback)));
                assert_eq!(loads, [0, 1, 0]);
            }
            // Scratch is allowed to differ on failure. Persistent state is not.
            assert_persistent_equal(&before_actor, &before_value, &trainer, &mut state);
            trainer.begin_gradient_accumulation(1).unwrap();
            trainer.cancel_gradient_accumulation();
        }

        trainer.restore_checkpoint(&before_actor).unwrap();
        state = PpoTrainingState::from_checkpoint(before_value.clone()).unwrap();
        let mut calls = [0; 2];
        let mut loads = Vec::new();
        let mut stats = FrozenGradientCacheStats::default();
        let load = |index: usize| {
            loads.push(index);
            calls[index] += 1;
            if index == 1 && calls[index] == 2 {
                Err(TrainingError::MalformedReadback)
            } else {
                Ok(collected[index].clone())
            }
        };
        let result = if cached {
            train_recurrent_ppo_cohort_with_frozen_gradients_bounded_for_test(
                &mut trainer,
                &mut state,
                2,
                8,
                load,
                config,
                6,
                replay_tick_bound(&collected),
                &mut stats,
            )
        } else {
            train_recurrent_ppo_cohort(&mut trainer, &mut state, 2, 8, load, config, 6)
        };
        assert!(matches!(result, Err(TrainingError::MalformedReadback)));
        assert_eq!(loads, [0, 1, 0, 1]);
        assert_eq!(calls, [2, 2]);
        if cached {
            assert!(stats.eligible);
            assert_eq!(stats.raw_cached_windows, 2);
            assert_eq!(stats.reused_windows, 1);
        }
        assert_persistent_equal(&before_actor, &before_value, &trainer, &mut state);
        trainer.begin_gradient_accumulation(1).unwrap();
        trainer.cancel_gradient_accumulation();
    }
    eprintln!("frozen-gradient faults: late old-probability rejection precedes earlier raw-norm overflow; valid-probability overflow control and fallible repeated loader preserve persistent bits and cancellation");
}

#[test]
#[ignore = "explicit synthetic cohort-size fallback oracle"]
fn frozen_gradients_fall_back_for_more_than_one_effective_batch() {
    let (mut trainer, weight, neuron, lane) = fixture();
    let mut state = PpoTrainingState::default();
    let config = PpoConfig {
        epochs: 1,
        target_kl: 100.0,
        entropy_coefficient: 0.0,
        ..Default::default()
    };
    let before_actor = trainer.checkpoint().unwrap();
    let collected = windows(&mut trainer, &mut state, neuron, lane, config, 7, 0.0);
    let before_value = state.checkpoint(trainer.session()).unwrap();
    let (normal, _, normal_loads) =
        run_owned(&mut trainer, &mut state, &collected, config, 7, false);
    let normal = normal.unwrap();
    assert_eq!(normal_loads, (0..9).chain(0..9).collect::<Vec<_>>());
    progress(&before_actor, &trainer.checkpoint().unwrap(), weight);
    let expected = capture(&trainer, &mut state);
    trainer.restore_checkpoint(&before_actor).unwrap();
    state = PpoTrainingState::from_checkpoint(before_value).unwrap();
    let (fallback, stats, fallback_loads) =
        run_owned(&mut trainer, &mut state, &collected, config, 7, true);
    assert!(!stats.eligible);
    assert_eq!(stats.requested_gpu_bytes, 0);
    assert_eq!(stats.raw_cached_windows, 0);
    assert_eq!(stats.reused_windows, 0);
    assert_eq!(normal_loads, fallback_loads);
    assert_eq!(receipt_words(&normal), receipt_words(&fallback.unwrap()));
    assert_semantics_equal(&expected, &capture(&trainer, &mut state));
    eprintln!("frozen-gradient >8-window fallback matches actual original controller with no cache allocation or reuse");
}
