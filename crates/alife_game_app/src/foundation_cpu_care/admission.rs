//! Dedicated CPU lane: exact history, durable exclusions and terminal attempts.
use super::{CpuCareRequest, Result};
use crate::foundation_training_cycle::{
    admit_native_cpu_care_collection_seed, admit_native_cpu_care_seed,
    read_native_cpu_care_evidence,
};
use alife_training::{
    cpu::{
        admit_cpu_collector_shards, read_cpu_collector_output, CpuCanonicalCheckpoint,
        CpuCanonicalHost, CpuCollectionSnapshotV1, CpuCollectorAssignmentV1, CpuContinuationConfig,
        CpuFoundationTrainer, CpuValueHead, CPU_COLLECTOR_COMMIT_FILE,
    },
    FoundationTrainerCheckpoint, FoundationTrainingBackendV1, PpoConfig, PpoValueHeadCheckpoint,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub(super) const LEDGER: &str = ".grab-food-seed-reservations.json";
const LOCK: &str = ".eat-held-food-owner.lock";

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(super) fn write_atomic(path: &Path, value: &Value) -> Result<()> {
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
            || !matches!(status, "pending" | "accepted" | "failed" | "collected")
            || status == "pending" && i + 1 != attempts.len()
            || latest.is_some_and(|pins| pins != &attempt["source_sha256"])
            || !ledger["training_seed_ranges"]
                .as_array()
                .is_some_and(|ranges| ranges.iter().any(|r| r == &json!([seed, seed])))
        {
            return Err("CPU attempt lineage/status/exclusion changed".into());
        }
        if attempt.get("collection").is_some() {
            collection_binding(attempt)?;
            if status == "accepted" {
                return Err("CPU collection cannot advance the accepted founder".into());
            }
        } else if status == "collected" {
            return Err("CPU collected attempt lacks its reserved assignment and plan".into());
        }
        if status == "collected" && !valid_sha(&attempt["result"]["collector_commit_sha256"]) {
            return Err("CPU collected attempt lacks its exact collector commit pin".into());
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
    pub fn reserve_collection(
        config: &CpuContinuationConfig,
        request: &CpuCareRequest,
        assignment: &CpuCollectorAssignmentV1,
        plan_sha256: [u8; 32],
    ) -> Result<Self> {
        if config.fixture
            || request.fixture
            || config.seed != request.seed
            || config.ticks
                != request
                    .decisions
                    .checked_add(1)
                    .ok_or("care dose overflow")?
            || config.temperature.to_bits() != request.temperature.to_bits()
        {
            return Err("CPU collection reservation differs from its care request".into());
        }
        let binding = json!({"assignment":assignment,"plan_sha256":plan_sha256,
            "temperature":config.temperature,"provenance_sha256":"0".repeat(64)});
        collection_binding(&json!({"seed":config.seed,"lesson":request.lesson,
            "decisions":request.decisions,"collection":binding}))?;
        let reservation = Self::reserve_inner(config, request, true)?;
        let mut ledger = read_ledger(&reservation.ledger_path)?;
        let attempt = ledger["cpu_care_attempts"]
            .as_array_mut()
            .and_then(|attempts| attempts.last_mut())
            .ok_or("CPU collection attempt missing")?;
        if attempt["status"] != "pending"
            || attempt["seed"] != config.seed
            || attempt["output"] != json!(config.output)
        {
            return Err("CPU collection attempt changed before binding".into());
        }
        attempt["collection"] = binding;
        attempt["collection"]["provenance_sha256"] =
            json!(sha(&serde_json::to_vec(&reservation.provenance)?));
        write_atomic(&reservation.ledger_path, &ledger)?;
        Ok(reservation)
    }

    pub fn reserve(config: &CpuContinuationConfig, request: &CpuCareRequest) -> Result<Self> {
        Self::reserve_inner(config, request, false)
    }

    fn reserve_inner(
        config: &CpuContinuationConfig,
        request: &CpuCareRequest,
        collection: bool,
    ) -> Result<Self> {
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
        let admit = if collection {
            admit_native_cpu_care_collection_seed
        } else {
            admit_native_cpu_care_seed
        };
        let admitted = admit(
            previous,
            history,
            &config.output,
            config.seed,
            request.known_exclusions,
        )?;
        fs::create_dir_all(lane)?;
        let lock = lock_lane(lane)?;
        let provenance = admit(
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
        if attempt.get("collection").is_some() {
            return Err("CPU collection reservation cannot accept a trained founder".into());
        }
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
        if has_seal(&self.output, "cpu-cycle.json")?
            || has_seal(&self.output, CPU_COLLECTOR_COMMIT_FILE)?
        {
            return Err(
                "CPU output acquired a seal despite failure; keep pending for reconciliation"
                    .into(),
            );
        }
        self.update("failed", json!({"error":reason,"seed_consumed":true}))
    }

    pub fn collected(&mut self) -> Result<Value> {
        let ledger = read_ledger(&self.ledger_path)?;
        let attempt = ledger["cpu_care_attempts"]
            .as_array()
            .and_then(|attempts| attempts.last())
            .ok_or("CPU collection attempt missing")?;
        if attempt["status"] != "pending"
            || attempt["seed"] != self.seed
            || attempt["output"] != json!(self.output)
        {
            return Err("CPU collection identity/status changed under owner lock".into());
        }
        let receipt = validate_collection_output(attempt, &self.output)?;
        self.update("collected", receipt.clone())?;
        Ok(receipt)
    }
}

fn has_seal(output: &Path, name: &str) -> Result<bool> {
    match fs::symlink_metadata(output.join(name)) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn valid_sha(value: &Value) -> bool {
    value.as_str().is_some_and(|text| {
        text.len() == 64
            && text
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn collection_binding(attempt: &Value) -> Result<(CpuCollectorAssignmentV1, [u8; 32], f32)> {
    let binding = attempt
        .get("collection")
        .ok_or("CPU attempt is not collection-bound")?;
    let assignment: CpuCollectorAssignmentV1 =
        serde_json::from_value(binding["assignment"].clone())?;
    let plan: [u8; 32] = serde_json::from_value(binding["plan_sha256"].clone())?;
    let temperature = binding["temperature"]
        .as_f64()
        .ok_or("missing collection temperature")? as f32;
    let seed = attempt["seed"].as_u64().ok_or("missing collection seed")?;
    let dose = attempt["decisions"]
        .as_u64()
        .filter(|n| (1..=128).contains(n))
        .ok_or("invalid collection decision dose")?;
    let text = |s: &str| !s.trim().is_empty() && s.len() <= 256;
    if plan == [0; 32]
        || !temperature.is_finite()
        || temperature <= 0.
        || !valid_sha(&binding["provenance_sha256"])
        || !text(&assignment.collector_run_id)
        || !text(&assignment.producer_revision)
        || !text(&assignment.lesson)
        || assignment.backend != FoundationTrainingBackendV1::NativeCpu
        || seed == 0
        || assignment.world_seed != seed
        || assignment.sampling_seed != (seed as u32) ^ ((seed >> 32) as u32)
        || assignment.sampling_counter_start != 0
        || u64::from(assignment.sampling_counter_end) != dose + 1
        || assignment.episode_config_sha256 == [0; 32]
        || attempt["lesson"] != assignment.lesson
    {
        return Err("CPU collection binding differs from its reserved episode".into());
    }
    Ok((assignment, plan, temperature))
}

fn validate_collection_output(attempt: &Value, output: &Path) -> Result<Value> {
    let (assignment, plan, temperature) = collection_binding(attempt)?;
    if has_seal(output, "cpu-cycle.json")? {
        return Err("CPU care output mixes trained and collected receipt domains".into());
    }
    let shard = read_cpu_collector_output(output)?;
    let (commit_bytes, _) = read_native_cpu_care_evidence(&output.join(CPU_COLLECTOR_COMMIT_FILE))?;
    let (_, metadata) = read_native_cpu_care_evidence(&output.join("collection-metadata.json"))?;
    let (_, episode) = read_native_cpu_care_evidence(&output.join("care-episode.json"))?;
    let (provenance_bytes, provenance) =
        read_native_cpu_care_evidence(&output.join("care-seed-admission.json"))?;
    let source_version = attempt["source_policy_version"]
        .as_u64()
        .ok_or("collection attempt lacks source version")?;
    let version = source_version
        .checked_add(1)
        .ok_or("collection policy version overflow")?;
    let dose = attempt["decisions"]
        .as_u64()
        .ok_or("collection dose missing")? as usize;
    if shard.assignment != assignment
        || metadata["schema_version"] != 1
        || metadata["mode"] != "native_cpu_collect_only"
        || metadata["backend"] != "native-cpu"
        || metadata["source_policy_version"] != source_version
        || metadata["collection_policy_version"] != version
        || metadata["offline_actor_optimizer_updates"] != 0
        || metadata["offline_value_optimizer_updates"] != 0
        || metadata["world_seed"] != assignment.world_seed
        || metadata["seed_admission"] != attempt["seed_admission"]
        || metadata["episode"] != episode
        || episode["kind"] != "care-first-cpu-episode"
        || episode["lesson"] != attempt["lesson"]
        || provenance["source_sha256"] != attempt["source_sha256"]
        || provenance["world_seed"] != assignment.world_seed
        || provenance["seed_admission"] != attempt["seed_admission"]
        || json!(sha(&provenance_bytes)) != attempt["collection"]["provenance_sha256"]
        || shard.trajectories.len() != 1
    {
        return Err("sealed CPU collection differs from its bound reservation/provenance".into());
    }
    let trajectory = &shard.trajectories[0];
    if !trajectory.final_record.lifecycle().is_alive() || trajectory.decisions.len() != dose + 1 {
        return Err(
            "CPU care collection requires its complete decision dose and real living bootstrap"
                .into(),
        );
    }
    let recorded = episode["raw_replay_recorded"]
        .as_bool()
        .ok_or("collection replay recording metadata missing")?;
    if recorded != output.join("care-replay.json").is_file() {
        return Err("CPU collection diagnostic replay differs from metadata".into());
    }
    let (_, trace) = read_native_cpu_care_evidence(&output.join("care-trace.json"))?;
    let trace_rows = trace["rows"]
        .as_array()
        .ok_or("collection trace rows missing")?;
    if trace["backend"] != "native-cpu"
        || trace["lesson"] != attempt["lesson"]
        || trace_rows.len() != dose + 1
        || trace_rows
            .iter()
            .enumerate()
            .any(|(i, row)| row["loss_row"] != (i < dose))
    {
        return Err("CPU care trace does not preserve the reserved loss/bootstrap boundary".into());
    }
    let source = output.join("source-native");
    let pins = attempt["source_sha256"]
        .as_object()
        .filter(|pins| pins.len() == 4)
        .ok_or("collection predecessor pins missing")?;
    let receipt_name = ["cpu-cycle.json", "cycle.json", "group50.json"]
        .into_iter()
        .filter(|name| pins.contains_key(*name))
        .collect::<Vec<_>>();
    let [receipt_name] = receipt_name.as_slice() else {
        return Err("collection predecessor receipt domain is ambiguous".into());
    };
    for name in [
        *receipt_name,
        "trained.alife-foundation",
        "actor-checkpoint.json",
        "value-checkpoint.json",
    ] {
        let expected = pins
            .get(name)
            .filter(|value| valid_sha(value))
            .ok_or("collection predecessor pin missing or invalid")?;
        if json!(sha(&fs::read(source.join(name))?)) != *expected {
            return Err("collection retained predecessor bytes changed".into());
        }
    }
    crate::validate_foundation_training_group_source(&source)?;
    let (_, native) = read_native_cpu_care_evidence(&source.join(receipt_name))?;
    // Raw receipt pins and the raw provenance artifact bind both originals.
    // Timing f64 values need not survive nested JSON reserialization to prove
    // source identity; compare its continuation-critical scalar fields here.
    let reserved_native = &provenance["native_source_receipt"];
    if [
        "policy_version",
        "seed",
        "founder_seed_base",
        "biological_objective_version",
        "critic_objective_version",
        "objective_state_reset",
        "mode",
        "backend",
        "receipt_filename",
        "handoff_filename",
    ]
    .iter()
    .any(|field| native[*field] != reserved_native[*field])
        || native["policy_version"] != source_version
    {
        return Err("collection source receipt differs from reservation provenance".into());
    }
    let imported: FoundationTrainerCheckpoint =
        serde_json::from_slice(&fs::read(output.join("collection-actor-checkpoint.json"))?)?;
    let imported_value: PpoValueHeadCheckpoint =
        serde_json::from_slice(&fs::read(output.join("initial-value-checkpoint.json"))?)?;
    let source_actor: FoundationTrainerCheckpoint =
        serde_json::from_slice(&fs::read(source.join("actor-checkpoint.json"))?)?;
    let source_value: PpoValueHeadCheckpoint =
        serde_json::from_slice(&fs::read(source.join("value-checkpoint.json"))?)?;
    let policy_bytes = fs::read(output.join("collection-policy.alife-foundation"))?;
    if serde_json::to_vec(&imported)? != serde_json::to_vec(&source_actor)?
        || serde_json::to_vec(&imported_value)? != serde_json::to_vec(&source_value)?
        || policy_bytes != fs::read(source.join("trained.alife-foundation"))?
    {
        return Err(
            "CPU collection changed its frozen founder or full actor/value optimizers".into(),
        );
    }
    let inputs: alife_core::PhenotypeCompilerInputs =
        serde_json::from_slice(&fs::read(output.join("collection-compiler-inputs.json"))?)?;
    let phenotype = alife_core::PhenotypeCompiler::compile_validated(
        &inputs,
        &alife_core::BrainCapacityClass::n2048(),
    )?;
    let asset = alife_core::FoundationWeightAsset::decode_canonical(&policy_bytes)?;
    if inputs.n2048_candidate_asset() != Some(&asset) {
        return Err("CPU collection compiler asset differs".into());
    }
    let founder = CpuFoundationTrainer::from_checkpoint(phenotype, asset, imported.clone())?;
    let value = CpuValueHead::from_checkpoint(imported_value.clone())?;
    let config = PpoConfig {
        temperature,
        ..Default::default()
    };
    if CpuCollectionSnapshotV1::capture(&founder, &value, &inputs, config, version)?
        != shard.snapshot
    {
        return Err(
            "CPU collection snapshot differs from its frozen actor/value/compiler/config".into(),
        );
    }
    let checkpoint: CpuCanonicalCheckpoint =
        serde_json::from_slice(&fs::read(output.join("collector-checkpoint.json"))?)?;
    if checkpoint.sampling_seed != assignment.sampling_seed
        || checkpoint.sampling_counter != assignment.sampling_counter_end
        || checkpoint.world.config.deterministic_seed != assignment.world_seed
    {
        return Err("CPU collection private host sampling provenance differs".into());
    }
    let restored = CpuCanonicalHost::from_checkpoint(checkpoint, &founder)?;
    let record = restored
        .world()
        .organism_registry()
        .get(trajectory.final_record.organism_id())
        .ok_or("CPU collection private host lost its trajectory organism")?;
    if serde_json::to_vec(record)? != serde_json::to_vec(&trajectory.final_record)? {
        return Err("CPU collection trajectory and private host state differ".into());
    }
    let (_, lives) = read_native_cpu_care_evidence(&output.join("care-prior-lives.json"))?;
    if lives["collection_policy_sha256"] != sha(&policy_bytes)
        || lives["world_tick"] != json!(restored.world().tick())
    {
        return Err(
            "CPU collection private prior archive differs from its frozen host boundary".into(),
        );
    }
    let shard_payload_sha256 = shard.payload_sha256;
    // This reuses native probability, replay, GAE and terminal/bootstrap
    // validation without calling the cohort's separate train method.
    let cohort = admit_cpu_collector_shards(
        &founder,
        &value,
        &inputs,
        config,
        version,
        std::slice::from_ref(&assignment),
        vec![shard],
        1,
        0,
    )?;
    if cohort
        .windows()
        .iter()
        .map(|window| window.actor.batch.len())
        .sum::<usize>()
        != dose
    {
        return Err("CPU collection admitted loss rows differ from reserved dose".into());
    }
    if fs::read(output.join(CPU_COLLECTOR_COMMIT_FILE))? != commit_bytes {
        return Err("CPU collector commit changed during reservation validation".into());
    }
    Ok(
        json!({"collector_commit_sha256":sha(&commit_bytes),"shard_payload_sha256":shard_payload_sha256,
        "plan_sha256":plan,"source_sha256":attempt["source_sha256"],"source_policy_version":source_version,
        "collection_policy_version":version,"seed":assignment.world_seed,"lesson":assignment.lesson,
        "training_rows":dose,"captured_rows":dose+1,"bootstrap_rows":1,
        "offline_actor_optimizer_updates":0,"offline_value_optimizer_updates":0,
        "actor_optimizer_step":imported.optimizer_step,"value_optimizer_step":imported_value.optimizer_step}),
    )
}

/// Read-only reuse of the consumed reservation; this does not seed-admit again.
pub(super) fn validate_collected_attempt(
    lane: &Path,
    output: &Path,
    assignment: &CpuCollectorAssignmentV1,
    plan_sha256: [u8; 32],
) -> Result<Value> {
    let lane = lane.canonicalize()?;
    if output.is_symlink() {
        return Err("collected CPU output must not be a symlink".into());
    }
    let output = output.canonicalize()?;
    if output.parent() != Some(lane.as_path()) {
        return Err("collected CPU output escaped its dedicated lane".into());
    }
    let attempts = collection_attempts(&lane)?;
    let attempt = attempts
        .iter()
        .find(|attempt| attempt["output"] == json!(output))
        .filter(|attempt| attempt["status"] == "collected")
        .ok_or("no terminal collected reservation for CPU output")?;
    let (bound_assignment, bound_plan, _) = collection_binding(attempt)?;
    if &bound_assignment != assignment || bound_plan != plan_sha256 {
        return Err("collected CPU reservation differs from requested assignment/plan".into());
    }
    let receipt = validate_collection_output(attempt, &output)?;
    if receipt != attempt["result"] {
        return Err("collected CPU receipt differs from durable reservation result".into());
    }
    Ok(receipt)
}

pub(super) fn collection_attempts(lane: &Path) -> Result<Vec<Value>> {
    let path = lane.join(LEDGER);
    if !has_seal(lane, LEDGER)? {
        return Ok(Vec::new());
    }
    Ok(read_ledger(&path)?["cpu_care_attempts"]
        .as_array()
        .ok_or("invalid CPU attempt ledger")?
        .clone())
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
    let trained = has_seal(&reservation.output, "cpu-cycle.json")?;
    let collected = has_seal(&reservation.output, CPU_COLLECTOR_COMMIT_FILE)?;
    if trained && collected {
        return Err("pending CPU output mixes trained and collected receipt domains".into());
    }
    if collected {
        let receipt = reservation.collected()?;
        Ok(
            json!({"status":"collected","output":reservation.output,"seed":reservation.seed,
            "dispatch":false,"result":receipt}),
        )
    } else if trained {
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
    // Earlier receipts always recorded the replay. Absence is only allowed by
    // an explicit new receipt field, never inferred from a missing file.
    let recorded = match receipt["episode"].get("raw_replay_recorded") {
        None => true,
        Some(Value::Bool(value)) => *value,
        _ => return Err("invalid raw replay recording metadata".into()),
    };
    if recorded != artifacts.contains_key("care-replay.json")
        || recorded != output.join("care-replay.json").is_file()
    {
        return Err("raw replay recording does not match its seal".into());
    }
    for name in [
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct Lane(PathBuf);
    impl Lane {
        fn new() -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "alife-care-collection-admission-{}-{}-{}",
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
    impl Drop for Lane {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    fn pins(byte: char) -> Value {
        let digest = byte.to_string().repeat(64);
        json!({"cpu-cycle.json":digest,"trained.alife-foundation":digest,
            "actor-checkpoint.json":digest,"value-checkpoint.json":digest})
    }

    fn attempt(lane: &Path, name: &str, seed: u64) -> Value {
        json!({"status":"pending","seed":seed,"output":lane.join(name),
            "source":lane.join("synthetic-source"),"source_sha256":pins('a'),
            "source_policy_version":1,"lesson":"synthetic-lesson","decisions":2,
            "seed_admission":{"policy":"strict","historical_freshness":"operator-attested-complete"},
            "collection":{"assignment":CpuCollectorAssignmentV1 {
                collector_run_id:format!("synthetic-{seed}"),backend:FoundationTrainingBackendV1::NativeCpu,
                producer_revision:"synthetic-v1".into(),world_seed:seed,
                sampling_seed:(seed as u32)^((seed>>32)as u32),sampling_counter_start:0,sampling_counter_end:3,
                lesson:"synthetic-lesson".into(),episode_config_sha256:[1;32]},
                "plan_sha256":vec![2u8;32],"temperature":1.0,"provenance_sha256":"a".repeat(64)}})
    }

    fn ledger(lane: &Path, attempts: Vec<Value>) -> Value {
        let ranges = attempts
            .iter()
            .map(|attempt| json!([attempt["seed"], attempt["seed"]]))
            .collect::<Vec<_>>();
        let value = json!({"schema":1,"kind":"grab-food-seed-ledger","cpu_care_schema":1,
            "training_seed_ranges":ranges,"assessment_seeds":[],"allocations":[],"cpu_care_attempts":attempts});
        write_atomic(&lane.join(LEDGER), &value).unwrap();
        value
    }

    #[test]
    fn collected_attempt_keeps_original_source_pins_for_the_next_dispatch() {
        let lane = Lane::new();
        let mut collected = attempt(&lane.0, "first", 11);
        collected["status"] = json!("collected");
        collected["result"] = json!({"collector_commit_sha256":"b".repeat(64)});
        // Even a stray native-output field cannot advance collection lineage.
        collected["native_output_sha256"] = pins('b');
        let pending = attempt(&lane.0, "second", 12);
        ledger(&lane.0, vec![collected.clone(), pending.clone()]);
        assert_eq!(collection_attempts(&lane.0).unwrap().len(), 2);
        let mut changed = pending;
        changed["source_sha256"] = pins('b');
        ledger(&lane.0, vec![collected, changed]);
        assert!(collection_attempts(&lane.0).is_err());
    }

    #[test]
    fn collected_requires_bound_plan_and_collection_cannot_be_accepted() {
        let lane = Lane::new();
        let mut item = attempt(&lane.0, "first", 11);
        item["status"] = json!("collected");
        item["result"] = json!({"collector_commit_sha256":"b".repeat(64)});
        item.as_object_mut().unwrap().remove("collection");
        ledger(&lane.0, vec![item]);
        assert!(collection_attempts(&lane.0).is_err());
        let mut item = attempt(&lane.0, "first", 11);
        item["status"] = json!("accepted");
        item["native_output_sha256"] = pins('b');
        ledger(&lane.0, vec![item]);
        assert!(collection_attempts(&lane.0).is_err());
    }

    #[test]
    fn failed_collection_with_any_commit_stays_pending_for_reconciliation() {
        let lane = Lane::new();
        let output = lane.0.join("first");
        fs::create_dir(&output).unwrap();
        let before = ledger(&lane.0, vec![attempt(&lane.0, "first", 11)]);
        fs::write(output.join(CPU_COLLECTOR_COMMIT_FILE), b"{}").unwrap();
        let mut reservation = CareSeedReservation {
            provenance: Value::Null,
            ledger_path: lane.0.join(LEDGER),
            output: output.clone(),
            seed: 11,
            _lock: lock_lane(&lane.0).unwrap(),
        };
        assert!(reservation
            .fail("synthetic interrupted acknowledgement")
            .is_err());
        assert_eq!(read_ledger(&lane.0.join(LEDGER)).unwrap(), before);
        assert_eq!(
            fs::read(output.join(CPU_COLLECTOR_COMMIT_FILE)).unwrap(),
            b"{}"
        );
        fs::remove_file(output.join(CPU_COLLECTOR_COMMIT_FILE)).unwrap();
        reservation.fail("synthetic unsealed failure").unwrap();
        let failed = read_ledger(&lane.0.join(LEDGER)).unwrap();
        assert_eq!(failed["cpu_care_attempts"][0]["status"], "failed");
        assert_eq!(failed["training_seed_ranges"], json!([[11, 11]]));
    }

    #[test]
    fn reconciliation_preserves_mixed_seals_and_unbound_collector_evidence() {
        let lane = Lane::new();
        let output = lane.0.join("first");
        fs::create_dir(&output).unwrap();
        let before = ledger(&lane.0, vec![attempt(&lane.0, "first", 11)]);
        fs::write(output.join(CPU_COLLECTOR_COMMIT_FILE), b"{}").unwrap();
        fs::write(output.join("cpu-cycle.json"), b"{}").unwrap();
        assert!(reconcile(&lane.0).is_err());
        assert_eq!(read_ledger(&lane.0.join(LEDGER)).unwrap(), before);
        fs::remove_file(output.join("cpu-cycle.json")).unwrap();
        let mut unbound = attempt(&lane.0, "first", 11);
        unbound.as_object_mut().unwrap().remove("collection");
        let before = ledger(&lane.0, vec![unbound]);
        assert!(reconcile(&lane.0).is_err());
        assert_eq!(read_ledger(&lane.0.join(LEDGER)).unwrap(), before);
        assert_eq!(
            fs::read(output.join(CPU_COLLECTOR_COMMIT_FILE)).unwrap(),
            b"{}"
        );
    }
}
