
const ACTIVE_DISPATCH_ROW_WORDS:u32 = 272u;
override microstep_index:u32 = 0u;

fn is_finite(value:f32) -> bool {
  return value == value && abs(value) <= 3.402823466e+38;
}

fn route_fires(cadence_raw:u32, step:u32) -> bool {
  switch cadence_raw {
    case 0u: { return true; }
    case 1u, 2u: { return step % 2u == 0u; }
    case 3u, 4u: { return step == 0u; }
    case 5u, 6u: { return false; }
    default: { return false; }
  }
}

fn apply_activation(value:f32, activation_raw:u32) -> f32 {
  switch activation_raw {
    case 0u: { return value; }
    case 1u: { return max(value, 0.0); }
    case 2u: { return tanh(value); }
    case 3u: { return 1.0 / (1.0 + exp(-value)); }
    default: { return 0.0; }
  }
}

@compute @workgroup_size(64)
fn recurrent_microstep(@builtin(global_invocation_id) gid:vec3<u32>) {
  // Contract notation: brain.neuron_dynamics_offset+target*8u,
  // brain.neuron_homeostasis_offset+target*2u, and
  // brain.encoded_input_offset+target. WGSL reserves `target`, so executable
  // code below names the same target-major index `target_index`.
  let header = load_perception_header(gid.y * ACTIVE_DISPATCH_ROW_WORDS);
  if (!validate_slice_a_slot(header.brain_slot_index, header)) { return; }
  let brain = brain_slots[header.brain_slot_index];
  if (microstep_index >= brain.microstep_count || microstep_index >= header.microstep_count) { return; }
  let target_index = gid.x;
  if (target_index >= brain.neuron_count) { return; }

  let source_side = header.active_activation_side ^ (microstep_index & 1u);
  let target_side = source_side ^ 1u;
  let source_base = select(brain.activation_a_offset, brain.activation_b_offset, source_side == 1u);
  let target_base = select(brain.activation_a_offset, brain.activation_b_offset, target_side == 1u);
  let begin = immutable_plan_words[brain.target_offsets_offset + target_index];
  let end = immutable_plan_words[brain.target_offsets_offset + target_index + 1u];
  var recurrent_sum = 0.0;
  var active_rows = 0u;
  for (var cursor = begin; cursor < end; cursor++) {
    let source = immutable_plan_words[brain.source_indices_offset + cursor];
    let route_index = immutable_plan_words[brain.route_indices_offset + cursor];
    let route = load_route_metadata(brain.route_metadata_offset + route_index * 12u);
    if (route.delay_microsteps != 0u) {
      atomicAdd(&mutable_state_words[brain.diagnostic_offset + 2u], 1u);
      continue;
    }
    if (!route_fires(route.update_cadence_raw, microstep_index)) { continue; }
    active_rows += 1u;
    let genetic = bitcast<f32>(immutable_weight_words[brain.genetic_weight_offset + cursor]);
    let alpha = bitcast<f32>(immutable_weight_words[brain.alpha_offset + cursor]);
    let lifetime = load_state_f32(brain.lifetime_weight_offset + cursor);
    let fast = load_state_f32(brain.fast_weight_offset + cursor);
    let effective = genetic + lifetime + alpha * fast;
    recurrent_sum += load_state_f32(source_base + source) * effective;
  }
  if (active_rows != 0u) {
    atomicAdd(&mutable_state_words[brain.diagnostic_offset + 1u], active_rows);
  }

  if (target_index == 0u) {
    let projection_count = (brain.route_metadata_offset - brain.projection_offset) / 8u;
    var active_tiles = 0u;
    for (var route_cursor = 0u; route_cursor < projection_count; route_cursor++) {
      let projection = load_projection(brain.projection_offset + route_cursor * 8u);
      let route = load_route_metadata(brain.route_metadata_offset + projection.route_index * 12u);
      if (route.delay_microsteps != 0u) {
        atomicAdd(&mutable_state_words[brain.diagnostic_offset + 2u], 1u);
      } else if (route_fires(route.update_cadence_raw, microstep_index)) {
        active_tiles += projection.active_tile_count;
      }
    }
    atomicAdd(&mutable_state_words[brain.diagnostic_offset], active_tiles);
  }

  let dynamics = load_neuron_dynamics(brain.neuron_dynamics_offset + target_index * 8u);
  let old_activity_ema = load_state_f32(brain.neuron_homeostasis_offset + target_index * 2u);
  let metabolic_load = load_state_f32(brain.neuron_homeostasis_offset + target_index * 2u + 1u);
  let encoded = load_state_f32(brain.encoded_input_offset + target_index);
  let bias = bitcast<f32>(dynamics.bias_bits);
  let leak = bitcast<f32>(dynamics.leak_bits);
  let homeostatic_gain = bitcast<f32>(dynamics.homeostatic_gain_bits);
  let pre_activation = bias + encoded + recurrent_sum - homeostatic_gain * metabolic_load;
  let prior = load_state_f32(source_base + target_index);
  var output = (1.0 - leak) * prior + leak * apply_activation(pre_activation, dynamics.activation_raw);
  var activity_ema = bitcast<f32>(dynamics.activity_ema_decay_bits) * old_activity_ema
    + (1.0 - bitcast<f32>(dynamics.activity_ema_decay_bits)) * abs(output);
  var next_metabolic_load = bitcast<f32>(dynamics.metabolic_decay_bits) * metabolic_load
    + (1.0 - bitcast<f32>(dynamics.metabolic_decay_bits)) * output * output;
  if (!is_finite(output) || !is_finite(activity_ema) || !is_finite(next_metabolic_load)) {
    atomicAdd(&mutable_state_words[brain.diagnostic_offset + 2u], 1u);
    output = 0.0;
    activity_ema = 0.0;
    next_metabolic_load = 0.0;
  }
  store_state_f32(target_base + target_index, output);
  store_state_f32(brain.neuron_homeostasis_offset + target_index * 2u, clamp(activity_ema, 0.0, 1.0));
  store_state_f32(brain.neuron_homeostasis_offset + target_index * 2u + 1u, clamp(next_metabolic_load, 0.0, 1.0));
  if (target_index == 0u) {
    atomicStore(&mutable_state_words[brain.diagnostic_offset + 3u], target_side);
  }
}
