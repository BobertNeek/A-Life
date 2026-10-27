//! A once-per-producer, pre-dispatch commitment. It never authorizes an episode.
use super::*;
use std::{collections::BTreeMap, path::PathBuf};

pub(super) const INITIALIZATION: &str = "resident-initialization.bin";
pub(super) const INITIALIZATION_NEXT: &str = "resident-initialization.next";
const MAGIC: &[u8; 16] = b"ALIFE-RES-INI-1\n";
const WORKSPACE: &str = "alife-current-private-validation-v1";
const WORKSPACE_LEASE: &[u8] = b"alife-current-private-validation-v1\n";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Initialization {
    schema: u16,
    kind: String,
    store: PathBuf,
    baseline_sha256: String,
    request_sha256: String,
    request: ResidentSerialRequest,
    pid: u32,
}

pub(super) fn initialization_pending(store: &Path) -> Result<bool> {
    Ok(entry_present(&store.join(INITIALIZATION))?
        || entry_present(&store.join(INITIALIZATION_NEXT))?)
}

pub(super) fn commit(
    store: &Path,
    baseline_sha256: &str,
    request: &ResidentSerialRequest,
    observe: &mut impl FnMut(&str, &Path) -> Result<()>,
) -> Result<Vec<u8>> {
    let bytes = envelope(
        MAGIC,
        &Initialization {
            schema: 1,
            kind: "native-resident-initialization-v1".into(),
            store: store.to_path_buf(),
            baseline_sha256: baseline_sha256.into(),
            request_sha256: hash(&serde_json::to_vec(request)?),
            request: request.clone(),
            pid: std::process::id(),
        },
    )?;
    // No .run exists yet. Check the exact outside staging peak before writing;
    // after .run creation the retained marker has a held budget reservation.
    if (bytes.len() as u64).saturating_mul(2) > request.max_temporary_bytes {
        return Err("resident initialization staging exceeds temporary allowance".into());
    }
    let pending = store.join(INITIALIZATION_NEXT);
    write_new(&pending, &bytes)?;
    observe("initialization-staged", store)?;
    replace(&pending, &store.join(INITIALIZATION))?;
    observe("initialization-committed", store)?;
    Ok(bytes)
}

fn incomplete_stage(bytes: &[u8]) -> bool {
    // A first write may tear before ANY binding bytes. This predicate recognizes
    // only unpublished envelope prefixes, never a complete invalid/different
    // record. Without a committed record it authorizes NO .run cleanup.
    if bytes.len() < MAGIC.len() {
        return MAGIC.starts_with(bytes);
    }
    if &bytes[..16] != MAGIC {
        return false;
    }
    if bytes.len() < 24 {
        return true;
    }
    let length = u64::from_le_bytes(bytes[16..24].try_into().expect("eight bytes"));
    length > 0 && length <= MAX_METADATA as u64 && (bytes.len() as u64) < 56 + length
}

fn report(store: &Path, digest: &str, bound: bool) -> serde_json::Value {
    serde_json::json!({"kind":"resident-initialization-aborted-v1",
        "initialization_aborted":true,"cleanup_completed":true,
        "already_published_current":true,"candidate":store.join("current.alife-checkpoint"),
        "candidate_sha256":digest,"canonical_current_modified":false,
        "initialization_binding_verified":bound,"dispatched_worlds":0,
        "seeds_consumed":0,"new_request_allocation_committed":false,
        "temporary_budget":null,"neural_execution":false})
}

/// Caller owns the stable store lease. Missing expected startup output is safe
/// after interrupted deletion; new/changed/aliased output is always refused.
pub(super) fn abort(store: &Path) -> Result<serde_json::Value> {
    abort_with_remove(store, |run| alife_storage::remove_dir_all(run))
}
fn abort_with_remove(
    store: &Path,
    remove: impl FnOnce(&Path) -> std::io::Result<()>,
) -> Result<serde_json::Value> {
    if retirement_pending(store)? {
        return Err("initialization and retirement commitments conflict".into());
    }
    let current = store.join("current.alife-checkpoint");
    regular(&current, alife_training::MAX_CURRENT_CHECKPOINT_BYTES)?;
    let baseline_bytes = fs::read(&current)?;
    let baseline_hash = hash(&baseline_bytes);
    let marker = store.join(INITIALIZATION);
    let pending = store.join(INITIALIZATION_NEXT);
    let run = store.join(".run");
    let committed = entry_present(&marker)?;
    let run_exists = entry_present(&run)?;
    if !committed {
        if run_exists {
            return Err("uncommitted initialization cannot own any resident run".into());
        }
        regular(&pending, (MAX_METADATA + 56) as u64)?;
        let bytes = fs::read(&pending)?;
        if incomplete_stage(&bytes) {
            // No Current/request claim is possible from a torn first stage.
            // Only this bounded unpublished file is removed; no run is inferred.
            alife_storage::remove_file(&pending)?;
            sync_directory(store)?;
            return Ok(report(store, &baseline_hash, false));
        }
    }
    let source = if committed { &marker } else { &pending };
    let record: Initialization = read_metadata(source, MAGIC)?;
    if record.schema != 1
        || record.kind != "native-resident-initialization-v1"
        || record.store != store
        || record.baseline_sha256 != baseline_hash
        || record.request_sha256 != hash(&serde_json::to_vec(&record.request)?)
        || record.pid == 0
    {
        return Err("resident initialization/current/request binding changed".into());
    }
    validate_request(
        &record.request,
        record
            .request
            .collection_cutoff_unix_seconds
            .saturating_sub(1),
    )?;
    let bytes = envelope(MAGIC, &record)?;
    if fs::read(source)? != bytes {
        return Err("initialization commitment is not the exact native encoding".into());
    }
    let pending_len = if committed && entry_present(&pending)? {
        regular(&pending, (MAX_METADATA + 56) as u64)?;
        let existing = fs::read(&pending)?;
        if !bytes.starts_with(&existing) {
            return Err("conflicting initialization staging; preserving evidence".into());
        }
        existing.len() as u64
    } else {
        0
    };
    let outside_bytes = bytes.len() as u64 + pending_len;
    if outside_bytes > record.request.max_temporary_bytes {
        return Err("initialization metadata exceeds temporary allowance".into());
    }
    let budget = if run_exists {
        directory(&run)?;
        Some(alife_storage::OutputBudget::register(
            &run,
            record.request.max_temporary_bytes,
        )?)
    } else {
        None
    };
    let _outside_capacity = budget
        .as_ref()
        .map(|b| b.reserve(outside_bytes))
        .transpose()?;
    if run_exists {
        validate_startup_tree(&run, &record, &baseline_bytes)?;
        remove(&run)?;
        sync_directory(store)?;
    }
    if hash(&fs::read(&current)?) != baseline_hash {
        return Err("Current changed during initialization abort; commitment retained".into());
    }
    // Keep committed authority until every dependent path is gone. A crash
    // after .run removal, or after staging removal, remains idempotent.
    if entry_present(&pending)? {
        alife_storage::remove_file(&pending)?;
    }
    if committed {
        alife_storage::remove_file(&marker)?;
    }
    sync_directory(store)?;
    Ok(report(store, &baseline_hash, true))
}

fn validate_startup_tree(run: &Path, record: &Initialization, baseline: &[u8]) -> Result<()> {
    let mut expected = BTreeMap::<PathBuf, Vec<u8>>::new();
    expected.insert(
        PathBuf::from("run.json"),
        serde_json::to_vec(&serde_json::json!({"schema":1,"kind":"native-resident-serial-run-v1",
            "pid":record.pid,"baseline_sha256":record.baseline_sha256,"request_sha256":record.request_sha256}))?,
    );
    expected.insert(
        PathBuf::from(WORKSPACE).join("lease"),
        WORKSPACE_LEASE.to_vec(),
    );
    // Structural decoding is only needed if startup got far enough to have
    // derived metadata/private scratch. This is recovery, not a second ordinary
    // producer baseline admission. An invalid input can still abandon an empty
    // explicitly committed initialization without certifying it as admissible.
    if let Ok(baseline) = alife_training::CurrentTrainingCheckpointV1::decode(baseline) {
        if let Ok(seeds) = seeds_after(&baseline.seeds, &record.request, 0) {
            let wal = envelope(
                WAL_MAGIC,
                &SeedWal {
                    schema: 1,
                    baseline_sha256: record.baseline_sha256.clone(),
                    request_sha256: record.request_sha256.clone(),
                    dispatched: 0,
                    seeds,
                },
            );
            let frontier = envelope(
                MAN_MAGIC,
                &Manifest {
                    schema: 1,
                    baseline_sha256: record.baseline_sha256.clone(),
                    request_sha256: record.request_sha256.clone(),
                    request: record.request.clone(),
                    snapshot: None,
                    previous_snapshot: None,
                },
            );
            if let Ok(wal) = wal {
                for name in ["seed-wal.bin", "seed-wal.next"] {
                    expected.insert(PathBuf::from(name), wal.clone());
                }
            }
            if let Ok(frontier) = frontier {
                for name in ["resident-frontier.bin", "resident-frontier.next"] {
                    expected.insert(PathBuf::from(name), frontier.clone());
                }
            }
        }
        if let Some(life) = baseline.life {
            let work = PathBuf::from(WORKSPACE).join("work");
            for name in life.assets_base64.keys() {
                let path = Path::new(name);
                if name.is_empty()
                    || name.contains(['\\', ':'])
                    || name.split('/').any(|p| matches!(p, "" | "." | ".."))
                    || !path
                        .components()
                        .all(|c| matches!(c, std::path::Component::Normal(_)))
                {
                    continue;
                }
                // Native computes/decode-validates bytes before creating this
                // leaf. Failed derivation must not strand an otherwise empty
                // initialization; a leaf without valid expected bytes refuses.
                if let Ok(bytes) = life.asset_bytes(name) {
                    expected.insert(work.join("world-assets").join(path), bytes);
                }
            }
            expected.insert(work.join("world.json"), life.world_json.into_bytes());
            expected.insert(
                work.join(".world.json.sleep-journal-v2.json"),
                life.sleep_journal_json.into_bytes(),
            );
        }
    }
    fn visit(root: &Path, here: &Path, expected: &BTreeMap<PathBuf, Vec<u8>>) -> Result<()> {
        for entry in fs::read_dir(here)? {
            let path = entry?.path();
            let metadata = fs::symlink_metadata(&path)?;
            let relative = path.strip_prefix(root)?;
            if reparse(&metadata) {
                return Err("initialization output alias refused".into());
            }
            if metadata.is_dir()
                && (expected
                    .keys()
                    .any(|p| p.starts_with(relative) && p != relative)
                    || relative == Path::new(WORKSPACE).join("work/world-assets"))
            {
                visit(root, &path, expected)?;
            } else if metadata.is_file() {
                let bytes = expected
                    .get(relative)
                    .ok_or("unknown initialization file; preserving evidence")?;
                if metadata.len() > bytes.len() as u64 || !bytes.starts_with(&fs::read(&path)?) {
                    return Err(
                        "initialization output changed or a world already dispatched".into(),
                    );
                }
            } else {
                return Err("unknown initialization directory/type; preserving evidence".into());
            }
        }
        Ok(())
    }
    visit(run, run, &expected)
}

#[cfg(test)]
mod tests;
