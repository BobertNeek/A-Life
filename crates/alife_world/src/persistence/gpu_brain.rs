//! Portable, digest-checked GPU-brain checkpoint records.
//!
//! This module owns serialization contracts only. It contains no neural math,
//! GPU handles, packed arena offsets, or fallback execution path.

use alife_core::{
    ActionId, BrainActivityPolicyV1, BrainCapacityClass, BrainClassId, CandidateActionFamily,
    CandidateFeatureDigest, CanonicalDigestBuilder, ConsolidationState, MemoryCompactionCheckpoint,
    MemoryCompactionPhase, MemorySidecarState, OrganismId, OutcomeCreditReplayKey,
    PerceptionFrameDigest, PhenotypeHash, PortableTopologySidecarAssetV1, ReplayEligibilitySample,
    ReplaySynapseSpan, ScaffoldContractError, SensorProfileIdentity, SleepReplayEvent, SleepState,
    Tick, TopologyCounts, TopologySidecar, Validate, MAX_REPLAY_CAPTURE_SYNAPSES,
};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};

use crate::TrackedObjectRegistrySaveState;

use super::{
    gpu_brain_vnext::{GpuBackendProvenanceSave, ThrottleReplaySaveState},
    AssetManifest, PersistenceError, PortableAssetDigest,
};

pub const GPU_BRAIN_SAVE_STATE_SCHEMA_VERSION: u16 = 3;
pub const GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION: u16 = 1;
pub const MEMORY_SIDECAR_SAVE_SCHEMA_VERSION: u16 = 1;
pub const TOPOLOGY_SIDECAR_SAVE_SCHEMA_VERSION: u16 = 1;
pub const RETAINED_LEARNING_RECOVERY_SAVE_SCHEMA_VERSION: u16 = 1;
pub const GPU_BRAIN_WEIGHT_LAYER_LIFETIME: u16 = 1;
pub const GPU_BRAIN_WEIGHT_LAYER_FAST: u16 = 2;
pub const GPU_BRAIN_HOMEOSTASIS_LANES_PER_NEURON: u16 = 2;

const ACTIVATION_DIGEST_DOMAIN: &[u8] = b"ALIFE-GPU-ACTIVATION-BANKS-V1";
const HOMEOSTASIS_DIGEST_DOMAIN: &[u8] = b"ALIFE-GPU-NEURON-HOMEOSTASIS-V1";
const WEIGHT_DIGEST_DOMAIN: &[u8] = b"ALIFE-GPU-DUAL-WEIGHT-BANK-V1";
const ELIGIBILITY_DIGEST_DOMAIN: &[u8] = b"ALIFE-GPU-ELIGIBILITY-BANKS-V1";
const REPLAY_JOURNAL_DIGEST_DOMAIN: &[u8] = b"ALIFE-GPU-REPLAY-JOURNAL-V1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuBrainAssetRef {
    pub asset_id: String,
    pub digest: PortableAssetDigest,
}

impl GpuBrainAssetRef {
    pub fn validate(&self) -> Result<(), PersistenceError> {
        if self.asset_id.trim().is_empty()
            || self.asset_id.len() > 256
            || self.asset_id.chars().any(char::is_control)
        {
            return Err(PersistenceError::InvalidAssetManifest {
                asset_id: self.asset_id.clone(),
                message: "GPU brain asset id must be non-empty, bounded UTF-8",
            });
        }
        self.digest.validate_format()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemorySidecarSaveSummary {
    pub schema_version: u16,
    pub organism_id_raw: u64,
    pub profile: SensorProfileIdentity,
    pub capacity: u32,
    pub record_count: u32,
    pub merge_count: u64,
    pub eviction_count: u64,
    pub compaction_count: u64,
    pub active_generation: u64,
    pub active_digest: [u64; 4],
}

impl MemorySidecarSaveSummary {
    fn validate_for(
        &self,
        organism_id: OrganismId,
        profile: SensorProfileIdentity,
    ) -> Result<(), PersistenceError> {
        self.profile.validate_contract()?;
        if self.schema_version != MEMORY_SIDECAR_SAVE_SCHEMA_VERSION
            || self.organism_id_raw != organism_id.raw()
            || self.profile != profile
            || self.capacity == 0
            || self.record_count > self.capacity
            || self.active_digest == [0; 4]
        {
            return Err(PersistenceError::Contract(
                ScaffoldContractError::InvalidMemoryQuery,
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryCompactionSaveState {
    pub checkpoint: MemoryCompactionCheckpoint,
    pub active_bank_asset: GpuBrainAssetRef,
    pub staged_bank_asset: Option<GpuBrainAssetRef>,
}

impl MemoryCompactionSaveState {
    fn validate_for(&self, summary: &MemorySidecarSaveSummary) -> Result<(), PersistenceError> {
        self.checkpoint.validate_contract()?;
        self.active_bank_asset.validate()?;
        if let Some(asset) = &self.staged_bank_asset {
            asset.validate()?;
        }
        if self.checkpoint.organism_id_raw != summary.organism_id_raw {
            return Err(PersistenceError::Contract(
                ScaffoldContractError::InvalidMemoryQuery,
            ));
        }

        let active_matches_checkpoint = self.checkpoint.active_generation
            == summary.active_generation
            && self.checkpoint.active_digest == summary.active_digest;
        let valid = match self.checkpoint.phase {
            MemoryCompactionPhase::Idle | MemoryCompactionPhase::Pending { .. } => {
                active_matches_checkpoint && self.staged_bank_asset.is_none()
            }
            MemoryCompactionPhase::Staged {
                input_generation,
                output_generation,
                input_digest,
                output_digest,
                ..
            } => {
                let active_is_input = summary.active_generation == input_generation
                    && summary.active_digest == input_digest;
                let active_is_output = summary.active_generation == output_generation
                    && summary.active_digest == output_digest;
                self.staged_bank_asset.is_some() && (active_is_input || active_is_output)
            }
            MemoryCompactionPhase::Committed { .. } => {
                active_matches_checkpoint && self.staged_bank_asset.is_none()
            }
        };
        if !valid {
            return Err(PersistenceError::Contract(
                ScaffoldContractError::MemoryCompactionConflict,
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetainedLearningRecoverySaveState {
    pub schema_version: u16,
    pub organism_id_raw: u64,
    pub pending: PendingEligibilityCheckpoint,
    pub sealed_patch_asset: GpuBrainAssetRef,
    pub attempts: u8,
    pub last_error_code: String,
}

impl RetainedLearningRecoverySaveState {
    fn validate_for(&self, organism_id: OrganismId) -> Result<(), PersistenceError> {
        self.pending.validate_contract()?;
        self.sealed_patch_asset.validate()?;
        if self.schema_version != RETAINED_LEARNING_RECOVERY_SAVE_SCHEMA_VERSION
            || self.organism_id_raw != organism_id.raw()
            || !(1..=3).contains(&self.attempts)
            || !matches!(
                self.last_error_code.as_str(),
                "learning-evidence-mismatch"
                    | "neural-backend-unavailable"
                    | "other-contract-failure"
            )
        {
            return Err(PersistenceError::Contract(
                ScaffoldContractError::LearningEvidenceMismatch,
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemorySidecarSaveState {
    pub summary: MemorySidecarSaveSummary,
    pub compaction: MemoryCompactionSaveState,
    pub retained_learning: Option<RetainedLearningRecoverySaveState>,
}

impl MemorySidecarSaveState {
    pub fn from_sidecar(
        sidecar: &MemorySidecarState,
        active_bank_asset: GpuBrainAssetRef,
        staged_bank_asset: Option<GpuBrainAssetRef>,
        retained_learning: Option<RetainedLearningRecoverySaveState>,
    ) -> Result<Self, PersistenceError> {
        let active = sidecar.export_active_bank()?;
        let has_staged_bank = sidecar.export_staged_bank()?.is_some();
        if has_staged_bank != staged_bank_asset.is_some() {
            return Err(PersistenceError::Contract(
                ScaffoldContractError::MemoryCompactionConflict,
            ));
        }
        let checkpoint = *sidecar.compaction_checkpoint();
        let summary = MemorySidecarSaveSummary {
            schema_version: MEMORY_SIDECAR_SAVE_SCHEMA_VERSION,
            organism_id_raw: sidecar.organism_id().raw(),
            profile: sidecar.profile(),
            capacity: active.capacity,
            record_count: u32::try_from(active.records.len()).map_err(|_| {
                PersistenceError::Contract(ScaffoldContractError::InvalidMemoryQuery)
            })?,
            merge_count: active.merge_count,
            eviction_count: active.eviction_count,
            compaction_count: checkpoint.last_committed_cycle_id.unwrap_or(0),
            active_generation: active.generation,
            active_digest: active.active_bank_digest,
        };
        let state = Self {
            summary,
            compaction: MemoryCompactionSaveState {
                checkpoint,
                active_bank_asset,
                staged_bank_asset,
            },
            retained_learning,
        };
        state.validate_for(sidecar.organism_id(), sidecar.profile())?;
        Ok(state)
    }

    pub fn validate_for(
        &self,
        organism_id: OrganismId,
        profile: SensorProfileIdentity,
    ) -> Result<(), PersistenceError> {
        self.summary.validate_for(organism_id, profile)?;
        self.compaction.validate_for(&self.summary)?;
        if let Some(recovery) = &self.retained_learning {
            recovery.validate_for(organism_id)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologySidecarSaveSummary {
    pub schema_version: u16,
    pub organism_id_raw: u64,
    pub profile: SensorProfileIdentity,
    pub counts: TopologyCounts,
    pub next_concept_id_raw: u64,
    pub next_edge_id_raw: u64,
    pub next_simplex_id_raw: u64,
    pub next_gap_id_raw: u64,
    pub max_bindings_per_kind: u32,
    pub has_last_observation: bool,
    pub last_observed_sequence_id_raw: u64,
    pub last_observed_key_digest: [u64; 4],
    pub degradation_count: u64,
    pub replay_rejection_count: u64,
    pub canonical_digest: [u64; 4],
    pub summary_asset: GpuBrainAssetRef,
}

impl TopologySidecarSaveSummary {
    pub fn from_sidecar(
        sidecar: &TopologySidecar,
        summary_asset: GpuBrainAssetRef,
    ) -> Result<Self, PersistenceError> {
        let asset = sidecar.export_portable()?;
        Self::from_asset(&asset, summary_asset)
    }

    pub fn from_asset(
        asset: &PortableTopologySidecarAssetV1,
        summary_asset: GpuBrainAssetRef,
    ) -> Result<Self, PersistenceError> {
        asset.validate_contract()?;
        let state = Self {
            schema_version: TOPOLOGY_SIDECAR_SAVE_SCHEMA_VERSION,
            organism_id_raw: asset.organism_id_raw,
            profile: asset.profile,
            counts: TopologyCounts {
                concepts: u32::try_from(asset.concepts.len()).map_err(|_| {
                    PersistenceError::Contract(ScaffoldContractError::InvalidMemoryQuery)
                })?,
                edges: u32::try_from(asset.edges.len()).map_err(|_| {
                    PersistenceError::Contract(ScaffoldContractError::InvalidMemoryQuery)
                })?,
                simplexes: u32::try_from(asset.simplexes.len()).map_err(|_| {
                    PersistenceError::Contract(ScaffoldContractError::InvalidMemoryQuery)
                })?,
                unresolved_gaps: u32::try_from(asset.gaps.len()).map_err(|_| {
                    PersistenceError::Contract(ScaffoldContractError::InvalidMemoryQuery)
                })?,
            },
            next_concept_id_raw: asset.next_concept_id_raw,
            next_edge_id_raw: asset.next_edge_id_raw,
            next_simplex_id_raw: asset.next_simplex_id_raw,
            next_gap_id_raw: asset.next_gap_id_raw,
            max_bindings_per_kind: asset.max_bindings_per_kind,
            has_last_observation: asset.last_observed_sequence_id_raw != 0,
            last_observed_sequence_id_raw: asset.last_observed_sequence_id_raw,
            last_observed_key_digest: asset.last_observed_key_digest,
            degradation_count: asset.degradation_count,
            replay_rejection_count: asset.replay_rejection_count,
            canonical_digest: asset.canonical_digest,
            summary_asset,
        };
        state.validate_for(OrganismId(asset.organism_id_raw), asset.profile)?;
        Ok(state)
    }

    pub fn validate_for(
        &self,
        organism_id: OrganismId,
        profile: SensorProfileIdentity,
    ) -> Result<(), PersistenceError> {
        self.profile.validate_contract()?;
        self.summary_asset.validate()?;
        let last_observation_valid = self.has_last_observation
            == (self.last_observed_sequence_id_raw != 0)
            && self.has_last_observation == (self.last_observed_key_digest != [0; 4]);
        if self.schema_version != TOPOLOGY_SIDECAR_SAVE_SCHEMA_VERSION
            || self.organism_id_raw != organism_id.raw()
            || self.profile != profile
            || self.next_concept_id_raw == 0
            || self.next_edge_id_raw == 0
            || self.next_simplex_id_raw == 0
            || self.next_gap_id_raw == 0
            || self.max_bindings_per_kind == 0
            || !last_observation_valid
            || self.canonical_digest == [0; 4]
        {
            return Err(PersistenceError::Contract(
                ScaffoldContractError::InvalidMemoryQuery,
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableActivationBanksV1 {
    pub schema_version: u16,
    pub phenotype_hash: PhenotypeHash,
    pub neuron_count: u32,
    pub active_side: u8,
    pub logical_dispatch_generation: u64,
    pub activation_a_bits: Vec<u32>,
    pub activation_b_bits: Vec<u32>,
    pub canonical_digest: [u64; 4],
}

impl PortableActivationBanksV1 {
    pub fn recompute_canonical_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        let mut digest = CanonicalDigestBuilder::new(ACTIVATION_DIGEST_DOMAIN);
        digest.write_u16(self.schema_version);
        write_digest4(&mut digest, self.phenotype_hash.0);
        digest.write_u32(self.neuron_count);
        digest.write_u8(self.active_side);
        digest.write_u64(self.logical_dispatch_generation);
        write_float_bits(&mut digest, &self.activation_a_bits)?;
        write_float_bits(&mut digest, &self.activation_b_bits)?;
        Ok(digest.finish256())
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        let neuron_count = checked_count(self.neuron_count)?;
        if self.schema_version != GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION
            || self.phenotype_hash == PhenotypeHash([0; 4])
            || self.active_side > 1
            || self.logical_dispatch_generation == 0
            || self.activation_a_bits.len() != neuron_count
            || self.activation_b_bits.len() != neuron_count
            || self.canonical_digest != self.recompute_canonical_digest()?
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableNeuronHomeostasisV1 {
    pub schema_version: u16,
    pub phenotype_hash: PhenotypeHash,
    pub neuron_count: u32,
    pub lanes_per_neuron: u16,
    pub value_bits: Vec<u32>,
    pub canonical_digest: [u64; 4],
}

impl PortableNeuronHomeostasisV1 {
    pub fn recompute_canonical_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        let mut digest = CanonicalDigestBuilder::new(HOMEOSTASIS_DIGEST_DOMAIN);
        digest.write_u16(self.schema_version);
        write_digest4(&mut digest, self.phenotype_hash.0);
        digest.write_u32(self.neuron_count);
        digest.write_u16(self.lanes_per_neuron);
        write_float_bits(&mut digest, &self.value_bits)?;
        Ok(digest.finish256())
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        let expected = checked_count(self.neuron_count)?
            .checked_mul(usize::from(GPU_BRAIN_HOMEOSTASIS_LANES_PER_NEURON))
            .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
        if self.schema_version != GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION
            || self.phenotype_hash == PhenotypeHash([0; 4])
            || self.lanes_per_neuron != GPU_BRAIN_HOMEOSTASIS_LANES_PER_NEURON
            || self.value_bits.len() != expected
            || self.canonical_digest != self.recompute_canonical_digest()?
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableDualWeightBankV1 {
    pub schema_version: u16,
    pub layer_raw: u16,
    pub phenotype_hash: PhenotypeHash,
    pub synapse_count: u32,
    pub active_generation: u64,
    pub active_bank: u8,
    pub bank_0_bits: Vec<u32>,
    pub bank_1_bits: Vec<u32>,
    pub canonical_digest: [u64; 4],
}

impl PortableDualWeightBankV1 {
    pub fn recompute_canonical_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        let mut digest = CanonicalDigestBuilder::new(WEIGHT_DIGEST_DOMAIN);
        digest.write_u16(self.schema_version);
        digest.write_u16(self.layer_raw);
        write_digest4(&mut digest, self.phenotype_hash.0);
        digest.write_u32(self.synapse_count);
        digest.write_u64(self.active_generation);
        digest.write_u8(self.active_bank);
        write_float_bits(&mut digest, &self.bank_0_bits)?;
        write_float_bits(&mut digest, &self.bank_1_bits)?;
        Ok(digest.finish256())
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        let synapse_count = checked_count(self.synapse_count)?;
        if self.schema_version != GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION
            || !matches!(
                self.layer_raw,
                GPU_BRAIN_WEIGHT_LAYER_LIFETIME | GPU_BRAIN_WEIGHT_LAYER_FAST
            )
            || self.phenotype_hash == PhenotypeHash([0; 4])
            || self.active_generation == 0
            || self.active_bank > 1
            || self.bank_0_bits.len() != synapse_count
            || self.bank_1_bits.len() != synapse_count
            || self.canonical_digest != self.recompute_canonical_digest()?
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableEligibilityBanksV1 {
    pub schema_version: u16,
    pub phenotype_hash: PhenotypeHash,
    pub recurrent_count: u32,
    pub decoder_count: u32,
    pub active_generation: u64,
    pub inactive_generation: u64,
    pub active_bank: u8,
    pub recurrent_bank_0_bits: Vec<u32>,
    pub recurrent_bank_1_bits: Vec<u32>,
    pub decoder_bank_0_bits: Vec<u32>,
    pub decoder_bank_1_bits: Vec<u32>,
    pub canonical_digest: [u64; 4],
}

impl PortableEligibilityBanksV1 {
    pub fn recompute_canonical_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        let mut digest = CanonicalDigestBuilder::new(ELIGIBILITY_DIGEST_DOMAIN);
        digest.write_u16(self.schema_version);
        write_digest4(&mut digest, self.phenotype_hash.0);
        digest.write_u32(self.recurrent_count);
        digest.write_u32(self.decoder_count);
        digest.write_u64(self.active_generation);
        digest.write_u64(self.inactive_generation);
        digest.write_u8(self.active_bank);
        write_float_bits(&mut digest, &self.recurrent_bank_0_bits)?;
        write_float_bits(&mut digest, &self.recurrent_bank_1_bits)?;
        write_float_bits(&mut digest, &self.decoder_bank_0_bits)?;
        write_float_bits(&mut digest, &self.decoder_bank_1_bits)?;
        Ok(digest.finish256())
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        let recurrent_count = checked_count(self.recurrent_count)?;
        let decoder_count = checked_count(self.decoder_count)?;
        let expected_inactive = self.active_generation.checked_add(1);
        if self.schema_version != GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION
            || self.phenotype_hash == PhenotypeHash([0; 4])
            || self.active_generation == 0
            || (self.inactive_generation != 0
                && Some(self.inactive_generation) != expected_inactive)
            || self.active_bank > 1
            || self.recurrent_bank_0_bits.len() != recurrent_count
            || self.recurrent_bank_1_bits.len() != recurrent_count
            || self.decoder_bank_0_bits.len() != decoder_count
            || self.decoder_bank_1_bits.len() != decoder_count
            || self.canonical_digest != self.recompute_canonical_digest()?
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        Ok(())
    }
}

/// Compact chronological replay rows plus enough ring metadata to reconstruct
/// the exact physical journal on restore.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortableReplayJournalV1 {
    pub schema_version: u16,
    pub phenotype_hash: PhenotypeHash,
    pub replay_capture_plan_digest: [u64; 4],
    pub generation: u64,
    pub cursor: u32,
    pub event_count: u32,
    pub event_capacity: u32,
    pub sample_capacity: u32,
    pub events: Vec<SleepReplayEvent>,
    pub synapse_spans: Vec<ReplaySynapseSpan>,
    pub eligibility_samples: Vec<ReplayEligibilitySample>,
    pub canonical_digest: [u64; 4],
}

impl PortableReplayJournalV1 {
    pub fn recompute_canonical_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        let mut digest = CanonicalDigestBuilder::new(REPLAY_JOURNAL_DIGEST_DOMAIN);
        digest.write_u16(self.schema_version);
        write_digest4(&mut digest, self.phenotype_hash.0);
        write_digest4(&mut digest, self.replay_capture_plan_digest);
        digest.write_u64(self.generation);
        digest.write_u32(self.cursor);
        digest.write_u32(self.event_count);
        digest.write_u32(self.event_capacity);
        digest.write_u32(self.sample_capacity);
        digest.write_sequence_len(self.events.len());
        for event in &self.events {
            write_replay_event(&mut digest, event)?;
        }
        digest.write_sequence_len(self.synapse_spans.len());
        for span in &self.synapse_spans {
            digest.write_u32(span.local_synapse_id);
            digest.write_u32(span.sample_start);
            digest.write_u32(span.sample_count);
            digest.write_u32(span.reserved);
        }
        digest.write_sequence_len(self.eligibility_samples.len());
        for sample in &self.eligibility_samples {
            digest.write_u16(sample.event_index);
            digest.write_i16(sample.eligibility_q15);
        }
        Ok(digest.finish256())
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        let span_count = u32::try_from(self.synapse_spans.len())
            .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?;
        let expected_sample_capacity = self
            .event_capacity
            .checked_mul(span_count)
            .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
        let expected_live_samples = self
            .event_count
            .checked_mul(span_count)
            .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
        if self.schema_version != GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION
            || self.phenotype_hash == PhenotypeHash([0; 4])
            || self.replay_capture_plan_digest == [0; 4]
            || self.generation == 0
            || self.event_capacity == 0
            || self.event_capacity > 65_536
            || self.event_count > self.event_capacity
            || self.cursor >= self.event_capacity
            || (self.event_count < self.event_capacity && self.cursor != self.event_count)
            || self.events.len() != checked_count_allow_zero(self.event_count)?
            || span_count == 0
            || span_count > MAX_REPLAY_CAPTURE_SYNAPSES
            || self.sample_capacity != expected_sample_capacity
            || self.eligibility_samples.len() != checked_count_allow_zero(expected_live_samples)?
            || self
                .events
                .windows(2)
                .any(|pair| pair[0].sequence_id.raw() >= pair[1].sequence_id.raw())
            || self
                .events
                .iter()
                .any(|event| validate_replay_event(event).is_err())
            || self
                .synapse_spans
                .windows(2)
                .any(|pair| pair[0].local_synapse_id >= pair[1].local_synapse_id)
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }

        let mut expected_start = 0_u32;
        for span in &self.synapse_spans {
            if span.reserved != 0
                || span.sample_start != expected_start
                || span.sample_count != self.event_count
            {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
            }
            let end = span
                .sample_start
                .checked_add(span.sample_count)
                .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
            let start_usize = checked_count_allow_zero(span.sample_start)?;
            let end_usize = checked_count_allow_zero(end)?;
            let samples = self
                .eligibility_samples
                .get(start_usize..end_usize)
                .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
            if samples.iter().enumerate().any(|(index, sample)| {
                sample.event_index as usize != index || sample.eligibility_q15 == i16::MIN
            }) {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
            }
            expected_start = end;
        }
        if expected_start != expected_live_samples
            || self.canonical_digest != self.recompute_canonical_digest()?
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingEligibilityCheckpoint {
    pub dispatch_generation: u64,
    pub originating_tick: Tick,
    pub frame_digest: PerceptionFrameDigest,
    pub active_activation_side: u8,
    pub candidate_index: u16,
    pub action_id: ActionId,
    pub action_family: CandidateActionFamily,
    pub candidate_feature_digest: CandidateFeatureDigest,
    pub active_eligibility_generation: u64,
    pub staging_eligibility_generation: u64,
}

impl PendingEligibilityCheckpoint {
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        dispatch_generation: u64,
        originating_tick: Tick,
        frame_digest: PerceptionFrameDigest,
        active_activation_side: u8,
        candidate_index: u16,
        action_id: ActionId,
        action_family: CandidateActionFamily,
        candidate_feature_digest: CandidateFeatureDigest,
        active_eligibility_generation: u64,
        staging_eligibility_generation: u64,
    ) -> Result<Self, ScaffoldContractError> {
        let value = Self {
            dispatch_generation,
            originating_tick,
            frame_digest,
            active_activation_side,
            candidate_index,
            action_id,
            action_family,
            candidate_feature_digest,
            active_eligibility_generation,
            staging_eligibility_generation,
        };
        value.validate_contract()?;
        Ok(value)
    }

    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.action_id.validate()?;
        let expected_staging = self.active_eligibility_generation.checked_add(1);
        if self.dispatch_generation == 0
            || self.frame_digest == PerceptionFrameDigest([0; 4])
            || self.active_activation_side > 1
            || self.candidate_feature_digest == CandidateFeatureDigest([0; 2])
            || self.active_eligibility_generation == 0
            || Some(self.staging_eligibility_generation) != expected_staging
        {
            return Err(ScaffoldContractError::LearningEvidenceMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuSleepAssetState {
    pub replay_batch: Option<GpuBrainAssetRef>,
    pub lifetime_staging: Option<GpuBrainAssetRef>,
    pub fast_staging: Option<GpuBrainAssetRef>,
    pub eligibility_staging: Option<GpuBrainAssetRef>,
    pub replay_journal_staging: Option<GpuBrainAssetRef>,
}

impl GpuSleepAssetState {
    fn validate_refs(&self) -> Result<(), PersistenceError> {
        for asset in [
            self.replay_batch.as_ref(),
            self.lifetime_staging.as_ref(),
            self.fast_staging.as_ref(),
            self.eligibility_staging.as_ref(),
            self.replay_journal_staging.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            asset.validate()?;
        }
        Ok(())
    }

    fn has_any_staging(&self) -> bool {
        self.lifetime_staging.is_some()
            || self.fast_staging.is_some()
            || self.eligibility_staging.is_some()
            || self.replay_journal_staging.is_some()
    }

    fn has_all_staging(&self) -> bool {
        self.lifetime_staging.is_some()
            && self.fast_staging.is_some()
            && self.eligibility_staging.is_some()
            && self.replay_journal_staging.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GpuBrainSaveState {
    pub schema_version: u16,
    pub organism_id: OrganismId,
    pub phenotype_hash: PhenotypeHash,
    pub capacity_class_id: BrainClassId,
    pub sensor_profile: SensorProfileIdentity,
    pub immutable_phenotype: GpuBrainAssetRef,
    pub phenotype_compiler_inputs: GpuBrainAssetRef,
    pub active_weight_generation: u64,
    pub active_weight_bank: u8,
    pub active_eligibility_bank: u8,
    pub learning_transaction_generation: u64,
    pub lifetime_weights: GpuBrainAssetRef,
    pub fast_weights: GpuBrainAssetRef,
    pub eligibility: GpuBrainAssetRef,
    pub replay_journal: GpuBrainAssetRef,
    pub replay_journal_generation: u64,
    pub replay_journal_cursor: u32,
    pub replay_journal_event_count: u32,
    pub activation_state: GpuBrainAssetRef,
    pub neuron_homeostasis: GpuBrainAssetRef,
    pub checkpoint_tick: Tick,
    pub last_learning_replay_key: Option<OutcomeCreditReplayKey>,
    pub pending_eligibility: Option<PendingEligibilityCheckpoint>,
    pub pending_experience_transaction: Option<GpuBrainAssetRef>,
    pub memory: MemorySidecarSaveState,
    pub topology: TopologySidecarSaveSummary,
    pub tracked_objects: TrackedObjectRegistrySaveState,
    pub sleep: SleepState,
    pub sleep_assets: GpuSleepAssetState,
    pub backend_provenance: GpuBackendProvenanceSave,
    pub runtime_profile_id: u16,
    pub runtime_profile_digest: [u64; 4],
    pub activity_policy_version: u16,
    pub activity_policy_digest: [u64; 4],
    pub throttle_replay: ThrottleReplaySaveState,
}

impl GpuBrainSaveState {
    pub fn validate(&self) -> Result<(), PersistenceError> {
        if self.schema_version != GPU_BRAIN_SAVE_STATE_SCHEMA_VERSION {
            return Err(PersistenceError::SchemaVersion {
                schema: "alife.gpu_brain_save_state.v3",
                expected: GPU_BRAIN_SAVE_STATE_SCHEMA_VERSION,
                actual: self.schema_version,
            });
        }
        self.organism_id.validate()?;
        BrainCapacityClass::production_for_id(self.capacity_class_id)?;
        self.sensor_profile.validate_contract()?;
        self.memory
            .validate_for(self.organism_id, self.sensor_profile)?;
        self.topology
            .validate_for(self.organism_id, self.sensor_profile)?;
        self.tracked_objects.validate_contract()?;
        if self.tracked_objects.organism_id != self.organism_id {
            return Err(PersistenceError::Contract(
                ScaffoldContractError::BrainOwnershipMismatch,
            ));
        }
        if self.phenotype_hash == PhenotypeHash([0; 4])
            || self.active_weight_generation == 0
            || self.active_weight_bank > 1
            || self.active_eligibility_bank > 1
            || self.learning_transaction_generation == 0
            || self.replay_journal_generation == 0
        {
            return Err(PersistenceError::Contract(
                ScaffoldContractError::ConsolidationGenerationMismatch,
            ));
        }

        for asset in [
            &self.immutable_phenotype,
            &self.phenotype_compiler_inputs,
            &self.lifetime_weights,
            &self.fast_weights,
            &self.eligibility,
            &self.replay_journal,
            &self.activation_state,
            &self.neuron_homeostasis,
        ] {
            asset.validate()?;
        }
        if let Some(asset) = &self.pending_experience_transaction {
            asset.validate()?;
        }
        self.sleep_assets.validate_refs()?;
        self.sleep.validate_contract()?;
        self.backend_provenance.validate()?;
        self.throttle_replay.validate()?;

        let activity_policy = BrainActivityPolicyV1::production_v1();
        let throttle_binding_matches =
            self.throttle_replay
                .last_checkpoint
                .as_ref()
                .is_none_or(|checkpoint| {
                    checkpoint.organism_id_raw == self.organism_id.raw()
                        && checkpoint.class_id_raw == self.capacity_class_id.raw()
                        && checkpoint.tick <= self.checkpoint_tick.raw()
                });
        if self.runtime_profile_id == 0
            || self.runtime_profile_digest == [0; 4]
            || self.activity_policy_version != activity_policy.policy_version
            || self.activity_policy_digest != activity_policy.policy_digest
            || self.throttle_replay.policy_version != self.activity_policy_version
            || self.throttle_replay.policy_digest != self.activity_policy_digest
            || !throttle_binding_matches
        {
            return Err(PersistenceError::Contract(
                ScaffoldContractError::BrainActivitySequenceMismatch,
            ));
        }

        if let Some(last) = self.last_learning_replay_key {
            last.organism_id.validate()?;
            last.sequence_id.validate()?;
            if last.organism_id != self.organism_id || last.phenotype_hash != self.phenotype_hash {
                return Err(PersistenceError::Contract(
                    ScaffoldContractError::LearningEvidenceMismatch,
                ));
            }
        }
        if let Some(pending) = self.pending_eligibility {
            pending.validate_contract()?;
        }
        match (
            self.pending_eligibility,
            self.pending_experience_transaction.as_ref(),
            self.memory.retained_learning.as_ref(),
        ) {
            (None, None, None) | (Some(_), Some(_), None) => {}
            (Some(pending), None, Some(recovery)) if recovery.pending == pending => {}
            _ => {
                return Err(PersistenceError::Contract(
                    ScaffoldContractError::LearningEvidenceMismatch,
                ));
            }
        }

        self.validate_sleep_transaction()
    }

    /// Builds the post-manifest-CAS state for an already committed GPU sleep
    /// transaction. The source remains unchanged so its exact serialized form
    /// can be used as the compare side of the durable swap.
    pub fn promoted_completed_sleep_state(&self) -> Result<Self, PersistenceError> {
        self.validate()?;
        let (request, staged) = match self.sleep.consolidation {
            ConsolidationState::Completed { request, staged } => (request, staged),
            _ => {
                return Err(PersistenceError::Contract(
                    ScaffoldContractError::ConsolidationGenerationMismatch,
                ));
            }
        };
        let missing =
            || PersistenceError::Contract(ScaffoldContractError::ConsolidationGenerationMismatch);
        let lifetime_weights = self
            .sleep_assets
            .lifetime_staging
            .clone()
            .ok_or_else(missing)?;
        let fast_weights = self.sleep_assets.fast_staging.clone().ok_or_else(missing)?;
        let eligibility = self
            .sleep_assets
            .eligibility_staging
            .clone()
            .ok_or_else(missing)?;
        let replay_journal = self
            .sleep_assets
            .replay_journal_staging
            .clone()
            .ok_or_else(missing)?;
        let learning_transaction_generation = self
            .learning_transaction_generation
            .checked_add(1)
            .ok_or_else(missing)?;

        let mut promoted = self.clone();
        promoted.active_weight_generation = staged.output_generation;
        promoted.active_weight_bank = staged.output_weight_bank;
        promoted.active_eligibility_bank = staged.output_eligibility_bank;
        promoted.learning_transaction_generation = learning_transaction_generation;
        promoted.lifetime_weights = lifetime_weights;
        promoted.fast_weights = fast_weights;
        promoted.eligibility = eligibility;
        promoted.replay_journal = replay_journal;
        promoted.replay_journal_generation = staged.replay_journal_generation;
        promoted.replay_journal_cursor = staged.replay_journal_cursor;
        promoted.replay_journal_event_count = staged.replay_journal_event_count;
        promoted.sleep.consolidation = ConsolidationState::Committed {
            cycle_id: request.cycle_id,
            output_generation: staged.output_generation,
            output_digest: staged.output_digest,
        };
        promoted.sleep_assets = GpuSleepAssetState::default();
        promoted.validate()?;
        Ok(promoted)
    }

    /// Verifies that every bulk checkpoint reference is present exactly once
    /// in the enclosing portable-save manifest and binds the same digest.
    pub fn validate_asset_manifest(
        &self,
        manifest: &AssetManifest,
    ) -> Result<(), PersistenceError> {
        self.validate()?;
        let mut refs = vec![
            &self.immutable_phenotype,
            &self.phenotype_compiler_inputs,
            &self.lifetime_weights,
            &self.fast_weights,
            &self.eligibility,
            &self.replay_journal,
            &self.activation_state,
            &self.neuron_homeostasis,
        ];
        refs.extend(self.pending_experience_transaction.iter());
        refs.push(&self.memory.compaction.active_bank_asset);
        refs.extend(self.memory.compaction.staged_bank_asset.iter());
        refs.push(&self.topology.summary_asset);
        refs.extend(
            self.memory
                .retained_learning
                .iter()
                .map(|recovery| &recovery.sealed_patch_asset),
        );
        refs.extend(self.sleep_assets.replay_batch.iter());
        refs.extend(self.sleep_assets.lifetime_staging.iter());
        refs.extend(self.sleep_assets.fast_staging.iter());
        refs.extend(self.sleep_assets.eligibility_staging.iter());
        refs.extend(self.sleep_assets.replay_journal_staging.iter());
        refs.push(&self.throttle_replay.sequence_asset);

        for asset in refs {
            let mut matches = manifest
                .entries
                .iter()
                .filter(|entry| entry.asset_id == asset.asset_id);
            let entry = matches
                .next()
                .ok_or_else(|| PersistenceError::MissingAssetReference {
                    asset_id: asset.asset_id.clone(),
                })?;
            if matches.next().is_some() {
                return Err(PersistenceError::InvalidAssetManifest {
                    asset_id: asset.asset_id.clone(),
                    message: "duplicate GPU checkpoint asset reference",
                });
            }
            if entry.digest != asset.digest {
                return Err(PersistenceError::DigestMismatch {
                    asset_id: asset.asset_id.clone(),
                    expected: asset.digest.0.clone(),
                    actual: entry.digest.0.clone(),
                });
            }
        }
        Ok(())
    }

    fn validate_sleep_transaction(&self) -> Result<(), PersistenceError> {
        let invalid =
            || PersistenceError::Contract(ScaffoldContractError::ConsolidationGenerationMismatch);
        match self.sleep.consolidation {
            ConsolidationState::None => {
                if self.sleep_assets.replay_batch.is_some() || self.sleep_assets.has_any_staging() {
                    return Err(invalid());
                }
            }
            ConsolidationState::Pending { .. } => {
                if self.sleep_assets.replay_batch.is_none() || self.sleep_assets.has_any_staging() {
                    return Err(invalid());
                }
            }
            ConsolidationState::Prepared { request }
            | ConsolidationState::Submitted { request, .. } => {
                if self.sleep_assets.replay_batch.is_none()
                    || self.sleep_assets.has_any_staging()
                    || request.phenotype_hash != self.phenotype_hash
                    || request.input_generation != self.active_weight_generation
                {
                    return Err(invalid());
                }
            }
            ConsolidationState::Completed { request, staged } => {
                if self.sleep_assets.replay_batch.is_none()
                    || !self.sleep_assets.has_all_staging()
                    || request.phenotype_hash != self.phenotype_hash
                    || request.input_generation != self.active_weight_generation
                    || staged.output_generation != request.expected_output_generation
                {
                    return Err(invalid());
                }
            }
            ConsolidationState::Committed {
                output_generation, ..
            } => {
                if self.sleep_assets.replay_batch.is_some()
                    || self.sleep_assets.has_any_staging()
                    || output_generation != self.active_weight_generation
                {
                    return Err(invalid());
                }
            }
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct GpuBrainSaveStateWire {
    schema_version: u16,
    organism_id: OrganismId,
    phenotype_hash: PhenotypeHash,
    capacity_class_id: BrainClassId,
    sensor_profile: SensorProfileIdentity,
    immutable_phenotype: GpuBrainAssetRef,
    phenotype_compiler_inputs: GpuBrainAssetRef,
    active_weight_generation: u64,
    active_weight_bank: u8,
    active_eligibility_bank: u8,
    learning_transaction_generation: u64,
    lifetime_weights: GpuBrainAssetRef,
    fast_weights: GpuBrainAssetRef,
    eligibility: GpuBrainAssetRef,
    replay_journal: GpuBrainAssetRef,
    replay_journal_generation: u64,
    replay_journal_cursor: u32,
    replay_journal_event_count: u32,
    activation_state: GpuBrainAssetRef,
    neuron_homeostasis: GpuBrainAssetRef,
    checkpoint_tick: Tick,
    last_learning_replay_key: Option<OutcomeCreditReplayKey>,
    pending_eligibility: Option<PendingEligibilityCheckpoint>,
    pending_experience_transaction: Option<GpuBrainAssetRef>,
    memory: MemorySidecarSaveState,
    topology: TopologySidecarSaveSummary,
    tracked_objects: TrackedObjectRegistrySaveState,
    sleep: SleepState,
    sleep_assets: GpuSleepAssetState,
    backend_provenance: GpuBackendProvenanceSave,
    runtime_profile_id: u16,
    runtime_profile_digest: [u64; 4],
    activity_policy_version: u16,
    activity_policy_digest: [u64; 4],
    throttle_replay: ThrottleReplaySaveState,
}

impl From<GpuBrainSaveStateWire> for GpuBrainSaveState {
    fn from(wire: GpuBrainSaveStateWire) -> Self {
        Self {
            schema_version: wire.schema_version,
            organism_id: wire.organism_id,
            phenotype_hash: wire.phenotype_hash,
            capacity_class_id: wire.capacity_class_id,
            sensor_profile: wire.sensor_profile,
            immutable_phenotype: wire.immutable_phenotype,
            phenotype_compiler_inputs: wire.phenotype_compiler_inputs,
            active_weight_generation: wire.active_weight_generation,
            active_weight_bank: wire.active_weight_bank,
            active_eligibility_bank: wire.active_eligibility_bank,
            learning_transaction_generation: wire.learning_transaction_generation,
            lifetime_weights: wire.lifetime_weights,
            fast_weights: wire.fast_weights,
            eligibility: wire.eligibility,
            replay_journal: wire.replay_journal,
            replay_journal_generation: wire.replay_journal_generation,
            replay_journal_cursor: wire.replay_journal_cursor,
            replay_journal_event_count: wire.replay_journal_event_count,
            activation_state: wire.activation_state,
            neuron_homeostasis: wire.neuron_homeostasis,
            checkpoint_tick: wire.checkpoint_tick,
            last_learning_replay_key: wire.last_learning_replay_key,
            pending_eligibility: wire.pending_eligibility,
            pending_experience_transaction: wire.pending_experience_transaction,
            memory: wire.memory,
            topology: wire.topology,
            tracked_objects: wire.tracked_objects,
            sleep: wire.sleep,
            sleep_assets: wire.sleep_assets,
            backend_provenance: wire.backend_provenance,
            runtime_profile_id: wire.runtime_profile_id,
            runtime_profile_digest: wire.runtime_profile_digest,
            activity_policy_version: wire.activity_policy_version,
            activity_policy_digest: wire.activity_policy_digest,
            throttle_replay: wire.throttle_replay,
        }
    }
}

impl<'de> Deserialize<'de> for GpuBrainSaveState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Self::from(GpuBrainSaveStateWire::deserialize(deserializer)?);
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

fn checked_count(value: u32) -> Result<usize, ScaffoldContractError> {
    if value == 0 {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    checked_count_allow_zero(value)
}

fn checked_count_allow_zero(value: u32) -> Result<usize, ScaffoldContractError> {
    usize::try_from(value).map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)
}

fn validate_float_bits(bits: &[u32]) -> Result<(), ScaffoldContractError> {
    if bits
        .iter()
        .any(|bits| *bits == 0x8000_0000 || !f32::from_bits(*bits).is_finite())
    {
        return Err(ScaffoldContractError::NonFiniteFloat);
    }
    Ok(())
}

fn write_float_bits(
    digest: &mut CanonicalDigestBuilder,
    bits: &[u32],
) -> Result<(), ScaffoldContractError> {
    validate_float_bits(bits)?;
    digest.write_sequence_len(bits.len());
    for bits in bits {
        digest.write_u32(*bits);
    }
    Ok(())
}

fn validate_replay_event(event: &SleepReplayEvent) -> Result<(), ScaffoldContractError> {
    event.sequence_id.validate()?;
    event.action_id.validate()?;
    let values = [
        event.modulator.reward_prediction_error(),
        event.modulator.pain(),
        event.modulator.homeostatic_improvement(),
        event.modulator.frustration(),
        event.modulator.novelty(),
        event.modulator.value(),
    ];
    if event.frame_digest == PerceptionFrameDigest([0; 4])
        || event.candidate_feature_digest == CandidateFeatureDigest([0; 2])
        || values
            .iter()
            .any(|value| !value.is_finite() || !(-1.0..=1.0).contains(value))
    {
        return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
    }
    Ok(())
}

fn write_replay_event(
    digest: &mut CanonicalDigestBuilder,
    event: &SleepReplayEvent,
) -> Result<(), ScaffoldContractError> {
    validate_replay_event(event)?;
    digest.write_u64(event.sequence_id.raw());
    digest.write_u64(event.originating_tick.raw());
    write_digest4(digest, event.frame_digest.0);
    for word in event.candidate_feature_digest.0 {
        digest.write_u64(word);
    }
    digest.write_u32(event.action_id.raw());
    digest.write_u8(event.family.raw());
    digest.write_f32(event.modulator.reward_prediction_error())?;
    digest.write_f32(event.modulator.pain())?;
    digest.write_f32(event.modulator.homeostatic_improvement())?;
    digest.write_f32(event.modulator.frustration())?;
    digest.write_f32(event.modulator.novelty())?;
    digest.write_f32(event.modulator.value())?;
    Ok(())
}

fn write_digest4(digest: &mut CanonicalDigestBuilder, words: [u64; 4]) {
    for word in words {
        digest.write_u64(word);
    }
}
