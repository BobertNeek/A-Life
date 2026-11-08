#![cfg(feature = "cpu-backend")]
use alife_core::*;
use alife_gpu_backend::{GpuClassBucketPlan, GpuPerceptionUpload};
use alife_runtime::canonical_preparation::*;
use alife_training::cpu::*;
use alife_training::*;
use alife_world::HeadlessScenarioBuilder;
fn prepared() -> (
    BrainPhenotype,
    FoundationWeightAsset,
    PerceptionFrame,
    FinalizedMemoryRecall,
    NeuralReceptorEffects,
) {
    let cap = BrainCapacityClass::n2048_grounded_drive_research();
    let g = BrainGenome::scaffold(0x2048_2026, cap.id());
    let d = DevelopmentState::new(g.id, Tick::ZERO, NormalizedScalar::new(1.).unwrap());
    let p = PhenotypeCompiler::compile_n2048_grounded_drive_basis(
        g.clone(),
        d.clone(),
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap()
    .0;
    let asset = FoundationWeightAsset::from_phenotype_for_genetic_birth(&p).unwrap();
    // Training requires an explicitly admitted asset-bound phenotype.
    let p = PhenotypeCompiler::compile_n2048_grounded_drive_basis_candidate(g, d, asset.clone())
        .unwrap()
        .0;
    asset.validate_against(&p).unwrap();
    // Static observed fixture only; no world step, life, reward or optimizer step.
    let mut world = HeadlessScenarioBuilder::new(61101)
        .agent("observer", OrganismId(1), Vec3f::ZERO)
        .food("object", Vec3f::new(0.4, 0., 0.), 0.6)
        .build()
        .unwrap();
    let draft = world
        .perception_frame_draft(
            OrganismId(1),
            Tick::ZERO,
            p.sensor_profile(),
            HomeostaticSnapshot::baseline(Tick::ZERO),
        )
        .unwrap();
    let bank = MemoryBank::new(
        MemoryBankConfig::new(8, 64, 4, 0.72, Confidence::new(0.).unwrap()).unwrap(),
    )
    .unwrap();
    let recall = bank.recall_frame(&draft).unwrap();
    let topology = TopologySidecar::new_profiled(
        OrganismId(1),
        draft.profile_provenance().identity(),
        TopologicalMapConfig::default(),
    )
    .unwrap();
    let topology_evidence = topology_evidence_for_draft(&draft, &topology).unwrap();
    let mut projection = cognitive_projection_for_draft(
        &draft,
        &recall,
        ExperienceSequenceId(1),
        &predictor_for_phenotype(&p).unwrap(),
        &topology_evidence,
    )
    .unwrap();
    for row in &mut projection.candidates {
        row.prediction.predicted_successor.fill(0.42);
    }
    let context =
        cognitive_memory_context_for_recall(OrganismId(1), ExperienceSequenceId(1), &recall)
            .unwrap();
    let context = cognitive_context_with_projection(context, projection).unwrap();
    let (frame, recall) = recall
        .with_cognitive_context(context)
        .unwrap()
        .finalize(draft)
        .unwrap();
    let effects = NeuralReceptorEffects {
        source_tick: Tick::ZERO,
        source_chemistry_version: 1,
        interoceptive_gain: 1.,
        regional_excitability: 1.,
        projection_gain: 1.,
        local_threshold_shift: 0.,
        attention_gain: 1.,
        plasticity_appetitive: 0.,
        plasticity_aversive: 0.,
        structural_growth_gate: 0.,
        sleep_gate: 0.,
        consolidation_gate: 0.,
    };
    (p, asset, frame, recall, effects)
}
#[test]
fn actual_cpu_encode_overwrites_only_reserved_cognitive_lanes_and_matches_gpu_upload() {
    let (p, _, f, r, e) = prepared();
    let tick = encode_cpu_frame(&p, &f, &r, &e, vec![true; p.projections().len()], vec![]).unwrap();
    let rows = prepare_grounded_drive_features(&f).unwrap();
    let mut bucket =
        GpuClassBucketPlan::new(BrainCapacityClass::n2048_grounded_drive_research(), 1).unwrap();
    let slot = bucket.insert_phenotype(0, 1, &p).unwrap();
    let upload = GpuPerceptionUpload::try_from_frame(&f, &slot, 0).unwrap();
    for (i, c) in tick.candidates.iter().enumerate() {
        assert_eq!(&c.decoder_inputs[..24], &f.candidates()[i].features.0);
        assert_eq!(&c.decoder_inputs[36..46], &rows[i]);
        assert_eq!(&c.decoder_inputs[46..49], &[0.42; 3]);
        assert_eq!(c.decoder_inputs[53], 0.);
        let at = upload.candidates[i].feature_offset as usize;
        assert_eq!(
            &upload.frame_payload_words[at + 36..at + 46],
            &rows[i].map(f32::to_bits)
        );
    }
}
#[test]
fn derived_readout_gradient_matches_finite_difference_without_state_or_optimizer_changes() {
    let (p, s, f, r, e) = prepared();
    let tick = encode_cpu_frame(&p, &f, &r, &e, vec![true; p.projections().len()], vec![]).unwrap();
    let index = tick
        .candidates
        .iter()
        .position(|c| c.decoder_inputs[36] == 1.)
        .unwrap();
    let family = tick.candidates[index].family;
    let id=p.synapses().iter().position(|s|matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate && c.family()==family && c.input_lane()==36 && c.motor_index()==u16::from(family.raw())*16)).unwrap();
    let mask = StageTrainableMask::from_synapse_indices(&p, &[id as u32]).unwrap();
    let trainer =
        CpuFoundationTrainer::new(p.clone(), s.clone(), mask, AdamWConfig::default()).unwrap();
    let before = trainer.checkpoint();
    let n = p.neuron_count() as usize;
    let sequence = TrainingSequence {
        phenotype_hash: p.phenotype_hash(),
        initial: TrainingInitialState {
            activations: vec![0.1; n],
            activity_ema: vec![0.; n],
            metabolic_load: vec![0.; n],
            dendrites: DendriticBranchSet::default(),
        },
        ticks: vec![tick],
        burn_in_ticks: 0,
        memory_candidate_gain: 0.5,
    };
    let mut adjoints = vec![vec![0.; f.candidates().len()]];
    adjoints[0][index] = 1.;
    let analytic = trainer
        .probe_replay_gradients(&sequence, &adjoints)
        .unwrap()
        .gradients[id];
    let eval = |delta: f32| {
        let mut ck = before.clone();
        ck.weights[id] += delta;
        CpuFoundationTrainer::from_checkpoint(p.clone(), s.clone(), ck)
            .unwrap()
            .evaluate_replay(&sequence)
            .unwrap()
            .candidate_logits[0][index]
    };
    let numeric = (eval(0.002) - eval(-0.002)) / 0.004;
    assert!(analytic.abs() > 1e-5);
    assert!(
        (analytic - numeric).abs() < 2e-4,
        "{analytic} versus {numeric}"
    );
    assert_eq!(before, trainer.checkpoint());
}
