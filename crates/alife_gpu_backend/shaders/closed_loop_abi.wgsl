const GPU_CLOSED_LOOP_LAYOUT_VERSION:u32 = 3u;
const GPU_LEARNING_SCHEMA_VERSION:u32 = 1u;
const GPU_SLEEP_SCHEMA_VERSION:u32 = 1u;

struct GpuPerceptionHeader {
  schema_version:u32, class_id:u32, slot:u32, slot_generation:u32,
  neuron_count:u32, candidate_count:u32, microstep_count:u32, active_activation_side:u32,
  tick_lo:u32, tick_hi:u32, sensory_offset:u32, candidate_offset:u32,
  brain_slot_index:u32, dispatch_generation_lo:u32, dispatch_generation_hi:u32, reserved:u32,
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
struct GpuCandidateMemoryRecord {
  @align(16) candidate_index:u32, target_confidence:f32, family_confidence:f32, source_counts_packed:u32,
  target_latent:array<f32,8>, family_value:array<f32,4>,
}
struct GpuMemoryContextHeader {
  @align(16) schema_version:u32, class_id:u32, slot:u32, slot_generation:u32,
  tick_lo:u32, tick_hi:u32, candidate_count:u32, memory_context_offset:u32,
  candidate_offset:u32, profile_id:u32, profile_schema_version:u32, sensory_abi_version:u32,
  brain_slot_index:u32, decoder_learning_input_offset:u32, perception_header_index:u32, reserved:u32,
}
struct GpuMemoryChannelPlan {
  @align(16) schema_version:u32, target_latent_lane_start:u32, family_value_lane_start:u32, decoder_input_stride:u32,
  max_candidate_gain:f32, memory_decoder_synapse_count:u32, reserved:vec2<u32>,
}
struct GpuBrainSlotExtensionRecord {
  schema_version:u32, projection_count:u32, decoder_synapse_local_start:u32, decoder_synapse_count:u32,
  receptor_offset:u32, decoder_input_plan_offset:u32, decoder_metadata_offset:u32, synapse_metadata_offset:u32,
  recurrent_eligibility_bank_1_offset:u32, decoder_eligibility_bank_1_offset:u32, fast_bank_1_offset:u32, lifetime_bank_1_offset:u32,
  sleep_parameter_offset:u32, memory_plan_offset:u32, memory_weight_map_offset:u32, learning_state_offset:u32,
  pending_eligibility_offset:u32, replay_plan_identity_offset:u32, reserved0:u32, reserved1:u32,
}
struct GpuSlotLearningStateRecord {
  schema_version:u32, active_weight_bank:u32, active_eligibility_bank:u32, pending_valid:u32,
  active_weight_generation_lo:u32, active_weight_generation_hi:u32, active_eligibility_generation_lo:u32, active_eligibility_generation_hi:u32,
  inactive_eligibility_generation_lo:u32, inactive_eligibility_generation_hi:u32, replay_generation_lo:u32, replay_generation_hi:u32,
  replay_cursor:u32, replay_event_count:u32, replay_event_capacity:u32, replay_sample_capacity:u32,
  replay_span_count:u32, replay_event_rows_offset:u32, replay_sample_offset:u32, replay_span_offset:u32,
  replay_plan_identity_offset:u32, pending_eligibility_offset:u32, transaction_generation_lo:u32, transaction_generation_hi:u32,
}
struct GpuLearningHeader {
  schema_version:u32, class_id:u32, slot:u32, slot_generation:u32,
  brain_slot_index:u32, active_activation_side:u32, dispatch_generation_lo:u32, dispatch_generation_hi:u32,
  candidate_count:u32, candidate_offset:u32, decoder_learning_input_offset:u32, selection_offset:u32,
  outcome_offset:u32, recurrent_synapse_count:u32, decoder_synapse_count:u32, decoder_input_stride:u32,
  pending_eligibility_offset:u32, reserved:array<u32,3>,
}
struct GpuSynapseLearningMetadata {
  global_synapse_id:u32, kind:u32, source_neuron:u32, target_neuron:u32,
  receptor_index:u32, eligibility_local_index:u32, decoder_metadata_local_or_max:u32, reserved:u32,
}
struct GpuDecoderEligibilityMetadata {
  global_synapse_id:u32, decoder_head:u32, family:u32, input_lane:u32,
  motor_index:u32, receptor_index:u32, eligibility_local_index:u32, reserved:u32,
}
struct GpuPlasticityReceptorRecord {
  eligibility_decay:f32, learning_rate:f32, sleep_replay_rate:f32, normalization_rate:f32,
  modulator_sign:f32, fast_min:f32, fast_max:f32, reserved:f32,
}
struct GpuSleepParameterRecord {
  schema_version:u32, staging_rate:f32, weight_limit:f32, fast_decay_rate:f32,
  eligibility_reset_policy:u32, replay_consume_policy:u32, reserved:vec2<u32>,
}
struct GpuReplayEventRecord {
  sequence_id:vec2<u32>, originating_tick:vec2<u32>, frame_digest:array<u32,8>,
  candidate_feature_digest:vec4<u32>, action_id:u32, family:u32,
  reward_prediction_error:f32, pain:f32, homeostatic_improvement:f32, frustration:f32,
  novelty:f32, modulator_value:f32,
}
struct GpuSleepHeader {
  schema_version:u32, class_id:u32, slot:u32, slot_generation:u32,
  brain_slot_index:u32, request_offset:u32, replay_event_offset:u32, replay_event_count:u32,
  replay_span_offset:u32, replay_span_count:u32, replay_sample_offset:u32, replay_sample_count:u32,
  synapse_count:u32, completion_offset:u32, job_id_lo:u32, job_id_hi:u32,
  cycle_id_lo:u32, cycle_id_hi:u32, flags:u32, reserved:u32,
}
struct GpuConsolidationRequestRecord {
  schema_version:u32, request_flags:u32, cycle_id_lo:u32, cycle_id_hi:u32,
  phenotype_hash:array<u32,8>, input_generation_lo:u32, input_generation_hi:u32,
  expected_output_generation_lo:u32, expected_output_generation_hi:u32,
  input_digest:array<u32,8>, replay_digest:array<u32,8>,
  max_replay_events:u32, max_replay_eligibility_samples:u32,
  request_digest:array<u32,8>, reserved_tail:vec2<u32>,
}
struct GpuPendingEligibilityRecord {
  schema_version:u32, slot:u32, slot_generation:u32, active_activation_side:u32,
  phenotype_hash:array<u32,8>, organism_id:vec2<u32>, dispatch_generation:vec2<u32>, originating_tick:vec2<u32>,
  frame_digest:array<u32,8>, candidate_index_and_family:u32, action_id:u32,
  candidate_feature_digest:vec4<u32>, active_eligibility_generation:vec2<u32>, staging_eligibility_generation:vec2<u32>,
}
struct GpuOutcomeCreditRecord {
  schema_version:u32, selected_candidate_and_family:u32, organism_id:vec2<u32>,
  phenotype_hash:array<u32,8>, sequence_id:vec2<u32>, originating_tick:vec2<u32>, outcome_tick:vec2<u32>,
  selected_action:u32, active_activation_side:u32, candidate_feature_digest:vec4<u32>,
  frame_digest:array<u32,8>, dispatch_generation:vec2<u32>,
  reward_prediction_error:f32, pain:f32, homeostatic_improvement:f32, frustration:f32,
  novelty:f32, modulator_value:f32,
}
struct GpuFastPlasticityCommitRecord {
  schema_version:u32, slot:u32, slot_generation:u32, status:u32,
  input_fast_generation:vec2<u32>, output_fast_generation:vec2<u32>,
  output_eligibility_generation:vec2<u32>, replay_generation:vec2<u32>,
  transaction_generation:vec2<u32>, fast_weights_changed:u32, max_abs_delta_bits:u32,
}
struct GpuReplaySynapseSpanRecord {
  local_synapse_id:u32, sample_start:u32, sample_count:u32, reserved:u32,
}
struct GpuWeightBankBases { lifetime:u32, fast:u32, }
struct GpuEligibilityBankBases { recurrent:u32, decoder:u32, }

@group(0) @binding(0) var<storage, read> brain_slots: array<GpuBrainSlotRecord>;
@group(0) @binding(1) var<storage, read> phenotype_identities: array<GpuPhenotypeIdentityRecord>;
@group(0) @binding(2) var<storage, read> immutable_plan_words: array<u32>;
@group(0) @binding(3) var<storage, read> immutable_weight_words: array<u32>;
@group(0) @binding(4) var<storage, read> dispatch_header_words: array<u32>;
@group(0) @binding(5) var<storage, read_write> frame_payload_words: array<u32>;
@group(0) @binding(6) var<storage, read_write> mutable_state_words: array<atomic<u32>>;

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
fn load_memory_context_header(base:u32) -> GpuMemoryContextHeader {
  return GpuMemoryContextHeader(
    dispatch_header_words[base],dispatch_header_words[base+1u],dispatch_header_words[base+2u],dispatch_header_words[base+3u],
    dispatch_header_words[base+4u],dispatch_header_words[base+5u],dispatch_header_words[base+6u],dispatch_header_words[base+7u],
    dispatch_header_words[base+8u],dispatch_header_words[base+9u],dispatch_header_words[base+10u],dispatch_header_words[base+11u],
    dispatch_header_words[base+12u],dispatch_header_words[base+13u],dispatch_header_words[base+14u],dispatch_header_words[base+15u]
  );
}
fn load_candidate_memory(base:u32) -> GpuCandidateMemoryRecord {
  return GpuCandidateMemoryRecord(
    frame_payload_words[base],bitcast<f32>(frame_payload_words[base+1u]),bitcast<f32>(frame_payload_words[base+2u]),frame_payload_words[base+3u],
    array<f32,8>(
      bitcast<f32>(frame_payload_words[base+4u]),bitcast<f32>(frame_payload_words[base+5u]),
      bitcast<f32>(frame_payload_words[base+6u]),bitcast<f32>(frame_payload_words[base+7u]),
      bitcast<f32>(frame_payload_words[base+8u]),bitcast<f32>(frame_payload_words[base+9u]),
      bitcast<f32>(frame_payload_words[base+10u]),bitcast<f32>(frame_payload_words[base+11u])
    ),
    array<f32,4>(
      bitcast<f32>(frame_payload_words[base+12u]),bitcast<f32>(frame_payload_words[base+13u]),
      bitcast<f32>(frame_payload_words[base+14u]),bitcast<f32>(frame_payload_words[base+15u])
    )
  );
}
fn load_memory_channel_plan(base:u32) -> GpuMemoryChannelPlan {
  return GpuMemoryChannelPlan(
    immutable_plan_words[base],immutable_plan_words[base+1u],immutable_plan_words[base+2u],immutable_plan_words[base+3u],
    bitcast<f32>(immutable_plan_words[base+4u]),immutable_plan_words[base+5u],
    vec2<u32>(immutable_plan_words[base+6u],immutable_plan_words[base+7u])
  );
}
fn load_perception_header(base:u32) -> GpuPerceptionHeader {
  return GpuPerceptionHeader(dispatch_header_words[base],dispatch_header_words[base+1u],dispatch_header_words[base+2u],dispatch_header_words[base+3u],dispatch_header_words[base+4u],dispatch_header_words[base+5u],dispatch_header_words[base+6u],dispatch_header_words[base+7u],dispatch_header_words[base+8u],dispatch_header_words[base+9u],dispatch_header_words[base+10u],dispatch_header_words[base+11u],dispatch_header_words[base+12u],dispatch_header_words[base+13u],dispatch_header_words[base+14u],dispatch_header_words[base+15u]);
}
fn load_candidate(base:u32) -> GpuCandidateRecord {
  return GpuCandidateRecord(dispatch_header_words[base],dispatch_header_words[base+1u],dispatch_header_words[base+2u],dispatch_header_words[base+3u],dispatch_header_words[base+4u],dispatch_header_words[base+5u],dispatch_header_words[base+6u],dispatch_header_words[base+7u]);
}
fn load_state_u32(base:u32) -> u32 { return atomicLoad(&mutable_state_words[base]); }
fn store_state_u32(base:u32, value:u32) { atomicStore(&mutable_state_words[base], value); }
fn load_state_f32(base:u32) -> f32 { return bitcast<f32>(load_state_u32(base)); }
fn store_state_f32(base:u32, value:f32) { store_state_u32(base, bitcast<u32>(value)); }
fn state_span_within(start:u32, count:u32) -> bool {
  let limit = arrayLength(&mutable_state_words);
  return start <= limit && count <= limit - start;
}
fn load_learning_header(base:u32) -> GpuLearningHeader {
  return GpuLearningHeader(
    dispatch_header_words[base],dispatch_header_words[base+1u],dispatch_header_words[base+2u],dispatch_header_words[base+3u],
    dispatch_header_words[base+4u],dispatch_header_words[base+5u],dispatch_header_words[base+6u],dispatch_header_words[base+7u],
    dispatch_header_words[base+8u],dispatch_header_words[base+9u],dispatch_header_words[base+10u],dispatch_header_words[base+11u],
    dispatch_header_words[base+12u],dispatch_header_words[base+13u],dispatch_header_words[base+14u],dispatch_header_words[base+15u],
    dispatch_header_words[base+16u],array<u32,3>(dispatch_header_words[base+17u],dispatch_header_words[base+18u],dispatch_header_words[base+19u])
  );
}
fn load_synapse_learning_metadata(base:u32) -> GpuSynapseLearningMetadata {
  return GpuSynapseLearningMetadata(
    immutable_plan_words[base],immutable_plan_words[base+1u],immutable_plan_words[base+2u],immutable_plan_words[base+3u],
    immutable_plan_words[base+4u],immutable_plan_words[base+5u],immutable_plan_words[base+6u],immutable_plan_words[base+7u]
  );
}
fn load_decoder_eligibility_metadata(base:u32) -> GpuDecoderEligibilityMetadata {
  return GpuDecoderEligibilityMetadata(
    immutable_plan_words[base],immutable_plan_words[base+1u],immutable_plan_words[base+2u],immutable_plan_words[base+3u],
    immutable_plan_words[base+4u],immutable_plan_words[base+5u],immutable_plan_words[base+6u],immutable_plan_words[base+7u]
  );
}
fn load_plasticity_receptor(base:u32) -> GpuPlasticityReceptorRecord {
  return GpuPlasticityReceptorRecord(
    bitcast<f32>(immutable_plan_words[base]),bitcast<f32>(immutable_plan_words[base+1u]),
    bitcast<f32>(immutable_plan_words[base+2u]),bitcast<f32>(immutable_plan_words[base+3u]),
    bitcast<f32>(immutable_plan_words[base+4u]),bitcast<f32>(immutable_plan_words[base+5u]),
    bitcast<f32>(immutable_plan_words[base+6u]),bitcast<f32>(immutable_plan_words[base+7u])
  );
}
fn load_sleep_parameter(base:u32) -> GpuSleepParameterRecord {
  return GpuSleepParameterRecord(
    immutable_plan_words[base],bitcast<f32>(immutable_plan_words[base+1u]),
    bitcast<f32>(immutable_plan_words[base+2u]),bitcast<f32>(immutable_plan_words[base+3u]),
    immutable_plan_words[base+4u],immutable_plan_words[base+5u],
    vec2<u32>(immutable_plan_words[base+6u],immutable_plan_words[base+7u])
  );
}
fn load_sleep_header(base:u32) -> GpuSleepHeader {
  return GpuSleepHeader(
    dispatch_header_words[base],dispatch_header_words[base+1u],dispatch_header_words[base+2u],dispatch_header_words[base+3u],
    dispatch_header_words[base+4u],dispatch_header_words[base+5u],dispatch_header_words[base+6u],dispatch_header_words[base+7u],
    dispatch_header_words[base+8u],dispatch_header_words[base+9u],dispatch_header_words[base+10u],dispatch_header_words[base+11u],
    dispatch_header_words[base+12u],dispatch_header_words[base+13u],dispatch_header_words[base+14u],dispatch_header_words[base+15u],
    dispatch_header_words[base+16u],dispatch_header_words[base+17u],dispatch_header_words[base+18u],dispatch_header_words[base+19u]
  );
}
fn load_consolidation_request(base:u32) -> GpuConsolidationRequestRecord {
  return GpuConsolidationRequestRecord(
    frame_payload_words[base],frame_payload_words[base+1u],frame_payload_words[base+2u],frame_payload_words[base+3u],
    array<u32,8>(
      frame_payload_words[base+4u],frame_payload_words[base+5u],frame_payload_words[base+6u],frame_payload_words[base+7u],
      frame_payload_words[base+8u],frame_payload_words[base+9u],frame_payload_words[base+10u],frame_payload_words[base+11u]
    ),
    frame_payload_words[base+12u],frame_payload_words[base+13u],frame_payload_words[base+14u],frame_payload_words[base+15u],
    array<u32,8>(
      frame_payload_words[base+16u],frame_payload_words[base+17u],frame_payload_words[base+18u],frame_payload_words[base+19u],
      frame_payload_words[base+20u],frame_payload_words[base+21u],frame_payload_words[base+22u],frame_payload_words[base+23u]
    ),
    array<u32,8>(
      frame_payload_words[base+24u],frame_payload_words[base+25u],frame_payload_words[base+26u],frame_payload_words[base+27u],
      frame_payload_words[base+28u],frame_payload_words[base+29u],frame_payload_words[base+30u],frame_payload_words[base+31u]
    ),
    frame_payload_words[base+32u],frame_payload_words[base+33u],
    array<u32,8>(
      frame_payload_words[base+34u],frame_payload_words[base+35u],frame_payload_words[base+36u],frame_payload_words[base+37u],
      frame_payload_words[base+38u],frame_payload_words[base+39u],frame_payload_words[base+40u],frame_payload_words[base+41u]
    ),
    vec2<u32>(frame_payload_words[base+42u],frame_payload_words[base+43u])
  );
}
fn load_slot_extension(brain:GpuBrainSlotRecord) -> GpuBrainSlotExtensionRecord {
  let base = brain.extension_record_offset;
  return GpuBrainSlotExtensionRecord(
    load_state_u32(base),load_state_u32(base+1u),load_state_u32(base+2u),load_state_u32(base+3u),
    load_state_u32(base+4u),load_state_u32(base+5u),load_state_u32(base+6u),load_state_u32(base+7u),
    load_state_u32(base+8u),load_state_u32(base+9u),load_state_u32(base+10u),load_state_u32(base+11u),
    load_state_u32(base+12u),load_state_u32(base+13u),load_state_u32(base+14u),load_state_u32(base+15u),
    load_state_u32(base+16u),load_state_u32(base+17u),load_state_u32(base+18u),load_state_u32(base+19u)
  );
}
fn load_slot_learning_state(extension:GpuBrainSlotExtensionRecord) -> GpuSlotLearningStateRecord {
  let base = extension.learning_state_offset;
  return GpuSlotLearningStateRecord(
    load_state_u32(base),load_state_u32(base+1u),load_state_u32(base+2u),load_state_u32(base+3u),
    load_state_u32(base+4u),load_state_u32(base+5u),load_state_u32(base+6u),load_state_u32(base+7u),
    load_state_u32(base+8u),load_state_u32(base+9u),load_state_u32(base+10u),load_state_u32(base+11u),
    load_state_u32(base+12u),load_state_u32(base+13u),load_state_u32(base+14u),load_state_u32(base+15u),
    load_state_u32(base+16u),load_state_u32(base+17u),load_state_u32(base+18u),load_state_u32(base+19u),
    load_state_u32(base+20u),load_state_u32(base+21u),load_state_u32(base+22u),load_state_u32(base+23u)
  );
}
fn active_weight_bases(
  brain:GpuBrainSlotRecord,
  extension:GpuBrainSlotExtensionRecord,
  learning:GpuSlotLearningStateRecord,
) -> GpuWeightBankBases {
  let bank_1 = learning.active_weight_bank == 1u;
  return GpuWeightBankBases(
    select(brain.lifetime_weight_offset, extension.lifetime_bank_1_offset, bank_1),
    select(brain.fast_weight_offset, extension.fast_bank_1_offset, bank_1)
  );
}
fn inactive_weight_bases(
  brain:GpuBrainSlotRecord,
  extension:GpuBrainSlotExtensionRecord,
  learning:GpuSlotLearningStateRecord,
) -> GpuWeightBankBases {
  let bank_1 = learning.active_weight_bank == 0u;
  return GpuWeightBankBases(
    select(brain.lifetime_weight_offset, extension.lifetime_bank_1_offset, bank_1),
    select(brain.fast_weight_offset, extension.fast_bank_1_offset, bank_1)
  );
}
fn active_eligibility_bases(
  brain:GpuBrainSlotRecord,
  extension:GpuBrainSlotExtensionRecord,
  learning:GpuSlotLearningStateRecord,
) -> GpuEligibilityBankBases {
  let bank_1 = learning.active_eligibility_bank == 1u;
  return GpuEligibilityBankBases(
    select(brain.recurrent_eligibility_offset, extension.recurrent_eligibility_bank_1_offset, bank_1),
    select(brain.decoder_eligibility_offset, extension.decoder_eligibility_bank_1_offset, bank_1)
  );
}
fn inactive_eligibility_bases(
  brain:GpuBrainSlotRecord,
  extension:GpuBrainSlotExtensionRecord,
  learning:GpuSlotLearningStateRecord,
) -> GpuEligibilityBankBases {
  let bank_1 = learning.active_eligibility_bank == 0u;
  return GpuEligibilityBankBases(
    select(brain.recurrent_eligibility_offset, extension.recurrent_eligibility_bank_1_offset, bank_1),
    select(brain.decoder_eligibility_offset, extension.decoder_eligibility_bank_1_offset, bank_1)
  );
}
fn validate_slice_a_slot(slot_index:u32, header:GpuPerceptionHeader) -> bool {
  let slot = brain_slots[slot_index];
  var valid = header.brain_slot_index == slot_index
    && slot.schema_version == GPU_CLOSED_LOOP_LAYOUT_VERSION
    && header.schema_version == GPU_CLOSED_LOOP_LAYOUT_VERSION
    && slot.class_id == header.class_id
    && slot.slot == header.slot
    && slot.slot_generation == header.slot_generation
    && slot.neuron_count == header.neuron_count
    && slot.microstep_count == header.microstep_count
    && header.active_activation_side <= 1u
    && slot.extension_record_offset != 0xffffffffu
    && state_span_within(slot.extension_record_offset, 20u)
    && slot.reserved[0] == 0u && slot.reserved[1] == 0u && slot.reserved[2] == 0u
    && (header.dispatch_generation_lo != 0u || header.dispatch_generation_hi != 0u)
    && header.reserved == 0u;
  if (!valid) { return false; }
  let extension = load_slot_extension(slot);
  valid = extension.schema_version == GPU_CLOSED_LOOP_LAYOUT_VERSION
    && extension.reserved0 == 0u && extension.reserved1 == 0u
    && state_span_within(extension.learning_state_offset, 24u)
    && extension.pending_eligibility_offset != 0xffffffffu
    && extension.replay_plan_identity_offset != 0xffffffffu
    && extension.sleep_parameter_offset != 0xffffffffu;
  if (!valid) { return false; }
  let learning = load_slot_learning_state(extension);
  return learning.schema_version == GPU_LEARNING_SCHEMA_VERSION
    && learning.active_weight_bank <= 1u
    && learning.active_eligibility_bank <= 1u
    && learning.pending_valid <= 1u
    && learning.pending_eligibility_offset == extension.pending_eligibility_offset
    && learning.replay_plan_identity_offset == extension.replay_plan_identity_offset;
}
