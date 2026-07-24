//! Production runtime-profile, adapter-budget, and GPU admission receipts.

use alife_core::{
    BrainCapacityClass, BrainExecutionBudget, CanonicalDigestBuilder, ScaffoldContractError,
};
use serde::{Deserialize, Serialize};

const ADMISSION_SCHEMA_VERSION: u16 = 1;
const PROFILE_DIGEST_DOMAIN: &[u8] = b"alife.gpu.runtime-profile.v1";
const ALLOCATION_EVENT_DIGEST_DOMAIN: &[u8] = b"alife.gpu.allocation-event.v1";
const REQUIRED_FEATURES_DIGEST_DOMAIN: &[u8] = b"alife.gpu.required-features.v1";
const REQUIRED_LIMITS_DIGEST_DOMAIN: &[u8] = b"alife.gpu.required-limits.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuRuntimeProfile {
    pub schema_version: u16,
    pub profile_id: u16,
    pub logical_neural_heap_budget_bytes: u64,
    pub physical_allocation_ceiling_bytes: u64,
    pub max_hot_brains: u32,
    pub max_in_flight_batches: u16,
    pub growth_chunk_slots: u16,
    pub retain_empty_chunks: u8,
    pub reserved: [u8; 7],
}

impl GpuRuntimeProfile {
    pub const fn production_v1() -> Self {
        Self {
            schema_version: ADMISSION_SCHEMA_VERSION,
            profile_id: 1,
            logical_neural_heap_budget_bytes: 2 * 1024 * 1024 * 1024,
            physical_allocation_ceiling_bytes: 2 * 1024 * 1024 * 1024,
            max_hot_brains: 500,
            max_in_flight_batches: 8,
            growth_chunk_slots: 32,
            retain_empty_chunks: 1,
            reserved: [0; 7],
        }
    }

    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != ADMISSION_SCHEMA_VERSION
            || self.profile_id == 0
            || self.logical_neural_heap_budget_bytes == 0
            || self.physical_allocation_ceiling_bytes == 0
            || self.max_hot_brains == 0
            || self.max_in_flight_batches == 0
            || self.growth_chunk_slots == 0
            || u32::from(self.growth_chunk_slots) > self.max_hot_brains
            || self.retain_empty_chunks > 1
            || self.reserved != [0; 7]
        {
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        Ok(())
    }

    pub fn canonical_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        self.validate_contract()?;
        let mut digest = CanonicalDigestBuilder::new(PROFILE_DIGEST_DOMAIN);
        digest.write_u16(self.schema_version);
        digest.write_u16(self.profile_id);
        digest.write_u64(self.logical_neural_heap_budget_bytes);
        digest.write_u64(self.physical_allocation_ceiling_bytes);
        digest.write_u32(self.max_hot_brains);
        digest.write_u16(self.max_in_flight_batches);
        digest.write_u16(self.growth_chunk_slots);
        digest.write_u8(self.retain_empty_chunks);
        for byte in self.reserved {
            digest.write_u8(byte);
        }
        Ok(digest.finish256())
    }

    /// Accepts a checkpoint envelope only when its runtime packing can be
    /// deterministically re-admitted without changing portable brain state.
    ///
    /// The sole v1 migration is from the former four-slot production chunks to
    /// the current fixed 32-slot arenas and wider heap ceilings. Checkpoints do
    /// not persist arena-local offsets, so this migration only changes fresh
    /// allocation; phenotype, learning, sleep, memory, and activity state keep
    /// their exact portable identities.
    pub fn accepts_portable_checkpoint_profile(
        &self,
        saved_profile_id: u16,
        saved_profile_digest: [u64; 4],
    ) -> Result<bool, ScaffoldContractError> {
        self.validate_contract()?;
        if saved_profile_id != self.profile_id {
            return Ok(false);
        }
        if saved_profile_digest == self.canonical_digest()? {
            return Ok(true);
        }
        if *self != Self::production_v1() {
            return Ok(false);
        }
        let previous_v1 = Self {
            logical_neural_heap_budget_bytes: 512 * 1024 * 1024,
            physical_allocation_ceiling_bytes: 1024 * 1024 * 1024,
            growth_chunk_slots: 4,
            ..Self::production_v1()
        };
        Ok(saved_profile_digest == previous_v1.canonical_digest()?)
    }
}

impl Default for GpuRuntimeProfile {
    fn default() -> Self {
        Self::production_v1()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuRuntimeBudget {
    pub schema_version: u16,
    pub profile_id: u16,
    pub gpu_layout_version: u16,
    pub required_limits_schema_version: u16,
    pub logical_neural_heap_budget_bytes: u64,
    pub physical_allocation_ceiling_bytes: u64,
    pub max_hot_brains: u32,
    pub max_in_flight_batches: u16,
    pub growth_chunk_slots: u16,
    pub storage_alignment_bytes: u32,
    pub uniform_alignment_bytes: u32,
    pub copy_buffer_alignment_bytes: u32,
    pub copy_bytes_per_row_alignment: u32,
    pub max_buffer_size: u64,
    pub max_storage_buffer_binding_size: u64,
    pub max_bind_groups: u32,
    pub max_bindings_per_bind_group: u32,
    pub max_storage_buffers_per_shader_stage: u32,
    pub max_uniform_buffers_per_shader_stage: u32,
    pub max_dynamic_storage_buffers_per_pipeline_layout: u32,
    pub max_dynamic_uniform_buffers_per_pipeline_layout: u32,
    pub max_compute_workgroup_storage_size: u32,
    pub max_compute_workgroup_size_x: u32,
    pub max_compute_workgroup_size_y: u32,
    pub max_compute_workgroup_size_z: u32,
    pub max_compute_invocations_per_workgroup: u32,
    pub max_compute_workgroups_per_dimension: u32,
    pub required_feature_mask_words: u8,
    pub required_feature_mask: u64,
    pub available_feature_mask: u64,
    pub profile_digest: [u64; 4],
    pub adapter_limits_digest: [u64; 4],
}

impl GpuRuntimeBudget {
    pub(crate) fn from_device(
        profile: GpuRuntimeProfile,
        features: wgpu::Features,
        limits: &wgpu::Limits,
        adapter_limits_digest: [u64; 4],
    ) -> Result<Self, ScaffoldContractError> {
        profile.validate_contract()?;
        let execution = BrainCapacityClass::n512().execution().to_owned();
        let available_feature_mask = u64::from(features.contains(wgpu::Features::TIMESTAMP_QUERY));
        let budget = Self {
            schema_version: ADMISSION_SCHEMA_VERSION,
            profile_id: profile.profile_id,
            gpu_layout_version: execution.gpu_layout_version(),
            required_limits_schema_version: execution.required_limits_schema_version(),
            logical_neural_heap_budget_bytes: profile.logical_neural_heap_budget_bytes,
            physical_allocation_ceiling_bytes: profile.physical_allocation_ceiling_bytes,
            max_hot_brains: profile.max_hot_brains,
            max_in_flight_batches: profile.max_in_flight_batches,
            growth_chunk_slots: profile.growth_chunk_slots,
            storage_alignment_bytes: limits.min_storage_buffer_offset_alignment,
            uniform_alignment_bytes: limits.min_uniform_buffer_offset_alignment,
            copy_buffer_alignment_bytes: execution.copy_buffer_alignment_bytes(),
            copy_bytes_per_row_alignment: execution.copy_bytes_per_row_alignment(),
            max_buffer_size: limits.max_buffer_size,
            max_storage_buffer_binding_size: limits.max_storage_buffer_binding_size,
            max_bind_groups: limits.max_bind_groups,
            max_bindings_per_bind_group: limits.max_bindings_per_bind_group,
            max_storage_buffers_per_shader_stage: limits.max_storage_buffers_per_shader_stage,
            max_uniform_buffers_per_shader_stage: limits.max_uniform_buffers_per_shader_stage,
            max_dynamic_storage_buffers_per_pipeline_layout: limits
                .max_dynamic_storage_buffers_per_pipeline_layout,
            max_dynamic_uniform_buffers_per_pipeline_layout: limits
                .max_dynamic_uniform_buffers_per_pipeline_layout,
            max_compute_workgroup_storage_size: limits.max_compute_workgroup_storage_size,
            max_compute_workgroup_size_x: limits.max_compute_workgroup_size_x,
            max_compute_workgroup_size_y: limits.max_compute_workgroup_size_y,
            max_compute_workgroup_size_z: limits.max_compute_workgroup_size_z,
            max_compute_invocations_per_workgroup: limits.max_compute_invocations_per_workgroup,
            max_compute_workgroups_per_dimension: limits.max_compute_workgroups_per_dimension,
            required_feature_mask_words: execution.required_feature_mask_words(),
            required_feature_mask: execution.required_feature_mask(),
            available_feature_mask,
            profile_digest: profile.canonical_digest()?,
            adapter_limits_digest,
        };
        for capacity in BrainCapacityClass::production_classes() {
            budget.validate_for(capacity.execution())?;
        }
        Ok(budget)
    }

    #[cfg(feature = "gpu-tests")]
    pub fn minimum_for_testing(
        profile: GpuRuntimeProfile,
        execution: &BrainExecutionBudget,
    ) -> Result<Self, ScaffoldContractError> {
        profile.validate_contract()?;
        let budget = Self {
            schema_version: ADMISSION_SCHEMA_VERSION,
            profile_id: profile.profile_id,
            gpu_layout_version: execution.gpu_layout_version(),
            required_limits_schema_version: execution.required_limits_schema_version(),
            logical_neural_heap_budget_bytes: profile.logical_neural_heap_budget_bytes,
            physical_allocation_ceiling_bytes: profile.physical_allocation_ceiling_bytes,
            max_hot_brains: profile.max_hot_brains,
            max_in_flight_batches: profile.max_in_flight_batches,
            growth_chunk_slots: profile.growth_chunk_slots,
            storage_alignment_bytes: execution.storage_offset_alignment_bytes(),
            uniform_alignment_bytes: execution.uniform_offset_alignment_bytes(),
            copy_buffer_alignment_bytes: execution.copy_buffer_alignment_bytes(),
            copy_bytes_per_row_alignment: execution.copy_bytes_per_row_alignment(),
            max_buffer_size: execution.required_max_buffer_size(),
            max_storage_buffer_binding_size: execution.required_max_storage_buffer_binding_size(),
            max_bind_groups: execution.required_max_bind_groups(),
            max_bindings_per_bind_group: execution.required_max_bindings_per_bind_group(),
            max_storage_buffers_per_shader_stage: execution
                .required_max_storage_buffers_per_shader_stage(),
            max_uniform_buffers_per_shader_stage: execution
                .required_max_uniform_buffers_per_shader_stage(),
            max_dynamic_storage_buffers_per_pipeline_layout: execution
                .required_max_dynamic_storage_buffers_per_pipeline_layout(),
            max_dynamic_uniform_buffers_per_pipeline_layout: execution
                .required_max_dynamic_uniform_buffers_per_pipeline_layout(),
            max_compute_workgroup_storage_size: execution
                .required_max_compute_workgroup_storage_size(),
            max_compute_workgroup_size_x: execution.required_max_compute_workgroup_size_x(),
            max_compute_workgroup_size_y: execution.required_max_compute_workgroup_size_y(),
            max_compute_workgroup_size_z: execution.required_max_compute_workgroup_size_z(),
            max_compute_invocations_per_workgroup: execution
                .required_max_compute_invocations_per_workgroup(),
            max_compute_workgroups_per_dimension: execution
                .required_max_compute_workgroups_per_dimension(),
            required_feature_mask_words: execution.required_feature_mask_words(),
            required_feature_mask: execution.required_feature_mask(),
            available_feature_mask: execution.required_feature_mask(),
            profile_digest: profile.canonical_digest()?,
            adapter_limits_digest: [1; 4],
        };
        budget.validate_for(execution)?;
        Ok(budget)
    }

    pub fn validate_for(
        &self,
        execution: &BrainExecutionBudget,
    ) -> Result<(), ScaffoldContractError> {
        if self.schema_version != ADMISSION_SCHEMA_VERSION
            || self.profile_id == 0
            || self.gpu_layout_version != execution.gpu_layout_version()
            || self.required_limits_schema_version != execution.required_limits_schema_version()
            || self.logical_neural_heap_budget_bytes == 0
            || self.physical_allocation_ceiling_bytes == 0
            || self.max_hot_brains == 0
            || self.max_in_flight_batches == 0
            || self.growth_chunk_slots == 0
            || u32::from(self.growth_chunk_slots) > self.max_hot_brains
            || self.required_feature_mask_words != 1
            || self.required_feature_mask_words != execution.required_feature_mask_words()
            || self.required_feature_mask != execution.required_feature_mask()
            || self.available_feature_mask & self.required_feature_mask
                != self.required_feature_mask
            || self.max_buffer_size < execution.required_max_buffer_size()
            || self.max_storage_buffer_binding_size
                < execution.required_max_storage_buffer_binding_size()
            || self.max_bind_groups < execution.required_max_bind_groups()
            || self.max_bindings_per_bind_group < execution.required_max_bindings_per_bind_group()
            || self.max_storage_buffers_per_shader_stage
                < execution.required_max_storage_buffers_per_shader_stage()
            || self.max_uniform_buffers_per_shader_stage
                < execution.required_max_uniform_buffers_per_shader_stage()
            || self.max_dynamic_storage_buffers_per_pipeline_layout
                < execution.required_max_dynamic_storage_buffers_per_pipeline_layout()
            || self.max_dynamic_uniform_buffers_per_pipeline_layout
                < execution.required_max_dynamic_uniform_buffers_per_pipeline_layout()
            || self.max_compute_workgroup_storage_size
                < execution.required_max_compute_workgroup_storage_size()
            || self.max_compute_workgroup_size_x < execution.required_max_compute_workgroup_size_x()
            || self.max_compute_workgroup_size_y < execution.required_max_compute_workgroup_size_y()
            || self.max_compute_workgroup_size_z < execution.required_max_compute_workgroup_size_z()
            || self.max_compute_invocations_per_workgroup
                < execution.required_max_compute_invocations_per_workgroup()
            || self.max_compute_workgroups_per_dimension
                < execution.required_max_compute_workgroups_per_dimension()
            || self.storage_alignment_bytes > execution.storage_offset_alignment_bytes()
            || self.uniform_alignment_bytes > execution.uniform_offset_alignment_bytes()
            || self.copy_buffer_alignment_bytes != execution.copy_buffer_alignment_bytes()
            || self.copy_bytes_per_row_alignment != execution.copy_bytes_per_row_alignment()
            || self.profile_digest == [0; 4]
            || self.adapter_limits_digest == [0; 4]
        {
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        Ok(())
    }

    pub fn required_features_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        if self.required_feature_mask_words != 1 || self.required_feature_mask == 0 {
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        let mut digest = CanonicalDigestBuilder::new(REQUIRED_FEATURES_DIGEST_DOMAIN);
        digest.write_u8(self.required_feature_mask_words);
        digest.write_u64(self.required_feature_mask);
        Ok(digest.finish256())
    }

    pub fn required_limits_digest_for(
        &self,
        execution: &BrainExecutionBudget,
    ) -> Result<[u64; 4], ScaffoldContractError> {
        self.validate_for(execution)?;
        let mut digest = CanonicalDigestBuilder::new(REQUIRED_LIMITS_DIGEST_DOMAIN);
        digest.write_u16(execution.required_limits_schema_version());
        digest.write_u16(execution.gpu_layout_version());
        digest.write_u32(execution.storage_offset_alignment_bytes());
        digest.write_u32(execution.uniform_offset_alignment_bytes());
        digest.write_u32(execution.copy_buffer_alignment_bytes());
        digest.write_u32(execution.copy_bytes_per_row_alignment());
        digest.write_u64(execution.required_max_buffer_size());
        digest.write_u64(execution.required_max_storage_buffer_binding_size());
        digest.write_u32(execution.required_max_bind_groups());
        digest.write_u32(execution.required_max_bindings_per_bind_group());
        digest.write_u32(execution.required_max_storage_buffers_per_shader_stage());
        digest.write_u32(execution.required_max_uniform_buffers_per_shader_stage());
        digest.write_u32(execution.required_max_dynamic_storage_buffers_per_pipeline_layout());
        digest.write_u32(execution.required_max_dynamic_uniform_buffers_per_pipeline_layout());
        digest.write_u32(execution.required_max_compute_workgroup_storage_size());
        digest.write_u32(execution.required_max_compute_workgroup_size_x());
        digest.write_u32(execution.required_max_compute_workgroup_size_y());
        digest.write_u32(execution.required_max_compute_workgroup_size_z());
        digest.write_u32(execution.required_max_compute_invocations_per_workgroup());
        digest.write_u32(execution.required_max_compute_workgroups_per_dimension());
        Ok(digest.finish256())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuSlotComponentBytes {
    pub immutable_topology_bytes: u64,
    pub activation_bytes: u64,
    pub learning_bytes: u64,
    pub candidate_and_memory_bytes: u64,
    pub diagnostic_and_readback_bytes: u64,
    pub staging_bytes: u64,
}

impl GpuSlotComponentBytes {
    pub fn checked_sum(self) -> Option<u64> {
        [
            self.immutable_topology_bytes,
            self.activation_bytes,
            self.learning_bytes,
            self.candidate_and_memory_bytes,
            self.diagnostic_and_readback_bytes,
            self.staging_bytes,
        ]
        .into_iter()
        .try_fold(0_u64, u64::checked_add)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuSlotAllocationReceipt {
    pub schema_version: u16,
    pub class_id_raw: u16,
    pub immutable_topology_bytes: u64,
    pub activation_bytes: u64,
    pub learning_bytes: u64,
    pub candidate_and_memory_bytes: u64,
    pub diagnostic_and_readback_bytes: u64,
    pub staging_bytes: u64,
    pub alignment_padding_bytes: u64,
    pub shared_class_bytes: u64,
    pub logical_slot_commit_bytes: u64,
}

impl GpuSlotAllocationReceipt {
    pub const fn per_slot_component_bytes(self) -> GpuSlotComponentBytes {
        GpuSlotComponentBytes {
            immutable_topology_bytes: self.immutable_topology_bytes,
            activation_bytes: self.activation_bytes,
            learning_bytes: self.learning_bytes,
            candidate_and_memory_bytes: self.candidate_and_memory_bytes,
            diagnostic_and_readback_bytes: self.diagnostic_and_readback_bytes,
            staging_bytes: self.staging_bytes,
        }
    }

    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        BrainCapacityClass::production_for_id(alife_core::BrainClassId(self.class_id_raw))?;
        if self.schema_version != ADMISSION_SCHEMA_VERSION
            || [
                self.immutable_topology_bytes,
                self.activation_bytes,
                self.learning_bytes,
                self.candidate_and_memory_bytes,
                self.diagnostic_and_readback_bytes,
                self.staging_bytes,
                self.shared_class_bytes,
            ]
            .contains(&0)
            || self.per_slot_component_bytes().checked_sum() != Some(self.logical_slot_commit_bytes)
        {
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        Ok(())
    }

    pub(crate) fn new(
        class_id_raw: u16,
        components: GpuSlotComponentBytes,
        alignment_padding_bytes: u64,
        shared_class_bytes: u64,
    ) -> Result<Self, ScaffoldContractError> {
        let receipt = Self {
            schema_version: ADMISSION_SCHEMA_VERSION,
            class_id_raw,
            immutable_topology_bytes: components.immutable_topology_bytes,
            activation_bytes: components.activation_bytes,
            learning_bytes: components.learning_bytes,
            candidate_and_memory_bytes: components.candidate_and_memory_bytes,
            diagnostic_and_readback_bytes: components.diagnostic_and_readback_bytes,
            staging_bytes: components.staging_bytes,
            alignment_padding_bytes,
            shared_class_bytes,
            logical_slot_commit_bytes: components
                .checked_sum()
                .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?,
        };
        receipt.validate_contract()?;
        Ok(receipt)
    }
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuAllocationEventKind {
    AdmitFromNewChunk = 1,
    AdmitFromRetainedSlot = 2,
    ReleaseToRetainedSlot = 3,
    ReleaseAndDropEmptyChunk = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuAllocationEventReceipt {
    pub schema_version: u16,
    pub event_kind_raw: u16,
    pub class_id_raw: u16,
    pub handle_slot: u32,
    pub handle_generation: u32,
    pub logical_committed_before_bytes: u64,
    pub logical_committed_after_bytes: u64,
    pub physical_allocated_before_bytes: u64,
    pub physical_allocated_after_bytes: u64,
    pub physical_unused_before_bytes: u64,
    pub physical_unused_after_bytes: u64,
    pub physical_shared_before_bytes: u64,
    pub physical_shared_after_bytes: u64,
    pub event_digest: [u64; 4],
}

impl GpuAllocationEventReceipt {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        kind: GpuAllocationEventKind,
        class_id_raw: u16,
        handle_slot: u32,
        handle_generation: u32,
        before: &GpuAdmissionReceipt,
        after: &GpuAdmissionReceipt,
    ) -> Result<Self, ScaffoldContractError> {
        let mut receipt = Self {
            schema_version: ADMISSION_SCHEMA_VERSION,
            event_kind_raw: kind as u16,
            class_id_raw,
            handle_slot,
            handle_generation,
            logical_committed_before_bytes: before.logical_committed_bytes,
            logical_committed_after_bytes: after.logical_committed_bytes,
            physical_allocated_before_bytes: before.physical_allocated_bytes,
            physical_allocated_after_bytes: after.physical_allocated_bytes,
            physical_unused_before_bytes: before.physical_unused_retained_bytes,
            physical_unused_after_bytes: after.physical_unused_retained_bytes,
            physical_shared_before_bytes: before.physical_shared_bytes,
            physical_shared_after_bytes: after.physical_shared_bytes,
            event_digest: [0; 4],
        };
        receipt.event_digest = receipt.recompute_digest();
        Ok(receipt)
    }

    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        BrainCapacityClass::production_for_id(alife_core::BrainClassId(self.class_id_raw))?;
        let transition_is_valid = match self.event_kind_raw {
            value if value == GpuAllocationEventKind::AdmitFromNewChunk as u16 => {
                self.logical_committed_after_bytes > self.logical_committed_before_bytes
                    && self.physical_allocated_after_bytes > self.physical_allocated_before_bytes
            }
            value if value == GpuAllocationEventKind::AdmitFromRetainedSlot as u16 => {
                self.logical_committed_after_bytes > self.logical_committed_before_bytes
                    && self.physical_allocated_after_bytes == self.physical_allocated_before_bytes
                    && self.physical_unused_after_bytes < self.physical_unused_before_bytes
            }
            value if value == GpuAllocationEventKind::ReleaseToRetainedSlot as u16 => {
                self.logical_committed_after_bytes < self.logical_committed_before_bytes
                    && self.physical_allocated_after_bytes == self.physical_allocated_before_bytes
                    && self.physical_unused_after_bytes > self.physical_unused_before_bytes
            }
            value if value == GpuAllocationEventKind::ReleaseAndDropEmptyChunk as u16 => {
                self.logical_committed_after_bytes < self.logical_committed_before_bytes
                    && self.physical_allocated_after_bytes < self.physical_allocated_before_bytes
                    && self.physical_unused_after_bytes <= self.physical_unused_before_bytes
            }
            _ => false,
        };
        if self.schema_version != ADMISSION_SCHEMA_VERSION
            || !(1..=4).contains(&self.event_kind_raw)
            || self.handle_generation == 0
            || !transition_is_valid
            || self.event_digest != self.recompute_digest()
        {
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        Ok(())
    }

    fn reconciles_after(&self, receipt: &GpuAdmissionReceipt) -> bool {
        self.logical_committed_after_bytes == receipt.logical_committed_bytes
            && self.physical_allocated_after_bytes == receipt.physical_allocated_bytes
            && self.physical_unused_after_bytes == receipt.physical_unused_retained_bytes
            && self.physical_shared_after_bytes == receipt.physical_shared_bytes
    }

    fn recompute_digest(&self) -> [u64; 4] {
        let mut digest = CanonicalDigestBuilder::new(ALLOCATION_EVENT_DIGEST_DOMAIN);
        digest.write_u16(self.schema_version);
        digest.write_u16(self.event_kind_raw);
        digest.write_u16(self.class_id_raw);
        digest.write_u32(self.handle_slot);
        digest.write_u32(self.handle_generation);
        for value in [
            self.logical_committed_before_bytes,
            self.logical_committed_after_bytes,
            self.physical_allocated_before_bytes,
            self.physical_allocated_after_bytes,
            self.physical_unused_before_bytes,
            self.physical_unused_after_bytes,
            self.physical_shared_before_bytes,
            self.physical_shared_after_bytes,
        ] {
            digest.write_u64(value);
        }
        digest.finish256()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuAdmissionReceipt {
    pub schema_version: u16,
    pub runtime: GpuRuntimeBudget,
    pub logical_committed_bytes: u64,
    pub logical_available_bytes: u64,
    pub physical_allocated_bytes: u64,
    pub physical_unused_retained_bytes: u64,
    pub physical_shared_bytes: u64,
    pub physical_alignment_slack_bytes: u64,
    pub peak_logical_committed_bytes: u64,
    pub peak_physical_allocated_bytes: u64,
    pub live_brains: u32,
    pub max_hot_brains: u32,
    pub allocation_generation: u64,
    pub last_event: Option<GpuAllocationEventReceipt>,
}

impl GpuAdmissionReceipt {
    pub(crate) fn empty(runtime: GpuRuntimeBudget) -> Self {
        Self {
            schema_version: ADMISSION_SCHEMA_VERSION,
            runtime,
            logical_committed_bytes: 0,
            logical_available_bytes: runtime.logical_neural_heap_budget_bytes,
            physical_allocated_bytes: 0,
            physical_unused_retained_bytes: 0,
            physical_shared_bytes: 0,
            physical_alignment_slack_bytes: 0,
            peak_logical_committed_bytes: 0,
            peak_physical_allocated_bytes: 0,
            live_brains: 0,
            max_hot_brains: runtime.max_hot_brains,
            allocation_generation: 0,
            last_event: None,
        }
    }

    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        let physical_sum = self
            .physical_shared_bytes
            .checked_add(self.logical_committed_bytes)
            .and_then(|value| value.checked_add(self.physical_unused_retained_bytes))
            .and_then(|value| value.checked_add(self.physical_alignment_slack_bytes));
        if self.schema_version != ADMISSION_SCHEMA_VERSION
            || self.max_hot_brains != self.runtime.max_hot_brains
            || self.live_brains > self.max_hot_brains
            || self.logical_committed_bytes > self.runtime.logical_neural_heap_budget_bytes
            || self.logical_available_bytes
                != self
                    .runtime
                    .logical_neural_heap_budget_bytes
                    .checked_sub(self.logical_committed_bytes)
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?
            || physical_sum != Some(self.physical_allocated_bytes)
            || self.physical_allocated_bytes > self.runtime.physical_allocation_ceiling_bytes
            || self.peak_logical_committed_bytes < self.logical_committed_bytes
            || self.peak_physical_allocated_bytes < self.physical_allocated_bytes
            || (self.allocation_generation == 0) != self.last_event.is_none()
            || self.last_event.as_ref().is_some_and(|event| {
                event.validate_contract().is_err() || !event.reconciles_after(self)
            })
        {
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        Ok(())
    }
}
