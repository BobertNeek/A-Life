//! One retained world, one WAL-consumed session nonce, many PPO updates.
//! The external owner alone publishes recovered Current; no gameplay/UI path.
use super::*;
use alife_training::{
    CurrentTrainingCheckpointV1, SameIndividualCurrentV1, SameIndividualProgressV1,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{ErrorKind, Read},
    path::PathBuf,
};

const KIND: &str = "native-resident-continuous-run-v1";
const INIT: &[u8; 16] = b"ALIFE-CON-INI-1\n";
const WAL: &[u8; 16] = b"ALIFE-CON-WAL-1\n";
const FRONTIER: &[u8; 16] = b"ALIFE-CON-MAN-1\n";
const INTENT: &[u8; 16] = b"ALIFE-CON-REC-1\n";
const RETIRE: &[u8; 16] = b"ALIFE-CON-RET-1\n";
const OUTCOME: &[u8; 16] = b"ALIFE-CON-OUT-1\n";
const VALIDATION: &str = "alife-current-private-validation-v1";
#[path = "continuous_store/protocol.rs"]
mod protocol;
#[cfg(test)]
#[path = "continuous_store/terminal_tests.rs"]
mod terminal_tests;

/// Hash identity-only reads without allocating another encoded checkpoint.
/// Callers retain their regular-file and size admission checks.
fn hash_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => count,
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.into()),
        };
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResidentContinuousRequest {
    pub schema: u16,
    /// One allocation; only its first nonce is consumed by this session.
    pub seed_range: [u64; 2],
    pub total_trained_decisions: usize,
    pub decisions_per_update: usize,
    pub snapshot_every_updates: usize,
    /// Opt-in training-only ordinary food placement and spatial language cues.
    #[serde(default, skip_serializing_if = "is_false")]
    pub care_assistance: bool,
    pub sampling_temperature: f32,
    pub collection_cutoff_unix_seconds: u64,
    pub finalization_deadline_unix_seconds: u64,
    pub max_temporary_bytes: u64,
}

fn is_false(value: &bool) -> bool {
    !*value
}

pub fn validate_resident_continuous_request(r: &ResidentContinuousRequest, now: u64) -> Result<()> {
    if r.schema != 1
        || r.seed_range[0] == 0
        || r.seed_range[0] > r.seed_range[1]
        || !(1..=10_000_000).contains(&r.total_trained_decisions)
        || !(1..=36_000).contains(&r.decisions_per_update)
        || r.decisions_per_update > r.total_trained_decisions
        || r.snapshot_every_updates == 0
        || !r.sampling_temperature.is_finite()
        || !(0.0001..=128.).contains(&r.sampling_temperature)
        || r.collection_cutoff_unix_seconds <= now
        || r.finalization_deadline_unix_seconds <= r.collection_cutoff_unix_seconds
        || r.max_temporary_bytes <= RECOVERY_HEADROOM + STARTUP_METADATA_HEADROOM
    {
        return Err("invalid same-individual request/cadence/session/cutoff/budget".into());
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Owner {
    schema: u16,
    kind: String,
    store: PathBuf,
    pid: u32,
    baseline_sha256: String,
    request_sha256: String,
    request: ResidentContinuousRequest,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionWal {
    schema: u16,
    baseline_sha256: String,
    request_sha256: String,
    consumed: bool,
    seeds: CheckpointSeedsV1,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContinuousSnapshot {
    file: String,
    sha256: String,
    policy: u64,
    state: SameIndividualCurrentV1,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContinuousFrontier {
    schema: u16,
    baseline_sha256: String,
    request_sha256: String,
    snapshot: Option<ContinuousSnapshot>,
    previous_snapshot: Option<ContinuousSnapshot>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryIntent {
    baseline_sha256: String,
    request_sha256: String,
    after_sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Outcome {
    baseline_sha256: String,
    request_sha256: String,
    value: serde_json::Value,
}
fn persist_outcome(run: &Path, o: &Owner, name: &str, value: &serde_json::Value) -> Result<()> {
    let outcome = Outcome {
        baseline_sha256: o.baseline_sha256.clone(),
        request_sha256: o.request_sha256.clone(),
        value: value.clone(),
    };
    let path = run.join(name);
    if path.exists() {
        let old: Outcome = read_metadata(&path, OUTCOME)?;
        if serde_json::to_vec(&old)? != serde_json::to_vec(&outcome)? {
            return Err("continuous outcome changed after commitment".into());
        }
        return Ok(());
    }
    let pending = path.with_extension("next");
    if entry_present(&pending)? {
        regular(&pending, (MAX_METADATA + 56) as u64)?;
        if !envelope(OUTCOME, &outcome)?.starts_with(&fs::read(pending)?) {
            return Err("conflicting uncommitted continuous outcome; preserving evidence".into());
        }
    }
    write_metadata(&path, OUTCOME, &outcome)
}
fn read_outcome(run: &Path, o: &Owner, name: &str) -> Result<Option<serde_json::Value>> {
    let path = run.join(name);
    if !entry_present(&path)? {
        if name == "continuous-terminal.bin" && entry_present(&path.with_extension("next"))? {
            return Err("uncommitted terminal outcome is not equivalent to a living recoverable session; preserving run".into());
        }
        return Ok(None);
    }
    let out: Outcome = read_metadata(&path, OUTCOME)?;
    if out.baseline_sha256 != o.baseline_sha256 || out.request_sha256 != o.request_sha256 {
        return Err("continuous outcome/source binding differs".into());
    }
    let pending = path.with_extension("next");
    if entry_present(&pending)? {
        regular(&pending, (MAX_METADATA + 56) as u64)?;
        if !envelope(OUTCOME, &out)?.starts_with(&fs::read(pending)?) {
            return Err("conflicting staged continuous outcome; preserving run".into());
        }
    }
    Ok(Some(out.value))
}

fn bind_terminal_disposition(
    checkpoint: CurrentTrainingCheckpointV1,
    o: &Owner,
    consumed: bool,
    terminal_event: Option<&serde_json::Value>,
    producer_result: Option<&serde_json::Value>,
) -> Result<CurrentTrainingCheckpointV1> {
    if let Some(event) = terminal_event {
        if !consumed {
            return Err("terminal observation without a dispatched continuous session".into());
        }
        return checkpoint.with_terminal_disposition(
            &o.baseline_sha256,
            &o.request_sha256,
            o.request.seed_range[0],
            event.clone(),
        );
    }
    if producer_result.is_some_and(|result| result["terminal"] == true) {
        return Err(
            "terminal producer result lacks its durable observation; preserving run".into(),
        );
    }
    Ok(checkpoint)
}

fn session_seeds(
    base: &CheckpointSeedsV1,
    r: &ResidentContinuousRequest,
    consumed: bool,
) -> Result<CheckpointSeedsV1> {
    let mut seeds = base.clone();
    seeds.reserve(r.seed_range[0], r.seed_range[1])?;
    if consumed {
        seeds.consume(r.seed_range[0])?;
    }
    Ok(seeds)
}
fn run_owner_kind(store: &Path) -> Result<Option<bool>> {
    let path = store.join(".run/run.json");
    if !entry_present(&path)? {
        if entry_present(&store.join(".run"))? {
            return Err("resident run has no intact owner; preserving evidence".into());
        }
        return Ok(None);
    }
    directory(&store.join(".run"))?;
    regular(&path, MAX_METADATA as u64)?;
    let owner: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
    match owner["kind"].as_str() {
        Some(KIND) => Ok(Some(true)),
        Some("native-resident-serial-run-v1") => Ok(Some(false)),
        _ => Err("unknown resident owner kind; preserving evidence".into()),
    }
}
pub(super) fn owns(store: &Path) -> Result<bool> {
    for (names, magic, legacy_magic, initialization) in [
        (
            [INITIALIZATION, INITIALIZATION_NEXT],
            INIT,
            b"ALIFE-RES-INI-1\n",
            true,
        ),
        ([RETIREMENT, RETIREMENT_NEXT], RETIRE, RET_MAGIC, false),
    ] {
        let mut present = false;
        let mut identified = None;
        for name in names {
            let path = store.join(name);
            if !entry_present(&path)? {
                continue;
            }
            present = true;
            regular(&path, (MAX_METADATA + 56) as u64)?;
            let kind = match protocol::header_owner(&fs::read(path)?, magic, legacy_magic) {
                protocol::HeaderOwner::Continuous => Some(true),
                protocol::HeaderOwner::Legacy => Some(false),
                protocol::HeaderOwner::Shared => None,
                protocol::HeaderOwner::Unknown => {
                    return Err("unknown resident metadata header; preserving evidence".into())
                }
            };
            if let Some(kind) = kind {
                if identified.is_some_and(|old| old != kind) {
                    return Err("conflicting resident metadata kinds; preserving evidence".into());
                }
                identified = Some(kind);
            }
        }
        if present {
            // An unambiguous authority header also works after run.json was
            // retired. A shared torn prefix owns no store kind by itself.
            if let Some(kind) = identified {
                return Ok(kind);
            }
            if let Some(kind) = run_owner_kind(store)? {
                return Ok(kind);
            }
            if initialization {
                // No run exists: either initializer may abort the unbound
                // prefix, consuming nothing and leaving Current unchanged.
                return Ok(true);
            }
            return Err(
                "shared retirement prefix has no intact run owner; preserving evidence".into(),
            );
        }
    }
    Ok(run_owner_kind(store)?.unwrap_or(false))
}
fn owner(store: &Path) -> Result<Owner> {
    let path = store.join(".run/run.json");
    regular(&path, MAX_METADATA as u64)?;
    let o: Owner = serde_json::from_slice(&fs::read(path)?)?;
    validate_owner(&o, store)?;
    Ok(o)
}
fn validate_owner(o: &Owner, store: &Path) -> Result<()> {
    if o.schema != 1
        || o.kind != KIND
        || o.store != store
        || o.pid == 0
        || o.request_sha256 != hash(&serde_json::to_vec(&o.request)?)
    {
        return Err("same-individual run source/request/store mismatch".into());
    }
    validate_resident_continuous_request(
        &o.request,
        o.request.collection_cutoff_unix_seconds.saturating_sub(1),
    )
}
fn initial_wal(o: &Owner, base: &CurrentTrainingCheckpointV1) -> Result<SessionWal> {
    Ok(SessionWal {
        schema: 1,
        baseline_sha256: o.baseline_sha256.clone(),
        request_sha256: o.request_sha256.clone(),
        consumed: false,
        seeds: session_seeds(&base.seeds, &o.request, false)?,
    })
}
fn initial_frontier(o: &Owner) -> ContinuousFrontier {
    ContinuousFrontier {
        schema: 1,
        baseline_sha256: o.baseline_sha256.clone(),
        request_sha256: o.request_sha256.clone(),
        snapshot: None,
        previous_snapshot: None,
    }
}

fn initialization_report(store: &Path, digest: &str, bound: bool) -> serde_json::Value {
    serde_json::json!({"kind":"same-individual-initialization-aborted-v1",
        "initialization_aborted":true,"cleanup_completed":true,
        "already_published_current":true,"candidate":store.join("current.alife-checkpoint"),
        "candidate_sha256":digest,"canonical_current_modified":false,
        "initialization_binding_verified":bound,"dispatched_sessions":0,
        "seeds_consumed":0,"new_request_allocation_committed":false,
        "temporary_budget":null,"neural_execution":false})
}

fn sampling_temperature(
    base: &CurrentTrainingCheckpointV1,
    request: &ResidentContinuousRequest,
) -> Result<f32> {
    Ok(protocol::effective_sampling_temperature(
        request.sampling_temperature,
        base.saved_training_sampling()?
            .map(|saved| saved.temperature),
    )?)
}

/// Caller owns the existing resident store lease. No nested lock is acquired.
fn abort_initialization(store: &Path) -> Result<serde_json::Value> {
    let marker = store.join(INITIALIZATION);
    let pending = store.join(INITIALIZATION_NEXT);
    let path = if marker.exists() { &marker } else { &pending };
    regular(path, (MAX_METADATA + 56) as u64)?;
    let bytes = fs::read(path)?;
    if !marker.exists() && protocol::incomplete_stage(&bytes, INIT, MAX_METADATA as u64) {
        if store.join(".run").exists() {
            return Err("uncommitted continuous initialization cannot own a run".into());
        }
        if !bytes.is_empty() && !(bytes.starts_with(INIT) || INIT.starts_with(&bytes)) {
            return Err("unknown continuous initialization prefix".into());
        }
        let current = store.join("current.alife-checkpoint");
        regular(&current, alife_training::MAX_CURRENT_CHECKPOINT_BYTES)?;
        let digest = hash_file(&current)?;
        alife_storage::remove_file(&pending)?;
        sync_directory(store)?;
        if hash_file(&current)? != digest {
            return Err("Current changed during unbound continuous startup cleanup".into());
        }
        return Ok(initialization_report(store, &digest, false));
    }
    let o: Owner = read_metadata(path, INIT)?;
    validate_owner(&o, store)?;
    let current = store.join("current.alife-checkpoint");
    regular(&current, alife_training::MAX_CURRENT_CHECKPOINT_BYTES)?;
    let base_bytes = fs::read(&current)?;
    if hash(&base_bytes) != o.baseline_sha256 {
        return Err("Current changed during continuous initialization".into());
    }
    let base = CurrentTrainingCheckpointV1::decode(&base_bytes).ok();
    if marker.exists() && pending.exists() {
        regular(&pending, (MAX_METADATA + 56) as u64)?;
        if !envelope(INIT, &o)?.starts_with(&fs::read(&pending)?) {
            return Err("conflicting initialization stage".into());
        }
    }
    let run = store.join(".run");
    if run.exists() {
        let mut expected = BTreeMap::from([
            (PathBuf::from("run.json"), serde_json::to_vec(&o)?),
            (PathBuf::from("baseline.alife-checkpoint"), base_bytes),
            (
                PathBuf::from("continuous-frontier.bin"),
                envelope(FRONTIER, &initial_frontier(&o))?,
            ),
            (
                PathBuf::from("continuous-frontier.next"),
                envelope(FRONTIER, &initial_frontier(&o))?,
            ),
            (
                PathBuf::from(VALIDATION).join("lease"),
                b"alife-current-private-validation-v1\n".to_vec(),
            ),
        ]);
        if let Some(base) = &base {
            if let Ok(wal) = initial_wal(&o, base) {
                for name in ["seed-wal.bin", "seed-wal.next"] {
                    expected.insert(PathBuf::from(name), envelope(WAL, &wal)?);
                }
            }
        }
        if let Some(life) = base.as_ref().and_then(|b| b.life.as_ref()) {
            let work = PathBuf::from(VALIDATION).join("work");
            expected.insert(work.join("world.json"), life.world_json.as_bytes().to_vec());
            expected.insert(
                work.join(".world.json.sleep-journal-v2.json"),
                life.sleep_journal_json.as_bytes().to_vec(),
            );
            for name in life.assets_base64.keys() {
                let p = Path::new(name);
                if !p
                    .components()
                    .all(|c| matches!(c, std::path::Component::Normal(_)))
                    || name.contains(['\\', ':'])
                {
                    return Err("private initialization asset path is not canonical".into());
                }
                expected.insert(work.join("world-assets").join(p), life.asset_bytes(name)?);
            }
        }
        let mut dirs = BTreeSet::from([
            PathBuf::from(VALIDATION),
            PathBuf::from(VALIDATION).join("work/world-assets"),
        ]);
        for p in expected.keys() {
            dirs.extend(
                p.ancestors()
                    .skip(1)
                    .filter(|p| !p.as_os_str().is_empty())
                    .map(Path::to_path_buf),
            );
        }
        validate_startup(&run, &run, &expected, &dirs)?;
        alife_storage::remove_dir_all(&run)?;
        sync_directory(store)?;
    }
    if hash_file(&current)? != o.baseline_sha256 {
        return Err("Current changed during continuous startup cleanup".into());
    }
    if pending.exists() {
        regular(&pending, (MAX_METADATA + 56) as u64)?;
        if !envelope(INIT, &o)?.starts_with(&fs::read(&pending)?) {
            return Err("conflicting initialization stage".into());
        }
        alife_storage::remove_file(pending)?;
    }
    if marker.exists() {
        alife_storage::remove_file(marker)?;
    }
    sync_directory(store)?;
    Ok(initialization_report(store, &o.baseline_sha256, true))
}
fn validate_startup(
    root: &Path,
    path: &Path,
    expected: &BTreeMap<PathBuf, Vec<u8>>,
    dirs: &BTreeSet<PathBuf>,
) -> Result<()> {
    directory(path)?;
    for entry in fs::read_dir(path)? {
        let p = entry?.path();
        let relative = p.strip_prefix(root)?;
        if fs::symlink_metadata(&p)?.is_dir() {
            if !dirs.contains(relative) {
                return Err("unknown continuous startup directory; preserving run".into());
            }
            validate_startup(root, &p, expected, dirs)?;
        } else {
            regular(&p, alife_training::MAX_CURRENT_CHECKPOINT_BYTES)?;
            let bytes = expected
                .get(relative)
                .ok_or("unknown continuous startup file; preserving run")?;
            if !bytes.starts_with(&fs::read(p)?) {
                return Err("continuous startup file differs from committed source".into());
            }
        }
    }
    Ok(())
}

pub fn run_resident_continuous_care(
    store: &Path,
    request: ResidentContinuousRequest,
) -> Result<serde_json::Value> {
    validate_resident_continuous_request(&request, now_seconds()?)?;
    let _lease = store_lease(store)?;
    let store = store.canonicalize()?;
    if initialization_pending(&store)?
        || retirement_pending(&store)?
        || entry_present(&store.join(".run"))?
    {
        return Err("recover/finish existing work before a same-individual producer".into());
    }
    let current = store.join("current.alife-checkpoint");
    regular(&current, alife_training::MAX_CURRENT_CHECKPOINT_BYTES)?;
    let base_bytes = fs::read(&current)?;
    // A copied/cross-lane terminal Current cannot allocate or consume a new
    // session. Full native private admission still follows under the budget.
    CurrentTrainingCheckpointV1::decode(&base_bytes)?
        .require_training_session_nonce(request.seed_range[0])?;
    let o = Owner {
        schema: 1,
        kind: KIND.into(),
        store: store.clone(),
        pid: std::process::id(),
        baseline_sha256: hash(&base_bytes),
        request_sha256: hash(&serde_json::to_vec(&request)?),
        request: request.clone(),
    };
    let init_bytes = envelope(INIT, &o)?;
    if (init_bytes.len() as u64).saturating_mul(2) > request.max_temporary_bytes {
        return Err("initialization staging exceeds budget".into());
    }
    write_new(&store.join(INITIALIZATION_NEXT), &init_bytes)?;
    replace(
        &store.join(INITIALIZATION_NEXT),
        &store.join(INITIALIZATION),
    )?;
    let run = store.join(".run");
    fs::create_dir(&run)?;
    sync_directory(&store)?;
    let budget = alife_storage::OutputBudget::register(&run, request.max_temporary_bytes)?;
    let init_capacity = budget.reserve(init_bytes.len() as u64)?;
    let _validation = current::validation_scope(&run)?;
    write_new(&run.join("run.json"), &serde_json::to_vec(&o)?)?;
    write_new(&run.join("baseline.alife-checkpoint"), &base_bytes)?;
    let baseline = read_current_training_checkpoint(&current)?;
    // Restoring a private sampler preserves its temperature, seed and counter.
    // Validate the effective temperature before consuming the session nonce.
    let actual_temperature = sampling_temperature(&baseline, &request)?;
    let _recovery_capacity = budget.reserve(RECOVERY_HEADROOM)?;
    let mut wal = initial_wal(&o, &baseline)?;
    let mut frontier = initial_frontier(&o);
    write_metadata(&run.join("seed-wal.bin"), WAL, &wal)?;
    write_metadata(&run.join("continuous-frontier.bin"), FRONTIER, &frontier)?;
    if fs::read(store.join(INITIALIZATION))? != init_bytes
        || hash_file(&current)? != o.baseline_sha256
    {
        return Err("continuous initialization or canonical Current changed".into());
    }
    alife_storage::remove_file(store.join(INITIALIZATION))?;
    sync_directory(&store)?;
    drop(init_capacity);
    if now_seconds()? >= request.collection_cutoff_unix_seconds || run.join("STOP").exists() {
        return Err("continuous collection stopped before session dispatch".into());
    }
    // Exactly one durable consume, before dispatch. Updates never mutate this WAL.
    wal.consumed = true;
    wal.seeds = session_seeds(&baseline.seeds, &request, true)?;
    write_metadata(&run.join("seed-wal.bin"), WAL, &wal)?;
    let result = super::super::continuous_world::run_same_individual_world(
        &baseline,
        request.seed_range[0],
        &request,
        &run.join("live"),
        |handoff, life, progress| {
            commit_snapshot(
                &run,
                &o,
                &baseline,
                &wal,
                &mut frontier,
                handoff,
                life,
                progress,
            )
        },
        |value| {
            if value["kind"] == "continuous-terminal-v1" {
                persist_outcome(&run, &o, "continuous-terminal.bin", &value)?;
            }
            emit(value);
            Ok(())
        },
    )?;
    if hash_file(&current)? != o.baseline_sha256 {
        return Err("immutable Current changed during continuous producer".into());
    }
    persist_outcome(&run, &o, "continuous-result.bin", &result)?;
    Ok(
        serde_json::json!({"kind":"resident-continuous-finished-v1","session_nonce":request.seed_range[0],
        "retained_world_seed":baseline.seed,"dispatched_sessions":1,"seeds_consumed":1,
        "sampling_temperature":actual_temperature,"requested_sampling_temperature":request.sampling_temperature,
        "last_validated_snapshot":frontier.snapshot,"producer":result,
        "terminal_event":read_outcome(&run,&o,"continuous-terminal.bin")?,
        "canonical_current_published":false,"temporary_budget":temporary_budget_report(&budget)?}),
    )
}

#[allow(clippy::too_many_arguments)]
fn commit_snapshot(
    run: &Path,
    o: &Owner,
    base: &CurrentTrainingCheckpointV1,
    wal: &SessionWal,
    frontier: &mut ContinuousFrontier,
    handoff: alife_training::FoundationTrainingHandoffV1,
    life: &Path,
    progress: SameIndividualProgressV1,
) -> Result<()> {
    if !wal.consumed
        || progress.session_nonce != o.request.seed_range[0]
        || progress.trained_rows > o.request.total_trained_decisions as u64
        || progress.sampling_temperature.to_bits()
            != sampling_temperature(base, &o.request)?.to_bits()
        || now_seconds()? > o.request.finalization_deadline_unix_seconds
    {
        return Err("continuous snapshot exceeds request/session/finalization boundary".into());
    }
    if let Some(old) = &frontier.snapshot {
        let p = &old.state.progress;
        if progress.trained_rows < p.trained_rows
            || progress.completed_updates < p.completed_updates
            || progress.world_tick < p.world_tick
        {
            return Err("continuous snapshot progress rolled backwards".into());
        }
    }
    let checkpoint = seal_same_individual_training_checkpoint(
        base,
        &fs::read(run.join("baseline.alife-checkpoint"))?,
        handoff,
        life,
        progress,
        wal.seeds.clone(),
    )?;
    let bytes = checkpoint.encode()?;
    let state = checkpoint
        .same_individual
        .clone()
        .ok_or("continuous snapshot tag missing")?;
    let file = format!(
        "checkpoint-{}-{}-{}.alife-checkpoint",
        checkpoint.handoff.policy_version,
        state.progress.completed_updates,
        state.progress.world_tick
    );
    let path = run.join(&file);
    if path.exists() {
        regular(&path, alife_training::MAX_CURRENT_CHECKPOINT_BYTES)?;
        if fs::read(&path)? != bytes {
            return Err("continuous snapshot output collision".into());
        }
    } else {
        let pending = path.with_extension("pending");
        write_new(&pending, &bytes)?;
        if read_current_training_checkpoint(&pending)? != checkpoint {
            return Err("continuous snapshot changed on full admission".into());
        }
        replace(&pending, &path)?;
    }
    let next = ContinuousSnapshot {
        file,
        sha256: hash(&bytes),
        policy: checkpoint.handoff.policy_version,
        state,
    };
    if frontier
        .snapshot
        .as_ref()
        .is_some_and(|old| old.file == next.file)
    {
        return Ok(());
    }
    let obsolete = frontier.previous_snapshot.take();
    frontier.previous_snapshot = frontier.snapshot.take();
    frontier.snapshot = Some(next.clone());
    write_metadata(&run.join("continuous-frontier.bin"), FRONTIER, frontier)?;
    if let Some(old) = obsolete {
        let p = run.join(old.file);
        regular(&p, alife_training::MAX_CURRENT_CHECKPOINT_BYTES)?;
        if hash_file(&p)? != old.sha256 {
            return Err("obsolete continuous snapshot changed; preserving it".into());
        }
        alife_storage::remove_file(p)?;
        sync_directory(run)?;
    }
    emit(
        serde_json::json!({"kind":"same-individual-snapshot-committed-v1","snapshot":next,
        "canonical_current_published":false,"neural_execution":false}),
    );
    Ok(())
}

fn admitted_run(
    store: &Path,
) -> Result<(
    Owner,
    CurrentTrainingCheckpointV1,
    SessionWal,
    ContinuousFrontier,
)> {
    let o = owner(store)?;
    let run = store.join(".run");
    directory(&run)?;
    let base_path = run.join("baseline.alife-checkpoint");
    regular(&base_path, alife_training::MAX_CURRENT_CHECKPOINT_BYTES)?;
    if hash_file(&base_path)? != o.baseline_sha256 {
        return Err("continuous baseline copy changed".into());
    }
    let base = read_current_training_checkpoint(&base_path)?;
    base.require_living_continuation()?;
    let wal: SessionWal = read_metadata(&run.join("seed-wal.bin"), WAL)?;
    let frontier: ContinuousFrontier =
        read_metadata(&run.join("continuous-frontier.bin"), FRONTIER)?;
    if wal.schema != 1
        || frontier.schema != 1
        || wal.baseline_sha256 != o.baseline_sha256
        || frontier.baseline_sha256 != o.baseline_sha256
        || wal.request_sha256 != o.request_sha256
        || frontier.request_sha256 != o.request_sha256
        || wal.seeds != session_seeds(&base.seeds, &o.request, wal.consumed)?
    {
        return Err("continuous WAL/frontier/source binding mismatch".into());
    }
    Ok((o, base, wal, frontier))
}
fn restored_snapshot(
    run: &Path,
    o: &Owner,
    base: &CurrentTrainingCheckpointV1,
    wal: &SessionWal,
    s: &ContinuousSnapshot,
) -> Result<CurrentTrainingCheckpointV1> {
    let p = &s.state.progress;
    if !wal.consumed
        || p.session_nonce != o.request.seed_range[0]
        || p.trained_rows > o.request.total_trained_decisions as u64
        || p.sampling_temperature.to_bits() != sampling_temperature(base, &o.request)?.to_bits()
        || s.state.baseline_current_sha256 != o.baseline_sha256
        || s.state.baseline_policy_version != base.handoff.policy_version
        || s.state.baseline_actor_optimizer_step != base.handoff.actor.optimizer_step
        || s.state.baseline_value_optimizer_step != base.handoff.value.optimizer_step
        || s.file
            != format!(
                "checkpoint-{}-{}-{}.alife-checkpoint",
                s.policy, p.completed_updates, p.world_tick
            )
    {
        return Err("continuous retained snapshot lineage/counter metadata mismatch".into());
    }
    let path = run.join(&s.file);
    regular(&path, alife_training::MAX_CURRENT_CHECKPOINT_BYTES)?;
    if hash_file(&path)? != s.sha256 {
        return Err("continuous committed snapshot changed".into());
    }
    let saved = read_current_training_checkpoint(&path)?;
    if saved.same_individual.as_ref() != Some(&s.state)
        || saved.handoff.policy_version != s.policy
        || saved.seed != base.seed
        || saved.founder_seed_base != base.founder_seed_base
        || saved.seeds != wal.seeds
    {
        return Err(
            "continuous snapshot differs from exact saved progress/heads/private state".into(),
        );
    }
    // Re-run the source-relative builder: baseline cumulative totals, individual,
    // birth/world and optimizer history cannot be invented in a new tag.
    let checked = base.same_individual_successor(
        &fs::read(run.join("baseline.alife-checkpoint"))?,
        saved.handoff.clone(),
        saved.life.clone().ok_or("snapshot life missing")?,
        p.clone(),
        wal.seeds.clone(),
    )?;
    if checked.encode()? != saved.encode()? {
        return Err("continuous snapshot changed its source-relative identity".into());
    }
    Ok(saved)
}

pub(super) fn recover_locked(store: &Path) -> Result<serde_json::Value> {
    if initialization_pending(store)? {
        return abort_initialization(store);
    }
    if retirement_pending(store)? {
        return finish_locked(store);
    }
    let o = owner(store)?;
    let run = store.join(".run");
    let budget = alife_storage::OutputBudget::register(&run, o.request.max_temporary_bytes)?;
    let _validation = current::validation_scope(&run)?;
    let (o, base, wal, frontier) = admitted_run(store)?;
    let current = store.join("current.alife-checkpoint");
    regular(&current, alife_training::MAX_CURRENT_CHECKPOINT_BYTES)?;
    let current_hash = hash_file(&current)?;
    let candidate_path = run.join("recovered-current.alife-checkpoint");
    let mut checkpoint = match &frontier.snapshot {
        Some(s) => restored_snapshot(&run, &o, &base, &wal, s)?,
        None => base,
    };
    checkpoint.seeds = wal.seeds;
    let terminal_event = read_outcome(&run, &o, "continuous-terminal.bin")?;
    let producer_result = read_outcome(&run, &o, "continuous-result.bin")?;
    let checkpoint = bind_terminal_disposition(
        checkpoint,
        &o,
        wal.consumed,
        terminal_event.as_ref(),
        producer_result.as_ref(),
    )?;
    let bytes = checkpoint.encode()?;
    let after = hash(&bytes);
    if current_hash != o.baseline_sha256 {
        let intent: RecoveryIntent = read_metadata(&run.join("recovery-intent.bin"), INTENT)?;
        if intent.baseline_sha256 != o.baseline_sha256
            || intent.request_sha256 != o.request_sha256
            || intent.after_sha256 != current_hash
            || after != current_hash
            || fs::read(&current)? != bytes
        {
            return Err("Current changed outside exact continuous recovery publication".into());
        }
        read_current_training_checkpoint(&current)?;
    } else {
        publish_recovery_candidate(&candidate_path, &bytes)?;
        write_metadata(
            &run.join("recovery-intent.bin"),
            INTENT,
            &RecoveryIntent {
                baseline_sha256: o.baseline_sha256,
                request_sha256: o.request_sha256,
                after_sha256: after.clone(),
            },
        )?;
    }
    Ok(serde_json::json!({"kind":"same-individual-recovery-v1",
        "candidate":if current_hash == after {current}else{candidate_path},"candidate_sha256":after,
        "already_published_current":current_hash == after,"session_nonce":o.request.seed_range[0],
        "retained_world_seed":checkpoint.seed,"dispatched_sessions":u8::from(wal.consumed),
        "seeds_consumed":u8::from(wal.consumed),"same_individual":checkpoint.same_individual,
        "retained_session_trained_rows":frontier.snapshot.as_ref().map_or(0,|s|s.state.progress.trained_rows),
        "retained_session_updates":frontier.snapshot.as_ref().map_or(0,|s|s.state.progress.completed_updates),
        "uncheckpointed_updates_restored":false,"neural_execution":false,"canonical_current_published":false,
        "terminal_event":terminal_event,"producer_result":producer_result,
        "terminal_disposition":checkpoint.terminal_disposition,
        "temporary_budget":temporary_budget_report(&budget)?}))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContinuousRetirement {
    schema: u16,
    kind: String,
    store: PathBuf,
    current_sha256: String,
    max_temporary_bytes: u64,
    files: BTreeMap<PathBuf, String>,
    directories: BTreeSet<PathBuf>,
}
fn inventory(
    root: &Path,
    path: &Path,
    files: &mut BTreeMap<PathBuf, String>,
    dirs: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    directory(path)?;
    for entry in fs::read_dir(path)? {
        let p = entry?.path();
        let relative = p.strip_prefix(root)?.to_path_buf();
        if fs::symlink_metadata(&p)?.is_dir() {
            dirs.insert(relative);
            inventory(root, &p, files, dirs)?;
        } else {
            regular(&p, alife_training::MAX_CURRENT_CHECKPOINT_BYTES)?;
            files.insert(relative, hash_file(&p)?);
        }
    }
    Ok(())
}
fn validate_retirement_tree(run: &Path, record: &ContinuousRetirement) -> Result<()> {
    let mut files = BTreeMap::new();
    let mut dirs = BTreeSet::new();
    inventory(run, run, &mut files, &mut dirs)?;
    if files.iter().any(|(p, h)| record.files.get(p) != Some(h))
        || !dirs.is_subset(&record.directories)
    {
        return Err("continuous retirement tree changed; preserving remaining work".into());
    }
    Ok(())
}
fn checkpoint_output_owned(run: &Path, name: &str, o: &Owner) -> Result<bool> {
    let Some(stem) = name
        .strip_suffix(".alife-checkpoint")
        .or_else(|| name.strip_suffix(".pending"))
    else {
        return Ok(false);
    };
    let Some(stem) = stem.strip_prefix("checkpoint-") else {
        return Ok(false);
    };
    let parts = stem
        .split('-')
        .map(str::parse::<u64>)
        .collect::<std::result::Result<Vec<_>, _>>();
    let Ok(parts) = parts else { return Ok(false) };
    if parts.len() != 3 {
        return Ok(false);
    }
    let path = run.join(name);
    regular(&path, alife_training::MAX_CURRENT_CHECKPOINT_BYTES)?;
    let bytes = fs::read(path)?;
    if name.ends_with(".pending")
        && protocol::incomplete_stage(
            &bytes,
            b"ALIFE-CURRENT-1\n",
            alife_training::MAX_CURRENT_CHECKPOINT_BYTES - 56,
        )
    {
        return Ok(true);
    }
    let saved = CurrentTrainingCheckpointV1::decode(&bytes)?;
    let Some(tag) = saved.same_individual.as_ref() else {
        return Ok(false);
    };
    Ok(tag.baseline_current_sha256 == o.baseline_sha256
        && tag.progress.session_nonce == o.request.seed_range[0]
        && saved.handoff.policy_version == parts[0]
        && tag.progress.completed_updates == parts[1]
        && tag.progress.world_tick == parts[2]
        && tag.progress.trained_rows <= o.request.total_trained_decisions as u64)
}
pub(super) fn finish_locked(store: &Path) -> Result<serde_json::Value> {
    if initialization_pending(store)? {
        return abort_initialization(store);
    }
    let run = store.join(".run");
    let current = store.join("current.alife-checkpoint");
    regular(&current, alife_training::MAX_CURRENT_CHECKPOINT_BYTES)?;
    let current_hash = hash_file(&current)?;
    let marker = store.join(RETIREMENT);
    let pending = store.join(RETIREMENT_NEXT);
    let record: ContinuousRetirement = if marker.exists() {
        read_metadata(&marker, RETIRE)?
    } else {
        let o = owner(store)?;
        let budget = alife_storage::OutputBudget::register(&run, o.request.max_temporary_bytes)?;
        let _validation = current::validation_scope(&run)?;
        let (o, base, wal, frontier) = admitted_run(store)?;
        let intent: RecoveryIntent = read_metadata(&run.join("recovery-intent.bin"), INTENT)?;
        let mut expected = match &frontier.snapshot {
            Some(s) => restored_snapshot(&run, &o, &base, &wal, s)?,
            None => base,
        };
        expected.seeds = wal.seeds;
        let expected = bind_terminal_disposition(
            expected,
            &o,
            wal.consumed,
            read_outcome(&run, &o, "continuous-terminal.bin")?.as_ref(),
            read_outcome(&run, &o, "continuous-result.bin")?.as_ref(),
        )?;
        if intent.baseline_sha256 != o.baseline_sha256
            || intent.request_sha256 != o.request_sha256
            || intent.after_sha256 != current_hash
            || hash(&expected.encode()?) != current_hash
            || read_current_training_checkpoint(&current)?.encode()? != expected.encode()?
        {
            return Err("continuous cleanup requires exact published recovery Current".into());
        }
        let mut files = BTreeMap::new();
        let mut directories = BTreeSet::new();
        inventory(&run, &run, &mut files, &mut directories)?;
        for name in fs::read_dir(&run)? {
            let e = name?;
            let n = e.file_name();
            let n = n.to_str().ok_or("non UTF8 continuous entry")?;
            let known = matches!(
                n,
                "run.json"
                    | "baseline.alife-checkpoint"
                    | "seed-wal.bin"
                    | "seed-wal.next"
                    | "continuous-frontier.bin"
                    | "continuous-frontier.next"
                    | "recovery-intent.bin"
                    | "recovery-intent.next"
                    | "recovered-current.alife-checkpoint"
                    | "recovered-current.pending"
                    | "STOP"
                    | "live"
                    | VALIDATION
            ) || matches!(
                n,
                "continuous-terminal.bin"
                    | "continuous-terminal.next"
                    | "continuous-result.bin"
                    | "continuous-result.next"
            ) || checkpoint_output_owned(&run, n, &o)?;
            if !known {
                return Err("unknown continuous entry; preserving run".into());
            }
            if n == VALIDATION {
                validate_completed_validation_workspace(&e.path())?;
            }
        }
        let record = ContinuousRetirement {
            schema: 1,
            kind: "same-individual-retirement-v1".into(),
            store: store.to_path_buf(),
            current_sha256: current_hash.clone(),
            max_temporary_bytes: o.request.max_temporary_bytes,
            files,
            directories,
        };
        let bytes = envelope(RETIRE, &record)?;
        let _capacity = budget.reserve((bytes.len() as u64).saturating_mul(2))?;
        if pending.exists() {
            regular(&pending, (MAX_METADATA + 56) as u64)?;
            let staged = fs::read(&pending)?;
            if !bytes.starts_with(&staged) {
                return Err("conflicting continuous retirement stage".into());
            }
            if staged != bytes {
                alife_storage::remove_file(&pending)?;
                write_new(&pending, &bytes)?;
            }
        } else {
            write_new(&pending, &bytes)?;
        }
        replace(&pending, &marker)?;
        record
    };
    if record.schema != 1
        || record.kind != "same-individual-retirement-v1"
        || record.store != store
        || record.current_sha256 != current_hash
    {
        return Err("continuous retirement no longer binds this store/Current".into());
    }
    if run.exists() {
        let _budget = alife_storage::OutputBudget::register(&run, record.max_temporary_bytes)?;
        validate_retirement_tree(&run, &record)?;
        alife_storage::remove_dir_all(&run)?;
        sync_directory(store)?;
    }
    if hash_file(&current)? != current_hash {
        return Err("Current changed during continuous retirement".into());
    }
    if pending.exists() {
        regular(&pending, (MAX_METADATA + 56) as u64)?;
        if fs::read(&pending)? != envelope(RETIRE, &record)? {
            return Err("continuous retirement pending changed".into());
        }
        alife_storage::remove_file(pending)?;
    }
    alife_storage::remove_file(marker)?;
    sync_directory(store)?;
    Ok(
        serde_json::json!({"kind":"same-individual-cleanup-v1","removed_run":true,"cleanup_completed":true,
        "current_sha256":current_hash,"canonical_current_modified":false,"neural_execution":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> ResidentContinuousRequest {
        ResidentContinuousRequest {
            schema: 1,
            seed_range: [11, 20],
            total_trained_decisions: 100,
            decisions_per_update: 10,
            snapshot_every_updates: 2,
            care_assistance: false,
            sampling_temperature: 1.,
            collection_cutoff_unix_seconds: 200,
            finalization_deadline_unix_seconds: 300,
            max_temporary_bytes: RECOVERY_HEADROOM + STARTUP_METADATA_HEADROOM + 10_000_000,
        }
    }
    #[test]
    fn care_assistance_is_explicit_and_legacy_request_identity_is_unchanged() {
        let mut original = request();
        let old = serde_json::to_value(&original).unwrap();
        assert!(old.get("care_assistance").is_none());
        let roundtrip: ResidentContinuousRequest = serde_json::from_value(old).unwrap();
        assert!(!roundtrip.care_assistance);
        original.care_assistance = true;
        let enabled = serde_json::to_value(&original).unwrap();
        assert_eq!(enabled["care_assistance"], true);
        assert!(
            serde_json::from_value::<ResidentContinuousRequest>(enabled)
                .unwrap()
                .care_assistance
        );
    }
    #[test]
    fn session_wal_consumes_one_nonce_and_never_one_per_update() {
        let base = CheckpointSeedsV1 {
            historical_freshness: "unknown".into(),
            excluded_ranges: vec![[1, 10]],
            pending_range: None,
        };
        let r = request();
        let before = session_seeds(&base, &r, false).unwrap();
        assert_eq!(before.pending_range, Some([11, 20]));
        let once = session_seeds(&base, &r, true).unwrap();
        assert_eq!(once.pending_range, Some([12, 20]));
        for _ in 0..100 {
            assert_eq!(session_seeds(&base, &r, true).unwrap(), once);
        }
        assert_eq!(base.pending_range, None);
        assert_eq!(once.historical_freshness, "unknown");
    }
    #[test]
    fn continuous_request_has_explicit_finite_clocks_cadence_and_dose() {
        let r = request();
        assert!(validate_resident_continuous_request(&r, 100).is_ok());
        let mut bad = r.clone();
        bad.snapshot_every_updates = 0;
        assert!(validate_resident_continuous_request(&bad, 100).is_err());
        let mut bad = r.clone();
        bad.decisions_per_update = 36001;
        assert!(validate_resident_continuous_request(&bad, 100).is_err());
        let mut bad = r.clone();
        bad.total_trained_decisions = 10_000_001;
        assert!(validate_resident_continuous_request(&bad, 100).is_err());
        let mut bad = r;
        bad.finalization_deadline_unix_seconds = 200;
        assert!(validate_resident_continuous_request(&bad, 100).is_err());
    }
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!(
                "alife-continuous-store-cpu-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir(&p).unwrap();
            Self(p.canonicalize().unwrap())
        }
        fn initialized(&self) -> Owner {
            let source = b"invalid source is preserved, not certified by initialization cleanup";
            fs::write(self.0.join("current.alife-checkpoint"), source).unwrap();
            let o = Owner {
                schema: 1,
                kind: KIND.into(),
                store: self.0.clone(),
                pid: std::process::id(),
                baseline_sha256: hash(source),
                request_sha256: hash(&serde_json::to_vec(&request()).unwrap()),
                request: request(),
            };
            write_new(&self.0.join(INITIALIZATION), &envelope(INIT, &o).unwrap()).unwrap();
            fs::create_dir(self.0.join(".run")).unwrap();
            o
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            assert_eq!(
                self.0.parent(),
                Some(std::env::temp_dir().canonicalize().unwrap().as_path())
            );
            assert!(self
                .0
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("alife-continuous-store-cpu-"));
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn file_hash_matches_checkpoint_bytes_across_read_boundaries() {
        let f = Fixture::new();
        let path = f.0.join("hash-input");
        assert!(hash_file(&path).is_err());
        for size in [0, 1, 65535, 65536, 65537, 2 * 65536, 2 * 65536 + 17] {
            let bytes: Vec<u8> = (0..size).map(|i| (i % 251) as u8).collect();
            fs::write(&path, &bytes).unwrap();
            assert_eq!(hash_file(&path).unwrap(), hash(&bytes), "size {size}");
            assert_eq!(fs::read(&path).unwrap(), bytes);
        }
    }

    #[test]
    fn changed_multichunk_baseline_refuses_recovery() {
        let f = Fixture::new();
        let mut o = f.initialized();
        let current = fs::read(f.0.join("current.alife-checkpoint")).unwrap();
        let path = f.0.join(".run/baseline.alife-checkpoint");
        let mut bytes = vec![7; 2 * 65536 + 17];
        o.baseline_sha256 = hash(&bytes);
        fs::write(f.0.join(".run/run.json"), serde_json::to_vec(&o).unwrap()).unwrap();
        *bytes.last_mut().unwrap() ^= 1;
        fs::write(&path, &bytes).unwrap();
        let _lease = store_lease(&f.0).unwrap();
        let error = admitted_run(&f.0).unwrap_err().to_string();
        assert!(error.contains("continuous baseline copy changed"));
        assert!(f.0.join(".run").is_dir());
        assert!(f.0.join(INITIALIZATION).is_file());
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(
            fs::read(f.0.join("current.alife-checkpoint")).unwrap(),
            current
        );
    }

    #[test]
    fn interrupted_initialization_accepts_only_its_exact_unpublished_prefixes() {
        let f = Fixture::new();
        let o = f.initialized();
        let before = fs::read(f.0.join("current.alife-checkpoint")).unwrap();
        let owner_bytes = serde_json::to_vec(&o).unwrap();
        fs::write(f.0.join(".run/run.json"), &owner_bytes[..20]).unwrap();
        fs::write(f.0.join(".run/baseline.alife-checkpoint"), &before[..10]).unwrap();
        let _lease = store_lease(&f.0).unwrap();
        let report = abort_initialization(&f.0).unwrap();
        assert_initialization_report(&f.0, &report, &before, true);
        assert!(!f.0.join(".run").exists());
        assert_eq!(
            fs::read(f.0.join("current.alife-checkpoint")).unwrap(),
            before
        );
    }
    fn assert_initialization_report(
        store: &Path,
        report: &serde_json::Value,
        source: &[u8],
        bound: bool,
    ) {
        assert_eq!(report["kind"], "same-individual-initialization-aborted-v1");
        for key in [
            "initialization_aborted",
            "cleanup_completed",
            "already_published_current",
        ] {
            assert_eq!(report[key], true, "{key}");
        }
        assert_eq!(
            report["candidate"],
            serde_json::to_value(store.join("current.alife-checkpoint")).unwrap()
        );
        assert_eq!(report["candidate_sha256"], hash(source));
        assert_eq!(report["initialization_binding_verified"], bound);
        for key in [
            "canonical_current_modified",
            "new_request_allocation_committed",
            "neural_execution",
        ] {
            assert_eq!(report[key], false, "{key}");
        }
        assert_eq!(report["dispatched_sessions"], 0);
        assert_eq!(report["seeds_consumed"], 0);
        assert!(report["temporary_budget"].is_null());
    }
    #[test]
    fn shared_retirement_prefix_with_unknown_owner_preserves_all_inputs() {
        let f = Fixture::new();
        let source = b"source must not be opened or changed by unknown-owner dispatch";
        fs::write(f.0.join("current.alife-checkpoint"), source).unwrap();
        fs::create_dir(f.0.join(".run")).unwrap();
        let owner = br#"{"kind":"unknown-future-resident-v1"}"#;
        fs::write(f.0.join(".run/run.json"), owner).unwrap();
        fs::write(f.0.join(RETIREMENT_NEXT), b"ALIF").unwrap();
        assert!(recover_resident_serial_checkpoint(&f.0).is_err());
        assert!(finish_resident_serial_run(&f.0).is_err());
        assert_eq!(
            fs::read(f.0.join("current.alife-checkpoint")).unwrap(),
            source
        );
        assert_eq!(fs::read(f.0.join(".run/run.json")).unwrap(), owner);
        assert_eq!(fs::read(f.0.join(RETIREMENT_NEXT)).unwrap(), b"ALIF");
        assert!(!f.0.join(RETIREMENT).exists());
    }
    #[test]
    fn torn_initialization_reports_unchanged_candidate_without_invented_binding() {
        let f = Fixture::new();
        let source = vec![42; 2 * 65536 + 17];
        fs::write(f.0.join("current.alife-checkpoint"), &source).unwrap();
        fs::write(f.0.join(INITIALIZATION_NEXT), &INIT[..4]).unwrap();
        let _lease = store_lease(&f.0).unwrap();
        assert!(owns(&f.0).unwrap());
        let report = recover_locked(&f.0).unwrap();
        assert_initialization_report(&f.0, &report, &source, false);
        assert_eq!(
            fs::read(f.0.join("current.alife-checkpoint")).unwrap(),
            source
        );
        assert!(!f.0.join(INITIALIZATION_NEXT).exists());
        assert!(!f.0.join(".run").exists());
    }
    #[test]
    fn initialization_cleanup_retries_after_owned_run_was_already_removed() {
        let f = Fixture::new();
        f.initialized();
        let before = fs::read(f.0.join("current.alife-checkpoint")).unwrap();
        fs::remove_dir(f.0.join(".run")).unwrap();
        let _lease = store_lease(&f.0).unwrap();
        abort_initialization(&f.0).unwrap();
        assert!(!f.0.join(INITIALIZATION).exists());
        assert_eq!(
            fs::read(f.0.join("current.alife-checkpoint")).unwrap(),
            before
        );
    }
    #[test]
    fn unknown_startup_files_and_conflicting_stages_preserve_run_and_current() {
        let f = Fixture::new();
        f.initialized();
        let before = fs::read(f.0.join("current.alife-checkpoint")).unwrap();
        fs::write(f.0.join(".run/unrelated"), b"preserve").unwrap();
        let _lease = store_lease(&f.0).unwrap();
        assert!(abort_initialization(&f.0).is_err());
        assert_eq!(fs::read(f.0.join(".run/unrelated")).unwrap(), b"preserve");
        fs::remove_file(f.0.join(".run/unrelated")).unwrap();
        fs::write(f.0.join(INITIALIZATION_NEXT), b"conflicting stage").unwrap();
        assert!(abort_initialization(&f.0).is_err());
        assert!(f.0.join(".run").exists());
        assert_eq!(
            fs::read(f.0.join("current.alife-checkpoint")).unwrap(),
            before
        );
    }
    #[test]
    fn retirement_retry_accepts_missing_owned_files_but_rejects_changed_or_new_files() {
        let f = Fixture::new();
        let run = f.0.join(".run");
        fs::create_dir(&run).unwrap();
        fs::write(run.join("a"), b"a").unwrap();
        let mut source = vec![9; 2 * 65536 + 17];
        fs::write(run.join("b"), &source).unwrap();
        let mut files = BTreeMap::new();
        let mut directories = BTreeSet::new();
        inventory(&run, &run, &mut files, &mut directories).unwrap();
        let r = ContinuousRetirement {
            schema: 1,
            kind: "same-individual-retirement-v1".into(),
            store: f.0.clone(),
            current_sha256: "source".into(),
            max_temporary_bytes: request().max_temporary_bytes,
            files,
            directories,
        };
        fs::remove_file(run.join("a")).unwrap();
        validate_retirement_tree(&run, &r).unwrap();
        fs::write(run.join("new"), b"unrelated").unwrap();
        assert!(validate_retirement_tree(&run, &r).is_err());
        fs::remove_file(run.join("new")).unwrap();
        *source.last_mut().unwrap() ^= 1;
        fs::write(run.join("b"), &source).unwrap();
        assert!(validate_retirement_tree(&run, &r).is_err());
        assert_eq!(fs::read(run.join("b")).unwrap(), source);
    }

    #[test]
    fn uncommitted_terminal_record_is_never_equivalent_to_a_living_session() {
        let f = Fixture::new();
        let o = f.initialized();
        let run = f.0.join(".run");
        let pending = run.join("continuous-terminal.next");
        fs::write(&pending, b"AL").unwrap();
        assert!(read_outcome(&run, &o, "continuous-terminal.bin").is_err());
        assert_eq!(fs::read(&pending).unwrap(), b"AL");
        let event =
            serde_json::json!({"kind":"continuous-terminal-v1","terminal":true,"death_tick":7});
        persist_outcome(&run, &o, "continuous-terminal.bin", &event).unwrap();
        assert_eq!(
            read_outcome(&run, &o, "continuous-terminal.bin").unwrap(),
            Some(event)
        );
    }
}
