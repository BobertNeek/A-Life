use std::path::Path;

use alife_archive::{ArchiveError, LineageLibrary};
use alife_core::{
    BiochemistryState, Blake3Digest, BrainPhenotype, FoundationGeneticIdentity, GenomeId,
    LineageId, N512FounderProjectionReceipt, OrganismId, PhenotypeHash, ScaffoldContractError,
    SensorProfile, Tick, Validate, WorldEntityId,
};
use alife_world::persistence::{GpuRuntimeSafeCheckpoint, PersistenceError, PortableSaveFile};
use alife_world::{HeadlessWorld, OrganismRegistryError, WorldObjectKind, WorldOrganismRecord};
use thiserror::Error;

use crate::{
    curated_founder_materializer::{CuratedFounderBundle, CuratedFounderBundleEntry},
    CuratedFounderPlan, CuratedFounderResetError, CuratedFounderResetReceipt,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CuratedFounderSaveReplacementMetadata {
    pub(crate) save_id: String,
    pub(crate) deterministic_seed: u64,
    pub(crate) world_seed: u64,
    pub(crate) world_tick: Tick,
    pub(crate) registry_persistence_deferred: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CuratedFounderArchiveBirthIntent {
    pub(crate) source_run_id: String,
    pub(crate) organism_id: OrganismId,
    pub(crate) genome_id: GenomeId,
    pub(crate) lineage_id: LineageId,
    pub(crate) birth_tick: Tick,
    pub(crate) foundation: FoundationGeneticIdentity,
    pub(crate) foundation_content_digest: Blake3Digest,
    pub(crate) sensor_profile: SensorProfile,
    pub(crate) projection_receipt: N512FounderProjectionReceipt,
    pub(crate) phenotype_hash: PhenotypeHash,
    pub(crate) compiled_phenotype: BrainPhenotype,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CuratedFounderResetStage {
    pub(crate) source_save_id: String,
    pub(crate) source_save_identity: String,
    pub(crate) deterministic_seed: u64,
    pub(crate) world_seed: u64,
    pub(crate) restored_tick: Tick,
    pub(crate) safe_checkpoint: Option<GpuRuntimeSafeCheckpoint>,
    pub(crate) receipt: CuratedFounderResetReceipt,
    pub(crate) ordered_founder_ids: Vec<OrganismId>,
    pub(crate) record_candidates: Vec<WorldOrganismRecord>,
    pub(crate) archive_birth_intents: Vec<CuratedFounderArchiveBirthIntent>,
    pub(crate) target_agent_bindings: Vec<(WorldEntityId, OrganismId)>,
    pub(crate) expected_registry_identity: Vec<(OrganismId, WorldEntityId)>,
    pub(crate) save_replacement: CuratedFounderSaveReplacementMetadata,
}

#[derive(Debug, Error)]
pub(crate) enum CuratedFounderStagingError {
    #[error("curated founder plan validation failed: {0}")]
    Plan(#[from] CuratedFounderResetError),
    #[error("curated founder bundle mismatch at slot {slot:?}: {field}")]
    BundleMismatch {
        slot: Option<u32>,
        field: &'static str,
    },
    #[error("curated founder {field} validation failed: {source}")]
    Contract {
        field: &'static str,
        #[source]
        source: ScaffoldContractError,
    },
    #[error("curated founder world validation failed: {0}")]
    World(#[from] ScaffoldContractError),
    #[error("curated founder record validation failed at slot {slot}: {source}")]
    Record {
        slot: u32,
        #[source]
        source: OrganismRegistryError,
    },
    #[error("curated founder save preflight failed: {0}")]
    Save(#[from] PersistenceError),
    #[error("curated founder archive preflight failed: {0}")]
    Archive(#[from] ArchiveError),
    #[error("curated founder staging mismatch: {field}")]
    Mismatch { field: &'static str },
    #[error("restored organism registry is not empty: {records} record(s)")]
    ExistingRegistry { records: usize },
    #[error("archive run ID is not portable: {value}")]
    InvalidArchiveRunId { value: String },
    #[error("an archive manifest already exists for organism {organism_id:?}")]
    ArchiveConflict { organism_id: OrganismId },
}

pub(crate) fn stage_curated_founder_reset(
    plan: &CuratedFounderPlan,
    bundle: &CuratedFounderBundle,
    source_save: &PortableSaveFile,
    restored_world: &HeadlessWorld,
    lineage_library: &LineageLibrary,
    asset_root: &Path,
    archive_run_id: &str,
) -> Result<CuratedFounderResetStage, CuratedFounderStagingError> {
    plan.validate()?;
    validate_bundle(plan, bundle)?;
    validate_archive_run_id(archive_run_id)?;

    source_save.validate_with_asset_root(asset_root)?;
    if source_save.save_id != plan.source_save_identity {
        return Err(CuratedFounderStagingError::Mismatch {
            field: "source save ID",
        });
    }
    if source_save.deterministic_seed != plan.source_save_seed {
        return Err(CuratedFounderStagingError::Mismatch {
            field: "source save deterministic seed",
        });
    }
    if source_save.world.seed != plan.world_seed {
        return Err(CuratedFounderStagingError::Mismatch {
            field: "source save world seed",
        });
    }
    if source_save.world.tick != plan.restored_tick {
        return Err(CuratedFounderStagingError::Mismatch {
            field: "source save restored tick",
        });
    }
    if restored_world.seed() != source_save.world.seed {
        return Err(CuratedFounderStagingError::Mismatch {
            field: "restored world seed",
        });
    }
    if restored_world.tick() != source_save.world.tick {
        return Err(CuratedFounderStagingError::Mismatch {
            field: "restored world tick",
        });
    }
    if let Some(gpu_runtime) = &source_save.gpu_runtime {
        if gpu_runtime.last_safe_checkpoint.save_id != source_save.save_id {
            return Err(CuratedFounderStagingError::Mismatch {
                field: "safe checkpoint save ID",
            });
        }
        if gpu_runtime.last_safe_checkpoint.world_tick != plan.restored_tick {
            return Err(CuratedFounderStagingError::Mismatch {
                field: "safe checkpoint world tick",
            });
        }
    }

    restored_world.validate_organism_bindings()?;
    if !restored_world.organism_registry().is_empty() {
        return Err(CuratedFounderStagingError::ExistingRegistry {
            records: restored_world.organism_registry().len(),
        });
    }
    for entry in &plan.entries {
        let object = restored_world.entity(entry.world_entity_id).ok_or(
            CuratedFounderStagingError::Mismatch {
                field: "restored target world entity",
            },
        )?;
        if object.kind != WorldObjectKind::Agent {
            return Err(CuratedFounderStagingError::Mismatch {
                field: "restored target entity kind",
            });
        }
        if object.organism_id != Some(entry.organism_id) {
            return Err(CuratedFounderStagingError::Mismatch {
                field: "restored target organism binding",
            });
        }
    }

    for entry in &plan.entries {
        if lineage_library
            .latest_manifest_for(archive_run_id, entry.organism_id)?
            .is_some()
        {
            return Err(CuratedFounderStagingError::ArchiveConflict {
                organism_id: entry.organism_id,
            });
        }
    }

    let mut record_candidates = Vec::with_capacity(bundle.entries.len());
    let mut archive_birth_intents = Vec::with_capacity(bundle.entries.len());
    for (index, entry) in bundle.entries.iter().enumerate() {
        let plan_entry = &plan.entries[index];
        validate_bundle_entry(plan, entry, plan_entry, index as u32)?;
        let record_biochemistry = BiochemistryState::new(&entry.phenotype, plan.restored_tick)
            .map_err(|source| CuratedFounderStagingError::Contract {
                field: "fresh record biochemistry",
                source,
            })?;
        let record = WorldOrganismRecord::new(
            entry.plan_entry.organism_id,
            entry.plan_entry.world_entity_id,
            entry.genome.clone(),
            entry.phenotype.clone(),
            record_biochemistry,
            plan.restored_tick,
        )
        .map_err(|source| CuratedFounderStagingError::Record {
            slot: index as u32,
            source,
        })?;
        if record.birth_tick() != plan.restored_tick
            || record.archive().birth_manifest_digest().is_some()
        {
            return Err(CuratedFounderStagingError::BundleMismatch {
                slot: Some(index as u32),
                field: "unlinked record candidate",
            });
        }
        record
            .validate_contract()
            .map_err(|source| CuratedFounderStagingError::Contract {
                field: "record candidate",
                source,
            })?;
        record_candidates.push(record);

        archive_birth_intents.push(CuratedFounderArchiveBirthIntent {
            source_run_id: archive_run_id.to_string(),
            organism_id: entry.plan_entry.organism_id,
            genome_id: entry.plan_entry.genome_id,
            lineage_id: entry.plan_entry.lineage_id,
            birth_tick: plan.restored_tick,
            foundation: plan.foundation,
            foundation_content_digest: plan.foundation_content_digest,
            sensor_profile: plan.sensor_profile,
            projection_receipt: entry.projection.receipt().clone(),
            phenotype_hash: entry.projection.receipt().phenotype_hash(),
            compiled_phenotype: entry.projection.compiled_phenotype().clone(),
        });
    }

    Ok(CuratedFounderResetStage {
        source_save_id: source_save.save_id.clone(),
        source_save_identity: plan.source_save_identity.clone(),
        deterministic_seed: source_save.deterministic_seed,
        world_seed: source_save.world.seed,
        restored_tick: plan.restored_tick,
        safe_checkpoint: source_save
            .gpu_runtime
            .as_ref()
            .map(|gpu_runtime| gpu_runtime.last_safe_checkpoint.clone()),
        receipt: plan.receipt.clone(),
        ordered_founder_ids: plan.entries.iter().map(|entry| entry.organism_id).collect(),
        record_candidates,
        archive_birth_intents,
        target_agent_bindings: plan
            .entries
            .iter()
            .map(|entry| (entry.world_entity_id, entry.organism_id))
            .collect(),
        expected_registry_identity: plan
            .entries
            .iter()
            .map(|entry| (entry.organism_id, entry.world_entity_id))
            .collect(),
        save_replacement: CuratedFounderSaveReplacementMetadata {
            save_id: source_save.save_id.clone(),
            deterministic_seed: source_save.deterministic_seed,
            world_seed: source_save.world.seed,
            world_tick: source_save.world.tick,
            registry_persistence_deferred: true,
        },
    })
}

fn validate_archive_run_id(archive_run_id: &str) -> Result<(), CuratedFounderStagingError> {
    if archive_run_id.trim().is_empty()
        || archive_run_id.chars().count() > 96
        || !archive_run_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(CuratedFounderStagingError::InvalidArchiveRunId {
            value: archive_run_id.to_string(),
        });
    }
    Ok(())
}

fn validate_bundle(
    plan: &CuratedFounderPlan,
    bundle: &CuratedFounderBundle,
) -> Result<(), CuratedFounderStagingError> {
    if bundle.identity.plan_receipt != plan.receipt {
        return Err(CuratedFounderStagingError::BundleMismatch {
            slot: None,
            field: "plan receipt",
        });
    }
    if bundle.entries.len() != plan.entries.len() {
        return Err(CuratedFounderStagingError::BundleMismatch {
            slot: None,
            field: "entry count",
        });
    }
    for (index, (entry, plan_entry)) in bundle.entries.iter().zip(&plan.entries).enumerate() {
        let slot = Some(index as u32);
        if entry.plan_entry != *plan_entry {
            return Err(CuratedFounderStagingError::BundleMismatch {
                slot,
                field: "plan entry",
            });
        }
    }
    Ok(())
}

fn validate_bundle_entry(
    plan: &CuratedFounderPlan,
    entry: &CuratedFounderBundleEntry,
    plan_entry: &crate::CuratedFounderPlanEntry,
    slot: u32,
) -> Result<(), CuratedFounderStagingError> {
    let slot = Some(slot);
    entry
        .genome
        .validate_contract()
        .map_err(|source| CuratedFounderStagingError::Contract {
            field: "bundle genome",
            source,
        })?;
    if entry.genome.id != plan_entry.genome_id
        || entry.genome.lineage_id != plan_entry.lineage_id
        || entry.genome.conception_seed != plan_entry.conception_seed
    {
        return Err(CuratedFounderStagingError::BundleMismatch {
            slot,
            field: "genome identity",
        });
    }
    let expected_phenotype =
        entry
            .genome
            .express()
            .map_err(|source| CuratedFounderStagingError::Contract {
                field: "bundle genome expression",
                source,
            })?;
    if entry.phenotype != expected_phenotype
        || entry.phenotype.source_genome_id != plan_entry.genome_id
        || entry.phenotype.lineage_id != plan_entry.lineage_id
    {
        return Err(CuratedFounderStagingError::BundleMismatch {
            slot,
            field: "phenotype identity",
        });
    }
    entry.biochemistry.validate_contract().map_err(|source| {
        CuratedFounderStagingError::Contract {
            field: "bundle biochemistry",
            source,
        }
    })?;
    if entry.biochemistry.source_genome_id != plan_entry.genome_id
        || entry.biochemistry.tick != plan.restored_tick
        || entry.biochemistry.homeostasis.tick != plan.restored_tick
    {
        return Err(CuratedFounderStagingError::BundleMismatch {
            slot,
            field: "per-entry biochemistry identity or tick",
        });
    }
    entry
        .projection
        .validate()
        .map_err(|source| CuratedFounderStagingError::Contract {
            field: "bundle projection",
            source,
        })?;
    entry
        .projection
        .receipt()
        .validate_against_projection(&entry.projection)
        .map_err(|source| CuratedFounderStagingError::Contract {
            field: "bundle projection receipt",
            source,
        })?;
    if entry.projection.source_genome_id() != plan_entry.genome_id
        || entry.projection.lineage_id() != plan_entry.lineage_id
        || entry.projection.foundation() != &plan.foundation
        || entry.projection.sensor_profile() != plan.sensor_profile
        || entry.projection.foundation_asset_digest() != plan.foundation_content_digest
        || entry.projection.source_brain_genome() != &entry.phenotype.brain_genome
        || entry.projection.genetic_provenance() != &entry.phenotype.genetic_provenance
        || entry.projection.receipt().source_genome_id() != plan_entry.genome_id
        || entry.projection.receipt().lineage_id() != plan_entry.lineage_id
        || entry.projection.compiled_phenotype().phenotype_hash()
            != entry.projection.receipt().phenotype_hash()
    {
        return Err(CuratedFounderStagingError::BundleMismatch {
            slot,
            field: "projection identity or phenotype",
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        fs,
        path::{Path, PathBuf},
    };

    use alife_archive::{GeneticArchiveInput, LineageLibrary, LineageLibraryConfig};
    use alife_core::{
        BiochemistryState, BrainCapacityClass, BrainScaleTier, FoundationGeneticIdentity,
        FoundationWeightAsset, GenomeId, OrganismId, SensorProfile, Tick, Vec3f, WorldEntityId,
    };
    use alife_world::{
        persistence::{
            AssetKind, AssetManifest, AssetManifestEntry, AssetPresence,
            GpuRuntimeActiveProfileCaps, GpuRuntimeAdapterIdentity, GpuRuntimeAuthorityState,
            GpuRuntimeClassBucketAllocation, GpuRuntimeResidencySlots, GpuRuntimeSafeCheckpoint,
            GpuRuntimeSaveState, GpuRuntimeShaderAbiVersions, PortableAssetDigest,
            PortableSaveFile, RuntimeConfig, FVR06_GPU_RUNTIME_STATE_SCHEMA,
            FVR06_GPU_RUNTIME_STATE_SCHEMA_VERSION,
        },
        HeadlessScenarioBuilder, HeadlessWorld, WorldOrganismRecord,
    };

    use crate::{
        curated_founder_materializer::{materialize_curated_founder_bundle, CuratedFounderBundle},
        plan_curated_founder_reset, CuratedFounderAgentInput, CuratedFounderPlan,
        CuratedFounderResetRequest, CURATED_FOUNDER_RESET_POLICY,
    };

    use super::{stage_curated_founder_reset, CuratedFounderStagingError};

    const WORLD_SEED: u64 = 0x5555_6666_7777_8888;
    const WORLD_ENTITY_IDS: [u64; 3] = [1, 2, 3];
    const ORGANISM_IDS: [u64; 3] = [101, 202, 303];

    fn stage_plan() -> CuratedFounderPlan {
        let sensor_profile = SensorProfile::PrivilegedAffordanceV1;
        let foundation_asset = FoundationWeightAsset::builtin_nano512_v1(sensor_profile).unwrap();
        let manifest = foundation_asset.manifest();
        let foundation = FoundationGeneticIdentity::new(
            manifest.foundation_id().raw(),
            manifest.foundation_version().raw() as u16,
            manifest.compatibility_family_id().raw(),
            BrainCapacityClass::N512_ID,
        )
        .unwrap();
        let final_agents = (0..3)
            .rev()
            .map(|slot| CuratedFounderAgentInput {
                world_entity_id: WorldEntityId(WORLD_ENTITY_IDS[slot as usize]),
                organism_id: Some(OrganismId(ORGANISM_IDS[slot as usize])),
                final_population_slot: slot,
                legacy_genome_id: None,
            })
            .collect();

        plan_curated_founder_reset(&CuratedFounderResetRequest {
            policy_label: Some(CURATED_FOUNDER_RESET_POLICY.to_string()),
            source_save_identity: "save-stage-3a".to_string(),
            source_save_label: "stage test source".to_string(),
            source_save_seed: WORLD_SEED,
            world_seed: WORLD_SEED,
            restored_tick: Tick::ZERO,
            target_population: 3,
            sensor_profile,
            foundation,
            foundation_content_digest: foundation_asset.digest(),
            source_run_identity: "source-run-stage".to_string(),
            final_agents,
        })
        .unwrap()
    }

    fn archive_snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn visit(root: &Path, current: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in fs::read_dir(current).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    visit(root, &path, files);
                } else {
                    files.insert(
                        path.strip_prefix(root).unwrap().to_path_buf(),
                        fs::read(path).unwrap(),
                    );
                }
            }
        }

        let mut files = BTreeMap::new();
        visit(root, root, &mut files);
        files
    }

    fn registry_snapshot(world: &HeadlessWorld) -> Vec<Vec<u8>> {
        let mut records = world
            .organism_registry()
            .iter()
            .map(|record| serde_json::to_vec(record).unwrap())
            .collect::<Vec<_>>();
        records.sort();
        records
    }

    struct StageFixture {
        plan: CuratedFounderPlan,
        bundle: CuratedFounderBundle,
        source_save: PortableSaveFile,
        restored_world: HeadlessWorld,
        lineage_library: Option<LineageLibrary>,
        archive_root: PathBuf,
    }

    impl StageFixture {
        fn lineage_library(&self) -> &LineageLibrary {
            self.lineage_library.as_ref().unwrap()
        }

        fn lineage_library_mut(&mut self) -> &mut LineageLibrary {
            self.lineage_library.as_mut().unwrap()
        }
    }

    impl Drop for StageFixture {
        fn drop(&mut self) {
            let _ = self.lineage_library.take();
            let _ = fs::remove_dir_all(&self.archive_root);
        }
    }

    fn stage_fixture(label: &str) -> StageFixture {
        let plan = stage_plan();
        let bundle = materialize_curated_founder_bundle(&plan).unwrap();
        let source_world = HeadlessScenarioBuilder::new(WORLD_SEED)
            .agent("founder-0", OrganismId(ORGANISM_IDS[0]), Vec3f::ZERO)
            .agent(
                "founder-1",
                OrganismId(ORGANISM_IDS[1]),
                Vec3f::new(1.0, 0.0, 0.0),
            )
            .agent(
                "founder-2",
                OrganismId(ORGANISM_IDS[2]),
                Vec3f::new(2.0, 0.0, 0.0),
            )
            .build()
            .unwrap();
        let source_save = PortableSaveFile::from_headless_world(
            "save-stage-3a",
            &source_world,
            RuntimeConfig::deterministic_default(WORLD_SEED, BrainScaleTier::Nano512),
            AssetManifest::empty(),
            Vec::new(),
        )
        .unwrap();
        let restored_world = source_save.restore_headless_world().unwrap();
        let archive_root = std::env::temp_dir().join(format!(
            "alife-curated-founder-staging-{}-{label}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&archive_root);
        let lineage_library =
            LineageLibrary::open(LineageLibraryConfig::profile_default(&archive_root)).unwrap();
        for entry in &plan.entries {
            assert_eq!(
                lineage_library
                    .latest_manifest_for("archive-run-3a", entry.organism_id)
                    .unwrap(),
                None
            );
        }
        StageFixture {
            plan,
            bundle,
            source_save,
            restored_world,
            lineage_library: Some(lineage_library),
            archive_root,
        }
    }

    struct AuthoritySnapshot {
        world_signature: alife_world::HeadlessWorldSignatureDigest,
        registry: Vec<Vec<u8>>,
        save: Vec<u8>,
        archive: BTreeMap<PathBuf, Vec<u8>>,
        archive_count: u64,
    }

    fn authority_snapshot(fixture: &StageFixture) -> AuthoritySnapshot {
        let lineage_library = fixture.lineage_library();
        AuthoritySnapshot {
            world_signature: fixture.restored_world.canonical_signature_digest().unwrap(),
            registry: registry_snapshot(&fixture.restored_world),
            save: serde_json::to_vec(&fixture.source_save).unwrap(),
            archive: archive_snapshot(lineage_library.root()),
            archive_count: lineage_library.manifest_count().unwrap(),
        }
    }

    fn assert_authority_unchanged(before: AuthoritySnapshot, fixture: &StageFixture) {
        let after = authority_snapshot(fixture);
        assert_eq!(after.world_signature, before.world_signature);
        assert_eq!(after.registry, before.registry);
        assert_eq!(after.save, before.save);
        assert_eq!(after.archive_count, before.archive_count);
        assert_eq!(after.archive, before.archive);
    }

    fn gpu_runtime_fixture(save_id: &str, world_tick: Tick) -> GpuRuntimeSaveState {
        GpuRuntimeSaveState {
            schema: FVR06_GPU_RUNTIME_STATE_SCHEMA.to_string(),
            schema_version: FVR06_GPU_RUNTIME_STATE_SCHEMA_VERSION,
            requested_backend_mode: "GpuAuthoritative".to_string(),
            selected_backend_mode: "Unavailable".to_string(),
            adapter_identity: GpuRuntimeAdapterIdentity {
                adapter_name: None,
                backend_api: None,
                adapter_type: None,
                driver: None,
                driver_info: Some("staging test fixture".to_string()),
            },
            validation_profile: "test-unavailable".to_string(),
            brain_residency_slots: GpuRuntimeResidencySlots {
                hot_slots: 1,
                warm_slots: 1,
                cold_slots: 0,
            },
            class_bucket_allocations: vec![GpuRuntimeClassBucketAllocation {
                brain_class: BrainScaleTier::Nano512,
                hot_slots: 1,
                warm_slots: 1,
                cold_slots: 0,
                max_creatures: 1,
            }],
            active_profile_caps: GpuRuntimeActiveProfileCaps {
                target_fps: 30,
                target_frame_ms: 33.333,
                renderer_reserve_ms: 12.0,
                gpu_neural_budget_ms: 4.0,
                neural_heap_mb: 64,
                staging_readback_budget_kib: 4,
                chunk_activation_radius: 1,
                active_chunk_cap: 9,
                vfx_budget: "conservative".to_string(),
                adaptive_throttling_order: vec!["vfx".to_string()],
            },
            shader_abi_versions: GpuRuntimeShaderAbiVersions {
                shader_manifest: vec!["closed-loop:v1".to_string()],
                abi_manifest: vec!["gpu-runtime:v1".to_string()],
            },
            authority: GpuRuntimeAuthorityState {
                authoritative: false,
                failure_stops_learned_actions: true,
                finite_rejections: 0,
            },
            last_safe_checkpoint: GpuRuntimeSafeCheckpoint {
                save_id: save_id.to_string(),
                world_tick,
                sealed_patch_boundary: true,
                checkpoint_label: format!("staging-test:tick={}", world_tick.raw()),
            },
            unavailable_reason: Some("staging test fixture".to_string()),
            selected_scale_profile: "MinimumSettings30x30".to_string(),
            compact_action_readback_bytes_per_creature: 64,
            no_active_bulk_readback: true,
        }
    }

    #[test]
    fn curated_stage_builds_complete_candidates_without_mutation() {
        let plan = stage_plan();
        let bundle = materialize_curated_founder_bundle(&plan).unwrap();
        let source_world = HeadlessScenarioBuilder::new(WORLD_SEED)
            .agent("founder-0", OrganismId(ORGANISM_IDS[0]), Vec3f::ZERO)
            .agent(
                "founder-1",
                OrganismId(ORGANISM_IDS[1]),
                Vec3f::new(1.0, 0.0, 0.0),
            )
            .agent(
                "founder-2",
                OrganismId(ORGANISM_IDS[2]),
                Vec3f::new(2.0, 0.0, 0.0),
            )
            .build()
            .unwrap();
        let source_save = PortableSaveFile::from_headless_world(
            "save-stage-3a",
            &source_world,
            RuntimeConfig::deterministic_default(WORLD_SEED, BrainScaleTier::Nano512),
            AssetManifest::empty(),
            Vec::new(),
        )
        .unwrap();
        let restored_world = source_save.restore_headless_world().unwrap();
        let archive_root = std::env::temp_dir().join(format!(
            "alife-curated-founder-staging-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&archive_root);
        let lineage_library =
            LineageLibrary::open(LineageLibraryConfig::profile_default(&archive_root)).unwrap();
        let archive_run_id = "archive-run-3a";
        for entry in &plan.entries {
            assert_eq!(
                lineage_library
                    .latest_manifest_for(archive_run_id, entry.organism_id)
                    .unwrap(),
                None
            );
        }
        let before_world_signature = restored_world.canonical_signature_digest().unwrap();
        let before_registry = registry_snapshot(&restored_world);
        let before_save = serde_json::to_vec(&source_save).unwrap();
        let before_archive = archive_snapshot(lineage_library.root());
        let before_archive_count = lineage_library.manifest_count().unwrap();

        let stage = stage_curated_founder_reset(
            &plan,
            &bundle,
            &source_save,
            &restored_world,
            &lineage_library,
            Path::new("."),
            archive_run_id,
        )
        .unwrap();

        assert_eq!(stage.source_save_id, source_save.save_id);
        assert_eq!(stage.source_save_identity, plan.source_save_identity);
        assert_eq!(stage.deterministic_seed, plan.source_save_seed);
        assert_eq!(stage.world_seed, plan.world_seed);
        assert_eq!(stage.restored_tick, plan.restored_tick);
        assert_eq!(stage.safe_checkpoint, None);
        assert_eq!(stage.receipt, plan.receipt);
        assert_eq!(
            stage.ordered_founder_ids,
            plan.entries
                .iter()
                .map(|entry| entry.organism_id)
                .collect::<Vec<_>>()
        );
        assert_eq!(stage.record_candidates.len(), 3);
        assert_eq!(stage.archive_birth_intents.len(), 3);
        assert_eq!(
            stage.target_agent_bindings,
            plan.entries
                .iter()
                .map(|entry| (entry.world_entity_id, entry.organism_id))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            stage.expected_registry_identity,
            plan.entries
                .iter()
                .map(|entry| (entry.organism_id, entry.world_entity_id))
                .collect::<Vec<_>>()
        );
        for ((record, intent), (plan_entry, bundle_entry)) in stage
            .record_candidates
            .iter()
            .zip(&stage.archive_birth_intents)
            .zip(plan.entries.iter().zip(&bundle.entries))
        {
            assert_eq!(record.organism_id(), plan_entry.organism_id);
            assert_eq!(record.world_entity_id(), plan_entry.world_entity_id);
            assert_eq!(record.birth_tick(), plan.restored_tick);
            assert_eq!(record.genome(), &bundle_entry.genome);
            assert_eq!(record.phenotype(), &bundle_entry.phenotype);
            assert_eq!(record.archive().birth_manifest_digest(), None);
            assert_eq!(intent.source_run_id, archive_run_id);
            assert_eq!(intent.organism_id, plan_entry.organism_id);
            assert_eq!(intent.genome_id, plan_entry.genome_id);
            assert_eq!(intent.lineage_id, plan_entry.lineage_id);
            assert_eq!(intent.birth_tick, plan.restored_tick);
            assert_eq!(intent.foundation, plan.foundation);
            assert_eq!(
                intent.foundation_content_digest,
                plan.foundation_content_digest
            );
            assert_eq!(intent.sensor_profile, plan.sensor_profile);
            assert_eq!(
                intent.projection_receipt,
                bundle_entry.projection.receipt().clone()
            );
            assert_eq!(
                intent.phenotype_hash,
                bundle_entry.projection.receipt().phenotype_hash()
            );
            assert_eq!(
                intent.compiled_phenotype,
                bundle_entry.projection.compiled_phenotype().clone()
            );
        }

        assert_eq!(
            restored_world.canonical_signature_digest().unwrap(),
            before_world_signature
        );
        assert_eq!(registry_snapshot(&restored_world), before_registry);
        assert_eq!(serde_json::to_vec(&source_save).unwrap(), before_save);
        assert_eq!(
            lineage_library.manifest_count().unwrap(),
            before_archive_count
        );
        assert_eq!(archive_snapshot(lineage_library.root()), before_archive);

        drop(lineage_library);
        let _ = fs::remove_dir_all(archive_root);
    }

    #[test]
    fn curated_stage_rejects_bundle_or_record_identity_mismatch() {
        let cases: &[(&str, fn(&mut CuratedFounderBundle))] = &[
            ("bundle-plan-entry", |bundle| {
                bundle.entries[1].plan_entry.lineage_id = bundle.entries[0].plan_entry.lineage_id;
            }),
            ("bundle-genome", |bundle| {
                bundle.entries[1].genome = bundle.entries[0].genome.clone();
            }),
            ("bundle-projection", |bundle| {
                bundle.entries[1].projection = bundle.entries[0].projection.clone();
            }),
            ("bundle-biology-source", |bundle| {
                bundle.entries[1].biochemistry.source_genome_id = GenomeId(999);
            }),
            ("bundle-biology-tick", |bundle| {
                bundle.entries[1].biochemistry.tick = Tick::new(1);
                bundle.entries[1].biochemistry.homeostasis.tick = Tick::new(1);
            }),
        ];

        for (label, mutate) in cases {
            let mut fixture = stage_fixture(label);
            mutate(&mut fixture.bundle);
            let before = authority_snapshot(&fixture);
            let error = stage_curated_founder_reset(
                &fixture.plan,
                &fixture.bundle,
                &fixture.source_save,
                &fixture.restored_world,
                fixture.lineage_library(),
                Path::new("."),
                "archive-run-3a",
            )
            .unwrap_err();
            assert!(
                matches!(
                    error,
                    CuratedFounderStagingError::BundleMismatch { .. }
                        | CuratedFounderStagingError::Contract { .. }
                ),
                "{label}: unexpected error: {error:?}"
            );
            assert_authority_unchanged(before, &fixture);
        }
    }

    #[test]
    fn curated_stage_rejects_duplicate_or_conflicting_ids() {
        let cases: &[(&str, fn(&mut CuratedFounderPlan))] = &[
            ("duplicate-world-entity", |plan| {
                plan.entries[1].world_entity_id = plan.entries[0].world_entity_id;
            }),
            ("zero-world-entity", |plan| {
                plan.entries[1].world_entity_id = WorldEntityId(0);
            }),
            ("duplicate-organism", |plan| {
                plan.entries[1].organism_id = plan.entries[0].organism_id;
            }),
            ("zero-organism", |plan| {
                plan.entries[1].organism_id = OrganismId(0);
            }),
            ("duplicate-slot", |plan| {
                plan.entries[1].final_population_slot = plan.entries[0].final_population_slot;
            }),
            ("duplicate-conception-seed", |plan| {
                plan.entries[1].conception_seed = plan.entries[0].conception_seed;
            }),
            ("zero-conception-seed", |plan| {
                plan.entries[1].conception_seed = 0;
            }),
            ("duplicate-genome", |plan| {
                plan.entries[1].genome_id = plan.entries[0].genome_id;
            }),
            ("duplicate-lineage", |plan| {
                plan.entries[1].lineage_id = plan.entries[0].lineage_id;
            }),
        ];

        for (label, mutate) in cases {
            let mut fixture = stage_fixture(label);
            mutate(&mut fixture.plan);
            let before = authority_snapshot(&fixture);
            let error = stage_curated_founder_reset(
                &fixture.plan,
                &fixture.bundle,
                &fixture.source_save,
                &fixture.restored_world,
                fixture.lineage_library(),
                Path::new("."),
                "archive-run-3a",
            )
            .unwrap_err();
            assert!(
                matches!(error, CuratedFounderStagingError::Plan(_)),
                "{label}: unexpected error: {error:?}"
            );
            assert_authority_unchanged(before, &fixture);
        }

        let mut fixture = stage_fixture("existing-registry");
        let entry = &fixture.bundle.entries[0];
        let record_biochemistry =
            BiochemistryState::new(&entry.phenotype, fixture.plan.restored_tick).unwrap();
        let record = WorldOrganismRecord::new(
            entry.plan_entry.organism_id,
            entry.plan_entry.world_entity_id,
            entry.genome.clone(),
            entry.phenotype.clone(),
            record_biochemistry,
            fixture.plan.restored_tick,
        )
        .unwrap();
        fixture
            .restored_world
            .register_organism_record(record)
            .unwrap();
        let before = authority_snapshot(&fixture);
        let error = stage_curated_founder_reset(
            &fixture.plan,
            &fixture.bundle,
            &fixture.source_save,
            &fixture.restored_world,
            fixture.lineage_library(),
            Path::new("."),
            "archive-run-3a",
        )
        .unwrap_err();
        assert!(matches!(
            error,
            CuratedFounderStagingError::ExistingRegistry { .. }
        ));
        assert_authority_unchanged(before, &fixture);
    }

    #[test]
    fn curated_stage_rejects_wrong_world_save_or_checkpoint_identity() {
        let cases: &[(&str, fn(&mut PortableSaveFile))] = &[
            ("wrong-save-id", |save| {
                save.save_id = "wrong-save".to_string()
            }),
            ("wrong-save-seed", |save| save.deterministic_seed = 9_999),
            ("wrong-save-world-seed", |save| {
                save.world.seed = WORLD_SEED + 1
            }),
            ("wrong-save-world-tick", |save| {
                save.world.tick = Tick::new(1)
            }),
        ];

        for (label, mutate) in cases {
            let mut fixture = stage_fixture(label);
            mutate(&mut fixture.source_save);
            let before = authority_snapshot(&fixture);
            let error = stage_curated_founder_reset(
                &fixture.plan,
                &fixture.bundle,
                &fixture.source_save,
                &fixture.restored_world,
                fixture.lineage_library(),
                Path::new("."),
                "archive-run-3a",
            )
            .unwrap_err();
            assert!(
                matches!(
                    error,
                    CuratedFounderStagingError::Mismatch { .. }
                        | CuratedFounderStagingError::Save(_)
                ),
                "{label}: unexpected error: {error:?}"
            );
            assert_authority_unchanged(before, &fixture);
        }

        for (label, wrong_checkpoint) in [
            (
                "wrong-checkpoint-save-id",
                GpuRuntimeSafeCheckpoint {
                    save_id: "wrong-save".to_string(),
                    world_tick: Tick::ZERO,
                    sealed_patch_boundary: true,
                    checkpoint_label: "staging-test:tick=0".to_string(),
                },
            ),
            (
                "wrong-checkpoint-tick",
                GpuRuntimeSafeCheckpoint {
                    save_id: "save-stage-3a".to_string(),
                    world_tick: Tick::new(1),
                    sealed_patch_boundary: true,
                    checkpoint_label: "staging-test:tick=1".to_string(),
                },
            ),
        ] {
            let mut fixture = stage_fixture(label);
            fixture.source_save.gpu_runtime =
                Some(gpu_runtime_fixture("save-stage-3a", Tick::ZERO));
            fixture
                .source_save
                .gpu_runtime
                .as_mut()
                .unwrap()
                .last_safe_checkpoint = wrong_checkpoint;
            let before = authority_snapshot(&fixture);
            let error = stage_curated_founder_reset(
                &fixture.plan,
                &fixture.bundle,
                &fixture.source_save,
                &fixture.restored_world,
                fixture.lineage_library(),
                Path::new("."),
                "archive-run-3a",
            )
            .unwrap_err();
            assert!(matches!(error, CuratedFounderStagingError::Save(_)));
            assert_authority_unchanged(before, &fixture);
        }

        let wrong_seed_fixture = stage_fixture("wrong-restored-seed");
        let wrong_seed_world = HeadlessScenarioBuilder::new(WORLD_SEED + 1)
            .agent("founder-0", OrganismId(ORGANISM_IDS[0]), Vec3f::ZERO)
            .agent(
                "founder-1",
                OrganismId(ORGANISM_IDS[1]),
                Vec3f::new(1.0, 0.0, 0.0),
            )
            .agent(
                "founder-2",
                OrganismId(ORGANISM_IDS[2]),
                Vec3f::new(2.0, 0.0, 0.0),
            )
            .build()
            .unwrap();
        let before_seed_world = wrong_seed_world.canonical_signature_digest().unwrap();
        let before_seed_authority = authority_snapshot(&wrong_seed_fixture);
        let error = stage_curated_founder_reset(
            &wrong_seed_fixture.plan,
            &wrong_seed_fixture.bundle,
            &wrong_seed_fixture.source_save,
            &wrong_seed_world,
            wrong_seed_fixture.lineage_library(),
            Path::new("."),
            "archive-run-3a",
        )
        .unwrap_err();
        assert!(matches!(
            error,
            CuratedFounderStagingError::Mismatch {
                field: "restored world seed"
            }
        ));
        assert_eq!(
            wrong_seed_world.canonical_signature_digest().unwrap(),
            before_seed_world
        );
        assert_authority_unchanged(before_seed_authority, &wrong_seed_fixture);

        let wrong_tick_fixture = stage_fixture("wrong-restored-tick");
        let mut wrong_tick_world = wrong_tick_fixture
            .source_save
            .restore_headless_world()
            .unwrap();
        wrong_tick_world.advance_tick();
        let before_tick_world = wrong_tick_world.canonical_signature_digest().unwrap();
        let before_tick_authority = authority_snapshot(&wrong_tick_fixture);
        let error = stage_curated_founder_reset(
            &wrong_tick_fixture.plan,
            &wrong_tick_fixture.bundle,
            &wrong_tick_fixture.source_save,
            &wrong_tick_world,
            wrong_tick_fixture.lineage_library(),
            Path::new("."),
            "archive-run-3a",
        )
        .unwrap_err();
        assert!(matches!(
            error,
            CuratedFounderStagingError::Mismatch {
                field: "restored world tick"
            }
        ));
        assert_eq!(
            wrong_tick_world.canonical_signature_digest().unwrap(),
            before_tick_world
        );
        assert_authority_unchanged(before_tick_authority, &wrong_tick_fixture);

        let mut asset_fixture = stage_fixture("wrong-asset-reference");
        asset_fixture
            .source_save
            .assets
            .entries
            .push(AssetManifestEntry {
                asset_id: "missing-staging-asset".to_string(),
                kind: AssetKind::Other,
                relative_path: "missing-staging-asset.bin".to_string(),
                digest: PortableAssetDigest::for_bytes(b"missing"),
                presence: AssetPresence::Required,
                schema_version: 1,
                size_bytes: None,
                provenance: None,
            });
        let before = authority_snapshot(&asset_fixture);
        let error = stage_curated_founder_reset(
            &asset_fixture.plan,
            &asset_fixture.bundle,
            &asset_fixture.source_save,
            &asset_fixture.restored_world,
            asset_fixture.lineage_library(),
            Path::new("."),
            "archive-run-3a",
        )
        .unwrap_err();
        assert!(matches!(error, CuratedFounderStagingError::Save(_)));
        assert_authority_unchanged(before, &asset_fixture);
    }

    #[test]
    fn curated_stage_rejects_invalid_archive_run_id_and_conflict() {
        for (label, archive_run_id) in [
            ("colon-run-id", "archive:run"),
            ("empty-run-id", ""),
            ("space-run-id", "archive run"),
            ("slash-run-id", "archive/run"),
        ] {
            let fixture = stage_fixture(label);
            let before = authority_snapshot(&fixture);
            let error = stage_curated_founder_reset(
                &fixture.plan,
                &fixture.bundle,
                &fixture.source_save,
                &fixture.restored_world,
                fixture.lineage_library(),
                Path::new("."),
                archive_run_id,
            )
            .unwrap_err();
            assert!(matches!(
                error,
                CuratedFounderStagingError::InvalidArchiveRunId { .. }
            ));
            assert_authority_unchanged(before, &fixture);
        }

        let mut fixture = stage_fixture("archive-conflict");
        let entry = &fixture.bundle.entries[0];
        let organism_id = entry.plan_entry.organism_id;
        let birth_tick = fixture.plan.restored_tick;
        let genome = entry.projection.source_brain_genome().clone();
        let phenotype = entry.projection.compiled_phenotype().clone();
        let foundation_asset =
            FoundationWeightAsset::builtin_nano512_v1(fixture.plan.sensor_profile).unwrap();
        let foundation_bytes = foundation_asset.encode_canonical().unwrap();
        fixture
            .lineage_library_mut()
            .archive_birth(GeneticArchiveInput {
                source_run_id: "archive-run-3a",
                organism_id,
                birth_tick,
                genome: &genome,
                phenotype: &phenotype,
                foundation_asset_bytes: Some(&foundation_bytes),
            })
            .unwrap();
        for entry in &fixture.plan.entries {
            fixture
                .lineage_library()
                .latest_manifest_for("archive-run-3a", entry.organism_id)
                .unwrap();
        }
        let before = authority_snapshot(&fixture);
        let error = stage_curated_founder_reset(
            &fixture.plan,
            &fixture.bundle,
            &fixture.source_save,
            &fixture.restored_world,
            fixture.lineage_library(),
            Path::new("."),
            "archive-run-3a",
        )
        .unwrap_err();
        assert!(matches!(
            error,
            CuratedFounderStagingError::ArchiveConflict {
                organism_id: OrganismId(101)
            }
        ));
        assert_authority_unchanged(before, &fixture);
    }

    #[test]
    fn curated_stage_constructs_prefix_candidates_locally_before_late_failure_without_publication()
    {
        let mut fixture = stage_fixture("late-failure");
        // This remains a contract-valid biochemistry value. Its source-genome
        // identity is wrong only for entry 2, so the ordered production loop
        // must construct and retain local candidates for entries 0 and 1
        // before rejecting slot 2.
        fixture.bundle.entries[2].biochemistry.source_genome_id = GenomeId(999);
        let before = authority_snapshot(&fixture);
        let error = stage_curated_founder_reset(
            &fixture.plan,
            &fixture.bundle,
            &fixture.source_save,
            &fixture.restored_world,
            fixture.lineage_library(),
            Path::new("."),
            "archive-run-3a",
        )
        .unwrap_err();
        assert!(
            matches!(
                error,
                CuratedFounderStagingError::BundleMismatch {
                    slot: Some(2),
                    field: "per-entry biochemistry identity or tick"
                }
            ),
            "unexpected late failure error: {error:?}"
        );
        assert_authority_unchanged(before, &fixture);
        assert!(fixture.restored_world.organism_registry().is_empty());
    }
}
