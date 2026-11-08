#![cfg(feature = "cpu-backend")]

//! CPU/native absent-speech receipt parity, independent of memory-head semantics.
use alife_core::*;
use alife_training::{cpu::*, AdamWConfig, StageTrainableMask};
use alife_world::*;

fn nano_recipe() -> Nano512ActionCreditCandidateV2 {
    let profile = SensorProfile::GroundedObjectSlotsV1;
    let builtin = FoundationWeightAsset::builtin_nano512_v1(profile).unwrap();
    let (p, _, _) =
        PhenotypeCompiler::compile_fixed_legacy_nano512_compatibility_asset(profile, &builtin)
            .unwrap()
            .into_runtime_parts();
    let source = FoundationWeightAsset::from_nano512_readout_candidate(
        &p,
        p.synapses().iter().map(|s| s.genetic_weight()).collect(),
        TrainingStageManifest::new(1, 1, 1),
    )
    .unwrap();
    Nano512ActionCreditCandidateV2::new(
        &source,
        ActionCandidateCreditProfileV1::SignedChoiceReadouts,
    )
    .unwrap()
}

fn trainer(p: BrainPhenotype, source: FoundationWeightAsset) -> CpuFoundationTrainer {
    let mask = StageTrainableMask::recurrent_only(&p).unwrap();
    CpuFoundationTrainer::new(p, source, mask, AdamWConfig::default()).unwrap()
}

fn nano_trainer(recipe: &Nano512ActionCreditCandidateV2) -> CpuFoundationTrainer {
    let (p, _) = PhenotypeCompiler::compile_nano512_action_credit_candidate(recipe).unwrap();
    trainer(p, recipe.asset().unwrap())
}

fn speech_trainer() -> CpuFoundationTrainer {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(0x7A11, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.8).unwrap());
    let source =
        FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    let p = PhenotypeCompiler::compile_from_foundation_asset(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
        &source,
    )
    .unwrap();
    trainer(p, source)
}

#[test]
fn absent_speech_head_returns_no_receipt_without_mutating_founder() {
    let base = nano_recipe();
    let recipe = base;
    let founder = nano_trainer(&recipe);
    let p = founder.phenotype();
    assert!(p.speech_decoder().is_none());
    assert!(!p.synapses().iter().any(|s| matches!(s.kind(),
        CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::SpeechPayload)));
    let before = serde_json::to_value(founder.checkpoint()).unwrap();
    let activations = vec![0.2; p.neuron_count() as usize];
    let offsets = vec![0.; p.synapses().len()];
    assert_eq!(
        decode_cpu_speech(&founder, &activations, &offsets).unwrap(),
        None
    );
    assert!(decode_cpu_speech(&founder, &activations[..activations.len() - 1], &offsets).is_err());
    assert!(decode_cpu_speech(&founder, &activations, &offsets[..offsets.len() - 1]).is_err());
    assert_eq!(serde_json::to_value(founder.checkpoint()).unwrap(), before);
}

#[test]
fn absent_speech_receipt_preserves_selected_vocal_action_and_world_failure() {
    let recipe = nano_recipe();
    let founder = nano_trainer(&recipe);
    let p = founder.phenotype();
    let receipt = decode_cpu_speech(
        &founder,
        &vec![0.2; p.neuron_count() as usize],
        &vec![0.; p.synapses().len()],
    )
    .unwrap();
    let mut game = create_canonical_new_game_with_nociceptive_candidate(
        &CanonicalNewGameConfig::phase3(41, 1).unwrap(),
        &recipe,
    )
    .unwrap();
    let id = OrganismId(1);
    let entity = game
        .world
        .organism_registry()
        .get(id)
        .unwrap()
        .world_entity_id();
    let command = HeadlessWorldCommand::vocalize(id).unwrap();
    let bundle = arbitrate_gpu_selected_command_into_factorized_bundle(
        id,
        ExperienceSequenceId(1),
        game.world.tick(),
        vec![],
        &command,
        receipt.as_ref(),
        false,
    )
    .unwrap();
    let selected = JointActionSelectionV1::new([0, 0, 0, 1, 0, 0], &bundle).unwrap();
    assert_eq!(selected.candidate_slots(), [0, 0, 0, 1, 0, 0]);
    assert_eq!(bundle.channels.len(), 1);
    assert_eq!(bundle.channels[0].channel, MotorChannel::Vocal);
    assert_eq!(bundle.channels[0].primitive, command.action_id);
    assert!(bundle.channels[0].payload.values.is_empty());
    let before_speech = game.world.audible_utterances();
    let result = game
        .world
        .apply_registered_motor_bundle(&bundle, entity)
        .unwrap();
    assert!(!result.succeeded);
    assert_eq!(game.world.audible_utterances(), before_speech);
    assert!(game.world.entity_id("voice-token-7").is_none());
    selected.validate_bundle(&bundle).unwrap();
}

#[test]
fn present_speech_head_keeps_shape_and_finite_guards() {
    let founder = speech_trainer();
    let p = founder.phenotype();
    assert_eq!(p.speech_decoder().unwrap().decoder_synapse_count(), 1024);
    let mut activations = vec![0.; p.neuron_count() as usize];
    let offsets = vec![0.; p.synapses().len()];
    assert_eq!(
        decode_cpu_speech(&founder, &activations, &offsets).unwrap(),
        None
    );
    assert!(decode_cpu_speech(&founder, &activations[..activations.len() - 1], &offsets).is_err());
    assert!(decode_cpu_speech(&founder, &activations, &offsets[..offsets.len() - 1]).is_err());
    activations[p.candidate_decoder().motor_start() as usize
        + SpeechDecoderLayoutV1::MOTOR_SOURCE_OFFSET as usize] = f32::NAN;
    assert!(decode_cpu_speech(&founder, &activations, &offsets).is_err());
}

#[test]
fn partial_or_unbound_speech_head_cannot_be_admitted_as_absent() {
    let founder = speech_trainer();
    let p = founder.phenotype();
    let first = p
        .synapses()
        .iter()
        .position(|s| {
            matches!(s.kind(),
        CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::SpeechPayload)
        })
        .unwrap();
    let original = serde_json::to_value(p).unwrap();
    let mut partial = original.clone();
    partial["synapses"].as_array_mut().unwrap().remove(first);
    assert!(serde_json::from_value::<BrainPhenotype>(partial).is_err());
    let mut unbound = original.clone();
    unbound["speech_decoder"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<BrainPhenotype>(unbound).is_err());
    let mut malformed = original;
    malformed["speech_decoder"]["decoder_synapse_count"] = serde_json::json!(1023);
    assert!(serde_json::from_value::<BrainPhenotype>(malformed).is_err());
}
