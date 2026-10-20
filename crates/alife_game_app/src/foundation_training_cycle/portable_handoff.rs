//! Device-free export after the existing serial/correction continuity verifier.
use super::*;
use alife_training::{
    resolve_isolated_training_output, FoundationSerialHandoffBindingV1,
    FoundationTrainingBackendV1, FoundationTrainingHandoffV1, SERIAL_HANDOFF_BINDING_FILE,
    SERIAL_HANDOFF_FILE, SERIAL_HANDOFF_SOURCE_FILES,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn protected_sources(previous: &Path) -> Result<std::collections::BTreeSet<std::path::PathBuf>> {
    let mut roots = std::collections::BTreeSet::new();
    let mut pending = vec![previous.canonicalize()?];
    while let Some(root) = pending.pop() {
        if !roots.insert(root.clone()) {
            continue;
        }
        if roots.len() > 64 {
            return Err("native source dependency limit exceeded".into());
        }
        for name in ["critic-correction.json", "terrain-continuation.json"] {
            let path = root.join(name);
            if path.is_file() {
                let metadata = std::fs::metadata(&path)?;
                if metadata.len() > 1024 * 1024 {
                    return Err("native continuity marker exceeds byte limit".into());
                }
                let marker: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
                let source = marker["source_directory"]
                    .as_str()
                    .ok_or("native continuity marker lacks source directory")?;
                pending.push(Path::new(source).canonicalize()?);
            }
        }
    }
    Ok(roots)
}

/// Export a portable founder/optimizer boundary without running a world or GPU.
/// The source remains read-only, including all individual/correction evidence.
pub fn export_native_training_handoff(
    previous: &Path,
    output: &Path,
) -> Result<FoundationSerialHandoffBindingV1> {
    let previous = previous.canonicalize()?;
    let sources = protected_sources(&previous)?;
    let output =
        resolve_isolated_training_output(output, sources.iter().map(std::path::PathBuf::as_path))?;
    if output.exists() {
        return Err("handoff output must be a new directory".into());
    }
    let previous = previous.as_path();
    let output = output.as_path();
    let source_bytes = SERIAL_HANDOFF_SOURCE_FILES
        .into_iter()
        .map(|name| {
            let path = previous.join(name);
            let metadata = std::fs::symlink_metadata(&path)?;
            if !metadata.is_file() || metadata.len() > 64 * 1024 * 1024 {
                return Err("native handoff input is not a bounded regular file".into());
            }
            Ok((name, std::fs::read(path)?))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    // Includes terrain rebinding, residual evidence and full critic-correction
    // preservation/qualification proof when declared. Never repair/reset state.
    let admitted = load_cycle_continuation(previous, None)?;
    let receipt: FoundationCycleReceipt = serde_json::from_slice(&source_bytes["cycle.json"])?;
    if receipt.biological_objective_version != 2
        || receipt.critic_objective_version != 1
        || receipt.objective_state_reset
    {
        return Err("portable handoff requires biological objective2 and residual critic objective1 without reset".into());
    }
    let (phenotype, inputs) =
        crate::foundation_archived_success_replay::compile_source_with_inputs(
            admitted.asset.clone(),
            receipt.seed,
            admitted.founder_seed_base,
        )?;
    admitted
        .actor
        .validate_for(&phenotype, admitted.asset.digest())?;
    let handoff = FoundationTrainingHandoffV1::new(
        FoundationTrainingBackendV1::NativeGpu,
        receipt.policy_version,
        inputs,
        admitted.actor,
        admitted.value,
    )?;
    // A native correction can intentionally retain an older cycle's critic age.
    // The verified, typed checkpoint preserves the actual carried integer age.
    if handoff
        .compiler_inputs
        .n2048_candidate_asset()
        .ok_or("missing handoff asset")?
        .encode_canonical()?
        != source_bytes["trained.alife-foundation"]
        || serde_json::from_slice::<alife_training::FoundationTrainerCheckpoint>(
            &source_bytes["actor-checkpoint.json"],
        )? != handoff.actor
        || serde_json::from_slice::<alife_training::PpoValueHeadCheckpoint>(
            &source_bytes["value-checkpoint.json"],
        )? != handoff.value
    {
        return Err("native source changed during continuity admission".into());
    }
    let handoff_bytes = serde_json::to_vec_pretty(&handoff)?;
    let binding = FoundationSerialHandoffBindingV1 {
        schema_version: 1,
        native_serial_continuity_validated: true,
        source_sha256: source_bytes
            .iter()
            .map(|(name, bytes)| ((*name).to_owned(), sha(bytes)))
            .collect(),
        training_handoff_sha256: sha(&handoff_bytes),
    };
    binding.validate(&binding.source_sha256, &sha(&handoff_bytes))?;
    for (name, bytes) in &source_bytes {
        if std::fs::read(previous.join(name))? != *bytes {
            return Err("native source changed during portable export".into());
        }
    }
    std::fs::create_dir_all(output.parent().unwrap_or(Path::new(".")))?;
    std::fs::create_dir(output)?;
    for (name, bytes) in source_bytes {
        std::fs::write(output.join(name), bytes)?;
    }
    std::fs::write(output.join(SERIAL_HANDOFF_FILE), handoff_bytes)?;
    std::fs::write(output.join("phase.txt"), "next-cohort-admitted")?;
    // Last write commits this portable package; incomplete output is inadmissible.
    std::fs::write(
        output.join(SERIAL_HANDOFF_BINDING_FILE),
        serde_json::to_vec_pretty(&binding)?,
    )?;
    Ok(binding)
}

#[cfg(test)]
mod tests;
