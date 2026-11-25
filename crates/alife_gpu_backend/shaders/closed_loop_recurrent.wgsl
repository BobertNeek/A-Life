
const ACTIVE_DISPATCH_ROW_WORDS:u32 = 332u;
const MEMORY_HEADER_ROW_OFFSET:u32 = 292u;
override microstep_index:u32 = 0u;

fn is_finite(value:f32) -> bool {
  return (bitcast<u32>(value) & 0x7f800000u) != 0x7f800000u;
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

struct GpuDendriticInvocationResult {
  sum:f32, branches:u32, inputs:u32, gated:u32,
}

@compute @workgroup_size(1)
fn clear_v11_work(@builtin(global_invocation_id) gid:vec3<u32>) {
  let header = load_perception_header(gid.y * ACTIVE_DISPATCH_ROW_WORDS);
  if (!activity_contract_prevalidated(header)) { return; }
  let brain = brain_slots[header.brain_slot_index];
  if (!atomic_span_within(brain.selection_offset, GPU_SELECTION_RECORD_WORDS)) { return; }
  atomicStore(&atomic_state_words[brain.selection_offset + 12u], 0u);
  atomicStore(&atomic_state_words[brain.selection_offset + 13u], 0u);
  atomicStore(&atomic_state_words[brain.selection_offset + 14u], 0u);
  atomicStore(&atomic_state_words[brain.selection_offset + 15u], 0u);
}

var<workgroup> wg_rec_header: GpuPerceptionHeader;
var<workgroup> wg_rec_receptors: GpuNeuralReceptorEffectsRecord;
var<workgroup> wg_rec_brain: GpuBrainSlotRecord;
var<workgroup> wg_rec_weight_bases: GpuWeightBankBases;
var<workgroup> wg_rec_dendritic_plan: u32;
var<workgroup> wg_rec_route_mask_base: u32;
var<workgroup> wg_rec_route_count: u32;
var<workgroup> wg_rec_valid: u32;
var<workgroup> wg_rec_use_shared_src: u32;
var<workgroup> wg_rec_source_base: u32;
var<workgroup> wg_rec_target_base: u32;
var<workgroup> wg_rec_target_side: u32;
var<workgroup> wg_rec_route_active_mask: array<u32, 8>;
var<workgroup> wg_rec_route_delay_mask: array<u32, 8>;
var<workgroup> wg_rec_source_activations: array<f32, 2048>;
var<workgroup> wg_rec_v11_branches: array<u32, 64>;
var<workgroup> wg_rec_v11_inputs: array<u32, 64>;
var<workgroup> wg_rec_v11_gated: array<u32, 64>;
var<workgroup> wg_rec_v11_structural: array<u32, 64>;

fn apply_dendritic_branches(
  brain:GpuBrainSlotRecord,
  dendritic_plan_offset:u32,
  source_base:u32,
  target_index:u32,
  use_shared_src:bool,
) -> GpuDendriticInvocationResult {
  let branch_count = immutable_plan_words[dendritic_plan_offset];
  let target_offset_count = brain.neuron_count + 1u;
  let target_offset_base = dendritic_plan_offset + 1u;
  let descriptor_base = target_offset_base + target_offset_count;
  if (branch_count > MAX_DENDRITIC_BRANCHES_GPU
      || !plan_span_within(target_offset_base, target_offset_count)
      || !plan_span_within(descriptor_base, branch_count * DENDRITIC_BRANCH_WORDS_GPU)) {
    atomicOr(&atomic_state_words[brain.diagnostic_offset + 2u], CONTRACT_INVALID_DIAGNOSTIC_BIT);
    return GpuDendriticInvocationResult(0.0, 0u, 0u, 0u);
  }
  let begin = immutable_plan_words[target_offset_base + target_index];
  let end = immutable_plan_words[target_offset_base + target_index + 1u];
  if (begin > end || end > branch_count) {
    atomicOr(&atomic_state_words[brain.diagnostic_offset + 2u], CONTRACT_INVALID_DIAGNOSTIC_BIT);
    return GpuDendriticInvocationResult(0.0, 0u, 0u, 0u);
  }
  var dendritic_sum = 0.0;
  var branches_evaluated = 0u;
  var inputs_evaluated = 0u;
  var gated_branches = 0u;
  for (var branch_index = begin; branch_index < end; branch_index++) {
    let branch = load_dendritic_branch(descriptor_base + branch_index * DENDRITIC_BRANCH_WORDS_GPU);
    if (branch.target_neuron != target_index) {
      atomicOr(&atomic_state_words[brain.diagnostic_offset + 2u], CONTRACT_INVALID_DIAGNOSTIC_BIT);
      return GpuDendriticInvocationResult(0.0, 0u, 0u, 0u);
    }
    branches_evaluated += 1u;
    if (branch.input_count == 0u || branch.input_count > MAX_DENDRITIC_INPUTS_GPU
        || !plan_span_within(branch.input_offset, branch.input_count * DENDRITIC_INPUT_WORDS_GPU)) {
      atomicOr(&atomic_state_words[brain.diagnostic_offset + 2u], CONTRACT_INVALID_DIAGNOSTIC_BIT);
      return GpuDendriticInvocationResult(0.0, 0u, 0u, 0u);
    }
    var weighted_sum = 0.0;
    for (var input_index = 0u; input_index < branch.input_count; input_index++) {
      let input = load_dendritic_input(branch.input_offset + input_index * DENDRITIC_INPUT_WORDS_GPU);
      inputs_evaluated += 1u;
      if (input.source >= brain.neuron_count) {
        atomicOr(&atomic_state_words[brain.diagnostic_offset + 2u], CONTRACT_INVALID_DIAGNOSTIC_BIT);
        return GpuDendriticInvocationResult(0.0, 0u, 0u, 0u);
      }
      let src_val = select(
        load_state_f32(source_base + input.source),
        wg_rec_source_activations[input.source],
        use_shared_src
      );
      weighted_sum += src_val * bitcast<f32>(input.weight_bits);
    }
    let excess = max(weighted_sum - bitcast<f32>(branch.threshold_bits), 0.0);
    if (excess > 0.0) {
      gated_branches += 1u;
    }
    dendritic_sum += tanh(excess) * bitcast<f32>(branch.output_gain_bits);
  }
  return GpuDendriticInvocationResult(
    dendritic_sum,
    branches_evaluated,
    inputs_evaluated,
    gated_branches
  );
}

@compute @workgroup_size(64)
fn recurrent_microstep(
  @builtin(global_invocation_id) gid:vec3<u32>,
  @builtin(local_invocation_id) lid:vec3<u32>,
) {
  // Contract notation: brain.neuron_dynamics_offset+target*8u,
  // brain.neuron_homeostasis_offset+target*2u, and
  // brain.encoded_input_offset+target. WGSL reserves `target`, so executable
  // code below names the same target-major index `target_index`.
  let tid = lid.x;
  if (tid == 0u) {
    wg_rec_valid = 0u;
    let row_base = gid.y * ACTIVE_DISPATCH_ROW_WORDS;
    let header = load_perception_header(row_base);
    if (activity_contract_prevalidated(header)) {
      let memory = load_memory_context_header(row_base + MEMORY_HEADER_ROW_OFFSET);
      if (memory.neural_receptor_effects_offset != 0u) {
        let receptors = load_neural_receptor_effects(memory.neural_receptor_effects_offset);
        if (receptors.schema_version == 1u
            && receptors.tick_lo == header.tick_lo
            && receptors.tick_hi == header.tick_hi
            && is_finite(receptors.projection_gain)
            && is_finite(receptors.local_threshold_shift)) {
          let brain = brain_slots[header.brain_slot_index];
          if (microstep_index < brain.microstep_count && microstep_index < header.microstep_count) {
            let source_side = header.active_activation_side ^ (microstep_index & 1u);
            let target_side = source_side ^ 1u;
            wg_rec_header = header;
            wg_rec_receptors = receptors;
            wg_rec_brain = brain;
            wg_rec_weight_bases = direct_active_weight_bases(brain);
            wg_rec_dendritic_plan = load_state_u32(brain.extension_record_offset + 19u);
            wg_rec_route_mask_base = row_base + ACTIVITY_HEADER_OFFSET + 8u;
            wg_rec_route_count = load_state_u32(brain.extension_record_offset + 1u);
            wg_rec_use_shared_src = select(0u, 1u, brain.neuron_count <= 2048u);
            wg_rec_source_base = select(brain.activation_a_offset, brain.activation_b_offset, source_side == 1u);
            wg_rec_target_base = select(brain.activation_a_offset, brain.activation_b_offset, target_side == 1u);
            wg_rec_target_side = target_side;
            wg_rec_valid = 1u;
          }
        }
      }
    }
  }
  workgroupBarrier();
  if (wg_rec_valid == 0u) { return; }

  let brain = wg_rec_brain;
  let source_base = wg_rec_source_base;
  if (tid < 8u) {
    let route_base = tid * 32u;
    let enabled_word = dispatch_header_words[wg_rec_route_mask_base + tid];
    var active_word = 0u;
    var delay_bad_word = 0u;
    if (enabled_word != 0u) {
      for (var bit = 0u; bit < 32u; bit++) {
        if ((enabled_word & (1u << bit)) != 0u) {
          let route_index = route_base + bit;
          if (route_index < wg_rec_route_count) {
            let route = load_route_metadata(brain.route_metadata_offset + route_index * 12u);
            if (route.delay_microsteps != 0u) {
              delay_bad_word |= 1u << bit;
            } else if (route_fires(route.update_cadence_raw, microstep_index)) {
              active_word |= 1u << bit;
            }
          }
        }
      }
    }
    wg_rec_route_active_mask[tid] = active_word;
    wg_rec_route_delay_mask[tid] = delay_bad_word;
  }
  let use_shared_src = wg_rec_use_shared_src != 0u;
  if (use_shared_src) {
    for (var idx = tid; idx < brain.neuron_count; idx += 64u) {
      wg_rec_source_activations[idx] = load_state_f32(source_base + idx);
    }
  }
  workgroupBarrier();

  let target_index = gid.x;
  var my_branches = 0u;
  var my_inputs = 0u;
  var my_gated = 0u;
  var my_structural = 0u;
  if (target_index < brain.neuron_count) {
    let effective_base = wg_rec_weight_bases.effective;
    let route_mask_base = wg_rec_route_mask_base;
    let begin = immutable_plan_words[brain.target_offsets_offset + target_index];
    let end = immutable_plan_words[brain.target_offsets_offset + target_index + 1u];
    var recurrent_sum = 0.0;
    var delay_invalid = false;
    for (var cursor = begin; cursor < end; cursor++) {
      let route_index = immutable_plan_words[brain.route_indices_offset + cursor];
      if (route_index < 256u) {
        let word_idx = route_index >> 5u;
        let bit_mask = 1u << (route_index & 31u);
        if ((wg_rec_route_delay_mask[word_idx] & bit_mask) != 0u) {
          atomicOr(&atomic_state_words[brain.diagnostic_offset + 2u], CONTRACT_INVALID_DIAGNOSTIC_BIT);
          delay_invalid = true;
          break;
        }
        if ((wg_rec_route_active_mask[word_idx] & bit_mask) == 0u) { continue; }
      } else {
        if (!route_enabled_at(route_mask_base, route_index)) { continue; }
        let route = load_route_metadata(brain.route_metadata_offset + route_index * 12u);
        if (route.delay_microsteps != 0u) {
          atomicOr(&atomic_state_words[brain.diagnostic_offset + 2u], CONTRACT_INVALID_DIAGNOSTIC_BIT);
          delay_invalid = true;
          break;
        }
        if (!route_fires(route.update_cadence_raw, microstep_index)) { continue; }
      }
      let source = immutable_plan_words[brain.source_indices_offset + cursor];
      let alpha_bits = immutable_weight_words[brain.alpha_offset + cursor];
      if (alpha_bits == 0x80000000u) {
        my_structural += 1u;
      }
      let effective = load_state_f32(effective_base + cursor);
      let src_val = select(
        load_state_f32(source_base + source),
        wg_rec_source_activations[source],
        use_shared_src
      );
      recurrent_sum += src_val * effective;
    }
    if (!delay_invalid) {
      let dynamics = load_neuron_dynamics(brain.neuron_dynamics_offset + target_index * 8u);
      let old_activity_ema = load_state_f32(brain.neuron_homeostasis_offset + target_index * 2u);
      let metabolic_load = load_state_f32(brain.neuron_homeostasis_offset + target_index * 2u + 1u);
      let encoded = load_state_f32(brain.encoded_input_offset + target_index);
      let bias = bitcast<f32>(dynamics.bias_bits);
      let leak = bitcast<f32>(dynamics.leak_bits);
      let homeostatic_gain = bitcast<f32>(dynamics.homeostatic_gain_bits);
      let dendritic = apply_dendritic_branches(
        brain,
        wg_rec_dendritic_plan,
        source_base,
        target_index,
        use_shared_src
      );
      my_branches = dendritic.branches;
      my_inputs = dendritic.inputs;
      my_gated = dendritic.gated;
      let receptors = wg_rec_receptors;
      let pre_activation = bias + encoded
        + receptors.projection_gain * (recurrent_sum + dendritic.sum)
        - homeostatic_gain * metabolic_load
        - receptors.local_threshold_shift;
      let prior = select(
        load_state_f32(source_base + target_index),
        wg_rec_source_activations[target_index],
        use_shared_src
      );
      var output = (1.0 - leak) * prior + leak * apply_activation(pre_activation, dynamics.activation_raw);
      var activity_ema = bitcast<f32>(dynamics.activity_ema_decay_bits) * old_activity_ema
        + (1.0 - bitcast<f32>(dynamics.activity_ema_decay_bits)) * abs(output);
      var next_metabolic_load = bitcast<f32>(dynamics.metabolic_decay_bits) * metabolic_load
        + (1.0 - bitcast<f32>(dynamics.metabolic_decay_bits)) * output * output;
      if (!is_finite(output) || !is_finite(activity_ema) || !is_finite(next_metabolic_load)) {
        atomicOr(&atomic_state_words[brain.diagnostic_offset + 2u], CONTRACT_INVALID_DIAGNOSTIC_BIT);
        output = 0.0;
        activity_ema = 0.0;
        next_metabolic_load = 0.0;
      }
      store_state_f32(wg_rec_target_base + target_index, output);
      store_state_f32(brain.neuron_homeostasis_offset + target_index * 2u, clamp(activity_ema, 0.0, 1.0));
      store_state_f32(brain.neuron_homeostasis_offset + target_index * 2u + 1u, clamp(next_metabolic_load, 0.0, 1.0));
      if (target_index == 0u) {
        atomicStore(&atomic_state_words[brain.diagnostic_offset + 3u], wg_rec_target_side);
      }
    } else {
      my_structural = 0u;
    }
  }
  wg_rec_v11_branches[tid] = my_branches;
  wg_rec_v11_inputs[tid] = my_inputs;
  wg_rec_v11_gated[tid] = my_gated;
  wg_rec_v11_structural[tid] = my_structural;
  workgroupBarrier();
  for (var stride = 32u; stride > 0u; stride >>= 1u) {
    if (tid < stride) {
      let other = tid + stride;
      wg_rec_v11_branches[tid] += wg_rec_v11_branches[other];
      wg_rec_v11_inputs[tid] += wg_rec_v11_inputs[other];
      wg_rec_v11_gated[tid] += wg_rec_v11_gated[other];
      wg_rec_v11_structural[tid] += wg_rec_v11_structural[other];
    }
    workgroupBarrier();
  }
  if (tid == 0u) {
    if (wg_rec_v11_branches[0] != 0u) {
      atomicAdd(&atomic_state_words[brain.selection_offset + 12u], wg_rec_v11_branches[0]);
    }
    if (wg_rec_v11_inputs[0] != 0u) {
      atomicAdd(&atomic_state_words[brain.selection_offset + 13u], wg_rec_v11_inputs[0]);
    }
    if (wg_rec_v11_gated[0] != 0u) {
      atomicAdd(&atomic_state_words[brain.selection_offset + 14u], wg_rec_v11_gated[0]);
    }
    if (wg_rec_v11_structural[0] != 0u) {
      atomicAdd(&atomic_state_words[brain.selection_offset + 15u], wg_rec_v11_structural[0]);
    }
  }
}
