// Training-only objective. Logits/features are produced by the existing sparse
// production-graph replay. No neural forward engine or sampling is implemented here.
struct PpoHeader { words: array<vec4<u32>, 8>, }
@group(0) @binding(0) var<uniform> header: PpoHeader;
@group(0) @binding(1) var<storage, read> rows: array<u32>;
@group(0) @binding(2) var<storage, read> logits: array<u32>;
@group(0) @binding(3) var<storage, read> features: array<u32>;
@group(0) @binding(4) var<storage, read_write> output: array<u32>;
@group(0) @binding(5) var<storage, read_write> head: array<u32>;

fn h(i:u32) -> u32 { return header.words[i / 4u][i % 4u]; }
fn hf(i:u32) -> f32 { return bitcast<f32>(h(i)); }
fn r(row:u32, i:u32) -> u32 { return rows[row * 64u + i]; }
fn rf(row:u32, i:u32) -> f32 { return bitcast<f32>(r(row, i)); }
fn finite(x:f32) -> bool { return x == x && abs(x) <= 3.402823466e+38; }
fn metric_base(row:u32) -> u32 { return h(2u) * 32u + row * 8u; }
fn value_base() -> u32 { return h(2u) * 40u; }
fn put(i:u32, x:f32) { output[i] = bitcast<u32>(x); }
fn get(i:u32) -> f32 { return bitcast<f32>(output[i]); }
fn head_get(i:u32) -> f32 { return bitcast<f32>(head[i]); }
fn head_put(i:u32, x:f32) { head[i] = bitcast<u32>(x); }

struct MaskedCategorical {
  probability: array<f32, 32>,
  log_probability: array<f32, 32>,
  entropy: f32,
  valid: bool,
}

fn masked_categorical(row:u32, mask:u32) -> MaskedCategorical {
  var distribution: MaskedCategorical;
  if (mask == 0u) { return distribution; }
  let count = r(row, 0u);
  var maximum = -3.402823466e+38;
  for (var candidate = 0u; candidate < count; candidate++) {
    if ((mask & (1u << candidate)) == 0u) { continue; }
    let logit = bitcast<f32>(logits[r(row, 51u) + candidate]) / hf(7u);
    if (!finite(logit)) { return distribution; }
    maximum = max(maximum, logit);
  }
  var partition = 0.0;
  for (var candidate = 0u; candidate < count; candidate++) {
    if ((mask & (1u << candidate)) == 0u) { continue; }
    let shifted = bitcast<f32>(logits[r(row, 51u) + candidate]) / hf(7u) - maximum;
    distribution.probability[candidate] = exp(shifted);
    partition += distribution.probability[candidate];
  }
  if (!finite(partition) || partition <= 0.0) { return distribution; }
  let log_partition = log(partition);
  for (var candidate = 0u; candidate < count; candidate++) {
    if ((mask & (1u << candidate)) == 0u) { continue; }
    let log_probability = bitcast<f32>(logits[r(row, 51u) + candidate]) / hf(7u) - maximum - log_partition;
    let probability = distribution.probability[candidate] / partition;
    distribution.probability[candidate] = probability;
    distribution.log_probability[candidate] = log_probability;
    if (probability > 0.0) { distribution.entropy -= probability * log_probability; }
  }
  distribution.valid = true;
  return distribution;
}

@compute @workgroup_size(1)
fn ppo_value_forward(@builtin(global_invocation_id) gid:vec3<u32>) {
  let row = gid.x;
  if (row >= h(0u)) { return; }
  var value = head_get(h(1u)); // final coordinate is bias
  for (var feature = 0u; feature < h(1u); feature++) {
    value += head_get(feature) * bitcast<f32>(features[r(row, 52u) + feature]);
  }
  put(value_base() + row, value);
}

@compute @workgroup_size(1)
fn ppo_objective(@builtin(global_invocation_id) gid:vec3<u32>) {
  let row = gid.x;
  if (row >= h(0u)) { return; }
  let metrics = metric_base(row);
  for (var i = 0u; i < 8u; i++) { put(metrics + i, 0.0); }
  for (var i = 0u; i < 32u; i++) { put(row * 32u + i, 0.0); }
  let count = r(row, 0u);
  let representative = r(row, 1u);
  if (count == 0u || count > 32u || representative >= count || !finite(hf(7u)) || hf(7u) <= 0.0) { return; }
  let forced = r(row, 19u + representative);
  var probabilities: array<array<f32, 32>, 7>;
  var log_probabilities: array<array<f32, 32>, 7>;
  var entropies: array<f32, 7>;
  var masks: array<u32, 7>;
  // Global representative is factor0; factors1..6 are motor slot conditionals.
  for (var factor = 0u; factor < 7u; factor++) {
    masks[factor] = r(row, 6u + factor);
    if (masks[factor] == 0u) { continue; }
    let distribution = masked_categorical(row, masks[factor]);
    if (!distribution.valid) { return; }
    probabilities[factor] = distribution.probability;
    log_probabilities[factor] = distribution.log_probability;
    entropies[factor] = distribution.entropy;
  }
  if ((masks[0] & (1u << representative)) == 0u) { return; }
  var joint_log_probability = log_probabilities[0][representative];
  for (var slot = 0u; slot < 6u; slot++) {
    if (slot == forced || masks[slot + 1u] == 0u) { continue; }
    let selected = r(row, 13u + slot);
    if (selected >= count || (masks[slot + 1u] & (1u << selected)) == 0u) { return; }
    joint_log_probability += log_probabilities[slot + 1u][selected];
  }
  // Exact conditional joint entropy: H(R) + sum_s (1-P(R forces s))*H(S_s).
  // Its derivative includes the representative/channel coupling, not merely a
  // sampled sum of factor entropies with a detached representative.
  var forced_mass: array<f32, 6>;
  for (var candidate = 0u; candidate < count; candidate++) {
    let slot = r(row, 19u + candidate);
    if (slot < 6u) { forced_mass[slot] += probabilities[0][candidate]; }
  }
  var entropy = entropies[0];
  var expected_forced_entropy = 0.0;
  for (var slot = 0u; slot < 6u; slot++) {
    entropy += (1.0 - forced_mass[slot]) * entropies[slot + 1u];
    expected_forced_entropy += forced_mass[slot] * entropies[slot + 1u];
  }
  let log_ratio = joint_log_probability - rf(row, 2u);
  let ratio = exp(log_ratio);
  let advantage = rf(row, 3u);
  let clipped_ratio = clamp(ratio, 1.0 - hf(4u), 1.0 + hf(4u));
  let policy_loss = -min(ratio * advantage, clipped_ratio * advantage);
  let clipped = (advantage >= 0.0 && ratio > 1.0 + hf(4u))
    || (advantage < 0.0 && ratio < 1.0 - hf(4u));
  let log_probability_gradient = select(-advantage * ratio, 0.0, clipped);
  let value_error = get(value_base() + row) - rf(row, 4u);
  let value_loss = 0.5 * hf(6u) * value_error * value_error;
  let approximate_kl = max(0.0, (ratio - 1.0) - log_ratio);
  if (!finite(joint_log_probability) || !finite(ratio) || !finite(entropy)
      || !finite(policy_loss) || !finite(value_loss) || !finite(approximate_kl)) { return; }
  var gradients: array<f32, 32>;
  for (var candidate = 0u; candidate < count; candidate++) {
    let global_p = probabilities[0][candidate];
    var logp_derivative = select(0.0, 1.0, candidate == representative) - global_p;
    var entropy_derivative = 0.0;
    if (global_p > 0.0) {
      let slot = r(row, 19u + candidate);
      var forced_entropy = 0.0;
      if (slot < 6u) { forced_entropy = entropies[slot + 1u]; }
      entropy_derivative = -global_p * (log_probabilities[0][candidate] + entropies[0]
        + forced_entropy - expected_forced_entropy);
    }
    for (var slot = 0u; slot < 6u; slot++) {
      if (masks[slot + 1u] == 0u) { continue; }
      let p = probabilities[slot + 1u][candidate];
      if (slot != forced) {
        logp_derivative += select(0.0, 1.0, candidate == r(row, 13u + slot)) - p;
      }
      if (p > 0.0) {
        entropy_derivative -= (1.0 - forced_mass[slot]) * p
          * (log_probabilities[slot + 1u][candidate] + entropies[slot + 1u]);
      }
    }
    gradients[candidate] = (log_probability_gradient * logp_derivative - hf(5u) * entropy_derivative)
      / (hf(7u) * f32(h(0u)));
    if (!finite(gradients[candidate])) { return; }
  }
  for (var candidate = 0u; candidate < count; candidate++) {
    put(row * 32u + candidate, gradients[candidate]);
  }
  put(metrics, joint_log_probability);
  put(metrics + 1u, approximate_kl);
  put(metrics + 2u, entropy);
  put(metrics + 3u, policy_loss);
  put(metrics + 4u, value_loss);
  put(metrics + 5u, select(0.0, 1.0, clipped));
  put(metrics + 6u, ratio);
  put(metrics + 7u, 1.0); // invalid rows remain0; host must reject before policy update
}

@compute @workgroup_size(64)
fn ppo_value_gradients(@builtin(global_invocation_id) gid:vec3<u32>) {
  let coordinate = gid.x;
  let width = h(1u) + 1u;
  if (coordinate >= width) { return; }
  var gradient = 0.0;
  for (var row = 0u; row < h(0u); row++) {
    var feature = 1.0;
    if (coordinate < h(1u)) { feature = bitcast<f32>(features[r(row, 52u) + coordinate]); }
    gradient += hf(6u) * (get(value_base() + row) - rf(row, 4u)) * feature / f32(h(0u));
  }
  // Detached features: this pass never writes any recurrent activation adjoint.
  head_put(width * 3u + coordinate, gradient);
}

fn value_update_scale() -> f32 {
  var mean_kl = 0.0;
  for (var row = 0u; row < h(0u); row++) {
    if (get(metric_base(row) + 7u) != 1.0) { return -1.0; }
    mean_kl += get(metric_base(row) + 1u) / f32(h(0u));
  }
  if (h(17u) != 0u) { mean_kl=hf(18u); }
  if (!finite(mean_kl) || mean_kl > hf(14u) || h(3u) == 0u) { return -1.0; }
  let width = h(1u) + 1u;
  var squared_norm = 0.0;
  for (var coordinate = 0u; coordinate < width; coordinate++) {
    let gradient = head_get(width * 3u + coordinate);
    if (!finite(gradient)) { return -1.0; }
    squared_norm += gradient * gradient;
  }
  if (!finite(squared_norm)) { return -1.0; }
  return min(1.0, hf(13u) / max(sqrt(squared_norm), 1.0e-12));
}

@compute @workgroup_size(1)
fn ppo_value_preflight() {
  put(h(2u) * 41u, 0.0);
  if (hf(6u) == 0.0) { put(h(2u) * 41u, 1.0); return; }
  let scale = value_update_scale();
  if (scale < 0.0) { return; }
  let width = h(1u) + 1u;
  let correction1 = 1.0 - pow(hf(9u), f32(h(3u)));
  let correction2 = 1.0 - pow(hf(10u), f32(h(3u)));
  // Validate the complete head before mutating any parameter or Adam moment.
  for (var coordinate = 0u; coordinate < width; coordinate++) {
    let next = value_adam_coordinate(coordinate, width, scale, correction1, correction2);
    if (!finite(next.x) || !finite(next.y) || !finite(next.z)) { return; }
  }
  put(h(2u) * 41u, 1.0);
}

@compute @workgroup_size(1)
fn ppo_value_update() {
  if (get(h(2u) * 41u) != 1.0 || hf(6u) == 0.0) { return; }
  let scale = value_update_scale();
  if (scale < 0.0) { return; }
  let width = h(1u) + 1u;
  let correction1 = 1.0 - pow(hf(9u), f32(h(3u)));
  let correction2 = 1.0 - pow(hf(10u), f32(h(3u)));
  for (var coordinate = 0u; coordinate < width; coordinate++) {
    let next = value_adam_coordinate(coordinate, width, scale, correction1, correction2);
    head_put(width + coordinate, next.y);
    head_put(width * 2u + coordinate, next.z);
    head_put(coordinate, next.x);
  }
}

fn value_adam_coordinate(coordinate:u32, width:u32, scale:f32, correction1:f32, correction2:f32) -> vec3<f32> {
  let gradient = head_get(width * 3u + coordinate) * scale;
  let m = hf(9u) * head_get(width + coordinate) + (1.0 - hf(9u)) * gradient;
  let v = hf(10u) * head_get(width * 2u + coordinate) + (1.0 - hf(10u)) * gradient * gradient;
  let weight = head_get(coordinate);
  let next = weight - hf(8u) * ((m / correction1) / (sqrt(v / correction2) + hf(11u)) + hf(12u) * weight);
  return vec3<f32>(next, m, v);
}

// Labels describe acceptable commands, never neural inputs. Marginalize over
// representatives because each representative deterministically overrides one slot.
fn imitation_objective(row:u32, add:bool) {
  let metrics = h(2u) * 41u + 1u + row * 4u;
  for (var i=0u; i<4u; i++) { put(metrics+i, 0.0); }
  if (!add) { for (var i=0u; i<32u; i++) { put(row*32u+i, 0.0); } }
  let count = r(row,0u);
  if (count == 0u || count > 32u || hf(15u) < 0.0 || !finite(hf(15u))) { return; }
  var distributions: array<MaskedCategorical,7>;
  var accepted: array<MaskedCategorical,6>;
  var log_mass: array<f32,6>;
  for (var factor=0u; factor<7u; factor++) {
    if (r(row,6u+factor) != 0u) {
      distributions[factor] = masked_categorical(row,r(row,6u+factor));
      if (!distributions[factor].valid) { return; }
    }
  }
  if (!distributions[0].valid) { return; }
  for (var slot=0u; slot<6u; slot++) {
    if ((r(row,60u) & (2u<<slot)) == 0u) { continue; }
    let mask = r(row,7u+slot) & r(row,54u+slot);
    if (mask == 0u) { continue; }
    accepted[slot] = masked_categorical(row,mask);
    if (!accepted[slot].valid) { return; }
    // Log mass uses a shared pivot rather than summing possibly underflowed p.
    var pivot = 0u;
    var pivot_log = -3.402823466e+38;
    for (var candidate=0u; candidate<count; candidate++) {
      if ((mask & (1u<<candidate)) != 0u && distributions[slot+1u].log_probability[candidate]>pivot_log) {
        pivot=candidate; pivot_log=distributions[slot+1u].log_probability[candidate];
      }
    }
    log_mass[slot] = distributions[slot+1u].log_probability[pivot] - accepted[slot].log_probability[pivot];
  }
  var terms: array<f32,32>;
  var compatible: array<bool,32>;
  var maximum = -3.402823466e+38;
  for (var representative=0u; representative<count; representative++) {
    if ((r(row,6u)&(1u<<representative)) == 0u) { continue; }
    if ((r(row,60u)&1u) != 0u && (r(row,53u)&(1u<<representative)) == 0u) { continue; }
    let forced = r(row,19u+representative);
    var term = distributions[0].log_probability[representative];
    var valid = true;
    for (var slot=0u; slot<6u; slot++) {
      if ((r(row,60u)&(2u<<slot)) == 0u) { continue; }
      if (forced == slot) {
        if ((r(row,54u+slot)&(1u<<representative)) == 0u) { valid=false; }
      } else {
        if (!accepted[slot].valid) { valid=false; }
        term += log_mass[slot];
      }
    }
    if (!valid || !finite(term)) { continue; }
    compatible[representative]=true;
    terms[representative]=term;
    maximum=max(maximum,term);
  }
  var partition=0.0;
  for (var candidate=0u; candidate<count; candidate++) {
    if (compatible[candidate]) { partition+=exp(terms[candidate]-maximum); }
  }
  if (partition <= 0.0 || !finite(partition)) { return; }
  let log_probability=maximum+log(partition);
  var posterior: array<f32,32>;
  var forced_mass: array<f32,6>;
  for (var candidate=0u; candidate<count; candidate++) {
    if (!compatible[candidate]) { continue; }
    posterior[candidate]=exp(terms[candidate]-maximum)/partition;
    let forced=r(row,19u+candidate);
    if (forced<6u) { forced_mass[forced]+=posterior[candidate]; }
  }
  var gradients: array<f32,32>;
  for (var candidate=0u; candidate<count; candidate++) {
    var gradient=distributions[0].probability[candidate]-posterior[candidate];
    for (var slot=0u; slot<6u; slot++) {
      if ((r(row,60u)&(2u<<slot)) == 0u) { continue; }
      gradient+=(1.0-forced_mass[slot])*(distributions[slot+1u].probability[candidate]-accepted[slot].probability[candidate]);
    }
    gradients[candidate]=hf(15u)*gradient/(hf(7u)*f32(h(0u)));
    if (add) { gradients[candidate]+=get(row*32u+candidate); }
    if (!finite(gradients[candidate])) { return; }
  }
  let loss=-hf(15u)*log_probability;
  if (!finite(loss)) { return; }
  for (var candidate=0u; candidate<count; candidate++) { put(row*32u+candidate,gradients[candidate]); }
  put(metrics,log_probability);
  put(metrics+1u,loss);
  put(metrics+2u,hf(15u));
  put(metrics+3u,1.0);
}

@compute @workgroup_size(1)
fn imitation_replace(@builtin(global_invocation_id) gid:vec3<u32>) {
  if (gid.x<h(0u)) { imitation_objective(gid.x,false); }
}
@compute @workgroup_size(1)
fn imitation_add(@builtin(global_invocation_id) gid:vec3<u32>) {
  if (gid.x<h(0u)) { imitation_objective(gid.x,true); }
}

@compute @workgroup_size(64)
fn value_accum_clear(@builtin(global_invocation_id) gid:vec3<u32>) {
  let width=h(1u)+1u;
  if (gid.x<width) { head_put(width*4u+gid.x,0.0); }
}
@compute @workgroup_size(64)
fn value_accum_add(@builtin(global_invocation_id) gid:vec3<u32>) {
  let width=h(1u)+1u;
  if (gid.x<width) { head_put(width*4u+gid.x,head_get(width*4u+gid.x)+hf(16u)*head_get(width*3u+gid.x)); }
}
@compute @workgroup_size(64)
fn value_accum_finish(@builtin(global_invocation_id) gid:vec3<u32>) {
  let width=h(1u)+1u;
  if (gid.x<width) { head_put(width*3u+gid.x,head_get(width*4u+gid.x)); }
}
