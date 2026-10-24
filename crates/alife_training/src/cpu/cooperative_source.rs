//! Explicit founder-only continuation from a completed cooperative CPU round.
//! This never runs biology or an optimizer, restores a private life, chooses an
//! accepted lineage, or claims historical freshness for generic collectors.
use super::{
    collector_output::{fingerprint, owned_root, publish_new, sync_directory_ancestry},
    read_cpu_cooperative_round, CpuCooperativeRoundResultV1, CPU_COOPERATIVE_ROUND_RESULT_FILE,
};
use crate::{
    native_receipt::{self, Lineage},
    resolve_isolated_training_output, FoundationTrainerCheckpoint, FoundationTrainingBackendV1,
    FoundationTrainingHandoffV1, PpoValueHeadCheckpoint,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Component, Path},
};

pub const CPU_COOPERATIVE_FOUNDER_MODE: &str = "native_cpu_cooperative_founder";
const RECEIPT: &str = "cpu-cycle.json";
const HANDOFF: &str = "training-handoff.json";
const SOURCE_HANDOFF: &str = "cooperative-source-handoff.json";
const ROUND: &str = "cooperative-result/cooperative-round.json";
const STAGING: &str = ".cooperative-source-staging";
const PHASE: &[u8] = b"complete-fresh-cohort-cpu";
const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_STAGES: usize = 256;
const STANDARD: [&str; 6] = [
    RECEIPT,
    HANDOFF,
    "trained.alife-foundation",
    "actor-checkpoint.json",
    "value-checkpoint.json",
    "phase.txt",
];
const OUTPUT_PINS: [&str; 4] = [
    HANDOFF,
    "trained.alife-foundation",
    "actor-checkpoint.json",
    "value-checkpoint.json",
];
type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Exact native files required to admit and retain the selected founder. Flat
/// source-lineage blobs are transported separately by the existing Lineage API.
pub struct CpuNativeFounderSource {
    pub receipt: Value,
    pub handoff: FoundationTrainingHandoffV1,
    pub files: BTreeMap<String, Vec<u8>>,
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn hash(bytes: &[u8]) -> String {
    native_receipt::sha256(bytes)
}
fn exact(a: &impl serde::Serialize, b: &impl serde::Serialize) -> Result<bool> {
    Ok(serde_json::to_vec(a)? == serde_json::to_vec(b)?)
}
fn relative(key: &str) -> Result<()> {
    if key.is_empty()
        || key.contains(['\\', ':'])
        || !Path::new(key)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
        || key.split('/').any(|part| matches!(part, "" | "." | ".."))
    {
        return Err(invalid("native founder artifact path is not canonical and relative").into());
    }
    Ok(())
}
fn read_file(root: &Path, key: &str) -> Result<Vec<u8>> {
    relative(key)?;
    let mut path = root.to_path_buf();
    for part in Path::new(key).components() {
        path.push(part.as_os_str());
        if fs::symlink_metadata(&path)?.file_type().is_symlink() {
            return Err(invalid("native founder artifacts must not contain symlinks").into());
        }
    }
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.is_file()
        || metadata.len() > MAX_FILE_BYTES
        || !path.canonicalize()?.starts_with(root)
    {
        return Err(invalid("invalid native founder artifact or byte limit").into());
    }
    let bytes = fs::read(&path)?;
    if bytes.len() as u64 != metadata.len() {
        return Err(invalid("native founder artifact changed while reading").into());
    }
    Ok(bytes)
}
fn asset_digest(handoff: &FoundationTrainingHandoffV1) -> Result<String> {
    let asset = handoff
        .compiler_inputs
        .n2048_candidate_asset()
        .ok_or_else(|| invalid("native founder handoff lacks candidate asset"))?;
    Ok(asset
        .digest()
        .bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}
fn validate_header(receipt: &Value, handoff: &FoundationTrainingHandoffV1) -> Result<()> {
    if receipt["schema_version"] != 1
        || receipt["complete"] != true
        || receipt["backend"] != "native-cpu"
        || !matches!(
            receipt["mode"].as_str(),
            Some("native_cpu_fresh_cohort" | CPU_COOPERATIVE_FOUNDER_MODE)
        )
        || receipt["receipt_filename"] != RECEIPT
        || receipt["handoff_filename"] != HANDOFF
        || receipt["biological_objective_version"] != 2
        || receipt["critic_objective_version"] != 1
        || receipt["objective_state_reset"] != false
        || receipt["policy_version"] != handoff.policy_version
        || receipt["actor_optimizer_step"] != handoff.actor.optimizer_step
        || receipt["value_optimizer_step"] != handoff.value.optimizer_step
        || receipt["seed"].as_u64().is_none_or(|seed| seed == 0)
        || receipt["founder_seed_base"]
            .as_u64()
            .is_none_or(|seed| seed == 0)
        || match receipt.get("fixture") {
            Some(value) => !value.is_boolean(),
            None => receipt["mode"] == CPU_COOPERATIVE_FOUNDER_MODE,
        }
        || receipt["new_asset_digest"] != asset_digest(handoff)?
        || handoff.origin != FoundationTrainingBackendV1::NativeCpu
    {
        return Err(
            invalid("native CPU founder receipt identity or optimizer ages disagree").into(),
        );
    }
    Ok(())
}

/// Admit an explicitly selected CPU native source without allocating a seed or
/// advancing policy state. Cooperative sources additionally require their raw
/// round proof and predecessor handoff on every direct admission.
pub fn read_cpu_founder_source(directory: &Path) -> Result<CpuNativeFounderSource> {
    let root = owned_root(directory)?;
    for name in ["cycle.json", "group50.json", "collector-commit.json"] {
        match fs::symlink_metadata(root.join(name)) {
            Ok(_) => return Err(invalid("native CPU founder has mixed receipt domains").into()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => (),
            Err(error) => return Err(error.into()),
        }
    }
    let mut files = STANDARD
        .into_iter()
        .map(|name| Ok((name.into(), read_file(&root, name)?)))
        .collect::<Result<BTreeMap<String, Vec<u8>>>>()?;
    if std::str::from_utf8(&files["phase.txt"])?.trim() != std::str::from_utf8(PHASE)? {
        return Err(invalid("native CPU founder lacks its completed phase marker").into());
    }
    let receipt = native_receipt::parse(&files[RECEIPT])?;
    match fs::symlink_metadata(root.join("compiler-inputs.json")) {
        Ok(_) => {
            files.insert(
                "compiler-inputs.json".into(),
                read_file(&root, "compiler-inputs.json")?,
            );
        }
        Err(error)
            if error.kind() == io::ErrorKind::NotFound
                && receipt["mode"] != CPU_COOPERATIVE_FOUNDER_MODE => {}
        Err(error) => return Err(error.into()),
    }
    let handoff: FoundationTrainingHandoffV1 = serde_json::from_slice(&files[HANDOFF])?;
    handoff.validate()?;
    validate_header(&receipt, &handoff)?;
    let actor: FoundationTrainerCheckpoint =
        serde_json::from_slice(&files["actor-checkpoint.json"])?;
    let value: PpoValueHeadCheckpoint = serde_json::from_slice(&files["value-checkpoint.json"])?;
    let inputs = files
        .get("compiler-inputs.json")
        .map(|bytes| serde_json::from_slice::<alife_core::PhenotypeCompilerInputs>(bytes))
        .transpose()?;
    let inputs_match = match inputs {
        Some(inputs) => exact(&inputs, &handoff.compiler_inputs)?,
        None => true,
    };
    if !exact(&actor, &handoff.actor)?
        || !exact(&value, &handoff.value)?
        || !inputs_match
        || handoff
            .compiler_inputs
            .n2048_candidate_asset()
            .ok_or_else(|| invalid("native source lacks candidate asset"))?
            .encode_canonical()?
            != files["trained.alife-foundation"]
        || OUTPUT_PINS
            .iter()
            .any(|name| receipt["native_output_sha256"][*name] != hash(&files[*name]))
    {
        return Err(invalid(
            "native CPU founder differs from its exact typed handoff or native pins",
        )
        .into());
    }
    let lineage = Lineage::load(&receipt, &root)?;
    lineage.verify_at(&root)?;
    lineage.check_parent(&files[RECEIPT])?;
    if receipt["mode"] == CPU_COOPERATIVE_FOUNDER_MODE {
        for name in [SOURCE_HANDOFF, ROUND] {
            files.insert(name.into(), read_file(&root, name)?);
        }
        validate_cooperative(&root, &receipt, &handoff, &files)?;
    }
    Ok(CpuNativeFounderSource {
        receipt,
        handoff,
        files,
    })
}

fn consumed_ranges(result: &CpuCooperativeRoundResultV1) -> Value {
    let seeds = result
        .plan
        .members
        .iter()
        .map(|member| member.assignment.world_seed)
        .collect::<BTreeSet<_>>();
    json!(seeds
        .into_iter()
        .map(|seed| [seed, seed])
        .collect::<Vec<_>>())
}

fn validate_cooperative(
    root: &Path,
    receipt: &Value,
    handoff: &FoundationTrainingHandoffV1,
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<()> {
    let result = read_cpu_cooperative_round(&root.join("cooperative-result"))?;
    let prior: FoundationTrainingHandoffV1 = serde_json::from_slice(&files[SOURCE_HANDOFF])?;
    prior.validate()?;
    let parent_hash = receipt["seed_provenance"]["native_source_receipt_ref"]
        .as_str()
        .ok_or_else(|| invalid("cooperative founder lacks its archived predecessor reference"))?;
    // Lineage validated the digest spelling and predecessor domain first.
    let parent_bytes = read_file(root, &format!("source-lineage/{parent_hash}.json"))?;
    let parent = native_receipt::parse(&parent_bytes)?;
    validate_header(&parent, &prior)?;
    let parent_lineage = Lineage::load(&parent, root)?;
    let parent_pins = json!({RECEIPT:hash(&parent_bytes),
        "trained.alife-foundation":parent["native_output_sha256"]["trained.alife-foundation"],
        "actor-checkpoint.json":parent["native_output_sha256"]["actor-checkpoint.json"],
        "value-checkpoint.json":parent["native_output_sha256"]["value-checkpoint.json"]});
    let expected_outputs = [
        HANDOFF,
        "trained.alife-foundation",
        "actor-checkpoint.json",
        "value-checkpoint.json",
        "compiler-inputs.json",
        SOURCE_HANDOFF,
        ROUND,
    ]
    .into_iter()
    .map(|name| (name.to_owned(), json!(hash(&files[name]))))
    .collect::<serde_json::Map<_, _>>();
    if Sha256::digest(&files[SOURCE_HANDOFF])[..] != result.plan.source_handoff_sha256
        || parent["native_output_sha256"][HANDOFF] != hash(&files[SOURCE_HANDOFF])
        || !exact(&prior, &result.source_handoff)?
        || !exact(handoff, &result.output_handoff)?
        || receipt["native_output_sha256"] != Value::Object(expected_outputs)
        || receipt["cooperative_result_sha256"] != hash(&files[ROUND])
        || receipt["cooperative_result_payload_sha256"] != json!(result.payload_sha256)
        || receipt["cooperative_plan_sha256"] != json!(result.plan.payload_sha256)
        || receipt["consumed_seed_ranges"] != consumed_ranges(&result)
        || receipt["seed"] != parent["seed"]
        || receipt["founder_seed_base"] != parent["founder_seed_base"]
        || receipt["source_policy_version"] != prior.policy_version
        || receipt["collection_policy_version"] != result.plan.snapshot.policy_version
        || receipt["initial_actor_optimizer_step"] != prior.actor.optimizer_step
        || receipt["initial_value_optimizer_step"] != prior.value.optimizer_step
        || receipt["actor_optimizer_updates"].as_u64()
            != handoff
                .actor
                .optimizer_step
                .checked_sub(prior.actor.optimizer_step)
                .map(u64::from)
        || receipt["value_optimizer_updates"].as_u64()
            != handoff
                .value
                .optimizer_step
                .checked_sub(prior.value.optimizer_step)
                .map(u64::from)
        || receipt["seed_provenance"]["source_sha256"] != parent_pins
        || receipt["fixture"] != parent_lineage.contains_fixture()?
        || receipt["continuation_kind"] != "fresh-lives-only-cooperative-founder"
        || receipt["member_historical_freshness"] != "unasserted"
        || receipt["seed_admission"]
            != json!({"policy":"explicit-cooperative-members","historical_freshness":"unasserted"})
        || receipt["seed_provenance"]["seed_admission"] != receipt["seed_admission"]
        || receipt["private_lifetime_continuation"] != "none; fresh lives only"
        || receipt["candidate_acceptance"] != "none; explicit source materialization only"
    {
        return Err(invalid(
            "cooperative founder proof, consumed seeds or inherited source identity disagree",
        )
        .into());
    }
    Ok(())
}

/// Materialize an explicitly selected completed round as a new founder source.
/// All learned parameters, integer ages and compiler coordinates come from its
/// existing output handoff, including a truthful zero-update policy version.
pub fn materialize_cpu_cooperative_source(
    source_native: &Path,
    result_directory: &Path,
    new_output: &Path,
) -> Result<Value> {
    if fs::symlink_metadata(new_output).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(invalid("cooperative source output must not be a symlink").into());
    }
    let source_root = owned_root(source_native)?;
    let round_root = owned_root(result_directory)?;
    let output = resolve_isolated_training_output(
        new_output,
        [&source_root, &round_root].map(|path| path.as_path()),
    )?;
    let source = read_cpu_founder_source(&source_root)?;
    let result = read_cpu_cooperative_round(&round_root)?;
    let result_bytes = read_file(&round_root, CPU_COOPERATIVE_ROUND_RESULT_FILE)?;
    let pinned_result: CpuCooperativeRoundResultV1 = serde_json::from_slice(&result_bytes)?;
    pinned_result.validate()?;
    if !exact(&pinned_result, &result)?
        || Sha256::digest(&source.files[HANDOFF])[..] != result.plan.source_handoff_sha256
        || !exact(&source.handoff, &result.source_handoff)?
    {
        return Err(
            invalid("cooperative round does not bind the selected native source handoff").into(),
        );
    }
    let mut lineage = Lineage::load(&source.receipt, &source_root)?;
    let fixture = lineage.contains_fixture()?;
    let parent_hash = lineage.retain_parent(source.files[RECEIPT].clone())?;
    let handoff = &result.output_handoff;
    let asset = handoff
        .compiler_inputs
        .n2048_candidate_asset()
        .ok_or_else(|| invalid("round output lacks candidate asset"))?;
    let mut expected: BTreeMap<String, Vec<u8>> = BTreeMap::from([
        (HANDOFF.into(), serde_json::to_vec(handoff)?),
        ("trained.alife-foundation".into(), asset.encode_canonical()?),
        (
            "actor-checkpoint.json".into(),
            serde_json::to_vec(&handoff.actor)?,
        ),
        (
            "value-checkpoint.json".into(),
            serde_json::to_vec(&handoff.value)?,
        ),
        (
            "compiler-inputs.json".into(),
            serde_json::to_vec(&handoff.compiler_inputs)?,
        ),
        ("phase.txt".into(), PHASE.to_vec()),
        (SOURCE_HANDOFF.into(), source.files[HANDOFF].clone()),
        (ROUND.into(), result_bytes),
    ]);
    let native_output = expected
        .iter()
        .filter(|(name, _)| name.as_str() != "phase.txt")
        .map(|(name, bytes)| (name.clone(), hash(bytes)))
        .collect::<BTreeMap<_, _>>();
    let source_pins = [
        RECEIPT,
        "trained.alife-foundation",
        "actor-checkpoint.json",
        "value-checkpoint.json",
    ]
    .into_iter()
    .map(|name| (name.to_owned(), hash(&source.files[name])))
    .collect::<BTreeMap<_, _>>();
    let seed_admission =
        json!({"policy":"explicit-cooperative-members","historical_freshness":"unasserted"});
    let receipt = json!({"schema_version":1,"complete":true,"mode":CPU_COOPERATIVE_FOUNDER_MODE,
        "receipt_filename":RECEIPT,"handoff_filename":HANDOFF,"backend":"native-cpu",
        "continuation_kind":"fresh-lives-only-cooperative-founder","fixture":fixture,
        "biological_objective_version":2,"critic_objective_version":1,"objective_state_reset":false,
        "source_policy_version":source.handoff.policy_version,"policy_version":handoff.policy_version,
        "collection_policy_version":result.plan.snapshot.policy_version,
        "seed":source.receipt["seed"],"founder_seed_base":source.receipt["founder_seed_base"],
        "initial_actor_optimizer_step":source.handoff.actor.optimizer_step,
        "initial_value_optimizer_step":source.handoff.value.optimizer_step,
        "actor_optimizer_step":handoff.actor.optimizer_step,"value_optimizer_step":handoff.value.optimizer_step,
        "actor_optimizer_updates":handoff.actor.optimizer_step.checked_sub(source.handoff.actor.optimizer_step),
        "value_optimizer_updates":handoff.value.optimizer_step.checked_sub(source.handoff.value.optimizer_step),
        "new_asset_digest":asset_digest(handoff)?,"native_output_sha256":native_output,
        "cooperative_result_sha256":hash(&expected[ROUND]),"cooperative_result_payload_sha256":result.payload_sha256,
        "cooperative_plan_sha256":result.plan.payload_sha256,"consumed_seed_ranges":consumed_ranges(&result),
        "member_historical_freshness":"unasserted","seed_admission":seed_admission,
        "seed_provenance":{"source_sha256":source_pins,"native_source_receipt_ref":parent_hash,"seed_admission":seed_admission},
        "private_lifetime_continuation":"none; fresh lives only","candidate_acceptance":"none; explicit source materialization only"});
    expected.insert(RECEIPT.into(), serde_json::to_vec(&receipt)?);
    expected.extend(lineage.raw_files()?);
    publish_files(&output, &expected)?;
    let admitted = read_cpu_founder_source(&output)?;
    if !exact(&admitted.handoff, handoff)? || admitted.receipt != receipt {
        return Err(invalid("materialized cooperative founder changed during publication").into());
    }
    Ok(admitted.receipt)
}

fn stage_prefix(key: &str, bytes: &[u8]) -> String {
    // Bind path and content with one full digest, keeping Windows paths short.
    let mut digest = Sha256::new();
    digest.update(b"ALIFE-COOPERATIVE-SOURCE-STAGE-V1");
    digest.update((key.len() as u64).to_le_bytes());
    digest.update(key.as_bytes());
    digest.update(bytes);
    format!("{:x}-", digest.finalize())
}

/// Inspect every existing path before any mutation. Known torn stages retain
/// exact prefixes; mismatched canonical bytes or foreign files are never reused.
fn verify_tree(root: &Path, expected: &BTreeMap<String, Vec<u8>>) -> Result<()> {
    let stages = expected
        .iter()
        .map(|(key, bytes)| (stage_prefix(key, bytes), bytes))
        .collect::<BTreeMap<_, _>>();
    let mut directories = BTreeSet::from([STAGING.to_owned()]);
    for key in expected.keys() {
        relative(key)?;
        let mut parent = Path::new(key).parent();
        while let Some(path) = parent.filter(|path| !path.as_os_str().is_empty()) {
            directories.insert(
                path.to_str()
                    .ok_or_else(|| invalid("non-UTF8 founder directory"))?
                    .replace('\\', "/"),
            );
            parent = path.parent();
        }
    }
    fn visit(
        root: &Path,
        directory: &Path,
        expected: &BTreeMap<String, Vec<u8>>,
        directories: &BTreeSet<String>,
        stages: &BTreeMap<String, &Vec<u8>>,
        depth: usize,
    ) -> Result<()> {
        if depth > 4 {
            return Err(invalid("cooperative source directory depth exceeded").into());
        }
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            let key = path
                .strip_prefix(root)?
                .to_str()
                .ok_or_else(|| invalid("non-UTF8 founder artifact"))?
                .replace('\\', "/");
            let kind = entry.file_type()?;
            if kind.is_symlink() || !path.canonicalize()?.starts_with(root) {
                return Err(
                    invalid("cooperative source artifact escapes its owned directory").into(),
                );
            }
            if kind.is_dir() {
                if !directories.contains(&key) {
                    return Err(invalid("foreign cooperative source directory").into());
                }
                visit(root, &path, expected, directories, stages, depth + 1)?;
            } else if kind.is_file() {
                if let Some(bytes) = expected.get(&key) {
                    if read_file(root, &key)? != *bytes {
                        return Err(invalid("existing cooperative source bytes disagree").into());
                    }
                } else if let Some(name) = key.strip_prefix(&format!("{STAGING}/")) {
                    let (prefix, expected_bytes) = stages
                        .iter()
                        .find(|(prefix, _)| name.starts_with(prefix.as_str()))
                        .ok_or_else(|| invalid("foreign cooperative source stage"))?;
                    let number = name
                        .strip_prefix(prefix.as_str())
                        .and_then(|suffix| suffix.strip_suffix(".pending"))
                        .ok_or_else(|| invalid("invalid cooperative source stage suffix"))?;
                    let index = number.parse::<usize>()?;
                    if index >= MAX_STAGES
                        || number != index.to_string()
                        || fs::metadata(&path)?.len() > expected_bytes.len() as u64
                        || !expected_bytes.starts_with(&read_file(root, &key)?)
                    {
                        return Err(invalid("cooperative source stage bytes disagree").into());
                    }
                } else {
                    return Err(invalid("foreign cooperative source file").into());
                }
            } else {
                return Err(invalid("nonregular cooperative source artifact").into());
            }
        }
        Ok(())
    }
    visit(root, root, expected, &directories, &stages, 0)
}

fn publish_files(output: &Path, expected: &BTreeMap<String, Vec<u8>>) -> Result<()> {
    match fs::symlink_metadata(output) {
        Ok(_) => (),
        Err(error) if error.kind() == io::ErrorKind::NotFound => fs::create_dir_all(output)?,
        Err(error) => return Err(error.into()),
    }
    let root = owned_root(output)?;
    verify_tree(&root, expected)?;
    fs::create_dir_all(root.join(STAGING))?;
    // A receipt is the commit, so publish it after every checkpoint and proof.
    let names = expected
        .keys()
        .filter(|name| name.as_str() != RECEIPT)
        .chain(expected.keys().filter(|name| name.as_str() == RECEIPT));
    for key in names {
        let bytes = &expected[key];
        let target = root.join(key);
        if target.exists() {
            fingerprint(&target, true)?;
            continue;
        }
        fs::create_dir_all(
            target
                .parent()
                .ok_or_else(|| invalid("founder file lacks parent"))?,
        )?;
        let prefix = stage_prefix(key, bytes);
        let mut selected = None;
        for index in 0..MAX_STAGES {
            let pending = root.join(STAGING).join(format!("{prefix}{index}.pending"));
            if pending.exists() {
                if fs::metadata(&pending)?.len() == bytes.len() as u64 {
                    fingerprint(&pending, true)?;
                    selected = Some(pending);
                    break;
                }
                continue;
            }
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&pending)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            selected = Some(pending);
            break;
        }
        let pending =
            selected.ok_or_else(|| invalid("cooperative source stage capacity exhausted"))?;
        sync_directory_ancestry(
            target
                .parent()
                .ok_or_else(|| invalid("founder file lacks parent"))?,
        )?;
        publish_new(&pending, &target)?;
    }
    sync_directory_ancestry(&root.join(STAGING))?;
    verify_tree(&root, expected)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };
    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "alife-cooperative-source-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path.canonicalize().unwrap())
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn files() -> BTreeMap<String, Vec<u8>> {
        BTreeMap::from([
            (RECEIPT.into(), b"synthetic receipt".to_vec()),
            (
                "cooperative-result/cooperative-round.json".into(),
                b"synthetic proof".to_vec(),
            ),
        ])
    }
    #[test]
    fn partial_stages_survive_retry_and_completed_publication_is_idempotent() {
        let root = Directory::new();
        let expected = files();
        fs::create_dir(root.0.join(STAGING)).unwrap();
        let torn = root.0.join(STAGING).join(format!(
            "{}0.pending",
            stage_prefix(RECEIPT, &expected[RECEIPT])
        ));
        fs::write(&torn, b"synthetic").unwrap();
        publish_files(&root.0, &expected).unwrap();
        publish_files(&root.0, &expected).unwrap();
        assert_eq!(fs::read(torn).unwrap(), b"synthetic");
        for (key, bytes) in expected {
            assert_eq!(fs::read(root.0.join(key)).unwrap(), bytes);
        }
    }
    #[test]
    fn foreign_or_changed_files_are_preserved_and_block_any_publication() {
        let root = Directory::new();
        let expected = files();
        let foreign = root.0.join("private-checkpoint.json");
        fs::write(&foreign, b"preserve").unwrap();
        assert!(publish_files(&root.0, &expected).is_err());
        assert!(!root.0.join(RECEIPT).exists());
        assert_eq!(fs::read(&foreign).unwrap(), b"preserve");
        fs::remove_file(foreign).unwrap();
        fs::write(root.0.join(RECEIPT), b"different").unwrap();
        assert!(publish_files(&root.0, &expected).is_err());
        assert_eq!(fs::read(root.0.join(RECEIPT)).unwrap(), b"different");
        assert!(!root.0.join("cooperative-result").exists());
    }
}
