//! v0 scaffold: central schema and ABI version registry.

use serde::{Deserialize, Serialize};

use crate::ScaffoldContractError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SchemaKind {
    Chemistry,
    SensoryAbi,
    ActionAbi,
    Experience,
    Perception,
    Phenotype,
    PackedLog,
    Genome,
    NeuralProjection,
    SleepConsolidation,
    Save,
    TeacherSchool,
    LineageExport,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContractVersion(pub u16);

impl ContractVersion {
    pub const V1: Self = Self(1);
    pub const V2: Self = Self(2);

    pub const fn raw(self) -> u16 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaVersions {
    pub chemistry: ContractVersion,
    pub sensory_abi: ContractVersion,
    pub action_abi: ContractVersion,
    pub experience: ContractVersion,
    pub perception: ContractVersion,
    pub phenotype: ContractVersion,
    pub packed_log: ContractVersion,
    pub genome: ContractVersion,
    pub neural_projection: ContractVersion,
    pub sleep_consolidation: ContractVersion,
    pub save: ContractVersion,
    pub teacher_school: ContractVersion,
    pub lineage_export: ContractVersion,
}

impl SchemaVersions {
    pub const CURRENT: Self = Self {
        chemistry: ContractVersion::V1,
        sensory_abi: ContractVersion::V1,
        action_abi: ContractVersion::V2,
        experience: ContractVersion::V2,
        perception: ContractVersion::V1,
        phenotype: ContractVersion::V1,
        packed_log: ContractVersion::V1,
        genome: ContractVersion::V1,
        neural_projection: ContractVersion::V1,
        sleep_consolidation: ContractVersion::V1,
        save: ContractVersion::V1,
        teacher_school: ContractVersion::V1,
        lineage_export: ContractVersion::V1,
    };

    pub const fn current_for(kind: SchemaKind) -> ContractVersion {
        match kind {
            SchemaKind::Chemistry => Self::CURRENT.chemistry,
            SchemaKind::SensoryAbi => Self::CURRENT.sensory_abi,
            SchemaKind::ActionAbi => Self::CURRENT.action_abi,
            SchemaKind::Experience => Self::CURRENT.experience,
            SchemaKind::Perception => Self::CURRENT.perception,
            SchemaKind::Phenotype => Self::CURRENT.phenotype,
            SchemaKind::PackedLog => Self::CURRENT.packed_log,
            SchemaKind::Genome => Self::CURRENT.genome,
            SchemaKind::NeuralProjection => Self::CURRENT.neural_projection,
            SchemaKind::SleepConsolidation => Self::CURRENT.sleep_consolidation,
            SchemaKind::Save => Self::CURRENT.save,
            SchemaKind::TeacherSchool => Self::CURRENT.teacher_school,
            SchemaKind::LineageExport => Self::CURRENT.lineage_export,
        }
    }
}

pub fn require_current_version(kind: SchemaKind, actual: u16) -> Result<(), ScaffoldContractError> {
    let expected = SchemaVersions::current_for(kind).raw();
    if actual == expected {
        Ok(())
    } else {
        Err(ScaffoldContractError::IncompatibleAbi {
            kind,
            expected,
            actual,
        })
    }
}

pub fn require_version(
    kind: SchemaKind,
    expected: u16,
    actual: u16,
) -> Result<(), ScaffoldContractError> {
    if actual == expected {
        Ok(())
    } else {
        Err(ScaffoldContractError::IncompatibleAbi {
            kind,
            expected,
            actual,
        })
    }
}
