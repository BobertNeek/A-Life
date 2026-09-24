// Offline-only exact sparse-graph truncated BPTT and AdamW.

struct TrainingHeader {
  words: array<vec4<u32>, 16>,
}

@group(0) @binding(0) var<uniform> header: TrainingHeader;
@group(0) @binding(1) var<storage, read> meta_words: array<u32>;
@group(0) @binding(2) var<storage, read_write> weight_words: array<u32>;
@group(0) @binding(3) var<storage, read_write> optimizer_words: array<u32>;
@group(0) @binding(4) var<storage, read> training_words: array<u32>;
@group(0) @binding(5) var<storage, read_write> state_words: array<u32>;
@group(0) @binding(6) var<storage, read_write> gradient_words: array<u32>;
@group(0) @binding(7) var<storage, read_write> output_words: array<u32>;
@group(0) @binding(8) var<storage, read> trainable_mask: array<u32>;

const SYNAPSE_STRIDE:u32 = 8u;
const DYNAMICS_STRIDE:u32 = 8u;
const CANDIDATE_RECORD_WORDS:u32 = 64u;
const CANDIDATE_FEATURE_COUNT:u32 = 24u;
const SYNAPSE_RECURRENT:u32 = 1u;
const SYNAPSE_DECODER:u32 = 2u;
const DECODER_ACTION_CANDIDATE:u32 = 1u;
const DECODER_SPEECH_PAYLOAD:u32 = 3u;

fn h(index:u32) -> u32 {
  return header.words[index / 4u][index % 4u];
}

fn load_training_f32(index:u32) -> f32 {
  return bitcast<f32>(training_words[index]);
}

fn load_weight_f32(index:u32) -> f32 {
  return bitcast<f32>(weight_words[index]);
}

fn store_weight_f32(index:u32, value:f32) {
  weight_words[index] = bitcast<u32>(value);
}

fn load_optimizer_f32(index:u32) -> f32 {
  return bitcast<f32>(optimizer_words[index]);
}

fn store_optimizer_f32(index:u32, value:f32) {
  optimizer_words[index] = bitcast<u32>(value);
}

fn load_state_f32(index:u32) -> f32 {
  return bitcast<f32>(state_words[index]);
}

fn store_state_f32(index:u32, value:f32) {
  state_words[index] = bitcast<u32>(value);
}

fn load_gradient_f32(index:u32) -> f32 {
  return bitcast<f32>(gradient_words[index]);
}

fn store_gradient_f32(index:u32, value:f32) {
  gradient_words[index] = bitcast<u32>(value);
}

fn load_output_f32(index:u32) -> f32 {
  return bitcast<f32>(output_words[index]);
}

fn store_output_f32(index:u32, value:f32) {
  output_words[index] = bitcast<u32>(value);
}

fn finite(value:f32) -> bool {
  return value == value && abs(value) <= 3.402823466e+38;
}

fn route_fires(cadence_raw:u32, step:u32) -> bool {
  switch cadence_raw {
    case 0u: { return true; }
    case 1u, 2u: { return step % 2u == 0u; }
    case 3u, 4u: { return step == 0u; }
    default: { return false; }
  }
}

fn activation(value:f32, kind:u32) -> f32 {
  switch kind {
    case 0u: { return value; }
    case 1u: { return max(value, 0.0); }
    case 2u: { return tanh(value); }
    case 3u: { return 1.0 / (1.0 + exp(-value)); }
    default: { return 0.0; }
  }
}

fn activation_derivative(output:f32, kind:u32) -> f32 {
  switch kind {
    case 0u: { return 1.0; }
    case 1u: { return select(0.0, 1.0, output > 0.0); }
    case 2u: { return max(0.0, 1.0 - output * output); }
    case 3u: { return output * (1.0 - output); }
    default: { return 0.0; }
  }
}

fn synapse_word(synapse:u32, field:u32) -> u32 {
  let value = meta_words[h(16u) + synapse * SYNAPSE_STRIDE + field];
  return select(value, value & 255u, field == 2u);
}

fn candidate_word(tick:u32, field:u32) -> u32 { return candidate_field(tick, 0u, field); }
fn candidate_field(tick:u32, candidate:u32, field:u32) -> u32 {
  return training_words[h(21u) + (tick * h(51u) + candidate) * CANDIDATE_RECORD_WORDS + field];
}
fn candidate_input(tick:u32, candidate:u32, lane:u32) -> f32 {
  let field = select(32u + lane - 24u, 4u + lane, lane < 24u);
  return bitcast<f32>(candidate_field(tick, candidate, field));
}
fn context_base(tick:u32) -> u32 { return h(42u) + tick * (4u + h(43u) + h(2u) + 64u * 5u); }
fn structural_edge_base(tick:u32) -> u32 { return context_base(tick) + 4u + h(43u) + h(2u); }
fn structural_edge_count(tick:u32) -> u32 { return training_words[context_base(tick) + 3u]; }
fn projection_gain(tick:u32) -> f32 {
  if (h(41u) == 0u) { return 1.0; }
  return load_training_f32(context_base(tick));
}
fn microstep_active(tick:u32, step:u32) -> bool {
  if (h(41u) == 0u) { return true; }
  return step < training_words[context_base(tick) + 2u];
}
fn route_enabled(tick:u32, synapse:u32) -> bool {
  if (h(41u) == 0u) { return true; }
  let route = meta_words[h(16u) + synapse * SYNAPSE_STRIDE + 2u] >> 8u;
  return training_words[context_base(tick) + 4u + route] != 0u;
}
fn effective_weight(tick:u32, synapse:u32) -> f32 {
  if (h(41u) == 0u) { return load_weight_f32(synapse); }
  return load_weight_f32(synapse) + load_training_f32(context_base(tick) + 4u + h(43u) + synapse);
}
fn dendritic_excess(branch:u32, previous_state:u32) -> f32 {
  let base = h(45u) + branch * 5u;
  let start = meta_words[base + 3u];
  let count = meta_words[base + 4u];
  var sum = 0.0;
  for (var i = 0u; i < count; i++) {
    let input = h(46u) + (start + i) * 2u;
    sum += load_state_f32(h(22u) + previous_state + meta_words[input]) * bitcast<f32>(meta_words[input + 1u]);
  }
  return max(sum - bitcast<f32>(meta_words[base + 1u]), 0.0);
}
fn candidate_adjoint(tick:u32, candidate:u32) -> f32 {
  if (h(52u) != 0u) {
    return load_gradient_f32(h(53u) + tick * h(51u) + candidate);
  }
  let expected = bitcast<f32>(candidate_field(tick, candidate, 2u));
  let weight = bitcast<f32>(candidate_field(tick, candidate, 3u));
  return 2.0 * weight * (load_output_f32(h(28u) + tick * h(51u) + candidate) - expected) / loss_denominator();
}
fn auxiliary_gate(tick:u32, candidate:u32, head:u32) -> f32 {
  let offset = select(h(58u), h(59u), head == 4u);
  let raw = load_output_f32(offset + tick * h(51u) + candidate);
  let limit = bitcast<f32>(h(56u));
  return select(0.0, 1.0, raw > -limit && raw < limit);
}
@compute @workgroup_size(64)
fn initialize_replay_state(@builtin(global_invocation_id) gid:vec3<u32>) {
  let n = gid.x;
  if (n >= h(1u) || h(41u) == 0u) { return; }
  store_state_f32(h(22u) + n, load_training_f32(h(49u) + n));
  store_state_f32(h(23u) + n, load_training_f32(h(49u) + h(1u) + n));
  store_state_f32(h(50u) + n, load_training_f32(h(49u) + 2u * h(1u) + n));
}

fn loss_denominator() -> f32 {
  return max(load_output_f32(h(29u) + 1u), 1.0);
}

@compute @workgroup_size(64)
fn forward_microstep(@builtin(global_invocation_id) gid:vec3<u32>) {
  let neuron = gid.x;
  let neuron_count = h(1u);
  if (neuron >= neuron_count) { return; }
  let step = h(7u);
  let local_step = h(8u);
  let tick = h(9u);
  let previous_state = step * neuron_count;
  let next_state = (step + 1u) * neuron_count;
  if (!microstep_active(tick, local_step)) {
    store_state_f32(h(22u) + next_state + neuron, load_state_f32(h(22u) + previous_state + neuron));
    store_state_f32(h(23u) + next_state + neuron, load_state_f32(h(23u) + previous_state + neuron));
    store_state_f32(h(50u) + next_state + neuron, load_state_f32(h(50u) + previous_state + neuron));
    return;
  }
  let begin = meta_words[h(12u) + neuron];
  let end = meta_words[h(12u) + neuron + 1u];
  var recurrent_sum = 0.0;
  for (var cursor = begin; cursor < end; cursor++) {
    let synapse = meta_words[h(13u) + cursor];
    if (synapse_word(synapse, 2u) != SYNAPSE_RECURRENT
        || !route_enabled(tick, synapse)
        || !route_fires(synapse_word(synapse, 3u), local_step)) { continue; }
    let source = synapse_word(synapse, 0u);
    recurrent_sum += load_state_f32(h(22u) + previous_state + source)
      * effective_weight(tick, synapse);
  }
  if (h(41u) != 0u) {
    for (var edge = 0u; edge < structural_edge_count(tick); edge++) {
      let at = structural_edge_base(tick) + edge * 5u;
      if (training_words[at + 1u] != neuron) { continue; }
      let route = training_words[at + 2u];
      if (training_words[context_base(tick) + 4u + route] == 0u
          || !route_fires(training_words[at + 3u], local_step)) { continue; }
      recurrent_sum += load_state_f32(h(22u) + previous_state + training_words[at])
        * bitcast<f32>(training_words[at + 4u]);
    }
  }
  var dendritic_sum = 0.0;
  for (var branch = meta_words[h(44u) + neuron]; branch < meta_words[h(44u) + neuron + 1u]; branch++) {
    dendritic_sum += tanh(dendritic_excess(branch, previous_state)) * bitcast<f32>(meta_words[h(45u) + branch * 5u + 2u]);
  }
  let dynamics = h(17u) + neuron * DYNAMICS_STRIDE;
  let bias = bitcast<f32>(meta_words[dynamics]);
  let leak = bitcast<f32>(meta_words[dynamics + 1u]);
  let activation_kind = meta_words[dynamics + 2u];
  let homeostatic_gain = bitcast<f32>(meta_words[dynamics + 3u]);
  let metabolic_decay = bitcast<f32>(meta_words[dynamics + 4u]);
  let prior = load_state_f32(h(22u) + previous_state + neuron);
  let metabolic = load_state_f32(h(23u) + previous_state + neuron);
  let encoded = load_training_f32(h(18u) + tick * neuron_count + neuron);
  var threshold = 0.0;
  if (h(41u) != 0u) { threshold = load_training_f32(context_base(tick) + 1u); }
  let activated = activation(bias + encoded + projection_gain(tick) * (recurrent_sum + dendritic_sum)
    - homeostatic_gain * metabolic - threshold, activation_kind);
  var output = (1.0 - leak) * prior + leak * activated;
  var next_metabolic = clamp(
    metabolic_decay * metabolic + (1.0 - metabolic_decay) * output * output,
    0.0,
    1.0
  );
  store_state_f32(h(22u) + next_state + neuron, output);
  store_state_f32(h(23u) + next_state + neuron, next_metabolic);
  let ema_decay = bitcast<f32>(meta_words[dynamics + 5u]);
  let ema = ema_decay * load_state_f32(h(50u) + previous_state + neuron) + (1.0 - ema_decay) * abs(output);
  store_state_f32(h(50u) + next_state + neuron, clamp(ema, 0.0, 1.0));
}

@compute @workgroup_size(32)
fn forward_candidate_logits(@builtin(global_invocation_id) gid:vec3<u32>) {
  let tick = gid.y;
  let candidate = gid.x;
  if (tick >= h(5u) || candidate >= h(51u)) { return; }
  let output_index = tick * h(51u) + candidate;
  if (candidate_field(tick, candidate, 0u) == 0u) { return; }
  let family = candidate_field(tick, candidate, 1u);
  let final_state = (tick + 1u) * h(4u) * h(1u);
  var logit = bitcast<f32>(meta_words[h(38u) + family]);
  var memory = 0.0;
  var cognitive = 0.0;
  let begin = meta_words[h(54u) + family];
  let end = meta_words[h(54u) + family + 1u];
  for (var cursor = begin; cursor < end; cursor++) {
    let synapse = meta_words[h(55u) + cursor];
    let head = synapse_word(synapse, 4u);
    let lane = synapse_word(synapse, 6u);
    let feature = candidate_input(tick, candidate, lane);
    let weighted = feature * effective_weight(tick, synapse);
    if (head == 1u) {
      logit += load_state_f32(h(22u) + final_state + synapse_word(synapse, 0u)) * weighted;
    } else if (h(41u) != 0u && head == 2u) { memory += weighted;
    } else if (h(41u) != 0u && head == 4u) { cognitive += weighted; }
  }
  let limit = bitcast<f32>(h(56u));
  store_output_f32(h(58u) + output_index, memory);
  store_output_f32(h(59u) + output_index, cognitive);
  store_output_f32(h(28u) + output_index, logit + clamp(memory, -limit, limit) + clamp(cognitive, -limit, limit));
}

@compute @workgroup_size(1)
fn forward_speech_logits(@builtin(global_invocation_id) gid:vec3<u32>) {
  let tick = gid.x;
  if (tick >= h(5u)) { return; }
  if (candidate_word(tick, 28u) == 0u) {
    store_output_f32(h(40u) + tick, 0.0);
    return;
  }
  let output_index = candidate_word(tick, 29u);
  let target_neuron = h(37u) + output_index;
  let final_state = (tick + 1u) * h(4u) * h(1u);
  var logit = 0.0;
  for (var synapse = 0u; synapse < h(2u); synapse++) {
    if (synapse_word(synapse, 2u) != SYNAPSE_DECODER
        || synapse_word(synapse, 4u) != DECODER_SPEECH_PAYLOAD
        || synapse_word(synapse, 1u) != target_neuron) { continue; }
    let source = synapse_word(synapse, 0u);
    logit += load_state_f32(h(22u) + final_state + source)
      * effective_weight(tick, synapse);
  }
  store_output_f32(h(40u) + tick, logit);
}

var<workgroup> loss_partial:array<f32,64>;
var<workgroup> count_partial:array<f32,64>;
@compute @workgroup_size(64)
fn reduce_loss(@builtin(local_invocation_index) lane:u32) {
  let neurons = h(1u);
  var total = 0.0;
  var count = 0.0;
  for (var tick = 0u; tick < h(5u); tick++) {
    let final_state = (tick + 1u) * h(4u) * neurons;
    for (var neuron = lane; neuron < neurons; neuron += 64u) {
      let weight = load_training_f32(h(20u) + tick * neurons + neuron);
      if (weight <= 0.0) { continue; }
      let observed = load_state_f32(h(22u) + final_state + neuron);
      let expected = load_training_f32(h(19u) + tick * neurons + neuron);
      let error = observed - expected;
      total += weight * error * error;
      count += weight;
    }
    if (lane == 0u && candidate_word(tick, 0u) != 0u) {
      let weight = bitcast<f32>(candidate_word(tick, 3u));
      let expected = bitcast<f32>(candidate_word(tick, 2u));
      let observed = load_output_f32(h(28u) + tick);
      let error = observed - expected;
      total += weight * error * error;
      count += weight;
    }
    if (lane == 0u && candidate_word(tick, 28u) != 0u) {
      let weight = bitcast<f32>(candidate_word(tick, 31u));
      let expected = bitcast<f32>(candidate_word(tick, 30u));
      let observed = load_output_f32(h(40u) + tick);
      let error = observed - expected;
      total += weight * error * error;
      count += weight;
    }
  }
  loss_partial[lane] = total;
  count_partial[lane] = count;
  workgroupBarrier();
  for (var stride = 32u; stride > 0u; stride /= 2u) {
    if (lane < stride) {
      loss_partial[lane] += loss_partial[lane + stride];
      count_partial[lane] += count_partial[lane + stride];
    }
    workgroupBarrier();
  }
  if (lane == 0u) {
    store_output_f32(h(29u), loss_partial[0] / max(count_partial[0], 1.0));
    store_output_f32(h(29u) + 1u, count_partial[0]);
  }
}

@compute @workgroup_size(64)
fn seed_candidate_activation_gradients(@builtin(global_invocation_id) gid:vec3<u32>) {
  let neuron = gid.x;
  let tick = gid.y;
  if (neuron >= h(1u) || tick >= h(5u) || tick * h(4u) < h(57u)) { return; }
  var gradient = 0.0;
  let begin = meta_words[h(14u) + neuron];
  let end = meta_words[h(14u) + neuron + 1u];
  for (var cursor = begin; cursor < end; cursor++) {
    let synapse = meta_words[h(15u) + cursor];
    if (synapse_word(synapse, 2u) != SYNAPSE_DECODER || synapse_word(synapse, 4u) != DECODER_ACTION_CANDIDATE) { continue; }
    for (var candidate = 0u; candidate < h(51u); candidate++) {
      if (candidate_field(tick, candidate, 0u) == 0u || candidate_field(tick, candidate, 1u) != synapse_word(synapse, 5u)) { continue; }
      gradient += candidate_adjoint(tick, candidate) * candidate_input(tick, candidate, synapse_word(synapse, 6u)) * effective_weight(tick, synapse);
    }
  }
  let index = h(24u) + (tick + 1u) * h(4u) * h(1u) + neuron;
  store_gradient_f32(index, load_gradient_f32(index) + gradient);
}

@compute @workgroup_size(64)
fn seed_speech_activation_gradients(@builtin(global_invocation_id) gid:vec3<u32>) {
  let neuron = gid.x;
  let tick = gid.y;
  if (neuron >= h(1u) || tick >= h(5u) || candidate_word(tick, 28u) == 0u) { return; }
  let target_neuron = h(37u) + candidate_word(tick, 29u);
  let observed = load_output_f32(h(40u) + tick);
  let expected = bitcast<f32>(candidate_word(tick, 30u));
  let loss_weight = bitcast<f32>(candidate_word(tick, 31u));
  let logit_gradient = 2.0 * loss_weight * (observed - expected) / loss_denominator();
  var gradient = 0.0;
  let begin = meta_words[h(14u) + neuron];
  let end = meta_words[h(14u) + neuron + 1u];
  for (var cursor = begin; cursor < end; cursor++) {
    let synapse = meta_words[h(15u) + cursor];
    if (synapse_word(synapse, 2u) != SYNAPSE_DECODER
        || synapse_word(synapse, 4u) != DECODER_SPEECH_PAYLOAD
        || synapse_word(synapse, 1u) != target_neuron) { continue; }
    gradient += logit_gradient * effective_weight(tick, synapse);
  }
  let final_state = (tick + 1u) * h(4u) * h(1u);
  let index = h(24u) + final_state + neuron;
  store_gradient_f32(index, load_gradient_f32(index) + gradient);
}

@compute @workgroup_size(64)
fn backward_local(@builtin(global_invocation_id) gid:vec3<u32>) {
  let neuron = gid.x;
  let neurons = h(1u);
  if (neuron >= neurons) { return; }
  let step = h(7u);
  let tick = h(9u);
  let current_state = (step + 1u) * neurons;
  let previous_state = step * neurons;
  if (!microstep_active(tick, h(8u))) {
    let ga = h(24u) + previous_state + neuron;
    let gm = h(25u) + previous_state + neuron;
    store_gradient_f32(ga, load_gradient_f32(ga) + load_gradient_f32(h(24u) + current_state + neuron));
    store_gradient_f32(gm, load_gradient_f32(gm) + load_gradient_f32(h(25u) + current_state + neuron));
    return;
  }
  let current = load_state_f32(h(22u) + current_state + neuron);
  let prior = load_state_f32(h(22u) + previous_state + neuron);
  let previous_metabolic = load_state_f32(h(23u) + previous_state + neuron);
  let current_metabolic_gradient = load_gradient_f32(h(25u) + current_state + neuron);
  let dynamics = h(17u) + neuron * DYNAMICS_STRIDE;
  let leak = bitcast<f32>(meta_words[dynamics + 1u]);
  let activation_kind = meta_words[dynamics + 2u];
  let homeostatic_gain = bitcast<f32>(meta_words[dynamics + 3u]);
  let metabolic_decay = bitcast<f32>(meta_words[dynamics + 4u]);
  var activation_gradient = load_gradient_f32(h(24u) + current_state + neuron);
  if (h(8u) + 1u == h(4u)) {
    let target_weight = load_training_f32(h(20u) + tick * neurons + neuron);
    if (target_weight > 0.0) {
      let expected = load_training_f32(h(19u) + tick * neurons + neuron);
      activation_gradient += 2.0 * target_weight * (current - expected) / loss_denominator();
    }
  }
  let metabolic_raw = metabolic_decay * previous_metabolic
    + (1.0 - metabolic_decay) * current * current;
  let metabolic_gate = select(0.0, 1.0, metabolic_raw > 0.0 && metabolic_raw < 1.0);
  let total_output_gradient = activation_gradient
    + current_metabolic_gradient * metabolic_gate * 2.0 * (1.0 - metabolic_decay) * current;
  let activated = select(
    0.0,
    (current - (1.0 - leak) * prior) / leak,
    leak > 1.0e-8
  );
  let delta = total_output_gradient * leak * activation_derivative(activated, activation_kind);
  store_gradient_f32(h(26u) + step * neurons + neuron, delta);
  let previous_activation_index = h(24u) + previous_state + neuron;
  store_gradient_f32(
    previous_activation_index,
    load_gradient_f32(previous_activation_index) + total_output_gradient * (1.0 - leak)
  );
  let previous_metabolic_index = h(25u) + previous_state + neuron;
  store_gradient_f32(
    previous_metabolic_index,
    load_gradient_f32(previous_metabolic_index)
      + current_metabolic_gradient * metabolic_gate * metabolic_decay
      - delta * homeostatic_gain
  );
}

@compute @workgroup_size(64)
fn backward_recurrent_sources(@builtin(global_invocation_id) gid:vec3<u32>) {
  let source = gid.x;
  let neurons = h(1u);
  if (source >= neurons) { return; }
  let step = h(7u);
  if (!microstep_active(h(9u), h(8u))) { return; }
  var gradient = 0.0;
  let begin = meta_words[h(14u) + source];
  let end = meta_words[h(14u) + source + 1u];
  for (var cursor = begin; cursor < end; cursor++) {
    let synapse = meta_words[h(15u) + cursor];
    if (synapse_word(synapse, 2u) != SYNAPSE_RECURRENT
        || !route_enabled(h(9u), synapse)
        || !route_fires(synapse_word(synapse, 3u), h(8u))) { continue; }
    let target_index = synapse_word(synapse, 1u);
    gradient += load_gradient_f32(h(26u) + step * neurons + target_index)
      * effective_weight(h(9u), synapse) * projection_gain(h(9u));
  }
  if (h(41u) != 0u) {
    let tick = h(9u);
    for (var edge = 0u; edge < structural_edge_count(tick); edge++) {
      let at = structural_edge_base(tick) + edge * 5u;
      if (training_words[at] != source) { continue; }
      let route = training_words[at + 2u];
      if (training_words[context_base(tick) + 4u + route] == 0u
          || !route_fires(training_words[at + 3u], h(8u))) { continue; }
      gradient += load_gradient_f32(h(26u) + step * neurons + training_words[at + 1u])
        * bitcast<f32>(training_words[at + 4u]) * projection_gain(tick);
    }
  }
  let branch_begin = meta_words[h(47u) + source];
  let branch_end = meta_words[h(47u) + source + 1u];
  for (var cursor = branch_begin; cursor < branch_end; cursor++) {
    let branch = meta_words[h(48u) + cursor * 2u];
    let weight = bitcast<f32>(meta_words[h(48u) + cursor * 2u + 1u]);
    let excess = dendritic_excess(branch, step * neurons);
    if (excess <= 0.0) { continue; }
    let output = tanh(excess);
    let base = h(45u) + branch * 5u;
    let target_index = meta_words[base];
    gradient += load_gradient_f32(h(26u) + step * neurons + target_index)
      * projection_gain(h(9u)) * bitcast<f32>(meta_words[base + 2u]) * (1.0 - output * output) * weight;
  }
  let index = h(24u) + step * neurons + source;
  store_gradient_f32(index, load_gradient_f32(index) + gradient);
}

@compute @workgroup_size(64)
fn recurrent_weight_gradients(@builtin(global_invocation_id) gid:vec3<u32>) {
  let synapse = gid.x;
  if (synapse >= h(2u) || synapse_word(synapse, 2u) != SYNAPSE_RECURRENT) { return; }
  let source = synapse_word(synapse, 0u);
  let target_index = synapse_word(synapse, 1u);
  let cadence = synapse_word(synapse, 3u);
  var gradient = 0.0;
  for (var step = h(57u); step < h(6u); step++) {
    if (!microstep_active(step / h(4u), step % h(4u)) || !route_enabled(step / h(4u), synapse) || !route_fires(cadence, step % h(4u))) { continue; }
    gradient += load_gradient_f32(h(26u) + step * h(1u) + target_index)
      * load_state_f32(h(22u) + step * h(1u) + source) * projection_gain(step / h(4u));
  }
  store_gradient_f32(h(27u) + synapse, gradient);
}

@compute @workgroup_size(64)
fn candidate_weight_gradients(@builtin(global_invocation_id) gid:vec3<u32>) {
  let synapse = gid.x;
  if (synapse >= h(2u) || synapse_word(synapse, 2u) != SYNAPSE_DECODER) { return; }
  let head = synapse_word(synapse, 4u);
  if (head != 1u && head != 2u && head != 4u) { return; }
  let source = synapse_word(synapse, 0u);
  let family = synapse_word(synapse, 5u);
  let lane = synapse_word(synapse, 6u);
  var gradient = 0.0;
  for (var tick = h(57u) / h(4u); tick < h(5u); tick++) {
    let final_state = (tick + 1u) * h(4u) * h(1u);
    for (var candidate = 0u; candidate < h(51u); candidate++) {
      if (candidate_field(tick, candidate, 0u) == 0u || candidate_field(tick, candidate, 1u) != family) { continue; }
      if (head == 1u) {
        gradient += candidate_adjoint(tick, candidate) * load_state_f32(h(22u) + final_state + source) * candidate_input(tick, candidate, lane);
      } else if (h(41u) != 0u) {
        gradient += candidate_adjoint(tick, candidate) * auxiliary_gate(tick, candidate, head) * candidate_input(tick, candidate, lane);
      }
    }
  }
  store_gradient_f32(h(27u) + synapse, gradient);
}

@compute @workgroup_size(64)
fn speech_weight_gradients(@builtin(global_invocation_id) gid:vec3<u32>) {
  let synapse = gid.x;
  if (synapse >= h(2u)
      || synapse_word(synapse, 2u) != SYNAPSE_DECODER
      || synapse_word(synapse, 4u) != DECODER_SPEECH_PAYLOAD) { return; }
  let source = synapse_word(synapse, 0u);
  let target_neuron = synapse_word(synapse, 1u);
  var gradient = 0.0;
  for (var tick = 0u; tick < h(5u); tick++) {
    if (candidate_word(tick, 28u) == 0u
        || h(37u) + candidate_word(tick, 29u) != target_neuron) { continue; }
    let observed = load_output_f32(h(40u) + tick);
    let expected = bitcast<f32>(candidate_word(tick, 30u));
    let loss_weight = bitcast<f32>(candidate_word(tick, 31u));
    let logit_gradient = 2.0 * loss_weight * (observed - expected) / loss_denominator();
    let final_state = (tick + 1u) * h(4u) * h(1u);
    gradient += logit_gradient * load_state_f32(h(22u) + final_state + source);
  }
  store_gradient_f32(h(27u) + synapse, gradient);
}

var<workgroup> norm_partial:array<f32,64>;
@compute @workgroup_size(64)
fn reduce_gradient_norm(@builtin(local_invocation_index) lane:u32) {
  var squared = 0.0;
  for (var synapse = lane; synapse < h(2u); synapse += 64u) {
    if (trainable_mask[synapse] == 0u) { continue; }
    let gradient = load_gradient_f32(h(27u) + synapse);
    squared += gradient * gradient;
  }
  norm_partial[lane] = squared;
  workgroupBarrier();
  for (var stride = 32u; stride > 0u; stride /= 2u) {
    if (lane < stride) { norm_partial[lane] += norm_partial[lane + stride]; }
    workgroupBarrier();
  }
  if (lane == 0u) { store_output_f32(h(29u) + 2u, sqrt(max(norm_partial[0], 0.0))); }
}

fn adam_proposal(synapse:u32) -> vec3<f32> {
  let learning_rate = bitcast<f32>(h(31u));
  let beta1 = bitcast<f32>(h(32u));
  let beta2 = bitcast<f32>(h(33u));
  let epsilon = bitcast<f32>(h(34u));
  let weight_decay = bitcast<f32>(h(35u));
  let gradient_clip = bitcast<f32>(h(36u));
  let norm = load_output_f32(h(29u) + 2u);
  let scale = select(1.0, gradient_clip / norm, norm > gradient_clip && norm > 0.0);
  let gradient = load_gradient_f32(h(27u) + synapse) * scale;
  let old_m = load_optimizer_f32(synapse);
  let old_v = load_optimizer_f32(h(30u) + synapse);
  let next_m = beta1 * old_m + (1.0 - beta1) * gradient;
  let next_v = beta2 * old_v + (1.0 - beta2) * gradient * gradient;
  // Frozen weights retain both their moments and their own update age.
  // Global optimizer steps count batches, not participation in those batches.
  let step = f32(min(optimizer_words[h(62u) + synapse], 0xfffffffeu) + 1u);
  let corrected_m = next_m / (1.0 - pow(beta1, step));
  let corrected_v = next_v / (1.0 - pow(beta2, step));
  let old_weight = load_weight_f32(synapse);
  var next_weight = old_weight - learning_rate
    * (corrected_m / (sqrt(corrected_v) + epsilon) + weight_decay * old_weight);
  let sign_policy = synapse_word(synapse, 7u);
  if (sign_policy == 1u && next_weight >= 0.0) { next_weight = -0.0001; }
  if (sign_policy == 2u && next_weight < 0.0) { next_weight = 0.0001; }
  return vec3<f32>(next_weight, next_m, next_v);
}

var<workgroup> adam_invalid:array<u32,64>;
@compute @workgroup_size(64)
fn validate_accumulation(@builtin(local_invocation_index) lane:u32) {
  var invalid = 0u;
  if (!finite(load_output_f32(h(29u) + 2u))) { invalid = 1u; }
  for (var synapse = lane; synapse < h(2u); synapse += 64u) {
    if (trainable_mask[synapse] == 0u) { continue; }
    let next = load_gradient_f32(h(60u) + synapse)
      + bitcast<f32>(h(61u)) * load_gradient_f32(h(27u) + synapse);
    if (!finite(next)) { invalid = 1u; }
  }
  adam_invalid[lane] = invalid;
  workgroupBarrier();
  for (var stride = 32u; stride > 0u; stride /= 2u) {
    if (lane < stride) { adam_invalid[lane] |= adam_invalid[lane + stride]; }
    workgroupBarrier();
  }
  if (lane == 0u) { store_output_f32(h(29u) + 3u, select(1.0, 0.0, adam_invalid[0] != 0u)); }
}

@compute @workgroup_size(64)
fn accumulate_weight_gradients(@builtin(global_invocation_id) gid:vec3<u32>) {
  let synapse = gid.x;
  if (synapse >= h(2u) || trainable_mask[synapse] == 0u || load_output_f32(h(29u) + 3u) != 1.0) { return; }
  store_gradient_f32(h(60u) + synapse, load_gradient_f32(h(60u) + synapse)
    + bitcast<f32>(h(61u)) * load_gradient_f32(h(27u) + synapse));
}

@compute @workgroup_size(64)
fn validate_adamw(@builtin(local_invocation_index) lane:u32) {
  var invalid = 0u;
  if (!finite(load_output_f32(h(29u) + 2u))) { invalid = 1u; }
  for (var synapse = lane; synapse < h(2u); synapse += 64u) {
    if (trainable_mask[synapse] == 0u) { continue; }
    if (optimizer_words[h(62u) + synapse] == 0xffffffffu) { invalid = 1u; }
    let next = adam_proposal(synapse);
    if (!finite(next.x) || !finite(next.y) || !finite(next.z)) { invalid = 1u; }
  }
  adam_invalid[lane] = invalid;
  workgroupBarrier();
  for (var stride = 32u; stride > 0u; stride /= 2u) {
    if (lane < stride) { adam_invalid[lane] |= adam_invalid[lane + stride]; }
    workgroupBarrier();
  }
  if (lane == 0u) { store_output_f32(h(29u) + 3u, select(1.0, 0.0, adam_invalid[0] != 0u)); }
}

@compute @workgroup_size(64)
fn apply_adamw(@builtin(global_invocation_id) gid:vec3<u32>) {
  let synapse = gid.x;
  if (synapse >= h(2u) || trainable_mask[synapse] == 0u || load_output_f32(h(29u) + 3u) != 1.0) { return; }
  let next = adam_proposal(synapse);
  store_optimizer_f32(synapse, next.y);
  store_optimizer_f32(h(30u) + synapse, next.z);
  store_weight_f32(synapse, next.x);
  optimizer_words[h(62u) + synapse] += 1u;
}
