//! Canonical Slice C memory, grounding, and bounded-saturation evidence contracts.

mod acceptance;
pub use acceptance::*;

use std::{fs, path::Path};

use alife_core::{
    BrainCapacityClass, BrainClassId, CanonicalDigestBuilder, PhenotypeHash, PolicyBackend,
    SensorProfile, SensorProfileIdentity, TopologyCounts, Validate,
};
use alife_gpu_backend::{GpuHardwareReceipt, GPU_HARDWARE_RECEIPT_SCHEMA_VERSION};
use serde::{Deserialize, Serialize};

use super::{
    canonical::{
        encode_hardware, encode_header_without_artifact_digest, encode_manifest_with_digest,
        hardware_digests, write_digest4,
    },
    capacity_slug, is_lower_hex_oid, GpuEvidenceError, GpuSliceEvidenceHeader,
    PhenotypeEvidenceManifest, GPU_EVIDENCE_MAX_ARTIFACT_BYTES, GPU_EVIDENCE_PASSING_STATUS_RAW,
    GPU_SLICE_EVIDENCE_ARTIFACT_SCHEMA,
};

pub const GPU_SLICE_C_RAW: u16 = 3;
pub const GPU_EVIDENCE_BACKEND_API_VERSION: u16 = 1;
pub const GPU_EVIDENCE_BACKEND_API_SLUG: &str = "gpu-closed-loop-v1";

const SLICE_C_ARTIFACT_DOMAIN: &[u8] = b"alife.gpu.evidence.slice-c-artifact.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfiledBehaviorReceiptHeader {
    #[serde(flatten)]
    pub common: GpuSliceEvidenceHeader,
    pub artifact_slug: String,
    pub backend_api_version: u16,
    pub backend_api_slug: String,
    pub adapter_name: String,
    pub adapter_backend: String,
    pub run_seed: u64,
}

impl ProfiledBehaviorReceiptHeader {
    pub fn aggregate_key(&self) -> String {
        let phenotype = self
            .common
            .phenotype_hash
            .0
            .map(|word| format!("{word:016x}"))
            .join("");
        format!(
            "{}:{phenotype}:{}",
            self.artifact_slug, self.common.source_tree_digest
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyCapacityReceipt {
    pub max_concepts: u32,
    pub max_edges: u32,
    pub max_simplexes: u32,
    pub max_unresolved_gaps: u32,
    pub max_bindings_per_kind: u32,
}

impl TopologyCapacityReceipt {
    pub const fn contains(&self, counts: TopologyCounts, observed_max_bindings: u32) -> bool {
        self.max_concepts > 0
            && self.max_edges > 0
            && self.max_simplexes > 0
            && self.max_unresolved_gaps > 0
            && self.max_bindings_per_kind > 0
            && counts.concepts <= self.max_concepts
            && counts.edges <= self.max_edges
            && counts.simplexes <= self.max_simplexes
            && counts.unresolved_gaps <= self.max_unresolved_gaps
            && observed_max_bindings <= self.max_bindings_per_kind
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapacitySaturationEvidence {
    pub grounded_semantic_label_channels_nonzero: u32,
    pub memory_records: u32,
    pub memory_capacity: u32,
    pub memory_merges: u64,
    pub memory_evictions: u64,
    pub tracked_object_records: u32,
    pub tracked_object_capacity: u32,
    pub tracked_object_evictions: u64,
    pub tracked_object_id_reuse_count: u64,
    pub topology_counts: TopologyCounts,
    pub topology_capacity: TopologyCapacityReceipt,
    pub max_observed_bindings_per_kind: u32,
    pub topology_degradations: u64,
    pub terminal_capacity_errors: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryContextProbeEvidence {
    pub phenotype_hash: PhenotypeHash,
    pub phenotype_manifest_digest: [u64; 4],
    pub activation_digest: [u64; 4],
    pub recurrent_activation_digest: [u64; 4],
    pub lifetime_weight_digest: [u64; 4],
    pub fast_weight_digest: [u64; 4],
    pub eligibility_digest: [u64; 4],
    pub poisoned_ingest_delta: f32,
    pub safe_ingest_delta: f32,
    pub selected_candidate: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuMemoryGroundingEvidenceReceipt {
    #[serde(flatten)]
    pub header: ProfiledBehaviorReceiptHeader,
    pub phenotype_manifest: PhenotypeEvidenceManifest,
    pub sensor_profile: SensorProfileIdentity,
    pub capacity_class_slug: String,
    pub policy_backend: PolicyBackend,
    pub hardware: GpuHardwareReceipt,
    pub completed_ticks: u64,
    pub completed_waking_ticks: u64,
    pub gpu_selection_count: u64,
    pub poisoned_ingest_candidate: u16,
    pub post_learning_selection: u16,
    pub poisoned_ingest_logit_before: f32,
    pub poisoned_ingest_logit_after: f32,
    pub poisoned_avoid_logit_before: f32,
    pub poisoned_avoid_logit_after: f32,
    pub poisoned_ingest_delta: f32,
    pub safe_ingest_delta: f32,
    pub cyan_ingest_target_latent: [f32; 8],
    pub cyan_avoid_target_latent: [f32; 8],
    pub cyan_ingest_family_value: [f32; 4],
    pub cyan_avoid_family_value: [f32; 4],
    pub amber_target_latent: [f32; 8],
    pub memory_enabled: MemoryContextProbeEvidence,
    pub memory_ablated: MemoryContextProbeEvidence,
    pub capacity_saturation: Option<CapacitySaturationEvidence>,
    pub compact_readback_bytes: u32,
    pub tolerance: f32,
}

impl GpuMemoryGroundingEvidenceReceipt {
    pub fn aggregate_key(&self) -> String {
        self.header.aggregate_key()
    }

    pub fn recompute_artifact_digest(&self) -> Result<[u64; 4], GpuEvidenceError> {
        let mut digest = CanonicalDigestBuilder::new(SLICE_C_ARTIFACT_DOMAIN);
        encode_header_without_artifact_digest(&mut digest, &self.header.common);
        digest.write_utf8(&self.header.artifact_slug);
        digest.write_u16(self.header.backend_api_version);
        digest.write_utf8(&self.header.backend_api_slug);
        digest.write_utf8(&self.header.adapter_name);
        digest.write_utf8(&self.header.adapter_backend);
        digest.write_u64(self.header.run_seed);
        encode_manifest_with_digest(&mut digest, &self.phenotype_manifest);
        digest.write_u16(self.sensor_profile.profile_id.raw());
        digest.write_u16(self.sensor_profile.profile_schema_version);
        digest.write_u16(self.sensor_profile.sensory_abi_version);
        digest.write_utf8(&self.capacity_class_slug);
        digest.write_u8(policy_raw(self.policy_backend));
        encode_hardware(&mut digest, &self.hardware);
        digest.write_u64(self.completed_ticks);
        digest.write_u64(self.completed_waking_ticks);
        digest.write_u64(self.gpu_selection_count);
        digest.write_u16(self.poisoned_ingest_candidate);
        digest.write_u16(self.post_learning_selection);
        digest.write_f32(self.poisoned_ingest_logit_before)?;
        digest.write_f32(self.poisoned_ingest_logit_after)?;
        digest.write_f32(self.poisoned_avoid_logit_before)?;
        digest.write_f32(self.poisoned_avoid_logit_after)?;
        digest.write_f32(self.poisoned_ingest_delta)?;
        digest.write_f32(self.safe_ingest_delta)?;
        encode_f32_array(&mut digest, &self.cyan_ingest_target_latent)?;
        encode_f32_array(&mut digest, &self.cyan_avoid_target_latent)?;
        encode_f32_array(&mut digest, &self.cyan_ingest_family_value)?;
        encode_f32_array(&mut digest, &self.cyan_avoid_family_value)?;
        encode_f32_array(&mut digest, &self.amber_target_latent)?;
        encode_probe(&mut digest, &self.memory_enabled)?;
        encode_probe(&mut digest, &self.memory_ablated)?;
        match &self.capacity_saturation {
            Some(saturation) => {
                digest.write_some();
                encode_saturation(&mut digest, saturation);
            }
            None => digest.write_none(),
        }
        digest.write_u32(self.compact_readback_bytes);
        digest.write_f32(self.tolerance)?;
        Ok(digest.finish256())
    }

    pub fn validate_in_memory(&self) -> Result<(), GpuEvidenceError> {
        let capacity =
            BrainCapacityClass::production_for_id(BrainClassId(self.header.common.class_id_raw))?;
        capacity.validate_contract()?;
        self.sensor_profile.validate_contract()?;
        let profile = self.sensor_profile.profile()?;
        self.phenotype_manifest.validate_for_capacity(&capacity)?;
        let expected_artifact_slug = slice_c_artifact_slug(profile, capacity.id())?;
        if self.header.common.artifact_schema != GPU_SLICE_EVIDENCE_ARTIFACT_SCHEMA
            || self.header.common.slice_raw != GPU_SLICE_C_RAW
            || self.header.common.status_raw != GPU_EVIDENCE_PASSING_STATUS_RAW
            || self.header.common.profile_id_raw != self.sensor_profile.profile_id.raw()
            || self.header.common.profile_schema != self.sensor_profile.profile_schema_version
            || !is_lower_hex_oid(&self.header.common.git_commit)
            || !is_lower_hex_oid(&self.header.common.source_tree_digest)
            || self.header.common.phenotype_hash != self.phenotype_manifest.phenotype_hash
            || self.header.common.phenotype_manifest_digest
                != self.phenotype_manifest.manifest_digest
            || self.header.common.capacity_digest != capacity.canonical_digest()
            || self.phenotype_manifest.class_id_raw != capacity.id().raw()
            || self.phenotype_manifest.phenotype_sensor_profile_raw != profile.raw()
            || self.header.artifact_slug != expected_artifact_slug
            || self.header.backend_api_version != GPU_EVIDENCE_BACKEND_API_VERSION
            || self.header.backend_api_slug != GPU_EVIDENCE_BACKEND_API_SLUG
            || self.header.run_seed == 0
            || self.header.adapter_name.trim().is_empty()
            || !self.header.adapter_backend.eq_ignore_ascii_case("vulkan")
            || self.capacity_class_slug != capacity_slug(capacity.id())?
            || self.policy_backend != PolicyBackend::NeuralClosedLoopGpu
            || self.hardware.schema_version != GPU_HARDWARE_RECEIPT_SCHEMA_VERSION
            || self.hardware.generation == 0
            || self.hardware.gpu_layout_version == 0
            || self.hardware.backend_version.trim().is_empty()
            || self.hardware.adapter_name != self.header.adapter_name
            || self.hardware.backend_api != self.header.adapter_backend
            || hardware_digests(&self.hardware)
                .into_iter()
                .any(|value| value == [0; 4])
            || self.completed_waking_ticks == 0
            || self.completed_waking_ticks > self.completed_ticks
            || self.gpu_selection_count != self.completed_waking_ticks
            || self.compact_readback_bytes == 0
            || self.compact_readback_bytes > 64
            || !self.tolerance.is_finite()
            || self.tolerance <= 0.0
            || self.tolerance > 1.0e-3
        {
            return Err(GpuEvidenceError::Contract(
                "Slice C evidence header or runtime provenance is inconsistent",
            ));
        }

        self.validate_behavior(profile)?;
        self.validate_profile_evidence(profile)?;
        if self.header.common.artifact_digest != self.recompute_artifact_digest()? {
            return Err(GpuEvidenceError::Contract(
                "Slice C artifact digest does not match its body",
            ));
        }
        Ok(())
    }

    fn validate_behavior(&self, profile: SensorProfile) -> Result<(), GpuEvidenceError> {
        let scalars = [
            self.poisoned_ingest_logit_before,
            self.poisoned_ingest_logit_after,
            self.poisoned_avoid_logit_before,
            self.poisoned_avoid_logit_after,
            self.poisoned_ingest_delta,
            self.safe_ingest_delta,
        ];
        if scalars.into_iter().any(|value| !value.is_finite())
            || arrays_nonfinite(&self.cyan_ingest_target_latent)
            || arrays_nonfinite(&self.cyan_avoid_target_latent)
            || arrays_nonfinite(&self.cyan_ingest_family_value)
            || arrays_nonfinite(&self.cyan_avoid_family_value)
            || arrays_nonfinite(&self.amber_target_latent)
            || self.poisoned_ingest_candidate == self.post_learning_selection
            || self.poisoned_ingest_logit_after >= self.poisoned_ingest_logit_before
            || self.safe_ingest_delta.abs() >= self.poisoned_ingest_delta.abs()
            || (self.poisoned_ingest_logit_after
                - self.poisoned_ingest_logit_before
                - self.poisoned_ingest_delta)
                .abs()
                > self.tolerance
            || self.cyan_ingest_family_value[2] <= 0.0
            || self.cyan_avoid_family_value != [0.0; 4]
            || self.amber_target_latent != [0.0; 8]
        {
            return Err(GpuEvidenceError::Contract(
                "Slice C target-conditional behavior evidence is inconsistent",
            ));
        }
        match profile {
            SensorProfile::GroundedObjectSlotsV1 => {
                if self.cyan_ingest_target_latent != self.cyan_avoid_target_latent
                    || self.cyan_ingest_target_latent[2] <= 0.0
                    || self.poisoned_avoid_logit_after <= self.poisoned_avoid_logit_before
                    || self.memory_enabled.selected_candidate
                        == self.memory_ablated.selected_candidate
                {
                    return Err(GpuEvidenceError::Contract(
                        "grounded Slice C behavior lacks shared target-local pain context",
                    ));
                }
            }
            SensorProfile::PrivilegedAffordanceV1 => {
                if self.cyan_ingest_target_latent != [0.0; 8]
                    || self.cyan_avoid_target_latent != [0.0; 8]
                    || (self.poisoned_avoid_logit_after - self.poisoned_avoid_logit_before).abs()
                        > self.tolerance
                {
                    return Err(GpuEvidenceError::Contract(
                        "privileged Slice C behavior claimed grounded target-local context",
                    ));
                }
            }
        }
        validate_probe(&self.memory_enabled, &self.header.common)?;
        validate_probe(&self.memory_ablated, &self.header.common)?;
        if self.memory_enabled.phenotype_hash != self.memory_ablated.phenotype_hash
            || self.memory_enabled.phenotype_manifest_digest
                != self.memory_ablated.phenotype_manifest_digest
            || self.memory_enabled.activation_digest != self.memory_ablated.activation_digest
            || self.memory_enabled.recurrent_activation_digest
                != self.memory_ablated.recurrent_activation_digest
            || self.memory_enabled.lifetime_weight_digest
                != self.memory_ablated.lifetime_weight_digest
            || self.memory_enabled.fast_weight_digest != self.memory_ablated.fast_weight_digest
            || self.memory_enabled.eligibility_digest != self.memory_ablated.eligibility_digest
            || (self.memory_enabled.poisoned_ingest_delta - self.poisoned_ingest_delta).abs()
                > self.tolerance
            || (self.memory_enabled.safe_ingest_delta - self.safe_ingest_delta).abs()
                > self.tolerance
            || self.memory_enabled.selected_candidate != self.post_learning_selection
            || self.memory_enabled.poisoned_ingest_delta
                >= self.memory_ablated.poisoned_ingest_delta - self.tolerance
            || (self.memory_enabled.safe_ingest_delta - self.memory_ablated.safe_ingest_delta).abs()
                > self.tolerance
        {
            return Err(GpuEvidenceError::Contract(
                "Slice C enabled and ablated memory probes are not comparable",
            ));
        }
        Ok(())
    }

    fn validate_profile_evidence(&self, profile: SensorProfile) -> Result<(), GpuEvidenceError> {
        match profile {
            SensorProfile::GroundedObjectSlotsV1 => {
                let saturation =
                    self.capacity_saturation
                        .as_ref()
                        .ok_or(GpuEvidenceError::Contract(
                            "grounded Slice C evidence requires bounded saturation evidence",
                        ))?;
                if self.completed_ticks != 10_240
                    || saturation.grounded_semantic_label_channels_nonzero != 0
                    || saturation.memory_capacity == 0
                    || saturation.memory_records > saturation.memory_capacity
                    || saturation.memory_merges + saturation.memory_evictions == 0
                    || saturation.tracked_object_capacity == 0
                    || saturation.tracked_object_records > saturation.tracked_object_capacity
                    || saturation.tracked_object_evictions == 0
                    || saturation.tracked_object_id_reuse_count != 0
                    || !saturation.topology_capacity.contains(
                        saturation.topology_counts,
                        saturation.max_observed_bindings_per_kind,
                    )
                    || saturation.topology_degradations == 0
                    || saturation.terminal_capacity_errors != 0
                {
                    return Err(GpuEvidenceError::Contract(
                        "grounded Slice C saturation evidence is inconsistent",
                    ));
                }
            }
            SensorProfile::PrivilegedAffordanceV1 => {
                if self.completed_ticks != 64 || self.capacity_saturation.is_some() {
                    return Err(GpuEvidenceError::Contract(
                        "privileged Slice C evidence made a grounded saturation claim",
                    ));
                }
            }
        }
        Ok(())
    }
}

pub fn load_gpu_slice_c_evidence(
    input: impl AsRef<Path>,
) -> Result<GpuMemoryGroundingEvidenceReceipt, GpuEvidenceError> {
    let input = input.as_ref();
    let metadata = fs::metadata(input)?;
    if metadata.len() == 0 || metadata.len() > GPU_EVIDENCE_MAX_ARTIFACT_BYTES {
        return Err(GpuEvidenceError::Contract(
            "GPU evidence artifact size is outside its bound",
        ));
    }
    let receipt: GpuMemoryGroundingEvidenceReceipt = serde_json::from_slice(&fs::read(input)?)?;
    receipt.validate_in_memory()?;
    Ok(receipt)
}

pub fn sensor_profile_slug(profile: SensorProfile) -> &'static str {
    match profile {
        SensorProfile::PrivilegedAffordanceV1 => "privileged-affordance-v1",
        SensorProfile::GroundedObjectSlotsV1 => "grounded-object-slots-v1",
    }
}

pub fn slice_c_artifact_slug(
    profile: SensorProfile,
    class_id: BrainClassId,
) -> Result<String, GpuEvidenceError> {
    Ok(format!(
        "gpu-memory-grounding-slice-c-{}-{}",
        sensor_profile_slug(profile),
        capacity_slug(class_id)?
    ))
}

fn validate_probe(
    probe: &MemoryContextProbeEvidence,
    header: &GpuSliceEvidenceHeader,
) -> Result<(), GpuEvidenceError> {
    if probe.phenotype_hash != header.phenotype_hash
        || probe.phenotype_manifest_digest != header.phenotype_manifest_digest
        || [
            probe.activation_digest,
            probe.recurrent_activation_digest,
            probe.lifetime_weight_digest,
            probe.fast_weight_digest,
            probe.eligibility_digest,
        ]
        .into_iter()
        .any(|value| value == [0; 4])
        || !probe.poisoned_ingest_delta.is_finite()
        || !probe.safe_ingest_delta.is_finite()
    {
        return Err(GpuEvidenceError::Contract(
            "Slice C memory-context probe identity is inconsistent",
        ));
    }
    Ok(())
}

fn encode_probe(
    digest: &mut CanonicalDigestBuilder,
    probe: &MemoryContextProbeEvidence,
) -> Result<(), GpuEvidenceError> {
    write_digest4(digest, probe.phenotype_hash.0);
    write_digest4(digest, probe.phenotype_manifest_digest);
    write_digest4(digest, probe.activation_digest);
    write_digest4(digest, probe.recurrent_activation_digest);
    write_digest4(digest, probe.lifetime_weight_digest);
    write_digest4(digest, probe.fast_weight_digest);
    write_digest4(digest, probe.eligibility_digest);
    digest.write_f32(probe.poisoned_ingest_delta)?;
    digest.write_f32(probe.safe_ingest_delta)?;
    digest.write_u16(probe.selected_candidate);
    Ok(())
}

fn encode_saturation(digest: &mut CanonicalDigestBuilder, value: &CapacitySaturationEvidence) {
    digest.write_u32(value.grounded_semantic_label_channels_nonzero);
    digest.write_u32(value.memory_records);
    digest.write_u32(value.memory_capacity);
    digest.write_u64(value.memory_merges);
    digest.write_u64(value.memory_evictions);
    digest.write_u32(value.tracked_object_records);
    digest.write_u32(value.tracked_object_capacity);
    digest.write_u64(value.tracked_object_evictions);
    digest.write_u64(value.tracked_object_id_reuse_count);
    digest.write_u32(value.topology_counts.concepts);
    digest.write_u32(value.topology_counts.edges);
    digest.write_u32(value.topology_counts.simplexes);
    digest.write_u32(value.topology_counts.unresolved_gaps);
    digest.write_u32(value.topology_capacity.max_concepts);
    digest.write_u32(value.topology_capacity.max_edges);
    digest.write_u32(value.topology_capacity.max_simplexes);
    digest.write_u32(value.topology_capacity.max_unresolved_gaps);
    digest.write_u32(value.topology_capacity.max_bindings_per_kind);
    digest.write_u32(value.max_observed_bindings_per_kind);
    digest.write_u64(value.topology_degradations);
    digest.write_u64(value.terminal_capacity_errors);
}

fn encode_f32_array<const N: usize>(
    digest: &mut CanonicalDigestBuilder,
    values: &[f32; N],
) -> Result<(), GpuEvidenceError> {
    digest.write_sequence_len(N);
    for value in values {
        digest.write_f32(*value)?;
    }
    Ok(())
}

fn arrays_nonfinite<const N: usize>(values: &[f32; N]) -> bool {
    values.iter().any(|value| !value.is_finite())
}

fn policy_raw(policy: PolicyBackend) -> u8 {
    match policy {
        PolicyBackend::NeuralClosedLoopGpu => 1,
        PolicyBackend::HeuristicBaseline => 2,
    }
}
