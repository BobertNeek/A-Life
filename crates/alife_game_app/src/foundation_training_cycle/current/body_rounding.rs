//! Source-bound, opt-in portability admission preserving every saved body bit.
use super::*;
use alife_world::SavedBodyRoundingCompatibilityV1;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurrentBodyRoundingCompatibilityV1 {
    pub source_current_sha256: String,
    pub proof: SavedBodyRoundingCompatibilityV1,
}

fn bytes_at(path: &Path, expected: &str) -> Result<Vec<u8>> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.len() > alife_training::MAX_CURRENT_CHECKPOINT_BYTES {
        return Err("compatibility source must be a bounded regular Current".into());
    }
    let bytes = std::fs::read(path)?;
    if format!("{:x}", Sha256::digest(&bytes)) != expected {
        return Err("body compatibility source differs from selected raw SHA256".into());
    }
    Ok(bytes)
}

/// Produces only a proposal, not native admission. The envelope, source pin,
/// typed global heads and exact saved-versus-local phenotype are checked here;
/// full private closure and same-individual admission must follow separately.
pub fn propose_current_body_rounding_compatibility(
    path: &Path,
    expected: &str,
) -> Result<CurrentBodyRoundingCompatibilityV1> {
    let bytes = bytes_at(path, expected)?;
    if bytes.len() < 56
        || &bytes[..16] != b"ALIFE-CURRENT-1\n"
        || u64::from_le_bytes(bytes[16..24].try_into()?) != (bytes.len() - 56) as u64
        || Sha256::digest(&bytes[56..]).as_slice() != &bytes[24..56]
    {
        return Err("Current envelope length or payload hash differs".into());
    }
    let current: CurrentTrainingCheckpointV1 = serde_json::from_slice(&bytes[56..])?;
    current.handoff.validate()?;
    current.seeds.validate()?;
    let life = current.life.as_ref().ok_or("Current lacks private life")?;
    let world: serde_json::Value = serde_json::from_str(&life.world_json)?;
    let records = world["world"]["organism_records"]
        .as_array()
        .ok_or("missing saved organism records")?;
    if records.len() != 1 {
        return Err(
            "body compatibility proposal requires exactly one actual saved organism".into(),
        );
    }
    let record: alife_world::WorldOrganismRecord = serde_json::from_value(records[0].clone())?;
    let proof = record.propose_saved_body_rounding_compatibility()?;
    if bytes_at(path, expected)? != bytes {
        return Err("source changed during compatibility proposal".into());
    }
    Ok(CurrentBodyRoundingCompatibilityV1 {
        source_current_sha256: expected.into(),
        proof,
    })
}

/// All existing native validators run under a lexical, exact-genome proof.
/// Only the archived metabolic exp2 rounding comparison has explicit alternate
/// provenance. The saved source, heads, private bytes and sampler are unchanged.
pub fn read_current_training_checkpoint_with_body_rounding(
    path: &Path,
    expected: &str,
    proof: &SavedBodyRoundingCompatibilityV1,
) -> Result<CurrentTrainingCheckpointV1> {
    let bytes = bytes_at(path, expected)?;
    let (current, used) =
        alife_world::with_saved_body_rounding_compatibility(proof.clone(), || {
            read_current_training_checkpoint(path)
        })?;
    let current = current?;
    if !used {
        return Err("declared source body rounding compatibility proof was not used".into());
    }
    if bytes_at(path, expected)? != bytes {
        return Err("source changed during compatible native admission".into());
    }
    Ok(current)
}
