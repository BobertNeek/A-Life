#![cfg(feature = "gpu-tests")]

use alife_core::*;
use alife_gpu_backend::{
    GpuClosedLoopBackend, GpuClosedLoopMemoryBatchInput, GpuClosedLoopMemoryTickInput,
    GpuRuntimeProfile,
};
use alife_runtime::{GpuAuthoritativeSession, GpuSessionConsumerKind};
use alife_world::HeadlessScenarioBuilder;

#[test]
fn canonical_candidate_reaches_production_gpu_decoder_with_exact_genes() {
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
    let upload = session
        .prepare_memory_context_upload(handle, &frame, &recall)
        .unwrap()
        .bind_neural_receptor_effects(receptor_effects)
        .unwrap();
    let member = GpuClosedLoopMemoryTickInput::try_new(handle, &frame, &upload).unwrap();
    let batch = GpuClosedLoopMemoryBatchInput::try_new(vec![member]).unwrap();
    let mut ticks = session
        .tick_memory_batch_with_selector_diagnostics(&batch, &[index])
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
}
