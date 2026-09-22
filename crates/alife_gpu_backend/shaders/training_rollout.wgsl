// Included only by the training-rollout feature. No CPU action selection.
fn training_random(seed:u32, counter:u32, factor:u32) -> f32 {
  var value = seed ^ (counter * 0x9e3779b9u) ^ (factor * 0x85ebca6bu);
  value = (value ^ (value >> 16u)) * 0x7feb352du;
  value = (value ^ (value >> 15u)) * 0x846ca68bu;
  value = value ^ (value >> 16u);
  // 23 bits keep the midpoint exactly representable: the draw is strictly < 1.
  return (f32(value >> 9u) + 0.5) / 8388608.0;
}

fn training_candidate_allowed(header:GpuPerceptionHeader, brain:GpuBrainSlotRecord, index:u32, factor:u32) -> bool {
  let bits = load_state_u32(brain.candidate_logit_offset + index);
  if (bits == INVALID_LOGIT_BITS || !finite_decode(bitcast<f32>(bits))) { return false; }
  if (factor == 0u) { return true; }
  let slot = factor - 1u;
  let candidate = load_candidate(header.candidate_offset + index * 8u);
  return (brain.reserved[0] & (1u << slot)) != 0u && factorized_motor_slot(candidate.kind) == slot;
}

fn training_sample(header:GpuPerceptionHeader, brain:GpuBrainSlotRecord, factor:u32) -> u32 {
  let offset = header.reserved & 0x7fffffffu;
  if (frame_payload_words[offset + 3u] == 1u) {
    if (!training_demonstrator_valid(header, brain)) { return 0xffffffffu; }
    return frame_payload_words[offset + 4u + factor];
  }
  let temperature = bitcast<f32>(frame_payload_words[offset + 2u]);
  var maximum = -3.402823e38;
  var last = 0xffffffffu;
  for (var index=0u; index<header.candidate_count; index++) {
    if (!training_candidate_allowed(header, brain, index, factor)) { continue; }
    maximum = max(maximum, load_state_f32(brain.candidate_logit_offset + index));
    last = index;
  }
  if (last == 0xffffffffu) { return last; }
  var total = 0.0;
  for (var index=0u; index<header.candidate_count; index++) {
    if (training_candidate_allowed(header, brain, index, factor)) {
      total += exp((load_state_f32(brain.candidate_logit_offset + index) - maximum) / temperature);
    }
  }
  let draw_target = training_random(frame_payload_words[offset], frame_payload_words[offset + 1u], factor) * total;
  var cumulative = 0.0;
  for (var index=0u; index<header.candidate_count; index++) {
    if (training_candidate_allowed(header, brain, index, factor)) {
      cumulative += exp((load_state_f32(brain.candidate_logit_offset + index) - maximum) / temperature);
      if (draw_target < cumulative) { return index; }
    }
  }
  return last;
}

fn training_demonstrator_valid(header:GpuPerceptionHeader, brain:GpuBrainSlotRecord) -> bool {
  let offset = header.reserved & 0x7fffffffu;
  let representative = frame_payload_words[offset + 4u];
  if (representative >= header.candidate_count) { return false; }
  if (!training_candidate_allowed(header, brain, representative, 0u)) { return false; }
  let record = load_candidate(header.candidate_offset + representative * 8u);
  var forced_slot = factorized_motor_slot(record.kind);
  if (record.kind == ACTION_KIND_IDLE || record.kind == ACTION_KIND_GESTURE) { forced_slot = 4u; }
  for (var slot=0u; slot<6u; slot++) {
    let requested = frame_payload_words[offset + 5u + slot];
    if (slot == forced_slot) {
      if (requested != representative) { return false; }
      continue;
    }
    if (requested < header.candidate_count) {
      if (!training_candidate_allowed(header, brain, requested, slot + 1u)) { return false; }
    } else {
      if (requested != 0xffffffffu) { return false; }
      for (var index=0u; index<header.candidate_count; index++) {
        if (training_candidate_allowed(header, brain, index, slot + 1u)) { return false; }
      }
    }
  }
  return true;
}
