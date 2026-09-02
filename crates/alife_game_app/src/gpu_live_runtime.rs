//! GPU-authoritative live cognition for the explicit neural policy.

mod durability_hold;
mod exact_population_checkpoint;

use durability_hold::{
    brain_atp_world_tick_mode, motor_eligible, sleep_recovery_body_event_due, BrainAtpWorldTickMode,
};
use exact_population_checkpoint::{
    ExactCheckpointRequestDispositionV1, ExactPopulationCheckpointCoordinatorV1,
    ExactPopulationCheckpointStageV1, ManualCheckpointRequestV1,
};

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
    thread::{self, JoinHandle},
    time::Instant,
};

use alife_archive::{GeneticArchiveInput, LifeArchiveInput, LineageLibrary, LineageLibraryConfig};
use alife_core::cognitive_work::{CognitiveWorkCostPolicy, CognitiveWorkCounters};
use alife_core::predictive::GroundedSuccessorPredictor;
use alife_core::sleep::{SleepReplayEvidence, SleepWorkReceipt};
use alife_core::{
    finalized_memory_attention_evidence, select_focal_targets, ActionKind,
    ArchiveCheckpointRetention, ArchiveLearnedCapturePolicy, ArchiveRetirementReceipt,
    AttentionFrame, AttentionSelectionPolicy, BiochemistryState, Blake3Digest, BodyEventDelta,
    BoundedMotorPayload, BoundedReplayBatch, BrainCapacityClass, BrainGenome, BrainScaleTier,
    BrainTickStatus, BrainWorkCounters, BrainWorkReceipt, CandidateObservationRef,
    CanonicalDigestBuilder, CognitiveConceptActivation, CognitiveContextFrame,
    CognitiveGapActivation, CognitiveMemoryExpectancy, CognitiveWorkReceipt, Confidence,
    ConsolidationDriverEvent, ConsolidationIntent, ConsolidationState, DecisionSnapshot,
    DevelopmentState, EnvironmentalRegime, ExperiencePatch, ExperienceSequenceId,
    FinalizedMemoryAttentionEvidence, FinalizedMemoryRecall, FoundationCompatibilityFamilyId,
    FoundationGeneticIdentity, FoundationId, FoundationVersion, FoundationWeightAsset,
    HomeostaticParameters, HomeostaticSnapshot, JointMotorCondition, LanguageGroundingLedger,
    LegacyNano512CompatibilityReceipt, LineageId, MemoryBankConfig, MemoryCompactionCheckpoint,
    MemoryCompactionReceipt, MemoryRecallReceipt, MemorySidecarState, MemoryUpdateReceipt,
    MotorChannel, MotorCommandBundle, N512FounderFoundationProjection, NeuralActionSelection,
    NeuralEmission, NeuralEmissionClass, NeuralEmissionFrame, NeuralReceptorEffects,
    NeuralReceptorFrame, NeuralReceptorPhenotype, NormalizedScalar, OrganismId, PassiveLifeEvent,
    PassiveLifeStatistics, PerceptionFrame, PerceptionFrameDraft, PhenotypeCompiler,
    PhenotypeCompilerInputs, PhysicalContactKind, PostActionOutcome, PreActionSnapshot,
    PredictionTargetReceipt, PreparedMemoryRecall, ScaffoldContractError, SemanticStateVector,
    SensorProfile, SensorProfileIdentity, SensoryAbiVersion, SignedValence,
    SleepConsolidationConfig, SleepConsolidator, SleepPhase, SleepState, SleepTransition, Tick,
    TopologicalMapConfig, TopologyObservationReceipt, TopologySidecar, UtteranceSourceKind,
    Validate, Vec3f, WorldEntityId, MAX_ACTIVE_CONCEPTS, MAX_ACTIVE_GAPS,
    MAX_CONTEXT_MEMORY_EXPECTANCIES,
};
use alife_gpu_backend::{
    decode_exact_population_sleep_replay, GpuActivityRuntimeSnapshot, GpuAuthorityReceiptV1,
    GpuBrainHandle, GpuClosedLoopBackend, GpuClosedLoopMemoryBatchInput,
    GpuClosedLoopMemoryTickInput, GpuClosedLoopTick, GpuCompactCheckpointAuthorityV1,
    GpuCuratedResidencyCohort, GpuCuratedResidencyEntry, GpuCuratedResidencyOutcome,
    GpuCuratedResidencyReceipt, GpuCuratedResidencyTargetIdentity,
    GpuExactPopulationCaptureMetricsV1, GpuExactPopulationCapturePollV1,
    GpuExactPopulationCaptureTicketV1, GpuExactPopulationCaptureV1, GpuLearningReceipt,
    GpuMemoryContextUpload, GpuV11WorkReceipt, PendingEligibilityDiscardReceipt,
    PendingEligibilityIdentity, PendingEligibilityReceipt, GPU_CLOSED_LOOP_TICK_READBACK_BYTES,
    GPU_FAST_PLASTICITY_COMMIT_BYTES, GPU_MOTOR_CHANNEL_SLOT_COUNT,
};
use alife_runtime::{
    DurableGpuCheckpointMonotonicityPermit, DurableGpuCheckpointRef, GpuAuthoritativeSession,
    GpuExactCheckpointTransactionContextV1, GpuSessionAuthority, GpuSessionConsumerKind,
    GpuSessionFailStopCause, GpuSleepJournalPublicationTiming, GpuSleepTransactionJournalEntryV2,
    GpuSleepTransactionJournalV2, SleepPhaseReceipt, SleepWorkDue,
};
use alife_world::{
    grounded_peripheral_summaries,
    persistence::{
        AssetManifest, CreatureMindSaveSummary, CreatureSaveState, ExactCognitiveCheckpointState,
        GpuBrainSaveState, LearningTraceSaveSummary, PortableAssetDigest, PortableSaveFile,
        RuntimeConfig, WeightLayerSaveSummary, V11_EXACT_COGNITIVE_STATE_SCHEMA_VERSION,
    },
    CreatureAppearanceGenome, HabitatActor, HabitatAuthorityError, HabitatBreedingKind,
    HabitatBreedingReceipt, HabitatBreedingRequest, HabitatId, HabitatMode, HabitatOperation,
    HabitatOperationRequest, HabitatPermissionReceipt, HeadlessWorld, HeadlessWorldSignatureDigest,
    WorldEditorSpawnSpec, WorldObjectKind, WorldOrganismAdmissionSnapshot, WorldOrganismRecord,
};
use thiserror::Error;

use crate::factorized_arbitration::{
    arbitrate_gpu_selected_command_into_factorized_bundle, channel_command_for_action,
    VOCAL_CHANNEL_PAYLOAD_MAGIC_V1,
};
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
    LiveBrainTickSummary, LiveCognitivePresentationSnapshot, RetainedLearningCapture,
    WorldEditCommand, WorldEditorConfig, CURATED_FOUNDER_RESET_POLICY, G03_LIVE_BRAIN_LOOP_SCHEMA,
    G03_LIVE_BRAIN_LOOP_SCHEMA_VERSION,
};

#[derive(Debug, Clone, serde::Serialize)]
struct ResidentCognition {
    phenotype: alife_core::BrainPhenotype,
    compiler_inputs: PhenotypeCompilerInputs,
    legacy_nano512_compatibility_receipt: Option<LegacyNano512CompatibilityReceipt>,
    genome: BrainGenome,
    development: DevelopmentState,
    homeostasis: HomeostaticSnapshot,
    sleep_scheduler: GpuSleepScheduler,
    next_sequence: u64,
    language_grounding: LanguageGroundingLedger,
    life_statistics: PassiveLifeStatistics,
    attention_hysteresis: alife_core::HysteresisState,
    predictor: GroundedSuccessorPredictor,
    last_cognitive_context: Option<CognitiveContextFrame>,
    last_selected_motor_bundle: Option<MotorCommandBundle>,
    last_cognitive_work: CognitiveWorkReceipt,
    last_sleep_work: Option<SleepWorkReceipt>,
    last_structural_edit_receipts: Vec<alife_core::StructuralEditBatch>,
    last_sleep_report: Option<alife_core::SleepConsolidationReport>,
}

struct StagedLiveAuthority {
    world: HeadlessWorld,
    residents: BTreeMap<u64, ResidentCognition>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct RollbackCloneSample {
    world_wall_ns: u64,
    residents_wall_ns: u64,
    resident_rows: u64,
    world_object_rows: u64,
}

impl StagedLiveAuthority {
    fn begin(
        world: &mut HeadlessWorld,
        residents: &mut BTreeMap<u64, ResidentCognition>,
        measure_wall_time: bool,
    ) -> (Self, RollbackCloneSample) {
        let world_started = measure_wall_time.then(Instant::now);
        let staged_world = world.clone();
        let world_wall_ns = world_started.map_or(0, elapsed_ns);
        let residents_started = measure_wall_time.then(Instant::now);
        let staged_residents = residents.clone();
        let residents_wall_ns = residents_started.map_or(0, elapsed_ns);
        let sample = RollbackCloneSample {
            world_wall_ns,
            residents_wall_ns,
            resident_rows: u64::try_from(residents.len()).unwrap_or(u64::MAX),
            world_object_rows: u64::try_from(world.object_count()).unwrap_or(u64::MAX),
        };
        (
            Self {
                world: staged_world,
                residents: staged_residents,
            }
            .install(world, residents),
            sample,
        )
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
    measure_clone_wall_time: bool,
    staged_tick: impl FnOnce(&mut O) -> Result<T, E>,
) -> (Result<T, E>, RollbackCloneSample)
where
    O: LiveAuthorityOwner,
{
    let (staged, clone_sample) = {
        let (world, residents) = owner.world_and_residents();
        StagedLiveAuthority::begin(world, residents, measure_clone_wall_time)
    };
    let result = staged_tick(owner);
    let (world, residents) = owner.world_and_residents();
    (staged.finish(world, residents, result), clone_sample)
}

#[derive(Debug, Clone)]
struct ResidentAuthorityPlan {
    organism_id: OrganismId,
    world_entity_id: WorldEntityId,
    world_tick: Tick,
    phenotype: alife_core::BrainPhenotype,
    compiler_inputs: PhenotypeCompilerInputs,
    legacy_nano512_compatibility_receipt: Option<LegacyNano512CompatibilityReceipt>,
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
    legacy_nano512_compatibility_receipt: Option<&'a LegacyNano512CompatibilityReceipt>,
}

#[derive(Debug, Clone, PartialEq)]
struct SleepJournalNeuralAuthority {
    compact: GpuCompactCheckpointAuthorityV1,
    activity: GpuActivityRuntimeSnapshot,
}

fn capture_sleep_journal_neural_authority(
    backend: &mut GpuAuthoritativeSession,
    handle: GpuBrainHandle,
) -> Result<SleepJournalNeuralAuthority, ScaffoldContractError> {
    let compact = backend.compact_checkpoint_authority(handle)?;
    backend.validate_compact_checkpoint_authority(handle, &compact)?;
    Ok(SleepJournalNeuralAuthority {
        compact,
        activity: backend.snapshot_activity_state(handle)?,
    })
}

fn validate_sleep_journal_neural_authority(
    backend: &mut GpuAuthoritativeSession,
    handle: GpuBrainHandle,
    expected: &SleepJournalNeuralAuthority,
) -> Result<(), ScaffoldContractError> {
    backend.validate_compact_checkpoint_authority(handle, &expected.compact)?;
    let current = backend.snapshot_activity_state(handle)?;
    if current.next_sequence_cursor != expected.activity.next_sequence_cursor
        || current.next_completed_gpu_time_ns != expected.activity.next_completed_gpu_time_ns
        || current.pressure != expected.activity.pressure
        || current.throttle != expected.activity.throttle
        || current.work != expected.activity.work
    {
        return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
    }
    Ok(())
}

fn captured_sleep_covers_queued_target(queued: SleepState, captured: SleepState) -> bool {
    if queued == captured {
        return true;
    }
    let same_sleep_identity = queued.schema_version == captured.schema_version
        && queued.phase == captured.phase
        && queued.phase_started_tick == captured.phase_started_tick
        && queued.entered_sleep_tick == captured.entered_sleep_tick
        && queued.cycles_completed == captured.cycles_completed
        && queued.last_trigger == captured.last_trigger
        && queued.active_cycle_id == captured.active_cycle_id
        && queued.last_consolidated_cycle_id == captured.last_consolidated_cycle_id;
    same_sleep_identity
        && matches!(
            (queued.consolidation, captured.consolidation),
            (
                ConsolidationState::Submitted {
                    request: queued_request,
                    job_id: queued_job_id,
                },
                ConsolidationState::Completed {
                    request: captured_request,
                    staged,
                },
            ) if queued_request == captured_request && queued_job_id == staged.job_id
        )
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
    let selects_legacy_nano512 = selects_legacy_nano512_compatibility_from_record(&admission)?;
    let (phenotype, compiler_inputs, legacy_nano512_compatibility_receipt) =
        if selects_legacy_nano512 {
            let foundation = FoundationWeightAsset::builtin_nano512_v1(sensor_profile)?;
            let (phenotype, compiler_inputs, receipt) =
                PhenotypeCompiler::compile_fixed_legacy_nano512_compatibility_asset(
                    sensor_profile,
                    &foundation,
                )?
                .into_runtime_parts();
            (phenotype, compiler_inputs, Some(receipt))
        } else {
            let (phenotype, compiler_inputs) = compile_gpu_components_from_genome(
                genome.clone(),
                development.clone(),
                sensor_profile,
            )?;
            (phenotype, compiler_inputs, None)
        };
    if phenotype.brain_class_id() != brain_class.default_class_id() {
        return Err(ScaffoldContractError::PhenotypeCompile);
    }
    Ok(ResidentAuthorityPlan {
        organism_id,
        world_entity_id,
        world_tick,
        phenotype,
        compiler_inputs,
        legacy_nano512_compatibility_receipt,
        genome,
        development,
        biochemistry: admission.biochemistry,
    })
}

fn selects_legacy_nano512_compatibility_from_record(
    admission: &WorldOrganismAdmissionSnapshot,
) -> Result<bool, ScaffoldContractError> {
    let expected_foundation = FoundationGeneticIdentity::new(
        FoundationId::N512_V1.raw(),
        FoundationVersion::V1.raw() as u16,
        FoundationCompatibilityFamilyId::N512_FOUNDATION.raw(),
        BrainCapacityClass::N512_ID,
    )?;
    let genome = &admission.genome;
    let phenotype = &admission.phenotype;
    let is_nano512_record = genome.foundation.brain_class_id == BrainCapacityClass::N512_ID
        || phenotype.foundation.brain_class_id == BrainCapacityClass::N512_ID
        || phenotype.brain_genome.brain_class_id == BrainCapacityClass::N512_ID;
    if !is_nano512_record {
        return Ok(false);
    }
    if genome.foundation != expected_foundation
        || phenotype.foundation != expected_foundation
        || phenotype.source_genome_id != genome.id
        || phenotype.lineage_id != genome.lineage_id
        || phenotype.genetic_provenance != genome.provenance
        || phenotype.brain_genome.id != genome.id
        || phenotype.brain_genome.lineage_id != Some(genome.lineage_id)
    {
        return Err(ScaffoldContractError::PhenotypeCompile);
    }
    Ok(true)
}

#[cfg(feature = "gpu-tests")]
pub fn legacy_nano512_compatibility_receipt_for_record_for_test(
    record: &WorldOrganismRecord,
    world_tick: Tick,
    sensor_profile: SensorProfile,
) -> Result<LegacyNano512CompatibilityReceipt, ScaffoldContractError> {
    let plan = resident_authority_plan_from_record(
        record,
        record.organism_id(),
        record.world_entity_id(),
        world_tick,
        BrainScaleTier::Nano512,
        sensor_profile,
    )?;
    plan.legacy_nano512_compatibility_receipt
        .ok_or(ScaffoldContractError::PhenotypeCompile)
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct AuthorityAdvanceTiming {
    world_advance_ns: u64,
    resident_synchronize_ns: u64,
}

fn advance_and_synchronize_authority(
    world: &mut HeadlessWorld,
    residents: &mut BTreeMap<u64, ResidentCognition>,
    tick_after: Tick,
    body_events: &BTreeMap<u64, BodyEventDelta>,
) -> Result<AuthorityAdvanceTiming, ScaffoldContractError> {
    let world_advance_started = Instant::now();
    let advanced_tick = world.try_advance_tick_with_body_events(body_events)?;
    let world_advance_ns = elapsed_ns(world_advance_started);
    if advanced_tick != tick_after {
        return Err(ScaffoldContractError::NonMonotonicTick);
    }
    let resident_synchronize_started = Instant::now();
    synchronize_residents_from_world(world, residents, tick_after)?;
    Ok(AuthorityAdvanceTiming {
        world_advance_ns,
        resident_synchronize_ns: elapsed_ns(resident_synchronize_started),
    })
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
        || checkpoint.compiler_inputs.genome() != plan.compiler_inputs.genome()
        || checkpoint.compiler_inputs.development() != plan.compiler_inputs.development()
        || checkpoint.compiler_inputs.sensor_profile() != plan.compiler_inputs.sensor_profile()
        || checkpoint.legacy_nano512_compatibility_receipt
            != plan.legacy_nano512_compatibility_receipt.as_ref()
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
        let sleep_config = sleep_consolidation_config_for(&self.phenotype)?;
        let predictor = predictor_for_phenotype(&self.phenotype)?;
        Ok(ResidentCognition {
            phenotype: self.phenotype,
            compiler_inputs: self.compiler_inputs,
            legacy_nano512_compatibility_receipt: self.legacy_nano512_compatibility_receipt,
            genome: self.genome,
            development: self.development,
            homeostasis: self.biochemistry.homeostasis,
            sleep_scheduler: GpuSleepScheduler::new(sleep_config)?,
            next_sequence: 1,
            language_grounding: LanguageGroundingLedger::default(),
            life_statistics: PassiveLifeStatistics::new(self.organism_id, self.world_tick)?,
            attention_hysteresis: alife_core::HysteresisState::default(),
            predictor,
            last_cognitive_context: None,
            last_selected_motor_bundle: None,
            last_cognitive_work: CognitiveWorkReceipt::zero(),
            last_sleep_work: None,
            last_structural_edit_receipts: Vec::new(),
            last_sleep_report: None,
        })
    }
}

const LIVE_COGNITIVE_ENERGY_PER_WORK_UNIT: f32 = 0.000_001;
const MAX_EXACT_CHECKPOINT_PENDING_JOURNAL_ENTRIES: usize = 64;

fn append_bounded_sleep_journal_entries(
    pending: &mut Vec<GpuSleepTransactionJournalEntryV2>,
    entries: Vec<GpuSleepTransactionJournalEntryV2>,
) -> Result<(), ScaffoldContractError> {
    let next_len = pending
        .len()
        .checked_add(entries.len())
        .ok_or(ScaffoldContractError::InvalidId)?;
    if next_len > MAX_EXACT_CHECKPOINT_PENDING_JOURNAL_ENTRIES {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    for entry in &entries {
        entry.validate()?;
    }
    let mut candidate = pending.clone();
    candidate.extend(entries);
    candidate.sort_unstable_by_key(|entry| {
        (
            entry.organism_id.raw(),
            entry.transition_tick.raw(),
            entry.transition_ordinal,
        )
    });
    for pair in candidate.windows(2) {
        if pair[0].organism_id != pair[1].organism_id {
            continue;
        }
        if (pair[0].transition_tick, pair[0].transition_ordinal)
            >= (pair[1].transition_tick, pair[1].transition_ordinal)
            || pair[0].target != pair[1].source
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
    }
    *pending = candidate;
    Ok(())
}

#[derive(Debug, Clone)]
struct GpuLiveCheckpointDurability {
    store: GpuCheckpointAssetStore,
    durable_manifest: GpuDurableSaveManifest,
    published: GpuLoadedSaveManifest,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct GpuLiveSleepJournalPublicationTiming {
    current_journal_load_validation_wall_ns: u64,
    merge_wall_ns: u64,
    sort_wall_ns: u64,
    journal_build_validation_wall_ns: u64,
    durable: GpuSleepJournalPublicationTiming,
    outer_manifest_reload_validation_wall_ns: u64,
    outer_journal_reload_validation_wall_ns: u64,
}

#[derive(Debug, Clone)]
struct ExactPopulationHostSnapshotV1 {
    checkpoint_tick: Tick,
    replacement: PortableSaveFile,
    brains: Vec<ExactBrainHostSnapshotV1>,
    restored_replay_patches: Vec<ExperiencePatch>,
    sealed_patches: Vec<ExperiencePatch>,
    last_sealed_patches: Vec<ExperiencePatch>,
}

#[derive(Debug, Clone)]
struct ExactBrainHostSnapshotV1 {
    handle: GpuBrainHandle,
    phenotype: alife_core::BrainPhenotype,
    compiler_inputs: PhenotypeCompilerInputs,
    sleep: SleepState,
    memory: MemorySidecarState,
    topology: TopologySidecar,
    tracked_objects: alife_world::TrackedObjectRegistrySaveState,
    language_grounding: LanguageGroundingLedger,
    life_statistics: PassiveLifeStatistics,
    legacy_nano512_compatibility_receipt: Option<LegacyNano512CompatibilityReceipt>,
    retained_learning: Option<ExactRetainedLearningHostSnapshotV1>,
    exact_cognitive_state: ExactCognitiveHostSnapshotV1,
}

#[derive(Debug, Clone)]
struct ExactRetainedLearningHostSnapshotV1 {
    sealed_patch: ExperiencePatch,
    neural_receptors: NeuralReceptorFrame,
    attempts: u8,
    last_error_code: &'static str,
}

#[derive(Debug, Clone)]
struct ExactCognitiveHostSnapshotV1 {
    organism_id: OrganismId,
    checkpoint_tick: Tick,
    cognitive_context: CognitiveContextFrame,
    predictor: GroundedSuccessorPredictor,
    selected_motor_bundle: Option<MotorCommandBundle>,
    cognitive_work: CognitiveWorkReceipt,
    sleep_state: SleepState,
    last_sleep_work: Option<SleepWorkReceipt>,
    structural_edit_receipts: Vec<alife_core::StructuralEditBatch>,
    last_sleep_report: Option<alife_core::SleepConsolidationReport>,
}

enum ExactPopulationCheckpointRuntimeWorkV1 {
    Idle,
    Capture {
        transaction_id: u64,
        expected_base_digest: String,
        host: ExactPopulationHostSnapshotV1,
        context: GpuExactCheckpointTransactionContextV1,
        ticket: GpuExactPopulationCaptureTicketV1,
    },
    CaptureFailed {
        transaction_id: u64,
        ticket: GpuExactPopulationCaptureTicketV1,
        error: Option<GameAppShellError>,
    },
    Worker {
        transaction_id: u64,
        checkpoint_tick: Tick,
        expected_base_digest: String,
        capture_transaction_generation: u64,
        population_set_digest: [u64; 4],
        worker: ExactPopulationCheckpointWorkerOwnerV1,
    },
    CommitWorker {
        prepared: ExactPopulationCheckpointWorkerPreparedV1,
        permit: DurableGpuCheckpointMonotonicityPermit,
        worker: ExactPopulationCheckpointWorkerOwnerV1,
    },
    AwaitingJournal {
        permit: DurableCompletedCheckpointPermitV1,
        worker: ExactPopulationCheckpointWorkerOwnerV1,
    },
    JournalWorker {
        transaction_id: u64,
        worker: ExactPopulationCheckpointWorkerOwnerV1,
        journal_commit: Option<ExactPopulationCheckpointJournalCommitV1>,
    },
    Finalizing {
        transaction_id: u64,
        report: ExactPopulationCheckpointWorkerFinalV1,
        join_handle: JoinHandle<()>,
        journal_commit: Option<ExactPopulationCheckpointJournalCommitV1>,
    },
    FailedJoining {
        transaction_id: u64,
        failed: FailedExactPopulationCheckpointWorkerJoinV1,
    },
    Failed,
}

impl Default for ExactPopulationCheckpointRuntimeWorkV1 {
    fn default() -> Self {
        Self::Idle
    }
}

struct ExactPopulationCheckpointWorkerSuccessV1 {
    transaction_id: u64,
    checkpoint_tick: Tick,
    expected_base_digest: String,
    capture_transaction_generation: u64,
    population_set_digest: [u64; 4],
    durable_reference: DurableGpuCheckpointRef,
    published: GpuLoadedSaveManifest,
    exact_neural_captures: u64,
    captured_journal_authorities: BTreeMap<u64, SleepJournalNeuralAuthority>,
}

struct RestoredDurableCompletedPermitV1 {
    transaction_id: u64,
    checkpoint_tick: Tick,
    published: GpuLoadedSaveManifest,
    rollback_journal: GpuSleepTransactionJournalV2,
    captured_journal_authorities: BTreeMap<u64, SleepJournalNeuralAuthority>,
}

enum DurableCompletedCheckpointPermitV1 {
    Captured(ExactPopulationCheckpointWorkerSuccessV1),
    Restored(RestoredDurableCompletedPermitV1),
}

impl DurableCompletedCheckpointPermitV1 {
    fn transaction_id(&self) -> u64 {
        match self {
            Self::Captured(success) => success.transaction_id,
            Self::Restored(permit) => permit.transaction_id,
        }
    }

    fn checkpoint_tick(&self) -> Tick {
        match self {
            Self::Captured(success) => success.checkpoint_tick,
            Self::Restored(permit) => permit.checkpoint_tick,
        }
    }

    fn published(&self) -> &GpuLoadedSaveManifest {
        match self {
            Self::Captured(success) => &success.published,
            Self::Restored(permit) => &permit.published,
        }
    }

    fn captured_journal_authorities(&self) -> &BTreeMap<u64, SleepJournalNeuralAuthority> {
        match self {
            Self::Captured(success) => &success.captured_journal_authorities,
            Self::Restored(permit) => &permit.captured_journal_authorities,
        }
    }

    fn validate_restored_provenance(&self) -> Result<(), ScaffoldContractError> {
        let Self::Restored(permit) = self else {
            return Ok(());
        };
        permit.rollback_journal.validate()?;
        if permit.rollback_journal.exact_base_checkpoint_tick != permit.checkpoint_tick
            || permit.rollback_journal.exact_base_manifest_digest
                != permit.published.exact_save_anchor_digest()?.0
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct ExactPopulationCheckpointWorkerPreparedV1 {
    transaction_id: u64,
    checkpoint_tick: Tick,
    expected_base_digest: String,
    capture_transaction_generation: u64,
    population_set_digest: [u64; 4],
    prospective_durable_reference: DurableGpuCheckpointRef,
    exact_neural_captures: u64,
    captured_journal_authorities: BTreeMap<u64, SleepJournalNeuralAuthority>,
}

enum ExactPopulationCheckpointWorkerCommandV1 {
    CommitExact,
    Finalize {
        promotions: Vec<ExactPopulationCheckpointJournalPromotionV1>,
        manual: Option<ManualCheckpointRequestV1>,
    },
    Abort,
}

struct ExactPopulationCheckpointJournalPromotionV1 {
    entry: GpuSleepTransactionJournalEntryV2,
    authority: SleepJournalNeuralAuthority,
    phenotype: alife_core::BrainPhenotype,
}

struct ExactPopulationCheckpointJournalCommitV1 {
    authorities: Vec<(u64, SleepJournalNeuralAuthority)>,
    entry_count: u64,
    contains_completed_promotion: bool,
}

enum ExactPopulationCheckpointWorkerEventV1 {
    ManifestPrepared(ExactPopulationCheckpointWorkerPreparedV1),
    ExactPublished(ExactPopulationCheckpointWorkerSuccessV1),
    Final(ExactPopulationCheckpointWorkerFinalV1),
}

struct ExactPopulationCheckpointWorkerOwnerV1 {
    command_sender: SyncSender<ExactPopulationCheckpointWorkerCommandV1>,
    event_receiver: Receiver<ExactPopulationCheckpointWorkerEventV1>,
    join_handle: JoinHandle<()>,
}

struct SleepJournalPublicationWorkerFinalV1 {
    result:
        Result<(GpuLoadedSaveManifest, GpuLiveSleepJournalPublicationTiming), GameAppShellError>,
    expected_base_digest: String,
    expected_base_generation: Option<u64>,
    entry_count: u64,
    worker_wall_ns: u64,
}

struct SleepJournalPublicationWorkerOwnerV1 {
    receiver: Receiver<SleepJournalPublicationWorkerFinalV1>,
    join_handle: Option<JoinHandle<()>>,
}

enum SleepJournalPublicationWorkerPollV1 {
    Pending,
    Ready(SleepJournalPublicationWorkerFinalV1),
    Panicked,
}

impl SleepJournalPublicationWorkerOwnerV1 {
    fn poll(&mut self) -> SleepJournalPublicationWorkerPollV1 {
        match self.receiver.try_recv() {
            Ok(final_result) => {
                let worker_panicked = self
                    .join_handle
                    .take()
                    .is_some_and(|join_handle| join_handle.join().is_err());
                if worker_panicked {
                    SleepJournalPublicationWorkerPollV1::Panicked
                } else {
                    SleepJournalPublicationWorkerPollV1::Ready(final_result)
                }
            }
            Err(TryRecvError::Empty) => SleepJournalPublicationWorkerPollV1::Pending,
            Err(TryRecvError::Disconnected) => {
                let worker_panicked = self
                    .join_handle
                    .take()
                    .is_some_and(|join_handle| join_handle.join().is_err());
                let _ = worker_panicked;
                SleepJournalPublicationWorkerPollV1::Panicked
            }
        }
    }

    fn finish(mut self) -> Result<SleepJournalPublicationWorkerFinalV1, ()> {
        let join_handle = self.join_handle.take().ok_or(())?;
        if join_handle.join().is_err() {
            return Err(());
        }
        self.receiver.recv().map_err(|_| ())
    }
}

impl Drop for SleepJournalPublicationWorkerOwnerV1 {
    fn drop(&mut self) {
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

enum FailedExactPopulationCheckpointWorkerJoinPollV1 {
    Pending,
    Ready {
        error: GameAppShellError,
        worker_panicked: bool,
    },
}

struct FailedExactPopulationCheckpointWorkerJoinV1 {
    error: Option<GameAppShellError>,
    join_handle: Option<JoinHandle<()>>,
    abort_delivery: ExactPopulationCheckpointAbortDeliveryV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExactPopulationCheckpointAbortDeliveryV1 {
    Enqueued,
    CommandAlreadyQueued,
    WorkerDisconnected,
}

impl ExactPopulationCheckpointWorkerOwnerV1 {
    fn try_recv_event(
        &self,
    ) -> Result<Option<ExactPopulationCheckpointWorkerEventV1>, TryRecvError> {
        match self.event_receiver.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(error @ TryRecvError::Disconnected) => Err(error),
        }
    }

    fn try_send_command(
        &self,
        command: ExactPopulationCheckpointWorkerCommandV1,
    ) -> Result<(), TrySendError<ExactPopulationCheckpointWorkerCommandV1>> {
        self.command_sender.try_send(command)
    }

    fn abort_and_retain(
        self,
        error: GameAppShellError,
    ) -> FailedExactPopulationCheckpointWorkerJoinV1 {
        let abort_delivery =
            match self.try_send_command(ExactPopulationCheckpointWorkerCommandV1::Abort) {
                Ok(()) => ExactPopulationCheckpointAbortDeliveryV1::Enqueued,
                Err(TrySendError::Full(_)) => {
                    ExactPopulationCheckpointAbortDeliveryV1::CommandAlreadyQueued
                }
                Err(TrySendError::Disconnected(_)) => {
                    ExactPopulationCheckpointAbortDeliveryV1::WorkerDisconnected
                }
            };
        FailedExactPopulationCheckpointWorkerJoinV1 {
            error: Some(error),
            join_handle: Some(self.join_handle),
            abort_delivery,
        }
    }

    fn into_join_handle(self) -> JoinHandle<()> {
        self.join_handle
    }
}

impl FailedExactPopulationCheckpointWorkerJoinV1 {
    #[cfg(test)]
    fn abort_delivery(&self) -> ExactPopulationCheckpointAbortDeliveryV1 {
        self.abort_delivery
    }

    fn poll(&mut self) -> FailedExactPopulationCheckpointWorkerJoinPollV1 {
        let join_handle = self
            .join_handle
            .as_ref()
            .expect("failed checkpoint worker join is terminal after one Ready result");
        if !join_handle.is_finished() {
            return FailedExactPopulationCheckpointWorkerJoinPollV1::Pending;
        }
        let join_handle = self
            .join_handle
            .take()
            .expect("finished checkpoint worker join handle");
        let worker_panicked = join_handle.join().is_err();
        FailedExactPopulationCheckpointWorkerJoinPollV1::Ready {
            error: self
                .error
                .take()
                .expect("failed checkpoint worker error is consumed once"),
            worker_panicked,
        }
    }
}

struct ExactPopulationCheckpointWorkerFinalV1 {
    durability: GpuLiveCheckpointDurability,
    result: Result<(), GameAppShellError>,
    manual_completion: Option<ManualCheckpointCompletionV1>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ManualCheckpointCompletionV1 {
    destination: PathBuf,
    checkpoint_tick: Tick,
}

impl ExactCognitiveHostSnapshotV1 {
    fn with_captured_v11(
        &self,
        v11: &alife_gpu_backend::GpuV11Checkpoint,
    ) -> Result<ExactCognitiveCheckpointState, GameAppShellError> {
        let state = ExactCognitiveCheckpointState {
            schema_version: V11_EXACT_COGNITIVE_STATE_SCHEMA_VERSION,
            organism_id: self.organism_id,
            checkpoint_tick: self.checkpoint_tick,
            cognitive_context: self.cognitive_context.clone(),
            predictor: self.predictor.clone(),
            selected_motor_bundle: self.selected_motor_bundle.clone(),
            cognitive_work: self.cognitive_work,
            sleep_state: self.sleep_state,
            last_sleep_work: self.last_sleep_work.clone(),
            dendritic_branches: v11.dendritic_branches.clone(),
            structural_plasticity: v11.structural.clone(),
            structural_edit_receipts: self.structural_edit_receipts.clone(),
            last_sleep_report: self.last_sleep_report.clone(),
        };
        state.validate()?;
        Ok(state)
    }
}

fn assemble_checkpointed_save_from_immutable_capture(
    mut host: ExactPopulationHostSnapshotV1,
    store: &GpuCheckpointAssetStore,
    capture: &GpuExactPopulationCaptureV1,
    context: &GpuExactCheckpointTransactionContextV1,
) -> Result<(PortableSaveFile, u64), GameAppShellError> {
    if capture.checkpoint_tick() != host.checkpoint_tick
        || capture.rows().len() != host.brains.len()
    {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
    }
    let mut manifest_entries = Vec::new();
    let mut exact_neural_captures = 0_u64;
    for brain in &host.brains {
        let organism_id = brain.handle.organism_id();
        let row = capture
            .rows()
            .iter()
            .find(|row| row.identity().organism_id == organism_id)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let exact_cognitive_state = brain
            .exact_cognitive_state
            .with_captured_v11(&row.identity().v11)?;
        let capacity = BrainCapacityClass::production_for_id(brain.phenotype.brain_class_id())?;
        let replay = decode_exact_population_sleep_replay(row, host.checkpoint_tick, &capacity)?;
        let replay_patches = replay_patches_for_batch(
            &replay,
            organism_id,
            &host.restored_replay_patches,
            &host.sealed_patches,
            &host.last_sealed_patches,
        )?;
        let retained_learning =
            brain
                .retained_learning
                .as_ref()
                .map(|recovery| RetainedLearningCapture {
                    sealed_patch: &recovery.sealed_patch,
                    neural_receptors: &recovery.neural_receptors,
                    attempts: recovery.attempts,
                    last_error_code: recovery.last_error_code,
                });
        let mut write = store.capture_brain_from_exact_population_capture(
            brain.handle,
            &brain.phenotype,
            &brain.compiler_inputs,
            brain.sleep,
            host.checkpoint_tick,
            None,
            &replay_patches,
            GpuBrainSidecarCapture {
                sensor_profile: brain.memory.profile(),
                memory: &brain.memory,
                topology: &brain.topology,
                tracked_objects: brain.tracked_objects.clone(),
                language_grounding: &brain.language_grounding,
                life_statistics: &brain.life_statistics,
                legacy_nano512_compatibility_receipt: brain
                    .legacy_nano512_compatibility_receipt
                    .as_ref(),
                retained_learning,
            },
            row,
            context,
        )?;
        write.attach_exact_cognitive_state(store, &exact_cognitive_state)?;
        exact_neural_captures = exact_neural_captures.saturating_add(1);
        manifest_entries.extend(write.manifest_entries);
        let creature = host
            .replacement
            .creatures
            .iter_mut()
            .find(|creature| creature.organism_id == organism_id)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        creature.gpu_brain = Some(write.save_state);
    }
    if host.replacement.creatures.len() != host.brains.len() {
        return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
    }
    merge_gpu_checkpoint_manifest_entries(&mut host.replacement.assets, manifest_entries)?;
    host.replacement.validate_with_asset_root(store.root())?;
    Ok((host.replacement, exact_neural_captures))
}

#[cfg(windows)]
fn configure_persistence_worker_priority() {
    use windows_sys::Win32::System::Threading::{
        GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_LOWEST,
    };

    // Persistence is durable background work. Prefer the render/update thread
    // when both are runnable without skipping or delaying any transaction.
    unsafe {
        let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_LOWEST);
    }
}

#[cfg(not(windows))]
fn configure_persistence_worker_priority() {}

fn spawn_exact_population_checkpoint_worker(
    transaction_id: u64,
    expected_base_digest: String,
    host: ExactPopulationHostSnapshotV1,
    capture: GpuExactPopulationCaptureV1,
    context: GpuExactCheckpointTransactionContextV1,
    mut durability: GpuLiveCheckpointDurability,
) -> ExactPopulationCheckpointWorkerOwnerV1 {
    let (event_sender, event_receiver) = mpsc::sync_channel(1);
    let (command_sender, command_receiver) = mpsc::sync_channel(1);
    let join_handle = thread::spawn(move || {
        configure_persistence_worker_priority();
        let checkpoint_tick = host.checkpoint_tick;
        let capture_transaction_generation = capture.capture_transaction_generation();
        let population_set_digest = capture.population_set_digest();
        let prepared = (|| {
            if durability.published.digest.as_str() != expected_base_digest {
                return Err(GameAppShellError::InvalidProductionFrontend {
                    message: "exact checkpoint worker base digest changed before assembly"
                        .to_string(),
                });
            }
            let store = durability.store.clone();
            let (replacement, exact_neural_captures) =
                assemble_checkpointed_save_from_immutable_capture(
                    host, &store, &capture, &context,
                )?;
            let captured_journal_authorities = capture
                .rows()
                .iter()
                .map(|row| {
                    Ok((
                        row.identity().organism_id.raw(),
                        SleepJournalNeuralAuthority {
                            compact: row.compact_checkpoint_authority()?,
                            activity: row.activity_snapshot().clone(),
                        },
                    ))
                })
                .collect::<Result<BTreeMap<_, _>, ScaffoldContractError>>()?;
            let prospective_durable_reference =
                durability.prospective_durable_reference(&replacement)?;
            let prepared = ExactPopulationCheckpointWorkerPreparedV1 {
                transaction_id,
                checkpoint_tick,
                expected_base_digest: expected_base_digest.clone(),
                capture_transaction_generation,
                population_set_digest,
                prospective_durable_reference,
                exact_neural_captures,
                captured_journal_authorities,
            };
            Ok((replacement, prepared))
        })();
        let (replacement, prepared) = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                let _ = event_sender.send(ExactPopulationCheckpointWorkerEventV1::Final(
                    ExactPopulationCheckpointWorkerFinalV1 {
                        durability,
                        result: Err(error),
                        manual_completion: None,
                    },
                ));
                return;
            }
        };
        if event_sender
            .send(ExactPopulationCheckpointWorkerEventV1::ManifestPrepared(
                prepared.clone(),
            ))
            .is_err()
        {
            return;
        }
        match command_receiver.recv() {
            Ok(ExactPopulationCheckpointWorkerCommandV1::CommitExact) => {}
            Ok(ExactPopulationCheckpointWorkerCommandV1::Abort) | Err(_) => {
                let _ = event_sender.send(ExactPopulationCheckpointWorkerEventV1::Final(
                    ExactPopulationCheckpointWorkerFinalV1 {
                        durability,
                        result: Err(ScaffoldContractError::NeuralBackendUnavailable.into()),
                        manual_completion: None,
                    },
                ));
                return;
            }
            Ok(ExactPopulationCheckpointWorkerCommandV1::Finalize { .. }) => {
                let _ = event_sender.send(ExactPopulationCheckpointWorkerEventV1::Final(
                    ExactPopulationCheckpointWorkerFinalV1 {
                        durability,
                        result: Err(ScaffoldContractError::ConsolidationGenerationMismatch.into()),
                        manual_completion: None,
                    },
                ));
                return;
            }
        }
        let published = (|| {
            let durable_reference = durability.publish(replacement)?;
            if durable_reference != prepared.prospective_durable_reference {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
            }
            Ok(ExactPopulationCheckpointWorkerSuccessV1 {
                transaction_id: prepared.transaction_id,
                checkpoint_tick: prepared.checkpoint_tick,
                expected_base_digest: prepared.expected_base_digest.clone(),
                capture_transaction_generation: prepared.capture_transaction_generation,
                population_set_digest: prepared.population_set_digest,
                durable_reference,
                published: durability.published.clone(),
                exact_neural_captures: prepared.exact_neural_captures,
                captured_journal_authorities: prepared.captured_journal_authorities.clone(),
            })
        })();
        let success = match published {
            Ok(success) => success,
            Err(error) => {
                let _ = event_sender.send(ExactPopulationCheckpointWorkerEventV1::Final(
                    ExactPopulationCheckpointWorkerFinalV1 {
                        durability,
                        result: Err(error),
                        manual_completion: None,
                    },
                ));
                return;
            }
        };
        if event_sender
            .send(ExactPopulationCheckpointWorkerEventV1::ExactPublished(
                success,
            ))
            .is_err()
        {
            return;
        }
        run_exact_population_checkpoint_finalize_worker(durability, command_receiver, event_sender);
    });
    ExactPopulationCheckpointWorkerOwnerV1 {
        command_sender,
        event_receiver,
        join_handle,
    }
}

fn spawn_exact_population_checkpoint_recommit_worker(
    durability: GpuLiveCheckpointDurability,
) -> ExactPopulationCheckpointWorkerOwnerV1 {
    let (event_sender, event_receiver) = mpsc::sync_channel(1);
    let (command_sender, command_receiver) = mpsc::sync_channel(1);
    let join_handle = thread::spawn(move || {
        configure_persistence_worker_priority();
        run_exact_population_checkpoint_finalize_worker(durability, command_receiver, event_sender);
    });
    ExactPopulationCheckpointWorkerOwnerV1 {
        command_sender,
        event_receiver,
        join_handle,
    }
}

fn spawn_sleep_journal_publication_worker(
    mut durability: GpuLiveCheckpointDurability,
    entries: Vec<GpuSleepTransactionJournalEntryV2>,
    measure: bool,
) -> SleepJournalPublicationWorkerOwnerV1 {
    let (sender, receiver) = mpsc::sync_channel(1);
    let join_handle = thread::spawn(move || {
        configure_persistence_worker_priority();
        let started = measure.then(Instant::now);
        let entry_count = u64::try_from(entries.len()).unwrap_or(u64::MAX);
        let expected_base_digest = durability.published.digest.as_str().to_string();
        let expected_base_generation = durability.published.authority_generation();
        let result = durability
            .publish_sleep_journal_entries(entries, measure)
            .map(|timing| (durability.published, timing));
        let worker_wall_ns = started.map_or(0, elapsed_ns);
        let _ = sender.send(SleepJournalPublicationWorkerFinalV1 {
            result,
            expected_base_digest,
            expected_base_generation,
            entry_count,
            worker_wall_ns,
        });
    });
    SleepJournalPublicationWorkerOwnerV1 {
        receiver,
        join_handle: Some(join_handle),
    }
}

fn run_exact_population_checkpoint_finalize_worker(
    mut durability: GpuLiveCheckpointDurability,
    command_receiver: Receiver<ExactPopulationCheckpointWorkerCommandV1>,
    event_sender: SyncSender<ExactPopulationCheckpointWorkerEventV1>,
) {
    let (result, manual_completion) = match command_receiver.recv() {
        Ok(ExactPopulationCheckpointWorkerCommandV1::Finalize { promotions, manual }) => {
            let validated_entries = (|| {
                let mut entries = Vec::with_capacity(promotions.len());
                let mut current_sleep_by_organism = BTreeMap::new();
                for promotion in promotions {
                    let creature = durability
                        .published
                        .save
                        .creatures
                        .iter()
                        .find(|creature| creature.organism_id == promotion.entry.organism_id)
                        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                    let exact_base = creature
                        .gpu_brain
                        .as_ref()
                        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
                    promotion.entry.validate()?;
                    let current_sleep = current_sleep_by_organism
                        .entry(promotion.entry.organism_id.raw())
                        .or_insert(exact_base.sleep);
                    if promotion.entry.target == *current_sleep {
                        // The exact tick-T capture already includes this same-tick edge.
                        continue;
                    }
                    if promotion.entry.source != *current_sleep
                        || promotion.entry.transition_tick < exact_base.checkpoint_tick
                    {
                        return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
                    }
                    durability.store.validate_compact_neural_reuse_evidence(
                        &durability.published.save.assets,
                        exact_base,
                        promotion.entry.organism_id,
                        &promotion.phenotype,
                        &promotion.authority.compact,
                        &promotion.authority.activity,
                    )?;
                    *current_sleep = promotion.entry.target;
                    entries.push(promotion.entry);
                }
                Ok::<_, GameAppShellError>(entries)
            })();
            // Journal publication already validates the immutable artifacts and
            // atomically installs the generation-checked pointer.
            let result = validated_entries
                .and_then(|entries| durability.publish_sleep_journal_entries(entries, false));
            match result {
                Err(error) => (Err(error), None),
                Ok(_) => match manual {
                    Some(request) => {
                        let completion = ManualCheckpointCompletionV1 {
                            destination: request.destination.clone(),
                            checkpoint_tick: durability.published.save.world.tick,
                        };
                        let manual_result = (|| {
                            GpuDurableSaveManifest::publish_snapshot(
                                &request.destination,
                                durability.store.root(),
                                &durability.published.save,
                            )?;
                            let manual_manifest = GpuDurableSaveManifest::open(
                                &request.destination,
                                durability.store.root(),
                            )?;
                            let manual_published = manual_manifest.load()?;
                            if manual_published.save != durability.published.save {
                                return Err(GameAppShellError::InvalidProductionFrontend {
                                        message: "manual checkpoint reload differs from the exact worker generation"
                                            .to_string(),
                                    });
                            }
                            Ok(())
                        })();
                        match manual_result {
                            Ok(()) => (Ok(()), Some(completion)),
                            Err(error) => (Err(error), None),
                        }
                    }
                    None => (Ok(()), None),
                },
            }
        }
        Ok(ExactPopulationCheckpointWorkerCommandV1::Abort) | Err(_) => (
            Err(ScaffoldContractError::NeuralBackendUnavailable.into()),
            None,
        ),
        Ok(ExactPopulationCheckpointWorkerCommandV1::CommitExact) => (
            Err(ScaffoldContractError::ConsolidationGenerationMismatch.into()),
            None,
        ),
    };
    let _ = event_sender.send(ExactPopulationCheckpointWorkerEventV1::Final(
        ExactPopulationCheckpointWorkerFinalV1 {
            durability,
            result,
            manual_completion,
        },
    ));
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
    fn publish_sleep_journal_entries(
        &mut self,
        entries: Vec<GpuSleepTransactionJournalEntryV2>,
        measure: bool,
    ) -> Result<GpuLiveSleepJournalPublicationTiming, GameAppShellError> {
        let mut timing = GpuLiveSleepJournalPublicationTiming::default();
        if entries.is_empty() {
            return Ok(timing);
        }
        let started = measure.then(Instant::now);
        let current = self
            .durable_manifest
            .load_sleep_transaction_journal(&self.published)?;
        record_optional_elapsed_ns(&mut timing.current_journal_load_validation_wall_ns, started);
        let started = measure.then(Instant::now);
        let mut combined = current.entries;
        combined.extend(entries);
        record_optional_elapsed_ns(&mut timing.merge_wall_ns, started);
        let started = measure.then(Instant::now);
        combined.sort_unstable_by_key(|entry| {
            (
                entry.organism_id.raw(),
                entry.transition_tick.raw(),
                entry.transition_ordinal,
            )
        });
        record_optional_elapsed_ns(&mut timing.sort_wall_ns, started);
        let started = measure.then(Instant::now);
        let journal = GpuSleepTransactionJournalV2::try_new(
            self.published.exact_save_anchor_digest()?.0,
            self.published.save.world.tick,
            combined,
        )?;
        record_optional_elapsed_ns(&mut timing.journal_build_validation_wall_ns, started);
        let receipt = self
            .durable_manifest
            .publish_sleep_transaction_journal_profiled(&self.published, &journal, measure)?;
        timing.durable = receipt.timing;
        self.published = receipt.published;
        Ok(timing)
    }

    fn durable_reference_for(
        save: &PortableSaveFile,
        manifest_digest: &str,
    ) -> Result<DurableGpuCheckpointRef, GameAppShellError> {
        let mut digest = CanonicalDigestBuilder::new(b"alife.runtime.durable-checkpoint-ref.v1");
        digest.write_u64(save.world.tick.raw());
        digest.write_utf8(manifest_digest);
        digest.write_sequence_len(save.creatures.len());
        for creature in &save.creatures {
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
            save.world.tick,
            manifest_digest.to_string(),
            digest.finish256(),
        )?)
    }

    fn durable_reference(&self) -> Result<DurableGpuCheckpointRef, GameAppShellError> {
        Self::durable_reference_for(&self.published.save, self.published.digest.as_str())
    }

    fn prospective_durable_reference(
        &self,
        replacement: &PortableSaveFile,
    ) -> Result<DurableGpuCheckpointRef, GameAppShellError> {
        let digest = self.durable_manifest.replacement_digest(replacement)?;
        Self::durable_reference_for(replacement, digest.as_str())
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
    sleep_config: Option<SleepConsolidationConfig>,
    context: Option<AuthoritativeSleepContext<'a>>,
    replay_evidence_before_commit: Option<SleepReplayEvidence>,
    last_sleep_work: Option<&'a mut Option<SleepWorkReceipt>>,
}

struct AuthoritativeSleepContext<'a> {
    memory: &'a mut MemorySidecarState,
    predictor: &'a mut GroundedSuccessorPredictor,
    topology: &'a mut TopologySidecar,
    restored_replay_patches: &'a [ExperiencePatch],
    sealed_patches: &'a [ExperiencePatch],
    last_sealed_patches: &'a [ExperiencePatch],
}

#[derive(Debug, Default)]
struct SleepPreparationTiming {
    phase_data_wall_ns: u64,
    replay_progress_wall_ns: u64,
    consolidation_wall_ns: u64,
}

fn build_authoritative_sleep_evidence(
    backend: &mut GpuClosedLoopBackend,
    handle: GpuBrainHandle,
    organism_id: OrganismId,
    restored_replay_patches: &[ExperiencePatch],
    sealed_patches: &[ExperiencePatch],
    last_sealed_patches: &[ExperiencePatch],
) -> Result<SleepReplayEvidence, ScaffoldContractError> {
    let batch = backend.build_sleep_replay_batch(handle)?;
    if batch.events.is_empty() {
        return Err(ScaffoldContractError::MissingPhaseData);
    }
    let prediction_targets = batch
        .events
        .iter()
        .map(|event| {
            restored_replay_patches
                .iter()
                .chain(last_sealed_patches.iter())
                .chain(sealed_patches.iter())
                .find_map(|patch| {
                    patch.prediction_target().and_then(|target| {
                        (target.organism_id == organism_id
                            && target.experience_sequence == event.sequence_id)
                            .then(|| target.clone())
                    })
                })
                .ok_or(ScaffoldContractError::MissingPhaseData)
        })
        .collect::<Result<Vec<_>, _>>()?;
    SleepReplayEvidence::new(batch, prediction_targets)
}

fn replay_patches_for_checkpoint(
    backend: &mut GpuClosedLoopBackend,
    handle: GpuBrainHandle,
    organism_id: OrganismId,
    restored_replay_patches: &[ExperiencePatch],
    sealed_patches: &[ExperiencePatch],
    last_sealed_patches: &[ExperiencePatch],
) -> Result<Vec<ExperiencePatch>, ScaffoldContractError> {
    let batch = backend.build_sleep_replay_batch(handle)?;
    replay_patches_for_batch(
        &batch,
        organism_id,
        restored_replay_patches,
        sealed_patches,
        last_sealed_patches,
    )
}

fn replay_patches_for_batch(
    batch: &BoundedReplayBatch,
    organism_id: OrganismId,
    restored_replay_patches: &[ExperiencePatch],
    sealed_patches: &[ExperiencePatch],
    last_sealed_patches: &[ExperiencePatch],
) -> Result<Vec<ExperiencePatch>, ScaffoldContractError> {
    if batch.events.is_empty() {
        return Ok(Vec::new());
    }
    let patches = batch
        .events
        .iter()
        .map(|event| {
            restored_replay_patches
                .iter()
                .chain(last_sealed_patches.iter())
                .chain(sealed_patches.iter())
                .find(|patch| {
                    patch.header().organism_id == organism_id
                        && patch.header().sequence_id == event.sequence_id
                })
                .cloned()
                .ok_or(ScaffoldContractError::MissingPhaseData)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let prediction_targets = patches
        .iter()
        .map(|patch| {
            patch
                .prediction_target()
                .cloned()
                .ok_or(ScaffoldContractError::MissingPhaseData)
        })
        .collect::<Result<Vec<_>, _>>()?;
    SleepReplayEvidence::new(batch.clone(), prediction_targets)?;
    Ok(patches)
}

fn run_authoritative_sleep_transaction_with_evidence(
    homeostasis: &HomeostaticSnapshot,
    tick: Tick,
    sleep_config: SleepConsolidationConfig,
    context: &mut AuthoritativeSleepContext<'_>,
    evidence: &SleepReplayEvidence,
) -> Result<SleepWorkReceipt, ScaffoldContractError> {
    let consolidator = SleepConsolidator::new(sleep_config)?;
    context.memory.run_bounded_sleep_transaction(
        &consolidator,
        homeostasis,
        tick,
        evidence,
        context.predictor,
        context.topology,
    )
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
                let request = self
                    .backend
                    .prepare_sleep_consolidation(self.handle, intent, &replay)?;
                ConsolidationDriverEvent::Prepared { request }
            }
            (ConsolidationState::Prepared { request }, None) => {
                let replay = self.backend.build_sleep_replay_batch(self.handle)?;
                if replay.canonical_digest != request.replay_digest {
                    return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
                }
                let job_id = self
                    .backend
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

    fn run_bounded_sleep_transaction(
        &mut self,
        organism_id: OrganismId,
        _state: SleepState,
        homeostasis: &HomeostaticSnapshot,
        tick: Tick,
        due_work: SleepWorkDue,
    ) -> Result<Option<SleepWorkReceipt>, ScaffoldContractError> {
        if organism_id != self.handle.organism_id() {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        let replay_evidence_before_commit = self.replay_evidence_before_commit.take();
        let context = self
            .context
            .as_mut()
            .ok_or(ScaffoldContractError::MissingPhaseData)?;
        let evidence = match replay_evidence_before_commit {
            Some(evidence) => Ok(evidence),
            None => build_authoritative_sleep_evidence(
                self.backend,
                self.handle,
                organism_id,
                context.restored_replay_patches,
                context.sealed_patches,
                context.last_sealed_patches,
            ),
        }?;
        let receipt = run_authoritative_sleep_transaction_with_evidence(
            homeostasis,
            tick,
            self.sleep_config
                .ok_or(ScaffoldContractError::MissingPhaseData)?,
            context,
            &evidence,
        )?;
        if due_work.contains(SleepWorkDue::STRUCTURAL_GROWTH_PRUNING) {
            self.backend
                .apply_v11_sleep_structural_phase(self.handle, &evidence)?;
        }
        if let Some(last_sleep_work) = self.last_sleep_work.as_mut() {
            **last_sleep_work = Some(receipt.clone());
        }
        Ok(Some(receipt))
    }

    fn has_bounded_sleep_phase_data(
        &mut self,
        organism_id: OrganismId,
        _state: SleepState,
    ) -> Result<bool, ScaffoldContractError> {
        if organism_id != self.handle.organism_id() {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        self.backend.has_bounded_sleep_phase_data(self.handle)
    }
}

type SleepProgressResult = Result<Option<ConsolidationDriverEvent>, ScaffoldContractError>;

struct RoutedGpuSleepDriver<'a, F> {
    authoritative: AuthoritativeGpuSleepDriver<'a>,
    progress: &'a mut F,
    timing: &'a mut SleepPreparationTiming,
    measure: bool,
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
        let started = self.measure.then(Instant::now);
        if matches!(state.consolidation, ConsolidationState::Completed { .. })
            && self
                .authoritative
                .has_bounded_sleep_phase_data(organism_id, state)?
        {
            let context = self
                .authoritative
                .context
                .as_mut()
                .ok_or(ScaffoldContractError::MissingPhaseData)?;
            self.authoritative.replay_evidence_before_commit =
                Some(build_authoritative_sleep_evidence(
                    self.authoritative.backend,
                    self.authoritative.handle,
                    organism_id,
                    context.restored_replay_patches,
                    context.sealed_patches,
                    context.last_sealed_patches,
                )?);
        }
        let result = (self.progress)(
            self.authoritative.backend,
            self.authoritative.handle,
            organism_id,
            state,
            intent,
        );
        self.timing.replay_progress_wall_ns = self
            .timing
            .replay_progress_wall_ns
            .saturating_add(started.map_or(0, elapsed_ns));
        result
    }

    fn run_bounded_sleep_transaction(
        &mut self,
        organism_id: OrganismId,
        _state: SleepState,
        homeostasis: &HomeostaticSnapshot,
        tick: Tick,
        due_work: SleepWorkDue,
    ) -> Result<Option<SleepWorkReceipt>, ScaffoldContractError> {
        let started = self.measure.then(Instant::now);
        let result = self.authoritative.run_bounded_sleep_transaction(
            organism_id,
            _state,
            homeostasis,
            tick,
            due_work,
        );
        self.timing.consolidation_wall_ns = self
            .timing
            .consolidation_wall_ns
            .saturating_add(started.map_or(0, elapsed_ns));
        result
    }

    fn has_bounded_sleep_phase_data(
        &mut self,
        organism_id: OrganismId,
        state: SleepState,
    ) -> Result<bool, ScaffoldContractError> {
        let started = self.measure.then(Instant::now);
        let result = self
            .authoritative
            .has_bounded_sleep_phase_data(organism_id, state);
        self.timing.phase_data_wall_ns = self
            .timing
            .phase_data_wall_ns
            .saturating_add(started.map_or(0, elapsed_ns));
        result
    }
}

struct PreparedLiveSelection {
    handle: GpuBrainHandle,
    world_entity_id: WorldEntityId,
    pending_eligibility: PendingEligibilityReceipt,
    frame: PerceptionFrame,
    memory_recall: FinalizedMemoryRecall,
    work: BrainWorkReceipt,
    v11_work: GpuV11WorkReceipt,
    cognitive_context_digest: [u64; 4],
    sequence_id: ExperienceSequenceId,
    outcome_tick: Tick,
    pre_action: PreActionSnapshot,
    decision: DecisionSnapshot,
    motor_bundle: MotorCommandBundle,
    speech_payload: Option<alife_core::SpeechMotorPayload>,
    speech_prompted: bool,
    neural_receptors: NeuralReceptorFrame,
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
    v11_work: GpuV11WorkReceipt,
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
    neural_receptors: NeuralReceptorFrame,
}

struct PreparedGpuBrainFrame {
    handle: GpuBrainHandle,
    world_entity_id: WorldEntityId,
    frame: PerceptionFrame,
    memory_recall: FinalizedMemoryRecall,
    memory_upload: GpuMemoryContextUpload,
    neural_receptors: NeuralReceptorFrame,
    receptor_effects: NeuralReceptorEffects,
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
    neural_receptors: NeuralReceptorFrame,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub struct GpuLivePerformanceMetrics {
    pub tick_calls: u64,
    pub tick_wall_ns: u64,
    pub tick_preamble_wall_ns: u64,
    pub rollback_clone_calls: u64,
    pub rollback_world_clone_wall_ns: u64,
    pub rollback_residents_clone_wall_ns: u64,
    pub rollback_resident_rows: u64,
    pub rollback_world_object_rows: u64,
    pub rollback_clone_progress_calls: u64,
    pub rollback_clone_zero_progress_calls: u64,
    pub exact_checkpoint_poll_calls: u64,
    pub exact_checkpoint_poll_wall_ns: u64,
    pub exact_checkpoint_transactions_started: u64,
    pub exact_checkpoint_transactions_completed: u64,
    pub exact_checkpoint_transaction_wall_ns: u64,
    pub perception_sleep_preparation_wall_ns: u64,
    pub preparation_sleep_eligibility_replay_wall_ns: u64,
    pub preparation_sleep_phase_data_wall_ns: u64,
    pub preparation_sleep_replay_progress_wall_ns: u64,
    pub preparation_sleep_consolidation_wall_ns: u64,
    pub preparation_grounded_perception_wall_ns: u64,
    pub preparation_episodic_retrieval_wall_ns: u64,
    pub preparation_attention_context_wall_ns: u64,
    pub preparation_topology_concept_wall_ns: u64,
    pub preparation_gpu_upload_wall_ns: u64,
    pub preparation_checkpoint_publication_wall_ns: u64,
    pub sleep_promotion_wall_ns: u64,
    pub inference_batches: u64,
    pub inference_rows: u64,
    pub inference_transaction_wall_ns: u64,
    pub selection_readback_calls: u64,
    pub selection_readback_bytes: u64,
    pub learning_batches: u64,
    pub learning_rows: u64,
    pub learning_transaction_wall_ns: u64,
    pub learning_readback_calls: u64,
    pub learning_readback_bytes: u64,
    pub selection_prepare_wall_ns: u64,
    pub seal_world_body_biochemistry_wall_ns: u64,
    pub sealed_commit_total_wall_ns: u64,
    pub sidecar_memory_wall_ns: u64,
    pub sidecar_topology_wall_ns: u64,
    pub cognitive_authority_seal_wall_ns: u64,
    pub ordinary_snapshot_calls: u64,
    pub ordinary_snapshot_bytes: u64,
    pub ordinary_snapshot_poll_wait_ns: u64,
    pub ordinary_snapshot_map_receive_wait_ns: u64,
    pub ordinary_snapshot_wall_ns: u64,
    pub state_reference_hash_calls: u64,
    pub resident_json_bytes: u64,
    pub topology_json_bytes: u64,
    pub state_reference_hash_wall_ns: u64,
    pub world_authority_advance_wall_ns: u64,
    pub resident_synchronize_wall_ns: u64,
    pub passive_observation_wall_ns: u64,
    pub population_reconcile_wall_ns: u64,
    pub sleep_persistence_wall_ns: u64,
    pub sleep_persistence_calls: u64,
    pub sleep_journal_current_load_validation_wall_ns: u64,
    pub sleep_journal_merge_wall_ns: u64,
    pub sleep_journal_sort_wall_ns: u64,
    pub sleep_journal_build_validation_wall_ns: u64,
    pub sleep_journal_input_validation_wall_ns: u64,
    pub sleep_journal_cas_lock_wait_wall_ns: u64,
    pub sleep_journal_cas_base_reload_wall_ns: u64,
    pub sleep_journal_save_encode_wall_ns: u64,
    pub sleep_journal_save_artifact_write_wall_ns: u64,
    pub sleep_journal_encode_wall_ns: u64,
    pub sleep_journal_artifact_write_wall_ns: u64,
    pub sleep_journal_pointer_build_validation_wall_ns: u64,
    pub sleep_journal_prepared_reload_validation_wall_ns: u64,
    pub sleep_journal_manifest_encode_wall_ns: u64,
    pub sleep_journal_manifest_write_wall_ns: u64,
    pub sleep_journal_manifest_reload_validation_wall_ns: u64,
    pub sleep_journal_final_reload_validation_wall_ns: u64,
    pub sleep_journal_outer_manifest_reload_validation_wall_ns: u64,
    pub sleep_journal_outer_reload_validation_wall_ns: u64,
    pub sleep_journal_worker_starts: u64,
    pub sleep_journal_worker_completions: u64,
    pub sleep_journal_worker_failures: u64,
    pub sleep_journal_worker_poll_calls: u64,
    pub sleep_journal_worker_poll_wall_ns: u64,
    pub sleep_journal_worker_wall_ns: u64,
    pub sleep_journal_pending_entries_peak: u64,
    pub sleep_journal_update_thread_enqueue_wall_ns: u64,
    pub sleep_checkpoint_capture_calls: u64,
    pub sleep_exact_neural_capture_organisms: u64,
    pub sleep_compact_journal_organisms: u64,
    pub sleep_checkpoint_capture_wall_ns: u64,
    pub sleep_checkpoint_readback_calls: u64,
    pub sleep_checkpoint_readback_bytes: u64,
    pub sleep_checkpoint_readback_poll_wait_ns: u64,
    pub sleep_checkpoint_readback_map_receive_wait_ns: u64,
    pub sleep_checkpoint_publish_calls: u64,
    pub sleep_checkpoint_publish_wall_ns: u64,
    pub sleep_promotion_calls: u64,
    pub sleep_promotion_publish_calls: u64,
    pub sleep_promotion_publish_wall_ns: u64,
    pub checkpoint_capture_calls: u64,
    pub checkpoint_capture_wall_ns: u64,
    pub checkpoint_snapshot_calls: u64,
    pub checkpoint_snapshot_bytes: u64,
    pub checkpoint_snapshot_poll_wait_ns: u64,
    pub checkpoint_snapshot_map_receive_wait_ns: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub(crate) struct ExactCheckpointPerformanceState {
    pub transaction_id: Option<u64>,
    pub checkpoint_tick: Option<u64>,
    pub stage: &'static str,
    pub worker_status: &'static str,
}

impl Default for ExactCheckpointPerformanceState {
    fn default() -> Self {
        Self {
            transaction_id: None,
            checkpoint_tick: None,
            stage: "idle",
            worker_status: "idle",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GpuLiveNoProgressReason {
    CheckpointPublicationPending,
    CheckpointFailed,
}

#[derive(Debug)]
pub enum GpuLiveTickOutcome {
    Progressed(Vec<LiveBrainTickSummary>),
    NoProgress(GpuLiveNoProgressReason),
}

const fn no_progress_reason_for_checkpoint_stage(
    stage: ExactPopulationCheckpointStageV1,
) -> Option<GpuLiveNoProgressReason> {
    match stage {
        ExactPopulationCheckpointStageV1::Failed => Some(GpuLiveNoProgressReason::CheckpointFailed),
        _ => None,
    }
}

impl GpuLivePerformanceMetrics {
    pub(crate) fn delta_from(self, before: Self) -> Self {
        macro_rules! delta {
            ($field:ident) => {
                self.$field.saturating_sub(before.$field)
            };
        }
        Self {
            tick_calls: delta!(tick_calls),
            tick_wall_ns: delta!(tick_wall_ns),
            tick_preamble_wall_ns: delta!(tick_preamble_wall_ns),
            rollback_clone_calls: delta!(rollback_clone_calls),
            rollback_world_clone_wall_ns: delta!(rollback_world_clone_wall_ns),
            rollback_residents_clone_wall_ns: delta!(rollback_residents_clone_wall_ns),
            rollback_resident_rows: delta!(rollback_resident_rows),
            rollback_world_object_rows: delta!(rollback_world_object_rows),
            rollback_clone_progress_calls: delta!(rollback_clone_progress_calls),
            rollback_clone_zero_progress_calls: delta!(rollback_clone_zero_progress_calls),
            exact_checkpoint_poll_calls: delta!(exact_checkpoint_poll_calls),
            exact_checkpoint_poll_wall_ns: delta!(exact_checkpoint_poll_wall_ns),
            exact_checkpoint_transactions_started: delta!(exact_checkpoint_transactions_started),
            exact_checkpoint_transactions_completed: delta!(
                exact_checkpoint_transactions_completed
            ),
            exact_checkpoint_transaction_wall_ns: delta!(exact_checkpoint_transaction_wall_ns),
            perception_sleep_preparation_wall_ns: delta!(perception_sleep_preparation_wall_ns),
            preparation_sleep_eligibility_replay_wall_ns: delta!(
                preparation_sleep_eligibility_replay_wall_ns
            ),
            preparation_sleep_phase_data_wall_ns: delta!(preparation_sleep_phase_data_wall_ns),
            preparation_sleep_replay_progress_wall_ns: delta!(
                preparation_sleep_replay_progress_wall_ns
            ),
            preparation_sleep_consolidation_wall_ns: delta!(
                preparation_sleep_consolidation_wall_ns
            ),
            preparation_grounded_perception_wall_ns: delta!(
                preparation_grounded_perception_wall_ns
            ),
            preparation_episodic_retrieval_wall_ns: delta!(preparation_episodic_retrieval_wall_ns),
            preparation_attention_context_wall_ns: delta!(preparation_attention_context_wall_ns),
            preparation_topology_concept_wall_ns: delta!(preparation_topology_concept_wall_ns),
            preparation_gpu_upload_wall_ns: delta!(preparation_gpu_upload_wall_ns),
            preparation_checkpoint_publication_wall_ns: delta!(
                preparation_checkpoint_publication_wall_ns
            ),
            sleep_promotion_wall_ns: delta!(sleep_promotion_wall_ns),
            inference_batches: delta!(inference_batches),
            inference_rows: delta!(inference_rows),
            inference_transaction_wall_ns: delta!(inference_transaction_wall_ns),
            selection_readback_calls: delta!(selection_readback_calls),
            selection_readback_bytes: delta!(selection_readback_bytes),
            learning_batches: delta!(learning_batches),
            learning_rows: delta!(learning_rows),
            learning_transaction_wall_ns: delta!(learning_transaction_wall_ns),
            learning_readback_calls: delta!(learning_readback_calls),
            learning_readback_bytes: delta!(learning_readback_bytes),
            selection_prepare_wall_ns: delta!(selection_prepare_wall_ns),
            seal_world_body_biochemistry_wall_ns: delta!(seal_world_body_biochemistry_wall_ns),
            sealed_commit_total_wall_ns: delta!(sealed_commit_total_wall_ns),
            sidecar_memory_wall_ns: delta!(sidecar_memory_wall_ns),
            sidecar_topology_wall_ns: delta!(sidecar_topology_wall_ns),
            cognitive_authority_seal_wall_ns: delta!(cognitive_authority_seal_wall_ns),
            ordinary_snapshot_calls: delta!(ordinary_snapshot_calls),
            ordinary_snapshot_bytes: delta!(ordinary_snapshot_bytes),
            ordinary_snapshot_poll_wait_ns: delta!(ordinary_snapshot_poll_wait_ns),
            ordinary_snapshot_map_receive_wait_ns: delta!(ordinary_snapshot_map_receive_wait_ns),
            ordinary_snapshot_wall_ns: delta!(ordinary_snapshot_wall_ns),
            state_reference_hash_calls: delta!(state_reference_hash_calls),
            resident_json_bytes: delta!(resident_json_bytes),
            topology_json_bytes: delta!(topology_json_bytes),
            state_reference_hash_wall_ns: delta!(state_reference_hash_wall_ns),
            world_authority_advance_wall_ns: delta!(world_authority_advance_wall_ns),
            resident_synchronize_wall_ns: delta!(resident_synchronize_wall_ns),
            passive_observation_wall_ns: delta!(passive_observation_wall_ns),
            population_reconcile_wall_ns: delta!(population_reconcile_wall_ns),
            sleep_persistence_wall_ns: delta!(sleep_persistence_wall_ns),
            sleep_persistence_calls: delta!(sleep_persistence_calls),
            sleep_journal_current_load_validation_wall_ns: delta!(
                sleep_journal_current_load_validation_wall_ns
            ),
            sleep_journal_merge_wall_ns: delta!(sleep_journal_merge_wall_ns),
            sleep_journal_sort_wall_ns: delta!(sleep_journal_sort_wall_ns),
            sleep_journal_build_validation_wall_ns: delta!(sleep_journal_build_validation_wall_ns),
            sleep_journal_input_validation_wall_ns: delta!(sleep_journal_input_validation_wall_ns),
            sleep_journal_cas_lock_wait_wall_ns: delta!(sleep_journal_cas_lock_wait_wall_ns),
            sleep_journal_cas_base_reload_wall_ns: delta!(sleep_journal_cas_base_reload_wall_ns),
            sleep_journal_save_encode_wall_ns: delta!(sleep_journal_save_encode_wall_ns),
            sleep_journal_save_artifact_write_wall_ns: delta!(
                sleep_journal_save_artifact_write_wall_ns
            ),
            sleep_journal_encode_wall_ns: delta!(sleep_journal_encode_wall_ns),
            sleep_journal_artifact_write_wall_ns: delta!(sleep_journal_artifact_write_wall_ns),
            sleep_journal_pointer_build_validation_wall_ns: delta!(
                sleep_journal_pointer_build_validation_wall_ns
            ),
            sleep_journal_prepared_reload_validation_wall_ns: delta!(
                sleep_journal_prepared_reload_validation_wall_ns
            ),
            sleep_journal_manifest_encode_wall_ns: delta!(sleep_journal_manifest_encode_wall_ns),
            sleep_journal_manifest_write_wall_ns: delta!(sleep_journal_manifest_write_wall_ns),
            sleep_journal_manifest_reload_validation_wall_ns: delta!(
                sleep_journal_manifest_reload_validation_wall_ns
            ),
            sleep_journal_final_reload_validation_wall_ns: delta!(
                sleep_journal_final_reload_validation_wall_ns
            ),
            sleep_journal_outer_manifest_reload_validation_wall_ns: delta!(
                sleep_journal_outer_manifest_reload_validation_wall_ns
            ),
            sleep_journal_outer_reload_validation_wall_ns: delta!(
                sleep_journal_outer_reload_validation_wall_ns
            ),
            sleep_journal_worker_starts: delta!(sleep_journal_worker_starts),
            sleep_journal_worker_completions: delta!(sleep_journal_worker_completions),
            sleep_journal_worker_failures: delta!(sleep_journal_worker_failures),
            sleep_journal_worker_poll_calls: delta!(sleep_journal_worker_poll_calls),
            sleep_journal_worker_poll_wall_ns: delta!(sleep_journal_worker_poll_wall_ns),
            sleep_journal_worker_wall_ns: delta!(sleep_journal_worker_wall_ns),
            sleep_journal_pending_entries_peak: self.sleep_journal_pending_entries_peak,
            sleep_journal_update_thread_enqueue_wall_ns: delta!(
                sleep_journal_update_thread_enqueue_wall_ns
            ),
            sleep_checkpoint_capture_calls: delta!(sleep_checkpoint_capture_calls),
            sleep_exact_neural_capture_organisms: delta!(sleep_exact_neural_capture_organisms),
            sleep_compact_journal_organisms: delta!(sleep_compact_journal_organisms),
            sleep_checkpoint_capture_wall_ns: delta!(sleep_checkpoint_capture_wall_ns),
            sleep_checkpoint_readback_calls: delta!(sleep_checkpoint_readback_calls),
            sleep_checkpoint_readback_bytes: delta!(sleep_checkpoint_readback_bytes),
            sleep_checkpoint_readback_poll_wait_ns: delta!(sleep_checkpoint_readback_poll_wait_ns),
            sleep_checkpoint_readback_map_receive_wait_ns: delta!(
                sleep_checkpoint_readback_map_receive_wait_ns
            ),
            sleep_checkpoint_publish_calls: delta!(sleep_checkpoint_publish_calls),
            sleep_checkpoint_publish_wall_ns: delta!(sleep_checkpoint_publish_wall_ns),
            sleep_promotion_calls: delta!(sleep_promotion_calls),
            sleep_promotion_publish_calls: delta!(sleep_promotion_publish_calls),
            sleep_promotion_publish_wall_ns: delta!(sleep_promotion_publish_wall_ns),
            checkpoint_capture_calls: delta!(checkpoint_capture_calls),
            checkpoint_capture_wall_ns: delta!(checkpoint_capture_wall_ns),
            checkpoint_snapshot_calls: delta!(checkpoint_snapshot_calls),
            checkpoint_snapshot_bytes: delta!(checkpoint_snapshot_bytes),
            checkpoint_snapshot_poll_wait_ns: delta!(checkpoint_snapshot_poll_wait_ns),
            checkpoint_snapshot_map_receive_wait_ns: delta!(
                checkpoint_snapshot_map_receive_wait_ns
            ),
        }
    }
}

fn elapsed_ns(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

fn record_optional_elapsed_ns(field: &mut u64, started: Option<Instant>) {
    if let Some(started) = started {
        *field = field.saturating_add(elapsed_ns(started));
    }
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
    sleep_journal_neural_authorities: BTreeMap<u64, SleepJournalNeuralAuthority>,
    pending_exact_sleep_journal_entries: Vec<GpuSleepTransactionJournalEntryV2>,
    sleep_journal_publication_worker: Option<SleepJournalPublicationWorkerOwnerV1>,
    pending_sleep_journal_entries: Vec<GpuSleepTransactionJournalEntryV2>,
    exact_checkpoint_waiting_for_sleep_journal: bool,
    manual_checkpoint_waiting_for_sleep_journal: Option<PathBuf>,
    post_promotion_fail_stop_armed: bool,
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
    restored_replay_patches: Vec<ExperiencePatch>,
    last_sealed_patches: Vec<ExperiencePatch>,
    observe_sidecars: bool,
    retain_sealed_patch_history: bool,
    last_learning_receipts: Vec<GpuLearningReceipt>,
    last_gpu_authority_receipts: Vec<GpuAuthorityReceiptV1>,
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
    #[cfg(feature = "gpu-tests")]
    last_sleep_memory_compaction_preparation_count: usize,
    last_eligibility_discard_receipts: Vec<PendingEligibilityDiscardReceipt>,
    last_pre_seal_discard_failures: Vec<PreSealDiscardFailure>,
    last_post_seal_learning_failures: Vec<PostSealLearningFailure>,
    last_gpu_metrics: GpuLiveBrainEvidenceMetrics,
    performance_metrics: GpuLivePerformanceMetrics,
    performance_measurement_enabled: bool,
    exact_checkpoint_transaction_started_at: Option<Instant>,
    checkpoint_durability: Option<GpuLiveCheckpointDurability>,
    canonical_save_id: Option<String>,
    manual_checkpoint_status: GpuManualCheckpointStatus,
    exact_checkpoint_coordinator: ExactPopulationCheckpointCoordinatorV1,
    exact_checkpoint_work: ExactPopulationCheckpointRuntimeWorkV1,
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
    #[cfg(any(test, feature = "gpu-tests"))]
    forced_late_advance_failure: bool,
}

pub const PLAYER_RESOURCE_PLACEMENT_SCHEMA_VERSION: u16 = 1;
const PLAYER_FOOD_NUTRITION: f32 = 0.25;
const PLAYER_FOOD_RADIUS: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerResourcePlacementRequest {
    pub schema_version: u16,
    pub position: Vec3f,
}

impl PlayerResourcePlacementRequest {
    pub const fn new(position: Vec3f) -> Self {
        Self {
            schema_version: PLAYER_RESOURCE_PLACEMENT_SCHEMA_VERSION,
            position,
        }
    }

    fn validate(self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != PLAYER_RESOURCE_PLACEMENT_SCHEMA_VERSION || self.position.y != 0.0
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        WorldEditCommand::place_food(
            "player-food-validation",
            self.position,
            PLAYER_FOOD_NUTRITION,
        )
        .validate(WorldEditorConfig::default())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerResourcePlacementReceipt {
    pub schema_version: u16,
    pub world_entity_id: WorldEntityId,
    pub label: String,
    pub position: Vec3f,
    pub nutrition: f32,
    pub radius: f32,
    pub world_signature: HeadlessWorldSignatureDigest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuLiveResidencySummary {
    pub handle_count: usize,
    pub resident_count: usize,
    pub memory_sidecar_count: usize,
    pub topology_sidecar_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuManualCheckpointStatus {
    Idle,
    Queued {
        destination: PathBuf,
        checkpoint_tick: Tick,
    },
    Complete {
        destination: PathBuf,
        checkpoint_tick: Tick,
    },
    Failed {
        destination: PathBuf,
        message: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuManualCheckpointRequestDisposition {
    Queued,
    Coalesced,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LiveRuntimeSaveAuthorityView {
    pub save_id: String,
    pub deterministic_seed: u64,
    pub sensor_profile: SensorProfile,
    pub organism_ids: Vec<OrganismId>,
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

type CuratedFounderDurableRefresh =
    fn(&mut GpuLiveCheckpointDurability, &str) -> Result<(), GameAppShellError>;

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

    let next_plan =
        CuratedFounderGpuResidencyPlan::from_accepted_operation(&operation, publication.receipt());
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

fn commit_staged_runtime<T, E, F>(live: &mut T, staged: Result<T, E>, commit: F) -> Result<(), E>
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
    for candidate in recall
        .context()
        .candidates
        .iter()
        .take(MAX_CONTEXT_MEMORY_EXPECTANCIES)
    {
        let expectancy = candidate
            .best_target_source
            .zip(candidate.target_latent.first().copied())
            .map(|(memory_id, value)| (memory_id, value, candidate.target_confidence.raw()))
            .or_else(|| {
                candidate
                    .best_family_source
                    .zip(candidate.family_value.first().copied())
                    .map(|(memory_id, value)| (memory_id, value, candidate.family_confidence.raw()))
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
    receptors: NeuralReceptorEffects,
) -> Result<(), ScaffoldContractError> {
    receptors.validate_contract()?;
    let concept_evidence = context
        .concept
        .active_concepts
        .iter()
        .map(|concept| concept.activation.raw() * concept.utility.raw())
        .fold(0.0, f32::max);
    let gap_evidence = context.gap.gap_voltage.raw().max(
        context
            .gap
            .active_gaps
            .iter()
            .map(|gap| gap.voltage.raw())
            .fold(0.0, f32::max),
    );
    for summary in summaries {
        summary.salience.drive =
            NormalizedScalar::new((body_need * receptors.interoceptive_gain).clamp(0.0, 1.0))?;
        summary.salience.peripheral_intensity = NormalizedScalar::new(
            (summary.salience.peripheral_intensity.raw()
                * receptors.regional_excitability
                * receptors.attention_gain)
                .clamp(0.0, 1.0),
        )?;
        summary.salience.concept =
            NormalizedScalar::new((concept_evidence * receptors.projection_gain).clamp(0.0, 1.0))?;
        summary.salience.gap_voltage = NormalizedScalar::new(
            (gap_evidence - receptors.local_threshold_shift).clamp(0.0, 1.0),
        )?;
        summary.salience.novelty = NormalizedScalar::new(
            summary
                .salience
                .novelty
                .raw()
                .max(receptors.structural_growth_gate * 0.25),
        )?;
        summary.salience.uncertainty = NormalizedScalar::new(
            summary
                .salience
                .uncertainty
                .raw()
                .max(receptors.sleep_gate * 0.25),
        )?;
        if let alife_core::StableFocusIdentity::TrackedObject(tracked_object_id) = summary.identity
        {
            if let Some(memory) = memory_evidence
                .iter()
                .find(|memory| memory.tracked_object_id == Some(tracked_object_id))
            {
                summary.salience.memory_expectancy = NormalizedScalar::new(
                    (memory.salience.raw()
                        * (0.5 * receptors.projection_gain + 0.5 * receptors.consolidation_gate))
                        .clamp(0.0, 1.0),
                )?;
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
        candidate.candidate_index =
            u16::try_from(index).map_err(|_| ScaffoldContractError::InvalidActionCandidate)?;
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

fn grounded_semantic_state_from_frame(
    frame: &PerceptionFrame,
) -> Result<SemanticStateVector, ScaffoldContractError> {
    let body = frame.body();
    let drives = frame.homeostasis().drives.to_array();
    SemanticStateVector::new(vec![
        bounded_successor_scalar(body.pose.translation.x)?,
        bounded_successor_scalar(body.pose.translation.y)?,
        bounded_successor_scalar(body.pose.translation.z)?,
        bounded_successor_scalar(body.velocity.linear.x)?,
        bounded_successor_scalar(body.velocity.linear.y)?,
        bounded_successor_scalar(body.velocity.linear.z)?,
        unit_successor_scalar(drives[0])?,
        unit_successor_scalar(drives[1])?,
        unit_successor_scalar(drives[2])?,
        unit_successor_scalar(drives[3])?,
        unit_successor_scalar(drives[4])?,
        unit_successor_scalar(drives[5])?,
        unit_successor_scalar(drives[6])?,
    ])
}

fn grounded_successor_state(
    world: &HeadlessWorld,
    world_entity_id: WorldEntityId,
    biology_after: &BiochemistryState,
    physical: alife_core::PhysicalActionOutcome,
    succeeded: bool,
    pain_delta: f32,
) -> Result<SemanticStateVector, ScaffoldContractError> {
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
        if succeeded { 1.0 } else { 0.0 },
        unit_successor_scalar(pain_delta)?,
    ];
    SemanticStateVector::new(features.to_vec())
}

const SINGLE_ACTION_COMPATIBILITY_ADAPTER_VERSION: u16 = 1;

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

    arbitrate_gpu_selected_command_into_factorized_bundle(
        organism_id,
        sequence_id,
        tick,
        channel_commands,
        compatibility_command,
        speech_payload,
        speech_prompted,
    )
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
    context.prediction.semantic_state_abi = target.source_state.abi_version;
    context.prediction.source_state = Some(target.source_state.clone());
    context.prediction.prediction_error = bounded_errors
        .iter()
        .copied()
        .map(NormalizedScalar::new)
        .collect::<Result<Vec<_>, _>>()?;
    context.prediction.action_sensitivity =
        NormalizedScalar::new(target.action_sensitivity_score.clamp(0.0, 1.0))?;

    let uncertainty = NormalizedScalar::new(mean_absolute_error)?;
    for summary in &mut context.attention.peripheral_summaries {
        summary.salience.uncertainty =
            NormalizedScalar::new(summary.salience.uncertainty.raw().max(mean_absolute_error))?;
        summary.salience.gap_voltage =
            NormalizedScalar::new(summary.salience.gap_voltage.raw().max(mean_absolute_error))?;
    }
    for salience in &mut context.attention.salience_components {
        salience.uncertainty = uncertainty;
        salience.gap_voltage =
            NormalizedScalar::new(salience.gap_voltage.raw().max(mean_absolute_error))?;
    }
    context.peripheral.summaries = context.attention.peripheral_summaries.clone();
    context.focal.salience = context.attention.salience_components.clone();
    context.gap.gap_voltage =
        NormalizedScalar::new(context.gap.gap_voltage.raw().max(mean_absolute_error))?;
    for gap in &mut context.gap.active_gaps {
        gap.voltage = NormalizedScalar::new(gap.voltage.raw().max(mean_absolute_error))?;
        gap.uncertainty = NormalizedScalar::new(gap.uncertainty.raw().max(mean_absolute_error))?;
    }
    context.validate_contract()?;
    Ok(mean_absolute_error)
}

fn attention_selection_policy_for(
    phenotype: &alife_core::BrainPhenotype,
) -> AttentionSelectionPolicy {
    let capacity = phenotype.cognitive_architecture().attention_capacity();
    AttentionSelectionPolicy {
        focal_capacity: capacity,
        requested_focal_count: capacity,
        ..AttentionSelectionPolicy::default()
    }
}

fn predictor_for_phenotype(
    phenotype: &alife_core::BrainPhenotype,
) -> Result<GroundedSuccessorPredictor, ScaffoldContractError> {
    GroundedSuccessorPredictor::with_learning_rate(
        phenotype.cognitive_architecture().predictor_learning_rate(),
    )
}

fn sleep_consolidation_config_for(
    phenotype: &alife_core::BrainPhenotype,
) -> Result<SleepConsolidationConfig, ScaffoldContractError> {
    let architecture = phenotype.cognitive_architecture();
    let plan = phenotype.sleep_consolidation_plan();
    let mut config = SleepConsolidationConfig::reference();
    config.sleep_pressure_threshold =
        NormalizedScalar::new(architecture.sleep_trigger_threshold())?;
    config.h_shadow_drain_rate = NormalizedScalar::new(plan.staging_rate())?;
    config.h_shadow_decay_rate = NormalizedScalar::new(plan.fast_decay_rate())?;
    config.lifetime_staging_rate = NormalizedScalar::new(architecture.sleep_consolidation_rate())?;
    config.structural_edit_candidate_limit =
        usize::from(architecture.structural_candidate_budget());
    config.weight_abs_limit = plan.weight_limit();
    config.validate_contract()?;
    Ok(config)
}

fn cognitive_work_receipt(
    context: &CognitiveContextFrame,
    memory: &MemoryRecallReceipt,
    neural_work: &BrainWorkCounters,
    v11_work: &GpuV11WorkReceipt,
    prediction_ops: u64,
) -> Result<CognitiveWorkReceipt, ScaffoldContractError> {
    let memory_ops = u64::from(memory.exact_bucket_reads)
        .saturating_add(u64::from(memory.neighbor_bucket_reads))
        .saturating_add(u64::from(memory.similarity_evaluations));
    cognitive_work_receipt_from_subsystems(
        neural_work,
        v11_work,
        context.attention.budget_receipt.work_units,
        memory_ops,
        context.concept.active_concepts.len() as u64,
        context.gap.active_gaps.len() as u64,
        prediction_ops,
        0,
        1,
        0,
    )
}

#[allow(clippy::too_many_arguments)]
fn cognitive_work_receipt_from_subsystems(
    neural_work: &BrainWorkCounters,
    v11_work: &GpuV11WorkReceipt,
    focal_target_ops: u64,
    memory_ops: u64,
    concept_ops: u64,
    gap_ops: u64,
    prediction_ops: u64,
    replay_ops: u64,
    learning_ops: u64,
    sleep_ops: u64,
) -> Result<CognitiveWorkReceipt, ScaffoldContractError> {
    CognitiveWorkCounters::new(
        neural_work.neuron_updates,
        neural_work.synapse_ops,
        v11_work.cognitive.dendritic_ops,
        focal_target_ops,
        memory_ops,
        concept_ops,
        gap_ops,
        prediction_ops,
        replay_ops,
        v11_work.cognitive.structural_ops,
        learning_ops,
        sleep_ops,
    )?
    .into_receipt()
}

fn sleep_cognitive_work_receipt(
    sleep_work: &SleepWorkReceipt,
) -> Result<CognitiveWorkReceipt, ScaffoldContractError> {
    CognitiveWorkCounters::new(
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        u64::from(sleep_work.predictor_update_count),
        u64::from(sleep_work.replay_event_count)
            .saturating_add(u64::from(sleep_work.replay_eligibility_sample_count)),
        0,
        0,
        sleep_work.work_units,
    )?
    .into_receipt()
}

fn apply_cognitive_work_cost(
    world: &mut HeadlessWorld,
    organism_id: OrganismId,
    receipt: CognitiveWorkReceipt,
    policy: CognitiveWorkCostPolicy,
) -> Result<(), GameAppShellError> {
    let mut record = world
        .organism_registry()
        .get(organism_id)
        .cloned()
        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
    record
        .account_cognitive_work(receipt, policy)
        .map_err(|error| GameAppShellError::InvalidProductionFrontend {
            message: error.to_string(),
        })?;
    world.replace_organism_record_exact(record)?;
    Ok(())
}

fn replace_canonical_organism_record(
    world: &mut HeadlessWorld,
    replacement: WorldOrganismRecord,
) -> Result<(), ScaffoldContractError> {
    world.replace_organism_record_exact(replacement)
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
        v11_work,
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
    let source_state = grounded_semantic_state_from_frame(&frame)?;
    let motor_condition = JointMotorCondition::from_bundle(&motor_bundle)?;
    let neural_evidence = decision.neural_evidence()?;
    let neural_emission = NeuralEmissionFrame::new(
        frame.tick(),
        neural_evidence.dispatch_generation,
        vec![
            NeuralEmission::new(
                NeuralEmissionClass::RegionalArousal,
                neural_evidence.logit.abs().tanh(),
                neural_evidence.confidence.raw(),
            )?,
            NeuralEmission::new(
                NeuralEmissionClass::MotorCommitment,
                decision.confidence.raw(),
                decision.confidence.raw(),
            )?,
            NeuralEmission::new(
                NeuralEmissionClass::ExecutiveSustain,
                decision.confidence.raw(),
                1.0,
            )?,
        ],
    )?;
    let motor_receipt = world
        .apply_registered_motor_bundle_with_neural_emission(
            &motor_bundle,
            world_entity_id,
            &neural_emission,
        )
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
    let target_state = grounded_successor_state(
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
        source_state,
        motor_condition,
        target_state,
    )?;
    let prediction_update = resident.predictor.observe(&prediction_target)?;
    let grounded_prediction_error = apply_prediction_evidence(
        &mut cognitive_context,
        &prediction_target,
        &prediction_update.error,
    )?;
    let cognitive_work = cognitive_work_receipt(
        &cognitive_context,
        &memory,
        &work.counters,
        &v11_work,
        prediction_update.error.len() as u64,
    )?;
    resident.last_cognitive_context = Some(cognitive_context.clone());
    resident.last_selected_motor_bundle = Some(motor_bundle.clone());
    resident.last_cognitive_work = cognitive_work;
    let combined_prediction_error = grounded_prediction_error;
    let physiology = alife_core::MeasuredPhysiologyTransition::new(
        motor_receipt.biology_before,
        motor_receipt.biology_after,
    )?;
    let mut outcome = PostActionOutcome::new(
        organism_id,
        sequence_id,
        outcome_tick,
        succeeded,
        physical,
        physiology.homeostatic_delta,
        SignedValence::ZERO,
        NormalizedScalar::new(if succeeded { 0.0 } else { 1.0 })?,
        NormalizedScalar::new(physiology.pain_delta.raw().max(0.0))?,
        physiology.energy_delta,
        NormalizedScalar::new(combined_prediction_error)?,
    )?
    .with_measured_physiology(physiology)?;
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
    let mut causal_stages = Vec::with_capacity(8);
    if schedule_sleep {
        causal_stages.extend([
            LiveBrainCausalStage::EvaluateSleep,
            LiveBrainCausalStage::AdvanceSleep,
        ]);
    }
    causal_stages.extend([
        LiveBrainCausalStage::GatherSensory,
        LiveBrainCausalStage::RecallMemory,
        LiveBrainCausalStage::GpuBrainTick,
        LiveBrainCausalStage::ExecuteAction,
        LiveBrainCausalStage::MeasureOutcome,
        LiveBrainCausalStage::SealPatch,
    ]);
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
        causal_stages,
    };
    Ok(SealedWorldSelection { summary, patch })
}

impl Drop for GpuLiveBrainRuntime {
    fn drop(&mut self) {
        if self.flush_sleep_journal_publication_blocking().is_err() {
            self.backend
                .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
        }
    }
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
            save.deterministic_seed,
            save.config.brain_class,
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
                message:
                    "GPU neural runtime requires matching persisted configuration and save seed",
            });
        }
        validate_replacement_policy(
            save.config.brain_policy.policy,
            save.deterministic_seed,
            save.config.brain_class,
            deterministic_seed,
            brain_class,
        )?;
        let rollback_journal = durable_manifest.load_sleep_transaction_journal(&loaded_save)?;
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
        runtime.canonical_save_id = Some(save.save_id.clone());
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
        if !rollback_journal.entries.is_empty() {
            let exact_base = runtime
                .checkpoint_durability
                .as_ref()
                .expect("durability was just installed");
            let cleared = GpuSleepTransactionJournalV2::empty(&exact_base.published)?;
            exact_base
                .durable_manifest
                .publish_sleep_transaction_journal(&exact_base.published, &cleared)?;
            let refreshed = exact_base.durable_manifest.load()?;
            runtime
                .checkpoint_durability
                .as_mut()
                .expect("durability remains installed during startup reconciliation")
                .published = refreshed;
        }
        runtime.admit_restored_durable_completed_recommit(rollback_journal)?;
        if requires_checkpoint_reconciliation {
            runtime.persist_sleep_checkpoint_boundary()?;
        }
        Ok(runtime)
    }

    fn admit_restored_durable_completed_recommit(
        &mut self,
        rollback_journal: GpuSleepTransactionJournalV2,
    ) -> Result<(), GameAppShellError> {
        rollback_journal.validate()?;
        let promotion_entries = rollback_journal
            .entries
            .iter()
            .filter(|entry| {
                matches!(
                    (entry.source.consolidation, entry.target.consolidation),
                    (
                        ConsolidationState::Completed { .. },
                        ConsolidationState::Committed { .. }
                    )
                )
            })
            .collect::<Vec<_>>();
        let durability = self
            .checkpoint_durability
            .as_ref()
            .ok_or(ScaffoldContractError::MissingPhaseData)?;
        if rollback_journal.exact_base_manifest_digest
            != durability.published.exact_save_anchor_digest()?.0
            || rollback_journal.exact_base_checkpoint_tick != durability.published.save.world.tick
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
        }
        let completed = durability
            .published
            .save
            .creatures
            .iter()
            .filter_map(|creature| {
                let exact_brain = creature.gpu_brain.as_ref()?;
                matches!(
                    exact_brain.sleep.consolidation,
                    ConsolidationState::Completed { .. }
                )
                .then_some((creature.organism_id, exact_brain))
            })
            .map(|(organism_id, exact_brain)| {
                Ok::<_, GameAppShellError>((
                    organism_id,
                    exact_brain.sleep,
                    exact_brain.promoted_completed_sleep_state()?.sleep,
                ))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if completed.is_empty() {
            return Ok(());
        }
        if promotion_entries
            .iter()
            .any(|entry| !completed.iter().any(|(id, _, _)| *id == entry.organism_id))
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
        }
        let mut captured_journal_authorities = BTreeMap::new();
        for (organism_id, exact_sleep, promoted_sleep) in completed {
            // load_sleep_transaction_journal already proves that the first
            // entry for each organism starts at the exact base and that every
            // later entry chains byte-for-byte. Select that base-adjacent edge
            // by its complete source state; never overwrite it with a later
            // cycle's Completed -> Committed edge.
            let mut base_adjacent = promotion_entries
                .iter()
                .filter(|entry| entry.organism_id == organism_id && entry.source == exact_sleep);
            if let Some(entry) = base_adjacent.next() {
                if exact_sleep != entry.source || promoted_sleep != entry.target {
                    return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
                }
            }
            if base_adjacent.next().is_some() {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
            }
            let resident = self
                .residents
                .get(&organism_id.raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            if resident.sleep_scheduler.state() != exact_sleep {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
            }
            let handle = self
                .handles
                .get(&organism_id.raw())
                .copied()
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let authority = capture_sleep_journal_neural_authority(&mut self.backend, handle)?;
            if captured_journal_authorities
                .insert(organism_id.raw(), authority)
                .is_some()
            {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
            }
        }
        let checkpoint_tick = durability.published.save.world.tick;
        let expected_base_digest = durability.published.digest.as_str().to_string();
        let transaction_id = self
            .exact_checkpoint_coordinator
            .admit_durable_recommit(checkpoint_tick, expected_base_digest)?;
        self.exact_checkpoint_transaction_started_at =
            self.performance_measurement_enabled.then(Instant::now);
        self.performance_metrics
            .exact_checkpoint_transactions_started = self
            .performance_metrics
            .exact_checkpoint_transactions_started
            .saturating_add(1);
        let durability = self
            .checkpoint_durability
            .take()
            .expect("durability was validated before recommit admission");
        let published = durability.published.clone();
        let worker = spawn_exact_population_checkpoint_recommit_worker(durability);
        self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::AwaitingJournal {
            permit: DurableCompletedCheckpointPermitV1::Restored(
                RestoredDurableCompletedPermitV1 {
                    transaction_id,
                    checkpoint_tick,
                    published,
                    rollback_journal,
                    captured_journal_authorities,
                },
            ),
            worker,
        };
        Ok(())
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
        self.flush_sleep_journal_publication_blocking()?;
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
        .and_then(|mut candidate| {
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
            candidate.archive_birth_manifests = if preserve_lineage_archive {
                candidate
                    .world
                    .organism_registry()
                    .iter()
                    .map(|record| {
                        let organism_id = record.organism_id();
                        let digest = record.archive().birth_manifest_digest().ok_or_else(|| {
                            GameAppShellError::InvalidProductionFrontend {
                                message: format!(
                                    "loaded organism {} is missing persisted birth-manifest identity",
                                    organism_id.raw()
                                ),
                            }
                        })?;
                        Ok((organism_id.raw(), digest))
                    })
                    .collect::<Result<BTreeMap<_, _>, GameAppShellError>>()?
            } else {
                BTreeMap::new()
            };
            Ok(candidate)
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
            entry.projection.validate().map_err(|error| {
                CuratedFounderResetRuntimeError::GpuResidencyPreSubmit { error }
            })?;
            if entry.projection.sensor_profile() != self.sensor_profile {
                return Err(CuratedFounderResetRuntimeError::GpuResidencyPreSubmit {
                    error: ScaffoldContractError::SensorProfileMismatch,
                });
            }
            let phenotype = entry.projection.compiled_phenotype().clone();
            let capacity = BrainCapacityClass::production_for_id(phenotype.brain_class_id())
                .map_err(
                    |error| CuratedFounderResetRuntimeError::GpuResidencyPreSubmit { error },
                )?;
            let compiler_inputs = PhenotypeCompilerInputs::try_new_with_foundation_abi(
                entry.projection.source_brain_genome().clone(),
                &capacity,
                entry.projection.runtime_development_state().clone(),
                entry.projection.sensor_profile(),
                phenotype
                    .foundation_abi()
                    .canonical_v2()
                    .cloned()
                    .ok_or(ScaffoldContractError::PhenotypeCompile)
                    .map_err(
                        |error| CuratedFounderResetRuntimeError::GpuResidencyPreSubmit { error },
                    )?,
            )
            .map_err(|error| CuratedFounderResetRuntimeError::GpuResidencyPreSubmit { error })?;
            let verified = PhenotypeCompiler::compile_validated(&compiler_inputs, &capacity)
                .map_err(
                    |error| CuratedFounderResetRuntimeError::GpuResidencyPreSubmit { error },
                )?;
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
                legacy_nano512_compatibility_receipt: None,
                homeostasis: HomeostaticSnapshot::baseline(plan.world_tick),
                sleep_scheduler: GpuSleepScheduler::new(
                    sleep_consolidation_config_for(&phenotype).map_err(|error| {
                        CuratedFounderResetRuntimeError::GpuResidencyPreSubmit { error }
                    })?,
                )
                .map_err(|error| {
                    CuratedFounderResetRuntimeError::GpuResidencyPreSubmit { error }
                })?,
                next_sequence: 1,
                language_grounding: LanguageGroundingLedger::default(),
                life_statistics: PassiveLifeStatistics::new(entry.organism_id, plan.world_tick)
                    .map_err(
                        |error| CuratedFounderResetRuntimeError::GpuResidencyPreSubmit { error },
                    )?,
                attention_hysteresis: alife_core::HysteresisState::default(),
                predictor: predictor_for_phenotype(&phenotype).map_err(|error| {
                    CuratedFounderResetRuntimeError::GpuResidencyPreSubmit { error }
                })?,
                last_cognitive_context: None,
                last_selected_motor_bundle: None,
                last_cognitive_work: CognitiveWorkReceipt::zero(),
                last_sleep_work: None,
                last_structural_edit_receipts: Vec::new(),
                last_sleep_report: None,
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
                .map_err(|error| {
                    CuratedFounderResetRuntimeError::GpuResidencyPreSubmit { error }
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
            .zip(
                receipt
                    .ordered_residents
                    .iter()
                    .map(|resident| resident.handle),
            )
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
                return Err(
                    CuratedFounderResetRuntimeError::DurableCheckpointNotification {
                        evidence,
                        error,
                    },
                );
            }
        };
        if let Err(error) = self.backend.note_durable_checkpoint(durable_reference) {
            self.retained_curated_founder_operation = None;
            if let Some(plan) = self.retained_curated_founder_gpu_residency_plan.as_mut() {
                plan.state = CuratedFounderGpuResidencyState::Unknown;
            }
            let mut evidence = CuratedFounderResetRuntimeEvidence::from_attempt(result);
            evidence.gpu_residency = CuratedFounderGpuResidencyState::Unknown;
            return Err(
                CuratedFounderResetRuntimeError::DurableCheckpointNotification {
                    evidence,
                    error: error.into(),
                },
            );
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
        let mut residency_gate_rejections: u8 = 0;
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
            sleep_journal_neural_authorities: BTreeMap::new(),
            pending_exact_sleep_journal_entries: Vec::new(),
            sleep_journal_publication_worker: None,
            pending_sleep_journal_entries: Vec::new(),
            exact_checkpoint_waiting_for_sleep_journal: false,
            manual_checkpoint_waiting_for_sleep_journal: None,
            post_promotion_fail_stop_armed: false,
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
            restored_replay_patches: Vec::new(),
            last_sealed_patches: Vec::new(),
            observe_sidecars: options.observe_sidecars,
            retain_sealed_patch_history: options.retain_sealed_patch_history,
            last_learning_receipts: Vec::new(),
            last_gpu_authority_receipts: Vec::new(),
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
            #[cfg(feature = "gpu-tests")]
            last_sleep_memory_compaction_preparation_count: 0,
            last_eligibility_discard_receipts: Vec::new(),
            last_pre_seal_discard_failures: Vec::new(),
            last_post_seal_learning_failures: Vec::new(),
            last_gpu_metrics: GpuLiveBrainEvidenceMetrics::default(),
            performance_metrics: GpuLivePerformanceMetrics::default(),
            performance_measurement_enabled: false,
            exact_checkpoint_transaction_started_at: None,
            checkpoint_durability: None,
            canonical_save_id: None,
            manual_checkpoint_status: GpuManualCheckpointStatus::Idle,
            exact_checkpoint_coordinator: ExactPopulationCheckpointCoordinatorV1::default(),
            exact_checkpoint_work: ExactPopulationCheckpointRuntimeWorkV1::Idle,
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
            #[cfg(any(test, feature = "gpu-tests"))]
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
            sleep_journal_neural_authorities: BTreeMap::new(),
            pending_exact_sleep_journal_entries: Vec::new(),
            sleep_journal_publication_worker: None,
            pending_sleep_journal_entries: Vec::new(),
            exact_checkpoint_waiting_for_sleep_journal: false,
            manual_checkpoint_waiting_for_sleep_journal: None,
            post_promotion_fail_stop_armed: false,
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
            restored_replay_patches: Vec::new(),
            last_sealed_patches: Vec::new(),
            observe_sidecars: true,
            retain_sealed_patch_history: true,
            last_learning_receipts: Vec::new(),
            last_gpu_authority_receipts: Vec::new(),
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
            #[cfg(feature = "gpu-tests")]
            last_sleep_memory_compaction_preparation_count: 0,
            last_eligibility_discard_receipts: Vec::new(),
            last_pre_seal_discard_failures: Vec::new(),
            last_post_seal_learning_failures: Vec::new(),
            last_gpu_metrics: GpuLiveBrainEvidenceMetrics::default(),
            performance_metrics: GpuLivePerformanceMetrics::default(),
            performance_measurement_enabled: false,
            exact_checkpoint_transaction_started_at: None,
            checkpoint_durability: None,
            canonical_save_id: None,
            manual_checkpoint_status: GpuManualCheckpointStatus::Idle,
            exact_checkpoint_coordinator: ExactPopulationCheckpointCoordinatorV1::default(),
            exact_checkpoint_work: ExactPopulationCheckpointRuntimeWorkV1::Idle,
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
            #[cfg(any(test, feature = "gpu-tests"))]
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
                let checkpoint = GpuBrainCheckpointWrite {
                    save_state: state.clone(),
                    manifest_entries: Vec::new(),
                    checkpoint_digest: [0; 4],
                };
                let restored =
                    store.restore_brain_checkpoint(&mut runtime.backend, manifest, &checkpoint)?;
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
                            legacy_nano512_compatibility_receipt: restored
                                .legacy_nano512_compatibility_receipt
                                .as_ref(),
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
                    let exact_cognitive_state = restored
                        .exact_cognitive_state
                        .ok_or(ScaffoldContractError::MissingPhaseData)?;
                    if exact_cognitive_state.sleep_state != restored.sleep {
                        return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
                    }
                    let v11_checkpoint = runtime.backend.checkpoint_v11(handle)?;
                    if v11_checkpoint.dendritic_branches != exact_cognitive_state.dendritic_branches
                        || v11_checkpoint.structural != exact_cognitive_state.structural_plasticity
                    {
                        return Err(ScaffoldContractError::InvalidSparseProjectionSchema.into());
                    }
                    let sleep_scheduler = GpuSleepScheduler::restore(
                        sleep_consolidation_config_for(&restored.phenotype)?,
                        exact_cognitive_state.sleep_state,
                    )?;
                    let ExactCognitiveCheckpointState {
                        cognitive_context,
                        predictor,
                        selected_motor_bundle,
                        cognitive_work,
                        last_sleep_work,
                        structural_edit_receipts,
                        last_sleep_report,
                        ..
                    } = exact_cognitive_state;
                    let attention_hysteresis = cognitive_context.attention.hysteresis;
                    let last_cognitive_context = Some(cognitive_context);
                    let last_selected_motor_bundle = selected_motor_bundle;
                    let last_cognitive_work = cognitive_work;
                    let last_structural_edit_receipts = structural_edit_receipts;
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
                            neural_receptors: recovery.neural_receptors,
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
                        legacy_nano512_compatibility_receipt: authority
                            .legacy_nano512_compatibility_receipt
                            .clone(),
                        homeostasis: authority.biochemistry.homeostasis,
                        sleep_scheduler,
                        next_sequence,
                        language_grounding: restored.language_grounding,
                        life_statistics,
                        attention_hysteresis,
                        predictor,
                        last_cognitive_context,
                        last_selected_motor_bundle,
                        last_cognitive_work,
                        last_sleep_work,
                        last_structural_edit_receipts,
                        last_sleep_report,
                    };
                    Ok((
                        resident,
                        restored.memory,
                        restored.topology,
                        restored.tracked_objects,
                        restored.replay_patches,
                        retained_learning,
                        discarded_eligibility,
                    ))
                })();
                let (
                    resident,
                    memory,
                    topology,
                    tracked_objects,
                    replay_patches,
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
                runtime.restored_replay_patches.extend(replay_patches);
                tracked_object_states.push(tracked_objects);
                if let Some(discard) = discarded_eligibility {
                    runtime.last_eligibility_discard_receipts.push(discard);
                }
                if let Some(recovery) = retained_learning {
                    runtime.retained_learning.insert(raw, recovery);
                }
            } else {
                let (_, resident) =
                    Self::compile_birth(&runtime.world, brain_class, sensor_profile, organism_id)?;
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
                if let Err(error) =
                    cleanup_restored_gpu_handle(&mut runtime.backend, handle, pending_eligibility)
                {
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
                neural_receptors: &recovery.neural_receptors,
                attempts: recovery.attempts,
                last_error_code: recovery.last_error.slug(),
            });
        let replay_patches = replay_patches_for_checkpoint(
            &mut self.backend,
            handle,
            organism_id,
            &self.restored_replay_patches,
            &self.sealed_patches,
            &self.last_sealed_patches,
        )?;
        let mut write = store.capture_brain_with_runtime_replay_state(
            &mut self.backend,
            handle,
            &resident.phenotype,
            &resident.compiler_inputs,
            resident.sleep_scheduler.state(),
            self.world.tick(),
            None,
            &replay_patches,
            GpuBrainSidecarCapture {
                sensor_profile: memory.profile(),
                memory,
                topology,
                tracked_objects: self.world.tracked_objects().save_state(organism_id)?,
                language_grounding: &resident.language_grounding,
                life_statistics: &resident.life_statistics,
                legacy_nano512_compatibility_receipt: resident
                    .legacy_nano512_compatibility_receipt
                    .as_ref(),
                retained_learning,
            },
        )?;
        let exact = self.exact_cognitive_state_for_checkpoint(
            organism_id,
            handle,
            resident,
            self.world.tick(),
        )?;
        write.attach_exact_cognitive_state(store, &exact)?;
        Ok(write)
    }

    fn exact_cognitive_state_for_checkpoint(
        &self,
        organism_id: OrganismId,
        handle: GpuBrainHandle,
        resident: &ResidentCognition,
        checkpoint_tick: Tick,
    ) -> Result<ExactCognitiveCheckpointState, GameAppShellError> {
        let v11_checkpoint = self.backend.backend().checkpoint_v11(handle)?;
        Self::exact_cognitive_state_for_checkpoint_with_v11(
            organism_id,
            resident,
            checkpoint_tick,
            v11_checkpoint,
        )
    }

    fn exact_cognitive_state_for_checkpoint_with_v11(
        organism_id: OrganismId,
        resident: &ResidentCognition,
        checkpoint_tick: Tick,
        v11_checkpoint: alife_gpu_backend::GpuV11Checkpoint,
    ) -> Result<ExactCognitiveCheckpointState, GameAppShellError> {
        Self::exact_cognitive_host_snapshot(organism_id, resident, checkpoint_tick)?
            .with_captured_v11(&v11_checkpoint)
    }

    fn exact_cognitive_host_snapshot(
        organism_id: OrganismId,
        resident: &ResidentCognition,
        checkpoint_tick: Tick,
    ) -> Result<ExactCognitiveHostSnapshotV1, GameAppShellError> {
        let mut cognitive_context =
            resident
                .last_cognitive_context
                .clone()
                .unwrap_or(CognitiveContextFrame::empty(
                    organism_id,
                    ExperienceSequenceId(resident.next_sequence.max(1)),
                    checkpoint_tick,
                )?);
        cognitive_context.world_tick = checkpoint_tick;
        cognitive_context.attention.world_tick = checkpoint_tick;
        cognitive_context.validate_contract()?;

        let mut selected_motor_bundle = resident.last_selected_motor_bundle.clone();
        if let Some(bundle) = &mut selected_motor_bundle {
            bundle.tick = checkpoint_tick;
            bundle.validate_contract()?;
        }

        Ok(ExactCognitiveHostSnapshotV1 {
            organism_id,
            checkpoint_tick,
            cognitive_context,
            predictor: resident.predictor.clone(),
            selected_motor_bundle,
            cognitive_work: resident.last_cognitive_work,
            sleep_state: resident.sleep_scheduler.state(),
            last_sleep_work: resident.last_sleep_work.clone(),
            structural_edit_receipts: resident.last_structural_edit_receipts.clone(),
            last_sleep_report: resident.last_sleep_report.clone(),
        })
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
                message:
                    "durable checkpoint base seed or tick does not match the canonical live world"
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
                message: "durable checkpoint base contains an incompatible brain class".to_string(),
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
        let canonical_save_id = published.save.save_id.clone();
        let durability = GpuLiveCheckpointDurability {
            store,
            durable_manifest,
            published,
        };
        let durable_reference = durability.durable_reference()?;
        self.backend.note_durable_checkpoint(durable_reference)?;
        self.canonical_save_id = Some(canonical_save_id);
        self.checkpoint_durability = Some(durability);
        Ok(())
    }

    /// Captures one exact, sealed-boundary portable save without publishing it.
    /// The caller may atomically publish the returned manifest as a manual save;
    /// all bulk neural state remains behind content-addressed asset references.
    pub fn capture_portable_checkpoint(&mut self) -> Result<PortableSaveFile, GameAppShellError> {
        self.flush_sleep_journal_publication_blocking()?;
        let started = Instant::now();
        let readback_before = self.backend.mutable_slot_readback_metrics();
        let Some(durability) = self.checkpoint_durability.take() else {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "GPU runtime has no durable save boundary".to_string(),
            });
        };
        let base = durability.published.save.clone();
        let store = durability.store.clone();
        let result = self
            .capture_checkpointed_save(base, &store)
            .map(|(save, _)| save);
        self.checkpoint_durability = Some(durability);
        let readback_after = self.backend.mutable_slot_readback_metrics();
        self.performance_metrics.checkpoint_capture_calls = self
            .performance_metrics
            .checkpoint_capture_calls
            .saturating_add(1);
        self.performance_metrics.checkpoint_capture_wall_ns = self
            .performance_metrics
            .checkpoint_capture_wall_ns
            .saturating_add(u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX));
        self.performance_metrics.checkpoint_snapshot_calls = self
            .performance_metrics
            .checkpoint_snapshot_calls
            .saturating_add(readback_after.calls.saturating_sub(readback_before.calls));
        self.performance_metrics.checkpoint_snapshot_bytes = self
            .performance_metrics
            .checkpoint_snapshot_bytes
            .saturating_add(readback_after.bytes.saturating_sub(readback_before.bytes));
        self.performance_metrics.checkpoint_snapshot_poll_wait_ns = self
            .performance_metrics
            .checkpoint_snapshot_poll_wait_ns
            .saturating_add(
                readback_after
                    .poll_wait_ns
                    .saturating_sub(readback_before.poll_wait_ns),
            );
        self.performance_metrics
            .checkpoint_snapshot_map_receive_wait_ns = self
            .performance_metrics
            .checkpoint_snapshot_map_receive_wait_ns
            .saturating_add(
                readback_after
                    .map_receive_wait_ns
                    .saturating_sub(readback_before.map_receive_wait_ns),
            );
        result
    }

    pub(crate) fn rebind_durable_checkpoint_boundary(
        &mut self,
        save_path: impl AsRef<Path>,
        asset_root: impl AsRef<Path>,
        expected: &PortableSaveFile,
    ) -> Result<(), GameAppShellError> {
        let durable_manifest =
            GpuDurableSaveManifest::open(save_path.as_ref(), asset_root.as_ref())?;
        let published = durable_manifest.load()?;
        if published.save != *expected {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "rebound GPU checkpoint boundary differs from the exact save".to_string(),
            });
        }
        let store = GpuCheckpointAssetStore::new(durable_manifest.asset_root().to_path_buf())?;
        let candidate = GpuLiveCheckpointDurability {
            store,
            durable_manifest,
            published,
        };
        let durable_reference = candidate.durable_reference()?;
        self.backend.note_durable_checkpoint(durable_reference)?;
        self.canonical_save_id = Some(candidate.published.save.save_id.clone());
        self.checkpoint_durability = Some(candidate);
        Ok(())
    }

    fn capture_checkpointed_save(
        &mut self,
        mut replacement: PortableSaveFile,
        store: &GpuCheckpointAssetStore,
    ) -> Result<(PortableSaveFile, u64), GameAppShellError> {
        let checkpoint_tick = self.world.tick();
        self.add_missing_checkpoint_creature_summaries(&mut replacement)?;
        replacement.replace_headless_world_snapshot(&self.world)?;
        let mut manifest_entries = Vec::new();
        let mut exact_neural_captures = 0_u64;
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
            let exact = self.exact_cognitive_state_for_checkpoint(
                organism_id,
                handle,
                resident,
                checkpoint_tick,
            )?;
            let replay_patches = replay_patches_for_checkpoint(
                &mut self.backend,
                handle,
                organism_id,
                &self.restored_replay_patches,
                &self.sealed_patches,
                &self.last_sealed_patches,
            )?;
            let mut write = store.capture_brain_with_runtime_replay_state(
                &mut self.backend,
                handle,
                &resident.phenotype,
                &resident.compiler_inputs,
                resident.sleep_scheduler.state(),
                checkpoint_tick,
                None,
                &replay_patches,
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
                    tracked_objects: self.world.tracked_objects().save_state(organism_id)?,
                    language_grounding: &resident.language_grounding,
                    life_statistics: &resident.life_statistics,
                    legacy_nano512_compatibility_receipt: resident
                        .legacy_nano512_compatibility_receipt
                        .as_ref(),
                    retained_learning: self.retained_learning.get(&raw).map(|recovery| {
                        RetainedLearningCapture {
                            sealed_patch: &recovery.sealed_patch,
                            neural_receptors: &recovery.neural_receptors,
                            attempts: recovery.attempts,
                            last_error_code: recovery.last_error.slug(),
                        }
                    }),
                },
            )?;
            write.attach_exact_cognitive_state(store, &exact)?;
            exact_neural_captures = exact_neural_captures.saturating_add(1);
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
        Ok((replacement, exact_neural_captures))
    }

    fn freeze_exact_population_host_snapshot(
        &self,
        mut replacement: PortableSaveFile,
    ) -> Result<ExactPopulationHostSnapshotV1, GameAppShellError> {
        let checkpoint_tick = self.world.tick();
        self.add_missing_checkpoint_creature_summaries(&mut replacement)?;
        replacement.replace_headless_world_snapshot(&self.world)?;
        let mut brains = Vec::with_capacity(self.handles.len());
        for (&raw, &handle) in &self.handles {
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
            if resident.homeostasis != record.biochemistry().homeostasis
                || resident.homeostasis.tick != checkpoint_tick
                || resident.development.age_ticks != record.age_at(checkpoint_tick)?
            {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
            }
            let canonical_biochemistry = record.biochemistry().clone();
            let creature = replacement
                .creatures
                .iter_mut()
                .find(|creature| creature.organism_id == organism_id)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            if creature.brain_class != self.brain_class {
                return Err(ScaffoldContractError::PhenotypeCompile.into());
            }
            creature.development_tick = canonical_biochemistry.development.last_update_tick;
            creature.mind.tick = canonical_biochemistry.tick;
            creature.mind.homeostasis = canonical_biochemistry.homeostasis;
            creature.mind.sleep_state_label =
                gpu_sleep_state_label(resident.sleep_scheduler.state());
            brains.push(ExactBrainHostSnapshotV1 {
                handle,
                phenotype: resident.phenotype.clone(),
                compiler_inputs: resident.compiler_inputs.clone(),
                sleep: resident.sleep_scheduler.state(),
                memory: self
                    .memories
                    .get(&raw)
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                    .clone(),
                topology: self
                    .topologies
                    .get(&raw)
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                    .clone(),
                tracked_objects: self.world.tracked_objects().save_state(organism_id)?,
                language_grounding: resident.language_grounding.clone(),
                life_statistics: resident.life_statistics.clone(),
                legacy_nano512_compatibility_receipt: resident
                    .legacy_nano512_compatibility_receipt
                    .clone(),
                retained_learning: self.retained_learning.get(&raw).map(|recovery| {
                    ExactRetainedLearningHostSnapshotV1 {
                        sealed_patch: recovery.sealed_patch.clone(),
                        neural_receptors: recovery.neural_receptors.clone(),
                        attempts: recovery.attempts,
                        last_error_code: recovery.last_error.slug(),
                    }
                }),
                exact_cognitive_state: Self::exact_cognitive_host_snapshot(
                    organism_id,
                    resident,
                    checkpoint_tick,
                )?,
            });
        }
        if replacement.creatures.len() != brains.len() {
            return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
        }
        Ok(ExactPopulationHostSnapshotV1 {
            checkpoint_tick,
            replacement,
            brains,
            restored_replay_patches: self.restored_replay_patches.clone(),
            sealed_patches: self.sealed_patches.clone(),
            last_sealed_patches: self.last_sealed_patches.clone(),
        })
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
            let summary =
                checkpoint_creature_save_state(replacement, record, resident, self.brain_class)?;
            replacement.creatures.push(summary);
        }
        replacement
            .creatures
            .sort_by_key(|creature| creature.organism_id.raw());
        Ok(())
    }

    fn start_sleep_journal_publication(
        &mut self,
        entries: Vec<GpuSleepTransactionJournalEntryV2>,
    ) -> Result<(), GameAppShellError> {
        if entries.is_empty() {
            return Ok(());
        }
        if self.sleep_journal_publication_worker.is_some()
            || self.checkpoint_durability.is_none()
        {
            append_bounded_sleep_journal_entries(&mut self.pending_sleep_journal_entries, entries)?;
            self.performance_metrics.sleep_journal_pending_entries_peak = self
                .performance_metrics
                .sleep_journal_pending_entries_peak
                .max(u64::try_from(self.pending_sleep_journal_entries.len()).unwrap_or(u64::MAX));
            return Ok(());
        }
        let durability = self
            .checkpoint_durability
            .as_ref()
            .cloned()
            .ok_or(ScaffoldContractError::MissingPhaseData)?;
        self.sleep_journal_publication_worker = Some(spawn_sleep_journal_publication_worker(
            durability,
            entries,
            self.performance_measurement_enabled,
        ));
        self.performance_metrics.sleep_journal_worker_starts = self
            .performance_metrics
            .sleep_journal_worker_starts
            .saturating_add(1);
        Ok(())
    }

    fn exact_checkpoint_accepts_journal_entries(&self) -> bool {
        matches!(
            self.exact_checkpoint_work,
            ExactPopulationCheckpointRuntimeWorkV1::Capture { .. }
                | ExactPopulationCheckpointRuntimeWorkV1::Worker { .. }
                | ExactPopulationCheckpointRuntimeWorkV1::CommitWorker { .. }
                | ExactPopulationCheckpointRuntimeWorkV1::AwaitingJournal { .. }
                | ExactPopulationCheckpointRuntimeWorkV1::JournalWorker { .. }
                | ExactPopulationCheckpointRuntimeWorkV1::Finalizing { .. }
        )
    }

    fn resume_exact_checkpoint_after_sleep_journal_drain(
        &mut self,
    ) -> Result<(), GameAppShellError> {
        if self.sleep_journal_publication_worker.is_some()
            || !self.pending_sleep_journal_entries.is_empty()
            || !self.exact_checkpoint_waiting_for_sleep_journal
        {
            return Ok(());
        }
        self.exact_checkpoint_waiting_for_sleep_journal = false;
        self.request_exact_population_checkpoint()?;
        if let Some(destination) = self.manual_checkpoint_waiting_for_sleep_journal.take() {
            let _ = self.request_manual_checkpoint(destination)?;
        }
        Ok(())
    }

    fn poll_sleep_journal_publication(&mut self) -> Result<(), GameAppShellError> {
        let started = self.performance_measurement_enabled.then(Instant::now);
        self.performance_metrics.sleep_journal_worker_poll_calls = self
            .performance_metrics
            .sleep_journal_worker_poll_calls
            .saturating_add(1);
        let Some(mut worker) = self.sleep_journal_publication_worker.take() else {
            self.resume_exact_checkpoint_after_sleep_journal_drain()?;
            self.performance_metrics.sleep_journal_worker_poll_wall_ns = self
                .performance_metrics
                .sleep_journal_worker_poll_wall_ns
                .saturating_add(started.map_or(0, elapsed_ns));
            return Ok(());
        };
        match worker.poll() {
            SleepJournalPublicationWorkerPollV1::Pending => {
                self.sleep_journal_publication_worker = Some(worker);
            }
            SleepJournalPublicationWorkerPollV1::Panicked => {
                self.performance_metrics.sleep_journal_worker_failures = self
                    .performance_metrics
                    .sleep_journal_worker_failures
                    .saturating_add(1);
                self.backend
                    .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                return Err(ScaffoldContractError::NeuralBackendUnavailable.into());
            }
            SleepJournalPublicationWorkerPollV1::Ready(final_result) => {
                self.admit_sleep_journal_publication(final_result)?;
                if !self.pending_sleep_journal_entries.is_empty() {
                    let pending = std::mem::take(&mut self.pending_sleep_journal_entries);
                    self.start_sleep_journal_publication(pending)?;
                } else {
                    self.resume_exact_checkpoint_after_sleep_journal_drain()?;
                }
            }
        }
        self.performance_metrics.sleep_journal_worker_poll_wall_ns = self
            .performance_metrics
            .sleep_journal_worker_poll_wall_ns
            .saturating_add(started.map_or(0, elapsed_ns));
        Ok(())
    }

    fn admit_sleep_journal_publication(
        &mut self,
        final_result: SleepJournalPublicationWorkerFinalV1,
    ) -> Result<(), GameAppShellError> {
        self.performance_metrics.sleep_journal_worker_wall_ns = self
            .performance_metrics
            .sleep_journal_worker_wall_ns
            .saturating_add(final_result.worker_wall_ns);
        let (published, timing) = match final_result.result {
            Ok(result) => result,
            Err(error) => {
                self.performance_metrics.sleep_journal_worker_failures = self
                    .performance_metrics
                    .sleep_journal_worker_failures
                    .saturating_add(1);
                self.backend
                    .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                return Err(error);
            }
        };
        let durability = self
            .checkpoint_durability
            .as_mut()
            .ok_or(ScaffoldContractError::MissingPhaseData)?;
        let expected_published_generation = match final_result.expected_base_generation {
            Some(generation) => generation.checked_add(1),
            None => Some(1),
        };
        if durability.published.digest.as_str() != final_result.expected_base_digest
            || durability.published.authority_generation() != final_result.expected_base_generation
            || published.authority_generation() != expected_published_generation
            || durability.published.save != published.save
            || durability.published.exact_save_anchor_digest()?
                != published.exact_save_anchor_digest()?
        {
            self.backend
                .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
        }
        durability.published = published;
        self.record_sleep_journal_publication_timing(timing);
        self.performance_metrics.sleep_journal_worker_completions = self
            .performance_metrics
            .sleep_journal_worker_completions
            .saturating_add(1);
        self.performance_metrics.sleep_compact_journal_organisms = self
            .performance_metrics
            .sleep_compact_journal_organisms
            .saturating_add(final_result.entry_count);
        Ok(())
    }

    pub(crate) fn persistence_idle_for_shutdown(&self) -> bool {
        self.sleep_journal_publication_worker.is_none()
            && self.pending_sleep_journal_entries.is_empty()
            && !self.exact_checkpoint_waiting_for_sleep_journal
            && !self.exact_checkpoint_coordinator.is_active()
            && matches!(
                self.exact_checkpoint_work,
                ExactPopulationCheckpointRuntimeWorkV1::Idle
            )
    }

    pub(crate) fn persistence_failed_for_shutdown(&self) -> bool {
        self.exact_checkpoint_coordinator.stage() == ExactPopulationCheckpointStageV1::Failed
            || matches!(
                self.exact_checkpoint_work,
                ExactPopulationCheckpointRuntimeWorkV1::Failed
            )
    }

    pub(crate) fn persistence_terminal_for_shutdown(&self) -> bool {
        self.persistence_idle_for_shutdown() || self.persistence_failed_for_shutdown()
    }

    pub(crate) fn persistence_shutdown_diagnostics(&self) -> String {
        format!(
            "checkpoint={:?}; sleep_worker_active={}; pending_sleep_entries={}; exact_waiting_for_sleep_journal={}; manual_waiting={}",
            self.exact_checkpoint_performance_state(),
            self.sleep_journal_publication_worker.is_some(),
            self.pending_sleep_journal_entries.len(),
            self.exact_checkpoint_waiting_for_sleep_journal,
            self.manual_checkpoint_waiting_for_sleep_journal.is_some()
        )
    }

    pub(crate) fn poll_persistence_for_shutdown(&mut self) -> Result<(), GameAppShellError> {
        self.poll_sleep_journal_publication()?;
        self.poll_exact_population_checkpoint()?;
        if matches!(
            self.exact_checkpoint_work,
            ExactPopulationCheckpointRuntimeWorkV1::AwaitingJournal { .. }
        ) {
            // A normal tick promotes durable Completed sleep states before it
            // tells the exact worker to finalize. Shutdown has quiesced ticks,
            // so finalize the already-durable boundary without inventing that
            // later Completed -> Committed transition.
            self.finalize_awaiting_exact_checkpoint(&[])?;
        }
        Ok(())
    }

    fn flush_sleep_journal_publication_blocking(&mut self) -> Result<(), GameAppShellError> {
        loop {
            if let Some(worker) = self.sleep_journal_publication_worker.take() {
                let final_result = worker.finish().map_err(|_| {
                    GameAppShellError::from(ScaffoldContractError::NeuralBackendUnavailable)
                })?;
                self.admit_sleep_journal_publication(final_result)?;
            }
            if self.pending_sleep_journal_entries.is_empty() {
                return Ok(());
            }
            let pending = std::mem::take(&mut self.pending_sleep_journal_entries);
            self.start_sleep_journal_publication(pending)?;
        }
    }

    fn request_exact_population_checkpoint(&mut self) -> Result<(), GameAppShellError> {
        if let Some(active) = self.exact_checkpoint_coordinator.active_identity() {
            let expected_base_digest = active.expected_base_digest.clone();
            let _ = self
                .exact_checkpoint_coordinator
                .request_exact(self.world.tick(), expected_base_digest)?;
            return Ok(());
        }
        if self.sleep_journal_publication_worker.is_some()
            || !self.pending_sleep_journal_entries.is_empty()
        {
            self.exact_checkpoint_waiting_for_sleep_journal = true;
            return Ok(());
        }
        let Some(durability) = self.checkpoint_durability.as_ref() else {
            return Ok(());
        };
        let checkpoint_tick = self.world.tick();
        let expected_base_digest = durability.published.digest.as_str().to_string();
        let disposition = self
            .exact_checkpoint_coordinator
            .request_exact(checkpoint_tick, expected_base_digest.clone())?;
        let ExactCheckpointRequestDispositionV1::Started { transaction_id } = disposition else {
            return Ok(());
        };
        self.exact_checkpoint_transaction_started_at =
            self.performance_measurement_enabled.then(Instant::now);
        self.performance_metrics
            .exact_checkpoint_transactions_started = self
            .performance_metrics
            .exact_checkpoint_transactions_started
            .saturating_add(1);
        let result = (|| {
            let base = self
                .checkpoint_durability
                .as_ref()
                .ok_or(ScaffoldContractError::MissingPhaseData)?
                .published
                .save
                .clone();
            let host = self.freeze_exact_population_host_snapshot(base)?;
            let capacity =
                BrainCapacityClass::production_for_id(self.brain_class.default_class_id())?;
            let context =
                GpuExactCheckpointTransactionContextV1::capture(self.backend.backend(), &capacity)?;
            let handles = self.handles.values().copied().collect::<Vec<_>>();
            let ticket = self.backend.submit_exact_population_capture(
                checkpoint_tick,
                transaction_id,
                &handles,
            )?;
            self.performance_metrics.sleep_checkpoint_capture_calls = self
                .performance_metrics
                .sleep_checkpoint_capture_calls
                .saturating_add(1);
            // The submitted exact capture starts a new neural-authority epoch.
            // Ordinary journal transitions remain queued until that epoch is
            // durably installed, so the prior compact cache is no longer a
            // valid source for later edges.
            self.sleep_journal_neural_authorities.clear();
            self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Capture {
                transaction_id,
                expected_base_digest,
                host,
                context,
                ticket,
            };
            Ok::<_, GameAppShellError>(())
        })();
        if result.is_err() {
            self.exact_checkpoint_coordinator.fail_stop();
            self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Failed;
        }
        result
    }

    fn queue_exact_checkpoint_journal_entries(
        &mut self,
        entries: Vec<GpuSleepTransactionJournalEntryV2>,
    ) -> Result<(), GameAppShellError> {
        if !self.exact_checkpoint_coordinator.is_active() {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
        }
        append_bounded_sleep_journal_entries(
            &mut self.pending_exact_sleep_journal_entries,
            entries,
        )?;
        Ok(())
    }

    fn take_exact_checkpoint_journal_writes(
        &mut self,
        permit: &DurableCompletedCheckpointPermitV1,
    ) -> Result<
        (
            Vec<ExactPopulationCheckpointJournalPromotionV1>,
            Vec<(u64, SleepJournalNeuralAuthority)>,
        ),
        GameAppShellError,
    > {
        let entries = self.pending_exact_sleep_journal_entries.clone();
        let mut captured_targets = BTreeMap::new();
        let mut follow_up_required = false;
        for entry in &entries {
            if entry.transition_tick <= permit.checkpoint_tick() {
                captured_targets.insert(entry.organism_id.raw(), entry.target);
            } else {
                follow_up_required = true;
            }
        }
        for (raw, expected_sleep) in captured_targets {
            let captured_sleep = permit
                .published()
                .save
                .creatures
                .iter()
                .find(|creature| creature.organism_id.raw() == raw)
                .and_then(|creature| creature.gpu_brain.as_ref())
                .map(|brain| brain.sleep)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            if !captured_sleep_covers_queued_target(expected_sleep, captured_sleep) {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
            }
        }
        if follow_up_required {
            let _ = self.exact_checkpoint_coordinator.request_exact(
                self.world.tick(),
                permit.published().digest.as_str().to_string(),
            )?;
        }
        self.pending_exact_sleep_journal_entries.clear();
        Ok((Vec::new(), Vec::new()))
    }

    fn retain_failed_exact_checkpoint_worker(
        &mut self,
        transaction_id: u64,
        worker: ExactPopulationCheckpointWorkerOwnerV1,
        error: GameAppShellError,
    ) {
        self.exact_checkpoint_coordinator.fail_stop();
        self.backend
            .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
        self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::FailedJoining {
            transaction_id,
            failed: worker.abort_and_retain(error),
        };
    }

    fn retain_failed_exact_checkpoint_capture(
        &mut self,
        transaction_id: u64,
        ticket: GpuExactPopulationCaptureTicketV1,
        error: GameAppShellError,
    ) {
        self.exact_checkpoint_coordinator.fail_stop();
        self.backend
            .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
        self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::CaptureFailed {
            transaction_id,
            ticket,
            error: Some(error),
        };
    }

    fn poll_exact_population_checkpoint(&mut self) -> Result<(), GameAppShellError> {
        let work = std::mem::take(&mut self.exact_checkpoint_work);
        match work {
            ExactPopulationCheckpointRuntimeWorkV1::Idle => Ok(()),
            ExactPopulationCheckpointRuntimeWorkV1::Failed => {
                self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Failed;
                Err(ScaffoldContractError::NeuralBackendUnavailable.into())
            }
            ExactPopulationCheckpointRuntimeWorkV1::Capture {
                transaction_id,
                expected_base_digest,
                host,
                context,
                mut ticket,
            } => {
                if self.exact_checkpoint_coordinator.stage()
                    == ExactPopulationCheckpointStageV1::CaptureSubmitted
                {
                    if let Err(error) = self
                        .exact_checkpoint_coordinator
                        .transition(ExactPopulationCheckpointStageV1::MappingPending)
                    {
                        self.retain_failed_exact_checkpoint_capture(
                            transaction_id,
                            ticket,
                            error.into(),
                        );
                        return Ok(());
                    }
                }
                let poll = match self.backend.poll_exact_population_capture(&mut ticket) {
                    Ok(poll) => poll,
                    Err(error) => {
                        self.retain_failed_exact_checkpoint_capture(
                            transaction_id,
                            ticket,
                            error.into(),
                        );
                        return Ok(());
                    }
                };
                match poll {
                    GpuExactPopulationCapturePollV1::Pending => {
                        self.exact_checkpoint_work =
                            ExactPopulationCheckpointRuntimeWorkV1::Capture {
                                transaction_id,
                                expected_base_digest,
                                host,
                                context,
                                ticket,
                            };
                        Ok(())
                    }
                    GpuExactPopulationCapturePollV1::Failed(_) => {
                        self.exact_checkpoint_coordinator.fail_stop();
                        self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Failed;
                        self.backend
                            .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                        Err(ScaffoldContractError::NeuralBackendUnavailable.into())
                    }
                    GpuExactPopulationCapturePollV1::Ready(capture) => {
                        if let Err(error) = self
                            .exact_checkpoint_coordinator
                            .transition(ExactPopulationCheckpointStageV1::CpuBytesReady)
                        {
                            self.exact_checkpoint_coordinator.fail_stop();
                            self.exact_checkpoint_work =
                                ExactPopulationCheckpointRuntimeWorkV1::Failed;
                            self.backend
                                .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                            return Err(error.into());
                        }
                        let Some(active) =
                            self.exact_checkpoint_coordinator.active_identity().cloned()
                        else {
                            self.exact_checkpoint_coordinator.fail_stop();
                            self.exact_checkpoint_work =
                                ExactPopulationCheckpointRuntimeWorkV1::Failed;
                            self.backend
                                .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                            return Err(
                                ScaffoldContractError::ConsolidationGenerationMismatch.into()
                            );
                        };
                        if active.transaction_id != transaction_id
                            || active.checkpoint_tick != host.checkpoint_tick
                            || active.expected_base_digest != expected_base_digest
                            || capture.capture_transaction_generation() != transaction_id
                        {
                            self.exact_checkpoint_coordinator.fail_stop();
                            self.exact_checkpoint_work =
                                ExactPopulationCheckpointRuntimeWorkV1::Failed;
                            self.backend
                                .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                            return Err(
                                ScaffoldContractError::ConsolidationGenerationMismatch.into()
                            );
                        }
                        let Some(durability) = self.checkpoint_durability.take() else {
                            self.exact_checkpoint_coordinator.fail_stop();
                            self.exact_checkpoint_work =
                                ExactPopulationCheckpointRuntimeWorkV1::Failed;
                            self.backend
                                .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                            return Err(ScaffoldContractError::MissingPhaseData.into());
                        };
                        if let Err(error) = self
                            .exact_checkpoint_coordinator
                            .transition(ExactPopulationCheckpointStageV1::Encoding)
                        {
                            self.checkpoint_durability = Some(durability);
                            self.exact_checkpoint_coordinator.fail_stop();
                            self.exact_checkpoint_work =
                                ExactPopulationCheckpointRuntimeWorkV1::Failed;
                            self.backend
                                .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                            return Err(error.into());
                        }
                        let capture_transaction_generation =
                            capture.capture_transaction_generation();
                        let population_set_digest = capture.population_set_digest();
                        let worker = spawn_exact_population_checkpoint_worker(
                            transaction_id,
                            expected_base_digest.clone(),
                            host,
                            capture,
                            context,
                            durability,
                        );
                        self.exact_checkpoint_work =
                            ExactPopulationCheckpointRuntimeWorkV1::Worker {
                                transaction_id,
                                checkpoint_tick: active.checkpoint_tick,
                                expected_base_digest,
                                capture_transaction_generation,
                                population_set_digest,
                                worker,
                            };
                        Ok(())
                    }
                }
            }
            ExactPopulationCheckpointRuntimeWorkV1::CaptureFailed {
                transaction_id,
                mut ticket,
                mut error,
            } => match self.backend.poll_exact_population_capture(&mut ticket) {
                Ok(GpuExactPopulationCapturePollV1::Pending) => {
                    self.exact_checkpoint_work =
                        ExactPopulationCheckpointRuntimeWorkV1::CaptureFailed {
                            transaction_id,
                            ticket,
                            error,
                        };
                    Ok(())
                }
                Ok(GpuExactPopulationCapturePollV1::Ready(_))
                | Ok(GpuExactPopulationCapturePollV1::Failed(_)) => {
                    self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Failed;
                    Err(error
                        .take()
                        .unwrap_or_else(|| ScaffoldContractError::NeuralBackendUnavailable.into()))
                }
                Err(poll_error) => {
                    self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Failed;
                    Err(error.take().unwrap_or_else(|| poll_error.into()))
                }
            },
            ExactPopulationCheckpointRuntimeWorkV1::Worker {
                transaction_id,
                checkpoint_tick,
                expected_base_digest,
                capture_transaction_generation,
                population_set_digest,
                worker,
            } => match worker.try_recv_event() {
                Ok(None) => {
                    self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Worker {
                        transaction_id,
                        checkpoint_tick,
                        expected_base_digest,
                        capture_transaction_generation,
                        population_set_digest,
                        worker,
                    };
                    Ok(())
                }
                Err(_) => {
                    self.retain_failed_exact_checkpoint_worker(
                        transaction_id,
                        worker,
                        ScaffoldContractError::NeuralBackendUnavailable.into(),
                    );
                    Ok(())
                }
                Ok(Some(ExactPopulationCheckpointWorkerEventV1::Final(report))) => {
                    self.exact_checkpoint_work =
                        ExactPopulationCheckpointRuntimeWorkV1::Finalizing {
                            transaction_id,
                            report,
                            join_handle: worker.into_join_handle(),
                            journal_commit: None,
                        };
                    Ok(())
                }
                Ok(Some(ExactPopulationCheckpointWorkerEventV1::ExactPublished(_))) => {
                    self.retain_failed_exact_checkpoint_worker(
                        transaction_id,
                        worker,
                        ScaffoldContractError::ConsolidationGenerationMismatch.into(),
                    );
                    Ok(())
                }
                Ok(Some(ExactPopulationCheckpointWorkerEventV1::ManifestPrepared(prepared))) => {
                    if prepared.transaction_id != transaction_id
                        || prepared.checkpoint_tick != checkpoint_tick
                        || prepared.expected_base_digest != expected_base_digest
                        || prepared.capture_transaction_generation != capture_transaction_generation
                        || prepared.population_set_digest != population_set_digest
                        || prepared.prospective_durable_reference.checkpoint_tick != checkpoint_tick
                    {
                        self.retain_failed_exact_checkpoint_worker(
                            transaction_id,
                            worker,
                            ScaffoldContractError::ConsolidationGenerationMismatch.into(),
                        );
                        return Ok(());
                    }
                    if let Err(error) = self
                        .exact_checkpoint_coordinator
                        .transition(ExactPopulationCheckpointStageV1::ManifestPrepared)
                    {
                        self.retain_failed_exact_checkpoint_worker(
                            transaction_id,
                            worker,
                            error.into(),
                        );
                        return Ok(());
                    }
                    let permit = match self.backend.prevalidate_durable_checkpoint(
                        prepared.prospective_durable_reference.clone(),
                    ) {
                        Ok(permit) => permit,
                        Err(error) => {
                            let surfaced_error = GameAppShellError::from(error);
                            self.retain_failed_exact_checkpoint_worker(
                                transaction_id,
                                worker,
                                ScaffoldContractError::NeuralBackendUnavailable.into(),
                            );
                            return Err(surfaced_error);
                        }
                    };
                    if worker
                        .try_send_command(ExactPopulationCheckpointWorkerCommandV1::CommitExact)
                        .is_err()
                    {
                        self.retain_failed_exact_checkpoint_worker(
                            transaction_id,
                            worker,
                            ScaffoldContractError::NeuralBackendUnavailable.into(),
                        );
                        return Ok(());
                    }
                    self.exact_checkpoint_work =
                        ExactPopulationCheckpointRuntimeWorkV1::CommitWorker {
                            prepared,
                            permit,
                            worker,
                        };
                    Ok(())
                }
            },
            ExactPopulationCheckpointRuntimeWorkV1::CommitWorker {
                prepared,
                permit,
                worker,
            } => match worker.try_recv_event() {
                Ok(None) => {
                    self.exact_checkpoint_work =
                        ExactPopulationCheckpointRuntimeWorkV1::CommitWorker {
                            prepared,
                            permit,
                            worker,
                        };
                    Ok(())
                }
                Err(_) => {
                    self.retain_failed_exact_checkpoint_worker(
                        prepared.transaction_id,
                        worker,
                        ScaffoldContractError::NeuralBackendUnavailable.into(),
                    );
                    Ok(())
                }
                Ok(Some(ExactPopulationCheckpointWorkerEventV1::ManifestPrepared(_))) => {
                    self.retain_failed_exact_checkpoint_worker(
                        prepared.transaction_id,
                        worker,
                        ScaffoldContractError::ConsolidationGenerationMismatch.into(),
                    );
                    Ok(())
                }
                Ok(Some(ExactPopulationCheckpointWorkerEventV1::Final(report))) => {
                    self.exact_checkpoint_work =
                        ExactPopulationCheckpointRuntimeWorkV1::Finalizing {
                            transaction_id: prepared.transaction_id,
                            report,
                            join_handle: worker.into_join_handle(),
                            journal_commit: None,
                        };
                    Ok(())
                }
                Ok(Some(ExactPopulationCheckpointWorkerEventV1::ExactPublished(success))) => {
                    let transaction_id = prepared.transaction_id;
                    if success.transaction_id != transaction_id
                        || success.checkpoint_tick != prepared.checkpoint_tick
                        || success.expected_base_digest != prepared.expected_base_digest
                        || success.capture_transaction_generation
                            != prepared.capture_transaction_generation
                        || success.population_set_digest != prepared.population_set_digest
                        || success.durable_reference != prepared.prospective_durable_reference
                        || success.exact_neural_captures != prepared.exact_neural_captures
                        || success.captured_journal_authorities
                            != prepared.captured_journal_authorities
                    {
                        self.retain_failed_exact_checkpoint_worker(
                            transaction_id,
                            worker,
                            ScaffoldContractError::ConsolidationGenerationMismatch.into(),
                        );
                        return Ok(());
                    }
                    if let Err(error) = self
                        .exact_checkpoint_coordinator
                        .transition(ExactPopulationCheckpointStageV1::CasCommitted)
                    {
                        self.retain_failed_exact_checkpoint_worker(
                            transaction_id,
                            worker,
                            error.into(),
                        );
                        return Ok(());
                    }
                    if let Err(error) = self
                        .exact_checkpoint_coordinator
                        .transition(ExactPopulationCheckpointStageV1::ReloadValidated)
                    {
                        self.retain_failed_exact_checkpoint_worker(
                            transaction_id,
                            worker,
                            error.into(),
                        );
                        return Ok(());
                    }
                    self.backend.install_prevalidated_durable_checkpoint(permit);
                    self.performance_metrics
                        .sleep_exact_neural_capture_organisms = self
                        .performance_metrics
                        .sleep_exact_neural_capture_organisms
                        .saturating_add(success.exact_neural_captures);
                    if let Err(error) = self
                        .exact_checkpoint_coordinator
                        .transition(ExactPopulationCheckpointStageV1::DurablePermitInstalled)
                    {
                        self.retain_failed_exact_checkpoint_worker(
                            transaction_id,
                            worker,
                            error.into(),
                        );
                        return Ok(());
                    }
                    let permit = DurableCompletedCheckpointPermitV1::Captured(success);
                    let has_completed = permit.published().save.creatures.iter().any(|creature| {
                        creature.gpu_brain.as_ref().is_some_and(|brain| {
                            matches!(
                                brain.sleep.consolidation,
                                ConsolidationState::Completed { .. }
                            )
                        })
                    });
                    if has_completed {
                        self.exact_checkpoint_work =
                            ExactPopulationCheckpointRuntimeWorkV1::AwaitingJournal {
                                permit,
                                worker,
                            };
                    } else {
                        let (journal_writes, authorities) =
                            match self.take_exact_checkpoint_journal_writes(&permit) {
                                Ok(writes) => writes,
                                Err(error) => {
                                    self.retain_failed_exact_checkpoint_worker(
                                        transaction_id,
                                        worker,
                                        error,
                                    );
                                    return Ok(());
                                }
                            };
                        let journal_entry_count =
                            u64::try_from(journal_writes.len()).unwrap_or(u64::MAX);
                        let manual = match self
                            .exact_checkpoint_coordinator
                            .take_pending_manual_after_durable_permit()
                        {
                            Ok(manual) => manual,
                            Err(error) => {
                                self.retain_failed_exact_checkpoint_worker(
                                    transaction_id,
                                    worker,
                                    error.into(),
                                );
                                return Ok(());
                            }
                        };
                        if let Err(error) = self
                            .exact_checkpoint_coordinator
                            .transition(ExactPopulationCheckpointStageV1::DeferredJournalPublishing)
                        {
                            self.retain_failed_exact_checkpoint_worker(
                                transaction_id,
                                worker,
                                error.into(),
                            );
                            return Ok(());
                        }
                        if worker
                            .try_send_command(ExactPopulationCheckpointWorkerCommandV1::Finalize {
                                promotions: journal_writes,
                                manual,
                            })
                            .is_err()
                        {
                            self.retain_failed_exact_checkpoint_worker(
                                transaction_id,
                                worker,
                                ScaffoldContractError::NeuralBackendUnavailable.into(),
                            );
                            return Ok(());
                        }
                        self.exact_checkpoint_work =
                            ExactPopulationCheckpointRuntimeWorkV1::JournalWorker {
                                transaction_id,
                                worker,
                                journal_commit: Some(ExactPopulationCheckpointJournalCommitV1 {
                                    entry_count: journal_entry_count,
                                    authorities,
                                    contains_completed_promotion: false,
                                }),
                            };
                    }
                    Ok(())
                }
            },
            ExactPopulationCheckpointRuntimeWorkV1::AwaitingJournal { permit, worker } => {
                self.exact_checkpoint_work =
                    ExactPopulationCheckpointRuntimeWorkV1::AwaitingJournal { permit, worker };
                Ok(())
            }
            ExactPopulationCheckpointRuntimeWorkV1::JournalWorker {
                transaction_id,
                worker,
                journal_commit,
            } => match worker.try_recv_event() {
                Ok(None) => {
                    self.exact_checkpoint_work =
                        ExactPopulationCheckpointRuntimeWorkV1::JournalWorker {
                            transaction_id,
                            worker,
                            journal_commit,
                        };
                    Ok(())
                }
                Err(_) => {
                    self.retain_failed_exact_checkpoint_worker(
                        transaction_id,
                        worker,
                        ScaffoldContractError::NeuralBackendUnavailable.into(),
                    );
                    Ok(())
                }
                Ok(Some(
                    ExactPopulationCheckpointWorkerEventV1::ManifestPrepared(_)
                    | ExactPopulationCheckpointWorkerEventV1::ExactPublished(_),
                )) => {
                    self.retain_failed_exact_checkpoint_worker(
                        transaction_id,
                        worker,
                        ScaffoldContractError::ConsolidationGenerationMismatch.into(),
                    );
                    Ok(())
                }
                Ok(Some(ExactPopulationCheckpointWorkerEventV1::Final(report))) => {
                    self.exact_checkpoint_work =
                        ExactPopulationCheckpointRuntimeWorkV1::Finalizing {
                            transaction_id,
                            report,
                            join_handle: worker.into_join_handle(),
                            journal_commit,
                        };
                    Ok(())
                }
            },
            ExactPopulationCheckpointRuntimeWorkV1::FailedJoining {
                transaction_id,
                mut failed,
            } => match failed.poll() {
                FailedExactPopulationCheckpointWorkerJoinPollV1::Pending => {
                    self.exact_checkpoint_work =
                        ExactPopulationCheckpointRuntimeWorkV1::FailedJoining {
                            transaction_id,
                            failed,
                        };
                    Ok(())
                }
                FailedExactPopulationCheckpointWorkerJoinPollV1::Ready {
                    error,
                    worker_panicked,
                } => {
                    self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Failed;
                    if worker_panicked {
                        Err(ScaffoldContractError::NeuralBackendUnavailable.into())
                    } else {
                        Err(error)
                    }
                }
            },
            ExactPopulationCheckpointRuntimeWorkV1::Finalizing {
                transaction_id,
                report,
                join_handle,
                journal_commit,
            } => {
                if !join_handle.is_finished() {
                    self.exact_checkpoint_work =
                        ExactPopulationCheckpointRuntimeWorkV1::Finalizing {
                            transaction_id,
                            report,
                            join_handle,
                            journal_commit,
                        };
                    return Ok(());
                }
                if join_handle.join().is_err() {
                    self.exact_checkpoint_coordinator.fail_stop();
                    self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Failed;
                    self.backend
                        .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                    return Err(ScaffoldContractError::NeuralBackendUnavailable.into());
                }
                self.checkpoint_durability = Some(report.durability);
                if let Err(error) = report.result {
                    if let GpuManualCheckpointStatus::Queued { destination, .. } =
                        &self.manual_checkpoint_status
                    {
                        self.manual_checkpoint_status = GpuManualCheckpointStatus::Failed {
                            destination: destination.clone(),
                            message: error.to_string(),
                        };
                    }
                    self.exact_checkpoint_coordinator.fail_stop();
                    self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Failed;
                    self.backend
                        .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                    return Err(error);
                }
                if let Some(manual) = report.manual_completion {
                    self.manual_checkpoint_status = GpuManualCheckpointStatus::Complete {
                        destination: manual.destination,
                        checkpoint_tick: manual.checkpoint_tick,
                    };
                }
                if let Some(journal_commit) = journal_commit {
                    if journal_commit.contains_completed_promotion {
                        self.performance_metrics.sleep_promotion_publish_calls = self
                            .performance_metrics
                            .sleep_promotion_publish_calls
                            .saturating_add(1);
                    }
                    let mut current_authorities =
                        Vec::with_capacity(journal_commit.authorities.len());
                    for (organism_id_raw, _) in &journal_commit.authorities {
                        let Some(handle) = self.handles.get(organism_id_raw).copied() else {
                            self.exact_checkpoint_coordinator.fail_stop();
                            self.exact_checkpoint_work =
                                ExactPopulationCheckpointRuntimeWorkV1::Failed;
                            self.backend
                                .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                            return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
                        };
                        let authority =
                            match capture_sleep_journal_neural_authority(&mut self.backend, handle)
                            {
                                Ok(authority) => authority,
                                Err(error) => {
                                    self.exact_checkpoint_coordinator.fail_stop();
                                    self.exact_checkpoint_work =
                                        ExactPopulationCheckpointRuntimeWorkV1::Failed;
                                    self.backend.fail_stop(
                                        GpuSessionFailStopCause::CheckpointRestoreFailed,
                                    );
                                    return Err(error.into());
                                }
                            };
                        current_authorities.push((*organism_id_raw, authority));
                    }
                    // Worker validation and publication use the immutable
                    // tick-T authority. Only after that durable success may
                    // later compact edges bind the now-promoted resident host
                    // metadata. This performs no mutable-buffer readback.
                    for (organism_id_raw, authority) in current_authorities {
                        self.sleep_journal_neural_authorities
                            .insert(organism_id_raw, authority);
                    }
                    self.performance_metrics.sleep_compact_journal_organisms = self
                        .performance_metrics
                        .sleep_compact_journal_organisms
                        .saturating_add(journal_commit.entry_count);
                }
                if !self.pending_exact_sleep_journal_entries.is_empty() {
                    let durability = self
                        .checkpoint_durability
                        .as_ref()
                        .ok_or(ScaffoldContractError::MissingPhaseData)?;
                    let _ = self.exact_checkpoint_coordinator.request_exact(
                        self.world.tick(),
                        durability.published.digest.as_str().to_string(),
                    )?;
                }
                if let Err(error) = self
                    .exact_checkpoint_coordinator
                    .transition(ExactPopulationCheckpointStageV1::Complete)
                {
                    self.exact_checkpoint_coordinator.fail_stop();
                    self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Failed;
                    self.backend
                        .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                    return Err(error.into());
                }
                let follow_up = match self.exact_checkpoint_coordinator.finish() {
                    Ok(follow_up) => follow_up,
                    Err(error) => {
                        self.exact_checkpoint_coordinator.fail_stop();
                        self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Failed;
                        self.backend
                            .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                        return Err(error.into());
                    }
                };
                self.performance_metrics
                    .exact_checkpoint_transactions_completed = self
                    .performance_metrics
                    .exact_checkpoint_transactions_completed
                    .saturating_add(1);
                self.performance_metrics
                    .exact_checkpoint_transaction_wall_ns = self
                    .performance_metrics
                    .exact_checkpoint_transaction_wall_ns
                    .saturating_add(
                        self.exact_checkpoint_transaction_started_at
                            .take()
                            .map_or(0, elapsed_ns),
                    );
                self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Idle;
                if !self.pending_sleep_journal_entries.is_empty() {
                    let pending = std::mem::take(&mut self.pending_sleep_journal_entries);
                    self.start_sleep_journal_publication(pending)?;
                }
                if follow_up {
                    if let Err(error) = self.request_exact_population_checkpoint() {
                        self.exact_checkpoint_coordinator.fail_stop();
                        self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Failed;
                        self.backend
                            .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                        return Err(error);
                    }
                }
                Ok(())
            }
        }
    }

    fn persist_sleep_checkpoint_boundary(&mut self) -> Result<(), GameAppShellError> {
        self.flush_sleep_journal_publication_blocking()?;
        let Some(mut durability) = self.checkpoint_durability.take() else {
            return Ok(());
        };
        self.performance_metrics.sleep_persistence_calls = self
            .performance_metrics
            .sleep_persistence_calls
            .saturating_add(1);
        let store = durability.store.clone();
        let readback_before = self.backend.mutable_slot_readback_metrics();
        let capture_started = Instant::now();
        let replacement = self.capture_checkpointed_save(durability.published.save.clone(), &store);
        let readback_after = self.backend.mutable_slot_readback_metrics();
        self.performance_metrics.sleep_checkpoint_capture_calls = self
            .performance_metrics
            .sleep_checkpoint_capture_calls
            .saturating_add(1);
        self.performance_metrics.sleep_checkpoint_capture_wall_ns = self
            .performance_metrics
            .sleep_checkpoint_capture_wall_ns
            .saturating_add(elapsed_ns(capture_started));
        self.performance_metrics.sleep_checkpoint_readback_calls = self
            .performance_metrics
            .sleep_checkpoint_readback_calls
            .saturating_add(readback_after.calls.saturating_sub(readback_before.calls));
        self.performance_metrics.sleep_checkpoint_readback_bytes = self
            .performance_metrics
            .sleep_checkpoint_readback_bytes
            .saturating_add(readback_after.bytes.saturating_sub(readback_before.bytes));
        self.performance_metrics
            .sleep_checkpoint_readback_poll_wait_ns = self
            .performance_metrics
            .sleep_checkpoint_readback_poll_wait_ns
            .saturating_add(
                readback_after
                    .poll_wait_ns
                    .saturating_sub(readback_before.poll_wait_ns),
            );
        self.performance_metrics
            .sleep_checkpoint_readback_map_receive_wait_ns = self
            .performance_metrics
            .sleep_checkpoint_readback_map_receive_wait_ns
            .saturating_add(
                readback_after
                    .map_receive_wait_ns
                    .saturating_sub(readback_before.map_receive_wait_ns),
            );
        let result = match replacement {
            Ok((replacement, exact_neural_captures)) => {
                self.performance_metrics
                    .sleep_exact_neural_capture_organisms = self
                    .performance_metrics
                    .sleep_exact_neural_capture_organisms
                    .saturating_add(exact_neural_captures);
                let prospective = durability.prospective_durable_reference(&replacement);
                match prospective.and_then(|reference| {
                    self.backend
                        .prevalidate_durable_checkpoint(reference)
                        .map_err(Into::into)
                }) {
                    Ok(permit) => {
                        let publish_started = Instant::now();
                        let result = durability.publish(replacement).map(|_| permit);
                        self.performance_metrics.sleep_checkpoint_publish_calls = self
                            .performance_metrics
                            .sleep_checkpoint_publish_calls
                            .saturating_add(1);
                        self.performance_metrics.sleep_checkpoint_publish_wall_ns = self
                            .performance_metrics
                            .sleep_checkpoint_publish_wall_ns
                            .saturating_add(elapsed_ns(publish_started));
                        result
                    }
                    Err(error) => Err(error),
                }
            }
            Err(error) => Err(error),
        };
        self.checkpoint_durability = Some(durability);
        let permit = result?;
        self.backend.install_prevalidated_durable_checkpoint(permit);
        Ok(())
    }

    fn promote_durable_completed_sleep_batch(
        &mut self,
        promotions: &[(OrganismId, SleepState)],
    ) -> Result<(), GameAppShellError> {
        if promotions.is_empty() {
            return Ok(());
        }
        self.finalize_awaiting_exact_checkpoint(promotions)
    }

    fn finalize_awaiting_exact_checkpoint(
        &mut self,
        promotions: &[(OrganismId, SleepState)],
    ) -> Result<(), GameAppShellError> {
        let work = std::mem::take(&mut self.exact_checkpoint_work);
        let ExactPopulationCheckpointRuntimeWorkV1::AwaitingJournal { permit, worker } = work
        else {
            self.exact_checkpoint_work = work;
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
        };
        permit.validate_restored_provenance()?;
        let transaction_id = permit.transaction_id();
        let (mut worker_promotions, queued_authorities) =
            match self.take_exact_checkpoint_journal_writes(&permit) {
                Ok(writes) => writes,
                Err(error) => {
                    self.retain_failed_exact_checkpoint_worker(transaction_id, worker, error);
                    return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
                }
            };
        if !promotions.is_empty() {
            self.performance_metrics.sleep_promotion_calls = self
                .performance_metrics
                .sleep_promotion_calls
                .saturating_add(1);
        }
        let prepared = (|| {
            let mut ordered = promotions.to_vec();
            ordered.sort_unstable_by_key(|(organism_id, _)| organism_id.raw());
            if ordered.windows(2).any(|pair| pair[0].0 == pair[1].0) {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
            }
            let base = &permit.published().save;
            let mut authorities = queued_authorities
                .iter()
                .cloned()
                .collect::<BTreeMap<_, _>>();
            for (organism_id, committed_sleep) in ordered {
                let creature = base
                    .creatures
                    .iter()
                    .find(|creature| creature.organism_id == organism_id)
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                let completed = creature
                    .gpu_brain
                    .as_ref()
                    .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
                let promoted = completed.promoted_completed_sleep_state()?;
                if promoted.sleep != committed_sleep || promoted.checkpoint_tick != base.world.tick
                {
                    return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
                }
                let resident = self
                    .residents
                    .get(&organism_id.raw())
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                let authority = permit
                    .captured_journal_authorities()
                    .get(&organism_id.raw())
                    .cloned()
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                let entry = GpuSleepTransactionJournalEntryV2::try_new(
                    organism_id,
                    Tick::new(completed.checkpoint_tick.raw().saturating_add(1)),
                    completed.sleep,
                    committed_sleep,
                )?;
                worker_promotions.push(ExactPopulationCheckpointJournalPromotionV1 {
                    entry,
                    authority: authority.clone(),
                    phenotype: resident.phenotype.clone(),
                });
                authorities.insert(organism_id.raw(), authority);
            }
            worker_promotions.sort_unstable_by_key(|write| {
                (
                    write.entry.organism_id.raw(),
                    write.entry.transition_tick.raw(),
                    write.entry.transition_ordinal,
                )
            });
            Ok::<_, GameAppShellError>((
                worker_promotions,
                authorities.into_iter().collect::<Vec<_>>(),
            ))
        })();
        let (worker_promotions, authorities) = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                self.retain_failed_exact_checkpoint_worker(
                    transaction_id,
                    worker,
                    ScaffoldContractError::NeuralBackendUnavailable.into(),
                );
                return Err(error);
            }
        };
        let entry_count = u64::try_from(worker_promotions.len()).unwrap_or(u64::MAX);
        let manual = match self
            .exact_checkpoint_coordinator
            .take_pending_manual_after_durable_permit()
        {
            Ok(manual) => manual,
            Err(error) => {
                self.retain_failed_exact_checkpoint_worker(
                    transaction_id,
                    worker,
                    ScaffoldContractError::NeuralBackendUnavailable.into(),
                );
                return Err(error.into());
            }
        };
        if let Err(error) = self
            .exact_checkpoint_coordinator
            .transition(ExactPopulationCheckpointStageV1::DeferredJournalPublishing)
        {
            self.retain_failed_exact_checkpoint_worker(
                transaction_id,
                worker,
                ScaffoldContractError::NeuralBackendUnavailable.into(),
            );
            return Err(error.into());
        }
        if worker
            .try_send_command(ExactPopulationCheckpointWorkerCommandV1::Finalize {
                promotions: worker_promotions,
                manual,
            })
            .is_err()
        {
            let error = GameAppShellError::Core(ScaffoldContractError::NeuralBackendUnavailable);
            self.retain_failed_exact_checkpoint_worker(transaction_id, worker, error);
            return Err(ScaffoldContractError::NeuralBackendUnavailable.into());
        }
        self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::JournalWorker {
            transaction_id,
            worker,
            journal_commit: Some(ExactPopulationCheckpointJournalCommitV1 {
                authorities,
                entry_count,
                contains_completed_promotion: !promotions.is_empty(),
            }),
        };
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
    pub fn execute_managed_breed(
        &mut self,
        receipt: HabitatBreedingReceipt,
    ) -> Result<OrganismId, GameAppShellError> {
        let max_live_id = self
            .world
            .organism_registry()
            .iter()
            .map(|record| record.organism_id().raw())
            .chain(
                self.world
                    .organism_entity_ids()
                    .into_iter()
                    .map(|(organism_id, _)| organism_id.raw()),
            )
            .chain(self.handles.keys().copied())
            .chain(self.residents.keys().copied())
            .chain(self.archive_birth_manifests.keys().copied())
            .max()
            .unwrap_or(0);
        let child_raw = max_live_id
            .checked_add(1)
            .ok_or(ScaffoldContractError::InvalidId)?;
        let child_organism_id = OrganismId(child_raw);
        let conception_seed = receipt
            .tick
            .raw()
            .rotate_left(17)
            .wrapping_add(receipt.first_parent.raw().rotate_left(31))
            .wrapping_add(receipt.second_parent.raw().rotate_right(11))
            .wrapping_add(receipt.habitat_id.raw().rotate_left(7))
            .wrapping_add(0xC0A1_CE71_4A2D_0001);

        self.apply_managed_breed_receipt(receipt, child_organism_id, conception_seed)?;
        Ok(child_organism_id)
    }

    pub fn apply_managed_breed_receipt(
        &mut self,
        receipt: HabitatBreedingReceipt,
        child_organism_id: OrganismId,
        conception_seed: u64,
    ) -> Result<(), GameAppShellError> {
        let invalid_receipt =
            |message: String| GameAppShellError::InvalidProductionFrontend { message };
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
        let resident_set_before = self.handles.keys().copied().collect::<BTreeSet<_>>();
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
        let resident_set_after = self.handles.keys().copied().collect::<BTreeSet<_>>();
        if resident_set_after != resident_set_before {
            if let Err(error) = self.request_exact_population_checkpoint() {
                self.backend
                    .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                return Err(error);
            }
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
            let birth_manifest_digest =
                *self.archive_birth_manifests.get(&raw).ok_or_else(|| {
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

    fn prepare_memory_compaction_at_sleep_commit(
        &mut self,
        organism_id: OrganismId,
        completed_sleep: SleepState,
    ) -> Result<(MemorySidecarState, MemoryCompactionReceipt), GameAppShellError> {
        let cycle_id = match completed_sleep.consolidation {
            ConsolidationState::Completed { request, .. } if request.cycle_id != 0 => {
                request.cycle_id
            }
            _ => return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into()),
        };
        #[cfg(feature = "gpu-tests")]
        if self
            .forced_memory_preparation_failures
            .remove(&organism_id.raw())
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery.into());
        }
        let mut memory = self
            .memories
            .get(&organism_id.raw())
            .cloned()
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let max_records_after = u32::try_from(memory.bank().capacity())
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
        let prepared = memory.prepare_compaction(cycle_id, max_records_after, 1)?;
        let receipt = memory.commit_compaction(prepared)?;
        Ok((memory, receipt))
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
        let recovery_receptors = recovery.neural_receptors.clone();
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
                .apply_sealed_outcome_batch(&[(
                    current_handle,
                    &recovery_patch,
                    &recovery_receptors,
                )])
                .and_then(|mut receipts| {
                    receipts
                        .pop()
                        .ok_or(ScaffoldContractError::LearningEvidenceMismatch)
                })
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
        Ok(match self.tick_outcome()? {
            GpuLiveTickOutcome::Progressed(summaries) => summaries,
            GpuLiveTickOutcome::NoProgress(_) => Vec::new(),
        })
    }

    pub fn tick_outcome(&mut self) -> Result<GpuLiveTickOutcome, GameAppShellError> {
        let started = Instant::now();
        let result =
            self.tick_with_sleep_progress_outcome(|backend, handle, organism_id, state, intent| {
                let mut driver = AuthoritativeGpuSleepDriver {
                    backend,
                    handle,
                    sleep_config: None,
                    context: None,
                    replay_evidence_before_commit: None,
                    last_sleep_work: None,
                };
                driver.progress(organism_id, state, intent)
            });
        self.performance_metrics.tick_calls = self.performance_metrics.tick_calls.saturating_add(1);
        self.performance_metrics.tick_wall_ns = self
            .performance_metrics
            .tick_wall_ns
            .saturating_add(u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX));
        result
    }

    pub(crate) fn live_cognitive_presentation_snapshots(
        &self,
        summaries: &[LiveBrainTickSummary],
    ) -> Vec<LiveCognitivePresentationSnapshot> {
        summaries
            .iter()
            .filter_map(|summary| {
                let organism_id = summary.organism_id;
                let resident = self.residents.get(&organism_id.raw())?;
                let memory = self.memories.get(&organism_id.raw());
                let topology = self.topologies.get(&organism_id.raw());
                let sleep = resident.sleep_scheduler.state();
                let learning_active = if self
                    .last_learning_receipts
                    .iter()
                    .any(|receipt| receipt.handle.organism_id() == organism_id)
                {
                    Some(true)
                } else if self
                    .last_post_seal_learning_failures
                    .iter()
                    .any(|failure| failure.organism_id == organism_id)
                    || self.retained_learning.contains_key(&organism_id.raw())
                {
                    Some(false)
                } else {
                    None
                };
                let last_consolidated_tick = resident
                    .last_sleep_work
                    .as_ref()
                    .filter(|work| {
                        matches!(
                            work.status,
                            alife_core::sleep::SleepWorkStatus::Consolidated
                        )
                    })
                    .map(|work| work.tick.raw());
                let topology_counts = topology.map(|sidecar| sidecar.counts());

                Some(LiveCognitivePresentationSnapshot {
                    organism_id,
                    brain_class_id: Some(resident.phenotype.brain_class_id().raw()),
                    brain_neuron_count: Some(resident.phenotype.neuron_count()),
                    fast_memory_count: memory
                        .and_then(|memory| u32::try_from(memory.bank().fast_len()).ok()),
                    lifetime_memory_count: memory
                        .and_then(|memory| u32::try_from(memory.bank().lifetime_len()).ok()),
                    concept_count: topology_counts.map(|counts| counts.concepts),
                    unresolved_gap_count: topology_counts.map(|counts| counts.unresolved_gaps),
                    learning_active,
                    sleep_phase_raw: Some(sleep.phase.raw()),
                    consolidation_state_raw: Some(sleep.consolidation.kind_raw()),
                    last_consolidated_tick,
                    topology_update_count: Some(summary.topology_updates),
                })
            })
            .collect()
    }

    pub fn request_recovery_sleep(
        &mut self,
        organism_id: OrganismId,
    ) -> Result<SleepTransition, GameAppShellError> {
        let world_tick = self.world.tick();
        let record = self
            .world
            .organism_registry()
            .get(organism_id)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        if !record.lifecycle().is_alive() {
            return Err(ScaffoldContractError::InvalidId.into());
        }
        self.residents
            .get_mut(&organism_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
            .sleep_scheduler
            .force_recovery_sleep(world_tick)
            .map_err(Into::into)
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
        progress: F,
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
        Ok(match self.tick_with_sleep_progress_outcome(progress)? {
            GpuLiveTickOutcome::Progressed(summaries) => summaries,
            GpuLiveTickOutcome::NoProgress(_) => Vec::new(),
        })
    }

    fn tick_with_sleep_progress_outcome<F>(
        &mut self,
        mut progress: F,
    ) -> Result<GpuLiveTickOutcome, GameAppShellError>
    where
        F: FnMut(
            &mut GpuClosedLoopBackend,
            GpuBrainHandle,
            OrganismId,
            SleepState,
            Option<ConsolidationIntent>,
        ) -> SleepProgressResult,
    {
        self.post_promotion_fail_stop_armed = false;
        self.poll_sleep_journal_publication()?;
        if self.exact_checkpoint_waiting_for_sleep_journal {
            return Ok(GpuLiveTickOutcome::NoProgress(
                GpuLiveNoProgressReason::CheckpointPublicationPending,
            ));
        }
        let checkpoint_poll_started = self.performance_measurement_enabled.then(Instant::now);
        self.poll_exact_population_checkpoint()?;
        self.performance_metrics.exact_checkpoint_poll_calls = self
            .performance_metrics
            .exact_checkpoint_poll_calls
            .saturating_add(1);
        self.performance_metrics.exact_checkpoint_poll_wall_ns = self
            .performance_metrics
            .exact_checkpoint_poll_wall_ns
            .saturating_add(checkpoint_poll_started.map_or(0, elapsed_ns));
        self.backend.ensure_neural_actions_available()?;
        if let Some(reason) =
            no_progress_reason_for_checkpoint_stage(self.exact_checkpoint_coordinator.stage())
        {
            return Ok(GpuLiveTickOutcome::NoProgress(reason));
        }
        let world_tick_before = self.world.tick().raw();
        let measure_clone_wall_time = self.performance_measurement_enabled;
        let (result, clone_sample) =
            tick_with_sleep_progress_inner(self, measure_clone_wall_time, |runtime| {
                runtime.tick_with_sleep_progress_staged(&mut progress)
            });
        self.performance_metrics.rollback_clone_calls = self
            .performance_metrics
            .rollback_clone_calls
            .saturating_add(1);
        self.performance_metrics.rollback_world_clone_wall_ns = self
            .performance_metrics
            .rollback_world_clone_wall_ns
            .saturating_add(clone_sample.world_wall_ns);
        self.performance_metrics.rollback_residents_clone_wall_ns = self
            .performance_metrics
            .rollback_residents_clone_wall_ns
            .saturating_add(clone_sample.residents_wall_ns);
        self.performance_metrics.rollback_resident_rows = self
            .performance_metrics
            .rollback_resident_rows
            .saturating_add(clone_sample.resident_rows);
        self.performance_metrics.rollback_world_object_rows = self
            .performance_metrics
            .rollback_world_object_rows
            .saturating_add(clone_sample.world_object_rows);
        if result.is_ok() && self.world.tick().raw() > world_tick_before {
            self.performance_metrics.rollback_clone_progress_calls = self
                .performance_metrics
                .rollback_clone_progress_calls
                .saturating_add(1);
        } else {
            self.performance_metrics.rollback_clone_zero_progress_calls = self
                .performance_metrics
                .rollback_clone_zero_progress_calls
                .saturating_add(1);
        }
        if result.is_err() && self.post_promotion_fail_stop_armed {
            self.backend
                .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
        }
        self.post_promotion_fail_stop_armed = false;
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
        result.map(GpuLiveTickOutcome::Progressed)
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
        let preamble_started = Instant::now();
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
        self.last_sealed_patches
            .retain(|patch| self.handles.contains_key(&patch.header().organism_id.raw()));
        self.restored_replay_patches
            .retain(|patch| self.handles.contains_key(&patch.header().organism_id.raw()));
        self.last_learning_receipts.clear();
        self.last_gpu_authority_receipts.clear();
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
        #[cfg(feature = "gpu-tests")]
        {
            self.last_sleep_memory_compaction_preparation_count = 0;
        }
        if self.handles.is_empty() {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "GPU neural policy requires at least one live organism",
            });
        }

        let tick_before = self.world.tick();
        let tick_after = Tick::new(tick_before.raw().saturating_add(1));
        let checkpoint_active = self.exact_checkpoint_coordinator.is_active();
        // A durable checkpoint permit is organism-scoped. Only the exact
        // Completed states captured in the active immutable save may advance
        // to Committed under this worker. Founders that complete later remain
        // Completed until the one bounded follow-up capture makes their state
        // durable; they must never be folded into the first worker's finalize.
        let durable_completed_sleep_permits = match &self.exact_checkpoint_work {
            ExactPopulationCheckpointRuntimeWorkV1::AwaitingJournal { permit, .. } => {
                permit.validate_restored_provenance()?;
                permit
                    .published()
                    .save
                    .creatures
                    .iter()
                    .filter_map(|creature| {
                        let brain = creature.gpu_brain.as_ref()?;
                        matches!(
                            brain.sleep.consolidation,
                            ConsolidationState::Completed { .. }
                        )
                        .then_some((creature.organism_id.raw(), brain.sleep))
                    })
                    .collect::<BTreeMap<_, _>>()
            }
            _ => BTreeMap::new(),
        };
        let completed_sleep_states = self
            .residents
            .iter()
            .filter_map(|(&raw, resident)| {
                let sleep = resident.sleep_scheduler.state();
                durable_completed_sleep_permits
                    .get(&raw)
                    .is_some_and(|durable| *durable == sleep)
                    .then_some((OrganismId(raw), sleep))
            })
            .collect::<Vec<_>>();
        let mut prepared_memory_commits = BTreeMap::new();
        for (organism_id, completed_sleep) in completed_sleep_states {
            let prepared =
                self.prepare_memory_compaction_at_sleep_commit(organism_id, completed_sleep)?;
            #[cfg(feature = "gpu-tests")]
            {
                self.last_sleep_memory_compaction_preparation_count = self
                    .last_sleep_memory_compaction_preparation_count
                    .saturating_add(1);
            }
            if prepared_memory_commits
                .insert(organism_id.raw(), prepared)
                .is_some()
            {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
            }
        }
        let homeostatic_parameters = self.homeostatic_parameters;
        let mut batch = Vec::with_capacity(self.handles.len());
        let mut summaries_by_organism = BTreeMap::new();
        let mut scheduled_body_events = BTreeMap::new();
        let mut persist_exact_sleep_boundary = false;
        let mut sleep_journal_entries = Vec::new();
        let mut sleep_journal_neural_authority_updates = BTreeMap::new();
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
                    let record = self
                        .world
                        .organism_registry()
                        .get(organism_id)
                        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                    let world_entity_id = record.world_entity_id();
                    let object = self
                        .world
                        .entity(world_entity_id)
                        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                    if object.kind != WorldObjectKind::Agent
                        || object.organism_id != Some(organism_id)
                    {
                        return Err(ScaffoldContractError::BrainOwnershipMismatch);
                    }
                    Ok::<_, ScaffoldContractError>((raw, handle, world_entity_id))
                })
                .collect::<Result<Vec<_>, _>>()?
        };
        let perception_index = self.world.build_perception_batch_index()?;
        self.performance_metrics.tick_preamble_wall_ns = self
            .performance_metrics
            .tick_preamble_wall_ns
            .saturating_add(elapsed_ns(preamble_started));
        let preparation_started = Instant::now();
        let measure_preparation = self.performance_measurement_enabled;
        let mut sleep_eligibility_replay_wall_ns = 0_u64;
        let mut sleep_timing = SleepPreparationTiming::default();
        let mut grounded_perception_wall_ns = 0_u64;
        let mut episodic_retrieval_wall_ns = 0_u64;
        let mut attention_context_wall_ns = 0_u64;
        let mut topology_concept_wall_ns = 0_u64;
        let mut gpu_upload_wall_ns = 0_u64;
        let mut checkpoint_publication_wall_ns = 0_u64;
        for (raw, handle, world_entity_id) in scheduled_handles {
            let sleep_preparation_started = measure_preparation.then(Instant::now);
            let retained_learning_pending =
                self.retry_retained_learning(OrganismId(raw), tick_before)?;
            let mut record = self
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
            let completed_waiting_for_durable_permit = matches!(
                sleep_before.consolidation,
                ConsolidationState::Completed { .. }
            ) && !durable_completed_sleep_permits
                .get(&raw)
                .is_some_and(|durable| *durable == sleep_before);
            let allow_sleep_progress = !completed_waiting_for_durable_permit;
            // Fixed continuous-wake lab protocols suppress sleep phases but
            // keep the production work-cost ledger. Applying the existing
            // sleep-rate recovery prevents ecology energy exhaustion from
            // truncating their bounded neural measurement windows.
            match brain_atp_world_tick_mode(
                phase_before,
                self.schedule_sleep,
                completed_waiting_for_durable_permit,
            ) {
                BrainAtpWorldTickMode::Charge { recover } => {
                    self.backend
                        .charge_world_brain_atp_tick(handle, tick_before.raw(), recover)?;
                }
                BrainAtpWorldTickMode::DurabilityHold => {
                    self.backend
                        .hold_world_brain_atp_tick(handle, tick_before.raw())?;
                }
            }
            if self.schedule_sleep
                && phase_before == SleepPhase::Awake
                && !retained_learning_pending
                && !self.backend.next_bounded_activity_is_affordable(handle)?
            {
                resident.sleep_scheduler.force_recovery_sleep(tick_before)?;
            }
            let sleep_event = if self.schedule_sleep && allow_sleep_progress {
                let sleep_config = sleep_consolidation_config_for(&resident.phenotype)?;
                let mut routed_driver = RoutedGpuSleepDriver {
                    authoritative: AuthoritativeGpuSleepDriver {
                        backend: &mut self.backend,
                        handle,
                        sleep_config: Some(sleep_config),
                        context: Some(AuthoritativeSleepContext {
                            memory: self
                                .memories
                                .get_mut(&raw)
                                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?,
                            predictor: &mut resident.predictor,
                            topology: self
                                .topologies
                                .get_mut(&raw)
                                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?,
                            restored_replay_patches: &self.restored_replay_patches,
                            sealed_patches: &self.sealed_patches,
                            last_sealed_patches: &self.last_sealed_patches,
                        }),
                        replay_evidence_before_commit: None,
                        last_sleep_work: Some(&mut resident.last_sleep_work),
                    },
                    progress,
                    timing: &mut sleep_timing,
                    measure: measure_preparation,
                };
                let event = resident.sleep_scheduler.scheduled_tick_with_organism(
                    &mut record,
                    homeostatic_parameters,
                    tick_before,
                    &mut routed_driver,
                    false,
                )?;
                replace_canonical_organism_record(&mut self.world, record)?;
                if event.sleep_work_units > 0 {
                    let sleep_work = resident
                        .last_sleep_work
                        .as_ref()
                        .ok_or(ScaffoldContractError::MissingPhaseData)?;
                    let cognitive_work = sleep_cognitive_work_receipt(sleep_work)?;
                    resident.last_cognitive_work = cognitive_work;
                    self.last_cognitive_work_receipts.push(cognitive_work);
                    apply_cognitive_work_cost(
                        &mut self.world,
                        OrganismId(raw),
                        cognitive_work,
                        self.cognitive_work_cost_policy,
                    )?;
                }
                event
            } else if !self.schedule_sleep {
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
                    motor_eligible: motor_eligible(SleepPhase::Awake),
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
            } else {
                GpuSleepScheduleEvent {
                    tick: tick_before,
                    phase: phase_before,
                    cycle_id: if sleep_before.active_cycle_id != 0 {
                        sleep_before.active_cycle_id
                    } else {
                        sleep_before.last_consolidated_cycle_id
                    },
                    transition: None,
                    consolidation_kind_raw: sleep_before.consolidation.kind_raw(),
                    selected_action: None,
                    motor_eligible: motor_eligible(phase_before),
                    sleep_work_units: 0,
                    phase_receipt: SleepPhaseReceipt {
                        phase: phase_before,
                        cycle_id: if sleep_before.active_cycle_id != 0 {
                            sleep_before.active_cycle_id
                        } else {
                            sleep_before.last_consolidated_cycle_id
                        },
                        tick: tick_before,
                        due_work: SleepWorkDue::empty(),
                        work_units: 0,
                        cumulative_work_units: 0,
                        sealed: false,
                    },
                }
            };
            let sleep_after = resident.sleep_scheduler.state();
            sleep_eligibility_replay_wall_ns = sleep_eligibility_replay_wall_ns
                .saturating_add(sleep_preparation_started.map_or(0, elapsed_ns));
            if sleep_recovery_body_event_due(phase_before, completed_waiting_for_durable_permit) {
                scheduled_body_events.insert(
                    raw,
                    BodyEventDelta {
                        sleep_recovery: 1.0,
                        ..BodyEventDelta::zero()
                    },
                );
            }
            let checkpoint_preparation_started = measure_preparation.then(Instant::now);
            if sleep_after != sleep_before {
                match (sleep_before.consolidation, sleep_after.consolidation) {
                    (
                        ConsolidationState::Completed { .. },
                        ConsolidationState::Committed { .. },
                    ) => completed_promotions.push((OrganismId(raw), sleep_after)),
                    (
                        ConsolidationState::Submitted { .. },
                        ConsolidationState::Completed { .. },
                    ) => persist_exact_sleep_boundary = true,
                    (ConsolidationState::None, ConsolidationState::Pending { .. })
                    | (ConsolidationState::Pending { .. }, ConsolidationState::Prepared { .. })
                    | (ConsolidationState::Prepared { .. }, ConsolidationState::Submitted { .. })
                    | (
                        ConsolidationState::Committed { .. },
                        ConsolidationState::Committed { .. },
                    )
                    | (ConsolidationState::Committed { .. }, ConsolidationState::None) => {
                        let refresh_authority = matches!(
                            (sleep_before.consolidation, sleep_after.consolidation),
                            (ConsolidationState::None, ConsolidationState::Pending { .. })
                        );
                        if refresh_authority && !checkpoint_active {
                            sleep_journal_neural_authority_updates.insert(
                                raw,
                                capture_sleep_journal_neural_authority(&mut self.backend, handle)?,
                            );
                        } else if !checkpoint_active {
                            if let Some(expected) = sleep_journal_neural_authority_updates
                                .get(&raw)
                                .or_else(|| self.sleep_journal_neural_authorities.get(&raw))
                            {
                                validate_sleep_journal_neural_authority(
                                    &mut self.backend,
                                    handle,
                                    expected,
                                )?;
                            }
                        }
                        if matches!(
                            (sleep_before.consolidation, sleep_after.consolidation),
                            (ConsolidationState::None, ConsolidationState::Pending { .. })
                        ) && sleep_before.phase != SleepPhase::Consolidating
                            && sleep_after.phase == SleepPhase::Consolidating
                        {
                            let intermediate = SleepState {
                                consolidation: ConsolidationState::None,
                                ..sleep_after
                            };
                            sleep_journal_entries.push(
                                GpuSleepTransactionJournalEntryV2::try_new_with_ordinal(
                                    OrganismId(raw),
                                    tick_after,
                                    0,
                                    sleep_before,
                                    intermediate,
                                )?,
                            );
                            sleep_journal_entries.push(
                                GpuSleepTransactionJournalEntryV2::try_new_with_ordinal(
                                    OrganismId(raw),
                                    tick_after,
                                    1,
                                    intermediate,
                                    sleep_after,
                                )?,
                            );
                        } else {
                            sleep_journal_entries.push(GpuSleepTransactionJournalEntryV2::try_new(
                                OrganismId(raw),
                                tick_after,
                                sleep_before,
                                sleep_after,
                            )?);
                        }
                    }
                    (ConsolidationState::None, ConsolidationState::None) => {
                        if sleep_before.phase == SleepPhase::Awake && !checkpoint_active {
                            sleep_journal_neural_authority_updates.insert(
                                raw,
                                capture_sleep_journal_neural_authority(&mut self.backend, handle)?,
                            );
                        } else if !checkpoint_active {
                            if let Some(expected) = sleep_journal_neural_authority_updates
                                .get(&raw)
                                .or_else(|| self.sleep_journal_neural_authorities.get(&raw))
                            {
                                validate_sleep_journal_neural_authority(
                                    &mut self.backend,
                                    handle,
                                    expected,
                                )?;
                            }
                        }
                        sleep_journal_entries.push(GpuSleepTransactionJournalEntryV2::try_new(
                            OrganismId(raw),
                            tick_after,
                            sleep_before,
                            sleep_after,
                        )?);
                    }
                    _ => return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into()),
                }
            }
            checkpoint_publication_wall_ns = checkpoint_publication_wall_ns
                .saturating_add(checkpoint_preparation_started.map_or(0, elapsed_ns));
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
                let grounded_perception_started = measure_preparation.then(Instant::now);
                let organism = self
                    .world
                    .organism_registry()
                    .get(OrganismId(raw))
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                let neural_receptors = organism
                    .biochemistry()
                    .neural_receptor_frame(organism.phenotype())?;
                if neural_receptors.source_tick != tick_before {
                    return Err(ScaffoldContractError::InvalidDecisionEvidence);
                }
                let receptor_phenotype = NeuralReceptorPhenotype::compile(&resident.phenotype)?;
                let receptor_effects =
                    NeuralReceptorEffects::from_frame(&neural_receptors, &receptor_phenotype)?;
                let draft = self.world.perception_frame_draft_indexed(
                    OrganismId(raw),
                    tick_before,
                    self.sensor_profile,
                    resident.homeostasis,
                    &perception_index,
                )?;
                grounded_perception_wall_ns = grounded_perception_wall_ns
                    .saturating_add(grounded_perception_started.map_or(0, elapsed_ns));
                let episodic_retrieval_started = measure_preparation.then(Instant::now);
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
                let (baseline_frame, baseline_recall) =
                    baseline_prepared.finalize(draft.clone())?;
                baseline_recall.validate_for_frame(&baseline_frame)?;
                let memory_evidence = finalized_memory_attention_evidence(&baseline_recall)?;
                episodic_retrieval_wall_ns = episodic_retrieval_wall_ns
                    .saturating_add(episodic_retrieval_started.map_or(0, elapsed_ns));
                let attention_context_started = measure_preparation.then(Instant::now);
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
                    receptor_effects,
                )?;
                let attention = select_focal_targets(
                    OrganismId(raw),
                    sequence_id,
                    tick_before,
                    &peripheral_summaries,
                    resident.attention_hysteresis,
                    attention_selection_policy_for(&resident.phenotype),
                )?;
                resident.attention_hysteresis = attention.hysteresis;
                let routed_draft = route_focal_candidates(draft, &attention)?;
                attention_context_wall_ns = attention_context_wall_ns
                    .saturating_add(attention_context_started.map_or(0, elapsed_ns));
                let topology_concept_started = measure_preparation.then(Instant::now);
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
                topology_concept_wall_ns = topology_concept_wall_ns
                    .saturating_add(topology_concept_started.map_or(0, elapsed_ns));
                let gpu_upload_started = measure_preparation.then(Instant::now);
                let memory_upload = self
                    .backend
                    .prepare_memory_context_upload(handle, &frame, &memory_recall)?
                    .bind_neural_receptor_effects(receptor_effects)
                    .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
                gpu_upload_wall_ns =
                    gpu_upload_wall_ns.saturating_add(gpu_upload_started.map_or(0, elapsed_ns));
                Ok(PreparedGpuBrainFrame {
                    handle,
                    world_entity_id,
                    frame,
                    memory_recall,
                    memory_upload,
                    neural_receptors,
                    receptor_effects,
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
        self.performance_metrics
            .perception_sleep_preparation_wall_ns = self
            .performance_metrics
            .perception_sleep_preparation_wall_ns
            .saturating_add(elapsed_ns(preparation_started));
        self.performance_metrics
            .preparation_sleep_eligibility_replay_wall_ns = self
            .performance_metrics
            .preparation_sleep_eligibility_replay_wall_ns
            .saturating_add(sleep_eligibility_replay_wall_ns);
        self.performance_metrics
            .preparation_sleep_phase_data_wall_ns = self
            .performance_metrics
            .preparation_sleep_phase_data_wall_ns
            .saturating_add(sleep_timing.phase_data_wall_ns);
        self.performance_metrics
            .preparation_sleep_replay_progress_wall_ns = self
            .performance_metrics
            .preparation_sleep_replay_progress_wall_ns
            .saturating_add(sleep_timing.replay_progress_wall_ns);
        self.performance_metrics
            .preparation_sleep_consolidation_wall_ns = self
            .performance_metrics
            .preparation_sleep_consolidation_wall_ns
            .saturating_add(sleep_timing.consolidation_wall_ns);
        self.performance_metrics
            .preparation_grounded_perception_wall_ns = self
            .performance_metrics
            .preparation_grounded_perception_wall_ns
            .saturating_add(grounded_perception_wall_ns);
        self.performance_metrics
            .preparation_episodic_retrieval_wall_ns = self
            .performance_metrics
            .preparation_episodic_retrieval_wall_ns
            .saturating_add(episodic_retrieval_wall_ns);
        self.performance_metrics
            .preparation_attention_context_wall_ns = self
            .performance_metrics
            .preparation_attention_context_wall_ns
            .saturating_add(attention_context_wall_ns);
        self.performance_metrics
            .preparation_topology_concept_wall_ns = self
            .performance_metrics
            .preparation_topology_concept_wall_ns
            .saturating_add(topology_concept_wall_ns);
        self.performance_metrics.preparation_gpu_upload_wall_ns = self
            .performance_metrics
            .preparation_gpu_upload_wall_ns
            .saturating_add(gpu_upload_wall_ns);
        self.performance_metrics
            .preparation_checkpoint_publication_wall_ns = self
            .performance_metrics
            .preparation_checkpoint_publication_wall_ns
            .saturating_add(checkpoint_publication_wall_ns);

        // The exact worker must receive every journal consequence from this
        // canonical tick before Completed promotion grants it permission to
        // finalize. Entries newer than the captured tick are consumed by the
        // coordinator's single bounded follow-up checkpoint request.
        if !completed_promotions.is_empty()
            && self.exact_checkpoint_accepts_journal_entries()
            && !sleep_journal_entries.is_empty()
        {
            self.queue_exact_checkpoint_journal_entries(sleep_journal_entries.clone())?;
            sleep_journal_entries.clear();
        }

        // The GPU selector has already committed, while the world is still at
        // the exact tick named by the durable Completed checkpoint. Publish
        // the manifest-side selector/ref promotion before any world action or
        // subsequent poll can occur.
        let sleep_promotion_started = Instant::now();
        let mut memory_commits = Vec::with_capacity(completed_promotions.len());
        for (organism_id, committed_sleep) in &completed_promotions {
            let committed_cycle_id = match committed_sleep.consolidation {
                ConsolidationState::Committed { cycle_id, .. } if cycle_id != 0 => cycle_id,
                _ => return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into()),
            };
            let (memory, receipt) = prepared_memory_commits
                .remove(&organism_id.raw())
                .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
            if receipt.identity.organism_id_raw != organism_id.raw()
                || receipt.identity.cycle_id != committed_cycle_id
            {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
            }
            memory_commits.push((*organism_id, memory, receipt));
        }
        if let Err(error) = self.promote_durable_completed_sleep_batch(&completed_promotions) {
            // The backend's Completed -> Committed transaction precedes the
            // manifest CAS. The staged tick wrapper restores world and host
            // sleep authority to Completed; GPU authority must fail-stop
            // because that committed device state cannot be rolled back.
            self.backend
                .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
            return Err(error);
        }
        self.post_promotion_fail_stop_armed = !completed_promotions.is_empty();
        for (organism_id, memory, receipt) in memory_commits {
            let previous = self.memories.insert(organism_id.raw(), memory);
            debug_assert!(previous.is_some());
            self.last_memory_compaction_receipts.push(receipt);
            self.restored_replay_patches
                .retain(|patch| patch.header().organism_id != organism_id);
        }
        self.performance_metrics.sleep_promotion_wall_ns = self
            .performance_metrics
            .sleep_promotion_wall_ns
            .saturating_add(elapsed_ns(sleep_promotion_started));

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
            let inference_rows = u64::try_from(batch.len()).unwrap_or(u64::MAX);
            let inference_started = Instant::now();
            let gpu_ticks = self.backend.tick_memory_batch(&memory_batch)?;
            self.performance_metrics.inference_batches =
                self.performance_metrics.inference_batches.saturating_add(1);
            self.performance_metrics.inference_rows = self
                .performance_metrics
                .inference_rows
                .saturating_add(inference_rows);
            self.performance_metrics.inference_transaction_wall_ns = self
                .performance_metrics
                .inference_transaction_wall_ns
                .saturating_add(
                    u64::try_from(inference_started.elapsed().as_nanos()).unwrap_or(u64::MAX),
                );
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
        #[cfg(any(test, feature = "gpu-tests"))]
        if std::mem::take(&mut self.forced_late_advance_failure) {
            return Err(ScaffoldContractError::NonMonotonicTick.into());
        }
        let authority_timing = advance_and_synchronize_authority(
            &mut self.world,
            &mut self.residents,
            tick_after,
            &scheduled_body_events,
        )?;
        self.performance_metrics.world_authority_advance_wall_ns = self
            .performance_metrics
            .world_authority_advance_wall_ns
            .saturating_add(authority_timing.world_advance_ns);
        self.performance_metrics.resident_synchronize_wall_ns = self
            .performance_metrics
            .resident_synchronize_wall_ns
            .saturating_add(authority_timing.resident_synchronize_ns);
        let passive_observation_started = Instant::now();
        self.observe_passive_tick(tick_before, tick_after)?;
        self.performance_metrics.passive_observation_wall_ns = self
            .performance_metrics
            .passive_observation_wall_ns
            .saturating_add(elapsed_ns(passive_observation_started));
        let population_reconcile_started = Instant::now();
        self.reconcile_population()?;
        self.performance_metrics.population_reconcile_wall_ns = self
            .performance_metrics
            .population_reconcile_wall_ns
            .saturating_add(elapsed_ns(population_reconcile_started));
        if persist_exact_sleep_boundary {
            let sleep_persistence_started = Instant::now();
            self.request_exact_population_checkpoint()?;
            if self.exact_checkpoint_accepts_journal_entries()
                && !sleep_journal_entries.is_empty()
            {
                self.queue_exact_checkpoint_journal_entries(sleep_journal_entries)?;
            } else if !sleep_journal_entries.is_empty() {
                self.sleep_journal_neural_authorities
                    .extend(sleep_journal_neural_authority_updates);
                let enqueue_started = self.performance_measurement_enabled.then(Instant::now);
                self.start_sleep_journal_publication(sleep_journal_entries)?;
                self.performance_metrics
                    .sleep_journal_update_thread_enqueue_wall_ns = self
                    .performance_metrics
                    .sleep_journal_update_thread_enqueue_wall_ns
                    .saturating_add(enqueue_started.map_or(0, elapsed_ns));
            }
            self.performance_metrics.sleep_persistence_wall_ns = self
                .performance_metrics
                .sleep_persistence_wall_ns
                .saturating_add(elapsed_ns(sleep_persistence_started));
        } else if !sleep_journal_entries.is_empty() {
            if self.exact_checkpoint_accepts_journal_entries() {
                self.queue_exact_checkpoint_journal_entries(sleep_journal_entries)?;
                return Ok(summaries_by_organism.into_values().collect());
            }
            let sleep_persistence_started = Instant::now();
            let durability = self.checkpoint_durability.take();
            let validation_result = (|| -> Result<(), GameAppShellError> {
                for entry in &sleep_journal_entries {
                    let raw = entry.organism_id.raw();
                    if sleep_journal_neural_authority_updates.contains_key(&raw)
                        || self.sleep_journal_neural_authorities.contains_key(&raw)
                    {
                        continue;
                    }
                    let durability = durability
                        .as_ref()
                        .ok_or(ScaffoldContractError::MissingPhaseData)?;
                    let handle = *self
                        .handles
                        .get(&raw)
                        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                    let resident = self
                        .residents
                        .get(&raw)
                        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                    let exact_base = durability
                        .published
                        .save
                        .creatures
                        .iter()
                        .find(|creature| creature.organism_id == entry.organism_id)
                        .and_then(|creature| creature.gpu_brain.as_ref())
                        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
                    durability.store.validate_compact_neural_reuse(
                        &mut self.backend,
                        &durability.published.save.assets,
                        exact_base,
                        handle,
                        &resident.phenotype,
                    )?;
                    sleep_journal_neural_authority_updates.insert(
                        raw,
                        capture_sleep_journal_neural_authority(&mut self.backend, handle)?,
                    );
                }
                Ok(())
            })();
            if let Err(error) = validation_result {
                self.checkpoint_durability = durability;
                return Err(error);
            }
            self.performance_metrics.sleep_persistence_calls = self
                .performance_metrics
                .sleep_persistence_calls
                .saturating_add(1);
            self.checkpoint_durability = durability;
            self.sleep_journal_neural_authorities
                .extend(sleep_journal_neural_authority_updates);
            let enqueue_started = self.performance_measurement_enabled.then(Instant::now);
            self.start_sleep_journal_publication(sleep_journal_entries)?;
            self.performance_metrics
                .sleep_journal_update_thread_enqueue_wall_ns = self
                .performance_metrics
                .sleep_journal_update_thread_enqueue_wall_ns
                .saturating_add(enqueue_started.map_or(0, elapsed_ns));
            self.performance_metrics.sleep_persistence_wall_ns = self
                .performance_metrics
                .sleep_persistence_wall_ns
                .saturating_add(elapsed_ns(sleep_persistence_started));
        }
        Ok(summaries_by_organism.into_values().collect())
    }

    /// Shared neural-session authority used by gameplay and laboratory hosts.
    pub const fn session_authority(&self) -> &GpuSessionAuthority {
        self.backend.authority()
    }

    pub fn request_manual_checkpoint(
        &mut self,
        destination: PathBuf,
    ) -> Result<GpuManualCheckpointRequestDisposition, GameAppShellError> {
        if destination.as_os_str().is_empty() {
            return Err(ScaffoldContractError::InvalidId.into());
        }
        if self.sleep_journal_publication_worker.is_some()
            || !self.pending_sleep_journal_entries.is_empty()
        {
            let checkpoint_tick = self.world.tick();
            if let Some(pending) = &self.manual_checkpoint_waiting_for_sleep_journal {
                return if pending == &destination {
                    Ok(GpuManualCheckpointRequestDisposition::Coalesced)
                } else {
                    Err(ScaffoldContractError::ConsolidationGenerationMismatch.into())
                };
            }
            self.exact_checkpoint_waiting_for_sleep_journal = true;
            self.manual_checkpoint_waiting_for_sleep_journal = Some(destination.clone());
            self.manual_checkpoint_status = GpuManualCheckpointStatus::Queued {
                destination,
                checkpoint_tick,
            };
            return Ok(GpuManualCheckpointRequestDisposition::Queued);
        }
        if !self.exact_checkpoint_coordinator.is_active() {
            self.request_exact_population_checkpoint()?;
        }
        let checkpoint_tick = self
            .exact_checkpoint_coordinator
            .active_identity()
            .map(|identity| identity.checkpoint_tick)
            .ok_or(ScaffoldContractError::MissingPhaseData)?;
        let disposition =
            self.exact_checkpoint_coordinator
                .request_manual(ManualCheckpointRequestV1 {
                    checkpoint_tick,
                    destination: destination.clone(),
                });
        match disposition {
            ExactCheckpointRequestDispositionV1::ManualQueued => {
                self.manual_checkpoint_status = GpuManualCheckpointStatus::Queued {
                    destination,
                    checkpoint_tick,
                };
                Ok(GpuManualCheckpointRequestDisposition::Queued)
            }
            ExactCheckpointRequestDispositionV1::ManualCoalesced => {
                self.manual_checkpoint_status = GpuManualCheckpointStatus::Queued {
                    destination,
                    checkpoint_tick,
                };
                Ok(GpuManualCheckpointRequestDisposition::Coalesced)
            }
            ExactCheckpointRequestDispositionV1::Busy => {
                Err(ScaffoldContractError::ConsolidationGenerationMismatch.into())
            }
            _ => Err(ScaffoldContractError::ConsolidationGenerationMismatch.into()),
        }
    }

    pub const fn manual_checkpoint_status(&self) -> &GpuManualCheckpointStatus {
        &self.manual_checkpoint_status
    }

    pub(crate) fn live_save_authority_view(
        &self,
    ) -> Result<LiveRuntimeSaveAuthorityView, GameAppShellError> {
        let save_id = self
            .canonical_save_id
            .clone()
            .ok_or(ScaffoldContractError::MissingPhaseData)?;
        let mut organism_ids = self
            .world
            .organism_registry()
            .iter()
            .map(|record| record.organism_id())
            .collect::<Vec<_>>();
        organism_ids.sort_unstable_by_key(|organism_id| organism_id.raw());
        let resident_ids = self
            .handles
            .keys()
            .copied()
            .map(OrganismId)
            .collect::<Vec<_>>();
        if organism_ids != resident_ids {
            return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
        }
        for memory in self.memories.values() {
            if memory.profile().profile()? != self.sensor_profile {
                return Err(ScaffoldContractError::SensorProfileMismatch.into());
            }
        }
        Ok(LiveRuntimeSaveAuthorityView {
            save_id,
            deterministic_seed: self.deterministic_seed,
            sensor_profile: self.sensor_profile,
            organism_ids,
        })
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

    /// Places one bounded food resource through canonical world authority.
    /// The candidate world is fully validated before it replaces live state.
    pub fn place_player_food(
        &mut self,
        position: Vec3f,
    ) -> Result<PlayerResourcePlacementReceipt, GameAppShellError> {
        let request = PlayerResourcePlacementRequest::new(position);
        request.validate()?;

        let config = WorldEditorConfig::default();
        if self.world.object_count() >= config.max_objects {
            return Err(ScaffoldContractError::ScalarOutOfRange.into());
        }
        let label = format!(
            "player-food-t{}-x{:08x}-z{:08x}",
            self.world.tick().raw(),
            position.x.to_bits(),
            position.z.to_bits()
        );
        let command = WorldEditCommand::place_food(&label, position, PLAYER_FOOD_NUTRITION);
        command.validate(config)?;

        let mut candidate = self.world.clone();
        let world_entity_id = candidate.editor_spawn_object(WorldEditorSpawnSpec {
            label: label.clone(),
            kind: WorldObjectKind::Food,
            organism_id: None,
            position,
            nutrition: PLAYER_FOOD_NUTRITION,
            hazard_pain: 0.0,
            radius: PLAYER_FOOD_RADIUS,
            token_id: None,
        })?;
        candidate.validate_organism_bindings()?;
        let world_signature = candidate.canonical_signature_digest()?;
        self.world = candidate;

        Ok(PlayerResourcePlacementReceipt {
            schema_version: PLAYER_RESOURCE_PLACEMENT_SCHEMA_VERSION,
            world_entity_id,
            label,
            position,
            nutrition: PLAYER_FOOD_NUTRITION,
            radius: PLAYER_FOOD_RADIUS,
            world_signature,
        })
    }

    pub fn residency_summary(&self) -> GpuLiveResidencySummary {
        GpuLiveResidencySummary {
            handle_count: self.handles.len(),
            resident_count: self.residents.len(),
            memory_sidecar_count: self.memories.len(),
            topology_sidecar_count: self.topologies.len(),
        }
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
        let authority = world.habitat_authority().clone();
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

    /// Executes an authorized school action against the live learner and GPU
    /// cognition. The teacher cue is ordinary spatial perception, and the
    /// resulting sealed tick is published by the existing production loop.
    pub fn execute_structured_education(
        &mut self,
        receipt: HabitatPermissionReceipt,
    ) -> Result<Vec<LiveBrainTickSummary>, GameAppShellError> {
        let expected = self
            .authorize_structured_education(receipt.organism_id, receipt.habitat_id, receipt.actor)
            .map_err(|error| GameAppShellError::InvalidProductionFrontend {
                message: format!(
                    "structured education receipt rejected by the live habitat authority: {error}"
                ),
            })?;
        if receipt != expected || receipt.tick != self.world.tick() {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message:
                    "structured education receipt is stale or does not match the live authority"
                        .to_string(),
            });
        }
        if !self.residents.contains_key(&receipt.organism_id.raw()) {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: format!(
                    "structured education learner {} is not admitted to live GPU cognition",
                    receipt.organism_id.raw()
                ),
            });
        }
        let sensory = self
            .world
            .sensory_report(receipt.organism_id, receipt.tick)?;
        if sensory
            .core_snapshot
            .language_context
            .teacher_channel_marker
            .is_none()
        {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "structured education requires a grounded teacher perception".to_string(),
            });
        }

        let summaries = self.tick()?;
        let learner_summary = summaries
            .iter()
            .find(|summary| summary.organism_id == receipt.organism_id)
            .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
                message: format!(
                    "structured education learner {} did not receive a live GPU summary",
                    receipt.organism_id.raw()
                ),
            })?;
        if !learner_summary.patch_sealed {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: format!(
                    "structured education learner {} did not seal a live experience patch",
                    receipt.organism_id.raw()
                ),
            });
        }
        Ok(summaries)
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

    /// Backend-issued compact authority receipts from the most recent seal.
    pub fn last_gpu_authority_receipts(&self) -> &[GpuAuthorityReceiptV1] {
        &self.last_gpu_authority_receipts
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

    fn record_sleep_journal_publication_timing(
        &mut self,
        timing: GpuLiveSleepJournalPublicationTiming,
    ) {
        macro_rules! add {
            ($field:ident, $value:expr) => {
                self.performance_metrics.$field =
                    self.performance_metrics.$field.saturating_add($value)
            };
        }
        add!(
            sleep_journal_current_load_validation_wall_ns,
            timing.current_journal_load_validation_wall_ns
        );
        add!(sleep_journal_merge_wall_ns, timing.merge_wall_ns);
        add!(sleep_journal_sort_wall_ns, timing.sort_wall_ns);
        add!(
            sleep_journal_build_validation_wall_ns,
            timing.journal_build_validation_wall_ns
        );
        add!(
            sleep_journal_input_validation_wall_ns,
            timing.durable.input_validation_wall_ns
        );
        add!(
            sleep_journal_cas_lock_wait_wall_ns,
            timing.durable.cas_lock_wait_wall_ns
        );
        add!(
            sleep_journal_cas_base_reload_wall_ns,
            timing.durable.cas_base_reload_wall_ns
        );
        add!(
            sleep_journal_save_encode_wall_ns,
            timing.durable.save_encode_wall_ns
        );
        add!(
            sleep_journal_save_artifact_write_wall_ns,
            timing.durable.save_artifact_write_wall_ns
        );
        add!(
            sleep_journal_encode_wall_ns,
            timing.durable.journal_encode_wall_ns
        );
        add!(
            sleep_journal_artifact_write_wall_ns,
            timing.durable.journal_artifact_write_wall_ns
        );
        add!(
            sleep_journal_pointer_build_validation_wall_ns,
            timing.durable.pointer_build_validation_wall_ns
        );
        add!(
            sleep_journal_prepared_reload_validation_wall_ns,
            timing.durable.prepared_artifact_reload_validation_wall_ns
        );
        add!(
            sleep_journal_manifest_encode_wall_ns,
            timing.durable.manifest_encode_wall_ns
        );
        add!(
            sleep_journal_manifest_write_wall_ns,
            timing.durable.manifest_write_wall_ns
        );
        add!(
            sleep_journal_manifest_reload_validation_wall_ns,
            timing.durable.manifest_reload_validation_wall_ns
        );
        add!(
            sleep_journal_final_reload_validation_wall_ns,
            timing.durable.final_journal_reload_validation_wall_ns
        );
        add!(
            sleep_journal_outer_manifest_reload_validation_wall_ns,
            timing.outer_manifest_reload_validation_wall_ns
        );
        add!(
            sleep_journal_outer_reload_validation_wall_ns,
            timing.outer_journal_reload_validation_wall_ns
        );
    }

    pub const fn performance_metrics(&self) -> GpuLivePerformanceMetrics {
        self.performance_metrics
    }

    pub(crate) fn exact_checkpoint_performance_state(&self) -> ExactCheckpointPerformanceState {
        let identity = self.exact_checkpoint_coordinator.active_identity();
        let stage = match self.exact_checkpoint_coordinator.stage() {
            ExactPopulationCheckpointStageV1::Idle => "idle",
            ExactPopulationCheckpointStageV1::CaptureSubmitted => "capture_submitted",
            ExactPopulationCheckpointStageV1::MappingPending => "mapping_pending",
            ExactPopulationCheckpointStageV1::CpuBytesReady => "cpu_bytes_ready",
            ExactPopulationCheckpointStageV1::Encoding => "encoding",
            ExactPopulationCheckpointStageV1::ManifestPrepared => "manifest_prepared",
            ExactPopulationCheckpointStageV1::CasCommitted => "cas_committed",
            ExactPopulationCheckpointStageV1::ReloadValidated => "reload_validated",
            ExactPopulationCheckpointStageV1::DurablePermitInstalled => "durable_permit_installed",
            ExactPopulationCheckpointStageV1::DeferredJournalPublishing => {
                "deferred_journal_publishing"
            }
            ExactPopulationCheckpointStageV1::Complete => "complete",
            ExactPopulationCheckpointStageV1::Failed => "failed",
        };
        let worker_status = match &self.exact_checkpoint_work {
            ExactPopulationCheckpointRuntimeWorkV1::Idle => "idle",
            ExactPopulationCheckpointRuntimeWorkV1::Capture { .. } => "capture",
            ExactPopulationCheckpointRuntimeWorkV1::CaptureFailed { .. } => "capture_failed",
            ExactPopulationCheckpointRuntimeWorkV1::Worker { .. } => "worker",
            ExactPopulationCheckpointRuntimeWorkV1::CommitWorker { .. } => "commit_worker",
            ExactPopulationCheckpointRuntimeWorkV1::AwaitingJournal { .. } => "awaiting_journal",
            ExactPopulationCheckpointRuntimeWorkV1::JournalWorker { .. } => "journal_worker",
            ExactPopulationCheckpointRuntimeWorkV1::Finalizing { .. } => "finalizing",
            ExactPopulationCheckpointRuntimeWorkV1::FailedJoining { .. } => "failed_joining",
            ExactPopulationCheckpointRuntimeWorkV1::Failed => "failed",
        };
        ExactCheckpointPerformanceState {
            transaction_id: identity.map(|identity| identity.transaction_id),
            checkpoint_tick: identity.map(|identity| identity.checkpoint_tick.raw()),
            stage,
            worker_status,
        }
    }

    pub fn set_performance_measurement_enabled(&mut self, enabled: bool) {
        self.performance_measurement_enabled = enabled;
        self.exact_checkpoint_transaction_started_at =
            if enabled && self.exact_checkpoint_coordinator.is_active() {
                Some(Instant::now())
            } else {
                None
            };
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
        if !gpu_ticks.is_empty() {
            self.performance_metrics.selection_readback_calls = self
                .performance_metrics
                .selection_readback_calls
                .saturating_add(1);
            self.performance_metrics.selection_readback_bytes = self
                .performance_metrics
                .selection_readback_bytes
                .saturating_add(u64::try_from(selection_readback_bytes).unwrap_or(u64::MAX));
        }
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
        let selection_prepare_started = Instant::now();
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
        self.performance_metrics.selection_prepare_wall_ns = self
            .performance_metrics
            .selection_prepare_wall_ns
            .saturating_add(elapsed_ns(selection_prepare_started));

        self.last_memory_recall_receipts.extend(
            prepared
                .iter()
                .map(|selection| selection.memory_recall.receipt().clone()),
        );

        let seal_started = Instant::now();
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
        self.performance_metrics
            .seal_world_body_biochemistry_wall_ns = self
            .performance_metrics
            .seal_world_body_biochemistry_wall_ns
            .saturating_add(elapsed_ns(seal_started));
        let commit_started = Instant::now();
        let result = self.commit_sealed_batch(sealed);
        self.performance_metrics.sealed_commit_total_wall_ns = self
            .performance_metrics
            .sealed_commit_total_wall_ns
            .saturating_add(elapsed_ns(commit_started));
        result
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
            neural_receptors,
            receptor_effects,
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
            || neural_receptors.source_tick != frame.tick()
            || receptor_effects.source_tick != frame.tick()
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
            v11_work: gpu_tick.v11_work,
            cognitive_context_digest,
            sequence_id,
            outcome_tick,
            pre_action,
            decision,
            motor_bundle,
            speech_payload: gpu_tick.speech_payload,
            speech_prompted,
            neural_receptors,
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
            v11_work,
            cognitive_context_digest,
            sequence_id,
            outcome_tick,
            pre_action,
            decision,
            motor_bundle,
            speech_payload,
            speech_prompted,
            neural_receptors,
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
                v11_work,
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
            neural_receptors,
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
            .map(|selection| {
                (
                    selection.handle,
                    &selection.patch,
                    &selection.neural_receptors,
                )
            })
            .collect::<Vec<_>>();
        let learning_started = Instant::now();
        let learning_result = self.backend.apply_sealed_outcome_batch(&learning_batch);
        self.performance_metrics.learning_batches =
            self.performance_metrics.learning_batches.saturating_add(1);
        self.performance_metrics.learning_rows = self
            .performance_metrics
            .learning_rows
            .saturating_add(u64::try_from(learning_batch.len()).unwrap_or(u64::MAX));
        self.performance_metrics.learning_transaction_wall_ns = self
            .performance_metrics
            .learning_transaction_wall_ns
            .saturating_add(
                u64::try_from(learning_started.elapsed().as_nanos()).unwrap_or(u64::MAX),
            );
        let learning = match learning_result {
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
                                neural_receptors: selection.neural_receptors.clone(),
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
                                neural_receptors: selection.neural_receptors.clone(),
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
            self.performance_metrics.learning_readback_calls = self
                .performance_metrics
                .learning_readback_calls
                .saturating_add(1);
            self.performance_metrics.learning_readback_bytes = self
                .performance_metrics
                .learning_readback_bytes
                .saturating_add(u64::try_from(learning_readback).unwrap_or(u64::MAX));
        }

        let (memory_updates, topology_updates) = if self.observe_sidecars {
            let memory_started = Instant::now();
            let memory_updates = self.observe_sealed_memory(&sealed);
            self.performance_metrics.sidecar_memory_wall_ns = self
                .performance_metrics
                .sidecar_memory_wall_ns
                .saturating_add(elapsed_ns(memory_started));
            let topology_started = Instant::now();
            let topology_updates = self.observe_sealed_topology(&sealed);
            self.performance_metrics.sidecar_topology_wall_ns = self
                .performance_metrics
                .sidecar_topology_wall_ns
                .saturating_add(elapsed_ns(topology_started));
            (memory_updates, topology_updates)
        } else {
            (vec![false; sealed.len()], vec![false; sealed.len()])
        };

        let cognitive_authority_seal_started = Instant::now();
        let mut authority_receipts = Vec::with_capacity(sealed.len());
        for (index, selection) in sealed.iter().enumerate() {
            let organism_id = selection.handle.organism_id();
            let hash_started = Instant::now();
            let authority_receipt = self.backend.authority_receipt_for_sealed_outcome(
                selection.handle,
                &selection.pending_eligibility,
                learning.as_ref().and_then(|receipts| receipts.get(index)),
                &selection.patch,
            )?;
            authority_receipt.validate()?;
            let topology_digest = self
                .topologies
                .get(&organism_id.raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                .diagnostics()
                .canonical_digest;
            let mut brain_authority = CanonicalDigestBuilder::new(b"alife.live-brain-authority.v4");
            for word in authority_receipt.receipt_digest() {
                brain_authority.write_u64(word);
            }
            for word in topology_digest {
                brain_authority.write_u64(word);
            }
            let brain_digest = brain_authority.finish256();
            authority_receipts.push(authority_receipt);
            self.performance_metrics.state_reference_hash_calls = self
                .performance_metrics
                .state_reference_hash_calls
                .saturating_add(1);
            self.performance_metrics.state_reference_hash_wall_ns = self
                .performance_metrics
                .state_reference_hash_wall_ns
                .saturating_add(elapsed_ns(hash_started));
            let memory_digest = self
                .memories
                .get(&organism_id.raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                .compaction_checkpoint()
                .active_digest;
            let mut record = self
                .world
                .organism_registry()
                .get(organism_id)
                .cloned()
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            record.seal_cognitive_subsystems(
                selection.patch.outcome().outcome_tick,
                brain_digest,
                memory_digest,
            )?;
            replace_canonical_organism_record(&mut self.world, record)?;
        }
        self.last_gpu_authority_receipts.extend(authority_receipts);
        self.performance_metrics.cognitive_authority_seal_wall_ns = self
            .performance_metrics
            .cognitive_authority_seal_wall_ns
            .saturating_add(elapsed_ns(cognitive_authority_seal_started));

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
            self.sealed_patches
                .extend(committed_patches.iter().cloned());
        }
        for patch in committed_patches {
            let organism_id = patch.header().organism_id;
            if let Some(previous) = self
                .last_sealed_patches
                .iter_mut()
                .find(|previous| previous.header().organism_id == organism_id)
            {
                *previous = patch;
            } else {
                self.last_sealed_patches.push(patch);
            }
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
                    let receipt = sidecar.observe_sealed_patch(&selection.patch);
                    let _lifecycle_result =
                        sidecar.advance_lifecycle(selection.patch.outcome().outcome_tick);
                    TopologyObservationDisposition::Observed(Box::new(receipt))
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
    pub fn brain_atp_q16_for_test(
        &self,
        organism_id: OrganismId,
    ) -> Result<u32, ScaffoldContractError> {
        let handle = self.evidence_handle(organism_id)?;
        self.backend.brain_atp_q16(handle)
    }

    #[cfg(feature = "gpu-tests")]
    pub fn set_brain_atp_q16_for_test(
        &mut self,
        organism_id: OrganismId,
        brain_atp_q16: u32,
    ) -> Result<(), ScaffoldContractError> {
        let handle = self.evidence_handle(organism_id)?;
        self.backend
            .set_brain_atp_q16_for_test(handle, brain_atp_q16)
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
    pub fn force_exact_checkpoint_pre_worker_transition_failure_for_test(
        &mut self,
    ) -> Result<(), GameAppShellError> {
        self.request_exact_population_checkpoint()?;
        self.exact_checkpoint_coordinator
            .force_pre_worker_transition_failure_for_test();
        Ok(())
    }

    #[cfg(feature = "gpu-tests")]
    pub fn force_exact_checkpoint_permit_prevalidation_failure_for_test(
        &mut self,
    ) -> Result<(), GameAppShellError> {
        self.backend
            .note_durable_checkpoint(DurableGpuCheckpointRef::try_new(
                Tick::new(u64::MAX),
                "fnv1a64:ffffffffffffffff".to_string(),
                [u64::MAX; 4],
            )?)?;
        self.request_exact_population_checkpoint()
    }

    #[cfg(feature = "gpu-tests")]
    pub fn exact_checkpoint_failed_for_test(&self) -> bool {
        self.exact_checkpoint_coordinator.stage() == ExactPopulationCheckpointStageV1::Failed
            && !matches!(
                self.exact_checkpoint_work,
                ExactPopulationCheckpointRuntimeWorkV1::Idle
            )
    }

    #[cfg(feature = "gpu-tests")]
    pub fn request_exact_checkpoint_for_test(&mut self) -> Result<(), GameAppShellError> {
        self.request_exact_population_checkpoint()
    }

    #[cfg(feature = "gpu-tests")]
    pub fn poll_exact_checkpoint_for_test(&mut self) -> Result<(), GameAppShellError> {
        self.poll_exact_population_checkpoint()
    }

    #[cfg(feature = "gpu-tests")]
    pub fn force_stranded_exact_journal_wait_for_test(&mut self) {
        assert!(self.sleep_journal_publication_worker.is_none());
        assert!(self.pending_sleep_journal_entries.is_empty());
        assert!(!self.exact_checkpoint_coordinator.is_active());
        self.exact_checkpoint_waiting_for_sleep_journal = true;
    }

    #[cfg(feature = "gpu-tests")]
    pub fn poll_persistence_for_shutdown_for_test(&mut self) -> Result<(), GameAppShellError> {
        self.poll_persistence_for_shutdown()
    }

    #[cfg(feature = "gpu-tests")]
    pub fn persistence_idle_for_shutdown_for_test(&self) -> bool {
        self.persistence_idle_for_shutdown()
    }

    #[cfg(feature = "gpu-tests")]
    pub fn exact_checkpoint_active_tick_for_test(&self) -> Option<Tick> {
        self.exact_checkpoint_coordinator
            .active_identity()
            .map(|identity| identity.checkpoint_tick)
    }

    #[cfg(feature = "gpu-tests")]
    pub fn exact_checkpoint_state_for_test(&self) -> (String, &'static str) {
        let work = match self.exact_checkpoint_work {
            ExactPopulationCheckpointRuntimeWorkV1::Idle => "Idle",
            ExactPopulationCheckpointRuntimeWorkV1::Capture { .. } => "Capture",
            ExactPopulationCheckpointRuntimeWorkV1::CaptureFailed { .. } => "CaptureFailed",
            ExactPopulationCheckpointRuntimeWorkV1::Worker { .. } => "Worker",
            ExactPopulationCheckpointRuntimeWorkV1::CommitWorker { .. } => "CommitWorker",
            ExactPopulationCheckpointRuntimeWorkV1::AwaitingJournal { .. } => "AwaitingJournal",
            ExactPopulationCheckpointRuntimeWorkV1::JournalWorker { .. } => "JournalWorker",
            ExactPopulationCheckpointRuntimeWorkV1::Finalizing { .. } => "Finalizing",
            ExactPopulationCheckpointRuntimeWorkV1::FailedJoining { .. } => "FailedJoining",
            ExactPopulationCheckpointRuntimeWorkV1::Failed => "Failed",
        };
        (
            format!("{:?}", self.exact_checkpoint_coordinator.stage()),
            work,
        )
    }

    #[cfg(feature = "gpu-tests")]
    pub fn exact_checkpoint_follow_up_queued_for_test(&self) -> bool {
        self.exact_checkpoint_coordinator
            .checkpoint_needed_after_current()
    }

    #[cfg(feature = "gpu-tests")]
    pub fn exact_checkpoint_accepts_journal_entries_for_test(&self) -> bool {
        self.exact_checkpoint_accepts_journal_entries()
    }

    #[cfg(feature = "gpu-tests")]
    pub fn durable_completed_sleep_permitted_ids_for_test(&self) -> Vec<OrganismId> {
        let ExactPopulationCheckpointRuntimeWorkV1::AwaitingJournal { permit, .. } =
            &self.exact_checkpoint_work
        else {
            return Vec::new();
        };
        permit
            .published()
            .save
            .creatures
            .iter()
            .filter_map(|creature| {
                let Some(brain) = creature.gpu_brain.as_ref() else {
                    return None;
                };
                (matches!(
                    brain.sleep.consolidation,
                    ConsolidationState::Completed { .. }
                ) && self
                    .residents
                    .get(&creature.organism_id.raw())
                    .is_some_and(|resident| resident.sleep_scheduler.state() == brain.sleep))
                .then_some(creature.organism_id)
            })
            .collect()
    }

    #[cfg(feature = "gpu-tests")]
    pub fn exact_population_capture_metrics_for_test(&self) -> GpuExactPopulationCaptureMetricsV1 {
        self.backend.backend().exact_population_capture_metrics()
    }

    #[cfg(feature = "gpu-tests")]
    pub fn pending_exact_sleep_journal_entries_for_test(
        &self,
    ) -> &[GpuSleepTransactionJournalEntryV2] {
        &self.pending_exact_sleep_journal_entries
    }

    #[cfg(feature = "gpu-tests")]
    pub fn queue_exact_sleep_journal_entries_for_test(
        &mut self,
        entries: Vec<GpuSleepTransactionJournalEntryV2>,
    ) -> Result<(), GameAppShellError> {
        self.queue_exact_checkpoint_journal_entries(entries)
    }

    #[cfg(feature = "gpu-tests")]
    pub fn force_compact_checkpoint_identity_drift_for_test(
        &mut self,
        organism_id: OrganismId,
    ) -> Result<(), ScaffoldContractError> {
        let handle = self.evidence_handle(organism_id)?;
        self.backend
            .force_activity_sequence_cursor_for_test(handle, u64::MAX - 1)
    }

    #[cfg(feature = "gpu-tests")]
    pub fn restored_clone_from_durability_for_test(&self) -> Result<Self, GameAppShellError> {
        let durability = self
            .checkpoint_durability
            .as_ref()
            .ok_or(ScaffoldContractError::MissingPhaseData)?;
        Self::restore_loaded_save(
            self.new_staging_like_live()?,
            durability.durable_manifest.clone(),
            durability.published.clone(),
            self.deterministic_seed,
            self.brain_class,
        )
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
    pub fn last_sleep_memory_compaction_preparation_count_for_test(&self) -> usize {
        self.last_sleep_memory_compaction_preparation_count
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
    pub fn restored_replay_patches_for_test(&self) -> &[ExperiencePatch] {
        &self.restored_replay_patches
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

    #[cfg(any(test, feature = "gpu-tests"))]
    pub fn force_late_advance_failure_for_test(&mut self) {
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
    if !matches!(
        capacity.id(),
        BrainCapacityClass::N512_ID | BrainCapacityClass::N2048_ID
    ) {
        return Ok(development.clone());
    }

    // Checked production foundation assets own a full immutable coordinate ABI.
    // World development remains authoritative in ResidentCognition; the
    // construction input removes runtime chronology and dynamic gates that
    // would reshape that ABI.
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

pub(crate) fn compile_gpu_components_from_genome(
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
        phenotype
            .foundation_abi()
            .canonical_v2()
            .cloned()
            .ok_or(ScaffoldContractError::PhenotypeCompile)?,
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
        let (phenotype, _) = compile_gpu_components_from_genome(
            genome.clone(),
            development.clone(),
            sensor_profile,
        )?;
        return Ok((phenotype, genome, development));
    }

    if capacity.id() == BrainCapacityClass::N512_ID {
        let genome = BrainGenome::scaffold(N512_FOUNDATION_SEED, capacity.id());
        let development = DevelopmentState::new(genome.id, tick, NormalizedScalar::new(1.0)?);
        let (phenotype, _) = compile_gpu_components_from_genome(
            genome.clone(),
            development.clone(),
            sensor_profile,
        )?;
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
        sync::Arc,
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
    use alife_runtime::{GpuDurableSaveManifest, GpuSessionAuthority, GpuSessionConsumerKind};
    use alife_world::{
        persistence::{AssetManifest, PortableSaveFile, RuntimeConfig},
        HeadlessScenarioBuilder, HeadlessWorld, WorldOrganismRecord,
    };

    #[test]
    fn deferred_checkpoint_publication_does_not_block_ordinary_ticks() {
        assert_eq!(
            no_progress_reason_for_checkpoint_stage(
                ExactPopulationCheckpointStageV1::DeferredJournalPublishing
            ),
            None
        );
        assert_eq!(
            no_progress_reason_for_checkpoint_stage(ExactPopulationCheckpointStageV1::Failed),
            Some(GpuLiveNoProgressReason::CheckpointFailed)
        );
        assert_eq!(
            no_progress_reason_for_checkpoint_stage(ExactPopulationCheckpointStageV1::Idle),
            None
        );
    }

    fn finish_failed_checkpoint_worker_join(
        failed: &mut FailedExactPopulationCheckpointWorkerJoinV1,
    ) -> (GameAppShellError, bool) {
        for _ in 0..10_000 {
            match failed.poll() {
                FailedExactPopulationCheckpointWorkerJoinPollV1::Pending => {
                    std::thread::yield_now();
                }
                FailedExactPopulationCheckpointWorkerJoinPollV1::Ready {
                    error,
                    worker_panicked,
                } => return (error, worker_panicked),
            }
        }
        panic!("checkpoint worker did not terminate within the bounded join poll budget");
    }

    #[test]
    fn prevalidate_failure_aborts_and_joins_the_worker_before_releasing_its_lease() {
        let mut authority = GpuSessionAuthority::new(GpuSessionConsumerKind::Gameplay);
        authority
            .note_durable_checkpoint(
                DurableGpuCheckpointRef::try_new(
                    Tick::new(9),
                    "fnv1a64:0000000000000009".to_string(),
                    [9; 4],
                )
                .unwrap(),
            )
            .unwrap();
        let prevalidate_error = authority
            .prevalidate_durable_checkpoint(
                DurableGpuCheckpointRef::try_new(
                    Tick::new(8),
                    "fnv1a64:0000000000000008".to_string(),
                    [8; 4],
                )
                .unwrap(),
            )
            .unwrap_err();

        let lease = Arc::new(());
        let worker_lease = Arc::clone(&lease);
        let (command_sender, command_receiver) = mpsc::sync_channel(1);
        let (event_sender, event_receiver) = mpsc::sync_channel(1);
        let join_handle = std::thread::spawn(move || {
            let _lease = worker_lease;
            assert!(matches!(
                command_receiver.recv(),
                Ok(ExactPopulationCheckpointWorkerCommandV1::Abort)
            ));
            drop(event_sender);
        });
        let worker = ExactPopulationCheckpointWorkerOwnerV1 {
            command_sender,
            event_receiver,
            join_handle,
        };
        let mut failed = worker.abort_and_retain(prevalidate_error.into());
        assert_eq!(
            failed.abort_delivery(),
            ExactPopulationCheckpointAbortDeliveryV1::Enqueued
        );
        assert_eq!(Arc::strong_count(&lease), 2);

        let (_, worker_panicked) = finish_failed_checkpoint_worker_join(&mut failed);
        assert!(!worker_panicked);
        assert_eq!(Arc::strong_count(&lease), 1);
    }

    #[test]
    fn disconnected_event_channel_sends_abort_and_retains_the_worker_until_join() {
        let lease = Arc::new(());
        let worker_lease = Arc::clone(&lease);
        let (command_sender, command_receiver) = mpsc::sync_channel(1);
        let (event_sender, event_receiver) = mpsc::sync_channel(1);
        let (event_dropped_sender, event_dropped_receiver) = mpsc::sync_channel(0);
        let join_handle = std::thread::spawn(move || {
            let _lease = worker_lease;
            drop(event_sender);
            event_dropped_sender.send(()).unwrap();
            assert!(matches!(
                command_receiver.recv(),
                Ok(ExactPopulationCheckpointWorkerCommandV1::Abort)
            ));
        });
        event_dropped_receiver.recv().unwrap();
        let worker = ExactPopulationCheckpointWorkerOwnerV1 {
            command_sender,
            event_receiver,
            join_handle,
        };
        assert!(matches!(
            worker.try_recv_event(),
            Err(TryRecvError::Disconnected)
        ));
        let mut failed = worker.abort_and_retain(GameAppShellError::Core(
            ScaffoldContractError::NeuralBackendUnavailable,
        ));

        let (_, worker_panicked) = finish_failed_checkpoint_worker_join(&mut failed);
        assert!(!worker_panicked);
        assert_eq!(Arc::strong_count(&lease), 1);
    }

    #[test]
    fn disconnected_command_channel_and_worker_panic_are_joined_once() {
        let lease = Arc::new(());
        let worker_lease = Arc::clone(&lease);
        let (command_sender, command_receiver) = mpsc::sync_channel(1);
        let (event_sender, event_receiver) = mpsc::sync_channel(1);
        let (ready_sender, ready_receiver) = mpsc::sync_channel(0);
        let join_handle = std::thread::spawn(move || {
            let _lease = worker_lease;
            drop(command_receiver);
            drop(event_sender);
            ready_sender.send(()).unwrap();
            panic!("forced checkpoint worker panic");
        });
        ready_receiver.recv().unwrap();
        let worker = ExactPopulationCheckpointWorkerOwnerV1 {
            command_sender,
            event_receiver,
            join_handle,
        };
        let mut failed = worker.abort_and_retain(GameAppShellError::Core(
            ScaffoldContractError::NeuralBackendUnavailable,
        ));
        assert_eq!(
            failed.abort_delivery(),
            ExactPopulationCheckpointAbortDeliveryV1::WorkerDisconnected
        );

        let (_, worker_panicked) = finish_failed_checkpoint_worker_join(&mut failed);
        assert!(worker_panicked);
        assert_eq!(Arc::strong_count(&lease), 1);
    }

    #[test]
    fn phenotype_policy_and_subsystem_work_join_are_causal() {
        let capacity = BrainCapacityClass::n512();
        let genome = BrainGenome::scaffold(N512_FOUNDATION_SEED, capacity.id())
            .with_cognitive_architecture(
                alife_core::genome::CognitiveArchitectureGenomeParameters::try_new_v1(
                    1, 16, 4, 8, 0.031, 1, 8, 64, 4, 1, 0.41, 0.73, 0.62, 0.11, 0.12, 0.13, 0.14,
                )
                .unwrap(),
            )
            .unwrap();
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
        let (phenotype, _) = compile_gpu_components_from_genome(
            genome,
            development,
            SensorProfile::GroundedObjectSlotsV1,
        )
        .unwrap();

        let attention = attention_selection_policy_for(&phenotype);
        assert_eq!(attention.focal_capacity, 1);
        assert_eq!(attention.requested_focal_count, 1);
        assert_eq!(
            phenotype.cognitive_architecture().predictor_learning_rate(),
            0.031
        );
        let sleep = sleep_consolidation_config_for(&phenotype).unwrap();
        assert_eq!(sleep.sleep_pressure_threshold.raw(), 0.41);
        assert_eq!(sleep.lifetime_staging_rate.raw(), 0.62);
        assert_eq!(sleep.structural_edit_candidate_limit, 4);

        let neural = BrainWorkCounters {
            neuron_updates: 11,
            synapse_ops: 13,
            ..BrainWorkCounters::default()
        };
        let v11 = GpuV11WorkReceipt {
            cognitive: CognitiveWorkReceipt::from_counters(0, 0, 5, 0, 0, 0, 0, 0, 0, 7, 0, 0)
                .unwrap(),
            ..GpuV11WorkReceipt::default()
        };
        let merged =
            cognitive_work_receipt_from_subsystems(&neural, &v11, 3, 4, 5, 6, 7, 8, 9, 10).unwrap();
        assert_eq!(merged.dendritic_ops, 5);
        assert_eq!(merged.replay_ops, 8);
        assert_eq!(merged.structural_ops, 7);
        assert_eq!(merged.weighted_total, 88);
    }

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
        let baseline_context =
            cognitive_context_for_recall(organism_id, sequence_id, &prepared_recall, &topology)
                .unwrap();
        let baseline_prepared = prepared_recall
            .clone()
            .with_cognitive_context(baseline_context.clone())
            .unwrap();
        let (baseline_frame, baseline_recall) = baseline_prepared.finalize(draft.clone()).unwrap();
        baseline_recall.validate_for_frame(&baseline_frame).unwrap();
        let memory_evidence = finalized_memory_attention_evidence(&baseline_recall).unwrap();
        let body_need = homeostasis
            .drives
            .to_array()
            .iter()
            .copied()
            .fold(0.0, f32::max);

        let mut base_summaries =
            grounded_peripheral_summaries(draft.grounded_object_slots()).unwrap();
        assert!(base_summaries.len() >= 2);
        for summary in &mut base_summaries {
            summary.salience = SalienceComponents::default();
        }
        base_summaries[0].salience.peripheral_intensity = NormalizedScalar::new(0.2).unwrap();
        base_summaries[1].salience.peripheral_intensity = NormalizedScalar::new(0.1).unwrap();
        let first_identity = base_summaries[0].identity;
        let second_identity = base_summaries[1].identity;
        let canonical = runtime.world.organism_registry().get(organism_id).unwrap();
        let receptors = canonical
            .biochemistry()
            .neural_receptor_frame(canonical.phenotype())
            .unwrap();
        apply_predecision_attention_evidence(
            &mut base_summaries,
            body_need,
            &memory_evidence,
            &baseline_context,
            NeuralReceptorEffects::from_frame(
                &receptors,
                &NeuralReceptorPhenotype::compile(&runtime.residents[&organism_id.raw()].phenotype)
                    .unwrap(),
            )
            .unwrap(),
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
            let context =
                cognitive_context_for_recall(organism_id, sequence_id, &routed_recall, &topology)?;
            let context = cognitive_context_with_attention(context, attention)?;
            let prepared = routed_recall.with_cognitive_context(context)?;
            let (frame, memory_recall) = prepared.finalize(routed_draft)?;
            memory_recall.validate_for_frame(&frame)?;
            let upload =
                runtime
                    .backend
                    .prepare_memory_context_upload(handle, &frame, &memory_recall)?;
            Ok::<_, ScaffoldContractError>((frame, memory_recall, upload))
        };
        let (base_frame, base_recall, base_upload) =
            finalize_with_attention(base_attention.clone()).unwrap();
        let (changed_frame, changed_recall, changed_upload) =
            finalize_with_attention(changed_attention.clone()).unwrap();
        assert_eq!(
            base_recall.cognitive_context().unwrap().focal.identities,
            base_attention.focal_targets
        );
        assert_eq!(
            changed_recall.cognitive_context().unwrap().focal.identities,
            changed_attention.focal_targets
        );
        assert_ne!(
            base_recall.cognitive_context_digest().unwrap(),
            changed_recall.cognitive_context_digest().unwrap()
        );
        assert_ne!(base_frame.base_digest(), changed_frame.base_digest());
        assert_ne!(base_frame.frame_digest(), changed_frame.frame_digest());
        assert_eq!(base_upload.final_frame_digest, base_frame.frame_digest());
        assert_eq!(
            changed_upload.final_frame_digest,
            changed_frame.frame_digest()
        );
        assert_ne!(
            base_upload.final_frame_digest,
            changed_upload.final_frame_digest
        );

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
        let restored_record = restored.world.organism_registry().get(organism_id).unwrap();
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
        assert_eq!(restored_resident.phenotype, source_phenotype);
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
        sleepy_biochemistry.homeostasis =
            HomeostaticSnapshot::new(sleepy_biochemistry.homeostasis.tick, drives, hormones)
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
        let sleep_summaries = restored.tick_with_sleep_driver(&mut sleep_driver).unwrap();
        assert_eq!(sleep_summaries.len(), 1);
        assert_eq!(sleep_summaries[0].status, BrainTickStatus::SafeIdle);
        assert_eq!(sleep_summaries[0].selected_action_id, None);
        assert!(!sleep_summaries[0].patch_sealed);
        assert_eq!(
            restored.world.tick(),
            Tick::new(sleep_tick_before.raw().saturating_add(1))
        );
        let refreshed_record = restored.world.organism_registry().get(organism_id).unwrap();
        let refreshed_resident = restored.residents.get(&organism_id.raw()).unwrap();
        assert_eq!(refreshed_record.biochemistry().tick, restored.world.tick());
        assert_eq!(
            refreshed_resident.homeostasis,
            refreshed_record.biochemistry().homeostasis
        );
        assert_eq!(
            refreshed_resident.development,
            refreshed_record
                .phenotype()
                .development_state_at(refreshed_record.age_at(restored.world.tick()).unwrap(),)
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
        let before_resident = runtime.residents.get(&organism_id.raw()).unwrap().clone();
        let sealed_before_failure = runtime.sealed_patches().len();
        runtime.force_late_advance_failure_for_test();
        let result = runtime.tick();
        assert!(matches!(
            result,
            Err(GameAppShellError::Core(
                ScaffoldContractError::NonMonotonicTick
            ))
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
                validate_replacement_policy(policy, seed, brain_class, 7, BrainScaleTier::Nano512,),
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
                    phenotype.foundation_abi().canonical_v2().cloned().unwrap(),
                )
                .unwrap();
                (
                    raw,
                    ResidentCognition {
                        phenotype,
                        compiler_inputs,
                        legacy_nano512_compatibility_receipt: None,
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
                        last_cognitive_context: None,
                        last_selected_motor_bundle: None,
                        last_cognitive_work: CognitiveWorkReceipt::zero(),
                        last_sleep_work: None,
                        last_structural_edit_receipts: Vec::new(),
                        last_sleep_report: None,
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
        assert!(
            retained_plan.is_none(),
            "a retained operation is not GPU residency"
        );
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
            CuratedFounderResetRuntimeError::PreCommit(CuratedFounderStagingError::Mismatch {
                field: "apply world tick"
            })
        ));
        assert!(retained_plan.is_none());
        assert_eq!(
            operation_fingerprint(retained_operation.as_ref().unwrap()),
            retained_fingerprint
        );

        conflict_fixture.world = conflict_fixture
            .source_save
            .restore_headless_world()
            .unwrap();
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
        let replacement = already_operation.as_ref().unwrap().test_replacement_save();
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
        let projected_refresh_failure = project_curated_founder_reset_runtime_error(refresh_error);
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
        assert_eq!(
            retained_refresh_plan.entries.len(),
            refresh_bundle.entries.len()
        );
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
        let genome =
            alife_core::CreatureGenome::early_mammal_founder(0x3_3B_00_0011, foundation).unwrap();
        let phenotype = genome.express().unwrap();
        let biochemistry = alife_core::BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
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
            .perception_frame(organism_id, Tick::ZERO, sensor_profile, initial_homeostasis)
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
            .apply_registered_neural_command(&command, world_entity_id, Tick::new(1), None, false)
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
        assert_eq!(plan.development.age_ticks, authoritative_age);
        assert_eq!(
            plan.development.genome_id,
            record.phenotype().brain_genome.id
        );
        assert_eq!(plan.development, authoritative_development);
        let (expected_phenotype, expected_inputs, expected_receipt) =
            PhenotypeCompiler::compile_fixed_legacy_nano512_compatibility_asset(
                sensor_profile,
                &foundation_asset,
            )
            .unwrap()
            .into_runtime_parts();
        assert_eq!(plan.phenotype, expected_phenotype);
        assert_ne!(plan.compiler_inputs.genome(), &plan.genome);
        assert_ne!(plan.compiler_inputs.development(), &plan.development);
        assert_eq!(plan.compiler_inputs.genome(), expected_inputs.genome());
        assert_eq!(
            plan.compiler_inputs.development(),
            expected_inputs.development()
        );
        assert_eq!(
            plan.legacy_nano512_compatibility_receipt.as_ref(),
            Some(&expected_receipt)
        );
        expected_receipt
            .validate_against(&plan.phenotype, &foundation_asset)
            .unwrap();
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
                legacy_nano512_compatibility_receipt: None,
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
            phenotype.foundation_abi().canonical_v2().cloned().unwrap(),
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
        assert!(
            runtime
                .world
                .organism_registry()
                .get(OrganismId(1))
                .unwrap()
                .biochemistry()
                .body
                .sleeping
        );

        let summaries = runtime.tick_with_sleep_driver(&mut driver).unwrap();

        assert_eq!(summaries[0].status, BrainTickStatus::Normal);
        assert!(summaries[0].patch_sealed);
        assert_eq!(runtime.backend.completed_dispatch_count(), 1);
        assert_eq!(driver.intents.len(), 1);
        assert!(
            !runtime
                .world
                .organism_registry()
                .get(OrganismId(1))
                .unwrap()
                .biochemistry()
                .body
                .sleeping
        );
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
            phenotype.foundation_abi().canonical_v2().cloned().unwrap(),
        )
        .unwrap();
        let mut residents = BTreeMap::from([(
            organism_id.raw(),
            ResidentCognition {
                phenotype: phenotype.clone(),
                compiler_inputs,
                legacy_nano512_compatibility_receipt: None,
                genome: genome.clone(),
                development: development.clone(),
                homeostasis: biology_before.homeostasis,
                sleep_scheduler: GpuSleepScheduler::new(SleepConsolidationConfig::reference())
                    .unwrap(),
                next_sequence: 1,
                language_grounding: LanguageGroundingLedger::default(),
                life_statistics: PassiveLifeStatistics::new(organism_id, Tick::ZERO).unwrap(),
                attention_hysteresis: alife_core::HysteresisState::default(),
                predictor: GroundedSuccessorPredictor::default(),
                last_cognitive_context: None,
                last_selected_motor_bundle: None,
                last_cognitive_work: CognitiveWorkReceipt::zero(),
                last_sleep_work: None,
                last_structural_edit_receipts: Vec::new(),
                last_sleep_report: None,
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
                frame: frame.clone(),
                memory,
                sequence_id,
                outcome_tick: Tick::new(1),
                cognitive_context,
                work,
                v11_work: GpuV11WorkReceipt::default(),
                pre_action,
                decision: decision.clone(),
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
        assert_eq!(
            sealed.patch.header().abi_version,
            ExperiencePatch::V11_ABI_VERSION
        );
        assert!(sealed.patch.prediction_target().is_some());
        assert_eq!(
            world
                .organism_registry()
                .get(organism_id)
                .unwrap()
                .cognitive_work(),
            sealed.patch.cognitive_work().unwrap()
        );

        assert_eq!(
            expected_receipt.action_result.body_event.sleep_recovery,
            1.0
        );
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
        assert_ne!(
            expected_receipt.biology_after.homeostasis,
            learning_projection
        );
        assert_eq!(
            world_after.homeostasis,
            expected_receipt.biology_after.homeostasis
        );
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
        let canonical = runtime.world.organism_registry().get(organism_id).unwrap();
        let neural_receptors = canonical
            .biochemistry()
            .neural_receptor_frame(canonical.phenotype())
            .unwrap();
        let receptor_effects = NeuralReceptorEffects::from_frame(
            &neural_receptors,
            &NeuralReceptorPhenotype::compile(&runtime.residents[&organism_id.raw()].phenotype)
                .unwrap(),
        )
        .unwrap();
        let memory_upload = runtime
            .backend
            .prepare_memory_context_upload(handle, &frame, &memory_recall)
            .unwrap()
            .bind_neural_receptor_effects(receptor_effects)
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
                neural_receptors,
                receptor_effects,
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
        let canonical = runtime
            .world
            .organism_registry()
            .get(OrganismId(1))
            .unwrap();
        let neural_receptors = canonical
            .biochemistry()
            .neural_receptor_frame(canonical.phenotype())
            .unwrap();
        let receptor_effects = NeuralReceptorEffects::from_frame(
            &neural_receptors,
            &NeuralReceptorPhenotype::compile(&runtime.residents[&1].phenotype).unwrap(),
        )
        .unwrap();
        let memory_upload = runtime
            .backend
            .prepare_memory_context_upload(handle, &frame, &memory_recall)
            .unwrap()
            .bind_neural_receptor_effects(receptor_effects)
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
                    neural_receptors,
                    receptor_effects,
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
        assert!(credit.modulator().homeostatic_improvement() < 0.0);
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
        let archive_root =
            std::env::temp_dir().join(format!("alife-gpu-newborn-{label}-{}", std::process::id()));
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
            .apply_registered_neural_command(&command, parent_entity_id, next_tick, None, false)
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
        assert_eq!(
            runtime
                .topologies
                .get(&newborn.raw())
                .unwrap()
                .organism_id(),
            newborn
        );
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
        assert_eq!(
            runtime
                .topologies
                .get(&newborn.raw())
                .unwrap()
                .organism_id(),
            newborn
        );
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
        let genome =
            alife_core::CreatureGenome::early_mammal_founder(0xE10_42C1, foundation).unwrap();
        let phenotype = genome.express().unwrap();
        let biochemistry = alife_core::BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
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

        let receipt = runtime.retire_organism(organism_id, "test-death").unwrap();
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

        let repeated = runtime.retire_organism(organism_id, "test-death").unwrap();
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
        let genome =
            alife_core::CreatureGenome::early_mammal_founder(0xE10_42C2, foundation).unwrap();
        let phenotype = genome.express().unwrap();
        let biochemistry = alife_core::BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
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
        assert_eq!(
            runtime.world.canonical_signature_digest().unwrap(),
            before_signature
        );
        assert_eq!(
            runtime.world.organism_registry().get(organism_id),
            Some(&before_record)
        );
        assert_eq!(runtime.world.entity(world_entity_id), Some(&before_object));
        assert_eq!(runtime.handles, before_handles);
        assert_eq!(
            runtime.residents.keys().copied().collect::<Vec<_>>(),
            before_resident_keys
        );
        assert_eq!(
            runtime.memories.keys().copied().collect::<Vec<_>>(),
            before_memory_keys
        );
        assert_eq!(
            runtime.topologies.keys().copied().collect::<Vec<_>>(),
            before_topology_keys
        );
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
