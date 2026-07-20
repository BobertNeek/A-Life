//! Exact portable checkpoint construction and validated GPU restore.

use alife_core::{
    BoundedReplayBatch, BrainCapacityClass, BrainPhenotype, ConsolidationState, ExperiencePatch,
    ExperiencePatchBuilder, MemorySidecarState, PhenotypeCompiler, PhenotypeCompilerInputs,
    PortableMemoryBankAssetV2, PortableTopologySidecarAssetV1, ScaffoldContractError,
    SensorProfileIdentity, SensoryAbiVersion, SleepState, Tick, TopologySidecar, Validate,
};
use alife_gpu_backend::{
    GpuBrainCheckpointParts, GpuBrainCheckpointSnapshot, GpuBrainHandle, GpuBrainRestoreReceipt,
    GpuBrainRestoreRequest, GpuClosedLoopBackend, GpuCompletedSleepStagingInputParts,
    GpuCompletedSleepStagingParts, GpuReplayEventRecord, GpuReplaySynapseSpanRecord,
    PendingEligibilityRestoreParts, GPU_BRAIN_CHECKPOINT_SCHEMA_VERSION,
};
use alife_world::persistence::{
    AssetManifest, AssetManifestEntry, GpuBrainSaveState, GpuSleepAssetState,
    MemorySidecarSaveState, PendingEligibilityCheckpoint, PortableActivationBanksV1,
    PortableDualWeightBankV1, PortableEligibilityBanksV1, PortableNeuronHomeostasisV1,
    PortableReplayJournalV1, RetainedLearningRecoverySaveState, TopologySidecarSaveSummary,
    GPU_BRAIN_HOMEOSTASIS_LANES_PER_NEURON, GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION,
    GPU_BRAIN_SAVE_STATE_SCHEMA_VERSION, GPU_BRAIN_WEIGHT_LAYER_FAST,
    GPU_BRAIN_WEIGHT_LAYER_LIFETIME,
};
use alife_world::TrackedObjectRegistrySaveState;
use serde::{Deserialize, Serialize};

use crate::GameAppShellError;

use super::{
    content_store::GpuCheckpointAssetStore,
    replay_codec::{decode_physical_replay, encode_portable_replay},
};

const PENDING_TRANSACTION_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq)]
pub struct GpuBrainCheckpointWrite {
    pub save_state: GpuBrainSaveState,
    pub manifest_entries: Vec<AssetManifestEntry>,
    pub checkpoint_digest: [u64; 4],
}

#[derive(Debug)]
pub struct RestoredGpuBrainCheckpoint {
    pub receipt: GpuBrainRestoreReceipt,
    pub phenotype: BrainPhenotype,
    pub compiler_inputs: PhenotypeCompilerInputs,
    pub sleep: SleepState,
    pub pending_transaction: Option<ExperiencePatchBuilder>,
    pub memory: MemorySidecarState,
    pub topology: TopologySidecar,
    pub tracked_objects: TrackedObjectRegistrySaveState,
    pub retained_learning: Option<RestoredRetainedLearning>,
}

pub struct GpuBrainSidecarCapture<'a> {
    pub sensor_profile: SensorProfileIdentity,
    pub memory: &'a MemorySidecarState,
    pub topology: &'a TopologySidecar,
    pub tracked_objects: TrackedObjectRegistrySaveState,
    pub retained_learning: Option<RetainedLearningCapture<'a>>,
}

pub struct RetainedLearningCapture<'a> {
    pub sealed_patch: &'a ExperiencePatch,
    pub attempts: u8,
    pub last_error_code: &'static str,
}

#[derive(Debug)]
pub struct RestoredRetainedLearning {
    pub sealed_patch: ExperiencePatch,
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

impl GpuCheckpointAssetStore {
    #[allow(clippy::too_many_arguments)]
    pub fn capture_brain(
        &self,
        backend: &mut GpuClosedLoopBackend,
        handle: GpuBrainHandle,
        phenotype: &BrainPhenotype,
        compiler_inputs: &PhenotypeCompilerInputs,
        sleep: SleepState,
        checkpoint_tick: Tick,
        pending_transaction: Option<&ExperiencePatchBuilder>,
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
                Some(RetainedLearningRecoverySaveState {
                    schema_version:
                        alife_world::persistence::RETAINED_LEARNING_RECOVERY_SAVE_SCHEMA_VERSION,
                    organism_id_raw: handle.organism_id().raw(),
                    pending,
                    sealed_patch_asset,
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
        let save_state = GpuBrainSaveState {
            schema_version: GPU_BRAIN_SAVE_STATE_SCHEMA_VERSION,
            organism_id: handle.organism_id(),
            phenotype_hash: phenotype.phenotype_hash(),
            capacity_class_id: phenotype.brain_class_id(),
            sensor_profile: sidecars.sensor_profile,
            immutable_phenotype,
            phenotype_compiler_inputs,
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
            last_learning_replay_key: parts.last_learning_replay_key,
            pending_eligibility: pending_checkpoint,
            pending_experience_transaction,
            memory,
            topology,
            tracked_objects: sidecars.tracked_objects,
            sleep,
            sleep_assets,
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
        backend: &mut GpuClosedLoopBackend,
        manifest: &AssetManifest,
        state: &GpuBrainSaveState,
    ) -> Result<RestoredGpuBrainCheckpoint, GameAppShellError> {
        state.validate()?;
        manifest.validate_with_root(self.root())?;
        let (compiler_inputs, _): (PhenotypeCompilerInputs, Vec<u8>) =
            self.read_json(manifest, &state.phenotype_compiler_inputs)?;
        let capacity = BrainCapacityClass::production_for_id(state.capacity_class_id)?;
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
                sealed_patch.validate_contract()?;
                if sealed_patch.pre_action().organism_id != state.organism_id
                    || sealed_patch.header().sensor_profile.identity() != state.sensor_profile
                    || state.pending_eligibility != Some(recovery.pending)
                    || recovery.pending.originating_tick != sealed_patch.pre_action().tick
                    || recovery.pending.frame_digest
                        != sealed_patch.decision().neural_evidence()?.frame_digest
                {
                    return Err(ScaffoldContractError::LearningEvidenceMismatch.into());
                }
                Some(RestoredRetainedLearning {
                    sealed_patch,
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
        let (replay, _): (PortableReplayJournalV1, Vec<u8>) =
            self.read_json(manifest, &state.replay_journal)?;
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
            sleep: state.sleep,
            pending_transaction,
            memory,
            topology,
            tracked_objects: state.tracked_objects.clone(),
            retained_learning,
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
