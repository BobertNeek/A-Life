const ACTIVE_DISPATCH_ROW_WORDS:u32 = 332u;
const INVALID_LOGIT_BITS:u32 = 0x7fc00001u;
const DECODER_SCHEMA_VERSION:u32 = 1u;

fn find_decoder_family(decoder:GpuDecoderPlanRecord, family_raw:u32) -> GpuDecoderFamilyRecord {
  for (var index=0u; index<decoder.family_count; index++) {
    let family = load_decoder_family(decoder.family_offset + index * 8u);
    if (family.family_raw == family_raw) { return family; }
  }
  return GpuDecoderFamilyRecord(0xffffffffu,0u,0u,0u,0u,0u,0u,0u);
}

fn finite_decode(value:f32) -> bool {
  return value == value && abs(value) <= 3.402823466e+38;
}

fn span_within(start:u32, count:u32, limit:u32) -> bool {
  return start <= limit && count <= limit - start;
}

@compute @workgroup_size(32)
fn decode_candidates(@builtin(global_invocation_id) gid:vec3<u32>) {
  let header = load_perception_header(gid.y * ACTIVE_DISPATCH_ROW_WORDS);
  if (!validate_slice_a_slot(header.brain_slot_index, header)) { return; }
  let brain = brain_slots[header.brain_slot_index];
  let extension = load_slot_extension(brain);
  let learning = load_slot_learning_state(extension);
  let weight_bases = active_weight_bases(brain, extension, learning);
  let candidate = gid.x;
  if (candidate >= header.candidate_count) { return; }
  let candidate_record = load_candidate(header.candidate_offset + candidate * 8u);
  let decoder = load_decoder_plan(brain.decoder_plan_offset);
  let frame_word_count = arrayLength(&frame_payload_words);
  var valid = decoder.schema_version == DECODER_SCHEMA_VERSION
    && decoder.feature_count == 24u
    && decoder.flattened_input_lane_count >= decoder.feature_count
    && decoder.flattened_input_lane_count <= 64u
    && decoder.family_offset == brain.decoder_family_offset
    && decoder.family_count == 8u
    && span_within(decoder.motor_start, decoder.motor_width, brain.neuron_count)
    && brain.recurrent_synapse_count <= brain.synapse_count
    && span_within(
      brain.recurrent_synapse_count,
      decoder.decoder_synapse_count,
      brain.synapse_count
    )
    && candidate_record.candidate_index == candidate
    && span_within(candidate_record.feature_offset, 24u, frame_word_count);
  var family = GpuDecoderFamilyRecord(0xffffffffu,0u,0u,0u,0u,0u,0u,0u);
  if (valid) { family = find_decoder_family(decoder, candidate_record.family); }
  valid = valid && family.family_raw == candidate_record.family
    && family.weight_index_count == family.decoder_synapse_count
    && family.reserved0 == 0u && family.reserved1 == 0u
    && family.decoder_synapse_start >= brain.recurrent_synapse_count
    && span_within(family.decoder_synapse_start, family.decoder_synapse_count, brain.synapse_count)
    && family.weight_index_start >= brain.decoder_weight_indices_offset
    && family.weight_index_count <= 0x3fffffffu
    && decoder.decoder_synapse_count <= 0x3fffffffu
    && span_within(
      family.weight_index_start - brain.decoder_weight_indices_offset,
      family.weight_index_count * 4u,
      decoder.decoder_synapse_count * 4u
    );
  let final_side = atomicLoad(&mutable_state_words[brain.diagnostic_offset + 3u]);
  valid = valid && final_side <= 1u;
  let activation_offset = select(brain.activation_a_offset, brain.activation_b_offset, final_side == 1u);
  var logit = bitcast<f32>(family.bias_bits);
  for (var index=0u; index<family.weight_index_count && valid; index++) {
    let map = load_decoder_weight_index(family.weight_index_start + index * 4u);
    valid = map.reserved0 == 0u
      && map.input_lane < decoder.feature_count
      && map.motor_index < decoder.motor_width
      && map.global_synapse_id == family.decoder_synapse_start + index
      && map.global_synapse_id >= brain.recurrent_synapse_count
      && map.global_synapse_id < brain.synapse_count;
    if (valid) {
      let motor = load_state_f32(activation_offset + decoder.motor_start + map.motor_index);
      let feature = bitcast<f32>(frame_payload_words[candidate_record.feature_offset + map.input_lane]);
      let genetic = bitcast<f32>(immutable_weight_words[brain.genetic_weight_offset + map.global_synapse_id]);
      let alpha = bitcast<f32>(immutable_weight_words[brain.alpha_offset + map.global_synapse_id]);
      let lifetime = load_state_f32(weight_bases.lifetime + map.global_synapse_id);
      let fast = load_state_f32(weight_bases.fast + map.global_synapse_id);
      logit += motor * feature * (genetic + lifetime + alpha * fast);
      valid = finite_decode(logit);
    }
  }
  if (!valid || !finite_decode(logit)) {
    store_state_u32(brain.candidate_logit_offset + candidate, INVALID_LOGIT_BITS);
    atomicAdd(&mutable_state_words[brain.diagnostic_offset + 2u], 1u);
    return;
  }
  store_state_f32(brain.candidate_logit_offset + candidate, logit);
}

@compute @workgroup_size(1)
fn select_candidate(@builtin(global_invocation_id) gid:vec3<u32>) {
  let header = load_perception_header(gid.y * ACTIVE_DISPATCH_ROW_WORDS);
  if (!validate_slice_a_slot(header.brain_slot_index, header)) { return; }
  let brain = brain_slots[header.brain_slot_index];
  var found = false;
  var selected_candidate = 0xffffffffu;
  var selected_logit = 0.0;
  var selected_confidence = 0u;
  for (var candidate=0u; candidate<header.candidate_count; candidate++) {
    let bits = load_state_u32(brain.candidate_logit_offset + candidate);
    if (bits == INVALID_LOGIT_BITS) { continue; }
    let logit = bitcast<f32>(bits);
    if (!finite_decode(logit)) { continue; }
    let candidate_record = load_candidate(header.candidate_offset + candidate * 8u);
    if (!found || logit > selected_logit || (logit == selected_logit && candidate < selected_candidate)) {
      found = true;
      selected_candidate = candidate;
      selected_logit = logit;
      selected_confidence = candidate_record.confidence_q16;
    }
  }
  let base = brain.selection_offset;
  atomicStore(&mutable_state_words[base], brain.slot);
  atomicStore(&mutable_state_words[base + 1u], brain.slot_generation);
  atomicStore(&mutable_state_words[base + 2u], select(0xffffffffu, selected_candidate, found));
  atomicStore(&mutable_state_words[base + 3u], select(0u, bitcast<u32>(selected_logit), found));
  atomicStore(&mutable_state_words[base + 4u], select(0u, selected_confidence, found));
  atomicStore(&mutable_state_words[base + 5u], select(2u, 1u, found));
  atomicStore(&mutable_state_words[base + 6u], atomicLoad(&mutable_state_words[brain.diagnostic_offset]));
  atomicStore(&mutable_state_words[base + 7u], atomicLoad(&mutable_state_words[brain.diagnostic_offset + 1u]));
  atomicStore(&mutable_state_words[base + 8u], atomicLoad(&mutable_state_words[brain.diagnostic_offset + 2u]));
  atomicStore(&mutable_state_words[base + 9u], header.dispatch_generation_lo);
  atomicStore(&mutable_state_words[base + 10u], header.dispatch_generation_hi);
  atomicStore(&mutable_state_words[base + 11u], atomicLoad(&mutable_state_words[brain.diagnostic_offset + 3u]));
}
