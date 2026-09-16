#![cfg(feature = "gpu-tests")]

use alife_core::*;
use alife_gpu_backend::{
    GpuClosedLoopBackend, GpuClosedLoopMemoryBatchInput, GpuClosedLoopMemoryTickInput,
    GpuRuntimeProfile,
};
use alife_runtime::{GpuAuthoritativeSession, GpuSessionConsumerKind};
use alife_training::{Nano512ReadoutTrainer, ProductionReadoutExample};
use alife_world::HeadlessScenarioBuilder;

#[test]
#[ignore = "requires ALIFE_SAMPLING_ASSET pointing to a retained experiment asset"]
fn retained_sampling_asset_compiles_without_relaxing_production_checks() {
    let path = std::env::var("ALIFE_SAMPLING_ASSET").unwrap();
    let asset = FoundationWeightAsset::decode_canonical(&std::fs::read(&path).unwrap()).unwrap();
    let source =
        FoundationWeightAsset::builtin_nano512_v1(asset.manifest().sensor_profile()).unwrap();
    let (baseline, _, _) = PhenotypeCompiler::compile_fixed_legacy_nano512_compatibility_asset(
        asset.manifest().sensor_profile(),
        &source,
    )
    .unwrap()
    .into_runtime_parts();
    let invalid_motor_genes: Vec<_> = baseline.synapses().iter().enumerate().filter(|(i, s)| matches!(s.kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::ActionCandidate) && asset.weights()[*i] < 0.0).map(|(i, s)| (i, s.route_index(), source.weights()[i], asset.weights()[i])).collect();
    println!(
        "negative_motor_genes={}; first={:?}",
        invalid_motor_genes.len(),
        invalid_motor_genes.first()
    );
    println!(
        "retained_asset={path}; digest={:?}; negative_weights={}; minimum={}",
        asset.digest(),
        asset.weights().iter().filter(|w| **w < 0.0).count(),
        asset
            .weights()
            .iter()
            .copied()
            .fold(f32::INFINITY, f32::min)
    );
    PhenotypeCompiler::compile_nano512_readout_candidate(&asset).unwrap();
}

#[test]
fn gpu_readout_training_reduces_loss_and_preserves_frozen_genes() {
    let profile = SensorProfile::GroundedObjectSlotsV1;
    let source = FoundationWeightAsset::builtin_nano512_v1(profile).unwrap();
    let (baseline, _, _) =
        PhenotypeCompiler::compile_fixed_legacy_nano512_compatibility_asset(profile, &source)
            .unwrap()
            .into_runtime_parts();
    let mut weights = source.weights().to_vec();
    let changed = baseline.synapses().iter().position(|s| matches!(s.kind(), CompiledSynapseKind::Decoder(c)
        if c.head() == DecoderHeadKind::ActionCandidate && c.family() == CandidateActionFamily::Approach)).unwrap();
    weights[changed] += 0.125;
    let candidate = FoundationWeightAsset::from_trained_weights(
        &baseline,
        weights.clone(),
        TrainingStageManifest::new(1, 1, 1),
    )
    .unwrap();
    let candidate =
        FoundationWeightAsset::decode_canonical(&candidate.encode_canonical().unwrap()).unwrap();
    let (_, inputs) = PhenotypeCompiler::compile_nano512_readout_candidate(&candidate).unwrap();
    let inputs: PhenotypeCompilerInputs =
        serde_json::from_slice(&serde_json::to_vec(&inputs).unwrap()).unwrap();
    let phenotype =
        PhenotypeCompiler::compile_validated(&inputs, &BrainCapacityClass::n512()).unwrap();
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap();
    println!(
        "hardware={:?}; source={:?}; candidate={:?}; phenotype={:?}",
        backend.hardware_receipt(),
        source.digest(),
        candidate.digest(),
        phenotype.phenotype_hash()
    );
    let mut session = GpuAuthoritativeSession::new(backend, GpuSessionConsumerKind::Training);
    let organism = OrganismId(990512);
    let handle = session.insert_brain(organism, phenotype.clone()).unwrap();
    let mut world = HeadlessScenarioBuilder::new(990513)
        .agent("learner", organism, Vec3f::ZERO)
        .food("observed-object", Vec3f::new(2.0, 0.0, 0.0), 1.0)
        .build()
        .unwrap();
    let genome = CreatureGenome::early_mammal_founder(
        990513,
        FoundationGeneticIdentity::new(
            FoundationId::N512_V1.raw(),
            1,
            FoundationCompatibilityFamilyId::N512_FOUNDATION.raw(),
            BrainCapacityClass::N512_ID,
        )
        .unwrap(),
    )
    .unwrap();
    let body = genome.express().unwrap();
    let chemistry = BiochemistryState::new(&body, world.tick()).unwrap();
    let receptor_frame = chemistry.neural_receptor_frame(&body).unwrap();
    let receptor_effects = NeuralReceptorEffects::from_frame(
        &receptor_frame,
        &NeuralReceptorPhenotype::compile(&phenotype).unwrap(),
    )
    .unwrap();
    let entity = world.organism_entity_ids()[0].1;
    world
        .register_organism_record(
            alife_world::WorldOrganismRecord::new(
                organism,
                entity,
                genome,
                body,
                chemistry,
                world.tick(),
            )
            .unwrap(),
        )
        .unwrap();
    let memory = MemorySidecarState::new_profiled(
        organism,
        SensorProfileIdentity {
            profile_id: profile.into(),
            profile_schema_version: 1,
            sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
        },
        MemoryBankConfig::new(64, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap(),
    )
    .unwrap();
    let draft = world
        .perception_frame_draft(
            organism,
            world.tick(),
            profile,
            HomeostaticSnapshot::baseline(world.tick()),
        )
        .unwrap();
    let recall = memory.recall_frame(&draft).unwrap();
    let (frame, recall) = recall.finalize(draft).unwrap();
    let index = frame
        .candidates()
        .iter()
        .position(|c| c.family == CandidateActionFamily::Approach)
        .unwrap() as u16;
    let rest = frame
        .candidates()
        .iter()
        .position(|c| c.family == CandidateActionFamily::Rest)
        .unwrap() as u16;
    let upload = session
        .prepare_memory_context_upload(handle, &frame, &recall)
        .unwrap()
        .bind_neural_receptor_effects(receptor_effects)
        .unwrap();
    let member = GpuClosedLoopMemoryTickInput::try_new(handle, &frame, &upload).unwrap();
    let batch = GpuClosedLoopMemoryBatchInput::try_new(vec![member]).unwrap();
    let mut requested = [index, rest];
    requested.sort_unstable();
    let mut ticks = session
        .tick_memory_batch_with_selector_diagnostics(&batch, &requested)
        .unwrap();
    let tick = ticks.pop().unwrap();
    let diagnostic = tick.selector_diagnostic.unwrap();
    assert_eq!(diagnostic.phenotype_hash, phenotype.phenotype_hash());
    assert_eq!(diagnostic.frame_digest, frame.frame_digest());
    let row = diagnostic
        .candidates
        .iter()
        .find(|row| row.candidate_index == index)
        .unwrap();
    assert!(!row.contributions.is_empty());
    assert!(row
        .contributions
        .iter()
        .any(|r| r.global_synapse_id as usize == changed));
    assert_eq!(row.memory_context_delta, Some(0.0));
    for contribution in &row.contributions {
        assert_eq!(
            contribution.genetic.to_bits(),
            weights[contribution.global_synapse_id as usize].to_bits()
        );
        assert_eq!(contribution.lifetime, 0.0);
        assert_eq!(contribution.fast, 0.0);
    }
    assert!(row.final_logit.unwrap().is_finite());
    println!(
        "frame={:?}; candidate_index={index}; observed_rows={}; logit={:?}; motor_slots={:?}",
        frame.frame_digest(),
        row.contributions.len(),
        row.final_logit,
        tick.factorized_motor_candidates
    );
    let example =
        ProductionReadoutExample::from_diagnostic(&phenotype, &frame, &diagnostic, rest, index)
            .unwrap();
    let constant = frame
        .candidates()
        .iter()
        .find(|candidate| {
            phenotype
                .candidate_decoder()
                .families()
                .iter()
                .any(|family| {
                    family.family() == candidate.family && family.decoder_synapse_count() == 0
                })
        })
        .unwrap();
    assert_eq!(constant.family, CandidateActionFamily::Other);
    assert!(!diagnostic
        .requested_candidate_indices
        .contains(&constant.candidate_index));
    let constant_row = &diagnostic.candidates[usize::from(constant.candidate_index)];
    assert!(constant_row.contributions.is_empty());
    assert_eq!(
        constant_row.final_logit,
        Some(constant_row.decoder_family_bias)
    );
    ProductionReadoutExample::from_diagnostic(
        &phenotype,
        &frame,
        &diagnostic,
        rest,
        constant.candidate_index,
    )
    .unwrap();
    let mut malformed = diagnostic.clone();
    malformed
        .candidates
        .iter_mut()
        .find(|row| row.candidate_index == index)
        .unwrap()
        .contributions
        .pop();
    assert!(
        ProductionReadoutExample::from_diagnostic(&phenotype, &frame, &malformed, rest, index)
            .is_err()
    );
    let mut trainer =
        Nano512ReadoutTrainer::new_required(baseline.clone(), candidate.clone(), &[example], 0.05)
            .unwrap();
    let before = trainer.loss().unwrap();
    let preferred_logit = diagnostic
        .candidates
        .iter()
        .find(|row| row.candidate_index == rest)
        .unwrap()
        .final_logit
        .unwrap();
    let rejected_logit = row.final_logit.unwrap();
    let difference = rejected_logit - preferred_logit;
    let expected_loss = difference.max(0.0) + (-difference.abs()).exp().ln_1p();
    assert!((before - expected_loss).abs() < 0.0001 + expected_loss.abs() * 0.00001,
        "optimizer pair order/coefficients must match production logits: {before} vs {expected_loss}");
    trainer.train_steps(64).unwrap();
    let after = trainer.loss().unwrap();
    assert!(
        after < before * 0.5,
        "production-readout pair loss must fall: {before} -> {after}"
    );
    let trained = trainer
        .export_candidate(TrainingStageManifest::new(2, 1, 1))
        .unwrap();
    let mut changed_readout = false;
    for (index, synapse) in baseline.synapses().iter().enumerate() {
        if matches!(synapse.kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::ActionCandidate)
        {
            assert!(
                trained.weights()[index] >= 0.0,
                "MotorProposal genetic sign must remain nonnegative"
            );
            changed_readout |= trained.weights()[index].to_bits() != weights[index].to_bits();
        } else {
            assert_eq!(trained.weights()[index].to_bits(), weights[index].to_bits());
        }
    }
    assert!(changed_readout);
    FoundationWeightAsset::decode_canonical(&trained.encode_canonical().unwrap()).unwrap();
    PhenotypeCompiler::compile_nano512_readout_candidate(&trained).unwrap();
    println!(
        "readout_loss={before}->{after}; trained={:?}",
        trained.digest()
    );
}
