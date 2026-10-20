//! Exact, reusable handoff for this runner. Native cycles keep their existing
//! admission path; simulator continuation never impersonates a native cycle.
use super::*;
use std::collections::BTreeMap;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Continuation {
    schema_version: u16,
    policy_version: u64,
    founder_seed_base: u64,
    biological_objective_version: u16,
    critic_objective_version: u16,
    actor_optimizer_step: u32,
    value_optimizer_step: u32,
    next_cohort_optimizer_rebound: bool,
    next_cohort_decision_checked: bool,
    optimizer_reset: bool,
    files: BTreeMap<String, [u8; 32]>,
}

const FILES: [&str; 4] = [
    "trained.alife-foundation",
    "actor-checkpoint.json",
    "value-checkpoint.json",
    "critic-residual-update.json",
];
fn read_bounded(path: &Path) -> Result<Vec<u8>> {
    use std::io::Read;
    let file = std::fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.take(64 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("continuation file exceeds 64 MiB".into());
    }
    Ok(bytes)
}

pub(super) fn inspect(
    source: &Path,
    expected: u64,
) -> Result<crate::FoundationCheckpointInspection> {
    if !source.join("rehearsal-continuation.json").try_exists()? {
        return crate::inspect_foundation_checkpoint(source, expected);
    }
    let manifest: Continuation =
        serde_json::from_slice(&read_bounded(&source.join("rehearsal-continuation.json"))?)?;
    if manifest.schema_version != 1
        || manifest.policy_version != expected
        || manifest.founder_seed_base == 0
        || manifest.biological_objective_version != 2
        || manifest.critic_objective_version != 1
        || !manifest.next_cohort_optimizer_rebound
        || !manifest.next_cohort_decision_checked
        || manifest.optimizer_reset
        || manifest.files.len() != FILES.len()
    {
        return Err("invalid simulator continuation contract".into());
    }
    let mut bytes = BTreeMap::new();
    for name in FILES {
        let payload = read_bounded(&source.join(name))?;
        if manifest.files.get(name) != Some(blake3::hash(&payload).as_bytes()) {
            return Err("simulator continuation file digest mismatch".into());
        }
        bytes.insert(name, payload);
    }
    let asset = FoundationWeightAsset::decode_canonical(&bytes["trained.alife-foundation"])?;
    let actor: alife_training::FoundationTrainerCheckpoint =
        serde_json::from_slice(&bytes["actor-checkpoint.json"])?;
    let value: alife_training::PpoValueHeadCheckpoint =
        serde_json::from_slice(&bytes["value-checkpoint.json"])?;
    let phenotype = crate::foundation_archived_success_replay::compile_source(
        asset.clone(),
        1,
        manifest.founder_seed_base,
    )?;
    actor.validate_for(&phenotype, asset.digest())?;
    if actor.optimizer_step != manifest.actor_optimizer_step
        || value.optimizer_step != manifest.value_optimizer_step
        || value.last_updated_policy_version != Some(expected)
        || value.feature_count != phenotype.neuron_count()
        || actor
            .weights
            .iter()
            .zip(asset.weights())
            .any(|(a, b)| a.to_bits() != b.to_bits())
    {
        return Err("simulator optimizer handoff disagrees with exported policy".into());
    }
    PpoTrainingState::from_checkpoint(value)?;
    Ok(crate::FoundationCheckpointInspection {
        expected_policy_version: expected,
        policy_version: expected,
        founder_seed_base: manifest.founder_seed_base,
        foundation_digest: asset.digest(),
        actor_optimizer_step: actor.optimizer_step,
        value_optimizer_step: manifest.value_optimizer_step,
        file_digests: FILES
            .into_iter()
            .map(|name| {
                use sha2::{Digest, Sha256};
                crate::FoundationCheckpointFileDigest {
                    file_name: name,
                    encoded_bytes: bytes[name].len(),
                    digest: alife_core::Blake3Digest::from_bytes(
                        *blake3::hash(&bytes[name]).as_bytes(),
                    ),
                    sha256: format!("{:x}", Sha256::digest(&bytes[name])),
                }
            })
            .collect(),
        neural_execution: false,
        weights_modified: false,
        archive_checked: false,
    })
}

pub(super) fn read_verified(
    source: &Path,
    name: &str,
    inspection: &crate::FoundationCheckpointInspection,
) -> Result<Vec<u8>> {
    let bytes = read_bounded(&source.join(name))?;
    let expected = inspection
        .file_digests
        .iter()
        .find(|file| file.file_name == name)
        .ok_or("source inspection omitted required file")?;
    if bytes.len() != expected.encoded_bytes
        || blake3::hash(&bytes).as_bytes() != expected.digest.bytes()
    {
        return Err("source changed after checkpoint admission".into());
    }
    Ok(bytes)
}

pub(super) fn publish(
    output: &Path,
    policy: u64,
    founder: u64,
    actor_age: u32,
    value_age: u32,
) -> Result<()> {
    let files = FILES
        .into_iter()
        .map(|name| {
            Ok((
                name.to_owned(),
                *blake3::hash(&read_bounded(&output.join(name))?).as_bytes(),
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let manifest = Continuation {
        schema_version: 1,
        policy_version: policy,
        founder_seed_base: founder,
        biological_objective_version: 2,
        critic_objective_version: 1,
        actor_optimizer_step: actor_age,
        value_optimizer_step: value_age,
        next_cohort_optimizer_rebound: true,
        next_cohort_decision_checked: true,
        optimizer_reset: false,
        files,
    };
    std::fs::write(
        output.join("rehearsal-continuation.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    inspect(output, policy)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_bytes_are_bound_to_the_exact_inspected_checkpoint() {
        let source = crate::foundation_archived_success_replay::tests::checkpoint_fixture(7400);
        let inspection = inspect(&source, 7400).unwrap();
        assert!(read_verified(&source, "actor-checkpoint.json", &inspection).is_ok());
        let path = source.join("actor-checkpoint.json");
        let mut bytes = std::fs::read(&path).unwrap();
        bytes.push(b' ');
        std::fs::write(&path, bytes).unwrap();
        assert!(read_verified(&source, "actor-checkpoint.json", &inspection).is_err());
        std::fs::remove_dir_all(source).unwrap();
    }
    #[test]
    fn continuation_rejects_policy_digest_and_optimizer_mismatches() {
        let source = crate::foundation_archived_success_replay::tests::checkpoint_fixture(7400);
        let inspection = inspect(&source, 7400).unwrap();
        std::fs::write(
            source.join("critic-residual-update.json"),
            b"{\"objective_version\":1}",
        )
        .unwrap();
        publish(
            &source,
            7400,
            inspection.founder_seed_base,
            inspection.actor_optimizer_step,
            inspection.value_optimizer_step,
        )
        .unwrap();
        assert!(inspect(&source, 7401).is_err());
        let path = source.join("value-checkpoint.json");
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        value["optimizer_step"] = serde_json::json!(0);
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(inspect(&source, 7400).is_err());
        std::fs::remove_dir_all(source).unwrap();
    }
}
