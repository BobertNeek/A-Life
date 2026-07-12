use bytemuck::{Pod, Zeroable};

use super::GpuClosedLoopError;

pub const GPU_PERCEPTION_HEADER_BYTES: usize = 64;
pub const GPU_BRAIN_SLOT_RECORD_BYTES: usize = 144;
pub const GPU_CANDIDATE_RECORD_BYTES: usize = 32;
pub const GPU_SELECTION_RECORD_BYTES: usize = 48;
/// Exact executable ordering/layout ABI understood by the current closed-loop shaders.
pub const GPU_CLOSED_LOOP_LAYOUT_VERSION: u32 = 2;
pub const GPU_NO_EXTENSION_SENTINEL: u32 = u32::MAX;
pub const CLOSED_LOOP_ABI_WGSL: &str = include_str!("../../shaders/closed_loop_abi.wgsl");

macro_rules! gpu_record {
    ($name:ident { $($field:ident : $ty:ty),+ $(,)? }) => {
        #[repr(C, align(16))]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
        pub struct $name { $(pub $field: $ty),+ }

        impl $name {
            pub fn from_words(words: &[u32]) -> Result<Self, GpuClosedLoopError> {
                let expected = std::mem::size_of::<Self>() / 4;
                if words.len() != expected { return Err(GpuClosedLoopError::MalformedUpload); }
                Ok(bytemuck::pod_read_unaligned(bytemuck::cast_slice(words)))
            }

            pub fn words(&self) -> &[u32] { bytemuck::cast_slice(std::slice::from_ref(self)) }
        }
    };
}

gpu_record!(GpuPerceptionHeader {
    schema_version: u32,
    class_id: u32,
    slot: u32,
    slot_generation: u32,
    neuron_count: u32,
    candidate_count: u32,
    microstep_count: u32,
    active_activation_side: u32,
    tick_lo: u32,
    tick_hi: u32,
    sensory_offset: u32,
    candidate_offset: u32,
    brain_slot_index: u32,
    dispatch_generation_lo: u32,
    dispatch_generation_hi: u32,
    reserved: u32
});
gpu_record!(GpuBrainSlotRecord {
    schema_version: u32,
    class_id: u32,
    slot: u32,
    slot_generation: u32,
    neuron_count: u32,
    microstep_count: u32,
    synapse_count: u32,
    recurrent_synapse_count: u32,
    encoder_plan_offset: u32,
    neuron_dynamics_offset: u32,
    projection_offset: u32,
    route_metadata_offset: u32,
    target_offsets_offset: u32,
    source_indices_offset: u32,
    route_indices_offset: u32,
    decoder_plan_offset: u32,
    decoder_family_offset: u32,
    decoder_weight_indices_offset: u32,
    genetic_weight_offset: u32,
    alpha_offset: u32,
    activation_a_offset: u32,
    activation_b_offset: u32,
    accumulator_offset: u32,
    lifetime_weight_offset: u32,
    fast_weight_offset: u32,
    recurrent_eligibility_offset: u32,
    decoder_eligibility_offset: u32,
    encoded_input_offset: u32,
    candidate_logit_offset: u32,
    diagnostic_offset: u32,
    selection_offset: u32,
    neuron_homeostasis_offset: u32,
    extension_record_offset: u32,
    reserved: [u32; 3]
});
gpu_record!(GpuPhenotypeIdentityRecord {
    phenotype_hash: [u32; 8]
});
gpu_record!(GpuCandidateRecord {
    action_id: u32,
    kind: u32,
    family: u32,
    candidate_index: u32,
    feature_offset: u32,
    observation_slot_or_max: u32,
    confidence_q16: u32,
    effort_q16: u32
});
gpu_record!(GpuSelectionRecord {
    slot: u32,
    slot_generation: u32,
    candidate_index: u32,
    logit_bits: u32,
    confidence_q16: u32,
    status: u32,
    active_tiles: u32,
    active_synapses: u32,
    finite_rejections: u32,
    dispatch_generation_lo: u32,
    dispatch_generation_hi: u32,
    active_activation_side: u32
});
gpu_record!(GpuEncoderPlanRecord {
    schema_version: u32,
    sensor_profile_raw: u32,
    assignment_offset: u32,
    assignment_count: u32,
    target_offsets_offset: u32,
    sensory_lane_count: u32,
    body_lane_count: u32,
    homeostasis_lane_count: u32
});
gpu_record!(GpuEncoderAssignmentRecord {
    source_group_raw: u32,
    source_index: u32,
    target_neuron: u32,
    reserved0: u32,
    scale_bits: u32,
    bias_bits: u32,
    clamp_min_bits: u32,
    clamp_max_bits: u32
});
gpu_record!(GpuNeuronDynamicsRecord {
    bias_bits: u32,
    leak_bits: u32,
    activation_raw: u32,
    homeostatic_gain_bits: u32,
    activity_ema_decay_bits: u32,
    metabolic_decay_bits: u32,
    reserved0: u32,
    reserved1: u32
});
gpu_record!(GpuProjectionRecord {
    route_index: u32,
    source_lobe_raw: u32,
    target_lobe_raw: u32,
    synapse_start: u32,
    synapse_count: u32,
    active_tile_count: u32,
    reserved0: u32,
    reserved1: u32
});
gpu_record!(GpuRouteMetadataRecord {
    route_index: u32,
    projection_type_raw: u32,
    active_tile_policy_raw: u32,
    update_cadence_raw: u32,
    biological_priority_raw: u32,
    delay_microsteps: u32,
    source_start: u32,
    source_count: u32,
    target_start: u32,
    target_count: u32,
    reserved0: u32,
    reserved1: u32
});
gpu_record!(GpuDecoderPlanRecord {
    schema_version: u32,
    motor_start: u32,
    motor_width: u32,
    feature_count: u32,
    flattened_input_lane_count: u32,
    family_offset: u32,
    family_count: u32,
    decoder_synapse_count: u32
});
gpu_record!(GpuDecoderFamilyRecord {
    family_raw: u32,
    bias_bits: u32,
    decoder_synapse_start: u32,
    decoder_synapse_count: u32,
    weight_index_start: u32,
    weight_index_count: u32,
    reserved0: u32,
    reserved1: u32
});
gpu_record!(GpuDecoderWeightIndexRecord {
    global_synapse_id: u32,
    input_lane: u32,
    motor_index: u32,
    reserved0: u32
});

impl GpuBrainSlotRecord {
    pub fn validate_slice_a(&self) -> Result<(), GpuClosedLoopError> {
        if self.schema_version != GPU_CLOSED_LOOP_LAYOUT_VERSION
            || self.reserved != [0; 3]
            || self.extension_record_offset != GPU_NO_EXTENSION_SENTINEL
            || self.slot_generation == 0
            || self.microstep_count == 0
        {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        Ok(())
    }
}
