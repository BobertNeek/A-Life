use super::*;
use crate::{
    AdamWConfig, StageTrainableMask, TrainingInitialState, TrainingReplayCandidate,
    TrainingReplayTick,
};
use alife_core::{
    BrainCapacityClass, BrainGenome, CandidateActionFamily, DendriticBranchSet, DevelopmentState,
    FoundationWeightAsset, NormalizedScalar, PhenotypeCompiler, SensorProfile, Tick,
    TrainingStageManifest,
};

pub(super) fn fixture() -> (CpuFoundationTrainer, PredictiveDataset, PredictiveHead) {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(0x7A11, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.8).unwrap());
    let base = PhenotypeCompiler::compile_testing_procedural_baseline(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let source = FoundationWeightAsset::from_phenotype_for_genetic_birth(&base).unwrap();
    let (p, _) =
        PhenotypeCompiler::compile_n2048_foundation_candidate(genome, development, source.clone())
            .unwrap();
    let n = p.neuron_count() as usize;
    let tick = TrainingReplayTick {
        encoded_inputs: (0..n).map(|i| (i % 11) as f32 * 0.015).collect(),
        projection_gain: 0.85,
        local_threshold_shift: 0.017,
        microstep_count: u32::from(p.microstep_count()),
        enabled_routes: vec![true; p.projections().len()],
        effective_weight_offsets: vec![0.002; p.synapses().len()],
        structural_synapses: vec![],
        candidates: vec![TrainingReplayCandidate {
            family: CandidateActionFamily::Idle,
            decoder_inputs: [0.; 54],
            innate_bias: 0.,
        }],
    };
    let replay = TrainingSequence {
        phenotype_hash: p.phenotype_hash(),
        initial: TrainingInitialState {
            activations: (0..n).map(|i| 0.1 + (i % 7) as f32 * 0.01).collect(),
            activity_ema: vec![0.2; n],
            metabolic_load: vec![0.03; n],
            dendrites: DendriticBranchSet::new(vec![]).unwrap(),
        },
        ticks: vec![tick; 3],
        burn_in_ticks: 1,
        memory_candidate_gain: p
            .candidate_decoder()
            .memory_channel()
            .map_or(0., |m| m.max_candidate_gain()),
    };
    let schema = PredictiveSchema {
        continuous_motors: vec![],
        version: "authored-numeric-contract-v1".into(),
        sensors: vec!["fixture_sensor".into()],
        body: vec!["fixture_body".into()],
        motors: vec![
            MotorChannel {
                name: "motion".into(),
                choices: vec!["none".into(), "forward".into()],
            },
            MotorChannel {
                name: "look".into(),
                choices: vec!["none".into(), "left".into(), "right".into()],
            },
        ],
    };
    let episode = PredictiveEpisode {
        motor_values: vec![],
        evidence: None,
        episode_id: "fixture-train".into(),
        split_group: "fixture-life-a".into(),
        split: PredictiveSplit::Train,
        replay,
        observations: vec![
            vec![0.1, 0.2],
            vec![0.2, 0.15],
            vec![0.4, 0.1],
            vec![0.3, 0.25],
        ],
        actions: vec![vec![0, 0], vec![1, 2], vec![0, 1]],
    };
    let mut validation = episode.clone();
    validation.episode_id = "fixture-validation".into();
    validation.split_group = "fixture-life-b".into();
    validation.split = PredictiveSplit::Validation;
    validation.observations[3] = vec![0.32, 0.23];
    validation.replay.ticks[0].encoded_inputs[0] += 0.01;
    let data = PredictiveDataset {
        schema_version: 1,
        source_description: "Authored numerical contract fixture; no world data, reward, creature learning or benchmark evidence".into(),
        schema: schema.clone(), episodes: vec![episode, validation],
    };
    let neurons = p
        .synapses()
        .iter()
        .filter(|s| s.kind() == CompiledSynapseKind::Recurrent)
        .map(|s| s.target() as usize)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(16)
        .collect();
    let head = PredictiveHead::new(&p, schema, neurons, 8, 71).unwrap();
    let mask = StageTrainableMask::recurrent_only(&p).unwrap();
    let trainer = CpuFoundationTrainer::new(p, source, mask, AdamWConfig::default()).unwrap();
    (trainer, data, head)
}

#[test]
fn all_motor_channels_condition_predictions_without_future_target_leakage() {
    let (trainer, data, head) = fixture();
    let base = head.probe(&trainer, &data, 0).unwrap();
    for (channel, choice) in [(0, 0), (1, 0)] {
        let mut changed = data.clone();
        changed.episodes[0].actions[1][channel] = choice;
        let probe = head.probe(&trainer, &changed, 0).unwrap();
        assert_ne!(base.predictions[1], probe.predictions[1]);
        assert!(
            base.recurrent
                .gradients
                .iter()
                .zip(&probe.recurrent.gradients)
                .any(|(a, b)| a != b),
            "motor channel {channel} did not affect recurrent gradient"
        );
        assert_eq!(base.predictions[0], probe.predictions[0]);
        assert_eq!(base.predictions[2], probe.predictions[2]);
    }
    let mut future = data.clone();
    future.episodes[0].observations[3] = vec![0.9, -0.8];
    let probe = head.probe(&trainer, &future, 0).unwrap();
    assert_eq!(base.predictions, probe.predictions);
    assert_ne!(base.metrics.mse, probe.metrics.mse);
    // A future replay input cannot alter earlier predictions.
    future.episodes[0].replay.ticks[2].encoded_inputs.fill(0.8);
    assert_eq!(
        base.predictions[..2],
        head.probe(&trainer, &future, 0).unwrap().predictions[..2]
    );
}

#[test]
fn predictive_mse_reaches_native_recurrent_bptt_and_matches_finite_differences() {
    let (trainer, data, head) = fixture();
    let probe = head.probe(&trainer, &data, 0).unwrap();
    assert!(probe
        .recurrent
        .gradients
        .iter()
        .zip(trainer.phenotype().synapses())
        .all(|(g, s)| s.kind() == CompiledSynapseKind::Recurrent || *g == 0.));
    let id = probe
        .recurrent
        .gradients
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.abs().total_cmp(&b.abs()))
        .unwrap()
        .0;
    let analytic = probe.recurrent.gradients[id];
    assert!(
        analytic.abs() > 1e-7,
        "missing native recurrent gradient: {analytic}"
    );
    // Freeze the actual burn-in output when perturbing weights. Re-running
    // burn-in under perturbed weights would differentiate a detached boundary.
    let e = &data.episodes[0];
    let eval = trainer.evaluate_replay(&e.replay).unwrap();
    let mut suffix = data.clone();
    let q = &mut suffix.episodes[0];
    q.replay.initial.activations = eval.final_activations[0].clone();
    q.replay.initial.metabolic_load = eval.final_metabolic_load[0].clone();
    q.replay.initial.activity_ema = eval.final_activity_ema[0].clone();
    q.replay.ticks.remove(0);
    q.replay.burn_in_ticks = 0;
    q.observations.remove(0);
    q.actions.remove(0);
    let epsilon = 0.002;
    let mut losses = Vec::new();
    for direction in [-1., 1.] {
        let mut checkpoint = trainer.checkpoint();
        checkpoint.weights[id] += direction * epsilon;
        let perturbed = CpuFoundationTrainer::from_checkpoint(
            trainer.phenotype().clone(),
            trainer.source_foundation.clone(),
            checkpoint,
        )
        .unwrap();
        losses.push(head.probe(&perturbed, &suffix, 0).unwrap().metrics.mse);
    }
    let numeric = ((losses[1] - losses[0]) / (2. * f64::from(epsilon))) as f32;
    assert!(
        (numeric - analytic).abs() < 2e-6 + 0.03 * analytic.abs(),
        "numeric={numeric} analytic={analytic}"
    );
    // Check the head's chain rule independently as well.
    let tape = trainer.forward(&e.replay).unwrap();
    let pass = head
        .pass(
            e,
            &tape.evaluation.final_activations,
            PredictiveInputs::Full,
        )
        .unwrap();
    let id = pass.gradients[0]
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.abs().total_cmp(&b.abs()))
        .unwrap()
        .0;
    let mut losses = Vec::new();
    for direction in [-1., 1.] {
        let mut perturbed = head.clone();
        perturbed.input_weights[id] += direction * epsilon;
        losses.push(
            perturbed
                .pass(
                    e,
                    &tape.evaluation.final_activations,
                    PredictiveInputs::Full,
                )
                .unwrap()
                .metrics
                .mse,
        );
    }
    let numeric = ((losses[1] - losses[0]) / (2. * f64::from(epsilon))) as f32;
    assert!((numeric - pass.gradients[0][id]).abs() < 2e-6);
}

#[test]
fn dataset_rejects_cross_split_lives_copied_evidence_bad_shapes_and_nonfinite_values() {
    let (trainer, data, _) = fixture();
    let encoded = serde_json::to_vec(&data).unwrap();
    assert_eq!(
        serde_json::from_slice::<PredictiveDataset>(&encoded).unwrap(),
        data
    );
    data.validate_for(trainer.phenotype()).unwrap();
    for variant in 0..9 {
        let mut broken = data.clone();
        match variant {
            0 => broken.episodes[1].split_group = broken.episodes[0].split_group.clone(),
            1 => {
                let mut e = broken.episodes[0].clone();
                e.episode_id = "copy".into();
                e.split_group = "new-name".into();
                e.split = PredictiveSplit::Validation;
                broken.episodes[1] = e;
            }
            2 => {
                broken.episodes[0].observations.pop();
            }
            3 => {
                broken.episodes[0].actions[1].pop();
            }
            4 => broken.episodes[0].actions[1][1] = 3,
            5 => broken.episodes[0].observations[1][0] = f32::NAN,
            6 => broken.episodes[0].replay.burn_in_ticks = 3,
            7 => broken.schema.body[0] = broken.schema.sensors[0].clone(),
            8 => broken.episodes[0].replay.ticks[0]
                .encoded_inputs
                .pop()
                .map(|_| ())
                .unwrap(),
            _ => unreachable!(),
        }
        assert!(
            broken.validate_for(trainer.phenotype()).is_err(),
            "accepted variant {variant}"
        );
    }
}

#[test]
fn deterministic_updates_preserve_frozen_decoder_state_and_inference_codec() {
    let (mut trainer, data, mut head) = fixture();
    let mut other = CpuFoundationTrainer::from_checkpoint(
        trainer.phenotype().clone(),
        trainer.source_foundation.clone(),
        trainer.checkpoint(),
    )
    .unwrap();
    let mut other_head = head.clone();
    let initial = trainer.checkpoint();
    for _ in 0..3 {
        head.train_step(&mut trainer, &data, 0, 0.03).unwrap();
        other_head
            .train_epoch(&mut other, &data, 0.03, PredictiveInputs::Full)
            .unwrap();
    }
    assert_eq!(head, other_head);
    assert_eq!(trainer.checkpoint(), other.checkpoint());
    let trained = trainer.checkpoint();
    assert_eq!(trained.optimizer_step, 3);
    assert!(trained
        .weights
        .iter()
        .zip(&initial.weights)
        .any(|(a, b)| a != b));
    for (i, s) in trainer.phenotype().synapses().iter().enumerate() {
        if s.kind() != CompiledSynapseKind::Recurrent {
            assert_eq!(trained.weights[i].to_bits(), initial.weights[i].to_bits());
            assert_eq!(trained.first_moment[i], initial.first_moment[i]);
            assert_eq!(trained.second_moment[i], initial.second_moment[i]);
            assert_eq!(trained.update_ages[i], initial.update_ages[i]);
        }
    }
    let saved_head = head.clone();
    head.evaluate(&trainer, &data, PredictiveSplit::Validation)
        .unwrap();
    assert!(head.train_step(&mut trainer, &data, 1, 0.03).is_err());
    assert_eq!(saved_head, head);
    assert_eq!(trained, trainer.checkpoint());
    let asset = trainer
        .export_candidate(TrainingStageManifest::new(1, 1, 1))
        .unwrap();
    let bytes = asset.encode_canonical().unwrap();
    let restored = FoundationWeightAsset::decode_canonical(&bytes).unwrap();
    assert_eq!(restored.weights(), trainer.weights());
    let checkpoint_json = serde_json::to_string(&trained).unwrap();
    assert!(!checkpoint_json.contains("predictive"));
    assert!(!checkpoint_json.contains("input_weights"));
    let resumed = CpuFoundationTrainer::from_checkpoint(
        trainer.phenotype().clone(),
        trainer.source_foundation.clone(),
        trained,
    )
    .unwrap();
    // Inference needs only native trainer state, and still works after head drop.
    drop(head);
    assert_eq!(
        trainer.evaluate_replay(&data.episodes[0].replay).unwrap(),
        resumed.evaluate_replay(&data.episodes[0].replay).unwrap()
    );
}

#[test]
fn topology_schema_and_bad_update_rejections_are_atomic() {
    let (mut trainer, data, mut head) = fixture();
    let checkpoint = trainer.checkpoint();
    let original = head.clone();
    for rate in [0., -1., f32::NAN, f32::INFINITY] {
        assert!(head.train_step(&mut trainer, &data, 0, rate).is_err());
    }
    let mut wrong = data.clone();
    wrong.schema.version = "incompatible".into();
    assert!(head.train_step(&mut trainer, &wrong, 0, 0.03).is_err());
    wrong = data.clone();
    wrong.episodes[0].replay.initial.activations.pop();
    assert!(head.train_step(&mut trainer, &wrong, 0, 0.03).is_err());
    assert_eq!(head, original);
    assert_eq!(trainer.checkpoint(), checkpoint);
}

#[test]
fn continuous_motor_conditioning_and_explicit_ablations_work() {
    let (trainer, mut data, _) = fixture();
    data.schema.continuous_motors = vec!["motor.duration".into()];
    for e in &mut data.episodes {
        e.motor_values = vec![vec![0.2]; 3];
    }
    let head = PredictiveHead::new(
        trainer.phenotype(),
        data.schema.clone(),
        vec![trainer
            .phenotype()
            .synapses()
            .iter()
            .find(|s| s.kind() == CompiledSynapseKind::Recurrent)
            .unwrap()
            .target() as usize],
        8,
        71,
    )
    .unwrap();
    let baseline = head.probe(&trainer, &data, 0).unwrap();
    let mut changed = data.clone();
    changed.episodes[0].motor_values[1][0] = 0.8;
    assert_ne!(
        baseline.predictions[1],
        head.probe(&trainer, &changed, 0).unwrap().predictions[1]
    );
    let a = head
        .evaluate_with_inputs(
            &trainer,
            &data,
            PredictiveSplit::Train,
            PredictiveInputs::NoAction,
        )
        .unwrap();
    let b = head
        .evaluate_with_inputs(
            &trainer,
            &changed,
            PredictiveSplit::Train,
            PredictiveInputs::NoAction,
        )
        .unwrap();
    assert_eq!(a, b);
    changed.episodes[0].motor_values.pop();
    assert!(changed.validate_for(trainer.phenotype()).is_err());
}

fn initial_handoff(trainer: &CpuFoundationTrainer) -> crate::FoundationTrainingHandoffV1 {
    let genome = BrainGenome::scaffold(0x7A11, BrainCapacityClass::n2048().id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.8).unwrap());
    let (_, inputs) = PhenotypeCompiler::compile_n2048_foundation_candidate(
        genome,
        development,
        trainer.source_foundation.clone(),
    )
    .unwrap();
    crate::FoundationTrainingHandoffV1::new(
        crate::FoundationTrainingBackendV1::NativeCpu,
        1,
        inputs,
        trainer.checkpoint(),
        crate::PpoValueHeadCheckpoint {
            feature_count: trainer.phenotype().neuron_count(),
            optimizer_step: 0,
            parameters: vec![0.; (trainer.phenotype().neuron_count() as usize + 1) * 3],
            last_updated_policy_version: Some(1),
        },
    )
    .unwrap()
}

#[test]
fn experimental_state_recovers_resumes_and_rebinds_without_losing_optimizer_state() {
    use artifact::*;
    let (mut trainer, data, mut head) = fixture();
    let source = initial_handoff(&trainer);
    let mut aged = trainer.checkpoint();
    aged.optimizer_step = 16_777_220;
    for i in 0..aged.weights.len() {
        aged.update_ages[i] = 16_777_217;
        aged.first_moment[i] = 1e-7;
        aged.second_moment[i] = 1e-8;
    }
    trainer.restore_checkpoint(aged).unwrap();
    head.train_step(&mut trainer, &data, 0, 0.03).unwrap();
    let run = PredictiveRun::from_dataset(
        71,
        &data,
        identity(&data).unwrap(),
        1,
        0.03,
        PredictiveInputs::Full,
    )
    .unwrap();
    let state =
        ExperimentalPredictiveState::capture(&trainer, &head, source.clone(), run.clone(), None)
            .unwrap();
    let root = std::env::temp_dir().join(format!(
        "predictive-state-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    let path = root.join("state.json");
    state.write_new(&path).unwrap();
    assert!(state.write_new(&path).is_err());
    let restored = ExperimentalPredictiveState::read(&path).unwrap();
    let mut reassigned = data.clone();
    reassigned.episodes[0].split = PredictiveSplit::Validation;
    reassigned.episodes[1].split = PredictiveSplit::Train;
    let reassigned_run = PredictiveRun::from_dataset(
        71,
        &reassigned,
        identity(&reassigned).unwrap(),
        1,
        0.03,
        PredictiveInputs::Full,
    )
    .unwrap();
    assert!(restored.validate_next_run(&reassigned_run).is_err());
    for e in &mut reassigned.episodes {
        e.split_group.push_str("-renamed");
    }
    let renamed_run = PredictiveRun::from_dataset(
        71,
        &reassigned,
        identity(&reassigned).unwrap(),
        1,
        0.03,
        PredictiveInputs::Full,
    )
    .unwrap();
    assert!(restored.validate_next_run(&renamed_run).is_err());
    let (mut other, mut other_head) = restored.restore().unwrap();
    assert_eq!(other.checkpoint(), trainer.checkpoint());
    assert_eq!(other_head, head);
    head.train_step(&mut trainer, &data, 0, 0.03).unwrap();
    other_head.train_step(&mut other, &data, 0, 0.03).unwrap();
    assert_eq!(other.checkpoint(), trainer.checkpoint());
    assert_eq!(other_head, head);
    let next = ExperimentalPredictiveState::capture(
        &other,
        &other_head,
        source.clone(),
        run,
        Some(&restored),
    )
    .unwrap();
    assert_eq!(
        next.payload.parent_state_sha256,
        Some(restored.payload_sha256)
    );
    let (rebound, inputs) = next.into_disposable_founder().unwrap();
    let a = other.checkpoint();
    let b = rebound.checkpoint();
    assert_eq!(a.weights, b.weights);
    assert_eq!(a.first_moment, b.first_moment);
    assert_eq!(a.second_moment, b.second_moment);
    assert_eq!(a.update_ages, b.update_ages);
    assert_eq!(a.optimizer_step, b.optimizer_step);
    assert_eq!(a.stage_mask, b.stage_mask);
    assert_eq!(inputs.n2048_candidate_asset().unwrap().weights(), a.weights);
    assert_eq!(next.payload.source, source);
    let pending = root.join(".recoverable.pending");
    std::fs::rename(&path, &pending).unwrap();
    assert_eq!(ExperimentalPredictiveState::read(&pending).unwrap(), state);
    let mut corrupt = state.clone();
    corrupt.payload.actor.weights[0] += 0.1;
    assert!(corrupt.restore().is_err());
    corrupt = state.clone();
    corrupt.payload.head.input_weights.pop();
    corrupt.payload_sha256 = identity(&corrupt.payload).unwrap();
    assert!(corrupt.restore().is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn public_world_capture_roundtrips_causal_pairs_and_rejects_misalignment() {
    use recording::*;
    let recording = capture_public_simulation(PublicCaptureConfig {
        founder_seed: 71,
        world_seeds: [72, 73, 74],
        sampling_seeds: [101, 202, 303],
        world_ticks: 5,
    })
    .unwrap();
    let bytes = serde_json::to_vec(&recording).unwrap();
    let decoded: PublicSimulationRecording = serde_json::from_slice(&bytes).unwrap();
    let (data, metrics) = convert_recording(&decoded).unwrap();
    assert!(metrics.converted_pairs >= 6);
    assert!(metrics.native_replay_max_absolute_error <= 1e-5);
    assert_eq!(recording.source.actor.optimizer_step, 0);
    assert!(data
        .episodes
        .iter()
        .all(|e| e.evidence.as_ref().unwrap().source_tick + 1
            == e.evidence.as_ref().unwrap().target_tick));
    assert_eq!(data.schema.continuous_motors.len(), 101);
    let mut broken = decoded.clone();
    broken.groups[1].world_seed = broken.groups[0].world_seed;
    assert!(convert_recording(&broken).is_err());
    broken = decoded.clone();
    broken.groups[0].decisions[0].final_features[0] += 0.5;
    assert!(convert_recording(&broken).is_err());
    let mut broken_data = data.clone();
    broken_data.episodes[0]
        .evidence
        .as_mut()
        .unwrap()
        .outcome_tick += 1;
    let p = recording.source.clone().admit().unwrap().phenotype;
    assert!(broken_data.validate_for(&p).is_err());
    // Observation persistence is evaluated on the exact same scored pairs.
    assert_eq!(
        persistence_metrics(&data, PredictiveSplit::Validation)
            .unwrap()
            .scored_ticks,
        data.episodes
            .iter()
            .filter(|e| e.split == PredictiveSplit::Validation)
            .count()
    );
}

#[test]
#[ignore = "explicit CPU numerical-fixture trial, not a gameplay or throughput benchmark"]
fn numerical_fixture_trial() {
    let (mut trainer, data, mut head) = fixture();
    let initial_train = head
        .evaluate(&trainer, &data, PredictiveSplit::Train)
        .unwrap();
    let initial_validation = head
        .evaluate(&trainer, &data, PredictiveSplit::Validation)
        .unwrap();
    let start = std::time::Instant::now();
    for _ in 0..12 {
        head.train_step(&mut trainer, &data, 0, 0.1).unwrap();
    }
    let elapsed = start.elapsed().as_secs_f64();
    let final_train = head
        .evaluate(&trainer, &data, PredictiveSplit::Train)
        .unwrap();
    let final_validation = head
        .evaluate(&trainer, &data, PredictiveSplit::Validation)
        .unwrap();
    println!(
        "{}",
        serde_json::json!({
            "evidence": "authored numerical fixture only; not recorded world data",
            "backend": trainer.hardware_receipt(), "neurons": trainer.phenotype().neuron_count(),
            "synapses": trainer.weights().len(), "ticks_per_episode": 3, "burn_in_ticks": 1,
            "steps": 12, "elapsed_seconds": elapsed,
            "initial_train": initial_train, "final_train": final_train,
            "initial_validation": initial_validation, "final_validation": final_validation,
            "optimizer_step": trainer.optimizer_step(), "head_exported": false,
            "throughput_improvement_measured": false, "gameplay_gain_measured": false
        })
    );
}

#[test]
#[ignore = "writes initial authored fixture inputs only to an explicitly named new temporary directory"]
fn write_numerical_fixture_inputs() {
    let root = std::path::PathBuf::from(
        std::env::var("ALIFE_PREDICTIVE_FIXTURE_OUTPUT")
            .expect("set ALIFE_PREDICTIVE_FIXTURE_OUTPUT to a new temporary directory"),
    );
    assert!(!root.exists(), "fixture output must be new");
    let (trainer, data, _) = fixture();
    let genome = BrainGenome::scaffold(0x7A11, BrainCapacityClass::n2048().id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.8).unwrap());
    let (p, inputs) = PhenotypeCompiler::compile_n2048_foundation_candidate(
        genome,
        development,
        trainer.source_foundation.clone(),
    )
    .unwrap();
    assert_eq!(p.phenotype_hash(), trainer.phenotype().phenotype_hash());
    let value = crate::PpoValueHeadCheckpoint {
        feature_count: p.neuron_count(),
        optimizer_step: 0,
        parameters: vec![0.; (p.neuron_count() as usize + 1) * 3],
        last_updated_policy_version: Some(1),
    };
    let handoff = crate::FoundationTrainingHandoffV1::new(
        crate::FoundationTrainingBackendV1::NativeCpu,
        1,
        inputs,
        trainer.checkpoint(),
        value,
    )
    .unwrap();
    std::fs::create_dir(&root).unwrap();
    for (name, bytes) in [
        ("dataset.json", serde_json::to_vec(&data).unwrap()),
        (
            "training-handoff.json",
            serde_json::to_vec(&handoff).unwrap(),
        ),
    ] {
        std::fs::write(root.join(name), bytes).unwrap();
    }
    println!("Authored numerical fixture inputs only: {}", root.display());
}
