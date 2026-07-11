const GPU_CLOSED_LOOP_LAYOUT_VERSION:u32 = 1u;

struct GpuPerceptionHeader {
  schema_version:u32, class_id:u32, slot:u32, slot_generation:u32,
  neuron_count:u32, candidate_count:u32, microstep_count:u32, active_activation_side:u32,
  tick_lo:u32, tick_hi:u32, sensory_offset:u32, candidate_offset:u32,
  brain_slot_index:u32, reserved:array<u32,3>,
}
struct GpuBrainSlotRecord {
  schema_version:u32, class_id:u32, slot:u32, slot_generation:u32,
  neuron_count:u32, microstep_count:u32, synapse_count:u32, recurrent_synapse_count:u32,
  encoder_plan_offset:u32, neuron_dynamics_offset:u32, projection_offset:u32, route_metadata_offset:u32,
  target_offsets_offset:u32, source_indices_offset:u32, route_indices_offset:u32, decoder_plan_offset:u32,
  decoder_family_offset:u32, decoder_weight_indices_offset:u32, genetic_weight_offset:u32, alpha_offset:u32,
  activation_a_offset:u32, activation_b_offset:u32, accumulator_offset:u32, lifetime_weight_offset:u32,
  fast_weight_offset:u32, recurrent_eligibility_offset:u32, decoder_eligibility_offset:u32, encoded_input_offset:u32,
  candidate_logit_offset:u32, diagnostic_offset:u32, selection_offset:u32, neuron_homeostasis_offset:u32,
  extension_record_offset:u32, reserved:array<u32,3>,
}
struct GpuPhenotypeIdentityRecord { phenotype_hash:array<u32,8>, }
struct GpuCandidateRecord {
  action_id:u32, kind:u32, family:u32, candidate_index:u32,
  feature_offset:u32, observation_slot_or_max:u32, confidence_q16:u32, effort_q16:u32,
}
struct GpuSelectionRecord {
  slot:u32, slot_generation:u32, candidate_index:u32, logit_bits:u32,
  confidence_q16:u32, status:u32, active_tiles:u32, active_synapses:u32,
  finite_rejections:u32, dispatch_generation_lo:u32, dispatch_generation_hi:u32, active_activation_side:u32,
}
struct GpuEncoderPlanRecord {
  schema_version:u32, sensor_profile_raw:u32, assignment_offset:u32, assignment_count:u32,
  target_offsets_offset:u32, sensory_lane_count:u32, body_lane_count:u32, homeostasis_lane_count:u32,
}
struct GpuEncoderAssignmentRecord {
  source_group_raw:u32, source_index:u32, target_neuron:u32, reserved0:u32,
  scale_bits:u32, bias_bits:u32, clamp_min_bits:u32, clamp_max_bits:u32,
}
struct GpuNeuronDynamicsRecord {
  bias_bits:u32, leak_bits:u32, activation_raw:u32, homeostatic_gain_bits:u32,
  activity_ema_decay_bits:u32, metabolic_decay_bits:u32, reserved0:u32, reserved1:u32,
}
struct GpuProjectionRecord {
  route_index:u32, source_lobe_raw:u32, target_lobe_raw:u32, synapse_start:u32,
  synapse_count:u32, active_tile_count:u32, reserved0:u32, reserved1:u32,
}
struct GpuRouteMetadataRecord {
  route_index:u32, projection_type_raw:u32, active_tile_policy_raw:u32, update_cadence_raw:u32,
  biological_priority_raw:u32, delay_microsteps:u32, source_start:u32, source_count:u32,
  target_start:u32, target_count:u32, reserved0:u32, reserved1:u32,
}
struct GpuDecoderPlanRecord {
  schema_version:u32, motor_start:u32, motor_width:u32, feature_count:u32,
  flattened_input_lane_count:u32, family_offset:u32, family_count:u32, decoder_synapse_count:u32,
}
struct GpuDecoderFamilyRecord {
  family_raw:u32, bias_bits:u32, decoder_synapse_start:u32, decoder_synapse_count:u32,
  weight_index_start:u32, weight_index_count:u32, reserved0:u32, reserved1:u32,
}
struct GpuDecoderWeightIndexRecord {
  global_synapse_id:u32, input_lane:u32, motor_index:u32, reserved0:u32,
}

@group(0) @binding(0) var<storage, read> brain_slots: array<GpuBrainSlotRecord>;
@group(0) @binding(1) var<storage, read> phenotype_identities: array<GpuPhenotypeIdentityRecord>;
@group(0) @binding(2) var<storage, read> immutable_plan_words: array<u32>;
@group(0) @binding(3) var<storage, read> immutable_weight_words: array<u32>;
@group(0) @binding(4) var<storage, read> dispatch_header_words: array<u32>;
@group(0) @binding(5) var<storage, read> frame_payload_words: array<u32>;
@group(0) @binding(6) var<storage, read_write> mutable_state_words: array<u32>;

fn load_encoder_plan(base:u32) -> GpuEncoderPlanRecord {
  return GpuEncoderPlanRecord(immutable_plan_words[base],immutable_plan_words[base+1u],immutable_plan_words[base+2u],immutable_plan_words[base+3u],immutable_plan_words[base+4u],immutable_plan_words[base+5u],immutable_plan_words[base+6u],immutable_plan_words[base+7u]);
}
fn load_encoder_assignment(base:u32) -> GpuEncoderAssignmentRecord {
  return GpuEncoderAssignmentRecord(immutable_plan_words[base],immutable_plan_words[base+1u],immutable_plan_words[base+2u],immutable_plan_words[base+3u],immutable_plan_words[base+4u],immutable_plan_words[base+5u],immutable_plan_words[base+6u],immutable_plan_words[base+7u]);
}
fn load_neuron_dynamics(base:u32) -> GpuNeuronDynamicsRecord {
  return GpuNeuronDynamicsRecord(immutable_plan_words[base],immutable_plan_words[base+1u],immutable_plan_words[base+2u],immutable_plan_words[base+3u],immutable_plan_words[base+4u],immutable_plan_words[base+5u],immutable_plan_words[base+6u],immutable_plan_words[base+7u]);
}
fn load_projection(base:u32) -> GpuProjectionRecord {
  return GpuProjectionRecord(immutable_plan_words[base],immutable_plan_words[base+1u],immutable_plan_words[base+2u],immutable_plan_words[base+3u],immutable_plan_words[base+4u],immutable_plan_words[base+5u],immutable_plan_words[base+6u],immutable_plan_words[base+7u]);
}
fn load_route_metadata(base:u32) -> GpuRouteMetadataRecord {
  return GpuRouteMetadataRecord(immutable_plan_words[base],immutable_plan_words[base+1u],immutable_plan_words[base+2u],immutable_plan_words[base+3u],immutable_plan_words[base+4u],immutable_plan_words[base+5u],immutable_plan_words[base+6u],immutable_plan_words[base+7u],immutable_plan_words[base+8u],immutable_plan_words[base+9u],immutable_plan_words[base+10u],immutable_plan_words[base+11u]);
}
fn load_decoder_plan(base:u32) -> GpuDecoderPlanRecord {
  return GpuDecoderPlanRecord(immutable_plan_words[base],immutable_plan_words[base+1u],immutable_plan_words[base+2u],immutable_plan_words[base+3u],immutable_plan_words[base+4u],immutable_plan_words[base+5u],immutable_plan_words[base+6u],immutable_plan_words[base+7u]);
}
fn load_decoder_family(base:u32) -> GpuDecoderFamilyRecord {
  return GpuDecoderFamilyRecord(immutable_plan_words[base],immutable_plan_words[base+1u],immutable_plan_words[base+2u],immutable_plan_words[base+3u],immutable_plan_words[base+4u],immutable_plan_words[base+5u],immutable_plan_words[base+6u],immutable_plan_words[base+7u]);
}
fn load_decoder_weight_index(base:u32) -> GpuDecoderWeightIndexRecord {
  return GpuDecoderWeightIndexRecord(immutable_plan_words[base],immutable_plan_words[base+1u],immutable_plan_words[base+2u],immutable_plan_words[base+3u]);
}
fn load_perception_header(base:u32) -> GpuPerceptionHeader {
  return GpuPerceptionHeader(dispatch_header_words[base],dispatch_header_words[base+1u],dispatch_header_words[base+2u],dispatch_header_words[base+3u],dispatch_header_words[base+4u],dispatch_header_words[base+5u],dispatch_header_words[base+6u],dispatch_header_words[base+7u],dispatch_header_words[base+8u],dispatch_header_words[base+9u],dispatch_header_words[base+10u],dispatch_header_words[base+11u],dispatch_header_words[base+12u],array<u32,3>(dispatch_header_words[base+13u],dispatch_header_words[base+14u],dispatch_header_words[base+15u]));
}
fn load_candidate(base:u32) -> GpuCandidateRecord {
  return GpuCandidateRecord(dispatch_header_words[base],dispatch_header_words[base+1u],dispatch_header_words[base+2u],dispatch_header_words[base+3u],dispatch_header_words[base+4u],dispatch_header_words[base+5u],dispatch_header_words[base+6u],dispatch_header_words[base+7u]);
}
fn load_state_f32(base:u32) -> f32 { return bitcast<f32>(mutable_state_words[base]); }
fn validate_slice_a_slot(slot_index:u32, header:GpuPerceptionHeader) -> bool {
  let slot = brain_slots[slot_index];
  return header.brain_slot_index == slot_index
    && slot.schema_version == GPU_CLOSED_LOOP_LAYOUT_VERSION
    && header.schema_version == GPU_CLOSED_LOOP_LAYOUT_VERSION
    && slot.class_id == header.class_id
    && slot.slot == header.slot
    && slot.slot_generation == header.slot_generation
    && slot.neuron_count == header.neuron_count
    && slot.microstep_count == header.microstep_count
    && header.active_activation_side <= 1u
    && slot.extension_record_offset == 0xffffffffu
    && slot.reserved[0] == 0u && slot.reserved[1] == 0u && slot.reserved[2] == 0u
    && header.reserved[0] == 0u && header.reserved[1] == 0u && header.reserved[2] == 0u;
}
