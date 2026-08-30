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
    time::Instant,
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
#[cfg(test)]
static AUTHORITY_TEST_FAILURE_STAGE: AtomicU64 = AtomicU64::new(0);
pub const GPU_SLEEP_TRANSACTION_JOURNAL_SCHEMA_VERSION: u16 = 2;
const SLEEP_JOURNAL_DIGEST_DOMAIN: &[u8] = b"alife.gpu.sleep-transaction-journal.v2";
const GPU_CHECKPOINT_AUTHORITY_FIELD: &str = "gpu_checkpoint_authority";
const GPU_CHECKPOINT_AUTHORITY_SCHEMA: &str = "alife.gpu-checkpoint-authority";
const GPU_CHECKPOINT_AUTHORITY_SCHEMA_VERSION: u16 = 1;
const GPU_CHECKPOINT_AUTHORITY_DIGEST_DOMAIN: &[u8] = b"alife.gpu-checkpoint-authority.v1";
const AUTHORITY_STAGE_SAVE_PREPARED: u64 = 1;
const AUTHORITY_STAGE_JOURNAL_PREPARED: u64 = 2;
const AUTHORITY_STAGE_POINTER_COMMIT: u64 = 3;
const AUTHORITY_STAGE_REOPEN: u64 = 4;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GpuSleepJournalPublicationTiming {
    pub input_validation_wall_ns: u64,
    pub cas_lock_wait_wall_ns: u64,
    pub cas_base_reload_wall_ns: u64,
    pub save_encode_wall_ns: u64,
    pub save_artifact_write_wall_ns: u64,
    pub journal_encode_wall_ns: u64,
    pub journal_artifact_write_wall_ns: u64,
    pub pointer_build_validation_wall_ns: u64,
    pub prepared_artifact_reload_validation_wall_ns: u64,
    pub manifest_encode_wall_ns: u64,
    pub manifest_write_wall_ns: u64,
    pub manifest_reload_validation_wall_ns: u64,
    pub final_journal_reload_validation_wall_ns: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuSleepJournalPublicationReceipt {
    pub published: GpuLoadedSaveManifest,
    pub timing: GpuSleepJournalPublicationTiming,
}

fn record_elapsed_ns(field: &mut u64, started: Option<Instant>) {
    if let Some(started) = started {
        *field =
            field.saturating_add(u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX));
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
enum AuthorityTestFailureStage {
    SavePrepared = AUTHORITY_STAGE_SAVE_PREPARED,
    JournalPrepared = AUTHORITY_STAGE_JOURNAL_PREPARED,
    PointerCommit = AUTHORITY_STAGE_POINTER_COMMIT,
    Reopen = AUTHORITY_STAGE_REOPEN,
}

#[cfg(test)]
fn set_authority_test_failure(stage: AuthorityTestFailureStage) {
    AUTHORITY_TEST_FAILURE_STAGE.store(stage as u64, Ordering::SeqCst);
}

fn maybe_fail_authority_stage(stage: u64) -> Result<(), GameAppShellError> {
    #[cfg(test)]
    if AUTHORITY_TEST_FAILURE_STAGE
        .compare_exchange(stage, 0, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: format!("injected GPU checkpoint authority failure at stage {stage}"),
        });
    }
    let _ = stage;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SleepJournalAnchorDisposition {
    Current,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuSleepTransactionJournalEntryV2 {
    pub organism_id: OrganismId,
    pub transition_tick: Tick,
    pub transition_ordinal: u8,
    pub source: SleepState,
    pub target: SleepState,
    pub rollback_to_exact_base: bool,
    pub entry_digest: [u64; 4],
}

impl GpuSleepTransactionJournalEntryV2 {
    pub fn try_new(
        organism_id: OrganismId,
        transition_tick: Tick,
        source: SleepState,
        target: SleepState,
    ) -> Result<Self, ScaffoldContractError> {
        Self::try_new_with_ordinal(organism_id, transition_tick, 0, source, target)
    }

    pub fn try_new_with_ordinal(
        organism_id: OrganismId,
        transition_tick: Tick,
        transition_ordinal: u8,
        source: SleepState,
        target: SleepState,
    ) -> Result<Self, ScaffoldContractError> {
        let mut entry = Self {
            organism_id,
            transition_tick,
            transition_ordinal,
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
            (ConsolidationState::None, ConsolidationState::None)
                | (ConsolidationState::None, ConsolidationState::Pending { .. })
                | (
                    ConsolidationState::Pending { .. },
                    ConsolidationState::Prepared { .. }
                )
                | (
                    ConsolidationState::Prepared { .. },
                    ConsolidationState::Submitted { .. }
                )
                | (
                    ConsolidationState::Completed { .. },
                    ConsolidationState::Committed { .. }
                )
                | (
                    ConsolidationState::Committed { .. },
                    ConsolidationState::Committed { .. }
                )
                | (
                    ConsolidationState::Committed { .. },
                    ConsolidationState::None
                )
        );
        if !permitted
            || !self.rollback_to_exact_base
            || self.transition_ordinal > 1
            || self.transition_tick.raw() == 0
            || self.source.phase_started_tick.raw() > self.transition_tick.raw()
            || self.target.phase_started_tick.raw() > self.transition_tick.raw()
            || self.entry_digest != self.recompute_digest()?
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        match (self.source.consolidation, self.target.consolidation) {
            (ConsolidationState::None, ConsolidationState::None)
                if canonical_pre_pending_phase_edge(self.source, self.target) => {}
            (ConsolidationState::None, ConsolidationState::Pending { intent, .. })
                if self.source.phase == alife_core::SleepPhase::Consolidating
                    && intent.cycle_id == self.source.active_cycle_id
                    && sleep_identity_unchanged_except_consolidation(self.source, self.target) => {}
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
                ConsolidationState::Submitted {
                    request: target, ..
                },
            ) if source == target => {}
            (
                ConsolidationState::Completed { request, staged },
                ConsolidationState::Committed {
                    cycle_id,
                    output_generation,
                    output_digest,
                },
            ) if cycle_id == request.cycle_id
                && output_generation == staged.output_generation
                && output_digest == staged.output_digest
                && sleep_identity_unchanged_except_consolidation(self.source, self.target) => {}
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
        digest.write_u8(self.transition_ordinal);
        write_sleep_identity(&mut digest, self.source)?;
        write_sleep_identity(&mut digest, self.target)?;
        digest.write_bool(self.rollback_to_exact_base);
        Ok(digest.finish256())
    }
}

fn canonical_pre_pending_phase_edge(source: SleepState, target: SleepState) -> bool {
    use alife_core::SleepPhase::{Awake, Consolidating, EnteringSleep, ForcedRecoverySleep};

    match (source.phase, target.phase) {
        (Awake, EnteringSleep) => {
            source.active_cycle_id == 0
                && source.entered_sleep_tick.is_none()
                && target.entered_sleep_tick == Some(target.phase_started_tick)
                && target.last_trigger == Some(SleepTrigger::FatigueThreshold)
                && target.active_cycle_id == source.last_consolidated_cycle_id.saturating_add(1)
                && sleep_cycle_fields_match(source, target)
        }
        (Awake, ForcedRecoverySleep) => {
            source.active_cycle_id == 0
                && source.entered_sleep_tick.is_none()
                && target.entered_sleep_tick == Some(target.phase_started_tick)
                && matches!(
                    target.last_trigger,
                    Some(
                        SleepTrigger::ForcedRequest
                            | SleepTrigger::RecoveryProtocol
                            | SleepTrigger::SeizureHyperactivity
                            | SleepTrigger::CatatoniaEnergyHypoplasia
                            | SleepTrigger::ExtremeFatigue
                            | SleepTrigger::UnsafeActiveState
                    )
                )
                && target.active_cycle_id == source.last_consolidated_cycle_id.saturating_add(1)
                && sleep_cycle_fields_match(source, target)
        }
        (EnteringSleep | ForcedRecoverySleep, Consolidating) => {
            source.entered_sleep_tick == target.entered_sleep_tick
                && source.last_trigger == target.last_trigger
                && source.active_cycle_id == target.active_cycle_id
                && sleep_cycle_fields_match(source, target)
        }
        _ => false,
    }
}

fn sleep_cycle_fields_match(source: SleepState, target: SleepState) -> bool {
    source.schema_version == target.schema_version
        && source.cycles_completed == target.cycles_completed
        && source.last_consolidated_cycle_id == target.last_consolidated_cycle_id
        && source.consolidation == target.consolidation
}

fn sleep_identity_unchanged_except_consolidation(source: SleepState, target: SleepState) -> bool {
    source.schema_version == target.schema_version
        && source.phase == target.phase
        && source.phase_started_tick == target.phase_started_tick
        && source.entered_sleep_tick == target.entered_sleep_tick
        && source.cycles_completed == target.cycles_completed
        && source.last_trigger == target.last_trigger
        && source.active_cycle_id == target.active_cycle_id
        && source.last_consolidated_cycle_id == target.last_consolidated_cycle_id
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuSleepTransactionJournalV2 {
    pub schema_version: u16,
    pub exact_base_manifest_digest: String,
    pub exact_base_checkpoint_tick: Tick,
    pub entries: Vec<GpuSleepTransactionJournalEntryV2>,
    pub journal_digest: [u64; 4],
}

impl GpuSleepTransactionJournalV2 {
    pub fn empty(base: &GpuLoadedSaveManifest) -> Result<Self, ScaffoldContractError> {
        Self::try_new(
            base.exact_save_anchor_digest()?.0,
            base.save.world.tick,
            Vec::new(),
        )
    }

    pub fn try_new(
        exact_base_manifest_digest: String,
        exact_base_checkpoint_tick: Tick,
        entries: Vec<GpuSleepTransactionJournalEntryV2>,
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
        let mut previous_key = None;
        let mut previous_entry: Option<&GpuSleepTransactionJournalEntryV2> = None;
        for entry in &self.entries {
            let key = (
                entry.organism_id.raw(),
                entry.transition_tick.raw(),
                entry.transition_ordinal,
            );
            if previous_key.is_some_and(|previous| previous >= key) {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
            }
            match previous_key {
                Some((organism, tick, ordinal)) if organism == key.0 && tick == key.1 => {
                    if ordinal.checked_add(1) != Some(key.2) {
                        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
                    }
                    let Some(previous) = previous_entry else {
                        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
                    };
                    if !matches!(
                        (previous.source.consolidation, previous.target.consolidation),
                        (ConsolidationState::None, ConsolidationState::None)
                    ) || !matches!(
                        (entry.source.consolidation, entry.target.consolidation),
                        (ConsolidationState::None, ConsolidationState::Pending { .. })
                    ) {
                        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
                    }
                }
                _ if key.2 != 0 => {
                    return Err(ScaffoldContractError::ConsolidationGenerationMismatch)
                }
                _ => {}
            }
            if let Some(previous) = last_by_organism.insert(entry.organism_id.raw(), entry.target) {
                if previous != entry.source {
                    return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
                }
            }
            previous_key = Some(key);
            previous_entry = Some(entry);
        }
        if self.schema_version != GPU_SLEEP_TRANSACTION_JOURNAL_SCHEMA_VERSION
            || PortableAssetDigest(self.exact_base_manifest_digest.clone())
                .validate_format()
                .is_err()
            || self.entries.len() > 256
            || self.entries.iter().any(|entry| entry.validate().is_err())
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
    journal: &GpuSleepTransactionJournalV2,
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
            for word in replay_digest {
                digest.write_u64(word);
            }
            digest.write_u32(replay_event_count);
            digest.write_u32(replay_eligibility_sample_count);
        }
        ConsolidationState::Prepared { request }
        | ConsolidationState::Submitted { request, .. }
        | ConsolidationState::Completed { request, .. } => {
            for word in request.request_digest {
                digest.write_u64(word);
            }
            if let ConsolidationState::Submitted { job_id, .. } = state.consolidation {
                digest.write_u64(job_id.raw());
            }
            if let ConsolidationState::Completed { staged, .. } = state.consolidation {
                for word in staged.staging_digest {
                    digest.write_u64(word);
                }
            }
        }
        ConsolidationState::Committed {
            cycle_id,
            output_generation,
            output_digest,
        } => {
            digest.write_u64(cycle_id);
            digest.write_u64(output_generation);
            for word in output_digest {
                digest.write_u64(word);
            }
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
pub struct GpuSaveManifestDigest {
    save_content: PortableAssetDigest,
    authority: Option<[u64; 4]>,
}

impl GpuSaveManifestDigest {
    fn for_bytes(bytes: &[u8]) -> Self {
        Self {
            save_content: PortableAssetDigest::for_bytes(bytes),
            authority: None,
        }
    }

    fn for_authority(save_content: PortableAssetDigest, authority: [u64; 4]) -> Self {
        Self {
            save_content,
            authority: Some(authority),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.save_content.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GpuCheckpointAuthoritySaveArtifactV1 {
    file_name: String,
    digest: PortableAssetDigest,
    size_bytes: u64,
    save_schema: String,
    save_schema_version: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GpuCheckpointAuthorityJournalArtifactV1 {
    file_name: String,
    digest: PortableAssetDigest,
    size_bytes: u64,
    journal_schema_version: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GpuCheckpointAuthorityPointerV1 {
    schema: String,
    schema_version: u16,
    generation: u64,
    prior_generation: Option<u64>,
    prior_authority_digest: Option<[u64; 4]>,
    asset_manifest_identity: PortableAssetDigest,
    save: GpuCheckpointAuthoritySaveArtifactV1,
    journal: GpuCheckpointAuthorityJournalArtifactV1,
    authority_digest: [u64; 4],
}

impl GpuCheckpointAuthorityPointerV1 {
    fn try_new(
        generation: u64,
        prior: Option<&Self>,
        asset_manifest_identity: PortableAssetDigest,
        save: GpuCheckpointAuthoritySaveArtifactV1,
        journal: GpuCheckpointAuthorityJournalArtifactV1,
    ) -> Result<Self, GameAppShellError> {
        let mut pointer = Self {
            schema: GPU_CHECKPOINT_AUTHORITY_SCHEMA.to_string(),
            schema_version: GPU_CHECKPOINT_AUTHORITY_SCHEMA_VERSION,
            generation,
            prior_generation: prior.map(|value| value.generation),
            prior_authority_digest: prior.map(|value| value.authority_digest),
            asset_manifest_identity,
            save,
            journal,
            authority_digest: [0; 4],
        };
        pointer.authority_digest = pointer.recompute_digest();
        pointer.validate()?;
        Ok(pointer)
    }

    fn validate(&self) -> Result<(), GameAppShellError> {
        self.asset_manifest_identity.validate_format()?;
        self.save.digest.validate_format()?;
        self.journal.digest.validate_format()?;
        let prior_is_valid = if self.generation == 1 {
            self.prior_generation.is_none() && self.prior_authority_digest.is_none()
        } else {
            self.prior_generation == Some(self.generation - 1)
                && self.prior_authority_digest.is_some()
        };
        if self.schema != GPU_CHECKPOINT_AUTHORITY_SCHEMA
            || self.schema_version != GPU_CHECKPOINT_AUTHORITY_SCHEMA_VERSION
            || self.generation == 0
            || !prior_is_valid
            || self.save.size_bytes == 0
            || self.save.save_schema.is_empty()
            || self.save.save_schema_version == 0
            || !valid_authority_artifact_file_name(&self.save.file_name)
            || self.journal.size_bytes == 0
            || self.journal.journal_schema_version != GPU_SLEEP_TRANSACTION_JOURNAL_SCHEMA_VERSION
            || !valid_authority_artifact_file_name(&self.journal.file_name)
            || self.authority_digest != self.recompute_digest()
        {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "GPU checkpoint authority pointer is malformed or unsupported".to_string(),
            });
        }
        Ok(())
    }

    fn recompute_digest(&self) -> [u64; 4] {
        let mut digest = CanonicalDigestBuilder::new(GPU_CHECKPOINT_AUTHORITY_DIGEST_DOMAIN);
        digest.write_u16(self.schema_version);
        digest.write_u64(self.generation);
        match (self.prior_generation, self.prior_authority_digest) {
            (Some(generation), Some(authority_digest)) => {
                digest.write_some();
                digest.write_u64(generation);
                for word in authority_digest {
                    digest.write_u64(word);
                }
            }
            _ => digest.write_none(),
        }
        digest.write_utf8(&self.asset_manifest_identity.0);
        digest.write_utf8(&self.save.file_name);
        digest.write_utf8(&self.save.digest.0);
        digest.write_u64(self.save.size_bytes);
        digest.write_utf8(&self.save.save_schema);
        digest.write_u16(self.save.save_schema_version);
        digest.write_utf8(&self.journal.file_name);
        digest.write_utf8(&self.journal.digest.0);
        digest.write_u64(self.journal.size_bytes);
        digest.write_u16(self.journal.journal_schema_version);
        digest.finish256()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuLoadedSaveManifest {
    pub save: PortableSaveFile,
    pub digest: GpuSaveManifestDigest,
    authority: GpuCheckpointAuthoritySource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GpuCheckpointAuthoritySource {
    LegacyDirectV1,
    GenerationV1(GpuCheckpointAuthorityPointerV1),
}

impl GpuCheckpointAuthoritySource {
    fn generation(&self) -> Option<&GpuCheckpointAuthorityPointerV1> {
        match self {
            Self::LegacyDirectV1 => None,
            Self::GenerationV1(pointer) => Some(pointer),
        }
    }
}

impl GpuLoadedSaveManifest {
    pub fn exact_save_anchor_digest(&self) -> Result<PortableAssetDigest, ScaffoldContractError> {
        let bytes = serde_json::to_vec_pretty(&self.save)
            .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?;
        Ok(PortableAssetDigest::for_bytes(&bytes))
    }

    pub fn authority_generation(&self) -> Option<u64> {
        match &self.authority {
            GpuCheckpointAuthoritySource::LegacyDirectV1 => None,
            GpuCheckpointAuthoritySource::GenerationV1(pointer) => Some(pointer.generation),
        }
    }
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
        Ok(self
            .save_path
            .with_file_name(format!(".{file_name}.sleep-journal-v2.json")))
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
        let value: serde_json::Value = serde_json::from_slice(&bytes)?;
        let authority = value
            .get(GPU_CHECKPOINT_AUTHORITY_FIELD)
            .cloned()
            .map(serde_json::from_value::<GpuCheckpointAuthorityPointerV1>)
            .transpose()?;
        let (save, digest, authority) = match authority {
            Some(pointer) => {
                pointer.validate()?;
                let save = self.read_authority_save(&pointer)?;
                if pointer.asset_manifest_identity != asset_manifest_identity(&save)? {
                    return Err(GameAppShellError::InvalidProductionFrontend {
                        message: "GPU checkpoint authority asset-manifest identity mismatch"
                            .to_string(),
                    });
                }
                self.read_authority_journal(&pointer, &save)?;
                (
                    save,
                    GpuSaveManifestDigest::for_authority(
                        pointer.save.digest.clone(),
                        pointer.authority_digest,
                    ),
                    GpuCheckpointAuthoritySource::GenerationV1(pointer),
                )
            }
            None => {
                let text = std::str::from_utf8(&bytes).map_err(|_| {
                    GameAppShellError::InvalidProductionFrontend {
                        message: "legacy GPU checkpoint save must be valid UTF-8 JSON".to_string(),
                    }
                })?;
                let save = PortableSaveFile::from_json_str(text)?;
                save.validate_with_asset_root(&self.asset_root)?;
                (
                    save,
                    GpuSaveManifestDigest::for_bytes(&bytes),
                    GpuCheckpointAuthoritySource::LegacyDirectV1,
                )
            }
        };
        Ok(GpuLoadedSaveManifest {
            save,
            digest,
            authority,
        })
    }

    fn read_authority_save(
        &self,
        pointer: &GpuCheckpointAuthorityPointerV1,
    ) -> Result<PortableSaveFile, GameAppShellError> {
        let bytes = fs::read(self.authority_artifact_path(&pointer.save.file_name)?)?;
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) != pointer.save.size_bytes
            || PortableAssetDigest::for_bytes(&bytes) != pointer.save.digest
        {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "GPU checkpoint authority save artifact mismatch".to_string(),
            });
        }
        let text = std::str::from_utf8(&bytes).map_err(|_| {
            GameAppShellError::InvalidProductionFrontend {
                message: "GPU checkpoint authority save artifact must be UTF-8 JSON".to_string(),
            }
        })?;
        let save = PortableSaveFile::from_json_str(text)?;
        if save.schema != pointer.save.save_schema
            || save.schema_version != pointer.save.save_schema_version
        {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "GPU checkpoint authority save schema identity mismatch".to_string(),
            });
        }
        save.validate_with_asset_root(&self.asset_root)?;
        Ok(save)
    }

    /// Loads a bounded rollback journal anchored to the exact save manifest.
    /// A journal superseded by a newer exact checkpoint is discarded in
    /// memory; it is never overlaid onto that newer world state.
    pub fn load_sleep_transaction_journal(
        &self,
        base: &GpuLoadedSaveManifest,
    ) -> Result<GpuSleepTransactionJournalV2, GameAppShellError> {
        let journal = match &base.authority {
            GpuCheckpointAuthoritySource::GenerationV1(pointer) => {
                self.read_authority_journal(pointer, &base.save)?
            }
            GpuCheckpointAuthoritySource::LegacyDirectV1 => {
                let path = self.sleep_journal_path()?;
                if !path.exists() {
                    return Ok(GpuSleepTransactionJournalV2::empty(base)?);
                }
                serde_json::from_slice(&fs::read(path)?)?
            }
        };
        journal.validate()?;
        let base_sleep_states = exact_base_sleep_states(base)?;
        match validate_journal_against_exact_base(
            &journal,
            &base.exact_save_anchor_digest()?.0,
            base.save.world.tick,
            &base_sleep_states,
        )? {
            SleepJournalAnchorDisposition::Current => Ok(journal),
            SleepJournalAnchorDisposition::Superseded => {
                Ok(GpuSleepTransactionJournalV2::empty(base)?)
            }
        }
    }

    fn read_authority_journal(
        &self,
        pointer: &GpuCheckpointAuthorityPointerV1,
        save: &PortableSaveFile,
    ) -> Result<GpuSleepTransactionJournalV2, GameAppShellError> {
        pointer.validate()?;
        let path = self.authority_artifact_path(&pointer.journal.file_name)?;
        let bytes = fs::read(path)?;
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) != pointer.journal.size_bytes
            || PortableAssetDigest::for_bytes(&bytes) != pointer.journal.digest
        {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "GPU checkpoint authority journal artifact mismatch".to_string(),
            });
        }
        let journal: GpuSleepTransactionJournalV2 = serde_json::from_slice(&bytes)?;
        journal.validate()?;
        let base = GpuLoadedSaveManifest {
            save: save.clone(),
            digest: GpuSaveManifestDigest::for_authority(
                pointer.save.digest.clone(),
                pointer.authority_digest,
            ),
            authority: GpuCheckpointAuthoritySource::GenerationV1(pointer.clone()),
        };
        let base_sleep_states = exact_base_sleep_states(&base)?;
        if validate_journal_against_exact_base(
            &journal,
            &base.exact_save_anchor_digest()?.0,
            save.world.tick,
            &base_sleep_states,
        )? != SleepJournalAnchorDisposition::Current
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
        }
        Ok(journal)
    }

    fn authority_artifact_path(&self, file_name: &str) -> Result<PathBuf, GameAppShellError> {
        if !valid_authority_artifact_file_name(file_name) {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "GPU checkpoint authority artifact name is invalid".to_string(),
            });
        }
        let parent = self.save_path.parent().ok_or_else(|| {
            GameAppShellError::InvalidProductionFrontend {
                message: "GPU checkpoint save manifest has no parent directory".to_string(),
            }
        })?;
        Ok(parent.join(file_name))
    }

    pub fn publish_sleep_transaction_journal(
        &self,
        base: &GpuLoadedSaveManifest,
        journal: &GpuSleepTransactionJournalV2,
    ) -> Result<(), GameAppShellError> {
        self.publish_sleep_transaction_journal_profiled(base, journal, false)
            .map(|_| ())
    }

    pub fn publish_sleep_transaction_journal_profiled(
        &self,
        base: &GpuLoadedSaveManifest,
        journal: &GpuSleepTransactionJournalV2,
        measure: bool,
    ) -> Result<GpuSleepJournalPublicationReceipt, GameAppShellError> {
        let mut timing = GpuSleepJournalPublicationTiming::default();
        let started = measure.then(Instant::now);
        journal.validate()?;
        let base_sleep_states = exact_base_sleep_states(base)?;
        if validate_journal_against_exact_base(
            journal,
            &base.exact_save_anchor_digest()?.0,
            base.save.world.tick,
            &base_sleep_states,
        )? != SleepJournalAnchorDisposition::Current
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
        }
        record_elapsed_ns(&mut timing.input_validation_wall_ns, started);

        let started = measure.then(Instant::now);
        let guard =
            SAVE_CAS_GUARD
                .lock()
                .map_err(|_| GameAppShellError::InvalidProductionFrontend {
                    message: "GPU checkpoint save CAS lock was poisoned".to_string(),
                })?;
        record_elapsed_ns(&mut timing.cas_lock_wait_wall_ns, started);

        let started = measure.then(Instant::now);
        let actual = self.load()?;
        record_elapsed_ns(&mut timing.cas_base_reload_wall_ns, started);
        if actual.digest != base.digest || actual.save != base.save {
            return Err(GameAppShellError::GpuCheckpointManifestConflict {
                expected: base.digest.as_str().to_string(),
                actual: actual.digest.as_str().to_string(),
            });
        }
        let pointer = self.prepare_journal_authority_generation_profiled(
            &actual,
            journal,
            measure,
            &mut timing,
        )?;
        let reopened =
            self.commit_authority_pointer_profiled(&actual.save, &pointer, measure, &mut timing)?;

        let started = measure.then(Instant::now);
        let journal_reopened = self.load_sleep_transaction_journal(&reopened)?;
        record_elapsed_ns(&mut timing.final_journal_reload_validation_wall_ns, started);
        if reopened.authority.generation() != Some(&pointer) || journal_reopened != *journal {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "GPU checkpoint journal authority generation failed reopen validation"
                    .to_string(),
            });
        }
        drop(guard);
        Ok(GpuSleepJournalPublicationReceipt {
            published: reopened,
            timing,
        })
    }

    fn prepare_authority_generation(
        &self,
        base: &GpuLoadedSaveManifest,
        journal: &GpuSleepTransactionJournalV2,
    ) -> Result<GpuCheckpointAuthorityPointerV1, GameAppShellError> {
        self.prepare_authority_generation_profiled(
            base,
            journal,
            false,
            &mut GpuSleepJournalPublicationTiming::default(),
        )
    }

    fn prepare_journal_authority_generation_profiled(
        &self,
        base: &GpuLoadedSaveManifest,
        journal: &GpuSleepTransactionJournalV2,
        measure: bool,
        timing: &mut GpuSleepJournalPublicationTiming,
    ) -> Result<GpuCheckpointAuthorityPointerV1, GameAppShellError> {
        let Some(prior) = base.authority.generation() else {
            return self.prepare_authority_generation_profiled(base, journal, measure, timing);
        };
        let generation = prior.generation.checked_add(1).ok_or_else(|| {
            GameAppShellError::InvalidProductionFrontend {
                message: "GPU checkpoint authority generation overflow".to_string(),
            }
        })?;
        let started = measure.then(Instant::now);
        let journal_bytes = serde_json::to_vec_pretty(journal)?;
        let journal_digest = PortableAssetDigest::for_bytes(&journal_bytes);
        let journal_file_name =
            authority_journal_file_name(&self.save_path, generation, &journal_digest)?;
        record_elapsed_ns(&mut timing.journal_encode_wall_ns, started);
        let started = measure.then(Instant::now);
        write_immutable_authority_artifact(
            &self.authority_artifact_path(&journal_file_name)?,
            &journal_bytes,
        )?;
        record_elapsed_ns(&mut timing.journal_artifact_write_wall_ns, started);
        maybe_fail_authority_stage(AUTHORITY_STAGE_JOURNAL_PREPARED)?;
        let started = measure.then(Instant::now);
        let pointer = GpuCheckpointAuthorityPointerV1::try_new(
            generation,
            Some(prior),
            prior.asset_manifest_identity.clone(),
            prior.save.clone(),
            GpuCheckpointAuthorityJournalArtifactV1 {
                file_name: journal_file_name,
                digest: journal_digest,
                size_bytes: u64::try_from(journal_bytes.len()).unwrap_or(u64::MAX),
                journal_schema_version: GPU_SLEEP_TRANSACTION_JOURNAL_SCHEMA_VERSION,
            },
        )?;
        record_elapsed_ns(&mut timing.pointer_build_validation_wall_ns, started);
        Ok(pointer)
    }

    fn prepare_authority_generation_profiled(
        &self,
        base: &GpuLoadedSaveManifest,
        journal: &GpuSleepTransactionJournalV2,
        measure: bool,
        timing: &mut GpuSleepJournalPublicationTiming,
    ) -> Result<GpuCheckpointAuthorityPointerV1, GameAppShellError> {
        let prior = base.authority.generation();
        let generation = match prior {
            Some(pointer) => pointer.generation.checked_add(1).ok_or_else(|| {
                GameAppShellError::InvalidProductionFrontend {
                    message: "GPU checkpoint authority generation overflow".to_string(),
                }
            })?,
            None => 1,
        };
        let started = measure.then(Instant::now);
        let save_bytes = serde_json::to_vec_pretty(&base.save)?;
        let save_digest = PortableAssetDigest::for_bytes(&save_bytes);
        let save_file_name = authority_save_file_name(&self.save_path, generation, &save_digest)?;
        record_elapsed_ns(&mut timing.save_encode_wall_ns, started);
        let started = measure.then(Instant::now);
        write_immutable_authority_artifact(
            &self.authority_artifact_path(&save_file_name)?,
            &save_bytes,
        )?;
        record_elapsed_ns(&mut timing.save_artifact_write_wall_ns, started);
        maybe_fail_authority_stage(AUTHORITY_STAGE_SAVE_PREPARED)?;
        let started = measure.then(Instant::now);
        let journal_bytes = serde_json::to_vec_pretty(journal)?;
        let journal_digest = PortableAssetDigest::for_bytes(&journal_bytes);
        let journal_file_name =
            authority_journal_file_name(&self.save_path, generation, &journal_digest)?;
        record_elapsed_ns(&mut timing.journal_encode_wall_ns, started);
        let started = measure.then(Instant::now);
        write_immutable_authority_artifact(
            &self.authority_artifact_path(&journal_file_name)?,
            &journal_bytes,
        )?;
        record_elapsed_ns(&mut timing.journal_artifact_write_wall_ns, started);
        maybe_fail_authority_stage(AUTHORITY_STAGE_JOURNAL_PREPARED)?;
        let started = measure.then(Instant::now);
        let pointer = GpuCheckpointAuthorityPointerV1::try_new(
            generation,
            prior,
            asset_manifest_identity(&base.save)?,
            GpuCheckpointAuthoritySaveArtifactV1 {
                file_name: save_file_name,
                digest: save_digest,
                size_bytes: u64::try_from(save_bytes.len()).unwrap_or(u64::MAX),
                save_schema: base.save.schema.clone(),
                save_schema_version: base.save.schema_version,
            },
            GpuCheckpointAuthorityJournalArtifactV1 {
                file_name: journal_file_name,
                digest: journal_digest,
                size_bytes: u64::try_from(journal_bytes.len()).unwrap_or(u64::MAX),
                journal_schema_version: GPU_SLEEP_TRANSACTION_JOURNAL_SCHEMA_VERSION,
            },
        )?;
        record_elapsed_ns(&mut timing.pointer_build_validation_wall_ns, started);
        Ok(pointer)
    }

    fn commit_authority_pointer(
        &self,
        compatibility_save: &PortableSaveFile,
        pointer: &GpuCheckpointAuthorityPointerV1,
    ) -> Result<GpuLoadedSaveManifest, GameAppShellError> {
        self.commit_authority_pointer_profiled(
            compatibility_save,
            pointer,
            false,
            &mut GpuSleepJournalPublicationTiming::default(),
        )
    }

    fn commit_authority_pointer_profiled(
        &self,
        compatibility_save: &PortableSaveFile,
        pointer: &GpuCheckpointAuthorityPointerV1,
        measure: bool,
        timing: &mut GpuSleepJournalPublicationTiming,
    ) -> Result<GpuLoadedSaveManifest, GameAppShellError> {
        let started = measure.then(Instant::now);
        pointer.validate()?;
        let prepared_save = self.read_authority_save(pointer)?;
        if prepared_save != *compatibility_save
            || pointer.asset_manifest_identity != asset_manifest_identity(&prepared_save)?
        {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "GPU checkpoint prepared authority save does not match the candidate"
                    .to_string(),
            });
        }
        self.read_authority_journal(pointer, &prepared_save)?;
        record_elapsed_ns(
            &mut timing.prepared_artifact_reload_validation_wall_ns,
            started,
        );

        let prior_public_pointer = match fs::read(&self.save_path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        maybe_fail_authority_stage(AUTHORITY_STAGE_POINTER_COMMIT)?;
        let started = measure.then(Instant::now);
        let encoded = encode_authoritative_save(compatibility_save, pointer)?;
        record_elapsed_ns(&mut timing.manifest_encode_wall_ns, started);
        let started = measure.then(Instant::now);
        write_atomic_manifest(&self.save_path, &encoded)?;
        record_elapsed_ns(&mut timing.manifest_write_wall_ns, started);
        let started = measure.then(Instant::now);
        match maybe_fail_authority_stage(AUTHORITY_STAGE_REOPEN).and_then(|_| self.load()) {
            Ok(reopened) => {
                record_elapsed_ns(&mut timing.manifest_reload_validation_wall_ns, started);
                Ok(reopened)
            }
            Err(reopen_error) => {
                record_elapsed_ns(&mut timing.manifest_reload_validation_wall_ns, started);
                let rollback = self.restore_prior_authority_pointer(
                    prior_public_pointer.as_deref(),
                    pointer.prior_generation,
                );
                let message = match rollback {
                    Ok(()) => format!(
                        "{reopen_error}; attempted authority generation was rolled back to the byte-identical prior pointer"
                    ),
                    Err(rollback_error) => format!(
                        "{reopen_error}; authority rollback could not be proven: {rollback_error}"
                    ),
                };
                Err(
                    GameAppShellError::GpuCheckpointAuthorityPostCommitValidation {
                        committed_generation: pointer.generation,
                        last_known_good_generation: pointer.prior_generation.unwrap_or(0),
                        message,
                    },
                )
            }
        }
    }

    fn restore_prior_authority_pointer(
        &self,
        prior_public_pointer: Option<&[u8]>,
        prior_generation: Option<u64>,
    ) -> Result<(), GameAppShellError> {
        match prior_public_pointer {
            Some(bytes) => {
                write_atomic_manifest(&self.save_path, bytes)?;
                if fs::read(&self.save_path)? != bytes {
                    return Err(GameAppShellError::InvalidProductionFrontend {
                        message: "GPU checkpoint prior authority pointer rollback changed bytes"
                            .to_string(),
                    });
                }
                let restored = self.load()?;
                if restored.authority_generation() != prior_generation {
                    return Err(GameAppShellError::InvalidProductionFrontend {
                        message:
                            "GPU checkpoint prior authority pointer rollback generation mismatch"
                                .to_string(),
                    });
                }
            }
            None => {
                match fs::remove_file(&self.save_path) {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                }
                sync_parent_directory(self.save_path.parent().ok_or_else(|| {
                    GameAppShellError::InvalidProductionFrontend {
                        message: "GPU checkpoint save manifest has no parent directory".to_string(),
                    }
                })?)?;
                if self.save_path.exists() {
                    return Err(GameAppShellError::InvalidProductionFrontend {
                        message:
                            "GPU checkpoint new authority pointer rollback left a public manifest"
                                .to_string(),
                    });
                }
            }
        }
        Ok(())
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
        let replacement_content_digest = GpuSaveManifestDigest::for_bytes(&replacement_bytes);
        let durable = Self {
            save_path: save_path.clone(),
            asset_root: asset_root.clone(),
        };
        let _guard =
            SAVE_CAS_GUARD
                .lock()
                .map_err(|_| GameAppShellError::InvalidProductionFrontend {
                    message: "GPU checkpoint save CAS lock was poisoned".to_string(),
                })?;
        let prior = if save_path.exists() {
            durable.load()?.authority
        } else {
            GpuCheckpointAuthoritySource::LegacyDirectV1
        };
        let replacement_base = GpuLoadedSaveManifest {
            save: replacement.clone(),
            digest: replacement_content_digest,
            authority: prior,
        };
        let journal = GpuSleepTransactionJournalV2::empty(&replacement_base)?;
        let pointer = durable.prepare_authority_generation(&replacement_base, &journal)?;
        let committed = durable.commit_authority_pointer(replacement, &pointer)?;
        drop(_guard);

        let durable = Self {
            save_path: fs::canonicalize(&save_path)?,
            asset_root,
        };
        let published = committed;
        if published.save != *replacement {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "atomic GPU checkpoint publication changed the replacement save"
                    .to_string(),
            });
        }
        if published.authority.generation() != Some(&pointer)
            || durable.load_sleep_transaction_journal(&published)? != journal
        {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "atomic GPU checkpoint authority generation failed reopen validation"
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
        let replacement_content_digest = GpuSaveManifestDigest::for_bytes(&replacement_bytes);
        let _guard =
            SAVE_CAS_GUARD
                .lock()
                .map_err(|_| GameAppShellError::InvalidProductionFrontend {
                    message: "GPU checkpoint save CAS lock was poisoned".to_string(),
                })?;

        let current = self.load()?;
        if current.digest.as_str() == replacement_content_digest.as_str() {
            return Ok(GpuSaveManifestCasOutcome::AlreadyApplied {
                replacement_digest: current.digest,
            });
        }
        if &current.digest != expected {
            return Err(GameAppShellError::GpuCheckpointManifestConflict {
                expected: expected.as_str().to_string(),
                actual: current.digest.as_str().to_string(),
            });
        }

        let pre_replace = self.load()?;
        if &pre_replace.digest != expected {
            return Err(GameAppShellError::GpuCheckpointManifestConflict {
                expected: expected.as_str().to_string(),
                actual: pre_replace.digest.as_str().to_string(),
            });
        }
        let replacement_base = GpuLoadedSaveManifest {
            save: replacement.clone(),
            digest: replacement_content_digest,
            authority: pre_replace.authority,
        };
        let reset_journal = GpuSleepTransactionJournalV2::empty(&replacement_base)?;
        let pointer = self.prepare_authority_generation(&replacement_base, &reset_journal)?;
        let published = self.commit_authority_pointer(replacement, &pointer)?;
        if published.save != *replacement
            || published.authority.generation() != Some(&pointer)
            || self.load_sleep_transaction_journal(&published)? != reset_journal
        {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "atomic GPU checkpoint authority generation failed reopen validation"
                    .to_string(),
            });
        }
        Ok(GpuSaveManifestCasOutcome::Replaced {
            replacement_digest: published.digest,
        })
    }
}

fn valid_authority_artifact_file_name(file_name: &str) -> bool {
    !file_name.is_empty()
        && file_name.len() <= 240
        && !file_name.contains('/')
        && !file_name.contains('\\')
        && file_name != "."
        && file_name != ".."
        && file_name.is_ascii()
}

fn asset_manifest_identity(
    save: &PortableSaveFile,
) -> Result<PortableAssetDigest, GameAppShellError> {
    // AssetManifest is itself a versioned wire record. Its content digest is
    // stable across filesystem relocation; validate_with_asset_root separately
    // proves that the supplied root owns the named bytes.
    Ok(PortableAssetDigest::for_bytes(&serde_json::to_vec(
        &save.assets,
    )?))
}

fn authority_save_file_name(
    save_path: &Path,
    generation: u64,
    digest: &PortableAssetDigest,
) -> Result<String, GameAppShellError> {
    authority_artifact_file_name(save_path, generation, "save", digest)
}

fn authority_journal_file_name(
    save_path: &Path,
    generation: u64,
    digest: &PortableAssetDigest,
) -> Result<String, GameAppShellError> {
    authority_artifact_file_name(save_path, generation, "sleep-journal-v2", digest)
}

fn authority_artifact_file_name(
    save_path: &Path,
    generation: u64,
    kind: &str,
    digest: &PortableAssetDigest,
) -> Result<String, GameAppShellError> {
    digest.validate_format()?;
    let save_name = save_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
            message: "GPU checkpoint save manifest requires a UTF-8 file name".to_string(),
        })?;
    let digest_hex = digest.0.strip_prefix("fnv1a64:").ok_or_else(|| {
        GameAppShellError::InvalidProductionFrontend {
            message: "GPU checkpoint journal digest uses an unsupported algorithm".to_string(),
        }
    })?;
    let file_name = format!(".{save_name}.{kind}-g{generation:020}-{digest_hex}.json");
    if !valid_authority_artifact_file_name(&file_name) {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: "GPU checkpoint authority journal file name is invalid".to_string(),
        });
    }
    Ok(file_name)
}

fn encode_authoritative_save(
    save: &PortableSaveFile,
    pointer: &GpuCheckpointAuthorityPointerV1,
) -> Result<Vec<u8>, GameAppShellError> {
    pointer.validate()?;
    let mut value = serde_json::to_value(save)?;
    let object =
        value
            .as_object_mut()
            .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
                message: "GPU checkpoint save must encode as a JSON object".to_string(),
            })?;
    object.insert(
        GPU_CHECKPOINT_AUTHORITY_FIELD.to_string(),
        serde_json::to_value(pointer)?,
    );
    Ok(serde_json::to_vec_pretty(&value)?)
}

fn write_immutable_authority_artifact(path: &Path, bytes: &[u8]) -> Result<(), GameAppShellError> {
    match OpenOptions::new().create_new(true).write(true).open(path) {
        Ok(mut file) => {
            if let Err(error) = (|| -> std::io::Result<()> {
                file.write_all(bytes)?;
                file.sync_all()?;
                drop(file);
                sync_parent_directory(path.parent().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "authority artifact has no parent",
                    )
                })?)
            })() {
                let _ = fs::remove_file(path);
                return Err(error.into());
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if fs::read(path)? != bytes {
                return Err(GameAppShellError::InvalidProductionFrontend {
                    message: "GPU checkpoint immutable authority artifact conflicts".to_string(),
                });
            }
        }
        Err(error) => return Err(error.into()),
    }
    if fs::read(path)? != bytes {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: "GPU checkpoint immutable authority artifact failed verification".to_string(),
        });
    }
    Ok(())
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
    use alife_core::{
        BrainScaleTier, ConsolidationIntent, ConsolidationState, SleepPhase, Vec3f,
        SLEEP_CONSOLIDATION_SCHEMA_VERSION,
    };
    use alife_world::{
        persistence::{AssetManifest, RuntimeConfig},
        HeadlessScenarioBuilder,
    };

    fn current_authority_save(save_id: &str) -> PortableSaveFile {
        let world = HeadlessScenarioBuilder::new(73_128)
            .food("authority-stage-food", Vec3f::ZERO, 0.25)
            .build()
            .unwrap();
        PortableSaveFile::from_headless_world(
            save_id,
            &world,
            RuntimeConfig::deterministic_default(world.seed(), BrainScaleTier::Nano512),
            AssetManifest::empty(),
            Vec::new(),
        )
        .unwrap()
    }

    fn forced_recovery_state(phase: SleepPhase, tick: u64) -> SleepState {
        SleepState {
            schema_version: SLEEP_CONSOLIDATION_SCHEMA_VERSION,
            phase,
            phase_started_tick: Tick::new(tick),
            entered_sleep_tick: Some(Tick::new(2)),
            cycles_completed: 0,
            last_trigger: Some(SleepTrigger::RecoveryProtocol),
            active_cycle_id: 1,
            last_consolidated_cycle_id: 0,
            consolidation: ConsolidationState::None,
        }
    }

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
        assert!(GpuSleepTransactionJournalV2::try_new(
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
            GpuSleepTransactionJournalEntryV2::try_new(
                OrganismId(1),
                Tick::new(2),
                a_source,
                a_target,
            )
            .unwrap(),
            GpuSleepTransactionJournalEntryV2::try_new(
                OrganismId(2),
                Tick::new(3),
                b_source,
                b_target,
            )
            .unwrap(),
            GpuSleepTransactionJournalEntryV2::try_new(
                OrganismId(1),
                Tick::new(4),
                a_source,
                a_target,
            )
            .unwrap(),
        ];
        assert!(GpuSleepTransactionJournalV2::try_new(
            "fnv1a64:0123456789abcdef".to_string(),
            Tick::new(1),
            entries,
        )
        .is_err());
    }

    #[test]
    fn journal_pre_pending_phase_edges_are_exact_and_fail_closed() {
        let awake = SleepState::awake_at(Tick::new(1));
        let forced = forced_recovery_state(SleepPhase::ForcedRecoverySleep, 2);
        let consolidating = forced_recovery_state(SleepPhase::Consolidating, 3);
        GpuSleepTransactionJournalEntryV2::try_new(OrganismId(1), Tick::new(2), awake, forced)
            .unwrap();
        GpuSleepTransactionJournalEntryV2::try_new(
            OrganismId(1),
            Tick::new(3),
            forced,
            consolidating,
        )
        .unwrap();

        let mut skipped_phase = consolidating;
        skipped_phase.entered_sleep_tick = Some(Tick::new(2));
        assert!(GpuSleepTransactionJournalEntryV2::try_new(
            OrganismId(1),
            Tick::new(3),
            awake,
            skipped_phase,
        )
        .is_err());
        assert!(GpuSleepTransactionJournalEntryV2::try_new(
            OrganismId(1),
            Tick::new(4),
            consolidating,
            forced,
        )
        .is_err());

        let mut changed_trigger = consolidating;
        changed_trigger.last_trigger = Some(SleepTrigger::UnsafeActiveState);
        assert!(GpuSleepTransactionJournalEntryV2::try_new(
            OrganismId(1),
            Tick::new(4),
            forced,
            changed_trigger,
        )
        .is_err());
        let mut changed_counter = consolidating;
        changed_counter.cycles_completed = 1;
        assert!(GpuSleepTransactionJournalEntryV2::try_new(
            OrganismId(1),
            Tick::new(4),
            forced,
            changed_counter,
        )
        .is_err());
        let mut changed_cycle = consolidating;
        changed_cycle.active_cycle_id = 2;
        assert!(GpuSleepTransactionJournalEntryV2::try_new(
            OrganismId(1),
            Tick::new(4),
            forced,
            changed_cycle,
        )
        .is_err());
        let entering = SleepState {
            phase: SleepPhase::EnteringSleep,
            ..forced
        };
        assert!(GpuSleepTransactionJournalEntryV2::try_new(
            OrganismId(1),
            Tick::new(4),
            entering,
            forced,
        )
        .is_err());
        assert!(GpuSleepTransactionJournalEntryV2::try_new(
            OrganismId(1),
            Tick::new(1),
            awake,
            forced,
        )
        .is_err());
        let mut future_phase = consolidating;
        future_phase.phase_started_tick = Tick::new(5);
        assert!(GpuSleepTransactionJournalEntryV2::try_new(
            OrganismId(1),
            Tick::new(4),
            forced,
            future_phase,
        )
        .is_err());
    }

    #[test]
    fn journal_none_to_pending_requires_canonical_consolidating_identity() {
        let source = forced_recovery_state(SleepPhase::Consolidating, 3);
        let target = SleepState {
            consolidation: ConsolidationState::Pending {
                intent: ConsolidationIntent { cycle_id: 1 },
                replay_digest: [9; 4],
                replay_event_count: 1,
                replay_eligibility_sample_count: 0,
            },
            ..source
        };
        GpuSleepTransactionJournalEntryV2::try_new(OrganismId(1), Tick::new(4), source, target)
            .unwrap();

        let mut changed_trigger = target;
        changed_trigger.last_trigger = Some(SleepTrigger::UnsafeActiveState);
        assert!(GpuSleepTransactionJournalEntryV2::try_new(
            OrganismId(1),
            Tick::new(4),
            source,
            changed_trigger,
        )
        .is_err());
        let mut changed_cycle = target;
        changed_cycle.active_cycle_id = 2;
        assert!(GpuSleepTransactionJournalEntryV2::try_new(
            OrganismId(1),
            Tick::new(4),
            source,
            changed_cycle,
        )
        .is_err());
    }

    #[test]
    fn journal_compound_tick_ordinals_are_bounded_and_contiguous() {
        let forced = forced_recovery_state(SleepPhase::ForcedRecoverySleep, 2);
        let consolidating = forced_recovery_state(SleepPhase::Consolidating, 3);
        let pending = SleepState {
            consolidation: ConsolidationState::Pending {
                intent: ConsolidationIntent { cycle_id: 1 },
                replay_digest: [9; 4],
                replay_event_count: 1,
                replay_eligibility_sample_count: 0,
            },
            ..consolidating
        };
        let first = GpuSleepTransactionJournalEntryV2::try_new_with_ordinal(
            OrganismId(1),
            Tick::new(4),
            0,
            forced,
            consolidating,
        )
        .unwrap();
        let second = GpuSleepTransactionJournalEntryV2::try_new_with_ordinal(
            OrganismId(1),
            Tick::new(4),
            1,
            consolidating,
            pending,
        )
        .unwrap();
        GpuSleepTransactionJournalV2::try_new(
            "fnv1a64:0123456789abcdef".to_string(),
            Tick::new(1),
            vec![first.clone(), second.clone()],
        )
        .unwrap();

        for malformed in [
            vec![first.clone(), first.clone()],
            vec![second.clone()],
            vec![second.clone(), first.clone()],
        ] {
            assert!(GpuSleepTransactionJournalV2::try_new(
                "fnv1a64:0123456789abcdef".to_string(),
                Tick::new(1),
                malformed,
            )
            .is_err());
        }
        assert!(GpuSleepTransactionJournalEntryV2::try_new_with_ordinal(
            OrganismId(1),
            Tick::new(4),
            2,
            forced,
            consolidating,
        )
        .is_err());

        let mismatched_source = SleepState {
            last_trigger: Some(SleepTrigger::UnsafeActiveState),
            ..consolidating
        };
        let mismatched_target = SleepState {
            last_trigger: Some(SleepTrigger::UnsafeActiveState),
            ..pending
        };
        let mismatched_second = GpuSleepTransactionJournalEntryV2::try_new_with_ordinal(
            OrganismId(1),
            Tick::new(4),
            1,
            mismatched_source,
            mismatched_target,
        )
        .unwrap();
        assert!(GpuSleepTransactionJournalV2::try_new(
            "fnv1a64:0123456789abcdef".to_string(),
            Tick::new(1),
            vec![first, mismatched_second],
        )
        .is_err());

        let awake = SleepState::awake_at(Tick::new(1));
        let phase_only_first = GpuSleepTransactionJournalEntryV2::try_new_with_ordinal(
            OrganismId(1),
            Tick::new(4),
            0,
            awake,
            forced,
        )
        .unwrap();
        let phase_only_second = GpuSleepTransactionJournalEntryV2::try_new_with_ordinal(
            OrganismId(1),
            Tick::new(4),
            1,
            forced,
            consolidating,
        )
        .unwrap();
        assert!(GpuSleepTransactionJournalV2::try_new(
            "fnv1a64:0123456789abcdef".to_string(),
            Tick::new(1),
            vec![phase_only_first, phase_only_second],
        )
        .is_err());
    }

    #[test]
    fn journal_vs_base_rejects_first_source_mismatch() {
        let source = committed_sleep(SleepPhase::Consolidating, 2);
        let target = committed_sleep(SleepPhase::Waking, 3);
        let journal = GpuSleepTransactionJournalV2::try_new(
            "fnv1a64:0123456789abcdef".to_string(),
            Tick::new(1),
            vec![GpuSleepTransactionJournalEntryV2::try_new(
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
        let journal = GpuSleepTransactionJournalV2::try_new(
            "fnv1a64:0123456789abcdef".to_string(),
            Tick::new(1),
            vec![GpuSleepTransactionJournalEntryV2::try_new(
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
        let journal = GpuSleepTransactionJournalV2::try_new(
            "fnv1a64:0123456789abcdef".to_string(),
            Tick::new(1),
            vec![GpuSleepTransactionJournalEntryV2::try_new(
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

    #[test]
    fn authority_failure_stages_never_expose_a_mixed_pair() {
        let root = std::env::temp_dir().join(format!(
            "alife-gpu-authority-stage-failure-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(&root).unwrap();
        let save_path = root.join("current.json");
        let base = current_authority_save("authority-stage-base");
        GpuDurableSaveManifest::publish_snapshot(&save_path, &root, &base).unwrap();
        let durable = GpuDurableSaveManifest::open(&save_path, &root).unwrap();
        let loaded = durable.load().unwrap();
        let old_pointer_bytes = fs::read(&save_path).unwrap();
        let mut replacement = loaded.save.clone();
        replacement.save_id = "authority-stage-replacement".to_string();

        for stage in [
            AuthorityTestFailureStage::SavePrepared,
            AuthorityTestFailureStage::JournalPrepared,
            AuthorityTestFailureStage::PointerCommit,
        ] {
            set_authority_test_failure(stage);
            assert!(durable
                .compare_and_swap(&loaded.digest, &replacement)
                .is_err());
            assert_eq!(fs::read(&save_path).unwrap(), old_pointer_bytes);
            let still_current = durable.load().unwrap();
            assert_eq!(still_current.save, loaded.save);
            assert_eq!(still_current.authority_generation(), Some(1));
        }

        set_authority_test_failure(AuthorityTestFailureStage::Reopen);
        assert!(matches!(
            durable.compare_and_swap(&loaded.digest, &replacement),
            Err(
                GameAppShellError::GpuCheckpointAuthorityPostCommitValidation {
                    committed_generation: 2,
                    last_known_good_generation: 1,
                    ..
                }
            )
        ));
        assert_eq!(fs::read(&save_path).unwrap(), old_pointer_bytes);
        let committed = durable.load().unwrap();
        assert_eq!(committed.save, loaded.save);
        assert_eq!(committed.authority_generation(), Some(1));
        assert!(durable
            .load_sleep_transaction_journal(&committed)
            .unwrap()
            .entries
            .is_empty());

        fs::remove_dir_all(root).unwrap();
    }
}
