const ACTIVE_DISPATCH_ROW_WORDS:u32 = 332u;
const INVALID_LOGIT_BITS:u32 = 0x7fc00001u;
const DECODER_SCHEMA_VERSION:u32 = 1u;
const FACTORIZED_MOTOR_CHANNEL_SLOT_COUNT:u32 = 6u;
const ACTION_KIND_IDLE:u32 = 0u;
const ACTION_KIND_HOLD:u32 = 1u;
const ACTION_KIND_REST:u32 = 2u;
const ACTION_KIND_INSPECT:u32 = 3u;
const ACTION_KIND_MOVE:u32 = 4u;
const ACTION_KIND_INTERACT:u32 = 5u;
const ACTION_KIND_VOCALIZE:u32 = 6u;
const ACTION_KIND_WRITE:u32 = 7u;
const ACTION_KIND_GESTURE:u32 = 8u;

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
  if (!activity_contract_prevalidated(header)) { return; }
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
  if (!activity_contract_prevalidated(header)) { return; }
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

const DECODER_HEAD_SPEECH_PAYLOAD:u32 = 3u;
const SPEECH_INPUT_WIDTH:u32 = 32u;
const SPEECH_OUTPUT_WIDTH:u32 = 32u;
const SPEECH_SOURCE_OFFSET:u32 = 128u;
const SPEECH_TARGET_OFFSET:u32 = 160u;
const SPEECH_SYNAPSE_COUNT:u32 = 1024u;
const MAX_SPEECH_TOKENS:u32 = 6u;

fn factorized_motor_slot(kind:u32) -> u32 {
  if (kind == ACTION_KIND_MOVE) { return 0u; }
  if (kind == ACTION_KIND_INTERACT || kind == ACTION_KIND_WRITE) { return 2u; }
  if (kind == ACTION_KIND_VOCALIZE) { return 3u; }
  if (kind == ACTION_KIND_HOLD || kind == ACTION_KIND_REST || kind == ACTION_KIND_INSPECT) {
    return 4u;
  }
  return 0xffffffffu;
}

fn select_factorized_motor_candidate(
  header:GpuPerceptionHeader,
  brain:GpuBrainSlotRecord,
  slot:u32
) -> u32 {
  var found = false;
  var selected = 0xffffffffu;
  var selected_logit = 0.0;
  for (var candidate=0u; candidate<header.candidate_count; candidate++) {
    let record = load_candidate(header.candidate_offset + candidate * 8u);
    if (factorized_motor_slot(record.kind) != slot) { continue; }
    let bits = load_state_u32(brain.candidate_logit_offset + candidate);
    if (bits == INVALID_LOGIT_BITS) { continue; }
    let logit = bitcast<f32>(bits);
    if (!finite_decode(logit)) { continue; }
    if (!found || logit > selected_logit || (logit == selected_logit && candidate < selected)) {
      found = true;
      selected = candidate;
      selected_logit = logit;
    }
  }
  if (!found || selected >= 255u) { return 0u; }
  return selected + 1u;
}

fn load_speech_selection(base:u32) -> GpuSelectionRecord {
  return GpuSelectionRecord(
    load_state_u32(base),load_state_u32(base+1u),load_state_u32(base+2u),load_state_u32(base+3u),
    load_state_u32(base+4u),load_state_u32(base+5u),load_state_u32(base+6u),load_state_u32(base+7u),
    load_state_u32(base+8u),load_state_u32(base+9u),load_state_u32(base+10u),load_state_u32(base+11u)
  );
}

// Runs after candidate selection in the same authoritative command buffer.
// The host receives only the packed act/token receipt and never reads neural
// activations or authors creature speech.
@compute @workgroup_size(1)
fn decode_speech_payload(@builtin(global_invocation_id) gid:vec3<u32>) {
  let header = load_perception_header(gid.y * ACTIVE_DISPATCH_ROW_WORDS);
  if (!activity_contract_prevalidated(header)) { return; }
  let brain = brain_slots[header.brain_slot_index];
  let extension = load_slot_extension(brain);
  let output_base = extension.reserved0;
  if (!state_span_within(output_base, 4u)) { return; }
  store_state_u32(output_base, 0u);
  store_state_u32(output_base + 1u, 0u);
  store_state_u32(output_base + 2u, 0u);
  store_state_u32(output_base + 3u, 0u);

  var motor_words:array<u32,2>;
  motor_words[0] = 0u;
  motor_words[1] = 0u;
  for (var slot=0u; slot<FACTORIZED_MOTOR_CHANNEL_SLOT_COUNT; slot++) {
    let selected = select_factorized_motor_candidate(header, brain, slot);
    motor_words[slot / 4u] |= selected << ((slot % 4u) * 8u);
  }
  store_state_u32(output_base + 2u, motor_words[0]);
  store_state_u32(output_base + 3u, motor_words[1]);

  let selection = load_speech_selection(brain.selection_offset);
  if (selection.status != 1u || selection.candidate_index >= header.candidate_count) { return; }
  let selected = load_candidate(header.candidate_offset + selection.candidate_index * 8u);
  if (selected.kind != ACTION_KIND_VOCALIZE) { return; }

  let decoder = load_decoder_plan(extension.decoder_input_plan_offset);
  let source_start = decoder.motor_start + SPEECH_SOURCE_OFFSET;
  let target_start = decoder.motor_start + SPEECH_TARGET_OFFSET;
  if (!span_within(source_start, SPEECH_INPUT_WIDTH, brain.neuron_count)
      || !span_within(target_start, SPEECH_OUTPUT_WIDTH, brain.neuron_count)) { return; }

  var speech_synapses = 0u;
  for (var local = 0u; local < extension.decoder_synapse_count; local++) {
    let metadata = load_decoder_eligibility_metadata(extension.decoder_metadata_offset + local * 8u);
    if (metadata.decoder_head == DECODER_HEAD_SPEECH_PAYLOAD) {
      speech_synapses += 1u;
    }
  }
  if (speech_synapses != SPEECH_SYNAPSE_COUNT) { return; }

  let learning = load_slot_learning_state(extension);
  let weight_bases = active_weight_bases(brain, extension, learning);
  let activation_base = select(
    brain.activation_a_offset,
    brain.activation_b_offset,
    selection.active_activation_side == 1u
  );
  var inputs:array<f32,32>;
  var outputs:array<f32,32>;
  var tokens:array<u32,6>;
  for (var lane = 0u; lane < SPEECH_INPUT_WIDTH; lane++) {
    inputs[lane] = load_state_f32(activation_base + source_start + lane);
  }

  var speech_act = 0u;
  var token_count = 0u;
  var confidence_byte = 0u;
  var valid = true;
  for (var step = 0u; step < MAX_SPEECH_TOKENS && valid; step++) {
    for (var output = 0u; output < SPEECH_OUTPUT_WIDTH; output++) {
      outputs[output] = 0.0;
    }
    for (var local = 0u; local < extension.decoder_synapse_count && valid; local++) {
      let metadata = load_decoder_eligibility_metadata(extension.decoder_metadata_offset + local * 8u);
      if (metadata.decoder_head != DECODER_HEAD_SPEECH_PAYLOAD) { continue; }
      let synapse = load_synapse_learning_metadata(
        extension.synapse_metadata_offset + metadata.global_synapse_id * 8u
      );
      if (metadata.reserved != 0u
          || metadata.input_lane >= SPEECH_INPUT_WIDTH
          || synapse.global_synapse_id != metadata.global_synapse_id
          || synapse.source_neuron != source_start + metadata.input_lane
          || synapse.target_neuron < target_start
          || synapse.target_neuron >= target_start + SPEECH_OUTPUT_WIDTH) {
        valid = false;
        continue;
      }
      let output = synapse.target_neuron - target_start;
      let genetic = bitcast<f32>(immutable_weight_words[brain.genetic_weight_offset + metadata.global_synapse_id]);
      let alpha = bitcast<f32>(immutable_weight_words[brain.alpha_offset + metadata.global_synapse_id]);
      let lifetime = load_state_f32(weight_bases.lifetime + metadata.global_synapse_id);
      let fast = load_state_f32(weight_bases.fast + metadata.global_synapse_id);
      outputs[output] += inputs[metadata.input_lane] * (genetic + lifetime + alpha * fast);
      valid = finite_decode(outputs[output]);
    }
    for (var output = 0u; output < SPEECH_OUTPUT_WIDTH && valid; output++) {
      outputs[output] = tanh(outputs[output]);
      valid = finite_decode(outputs[output]);
    }
    if (!valid) { break; }

    if (step == 0u) {
      var best = outputs[0];
      for (var act = 1u; act < 8u; act++) {
        if (outputs[act] > best) {
          best = outputs[act];
          speech_act = act;
        }
      }
      confidence_byte = u32(round(clamp(abs(best), 0.0, 1.0) * 255.0));
    }

    if (outputs[16] > 0.0) {
      var token = 0u;
      for (var bit = 0u; bit < 8u; bit++) {
        if (outputs[8u + bit] > 0.0) { token |= 1u << bit; }
      }
      if (token != 0u) {
        tokens[token_count] = token;
        token_count += 1u;
      }
    }
    if (outputs[17] > 0.0) { break; }
    for (var control = 0u; control < 14u; control++) {
      inputs[18u + control] = outputs[18u + control];
    }
  }

  if (!valid || token_count == 0u) { return; }
  let word0 = 1u
    | (speech_act << 2u)
    | (token_count << 5u)
    | (tokens[0] << 8u)
    | (tokens[1] << 16u)
    | (tokens[2] << 24u);
  let word1 = tokens[3]
    | (tokens[4] << 8u)
    | (tokens[5] << 16u)
    | (confidence_byte << 24u);
  store_state_u32(output_base, word0);
  store_state_u32(output_base + 1u, word1);
}
