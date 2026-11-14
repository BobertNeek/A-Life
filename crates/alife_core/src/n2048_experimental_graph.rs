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
    /// Fresh opt-in anchored graph using an explicitly selected physical profile.
    /// Existing SlotsV1 is unchanged; terrain profiles add their existing sensing
    /// ports without replacing the anchored weight-coordinate recipe.
    pub fn fresh_anchored_v2(sensor_profile: SensorProfile) -> Result<Self, ScaffoldContractError> {
        if !Self::supports_anchored_profile(sensor_profile) {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        let mut v = Self::fresh(sensor_profile)?;
        v.recipe_digest = Self::anchored_recipe_digest();
        v.set_anchored_version();
        Ok(v)
    }
    /// Fresh opt-in identity: V2 coordinates with signed recurrent motor genes.
    pub fn fresh_signed_motor_v3(
        sensor_profile: SensorProfile,
    ) -> Result<Self, ScaffoldContractError> {
        // Signed-motor V3 remains its original SlotsV1-only contract.
        if sensor_profile != SensorProfile::GroundedObjectSlotsV1 {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        let mut value = Self::fresh_anchored_v2(sensor_profile)?;
        value.schema_version = 3;
        value.recipe_digest = Self::signed_motor_recipe_digest();
        Ok(value)
    }
    pub fn signed_motor_recipe_digest() -> [u64; 4] {
        let mut d = CanonicalDigestBuilder::new(b"alife.n2048.experimental-signed-motor.v3");
        for word in Self::anchored_recipe_digest() {
            d.write_u64(word);
        }
        d.write_u16(1); // SignedRecurrentMotorProposalV1, immutable policy.
        d.finish256()
    }
    pub const fn is_signed_motor_v3(&self) -> bool {
        self.schema_version == 3
    }
    pub(crate) const fn supports_anchored_profile(profile: SensorProfile) -> bool {
        matches!(
            profile,
            SensorProfile::GroundedObjectSlotsV1
                | SensorProfile::GroundedTerrainVisionV1
                | SensorProfile::GroundedTerrainObjectPortsV2
        )
    }
    pub const fn uses_anchored_sensor_layout(&self) -> bool {
        self.is_anchored_v2() || self.is_signed_motor_v3()
    }
    pub fn anchored_recipe_digest() -> [u64; 4] {
        let mut d =
            CanonicalDigestBuilder::new(b"alife.n2048.experimental-anchored-sensor-graph.v2");
        for word in Self::recipe_digest() {
            d.write_u64(word);
        }
        d.write_u64(crate::phenotype::experimental_sensor_anchor::SENSOR_SEED);
        for &(g, l, n) in crate::phenotype::experimental_sensor_anchor::PORTS {
            d.write_u16(g);
            d.write_u16(l);
            d.write_u32(n);
        }
        d.finish256()
    }
    pub const fn is_anchored_v2(&self) -> bool {
        self.schema_version == 2
    }
    pub(crate) fn set_anchored_version(&mut self) {
        self.schema_version = 2;
    }
    pub fn for_asset(asset: &FoundationWeightAsset) -> Result<Self, ScaffoldContractError> {
        if !asset.is_n2048_experimental_graph() {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        let mut value = if asset.is_n2048_signed_motor_v3() {
            Self::fresh_signed_motor_v3(asset.manifest().sensor_profile())?
        } else if asset.is_n2048_anchored_v2() {
            Self::fresh_anchored_v2(asset.manifest().sensor_profile())?
        } else {
            Self::fresh(asset.manifest().sensor_profile())?
        };
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
    pub const fn selected_recipe_digest(&self) -> [u64; 4] {
        self.recipe_digest
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
        if !((self.schema_version == 1 && self.recipe_digest == Self::recipe_digest())
            || (self.schema_version == 2 && self.recipe_digest == Self::anchored_recipe_digest())
            || (self.schema_version == 3
                && self.recipe_digest == Self::signed_motor_recipe_digest()))
            || capacity.id() != BrainCapacityClass::N2048_ID
            || self.sensor_profile != profile
            || (self.is_anchored_v2() && !Self::supports_anchored_profile(profile))
            || (self.is_signed_motor_v3() && profile != SensorProfile::GroundedObjectSlotsV1)
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        SensorProfile::try_from_raw(profile.raw())?;
        if let Some(asset) = self.asset()? {
            if !asset.is_n2048_experimental_graph()
                || asset.manifest().sensor_profile() != profile
                || self.is_signed_motor_v3() != asset.is_n2048_signed_motor_v3()
                || self.is_anchored_v2() != asset.is_n2048_anchored_v2()
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

#[cfg(test)]
mod profile_transfer_tests {
    use super::*;
    #[test]
    fn anchored_profile_transfer_is_explicit_and_signed_motor_stays_slots_only() {
        let capacity = BrainCapacityClass::n2048();
        for profile in [
            SensorProfile::GroundedObjectSlotsV1,
            SensorProfile::GroundedTerrainVisionV1,
            SensorProfile::GroundedTerrainObjectPortsV2,
        ] {
            let graph = N2048ExperimentalGraphV1::fresh_anchored_v2(profile).unwrap();
            graph.validate_against(&capacity, profile).unwrap();
            assert_eq!(
                graph.selected_recipe_digest(),
                N2048ExperimentalGraphV1::anchored_recipe_digest()
            );
            assert!(graph
                .validate_against(&capacity, SensorProfile::PrivilegedAffordanceV1)
                .is_err());
            if profile != SensorProfile::GroundedObjectSlotsV1 {
                assert!(N2048ExperimentalGraphV1::fresh_signed_motor_v3(profile).is_err());
                let mut forged = N2048ExperimentalGraphV1::fresh_signed_motor_v3(
                    SensorProfile::GroundedObjectSlotsV1,
                )
                .unwrap();
                forged.sensor_profile = profile;
                assert!(forged.validate_against(&capacity, profile).is_err());
            }
        }
        assert!(
            N2048ExperimentalGraphV1::fresh_anchored_v2(SensorProfile::PrivilegedAffordanceV1)
                .is_err()
        );
    }
}
