use std::path::Path;

use alife_archive::{ResolvedFounder, ResolvedFounderCohort};
use alife_core::{
    BrainCapacityClass, DevelopmentState, FounderMode, LanguageGroundingLedger, MemoryBankConfig,
    MemorySidecarState, NormalizedScalar, PassiveLifeStatistics, PhenotypeCompiler,
    PhenotypeCompilerInputs, SensorProfileIdentity, SensoryAbiVersion, SleepState, Tick,
    TopologicalMapConfig, TopologySidecar, Validate,
};
use alife_gpu_backend::GpuClosedLoopBackend;
use alife_runtime::{
    merge_gpu_checkpoint_manifest_entries, GpuAuthoritativeSession, GpuBrainCheckpointWrite,
    GpuBrainSidecarCapture, GpuCheckpointAssetStore, GpuSessionConsumerKind,
};
use alife_world::persistence::PortableSaveFile;
use alife_world::{TrackedObjectRegistry, DEFAULT_TRACKED_OBJECT_CAPACITY_PER_ORGANISM};

use crate::GameAppShellError;

/// Materializes every selected founder through the production GPU checkpoint
/// boundary. Genetic founders receive an empty learned state compiled from the
/// archived genome/foundation; explicit mind clones additionally transplant
/// consolidated learning and rebound durable memories.
pub fn materialize_founder_gpu_states(
    backend: GpuClosedLoopBackend,
    mut save: PortableSaveFile,
    asset_root: impl AsRef<Path>,
    cohort: &ResolvedFounderCohort,
) -> Result<PortableSaveFile, GameAppShellError> {
    cohort.manifest.validate_contract()?;
    if save.save_id != cohort.manifest.target_save_id
        || save.deterministic_seed != cohort.manifest.deterministic_seed
        || save.creatures.len() != cohort.founders.len()
    {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: "founder save does not match its resolved cohort".to_string(),
        });
    }
    let asset_root = asset_root.as_ref();
    let store = GpuCheckpointAssetStore::new(asset_root)?;
    let mut session = GpuAuthoritativeSession::new(backend, GpuSessionConsumerKind::Gameplay);
    for founder in &cohort.founders {
        let target = &founder.provenance.remap;
        let write = match founder.selection.mode {
            FounderMode::MindStateClone { .. } => {
                let source = founder.gpu_checkpoint.as_ref().ok_or_else(|| {
                    GameAppShellError::InvalidProductionFrontend {
                        message: "mind-state founder has no validated GPU checkpoint".to_string(),
                    }
                })?;
                store
                    .clone_durable_founder(
                        &mut session,
                        &save.assets,
                        &source.save_state,
                        target.target_organism_id,
                        save.deterministic_seed,
                        save.world.tick,
                    )?
                    .checkpoint
            }
            FounderMode::GeneticFounder | FounderMode::GeneticOffspring { .. } => {
                capture_genetic_founder(
                    &store,
                    &mut session,
                    founder,
                    save.deterministic_seed,
                    save.world.tick,
                )?
            }
        };
        merge_gpu_checkpoint_manifest_entries(&mut save.assets, write.manifest_entries)?;
        let creature = save
            .creatures
            .iter_mut()
            .find(|creature| creature.organism_id == target.target_organism_id)
            .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
                message: "founder creature is missing".to_string(),
            })?;
        let state = write.save_state;
        creature.development_tick = if matches!(
            founder.selection.mode,
            FounderMode::GeneticFounder | FounderMode::GeneticOffspring { .. }
        ) {
            Tick::ZERO
        } else {
            save.world.tick
        };
        creature.mind.tick = save.world.tick;
        creature.mind.homeostasis = alife_core::HomeostaticSnapshot::baseline(save.world.tick);
        creature.mind.memory_record_count = state.memory.summary.record_count;
        creature.mind.concept_count = state.topology.counts.concepts;
        creature.mind.edge_count = state.topology.counts.edges;
        creature.mind.simplex_count = state.topology.counts.simplexes;
        creature.mind.unresolved_gap_count = state.topology.counts.unresolved_gaps;
        creature.mind.sleep_state_label = "awake".to_string();
        creature.weights.lifetime_consolidated_entries =
            if matches!(founder.selection.mode, FounderMode::MindStateClone { .. }) {
                BrainCapacityClass::production_for_id(state.capacity_class_id)?
                    .execution()
                    .max_total_synapses()
            } else {
                0
            };
        creature.weights.h_operational_entries = 0;
        creature.weights.h_shadow_entries = 0;
        creature.learning.last_consolidated_tick =
            matches!(founder.selection.mode, FounderMode::MindStateClone { .. })
                .then_some(save.world.tick);
        creature.gpu_brain = Some(state);
    }
    save.validate_with_asset_root(asset_root)?;
    Ok(save)
}

/// Backward-compatible descriptive name for callers that only selected mind
/// clones. The implementation now also makes genetic founders launch-ready.
pub fn materialize_founder_mind_clones(
    backend: GpuClosedLoopBackend,
    save: PortableSaveFile,
    asset_root: impl AsRef<Path>,
    cohort: &ResolvedFounderCohort,
) -> Result<PortableSaveFile, GameAppShellError> {
    materialize_founder_gpu_states(backend, save, asset_root, cohort)
}

fn capture_genetic_founder(
    store: &GpuCheckpointAssetStore,
    session: &mut GpuAuthoritativeSession,
    founder: &ResolvedFounder,
    world_seed: u64,
    tick: Tick,
) -> Result<GpuBrainCheckpointWrite, GameAppShellError> {
    let target = &founder.provenance.remap;
    let capacity = BrainCapacityClass::production_for_id(founder.genome.brain_class_id)?;
    let development =
        DevelopmentState::new(founder.genome.id, Tick::ZERO, NormalizedScalar::new(0.25)?);
    let phenotype = match &founder.foundation_bytes {
        Some(bytes) => {
            let foundation = alife_core::FoundationWeightAsset::decode_canonical(bytes)?;
            PhenotypeCompiler::compile_from_foundation_asset(
                &founder.genome,
                &capacity,
                &development,
                founder.manifest.genetic.sensor_profile,
                &foundation,
            )?
        }
        None => PhenotypeCompiler::compile(
            &founder.genome,
            &capacity,
            &development,
            founder.manifest.genetic.sensor_profile,
        )?,
    };
    if (!matches!(founder.selection.mode, FounderMode::GeneticOffspring { .. })
        && phenotype.phenotype_hash() != founder.manifest.genetic.phenotype_hash)
        || phenotype.persistent_address_map().digest()
            != founder.manifest.genetic.persistent_address_map_digest
        || phenotype.language_codebook().id() != founder.manifest.genetic.language_codebook_id
        || phenotype.language_codebook().canonical_digest()
            != founder.manifest.genetic.language_codebook_digest
    {
        return Err(alife_core::ScaffoldContractError::PhenotypeCompile.into());
    }
    let compiler_inputs = PhenotypeCompilerInputs::try_new_with_foundation_abi(
        founder.genome.clone(),
        &capacity,
        development,
        founder.manifest.genetic.sensor_profile,
        phenotype.foundation_abi().clone(),
    )?;
    let sensor_profile = SensorProfileIdentity {
        profile_id: founder.manifest.genetic.sensor_profile.into(),
        profile_schema_version: 1,
        sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
    };
    let memory = MemorySidecarState::new_profiled(
        target.target_organism_id,
        sensor_profile,
        MemoryBankConfig::new(256, 64, 4, 0.72, alife_core::Confidence::new(0.0)?)?,
    )?;
    let topology = TopologySidecar::new_profiled(
        target.target_organism_id,
        sensor_profile,
        TopologicalMapConfig::default(),
    )?;
    let tracked_objects =
        TrackedObjectRegistry::new(world_seed, DEFAULT_TRACKED_OBJECT_CAPACITY_PER_ORGANISM)?
            .save_state(target.target_organism_id)?;
    let language_grounding = LanguageGroundingLedger::default();
    let statistics = PassiveLifeStatistics::new(target.target_organism_id, tick)?;
    let handle = session.insert_brain(target.target_organism_id, phenotype.clone())?;
    let write = store.capture_brain(
        session,
        handle,
        &phenotype,
        &compiler_inputs,
        SleepState::awake_at(tick),
        tick,
        None,
        GpuBrainSidecarCapture {
            sensor_profile,
            memory: &memory,
            topology: &topology,
            tracked_objects,
            language_grounding: &language_grounding,
            life_statistics: &statistics,
            retained_learning: None,
        },
    );
    let removal = session.remove_brain(handle);
    let write = write?;
    removal?;
    Ok(write)
}
