#![cfg(feature = "gpu-tests")]

use alife_core::{
    BrainCapacityClass, BrainGenome, CandidateActionFamily, CandidateFeatureVector,
    CompiledSynapseKind, DecoderHeadKind, DevelopmentState, FoundationWeightAsset,
    NormalizedScalar, PhenotypeCompiler, SensorProfile, Tick, TrainingStageManifest,
};
use alife_training::{
    AdamWConfig, CandidateTrainingTarget, FoundationCurriculumStage, FoundationTrainer,
    N2048CurriculumV1, N2048FoundationProgram, SpeechTrainingTarget, StageTrainableMask,
    TrainingInitialState, TrainingReplayCandidate, TrainingReplayTick, TrainingSequence,
    TrainingSequence32, TrainingTick, TRAINING_SEQUENCE_TICKS,
};

fn native_exact_foundation(
    genome: &BrainGenome,
    development: &DevelopmentState,
) -> (alife_core::BrainPhenotype, FoundationWeightAsset) {
    let native = PhenotypeCompiler::compile_testing_procedural_baseline(
        genome,
        &BrainCapacityClass::n2048(),
        development,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let source = FoundationWeightAsset::from_phenotype_for_genetic_birth(&native).unwrap();
    let (phenotype, _) = PhenotypeCompiler::compile_n2048_foundation_candidate(
        genome.clone(),
        development.clone(),
        source.clone(),
    )
    .unwrap();
    assert!(phenotype
        .synapses()
        .iter()
        .zip(source.weights())
        .all(|(synapse, weight)| synapse.genetic_weight().to_bits() == weight.to_bits()));
    (phenotype, source)
}

#[test]
fn n2048_exact_graph_adamw_step_changes_only_the_masked_weight_and_exports() {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(0x7A11_2048, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.8).unwrap());
    let (phenotype, source) = native_exact_foundation(&genome, &development);
    let (synapse_index, source_neuron, input_lane) = phenotype
        .synapses()
        .iter()
        .enumerate()
        .find_map(|(index, synapse)| match synapse.kind() {
            CompiledSynapseKind::Decoder(coordinate)
                if coordinate.head() == DecoderHeadKind::ActionCandidate
                    && coordinate.family() == CandidateActionFamily::Approach =>
            {
                Some((index as u32, synapse.source(), coordinate.input_lane()))
            }
            _ => None,
        })
        .unwrap();
    let mask = StageTrainableMask::from_synapse_indices(&phenotype, &[synapse_index]).unwrap();
    let neuron_count = phenotype.neuron_count() as usize;
    let mut features = [0.0_f32; 24];
    features[usize::from(input_lane)] = 1.0;
    let candidate = CandidateTrainingTarget::try_new(
        CandidateActionFamily::Approach,
        CandidateFeatureVector(features),
        1.0,
        1.0,
    )
    .unwrap();
    let mut inputs = vec![0.0; neuron_count];
    inputs[source_neuron as usize] = 1.0;
    let tick = TrainingTick::try_new(
        inputs,
        vec![0.0; neuron_count],
        vec![0.0; neuron_count],
        Some(candidate),
    )
    .unwrap();
    let sequence = TrainingSequence32::try_new(vec![tick; TRAINING_SEQUENCE_TICKS]).unwrap();
    let before = phenotype
        .synapses()
        .iter()
        .map(|s| s.genetic_weight())
        .collect::<Vec<_>>();
    let mut trainer =
        FoundationTrainer::new_required(phenotype.clone(), source, mask, AdamWConfig::default())
            .unwrap();
    let weights_before_evaluation = trainer.read_weights().unwrap();
    let evaluation_before = trainer.evaluate_sequence(&sequence).unwrap();
    assert_eq!(
        evaluation_before.candidate_logits().len(),
        TRAINING_SEQUENCE_TICKS
    );
    assert_eq!(
        evaluation_before.episode_count(),
        TRAINING_SEQUENCE_TICKS as u32
    );
    assert_eq!(trainer.read_weights().unwrap(), weights_before_evaluation);
    let receipt = trainer.train_step(&sequence).unwrap();
    assert_eq!(receipt.optimizer_step, 1);
    assert_eq!(receipt.trained_weight_count, 1);
    assert!(receipt.unclipped_gradient_norm > 0.0);
    assert!(receipt.loss_after < receipt.loss_before);
    let evaluation_after = trainer.evaluate_sequence(&sequence).unwrap();
    assert!(evaluation_after.mean_loss() < evaluation_before.mean_loss());

    let trained = trainer.read_weights().unwrap();
    let changed = before
        .iter()
        .zip(&trained)
        .enumerate()
        .filter(|(_, (left, right))| left.to_bits() != right.to_bits())
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    assert_eq!(changed, vec![synapse_index as usize]);

    let asset = trainer
        .export_candidate(TrainingStageManifest::new(1, 1, 1))
        .unwrap();
    let encoded = asset.encode_canonical().unwrap();
    let decoded = FoundationWeightAsset::decode_canonical(&encoded).unwrap();
    assert_eq!(decoded.weights(), trained);
    let (rebuilt, _) = PhenotypeCompiler::compile_n2048_foundation_candidate(
        genome.clone(),
        development.clone(),
        decoded.clone(),
    )
    .unwrap();
    decoded.validate_against(&rebuilt).unwrap();
    assert!(rebuilt
        .synapses()
        .iter()
        .zip(decoded.weights())
        .all(|(synapse, weight)| synapse.genetic_weight().to_bits() == weight.to_bits()));

    // Optimizer resume includes both moments and the step, not just weights.
    let checkpoint = trainer.checkpoint().unwrap();
    let stats = trainer
        .train_step_with_statistics(&sequence, false)
        .unwrap();
    assert_eq!(stats.loss_after, None);
    let uninterrupted = trainer.checkpoint().unwrap();
    trainer.restore_checkpoint(&checkpoint).unwrap();
    trainer
        .train_step_with_statistics(&sequence, false)
        .unwrap();
    assert_eq!(trainer.checkpoint().unwrap(), uninterrupted);

    // A fixed-context replay with zero actual microsteps must carry the initial
    // state through padding. Nonzero lifetime/fast offsets participate in every
    // decoder head; only genetic weights receive the supplied policy adjoint.
    let mut initial = vec![0.0; neuron_count];
    initial[source_neuron as usize] = 0.4;
    let mut decoder_inputs = [0.0; 54];
    decoder_inputs[usize::from(input_lane)] = 1.0;
    decoder_inputs[24..36].fill(0.1);
    decoder_inputs[36..].fill(0.02);
    let context = TrainingReplayTick {
        encoded_inputs: vec![0.0; neuron_count],
        projection_gain: 1.3,
        local_threshold_shift: 0.1,
        microstep_count: 0,
        enabled_routes: vec![false; phenotype.projections().len()],
        effective_weight_offsets: vec![0.01; phenotype.synapses().len()],
        structural_synapses: Vec::new(),
        candidates: vec![TrainingReplayCandidate {
            family: candidate.family,
            decoder_inputs,
        }],
    };
    let replay = TrainingSequence {
        phenotype_hash: phenotype.phenotype_hash(),
        initial: TrainingInitialState {
            activations: initial.clone(),
            activity_ema: vec![0.2; neuron_count],
            metabolic_load: vec![0.3; neuron_count],
            dendrites: Default::default(),
        },
        ticks: vec![context.clone(), context],
        burn_in_ticks: 1,
        memory_candidate_gain: phenotype
            .candidate_decoder()
            .memory_channel()
            .map_or(0.0, |p| p.max_candidate_gain()),
    };
    trainer.prepare_replay(&replay).unwrap();
    assert_eq!(trainer.checkpoint().unwrap(), uninterrupted);
    let (device, queue) = trainer
        .session()
        .backend()
        .offline_training_device_queue()
        .unwrap();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    trainer.encode_replay_forward(&mut encoder).unwrap();
    queue.submit(Some(encoder.finish()));
    let logits = trainer.read_replay_logits().unwrap();
    let mut expected = phenotype
        .candidate_decoder()
        .families()
        .iter()
        .find(|f| f.family() == candidate.family)
        .unwrap()
        .bias();
    let mut memory = 0.0;
    let mut cognitive = 0.0;
    for (synapse, weight) in phenotype.synapses().iter().zip(&uninterrupted.weights) {
        if let CompiledSynapseKind::Decoder(c) = synapse.kind() {
            if c.family() != candidate.family || c.head() == DecoderHeadKind::SpeechPayload {
                continue;
            }
            let weighted = decoder_inputs[usize::from(c.input_lane())] * (weight + 0.01);
            match c.head().raw() {
                1 => expected += initial[synapse.source() as usize] * weighted,
                2 => memory += weighted,
                4 => cognitive += weighted,
                _ => {}
            }
        }
    }
    let limit = replay.memory_candidate_gain;
    expected += memory.clamp(-limit, limit) + cognitive.clamp(-limit, limit);
    assert!((logits[0] - expected).abs() < 1.0e-5);
    assert_eq!(
        logits[0].to_bits(),
        logits[alife_core::MAX_ACTION_CANDIDATES].to_bits()
    );
    assert_eq!(trainer.replay_rows().unwrap().len(), 1);
    let mut wrong = replay.clone();
    wrong.ticks[0].effective_weight_offsets.pop();
    assert!(trainer.prepare_replay(&wrong).is_err());

    use wgpu::util::DeviceExt;
    let (device, _) = trainer
        .session()
        .backend()
        .offline_training_device_queue()
        .unwrap();
    let mut adjoints = [0.0f32; alife_core::MAX_ACTION_CANDIDATES];
    adjoints[0] = 1.0;
    let adjoints = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("replay-test-policy-adjoint"),
        contents: bytemuck::cast_slice(&adjoints),
        usage: wgpu::BufferUsages::COPY_SRC,
    });
    let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    trainer.submit_replay_update(encoder, &adjoints, 0).unwrap();
    let updated = trainer.checkpoint().unwrap();
    let index = synapse_index as usize;
    let expected_moment = uninterrupted.config.beta1 * uninterrupted.first_moment[index]
        + (1.0 - uninterrupted.config.beta1) * 0.4;
    assert!((updated.first_moment[index] - expected_moment).abs() < 1.0e-6);

    // Eight normalized windows form one effective batch, including when each
    // window refreshes replay scratch. No Adam state advances per window.
    trainer.prepare_replay(&replay).unwrap();
    let (device, queue) = trainer
        .session()
        .backend()
        .offline_training_device_queue()
        .unwrap();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    trainer.encode_replay_forward(&mut encoder).unwrap();
    queue.submit(Some(encoder.finish()));
    let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    trainer.submit_replay_update(encoder, &adjoints, 0).unwrap();
    let single = trainer.checkpoint().unwrap();
    trainer.restore_checkpoint(&updated).unwrap();
    trainer.begin_gradient_accumulation(8).unwrap();
    for count in 1..=8 {
        trainer.prepare_replay(&replay).unwrap();
        let (device, queue) = trainer
            .session()
            .backend()
            .offline_training_device_queue()
            .unwrap();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        trainer.encode_replay_forward(&mut encoder).unwrap();
        queue.submit(Some(encoder.finish()));
        let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        assert_eq!(
            trainer
                .accumulate_replay_gradients(encoder, &adjoints, 0, 0.125)
                .unwrap(),
            count
        );
        assert_eq!(trainer.optimizer_step(), updated.optimizer_step);
    }
    assert_eq!(trainer.read_weights().unwrap(), updated.weights);
    assert!(
        trainer.checkpoint().is_err(),
        "partial optimizer state is not a complete checkpoint"
    );
    let (device, _) = trainer
        .session()
        .backend()
        .offline_training_device_queue()
        .unwrap();
    let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    trainer.apply_accumulated_gradients(encoder).unwrap();
    let batched = trainer.checkpoint().unwrap();
    assert_eq!(batched.optimizer_step, single.optimizer_step);
    for (a, b) in batched
        .weights
        .iter()
        .chain(&batched.first_moment)
        .chain(&batched.second_moment)
        .zip(
            single
                .weights
                .iter()
                .chain(&single.first_moment)
                .chain(&single.second_moment),
        )
    {
        assert!((a - b).abs() < 1.0e-6);
    }
    trainer.begin_gradient_accumulation(1).unwrap();
    trainer.cancel_gradient_accumulation();
    assert_eq!(trainer.checkpoint().unwrap(), batched);

    verify_newly_enabled_adam_age(&mut trainer, &phenotype, &replay, index);
    verify_sampled_replay_gradients(&mut trainer, &phenotype, source_neuron, input_lane);

    // A cohort transition admits the exact trained export in fresh organisms;
    // it preserves optimizer coordinates and rejects stale replay/checkpoints.
    let before_rebind = trainer.checkpoint().unwrap();
    assert!(trainer.rebind_for_next_cohort(rebuilt, decoded).is_err());
    let mut incompatible_genome = genome.clone();
    incompatible_genome.alpha_mask =
        alife_core::AlphaMask::default_for_projection(NormalizedScalar::new(0.731).unwrap());
    let (incompatible, incompatible_asset) =
        native_exact_foundation(&incompatible_genome, &development);
    assert!(phenotype
        .synapses()
        .iter()
        .zip(incompatible.synapses())
        .any(|(a, b)| a.alpha().to_bits() != b.alpha().to_bits()));
    assert!(trainer
        .rebind_for_next_cohort(incompatible, incompatible_asset)
        .is_err());
    assert_eq!(trainer.checkpoint().unwrap(), before_rebind);
    let before_forward = trainer.evaluate_replay(&replay).unwrap();
    let value_checkpoint = alife_training::PpoValueHeadCheckpoint {
        feature_count: phenotype.neuron_count(),
        optimizer_step: 7,
        parameters: vec![0.125; (neuron_count + 1) * 3],
        last_updated_policy_version: Some(2),
    };
    let mut value_state =
        alife_training::PpoTrainingState::from_checkpoint(value_checkpoint.clone()).unwrap();
    value_state.prepare_for_replay(&trainer).unwrap();
    let next_asset = trainer
        .export_candidate(TrainingStageManifest::new(1, 1, 1))
        .unwrap();
    let (next_phenotype, _) = PhenotypeCompiler::compile_n2048_foundation_candidate(
        genome,
        development,
        next_asset.clone(),
    )
    .unwrap();
    trainer.begin_gradient_accumulation(1).unwrap();
    assert!(trainer
        .rebind_for_next_cohort(next_phenotype.clone(), next_asset.clone())
        .is_err());
    trainer.cancel_gradient_accumulation();
    trainer
        .rebind_for_next_cohort(next_phenotype.clone(), next_asset.clone())
        .unwrap();
    let mut expected_checkpoint = before_rebind.clone();
    expected_checkpoint.phenotype_hash = next_phenotype.phenotype_hash();
    expected_checkpoint.source_foundation_digest = next_asset.digest();
    assert_ne!(
        expected_checkpoint.phenotype_hash,
        before_rebind.phenotype_hash
    );
    assert_eq!(trainer.checkpoint().unwrap(), expected_checkpoint);
    assert!(trainer.replay_all_rows().is_err());
    assert!(value_state.prepare_for_replay(&trainer).is_err());
    assert!(trainer.prepare_replay(&replay).is_err());
    assert!(trainer.restore_checkpoint(&before_rebind).is_err());
    // Synthetic parity fixture only: production must recollect under the new
    // asset, never relabel captures from the preceding policy.
    let mut next_replay = replay;
    next_replay.phenotype_hash = next_phenotype.phenotype_hash();
    let encoded = serde_json::to_vec(&next_replay).unwrap();
    let next_replay: TrainingSequence = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(
        trainer.evaluate_replay(&next_replay).unwrap(),
        before_forward
    );
    value_state.prepare_for_replay(&trainer).unwrap();
    assert_eq!(
        value_state.checkpoint(trainer.session()).unwrap(),
        value_checkpoint
    );
    trainer.restore_checkpoint(&expected_checkpoint).unwrap();
    assert_eq!(trainer.checkpoint().unwrap(), expected_checkpoint);
}

fn verify_newly_enabled_adam_age(
    trainer: &mut FoundationTrainer,
    phenotype: &alife_core::BrainPhenotype,
    replay: &TrainingSequence,
    previously_trained: usize,
) {
    use wgpu::util::DeviceExt;
    let baseline = trainer.checkpoint().unwrap();
    assert_eq!(baseline.schema_version, 2);
    assert!(baseline.update_ages[previously_trained] > 0);
    let (new_index, new_source, new_lane) = phenotype
        .synapses()
        .iter()
        .enumerate()
        .find_map(|(i, s)| match s.kind() {
            CompiledSynapseKind::Decoder(c)
                if i != previously_trained
                    && c.head() == DecoderHeadKind::ActionCandidate
                    && c.family() == CandidateActionFamily::Approach =>
            {
                Some((i, s.source(), c.input_lane()))
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(baseline.update_ages[new_index], 0);
    let mask = StageTrainableMask::from_synapse_indices(phenotype, &[new_index as u32]).unwrap();
    let mut sequence = replay.clone();
    sequence.initial.activations.fill(0.0);
    sequence.initial.activations[new_source as usize] = 0.4;
    for tick in &mut sequence.ticks {
        tick.candidates[0].decoder_inputs.fill(0.0);
        tick.candidates[0].decoder_inputs[new_lane as usize] = 1.0;
    }
    let update = |trainer: &mut FoundationTrainer| {
        trainer.prepare_replay(&sequence).unwrap();
        let (device, queue) = trainer
            .session()
            .backend()
            .offline_training_device_queue()
            .unwrap();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        trainer.encode_replay_forward(&mut encoder).unwrap();
        queue.submit(Some(encoder.finish()));
        let mut values = [0.0f32; alife_core::MAX_ACTION_CANDIDATES];
        values[0] = 1.0;
        let adjoints = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("newly-enabled-age-adjoint"),
            contents: bytemuck::cast_slice(&values),
            usage: wgpu::BufferUsages::COPY_SRC,
        });
        let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        trainer.submit_replay_update(encoder, &adjoints, 0).unwrap();
        trainer.checkpoint().unwrap()
    };
    trainer.set_stage_mask(mask.clone()).unwrap();
    let continued = update(trainer);
    assert_eq!(continued.update_ages[new_index], 1);
    for i in 0..baseline.weights.len() {
        if i == new_index {
            continue;
        }
        assert_eq!(continued.update_ages[i], baseline.update_ages[i]);
        assert_eq!(
            continued.first_moment[i].to_bits(),
            baseline.first_moment[i].to_bits()
        );
        assert_eq!(
            continued.second_moment[i].to_bits(),
            baseline.second_moment[i].to_bits()
        );
        assert_eq!(
            continued.weights[i].to_bits(),
            baseline.weights[i].to_bits()
        );
    }
    // Same weights/gradient, genuinely fresh optimizer, so B's first step must
    // be identical despite the other run's older global batch counter.
    let mut fresh = baseline;
    fresh.optimizer_step = 0;
    fresh.first_moment.fill(0.0);
    fresh.second_moment.fill(0.0);
    fresh.update_ages.fill(0);
    fresh.stage_mask = mask;
    trainer.restore_checkpoint(&fresh).unwrap();
    let reference = update(trainer);
    assert_eq!(
        continued.weights[new_index].to_bits(),
        reference.weights[new_index].to_bits()
    );
    assert_eq!(
        continued.first_moment[new_index].to_bits(),
        reference.first_moment[new_index].to_bits()
    );
    assert_eq!(
        continued.second_moment[new_index].to_bits(),
        reference.second_moment[new_index].to_bits()
    );
    assert_eq!(
        continued.update_ages[new_index],
        reference.update_ages[new_index]
    );
    trainer.restore_checkpoint(&continued).unwrap();
    assert_eq!(trainer.checkpoint().unwrap(), continued);
    let mut legacy = continued.clone();
    legacy.schema_version = 1;
    assert!(trainer.restore_checkpoint(&legacy).is_err());
    assert_eq!(trainer.checkpoint().unwrap(), continued);
}

fn verify_sampled_replay_gradients(
    trainer: &mut FoundationTrainer,
    phenotype: &alife_core::BrainPhenotype,
    motor_source: u32,
    action_lane: u16,
) {
    let motor = phenotype.candidate_decoder();
    let recurrent = phenotype
        .synapses()
        .iter()
        .enumerate()
        .find(|(_, s)| {
            s.kind() == CompiledSynapseKind::Recurrent
                && s.source() != s.target()
                && !(motor.motor_start()..motor.motor_start() + u32::from(motor.motor_width()))
                    .contains(&s.target())
                && s.genetic_weight() > 0.0
        })
        .unwrap()
        .0;
    let selected = phenotype.synapses()[recurrent];
    let decoder_index = |head| {
        phenotype
            .synapses()
            .iter()
            .enumerate()
            .find_map(|(i, s)| match s.kind() {
                CompiledSynapseKind::Decoder(c)
                    if c.head() == head
                        && c.family() == CandidateActionFamily::Approach
                        && (head != DecoderHeadKind::ActionCandidate
                            || (s.source() == motor_source && c.input_lane() == action_lane)) =>
                {
                    Some(i)
                }
                _ => None,
            })
            .unwrap()
    };
    let action = decoder_index(DecoderHeadKind::ActionCandidate);
    let memory = decoder_index(DecoderHeadKind::MemoryContext);
    assert_eq!(phenotype.synapses().iter().filter(|s| matches!(s.kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::CognitiveContext)).count(), 0,
        "this native N2048 fixture has no cognitive decoder; no executed derivative claim for that head");
    let mut checkpoint = trainer.checkpoint().unwrap();
    checkpoint.weights[recurrent] = 0.25;
    checkpoint.weights[action] = 0.2;
    checkpoint.weights[memory] = 0.05;
    trainer.restore_checkpoint(&checkpoint).unwrap();
    let neurons = phenotype.neuron_count() as usize;
    let mut initial = vec![0.0; neurons];
    initial[selected.source() as usize] = 0.3;
    initial[selected.target() as usize] = 0.1;
    let mut encoded = vec![0.0; neurons];
    encoded[selected.source() as usize] = 0.3;
    encoded[selected.target() as usize] = 0.2;
    encoded[motor_source as usize] = 0.2;
    let mut enabled_routes = vec![false; phenotype.projections().len()];
    enabled_routes[selected.route_index() as usize] = true;
    let mut offsets = vec![0.0; phenotype.synapses().len()];
    for (i, s) in phenotype.synapses().iter().enumerate() {
        if s.kind() == CompiledSynapseKind::Recurrent && i != recurrent {
            offsets[i] = -checkpoint.weights[i];
        }
    }
    let mut decoder_inputs = [0.0; 54];
    decoder_inputs[action_lane as usize] = 1.0;
    decoder_inputs[24..36].fill(0.0001);
    decoder_inputs[36..].fill(0.001);
    let tick = TrainingReplayTick {
        encoded_inputs: encoded,
        projection_gain: 0.7,
        local_threshold_shift: -0.05,
        microstep_count: u32::from(phenotype.microstep_count()).min(4),
        enabled_routes,
        effective_weight_offsets: offsets,
        structural_synapses: Vec::new(),
        candidates: vec![TrainingReplayCandidate {
            family: CandidateActionFamily::Approach,
            decoder_inputs,
        }],
    };
    assert!(tick.microstep_count >= 2);
    let dendrites = alife_core::DendriticBranchSet::new(vec![alife_core::DendriticBranch::new(
        motor_source,
        -0.2,
        0.4,
        vec![alife_core::DendriticInputRef::new(selected.target(), 0.6).unwrap()],
    )
    .unwrap()])
    .unwrap();
    let sequence = TrainingSequence {
        phenotype_hash: phenotype.phenotype_hash(),
        initial: TrainingInitialState {
            activations: initial,
            activity_ema: vec![0.1; neurons],
            metabolic_load: vec![0.02; neurons],
            dendrites,
        },
        ticks: vec![tick; 3],
        burn_in_ticks: 1,
        memory_candidate_gain: motor.memory_channel().unwrap().max_candidate_gain(),
    };
    let adjoints = vec![vec![0.7], vec![-0.2]];
    let probe = trainer
        .probe_replay_gradients(&sequence, &adjoints)
        .unwrap();
    assert_eq!(
        trainer.checkpoint().unwrap(),
        checkpoint,
        "probe must not optimize"
    );

    // Detach burn-in for numerical perturbations too. Recomputing burn-in with
    // perturbed weights would test a different derivative contract.
    let receipt = trainer.evaluate_replay(&sequence).unwrap();
    let mut suffix = sequence.clone();
    suffix.initial.activations = receipt.final_activations[0].clone();
    suffix.initial.activity_ema = receipt.final_activity_ema[0].clone();
    suffix.initial.metabolic_load = receipt.final_metabolic_load[0].clone();
    suffix.ticks.remove(0);
    suffix.burn_in_ticks = 0;
    let detached = trainer.probe_replay_gradients(&suffix, &adjoints).unwrap();
    for index in [recurrent, action, memory] {
        let analytical = f64::from(probe.gradients[index]);
        assert!((analytical - f64::from(detached.gradients[index])).abs() < 1.0e-7);
        assert!(
            analytical.abs() > 1.0e-6,
            "sample {index} must exercise an active gradient"
        );
        let epsilon = 0.01f32;
        let mut perturbed = checkpoint.clone();
        perturbed.weights[index] += epsilon;
        trainer.restore_checkpoint(&perturbed).unwrap();
        let plus = trainer.evaluate_replay(&suffix).unwrap();
        perturbed.weights[index] = checkpoint.weights[index] - epsilon;
        trainer.restore_checkpoint(&perturbed).unwrap();
        let minus = trainer.evaluate_replay(&suffix).unwrap();
        let loss = |values: &alife_training::TrainingReplayEvaluation| {
            values
                .candidate_logits
                .iter()
                .zip(&adjoints)
                .map(|(row, a)| f64::from(row[0]) * f64::from(a[0]))
                .sum::<f64>()
        };
        let numerical = (loss(&plus) - loss(&minus)) / f64::from(2.0 * epsilon);
        assert!(
            (analytical - numerical).abs() <= 2.0e-6 + 0.02 * analytical.abs(),
            "synapse {index}: analytic {analytical}, numerical {numerical}"
        );
    }
    trainer.restore_checkpoint(&checkpoint).unwrap();
    // With the branch removed the selected nonmotor recurrence has no active
    // route into a decoder. This isolates its source-activation chain rule.
    let mut without_branch = suffix.clone();
    without_branch.initial.dendrites = Default::default();
    let without = trainer
        .probe_replay_gradients(&without_branch, &adjoints)
        .unwrap();
    assert!(without.gradients[recurrent].abs() < 1.0e-7);
    assert!(probe.gradients[recurrent].abs() > 1.0e-6);
    assert_eq!(trainer.checkpoint().unwrap(), checkpoint);
}

#[test]
fn n2048_speech_payload_head_trains_on_gpu_and_remains_exportable() {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(0x5AEE_2048, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.8).unwrap());
    let (phenotype, source) = native_exact_foundation(&genome, &development);
    let (synapse_index, source_neuron, output_index) = phenotype
        .synapses()
        .iter()
        .enumerate()
        .find_map(|(index, synapse)| match synapse.kind() {
            CompiledSynapseKind::Decoder(coordinate)
                if coordinate.head() == DecoderHeadKind::SpeechPayload =>
            {
                Some((index as u32, synapse.source(), coordinate.motor_index()))
            }
            _ => None,
        })
        .unwrap();
    let mask = StageTrainableMask::from_synapse_indices(&phenotype, &[synapse_index]).unwrap();
    let mut inputs = vec![0.0; phenotype.neuron_count() as usize];
    inputs[source_neuron as usize] = 1.0;
    let tick = TrainingTick::try_new(
        inputs,
        vec![0.0; phenotype.neuron_count() as usize],
        vec![0.0; phenotype.neuron_count() as usize],
        None,
    )
    .unwrap()
    .with_speech_target(SpeechTrainingTarget::try_new(output_index, 1.0, 1.0).unwrap())
    .unwrap();
    let sequence = TrainingSequence32::try_new(vec![tick; TRAINING_SEQUENCE_TICKS]).unwrap();
    let before = phenotype
        .synapses()
        .iter()
        .map(|s| s.genetic_weight())
        .collect::<Vec<_>>();
    let mut trainer =
        FoundationTrainer::new_required(phenotype, source, mask, AdamWConfig::default()).unwrap();
    let evaluation_before = trainer.evaluate_sequence(&sequence).unwrap();
    let receipt = trainer.train_step(&sequence).unwrap();
    let evaluation_after = trainer.evaluate_sequence(&sequence).unwrap();
    assert_eq!(
        evaluation_before.speech_logits().len(),
        TRAINING_SEQUENCE_TICKS
    );
    assert!(receipt.unclipped_gradient_norm > 0.0);
    assert!(evaluation_after.mean_loss() < evaluation_before.mean_loss());
    let trained = trainer.read_weights().unwrap();
    let changed = before
        .iter()
        .zip(&trained)
        .enumerate()
        .filter(|(_, (left, right))| left.to_bits() != right.to_bits())
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    assert_eq!(changed, vec![synapse_index as usize]);
}

#[test]
fn n2048_curriculum_stage_trains_and_evaluates_256_held_out_gpu_episodes() {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(0xC011_2048, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.8).unwrap());
    let (phenotype, source) = native_exact_foundation(&genome, &development);
    let curriculum = N2048CurriculumV1::new();
    let mask = curriculum
        .stage_mask(&phenotype, FoundationCurriculumStage::GroundedPerception)
        .unwrap();
    let trainer =
        FoundationTrainer::new_required(phenotype, source, mask, AdamWConfig::default()).unwrap();
    let mut program = N2048FoundationProgram::new(trainer);
    let receipt = program
        .run_stage(
            FoundationCurriculumStage::GroundedPerception,
            1,
            0xA11C_E001,
        )
        .unwrap();
    println!("{receipt:?}");
    assert_eq!(receipt.evaluation.episodes(), 256);
    assert!(receipt.evaluation.frozen_weights_bit_identical());
    assert!(receipt.first_training_loss.is_finite());
    assert!(receipt.final_training_loss.is_finite());
    assert_eq!(
        program.completed_stage_count(),
        u16::from(receipt.gate_passed)
    );
}

#[test]
fn n2048_ppo_and_imitation_gpu_objective_matches_joint_derivatives_and_partial_batches() {
    use alife_training::{
        train_recurrent_ppo_cohort, ImitationExample, ImitationTarget, PpoBatch, PpoBoundary,
        PpoConfig, PpoGpuObjective, PpoJointAction, PpoReplayRow, PpoTrainingState,
        PpoTrainingWindow, PpoTransition,
    };
    let genome = BrainGenome::scaffold(0x9900_2048, BrainCapacityClass::n2048().id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.8).unwrap());
    let (phenotype, source) = native_exact_foundation(&genome, &development);
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
                    index as u32,
                    synapse.source() as usize,
                    usize::from(coordinate.input_lane()),
                ))
            }
            _ => None,
        })
        .unwrap();
    let mask = StageTrainableMask::from_synapse_indices(&phenotype, &[weight]).unwrap();
    let n = phenotype.neuron_count() as usize;
    let mut initial = vec![0.0; n];
    initial[neuron] = 0.4;
    let candidates = [
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
        }
    })
    .collect();
    let sequence = TrainingSequence {
        phenotype_hash: phenotype.phenotype_hash(),
        initial: TrainingInitialState {
            activations: initial,
            activity_ema: vec![0.0; n],
            metabolic_load: vec![0.0; n],
            dendrites: Default::default(),
        },
        ticks: vec![TrainingReplayTick {
            encoded_inputs: vec![0.0; n],
            projection_gain: 1.0,
            local_threshold_shift: 0.0,
            microstep_count: 0,
            enabled_routes: vec![false; phenotype.projections().len()],
            effective_weight_offsets: vec![0.0; phenotype.synapses().len()],
            structural_synapses: Vec::new(),
            candidates,
        }],
        burn_in_ticks: 0,
        memory_candidate_gain: phenotype
            .candidate_decoder()
            .memory_channel()
            .map_or(0.0, |p| p.max_candidate_gain()),
    };
    let mut trainer =
        FoundationTrainer::new_required(phenotype, source, mask, AdamWConfig::default()).unwrap();
    let frozen = trainer.checkpoint().unwrap();
    let actual = trainer.evaluate_replay(&sequence).unwrap();
    let original = actual.candidate_logits[0].clone();
    let probabilities = |logits: &[f64]| {
        let maximum = logits[..3]
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let z = logits[..3].iter().map(|v| (v - maximum).exp()).sum::<f64>();
        logits[..3]
            .iter()
            .map(|v| (v - maximum).exp() / z)
            .collect::<Vec<_>>()
    };
    // Idle representative forces posture once; movement remains an independent factor.
    let joint_logp = |logits: &[f64]| {
        let p = probabilities(logits);
        (p[2] * p[0] / (p[0] + p[1])).ln()
    };
    let original64 = original.iter().map(|v| f64::from(*v)).collect::<Vec<_>>();
    let old_logp = joint_logp(&original64) as f32;
    let config = PpoConfig {
        entropy_coefficient: 0.0,
        epochs: 1,
        target_kl: 100.0,
        ..Default::default()
    };
    let action = PpoJointAction {
        candidate_count: 4,
        representative_mask: 7,
        motor_masks: [3, 0, 0, 0, 0, 0],
        representative: 2,
        forced_slots: vec![Some(0), Some(0), Some(4), Some(4)],
        motor_candidates: [Some(0), None, None, None, Some(2), None],
        old_joint_log_probability: old_logp,
        temperature: 1.0,
    };
    let batch = PpoBatch::from_rollout(
        7,
        vec![PpoTransition {
            policy_version: 7,
            trajectory_id: 0,
            step: 0,
            action: action.clone(),
            reward: 1.0,
            old_value: 0.0,
            next_value: 0.0,
            elapsed_seconds: 0.05,
            boundary: PpoBoundary::Terminated,
        }],
        config,
    )
    .unwrap();
    let example = ImitationExample {
        candidate_count: 4,
        representative_mask: 7,
        motor_masks: action.motor_masks,
        forced_slots: action.forced_slots.clone(),
        target: ImitationTarget {
            representative: Some(5),
            motors: [Some(1), None, None, None, None, None],
        },
    };
    let row = PpoReplayRow {
        logits_word_offset: 0,
        features_word_offset: trainer.replay_rows().unwrap()[0].features_word_offset,
    };
    {
        let (device, queue) = trainer
            .session()
            .backend()
            .offline_training_device_queue()
            .unwrap();
        let logits = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("objective-numerical-logits"),
            size: 32 * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // These logits come from actual graph replay. Only the illegal lane is
        // made enormous to ensure masking excludes it from BOTH objectives.
        let mut values = original.clone();
        values[3] = 100.0;
        queue.write_buffer(&logits, 0, bytemuck::cast_slice(&values));
        let mut objective = PpoGpuObjective::new(
            trainer.session(),
            &logits,
            trainer.replay_state_buffer(),
            n as u32,
            1,
        )
        .unwrap();
        objective
            .upload(trainer.session(), &batch, &[row], config, 1, 7)
            .unwrap();
        let mut encoder = device.create_command_encoder(&Default::default());
        objective.encode_evaluate(&mut encoder).unwrap();
        queue.submit(Some(encoder.finish()));
        let metrics = ppo_contract_read(
            trainer.session(),
            objective.output_buffer(),
            objective.metric_bytes(),
        );
        let ppo_gradient = ppo_contract_read(
            trainer.session(),
            objective.output_buffer(),
            objective.adjoint_bytes(),
        );
        assert!((metrics[0] - old_logp).abs() < 2.0e-5);
        assert!((metrics[6] - 1.0).abs() < 2.0e-5);
        assert_eq!(metrics[7], 1.0);
        let epsilon = 1.0e-4;
        for candidate in 0..4 {
            let mut plus = original64.clone();
            plus[candidate] += epsilon;
            let mut minus = original64.clone();
            minus[candidate] -= epsilon;
            let loss = |x: &[f64]| -(joint_logp(x) - f64::from(old_logp)).exp();
            let numerical = (loss(&plus) - loss(&minus)) / (2.0 * epsilon);
            assert!(
                (f64::from(ppo_gradient[candidate]) - numerical).abs() < 2.0e-4,
                "PPO candidate {candidate}: {} vs {numerical}",
                ppo_gradient[candidate]
            );
        }
        assert_eq!(ppo_gradient[3], 0.0);
        assert_eq!(
            ppo_contract_read(
                trainer.session(),
                objective.output_buffer(),
                objective.value_preflight_bytes()
            ),
            vec![1.0]
        );
        let mut encoder = device.create_command_encoder(&Default::default());
        objective.encode_value_update(&mut encoder).unwrap();
        queue.submit(Some(encoder.finish()));
        let head = ppo_contract_read(
            trainer.session(),
            objective.value_head_buffer(),
            0..((n + 1) * 3 * 4) as u64,
        );
        assert!(
            head[n] > 0.0,
            "positive return must increase detached value bias"
        );
        objective
            .upload_imitation(trainer.session(), &[example.clone()], &[row], 1.0, 1.0)
            .unwrap();
        let mut encoder = device.create_command_encoder(&Default::default());
        objective.encode_imitation(&mut encoder, false).unwrap();
        queue.submit(Some(encoder.finish()));
        let metrics = ppo_contract_read(
            trainer.session(),
            objective.output_buffer(),
            objective.imitation_metric_bytes(),
        );
        let bc_gradient = ppo_contract_read(
            trainer.session(),
            objective.output_buffer(),
            objective.adjoint_bytes(),
        );
        let bc_loss = |x: &[f64]| {
            let p = probabilities(x);
            -(p[0] + p[2] * p[0] / (p[0] + p[1])).ln()
        };
        assert!((f64::from(metrics[1]) - bc_loss(&original64)).abs() < 2.0e-5);
        assert_eq!(metrics[3], 1.0);
        for candidate in 0..4 {
            let mut plus = original64.clone();
            plus[candidate] += epsilon;
            let mut minus = original64.clone();
            minus[candidate] -= epsilon;
            let numerical = (bc_loss(&plus) - bc_loss(&minus)) / (2.0 * epsilon);
            assert!(
                (f64::from(bc_gradient[candidate]) - numerical).abs() < 2.0e-4,
                "BC candidate {candidate}: {} vs {numerical}",
                bc_gradient[candidate]
            );
        }
        assert!(bc_gradient[0] < 0.0 && bc_gradient[1] > 0.0);
        assert_eq!(bc_gradient[3], 0.0);
        // Auxiliary mode adds the same gradient to PPO without overwriting it.
        objective
            .upload(trainer.session(), &batch, &[row], config, 1, 7)
            .unwrap();
        objective
            .upload_auxiliary_targets(trainer.session(), &batch, &[example.target], 1.0)
            .unwrap();
        let mut encoder = device.create_command_encoder(&Default::default());
        objective.encode_evaluate(&mut encoder).unwrap();
        objective.encode_imitation(&mut encoder, true).unwrap();
        queue.submit(Some(encoder.finish()));
        let combined = ppo_contract_read(
            trainer.session(),
            objective.output_buffer(),
            objective.adjoint_bytes(),
        );
        for i in 0..4 {
            assert!((combined[i] - ppo_gradient[i] - bc_gradient[i]).abs() < 2.0e-5);
        }
    }
    assert_eq!(
        trainer.checkpoint().unwrap(),
        frozen,
        "objective/value probes must leave actor frozen"
    );
    let mut state = PpoTrainingState::default();
    assert_eq!(
        state.predict_values(&mut trainer, &sequence).unwrap(),
        vec![0.0]
    );
    assert_eq!(trainer.checkpoint().unwrap(), frozen);
    let receipt = train_recurrent_ppo_cohort(
        &mut trainer,
        &mut state,
        3,
        2,
        |_| {
            Ok(PpoTrainingWindow {
                sequence: sequence.clone(),
                batch: batch.clone(),
                auxiliary: None,
            })
        },
        config,
        7,
    )
    .unwrap();
    assert_eq!(receipt.completed_epochs, 1);
    assert!(!receipt.stopped_for_kl);
    assert_eq!(
        receipt.actor_optimizer_step, 2,
        "two-window batch plus one-window partial batch"
    );
    assert_eq!(receipt.value_optimizer_step, 2);
    assert_eq!(receipt.mean_kl_before_updates.len(), 2);
    assert_eq!(
        state.checkpoint(trainer.session()).unwrap().optimizer_step,
        2
    );
    assert!(state.predict_values(&mut trainer, &sequence).unwrap()[0] > 0.0);
    assert_eq!(
        batch.transitions()[0]
            .action
            .old_joint_log_probability
            .to_bits(),
        old_logp.to_bits(),
        "updates must never rewrite the recorded behavior likelihood"
    );
}

fn ppo_contract_read(
    session: &alife_runtime::GpuAuthoritativeSession,
    source: &wgpu::Buffer,
    range: std::ops::Range<u64>,
) -> Vec<f32> {
    let (device, queue) = session.backend().offline_training_device_queue().unwrap();
    let size = range.end - range.start;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ppo-contract-readback"),
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
            timeout: None,
        })
        .unwrap();
    receiver.recv().unwrap().unwrap();
    let mapped = buffer.slice(..size).get_mapped_range();
    let result = bytemuck::cast_slice::<u8, f32>(&mapped).to_vec();
    drop(mapped);
    buffer.unmap();
    result
}
