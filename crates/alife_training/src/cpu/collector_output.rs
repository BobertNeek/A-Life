//! Durable collect-only output. This commit is never a trained continuation or
//! an accepted-source receipt. Callers exclusively own the fresh output tree;
//! concurrent writers and shared mutable prior caches are not supported.
use super::CpuCollectorShardV1;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    error::Error,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
};

pub const CPU_COLLECTOR_SHARD_FILE: &str = "collector-shard.json";
pub const CPU_COLLECTOR_COMMIT_FILE: &str = "collector-commit.json";
const SHARD_PENDING: &str = "collector-shard.pending";
const COMMIT_PENDING: &str = "collector-commit.pending";
const MAX_MANIFEST_BYTES: u64 = 16 * 1024 * 1024;
const MAX_SHARD_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_ARTIFACTS: usize = 65_536;
const REQUIRED: [&str; 5] = [
    "collection-policy.alife-foundation",
    "collection-compiler-inputs.json",
    "collection-actor-checkpoint.json",
    "initial-value-checkpoint.json",
    "collector-checkpoint.json",
];
const TRAINED_SOURCE_MARKERS: [&str; 5] = [
    "cpu-cycle.json",
    "cycle.json",
    "group50.json",
    "trained.alife-foundation",
    "training-handoff.json",
];
type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuCollectorArtifact {
    pub encoded_bytes: u64,
    pub sha256: [u8; 32],
}

/// Pins the native shard and every recovery/source/episode artifact present at
/// collection completion. Only the root's mutable phase marker is excluded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuCollectorOutputCommit {
    pub schema_version: u16,
    pub mode: String,
    pub shard_payload_sha256: [u8; 32],
    pub artifacts: BTreeMap<String, CpuCollectorArtifact>,
    pub payload_sha256: [u8; 32],
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

impl CpuCollectorOutputCommit {
    fn digest(&self) -> Result<[u8; 32]> {
        let mut digest = Sha256::new();
        digest.update(b"ALIFE-CPU-COLLECTOR-OUTPUT-V1");
        serde_json::to_writer(
            &mut digest,
            &(
                self.schema_version,
                &self.mode,
                self.shard_payload_sha256,
                &self.artifacts,
            ),
        )?;
        Ok(digest.finalize().into())
    }

    fn validate(&self) -> Result<()> {
        if self.schema_version != 1
            || self.mode != "native_cpu_collect_only"
            || self.payload_sha256 != self.digest()?
        {
            return Err(invalid("invalid collect-only commit seal").into());
        }
        validate_inventory(&self.artifacts)?;
        if !self.artifacts.contains_key(CPU_COLLECTOR_SHARD_FILE) {
            return Err(invalid("collect-only commit lacks its shard").into());
        }
        Ok(())
    }
}

pub(super) fn owned_root(root: &Path) -> Result<PathBuf> {
    let metadata = fs::symlink_metadata(root)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(invalid("collector output root must be a real directory").into());
    }
    Ok(root.canonicalize()?)
}

fn relative_key(root: &Path, path: &Path) -> Result<String> {
    let relative = path.strip_prefix(root)?;
    let mut parts = Vec::new();
    for component in relative.components() {
        let Component::Normal(part) = component else {
            return Err(invalid("collector artifact path escapes its root").into());
        };
        let part = part
            .to_str()
            .filter(|part| !part.contains(['\\', ':']))
            .ok_or_else(|| invalid("collector artifact path is not portable UTF-8"))?;
        parts.push(part);
    }
    if parts.is_empty() {
        return Err(invalid("collector artifact path is empty").into());
    }
    Ok(parts.join("/"))
}

fn excluded(key: &str) -> bool {
    matches!(
        key,
        "phase.txt" | CPU_COLLECTOR_COMMIT_FILE | SHARD_PENDING | COMMIT_PENDING
    )
}

pub(super) fn fingerprint(path: &Path, sync: bool) -> Result<CpuCollectorArtifact> {
    let before = fs::symlink_metadata(path)?;
    if !before.is_file() || before.file_type().is_symlink() {
        return Err(invalid("collector artifacts must be regular files").into());
    }
    #[cfg(windows)]
    let mut file = OpenOptions::new().read(true).write(sync).open(path)?;
    #[cfg(not(windows))]
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let bytes = io::copy(&mut file, &mut digest)?;
    let after = fs::symlink_metadata(path)?;
    if !after.is_file()
        || after.file_type().is_symlink()
        || bytes != before.len()
        || bytes != after.len()
        || bytes != file.metadata()?.len()
    {
        return Err(invalid("collector artifact changed while reading").into());
    }
    if sync {
        file.sync_all()?;
    }
    Ok(CpuCollectorArtifact {
        encoded_bytes: bytes,
        sha256: digest.finalize().into(),
    })
}

fn inventory(root: &Path, sync: bool) -> Result<BTreeMap<String, CpuCollectorArtifact>> {
    fn visit(
        root: &Path,
        directory: &Path,
        sync: bool,
        artifacts: &mut BTreeMap<String, CpuCollectorArtifact>,
        depth: usize,
    ) -> Result<()> {
        if depth > 128 {
            return Err(invalid("collector artifact directory depth exceeds limit").into());
        }
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                return Err(invalid("collector output cannot contain symlinks").into());
            }
            if kind.is_dir() {
                if !path.canonicalize()?.starts_with(root) {
                    return Err(invalid("collector directory escapes its owned root").into());
                }
                visit(root, &path, sync, artifacts, depth + 1)?;
            } else if kind.is_file() {
                let key = relative_key(root, &path)?;
                if !excluded(&key) {
                    if artifacts.len() >= MAX_ARTIFACTS || !path.canonicalize()?.starts_with(root) {
                        return Err(invalid("collector inventory exceeds its root or limit").into());
                    }
                    artifacts.insert(key, fingerprint(&path, sync)?);
                }
            } else {
                return Err(invalid("collector output contains a nonregular artifact").into());
            }
        }
        if sync {
            sync_directory(directory)?;
        }
        Ok(())
    }
    // Use one canonical spelling for traversal, relative keys and containment.
    // Windows canonical paths can carry a verbatim prefix absent from callers.
    let root = owned_root(root)?;
    let mut artifacts = BTreeMap::new();
    visit(&root, &root, sync, &mut artifacts, 0)?;
    Ok(artifacts)
}

fn validate_inventory(artifacts: &BTreeMap<String, CpuCollectorArtifact>) -> Result<()> {
    if artifacts.len() > MAX_ARTIFACTS
        || REQUIRED.iter().any(|name| {
            artifacts
                .get(*name)
                .is_none_or(|file| file.encoded_bytes == 0)
        })
    {
        return Err(invalid("collect-only output lacks required recovery artifacts").into());
    }
    for key in artifacts.keys() {
        // Require one canonical relative spelling on both Windows and Unix.
        if key.is_empty()
            || excluded(key)
            || key.contains(['\\', ':'])
            || key.split('/').any(|part| matches!(part, "" | "." | ".."))
            || !Path::new(key)
                .components()
                .all(|part| matches!(part, Component::Normal(_)))
        {
            return Err(invalid("invalid path in collector manifest").into());
        }
    }
    Ok(())
}

pub(super) fn write_pending(path: &Path, value: &impl Serialize, max_bytes: u64) -> Result<()> {
    let file = OpenOptions::new().write(true).create_new(true).open(path)?;
    let mut file = io::BufWriter::new(file);
    serde_json::to_writer(&mut file, value)?;
    file.flush()?;
    if file.get_ref().metadata()?.len() > max_bytes {
        return Err(invalid("collector JSON exceeds its byte limit").into());
    }
    file.get_ref().sync_all()?;
    Ok(())
}

/// Exclusively publish a collect-only output after synchronizing its native
/// recovery files. Failed publication preserves all original artifacts and any
/// incomplete pending files. A retry must reconcile that output explicitly.
pub fn write_cpu_collector_output(
    root: &Path,
    shard: &CpuCollectorShardV1,
) -> Result<CpuCollectorOutputCommit> {
    write_with_publish(root, shard, publish_new)
}

fn write_with_publish(
    root: &Path,
    shard: &CpuCollectorShardV1,
    mut publish: impl FnMut(&Path, &Path) -> io::Result<()>,
) -> Result<CpuCollectorOutputCommit> {
    shard.validate_seal()?;
    let root = owned_root(root)?;
    for name in TRAINED_SOURCE_MARKERS {
        match fs::symlink_metadata(root.join(name)) {
            Ok(_) => {
                return Err(
                    invalid("collector output cannot attach to a trained source root").into(),
                )
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => (),
            Err(error) => return Err(error.into()),
        }
    }
    for name in [
        CPU_COLLECTOR_SHARD_FILE,
        CPU_COLLECTOR_COMMIT_FILE,
        SHARD_PENDING,
        COMMIT_PENDING,
    ] {
        match fs::symlink_metadata(root.join(name)) {
            Ok(_) => return Err(io::Error::from(io::ErrorKind::AlreadyExists).into()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => (),
            Err(error) => return Err(error.into()),
        }
    }
    let before = inventory(&root, true)?;
    validate_inventory(&before)?;
    sync_directory_ancestry(&root)?;
    write_pending(&root.join(SHARD_PENDING), shard, MAX_SHARD_BYTES)?;
    publish(
        &root.join(SHARD_PENDING),
        &root.join(CPU_COLLECTOR_SHARD_FILE),
    )?;
    let mut artifacts = inventory(&root, true)?;
    let shard_artifact = artifacts
        .remove(CPU_COLLECTOR_SHARD_FILE)
        .ok_or_else(|| invalid("published collector shard is missing"))?;
    if artifacts != before {
        return Err(invalid("recovery artifacts changed while sealing collection").into());
    }
    artifacts.insert(CPU_COLLECTOR_SHARD_FILE.into(), shard_artifact);
    let mut commit = CpuCollectorOutputCommit {
        schema_version: 1,
        mode: "native_cpu_collect_only".into(),
        shard_payload_sha256: shard.payload_sha256,
        artifacts,
        payload_sha256: [0; 32],
    };
    commit.payload_sha256 = commit.digest()?;
    commit.validate()?;
    write_pending(&root.join(COMMIT_PENDING), &commit, MAX_MANIFEST_BYTES)?;
    publish(
        &root.join(COMMIT_PENDING),
        &root.join(CPU_COLLECTOR_COMMIT_FILE),
    )?;
    Ok(commit)
}

pub(super) fn read_json<T: serde::de::DeserializeOwned>(path: &Path, max_bytes: u64) -> Result<T> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > max_bytes {
        return Err(invalid("invalid collector JSON file or byte length").into());
    }
    let file = File::open(path)?;
    Ok(serde_json::from_reader(file.take(max_bytes + 1))?)
}

/// Read only the canonical committed output. Pending files alone never authorize
/// collection evidence, and every recovery artifact must remain byte-identical.
/// If publication returned an I/O error after creating the canonical commit,
/// this reader permits explicit reconciliation; never blindly overwrite it.
/// Normal learner admission still validates the frozen policy and trajectories.
pub fn read_cpu_collector_output(root: &Path) -> Result<CpuCollectorShardV1> {
    let root = owned_root(root)?;
    let commit: CpuCollectorOutputCommit =
        read_json(&root.join(CPU_COLLECTOR_COMMIT_FILE), MAX_MANIFEST_BYTES)?;
    commit.validate()?;
    if inventory(&root, false)? != commit.artifacts {
        return Err(invalid("collector artifact inventory or bytes changed").into());
    }
    let shard: CpuCollectorShardV1 =
        read_json(&root.join(CPU_COLLECTOR_SHARD_FILE), MAX_SHARD_BYTES)?;
    shard.validate_seal()?;
    if shard.payload_sha256 != commit.shard_payload_sha256 {
        return Err(invalid("collector commit binds a different shard").into());
    }
    Ok(shard)
}

#[cfg(unix)]
fn sync_directory(directory: &Path) -> io::Result<()> {
    File::open(directory)?.sync_all()
}
#[cfg(not(unix))]
fn sync_directory(_: &Path) -> io::Result<()> {
    Ok(())
}
pub(super) fn sync_directory_ancestry(root: &Path) -> io::Result<()> {
    for directory in root.ancestors() {
        sync_directory(directory)?;
    }
    Ok(())
}

#[cfg(unix)]
pub(super) fn publish_new(pending: &Path, sealed: &Path) -> io::Result<()> {
    fs::hard_link(pending, sealed)?;
    fs::remove_file(pending)?;
    sync_directory(
        sealed
            .parent()
            .ok_or_else(|| invalid("collector parent missing"))?,
    )
}

#[cfg(windows)]
pub(super) fn publish_new(pending: &Path, sealed: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(existing: *const u16, new: *const u16, flags: u32) -> i32;
    }
    let wide = |path: &Path| {
        path.as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>()
    };
    let pending = wide(pending);
    let sealed = wide(sealed);
    // SAFETY: both paths own live NUL-terminated UTF-16 buffers. Exactly as in
    // cohort replay publication, WRITE_THROUGH persists the rename; omitting
    // REPLACE_EXISTING guarantees that another output cannot be overwritten.
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;
    if unsafe { MoveFileExW(pending.as_ptr(), sealed.as_ptr(), MOVEFILE_WRITE_THROUGH) } == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
#[cfg(not(any(unix, windows)))]
pub(super) fn publish_new(_: &Path, _: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "durable collector publication unsupported",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::{CpuCollectionSnapshotV1, CpuCollectorAssignmentV1, CpuCollectorTrajectoryV1};
    use crate::FoundationTrainingBackendV1;
    use alife_core::{
        BrainCapacityClass, CreatureGenome, FoundationGeneticIdentity, OrganismId, Tick,
        WorldEntityId,
    };
    use std::sync::atomic::{AtomicU64, Ordering};

    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let directory = std::env::temp_dir().join(format!(
                "alife-collector-output-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&directory).unwrap();
            Self(directory)
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn artifacts(root: &Path) {
        for name in REQUIRED {
            fs::write(root.join(name), format!("synthetic recovery: {name}")).unwrap();
        }
        fs::create_dir(root.join("source-native")).unwrap();
        fs::write(
            root.join("source-native/source.json"),
            b"{\"synthetic\":true}",
        )
        .unwrap();
        fs::write(
            root.join("source-native/phase.txt"),
            b"complete-native-source",
        )
        .unwrap();
        fs::write(root.join("phase.txt"), b"collecting").unwrap();
    }
    fn shard() -> CpuCollectorShardV1 {
        // A small transport fixture, deliberately not learner-admissible. Native
        // collection/admission integration tests supply actual n2048 decisions.
        let genome = CreatureGenome::early_mammal_founder(
            0xC011EC7,
            FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
        )
        .unwrap();
        let record = alife_world::WorldOrganismRecord::newborn(
            OrganismId(1),
            WorldEntityId(101),
            genome.clone(),
            genome.express().unwrap(),
            Tick(0),
        )
        .unwrap();
        CpuCollectorShardV1::seal(
            CpuCollectionSnapshotV1 {
                policy_version: 1,
                biological_objective_version: 2,
                critic_objective_version: 1,
                actor_sha256: [1; 32],
                critic_sha256: [2; 32],
                compiler_sha256: [3; 32],
                ppo_config_sha256: [4; 32],
            },
            CpuCollectorAssignmentV1 {
                collector_run_id: "synthetic-storage-fixture".into(),
                backend: FoundationTrainingBackendV1::NativeCpu,
                producer_revision: "synthetic-v1".into(),
                world_seed: 1,
                sampling_seed: 1,
                sampling_counter_start: 0,
                sampling_counter_end: 1,
                lesson: "transport-only".into(),
                episode_config_sha256: [5; 32],
            },
            vec![CpuCollectorTrajectoryV1 {
                final_record: record,
                decisions: Vec::new(),
            }],
        )
        .unwrap()
    }

    #[test]
    fn commit_roundtrip_pins_all_private_and_source_files_but_not_progress() {
        let root = Directory::new();
        artifacts(&root.0);
        let original = shard();
        let commit = write_cpu_collector_output(&root.0, &original).unwrap();
        assert!(commit.artifacts.contains_key("source-native/phase.txt"));
        assert!(!commit.artifacts.contains_key("phase.txt"));
        fs::write(root.0.join("phase.txt"), b"collected-only-cpu").unwrap();
        let restored = read_cpu_collector_output(&root.0).unwrap();
        assert_eq!(
            serde_json::to_vec(&original).unwrap(),
            serde_json::to_vec(&restored).unwrap()
        );
        let private = root.0.join("collector-checkpoint.json");
        let saved = fs::read(&private).unwrap();
        fs::write(&private, b"modified private checkpoint").unwrap();
        assert!(read_cpu_collector_output(&root.0).is_err());
        fs::write(&private, saved).unwrap();
        assert!(read_cpu_collector_output(&root.0).is_ok());
        fs::write(root.0.join("uncommitted.json"), b"{}").unwrap();
        assert!(read_cpu_collector_output(&root.0).is_err());
    }

    #[test]
    fn failed_final_publication_and_torn_commits_preserve_recovery_artifacts() {
        let root = Directory::new();
        artifacts(&root.0);
        let before = inventory(&root.0, false).unwrap();
        let mut publications = 0;
        assert!(write_with_publish(&root.0, &shard(), |pending, sealed| {
            publications += 1;
            if publications == 2 {
                Err(io::Error::other(
                    "injected final commit publication failure",
                ))
            } else {
                publish_new(pending, sealed)
            }
        })
        .is_err());
        assert!(root.0.join(CPU_COLLECTOR_SHARD_FILE).is_file());
        assert!(!root.0.join(CPU_COLLECTOR_COMMIT_FILE).exists());
        assert!(read_cpu_collector_output(&root.0).is_err());
        for (path, digest) in before {
            assert_eq!(fingerprint(&root.0.join(path), false).unwrap(), digest);
        }
        assert!(write_cpu_collector_output(&root.0, &shard()).is_err());
        fs::write(root.0.join(CPU_COLLECTOR_COMMIT_FILE), b"{").unwrap();
        assert!(read_cpu_collector_output(&root.0).is_err());
    }

    #[test]
    fn publication_never_overwrites_foreign_or_committed_outputs() {
        let root = Directory::new();
        artifacts(&root.0);
        fs::write(root.0.join(CPU_COLLECTOR_SHARD_FILE), b"foreign shard").unwrap();
        let before = inventory(&root.0, false).unwrap();
        assert!(write_cpu_collector_output(&root.0, &shard()).is_err());
        assert_eq!(inventory(&root.0, false).unwrap(), before);
        let committed = Directory::new();
        artifacts(&committed.0);
        write_cpu_collector_output(&committed.0, &shard()).unwrap();
        let before = fs::read(committed.0.join(CPU_COLLECTOR_COMMIT_FILE)).unwrap();
        assert!(write_cpu_collector_output(&committed.0, &shard()).is_err());
        assert_eq!(
            fs::read(committed.0.join(CPU_COLLECTOR_COMMIT_FILE)).unwrap(),
            before
        );
        assert!(read_cpu_collector_output(&committed.0).is_ok());
    }

    #[test]
    fn sealing_does_not_adopt_a_changed_recovery_file() {
        let root = Directory::new();
        artifacts(&root.0);
        assert!(write_with_publish(&root.0, &shard(), |pending, sealed| {
            publish_new(pending, sealed)?;
            fs::write(
                root.0.join("collector-checkpoint.json"),
                b"changed during seal",
            )
        })
        .is_err());
        assert!(!root.0.join(CPU_COLLECTOR_COMMIT_FILE).exists());
        assert!(read_cpu_collector_output(&root.0).is_err());
    }

    #[test]
    fn ambiguous_publication_is_reconciled_from_its_complete_canonical_commit() {
        let root = Directory::new();
        artifacts(&root.0);
        let mut publications = 0;
        let intended = shard();
        assert!(write_with_publish(&root.0, &intended, |pending, sealed| {
            publications += 1;
            publish_new(pending, sealed)?;
            if publications == 2 {
                Err(io::Error::other(
                    "injected error after canonical publication",
                ))
            } else {
                Ok(())
            }
        })
        .is_err());
        let reconciled = read_cpu_collector_output(&root.0).unwrap();
        assert_eq!(
            serde_json::to_vec(&reconciled).unwrap(),
            serde_json::to_vec(&intended).unwrap()
        );
        assert!(write_cpu_collector_output(&root.0, &shard()).is_err());
    }

    #[test]
    fn missing_recovery_file_and_manifest_path_escape_are_rejected() {
        let root = Directory::new();
        artifacts(&root.0);
        fs::remove_file(root.0.join("collector-checkpoint.json")).unwrap();
        assert!(write_cpu_collector_output(&root.0, &shard()).is_err());
        assert!(!root.0.join(CPU_COLLECTOR_SHARD_FILE).exists());
        fs::write(
            root.0.join("collector-checkpoint.json"),
            b"private synthetic",
        )
        .unwrap();
        let mut commit = write_cpu_collector_output(&root.0, &shard()).unwrap();
        commit.artifacts.insert(
            "../outside.json".into(),
            CpuCollectorArtifact {
                encoded_bytes: 1,
                sha256: [1; 32],
            },
        );
        commit.payload_sha256 = commit.digest().unwrap();
        fs::write(
            root.0.join(CPU_COLLECTOR_COMMIT_FILE),
            serde_json::to_vec(&commit).unwrap(),
        )
        .unwrap();
        assert!(read_cpu_collector_output(&root.0).is_err());
    }

    #[test]
    fn direct_writer_preserves_roots_with_native_training_or_source_markers() {
        for marker in TRAINED_SOURCE_MARKERS {
            let root = Directory::new();
            artifacts(&root.0);
            fs::write(root.0.join(marker), b"synthetic existing source marker").unwrap();
            let before = inventory(&root.0, false).unwrap();
            assert!(
                write_cpu_collector_output(&root.0, &shard()).is_err(),
                "{marker}"
            );
            assert_eq!(inventory(&root.0, false).unwrap(), before);
            for owned in [
                CPU_COLLECTOR_SHARD_FILE,
                CPU_COLLECTOR_COMMIT_FILE,
                SHARD_PENDING,
                COMMIT_PENDING,
            ] {
                assert!(!root.0.join(owned).exists(), "{marker}: {owned}");
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_rejected_without_changing_their_target() {
        let root = Directory::new();
        let outside = Directory::new();
        artifacts(&root.0);
        fs::write(outside.0.join("private.json"), b"unrelated synthetic data").unwrap();
        std::os::unix::fs::symlink(&outside.0, root.0.join("escape")).unwrap();
        assert!(write_cpu_collector_output(&root.0, &shard()).is_err());
        assert_eq!(
            fs::read(outside.0.join("private.json")).unwrap(),
            b"unrelated synthetic data"
        );
        assert!(!root.0.join(CPU_COLLECTOR_SHARD_FILE).exists());
    }

    #[cfg(windows)]
    #[test]
    fn junctions_cannot_escape_the_collector_root() {
        let root = Directory::new();
        let outside = Directory::new();
        artifacts(&root.0);
        fs::write(outside.0.join("private.json"), b"unrelated synthetic data").unwrap();
        let result = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(root.0.join("escape"))
            .arg(&outside.0)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(write_cpu_collector_output(&root.0, &shard()).is_err());
        assert_eq!(
            fs::read(outside.0.join("private.json")).unwrap(),
            b"unrelated synthetic data"
        );
        assert!(!root.0.join(CPU_COLLECTOR_SHARD_FILE).exists());
        fs::remove_dir(root.0.join("escape")).unwrap();
    }
}
