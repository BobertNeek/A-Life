
const ACTIVE_DISPATCH_ROW_WORDS:u32 = 332u;
const ACTIVITY_HEADER_OFFSET:u32 = 308u;
override microstep_index:u32 = 0u;

struct GpuActivityDispatchHeader {
  schema_version:u32, policy_version:u32, class_id:u32, slot:u32,
  slot_generation:u32, brain_slot_index:u32, microsteps:u32, enabled_route_count:u32,
  enabled_route_mask:array<u32,8>, route_schedule_digest:array<u32,8>,
}

fn load_activity_header(base:u32) -> GpuActivityDispatchHeader {
  return GpuActivityDispatchHeader(
    dispatch_header_words[base], dispatch_header_words[base+1u], dispatch_header_words[base+2u], dispatch_header_words[base+3u],
    dispatch_header_words[base+4u], dispatch_header_words[base+5u], dispatch_header_words[base+6u], dispatch_header_words[base+7u],
    array<u32,8>(
      dispatch_header_words[base+8u], dispatch_header_words[base+9u], dispatch_header_words[base+10u], dispatch_header_words[base+11u],
      dispatch_header_words[base+12u], dispatch_header_words[base+13u], dispatch_header_words[base+14u], dispatch_header_words[base+15u]
    ),
    array<u32,8>(
      dispatch_header_words[base+16u], dispatch_header_words[base+17u], dispatch_header_words[base+18u], dispatch_header_words[base+19u],
      dispatch_header_words[base+20u], dispatch_header_words[base+21u], dispatch_header_words[base+22u], dispatch_header_words[base+23u]
    )
  );
}

fn route_enabled(activity:GpuActivityDispatchHeader, route_index:u32) -> bool {
  if (route_index >= 256u) { return false; }
  return (activity.enabled_route_mask[route_index / 32u] & (1u << (route_index % 32u))) != 0u;
}

fn mix_route_digest_word(input:array<u32,8>, word:u32) -> array<u32,8> {
  let salts = array<u32,8>(
    0x00000000u, 0x85ebca6bu, 0xc2b2ae35u, 0x27d4eb2fu,
    0x165667b1u, 0xd3a2646cu, 0xfd7046c5u, 0xb55a4f09u
  );
  var output = input;
  for (var lane = 0u; lane < 8u; lane++) {
    output[lane] = ((output[lane] + salts[lane]) ^ word) * 16777619u;
  }
  return output;
}

fn compute_route_schedule_digest(activity:GpuActivityDispatchHeader) -> array<u32,8> {
  var digest = array<u32,8>(
    0x811c9dc5u, 0x9e3779b9u, 0x243f6a88u, 0xb7e15163u,
    0xa4093822u, 0x299f31d0u, 0x082efa98u, 0xec4e6c89u
  );
  let phenotype = phenotype_identities[activity.brain_slot_index];
  for (var word = 0u; word < 8u; word++) {
    digest = mix_route_digest_word(digest, phenotype.phenotype_hash[word]);
  }
  digest = mix_route_digest_word(digest, activity.microsteps);
  digest = mix_route_digest_word(digest, activity.enabled_route_count);
  for (var route_index = 0u; route_index < 256u; route_index++) {
    if (route_enabled(activity, route_index)) {
      digest = mix_route_digest_word(digest, route_index);
    }
  }
  return digest;
}

fn validate_activity_header(activity:GpuActivityDispatchHeader, header:GpuPerceptionHeader) -> bool {
  var enabled_count = 0u;
  for (var word = 0u; word < 8u; word++) {
    enabled_count += countOneBits(activity.enabled_route_mask[word]);
  }
  let expected_digest = compute_route_schedule_digest(activity);
  var digest_matches = true;
  for (var word = 0u; word < 8u; word++) {
    digest_matches = digest_matches && activity.route_schedule_digest[word] == expected_digest[word];
  }
  return activity.schema_version != 0u
    && activity.policy_version != 0u
    && activity.class_id == header.class_id
    && activity.slot == header.slot
    && activity.slot_generation == header.slot_generation
    && activity.brain_slot_index == header.brain_slot_index
    && activity.microsteps == header.microstep_count
    && activity.enabled_route_count == enabled_count
    && enabled_count != 0u
    && digest_matches;
}

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
  let activity = load_activity_header(gid.y * ACTIVE_DISPATCH_ROW_WORDS + ACTIVITY_HEADER_OFFSET);
  if (!validate_activity_header(activity, header)) { return; }
  let brain = brain_slots[header.brain_slot_index];
  let extension = load_slot_extension(brain);
  let learning = load_slot_learning_state(extension);
  let weight_bases = active_weight_bases(brain, extension, learning);
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
    if (!route_enabled(activity, route_index)) { continue; }
    let route = load_route_metadata(brain.route_metadata_offset + route_index * 12u);
    if (route.delay_microsteps != 0u) {
      atomicAdd(&mutable_state_words[brain.diagnostic_offset + 2u], 1u);
      continue;
    }
    if (!route_fires(route.update_cadence_raw, microstep_index)) { continue; }
    active_rows += 1u;
    let genetic = bitcast<f32>(immutable_weight_words[brain.genetic_weight_offset + cursor]);
    let alpha = bitcast<f32>(immutable_weight_words[brain.alpha_offset + cursor]);
    let lifetime = load_state_f32(weight_bases.lifetime + cursor);
    let fast = load_state_f32(weight_bases.fast + cursor);
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
      if (!route_enabled(activity, projection.route_index)) { continue; }
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
