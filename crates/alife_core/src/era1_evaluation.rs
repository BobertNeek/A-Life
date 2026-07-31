//! Contract-only evidence identities for the Era 1 Norn-plus program.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    BrainCapacityClass, BrainClassId, GenomeId, LineageId, MetricReading, OrganismId,
    PhenotypeHash, PolicyBackend, ScaffoldContractError, SensorProfile, Validate,
};

pub const ERA1_EVALUATION_SCHEMA_VERSION: u16 = 1;
pub const ERA1_ABILITY_COUNT: usize = 11;
pub const ERA1_CONTROL_COUNT: usize = 5;
const Q16_ONE: u32 = 65_535;
const GIT_OBJECT_HEX_LEN: usize = 40;
const MAX_PROVENANCE_TEXT_CHARS: usize = 128;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Era1Ability {
    FlexibleForaging = 1,
    HazardAvoidance = 2,
    SpatialMemory = 3,
    DelayedChoice = 4,
    RewardReversal = 5,
    ObjectTransfer = 6,
    MultiStepProblem = 7,
    IndividualRecognition = 8,
    Imitation = 9,
    GroundedLanguage = 10,
    PostSleepRetention = 11,
}

impl Era1Ability {
    pub const ALL: [Self; ERA1_ABILITY_COUNT] = [
        Self::FlexibleForaging,
        Self::HazardAvoidance,
        Self::SpatialMemory,
        Self::DelayedChoice,
        Self::RewardReversal,
        Self::ObjectTransfer,
        Self::MultiStepProblem,
        Self::IndividualRecognition,
        Self::Imitation,
        Self::GroundedLanguage,
        Self::PostSleepRetention,
    ];
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Era1Control {
    Intact = 0,
    PlasticityDisabled = 1,
    MemoryDisabled = 2,
    SleepDisabled = 3,
    SocialDisabled = 4,
}

impl Era1Control {
    pub const ALL: [Self; ERA1_CONTROL_COUNT] = [
        Self::Intact,
        Self::PlasticityDisabled,
        Self::MemoryDisabled,
        Self::SleepDisabled,
        Self::SocialDisabled,
    ];
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Era1EvidencePartition {
    Acquisition = 1,
    DelayedProbe = 2,
    ReversalProbe = 3,
    HeldOutTransfer = 4,
    PostSleepProbe = 5,
    SocialTransfer = 6,
    ReproducedOffspring = 7,
}

impl Era1EvidencePartition {
    pub const fn requires_unassisted_evidence(self) -> bool {
        !matches!(self, Self::Acquisition)
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Era1AssistanceKind {
    Teacher = 1,
    SemanticPrior = 2,
    Translation = 3,
    Player = 4,
    Possession = 5,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1TrialIdentity {
    pub seed: u64,
    pub organism_id: OrganismId,
    pub genome_id: GenomeId,
    pub parent_genome_ids: Vec<GenomeId>,
    pub lineage_id: LineageId,
    pub generation: u32,
    pub brain_class_id: BrainClassId,
    pub world_family_id: u64,
    pub world_variant_id: u64,
}

impl Validate for Era1TrialIdentity {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        self.genome_id.validate()?;
        self.lineage_id.validate()?;
        self.brain_class_id.validate()?;
        if self.seed == 0
            || self.world_family_id == 0
            || self.world_variant_id == 0
            || self.brain_class_id != BrainCapacityClass::N2048_ID
        {
            return Err(ScaffoldContractError::InvalidId);
        }

        match self.generation {
            0 if self.parent_genome_ids.is_empty() => {}
            0 => return Err(ScaffoldContractError::InvalidId),
            _ if self.parent_genome_ids.len() == 2 => {
                let maternal = self.parent_genome_ids[0];
                let paternal = self.parent_genome_ids[1];
                maternal.validate()?;
                paternal.validate()?;
                if maternal == paternal || maternal == self.genome_id || paternal == self.genome_id
                {
                    return Err(ScaffoldContractError::InvalidId);
                }
            }
            _ => return Err(ScaffoldContractError::InvalidId),
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1TrialReceipt {
    pub schema_version: u16,
    pub identity: Era1TrialIdentity,
    pub ability: Era1Ability,
    pub control: Era1Control,
    pub partition: Era1EvidencePartition,
    pub score: MetricReading,
    pub phenotype_hash: PhenotypeHash,
    pub foundation_id: u64,
    pub foundation_version: u32,
    pub sensor_profile: SensorProfile,
    pub policy_backend: PolicyBackend,
    pub world_digest: [u64; 4],
    pub perception_digest: [u64; 4],
    pub sealed_evidence_digest: [u64; 4],
    pub assistance: Vec<Era1AssistanceKind>,
    pub adapter_name: String,
    pub backend_api: String,
    pub source_commit: String,
    pub source_tree: String,
}

impl Era1TrialReceipt {
    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        <Self as Validate>::validate_contract(self)
    }
}

impl Validate for Era1TrialReceipt {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.identity.validate_contract()?;
        if self.schema_version != ERA1_EVALUATION_SCHEMA_VERSION
            || self.phenotype_hash.0 == [0; 4]
            || self.foundation_id == 0
            || self.foundation_version == 0
            || self.sensor_profile != SensorProfile::GroundedObjectSlotsV1
            || self.policy_backend != PolicyBackend::NeuralClosedLoopGpu
            || self.world_digest == [0; 4]
            || self.perception_digest == [0; 4]
            || self.sealed_evidence_digest == [0; 4]
            || !valid_provenance_text(&self.adapter_name)
            || self.backend_api != "vulkan"
            || !valid_git_object_id(&self.source_commit)
            || !valid_git_object_id(&self.source_tree)
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        match self.score {
            MetricReading::Unknown => {}
            MetricReading::Measured {
                value_q16,
                exposures,
            } if value_q16 <= Q16_ONE && exposures > 0 => {}
            MetricReading::Measured { .. } => return Err(ScaffoldContractError::ScalarOutOfRange),
        }
        if self.partition == Era1EvidencePartition::ReproducedOffspring
            && self.identity.generation == 0
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        let unique_assistance = self.assistance.iter().copied().collect::<BTreeSet<_>>();
        if unique_assistance.len() != self.assistance.len()
            || self.assistance.contains(&Era1AssistanceKind::Possession)
            || (self.partition.requires_unassisted_evidence() && !self.assistance.is_empty())
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1MatchedComparison {
    pub ability: Era1Ability,
    pub control: Era1Control,
    pub intact_mean_q16: u32,
    pub control_mean_q16: u32,
    pub margin_q16: i32,
    pub matched_cells: u16,
}

impl Validate for Era1MatchedComparison {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        let expected_margin = i64::from(self.intact_mean_q16) - i64::from(self.control_mean_q16);
        if self.control == Era1Control::Intact
            || self.intact_mean_q16 > Q16_ONE
            || self.control_mean_q16 > Q16_ONE
            || self.matched_cells == 0
            || i64::from(self.margin_q16) != expected_margin
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1PlateauWindow {
    pub first_generation: u32,
    pub last_generation: u32,
    pub improvement_q16: i32,
    pub complete_cells: u32,
    pub ecological_regression: bool,
    pub diversity_regression: bool,
}

impl Validate for Era1PlateauWindow {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        let generation_count = self
            .last_generation
            .checked_sub(self.first_generation)
            .and_then(|span| span.checked_add(1))
            .ok_or(ScaffoldContractError::InvalidId)?;
        if generation_count < 3
            || self.complete_cells == 0
            || self.improvement_q16.unsigned_abs() > Q16_ONE
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
}

fn valid_provenance_text(value: &str) -> bool {
    !value.trim().is_empty() && value.chars().count() <= MAX_PROVENANCE_TEXT_CHARS
}

fn valid_git_object_id(value: &str) -> bool {
    value.len() == GIT_OBJECT_HEX_LEN
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
