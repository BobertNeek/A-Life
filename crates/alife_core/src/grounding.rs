//! Contract-only grounded sensor-profile provenance and object-slot records.

use serde::{Deserialize, Serialize};

use crate::{
    ensure_current_version, CandidateFeatureVector, Confidence, ScaffoldContractError, SchemaKind,
    SensorProfile, SensoryAbiVersion, Tick, TrackedObjectId, Validate,
};

pub const MAX_GROUNDED_OBJECT_SLOTS: usize = 16;
pub const GROUNDED_OBJECT_SLOT_SCHEMA_VERSION: u16 = 1;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SensorProfileId(pub u16);

impl SensorProfileId {
    pub const fn raw(self) -> u16 {
        self.0
    }
}

impl From<SensorProfile> for SensorProfileId {
    fn from(profile: SensorProfile) -> Self {
        Self(profile.raw())
    }
}

impl TryFrom<SensorProfileId> for SensorProfile {
    type Error = ScaffoldContractError;

    fn try_from(value: SensorProfileId) -> Result<Self, Self::Error> {
        Self::try_from_raw(value.raw())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SensorProfileIdentity {
    pub profile_id: SensorProfileId,
    pub profile_schema_version: u16,
    pub sensory_abi_version: u16,
}

impl SensorProfileIdentity {
    pub fn profile(self) -> Result<SensorProfile, ScaffoldContractError> {
        self.validate_contract()?;
        SensorProfile::try_from(self.profile_id)
    }
}

impl Validate for SensorProfileIdentity {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        ensure_current_version(SchemaKind::SensorProfile, self.profile_schema_version)
            .map_err(|_| ScaffoldContractError::SensorProfileMismatch)?;
        SensorProfile::try_from(self.profile_id)?;
        ensure_current_version(SchemaKind::SensoryAbi, self.sensory_abi_version)
            .map_err(|_| ScaffoldContractError::SensorProfileMismatch)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensorProfileProvenance {
    pub schema_version: u16,
    pub profile: SensorProfile,
    pub sensory_abi_version: SensoryAbiVersion,
    pub source_tick: Tick,
}

impl SensorProfileProvenance {
    pub fn new(
        profile: SensorProfile,
        sensory_abi_version: SensoryAbiVersion,
        source_tick: Tick,
    ) -> Result<Self, ScaffoldContractError> {
        let provenance = Self {
            schema_version: GROUNDED_OBJECT_SLOT_SCHEMA_VERSION,
            profile,
            sensory_abi_version,
            source_tick,
        };
        provenance.validate_contract()?;
        Ok(provenance)
    }

    pub fn identity(self) -> SensorProfileIdentity {
        SensorProfileIdentity {
            profile_id: self.profile.into(),
            profile_schema_version: self.schema_version,
            sensory_abi_version: self.sensory_abi_version.raw(),
        }
    }
}

impl Validate for SensorProfileProvenance {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.identity().validate_contract()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GroundedObjectSlotV1 {
    pub slot_index: u16,
    pub tracked_object_id: TrackedObjectId,
    pub bearing: [f32; 2],
    pub distance: f32,
    pub relative_velocity: [f32; 3],
    pub color: [f32; 3],
    pub material: [f32; 3],
    pub shape: [f32; 3],
    pub chemical: [f32; 3],
    pub contact: f32,
    pub proprioception: [f32; 2],
    pub temperature: f32,
    pub terrain: [f32; 2],
    pub confidence: Confidence,
}

impl GroundedObjectSlotV1 {
    pub fn candidate_features(self) -> Result<CandidateFeatureVector, ScaffoldContractError> {
        self.validate_contract()?;
        let features = CandidateFeatureVector([
            self.bearing[0],
            self.bearing[1],
            self.distance,
            self.relative_velocity[0],
            self.relative_velocity[1],
            self.relative_velocity[2],
            self.color[0],
            self.color[1],
            self.color[2],
            self.material[0],
            self.material[1],
            self.material[2],
            self.shape[0],
            self.shape[1],
            self.shape[2],
            self.chemical[0],
            self.chemical[1],
            self.chemical[2],
            self.contact,
            self.proprioception[0],
            self.proprioception[1],
            self.temperature,
            self.terrain[0],
            self.terrain[1],
        ]);
        features.validate()?;
        Ok(features)
    }
}

impl Validate for GroundedObjectSlotV1 {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.tracked_object_id
            .validate()
            .map_err(|_| ScaffoldContractError::InvalidPerceptionFrame)?;
        if usize::from(self.slot_index) >= MAX_GROUNDED_OBJECT_SLOTS
            || !signed_values_valid(&self.bearing)
            || !signed_values_valid(&self.relative_velocity)
            || !unit_values_valid(&self.color)
            || !unit_values_valid(&self.material)
            || !unit_values_valid(&self.shape)
            || !signed_values_valid(&self.chemical)
            || !unit_value_valid(self.contact)
            || !signed_values_valid(&self.proprioception)
            || !signed_value_valid(self.temperature)
            || !unit_values_valid(&self.terrain)
            || !unit_value_valid(self.distance)
        {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        Confidence::new(self.confidence.raw())
            .map_err(|_| ScaffoldContractError::InvalidPerceptionFrame)?;
        Ok(())
    }
}

fn signed_value_valid(value: f32) -> bool {
    value.is_finite() && (-1.0..=1.0).contains(&value)
}

fn unit_value_valid(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn signed_values_valid<const N: usize>(values: &[f32; N]) -> bool {
    values.iter().copied().all(signed_value_valid)
}

fn unit_values_valid<const N: usize>(values: &[f32; N]) -> bool {
    values.iter().copied().all(unit_value_valid)
}
