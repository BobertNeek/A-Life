//! Profile-local immutable creature archives with a rebuildable SQLite index.

mod bundle;

pub use bundle::{
    BundleImportReceipt, FounderBundleKind, ResolvedFounder, ResolvedFounderCohort,
    ResolvedGpuFounderCheckpoint, MAX_BUNDLE_UNCOMPRESSED_BYTES, MAX_COHORT_FOUNDERS,
};

use std::{
    collections::{HashMap, HashSet},
    fmt,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use alife_core::{
    ArchiveAssetKind, ArchiveAssetRef, ArchiveCheckpointDisposition, ArchiveCheckpointRef,
    ArchiveCheckpointRetention, ArchivePageRef, ArchiveRetirementReceipt, Blake3Digest,
    BrainCapacityClass, BrainGenome, BrainPhenotype, CreatureArchiveManifest, CreatureGenome,
    CreatureLifeArchiveRecord, ExperienceSequenceId, FoundationGeneticIdentity,
    FoundationWeightAsset, GeneticArchiveRecord, GenomeId, LineageId,
    N512FounderFoundationProjection, N512FounderProjectionReceipt, OrganismId,
    PassiveLifeStatistics, PhenotypeCompiler, PhenotypeHash, ScaffoldContractError, SensorProfile,
    Tick, Validate, CREATURE_ARCHIVE_SCHEMA_VERSION,
};
use rusqlite::{params, Connection};

pub const ARCHIVE_PAGE_BYTES: usize = 65_536;
pub const DEFAULT_FULL_STATE_QUOTA_BYTES: u64 = 4 * 1024 * 1024 * 1024;
pub const DEFAULT_MAX_TEMPORARY_PER_RUN: u32 = 64;
pub const DEFAULT_MAX_AUTOMATIC_PER_RUN: u32 = 24;
pub const MAX_COMPOSITE_BIRTH_BATCH_ITEMS: usize = 256;
pub const MAX_COMPOSITE_BIRTH_BATCH_BYTES: u64 = 32 * 1024 * 1024;

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    #[error("archive I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("archive database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("archive JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("archive contract error: {0}")]
    Contract(#[from] ScaffoldContractError),
    #[error("archive integrity error: {0}")]
    Integrity(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineageLibraryConfig {
    pub root: PathBuf,
    pub full_state_quota_bytes: u64,
    pub max_temporary_per_run: u32,
    pub max_automatic_per_run: u32,
}

impl LineageLibraryConfig {
    pub fn profile_default(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            full_state_quota_bytes: DEFAULT_FULL_STATE_QUOTA_BYTES,
            max_temporary_per_run: DEFAULT_MAX_TEMPORARY_PER_RUN,
            max_automatic_per_run: DEFAULT_MAX_AUTOMATIC_PER_RUN,
        }
    }

    fn validate(&self) -> Result<(), ArchiveError> {
        if self.root.as_os_str().is_empty()
            || self.full_state_quota_bytes == 0
            || self.max_temporary_per_run == 0
            || self.max_automatic_per_run == 0
        {
            return Err(ArchiveError::Integrity(
                "lineage library config must use nonzero bounded storage".to_string(),
            ));
        }
        Ok(())
    }
}

pub struct GeneticArchiveInput<'a> {
    pub source_run_id: &'a str,
    pub organism_id: OrganismId,
    pub birth_tick: Tick,
    pub genome: &'a BrainGenome,
    pub phenotype: &'a BrainPhenotype,
    pub foundation_asset_bytes: Option<&'a [u8]>,
}

pub struct CompositeGeneticArchiveInput<'a> {
    pub source_run_id: &'a str,
    pub organism_id: OrganismId,
    pub birth_tick: Tick,
    pub creature_genome: &'a CreatureGenome,
    pub phenotype: &'a BrainPhenotype,
    pub foundation_asset_bytes: &'a [u8],
}

/// Complete input boundary for the read-only composite-birth preparation phase.
///
/// The older [`CompositeGeneticArchiveInput`] remains source-compatible for
/// existing single-record callers. The batch boundary carries the explicit
/// identity and provenance values needed to validate a curated founder without
/// guessing any field from a hard-coded profile.
#[derive(Debug, Clone, Copy)]
pub struct CompositeGeneticArchiveBatchInput<'a> {
    pub source_run_id: &'a str,
    pub organism_id: OrganismId,
    pub genome_id: GenomeId,
    pub lineage_id: LineageId,
    pub birth_tick: Tick,
    pub foundation: FoundationGeneticIdentity,
    /// Semantic foundation weight-payload digest. This is not the raw CAS
    /// digest of the canonical foundation file.
    pub foundation_content_digest: Blake3Digest,
    pub sensor_profile: SensorProfile,
    pub projection_receipt: Option<&'a N512FounderProjectionReceipt>,
    pub phenotype_hash: PhenotypeHash,
    pub creature_genome: &'a CreatureGenome,
    pub phenotype: &'a BrainPhenotype,
    pub foundation_asset_bytes: &'a [u8],
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedCompositeBirth {
    source_run_id: String,
    organism_id: OrganismId,
    genome_id: GenomeId,
    lineage_id: LineageId,
    birth_tick: Tick,
    manifest_digest: Blake3Digest,
    manifest: CreatureArchiveManifest,
}

impl PreparedCompositeBirth {
    pub fn source_run_id(&self) -> &str {
        &self.source_run_id
    }

    pub const fn organism_id(&self) -> OrganismId {
        self.organism_id
    }

    pub const fn genome_id(&self) -> GenomeId {
        self.genome_id
    }

    pub const fn lineage_id(&self) -> LineageId {
        self.lineage_id
    }

    pub const fn birth_tick(&self) -> Tick {
        self.birth_tick
    }

    pub const fn manifest_digest(&self) -> Blake3Digest {
        self.manifest_digest
    }

    pub const fn manifest(&self) -> &CreatureArchiveManifest {
        &self.manifest
    }
}

#[allow(dead_code)]
pub struct PreparedCompositeBirthBatch {
    items: Vec<PreparedCompositeBirth>,
    payloads: Vec<PreparedArchivePayload>,
    observations: PreparedArchiveObservation,
    aggregate_bytes: u64,
}

impl fmt::Debug for PreparedCompositeBirthBatch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedCompositeBirthBatch")
            .field("items", &self.items)
            .finish()
    }
}

impl PreparedCompositeBirthBatch {
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn items(&self) -> &[PreparedCompositeBirth] {
        &self.items
    }

    pub fn manifest_digests(&self) -> Vec<Blake3Digest> {
        self.items
            .iter()
            .map(PreparedCompositeBirth::manifest_digest)
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreparedPayloadDestination {
    Asset(ArchiveAssetKind),
    Manifest,
}

#[derive(Debug)]
struct PreparedArchivePayload {
    digest: Blake3Digest,
    bytes: Vec<u8>,
    destinations: Vec<PreparedPayloadDestination>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct PreparedManifestObservation {
    digest: Blake3Digest,
    manifest: CreatureArchiveManifest,
    raw_bytes: Vec<u8>,
    indexed_rows: Vec<PreparedIndexedManifestRow>,
    indexed: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PreparedIndexedManifestRow {
    digest: Blake3Digest,
    source_run_id: String,
    organism_id: OrganismId,
    genome_id: GenomeId,
    is_life: bool,
    death_tick: Option<Tick>,
}

#[allow(dead_code)]
#[derive(Debug)]
struct PreparedTargetObservation {
    source_run_id: String,
    organism_id: OrganismId,
    indexed_manifests: Vec<PreparedManifestObservation>,
    final_manifest_files: Vec<PreparedManifestObservation>,
}

#[allow(dead_code)]
#[derive(Debug)]
struct PreparedFinalFileObservation {
    destination: PreparedPayloadDestination,
    expected_digest: Blake3Digest,
    canonical_path: PathBuf,
    existed: bool,
    observed_bytes: Vec<u8>,
    observed_digest: Option<Blake3Digest>,
    size_bytes: u64,
}

#[allow(dead_code)]
#[derive(Debug)]
struct PreparedArchiveObservation {
    archive_root: PathBuf,
    targets: Vec<PreparedTargetObservation>,
    final_files: Vec<PreparedFinalFileObservation>,
}

struct ExistingManifestFile {
    digest: Blake3Digest,
    manifest: CreatureArchiveManifest,
    raw_bytes: Vec<u8>,
}

pub struct LifeArchiveInput<'a> {
    pub birth_manifest_digest: Blake3Digest,
    pub death_tick: Tick,
    pub final_experience_sequence: Option<ExperienceSequenceId>,
    pub statistics_bytes: &'a [u8],
    pub learned_checkpoint_bytes: Option<&'a [u8]>,
    pub checkpoint_retention: ArchiveCheckpointRetention,
}

pub struct LineageLibrary {
    config: LineageLibraryConfig,
    connection: Connection,
}

impl LineageLibrary {
    pub fn open(config: LineageLibraryConfig) -> Result<Self, ArchiveError> {
        config.validate()?;
        fs::create_dir_all(config.root.join("manifests"))?;
        fs::create_dir_all(config.root.join("assets"))?;
        fs::create_dir_all(config.root.join("checkpoints"))?;
        let staging = config.root.join("staging");
        if staging.exists() {
            fs::remove_dir_all(&staging)?;
        }
        fs::create_dir_all(&staging)?;

        let database = config.root.join("lineage.db");
        let connection = match open_index(&database) {
            Ok(connection) => connection,
            Err(_) => {
                if database.exists() {
                    fs::remove_file(&database)?;
                }
                open_index(&database)?
            }
        };
        let mut library = Self { config, connection };
        library.rebuild_index()?;
        Ok(library)
    }

    pub fn root(&self) -> &Path {
        &self.config.root
    }

    pub fn archive_birth(
        &mut self,
        input: GeneticArchiveInput<'_>,
    ) -> Result<Blake3Digest, ArchiveError> {
        self.archive_birth_internal(input, None)
    }

    pub fn archive_composite_birth(
        &mut self,
        input: CompositeGeneticArchiveInput<'_>,
    ) -> Result<Blake3Digest, ArchiveError> {
        input.creature_genome.validate_contract()?;
        let expressed = input.creature_genome.express()?;
        let foundation = FoundationWeightAsset::decode_canonical(input.foundation_asset_bytes)?;
        let development = expressed.development_state_at(Tick::new(u64::from(
            expressed.development.maturation_duration_ticks,
        )))?;
        let expected = PhenotypeCompiler::compile_from_foundation_asset(
            &expressed.brain_genome,
            &BrainCapacityClass::production_for_id(
                input.creature_genome.foundation.brain_class_id,
            )?,
            &development,
            SensorProfile::GroundedObjectSlotsV1,
            &foundation,
        )?;
        if expected.phenotype_hash() != input.phenotype.phenotype_hash()
            || expected.foundation_abi() != input.phenotype.foundation_abi()
        {
            return Err(ArchiveError::Integrity(
                "composite genome phenotype does not match independent compilation".to_string(),
            ));
        }
        self.archive_birth_internal(
            GeneticArchiveInput {
                source_run_id: input.source_run_id,
                organism_id: input.organism_id,
                birth_tick: input.birth_tick,
                genome: &expressed.brain_genome,
                phenotype: input.phenotype,
                foundation_asset_bytes: Some(input.foundation_asset_bytes),
            },
            Some(input.creature_genome),
        )
    }

    /// Reads and validates an ordered composite-birth batch without changing
    /// the archive. The owned payloads and observations are private so the
    /// later commit phase can consume them without exposing mutable storage or
    /// staging details to callers.
    pub fn prepare_composite_birth_batch(
        &self,
        inputs: &[CompositeGeneticArchiveBatchInput<'_>],
    ) -> Result<PreparedCompositeBirthBatch, ArchiveError> {
        if inputs.is_empty() {
            return Err(ArchiveError::Integrity(
                "composite birth batch cannot be empty".to_string(),
            ));
        }
        if inputs.len() > MAX_COMPOSITE_BIRTH_BATCH_ITEMS {
            return Err(ArchiveError::Integrity(format!(
                "composite birth batch exceeds {} items",
                MAX_COMPOSITE_BIRTH_BATCH_ITEMS
            )));
        }

        let mut target_keys = HashSet::with_capacity(inputs.len());
        let mut organism_ids = HashSet::with_capacity(inputs.len());
        let mut genome_ids = HashSet::with_capacity(inputs.len());
        let mut lineage_ids = HashSet::with_capacity(inputs.len());
        for input in inputs {
            validate_run_id(input.source_run_id)?;
            input.organism_id.validate()?;
            input.genome_id.validate()?;
            input.lineage_id.validate()?;
            input.foundation.validate_contract()?;
            if !target_keys.insert((input.source_run_id, input.organism_id))
                || !organism_ids.insert(input.organism_id)
                || !genome_ids.insert(input.genome_id)
                || !lineage_ids.insert(input.lineage_id)
            {
                return Err(ArchiveError::Integrity(
                    "composite birth batch contains duplicate target or identity".to_string(),
                ));
            }
        }

        let mut items = Vec::with_capacity(inputs.len());
        let mut payloads = Vec::new();
        let mut aggregate_bytes = 0_u64;
        for input in inputs {
            items.push(self.prepare_composite_birth_item(
                input,
                &mut payloads,
                &mut aggregate_bytes,
            )?);
        }

        let archive_root = canonical_archive_root(&self.config.root)?;
        let indexed_by_digest = self.observe_indexed_manifests(&archive_root)?;
        let mut targets = inputs
            .iter()
            .map(|input| PreparedTargetObservation {
                source_run_id: input.source_run_id.to_string(),
                organism_id: input.organism_id,
                indexed_manifests: Vec::new(),
                final_manifest_files: Vec::new(),
            })
            .collect::<Vec<_>>();
        let target_indexes = inputs
            .iter()
            .enumerate()
            .map(|(index, input)| ((input.source_run_id.to_string(), input.organism_id), index))
            .collect::<HashMap<_, _>>();

        for (index, input) in inputs.iter().enumerate() {
            targets[index].indexed_manifests = indexed_by_digest
                .values()
                .flatten()
                .filter(|observation| {
                    observation.manifest.genetic.source_run_id == input.source_run_id
                        && observation.manifest.genetic.organism_id == input.organism_id
                })
                .cloned()
                .collect();
        }

        for existing in self.scan_existing_manifest_files(&archive_root)? {
            let key = (
                existing.manifest.genetic.source_run_id.clone(),
                existing.manifest.genetic.organism_id,
            );
            let Some(index) = target_indexes.get(&key).copied() else {
                continue;
            };
            let indexed_rows = indexed_by_digest
                .get(&existing.digest)
                .into_iter()
                .flatten()
                .flat_map(|observation| observation.indexed_rows.iter().cloned())
                .collect::<Vec<_>>();
            targets[index]
                .final_manifest_files
                .push(PreparedManifestObservation {
                    digest: existing.digest,
                    manifest: existing.manifest,
                    raw_bytes: existing.raw_bytes,
                    indexed: !indexed_rows.is_empty(),
                    indexed_rows,
                });
        }

        for (item, target) in items.iter().zip(&targets) {
            for observation in target
                .indexed_manifests
                .iter()
                .chain(&target.final_manifest_files)
            {
                if observation.manifest.life.is_some()
                    || observation.digest != item.manifest_digest
                    || observation.manifest != item.manifest
                {
                    return Err(ArchiveError::Integrity(format!(
                        "existing archive state conflicts with composite birth target {}:{}",
                        item.source_run_id, item.organism_id.0
                    )));
                }
            }
        }

        let mut final_files = Vec::new();
        for payload in &payloads {
            for destination in &payload.destinations {
                final_files.push(observe_prepared_final_file(
                    &archive_root,
                    *destination,
                    payload.digest,
                    &payload.bytes,
                )?);
            }
        }

        Ok(PreparedCompositeBirthBatch {
            items,
            payloads,
            observations: PreparedArchiveObservation {
                archive_root,
                targets,
                final_files,
            },
            aggregate_bytes,
        })
    }

    fn prepare_composite_birth_item(
        &self,
        input: &CompositeGeneticArchiveBatchInput<'_>,
        payloads: &mut Vec<PreparedArchivePayload>,
        aggregate_bytes: &mut u64,
    ) -> Result<PreparedCompositeBirth, ArchiveError> {
        if input.foundation != input.creature_genome.foundation
            || input.genome_id != input.creature_genome.id
            || input.lineage_id != input.creature_genome.lineage_id
        {
            return Err(ArchiveError::Integrity(
                "composite birth explicit identity does not match creature genome".to_string(),
            ));
        }
        input.creature_genome.validate_contract()?;
        let expressed = input.creature_genome.express()?;
        if expressed.brain_genome.id != input.genome_id
            || expressed.brain_genome.lineage_id != Some(input.lineage_id)
        {
            return Err(ArchiveError::Integrity(
                "expressed brain genome identity does not match composite input".to_string(),
            ));
        }

        let capacity = BrainCapacityClass::production_for_id(input.foundation.brain_class_id)?;
        input.phenotype.validate_against(&capacity)?;
        if input.phenotype.sensor_profile() != input.sensor_profile
            || input.phenotype.phenotype_hash() != input.phenotype_hash
        {
            return Err(ArchiveError::Integrity(
                "composite birth phenotype profile or hash does not match input".to_string(),
            ));
        }
        validate_foundation_identity(input, input.phenotype)?;
        ensure_prepared_payload_within_limit(input.foundation_asset_bytes.len())?;
        if input.foundation_asset_bytes.is_empty() {
            return Err(ArchiveError::Integrity(
                "composite birth foundation bytes cannot be empty".to_string(),
            ));
        }
        let foundation = FoundationWeightAsset::decode_canonical(input.foundation_asset_bytes)?;
        if foundation.encode_canonical()? != input.foundation_asset_bytes {
            return Err(ArchiveError::Integrity(
                "composite birth foundation bytes are not canonical".to_string(),
            ));
        }
        let foundation_manifest = foundation.manifest();
        if foundation_manifest.foundation_id().raw() != input.foundation.foundation_id
            || u32::from(foundation_manifest.foundation_version().raw())
                != u32::from(input.foundation.version)
            || foundation_manifest.compatibility_family_id().raw()
                != input.foundation.compatibility_family_id
        {
            return Err(ArchiveError::Integrity(
                "composite birth foundation identity does not match bytes".to_string(),
            ));
        }
        foundation.validate_against(input.phenotype)?;
        if foundation.digest() != input.foundation_content_digest {
            return Err(ArchiveError::Integrity(
                "composite birth semantic foundation digest does not match bytes".to_string(),
            ));
        }

        if let Some(receipt) = input.projection_receipt {
            let projection = N512FounderFoundationProjection::compile(
                &expressed,
                input.sensor_profile,
                &foundation,
            )?;
            receipt.validate_against_projection(&projection)?;
            if projection.compiled_phenotype() != input.phenotype {
                return Err(ArchiveError::Integrity(
                    "composite birth projection phenotype does not match caller".to_string(),
                ));
            }
        } else {
            let development = expressed.development_state_at(Tick::new(u64::from(
                expressed.development.maturation_duration_ticks,
            )))?;
            let expected = PhenotypeCompiler::compile_from_foundation_asset(
                &expressed.brain_genome,
                &capacity,
                &development,
                input.sensor_profile,
                &foundation,
            )?;
            if expected != *input.phenotype || expected.phenotype_hash() != input.phenotype_hash {
                return Err(ArchiveError::Integrity(
                    "generic composite birth phenotype does not match caller".to_string(),
                ));
            }
        }

        let genome_bytes = serde_json::to_vec(&expressed.brain_genome)?;
        let composite_genome_bytes = serde_json::to_vec(input.creature_genome)?;
        let foundation_bytes = input.foundation_asset_bytes.to_vec();
        let genome_asset = ArchiveAssetRef {
            kind: ArchiveAssetKind::Genome,
            digest: digest_bytes(&genome_bytes),
            size_bytes: checked_byte_len(genome_bytes.len())?,
        };
        let composite_genome_asset = ArchiveAssetRef {
            kind: ArchiveAssetKind::CompositeGenome,
            digest: digest_bytes(&composite_genome_bytes),
            size_bytes: checked_byte_len(composite_genome_bytes.len())?,
        };
        let foundation_asset = ArchiveAssetRef {
            kind: ArchiveAssetKind::Foundation,
            digest: digest_bytes(&foundation_bytes),
            size_bytes: checked_byte_len(foundation_bytes.len())?,
        };
        let abi = input.phenotype.foundation_abi();
        let language = input.phenotype.language_codebook();
        let manifest = CreatureArchiveManifest {
            schema_version: CREATURE_ARCHIVE_SCHEMA_VERSION,
            genetic: GeneticArchiveRecord {
                source_run_id: input.source_run_id.to_string(),
                organism_id: input.organism_id,
                genome_id: input.genome_id,
                lineage_id: Some(input.lineage_id),
                brain_class_id: input.phenotype.brain_class_id(),
                birth_tick: input.birth_tick,
                sensor_profile: input.sensor_profile,
                phenotype_hash: input.phenotype_hash,
                foundation_id: abi.foundation_id(),
                foundation_version: abi.foundation_version(),
                compatibility_family_id: abi.compatibility_family_id(),
                foundation_payload_digest: Some(input.foundation_content_digest),
                persistent_address_map_digest: input.phenotype.persistent_address_map().digest(),
                language_codebook_id: language.id(),
                language_codebook_digest: language.canonical_digest(),
                genome_asset,
                composite_genome_asset: Some(composite_genome_asset),
                foundation_asset: Some(foundation_asset),
            },
            previous_manifest_digest: None,
            life: None,
        };
        manifest.validate_contract()?;
        let manifest_bytes = serde_json::to_vec(&manifest)?;
        let manifest_digest = digest_bytes(&manifest_bytes);

        register_prepared_payload(
            payloads,
            genome_bytes,
            PreparedPayloadDestination::Asset(ArchiveAssetKind::Genome),
            aggregate_bytes,
        )?;
        register_prepared_payload(
            payloads,
            composite_genome_bytes,
            PreparedPayloadDestination::Asset(ArchiveAssetKind::CompositeGenome),
            aggregate_bytes,
        )?;
        register_prepared_payload(
            payloads,
            foundation_bytes,
            PreparedPayloadDestination::Asset(ArchiveAssetKind::Foundation),
            aggregate_bytes,
        )?;
        register_prepared_payload(
            payloads,
            manifest_bytes,
            PreparedPayloadDestination::Manifest,
            aggregate_bytes,
        )?;

        Ok(PreparedCompositeBirth {
            source_run_id: input.source_run_id.to_string(),
            organism_id: input.organism_id,
            genome_id: input.genome_id,
            lineage_id: input.lineage_id,
            birth_tick: input.birth_tick,
            manifest_digest,
            manifest,
        })
    }

    fn observe_indexed_manifests(
        &self,
        archive_root: &Path,
    ) -> Result<HashMap<Blake3Digest, Vec<PreparedManifestObservation>>, ArchiveError> {
        let mut statement = self.connection.prepare(
            "SELECT digest,source_run_id,organism_id,genome_id,is_life,death_tick \
             FROM manifests ORDER BY rowid",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })?;
        let mut observations = HashMap::new();
        for row in rows {
            let (
                digest_text,
                source_run_id,
                organism_id_text,
                genome_id_text,
                is_life_value,
                death_tick_text,
            ) = row?;
            let digest = parse_digest_hex(&digest_text)?;
            if digest_hex(digest) != digest_text {
                return Err(ArchiveError::Integrity(
                    "archive index contains a noncanonical manifest digest".to_string(),
                ));
            }
            validate_run_id(&source_run_id)?;
            let organism_id = OrganismId(organism_id_text.parse::<u64>().map_err(|_| {
                ArchiveError::Integrity("archive index contains an invalid organism id".to_string())
            })?);
            organism_id.validate()?;
            let genome_id = GenomeId(genome_id_text.parse::<u64>().map_err(|_| {
                ArchiveError::Integrity("archive index contains an invalid genome id".to_string())
            })?);
            genome_id.validate()?;
            let is_life = match is_life_value {
                0 => false,
                1 => true,
                _ => {
                    return Err(ArchiveError::Integrity(
                        "archive index contains an invalid life flag".to_string(),
                    ));
                }
            };
            let death_tick = death_tick_text
                .as_deref()
                .map(|text| {
                    text.parse::<u64>().map(Tick::new).map_err(|_| {
                        ArchiveError::Integrity(
                            "archive index contains an invalid death tick".to_string(),
                        )
                    })
                })
                .transpose()?;
            let indexed_row = PreparedIndexedManifestRow {
                digest,
                source_run_id: source_run_id.clone(),
                organism_id,
                genome_id,
                is_life,
                death_tick,
            };
            let (raw_bytes, manifest) = self.load_manifest_with_bytes(archive_root, digest)?;
            if manifest.genetic.source_run_id != source_run_id
                || manifest.genetic.organism_id != organism_id
                || manifest.genetic.genome_id != genome_id
                || manifest.life.is_some() != is_life
                || manifest.life.as_ref().map(|life| life.death_tick) != death_tick
            {
                return Err(ArchiveError::Integrity(
                    "archive index row does not match its manifest".to_string(),
                ));
            }
            self.validate_manifest_assets(&manifest)?;
            observations
                .entry(digest)
                .or_insert_with(Vec::new)
                .push(PreparedManifestObservation {
                    digest,
                    manifest,
                    raw_bytes,
                    indexed_rows: vec![indexed_row],
                    indexed: true,
                });
        }
        Ok(observations)
    }

    fn scan_existing_manifest_files(
        &self,
        archive_root: &Path,
    ) -> Result<Vec<ExistingManifestFile>, ArchiveError> {
        let manifests_dir = checked_archive_path(archive_root, &archive_root.join("manifests"))?;
        let mut paths = fs::read_dir(manifests_dir.canonical_path)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, std::io::Error>>()?;
        paths.retain(|path| {
            path.extension().and_then(|extension| extension.to_str()) == Some("json")
        });
        paths.sort_by_key(|path| path.file_name().map(ToOwned::to_owned));

        let mut manifests = Vec::with_capacity(paths.len());
        for path in paths {
            let stem = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .ok_or_else(|| ArchiveError::Integrity("invalid manifest file name".to_string()))?;
            let digest = parse_digest_hex(stem)?;
            if digest_hex(digest) != stem {
                return Err(ArchiveError::Integrity(
                    "manifest path does not use canonical digest spelling".to_string(),
                ));
            }
            let checked_path = checked_archive_path(archive_root, &path)?;
            let bytes = fs::read(&checked_path.canonical_path)?;
            if digest_bytes(&bytes) != digest {
                return Err(ArchiveError::Integrity(format!(
                    "manifest file digest mismatch at {}",
                    path.display()
                )));
            }
            let manifest = serde_json::from_slice::<CreatureArchiveManifest>(&bytes)?;
            manifest.validate_contract()?;
            validate_run_id(&manifest.genetic.source_run_id)?;
            self.validate_manifest_assets(&manifest)?;
            manifests.push(ExistingManifestFile {
                digest,
                manifest,
                raw_bytes: bytes,
            });
        }
        Ok(manifests)
    }

    fn validate_manifest_assets(
        &self,
        manifest: &CreatureArchiveManifest,
    ) -> Result<(), ArchiveError> {
        let _ = self.load_brain_genome(manifest)?;
        if manifest.genetic.composite_genome_asset.is_some() {
            let _ = self.load_creature_genome(manifest)?;
        }
        if let Some(reference) = &manifest.genetic.foundation_asset {
            let bytes = self.read_archive_asset(reference)?;
            let foundation = FoundationWeightAsset::decode_canonical(&bytes)?;
            if foundation.encode_canonical()? != bytes {
                return Err(ArchiveError::Integrity(
                    "foundation asset is not in canonical form".to_string(),
                ));
            }
            let foundation_manifest = foundation.manifest();
            if manifest.genetic.foundation_id != Some(foundation_manifest.foundation_id())
                || manifest.genetic.foundation_version
                    != Some(foundation_manifest.foundation_version())
                || manifest.genetic.compatibility_family_id
                    != Some(foundation_manifest.compatibility_family_id())
                || manifest.genetic.foundation_payload_digest != Some(foundation.digest())
            {
                return Err(ArchiveError::Integrity(
                    "foundation asset identity does not match manifest".to_string(),
                ));
            }
        }
        if manifest.life.is_some() {
            let _ = self.load_life_statistics(manifest)?;
            if let Some(life) = &manifest.life {
                if let ArchiveCheckpointDisposition::Stored(reference) = &life.checkpoint {
                    let _ = self.read_checkpoint(reference)?;
                }
            }
        }
        Ok(())
    }

    fn read_archive_asset(&self, reference: &ArchiveAssetRef) -> Result<Vec<u8>, ArchiveError> {
        reference.validate_contract()?;
        let archive_root = canonical_archive_root(&self.config.root)?;
        let path = archive_root
            .join("assets")
            .join(digest_hex(reference.digest))
            .join("payload.bin");
        let checked_path = checked_archive_path(&archive_root, &path)?;
        let bytes = fs::read(&checked_path.canonical_path)?;
        if checked_byte_len(bytes.len())? != reference.size_bytes
            || digest_bytes(&bytes) != reference.digest
        {
            return Err(ArchiveError::Integrity(format!(
                "archive asset digest mismatch at {}",
                path.display()
            )));
        }
        Ok(bytes)
    }

    fn archive_birth_internal(
        &mut self,
        input: GeneticArchiveInput<'_>,
        composite_genome: Option<&CreatureGenome>,
    ) -> Result<Blake3Digest, ArchiveError> {
        validate_run_id(input.source_run_id)?;
        input.organism_id.validate()?;
        input.genome.validate_contract()?;
        let genome_bytes = serde_json::to_vec(input.genome)?;
        let genome_asset = self.write_asset(ArchiveAssetKind::Genome, &genome_bytes)?;
        let composite_genome_asset = composite_genome
            .map(|genome| serde_json::to_vec(genome))
            .transpose()?
            .map(|bytes| self.write_asset(ArchiveAssetKind::CompositeGenome, &bytes))
            .transpose()?;
        let foundation_asset = input
            .foundation_asset_bytes
            .map(|bytes| self.write_asset(ArchiveAssetKind::Foundation, bytes))
            .transpose()?;
        let abi = input.phenotype.foundation_abi();
        let language = input.phenotype.language_codebook();
        let manifest = CreatureArchiveManifest {
            schema_version: CREATURE_ARCHIVE_SCHEMA_VERSION,
            genetic: GeneticArchiveRecord {
                source_run_id: input.source_run_id.to_string(),
                organism_id: input.organism_id,
                genome_id: input.genome.id,
                lineage_id: input.genome.lineage_id,
                brain_class_id: input.phenotype.brain_class_id(),
                birth_tick: input.birth_tick,
                sensor_profile: input.phenotype.sensor_profile(),
                phenotype_hash: input.phenotype.phenotype_hash(),
                foundation_id: abi.foundation_id(),
                foundation_version: abi.foundation_version(),
                compatibility_family_id: abi.compatibility_family_id(),
                foundation_payload_digest: abi.foundation_payload_digest(),
                persistent_address_map_digest: input.phenotype.persistent_address_map().digest(),
                language_codebook_id: language.id(),
                language_codebook_digest: language.canonical_digest(),
                genome_asset,
                composite_genome_asset,
                foundation_asset,
            },
            previous_manifest_digest: None,
            life: None,
        };
        manifest.validate_contract()?;
        let digest = self.write_manifest(&manifest)?;
        self.index_manifest(digest, &manifest)?;
        Ok(digest)
    }

    pub fn archive_life(
        &mut self,
        input: LifeArchiveInput<'_>,
    ) -> Result<ArchiveRetirementReceipt, ArchiveError> {
        if input.statistics_bytes.is_empty() {
            return Err(ArchiveError::Integrity(
                "life archive requires final statistics".to_string(),
            ));
        }
        let birth = self.load_manifest(input.birth_manifest_digest)?;
        if birth.life.is_some() || birth.previous_manifest_digest.is_some() {
            return Err(ArchiveError::Integrity(
                "life archive must extend an immutable birth manifest".to_string(),
            ));
        }
        let statistics_asset =
            self.write_asset(ArchiveAssetKind::LifeStatistics, input.statistics_bytes)?;
        let checkpoint = match input.learned_checkpoint_bytes {
            Some(bytes) => self.store_checkpoint(
                &birth.genetic.source_run_id,
                bytes,
                input.checkpoint_retention,
            )?,
            None => ArchiveCheckpointDisposition::NotSelected,
        };
        let learned_checkpoint_digest = match &checkpoint {
            ArchiveCheckpointDisposition::Stored(reference) => Some(reference.digest),
            _ => None,
        };
        let manifest = CreatureArchiveManifest {
            schema_version: CREATURE_ARCHIVE_SCHEMA_VERSION,
            genetic: birth.genetic,
            previous_manifest_digest: Some(input.birth_manifest_digest),
            life: Some(CreatureLifeArchiveRecord {
                death_tick: input.death_tick,
                final_experience_sequence: input.final_experience_sequence,
                statistics_asset,
                checkpoint,
            }),
        };
        manifest.validate_contract()?;
        let digest = self.write_manifest(&manifest)?;
        self.index_manifest(digest, &manifest)?;
        let receipt = ArchiveRetirementReceipt {
            organism_id: manifest.genetic.organism_id,
            committed_manifest_digest: digest,
            learned_checkpoint_digest,
            death_tick: input.death_tick,
        };
        receipt.validate_contract()?;
        Ok(receipt)
    }

    pub fn load_manifest(
        &self,
        digest: Blake3Digest,
    ) -> Result<CreatureArchiveManifest, ArchiveError> {
        let archive_root = canonical_archive_root(&self.config.root)?;
        let (_, manifest) = self.load_manifest_with_bytes(&archive_root, digest)?;
        Ok(manifest)
    }

    fn load_manifest_with_bytes(
        &self,
        archive_root: &Path,
        digest: Blake3Digest,
    ) -> Result<(Vec<u8>, CreatureArchiveManifest), ArchiveError> {
        let path = archive_root
            .join("manifests")
            .join(format!("{}.json", digest_hex(digest)));
        let checked_path = checked_archive_path(archive_root, &path)?;
        let bytes = fs::read(&checked_path.canonical_path)?;
        if digest_bytes(&bytes) != digest {
            return Err(ArchiveError::Integrity(
                "creature manifest digest mismatch".to_string(),
            ));
        }
        let manifest = serde_json::from_slice::<CreatureArchiveManifest>(&bytes)?;
        manifest.validate_contract()?;
        Ok((bytes, manifest))
    }

    pub fn read_checkpoint(
        &self,
        reference: &ArchiveCheckpointRef,
    ) -> Result<Vec<u8>, ArchiveError> {
        reference.validate_contract()?;
        let archive_root = canonical_archive_root(&self.config.root)?;
        let root = archive_root
            .join("checkpoints")
            .join(digest_hex(reference.digest));
        let output_capacity =
            usize::try_from(reference.total_uncompressed_bytes).map_err(|_| {
                ArchiveError::Integrity("checkpoint byte count does not fit usize".to_string())
            })?;
        let mut output = Vec::with_capacity(output_capacity);
        for (index, page) in reference.pages.iter().enumerate() {
            let path = root.join(format!("{index:08}-{}.zst", digest_hex(page.digest)));
            let checked_path = checked_archive_path(&archive_root, &path)?;
            let compressed = fs::read(&checked_path.canonical_path)?;
            let compressed_bytes = usize::try_from(page.compressed_bytes).map_err(|_| {
                ArchiveError::Integrity("checkpoint page byte count does not fit usize".to_string())
            })?;
            let uncompressed_bytes = usize::try_from(page.uncompressed_bytes).map_err(|_| {
                ArchiveError::Integrity("checkpoint page byte count does not fit usize".to_string())
            })?;
            if compressed.len() != compressed_bytes || digest_bytes(&compressed) != page.digest {
                return Err(ArchiveError::Integrity(
                    "learned checkpoint page digest mismatch".to_string(),
                ));
            }
            let decoded = zstd::stream::decode_all(compressed.as_slice())?;
            if decoded.len() != uncompressed_bytes {
                return Err(ArchiveError::Integrity(
                    "learned checkpoint page length mismatch".to_string(),
                ));
            }
            output.extend_from_slice(&decoded);
        }
        if output.len() != output_capacity || digest_bytes(&output) != reference.digest {
            return Err(ArchiveError::Integrity(
                "learned checkpoint digest mismatch".to_string(),
            ));
        }
        Ok(output)
    }

    pub fn load_life_statistics(
        &self,
        manifest: &CreatureArchiveManifest,
    ) -> Result<PassiveLifeStatistics, ArchiveError> {
        manifest.validate_contract()?;
        let life = manifest.life.as_ref().ok_or_else(|| {
            ArchiveError::Integrity("birth manifest has no life statistics".to_string())
        })?;
        let reference = &life.statistics_asset;
        if reference.kind != ArchiveAssetKind::LifeStatistics {
            return Err(ArchiveError::Integrity(
                "life manifest references the wrong asset kind".to_string(),
            ));
        }
        let bytes = self.read_archive_asset(reference)?;
        let statistics = serde_json::from_slice::<PassiveLifeStatistics>(&bytes)?;
        statistics.validate_contract()?;
        if statistics.organism_id() != manifest.genetic.organism_id
            || statistics.death_tick() != Some(life.death_tick)
        {
            return Err(ArchiveError::Integrity(
                "life statistics identity does not match manifest".to_string(),
            ));
        }
        Ok(statistics)
    }

    /// Loads and validates the immutable genetic payload used by lineage
    /// comparison and genetic-founder restoration.
    pub fn load_brain_genome(
        &self,
        manifest: &CreatureArchiveManifest,
    ) -> Result<BrainGenome, ArchiveError> {
        manifest.validate_contract()?;
        let reference = &manifest.genetic.genome_asset;
        if reference.kind != ArchiveAssetKind::Genome {
            return Err(ArchiveError::Integrity(
                "genetic manifest references the wrong asset kind".to_string(),
            ));
        }
        let bytes = self.read_archive_asset(reference)?;
        let genome = serde_json::from_slice::<BrainGenome>(&bytes)?;
        genome.validate_contract()?;
        if genome.id != manifest.genetic.genome_id
            || genome.lineage_id != manifest.genetic.lineage_id
        {
            return Err(ArchiveError::Integrity(
                "genome asset identity does not match manifest".to_string(),
            ));
        }
        Ok(genome)
    }

    pub fn load_creature_genome(
        &self,
        manifest: &CreatureArchiveManifest,
    ) -> Result<CreatureGenome, ArchiveError> {
        manifest.validate_contract()?;
        let reference = manifest
            .genetic
            .composite_genome_asset
            .as_ref()
            .ok_or_else(|| {
                ArchiveError::Integrity(
                    "genetic manifest has no composite creature genome".to_string(),
                )
            })?;
        if reference.kind != ArchiveAssetKind::CompositeGenome {
            return Err(ArchiveError::Integrity(
                "composite genetic manifest references the wrong asset kind".to_string(),
            ));
        }
        let bytes = self.read_archive_asset(reference)?;
        let genome = serde_json::from_slice::<CreatureGenome>(&bytes)?;
        genome.validate_contract()?;
        if genome.id != manifest.genetic.genome_id
            || Some(genome.lineage_id) != manifest.genetic.lineage_id
            || genome.foundation.foundation_id
                != manifest
                    .genetic
                    .foundation_id
                    .map_or(0, alife_core::FoundationId::raw)
            || u32::from(genome.foundation.version)
                != manifest
                    .genetic
                    .foundation_version
                    .map_or(0, alife_core::FoundationVersion::raw)
            || genome.foundation.compatibility_family_id
                != manifest
                    .genetic
                    .compatibility_family_id
                    .map_or(0, alife_core::FoundationCompatibilityFamilyId::raw)
        {
            return Err(ArchiveError::Integrity(
                "composite genome identity does not match manifest".to_string(),
            ));
        }
        Ok(genome)
    }

    pub fn life_manifest_digests(&self) -> Result<Vec<Blake3Digest>, ArchiveError> {
        let mut statement = self.connection.prepare(
            "SELECT digest FROM manifests WHERE is_life=1 \
             ORDER BY source_run_id, organism_id, rowid",
        )?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.map(|row| parse_digest_hex(&row?)).collect()
    }

    /// Returns one current manifest per archived creature. A completed life
    /// record supersedes its birth record; living creatures remain available
    /// through their immutable genetic archive.
    pub fn latest_manifest_digests(&self) -> Result<Vec<Blake3Digest>, ArchiveError> {
        let mut statement = self.connection.prepare(
            "SELECT digest,source_run_id,organism_id,is_life,rowid FROM manifests \
             ORDER BY source_run_id,organism_id,is_life DESC,rowid DESC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        let mut seen = std::collections::BTreeSet::new();
        let mut digests = Vec::new();
        for row in rows {
            let (digest, source_run_id, organism_id) = row?;
            if seen.insert((source_run_id, organism_id)) {
                digests.push(parse_digest_hex(&digest)?);
            }
        }
        Ok(digests)
    }

    pub fn rebuild_index(&mut self) -> Result<(), ArchiveError> {
        let transaction = self.connection.transaction()?;
        transaction.execute("DELETE FROM checkpoints", [])?;
        transaction.execute("DELETE FROM manifests", [])?;
        let mut manifests = fs::read_dir(self.config.root.join("manifests"))?
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
            .collect::<Vec<_>>();
        manifests.sort_by_key(|entry| entry.file_name());
        for entry in manifests {
            let path = entry.path();
            let bytes = fs::read(&path)?;
            let digest = digest_bytes(&bytes);
            let expected = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .ok_or_else(|| ArchiveError::Integrity("invalid manifest file name".to_string()))?;
            if expected != digest_hex(digest) {
                return Err(ArchiveError::Integrity(
                    "manifest path does not match content digest".to_string(),
                ));
            }
            let manifest = serde_json::from_slice::<CreatureArchiveManifest>(&bytes)?;
            manifest.validate_contract()?;
            index_manifest_transaction(&transaction, digest, &manifest)?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn manifest_count(&self) -> Result<u64, ArchiveError> {
        Ok(self
            .connection
            .query_row("SELECT COUNT(*) FROM manifests", [], |row| row.get(0))?)
    }

    pub fn latest_manifest_for(
        &self,
        source_run_id: &str,
        organism_id: OrganismId,
    ) -> Result<Option<Blake3Digest>, ArchiveError> {
        validate_run_id(source_run_id)?;
        organism_id.validate()?;
        let mut statement = self.connection.prepare(
            "SELECT digest FROM manifests WHERE source_run_id=?1 AND organism_id=?2 \
             ORDER BY is_life DESC, rowid DESC LIMIT 1",
        )?;
        let mut rows = statement.query(params![source_run_id, organism_id.raw().to_string()])?;
        rows.next()?
            .map(|row| parse_digest_hex(&row.get::<_, String>(0)?))
            .transpose()
    }

    fn write_asset(
        &self,
        kind: ArchiveAssetKind,
        bytes: &[u8],
    ) -> Result<ArchiveAssetRef, ArchiveError> {
        if bytes.is_empty() {
            return Err(ArchiveError::Integrity(
                "archive assets cannot be empty".to_string(),
            ));
        }
        let digest = digest_bytes(bytes);
        let destination = self
            .config
            .root
            .join("assets")
            .join(digest_hex(digest))
            .join("payload.bin");
        write_content_addressed(&self.config.root.join("staging"), &destination, bytes)?;
        Ok(ArchiveAssetRef {
            kind,
            digest,
            size_bytes: bytes.len() as u64,
        })
    }

    fn write_manifest(
        &self,
        manifest: &CreatureArchiveManifest,
    ) -> Result<Blake3Digest, ArchiveError> {
        let bytes = serde_json::to_vec(manifest)?;
        let digest = digest_bytes(&bytes);
        let destination = self
            .config
            .root
            .join("manifests")
            .join(format!("{}.json", digest_hex(digest)));
        write_content_addressed(&self.config.root.join("staging"), &destination, &bytes)?;
        Ok(digest)
    }

    fn store_checkpoint(
        &self,
        source_run_id: &str,
        bytes: &[u8],
        retention: ArchiveCheckpointRetention,
    ) -> Result<ArchiveCheckpointDisposition, ArchiveError> {
        if bytes.is_empty() {
            return Err(ArchiveError::Integrity(
                "learned checkpoint cannot be empty".to_string(),
            ));
        }
        let count_limit = match retention {
            ArchiveCheckpointRetention::TemporaryPeak => Some(self.config.max_temporary_per_run),
            ArchiveCheckpointRetention::AutomaticPermanent => {
                Some(self.config.max_automatic_per_run)
            }
            ArchiveCheckpointRetention::Pinned => None,
        };
        if let Some(limit) = count_limit {
            let count: u32 = self.connection.query_row(
                "SELECT COUNT(*) FROM checkpoints WHERE source_run_id=?1 AND retention=?2",
                params![source_run_id, retention_slug(retention)],
                |row| row.get(0),
            )?;
            if count >= limit {
                return Ok(ArchiveCheckpointDisposition::DowngradedToGeneticOnly {
                    reason: format!("{} checkpoint limit reached", retention_slug(retention)),
                });
            }
        }

        let mut compressed_pages = Vec::new();
        let mut page_refs = Vec::new();
        for page in bytes.chunks(ARCHIVE_PAGE_BYTES) {
            let compressed = zstd::stream::encode_all(page, 3)?;
            let page_ref = ArchivePageRef {
                digest: digest_bytes(&compressed),
                compressed_bytes: compressed.len() as u32,
                uncompressed_bytes: page.len() as u32,
            };
            compressed_pages.push(compressed);
            page_refs.push(page_ref);
        }
        let total_compressed_bytes = page_refs
            .iter()
            .map(|page| u64::from(page.compressed_bytes))
            .sum::<u64>();
        if retention != ArchiveCheckpointRetention::Pinned {
            let used: u64 = self.connection.query_row(
                "SELECT COALESCE(SUM(compressed_bytes),0) FROM checkpoints",
                [],
                |row| row.get(0),
            )?;
            if used.saturating_add(total_compressed_bytes) > self.config.full_state_quota_bytes {
                return Ok(ArchiveCheckpointDisposition::DowngradedToGeneticOnly {
                    reason: "full-state quota reached".to_string(),
                });
            }
        }

        let digest = digest_bytes(bytes);
        let destination = self
            .config
            .root
            .join("checkpoints")
            .join(digest_hex(digest));
        if !destination.exists() {
            let staged = self.config.root.join("staging").join(format!(
                "checkpoint-{}-{}-{}",
                digest_hex(digest),
                std::process::id(),
                TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&staged)?;
            for (index, (reference, compressed)) in
                page_refs.iter().zip(&compressed_pages).enumerate()
            {
                let path = staged.join(format!("{index:08}-{}.zst", digest_hex(reference.digest)));
                fs::write(path, compressed)?;
            }
            match fs::rename(&staged, &destination) {
                Ok(()) => {}
                Err(_) if destination.exists() => fs::remove_dir_all(staged)?,
                Err(error) => {
                    let _ = fs::remove_dir_all(staged);
                    return Err(error.into());
                }
            }
        }
        let reference = ArchiveCheckpointRef {
            digest,
            retention,
            total_uncompressed_bytes: bytes.len() as u64,
            total_compressed_bytes,
            pages: page_refs,
        };
        reference.validate_contract()?;
        Ok(ArchiveCheckpointDisposition::Stored(reference))
    }

    fn index_manifest(
        &mut self,
        digest: Blake3Digest,
        manifest: &CreatureArchiveManifest,
    ) -> Result<(), ArchiveError> {
        let transaction = self.connection.transaction()?;
        index_manifest_transaction(&transaction, digest, manifest)?;
        transaction.commit()?;
        Ok(())
    }
}

fn validate_foundation_identity(
    input: &CompositeGeneticArchiveBatchInput<'_>,
    phenotype: &BrainPhenotype,
) -> Result<(), ArchiveError> {
    let abi = phenotype.foundation_abi();
    let matches = abi.capacity_class_id() == input.foundation.brain_class_id
        && abi
            .foundation_id()
            .is_some_and(|id| id.raw() == input.foundation.foundation_id)
        && abi
            .foundation_version()
            .is_some_and(|version| u32::from(version.raw()) == u32::from(input.foundation.version))
        && abi
            .compatibility_family_id()
            .is_some_and(|family| family.raw() == input.foundation.compatibility_family_id)
        && abi.foundation_payload_digest() == Some(input.foundation_content_digest);
    if !matches {
        return Err(ArchiveError::Integrity(
            "composite birth foundation identity does not match phenotype ABI".to_string(),
        ));
    }
    Ok(())
}

fn checked_byte_len(length: usize) -> Result<u64, ArchiveError> {
    u64::try_from(length)
        .map_err(|_| ArchiveError::Integrity("archive byte count does not fit u64".to_string()))
}

fn ensure_prepared_byte_capacity(current: u64, length: usize) -> Result<u64, ArchiveError> {
    let byte_count = checked_byte_len(length)?;
    let next = current.checked_add(byte_count).ok_or_else(|| {
        ArchiveError::Integrity("prepared composite birth aggregate byte overflow".to_string())
    })?;
    if next > MAX_COMPOSITE_BIRTH_BATCH_BYTES {
        return Err(ArchiveError::Integrity(format!(
            "prepared composite birth aggregate exceeds {} bytes",
            MAX_COMPOSITE_BIRTH_BATCH_BYTES
        )));
    }
    Ok(next)
}

fn ensure_prepared_payload_within_limit(length: usize) -> Result<(), ArchiveError> {
    let byte_count = checked_byte_len(length)?;
    if byte_count > MAX_COMPOSITE_BIRTH_BATCH_BYTES {
        return Err(ArchiveError::Integrity(format!(
            "prepared composite birth aggregate exceeds {} bytes",
            MAX_COMPOSITE_BIRTH_BATCH_BYTES
        )));
    }
    Ok(())
}

#[cfg(windows)]
fn archive_metadata_is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    archive_reparse_point_flags(
        metadata.file_type().is_symlink(),
        metadata.file_attributes(),
    )
}

#[cfg(not(windows))]
fn archive_metadata_is_reparse_point(metadata: &fs::Metadata) -> bool {
    archive_reparse_point_flags(metadata.file_type().is_symlink(), 0)
}

fn archive_reparse_point_flags(is_symlink: bool, file_attributes: u32) -> bool {
    is_symlink || file_attributes & 0x400 != 0
}

#[cfg(test)]
mod archive_path_tests {
    #[test]
    fn reparse_metadata_predicate_rejects_link_bits_without_link_creation() {
        assert!(super::archive_reparse_point_flags(false, 0x400));
        assert!(super::archive_reparse_point_flags(true, 0));
        assert!(!super::archive_reparse_point_flags(false, 0));
    }
}

fn canonical_archive_root(root: &Path) -> Result<PathBuf, ArchiveError> {
    let metadata = fs::symlink_metadata(root)?;
    if archive_metadata_is_reparse_point(&metadata) || !metadata.is_dir() {
        return Err(ArchiveError::Integrity(
            "archive root must be a real directory".to_string(),
        ));
    }
    let canonical = root.canonicalize()?;
    let canonical_metadata = fs::symlink_metadata(&canonical)?;
    if archive_metadata_is_reparse_point(&canonical_metadata) || !canonical_metadata.is_dir() {
        return Err(ArchiveError::Integrity(
            "canonical archive root must be a real directory".to_string(),
        ));
    }
    Ok(canonical)
}

#[derive(Debug)]
struct CheckedArchivePath {
    canonical_path: PathBuf,
    existed: bool,
}

fn checked_archive_path(
    archive_root: &Path,
    candidate: &Path,
) -> Result<CheckedArchivePath, ArchiveError> {
    if !candidate.starts_with(archive_root) {
        return Err(ArchiveError::Integrity(
            "archive path escapes the canonical archive root".to_string(),
        ));
    }
    let relative = candidate.strip_prefix(archive_root).map_err(|_| {
        ArchiveError::Integrity("archive path is not below the canonical root".to_string())
    })?;
    let components = relative.components().collect::<Vec<_>>();
    let mut current = archive_root.to_path_buf();
    if components.is_empty() {
        return Ok(CheckedArchivePath {
            canonical_path: archive_root.to_path_buf(),
            existed: true,
        });
    }

    for (index, component) in components.iter().enumerate() {
        if matches!(
            component,
            std::path::Component::CurDir
                | std::path::Component::ParentDir
                | std::path::Component::RootDir
                | std::path::Component::Prefix(_)
        ) {
            return Err(ArchiveError::Integrity(
                "archive path contains unsafe components".to_string(),
            ));
        }
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if archive_metadata_is_reparse_point(&metadata) {
                    return Err(ArchiveError::Integrity(format!(
                        "archive path contains a symbolic link or reparse point at {}",
                        current.display()
                    )));
                }
                let canonical = current.canonicalize()?;
                if !canonical.starts_with(archive_root) {
                    return Err(ArchiveError::Integrity(
                        "canonical archive path escapes the archive root".to_string(),
                    ));
                }
                if index + 1 < components.len() && !metadata.is_dir() {
                    return Err(ArchiveError::Integrity(
                        "archive path parent is not a directory".to_string(),
                    ));
                }
                if index + 1 == components.len() {
                    return Ok(CheckedArchivePath {
                        canonical_path: canonical,
                        existed: true,
                    });
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let parent = current.parent().ok_or_else(|| {
                    ArchiveError::Integrity("archive path has no parent".to_string())
                })?;
                let parent_canonical = parent.canonicalize()?;
                if !parent_canonical.starts_with(archive_root) {
                    return Err(ArchiveError::Integrity(
                        "canonical archive parent escapes the archive root".to_string(),
                    ));
                }
                let mut canonical_candidate =
                    parent_canonical.join(current.file_name().ok_or_else(|| {
                        ArchiveError::Integrity("archive path has no file name".to_string())
                    })?);
                for remaining in components.iter().skip(index + 1) {
                    if matches!(
                        remaining,
                        std::path::Component::CurDir
                            | std::path::Component::ParentDir
                            | std::path::Component::RootDir
                            | std::path::Component::Prefix(_)
                    ) {
                        return Err(ArchiveError::Integrity(
                            "archive path contains unsafe components".to_string(),
                        ));
                    }
                    canonical_candidate.push(remaining.as_os_str());
                }
                return Ok(CheckedArchivePath {
                    canonical_path: canonical_candidate,
                    existed: false,
                });
            }
            Err(error) => return Err(error.into()),
        }
    }

    Err(ArchiveError::Integrity(
        "archive path validation did not resolve".to_string(),
    ))
}

fn register_prepared_payload(
    payloads: &mut Vec<PreparedArchivePayload>,
    bytes: Vec<u8>,
    destination: PreparedPayloadDestination,
    aggregate_bytes: &mut u64,
) -> Result<Blake3Digest, ArchiveError> {
    let digest = digest_bytes(&bytes);
    if let Some(existing) = payloads.iter_mut().find(|payload| payload.digest == digest) {
        if existing.bytes != bytes {
            return Err(ArchiveError::Integrity(
                "same-digest/different-byte prepared CAS collision".to_string(),
            ));
        }
        if !existing.destinations.contains(&destination) {
            existing.destinations.push(destination);
        }
        return Ok(digest);
    }
    *aggregate_bytes = ensure_prepared_byte_capacity(*aggregate_bytes, bytes.len())?;
    payloads.push(PreparedArchivePayload {
        digest,
        bytes,
        destinations: vec![destination],
    });
    Ok(digest)
}

fn observe_prepared_final_file(
    archive_root: &Path,
    destination: PreparedPayloadDestination,
    digest: Blake3Digest,
    bytes: &[u8],
) -> Result<PreparedFinalFileObservation, ArchiveError> {
    let path = match destination {
        PreparedPayloadDestination::Asset(_) => archive_root
            .join("assets")
            .join(digest_hex(digest))
            .join("payload.bin"),
        PreparedPayloadDestination::Manifest => archive_root
            .join("manifests")
            .join(format!("{}.json", digest_hex(digest))),
    };
    let checked_path = checked_archive_path(archive_root, &path)?;
    if !checked_path.existed {
        return Ok(PreparedFinalFileObservation {
            destination,
            expected_digest: digest,
            canonical_path: checked_path.canonical_path,
            existed: false,
            observed_bytes: Vec::new(),
            observed_digest: None,
            size_bytes: 0,
        });
    }
    let existing = fs::read(&checked_path.canonical_path)?;
    let observed_digest = digest_bytes(&existing);
    if observed_digest != digest || existing != bytes {
        return Err(ArchiveError::Integrity(format!(
            "prepared final file has a content-addressed collision at {}",
            checked_path.canonical_path.display()
        )));
    }
    Ok(PreparedFinalFileObservation {
        destination,
        expected_digest: digest,
        canonical_path: checked_path.canonical_path,
        existed: true,
        observed_bytes: existing.clone(),
        observed_digest: Some(observed_digest),
        size_bytes: checked_byte_len(existing.len())?,
    })
}

fn open_index(path: &Path) -> Result<Connection, ArchiveError> {
    let connection = Connection::open(path)?;
    connection.execute_batch(
        "PRAGMA journal_mode=WAL;
         CREATE TABLE IF NOT EXISTS manifests(
           digest TEXT PRIMARY KEY,
           source_run_id TEXT NOT NULL,
           organism_id TEXT NOT NULL,
           genome_id TEXT NOT NULL,
           is_life INTEGER NOT NULL,
           death_tick TEXT
         );
         CREATE TABLE IF NOT EXISTS checkpoints(
           digest TEXT PRIMARY KEY,
           source_run_id TEXT NOT NULL,
           retention TEXT NOT NULL,
           compressed_bytes INTEGER NOT NULL,
           manifest_digest TEXT NOT NULL
         );",
    )?;
    Ok(connection)
}

fn index_manifest_transaction(
    transaction: &rusqlite::Transaction<'_>,
    digest: Blake3Digest,
    manifest: &CreatureArchiveManifest,
) -> Result<(), ArchiveError> {
    transaction.execute(
        "INSERT OR REPLACE INTO manifests(digest,source_run_id,organism_id,genome_id,is_life,death_tick) \
         VALUES(?1,?2,?3,?4,?5,?6)",
        params![
            digest_hex(digest),
            manifest.genetic.source_run_id,
            manifest.genetic.organism_id.raw().to_string(),
            manifest.genetic.genome_id.raw().to_string(),
            i64::from(manifest.life.is_some()),
            manifest
                .life
                .as_ref()
                .map(|life| life.death_tick.raw().to_string()),
        ],
    )?;
    if let Some(CreatureLifeArchiveRecord {
        checkpoint: ArchiveCheckpointDisposition::Stored(reference),
        ..
    }) = &manifest.life
    {
        transaction.execute(
            "INSERT OR REPLACE INTO checkpoints(digest,source_run_id,retention,compressed_bytes,manifest_digest) \
             VALUES(?1,?2,?3,?4,?5)",
            params![
                digest_hex(reference.digest),
                manifest.genetic.source_run_id,
                retention_slug(reference.retention),
                reference.total_compressed_bytes,
                digest_hex(digest),
            ],
        )?;
    }
    Ok(())
}

fn write_content_addressed(
    staging_root: &Path,
    destination: &Path,
    bytes: &[u8],
) -> Result<(), ArchiveError> {
    if destination.exists() {
        if fs::read(destination)? == bytes {
            return Ok(());
        }
        return Err(ArchiveError::Integrity(format!(
            "content-addressed collision at {}",
            destination.display()
        )));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| ArchiveError::Integrity("archive destination has no parent".to_string()))?;
    fs::create_dir_all(parent)?;
    let staged = staging_root.join(format!(
        "asset-{}-{}",
        std::process::id(),
        TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&staged)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    match fs::rename(&staged, destination) {
        Ok(()) => Ok(()),
        Err(_) if destination.exists() && fs::read(destination)? == bytes => {
            fs::remove_file(staged)?;
            Ok(())
        }
        Err(error) => {
            let _ = fs::remove_file(staged);
            Err(error.into())
        }
    }
}

fn validate_run_id(run_id: &str) -> Result<(), ArchiveError> {
    if run_id.trim().is_empty()
        || run_id.chars().count() > 96
        || !run_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(ArchiveError::Integrity(
            "source run id is not a bounded portable identifier".to_string(),
        ));
    }
    Ok(())
}

fn retention_slug(retention: ArchiveCheckpointRetention) -> &'static str {
    match retention {
        ArchiveCheckpointRetention::TemporaryPeak => "temporary-peak",
        ArchiveCheckpointRetention::AutomaticPermanent => "automatic-permanent",
        ArchiveCheckpointRetention::Pinned => "pinned",
    }
}

fn digest_bytes(bytes: &[u8]) -> Blake3Digest {
    Blake3Digest::from_bytes(*blake3::hash(bytes).as_bytes())
}

fn digest_hex(digest: Blake3Digest) -> String {
    digest
        .bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn parse_digest_hex(value: &str) -> Result<Blake3Digest, ArchiveError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ArchiveError::Integrity(
            "invalid BLAKE3-256 archive digest".to_string(),
        ));
    }
    let mut bytes = [0_u8; 32];
    for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
        let text = std::str::from_utf8(chunk)
            .map_err(|_| ArchiveError::Integrity("invalid digest UTF-8".to_string()))?;
        bytes[index] = u8::from_str_radix(text, 16)
            .map_err(|_| ArchiveError::Integrity("invalid digest hex".to_string()))?;
    }
    Ok(Blake3Digest::from_bytes(bytes))
}
