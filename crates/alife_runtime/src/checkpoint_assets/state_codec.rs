//! Exact portable checkpoint construction and validated GPU restore.

use alife_core::{
    BoundedReplayBatch, BrainCapacityClass, BrainPhenotype, ConsolidationState, ExperiencePatch,
    ExperiencePatchBuilder, FoundationWeightAsset, LanguageGroundingLedger,
    LegacyNano512CompatibilityReceipt, MemorySidecarState, NeuralReceptorFrame, OrganismId,
    PassiveLifeStatistics, PhenotypeCompiler, PhenotypeCompilerInputs, PortableMemoryBankAssetV2,
    PortableTopologySidecarAssetV1, ScaffoldContractError, SensorProfileIdentity,
    SensoryAbiVersion, SleepState, Tick, TopologySidecar, Validate,
};
use alife_gpu_backend::{
    GpuActivityRestoreInput, GpuActivityRuntimeSnapshot, GpuBrainCheckpointParts,
    GpuBrainCheckpointSnapshot, GpuBrainHandle, GpuBrainRestoreReceipt, GpuBrainRestoreRequest,
    GpuClosedLoopBackend, GpuCompletedSleepStagingInputParts, GpuCompletedSleepStagingParts,
    GpuPortableActivityRestoreRecord, GpuReplayEventRecord, GpuReplaySynapseSpanRecord,
    PendingEligibilityRestoreParts, GPU_BRAIN_CHECKPOINT_SCHEMA_VERSION,
};
use alife_world::persistence::{
    AssetManifest, AssetManifestEntry, DurableFounderCognitiveState, ExactCognitiveCheckpointState,
    GpuBackendProvenanceSave, GpuBrainAssetRef, GpuBrainSaveState, GpuSleepAssetState,
    MemorySidecarSaveState, NeuralGpuBackendApi, PendingEligibilityCheckpoint,
    PortableActivationBanksV1, PortableDualWeightBankV1, PortableEligibilityBanksV1,
    PortableNeuronHomeostasisV1, PortableReplayJournalV1, PortableThrottleCheckpoint,
    RetainedLearningRecoverySaveState, ThrottleReplaySaveInput, ThrottleReplaySaveState,
    TopologySidecarSaveSummary, GPU_BACKEND_PROVENANCE_SAVE_SCHEMA_VERSION,
    GPU_BRAIN_HOMEOSTASIS_LANES_PER_NEURON, GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION,
    GPU_BRAIN_SAVE_STATE_SCHEMA_VERSION, GPU_BRAIN_WEIGHT_LAYER_FAST,
    GPU_BRAIN_WEIGHT_LAYER_LIFETIME, THROTTLE_REPLAY_SAVE_SCHEMA_VERSION,
};
use alife_world::{
    initial_tracked_object_id, TrackedObjectRegistrySaveState,
    TRACKED_OBJECT_REGISTRY_SAVE_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};

use crate::{GameAppShellError, GpuAuthoritativeSession};

use super::{
    content_store::GpuCheckpointAssetStore,
    replay_codec::{decode_physical_replay, encode_portable_replay},
};

const PENDING_TRANSACTION_SCHEMA_VERSION: u16 = 1;
const RUNTIME_REPLAY_STATE_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct PortableRuntimeReplayStateV1 {
    schema_version: u16,
    journal: PortableReplayJournalV1,
    replay_patches: Vec<ExperiencePatch>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PortableRuntimeReplayStateWire {
    Runtime(PortableRuntimeReplayStateV1),
    Legacy(PortableReplayJournalV1),
}

pub fn current_backend_provenance(
    backend: &GpuClosedLoopBackend,
    capacity: &BrainCapacityClass,
) -> Result<GpuBackendProvenanceSave, GameAppShellError> {
    let hardware = backend.hardware_receipt();
    let budget = backend.runtime_budget();
    let api = NeuralGpuBackendApi::try_from_slug(&hardware.backend_api)?;
    let mut version_parts = hardware.backend_version.split('.');
    let major = version_parts
        .next()
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
    let minor = version_parts
        .next()
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
    let patch = version_parts
        .next()
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
    if version_parts.next().is_some() {
        return Err(ScaffoldContractError::NeuralBackendUnavailable.into());
    }
    let mut provenance = GpuBackendProvenanceSave {
        schema_version: GPU_BACKEND_PROVENANCE_SAVE_SCHEMA_VERSION,
        backend_api_raw: api.raw(),
        vendor_id: hardware.vendor_id,
        device_id: hardware.device_id,
        backend_version_major: major,
        backend_version_minor: minor,
        backend_version_patch: patch,
        adapter_name_len: 0,
        adapter_name_utf8: [0; 128],
        driver_digest: hardware.driver_digest,
        required_features_digest: budget.required_features_digest()?,
        required_limits_digest: budget.required_limits_digest_for(capacity.execution())?,
        available_features_digest: hardware.feature_digest,
        adapter_limits_digest: hardware.limits_digest,
    };
    provenance.set_adapter_name(&hardware.adapter_name)?;
    Ok(provenance)
}

fn portable_activity_checkpoint(
    backend: &GpuClosedLoopBackend,
    handle: GpuBrainHandle,
) -> Result<(GpuActivityRuntimeSnapshot, Vec<PortableThrottleCheckpoint>), GameAppShellError> {
    let snapshot = backend.snapshot_activity_state(handle)?;
    let records = match (
        snapshot.pressure,
        snapshot.throttle.clone(),
        snapshot.work.clone(),
    ) {
        (Some(pressure), Some(throttle), Some(work)) => {
            if snapshot.next_sequence_cursor != pressure.sequence_cursor.checked_add(1).unwrap_or(0)
            {
                return Err(ScaffoldContractError::BrainActivitySequenceMismatch.into());
            }
            vec![PortableThrottleCheckpoint {
                schema_version:
                    alife_world::persistence::PORTABLE_THROTTLE_CHECKPOINT_SCHEMA_VERSION,
                policy_version: throttle.policy_version,
                organism_id_raw: pressure.organism_id_raw,
                tick: pressure.tick,
                class_id_raw: pressure.class_id_raw,
                sequence_cursor: pressure.sequence_cursor,
                dispatch_generation: pressure.dispatch_generation,
                frame_digest: pressure.frame_digest,
                source_dispatch_generation: pressure.source_dispatch_generation,
                source_frame_digest: pressure.source_frame_digest,
                completed_gpu_time_ns: pressure.completed_gpu_time_ns,
                queue_depth: pressure.queue_depth,
                logical_heap_pressure_q16: pressure.logical_heap_pressure_q16,
                brain_atp_fraction_q16: pressure.brain_atp_fraction_q16,
                level: throttle.level,
                microsteps: throttle.microsteps,
                enabled_route_ids: throttle.enabled_route_ids,
                route_schedule_digest: throttle.route_schedule_digest,
                work: work.counters,
                neural_cost_q24: work.neural_cost_q24,
                atp_before_q16: work.atp_before_q16,
                atp_debit_q16: work.atp_debit_q16,
                atp_after_q16: work.atp_after_q16,
                policy_digest: throttle.policy_digest,
                portable_digest: [0; 4],
            }
            .seal()?]
        }
        (None, None, None) if snapshot.next_sequence_cursor == 1 => Vec::new(),
        _ => return Err(ScaffoldContractError::BrainActivitySequenceMismatch.into()),
    };
    Ok((snapshot, records))
}

fn activity_restore_record(
    checkpoint: &PortableThrottleCheckpoint,
) -> GpuPortableActivityRestoreRecord {
    GpuPortableActivityRestoreRecord {
        policy_version: checkpoint.policy_version,
        organism_id_raw: checkpoint.organism_id_raw,
        tick: checkpoint.tick,
        class_id_raw: checkpoint.class_id_raw,
        sequence_cursor: checkpoint.sequence_cursor,
        dispatch_generation: checkpoint.dispatch_generation,
        frame_digest: checkpoint.frame_digest,
        source_dispatch_generation: checkpoint.source_dispatch_generation,
        source_frame_digest: checkpoint.source_frame_digest,
        completed_gpu_time_ns: checkpoint.completed_gpu_time_ns,
        queue_depth: checkpoint.queue_depth,
        logical_heap_pressure_q16: checkpoint.logical_heap_pressure_q16,
        brain_atp_fraction_q16: checkpoint.brain_atp_fraction_q16,
        level: checkpoint.level,
        microsteps: checkpoint.microsteps,
        enabled_route_ids: checkpoint.enabled_route_ids.clone(),
        route_schedule_digest: checkpoint.route_schedule_digest,
        work: checkpoint.work,
        neural_cost_q24: checkpoint.neural_cost_q24,
        atp_before_q16: checkpoint.atp_before_q16,
        atp_debit_q16: checkpoint.atp_debit_q16,
        atp_after_q16: checkpoint.atp_after_q16,
        policy_digest: checkpoint.policy_digest,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuBrainCheckpointWrite {
    pub save_state: GpuBrainSaveState,
    pub manifest_entries: Vec<AssetManifestEntry>,
    pub checkpoint_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuDurableFounderWrite {
    pub checkpoint: GpuBrainCheckpointWrite,
    pub phenotype: BrainPhenotype,
    pub compiler_inputs: PhenotypeCompilerInputs,
    pub next_experience_sequence_raw: u64,
    pub durable_cognitive_state: Option<GpuBrainAssetRef>,
}

#[derive(Debug)]
pub struct RestoredGpuBrainCheckpoint {
    pub receipt: GpuBrainRestoreReceipt,
    pub phenotype: BrainPhenotype,
    pub compiler_inputs: PhenotypeCompilerInputs,
    pub legacy_nano512_compatibility_receipt: Option<LegacyNano512CompatibilityReceipt>,
    pub sleep: SleepState,
    pub pending_transaction: Option<ExperiencePatchBuilder>,
    pub memory: MemorySidecarState,
    pub topology: TopologySidecar,
    pub tracked_objects: TrackedObjectRegistrySaveState,
    pub language_grounding: LanguageGroundingLedger,
    pub life_statistics: Option<PassiveLifeStatistics>,
    pub retained_learning: Option<RestoredRetainedLearning>,
    pub exact_cognitive_state: Option<ExactCognitiveCheckpointState>,
    pub replay_patches: Vec<ExperiencePatch>,
}

pub struct GpuBrainSidecarCapture<'a> {
    pub sensor_profile: SensorProfileIdentity,
    pub memory: &'a MemorySidecarState,
    pub topology: &'a TopologySidecar,
    pub tracked_objects: TrackedObjectRegistrySaveState,
    pub language_grounding: &'a LanguageGroundingLedger,
    pub life_statistics: &'a PassiveLifeStatistics,
    pub legacy_nano512_compatibility_receipt: Option<&'a LegacyNano512CompatibilityReceipt>,
    pub retained_learning: Option<RetainedLearningCapture<'a>>,
}

pub struct RetainedLearningCapture<'a> {
    pub sealed_patch: &'a ExperiencePatch,
    pub neural_receptors: &'a NeuralReceptorFrame,
    pub attempts: u8,
    pub last_error_code: &'static str,
}

#[derive(Debug)]
pub struct RestoredRetainedLearning {
    pub sealed_patch: ExperiencePatch,
    pub neural_receptors: NeuralReceptorFrame,
    pub attempts: u8,
    pub last_error_code: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct PhysicalReplayParts {
    pub events: Vec<GpuReplayEventRecord>,
    pub spans: Vec<GpuReplaySynapseSpanRecord>,
    pub samples: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct PendingExperienceTransactionV1 {
    schema_version: u16,
    builder: ExperiencePatchBuilder,
}

impl GpuBrainCheckpointWrite {
    pub fn attach_exact_cognitive_state(
        &mut self,
        store: &GpuCheckpointAssetStore,
        state: &ExactCognitiveCheckpointState,
    ) -> Result<(), GameAppShellError> {
        state.validate()?;
        if state.organism_id != self.save_state.organism_id
            || state.checkpoint_tick != self.save_state.checkpoint_tick
        {
            return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
        }
        let _ = state.encode()?;
        let (asset, entry) = store.write_json("v11-exact-cognitive", state)?;
        self.save_state.exact_cognitive_state = Some(asset.clone());
        self.manifest_entries.push(entry);
        Ok(())
    }

    pub fn attach_durable_founder_state(
        &mut self,
        store: &GpuCheckpointAssetStore,
        state: &DurableFounderCognitiveState,
    ) -> Result<GpuBrainAssetRef, GameAppShellError> {
        state.validate()?;
        let _ = state.encode()?;
        let (asset, entry) = store.write_json("v11-durable-founder-cognitive", state)?;
        self.manifest_entries.push(entry);
        Ok(asset)
    }
}

impl GpuCheckpointAssetStore {
    pub fn restore_brain_checkpoint(
        &self,
        backend: &mut GpuAuthoritativeSession,
        manifest: &AssetManifest,
        checkpoint: &GpuBrainCheckpointWrite,
    ) -> Result<RestoredGpuBrainCheckpoint, GameAppShellError> {
        let exact_asset = checkpoint
            .save_state
            .exact_cognitive_state
            .as_ref()
            .ok_or(ScaffoldContractError::MissingPhaseData)?;
        let exact_cognitive_state = self.read_exact_cognitive_state(manifest, exact_asset)?;
        if exact_cognitive_state.organism_id != checkpoint.save_state.organism_id
            || exact_cognitive_state.checkpoint_tick != checkpoint.save_state.checkpoint_tick
        {
            return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
        }
        let mut restored = self.restore_brain(backend, manifest, &checkpoint.save_state)?;
        restored.exact_cognitive_state = Some(exact_cognitive_state);
        Ok(restored)
    }

    pub fn read_exact_cognitive_state(
        &self,
        manifest: &AssetManifest,
        asset: &GpuBrainAssetRef,
    ) -> Result<ExactCognitiveCheckpointState, GameAppShellError> {
        let (_candidate, bytes): (ExactCognitiveCheckpointState, Vec<u8>) =
            self.read_json(manifest, asset)?;
        Ok(ExactCognitiveCheckpointState::decode(&bytes)?)
    }

    pub fn read_durable_founder_state(
        &self,
        manifest: &AssetManifest,
        asset: &GpuBrainAssetRef,
    ) -> Result<DurableFounderCognitiveState, GameAppShellError> {
        let (_candidate, bytes): (DurableFounderCognitiveState, Vec<u8>) =
            self.read_json(manifest, asset)?;
        Ok(DurableFounderCognitiveState::decode(&bytes)?)
    }

    /// Creates a healthy cross-save mind clone through the production GPU
    /// checkpoint path. Only consolidated lifetime weights and durable learned
    /// sidecars cross the boundary; transient neural and world state come from
    /// a fresh birth slot for the target identity.
    pub fn clone_durable_founder(
        &self,
        backend: &mut GpuAuthoritativeSession,
        manifest: &AssetManifest,
        source_state: &GpuBrainSaveState,
        target_organism_id: OrganismId,
        target_world_seed: u64,
        target_tick: Tick,
    ) -> Result<GpuDurableFounderWrite, GameAppShellError> {
        target_organism_id.validate()?;
        if target_world_seed == 0 || source_state.organism_id == target_organism_id {
            return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
        }
        source_state.sleep.validate_contract()?;
        if !matches!(
            source_state.sleep.phase,
            alife_core::SleepPhase::Awake | alife_core::SleepPhase::Waking
        ) {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
        }

        let restored = self.restore_brain(backend, manifest, source_state)?;
        let durable_state = restored
            .exact_cognitive_state
            .as_ref()
            .map(ExactCognitiveCheckpointState::to_durable_founder)
            .transpose()?;
        let source_handle = restored.receipt.handle;
        let source_snapshot =
            match backend.snapshot_brain(source_handle, source_state.checkpoint_tick) {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    let _ = backend.remove_brain(source_handle);
                    return Err(error.into());
                }
            };
        backend.remove_brain(source_handle)?;
        let source_parts = source_snapshot.into_parts();
        let learned_lifetime = if source_parts.active_weight_bank == 0 {
            source_parts.lifetime_bank_0_bits
        } else {
            source_parts.lifetime_bank_1_bits
        };

        let phenotype = restored.phenotype;
        let compiler_inputs = restored.compiler_inputs;
        let fresh_handle = backend.insert_brain(target_organism_id, phenotype.clone())?;
        let fresh_snapshot = match backend.snapshot_brain(fresh_handle, target_tick) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                let _ = backend.remove_brain(fresh_handle);
                return Err(error.into());
            }
        };
        backend.remove_brain(fresh_handle)?;
        let mut fresh_parts = fresh_snapshot.into_parts();
        if fresh_parts.lifetime_bank_0_bits.len() != learned_lifetime.len() {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
        }
        fresh_parts
            .lifetime_bank_0_bits
            .clone_from(&learned_lifetime);
        fresh_parts.lifetime_bank_1_bits = learned_lifetime;
        let transplanted = GpuBrainCheckpointSnapshot::try_from_parts(fresh_parts)?;
        let receipt = backend.restore_brain(
            target_organism_id,
            phenotype.clone(),
            GpuBrainRestoreRequest::try_new(transplanted)?,
        )?;
        let target_handle = receipt.handle;

        let memory = restored.memory.durable_founder_clone(target_organism_id)?;
        let topology = restored
            .topology
            .durable_founder_clone(target_organism_id)?;
        let tracked_objects = TrackedObjectRegistrySaveState {
            schema_version: TRACKED_OBJECT_REGISTRY_SAVE_SCHEMA_VERSION,
            world_seed: target_world_seed,
            organism_id: target_organism_id,
            capacity: restored.tracked_objects.capacity,
            next_id: initial_tracked_object_id(target_world_seed, target_organism_id.raw()),
            records: Vec::new(),
        };
        tracked_objects.validate_contract()?;
        let statistics = PassiveLifeStatistics::new(target_organism_id, target_tick)?;
        let next_experience_sequence_raw = memory
            .latest_durable_sequence_raw()
            .checked_add(1)
            .ok_or(ScaffoldContractError::ScalarOutOfRange)?
            .max(1);
        let checkpoint = self.capture_brain(
            backend,
            target_handle,
            &phenotype,
            &compiler_inputs,
            SleepState::awake_at(target_tick),
            target_tick,
            None,
            GpuBrainSidecarCapture {
                sensor_profile: source_state.sensor_profile,
                memory: &memory,
                topology: &topology,
                tracked_objects,
                language_grounding: &restored.language_grounding,
                life_statistics: &statistics,
                legacy_nano512_compatibility_receipt: restored
                    .legacy_nano512_compatibility_receipt
                    .as_ref(),
                retained_learning: None,
            },
        );
        let removal = backend.remove_brain(target_handle);
        let mut checkpoint = checkpoint?;
        let durable_cognitive_state = durable_state
            .as_ref()
            .map(|state| checkpoint.attach_durable_founder_state(self, state))
            .transpose()?;
        removal?;
        Ok(GpuDurableFounderWrite {
            checkpoint,
            phenotype,
            compiler_inputs,
            next_experience_sequence_raw,
            durable_cognitive_state,
        })
    }

    pub fn capture_brain(
        &self,
        backend: &mut GpuAuthoritativeSession,
        handle: GpuBrainHandle,
        phenotype: &BrainPhenotype,
        compiler_inputs: &PhenotypeCompilerInputs,
        sleep: SleepState,
        checkpoint_tick: Tick,
        pending_transaction: Option<&ExperiencePatchBuilder>,
        sidecars: GpuBrainSidecarCapture<'_>,
    ) -> Result<GpuBrainCheckpointWrite, GameAppShellError> {
        self.capture_brain_with_runtime_replay_state(
            backend,
            handle,
            phenotype,
            compiler_inputs,
            sleep,
            checkpoint_tick,
            pending_transaction,
            &[],
            sidecars,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn capture_brain_with_runtime_replay_state(
        &self,
        backend: &mut GpuAuthoritativeSession,
        handle: GpuBrainHandle,
        phenotype: &BrainPhenotype,
        compiler_inputs: &PhenotypeCompilerInputs,
        sleep: SleepState,
        checkpoint_tick: Tick,
        pending_transaction: Option<&ExperiencePatchBuilder>,
        replay_patches: &[ExperiencePatch],
        sidecars: GpuBrainSidecarCapture<'_>,
    ) -> Result<GpuBrainCheckpointWrite, GameAppShellError> {
        sleep.validate_contract()?;
        let capacity = BrainCapacityClass::production_for_id(phenotype.brain_class_id())?;
        compiler_inputs.validate_against(&capacity)?;
        phenotype.validate_against(&capacity)?;
        let recompiled = PhenotypeCompiler::compile_validated(compiler_inputs, &capacity)?;
        let phenotype_bytes = serde_json::to_vec(phenotype)?;
        if recompiled != *phenotype || serde_json::to_vec(&recompiled)? != phenotype_bytes {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        match (
            phenotype.legacy_foundation_compatibility_abi(),
            sidecars.legacy_nano512_compatibility_receipt,
        ) {
            (Some(_), Some(receipt)) => {
                let asset = FoundationWeightAsset::builtin_nano512_v1(phenotype.sensor_profile())?;
                receipt.validate_against(phenotype, &asset)?;
            }
            (None, None) => {}
            _ => return Err(ScaffoldContractError::PhenotypeCompile.into()),
        }
        if phenotype.phenotype_hash() != handle.phenotype_hash()
            || handle.class_id() != phenotype.brain_class_id()
            || handle.organism_id().raw() == 0
            || sidecars.sensor_profile.profile()? != phenotype.sensor_profile()
            || sidecars.memory.organism_id() != handle.organism_id()
            || sidecars.memory.profile() != sidecars.sensor_profile
            || sidecars.topology.organism_id() != handle.organism_id()
            || sidecars.topology.profile() != sidecars.sensor_profile
            || sidecars.tracked_objects.organism_id != handle.organism_id()
        {
            return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
        }

        let snapshot = backend.snapshot_brain(handle, checkpoint_tick)?;
        let checkpoint_digest = snapshot.canonical_digest();
        let parts = snapshot.into_parts();
        let pending_checkpoint = parts
            .pending_eligibility
            .map(pending_checkpoint_from_parts)
            .transpose()?;
        sidecars.tracked_objects.validate_contract()?;
        sidecars.language_grounding.validate_contract()?;
        sidecars.life_statistics.validate_contract()?;
        if sidecars.life_statistics.organism_id() != handle.organism_id() {
            return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
        }
        match (
            pending_checkpoint,
            pending_transaction,
            sidecars.retained_learning.as_ref(),
        ) {
            (None, None, None) => {}
            (Some(pending), Some(builder), None) => {
                validate_pending_transaction(builder, handle, pending)?;
            }
            (Some(pending), None, Some(recovery)) => {
                recovery.sealed_patch.validate_contract()?;
                if recovery.sealed_patch.pre_action().organism_id != handle.organism_id()
                    || recovery.sealed_patch.header().sensor_profile.identity()
                        != sidecars.sensor_profile
                    || recovery.attempts == 0
                    || recovery.attempts > 3
                    || !matches!(
                        recovery.last_error_code,
                        "learning-evidence-mismatch"
                            | "neural-backend-unavailable"
                            | "other-contract-failure"
                    )
                    || pending.originating_tick != recovery.sealed_patch.pre_action().tick
                    || pending.frame_digest
                        != recovery
                            .sealed_patch
                            .decision()
                            .neural_evidence()?
                            .frame_digest
                {
                    return Err(ScaffoldContractError::LearningEvidenceMismatch.into());
                }
            }
            _ => return Err(ScaffoldContractError::LearningEvidenceMismatch.into()),
        }

        let mut entries = Vec::new();
        let (immutable_phenotype, entry) = self.write_json("phenotype", phenotype)?;
        entries.push(entry);
        let (phenotype_compiler_inputs, entry) =
            self.write_json("compiler-inputs", compiler_inputs)?;
        entries.push(entry);

        let neuron_count = phenotype.neuron_count();
        let synapse_count = phenotype.budgets().global.total_synapses;
        let recurrent_count = phenotype.budgets().global.recurrent_synapses;
        let decoder_count = synapse_count
            .checked_sub(recurrent_count)
            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
        let activation = activation_asset(&parts, neuron_count)?;
        let homeostasis = homeostasis_asset(&parts, neuron_count)?;
        let lifetime = weight_asset(
            &parts,
            phenotype.phenotype_hash(),
            synapse_count,
            GPU_BRAIN_WEIGHT_LAYER_LIFETIME,
        )?;
        let fast = weight_asset(
            &parts,
            phenotype.phenotype_hash(),
            synapse_count,
            GPU_BRAIN_WEIGHT_LAYER_FAST,
        )?;
        let eligibility = eligibility_asset(
            &parts,
            phenotype.phenotype_hash(),
            recurrent_count,
            decoder_count,
        )?;
        let replay = encode_portable_replay(
            phenotype.phenotype_hash(),
            phenotype.replay_capture_plan().canonical_digest(),
            parts.replay_journal_generation,
            parts.replay_journal_cursor,
            parts.replay_journal_event_count,
            &PhysicalReplayParts {
                events: parts.replay_events.clone(),
                spans: parts.replay_spans.clone(),
                samples: parts.replay_samples.clone(),
            },
        )?;
        let replay = PortableRuntimeReplayStateV1 {
            schema_version: RUNTIME_REPLAY_STATE_SCHEMA_VERSION,
            journal: replay,
            replay_patches: replay_patches.to_vec(),
        };
        validate_runtime_replay_state(handle.organism_id(), &replay)?;

        let (activation_state, entry) = self.write_json("activation", &activation)?;
        entries.push(entry);
        let (neuron_homeostasis, entry) = self.write_json("homeostasis", &homeostasis)?;
        entries.push(entry);
        let (lifetime_weights, entry) = self.write_json("lifetime", &lifetime)?;
        entries.push(entry);
        let (fast_weights, entry) = self.write_json("fast", &fast)?;
        entries.push(entry);
        let (eligibility_ref, entry) = self.write_json("eligibility", &eligibility)?;
        entries.push(entry);
        let (replay_journal, entry) = self.write_json("replay-journal", &replay)?;
        entries.push(entry);

        let pending_experience_transaction = match pending_transaction {
            Some(builder) => {
                let (asset, entry) = self.write_json(
                    "pending-transaction",
                    &PendingExperienceTransactionV1 {
                        schema_version: PENDING_TRANSACTION_SCHEMA_VERSION,
                        builder: builder.clone(),
                    },
                )?;
                entries.push(entry);
                Some(asset)
            }
            None => None,
        };

        let active_memory = sidecars.memory.export_active_bank()?;
        let staged_memory = sidecars.memory.export_staged_bank()?;
        let (active_memory_ref, entry) = self.write_json("memory-active", &active_memory)?;
        entries.push(entry);
        let staged_memory_ref = match &staged_memory {
            Some(asset) => {
                let (asset_ref, entry) = self.write_json("memory-staged", asset)?;
                entries.push(entry);
                Some(asset_ref)
            }
            None => None,
        };
        let retained_learning = match sidecars.retained_learning {
            Some(recovery) => {
                let pending =
                    pending_checkpoint.ok_or(ScaffoldContractError::LearningEvidenceMismatch)?;
                let (sealed_patch_asset, entry) =
                    self.write_json("retained-learning-patch", recovery.sealed_patch)?;
                entries.push(entry);
                let (neural_receptor_frame_asset, entry) =
                    self.write_json("retained-learning-receptors", recovery.neural_receptors)?;
                entries.push(entry);
                Some(RetainedLearningRecoverySaveState {
                    schema_version:
                        alife_world::persistence::RETAINED_LEARNING_RECOVERY_SAVE_SCHEMA_VERSION,
                    organism_id_raw: handle.organism_id().raw(),
                    pending,
                    sealed_patch_asset,
                    neural_receptor_frame_asset,
                    attempts: recovery.attempts,
                    last_error_code: recovery.last_error_code.to_string(),
                })
            }
            None => None,
        };
        let memory = MemorySidecarSaveState::from_sidecar(
            sidecars.memory,
            active_memory_ref,
            staged_memory_ref,
            retained_learning,
        )?;

        let portable_topology = sidecars.topology.export_portable()?;
        let (topology_asset_ref, entry) =
            self.write_json("topology-sidecar", &portable_topology)?;
        entries.push(entry);
        let topology =
            TopologySidecarSaveSummary::from_asset(&portable_topology, topology_asset_ref)?;

        let sleep_assets =
            self.capture_sleep_assets(backend, handle, phenotype, sleep, &mut entries)?;
        let backend_provenance = current_backend_provenance(backend, &capacity)?;
        let runtime_profile = *backend.runtime_profile();
        let activity_policy = *backend.activity_policy();
        let (activity_snapshot, throttle_sequence) = portable_activity_checkpoint(backend, handle)?;
        let (throttle_sequence_asset, entry) =
            self.write_json("throttle-sequence", &throttle_sequence)?;
        entries.push(entry);
        let last_checkpoint = throttle_sequence.last().cloned();
        let throttle_replay = ThrottleReplaySaveState::try_new(
            ThrottleReplaySaveInput {
                schema_version: THROTTLE_REPLAY_SAVE_SCHEMA_VERSION,
                policy_version: activity_policy.policy_version,
                next_sequence_cursor: activity_snapshot.next_sequence_cursor,
                last_committed_sequence_cursor: last_checkpoint
                    .as_ref()
                    .map(|checkpoint| checkpoint.sequence_cursor),
                policy_digest: activity_policy.policy_digest,
                next_completed_gpu_time_ns: activity_snapshot.next_completed_gpu_time_ns,
                brain_atp_q16: activity_snapshot.brain_atp_q16,
                last_world_atp_tick: activity_snapshot.last_world_atp_tick,
            },
            throttle_sequence_asset,
            last_checkpoint,
        )?;
        let save_state = GpuBrainSaveState {
            schema_version: GPU_BRAIN_SAVE_STATE_SCHEMA_VERSION,
            organism_id: handle.organism_id(),
            phenotype_hash: phenotype.phenotype_hash(),
            capacity_class_id: phenotype.brain_class_id(),
            sensor_profile: sidecars.sensor_profile,
            immutable_phenotype,
            phenotype_compiler_inputs,
            legacy_nano512_compatibility_receipt: sidecars
                .legacy_nano512_compatibility_receipt
                .cloned(),
            active_weight_generation: parts.active_weight_generation,
            active_weight_bank: parts.active_weight_bank,
            active_eligibility_bank: parts.active_eligibility_bank,
            learning_transaction_generation: parts.learning_transaction_generation,
            lifetime_weights,
            fast_weights,
            eligibility: eligibility_ref,
            replay_journal,
            replay_journal_generation: parts.replay_journal_generation,
            replay_journal_cursor: parts.replay_journal_cursor,
            replay_journal_event_count: parts.replay_journal_event_count,
            activation_state,
            neuron_homeostasis,
            checkpoint_tick,
            exact_cognitive_state: None,
            last_learning_replay_key: parts.last_learning_replay_key,
            pending_eligibility: pending_checkpoint,
            pending_experience_transaction,
            memory,
            topology,
            tracked_objects: sidecars.tracked_objects,
            language_grounding: sidecars.language_grounding.clone(),
            life_statistics: Some(sidecars.life_statistics.clone()),
            sleep,
            sleep_assets,
            backend_provenance,
            runtime_profile_id: runtime_profile.profile_id,
            runtime_profile_digest: runtime_profile.canonical_digest()?,
            activity_policy_version: activity_policy.policy_version,
            activity_policy_digest: activity_policy.policy_digest,
            throttle_replay,
        };
        save_state.validate()?;
        Ok(GpuBrainCheckpointWrite {
            save_state,
            manifest_entries: entries,
            checkpoint_digest,
        })
    }

    pub fn restore_brain(
        &self,
        backend: &mut GpuAuthoritativeSession,
        manifest: &AssetManifest,
        state: &GpuBrainSaveState,
    ) -> Result<RestoredGpuBrainCheckpoint, GameAppShellError> {
        state.validate()?;
        manifest.validate_with_root(self.root())?;
        state.validate_asset_manifest(manifest)?;
        let exact_cognitive_state = state
            .exact_cognitive_state
            .as_ref()
            .map(|asset| self.read_exact_cognitive_state(manifest, asset))
            .transpose()?;
        let capacity = BrainCapacityClass::production_for_id(state.capacity_class_id)?;
        let current_provenance = current_backend_provenance(backend, &capacity)?;
        state
            .backend_provenance
            .validate_portable_restore_against(&current_provenance)?;
        let runtime_profile = backend.runtime_profile();
        let activity_policy = backend.activity_policy();
        if !runtime_profile.accepts_portable_checkpoint_profile(
            state.runtime_profile_id,
            state.runtime_profile_digest,
        )? || state.activity_policy_version != activity_policy.policy_version
            || state.activity_policy_digest != activity_policy.policy_digest
        {
            return Err(ScaffoldContractError::NeuralBackendUnavailable.into());
        }
        let (throttle_sequence, _): (Vec<PortableThrottleCheckpoint>, Vec<u8>) =
            self.read_json(manifest, &state.throttle_replay.sequence_asset)?;
        for checkpoint in &throttle_sequence {
            checkpoint.validate()?;
            if checkpoint.organism_id_raw != state.organism_id.raw()
                || checkpoint.class_id_raw != state.capacity_class_id.raw()
                || checkpoint.policy_version != state.activity_policy_version
                || checkpoint.policy_digest != state.activity_policy_digest
            {
                return Err(ScaffoldContractError::BrainActivitySequenceMismatch.into());
            }
        }
        if !throttle_sequence
            .windows(2)
            .all(|pair| pair[0].sequence_cursor.checked_add(1) == Some(pair[1].sequence_cursor))
            || throttle_sequence.last() != state.throttle_replay.last_checkpoint.as_ref()
            || throttle_sequence.is_empty() != state.throttle_replay.last_checkpoint.is_none()
        {
            return Err(ScaffoldContractError::BrainActivitySequenceMismatch.into());
        }
        let (compiler_inputs, _): (PhenotypeCompilerInputs, Vec<u8>) =
            self.read_json(manifest, &state.phenotype_compiler_inputs)?;
        compiler_inputs.validate_against(&capacity)?;
        let (phenotype, phenotype_bytes): (BrainPhenotype, Vec<u8>) =
            self.read_json(manifest, &state.immutable_phenotype)?;
        phenotype.validate_against(&capacity)?;
        let recompiled = PhenotypeCompiler::compile_validated(&compiler_inputs, &capacity)?;
        if phenotype.phenotype_hash() != state.phenotype_hash
            || phenotype.brain_class_id() != state.capacity_class_id
            || phenotype.compiler_inputs_digest() != compiler_inputs.canonical_digest()
            || recompiled != phenotype
            || serde_json::to_vec(&recompiled)? != phenotype_bytes
        {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        match (
            phenotype.legacy_foundation_compatibility_abi(),
            state.legacy_nano512_compatibility_receipt.as_ref(),
        ) {
            (Some(_), Some(receipt)) => {
                let asset = FoundationWeightAsset::builtin_nano512_v1(phenotype.sensor_profile())?;
                receipt.validate_against(&phenotype, &asset)?;
            }
            (None, None) => {}
            _ => return Err(ScaffoldContractError::PhenotypeCompile.into()),
        }
        let phenotype_profile = SensorProfileIdentity {
            profile_id: phenotype.sensor_profile().into(),
            profile_schema_version: 1,
            sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
        };
        if state.sensor_profile != phenotype_profile {
            return Err(ScaffoldContractError::SensorProfileMismatch.into());
        }

        let (active_memory, _): (PortableMemoryBankAssetV2, Vec<u8>) =
            self.read_json(manifest, &state.memory.compaction.active_bank_asset)?;
        let staged_memory = match &state.memory.compaction.staged_bank_asset {
            Some(asset_ref) => {
                let (asset, _): (PortableMemoryBankAssetV2, Vec<u8>) =
                    self.read_json(manifest, asset_ref)?;
                Some(asset)
            }
            None => None,
        };
        validate_memory_assets(state, &active_memory, staged_memory.as_ref())?;
        let memory = MemorySidecarState::restore_portable(
            state.sensor_profile,
            state.memory.compaction.checkpoint,
            active_memory,
            staged_memory,
        )?;

        let (portable_topology, _): (PortableTopologySidecarAssetV1, Vec<u8>) =
            self.read_json(manifest, &state.topology.summary_asset)?;
        let reconstructed_topology = TopologySidecarSaveSummary::from_asset(
            &portable_topology,
            state.topology.summary_asset.clone(),
        )?;
        if reconstructed_topology != state.topology {
            return Err(ScaffoldContractError::InvalidMemoryQuery.into());
        }
        let topology = TopologySidecar::restore_portable(portable_topology)?;

        let retained_learning = match &state.memory.retained_learning {
            Some(recovery) => {
                let (sealed_patch, _): (ExperiencePatch, Vec<u8>) =
                    self.read_json(manifest, &recovery.sealed_patch_asset)?;
                let (neural_receptors, _): (NeuralReceptorFrame, Vec<u8>) =
                    self.read_json(manifest, &recovery.neural_receptor_frame_asset)?;
                sealed_patch.validate_contract()?;
                neural_receptors.validate_contract()?;
                if sealed_patch.pre_action().organism_id != state.organism_id
                    || sealed_patch.header().sensor_profile.identity() != state.sensor_profile
                    || state.pending_eligibility != Some(recovery.pending)
                    || recovery.pending.originating_tick != sealed_patch.pre_action().tick
                    || recovery.pending.frame_digest
                        != sealed_patch.decision().neural_evidence()?.frame_digest
                    || neural_receptors.source_tick != sealed_patch.pre_action().tick
                {
                    return Err(ScaffoldContractError::LearningEvidenceMismatch.into());
                }
                Some(RestoredRetainedLearning {
                    sealed_patch,
                    neural_receptors,
                    attempts: recovery.attempts,
                    last_error_code: recovery.last_error_code.clone(),
                })
            }
            None => None,
        };

        let (activation, _): (PortableActivationBanksV1, Vec<u8>) =
            self.read_json(manifest, &state.activation_state)?;
        let (homeostasis, _): (PortableNeuronHomeostasisV1, Vec<u8>) =
            self.read_json(manifest, &state.neuron_homeostasis)?;
        let (lifetime, _): (PortableDualWeightBankV1, Vec<u8>) =
            self.read_json(manifest, &state.lifetime_weights)?;
        let (fast, _): (PortableDualWeightBankV1, Vec<u8>) =
            self.read_json(manifest, &state.fast_weights)?;
        let (eligibility, _): (PortableEligibilityBanksV1, Vec<u8>) =
            self.read_json(manifest, &state.eligibility)?;
        let (replay_state, _): (PortableRuntimeReplayStateWire, Vec<u8>) =
            self.read_json(manifest, &state.replay_journal)?;
        let (replay, replay_patches) = match replay_state {
            PortableRuntimeReplayStateWire::Runtime(replay_state) => {
                validate_runtime_replay_state(state.organism_id, &replay_state)?;
                (replay_state.journal, replay_state.replay_patches)
            }
            PortableRuntimeReplayStateWire::Legacy(replay) => {
                if replay.event_count != 0 {
                    return Err(ScaffoldContractError::MissingPhaseData.into());
                }
                (replay, Vec::new())
            }
        };
        validate_main_assets(
            state,
            &phenotype,
            &activation,
            &homeostasis,
            &lifetime,
            &fast,
            &eligibility,
            &replay,
        )?;
        let physical = decode_physical_replay(&replay)?;
        let pending = state
            .pending_eligibility
            .map(pending_restore_parts)
            .transpose()?;
        let checkpoint = GpuBrainCheckpointSnapshot::try_from_parts(GpuBrainCheckpointParts {
            schema_version: GPU_BRAIN_CHECKPOINT_SCHEMA_VERSION,
            organism_id: state.organism_id,
            phenotype_hash: state.phenotype_hash,
            checkpoint_tick: state.checkpoint_tick,
            active_activation_side: activation.active_side,
            logical_dispatch_generation: activation.logical_dispatch_generation,
            activation_a_bits: activation.activation_a_bits,
            activation_b_bits: activation.activation_b_bits,
            neuron_homeostasis_bits: homeostasis.value_bits,
            active_weight_generation: state.active_weight_generation,
            active_weight_bank: state.active_weight_bank,
            lifetime_bank_0_bits: lifetime.bank_0_bits,
            lifetime_bank_1_bits: lifetime.bank_1_bits,
            fast_bank_0_bits: fast.bank_0_bits,
            fast_bank_1_bits: fast.bank_1_bits,
            active_eligibility_generation: eligibility.active_generation,
            inactive_eligibility_generation: eligibility.inactive_generation,
            active_eligibility_bank: eligibility.active_bank,
            learning_transaction_generation: state.learning_transaction_generation,
            recurrent_eligibility_bank_0_bits: eligibility.recurrent_bank_0_bits,
            recurrent_eligibility_bank_1_bits: eligibility.recurrent_bank_1_bits,
            decoder_eligibility_bank_0_bits: eligibility.decoder_bank_0_bits,
            decoder_eligibility_bank_1_bits: eligibility.decoder_bank_1_bits,
            replay_journal_generation: replay.generation,
            replay_journal_cursor: replay.cursor,
            replay_journal_event_count: replay.event_count,
            replay_events: physical.events,
            replay_spans: physical.spans,
            replay_samples: physical.samples,
            last_learning_replay_key: state.last_learning_replay_key,
            pending_eligibility: pending,
        })?;
        let request = GpuBrainRestoreRequest::try_new(checkpoint)?;
        let receipt = backend.restore_brain(state.organism_id, phenotype.clone(), request)?;
        if let Err(error) = backend.restore_activity_state(
            receipt.handle,
            GpuActivityRestoreInput {
                next_sequence_cursor: state.throttle_replay.next_sequence_cursor,
                checkpoint_tick: state.checkpoint_tick.raw(),
                next_completed_gpu_time_ns: state.throttle_replay.next_completed_gpu_time_ns,
                brain_atp_q16: state.throttle_replay.brain_atp_q16,
                last_world_atp_tick: state.throttle_replay.last_world_atp_tick,
                record: state
                    .throttle_replay
                    .last_checkpoint
                    .as_ref()
                    .map(activity_restore_record),
            },
        ) {
            if let Some(pending) = receipt.pending_eligibility {
                let _ = backend.discard_pending_eligibility(receipt.handle, pending.identity());
            }
            let _ = backend.remove_brain(receipt.handle);
            return Err(error.into());
        }

        let pending_transaction = match &state.pending_experience_transaction {
            Some(asset_ref) => {
                let (pending, _): (PendingExperienceTransactionV1, Vec<u8>) =
                    self.read_json(manifest, asset_ref)?;
                if pending.schema_version != PENDING_TRANSACTION_SCHEMA_VERSION {
                    return Err(ScaffoldContractError::LearningEvidenceMismatch.into());
                }
                let saved_pending = state
                    .pending_eligibility
                    .ok_or(ScaffoldContractError::LearningEvidenceMismatch)?;
                validate_pending_transaction(&pending.builder, receipt.handle, saved_pending)?;
                Some(pending.builder)
            }
            None => None,
        };
        let pending_shape_valid = if retained_learning.is_some() {
            pending_transaction.is_none() && receipt.pending_eligibility.is_some()
        } else {
            pending_transaction.is_some() == receipt.pending_eligibility.is_some()
        };
        if !pending_shape_valid {
            return Err(ScaffoldContractError::LearningEvidenceMismatch.into());
        }

        self.restore_sleep_assets(backend, manifest, state, receipt.handle, &phenotype)?;
        Ok(RestoredGpuBrainCheckpoint {
            receipt,
            phenotype,
            compiler_inputs,
            legacy_nano512_compatibility_receipt: state
                .legacy_nano512_compatibility_receipt
                .clone(),
            sleep: state.sleep,
            pending_transaction,
            memory,
            topology,
            tracked_objects: state.tracked_objects.clone(),
            language_grounding: state.language_grounding.clone(),
            life_statistics: state.life_statistics.clone(),
            retained_learning,
            exact_cognitive_state,
            replay_patches,
        })
    }

    fn capture_sleep_assets(
        &self,
        backend: &mut GpuClosedLoopBackend,
        handle: GpuBrainHandle,
        phenotype: &BrainPhenotype,
        sleep: SleepState,
        entries: &mut Vec<AssetManifestEntry>,
    ) -> Result<GpuSleepAssetState, GameAppShellError> {
        let mut assets = GpuSleepAssetState::default();
        let needs_replay = !matches!(
            sleep.consolidation,
            ConsolidationState::None | ConsolidationState::Committed { .. }
        );
        let replay = if needs_replay {
            let replay = backend.build_sleep_replay_batch(handle)?;
            validate_sleep_replay_state(sleep.consolidation, &replay)?;
            let (asset_ref, entry) = self.write_json("sleep-replay", &replay)?;
            entries.push(entry);
            assets.replay_batch = Some(asset_ref);
            Some(replay)
        } else {
            None
        };

        if let ConsolidationState::Completed { request, staged } = sleep.consolidation {
            let replay = replay
                .as_ref()
                .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
            if replay.canonical_digest != request.replay_digest {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
            }
            let staging = backend.snapshot_completed_sleep_staging(handle, &request, &staged)?;
            let staging = staging.into_parts();
            let synapse_count = phenotype.budgets().global.total_synapses;
            let recurrent_count = phenotype.budgets().global.recurrent_synapses;
            let decoder_count = synapse_count
                .checked_sub(recurrent_count)
                .ok_or(ScaffoldContractError::PhenotypeCompile)?;
            let lifetime = staged_weight_asset(
                &staging,
                phenotype.phenotype_hash(),
                synapse_count,
                GPU_BRAIN_WEIGHT_LAYER_LIFETIME,
            )?;
            let fast = staged_weight_asset(
                &staging,
                phenotype.phenotype_hash(),
                synapse_count,
                GPU_BRAIN_WEIGHT_LAYER_FAST,
            )?;
            let eligibility = staged_eligibility_asset(
                &staging,
                phenotype.phenotype_hash(),
                recurrent_count,
                decoder_count,
            )?;
            let journal = encode_portable_replay(
                phenotype.phenotype_hash(),
                phenotype.replay_capture_plan().canonical_digest(),
                staging.replay_journal_generation,
                staging.replay_journal_cursor,
                staging.replay_journal_event_count,
                &PhysicalReplayParts {
                    events: staging.replay_events,
                    spans: staging.replay_spans,
                    samples: staging.replay_samples,
                },
            )?;
            let (asset, entry) = self.write_json("lifetime-staging", &lifetime)?;
            assets.lifetime_staging = Some(asset);
            entries.push(entry);
            let (asset, entry) = self.write_json("fast-staging", &fast)?;
            assets.fast_staging = Some(asset);
            entries.push(entry);
            let (asset, entry) = self.write_json("eligibility-staging", &eligibility)?;
            assets.eligibility_staging = Some(asset);
            entries.push(entry);
            let (asset, entry) = self.write_json("replay-staging", &journal)?;
            assets.replay_journal_staging = Some(asset);
            entries.push(entry);
        }
        Ok(assets)
    }

    fn restore_sleep_assets(
        &self,
        backend: &mut GpuClosedLoopBackend,
        manifest: &AssetManifest,
        state: &GpuBrainSaveState,
        handle: GpuBrainHandle,
        phenotype: &BrainPhenotype,
    ) -> Result<(), GameAppShellError> {
        let replay = match &state.sleep_assets.replay_batch {
            Some(asset_ref) => {
                let (replay, _): (BoundedReplayBatch, Vec<u8>) =
                    self.read_json(manifest, asset_ref)?;
                replay.validate_contract(
                    phenotype.budgets().global.replay_event_capacity,
                    phenotype
                        .budgets()
                        .global
                        .replay_eligibility_sample_capacity,
                    phenotype.budgets().global.total_synapses,
                )?;
                validate_sleep_replay_state(state.sleep.consolidation, &replay)?;
                Some(replay)
            }
            None => None,
        };
        if let ConsolidationState::Completed { request, staged } = state.sleep.consolidation {
            let replay = replay
                .as_ref()
                .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
            let (lifetime, _): (PortableDualWeightBankV1, Vec<u8>) = self.read_json(
                manifest,
                state
                    .sleep_assets
                    .lifetime_staging
                    .as_ref()
                    .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?,
            )?;
            let (fast, _): (PortableDualWeightBankV1, Vec<u8>) = self.read_json(
                manifest,
                state
                    .sleep_assets
                    .fast_staging
                    .as_ref()
                    .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?,
            )?;
            let (eligibility, _): (PortableEligibilityBanksV1, Vec<u8>) = self.read_json(
                manifest,
                state
                    .sleep_assets
                    .eligibility_staging
                    .as_ref()
                    .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?,
            )?;
            let (journal, _): (PortableReplayJournalV1, Vec<u8>) = self.read_json(
                manifest,
                state
                    .sleep_assets
                    .replay_journal_staging
                    .as_ref()
                    .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?,
            )?;
            validate_staging_assets(
                state,
                phenotype,
                &lifetime,
                &fast,
                &eligibility,
                &journal,
                &request,
                &staged,
            )?;
            let physical = decode_physical_replay(&journal)?;
            let parts = GpuCompletedSleepStagingParts::try_from_parts(
                GpuCompletedSleepStagingInputParts {
                    output_weight_generation: lifetime.active_generation,
                    output_weight_bank: lifetime.active_bank,
                    lifetime_bank_0_bits: lifetime.bank_0_bits,
                    lifetime_bank_1_bits: lifetime.bank_1_bits,
                    fast_bank_0_bits: fast.bank_0_bits,
                    fast_bank_1_bits: fast.bank_1_bits,
                    eligibility_reset_generation: eligibility.active_generation,
                    output_eligibility_bank: eligibility.active_bank,
                    recurrent_eligibility_bank_0_bits: eligibility.recurrent_bank_0_bits,
                    recurrent_eligibility_bank_1_bits: eligibility.recurrent_bank_1_bits,
                    decoder_eligibility_bank_0_bits: eligibility.decoder_bank_0_bits,
                    decoder_eligibility_bank_1_bits: eligibility.decoder_bank_1_bits,
                    replay_journal_generation: journal.generation,
                    replay_journal_cursor: journal.cursor,
                    replay_journal_event_count: journal.event_count,
                    replay_events: physical.events,
                    replay_spans: physical.spans,
                    replay_samples: physical.samples,
                },
            )?;
            backend.restore_completed_sleep_staging(handle, &request, replay, &staged, parts)?;
        }
        Ok(())
    }
}

fn activation_asset(
    parts: &GpuBrainCheckpointParts,
    neuron_count: u32,
) -> Result<PortableActivationBanksV1, ScaffoldContractError> {
    let mut asset = PortableActivationBanksV1 {
        schema_version: GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION,
        phenotype_hash: parts.phenotype_hash,
        neuron_count,
        active_side: parts.active_activation_side,
        logical_dispatch_generation: parts.logical_dispatch_generation,
        activation_a_bits: parts.activation_a_bits.clone(),
        activation_b_bits: parts.activation_b_bits.clone(),
        canonical_digest: [0; 4],
    };
    asset.canonical_digest = asset.recompute_canonical_digest()?;
    asset.validate()?;
    Ok(asset)
}

fn validate_memory_assets(
    state: &GpuBrainSaveState,
    active: &PortableMemoryBankAssetV2,
    staged: Option<&PortableMemoryBankAssetV2>,
) -> Result<(), ScaffoldContractError> {
    active.validate_contract()?;
    if let Some(asset) = staged {
        asset.validate_contract()?;
        if asset.organism_id_raw != active.organism_id_raw
            || asset.profile != active.profile
            || asset.capacity != active.capacity
            || asset.max_feature_len != active.max_feature_len
            || asset.max_match_count != active.max_match_count
            || asset.min_match_score_bits != active.min_match_score_bits
            || asset.empty_confidence_bits != active.empty_confidence_bits
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
    }
    let summary = &state.memory.summary;
    let record_count = u32::try_from(active.records.len())
        .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
    if active.organism_id_raw != state.organism_id.raw()
        || active.profile != state.sensor_profile
        || summary.organism_id_raw != active.organism_id_raw
        || summary.profile != active.profile
        || summary.capacity != active.capacity
        || summary.record_count != record_count
        || summary.merge_count != active.merge_count
        || summary.eviction_count != active.eviction_count
        || summary.compaction_count
            != state
                .memory
                .compaction
                .checkpoint
                .last_committed_cycle_id
                .unwrap_or(0)
        || summary.active_generation != active.generation
        || summary.active_digest != active.active_bank_digest
    {
        return Err(ScaffoldContractError::InvalidMemoryQuery);
    }
    Ok(())
}

fn homeostasis_asset(
    parts: &GpuBrainCheckpointParts,
    neuron_count: u32,
) -> Result<PortableNeuronHomeostasisV1, ScaffoldContractError> {
    let mut asset = PortableNeuronHomeostasisV1 {
        schema_version: GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION,
        phenotype_hash: parts.phenotype_hash,
        neuron_count,
        lanes_per_neuron: GPU_BRAIN_HOMEOSTASIS_LANES_PER_NEURON,
        value_bits: parts.neuron_homeostasis_bits.clone(),
        canonical_digest: [0; 4],
    };
    asset.canonical_digest = asset.recompute_canonical_digest()?;
    asset.validate()?;
    Ok(asset)
}

fn weight_asset(
    parts: &GpuBrainCheckpointParts,
    phenotype_hash: alife_core::PhenotypeHash,
    synapse_count: u32,
    layer_raw: u16,
) -> Result<PortableDualWeightBankV1, ScaffoldContractError> {
    let (bank_0_bits, bank_1_bits) = match layer_raw {
        GPU_BRAIN_WEIGHT_LAYER_LIFETIME => (
            parts.lifetime_bank_0_bits.clone(),
            parts.lifetime_bank_1_bits.clone(),
        ),
        GPU_BRAIN_WEIGHT_LAYER_FAST => (
            parts.fast_bank_0_bits.clone(),
            parts.fast_bank_1_bits.clone(),
        ),
        _ => return Err(ScaffoldContractError::ConsolidationGenerationMismatch),
    };
    make_weight_asset(
        phenotype_hash,
        synapse_count,
        layer_raw,
        parts.active_weight_generation,
        parts.active_weight_bank,
        bank_0_bits,
        bank_1_bits,
    )
}

fn staged_weight_asset(
    parts: &GpuCompletedSleepStagingInputParts,
    phenotype_hash: alife_core::PhenotypeHash,
    synapse_count: u32,
    layer_raw: u16,
) -> Result<PortableDualWeightBankV1, ScaffoldContractError> {
    let (bank_0_bits, bank_1_bits) = match layer_raw {
        GPU_BRAIN_WEIGHT_LAYER_LIFETIME => (
            parts.lifetime_bank_0_bits.clone(),
            parts.lifetime_bank_1_bits.clone(),
        ),
        GPU_BRAIN_WEIGHT_LAYER_FAST => (
            parts.fast_bank_0_bits.clone(),
            parts.fast_bank_1_bits.clone(),
        ),
        _ => return Err(ScaffoldContractError::ConsolidationGenerationMismatch),
    };
    make_weight_asset(
        phenotype_hash,
        synapse_count,
        layer_raw,
        parts.output_weight_generation,
        parts.output_weight_bank,
        bank_0_bits,
        bank_1_bits,
    )
}

#[allow(clippy::too_many_arguments)]
fn make_weight_asset(
    phenotype_hash: alife_core::PhenotypeHash,
    synapse_count: u32,
    layer_raw: u16,
    active_generation: u64,
    active_bank: u8,
    bank_0_bits: Vec<u32>,
    bank_1_bits: Vec<u32>,
) -> Result<PortableDualWeightBankV1, ScaffoldContractError> {
    let mut asset = PortableDualWeightBankV1 {
        schema_version: GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION,
        layer_raw,
        phenotype_hash,
        synapse_count,
        active_generation,
        active_bank,
        bank_0_bits,
        bank_1_bits,
        canonical_digest: [0; 4],
    };
    asset.canonical_digest = asset.recompute_canonical_digest()?;
    asset.validate()?;
    Ok(asset)
}

fn eligibility_asset(
    parts: &GpuBrainCheckpointParts,
    phenotype_hash: alife_core::PhenotypeHash,
    recurrent_count: u32,
    decoder_count: u32,
) -> Result<PortableEligibilityBanksV1, ScaffoldContractError> {
    make_eligibility_asset(
        phenotype_hash,
        recurrent_count,
        decoder_count,
        parts.active_eligibility_generation,
        parts.inactive_eligibility_generation,
        parts.active_eligibility_bank,
        parts.recurrent_eligibility_bank_0_bits.clone(),
        parts.recurrent_eligibility_bank_1_bits.clone(),
        parts.decoder_eligibility_bank_0_bits.clone(),
        parts.decoder_eligibility_bank_1_bits.clone(),
    )
}

fn staged_eligibility_asset(
    parts: &GpuCompletedSleepStagingInputParts,
    phenotype_hash: alife_core::PhenotypeHash,
    recurrent_count: u32,
    decoder_count: u32,
) -> Result<PortableEligibilityBanksV1, ScaffoldContractError> {
    make_eligibility_asset(
        phenotype_hash,
        recurrent_count,
        decoder_count,
        parts.eligibility_reset_generation,
        0,
        parts.output_eligibility_bank,
        parts.recurrent_eligibility_bank_0_bits.clone(),
        parts.recurrent_eligibility_bank_1_bits.clone(),
        parts.decoder_eligibility_bank_0_bits.clone(),
        parts.decoder_eligibility_bank_1_bits.clone(),
    )
}

#[allow(clippy::too_many_arguments)]
fn make_eligibility_asset(
    phenotype_hash: alife_core::PhenotypeHash,
    recurrent_count: u32,
    decoder_count: u32,
    active_generation: u64,
    inactive_generation: u64,
    active_bank: u8,
    recurrent_bank_0_bits: Vec<u32>,
    recurrent_bank_1_bits: Vec<u32>,
    decoder_bank_0_bits: Vec<u32>,
    decoder_bank_1_bits: Vec<u32>,
) -> Result<PortableEligibilityBanksV1, ScaffoldContractError> {
    let mut asset = PortableEligibilityBanksV1 {
        schema_version: GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION,
        phenotype_hash,
        recurrent_count,
        decoder_count,
        active_generation,
        inactive_generation,
        active_bank,
        recurrent_bank_0_bits,
        recurrent_bank_1_bits,
        decoder_bank_0_bits,
        decoder_bank_1_bits,
        canonical_digest: [0; 4],
    };
    asset.canonical_digest = asset.recompute_canonical_digest()?;
    asset.validate()?;
    Ok(asset)
}

fn pending_checkpoint_from_parts(
    parts: PendingEligibilityRestoreParts,
) -> Result<PendingEligibilityCheckpoint, ScaffoldContractError> {
    PendingEligibilityCheckpoint::try_new(
        parts.dispatch_generation(),
        parts.originating_tick(),
        parts.frame_digest(),
        parts.active_activation_side(),
        parts.candidate_index(),
        parts.action_id(),
        parts.action_family(),
        parts.candidate_feature_digest(),
        parts.active_eligibility_generation(),
        parts.staging_eligibility_generation(),
    )
}

fn pending_restore_parts(
    pending: PendingEligibilityCheckpoint,
) -> Result<PendingEligibilityRestoreParts, ScaffoldContractError> {
    PendingEligibilityRestoreParts::try_new(
        pending.dispatch_generation,
        pending.originating_tick,
        pending.frame_digest,
        pending.active_activation_side,
        pending.candidate_index,
        pending.action_id,
        pending.action_family,
        pending.candidate_feature_digest,
        pending.active_eligibility_generation,
        pending.staging_eligibility_generation,
    )
}

fn validate_pending_transaction(
    builder: &ExperiencePatchBuilder,
    handle: GpuBrainHandle,
    pending: PendingEligibilityCheckpoint,
) -> Result<(), ScaffoldContractError> {
    let (pre_action, decision) = builder.pending_decision()?;
    let evidence = decision.neural_evidence()?;
    if pre_action.organism_id != handle.organism_id()
        || evidence.phenotype_hash != handle.phenotype_hash()
        || evidence.dispatch_generation != pending.dispatch_generation
        || evidence.frame_digest != pending.frame_digest
        || evidence.active_activation_side != pending.active_activation_side
        || evidence.candidate_index != pending.candidate_index
        || evidence.action_id != pending.action_id
        || evidence.action_family != pending.action_family
        || evidence.candidate_feature_digest != pending.candidate_feature_digest
    {
        return Err(ScaffoldContractError::LearningEvidenceMismatch);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_main_assets(
    state: &GpuBrainSaveState,
    phenotype: &BrainPhenotype,
    activation: &PortableActivationBanksV1,
    homeostasis: &PortableNeuronHomeostasisV1,
    lifetime: &PortableDualWeightBankV1,
    fast: &PortableDualWeightBankV1,
    eligibility: &PortableEligibilityBanksV1,
    replay: &PortableReplayJournalV1,
) -> Result<(), ScaffoldContractError> {
    activation.validate()?;
    homeostasis.validate()?;
    lifetime.validate()?;
    fast.validate()?;
    eligibility.validate()?;
    replay.validate()?;
    let budget = &phenotype.budgets().global;
    if activation.phenotype_hash != state.phenotype_hash
        || activation.neuron_count != phenotype.neuron_count()
        || homeostasis.phenotype_hash != state.phenotype_hash
        || homeostasis.neuron_count != phenotype.neuron_count()
        || lifetime.layer_raw != GPU_BRAIN_WEIGHT_LAYER_LIFETIME
        || fast.layer_raw != GPU_BRAIN_WEIGHT_LAYER_FAST
        || lifetime.phenotype_hash != state.phenotype_hash
        || fast.phenotype_hash != state.phenotype_hash
        || lifetime.synapse_count != budget.total_synapses
        || fast.synapse_count != budget.total_synapses
        || lifetime.active_generation != state.active_weight_generation
        || fast.active_generation != state.active_weight_generation
        || lifetime.active_bank != state.active_weight_bank
        || fast.active_bank != state.active_weight_bank
        || eligibility.phenotype_hash != state.phenotype_hash
        || eligibility.recurrent_count != budget.recurrent_synapses
        || eligibility.decoder_count
            != budget
                .total_synapses
                .saturating_sub(budget.recurrent_synapses)
        || eligibility.active_bank != state.active_eligibility_bank
        || replay.phenotype_hash != state.phenotype_hash
        || replay.replay_capture_plan_digest != phenotype.replay_capture_plan().canonical_digest()
        || replay.generation != state.replay_journal_generation
        || replay.cursor != state.replay_journal_cursor
        || replay.event_count != state.replay_journal_event_count
        || replay.event_capacity != budget.replay_event_capacity
        || replay.sample_capacity != budget.replay_eligibility_sample_capacity
    {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    if let Some(pending) = state.pending_eligibility {
        if pending.active_activation_side != activation.active_side
            || pending.dispatch_generation != activation.logical_dispatch_generation
            || pending.active_eligibility_generation != eligibility.active_generation
            || pending.staging_eligibility_generation != eligibility.inactive_generation
        {
            return Err(ScaffoldContractError::LearningEvidenceMismatch);
        }
    } else if eligibility.inactive_generation != 0 {
        return Err(ScaffoldContractError::LearningEvidenceMismatch);
    }
    Ok(())
}

fn validate_runtime_replay_state(
    organism_id: OrganismId,
    state: &PortableRuntimeReplayStateV1,
) -> Result<(), ScaffoldContractError> {
    state.journal.validate()?;
    if state.schema_version != RUNTIME_REPLAY_STATE_SCHEMA_VERSION
        || state.replay_patches.len() != state.journal.events.len()
    {
        return Err(ScaffoldContractError::MissingPhaseData);
    }
    for (event, patch) in state.journal.events.iter().zip(&state.replay_patches) {
        patch.validate_contract()?;
        let target = patch
            .prediction_target()
            .ok_or(ScaffoldContractError::MissingPhaseData)?;
        target.validate_contract()?;
        if patch.header().organism_id != organism_id
            || target.organism_id != organism_id
            || target.experience_sequence != event.sequence_id
            || target.decision != event.action_id
            || target.source_digest != event.frame_digest.0
            || target.world_tick.raw() < event.originating_tick.raw()
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
    }
    Ok(())
}

fn validate_sleep_replay_state(
    state: ConsolidationState,
    replay: &BoundedReplayBatch,
) -> Result<(), ScaffoldContractError> {
    match state {
        ConsolidationState::Pending {
            replay_digest,
            replay_event_count,
            replay_eligibility_sample_count,
            ..
        } if replay.canonical_digest == replay_digest
            && replay.events.len() as u32 == replay_event_count
            && replay.eligibility_samples.len() as u32 == replay_eligibility_sample_count =>
        {
            Ok(())
        }
        ConsolidationState::Prepared { request }
        | ConsolidationState::Submitted { request, .. }
        | ConsolidationState::Completed { request, .. }
            if replay.canonical_digest == request.replay_digest
                && replay.events.len() <= request.max_replay_events as usize
                && replay.eligibility_samples.len()
                    <= request.max_replay_eligibility_samples as usize =>
        {
            Ok(())
        }
        _ => Err(ScaffoldContractError::ConsolidationGenerationMismatch),
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_staging_assets(
    state: &GpuBrainSaveState,
    phenotype: &BrainPhenotype,
    lifetime: &PortableDualWeightBankV1,
    fast: &PortableDualWeightBankV1,
    eligibility: &PortableEligibilityBanksV1,
    journal: &PortableReplayJournalV1,
    request: &alife_core::GpuConsolidationRequest,
    staged: &alife_core::ConsolidationStagedOutput,
) -> Result<(), ScaffoldContractError> {
    lifetime.validate()?;
    fast.validate()?;
    eligibility.validate()?;
    journal.validate()?;
    let budget = &phenotype.budgets().global;
    let zero = |words: &[u32]| words.iter().all(|word| *word == 0);
    if lifetime.layer_raw != GPU_BRAIN_WEIGHT_LAYER_LIFETIME
        || fast.layer_raw != GPU_BRAIN_WEIGHT_LAYER_FAST
        || lifetime.phenotype_hash != state.phenotype_hash
        || fast.phenotype_hash != state.phenotype_hash
        || lifetime.synapse_count != budget.total_synapses
        || fast.synapse_count != budget.total_synapses
        || lifetime.active_generation != staged.output_generation
        || fast.active_generation != staged.output_generation
        || lifetime.active_bank != staged.output_weight_bank
        || fast.active_bank != staged.output_weight_bank
        || eligibility.phenotype_hash != state.phenotype_hash
        || eligibility.active_generation != staged.eligibility_reset_generation
        || eligibility.inactive_generation != 0
        || eligibility.active_bank != staged.output_eligibility_bank
        || !zero(&eligibility.recurrent_bank_0_bits)
        || !zero(&eligibility.recurrent_bank_1_bits)
        || !zero(&eligibility.decoder_bank_0_bits)
        || !zero(&eligibility.decoder_bank_1_bits)
        || journal.phenotype_hash != state.phenotype_hash
        || journal.replay_capture_plan_digest != phenotype.replay_capture_plan().canonical_digest()
        || journal.generation != staged.replay_journal_generation
        || journal.cursor != staged.replay_journal_cursor
        || journal.event_count != staged.replay_journal_event_count
        || !journal.events.is_empty()
        || !journal.eligibility_samples.is_empty()
        || request.expected_output_generation != staged.output_generation
    {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    Ok(())
}
