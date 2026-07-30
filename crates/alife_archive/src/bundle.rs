use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Cursor, Read},
    path::{Component, Path},
};

use alife_core::{
    ArchiveCheckpointDisposition, Blake3Digest, BrainCapacityClass, BrainGenome, BrainScaleTier,
    CreatureArchiveManifest, FoundationWeightAsset, FounderCohortManifest, FounderIdentityRemap,
    FounderMode, FounderProvenance, FounderSelection, GenomeId, HomeostaticSnapshot,
    LanguageCodebookV1, LineageId, OrganismId, Tick, Validate, Vec3f,
    FOUNDER_COHORT_SCHEMA_VERSION,
};
use alife_world::persistence::{
    AssetKind, AssetManifest, AssetManifestEntry, AssetPresence, CreatureMindSaveSummary,
    CreatureSaveState, GpuBrainSaveState, LearningTraceSaveSummary, PortableAssetDigest,
    PortableSaveFile, WeightLayerSaveSummary, P34_ASSET_MANIFEST_SCHEMA,
    P34_ASSET_MANIFEST_SCHEMA_VERSION,
};
use alife_world::{CreatureAppearanceGenome, WorldObjectKind};
use serde::{Deserialize, Serialize};

use super::{
    digest_bytes, digest_hex, write_content_addressed, ArchiveError, LineageLibrary, TEMP_SEQUENCE,
};
use std::sync::atomic::Ordering;

const BUNDLE_MAGIC: &[u8; 8] = b"ALIFEBND";
const BUNDLE_VERSION: u16 = 1;
const DESCRIPTOR_PATH: &str = "bundle/descriptor.json";
const MAX_ENTRY_COUNT: usize = 65_536;
const MAX_ENTRY_BYTES: usize = 256 * 1024 * 1024;
const MAX_PATH_BYTES: usize = 512;
pub const MAX_BUNDLE_UNCOMPRESSED_BYTES: usize = 512 * 1024 * 1024;
pub const MAX_COHORT_FOUNDERS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FounderBundleKind {
    Creature,
    Cohort,
}

impl FounderBundleKind {
    fn byte(self) -> u8 {
        match self {
            Self::Creature => 1,
            Self::Cohort => 2,
        }
    }

    fn from_byte(value: u8) -> Result<Self, ArchiveError> {
        match value {
            1 => Ok(Self::Creature),
            2 => Ok(Self::Cohort),
            _ => Err(ArchiveError::Integrity(
                "unsupported bundle kind".to_string(),
            )),
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Self::Creature => "alife-creature",
            Self::Cohort => "alife-cohort",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct BundleDescriptor {
    schema_version: u16,
    kind: FounderBundleKind,
    manifest_digests: Vec<Blake3Digest>,
}

#[derive(Debug, Deserialize)]
struct ArchivedGpuCheckpointEnvelope {
    save_state: GpuBrainSaveState,
    manifest_entries: Vec<AssetManifestEntry>,
    checkpoint_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BundleEntry {
    path: String,
    digest: Blake3Digest,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleImportReceipt {
    pub kind: FounderBundleKind,
    pub manifest_digests: Vec<Blake3Digest>,
    pub imported_entry_count: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedFounder {
    pub selection: FounderSelection,
    pub manifest: CreatureArchiveManifest,
    pub genome: BrainGenome,
    pub foundation_bytes: Option<Vec<u8>>,
    pub learned_checkpoint_bytes: Option<Vec<u8>>,
    pub gpu_checkpoint: Option<ResolvedGpuFounderCheckpoint>,
    pub provenance: FounderProvenance,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedGpuFounderCheckpoint {
    pub save_state: GpuBrainSaveState,
    pub manifest_entries: Vec<AssetManifestEntry>,
    pub checkpoint_digest: [u64; 4],
    pub assets: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedFounderCohort {
    pub manifest: FounderCohortManifest,
    pub founders: Vec<ResolvedFounder>,
}

impl LineageLibrary {
    pub fn export_creature_bundle(
        &self,
        manifest_digest: Blake3Digest,
        destination: impl AsRef<Path>,
    ) -> Result<(), ArchiveError> {
        self.export_bundle(
            FounderBundleKind::Creature,
            &[manifest_digest],
            destination.as_ref(),
        )
    }

    pub fn export_cohort_bundle(
        &self,
        manifest_digests: &[Blake3Digest],
        destination: impl AsRef<Path>,
    ) -> Result<(), ArchiveError> {
        self.export_bundle(
            FounderBundleKind::Cohort,
            manifest_digests,
            destination.as_ref(),
        )
    }

    fn export_bundle(
        &self,
        kind: FounderBundleKind,
        manifest_digests: &[Blake3Digest],
        destination: &Path,
    ) -> Result<(), ArchiveError> {
        validate_bundle_destination(destination, kind)?;
        if manifest_digests.is_empty()
            || manifest_digests.len() > MAX_COHORT_FOUNDERS
            || (kind == FounderBundleKind::Creature && manifest_digests.len() != 1)
        {
            return Err(ArchiveError::Integrity(
                "bundle has an invalid founder count".to_string(),
            ));
        }
        let mut selected = manifest_digests.to_vec();
        selected.sort();
        selected.dedup();
        if selected.len() != manifest_digests.len() {
            return Err(ArchiveError::Integrity(
                "bundle contains duplicate founder manifests".to_string(),
            ));
        }

        let descriptor = BundleDescriptor {
            schema_version: BUNDLE_VERSION,
            kind,
            manifest_digests: selected,
        };
        let descriptor_bytes = serde_json::to_vec(&descriptor)?;
        let mut entries = BTreeMap::<String, Vec<u8>>::new();
        entries.insert(DESCRIPTOR_PATH.to_string(), descriptor_bytes);
        for digest in &descriptor.manifest_digests {
            self.collect_manifest_entries(*digest, &mut entries)?;
        }
        let entries = entries
            .into_iter()
            .map(|(path, bytes)| BundleEntry {
                path,
                digest: digest_bytes(&bytes),
                bytes,
            })
            .collect::<Vec<_>>();
        let encoded = encode_bundle(kind, &entries)?;
        let compressed = zstd::stream::encode_all(encoded.as_slice(), 3)?;
        let staging = self.config.root.join("staging").join(format!(
            "bundle-{}-{}-{}.tmp",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed),
            kind.byte()
        ));
        fs::write(&staging, compressed)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        match fs::rename(&staging, destination) {
            Ok(()) => Ok(()),
            Err(error) => {
                let _ = fs::remove_file(staging);
                Err(error.into())
            }
        }
    }

    fn collect_manifest_entries(
        &self,
        digest: Blake3Digest,
        entries: &mut BTreeMap<String, Vec<u8>>,
    ) -> Result<(), ArchiveError> {
        let manifest_path = format!("manifests/{}.json", digest_hex(digest));
        let manifest_bytes = fs::read(self.config.root.join(&manifest_path))?;
        if digest_bytes(&manifest_bytes) != digest {
            return Err(ArchiveError::Integrity(
                "creature manifest digest mismatch".to_string(),
            ));
        }
        let manifest = serde_json::from_slice::<CreatureArchiveManifest>(&manifest_bytes)?;
        manifest.validate_contract()?;
        entries.insert(manifest_path, manifest_bytes);
        if let Some(previous) = manifest.previous_manifest_digest {
            self.collect_manifest_entries(previous, entries)?;
        }
        let mut assets = vec![&manifest.genetic.genome_asset];
        if let Some(foundation) = &manifest.genetic.foundation_asset {
            assets.push(foundation);
        }
        if let Some(life) = &manifest.life {
            assets.push(&life.statistics_asset);
            if let ArchiveCheckpointDisposition::Stored(checkpoint) = &life.checkpoint {
                let checkpoint_root = format!("checkpoints/{}", digest_hex(checkpoint.digest));
                for (index, page) in checkpoint.pages.iter().enumerate() {
                    let path = format!(
                        "{checkpoint_root}/{index:08}-{}.zst",
                        digest_hex(page.digest)
                    );
                    let bytes = fs::read(self.config.root.join(&path))?;
                    if bytes.len() != page.compressed_bytes as usize
                        || digest_bytes(&bytes) != page.digest
                    {
                        return Err(ArchiveError::Integrity(
                            "learned checkpoint page digest mismatch".to_string(),
                        ));
                    }
                    entries.insert(path, bytes);
                }
                let checkpoint_bytes = self.read_checkpoint(checkpoint)?;
                if let Ok(envelope) =
                    serde_json::from_slice::<ArchivedGpuCheckpointEnvelope>(&checkpoint_bytes)
                {
                    validate_checkpoint_envelope(&envelope)?;
                    for entry in envelope.manifest_entries {
                        validate_entry_path(&entry.relative_path)?;
                        let bytes = fs::read(self.config.root.join(&entry.relative_path))?;
                        if entry
                            .size_bytes
                            .is_some_and(|size| size != bytes.len() as u64)
                            || PortableAssetDigest::for_bytes(&bytes) != entry.digest
                        {
                            return Err(ArchiveError::Integrity(
                                "GPU checkpoint asset digest mismatch".to_string(),
                            ));
                        }
                        entries.insert(entry.relative_path, bytes);
                    }
                }
            }
        }
        for asset in assets {
            let path = format!("assets/{}/payload.bin", digest_hex(asset.digest));
            let bytes = fs::read(self.config.root.join(&path))?;
            if bytes.len() as u64 != asset.size_bytes || digest_bytes(&bytes) != asset.digest {
                return Err(ArchiveError::Integrity(
                    "archive asset digest mismatch".to_string(),
                ));
            }
            entries.insert(path, bytes);
        }
        Ok(())
    }

    pub fn import_bundle(
        &mut self,
        source: impl AsRef<Path>,
    ) -> Result<BundleImportReceipt, ArchiveError> {
        let source = source.as_ref();
        let compressed = fs::read(source)?;
        let mut decoder = zstd::stream::read::Decoder::new(compressed.as_slice())?;
        let mut encoded = Vec::new();
        decoder
            .by_ref()
            .take((MAX_BUNDLE_UNCOMPRESSED_BYTES + 1) as u64)
            .read_to_end(&mut encoded)?;
        if encoded.len() > MAX_BUNDLE_UNCOMPRESSED_BYTES {
            return Err(ArchiveError::Integrity(
                "bundle exceeds size limit".to_string(),
            ));
        }
        let (kind, entries) = decode_bundle(&encoded)?;
        validate_bundle_destination(source, kind)?;
        let descriptor_entry = entries
            .iter()
            .find(|entry| entry.path == DESCRIPTOR_PATH)
            .ok_or_else(|| ArchiveError::Integrity("bundle descriptor is missing".to_string()))?;
        let descriptor = serde_json::from_slice::<BundleDescriptor>(&descriptor_entry.bytes)?;
        if descriptor.schema_version != BUNDLE_VERSION || descriptor.kind != kind {
            return Err(ArchiveError::Integrity(
                "bundle descriptor is incompatible".to_string(),
            ));
        }
        validate_bundle_graph(&entries, &descriptor)?;

        let staged = self.config.root.join("staging").join(format!(
            "import-{}-{}",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&staged)?;
        let publish = (|| -> Result<(), ArchiveError> {
            for entry in entries.iter().filter(|entry| entry.path != DESCRIPTOR_PATH) {
                let staged_path = staged.join(&entry.path);
                if let Some(parent) = staged_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&staged_path, &entry.bytes)?;
            }
            for entry in entries.iter().filter(|entry| {
                entry.path.starts_with("assets/") || entry.path.starts_with("checkpoints/")
            }) {
                write_content_addressed(
                    &self.config.root.join("staging"),
                    &self.config.root.join(&entry.path),
                    &entry.bytes,
                )?;
            }
            for entry in entries
                .iter()
                .filter(|entry| entry.path.starts_with("manifests/"))
            {
                write_content_addressed(
                    &self.config.root.join("staging"),
                    &self.config.root.join(&entry.path),
                    &entry.bytes,
                )?;
            }
            Ok(())
        })();
        let _ = fs::remove_dir_all(&staged);
        publish?;
        self.rebuild_index()?;
        Ok(BundleImportReceipt {
            kind,
            manifest_digests: descriptor.manifest_digests,
            imported_entry_count: u32::try_from(entries.len())
                .map_err(|_| ArchiveError::Integrity("too many bundle entries".to_string()))?,
        })
    }

    pub fn resolve_founder_cohort(
        &self,
        target_save_id: impl Into<String>,
        deterministic_seed: u64,
        selections: &[FounderSelection],
    ) -> Result<ResolvedFounderCohort, ArchiveError> {
        if deterministic_seed == 0
            || selections.is_empty()
            || selections.len() > MAX_COHORT_FOUNDERS
        {
            return Err(ArchiveError::Integrity(
                "invalid founder cohort".to_string(),
            ));
        }
        let target_save_id = target_save_id.into();
        let mut founders = Vec::with_capacity(selections.len());
        let mut provenances = Vec::with_capacity(selections.len());
        for (index, selection) in selections.iter().enumerate() {
            selection.validate_contract()?;
            let manifest = self.load_manifest(selection.source_manifest_digest)?;
            validate_supported_manifest(&manifest)?;
            let genome_bytes = read_archive_asset(self, &manifest.genetic.genome_asset)?;
            let archived_genome = serde_json::from_slice::<BrainGenome>(&genome_bytes)?;
            archived_genome.validate_contract()?;
            if archived_genome.id != manifest.genetic.genome_id
                || archived_genome.brain_class_id != manifest.genetic.brain_class_id
            {
                return Err(ArchiveError::Integrity(
                    "genome identity does not match archive manifest".to_string(),
                ));
            }
            let genome = match selection.mode {
                FounderMode::GeneticOffspring { mutation_seed } => {
                    deterministic_offspring_genome(&archived_genome, mutation_seed)?
                }
                FounderMode::GeneticFounder | FounderMode::MindStateClone { .. } => archived_genome,
            };
            let foundation_bytes = manifest
                .genetic
                .foundation_asset
                .as_ref()
                .map(|asset| read_archive_asset(self, asset))
                .transpose()?;
            if let Some(bytes) = &foundation_bytes {
                let foundation = FoundationWeightAsset::decode_canonical(bytes)?;
                if Some(foundation.digest()) != manifest.genetic.foundation_payload_digest {
                    return Err(ArchiveError::Integrity(
                        "foundation digest does not match archive manifest".to_string(),
                    ));
                }
            }
            let learned_checkpoint_bytes = match selection.mode {
                FounderMode::MindStateClone { checkpoint_digest } => {
                    let checkpoint = manifest
                        .life
                        .as_ref()
                        .and_then(|life| match &life.checkpoint {
                            ArchiveCheckpointDisposition::Stored(reference) => Some(reference),
                            _ => None,
                        })
                        .ok_or_else(|| {
                            ArchiveError::Integrity(
                                "mind-state clone requires a stored learned checkpoint".to_string(),
                            )
                        })?;
                    if checkpoint.digest != checkpoint_digest {
                        return Err(ArchiveError::Integrity(
                            "selected checkpoint does not match archive manifest".to_string(),
                        ));
                    }
                    Some(self.read_checkpoint(checkpoint)?)
                }
                _ => None,
            };
            let gpu_checkpoint = match learned_checkpoint_bytes.as_deref() {
                Some(bytes) => {
                    let envelope = serde_json::from_slice::<ArchivedGpuCheckpointEnvelope>(bytes)
                        .map_err(|_| {
                        ArchiveError::Integrity(
                            "mind-state clone requires a GPU checkpoint envelope".to_string(),
                        )
                    })?;
                    validate_checkpoint_envelope(&envelope)?;
                    if envelope.save_state.organism_id != manifest.genetic.organism_id {
                        return Err(ArchiveError::Integrity(
                            "GPU checkpoint identity does not match archive manifest".to_string(),
                        ));
                    }
                    let assets = envelope
                        .manifest_entries
                        .iter()
                        .map(|entry| {
                            validate_entry_path(&entry.relative_path)?;
                            let bytes = fs::read(self.config.root.join(&entry.relative_path))?;
                            if entry
                                .size_bytes
                                .is_some_and(|size| size != bytes.len() as u64)
                                || PortableAssetDigest::for_bytes(&bytes) != entry.digest
                            {
                                return Err(ArchiveError::Integrity(
                                    "GPU checkpoint asset digest mismatch".to_string(),
                                ));
                            }
                            Ok(bytes)
                        })
                        .collect::<Result<Vec<_>, ArchiveError>>()?;
                    Some(ResolvedGpuFounderCheckpoint {
                        save_state: envelope.save_state,
                        manifest_entries: envelope.manifest_entries,
                        checkpoint_digest: envelope.checkpoint_digest,
                        assets,
                    })
                }
                None => None,
            };
            let remap =
                deterministic_remap(deterministic_seed, index as u64, &manifest, &selection.mode)?;
            let provenance = FounderProvenance {
                source_run_id: manifest.genetic.source_run_id.clone(),
                source_manifest_digest: selection.source_manifest_digest,
                source_checkpoint_digest: learned_checkpoint_bytes
                    .as_ref()
                    .map(|bytes| digest_bytes(bytes)),
                mode: selection.mode,
                remap,
            };
            provenance.validate_contract()?;
            provenances.push(provenance.clone());
            founders.push(ResolvedFounder {
                selection: selection.clone(),
                manifest,
                genome,
                foundation_bytes,
                learned_checkpoint_bytes,
                gpu_checkpoint,
                provenance,
            });
        }
        let cohort_manifest = FounderCohortManifest {
            schema_version: FOUNDER_COHORT_SCHEMA_VERSION,
            target_save_id,
            deterministic_seed,
            founders: provenances,
        };
        cohort_manifest.validate_contract()?;
        Ok(ResolvedFounderCohort {
            manifest: cohort_manifest,
            founders,
        })
    }

    /// Builds a validated new-world save from resolved archives. Genetic
    /// founders are ready for ordinary GPU birth. Mind-clone source assets are
    /// copied into the save root for the subsequent GPU transplant.
    pub fn create_new_save_from_founders(
        &self,
        mut base_save: PortableSaveFile,
        asset_root: impl AsRef<Path>,
        cohort: &ResolvedFounderCohort,
    ) -> Result<PortableSaveFile, ArchiveError> {
        cohort.manifest.validate_contract()?;
        let asset_root = asset_root.as_ref();
        if base_save.save_id != cohort.manifest.target_save_id
            || base_save.deterministic_seed != cohort.manifest.deterministic_seed
            || !base_save.creatures.is_empty()
        {
            return Err(ArchiveError::Integrity(
                "founder save requires a matching empty base world".to_string(),
            ));
        }
        let mut world = base_save.restore_headless_world().map_err(|error| {
            ArchiveError::Integrity(format!("invalid founder base world: {error}"))
        })?;
        if world
            .object_snapshots()
            .iter()
            .any(|object| object.kind == WorldObjectKind::Agent)
        {
            return Err(ArchiveError::Integrity(
                "founder base world already contains creatures".to_string(),
            ));
        }
        let staging = asset_root.join(".founder-staging");
        fs::create_dir_all(&staging)?;
        let cohort_bytes = serde_json::to_vec_pretty(&cohort.manifest)?;
        add_save_asset(
            &mut base_save.assets,
            asset_root,
            &staging,
            "founder.cohort",
            "founders/cohort.json",
            &cohort_bytes,
            "cross-save founder provenance",
        )?;

        let founder_tier = cohort
            .founders
            .first()
            .ok_or_else(|| ArchiveError::Integrity("founder cohort is empty".to_string()))
            .and_then(|founder| tier_for_class(founder.manifest.genetic.brain_class_id))?;
        if cohort.founders.iter().any(|founder| {
            tier_for_class(founder.manifest.genetic.brain_class_id).ok() != Some(founder_tier)
        }) {
            return Err(ArchiveError::Integrity(
                "one save cannot mix founder brain classes".to_string(),
            ));
        }
        base_save.config.brain_class = founder_tier;

        for (index, founder) in cohort.founders.iter().enumerate() {
            if cohort.manifest.founders.get(index) != Some(&founder.provenance) {
                return Err(ArchiveError::Integrity(
                    "resolved founder order differs from cohort provenance".to_string(),
                ));
            }
            let target = &founder.provenance.remap;
            world.spawn_social_agent(
                &format!("founder-{:016x}", target.target_name_id),
                target.target_organism_id,
                safe_founder_position(&world, index)?,
                0.0,
            )?;

            founder.genome.validate_contract()?;
            let genome_bytes = serde_json::to_vec(&founder.genome)?;
            add_save_asset(
                &mut base_save.assets,
                asset_root,
                &staging,
                &format!("founder.{index}.genome"),
                &format!("founders/{index:04}/genome.json"),
                &genome_bytes,
                "immutable founder genetic content",
            )?;
            if let Some(foundation) = &founder.foundation_bytes {
                add_save_asset(
                    &mut base_save.assets,
                    asset_root,
                    &staging,
                    &format!("founder.{index}.foundation"),
                    &format!("founders/{index:04}/foundation.alife-foundation"),
                    foundation,
                    "founder foundation",
                )?;
            }
            if let Some(checkpoint) = &founder.gpu_checkpoint {
                for (entry, bytes) in checkpoint.manifest_entries.iter().zip(&checkpoint.assets) {
                    copy_manifest_asset(&mut base_save.assets, asset_root, &staging, entry, bytes)?;
                }
            }

            let tick = base_save.world.tick;
            base_save.creatures.push(CreatureSaveState {
                organism_id: target.target_organism_id,
                genome_id: target.target_genome_id,
                brain_class: tier_for_class(founder.manifest.genetic.brain_class_id)?,
                development_tick: Tick::ZERO,
                appearance: CreatureAppearanceGenome::founder_for_species(
                    (index % 12) as u8,
                    cohort.manifest.deterministic_seed ^ target.target_organism_id.raw(),
                ),
                mind: CreatureMindSaveSummary {
                    tick,
                    homeostasis: HomeostaticSnapshot::baseline(tick),
                    memory_record_count: 0,
                    memory_source_ids: Vec::new(),
                    concept_count: 0,
                    edge_count: 0,
                    simplex_count: 0,
                    unresolved_gap_count: 0,
                    sleep_state_label: "awake".to_string(),
                    diagnostics: vec![format!(
                        "founder:{}",
                        match founder.selection.mode {
                            FounderMode::GeneticFounder => "genetic",
                            FounderMode::MindStateClone { .. } => "mind-clone",
                            FounderMode::GeneticOffspring { .. } => "genetic-offspring",
                        }
                    )],
                },
                weights: WeightLayerSaveSummary {
                    generated_weight_asset_id: None,
                    genetic_fixed_digest: PortableAssetDigest::for_bytes(&genome_bytes).0,
                    genetic_layer_mutable: false,
                    lifetime_consolidated_entries: 0,
                    h_operational_entries: 0,
                    h_shadow_entries: 0,
                },
                learning: LearningTraceSaveSummary {
                    lifetime_learning_enabled: true,
                    lamarckian_mode_enabled: false,
                    last_consolidated_tick: None,
                },
                composite_genetics: None,
                lifetime_state_asset: None,
                gpu_brain: None,
            });
        }
        base_save
            .replace_headless_world_snapshot(&world)
            .map_err(|error| ArchiveError::Integrity(format!("invalid founder world: {error}")))?;
        base_save.gpu_runtime = None;
        base_save.generated_weight_asset_refs.clear();
        base_save.adapter_remap.entries.clear();
        base_save
            .validate_with_asset_root(asset_root)
            .map_err(|error| ArchiveError::Integrity(format!("invalid founder save: {error}")))?;
        let _ = fs::remove_dir_all(staging);
        Ok(base_save)
    }
}

fn tier_for_class(class_id: alife_core::BrainClassId) -> Result<BrainScaleTier, ArchiveError> {
    match class_id {
        BrainCapacityClass::N512_ID => Ok(BrainScaleTier::Nano512),
        BrainCapacityClass::N1024_ID => Ok(BrainScaleTier::Small1024),
        BrainCapacityClass::N2048_ID => Ok(BrainScaleTier::Standard2048),
        _ => Err(ArchiveError::Integrity(
            "unsupported founder brain class".to_string(),
        )),
    }
}

fn safe_founder_position(
    world: &alife_world::HeadlessWorld,
    founder_index: usize,
) -> Result<Vec3f, ArchiveError> {
    let objects = world.object_snapshots();
    for offset in 0..4096_usize {
        let slot = founder_index.saturating_add(offset);
        let candidate = Vec3f::new((slot % 32) as f32 * 3.0, 0.0, (slot / 32) as f32 * 3.0);
        let safe = objects.iter().all(|object| {
            if object.consumed {
                return true;
            }
            let dx = object.position.x - candidate.x;
            let dy = object.position.y - candidate.y;
            let dz = object.position.z - candidate.z;
            let distance_sq = dx * dx + dy * dy + dz * dz;
            let clearance = match object.kind {
                WorldObjectKind::Hazard => 4.0,
                WorldObjectKind::Obstacle => object.radius.max(1.0) + 1.0,
                WorldObjectKind::Agent => 2.0,
                _ => 1.0,
            };
            distance_sq > clearance * clearance
        });
        if safe {
            return Ok(candidate);
        }
    }
    Err(ArchiveError::Integrity(
        "no safe founder position is available".to_string(),
    ))
}

fn add_save_asset(
    manifest: &mut AssetManifest,
    root: &Path,
    staging: &Path,
    asset_id: &str,
    relative_path: &str,
    bytes: &[u8],
    provenance: &str,
) -> Result<(), ArchiveError> {
    validate_entry_path(relative_path)?;
    write_content_addressed(staging, &root.join(relative_path), bytes)?;
    merge_save_asset_entry(
        manifest,
        AssetManifestEntry {
            asset_id: asset_id.to_string(),
            kind: AssetKind::Other,
            relative_path: relative_path.to_string(),
            digest: PortableAssetDigest::for_bytes(bytes),
            presence: AssetPresence::Required,
            schema_version: 1,
            size_bytes: Some(bytes.len() as u64),
            provenance: Some(provenance.to_string()),
        },
    )
}

fn copy_manifest_asset(
    manifest: &mut AssetManifest,
    root: &Path,
    staging: &Path,
    entry: &AssetManifestEntry,
    bytes: &[u8],
) -> Result<(), ArchiveError> {
    validate_entry_path(&entry.relative_path)?;
    if entry
        .size_bytes
        .is_some_and(|size| size != bytes.len() as u64)
        || PortableAssetDigest::for_bytes(bytes) != entry.digest
    {
        return Err(ArchiveError::Integrity(
            "founder checkpoint asset digest mismatch".to_string(),
        ));
    }
    write_content_addressed(staging, &root.join(&entry.relative_path), bytes)?;
    merge_save_asset_entry(manifest, entry.clone())
}

fn merge_save_asset_entry(
    manifest: &mut AssetManifest,
    entry: AssetManifestEntry,
) -> Result<(), ArchiveError> {
    if let Some(existing) = manifest
        .entries
        .iter()
        .find(|existing| existing.asset_id == entry.asset_id)
    {
        if existing == &entry {
            return Ok(());
        }
        return Err(ArchiveError::Integrity(
            "founder save asset id collision".to_string(),
        ));
    }
    manifest.entries.push(entry);
    Ok(())
}

fn validate_supported_manifest(manifest: &CreatureArchiveManifest) -> Result<(), ArchiveError> {
    manifest.validate_contract()?;
    BrainCapacityClass::production_for_id(manifest.genetic.brain_class_id)?;
    let language = LanguageCodebookV1::canonical();
    if manifest.genetic.language_codebook_id != language.id()
        || manifest.genetic.language_codebook_digest != language.canonical_digest()
    {
        return Err(ArchiveError::Integrity(
            "unsupported language codebook".to_string(),
        ));
    }
    Ok(())
}

fn deterministic_remap(
    seed: u64,
    index: u64,
    manifest: &CreatureArchiveManifest,
    mode: &FounderMode,
) -> Result<FounderIdentityRemap, ArchiveError> {
    fn lane(seed: u64, index: u64, label: &[u8]) -> u64 {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"alife-founder-remap-v1");
        hasher.update(&seed.to_le_bytes());
        hasher.update(&index.to_le_bytes());
        hasher.update(label);
        let mut bytes = [0_u8; 8];
        bytes.copy_from_slice(&hasher.finalize().as_bytes()[..8]);
        u64::from_le_bytes(bytes).max(1)
    }
    let target_genome_id = match mode {
        FounderMode::GeneticFounder | FounderMode::MindStateClone { .. } => {
            GenomeId(lane(seed, index, b"genome"))
        }
        FounderMode::GeneticOffspring { mutation_seed } => {
            GenomeId(lane(seed ^ mutation_seed, index, b"offspring-genome"))
        }
    };
    let value = FounderIdentityRemap {
        source_organism_id: manifest.genetic.organism_id,
        source_genome_id: manifest.genetic.genome_id,
        source_lineage_id: manifest.genetic.lineage_id,
        target_organism_id: OrganismId(lane(seed, index, b"organism")),
        target_genome_id,
        target_lineage_id: LineageId(lane(seed, index, b"lineage")),
        target_name_id: lane(seed, index, b"name"),
        target_social_id: lane(seed, index, b"social"),
    };
    value.validate_contract()?;
    Ok(value)
}

fn deterministic_offspring_genome(
    parent: &BrainGenome,
    mutation_seed: u64,
) -> Result<BrainGenome, ArchiveError> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"alife-genetic-offspring-v1");
    hasher.update(&parent.species_seed.to_le_bytes());
    hasher.update(&mutation_seed.to_le_bytes());
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&hasher.finalize().as_bytes()[..8]);
    let species_seed = u64::from_le_bytes(bytes).max(1);
    let mut child = BrainGenome::scaffold(species_seed, parent.brain_class_id);
    child.parent_genome_ids = vec![parent.id];
    child.lineage_id = parent.lineage_id;
    child.validate_contract()?;
    Ok(child)
}

fn read_archive_asset(
    library: &LineageLibrary,
    reference: &alife_core::ArchiveAssetRef,
) -> Result<Vec<u8>, ArchiveError> {
    let path = library
        .config
        .root
        .join("assets")
        .join(digest_hex(reference.digest))
        .join("payload.bin");
    let bytes = fs::read(path)?;
    if bytes.len() as u64 != reference.size_bytes || digest_bytes(&bytes) != reference.digest {
        return Err(ArchiveError::Integrity(
            "archive asset digest mismatch".to_string(),
        ));
    }
    Ok(bytes)
}

fn validate_bundle_graph(
    entries: &[BundleEntry],
    descriptor: &BundleDescriptor,
) -> Result<(), ArchiveError> {
    if descriptor.manifest_digests.is_empty()
        || descriptor.manifest_digests.len() > MAX_COHORT_FOUNDERS
        || (descriptor.kind == FounderBundleKind::Creature
            && descriptor.manifest_digests.len() != 1)
    {
        return Err(ArchiveError::Integrity(
            "bundle descriptor has invalid founder count".to_string(),
        ));
    }
    let map = entries
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    for selected in &descriptor.manifest_digests {
        let path = format!("manifests/{}.json", digest_hex(*selected));
        let entry = map.get(path.as_str()).ok_or_else(|| {
            ArchiveError::Integrity("selected creature manifest is missing".to_string())
        })?;
        validate_manifest_graph(entry, &map)?;
    }
    Ok(())
}

fn validate_manifest_graph(
    entry: &BundleEntry,
    entries: &BTreeMap<&str, &BundleEntry>,
) -> Result<(), ArchiveError> {
    let manifest = serde_json::from_slice::<CreatureArchiveManifest>(&entry.bytes)?;
    validate_supported_manifest(&manifest)?;
    for asset in [
        Some(&manifest.genetic.genome_asset),
        manifest.genetic.foundation_asset.as_ref(),
        manifest.life.as_ref().map(|life| &life.statistics_asset),
    ]
    .into_iter()
    .flatten()
    {
        let path = format!("assets/{}/payload.bin", digest_hex(asset.digest));
        let actual = entries
            .get(path.as_str())
            .ok_or_else(|| ArchiveError::Integrity("bundle asset is missing".to_string()))?;
        if actual.bytes.len() as u64 != asset.size_bytes || actual.digest != asset.digest {
            return Err(ArchiveError::Integrity(
                "bundle asset does not match its manifest".to_string(),
            ));
        }
    }
    if let Some(previous) = manifest.previous_manifest_digest {
        let path = format!("manifests/{}.json", digest_hex(previous));
        let previous_entry = entries
            .get(path.as_str())
            .ok_or_else(|| ArchiveError::Integrity("birth manifest is missing".to_string()))?;
        validate_manifest_graph(previous_entry, entries)?;
    }
    if let Some(life) = &manifest.life {
        if let ArchiveCheckpointDisposition::Stored(checkpoint) = &life.checkpoint {
            for (index, page) in checkpoint.pages.iter().enumerate() {
                let path = format!(
                    "checkpoints/{}/{index:08}-{}.zst",
                    digest_hex(checkpoint.digest),
                    digest_hex(page.digest)
                );
                let actual = entries.get(path.as_str()).ok_or_else(|| {
                    ArchiveError::Integrity("checkpoint page is missing".to_string())
                })?;
                if actual.bytes.len() != page.compressed_bytes as usize
                    || actual.digest != page.digest
                {
                    return Err(ArchiveError::Integrity(
                        "checkpoint page does not match its manifest".to_string(),
                    ));
                }
            }
            let checkpoint_bytes = decode_checkpoint_from_entries(checkpoint, entries)?;
            if let Ok(envelope) =
                serde_json::from_slice::<ArchivedGpuCheckpointEnvelope>(&checkpoint_bytes)
            {
                validate_checkpoint_envelope(&envelope)?;
                for asset in &envelope.manifest_entries {
                    validate_entry_path(&asset.relative_path)?;
                    let actual = entries.get(asset.relative_path.as_str()).ok_or_else(|| {
                        ArchiveError::Integrity("GPU checkpoint asset is missing".to_string())
                    })?;
                    if asset
                        .size_bytes
                        .is_some_and(|size| size != actual.bytes.len() as u64)
                        || PortableAssetDigest::for_bytes(&actual.bytes) != asset.digest
                    {
                        return Err(ArchiveError::Integrity(
                            "GPU checkpoint asset does not match its manifest".to_string(),
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_checkpoint_envelope(
    envelope: &ArchivedGpuCheckpointEnvelope,
) -> Result<(), ArchiveError> {
    if envelope.checkpoint_digest == [0; 4] || envelope.manifest_entries.is_empty() {
        return Err(ArchiveError::Integrity(
            "GPU checkpoint envelope is incomplete".to_string(),
        ));
    }
    envelope
        .save_state
        .validate()
        .map_err(|error| ArchiveError::Integrity(format!("invalid GPU checkpoint: {error}")))?;
    let manifest = AssetManifest {
        schema: P34_ASSET_MANIFEST_SCHEMA.to_string(),
        schema_version: P34_ASSET_MANIFEST_SCHEMA_VERSION,
        entries: envelope.manifest_entries.clone(),
    };
    envelope
        .save_state
        .validate_asset_manifest(&manifest)
        .map_err(|error| ArchiveError::Integrity(format!("invalid GPU checkpoint: {error}")))?;
    Ok(())
}

fn decode_checkpoint_from_entries(
    checkpoint: &alife_core::ArchiveCheckpointRef,
    entries: &BTreeMap<&str, &BundleEntry>,
) -> Result<Vec<u8>, ArchiveError> {
    let mut output = Vec::with_capacity(checkpoint.total_uncompressed_bytes as usize);
    for (index, page) in checkpoint.pages.iter().enumerate() {
        let path = format!(
            "checkpoints/{}/{index:08}-{}.zst",
            digest_hex(checkpoint.digest),
            digest_hex(page.digest)
        );
        let entry = entries
            .get(path.as_str())
            .ok_or_else(|| ArchiveError::Integrity("checkpoint page is missing".to_string()))?;
        let decoded = zstd::stream::decode_all(entry.bytes.as_slice())?;
        if decoded.len() != page.uncompressed_bytes as usize {
            return Err(ArchiveError::Integrity(
                "checkpoint page length mismatch".to_string(),
            ));
        }
        output.extend_from_slice(&decoded);
    }
    if output.len() != checkpoint.total_uncompressed_bytes as usize
        || digest_bytes(&output) != checkpoint.digest
    {
        return Err(ArchiveError::Integrity(
            "learned checkpoint digest mismatch".to_string(),
        ));
    }
    Ok(output)
}

fn validate_bundle_destination(path: &Path, kind: FounderBundleKind) -> Result<(), ArchiveError> {
    if path.extension().and_then(|value| value.to_str()) != Some(kind.extension()) {
        return Err(ArchiveError::Integrity(format!(
            "bundle must use .{} extension",
            kind.extension()
        )));
    }
    Ok(())
}

fn validate_entry_path(path: &str) -> Result<(), ArchiveError> {
    if path.is_empty() || path.len() > MAX_PATH_BYTES || path.contains('\\') {
        return Err(ArchiveError::Integrity("unsafe bundle path".to_string()));
    }
    let path = Path::new(path);
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(ArchiveError::Integrity("unsafe bundle path".to_string()));
    }
    Ok(())
}

fn encode_bundle(
    kind: FounderBundleKind,
    entries: &[BundleEntry],
) -> Result<Vec<u8>, ArchiveError> {
    if entries.is_empty() || entries.len() > MAX_ENTRY_COUNT {
        return Err(ArchiveError::Integrity(
            "invalid bundle entry count".to_string(),
        ));
    }
    let mut output = Vec::new();
    output.extend_from_slice(BUNDLE_MAGIC);
    output.extend_from_slice(&BUNDLE_VERSION.to_le_bytes());
    output.push(kind.byte());
    output.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    let mut paths = BTreeSet::new();
    for entry in entries {
        validate_entry_path(&entry.path)?;
        if entry.bytes.is_empty()
            || entry.bytes.len() > MAX_ENTRY_BYTES
            || digest_bytes(&entry.bytes) != entry.digest
            || !paths.insert(entry.path.as_str())
        {
            return Err(ArchiveError::Integrity("invalid bundle entry".to_string()));
        }
        let path = entry.path.as_bytes();
        output.extend_from_slice(&(path.len() as u16).to_le_bytes());
        output.extend_from_slice(path);
        output.extend_from_slice(&(entry.bytes.len() as u64).to_le_bytes());
        output.extend_from_slice(entry.digest.bytes());
        output.extend_from_slice(&entry.bytes);
        if output.len() > MAX_BUNDLE_UNCOMPRESSED_BYTES {
            return Err(ArchiveError::Integrity(
                "bundle exceeds size limit".to_string(),
            ));
        }
    }
    Ok(output)
}

fn decode_bundle(bytes: &[u8]) -> Result<(FounderBundleKind, Vec<BundleEntry>), ArchiveError> {
    let mut cursor = Cursor::new(bytes);
    let mut magic = [0_u8; 8];
    cursor.read_exact(&mut magic)?;
    if &magic != BUNDLE_MAGIC || read_u16(&mut cursor)? != BUNDLE_VERSION {
        return Err(ArchiveError::Integrity(
            "unsupported bundle format".to_string(),
        ));
    }
    let mut kind = [0_u8; 1];
    cursor.read_exact(&mut kind)?;
    let kind = FounderBundleKind::from_byte(kind[0])?;
    let entry_count = read_u32(&mut cursor)? as usize;
    if entry_count == 0 || entry_count > MAX_ENTRY_COUNT {
        return Err(ArchiveError::Integrity(
            "invalid bundle entry count".to_string(),
        ));
    }
    let mut paths = BTreeSet::new();
    let mut entries = Vec::with_capacity(entry_count);
    for _ in 0..entry_count {
        let path_len = read_u16(&mut cursor)? as usize;
        if path_len == 0 || path_len > MAX_PATH_BYTES {
            return Err(ArchiveError::Integrity("unsafe bundle path".to_string()));
        }
        let mut path_bytes = vec![0_u8; path_len];
        cursor.read_exact(&mut path_bytes)?;
        let path = String::from_utf8(path_bytes)
            .map_err(|_| ArchiveError::Integrity("bundle path is not UTF-8".to_string()))?;
        validate_entry_path(&path)?;
        if !paths.insert(path.clone()) {
            return Err(ArchiveError::Integrity("duplicate bundle path".to_string()));
        }
        let content_len = usize::try_from(read_u64(&mut cursor)?)
            .map_err(|_| ArchiveError::Integrity("bundle entry is oversized".to_string()))?;
        if content_len == 0 || content_len > MAX_ENTRY_BYTES {
            return Err(ArchiveError::Integrity(
                "bundle entry is oversized".to_string(),
            ));
        }
        let mut digest = [0_u8; 32];
        cursor.read_exact(&mut digest)?;
        let mut content = vec![0_u8; content_len];
        cursor.read_exact(&mut content)?;
        let digest = Blake3Digest::from_bytes(digest);
        if digest_bytes(&content) != digest {
            return Err(ArchiveError::Integrity(
                "bundle entry digest mismatch".to_string(),
            ));
        }
        entries.push(BundleEntry {
            path,
            digest,
            bytes: content,
        });
    }
    if cursor.position() != bytes.len() as u64 {
        return Err(ArchiveError::Integrity(
            "bundle has trailing data".to_string(),
        ));
    }
    Ok((kind, entries))
}

fn read_u16(cursor: &mut Cursor<&[u8]>) -> Result<u16, ArchiveError> {
    let mut bytes = [0_u8; 2];
    cursor.read_exact(&mut bytes)?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_u32(cursor: &mut Cursor<&[u8]>) -> Result<u32, ArchiveError> {
    let mut bytes = [0_u8; 4];
    cursor.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64(cursor: &mut Cursor<&[u8]>) -> Result<u64, ArchiveError> {
    let mut bytes = [0_u8; 8];
    cursor.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, time::SystemTime};

    use alife_core::{
        ArchiveCheckpointRetention, BrainCapacityClass, BrainGenome, DevelopmentState, FounderMode,
        FounderSelection, NormalizedScalar, OrganismId, PhenotypeCompiler, SensorProfile, Tick,
    };

    use super::*;
    use crate::{GeneticArchiveInput, LifeArchiveInput, LineageLibraryConfig};

    fn temp_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "alife-founder-bundle-{label}-{}-{nonce}",
            std::process::id()
        ))
    }

    fn copy_tree(source: &Path, destination: &Path) {
        fs::create_dir_all(destination).unwrap();
        for entry in fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            let target = destination.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&entry.path(), &target);
            } else {
                fs::copy(entry.path(), target).unwrap();
            }
        }
    }

    fn archive_fixture(
        root: &Path,
        organism_raw: u64,
        learned: bool,
    ) -> (LineageLibrary, Blake3Digest, Option<Blake3Digest>) {
        let mut library =
            LineageLibrary::open(LineageLibraryConfig::profile_default(root)).unwrap();
        let capacity = BrainCapacityClass::production_for_id(BrainCapacityClass::N512_ID).unwrap();
        let genome = BrainGenome::scaffold(1000 + organism_raw, capacity.id());
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.25).unwrap());
        let phenotype = PhenotypeCompiler::compile(
            &genome,
            &capacity,
            &development,
            SensorProfile::GroundedObjectSlotsV1,
        )
        .unwrap();
        let birth = library
            .archive_birth(GeneticArchiveInput {
                source_run_id: "founder-source",
                organism_id: OrganismId(organism_raw),
                birth_tick: Tick::ZERO,
                genome: &genome,
                phenotype: &phenotype,
                foundation_asset_bytes: None,
            })
            .unwrap();
        if !learned {
            return (library, birth, None);
        }
        let checkpoint = format!("durable-learned-founder-{organism_raw}").into_bytes();
        let retirement = library
            .archive_life(LifeArchiveInput {
                birth_manifest_digest: birth,
                death_tick: Tick(80),
                final_experience_sequence: None,
                statistics_bytes: b"{}",
                learned_checkpoint_bytes: Some(&checkpoint),
                checkpoint_retention: ArchiveCheckpointRetention::Pinned,
            })
            .unwrap();
        (
            library,
            retirement.committed_manifest_digest,
            retirement.learned_checkpoint_digest,
        )
    }

    #[test]
    fn creature_bundle_round_trip_and_founder_modes_are_explicit() {
        let source_root = temp_root("roundtrip-source");
        let import_root = temp_root("roundtrip-import");
        let bundle_path = temp_root("roundtrip-bundle").with_extension("alife-creature");
        let (source, manifest_digest, checkpoint_digest) = archive_fixture(&source_root, 41, true);
        source
            .export_creature_bundle(manifest_digest, &bundle_path)
            .unwrap();

        let mut imported =
            LineageLibrary::open(LineageLibraryConfig::profile_default(&import_root)).unwrap();
        let receipt = imported.import_bundle(&bundle_path).unwrap();
        assert_eq!(receipt.kind, FounderBundleKind::Creature);
        assert_eq!(receipt.manifest_digests, vec![manifest_digest]);
        assert_eq!(imported.manifest_count().unwrap(), 2);

        let genetic = imported
            .resolve_founder_cohort(
                "new-genetic-world",
                500,
                &[FounderSelection {
                    source_manifest_digest: manifest_digest,
                    mode: FounderMode::default(),
                }],
            )
            .unwrap();
        assert!(genetic.founders[0].learned_checkpoint_bytes.is_none());
        assert!(matches!(
            genetic.founders[0].selection.mode,
            FounderMode::GeneticFounder
        ));

        let offspring = imported
            .resolve_founder_cohort(
                "new-offspring-world",
                502,
                &[FounderSelection {
                    source_manifest_digest: manifest_digest,
                    mode: FounderMode::GeneticOffspring { mutation_seed: 77 },
                }],
            )
            .unwrap();
        assert_ne!(
            offspring.founders[0].genome.id,
            genetic.founders[0].genome.id
        );
        assert_eq!(
            offspring.founders[0].genome.parent_genome_ids,
            vec![genetic.founders[0].genome.id]
        );

        let checkpoint_digest = checkpoint_digest.unwrap();
        let learned = imported.resolve_founder_cohort(
            "new-learned-world",
            501,
            &[FounderSelection {
                source_manifest_digest: manifest_digest,
                mode: FounderMode::MindStateClone { checkpoint_digest },
            }],
        );
        assert!(learned.is_err(), "opaque checkpoints are not mind clones");

        drop(imported);
        drop(source);
        let _ = fs::remove_dir_all(source_root);
        let _ = fs::remove_dir_all(import_root);
        let _ = fs::remove_file(bundle_path);
    }

    #[test]
    fn cohort_bundle_preserves_distinct_selected_creatures() {
        let source_root = temp_root("cohort-source");
        let import_root = temp_root("cohort-import");
        let bundle_path = temp_root("cohort-bundle").with_extension("alife-cohort");
        let (mut source, first, _) = archive_fixture(&source_root, 51, false);

        let capacity = BrainCapacityClass::production_for_id(BrainCapacityClass::N512_ID).unwrap();
        let genome = BrainGenome::scaffold(1052, capacity.id());
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.25).unwrap());
        let phenotype = PhenotypeCompiler::compile(
            &genome,
            &capacity,
            &development,
            SensorProfile::GroundedObjectSlotsV1,
        )
        .unwrap();
        let second = source
            .archive_birth(GeneticArchiveInput {
                source_run_id: "founder-source-two",
                organism_id: OrganismId(52),
                birth_tick: Tick::ZERO,
                genome: &genome,
                phenotype: &phenotype,
                foundation_asset_bytes: None,
            })
            .unwrap();
        source
            .export_cohort_bundle(&[first, second], &bundle_path)
            .unwrap();
        let mut imported =
            LineageLibrary::open(LineageLibraryConfig::profile_default(&import_root)).unwrap();
        let receipt = imported.import_bundle(&bundle_path).unwrap();
        assert_eq!(receipt.kind, FounderBundleKind::Cohort);
        assert_eq!(receipt.manifest_digests.len(), 2);
        assert_eq!(imported.manifest_count().unwrap(), 2);
        let cohort = imported
            .resolve_founder_cohort(
                "new-cohort-world",
                800,
                &receipt
                    .manifest_digests
                    .iter()
                    .map(|digest| FounderSelection {
                        source_manifest_digest: *digest,
                        mode: FounderMode::GeneticFounder,
                    })
                    .collect::<Vec<_>>(),
            )
            .unwrap();
        assert_eq!(cohort.founders.len(), 2);
        assert_ne!(
            cohort.founders[0].provenance.remap.target_organism_id,
            cohort.founders[1].provenance.remap.target_organism_id
        );

        drop(imported);
        drop(source);
        let _ = fs::remove_dir_all(source_root);
        let _ = fs::remove_dir_all(import_root);
        let _ = fs::remove_file(bundle_path);
    }

    #[test]
    fn malformed_bundle_cannot_publish_partial_content() {
        let source_root = temp_root("missing-source");
        let import_root = temp_root("missing-import");
        let bundle_path = temp_root("missing-bundle").with_extension("alife-creature");
        let bad_path = temp_root("missing-bad").with_extension("alife-creature");
        let (source, manifest_digest, _) = archive_fixture(&source_root, 61, false);
        source
            .export_creature_bundle(manifest_digest, &bundle_path)
            .unwrap();
        let compressed = fs::read(&bundle_path).unwrap();
        let encoded = zstd::stream::decode_all(compressed.as_slice()).unwrap();
        let (kind, mut entries) = decode_bundle(&encoded).unwrap();
        entries.retain(|entry| !entry.path.starts_with("assets/"));
        let malformed = encode_bundle(kind, &entries).unwrap();
        fs::write(
            &bad_path,
            zstd::stream::encode_all(malformed.as_slice(), 3).unwrap(),
        )
        .unwrap();

        let mut imported =
            LineageLibrary::open(LineageLibraryConfig::profile_default(&import_root)).unwrap();
        assert!(imported.import_bundle(&bad_path).is_err());
        assert_eq!(imported.manifest_count().unwrap(), 0);
        assert!(fs::read_dir(import_root.join("assets"))
            .unwrap()
            .next()
            .is_none());
        assert!(fs::read_dir(import_root.join("manifests"))
            .unwrap()
            .next()
            .is_none());

        drop(imported);
        drop(source);
        let _ = fs::remove_dir_all(source_root);
        let _ = fs::remove_dir_all(import_root);
        let _ = fs::remove_file(bundle_path);
        let _ = fs::remove_file(bad_path);
    }

    #[test]
    fn unsafe_duplicate_oversized_and_digest_mismatch_entries_are_rejected() {
        let bytes = b"safe".to_vec();
        let digest = digest_bytes(&bytes);
        assert!(encode_bundle(
            FounderBundleKind::Creature,
            &[BundleEntry {
                path: "../escape".to_string(),
                digest,
                bytes: bytes.clone(),
            }]
        )
        .is_err());
        assert!(encode_bundle(
            FounderBundleKind::Creature,
            &[
                BundleEntry {
                    path: "assets/same".to_string(),
                    digest,
                    bytes: bytes.clone(),
                },
                BundleEntry {
                    path: "assets/same".to_string(),
                    digest,
                    bytes: bytes.clone(),
                },
            ]
        )
        .is_err());

        let mut oversized = Vec::new();
        oversized.extend_from_slice(BUNDLE_MAGIC);
        oversized.extend_from_slice(&BUNDLE_VERSION.to_le_bytes());
        oversized.push(FounderBundleKind::Creature.byte());
        oversized.extend_from_slice(&1_u32.to_le_bytes());
        oversized.extend_from_slice(&1_u16.to_le_bytes());
        oversized.push(b'x');
        oversized.extend_from_slice(&((MAX_ENTRY_BYTES as u64) + 1).to_le_bytes());
        oversized.extend_from_slice(&[0_u8; 32]);
        assert!(decode_bundle(&oversized).is_err());

        let mut encoded = encode_bundle(
            FounderBundleKind::Creature,
            &[BundleEntry {
                path: "assets/value".to_string(),
                digest,
                bytes,
            }],
        )
        .unwrap();
        *encoded.last_mut().unwrap() ^= 0xFF;
        assert!(decode_bundle(&encoded).is_err());
    }

    #[test]
    fn mind_clone_requires_the_exact_archived_checkpoint() {
        let root = temp_root("checkpoint-selection");
        let (library, manifest_digest, _) = archive_fixture(&root, 71, true);
        let result = library.resolve_founder_cohort(
            "bad-checkpoint-world",
            900,
            &[FounderSelection {
                source_manifest_digest: manifest_digest,
                mode: FounderMode::MindStateClone {
                    checkpoint_digest: Blake3Digest::from_bytes([7; 32]),
                },
            }],
        );
        assert!(result.is_err());
        drop(library);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn genetic_founders_create_a_complete_valid_new_save() {
        let archive_root = temp_root("new-save-archive");
        let save_root = temp_root("new-save-root");
        copy_tree(Path::new("../alife_world/tests/fixtures/p34"), &save_root);
        let (library, manifest_digest, _) = archive_fixture(&archive_root, 81, false);
        let cohort = library
            .resolve_founder_cohort(
                "founder-world",
                4242,
                &[FounderSelection {
                    source_manifest_digest: manifest_digest,
                    mode: FounderMode::GeneticFounder,
                }],
            )
            .unwrap();
        let mut base = PortableSaveFile::from_json_file(save_root.join("tiny_save.json")).unwrap();
        let mut world = base.restore_headless_world().unwrap();
        world.remove_organism(OrganismId(1)).unwrap();
        base.replace_headless_world_snapshot(&world).unwrap();
        base.save_id = "founder-world".to_string();
        base.gpu_runtime = None;
        base.creatures.clear();

        let save = library
            .create_new_save_from_founders(base, &save_root, &cohort)
            .unwrap();
        save.validate_with_asset_root(&save_root).unwrap();
        assert_eq!(save.creatures.len(), 1);
        assert_eq!(
            save.creatures[0].organism_id,
            cohort.manifest.founders[0].remap.target_organism_id
        );
        assert_eq!(
            save.creatures[0].genome_id,
            cohort.manifest.founders[0].remap.target_genome_id
        );
        assert!(save.creatures[0].gpu_brain.is_none());
        assert_eq!(
            save.restore_headless_world()
                .unwrap()
                .organism_entity_ids()
                .len(),
            1
        );
        assert!(save_root.join("founders/cohort.json").is_file());

        drop(library);
        let _ = fs::remove_dir_all(archive_root);
        let _ = fs::remove_dir_all(save_root);
    }
}
