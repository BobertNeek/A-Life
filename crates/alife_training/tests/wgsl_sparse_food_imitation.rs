//! Numerical qualification only; these synthetic logits do not prove a meal or gameplay.
#![cfg(all(feature = "gpu-tests", feature = "cpu-backend"))]

use alife_core::{
    BrainCapacityClass, BrainGenome, CandidateActionFamily, CompiledSynapseKind, DevelopmentState,
    FoundationWeightAsset, NormalizedScalar, PhenotypeCompiler, SensorProfile, Tick,
};
use alife_training::cpu::{
    cpu_imitation_objective, cpu_ppo_objective, train_cpu_ppo_cohort, CpuFoundationTrainer,
    CpuValueHead,
};
use alife_training::{
    imitation_mean_loss, train_recurrent_ppo_cohort, AdamWConfig, FoundationTrainer,
    ImitationExample, ImitationTarget, PpoBatch, PpoBoundary, PpoConfig, PpoGpuObjective,
    PpoJointAction, PpoReplayRow, PpoTrainingState, PpoTrainingWindow, PpoTransition,
    PpoValueHeadCheckpoint, StageTrainableMask, TrainingInitialState, TrainingReplayCandidate,
    TrainingReplayTick, TrainingSequence,
};

fn near(actual: f32, expected: f32, absolute: f32, relative: f32) {
    assert!(actual.is_finite() && expected.is_finite());
    assert!(
        (actual - expected).abs() <= absolute + relative * actual.abs().max(expected.abs()),
        "GPU {actual} != CPU {expected}"
    );
}

fn read(
    session: &alife_runtime::GpuAuthoritativeSession,
    source: &wgpu::Buffer,
    range: std::ops::Range<u64>,
) -> Vec<f32> {
    let (device, queue) = session.backend().offline_training_device_queue().unwrap();
    let size = range.end - range.start;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("sparse-imitation-qualification-readback"),
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(source, range.start, &buffer, 0, size);
    let commands = encoder.finish();
    let (sender, receiver) = std::sync::mpsc::channel();
    commands.map_buffer_on_submit(&buffer, wgpu::MapMode::Read, 0..size, move |result| {
        sender.send(result).unwrap();
    });
    let submission = queue.submit(Some(commands));
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(std::time::Duration::from_secs(60)),
        })
        .unwrap();
    receiver.recv().unwrap().unwrap();
    let mapped = buffer.slice(..size).get_mapped_range();
    let values = bytemuck::cast_slice::<u8, f32>(&mapped).to_vec();
    drop(mapped);
    buffer.unmap();
    values
}

fn action(logits: &[f32], config: PpoConfig) -> PpoJointAction {
    let mut action = PpoJointAction {
        candidate_count: 3,
        representative_mask: 3, // The enormous third logit is illegal.
        motor_masks: [0; 6],
        representative: 0, // Real production seam: movement is selected.
        forced_slots: vec![Some(0), Some(2), Some(4)],
        motor_candidates: [Some(0), None, None, None, None, None],
        old_joint_log_probability: 0.0,
        temperature: config.temperature,
    };
    action.old_joint_log_probability = cpu_ppo_objective(logits, &action, 0.0, config)
        .unwrap()
        .joint_log_probability;
    action
}

fn batch(logits: &[f32], config: PpoConfig, rewards: &[f32]) -> PpoBatch {
    PpoBatch::from_rollout(
        7,
        rewards
            .iter()
            .enumerate()
            .map(|(row, reward)| PpoTransition {
                policy_version: 7,
                trajectory_id: row as u64,
                step: 0,
                action: action(logits, config),
                reward: *reward,
                old_value: 0.0,
                next_value: 0.0,
                elapsed_seconds: 0.05,
                boundary: PpoBoundary::Terminated,
            })
            .collect(),
        config,
    )
    .unwrap()
}

fn eat() -> ImitationTarget {
    ImitationTarget {
        representative: None,
        motors: [None, None, Some(2), None, None, None],
    }
}

fn example(action: &PpoJointAction, target: ImitationTarget) -> ImitationExample {
    ImitationExample {
        candidate_count: action.candidate_count,
        representative_mask: action.representative_mask,
        motor_masks: action.motor_masks,
        forced_slots: action.forced_slots.clone(),
        target,
    }
}

#[test]
fn sparse_safe_food_gpu_objective_and_adam_match_cpu() {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(0x5AFE_2048, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.8).unwrap());
    // Same exact native graph admission as the existing GPU contract fixtures.
    let native = PhenotypeCompiler::compile_testing_procedural_baseline(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let source = FoundationWeightAsset::from_phenotype_for_genetic_birth(&native).unwrap();
    let (phenotype, _) =
        PhenotypeCompiler::compile_n2048_foundation_candidate(genome, development, source.clone())
            .unwrap();
    let mask = StageTrainableMask::from_synapse_indices(
        &phenotype,
        &(0..phenotype.synapses().len() as u32).collect::<Vec<_>>(),
    )
    .unwrap();
    let mut trainer = FoundationTrainer::new_required(
        phenotype.clone(),
        source.clone(),
        mask,
        AdamWConfig {
            learning_rate: 1e-3,
            weight_decay: 0.0,
            gradient_clip: 10.0,
            ..Default::default()
        },
    )
    .unwrap();
    println!(
        "GPU hardware receipt: {}",
        serde_json::to_string(trainer.session().backend().hardware_receipt()).unwrap()
    );
    let mut initial = trainer.checkpoint().unwrap();
    initial.optimizer_step = 16_777_220;
    for i in 0..initial.weights.len() {
        initial.first_moment[i] = (i % 7 + 1) as f32 * 1e-6;
        initial.second_moment[i] = (i % 11 + 1) as f32 * 1e-7;
        initial.update_ages[i] = 16_777_217 + (i % 3) as u32;
    }
    trainer.restore_checkpoint(&initial).unwrap();
    assert_eq!(trainer.checkpoint().unwrap(), initial);
    let n = phenotype.neuron_count() as usize;
    let tick = TrainingReplayTick {
        encoded_inputs: vec![0.0; n],
        projection_gain: 1.0,
        local_threshold_shift: 0.0,
        microstep_count: 0,
        enabled_routes: vec![false; phenotype.projections().len()],
        effective_weight_offsets: vec![0.0; phenotype.synapses().len()],
        structural_synapses: Vec::new(),
        candidates: [
            CandidateActionFamily::Approach,
            CandidateActionFamily::Ingest,
            CandidateActionFamily::Idle,
        ]
        .into_iter()
        .zip([18.0, 0.0, 1000.0])
        .map(|(family, innate_bias)| TrainingReplayCandidate {
            memory_readout: None,
            family,
            decoder_inputs: [0.1; 54],
            innate_bias,
        })
        .collect(),
    };
    let sequence = TrainingSequence {
        phenotype_hash: phenotype.phenotype_hash(),
        initial: TrainingInitialState {
            activations: vec![0.4; n],
            activity_ema: vec![0.0; n],
            metabolic_load: vec![0.0; n],
            dendrites: Default::default(),
        },
        ticks: vec![tick; 3],
        burn_in_ticks: 0,
        memory_candidate_gain: phenotype
            .candidate_decoder()
            .memory_channel()
            .map_or(0.0, |m| m.max_candidate_gain()),
    };
    let replay = trainer.evaluate_replay(&sequence).unwrap();
    let config = PpoConfig {
        epochs: 1,
        temperature: 1.0,
        value_coefficient: 0.0,
        entropy_coefficient: 0.0,
        target_kl: 100.0,
        ..Default::default()
    };
    {
        let logits = [18.0, 0.0, 1000.0];
        let batch = batch(&logits, config, &[0.3, -0.2, 0.4]);
        let (device, queue) = trainer
            .session()
            .backend()
            .offline_training_device_queue()
            .unwrap();
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sparse-imitation-numerical-logits"),
            size: 4 * 32 * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut packed = vec![0.0f32; 4 * 32];
        for row in 0..3 {
            packed[row * 32..row * 32 + 3].copy_from_slice(&logits);
        }
        queue.write_buffer(&buffer, 0, bytemuck::cast_slice(&packed));
        let rows = trainer
            .replay_rows()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(row, replay)| PpoReplayRow {
                logits_word_offset: (row * 32) as u32,
                features_word_offset: replay.features_word_offset,
            })
            .collect::<Vec<_>>();
        let mut objective = PpoGpuObjective::new(
            trainer.session(),
            &buffer,
            trainer.replay_state_buffer(),
            n as u32,
            4,
        )
        .unwrap();
        // Actual batch length3 is smaller than capacity4; then reuse the buffers
        // from labeled to empty to catch stale labels/counts and skipped metrics.
        for targets in [
            vec![ImitationTarget::unlabeled(); 3],
            vec![
                ImitationTarget::unlabeled(),
                eat(),
                ImitationTarget::unlabeled(),
            ],
            vec![eat(), ImitationTarget::unlabeled(), eat()],
            vec![ImitationTarget::unlabeled(); 3],
        ] {
            objective
                .upload(trainer.session(), &batch, &rows, config, 1, 7)
                .unwrap();
            let mut encoder = device.create_command_encoder(&Default::default());
            objective.encode_evaluate(&mut encoder).unwrap();
            queue.submit(Some(encoder.finish()));
            let ppo = read(
                trainer.session(),
                objective.output_buffer(),
                objective.adjoint_bytes(),
            );
            let ppo_metrics = read(
                trainer.session(),
                objective.output_buffer(),
                objective.metric_bytes(),
            );
            objective
                .upload_auxiliary_targets(trainer.session(), &batch, &targets, 0.1)
                .unwrap();
            let mut encoder = device.create_command_encoder(&Default::default());
            objective.encode_evaluate(&mut encoder).unwrap();
            objective.encode_imitation(&mut encoder, true).unwrap();
            queue.submit(Some(encoder.finish()));
            let combined = read(
                trainer.session(),
                objective.output_buffer(),
                objective.adjoint_bytes(),
            );
            let metrics = read(
                trainer.session(),
                objective.output_buffer(),
                objective.imitation_metric_bytes(),
            );
            assert_eq!(
                read(
                    trainer.session(),
                    objective.output_buffer(),
                    objective.metric_bytes()
                ),
                ppo_metrics
            );
            let labels = targets.iter().filter(|target| target.is_labeled()).count();
            let first_label = targets
                .iter()
                .position(|target| target.is_labeled())
                .unwrap_or(0);
            println!(
                "GPU sparse objective: labeled_rows={labels}, mean_loss={}, Eat_auxiliary_adjoint={}",
                imitation_mean_loss(&metrics).unwrap(),
                combined[first_label * 32 + 1] - ppo[first_label * 32 + 1]
            );
            let mut expected_loss = 0.0;
            for row in 0..3 {
                let transition = &batch.transitions()[row];
                let base = cpu_ppo_objective(
                    &logits,
                    &transition.action,
                    batch.actor_advantage(row).unwrap(),
                    config,
                )
                .unwrap();
                let extra = if targets[row].is_labeled() {
                    let extra = cpu_imitation_objective(
                        &logits,
                        &example(&transition.action, targets[row].clone()),
                        config.temperature,
                        0.1,
                    )
                    .unwrap();
                    expected_loss += extra.loss;
                    near(metrics[row * 4 + 1], extra.loss, 2e-5, 2e-5);
                    assert_eq!(metrics[row * 4 + 3], 1.0);
                    assert!(extra.logit_gradients[1] < 0.0);
                    extra.logit_gradients
                } else {
                    assert_eq!(&metrics[row * 4..row * 4 + 4], &[0.0, 0.0, 0.0, 2.0]);
                    assert_eq!(
                        &combined[row * 32..row * 32 + 32],
                        &ppo[row * 32..row * 32 + 32]
                    );
                    vec![0.0; 3]
                };
                for candidate in 0..3 {
                    near(
                        ppo[row * 32 + candidate],
                        base.logit_gradients[candidate] / 3.0,
                        2e-6,
                        2e-5,
                    );
                    near(
                        combined[row * 32 + candidate],
                        base.logit_gradients[candidate] / 3.0
                            + extra[candidate] / labels.max(1) as f32,
                        2e-6,
                        2e-5,
                    );
                }
                assert_eq!(
                    combined[row * 32 + 2],
                    0.0,
                    "illegal candidate must have zero adjoint"
                );
                if targets[row].is_labeled() {
                    let before = cpu_imitation_objective(
                        &logits,
                        &example(&transition.action, eat()),
                        1.0,
                        0.1,
                    )
                    .unwrap()
                    .loss;
                    let after = [
                        logits[0] - (combined[row * 32] - ppo[row * 32]),
                        logits[1] - (combined[row * 32 + 1] - ppo[row * 32 + 1]),
                        logits[2],
                    ];
                    assert!(
                        cpu_imitation_objective(
                            &after,
                            &example(&transition.action, eat()),
                            1.0,
                            0.1
                        )
                        .unwrap()
                        .loss
                            < before
                    );
                }
            }
            near(
                imitation_mean_loss(&metrics).unwrap(),
                expected_loss / labels.max(1) as f32,
                2e-5,
                2e-5,
            );
        }
    }
    assert_eq!(
        trainer.checkpoint().unwrap(),
        initial,
        "objective probes must not update Adam"
    );

    let batch = batch(&replay.candidate_logits[0], config, &[0.0; 3]);
    let windows = [1usize, 2, 1]
        .into_iter()
        .enumerate()
        .map(|(index, count)| {
            let mut sequence = sequence.clone();
            sequence.ticks.truncate(count);
            let mut targets = vec![ImitationTarget::unlabeled(); count];
            if index == 1 {
                targets[0] = eat();
            }
            PpoTrainingWindow {
                sequence,
                batch: batch.window(0..count).unwrap(),
                auxiliary: Some((targets, 0.1)),
                speech_targets: vec![None; count],
            }
        })
        .collect::<Vec<_>>();
    let value = PpoValueHeadCheckpoint {
        feature_count: n as u32,
        optimizer_step: 19_800,
        parameters: (0..(n + 1) * 3)
            .map(|i| (i % 7 + 1) as f32 * 1e-6)
            .collect(),
        last_updated_policy_version: Some(6),
    };
    let mut gpu_value = PpoTrainingState::from_checkpoint(value.clone()).unwrap();
    let mut cpu =
        CpuFoundationTrainer::from_checkpoint(phenotype.clone(), source.clone(), initial.clone())
            .unwrap();
    let mut cpu_value = CpuValueHead::from_checkpoint(value.clone()).unwrap();
    let cpu_receipt = train_cpu_ppo_cohort(
        &mut cpu,
        &mut cpu_value,
        3,
        2,
        |i| Ok(windows[i].clone()),
        config,
        7,
    )
    .unwrap();
    let gpu_receipt = train_recurrent_ppo_cohort(
        &mut trainer,
        &mut gpu_value,
        3,
        2,
        |i| Ok(windows[i].clone()),
        config,
        7,
    )
    .unwrap();
    assert_eq!(gpu_receipt.actor_optimizer_step, initial.optimizer_step + 2);
    assert_eq!(
        gpu_receipt.actor_optimizer_step,
        cpu_receipt.actor_optimizer_step
    );
    assert_eq!(gpu_receipt.completed_epochs, 1);
    assert!(!gpu_receipt.stopped_for_kl);
    let observed = trainer.checkpoint().unwrap();
    let expected = cpu.checkpoint();
    for i in 0..initial.weights.len() {
        assert_eq!(observed.update_ages[i], initial.update_ages[i] + 2);
        assert_eq!(observed.update_ages[i], expected.update_ages[i]);
        near(observed.weights[i], expected.weights[i], 3e-5, 1e-3);
        near(
            observed.first_moment[i],
            expected.first_moment[i],
            3e-6,
            1e-3,
        );
        near(
            observed.second_moment[i],
            expected.second_moment[i],
            3e-7,
            1e-3,
        );
        assert!(observed.second_moment[i] > 0.0);
        // With zero recurrent microsteps these coordinates receive no gradient.
        // Their retained nonzero moments must decay, rather than be reset. Use
        // tighter bounds than the general CPU/GPU arithmetic comparison above.
        if !matches!(
            trainer.phenotype().synapses()[i].kind(),
            CompiledSynapseKind::Decoder(_)
        ) {
            near(
                observed.first_moment[i],
                initial.first_moment[i] * initial.config.beta1.powi(2),
                2e-11,
                2e-5,
            );
            near(
                observed.second_moment[i],
                initial.second_moment[i] * initial.config.beta2.powi(2),
                2e-12,
                2e-5,
            );
        }
    }
    let observed_value = gpu_value.checkpoint(trainer.session()).unwrap();
    assert_eq!(observed_value, cpu_value.checkpoint());
    assert_eq!(observed_value.optimizer_step, value.optimizer_step);
    assert_eq!(observed_value.parameters, value.parameters);
    let mut control =
        CpuFoundationTrainer::from_checkpoint(phenotype, source, initial.clone()).unwrap();
    let mut control_value = CpuValueHead::from_checkpoint(value).unwrap();
    train_cpu_ppo_cohort(
        &mut control,
        &mut control_value,
        3,
        2,
        |i| {
            let mut window = windows[i].clone();
            window.auxiliary = None;
            Ok(window)
        },
        config,
        7,
    )
    .unwrap();
    assert!(
        observed
            .weights
            .iter()
            .zip(control.weights())
            .any(|(a, b)| (a - b).abs() > 1e-7),
        "auxiliary must cause a nonzero update beyond retained Adam momentum"
    );
    let after = trainer.evaluate_replay(&sequence).unwrap();
    let loss = |logits: &[f32]| {
        cpu_imitation_objective(logits, &example(&action(logits, config), eat()), 1.0, 0.1)
            .unwrap()
            .loss
    };
    let before_loss = loss(&replay.candidate_logits[0]);
    let after_loss = loss(&after.candidate_logits[0]);
    assert!(
        after_loss < before_loss,
        "real GPU actor update must improve Eat objective: {before_loss} -> {after_loss}"
    );
    println!(
        "GPU sparse qualification: actor_age={} -> {}, value_age={}, Eat_loss={} -> {}",
        initial.optimizer_step,
        observed.optimizer_step,
        observed_value.optimizer_step,
        before_loss,
        after_loss
    );
}
