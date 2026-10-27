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
        if name != "current.alife-checkpoint" && name != ".current.writer.lock" && name != ".run" {
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
        || request.max_temporary_bytes <= RECOVERY_HEADROOM
    {
        return Err("invalid resident request/cadence/seed range/cutoff/budget".into());
    }
    cycle_ppo_config(request.sampling_temperature)?;
    for row in &request.rows {
        if !(1..=128).contains(&row.decisions)
            || !matches!(
                row.lesson,
                FoundationTeacherLesson::EatHeldFood
                    | FoundationTeacherLesson::EatReachableFood
                    | FoundationTeacherLesson::Feeding
                    | FoundationTeacherLesson::RelocatedFood
            )
        {
            return Err("resident serial supports the existing bounded care-first lessons".into());
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
    serde_json::json!({"kind":"resident-provisional-episode-v1","seed":receipt.seed,"policy_version":receipt.policy_version,
        "lesson":receipt.lesson,"trained_rows":receipt.trained_replay_rows,"exact_trained_nourishing_events":exact,
        "generic_consumed_events":receipt.consumed_events,"full_relocation":r["food_relocation"]["lesson_completed"],
        "actor_optimizer_step":receipt.actor_optimizer_step,"value_optimizer_step":receipt.value_optimizer_step,
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
    validate_request(&request, now_seconds()?)?;
    let _lease = store_lease(store)?;
    let store = store.canonicalize()?;
    let current = store.join("current.alife-checkpoint");
    regular(&current, alife_training::MAX_CURRENT_CHECKPOINT_BYTES)?;
    let baseline_bytes = fs::read(&current)?;
    let baseline_hash = hash(&baseline_bytes);
    let run = store.join(".run");
    fs::create_dir(&run)?;
    let budget = alife_storage::OutputBudget::register(&run, request.max_temporary_bytes)?;
    let _validation = current::validation_scope(&run)?;
    let baseline = read_current_training_checkpoint(&current)?;
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
    let mut manifest = Manifest {
        schema: 1,
        baseline_sha256: baseline_hash.clone(),
        request_sha256: request_hash.clone(),
        request: request.clone(),
        snapshot: None,
        previous_snapshot: None,
    };
    write_metadata(&run.join("seed-wal.bin"), WAL_MAGIC, &wal)?;
    write_metadata(&run.join("resident-frontier.bin"), MAN_MAGIC, &manifest)?;
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
        return Ok(
            serde_json::json!({"kind":"resident-recovery-v1","already_published_current":true,"candidate":current,"candidate_sha256":current_hash,"neural_execution":false}),
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
    let usage = budget.usage()?;
    emit(
        serde_json::json!({"kind":"resident-temporary-budget-v1", "logical_bytes":usage.used,
        "high_water_bytes":usage.high_water,"limit_bytes":usage.limit}),
    );
    Ok(
        serde_json::json!({"kind":"resident-recovery-v1","candidate":recovery_path,"candidate_sha256":after,
        "policy_version":checkpoint.handoff.policy_version,"actor_optimizer_step":checkpoint.handoff.actor.optimizer_step,
        "value_optimizer_step":checkpoint.handoff.value.optimizer_step,"seeds":checkpoint.seeds,
        "already_published_current":false,"neural_execution":false,"weights_modified":false,
        "uncheckpointed_updates_restored":false,"canonical_current_published":false}),
    )
}

#[cfg(test)]
mod quota_tests;

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
        bad.rows[0].decisions = 129;
        assert!(validate_request(&bad, 100).is_err());
        let mut bad = r;
        bad.sampling_temperature = f32::NAN;
        assert!(validate_request(&bad, 100).is_err());
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
