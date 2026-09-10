const MEMORY_SCHEMA_VERSION:u32 = 1u;
const ACTIVE_DISPATCH_ROW_WORDS:u32 = 332u;
const MEMORY_HEADER_ROW_OFFSET:u32 = 292u;
const MEMORY_RECORD_WORDS:u32 = 16u;
const MEMORY_FAMILY_COUNT:u32 = 8u;
const MEMORY_TARGET_WIDTH:u32 = 8u;
const MEMORY_VALUE_WIDTH:u32 = 4u;
const MEMORY_CHANNEL_WIDTH:u32 = MEMORY_TARGET_WIDTH + MEMORY_VALUE_WIDTH;
const MEMORY_CONTEXT_DIAGNOSTIC_LANE:u32 = 2u;
const COGNITIVE_PROJECTION_SCHEMA_VERSION:u32 = 1u;
const COGNITIVE_LANE_START:u32 = 36u;
const COGNITIVE_LANE_COUNT:u32 = 18u;

fn finite_memory_value(value:f32) -> bool {
  return value == value && abs(value) <= 3.402823466e+38;
}

fn memory_sample(context:GpuCandidateMemoryRecord, channel:u32) -> f32 {
  if (channel < MEMORY_TARGET_WIDTH) {
    return context.target_latent[channel] * clamp(context.target_confidence, 0.0, 1.0);
  }
  return context.family_value[channel - MEMORY_TARGET_WIDTH]
    * clamp(context.family_confidence, 0.0, 1.0);
}

@compute @workgroup_size(32)
fn add_candidate_memory_context(@builtin(global_invocation_id) gid:vec3<u32>) {
  let row_base = gid.y * ACTIVE_DISPATCH_ROW_WORDS;
  let header = load_memory_context_header(row_base + MEMORY_HEADER_ROW_OFFSET);
  if (header.schema_version != MEMORY_SCHEMA_VERSION || header.brain_slot_index >= arrayLength(&brain_slots)) { return; }
  let brain = brain_slots[header.brain_slot_index];
  if (brain.schema_version != GPU_CLOSED_LOOP_LAYOUT_VERSION
      || brain.class_id != header.class_id
      || brain.slot != header.slot
      || brain.slot_generation != header.slot_generation
      || header.candidate_count == 0u
      || header.neural_receptor_effects_offset == 0u
      || gid.x >= header.candidate_count) { return; }
  let extension_base = brain.extension_record_offset;
  let decoder_metadata_offset = load_state_u32(extension_base + 6u);
  let memory_plan_offset = load_state_u32(extension_base + 13u);
  let memory_weight_map_offset = load_state_u32(extension_base + 14u);
  if (memory_plan_offset == 0xffffffffu || memory_weight_map_offset == 0xffffffffu) { return; }
  let plan = load_memory_channel_plan(memory_plan_offset);
  let valid_plan = plan.schema_version == MEMORY_SCHEMA_VERSION
    && plan.target_latent_lane_start == 24u
    && plan.family_value_lane_start == 32u
    && plan.decoder_input_stride == 36u
    && plan.memory_decoder_synapse_count >= MEMORY_FAMILY_COUNT * MEMORY_CHANNEL_WIDTH
    && plan.memory_decoder_synapse_count % MEMORY_FAMILY_COUNT == 0u
    && finite_memory_value(plan.max_candidate_gain)
    && plan.max_candidate_gain >= 0.0
    && all(plan.reserved == vec2<u32>(0u));
  if (!valid_plan) {
    atomicOr(&mutable_state_words[brain.diagnostic_offset + MEMORY_CONTEXT_DIAGNOSTIC_LANE], CONTRACT_INVALID_DIAGNOSTIC_BIT);
    return;
  }
  let candidate = load_candidate(header.candidate_offset + gid.x * 8u);
  let context = load_candidate_memory(header.memory_context_offset + gid.x * MEMORY_RECORD_WORDS);
  if (candidate.candidate_index != gid.x || context.candidate_index != gid.x || candidate.family >= MEMORY_FAMILY_COUNT) {
    atomicOr(&mutable_state_words[brain.diagnostic_offset + MEMORY_CONTEXT_DIAGNOSTIC_LANE], CONTRACT_INVALID_DIAGNOSTIC_BIT);
    return;
  }
  var samples:array<f32,12>;
  for (var channel=0u; channel<MEMORY_CHANNEL_WIDTH; channel++) {
    let sample = memory_sample(context, channel);
    if (!finite_memory_value(sample)) {
      atomicOr(&mutable_state_words[brain.diagnostic_offset + MEMORY_CONTEXT_DIAGNOSTIC_LANE], CONTRACT_INVALID_DIAGNOSTIC_BIT);
      return;
    }
    samples[channel] = sample;
    let input_lane = plan.target_latent_lane_start + channel;
    frame_payload_words[
      header.decoder_learning_input_offset + gid.x * plan.decoder_input_stride + input_lane
    ] = bitcast<u32>(sample);
  }
  let direct_weight_banks = load_weight_bank_pair_direct(brain);
  let weight_bases = direct_weight_banks.active_bases;
  let rows_per_family = plan.memory_decoder_synapse_count / MEMORY_FAMILY_COUNT;
  var delta = 0.0;
  for (var row=0u; row<rows_per_family; row++) {
    let local_synapse = immutable_plan_words[
      memory_weight_map_offset + candidate.family * rows_per_family + row
    ];
    if (local_synapse < brain.recurrent_synapse_count || local_synapse >= brain.synapse_count) {
      atomicOr(&mutable_state_words[brain.diagnostic_offset + MEMORY_CONTEXT_DIAGNOSTIC_LANE], CONTRACT_INVALID_DIAGNOSTIC_BIT);
      return;
    }
    let decoder_local = local_synapse - brain.recurrent_synapse_count;
    let metadata = load_decoder_eligibility_metadata(decoder_metadata_offset + decoder_local * 8u);
    if (metadata.global_synapse_id != local_synapse
        || metadata.decoder_head != 2u
        || metadata.family != candidate.family
        || metadata.input_lane < plan.target_latent_lane_start
        || metadata.input_lane >= plan.decoder_input_stride) {
      atomicOr(&mutable_state_words[brain.diagnostic_offset + MEMORY_CONTEXT_DIAGNOSTIC_LANE], CONTRACT_INVALID_DIAGNOSTIC_BIT);
      return;
    }
    let channel = metadata.input_lane - plan.target_latent_lane_start;
    let genetic = bitcast<f32>(immutable_weight_words[brain.genetic_weight_offset + local_synapse]);
    let alpha = bitcast<f32>(immutable_weight_words[brain.alpha_offset + local_synapse]);
    let lifetime = load_state_f32(weight_bases.lifetime + local_synapse);
    let fast = load_state_f32(weight_bases.fast + local_synapse);
    delta += samples[channel] * (genetic + lifetime + alpha * fast);
    if (!finite_memory_value(delta)) {
      atomicOr(&mutable_state_words[brain.diagnostic_offset + MEMORY_CONTEXT_DIAGNOSTIC_LANE], CONTRACT_INVALID_DIAGNOSTIC_BIT);
      return;
    }
  }
  let logit_index = brain.candidate_logit_offset + gid.x;
  let base_logit = load_state_f32(logit_index);
  var cognitive_delta = 0.0;
  let cognitive_base = header.memory_context_offset
    + header.candidate_count * MEMORY_RECORD_WORDS;
  let cognitive = load_cognitive_projection(cognitive_base + gid.x * 24u);
  if (cognitive.schema_version == COGNITIVE_PROJECTION_SCHEMA_VERSION) {
    if (cognitive.candidate_index != gid.x
        || cognitive.forecast_available > 1u
        || cognitive.reserved != 0u
        || cognitive.reserved_tail[0] != 0u
        || cognitive.reserved_tail[1] != 0u) {
      atomicOr(&mutable_state_words[brain.diagnostic_offset + MEMORY_CONTEXT_DIAGNOSTIC_LANE], CONTRACT_INVALID_DIAGNOSTIC_BIT);
      return;
    }
    for (var channel=0u; channel<COGNITIVE_LANE_COUNT; channel++) {
      let sample = cognitive.values[channel];
      if (!finite_memory_value(sample)) {
        atomicOr(&mutable_state_words[brain.diagnostic_offset + MEMORY_CONTEXT_DIAGNOSTIC_LANE], CONTRACT_INVALID_DIAGNOSTIC_BIT);
        return;
      }
      frame_payload_words[
        header.decoder_learning_input_offset + gid.x * plan.decoder_input_stride
          + COGNITIVE_LANE_START + channel
      ] = bitcast<u32>(sample);
    }
    var cognitive_rows = 0u;
    let decoder_count = brain.synapse_count - brain.recurrent_synapse_count;
    for (var decoder_local=0u; decoder_local<decoder_count; decoder_local++) {
      let metadata = load_decoder_eligibility_metadata(decoder_metadata_offset + decoder_local * 8u);
      if (metadata.decoder_head == 4u
          && metadata.family == candidate.family
          && metadata.input_lane >= COGNITIVE_LANE_START
          && metadata.input_lane < COGNITIVE_LANE_START + COGNITIVE_LANE_COUNT) {
        if (metadata.global_synapse_id < brain.recurrent_synapse_count
            || metadata.global_synapse_id >= brain.synapse_count) {
          atomicOr(&mutable_state_words[brain.diagnostic_offset + MEMORY_CONTEXT_DIAGNOSTIC_LANE], CONTRACT_INVALID_DIAGNOSTIC_BIT);
          return;
        }
        let sample = cognitive.values[metadata.input_lane - COGNITIVE_LANE_START];
        let genetic = bitcast<f32>(immutable_weight_words[brain.genetic_weight_offset + metadata.global_synapse_id]);
        let alpha = bitcast<f32>(immutable_weight_words[brain.alpha_offset + metadata.global_synapse_id]);
        let lifetime = load_state_f32(weight_bases.lifetime + metadata.global_synapse_id);
        let fast = load_state_f32(weight_bases.fast + metadata.global_synapse_id);
        cognitive_delta += sample * (genetic + lifetime + alpha * fast);
        cognitive_rows += 1u;
      }
    }
    if (cognitive_rows != 0u && cognitive_rows != COGNITIVE_LANE_COUNT) {
      atomicOr(&mutable_state_words[brain.diagnostic_offset + MEMORY_CONTEXT_DIAGNOSTIC_LANE], CONTRACT_INVALID_DIAGNOSTIC_BIT);
      return;
    }
    if (!finite_memory_value(cognitive_delta)) {
      atomicOr(&mutable_state_words[brain.diagnostic_offset + MEMORY_CONTEXT_DIAGNOSTIC_LANE], CONTRACT_INVALID_DIAGNOSTIC_BIT);
      return;
    }
  } else if (cognitive.schema_version != 0u) {
    atomicOr(&mutable_state_words[brain.diagnostic_offset + MEMORY_CONTEXT_DIAGNOSTIC_LANE], CONTRACT_INVALID_DIAGNOSTIC_BIT);
    return;
  }
  store_state_f32(logit_index,
    base_logit
      + clamp(delta, -plan.max_candidate_gain, plan.max_candidate_gain)
      + clamp(cognitive_delta, -plan.max_candidate_gain, plan.max_candidate_gain));
}
