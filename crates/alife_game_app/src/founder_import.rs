use std::path::Path;

use alife_archive::{ResolvedFounder, ResolvedFounderCohort};
use alife_core::{
    BrainCapacityClass, DevelopmentState, FounderMode, LanguageGroundingLedger, MemoryBankConfig,
    MemorySidecarState, NormalizedScalar, PassiveLifeStatistics, PhenotypeCompiler,
    PhenotypeCompilerInputs, ScaffoldContractError, SensorProfileIdentity, SensoryAbiVersion,
    SleepState, Tick, TopologicalMapConfig, TopologySidecar, Validate,
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
    save.validate_with_asset_root(asset_root)?;
    if cohort.founders.len() != cohort.manifest.founders.len()
        || cohort
            .founders
            .iter()
            .zip(&cohort.manifest.founders)
            .any(|(founder, provenance)| founder.provenance != *provenance)
    {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: "resolved founder order differs from cohort provenance".to_string(),
        });
    }
    for founder in &cohort.founders {
        founder.manifest.validate_contract()?;
        founder.genome.validate_contract()?;
    }
    let save_organism_ids = save
        .creatures
        .iter()
        .map(|creature| creature.organism_id)
        .collect::<Vec<_>>();
    let founder_organism_ids = cohort
        .founders
        .iter()
        .map(|founder| founder.provenance.remap.target_organism_id)
        .collect::<Vec<_>>();
    if !same_organism_id_set(&save_organism_ids, &founder_organism_ids) {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: "founder targets do not exactly cover save creatures".to_string(),
        });
    }
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

fn same_organism_id_set(left: &[alife_core::OrganismId], right: &[alife_core::OrganismId]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut left = left.iter().map(|id| id.raw()).collect::<Vec<_>>();
    let mut right = right.iter().map(|id| id.raw()).collect::<Vec<_>>();
    left.sort_unstable();
    right.sort_unstable();
    if left.windows(2).any(|ids| ids[0] == ids[1]) || right.windows(2).any(|ids| ids[0] == ids[1]) {
        return false;
    }
    left == right
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
        phenotype
            .foundation_abi()
            .canonical_v2()
            .cloned()
            .ok_or(ScaffoldContractError::PhenotypeCompile)?,
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
            legacy_nano512_compatibility_receipt: None,
            retained_learning: None,
        },
    );
    let removal = session.remove_brain(handle);
    let write = write?;
    removal?;
    Ok(write)
}

#[cfg(test)]
mod tests {
    use alife_core::OrganismId;

    use super::same_organism_id_set;

    #[test]
    fn founder_targets_must_exactly_cover_save_creatures() {
        assert!(same_organism_id_set(
            &[OrganismId(1), OrganismId(2)],
            &[OrganismId(2), OrganismId(1)]
        ));
        assert!(!same_organism_id_set(
            &[OrganismId(1), OrganismId(2)],
            &[OrganismId(1), OrganismId(1)]
        ));
        assert!(!same_organism_id_set(
            &[OrganismId(1)],
            &[OrganismId(1), OrganismId(2)]
        ));
    }
}
