use std::{
    fs,
    path::{Path, PathBuf},
};

use alife_archive::{
    ArchiveError, CommittedCompositeBirthBatch, CompositeGeneticArchiveBatchInput, LineageLibrary,
    PreparedCompositeBirthBatch,
};
use alife_core::{
    BiochemistryState, Blake3Digest, BrainCapacityClass, BrainPhenotype, FoundationGeneticIdentity,
    FoundationWeightAsset, GenomeId, LineageId, N512FounderProjectionReceipt, OrganismId,
    PhenotypeHash, ScaffoldContractError, SensorProfile, Tick, Validate, WorldEntityId,
};
use alife_runtime::{
    GpuDurableSaveManifest, GpuLoadedSaveManifest, GpuRuntimeError, GpuSaveManifestCasOutcome,
};
use alife_world::persistence::{
    GpuRuntimeSafeCheckpoint, PersistenceError, PortableAssetDigest, PortableSaveFile,
};
use alife_world::{
    HeadlessWorld, HeadlessWorldSignatureDigest, OrganismRegistryError, WorldObjectKind,
    WorldOrganismRecord,
};
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

#[derive(Debug, PartialEq)]
struct CuratedFounderResetApplyResult {
    committed_archive_batch: CommittedCompositeBirthBatch,
    reset_receipt: CuratedFounderResetReceipt,
    applied_registry_identity: Vec<(OrganismId, WorldEntityId)>,
    applied_world_signature: HeadlessWorldSignatureDigest,
}

#[derive(Debug)]
struct CuratedFounderPreparedReset {
    prepared_archive_batch: PreparedCompositeBirthBatch,
    linked_record_candidates: Vec<WorldOrganismRecord>,
    candidate_world: HeadlessWorld,
    candidate_world_signature: HeadlessWorldSignatureDigest,
}

#[derive(Debug, Clone)]
struct CuratedFounderBoundSourceSave {
    loaded_generation: GpuLoadedSaveManifest,
    canonical_save_path: PathBuf,
    canonical_asset_root: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CuratedFounderPublicationStatus {
    Published,
    AlreadyApplied,
    ArchiveCommittedSaveConflict,
    ArchiveCommittedSaveFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CuratedFounderSaveState {
    Verified,
    Conflict,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
struct CuratedFounderArchiveReceiptRow {
    final_population_slot: u32,
    world_entity_id: WorldEntityId,
    organism_id: OrganismId,
    genome_id: GenomeId,
    lineage_id: LineageId,
    birth_tick: Tick,
    manifest_digest: Blake3Digest,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CuratedFounderDurablePublicationReceipt {
    source_save_identity: String,
    source_save_seed: u64,
    source_world_seed: u64,
    source_tick: Tick,
    reset_receipt: CuratedFounderResetReceipt,
    durable_save_path: PathBuf,
    expected_save_digest: String,
    proposed_save_digest: String,
    final_save_digest: Option<String>,
    archive_source_run: String,
    archive_receipts: Vec<CuratedFounderArchiveReceiptRow>,
    candidate_world_signature: HeadlessWorldSignatureDigest,
    candidate_world_schema_version: u16,
    candidate_world_seed: u64,
    candidate_world_tick: Tick,
    status: CuratedFounderPublicationStatus,
}

impl CuratedFounderDurablePublicationReceipt {
    pub(crate) fn archive_receipt_count(&self) -> usize {
        self.archive_receipts.len()
    }

    pub(crate) fn final_save_digest(&self) -> Option<&str> {
        self.final_save_digest.as_deref()
    }

    pub(crate) fn candidate_world_signature(&self) -> HeadlessWorldSignatureDigest {
        self.candidate_world_signature
    }

    pub(crate) fn candidate_world_seed(&self) -> u64 {
        self.candidate_world_seed
    }

    pub(crate) fn candidate_world_tick(&self) -> Tick {
        self.candidate_world_tick
    }

    pub(crate) fn archive_source_run(&self) -> &str {
        &self.archive_source_run
    }

    pub(crate) fn archive_receipt_identities(
        &self,
    ) -> Vec<(u32, WorldEntityId, OrganismId, LineageId, Blake3Digest)> {
        self.archive_receipts
            .iter()
            .map(|row| {
                (
                    row.final_population_slot,
                    row.world_entity_id,
                    row.organism_id,
                    row.lineage_id,
                    row.manifest_digest,
                )
            })
            .collect()
    }
}

#[derive(Debug, Error)]
pub(crate) enum CuratedFounderDurablePublicationError {
    #[error("curated founder publication failed before archive commit: {0}")]
    PreCommit(#[source] CuratedFounderStagingError),
    #[error(
        "curated founder archive committed but save CAS conflicted: expected {expected_save_digest}, actual {actual_save_digest}, proposed {proposed_save_digest}"
    )]
    ArchiveCommittedSaveConflict {
        receipt: CuratedFounderDurablePublicationReceipt,
        expected_save_digest: String,
        actual_save_digest: String,
        proposed_save_digest: String,
    },
    #[error(
        "curated founder archive committed but save publication failed: {cause}; save state is {save_state:?}"
    )]
    ArchiveCommittedSaveFailure {
        receipt: CuratedFounderDurablePublicationReceipt,
        cause: String,
        proposed_save_digest: String,
        save_state: CuratedFounderSaveState,
    },
}

#[derive(Debug)]
pub(crate) struct CuratedFounderDurableOperation {
    stage: CuratedFounderResetStage,
    bundle: CuratedFounderBundle,
    bound_source: CuratedFounderBoundSourceSave,
    candidate_world: HeadlessWorld,
    linked_record_candidates: Vec<WorldOrganismRecord>,
    candidate_world_signature: HeadlessWorldSignatureDigest,
    replacement_save: PortableSaveFile,
    proposed_save_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CuratedFounderDurableOperationAttempt {
    Published {
        receipt: CuratedFounderDurablePublicationReceipt,
    },
    AlreadyApplied {
        receipt: CuratedFounderDurablePublicationReceipt,
    },
    ArchiveCommittedSaveConflict {
        receipt: CuratedFounderDurablePublicationReceipt,
        expected_save_digest: String,
        actual_save_digest: String,
        proposed_save_digest: String,
    },
    ArchiveCommittedSaveFailure {
        receipt: CuratedFounderDurablePublicationReceipt,
        cause: String,
        proposed_save_digest: String,
        save_state: CuratedFounderSaveState,
    },
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
    #[error("curated founder durable save preflight failed: {0}")]
    DurableSave(#[from] GpuRuntimeError),
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

fn bind_curated_founder_source(
    durable_manifest: &GpuDurableSaveManifest,
) -> Result<CuratedFounderBoundSourceSave, CuratedFounderStagingError> {
    let loaded_generation = durable_manifest.load()?;
    Ok(CuratedFounderBoundSourceSave {
        loaded_generation,
        canonical_save_path: durable_manifest.save_path().to_path_buf(),
        canonical_asset_root: durable_manifest.asset_root().to_path_buf(),
    })
}

impl CuratedFounderDurableOperation {
    pub(crate) fn bind_and_stage(
        plan: &CuratedFounderPlan,
        bundle: CuratedFounderBundle,
        durable_manifest: &GpuDurableSaveManifest,
        canonical_live_world: &HeadlessWorld,
        lineage_library: &LineageLibrary,
        archive_run_id: &str,
    ) -> Result<Self, CuratedFounderStagingError> {
        let bound_source = bind_curated_founder_source(durable_manifest)?;
        let stage = stage_curated_founder_reset(
            plan,
            &bundle,
            &bound_source.loaded_generation.save,
            canonical_live_world,
            lineage_library,
            &bound_source.canonical_asset_root,
            archive_run_id,
        )?;
        let prepared = prepare_curated_founder_reset(
            &stage,
            &bundle,
            lineage_library,
            canonical_live_world,
            false,
        )?;
        let mut replacement_save = bound_source.loaded_generation.save.clone();
        replacement_save.replace_headless_world_snapshot(&prepared.candidate_world)?;
        replacement_save.validate_with_asset_root(&bound_source.canonical_asset_root)?;
        let proposed_save_digest = portable_save_digest(&replacement_save)?;

        Ok(Self {
            stage,
            bundle,
            bound_source,
            candidate_world: prepared.candidate_world,
            linked_record_candidates: prepared.linked_record_candidates,
            candidate_world_signature: prepared.candidate_world_signature,
            replacement_save,
            proposed_save_digest,
        })
    }

    pub(crate) fn attempt(
        &self,
        durable_manifest: &GpuDurableSaveManifest,
        lineage_library: &LineageLibrary,
        live_world: &mut HeadlessWorld,
    ) -> Result<CuratedFounderDurableOperationAttempt, CuratedFounderStagingError> {
        match publish_curated_founder_operation_durably(
            self,
            durable_manifest,
            lineage_library,
            live_world,
        ) {
            Ok(receipt) => match receipt.status {
                CuratedFounderPublicationStatus::Published => {
                    Ok(CuratedFounderDurableOperationAttempt::Published { receipt })
                }
                CuratedFounderPublicationStatus::AlreadyApplied => {
                    Ok(CuratedFounderDurableOperationAttempt::AlreadyApplied { receipt })
                }
                CuratedFounderPublicationStatus::ArchiveCommittedSaveConflict
                | CuratedFounderPublicationStatus::ArchiveCommittedSaveFailure => {
                    Err(CuratedFounderStagingError::Mismatch {
                        field: "successful durable publication status",
                    })
                }
            },
            Err(CuratedFounderDurablePublicationError::PreCommit(error)) => Err(error),
            Err(CuratedFounderDurablePublicationError::ArchiveCommittedSaveConflict {
                receipt,
                expected_save_digest,
                actual_save_digest,
                proposed_save_digest,
            }) => Ok(
                CuratedFounderDurableOperationAttempt::ArchiveCommittedSaveConflict {
                    receipt,
                    expected_save_digest,
                    actual_save_digest,
                    proposed_save_digest,
                },
            ),
            Err(CuratedFounderDurablePublicationError::ArchiveCommittedSaveFailure {
                receipt,
                cause,
                proposed_save_digest,
                save_state,
            }) => Ok(
                CuratedFounderDurableOperationAttempt::ArchiveCommittedSaveFailure {
                    receipt,
                    cause,
                    proposed_save_digest,
                    save_state,
                },
            ),
        }
    }

    pub(crate) fn proposed_save_digest(&self) -> &str {
        &self.proposed_save_digest
    }

    pub(crate) fn accepted_bundle(&self) -> &CuratedFounderBundle {
        &self.bundle
    }

    #[cfg(test)]
    pub(crate) fn test_replacement_save(&self) -> PortableSaveFile {
        self.replacement_save.clone()
    }

    #[cfg(test)]
    pub(crate) fn test_identity_fingerprint(
        &self,
    ) -> (Vec<OrganismId>, Vec<(WorldEntityId, OrganismId)>, String) {
        (
            self.stage.ordered_founder_ids.clone(),
            self.stage.target_agent_bindings.clone(),
            self.proposed_save_digest.clone(),
        )
    }
}

impl CuratedFounderDurableOperationAttempt {
    pub(crate) const fn status(&self) -> CuratedFounderPublicationStatus {
        match self {
            Self::Published { .. } => CuratedFounderPublicationStatus::Published,
            Self::AlreadyApplied { .. } => CuratedFounderPublicationStatus::AlreadyApplied,
            Self::ArchiveCommittedSaveConflict { .. } => {
                CuratedFounderPublicationStatus::ArchiveCommittedSaveConflict
            }
            Self::ArchiveCommittedSaveFailure { .. } => {
                CuratedFounderPublicationStatus::ArchiveCommittedSaveFailure
            }
        }
    }

    pub(crate) const fn save_state(&self) -> CuratedFounderSaveState {
        match self {
            Self::Published { .. } | Self::AlreadyApplied { .. } => {
                CuratedFounderSaveState::Verified
            }
            Self::ArchiveCommittedSaveConflict { .. } => CuratedFounderSaveState::Conflict,
            Self::ArchiveCommittedSaveFailure { save_state, .. } => *save_state,
        }
    }

    pub(crate) const fn retains_operation(&self) -> bool {
        matches!(
            self,
            Self::ArchiveCommittedSaveConflict { .. } | Self::ArchiveCommittedSaveFailure { .. }
        )
    }

    pub(crate) fn receipt(&self) -> &CuratedFounderDurablePublicationReceipt {
        match self {
            Self::Published { receipt }
            | Self::AlreadyApplied { receipt }
            | Self::ArchiveCommittedSaveConflict { receipt, .. }
            | Self::ArchiveCommittedSaveFailure { receipt, .. } => receipt,
        }
    }

    pub(crate) fn expected_save_digest(&self) -> Option<&str> {
        match self {
            Self::ArchiveCommittedSaveConflict {
                expected_save_digest,
                ..
            } => Some(expected_save_digest),
            _ => None,
        }
    }

    pub(crate) fn actual_save_digest(&self) -> Option<&str> {
        match self {
            Self::ArchiveCommittedSaveConflict {
                actual_save_digest, ..
            } => Some(actual_save_digest),
            _ => None,
        }
    }

    pub(crate) fn proposed_save_digest(&self) -> &str {
        match self {
            Self::Published { receipt } | Self::AlreadyApplied { receipt } => {
                &receipt.proposed_save_digest
            }
            Self::ArchiveCommittedSaveConflict {
                proposed_save_digest,
                ..
            }
            | Self::ArchiveCommittedSaveFailure {
                proposed_save_digest,
                ..
            } => proposed_save_digest,
        }
    }

    pub(crate) fn cause(&self) -> Option<&str> {
        match self {
            Self::ArchiveCommittedSaveFailure { cause, .. } => Some(cause),
            _ => None,
        }
    }

    pub(crate) fn final_save_digest(&self) -> Option<&str> {
        self.receipt().final_save_digest.as_deref()
    }
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

fn apply_curated_founder_reset(
    stage: &CuratedFounderResetStage,
    bundle: &CuratedFounderBundle,
    lineage_library: &mut LineageLibrary,
    world: &mut HeadlessWorld,
) -> Result<CuratedFounderResetApplyResult, CuratedFounderStagingError> {
    let prepared = prepare_curated_founder_reset(stage, bundle, lineage_library, world, false)?;
    let CuratedFounderPreparedReset {
        prepared_archive_batch,
        linked_record_candidates,
        candidate_world,
        candidate_world_signature,
    } = prepared;

    let applied_registry_identity = linked_record_candidates
        .iter()
        .map(|record| (record.organism_id(), record.world_entity_id()))
        .collect();
    let committed_archive_batch =
        lineage_library.commit_composite_birth_batch(prepared_archive_batch)?;
    *world = candidate_world;

    Ok(CuratedFounderResetApplyResult {
        committed_archive_batch,
        reset_receipt: stage.receipt.clone(),
        applied_registry_identity,
        applied_world_signature: candidate_world_signature,
    })
}

fn prepare_curated_founder_reset(
    stage: &CuratedFounderResetStage,
    bundle: &CuratedFounderBundle,
    lineage_library: &LineageLibrary,
    world: &HeadlessWorld,
    allow_existing_registry: bool,
) -> Result<CuratedFounderPreparedReset, CuratedFounderStagingError> {
    validate_curated_founder_apply_inputs(stage, bundle, world, allow_existing_registry)?;

    let foundation_asset = FoundationWeightAsset::builtin_nano512_v1(stage.receipt.sensor_profile)
        .map_err(|source| CuratedFounderStagingError::Contract {
            field: "checked Nano512 foundation asset",
            source,
        })?;
    let expected_foundation = FoundationGeneticIdentity::new(
        foundation_asset.manifest().foundation_id().raw(),
        foundation_asset.manifest().foundation_version().raw() as u16,
        foundation_asset.manifest().compatibility_family_id().raw(),
        BrainCapacityClass::N512_ID,
    )
    .map_err(|source| CuratedFounderStagingError::Contract {
        field: "checked Nano512 foundation identity",
        source,
    })?;
    if stage.receipt.foundation != expected_foundation
        || stage.receipt.foundation_content_digest != foundation_asset.digest()
    {
        return Err(CuratedFounderStagingError::Mismatch {
            field: "checked Nano512 foundation identity or digest",
        });
    }
    let foundation_asset_bytes = foundation_asset.encode_canonical().map_err(|source| {
        CuratedFounderStagingError::Contract {
            field: "canonical Nano512 foundation bytes",
            source,
        }
    })?;
    let decoded_foundation = FoundationWeightAsset::decode_canonical(&foundation_asset_bytes)
        .map_err(|source| CuratedFounderStagingError::Contract {
            field: "canonical Nano512 foundation bytes",
            source,
        })?;
    if decoded_foundation.manifest() != foundation_asset.manifest()
        || decoded_foundation.digest() != foundation_asset.digest()
    {
        return Err(CuratedFounderStagingError::Mismatch {
            field: "canonical Nano512 foundation identity or digest",
        });
    }

    let mut archive_inputs = Vec::with_capacity(stage.archive_birth_intents.len());
    for (entry, intent) in bundle.entries.iter().zip(&stage.archive_birth_intents) {
        archive_inputs.push(CompositeGeneticArchiveBatchInput {
            source_run_id: &intent.source_run_id,
            organism_id: intent.organism_id,
            genome_id: intent.genome_id,
            lineage_id: intent.lineage_id,
            birth_tick: intent.birth_tick,
            foundation: intent.foundation,
            foundation_content_digest: intent.foundation_content_digest,
            sensor_profile: intent.sensor_profile,
            projection_receipt: Some(&intent.projection_receipt),
            phenotype_hash: intent.phenotype_hash,
            creature_genome: &entry.genome,
            phenotype: &intent.compiled_phenotype,
            foundation_asset_bytes: &foundation_asset_bytes,
        });
    }
    let prepared = lineage_library.prepare_composite_birth_batch(&archive_inputs)?;

    let mut linked_records = stage.record_candidates.clone();
    for (slot, (record, digest)) in linked_records
        .iter_mut()
        .zip(prepared.manifest_digests())
        .enumerate()
    {
        record.link_birth_manifest(digest).map_err(|source| {
            CuratedFounderStagingError::Record {
                slot: slot as u32,
                source,
            }
        })?;
    }
    let mut replacement_world = world.clone();
    if !world.organism_registry().is_empty() && !registry_matches_records(world, &linked_records) {
        return Err(CuratedFounderStagingError::Mismatch {
            field: "already-applied candidate registry",
        });
    }
    replacement_world.replace_organism_registry_exact(linked_records.clone())?;
    let candidate_world_signature = replacement_world.canonical_signature_digest()?;

    Ok(CuratedFounderPreparedReset {
        prepared_archive_batch: prepared,
        linked_record_candidates: linked_records,
        candidate_world: replacement_world,
        candidate_world_signature,
    })
}

fn publish_curated_founder_operation_durably(
    operation: &CuratedFounderDurableOperation,
    durable_manifest: &GpuDurableSaveManifest,
    lineage_library: &LineageLibrary,
    live_world: &mut HeadlessWorld,
) -> Result<CuratedFounderDurablePublicationReceipt, CuratedFounderDurablePublicationError> {
    let stage = &operation.stage;
    let bundle = &operation.bundle;
    let bound_source = &operation.bound_source;
    let canonical_asset_root = bound_source.canonical_asset_root.as_path();
    validate_durable_publication_inputs(
        stage,
        bound_source,
        durable_manifest,
        canonical_asset_root,
    )
    .map_err(CuratedFounderDurablePublicationError::PreCommit)?;
    let source_save = &bound_source.loaded_generation.save;
    let expected_save_digest = &bound_source.loaded_generation.digest;
    let prepared = prepare_curated_founder_reset(stage, bundle, lineage_library, live_world, true)
        .map_err(CuratedFounderDurablePublicationError::PreCommit)?;
    if prepared.candidate_world_signature != operation.candidate_world_signature
        || prepared.linked_record_candidates != operation.linked_record_candidates
    {
        return Err(CuratedFounderDurablePublicationError::PreCommit(
            CuratedFounderStagingError::Mismatch {
                field: "retained curated founder candidate",
            },
        ));
    }

    let replacement_save = operation.replacement_save.clone();
    replacement_save
        .validate_with_asset_root(canonical_asset_root)
        .map_err(|error| {
            CuratedFounderDurablePublicationError::PreCommit(CuratedFounderStagingError::Save(
                error,
            ))
        })?;
    let proposed_save_digest = portable_save_digest(&replacement_save)
        .map_err(CuratedFounderDurablePublicationError::PreCommit)?;
    if proposed_save_digest != operation.proposed_save_digest {
        return Err(CuratedFounderDurablePublicationError::PreCommit(
            CuratedFounderStagingError::Mismatch {
                field: "retained curated founder replacement identity",
            },
        ));
    }
    let archive_source_run = stage
        .archive_birth_intents
        .first()
        .map(|intent| intent.source_run_id.clone())
        .ok_or_else(|| {
            CuratedFounderDurablePublicationError::PreCommit(CuratedFounderStagingError::Mismatch {
                field: "archive source run",
            })
        })?;

    let mut receipt = CuratedFounderDurablePublicationReceipt {
        source_save_identity: source_save.save_id.clone(),
        source_save_seed: source_save.deterministic_seed,
        source_world_seed: source_save.world.seed,
        source_tick: source_save.world.tick,
        reset_receipt: stage.receipt.clone(),
        durable_save_path: durable_manifest.save_path().to_path_buf(),
        expected_save_digest: expected_save_digest.as_str().to_string(),
        proposed_save_digest: proposed_save_digest.clone(),
        final_save_digest: None,
        archive_source_run,
        archive_receipts: Vec::new(),
        candidate_world_signature: prepared.candidate_world_signature,
        candidate_world_schema_version: prepared.candidate_world_signature.schema_version,
        candidate_world_seed: prepared.candidate_world.seed(),
        candidate_world_tick: prepared.candidate_world.tick(),
        status: CuratedFounderPublicationStatus::ArchiveCommittedSaveFailure,
    };

    let prepared_archive_batch = prepared.prepared_archive_batch;
    let linked_record_candidates = operation.linked_record_candidates.clone();
    let candidate_world = operation.candidate_world.clone();
    let candidate_world_signature = operation.candidate_world_signature;

    let committed_archive_batch = lineage_library
        .commit_composite_birth_batch(prepared_archive_batch)
        .map_err(|error| {
            CuratedFounderDurablePublicationError::PreCommit(CuratedFounderStagingError::Archive(
                error,
            ))
        })?;
    receipt.archive_receipts = build_archive_receipt_rows(stage, bundle, &committed_archive_batch)
        .map_err(|error| {
            archive_committed_save_failure(&receipt, proposed_save_digest.clone(), error)
        })?;
    if let Err(error) =
        verify_archive_receipt_rows(stage, bundle, lineage_library, &receipt.archive_receipts)
    {
        return Err(archive_committed_save_failure(
            &receipt,
            proposed_save_digest.clone(),
            error,
        ));
    }

    let cas_outcome =
        match durable_manifest.compare_and_swap(expected_save_digest, &replacement_save) {
            Ok(outcome) => outcome,
            Err(GpuRuntimeError::GpuCheckpointManifestConflict { expected, actual }) => {
                receipt.status = CuratedFounderPublicationStatus::ArchiveCommittedSaveConflict;
                return Err(
                    CuratedFounderDurablePublicationError::ArchiveCommittedSaveConflict {
                        receipt,
                        expected_save_digest: expected,
                        actual_save_digest: actual,
                        proposed_save_digest,
                    },
                );
            }
            Err(error) => {
                return Err(archive_committed_save_failure(
                    &receipt,
                    proposed_save_digest,
                    error,
                ));
            }
        };
    let (status, final_save_digest) = match cas_outcome {
        GpuSaveManifestCasOutcome::Replaced { replacement_digest } => (
            CuratedFounderPublicationStatus::Published,
            replacement_digest.as_str().to_string(),
        ),
        GpuSaveManifestCasOutcome::AlreadyApplied { replacement_digest } => (
            CuratedFounderPublicationStatus::AlreadyApplied,
            replacement_digest.as_str().to_string(),
        ),
    };
    if final_save_digest != proposed_save_digest {
        return Err(archive_committed_save_failure(
            &receipt,
            proposed_save_digest.clone(),
            CuratedFounderStagingError::Mismatch {
                field: "CAS replacement digest",
            },
        ));
    }
    receipt.status = status;
    receipt.final_save_digest = Some(final_save_digest.clone());

    if let Err(error) = verify_durable_reload(
        durable_manifest,
        canonical_asset_root,
        &replacement_save,
        &final_save_digest,
        candidate_world_signature,
        candidate_world.seed(),
        candidate_world.tick(),
        &linked_record_candidates,
        &receipt.archive_receipts,
    ) {
        return Err(archive_committed_save_failure(
            &receipt,
            proposed_save_digest,
            error,
        ));
    }
    *live_world = candidate_world;
    Ok(receipt)
}

#[cfg(test)]
fn publish_curated_founder_reset_durably(
    stage: &CuratedFounderResetStage,
    bundle: &CuratedFounderBundle,
    bound_source: &CuratedFounderBoundSourceSave,
    durable_manifest: &GpuDurableSaveManifest,
    _canonical_asset_root: &Path,
    lineage_library: &LineageLibrary,
    live_world: &mut HeadlessWorld,
) -> Result<CuratedFounderDurablePublicationReceipt, CuratedFounderDurablePublicationError> {
    let prepared = prepare_curated_founder_reset(stage, bundle, lineage_library, live_world, true)
        .map_err(CuratedFounderDurablePublicationError::PreCommit)?;
    let mut replacement_save = bound_source.loaded_generation.save.clone();
    replacement_save
        .replace_headless_world_snapshot(&prepared.candidate_world)
        .map_err(|error| {
            CuratedFounderDurablePublicationError::PreCommit(CuratedFounderStagingError::Save(
                error,
            ))
        })?;
    replacement_save
        .validate_with_asset_root(&bound_source.canonical_asset_root)
        .map_err(|error| {
            CuratedFounderDurablePublicationError::PreCommit(CuratedFounderStagingError::Save(
                error,
            ))
        })?;
    let proposed_save_digest = portable_save_digest(&replacement_save)
        .map_err(CuratedFounderDurablePublicationError::PreCommit)?;
    let operation = CuratedFounderDurableOperation {
        stage: stage.clone(),
        bundle: bundle.clone(),
        bound_source: bound_source.clone(),
        candidate_world: prepared.candidate_world,
        linked_record_candidates: prepared.linked_record_candidates,
        candidate_world_signature: prepared.candidate_world_signature,
        replacement_save,
        proposed_save_digest,
    };
    publish_curated_founder_operation_durably(
        &operation,
        durable_manifest,
        lineage_library,
        live_world,
    )
}

fn validate_durable_publication_inputs(
    stage: &CuratedFounderResetStage,
    bound_source: &CuratedFounderBoundSourceSave,
    durable_manifest: &GpuDurableSaveManifest,
    canonical_asset_root: &Path,
) -> Result<(), CuratedFounderStagingError> {
    if bound_source.canonical_save_path.as_path() != durable_manifest.save_path()
        || bound_source.canonical_asset_root.as_path() != durable_manifest.asset_root()
    {
        return Err(CuratedFounderStagingError::Mismatch {
            field: "bound source durable manifest provenance",
        });
    }
    let canonical_asset_root = fs::canonicalize(canonical_asset_root)
        .map_err(|error| CuratedFounderStagingError::DurableSave(GpuRuntimeError::Io(error)))?;
    if durable_manifest.asset_root() != canonical_asset_root
        || !durable_manifest.save_path().is_absolute()
    {
        return Err(CuratedFounderStagingError::Mismatch {
            field: "durable save or canonical asset-root identity",
        });
    }
    bound_source
        .loaded_generation
        .save
        .validate_with_asset_root(&canonical_asset_root)?;
    validate_source_save_identity(stage, &bound_source.loaded_generation.save)?;
    Ok(())
}

fn validate_source_save_identity(
    stage: &CuratedFounderResetStage,
    source_save: &PortableSaveFile,
) -> Result<(), CuratedFounderStagingError> {
    let safe_checkpoint = source_save
        .gpu_runtime
        .as_ref()
        .map(|runtime| runtime.last_safe_checkpoint.clone());
    if source_save.save_id != stage.source_save_id
        || source_save.save_id != stage.source_save_identity
        || source_save.deterministic_seed != stage.deterministic_seed
        || source_save.world.seed != stage.world_seed
        || source_save.world.tick != stage.restored_tick
        || safe_checkpoint != stage.safe_checkpoint
    {
        return Err(CuratedFounderStagingError::Mismatch {
            field: "current source save identity",
        });
    }
    Ok(())
}

fn portable_save_digest(save: &PortableSaveFile) -> Result<String, CuratedFounderStagingError> {
    let json = serde_json::to_vec_pretty(save).map_err(PersistenceError::Json)?;
    Ok(PortableAssetDigest::for_bytes(&json).0)
}

fn registry_matches_records(world: &HeadlessWorld, expected: &[WorldOrganismRecord]) -> bool {
    let mut actual = world
        .organism_registry()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let mut expected = expected.to_vec();
    actual.sort_by_key(|record| record.organism_id().raw());
    expected.sort_by_key(|record| record.organism_id().raw());
    actual == expected
}

fn build_archive_receipt_rows(
    stage: &CuratedFounderResetStage,
    bundle: &CuratedFounderBundle,
    committed: &CommittedCompositeBirthBatch,
) -> Result<Vec<CuratedFounderArchiveReceiptRow>, CuratedFounderStagingError> {
    if committed.len() != stage.archive_birth_intents.len()
        || stage.target_agent_bindings.len() != committed.len()
        || bundle.entries.len() != committed.len()
    {
        return Err(CuratedFounderStagingError::Mismatch {
            field: "committed archive receipt count",
        });
    }
    committed
        .entries()
        .iter()
        .enumerate()
        .map(|(_batch_index, entry)| {
            let plan_entry = bundle
                .entries
                .iter()
                .find(|candidate| candidate.plan_entry.organism_id == entry.organism_id())
                .ok_or(CuratedFounderStagingError::Mismatch {
                    field: "committed archive receipt plan entry",
                })?;
            Ok(CuratedFounderArchiveReceiptRow {
                final_population_slot: plan_entry.plan_entry.final_population_slot,
                world_entity_id: plan_entry.plan_entry.world_entity_id,
                organism_id: entry.organism_id(),
                genome_id: entry.genome_id(),
                lineage_id: entry.lineage_id(),
                birth_tick: entry.birth_tick(),
                manifest_digest: entry.manifest_digest(),
            })
        })
        .collect()
}

fn verify_archive_receipt_rows(
    stage: &CuratedFounderResetStage,
    bundle: &CuratedFounderBundle,
    lineage_library: &LineageLibrary,
    rows: &[CuratedFounderArchiveReceiptRow],
) -> Result<(), CuratedFounderStagingError> {
    if rows.len() != stage.archive_birth_intents.len() || rows.len() != bundle.entries.len() {
        return Err(CuratedFounderStagingError::Mismatch {
            field: "durable archive receipt count",
        });
    }
    for (batch_index, row) in rows.iter().enumerate() {
        let intent = stage.archive_birth_intents.get(batch_index).ok_or(
            CuratedFounderStagingError::Mismatch {
                field: "durable archive receipt batch index",
            },
        )?;
        let entry = bundle
            .entries
            .iter()
            .find(|candidate| candidate.plan_entry.organism_id == row.organism_id)
            .ok_or(CuratedFounderStagingError::Mismatch {
                field: "durable archive bundle identity",
            })?;
        if row.final_population_slot != entry.plan_entry.final_population_slot
            || row.world_entity_id != stage.target_agent_bindings[batch_index].0
            || row.organism_id != intent.organism_id
            || row.genome_id != intent.genome_id
            || row.lineage_id != intent.lineage_id
            || row.birth_tick != intent.birth_tick
        {
            return Err(CuratedFounderStagingError::Mismatch {
                field: "durable archive receipt identity",
            });
        }
        if lineage_library.latest_manifest_for(&intent.source_run_id, intent.organism_id)?
            != Some(row.manifest_digest)
        {
            return Err(CuratedFounderStagingError::Mismatch {
                field: "durable latest archive manifest",
            });
        }
        let manifest = lineage_library.load_manifest(row.manifest_digest)?;
        let genetic = &manifest.genetic;
        let projection = &entry.projection;
        let phenotype = projection.compiled_phenotype();
        projection
            .validate()
            .map_err(|source| CuratedFounderStagingError::Contract {
                field: "durable archive projection",
                source,
            })?;
        intent
            .projection_receipt
            .validate_against_projection(projection)
            .map_err(|source| CuratedFounderStagingError::Contract {
                field: "durable archive projection receipt",
                source,
            })?;
        if genetic.source_run_id != intent.source_run_id
            || genetic.organism_id != intent.organism_id
            || genetic.genome_id != intent.genome_id
            || genetic.lineage_id != Some(intent.lineage_id)
            || genetic.birth_tick != intent.birth_tick
            || genetic.brain_class_id != intent.foundation.brain_class_id
            || genetic.sensor_profile != intent.sensor_profile
            || genetic.phenotype_hash != intent.phenotype_hash
            || genetic.foundation_id.map(|value| value.raw())
                != Some(intent.foundation.foundation_id)
            || genetic.foundation_version.map(|value| value.raw())
                != Some(u32::from(intent.foundation.version))
            || genetic.compatibility_family_id.map(|value| value.raw())
                != Some(intent.foundation.compatibility_family_id)
            || genetic.foundation_payload_digest != Some(intent.foundation_content_digest)
            || genetic.genome_asset.size_bytes == 0
            || genetic.composite_genome_asset.is_none()
            || genetic.foundation_asset.is_none()
            || genetic.persistent_address_map_digest != phenotype.persistent_address_map().digest()
            || genetic.language_codebook_id != phenotype.language_codebook().id()
            || genetic.language_codebook_digest != phenotype.language_codebook().canonical_digest()
            || manifest.previous_manifest_digest.is_some()
            || manifest.life.is_some()
        {
            return Err(CuratedFounderStagingError::Mismatch {
                field: "durable archive manifest identity",
            });
        }
    }
    Ok(())
}

fn verify_durable_reload(
    durable_manifest: &GpuDurableSaveManifest,
    canonical_asset_root: &Path,
    replacement_save: &PortableSaveFile,
    final_save_digest: &str,
    candidate_world_signature: HeadlessWorldSignatureDigest,
    candidate_world_seed: u64,
    candidate_world_tick: Tick,
    linked_record_candidates: &[WorldOrganismRecord],
    archive_receipts: &[CuratedFounderArchiveReceiptRow],
) -> Result<(), CuratedFounderStagingError> {
    let loaded = durable_manifest.load()?;
    if loaded.save != *replacement_save || loaded.digest.as_str() != final_save_digest {
        return Err(CuratedFounderStagingError::Mismatch {
            field: "durable replacement save reload",
        });
    }
    loaded.save.validate_with_asset_root(canonical_asset_root)?;
    let restored_world = loaded.save.restore_headless_world()?;
    if restored_world.seed() != candidate_world_seed
        || restored_world.tick() != candidate_world_tick
        || restored_world.canonical_signature_digest()? != candidate_world_signature
        || !registry_matches_records(&restored_world, linked_record_candidates)
    {
        return Err(CuratedFounderStagingError::Mismatch {
            field: "durable restored candidate world",
        });
    }
    for row in archive_receipts {
        let record = restored_world
            .organism_registry()
            .get(row.organism_id)
            .ok_or(CuratedFounderStagingError::Mismatch {
                field: "durable restored registry link",
            })?;
        if record.world_entity_id() != row.world_entity_id
            || record.archive().birth_manifest_digest() != Some(row.manifest_digest)
        {
            return Err(CuratedFounderStagingError::Mismatch {
                field: "durable restored archive link",
            });
        }
    }
    Ok(())
}

fn archive_committed_save_failure(
    receipt: &CuratedFounderDurablePublicationReceipt,
    proposed_save_digest: String,
    cause: impl std::fmt::Display,
) -> CuratedFounderDurablePublicationError {
    let mut receipt = receipt.clone();
    receipt.status = CuratedFounderPublicationStatus::ArchiveCommittedSaveFailure;
    CuratedFounderDurablePublicationError::ArchiveCommittedSaveFailure {
        receipt,
        cause: cause.to_string(),
        proposed_save_digest,
        save_state: CuratedFounderSaveState::Unknown,
    }
}

fn validate_curated_founder_apply_inputs(
    stage: &CuratedFounderResetStage,
    bundle: &CuratedFounderBundle,
    world: &HeadlessWorld,
    allow_existing_registry: bool,
) -> Result<(), CuratedFounderStagingError> {
    stage.receipt.validate()?;
    if bundle.identity.plan_receipt != stage.receipt {
        return Err(CuratedFounderStagingError::BundleMismatch {
            slot: None,
            field: "apply plan receipt",
        });
    }
    if stage.source_save_id != stage.source_save_identity
        || stage.source_save_identity != stage.receipt.source_save_identity
        || stage.deterministic_seed != stage.receipt.source_save_seed
        || stage.world_seed != stage.receipt.world_seed
        || stage.restored_tick != stage.receipt.restored_tick
        || stage.save_replacement.save_id != stage.source_save_id
        || stage.save_replacement.deterministic_seed != stage.deterministic_seed
        || stage.save_replacement.world_seed != stage.world_seed
        || stage.save_replacement.world_tick != stage.restored_tick
        || !stage.save_replacement.registry_persistence_deferred
    {
        return Err(CuratedFounderStagingError::Mismatch {
            field: "apply stage save and receipt identity",
        });
    }
    let expected_count = stage.receipt.target_population as usize;
    if bundle.entries.len() != expected_count
        || stage.ordered_founder_ids.len() != expected_count
        || stage.record_candidates.len() != expected_count
        || stage.archive_birth_intents.len() != expected_count
        || stage.target_agent_bindings.len() != expected_count
        || stage.expected_registry_identity.len() != expected_count
    {
        return Err(CuratedFounderStagingError::BundleMismatch {
            slot: None,
            field: "apply entry count",
        });
    }
    if world.seed() != stage.world_seed {
        return Err(CuratedFounderStagingError::Mismatch {
            field: "apply world seed",
        });
    }
    if world.tick() != stage.restored_tick {
        return Err(CuratedFounderStagingError::Mismatch {
            field: "apply world tick",
        });
    }
    world.validate_organism_bindings()?;
    if !allow_existing_registry && !world.organism_registry().is_empty() {
        return Err(CuratedFounderStagingError::ExistingRegistry {
            records: world.organism_registry().len(),
        });
    }

    let mut source_run_id = None;
    for (index, (((entry, intent), record), (target_binding, registry_identity))) in bundle
        .entries
        .iter()
        .zip(&stage.archive_birth_intents)
        .zip(&stage.record_candidates)
        .zip(
            stage
                .target_agent_bindings
                .iter()
                .zip(&stage.expected_registry_identity),
        )
        .enumerate()
    {
        let slot = index as u32;
        validate_archive_run_id(&intent.source_run_id)?;
        if source_run_id.is_some_and(|expected| expected != intent.source_run_id) {
            return Err(CuratedFounderStagingError::BundleMismatch {
                slot: Some(slot),
                field: "source run order",
            });
        }
        source_run_id = Some(intent.source_run_id.as_str());

        entry.genome.validate_contract().map_err(|source| {
            CuratedFounderStagingError::Contract {
                field: "apply bundle genome",
                source,
            }
        })?;
        entry.biochemistry.validate_contract().map_err(|source| {
            CuratedFounderStagingError::Contract {
                field: "apply bundle biochemistry",
                source,
            }
        })?;
        entry
            .projection
            .validate()
            .map_err(|source| CuratedFounderStagingError::Contract {
                field: "apply bundle projection",
                source,
            })?;
        entry
            .projection
            .receipt()
            .validate_against_projection(&entry.projection)
            .map_err(|source| CuratedFounderStagingError::Contract {
                field: "apply bundle projection receipt",
                source,
            })?;
        record
            .validate_contract()
            .map_err(|source| CuratedFounderStagingError::Contract {
                field: "apply record candidate",
                source,
            })?;
        intent.foundation.validate_contract().map_err(|source| {
            CuratedFounderStagingError::Contract {
                field: "apply intent foundation",
                source,
            }
        })?;
        intent
            .projection_receipt
            .validate_against_projection(&entry.projection)
            .map_err(|source| CuratedFounderStagingError::Contract {
                field: "apply intent projection receipt",
                source,
            })?;
        intent
            .organism_id
            .validate()
            .map_err(|source| CuratedFounderStagingError::Contract {
                field: "apply intent organism",
                source,
            })?;
        intent
            .genome_id
            .validate()
            .map_err(|source| CuratedFounderStagingError::Contract {
                field: "apply intent genome",
                source,
            })?;
        intent
            .lineage_id
            .validate()
            .map_err(|source| CuratedFounderStagingError::Contract {
                field: "apply intent lineage",
                source,
            })?;

        let receipt_identity = &stage.receipt.ordered_agent_identities[index];
        if entry.plan_entry.final_population_slot != receipt_identity.final_population_slot
            || entry.plan_entry.world_entity_id != receipt_identity.world_entity_id
            || entry.plan_entry.organism_id != receipt_identity.organism_id
            || stage.ordered_founder_ids[index] != entry.plan_entry.organism_id
            || intent.organism_id != entry.plan_entry.organism_id
            || record.organism_id() != entry.plan_entry.organism_id
            || record.world_entity_id() != entry.plan_entry.world_entity_id
            || *target_binding
                != (
                    entry.plan_entry.world_entity_id,
                    entry.plan_entry.organism_id,
                )
            || *registry_identity
                != (
                    entry.plan_entry.organism_id,
                    entry.plan_entry.world_entity_id,
                )
            || stage.receipt.derived_conception_seeds[index] != entry.plan_entry.conception_seed
            || stage.receipt.derived_genome_ids[index] != intent.genome_id
            || stage.receipt.derived_lineage_ids[index] != intent.lineage_id
        {
            return Err(CuratedFounderStagingError::BundleMismatch {
                slot: Some(slot),
                field: "apply ordered identity or target binding",
            });
        }
        if intent.genome_id != entry.genome.id
            || entry.plan_entry.genome_id != intent.genome_id
            || intent.lineage_id != entry.genome.lineage_id
            || entry.plan_entry.lineage_id != intent.lineage_id
            || entry.genome.conception_seed != entry.plan_entry.conception_seed
            || intent.foundation != entry.genome.foundation
            || record.genome() != &entry.genome
            || record.phenotype() != &entry.phenotype
            || record.biochemistry() != &entry.biochemistry
        {
            return Err(CuratedFounderStagingError::BundleMismatch {
                slot: Some(slot),
                field: "apply genome, phenotype, or biology pairing",
            });
        }
        if intent.birth_tick != stage.restored_tick
            || record.birth_tick() != stage.restored_tick
            || entry.biochemistry.tick != stage.restored_tick
            || !record.lifecycle().is_alive()
            || record.archive().birth_manifest_digest().is_some()
            || record.archive().life_manifest_digest().is_some()
        {
            return Err(CuratedFounderStagingError::BundleMismatch {
                slot: Some(slot),
                field: "apply fresh birth state or tick",
            });
        }
        if intent.foundation != stage.receipt.foundation
            || intent.foundation_content_digest != stage.receipt.foundation_content_digest
            || intent.sensor_profile != stage.receipt.sensor_profile
            || &intent.projection_receipt != entry.projection.receipt()
            || intent.phenotype_hash != intent.compiled_phenotype.phenotype_hash()
            || intent.phenotype_hash != entry.projection.receipt().phenotype_hash()
            || entry.projection.source_genome_id() != intent.genome_id
            || entry.projection.lineage_id() != intent.lineage_id
            || entry.projection.foundation() != &intent.foundation
            || entry.projection.sensor_profile() != intent.sensor_profile
            || entry.projection.foundation_asset_digest() != intent.foundation_content_digest
            || entry.projection.compiled_phenotype() != &intent.compiled_phenotype
        {
            return Err(CuratedFounderStagingError::BundleMismatch {
                slot: Some(slot),
                field: "apply projection, phenotype, or foundation pairing",
            });
        }

        let object = world.entity(entry.plan_entry.world_entity_id).ok_or(
            CuratedFounderStagingError::Mismatch {
                field: "apply target world entity",
            },
        )?;
        if object.kind != WorldObjectKind::Agent
            || object.organism_id != Some(entry.plan_entry.organism_id)
        {
            return Err(CuratedFounderStagingError::Mismatch {
                field: "apply target world binding",
            });
        }
    }
    Ok(())
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
        time::{Duration, Instant},
    };

    use alife_archive::{
        CompositeGeneticArchiveBatchInput, GeneticArchiveInput, LineageLibrary,
        LineageLibraryConfig,
    };
    use alife_core::{
        BiochemistryState, BrainCapacityClass, BrainScaleTier, FoundationGeneticIdentity,
        FoundationWeightAsset, GenomeId, OrganismId, SensorProfile, Tick, Vec3f, WorldEntityId,
    };
    use alife_runtime::GpuDurableSaveManifest;
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

    use super::{
        apply_curated_founder_reset, bind_curated_founder_source, build_archive_receipt_rows,
        publish_curated_founder_reset_durably, stage_curated_founder_reset,
        CuratedFounderDurablePublicationError, CuratedFounderStagingError,
    };

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
        asset_root: PathBuf,
        durable_save_path: PathBuf,
        durable_manifest: Option<GpuDurableSaveManifest>,
    }

    impl StageFixture {
        fn lineage_library(&self) -> &LineageLibrary {
            self.lineage_library.as_ref().unwrap()
        }

        fn lineage_library_mut(&mut self) -> &mut LineageLibrary {
            self.lineage_library.as_mut().unwrap()
        }

        fn durable_manifest(&self) -> &GpuDurableSaveManifest {
            self.durable_manifest.as_ref().unwrap()
        }
    }

    impl Drop for StageFixture {
        fn drop(&mut self) {
            let _ = self.lineage_library.take();
            let _ = self.durable_manifest.take();
            let _ = fs::remove_file(&self.durable_save_path);
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
        let asset_root = std::env::current_dir().unwrap();
        let durable_save_path = archive_root.with_extension("durable-save.json");
        GpuDurableSaveManifest::publish_snapshot(&durable_save_path, &asset_root, &source_save)
            .unwrap();
        let durable_manifest =
            GpuDurableSaveManifest::open(&durable_save_path, &asset_root).unwrap();
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
            asset_root,
            durable_save_path,
            durable_manifest: Some(durable_manifest),
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

    fn wait_for_publication_lease_ready(lease: &Path) -> bool {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if fs::read(lease).is_ok_and(|bytes| bytes == b"ready") {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::yield_now();
        }
    }

    fn wait_for_staged_manifest_path(root: &Path) -> Option<PathBuf> {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let mut pending = vec![root.join("staging")];
            while let Some(current) = pending.pop() {
                let Ok(entries) = fs::read_dir(&current) else {
                    continue;
                };
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        pending.push(path);
                        continue;
                    }
                    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                        continue;
                    };
                    if !name.starts_with("payload-") {
                        continue;
                    }
                    let Ok(bytes) = fs::read(&path) else {
                        continue;
                    };
                    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
                        continue;
                    };
                    if value.get("schema_version").is_none()
                        || value.get("genetic").is_none()
                        || value.get("life").is_none()
                    {
                        continue;
                    }
                    let Some(digest) = name.rsplit('-').next() else {
                        continue;
                    };
                    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                        continue;
                    }
                    return Some(root.join("manifests").join(format!("{digest}.json")));
                }
            }
            if Instant::now() >= deadline {
                return None;
            }
            std::thread::yield_now();
        }
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

    fn competing_valid_save(source: &PortableSaveFile) -> PortableSaveFile {
        let mut competing = source.clone();
        let mut world = source.restore_headless_world().unwrap();
        world.advance_tick();
        competing.replace_headless_world_snapshot(&world).unwrap();
        competing
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
                fixture.lineage_library.as_ref().unwrap(),
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
                fixture.lineage_library.as_ref().unwrap(),
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
            fixture.lineage_library.as_ref().unwrap(),
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
                fixture.lineage_library.as_ref().unwrap(),
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
                fixture.lineage_library.as_ref().unwrap(),
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
            wrong_seed_fixture.lineage_library.as_ref().unwrap(),
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
            wrong_tick_fixture.lineage_library.as_ref().unwrap(),
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
            asset_fixture.lineage_library.as_ref().unwrap(),
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
                fixture.lineage_library.as_ref().unwrap(),
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
            fixture.lineage_library.as_ref().unwrap(),
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
            fixture.lineage_library.as_ref().unwrap(),
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

    #[test]
    fn curated_apply_commits_one_ordered_archive_batch_before_registry_publish() {
        let mut fixture = stage_fixture("apply-order");
        let stage = stage_curated_founder_reset(
            &fixture.plan,
            &fixture.bundle,
            &fixture.source_save,
            &fixture.restored_world,
            fixture.lineage_library.as_ref().unwrap(),
            Path::new("."),
            "archive-run-3b3",
        )
        .unwrap();
        let before_world_signature = fixture.restored_world.canonical_signature_digest().unwrap();
        let before_registry = registry_snapshot(&fixture.restored_world);

        let result = {
            let lineage_library = fixture.lineage_library.as_mut().unwrap();
            apply_curated_founder_reset(
                &stage,
                &fixture.bundle,
                lineage_library,
                &mut fixture.restored_world,
            )
        }
        .unwrap();

        assert_eq!(result.committed_archive_batch.len(), 3);
        assert_eq!(
            result.committed_archive_batch.manifest_digests(),
            result
                .committed_archive_batch
                .entries()
                .iter()
                .map(|entry| entry.manifest_digest())
                .collect::<Vec<_>>()
        );
        assert_eq!(result.reset_receipt, stage.receipt);
        assert_eq!(
            result.applied_registry_identity,
            stage.expected_registry_identity
        );
        assert_eq!(
            result.applied_world_signature,
            fixture.restored_world.canonical_signature_digest().unwrap()
        );
        assert_ne!(
            fixture.restored_world.canonical_signature_digest().unwrap(),
            before_world_signature
        );
        assert_ne!(registry_snapshot(&fixture.restored_world), before_registry);
        assert_eq!(fixture.lineage_library().manifest_count().unwrap(), 3);

        for (index, intent) in stage.archive_birth_intents.iter().enumerate() {
            let committed = &result.committed_archive_batch.entries()[index];
            assert_eq!(committed.source_run_id(), intent.source_run_id);
            assert_eq!(committed.organism_id(), intent.organism_id);
            assert_eq!(committed.genome_id(), intent.genome_id);
            assert_eq!(committed.lineage_id(), intent.lineage_id);
            assert_eq!(committed.birth_tick(), intent.birth_tick);
            assert_eq!(
                fixture
                    .lineage_library()
                    .latest_manifest_for(&intent.source_run_id, intent.organism_id)
                    .unwrap(),
                Some(committed.manifest_digest())
            );
            assert_eq!(
                fixture
                    .restored_world
                    .organism_registry()
                    .get(intent.organism_id)
                    .unwrap()
                    .archive()
                    .birth_manifest_digest(),
                Some(committed.manifest_digest())
            );
        }
    }

    #[test]
    fn curated_apply_rejects_world_candidate_before_archive_commit() {
        let mut fixture = stage_fixture("apply-invalid-world-candidate");
        let stage = stage_curated_founder_reset(
            &fixture.plan,
            &fixture.bundle,
            &fixture.source_save,
            &fixture.restored_world,
            fixture.lineage_library.as_ref().unwrap(),
            Path::new("."),
            "archive-run-3b3",
        )
        .unwrap();
        fixture
            .restored_world
            .spawn_social_agent(
                "unexpected-agent",
                OrganismId(404),
                Vec3f::new(3.0, 0.0, 0.0),
                0.0,
            )
            .unwrap();
        let before = authority_snapshot(&fixture);

        let error = {
            let lineage_library = fixture.lineage_library.as_mut().unwrap();
            apply_curated_founder_reset(
                &stage,
                &fixture.bundle,
                lineage_library,
                &mut fixture.restored_world,
            )
        }
        .unwrap_err();

        assert!(matches!(error, CuratedFounderStagingError::World(_)));
        assert_authority_unchanged(before, &fixture);
    }

    #[test]
    fn curated_publication_conflict_leaves_old_save_and_live_world_after_archive_commit() {
        let mut fixture = stage_fixture("publication-conflict");
        let stage = stage_curated_founder_reset(
            &fixture.plan,
            &fixture.bundle,
            &fixture.source_save,
            &fixture.restored_world,
            fixture.lineage_library.as_ref().unwrap(),
            Path::new("."),
            "archive-run-3c2",
        )
        .unwrap();
        let bound_source = bind_curated_founder_source(fixture.durable_manifest()).unwrap();
        let competing_save = competing_valid_save(&fixture.source_save);
        GpuDurableSaveManifest::publish_snapshot(
            &fixture.durable_save_path,
            &fixture.asset_root,
            &competing_save,
        )
        .unwrap();
        let old_world_signature = fixture.restored_world.canonical_signature_digest().unwrap();
        let old_registry = registry_snapshot(&fixture.restored_world);

        let result = publish_curated_founder_reset_durably(
            &stage,
            &fixture.bundle,
            &bound_source,
            fixture.durable_manifest.as_ref().unwrap(),
            &fixture.asset_root,
            fixture.lineage_library.as_ref().unwrap(),
            &mut fixture.restored_world,
        );
        let error = result.unwrap_err();
        let (receipt, expected, actual, proposed) = match error {
            CuratedFounderDurablePublicationError::ArchiveCommittedSaveConflict {
                receipt,
                expected_save_digest,
                actual_save_digest,
                proposed_save_digest,
            } => (
                receipt,
                expected_save_digest,
                actual_save_digest,
                proposed_save_digest,
            ),
            other => panic!("unexpected publication result: {other:?}"),
        };

        assert_eq!(expected, bound_source.loaded_generation.digest.as_str());
        assert_eq!(
            actual,
            fixture.durable_manifest().load().unwrap().digest.as_str()
        );
        assert_eq!(proposed, receipt.proposed_save_digest);
        assert_eq!(
            receipt.expected_save_digest,
            bound_source.loaded_generation.digest.as_str()
        );
        assert_eq!(receipt.final_save_digest, None);
        assert_eq!(receipt.archive_receipts.len(), 3);
        assert_eq!(
            fixture.durable_manifest().load().unwrap().save,
            competing_save
        );
        assert_eq!(
            fixture.restored_world.canonical_signature_digest().unwrap(),
            old_world_signature
        );
        assert_eq!(registry_snapshot(&fixture.restored_world), old_registry);
        assert_eq!(fixture.lineage_library().manifest_count().unwrap(), 3);
        for row in &receipt.archive_receipts {
            assert_eq!(
                fixture
                    .lineage_library()
                    .latest_manifest_for(&receipt.archive_source_run, row.organism_id)
                    .unwrap(),
                Some(row.manifest_digest)
            );
            let manifest = fixture
                .lineage_library()
                .load_manifest(row.manifest_digest)
                .unwrap();
            assert_eq!(manifest.genetic.organism_id, row.organism_id);
            assert_eq!(manifest.genetic.birth_tick, row.birth_tick);
            assert!(manifest.life.is_none());
        }
    }

    #[test]
    fn curated_publication_success_reloads_registry_and_verified_archive_links() {
        let mut fixture = stage_fixture("publication-success");
        let stage = stage_curated_founder_reset(
            &fixture.plan,
            &fixture.bundle,
            &fixture.source_save,
            &fixture.restored_world,
            fixture.lineage_library.as_ref().unwrap(),
            Path::new("."),
            "archive-run-3c2",
        )
        .unwrap();
        let bound_source = bind_curated_founder_source(fixture.durable_manifest()).unwrap();

        let result = publish_curated_founder_reset_durably(
            &stage,
            &fixture.bundle,
            &bound_source,
            fixture.durable_manifest.as_ref().unwrap(),
            &fixture.asset_root,
            fixture.lineage_library.as_ref().unwrap(),
            &mut fixture.restored_world,
        )
        .unwrap();

        assert_eq!(
            result.status,
            super::CuratedFounderPublicationStatus::Published
        );
        assert_eq!(result.archive_receipts.len(), 3);
        assert_eq!(result.source_save_identity, fixture.source_save.save_id);
        assert_eq!(
            result.source_save_seed,
            fixture.source_save.deterministic_seed
        );
        assert_eq!(result.source_world_seed, fixture.source_save.world.seed);
        assert_eq!(result.source_tick, fixture.source_save.world.tick);
        assert_eq!(result.reset_receipt, stage.receipt);
        assert_eq!(
            result.candidate_world_schema_version,
            result.candidate_world_signature.schema_version
        );
        assert_eq!(result.candidate_world_seed, fixture.restored_world.seed());
        assert_eq!(result.candidate_world_tick, fixture.restored_world.tick());
        assert_eq!(
            result.candidate_world_signature,
            fixture.restored_world.canonical_signature_digest().unwrap()
        );

        let loaded = fixture.durable_manifest().load().unwrap();
        assert_eq!(loaded.save, {
            let mut expected = fixture.source_save.clone();
            expected
                .replace_headless_world_snapshot(&fixture.restored_world)
                .unwrap();
            expected
        });
        assert_eq!(
            result.final_save_digest.as_deref(),
            Some(loaded.digest.as_str())
        );
        assert_eq!(result.proposed_save_digest, loaded.digest.as_str());
        let restored = loaded.save.restore_headless_world().unwrap();
        assert_eq!(
            restored.canonical_signature_digest().unwrap(),
            result.candidate_world_signature
        );
        assert_eq!(
            registry_snapshot(&restored),
            registry_snapshot(&fixture.restored_world)
        );
        assert_eq!(
            result.archive_receipts[0].final_population_slot,
            fixture.bundle.entries[0].plan_entry.final_population_slot
        );
        for (batch_index, row) in result.archive_receipts.iter().enumerate() {
            assert_eq!(
                row.final_population_slot,
                fixture.bundle.entries[batch_index]
                    .plan_entry
                    .final_population_slot
            );
            assert_eq!(
                row.organism_id,
                stage.archive_birth_intents[batch_index].organism_id
            );
            let record = restored.organism_registry().get(row.organism_id).unwrap();
            assert_eq!(record.world_entity_id(), row.world_entity_id);
            assert_eq!(
                record.archive().birth_manifest_digest(),
                Some(row.manifest_digest)
            );
            assert_eq!(
                fixture
                    .lineage_library()
                    .latest_manifest_for(&result.archive_source_run, row.organism_id)
                    .unwrap(),
                Some(row.manifest_digest)
            );
            let manifest = fixture
                .lineage_library()
                .load_manifest(row.manifest_digest)
                .unwrap();
            assert_eq!(manifest.genetic.source_run_id, result.archive_source_run);
            assert_eq!(manifest.genetic.genome_id, row.genome_id);
            assert_eq!(manifest.genetic.lineage_id, Some(row.lineage_id));
            assert_eq!(manifest.genetic.birth_tick, row.birth_tick);
            assert_eq!(
                manifest.genetic.sensor_profile,
                stage.receipt.sensor_profile
            );
            assert_eq!(
                manifest.genetic.phenotype_hash,
                fixture.bundle.entries[batch_index]
                    .projection
                    .receipt()
                    .phenotype_hash()
            );
            assert!(manifest.life.is_none());
        }
    }

    #[test]
    fn curated_publication_accepts_minified_source_with_raw_generation_digest() {
        let mut fixture = stage_fixture("publication-minified-source");
        let minified_source = serde_json::to_string(&fixture.source_save).unwrap();
        fs::write(&fixture.durable_save_path, minified_source.as_bytes()).unwrap();
        let bound_source = bind_curated_founder_source(fixture.durable_manifest()).unwrap();
        let pretty_digest = PortableAssetDigest::for_bytes(
            &serde_json::to_vec_pretty(&bound_source.loaded_generation.save).unwrap(),
        )
        .0;
        assert_ne!(
            bound_source.loaded_generation.digest.as_str(),
            pretty_digest
        );

        let stage = stage_curated_founder_reset(
            &fixture.plan,
            &fixture.bundle,
            &fixture.source_save,
            &fixture.restored_world,
            fixture.lineage_library.as_ref().unwrap(),
            Path::new("."),
            "archive-run-3c2",
        )
        .unwrap();
        let result = publish_curated_founder_reset_durably(
            &stage,
            &fixture.bundle,
            &bound_source,
            fixture.durable_manifest.as_ref().unwrap(),
            &fixture.asset_root,
            fixture.lineage_library.as_ref().unwrap(),
            &mut fixture.restored_world,
        )
        .unwrap();

        assert_eq!(
            result.status,
            super::CuratedFounderPublicationStatus::Published
        );
        assert_eq!(
            result.expected_save_digest,
            bound_source.loaded_generation.digest.as_str()
        );
        assert_eq!(
            result.archive_receipts[0].final_population_slot,
            fixture.bundle.entries[0].plan_entry.final_population_slot
        );
    }

    #[test]
    fn curated_publication_receipt_uses_matching_plan_slot_for_reversed_batch() {
        let fixture = stage_fixture("publication-reversed-batch");
        let stage = stage_curated_founder_reset(
            &fixture.plan,
            &fixture.bundle,
            &fixture.source_save,
            &fixture.restored_world,
            fixture.lineage_library.as_ref().unwrap(),
            Path::new("."),
            "archive-run-3c2",
        )
        .unwrap();
        let foundation =
            FoundationWeightAsset::builtin_nano512_v1(stage.receipt.sensor_profile).unwrap();
        let foundation_asset_bytes = foundation.encode_canonical().unwrap();
        let reversed_inputs = (0..stage.archive_birth_intents.len())
            .rev()
            .map(|batch_index| {
                let entry = &fixture.bundle.entries[batch_index];
                let intent = &stage.archive_birth_intents[batch_index];
                CompositeGeneticArchiveBatchInput {
                    source_run_id: &intent.source_run_id,
                    organism_id: intent.organism_id,
                    genome_id: intent.genome_id,
                    lineage_id: intent.lineage_id,
                    birth_tick: intent.birth_tick,
                    foundation: intent.foundation,
                    foundation_content_digest: intent.foundation_content_digest,
                    sensor_profile: intent.sensor_profile,
                    projection_receipt: Some(&intent.projection_receipt),
                    phenotype_hash: intent.phenotype_hash,
                    creature_genome: &entry.genome,
                    phenotype: &intent.compiled_phenotype,
                    foundation_asset_bytes: &foundation_asset_bytes,
                }
            })
            .collect::<Vec<_>>();
        let prepared = fixture
            .lineage_library()
            .prepare_composite_birth_batch(&reversed_inputs)
            .unwrap();
        let committed = fixture
            .lineage_library()
            .commit_composite_birth_batch(prepared)
            .unwrap();

        let rows = build_archive_receipt_rows(&stage, &fixture.bundle, &committed).unwrap();

        assert_eq!(
            rows[0].organism_id,
            fixture.bundle.entries[2].plan_entry.organism_id
        );
        assert_eq!(rows[0].final_population_slot, 2);
        assert_eq!(
            rows[1].organism_id,
            fixture.bundle.entries[1].plan_entry.organism_id
        );
        assert_eq!(rows[1].final_population_slot, 1);
        assert_eq!(
            rows[2].organism_id,
            fixture.bundle.entries[0].plan_entry.organism_id
        );
        assert_eq!(rows[2].final_population_slot, 0);
    }

    #[test]
    fn curated_publication_rejects_mixed_bound_source_and_target_before_archive_commit() {
        let mut fixture = stage_fixture("publication-mixed-target");
        let stage = stage_curated_founder_reset(
            &fixture.plan,
            &fixture.bundle,
            &fixture.source_save,
            &fixture.restored_world,
            fixture.lineage_library.as_ref().unwrap(),
            Path::new("."),
            "archive-run-3c2",
        )
        .unwrap();
        let bound_source = bind_curated_founder_source(fixture.durable_manifest()).unwrap();
        let other_save_path = fixture
            .archive_root
            .with_extension("durable-save-other.json");
        GpuDurableSaveManifest::publish_snapshot(
            &other_save_path,
            &fixture.asset_root,
            &fixture.source_save,
        )
        .unwrap();
        let other_manifest =
            GpuDurableSaveManifest::open(&other_save_path, &fixture.asset_root).unwrap();
        let before = authority_snapshot(&fixture);
        let primary_save_before = fs::read(&fixture.durable_save_path).unwrap();
        let other_save_before = fs::read(&other_save_path).unwrap();

        let error = publish_curated_founder_reset_durably(
            &stage,
            &fixture.bundle,
            &bound_source,
            &other_manifest,
            &fixture.asset_root,
            fixture.lineage_library.as_ref().unwrap(),
            &mut fixture.restored_world,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            CuratedFounderDurablePublicationError::PreCommit(
                CuratedFounderStagingError::Mismatch {
                    field: "bound source durable manifest provenance"
                }
            )
        ));
        assert_authority_unchanged(before, &fixture);
        assert_eq!(
            fs::read(&fixture.durable_save_path).unwrap(),
            primary_save_before
        );
        assert_eq!(fs::read(&other_save_path).unwrap(), other_save_before);

        let other_asset_root = fixture.archive_root.join("asset-root-mismatch");
        fs::create_dir_all(&other_asset_root).unwrap();
        let root_mismatch_manifest =
            GpuDurableSaveManifest::open(&fixture.durable_save_path, &other_asset_root).unwrap();
        let before = authority_snapshot(&fixture);
        let primary_save_before = fs::read(&fixture.durable_save_path).unwrap();

        let error = publish_curated_founder_reset_durably(
            &stage,
            &fixture.bundle,
            &bound_source,
            &root_mismatch_manifest,
            &fixture.asset_root,
            fixture.lineage_library.as_ref().unwrap(),
            &mut fixture.restored_world,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            CuratedFounderDurablePublicationError::PreCommit(
                CuratedFounderStagingError::Mismatch {
                    field: "bound source durable manifest provenance"
                }
            )
        ));
        assert_authority_unchanged(before, &fixture);
        assert_eq!(
            fs::read(&fixture.durable_save_path).unwrap(),
            primary_save_before
        );

        drop(root_mismatch_manifest);
        drop(other_manifest);
        let _ = fs::remove_file(&other_save_path);
    }

    #[test]
    fn curated_publication_retry_reuses_archive_and_handles_already_applied() {
        let mut fixture = stage_fixture("publication-retry");
        let stage = stage_curated_founder_reset(
            &fixture.plan,
            &fixture.bundle,
            &fixture.source_save,
            &fixture.restored_world,
            fixture.lineage_library.as_ref().unwrap(),
            Path::new("."),
            "archive-run-3c2",
        )
        .unwrap();
        let bound_source = bind_curated_founder_source(fixture.durable_manifest()).unwrap();
        let competing_save = competing_valid_save(&fixture.source_save);
        GpuDurableSaveManifest::publish_snapshot(
            &fixture.durable_save_path,
            &fixture.asset_root,
            &competing_save,
        )
        .unwrap();
        let conflict = publish_curated_founder_reset_durably(
            &stage,
            &fixture.bundle,
            &bound_source,
            fixture.durable_manifest.as_ref().unwrap(),
            &fixture.asset_root,
            fixture.lineage_library.as_ref().unwrap(),
            &mut fixture.restored_world,
        )
        .unwrap_err();
        let conflict_receipt = match conflict {
            CuratedFounderDurablePublicationError::ArchiveCommittedSaveConflict {
                receipt, ..
            } => receipt,
            other => panic!("unexpected conflict result: {other:?}"),
        };
        let archive_after_conflict = archive_snapshot(fixture.lineage_library().root());
        let archive_count_after_conflict = fixture.lineage_library().manifest_count().unwrap();

        GpuDurableSaveManifest::publish_snapshot(
            &fixture.durable_save_path,
            &fixture.asset_root,
            &fixture.source_save,
        )
        .unwrap();
        let published = publish_curated_founder_reset_durably(
            &stage,
            &fixture.bundle,
            &bound_source,
            fixture.durable_manifest.as_ref().unwrap(),
            &fixture.asset_root,
            fixture.lineage_library.as_ref().unwrap(),
            &mut fixture.restored_world,
        )
        .unwrap();
        assert_eq!(
            published.status,
            super::CuratedFounderPublicationStatus::Published
        );
        assert_eq!(
            published.archive_receipts,
            conflict_receipt.archive_receipts
        );
        assert_eq!(
            fixture.lineage_library().manifest_count().unwrap(),
            archive_count_after_conflict
        );
        assert_eq!(
            archive_snapshot(fixture.lineage_library().root()),
            archive_after_conflict
        );
        let published_world_signature =
            fixture.restored_world.canonical_signature_digest().unwrap();

        let already_applied = publish_curated_founder_reset_durably(
            &stage,
            &fixture.bundle,
            &bound_source,
            fixture.durable_manifest.as_ref().unwrap(),
            &fixture.asset_root,
            fixture.lineage_library.as_ref().unwrap(),
            &mut fixture.restored_world,
        )
        .unwrap();
        assert_eq!(
            already_applied.status,
            super::CuratedFounderPublicationStatus::AlreadyApplied
        );
        assert_eq!(already_applied.archive_receipts, published.archive_receipts);
        assert_eq!(
            fixture.lineage_library().manifest_count().unwrap(),
            archive_count_after_conflict
        );
        assert_eq!(
            archive_snapshot(fixture.lineage_library().root()),
            archive_after_conflict
        );
        assert_eq!(
            fixture.restored_world.canonical_signature_digest().unwrap(),
            published_world_signature
        );
        assert_eq!(
            fixture.durable_manifest().load().unwrap().digest.as_str(),
            already_applied.final_save_digest.as_deref().unwrap()
        );
    }

    #[test]
    fn curated_apply_archive_commit_failure_leaves_world_unpublished() {
        let mut fixture = stage_fixture("apply-archive-failure");
        let stage = stage_curated_founder_reset(
            &fixture.plan,
            &fixture.bundle,
            &fixture.source_save,
            &fixture.restored_world,
            fixture.lineage_library.as_ref().unwrap(),
            Path::new("."),
            "archive-run-3b3",
        )
        .unwrap();
        let before_world_signature = fixture.restored_world.canonical_signature_digest().unwrap();
        let before_registry = registry_snapshot(&fixture.restored_world);
        let archive_root = fixture.archive_root.clone();
        let before_archive_count = fixture.lineage_library().manifest_count().unwrap();
        let publication_lease = archive_root
            .join("staging")
            .join(".composite-birth-publication-lease");
        fs::write(&publication_lease, b"hold").unwrap();

        let worker_stage = stage.clone();
        let worker_bundle = fixture.bundle.clone();
        let mut worker_world = fixture.restored_world.clone();
        let mut worker_library = fixture.lineage_library.take().unwrap();
        let worker = std::thread::spawn(move || {
            let archive_error = matches!(
                apply_curated_founder_reset(
                    &worker_stage,
                    &worker_bundle,
                    &mut worker_library,
                    &mut worker_world,
                ),
                Err(CuratedFounderStagingError::Archive(_))
            );
            (archive_error, worker_world, worker_library)
        });

        if !wait_for_publication_lease_ready(&publication_lease) {
            let _ = fs::remove_file(&publication_lease);
            let _ = worker.join();
            panic!("archive publication lease was not reached");
        }
        let collision_path = match wait_for_staged_manifest_path(&archive_root) {
            Some(path) => path,
            None => {
                let _ = fs::remove_file(&publication_lease);
                let _ = worker.join();
                panic!("staged archive manifest was not observed");
            }
        };
        fs::create_dir(&collision_path).unwrap();
        fs::remove_file(&publication_lease).unwrap();

        let (archive_error, worker_world, worker_library) = worker.join().unwrap();
        assert!(archive_error);
        assert_eq!(
            worker_world.canonical_signature_digest().unwrap(),
            before_world_signature
        );
        assert_eq!(registry_snapshot(&worker_world), before_registry);
        assert_eq!(
            worker_library.manifest_count().unwrap(),
            before_archive_count
        );
        assert_eq!(
            fs::read_dir(archive_root.join("manifests"))
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect::<Vec<_>>(),
            vec![collision_path]
        );
        drop(worker_library);
    }
}
