
const ACTIVE_DISPATCH_ROW_WORDS:u32 = 332u;
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

const MAX_DENDRITIC_BRANCHES_GPU:u32 = 4096u;
const MAX_DENDRITIC_INPUTS_GPU:u32 = 32u;
const DENDRITIC_BRANCH_WORDS_GPU:u32 = 8u;
const DENDRITIC_INPUT_WORDS_GPU:u32 = 4u;

@compute @workgroup_size(1)
fn clear_v11_work(@builtin(global_invocation_id) gid:vec3<u32>) {
  let header = load_perception_header(gid.y * ACTIVE_DISPATCH_ROW_WORDS);
  if (!activity_contract_prevalidated(header)) { return; }
  let brain = brain_slots[header.brain_slot_index];
  if (!state_span_within(brain.selection_offset, GPU_SELECTION_RECORD_WORDS)) { return; }
  atomicStore(&mutable_state_words[brain.selection_offset + 12u], 0u);
  atomicStore(&mutable_state_words[brain.selection_offset + 13u], 0u);
  atomicStore(&mutable_state_words[brain.selection_offset + 14u], 0u);
  atomicStore(&mutable_state_words[brain.selection_offset + 15u], 0u);
}

fn apply_dendritic_branches(
  brain:GpuBrainSlotRecord,
  extension:GpuBrainSlotExtensionRecord,
  source_base:u32,
  target_index:u32,
) -> f32 {
  let branch_count = immutable_plan_words[extension.reserved1];
  let target_offset_count = brain.neuron_count + 1u;
  let target_offset_base = extension.reserved1 + 1u;
  let descriptor_base = target_offset_base + target_offset_count;
  if (branch_count > MAX_DENDRITIC_BRANCHES_GPU
      || !plan_span_within(target_offset_base, target_offset_count)
      || !plan_span_within(descriptor_base, branch_count * DENDRITIC_BRANCH_WORDS_GPU)) {
    atomicAdd(&mutable_state_words[brain.diagnostic_offset + 2u], 1u);
    return 0.0;
  }
  let begin = immutable_plan_words[target_offset_base + target_index];
  let end = immutable_plan_words[target_offset_base + target_index + 1u];
  if (begin > end || end > branch_count) {
    atomicAdd(&mutable_state_words[brain.diagnostic_offset + 2u], 1u);
    return 0.0;
  }
  var dendritic_sum = 0.0;
  for (var branch_index = begin; branch_index < end; branch_index++) {
    let branch = load_dendritic_branch(descriptor_base + branch_index * DENDRITIC_BRANCH_WORDS_GPU);
    if (branch.target_neuron != target_index) {
      atomicAdd(&mutable_state_words[brain.diagnostic_offset + 2u], 1u);
      continue;
    }
    atomicAdd(&mutable_state_words[brain.selection_offset + 12u], 1u);
    if (branch.input_count == 0u || branch.input_count > MAX_DENDRITIC_INPUTS_GPU
        || !plan_span_within(branch.input_offset, branch.input_count * DENDRITIC_INPUT_WORDS_GPU)) {
      atomicAdd(&mutable_state_words[brain.diagnostic_offset + 2u], 1u);
      continue;
    }
    var weighted_sum = 0.0;
    for (var input_index = 0u; input_index < branch.input_count; input_index++) {
      let input = load_dendritic_input(branch.input_offset + input_index * DENDRITIC_INPUT_WORDS_GPU);
      atomicAdd(&mutable_state_words[brain.selection_offset + 13u], 1u);
      if (input.source >= brain.neuron_count) {
        atomicAdd(&mutable_state_words[brain.diagnostic_offset + 2u], 1u);
        continue;
      }
      weighted_sum += load_state_f32(source_base + input.source) * bitcast<f32>(input.weight_bits);
    }
    let excess = max(weighted_sum - bitcast<f32>(branch.threshold_bits), 0.0);
    if (excess > 0.0) {
      atomicAdd(&mutable_state_words[brain.selection_offset + 14u], 1u);
    }
    dendritic_sum += tanh(excess) * bitcast<f32>(branch.output_gain_bits);
  }
  return dendritic_sum;
}

@compute @workgroup_size(64)
fn recurrent_microstep(@builtin(global_invocation_id) gid:vec3<u32>) {
  // Contract notation: brain.neuron_dynamics_offset+target*8u,
  // brain.neuron_homeostasis_offset+target*2u, and
  // brain.encoded_input_offset+target. WGSL reserves `target`, so executable
  // code below names the same target-major index `target_index`.
  let header = load_perception_header(gid.y * ACTIVE_DISPATCH_ROW_WORDS);
  if (!activity_contract_prevalidated(header)) { return; }
  let route_mask_base = gid.y * ACTIVE_DISPATCH_ROW_WORDS + ACTIVITY_HEADER_OFFSET + 8u;
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
  for (var cursor = begin; cursor < end; cursor++) {
    let source = immutable_plan_words[brain.source_indices_offset + cursor];
    let route_index = immutable_plan_words[brain.route_indices_offset + cursor];
    if (!route_enabled_at(route_mask_base, route_index)) { continue; }
    let route = load_route_metadata(brain.route_metadata_offset + route_index * 12u);
    if (route.delay_microsteps != 0u) {
      atomicAdd(&mutable_state_words[brain.diagnostic_offset + 2u], 1u);
      continue;
    }
    if (!route_fires(route.update_cadence_raw, microstep_index)) { continue; }
    let alpha_bits = immutable_weight_words[brain.alpha_offset + cursor];
    if (alpha_bits == 0x80000000u) {
      atomicAdd(&mutable_state_words[brain.selection_offset + 15u], 1u);
    }
    let genetic = bitcast<f32>(immutable_weight_words[brain.genetic_weight_offset + cursor]);
    let alpha = bitcast<f32>(alpha_bits);
    let lifetime = load_state_f32(weight_bases.lifetime + cursor);
    let fast = load_state_f32(weight_bases.fast + cursor);
    let effective = genetic + lifetime + alpha * fast;
    recurrent_sum += load_state_f32(source_base + source) * effective;
  }

  let dynamics = load_neuron_dynamics(brain.neuron_dynamics_offset + target_index * 8u);
  let old_activity_ema = load_state_f32(brain.neuron_homeostasis_offset + target_index * 2u);
  let metabolic_load = load_state_f32(brain.neuron_homeostasis_offset + target_index * 2u + 1u);
  let encoded = load_state_f32(brain.encoded_input_offset + target_index);
  let bias = bitcast<f32>(dynamics.bias_bits);
  let leak = bitcast<f32>(dynamics.leak_bits);
  let homeostatic_gain = bitcast<f32>(dynamics.homeostatic_gain_bits);
  let dendritic_sum = apply_dendritic_branches(brain, extension, source_base, target_index);
  let pre_activation = bias + encoded + recurrent_sum + dendritic_sum - homeostatic_gain * metabolic_load;
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
