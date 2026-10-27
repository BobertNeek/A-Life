//! Opt-in finite resident learner. The existing canonical current is immutable
//! until the external owner publishes a validated recovery candidate.
use super::*;
use alife_training::CheckpointSeedsV1;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_METADATA: usize = 1024 * 1024;
const MAX_EPISODES: usize = 4096;
const WAL_MAGIC: &[u8; 16] = b"ALIFE-RES-WAL-1\n";
const MAN_MAGIC: &[u8; 16] = b"ALIFE-RES-MAN-1\n";
const REC_MAGIC: &[u8; 16] = b"ALIFE-RES-REC-1\n";
const RET_MAGIC: &[u8; 16] = b"ALIFE-RES-RET-1\n";
const RETIREMENT: &str = "resident-retirement.bin";
const RETIREMENT_NEXT: &str = "resident-retirement.next";
mod initialization;
use initialization::{initialization_pending, INITIALIZATION, INITIALIZATION_NEXT};
#[path = "continuous_store.rs"]
mod continuous_store;
pub use continuous_store::*;

// Actual typed recovery peak, NOT a bound on an episode's raw private assets.
// One encoded candidate plus its admission materialization coexist. In a valid
// Current, decoded asset bytes + world/journal UTF-8 are no larger than their
// JSON/base64 representations within the 256-MiB encoded envelope. Native
// materialization only loads this closure; it does not generate new authority
// generations. Committed snapshots/baseline and all active output remain
// separately charged. Three bounded metadata stages and the 128-byte validation
// lease cover recovery-intent/WAL/frontier staging; no disk bytes are guessed.
const RECOVERY_HEADROOM: u64 =
    2 * alife_training::MAX_CURRENT_CHECKPOINT_BYTES + 3 * (MAX_METADATA as u64 + 56) + 128;
// Three actual bounded startup records (run JSON, WAL, frontier) and the
// validation lease must fit in addition to, rather than spend, recovery space.
const STARTUP_METADATA_HEADROOM: u64 = 3 * (MAX_METADATA as u64 + 56) + 128;

pub(super) struct ResidentLearner {
    pub trainer: FoundationTrainer,
    pub value: PpoTrainingState,
    pub asset: FoundationWeightAsset,
    pub next_policy: u64,
    pub founder_seed_base: u64,
}
pub(super) struct ResidentEpisode {
    pub learner: ResidentLearner,
    pub receipt: FoundationCycleReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResidentSerialRow {
    pub lesson: FoundationTeacherLesson,
    pub decisions: usize,
    #[serde(default)]
    pub food_need_condition: crate::FoundationFoodNeedCondition,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResidentSerialRequest {
    pub schema: u16,
    pub seed_range: [u64; 2],
    pub rows: Vec<ResidentSerialRow>,
    /// Explicit caller policy; >= rows.len() means final-only on normal completion.
    pub checkpoint_every_episodes: usize,
    pub sampling_temperature: f32,
    pub collection_cutoff_unix_seconds: u64,
    pub max_temporary_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SeedWal {
    schema: u16,
    baseline_sha256: String,
    request_sha256: String,
    dispatched: usize,
    seeds: CheckpointSeedsV1,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    file: String,
    sha256: String,
    policy: u64,
    completed_episodes: usize,
    trained_rows: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: u16,
    baseline_sha256: String,
    request_sha256: String,
    request: ResidentSerialRequest,
    snapshot: Option<Snapshot>,
    previous_snapshot: Option<Snapshot>,
}

fn now_seconds() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn regular(path: &Path, limit: u64) -> Result<()> {
    let m = fs::symlink_metadata(path)?;
    if !m.is_file() || m.len() > limit || reparse(&m) {
        return Err("resident input must be a bounded regular file".into());
    }
    Ok(())
}
#[cfg(windows)]
fn reparse(m: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    m.file_attributes() & 0x400 != 0
}
#[cfg(not(windows))]
fn reparse(m: &fs::Metadata) -> bool {
    m.file_type().is_symlink()
}
fn directory(path: &Path) -> Result<()> {
    let m = fs::symlink_metadata(path)?;
    if !m.is_dir() || reparse(&m) {
        return Err("resident directory aliases are refused".into());
    }
    Ok(())
}
fn sync_directory(path: &Path) -> Result<()> {
    #[cfg(not(windows))]
    {
        fs::File::open(path)?.sync_all()?;
    }
    #[cfg(windows)]
    {
        let _ = path;
    }
    Ok(())
}
fn replace(from: &Path, to: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{
            MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
        };
        let a = from
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        let b = to
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>();
        alife_storage::replace_with(from, to, || {
            // Same owned directory; files were flushed first. This is filesystem
            // publication only, never desktop input or application control.
            if unsafe {
                MoveFileExW(
                    a.as_ptr(),
                    b.as_ptr(),
                    MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
                )
            } == 0
            {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        })?;
    }
    #[cfg(not(windows))]
    {
        alife_storage::rename(from, to)?;
    }
    sync_directory(to.parent().ok_or("resident publication lacks parent")?)
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    alife_storage::write_all(path, &mut file, bytes)?;
    file.sync_all()?;
    sync_directory(path.parent().ok_or("resident output lacks parent")?)
}
fn envelope<T: Serialize>(magic: &[u8; 16], value: &T) -> Result<Vec<u8>> {
    let payload = serde_json::to_vec(value)?;
    if payload.len() > MAX_METADATA {
        return Err("resident metadata exceeds 1 MiB".into());
    }
    let mut out = Vec::with_capacity(56 + payload.len());
    out.extend_from_slice(magic);
    out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    out.extend_from_slice(&Sha256::digest(&payload));
    out.extend(payload);
    Ok(out)
}
fn write_metadata<T: Serialize>(path: &Path, magic: &[u8; 16], value: &T) -> Result<()> {
    let bytes = envelope(magic, value)?;
    let pending = path.with_extension("next");
    // Only an uncommitted derived staging file is replaceable. The caller owns
    // the writer lease; committed source/WAL/frontier files stay authoritative.
    if pending.exists() {
        regular(&pending, (MAX_METADATA + 56) as u64)?;
        alife_storage::remove_file(&pending)?;
    }
    write_new(&pending, &bytes)?;
    replace(&pending, path)
}

fn publish_recovery_candidate(path: &Path, bytes: &[u8]) -> Result<()> {
    let root = path.parent().ok_or("resident recovery lacks parent")?;
    if path.exists() {
        regular(path, alife_training::MAX_CURRENT_CHECKPOINT_BYTES)?;
        if hash(&fs::read(path)?) == hash(bytes) {
            return Ok(());
        }
        // An interrupted pre-intent write from the first experimental version
        // is recoverable. A complete different candidate or a committed intent
        // is evidence that must not be overwritten under a repair assumption.
        if root.join("recovery-intent.bin").exists()
            || alife_training::CurrentTrainingCheckpointV1::read(path).is_ok()
        {
            return Err("resident recovery output collision".into());
        }
        // Only an incomplete/unencoded pre-intent candidate may retire here.
        // Never classify a valid checkpoint as incomplete because full scratch
        // admission happened to run out of output capacity.
        alife_storage::remove_file(path)?;
    }
    let pending = root.join("recovered-current.pending");
    if pending.exists() {
        regular(&pending, alife_training::MAX_CURRENT_CHECKPOINT_BYTES)?;
        alife_storage::remove_file(&pending)?;
    }
    write_new(&pending, bytes)?;
    read_current_training_checkpoint(&pending)?;
    replace(&pending, path)
}
fn read_metadata<T: serde::de::DeserializeOwned>(path: &Path, magic: &[u8; 16]) -> Result<T> {
    regular(path, (MAX_METADATA + 56) as u64)?;
    let bytes = fs::read(path)?;
    if bytes.len() < 56
        || &bytes[..16] != magic
        || u64::from_le_bytes(bytes[16..24].try_into()?) != (bytes.len() - 56) as u64
        || Sha256::digest(&bytes[56..]).as_slice() != &bytes[24..56]
    {
        return Err("resident metadata length/digest mismatch".into());
    }
    Ok(serde_json::from_slice(&bytes[56..])?)
}
fn store_lease(store: &Path) -> Result<fs::File> {
    directory(store)?;
    let lock = store.join(".current.writer.lock");
    if lock.exists() {
        regular(&lock, 1)?;
    }
    let mut lease = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock)?;
    lease.try_lock()?;
    if lease.metadata()?.len() == 0 {
        lease.write_all(&[0])?;
        lease.sync_all()?;
    }
    for entry in fs::read_dir(store)? {
        let name = entry?.file_name();
        if name != "current.alife-checkpoint"
            && name != ".current.writer.lock"
            && name != ".run"
            && name != RETIREMENT
            && name != RETIREMENT_NEXT
            && name != INITIALIZATION
            && name != INITIALIZATION_NEXT
        {
            return Err(
                "resident store needs existing publication recovery or contains unknown entries"
                    .into(),
            );
        }
    }
    Ok(lease)
}
fn validate_request(request: &ResidentSerialRequest, now: u64) -> Result<()> {
    let [first, last] = request.seed_range;
    if request.schema != 1
        || first == 0
        || first > last
        || request.rows.is_empty()
        || request.rows.len() > MAX_EPISODES
        || last - first < (request.rows.len() - 1) as u64
        || request.checkpoint_every_episodes == 0
        || request.checkpoint_every_episodes > MAX_EPISODES
        || request.collection_cutoff_unix_seconds <= now
        || request.collection_cutoff_unix_seconds > now.saturating_add(86400)
        || request.max_temporary_bytes < RECOVERY_HEADROOM + STARTUP_METADATA_HEADROOM
    {
        return Err("invalid resident request/cadence/seed range/cutoff/budget".into());
    }
    cycle_ppo_config(request.sampling_temperature)?;
    for row in &request.rows {
        let decision_limit = if row.lesson == FoundationTeacherLesson::Feeding {
            36_000
        } else {
            128
        };
        if !(1..=decision_limit).contains(&row.decisions)
            || !matches!(
                row.lesson,
                FoundationTeacherLesson::EatHeldFood
                    | FoundationTeacherLesson::EatReachableFood
                    | FoundationTeacherLesson::Feeding
                    | FoundationTeacherLesson::RelocatedFood
            )
        {
            return Err("resident care supports finite Feeding worlds up to36000 decisions; other care rows remain bounded at128".into());
        }
        row.food_need_condition.validate_lesson(Some(row.lesson))?;
    }
    Ok(())
}
fn seeds_after(
    baseline: &CheckpointSeedsV1,
    request: &ResidentSerialRequest,
    dispatched: usize,
) -> Result<CheckpointSeedsV1> {
    if dispatched > request.rows.len() {
        return Err("resident consumed cursor exceeds planned rows".into());
    }
    let mut seeds = baseline.clone();
    let [first, last] = request.seed_range;
    seeds.reserve(first, last)?;
    seeds.pending_range = if dispatched as u64 == last - first + 1 {
        None
    } else {
        Some([first + dispatched as u64, last])
    };
    seeds.validate()?;
    Ok(seeds)
}
fn logical_bytes(root: &Path) -> Result<u64> {
    directory(root)?;
    let mut sum = 0u64;
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        let m = fs::symlink_metadata(&path)?;
        if reparse(&m) {
            return Err("resident temporary alias refused".into());
        }
        let bytes = if m.is_dir() {
            logical_bytes(&path)?
        } else if m.is_file() {
            m.len()
        } else {
            return Err("resident temporary special file refused".into());
        };
        sum = sum
            .checked_add(bytes)
            .ok_or("resident temporary byte overflow")?;
    }
    Ok(sum)
}
fn remove_episode(run: &Path, episode: &Path) -> Result<()> {
    if episode.parent() != Some(run)
        || !episode
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("episode-"))
    {
        return Err("resident cleanup target is not its owned episode".into());
    }
    logical_bytes(episode)?;
    alife_storage::remove_dir_all(episode)?;
    sync_directory(run)
}
fn emit(value: serde_json::Value) {
    // Losing the optional observation pipe must not reset resident learning.
    let _ = writeln!(std::io::stdout(), "{value}");
    let _ = std::io::stdout().flush();
}
fn row_summary(receipt: &FoundationCycleReceipt) -> serde_json::Value {
    let r = serde_json::to_value(receipt).unwrap_or(serde_json::Value::Null);
    let exact = match receipt.lesson {
        Some(FoundationTeacherLesson::EatHeldFood | FoundationTeacherLesson::EatReachableFood) => {
            Some(receipt.trained_held_food_consumption_events)
        }
        Some(FoundationTeacherLesson::Feeding) => r["feeding_food"]["nourished_meal_rows"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter(|v| {
                        v.as_u64()
                            .is_some_and(|n| n < receipt.trained_replay_rows as u64)
                    })
                    .count()
            }),
        Some(FoundationTeacherLesson::RelocatedFood) => r["food_relocation"]["nourished_meal_rows"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter(|v| {
                        v.as_u64()
                            .is_some_and(|n| n < receipt.trained_replay_rows as u64)
                    })
                    .count()
            }),
        _ => None,
    };
    // Compact counts survive normal temporary cleanup. Do not retain the
    // bounded but detailed context/action receipt rings in every stdout row.
    let prior = receipt.semantic_prior.as_ref().map(|prior| {
        serde_json::json!({
            "backend":prior.backend,"provider_identity":prior.provider_identity,
            "context_contract":prior.context_contract,
            "rich_information_required":prior.rich_information_required,
            "requests":prior.requests,"delivered_frames":prior.delivered_frames,
            "cache_hits":prior.cache_hits,"stale_replies":prior.stale_replies,
            "failures":prior.failures,"dropout_frames":prior.dropout_frames,
            "prime_wait_ms":prior.prime_wait_ms,"prime_timeouts":prior.prime_timeouts,
            "empty_hint_frames":prior.empty_hint_frames,
            "validated_provider_replies":prior.validated_provider_replies,
            "decision_frames":prior.decision_frames,
            "decision_frames_with_prior":prior.decision_frames_with_prior,
            "decision_frames_without_prior":prior.decision_frames_without_prior,
            "decision_frames_with_heard_language":prior.decision_frames_with_heard_language,
            "unusable_hint_retry_requests":prior.unusable_hint_retry_requests
        })
    });
    serde_json::json!({"kind":"resident-provisional-episode-v1","seed":receipt.seed,"policy_version":receipt.policy_version,
        "lesson":receipt.lesson,"trained_rows":receipt.trained_replay_rows,"exact_trained_nourishing_events":exact,
        "generic_consumed_events":receipt.consumed_events,"full_relocation":r["food_relocation"]["lesson_completed"],
        "actor_optimizer_step":receipt.actor_optimizer_step,"value_optimizer_step":receipt.value_optimizer_step,
        "requested_training_ticks":receipt.requested_training_ticks,"training_ticks":receipt.training_ticks,
        "captured_replay_rows":receipt.captured_replay_rows,"bootstrap_replay_rows":receipt.bootstrap_replay_rows,
        "terminal_death_tick":receipt.terminal_death_tick,
        "actor_optimizer_updates":receipt.actor_optimizer_updates,"value_optimizer_updates":receipt.value_optimizer_updates,
        "completed_epochs":receipt.completed_epochs,"teacher_cue_frames":receipt.teacher_cue_frames,
        "semantic_prior":prior,
        "collection_seconds":receipt.collection_seconds,"update_seconds":receipt.update_seconds,
        "causal_collection":receipt.causal_collection,
        "canonical_current_published":false,"durability":"provisional until a full snapshot commits"})
}

fn snapshot_episode(
    run: &Path,
    episode: &Path,
    completed: &ResidentEpisode,
    wal: &SeedWal,
    manifest: &mut Manifest,
    trained_rows: usize,
) -> Result<Snapshot> {
    // This requires the unchanged PRE-PPO final-private-capture branch to have
    // run. No late capture of potentially altered live buffers is substituted.
    alife_storage::write(
        episode.join("actor-checkpoint.json"),
        serde_json::to_vec(&completed.learner.trainer.checkpoint()?)?,
    )?;
    alife_storage::write(
        episode.join("value-checkpoint.json"),
        serde_json::to_vec(
            &completed
                .learner
                .value
                .checkpoint(completed.learner.trainer.session())?,
        )?,
    )?;
    alife_storage::write(
        episode.join("cycle.json"),
        serde_json::to_vec_pretty(&completed.receipt)?,
    )?;
    inspect_native_training_frontier(episode, completed.receipt.policy_version)?;
    let checkpoint = migrate_current_training_checkpoint(episode, wal.seeds.clone())?;
    let bytes = checkpoint.encode()?;
    if logical_bytes(run)?
        .checked_add(bytes.len() as u64)
        .ok_or("resident snapshot budget overflow")?
        > manifest.request.max_temporary_bytes
    {
        return Err(
            "resident temporary budget prevents snapshot; last snapshot/baseline retained".into(),
        );
    }
    let file = format!(
        "checkpoint-{}.alife-checkpoint",
        completed.receipt.policy_version
    );
    let path = run.join(&file);
    write_new(&path, &bytes)?;
    let admitted = read_current_training_checkpoint(&path)?;
    if admitted != checkpoint {
        return Err("resident snapshot changed during full admission".into());
    }
    let next = Snapshot {
        file,
        sha256: hash(&bytes),
        policy: checkpoint.handoff.policy_version,
        completed_episodes: wal.dispatched,
        trained_rows,
    };
    let obsolete = manifest.previous_snapshot.take();
    manifest.previous_snapshot = manifest.snapshot.take();
    manifest.snapshot = Some(next.clone());
    write_metadata(&run.join("resident-frontier.bin"), MAN_MAGIC, manifest)?;
    if let Some(old) = obsolete {
        regular(
            &run.join(&old.file),
            alife_training::MAX_CURRENT_CHECKPOINT_BYTES,
        )?;
        if hash(&fs::read(run.join(&old.file))?) != old.sha256 {
            return Err("resident prior snapshot changed; refusing retirement".into());
        }
        alife_storage::remove_file(run.join(old.file))?;
        sync_directory(run)?;
    }
    emit(
        serde_json::json!({"kind":"resident-snapshot-committed-v1","snapshot":next,
        "canonical_current_published":false,"durability":"self-contained validated temporary snapshot; canonical publication remains with owner"}),
    );
    Ok(next)
}

/// Runs only an explicit finite care schedule. The caller remains responsible
/// for actual resource/activity supervision and hard-stop enforcement.
pub fn run_resident_serial_care(
    store: &Path,
    request: ResidentSerialRequest,
) -> Result<serde_json::Value> {
    run_resident_with_startup_observer(store, request, |_, _| Ok(()))
}
fn run_resident_with_startup_observer(
    store: &Path,
    request: ResidentSerialRequest,
    mut observe: impl FnMut(&str, &Path) -> Result<()>,
) -> Result<serde_json::Value> {
    validate_request(&request, now_seconds()?)?;
    let _lease = store_lease(store)?;
    let store = store.canonicalize()?;
    if retirement_pending(&store)? || initialization_pending(&store)? {
        return Err(
            "finish pending resident initialization/retirement before another producer".into(),
        );
    }
    let current = store.join("current.alife-checkpoint");
    regular(&current, alife_training::MAX_CURRENT_CHECKPOINT_BYTES)?;
    let baseline_bytes = fs::read(&current)?;
    let baseline_hash = hash(&baseline_bytes);
    let run = store.join(".run");
    if entry_present(&run)? {
        return Err("resident run already exists; recover it before another producer".into());
    }
    let initialization = initialization::commit(&store, &baseline_hash, &request, &mut observe)?;
    fs::create_dir(&run)?;
    sync_directory(&store)?;
    observe("run-created", &store)?;
    let budget = alife_storage::OutputBudget::register(&run, request.max_temporary_bytes)?;
    let initialization_capacity = budget.reserve(initialization.len() as u64)?;
    let _validation = current::validation_scope(&run)?;
    let baseline = read_current_training_checkpoint(&current)?;
    observe("baseline-admitted", &store)?;
    let baseline_policy = baseline.handoff.policy_version;
    let initial_seeds = seeds_after(&baseline.seeds, &request, 0)?;
    let request_hash = hash(&serde_json::to_vec(&request)?);
    let mut wal = SeedWal {
        schema: 1,
        baseline_sha256: baseline_hash.clone(),
        request_sha256: request_hash.clone(),
        dispatched: 0,
        seeds: initial_seeds,
    };
    envelope(WAL_MAGIC, &wal)?;
    // Refuse inadequate recovery capacity before writing the seed WAL or
    // dispatching any world. Ordinary writers cannot spend this reservation.
    let _recovery_capacity = budget.reserve(RECOVERY_HEADROOM)?;
    // The WAL's directory entry must itself survive before dispatch. Flushing
    // children and .run alone does not persist .run in its Unix parent.
    sync_directory(&store)?;
    write_new(
        &run.join("run.json"),
        &serde_json::to_vec(
            &serde_json::json!({"schema":1,"kind":"native-resident-serial-run-v1","pid":std::process::id(),"baseline_sha256":baseline_hash,"request_sha256":request_hash}),
        )?,
    )?;
    observe("owner-written", &store)?;
    let mut manifest = Manifest {
        schema: 1,
        baseline_sha256: baseline_hash.clone(),
        request_sha256: request_hash.clone(),
        request: request.clone(),
        snapshot: None,
        previous_snapshot: None,
    };
    write_metadata(&run.join("seed-wal.bin"), WAL_MAGIC, &wal)?;
    observe("wal-written", &store)?;
    write_metadata(&run.join("resident-frontier.bin"), MAN_MAGIC, &manifest)?;
    observe("frontier-written", &store)?;
    let admitted = baseline.handoff.clone().admit()?;
    let mut state = Some(AdmittedCycleState {
        asset: admitted.foundation,
        policy_version: admitted
            .policy_version
            .checked_add(1)
            .ok_or("policy overflow")?,
        restored_actor: Some(admitted.actor),
        restored_value: Some(admitted.value),
        founder_seed_base: baseline.founder_seed_base,
        objective_state_reset: false,
        objective_transition_from: None,
        resident: None,
    });
    // The marker is the durable proof that no world has dispatched. Only retire
    // it after ALL admission/initialization succeeded, before the first consume.
    if fs::read(store.join(INITIALIZATION))? != initialization
        || hash(&fs::read(&current)?) != baseline_hash
    {
        return Err("resident initialization/current changed before dispatch".into());
    }
    alife_storage::remove_file(store.join(INITIALIZATION))?;
    sync_directory(&store)?;
    drop(initialization_capacity);
    observe("initialization-complete", &store)?;
    let mut completed_count = 0usize;
    let mut trained_rows = 0usize;
    let mut last_policy = baseline_policy;
    let mut stopped = false;
    for (index, row) in request.rows.iter().enumerate() {
        if now_seconds()? >= request.collection_cutoff_unix_seconds || run.join("STOP").exists() {
            stopped = true;
            break;
        }
        if logical_bytes(&run)? > request.max_temporary_bytes {
            return Err("resident temporary byte guard closed".into());
        }
        let seed = request.seed_range[0]
            .checked_add(index as u64)
            .ok_or("seed overflow")?;
        wal.seeds.consume(seed)?;
        wal.dispatched += 1;
        // The small checksummed cursor commits BEFORE any world/brain work.
        write_metadata(&run.join("seed-wal.bin"), WAL_MAGIC, &wal)?;
        let checkpoint_due =
            (index + 1) % request.checkpoint_every_episodes == 0 || index + 1 == request.rows.len();
        let episode = run.join(format!("episode-{index:05}"));
        let timing = CycleStageTiming::from_environment(
            || serde_json::json!({"kind":"resident-episode","seed":seed,"checkpoint_due":checkpoint_due}),
        );
        let admitted = state
            .take()
            .ok_or("resident learner missing between episodes")?;
        let outcome = crate::foundation_training_output::in_new_directory(
            &episode,
            || {
                timing.own_output(&episode);
                run_foundation_training_cycle_in_owned_output(
                    &episode,
                    seed,
                    row.decisions,
                    None,
                    Some(row.lesson),
                    admitted,
                    request.sampling_temperature,
                    0,
                    None,
                    false,
                    &timing,
                    None,
                    checkpoint_due,
                    row.food_need_condition,
                    true,
                )
            },
            |error| {
                record_cycle_failure(&episode, error, seed, row.decisions, None, Some(row.lesson))
            },
        )?;
        let FoundationCycleOutcome::Resident(completed) = outcome else {
            return Err("resident cycle returned a nonresident result".into());
        };
        completed_count += 1;
        trained_rows = trained_rows
            .checked_add(completed.receipt.trained_replay_rows)
            .ok_or("resident row count overflow")?;
        last_policy = completed.receipt.policy_version;
        emit(row_summary(&completed.receipt));
        if checkpoint_due {
            snapshot_episode(
                &run,
                &episode,
                &completed,
                &wal,
                &mut manifest,
                trained_rows,
            )?;
            if hash(&fs::read(&current)?) != baseline_hash {
                return Err("immutable resident baseline changed".into());
            }
        }
        let ResidentEpisode { learner, .. } = *completed;
        let ResidentLearner {
            asset,
            next_policy,
            founder_seed_base,
            trainer,
            value,
        } = learner;
        state = Some(AdmittedCycleState {
            asset: asset.clone(),
            policy_version: next_policy,
            restored_actor: None,
            restored_value: None,
            founder_seed_base,
            objective_state_reset: false,
            objective_transition_from: None,
            resident: Some(ResidentLearner {
                asset,
                next_policy,
                founder_seed_base,
                trainer,
                value,
            }),
        });
        timing.finish(None);
        remove_episode(&run, &episode)?;
    }
    // No late private capture: an unexpected stop may lose recent provisional
    // work. A planned final row always took the original pre-PPO capture path.
    drop(state);
    Ok(
        serde_json::json!({"kind":"resident-serial-finished-v1","stopped_at_boundary":stopped,
        "baseline_policy":baseline_policy,"provisional_policy":last_policy,"completed_provisional_episodes":completed_count,
        "trained_provisional_rows":trained_rows,"last_validated_snapshot":manifest.snapshot,
        "seed_wal":run.join("seed-wal.bin"),"canonical_current_unchanged":hash(&fs::read(current)?)==baseline_hash,
        "canonical_current_published":false,"uncheckpointed_learning_may_be_lost":true}),
    )
}

/// Device-free stopped-run reconciliation. It never resumes a world or changes
/// canonical current; the existing owner/publisher must admit this candidate.
pub fn recover_resident_serial_checkpoint(store: &Path) -> Result<serde_json::Value> {
    let _lease = store_lease(store)?;
    let store = store.canonicalize()?;
    if continuous_store::owns(&store)? {
        return continuous_store::recover_locked(&store);
    }
    if initialization_pending(&store)? {
        return initialization::abort(&store);
    }
    if retirement_pending(&store)? {
        drop(_lease);
        let finished = finish_resident_serial_run(&store)?;
        let current = store.join("current.alife-checkpoint");
        return Ok(
            serde_json::json!({"kind":"resident-recovery-v1", "already_published_current":true,
            "candidate":current,"candidate_sha256":finished["current_sha256"],
            "cleanup_completed":true,"temporary_budget":null,"neural_execution":false}),
        );
    }
    let run = store.join(".run");
    directory(&run)?;
    let manifest: Manifest = read_metadata(&run.join("resident-frontier.bin"), MAN_MAGIC)?;
    let wal: SeedWal = read_metadata(&run.join("seed-wal.bin"), WAL_MAGIC)?;
    if manifest.schema != 1
        || wal.schema != 1
        || wal.baseline_sha256 != manifest.baseline_sha256
        || wal.request_sha256 != manifest.request_sha256
        || hash(&serde_json::to_vec(&manifest.request)?) != manifest.request_sha256
    {
        return Err("resident recovery source/request binding mismatch".into());
    }
    validate_request(
        &manifest.request,
        manifest
            .request
            .collection_cutoff_unix_seconds
            .saturating_sub(1),
    )?;
    let budget = alife_storage::OutputBudget::register(&run, manifest.request.max_temporary_bytes)?;
    let _validation = current::validation_scope(&run)?;
    // Producer is stopped and its lease is held. This is only the exact
    // unpublished derived staging path, never a committed snapshot/candidate.
    let pending = run.join("recovered-current.pending");
    if pending.exists() {
        regular(&pending, alife_training::MAX_CURRENT_CHECKPOINT_BYTES)?;
        alife_storage::remove_file(&pending)?;
    }
    let current = store.join("current.alife-checkpoint");
    regular(&current, alife_training::MAX_CURRENT_CHECKPOINT_BYTES)?;
    let current_hash = hash(&fs::read(&current)?);
    let recovery_path = run.join("recovered-current.alife-checkpoint");
    if current_hash != manifest.baseline_sha256 {
        let intent: serde_json::Value = read_metadata(&run.join("recovery-intent.bin"), REC_MAGIC)?;
        if intent["baseline_sha256"] != manifest.baseline_sha256
            || intent["request_sha256"] != manifest.request_sha256
            || intent["after_sha256"] != current_hash
        {
            return Err("canonical current changed outside this resident recovery".into());
        }
        read_current_training_checkpoint(&current)?;
        let temporary_budget = temporary_budget_report(&budget)?;
        return Ok(
            serde_json::json!({"kind":"resident-recovery-v1","already_published_current":true,"candidate":current,"candidate_sha256":current_hash,"temporary_budget":temporary_budget,"neural_execution":false}),
        );
    }
    let baseline = read_current_training_checkpoint(&current)?;
    if seeds_after(&baseline.seeds, &manifest.request, wal.dispatched)? != wal.seeds {
        return Err("resident WAL cursor/exclusion state is invalid".into());
    }
    let mut checkpoint = if let Some(snapshot) = &manifest.snapshot {
        if snapshot.file != format!("checkpoint-{}.alife-checkpoint", snapshot.policy)
            || snapshot.completed_episodes > wal.dispatched
            || snapshot.completed_episodes == 0
            || snapshot.completed_episodes > manifest.request.rows.len()
        {
            return Err("invalid resident snapshot reference".into());
        }
        let path = run.join(&snapshot.file);
        regular(&path, alife_training::MAX_CURRENT_CHECKPOINT_BYTES)?;
        if hash(&fs::read(&path)?) != snapshot.sha256 {
            return Err("resident committed snapshot changed".into());
        }
        let saved = read_current_training_checkpoint(&path)?;
        if saved.handoff.policy_version
            != baseline
                .handoff
                .policy_version
                .checked_add(snapshot.completed_episodes as u64)
                .ok_or("policy overflow")?
            || saved.handoff.policy_version != snapshot.policy
            || saved.seed
                != manifest.request.seed_range[0]
                    .checked_add(snapshot.completed_episodes as u64 - 1)
                    .ok_or("snapshot seed overflow")?
            || saved.seeds
                != seeds_after(
                    &baseline.seeds,
                    &manifest.request,
                    snapshot.completed_episodes,
                )?
            || saved.founder_seed_base != baseline.founder_seed_base
        {
            return Err("resident snapshot lineage/cursor mismatch".into());
        }
        saved
    } else {
        baseline
    };
    checkpoint.seeds = wal.seeds;
    checkpoint.validate()?;
    let bytes = checkpoint.encode()?;
    let after = hash(&bytes);
    publish_recovery_candidate(&recovery_path, &bytes)?;
    if read_current_training_checkpoint(&recovery_path)? != checkpoint {
        return Err("resident recovery candidate changed".into());
    }
    write_metadata(
        &run.join("recovery-intent.bin"),
        REC_MAGIC,
        &serde_json::json!({"baseline_sha256":manifest.baseline_sha256,"request_sha256":manifest.request_sha256,"after_sha256":after}),
    )?;
    let temporary_budget = temporary_budget_report(&budget)?;
    Ok(
        serde_json::json!({"kind":"resident-recovery-v1","candidate":recovery_path,"candidate_sha256":after,
        "policy_version":checkpoint.handoff.policy_version,"actor_optimizer_step":checkpoint.handoff.actor.optimizer_step,
        "value_optimizer_step":checkpoint.handoff.value.optimizer_step,"seeds":checkpoint.seeds,
        "already_published_current":false,"neural_execution":false,"weights_modified":false,
        "uncheckpointed_updates_restored":false,"canonical_current_published":false,"temporary_budget":temporary_budget}),
    )
}

fn temporary_budget_report(budget: &alife_storage::OutputBudget) -> Result<serde_json::Value> {
    let usage = budget.usage()?;
    Ok(
        serde_json::json!({"used":usage.used,"reserved":usage.reserved,"high_water":usage.high_water,
        "limit":usage.limit,"scope":"STORE/.run logical file bytes","sample":"this recovery invocation"}),
    )
}

fn cleanup_entry_is_owned(name: &str, is_directory: bool, dispatched: usize) -> bool {
    if is_directory {
        return (0..dispatched).any(|index| name == format!("episode-{index:05}"));
    }
    matches!(
        name,
        "run.json"
            | "seed-wal.bin"
            | "seed-wal.next"
            | "resident-frontier.bin"
            | "resident-frontier.next"
            | "recovered-current.alife-checkpoint"
            | "recovered-current.pending"
            | "recovery-intent.bin"
            | "recovery-intent.next"
            | "STOP"
    ) || name
        .strip_prefix("checkpoint-")
        .and_then(|value| value.strip_suffix(".alife-checkpoint"))
        .is_some_and(|value| !value.is_empty() && value.bytes().all(|c| c.is_ascii_digit()))
}

/// Retires only this native run, after the external owner published its exact
/// admitted recovery candidate. It never publishes or resumes a world itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RetiredEntry {
    name: String,
    directory: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    binding_sha256: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Retirement {
    schema: u16,
    kind: String,
    store: std::path::PathBuf,
    current_sha256: String,
    policy_version: u64,
    baseline_sha256: String,
    request_sha256: String,
    dispatched: usize,
    retained: usize,
    max_temporary_bytes: u64,
    entries: Vec<RetiredEntry>,
}
fn entry_present(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}
fn retirement_pending(store: &Path) -> Result<bool> {
    Ok(entry_present(&store.join(RETIREMENT))? || entry_present(&store.join(RETIREMENT_NEXT))?)
}
fn validate_retirement(record: &Retirement, store: &Path, current_hash: &str) -> Result<()> {
    let digest = |value: &str| value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit());
    if record.schema != 1
        || record.kind != "native-resident-retirement-v1"
        || record.store != store
        || record.current_sha256 != current_hash
        || !digest(&record.current_sha256)
        || !digest(&record.baseline_sha256)
        || !digest(&record.request_sha256)
        || record.dispatched > MAX_EPISODES
        || record.retained > record.dispatched
        || record.max_temporary_bytes < RECOVERY_HEADROOM + STARTUP_METADATA_HEADROOM
    {
        return Err("resident retirement no longer binds the admitted current/store".into());
    }
    let mut names = std::collections::BTreeSet::new();
    for entry in &record.entries {
        if !names.insert(entry.name.as_str())
            || !(cleanup_entry_is_owned(&entry.name, entry.directory, record.dispatched)
                || entry.name == "alife-current-private-validation-v1" && entry.directory)
            || entry
                .binding_sha256
                .as_ref()
                .is_some_and(|d| !digest(d) || entry.directory)
        {
            return Err("invalid resident retirement entry inventory".into());
        }
    }
    if [
        "run.json",
        "seed-wal.bin",
        "resident-frontier.bin",
        "recovery-intent.bin",
        "recovered-current.alife-checkpoint",
    ]
    .iter()
    .any(|name| {
        !record
            .entries
            .iter()
            .any(|e| e.name == *name && !e.directory && e.binding_sha256.is_some())
    }) {
        return Err("resident retirement lacks admitted binding entries".into());
    }
    Ok(())
}
fn validate_retired_remainder(run: &Path, record: &Retirement) -> Result<()> {
    directory(run)?;
    for item in fs::read_dir(run)? {
        let item = item?;
        let metadata = fs::symlink_metadata(item.path())?;
        let name = item
            .file_name()
            .into_string()
            .map_err(|_| "non-UTF8 retired entry")?;
        let entry = record
            .entries
            .iter()
            .find(|e| e.name == name)
            .ok_or("new unknown entry appeared during resident retirement")?;
        if reparse(&metadata)
            || metadata.is_dir() != entry.directory
            || !(metadata.is_file() || metadata.is_dir())
        {
            return Err("retired entry changed type or became an alias".into());
        }
        if let Some(expected) = &entry.binding_sha256 {
            let limit = if name == "recovered-current.alife-checkpoint" {
                alife_training::MAX_CURRENT_CHECKPOINT_BYTES
            } else {
                (MAX_METADATA + 56) as u64
            };
            regular(&item.path(), limit)?;
            if hash(&fs::read(item.path())?) != *expected {
                return Err("retired binding entry changed after cleanup commitment".into());
            }
        }
        if name == "alife-current-private-validation-v1" {
            // Its only initially admitted child was the exact lease marker.
            // Missing children are normal after an interrupted recursive delete.
            for child in fs::read_dir(item.path())? {
                let child = child?;
                if child.file_name() != "lease" {
                    return Err("unknown retired validation scratch entry".into());
                }
                regular(&child.path(), 128)?;
                if fs::read(child.path())? != b"alife-current-private-validation-v1\n" {
                    return Err("retired validation lease changed".into());
                }
            }
        }
    }
    logical_bytes(run)?;
    Ok(())
}
pub fn finish_resident_serial_run(store: &Path) -> Result<serde_json::Value> {
    finish_resident_with_remove(store, |run| alife_storage::remove_dir_all(run))
}
fn finish_resident_with_remove(
    store: &Path,
    remove: impl FnOnce(&Path) -> std::io::Result<()>,
) -> Result<serde_json::Value> {
    let _lease = store_lease(store)?;
    let store = store.canonicalize()?;
    if continuous_store::owns(&store)? {
        return continuous_store::finish_locked(&store);
    }
    if initialization_pending(&store)? {
        return initialization::abort(&store);
    }
    let run = store.join(".run");
    let current = store.join("current.alife-checkpoint");
    regular(&current, alife_training::MAX_CURRENT_CHECKPOINT_BYTES)?;
    let current_hash = hash(&fs::read(&current)?);
    let marker = store.join(RETIREMENT);
    let pending = store.join(RETIREMENT_NEXT);
    let committed = entry_present(&marker)?;
    let run_exists = match fs::symlink_metadata(&run) {
        Ok(_) => true,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => return Err(e.into()),
    };
    if !committed && !run_exists {
        if entry_present(&pending)? {
            return Err("uncommitted resident retirement has no intact run".into());
        }
        let checkpoint = read_current_training_checkpoint(&current)?;
        return Ok(
            serde_json::json!({"kind":"resident-cleanup-v1","removed_run":false,"already_clean":true,
            "current_sha256":current_hash,"policy_version":checkpoint.handoff.policy_version,
            "neural_execution":false,"canonical_current_modified":false}),
        );
    }
    let (record, budget) = if committed {
        let record: Retirement = read_metadata(&marker, RET_MAGIC)?;
        validate_retirement(&record, &store, &current_hash)?;
        // Exact bytes were fully Native-admitted before this durable commitment.
        // Do not recreate private scratch inside a partially deleted run.
        let budget = if run_exists {
            directory(&run)?;
            Some(alife_storage::OutputBudget::register(
                &run,
                record.max_temporary_bytes,
            )?)
        } else {
            None
        };
        (record, budget)
    } else {
        let (record, budget) = prepare_retirement(&store, &current_hash)?;
        validate_retirement(&record, &store, &current_hash)?;
        (record, Some(budget))
    };
    let bytes = envelope(RET_MAGIC, &record)?;
    let pending_bytes = if entry_present(&pending)? {
        regular(&pending, (MAX_METADATA + 56) as u64)?;
        let existing = fs::read(&pending)?;
        if existing != bytes && !bytes.starts_with(&existing) {
            return Err("complete/conflicting retirement staging; preserving evidence".into());
        }
        existing.len() as u64
    } else {
        0
    };
    let outside_bytes = if committed {
        fs::metadata(&marker)?.len() + pending_bytes
    } else {
        bytes.len() as u64 + pending_bytes
    };
    // Small outside-tree authority must survive deletion. Charge its actual
    // bounded staging bytes to the same allowance while this process owns it.
    let _outside_capacity = budget
        .as_ref()
        .map(|b| b.reserve(outside_bytes))
        .transpose()?;
    if run_exists {
        validate_retired_remainder(&run, &record)?;
    }
    if !committed {
        write_metadata(&marker, RET_MAGIC, &record)?;
    }
    if entry_present(&pending)? {
        regular(&pending, (MAX_METADATA + 56) as u64)?;
        alife_storage::remove_file(&pending)?;
    }
    // This durable outside-tree record now remains even if deletion removes all
    // original WAL/manifest/owner files before returning an error or dying.
    if run_exists {
        remove(&run)?;
        sync_directory(&store)?;
    }
    if hash(&fs::read(&current)?) != record.current_sha256 {
        return Err("canonical current changed during retirement; marker retained".into());
    }
    alife_storage::remove_file(&marker)?;
    sync_directory(&store)?;
    Ok(
        serde_json::json!({"kind":"resident-cleanup-v1","removed_run":run_exists,"already_clean":!run_exists,
        "retirement_completed":true,"current_sha256":current_hash,"policy_version":record.policy_version,
        "dispatched_worlds":record.dispatched,"retained_episodes":record.retained,
        "uncheckpointed_work_may_be_lost":record.dispatched>record.retained,
        "neural_execution":false,"canonical_current_modified":false}),
    )
}

fn prepare_retirement(
    store: &Path,
    current_hash: &str,
) -> Result<(Retirement, alife_storage::OutputBudget)> {
    let current = store.join("current.alife-checkpoint");
    let run = store.join(".run");
    directory(&run)?;
    let manifest: Manifest = read_metadata(&run.join("resident-frontier.bin"), MAN_MAGIC)?;
    validate_request(
        &manifest.request,
        manifest
            .request
            .collection_cutoff_unix_seconds
            .saturating_sub(1),
    )?;
    let budget = alife_storage::OutputBudget::register(&run, manifest.request.max_temporary_bytes)?;
    let _validation = current::validation_scope(&run)?;
    let checkpoint = read_current_training_checkpoint(&current)?;
    let wal: SeedWal = read_metadata(&run.join("seed-wal.bin"), WAL_MAGIC)?;
    let intent: serde_json::Value = read_metadata(&run.join("recovery-intent.bin"), REC_MAGIC)?;
    regular(&run.join("run.json"), MAX_METADATA as u64)?;
    let owner: serde_json::Value = serde_json::from_slice(&fs::read(run.join("run.json"))?)?;
    let candidate = run.join("recovered-current.alife-checkpoint");
    regular(&candidate, alife_training::MAX_CURRENT_CHECKPOINT_BYTES)?;
    if manifest.schema != 1
        || wal.schema != 1
        || owner["schema"] != 1
        || owner["kind"] != "native-resident-serial-run-v1"
        || owner["baseline_sha256"] != manifest.baseline_sha256
        || owner["request_sha256"] != manifest.request_sha256
        || wal.baseline_sha256 != manifest.baseline_sha256
        || wal.request_sha256 != manifest.request_sha256
        || hash(&serde_json::to_vec(&manifest.request)?) != manifest.request_sha256
        || intent["baseline_sha256"] != manifest.baseline_sha256
        || intent["request_sha256"] != manifest.request_sha256
        || intent["after_sha256"] != current_hash
        || hash(&fs::read(&candidate)?) != current_hash
        || read_current_training_checkpoint(&candidate)? != checkpoint
        || wal.dispatched > manifest.request.rows.len()
        || checkpoint.seeds != wal.seeds
    {
        return Err(
            "resident cleanup is not bound to the published full current/seed cursor".into(),
        );
    }
    let mut entries = Vec::new();
    for entry in fs::read_dir(&run)? {
        let entry = entry?;
        let metadata = fs::symlink_metadata(entry.path())?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "non-UTF8 resident cleanup entry")?;
        if name == "alife-current-private-validation-v1" {
            validate_completed_validation_workspace(&entry.path())?;
        } else if reparse(&metadata)
            || !cleanup_entry_is_owned(&name, metadata.is_dir(), wal.dispatched)
        {
            return Err("unknown resident cleanup entry; preserving the run".into());
        }
        let binding_sha256 = if name == "recovered-current.alife-checkpoint" {
            Some(current_hash.to_string())
        } else if matches!(
            name.as_str(),
            "run.json" | "seed-wal.bin" | "resident-frontier.bin" | "recovery-intent.bin"
        ) {
            Some(hash(&fs::read(entry.path())?))
        } else {
            None
        };
        entries.push(RetiredEntry {
            name,
            directory: metadata.is_dir(),
            binding_sha256,
        });
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    logical_bytes(&run)?;
    let retirement = Retirement {
        schema: 1,
        kind: "native-resident-retirement-v1".into(),
        store: store.to_path_buf(),
        current_sha256: current_hash.to_string(),
        policy_version: checkpoint.handoff.policy_version,
        baseline_sha256: manifest.baseline_sha256,
        request_sha256: manifest.request_sha256,
        dispatched: wal.dispatched,
        retained: manifest
            .snapshot
            .as_ref()
            .map_or(0, |s| s.completed_episodes),
        max_temporary_bytes: manifest.request.max_temporary_bytes,
        entries,
    };
    envelope(RET_MAGIC, &retirement)?;
    Ok((retirement, budget))
}

fn validate_completed_validation_workspace(path: &Path) -> Result<()> {
    directory(path)?;
    let entries = fs::read_dir(path)?.collect::<std::io::Result<Vec<_>>>()?;
    if entries.len() != 1 || entries[0].file_name() != "lease" {
        return Err("unknown or active private admission workspace; preserving run".into());
    }
    let lease = path.join("lease");
    regular(&lease, 128)?;
    if fs::read(lease)? != b"alife-current-private-validation-v1\n" {
        return Err("private admission workspace lease marker differs".into());
    }
    Ok(())
}

#[cfg(test)]
mod quota_tests;
#[cfg(test)]
mod retirement_tests;

#[cfg(test)]
pub(super) fn stage_resident_loss_fixture(
    store: &Path,
    request: ResidentSerialRequest,
    dispatched: usize,
) -> Result<()> {
    let baseline_path = store.join("current.alife-checkpoint");
    let baseline = read_current_training_checkpoint(&baseline_path)?;
    let baseline_sha256 = hash(&fs::read(baseline_path)?);
    let request_sha256 = hash(&serde_json::to_vec(&request)?);
    let seeds = seeds_after(&baseline.seeds, &request, dispatched)?;
    let run = store.join(".run");
    fs::create_dir(&run)?;
    write_new(
        &run.join("run.json"),
        &serde_json::to_vec(&serde_json::json!({
            "schema":1,"kind":"native-resident-serial-run-v1","pid":std::process::id(),
            "baseline_sha256":baseline_sha256,"request_sha256":request_sha256
        }))?,
    )?;
    write_metadata(
        &run.join("seed-wal.bin"),
        WAL_MAGIC,
        &SeedWal {
            schema: 1,
            baseline_sha256: baseline_sha256.clone(),
            request_sha256: request_sha256.clone(),
            dispatched,
            seeds,
        },
    )?;
    write_metadata(
        &run.join("resident-frontier.bin"),
        MAN_MAGIC,
        &Manifest {
            schema: 1,
            baseline_sha256,
            request_sha256,
            request,
            snapshot: None,
            previous_snapshot: None,
        },
    )
}

#[cfg(test)]
mod carry_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resident_compact_summary_preserves_dose_prior_and_unknown_nourishment() {
        let receipt: FoundationCycleReceipt = serde_json::from_value(serde_json::json!({
            "seed":17,"policy_version":4,"training_ticks":10,
            "requested_training_ticks":512,"captured_replay_rows":10,
            "trained_replay_rows":10,"bootstrap_replay_rows":0,
            "terminal_death_tick":21,"lesson":"feeding","consumed_events":2,
            "actor_optimizer_updates":2,"value_optimizer_updates":3,
            "collection_seconds":1.0,"update_seconds":1.0,
            "old_asset_digest":"old","new_asset_digest":"new",
            "actor_optimizer_step":7,"value_optimizer_step":5,"completed_epochs":1,
            "next_cohort_tick_captured":true,"next_cohort_optimizer_rebound":true,
            "semantic_prior":{"backend":"deterministic","decision_frames":10,
                "decision_frames_with_prior":8,"decision_frames_without_prior":2,
                "empty_hint_frames":2,"failures":2,"requests":3}
        }))
        .unwrap();
        let row = row_summary(&receipt);
        assert_eq!(row["requested_training_ticks"], 512);
        assert_eq!(row["training_ticks"], 10);
        assert_eq!(row["captured_replay_rows"], 10);
        assert_eq!(row["bootstrap_replay_rows"], 0);
        assert_eq!(row["terminal_death_tick"], 21);
        assert_eq!(row["actor_optimizer_updates"], 2);
        assert_eq!(row["value_optimizer_updates"], 3);
        assert_eq!(row["generic_consumed_events"], 2);
        assert!(row["exact_trained_nourishing_events"].is_null());
        assert!(row["causal_collection"].is_null());
        assert_eq!(row["semantic_prior"]["decision_frames_with_prior"], 8);
        assert_eq!(row["semantic_prior"]["failures"], 2);
        for raw in [
            "decision_inputs",
            "deliveries",
            "provider_failures",
            "last_priming",
        ] {
            assert!(row["semantic_prior"].get(raw).is_none());
        }
    }
    fn request() -> ResidentSerialRequest {
        ResidentSerialRequest {
            schema: 1,
            seed_range: [100, 104],
            rows: vec![
                ResidentSerialRow {
                    lesson: FoundationTeacherLesson::Feeding,
                    decisions: 16,
                    food_need_condition: Default::default(),
                };
                3
            ],
            checkpoint_every_episodes: 3,
            sampling_temperature: 1.0,
            collection_cutoff_unix_seconds: 200,
            max_temporary_bytes: 1024 * 1024 * 1024,
        }
    }
    #[test]
    fn resident_dispatch_cursor_survives_lost_work_without_reusing_worlds() {
        let base = CheckpointSeedsV1 {
            historical_freshness: "unknown".into(),
            excluded_ranges: vec![[1, 99]],
            pending_range: None,
        };
        let r = request();
        let mut live = seeds_after(&base, &r, 0).unwrap();
        for dispatched in 0..3 {
            live.consume(100 + dispatched as u64).unwrap();
            assert_eq!(live, seeds_after(&base, &r, dispatched + 1).unwrap());
        }
        assert_eq!(live.pending_range, Some([103, 104]));
        assert_eq!(live.excluded_ranges, vec![[1, 104]]);
        assert!(live.consume(101).is_err());
        assert!(seeds_after(&base, &r, 4).is_err());
        assert_eq!(base.pending_range, None);
    }
    #[test]
    fn resident_request_requires_explicit_valid_cadence_cutoff_and_care_rows() {
        let r = request();
        validate_request(&r, 100).unwrap();
        let mut bad = r.clone();
        bad.checkpoint_every_episodes = 0;
        assert!(validate_request(&bad, 100).is_err());
        let mut bad = r.clone();
        bad.collection_cutoff_unix_seconds = 100;
        assert!(validate_request(&bad, 100).is_err());
        let mut bad = r.clone();
        bad.seed_range = [100, 101];
        assert!(validate_request(&bad, 100).is_err());
        let mut bad = r.clone();
        bad.rows[0].decisions = 36_001;
        assert!(validate_request(&bad, 100).is_err());
        let mut bad = r;
        bad.sampling_temperature = f32::NAN;
        assert!(validate_request(&bad, 100).is_err());
    }
    #[test]
    fn resident_long_world_is_one_explicit_feeding_row_with_existing_horizon_bounds() {
        let mut r = request();
        r.rows.truncate(1);
        r.rows[0].decisions = 512;
        validate_request(&r, 100).unwrap();
        assert_eq!(r.rows.len(), 1);
        r.rows[0].decisions = 36_000;
        validate_request(&r, 100).unwrap();
        r.rows[0].decisions = 36_001;
        assert!(validate_request(&r, 100).is_err());
        for lesson in [
            FoundationTeacherLesson::EatHeldFood,
            FoundationTeacherLesson::EatReachableFood,
            FoundationTeacherLesson::RelocatedFood,
        ] {
            r.rows[0].lesson = lesson;
            r.rows[0].decisions = 129;
            assert!(validate_request(&r, 100).is_err());
            r.rows[0].decisions = 128;
            validate_request(&r, 100).unwrap();
        }
    }
    #[test]
    fn resident_cleanup_rejects_unknown_or_undispatched_paths() {
        assert!(cleanup_entry_is_owned("episode-00001", true, 2));
        assert!(!cleanup_entry_is_owned("episode-00002", true, 2));
        assert!(!cleanup_entry_is_owned("../episode-00001", true, 2));
        assert!(cleanup_entry_is_owned(
            "checkpoint-10977.alife-checkpoint",
            false,
            2
        ));
        assert!(!cleanup_entry_is_owned(
            "checkpoint-../current.alife-checkpoint",
            false,
            2
        ));
        assert!(!cleanup_entry_is_owned("personal-file", false, 2));
        assert!(!cleanup_entry_is_owned("recovery-intent.bin", true, 2));
    }
    #[test]
    fn resident_metadata_rejects_partial_corrupt_and_cross_kind_records() {
        let root = std::env::temp_dir().join(format!(
            "resident-meta-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let path = root.join("wal.bin");
        let value = serde_json::json!({"dispatched": 2});
        write_metadata(&path, WAL_MAGIC, &value).unwrap();
        assert_eq!(
            read_metadata::<serde_json::Value>(&path, WAL_MAGIC).unwrap(),
            value
        );
        assert!(read_metadata::<serde_json::Value>(&path, MAN_MAGIC).is_err());
        let bytes = fs::read(&path).unwrap();
        alife_storage::write(&path, &bytes[..bytes.len() - 1]).unwrap();
        assert!(read_metadata::<serde_json::Value>(&path, WAL_MAGIC).is_err());
        let mut corrupt = bytes;
        *corrupt.last_mut().unwrap() ^= 1;
        alife_storage::write(&path, corrupt).unwrap();
        assert!(read_metadata::<serde_json::Value>(&path, WAL_MAGIC).is_err());
        alife_storage::remove_dir_all(root).unwrap();
    }
    #[test]
    fn resident_writer_lease_excludes_concurrent_producer_and_recovery() {
        let root = std::env::temp_dir().join(format!(
            "resident-lock-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let lease = store_lease(&root).unwrap();
        assert!(store_lease(&root).is_err());
        drop(lease);
        drop(store_lease(&root).unwrap());
        alife_storage::write(root.join("unknown-writer-intent"), b"pending").unwrap();
        assert!(store_lease(&root).is_err());
        alife_storage::remove_dir_all(root).unwrap();
    }
}
