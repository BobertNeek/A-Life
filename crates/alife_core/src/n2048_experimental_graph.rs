//! Explicit, fresh-founder research graph. Never a V1 coordinate migration.
use crate::{
    BrainCapacityClass, CanonicalDigestBuilder, FoundationWeightAsset, FoundationWeightAssetRef,
    ScaffoldContractError, SensorProfile,
};
use serde::{Deserialize, Serialize};

/// A sealed deterministic wiring recipe. Optional canonical bytes bind exact
/// genetic weights; acquired state and optimizer state are deliberately absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct N2048ExperimentalGraphV1 {
    schema_version: u16,
    sensor_profile: SensorProfile,
    recipe_digest: [u64; 4],
    foundation_bytes: Option<Vec<u8>>,
}
impl N2048ExperimentalGraphV1 {
    pub fn fresh(sensor_profile: SensorProfile) -> Result<Self, ScaffoldContractError> {
        SensorProfile::try_from_raw(sensor_profile.raw())?;
        Ok(Self {
            schema_version: 1,
            sensor_profile,
            recipe_digest: Self::recipe_digest(),
            foundation_bytes: None,
        })
    }
    pub fn for_asset(asset: &FoundationWeightAsset) -> Result<Self, ScaffoldContractError> {
        if !asset.is_n2048_experimental_graph() {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        let mut value = Self::fresh(asset.manifest().sensor_profile())?;
        value.foundation_bytes = Some(asset.encode_canonical()?);
        Ok(value)
    }
    pub fn recipe_digest() -> [u64; 4] {
        let mut d = CanonicalDigestBuilder::new(b"alife.n2048.experimental-grounded-graph.v1");
        // Recipe freezes the five hub/coverage routes, one anchor tile in every
        // other route, and balanced within-tile cyclic pairs. Not a graph flag.
        for value in [1_u32, 2048, 24576, 32768, 16, 0, 1, 2, 3, 5] {
            d.write_u32(value);
        }
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
            || self.sensor_profile != profile
            || self.recipe_digest != Self::recipe_digest()
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        SensorProfile::try_from_raw(profile.raw())?;
        if let Some(asset) = self.asset()? {
            if !asset.is_n2048_experimental_graph() || asset.manifest().sensor_profile() != profile
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
