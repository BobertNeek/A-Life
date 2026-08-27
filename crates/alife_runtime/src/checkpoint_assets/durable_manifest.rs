//! Atomic portable-save publication for durable GPU checkpoint transactions.

use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};

use alife_core::{
    CanonicalDigestBuilder, ConsolidationState, OrganismId, ScaffoldContractError, SleepState,
    SleepTrigger, Tick, Validate,
};
use alife_world::persistence::{PortableAssetDigest, PortableSaveFile};
use serde::{Deserialize, Serialize};

use crate::GameAppShellError;

static SAVE_CAS_GUARD: Mutex<()> = Mutex::new(());
static SAVE_CAS_NONCE: AtomicU64 = AtomicU64::new(1);
pub const GPU_SLEEP_TRANSACTION_JOURNAL_SCHEMA_VERSION: u16 = 1;
const SLEEP_JOURNAL_DIGEST_DOMAIN: &[u8] = b"alife.gpu.sleep-transaction-journal.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SleepJournalAnchorDisposition {
    Current,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuSleepTransactionJournalEntryV1 {
    pub organism_id: OrganismId,
    pub transition_tick: Tick,
    pub source: SleepState,
    pub target: SleepState,
    pub rollback_to_exact_base: bool,
    pub entry_digest: [u64; 4],
}

impl GpuSleepTransactionJournalEntryV1 {
    pub fn try_new(
        organism_id: OrganismId,
        transition_tick: Tick,
        source: SleepState,
        target: SleepState,
    ) -> Result<Self, ScaffoldContractError> {
        let mut entry = Self {
            organism_id,
            transition_tick,
            source,
            target,
            rollback_to_exact_base: true,
            entry_digest: [0; 4],
        };
        entry.entry_digest = entry.recompute_digest()?;
        entry.validate()?;
        Ok(entry)
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        self.source.validate_contract()?;
        self.target.validate_contract()?;
        let permitted = matches!(
            (self.source.consolidation, self.target.consolidation),
            (ConsolidationState::Pending { .. }, ConsolidationState::Prepared { .. })
                | (ConsolidationState::Prepared { .. }, ConsolidationState::Submitted { .. })
                | (ConsolidationState::Committed { .. }, ConsolidationState::Committed { .. })
                | (ConsolidationState::Committed { .. }, ConsolidationState::None)
        );
        if !permitted
            || !self.rollback_to_exact_base
            || self.transition_tick.raw() == 0
            || self.entry_digest != self.recompute_digest()?
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        match (self.source.consolidation, self.target.consolidation) {
            (
                ConsolidationState::Pending {
                    replay_digest,
                    replay_event_count,
                    replay_eligibility_sample_count,
                    ..
                },
                ConsolidationState::Prepared { request },
            ) if request.cycle_id == self.source.active_cycle_id
                && request.replay_digest == replay_digest
                && request.max_replay_events >= replay_event_count
                && request.max_replay_eligibility_samples >= replay_eligibility_sample_count => {}
            (
                ConsolidationState::Prepared { request: source },
                ConsolidationState::Submitted { request: target, .. },
            ) if source == target => {}
            (ConsolidationState::Committed { .. }, ConsolidationState::Committed { .. })
                if self.source.consolidation == self.target.consolidation => {}
            (ConsolidationState::Committed { .. }, ConsolidationState::None)
                if self.target.phase == alife_core::SleepPhase::Awake => {}
            _ => return Err(ScaffoldContractError::ConsolidationGenerationMismatch),
        }
        Ok(())
    }

    fn recompute_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        let mut digest = CanonicalDigestBuilder::new(SLEEP_JOURNAL_DIGEST_DOMAIN);
        digest.write_u16(GPU_SLEEP_TRANSACTION_JOURNAL_SCHEMA_VERSION);
        digest.write_u64(self.organism_id.raw());
        digest.write_u64(self.transition_tick.raw());
        write_sleep_identity(&mut digest, self.source)?;
        write_sleep_identity(&mut digest, self.target)?;
        digest.write_bool(self.rollback_to_exact_base);
        Ok(digest.finish256())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuSleepTransactionJournalV1 {
    pub schema_version: u16,
    pub exact_base_manifest_digest: String,
    pub exact_base_checkpoint_tick: Tick,
    pub entries: Vec<GpuSleepTransactionJournalEntryV1>,
    pub journal_digest: [u64; 4],
}

impl GpuSleepTransactionJournalV1 {
    pub fn empty(base: &GpuLoadedSaveManifest) -> Result<Self, ScaffoldContractError> {
        Self::try_new(base.digest.as_str().to_string(), base.save.world.tick, Vec::new())
    }

    pub fn try_new(
        exact_base_manifest_digest: String,
        exact_base_checkpoint_tick: Tick,
        entries: Vec<GpuSleepTransactionJournalEntryV1>,
    ) -> Result<Self, ScaffoldContractError> {
        let mut journal = Self {
            schema_version: GPU_SLEEP_TRANSACTION_JOURNAL_SCHEMA_VERSION,
            exact_base_manifest_digest,
            exact_base_checkpoint_tick,
            entries,
            journal_digest: [0; 4],
        };
        journal.journal_digest = journal.recompute_digest()?;
        journal.validate()?;
        Ok(journal)
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        let mut last_by_organism = BTreeMap::new();
        for entry in &self.entries {
            if let Some(previous) = last_by_organism.insert(entry.organism_id.raw(), entry.target) {
                if previous != entry.source {
                    return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
                }
            }
        }
        if self.schema_version != GPU_SLEEP_TRANSACTION_JOURNAL_SCHEMA_VERSION
            || PortableAssetDigest(self.exact_base_manifest_digest.clone())
                .validate_format()
                .is_err()
            || self.entries.len() > 256
            || self.entries.iter().any(|entry| entry.validate().is_err())
            || self.entries.windows(2).any(|pair| {
                (pair[0].transition_tick.raw(), pair[0].organism_id.raw())
                    >= (pair[1].transition_tick.raw(), pair[1].organism_id.raw())
            })
            || self.journal_digest != self.recompute_digest()?
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        Ok(())
    }

    fn recompute_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        let mut digest = CanonicalDigestBuilder::new(SLEEP_JOURNAL_DIGEST_DOMAIN);
        digest.write_u16(self.schema_version);
        digest.write_utf8(&self.exact_base_manifest_digest);
        digest.write_u64(self.exact_base_checkpoint_tick.raw());
        digest.write_sequence_len(self.entries.len());
        for entry in &self.entries {
            for word in entry.entry_digest {
                digest.write_u64(word);
            }
        }
        Ok(digest.finish256())
    }
}

fn validate_journal_against_exact_base(
    journal: &GpuSleepTransactionJournalV1,
    exact_base_manifest_digest: &str,
    exact_base_checkpoint_tick: Tick,
    exact_base_sleep_states: &BTreeMap<u64, SleepState>,
) -> Result<SleepJournalAnchorDisposition, ScaffoldContractError> {
    journal.validate()?;
    if journal.exact_base_manifest_digest != exact_base_manifest_digest
        || journal.exact_base_checkpoint_tick != exact_base_checkpoint_tick
    {
        return Ok(SleepJournalAnchorDisposition::Superseded);
    }
    let mut seen = BTreeMap::new();
    for entry in &journal.entries {
        if entry.transition_tick.raw() <= exact_base_checkpoint_tick.raw() {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        if seen.insert(entry.organism_id.raw(), ()).is_none()
            && exact_base_sleep_states.get(&entry.organism_id.raw()) != Some(&entry.source)
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
    }
    Ok(SleepJournalAnchorDisposition::Current)
}

fn exact_base_sleep_states(
    base: &GpuLoadedSaveManifest,
) -> Result<BTreeMap<u64, SleepState>, ScaffoldContractError> {
    let mut sleep_states = BTreeMap::new();
    for creature in &base.save.creatures {
        if let Some(gpu_brain) = &creature.gpu_brain {
            if sleep_states
                .insert(creature.organism_id.raw(), gpu_brain.sleep)
                .is_some()
            {
                return Err(ScaffoldContractError::BrainOwnershipMismatch);
            }
        }
    }
    Ok(sleep_states)
}

fn write_sleep_identity(
    digest: &mut CanonicalDigestBuilder,
    state: SleepState,
) -> Result<(), ScaffoldContractError> {
    state.validate_contract()?;
    digest.write_u16(state.schema_version);
    digest.write_u16(state.phase.raw());
    digest.write_u64(state.phase_started_tick.raw());
    digest.write_bool(state.entered_sleep_tick.is_some());
    if let Some(tick) = state.entered_sleep_tick {
        digest.write_u64(tick.raw());
    }
    digest.write_u32(state.cycles_completed);
    digest.write_u8(sleep_trigger_journal_raw(state.last_trigger));
    digest.write_u64(state.active_cycle_id);
    digest.write_u64(state.last_consolidated_cycle_id);
    digest.write_u16(state.consolidation.kind_raw());
    match state.consolidation {
        ConsolidationState::None => {}
        ConsolidationState::Pending {
            intent,
            replay_digest,
            replay_event_count,
            replay_eligibility_sample_count,
        } => {
            digest.write_u64(intent.cycle_id);
            for word in replay_digest { digest.write_u64(word); }
            digest.write_u32(replay_event_count);
            digest.write_u32(replay_eligibility_sample_count);
        }
        ConsolidationState::Prepared { request }
        | ConsolidationState::Submitted { request, .. }
        | ConsolidationState::Completed { request, .. } => {
            for word in request.request_digest { digest.write_u64(word); }
            if let ConsolidationState::Submitted { job_id, .. } = state.consolidation {
                digest.write_u64(job_id.raw());
            }
            if let ConsolidationState::Completed { staged, .. } = state.consolidation {
                for word in staged.staging_digest { digest.write_u64(word); }
            }
        }
        ConsolidationState::Committed { cycle_id, output_generation, output_digest } => {
            digest.write_u64(cycle_id);
            digest.write_u64(output_generation);
            for word in output_digest { digest.write_u64(word); }
        }
    }
    Ok(())
}

const fn sleep_trigger_journal_raw(trigger: Option<SleepTrigger>) -> u8 {
    match trigger {
        None => 0,
        Some(SleepTrigger::FatigueThreshold) => 1,
        Some(SleepTrigger::ForcedRequest) => 2,
        Some(SleepTrigger::RecoveryProtocol) => 3,
        Some(SleepTrigger::SeizureHyperactivity) => 4,
        Some(SleepTrigger::CatatoniaEnergyHypoplasia) => 5,
        Some(SleepTrigger::ExtremeFatigue) => 6,
        Some(SleepTrigger::UnsafeActiveState) => 7,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuSaveManifestDigest(PortableAssetDigest);

impl GpuSaveManifestDigest {
    fn for_bytes(bytes: &[u8]) -> Self {
        Self(PortableAssetDigest::for_bytes(bytes))
    }

    pub fn as_str(&self) -> &str {
        &self.0 .0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuLoadedSaveManifest {
    pub save: PortableSaveFile,
    pub digest: GpuSaveManifestDigest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuSaveManifestCasOutcome {
    Replaced {
        replacement_digest: GpuSaveManifestDigest,
    },
    AlreadyApplied {
        replacement_digest: GpuSaveManifestDigest,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuDurableSaveManifest {
    save_path: PathBuf,
    asset_root: PathBuf,
}

impl GpuDurableSaveManifest {
    fn sleep_journal_path(&self) -> Result<PathBuf, GameAppShellError> {
        let file_name = self
            .save_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
                message: "GPU checkpoint save manifest requires a UTF-8 file name".to_string(),
            })?;
        Ok(self.save_path.with_file_name(format!(
            ".{file_name}.sleep-journal-v1.json"
        )))
    }

    pub fn open(
        save_path: impl Into<PathBuf>,
        asset_root: impl AsRef<Path>,
    ) -> Result<Self, GameAppShellError> {
        let save_path = save_path.into();
        let asset_root = fs::canonicalize(asset_root)?;
        let canonical_save = fs::canonicalize(&save_path)?;
        let durable = Self {
            save_path: canonical_save,
            asset_root,
        };
        durable.load()?;
        Ok(durable)
    }

    pub fn save_path(&self) -> &Path {
        &self.save_path
    }

    pub fn asset_root(&self) -> &Path {
        &self.asset_root
    }

    pub fn load(&self) -> Result<GpuLoadedSaveManifest, GameAppShellError> {
        let bytes = fs::read(&self.save_path)?;
        let text = std::str::from_utf8(&bytes).map_err(|_| {
            GameAppShellError::InvalidProductionFrontend {
                message: "GPU checkpoint save manifest must be valid UTF-8 JSON".to_string(),
            }
        })?;
        let save = PortableSaveFile::from_json_str(text)?;
        save.validate_with_asset_root(&self.asset_root)?;
        Ok(GpuLoadedSaveManifest {
            save,
            digest: GpuSaveManifestDigest::for_bytes(&bytes),
        })
    }

    /// Loads a bounded rollback journal anchored to the exact save manifest.
    /// A journal superseded by a newer exact checkpoint is discarded in
    /// memory; it is never overlaid onto that newer world state.
    pub fn load_sleep_transaction_journal(
        &self,
        base: &GpuLoadedSaveManifest,
    ) -> Result<GpuSleepTransactionJournalV1, GameAppShellError> {
        let path = self.sleep_journal_path()?;
        if !path.exists() {
            return Ok(GpuSleepTransactionJournalV1::empty(base)?);
        }
        let journal: GpuSleepTransactionJournalV1 = serde_json::from_slice(&fs::read(path)?)?;
        journal.validate()?;
        let base_sleep_states = exact_base_sleep_states(base)?;
        match validate_journal_against_exact_base(
            &journal,
            base.digest.as_str(),
            base.save.world.tick,
            &base_sleep_states,
        )? {
            SleepJournalAnchorDisposition::Current => Ok(journal),
            SleepJournalAnchorDisposition::Superseded => {
                Ok(GpuSleepTransactionJournalV1::empty(base)?)
            }
        }
    }

    pub fn publish_sleep_transaction_journal(
        &self,
        base: &GpuLoadedSaveManifest,
        journal: &GpuSleepTransactionJournalV1,
    ) -> Result<(), GameAppShellError> {
        journal.validate()?;
        let base_sleep_states = exact_base_sleep_states(base)?;
        if validate_journal_against_exact_base(
            journal,
            base.digest.as_str(),
            base.save.world.tick,
            &base_sleep_states,
        )? != SleepJournalAnchorDisposition::Current
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
        }
        let bytes = serde_json::to_vec_pretty(journal)?;
        let _guard = SAVE_CAS_GUARD.lock().map_err(|_| {
            GameAppShellError::InvalidProductionFrontend {
                message: "GPU checkpoint save CAS lock was poisoned".to_string(),
            }
        })?;
        let actual = GpuSaveManifestDigest::for_bytes(&fs::read(&self.save_path)?);
        if actual != base.digest {
            return Err(GameAppShellError::GpuCheckpointManifestConflict {
                expected: base.digest.as_str().to_string(),
                actual: actual.as_str().to_string(),
            });
        }
        write_atomic_manifest(&self.sleep_journal_path()?, &bytes)
    }

    pub fn replacement_digest(
        &self,
        replacement: &PortableSaveFile,
    ) -> Result<GpuSaveManifestDigest, GameAppShellError> {
        replacement.validate_with_asset_root(&self.asset_root)?;
        Ok(GpuSaveManifestDigest::for_bytes(
            &serde_json::to_vec_pretty(replacement)?,
        ))
    }

    /// Atomically publishes a complete manual/autosave checkpoint, including
    /// first creation of the target manifest. The save may live in a selected
    /// save directory while neural assets remain validated against the
    /// separate asset root.
    pub fn publish_snapshot(
        save_path: impl AsRef<Path>,
        asset_root: impl AsRef<Path>,
        replacement: &PortableSaveFile,
    ) -> Result<GpuLoadedSaveManifest, GameAppShellError> {
        let asset_root = fs::canonicalize(asset_root)?;
        let requested = save_path.as_ref();
        let requested = if requested.is_absolute() {
            requested.to_path_buf()
        } else {
            std::env::current_dir()?.join(requested)
        };
        let parent =
            requested
                .parent()
                .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
                    message: "GPU checkpoint save manifest has no parent directory".to_string(),
                })?;
        fs::create_dir_all(parent)?;
        let parent = fs::canonicalize(parent)?;
        let file_name =
            requested
                .file_name()
                .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
                    message: "GPU checkpoint save manifest requires a file name".to_string(),
                })?;
        let save_path = parent.join(file_name);
        replacement.validate_with_asset_root(&asset_root)?;
        // This is a durable file manifest, not the bounded in-memory save-slot
        // payload governed by P34_MAX_INLINE_SAVE_BYTES. Bulk neural arrays are
        // already external content-addressed assets.
        let replacement_bytes = serde_json::to_vec_pretty(replacement)?;
        let replacement_digest = GpuSaveManifestDigest::for_bytes(&replacement_bytes);
        let replacement_base = GpuLoadedSaveManifest {
            save: replacement.clone(),
            digest: replacement_digest,
        };
        let journal = GpuSleepTransactionJournalV1::empty(&replacement_base)?;
        let journal_path = parent.join(format!(
            ".{}.sleep-journal-v1.json",
            file_name.to_string_lossy()
        ));
        let prepared_journal = prepare_atomic_manifest(
            &journal_path,
            &serde_json::to_vec_pretty(&journal)?,
        )?;
        let _guard =
            SAVE_CAS_GUARD
                .lock()
                .map_err(|_| GameAppShellError::InvalidProductionFrontend {
                    message: "GPU checkpoint save CAS lock was poisoned".to_string(),
                })?;
        if let Err(error) = write_atomic_manifest(&save_path, &replacement_bytes) {
            let _ = fs::remove_file(&prepared_journal);
            return Err(error);
        }
        if commit_prepared_manifest(&prepared_journal, &journal_path).is_err() {
            let _ = fs::remove_file(&prepared_journal);
        }
        drop(_guard);

        let durable = Self {
            save_path: fs::canonicalize(&save_path)?,
            asset_root,
        };
        let published = durable.load()?;
        if published.save != *replacement {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "atomic GPU checkpoint publication changed the replacement save"
                    .to_string(),
            });
        }
        Ok(published)
    }

    pub fn compare_and_swap(
        &self,
        expected: &GpuSaveManifestDigest,
        replacement: &PortableSaveFile,
    ) -> Result<GpuSaveManifestCasOutcome, GameAppShellError> {
        replacement.validate_with_asset_root(&self.asset_root)?;
        let replacement_bytes = serde_json::to_vec_pretty(replacement)?;
        let replacement_digest = GpuSaveManifestDigest::for_bytes(&replacement_bytes);
        let _guard =
            SAVE_CAS_GUARD
                .lock()
                .map_err(|_| GameAppShellError::InvalidProductionFrontend {
                    message: "GPU checkpoint save CAS lock was poisoned".to_string(),
                })?;

        let current_bytes = fs::read(&self.save_path)?;
        let current_digest = GpuSaveManifestDigest::for_bytes(&current_bytes);
        if current_digest == replacement_digest {
            return Ok(GpuSaveManifestCasOutcome::AlreadyApplied { replacement_digest });
        }
        if &current_digest != expected {
            return Err(GameAppShellError::GpuCheckpointManifestConflict {
                expected: expected.as_str().to_string(),
                actual: current_digest.as_str().to_string(),
            });
        }

        let pre_replace_digest = GpuSaveManifestDigest::for_bytes(&fs::read(&self.save_path)?);
        if &pre_replace_digest != expected {
            return Err(GameAppShellError::GpuCheckpointManifestConflict {
                expected: expected.as_str().to_string(),
                actual: pre_replace_digest.as_str().to_string(),
            });
        }
        let replacement_base = GpuLoadedSaveManifest {
            save: replacement.clone(),
            digest: replacement_digest.clone(),
        };
        let reset_journal = GpuSleepTransactionJournalV1::empty(&replacement_base)?;
        let journal_path = self.sleep_journal_path()?;
        let prepared_journal = prepare_atomic_manifest(
            &journal_path,
            &serde_json::to_vec_pretty(&reset_journal)?,
        )?;
        if let Err(error) = write_atomic_manifest(&self.save_path, &replacement_bytes) {
            let _ = fs::remove_file(&prepared_journal);
            return Err(error);
        }
        if commit_prepared_manifest(&prepared_journal, &journal_path).is_err() {
            let _ = fs::remove_file(&prepared_journal);
        }

        let published = fs::read(&self.save_path)?;
        let published_digest = GpuSaveManifestDigest::for_bytes(&published);
        if published_digest != replacement_digest {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "atomic GPU checkpoint publication digest mismatch".to_string(),
            });
        }
        Ok(GpuSaveManifestCasOutcome::Replaced { replacement_digest })
    }
}

fn write_atomic_manifest(save_path: &Path, bytes: &[u8]) -> Result<(), GameAppShellError> {
    let temporary = prepare_atomic_manifest(save_path, bytes)?;
    if let Err(error) = commit_prepared_manifest(&temporary, save_path) {
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    if fs::read(save_path)? != bytes {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: "atomic GPU checkpoint publication digest mismatch".to_string(),
        });
    }
    Ok(())
}

fn prepare_atomic_manifest(save_path: &Path, bytes: &[u8]) -> Result<PathBuf, GameAppShellError> {
    let parent =
        save_path
            .parent()
            .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
                message: "GPU checkpoint save manifest has no parent directory".to_string(),
            })?;
    let nonce = SAVE_CAS_NONCE.fetch_add(1, Ordering::Relaxed);
    let file_name = save_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
            message: "GPU checkpoint save manifest requires a UTF-8 file name".to_string(),
        })?;
    let temporary = parent.join(format!(
        ".{file_name}.gpu-cas-{}-{nonce}.tmp",
        std::process::id()
    ));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)?;
    if let Err(error) = (|| -> std::io::Result<()> {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        Ok(())
    })() {
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    Ok(temporary)
}

fn commit_prepared_manifest(temporary: &Path, save_path: &Path) -> std::io::Result<()> {
    let parent = save_path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "manifest has no parent")
    })?;
    atomic_replace(temporary, save_path)?;
    sync_parent_directory(parent)
}

#[cfg(windows)]
fn atomic_replace(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    // SAFETY: both pointers reference live, NUL-terminated UTF-16 buffers for
    // the duration of the call, and both paths are on the same directory tree.
    let moved = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn atomic_replace(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(unix)]
fn sync_parent_directory(parent: &Path) -> std::io::Result<()> {
    std::fs::File::open(parent)?.sync_all()
}

#[cfg(not(unix))]
fn sync_parent_directory(_parent: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alife_core::{ConsolidationState, SleepPhase, SLEEP_CONSOLIDATION_SCHEMA_VERSION};

    fn committed_sleep(phase: SleepPhase, tick: u64) -> SleepState {
        SleepState {
            schema_version: SLEEP_CONSOLIDATION_SCHEMA_VERSION,
            phase,
            phase_started_tick: Tick::new(tick),
            entered_sleep_tick: Some(Tick::new(1)),
            cycles_completed: 0,
            last_trigger: None,
            active_cycle_id: 1,
            last_consolidated_cycle_id: 0,
            consolidation: ConsolidationState::Committed {
                cycle_id: 1,
                output_generation: 2,
                output_digest: [7; 4],
            },
        }
    }

    #[test]
    fn journal_rejects_malformed_base_digest() {
        assert!(GpuSleepTransactionJournalV1::try_new(
            "not-a-portable-digest".to_string(),
            Tick::new(1),
            Vec::new(),
        )
        .is_err());
    }

    #[test]
    fn journal_rejects_interleaved_broken_organism_chain() {
        let a_source = committed_sleep(SleepPhase::Consolidating, 1);
        let a_target = committed_sleep(SleepPhase::Waking, 1);
        let b_source = committed_sleep(SleepPhase::Consolidating, 1);
        let b_target = committed_sleep(SleepPhase::Waking, 1);
        let entries = vec![
            GpuSleepTransactionJournalEntryV1::try_new(
                OrganismId(1),
                Tick::new(2),
                a_source,
                a_target,
            )
            .unwrap(),
            GpuSleepTransactionJournalEntryV1::try_new(
                OrganismId(2),
                Tick::new(3),
                b_source,
                b_target,
            )
            .unwrap(),
            GpuSleepTransactionJournalEntryV1::try_new(
                OrganismId(1),
                Tick::new(4),
                a_source,
                a_target,
            )
            .unwrap(),
        ];
        assert!(GpuSleepTransactionJournalV1::try_new(
            "fnv1a64:0123456789abcdef".to_string(),
            Tick::new(1),
            entries,
        )
        .is_err());
    }

    #[test]
    fn journal_vs_base_rejects_first_source_mismatch() {
        let source = committed_sleep(SleepPhase::Consolidating, 2);
        let target = committed_sleep(SleepPhase::Waking, 3);
        let journal = GpuSleepTransactionJournalV1::try_new(
            "fnv1a64:0123456789abcdef".to_string(),
            Tick::new(1),
            vec![GpuSleepTransactionJournalEntryV1::try_new(
                OrganismId(1),
                Tick::new(3),
                source,
                target,
            )
            .unwrap()],
        )
        .unwrap();
        let mut exact_base_sleep_states = BTreeMap::new();
        exact_base_sleep_states.insert(OrganismId(1).raw(), SleepState::awake_at(Tick::new(1)));

        assert!(validate_journal_against_exact_base(
            &journal,
            "fnv1a64:0123456789abcdef",
            Tick::new(1),
            &exact_base_sleep_states,
        )
        .is_err());
    }

    #[test]
    fn journal_vs_base_accepts_matching_first_source() {
        let source = committed_sleep(SleepPhase::Consolidating, 2);
        let target = committed_sleep(SleepPhase::Waking, 3);
        let journal = GpuSleepTransactionJournalV1::try_new(
            "fnv1a64:0123456789abcdef".to_string(),
            Tick::new(1),
            vec![GpuSleepTransactionJournalEntryV1::try_new(
                OrganismId(1),
                Tick::new(3),
                source,
                target,
            )
            .unwrap()],
        )
        .unwrap();
        let mut exact_base_sleep_states = BTreeMap::new();
        exact_base_sleep_states.insert(OrganismId(1).raw(), source);

        assert_eq!(
            validate_journal_against_exact_base(
                &journal,
                "fnv1a64:0123456789abcdef",
                Tick::new(1),
                &exact_base_sleep_states,
            )
            .unwrap(),
            SleepJournalAnchorDisposition::Current
        );
    }

    #[test]
    fn journal_vs_base_discards_superseded_ahead_journal() {
        let source = committed_sleep(SleepPhase::Consolidating, 2);
        let target = committed_sleep(SleepPhase::Waking, 30);
        let journal = GpuSleepTransactionJournalV1::try_new(
            "fnv1a64:0123456789abcdef".to_string(),
            Tick::new(1),
            vec![GpuSleepTransactionJournalEntryV1::try_new(
                OrganismId(1),
                Tick::new(30),
                source,
                target,
            )
            .unwrap()],
        )
        .unwrap();

        assert_eq!(
            validate_journal_against_exact_base(
                &journal,
                "fnv1a64:fedcba9876543210",
                Tick::new(2),
                &BTreeMap::new(),
            )
            .unwrap(),
            SleepJournalAnchorDisposition::Superseded,
        );
    }

    #[test]
    fn journal_sleep_trigger_raw_mapping_is_stable() {
        assert_eq!(sleep_trigger_journal_raw(None), 0);
        assert_eq!(
            sleep_trigger_journal_raw(Some(SleepTrigger::FatigueThreshold)),
            1
        );
        assert_eq!(
            sleep_trigger_journal_raw(Some(SleepTrigger::ForcedRequest)),
            2
        );
        assert_eq!(
            sleep_trigger_journal_raw(Some(SleepTrigger::RecoveryProtocol)),
            3
        );
        assert_eq!(
            sleep_trigger_journal_raw(Some(SleepTrigger::SeizureHyperactivity)),
            4
        );
        assert_eq!(
            sleep_trigger_journal_raw(Some(SleepTrigger::CatatoniaEnergyHypoplasia)),
            5
        );
        assert_eq!(
            sleep_trigger_journal_raw(Some(SleepTrigger::ExtremeFatigue)),
            6
        );
        assert_eq!(
            sleep_trigger_journal_raw(Some(SleepTrigger::UnsafeActiveState)),
            7
        );
    }
}
