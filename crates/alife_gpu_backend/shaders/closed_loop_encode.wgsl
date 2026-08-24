
const ACTIVE_DISPATCH_ROW_WORDS:u32 = 332u;
const MEMORY_HEADER_ROW_OFFSET:u32 = 292u;
const INVALID_LANE:u32 = 0xffffffffu;

fn finite_receptor_value(value:f32) -> bool {
  return value == value && abs(value) <= 3.402823466e+38;
}

fn resolve_encoder_source_lane(encoder:GpuEncoderPlanRecord, assignment:GpuEncoderAssignmentRecord) -> u32 {
  let source_group_raw = assignment.source_group_raw;
  let source_index = assignment.source_index;
  if (source_group_raw == 1u && source_index < encoder.sensory_lane_count) {
    return source_index;
  }
  if (source_group_raw == 2u && source_index < encoder.body_lane_count) {
    return encoder.sensory_lane_count + source_index;
  }
  if (source_group_raw == 3u && source_index < encoder.homeostasis_lane_count) {
    return encoder.sensory_lane_count + encoder.body_lane_count + source_index;
  }
  return INVALID_LANE;
}

@compute @workgroup_size(64)
fn encode_perception(@builtin(global_invocation_id) gid:vec3<u32>) {
  let row_base = gid.y * ACTIVE_DISPATCH_ROW_WORDS;
  let header = load_perception_header(row_base);
  if (!activity_contract_prevalidated(header)) { return; }
  let memory = load_memory_context_header(row_base + MEMORY_HEADER_ROW_OFFSET);
  if (memory.neural_receptor_effects_offset == 0u) { return; }
  let receptors = load_neural_receptor_effects(memory.neural_receptor_effects_offset);
  if (receptors.schema_version != 1u
      || receptors.source_chemistry_version == 0u
      || receptors.tick_lo != header.tick_lo
      || receptors.tick_hi != header.tick_hi
      || any(receptors.reserved != vec2<u32>(0u))
      || !finite_receptor_value(receptors.interoceptive_gain)
      || !finite_receptor_value(receptors.regional_excitability)
      || !finite_receptor_value(receptors.attention_gain)
      || !finite_receptor_value(receptors.structural_growth_gate)
      || !finite_receptor_value(receptors.sleep_gate)
      || !finite_receptor_value(receptors.consolidation_gate)) { return; }
  let brain = brain_slots[header.brain_slot_index];
  if (brain.slot_generation != header.slot_generation) { return; }
  let index = gid.x;
  if (index >= brain.neuron_count) { return; }

  let encoder = load_encoder_plan(brain.encoder_plan_offset);
  let begin = immutable_plan_words[encoder.target_offsets_offset + index];
  let end = immutable_plan_words[encoder.target_offsets_offset + index + 1u];
  if (begin > end || end > encoder.assignment_count) {
    atomicAdd(&mutable_state_words[brain.diagnostic_offset + 2u], 1u);
    store_state_f32(brain.encoded_input_offset + index, 0.0);
    return;
  }
  var value = 0.0;
  for (var cursor = begin; cursor < end; cursor++) {
    let assignment = load_encoder_assignment(encoder.assignment_offset + cursor * 8u);
    if (assignment.target_neuron != index) {
      atomicAdd(&mutable_state_words[brain.diagnostic_offset + 2u], 1u);
      return;
    }
    let source_lane = resolve_encoder_source_lane(encoder, assignment);
    if (source_lane == INVALID_LANE) {
      atomicAdd(&mutable_state_words[brain.diagnostic_offset + 2u], 1u);
      continue;
    }
    var source = bitcast<f32>(frame_payload_words[header.sensory_offset + source_lane]);
    if (assignment.source_group_raw == 1u) {
      source *= receptors.attention_gain;
    } else if (assignment.source_group_raw == 3u) {
      source *= receptors.interoceptive_gain;
    }
    value += clamp(
      source * bitcast<f32>(assignment.scale_bits) + bitcast<f32>(assignment.bias_bits),
      bitcast<f32>(assignment.clamp_min_bits),
      bitcast<f32>(assignment.clamp_max_bits)
    );
  }
  let state_gate = (receptors.structural_growth_gate + receptors.sleep_gate
    + receptors.consolidation_gate) / 3.0;
  value = value * receptors.regional_excitability + (state_gate - 0.5) * 0.05;
  store_state_f32(brain.encoded_input_offset + index, clamp(value, -1.0, 1.0));
}
