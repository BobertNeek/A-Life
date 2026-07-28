//! Profile-local immutable creature archives with a rebuildable SQLite index.

mod bundle;

pub use bundle::{
    BundleImportReceipt, FounderBundleKind, ResolvedFounder, ResolvedFounderCohort,
    ResolvedGpuFounderCheckpoint, MAX_BUNDLE_UNCOMPRESSED_BYTES, MAX_COHORT_FOUNDERS,
};

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use alife_core::{
    ArchiveAssetKind, ArchiveAssetRef, ArchiveCheckpointDisposition, ArchiveCheckpointRef,
    ArchiveCheckpointRetention, ArchivePageRef, ArchiveRetirementReceipt, Blake3Digest,
    BrainGenome, BrainPhenotype, CreatureArchiveManifest, CreatureLifeArchiveRecord,
    ExperienceSequenceId, GeneticArchiveRecord, OrganismId, PassiveLifeStatistics,
    ScaffoldContractError, Tick, Validate, CREATURE_ARCHIVE_SCHEMA_VERSION,
};
use rusqlite::{params, Connection};

pub const ARCHIVE_PAGE_BYTES: usize = 65_536;
pub const DEFAULT_FULL_STATE_QUOTA_BYTES: u64 = 4 * 1024 * 1024 * 1024;
pub const DEFAULT_MAX_TEMPORARY_PER_RUN: u32 = 64;
pub const DEFAULT_MAX_AUTOMATIC_PER_RUN: u32 = 24;

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
        validate_run_id(input.source_run_id)?;
        input.organism_id.validate()?;
        input.genome.validate_contract()?;
        let genome_bytes = serde_json::to_vec(input.genome)?;
        let genome_asset = self.write_asset(ArchiveAssetKind::Genome, &genome_bytes)?;
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
        let path = self
            .config
            .root
            .join("manifests")
            .join(format!("{}.json", digest_hex(digest)));
        let bytes = fs::read(path)?;
        if digest_bytes(&bytes) != digest {
            return Err(ArchiveError::Integrity(
                "creature manifest digest mismatch".to_string(),
            ));
        }
        let manifest = serde_json::from_slice::<CreatureArchiveManifest>(&bytes)?;
        manifest.validate_contract()?;
        Ok(manifest)
    }

    pub fn read_checkpoint(
        &self,
        reference: &ArchiveCheckpointRef,
    ) -> Result<Vec<u8>, ArchiveError> {
        reference.validate_contract()?;
        let root = self
            .config
            .root
            .join("checkpoints")
            .join(digest_hex(reference.digest));
        let mut output = Vec::with_capacity(reference.total_uncompressed_bytes as usize);
        for (index, page) in reference.pages.iter().enumerate() {
            let path = root.join(format!("{index:08}-{}.zst", digest_hex(page.digest)));
            let compressed = fs::read(path)?;
            if compressed.len() != page.compressed_bytes as usize
                || digest_bytes(&compressed) != page.digest
            {
                return Err(ArchiveError::Integrity(
                    "learned checkpoint page digest mismatch".to_string(),
                ));
            }
            let decoded = zstd::stream::decode_all(compressed.as_slice())?;
            if decoded.len() != page.uncompressed_bytes as usize {
                return Err(ArchiveError::Integrity(
                    "learned checkpoint page length mismatch".to_string(),
                ));
            }
            output.extend_from_slice(&decoded);
        }
        if output.len() != reference.total_uncompressed_bytes as usize
            || digest_bytes(&output) != reference.digest
        {
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
        let path = self
            .config
            .root
            .join("assets")
            .join(digest_hex(reference.digest))
            .join("payload.bin");
        let bytes = fs::read(path)?;
        if bytes.len() as u64 != reference.size_bytes || digest_bytes(&bytes) != reference.digest {
            return Err(ArchiveError::Integrity(
                "life statistics asset digest mismatch".to_string(),
            ));
        }
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
