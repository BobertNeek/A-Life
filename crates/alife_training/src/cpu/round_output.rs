//! One self-contained central-learner receipt, published without replacement.
//! The coordinator exclusively owns this directory. Stages retain recoverable
//! local results; they never authorize a second optimizer update or promotion.
use super::{
    collector_output::{
        fingerprint, owned_root, publish_new, read_json, sync_directory_ancestry, write_pending,
    },
    cooperative_round::{CpuCooperativeRoundPlanV1, CpuCooperativeRoundResultV1},
    CpuCollectorArtifact,
};
use std::{
    collections::BTreeMap,
    error::Error,
    fs, io,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

pub const CPU_COOPERATIVE_ROUND_RESULT_FILE: &str = "cooperative-round.json";
const MAX_ROUND_BYTES: u64 = 256 * 1024 * 1024;
const MAX_ROUND_FILES: usize = 256;
type Result<T> = std::result::Result<T, Box<dyn Error>>;
type Inventory = BTreeMap<String, CpuCollectorArtifact>;

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn stage_prefix(plan_digest: &[u8; 32]) -> String {
    let mut prefix = String::with_capacity(71);
    prefix.push_str("round-");
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in plan_digest {
        prefix.push(HEX[usize::from(byte >> 4)] as char);
        prefix.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    prefix.push('-');
    prefix
}

fn valid_stage_suffix(suffix: &str) -> bool {
    !suffix.is_empty()
        && suffix
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

pub(super) fn round_stage_name(plan: &CpuCooperativeRoundPlanV1, suffix: &str) -> Result<String> {
    if !valid_stage_suffix(suffix) {
        return Err(invalid("invalid cooperative round stage suffix").into());
    }
    Ok(format!(
        "{}{suffix}.pending",
        stage_prefix(&plan.payload_sha256)
    ))
}

/// This directory contains only this plan's flat, regular receipt files. A
/// pending file from another plan is foreign even when its JSON is incomplete.
fn inventory(root: &Path, plan_digest: &[u8; 32]) -> Result<Inventory> {
    let prefix = stage_prefix(plan_digest);
    let mut files = BTreeMap::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| invalid("cooperative round file name must be UTF-8"))?;
        let is_stage = name
            .strip_prefix(&prefix)
            .and_then(|name| name.strip_suffix(".pending"))
            .is_some_and(valid_stage_suffix);
        if (name != CPU_COOPERATIVE_ROUND_RESULT_FILE && !is_stage)
            || !kind.is_file()
            || kind.is_symlink()
            || !path.canonicalize()?.starts_with(root)
            || files.len() >= MAX_ROUND_FILES
        {
            return Err(invalid("foreign or nonregular file in cooperative round output").into());
        }
        if fs::symlink_metadata(&path)?.len() > MAX_ROUND_BYTES {
            return Err(invalid("cooperative round receipt exceeds its byte limit").into());
        }
        let artifact = fingerprint(&path, false)?;
        if artifact.encoded_bytes > MAX_ROUND_BYTES {
            return Err(invalid("cooperative round receipt exceeds its byte limit").into());
        }
        files.insert(name, artifact);
    }
    Ok(files)
}

fn read_stage(path: &Path) -> Result<Option<CpuCooperativeRoundResultV1>> {
    match read_json::<CpuCooperativeRoundResultV1>(path, MAX_ROUND_BYTES) {
        Ok(result) => {
            result.validate()?;
            Ok(Some(result))
        }
        Err(error)
            if error
                .downcast_ref::<serde_json::Error>()
                .is_some_and(serde_json::Error::is_eof) =>
        {
            // Only an incomplete JSON stream is recoverably torn. A complete
            // invalid receipt must not silently trigger another local update.
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

struct Candidate {
    name: String,
    result: CpuCooperativeRoundResultV1,
}

fn inspect(
    root: &Path,
    plan: &CpuCooperativeRoundPlanV1,
) -> Result<(Inventory, Option<Candidate>)> {
    let before = inventory(root, &plan.payload_sha256)?;
    let mut candidate: Option<Candidate> = None;
    for name in before.keys() {
        let result = if name == CPU_COOPERATIVE_ROUND_RESULT_FILE {
            let result: CpuCooperativeRoundResultV1 = read_json(&root.join(name), MAX_ROUND_BYTES)?;
            result.validate()?;
            Some(result)
        } else {
            read_stage(&root.join(name))?
        };
        let Some(result) = result else { continue };
        if &result.plan != plan {
            return Err(invalid("cooperative round receipt binds a different plan").into());
        }
        if let Some(previous) = &candidate {
            if previous.result.payload_sha256 != result.payload_sha256 {
                return Err(invalid("ambiguous cooperative round recovery results").into());
            }
        } else {
            // BTreeMap places the canonical name before any stage, so a valid
            // canonical receipt always remains the authoritative candidate.
            candidate = Some(Candidate {
                name: name.clone(),
                result,
            });
        }
    }
    if inventory(root, &plan.payload_sha256)? != before {
        return Err(invalid("cooperative round output changed during recovery").into());
    }
    // Returning no result permits the coordinator to learn locally. Reserve
    // space for its new durable stage before authorizing that work; an existing
    // complete candidate needs no additional slot and remains recoverable.
    if candidate.is_none() && before.len() >= MAX_ROUND_FILES {
        return Err(invalid("cooperative round output has no free result stage slot").into());
    }
    Ok((before, candidate))
}

fn finish(
    root: &Path,
    mut files: Inventory,
    candidate: Candidate,
) -> Result<CpuCooperativeRoundResultV1> {
    let plan_digest = candidate.result.plan.payload_sha256;
    let path = root.join(&candidate.name);
    // Prior write/sync failure does not prove durable staging. Re-sync even a
    // canonical receipt before acknowledging recovery, and persist new parents.
    let synced = fingerprint(&path, true)?;
    if files.get(&candidate.name) != Some(&synced) {
        return Err(invalid("cooperative round candidate changed before publication").into());
    }
    sync_directory_ancestry(root)?;
    if inventory(root, &plan_digest)? != files {
        return Err(invalid("cooperative round inventory changed before publication").into());
    }
    if candidate.name != CPU_COOPERATIVE_ROUND_RESULT_FILE {
        publish_new(&path, &root.join(CPU_COOPERATIVE_ROUND_RESULT_FILE))?;
        files.remove(&candidate.name);
        files.insert(CPU_COOPERATIVE_ROUND_RESULT_FILE.into(), synced);
        if inventory(root, &plan_digest)? != files {
            return Err(invalid("cooperative round inventory changed after publication").into());
        }
    }
    Ok(candidate.result)
}

/// Reuse a complete result for exactly this plan without running learning.
/// EOF-torn own stages are preserved. Foreign files and contradictory complete
/// receipts fail before publication; a prior ambiguous I/O outcome is reconciled
/// only after re-synchronizing the chosen valid receipt and directory ancestry.
pub(super) fn recover_round_result(
    root: &Path,
    plan: &CpuCooperativeRoundPlanV1,
) -> Result<Option<CpuCooperativeRoundResultV1>> {
    plan.validate()?;
    let root = owned_root(root)?;
    let (files, candidate) = inspect(&root, plan)?;
    candidate
        .map(|candidate| finish(&root, files, candidate))
        .transpose()
}

/// Persist a self-contained local result. Failed writes leave their unique
/// stages intact; another attempt must inspect and reconcile them first.
pub(super) fn publish_round_result(
    root: &Path,
    result: &CpuCooperativeRoundResultV1,
) -> Result<CpuCooperativeRoundResultV1> {
    result.validate()?;
    let root = owned_root(root)?;
    let (mut files, candidate) = inspect(&root, &result.plan)?;
    if let Some(candidate) = candidate {
        if candidate.result.payload_sha256 != result.payload_sha256 {
            return Err(
                invalid("cooperative round output already contains a different result").into(),
            );
        }
        return finish(&root, files, candidate);
    }
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let suffix = format!(
        "{}-{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let name = round_stage_name(&result.plan, &suffix)?;
    let path = root.join(&name);
    write_pending(&path, result, MAX_ROUND_BYTES)?;
    let staged = read_stage(&path)?
        .ok_or_else(|| invalid("cooperative round result write was incomplete"))?;
    if staged.plan != result.plan || staged.payload_sha256 != result.payload_sha256 {
        return Err(invalid("cooperative round staged result changed").into());
    }
    files.insert(name.clone(), fingerprint(&path, false)?);
    finish(
        &root,
        files,
        Candidate {
            name,
            result: staged,
        },
    )
}

/// Read the committed, self-contained receipt. Pending stages alone never
/// authorize a result; unrelated files cannot be adopted as round output.
pub fn read_cpu_cooperative_round(root: &Path) -> Result<CpuCooperativeRoundResultV1> {
    let root = owned_root(root)?;
    let canonical = root.join(CPU_COOPERATIVE_ROUND_RESULT_FILE);
    if fs::symlink_metadata(&canonical)?.len() > MAX_ROUND_BYTES {
        return Err(invalid("cooperative round receipt exceeds its byte limit").into());
    }
    let original = fingerprint(&canonical, false)?;
    let result: CpuCooperativeRoundResultV1 = read_json(&canonical, MAX_ROUND_BYTES)?;
    result.validate()?;
    let before = inventory(&root, &result.plan.payload_sha256)?;
    if before.get(CPU_COOPERATIVE_ROUND_RESULT_FILE) != Some(&original)
        || inventory(&root, &result.plan.payload_sha256)? != before
    {
        return Err(invalid("cooperative round output changed while reading").into());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::{
        CpuCollectionSnapshotV1, CpuCollectorAssignmentV1, CpuCooperativeRoundMemberV1,
    };
    use crate::{FoundationTrainingBackendV1, PpoConfig};
    use std::path::PathBuf;

    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "alife-round-output-{}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
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

    #[test]
    fn torn_own_stages_are_preserved_but_complete_invalid_receipts_fail() {
        let root = Directory::new();
        let name = format!("{}test.pending", stage_prefix(&[7; 32]));
        let stage = root.0.join(name);
        fs::write(&stage, b"{\"").unwrap();
        let before = inventory(&root.0, &[7; 32]).unwrap();
        assert!(read_stage(&stage).unwrap().is_none());
        assert_eq!(inventory(&root.0, &[7; 32]).unwrap(), before);
        assert_eq!(fs::read(&stage).unwrap(), b"{\"");
        fs::write(&stage, b"{}").unwrap();
        assert!(read_stage(&stage).is_err());
        assert_eq!(fs::read(&stage).unwrap(), b"{}");
    }

    #[test]
    fn recovery_reserves_a_stage_slot_before_permitting_new_learning() {
        // Plan validation needs no world, checkpoints or optimizer updates.
        let plan = CpuCooperativeRoundPlanV1::seal(
            "synthetic-stage-capacity".into(),
            [1; 32],
            CpuCollectionSnapshotV1 {
                policy_version: 1,
                biological_objective_version: 2,
                critic_objective_version: 1,
                actor_sha256: [2; 32],
                critic_sha256: [3; 32],
                compiler_sha256: [4; 32],
                ppo_config_sha256: [5; 32],
            },
            PpoConfig::default(),
            1,
            0,
            1,
            vec![CpuCooperativeRoundMemberV1 {
                assignment: CpuCollectorAssignmentV1 {
                    collector_run_id: "synthetic-stage-capacity".into(),
                    backend: FoundationTrainingBackendV1::NativeCpu,
                    producer_revision: "synthetic-v1".into(),
                    world_seed: 1,
                    sampling_seed: 1,
                    sampling_counter_start: 0,
                    sampling_counter_end: 1,
                    lesson: "synthetic-stage-capacity".into(),
                    episode_config_sha256: [6; 32],
                },
                collector_commit_sha256: [7; 32],
            }],
        )
        .unwrap();
        let root = Directory::new();
        for index in 0..MAX_ROUND_FILES - 1 {
            let name = round_stage_name(&plan, &format!("torn-{index}")).unwrap();
            fs::write(root.0.join(name), b"{").unwrap();
        }
        let before = inventory(&root.0, &plan.payload_sha256).unwrap();
        assert!(recover_round_result(&root.0, &plan).unwrap().is_none());
        assert_eq!(inventory(&root.0, &plan.payload_sha256).unwrap(), before);

        let final_name = round_stage_name(&plan, "last-torn").unwrap();
        fs::write(root.0.join(final_name), b"{").unwrap();
        let full = inventory(&root.0, &plan.payload_sha256).unwrap();
        assert_eq!(full.len(), MAX_ROUND_FILES);
        assert!(inspect(&root.0, &plan).is_err());
        assert!(recover_round_result(&root.0, &plan).is_err());
        assert_eq!(inventory(&root.0, &plan.payload_sha256).unwrap(), full);
        for name in full.keys() {
            assert_eq!(fs::read(root.0.join(name)).unwrap(), b"{");
        }
        assert!(!root.0.join(CPU_COOPERATIVE_ROUND_RESULT_FILE).exists());
    }

    #[test]
    fn other_plan_or_foreign_torn_stages_cannot_be_adopted() {
        let root = Directory::new();
        for name in [
            format!("{}test.pending", stage_prefix(&[8; 32])),
            "training-handoff.json".into(),
            format!("{}bad.suffix.pending", stage_prefix(&[7; 32])),
        ] {
            let path = root.0.join(name);
            fs::write(&path, b"{").unwrap();
            assert!(inventory(&root.0, &[7; 32]).is_err());
            assert_eq!(fs::read(&path).unwrap(), b"{");
            fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn directories_cannot_hide_foreign_or_escaped_artifacts() {
        let root = Directory::new();
        let path = root
            .0
            .join(format!("{}test.pending", stage_prefix(&[7; 32])));
        fs::create_dir(&path).unwrap();
        fs::write(path.join("private-checkpoint.json"), b"preserve").unwrap();
        assert!(inventory(&root.0, &[7; 32]).is_err());
        assert_eq!(
            fs::read(path.join("private-checkpoint.json")).unwrap(),
            b"preserve"
        );
    }

    #[cfg(unix)]
    #[test]
    fn stage_symlinks_cannot_publish_or_modify_external_bytes() {
        let root = Directory::new();
        let outside = Directory::new();
        let target = outside.0.join("private-checkpoint.json");
        fs::write(&target, b"preserve").unwrap();
        std::os::unix::fs::symlink(
            &target,
            root.0
                .join(format!("{}test.pending", stage_prefix(&[7; 32]))),
        )
        .unwrap();
        assert!(inventory(&root.0, &[7; 32]).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"preserve");
    }
}
