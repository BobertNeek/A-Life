//! Actual public PPO controller on explicit software Vulkan, never hardware evidence.
#![cfg(feature = "gpu-tests")]

use std::{collections::BTreeMap, sync::Arc};

use alife_core::{
    BrainCapacityClass, BrainGenome, CandidateActionFamily, CompiledSynapseKind, DecoderHeadKind,
    DevelopmentState, FoundationWeightAsset, NormalizedScalar, PhenotypeCompiler,
    ScaffoldContractError, SensorProfile, Tick,
};
use alife_runtime::GpuAuthoritativeSession;
use alife_training::{
    train_recurrent_ppo_cohort, train_recurrent_ppo_cohort_with_windows, AdamWConfig,
    FoundationTrainer, FoundationTrainerCheckpoint, PpoBatch, PpoBoundary, PpoConfig,
    PpoJointAction, PpoTrainingState, PpoTrainingWindow, PpoTransition, PpoUpdateReceipt,
    StageTrainableMask, TrainingError, TrainingInitialState, TrainingReplayCandidate,
    TrainingReplayTick, TrainingSequence,
};

fn fixture() -> (FoundationTrainer, usize, usize, usize) {
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
    let mask = StageTrainableMask::from_synapse_indices(&phenotype, &[weight as u32]).unwrap();
    let session = GpuAuthoritativeSession::new_software_vulkan_training_test().unwrap();
    assert!(session.backend().is_software_training_test());
    let trainer =
        FoundationTrainer::from_session(session, phenotype, source, mask, AdamWConfig::default())
            .unwrap();
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
    (0..9)
        .map(|index| {
            let (ticks, burn_in_ticks) = if index % 2 == 0 { (3, 0) } else { (5, 1) };
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
                            microstep_count: if tick % 2 == 0 { 4 } else { 0 },
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
                        trajectory_id: (index * 8 + step) as u64,
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
    assert_eq!(after.optimizer_step, before.optimizer_step + 2);
    let mut changed = 0;
    for index in 0..before.weights.len() {
        if index == weight {
            changed +=
                usize::from(before.weights[index].to_bits() != after.weights[index].to_bits());
            assert_eq!(after.update_ages[index], before.update_ages[index] + 2);
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

#[test]
#[ignore = "explicit synthetic software Vulkan full PPO controller oracle; no hardware or throughput claim"]
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
        assert_eq!(trainer.checkpoint().unwrap(), initial_actor);
        assert_eq!(state.checkpoint(trainer.session()).unwrap(), initial_value);

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
        assert_eq!(continued, capture(&trainer, &mut state), "all logical words/checkpoints must survive restore and owned/shared window loading exactly");
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
        assert_eq!(trainer.checkpoint().unwrap(), stopped_actor);
        assert_eq!(state.checkpoint(trainer.session()).unwrap(), stopped_value);
        artifact.insert(
            format!("{value_coefficient}/kl_stop"),
            capture(&trainer, &mut state),
        );
        artifact.insert(
            format!("{value_coefficient}/kl_receipt"),
            BTreeMap::from([("receipt".into(), receipt_words(&stopped))]),
        );
        eprintln!("software full PPO value_coefficient={value_coefficient}: 9 unequal windows (3/4 loss rows), effective batch8, final actor optimizer_step=4, changed masked weight, exact restore, late frozen rejection and KL cancellation passed");
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
