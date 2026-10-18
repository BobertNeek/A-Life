//! CPU-only admission of an explicitly receipted critic derivative. This proves
//! checkpoint continuity, never behavioral qualification or accepted PPO work.
use crate::foundation_training_cycle::{validate_cycle_optimizer_handoff, FoundationCycleReceipt};
use alife_core::FoundationWeightAsset;
use alife_training::{FoundationTrainerCheckpoint, PpoTrainingState, PpoValueHeadCheckpoint};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const MARKER: &str = "critic-correction.json";
pub(crate) const RESIDUAL_UPDATE: &str = "critic-residual-update.json";
const CORE: [&str; 4] = [
    "cycle.json",
    "trained.alife-foundation",
    "actor-checkpoint.json",
    "value-checkpoint.json",
];
const MAX_FILE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 512 * 1024 * 1024;
const MAX_FILES: usize = 4096;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Correction {
    schema_version: u16,
    purpose: String,
    created_utc: String,
    source_directory: PathBuf,
    source_file_sha256: BTreeMap<String, String>,
    target_value_checkpoint_sha256: String,
    source_policy_version: u64,
    actor_optimizer_step: u32,
    source_value_optimizer_step: u32,
    value_optimizer_step: u32,
    critic_only_updates: u32,
    optimizer_reset: bool,
    value_state_reset: bool,
    biological_objective_version: u16,
    cycle_receipt_unchanged: bool,
    #[serde(rename = "no_PPO_cycles_claimed")]
    no_ppo_cycles_claimed: bool,
    live_qualified: bool,
    qualification_evidence_sha256: BTreeMap<PathBuf, String>,
    correction_objective: String,
    source_recovery_manifest_sha256: String,
}

fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > limit {
        return Err(format!(
            "correction input is not a bounded regular file: {}",
            path.display()
        )
        .into());
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 != metadata.len() || bytes.len() as u64 > limit {
        return Err("correction input changed size while reading".into());
    }
    Ok(bytes)
}

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_sha(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn normalize_map(map: &BTreeMap<String, String>) -> Result<BTreeMap<String, String>> {
    if map.is_empty() || map.len() > MAX_FILES {
        return Err("invalid correction file count".into());
    }
    let mut normalized = BTreeMap::new();
    for (name, digest) in map {
        // Check both separators on every host; reject Windows prefixes and ADS too.
        let name = name.replace('\\', "/");
        if name.contains(':')
            || name
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
            || !valid_sha(digest)
            || normalized.insert(name, digest.clone()).is_some()
        {
            return Err("unsafe, duplicate or malformed correction file reference".into());
        }
    }
    Ok(normalized)
}

fn snapshot(root: &Path) -> Result<BTreeMap<String, String>> {
    fn visit(
        root: &Path,
        path: &Path,
        depth: usize,
        entries: &mut usize,
        total: &mut u64,
        map: &mut BTreeMap<String, String>,
    ) -> Result<()> {
        if depth > 32 || map.len() >= MAX_FILES {
            return Err("correction file tree exceeds bounds".into());
        }
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            *entries += 1;
            if *entries > MAX_FILES * 2 {
                return Err("correction file tree exceeds bounds".into());
            }
            if entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.contains('\\'))
            {
                return Err("correction file tree contains a literal backslash name".into());
            }
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                return Err("correction file tree contains a symlink".into());
            }
            if kind.is_dir() {
                visit(root, &entry.path(), depth + 1, entries, total, map)?;
            } else if kind.is_file() {
                let bytes = read_bounded(&entry.path(), MAX_FILE_BYTES)?;
                *total = total
                    .checked_add(bytes.len() as u64)
                    .ok_or("correction file size overflow")?;
                if *total > MAX_TOTAL_BYTES || map.len() >= MAX_FILES {
                    return Err("correction file tree exceeds bounds".into());
                }
                let name = entry
                    .path()
                    .strip_prefix(root)?
                    .to_str()
                    .ok_or("non-UTF8 correction path")?
                    .replace('\\', "/");
                if map.insert(name, sha(&bytes)).is_some() {
                    return Err("duplicate normalized correction file reference".into());
                }
            } else {
                return Err("correction file tree contains a special file".into());
            }
        }
        Ok(())
    }
    let mut map = BTreeMap::new();
    visit(root, root, 0, &mut 0, &mut 0, &mut map)?;
    normalize_map(&map)
}

fn pinned_read(root: &Path, name: &str, expected: &BTreeMap<String, String>) -> Result<Vec<u8>> {
    let bytes = read_bounded(&root.join(name), MAX_FILE_BYTES)?;
    if expected.get(name) != Some(&sha(&bytes)) {
        return Err("correction checkpoint changed during validation".into());
    }
    Ok(bytes)
}

fn validate_residual_update(directory: &Path, receipt: &FoundationCycleReceipt) -> Result<()> {
    if let Some(expected) = &receipt.critic_residual_update_sha256 {
        if !valid_sha(expected) {
            return Err("invalid critic residual update SHA256".into());
        }
        let bytes = read_bounded(&directory.join(RESIDUAL_UPDATE), MAX_FILE_BYTES)?;
        if sha(&bytes) != *expected {
            return Err("critic residual update SHA256 mismatch".into());
        }
    }
    Ok(())
}

/// Validate any declared residual evidence before either sealed-cycle route.
pub(crate) fn validate_cycle_optimizer_handoff_at(
    directory: &Path,
    receipt: &FoundationCycleReceipt,
    asset: &FoundationWeightAsset,
    actor: &FoundationTrainerCheckpoint,
    value: &PpoValueHeadCheckpoint,
) -> Result<()> {
    validate_residual_update(directory, receipt)?;
    let marker = directory.join(MARKER);
    match std::fs::symlink_metadata(&marker) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return validate_cycle_optimizer_handoff(receipt, asset, actor, value)
        }
        Err(error) => return Err(error.into()),
        Ok(_) => {}
    }
    let marker_bytes = read_bounded(&marker, 1024 * 1024)?;
    let correction: Correction = serde_json::from_slice(&marker_bytes)?;
    if correction.schema_version != 1
        || correction.purpose != "critic_only_correction_candidate_clone_validation"
        || correction.created_utc.is_empty()
        || correction.correction_objective.is_empty()
        || correction.optimizer_reset
        || correction.value_state_reset
        || correction.live_qualified
        || !correction.cycle_receipt_unchanged
        || !correction.no_ppo_cycles_claimed
        || !(1..=5000).contains(&correction.critic_only_updates)
        || correction.biological_objective_version != 2
        || receipt.biological_objective_version != 2
        || receipt.objective_state_reset
        || receipt.founder_seed_base == 0
        || !correction.source_directory.is_absolute()
    {
        return Err("invalid critic correction continuity claims".into());
    }
    let source = correction.source_directory.canonicalize()?;
    if source == directory.canonicalize()? || source.join(MARKER).try_exists()? {
        return Err("critic correction requires a distinct original sealed source".into());
    }
    let source_files = normalize_map(&correction.source_file_sha256)?;
    if CORE.iter().any(|name| !source_files.contains_key(*name))
        || source_files.contains_key(MARKER)
        || snapshot(&source)? != source_files
    {
        return Err("critic correction source preservation hash mismatch".into());
    }
    let mut target_files = source_files.clone();
    if !valid_sha(&correction.target_value_checkpoint_sha256)
        || source_files.get("value-checkpoint.json")
            == Some(&correction.target_value_checkpoint_sha256)
    {
        return Err("critic correction must change the value checkpoint".into());
    }
    target_files.insert(
        "value-checkpoint.json".into(),
        correction.target_value_checkpoint_sha256.clone(),
    );
    target_files.insert(MARKER.into(), sha(&marker_bytes));
    if snapshot(directory)? != target_files {
        return Err("critic correction changed state outside its value checkpoint".into());
    }

    let source_receipt: FoundationCycleReceipt =
        serde_json::from_slice(&pinned_read(&source, "cycle.json", &source_files)?)?;
    let source_asset = FoundationWeightAsset::decode_canonical(&pinned_read(
        &source,
        "trained.alife-foundation",
        &source_files,
    )?)?;
    let source_actor: FoundationTrainerCheckpoint = serde_json::from_slice(&pinned_read(
        &source,
        "actor-checkpoint.json",
        &source_files,
    )?)?;
    let source_value: PpoValueHeadCheckpoint = serde_json::from_slice(&pinned_read(
        &source,
        "value-checkpoint.json",
        &source_files,
    )?)?;
    validate_residual_update(&source, &source_receipt)?;
    validate_cycle_optimizer_handoff(&source_receipt, &source_asset, &source_actor, &source_value)?;
    let source_digest: String = source_asset
        .digest()
        .bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let phenotype = crate::foundation_archived_success_replay::compile_source(
        source_asset.clone(),
        source_receipt.seed,
        source_receipt.founder_seed_base,
    )?;
    source_actor.validate_for(&phenotype, source_asset.digest())?;
    PpoTrainingState::from_checkpoint(source_value.clone())?;
    PpoTrainingState::from_checkpoint(value.clone())?;
    let pinned_value: PpoValueHeadCheckpoint = serde_json::from_slice(&pinned_read(
        directory,
        "value-checkpoint.json",
        &target_files,
    )?)?;
    // Serialized typed state also distinguishes signed zero in Adam lanes.
    if serde_json::to_vec(&source_receipt)? != serde_json::to_vec(receipt)?
        || source_asset.digest() != asset.digest()
        || source_digest != receipt.new_asset_digest
        || serde_json::to_vec(&source_actor)? != serde_json::to_vec(actor)?
        || serde_json::to_vec(&pinned_value)? != serde_json::to_vec(value)?
        || source_value.feature_count != phenotype.neuron_count()
        || value.feature_count != source_value.feature_count
        || value.last_updated_policy_version != source_value.last_updated_policy_version
        || value.last_updated_policy_version != Some(receipt.policy_version)
        || correction.source_policy_version != receipt.policy_version
        || correction.actor_optimizer_step != actor.optimizer_step
        || correction.source_value_optimizer_step != source_value.optimizer_step
        || correction.value_optimizer_step != value.optimizer_step
        || source_value
            .optimizer_step
            .checked_add(correction.critic_only_updates)
            != Some(value.optimizer_step)
    {
        return Err(
            "critic correction checkpoint identity, optimizer age or feature mismatch".into(),
        );
    }

    let manifest_path = source
        .parent()
        .ok_or("correction source has no parent")?
        .join("RECOVERY-PRESERVED.json");
    let manifest_bytes = read_bounded(&manifest_path, 1024 * 1024)?;
    if !valid_sha(&correction.source_recovery_manifest_sha256)
        || sha(&manifest_bytes) != correction.source_recovery_manifest_sha256
    {
        return Err("critic correction recovery manifest hash mismatch".into());
    }
    let manifest: serde_json::Value = serde_json::from_slice(&manifest_bytes)?;
    let manifest_source = PathBuf::from(
        manifest["latest_clone"]
            .as_str()
            .ok_or("missing preserved source directory")?,
    );
    let manifest_files = serde_json::from_value(manifest["latest_all_file_sha256"].clone())?;
    if manifest_source.canonicalize()? != source || normalize_map(&manifest_files)? != source_files
    {
        return Err("critic correction recovery manifest identifies another source".into());
    }
    if correction.qualification_evidence_sha256.is_empty()
        || correction.qualification_evidence_sha256.len() > 64
    {
        return Err("critic correction requires bounded, hashed evidence".into());
    }
    for (path, digest) in &correction.qualification_evidence_sha256 {
        if !path.is_absolute()
            || !valid_sha(digest)
            || sha(&read_bounded(path, 16 * 1024 * 1024)?) != *digest
        {
            return Err("critic correction evidence hash mismatch".into());
        }
    }
    if snapshot(&source)? != source_files
        || snapshot(directory)? != target_files
        || read_bounded(&manifest_path, 1024 * 1024)? != manifest_bytes
    {
        return Err("critic correction inputs changed during validation".into());
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests;
