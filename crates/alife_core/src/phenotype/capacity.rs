//! Contract-only canonical production capacity and GPU ABI budget authority.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{BrainClassId, CanonicalDigestBuilder, ScaffoldContractError, CANDIDATE_FEATURE_COUNT};

const CAPACITY_SCHEMA_VERSION: u16 = 1;
const GPU_LAYOUT_VERSION: u16 = 2;
const REQUIRED_LIMITS_SCHEMA_VERSION: u16 = 1;
const CAPACITY_DIGEST_DOMAIN: &[u8] = b"alife.brain.capacity.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct BrainExecutionBudget {
    schema_version: u16,
    gpu_layout_version: u16,
    max_neurons: u32,
    max_total_synapses: u32,
    max_recurrent_synapses: u32,
    max_action_decoder_synapses: u32,
    max_memory_decoder_synapses: u32,
    max_active_tiles: u32,
    max_candidates: u16,
    max_object_slots: u16,
    max_memory_context_records: u16,
    min_microsteps: u8,
    max_microsteps: u8,
    max_replay_events: u32,
    max_replay_eligibility_samples: u32,
    max_compact_readback_bytes: u32,
    microtile_edge: u16,
    supertile_edge: u16,
    candidate_feature_count: u16,
    max_decoder_input_lanes: u16,
    required_limits_schema_version: u16,
    required_feature_mask_words: u8,
    required_feature_mask: u64,
    required_max_buffer_size: u64,
    required_max_storage_buffer_binding_size: u64,
    required_max_bind_groups: u32,
    required_max_bindings_per_bind_group: u32,
    required_max_storage_buffers_per_shader_stage: u32,
    required_max_uniform_buffers_per_shader_stage: u32,
    required_max_dynamic_storage_buffers_per_pipeline_layout: u32,
    required_max_dynamic_uniform_buffers_per_pipeline_layout: u32,
    required_max_compute_workgroup_storage_size: u32,
    required_max_compute_workgroup_size_x: u32,
    required_max_compute_workgroup_size_y: u32,
    required_max_compute_workgroup_size_z: u32,
    required_max_compute_invocations_per_workgroup: u32,
    required_max_compute_workgroups_per_dimension: u32,
    storage_offset_alignment_bytes: u32,
    uniform_offset_alignment_bytes: u32,
    copy_buffer_alignment_bytes: u32,
    copy_bytes_per_row_alignment: u32,
}

impl BrainExecutionBudget {
    #[allow(clippy::too_many_arguments)]
    const fn production(
        max_neurons: u32,
        max_total_synapses: u32,
        max_recurrent_synapses: u32,
        max_action_decoder_synapses: u32,
        max_memory_decoder_synapses: u32,
        max_active_tiles: u32,
        max_replay_events: u32,
        max_replay_eligibility_samples: u32,
    ) -> Self {
        Self {
            schema_version: CAPACITY_SCHEMA_VERSION,
            gpu_layout_version: GPU_LAYOUT_VERSION,
            max_neurons,
            max_total_synapses,
            max_recurrent_synapses,
            max_action_decoder_synapses,
            max_memory_decoder_synapses,
            max_active_tiles,
            max_candidates: 32,
            max_object_slots: 16,
            max_memory_context_records: 16,
            min_microsteps: 2,
            max_microsteps: 4,
            max_replay_events,
            max_replay_eligibility_samples,
            max_compact_readback_bytes: 64,
            microtile_edge: 16,
            supertile_edge: 128,
            candidate_feature_count: CANDIDATE_FEATURE_COUNT as u16,
            max_decoder_input_lanes: 64,
            required_limits_schema_version: REQUIRED_LIMITS_SCHEMA_VERSION,
            required_feature_mask_words: 1,
            required_feature_mask: 0,
            required_max_buffer_size: 268_435_456,
            required_max_storage_buffer_binding_size: 134_217_728,
            required_max_bind_groups: 4,
            required_max_bindings_per_bind_group: 1_000,
            required_max_storage_buffers_per_shader_stage: 8,
            required_max_uniform_buffers_per_shader_stage: 12,
            required_max_dynamic_storage_buffers_per_pipeline_layout: 4,
            required_max_dynamic_uniform_buffers_per_pipeline_layout: 8,
            required_max_compute_workgroup_storage_size: 16_384,
            required_max_compute_workgroup_size_x: 256,
            required_max_compute_workgroup_size_y: 256,
            required_max_compute_workgroup_size_z: 64,
            required_max_compute_invocations_per_workgroup: 256,
            required_max_compute_workgroups_per_dimension: 65_535,
            storage_offset_alignment_bytes: 256,
            uniform_offset_alignment_bytes: 256,
            copy_buffer_alignment_bytes: 4,
            copy_bytes_per_row_alignment: 256,
        }
    }

    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    pub const fn gpu_layout_version(&self) -> u16 {
        self.gpu_layout_version
    }

    pub const fn max_neurons(&self) -> u32 {
        self.max_neurons
    }

    pub const fn max_total_synapses(&self) -> u32 {
        self.max_total_synapses
    }

    pub const fn max_recurrent_synapses(&self) -> u32 {
        self.max_recurrent_synapses
    }

    pub const fn max_action_decoder_synapses(&self) -> u32 {
        self.max_action_decoder_synapses
    }

    pub const fn max_memory_decoder_synapses(&self) -> u32 {
        self.max_memory_decoder_synapses
    }

    pub const fn max_active_tiles(&self) -> u32 {
        self.max_active_tiles
    }

    pub const fn max_candidates(&self) -> u16 {
        self.max_candidates
    }

    pub const fn max_object_slots(&self) -> u16 {
        self.max_object_slots
    }

    pub const fn max_memory_context_records(&self) -> u16 {
        self.max_memory_context_records
    }

    pub const fn microstep_range(&self) -> (u8, u8) {
        (self.min_microsteps, self.max_microsteps)
    }

    pub const fn max_replay_events(&self) -> u32 {
        self.max_replay_events
    }

    pub const fn max_replay_eligibility_samples(&self) -> u32 {
        self.max_replay_eligibility_samples
    }

    pub const fn max_compact_readback_bytes(&self) -> u32 {
        self.max_compact_readback_bytes
    }

    pub const fn microtile_edge(&self) -> u16 {
        self.microtile_edge
    }

    pub const fn supertile_edge(&self) -> u16 {
        self.supertile_edge
    }

    pub const fn candidate_feature_count(&self) -> u16 {
        self.candidate_feature_count
    }

    pub const fn max_decoder_input_lanes(&self) -> u16 {
        self.max_decoder_input_lanes
    }

    pub const fn required_limits_schema_version(&self) -> u16 {
        self.required_limits_schema_version
    }

    pub const fn required_feature_mask_words(&self) -> u8 {
        self.required_feature_mask_words
    }

    pub const fn required_feature_mask(&self) -> u64 {
        self.required_feature_mask
    }

    pub const fn required_max_buffer_size(&self) -> u64 {
        self.required_max_buffer_size
    }

    pub const fn required_max_storage_buffer_binding_size(&self) -> u64 {
        self.required_max_storage_buffer_binding_size
    }

    pub const fn required_max_bind_groups(&self) -> u32 {
        self.required_max_bind_groups
    }

    pub const fn required_max_bindings_per_bind_group(&self) -> u32 {
        self.required_max_bindings_per_bind_group
    }

    pub const fn required_max_storage_buffers_per_shader_stage(&self) -> u32 {
        self.required_max_storage_buffers_per_shader_stage
    }

    pub const fn required_max_uniform_buffers_per_shader_stage(&self) -> u32 {
        self.required_max_uniform_buffers_per_shader_stage
    }

    pub const fn required_max_dynamic_storage_buffers_per_pipeline_layout(&self) -> u32 {
        self.required_max_dynamic_storage_buffers_per_pipeline_layout
    }

    pub const fn required_max_dynamic_uniform_buffers_per_pipeline_layout(&self) -> u32 {
        self.required_max_dynamic_uniform_buffers_per_pipeline_layout
    }

    pub const fn required_max_compute_workgroup_storage_size(&self) -> u32 {
        self.required_max_compute_workgroup_storage_size
    }

    pub const fn required_max_compute_workgroup_size_x(&self) -> u32 {
        self.required_max_compute_workgroup_size_x
    }

    pub const fn required_max_compute_workgroup_size_y(&self) -> u32 {
        self.required_max_compute_workgroup_size_y
    }

    pub const fn required_max_compute_workgroup_size_z(&self) -> u32 {
        self.required_max_compute_workgroup_size_z
    }

    pub const fn required_max_compute_invocations_per_workgroup(&self) -> u32 {
        self.required_max_compute_invocations_per_workgroup
    }

    pub const fn required_max_compute_workgroups_per_dimension(&self) -> u32 {
        self.required_max_compute_workgroups_per_dimension
    }

    pub const fn storage_offset_alignment_bytes(&self) -> u32 {
        self.storage_offset_alignment_bytes
    }

    pub const fn uniform_offset_alignment_bytes(&self) -> u32 {
        self.uniform_offset_alignment_bytes
    }

    pub const fn copy_buffer_alignment_bytes(&self) -> u32 {
        self.copy_buffer_alignment_bytes
    }

    pub const fn copy_bytes_per_row_alignment(&self) -> u32 {
        self.copy_bytes_per_row_alignment
    }

    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        let canonical = BrainCapacityClass::production_classes();
        if canonical.iter().any(|capacity| capacity.execution == *self) {
            Ok(())
        } else {
            Err(ScaffoldContractError::PhenotypeCompile)
        }
    }

    fn write_canonical(&self, digest: &mut CanonicalDigestBuilder) {
        digest.write_u16(self.schema_version);
        digest.write_u16(self.gpu_layout_version);
        digest.write_u32(self.max_neurons);
        digest.write_u32(self.max_total_synapses);
        digest.write_u32(self.max_recurrent_synapses);
        digest.write_u32(self.max_action_decoder_synapses);
        digest.write_u32(self.max_memory_decoder_synapses);
        digest.write_u32(self.max_active_tiles);
        digest.write_u16(self.max_candidates);
        digest.write_u16(self.max_object_slots);
        digest.write_u16(self.max_memory_context_records);
        digest.write_u8(self.min_microsteps);
        digest.write_u8(self.max_microsteps);
        digest.write_u32(self.max_replay_events);
        digest.write_u32(self.max_replay_eligibility_samples);
        digest.write_u32(self.max_compact_readback_bytes);
        digest.write_u16(self.microtile_edge);
        digest.write_u16(self.supertile_edge);
        digest.write_u16(self.candidate_feature_count);
        digest.write_u16(self.max_decoder_input_lanes);
        digest.write_u16(self.required_limits_schema_version);
        digest.write_u8(self.required_feature_mask_words);
        digest.write_u64(self.required_feature_mask);
        digest.write_u64(self.required_max_buffer_size);
        digest.write_u64(self.required_max_storage_buffer_binding_size);
        digest.write_u32(self.required_max_bind_groups);
        digest.write_u32(self.required_max_bindings_per_bind_group);
        digest.write_u32(self.required_max_storage_buffers_per_shader_stage);
        digest.write_u32(self.required_max_uniform_buffers_per_shader_stage);
        digest.write_u32(self.required_max_dynamic_storage_buffers_per_pipeline_layout);
        digest.write_u32(self.required_max_dynamic_uniform_buffers_per_pipeline_layout);
        digest.write_u32(self.required_max_compute_workgroup_storage_size);
        digest.write_u32(self.required_max_compute_workgroup_size_x);
        digest.write_u32(self.required_max_compute_workgroup_size_y);
        digest.write_u32(self.required_max_compute_workgroup_size_z);
        digest.write_u32(self.required_max_compute_invocations_per_workgroup);
        digest.write_u32(self.required_max_compute_workgroups_per_dimension);
        digest.write_u32(self.storage_offset_alignment_bytes);
        digest.write_u32(self.uniform_offset_alignment_bytes);
        digest.write_u32(self.copy_buffer_alignment_bytes);
        digest.write_u32(self.copy_bytes_per_row_alignment);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct BrainCapacityClass {
    id: BrainClassId,
    execution: BrainExecutionBudget,
}

impl BrainCapacityClass {
    pub const N512_ID: BrainClassId = BrainClassId(1);
    pub const N1024_ID: BrainClassId = BrainClassId(2);
    pub const N2048_ID: BrainClassId = BrainClassId(3);

    pub const fn n512() -> Self {
        Self {
            id: Self::N512_ID,
            execution: BrainExecutionBudget::production(
                512, 8_192, 6_144, 1_024, 1_024, 64, 32, 2_048,
            ),
        }
    }

    pub const fn n1024() -> Self {
        Self {
            id: Self::N1024_ID,
            execution: BrainExecutionBudget::production(
                1_024, 16_384, 12_288, 2_048, 2_048, 128, 64, 4_096,
            ),
        }
    }

    pub const fn n2048() -> Self {
        Self {
            id: Self::N2048_ID,
            execution: BrainExecutionBudget::production(
                2_048, 32_768, 24_576, 4_096, 4_096, 192, 128, 8_192,
            ),
        }
    }

    pub fn production_for_id(id: BrainClassId) -> Result<Self, ScaffoldContractError> {
        match id {
            Self::N512_ID => Ok(Self::n512()),
            Self::N1024_ID => Ok(Self::n1024()),
            Self::N2048_ID => Ok(Self::n2048()),
            _ => Err(ScaffoldContractError::UnsupportedProductionBrainClass),
        }
    }

    pub const fn production_classes() -> [Self; 3] {
        [Self::n512(), Self::n1024(), Self::n2048()]
    }

    pub const fn id(&self) -> BrainClassId {
        self.id
    }

    pub const fn execution(&self) -> &BrainExecutionBudget {
        &self.execution
    }

    pub fn canonical_digest(&self) -> [u64; 4] {
        let mut digest = CanonicalDigestBuilder::new(CAPACITY_DIGEST_DOMAIN);
        digest.write_u16(self.id.raw());
        self.execution.write_canonical(&mut digest);
        digest.finish256()
    }

    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        let canonical = Self::production_for_id(self.id)?;
        if *self != canonical {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        self.execution.validate_contract()
    }
}

#[derive(Debug, Deserialize)]
struct BrainCapacityClassWire {
    id: BrainClassId,
    execution: BrainExecutionBudgetWire,
}

#[derive(Debug, Deserialize)]
struct BrainExecutionBudgetWire {
    schema_version: u16,
    gpu_layout_version: u16,
    max_neurons: u32,
    max_total_synapses: u32,
    max_recurrent_synapses: u32,
    max_action_decoder_synapses: u32,
    max_memory_decoder_synapses: u32,
    max_active_tiles: u32,
    max_candidates: u16,
    max_object_slots: u16,
    max_memory_context_records: u16,
    min_microsteps: u8,
    max_microsteps: u8,
    max_replay_events: u32,
    max_replay_eligibility_samples: u32,
    max_compact_readback_bytes: u32,
    microtile_edge: u16,
    supertile_edge: u16,
    candidate_feature_count: u16,
    max_decoder_input_lanes: u16,
    required_limits_schema_version: u16,
    required_feature_mask_words: u8,
    required_feature_mask: u64,
    required_max_buffer_size: u64,
    required_max_storage_buffer_binding_size: u64,
    required_max_bind_groups: u32,
    required_max_bindings_per_bind_group: u32,
    required_max_storage_buffers_per_shader_stage: u32,
    required_max_uniform_buffers_per_shader_stage: u32,
    required_max_dynamic_storage_buffers_per_pipeline_layout: u32,
    required_max_dynamic_uniform_buffers_per_pipeline_layout: u32,
    required_max_compute_workgroup_storage_size: u32,
    required_max_compute_workgroup_size_x: u32,
    required_max_compute_workgroup_size_y: u32,
    required_max_compute_workgroup_size_z: u32,
    required_max_compute_invocations_per_workgroup: u32,
    required_max_compute_workgroups_per_dimension: u32,
    storage_offset_alignment_bytes: u32,
    uniform_offset_alignment_bytes: u32,
    copy_buffer_alignment_bytes: u32,
    copy_bytes_per_row_alignment: u32,
}

impl From<BrainExecutionBudgetWire> for BrainExecutionBudget {
    fn from(wire: BrainExecutionBudgetWire) -> Self {
        Self {
            schema_version: wire.schema_version,
            gpu_layout_version: wire.gpu_layout_version,
            max_neurons: wire.max_neurons,
            max_total_synapses: wire.max_total_synapses,
            max_recurrent_synapses: wire.max_recurrent_synapses,
            max_action_decoder_synapses: wire.max_action_decoder_synapses,
            max_memory_decoder_synapses: wire.max_memory_decoder_synapses,
            max_active_tiles: wire.max_active_tiles,
            max_candidates: wire.max_candidates,
            max_object_slots: wire.max_object_slots,
            max_memory_context_records: wire.max_memory_context_records,
            min_microsteps: wire.min_microsteps,
            max_microsteps: wire.max_microsteps,
            max_replay_events: wire.max_replay_events,
            max_replay_eligibility_samples: wire.max_replay_eligibility_samples,
            max_compact_readback_bytes: wire.max_compact_readback_bytes,
            microtile_edge: wire.microtile_edge,
            supertile_edge: wire.supertile_edge,
            candidate_feature_count: wire.candidate_feature_count,
            max_decoder_input_lanes: wire.max_decoder_input_lanes,
            required_limits_schema_version: wire.required_limits_schema_version,
            required_feature_mask_words: wire.required_feature_mask_words,
            required_feature_mask: wire.required_feature_mask,
            required_max_buffer_size: wire.required_max_buffer_size,
            required_max_storage_buffer_binding_size: wire.required_max_storage_buffer_binding_size,
            required_max_bind_groups: wire.required_max_bind_groups,
            required_max_bindings_per_bind_group: wire.required_max_bindings_per_bind_group,
            required_max_storage_buffers_per_shader_stage: wire
                .required_max_storage_buffers_per_shader_stage,
            required_max_uniform_buffers_per_shader_stage: wire
                .required_max_uniform_buffers_per_shader_stage,
            required_max_dynamic_storage_buffers_per_pipeline_layout: wire
                .required_max_dynamic_storage_buffers_per_pipeline_layout,
            required_max_dynamic_uniform_buffers_per_pipeline_layout: wire
                .required_max_dynamic_uniform_buffers_per_pipeline_layout,
            required_max_compute_workgroup_storage_size: wire
                .required_max_compute_workgroup_storage_size,
            required_max_compute_workgroup_size_x: wire.required_max_compute_workgroup_size_x,
            required_max_compute_workgroup_size_y: wire.required_max_compute_workgroup_size_y,
            required_max_compute_workgroup_size_z: wire.required_max_compute_workgroup_size_z,
            required_max_compute_invocations_per_workgroup: wire
                .required_max_compute_invocations_per_workgroup,
            required_max_compute_workgroups_per_dimension: wire
                .required_max_compute_workgroups_per_dimension,
            storage_offset_alignment_bytes: wire.storage_offset_alignment_bytes,
            uniform_offset_alignment_bytes: wire.uniform_offset_alignment_bytes,
            copy_buffer_alignment_bytes: wire.copy_buffer_alignment_bytes,
            copy_bytes_per_row_alignment: wire.copy_bytes_per_row_alignment,
        }
    }
}

impl<'de> Deserialize<'de> for BrainCapacityClass {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = BrainCapacityClassWire::deserialize(deserializer)?;
        let canonical = Self::production_for_id(wire.id).map_err(D::Error::custom)?;
        let execution = BrainExecutionBudget::from(wire.execution);
        if execution != *canonical.execution() {
            return Err(D::Error::custom("noncanonical brain execution budget"));
        }
        Ok(canonical)
    }
}
