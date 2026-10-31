//! Public objective upload admission after a real trainer scratch shrink.
#![cfg(feature = "gpu-tests")]

use alife_core::{
    BrainCapacityClass, BrainGenome, CandidateActionFamily, DevelopmentState,
    FoundationWeightAsset, NormalizedScalar, PhenotypeCompiler, ScaffoldContractError,
    SensorProfile, Tick,
};
use alife_runtime::GpuAuthoritativeSession;
use alife_training::{
    AdamWConfig, FoundationTrainer, ImitationExample, ImitationTarget, PpoBatch, PpoBoundary,
    PpoConfig, PpoJointAction, PpoReplayRow, PpoTrainingState, PpoTransition, StageTrainableMask,
    TrainingError, TrainingInitialState, TrainingReplayCandidate, TrainingReplayTick,
    TrainingSequence,
};

fn rejected(result: Result<(), TrainingError>) {
    assert!(matches!(
        result,
        Err(TrainingError::Contract(
            ScaffoldContractError::InvalidDecisionEvidence
        ))
    ));
}

#[test]
#[ignore = "explicit synthetic software Vulkan logical upload admission; no kernel or performance claim"]
fn public_uploads_keep_logical_bounds_after_real_scratch_shrink() {
    let genome = BrainGenome::scaffold(0xB0AD_2048, BrainCapacityClass::n2048().id());
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
    let mask = StageTrainableMask::from_synapse_indices(&phenotype, &[0]).unwrap();
    let session = GpuAuthoritativeSession::new_software_vulkan_training_test().unwrap();
    let mut trainer =
        FoundationTrainer::from_session(session, phenotype, source, mask, AdamWConfig::default())
            .unwrap();
    let n = trainer.phenotype().neuron_count();
    let sequence = |ticks| TrainingSequence {
        phenotype_hash: trainer.phenotype().phenotype_hash(),
        initial: TrainingInitialState {
            activations: vec![0.0; n as usize],
            activity_ema: vec![0.0; n as usize],
            metabolic_load: vec![0.0; n as usize],
            dendrites: Default::default(),
        },
        ticks: vec![
            TrainingReplayTick {
                encoded_inputs: vec![0.0; n as usize],
                projection_gain: 1.0,
                local_threshold_shift: 0.0,
                microstep_count: 0,
                enabled_routes: vec![false; trainer.phenotype().projections().len()],
                effective_weight_offsets: vec![0.0; trainer.phenotype().synapses().len()],
                structural_synapses: Vec::new(),
                candidates: [CandidateActionFamily::Approach, CandidateActionFamily::Idle]
                    .into_iter()
                    .map(|family| TrainingReplayCandidate {
                        family,
                        decoder_inputs: [0.0; 54],
                        innate_bias: 0.0,
                    })
                    .collect(),
            };
            ticks
        ],
        burn_in_ticks: 0,
        memory_candidate_gain: trainer
            .phenotype()
            .candidate_decoder()
            .memory_channel()
            .map_or(0.0, |channel| channel.max_candidate_gain()),
    };
    let (long, short) = (sequence(5), sequence(3));
    let mut state = PpoTrainingState::default();
    trainer.prepare_replay(&long).unwrap();
    state.prepare_for_replay(&trainer).unwrap();
    trainer.prepare_replay(&short).unwrap();
    state.prepare_for_replay(&trainer).unwrap();
    let (output_words, state_words) = trainer.replay_buffer_word_lengths().unwrap();
    let physical_output_words = trainer.replay_output_buffer().size() / 4;
    let physical_state_words = trainer.replay_state_buffer().size() / 4;
    assert!(physical_output_words >= output_words && physical_state_words >= state_words);
    if physical_output_words > output_words {
        assert!(output_words < physical_output_words);
        assert!(state_words < physical_state_words);
    }
    eprintln!(
        "actual_facade_bounds logical_output={output_words} physical_output={physical_output_words} logical_state={state_words} physical_state={physical_state_words}; invalid ends are logical+1 words"
    );
    let valid = PpoReplayRow {
        logits_word_offset: u32::try_from(output_words - 2).unwrap(),
        features_word_offset: u32::try_from(state_words - u64::from(n)).unwrap(),
    };
    let bad_logits = PpoReplayRow {
        logits_word_offset: valid.logits_word_offset + 1,
        ..valid
    };
    let bad_features = PpoReplayRow {
        features_word_offset: valid.features_word_offset + 1,
        ..valid
    };
    let config = PpoConfig::default();
    let action = PpoJointAction {
        candidate_count: 2,
        representative_mask: 3,
        motor_masks: [1, 0, 0, 0, 0, 0],
        representative: 1,
        forced_slots: vec![Some(0), Some(4)],
        motor_candidates: [Some(0), None, None, None, Some(1), None],
        old_joint_log_probability: -std::f32::consts::LN_2,
        temperature: 1.0,
    };
    let batch = PpoBatch::from_rollout(
        7,
        vec![PpoTransition {
            policy_version: 7,
            trajectory_id: 0,
            step: 0,
            action,
            reward: 1.0,
            old_value: 0.0,
            next_value: 0.0,
            elapsed_seconds: 0.05,
            boundary: PpoBoundary::Terminated,
        }],
        config,
    )
    .unwrap();
    let session = trainer.session();
    let objective = state.objective_mut().unwrap();
    objective
        .upload(session, &batch, &[valid], config, 1, 7)
        .unwrap();
    rejected(objective.upload(session, &batch, &[bad_logits], config, 1, 7));
    rejected(objective.upload(session, &batch, &[bad_features], config, 1, 7));
    objective
        .upload_value_features(session, &[valid.features_word_offset])
        .unwrap();
    rejected(objective.upload_value_features(session, &[bad_features.features_word_offset]));
    let example = ImitationExample {
        candidate_count: 2,
        representative_mask: 3,
        motor_masks: [1, 0, 0, 0, 0, 0],
        forced_slots: vec![Some(0), Some(4)],
        target: ImitationTarget {
            representative: Some(2),
            motors: [Some(1), None, None, None, None, None],
        },
    };
    objective
        .upload_imitation(session, std::slice::from_ref(&example), &[valid], 1.0, 1.0)
        .unwrap();
    rejected(objective.upload_imitation(session, &[example], &[bad_logits], 1.0, 1.0));
    // These calls test descriptor admission only. No fabricated boundary row is
    // evaluated by a kernel, and no actor/value update is submitted.
    assert_eq!(trainer.optimizer_step(), 0);
    eprintln!("actual_facade_bounds last-boundary acceptance3 and one-word-overrun typed rejection4 passed");
}
