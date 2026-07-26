//! Durable cross-save creature archive and retirement provenance contracts.

use serde::{Deserialize, Serialize};

use crate::{
    Blake3Digest, BrainClassId, ExperienceSequenceId, FoundationCompatibilityFamilyId,
    FoundationId, FoundationVersion, GenomeId, LanguageCodebookId, LineageId, OrganismId,
    PhenotypeHash, ScaffoldContractError, SensorProfile, Tick, Validate,
};

pub const CREATURE_ARCHIVE_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArchiveAssetKind {
    Genome,
    Foundation,
    LifeStatistics,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveAssetRef {
    pub kind: ArchiveAssetKind,
    pub digest: Blake3Digest,
    pub size_bytes: u64,
}

impl Validate for ArchiveAssetRef {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.digest.bytes().iter().all(|byte| *byte == 0) || self.size_bytes == 0 {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArchiveCheckpointRetention {
    TemporaryPeak,
    AutomaticPermanent,
    Pinned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArchiveLearnedCapturePolicy {
    GeneticOnly,
    TemporaryPeak,
    AutomaticPermanent,
    Pinned,
}

impl ArchiveLearnedCapturePolicy {
    pub const fn retention(self) -> Option<ArchiveCheckpointRetention> {
        match self {
            Self::GeneticOnly => None,
            Self::TemporaryPeak => Some(ArchiveCheckpointRetention::TemporaryPeak),
            Self::AutomaticPermanent => Some(ArchiveCheckpointRetention::AutomaticPermanent),
            Self::Pinned => Some(ArchiveCheckpointRetention::Pinned),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchivePageRef {
    pub digest: Blake3Digest,
    pub compressed_bytes: u32,
    pub uncompressed_bytes: u32,
}

impl Validate for ArchivePageRef {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.digest.bytes().iter().all(|byte| *byte == 0)
            || self.compressed_bytes == 0
            || self.uncompressed_bytes == 0
            || self.uncompressed_bytes > 65_536
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveCheckpointRef {
    pub digest: Blake3Digest,
    pub retention: ArchiveCheckpointRetention,
    pub total_uncompressed_bytes: u64,
    pub total_compressed_bytes: u64,
    pub pages: Vec<ArchivePageRef>,
}

impl Validate for ArchiveCheckpointRef {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.digest.bytes().iter().all(|byte| *byte == 0)
            || self.total_uncompressed_bytes == 0
            || self.total_compressed_bytes == 0
            || self.pages.is_empty()
            || self.pages.len() > 65_536
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        for page in &self.pages {
            page.validate_contract()?;
        }
        let uncompressed = self
            .pages
            .iter()
            .map(|page| u64::from(page.uncompressed_bytes))
            .sum::<u64>();
        let compressed = self
            .pages
            .iter()
            .map(|page| u64::from(page.compressed_bytes))
            .sum::<u64>();
        if uncompressed != self.total_uncompressed_bytes
            || compressed != self.total_compressed_bytes
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArchiveCheckpointDisposition {
    NotSelected,
    Stored(ArchiveCheckpointRef),
    DowngradedToGeneticOnly { reason: String },
}

impl Validate for ArchiveCheckpointDisposition {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        match self {
            Self::NotSelected => Ok(()),
            Self::Stored(checkpoint) => checkpoint.validate_contract(),
            Self::DowngradedToGeneticOnly { reason }
                if !reason.trim().is_empty() && reason.chars().count() <= 160 =>
            {
                Ok(())
            }
            Self::DowngradedToGeneticOnly { .. } => Err(ScaffoldContractError::InvalidId),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneticArchiveRecord {
    pub source_run_id: String,
    pub organism_id: OrganismId,
    pub genome_id: GenomeId,
    pub lineage_id: Option<LineageId>,
    pub brain_class_id: BrainClassId,
    pub birth_tick: Tick,
    pub sensor_profile: SensorProfile,
    pub phenotype_hash: PhenotypeHash,
    pub foundation_id: Option<FoundationId>,
    pub foundation_version: Option<FoundationVersion>,
    pub compatibility_family_id: Option<FoundationCompatibilityFamilyId>,
    pub foundation_payload_digest: Option<Blake3Digest>,
    pub persistent_address_map_digest: Blake3Digest,
    pub language_codebook_id: LanguageCodebookId,
    pub language_codebook_digest: Blake3Digest,
    pub genome_asset: ArchiveAssetRef,
    pub foundation_asset: Option<ArchiveAssetRef>,
}

impl Validate for GeneticArchiveRecord {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        self.genome_id.validate()?;
        self.brain_class_id.validate()?;
        if let Some(lineage) = self.lineage_id {
            lineage.validate()?;
        }
        self.genome_asset.validate_contract()?;
        if self.genome_asset.kind != ArchiveAssetKind::Genome
            || self.source_run_id.trim().is_empty()
            || self.source_run_id.chars().count() > 96
            || self.phenotype_hash.0 == [0; 4]
            || self
                .persistent_address_map_digest
                .bytes()
                .iter()
                .all(|byte| *byte == 0)
            || self
                .language_codebook_digest
                .bytes()
                .iter()
                .all(|byte| *byte == 0)
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        let foundation_identity = (
            self.foundation_id,
            self.foundation_version,
            self.compatibility_family_id,
            self.foundation_payload_digest,
            self.foundation_asset.as_ref(),
        );
        if !matches!(
            foundation_identity,
            (Some(_), Some(_), Some(_), Some(_), Some(_)) | (None, None, None, None, None)
        ) {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        if let Some(asset) = &self.foundation_asset {
            asset.validate_contract()?;
            if asset.kind != ArchiveAssetKind::Foundation {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatureLifeArchiveRecord {
    pub death_tick: Tick,
    pub final_experience_sequence: Option<ExperienceSequenceId>,
    pub statistics_asset: ArchiveAssetRef,
    pub checkpoint: ArchiveCheckpointDisposition,
}

impl Validate for CreatureLifeArchiveRecord {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if let Some(sequence) = self.final_experience_sequence {
            sequence.validate()?;
        }
        self.statistics_asset.validate_contract()?;
        self.checkpoint.validate_contract()?;
        if self.statistics_asset.kind != ArchiveAssetKind::LifeStatistics {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreatureArchiveManifest {
    pub schema_version: u16,
    pub genetic: GeneticArchiveRecord,
    pub previous_manifest_digest: Option<Blake3Digest>,
    pub life: Option<CreatureLifeArchiveRecord>,
}

impl Validate for CreatureArchiveManifest {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != CREATURE_ARCHIVE_SCHEMA_VERSION
            || self.life.is_some() != self.previous_manifest_digest.is_some()
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        self.genetic.validate_contract()?;
        if let Some(previous) = self.previous_manifest_digest {
            if previous.bytes().iter().all(|byte| *byte == 0) {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        if let Some(life) = &self.life {
            life.validate_contract()?;
            if life.death_tick.raw() < self.genetic.birth_tick.raw() {
                return Err(ScaffoldContractError::NonMonotonicTick);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveRetirementReceipt {
    pub organism_id: OrganismId,
    pub committed_manifest_digest: Blake3Digest,
    pub learned_checkpoint_digest: Option<Blake3Digest>,
    pub death_tick: Tick,
}

impl Validate for ArchiveRetirementReceipt {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        if self
            .committed_manifest_digest
            .bytes()
            .iter()
            .all(|byte| *byte == 0)
            || self
                .learned_checkpoint_digest
                .is_some_and(|digest| digest.bytes().iter().all(|byte| *byte == 0))
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }
}
