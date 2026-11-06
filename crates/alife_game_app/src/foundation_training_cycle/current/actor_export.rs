//! Read-only extraction of the exact admitted global actor, never a life restore.
use super::*;
use std::{fs, io::Write};

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn exact_actor_asset(handoff: &alife_training::FoundationTrainingHandoffV1) -> Result<Vec<u8>> {
    handoff.validate()?;
    let asset = handoff
        .compiler_inputs
        .n2048_candidate_asset()
        .ok_or("current actor has no N2048 foundation asset")?;
    // Do not rebuild/relabel an asset or bake acquired deltas into its weights.
    // Both length and bits matter: float equality would erase signed zero.
    if asset.weights().len() != handoff.actor.weights.len()
        || asset
            .weights()
            .iter()
            .zip(&handoff.actor.weights)
            .any(|(a, b)| a.to_bits() != b.to_bits())
    {
        return Err("current actor weights differ from its foundation asset".into());
    }
    let bytes = asset.encode_canonical()?;
    let decoded = FoundationWeightAsset::decode_canonical(&bytes)?;
    if decoded.digest() != asset.digest()
        || decoded.encode_canonical()? != bytes
        || decoded
            .weights()
            .iter()
            .zip(&handoff.actor.weights)
            .any(|(a, b)| a.to_bits() != b.to_bits())
    {
        return Err("exported actor asset does not preserve exact identity and weight bits".into());
    }
    Ok(bytes)
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

/// Extract only a canonical actor asset and binding receipt to a new directory.
/// Full native Current/private admission runs first, without a device. The source
/// is never written, and partial output remains unsealed for explicit inspection.
/// Optimizers, acquired deltas, memories, body, world and prior state stay solely
/// in Current: this output supports a fresh-founder policy probe, not continuity.
pub fn export_current_actor_asset(source: &Path, output: &Path) -> Result<serde_json::Value> {
    let metadata = fs::symlink_metadata(source)?;
    if !metadata.is_file() || metadata.len() > alife_training::MAX_CURRENT_CHECKPOINT_BYTES {
        return Err("actor export requires a bounded regular Current file".into());
    }
    let source = source.canonicalize()?;
    let output = alife_training::resolve_isolated_training_output(output, [source.as_path()])?;
    if output.exists() {
        return Err("actor export output must be a new directory".into());
    }
    let original = fs::read(&source)?;
    let checkpoint =
        admit_current_training_checkpoint(CurrentTrainingCheckpointV1::decode(&original)?)?;
    let bytes = exact_actor_asset(&checkpoint.handoff)?;
    let asset = FoundationWeightAsset::decode_canonical(&bytes)?;
    let mut weight_bits = Sha256::new();
    for weight in &checkpoint.handoff.actor.weights {
        weight_bits.update(weight.to_bits().to_le_bytes());
    }
    let receipt = serde_json::json!({
        "schema":1,"kind":"current-actor-asset-export-v1",
        "source_current_sha256":sha(&original),
        "source_actor_sha256":sha(&serde_json::to_vec(&checkpoint.handoff.actor)?),
        "source_compiler_inputs_sha256":sha(&serde_json::to_vec(&checkpoint.handoff.compiler_inputs)?),
        "source_continuation":checkpoint.handoff.continuation,
        "source_phenotype_hash":checkpoint.handoff.actor.phenotype_hash,
        "policy_version":checkpoint.handoff.policy_version,
        "seed":checkpoint.seed,"founder_seed_base":checkpoint.founder_seed_base,
        "sensor_profile":asset.manifest().sensor_profile(),
        "asset_file":"actor.alife-foundation","asset_sha256":sha(&bytes),
        "asset_digest":asset.digest(),"asset_bytes":bytes.len(),
        "weight_count":checkpoint.handoff.actor.weights.len(),
        "weight_bits_sha256":format!("{:x}",weight_bits.finalize()),
        "weight_bits_encoding":"canonical actor order; f32 bits as little-endian u32",
        "all_actor_weight_bits_verified":true,"source_unchanged":true,
        "neural_execution":false,"promotion":"none",
        "scope":"exact global actor only; fresh-founder probe, not same-individual continuation",
        "private_state_exported":false,"optimizer_state_exported":false
    });
    if fs::read(&source)? != original {
        return Err("Current changed during actor export admission".into());
    }
    fs::create_dir_all(output.parent().ok_or("actor export output has no parent")?)?;
    fs::create_dir(&output)?;
    let asset_path = output.join("actor.alife-foundation");
    write_new(&asset_path, &bytes)?;
    if fs::read(asset_path)? != bytes || fs::read(&source)? != original {
        return Err("actor export/source changed; output remains unsealed".into());
    }
    // Last write commits the package. Existing or failed output is never reused.
    write_new(
        &output.join("actor-export.json"),
        &serde_json::to_vec_pretty(&receipt)?,
    )?;
    Ok(receipt)
}

#[cfg(test)]
mod tests;
