//! Explicit, unpromoted fixed-graph candidate admission. Old builtin inputs
//! retain their existing interpretation and never resolve through this path.

use crate::{FoundationWeightAsset, ScaffoldContractError};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

/// Versioned receptor configuration over an unchanged, validated V1 weight file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Nano512ActionCreditCandidateV2 {
    source: Nano512ReadoutCandidateV1,
    action_profile: crate::ActionCandidateCreditProfileV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cognitive_channel_extension: Option<crate::CognitiveChannelExtensionV1>,
}

impl Nano512ActionCreditCandidateV2 {
    pub fn new(
        asset: &FoundationWeightAsset,
        action_profile: crate::ActionCandidateCreditProfileV1,
    ) -> Result<Self, ScaffoldContractError> {
        Ok(Self {
            source: Nano512ReadoutCandidateV1::new(asset)?,
            action_profile,
            cognitive_channel_extension: None,
        })
    }

    pub fn new_with_cognitive_extension(
        asset: &FoundationWeightAsset,
        extension: crate::CognitiveChannelExtensionV1,
    ) -> Result<Self, ScaffoldContractError> {
        let value = Self {
            source: Nano512ReadoutCandidateV1::new(asset)?,
            action_profile: extension.action_candidate_credit_profile(),
            cognitive_channel_extension: Some(extension),
        };
        value.validate_cognitive_extension()?;
        Ok(value)
    }
    pub fn asset(&self) -> Result<FoundationWeightAsset, ScaffoldContractError> {
        self.source.asset()
    }
    pub const fn source(&self) -> &Nano512ReadoutCandidateV1 {
        &self.source
    }
    pub const fn action_profile(&self) -> crate::ActionCandidateCreditProfileV1 {
        self.action_profile
    }
    pub const fn cognitive_channel_extension(&self) -> Option<&crate::CognitiveChannelExtensionV1> {
        self.cognitive_channel_extension.as_ref()
    }

    pub(crate) fn validate_cognitive_extension(&self) -> Result<(), ScaffoldContractError> {
        if let Some(extension) = self.cognitive_channel_extension {
            extension.validate_contract()?;
            if extension.source_identity() != self.source.genetic_identity()
                || extension.action_candidate_credit_profile() != self.action_profile
            {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Nano512ReadoutCandidateV1 {
    canonical_asset: Vec<u8>,
    #[serde(skip)]
    profile: crate::SensorProfile,
    #[serde(skip)]
    weight_asset: crate::FoundationWeightAssetRef,
    #[serde(skip)]
    genetic_identity: crate::FoundationGeneticIdentity,
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
            genetic_identity: crate::FoundationGeneticIdentity::new(
                asset.manifest().foundation_id().raw(),
                asset.manifest().foundation_version().raw() as u16,
                asset.manifest().compatibility_family_id().raw(),
                asset.manifest().capacity_class_id(),
            )?,
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
    pub(crate) const fn genetic_identity(&self) -> crate::FoundationGeneticIdentity {
        self.genetic_identity
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
