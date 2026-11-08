//! Separate fresh grip interface/reader identity; never an acquired-life migration.
use crate::{
    BrainCapacityClass, CanonicalDigestBuilder, FoundationWeightAsset, FoundationWeightAssetRef,
    ScaffoldContractError, SensorProfile,
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct N2048GripReaderCandidateV1 {
    schema_version: u16,
    sensor_profile: SensorProfile,
    recipe_digest: [u64; 4],
    foundation_bytes: Option<Vec<u8>>,
}
impl N2048GripReaderCandidateV1 {
    pub const MAX_CANDIDATE_GAIN: f32 = 12.0;
    /// A new variant-specific port in the pre-existing 54-float replay row.
    pub const OPERATION_VALUE_LANE: u16 = 53;
    pub const RETARGETED_DUPLICATES_PER_FAMILY: u16 = 8;
    pub fn fresh(sensor_profile: SensorProfile) -> Result<Self, ScaffoldContractError> {
        if sensor_profile != SensorProfile::GroundedGripObjectSlotsV1 {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(Self {
            schema_version: 1,
            sensor_profile,
            recipe_digest: Self::recipe_digest(),
            foundation_bytes: None,
        })
    }
    pub fn for_asset(asset: &FoundationWeightAsset) -> Result<Self, ScaffoldContractError> {
        if !asset.is_n2048_grip_reader_candidate() {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        let mut value = Self::fresh(asset.manifest().sensor_profile())?;
        value.foundation_bytes = Some(asset.encode_canonical()?);
        Ok(value)
    }
    pub fn recipe_digest() -> [u64; 4] {
        let mut d = CanonicalDigestBuilder::new(b"alife.n2048.grip-reader-candidate.v1");
        for word in crate::N2048ExperimentalGraphV1::recipe_digest() {
            d.write_u64(word);
        }
        d.write_u16(SensorProfile::GroundedGripObjectSlotsV1.raw());
        d.write_u32(crate::GripOperationV1::Close.action_id().raw());
        d.write_u32(crate::GripOperationV1::Release.action_id().raw());
        d.write_bytes(b"read-ingest-store-operation.v1");
        d.write_bytes(b"body-selfheld-vs-notselfheld.v1");
        d.write_u16(3);
        d.write_u16(Self::OPERATION_VALUE_LANE);
        d.write_u16(Self::RETARGETED_DUPLICATES_PER_FAMILY);
        d.write_bytes(b"retarget-memory-source55-channel7-to53.v1");
        d.write_f32(Self::MAX_CANDIDATE_GAIN)
            .expect("fixed finite gain");
        d.finish256()
    }
    pub const fn sensor_profile(&self) -> SensorProfile {
        self.sensor_profile
    }
    pub fn asset(&self) -> Result<Option<FoundationWeightAsset>, ScaffoldContractError> {
        self.foundation_bytes
            .as_deref()
            .map(FoundationWeightAsset::decode_canonical)
            .transpose()
    }
    pub fn asset_ref(&self) -> Option<FoundationWeightAssetRef> {
        self.asset().ok().flatten().map(|asset| asset.asset_ref())
    }
    pub(crate) fn validate_against(
        &self,
        capacity: &BrainCapacityClass,
        profile: SensorProfile,
    ) -> Result<(), ScaffoldContractError> {
        if self.schema_version != 1
            || capacity.id() != BrainCapacityClass::N2048_ID
            || profile != SensorProfile::GroundedGripObjectSlotsV1
            || self.sensor_profile != profile
            || self.recipe_digest != Self::recipe_digest()
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        SensorProfile::try_from_raw(profile.raw())?;
        if let Some(asset) = self.asset()? {
            if !asset.is_n2048_grip_reader_candidate()
                || asset.manifest().sensor_profile() != profile
            {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
        }
        Ok(())
    }
    pub(crate) fn write_canonical(&self, d: &mut CanonicalDigestBuilder) {
        d.write_u16(self.schema_version);
        d.write_u16(self.sensor_profile.raw());
        for word in self.recipe_digest {
            d.write_u64(word);
        }
        match &self.foundation_bytes {
            Some(bytes) => {
                d.write_some();
                d.write_sequence_len(bytes.len());
                d.write_bytes(bytes);
            }
            None => d.write_none(),
        }
    }
}
