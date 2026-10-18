//! Read-only comparison evidence. Never replace the ordinary world identity.

use alife_core::{Blake3Digest, FoundationWeightAsset};
use alife_world::{HeadlessPolicyWeightComparisonSignatureV1, HeadlessWorld};
use sha2::{Digest, Sha256};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoundationProbeComparisonIdentityV1 {
    pub schema_version: u16,
    pub exclusion: String,
    pub world: HeadlessPolicyWeightComparisonSignatureV1,
    pub policy_asset_digest: Blake3Digest,
    pub policy_sha256: String,
    pub executable_sha256: String,
}

pub(super) fn initial_identity(
    world: &HeadlessWorld,
    asset: &FoundationWeightAsset,
) -> super::Result<FoundationProbeComparisonIdentityV1> {
    let subject = world
        .organism_entity_ids()
        .first()
        .map(|(id, _)| *id)
        .ok_or_else(super::invalid)?;
    let admitted_asset = world
        .organism_registry()
        .get(subject)
        .and_then(|record| record.genome().n2048_foundation_candidate.as_ref())
        .ok_or_else(super::invalid)?;
    if admitted_asset != asset {
        return Err(super::invalid().into());
    }
    let encoded_asset = asset.encode_canonical()?;
    let signature = world.policy_weight_comparison_signature_v1(subject)?;
    Ok(FoundationProbeComparisonIdentityV1 {
        schema_version: 1,
        exclusion: "subject_n2048_policy_weights_v1".into(),
        world: signature,
        policy_asset_digest: asset.digest(),
        policy_sha256: format!("{:x}", Sha256::digest(&encoded_asset)),
        executable_sha256: format!(
            "{:x}",
            Sha256::digest(std::fs::read(std::env::current_exe()?)?)
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_identity_binds_actual_policy_and_executable_before_runtime() {
        let asset = crate::initial_n2048_care_asset(539_363_617).unwrap();
        let mut config = alife_world::CanonicalNewGameConfig::phase3(7001, 1).unwrap();
        config.brain_class = alife_core::BrainScaleTier::Standard2048;
        config.founder_seed_base = 539_363_617;
        config.sensor_profile = asset.manifest().sensor_profile();
        let game =
            alife_world::create_canonical_new_game_with_n2048_candidate(&config, &asset).unwrap();
        let before = game.world.canonical_signature_digest().unwrap();
        let identity = initial_identity(&game.world, &asset).unwrap();
        assert_eq!(identity.policy_asset_digest, asset.digest());
        assert_eq!(
            identity.policy_sha256,
            format!("{:x}", Sha256::digest(asset.encode_canonical().unwrap()))
        );
        assert_eq!(
            identity.executable_sha256,
            format!(
                "{:x}",
                Sha256::digest(std::fs::read(std::env::current_exe().unwrap()).unwrap())
            )
        );
        assert_eq!(game.world.canonical_signature_digest().unwrap(), before);
        let other = crate::initial_n2048_care_asset(539_363_618).unwrap();
        assert!(initial_identity(&game.world, &other).is_err());
    }
}
