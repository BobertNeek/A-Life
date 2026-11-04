//! Lossless file snapshot of an already extracted native CPU parent. The
//! original transport archive stays with its owner; this is not a ZIP hash.
use super::*;
use crate::{cpu::*, FoundationTrainingHandoffV1};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const MAX_BYTES: u64 = 512 * 1024 * 1024;
const MAX_FILES: usize = 8192;
type ArchiveResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeCpuParentArchive {
    #[serde(with = "super::archive_encoding::files")]
    pub files: BTreeMap<String, Vec<u8>>,
    pub expected_tree_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeCpuArchiveBinding {
    pub kind: String,
    pub tree_sha256: String,
    pub files: usize,
    pub bytes: u64,
    pub native_receipt_sha256: String,
    pub handoff_sha256: String,
    pub actor_sha256: String,
    pub value_sha256: String,
    pub policy_version: u64,
    pub actor_adam_step: u32,
    pub value_adam_step: u32,
    pub private_collector_policy_version: u64,
    pub private_collector_sha256: String,
    pub private_cold_restore_exact: bool,
    pub historical_freshness: String,
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn relative(name: &str) -> ArchiveResult<()> {
    if name.is_empty()
        || name.contains(['\\', ':', '\0'])
        || name.split('/').any(|p| matches!(p, "" | "." | ".."))
    {
        return Err(invalid("CPU archive paths must be canonical and relative").into());
    }
    Ok(())
}

impl NativeCpuParentArchive {
    pub fn tree_sha256(&self) -> ArchiveResult<String> {
        if self.files.is_empty() || self.files.len() > MAX_FILES {
            return Err(invalid("CPU archive file bound exceeded").into());
        }
        let mut hash = Sha256::new();
        hash.update(b"alife.native-cpu-parent-file-tree.v1\0");
        let mut total = 0u64;
        for (path, bytes) in &self.files {
            relative(path)?;
            total = total
                .checked_add(bytes.len() as u64)
                .ok_or_else(|| invalid("CPU archive size overflow"))?;
            if total > MAX_BYTES {
                return Err(invalid("CPU archive exceeds 512 MiB bound").into());
            }
            hash.update((path.len() as u64).to_le_bytes());
            hash.update(path.as_bytes());
            hash.update((bytes.len() as u64).to_le_bytes());
            hash.update(bytes);
        }
        Ok(format!("{:x}", hash.finalize()))
    }
}

fn capture(root: &Path) -> ArchiveResult<NativeCpuParentArchive> {
    if fs::symlink_metadata(root)?.file_type().is_symlink() || !root.is_dir() {
        return Err(invalid("CPU archive root must be a real directory").into());
    }
    fn walk(
        root: &Path,
        path: &Path,
        files: &mut BTreeMap<String, Vec<u8>>,
        total: &mut u64,
    ) -> ArchiveResult<()> {
        for entry in fs::read_dir(path)? {
            let path = entry?.path();
            let meta = fs::symlink_metadata(&path)?;
            if meta.file_type().is_symlink() {
                return Err(invalid("CPU archive symlinks refused").into());
            }
            if meta.is_dir() {
                walk(root, &path, files, total)?;
            } else if meta.is_file() {
                let name = path
                    .strip_prefix(root)?
                    .to_str()
                    .ok_or_else(|| invalid("CPU archive path is not UTF8"))?
                    .replace(std::path::MAIN_SEPARATOR, "/");
                relative(&name)?;
                *total = total
                    .checked_add(meta.len())
                    .ok_or_else(|| invalid("CPU archive size overflow"))?;
                if *total > MAX_BYTES || files.len() >= MAX_FILES {
                    return Err(invalid("CPU archive capture bound exceeded").into());
                }
                let bytes = fs::read(&path)?;
                if bytes.len() as u64 != meta.len() {
                    return Err(invalid("CPU archive changed during capture").into());
                }
                files.insert(name, bytes);
            } else {
                return Err(invalid("CPU archive contains a nonregular entry").into());
            }
        }
        Ok(())
    }
    let root = root.canonicalize()?;
    let mut files = BTreeMap::new();
    walk(&root, &root, &mut files, &mut 0)?;
    let mut result = NativeCpuParentArchive {
        files,
        expected_tree_sha256: String::new(),
    };
    result.expected_tree_sha256 = result.tree_sha256()?;
    Ok(result)
}

/// Observe the complete extracted regular-file tree. The owner must retain the
/// independently verified transport archive and record this distinct tree pin.
pub fn inspect_native_cpu_archive(root: &Path) -> ArchiveResult<NativeCpuArchiveBinding> {
    let archive = capture(root)?;
    let (_, binding) = admit_native_cpu_archive(&archive)?;
    if capture(root)?.tree_sha256()? != binding.tree_sha256 {
        return Err(invalid("CPU source changed during inspection").into());
    }
    Ok(binding)
}

pub fn capture_native_cpu_archive(
    root: &Path,
    expected_tree_sha256: &str,
) -> ArchiveResult<NativeCpuParentArchive> {
    let mut archive = capture(root)?;
    archive.expected_tree_sha256 = expected_tree_sha256.into();
    admit_native_cpu_archive(&archive)?;
    if capture(root)?.tree_sha256()? != archive.expected_tree_sha256 {
        return Err(invalid("CPU source changed during capture").into());
    }
    Ok(archive)
}

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> ArchiveResult<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "alife-cpu-parent-admission-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root)?;
        Ok(Self(root))
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub(super) fn admit_native_cpu_archive(
    archive: &NativeCpuParentArchive,
) -> ArchiveResult<(FoundationTrainingHandoffV1, NativeCpuArchiveBinding)> {
    let tree = archive.tree_sha256()?;
    if tree != archive.expected_tree_sha256 {
        return Err(invalid("CPU parent differs from its explicit file-tree SHA256 pin").into());
    }
    let scratch = Scratch::new()?;
    for (name, bytes) in &archive.files {
        let path = scratch.0.join(name);
        fs::create_dir_all(
            path.parent()
                .ok_or_else(|| invalid("CPU archive file lacks parent"))?,
        )?;
        fs::write(path, bytes)?;
    }
    // The supported native reader authenticates receipt/header, output pins,
    // full actor/value Adam, typed compiler/asset identity and existing lineage.
    let source = read_cpu_founder_source(&scratch.0)?;
    let inputs: PhenotypeCompilerInputs = serde_json::from_slice(
        archive
            .files
            .get("collection-compiler-inputs.json")
            .ok_or_else(|| invalid("CPU collection compiler inputs missing"))?,
    )?;
    let p =
        alife_core::PhenotypeCompiler::compile_validated(&inputs, &BrainCapacityClass::n2048())?;
    let asset = FoundationWeightAsset::decode_canonical(
        archive
            .files
            .get("collection-policy.alife-foundation")
            .ok_or_else(|| invalid("CPU collection policy missing"))?,
    )?;
    let actor: crate::FoundationTrainerCheckpoint = serde_json::from_slice(
        archive
            .files
            .get("collection-actor-checkpoint.json")
            .ok_or_else(|| invalid("CPU collection actor missing"))?,
    )?;
    if actor
        .weights
        .iter()
        .map(|w| w.to_bits())
        .collect::<Vec<_>>()
        != asset
            .weights()
            .iter()
            .map(|w| w.to_bits())
            .collect::<Vec<_>>()
        || source.receipt["initial_actor_optimizer_step"] != actor.optimizer_step
    {
        return Err(
            invalid("CPU collection actor differs from retained policy or native age").into(),
        );
    }
    let value: crate::PpoValueHeadCheckpoint = serde_json::from_slice(
        archive
            .files
            .get("initial-value-checkpoint.json")
            .ok_or_else(|| invalid("CPU collection value head missing"))?,
    )?;
    CpuValueHead::from_checkpoint(value.clone())?;
    if source.receipt["initial_value_optimizer_step"] != value.optimizer_step {
        return Err(invalid("CPU collection value age differs from native receipt").into());
    }
    let trainer = CpuFoundationTrainer::from_checkpoint(p, asset, actor.clone())?;
    let private_path = cpu_private_checkpoint_path(&scratch.0)?;
    if source.receipt["private_individual_archive"].as_str()
        != private_path.file_name().and_then(|p| p.to_str())
        || source.receipt["private_archive_policy"] != "collection-policy.alife-foundation"
    {
        return Err(invalid("CPU private collector binding differs from native receipt").into());
    }
    let raw = read_cpu_private_checkpoint_bytes(&scratch.0, MAX_BYTES)?;
    let checkpoint: CpuCanonicalCheckpoint = serde_json::from_slice(&raw)?;
    let restored = CpuCanonicalHost::from_checkpoint(checkpoint.clone(), &trainer)?;
    if serde_json::to_vec(&checkpoint)? != serde_json::to_vec(&restored.checkpoint(&trainer)?)?
        || serde_json::to_vec(&trainer.checkpoint())? != serde_json::to_vec(&actor)?
    {
        return Err(invalid("CPU private cold restore changed retained state").into());
    }
    let private_name = private_path
        .file_name()
        .and_then(|p| p.to_str())
        .ok_or_else(|| invalid("CPU private path invalid"))?;
    let hash = |name: &str| -> ArchiveResult<String> {
        Ok(sha256(archive.files.get(name).ok_or_else(|| {
            invalid("CPU native archive artifact missing")
        })?))
    };
    let binding = NativeCpuArchiveBinding {
        kind: "native-cpu-file-archive-preserved-heads-and-separate-collector-v1".into(),
        tree_sha256: tree,
        files: archive.files.len(),
        bytes: archive.files.values().map(|b| b.len() as u64).sum(),
        native_receipt_sha256: hash("cpu-cycle.json")?,
        handoff_sha256: hash("training-handoff.json")?,
        actor_sha256: hash("actor-checkpoint.json")?,
        value_sha256: hash("value-checkpoint.json")?,
        policy_version: source.handoff.policy_version,
        actor_adam_step: source.handoff.actor.optimizer_step,
        value_adam_step: source.handoff.value.optimizer_step,
        private_collector_policy_version: source.receipt["source_policy_version"]
            .as_u64()
            .ok_or_else(|| invalid("CPU collector policy version missing"))?,
        private_collector_sha256: hash(private_name)?,
        private_cold_restore_exact: true,
        historical_freshness: "unknown".into(),
    };
    if capture(&scratch.0)?.tree_sha256()? != archive.expected_tree_sha256 {
        return Err(invalid("native CPU archive admission modified a parent file").into());
    }
    Ok((source.handoff, binding))
}
