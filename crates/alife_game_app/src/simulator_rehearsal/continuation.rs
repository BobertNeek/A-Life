//! Exact, reusable handoff for this runner. Native cycles keep their existing
//! admission path; simulator continuation never impersonates a native cycle.
use super::*;
use std::collections::BTreeMap;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Continuation {
    schema_version: u16,
    complete: bool,
    receipt_file: String,
    execution_domain: ExecutionDomain,
    seed: u64,
    historical_freshness: String,
    policy_version: u64,
    founder_seed_base: u64,
    biological_objective_version: u16,
    critic_objective_version: u16,
    actor_optimizer_step: u32,
    value_optimizer_step: u32,
    next_cohort_optimizer_rebound: bool,
    next_cohort_decision_checked: bool,
    optimizer_reset: bool,
    source: SourceIdentity,
    files: BTreeMap<String, [u8; 32]>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ExecutionDomain {
    GpuWgsl,
}

/// Exact parent bytes, including its manifest when the parent is rehearsal.
/// This records ancestry without claiming a native cycle took place.
#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct SourceIdentity {
    policy_version: u64,
    founder_seed_base: u64,
    foundation_digest: alife_core::Blake3Digest,
    actor_optimizer_step: u32,
    value_optimizer_step: u32,
    file_digests: Vec<SourceFileIdentity>,
}

#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct SourceFileIdentity {
    file_name: String,
    encoded_bytes: usize,
    digest: alife_core::Blake3Digest,
}

impl SourceIdentity {
    fn from_inspection(source: &crate::FoundationCheckpointInspection) -> Result<Self> {
        // Decode only this typed subset; admission-only flags and SHA-256 display
        // fields remain in the complete rehearsal receipt.
        Ok(serde_json::from_value(serde_json::to_value(source)?)?)
    }

    fn valid_for(&self, manifest: &Continuation) -> bool {
        let expected: Vec<_> = if self
            .file_digests
            .iter()
            .any(|file| file.file_name == "cycle.json")
        {
            vec![
                "cycle.json",
                "trained.alife-foundation",
                "actor-checkpoint.json",
                "value-checkpoint.json",
            ]
        } else {
            FILES
                .into_iter()
                .chain(["rehearsal-continuation.json"])
                .collect()
        };
        self.policy_version.checked_add(1) == Some(manifest.policy_version)
            && self.founder_seed_base == manifest.founder_seed_base
            && self.actor_optimizer_step.checked_add(1) == Some(manifest.actor_optimizer_step)
            && self.value_optimizer_step.checked_add(1) == Some(manifest.value_optimizer_step)
            && self.file_digests.len() == expected.len()
            && expected.iter().all(|name| {
                self.file_digests
                    .iter()
                    .filter(|file| file.file_name == *name)
                    .count()
                    == 1
            })
    }
}

#[derive(serde::Deserialize)]
struct ReceiptIdentity {
    schema: String,
    source: SourceIdentity,
    config: ReceiptConfig,
    collection_policy: u64,
    actor_optimizer_before: u32,
    actor_optimizer_after: u32,
    value_optimizer_before: u32,
    value_optimizer_after: u32,
    trained_decisions: usize,
    first_update_covers_all_trainable_attempts: bool,
    historical_freshness: String,
}

#[derive(serde::Deserialize)]
struct ReceiptConfig {
    seed: u64,
    expected_source_policy: u64,
}

#[derive(serde::Deserialize)]
struct ResidualIdentity {
    objective_version: u16,
    initial_age: u32,
    final_age: u32,
    #[serde(rename = "Adam_reset")]
    adam_reset: bool,
}

const FILES: [&str; 5] = [
    "trained.alife-foundation",
    "actor-checkpoint.json",
    "value-checkpoint.json",
    "critic-residual-update.json",
    "rehearsal.json",
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
    let manifest_bytes = read_bounded(&source.join("rehearsal-continuation.json"))?;
    let manifest: Continuation = serde_json::from_slice(&manifest_bytes)?;
    if manifest.schema_version != 2
        || !manifest.complete
        || manifest.receipt_file != "rehearsal.json"
        || manifest.seed == 0
        || manifest.historical_freshness != "unknown"
        || manifest.policy_version != expected
        || manifest.founder_seed_base == 0
        || manifest.biological_objective_version != 2
        || manifest.critic_objective_version != 1
        || !manifest.next_cohort_optimizer_rebound
        || !manifest.next_cohort_decision_checked
        || manifest.optimizer_reset
        || manifest.files.len() != FILES.len()
        || !manifest.source.valid_for(&manifest)
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
    let receipt: ReceiptIdentity = serde_json::from_slice(&bytes["rehearsal.json"])?;
    let residual: ResidualIdentity = serde_json::from_slice(&bytes["critic-residual-update.json"])?;
    if receipt.schema != "alife.simulator-rehearsal.v1"
        || receipt.source != manifest.source
        || receipt.config.seed != manifest.seed
        || receipt.config.expected_source_policy != manifest.source.policy_version
        || receipt.historical_freshness != manifest.historical_freshness
        || receipt.collection_policy != expected
        || receipt.actor_optimizer_before != manifest.source.actor_optimizer_step
        || receipt.actor_optimizer_after != manifest.actor_optimizer_step
        || receipt.value_optimizer_before != manifest.source.value_optimizer_step
        || receipt.value_optimizer_after != manifest.value_optimizer_step
        || receipt.trained_decisions == 0
        || !receipt.first_update_covers_all_trainable_attempts
        || residual.objective_version != manifest.critic_objective_version
        || residual.initial_age != manifest.source.value_optimizer_step
        || residual.final_age != manifest.value_optimizer_step
        || residual.adam_reset
    {
        return Err(
            "simulator receipt disagrees with source ancestry or optimizer continuation".into(),
        );
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
    bytes.insert("rehearsal-continuation.json", manifest_bytes);
    Ok(crate::FoundationCheckpointInspection {
        expected_policy_version: expected,
        policy_version: expected,
        founder_seed_base: manifest.founder_seed_base,
        foundation_digest: asset.digest(),
        actor_optimizer_step: actor.optimizer_step,
        value_optimizer_step: manifest.value_optimizer_step,
        file_digests: FILES
            .into_iter()
            .chain(["rehearsal-continuation.json"])
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
    source: &crate::FoundationCheckpointInspection,
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
    let receipt: ReceiptIdentity =
        serde_json::from_slice(&read_bounded(&output.join("rehearsal.json"))?)?;
    let manifest = Continuation {
        schema_version: 2,
        complete: true,
        receipt_file: "rehearsal.json".into(),
        execution_domain: ExecutionDomain::GpuWgsl,
        seed: receipt.config.seed,
        historical_freshness: "unknown".into(),
        policy_version: policy,
        founder_seed_base: source.founder_seed_base,
        biological_objective_version: 2,
        critic_objective_version: 1,
        actor_optimizer_step: actor_age,
        value_optimizer_step: value_age,
        next_cohort_optimizer_rebound: true,
        next_cohort_decision_checked: true,
        optimizer_reset: false,
        source: SourceIdentity::from_inspection(source)?,
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

    /// Synthetic format fixture only. It claims no neural execution or learning.
    fn continuation_fixture(
        source: &Path,
        inspection: &crate::FoundationCheckpointInspection,
        name: &str,
    ) -> std::path::PathBuf {
        let output = source.join(name);
        std::fs::create_dir(&output).unwrap();
        std::fs::copy(
            source.join("trained.alife-foundation"),
            output.join("trained.alife-foundation"),
        )
        .unwrap();
        let next_policy = inspection.policy_version + 1;
        for (name, age) in [
            ("actor-checkpoint.json", inspection.actor_optimizer_step + 1),
            ("value-checkpoint.json", inspection.value_optimizer_step + 1),
        ] {
            let mut data: serde_json::Value =
                serde_json::from_slice(&std::fs::read(source.join(name)).unwrap()).unwrap();
            data["optimizer_step"] = serde_json::json!(age);
            if name == "value-checkpoint.json" {
                data["last_updated_policy_version"] = serde_json::json!(next_policy);
            }
            std::fs::write(output.join(name), serde_json::to_vec(&data).unwrap()).unwrap();
        }
        std::fs::write(
            output.join("critic-residual-update.json"),
            serde_json::to_vec(&serde_json::json!({
                "objective_version":1,"initial_age":inspection.value_optimizer_step,
                "final_age":inspection.value_optimizer_step+1,"Adam_reset":false
            }))
            .unwrap(),
        )
        .unwrap();
        std::fs::write(
            output.join("rehearsal.json"),
            serde_json::to_vec(&serde_json::json!({
                "schema":"alife.simulator-rehearsal.v1",
                "source":inspection,"collection_policy":next_policy,
                "config":{"seed":37,"expected_source_policy":inspection.policy_version},
                "historical_freshness":"unknown",
                "actor_optimizer_before":inspection.actor_optimizer_step,
                "actor_optimizer_after":inspection.actor_optimizer_step+1,
                "value_optimizer_before":inspection.value_optimizer_step,
                "value_optimizer_after":inspection.value_optimizer_step+1,
                "trained_decisions":1,"first_update_covers_all_trainable_attempts":true
            }))
            .unwrap(),
        )
        .unwrap();
        output
    }

    fn publish_fixture(
        output: &Path,
        inspection: &crate::FoundationCheckpointInspection,
    ) -> Result<()> {
        publish(
            output,
            inspection.policy_version + 1,
            inspection,
            inspection.actor_optimizer_step + 1,
            inspection.value_optimizer_step + 1,
        )
    }

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
        let output = continuation_fixture(&source, &inspection, "next");
        publish_fixture(&output, &inspection).unwrap();
        assert!(inspect(&output, 7401).is_ok());
        assert!(inspect(&output, 7400).is_err());
        assert!(!output.join("cycle.json").exists());
        assert!(crate::inspect_foundation_checkpoint(&output, 7401).is_err());
        let path = output.join("value-checkpoint.json");
        let mut value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        value["optimizer_step"] = serde_json::json!(0);
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(inspect(&output, 7401).is_err());
        std::fs::remove_dir_all(source).unwrap();
    }

    #[test]
    fn ancestry_chain_binds_parent_manifest_and_exact_optimizer_ages() {
        let source = crate::foundation_archived_success_replay::tests::checkpoint_fixture(7400);
        let original = inspect(&source, 7400).unwrap();
        let first = continuation_fixture(&source, &original, "first");
        publish_fixture(&first, &original).unwrap();
        let first_inspection = inspect(&first, 7401).unwrap();
        let second = continuation_fixture(&first, &first_inspection, "second");
        publish_fixture(&second, &first_inspection).unwrap();
        let final_inspection = inspect(&second, 7402).unwrap();
        assert_eq!(
            final_inspection.actor_optimizer_step,
            original.actor_optimizer_step + 2
        );
        assert_eq!(
            final_inspection.value_optimizer_step,
            original.value_optimizer_step + 2
        );
        let manifest: Continuation = serde_json::from_slice(
            &std::fs::read(second.join("rehearsal-continuation.json")).unwrap(),
        )
        .unwrap();
        let parent = manifest
            .source
            .file_digests
            .iter()
            .find(|file| file.file_name == "rehearsal-continuation.json")
            .unwrap();
        assert_eq!(
            parent.digest.bytes(),
            blake3::hash(&std::fs::read(first.join(&parent.file_name)).unwrap()).as_bytes()
        );
        assert_eq!(
            inspect(&source, 7400).unwrap().actor_optimizer_step,
            original.actor_optimizer_step
        );
        std::fs::remove_dir_all(source).unwrap();
    }

    #[test]
    fn shared_admission_adapter_returns_full_state_and_actual_receipts_without_a_device() {
        let source = crate::foundation_archived_success_replay::tests::checkpoint_fixture(7400);
        let original = inspect(&source, 7400).unwrap();
        assert!(admit_simulator_rehearsal_training_source(&source, 7400).is_err());
        let output = continuation_fixture(&source, &original, "next");
        publish_fixture(&output, &original).unwrap();
        let admitted = admit_simulator_rehearsal_training_source(&output, 7401).unwrap();
        assert_eq!(admitted.receipt_file, "rehearsal.json");
        assert_eq!(admitted.manifest_file, "rehearsal-continuation.json");
        assert_eq!(admitted.execution_domain, "gpu-wgsl");
        assert_eq!(admitted.inspection.policy_version, 7401);
        assert!(!admitted.inspection.neural_execution);
        assert_eq!(
            admitted.asset.digest(),
            admitted.inspection.foundation_digest
        );
        assert_eq!(
            admitted.actor.optimizer_step,
            original.actor_optimizer_step + 1
        );
        assert_eq!(
            admitted.value.optimizer_step,
            original.value_optimizer_step + 1
        );
        assert_eq!(admitted.value.last_updated_policy_version, Some(7401));
        let stored: alife_training::FoundationTrainerCheckpoint =
            serde_json::from_slice(&std::fs::read(output.join("actor-checkpoint.json")).unwrap())
                .unwrap();
        // Compare the same typed f32 encoding, without f64 JSON normalization
        // or dumping the large checkpoint on an assertion failure.
        assert!(
            serde_json::to_vec(&admitted.actor).unwrap() == serde_json::to_vec(&stored).unwrap()
        );
        assert_eq!(admitted.compiler_world_seed, 37);
        assert_eq!(admitted.manifest["source"]["policy_version"], 7400);
        assert_eq!(admitted.receipt["collection_policy"], 7401);
        assert!(admit_simulator_rehearsal_training_source(&output, 7402).is_err());
        std::fs::remove_dir_all(source).unwrap();
    }

    #[test]
    fn unsealed_evidence_false_ancestry_and_reset_ages_cannot_be_admitted() {
        let source = crate::foundation_archived_success_replay::tests::checkpoint_fixture(7400);
        let original = inspect(&source, 7400).unwrap();
        let output = continuation_fixture(&source, &original, "next");
        assert!(inspect(&output, 7401).is_err());
        publish_fixture(&output, &original).unwrap();
        let marker = output.join("rehearsal-continuation.json");
        let accepted: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&marker).unwrap()).unwrap();
        for (field, value) in [
            ("schema_version", serde_json::json!(1)),
            ("complete", serde_json::json!(false)),
            ("receipt_file", serde_json::json!("cycle.json")),
            ("execution_domain", serde_json::json!("cpu")),
            ("actor_optimizer_step", serde_json::json!(0)),
        ] {
            let mut forged = accepted.clone();
            forged[field] = value;
            std::fs::write(&marker, serde_json::to_vec(&forged).unwrap()).unwrap();
            assert!(inspect(&output, 7401).is_err(), "accepted {field}");
        }
        std::fs::write(&marker, serde_json::to_vec(&accepted).unwrap()).unwrap();
        let receipt_path = output.join("rehearsal.json");
        let receipt_bytes = std::fs::read(&receipt_path).unwrap();
        for field in [
            "actor_optimizer_after",
            "value_optimizer_before",
            "trained_decisions",
        ] {
            let mut receipt: serde_json::Value = serde_json::from_slice(&receipt_bytes).unwrap();
            receipt[field] = serde_json::json!(0);
            let payload = serde_json::to_vec(&receipt).unwrap();
            std::fs::write(&receipt_path, &payload).unwrap();
            // Rehash to exercise semantic rejection, beyond byte tampering.
            let mut forged = accepted.clone();
            forged["files"]["rehearsal.json"] =
                serde_json::json!(blake3::hash(&payload).as_bytes());
            std::fs::write(&marker, serde_json::to_vec(&forged).unwrap()).unwrap();
            assert!(inspect(&output, 7401).is_err(), "accepted receipt {field}");
        }
        std::fs::write(&receipt_path, &receipt_bytes).unwrap();
        let mut forged = accepted;
        forged["source"]["file_digests"][0]["file_name"] = serde_json::json!("unknown.json");
        std::fs::write(&marker, serde_json::to_vec(&forged).unwrap()).unwrap();
        assert!(inspect(&output, 7401).is_err());
        std::fs::remove_dir_all(source).unwrap();
    }
}
