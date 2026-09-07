//! Explicit, unpromoted fixed-graph candidate admission. Old builtin inputs
//! retain their existing interpretation and never resolve through this path.

use crate::{FoundationWeightAsset, ScaffoldContractError};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Nano512ReadoutCandidateV1 {
    canonical_asset: Vec<u8>,
    #[serde(skip)]
    profile: crate::SensorProfile,
    #[serde(skip)]
    weight_asset: crate::FoundationWeightAssetRef,
}

impl Nano512ReadoutCandidateV1 {
    pub fn new(asset: &FoundationWeightAsset) -> Result<Self, ScaffoldContractError> {
        if !asset.is_nano512_readout_candidate() {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(Self {
            canonical_asset: asset.encode_canonical()?,
            profile: asset.manifest().sensor_profile(),
            weight_asset: asset.asset_ref(),
        })
    }

    pub fn asset(&self) -> Result<FoundationWeightAsset, ScaffoldContractError> {
        let asset = FoundationWeightAsset::decode_canonical(&self.canonical_asset)?;
        if !asset.is_nano512_readout_candidate() {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(asset)
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.canonical_asset
    }
    pub const fn sensor_profile(&self) -> crate::SensorProfile {
        self.profile
    }
    pub const fn asset_ref(&self) -> crate::FoundationWeightAssetRef {
        self.weight_asset
    }
}

impl<'de> Deserialize<'de> for Nano512ReadoutCandidateV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Wire {
            canonical_asset: Vec<u8>,
        }
        let wire = Wire::deserialize(deserializer)?;
        let asset = FoundationWeightAsset::decode_canonical(&wire.canonical_asset)
            .map_err(D::Error::custom)?;
        Self::new(&asset).map_err(D::Error::custom)
    }
}
