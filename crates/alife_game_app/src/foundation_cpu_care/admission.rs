//! Dedicated CPU lane: exact history, durable exclusions and terminal attempts.
use super::{CpuCareRequest, Result};
use crate::foundation_training_cycle::{admit_native_cpu_care_seed, read_native_cpu_care_evidence};
use alife_training::{
    cpu::CpuContinuationConfig, FoundationTrainerCheckpoint, PpoValueHeadCheckpoint,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

const LEDGER: &str = ".grab-food-seed-reservations.json";
const LOCK: &str = ".eat-held-food-owner.lock";

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn write_atomic(path: &Path, value: &Value) -> Result<()> {
    let temporary = path.with_extension(format!(
        "{}-{}.pending",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)?;
    let result = (|| -> Result<()> {
        file.write_all(&serde_json::to_vec_pretty(value)?)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)?;
        #[cfg(unix)]
        fs::File::open(path.parent().ok_or("ledger lacks parent")?)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
struct LaneLock(PathBuf, Option<fs::File>);
fn lock_lane(lane: &Path) -> Result<LaneLock> {
    let path = lane.join(LOCK);
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)?;
    writeln!(file, "pid={}", std::process::id())?;
    file.sync_all()?;
    Ok(LaneLock(path, Some(file)))
}

fn read_ledger(path: &Path) -> Result<Value> {
    if fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Err("CPU care ledger must not be a symlink".into());
    }
    let (_, ledger) = read_native_cpu_care_evidence(path)?;
    if ledger["kind"] != "grab-food-seed-ledger" || ledger["cpu_care_schema"] != 1 {
        return Err("CPU care refuses another owner's ledger; use a dedicated lane".into());
    }
    let attempts = ledger["cpu_care_attempts"]
        .as_array()
        .ok_or("invalid CPU attempts")?;
    let mut seeds = std::collections::BTreeSet::new();
    let mut outputs = std::collections::BTreeSet::new();
    let mut latest = None;
    for (i, attempt) in attempts.iter().enumerate() {
        let seed = attempt["seed"]
            .as_u64()
            .filter(|s| *s > 0)
            .ok_or("invalid attempted seed")?;
        let output = attempt["output"]
            .as_str()
            .ok_or("invalid attempted output")?;
        let status = attempt["status"].as_str().ok_or("invalid attempt status")?;
        if !seeds.insert(seed)
            || !outputs.insert(output)
            || !Path::new(output).is_absolute()
            || !matches!(status, "pending" | "accepted" | "failed")
            || status == "pending" && i + 1 != attempts.len()
            || latest.is_some_and(|pins| pins != &attempt["source_sha256"])
            || !ledger["training_seed_ranges"]
                .as_array()
                .is_some_and(|ranges| ranges.iter().any(|r| r == &json!([seed, seed])))
        {
            return Err("CPU attempt lineage/status/exclusion changed".into());
        }
        latest = Some(if status == "accepted" {
            &attempt["native_output_sha256"]
        } else {
            &attempt["source_sha256"]
        });
        if latest.is_none_or(|pins| pins.as_object().is_none_or(|p| p.len() != 4)) {
            return Err("CPU attempt lacks exact native source pins".into());
        }
    }
    Ok(ledger)
}
impl Drop for LaneLock {
    fn drop(&mut self) {
        drop(self.1.take());
        let _ = fs::remove_file(&self.0);
    }
}

pub(super) struct CareSeedReservation {
    pub provenance: Value,
    ledger_path: PathBuf,
    output: PathBuf,
    seed: u64,
    _lock: LaneLock,
}

impl CareSeedReservation {
    pub fn reserve(config: &CpuContinuationConfig, request: &CpuCareRequest) -> Result<Self> {
        let previous = config
            .previous
            .as_deref()
            .ok_or("care continuation lacks source")?;
        let history = request
            .seed_history
            .as_deref()
            .ok_or("care continuation requires seed history")?;
        let lane = config.output.parent().ok_or("care output has no lane")?;
        // A portable clone is not the original PC lane. Never alter a running
        // or surviving GPU campaign's lock, ledger or acceptance semantics.
        if lane.join(".mixed-care-seed-reservations.json").exists() {
            return Err(
                "CPU care requires a dedicated CPU lane; preserve the original mixed-care lane"
                    .into(),
            );
        }
        let admitted = admit_native_cpu_care_seed(
            previous,
            history,
            &config.output,
            config.seed,
            request.known_exclusions,
        )?;
        fs::create_dir_all(lane)?;
        let lock = lock_lane(lane)?;
        let provenance = admit_native_cpu_care_seed(
            previous,
            history,
            &config.output,
            config.seed,
            request.known_exclusions,
        )?;
        if provenance != admitted {
            return Err("care source/history changed before reservation".into());
        }
        let ledger_path = lane.join(LEDGER);
        let mut ledger = if ledger_path.exists() {
            read_ledger(&ledger_path)?
        } else {
            json!({"schema":1,"kind":"grab-food-seed-ledger","cpu_care_schema":1,
                "training_seed_ranges":[],"assessment_seeds":[],"allocations":[],"cpu_care_attempts":[]})
        };
        let attempts = ledger["cpu_care_attempts"]
            .as_array()
            .ok_or("invalid CPU care attempt ledger")?;
        if attempts.iter().any(|a| a["status"] == "pending") {
            return Err("CPU lane has an uncertain pending attempt; reconcile its sealed output before any new dispatch".into());
        }
        if attempts
            .iter()
            .any(|a| a["seed"].as_u64() == Some(config.seed) || a["output"] == json!(config.output))
        {
            return Err("care seed/output already attempted; no retries or overwrites".into());
        }
        let expected = attempts
            .iter()
            .rev()
            .find(|a| a["status"] == "accepted")
            .map(|a| &a["native_output_sha256"])
            .or_else(|| attempts.first().map(|a| &a["source_sha256"]));
        if expected.is_some_and(|pins| *pins != provenance["source_sha256"]) {
            return Err(
                "CPU care must continue the latest accepted founder and full optimizers".into(),
            );
        }
        ledger["training_seed_ranges"]
            .as_array_mut()
            .ok_or("invalid CPU seed ranges")?
            .push(json!([config.seed, config.seed]));
        ledger["cpu_care_attempts"].as_array_mut().unwrap().push(json!({"status":"pending","seed":config.seed,
            "output":config.output,"source":previous,"source_sha256":provenance["source_sha256"],
            "source_policy_version":provenance["native_source_receipt"]["policy_version"],
            "lesson":request.lesson,"decisions":request.decisions,"seed_admission":provenance["seed_admission"]}));
        write_atomic(&ledger_path, &ledger)?;
        Ok(Self {
            provenance,
            ledger_path,
            output: config.output.clone(),
            seed: config.seed,
            _lock: lock,
        })
    }
    fn update(&self, status: &str, receipt: Value) -> Result<()> {
        let mut ledger = read_ledger(&self.ledger_path)?;
        let attempt = ledger["cpu_care_attempts"]
            .as_array_mut()
            .ok_or("CPU attempt ledger missing")?
            .last_mut()
            .ok_or("CPU attempt missing")?;
        if attempt["status"] != "pending"
            || attempt["seed"].as_u64() != Some(self.seed)
            || attempt["output"] != json!(self.output)
        {
            return Err("CPU attempt identity/status changed under owner lock".into());
        }
        attempt["status"] = json!(status);
        attempt["result"] = receipt.clone();
        if status == "accepted" {
            attempt["native_output_sha256"] = receipt["native_source_sha256"].clone();
        }
        write_atomic(&self.ledger_path, &ledger)
    }
    pub fn accept(&mut self, receipt: &Value) -> Result<()> {
        validate_care_output(&self.output, receipt)?;
        let ledger = read_ledger(&self.ledger_path)?;
        let attempt = ledger["cpu_care_attempts"]
            .as_array()
            .unwrap()
            .last()
            .unwrap();
        validate_attempt_output(attempt, receipt)?;
        let pins = [
            "cpu-cycle.json",
            "trained.alife-foundation",
            "actor-checkpoint.json",
            "value-checkpoint.json",
        ]
        .into_iter()
        .map(|name| {
            Ok((
                name.to_owned(),
                json!(sha(&fs::read(self.output.join(name))?)),
            ))
        })
        .collect::<Result<serde_json::Map<_, _>>>()?;
        self.update("accepted", json!({"native_source_sha256":pins,"policy_version":receipt["policy_version"],
            "actor_optimizer_step":receipt["actor_optimizer_step"],"value_optimizer_step":receipt["value_optimizer_step"]}))
    }
    pub fn fail(&mut self, reason: &str) -> Result<()> {
        // The exclusion survives even a zero-dose failure. A complete seal with
        // a failed acceptance write stays pending for explicit reconciliation.
        if self.output.join("cpu-cycle.json").exists() {
            return Err(
                "CPU output acquired a seal despite failure; keep pending for reconciliation"
                    .into(),
            );
        }
        self.update("failed", json!({"error":reason,"seed_consumed":true}))
    }
}

fn validate_attempt_output(attempt: &Value, receipt: &Value) -> Result<()> {
    if receipt["seed"] != attempt["seed"]
        || receipt["source_policy_version"] != attempt["source_policy_version"]
        || receipt["seed_provenance"]["source_sha256"] != attempt["source_sha256"]
        || receipt["episode"]["lesson"] != attempt["lesson"]
        || receipt["requested_training_ticks"] != attempt["decisions"]
        || receipt["seed_admission"] != attempt["seed_admission"]
    {
        return Err("sealed CPU output differs from its reserved attempt".into());
    }
    Ok(())
}

/// No dispatch occurs here. An unsealed interrupted attempt is terminally failed;
/// a complete seal must pass exactly the same acceptance path as a normal run.
pub(super) fn reconcile(lane: &Path) -> Result<Value> {
    let lane = lane.canonicalize()?;
    let lock = lock_lane(&lane)?;
    let ledger_path = lane.join(LEDGER);
    let ledger = read_ledger(&ledger_path)?;
    let attempt = ledger["cpu_care_attempts"]
        .as_array()
        .ok_or("missing attempts")?
        .last()
        .filter(|a| a["status"] == "pending")
        .ok_or("no pending CPU attempt")?;
    let output = PathBuf::from(attempt["output"].as_str().ok_or("invalid pending output")?);
    if output.parent() != Some(lane.as_path()) || output.is_symlink() {
        return Err("pending output escaped its dedicated lane".into());
    }
    let mut reservation = CareSeedReservation {
        provenance: Value::Null,
        ledger_path,
        output,
        seed: attempt["seed"].as_u64().ok_or("missing pending seed")?,
        _lock: lock,
    };
    if reservation.output.join("cpu-cycle.json").exists() {
        let (_, receipt) =
            read_native_cpu_care_evidence(&reservation.output.join("cpu-cycle.json"))?;
        reservation.accept(&receipt)?;
        Ok(
            json!({"status":"accepted","output":reservation.output,"seed":reservation.seed,"dispatch":false}),
        )
    } else {
        reservation.fail("interrupted unsealed episode reconciled; seed remains excluded")?;
        Ok(
            json!({"status":"failed","output":reservation.output,"seed":reservation.seed,"dispatch":false}),
        )
    }
}

pub(super) fn validate_care_output(output: &Path, receipt: &Value) -> Result<()> {
    crate::validate_foundation_training_group_source(output)?;
    let (_, actual) = read_native_cpu_care_evidence(&output.join("cpu-cycle.json"))?;
    let n = receipt["requested_training_ticks"]
        .as_u64()
        .filter(|n| (1..=128).contains(n))
        .ok_or("CPU care dose missing")?;
    if actual != *receipt
        || receipt["complete"] != true
        || receipt["backend"] != "native-cpu"
        || receipt["objective_state_reset"] != false
        || receipt["biological_objective_version"] != 2
        || receipt["critic_objective_version"] != 1
        || receipt["policy_version"].as_u64()
            != receipt["source_policy_version"]
                .as_u64()
                .and_then(|p| p.checked_add(1))
        || receipt["training_ticks"] != n
        || receipt["trained_replay_rows"] != n
        || receipt["captured_replay_rows"] != n + 1
        || receipt["bootstrap_replay_rows"] != 1
        || receipt["episode"]["kind"] != "care-first-cpu-episode"
        || receipt["terminal_gpu_individual_restore_proven"] != false
    {
        return Err(
            "CPU care output has inconsistent sealed identity or decision/bootstrap dose".into(),
        );
    }
    let artifacts = receipt["episode_artifact_sha256"]
        .as_object()
        .ok_or("care artifacts missing")?;
    for name in [
        "care-replay.json",
        "care-trace.json",
        "care-prior-lives.json",
        "care-episode.json",
        "collector-checkpoint.json",
        "collection-policy.alife-foundation",
        "collection-compiler-inputs.json",
        "collection-actor-checkpoint.json",
        "initial-value-checkpoint.json",
    ] {
        if !artifacts.contains_key(name) {
            return Err("required CPU care artifact missing".into());
        }
    }
    for (name, expected) in receipt["episode_artifact_sha256"]
        .as_object()
        .ok_or("care artifacts missing")?
    {
        if Path::new(name).components().count() != 1
            || !matches!(
                Path::new(name).components().next(),
                Some(std::path::Component::Normal(_))
            )
            || expected.as_str() != Some(sha(&fs::read(output.join(name))?).as_str())
        {
            return Err("CPU care artifact changed".into());
        }
    }
    if read_native_cpu_care_evidence(&output.join("care-episode.json"))?.1 != receipt["episode"] {
        return Err("CPU care episode metadata differs from seal".into());
    }
    let imported: FoundationTrainerCheckpoint =
        serde_json::from_slice(&fs::read(output.join("collection-actor-checkpoint.json"))?)?;
    let imported_value: PpoValueHeadCheckpoint =
        serde_json::from_slice(&fs::read(output.join("initial-value-checkpoint.json"))?)?;
    let inputs: alife_core::PhenotypeCompilerInputs =
        serde_json::from_slice(&fs::read(output.join("collection-compiler-inputs.json"))?)?;
    let phenotype = alife_core::PhenotypeCompiler::compile_validated(
        &inputs,
        &alife_core::BrainCapacityClass::n2048(),
    )?;
    let asset = alife_core::FoundationWeightAsset::decode_canonical(&fs::read(
        output.join("collection-policy.alife-foundation"),
    )?)?;
    if inputs.n2048_candidate_asset() != Some(&asset) {
        return Err("collection compiler asset differs".into());
    }
    let founder = alife_training::cpu::CpuFoundationTrainer::from_checkpoint(
        phenotype,
        asset,
        imported.clone(),
    )?;
    let checkpoint = serde_json::from_slice(&fs::read(output.join("collector-checkpoint.json"))?)?;
    let _restored = alife_training::cpu::CpuCanonicalHost::from_checkpoint(checkpoint, &founder)?;
    let source = output.join("source-native");
    let source_pins = receipt["source_native_sha256"]
        .as_object()
        .ok_or("missing native source pins")?;
    let ancestry_pins = receipt["seed_provenance"]["source_sha256"]
        .as_object()
        .ok_or("missing predecessor pins")?;
    for (name, expected) in ancestry_pins {
        if source_pins.get(name) != Some(expected) {
            return Err("retained source pins differ from reserved predecessor".into());
        }
    }
    for (name, expected) in source_pins {
        if Path::new(name).components().count() != 1
            || !matches!(
                Path::new(name).components().next(),
                Some(std::path::Component::Normal(_))
            )
            || expected.as_str() != Some(sha(&fs::read(source.join(name))?).as_str())
        {
            return Err("retained native source changed".into());
        }
    }
    if receipt["source_policy_version"] == 0 && receipt["fixture"] == true && source_pins.is_empty()
    {
        // Only the explicit root development fixture has no retained predecessor.
    } else {
        if !artifacts.contains_key("care-seed-admission.json") {
            return Err("care admission artifact missing".into());
        }
        let provenance = read_native_cpu_care_evidence(&output.join("care-seed-admission.json"))?.1;
        if provenance["source_sha256"] != receipt["seed_provenance"]["source_sha256"]
            || provenance["seed_admission"] != receipt["seed_admission"]
            || provenance["world_seed"] != receipt["seed"]
        {
            return Err("care admission provenance differs".into());
        }
        let actor: FoundationTrainerCheckpoint =
            serde_json::from_slice(&fs::read(source.join("actor-checkpoint.json"))?)?;
        let value: PpoValueHeadCheckpoint =
            serde_json::from_slice(&fs::read(source.join("value-checkpoint.json"))?)?;
        if actor != imported
            || value != imported_value
            || fs::read(source.join("trained.alife-foundation"))?
                != fs::read(output.join("collection-policy.alife-foundation"))?
        {
            return Err(
                "CPU care changed retained founder/full optimizer state before collection".into(),
            );
        }
    }
    for (initial, final_age, updates) in [
        (
            imported.optimizer_step,
            &receipt["actor_optimizer_step"],
            &receipt["actor_optimizer_updates"],
        ),
        (
            imported_value.optimizer_step,
            &receipt["value_optimizer_step"],
            &receipt["value_optimizer_updates"],
        ),
    ] {
        if updates
            .as_u64()
            .is_none_or(|u| u == 0 || u64::from(initial).checked_add(u) != final_age.as_u64())
        {
            return Err("CPU care optimizer age/update evidence disagrees".into());
        }
    }
    Ok(())
}
