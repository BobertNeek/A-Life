//! v0 scaffold: stable IDs shared across A-Life contracts.

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Pod, Zeroable)]
pub struct BrainClassId(pub u16);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Pod, Zeroable)]
pub struct GenomeId(pub u64);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Pod, Zeroable)]
pub struct CreatureId(pub u64);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Pod, Zeroable)]
pub struct OrganismId(pub u64);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Pod, Zeroable)]
pub struct WorldEntityId(pub u64);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Pod, Zeroable)]
pub struct LineageId(pub u64);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Pod, Zeroable)]
pub struct GaussianClusterId(pub u64);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Pod, Zeroable)]
pub struct ConceptCellId(pub u64);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Pod, Zeroable)]
pub struct MemoryId(pub u64);

#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Pod, Zeroable,
)]
pub struct TrackedObjectId(pub u64);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Pod, Zeroable)]
pub struct ActionId(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Pod, Zeroable)]
pub struct ExperienceSequenceId(pub u64);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Pod, Zeroable)]
pub struct NeuronIndex(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Pod, Zeroable)]
pub struct LobeIndex(pub u16);

macro_rules! impl_nonzero_id {
    ($id:ty) => {
        impl $id {
            pub const INVALID: Self = Self(0);

            pub const fn new(raw: u64) -> Option<Self> {
                if raw == 0 {
                    None
                } else {
                    Some(Self(raw))
                }
            }

            pub const fn raw(self) -> u64 {
                self.0
            }

            pub const fn is_valid(self) -> bool {
                self.0 != 0
            }

            pub fn validate(self) -> Result<Self, crate::ScaffoldContractError> {
                if self.is_valid() {
                    Ok(self)
                } else {
                    Err(crate::ScaffoldContractError::InvalidId)
                }
            }
        }
    };
}

impl_nonzero_id!(GenomeId);
impl_nonzero_id!(CreatureId);
impl_nonzero_id!(OrganismId);
impl_nonzero_id!(WorldEntityId);
impl_nonzero_id!(LineageId);
impl_nonzero_id!(GaussianClusterId);
impl_nonzero_id!(ConceptCellId);
impl_nonzero_id!(MemoryId);
impl_nonzero_id!(TrackedObjectId);
impl_nonzero_id!(ExperienceSequenceId);

impl BrainClassId {
    pub const INVALID: Self = Self(0);

    pub const fn new(raw: u16) -> Option<Self> {
        if raw == 0 {
            None
        } else {
            Some(Self(raw))
        }
    }

    pub const fn raw(self) -> u16 {
        self.0
    }

    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }

    pub fn validate(self) -> Result<Self, crate::ScaffoldContractError> {
        if self.is_valid() {
            Ok(self)
        } else {
            Err(crate::ScaffoldContractError::InvalidId)
        }
    }
}

impl ActionId {
    pub const INVALID: Self = Self(0);

    pub const fn new(raw: u32) -> Option<Self> {
        if raw == 0 {
            None
        } else {
            Some(Self(raw))
        }
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }

    pub fn validate(self) -> Result<Self, crate::ScaffoldContractError> {
        if self.is_valid() {
            Ok(self)
        } else {
            Err(crate::ScaffoldContractError::InvalidId)
        }
    }
}

impl NeuronIndex {
    pub const fn raw(self) -> u32 {
        self.0
    }
}

impl LobeIndex {
    pub const fn raw(self) -> u16 {
        self.0
    }
}

impl From<CreatureId> for OrganismId {
    fn from(value: CreatureId) -> Self {
        Self(value.0)
    }
}

impl From<OrganismId> for CreatureId {
    fn from(value: OrganismId) -> Self {
        Self(value.0)
    }
}

pub fn validate_optional_target(
    target: Option<WorldEntityId>,
) -> Result<Option<WorldEntityId>, crate::ScaffoldContractError> {
    if let Some(id) = target {
        id.validate().map(Some)
    } else {
        Ok(None)
    }
}
