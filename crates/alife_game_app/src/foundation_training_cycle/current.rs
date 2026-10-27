//! Admission/migration for the one durable fresh-cohort checkpoint.
use super::*;
use alife_training::{CheckpointSeedsV1, CurrentTrainingCheckpointV1};
use sha2::{Digest, Sha256};
mod life;
pub use life::extract_current_training_life;
#[cfg(test)]
pub(super) fn validation_workspace_for_test(root: &Path) -> Result<impl AsRef<Path>> {
    life::workspace_at(root)
}

pub fn resolve_current_training_destination(
    source: &Path,
    output: &Path,
) -> Result<std::path::PathBuf> {
    let mut roots = portable_handoff::protected_sources(source)?;
    let assets = roots
        .iter()
        .map(|root| root.join("world-assets"))
        .filter(|p| p.exists())
        .map(|p| p.canonicalize())
        .collect::<std::io::Result<Vec<_>>>()?;
    roots.extend(assets);
    Ok(alife_training::resolve_isolated_training_output(
        output,
        roots.iter().map(std::path::PathBuf::as_path),
    )?)
}

/// Device-free validation, including the ordinary cohort's compiler coordinates.
pub fn read_current_training_checkpoint(path: &Path) -> Result<CurrentTrainingCheckpointV1> {
    let checkpoint = CurrentTrainingCheckpointV1::read(path)?;
    life::validate(
        checkpoint
            .life
            .as_ref()
            .ok_or("native current checkpoint lacks private/lifetime recovery")?,
    )?;
    let handoff = checkpoint.handoff.clone().admit()?;
    let phenotype = crate::foundation_archived_success_replay::compile_source(
        handoff.foundation.clone(),
        checkpoint.seed,
        checkpoint.founder_seed_base,
    )?;
    handoff
        .actor
        .validate_for(&phenotype, handoff.foundation.digest())?;
    if phenotype.phenotype_hash() != handoff.phenotype.phenotype_hash() {
        return Err("current checkpoint compiler differs from the native cohort".into());
    }
    Ok(checkpoint)
}

/// Migrate only through the existing full legacy continuity verifier, including
/// terrain/corrected-critic proof. The resulting checkpoint has no source paths.
pub fn migrate_current_training_checkpoint(
    source: &Path,
    seeds: CheckpointSeedsV1,
) -> Result<CurrentTrainingCheckpointV1> {
    read_native_current(source, seeds).map(|(checkpoint, _)| checkpoint)
}
fn read_native_current(
    source: &Path,
    seeds: CheckpointSeedsV1,
) -> Result<(CurrentTrainingCheckpointV1, FoundationCycleReceipt)> {
    let portable_handoff::VerifiedSerialHandoff {
        handoff, receipt, ..
    } = portable_handoff::verified_serial_handoff(source)?;
    let checkpoint = CurrentTrainingCheckpointV1 {
        schema_version: 1,
        seed: receipt.seed,
        founder_seed_base: if receipt.founder_seed_base == 0 {
            receipt.seed
        } else {
            receipt.founder_seed_base
        },
        seeds,
        handoff,
        life: Some(life::capture(source)?),
    };
    checkpoint.validate()?;
    Ok((checkpoint, receipt))
}

/// Check a completed prospective episode against the exact accepted predecessor.
pub fn advance_current_training_checkpoint(
    previous: &CurrentTrainingCheckpointV1,
    source: &Path,
) -> Result<CurrentTrainingCheckpointV1> {
    previous.validate()?;
    let (next, receipt) = read_native_current(source, previous.seeds.clone())?;
    let asset = previous
        .handoff
        .compiler_inputs
        .n2048_candidate_asset()
        .ok_or("current asset missing")?;
    if next.seed == previous.seed
        || next.founder_seed_base != previous.founder_seed_base
        || next.handoff.policy_version
            != previous
                .handoff
                .policy_version
                .checked_add(1)
                .ok_or("policy overflow")?
        || next
            .handoff
            .actor
            .optimizer_step
            .checked_sub(previous.handoff.actor.optimizer_step)
            != Some(receipt.actor_optimizer_updates)
        || next
            .handoff
            .value
            .optimizer_step
            .checked_sub(previous.handoff.value.optimizer_step)
            != Some(receipt.value_optimizer_updates)
        || std::fs::read(source.join("initial.alife-foundation"))? != asset.encode_canonical()?
    {
        return Err("episode does not continue the exact current brain and optimizer ages".into());
    }
    Ok(next)
}

pub fn inspect_current_training_checkpoint(path: &Path) -> Result<serde_json::Value> {
    let checkpoint = read_current_training_checkpoint(path)?;
    let asset = checkpoint
        .handoff
        .compiler_inputs
        .n2048_candidate_asset()
        .ok_or("current asset missing")?;
    Ok(serde_json::json!({
        "schema":1, "kind":"single-current-training-checkpoint", "seed":checkpoint.seed,
        "founder_seed_base":checkpoint.founder_seed_base,
        "policy_version":checkpoint.handoff.policy_version,
        "actor_optimizer_step":checkpoint.handoff.actor.optimizer_step,
        "value_optimizer_step":checkpoint.handoff.value.optimizer_step,
        "new_asset_digest":digest(asset),
        "asset_sha256":format!("{:x}", Sha256::digest(asset.encode_canonical()?)),
        "seeds":{"historical_freshness":checkpoint.seeds.historical_freshness,
            "pending_range":checkpoint.seeds.pending_range,
            "excluded_range_count":checkpoint.seeds.excluded_ranges.len()},
        "neural_execution":false, "weights_modified":false,
        "external_dependencies":[]
    }))
}
