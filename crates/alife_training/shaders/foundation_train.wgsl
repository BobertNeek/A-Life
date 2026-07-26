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
const CANDIDATE_RECORD_WORDS:u32 = 32u;
const CANDIDATE_FEATURE_COUNT:u32 = 24u;
const SYNAPSE_RECURRENT:u32 = 1u;
const SYNAPSE_DECODER:u32 = 2u;
const DECODER_ACTION_CANDIDATE:u32 = 1u;

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
  return meta_words[h(16u) + synapse * SYNAPSE_STRIDE + field];
}

fn candidate_word(tick:u32, field:u32) -> u32 {
  return training_words[h(21u) + tick * CANDIDATE_RECORD_WORDS + field];
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
  let begin = meta_words[h(12u) + neuron];
  let end = meta_words[h(12u) + neuron + 1u];
  var recurrent_sum = 0.0;
  for (var cursor = begin; cursor < end; cursor++) {
    let synapse = meta_words[h(13u) + cursor];
    if (synapse_word(synapse, 2u) != SYNAPSE_RECURRENT
        || !route_fires(synapse_word(synapse, 3u), local_step)) { continue; }
    let source = synapse_word(synapse, 0u);
    recurrent_sum += load_state_f32(h(22u) + previous_state + source)
      * load_weight_f32(synapse);
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
  let activated = activation(bias + encoded + recurrent_sum - homeostatic_gain * metabolic, activation_kind);
  var output = (1.0 - leak) * prior + leak * activated;
  var next_metabolic = clamp(
    metabolic_decay * metabolic + (1.0 - metabolic_decay) * output * output,
    0.0,
    1.0
  );
  if (!finite(output) || !finite(next_metabolic)) {
    output = 0.0;
    next_metabolic = 0.0;
  }
  store_state_f32(h(22u) + next_state + neuron, output);
  store_state_f32(h(23u) + next_state + neuron, next_metabolic);
}

@compute @workgroup_size(1)
fn forward_candidate_logits(@builtin(global_invocation_id) gid:vec3<u32>) {
  let tick = gid.x;
  if (tick >= h(5u)) { return; }
  if (candidate_word(tick, 0u) == 0u) {
    store_output_f32(h(28u) + tick, 0.0);
    return;
  }
  let family = candidate_word(tick, 1u);
  let final_state = (tick + 1u) * h(4u) * h(1u);
  var logit = bitcast<f32>(meta_words[h(38u) + family]);
  for (var synapse = 0u; synapse < h(2u); synapse++) {
    if (synapse_word(synapse, 2u) != SYNAPSE_DECODER
        || synapse_word(synapse, 4u) != DECODER_ACTION_CANDIDATE
        || synapse_word(synapse, 5u) != family) { continue; }
    let lane = synapse_word(synapse, 6u);
    let source = synapse_word(synapse, 0u);
    let feature = bitcast<f32>(candidate_word(tick, 4u + lane));
    logit += load_state_f32(h(22u) + final_state + source)
      * feature * load_weight_f32(synapse);
  }
  store_output_f32(h(28u) + tick, logit);
}

@compute @workgroup_size(1)
fn reduce_loss() {
  let neurons = h(1u);
  var total = 0.0;
  var count = 0.0;
  for (var tick = 0u; tick < h(5u); tick++) {
    let final_state = (tick + 1u) * h(4u) * neurons;
    for (var neuron = 0u; neuron < neurons; neuron++) {
      let weight = load_training_f32(h(20u) + tick * neurons + neuron);
      if (weight <= 0.0) { continue; }
      let observed = load_state_f32(h(22u) + final_state + neuron);
      let expected = load_training_f32(h(19u) + tick * neurons + neuron);
      let error = observed - expected;
      total += weight * error * error;
      count += weight;
    }
    if (candidate_word(tick, 0u) != 0u) {
      let weight = bitcast<f32>(candidate_word(tick, 3u));
      let expected = bitcast<f32>(candidate_word(tick, 2u));
      let observed = load_output_f32(h(28u) + tick);
      let error = observed - expected;
      total += weight * error * error;
      count += weight;
    }
  }
  store_output_f32(h(29u), total / max(count, 1.0));
  store_output_f32(h(29u) + 1u, count);
}

@compute @workgroup_size(64)
fn seed_candidate_activation_gradients(@builtin(global_invocation_id) gid:vec3<u32>) {
  let neuron = gid.x;
  let tick = gid.y;
  if (neuron >= h(1u) || tick >= h(5u) || candidate_word(tick, 0u) == 0u) { return; }
  let family = candidate_word(tick, 1u);
  let observed = load_output_f32(h(28u) + tick);
  let expected = bitcast<f32>(candidate_word(tick, 2u));
  let loss_weight = bitcast<f32>(candidate_word(tick, 3u));
  let logit_gradient = 2.0 * loss_weight * (observed - expected) / loss_denominator();
  var gradient = 0.0;
  let begin = meta_words[h(14u) + neuron];
  let end = meta_words[h(14u) + neuron + 1u];
  for (var cursor = begin; cursor < end; cursor++) {
    let synapse = meta_words[h(15u) + cursor];
    if (synapse_word(synapse, 2u) != SYNAPSE_DECODER
        || synapse_word(synapse, 4u) != DECODER_ACTION_CANDIDATE
        || synapse_word(synapse, 5u) != family) { continue; }
    let lane = synapse_word(synapse, 6u);
    gradient += logit_gradient * bitcast<f32>(candidate_word(tick, 4u + lane))
      * load_weight_f32(synapse);
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
  var gradient = 0.0;
  let begin = meta_words[h(14u) + source];
  let end = meta_words[h(14u) + source + 1u];
  for (var cursor = begin; cursor < end; cursor++) {
    let synapse = meta_words[h(15u) + cursor];
    if (synapse_word(synapse, 2u) != SYNAPSE_RECURRENT
        || !route_fires(synapse_word(synapse, 3u), h(8u))) { continue; }
    let target_index = synapse_word(synapse, 1u);
    gradient += load_gradient_f32(h(26u) + step * neurons + target_index)
      * load_weight_f32(synapse);
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
  for (var step = 0u; step < h(6u); step++) {
    if (!route_fires(cadence, step % h(4u))) { continue; }
    gradient += load_gradient_f32(h(26u) + step * h(1u) + target_index)
      * load_state_f32(h(22u) + step * h(1u) + source);
  }
  store_gradient_f32(h(27u) + synapse, gradient);
}

@compute @workgroup_size(64)
fn candidate_weight_gradients(@builtin(global_invocation_id) gid:vec3<u32>) {
  let synapse = gid.x;
  if (synapse >= h(2u)
      || synapse_word(synapse, 2u) != SYNAPSE_DECODER
      || synapse_word(synapse, 4u) != DECODER_ACTION_CANDIDATE) { return; }
  let source = synapse_word(synapse, 0u);
  let family = synapse_word(synapse, 5u);
  let lane = synapse_word(synapse, 6u);
  var gradient = 0.0;
  for (var tick = 0u; tick < h(5u); tick++) {
    if (candidate_word(tick, 0u) == 0u || candidate_word(tick, 1u) != family) { continue; }
    let observed = load_output_f32(h(28u) + tick);
    let expected = bitcast<f32>(candidate_word(tick, 2u));
    let loss_weight = bitcast<f32>(candidate_word(tick, 3u));
    let logit_gradient = 2.0 * loss_weight * (observed - expected) / loss_denominator();
    let final_state = (tick + 1u) * h(4u) * h(1u);
    gradient += logit_gradient
      * load_state_f32(h(22u) + final_state + source)
      * bitcast<f32>(candidate_word(tick, 4u + lane));
  }
  store_gradient_f32(h(27u) + synapse, gradient);
}

@compute @workgroup_size(1)
fn reduce_gradient_norm() {
  var squared = 0.0;
  for (var synapse = 0u; synapse < h(2u); synapse++) {
    if (trainable_mask[synapse] == 0u) { continue; }
    let gradient = load_gradient_f32(h(27u) + synapse);
    squared += gradient * gradient;
  }
  store_output_f32(h(29u) + 2u, sqrt(max(squared, 0.0)));
}

@compute @workgroup_size(64)
fn apply_adamw(@builtin(global_invocation_id) gid:vec3<u32>) {
  let synapse = gid.x;
  if (synapse >= h(2u) || trainable_mask[synapse] == 0u) { return; }
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
  let step = f32(h(10u));
  let corrected_m = next_m / (1.0 - pow(beta1, step));
  let corrected_v = next_v / (1.0 - pow(beta2, step));
  let old_weight = load_weight_f32(synapse);
  var next_weight = old_weight - learning_rate
    * (corrected_m / (sqrt(corrected_v) + epsilon) + weight_decay * old_weight);
  let sign_policy = synapse_word(synapse, 7u);
  if (sign_policy == 1u && next_weight >= 0.0) { next_weight = -0.0001; }
  if (sign_policy == 2u && next_weight < 0.0) { next_weight = 0.0001; }
  if (!finite(next_weight) || !finite(next_m) || !finite(next_v)) { return; }
  store_optimizer_f32(synapse, next_m);
  store_optimizer_f32(h(30u) + synapse, next_v);
  store_weight_f32(synapse, next_weight);
}
