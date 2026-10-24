//! Serial care collection from one frozen source, followed by one existing
//! cooperative learner. This is a dispatch parity control, not serial learning.
use super::*;
use crate::foundation_training_cycle::{
    admit_native_cpu_care_seed, read_native_cpu_care_evidence, verify_native_cpu_care_recovery,
};
use alife_training::{
    resolve_isolated_training_output, FoundationTrainingBackendV1, FoundationTrainingHandoffV1,
    PpoConfig,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io,
    path::{Path, PathBuf},
};

const PLAN_FILE: &str = "care-round-plan.json";
const OWNER_FILE: &str = ".care-round-owner.lock";
const ADMISSION_HELPERS: [&str; 5] = [
    "run_mixed_care_serial.py",
    "grab_food_seed_reservations.py",
    "mixed_care_ledger.py",
    "run_eat_held_food_night.py",
    "summarize_ppo_diagnostics.py",
];
const SOURCE_FILES: [&str; 6] = [
    "cpu-cycle.json",
    "trained.alife-foundation",
    "actor-checkpoint.json",
    "value-checkpoint.json",
    "training-handoff.json",
    "phase.txt",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CpuCareRoundPriorV1 {
    DeterministicV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuCareRoundLessonV1 {
    pub collector_run_id: String,
    pub seed: u64,
    pub lesson: FoundationTeacherLesson,
    pub decisions: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuCareRoundSpecV1 {
    pub schema_version: u16,
    pub previous: PathBuf,
    pub seed_history: PathBuf,
    pub output: PathBuf,
    #[serde(default)]
    pub known_exclusions: bool,
    pub temperature: f32,
    pub prior: CpuCareRoundPriorV1,
    pub window_ticks: usize,
    pub burn_in: usize,
    pub effective_batch_size: usize,
    #[serde(default)]
    pub record_diagnostic_replay: bool,
    pub collectors: Vec<CpuCareRoundLessonV1>,
}

/// Immutable recipe approved before any reservation. The coordinator's existing
/// V1 plan is subsequently sealed from these assignments and their actual commits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuCareRoundPlanV1 {
    pub schema_version: u16,
    pub kind: String,
    pub spec: CpuCareRoundSpecV1,
    pub source_sha256: BTreeMap<String, String>,
    pub seed_evidence_sha256: BTreeMap<PathBuf, String>,
    pub executable_sha256: String,
    pub admission_checkout: Option<PathBuf>,
    pub admission_helpers_sha256: BTreeMap<PathBuf, String>,
    pub prior_provider_identity: String,
    pub snapshot: CpuCollectionSnapshotV1,
    pub assignments: Vec<CpuCollectorAssignmentV1>,
    pub payload_sha256: [u8; 32],
}

fn digest(domain: &[u8], value: &impl Serialize) -> Result<[u8; 32]> {
    let mut hash = Sha256::new();
    hash.update(domain);
    serde_json::to_writer(&mut hash, value)?;
    Ok(hash.finalize().into())
}
fn file_sha(path: &Path) -> Result<String> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("care round inputs must be regular nonsymlink files".into());
    }
    let mut hash = Sha256::new();
    io::copy(&mut File::open(path)?, &mut hash)?;
    Ok(format!("{:x}", hash.finalize()))
}
fn qualified_checkout() -> Result<PathBuf> {
    let result = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;
    if !result.status.success() {
        return Err("known-exclusions care round requires its qualified source checkout".into());
    }
    Ok(PathBuf::from(std::str::from_utf8(&result.stdout)?.trim()).canonicalize()?)
}
fn config(spec: &CpuCareRoundSpecV1) -> PpoConfig {
    PpoConfig {
        temperature: spec.temperature,
        ..Default::default()
    }
}
fn portable_run_id(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        && !matches!(lower.as_str(), "con" | "prn" | "aux" | "nul")
        && !(lower.len() == 4
            && (lower.starts_with("com") || lower.starts_with("lpt"))
            && matches!(lower.as_bytes()[3], b'1'..=b'9'))
}
fn check_spec(spec: &CpuCareRoundSpecV1) -> Result<()> {
    let mut seeds = BTreeSet::new();
    let mut ids = BTreeSet::new();
    if spec.schema_version != 1
        || !(1..=16).contains(&spec.collectors.len())
        || spec.window_ticks == 0
        || spec.effective_batch_size == 0
        || spec
            .window_ticks
            .checked_add(spec.burn_in)
            .is_none_or(|n| n > alife_training::MAX_TRAINING_SEQUENCE_TICKS)
    {
        return Err(
            "care round requires schema1, 1..=16 collectors and bounded replay settings".into(),
        );
    }
    config(spec).validate()?;
    for lesson in &spec.collectors {
        if !portable_run_id(&lesson.collector_run_id)
            || !ids.insert(lesson.collector_run_id.to_ascii_lowercase())
            || lesson.seed == 0
            || !seeds.insert(lesson.seed)
            || !(1..=128).contains(&lesson.decisions)
            || !matches!(
                lesson.lesson,
                FoundationTeacherLesson::EatHeldFood
                    | FoundationTeacherLesson::EatReachableFood
                    | FoundationTeacherLesson::Feeding
                    | FoundationTeacherLesson::ObstacleNavigation
                    | FoundationTeacherLesson::RelocatedFood
            )
        {
            return Err("care round requires unique portable run IDs/seeds and explicit supported lesson doses".into());
        }
    }
    if spec
        .collectors
        .windows(2)
        .any(|w| w[0].collector_run_id >= w[1].collector_run_id)
    {
        return Err("care round dispatch order must be canonical by collector run ID".into());
    }
    Ok(())
}
fn assignments(
    spec: &CpuCareRoundSpecV1,
    source: &BTreeMap<String, String>,
    executable: &str,
    provider: &str,
) -> Result<Vec<CpuCollectorAssignmentV1>> {
    spec.collectors
        .iter()
        .map(|lesson| {
            Ok(CpuCollectorAssignmentV1 {
                collector_run_id: lesson.collector_run_id.clone(),
                backend: FoundationTrainingBackendV1::NativeCpu,
                producer_revision: format!("binary-sha256:{executable}"),
                world_seed: lesson.seed,
                sampling_seed: lesson.seed as u32 ^ (lesson.seed >> 32) as u32,
                sampling_counter_start: 0,
                sampling_counter_end: u32::try_from(lesson.decisions + 1)?,
                lesson: serde_json::to_value(lesson.lesson)?
                    .as_str()
                    .ok_or("invalid lesson label")?
                    .into(),
                episode_config_sha256: digest(
                    b"ALIFE-SERIAL-CPU-CARE-EPISODE-V1",
                    &(
                        lesson,
                        spec.temperature,
                        spec.prior,
                        spec.record_diagnostic_replay,
                        source,
                        executable,
                        provider,
                    ),
                )?,
            })
        })
        .collect()
}
impl CpuCareRoundPlanV1 {
    fn digest(&self) -> Result<[u8; 32]> {
        digest(
            b"ALIFE-SERIAL-CPU-CARE-PLAN-V1",
            &(
                self.schema_version,
                &self.kind,
                &self.spec,
                &self.source_sha256,
                &self.seed_evidence_sha256,
                &self.executable_sha256,
                &self.admission_checkout,
                &self.admission_helpers_sha256,
                &self.prior_provider_identity,
                &self.snapshot,
                &self.assignments,
            ),
        )
    }
    fn validate(&self) -> Result<()> {
        check_spec(&self.spec)?;
        let valid_hash = |s: &str| {
            s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        };
        if self.schema_version != 1
            || self.kind != "serial-cpu-care-round"
            || self.payload_sha256 != self.digest()?
            || self.source_sha256.len() != SOURCE_FILES.len()
            || SOURCE_FILES
                .iter()
                .any(|n| !self.source_sha256.contains_key(*n))
            || !self
                .seed_evidence_sha256
                .contains_key(&self.spec.seed_history)
            || self.seed_evidence_sha256.len() > 128
            || !self.spec.previous.is_absolute()
            || !self.spec.seed_history.is_absolute()
            || !self.spec.output.is_absolute()
            || !valid_hash(&self.executable_sha256)
            || !valid_hash(&self.prior_provider_identity)
            || self.spec.known_exclusions != self.admission_checkout.is_some()
            || (!self.spec.known_exclusions && !self.admission_helpers_sha256.is_empty())
            || (self.spec.known_exclusions
                && self.admission_helpers_sha256.len() != ADMISSION_HELPERS.len())
            || self
                .source_sha256
                .values()
                .chain(self.seed_evidence_sha256.values())
                .any(|s| !valid_hash(s))
            || self.assignments
                != assignments(
                    &self.spec,
                    &self.source_sha256,
                    &self.executable_sha256,
                    &self.prior_provider_identity,
                )?
        {
            return Err("care round plan seal/assignment inventory is invalid".into());
        }
        Ok(())
    }
    fn verify_inputs(&self) -> Result<FoundationTrainingHandoffV1> {
        self.validate()?;
        if let Some(checkout) = &self.admission_checkout {
            if qualified_checkout()? != *checkout {
                return Err("known-exclusions admission checkout changed".into());
            }
            for name in ADMISSION_HELPERS {
                let path = checkout.join("scripts").join(name);
                if self.admission_helpers_sha256.get(&path) != Some(&file_sha(&path)?) {
                    return Err("known-exclusions admission helper bytes changed".into());
                }
            }
        }
        if file_sha(&std::env::current_exe()?.canonicalize()?)? != self.executable_sha256
            || RuntimeSemanticPrior::deterministic_for_training(0)
                .metrics
                .provider_identity
                != self.prior_provider_identity
        {
            return Err(
                "care round executable/prior identity changed; use the pinned producer".into(),
            );
        }
        for (name, expected) in &self.source_sha256 {
            if file_sha(&self.spec.previous.join(name))? != *expected {
                return Err("care round frozen native source bytes changed".into());
            }
        }
        for (path, expected) in &self.seed_evidence_sha256 {
            if file_sha(path)? != *expected {
                return Err("care round immutable seed evidence changed".into());
            }
        }
        if self.spec.known_exclusions {
            verify_native_cpu_care_recovery(
                &self.spec.previous,
                &self.spec.seed_history,
                &self.spec.output.join("collectors"),
            )?;
        }
        let handoff = source_handoff(&self.spec.previous)?;
        if capture(&handoff, &self.spec)? != self.snapshot {
            return Err("care round source snapshot changed".into());
        }
        Ok(handoff)
    }
}
fn source_handoff(previous: &Path) -> Result<FoundationTrainingHandoffV1> {
    let validation = crate::validate_foundation_training_group_source(previous)?;
    if validation["source_receipt_filename"] != "cpu-cycle.json"
        || validation["source_execution_domain"] != "native-cpu"
    {
        return Err(
            "care round V1 requires an explicit completed native CPU source directory".into(),
        );
    }
    let (bytes, _) = read_native_cpu_care_evidence(&previous.join("training-handoff.json"))?;
    let handoff: FoundationTrainingHandoffV1 = serde_json::from_slice(&bytes)?;
    if handoff.origin != FoundationTrainingBackendV1::NativeCpu {
        return Err("care round source must retain native CPU optimizer history".into());
    }
    Ok(handoff)
}
fn capture(
    source: &FoundationTrainingHandoffV1,
    spec: &CpuCareRoundSpecV1,
) -> Result<CpuCollectionSnapshotV1> {
    let admitted = source.clone().admit()?;
    let trainer = CpuFoundationTrainer::from_checkpoint(
        admitted.phenotype,
        admitted.foundation,
        admitted.actor,
    )?;
    if trainer.phenotype().sensor_profile() != SensorProfile::GroundedTerrainVisionV1 {
        return Err(
            "care round requires the existing grounded natural-senses source; no sensor conversion"
                .into(),
        );
    }
    let value = CpuValueHead::from_checkpoint(admitted.value)?;
    Ok(CpuCollectionSnapshotV1::capture(
        &trainer,
        &value,
        &source.compiler_inputs,
        config(spec),
        source
            .policy_version
            .checked_add(1)
            .ok_or("policy version overflow")?,
    )?)
}

fn check_history_wrapper_depth(records: &[Value], path: &Path, depth: usize) -> Result<()> {
    // Dispatch adds one import level around the original immutable history.
    if depth > 7 {
        return Err("care round history must leave one import nesting level for dispatch".into());
    }
    let record = records
        .iter()
        .find(|r| r["path"].as_str().is_some_and(|p| Path::new(p) == path))
        .ok_or("admitted history record missing")?;
    if let Some(imports) = record["document"].get("imports") {
        for import in imports.as_array().ok_or("history import list changed")? {
            let child = Path::new(
                import["path"]
                    .as_str()
                    .ok_or("history import path missing")?,
            );
            let child = if child.is_absolute() {
                child.to_owned()
            } else {
                path.parent().ok_or("history lacks parent")?.join(child)
            };
            check_history_wrapper_depth(records, &child.canonicalize()?, depth + 1)?;
        }
    }
    Ok(())
}

/// Prepare without dispatch, reservation, environment changes or source mutation.
pub fn prepare_cpu_care_round(mut spec: CpuCareRoundSpecV1) -> Result<CpuCareRoundPlanV1> {
    spec.collectors
        .sort_by(|a, b| a.collector_run_id.cmp(&b.collector_run_id));
    check_spec(&spec)?;
    spec.previous = spec.previous.canonicalize()?;
    spec.seed_history = spec.seed_history.canonicalize()?;
    spec.output = resolve_isolated_training_output(
        &spec.output,
        [&spec.previous, &spec.seed_history]
            .into_iter()
            .map(PathBuf::as_path),
    )?;
    if spec.output.exists() {
        return Err("prepare requires a new dedicated round output directory".into());
    }
    let handoff = source_handoff(&spec.previous)?;
    let admission_checkout = if spec.known_exclusions {
        Some(qualified_checkout()?)
    } else {
        None
    };
    let mut admission_helpers_sha256 = BTreeMap::new();
    if let Some(checkout) = &admission_checkout {
        for name in ADMISSION_HELPERS {
            let path = checkout.join("scripts").join(name);
            admission_helpers_sha256.insert(path.clone(), file_sha(&path)?);
        }
    }
    let source_sha256 = SOURCE_FILES
        .into_iter()
        .map(|name| Ok((name.to_string(), file_sha(&spec.previous.join(name))?)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let mut evidence = BTreeMap::new();
    for lesson in &spec.collectors {
        let output = spec
            .output
            .join("collectors")
            .join(&lesson.collector_run_id);
        let provenance = admit_native_cpu_care_seed(
            &spec.previous,
            &spec.seed_history,
            &output,
            lesson.seed,
            spec.known_exclusions,
        )?;
        let mut current = BTreeMap::new();
        if provenance["evidence"]
            .as_array()
            .is_none_or(|records| records.len() > 126)
        {
            return Err(
                "care round history must leave two import slots for dispatch snapshots".into(),
            );
        }
        check_history_wrapper_depth(
            provenance["evidence"]
                .as_array()
                .ok_or("history evidence missing")?,
            &spec.seed_history,
            0,
        )?;
        for record in provenance["evidence"]
            .as_array()
            .ok_or("missing admitted history evidence")?
        {
            let path = PathBuf::from(
                record["path"]
                    .as_str()
                    .ok_or("seed evidence path missing")?,
            );
            let hash = record["sha256"]
                .as_str()
                .ok_or("seed evidence digest missing")?
                .to_owned();
            current.insert(path, hash);
        }
        if !evidence.is_empty() && evidence != current {
            return Err("seed evidence changed while preparing the round".into());
        }
        evidence = current;
    }
    if spec.known_exclusions {
        let document = read_value(&spec.seed_history)?;
        for reference in document["recovery_reconciliation"]["evidence"]
            .as_array()
            .ok_or("known-exclusions recovery evidence missing")?
        {
            let path = PathBuf::from(
                reference["path"]
                    .as_str()
                    .ok_or("recovery evidence path missing")?,
            )
            .canonicalize()?;
            let expected = reference["sha256"]
                .as_str()
                .ok_or("recovery evidence digest missing")?;
            if file_sha(&path)? != expected {
                return Err("cited recovery evidence changed".into());
            }
            if evidence
                .insert(path, expected.into())
                .is_some_and(|old| old != expected)
            {
                return Err("recovery evidence has conflicting raw pins".into());
            }
        }
    }
    let mut preserved = evidence.keys().map(PathBuf::as_path).collect::<Vec<_>>();
    preserved.push(&spec.previous);
    if resolve_isolated_training_output(&spec.output, preserved)? != spec.output {
        return Err("round output resolution changed".into());
    }
    let executable_sha256 = file_sha(&std::env::current_exe()?.canonicalize()?)?;
    let provider = RuntimeSemanticPrior::deterministic_for_training(0)
        .metrics
        .provider_identity;
    let approved = assignments(&spec, &source_sha256, &executable_sha256, &provider)?;
    let snapshot = capture(&handoff, &spec)?;
    let mut plan = CpuCareRoundPlanV1 {
        schema_version: 1,
        kind: "serial-cpu-care-round".into(),
        spec,
        source_sha256,
        seed_evidence_sha256: evidence,
        executable_sha256,
        admission_checkout,
        admission_helpers_sha256,
        prior_provider_identity: provider,
        snapshot,
        assignments: approved,
        payload_sha256: [0; 32],
    };
    plan.payload_sha256 = plan.digest()?;
    plan.verify_inputs()?;
    Ok(plan)
}

fn write_once(path: &Path, value: &impl Serialize) -> Result<()> {
    let value = serde_json::to_value(value)?;
    let expected = serde_json::to_vec_pretty(&value)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.is_file()
                || metadata.file_type().is_symlink()
                || fs::read(path)? != expected
            {
                return Err(
                    "care round immutable metadata changed; preserve it for reconciliation".into(),
                );
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            admission::write_atomic(path, &value)?
        }
        Err(error) => return Err(error.into()),
    }
    Ok(())
}
fn read_value(path: &Path) -> Result<Value> {
    Ok(read_native_cpu_care_evidence(path)?.1)
}
fn verify_dispatch_evidence(output: &Path) -> Result<()> {
    let provenance = read_value(&output.join("care-seed-admission.json"))?;
    for record in provenance["evidence"]
        .as_array()
        .ok_or("sealed dispatch evidence missing")?
    {
        let path = Path::new(
            record["path"]
                .as_str()
                .ok_or("dispatch evidence path missing")?,
        );
        if record["sha256"].as_str() != Some(file_sha(path)?.as_str()) {
            return Err(
                "immutable dispatch history/snapshot changed; consumed seed is not readmitted"
                    .into(),
            );
        }
    }
    Ok(())
}
fn dispatch_paths(histories: &Path, run_id: &str) -> (PathBuf, PathBuf) {
    // Dots are forbidden in run IDs, so no history can alias a ledger snapshot.
    (
        histories.join(format!("{run_id}.json")),
        histories.join(format!("{run_id}.ledger.json")),
    )
}
fn history_for(
    plan: &CpuCareRoundPlanV1,
    lane: &Path,
    attempts: &[Value],
    snapshot: &Path,
) -> Result<Value> {
    let mut document = read_value(&plan.spec.seed_history)?;
    let mut imports = vec![json!({"path":plan.spec.seed_history,
        "sha256":plan.seed_evidence_sha256[&plan.spec.seed_history]})];
    let ledger = lane.join(admission::LEDGER);
    if ledger.exists() {
        let bytes = fs::read(&ledger)?;
        match fs::symlink_metadata(snapshot) {
            Ok(metadata) => {
                if !metadata.is_file()
                    || metadata.file_type().is_symlink()
                    || fs::read(snapshot)? != bytes
                {
                    return Err("dispatch ledger snapshot changed; no seed is requalified".into());
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                use std::io::Write;
                let mut file = OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(snapshot)?;
                file.write_all(&bytes)?;
                file.sync_all()?;
                #[cfg(unix)]
                File::open(snapshot.parent().ok_or("snapshot lacks parent")?)?.sync_all()?;
            }
            Err(error) => return Err(error.into()),
        }
        imports.push(json!({"path":snapshot,"sha256":file_sha(snapshot)?}));
    }
    document["imports"] = json!(imports);
    let ranges = document["consumed_seed_ranges"]
        .as_array_mut()
        .ok_or("history consumed ranges missing")?;
    for attempt in attempts {
        ranges.push(json!([attempt["seed"], attempt["seed"]]));
    }
    Ok(document)
}
fn validate_attempts(plan: &CpuCareRoundPlanV1, lane: &Path, attempts: &[Value]) -> Result<()> {
    if attempts.len() > plan.assignments.len() {
        return Err("round lane contains unplanned attempts".into());
    }
    for (i, attempt) in attempts.iter().enumerate() {
        if attempt["collection"]["plan_sha256"] != json!(plan.payload_sha256)
            || attempt["collection"]["assignment"] != json!(plan.assignments[i])
            || attempt["output"] != json!(lane.join(&plan.assignments[i].collector_run_id))
        {
            return Err("round lane attempt differs from the sealed recipe".into());
        }
    }
    Ok(())
}
fn collect(
    plan: &CpuCareRoundPlanV1,
    index: usize,
    output: PathBuf,
    history: PathBuf,
) -> Result<()> {
    let lesson = &plan.spec.collectors[index];
    let assignment = &plan.assignments[index];
    let (config, mut episode) = prepare_care_episode(CpuCareRequest {
        previous: Some(plan.spec.previous.clone()),
        output,
        seed: lesson.seed,
        decisions: lesson.decisions,
        temperature: plan.spec.temperature,
        lesson: lesson.lesson,
        seed_history: Some(history),
        known_exclusions: plan.spec.known_exclusions,
        record_diagnostic_replay: plan.spec.record_diagnostic_replay,
        fixture: false,
    })?;
    episode.collection_binding = Some((assignment.clone(), plan.payload_sha256));
    episode.explicit_deterministic_prior = true;
    let result = (|| -> Result<()> {
        let collected = collect_cpu_episode(config, &mut episode)?;
        let prior = episode
            .prior
            .as_ref()
            .ok_or("explicit deterministic prior missing")?;
        if prior.metrics.provider_identity != plan.prior_provider_identity
            || prior.metrics.failures != 0
        {
            return Err("care round prior failed or changed; collection remains unsealed".into());
        }
        collected.seal(assignment.clone())?;
        episode
            .reservation
            .as_mut()
            .ok_or("care collection lacks reservation")?
            .collected()?;
        Ok(())
    })();
    if let Err(error) = result {
        if let Some(reservation) = episode.reservation.as_mut() {
            if let Err(mark) = reservation.fail(&error.to_string()) {
                return Err(format!("{error}; attempt retained for reconciliation: {mark}").into());
            }
        }
        return Err(error);
    }
    Ok(())
}

/// Run/resume exactly one serial frozen-source round. Complete collectors and
/// learner results are reused. Failed/unsealed lifetimes are never reset/retried.
pub fn run_cpu_care_round(plan: &CpuCareRoundPlanV1) -> Result<CpuCooperativeRoundResultV1> {
    plan.verify_inputs()?;
    let root = &plan.spec.output;
    let mut preserved = plan
        .seed_evidence_sha256
        .keys()
        .map(PathBuf::as_path)
        .collect::<Vec<_>>();
    preserved.push(&plan.spec.previous);
    if resolve_isolated_training_output(root, preserved)? != *root {
        return Err("round output path changed".into());
    }
    if root.exists() {
        if root.is_symlink() || !root.is_dir() {
            return Err("round output must be a real directory".into());
        }
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "non-UTF8 round artifact")?;
            if !matches!(
                name.as_str(),
                PLAN_FILE
                    | OWNER_FILE
                    | "histories"
                    | "collectors"
                    | "learner"
                    | "central-plan.json"
            ) && !((name.starts_with("care-round-plan.") || name.starts_with("central-plan."))
                && name.ends_with(".pending"))
            {
                return Err("foreign artifact in round output; nothing is overwritten".into());
            }
            if entry.file_type()?.is_symlink() {
                return Err("round artifacts must not be symlinks".into());
            }
        }
        if !root.join(PLAN_FILE).exists()
            && fs::read_dir(root)?.any(|entry| {
                entry.is_ok_and(|e| e.file_name() == "collectors" || e.file_name() == "learner")
            })
        {
            return Err(
                "round recovery lacks its sealed recipe; preserve existing lifetimes".into(),
            );
        }
    } else {
        fs::create_dir_all(root)?;
    }
    let owner_path = root.join(OWNER_FILE);
    if owner_path.is_symlink() {
        return Err("round owner file must not be a symlink".into());
    }
    let owner = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(owner_path)?;
    owner
        .try_lock()
        .map_err(|e| format!("another round owner is active or locking is unavailable: {e}"))?;
    write_once(&root.join(PLAN_FILE), plan)?;
    let lane = root.join("collectors");
    let histories = root.join("histories");
    fs::create_dir_all(&lane)?;
    fs::create_dir_all(&histories)?;
    let mut inputs = Vec::new();
    let mut members = Vec::new();
    for (index, assignment) in plan.assignments.iter().enumerate() {
        plan.verify_inputs()?;
        let output = lane.join(&assignment.collector_run_id);
        let (history, snapshot) = dispatch_paths(&histories, &assignment.collector_run_id);
        let mut attempts = admission::collection_attempts(&lane)?;
        validate_attempts(plan, &lane, &attempts)?;
        if attempts
            .get(index)
            .is_some_and(|a| a["status"] == "pending")
        {
            if output.join(CPU_COLLECTOR_COMMIT_FILE).is_file() {
                admission::reconcile(&lane)?;
                attempts = admission::collection_attempts(&lane)?;
            } else {
                return Err("interrupted unsealed care life is not restarted; preserve its checkpoint/prior state and reconcile the stopped owner explicitly".into());
            }
        }
        if let Some(attempt) = attempts.get(index) {
            if attempt["status"] != "collected" {
                return Err("failed/incomplete care attempt remains excluded; no replacement seed is chosen".into());
            }
        } else {
            if output.exists() {
                return Err(
                    "unreserved existing care life is never overwritten or restarted".into(),
                );
            }
            write_once(&history, &history_for(plan, &lane, &attempts, &snapshot)?)?;
            collect(plan, index, output.clone(), history.clone())?;
        }
        admission::validate_collected_attempt(&lane, &output, assignment, plan.payload_sha256)?;
        let provenance = read_value(&output.join("care-seed-admission.json"))?;
        if provenance["history_sha256"] != file_sha(&history)? {
            return Err("care round dispatch history changed".into());
        }
        verify_dispatch_evidence(&output)?;
        let shard = read_cpu_collector_output(&output)?;
        if shard.snapshot != plan.snapshot {
            return Err("care collector source snapshot changed".into());
        }
        let commit = Sha256::digest(fs::read(output.join(CPU_COLLECTOR_COMMIT_FILE))?).into();
        members.push(CpuCooperativeRoundMemberV1 {
            assignment: assignment.clone(),
            collector_commit_sha256: commit,
        });
        inputs.push(CpuCooperativeRoundInput {
            collector_run_id: assignment.collector_run_id.clone(),
            directory: output,
        });
    }
    let central = CpuCooperativeRoundPlanV1::seal(
        format!("care-{}", &plan.executable_sha256[..16]),
        Sha256::digest(fs::read(plan.spec.previous.join("training-handoff.json"))?).into(),
        plan.snapshot.clone(),
        config(&plan.spec),
        plan.spec.window_ticks,
        plan.spec.burn_in,
        plan.spec.effective_batch_size,
        members,
    )?;
    write_once(&root.join("central-plan.json"), &central)?;
    plan.verify_inputs()?;
    for input in &inputs {
        verify_dispatch_evidence(&input.directory)?;
    }
    let result = run_cpu_cooperative_round(
        &plan.spec.previous.join("training-handoff.json"),
        &central,
        &inputs,
        &root.join("learner"),
    )?;
    plan.verify_inputs()?;
    for input in &inputs {
        verify_dispatch_evidence(&input.directory)?;
    }
    drop(owner);
    Ok(result)
}

/// Compact CLI/automation result; full checkpoint arrays stay in the receipt.
pub fn cpu_care_round_summary(result: &CpuCooperativeRoundResultV1, directory: &Path) -> Value {
    json!({"mode":"serial-cpu-care-round","result":directory.join(CPU_COOPERATIVE_ROUND_RESULT_FILE),
        "source_policy_version":result.source_handoff.policy_version,
        "policy_version":result.output_handoff.policy_version,
        "actor_optimizer_step":result.output_handoff.actor.optimizer_step,
        "value_optimizer_step":result.output_handoff.value.optimizer_step,
        "training_rows":result.training_rows,"windows":result.windows,
        "completed_epochs":result.update.completed_epochs,"stopped_for_kl":result.update.stopped_for_kl,
        "comparison_role":"serial-collection-parity-control","promotion":"none"})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legal_run_names_cannot_alias_dispatch_history_and_ledger_snapshots() {
        let mut paths = BTreeSet::new();
        for name in ["a", "b", "b-ledger", "b-ledger-ledger", "00_held"] {
            assert!(portable_run_id(name));
            let (history, ledger) = dispatch_paths(Path::new("histories"), name);
            assert!(paths.insert(history));
            assert!(paths.insert(ledger));
        }
        assert_eq!(paths.len(), 10);
        for name in ["CON", "aux", "Prn", "NUL", "com1", "LPT9", "a.b", "../a"] {
            assert!(!portable_run_id(name));
        }
    }

    #[test]
    fn case_aliasing_collectors_are_rejected_before_any_source_or_output_access() {
        let spec: CpuCareRoundSpecV1 = serde_json::from_value(json!({
            "schema_version":1,"previous":"absent-source","seed_history":"absent-history",
            "output":"absent-output","temperature":1.,"prior":"deterministic_v1",
            "window_ticks":32,"burn_in":8,"effective_batch_size":1,
            "collectors":[
                {"collector_run_id":"A","seed":1,"lesson":"eat_held_food","decisions":2},
                {"collector_run_id":"a","seed":2,"lesson":"eat_reachable_food","decisions":2}
            ]
        }))
        .unwrap();
        assert!(check_spec(&spec).is_err());
    }

    #[test]
    fn retained_dispatch_snapshot_must_stay_byte_identical_without_seed_readmission() {
        let root = std::env::temp_dir().join(format!(
            "alife-care-evidence-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let snapshot = root.join("original.ledger.json");
        let bytes = b"{\"schema\":1,\"training_seed_ranges\":[[7,7]]}";
        fs::write(&snapshot, bytes).unwrap();
        let provenance =
            json!({"evidence":[{"path":snapshot,"sha256":file_sha(&snapshot).unwrap()}]});
        fs::write(
            root.join("care-seed-admission.json"),
            serde_json::to_vec(&provenance).unwrap(),
        )
        .unwrap();
        verify_dispatch_evidence(&root).unwrap();
        let mut changed = bytes.to_vec();
        changed.push(b'\n');
        fs::write(&snapshot, &changed).unwrap();
        assert!(verify_dispatch_evidence(&root).is_err());
        assert_eq!(fs::read(&snapshot).unwrap(), changed);
        fs::remove_file(&snapshot).unwrap();
        assert!(verify_dispatch_evidence(&root).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
