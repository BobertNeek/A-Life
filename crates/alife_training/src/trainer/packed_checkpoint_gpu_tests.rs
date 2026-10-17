//! Real-adapter tests only: a separate-copy oracle for the packed capture.
//! This synthetic fixed-context fixture is not training/promotion evidence.

use super::*;
use crate::{
    FoundationTrainerCheckpoint, TrainingInitialState, TrainingReplayCandidate, TrainingReplayTick,
};
use alife_core::{
    BrainCapacityClass, BrainGenome, CandidateActionFamily, DecoderHeadKind, DevelopmentState,
    NormalizedScalar, PhenotypeCompiler, SensorProfile, Tick,
};

// The old checkpoint body, using the retained independent buffer readers.
// Never call read_checkpoint_state or its decoder from this oracle.
fn separate_checkpoint(
    trainer: &FoundationTrainer,
) -> Result<FoundationTrainerCheckpoint, TrainingError> {
    if trainer.accumulation.is_some() {
        return Err(ScaffoldContractError::PhenotypeCompile.into());
    }
    let n = trainer.phenotype.synapses().len();
    let moments = trainer.read_float_buffer(&trainer.gpu._optimizer, 0, n * 2)?;
    Ok(FoundationTrainerCheckpoint {
        schema_version: 2,
        phenotype_hash: trainer.phenotype.phenotype_hash(),
        source_foundation_digest: trainer.source_foundation.digest(),
        optimizer_step: trainer.optimizer_step,
        config: trainer.config,
        stage_mask: trainer.stage_mask.clone(),
        weights: trainer.read_weights()?,
        first_moment: moments[..n].to_vec(),
        second_moment: moments[n..].to_vec(),
        update_ages: trainer.read_word_buffer(&trainer.gpu._optimizer, n as u64 * 8, n)?,
    })
}

fn payload_bytes(checkpoint: &FoundationTrainerCheckpoint) -> Vec<u8> {
    checkpoint
        .weights
        .iter()
        .chain(&checkpoint.first_moment)
        .chain(&checkpoint.second_moment)
        .map(|value| value.to_bits())
        .chain(checkpoint.update_ages.iter().copied())
        .flat_map(u32::to_ne_bytes)
        .collect()
}

fn assert_exact(actual: &FoundationTrainerCheckpoint, expected: &FoundationTrainerCheckpoint) {
    // PartialEq covers metadata; byte comparison additionally distinguishes
    // signed zeros in every float lane and preserves ages as integer words.
    let actual_bytes = payload_bytes(actual);
    let expected_bytes = payload_bytes(expected);
    assert_eq!(actual_bytes.len(), expected_bytes.len());
    assert_eq!(
        actual_bytes
            .iter()
            .zip(&expected_bytes)
            .position(|(a, b)| a != b),
        None,
        "first differing checkpoint payload byte"
    );
    assert!(actual == expected, "checkpoint metadata/value mismatch");
    assert!(
        serde_json::to_vec(actual).unwrap() == serde_json::to_vec(expected).unwrap(),
        "serialized checkpoint identity differs"
    );
}

fn compare_captures(
    trainer: &FoundationTrainer,
) -> (FoundationTrainerCheckpoint, FoundationTrainerCheckpoint) {
    let separate = separate_checkpoint(trainer).unwrap();
    let packed = trainer.checkpoint().unwrap();
    assert_exact(&packed, &separate);
    // Capturing must not mutate the authoritative device state.
    assert_exact(&separate_checkpoint(trainer).unwrap(), &separate);
    (separate, packed)
}

fn update(trainer: &mut FoundationTrainer, replay: &TrainingSequence) {
    trainer.prepare_replay(replay).unwrap();
    let (device, queue) = trainer
        .session
        .backend()
        .offline_training_device_queue()
        .unwrap();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    trainer.encode_replay_forward(&mut encoder).unwrap();
    queue.submit(Some(encoder.finish()));
    let mut values = vec![
        0.0_f32;
        (replay.ticks.len() - replay.burn_in_ticks)
            * alife_core::MAX_ACTION_CANDIDATES
    ];
    for row in values
        .as_chunks_mut::<{ alife_core::MAX_ACTION_CANDIDATES }>()
        .0
    {
        row[0] = 1.0;
    }
    let adjoints = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("packed-checkpoint-regression-adjoints"),
        contents: bytemuck::cast_slice(&values),
        usage: wgpu::BufferUsages::COPY_SRC,
    });
    let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    trainer.submit_replay_update(encoder, &adjoints, 0).unwrap();
}

fn compare_next_update(
    trainer: &mut FoundationTrainer,
    replay: &TrainingSequence,
    trainable: usize,
) -> FoundationTrainerCheckpoint {
    let (separate, packed) = compare_captures(trainer);
    update(trainer, replay);
    let (_, uninterrupted) = compare_captures(trainer);
    assert_eq!(uninterrupted.optimizer_step, separate.optimizer_step + 1);
    assert_eq!(
        uninterrupted.update_ages[trainable],
        separate.update_ages[trainable] + 1
    );
    assert_ne!(
        uninterrupted.weights[trainable].to_bits(),
        separate.weights[trainable].to_bits()
    );
    for index in 0..separate.weights.len() {
        if index != trainable {
            assert_eq!(
                uninterrupted.weights[index].to_bits(),
                separate.weights[index].to_bits()
            );
            assert_eq!(
                uninterrupted.first_moment[index].to_bits(),
                separate.first_moment[index].to_bits()
            );
            assert_eq!(
                uninterrupted.second_moment[index].to_bits(),
                separate.second_moment[index].to_bits()
            );
            assert_eq!(
                uninterrupted.update_ages[index],
                separate.update_ages[index]
            );
        }
    }
    // Both captures independently restore to the same next real WGSL update.
    for saved in [separate, packed] {
        trainer.restore_checkpoint(&saved).unwrap();
        update(trainer, replay);
        let (_, resumed) = compare_captures(trainer);
        assert_exact(&resumed, &uninterrupted);
    }
    uninterrupted
}

#[test]
fn packed_checkpoint_matches_separate_captures_and_next_adam_after_shape_and_rebind() {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(0x7A11_2048, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.8).unwrap());
    // Same exact native production graph fixture as wgsl_foundation_trainer.
    let native = PhenotypeCompiler::compile_testing_procedural_baseline(
        &genome,
        &capacity,
        &development,
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
    let (index, source_neuron, input_lane) = phenotype
        .synapses()
        .iter()
        .enumerate()
        .find_map(|(index, synapse)| match synapse.kind() {
            CompiledSynapseKind::Decoder(coordinate)
                if coordinate.head() == DecoderHeadKind::ActionCandidate
                    && coordinate.family() == CandidateActionFamily::Approach =>
            {
                Some((index, synapse.source(), coordinate.input_lane()))
            }
            _ => None,
        })
        .unwrap();
    let mask = StageTrainableMask::from_synapse_indices(&phenotype, &[index as u32]).unwrap();
    let mut trainer = FoundationTrainer::new_required(
        phenotype.clone(),
        source.clone(),
        mask,
        AdamWConfig::default(),
    )
    .unwrap();
    eprintln!(
        "packed checkpoint regression adapter: {:?}",
        trainer.hardware_receipt()
    );
    let mut seeded = separate_checkpoint(&trainer).unwrap();
    seeded.optimizer_step = 8;
    for i in 0..seeded.weights.len() {
        let age = (i % 8) as u32;
        seeded.update_ages[i] = age;
        seeded.first_moment[i] = if age == 0 {
            -0.0
        } else {
            ((i % 17) as f32 - 8.0) / 128.0
        };
        seeded.second_moment[i] = if age == 0 {
            -0.0
        } else {
            (1 + i % 19) as f32 / 256.0
        };
    }
    seeded.update_ages[index] = 2;
    seeded.first_moment[index] = -0.125;
    seeded.second_moment[index] = 0.25;
    let frozen = (0..seeded.weights.len())
        .filter(|i| *i != index)
        .take(2)
        .collect::<Vec<_>>();
    seeded.weights[frozen[0]] = -0.0;
    seeded.weights[frozen[1]] = f32::from_bits(1);
    seeded.validate_for(&phenotype, source.digest()).unwrap();
    trainer.restore_checkpoint(&seeded).unwrap();
    let (_, captured) = compare_captures(&trainer);
    assert_exact(&captured, &seeded);

    let neurons = phenotype.neuron_count() as usize;
    let mut activations = vec![0.0; neurons];
    activations[source_neuron as usize] = 0.4;
    let mut decoder_inputs = [0.0; 54];
    decoder_inputs[input_lane as usize] = 1.0;
    let tick = TrainingReplayTick {
        encoded_inputs: vec![0.0; neurons],
        projection_gain: 1.0,
        local_threshold_shift: 0.0,
        microstep_count: 0,
        enabled_routes: vec![false; phenotype.projections().len()],
        effective_weight_offsets: vec![0.0; phenotype.synapses().len()],
        structural_synapses: Vec::new(),
        candidates: vec![TrainingReplayCandidate {
            family: CandidateActionFamily::Approach,
            decoder_inputs,
            innate_bias: 0.0,
        }],
    };
    let mut replay = TrainingSequence {
        phenotype_hash: phenotype.phenotype_hash(),
        initial: TrainingInitialState {
            activations,
            activity_ema: vec![0.0; neurons],
            metabolic_load: vec![0.0; neurons],
            dendrites: Default::default(),
        },
        ticks: vec![tick.clone(); 2],
        burn_in_ticks: 1,
        memory_candidate_gain: phenotype
            .candidate_decoder()
            .memory_channel()
            .map_or(0.0, |p| p.max_candidate_gain()),
    };
    // Enter replay storage, then change its shape before capture/update.
    trainer.prepare_replay(&replay).unwrap();
    assert_eq!(trainer.gpu.layout.ticks, 2);
    assert_exact(&compare_captures(&trainer).1, &seeded);
    replay.ticks.push(tick);
    trainer.prepare_replay(&replay).unwrap();
    assert_eq!(trainer.gpu.layout.ticks, 3);
    assert_exact(&compare_captures(&trainer).1, &seeded);
    let continued = compare_next_update(&mut trainer, &replay, index);

    let next_asset = trainer
        .export_candidate(TrainingStageManifest::new(1, 1, 1))
        .unwrap();
    let (next_phenotype, _) = PhenotypeCompiler::compile_n2048_foundation_candidate(
        genome,
        development,
        next_asset.clone(),
    )
    .unwrap();
    trainer
        .rebind_for_next_cohort(next_phenotype.clone(), next_asset.clone())
        .unwrap();
    let mut rebound = continued;
    rebound.phenotype_hash = next_phenotype.phenotype_hash();
    rebound.source_foundation_digest = next_asset.digest();
    assert_exact(&compare_captures(&trainer).1, &rebound);
    assert!(trainer.prepare_replay(&replay).is_err());
    // Synthetic test-only identity relabel, as in the existing GPU fixture.
    // A real training cohort must recollect under the newly exported asset.
    replay.phenotype_hash = next_phenotype.phenotype_hash();
    trainer.prepare_replay(&replay).unwrap();
    assert_exact(&compare_captures(&trainer).1, &rebound);
    let final_state = compare_next_update(&mut trainer, &replay, index);
    trainer.begin_gradient_accumulation(1).unwrap();
    assert!(trainer.checkpoint().is_err());
    trainer.cancel_gradient_accumulation();
    assert_exact(&compare_captures(&trainer).1, &final_state);
}
