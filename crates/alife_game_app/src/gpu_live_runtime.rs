//! GPU-authoritative live cognition for the explicit neural policy.

use std::collections::{BTreeMap, BTreeSet};

use alife_core::{
    BrainCapacityClass, BrainGenome, BrainScaleTier, BrainTickStatus, BrainWorkReceipt, Confidence,
    ConsolidationDriverEvent, ConsolidationIntent, ConsolidationState, DecisionSnapshot,
    DevelopmentState, ExperiencePatch, ExperiencePatchBuilder, ExperienceSequenceId,
    FinalizedMemoryRecall, HomeostaticDelta, HomeostaticParameters, HomeostaticSnapshot,
    MemoryBankConfig, MemoryCompactionCheckpoint, MemoryCompactionReceipt, MemoryRecallReceipt,
    MemorySidecarState, MemoryUpdateReceipt, NeuralActionSelection, NormalizedScalar, OrganismId,
    PerceptionFrame, PhenotypeCompiler, PhenotypeCompilerInputs, PostActionOutcome,
    PreActionSnapshot, ScaffoldContractError, SensorProfile, SensorProfileIdentity,
    SensoryAbiVersion, SleepConsolidationConfig, SleepPhase, SleepState, Tick,
    TopologicalMapConfig, TopologyObservationReceipt, TopologySidecar, Validate,
};
use alife_gpu_backend::{
    GpuBrainHandle, GpuClosedLoopBackend, GpuClosedLoopMemoryBatchInput,
    GpuClosedLoopMemoryTickInput, GpuClosedLoopTick, GpuLearningReceipt, GpuMemoryContextUpload,
    PendingEligibilityDiscardReceipt, PendingEligibilityIdentity, PendingEligibilityReceipt,
    GPU_FAST_PLASTICITY_COMMIT_BYTES, GPU_SELECTION_RECORD_BYTES,
};
use alife_world::{
    persistence::{AssetManifest, GpuBrainSaveState, PortableSaveFile, RuntimeConfig},
    HeadlessWorld,
};

use crate::{
    merge_gpu_checkpoint_manifest_entries, AppShellLaunchConfig, GameAppShellError,
    GpuBrainAuthorityTelemetry, GpuBrainCheckpointWrite, GpuBrainSidecarCapture,
    GpuCheckpointAssetStore, GpuDurableSaveManifest, GpuLoadedSaveManifest,
    GpuSleepConsolidationDriver, GpuSleepScheduler, LiveBrainCausalStage, LiveBrainTickSummary,
    RetainedLearningCapture, G03_LIVE_BRAIN_LOOP_SCHEMA, G03_LIVE_BRAIN_LOOP_SCHEMA_VERSION,
};

#[derive(Debug, Clone)]
struct ResidentCognition {
    phenotype: alife_core::BrainPhenotype,
    compiler_inputs: PhenotypeCompilerInputs,
    genome: BrainGenome,
    development: DevelopmentState,
    homeostasis: HomeostaticSnapshot,
    sleep_scheduler: GpuSleepScheduler,
    next_sequence: u64,
}

#[derive(Debug, Clone)]
struct GpuLiveCheckpointDurability {
    store: GpuCheckpointAssetStore,
    durable_manifest: GpuDurableSaveManifest,
    published: GpuLoadedSaveManifest,
}

impl GpuLiveCheckpointDurability {
    fn publish(&mut self, replacement: PortableSaveFile) -> Result<(), GameAppShellError> {
        self.durable_manifest
            .compare_and_swap(&self.published.digest, &replacement)?;
        let published = self.durable_manifest.load()?;
        if published.save != replacement {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "published GPU checkpoint save differs from its validated replacement"
                    .to_string(),
            });
        }
        self.published = published;
        Ok(())
    }
}

struct AuthoritativeGpuSleepDriver<'a> {
    backend: &'a mut GpuClosedLoopBackend,
    handle: GpuBrainHandle,
}

impl GpuSleepConsolidationDriver for AuthoritativeGpuSleepDriver<'_> {
    fn progress(
        &mut self,
        organism_id: OrganismId,
        state: SleepState,
        intent: Option<ConsolidationIntent>,
    ) -> Result<Option<ConsolidationDriverEvent>, ScaffoldContractError> {
        if organism_id != self.handle.organism_id() {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        let event = match (state.consolidation, intent) {
            (ConsolidationState::None, Some(intent)) => {
                let replay = self.backend.build_sleep_replay_batch(self.handle)?;
                ConsolidationDriverEvent::ReplayAssetPersisted {
                    intent,
                    replay_digest: replay.canonical_digest,
                    replay_event_count: replay.events.len() as u32,
                    replay_eligibility_sample_count: replay.eligibility_samples.len() as u32,
                }
            }
            (
                ConsolidationState::Pending {
                    intent,
                    replay_digest,
                    replay_event_count,
                    replay_eligibility_sample_count,
                },
                None,
            ) => {
                let replay = self.backend.build_sleep_replay_batch(self.handle)?;
                if replay.canonical_digest != replay_digest
                    || replay.events.len() as u32 != replay_event_count
                    || replay.eligibility_samples.len() as u32 != replay_eligibility_sample_count
                {
                    return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
                }
                let request =
                    self.backend
                        .prepare_sleep_consolidation(self.handle, intent, &replay)?;
                ConsolidationDriverEvent::Prepared { request }
            }
            (ConsolidationState::Prepared { request }, None) => {
                let replay = self.backend.build_sleep_replay_batch(self.handle)?;
                if replay.canonical_digest != request.replay_digest {
                    return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
                }
                let job_id =
                    self.backend
                        .submit_sleep_consolidation(self.handle, &request, &replay)?;
                ConsolidationDriverEvent::Submitted { request, job_id }
            }
            (ConsolidationState::Submitted { request, job_id }, None) => {
                match self.backend.poll_sleep_consolidation(self.handle, job_id) {
                    Ok(Some(staged)) => ConsolidationDriverEvent::Completed {
                        request,
                        staged: staged.staged,
                    },
                    Ok(None) => return Ok(None),
                    Err(ScaffoldContractError::ConsolidationGenerationMismatch) => {
                        let replay = self.backend.build_sleep_replay_batch(self.handle)?;
                        if replay.canonical_digest != request.replay_digest {
                            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
                        }
                        let recovered_job_id = self.backend.recover_submitted_sleep_consolidation(
                            self.handle,
                            &request,
                            &replay,
                            job_id,
                        )?;
                        ConsolidationDriverEvent::RecoveredSubmitted {
                            request,
                            lost_job_id: job_id,
                            recovered_job_id,
                        }
                    }
                    Err(error) => return Err(error),
                }
            }
            (ConsolidationState::Completed { request, staged }, None) => {
                let receipt =
                    self.backend
                        .commit_sleep_consolidation(self.handle, &request, &staged)?;
                ConsolidationDriverEvent::Committed {
                    cycle_id: request.cycle_id,
                    output_generation: receipt.output_generation,
                    output_digest: receipt.output_digest,
                }
            }
            (ConsolidationState::Committed { .. }, None) => return Ok(None),
            _ => return Err(ScaffoldContractError::ConsolidationGenerationMismatch),
        };
        Ok(Some(event))
    }
}

type SleepProgressResult = Result<Option<ConsolidationDriverEvent>, ScaffoldContractError>;

struct RoutedGpuSleepDriver<'a, F> {
    backend: &'a mut GpuClosedLoopBackend,
    handle: GpuBrainHandle,
    progress: &'a mut F,
}

impl<F> GpuSleepConsolidationDriver for RoutedGpuSleepDriver<'_, F>
where
    F: FnMut(
        &mut GpuClosedLoopBackend,
        GpuBrainHandle,
        OrganismId,
        SleepState,
        Option<ConsolidationIntent>,
    ) -> SleepProgressResult,
{
    fn progress(
        &mut self,
        organism_id: OrganismId,
        state: SleepState,
        intent: Option<ConsolidationIntent>,
    ) -> SleepProgressResult {
        (self.progress)(self.backend, self.handle, organism_id, state, intent)
    }
}

struct PreparedLiveSelection {
    handle: GpuBrainHandle,
    pending_eligibility: PendingEligibilityReceipt,
    frame: PerceptionFrame,
    memory_recall: FinalizedMemoryRecall,
    sequence_id: ExperienceSequenceId,
    outcome_tick: Tick,
    pre_action: PreActionSnapshot,
    decision: DecisionSnapshot,
}

struct SealedLiveSelection {
    handle: GpuBrainHandle,
    pending_eligibility: PendingEligibilityReceipt,
    summary: LiveBrainTickSummary,
    patch: ExperiencePatch,
}

struct PreparedGpuBrainFrame {
    handle: GpuBrainHandle,
    frame: PerceptionFrame,
    memory_recall: FinalizedMemoryRecall,
    memory_upload: GpuMemoryContextUpload,
}

const LIVE_MEMORY_CAPACITY: usize = 64;
const LIVE_MEMORY_MAX_FEATURE_LEN: usize = 64;
const LIVE_MEMORY_MAX_MATCH_COUNT: usize = 4;
const LIVE_MEMORY_MIN_MATCH_SCORE: f32 = 0.72;
const MAX_RETAINED_LEARNING_RETRIES: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetainedLearningErrorCode {
    LearningEvidenceMismatch,
    NeuralBackendUnavailable,
    OtherContractFailure,
}

impl RetainedLearningErrorCode {
    fn from_error(error: &ScaffoldContractError) -> Self {
        match error {
            ScaffoldContractError::LearningEvidenceMismatch => Self::LearningEvidenceMismatch,
            ScaffoldContractError::NeuralBackendUnavailable => Self::NeuralBackendUnavailable,
            _ => Self::OtherContractFailure,
        }
    }

    const fn slug(self) -> &'static str {
        match self {
            Self::LearningEvidenceMismatch => "learning-evidence-mismatch",
            Self::NeuralBackendUnavailable => "neural-backend-unavailable",
            Self::OtherContractFailure => "other-contract-failure",
        }
    }

    fn from_slug(slug: &str) -> Result<Self, ScaffoldContractError> {
        match slug {
            "learning-evidence-mismatch" => Ok(Self::LearningEvidenceMismatch),
            "neural-backend-unavailable" => Ok(Self::NeuralBackendUnavailable),
            "other-contract-failure" => Ok(Self::OtherContractFailure),
            _ => Err(ScaffoldContractError::LearningEvidenceMismatch),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetainedLearningRecoveryStatus {
    pub organism_id: OrganismId,
    pub sequence_id: ExperienceSequenceId,
    pub attempts: u8,
    pub last_error: RetainedLearningErrorCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreSealDiscardFailure {
    pub organism_id: OrganismId,
    pub identity: PendingEligibilityIdentity,
    pub error: RetainedLearningErrorCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PostSealLearningFailure {
    pub organism_id: OrganismId,
    pub sequence_id: ExperienceSequenceId,
    pub pending: PendingEligibilityReceipt,
    pub error: RetainedLearningErrorCode,
    pub retained_for_recovery: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TopologyObservationDisposition {
    Observed(Box<TopologyObservationReceipt>),
    RejectedMissingOwner { organism_id: OrganismId },
}

impl TopologyObservationDisposition {
    pub fn was_observed(&self) -> bool {
        matches!(
            self,
            Self::Observed(receipt) if !receipt.rejected_invalid && !receipt.replay_rejected
        )
    }

    pub fn receipt(&self) -> Option<&TopologyObservationReceipt> {
        match self {
            Self::Observed(receipt) => Some(receipt),
            Self::RejectedMissingOwner { .. } => None,
        }
    }
}

struct RetainedLearningRecovery {
    handle: GpuBrainHandle,
    pending: PendingEligibilityReceipt,
    sealed_patch: ExperiencePatch,
    attempts: u8,
    last_error: RetainedLearningErrorCode,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct GpuLiveBrainEvidenceMetrics {
    pub completed_dispatch_count: u64,
    pub completed_selection_count: u64,
    pub selection_readback_bytes: usize,
    pub pending_eligibility_readback_bytes: usize,
    pub learning_readback_bytes: usize,
    pub compact_readback_bytes: usize,
    pub active_tiles: u32,
    pub active_synapses: u32,
}

/// Owns all production neural authority for one headless world.
pub struct GpuLiveBrainRuntime {
    backend: GpuClosedLoopBackend,
    handles: BTreeMap<u64, GpuBrainHandle>,
    residents: BTreeMap<u64, ResidentCognition>,
    memories: BTreeMap<u64, MemorySidecarState>,
    topologies: BTreeMap<u64, TopologySidecar>,
    retained_learning: BTreeMap<u64, RetainedLearningRecovery>,
    world: HeadlessWorld,
    deterministic_seed: u64,
    brain_class: BrainScaleTier,
    sensor_profile: SensorProfile,
    sealed_patches: Vec<ExperiencePatch>,
    last_learning_receipts: Vec<GpuLearningReceipt>,
    last_activity_work_receipts: Vec<BrainWorkReceipt>,
    last_memory_recall_receipts: Vec<MemoryRecallReceipt>,
    last_memory_update_receipts: Vec<MemoryUpdateReceipt>,
    last_memory_compaction_receipts: Vec<MemoryCompactionReceipt>,
    last_memory_preparation_errors: Vec<(OrganismId, ScaffoldContractError)>,
    last_memory_observation_errors: Vec<(OrganismId, ScaffoldContractError)>,
    last_topology_observations: Vec<TopologyObservationDisposition>,
    #[cfg(feature = "gpu-tests")]
    forced_memory_preparation_failures: BTreeSet<u64>,
    last_eligibility_discard_receipts: Vec<PendingEligibilityDiscardReceipt>,
    last_pre_seal_discard_failures: Vec<PreSealDiscardFailure>,
    last_post_seal_learning_failures: Vec<PostSealLearningFailure>,
    last_gpu_metrics: GpuLiveBrainEvidenceMetrics,
    checkpoint_durability: Option<GpuLiveCheckpointDurability>,
}

impl GpuLiveBrainRuntime {
    pub fn from_p34_launch(
        backend: GpuClosedLoopBackend,
        launch: &AppShellLaunchConfig,
    ) -> Result<Self, GameAppShellError> {
        let config = RuntimeConfig::from_json_file(&launch.config_path)?;
        config.validate()?;
        let manifest = AssetManifest::from_json_file(&launch.asset_manifest_path)?;
        manifest.validate_with_root(&launch.asset_root)?;
        let durable_manifest = GpuDurableSaveManifest::open(&launch.save_path, &launch.asset_root)?;
        let loaded_save = durable_manifest.load()?;
        let save = loaded_save.save.clone();
        save.validate_with_asset_root(&launch.asset_root)?;
        if launch.brain_policy != alife_core::PolicyBackend::NeuralClosedLoopGpu
            || config.brain_policy.policy != alife_core::PolicyBackend::NeuralClosedLoopGpu
            || save.config.brain_policy.policy != alife_core::PolicyBackend::NeuralClosedLoopGpu
            || config.deterministic_seed != save.deterministic_seed
        {
            return Err(GameAppShellError::InvalidGraphicalLaunch {
                message: "GPU neural runtime requires matching persisted neural policy and seed",
            });
        }
        let world = save.restore_headless_world()?;
        let world_tick = world.tick();
        let store = GpuCheckpointAssetStore::new(launch.asset_root.clone())?;
        let checkpoints = save
            .creatures
            .iter()
            .filter_map(|creature| creature.gpu_brain.clone())
            .collect::<Vec<_>>();
        let requires_checkpoint_reconciliation = checkpoints.len() != save.creatures.len()
            || checkpoints.iter().any(|state| {
                state.pending_eligibility.is_some()
                    || state.pending_experience_transaction.is_some()
            });
        let mut runtime = Self::restore_with_checkpoints(
            backend,
            world,
            config.deterministic_seed,
            config.brain_class,
            &store,
            &save.assets,
            &checkpoints,
        )?;
        for creature in &save.creatures {
            let Some(resident) = runtime.residents.get_mut(&creature.organism_id.raw()) else {
                return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
            };
            if creature.brain_class != config.brain_class {
                return Err(ScaffoldContractError::PhenotypeCompile.into());
            }
            resident.homeostasis = HomeostaticSnapshot::new(
                world_tick,
                creature.mind.homeostasis.drives,
                creature.mind.homeostasis.hormones,
            )?;
        }
        runtime.checkpoint_durability = Some(GpuLiveCheckpointDurability {
            store,
            durable_manifest,
            published: loaded_save,
        });
        if requires_checkpoint_reconciliation {
            runtime.persist_sleep_checkpoint_boundary()?;
        }
        Ok(runtime)
    }

    pub fn new(
        backend: GpuClosedLoopBackend,
        world: HeadlessWorld,
        deterministic_seed: u64,
        brain_class: BrainScaleTier,
    ) -> Result<Self, GameAppShellError> {
        Self::new_profiled(
            backend,
            world,
            deterministic_seed,
            brain_class,
            SensorProfile::PrivilegedAffordanceV1,
        )
    }

    pub fn new_profiled(
        backend: GpuClosedLoopBackend,
        world: HeadlessWorld,
        deterministic_seed: u64,
        brain_class: BrainScaleTier,
        sensor_profile: SensorProfile,
    ) -> Result<Self, GameAppShellError> {
        if deterministic_seed == 0 || brain_class.neuron_count().is_none() {
            return Err(GameAppShellError::Core(
                ScaffoldContractError::PhenotypeCompile,
            ));
        }
        let mut runtime = Self {
            backend,
            handles: BTreeMap::new(),
            residents: BTreeMap::new(),
            memories: BTreeMap::new(),
            topologies: BTreeMap::new(),
            retained_learning: BTreeMap::new(),
            world,
            deterministic_seed,
            brain_class,
            sensor_profile,
            sealed_patches: Vec::new(),
            last_learning_receipts: Vec::new(),
            last_activity_work_receipts: Vec::new(),
            last_memory_recall_receipts: Vec::new(),
            last_memory_update_receipts: Vec::new(),
            last_memory_compaction_receipts: Vec::new(),
            last_memory_preparation_errors: Vec::new(),
            last_memory_observation_errors: Vec::new(),
            last_topology_observations: Vec::new(),
            #[cfg(feature = "gpu-tests")]
            forced_memory_preparation_failures: BTreeSet::new(),
            last_eligibility_discard_receipts: Vec::new(),
            last_pre_seal_discard_failures: Vec::new(),
            last_post_seal_learning_failures: Vec::new(),
            last_gpu_metrics: GpuLiveBrainEvidenceMetrics::default(),
            checkpoint_durability: None,
        };
        runtime.reconcile_population()?;
        Ok(runtime)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn restore_with_checkpoints(
        backend: GpuClosedLoopBackend,
        world: HeadlessWorld,
        deterministic_seed: u64,
        brain_class: BrainScaleTier,
        store: &GpuCheckpointAssetStore,
        manifest: &AssetManifest,
        checkpoints: &[GpuBrainSaveState],
    ) -> Result<Self, GameAppShellError> {
        if deterministic_seed == 0 || brain_class.neuron_count().is_none() {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        manifest.validate_with_root(store.root())?;
        let checkpoint_index = checkpoints
            .iter()
            .map(|state| (state.organism_id.raw(), state))
            .collect::<BTreeMap<_, _>>();
        if checkpoint_index.len() != checkpoints.len() {
            return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
        }
        let saved_profile = checkpoints.first().map_or(
            SensorProfileIdentity {
                profile_id: SensorProfile::PrivilegedAffordanceV1.into(),
                profile_schema_version: 1,
                sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
            },
            |state| state.sensor_profile,
        );
        saved_profile.validate_contract()?;
        if checkpoints
            .iter()
            .any(|state| state.sensor_profile != saved_profile)
        {
            return Err(ScaffoldContractError::SensorProfileMismatch.into());
        }
        if checkpoints.iter().any(|state| {
            state.memory.summary.profile != state.sensor_profile
                || state.topology.profile != state.sensor_profile
        }) {
            return Err(ScaffoldContractError::SensorProfileMismatch.into());
        }
        let sensor_profile = saved_profile.profile()?;
        let live_ids = world
            .organism_entity_ids()
            .into_iter()
            .map(|(organism_id, _)| organism_id.raw())
            .collect::<BTreeSet<_>>();
        if checkpoint_index.keys().any(|raw| !live_ids.contains(raw)) {
            return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
        }
        let world_tick = world.tick();
        let mut runtime = Self {
            backend,
            handles: BTreeMap::new(),
            residents: BTreeMap::new(),
            memories: BTreeMap::new(),
            topologies: BTreeMap::new(),
            retained_learning: BTreeMap::new(),
            world,
            deterministic_seed,
            brain_class,
            sensor_profile,
            sealed_patches: Vec::new(),
            last_learning_receipts: Vec::new(),
            last_activity_work_receipts: Vec::new(),
            last_memory_recall_receipts: Vec::new(),
            last_memory_update_receipts: Vec::new(),
            last_memory_compaction_receipts: Vec::new(),
            last_memory_preparation_errors: Vec::new(),
            last_memory_observation_errors: Vec::new(),
            last_topology_observations: Vec::new(),
            #[cfg(feature = "gpu-tests")]
            forced_memory_preparation_failures: BTreeSet::new(),
            last_eligibility_discard_receipts: Vec::new(),
            last_pre_seal_discard_failures: Vec::new(),
            last_post_seal_learning_failures: Vec::new(),
            last_gpu_metrics: GpuLiveBrainEvidenceMetrics::default(),
            checkpoint_durability: None,
        };
        let mut tracked_object_states = Vec::new();
        for raw in live_ids {
            let organism_id = OrganismId(raw);
            if let Some(state) = checkpoint_index.get(&raw).copied() {
                if state.checkpoint_tick != world_tick
                    || state.capacity_class_id != brain_class.default_class_id()
                {
                    return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
                }
                let restored = store.restore_brain(&mut runtime.backend, manifest, state)?;
                let handle = restored.receipt.handle;
                let retained_sequence = restored
                    .retained_learning
                    .as_ref()
                    .map(|recovery| recovery.sealed_patch.pre_action().sequence_id);
                let pending_sequence = restored
                    .pending_transaction
                    .as_ref()
                    .map(|builder| builder.pending_decision().map(|(pre, _)| pre.sequence_id))
                    .transpose()?;
                if restored.retained_learning.is_none() {
                    if let Some(receipt) = restored.receipt.pending_eligibility {
                        let discard = runtime
                            .backend
                            .discard_pending_eligibility(handle, receipt.identity())?;
                        runtime.last_eligibility_discard_receipts.push(discard);
                    }
                }
                let next_sequence = match retained_sequence {
                    Some(sequence) => sequence
                        .raw()
                        .checked_add(1)
                        .ok_or(ScaffoldContractError::ScalarOutOfRange)?,
                    None => match pending_sequence {
                        Some(sequence) => sequence.raw(),
                        None => match state.last_learning_replay_key {
                            Some(key) => key
                                .sequence_id
                                .raw()
                                .checked_add(1)
                                .ok_or(ScaffoldContractError::ScalarOutOfRange)?,
                            None => 1,
                        },
                    },
                };
                let resident = ResidentCognition {
                    phenotype: restored.phenotype,
                    genome: restored.compiler_inputs.genome().clone(),
                    development: restored.compiler_inputs.development().clone(),
                    compiler_inputs: restored.compiler_inputs,
                    homeostasis: HomeostaticSnapshot::baseline(world_tick),
                    sleep_scheduler: GpuSleepScheduler::restore(
                        SleepConsolidationConfig::reference(),
                        restored.sleep,
                    )?,
                    next_sequence,
                };
                runtime.handles.insert(raw, handle);
                runtime.residents.insert(raw, resident);
                runtime.memories.insert(raw, restored.memory);
                runtime.topologies.insert(raw, restored.topology);
                tracked_object_states.push(restored.tracked_objects);
                if let Some(recovery) = restored.retained_learning {
                    let pending = restored
                        .receipt
                        .pending_eligibility
                        .ok_or(ScaffoldContractError::LearningEvidenceMismatch)?;
                    runtime.retained_learning.insert(
                        raw,
                        RetainedLearningRecovery {
                            handle,
                            pending,
                            sealed_patch: recovery.sealed_patch,
                            attempts: recovery.attempts,
                            last_error: RetainedLearningErrorCode::from_slug(
                                &recovery.last_error_code,
                            )?,
                        },
                    );
                }
            } else {
                let (phenotype, resident) = runtime.compile_birth(organism_id)?;
                let handle = runtime.backend.insert_brain(organism_id, phenotype)?;
                runtime.handles.insert(raw, handle);
                runtime.residents.insert(raw, resident);
                runtime
                    .memories
                    .insert(raw, Self::new_memory_sidecar(organism_id, sensor_profile)?);
                runtime.topologies.insert(
                    raw,
                    TopologySidecar::new_profiled(
                        organism_id,
                        saved_profile,
                        TopologicalMapConfig::default(),
                    )?,
                );
            }
        }
        runtime
            .world
            .restore_tracked_object_states(tracked_object_states)?;
        Ok(runtime)
    }

    pub fn checkpoint_brain(
        &mut self,
        organism_id: OrganismId,
        store: &GpuCheckpointAssetStore,
    ) -> Result<GpuBrainCheckpointWrite, GameAppShellError> {
        let handle = *self
            .handles
            .get(&organism_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let resident = self
            .residents
            .get(&organism_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let memory = self
            .memories
            .get(&organism_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let topology = self
            .topologies
            .get(&organism_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let retained_learning = self
            .retained_learning
            .get(&organism_id.raw())
            .map(|recovery| RetainedLearningCapture {
                sealed_patch: &recovery.sealed_patch,
                attempts: recovery.attempts,
                last_error_code: recovery.last_error.slug(),
            });
        store.capture_brain(
            &mut self.backend,
            handle,
            &resident.phenotype,
            &resident.compiler_inputs,
            resident.sleep_scheduler.state(),
            self.world.tick(),
            None,
            GpuBrainSidecarCapture {
                sensor_profile: memory.profile(),
                memory,
                topology,
                tracked_objects: self.world.tracked_objects().save_state(organism_id)?,
                retained_learning,
            },
        )
    }

    /// Captures one exact, sealed-boundary portable save without publishing it.
    /// The caller may atomically publish the returned manifest as a manual save;
    /// all bulk neural state remains behind content-addressed asset references.
    pub fn capture_portable_checkpoint(&mut self) -> Result<PortableSaveFile, GameAppShellError> {
        let Some(durability) = self.checkpoint_durability.take() else {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "GPU runtime has no durable save boundary".to_string(),
            });
        };
        let base = durability.published.save.clone();
        let store = durability.store.clone();
        let result = self.capture_checkpointed_save(base, &store);
        self.checkpoint_durability = Some(durability);
        result
    }

    fn capture_checkpointed_save(
        &mut self,
        mut replacement: PortableSaveFile,
        store: &GpuCheckpointAssetStore,
    ) -> Result<PortableSaveFile, GameAppShellError> {
        let checkpoint_tick = self.world.tick();
        replacement.replace_headless_world_snapshot(&self.world)?;
        let mut manifest_entries = Vec::new();
        for (&raw, &handle) in &self.handles {
            let resident = self
                .residents
                .get(&raw)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            if resident.homeostasis.tick != checkpoint_tick
                || resident.development.age_ticks != checkpoint_tick
            {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
            }
            let write = store.capture_brain(
                &mut self.backend,
                handle,
                &resident.phenotype,
                &resident.compiler_inputs,
                resident.sleep_scheduler.state(),
                checkpoint_tick,
                None,
                GpuBrainSidecarCapture {
                    sensor_profile: self
                        .memories
                        .get(&raw)
                        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                        .profile(),
                    memory: self
                        .memories
                        .get(&raw)
                        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?,
                    topology: self
                        .topologies
                        .get(&raw)
                        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?,
                    tracked_objects: self.world.tracked_objects().save_state(OrganismId(raw))?,
                    retained_learning: self.retained_learning.get(&raw).map(|recovery| {
                        RetainedLearningCapture {
                            sealed_patch: &recovery.sealed_patch,
                            attempts: recovery.attempts,
                            last_error_code: recovery.last_error.slug(),
                        }
                    }),
                },
            )?;
            manifest_entries.extend(write.manifest_entries);
            let creature = replacement
                .creatures
                .iter_mut()
                .find(|creature| creature.organism_id.raw() == raw)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            if creature.brain_class != self.brain_class {
                return Err(ScaffoldContractError::PhenotypeCompile.into());
            }
            creature.development_tick = checkpoint_tick;
            creature.mind.tick = checkpoint_tick;
            creature.mind.homeostasis = resident.homeostasis;
            creature.mind.sleep_state_label =
                gpu_sleep_state_label(resident.sleep_scheduler.state());
            creature.gpu_brain = Some(write.save_state);
        }
        if replacement.creatures.len() != self.handles.len() {
            return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
        }
        merge_gpu_checkpoint_manifest_entries(&mut replacement.assets, manifest_entries)?;
        replacement.validate_with_asset_root(store.root())?;
        Ok(replacement)
    }

    fn persist_sleep_checkpoint_boundary(&mut self) -> Result<(), GameAppShellError> {
        let Some(mut durability) = self.checkpoint_durability.take() else {
            return Ok(());
        };
        let result = (|| {
            let store = durability.store.clone();
            let replacement =
                self.capture_checkpointed_save(durability.published.save.clone(), &store)?;
            durability.publish(replacement)
        })();
        self.checkpoint_durability = Some(durability);
        result
    }

    fn promote_durable_completed_sleep(
        &mut self,
        organism_id: OrganismId,
        committed_sleep: SleepState,
    ) -> Result<(), GameAppShellError> {
        let Some(mut durability) = self.checkpoint_durability.take() else {
            return Ok(());
        };
        let result = (|| {
            let mut replacement = durability.published.save.clone();
            let creature = replacement
                .creatures
                .iter_mut()
                .find(|creature| creature.organism_id == organism_id)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let completed = creature
                .gpu_brain
                .as_ref()
                .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
            let promoted = completed.promoted_completed_sleep_state()?;
            if promoted.sleep != committed_sleep
                || promoted.checkpoint_tick != replacement.world.tick
            {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
            }
            creature.mind.sleep_state_label = gpu_sleep_state_label(committed_sleep);
            creature.gpu_brain = Some(promoted);
            durability.publish(replacement)
        })();
        self.checkpoint_durability = Some(durability);
        result
    }

    pub fn reconcile_population(&mut self) -> Result<(), GameAppShellError> {
        let live_ids = self
            .world
            .organism_entity_ids()
            .into_iter()
            .map(|(organism_id, _)| organism_id.raw())
            .collect::<BTreeSet<_>>();

        let retired = self
            .handles
            .keys()
            .copied()
            .filter(|raw| !live_ids.contains(raw))
            .collect::<Vec<_>>();
        for raw in retired {
            let handle = *self
                .handles
                .get(&raw)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            self.backend.remove_brain(handle)?;
            self.handles.remove(&raw);
            self.residents.remove(&raw);
            self.memories.remove(&raw);
            self.topologies.remove(&raw);
        }

        for raw in live_ids {
            if self.handles.contains_key(&raw) {
                if !self.residents.contains_key(&raw)
                    || !self.memories.contains_key(&raw)
                    || self
                        .topologies
                        .get(&raw)
                        .is_none_or(|sidecar| sidecar.organism_id().raw() != raw)
                {
                    return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
                }
                continue;
            }
            let organism_id = OrganismId(raw);
            let (phenotype, resident) = self.compile_birth(organism_id)?;
            let handle = self.backend.insert_brain(organism_id, phenotype)?;
            if handle.organism_id().raw() != raw {
                self.backend.remove_brain(handle)?;
                return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
            }
            self.handles.insert(raw, handle);
            self.residents.insert(raw, resident);
            self.memories.insert(
                raw,
                Self::new_memory_sidecar(organism_id, self.sensor_profile)?,
            );
            self.topologies.insert(
                raw,
                TopologySidecar::new_profiled(
                    organism_id,
                    SensorProfileIdentity {
                        profile_id: self.sensor_profile.into(),
                        profile_schema_version: 1,
                        sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
                    },
                    TopologicalMapConfig::default(),
                )?,
            );
        }
        Ok(())
    }

    fn new_memory_sidecar(
        organism_id: OrganismId,
        sensor_profile: SensorProfile,
    ) -> Result<MemorySidecarState, ScaffoldContractError> {
        MemorySidecarState::new_profiled(
            organism_id,
            SensorProfileIdentity {
                profile_id: sensor_profile.into(),
                profile_schema_version: 1,
                sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
            },
            MemoryBankConfig::new(
                LIVE_MEMORY_CAPACITY,
                LIVE_MEMORY_MAX_FEATURE_LEN,
                LIVE_MEMORY_MAX_MATCH_COUNT,
                LIVE_MEMORY_MIN_MATCH_SCORE,
                Confidence::new(0.0)?,
            )?,
        )
    }

    fn compact_memory_at_sleep_commit(
        &mut self,
        organism_id: OrganismId,
        committed_sleep: SleepState,
    ) -> Result<MemoryCompactionReceipt, GameAppShellError> {
        let cycle_id = match committed_sleep.consolidation {
            ConsolidationState::Committed { cycle_id, .. } if cycle_id != 0 => cycle_id,
            _ => return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into()),
        };
        let memory = self
            .memories
            .get_mut(&organism_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let prepared = memory.prepare_compaction(cycle_id, LIVE_MEMORY_CAPACITY as u32, 1)?;
        let receipt = memory.commit_compaction(prepared)?;
        self.last_memory_compaction_receipts.push(receipt);
        Ok(receipt)
    }

    fn retry_retained_learning(
        &mut self,
        organism_id: OrganismId,
        tick: Tick,
    ) -> Result<bool, GameAppShellError> {
        let raw = organism_id.raw();
        let Some(recovery) = self.retained_learning.get(&raw) else {
            return Ok(false);
        };
        let recovery_handle = recovery.handle;
        let recovery_pending = recovery.pending;
        let recovery_patch = recovery.sealed_patch.clone();
        let current_handle = self
            .handles
            .get(&raw)
            .copied()
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let pending_matches = match self.backend.pending_eligibility(current_handle) {
            Ok(pending) => current_handle == recovery_handle && pending == Some(recovery_pending),
            Err(error) => {
                return self.record_retained_retry_failure(
                    organism_id,
                    tick,
                    RetainedLearningErrorCode::from_error(&error),
                );
            }
        };
        let result = if pending_matches {
            self.backend
                .apply_sealed_outcome(current_handle, &recovery_patch)
        } else {
            Err(ScaffoldContractError::LearningEvidenceMismatch)
        };
        match result {
            Ok(receipt) => {
                self.retained_learning.remove(&raw);
                self.last_learning_receipts.push(receipt);
                Ok(false)
            }
            Err(error) => self.record_retained_retry_failure(
                organism_id,
                tick,
                RetainedLearningErrorCode::from_error(&error),
            ),
        }
    }

    fn record_retained_retry_failure(
        &mut self,
        organism_id: OrganismId,
        tick: Tick,
        error: RetainedLearningErrorCode,
    ) -> Result<bool, GameAppShellError> {
        let raw = organism_id.raw();
        let attempts = {
            let recovery = self
                .retained_learning
                .get_mut(&raw)
                .ok_or(ScaffoldContractError::LearningEvidenceMismatch)?;
            recovery.attempts = recovery.attempts.saturating_add(1);
            recovery.last_error = error;
            recovery.attempts
        };
        if attempts >= MAX_RETAINED_LEARNING_RETRIES {
            self.residents
                .get_mut(&raw)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                .sleep_scheduler
                .force_recovery_sleep(tick)?;
        }
        Ok(true)
    }

    pub fn tick(&mut self) -> Result<Vec<LiveBrainTickSummary>, GameAppShellError> {
        self.tick_with_sleep_progress(|backend, handle, organism_id, state, intent| {
            let mut driver = AuthoritativeGpuSleepDriver { backend, handle };
            driver.progress(organism_id, state, intent)
        })
    }

    pub fn tick_with_sleep_driver<D: GpuSleepConsolidationDriver>(
        &mut self,
        driver: &mut D,
    ) -> Result<Vec<LiveBrainTickSummary>, GameAppShellError> {
        self.tick_with_sleep_progress(|_, _, organism_id, state, intent| {
            driver.progress(organism_id, state, intent)
        })
    }

    fn tick_with_sleep_progress<F>(
        &mut self,
        mut progress: F,
    ) -> Result<Vec<LiveBrainTickSummary>, GameAppShellError>
    where
        F: FnMut(
            &mut GpuClosedLoopBackend,
            GpuBrainHandle,
            OrganismId,
            SleepState,
            Option<ConsolidationIntent>,
        ) -> SleepProgressResult,
    {
        self.reconcile_population()?;
        self.last_learning_receipts.clear();
        self.last_activity_work_receipts.clear();
        self.last_memory_recall_receipts.clear();
        self.last_memory_update_receipts.clear();
        self.last_memory_compaction_receipts.clear();
        self.last_memory_preparation_errors.clear();
        self.last_memory_observation_errors.clear();
        self.last_topology_observations.clear();
        self.last_eligibility_discard_receipts.clear();
        self.last_pre_seal_discard_failures.clear();
        self.last_post_seal_learning_failures.clear();
        if self.handles.is_empty() {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "GPU neural policy requires at least one live organism",
            });
        }

        let tick_before = self.world.tick();
        let tick_after = Tick::new(tick_before.raw().saturating_add(1));
        let mut batch = Vec::with_capacity(self.handles.len());
        let mut summaries_by_organism = BTreeMap::new();
        let mut persist_sleep_boundary = false;
        let mut completed_promotions = Vec::new();
        let scheduled_handles = self
            .handles
            .iter()
            .map(|(&raw, &handle)| (raw, handle))
            .collect::<Vec<_>>();
        for (raw, handle) in scheduled_handles {
            let retained_learning_pending =
                self.retry_retained_learning(OrganismId(raw), tick_before)?;
            let resident = self
                .residents
                .get_mut(&raw)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            Self::synchronize_resident_tick(resident, tick_before)?;
            let sleep_before = resident.sleep_scheduler.state();
            let phase_before = sleep_before.phase;
            self.backend.charge_world_brain_atp_tick(
                handle,
                tick_before.raw(),
                phase_before != SleepPhase::Awake,
            )?;
            let sleep_event = {
                let mut routed_driver = RoutedGpuSleepDriver {
                    backend: &mut self.backend,
                    handle,
                    progress: &mut progress,
                };
                resident.sleep_scheduler.scheduled_tick(
                    OrganismId(raw),
                    &resident.homeostasis,
                    HomeostaticParameters::reference(),
                    tick_before,
                    &mut routed_driver,
                )?
            };
            let sleep_after = resident.sleep_scheduler.state();
            if sleep_after != sleep_before {
                if matches!(
                    (sleep_before.consolidation, sleep_after.consolidation),
                    (
                        ConsolidationState::Completed { .. },
                        ConsolidationState::Committed { .. }
                    )
                ) {
                    completed_promotions.push((OrganismId(raw), sleep_after));
                } else {
                    persist_sleep_boundary = true;
                }
            }
            let remains_dispatchable = phase_before == SleepPhase::Awake
                && sleep_event.phase == SleepPhase::Awake
                && sleep_event.transition.is_none();
            if !remains_dispatchable || retained_learning_pending {
                if sleep_event.phase == SleepPhase::Awake {
                    Self::advance_failed_resident(resident, tick_after)?;
                } else {
                    Self::advance_sleeping_resident(resident, tick_after)?;
                }
                summaries_by_organism.insert(
                    raw,
                    if retained_learning_pending && sleep_event.phase == SleepPhase::Awake {
                        Self::retained_learning_summary(
                            OrganismId(raw),
                            tick_before,
                            tick_after,
                            self.sealed_patches.len(),
                        )
                    } else {
                        Self::sleeping_tick_summary(
                            OrganismId(raw),
                            tick_before,
                            tick_after,
                            self.sealed_patches.len(),
                        )
                    },
                );
                continue;
            }
            #[cfg(feature = "gpu-tests")]
            let force_preparation_failure = self.forced_memory_preparation_failures.remove(&raw);
            #[cfg(not(feature = "gpu-tests"))]
            let force_preparation_failure = false;
            let preparation = (|| -> Result<PreparedGpuBrainFrame, ScaffoldContractError> {
                if force_preparation_failure {
                    return Err(ScaffoldContractError::InvalidMemoryQuery);
                }
                let draft = self.world.perception_frame_draft(
                    OrganismId(raw),
                    tick_before,
                    self.sensor_profile,
                    resident.homeostasis,
                )?;
                let prepared_recall = self
                    .memories
                    .get(&raw)
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                    .recall_frame(&draft)?;
                let (frame, memory_recall) = prepared_recall.finalize(draft)?;
                memory_recall.validate_for_frame(&frame)?;
                let memory_upload =
                    self.backend
                        .prepare_memory_context_upload(handle, &frame, &memory_recall)?;
                Ok(PreparedGpuBrainFrame {
                    handle,
                    frame,
                    memory_recall,
                    memory_upload,
                })
            })();
            match preparation {
                Ok(prepared) => batch.push(prepared),
                Err(error) => {
                    self.last_memory_preparation_errors
                        .push((OrganismId(raw), error));
                    Self::advance_failed_resident(resident, tick_after)?;
                    summaries_by_organism.insert(
                        raw,
                        Self::preparation_failure_summary(
                            OrganismId(raw),
                            tick_before,
                            tick_after,
                            self.sealed_patches.len(),
                        ),
                    );
                }
            }
        }

        // The GPU selector has already committed, while the world is still at
        // the exact tick named by the durable Completed checkpoint. Publish
        // the manifest-side selector/ref promotion before any world action or
        // subsequent poll can occur.
        for (organism_id, committed_sleep) in completed_promotions {
            self.compact_memory_at_sleep_commit(organism_id, committed_sleep)?;
            self.promote_durable_completed_sleep(organism_id, committed_sleep)?;
        }

        let awake_summaries = if batch.is_empty() {
            self.record_gpu_tick_metrics(&[])?;
            Vec::new()
        } else {
            let memory_inputs = batch
                .iter()
                .map(|prepared| {
                    GpuClosedLoopMemoryTickInput::try_new(
                        prepared.handle,
                        &prepared.frame,
                        &prepared.memory_upload,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            let memory_batch = GpuClosedLoopMemoryBatchInput::try_new(memory_inputs)?;
            let gpu_ticks = self.backend.tick_memory_batch(&memory_batch)?;
            if gpu_ticks.len() != batch.len() {
                return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
            }
            self.record_gpu_tick_metrics(&gpu_ticks)?;
            let rows = batch.into_iter().zip(gpu_ticks).collect();
            self.process_selection_batch(rows)?
        };
        for summary in awake_summaries {
            summaries_by_organism.insert(summary.organism_id.raw(), summary);
        }
        if summaries_by_organism.len() != self.handles.len() {
            return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
        }
        self.world.advance_tick();
        if persist_sleep_boundary {
            self.persist_sleep_checkpoint_boundary()?;
        }
        Ok(summaries_by_organism.into_values().collect())
    }

    pub fn sealed_patches(&self) -> &[ExperiencePatch] {
        &self.sealed_patches
    }

    /// Engine-neutral world snapshot paired with an explicit GPU checkpoint.
    /// It contains no GPU handles or neural payloads.
    pub fn world_snapshot(&self) -> HeadlessWorld {
        self.world.clone()
    }

    /// Compact receipts from the most recently attempted world tick. Receipts
    /// contain generation and causal identity only, never weight payloads.
    pub fn last_learning_receipts(&self) -> &[GpuLearningReceipt] {
        &self.last_learning_receipts
    }

    /// Exact fixed-point neural work receipts from the most recent world tick.
    /// These are audit and persistence inputs only; they never influence world
    /// candidate enumeration or action legality.
    pub fn last_activity_work_receipts(&self) -> &[BrainWorkReceipt] {
        &self.last_activity_work_receipts
    }

    /// Candidate-conditioned recall receipts consumed by the most recent GPU
    /// dispatch. The records bind organism, bank generation, frame, and every
    /// candidate query without exposing memory payloads as host policy.
    pub fn last_memory_recall_receipts(&self) -> &[MemoryRecallReceipt] {
        &self.last_memory_recall_receipts
    }

    /// Successful post-learning observations from the most recent world tick.
    pub fn last_memory_update_receipts(&self) -> &[MemoryUpdateReceipt] {
        &self.last_memory_update_receipts
    }

    pub fn last_memory_compaction_receipts(&self) -> &[MemoryCompactionReceipt] {
        &self.last_memory_compaction_receipts
    }

    pub fn memory_compaction_checkpoint(
        &self,
        organism_id: OrganismId,
    ) -> Option<MemoryCompactionCheckpoint> {
        self.memories
            .get(&organism_id.raw())
            .map(|memory| *memory.compaction_checkpoint())
    }

    /// Typed per-organism recall/finalization/upload failures. Other prepared
    /// organisms remain eligible for the same world-tick GPU submission.
    pub fn last_memory_preparation_errors(&self) -> &[(OrganismId, ScaffoldContractError)] {
        &self.last_memory_preparation_errors
    }

    /// Typed post-seal memory failures. A failed sidecar update never rewrites
    /// the already measured world outcome or the committed GPU learning step.
    pub fn last_memory_observation_errors(&self) -> &[(OrganismId, ScaffoldContractError)] {
        &self.last_memory_observation_errors
    }

    /// Diagnostic-only topology dispositions from the most recent sealed
    /// transaction batch. These receipts are never uploaded to candidate
    /// memory, neural inputs, or arbitration.
    pub fn last_topology_observations(&self) -> &[TopologyObservationDisposition] {
        &self.last_topology_observations
    }

    pub fn retained_learning_recovery(
        &self,
        organism_id: OrganismId,
    ) -> Option<RetainedLearningRecoveryStatus> {
        self.retained_learning
            .get(&organism_id.raw())
            .map(|recovery| RetainedLearningRecoveryStatus {
                organism_id,
                sequence_id: recovery.sealed_patch.header().sequence_id,
                attempts: recovery.attempts,
                last_error: recovery.last_error,
            })
    }

    /// Compact receipts for pending eligibility transactions explicitly
    /// abandoned during the most recently attempted world tick.
    pub fn last_eligibility_discard_receipts(&self) -> &[PendingEligibilityDiscardReceipt] {
        &self.last_eligibility_discard_receipts
    }

    pub fn last_pre_seal_discard_failures(&self) -> &[PreSealDiscardFailure] {
        &self.last_pre_seal_discard_failures
    }

    pub fn last_post_seal_learning_failures(&self) -> &[PostSealLearningFailure] {
        &self.last_post_seal_learning_failures
    }

    pub(crate) const fn evidence_metrics(&self) -> GpuLiveBrainEvidenceMetrics {
        self.last_gpu_metrics
    }

    pub(crate) const fn hardware_receipt(&self) -> &alife_gpu_backend::GpuHardwareReceipt {
        self.backend.hardware_receipt()
    }

    pub fn authority_telemetry(&self) -> GpuBrainAuthorityTelemetry {
        let mut telemetry = GpuBrainAuthorityTelemetry::pending(
            self.brain_class
                .neuron_count()
                .map_or_else(|| "unknown".to_string(), |count| format!("N{count}")),
        );
        telemetry.authoritative = true;
        telemetry.adapter = self.backend.hardware_receipt().adapter_name.clone();
        telemetry.compact_readback_bytes = self.last_gpu_metrics.compact_readback_bytes;
        telemetry.sealed_patches = self.sealed_patches.len();
        telemetry.learning_updates =
            u32::try_from(self.last_learning_receipts.len()).unwrap_or(u32::MAX);
        telemetry.last_learning_delta = self
            .last_learning_receipts
            .iter()
            .map(|receipt| receipt.max_abs_delta)
            .fold(0.0_f32, f32::max);
        telemetry.active_ticks = u32::try_from(self.sealed_patches.len()).unwrap_or(u32::MAX);
        if let Some((&organism_raw, resident)) = self.residents.first_key_value() {
            telemetry.phenotype_hash_prefix =
                format!("{:08x}", resident.phenotype.phenotype_hash().0[0]);
            let live_sleep = resident.sleep_scheduler.state();
            telemetry.checkpoint_sleep_phase =
                gpu_sleep_phase_overlay_label(live_sleep.phase).to_string();
            telemetry.checkpoint_consolidation_state =
                gpu_consolidation_overlay_label(&live_sleep.consolidation).to_string();
            if let Some(saved) = self
                .checkpoint_durability
                .as_ref()
                .and_then(|durability| {
                    durability
                        .published
                        .save
                        .creatures
                        .iter()
                        .find(|creature| creature.organism_id.raw() == organism_raw)
                })
                .and_then(|creature| creature.gpu_brain.as_ref())
            {
                telemetry.checkpoint_tick = Some(saved.checkpoint_tick.raw());
                telemetry.checkpoint_sleep_phase =
                    gpu_sleep_phase_overlay_label(saved.sleep.phase).to_string();
                telemetry.checkpoint_consolidation_state =
                    gpu_consolidation_overlay_label(&saved.sleep.consolidation).to_string();
            }
        }
        if let Some(patch) = self.sealed_patches.last() {
            if let Ok(evidence) = patch.decision().neural_evidence() {
                telemetry.selected_candidate = Some(evidence.candidate_index);
                telemetry.selected_logit = Some(evidence.logit);
                telemetry.phenotype_hash_prefix = format!("{:08x}", evidence.phenotype_hash.0[0]);
            }
        }
        telemetry
    }

    fn compile_birth(
        &self,
        organism_id: OrganismId,
    ) -> Result<(alife_core::BrainPhenotype, ResidentCognition), GameAppShellError> {
        let (phenotype, genome, development) = compile_gpu_birth_components(
            self.deterministic_seed,
            self.brain_class,
            organism_id,
            self.world.tick(),
            self.sensor_profile,
        )?;
        let capacity = BrainCapacityClass::production_for_id(phenotype.brain_class_id())?;
        let compiler_inputs = PhenotypeCompilerInputs::try_new(
            genome.clone(),
            &capacity,
            development.clone(),
            self.sensor_profile,
        )?;
        if PhenotypeCompiler::compile_validated(&compiler_inputs, &capacity)? != phenotype {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        let resident = ResidentCognition {
            phenotype: phenotype.clone(),
            compiler_inputs,
            genome,
            development,
            homeostasis: HomeostaticSnapshot::baseline(self.world.tick()),
            sleep_scheduler: GpuSleepScheduler::new(SleepConsolidationConfig::reference())?,
            next_sequence: 1,
        };
        Ok((phenotype, resident))
    }

    fn synchronize_resident_tick(
        resident: &mut ResidentCognition,
        tick: Tick,
    ) -> Result<(), ScaffoldContractError> {
        if resident.homeostasis.tick != tick {
            resident.homeostasis = resident.homeostasis.advance(
                tick,
                HomeostaticDelta::zero(),
                HomeostaticParameters::reference(),
            )?;
            resident.development.age_ticks = tick;
        }
        Ok(())
    }

    fn advance_sleeping_resident(
        resident: &mut ResidentCognition,
        tick_after: Tick,
    ) -> Result<(), ScaffoldContractError> {
        resident.homeostasis = resident.homeostasis.advance(
            tick_after,
            HomeostaticDelta::sleep_recovery_per_tick(),
            HomeostaticParameters::reference(),
        )?;
        resident.development.age_ticks = tick_after;
        Ok(())
    }

    fn advance_failed_resident(
        resident: &mut ResidentCognition,
        tick_after: Tick,
    ) -> Result<(), ScaffoldContractError> {
        resident.homeostasis = resident.homeostasis.advance(
            tick_after,
            HomeostaticDelta::zero(),
            HomeostaticParameters::reference(),
        )?;
        resident.development.age_ticks = tick_after;
        Ok(())
    }

    fn preparation_failure_summary(
        organism_id: OrganismId,
        tick_before: Tick,
        tick_after: Tick,
        sealed_patch_count: usize,
    ) -> LiveBrainTickSummary {
        LiveBrainTickSummary {
            schema: G03_LIVE_BRAIN_LOOP_SCHEMA,
            schema_version: G03_LIVE_BRAIN_LOOP_SCHEMA_VERSION,
            organism_id,
            tick_before,
            tick_after,
            world_tick_before: tick_before,
            world_tick_after: tick_after,
            status: BrainTickStatus::TerminalInvalidState,
            selected_action_kind: None,
            selected_action_id: None,
            target_entity: None,
            patch_sealed: false,
            patch_sequence_id: None,
            patch_success: None,
            physical_contact: None,
            action_failure: None,
            sealed_patch_count,
            packed_record_count: 0,
            memory_updates: 0,
            topology_updates: 0,
            learning_updates: 0,
            invalid_or_rejected_action_count: 1,
            last_diagnostic: None,
            causal_stages: vec![
                LiveBrainCausalStage::GatherSensory,
                LiveBrainCausalStage::RecallMemory,
            ],
        }
    }

    fn retained_learning_summary(
        organism_id: OrganismId,
        tick_before: Tick,
        tick_after: Tick,
        sealed_patch_count: usize,
    ) -> LiveBrainTickSummary {
        LiveBrainTickSummary {
            schema: G03_LIVE_BRAIN_LOOP_SCHEMA,
            schema_version: G03_LIVE_BRAIN_LOOP_SCHEMA_VERSION,
            organism_id,
            tick_before,
            tick_after,
            world_tick_before: tick_before,
            world_tick_after: tick_after,
            status: BrainTickStatus::SafeIdle,
            selected_action_kind: None,
            selected_action_id: None,
            target_entity: None,
            patch_sealed: false,
            patch_sequence_id: None,
            patch_success: None,
            physical_contact: None,
            action_failure: None,
            sealed_patch_count,
            packed_record_count: 0,
            memory_updates: 0,
            topology_updates: 0,
            learning_updates: 0,
            invalid_or_rejected_action_count: 0,
            last_diagnostic: None,
            causal_stages: vec![LiveBrainCausalStage::ApplyLearning],
        }
    }

    fn sleeping_tick_summary(
        organism_id: OrganismId,
        tick_before: Tick,
        tick_after: Tick,
        sealed_patch_count: usize,
    ) -> LiveBrainTickSummary {
        LiveBrainTickSummary {
            schema: G03_LIVE_BRAIN_LOOP_SCHEMA,
            schema_version: G03_LIVE_BRAIN_LOOP_SCHEMA_VERSION,
            organism_id,
            tick_before,
            tick_after,
            world_tick_before: tick_before,
            world_tick_after: tick_after,
            status: BrainTickStatus::SafeIdle,
            selected_action_kind: None,
            selected_action_id: None,
            target_entity: None,
            patch_sealed: false,
            patch_sequence_id: None,
            patch_success: None,
            physical_contact: None,
            action_failure: None,
            sealed_patch_count,
            packed_record_count: 0,
            memory_updates: 0,
            topology_updates: 0,
            learning_updates: 0,
            invalid_or_rejected_action_count: 0,
            last_diagnostic: None,
            causal_stages: vec![
                LiveBrainCausalStage::EvaluateSleep,
                LiveBrainCausalStage::AdvanceSleep,
            ],
        }
    }

    fn record_gpu_tick_metrics(
        &mut self,
        gpu_ticks: &[GpuClosedLoopTick],
    ) -> Result<(), ScaffoldContractError> {
        self.last_activity_work_receipts
            .extend(gpu_ticks.iter().map(|tick| tick.work.clone()));
        let compact_readback_bytes = gpu_ticks
            .iter()
            .try_fold(0_usize, |total, tick| {
                total.checked_add(tick.compact_readback_bytes)
            })
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let selection_readback_bytes = gpu_ticks
            .len()
            .checked_mul(GPU_SELECTION_RECORD_BYTES)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let pending_eligibility_readback_bytes = 0;
        if selection_readback_bytes != compact_readback_bytes {
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        self.last_gpu_metrics = GpuLiveBrainEvidenceMetrics {
            completed_dispatch_count: self.backend.completed_dispatch_count(),
            completed_selection_count: self.backend.completed_selection_count(),
            selection_readback_bytes,
            pending_eligibility_readback_bytes,
            learning_readback_bytes: 0,
            compact_readback_bytes,
            active_tiles: gpu_ticks
                .iter()
                .map(|tick| tick.selection.active_tiles)
                .max()
                .unwrap_or(0),
            active_synapses: gpu_ticks
                .iter()
                .map(|tick| tick.selection.active_synapses)
                .max()
                .unwrap_or(0),
        };
        Ok(())
    }

    fn process_selection_batch(
        &mut self,
        rows: Vec<(PreparedGpuBrainFrame, GpuClosedLoopTick)>,
    ) -> Result<Vec<LiveBrainTickSummary>, GameAppShellError> {
        let pending = rows
            .iter()
            .map(|(prepared, gpu_tick)| (prepared.handle, *gpu_tick.pending_eligibility.identity()))
            .collect::<Vec<_>>();
        let mut prepared = Vec::with_capacity(rows.len());
        for (frame, gpu_tick) in rows {
            match self.prepare_selection(frame, gpu_tick) {
                Ok(selection) => prepared.push(selection),
                Err(error) => {
                    self.discard_pending_transactions(&pending);
                    return Err(error);
                }
            }
        }

        self.last_memory_recall_receipts.extend(
            prepared
                .iter()
                .map(|selection| selection.memory_recall.receipt().clone()),
        );

        let mut sealed = Vec::with_capacity(prepared.len());
        for (index, selection) in prepared.into_iter().enumerate() {
            match self.seal_prepared_selection(selection) {
                Ok(selection) => sealed.push(selection),
                Err(error) => {
                    if !sealed.is_empty() {
                        self.commit_sealed_batch(sealed)?;
                    }
                    self.discard_pending_transactions(&pending[index..]);
                    return Err(error);
                }
            }
        }
        self.commit_sealed_batch(sealed)
    }

    fn prepare_selection(
        &self,
        prepared: PreparedGpuBrainFrame,
        gpu_tick: GpuClosedLoopTick,
    ) -> Result<PreparedLiveSelection, GameAppShellError> {
        let PreparedGpuBrainFrame {
            handle,
            frame,
            memory_recall,
            memory_upload: _,
        } = prepared;
        let memory_binding = gpu_tick
            .memory_context_binding
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        if gpu_tick.handle != handle
            || gpu_tick.base_digest != frame.base_digest()
            || gpu_tick.frame_digest != frame.frame_digest()
            || gpu_tick.hardware_receipt_generation != self.backend.hardware_receipt().generation
            || memory_binding.slot != handle.slot()
            || memory_binding.slot_generation != handle.generation()
            || memory_binding.base_frame_digest != memory_recall.base_frame_digest()
            || memory_binding.context_digest != memory_recall.context_digest()
            || memory_binding.final_frame_digest != memory_recall.final_frame_digest()
            || usize::from(memory_binding.candidate_count) != frame.candidates().len()
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
        }
        memory_recall.validate_for_frame(&frame)?;
        let organism_id = handle.organism_id();
        let resident = self
            .residents
            .get(&organism_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let sequence_id = ExperienceSequenceId(resident.next_sequence);
        sequence_id.validate()?;
        let candidate = *frame
            .candidates()
            .get(usize::from(gpu_tick.selection.candidate_index))
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        let pending_identity = gpu_tick.pending_eligibility.identity();
        if pending_identity.handle_generation() != handle.generation()
            || pending_identity.phenotype_hash() != handle.phenotype_hash()
            || pending_identity.dispatch_generation() != gpu_tick.dispatch_generation
            || pending_identity.originating_tick() != frame.tick()
            || pending_identity.frame_digest() != frame.frame_digest()
            || pending_identity.active_activation_side() != gpu_tick.active_activation_side
            || pending_identity.candidate_index() != gpu_tick.selection.candidate_index
            || pending_identity.action_id() != candidate.action_id
            || pending_identity.action_family() != candidate.family
            || pending_identity.candidate_feature_digest() != candidate.feature_digest()?
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
        }
        let command = candidate.to_command(organism_id, gpu_tick.selection.confidence)?;
        let pre_action = PreActionSnapshot::from_neural_frame(
            sequence_id,
            handle.class_id(),
            handle.phenotype_hash(),
            resident.genome.id,
            resident.genome.schema_version,
            resident.development.clone(),
            frame.clone(),
        )?;
        let decision = DecisionSnapshot::from_neural_selection(
            sequence_id,
            handle.phenotype_hash(),
            gpu_tick.dispatch_generation,
            gpu_tick.active_activation_side,
            &frame,
            NeuralActionSelection {
                candidate_index: gpu_tick.selection.candidate_index,
                logit: gpu_tick.selection.logit,
                confidence: gpu_tick.selection.confidence,
                active_tiles: gpu_tick.selection.active_tiles,
                active_synapses: gpu_tick.selection.active_synapses,
            },
            command,
        )?
        .with_finalized_memory_recall(
            &frame,
            &memory_recall,
            gpu_tick.selection.candidate_index,
        )?;
        let outcome_tick = Tick::new(frame.tick().raw().saturating_add(1));
        Ok(PreparedLiveSelection {
            handle,
            pending_eligibility: gpu_tick.pending_eligibility,
            frame,
            memory_recall,
            sequence_id,
            outcome_tick,
            pre_action,
            decision,
        })
    }

    fn seal_prepared_selection(
        &mut self,
        prepared: PreparedLiveSelection,
    ) -> Result<SealedLiveSelection, GameAppShellError> {
        let PreparedLiveSelection {
            handle,
            pending_eligibility,
            frame,
            memory_recall: _,
            sequence_id,
            outcome_tick,
            pre_action,
            decision,
        } = prepared;
        let organism_id = handle.organism_id();
        let action_result = self.world.apply_command(&decision.selected_action)?;
        let mut outcome = PostActionOutcome::new(
            organism_id,
            sequence_id,
            outcome_tick,
            action_result.observation.success && action_result.execution.succeeded,
            action_result.execution.physical,
            action_result.observation.homeostatic_delta,
            action_result.observation.reward_valence,
            action_result.observation.frustration_delta,
            action_result.observation.pain_delta,
            action_result.observation.energy_delta,
            action_result.observation.prediction_error,
        )?;
        outcome.contradiction_observed =
            action_result.observation.contradiction_observed || !action_result.execution.succeeded;
        outcome.validate_contract()?;
        let patch = ExperiencePatchBuilder::new(sequence_id)
            .record_pre_action(pre_action)?
            .record_decision(decision.clone())?
            .record_outcome(outcome)?
            .seal()?;
        let resident = self
            .residents
            .get_mut(&organism_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        if resident.next_sequence != sequence_id.raw() {
            return Err(ScaffoldContractError::LearningEvidenceMismatch.into());
        }
        resident.homeostasis = resident.homeostasis.advance(
            outcome_tick,
            patch.outcome().homeostatic_delta,
            HomeostaticParameters::reference(),
        )?;
        resident.development.age_ticks = outcome_tick;
        resident.next_sequence = resident
            .next_sequence
            .checked_add(1)
            .ok_or(ScaffoldContractError::InvalidId)?;
        let summary = LiveBrainTickSummary {
            schema: G03_LIVE_BRAIN_LOOP_SCHEMA,
            schema_version: G03_LIVE_BRAIN_LOOP_SCHEMA_VERSION,
            organism_id,
            tick_before: frame.tick(),
            tick_after: outcome_tick,
            world_tick_before: frame.tick(),
            world_tick_after: outcome_tick,
            status: BrainTickStatus::Normal,
            selected_action_kind: Some(decision.selected_action.kind),
            selected_action_id: Some(decision.selected_action.action_id),
            target_entity: decision.selected_action.target_entity,
            patch_sealed: true,
            patch_sequence_id: Some(sequence_id.raw()),
            patch_success: Some(patch.outcome().success),
            physical_contact: Some(patch.outcome().physical.contact),
            action_failure: action_result.execution.failure,
            sealed_patch_count: self.sealed_patches.len().saturating_add(1),
            packed_record_count: 0,
            memory_updates: 0,
            topology_updates: 0,
            learning_updates: 0,
            invalid_or_rejected_action_count: u32::from(!action_result.execution.succeeded),
            last_diagnostic: None,
            causal_stages: vec![
                LiveBrainCausalStage::GatherSensory,
                LiveBrainCausalStage::RecallMemory,
                LiveBrainCausalStage::GpuBrainTick,
                LiveBrainCausalStage::ExecuteAction,
                LiveBrainCausalStage::MeasureOutcome,
                LiveBrainCausalStage::SealPatch,
            ],
        };
        Ok(SealedLiveSelection {
            handle,
            pending_eligibility,
            summary,
            patch,
        })
    }

    fn commit_sealed_batch(
        &mut self,
        mut sealed: Vec<SealedLiveSelection>,
    ) -> Result<Vec<LiveBrainTickSummary>, GameAppShellError> {
        if sealed.is_empty() {
            return Ok(Vec::new());
        }
        let learning_batch = sealed
            .iter()
            .map(|selection| (selection.handle, &selection.patch))
            .collect::<Vec<_>>();
        let learning = match self.backend.apply_sealed_outcome_batch(&learning_batch) {
            Ok(receipts) if receipts.len() == sealed.len() => Some(receipts),
            Ok(_) => {
                for selection in &sealed {
                    let organism_id = selection.handle.organism_id();
                    let pending_is_live = self
                        .backend
                        .pending_eligibility(selection.handle)
                        .ok()
                        .flatten()
                        == Some(selection.pending_eligibility);
                    let retained_for_recovery =
                        pending_is_live && !self.retained_learning.contains_key(&organism_id.raw());
                    if retained_for_recovery {
                        self.retained_learning.insert(
                            organism_id.raw(),
                            RetainedLearningRecovery {
                                handle: selection.handle,
                                pending: selection.pending_eligibility,
                                sealed_patch: selection.patch.clone(),
                                attempts: 0,
                                last_error: RetainedLearningErrorCode::NeuralBackendUnavailable,
                            },
                        );
                    }
                    self.last_post_seal_learning_failures
                        .push(PostSealLearningFailure {
                            organism_id,
                            sequence_id: selection.patch.header().sequence_id,
                            pending: selection.pending_eligibility,
                            error: RetainedLearningErrorCode::NeuralBackendUnavailable,
                            retained_for_recovery,
                        });
                }
                None
            }
            Err(error) => {
                let error_code = RetainedLearningErrorCode::from_error(&error);
                for selection in &sealed {
                    let organism_id = selection.handle.organism_id();
                    let retained_for_recovery =
                        !self.retained_learning.contains_key(&organism_id.raw());
                    if retained_for_recovery {
                        self.retained_learning.insert(
                            organism_id.raw(),
                            RetainedLearningRecovery {
                                handle: selection.handle,
                                pending: selection.pending_eligibility,
                                sealed_patch: selection.patch.clone(),
                                attempts: 0,
                                last_error: error_code,
                            },
                        );
                    }
                    self.last_post_seal_learning_failures
                        .push(PostSealLearningFailure {
                            organism_id,
                            sequence_id: selection.patch.header().sequence_id,
                            pending: selection.pending_eligibility,
                            error: error_code,
                            retained_for_recovery,
                        });
                }
                None
            }
        };
        if let Some(ref receipts) = learning {
            let learning_readback = receipts
                .len()
                .saturating_mul(GPU_FAST_PLASTICITY_COMMIT_BYTES);
            self.last_gpu_metrics.compact_readback_bytes = self
                .last_gpu_metrics
                .compact_readback_bytes
                .max(learning_readback);
            self.last_gpu_metrics.learning_readback_bytes = self
                .last_gpu_metrics
                .learning_readback_bytes
                .saturating_add(learning_readback);
        }

        let memory_updates = self.observe_sealed_memory(&sealed);
        let topology_updates = self.observe_sealed_topology(&sealed);

        let first_patch_count = self.sealed_patches.len();
        let mut summaries = Vec::with_capacity(sealed.len());
        for (index, selection) in sealed.iter_mut().enumerate() {
            selection.summary.sealed_patch_count = first_patch_count + index + 1;
            selection.summary.learning_updates = u32::from(learning.is_some());
            selection.summary.memory_updates = u32::from(memory_updates[index]);
            selection.summary.topology_updates = u32::from(topology_updates[index]);
            if learning.is_none() {
                selection.summary.status = BrainTickStatus::RecoverableActionFailure;
            }
            selection
                .summary
                .causal_stages
                .push(LiveBrainCausalStage::ApplyLearning);
            selection
                .summary
                .causal_stages
                .push(LiveBrainCausalStage::ObserveMemory);
            selection
                .summary
                .causal_stages
                .push(LiveBrainCausalStage::ObserveTopology);
            selection
                .summary
                .causal_stages
                .push(LiveBrainCausalStage::UpdateLogs);
            summaries.push(selection.summary.clone());
        }
        if let Some(learning) = learning {
            self.last_learning_receipts.extend(learning);
        }
        self.sealed_patches
            .extend(sealed.into_iter().map(|selection| selection.patch));
        Ok(summaries)
    }

    fn observe_sealed_memory(&mut self, sealed: &[SealedLiveSelection]) -> Vec<bool> {
        let mut memory_updates = Vec::with_capacity(sealed.len());
        for selection in sealed {
            let organism_id = selection.handle.organism_id();
            let observation = match self.memories.get_mut(&organism_id.raw()) {
                Some(memory) => memory.observe_sealed_patch(&selection.patch),
                None => Err(ScaffoldContractError::BrainOwnershipMismatch),
            };
            match observation {
                Ok(receipt) => {
                    self.last_memory_update_receipts.push(receipt);
                    memory_updates.push(true);
                }
                Err(error) => {
                    self.last_memory_observation_errors
                        .push((organism_id, error));
                    memory_updates.push(false);
                }
            }
        }
        memory_updates
    }

    fn observe_sealed_topology(&mut self, sealed: &[SealedLiveSelection]) -> Vec<bool> {
        let mut topology_updates = Vec::with_capacity(sealed.len());
        for selection in sealed {
            let organism_id = selection.handle.organism_id();
            let disposition = match self.topologies.get_mut(&organism_id.raw()) {
                Some(sidecar) if sidecar.organism_id() == organism_id => {
                    TopologyObservationDisposition::Observed(Box::new(
                        sidecar.observe_sealed_patch(&selection.patch),
                    ))
                }
                _ => TopologyObservationDisposition::RejectedMissingOwner { organism_id },
            };
            topology_updates.push(disposition.was_observed());
            self.last_topology_observations.push(disposition);
        }
        topology_updates
    }

    fn discard_pending_transactions(
        &mut self,
        pending: &[(GpuBrainHandle, PendingEligibilityIdentity)],
    ) {
        for (handle, identity) in pending {
            match self.backend.discard_pending_eligibility(*handle, identity) {
                Ok(receipt) => self.last_eligibility_discard_receipts.push(receipt),
                Err(error) => self
                    .last_pre_seal_discard_failures
                    .push(PreSealDiscardFailure {
                        organism_id: handle.organism_id(),
                        identity: *identity,
                        error: RetainedLearningErrorCode::from_error(&error),
                    }),
            }
        }
    }

    #[cfg(feature = "gpu-tests")]
    fn evidence_handle(
        &self,
        organism_id: OrganismId,
    ) -> Result<GpuBrainHandle, ScaffoldContractError> {
        self.handles
            .get(&organism_id.raw())
            .copied()
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)
    }

    #[cfg(feature = "gpu-tests")]
    pub(crate) fn evidence_world_tick(&self) -> Tick {
        self.world.tick()
    }

    pub(crate) fn evidence_completed_dispatch_count(&self) -> u64 {
        self.backend.completed_dispatch_count()
    }

    pub(crate) fn evidence_sleep_state(
        &self,
        organism_id: OrganismId,
    ) -> Result<SleepState, ScaffoldContractError> {
        self.residents
            .get(&organism_id.raw())
            .map(|resident| resident.sleep_scheduler.state())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)
    }

    pub(crate) fn evidence_set_homeostasis(
        &mut self,
        organism_id: OrganismId,
        homeostasis: HomeostaticSnapshot,
    ) -> Result<(), ScaffoldContractError> {
        homeostasis.validate_contract()?;
        if homeostasis.tick != self.world.tick() {
            return Err(ScaffoldContractError::NonMonotonicTick);
        }
        self.residents
            .get_mut(&organism_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
            .homeostasis = homeostasis;
        Ok(())
    }

    /// Offline acceptance access to the exact world owned by this runtime.
    /// Production gameplay never routes neural scores through this boundary.
    pub(crate) const fn evidence_world(&self) -> &HeadlessWorld {
        &self.world
    }

    /// Offline challenge-world mutation between sealed ticks. This cannot
    /// mutate candidates or outcomes during an active neural transaction.
    pub(crate) fn evidence_world_mut(&mut self) -> &mut HeadlessWorld {
        &mut self.world
    }

    pub(crate) fn evidence_memory_sidecar(
        &self,
        organism_id: OrganismId,
    ) -> Option<&MemorySidecarState> {
        self.memories.get(&organism_id.raw())
    }

    pub(crate) fn evidence_topology_sidecar(
        &self,
        organism_id: OrganismId,
    ) -> Option<&TopologySidecar> {
        self.topologies.get(&organism_id.raw())
    }

    #[cfg(feature = "gpu-tests")]
    pub fn world_tick_for_test(&self) -> Tick {
        self.evidence_world_tick()
    }

    #[cfg(feature = "gpu-tests")]
    pub fn completed_dispatch_count_for_test(&self) -> u64 {
        self.evidence_completed_dispatch_count()
    }

    #[cfg(feature = "gpu-tests")]
    pub fn sleep_state_for_test(
        &self,
        organism_id: OrganismId,
    ) -> Result<SleepState, ScaffoldContractError> {
        self.evidence_sleep_state(organism_id)
    }

    #[cfg(feature = "gpu-tests")]
    pub fn set_homeostasis_for_test(
        &mut self,
        organism_id: OrganismId,
        homeostasis: HomeostaticSnapshot,
    ) -> Result<(), ScaffoldContractError> {
        self.evidence_set_homeostasis(organism_id, homeostasis)
    }

    #[cfg(feature = "gpu-tests")]
    pub fn homeostasis_for_test(
        &self,
        organism_id: OrganismId,
    ) -> Result<HomeostaticSnapshot, ScaffoldContractError> {
        self.residents
            .get(&organism_id.raw())
            .map(|resident| resident.homeostasis)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)
    }

    #[cfg(feature = "gpu-tests")]
    pub fn learning_state_for_test(
        &mut self,
        organism_id: OrganismId,
    ) -> Result<alife_gpu_backend::GpuLearningStateSnapshot, ScaffoldContractError> {
        let handle = self.evidence_handle(organism_id)?;
        self.backend.learning_state_snapshot_for_test(handle)
    }

    #[cfg(feature = "gpu-tests")]
    pub fn active_fast_weights_for_test(
        &mut self,
        organism_id: OrganismId,
    ) -> Result<Vec<f32>, ScaffoldContractError> {
        let handle = self.evidence_handle(organism_id)?;
        self.backend.read_active_fast_weights_for_test(handle)
    }

    #[cfg(feature = "gpu-tests")]
    pub fn active_lifetime_weights_for_test(
        &mut self,
        organism_id: OrganismId,
    ) -> Result<Vec<f32>, ScaffoldContractError> {
        let handle = self.evidence_handle(organism_id)?;
        self.backend.read_active_lifetime_weights_for_test(handle)
    }

    #[cfg(feature = "gpu-tests")]
    pub fn sleep_replay_for_test(
        &mut self,
        organism_id: OrganismId,
    ) -> Result<alife_core::BoundedReplayBatch, ScaffoldContractError> {
        let handle = self.evidence_handle(organism_id)?;
        self.backend.build_sleep_replay_batch(handle)
    }

    #[cfg(feature = "gpu-tests")]
    pub fn force_memory_preparation_failure_for_test(&mut self, organism_id: OrganismId) {
        self.forced_memory_preparation_failures
            .insert(organism_id.raw());
    }

    #[cfg(feature = "gpu-tests")]
    pub fn memory_sidecar_for_test(&self, organism_id: OrganismId) -> Option<&MemorySidecarState> {
        self.evidence_memory_sidecar(organism_id)
    }

    #[cfg(feature = "gpu-tests")]
    pub fn topology_sidecar_for_test(&self, organism_id: OrganismId) -> Option<&TopologySidecar> {
        self.evidence_topology_sidecar(organism_id)
    }

    #[cfg(feature = "gpu-tests")]
    pub fn force_learning_rejections_for_test(&mut self, rejection_count: u8) {
        self.backend
            .force_learning_rejections_for_test(rejection_count);
    }

    #[cfg(test)]
    pub(crate) fn handle_for(&self, organism_id: OrganismId) -> Option<GpuBrainHandle> {
        self.handles.get(&organism_id.raw()).copied()
    }

    #[cfg(test)]
    pub(crate) fn world_mut(&mut self) -> &mut HeadlessWorld {
        &mut self.world
    }

    #[cfg(test)]
    pub(crate) fn test_tick_retired_handle(
        &mut self,
        handle: GpuBrainHandle,
        frame: PerceptionFrame,
    ) -> Result<Vec<GpuClosedLoopTick>, ScaffoldContractError> {
        self.backend.tick_batch(&[(handle, frame)])
    }
}

fn gpu_sleep_state_label(state: SleepState) -> String {
    format!(
        "gpu:{:?}:consolidation-{}:cycle-{}",
        state.phase,
        state.consolidation.kind_raw(),
        if state.active_cycle_id == 0 {
            state.last_consolidated_cycle_id
        } else {
            state.active_cycle_id
        }
    )
}

const fn gpu_sleep_phase_overlay_label(phase: SleepPhase) -> &'static str {
    match phase {
        SleepPhase::Awake => "Awake",
        SleepPhase::EnteringSleep => "Entering sleep",
        SleepPhase::Consolidating => "Consolidating",
        SleepPhase::Waking => "Waking",
        SleepPhase::ForcedRecoverySleep => "Forced recovery sleep",
    }
}

const fn gpu_consolidation_overlay_label(state: &ConsolidationState) -> &'static str {
    match state {
        ConsolidationState::None => "None",
        ConsolidationState::Pending { .. } => "Pending",
        ConsolidationState::Prepared { .. } => "Prepared",
        ConsolidationState::Submitted { .. } => "Submitted",
        ConsolidationState::Completed { .. } => "Completed",
        ConsolidationState::Committed { .. } => "Committed",
    }
}

pub(crate) fn compile_gpu_birth_components(
    deterministic_seed: u64,
    brain_class: BrainScaleTier,
    organism_id: OrganismId,
    tick: Tick,
    sensor_profile: SensorProfile,
) -> Result<(alife_core::BrainPhenotype, BrainGenome, DevelopmentState), ScaffoldContractError> {
    if deterministic_seed == 0 {
        return Err(ScaffoldContractError::PhenotypeCompile);
    }
    organism_id.validate()?;
    let capacity = BrainCapacityClass::production_for_id(brain_class.default_class_id())?;
    let birth_seed = deterministic_seed ^ organism_id.raw().rotate_left(17);
    let genome = BrainGenome::scaffold(birth_seed, capacity.id());
    let development = DevelopmentState::new(genome.id, tick, NormalizedScalar::new(0.35)?);
    let phenotype = PhenotypeCompiler::compile(&genome, &capacity, &development, sensor_profile)?;
    Ok((phenotype, genome, development))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GpuSleepConsolidationDriver;
    use alife_core::{
        ActionTarget, CandidateActionFamily, OutcomeCreditPacket, PreActionBrainEvidence, Vec3f,
        WorldEntityId,
    };
    use alife_world::HeadlessScenarioBuilder;

    struct NoProgressSleepDriver;

    impl GpuSleepConsolidationDriver for NoProgressSleepDriver {
        fn progress(
            &mut self,
            _organism_id: OrganismId,
            _state: alife_core::SleepState,
            _intent: Option<alife_core::ConsolidationIntent>,
        ) -> Result<Option<alife_core::ConsolidationDriverEvent>, alife_core::ScaffoldContractError>
        {
            Ok(None)
        }
    }

    #[derive(Default)]
    struct CompletingSleepDriver {
        intents: Vec<alife_core::ConsolidationIntent>,
    }

    impl GpuSleepConsolidationDriver for CompletingSleepDriver {
        fn progress(
            &mut self,
            _organism_id: OrganismId,
            state: alife_core::SleepState,
            intent: Option<alife_core::ConsolidationIntent>,
        ) -> Result<Option<alife_core::ConsolidationDriverEvent>, alife_core::ScaffoldContractError>
        {
            if let Some(intent) = intent {
                self.intents.push(intent);
                return Ok(Some(
                    alife_core::ConsolidationDriverEvent::ReplayAssetPersisted {
                        intent,
                        replay_digest: [11, 12, 13, 14],
                        replay_event_count: 1,
                        replay_eligibility_sample_count: 1,
                    },
                ));
            }
            let event = match state.consolidation {
                alife_core::ConsolidationState::Pending {
                    intent,
                    replay_digest,
                    replay_event_count,
                    replay_eligibility_sample_count,
                } => {
                    let mut request = alife_core::GpuConsolidationRequest {
                        schema_version: alife_core::GPU_CONSOLIDATION_REQUEST_SCHEMA_VERSION,
                        request_flags: 0,
                        cycle_id: intent.cycle_id,
                        phenotype_hash: alife_core::PhenotypeHash([21, 22, 23, 24]),
                        input_generation: 1,
                        expected_output_generation: 2,
                        input_digest: [31, 32, 33, 34],
                        replay_digest,
                        max_replay_events: replay_event_count.max(1),
                        max_replay_eligibility_samples: replay_eligibility_sample_count.max(1),
                        request_digest: [0; 4],
                    };
                    request.request_digest = request.recompute_request_digest()?;
                    alife_core::ConsolidationDriverEvent::Prepared { request }
                }
                alife_core::ConsolidationState::Prepared { request } => {
                    alife_core::ConsolidationDriverEvent::Submitted {
                        request,
                        job_id: alife_core::ConsolidationJobId::try_from_raw(1)?,
                    }
                }
                alife_core::ConsolidationState::Submitted { request, job_id } => {
                    let mut staged = alife_core::ConsolidationStagedOutput {
                        job_id,
                        output_generation: request.expected_output_generation,
                        output_weight_bank: 1,
                        output_digest: [41, 42, 43, 44],
                        eligibility_reset_generation: 2,
                        output_eligibility_bank: 0,
                        eligibility_output_digest: [51, 52, 53, 54],
                        replay_journal_generation: 2,
                        replay_journal_cursor: 0,
                        replay_journal_event_count: 0,
                        replay_journal_output_digest: [61, 62, 63, 64],
                        staging_digest: [0; 4],
                        promoted_fast_l1_bits: 0.25_f32.to_bits(),
                        replay_induced_fast_l1_bits: 0.125_f32.to_bits(),
                    };
                    staged.staging_digest = staged.recompute_staging_digest(&request, 1, 1)?;
                    alife_core::ConsolidationDriverEvent::Completed { request, staged }
                }
                alife_core::ConsolidationState::Completed { request, staged } => {
                    alife_core::ConsolidationDriverEvent::Committed {
                        cycle_id: request.cycle_id,
                        output_generation: staged.output_generation,
                        output_digest: staged.output_digest,
                    }
                }
                _ => return Ok(None),
            };
            Ok(Some(event))
        }
    }

    #[test]
    fn live_runtime_charges_one_exact_basal_debit_before_each_neural_dispatch() {
        let backend = GpuClosedLoopBackend::new_required(
            alife_gpu_backend::GpuRuntimeProfile::production_v1(),
        )
        .expect("required GPU");
        let world = HeadlessScenarioBuilder::new(9_311)
            .agent("one", OrganismId(1), Vec3f::ZERO)
            .build()
            .unwrap();
        let mut runtime =
            GpuLiveBrainRuntime::new(backend, world, 9_311, BrainScaleTier::Nano512).unwrap();

        runtime.tick().unwrap();
        let first = runtime.last_activity_work_receipts()[0].clone();
        assert_eq!(
            first.atp_before_q16,
            alife_core::BRAIN_ATP_Q16_MAX - alife_core::BRAIN_ATP_BASAL_DEBIT_Q16
        );
        let handle = runtime.handle_for(OrganismId(1)).unwrap();
        assert_eq!(
            runtime.backend.brain_atp_q16(handle).unwrap(),
            first.atp_after_q16
        );

        runtime.tick().unwrap();
        let second = runtime.last_activity_work_receipts()[0].clone();
        assert_eq!(
            second.atp_before_q16,
            first
                .atp_after_q16
                .saturating_sub(alife_core::BRAIN_ATP_BASAL_DEBIT_Q16)
        );
        assert_eq!(
            runtime.backend.brain_atp_q16(handle).unwrap(),
            second.atp_after_q16
        );
    }

    #[test]
    fn organism_despawn_retires_its_gpu_handle_before_slot_reuse() {
        let backend = GpuClosedLoopBackend::new_required(
            alife_gpu_backend::GpuRuntimeProfile::production_v1(),
        )
        .expect("required GPU");
        let world = HeadlessScenarioBuilder::new(91)
            .agent("one", OrganismId(1), Vec3f::ZERO)
            .agent("two", OrganismId(2), Vec3f::new(2.0, 0.0, 0.0))
            .build()
            .unwrap();
        let mut runtime =
            GpuLiveBrainRuntime::new(backend, world, 91, BrainScaleTier::Nano512).unwrap();
        let retired = runtime.handle_for(OrganismId(1)).unwrap();
        let retired_frame = runtime
            .world
            .perception_frame(
                OrganismId(1),
                Tick::ZERO,
                SensorProfile::PrivilegedAffordanceV1,
                HomeostaticSnapshot::baseline(Tick::ZERO),
            )
            .unwrap();
        runtime.world_mut().remove_organism(OrganismId(1)).unwrap();
        runtime.reconcile_population().unwrap();

        assert!(runtime.handle_for(OrganismId(1)).is_none());
        assert!(runtime
            .test_tick_retired_handle(retired, retired_frame)
            .is_err());
    }

    #[test]
    fn fatigued_runtime_enters_sleep_before_gpu_dispatch_and_emits_no_action() {
        let backend = GpuClosedLoopBackend::new_required(
            alife_gpu_backend::GpuRuntimeProfile::production_v1(),
        )
        .expect("required GPU");
        let world = HeadlessScenarioBuilder::new(90)
            .agent("sleeper", OrganismId(1), Vec3f::ZERO)
            .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.8)
            .build()
            .unwrap();
        let mut runtime =
            GpuLiveBrainRuntime::new(backend, world, 90, BrainScaleTier::Nano512).unwrap();
        let resident = runtime.residents.get_mut(&1).unwrap();
        let mut drives = alife_core::DriveSnapshot::baseline();
        drives.fatigue = 0.99;
        let mut hormones = alife_core::EndocrineSnapshot::baseline();
        hormones.sleep_pressure = 0.99;
        resident.homeostasis = HomeostaticSnapshot::new(Tick::ZERO, drives, hormones).unwrap();
        let mut driver = NoProgressSleepDriver;

        let summaries = runtime.tick_with_sleep_driver(&mut driver).unwrap();

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].status, BrainTickStatus::SafeIdle);
        assert_eq!(summaries[0].selected_action_id, None);
        assert!(!summaries[0].patch_sealed);
        assert_eq!(runtime.backend.completed_dispatch_count(), 0);
        assert_eq!(runtime.world.tick(), Tick::new(1));
        assert_eq!(
            runtime
                .residents
                .get(&1)
                .unwrap()
                .sleep_scheduler
                .state()
                .phase,
            alife_core::SleepPhase::EnteringSleep
        );
    }

    #[test]
    fn mixed_sleeping_and_awake_residents_dispatch_only_the_awake_brain() {
        let backend = GpuClosedLoopBackend::new_required(
            alife_gpu_backend::GpuRuntimeProfile::production_v1(),
        )
        .expect("required GPU");
        let world = HeadlessScenarioBuilder::new(901)
            .agent("sleeper", OrganismId(1), Vec3f::ZERO)
            .agent("awake", OrganismId(2), Vec3f::new(2.0, 0.0, 0.0))
            .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.8)
            .build()
            .unwrap();
        let mut runtime =
            GpuLiveBrainRuntime::new(backend, world, 901, BrainScaleTier::Nano512).unwrap();
        let resident = runtime.residents.get_mut(&1).unwrap();
        let mut drives = alife_core::DriveSnapshot::baseline();
        drives.fatigue = 0.99;
        let mut hormones = alife_core::EndocrineSnapshot::baseline();
        hormones.sleep_pressure = 0.99;
        resident.homeostasis = HomeostaticSnapshot::new(Tick::ZERO, drives, hormones).unwrap();
        let mut driver = NoProgressSleepDriver;

        let summaries = runtime.tick_with_sleep_driver(&mut driver).unwrap();

        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].organism_id, OrganismId(1));
        assert_eq!(summaries[0].status, BrainTickStatus::SafeIdle);
        assert_eq!(summaries[1].organism_id, OrganismId(2));
        assert_eq!(summaries[1].status, BrainTickStatus::Normal);
        assert_eq!(runtime.backend.completed_dispatch_count(), 1);
        assert_eq!(runtime.sealed_patches().len(), 1);
        assert_eq!(runtime.world.tick(), Tick::new(1));
        assert_eq!(
            runtime.evidence_metrics().selection_readback_bytes,
            GPU_SELECTION_RECORD_BYTES
        );
    }

    #[test]
    fn completed_sleep_cycle_wakes_once_and_dispatch_resumes_next_tick() {
        let backend = GpuClosedLoopBackend::new_required(
            alife_gpu_backend::GpuRuntimeProfile::production_v1(),
        )
        .expect("required GPU");
        let world = HeadlessScenarioBuilder::new(902)
            .agent("sleeper", OrganismId(1), Vec3f::ZERO)
            .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.8)
            .build()
            .unwrap();
        let mut runtime =
            GpuLiveBrainRuntime::new(backend, world, 902, BrainScaleTier::Nano512).unwrap();
        let resident = runtime.residents.get_mut(&1).unwrap();
        let mut drives = alife_core::DriveSnapshot::baseline();
        drives.fatigue = 0.99;
        let mut hormones = alife_core::EndocrineSnapshot::baseline();
        hormones.sleep_pressure = 0.99;
        resident.homeostasis = HomeostaticSnapshot::new(Tick::ZERO, drives, hormones).unwrap();
        let mut driver = CompletingSleepDriver::default();

        let mut woke = false;
        for _ in 0..32 {
            let summaries = runtime.tick_with_sleep_driver(&mut driver).unwrap();
            assert_eq!(summaries.len(), 1);
            assert_eq!(summaries[0].status, BrainTickStatus::SafeIdle);
            assert_eq!(summaries[0].selected_action_id, None);
            assert!(!summaries[0].patch_sealed);
            assert_eq!(runtime.backend.completed_dispatch_count(), 0);
            let state = runtime.residents.get(&1).unwrap().sleep_scheduler.state();
            if state.phase == alife_core::SleepPhase::Awake && state.last_consolidated_cycle_id == 1
            {
                woke = true;
                break;
            }
        }

        assert!(woke);
        assert_eq!(driver.intents.len(), 1);

        let summaries = runtime.tick_with_sleep_driver(&mut driver).unwrap();

        assert_eq!(summaries[0].status, BrainTickStatus::Normal);
        assert!(summaries[0].patch_sealed);
        assert_eq!(runtime.backend.completed_dispatch_count(), 1);
        assert_eq!(driver.intents.len(), 1);
    }

    #[test]
    fn gpu_tick_executes_and_seals_neural_evidence_before_world_advance() {
        let backend = GpuClosedLoopBackend::new_required(
            alife_gpu_backend::GpuRuntimeProfile::production_v1(),
        )
        .expect("required GPU");
        let world = HeadlessScenarioBuilder::new(92)
            .agent("agent", OrganismId(1), Vec3f::ZERO)
            .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.8)
            .build()
            .unwrap();
        let mut runtime =
            GpuLiveBrainRuntime::new(backend, world, 92, BrainScaleTier::Nano512).unwrap();
        let handle = runtime.handle_for(OrganismId(1)).unwrap();

        let summaries = runtime.tick().unwrap();

        assert_eq!(summaries.len(), 1);
        assert!(summaries[0].patch_sealed);
        assert_eq!(runtime.backend.completed_dispatch_count(), 1);
        assert_eq!(runtime.world.tick(), Tick::new(1));
        assert_eq!(runtime.sealed_patches().len(), 1);
        assert_eq!(runtime.backend.pending_eligibility(handle).unwrap(), None);
        let metrics = runtime.evidence_metrics();
        assert_eq!(metrics.selection_readback_bytes, GPU_SELECTION_RECORD_BYTES);
        assert_eq!(metrics.pending_eligibility_readback_bytes, 0);
        assert_eq!(
            metrics.learning_readback_bytes,
            GPU_FAST_PLASTICITY_COMMIT_BYTES
        );
        assert_eq!(
            metrics.compact_readback_bytes,
            metrics
                .selection_readback_bytes
                .max(metrics.learning_readback_bytes)
        );
        assert!(matches!(
            runtime.sealed_patches()[0].pre_action().brain_evidence,
            PreActionBrainEvidence::NeuralClosedLoopGpu { .. }
        ));
    }

    #[test]
    fn failed_sealing_discards_the_exact_pending_eligibility_and_next_tick_recovers() {
        let backend = GpuClosedLoopBackend::new_required(
            alife_gpu_backend::GpuRuntimeProfile::production_v1(),
        )
        .expect("required GPU");
        let world = HeadlessScenarioBuilder::new(93)
            .agent("agent", OrganismId(1), Vec3f::ZERO)
            .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.8)
            .build()
            .unwrap();
        let mut runtime =
            GpuLiveBrainRuntime::new(backend, world, 93, BrainScaleTier::Nano512).unwrap();
        let handle = runtime.handle_for(OrganismId(1)).unwrap();
        runtime.residents.get_mut(&1).unwrap().next_sequence = 0;

        assert!(runtime.tick().is_err());
        assert_eq!(runtime.backend.pending_eligibility(handle).unwrap(), None);
        assert!(runtime.sealed_patches().is_empty());
        assert!(runtime.last_learning_receipts().is_empty());
        assert_eq!(runtime.last_eligibility_discard_receipts().len(), 1);

        runtime.residents.get_mut(&1).unwrap().next_sequence = 1;
        let summaries = runtime.tick().unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].learning_updates, 1);
        assert_eq!(runtime.backend.pending_eligibility(handle).unwrap(), None);
    }

    #[cfg(feature = "gpu-tests")]
    #[test]
    fn failed_pre_seal_discard_is_typed_and_leaves_pending_credit_intact() {
        let backend = GpuClosedLoopBackend::new_required(
            alife_gpu_backend::GpuRuntimeProfile::production_v1(),
        )
        .expect("required GPU");
        let world = HeadlessScenarioBuilder::new(9_306)
            .agent("agent", OrganismId(1), Vec3f::ZERO)
            .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.8)
            .build()
            .unwrap();
        let mut runtime =
            GpuLiveBrainRuntime::new(backend, world, 9_306, BrainScaleTier::Nano512).unwrap();
        let handle = runtime.handle_for(OrganismId(1)).unwrap();
        runtime.residents.get_mut(&1).unwrap().next_sequence = 0;
        runtime.backend.force_discard_rejections_for_test(1);

        assert!(runtime.tick().is_err());
        let pending = runtime
            .backend
            .pending_eligibility(handle)
            .unwrap()
            .expect("failed discard preserves pending eligibility");
        assert!(runtime.last_eligibility_discard_receipts().is_empty());
        let failures = runtime.last_pre_seal_discard_failures();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].organism_id, OrganismId(1));
        assert_eq!(failures[0].identity, *pending.identity());
        assert_eq!(
            failures[0].error,
            RetainedLearningErrorCode::LearningEvidenceMismatch
        );
    }

    #[test]
    fn tampered_selected_candidate_is_rejected_before_world_execution() {
        let backend = GpuClosedLoopBackend::new_required(
            alife_gpu_backend::GpuRuntimeProfile::production_v1(),
        )
        .expect("required GPU");
        let organism_id = OrganismId(1);
        let world = HeadlessScenarioBuilder::new(9_307)
            .agent("agent", organism_id, Vec3f::ZERO)
            .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.8)
            .build()
            .unwrap();
        let mut runtime =
            GpuLiveBrainRuntime::new(backend, world, 9_307, BrainScaleTier::Nano512).unwrap();
        let handle = runtime.handle_for(organism_id).unwrap();
        let draft = runtime
            .world
            .perception_frame_draft(
                organism_id,
                Tick::ZERO,
                SensorProfile::PrivilegedAffordanceV1,
                runtime.residents[&1].homeostasis,
            )
            .unwrap();
        let recall = runtime.memories[&1].recall_frame(&draft).unwrap();
        let (frame, memory_recall) = recall.finalize(draft).unwrap();
        let memory_upload = runtime
            .backend
            .prepare_memory_context_upload(handle, &frame, &memory_recall)
            .unwrap();
        let input = GpuClosedLoopMemoryTickInput::try_new(handle, &frame, &memory_upload).unwrap();
        let batch = GpuClosedLoopMemoryBatchInput::try_new(vec![input]).unwrap();
        let mut gpu_tick = runtime.backend.tick_memory_batch(&batch).unwrap().remove(0);
        gpu_tick.selection.candidate_index =
            (gpu_tick.selection.candidate_index + 1) % frame.candidates().len() as u16;

        let result = runtime.process_selection_batch(vec![(
            PreparedGpuBrainFrame {
                handle,
                frame,
                memory_recall,
                memory_upload,
            },
            gpu_tick,
        )]);

        assert!(result.is_err());
        assert_eq!(runtime.world.tick(), Tick::ZERO);
        assert!(runtime.sealed_patches().is_empty());
        assert_eq!(runtime.backend.pending_eligibility(handle).unwrap(), None);
        assert_eq!(runtime.last_eligibility_discard_receipts().len(), 1);
    }

    #[test]
    fn failed_batch_sealing_clears_every_abandoned_pending_transaction() {
        let backend = GpuClosedLoopBackend::new_required(
            alife_gpu_backend::GpuRuntimeProfile::production_v1(),
        )
        .expect("required GPU");
        let world = HeadlessScenarioBuilder::new(94)
            .agent("one", OrganismId(1), Vec3f::ZERO)
            .agent("two", OrganismId(2), Vec3f::new(2.0, 0.0, 0.0))
            .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.8)
            .build()
            .unwrap();
        let mut runtime =
            GpuLiveBrainRuntime::new(backend, world, 94, BrainScaleTier::Nano512).unwrap();
        let one = runtime.handle_for(OrganismId(1)).unwrap();
        let two = runtime.handle_for(OrganismId(2)).unwrap();
        runtime.residents.get_mut(&1).unwrap().next_sequence = 0;

        assert!(runtime.tick().is_err());
        assert_eq!(runtime.backend.pending_eligibility(one).unwrap(), None);
        assert_eq!(runtime.backend.pending_eligibility(two).unwrap(), None);
        assert_eq!(runtime.last_eligibility_discard_receipts().len(), 2);

        runtime.residents.get_mut(&1).unwrap().next_sequence = 1;
        let summaries = runtime.tick().unwrap();
        assert_eq!(summaries.len(), 2);
        assert!(summaries
            .iter()
            .all(|summary| summary.learning_updates == 1));
    }

    #[test]
    fn world_illegality_is_sealed_as_negative_credit_and_learned() {
        let backend = GpuClosedLoopBackend::new_required(
            alife_gpu_backend::GpuRuntimeProfile::production_v1(),
        )
        .expect("required GPU");
        let world = HeadlessScenarioBuilder::new(95)
            .agent("agent", OrganismId(1), Vec3f::ZERO)
            .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.8)
            .build()
            .unwrap();
        let mut runtime =
            GpuLiveBrainRuntime::new(backend, world, 95, BrainScaleTier::Nano512).unwrap();
        let handle = runtime.handle_for(OrganismId(1)).unwrap();
        let resident = runtime.residents.get(&1).unwrap();
        let normal = runtime
            .world
            .perception_frame(
                OrganismId(1),
                Tick::ZERO,
                SensorProfile::PrivilegedAffordanceV1,
                resident.homeostasis,
            )
            .unwrap();
        let mut illegal = *normal
            .candidates()
            .iter()
            .find(|candidate| candidate.family == CandidateActionFamily::Ingest)
            .expect("food frame exposes Eat");
        illegal.candidate_index = 0;
        illegal.target = ActionTarget::new(Some(WorldEntityId(999)), illegal.target.position);
        let draft = alife_core::PerceptionFrameDraft::new(
            OrganismId(1),
            Tick::ZERO,
            SensorProfile::PrivilegedAffordanceV1,
            normal.sensory().clone(),
            normal.body(),
            *normal.homeostasis(),
            vec![illegal],
            normal.profile_provenance(),
            normal.grounded_object_slots().to_vec(),
        )
        .unwrap();
        let prepared_recall = runtime.memories[&1].recall_frame(&draft).unwrap();
        let (frame, memory_recall) = prepared_recall.finalize(draft).unwrap();
        let memory_upload = runtime
            .backend
            .prepare_memory_context_upload(handle, &frame, &memory_recall)
            .unwrap();
        let input = GpuClosedLoopMemoryTickInput::try_new(handle, &frame, &memory_upload).unwrap();
        let batch = GpuClosedLoopMemoryBatchInput::try_new(vec![input]).unwrap();
        let gpu_tick = runtime.backend.tick_memory_batch(&batch).unwrap().remove(0);

        let summary = runtime
            .process_selection_batch(vec![(
                PreparedGpuBrainFrame {
                    handle,
                    frame,
                    memory_recall,
                    memory_upload,
                },
                gpu_tick,
            )])
            .unwrap()
            .remove(0);
        let patch = runtime.sealed_patches().last().unwrap();
        let credit = OutcomeCreditPacket::from_sealed_patch(patch).unwrap();

        assert!(summary.action_failure.is_some());
        assert_eq!(summary.learning_updates, 1);
        assert!(!patch.outcome().success);
        assert!(credit.modulator().value() < 0.0);
        assert_eq!(runtime.backend.pending_eligibility(handle).unwrap(), None);
        assert_eq!(runtime.last_learning_receipts().len(), 1);
        assert!(runtime.last_eligibility_discard_receipts().is_empty());
    }
}
