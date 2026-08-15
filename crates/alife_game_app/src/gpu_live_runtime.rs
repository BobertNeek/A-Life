//! GPU-authoritative live cognition for the explicit neural policy.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use alife_archive::{GeneticArchiveInput, LifeArchiveInput, LineageLibrary, LineageLibraryConfig};
use alife_core::cognitive_work::{CognitiveWorkCostPolicy, CognitiveWorkCounters};
use alife_core::predictive::GroundedSuccessorPredictor;
use alife_core::{
    ActionKind, ActionTarget, BiochemistryState, BoundedCoordinationSummary, BoundedMotorPayload,
    ChannelCommand, CognitiveWorkReceipt, CoordinationGroup,
    HomeostaticDelta, MotorChannel, MotorCommandBundle,
    PhysicalContactKind, PredictionTargetReceipt,
    ArchiveCheckpointRetention, ArchiveLearnedCapturePolicy, ArchiveRetirementReceipt,
    Blake3Digest, BrainCapacityClass, BrainGenome, BrainScaleTier, BrainTickStatus,
    finalized_memory_attention_evidence, select_focal_targets, AttentionFrame,
    AttentionSelectionPolicy, BrainWorkReceipt, CanonicalDigestBuilder, CognitiveConceptActivation,
    CognitiveContextFrame, CognitiveGapActivation, CognitiveMemoryExpectancy,
    FinalizedMemoryAttentionEvidence,
    Confidence, ConsolidationDriverEvent, ConsolidationIntent, ConsolidationState,
    DecisionSnapshot, DevelopmentState, EnvironmentalRegime, ExperiencePatch,
    ExperienceSequenceId, FinalizedMemoryRecall, FoundationGeneticIdentity,
    FoundationWeightAsset, HomeostaticParameters, HomeostaticSnapshot, LanguageGroundingLedger,
    MAX_CONTEXT_MEMORY_EXPECTANCIES, MemoryBankConfig,
    MemoryCompactionCheckpoint, MemoryCompactionReceipt, MemoryRecallReceipt, MemorySidecarState,
    MemoryUpdateReceipt, NeuralActionSelection, NormalizedScalar, OrganismId, PassiveLifeEvent,
    PassiveLifeStatistics, PerceptionFrame, PerceptionFrameDraft, PhenotypeCompiler,
    PhenotypeCompilerInputs,
    PostActionOutcome, PreActionSnapshot, PreparedMemoryRecall, ScaffoldContractError,
    CandidateObservationRef,
    SensorProfile, SignedValence,
    SensorProfileIdentity, SensoryAbiVersion, SleepConsolidationConfig, SleepPhase, SleepState,
    Tick, TopologicalMapConfig, TopologyObservationReceipt, TopologySidecar, UtteranceSourceKind,
    Validate, Vec3f, WorldEntityId, LineageId, N512FounderFoundationProjection,
    MAX_ACTIVE_CONCEPTS, MAX_ACTIVE_GAPS,
};
use alife_gpu_backend::{
    GpuBrainHandle, GpuClosedLoopBackend, GpuClosedLoopMemoryBatchInput,
    GpuClosedLoopMemoryTickInput, GpuClosedLoopTick, GpuCuratedResidencyCohort,
    GpuCuratedResidencyEntry, GpuCuratedResidencyOutcome, GpuCuratedResidencyReceipt,
    GpuCuratedResidencyTargetIdentity, GpuLearningReceipt, GpuMemoryContextUpload,
    PendingEligibilityDiscardReceipt, PendingEligibilityIdentity, PendingEligibilityReceipt,
    GPU_CLOSED_LOOP_TICK_READBACK_BYTES, GPU_FAST_PLASTICITY_COMMIT_BYTES,
    GPU_MOTOR_CHANNEL_SLOT_COUNT,
};
use alife_runtime::{
    DurableGpuCheckpointRef, GpuAuthoritativeSession, GpuSessionAuthority,
    GpuSessionConsumerKind, GpuSessionFailStopCause, SleepPhaseReceipt, SleepWorkDue,
};
use alife_world::{
    grounded_peripheral_summaries,
    persistence::{
        AssetManifest, CreatureMindSaveSummary, CreatureSaveState, GpuBrainSaveState,
        LearningTraceSaveSummary, PortableAssetDigest, PortableSaveFile, RuntimeConfig,
        WeightLayerSaveSummary,
    },
    CreatureAppearanceGenome, HabitatActor, HabitatAuthorityError, HabitatBreedingKind,
    HabitatBreedingReceipt,
    HabitatBreedingRequest, HabitatId, HabitatMode, HabitatOperation, HabitatOperationRequest,
    HabitatPermissionReceipt, HeadlessWorld, HeadlessWorldSignatureDigest, WorldEditorSpawnSpec,
    WorldObjectKind, WorldOrganismRecord,
};
use thiserror::Error;

use crate::{
    curated_founder_materializer::{
        materialize_curated_founder_bundle, CuratedFounderMaterializationError,
    },
    curated_founder_staging::{
        CuratedFounderDurableOperation, CuratedFounderDurableOperationAttempt,
        CuratedFounderDurablePublicationReceipt, CuratedFounderPublicationStatus,
        CuratedFounderSaveState, CuratedFounderStagingError,
    },
    merge_gpu_checkpoint_manifest_entries, plan_curated_founder_reset, AppShellLaunchConfig,
    CuratedFounderAgentInput, CuratedFounderResetError, CuratedFounderResetRequest,
    GameAppShellError, GpuBrainAuthorityTelemetry, GpuBrainCheckpointWrite, GpuBrainSidecarCapture,
    GpuCheckpointAssetStore, GpuDurableSaveManifest, GpuLoadedSaveManifest,
    GpuSleepConsolidationDriver, GpuSleepScheduleEvent, GpuSleepScheduler, LiveBrainCausalStage,
    LiveBrainTickSummary, RetainedLearningCapture, CURATED_FOUNDER_RESET_POLICY,
    G03_LIVE_BRAIN_LOOP_SCHEMA, G03_LIVE_BRAIN_LOOP_SCHEMA_VERSION,
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
    language_grounding: LanguageGroundingLedger,
    life_statistics: PassiveLifeStatistics,
    attention_hysteresis: alife_core::HysteresisState,
    predictor: GroundedSuccessorPredictor,
}

struct StagedLiveAuthority {
    world: HeadlessWorld,
    residents: BTreeMap<u64, ResidentCognition>,
}

impl StagedLiveAuthority {
    fn begin(
        world: &mut HeadlessWorld,
        residents: &mut BTreeMap<u64, ResidentCognition>,
    ) -> Self {
        Self {
            world: world.clone(),
            residents: residents.clone(),
        }
        .install(world, residents)
    }

    fn install(
        self,
        world: &mut HeadlessWorld,
        residents: &mut BTreeMap<u64, ResidentCognition>,
    ) -> Self {
        Self {
            world: std::mem::replace(world, self.world),
            residents: std::mem::replace(residents, self.residents),
        }
    }

    fn finish<T, E>(
        self,
        world: &mut HeadlessWorld,
        residents: &mut BTreeMap<u64, ResidentCognition>,
        result: Result<T, E>,
    ) -> Result<T, E> {
        let candidate = self.install(world, residents);
        match result {
            Ok(value) => {
                let _previous = candidate.install(world, residents);
                Ok(value)
            }
            Err(error) => Err(error),
        }
    }
}

trait LiveAuthorityOwner {
    fn world_and_residents(
        &mut self,
    ) -> (&mut HeadlessWorld, &mut BTreeMap<u64, ResidentCognition>);
}

fn tick_with_sleep_progress_inner<O, T, E>(
    owner: &mut O,
    staged_tick: impl FnOnce(&mut O) -> Result<T, E>,
) -> Result<T, E>
where
    O: LiveAuthorityOwner,
{
    let staged = {
        let (world, residents) = owner.world_and_residents();
        StagedLiveAuthority::begin(world, residents)
    };
    let result = staged_tick(owner);
    let (world, residents) = owner.world_and_residents();
    staged.finish(world, residents, result)
}

#[derive(Debug, Clone)]
struct ResidentAuthorityPlan {
    organism_id: OrganismId,
    world_entity_id: WorldEntityId,
    world_tick: Tick,
    phenotype: alife_core::BrainPhenotype,
    compiler_inputs: PhenotypeCompilerInputs,
    genome: BrainGenome,
    development: DevelopmentState,
    biochemistry: BiochemistryState,
}

#[derive(Debug, Clone, Copy)]
struct ResidentCheckpointMetadata<'a> {
    organism_id: OrganismId,
    phenotype_hash: alife_core::PhenotypeHash,
    capacity_class_id: alife_core::BrainClassId,
    checkpoint_tick: Tick,
    phenotype: &'a alife_core::BrainPhenotype,
    compiler_inputs: &'a PhenotypeCompilerInputs,
}

fn resident_authority_plan_from_record(
    record: &WorldOrganismRecord,
    organism_id: OrganismId,
    world_entity_id: WorldEntityId,
    world_tick: Tick,
    brain_class: BrainScaleTier,
    sensor_profile: SensorProfile,
) -> Result<ResidentAuthorityPlan, ScaffoldContractError> {
    let admission = record.authoritative_admission_at(world_tick)?;
    if admission.organism_id != organism_id || admission.world_entity_id != world_entity_id {
        return Err(ScaffoldContractError::BrainOwnershipMismatch);
    }
    let development = admission.phenotype.development_state_at(admission.age)?;
    let genome = admission.phenotype.brain_genome.clone();
    let (phenotype, compiler_inputs) =
        compile_gpu_components_from_genome(genome.clone(), development.clone(), sensor_profile)?;
    if phenotype.brain_class_id() != brain_class.default_class_id() {
        return Err(ScaffoldContractError::PhenotypeCompile);
    }
    Ok(ResidentAuthorityPlan {
        organism_id,
        world_entity_id,
        world_tick,
        phenotype,
        compiler_inputs,
        genome,
        development,
        biochemistry: admission.biochemistry,
    })
}

fn synchronize_resident_from_record(
    resident: &mut ResidentCognition,
    record: &WorldOrganismRecord,
    world_tick: Tick,
) -> Result<(), ScaffoldContractError> {
    let admission = record.authoritative_admission_at(world_tick)?;
    if resident.genome != admission.phenotype.brain_genome {
        return Err(ScaffoldContractError::BrainOwnershipMismatch);
    }
    resident.homeostasis = admission.biochemistry.homeostasis;
    resident.development = admission.phenotype.development_state_at(admission.age)?;
    Ok(())
}

fn synchronize_residents_from_world(
    world: &HeadlessWorld,
    residents: &mut BTreeMap<u64, ResidentCognition>,
    world_tick: Tick,
) -> Result<(), ScaffoldContractError> {
    let organism_ids = residents.keys().copied().collect::<Vec<_>>();
    for raw in organism_ids {
        let record = world
            .organism_registry()
            .get(OrganismId(raw))
            .cloned()
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        if !record.lifecycle().is_alive() {
            continue;
        }
        let resident = residents
            .get_mut(&raw)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        synchronize_resident_from_record(resident, &record, world_tick)?;
    }
    Ok(())
}

fn advance_and_synchronize_authority(
    world: &mut HeadlessWorld,
    residents: &mut BTreeMap<u64, ResidentCognition>,
    tick_after: Tick,
) -> Result<(), ScaffoldContractError> {
    let advanced_tick = world.try_advance_tick()?;
    if advanced_tick != tick_after {
        return Err(ScaffoldContractError::NonMonotonicTick);
    }
    synchronize_residents_from_world(world, residents, tick_after)
}

fn compare_resident_checkpoint_metadata(
    plan: &ResidentAuthorityPlan,
    checkpoint: ResidentCheckpointMetadata<'_>,
) -> Result<(), ScaffoldContractError> {
    if checkpoint.organism_id != plan.organism_id {
        return Err(ScaffoldContractError::BrainOwnershipMismatch);
    }
    if checkpoint.checkpoint_tick != plan.world_tick {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    if checkpoint.capacity_class_id != plan.phenotype.brain_class_id()
        || checkpoint.phenotype_hash != plan.phenotype.phenotype_hash()
        || checkpoint.phenotype != &plan.phenotype
        || checkpoint.compiler_inputs.genome() != &plan.genome
        || checkpoint.compiler_inputs.development() != plan.compiler_inputs.development()
        || checkpoint.compiler_inputs.sensor_profile() != plan.compiler_inputs.sensor_profile()
    {
        return Err(ScaffoldContractError::PhenotypeCompile);
    }
    Ok(())
}

fn restore_resident_authority_from_record(
    record: &WorldOrganismRecord,
    organism_id: OrganismId,
    world_entity_id: WorldEntityId,
    world_tick: Tick,
    brain_class: BrainScaleTier,
    sensor_profile: SensorProfile,
    checkpoint: Option<ResidentCheckpointMetadata<'_>>,
) -> Result<ResidentAuthorityPlan, ScaffoldContractError> {
    let authority = resident_authority_plan_from_record(
        record,
        organism_id,
        world_entity_id,
        world_tick,
        brain_class,
        sensor_profile,
    )?;
    if let Some(checkpoint) = checkpoint {
        compare_resident_checkpoint_metadata(&authority, checkpoint)?;
    }
    Ok(authority)
}

fn cleanup_restored_gpu_handle(
    backend: &mut GpuAuthoritativeSession,
    handle: GpuBrainHandle,
    pending_eligibility: Option<PendingEligibilityReceipt>,
) -> Result<(), GameAppShellError> {
    let discard_result: Result<(), GameAppShellError> = match pending_eligibility {
        Some(receipt) => backend
            .discard_pending_eligibility(handle, receipt.identity())
            .map(|_| ())
            .map_err(GameAppShellError::from),
        None => Ok(()),
    };
    let remove_result = backend
        .remove_brain(handle)
        .map_err(GameAppShellError::from);
    if let Err(error) = discard_result {
        return Err(error);
    }
    remove_result
}

impl ResidentAuthorityPlan {
    fn into_fresh_resident(self) -> Result<ResidentCognition, ScaffoldContractError> {
        Ok(ResidentCognition {
            phenotype: self.phenotype,
            compiler_inputs: self.compiler_inputs,
            genome: self.genome,
            development: self.development,
            homeostasis: self.biochemistry.homeostasis,
            sleep_scheduler: GpuSleepScheduler::new(SleepConsolidationConfig::reference())?,
            next_sequence: 1,
            language_grounding: LanguageGroundingLedger::default(),
            life_statistics: PassiveLifeStatistics::new(self.organism_id, self.world_tick)?,
            attention_hysteresis: alife_core::HysteresisState::default(),
            predictor: GroundedSuccessorPredictor::default(),
        })
    }
}

const LIVE_COGNITIVE_ENERGY_PER_WORK_UNIT: f32 = 0.000_001;

#[derive(Debug, Clone)]
struct GpuLiveCheckpointDurability {
    store: GpuCheckpointAssetStore,
    durable_manifest: GpuDurableSaveManifest,
    published: GpuLoadedSaveManifest,
}

#[derive(Debug, Clone, Copy)]
struct GpuLiveRuntimeConstructionOptions {
    homeostatic_parameters: HomeostaticParameters,
    schedule_sleep: bool,
    observe_sidecars: bool,
    retain_sealed_patch_history: bool,
    cognitive_work_cost_policy: CognitiveWorkCostPolicy,
}

impl GpuLiveRuntimeConstructionOptions {
    const fn production() -> Self {
        Self {
            homeostatic_parameters: HomeostaticParameters::reference(),
            schedule_sleep: true,
            observe_sidecars: true,
            retain_sealed_patch_history: true,
            cognitive_work_cost_policy: CognitiveWorkCostPolicy {
                enabled: true,
                energy_per_work_unit: LIVE_COGNITIVE_ENERGY_PER_WORK_UNIT,
            },
        }
    }

    const fn benchmark(homeostatic_parameters: HomeostaticParameters) -> Self {
        Self {
            homeostatic_parameters,
            schedule_sleep: false,
            observe_sidecars: false,
            retain_sealed_patch_history: false,
            cognitive_work_cost_policy: CognitiveWorkCostPolicy::disabled(),
        }
    }

    const fn causal_acceptance() -> Self {
        Self {
            homeostatic_parameters: HomeostaticParameters::reference(),
            schedule_sleep: false,
            observe_sidecars: true,
            retain_sealed_patch_history: true,
            cognitive_work_cost_policy: CognitiveWorkCostPolicy {
                enabled: true,
                energy_per_work_unit: LIVE_COGNITIVE_ENERGY_PER_WORK_UNIT,
            },
        }
    }

    #[cfg(feature = "gpu-tests")]
    const fn soak() -> Self {
        Self {
            homeostatic_parameters: HomeostaticParameters::reference(),
            schedule_sleep: true,
            observe_sidecars: true,
            retain_sealed_patch_history: false,
            cognitive_work_cost_policy: CognitiveWorkCostPolicy {
                enabled: true,
                energy_per_work_unit: LIVE_COGNITIVE_ENERGY_PER_WORK_UNIT,
            },
        }
    }
}

impl GpuLiveCheckpointDurability {
    fn durable_reference(&self) -> Result<DurableGpuCheckpointRef, GameAppShellError> {
        let mut digest = CanonicalDigestBuilder::new(b"alife.runtime.durable-checkpoint-ref.v1");
        digest.write_u64(self.published.save.world.tick.raw());
        digest.write_utf8(self.published.digest.as_str());
        digest.write_sequence_len(self.published.save.creatures.len());
        for creature in &self.published.save.creatures {
            digest.write_u64(creature.organism_id.raw());
            match &creature.gpu_brain {
                Some(brain) => {
                    digest.write_some();
                    for word in brain.phenotype_hash.0 {
                        digest.write_u64(word);
                    }
                    digest.write_u64(brain.active_weight_generation);
                    digest.write_u64(brain.learning_transaction_generation);
                    digest.write_u64(brain.replay_journal_generation);
                    digest.write_utf8(&brain.lifetime_weights.digest.0);
                    digest.write_utf8(&brain.fast_weights.digest.0);
                    digest.write_utf8(&brain.eligibility.digest.0);
                    digest.write_utf8(&brain.activation_state.digest.0);
                    digest.write_utf8(&brain.neuron_homeostasis.digest.0);
                }
                None => digest.write_none(),
            }
        }
        Ok(DurableGpuCheckpointRef::try_new(
            self.published.save.world.tick,
            self.published.digest.as_str().to_string(),
            digest.finish256(),
        )?)
    }

    fn publish(
        &mut self,
        replacement: PortableSaveFile,
    ) -> Result<DurableGpuCheckpointRef, GameAppShellError> {
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
        self.durable_reference()
    }

    fn refresh_published(
        &mut self,
        expected_digest: &str,
    ) -> Result<DurableGpuCheckpointRef, GameAppShellError> {
        let published = self.durable_manifest.load()?;
        if published.digest.as_str() != expected_digest {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: format!(
                    "curated founder publication reload digest differs from verified result: expected {expected_digest}, actual {}",
                    published.digest.as_str()
                ),
            });
        }
        self.published = published;
        self.durable_reference()
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
    world_entity_id: WorldEntityId,
    pending_eligibility: PendingEligibilityReceipt,
    frame: PerceptionFrame,
    memory_recall: FinalizedMemoryRecall,
    work: BrainWorkReceipt,
    cognitive_context_digest: [u64; 4],
    sequence_id: ExperienceSequenceId,
    outcome_tick: Tick,
    pre_action: PreActionSnapshot,
    decision: DecisionSnapshot,
    motor_bundle: MotorCommandBundle,
    speech_payload: Option<alife_core::SpeechMotorPayload>,
    speech_prompted: bool,
}

struct PreparedSealInput {
    organism_id: OrganismId,
    world_entity_id: WorldEntityId,
    frame: PerceptionFrame,
    memory: MemoryRecallReceipt,
    sequence_id: ExperienceSequenceId,
    outcome_tick: Tick,
    cognitive_context: CognitiveContextFrame,
    work: BrainWorkReceipt,
    pre_action: PreActionSnapshot,
    decision: DecisionSnapshot,
    motor_bundle: MotorCommandBundle,
    speech_payload: Option<alife_core::SpeechMotorPayload>,
    speech_prompted: bool,
}

struct SealedWorldSelection {
    summary: LiveBrainTickSummary,
    patch: ExperiencePatch,
}

struct SealedLiveSelection {
    handle: GpuBrainHandle,
    pending_eligibility: PendingEligibilityReceipt,
    cognitive_context_digest: [u64; 4],
    summary: LiveBrainTickSummary,
    patch: ExperiencePatch,
}

struct PreparedGpuBrainFrame {
    handle: GpuBrainHandle,
    world_entity_id: WorldEntityId,
    frame: PerceptionFrame,
    memory_recall: FinalizedMemoryRecall,
    memory_upload: GpuMemoryContextUpload,
}

// The bank must exceed each 64-record target/family shortlist so production
// recall can remain compute-bounded while reporting genuine capacity pressure.
const LIVE_MEMORY_CAPACITY: usize = 256;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CuratedFounderGpuResidencyState {
    NotStarted,
    Pending,
    Committed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
struct CuratedFounderGpuResidencyPlanEntry {
    final_population_slot: u32,
    world_entity_id: WorldEntityId,
    organism_id: OrganismId,
    lineage_id: LineageId,
    archive_birth_manifest_digest: Blake3Digest,
    projection: N512FounderFoundationProjection,
}

#[derive(Debug, Clone, PartialEq)]
struct CuratedFounderGpuResidencyPlan {
    state: CuratedFounderGpuResidencyState,
    final_save_digest: String,
    candidate_world_signature: HeadlessWorldSignatureDigest,
    world_seed: u64,
    world_tick: Tick,
    source_run_identity: String,
    entries: Vec<CuratedFounderGpuResidencyPlanEntry>,
    fingerprint: [u64; 4],
}

impl CuratedFounderGpuResidencyPlan {
    fn from_accepted_operation(
        operation: &CuratedFounderDurableOperation,
        publication: &CuratedFounderDurablePublicationReceipt,
    ) -> Self {
        let archive_identities = publication.archive_receipt_identities();
        let entries = operation
            .accepted_bundle()
            .entries
            .iter()
            .map(|accepted| {
                let archived = archive_identities
                    .iter()
                    .find(|row| row.2 == accepted.plan_entry.organism_id)
                    .expect("validated durable publication contains every accepted founder");
                CuratedFounderGpuResidencyPlanEntry {
                    final_population_slot: accepted.plan_entry.final_population_slot,
                    world_entity_id: accepted.plan_entry.world_entity_id,
                    organism_id: accepted.plan_entry.organism_id,
                    lineage_id: accepted.plan_entry.lineage_id,
                    archive_birth_manifest_digest: archived.4,
                    projection: accepted.projection.clone(),
                }
            })
            .collect::<Vec<_>>();
        let mut plan = Self {
            state: CuratedFounderGpuResidencyState::NotStarted,
            final_save_digest: publication
                .final_save_digest()
                .expect("published curated founder receipt has a final save digest")
                .to_string(),
            candidate_world_signature: publication.candidate_world_signature(),
            world_seed: publication.candidate_world_seed(),
            world_tick: publication.candidate_world_tick(),
            source_run_identity: publication.archive_source_run().to_string(),
            entries,
            fingerprint: [0; 4],
        };
        plan.fingerprint = curated_founder_gpu_residency_plan_fingerprint(&plan);
        plan
    }
}

fn curated_founder_gpu_residency_plan_fingerprint(
    plan: &CuratedFounderGpuResidencyPlan,
) -> [u64; 4] {
    let mut digest = CanonicalDigestBuilder::new(b"alife.curated-founder.gpu-residency-plan.v1");
    digest.write_utf8(&plan.final_save_digest);
    digest.write_u16(plan.candidate_world_signature.schema_version);
    for word in plan.candidate_world_signature.words {
        digest.write_u64(word);
    }
    digest.write_u64(plan.world_seed);
    digest.write_u64(plan.world_tick.raw());
    digest.write_utf8(&plan.source_run_identity);
    digest.write_sequence_len(plan.entries.len());
    for entry in &plan.entries {
        digest.write_u32(entry.final_population_slot);
        digest.write_u64(entry.world_entity_id.raw());
        digest.write_u64(entry.organism_id.raw());
        digest.write_u64(entry.lineage_id.raw());
        digest.write_bytes(entry.archive_birth_manifest_digest.bytes());
        let projection = &entry.projection;
        let foundation = projection.foundation();
        digest.write_u64(foundation.foundation_id);
        digest.write_u16(foundation.version);
        digest.write_u64(foundation.compatibility_family_id);
        digest.write_u16(foundation.brain_class_id.raw());
        digest.write_u16(projection.sensor_profile().raw());
        digest.write_bytes(projection.foundation_asset_digest().bytes());
        for word in projection.receipt().phenotype_hash().0 {
            digest.write_u64(word);
        }
        for word in projection.receipt().digest() {
            digest.write_u64(word);
        }
        digest.write_bytes(projection.frozen_abi().address_map_digest().bytes());
    }
    digest.finish256()
}

#[derive(Debug, Error)]
pub(crate) enum CuratedFounderResetRuntimeError {
    #[error("curated founder reset requires the runtime-owned durable save boundary")]
    MissingDurability,
    #[error("curated founder reset requires the already-attached lineage archive")]
    MissingLineageArchive,
    #[error("curated founder reset requires the attached lineage archive source run ID")]
    MissingLineageRunId,
    #[error("no retained curated founder operation is available for same-process retry")]
    NoRetainedOperation,
    #[error("a curated founder operation is retained; retry it before starting another reset")]
    RetainedOperationPending,
    #[error("a curated founder GPU residency plan is retained; recover it before starting another reset")]
    RetainedResidencyPlanPending,
    #[error("the retained curated founder GPU residency plan changed during retry")]
    ResidencyPlanMismatch,
    #[error("curated founder reset plan rejected before archive commit: {0}")]
    Plan(#[from] CuratedFounderResetError),
    #[error("curated founder reset bundle rejected before archive commit: {0}")]
    Materialization(#[from] CuratedFounderMaterializationError),
    #[error("curated founder reset staging rejected before archive commit: {0}")]
    PreCommit(#[from] CuratedFounderStagingError),
    #[error("curated founder reset durable publication refresh failed after publication: {error}")]
    DurableRefresh {
        evidence: CuratedFounderResetRuntimeEvidence,
        #[source]
        error: GameAppShellError,
    },
    #[error("curated founder reset GPU checkpoint notification failed after publication: {error}")]
    DurableCheckpointNotification {
        evidence: CuratedFounderResetRuntimeEvidence,
        #[source]
        error: GameAppShellError,
    },
    #[error("curated founder GPU residency remains retryable before submission: {error}")]
    GpuResidencyPreSubmit {
        #[source]
        error: ScaffoldContractError,
    },
    #[error("curated founder GPU residency is unknown after submission: {error}")]
    GpuResidencyUnknown {
        evidence: Option<CuratedFounderResetRuntimeEvidence>,
        #[source]
        error: ScaffoldContractError,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CuratedFounderResetAttempt {
    publication: CuratedFounderDurableOperationAttempt,
    gpu_residency: CuratedFounderGpuResidencyState,
}

impl CuratedFounderResetAttempt {
    pub(crate) const fn publication_status(&self) -> CuratedFounderPublicationStatus {
        self.publication.status()
    }

    pub(crate) const fn save_state(&self) -> CuratedFounderSaveState {
        self.publication.save_state()
    }

    pub(crate) const fn gpu_residency_state(&self) -> CuratedFounderGpuResidencyState {
        self.gpu_residency
    }

    pub(crate) fn receipt(
        &self,
    ) -> &crate::curated_founder_staging::CuratedFounderDurablePublicationReceipt {
        self.publication.receipt()
    }

    pub(crate) fn expected_save_digest(&self) -> Option<&str> {
        self.publication.expected_save_digest()
    }

    pub(crate) fn actual_save_digest(&self) -> Option<&str> {
        self.publication.actual_save_digest()
    }

    pub(crate) fn proposed_save_digest(&self) -> &str {
        self.publication.proposed_save_digest()
    }

    pub(crate) fn cause(&self) -> Option<&str> {
        self.publication.cause()
    }

    pub(crate) fn archive_receipt_count(&self) -> usize {
        self.receipt().archive_receipt_count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratedFounderResetRuntimeEvidence {
    pub(crate) status: CuratedFounderPublicationStatus,
    pub(crate) save_state: CuratedFounderSaveState,
    pub(crate) gpu_residency: CuratedFounderGpuResidencyState,
    pub(crate) expected_save_digest: Option<String>,
    pub(crate) actual_save_digest: Option<String>,
    pub(crate) proposed_save_digest: String,
    pub(crate) cause: Option<String>,
    pub(crate) archive_count: usize,
}

impl CuratedFounderResetRuntimeEvidence {
    fn from_attempt(result: &CuratedFounderResetAttempt) -> Self {
        Self {
            status: result.publication_status(),
            save_state: result.save_state(),
            gpu_residency: result.gpu_residency_state(),
            expected_save_digest: result.expected_save_digest().map(str::to_string),
            actual_save_digest: result.actual_save_digest().map(str::to_string),
            proposed_save_digest: result.proposed_save_digest().to_string(),
            cause: result.cause().map(str::to_string),
            archive_count: result.archive_receipt_count(),
        }
    }
}

pub(crate) type CuratedFounderResetRuntimeResult =
    Result<CuratedFounderResetRuntimeEvidence, CuratedFounderResetRuntimeError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LiveAgentResetIntent {
    pub(crate) final_agents: Vec<CuratedFounderAgentInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CuratedFounderResetDispatchRejection {
    MultipleCommands,
    RetainedOperationPending,
    RetainedResidencyPlanPending,
    Runtime { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CuratedFounderResetDispatchResult {
    Idle,
    PreCommitRejected {
        rejection: CuratedFounderResetDispatchRejection,
    },
    Published {
        status: CuratedFounderPublicationStatus,
        save_state: CuratedFounderSaveState,
        gpu_residency: CuratedFounderGpuResidencyState,
        proposed_save_digest: String,
        archive_count: usize,
    },
    Conflict {
        expected_save_digest: String,
        actual_save_digest: String,
        proposed_save_digest: String,
        archive_count: usize,
        save_state: CuratedFounderSaveState,
        gpu_residency: CuratedFounderGpuResidencyState,
        retryable: bool,
    },
    Unknown {
        cause: String,
        proposed_save_digest: String,
        archive_count: usize,
        save_state: CuratedFounderSaveState,
        gpu_residency: CuratedFounderGpuResidencyState,
        retryable: bool,
    },
}

pub(crate) trait CuratedFounderResetRuntimePort {
    fn dispatch_attempt(
        &mut self,
        intent: LiveAgentResetIntent,
    ) -> CuratedFounderResetRuntimeResult;

    fn dispatch_retry(&mut self) -> CuratedFounderResetRuntimeResult;
}

pub(crate) fn project_curated_founder_reset_result(
    result: CuratedFounderResetRuntimeResult,
) -> CuratedFounderResetDispatchResult {
    let result = match result {
        Ok(result) => result,
        Err(error) => return project_curated_founder_reset_runtime_error(error),
    };

    match result.status {
        CuratedFounderPublicationStatus::Published
        | CuratedFounderPublicationStatus::AlreadyApplied => {
            CuratedFounderResetDispatchResult::Published {
                status: result.status,
                save_state: result.save_state,
                gpu_residency: result.gpu_residency,
                proposed_save_digest: result.proposed_save_digest,
                archive_count: result.archive_count,
            }
        }
        CuratedFounderPublicationStatus::ArchiveCommittedSaveConflict => {
            CuratedFounderResetDispatchResult::Conflict {
                expected_save_digest: result
                    .expected_save_digest
                    .expect("conflict runtime evidence has an expected save digest"),
                actual_save_digest: result
                    .actual_save_digest
                    .expect("conflict runtime evidence has an actual save digest"),
                proposed_save_digest: result.proposed_save_digest,
                archive_count: result.archive_count,
                save_state: result.save_state,
                gpu_residency: result.gpu_residency,
                retryable: true,
            }
        }
        CuratedFounderPublicationStatus::ArchiveCommittedSaveFailure => {
            CuratedFounderResetDispatchResult::Unknown {
                cause: result
                    .cause
                    .expect("unknown runtime evidence has a publication cause"),
                proposed_save_digest: result.proposed_save_digest,
                archive_count: result.archive_count,
                save_state: result.save_state,
                gpu_residency: result.gpu_residency,
                retryable: true,
            }
        }
    }
}

fn project_curated_founder_reset_runtime_error(
    error: CuratedFounderResetRuntimeError,
) -> CuratedFounderResetDispatchResult {
    match error {
        CuratedFounderResetRuntimeError::RetainedOperationPending => {
            CuratedFounderResetDispatchResult::PreCommitRejected {
                rejection: CuratedFounderResetDispatchRejection::RetainedOperationPending,
            }
        }
        CuratedFounderResetRuntimeError::RetainedResidencyPlanPending => {
            CuratedFounderResetDispatchResult::PreCommitRejected {
                rejection: CuratedFounderResetDispatchRejection::RetainedResidencyPlanPending,
            }
        }
        CuratedFounderResetRuntimeError::DurableRefresh { evidence, error } => {
            project_post_publication_failure(
                evidence,
                "durable publication refresh",
                error,
                CuratedFounderSaveState::Unknown,
                false,
                "manual recovery is required",
            )
        }
        CuratedFounderResetRuntimeError::DurableCheckpointNotification { evidence, error } => {
            let save_state = evidence.save_state;
            project_post_publication_failure(
                evidence,
                "durable checkpoint notification",
                error,
                save_state,
                false,
                "manual recovery is required",
            )
        }
        error @ (CuratedFounderResetRuntimeError::MissingDurability
        | CuratedFounderResetRuntimeError::MissingLineageArchive
        | CuratedFounderResetRuntimeError::MissingLineageRunId
        | CuratedFounderResetRuntimeError::NoRetainedOperation
        | CuratedFounderResetRuntimeError::ResidencyPlanMismatch
        | CuratedFounderResetRuntimeError::GpuResidencyPreSubmit { .. }
        | CuratedFounderResetRuntimeError::Plan(_)
        | CuratedFounderResetRuntimeError::Materialization(_)
        | CuratedFounderResetRuntimeError::PreCommit(_)) => {
            CuratedFounderResetDispatchResult::PreCommitRejected {
                rejection: CuratedFounderResetDispatchRejection::Runtime {
                    message: error.to_string(),
                },
            }
        }
        CuratedFounderResetRuntimeError::GpuResidencyUnknown { evidence, error } => {
            let (proposed_save_digest, archive_count, save_state) = evidence.map_or_else(
                || (String::new(), 0, CuratedFounderSaveState::Unknown),
                |evidence| {
                    (
                        evidence.proposed_save_digest,
                        evidence.archive_count,
                        evidence.save_state,
                    )
                },
            );
            CuratedFounderResetDispatchResult::Unknown {
                cause: error.to_string(),
                proposed_save_digest,
                archive_count,
                save_state,
                gpu_residency: CuratedFounderGpuResidencyState::Unknown,
                retryable: false,
            }
        }
    }
}

fn project_post_publication_failure(
    evidence: CuratedFounderResetRuntimeEvidence,
    phase: &str,
    error: GameAppShellError,
    save_state: CuratedFounderSaveState,
    retryable: bool,
    recovery: &str,
) -> CuratedFounderResetDispatchResult {
    let publication = match evidence.status {
        CuratedFounderPublicationStatus::Published => "published",
        CuratedFounderPublicationStatus::AlreadyApplied => "already-applied",
        CuratedFounderPublicationStatus::ArchiveCommittedSaveConflict => {
            "archive-committed-conflict"
        }
        CuratedFounderPublicationStatus::ArchiveCommittedSaveFailure => {
            "archive-committed-save-failure"
        }
    };
    CuratedFounderResetDispatchResult::Unknown {
        cause: format!(
            "{phase} failed after {publication} durable publication: {error}; {recovery}"
        ),
        proposed_save_digest: evidence.proposed_save_digest,
        archive_count: evidence.archive_count,
        save_state,
        gpu_residency: evidence.gpu_residency,
        retryable,
    }
}

impl CuratedFounderResetRuntimePort for GpuLiveBrainRuntime {
    fn dispatch_attempt(
        &mut self,
        intent: LiveAgentResetIntent,
    ) -> CuratedFounderResetRuntimeResult {
        self.attempt_live_agent_reset(intent)
            .map(|result| CuratedFounderResetRuntimeEvidence::from_attempt(&result))
    }

    fn dispatch_retry(&mut self) -> CuratedFounderResetRuntimeResult {
        self.retry_curated_founder_reset()
            .map(|result| CuratedFounderResetRuntimeEvidence::from_attempt(&result))
    }
}

/// Owns all production neural authority for one headless world.
pub struct GpuLiveBrainRuntime {
    backend: GpuAuthoritativeSession,
    handles: BTreeMap<u64, GpuBrainHandle>,
    residents: BTreeMap<u64, ResidentCognition>,
    memories: BTreeMap<u64, MemorySidecarState>,
    topologies: BTreeMap<u64, TopologySidecar>,
    retained_learning: BTreeMap<u64, RetainedLearningRecovery>,
    world: HeadlessWorld,
    deterministic_seed: u64,
    brain_class: BrainScaleTier,
    sensor_profile: SensorProfile,
    homeostatic_parameters: HomeostaticParameters,
    cognitive_work_cost_policy: CognitiveWorkCostPolicy,
    schedule_sleep: bool,
    sealed_patches: Vec<ExperiencePatch>,
    sealed_patch_count: usize,
    last_sealed_patches: Vec<ExperiencePatch>,
    observe_sidecars: bool,
    retain_sealed_patch_history: bool,
    last_learning_receipts: Vec<GpuLearningReceipt>,
    last_activity_work_receipts: Vec<BrainWorkReceipt>,
    last_cognitive_work_receipts: Vec<CognitiveWorkReceipt>,
    last_memory_recall_receipts: Vec<MemoryRecallReceipt>,
    last_memory_update_receipts: Vec<MemoryUpdateReceipt>,
    last_cognitive_context_digests: Vec<[u64; 4]>,
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
    lineage_library: Option<LineageLibrary>,
    lineage_run_id: Option<String>,
    retained_curated_founder_operation: Option<CuratedFounderDurableOperation>,
    retained_curated_founder_gpu_residency_plan: Option<CuratedFounderGpuResidencyPlan>,
    retained_curated_founder_gpu_residency_receipt: Option<GpuCuratedResidencyReceipt>,
    curated_first_tick_pending: bool,
    archive_learned_capture_policy: ArchiveLearnedCapturePolicy,
    archive_birth_manifests: BTreeMap<u64, Blake3Digest>,
    archive_retirement_receipts: BTreeMap<u64, ArchiveRetirementReceipt>,
    presentation_retirements: BTreeSet<u64>,
    #[cfg(test)]
    forced_retirement_post_receipt_failure: bool,
    #[cfg(test)]
    retirement_backend_removal_count: usize,
    #[cfg(test)]
    forced_late_advance_failure: bool,
}

impl LiveAuthorityOwner for GpuLiveBrainRuntime {
    fn world_and_residents(
        &mut self,
    ) -> (&mut HeadlessWorld, &mut BTreeMap<u64, ResidentCognition>) {
        (&mut self.world, &mut self.residents)
    }
}

#[cfg(feature = "bevy-app")]
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct CuratedFirstGpuActionTestEvidence {
    pub receipt: alife_gpu_backend::GpuCuratedResidencyReceipt,
    pub summary: LiveBrainTickSummary,
    pub post_action_world: HeadlessWorld,
    pub pre_action_position: Vec3f,
    pub selected_world_entity_id: WorldEntityId,
    pub selected_organism_id: OrganismId,
    pub residency_gate_rejections: u8,
    pub gpu_selection_count: u64,
    pub sealed_patch_count: usize,
}

type CuratedFounderDurableRefresh = fn(
    &mut GpuLiveCheckpointDurability,
    &str,
) -> Result<(), GameAppShellError>;

fn curated_founder_gpu_residency_state(
    retained_plan: &Option<CuratedFounderGpuResidencyPlan>,
) -> CuratedFounderGpuResidencyState {
    retained_plan
        .as_ref()
        .map(|plan| plan.state)
        .unwrap_or(CuratedFounderGpuResidencyState::NotStarted)
}

fn attempt_curated_founder_reset_with_owned_authorities(
    checkpoint_durability: &mut Option<GpuLiveCheckpointDurability>,
    lineage_library: &mut Option<LineageLibrary>,
    lineage_run_id: Option<&str>,
    world: &mut HeadlessWorld,
    retained_operation: &mut Option<CuratedFounderDurableOperation>,
    retained_plan: &mut Option<CuratedFounderGpuResidencyPlan>,
    request: Option<CuratedFounderResetRequest>,
) -> Result<CuratedFounderResetAttempt, CuratedFounderResetRuntimeError> {
    attempt_curated_founder_reset_with_owned_authorities_and_refresh(
        checkpoint_durability,
        lineage_library,
        lineage_run_id,
        world,
        retained_operation,
        retained_plan,
        request,
        |durability, expected_digest| durability.refresh_published(expected_digest).map(|_| ()),
    )
}

fn attempt_curated_founder_reset_with_owned_authorities_and_refresh(
    checkpoint_durability: &mut Option<GpuLiveCheckpointDurability>,
    lineage_library: &mut Option<LineageLibrary>,
    lineage_run_id: Option<&str>,
    world: &mut HeadlessWorld,
    retained_operation: &mut Option<CuratedFounderDurableOperation>,
    retained_plan: &mut Option<CuratedFounderGpuResidencyPlan>,
    request: Option<CuratedFounderResetRequest>,
    refresh_published: CuratedFounderDurableRefresh,
) -> Result<CuratedFounderResetAttempt, CuratedFounderResetRuntimeError> {
    if checkpoint_durability.is_none() {
        return Err(CuratedFounderResetRuntimeError::MissingDurability);
    }
    if lineage_library.is_none() {
        return Err(CuratedFounderResetRuntimeError::MissingLineageArchive);
    }
    if retained_operation.is_some() && request.is_some() {
        return Err(CuratedFounderResetRuntimeError::RetainedOperationPending);
    }
    if retained_plan.is_some() && request.is_some() {
        return Err(CuratedFounderResetRuntimeError::RetainedResidencyPlanPending);
    }

    let (operation, operation_was_retained) = match retained_operation.take() {
        Some(operation) => (operation, true),
        None => {
            let request = request.ok_or(CuratedFounderResetRuntimeError::NoRetainedOperation)?;
            let archive_run_id =
                lineage_run_id.ok_or(CuratedFounderResetRuntimeError::MissingLineageRunId)?;
            let plan = plan_curated_founder_reset(&request)?;
            let bundle = materialize_curated_founder_bundle(&plan)?;
            let durability = checkpoint_durability
                .as_ref()
                .expect("durability presence was checked above");
            let lineage_library = lineage_library
                .as_ref()
                .expect("lineage archive presence was checked above");
            let operation = CuratedFounderDurableOperation::bind_and_stage(
                &plan,
                bundle,
                &durability.durable_manifest,
                world,
                lineage_library,
                archive_run_id,
            )?;
            (operation, false)
        }
    };

    let publication = {
        let durability = checkpoint_durability
            .as_ref()
            .expect("durability presence was checked above");
        let lineage_library = lineage_library
            .as_ref()
            .expect("lineage archive presence was checked above");
        match operation.attempt(&durability.durable_manifest, lineage_library, world) {
            Ok(publication) => publication,
            Err(error) => {
                if operation_was_retained {
                    *retained_operation = Some(operation);
                }
                return Err(error.into());
            }
        }
    };

    if publication.retains_operation() {
        *retained_operation = Some(operation);
        return Ok(CuratedFounderResetAttempt {
            publication,
            gpu_residency: curated_founder_gpu_residency_state(retained_plan),
        });
    }

    let final_save_digest = match publication.final_save_digest() {
        Some(digest) => digest,
        None => {
            *retained_operation = None;
            let result = CuratedFounderResetAttempt {
                publication,
                gpu_residency: curated_founder_gpu_residency_state(retained_plan),
            };
            return Err(CuratedFounderResetRuntimeError::DurableRefresh {
                evidence: CuratedFounderResetRuntimeEvidence::from_attempt(&result),
                error: GameAppShellError::InvalidProductionFrontend {
                    message: "curated founder success omitted its verified final save digest"
                        .to_string(),
                },
            });
        }
    };

    let next_plan = CuratedFounderGpuResidencyPlan::from_accepted_operation(
        &operation,
        publication.receipt(),
    );
    if let Some(existing_plan) = retained_plan.as_ref() {
        if existing_plan.fingerprint != next_plan.fingerprint {
            *retained_operation = Some(operation);
            return Err(CuratedFounderResetRuntimeError::ResidencyPlanMismatch);
        }
    } else {
        *retained_plan = Some(next_plan);
    }

    if let Err(error) = refresh_published(
        checkpoint_durability
            .as_mut()
            .expect("durability presence was checked above"),
        final_save_digest,
    ) {
        *retained_operation = None;
        if let Some(plan) = retained_plan.as_mut() {
            plan.state = CuratedFounderGpuResidencyState::Unknown;
        }
        let result = CuratedFounderResetAttempt {
            publication,
            gpu_residency: CuratedFounderGpuResidencyState::Unknown,
        };
        return Err(CuratedFounderResetRuntimeError::DurableRefresh {
            evidence: CuratedFounderResetRuntimeEvidence::from_attempt(&result),
            error,
        });
    }

    let plan = retained_plan
        .as_mut()
        .expect("published curated founder reset has an exact GPU residency plan");
    plan.state = CuratedFounderGpuResidencyState::Pending;
    let result = CuratedFounderResetAttempt {
        publication,
        gpu_residency: CuratedFounderGpuResidencyState::Pending,
    };
    Ok(result)
}

const LINEAGE_SOURCE_RUN_DOMAIN: &[u8] = b"alife.production.lineage-source-run.v1";
const CANONICAL_DIGEST_TAG_U64: u8 = 0x05;
const CANONICAL_DIGEST_TAG_UTF8: u8 = 0x10;
const CANONICAL_DIGEST_TAG_DOMAIN: u8 = 0xd0;

fn append_canonical_length(bytes: &mut Vec<u8>, length: usize) {
    let length = u64::try_from(length).expect("canonical input length fits in u64");
    bytes.extend_from_slice(&length.to_le_bytes());
}

fn append_canonical_utf8(bytes: &mut Vec<u8>, value: &str) {
    bytes.push(CANONICAL_DIGEST_TAG_UTF8);
    append_canonical_length(bytes, value.len());
    bytes.extend_from_slice(value.as_bytes());
}

fn append_canonical_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.push(CANONICAL_DIGEST_TAG_U64);
    bytes.extend_from_slice(&value.to_le_bytes());
}

/// Builds the versioned bytes emitted before `CanonicalDigestBuilder`'s
/// legacy finalizer. The production source-run identity hashes these bytes
/// with BLAKE3 instead of using that finalizer.
fn lineage_source_run_canonical_bytes(
    save_id: &str,
    deterministic_seed: u64,
    world_seed: u64,
    world_tick: u64,
    raw_generation_digest: &str,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.push(CANONICAL_DIGEST_TAG_DOMAIN);
    append_canonical_length(&mut bytes, LINEAGE_SOURCE_RUN_DOMAIN.len());
    bytes.extend_from_slice(LINEAGE_SOURCE_RUN_DOMAIN);
    append_canonical_utf8(&mut bytes, save_id);
    append_canonical_u64(&mut bytes, deterministic_seed);
    append_canonical_u64(&mut bytes, world_seed);
    append_canonical_u64(&mut bytes, world_tick);
    append_canonical_utf8(&mut bytes, raw_generation_digest);
    bytes
}

fn lineage_source_run_id_for_fields(
    save_id: &str,
    deterministic_seed: u64,
    world_seed: u64,
    world_tick: u64,
    raw_generation_digest: &str,
) -> String {
    let digest = Blake3Digest::from_bytes(
        *blake3::hash(&lineage_source_run_canonical_bytes(
            save_id,
            deterministic_seed,
            world_seed,
            world_tick,
            raw_generation_digest,
        ))
        .as_bytes(),
    );
    let digest_hex = digest
        .bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("runtime-save-v1-{digest_hex}")
}

fn derive_lineage_run_id(published: &GpuLoadedSaveManifest) -> String {
    lineage_source_run_id_for_fields(
        &published.save.save_id,
        published.save.deterministic_seed,
        published.save.world.seed,
        published.save.world.tick.raw(),
        published.digest.as_str(),
    )
}

fn validate_replacement_policy(
    persisted_policy: alife_core::PolicyBackend,
    persisted_seed: u64,
    persisted_brain_class: BrainScaleTier,
    expected_seed: u64,
    expected_brain_class: BrainScaleTier,
) -> Result<(), GameAppShellError> {
    if persisted_policy != alife_core::PolicyBackend::NeuralClosedLoopGpu
        || persisted_seed != expected_seed
        || persisted_brain_class != expected_brain_class
    {
        return Err(GameAppShellError::InvalidGraphicalLaunch {
            message: "GPU neural runtime replacement requires persisted GPU policy and matching live seed and brain class",
        });
    }
    Ok(())
}

fn commit_staged_runtime<T, E, F>(
    live: &mut T,
    staged: Result<T, E>,
    commit: F,
) -> Result<(), E>
where
    F: FnOnce(&mut T, T),
{
    let candidate = staged?;
    commit(live, candidate);
    Ok(())
}

fn attach_lineage_archive_with_owned_authorities(
    checkpoint_durability: Option<&GpuLiveCheckpointDurability>,
    sensor_profile: SensorProfile,
    world_tick: Tick,
    residents: &BTreeMap<u64, ResidentCognition>,
    lineage_library: &mut Option<LineageLibrary>,
    lineage_run_id: &mut Option<String>,
    archive_learned_capture_policy: &mut ArchiveLearnedCapturePolicy,
    archive_birth_manifests: &mut BTreeMap<u64, Blake3Digest>,
    config: LineageLibraryConfig,
    learned_capture_policy: ArchiveLearnedCapturePolicy,
) -> Result<(), GameAppShellError> {
    if lineage_library.is_some() || lineage_run_id.is_some() || !archive_birth_manifests.is_empty()
    {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: "lineage archive is already attached".to_string(),
        });
    }
    let durability =
        checkpoint_durability.ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
            message: "lineage archive attachment requires the durable save boundary".to_string(),
        })?;
    let source_run_id = derive_lineage_run_id(&durability.published);
    let mut candidate_library = LineageLibrary::open(config)?;
    let mut candidate_birth_manifests = BTreeMap::new();
    for (&raw, resident) in residents {
        let organism_id = OrganismId(raw);
        let digest = archive_birth_into_library(
            &mut candidate_library,
            &source_run_id,
            organism_id,
            world_tick,
            sensor_profile,
            resident,
        )?;
        candidate_birth_manifests.insert(raw, digest);
    }

    *lineage_library = Some(candidate_library);
    *lineage_run_id = Some(source_run_id);
    *archive_learned_capture_policy = learned_capture_policy;
    *archive_birth_manifests = candidate_birth_manifests;
    Ok(())
}

fn archive_birth_into_library(
    lineage_library: &mut LineageLibrary,
    source_run_id: &str,
    organism_id: OrganismId,
    birth_tick: Tick,
    sensor_profile: SensorProfile,
    resident: &ResidentCognition,
) -> Result<Blake3Digest, GameAppShellError> {
    if resident.phenotype.sensor_profile() != sensor_profile {
        return Err(ScaffoldContractError::SensorProfileMismatch.into());
    }
    if let Some(existing_digest) =
        lineage_library.latest_manifest_for(source_run_id, organism_id)?
    {
        let manifest = lineage_library.load_manifest(existing_digest)?;
        let genetic = &manifest.genetic;
        let abi = resident.phenotype.foundation_abi();
        let language = resident.phenotype.language_codebook();
        let archived_genome = lineage_library.load_brain_genome(&manifest)?;
        if manifest.life.is_some()
            || manifest.previous_manifest_digest.is_some()
            || archived_genome != resident.genome
            || genetic.source_run_id != source_run_id
            || genetic.organism_id != organism_id
            || genetic.genome_id != resident.genome.id
            || genetic.lineage_id != resident.genome.lineage_id
            || genetic.brain_class_id != resident.phenotype.brain_class_id()
            || genetic.birth_tick != birth_tick
            || genetic.sensor_profile != resident.phenotype.sensor_profile()
            || genetic.phenotype_hash != resident.phenotype.phenotype_hash()
            || genetic.foundation_id != abi.foundation_id()
            || genetic.foundation_version != abi.foundation_version()
            || genetic.compatibility_family_id != abi.compatibility_family_id()
            || genetic.foundation_payload_digest != abi.foundation_payload_digest()
            || genetic.persistent_address_map_digest
                != resident.phenotype.persistent_address_map().digest()
            || genetic.language_codebook_id != language.id()
            || genetic.language_codebook_digest != language.canonical_digest()
        {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: format!(
                    "existing lineage birth target conflicts for source run {source_run_id}, organism {}",
                    organism_id.raw()
                ),
            });
        }
        return Ok(existing_digest);
    }

    let foundation_asset_bytes = archive_foundation_asset_bytes(resident)?;
    Ok(lineage_library.archive_birth(GeneticArchiveInput {
        source_run_id,
        organism_id,
        birth_tick,
        genome: &resident.genome,
        phenotype: &resident.phenotype,
        foundation_asset_bytes: foundation_asset_bytes.as_deref(),
    })?)
}

fn archive_foundation_asset_bytes(
    resident: &ResidentCognition,
) -> Result<Option<Vec<u8>>, GameAppShellError> {
    let Some(expected_digest) = resident
        .phenotype
        .foundation_abi()
        .foundation_payload_digest()
    else {
        return Ok(None);
    };
    let sensor_profile = resident.phenotype.sensor_profile();
    let foundation = match resident.phenotype.brain_class_id() {
        id if id == BrainCapacityClass::N512_ID => {
            alife_core::FoundationWeightAsset::builtin_nano512_v1(sensor_profile)?
        }
        id if id == BrainCapacityClass::N2048_ID => {
            alife_core::FoundationWeightAsset::builtin_n2048_v1(sensor_profile)?
        }
        _ => return Err(ScaffoldContractError::UnsupportedProductionBrainClass.into()),
    };
    if foundation.digest() != expected_digest {
        return Err(ScaffoldContractError::PhenotypeCompile.into());
    }
    Ok(Some(foundation.encode_canonical()?))
}

fn cognitive_context_for_recall(
    organism_id: OrganismId,
    sequence_id: ExperienceSequenceId,
    recall: &PreparedMemoryRecall,
    topology: &TopologySidecar,
) -> Result<CognitiveContextFrame, ScaffoldContractError> {
    let mut context =
        CognitiveContextFrame::empty(organism_id, sequence_id, recall.context().tick)?;
    context.prediction.successor_feature_abi = 1;
    context.prediction.source_digest = recall.receipt().bank_digest;
    if let Some(candidate) = recall.context().candidates.first() {
        context.prediction.prediction_error.push(NormalizedScalar::new(
            candidate.family_value.first().copied().unwrap_or(0.0).abs(),
        )?);
        context.prediction.action_sensitivity =
            NormalizedScalar::new(candidate.family_confidence.raw())?;
    }
    for candidate in recall
        .context()
        .candidates
        .iter()
        .take(MAX_CONTEXT_MEMORY_EXPECTANCIES)
    {
        let expectancy = candidate
            .best_target_source
            .zip(candidate.target_latent.first().copied())
            .map(|(memory_id, value)| {
                (
                    memory_id,
                    value,
                    candidate.target_confidence.raw(),
                )
            })
            .or_else(|| {
                candidate
                    .best_family_source
                    .zip(candidate.family_value.first().copied())
                    .map(|(memory_id, value)| {
                        (
                            memory_id,
                            value,
                            candidate.family_confidence.raw(),
                        )
                    })
            });
        if let Some((memory_id, value, confidence)) = expectancy {
            context.memory.expectancies.push(CognitiveMemoryExpectancy {
                memory_id,
                expected_valence: SignedValence::new(value.clamp(-1.0, 1.0))?,
                confidence: NormalizedScalar::new(confidence.clamp(0.0, 1.0))?,
            });
        }
    }
    let mut concepts = topology.map().concepts().iter().collect::<Vec<_>>();
    concepts.sort_by(|left, right| {
        (right.salience.raw() * right.confidence.raw())
            .total_cmp(&(left.salience.raw() * left.confidence.raw()))
            .then_with(|| left.id.raw().cmp(&right.id.raw()))
    });
    context.concept.active_concepts = concepts
        .into_iter()
        .take(MAX_ACTIVE_CONCEPTS)
        .map(|concept| {
            Ok(CognitiveConceptActivation {
                concept_id: concept.id,
                activation: concept.salience,
                utility: NormalizedScalar::new(concept.confidence.raw())?,
            })
        })
        .collect::<Result<Vec<_>, ScaffoldContractError>>()?;
    context.concept.topology_digest = topology.diagnostics().canonical_digest;

    let mut gaps = topology.map().curiosity_biases();
    gaps.sort_by(|left, right| {
        right
            .salience
            .raw()
            .total_cmp(&left.salience.raw())
            .then_with(|| left.gap_id.raw().cmp(&right.gap_id.raw()))
    });
    context.gap.gap_voltage = NormalizedScalar::new(
        gaps.iter()
            .map(|gap| gap.salience.raw())
            .fold(0.0, f32::max),
    )?;
    context.gap.active_gaps = gaps
        .into_iter()
        .take(MAX_ACTIVE_GAPS)
        .map(|gap| {
            Ok(CognitiveGapActivation {
                gap_id: gap.gap_id,
                voltage: gap.salience,
                uncertainty: NormalizedScalar::new((1.0 - gap.confidence.raw()).clamp(0.0, 1.0))?,
            })
        })
        .collect::<Result<Vec<_>, ScaffoldContractError>>()?;
    context.validate_contract()?;
    Ok(context)
}

fn apply_predecision_attention_evidence(
    summaries: &mut [alife_core::PeripheralSummary],
    body_need: f32,
    memory_evidence: &[FinalizedMemoryAttentionEvidence],
    context: &CognitiveContextFrame,
) -> Result<(), ScaffoldContractError> {
    let concept_evidence = context
        .concept
        .active_concepts
        .iter()
        .map(|concept| concept.activation.raw() * concept.utility.raw())
        .fold(0.0, f32::max);
    let gap_evidence = context
        .gap
        .gap_voltage
        .raw()
        .max(
            context
                .gap
                .active_gaps
                .iter()
                .map(|gap| gap.voltage.raw())
                .fold(0.0, f32::max),
        );
    for summary in summaries {
        summary.salience.drive = NormalizedScalar::new(body_need.clamp(0.0, 1.0))?;
        summary.salience.concept = NormalizedScalar::new(concept_evidence.clamp(0.0, 1.0))?;
        summary.salience.gap_voltage = NormalizedScalar::new(gap_evidence.clamp(0.0, 1.0))?;
        if let alife_core::StableFocusIdentity::TrackedObject(tracked_object_id) = summary.identity
        {
            if let Some(memory) = memory_evidence.iter().find(|memory| {
                memory.tracked_object_id == Some(tracked_object_id)
            }) {
                summary.salience.memory_expectancy = memory.salience;
            }
        }
        summary.validate_contract()?;
    }
    Ok(())
}

fn route_focal_candidates(
    draft: PerceptionFrameDraft,
    attention: &AttentionFrame,
) -> Result<PerceptionFrameDraft, ScaffoldContractError> {
    attention.validate_contract()?;
    let Some(alife_core::StableFocusIdentity::TrackedObject(focal_id)) =
        attention.focal_targets.first().copied()
    else {
        return Ok(draft);
    };
    let Some(focal_slot) = draft
        .grounded_object_slots()
        .iter()
        .position(|slot| slot.tracked_object_id == focal_id)
        .and_then(|index| u16::try_from(index).ok())
    else {
        return Ok(draft);
    };

    let mut candidates = draft.candidates().to_vec();
    candidates.sort_by_key(|candidate| {
        (
            !matches!(
                candidate.observation,
                CandidateObservationRef::ObjectSlot(slot) if slot == focal_slot
            ),
            candidate.candidate_index,
        )
    });
    for (index, candidate) in candidates.iter_mut().enumerate() {
        candidate.candidate_index = u16::try_from(index)
            .map_err(|_| ScaffoldContractError::InvalidActionCandidate)?;
    }

    PerceptionFrameDraft::new(
        draft.organism_id(),
        draft.tick(),
        draft.sensor_profile(),
        draft.sensory().clone(),
        draft.body(),
        draft.homeostasis().clone(),
        candidates,
        draft.profile_provenance(),
        draft.grounded_object_slots().to_vec(),
    )
}

fn cognitive_context_with_attention(
    mut context: CognitiveContextFrame,
    attention: AttentionFrame,
) -> Result<CognitiveContextFrame, ScaffoldContractError> {
    context.attention = attention.clone();
    context.peripheral.summaries = attention.peripheral_summaries.clone();
    context.focal.identities = attention.focal_targets.clone();
    context.focal.salience = attention.salience_components.clone();
    context.focal.hysteresis = attention.hysteresis;
    context.budget.peripheral_capacity = attention.budget_receipt.peripheral_capacity;
    context.budget.focal_capacity = attention.budget_receipt.focal_capacity;
    context.budget.work_used = attention.budget_receipt.work_units;
    context.budget.work_limit = attention.budget_receipt.work_units;
    context.validate_contract()?;
    Ok(context)
}

fn bounded_successor_scalar(value: f32) -> Result<f32, ScaffoldContractError> {
    if !value.is_finite() {
        return Err(ScaffoldContractError::NonFiniteFloat);
    }
    Ok((0.5 + 0.5 * (value / (1.0 + value.abs()))).clamp(0.0, 1.0))
}

fn unit_successor_scalar(value: f32) -> Result<f32, ScaffoldContractError> {
    if !value.is_finite() {
        return Err(ScaffoldContractError::NonFiniteFloat);
    }
    Ok(value.clamp(0.0, 1.0))
}

fn grounded_successor_features(
    world: &HeadlessWorld,
    world_entity_id: WorldEntityId,
    biology_after: &BiochemistryState,
    physical: alife_core::PhysicalActionOutcome,
    succeeded: bool,
    pain_delta: f32,
) -> Result<Vec<f32>, ScaffoldContractError> {
    let object = world
        .entity(world_entity_id)
        .ok_or(ScaffoldContractError::InvalidId)?;
    let displacement = physical.displacement;
    let body = biology_after.body;
    let contact = match physical.contact {
        PhysicalContactKind::None => 0.0,
        PhysicalContactKind::Touch => 0.2,
        PhysicalContactKind::Collision => 0.4,
        PhysicalContactKind::Blocked => 0.6,
        PhysicalContactKind::Consumed => 0.8,
        PhysicalContactKind::Moved => 1.0,
    };
    let features = [
        bounded_successor_scalar(object.position.x)?,
        bounded_successor_scalar(object.position.y)?,
        bounded_successor_scalar(object.position.z)?,
        bounded_successor_scalar(displacement.x)?,
        bounded_successor_scalar(displacement.y)?,
        bounded_successor_scalar(displacement.z)?,
        unit_successor_scalar(body.energy)?,
        unit_successor_scalar(body.health)?,
        unit_successor_scalar(body.injury)?,
        unit_successor_scalar(body.temperature_stress)?,
        contact,
        if succeeded {
            1.0
        } else {
            0.0
        },
        pain_delta,
    ];
    Ok(features.to_vec())
}

const SINGLE_ACTION_COMPATIBILITY_ADAPTER_VERSION: u16 = 1;
const VOCAL_CHANNEL_PAYLOAD_MAGIC_V1: u32 = 0x5348_5031;

fn channel_command_for_action(
    channel: MotorChannel,
    command: &alife_core::ActionCommand,
) -> Result<ChannelCommand, ScaffoldContractError> {
    let target = (command.target_entity.is_some() || command.target_position.is_some())
        .then(|| ActionTarget::new(command.target_entity, command.target_position));
    ChannelCommand::new(
        channel,
        command.action_id,
        target,
        command.target_position.unwrap_or(Vec3f::ZERO),
        command.intensity,
        command.duration_ticks,
        0.0,
        command.confidence,
        0,
    )
}

fn factorized_motor_channel_for_action(kind: ActionKind) -> Option<MotorChannel> {
    match kind {
        ActionKind::Move => Some(MotorChannel::Locomotion),
        ActionKind::Interact | ActionKind::Write => Some(MotorChannel::Manipulation),
        ActionKind::Vocalize => Some(MotorChannel::Vocal),
        ActionKind::Hold | ActionKind::Rest | ActionKind::Inspect => Some(MotorChannel::Posture),
        ActionKind::Idle | ActionKind::Gesture => None,
    }
}

/// Versioned migration adapter for the old one-action production ABI.
fn compatibility_bundle_for_selected_action_v1(
    organism_id: OrganismId,
    sequence_id: ExperienceSequenceId,
    tick: Tick,
    command: &alife_core::ActionCommand,
) -> Result<MotorCommandBundle, ScaffoldContractError> {
    debug_assert_eq!(SINGLE_ACTION_COMPATIBILITY_ADAPTER_VERSION, 1);
    let channel = match command.kind {
        ActionKind::Idle | ActionKind::Hold | ActionKind::Rest | ActionKind::Inspect => {
            MotorChannel::Posture
        }
        ActionKind::Move => MotorChannel::Locomotion,
        ActionKind::Interact | ActionKind::Write => MotorChannel::Manipulation,
        ActionKind::Vocalize => MotorChannel::Vocal,
        ActionKind::Gesture => MotorChannel::Posture,
    };
    let channel_command = channel_command_for_action(channel, command)?;
    MotorCommandBundle::new(organism_id, sequence_id, tick, vec![channel_command])
}

fn factorized_motor_bundle_for_candidates(
    organism_id: OrganismId,
    sequence_id: ExperienceSequenceId,
    tick: Tick,
    frame: &PerceptionFrame,
    candidate_slots: [u16; GPU_MOTOR_CHANNEL_SLOT_COUNT],
    channels: &[MotorChannel],
    compatibility_command: &alife_core::ActionCommand,
    selected_candidate_index: u16,
    speech_payload: Option<&alife_core::SpeechMotorPayload>,
    speech_prompted: bool,
) -> Result<MotorCommandBundle, ScaffoldContractError> {
    let mut channel_commands = Vec::with_capacity(channels.len());
    for head_channel in channels {
        let slot = match head_channel {
            MotorChannel::Locomotion => 0,
            MotorChannel::Orientation => 1,
            MotorChannel::Manipulation => 2,
            MotorChannel::Vocal => 3,
            MotorChannel::Posture => 4,
            MotorChannel::SpeciesSpecific(_) => 5,
        };
        let encoded = candidate_slots
            .get(slot)
            .copied()
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        if encoded == 0 {
            continue;
        }
        let candidate_index = encoded - 1;
        let candidate = *frame
            .candidates()
            .get(usize::from(candidate_index))
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        let command = candidate.to_command(organism_id, candidate.sensor_confidence)?;
        let channel = factorized_motor_channel_for_action(command.kind)
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        if channel != *head_channel {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let mut channel_command = channel_command_for_action(channel, &command)?;
        if channel == MotorChannel::Vocal && candidate_index == selected_candidate_index {
            if let Some(payload) = speech_payload {
                let mut values = Vec::with_capacity(payload.tokens.len() + 4);
                values.push(VOCAL_CHANNEL_PAYLOAD_MAGIC_V1);
                values.push(u32::from(payload.speech_act.raw()));
                values.push(if speech_prompted { 1 } else { 0 });
                values.push((payload.confidence.raw() * 65_535.0).round() as u32);
                values.extend(payload.tokens.iter().map(|token| u32::from(token.raw())));
                let payload = BoundedMotorPayload::new(values)?;
                channel_command = channel_command.with_payload(payload)?;
            }
        }
        channel_commands.push(channel_command);
    }

    if channel_commands.is_empty() {
        return compatibility_bundle_for_selected_action_v1(
            organism_id,
            sequence_id,
            tick,
            compatibility_command,
        );
    }

    let coordination = (channel_commands.len() > 1).then(|| BoundedCoordinationSummary {
        groups: vec![CoordinationGroup {
            group_id: 0,
            channels: channel_commands
                .iter()
                .map(|command| command.channel)
                .collect(),
        }],
    });
    let bundle = MotorCommandBundle::new(organism_id, sequence_id, tick, channel_commands)?;
    if let Some(coordination) = coordination {
        bundle.with_coordination(coordination)
    } else {
        Ok(bundle)
    }
}

fn apply_prediction_evidence(
    context: &mut CognitiveContextFrame,
    target: &PredictionTargetReceipt,
    errors: &[f32],
) -> Result<f32, ScaffoldContractError> {
    let bounded_errors = errors
        .iter()
        .map(|error| error.abs().clamp(0.0, 1.0))
        .collect::<Vec<_>>();
    let mean_absolute_error = if bounded_errors.is_empty() {
        0.0
    } else {
        bounded_errors.iter().copied().sum::<f32>() / bounded_errors.len() as f32
    };
    context.prediction.source_digest = target.source_digest;
    context.prediction.successor_feature_abi = target.successor_feature_abi;
    context.prediction.prediction_error = bounded_errors
        .iter()
        .copied()
        .map(NormalizedScalar::new)
        .collect::<Result<Vec<_>, _>>()?;
    context.prediction.action_sensitivity =
        NormalizedScalar::new(target.action_sensitivity_score.clamp(0.0, 1.0))?;

    let uncertainty = NormalizedScalar::new(mean_absolute_error)?;
    for summary in &mut context.attention.peripheral_summaries {
        summary.salience.uncertainty = NormalizedScalar::new(
            summary
                .salience
                .uncertainty
                .raw()
                .max(mean_absolute_error),
        )?;
        summary.salience.gap_voltage = NormalizedScalar::new(
            summary.salience.gap_voltage.raw().max(mean_absolute_error),
        )?;
    }
    for salience in &mut context.attention.salience_components {
        salience.uncertainty = uncertainty;
        salience.gap_voltage = NormalizedScalar::new(
            salience.gap_voltage.raw().max(mean_absolute_error),
        )?;
    }
    context.peripheral.summaries = context.attention.peripheral_summaries.clone();
    context.focal.salience = context.attention.salience_components.clone();
    context.gap.gap_voltage = NormalizedScalar::new(
        context.gap.gap_voltage.raw().max(mean_absolute_error),
    )?;
    for gap in &mut context.gap.active_gaps {
        gap.voltage = NormalizedScalar::new(gap.voltage.raw().max(mean_absolute_error))?;
        gap.uncertainty = NormalizedScalar::new(
            gap.uncertainty.raw().max(mean_absolute_error),
        )?;
    }
    context.validate_contract()?;
    Ok(mean_absolute_error)
}

fn cognitive_work_receipt(
    context: &CognitiveContextFrame,
    memory: &MemoryRecallReceipt,
    neural_work: &BrainWorkReceipt,
    schedule_sleep: bool,
) -> Result<CognitiveWorkReceipt, ScaffoldContractError> {
    let memory_ops = u64::from(memory.exact_bucket_reads)
        .saturating_add(u64::from(memory.neighbor_bucket_reads))
        .saturating_add(u64::from(memory.similarity_evaluations));
    CognitiveWorkCounters::new(
        u64::from(neural_work.counters.neuron_updates),
        u64::from(neural_work.counters.synapse_ops),
        0,
        context.attention.budget_receipt.work_units,
        memory_ops,
        context.concept.active_concepts.len() as u64,
        context.gap.active_gaps.len() as u64,
        2,
        0,
        0,
        1,
        if schedule_sleep { 1 } else { 0 },
    )?
    .into_receipt()
}

fn apply_cognitive_work_cost(
    world: &mut HeadlessWorld,
    organism_id: OrganismId,
    receipt: CognitiveWorkReceipt,
    policy: CognitiveWorkCostPolicy,
) -> Result<(), GameAppShellError> {
    let mut records = world.organism_registry().iter().cloned().collect::<Vec<_>>();
    let record = records
        .iter_mut()
        .find(|record| record.organism_id() == organism_id)
        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
    record
        .account_cognitive_work(receipt, policy)
        .map_err(|error| GameAppShellError::InvalidProductionFrontend {
            message: error.to_string(),
        })?;
    world.replace_organism_registry_exact(records)?;
    Ok(())
}

fn seal_prepared_selection_core(
    world: &mut HeadlessWorld,
    residents: &mut BTreeMap<u64, ResidentCognition>,
    sealed_patch_count: usize,
    cognitive_work_cost_policy: CognitiveWorkCostPolicy,
    schedule_sleep: bool,
    prepared: PreparedSealInput,
) -> Result<SealedWorldSelection, GameAppShellError> {
    let PreparedSealInput {
        organism_id,
        world_entity_id,
        frame,
        memory,
        sequence_id,
        outcome_tick,
        mut cognitive_context,
        work,
        pre_action,
        decision,
        motor_bundle,
        speech_payload: _speech_payload,
        speech_prompted: _speech_prompted,
    } = prepared;
    let resident = residents
        .get_mut(&organism_id.raw())
        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
    if resident.next_sequence != sequence_id.raw() {
        return Err(ScaffoldContractError::LearningEvidenceMismatch.into());
    }
    let motor_receipt = world
        .apply_registered_motor_bundle(&motor_bundle, world_entity_id)
        .map_err(|error| match error {
            alife_world::HeadlessMotorTransactionError::Contract(error) => {
                GameAppShellError::Core(error)
            }
            alife_world::HeadlessMotorTransactionError::UnsupportedChannel(_) => {
                GameAppShellError::Core(ScaffoldContractError::InvalidActionDecision)
            }
        })?;
    let physical = motor_receipt.joint.execution;
    let succeeded = motor_receipt.succeeded;
    let successor_features = grounded_successor_features(
        world,
        world_entity_id,
        &motor_receipt.biology_after,
        physical,
        succeeded,
        motor_receipt.body_event.damage,
    )?;
    let prediction_target = PredictionTargetReceipt::for_successor(
        organism_id,
        sequence_id,
        decision.selected_action.action_id,
        frame.tick(),
        frame.frame_digest().0,
        alife_core::SUCCESSOR_FEATURE_ABI_V1,
        successor_features,
    )?;
    let prediction_update = resident.predictor.observe(&prediction_target)?;
    let grounded_prediction_error =
        apply_prediction_evidence(&mut cognitive_context, &prediction_target, &prediction_update.error)?;
    let cognitive_work = cognitive_work_receipt(
        &cognitive_context,
        &memory,
        &work,
        schedule_sleep,
    )?;
    let combined_prediction_error = grounded_prediction_error;
    let mut outcome = PostActionOutcome::new(
        organism_id,
        sequence_id,
        outcome_tick,
        succeeded,
        physical,
        HomeostaticDelta::zero(),
        SignedValence::new(motor_receipt.body_event.reward_outcome)?,
        NormalizedScalar::new(if succeeded { 0.0 } else { 1.0 })?,
        NormalizedScalar::new(motor_receipt.body_event.damage)?,
        SignedValence::new(motor_receipt.body_event.energy)?,
        NormalizedScalar::new(combined_prediction_error)?,
    )?;
    outcome.contradiction_observed = !succeeded;
    outcome = outcome.with_v11_joint(motor_receipt.joint, cognitive_work)?;
    let selected_action_kind = decision.selected_action.kind;
    let selected_action_id = decision.selected_action.action_id;
    let target_entity = decision.selected_action.target_entity;
    let patch = ExperiencePatch::new_v11_with_decision(
        pre_action,
        decision,
        motor_bundle,
        outcome,
        prediction_target,
        cognitive_work,
        cognitive_context,
    )?;
    apply_cognitive_work_cost(
        world,
        organism_id,
        cognitive_work,
        cognitive_work_cost_policy,
    )?;
    resident.language_grounding.observe_sealed(&patch)?;
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
        selected_action_kind: Some(selected_action_kind),
        selected_action_id: Some(selected_action_id),
        target_entity,
        patch_sealed: true,
        patch_sequence_id: Some(sequence_id.raw()),
        patch_success: Some(patch.outcome().success),
        physical_contact: Some(patch.outcome().physical.contact),
        action_failure: None,
        sealed_patch_count: sealed_patch_count.saturating_add(1),
        packed_record_count: 0,
        memory_updates: 0,
        topology_updates: 0,
        learning_updates: 0,
        invalid_or_rejected_action_count: u32::from(!succeeded),
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
    Ok(SealedWorldSelection { summary, patch })
}

impl GpuLiveBrainRuntime {
    /// Creates an ephemeral restore target on the live session's exact GPU
    /// context without exposing adapter selection to the caller.
    pub fn new_staging_like_live(&self) -> Result<GpuClosedLoopBackend, ScaffoldContractError> {
        self.backend.ensure_neural_actions_available()?;
        self.backend.backend().new_staging_like_live()
    }

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
        Self::restore_loaded_save(
            backend,
            durable_manifest,
            loaded_save,
            config.deterministic_seed,
            config.brain_class,
        )
    }

    fn restore_loaded_save(
        backend: GpuClosedLoopBackend,
        durable_manifest: GpuDurableSaveManifest,
        loaded_save: GpuLoadedSaveManifest,
        deterministic_seed: u64,
        brain_class: BrainScaleTier,
    ) -> Result<Self, GameAppShellError> {
        let save = &loaded_save.save;
        if save.config.deterministic_seed != save.deterministic_seed {
            return Err(GameAppShellError::InvalidGraphicalLaunch {
                message: "GPU neural runtime requires matching persisted configuration and save seed",
            });
        }
        validate_replacement_policy(
            save.config.brain_policy.policy,
            save.deterministic_seed,
            save.config.brain_class,
            deterministic_seed,
            brain_class,
        )?;
        let world = save.restore_headless_world()?;
        let store = GpuCheckpointAssetStore::new(durable_manifest.asset_root().to_path_buf())?;
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
            deterministic_seed,
            brain_class,
            &store,
            &save.assets,
            &checkpoints,
        )?;
        for creature in &save.creatures {
            if !runtime.residents.contains_key(&creature.organism_id.raw()) {
                return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
            }
            if creature.brain_class != brain_class {
                return Err(ScaffoldContractError::PhenotypeCompile.into());
            }
        }
        runtime.checkpoint_durability = Some(GpuLiveCheckpointDurability {
            store,
            durable_manifest,
            published: loaded_save,
        });
        let durable_reference = runtime
            .checkpoint_durability
            .as_ref()
            .expect("durability was just installed")
            .durable_reference()?;
        runtime.backend.note_durable_checkpoint(durable_reference)?;
        if requires_checkpoint_reconciliation {
            runtime.persist_sleep_checkpoint_boundary()?;
        }
        Ok(runtime)
    }

    /// Stages a complete durable save in a separate GPU backend, then commits
    /// the replacement only after the world, residents, sidecars, and save
    /// boundary all validate. The caller owns creation and adapter selection
    /// for the staging backend.
    pub fn replace_from_durable_save(
        &mut self,
        backend: GpuClosedLoopBackend,
        durable_manifest: GpuDurableSaveManifest,
    ) -> Result<(), GameAppShellError> {
        let loaded_save = durable_manifest.load()?;
        let deterministic_seed = self.deterministic_seed;
        let brain_class = self.brain_class;
        let preserve_lineage_archive = self.lineage_library.is_some();
        let homeostatic_parameters = self.homeostatic_parameters.clone();
        let cognitive_work_cost_policy = self.cognitive_work_cost_policy;
        let schedule_sleep = self.schedule_sleep;
        let observe_sidecars = self.observe_sidecars;
        let retain_sealed_patch_history = self.retain_sealed_patch_history;
        let archive_learned_capture_policy = self.archive_learned_capture_policy.clone();
        let staged = Self::restore_loaded_save(
            backend,
            durable_manifest,
            loaded_save,
            deterministic_seed,
            brain_class,
        )
        .map(|mut candidate| {
            candidate.homeostatic_parameters = homeostatic_parameters;
            candidate.cognitive_work_cost_policy = cognitive_work_cost_policy;
            candidate.schedule_sleep = schedule_sleep;
            candidate.observe_sidecars = observe_sidecars;
            candidate.retain_sealed_patch_history = retain_sealed_patch_history;
            candidate.archive_learned_capture_policy = archive_learned_capture_policy;
            candidate.lineage_run_id = if preserve_lineage_archive {
                candidate
                    .checkpoint_durability
                    .as_ref()
                    .map(|durability| derive_lineage_run_id(&durability.published))
            } else {
                None
            };
            candidate.archive_birth_manifests.clear();
            candidate
        });

        commit_staged_runtime(self, staged, |live, candidate| {
            let lineage_library = live.lineage_library.take();
            let _old_runtime = std::mem::replace(live, candidate);
            live.lineage_library = lineage_library;
        })
    }

    fn build_live_agent_reset_request(
        &self,
        intent: LiveAgentResetIntent,
    ) -> Result<CuratedFounderResetRequest, CuratedFounderResetRuntimeError> {
        let durability = self
            .checkpoint_durability
            .as_ref()
            .ok_or(CuratedFounderResetRuntimeError::MissingDurability)?;
        let source_run_identity = self
            .lineage_run_id
            .clone()
            .ok_or(CuratedFounderResetRuntimeError::MissingLineageRunId)?;
        let target_population = u32::try_from(intent.final_agents.len()).map_err(|_| {
            CuratedFounderResetRuntimeError::Plan(CuratedFounderResetError::AgentCountMismatch {
                expected: u32::MAX,
                actual: intent.final_agents.len(),
            })
        })?;
        let foundation_asset = FoundationWeightAsset::builtin_nano512_v1(self.sensor_profile)
            .map_err(|_| {
                CuratedFounderResetRuntimeError::Plan(CuratedFounderResetError::FoundationMismatch)
            })?;
        let foundation_manifest = foundation_asset.manifest();
        let foundation = FoundationGeneticIdentity::new(
            foundation_manifest.foundation_id().raw(),
            foundation_manifest.foundation_version().raw() as u16,
            foundation_manifest.compatibility_family_id().raw(),
            BrainCapacityClass::N512_ID,
        )
        .map_err(|_| {
            CuratedFounderResetRuntimeError::Plan(CuratedFounderResetError::FoundationMismatch)
        })?;
        let save = &durability.published.save;
        Ok(CuratedFounderResetRequest {
            policy_label: Some(CURATED_FOUNDER_RESET_POLICY.to_string()),
            source_save_identity: save.save_id.clone(),
            source_save_label: format!("durable-save:{}", save.save_id),
            source_save_seed: save.deterministic_seed,
            world_seed: save.world.seed,
            restored_tick: save.world.tick,
            target_population,
            sensor_profile: self.sensor_profile,
            foundation,
            foundation_content_digest: foundation_asset.digest(),
            source_run_identity,
            final_agents: intent.final_agents,
        })
    }

    pub(crate) fn attempt_live_agent_reset(
        &mut self,
        intent: LiveAgentResetIntent,
    ) -> Result<CuratedFounderResetAttempt, CuratedFounderResetRuntimeError> {
        let request = self.build_live_agent_reset_request(intent)?;
        self.attempt_curated_founder_reset(request)
    }

    pub(crate) fn attempt_curated_founder_reset(
        &mut self,
        request: CuratedFounderResetRequest,
    ) -> Result<CuratedFounderResetAttempt, CuratedFounderResetRuntimeError> {
        let result = attempt_curated_founder_reset_with_owned_authorities(
            &mut self.checkpoint_durability,
            &mut self.lineage_library,
            self.lineage_run_id.as_deref(),
            &mut self.world,
            &mut self.retained_curated_founder_operation,
            &mut self.retained_curated_founder_gpu_residency_plan,
            Some(request),
        )?;
        self.note_curated_founder_durable_checkpoint(&result)?;
        if self
            .retained_curated_founder_gpu_residency_plan
            .as_ref()
            .is_some_and(|plan| matches!(plan.state, CuratedFounderGpuResidencyState::Pending))
        {
            let receipt = self.commit_retained_curated_founder_gpu_residency(Some(&result))?;
            self.retained_curated_founder_gpu_residency_receipt = Some(receipt);
            self.curated_first_tick_pending = true;
        }
        Ok(result)
    }

    pub(crate) fn retry_curated_founder_reset(
        &mut self,
    ) -> Result<CuratedFounderResetAttempt, CuratedFounderResetRuntimeError> {
        let result = attempt_curated_founder_reset_with_owned_authorities(
            &mut self.checkpoint_durability,
            &mut self.lineage_library,
            self.lineage_run_id.as_deref(),
            &mut self.world,
            &mut self.retained_curated_founder_operation,
            &mut self.retained_curated_founder_gpu_residency_plan,
            None,
        )?;
        self.note_curated_founder_durable_checkpoint(&result)?;
        if self
            .retained_curated_founder_gpu_residency_plan
            .as_ref()
            .is_some_and(|plan| matches!(plan.state, CuratedFounderGpuResidencyState::Pending))
        {
            let receipt = self.commit_retained_curated_founder_gpu_residency(Some(&result))?;
            self.retained_curated_founder_gpu_residency_receipt = Some(receipt);
            self.curated_first_tick_pending = true;
        }
        Ok(result)
    }

    /// Consumes the retained exact 4a projection only after every app-side
    /// resident and sidecar candidate is ready. The four maps publish only
    /// after the backend returns a completed residency receipt.
    pub(crate) fn commit_retained_curated_founder_gpu_residency(
        &mut self,
        publication: Option<&CuratedFounderResetAttempt>,
    ) -> Result<GpuCuratedResidencyReceipt, CuratedFounderResetRuntimeError> {
        let plan = self
            .retained_curated_founder_gpu_residency_plan
            .as_ref()
            .ok_or(CuratedFounderResetRuntimeError::NoRetainedOperation)?
            .clone();
        if !matches!(plan.state, CuratedFounderGpuResidencyState::Pending)
            || curated_founder_gpu_residency_plan_fingerprint(&plan) != plan.fingerprint
        {
            return Err(CuratedFounderResetRuntimeError::ResidencyPlanMismatch);
        }

        let mut candidate_residents = BTreeMap::new();
        let mut candidate_memories = BTreeMap::new();
        let mut candidate_topologies = BTreeMap::new();
        let mut candidate_archive_birth_manifests = BTreeMap::new();
        let mut candidate_handle_keys = Vec::with_capacity(plan.entries.len());
        let mut ordered_entries = Vec::with_capacity(plan.entries.len());
        let profile_identity = SensorProfileIdentity {
            profile_id: self.sensor_profile.into(),
            profile_schema_version: 1,
            sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
        };
        for entry in &plan.entries {
            entry
                .projection
                .validate()
                .map_err(|error| CuratedFounderResetRuntimeError::GpuResidencyPreSubmit {
                    error,
                })?;
            if entry.projection.sensor_profile() != self.sensor_profile {
                return Err(CuratedFounderResetRuntimeError::GpuResidencyPreSubmit {
                    error: ScaffoldContractError::SensorProfileMismatch,
                });
            }
            let phenotype = entry.projection.compiled_phenotype().clone();
            let capacity = BrainCapacityClass::production_for_id(phenotype.brain_class_id())
                .map_err(|error| CuratedFounderResetRuntimeError::GpuResidencyPreSubmit {
                    error,
                })?;
            let compiler_inputs = PhenotypeCompilerInputs::try_new_with_foundation_abi(
                entry.projection.source_brain_genome().clone(),
                &capacity,
                entry.projection.runtime_development_state().clone(),
                entry.projection.sensor_profile(),
                phenotype.foundation_abi().clone(),
            )
            .map_err(|error| CuratedFounderResetRuntimeError::GpuResidencyPreSubmit {
                error,
            })?;
            let verified = PhenotypeCompiler::compile_validated(&compiler_inputs, &capacity)
                .map_err(|error| CuratedFounderResetRuntimeError::GpuResidencyPreSubmit {
                    error,
                })?;
            if verified != phenotype
                || verified.phenotype_hash() != entry.projection.receipt().phenotype_hash()
                || phenotype.foundation_abi().foundation_payload_digest()
                    != Some(entry.projection.foundation_asset_digest())
            {
                return Err(CuratedFounderResetRuntimeError::GpuResidencyPreSubmit {
                    error: ScaffoldContractError::PhenotypeCompile,
                });
            }
            let resident = ResidentCognition {
                phenotype: phenotype.clone(),
                genome: compiler_inputs.genome().clone(),
                development: compiler_inputs.development().clone(),
                compiler_inputs,
                homeostasis: HomeostaticSnapshot::baseline(plan.world_tick),
                sleep_scheduler: GpuSleepScheduler::new(SleepConsolidationConfig::reference())
                    .map_err(|error| {
                        CuratedFounderResetRuntimeError::GpuResidencyPreSubmit { error }
                    })?,
                next_sequence: 1,
                language_grounding: LanguageGroundingLedger::default(),
                life_statistics: PassiveLifeStatistics::new(entry.organism_id, plan.world_tick)
                    .map_err(|error| {
                        CuratedFounderResetRuntimeError::GpuResidencyPreSubmit { error }
                    })?,
                attention_hysteresis: alife_core::HysteresisState::default(),
                predictor: GroundedSuccessorPredictor::default(),
            };
            let raw = entry.organism_id.raw();
            if candidate_residents.insert(raw, resident).is_some()
                || candidate_archive_birth_manifests
                    .insert(raw, entry.archive_birth_manifest_digest)
                    .is_some()
            {
                return Err(CuratedFounderResetRuntimeError::GpuResidencyPreSubmit {
                    error: ScaffoldContractError::BrainOwnershipMismatch,
                });
            }
            candidate_handle_keys.push(raw);
            candidate_memories.insert(
                raw,
                Self::new_memory_sidecar(entry.organism_id, self.sensor_profile).map_err(
                    |error| CuratedFounderResetRuntimeError::GpuResidencyPreSubmit { error },
                )?,
            );
            candidate_topologies.insert(
                raw,
                TopologySidecar::new_profiled(
                    entry.organism_id,
                    profile_identity,
                    TopologicalMapConfig::default(),
                )
                .map_err(|error| CuratedFounderResetRuntimeError::GpuResidencyPreSubmit {
                    error,
                })?,
            );
            ordered_entries.push(GpuCuratedResidencyEntry {
                organism_id: entry.organism_id,
                opaque_target_identity: GpuCuratedResidencyTargetIdentity::new(
                    entry.world_entity_id.raw(),
                ),
                phenotype,
                exact_phenotype_hash: entry.projection.receipt().phenotype_hash(),
                exact_foundation_hash: entry.projection.foundation_asset_digest(),
            });
        }
        let cohort = GpuCuratedResidencyCohort {
            expected_old_generation: self.backend.curated_residency_generation(),
            new_generation_fingerprint: plan.fingerprint,
            ordered_entries,
        };
        let outcome = self.backend.replace_curated_cohort(&cohort);
        let receipt = match outcome {
            GpuCuratedResidencyOutcome::Committed(receipt) => receipt,
            GpuCuratedResidencyOutcome::PreSubmitFailure { error, .. } => {
                return Err(CuratedFounderResetRuntimeError::GpuResidencyPreSubmit { error });
            }
            GpuCuratedResidencyOutcome::Unknown { error, .. } => {
                if let Some(plan) = self.retained_curated_founder_gpu_residency_plan.as_mut() {
                    plan.state = CuratedFounderGpuResidencyState::Unknown;
                }
                return Err(CuratedFounderResetRuntimeError::GpuResidencyUnknown {
                    evidence: publication.map(CuratedFounderResetRuntimeEvidence::from_attempt),
                    error,
                });
            }
        };
        let candidate_handles = candidate_handle_keys
            .into_iter()
            .zip(receipt.ordered_residents.iter().map(|resident| resident.handle))
            .collect();
        self.handles = candidate_handles;
        self.residents = candidate_residents;
        self.memories = candidate_memories;
        self.topologies = candidate_topologies;
        self.retained_learning.clear();
        self.archive_birth_manifests = candidate_archive_birth_manifests;
        if let Some(plan) = self.retained_curated_founder_gpu_residency_plan.as_mut() {
            plan.state = CuratedFounderGpuResidencyState::Committed;
        }
        Ok(receipt)
    }

    fn note_curated_founder_durable_checkpoint(
        &mut self,
        result: &CuratedFounderResetAttempt,
    ) -> Result<(), CuratedFounderResetRuntimeError> {
        if !matches!(
            result.publication_status(),
            CuratedFounderPublicationStatus::Published
                | CuratedFounderPublicationStatus::AlreadyApplied
        ) {
            return Ok(());
        }
        let durability = self
            .checkpoint_durability
            .as_ref()
            .ok_or(CuratedFounderResetRuntimeError::MissingDurability)?;
        let durable_reference = match durability.durable_reference() {
            Ok(reference) => reference,
            Err(error) => {
                self.retained_curated_founder_operation = None;
                if let Some(plan) = self.retained_curated_founder_gpu_residency_plan.as_mut() {
                    plan.state = CuratedFounderGpuResidencyState::Unknown;
                }
                let mut evidence = CuratedFounderResetRuntimeEvidence::from_attempt(result);
                evidence.gpu_residency = CuratedFounderGpuResidencyState::Unknown;
                return Err(CuratedFounderResetRuntimeError::DurableCheckpointNotification {
                    evidence,
                    error,
                });
            }
        };
        if let Err(error) = self.backend.note_durable_checkpoint(durable_reference) {
            self.retained_curated_founder_operation = None;
            if let Some(plan) = self.retained_curated_founder_gpu_residency_plan.as_mut() {
                plan.state = CuratedFounderGpuResidencyState::Unknown;
            }
            let mut evidence = CuratedFounderResetRuntimeEvidence::from_attempt(result);
            evidence.gpu_residency = CuratedFounderGpuResidencyState::Unknown;
            return Err(CuratedFounderResetRuntimeError::DurableCheckpointNotification {
                evidence,
                error: error.into(),
            });
        }
        Ok(())
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
        Self::new_profiled_with_parameters(
            backend,
            world,
            deterministic_seed,
            brain_class,
            sensor_profile,
            GpuLiveRuntimeConstructionOptions::production(),
        )
    }

    #[cfg(feature = "bevy-app")]
    #[doc(hidden)]
    pub fn run_curated_first_gpu_action_for_test(
        save_path: impl AsRef<std::path::Path>,
    ) -> Result<CuratedFirstGpuActionTestEvidence, GameAppShellError> {
        let invalid = |message: &str| GameAppShellError::InvalidProductionFrontend {
            message: message.to_string(),
        };
        let save = PortableSaveFile::from_json_file(save_path.as_ref())?;
        let world = save.restore_headless_world()?;
        let backend = GpuClosedLoopBackend::new_required(
            alife_gpu_backend::GpuRuntimeProfile::production_v1(),
        )
        .map_err(|error| GameAppShellError::NeuralBackendUnavailable {
            message: error.to_string(),
        })?;
        let mut runtime = Self::new_causal_acceptance_profiled(
            backend,
            world,
            save.deterministic_seed,
            save.config.brain_class,
            SensorProfile::PrivilegedAffordanceV1,
        )?;
        let bindings = runtime.world.organism_entity_ids();
        if bindings.len() < 2 {
            return Err(invalid(
                "curated first-tick test seam requires two registered organisms",
            ));
        }
        let foundation_asset = FoundationWeightAsset::builtin_nano512_v1(runtime.sensor_profile)
            .map_err(|error| invalid(&error.to_string()))?;
        let foundation_manifest = foundation_asset.manifest();
        let foundation = FoundationGeneticIdentity::new(
            foundation_manifest.foundation_id().raw(),
            foundation_manifest.foundation_version().raw() as u16,
            foundation_manifest.compatibility_family_id().raw(),
            BrainCapacityClass::N512_ID,
        )
        .map_err(|error| invalid(&error.to_string()))?;
        let final_agents = bindings
            .iter()
            .take(2)
            .enumerate()
            .map(
                |(slot, (organism_id, world_entity_id))| CuratedFounderAgentInput {
                    world_entity_id: *world_entity_id,
                    organism_id: Some(*organism_id),
                    final_population_slot: u32::try_from(slot)
                        .expect("the two-row test cohort fits in a population slot"),
                    legacy_genome_id: None,
                },
            )
            .collect::<Vec<_>>();
        let source_run_identity = format!("task-3.2b4c-test-seam:{}", save.save_id);
        let founder_plan = plan_curated_founder_reset(&CuratedFounderResetRequest {
            policy_label: Some(CURATED_FOUNDER_RESET_POLICY.to_string()),
            source_save_identity: save.save_id.clone(),
            source_save_label: format!("test-save:{}", save.save_id),
            source_save_seed: save.deterministic_seed,
            world_seed: save.world.seed,
            restored_tick: runtime.world.tick(),
            target_population: 2,
            sensor_profile: runtime.sensor_profile,
            foundation,
            foundation_content_digest: foundation_asset.digest(),
            source_run_identity: source_run_identity.clone(),
            final_agents,
        })
        .map_err(|error| invalid(&error.to_string()))?;
        let bundle = materialize_curated_founder_bundle(&founder_plan)
            .map_err(|error| invalid(&error.to_string()))?;
        let mut plan = CuratedFounderGpuResidencyPlan {
            state: CuratedFounderGpuResidencyState::Pending,
            final_save_digest: format!("test-save-digest:{}", save.save_id),
            candidate_world_signature: runtime.world.canonical_signature_digest()?,
            world_seed: save.world.seed,
            world_tick: runtime.world.tick(),
            source_run_identity,
            entries: bundle
                .entries
                .into_iter()
                .map(|entry| {
                    let mut digest_bytes = [0; 32];
                    digest_bytes[..4]
                        .copy_from_slice(&entry.plan_entry.final_population_slot.to_le_bytes());
                    CuratedFounderGpuResidencyPlanEntry {
                        final_population_slot: entry.plan_entry.final_population_slot,
                        world_entity_id: entry.plan_entry.world_entity_id,
                        organism_id: entry.plan_entry.organism_id,
                        lineage_id: entry.plan_entry.lineage_id,
                        archive_birth_manifest_digest: Blake3Digest::from_bytes(digest_bytes),
                        projection: entry.projection,
                    }
                })
                .collect(),
            fingerprint: [0; 4],
        };
        plan.fingerprint = curated_founder_gpu_residency_plan_fingerprint(&plan);

        runtime.curated_first_tick_pending = true;
        let baseline_tick = runtime.world.tick();
        let baseline_handles = runtime.handles.clone();
        let baseline_resident_keys = runtime.residents.keys().copied().collect::<Vec<_>>();
        let baseline_memory_keys = runtime.memories.keys().copied().collect::<Vec<_>>();
        let baseline_topology_keys = runtime.topologies.keys().copied().collect::<Vec<_>>();
        let mut residency_gate_rejections = 0;
        let mut record_rejection = |runtime: &Self| -> Result<(), GameAppShellError> {
            if runtime.curated_first_tick_residency_gate().is_ok()
                || runtime.world.tick() != baseline_tick
                || runtime.handles != baseline_handles
                || runtime.residents.keys().copied().collect::<Vec<_>>() != baseline_resident_keys
                || runtime.memories.keys().copied().collect::<Vec<_>>() != baseline_memory_keys
                || runtime.topologies.keys().copied().collect::<Vec<_>>() != baseline_topology_keys
            {
                return Err(invalid(
                    "curated first-tick residency rejection mutated runtime state",
                ));
            }
            residency_gate_rejections = residency_gate_rejections.saturating_add(1);
            Ok(())
        };

        runtime.retained_curated_founder_gpu_residency_plan = None;
        runtime.retained_curated_founder_gpu_residency_receipt = None;
        record_rejection(&runtime)?;

        let mut invalid_plan = plan.clone();
        runtime.curated_first_tick_pending = false;
        invalid_plan.state = CuratedFounderGpuResidencyState::NotStarted;
        runtime.retained_curated_founder_gpu_residency_plan = Some(invalid_plan.clone());
        record_rejection(&runtime)?;

        invalid_plan.state = CuratedFounderGpuResidencyState::Pending;
        runtime.retained_curated_founder_gpu_residency_plan = Some(invalid_plan.clone());
        record_rejection(&runtime)?;

        invalid_plan.state = CuratedFounderGpuResidencyState::Unknown;
        runtime.retained_curated_founder_gpu_residency_plan = Some(invalid_plan);
        record_rejection(&runtime)?;

        runtime.retained_curated_founder_gpu_residency_plan = Some(plan);
        let receipt = runtime
            .commit_retained_curated_founder_gpu_residency(None)
            .map_err(|error| invalid(&error.to_string()))?;
        runtime.retained_curated_founder_gpu_residency_receipt = Some(receipt.clone());
        runtime.curated_first_tick_pending = true;

        let mut mismatched_receipt = receipt.clone();
        mismatched_receipt.generation_fingerprint[0] ^= 1;
        runtime.retained_curated_founder_gpu_residency_receipt = Some(mismatched_receipt);
        if runtime.curated_first_tick_residency_gate().is_ok()
            || runtime.world.tick() != baseline_tick
        {
            return Err(invalid(
                "mismatched curated first-tick receipt was admitted",
            ));
        }
        residency_gate_rejections = residency_gate_rejections.saturating_add(1);
        runtime.retained_curated_founder_gpu_residency_receipt = Some(receipt.clone());

        let first_resident = receipt
            .ordered_residents
            .first()
            .ok_or_else(|| invalid("curated first-tick receipt is empty"))?;
        let selected_world_entity_id = WorldEntityId(first_resident.opaque_target_identity.raw());
        let selected_organism_id = first_resident.organism_id;
        let pre_action_position = runtime
            .world
            .entity(selected_world_entity_id)
            .ok_or_else(|| invalid("receipt-bound world object is absent"))?
            .position;
        let selection_before = runtime.backend.completed_selection_count();
        let summary = runtime
            .tick()?
            .into_iter()
            .next()
            .ok_or_else(|| invalid("curated first tick returned no summary"))?;
        let gpu_selection_count = runtime
            .backend
            .completed_selection_count()
            .saturating_sub(selection_before);
        let sealed_patch_count = runtime.sealed_patch_count();
        Ok(CuratedFirstGpuActionTestEvidence {
            receipt,
            summary,
            post_action_world: runtime.world_snapshot(),
            pre_action_position,
            selected_world_entity_id,
            selected_organism_id,
            residency_gate_rejections,
            gpu_selection_count,
            sealed_patch_count,
        })
    }

    /// Creates a production GPU runtime whose immutable genetic archives are
    /// committed before each neural slot is allocated.
    #[allow(clippy::too_many_arguments)]
    pub fn new_profiled_archived(
        backend: GpuClosedLoopBackend,
        world: HeadlessWorld,
        deterministic_seed: u64,
        brain_class: BrainScaleTier,
        sensor_profile: SensorProfile,
        archive_config: LineageLibraryConfig,
        source_run_id: impl Into<String>,
        learned_capture_policy: ArchiveLearnedCapturePolicy,
    ) -> Result<Self, GameAppShellError> {
        let library = LineageLibrary::open(archive_config)?;
        Self::new_profiled_with_parameters_and_archive(
            backend,
            world,
            deterministic_seed,
            brain_class,
            sensor_profile,
            GpuLiveRuntimeConstructionOptions::production(),
            Some((library, source_run_id.into(), learned_capture_policy)),
        )
    }

    pub(crate) fn new_benchmark_profiled(
        backend: GpuClosedLoopBackend,
        world: HeadlessWorld,
        deterministic_seed: u64,
        brain_class: BrainScaleTier,
        sensor_profile: SensorProfile,
    ) -> Result<Self, GameAppShellError> {
        let mut parameters = HomeostaticParameters::reference();
        parameters.hunger_drift_per_update = 0.0;
        parameters.fatigue_drift_per_update = 0.0;
        parameters.loneliness_drift_per_update = 0.0;
        parameters.curiosity_drift_per_update = 0.0;
        parameters.reproductive_drift_per_update = 0.0;
        parameters.brain_atp_drain_per_update = 0.0;
        parameters.sleep_pressure_drift_per_update = 0.0;
        parameters.catatonia_brain_atp_threshold = 0.0;
        parameters.fatigue_sleep_threshold = 1.0;
        parameters.sleep_pressure_threshold = 1.0;
        parameters.pain_frustration_threshold = 1.0;
        parameters.validate_contract()?;
        Self::new_profiled_with_parameters(
            backend,
            world,
            deterministic_seed,
            brain_class,
            sensor_profile,
            GpuLiveRuntimeConstructionOptions::benchmark(parameters),
        )
    }

    /// Runs the production neural, memory, action, learning, and work-cost path
    /// as one fixed continuous-wake lab protocol. Natural ATP/sleep behavior is
    /// proven separately by Slice D; this profile prevents sleep from
    /// interrupting an acceptance run's exact sealed causal transactions.
    pub(crate) fn new_causal_acceptance_profiled(
        backend: GpuClosedLoopBackend,
        world: HeadlessWorld,
        deterministic_seed: u64,
        brain_class: BrainScaleTier,
        sensor_profile: SensorProfile,
    ) -> Result<Self, GameAppShellError> {
        Self::new_profiled_with_parameters(
            backend,
            world,
            deterministic_seed,
            brain_class,
            sensor_profile,
            GpuLiveRuntimeConstructionOptions::causal_acceptance(),
        )
    }

    #[cfg(feature = "gpu-tests")]
    pub(crate) fn new_soak_profiled(
        backend: GpuClosedLoopBackend,
        world: HeadlessWorld,
        deterministic_seed: u64,
        brain_class: BrainScaleTier,
        sensor_profile: SensorProfile,
    ) -> Result<Self, GameAppShellError> {
        Self::new_profiled_with_parameters(
            backend,
            world,
            deterministic_seed,
            brain_class,
            sensor_profile,
            GpuLiveRuntimeConstructionOptions::soak(),
        )
    }

    fn new_profiled_with_parameters(
        backend: GpuClosedLoopBackend,
        world: HeadlessWorld,
        deterministic_seed: u64,
        brain_class: BrainScaleTier,
        sensor_profile: SensorProfile,
        options: GpuLiveRuntimeConstructionOptions,
    ) -> Result<Self, GameAppShellError> {
        Self::new_profiled_with_parameters_and_archive(
            backend,
            world,
            deterministic_seed,
            brain_class,
            sensor_profile,
            options,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new_profiled_with_parameters_and_archive(
        backend: GpuClosedLoopBackend,
        world: HeadlessWorld,
        deterministic_seed: u64,
        brain_class: BrainScaleTier,
        sensor_profile: SensorProfile,
        options: GpuLiveRuntimeConstructionOptions,
        lineage_archive: Option<(LineageLibrary, String, ArchiveLearnedCapturePolicy)>,
    ) -> Result<Self, GameAppShellError> {
        if deterministic_seed == 0 || brain_class.neuron_count().is_none() {
            return Err(GameAppShellError::Core(
                ScaffoldContractError::PhenotypeCompile,
            ));
        }
        options.homeostatic_parameters.validate_contract()?;
        options.cognitive_work_cost_policy.validate_contract()?;
        let (lineage_library, lineage_run_id, archive_learned_capture_policy) =
            match lineage_archive {
                Some((library, run_id, policy)) => (Some(library), Some(run_id), policy),
                None => (None, None, ArchiveLearnedCapturePolicy::GeneticOnly),
            };
        let mut runtime = Self {
            backend: GpuAuthoritativeSession::new(backend, GpuSessionConsumerKind::Gameplay),
            handles: BTreeMap::new(),
            residents: BTreeMap::new(),
            memories: BTreeMap::new(),
            topologies: BTreeMap::new(),
            retained_learning: BTreeMap::new(),
            world,
            deterministic_seed,
            brain_class,
            sensor_profile,
            homeostatic_parameters: options.homeostatic_parameters,
            cognitive_work_cost_policy: options.cognitive_work_cost_policy,
            schedule_sleep: options.schedule_sleep,
            sealed_patches: Vec::new(),
            sealed_patch_count: 0,
            last_sealed_patches: Vec::new(),
            observe_sidecars: options.observe_sidecars,
            retain_sealed_patch_history: options.retain_sealed_patch_history,
            last_learning_receipts: Vec::new(),
            last_activity_work_receipts: Vec::new(),
            last_cognitive_work_receipts: Vec::new(),
            last_memory_recall_receipts: Vec::new(),
            last_memory_update_receipts: Vec::new(),
            last_cognitive_context_digests: Vec::new(),
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
            lineage_library,
            lineage_run_id,
            retained_curated_founder_operation: None,
            retained_curated_founder_gpu_residency_plan: None,
            retained_curated_founder_gpu_residency_receipt: None,
            curated_first_tick_pending: false,
            archive_learned_capture_policy,
            archive_birth_manifests: BTreeMap::new(),
            archive_retirement_receipts: BTreeMap::new(),
            presentation_retirements: BTreeSet::new(),
            #[cfg(test)]
            forced_retirement_post_receipt_failure: false,
            #[cfg(test)]
            retirement_backend_removal_count: 0,
            #[cfg(test)]
            forced_late_advance_failure: false,
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
        let live_bindings = world
            .organism_entity_ids()
            .into_iter()
            .map(|(organism_id, world_entity_id)| {
                (organism_id.raw(), (organism_id, world_entity_id))
            })
            .collect::<BTreeMap<_, _>>();
        let live_ids = live_bindings.keys().copied().collect::<BTreeSet<_>>();
        if checkpoint_index.keys().any(|raw| !live_ids.contains(raw)) {
            return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
        }
        let world_tick = world.tick();
        let mut runtime = Self {
            backend: GpuAuthoritativeSession::new(backend, GpuSessionConsumerKind::Gameplay),
            handles: BTreeMap::new(),
            residents: BTreeMap::new(),
            memories: BTreeMap::new(),
            topologies: BTreeMap::new(),
            retained_learning: BTreeMap::new(),
            world,
            deterministic_seed,
            brain_class,
            sensor_profile,
            homeostatic_parameters: HomeostaticParameters::reference(),
            cognitive_work_cost_policy: GpuLiveRuntimeConstructionOptions::production()
                .cognitive_work_cost_policy,
            schedule_sleep: true,
            sealed_patches: Vec::new(),
            sealed_patch_count: 0,
            last_sealed_patches: Vec::new(),
            observe_sidecars: true,
            retain_sealed_patch_history: true,
            last_learning_receipts: Vec::new(),
            last_activity_work_receipts: Vec::new(),
            last_cognitive_work_receipts: Vec::new(),
            last_memory_recall_receipts: Vec::new(),
            last_memory_update_receipts: Vec::new(),
            last_cognitive_context_digests: Vec::new(),
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
            lineage_library: None,
            lineage_run_id: None,
            retained_curated_founder_operation: None,
            retained_curated_founder_gpu_residency_plan: None,
            retained_curated_founder_gpu_residency_receipt: None,
            curated_first_tick_pending: false,
            archive_learned_capture_policy: ArchiveLearnedCapturePolicy::GeneticOnly,
            archive_birth_manifests: BTreeMap::new(),
            archive_retirement_receipts: BTreeMap::new(),
            presentation_retirements: BTreeSet::new(),
            #[cfg(test)]
            forced_retirement_post_receipt_failure: false,
            #[cfg(test)]
            retirement_backend_removal_count: 0,
            #[cfg(test)]
            forced_late_advance_failure: false,
        };
        let mut tracked_object_states = Vec::new();
        for raw in live_ids {
            let organism_id = OrganismId(raw);
            let (bound_organism_id, world_entity_id) = live_bindings
                .get(&raw)
                .copied()
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            if bound_organism_id != organism_id {
                return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
            }
            let record = runtime
                .world
                .organism_registry()
                .get(organism_id)
                .cloned()
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            if let Some(state) = checkpoint_index.get(&raw).copied() {
                let restored = store.restore_brain(&mut runtime.backend, manifest, state)?;
                let handle = restored.receipt.handle;
                let mut pending_eligibility_for_cleanup = restored.receipt.pending_eligibility;
                let install_result: Result<_, GameAppShellError> = (|| {
                    let authority = restore_resident_authority_from_record(
                        &record,
                        organism_id,
                        world_entity_id,
                        world_tick,
                        brain_class,
                        sensor_profile,
                        Some(ResidentCheckpointMetadata {
                            organism_id: state.organism_id,
                            phenotype_hash: state.phenotype_hash,
                            capacity_class_id: state.capacity_class_id,
                            checkpoint_tick: state.checkpoint_tick,
                            phenotype: &restored.phenotype,
                            compiler_inputs: &restored.compiler_inputs,
                        }),
                    )?;
                    let retained_sequence = restored
                        .retained_learning
                        .as_ref()
                        .map(|recovery| recovery.sealed_patch.pre_action().sequence_id);
                    let pending_sequence = restored
                        .pending_transaction
                        .as_ref()
                        .map(|builder| builder.pending_decision().map(|(pre, _)| pre.sequence_id))
                        .transpose()?;
                    let mut discarded_eligibility = None;
                    if restored.retained_learning.is_none() {
                        if let Some(receipt) = pending_eligibility_for_cleanup.as_ref().cloned() {
                            let discard = runtime
                                .backend
                                .discard_pending_eligibility(handle, receipt.identity())?;
                            pending_eligibility_for_cleanup = None;
                            discarded_eligibility = Some(discard);
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
                                None => restored
                                    .memory
                                    .latest_durable_sequence_raw()
                                    .checked_add(1)
                                    .ok_or(ScaffoldContractError::ScalarOutOfRange)?
                                    .max(1),
                            },
                        },
                    };
                    let sleep_scheduler = GpuSleepScheduler::restore(
                        SleepConsolidationConfig::reference(),
                        restored.sleep,
                    )?;
                    let life_statistics = restored
                        .life_statistics
                        .unwrap_or(PassiveLifeStatistics::new(OrganismId(raw), world_tick)?);
                    let retained_learning = if let Some(recovery) = restored.retained_learning {
                        let pending = pending_eligibility_for_cleanup
                            .as_ref()
                            .cloned()
                            .ok_or(ScaffoldContractError::LearningEvidenceMismatch)?;
                        Some(RetainedLearningRecovery {
                            handle,
                            pending,
                            sealed_patch: recovery.sealed_patch,
                            attempts: recovery.attempts,
                            last_error: RetainedLearningErrorCode::from_slug(
                                &recovery.last_error_code,
                            )?,
                        })
                    } else {
                        None
                    };
                    let resident = ResidentCognition {
                        phenotype: authority.phenotype.clone(),
                        genome: authority.genome.clone(),
                        development: authority.development.clone(),
                        compiler_inputs: authority.compiler_inputs.clone(),
                        homeostasis: authority.biochemistry.homeostasis,
                        sleep_scheduler,
                        next_sequence,
                        language_grounding: restored.language_grounding,
                        life_statistics,
                        attention_hysteresis: alife_core::HysteresisState::default(),
                        predictor: GroundedSuccessorPredictor::default(),
                    };
                    Ok((
                        resident,
                        restored.memory,
                        restored.topology,
                        restored.tracked_objects,
                        retained_learning,
                        discarded_eligibility,
                    ))
                })();
                let (
                    resident,
                    memory,
                    topology,
                    tracked_objects,
                    retained_learning,
                    discarded_eligibility,
                ) = match install_result {
                    Ok(install) => install,
                    Err(error) => {
                        let cleanup_result = cleanup_restored_gpu_handle(
                            &mut runtime.backend,
                            handle,
                            pending_eligibility_for_cleanup,
                        );
                        if let Err(cleanup_error) = cleanup_result {
                            return Err(GameAppShellError::InvalidProductionFrontend {
                                message: format!(
                                    "resident restore failed: {error}; cleanup failed: {cleanup_error}"
                                ),
                            });
                        }
                        return Err(error);
                    }
                };
                runtime.handles.insert(raw, handle);
                runtime.residents.insert(raw, resident);
                runtime.memories.insert(raw, memory);
                runtime.topologies.insert(raw, topology);
                tracked_object_states.push(tracked_objects);
                if let Some(discard) = discarded_eligibility {
                    runtime.last_eligibility_discard_receipts.push(discard);
                }
                if let Some(recovery) = retained_learning {
                    runtime.retained_learning.insert(
                        raw,
                        recovery,
                    );
                }
            } else {
                let (_, resident) = Self::compile_birth(
                    &runtime.world,
                    brain_class,
                    sensor_profile,
                    organism_id,
                )?;
                let memory = Self::new_memory_sidecar(organism_id, sensor_profile)?;
                let topology = TopologySidecar::new_profiled(
                    organism_id,
                    saved_profile,
                    TopologicalMapConfig::default(),
                )?;
                let handle = runtime
                    .backend
                    .insert_brain(organism_id, resident.phenotype.clone())?;
                runtime.handles.insert(raw, handle);
                runtime.residents.insert(raw, resident);
                runtime.memories.insert(raw, memory);
                runtime.topologies.insert(raw, topology);
            }
        }
        if let Err(error) = runtime
            .world
            .restore_tracked_object_states(tracked_object_states)
        {
            let cleanup_targets = runtime
                .handles
                .iter()
                .map(|(raw, handle)| {
                    (
                        *handle,
                        runtime
                            .retained_learning
                            .get(raw)
                            .map(|recovery| recovery.pending),
                    )
                })
                .collect::<Vec<_>>();
            let mut cleanup_error = None;
            for (handle, pending_eligibility) in cleanup_targets {
                if let Err(error) = cleanup_restored_gpu_handle(
                    &mut runtime.backend,
                    handle,
                    pending_eligibility,
                ) {
                    if cleanup_error.is_none() {
                        cleanup_error = Some(error);
                    }
                }
            }
            if let Some(cleanup_error) = cleanup_error {
                return Err(cleanup_error);
            }
            return Err(error.into());
        }
        Ok(runtime)
    }

    #[cfg(feature = "gpu-tests")]
    pub(crate) fn restore_soak_with_checkpoints(
        backend: GpuClosedLoopBackend,
        world: HeadlessWorld,
        deterministic_seed: u64,
        brain_class: BrainScaleTier,
        store: &GpuCheckpointAssetStore,
        manifest: &AssetManifest,
        checkpoints: &[GpuBrainSaveState],
    ) -> Result<Self, GameAppShellError> {
        let mut runtime = Self::restore_with_checkpoints(
            backend,
            world,
            deterministic_seed,
            brain_class,
            store,
            manifest,
            checkpoints,
        )?;
        runtime.retain_sealed_patch_history = false;
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
        Ok(store.capture_brain(
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
                language_grounding: &resident.language_grounding,
                life_statistics: &resident.life_statistics,
                retained_learning,
            },
        )?)
    }

    /// Attaches the runtime-owned durable save boundary to an already
    /// materialized canonical world. The base save is validated and published
    /// through the existing portable-save manifest before the live runtime
    /// adopts its content-addressed store and durable reference.
    pub fn attach_durable_checkpoint_boundary(
        &mut self,
        save_path: impl AsRef<Path>,
        asset_root: impl AsRef<Path>,
        mut base: PortableSaveFile,
    ) -> Result<(), GameAppShellError> {
        if self.checkpoint_durability.is_some() {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "GPU runtime already has a durable save boundary".to_string(),
            });
        }
        validate_replacement_policy(
            base.config.brain_policy.policy,
            base.deterministic_seed,
            base.config.brain_class,
            self.deterministic_seed,
            self.brain_class,
        )?;
        let base_world = base.restore_headless_world()?;
        if base.deterministic_seed != self.deterministic_seed
            || base.config.deterministic_seed != self.deterministic_seed
            || base_world.seed() != self.world.seed()
            || base_world.tick() != self.world.tick()
        {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "durable checkpoint base seed or tick does not match the canonical live world"
                    .to_string(),
            });
        }
        let live_ids = self.handles.keys().copied().collect::<BTreeSet<_>>();
        let saved_ids = base
            .creatures
            .iter()
            .map(|creature| creature.organism_id.raw())
            .collect::<BTreeSet<_>>();
        if saved_ids != live_ids || saved_ids.len() != base.creatures.len() {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "durable checkpoint base does not cover the live GPU residents"
                    .to_string(),
            });
        }
        if base
            .creatures
            .iter()
            .any(|creature| creature.brain_class != self.brain_class)
        {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "durable checkpoint base contains an incompatible brain class"
                    .to_string(),
            });
        }
        // The full canonical signature also binds runtime-only tracked-object
        // state. PortableSaveFile normalizes that state through WorldSaveState,
        // so compare the supplied durable representation with the exact
        // normalized representation expected from the live world. This keeps
        // persisted organisms, archive identity, objects, ecology, habitats,
        // and counters strict without rejecting a valid save for transient
        // state that the save authority does not persist.
        let mut normalized_base = base.clone();
        normalized_base.replace_headless_world_snapshot(&self.world)?;
        if normalized_base.world != base.world {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "durable checkpoint base does not match the canonical live world"
                    .to_string(),
            });
        }
        base.replace_headless_world_snapshot(&self.world)?;

        let save_path = save_path.as_ref();
        let asset_root = asset_root.as_ref();
        GpuDurableSaveManifest::publish_snapshot(save_path, asset_root, &base)?;
        let durable_manifest = GpuDurableSaveManifest::open(save_path, asset_root)?;
        let published = durable_manifest.load()?;
        let store = GpuCheckpointAssetStore::new(durable_manifest.asset_root().to_path_buf())?;
        let durability = GpuLiveCheckpointDurability {
            store,
            durable_manifest,
            published,
        };
        let durable_reference = durability.durable_reference()?;
        self.backend.note_durable_checkpoint(durable_reference)?;
        self.checkpoint_durability = Some(durability);
        Ok(())
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
        self.add_missing_checkpoint_creature_summaries(&mut replacement)?;
        replacement.replace_headless_world_snapshot(&self.world)?;
        let mut manifest_entries = Vec::new();
        for (&raw, &handle) in &self.handles {
            let organism_id = OrganismId(raw);
            let record = self
                .world
                .organism_registry()
                .get(organism_id)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let authoritative_age = record.age_at(checkpoint_tick)?;
            let resident = self
                .residents
                .get(&raw)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            if resident.homeostasis != record.biochemistry().homeostasis
                || resident.homeostasis.tick != checkpoint_tick
                || resident.development.age_ticks != authoritative_age
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
                    language_grounding: &resident.language_grounding,
                    life_statistics: &resident.life_statistics,
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
            let canonical_biochemistry = self
                .world
                .organism_registry()
                .get(organism_id)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                .biochemistry()
                .clone();
            let creature = replacement
                .creatures
                .iter_mut()
                .find(|creature| creature.organism_id.raw() == raw)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            if creature.brain_class != self.brain_class {
                return Err(ScaffoldContractError::PhenotypeCompile.into());
            }
            creature.development_tick = canonical_biochemistry.development.last_update_tick;
            creature.mind.tick = canonical_biochemistry.tick;
            creature.mind.homeostasis = canonical_biochemistry.homeostasis;
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

    fn add_missing_checkpoint_creature_summaries(
        &self,
        replacement: &mut PortableSaveFile,
    ) -> Result<(), GameAppShellError> {
        let live_ids = self.handles.keys().copied().collect::<BTreeSet<_>>();
        for raw in live_ids {
            if replacement
                .creatures
                .iter()
                .any(|creature| creature.organism_id.raw() == raw)
            {
                continue;
            }
            let organism_id = OrganismId(raw);
            let record = self
                .world
                .organism_registry()
                .get(organism_id)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let resident = self
                .residents
                .get(&raw)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let summary = checkpoint_creature_save_state(
                replacement,
                record,
                resident,
                self.brain_class,
            )?;
            replacement.creatures.push(summary);
        }
        replacement
            .creatures
            .sort_by_key(|creature| creature.organism_id.raw());
        Ok(())
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
        let durable_reference = result?;
        self.backend.note_durable_checkpoint(durable_reference)?;
        Ok(())
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
        let durable_reference = result?;
        self.backend.note_durable_checkpoint(durable_reference)?;
        Ok(())
    }

    fn retire_dead_organisms(&mut self) -> Result<(), GameAppShellError> {
        let mut dead_ids = self
            .world
            .organism_registry()
            .iter()
            .filter(|record| !record.lifecycle().is_alive())
            .map(|record| record.organism_id())
            .collect::<Vec<_>>();
        dead_ids.sort_unstable_by_key(|organism_id| organism_id.raw());
        for organism_id in dead_ids {
            self.retire_organism(organism_id, "world-authoritative death")?;
        }
        Ok(())
    }

    /// Applies an explicit managed-habitat breeding receipt to the live
    /// world. The receipt is reauthorized before a candidate child world is
    /// built, then the existing reconciliation path owns archive-before-GPU
    /// admission for the inherited child record.
    pub fn apply_managed_breed_receipt(
        &mut self,
        receipt: HabitatBreedingReceipt,
        child_organism_id: OrganismId,
        conception_seed: u64,
    ) -> Result<(), GameAppShellError> {
        let invalid_receipt = |message: String| GameAppShellError::InvalidProductionFrontend {
            message,
        };
        let expected = self
            .world
            .habitat_authority()
            .authorize_breeding(HabitatBreedingRequest {
                habitat_id: receipt.habitat_id,
                first_parent: receipt.first_parent,
                second_parent: receipt.second_parent,
                kind: receipt.kind,
                actor: receipt.actor,
                tick: receipt.tick,
            })
            .map_err(|error| {
                invalid_receipt(format!(
                    "managed breeding receipt rejected by the live habitat authority: {error}"
                ))
            })?;
        if receipt != expected
            || receipt.mode != HabitatMode::Managed
            || receipt.kind != HabitatBreedingKind::Explicit
            || receipt.tick != self.world.tick()
            || receipt.cognition_policy != alife_core::PolicyBackend::NeuralClosedLoopGpu
        {
            return Err(invalid_receipt(
                "managed breeding receipt is stale or does not match the live authority"
                    .to_string(),
            ));
        }
        if self.lineage_library.is_none() || self.lineage_run_id.is_none() {
            return Err(invalid_receipt(
                "managed breeding requires an attached lineage archive".to_string(),
            ));
        }

        child_organism_id.validate()?;
        let child_raw = child_organism_id.raw();
        if self
            .world
            .organism_registry()
            .get(child_organism_id)
            .is_some()
            || self
                .world
                .organism_entity_ids()
                .into_iter()
                .any(|(organism_id, _)| organism_id == child_organism_id)
            || self.handles.contains_key(&child_raw)
            || self.residents.contains_key(&child_raw)
            || self.archive_birth_manifests.contains_key(&child_raw)
        {
            return Err(invalid_receipt(format!(
                "managed breeding child organism {child_raw} is already present"
            )));
        }

        let current_tick = self.world.tick();
        let first_record = self
            .world
            .organism_registry()
            .get(receipt.first_parent)
            .cloned()
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let second_record = self
            .world
            .organism_registry()
            .get(receipt.second_parent)
            .cloned()
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let first_admission = first_record.authoritative_admission_at(current_tick)?;
        let second_admission = second_record.authoritative_admission_at(current_tick)?;
        for (organism_id, admission) in [
            (receipt.first_parent, &first_admission),
            (receipt.second_parent, &second_admission),
        ] {
            let resident = self
                .residents
                .get(&organism_id.raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let handle = self
                .handles
                .get(&organism_id.raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            if resident.genome != admission.phenotype.brain_genome
                || handle.organism_id() != organism_id
                || handle.phenotype_hash() != resident.phenotype.phenotype_hash()
            {
                return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
            }
        }

        let first_object = self
            .world
            .entity(first_admission.world_entity_id)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let second_object = self
            .world
            .entity(second_admission.world_entity_id)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        if first_object.kind != WorldObjectKind::Agent
            || first_object.organism_id != Some(receipt.first_parent)
            || second_object.kind != WorldObjectKind::Agent
            || second_object.organism_id != Some(receipt.second_parent)
        {
            return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
        }

        let child_genome = alife_core::CreatureGenome::reproduce(
            first_record.genome(),
            second_record.genome(),
            conception_seed,
        )?;
        let child_phenotype = child_genome.express()?;
        let child_position = Vec3f::new(
            (first_object.position.x + second_object.position.x) * 0.5,
            (first_object.position.y + second_object.position.y) * 0.5,
            (first_object.position.z + second_object.position.z) * 0.5,
        );
        child_position.validate()?;
        let child_affinity =
            ((first_object.social_affinity + second_object.social_affinity) * 0.5).clamp(-1.0, 1.0);

        let mut candidate_world = self.world.clone();
        let child_entity_id = candidate_world.spawn_social_agent(
            &format!("organism-{child_raw}"),
            child_organism_id,
            child_position,
            child_affinity,
        )?;
        let child_record = WorldOrganismRecord::newborn(
            child_organism_id,
            child_entity_id,
            child_genome,
            child_phenotype,
            current_tick,
        )
        .map_err(|error| {
            invalid_receipt(format!("managed breeding child record rejected: {error}"))
        })?;
        candidate_world.register_organism_record(child_record)?;
        let mut authority = candidate_world.habitat_authority().clone();
        authority
            .register_creature(child_organism_id, receipt.habitat_id, current_tick)
            .map_err(|error| {
                invalid_receipt(format!(
                    "managed breeding child habitat membership rejected: {error}"
                ))
            })?;
        candidate_world
            .replace_habitat_authority(authority)
            .map_err(|error| {
                invalid_receipt(format!(
                    "managed breeding child habitat authority rejected: {error}"
                ))
            })?;
        candidate_world.validate_organism_bindings()?;
        validate_candidate_newborn(&candidate_world, child_organism_id)?;
        Self::compile_birth(
            &candidate_world,
            self.brain_class,
            self.sensor_profile,
            child_organism_id,
        )?;

        self.world = candidate_world;
        self.reconcile_population()?;
        Ok(())
    }

    pub fn reconcile_population(&mut self) -> Result<(), GameAppShellError> {
        self.retire_dead_organisms()?;
        let live_ids = self
            .world
            .organism_entity_ids()
            .into_iter()
            .filter(|(organism_id, _)| {
                self.world
                    .organism_registry()
                    .get(*organism_id)
                    .is_none_or(|record| record.lifecycle().is_alive())
            })
            .map(|(organism_id, _)| organism_id.raw())
            .collect::<BTreeSet<_>>();

        let retired = self
            .handles
            .keys()
            .copied()
            .filter(|raw| !live_ids.contains(raw))
            .collect::<Vec<_>>();
        for raw in retired {
            if self.lineage_library.is_some()
                && !self.archive_retirement_receipts.contains_key(&raw)
            {
                return Err(GameAppShellError::InvalidProductionFrontend {
                    message: format!(
                        "organism {raw} must retire through the archive transaction before despawn"
                    ),
                });
            }
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
            let (phenotype, resident) = Self::compile_birth(
                &self.world,
                self.brain_class,
                self.sensor_profile,
                organism_id,
            )?;
            let birth_manifest_digest =
                self.archive_birth_before_gpu_insert(organism_id, &resident)?;
            let mut candidate_world = self.world.clone();
            if let Some(digest) = birth_manifest_digest {
                candidate_world.link_birth_manifest(organism_id, digest)?;
            }
            validate_candidate_newborn(&candidate_world, organism_id)?;
            let memory = Self::new_memory_sidecar(organism_id, self.sensor_profile)?;
            let topology = TopologySidecar::new_profiled(
                organism_id,
                SensorProfileIdentity {
                    profile_id: self.sensor_profile.into(),
                    profile_schema_version: 1,
                    sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
                },
                TopologicalMapConfig::default(),
            )?;
            let handle = self.backend.insert_brain(organism_id, phenotype)?;
            if handle.organism_id() != organism_id
                || handle.phenotype_hash() != resident.phenotype.phenotype_hash()
            {
                self.backend.remove_brain(handle)?;
                return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
            }
            self.world = candidate_world;
            if let Some(digest) = birth_manifest_digest {
                self.archive_birth_manifests.insert(raw, digest);
            }
            self.handles.insert(raw, handle);
            self.residents.insert(raw, resident);
            self.memories.insert(raw, memory);
            self.topologies.insert(raw, topology);
        }
        Ok(())
    }

    /// Attaches a lineage library to a restored runtime and backfills genetic
    /// manifests for residents whose original birth predates archive support.
    pub fn attach_lineage_archive(
        &mut self,
        config: LineageLibraryConfig,
        learned_capture_policy: ArchiveLearnedCapturePolicy,
    ) -> Result<(), GameAppShellError> {
        attach_lineage_archive_with_owned_authorities(
            self.checkpoint_durability.as_ref(),
            self.sensor_profile,
            self.world.tick(),
            &self.residents,
            &mut self.lineage_library,
            &mut self.lineage_run_id,
            &mut self.archive_learned_capture_policy,
            &mut self.archive_birth_manifests,
            config,
            learned_capture_policy,
        )
    }

    /// Performs the canonical death transaction. The immutable life manifest
    /// and optional learned checkpoint are committed before GPU retirement,
    /// and the world entity is despawned only after the receipt exists.
    pub fn retire_organism(
        &mut self,
        organism_id: OrganismId,
        death_reason: &str,
    ) -> Result<ArchiveRetirementReceipt, GameAppShellError> {
        organism_id.validate()?;
        if death_reason.trim().is_empty() || death_reason.chars().count() > 160 {
            return Err(ScaffoldContractError::InvalidId.into());
        }
        let raw = organism_id.raw();
        let existing_receipt = self.archive_retirement_receipts.get(&raw).cloned();
        let Some(record) = self.world.organism_registry().get(organism_id).cloned() else {
            return existing_receipt.ok_or_else(|| {
                GameAppShellError::Core(ScaffoldContractError::BrainOwnershipMismatch)
            });
        };
        let death_tick = record
            .lifecycle()
            .death_tick()
            .ok_or(ScaffoldContractError::InvalidId)?;
        let world_entity_id = record.world_entity_id();
        let object = self
            .world
            .entity(world_entity_id)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        if object.id != world_entity_id
            || object.kind != WorldObjectKind::Agent
            || object.organism_id != Some(organism_id)
        {
            return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
        }
        let handle = self.handles.get(&raw).copied();
        if existing_receipt.is_none() && handle.is_none() {
            return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
        }

        let receipt = if let Some(receipt) = existing_receipt {
            receipt.validate_contract()?;
            receipt
        } else {
            let birth_manifest_digest = *self.archive_birth_manifests.get(&raw).ok_or_else(|| {
                GameAppShellError::InvalidProductionFrontend {
                    message: format!("organism {raw} has no committed genetic archive"),
                }
            })?;
            let archive_root = self
                .lineage_library
                .as_ref()
                .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
                    message: "lineage archive is not attached".to_string(),
                })?
                .root()
                .to_path_buf();
            let checkpoint_retention = self.archive_learned_capture_policy.retention();
            let checkpoint_bytes = checkpoint_retention
                .map(|_| {
                    let checkpoint_store = GpuCheckpointAssetStore::new(archive_root)?;
                    let checkpoint = self.checkpoint_brain(organism_id, &checkpoint_store)?;
                    Ok::<_, GameAppShellError>(serde_json::to_vec(&serde_json::json!({
                        "save_state": checkpoint.save_state,
                        "manifest_entries": checkpoint.manifest_entries,
                        "checkpoint_digest": checkpoint.checkpoint_digest,
                    }))?)
                })
                .transpose()?;
            let final_experience_sequence = self
                .sealed_patches
                .iter()
                .rev()
                .find(|patch| patch.header().organism_id == organism_id)
                .map(|patch| patch.header().sequence_id);
            let resident = self
                .residents
                .get(&raw)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let mut final_statistics = resident.life_statistics.clone();
            final_statistics.finalize(death_tick, death_reason)?;
            let statistics = serde_json::to_vec(&final_statistics)?;
            let receipt = self
                .lineage_library
                .as_mut()
                .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
                    message: "lineage archive is not attached".to_string(),
                })?
                .archive_life(LifeArchiveInput {
                    birth_manifest_digest,
                    death_tick,
                    final_experience_sequence,
                    statistics_bytes: &statistics,
                    learned_checkpoint_bytes: checkpoint_bytes.as_deref(),
                    checkpoint_retention: checkpoint_retention
                        .unwrap_or(ArchiveCheckpointRetention::TemporaryPeak),
                })?;
            receipt.validate_contract()?;
            self.archive_retirement_receipts
                .insert(raw, receipt.clone());
            receipt
        };

        self.world
            .link_life_manifest(organism_id, receipt.committed_manifest_digest)?;
        #[cfg(test)]
        if self.forced_retirement_post_receipt_failure {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "test-forced post-receipt retirement failure".to_string(),
            });
        }

        if let Some(handle) = handle {
            #[cfg(test)]
            {
                self.retirement_backend_removal_count =
                    self.retirement_backend_removal_count.saturating_add(1);
            }
            self.backend.remove_brain(handle)?;
            self.handles.remove(&raw);
        }
        self.residents.remove(&raw);
        self.memories.remove(&raw);
        self.topologies.remove(&raw);
        self.retained_learning.remove(&raw);
        let (final_record, _) = self.world.retire_dead_organism(organism_id)?;
        self.presentation_retirements
            .insert(final_record.world_entity_id().raw());
        Ok(receipt)
    }

    pub fn archive_birth_manifest(&self, organism_id: OrganismId) -> Option<Blake3Digest> {
        self.archive_birth_manifests
            .get(&organism_id.raw())
            .copied()
    }

    pub fn archive_retirement_receipt(
        &self,
        organism_id: OrganismId,
    ) -> Option<&ArchiveRetirementReceipt> {
        self.archive_retirement_receipts.get(&organism_id.raw())
    }

    pub fn take_presentation_retirements(&mut self) -> Vec<WorldEntityId> {
        std::mem::take(&mut self.presentation_retirements)
            .into_iter()
            .map(WorldEntityId)
            .collect()
    }

    pub fn lineage_archive_manifest_count(&self) -> Result<Option<u64>, GameAppShellError> {
        self.lineage_library
            .as_ref()
            .map(LineageLibrary::manifest_count)
            .transpose()
            .map_err(Into::into)
    }

    pub fn passive_life_statistics(
        &self,
        organism_id: OrganismId,
    ) -> Option<&PassiveLifeStatistics> {
        self.residents
            .get(&organism_id.raw())
            .map(|resident| &resident.life_statistics)
    }

    pub fn observe_passive_life_event(
        &mut self,
        organism_id: OrganismId,
        event: PassiveLifeEvent,
    ) -> Result<(), GameAppShellError> {
        self.residents
            .get_mut(&organism_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
            .life_statistics
            .observe(event)?;
        Ok(())
    }

    fn archive_birth_before_gpu_insert(
        &mut self,
        organism_id: OrganismId,
        resident: &ResidentCognition,
    ) -> Result<Option<Blake3Digest>, GameAppShellError> {
        if self.lineage_library.is_none() {
            return Ok(None);
        }
        if let Some(existing_digest) = self.archive_birth_manifests.get(&organism_id.raw()) {
            return Ok(Some(*existing_digest));
        }
        let source_run_id = self.lineage_run_id.as_deref().ok_or_else(|| {
            GameAppShellError::InvalidProductionFrontend {
                message: "lineage archive source run id is missing".to_string(),
            }
        })?;
        let source_run_id = source_run_id.to_string();
        let digest = archive_birth_into_library(
            self.lineage_library
                .as_mut()
                .expect("lineage library presence checked above"),
            &source_run_id,
            organism_id,
            self.world.tick(),
            self.sensor_profile,
            resident,
        )?;
        Ok(Some(digest))
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
        let max_records_after = u32::try_from(memory.bank().capacity())
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
        let prepared = memory.prepare_compaction(cycle_id, max_records_after, 1)?;
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

    fn curated_first_tick_residency_gate(
        &self,
    ) -> Result<Option<&GpuCuratedResidencyReceipt>, GameAppShellError> {
        let reject = || GameAppShellError::InvalidProductionFrontend {
            message: "curated first tick residency receipt is absent or mismatched".to_string(),
        };
        let Some(plan) = self.retained_curated_founder_gpu_residency_plan.as_ref() else {
            return if self.curated_first_tick_pending
                || self
                    .retained_curated_founder_gpu_residency_receipt
                    .is_some()
            {
                Err(reject())
            } else {
                Ok(None)
            };
        };
        if plan.state != CuratedFounderGpuResidencyState::Committed {
            return Err(reject());
        }
        if !self.curated_first_tick_pending {
            return Ok(None);
        }
        if curated_founder_gpu_residency_plan_fingerprint(plan) != plan.fingerprint
            || plan.world_tick > self.world.tick()
        {
            return Err(reject());
        }
        let receipt = self
            .retained_curated_founder_gpu_residency_receipt
            .as_ref()
            .ok_or_else(|| reject())?;
        if !receipt.submission_completed
            || receipt.generation_fingerprint != plan.fingerprint
            || receipt.backend_hardware_generation != self.backend.hardware_receipt().generation
            || receipt.ordered_residents.len() != plan.entries.len()
            || self.handles.len() != receipt.ordered_residents.len()
            || self.residents.len() != receipt.ordered_residents.len()
            || self.memories.len() != receipt.ordered_residents.len()
            || self.topologies.len() != receipt.ordered_residents.len()
            || receipt.ordered_residents.is_empty()
        {
            return Err(reject());
        }

        let world_bindings = self.world.organism_entity_ids();
        for (entry, resident) in plan.entries.iter().zip(&receipt.ordered_residents) {
            let world_entity_id = WorldEntityId(resident.opaque_target_identity.raw());
            if world_entity_id.validate().is_err()
                || resident.organism_id != entry.organism_id
                || resident.opaque_target_identity.raw() != entry.world_entity_id.raw()
                || resident.exact_phenotype_hash != entry.projection.receipt().phenotype_hash()
                || resident.exact_foundation_hash != entry.projection.foundation_asset_digest()
                || !world_bindings.iter().any(|(organism_id, bound_entity_id)| {
                    *organism_id == resident.organism_id && *bound_entity_id == world_entity_id
                })
            {
                return Err(reject());
            }
            let Some(handle) = self.handles.get(&resident.organism_id.raw()) else {
                return Err(reject());
            };
            if *handle != resident.handle
                || handle.organism_id() != resident.organism_id
                || handle.phenotype_hash() != resident.exact_phenotype_hash
            {
                return Err(reject());
            }
        }
        if self.curated_first_tick_pending {
            Ok(Some(receipt))
        } else {
            Ok(None)
        }
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
        self.backend.ensure_neural_actions_available()?;
        let result = tick_with_sleep_progress_inner(self, |runtime| {
            runtime.tick_with_sleep_progress_staged(&mut progress)
        });
        if let Err(error) = &result {
            let contract_error = match error {
                GameAppShellError::Core(error)
                | GameAppShellError::GpuRuntime(alife_runtime::GpuRuntimeError::Core(error)) => {
                    Some(error)
                }
                _ => None,
            };
            if let Some(error) = contract_error {
                self.backend.record_contract_failure(error);
            }
        }
        result
    }

    fn tick_with_sleep_progress_staged<F>(
        &mut self,
        progress: &mut F,
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
        let curated_first_tick_resident = match self.curated_first_tick_residency_gate() {
            Ok(receipt) => receipt.and_then(|receipt| receipt.ordered_residents.first().cloned()),
            Err(error) => {
                self.backend
                    .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                return Err(error);
            }
        };
        let curated_first_tick = curated_first_tick_resident.is_some();
        self.retire_dead_organisms()?;
        self.reconcile_population()?;
        self.last_sealed_patches.clear();
        self.last_learning_receipts.clear();
        self.last_activity_work_receipts.clear();
        self.last_cognitive_work_receipts.clear();
        self.last_memory_recall_receipts.clear();
        self.last_memory_update_receipts.clear();
        self.last_cognitive_context_digests.clear();
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
        let homeostatic_parameters = self.homeostatic_parameters;
        let mut batch = Vec::with_capacity(self.handles.len());
        let mut summaries_by_organism = BTreeMap::new();
        let mut persist_sleep_boundary = false;
        let mut completed_promotions = Vec::new();
        let scheduled_handles = if let Some(first) = curated_first_tick_resident {
            vec![(
                first.organism_id.raw(),
                first.handle,
                WorldEntityId(first.opaque_target_identity.raw()),
            )]
        } else {
            self.handles
                .iter()
                .map(|(&raw, &handle)| {
                    let organism_id = OrganismId(raw);
                    let world_entity_id = self
                        .world
                        .organism_entity_ids()
                        .into_iter()
                        .find_map(|(bound_organism_id, world_entity_id)| {
                            (bound_organism_id == organism_id).then_some(world_entity_id)
                        })
                        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                    Ok::<_, ScaffoldContractError>((raw, handle, world_entity_id))
                })
                .collect::<Result<Vec<_>, _>>()?
        };
        let perception_index = self.world.build_perception_batch_index()?;
        for (raw, handle, world_entity_id) in scheduled_handles {
            let retained_learning_pending =
                self.retry_retained_learning(OrganismId(raw), tick_before)?;
            let record = self
                .world
                .organism_registry()
                .get(OrganismId(raw))
                .cloned()
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let resident = self
                .residents
                .get_mut(&raw)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            synchronize_resident_from_record(resident, &record, tick_before)?;
            let sleep_before = resident.sleep_scheduler.state();
            let phase_before = sleep_before.phase;
            // Fixed continuous-wake lab protocols suppress sleep phases but
            // keep the production work-cost ledger. Applying the existing
            // sleep-rate recovery prevents ecology energy exhaustion from
            // truncating their bounded neural measurement windows.
            let recover_brain_atp = phase_before != SleepPhase::Awake || !self.schedule_sleep;
            self.backend.charge_world_brain_atp_tick(
                handle,
                tick_before.raw(),
                recover_brain_atp,
            )?;
            let sleep_event = if self.schedule_sleep {
                let mut routed_driver = RoutedGpuSleepDriver {
                    backend: &mut self.backend,
                    handle,
                    progress,
                };
                resident.sleep_scheduler.scheduled_tick(
                    OrganismId(raw),
                    &resident.homeostasis,
                    homeostatic_parameters,
                    tick_before,
                    &mut routed_driver,
                )?
            } else {
                if phase_before != SleepPhase::Awake {
                    return Err(ScaffoldContractError::MissingPhaseData.into());
                }
                GpuSleepScheduleEvent {
                    tick: tick_before,
                    phase: SleepPhase::Awake,
                    cycle_id: sleep_before.last_consolidated_cycle_id,
                    transition: None,
                    consolidation_kind_raw: sleep_before.consolidation.kind_raw(),
                    selected_action: None,
                    motor_eligible: true,
                    sleep_work_units: 0,
                    phase_receipt: SleepPhaseReceipt {
                        phase: SleepPhase::Awake,
                        cycle_id: sleep_before.last_consolidated_cycle_id,
                        tick: tick_before,
                        due_work: SleepWorkDue::empty(),
                        work_units: 0,
                        cumulative_work_units: 0,
                        sealed: false,
                    },
                }
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
                summaries_by_organism.insert(
                    raw,
                    if retained_learning_pending && sleep_event.phase == SleepPhase::Awake {
                        Self::retained_learning_summary(
                            OrganismId(raw),
                            tick_before,
                            tick_after,
                            self.sealed_patch_count,
                        )
                    } else {
                        Self::sleeping_tick_summary(
                            OrganismId(raw),
                            tick_before,
                            tick_after,
                            self.sealed_patch_count,
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
                let draft = self.world.perception_frame_draft_indexed(
                    OrganismId(raw),
                    tick_before,
                    self.sensor_profile,
                    resident.homeostasis,
                    &perception_index,
                )?;
                let memory = self
                    .memories
                    .get(&raw)
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                let topology = self
                    .topologies
                    .get(&raw)
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                let sequence_id = ExperienceSequenceId(resident.next_sequence);
                sequence_id.validate()?;
                let prepared_recall = memory.recall_frame(&draft)?;
                let baseline_context = cognitive_context_for_recall(
                    OrganismId(raw),
                    sequence_id,
                    &prepared_recall,
                    topology,
                )?;
                let baseline_prepared = prepared_recall
                    .clone()
                    .with_cognitive_context(baseline_context.clone())?;
                let (baseline_frame, baseline_recall) = baseline_prepared.finalize(draft.clone())?;
                baseline_recall.validate_for_frame(&baseline_frame)?;
                let memory_evidence = finalized_memory_attention_evidence(&baseline_recall)?;
                let mut peripheral_summaries =
                    grounded_peripheral_summaries(draft.grounded_object_slots())?;
                let body_need = resident
                    .homeostasis
                    .drives
                    .to_array()
                    .iter()
                    .copied()
                    .fold(0.0, f32::max);
                apply_predecision_attention_evidence(
                    &mut peripheral_summaries,
                    body_need,
                    &memory_evidence,
                    &baseline_context,
                )?;
                let attention = select_focal_targets(
                    OrganismId(raw),
                    sequence_id,
                    tick_before,
                    &peripheral_summaries,
                    resident.attention_hysteresis,
                    AttentionSelectionPolicy::default(),
                )?;
                resident.attention_hysteresis = attention.hysteresis;
                let routed_draft = route_focal_candidates(draft, &attention)?;
                let routed_recall = memory.recall_frame(&routed_draft)?;
                let cognitive_context = cognitive_context_for_recall(
                    OrganismId(raw),
                    sequence_id,
                    &routed_recall,
                    topology,
                )?;
                let cognitive_context =
                    cognitive_context_with_attention(cognitive_context, attention)?;
                let prepared_recall = routed_recall.with_cognitive_context(cognitive_context)?;
                let (frame, memory_recall) = prepared_recall.finalize(routed_draft)?;
                memory_recall.validate_for_frame(&frame)?;
                let memory_upload =
                    self.backend
                        .prepare_memory_context_upload(handle, &frame, &memory_recall)?;
                Ok(PreparedGpuBrainFrame {
                    handle,
                    world_entity_id,
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
                    summaries_by_organism.insert(
                        raw,
                        Self::preparation_failure_summary(
                            OrganismId(raw),
                            tick_before,
                            tick_after,
                            self.sealed_patch_count,
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
        let expected_summary_count = if curated_first_tick {
            1
        } else {
            self.handles.len()
        };
        if summaries_by_organism.len() != expected_summary_count {
            return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
        }
        #[cfg(test)]
        if std::mem::take(&mut self.forced_late_advance_failure) {
            return Err(ScaffoldContractError::NonMonotonicTick.into());
        }
        advance_and_synchronize_authority(
            &mut self.world,
            &mut self.residents,
            tick_after,
        )?;
        self.observe_passive_tick(tick_before, tick_after)?;
        self.reconcile_population()?;
        if persist_sleep_boundary {
            self.persist_sleep_checkpoint_boundary()?;
        }
        Ok(summaries_by_organism.into_values().collect())
    }

    /// Shared neural-session authority used by gameplay and laboratory hosts.
    pub const fn session_authority(&self) -> &GpuSessionAuthority {
        self.backend.authority()
    }

    pub fn sealed_patches(&self) -> &[ExperiencePatch] {
        &self.sealed_patches
    }

    pub(crate) const fn sealed_patch_count(&self) -> usize {
        self.sealed_patch_count
    }

    pub(crate) fn last_sealed_patches(&self) -> &[ExperiencePatch] {
        &self.last_sealed_patches
    }

    /// Switches an explicit no-sleep benchmark fixture from its populated
    /// stimulus phase to an isolated phase. The world still owns the resulting
    /// unscored candidate set; this method never supplies a score or action.
    pub(crate) fn enter_isolated_benchmark_phase(&mut self) -> Result<(), GameAppShellError> {
        if self.schedule_sleep {
            return Err(ScaffoldContractError::InvalidPerceptionFrame.into());
        }
        let mut agent_ordinal = 0_u32;
        let mut stimulus_ordinal = 0_u32;
        for object in self.world.object_snapshots() {
            let position = if object.organism_id.is_some() {
                let position = Vec3f::new(agent_ordinal as f32 * 1_024.0, 0.0, 0.0);
                agent_ordinal = agent_ordinal
                    .checked_add(1)
                    .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
                position
            } else {
                let position =
                    Vec3f::new(-1_000_000.0 - stimulus_ordinal as f32 * 1_024.0, 0.0, 0.0);
                stimulus_ordinal = stimulus_ordinal
                    .checked_add(1)
                    .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
                position
            };
            self.world.editor_move_object(object.id, position)?;
        }
        Ok(())
    }

    /// Resets the one populated benchmark observer after warmup and creates a
    /// fresh ordinary food object, so measured causal work cannot depend on a
    /// stimulus consumed during the unmeasured phase.
    pub(crate) fn prepare_measured_benchmark_phase(&mut self) -> Result<(), GameAppShellError> {
        if self.schedule_sleep {
            return Err(ScaffoldContractError::InvalidPerceptionFrame.into());
        }
        let observer_entity = self
            .world
            .organism_entity_ids()
            .into_iter()
            .find_map(|(organism_id, entity_id)| {
                (organism_id == OrganismId(1)).then_some(entity_id)
            })
            .ok_or(ScaffoldContractError::InvalidId)?;
        self.world
            .editor_move_object(observer_entity, Vec3f::ZERO)?;
        self.world.editor_spawn_object(WorldEditorSpawnSpec {
            label: "benchmark-measured-food".to_string(),
            kind: WorldObjectKind::Food,
            organism_id: None,
            position: Vec3f::new(1.5, 0.0, 0.0),
            nutrition: 0.2,
            hazard_pain: 0.0,
            radius: 0.2,
            token_id: None,
        })?;
        Ok(())
    }

    /// Engine-neutral world snapshot paired with an explicit GPU checkpoint.
    /// It contains no GPU handles or neural payloads.
    pub fn world_snapshot(&self) -> HeadlessWorld {
        self.world.clone()
    }

    /// Authorizes structured education against the live world's habitat authority.
    pub fn authorize_structured_education(
        &mut self,
        organism_id: OrganismId,
        habitat_id: HabitatId,
        actor: HabitatActor,
    ) -> Result<HabitatPermissionReceipt, HabitatAuthorityError> {
        let world = self.world_snapshot();
        let tick = world.tick();
        let mut authority = world.habitat_authority().clone();
        let receipt = authority.authorize_operation(HabitatOperationRequest {
            habitat_id,
            organism_id,
            operation: HabitatOperation::StructuredEducation,
            actor,
            tick,
        })?;
        self.replace_habitat_authority(authority)?;
        Ok(receipt)
    }

    pub(crate) fn replace_habitat_authority(
        &mut self,
        authority: alife_world::HabitatAuthority,
    ) -> Result<(), alife_world::HabitatAuthorityError> {
        self.world.replace_habitat_authority(authority)
    }

    pub fn emit_player_tokens(
        &mut self,
        addressee: Option<OrganismId>,
        source_position: Vec3f,
        tokens: Vec<alife_core::LanguageTokenId>,
    ) -> Result<alife_world::AudibleUtterance, GameAppShellError> {
        Ok(self
            .world
            .emit_player_tokens(addressee, source_position, tokens)?)
    }

    pub fn active_utterances(&self) -> Vec<alife_world::AudibleUtterance> {
        self.world.audible_utterances()
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

    /// Hardware-independent cognitive work sealed by the most recent world
    /// tick. A cost policy can consume these receipts without changing action
    /// legality or world candidate enumeration.
    pub fn last_cognitive_work_receipts(&self) -> &[CognitiveWorkReceipt] {
        &self.last_cognitive_work_receipts
    }

    pub fn cognitive_work_cost_policy(&self) -> CognitiveWorkCostPolicy {
        self.cognitive_work_cost_policy
    }

    pub fn set_cognitive_work_cost_policy(
        &mut self,
        policy: CognitiveWorkCostPolicy,
    ) -> Result<(), ScaffoldContractError> {
        policy.validate_contract()?;
        self.cognitive_work_cost_policy = policy;
        Ok(())
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

    /// v1.1 context digests whose corresponding sealed GPU outcome reached
    /// fast plasticity in the most recent world tick.
    pub fn last_cognitive_context_digests(&self) -> &[[u64; 4]] {
        &self.last_cognitive_context_digests
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
        self.backend.backend().hardware_receipt()
    }

    pub(crate) fn take_completed_neural_timing_sample(
        &mut self,
    ) -> Option<alife_gpu_backend::GpuNeuralTimingSample> {
        self.backend.take_completed_neural_timing_sample()
    }

    pub(crate) const fn admission_receipt(&self) -> &alife_gpu_backend::GpuAdmissionReceipt {
        self.backend.backend().admission_receipt()
    }

    pub(crate) fn runtime_profile_digest(&self) -> Result<[u64; 4], GameAppShellError> {
        Ok(self.backend.runtime_profile().canonical_digest()?)
    }

    pub(crate) const fn activity_policy_digest(&self) -> [u64; 4] {
        self.backend.backend().activity_policy().policy_digest
    }

    pub(crate) fn evidence_activity_snapshot(
        &self,
        organism_id: OrganismId,
    ) -> Result<alife_gpu_backend::GpuActivityRuntimeSnapshot, ScaffoldContractError> {
        let handle = self.evidence_handle(organism_id)?;
        self.backend.snapshot_activity_state(handle)
    }

    pub(crate) fn install_recorded_pressure_replay(
        &mut self,
        samples: Vec<alife_core::GpuPressureSample>,
    ) -> Result<(), ScaffoldContractError> {
        self.backend.install_recorded_pressure_replay(samples)
    }

    pub(crate) fn recorded_pressure_replay_remaining(&self) -> usize {
        self.backend.recorded_pressure_replay_remaining()
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
        telemetry.sealed_patches = self.sealed_patch_count;
        telemetry.learning_updates =
            u32::try_from(self.last_learning_receipts.len()).unwrap_or(u32::MAX);
        telemetry.last_learning_delta = self
            .last_learning_receipts
            .iter()
            .map(|receipt| receipt.max_abs_delta)
            .fold(0.0_f32, f32::max);
        telemetry.active_ticks = u32::try_from(self.sealed_patch_count).unwrap_or(u32::MAX);
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
        world: &HeadlessWorld,
        brain_class: BrainScaleTier,
        sensor_profile: SensorProfile,
        organism_id: OrganismId,
    ) -> Result<(alife_core::BrainPhenotype, ResidentCognition), GameAppShellError> {
        let record = world
            .organism_registry()
            .get(organism_id)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let authority = restore_resident_authority_from_record(
            record,
            organism_id,
            record.world_entity_id(),
            world.tick(),
            brain_class,
            sensor_profile,
            None,
        )?;
        let phenotype = authority.phenotype.clone();
        let resident = authority.into_fresh_resident()?;
        Ok((phenotype, resident))
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
            .checked_mul(GPU_CLOSED_LOOP_TICK_READBACK_BYTES)
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

    fn observe_passive_tick(
        &mut self,
        tick_before: Tick,
        tick_after: Tick,
    ) -> Result<(), ScaffoldContractError> {
        let mut movement_by_organism = BTreeMap::<u64, u32>::new();
        let (residents, retained, recent) = (
            &mut self.residents,
            &self.sealed_patches,
            &self.last_sealed_patches,
        );
        for patch in retained
            .iter()
            .rev()
            .take(residents.len())
            .chain(recent)
            .filter(|patch| {
                patch.header().world_tick == tick_before
                    && patch.outcome().outcome_tick == tick_after
            })
        {
            let raw = patch.header().organism_id.raw();
            let displacement = patch.outcome().physical.displacement;
            let distance = (displacement.x * displacement.x
                + displacement.y * displacement.y
                + displacement.z * displacement.z)
                .sqrt()
                .clamp(0.0, 1.0);
            movement_by_organism.insert(raw, unit_f32_to_q16(distance));
            residents
                .get_mut(&raw)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                .life_statistics
                .observe_sealed_patch(patch)?;
        }
        for (&raw, resident) in residents {
            let work = self.last_activity_work_receipts.iter().find(|receipt| {
                receipt.organism_id_raw == raw && receipt.tick == tick_before.raw()
            });
            let gpu_dispatched = work.is_some();
            let gpu_throttled = work.is_some_and(|receipt| {
                receipt.counters.microsteps < u32::from(resident.phenotype.microstep_count())
            });
            resident
                .life_statistics
                .observe(PassiveLifeEvent::SurvivalTick {
                    tick: tick_after,
                    regime: EnvironmentalRegime::Temperate,
                    energy_q16: unit_f32_to_q16(resident.homeostasis.drives.brain_atp),
                    movement_distance_q16: movement_by_organism.get(&raw).copied().unwrap_or(0),
                    gpu_dispatched,
                    gpu_throttled,
                })?;
        }
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
            world_entity_id,
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
        memory_recall
            .cognitive_context()
            .ok_or(ScaffoldContractError::MissingPhaseData)?;
        let cognitive_context_digest = memory_recall.cognitive_context_digest()?;
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
        let speech_prompted = frame
            .sensory()
            .language_context
            .heard_tokens
            .iter()
            .flatten()
            .any(|token| token.source_kind == UtteranceSourceKind::Player);
        let factorized_channels = resident
            .phenotype
            .candidate_decoder()
            .factorized_motor_channels(&resident.phenotype)?;
        let motor_bundle = factorized_motor_bundle_for_candidates(
            organism_id,
            sequence_id,
            frame.tick(),
            &frame,
            gpu_tick.factorized_motor_candidates,
            &factorized_channels,
            &command,
            gpu_tick.selection.candidate_index,
            gpu_tick.speech_payload.as_ref(),
            speech_prompted,
        )?;
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
            world_entity_id,
            pending_eligibility: gpu_tick.pending_eligibility,
            frame,
            memory_recall,
            work: gpu_tick.work,
            cognitive_context_digest,
            sequence_id,
            outcome_tick,
            pre_action,
            decision,
            motor_bundle,
            speech_payload: gpu_tick.speech_payload,
            speech_prompted,
        })
    }

    fn seal_prepared_selection(
        &mut self,
        prepared: PreparedLiveSelection,
    ) -> Result<SealedLiveSelection, GameAppShellError> {
        let PreparedLiveSelection {
            handle,
            world_entity_id,
            pending_eligibility,
            frame,
            memory_recall,
            work,
            cognitive_context_digest,
            sequence_id,
            outcome_tick,
            pre_action,
            decision,
            motor_bundle,
            speech_payload,
            speech_prompted,
        } = prepared;
        let organism_id = handle.organism_id();
        let cognitive_context = memory_recall
            .cognitive_context()
            .cloned()
            .ok_or(ScaffoldContractError::MissingPhaseData)?;
        let sealed = seal_prepared_selection_core(
            &mut self.world,
            &mut self.residents,
            self.sealed_patch_count,
            self.cognitive_work_cost_policy,
            self.schedule_sleep,
            PreparedSealInput {
                organism_id,
                world_entity_id,
                frame,
                memory: memory_recall.receipt().clone(),
                sequence_id,
                outcome_tick,
                cognitive_context,
                work,
                pre_action,
                decision,
                motor_bundle,
                speech_payload,
                speech_prompted,
            },
        )?;
        Ok(SealedLiveSelection {
            handle,
            pending_eligibility,
            cognitive_context_digest,
            summary: sealed.summary,
            patch: sealed.patch,
        })
    }

    fn commit_sealed_batch(
        &mut self,
        mut sealed: Vec<SealedLiveSelection>,
    ) -> Result<Vec<LiveBrainTickSummary>, GameAppShellError> {
        if sealed.is_empty() {
            return Ok(Vec::new());
        }
        let curated_first_tick_succeeded = self.curated_first_tick_pending
            && sealed.len() == 1
            && sealed[0].patch.outcome().success;
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

        let (memory_updates, topology_updates) = if self.observe_sidecars {
            (
                self.observe_sealed_memory(&sealed),
                self.observe_sealed_topology(&sealed),
            )
        } else {
            (vec![false; sealed.len()], vec![false; sealed.len()])
        };

        let first_patch_count = self.sealed_patch_count;
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
            self.last_cognitive_context_digests.extend(
                sealed
                    .iter()
                    .map(|selection| selection.cognitive_context_digest),
            );
            self.last_learning_receipts.extend(learning);
        }
        self.last_cognitive_work_receipts.extend(
            sealed
                .iter()
                .filter_map(|selection| selection.patch.cognitive_work().copied()),
        );
        let committed_patches = sealed
            .into_iter()
            .map(|selection| selection.patch)
            .collect::<Vec<_>>();
        self.sealed_patch_count = self
            .sealed_patch_count
            .checked_add(committed_patches.len())
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        if self.retain_sealed_patch_history {
            self.sealed_patches.extend(committed_patches);
        } else {
            self.last_sealed_patches = committed_patches;
        }
        if curated_first_tick_succeeded {
            self.curated_first_tick_pending = false;
        }
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
    pub fn force_device_lost_after_next_submit_for_test(&mut self) {
        self.backend.force_device_lost_after_next_submit_for_test();
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

    #[cfg(test)]
    fn force_late_advance_failure_for_test(&mut self) {
        self.forced_late_advance_failure = true;
    }
}

fn validate_candidate_newborn(
    world: &HeadlessWorld,
    organism_id: OrganismId,
) -> Result<WorldEntityId, GameAppShellError> {
    let record = world
        .organism_registry()
        .get(organism_id)
        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
    if record.organism_id() != organism_id {
        return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
    }
    let world_entity_id = record.world_entity_id();
    let object = world
        .entity(world_entity_id)
        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
    if object.id != world_entity_id
        || object.kind != WorldObjectKind::Agent
        || object.organism_id != Some(organism_id)
    {
        return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
    }
    Ok(world_entity_id)
}

fn checkpoint_creature_save_state(
    replacement: &PortableSaveFile,
    record: &WorldOrganismRecord,
    resident: &ResidentCognition,
    brain_class: BrainScaleTier,
) -> Result<CreatureSaveState, GameAppShellError> {
    if record.genome().foundation.brain_class_id != brain_class.default_class_id() {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: "live organism genome class differs from GPU runtime class".to_string(),
        });
    }
    let biochemistry = record.biochemistry();
    let appearance = match (
        record
            .genome()
            .parent_genome_ids
            .first()
            .and_then(|genome_id| {
                replacement
                    .creatures
                    .iter()
                    .find(|creature| creature.genome_id == *genome_id)
                    .map(|creature| creature.appearance.clone())
            }),
        record
            .genome()
            .parent_genome_ids
            .get(1)
            .and_then(|genome_id| {
                replacement
                    .creatures
                    .iter()
                    .find(|creature| creature.genome_id == *genome_id)
                    .map(|creature| creature.appearance.clone())
            }),
    ) {
        (Some(parent_a), Some(parent_b)) => CreatureAppearanceGenome::offspring_from_parents(
            parent_a,
            parent_b,
            record.genome().conception_seed,
        ),
        _ => CreatureAppearanceGenome::default(),
    };
    Ok(CreatureSaveState {
        organism_id: record.organism_id(),
        genome_id: record.genome().id,
        brain_class,
        development_tick: biochemistry.development.last_update_tick,
        appearance,
        mind: CreatureMindSaveSummary {
            tick: biochemistry.tick,
            homeostasis: biochemistry.homeostasis,
            memory_record_count: 0,
            memory_source_ids: Vec::new(),
            concept_count: 0,
            edge_count: 0,
            simplex_count: 0,
            unresolved_gap_count: 0,
            sleep_state_label: gpu_sleep_state_label(resident.sleep_scheduler.state()),
            diagnostics: vec!["live canonical organism admitted".to_string()],
        },
        weights: WeightLayerSaveSummary {
            generated_weight_asset_id: None,
            genetic_fixed_digest: PortableAssetDigest::for_bytes(&serde_json::to_vec(
                record.genome(),
            )?)
            .0,
            genetic_layer_mutable: false,
            lifetime_consolidated_entries: 0,
            h_operational_entries: 0,
            h_shadow_entries: 0,
        },
        learning: LearningTraceSaveSummary {
            lifetime_learning_enabled: true,
            lamarckian_mode_enabled: false,
            last_consolidated_tick: None,
        },
        composite_genetics: None,
        lifetime_state_asset: None,
        gpu_brain: None,
    })
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

fn unit_f32_to_q16(value: f32) -> u32 {
    (value.clamp(0.0, 1.0) * 65_535.0).round() as u32
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

const N512_FOUNDATION_SEED: u64 = 0x4E35_3132_5F00_0001;

fn foundation_construction_development(
    genome: &BrainGenome,
    capacity: &BrainCapacityClass,
    development: &DevelopmentState,
) -> Result<DevelopmentState, ScaffoldContractError> {
    development.validate_contract()?;
    if development.genome_id != genome.id {
        return Err(ScaffoldContractError::PhenotypeCompile);
    }
    if capacity.id() != BrainCapacityClass::N2048_ID {
        return Ok(development.clone());
    }

    // The checked N2048 asset owns a full immutable coordinate ABI. World
    // development remains authoritative in ResidentCognition; the construction
    // input removes runtime chronology and dynamic gates that would reshape
    // that ABI.
    let mut construction = development.clone();
    construction.age_ticks = Tick::ZERO;
    construction.maturation = NormalizedScalar::new(1.0)?;
    construction.enabled_lobes.clear();
    construction.active_sensor_channels.clear();
    construction.active_motor_affordances.clear();
    construction.open_critical_periods.clear();
    construction.sleep_cycle_count = 0;
    construction.consolidation_cycle_count = 0;
    construction.last_sleep_tick = None;
    construction.validate_contract()?;
    Ok(construction)
}

fn compile_gpu_components_from_genome(
    genome: BrainGenome,
    development: DevelopmentState,
    sensor_profile: SensorProfile,
) -> Result<(alife_core::BrainPhenotype, PhenotypeCompilerInputs), ScaffoldContractError> {
    let capacity = BrainCapacityClass::production_for_id(genome.brain_class_id)?;
    let foundation = match capacity.id() {
        BrainCapacityClass::N2048_ID => FoundationWeightAsset::builtin_n2048_v1(sensor_profile)?,
        BrainCapacityClass::N512_ID => FoundationWeightAsset::builtin_nano512_v1(sensor_profile)?,
        _ => return Err(ScaffoldContractError::UnsupportedProductionBrainClass),
    };
    let construction_development =
        foundation_construction_development(&genome, &capacity, &development)?;
    let phenotype = PhenotypeCompiler::compile_from_foundation_asset(
        &genome,
        &capacity,
        &construction_development,
        sensor_profile,
        &foundation,
    )?;
    let compiler_inputs = PhenotypeCompilerInputs::try_new_with_foundation_abi(
        genome,
        &capacity,
        construction_development,
        sensor_profile,
        phenotype.foundation_abi().clone(),
    )?;
    let verified_phenotype = PhenotypeCompiler::compile_validated(&compiler_inputs, &capacity)?;
    if verified_phenotype != phenotype {
        return Err(ScaffoldContractError::PhenotypeCompile);
    }
    Ok((phenotype, compiler_inputs))
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
    if capacity.id() == BrainCapacityClass::N2048_ID {
        let birth_seed = deterministic_seed ^ organism_id.raw().rotate_left(17);
        let genome = BrainGenome::scaffold(birth_seed, capacity.id());
        let development = DevelopmentState::new(genome.id, tick, NormalizedScalar::new(0.35)?);
        let (phenotype, _) =
            compile_gpu_components_from_genome(genome.clone(), development.clone(), sensor_profile)?;
        return Ok((phenotype, genome, development));
    }

    if capacity.id() == BrainCapacityClass::N512_ID {
        let genome = BrainGenome::scaffold(N512_FOUNDATION_SEED, capacity.id());
        let development = DevelopmentState::new(genome.id, tick, NormalizedScalar::new(1.0)?);
        let (phenotype, _) =
            compile_gpu_components_from_genome(genome.clone(), development.clone(), sensor_profile)?;
        return Ok((phenotype, genome, development));
    }

    Err(ScaffoldContractError::UnsupportedProductionBrainClass)
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        fs,
        path::{Path, PathBuf},
    };

    use super::*;
    use crate::{
        curated_founder_materializer::materialize_curated_founder_bundle,
        plan_curated_founder_reset, CuratedFounderAgentInput, CuratedFounderResetRequest,
        CURATED_FOUNDER_RESET_POLICY,
    };
    use alife_archive::{LineageLibrary, LineageLibraryConfig};
    use alife_core::{
        ActionTarget, AttentionSelectionPolicy, BrainCapacityClass, CandidateActionFamily,
        Confidence, ExperienceSequenceId, FoundationGeneticIdentity, FoundationWeightAsset,
        GenomeId, HysteresisState, NormalizedScalar, OrganismId, OutcomeCreditPacket,
        PeripheralSummary, PreActionBrainEvidence, SalienceComponents, SensorProfile,
        StableFocusIdentity, Tick, TrackedObjectId, Vec3f, WorldEntityId,
    };
    use alife_runtime::GpuDurableSaveManifest;
    use alife_world::{
        persistence::{AssetManifest, PortableSaveFile, RuntimeConfig},
        HeadlessScenarioBuilder, HeadlessWorld, WorldOrganismRecord,
    };

    #[test]
    fn v11_attention_causally_changes_finalized_upload_and_holds_top_k_primary() {
        let organism_id = OrganismId(1);
        let seed = 77_111;
        let world = HeadlessScenarioBuilder::new(seed)
            .agent("agent", organism_id, Vec3f::ZERO)
            .food("food-a", Vec3f::new(1.0, 0.0, 0.0), 0.8)
            .food("food-b", Vec3f::new(-1.0, 0.0, 0.0), 0.8)
            .build()
            .unwrap();
        let mut runtime = GpuLiveBrainRuntime::new_profiled(
            GpuClosedLoopBackend::new_required(
                alife_gpu_backend::GpuRuntimeProfile::production_v1(),
            )
            .expect("required GPU"),
            world,
            seed,
            BrainScaleTier::Nano512,
            SensorProfile::GroundedObjectSlotsV1,
        )
        .unwrap();
        let handle = runtime.handle_for(organism_id).unwrap();
        let homeostasis = runtime.residents[&organism_id.raw()].homeostasis;
        let perception_index = runtime.world.build_perception_batch_index().unwrap();
        let draft = runtime
            .world
            .perception_frame_draft_indexed(
                organism_id,
                Tick::ZERO,
                SensorProfile::GroundedObjectSlotsV1,
                homeostasis,
                &perception_index,
            )
            .unwrap();
        let memory = runtime.memories[&organism_id.raw()].clone();
        let topology = runtime.topologies[&organism_id.raw()].clone();
        let sequence_id = ExperienceSequenceId(runtime.residents[&organism_id.raw()].next_sequence);
        let prepared_recall = memory.recall_frame(&draft).unwrap();
        let baseline_context = cognitive_context_for_recall(
            organism_id,
            sequence_id,
            &prepared_recall,
            &topology,
        )
        .unwrap();
        let baseline_prepared = prepared_recall
            .clone()
            .with_cognitive_context(baseline_context.clone())
            .unwrap();
        let (baseline_frame, baseline_recall) = baseline_prepared.finalize(draft.clone()).unwrap();
        baseline_recall
            .validate_for_frame(&baseline_frame)
            .unwrap();
        let memory_evidence = finalized_memory_attention_evidence(&baseline_recall).unwrap();
        let body_need = homeostasis
            .drives
            .to_array()
            .iter()
            .copied()
            .fold(0.0, f32::max);

        let mut base_summaries = grounded_peripheral_summaries(draft.grounded_object_slots())
            .unwrap();
        assert!(base_summaries.len() >= 2);
        for summary in &mut base_summaries {
            summary.salience = SalienceComponents::default();
        }
        base_summaries[0].salience.peripheral_intensity = NormalizedScalar::new(0.2).unwrap();
        base_summaries[1].salience.peripheral_intensity = NormalizedScalar::new(0.1).unwrap();
        let first_identity = base_summaries[0].identity;
        let second_identity = base_summaries[1].identity;
        apply_predecision_attention_evidence(
            &mut base_summaries,
            body_need,
            &memory_evidence,
            &baseline_context,
        )
        .unwrap();
        let single_target_policy = AttentionSelectionPolicy {
            focal_capacity: 1,
            protected_minimum: 1,
            requested_focal_count: 1,
            ..AttentionSelectionPolicy::default()
        };
        let base_attention = select_focal_targets(
            organism_id,
            sequence_id,
            Tick::ZERO,
            &base_summaries,
            HysteresisState::default(),
            single_target_policy,
        )
        .unwrap();
        assert_eq!(base_attention.focal_targets, vec![first_identity]);

        let mut changed_summaries = base_summaries.clone();
        changed_summaries[0].salience.peripheral_intensity = NormalizedScalar::new(0.2).unwrap();
        changed_summaries[1].salience.peripheral_intensity = NormalizedScalar::new(1.0).unwrap();
        let changed_attention = select_focal_targets(
            organism_id,
            sequence_id,
            Tick::ZERO,
            &changed_summaries,
            HysteresisState::default(),
            single_target_policy,
        )
        .unwrap();
        assert_eq!(changed_attention.focal_targets, vec![second_identity]);

        let mut finalize_with_attention = |attention: AttentionFrame| {
            let routed_draft = route_focal_candidates(draft.clone(), &attention)?;
            let routed_recall = memory.recall_frame(&routed_draft)?;
            let context = cognitive_context_for_recall(
                organism_id,
                sequence_id,
                &routed_recall,
                &topology,
            )?;
            let context = cognitive_context_with_attention(context, attention)?;
            let prepared = routed_recall.with_cognitive_context(context)?;
            let (frame, memory_recall) = prepared.finalize(routed_draft)?;
            memory_recall.validate_for_frame(&frame)?;
            let upload = runtime
                .backend
                .prepare_memory_context_upload(handle, &frame, &memory_recall)?;
            Ok::<_, ScaffoldContractError>((frame, memory_recall, upload))
        };
        let (base_frame, base_recall, base_upload) =
            finalize_with_attention(base_attention.clone()).unwrap();
        let (changed_frame, changed_recall, changed_upload) =
            finalize_with_attention(changed_attention.clone()).unwrap();
        assert_eq!(
            base_recall
                .cognitive_context()
                .unwrap()
                .focal
                .identities,
            base_attention.focal_targets
        );
        assert_eq!(
            changed_recall
                .cognitive_context()
                .unwrap()
                .focal
                .identities,
            changed_attention.focal_targets
        );
        assert_ne!(
            base_recall.cognitive_context_digest().unwrap(),
            changed_recall.cognitive_context_digest().unwrap()
        );
        assert_ne!(base_frame.base_digest(), changed_frame.base_digest());
        assert_ne!(base_frame.frame_digest(), changed_frame.frame_digest());
        assert_eq!(base_upload.final_frame_digest, base_frame.frame_digest());
        assert_eq!(changed_upload.final_frame_digest, changed_frame.frame_digest());
        assert_ne!(base_upload.final_frame_digest, changed_upload.final_frame_digest);

        let summary = |id: u64, intensity: f32| PeripheralSummary {
            identity: StableFocusIdentity::TrackedObject(TrackedObjectId(id)),
            salience: SalienceComponents {
                peripheral_intensity: NormalizedScalar::new(intensity).unwrap(),
                ..SalienceComponents::default()
            },
            confidence: Confidence::new(1.0).unwrap(),
        };
        let top_k_policy = AttentionSelectionPolicy {
            focal_capacity: 2,
            protected_minimum: 1,
            requested_focal_count: 2,
            switch_cost: NormalizedScalar::new(0.05).unwrap(),
            ..AttentionSelectionPolicy::default()
        };
        let first = select_focal_targets(
            organism_id,
            sequence_id,
            Tick::ZERO,
            &[summary(101, 0.40), summary(102, 0.39), summary(103, 0.10)],
            HysteresisState::default(),
            top_k_policy,
        )
        .unwrap();
        let near_challenger = select_focal_targets(
            organism_id,
            sequence_id,
            Tick::ZERO,
            &[summary(101, 0.40), summary(102, 0.44), summary(103, 0.10)],
            first.hysteresis,
            top_k_policy,
        )
        .unwrap();
        assert_eq!(
            near_challenger.focal_targets,
            vec![
                StableFocusIdentity::TrackedObject(TrackedObjectId(101)),
                StableFocusIdentity::TrackedObject(TrackedObjectId(102)),
            ]
        );
        let far_challenger = select_focal_targets(
            organism_id,
            sequence_id,
            Tick::ZERO,
            &[summary(101, 0.40), summary(102, 0.80), summary(103, 0.10)],
            first.hysteresis,
            top_k_policy,
        )
        .unwrap();
        assert_eq!(
            far_challenger.focal_targets[0],
            StableFocusIdentity::TrackedObject(TrackedObjectId(102))
        );
    }

    #[test]
    fn v11_authoritative_admission_real_runtime() {
        let organism_id = OrganismId(1);
        let seed = 7_701;
        let world = HeadlessScenarioBuilder::new(seed)
            .agent("learner", organism_id, Vec3f::ZERO)
            .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.9)
            .hazard("hazard", Vec3f::new(-2.0, 0.0, 0.0), 0.7)
            .build()
            .unwrap();
        let mut runtime = GpuLiveBrainRuntime::new(
            GpuClosedLoopBackend::new_required(
                alife_gpu_backend::GpuRuntimeProfile::production_v1(),
            )
            .expect("required Vulkan adapter"),
            world,
            seed,
            BrainScaleTier::Nano512,
        )
        .unwrap();

        let birth_record = runtime
            .world
            .organism_registry()
            .get(organism_id)
            .unwrap()
            .clone();
        let birth_resident = runtime.residents.get(&organism_id.raw()).unwrap();
        assert_eq!(
            birth_resident.genome,
            birth_record.phenotype().brain_genome.clone()
        );
        assert_eq!(
            birth_resident.compiler_inputs.genome(),
            &birth_record.phenotype().brain_genome
        );
        assert_eq!(
            birth_resident.homeostasis,
            birth_record.biochemistry().homeostasis
        );
        assert_eq!(
            birth_resident.development,
            birth_record
                .phenotype()
                .development_state_at(birth_record.age_at(runtime.world.tick()).unwrap())
                .unwrap()
        );
        runtime.reconcile_population().unwrap();

        let hardware = runtime.hardware_receipt().clone();
        eprintln!(
            "v11 real runtime adapter={} backend_api={} backend_version={}",
            hardware.adapter_name, hardware.backend_api, hardware.backend_version
        );
        let summaries = runtime.tick().unwrap();
        assert_eq!(summaries.len(), 1);
        assert!(summaries[0].patch_sealed);
        assert_eq!(summaries[0].patch_success, Some(true));
        assert_eq!(summaries[0].learning_updates, 1);
        assert!(runtime.sealed_patches()[0].outcome().success);
        assert_eq!(runtime.last_learning_receipts().len(), 1);

        let asset_root = std::env::temp_dir().join(format!(
            "alife-v11-authoritative-admission-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&asset_root);
        fs::create_dir_all(&asset_root).unwrap();
        let store = GpuCheckpointAssetStore::new(asset_root.clone()).unwrap();
        let write = runtime.checkpoint_brain(organism_id, &store).unwrap();
        let mut manifest = AssetManifest::empty();
        merge_gpu_checkpoint_manifest_entries(&mut manifest, write.manifest_entries).unwrap();
        let world_at_checkpoint = runtime.world_snapshot();
        let mut restored = GpuLiveBrainRuntime::restore_with_checkpoints(
            GpuClosedLoopBackend::new_required(
                alife_gpu_backend::GpuRuntimeProfile::production_v1(),
            )
            .expect("required Vulkan adapter"),
            world_at_checkpoint,
            seed,
            BrainScaleTier::Nano512,
            &store,
            &manifest,
            std::slice::from_ref(&write.save_state),
        )
        .unwrap();
        let restored_record = restored
            .world
            .organism_registry()
            .get(organism_id)
            .unwrap();
        let restored_resident = restored.residents.get(&organism_id.raw()).unwrap();
        let source_phenotype = runtime
            .residents
            .get(&organism_id.raw())
            .unwrap()
            .phenotype
            .clone();
        assert_eq!(
            restored.handle_for(organism_id).unwrap().organism_id(),
            organism_id
        );
        assert_eq!(
            restored_resident.genome,
            restored_record.phenotype().brain_genome.clone()
        );
        assert_eq!(
            restored_resident.phenotype,
            source_phenotype
        );
        assert_eq!(
            restored_resident.homeostasis,
            restored_record.biochemistry().homeostasis
        );
        assert_eq!(
            restored_resident.development,
            restored_record
                .phenotype()
                .development_state_at(restored_record.age_at(restored.world.tick()).unwrap())
                .unwrap()
        );

        let source_record = restored_record.clone();
        let mut sleepy_biochemistry = source_record.biochemistry().clone();
        let mut drives = sleepy_biochemistry.homeostasis.drives;
        drives.fatigue = 0.99;
        let mut hormones = sleepy_biochemistry.homeostasis.hormones;
        hormones.sleep_pressure = 0.99;
        sleepy_biochemistry.homeostasis = HomeostaticSnapshot::new(
            sleepy_biochemistry.homeostasis.tick,
            drives,
            hormones,
        )
        .unwrap();
        let sleepy_record = WorldOrganismRecord::new(
            source_record.organism_id(),
            source_record.world_entity_id(),
            source_record.genome().clone(),
            source_record.phenotype().clone(),
            sleepy_biochemistry,
            source_record.birth_tick(),
        )
        .unwrap();
        restored
            .world_mut()
            .replace_organism_registry_exact([sleepy_record])
            .unwrap();
        let sleep_tick_before = restored.world.tick();
        let mut sleep_driver = NoProgressSleepDriver;
        let sleep_summaries = restored
            .tick_with_sleep_driver(&mut sleep_driver)
            .unwrap();
        assert_eq!(sleep_summaries.len(), 1);
        assert_eq!(sleep_summaries[0].status, BrainTickStatus::SafeIdle);
        assert_eq!(sleep_summaries[0].selected_action_id, None);
        assert!(!sleep_summaries[0].patch_sealed);
        assert_eq!(
            restored.world.tick(),
            Tick::new(sleep_tick_before.raw().saturating_add(1))
        );
        let refreshed_record = restored
            .world
            .organism_registry()
            .get(organism_id)
            .unwrap();
        let refreshed_resident = restored.residents.get(&organism_id.raw()).unwrap();
        assert_eq!(
            refreshed_record.biochemistry().tick,
            restored.world.tick()
        );
        assert_eq!(
            refreshed_resident.homeostasis,
            refreshed_record.biochemistry().homeostasis
        );
        assert_eq!(
            refreshed_resident.development,
            refreshed_record
                .phenotype()
                .development_state_at(
                    refreshed_record
                        .age_at(restored.world.tick())
                        .unwrap(),
                )
                .unwrap()
        );

        let before_tick = runtime.world.tick();
        let before_objects = runtime.world.object_snapshots();
        let before_records = runtime
            .world
            .organism_registry()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let before_resident = runtime
            .residents
            .get(&organism_id.raw())
            .unwrap()
            .clone();
        let sealed_before_failure = runtime.sealed_patches().len();
        runtime.force_late_advance_failure_for_test();
        let result = runtime.tick();
        assert!(matches!(
            result,
            Err(GameAppShellError::Core(ScaffoldContractError::NonMonotonicTick))
        ));
        assert_eq!(runtime.last_learning_receipts().len(), 1);
        assert_eq!(runtime.sealed_patches().len(), sealed_before_failure + 1);
        assert_eq!(runtime.world.tick(), before_tick);
        assert_eq!(runtime.world.object_snapshots(), before_objects);
        assert_eq!(
            runtime
                .world
                .organism_registry()
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
            before_records
        );
        let resident = runtime.residents.get(&organism_id.raw()).unwrap();
        assert_eq!(resident.genome, before_resident.genome);
        assert_eq!(resident.phenotype, before_resident.phenotype);
        assert_eq!(resident.homeostasis, before_resident.homeostasis);
        assert_eq!(resident.development, before_resident.development);
        assert_eq!(resident.next_sequence, before_resident.next_sequence);

        let projected = project_curated_founder_reset_runtime_error(
            CuratedFounderResetRuntimeError::GpuResidencyUnknown {
                evidence: Some(CuratedFounderResetRuntimeEvidence {
                    status: CuratedFounderPublicationStatus::Published,
                    save_state: CuratedFounderSaveState::Verified,
                    gpu_residency: CuratedFounderGpuResidencyState::Pending,
                    expected_save_digest: Some("old-save".to_string()),
                    actual_save_digest: Some("old-save".to_string()),
                    proposed_save_digest: "published-save".to_string(),
                    cause: None,
                    archive_count: 2,
                }),
                error: ScaffoldContractError::NeuralBackendUnavailable,
            },
        );

        assert!(matches!(
            projected,
            CuratedFounderResetDispatchResult::Unknown {
                proposed_save_digest,
                archive_count: 2,
                save_state: CuratedFounderSaveState::Verified,
                gpu_residency: CuratedFounderGpuResidencyState::Unknown,
                retryable: false,
                ..
            } if proposed_save_digest == "published-save"
        ));

        fs::remove_dir_all(asset_root).unwrap();
    }

    #[test]
    fn failed_staged_runtime_commit_leaves_live_state_unchanged() {
        let mut live = (7_u64, BTreeMap::from([(11_u64, "old")]));
        let before = live.clone();
        let result = commit_staged_runtime(
            &mut live,
            Err("checkpoint staging failed"),
            |live, candidate| *live = candidate,
        );

        assert_eq!(result, Err("checkpoint staging failed"));
        assert_eq!(live, before);
    }

    #[test]
    fn replacement_rejects_persisted_policy_seed_or_brain_class_mismatch() {
        for (policy, seed, brain_class) in [
            (
                alife_core::PolicyBackend::HeuristicBaseline,
                7_u64,
                BrainScaleTier::Nano512,
            ),
            (
                alife_core::PolicyBackend::NeuralClosedLoopGpu,
                8_u64,
                BrainScaleTier::Nano512,
            ),
            (
                alife_core::PolicyBackend::NeuralClosedLoopGpu,
                7_u64,
                BrainScaleTier::Small1024,
            ),
        ] {
            assert!(matches!(
                validate_replacement_policy(
                    policy,
                    seed,
                    brain_class,
                    7,
                    BrainScaleTier::Nano512,
                ),
                Err(GameAppShellError::InvalidGraphicalLaunch { .. })
            ));
        }
    }

    struct NoProgressSleepDriver;

    const CURATED_RUNTIME_WORLD_SEED: u64 = 0xA11F_E3E2_3C3A_0001;
    const CURATED_RUNTIME_WORLD_ENTITY_IDS: [u64; 3] = [1, 2, 3];
    const CURATED_RUNTIME_ORGANISM_IDS: [u64; 3] = [101, 202, 303];

    struct CuratedRuntimeAuthorityFixture {
        request: CuratedFounderResetRequest,
        source_save: PortableSaveFile,
        world: HeadlessWorld,
        durable_root: PathBuf,
        durable_save_path: PathBuf,
        asset_root: PathBuf,
        archive_root: PathBuf,
        durability: Option<GpuLiveCheckpointDurability>,
        lineage_library: Option<LineageLibrary>,
        archive_run_id: String,
    }

    impl Drop for CuratedRuntimeAuthorityFixture {
        fn drop(&mut self) {
            let _ = self.lineage_library.take();
            let _ = self.durability.take();
            let _ = fs::remove_dir_all(&self.durable_root);
            let _ = fs::remove_dir_all(&self.archive_root);
        }
    }

    fn curated_runtime_request() -> CuratedFounderResetRequest {
        let sensor_profile = SensorProfile::PrivilegedAffordanceV1;
        let foundation_asset = FoundationWeightAsset::builtin_nano512_v1(sensor_profile).unwrap();
        let foundation_manifest = foundation_asset.manifest();
        let foundation = FoundationGeneticIdentity::new(
            foundation_manifest.foundation_id().raw(),
            foundation_manifest.foundation_version().raw() as u16,
            foundation_manifest.compatibility_family_id().raw(),
            BrainCapacityClass::N512_ID,
        )
        .unwrap();
        let final_agents = (0..3)
            .rev()
            .map(|slot| CuratedFounderAgentInput {
                world_entity_id: WorldEntityId(CURATED_RUNTIME_WORLD_ENTITY_IDS[slot as usize]),
                organism_id: Some(OrganismId(CURATED_RUNTIME_ORGANISM_IDS[slot as usize])),
                final_population_slot: slot,
                legacy_genome_id: None,
            })
            .collect();
        CuratedFounderResetRequest {
            policy_label: Some(CURATED_FOUNDER_RESET_POLICY.to_string()),
            source_save_identity: "save-curated-runtime".to_string(),
            source_save_label: "runtime authority fixture".to_string(),
            source_save_seed: CURATED_RUNTIME_WORLD_SEED,
            world_seed: CURATED_RUNTIME_WORLD_SEED,
            restored_tick: Tick::ZERO,
            target_population: 3,
            sensor_profile,
            foundation,
            foundation_content_digest: foundation_asset.digest(),
            source_run_identity: "curated-runtime-source".to_string(),
            final_agents,
        }
    }

    fn archive_snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn visit(root: &Path, current: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in fs::read_dir(current).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    visit(root, &path, files);
                } else {
                    files.insert(
                        path.strip_prefix(root).unwrap().to_path_buf(),
                        fs::read(path).unwrap(),
                    );
                }
            }
        }
        let mut files = BTreeMap::new();
        visit(root, root, &mut files);
        files
    }

    fn registry_snapshot(world: &HeadlessWorld) -> Vec<Vec<u8>> {
        let mut records = world
            .organism_registry()
            .iter()
            .map(|record| serde_json::to_vec(record).unwrap())
            .collect::<Vec<_>>();
        records.sort();
        records
    }

    struct CuratedRuntimeAuthoritySnapshot {
        world_signature: alife_world::HeadlessWorldSignatureDigest,
        registry: Vec<Vec<u8>>,
        save: Vec<u8>,
        archive: BTreeMap<PathBuf, Vec<u8>>,
        archive_count: Option<u64>,
    }

    fn authority_snapshot(
        fixture: &CuratedRuntimeAuthorityFixture,
    ) -> CuratedRuntimeAuthoritySnapshot {
        let archive_count = fixture
            .lineage_library
            .as_ref()
            .map(|library| library.manifest_count().unwrap());
        CuratedRuntimeAuthoritySnapshot {
            world_signature: fixture.world.canonical_signature_digest().unwrap(),
            registry: registry_snapshot(&fixture.world),
            save: fs::read(&fixture.durable_save_path).unwrap(),
            archive: archive_snapshot(&fixture.archive_root),
            archive_count,
        }
    }

    fn assert_authority_unchanged(
        before: CuratedRuntimeAuthoritySnapshot,
        fixture: &CuratedRuntimeAuthorityFixture,
    ) {
        let after = authority_snapshot(fixture);
        assert_eq!(after.world_signature, before.world_signature);
        assert_eq!(after.registry, before.registry);
        assert_eq!(after.save, before.save);
        assert_eq!(after.archive, before.archive);
        if let (Some(before_count), Some(after_count)) = (before.archive_count, after.archive_count)
        {
            assert_eq!(after_count, before_count);
        }
    }

    fn curated_runtime_authority_fixture(label: &str) -> CuratedRuntimeAuthorityFixture {
        let request = curated_runtime_request();
        let source_world = HeadlessScenarioBuilder::new(CURATED_RUNTIME_WORLD_SEED)
            .agent(
                "runtime-founder-0",
                OrganismId(CURATED_RUNTIME_ORGANISM_IDS[0]),
                Vec3f::ZERO,
            )
            .agent(
                "runtime-founder-1",
                OrganismId(CURATED_RUNTIME_ORGANISM_IDS[1]),
                Vec3f::new(1.0, 0.0, 0.0),
            )
            .agent(
                "runtime-founder-2",
                OrganismId(CURATED_RUNTIME_ORGANISM_IDS[2]),
                Vec3f::new(2.0, 0.0, 0.0),
            )
            .build()
            .unwrap();
        let source_save = PortableSaveFile::from_headless_world(
            "save-curated-runtime",
            &source_world,
            RuntimeConfig::deterministic_default(
                CURATED_RUNTIME_WORLD_SEED,
                BrainScaleTier::Nano512,
            ),
            AssetManifest::empty(),
            Vec::new(),
        )
        .unwrap();
        let world = source_save.restore_headless_world().unwrap();
        let suffix = format!("{}-{label}", std::process::id());
        let archive_root =
            std::env::temp_dir().join(format!("alife-curated-runtime-archive-{suffix}"));
        let durable_root =
            std::env::temp_dir().join(format!("alife-curated-runtime-durable-{suffix}"));
        let _ = fs::remove_dir_all(&archive_root);
        let _ = fs::remove_dir_all(&durable_root);
        fs::create_dir_all(&durable_root).unwrap();
        let lineage_library =
            LineageLibrary::open(LineageLibraryConfig::profile_default(&archive_root)).unwrap();
        let asset_root = std::env::current_dir().unwrap();
        let durable_save_path = durable_root.join("live-save.json");
        GpuDurableSaveManifest::publish_snapshot(&durable_save_path, &asset_root, &source_save)
            .unwrap();
        let durable_manifest =
            GpuDurableSaveManifest::open(&durable_save_path, &asset_root).unwrap();
        let published = durable_manifest.load().unwrap();
        let store = GpuCheckpointAssetStore::new(asset_root.clone()).unwrap();
        CuratedRuntimeAuthorityFixture {
            request,
            source_save,
            world,
            durable_root,
            durable_save_path,
            asset_root,
            archive_root,
            durability: Some(GpuLiveCheckpointDurability {
                store,
                durable_manifest,
                published,
            }),
            lineage_library: Some(lineage_library),
            archive_run_id: "curated-runtime-archive".to_string(),
        }
    }

    fn seed_retained_operation(
        fixture: &CuratedRuntimeAuthorityFixture,
        retained_operation: &mut Option<CuratedFounderDurableOperation>,
    ) -> String {
        let plan = plan_curated_founder_reset(&fixture.request).unwrap();
        let bundle = materialize_curated_founder_bundle(&plan).unwrap();
        let durability = fixture.durability.as_ref().unwrap();
        let lineage_library = fixture.lineage_library.as_ref().unwrap();
        let operation = CuratedFounderDurableOperation::bind_and_stage(
            &plan,
            bundle,
            &durability.durable_manifest,
            &fixture.world,
            lineage_library,
            &fixture.archive_run_id,
        )
        .unwrap();
        let proposed = operation.proposed_save_digest().to_string();
        *retained_operation = Some(operation);
        proposed
    }

    fn operation_fingerprint(
        operation: &CuratedFounderDurableOperation,
    ) -> (Vec<OrganismId>, Vec<(WorldEntityId, OrganismId)>, String) {
        operation.test_identity_fingerprint()
    }

    struct ArchiveAttachmentTestCarrier {
        checkpoint_durability: Option<GpuLiveCheckpointDurability>,
        residents: BTreeMap<u64, ResidentCognition>,
        sensor_profile: SensorProfile,
        world_tick: Tick,
        lineage_library: Option<LineageLibrary>,
        lineage_run_id: Option<String>,
        archive_learned_capture_policy: ArchiveLearnedCapturePolicy,
        archive_birth_manifests: BTreeMap<u64, Blake3Digest>,
    }

    impl ArchiveAttachmentTestCarrier {
        fn attach(
            &mut self,
            config: LineageLibraryConfig,
            learned_capture_policy: ArchiveLearnedCapturePolicy,
        ) -> Result<(), GameAppShellError> {
            attach_lineage_archive_with_owned_authorities(
                self.checkpoint_durability.as_ref(),
                self.sensor_profile,
                self.world_tick,
                &self.residents,
                &mut self.lineage_library,
                &mut self.lineage_run_id,
                &mut self.archive_learned_capture_policy,
                &mut self.archive_birth_manifests,
                config,
                learned_capture_policy,
            )
        }
    }

    struct ArchiveAttachmentTestFixture {
        durability: GpuLiveCheckpointDurability,
        residents: BTreeMap<u64, ResidentCognition>,
        sensor_profile: SensorProfile,
        world_tick: Tick,
        archive_root: PathBuf,
        durable_root: PathBuf,
        asset_root: PathBuf,
    }

    impl Drop for ArchiveAttachmentTestFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.archive_root);
            let _ = fs::remove_dir_all(&self.durable_root);
        }
    }

    fn archive_attachment_test_fixture(label: &str) -> ArchiveAttachmentTestFixture {
        let sensor_profile = SensorProfile::PrivilegedAffordanceV1;
        let world_tick = Tick::ZERO;
        let seed = 0xA11F_E3C3_3B01_0001;
        let source_world = HeadlessScenarioBuilder::new(seed)
            .agent("archive-attachment-0", OrganismId(101), Vec3f::ZERO)
            .agent(
                "archive-attachment-1",
                OrganismId(202),
                Vec3f::new(1.0, 0.0, 0.0),
            )
            .build()
            .unwrap();
        let source_save = PortableSaveFile::from_headless_world(
            "save-archive-attachment",
            &source_world,
            RuntimeConfig::deterministic_default(seed, BrainScaleTier::Nano512),
            AssetManifest::empty(),
            Vec::new(),
        )
        .unwrap();
        let suffix = format!("{}-{label}", std::process::id());
        let archive_root =
            std::env::temp_dir().join(format!("alife-production-archive-attachment-{suffix}"));
        let durable_root = std::env::temp_dir().join(format!(
            "alife-production-archive-attachment-durable-{suffix}"
        ));
        let _ = fs::remove_dir_all(&archive_root);
        let _ = fs::remove_dir_all(&durable_root);
        fs::create_dir_all(&durable_root).unwrap();
        let asset_root = std::env::current_dir().unwrap();
        let durable_save_path = durable_root.join("live-save.json");
        GpuDurableSaveManifest::publish_snapshot(&durable_save_path, &asset_root, &source_save)
            .unwrap();
        let durable_manifest =
            GpuDurableSaveManifest::open(&durable_save_path, &asset_root).unwrap();
        let published = durable_manifest.load().unwrap();
        let store = GpuCheckpointAssetStore::new(asset_root.clone()).unwrap();
        let residents = [101_u64, 202_u64]
            .into_iter()
            .map(|raw| {
                let (phenotype, genome, development) = compile_gpu_birth_components(
                    seed,
                    BrainScaleTier::Nano512,
                    OrganismId(raw),
                    world_tick,
                    sensor_profile,
                )
                .unwrap();
                let capacity =
                    BrainCapacityClass::production_for_id(phenotype.brain_class_id()).unwrap();
                let compiler_inputs = PhenotypeCompilerInputs::try_new_with_foundation_abi(
                    genome.clone(),
                    &capacity,
                    development.clone(),
                    sensor_profile,
                    phenotype.foundation_abi().clone(),
                )
                .unwrap();
                (
                    raw,
                    ResidentCognition {
                        phenotype,
                        compiler_inputs,
                        genome,
                        development,
                        homeostasis: HomeostaticSnapshot::baseline(world_tick),
                        sleep_scheduler: GpuSleepScheduler::new(
                            SleepConsolidationConfig::reference(),
                        )
                        .unwrap(),
                        next_sequence: 1,
                        language_grounding: LanguageGroundingLedger::default(),
                        life_statistics: PassiveLifeStatistics::new(OrganismId(raw), world_tick)
                            .unwrap(),
                        attention_hysteresis: alife_core::HysteresisState::default(),
                        predictor: GroundedSuccessorPredictor::default(),
                    },
                )
            })
            .collect();
        ArchiveAttachmentTestFixture {
            durability: GpuLiveCheckpointDurability {
                store,
                durable_manifest,
                published,
            },
            residents,
            sensor_profile,
            world_tick,
            archive_root,
            durable_root,
            asset_root,
        }
    }

    fn archive_attachment_test_carrier(
        fixture: &ArchiveAttachmentTestFixture,
        durability: Option<GpuLiveCheckpointDurability>,
    ) -> ArchiveAttachmentTestCarrier {
        ArchiveAttachmentTestCarrier {
            checkpoint_durability: durability.or_else(|| Some(fixture.durability.clone())),
            residents: fixture.residents.clone(),
            sensor_profile: fixture.sensor_profile,
            world_tick: fixture.world_tick,
            lineage_library: None,
            lineage_run_id: None,
            archive_learned_capture_policy: ArchiveLearnedCapturePolicy::GeneticOnly,
            archive_birth_manifests: BTreeMap::new(),
        }
    }

    #[test]
    fn production_archive_source_run_identity_exact_vector() {
        assert_eq!(
            lineage_source_run_id_for_fields(
                "save-vector",
                0x0102_0304_0506_0708,
                0x1112_1314_1516_1718,
                0x1920_2122_2324_2526,
                "generation-vector",
            ),
            "runtime-save-v1-a86cb6f2aa429bfe69efc1b82c1cdb5d046a5dcdb85dde9a45e8028bccf0e36c"
        );
    }

    #[test]
    fn production_archive_attachment_derives_save_bound_run_id() {
        let fixture = archive_attachment_test_fixture("identity");
        let config = LineageLibraryConfig::profile_default(&fixture.archive_root);
        let mut first = archive_attachment_test_carrier(&fixture, None);
        first
            .attach(config.clone(), ArchiveLearnedCapturePolicy::GeneticOnly)
            .unwrap();
        let first_root = first.lineage_library.as_ref().unwrap().root().to_path_buf();
        let first_run_id = first.lineage_run_id.clone().unwrap();
        let first_birth_manifests = first.archive_birth_manifests.clone();
        let first_manifest_count = first
            .lineage_library
            .as_ref()
            .unwrap()
            .manifest_count()
            .unwrap();
        assert_eq!(first_root, fixture.archive_root);
        assert!(first_run_id.starts_with("runtime-save-v1-"));
        assert_eq!(first_birth_manifests.len(), 2);
        assert_eq!(first_manifest_count, 2);

        let second_attach = first.attach(
            LineageLibraryConfig::profile_default(fixture.archive_root.join("must-not-be-opened")),
            ArchiveLearnedCapturePolicy::Pinned,
        );
        assert!(matches!(
            second_attach,
            Err(GameAppShellError::InvalidProductionFrontend { .. })
        ));
        assert_eq!(
            first.lineage_library.as_ref().unwrap().root(),
            first_root.as_path()
        );
        assert_eq!(first.lineage_run_id.as_deref(), Some(first_run_id.as_str()));
        assert_eq!(first.archive_birth_manifests, first_birth_manifests);
        assert_eq!(
            first
                .lineage_library
                .as_ref()
                .unwrap()
                .manifest_count()
                .unwrap(),
            first_manifest_count
        );

        let _ = first.lineage_library.take();
        let mut relaunch = archive_attachment_test_carrier(&fixture, None);
        relaunch
            .attach(config, ArchiveLearnedCapturePolicy::GeneticOnly)
            .unwrap();
        assert_eq!(
            relaunch.lineage_run_id.as_deref(),
            Some(first_run_id.as_str())
        );
        assert_eq!(relaunch.archive_birth_manifests, first_birth_manifests);
        assert_eq!(
            relaunch
                .lineage_library
                .as_ref()
                .unwrap()
                .manifest_count()
                .unwrap(),
            first_manifest_count
        );

        let changed_save_path = fixture.durable_root.join("changed-save.json");
        let mut changed_save = fixture.durability.published.save.clone();
        changed_save.save_id = "save-archive-attachment-changed".to_string();
        GpuDurableSaveManifest::publish_snapshot(
            &changed_save_path,
            &fixture.asset_root,
            &changed_save,
        )
        .unwrap();
        let changed_durable_manifest =
            GpuDurableSaveManifest::open(&changed_save_path, &fixture.asset_root).unwrap();
        let changed_durability = GpuLiveCheckpointDurability {
            store: GpuCheckpointAssetStore::new(fixture.asset_root.clone()).unwrap(),
            published: changed_durable_manifest.load().unwrap(),
            durable_manifest: changed_durable_manifest,
        };
        let changed_root = fixture.archive_root.join("changed-generation");
        let mut changed = archive_attachment_test_carrier(&fixture, Some(changed_durability));
        changed
            .attach(
                LineageLibraryConfig::profile_default(changed_root),
                ArchiveLearnedCapturePolicy::GeneticOnly,
            )
            .unwrap();
        assert_ne!(changed.lineage_run_id, Some(first_run_id.clone()));

        let mut malformed_residents = fixture.residents.clone();
        malformed_residents.values_mut().next().unwrap().genome.id = GenomeId(0);
        let failed_root = fixture.archive_root.join("failed-backfill");
        let mut failed = archive_attachment_test_carrier(&fixture, None);
        failed.residents = malformed_residents;
        assert!(failed
            .attach(
                LineageLibraryConfig::profile_default(failed_root),
                ArchiveLearnedCapturePolicy::GeneticOnly,
            )
            .is_err());
        assert!(failed.lineage_library.is_none());
        assert!(failed.lineage_run_id.is_none());
        assert!(failed.archive_birth_manifests.is_empty());
        assert!(matches!(
            failed.archive_learned_capture_policy,
            ArchiveLearnedCapturePolicy::GeneticOnly
        ));
    }

    fn run_curated_runtime_authority(
        fixture: &mut CuratedRuntimeAuthorityFixture,
        retained_operation: &mut Option<CuratedFounderDurableOperation>,
        request: Option<CuratedFounderResetRequest>,
    ) -> Result<CuratedFounderResetAttempt, CuratedFounderResetRuntimeError> {
        let archive_run_id = fixture.archive_run_id.clone();
        let mut retained_plan = None;
        attempt_curated_founder_reset_with_owned_authorities(
            &mut fixture.durability,
            &mut fixture.lineage_library,
            Some(&archive_run_id),
            &mut fixture.world,
            retained_operation,
            &mut retained_plan,
            request,
        )
    }

    fn run_curated_runtime_authority_with_plan(
        fixture: &mut CuratedRuntimeAuthorityFixture,
        retained_operation: &mut Option<CuratedFounderDurableOperation>,
        retained_plan: &mut Option<CuratedFounderGpuResidencyPlan>,
        request: Option<CuratedFounderResetRequest>,
    ) -> Result<CuratedFounderResetAttempt, CuratedFounderResetRuntimeError> {
        let archive_run_id = fixture.archive_run_id.clone();
        attempt_curated_founder_reset_with_owned_authorities(
            &mut fixture.durability,
            &mut fixture.lineage_library,
            Some(&archive_run_id),
            &mut fixture.world,
            retained_operation,
            retained_plan,
            request,
        )
    }

    fn run_curated_runtime_authority_with_refresh_failure(
        fixture: &mut CuratedRuntimeAuthorityFixture,
        retained_operation: &mut Option<CuratedFounderDurableOperation>,
        retained_plan: &mut Option<CuratedFounderGpuResidencyPlan>,
        request: Option<CuratedFounderResetRequest>,
    ) -> Result<CuratedFounderResetAttempt, CuratedFounderResetRuntimeError> {
        let archive_run_id = fixture.archive_run_id.clone();
        attempt_curated_founder_reset_with_owned_authorities_and_refresh(
            &mut fixture.durability,
            &mut fixture.lineage_library,
            Some(&archive_run_id),
            &mut fixture.world,
            retained_operation,
            retained_plan,
            request,
            |_durability, _expected_digest| {
                Err(GameAppShellError::InvalidProductionFrontend {
                    message: "test-forced post-publication refresh failure".to_string(),
                })
            },
        )
    }

    fn competing_valid_save(source: &PortableSaveFile) -> PortableSaveFile {
        let mut competing = source.clone();
        let mut world = source.restore_headless_world().unwrap();
        world.advance_tick();
        competing.replace_headless_world_snapshot(&world).unwrap();
        competing
    }

    fn assert_runtime_owned_curated_reset_call(
        runtime: &mut GpuLiveBrainRuntime,
        request: CuratedFounderResetRequest,
    ) {
        let _ = runtime.attempt_curated_founder_reset(request);
    }

    #[test]
    fn curated_reset_runtime_owned_call_seam_exists() {
        let _ = assert_runtime_owned_curated_reset_call
            as fn(&mut GpuLiveBrainRuntime, CuratedFounderResetRequest);
    }

    #[test]
    fn curated_reset_runtime_owned_authorities_publish_and_leave_gpu_residency_pending() {
        let mut fixture = curated_runtime_authority_fixture("publish");
        let mut retained_operation = None;
        let request = fixture.request.clone();
        let result =
            run_curated_runtime_authority(&mut fixture, &mut retained_operation, Some(request))
                .unwrap();

        assert_eq!(
            result.publication_status(),
            CuratedFounderPublicationStatus::Published
        );
        assert_eq!(result.save_state(), CuratedFounderSaveState::Verified);
        assert_eq!(
            result.gpu_residency_state(),
            CuratedFounderGpuResidencyState::Pending
        );
        assert_eq!(result.receipt().archive_receipt_count(), 3);
        assert_eq!(fixture.world.organism_registry().len(), 3);
        assert!(retained_operation.is_none());
        assert_eq!(
            fixture
                .durability
                .as_ref()
                .unwrap()
                .published
                .digest
                .as_str(),
            fixture
                .durability
                .as_ref()
                .unwrap()
                .durable_manifest
                .load()
                .unwrap()
                .digest
                .as_str()
        );
        assert_eq!(
            fixture
                .lineage_library
                .as_ref()
                .unwrap()
                .manifest_count()
                .unwrap(),
            3
        );
    }

    #[test]
    fn curated_reset_missing_owned_authority_fails_before_archive_save_or_world_mutation() {
        let mut fixture = curated_runtime_authority_fixture("missing-durability");
        let before = authority_snapshot(&fixture);
        let mut missing_durability = None;
        let mut retained_operation = None;
        let mut retained_plan = None;
        let archive_run_id = fixture.archive_run_id.clone();
        let request = fixture.request.clone();
        let error = attempt_curated_founder_reset_with_owned_authorities(
            &mut missing_durability,
            &mut fixture.lineage_library,
            Some(&archive_run_id),
            &mut fixture.world,
            &mut retained_operation,
            &mut retained_plan,
            Some(request),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            CuratedFounderResetRuntimeError::MissingDurability
        ));
        assert!(retained_operation.is_none());
        assert_authority_unchanged(before, &fixture);

        let mut fixture = curated_runtime_authority_fixture("missing-archive");
        let before = authority_snapshot(&fixture);
        let mut durability = fixture.durability.take();
        let mut missing_library = None;
        let mut retained_operation = None;
        let mut retained_plan = None;
        let archive_run_id = fixture.archive_run_id.clone();
        let request = fixture.request.clone();
        let error = attempt_curated_founder_reset_with_owned_authorities(
            &mut durability,
            &mut missing_library,
            Some(&archive_run_id),
            &mut fixture.world,
            &mut retained_operation,
            &mut retained_plan,
            Some(request),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            CuratedFounderResetRuntimeError::MissingLineageArchive
        ));
        assert!(retained_operation.is_none());
        assert_authority_unchanged(before, &fixture);
    }

    #[test]
    fn curated_reset_conflict_retry_reuses_exact_operation_without_double_archive() {
        let mut fixture = curated_runtime_authority_fixture("conflict-retry");
        let mut retained_operation = None;
        let proposed = seed_retained_operation(&fixture, &mut retained_operation);
        let old_world_signature = fixture.world.canonical_signature_digest().unwrap();
        let old_registry = registry_snapshot(&fixture.world);
        let competing_save = competing_valid_save(&fixture.source_save);
        GpuDurableSaveManifest::publish_snapshot(
            &fixture.durable_save_path,
            &fixture.asset_root,
            &competing_save,
        )
        .unwrap();

        let conflict =
            run_curated_runtime_authority(&mut fixture, &mut retained_operation, None).unwrap();
        assert_eq!(
            conflict.publication_status(),
            CuratedFounderPublicationStatus::ArchiveCommittedSaveConflict
        );
        assert_eq!(conflict.save_state(), CuratedFounderSaveState::Conflict);
        assert_eq!(
            conflict.gpu_residency_state(),
            CuratedFounderGpuResidencyState::NotStarted
        );
        assert_eq!(conflict.proposed_save_digest(), proposed);
        assert!(conflict.expected_save_digest().is_some());
        assert!(conflict.actual_save_digest().is_some());
        assert_eq!(conflict.receipt().archive_receipt_count(), 3);
        assert_eq!(
            retained_operation.as_ref().unwrap().proposed_save_digest(),
            proposed
        );
        assert_eq!(
            fixture.world.canonical_signature_digest().unwrap(),
            old_world_signature
        );
        assert_eq!(registry_snapshot(&fixture.world), old_registry);
        let archive_after_conflict =
            archive_snapshot(fixture.lineage_library.as_ref().unwrap().root());
        let archive_count_after_conflict = fixture
            .lineage_library
            .as_ref()
            .unwrap()
            .manifest_count()
            .unwrap();
        assert_eq!(archive_count_after_conflict, 3);

        GpuDurableSaveManifest::publish_snapshot(
            &fixture.durable_save_path,
            &fixture.asset_root,
            &fixture.source_save,
        )
        .unwrap();
        let retry =
            run_curated_runtime_authority(&mut fixture, &mut retained_operation, None).unwrap();
        assert!(matches!(
            retry.publication_status(),
            CuratedFounderPublicationStatus::Published
                | CuratedFounderPublicationStatus::AlreadyApplied
        ));
        assert_eq!(retry.save_state(), CuratedFounderSaveState::Verified);
        assert_eq!(retry.proposed_save_digest(), proposed);
        assert!(retained_operation.is_none());
        assert_eq!(
            fixture
                .lineage_library
                .as_ref()
                .unwrap()
                .manifest_count()
                .unwrap(),
            3
        );
        assert_eq!(
            archive_snapshot(fixture.lineage_library.as_ref().unwrap().root()),
            archive_after_conflict
        );
    }

    #[test]
    fn curated_reset_precommit_retry_restores_exact_retained_operation() {
        let mut fixture = curated_runtime_authority_fixture("precommit-retry");
        let mut retained_operation = None;
        let proposed = seed_retained_operation(&fixture, &mut retained_operation);
        let staged_fingerprint = operation_fingerprint(retained_operation.as_ref().unwrap());
        let original_world = fixture.world.clone();
        let competing_save = competing_valid_save(&fixture.source_save);
        GpuDurableSaveManifest::publish_snapshot(
            &fixture.durable_save_path,
            &fixture.asset_root,
            &competing_save,
        )
        .unwrap();

        let conflict =
            run_curated_runtime_authority(&mut fixture, &mut retained_operation, None).unwrap();
        assert_eq!(
            conflict.publication_status(),
            CuratedFounderPublicationStatus::ArchiveCommittedSaveConflict
        );
        assert_eq!(
            operation_fingerprint(retained_operation.as_ref().unwrap()),
            staged_fingerprint
        );
        let archive_after_conflict =
            archive_snapshot(fixture.lineage_library.as_ref().unwrap().root());
        let archive_count_after_conflict = fixture
            .lineage_library
            .as_ref()
            .unwrap()
            .manifest_count()
            .unwrap();

        fixture.world.advance_tick();
        let precommit =
            run_curated_runtime_authority(&mut fixture, &mut retained_operation, None).unwrap_err();
        assert!(matches!(
            precommit,
            CuratedFounderResetRuntimeError::PreCommit(CuratedFounderStagingError::Mismatch {
                field: "apply world tick"
            })
        ));
        assert!(
            retained_operation.is_some(),
            "a retained operation must survive a retry pre-commit error"
        );
        assert_eq!(
            operation_fingerprint(retained_operation.as_ref().unwrap()),
            staged_fingerprint
        );
        assert_eq!(
            retained_operation.as_ref().unwrap().proposed_save_digest(),
            proposed
        );
        assert_eq!(
            fixture
                .lineage_library
                .as_ref()
                .unwrap()
                .manifest_count()
                .unwrap(),
            archive_count_after_conflict
        );
        assert_eq!(
            archive_snapshot(fixture.lineage_library.as_ref().unwrap().root()),
            archive_after_conflict
        );

        fixture.world = original_world;
        GpuDurableSaveManifest::publish_snapshot(
            &fixture.durable_save_path,
            &fixture.asset_root,
            &fixture.source_save,
        )
        .unwrap();
        let retry =
            run_curated_runtime_authority(&mut fixture, &mut retained_operation, None).unwrap();
        assert!(matches!(
            retry.publication_status(),
            CuratedFounderPublicationStatus::Published
                | CuratedFounderPublicationStatus::AlreadyApplied
        ));
        assert_eq!(retry.proposed_save_digest(), proposed);
        assert!(retained_operation.is_none());
        assert_eq!(
            fixture
                .lineage_library
                .as_ref()
                .unwrap()
                .manifest_count()
                .unwrap(),
            archive_count_after_conflict
        );
        assert_eq!(
            archive_snapshot(fixture.lineage_library.as_ref().unwrap().root()),
            archive_after_conflict
        );
    }

    #[test]
    fn curated_reset_post_archive_failure_retains_unknown_receipt_and_retries_exact_operation() {
        let mut fixture = curated_runtime_authority_fixture("failure-retry");
        let mut retained_operation = None;
        let proposed = seed_retained_operation(&fixture, &mut retained_operation);
        let old_world_signature = fixture.world.canonical_signature_digest().unwrap();
        let old_registry = registry_snapshot(&fixture.world);
        let durable_root = fixture.durable_root.clone();
        fs::remove_dir_all(&durable_root).unwrap();

        let failure =
            run_curated_runtime_authority(&mut fixture, &mut retained_operation, None).unwrap();
        assert_eq!(
            failure.publication_status(),
            CuratedFounderPublicationStatus::ArchiveCommittedSaveFailure
        );
        assert_eq!(failure.save_state(), CuratedFounderSaveState::Unknown);
        assert_eq!(
            failure.gpu_residency_state(),
            CuratedFounderGpuResidencyState::NotStarted
        );
        assert_eq!(failure.proposed_save_digest(), proposed);
        assert!(failure.cause().is_some());
        assert_eq!(failure.receipt().archive_receipt_count(), 3);
        assert!(retained_operation.is_some());
        assert_eq!(
            fixture.world.canonical_signature_digest().unwrap(),
            old_world_signature
        );
        assert_eq!(registry_snapshot(&fixture.world), old_registry);
        let archive_after_failure =
            archive_snapshot(fixture.lineage_library.as_ref().unwrap().root());
        let archive_count_after_failure = fixture
            .lineage_library
            .as_ref()
            .unwrap()
            .manifest_count()
            .unwrap();
        assert_eq!(archive_count_after_failure, 3);

        fs::create_dir_all(&durable_root).unwrap();
        GpuDurableSaveManifest::publish_snapshot(
            &fixture.durable_save_path,
            &fixture.asset_root,
            &fixture.source_save,
        )
        .unwrap();
        let retry =
            run_curated_runtime_authority(&mut fixture, &mut retained_operation, None).unwrap();
        assert!(matches!(
            retry.publication_status(),
            CuratedFounderPublicationStatus::Published
                | CuratedFounderPublicationStatus::AlreadyApplied
        ));
        assert_eq!(retry.save_state(), CuratedFounderSaveState::Verified);
        assert_eq!(retry.proposed_save_digest(), proposed);
        assert!(retained_operation.is_none());
        assert_eq!(
            fixture
                .lineage_library
                .as_ref()
                .unwrap()
                .manifest_count()
                .unwrap(),
            3
        );
        assert_eq!(
            archive_snapshot(fixture.lineage_library.as_ref().unwrap().root()),
            archive_after_failure
        );
    }

    #[test]
    fn published_curated_reset_retains_receipt_bound_gpu_residency_plan() {
        fn reduce_to_two_entries(fixture: &mut CuratedRuntimeAuthorityFixture) {
            fixture
                .request
                .final_agents
                .sort_by_key(|agent| agent.final_population_slot);
            fixture.request.final_agents.truncate(2);
            fixture.request.target_population = 2;
        }

        fn assert_plan_matches_accepted_bundle(
            plan: &CuratedFounderGpuResidencyPlan,
            bundle: &crate::curated_founder_materializer::CuratedFounderBundle,
            result: &CuratedFounderResetAttempt,
            source_run_identity: &str,
        ) {
            assert_eq!(plan.entries.len(), 2);
            assert_eq!(plan.source_run_identity, source_run_identity);
            assert_eq!(
                plan.final_save_digest,
                result.receipt().final_save_digest().unwrap()
            );
            assert_eq!(
                plan.candidate_world_signature,
                result.receipt().candidate_world_signature()
            );
            assert_eq!(plan.world_seed, result.receipt().candidate_world_seed());
            assert_eq!(plan.world_tick, result.receipt().candidate_world_tick());

            let archive_identities = result.receipt().archive_receipt_identities();
            assert_eq!(archive_identities.len(), bundle.entries.len());
            for ((planned, accepted), archived) in plan
                .entries
                .iter()
                .zip(&bundle.entries)
                .zip(archive_identities)
            {
                assert_eq!(
                    planned.final_population_slot,
                    accepted.plan_entry.final_population_slot
                );
                assert_eq!(planned.world_entity_id, accepted.plan_entry.world_entity_id);
                assert_eq!(planned.organism_id, accepted.plan_entry.organism_id);
                assert_eq!(planned.lineage_id, accepted.plan_entry.lineage_id);
                assert_eq!(planned.projection, accepted.projection);
                assert_eq!(
                    planned.projection.foundation_asset_digest(),
                    accepted.projection.foundation_asset_digest()
                );
                assert_eq!(
                    planned.projection.sensor_profile(),
                    accepted.projection.sensor_profile()
                );
                assert_eq!(
                    planned.projection.receipt().phenotype_hash(),
                    accepted.projection.receipt().phenotype_hash()
                );
                assert_eq!(
                    planned.projection.receipt().digest(),
                    accepted.projection.receipt().digest()
                );
                assert_eq!(
                    (
                        planned.final_population_slot,
                        planned.world_entity_id,
                        planned.organism_id
                    ),
                    (archived.0, archived.1, archived.2)
                );
                assert_eq!(planned.lineage_id, archived.3);
                assert_eq!(planned.archive_birth_manifest_digest, archived.4);
            }
            assert_eq!(
                plan.fingerprint,
                curated_founder_gpu_residency_plan_fingerprint(plan)
            );
        }

        let mut conflict_fixture = curated_runtime_authority_fixture("gpu-plan-conflict");
        reduce_to_two_entries(&mut conflict_fixture);
        let mut retained_operation = None;
        let mut retained_plan = None;
        let proposed = seed_retained_operation(&conflict_fixture, &mut retained_operation);
        let accepted_bundle = retained_operation
            .as_ref()
            .unwrap()
            .accepted_bundle()
            .clone();
        assert!(retained_plan.is_none(), "a retained operation is not GPU residency");
        let retained_fingerprint = operation_fingerprint(retained_operation.as_ref().unwrap());

        let competing = competing_valid_save(&conflict_fixture.source_save);
        GpuDurableSaveManifest::publish_snapshot(
            &conflict_fixture.durable_save_path,
            &conflict_fixture.asset_root,
            &competing,
        )
        .unwrap();
        let conflict = run_curated_runtime_authority_with_plan(
            &mut conflict_fixture,
            &mut retained_operation,
            &mut retained_plan,
            None,
        )
        .unwrap();
        assert_eq!(
            conflict.publication_status(),
            CuratedFounderPublicationStatus::ArchiveCommittedSaveConflict
        );
        assert_eq!(
            conflict.gpu_residency_state(),
            CuratedFounderGpuResidencyState::NotStarted
        );
        assert!(retained_plan.is_none());
        assert_eq!(
            operation_fingerprint(retained_operation.as_ref().unwrap()),
            retained_fingerprint
        );
        assert_eq!(conflict.proposed_save_digest(), proposed);

        conflict_fixture.world.advance_tick();
        let precommit = run_curated_runtime_authority_with_plan(
            &mut conflict_fixture,
            &mut retained_operation,
            &mut retained_plan,
            None,
        )
        .unwrap_err();
        assert!(matches!(
            precommit,
            CuratedFounderResetRuntimeError::PreCommit(
                CuratedFounderStagingError::Mismatch {
                    field: "apply world tick"
                }
            )
        ));
        assert!(retained_plan.is_none());
        assert_eq!(
            operation_fingerprint(retained_operation.as_ref().unwrap()),
            retained_fingerprint
        );

        conflict_fixture.world = conflict_fixture.source_save.restore_headless_world().unwrap();
        GpuDurableSaveManifest::publish_snapshot(
            &conflict_fixture.durable_save_path,
            &conflict_fixture.asset_root,
            &conflict_fixture.source_save,
        )
        .unwrap();
        let retried = run_curated_runtime_authority_with_plan(
            &mut conflict_fixture,
            &mut retained_operation,
            &mut retained_plan,
            None,
        )
        .unwrap();
        assert!(matches!(
            retried.publication_status(),
            CuratedFounderPublicationStatus::Published
                | CuratedFounderPublicationStatus::AlreadyApplied
        ));
        assert_eq!(
            retried.gpu_residency_state(),
            CuratedFounderGpuResidencyState::Pending
        );
        assert!(retained_operation.is_none());
        assert_plan_matches_accepted_bundle(
            retained_plan.as_ref().unwrap(),
            &accepted_bundle,
            &retried,
            &conflict_fixture.archive_run_id,
        );
        let pending_plan = retained_plan.as_ref().unwrap().clone();
        let pending_attempt_request = conflict_fixture.request.clone();
        let pending_attempt = run_curated_runtime_authority_with_plan(
            &mut conflict_fixture,
            &mut retained_operation,
            &mut retained_plan,
            Some(pending_attempt_request),
        )
        .unwrap_err();
        assert!(matches!(
            pending_attempt,
            CuratedFounderResetRuntimeError::RetainedResidencyPlanPending
        ));
        assert!(retained_operation.is_none());
        assert_eq!(retained_plan.as_ref(), Some(&pending_plan));

        let mut already_applied_fixture = curated_runtime_authority_fixture("gpu-plan-already");
        reduce_to_two_entries(&mut already_applied_fixture);
        let mut already_operation = None;
        let mut already_plan = None;
        seed_retained_operation(&already_applied_fixture, &mut already_operation);
        let already_bundle = already_operation
            .as_ref()
            .unwrap()
            .accepted_bundle()
            .clone();
        let replacement = already_operation
            .as_ref()
            .unwrap()
            .test_replacement_save();
        GpuDurableSaveManifest::publish_snapshot(
            &already_applied_fixture.durable_save_path,
            &already_applied_fixture.asset_root,
            &replacement,
        )
        .unwrap();
        let already_applied = run_curated_runtime_authority_with_plan(
            &mut already_applied_fixture,
            &mut already_operation,
            &mut already_plan,
            None,
        )
        .unwrap();
        assert_eq!(
            already_applied.publication_status(),
            CuratedFounderPublicationStatus::AlreadyApplied
        );
        assert_eq!(
            already_applied.gpu_residency_state(),
            CuratedFounderGpuResidencyState::Pending
        );
        assert_plan_matches_accepted_bundle(
            already_plan.as_ref().unwrap(),
            &already_bundle,
            &already_applied,
            &already_applied_fixture.archive_run_id,
        );

        let mut refresh_fixture = curated_runtime_authority_fixture("gpu-plan-refresh-failure");
        reduce_to_two_entries(&mut refresh_fixture);
        let mut refresh_operation = None;
        let mut refresh_plan = None;
        seed_retained_operation(&refresh_fixture, &mut refresh_operation);
        let refresh_bundle = refresh_operation
            .as_ref()
            .unwrap()
            .accepted_bundle()
            .clone();
        let refresh_error = run_curated_runtime_authority_with_refresh_failure(
            &mut refresh_fixture,
            &mut refresh_operation,
            &mut refresh_plan,
            None,
        )
        .unwrap_err();
        let projected_refresh_failure =
            project_curated_founder_reset_runtime_error(refresh_error);
        assert!(matches!(
            projected_refresh_failure,
            CuratedFounderResetDispatchResult::Unknown {
                gpu_residency: CuratedFounderGpuResidencyState::Unknown,
                retryable: false,
                ..
            }
        ));
        assert!(refresh_operation.is_none());
        let retained_refresh_plan = refresh_plan.as_ref().unwrap().clone();
        assert_eq!(
            retained_refresh_plan.state,
            CuratedFounderGpuResidencyState::Unknown
        );
        assert_eq!(retained_refresh_plan.entries.len(), refresh_bundle.entries.len());
        assert_eq!(
            retained_refresh_plan.fingerprint,
            curated_founder_gpu_residency_plan_fingerprint(&retained_refresh_plan)
        );

        let before_retry = authority_snapshot(&refresh_fixture);
        let retry_error = run_curated_runtime_authority_with_plan(
            &mut refresh_fixture,
            &mut refresh_operation,
            &mut refresh_plan,
            None,
        )
        .unwrap_err();
        assert!(matches!(
            retry_error,
            CuratedFounderResetRuntimeError::NoRetainedOperation
        ));
        assert!(refresh_operation.is_none());
        assert_eq!(refresh_plan.as_ref(), Some(&retained_refresh_plan));
        assert_authority_unchanged(before_retry, &refresh_fixture);

        let new_attempt_request = refresh_fixture.request.clone();
        let before_new_attempt = authority_snapshot(&refresh_fixture);
        let new_attempt_error = run_curated_runtime_authority_with_plan(
            &mut refresh_fixture,
            &mut refresh_operation,
            &mut refresh_plan,
            Some(new_attempt_request),
        )
        .unwrap_err();
        assert!(matches!(
            new_attempt_error,
            CuratedFounderResetRuntimeError::RetainedResidencyPlanPending
        ));
        assert!(refresh_operation.is_none());
        assert_eq!(refresh_plan.as_ref(), Some(&retained_refresh_plan));
        assert_authority_unchanged(before_new_attempt, &refresh_fixture);
    }

    #[test]
    fn gpu_restore_resident_identity_uses_world_record_and_rejects_checkpoint_metadata_drift() {
        let organism_id = OrganismId::new(77).unwrap();
        let sensor_profile = SensorProfile::PrivilegedAffordanceV1;
        let mut world = HeadlessScenarioBuilder::new(0x3_3B_00_0001)
            .agent("record-authority", organism_id, Vec3f::ZERO)
            .build()
            .unwrap();
        let world_entity_id = world
            .organism_entity_ids()
            .into_iter()
            .find(|(candidate, _)| *candidate == organism_id)
            .map(|(_, entity)| entity)
            .unwrap();
        let foundation_asset = FoundationWeightAsset::builtin_nano512_v1(sensor_profile).unwrap();
        let foundation_manifest = foundation_asset.manifest();
        let foundation = FoundationGeneticIdentity::new(
            foundation_manifest.foundation_id().raw(),
            foundation_manifest.foundation_version().raw() as u16,
            foundation_manifest.compatibility_family_id().raw(),
            BrainCapacityClass::N512_ID,
        )
        .unwrap();
        let genome = alife_core::CreatureGenome::early_mammal_founder(
            0x3_3B_00_0011,
            foundation,
        )
        .unwrap();
        let phenotype = genome.express().unwrap();
        let biochemistry =
            alife_core::BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
        world
            .register_organism_record(
                WorldOrganismRecord::new(
                    organism_id,
                    world_entity_id,
                    genome,
                    phenotype,
                    biochemistry,
                    Tick::ZERO,
                )
                .unwrap(),
            )
            .unwrap();
        let initial_homeostasis = world
            .organism_registry()
            .get(organism_id)
            .unwrap()
            .biochemistry()
            .homeostasis;
        let frame = world
            .perception_frame(
                organism_id,
                Tick::ZERO,
                sensor_profile,
                initial_homeostasis,
            )
            .unwrap();
        let rest = *frame
            .candidates()
            .iter()
            .find(|candidate| candidate.kind == alife_core::ActionKind::Rest)
            .expect("rest candidate");
        let command = rest
            .to_command(organism_id, Confidence::new(1.0).unwrap())
            .unwrap();
        world
            .apply_registered_neural_command(
                &command,
                world_entity_id,
                Tick::new(1),
                None,
                false,
            )
            .unwrap();
        world.advance_tick();

        let record = world.organism_registry().get(organism_id).unwrap();
        let (_, scaffold_genome, _) = compile_gpu_birth_components(
            0x3_3B_00_0001,
            BrainScaleTier::Nano512,
            organism_id,
            world.tick(),
            sensor_profile,
        )
        .unwrap();
        assert_ne!(record.phenotype().brain_genome, scaffold_genome);
        assert!(record.age_at(world.tick()).unwrap().raw() > 0);
        assert_ne!(
            record.biochemistry().homeostasis,
            HomeostaticSnapshot::baseline(world.tick())
        );

        let plan = resident_authority_plan_from_record(
            record,
            organism_id,
            world_entity_id,
            world.tick(),
            BrainScaleTier::Nano512,
            sensor_profile,
        )
        .unwrap();
        assert_eq!(plan.world_entity_id, world_entity_id);
        let authoritative_age = record.age_at(world.tick()).unwrap();
        let authoritative_development = record
            .phenotype()
            .development_state_at(authoritative_age)
            .unwrap();
        assert_eq!(plan.genome, record.phenotype().brain_genome);
        assert_eq!(
            plan.compiler_inputs.genome(),
            &record.phenotype().brain_genome
        );
        assert_eq!(plan.development.age_ticks, authoritative_age);
        assert_eq!(
            plan.development.genome_id,
            record.phenotype().brain_genome.id
        );
        assert_eq!(plan.development, authoritative_development);
        let (expected_phenotype, expected_inputs) = compile_gpu_components_from_genome(
            record.phenotype().brain_genome.clone(),
            authoritative_development,
            sensor_profile,
        )
        .unwrap();
        assert_eq!(plan.phenotype, expected_phenotype);
        assert_eq!(plan.compiler_inputs.genome(), expected_inputs.genome());
        assert_eq!(
            plan.compiler_inputs.development(),
            expected_inputs.development()
        );
        assert_eq!(
            plan.biochemistry.homeostasis,
            record.biochemistry().homeostasis
        );

        let (_, checkpoint_genome, checkpoint_development) = compile_gpu_birth_components(
            0x3_3B_00_0002,
            BrainScaleTier::Nano512,
            organism_id,
            world.tick(),
            sensor_profile,
        )
        .unwrap();
        let (checkpoint_phenotype, checkpoint_inputs) = compile_gpu_components_from_genome(
            checkpoint_genome,
            checkpoint_development,
            sensor_profile,
        )
        .unwrap();
        let comparison = compare_resident_checkpoint_metadata(
            &plan,
            ResidentCheckpointMetadata {
                organism_id,
                phenotype_hash: checkpoint_phenotype.phenotype_hash(),
                capacity_class_id: checkpoint_phenotype.brain_class_id(),
                checkpoint_tick: world.tick(),
                phenotype: &checkpoint_phenotype,
                compiler_inputs: &checkpoint_inputs,
            },
        );
        assert_eq!(comparison, Err(ScaffoldContractError::PhenotypeCompile));
        let accepted_authority_plan = comparison.ok().map(|_| plan.clone());
        assert!(accepted_authority_plan.is_none());
    }

    #[test]
    fn n2048_gpu_birth_uses_foundation_bound_compiler_inputs() {
        let (phenotype, genome, development) = compile_gpu_birth_components(
            0xB17A_DA7A,
            BrainScaleTier::Standard2048,
            OrganismId::new(77).unwrap(),
            Tick::ZERO,
            SensorProfile::PrivilegedAffordanceV1,
        )
        .unwrap();
        assert_eq!(phenotype.brain_class_id(), BrainCapacityClass::N2048_ID);
        assert!(phenotype
            .foundation_abi()
            .foundation_payload_digest()
            .is_some());
        let capacity = BrainCapacityClass::production_for_id(phenotype.brain_class_id()).unwrap();
        let inputs = PhenotypeCompilerInputs::try_new_with_foundation_abi(
            genome,
            &capacity,
            development,
            SensorProfile::PrivilegedAffordanceV1,
            phenotype.foundation_abi().clone(),
        )
        .unwrap();
        assert_eq!(inputs.foundation_abi(), phenotype.foundation_abi());
    }

    #[test]
    fn checked_in_n2048_assets_decode_for_every_production_sensor_profile() {
        for (index, profile) in [
            SensorProfile::PrivilegedAffordanceV1,
            SensorProfile::GroundedObjectSlotsV1,
        ]
        .into_iter()
        .enumerate()
        {
            let asset = alife_core::FoundationWeightAsset::builtin_n2048_v1(profile).unwrap();
            assert_eq!(asset.manifest().training_stage().completed_stage_count(), 0);
            assert!(!asset.manifest().promotion_receipt().is_promoted());
            assert!(!asset.weights().is_empty());
            let (phenotype, _, _) = compile_gpu_birth_components(
                0xB17A_DA7B,
                BrainScaleTier::Standard2048,
                OrganismId::new(78 + index as u64).unwrap(),
                Tick::ZERO,
                profile,
            )
            .unwrap();
            assert_eq!(phenotype.sensor_profile(), profile);
            assert_eq!(
                phenotype.foundation_abi().foundation_payload_digest(),
                Some(asset.digest())
            );
        }
    }

    #[test]
    fn n512_gpu_birth_uses_checked_foundation_and_rejects_unsupported_classes() {
        for (organism_id, profile) in [
            (79, SensorProfile::PrivilegedAffordanceV1),
            (80, SensorProfile::GroundedObjectSlotsV1),
        ] {
            let asset = alife_core::FoundationWeightAsset::builtin_nano512_v1(profile).unwrap();
            let (phenotype, _, _) = compile_gpu_birth_components(
                0xB17A_DA7C,
                BrainScaleTier::Nano512,
                OrganismId::new(organism_id).unwrap(),
                Tick::ZERO,
                profile,
            )
            .unwrap();
            let abi = phenotype.foundation_abi();

            assert_eq!(abi.capacity_class_id(), BrainCapacityClass::N512_ID);
            assert_eq!(phenotype.sensor_profile(), profile);
            assert_eq!(
                abi.foundation_id().map(|id| id.raw()),
                Some(0x004E_3531_325F_5631)
            );
            assert_eq!(
                abi.compatibility_family_id().map(|id| id.raw()),
                Some(0x4E35_3132_5F00_FA11)
            );
            assert_eq!(abi.foundation_payload_digest(), Some(asset.digest()));
        }

        let (n2048, _, _) = compile_gpu_birth_components(
            0xB17A_DA7D,
            BrainScaleTier::Standard2048,
            OrganismId::new(81).unwrap(),
            Tick::ZERO,
            SensorProfile::PrivilegedAffordanceV1,
        )
        .unwrap();
        let n2048_asset = alife_core::FoundationWeightAsset::builtin_n2048_v1(
            SensorProfile::PrivilegedAffordanceV1,
        )
        .unwrap();
        assert_eq!(n2048.brain_class_id(), BrainCapacityClass::N2048_ID);
        assert_eq!(
            n2048.foundation_abi().foundation_payload_digest(),
            Some(n2048_asset.digest())
        );
        assert_ne!(
            n2048.foundation_abi().foundation_id().unwrap().raw(),
            0x004E_3531_325F_5631
        );

        assert!(compile_gpu_birth_components(
            0xB17A_DA7E,
            BrainScaleTier::Small1024,
            OrganismId::new(82).unwrap(),
            Tick::ZERO,
            SensorProfile::PrivilegedAffordanceV1,
        )
        .is_err());
    }

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
            GPU_CLOSED_LOOP_TICK_READBACK_BYTES
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
        assert_eq!(
            metrics.selection_readback_bytes,
            GPU_CLOSED_LOOP_TICK_READBACK_BYTES
        );
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
    fn seal_prepared_selection_uses_world_biology_receipt_as_resident_authority() {
        let organism_id = OrganismId(1);
        let mut world = HeadlessScenarioBuilder::new(9_308)
            .agent("agent", organism_id, Vec3f::ZERO)
            .build()
            .unwrap();
        let world_entity_id = world
            .organism_entity_ids()
            .into_iter()
            .find(|(bound_organism_id, _)| *bound_organism_id == organism_id)
            .map(|(_, world_entity_id)| world_entity_id)
            .unwrap();
        let biology_before = world
            .organism_registry()
            .get(organism_id)
            .unwrap()
            .biochemistry()
            .clone();
        let normal = world
            .perception_frame(
                organism_id,
                Tick::ZERO,
                SensorProfile::PrivilegedAffordanceV1,
                biology_before.homeostasis,
            )
            .unwrap();
        let mut rest = *normal
            .candidates()
            .iter()
            .find(|candidate| candidate.kind == alife_core::ActionKind::Rest)
            .expect("rest candidate");
        rest.candidate_index = 0;
        let draft = alife_core::PerceptionFrameDraft::new(
            organism_id,
            Tick::ZERO,
            SensorProfile::PrivilegedAffordanceV1,
            normal.sensory().clone(),
            normal.body(),
            *normal.homeostasis(),
            vec![rest],
            normal.profile_provenance(),
            normal.grounded_object_slots().to_vec(),
        )
        .unwrap();
        let frame = draft
            .finalize(alife_core::PerceptionContextBlock::empty())
            .unwrap();
        let (phenotype, genome, development) = compile_gpu_birth_components(
            9_308,
            BrainScaleTier::Nano512,
            organism_id,
            Tick::ZERO,
            SensorProfile::PrivilegedAffordanceV1,
        )
        .unwrap();
        let capacity = BrainCapacityClass::production_for_id(phenotype.brain_class_id()).unwrap();
        let compiler_inputs = PhenotypeCompilerInputs::try_new_with_foundation_abi(
            genome.clone(),
            &capacity,
            development.clone(),
            SensorProfile::PrivilegedAffordanceV1,
            phenotype.foundation_abi().clone(),
        )
        .unwrap();
        let mut residents = BTreeMap::from([(
            organism_id.raw(),
            ResidentCognition {
                phenotype: phenotype.clone(),
                compiler_inputs,
                genome: genome.clone(),
                development: development.clone(),
                homeostasis: biology_before.homeostasis,
                sleep_scheduler: GpuSleepScheduler::new(
                    SleepConsolidationConfig::reference(),
                )
                .unwrap(),
                next_sequence: 1,
                language_grounding: LanguageGroundingLedger::default(),
                life_statistics: PassiveLifeStatistics::new(organism_id, Tick::ZERO).unwrap(),
                attention_hysteresis: alife_core::HysteresisState::default(),
                predictor: GroundedSuccessorPredictor::default(),
            },
        )]);
        let confidence = Confidence::new(1.0).unwrap();
        let command = rest.to_command(organism_id, confidence).unwrap();
        let sequence_id = ExperienceSequenceId(1);
        let pre_action = PreActionSnapshot::from_neural_frame(
            sequence_id,
            phenotype.brain_class_id(),
            phenotype.phenotype_hash(),
            genome.id,
            genome.schema_version,
            development,
            frame.clone(),
        )
        .unwrap();
        let decision = DecisionSnapshot::from_neural_selection(
            sequence_id,
            phenotype.phenotype_hash(),
            1,
            0,
            &frame,
            NeuralActionSelection {
                candidate_index: 0,
                logit: 0.0,
                confidence,
                active_tiles: 1,
                active_synapses: 1,
            },
            command,
        )
        .unwrap();
        let mut world_before = world.clone();
        let expected_receipt = world_before
            .apply_registered_neural_command(
                &decision.selected_action,
                world_entity_id,
                Tick::new(1),
                None,
                false,
            )
            .unwrap();
        let cognitive_context =
            CognitiveContextFrame::empty(organism_id, sequence_id, frame.tick()).unwrap();
        let memory = MemoryRecallReceipt {
            schema_version: 1,
            organism_id_raw: organism_id.raw(),
            input_generation: 1,
            bank_digest: [0; 4],
            base_frame_digest: frame.base_digest(),
            context_digest: frame.context().canonical_digest(),
            candidate_count: 0,
            exact_bucket_reads: 0,
            neighbor_bucket_reads: 0,
            similarity_evaluations: 0,
            candidates: Vec::new(),
            degradations: Vec::new(),
        };
        let work = BrainWorkReceipt {
            schema_version: 1,
            class_id_raw: 0,
            organism_id_raw: organism_id.raw(),
            tick: frame.tick().raw(),
            handle_slot: 0,
            handle_generation: 0,
            dispatch_generation: 1,
            frame_digest: frame.frame_digest().0,
            sequence_cursor: sequence_id.raw(),
            counters: Default::default(),
            route_schedule_digest: [0; 4],
            neural_cost_q24: 0,
            atp_before_q16: 0,
            atp_debit_q16: 0,
            atp_after_q16: 0,
            receipt_digest: [0; 4],
        };
        let sealed = seal_prepared_selection_core(
            &mut world,
            &mut residents,
            0,
            CognitiveWorkCostPolicy::disabled(),
            false,
            PreparedSealInput {
                organism_id,
                world_entity_id,
                frame,
                memory,
                sequence_id,
                outcome_tick: Tick::new(1),
                cognitive_context,
                work,
                pre_action,
                decision,
                motor_bundle: compatibility_bundle_for_selected_action_v1(
                    organism_id,
                    sequence_id,
                    frame.tick(),
                    &decision.selected_action,
                )
                .unwrap(),
                speech_payload: None,
                speech_prompted: false,
            },
        )
        .unwrap();
        let world_after = *world
            .organism_registry()
            .get(organism_id)
            .unwrap()
            .biochemistry();
        assert_eq!(sealed.patch.header().abi_version, ExperiencePatch::V11_ABI_VERSION);
        assert!(sealed.patch.prediction_target().is_some());
        assert_eq!(
            world
                .organism_registry()
                .get(organism_id)
                .unwrap()
                .cognitive_work(),
            sealed.patch.cognitive_work().unwrap()
        );

        assert_eq!(expected_receipt.action_result.body_event.sleep_recovery, 1.0);
        assert_eq!(
            expected_receipt
                .action_result
                .observation
                .homeostatic_delta
                .drives
                .fatigue,
            -0.35
        );
        let learning_projection = expected_receipt
            .biology_before
            .homeostasis
            .advance(
                expected_receipt.outcome_tick,
                expected_receipt.action_result.observation.homeostatic_delta,
                HomeostaticParameters::reference(),
            )
            .unwrap();
        assert_ne!(expected_receipt.biology_after.homeostasis, learning_projection);
        assert_eq!(world_after.homeostasis, expected_receipt.biology_after.homeostasis);
        assert_eq!(
            residents.get(&organism_id.raw()).unwrap().homeostasis,
            expected_receipt.biology_after.homeostasis
        );
        let next_frame = world
            .perception_frame(
                organism_id,
                expected_receipt.outcome_tick,
                SensorProfile::PrivilegedAffordanceV1,
                residents.get(&organism_id.raw()).unwrap().homeostasis,
            )
            .unwrap();
        assert_eq!(
            *next_frame.homeostasis(),
            expected_receipt.biology_after.homeostasis
        );
        assert_eq!(
            sealed.patch.outcome().homeostatic_delta,
            expected_receipt.action_result.observation.homeostatic_delta
        );
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
        let world_entity_id = runtime
            .world
            .organism_entity_ids()
            .into_iter()
            .find(|(bound_organism_id, _)| *bound_organism_id == organism_id)
            .map(|(_, world_entity_id)| world_entity_id)
            .unwrap();
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
                world_entity_id,
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
        let world_entity_id = runtime
            .world
            .organism_entity_ids()
            .into_iter()
            .find(|(bound_organism_id, _)| *bound_organism_id == OrganismId(1))
            .map(|(_, world_entity_id)| world_entity_id)
            .unwrap();
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
                    world_entity_id,
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

    #[cfg(feature = "gpu-tests")]
    fn archived_newborn_runtime(label: &str) -> (GpuLiveBrainRuntime, PathBuf) {
        let sensor_profile = SensorProfile::PrivilegedAffordanceV1;
        let seed = 0x43B1_0001;
        let world = HeadlessScenarioBuilder::new(seed)
            .agent("parent-a", OrganismId(1), Vec3f::ZERO)
            .agent("parent-b", OrganismId(2), Vec3f::new(1.0, 0.0, 0.0))
            .build()
            .unwrap();
        let archive_root = std::env::temp_dir().join(format!(
            "alife-gpu-newborn-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&archive_root);
        let backend = GpuClosedLoopBackend::new_in_process(
            alife_gpu_backend::GpuRuntimeProfile::production_v1(),
        )
        .expect("in-process GPU backend");
        let runtime = GpuLiveBrainRuntime::new_profiled_archived(
            backend,
            world,
            seed,
            BrainScaleTier::Nano512,
            sensor_profile,
            LineageLibraryConfig::profile_default(&archive_root),
            "task-4.3b1-newborn",
            ArchiveLearnedCapturePolicy::GeneticOnly,
        )
        .unwrap();
        (runtime, archive_root)
    }

    #[cfg(feature = "gpu-tests")]
    fn queue_deterministic_conception(runtime: &mut GpuLiveBrainRuntime) {
        let parent = OrganismId(1);
        let parent_entity_id = runtime
            .world
            .organism_entity_ids()
            .into_iter()
            .find_map(|(organism_id, world_entity_id)| {
                (organism_id == parent).then_some(world_entity_id)
            })
            .expect("parent binding");
        let resident = runtime.residents.get(&parent.raw()).unwrap();
        let frame = runtime
            .world
            .perception_frame(
                parent,
                runtime.world.tick(),
                runtime.sensor_profile,
                resident.homeostasis,
            )
            .unwrap();
        let conception = frame
            .candidates()
            .iter()
            .find(|candidate| {
                let family = format!("{:?}", candidate.family);
                family.contains("Repro") || family.contains("Mate")
            })
            .copied()
            .expect("deterministic conception candidate");
        let command = conception
            .to_command(parent, Confidence::new(1.0).unwrap())
            .unwrap();
        let next_tick = Tick::new(runtime.world.tick().raw().saturating_add(1));
        runtime
            .world
            .apply_registered_neural_command(
                &command,
                parent_entity_id,
                next_tick,
                None,
                false,
            )
            .unwrap();
    }

    #[cfg(feature = "gpu-tests")]
    fn newborn_id(runtime: &GpuLiveBrainRuntime) -> OrganismId {
        let newborns = runtime
            .world
            .organism_entity_ids()
            .into_iter()
            .filter(|(organism_id, _)| organism_id.raw() > 2)
            .map(|(organism_id, _)| organism_id)
            .collect::<Vec<_>>();
        assert_eq!(newborns.len(), 1);
        newborns[0]
    }

    #[cfg(feature = "gpu-tests")]
    #[test]
    fn newborn_is_archived_linked_and_admitted_before_tick_returns() {
        let (mut runtime, archive_root) = archived_newborn_runtime("success");
        queue_deterministic_conception(&mut runtime);

        runtime.tick().unwrap();

        let newborn = newborn_id(&runtime);
        let record = runtime.world.organism_registry().get(newborn).unwrap();
        let digest = runtime
            .archive_birth_manifest(newborn)
            .expect("newborn archive manifest");
        assert_eq!(record.birth_manifest_digest(), Some(digest));
        assert_eq!(runtime.world.tick(), Tick::new(1));
        let world_entity_id = record.world_entity_id();
        assert!(runtime
            .world
            .organism_entity_ids()
            .into_iter()
            .any(|(organism_id, entity_id)| {
                organism_id == newborn && entity_id == world_entity_id
            }));
        let handle = runtime.handle_for(newborn).expect("newborn handle");
        assert_eq!(handle.organism_id(), newborn);
        assert_eq!(
            runtime
                .residents
                .get(&newborn.raw())
                .unwrap()
                .phenotype
                .phenotype_hash(),
            handle.phenotype_hash()
        );
        assert!(runtime.memories.contains_key(&newborn.raw()));
        assert_eq!(runtime.topologies.get(&newborn.raw()).unwrap().organism_id(), newborn);
        drop(runtime);
        fs::remove_dir_all(archive_root).unwrap();
    }

    #[cfg(feature = "gpu-tests")]
    #[test]
    fn failed_newborn_admission_publishes_nothing_and_retry_reuses_manifest() {
        let (mut runtime, archive_root) = archived_newborn_runtime("retry");
        queue_deterministic_conception(&mut runtime);
        runtime.backend.force_admission_failures_for_test(1);

        assert!(runtime.tick().is_err());

        let newborn = newborn_id(&runtime);
        let failed_record = runtime.world.organism_registry().get(newborn).unwrap();
        assert_eq!(failed_record.birth_manifest_digest(), None);
        assert!(runtime.archive_birth_manifest(newborn).is_none());
        assert!(!runtime.handles.contains_key(&newborn.raw()));
        assert!(!runtime.residents.contains_key(&newborn.raw()));
        assert!(!runtime.memories.contains_key(&newborn.raw()));
        assert!(!runtime.topologies.contains_key(&newborn.raw()));
        let archive_count_after_failure = runtime.lineage_archive_manifest_count().unwrap();

        runtime.reconcile_population().unwrap();

        let digest = runtime
            .archive_birth_manifest(newborn)
            .expect("retried newborn archive manifest");
        assert_eq!(
            runtime.lineage_archive_manifest_count().unwrap(),
            archive_count_after_failure
        );
        assert_eq!(
            runtime
                .world
                .organism_registry()
                .get(newborn)
                .unwrap()
                .birth_manifest_digest(),
            Some(digest)
        );
        assert!(runtime.handles.contains_key(&newborn.raw()));
        assert!(runtime.residents.contains_key(&newborn.raw()));
        assert!(runtime.memories.contains_key(&newborn.raw()));
        assert_eq!(runtime.topologies.get(&newborn.raw()).unwrap().organism_id(), newborn);
        drop(runtime);
        fs::remove_dir_all(archive_root).unwrap();
    }

    #[cfg(feature = "gpu-tests")]
    #[test]
    fn archived_death_commits_before_gpu_scrub_and_world_despawn() {
        let organism_id = OrganismId(1);
        let sensor_profile = SensorProfile::GroundedObjectSlotsV1;
        let root = std::env::temp_dir().join(format!(
            "alife-gpu-retirement-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let config = LineageLibraryConfig::profile_default(&root);
        let backend = GpuClosedLoopBackend::new_required(
            alife_gpu_backend::GpuRuntimeProfile::production_v1(),
        )
        .expect("required GPU");
        let world = HeadlessScenarioBuilder::new(96)
            .agent("archived", organism_id, Vec3f::ZERO)
            .hazard("terminal", Vec3f::new(1.0, 0.0, 0.0), 1_000.0)
            .build()
            .unwrap();
        let world_entity_id = world.entity_id("archived").unwrap();
        let foundation_asset = FoundationWeightAsset::builtin_nano512_v1(sensor_profile).unwrap();
        let foundation_manifest = foundation_asset.manifest();
        let foundation = FoundationGeneticIdentity::new(
            foundation_manifest.foundation_id().raw(),
            foundation_manifest.foundation_version().raw() as u16,
            foundation_manifest.compatibility_family_id().raw(),
            BrainCapacityClass::N512_ID,
        )
        .unwrap();
        let genome = alife_core::CreatureGenome::early_mammal_founder(0xE10_42C1, foundation)
            .unwrap();
        let phenotype = genome.express().unwrap();
        let biochemistry =
            alife_core::BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
        let mut world = world;
        world
            .register_organism_record(
                WorldOrganismRecord::new(
                    organism_id,
                    world_entity_id,
                    genome,
                    phenotype,
                    biochemistry,
                    Tick::ZERO,
                )
                .unwrap(),
            )
            .unwrap();
        let mut runtime = GpuLiveBrainRuntime::new_profiled_archived(
            backend,
            world,
            96,
            BrainScaleTier::Nano512,
            sensor_profile,
            config.clone(),
            "gpu-retirement-test",
            ArchiveLearnedCapturePolicy::Pinned,
        )
        .unwrap();
        let birth = runtime.archive_birth_manifest(organism_id).unwrap();
        assert_eq!(runtime.lineage_archive_manifest_count().unwrap(), Some(1));
        assert!(runtime.handle_for(organism_id).is_some());

        let terminal = runtime.world.entity_id("terminal").unwrap();
        runtime
            .world_mut()
            .apply_registered_command(
                &HeadlessWorldCommand::approach(organism_id, terminal).unwrap(),
                world_entity_id,
                Tick(1),
            )
            .unwrap();
        assert_eq!(runtime.world_mut().try_advance_tick().unwrap(), Tick(1));
        let final_record = runtime
            .world
            .organism_registry()
            .get(organism_id)
            .unwrap()
            .clone();
        let final_object = runtime.world.entity(world_entity_id).unwrap().clone();
        assert_eq!(final_record.world_entity_id(), final_object.id);
        assert_eq!(final_object.organism_id, Some(organism_id));
        assert_eq!(final_record.lifecycle().death_tick(), Some(Tick(1)));

        let receipt = runtime
            .retire_organism(organism_id, "test-death")
            .unwrap();
        assert_eq!(
            runtime.archive_retirement_receipt(organism_id),
            Some(&receipt)
        );
        assert_eq!(runtime.lineage_archive_manifest_count().unwrap(), Some(2));
        assert!(runtime.handle_for(organism_id).is_none());
        assert!(!runtime.residents.contains_key(&organism_id.raw()));
        assert!(!runtime.memories.contains_key(&organism_id.raw()));
        assert!(!runtime.topologies.contains_key(&organism_id.raw()));
        assert!(runtime.world.organism_registry().get(organism_id).is_none());
        assert!(runtime.world.entity(world_entity_id).is_none());
        assert_eq!(
            runtime.take_presentation_retirements(),
            vec![world_entity_id]
        );
        assert!(runtime.take_presentation_retirements().is_empty());

        let repeated = runtime
            .retire_organism(organism_id, "test-death")
            .unwrap();
        assert_eq!(repeated, receipt);
        assert_eq!(runtime.lineage_archive_manifest_count().unwrap(), Some(2));
        assert_eq!(runtime.retirement_backend_removal_count, 1);
        assert!(runtime.take_presentation_retirements().is_empty());
        assert!(runtime.world.organism_entity_ids().is_empty());
        drop(runtime);

        let library = LineageLibrary::open(config).unwrap();
        let final_manifest = library
            .load_manifest(receipt.committed_manifest_digest)
            .unwrap();
        assert_eq!(final_manifest.previous_manifest_digest, Some(birth));
        let final_statistics = library.load_life_statistics(&final_manifest).unwrap();
        assert_eq!(final_statistics.survival_ticks(), 1);
        assert_eq!(final_statistics.death_tick(), Some(Tick(1)));
        assert!(matches!(
            final_manifest.life.as_ref().unwrap().checkpoint,
            alife_core::ArchiveCheckpointDisposition::Stored(_)
        ));
        drop(library);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(feature = "gpu-tests")]
    #[test]
    fn retirement_failures_preserve_pre_receipt_state_and_retry_post_receipt_forward() {
        let organism_id = OrganismId(1);
        let sensor_profile = SensorProfile::GroundedObjectSlotsV1;
        let root = std::env::temp_dir().join(format!(
            "alife-gpu-retirement-failure-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let config = LineageLibraryConfig::profile_default(&root);
        let backend = GpuClosedLoopBackend::new_required(
            alife_gpu_backend::GpuRuntimeProfile::production_v1(),
        )
        .expect("required GPU");
        let mut world = HeadlessScenarioBuilder::new(97)
            .agent("archived", organism_id, Vec3f::ZERO)
            .hazard("terminal", Vec3f::new(1.0, 0.0, 0.0), 1_000.0)
            .build()
            .unwrap();
        let world_entity_id = world.entity_id("archived").unwrap();
        let foundation_asset = FoundationWeightAsset::builtin_nano512_v1(sensor_profile).unwrap();
        let foundation_manifest = foundation_asset.manifest();
        let foundation = FoundationGeneticIdentity::new(
            foundation_manifest.foundation_id().raw(),
            foundation_manifest.foundation_version().raw() as u16,
            foundation_manifest.compatibility_family_id().raw(),
            BrainCapacityClass::N512_ID,
        )
        .unwrap();
        let genome = alife_core::CreatureGenome::early_mammal_founder(0xE10_42C2, foundation)
            .unwrap();
        let phenotype = genome.express().unwrap();
        let biochemistry =
            alife_core::BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
        world
            .register_organism_record(
                WorldOrganismRecord::new(
                    organism_id,
                    world_entity_id,
                    genome,
                    phenotype,
                    biochemistry,
                    Tick::ZERO,
                )
                .unwrap(),
            )
            .unwrap();
        let mut runtime = GpuLiveBrainRuntime::new_profiled_archived(
            backend,
            world,
            97,
            BrainScaleTier::Nano512,
            sensor_profile,
            config.clone(),
            "gpu-retirement-failure-test",
            ArchiveLearnedCapturePolicy::Pinned,
        )
        .unwrap();
        let terminal = runtime.world.entity_id("terminal").unwrap();
        runtime
            .world_mut()
            .apply_registered_command(
                &HeadlessWorldCommand::approach(organism_id, terminal).unwrap(),
                world_entity_id,
                Tick(1),
            )
            .unwrap();
        runtime.world_mut().try_advance_tick().unwrap();

        let before_record = runtime
            .world
            .organism_registry()
            .get(organism_id)
            .unwrap()
            .clone();
        let before_object = runtime.world.entity(world_entity_id).unwrap().clone();
        let before_handles = runtime.handles.clone();
        let before_resident_keys = runtime.residents.keys().copied().collect::<Vec<_>>();
        let before_memory_keys = runtime.memories.keys().copied().collect::<Vec<_>>();
        let before_topology_keys = runtime.topologies.keys().copied().collect::<Vec<_>>();
        let before_signature = runtime.world.canonical_signature_digest().unwrap();
        let before_handle = runtime.handle_for(organism_id).unwrap();
        let archive = runtime.lineage_library.take();
        assert!(runtime
            .retire_organism(organism_id, "pre-receipt-failure")
            .is_err());
        runtime.lineage_library = archive;
        assert!(runtime.archive_retirement_receipt(organism_id).is_none());
        assert_eq!(runtime.world.canonical_signature_digest().unwrap(), before_signature);
        assert_eq!(
            runtime.world.organism_registry().get(organism_id),
            Some(&before_record)
        );
        assert_eq!(runtime.world.entity(world_entity_id), Some(&before_object));
        assert_eq!(runtime.handles, before_handles);
        assert_eq!(runtime.residents.keys().copied().collect::<Vec<_>>(), before_resident_keys);
        assert_eq!(runtime.memories.keys().copied().collect::<Vec<_>>(), before_memory_keys);
        assert_eq!(runtime.topologies.keys().copied().collect::<Vec<_>>(), before_topology_keys);
        assert_eq!(runtime.handle_for(organism_id), Some(before_handle));
        assert_eq!(runtime.retirement_backend_removal_count, 0);
        assert!(runtime.take_presentation_retirements().is_empty());

        runtime.forced_retirement_post_receipt_failure = true;
        assert!(runtime
            .retire_organism(organism_id, "post-receipt-failure")
            .is_err());
        let receipt = runtime
            .archive_retirement_receipt(organism_id)
            .unwrap()
            .clone();
        assert_eq!(runtime.lineage_archive_manifest_count().unwrap(), Some(2));
        assert_eq!(
            runtime
                .world
                .organism_registry()
                .get(organism_id)
                .unwrap()
                .archive()
                .life_manifest_digest(),
            Some(receipt.committed_manifest_digest)
        );
        assert_eq!(runtime.handle_for(organism_id), Some(before_handle));
        assert!(runtime.world.entity(world_entity_id).is_some());
        assert_eq!(runtime.retirement_backend_removal_count, 0);
        assert!(runtime.take_presentation_retirements().is_empty());

        runtime.forced_retirement_post_receipt_failure = false;
        assert_eq!(
            runtime
                .retire_organism(organism_id, "post-receipt-failure")
                .unwrap(),
            receipt
        );
        assert_eq!(runtime.lineage_archive_manifest_count().unwrap(), Some(2));
        assert_eq!(runtime.retirement_backend_removal_count, 1);
        assert!(runtime.handle_for(organism_id).is_none());
        assert!(runtime.world.organism_registry().get(organism_id).is_none());
        assert!(runtime.world.entity(world_entity_id).is_none());
        assert_eq!(
            runtime.take_presentation_retirements(),
            vec![world_entity_id]
        );
        assert!(runtime.take_presentation_retirements().is_empty());
        drop(runtime);
        std::fs::remove_dir_all(root).unwrap();
    }
}
