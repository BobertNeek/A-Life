//! Canonical real-hardware evidence for the GPU-authoritative closed loop.
//!
//! The evidence runner drives the production live runtime twice from the same
//! deterministic birth state. It records only compact GPU selection receipts
//! and sealed experience metadata; neural activations and weights never cross
//! back to the host.

use std::fs;
use std::path::Path;

use alife_core::{
    BrainCapacityClass, BrainClassId, BrainPhenotype, BrainScaleTier, PhenotypeHash, PolicyBackend,
    ScaffoldContractError, SensorProfile, Tick, Validate, Vec3f,
};
use alife_gpu_backend::{
    GpuClosedLoopBackend, GpuHardwareReceipt, GPU_HARDWARE_RECEIPT_SCHEMA_VERSION,
};
use alife_world::HeadlessScenarioBuilder;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    compile_gpu_birth_components, GameAppShellError, GpuLiveBrainEvidenceMetrics,
    GpuLiveBrainRuntime,
};

mod canonical;
use canonical::*;
mod learning_sleep;
pub use learning_sleep::*;
mod memory_grounding;
pub use memory_grounding::*;
mod persistence;
use persistence::*;

pub const GPU_SLICE_EVIDENCE_ARTIFACT_SCHEMA: u16 = 1;
pub const GPU_SLICE_A_RAW: u16 = 1;
pub const GPU_SLICE_B_RAW: u16 = 2;
pub const GPU_EVIDENCE_PASSING_STATUS_RAW: u16 = 1;
pub const GPU_PHENOTYPE_EVIDENCE_MANIFEST_SCHEMA: u16 = 1;
pub const GPU_SLICE_A_FIXTURE_SCHEMA: u16 = 1;
pub const GPU_SLICE_A_MAX_TICKS: u32 = 4_096;
pub const GPU_SLICE_A_REPLAY_TOLERANCE: f32 = 1.0e-6;

const GPU_EVIDENCE_MAX_ARTIFACT_BYTES: u64 = 4 * 1024 * 1024;
const GPU_EVIDENCE_ORGANISM_ID: alife_core::OrganismId = alife_core::OrganismId(1);

const MANIFEST_DOMAIN: &[u8] = b"alife.gpu.evidence.phenotype-manifest.v1";
const ARTIFACT_DOMAIN: &[u8] = b"alife.gpu.evidence.slice-a-artifact.v1";
const LOBE_LAYOUT_DOMAIN: &[u8] = b"alife.gpu.evidence.lobe-layout.v1";
const PROJECTION_PLAN_DOMAIN: &[u8] = b"alife.gpu.evidence.projection-plan.v1";
const SYNAPSE_PAYLOAD_DOMAIN: &[u8] = b"alife.gpu.evidence.synapse-payload.v1";
const PLASTICITY_NONE_DOMAIN: &[u8] = b"alife.gpu.evidence.plasticity-plan.none.v1";
const REPLAY_CAPTURE_NONE_DOMAIN: &[u8] = b"alife.gpu.evidence.replay-capture.none.v1";
const ADAPTER_IDENTITY_DOMAIN: &[u8] = b"alife.gpu.evidence.adapter-identity.v1";
const INITIAL_STATE_DOMAIN: &[u8] = b"alife.gpu.evidence.initial-state.v1";
const FRAME_SEQUENCE_DOMAIN: &[u8] = b"alife.gpu.evidence.frame-sequence.v1";
const CANDIDATE_SEQUENCE_DOMAIN: &[u8] = b"alife.gpu.evidence.candidate-sequence.v1";
const LOGIT_SEQUENCE_DOMAIN: &[u8] = b"alife.gpu.evidence.logit-sequence.v1";

#[derive(Debug, Error)]
pub enum GpuEvidenceError {
    #[error("GPU evidence contract failed: {0}")]
    Contract(&'static str),
    #[error("GPU evidence contract failed: {0}")]
    ContractDetail(String),
    #[error("GPU evidence Git provenance failed: {0}")]
    Git(String),
    #[error("GPU evidence does not yet define slice {0}")]
    UnsupportedSlice(u16),
    #[error("GPU evidence stage `{stage}` failed: {source}")]
    Stage {
        stage: &'static str,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error(transparent)]
    Core(#[from] ScaffoldContractError),
    #[error(transparent)]
    App(#[from] GameAppShellError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuSliceEvidenceHeader {
    pub artifact_schema: u16,
    pub slice_raw: u16,
    pub class_id_raw: u16,
    pub profile_id_raw: u16,
    pub profile_schema: u16,
    pub status_raw: u16,
    pub git_commit: String,
    pub source_tree_digest: String,
    pub artifact_digest: [u64; 4],
    pub phenotype_hash: PhenotypeHash,
    pub phenotype_manifest_digest: [u64; 4],
    pub capacity_digest: [u64; 4],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhenotypeEvidenceManifest {
    pub schema_version: u16,
    pub class_id_raw: u16,
    pub phenotype_sensor_profile_raw: u16,
    pub phenotype_hash: PhenotypeHash,
    pub compile_inputs_digest: [u64; 4],
    pub capacity_digest: [u64; 4],
    pub lobe_layout_digest: [u64; 4],
    pub projection_plan_digest: [u64; 4],
    pub synapse_payload_digest: [u64; 4],
    pub encoder_plan_digest: [u64; 4],
    pub decoder_plan_digest: [u64; 4],
    pub plasticity_plan_digest: [u64; 4],
    pub replay_capture_plan_digest: [u64; 4],
    pub manifest_digest: [u64; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GpuSameAdapterReplayEvidence {
    pub adapter_identity_digest: [u64; 4],
    pub initial_state_digest: [u64; 4],
    pub frame_sequence_digest: [u64; 4],
    pub selected_candidate_digest: [u64; 4],
    pub first_logit_digest: [u64; 4],
    pub second_logit_digest: [u64; 4],
    pub tolerance: f32,
    pub max_abs_error: f32,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuSelectionEvidence {
    pub tick: u64,
    pub frame_digest: [u64; 4],
    pub candidate_index: u16,
    pub action_id_raw: u32,
    pub action_family_raw: u8,
    pub candidate_feature_digest: [u64; 2],
    pub logit: f32,
    pub active_activation_side: u8,
    pub active_tiles: u32,
    pub active_synapses: u32,
    pub compact_readback_bytes: u32,
    pub outcome_success: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuSliceAAcceptanceReceipt {
    #[serde(flatten)]
    pub header: GpuSliceEvidenceHeader,
    pub capacity_class: String,
    pub capacity_class_id: BrainClassId,
    pub capacity: BrainCapacityClass,
    pub phenotype_manifest: PhenotypeEvidenceManifest,
    pub fixture_schema_version: u16,
    pub deterministic_seed: u64,
    pub source_tree_clean: bool,
    pub backend_api: String,
    pub adapter_name: String,
    pub hardware: GpuHardwareReceipt,
    pub authoritative: bool,
    pub policy_backend: PolicyBackend,
    pub requested_ticks: u32,
    pub neural_dispatch_count: u64,
    pub gpu_selection_count: u64,
    pub sealed_patch_count: u64,
    pub compact_readback_bytes: u32,
    pub active_tiles: u32,
    pub active_synapses: u32,
    pub selection_trace: Vec<GpuSelectionEvidence>,
    pub replay: GpuSameAdapterReplayEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuClosedLoopAcceptanceOptions {
    pub capacity: BrainCapacityClass,
    pub requested_ticks: u32,
    pub deterministic_seed: u64,
    pub sensor_profile: SensorProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GitProvenance {
    commit: String,
    tree: String,
    clean: bool,
}

#[derive(Debug)]
struct TrialEvidence {
    hardware: GpuHardwareReceipt,
    metrics: GpuLiveBrainEvidenceMetrics,
    trace: Vec<GpuSelectionEvidence>,
}

impl GpuClosedLoopAcceptanceOptions {
    fn validate(self) -> Result<Self, GpuEvidenceError> {
        self.capacity.validate_contract()?;
        if self.requested_ticks == 0 || self.requested_ticks > GPU_SLICE_A_MAX_TICKS {
            return Err(GpuEvidenceError::Contract(
                "requested tick count is outside the Slice A evidence bound",
            ));
        }
        if self.deterministic_seed == 0 {
            return Err(GpuEvidenceError::Contract(
                "deterministic evidence seed must be nonzero",
            ));
        }
        if self.sensor_profile != SensorProfile::PrivilegedAffordanceV1 {
            return Err(GpuEvidenceError::Contract(
                "Slice A evidence requires privileged-affordance-v1",
            ));
        }
        Ok(self)
    }
}

impl PhenotypeEvidenceManifest {
    pub fn from_phenotype(
        phenotype: &BrainPhenotype,
        capacity: &BrainCapacityClass,
    ) -> Result<Self, GpuEvidenceError> {
        phenotype.validate_against(capacity)?;
        let mut manifest = Self {
            schema_version: GPU_PHENOTYPE_EVIDENCE_MANIFEST_SCHEMA,
            class_id_raw: phenotype.brain_class_id().raw(),
            phenotype_sensor_profile_raw: phenotype.sensor_profile().raw(),
            phenotype_hash: phenotype.phenotype_hash(),
            compile_inputs_digest: phenotype.compiler_inputs_digest(),
            capacity_digest: capacity.canonical_digest(),
            lobe_layout_digest: lobe_layout_digest(phenotype),
            projection_plan_digest: projection_plan_digest(phenotype),
            synapse_payload_digest: synapse_payload_digest(phenotype)?,
            encoder_plan_digest: phenotype.sensor_encoder().canonical_digest(),
            decoder_plan_digest: phenotype.candidate_decoder().canonical_digest(),
            plasticity_plan_digest: explicit_none_digest(PLASTICITY_NONE_DOMAIN),
            replay_capture_plan_digest: explicit_none_digest(REPLAY_CAPTURE_NONE_DOMAIN),
            manifest_digest: [0; 4],
        };
        manifest.manifest_digest = manifest.recompute_manifest_digest();
        manifest.validate(capacity)?;
        Ok(manifest)
    }

    pub fn from_learning_phenotype(
        phenotype: &BrainPhenotype,
        capacity: &BrainCapacityClass,
    ) -> Result<Self, GpuEvidenceError> {
        phenotype.validate_against(capacity)?;
        let mut manifest = Self {
            schema_version: GPU_PHENOTYPE_EVIDENCE_MANIFEST_SCHEMA,
            class_id_raw: phenotype.brain_class_id().raw(),
            phenotype_sensor_profile_raw: phenotype.sensor_profile().raw(),
            phenotype_hash: phenotype.phenotype_hash(),
            compile_inputs_digest: phenotype.compiler_inputs_digest(),
            capacity_digest: capacity.canonical_digest(),
            lobe_layout_digest: lobe_layout_digest(phenotype),
            projection_plan_digest: projection_plan_digest(phenotype),
            synapse_payload_digest: synapse_payload_digest(phenotype)?,
            encoder_plan_digest: phenotype.sensor_encoder().canonical_digest(),
            decoder_plan_digest: phenotype.candidate_decoder().canonical_digest(),
            plasticity_plan_digest: phenotype.plasticity_plan_digest(),
            replay_capture_plan_digest: phenotype.replay_capture_plan().canonical_digest(),
            manifest_digest: [0; 4],
        };
        manifest.manifest_digest = manifest.recompute_manifest_digest();
        manifest.validate(capacity)?;
        Ok(manifest)
    }

    pub fn recompute_manifest_digest(&self) -> [u64; 4] {
        let mut digest = new_manifest_digest();
        encode_manifest_without_digest(&mut digest, self);
        digest.finish256()
    }

    fn validate(&self, capacity: &BrainCapacityClass) -> Result<(), GpuEvidenceError> {
        capacity.validate_contract()?;
        SensorProfile::try_from_raw(self.phenotype_sensor_profile_raw)?;
        if self.schema_version != GPU_PHENOTYPE_EVIDENCE_MANIFEST_SCHEMA
            || self.class_id_raw != capacity.id().raw()
            || self.capacity_digest != capacity.canonical_digest()
            || self.phenotype_hash == PhenotypeHash([0; 4])
            || self.manifest_digest != self.recompute_manifest_digest()
            || manifest_component_digests(self)
                .into_iter()
                .any(|digest| digest == [0; 4])
        {
            return Err(GpuEvidenceError::Contract(
                "phenotype evidence manifest is inconsistent",
            ));
        }
        Ok(())
    }
}

impl GpuSliceAAcceptanceReceipt {
    pub fn recompute_artifact_digest(&self) -> Result<[u64; 4], GpuEvidenceError> {
        let mut digest = new_artifact_digest();
        encode_header_without_artifact_digest(&mut digest, &self.header);
        encode_manifest_with_digest(&mut digest, &self.phenotype_manifest);
        digest.write_utf8(&self.capacity_class);
        digest.write_u16(self.capacity_class_id.raw());
        write_digest4(&mut digest, self.capacity.canonical_digest());
        digest.write_u16(self.fixture_schema_version);
        digest.write_u64(self.deterministic_seed);
        digest.write_bool(self.source_tree_clean);
        digest.write_utf8(&self.backend_api);
        digest.write_utf8(&self.adapter_name);
        encode_hardware(&mut digest, &self.hardware);
        digest.write_bool(self.authoritative);
        digest.write_u8(policy_raw(self.policy_backend));
        digest.write_u32(self.requested_ticks);
        digest.write_u64(self.neural_dispatch_count);
        digest.write_u64(self.gpu_selection_count);
        digest.write_u64(self.sealed_patch_count);
        digest.write_u32(self.compact_readback_bytes);
        digest.write_u32(self.active_tiles);
        digest.write_u32(self.active_synapses);
        digest.write_sequence_len(self.selection_trace.len());
        for selection in &self.selection_trace {
            encode_selection(&mut digest, selection)?;
        }
        encode_replay(&mut digest, &self.replay)?;
        Ok(digest.finish256())
    }

    pub fn validate_in_memory(&self) -> Result<(), GpuEvidenceError> {
        self.validate(false)
    }

    fn validate(&self, require_clean_source: bool) -> Result<(), GpuEvidenceError> {
        self.capacity.validate_contract()?;
        self.phenotype_manifest.validate(&self.capacity)?;
        let expected_slug = capacity_slug(self.capacity.id())?;
        if self.header.artifact_schema != GPU_SLICE_EVIDENCE_ARTIFACT_SCHEMA
            || self.header.slice_raw != GPU_SLICE_A_RAW
            || self.header.class_id_raw != self.capacity.id().raw()
            || self.header.profile_id_raw != 0
            || self.header.profile_schema != 0
            || self.header.status_raw != GPU_EVIDENCE_PASSING_STATUS_RAW
            || !is_lower_hex_oid(&self.header.git_commit)
            || !is_lower_hex_oid(&self.header.source_tree_digest)
            || self.header.phenotype_hash != self.phenotype_manifest.phenotype_hash
            || self.header.phenotype_manifest_digest != self.phenotype_manifest.manifest_digest
            || self.header.capacity_digest != self.capacity.canonical_digest()
            || self.capacity_class != expected_slug
            || self.capacity_class_id != self.capacity.id()
            || self.phenotype_manifest.class_id_raw != self.capacity.id().raw()
            || self.fixture_schema_version != GPU_SLICE_A_FIXTURE_SCHEMA
            || self.deterministic_seed == 0
            || (require_clean_source && !self.source_tree_clean)
            || self.backend_api != "vulkan"
            || self.adapter_name.trim().is_empty()
            || self.hardware.schema_version != GPU_HARDWARE_RECEIPT_SCHEMA_VERSION
            || self.hardware.backend_api != self.backend_api
            || self.hardware.adapter_name != self.adapter_name
            || self.hardware.generation == 0
            || self.hardware.gpu_layout_version == 0
            || self.hardware.backend_version.trim().is_empty()
            || !self.authoritative
            || self.policy_backend != PolicyBackend::NeuralClosedLoopGpu
            || self.requested_ticks == 0
            || self.requested_ticks > GPU_SLICE_A_MAX_TICKS
            || self.neural_dispatch_count != u64::from(self.requested_ticks)
            || self.gpu_selection_count != u64::from(self.requested_ticks)
            || self.sealed_patch_count != u64::from(self.requested_ticks)
            || self.selection_trace.len() != self.requested_ticks as usize
            || self.compact_readback_bytes == 0
            || self.compact_readback_bytes > 64
            || self.active_tiles == 0
            || self.active_tiles > self.capacity.execution().max_active_tiles()
            || self.active_synapses == 0
            || self.active_synapses > self.capacity.execution().max_total_synapses()
            || hardware_digests(&self.hardware)
                .into_iter()
                .any(|digest| digest == [0; 4])
        {
            return Err(GpuEvidenceError::Contract(
                "Slice A evidence header or body is inconsistent",
            ));
        }

        validate_selection_trace(self)?;
        validate_replay(self)?;
        if self.header.artifact_digest != self.recompute_artifact_digest()? {
            return Err(GpuEvidenceError::Contract(
                "Slice A artifact digest does not match its body",
            ));
        }
        Ok(())
    }
}

pub fn run_gpu_closed_loop_acceptance(
    options: GpuClosedLoopAcceptanceOptions,
) -> Result<GpuSliceAAcceptanceReceipt, GpuEvidenceError> {
    let provenance = read_git_provenance()?;
    run_gpu_closed_loop_acceptance_with_provenance(options.validate()?, provenance)
}

pub fn run_and_write_gpu_closed_loop_acceptance(
    options: GpuClosedLoopAcceptanceOptions,
    output: impl AsRef<Path>,
) -> Result<GpuSliceAAcceptanceReceipt, GpuEvidenceError> {
    let options = options.validate()?;
    let output = output.as_ref();
    validate_output_filename(output, options.capacity.id())?;
    let before = read_git_provenance()?;
    if !before.clean {
        return Err(GpuEvidenceError::Git(
            "persistent evidence requires a clean committed worktree".to_string(),
        ));
    }
    let receipt = run_gpu_closed_loop_acceptance_with_provenance(options, before.clone())?;
    let after = read_git_provenance()?;
    if before != after || !after.clean {
        return Err(GpuEvidenceError::Git(
            "source commit or tree changed during evidence capture".to_string(),
        ));
    }
    receipt.validate(true)?;
    atomic_write_receipt(output, &receipt)?;
    let loaded = load_gpu_slice_a_evidence(output)?;
    if loaded != receipt {
        return Err(GpuEvidenceError::Contract(
            "persisted Slice A evidence changed during round trip",
        ));
    }
    Ok(loaded)
}

pub fn load_gpu_slice_a_evidence(
    input: impl AsRef<Path>,
) -> Result<GpuSliceAAcceptanceReceipt, GpuEvidenceError> {
    let input = input.as_ref();
    let metadata = fs::metadata(input)?;
    if metadata.len() == 0 || metadata.len() > GPU_EVIDENCE_MAX_ARTIFACT_BYTES {
        return Err(GpuEvidenceError::Contract(
            "GPU evidence artifact size is outside its bound",
        ));
    }
    let receipt: GpuSliceAAcceptanceReceipt = serde_json::from_slice(&fs::read(input)?)?;
    receipt.validate(true)?;
    Ok(receipt)
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidatedGpuEvidence {
    SliceA(GpuSliceAAcceptanceReceipt),
    SliceB(GpuSliceBAcceptanceReceipt),
    SliceC(GpuMemoryGroundingEvidenceReceipt),
}

impl ValidatedGpuEvidence {
    pub const fn header(&self) -> &GpuSliceEvidenceHeader {
        match self {
            Self::SliceA(receipt) => &receipt.header,
            Self::SliceB(receipt) => &receipt.header,
            Self::SliceC(receipt) => &receipt.header.common,
        }
    }

    pub fn capacity_class(&self) -> &str {
        match self {
            Self::SliceA(receipt) => &receipt.capacity_class,
            Self::SliceB(receipt) => &receipt.capacity_class,
            Self::SliceC(receipt) => &receipt.capacity_class_slug,
        }
    }

    pub fn backend_api(&self) -> &str {
        match self {
            Self::SliceA(receipt) => &receipt.backend_api,
            Self::SliceB(receipt) => &receipt.backend_api,
            Self::SliceC(receipt) => &receipt.header.adapter_backend,
        }
    }

    pub fn adapter_name(&self) -> &str {
        match self {
            Self::SliceA(receipt) => &receipt.adapter_name,
            Self::SliceB(receipt) => &receipt.adapter_name,
            Self::SliceC(receipt) => &receipt.header.adapter_name,
        }
    }

    pub const fn activity_count(&self) -> u64 {
        match self {
            Self::SliceA(receipt) => receipt.neural_dispatch_count,
            Self::SliceB(receipt) => receipt.gpu_learning_dispatches,
            Self::SliceC(receipt) => receipt.gpu_selection_count,
        }
    }
}

pub fn validate_gpu_evidence_file(
    slice_raw: u16,
    input: impl AsRef<Path>,
) -> Result<ValidatedGpuEvidence, GpuEvidenceError> {
    match slice_raw {
        GPU_SLICE_A_RAW => load_gpu_slice_a_evidence(input).map(ValidatedGpuEvidence::SliceA),
        GPU_SLICE_B_RAW => load_gpu_slice_b_evidence(input).map(ValidatedGpuEvidence::SliceB),
        GPU_SLICE_C_RAW => load_gpu_slice_c_evidence(input).map(ValidatedGpuEvidence::SliceC),
        other => Err(GpuEvidenceError::UnsupportedSlice(other)),
    }
}

fn run_gpu_closed_loop_acceptance_with_provenance(
    options: GpuClosedLoopAcceptanceOptions,
    provenance: GitProvenance,
) -> Result<GpuSliceAAcceptanceReceipt, GpuEvidenceError> {
    let tier = tier_for_capacity(options.capacity.id())?;
    let (phenotype, genome, development) = compile_gpu_birth_components(
        options.deterministic_seed,
        tier,
        GPU_EVIDENCE_ORGANISM_ID,
        Tick::ZERO,
        options.sensor_profile,
    )?;
    phenotype.validate_against(&options.capacity)?;
    let phenotype_manifest =
        PhenotypeEvidenceManifest::from_phenotype(&phenotype, &options.capacity)?;
    let initial_state_digest = initial_state_digest(&options, &phenotype, &genome, &development)?;

    let first = run_trial(&options, tier, phenotype.phenotype_hash())?;
    let second = run_trial(&options, tier, phenotype.phenotype_hash())?;
    let first_adapter_digest = adapter_identity_digest(&first.hardware);
    let second_adapter_digest = adapter_identity_digest(&second.hardware);
    if first.hardware.backend_api != "vulkan"
        || second.hardware.backend_api != "vulkan"
        || first_adapter_digest != second_adapter_digest
        || first.trace.len() != second.trace.len()
    {
        return Err(GpuEvidenceError::Contract(
            "same-adapter Vulkan replay precondition failed",
        ));
    }

    let frame_digest = frame_sequence_digest(&first.trace);
    let selected_candidate_digest = candidate_sequence_digest(&first.trace);
    if frame_digest != frame_sequence_digest(&second.trace)
        || selected_candidate_digest != candidate_sequence_digest(&second.trace)
    {
        return Err(GpuEvidenceError::Contract(
            "same-adapter replay selected a different frame or candidate sequence",
        ));
    }
    let first_logit_digest = logit_sequence_digest(&first.trace)?;
    let second_logit_digest = logit_sequence_digest(&second.trace)?;
    let max_abs_error = max_logit_error(&first.trace, &second.trace)?;
    if max_abs_error > GPU_SLICE_A_REPLAY_TOLERANCE {
        return Err(GpuEvidenceError::Contract(
            "same-adapter replay exceeded its declared logit tolerance",
        ));
    }

    let compact_readback_bytes = trace_max(&first.trace, |entry| entry.compact_readback_bytes);
    let active_tiles = trace_max(&first.trace, |entry| entry.active_tiles);
    let active_synapses = trace_max(&first.trace, |entry| entry.active_synapses);
    let mut receipt = GpuSliceAAcceptanceReceipt {
        header: GpuSliceEvidenceHeader {
            artifact_schema: GPU_SLICE_EVIDENCE_ARTIFACT_SCHEMA,
            slice_raw: GPU_SLICE_A_RAW,
            class_id_raw: options.capacity.id().raw(),
            profile_id_raw: 0,
            profile_schema: 0,
            status_raw: GPU_EVIDENCE_PASSING_STATUS_RAW,
            git_commit: provenance.commit,
            source_tree_digest: provenance.tree,
            artifact_digest: [0; 4],
            phenotype_hash: phenotype.phenotype_hash(),
            phenotype_manifest_digest: phenotype_manifest.manifest_digest,
            capacity_digest: options.capacity.canonical_digest(),
        },
        capacity_class: capacity_slug(options.capacity.id())?.to_string(),
        capacity_class_id: options.capacity.id(),
        capacity: options.capacity,
        phenotype_manifest,
        fixture_schema_version: GPU_SLICE_A_FIXTURE_SCHEMA,
        deterministic_seed: options.deterministic_seed,
        source_tree_clean: provenance.clean,
        backend_api: first.hardware.backend_api.clone(),
        adapter_name: first.hardware.adapter_name.clone(),
        hardware: first.hardware,
        authoritative: true,
        policy_backend: PolicyBackend::NeuralClosedLoopGpu,
        requested_ticks: options.requested_ticks,
        neural_dispatch_count: first.metrics.completed_dispatch_count,
        gpu_selection_count: first.metrics.completed_selection_count,
        sealed_patch_count: first.trace.len() as u64,
        compact_readback_bytes,
        active_tiles,
        active_synapses,
        selection_trace: first.trace,
        replay: GpuSameAdapterReplayEvidence {
            adapter_identity_digest: first_adapter_digest,
            initial_state_digest,
            frame_sequence_digest: frame_digest,
            selected_candidate_digest,
            first_logit_digest,
            second_logit_digest,
            tolerance: GPU_SLICE_A_REPLAY_TOLERANCE,
            max_abs_error,
            passed: true,
        },
    };
    receipt.header.artifact_digest = receipt.recompute_artifact_digest()?;
    receipt.validate_in_memory()?;
    Ok(receipt)
}

fn run_trial(
    options: &GpuClosedLoopAcceptanceOptions,
    tier: BrainScaleTier,
    expected_phenotype_hash: PhenotypeHash,
) -> Result<TrialEvidence, GpuEvidenceError> {
    let backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())?;
    let world = acceptance_world(options.deterministic_seed)?;
    let mut runtime = GpuLiveBrainRuntime::new_profiled(
        backend,
        world,
        options.deterministic_seed,
        tier,
        options.sensor_profile,
    )?;
    let hardware = runtime.hardware_receipt().clone();
    let mut trace = Vec::with_capacity(options.requested_ticks as usize);
    for expected_tick in 0..options.requested_ticks {
        let patch_count_before = runtime.sealed_patches().len();
        let summaries = runtime.tick()?;
        if summaries.len() != 1
            || !summaries[0].patch_sealed
            || runtime.sealed_patches().len() != patch_count_before + 1
        {
            return Err(GpuEvidenceError::Contract(
                "production GPU runtime failed to seal exactly one patch",
            ));
        }
        let patch = runtime
            .sealed_patches()
            .last()
            .ok_or(GpuEvidenceError::Contract("sealed patch is missing"))?;
        patch.validate_contract()?;
        let neural = patch.decision().neural_evidence()?;
        let metrics = runtime.evidence_metrics();
        if neural.phenotype_hash != expected_phenotype_hash
            || patch.pre_action().tick != Tick::new(u64::from(expected_tick))
            || patch.pre_action().policy_backend() != PolicyBackend::NeuralClosedLoopGpu
            || patch.pre_action().perception().sensor_profile() != options.sensor_profile
            || metrics.completed_dispatch_count != u64::from(expected_tick) + 1
            || metrics.completed_selection_count != u64::from(expected_tick) + 1
        {
            return Err(GpuEvidenceError::Contract(
                "production GPU runtime evidence diverged from its compiled birth",
            ));
        }
        trace.push(GpuSelectionEvidence {
            tick: patch.pre_action().tick.raw(),
            frame_digest: neural.frame_digest.0,
            candidate_index: neural.candidate_index,
            action_id_raw: neural.action_id.0,
            action_family_raw: neural.action_family.raw(),
            candidate_feature_digest: neural.candidate_feature_digest.0,
            logit: neural.logit,
            active_activation_side: neural.active_activation_side,
            active_tiles: metrics.active_tiles,
            active_synapses: metrics.active_synapses,
            compact_readback_bytes: u32::try_from(metrics.selection_readback_bytes)
                .map_err(|_| GpuEvidenceError::Contract("compact readback does not fit u32"))?,
            outcome_success: patch.outcome().success,
        });
    }
    Ok(TrialEvidence {
        hardware,
        metrics: runtime.evidence_metrics(),
        trace,
    })
}

fn acceptance_world(seed: u64) -> Result<alife_world::HeadlessWorld, ScaffoldContractError> {
    HeadlessScenarioBuilder::new(seed)
        .agent("slice-a-agent", GPU_EVIDENCE_ORGANISM_ID, Vec3f::ZERO)
        .food("slice-a-food", Vec3f::new(1.0, 0.0, 0.0), 0.8)
        .hazard("slice-a-hazard", Vec3f::new(-1.5, 0.0, 0.0), 0.7)
        .obstacle("slice-a-obstacle", Vec3f::new(0.0, 0.0, 2.0), 0.6)
        .token("slice-a-token", Vec3f::new(0.0, 0.0, -2.0), 17)
        .build()
}

fn tier_for_capacity(class_id: BrainClassId) -> Result<BrainScaleTier, GpuEvidenceError> {
    match class_id.raw() {
        1 => Ok(BrainScaleTier::Nano512),
        2 => Ok(BrainScaleTier::Small1024),
        3 => Ok(BrainScaleTier::Standard2048),
        _ => Err(GpuEvidenceError::Contract(
            "evidence class is not a promoted GPU capacity",
        )),
    }
}

pub fn capacity_slug(class_id: BrainClassId) -> Result<&'static str, GpuEvidenceError> {
    match class_id.raw() {
        1 => Ok("n512"),
        2 => Ok("n1024"),
        3 => Ok("n2048"),
        _ => Err(GpuEvidenceError::Contract(
            "capacity class has no production evidence slug",
        )),
    }
}

fn validate_selection_trace(receipt: &GpuSliceAAcceptanceReceipt) -> Result<(), GpuEvidenceError> {
    for (index, selection) in receipt.selection_trace.iter().enumerate() {
        if selection.tick != index as u64
            || selection.frame_digest == [0; 4]
            || usize::from(selection.candidate_index)
                >= usize::from(receipt.capacity.execution().max_candidates())
            || selection.action_id_raw == 0
            || selection.action_family_raw > 7
            || selection.candidate_feature_digest == [0; 2]
            || !selection.logit.is_finite()
            || selection.active_activation_side > 1
            || selection.active_tiles == 0
            || selection.active_tiles > receipt.capacity.execution().max_active_tiles()
            || selection.active_synapses == 0
            || selection.active_synapses > receipt.capacity.execution().max_total_synapses()
            || selection.compact_readback_bytes == 0
            || selection.compact_readback_bytes > 64
        {
            return Err(GpuEvidenceError::Contract(
                "selection trace contains invalid compact GPU evidence",
            ));
        }
    }
    if trace_max(&receipt.selection_trace, |entry| {
        entry.compact_readback_bytes
    }) != receipt.compact_readback_bytes
        || trace_max(&receipt.selection_trace, |entry| entry.active_tiles) != receipt.active_tiles
        || trace_max(&receipt.selection_trace, |entry| entry.active_synapses)
            != receipt.active_synapses
    {
        return Err(GpuEvidenceError::Contract(
            "selection trace maxima disagree with the receipt",
        ));
    }
    Ok(())
}

fn validate_replay(receipt: &GpuSliceAAcceptanceReceipt) -> Result<(), GpuEvidenceError> {
    let replay = &receipt.replay;
    if replay.adapter_identity_digest != adapter_identity_digest(&receipt.hardware)
        || replay.initial_state_digest
            != initial_state_digest_from_receipt(receipt.phenotype_manifest.phenotype_hash, receipt)
        || replay.frame_sequence_digest != frame_sequence_digest(&receipt.selection_trace)
        || replay.selected_candidate_digest != candidate_sequence_digest(&receipt.selection_trace)
        || replay.first_logit_digest != logit_sequence_digest(&receipt.selection_trace)?
        || replay.second_logit_digest == [0; 4]
        || !replay.tolerance.is_finite()
        || replay.tolerance <= 0.0
        || replay.tolerance > 1.0e-3
        || !replay.max_abs_error.is_finite()
        || replay.max_abs_error < 0.0
        || replay.max_abs_error > replay.tolerance
        || !replay.passed
    {
        return Err(GpuEvidenceError::Contract(
            "same-adapter replay evidence is invalid",
        ));
    }
    Ok(())
}

fn trace_max<T: Ord + Copy>(
    trace: &[GpuSelectionEvidence],
    field: impl Fn(&GpuSelectionEvidence) -> T,
) -> T {
    trace
        .iter()
        .map(field)
        .max()
        .expect("validated evidence traces are nonempty")
}
