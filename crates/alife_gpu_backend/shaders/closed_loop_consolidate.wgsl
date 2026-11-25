const SLEEP_STATUS_PREPARED:u32 = 256u;
const SLEEP_STATUS_SUCCESS:u32 = 1u;
const SLEEP_STATUS_COMMITTED:u32 = 2u;
const SLEEP_STATUS_REJECTED:u32 = 512u;
const SLEEP_DIAGNOSTIC_Q12:f32 = 4096.0;
const SLEEP_PENDING_ELIGIBILITY_WORDS:u32 = 36u;

fn consolidate_frame_span_within(start:u32, count:u32) -> bool {
  let limit = arrayLength(&frame_payload_words);
  return start <= limit && count <= limit-start;
}

fn consolidate_immutable_plan_span_within(start:u32, count:u32) -> bool {
  let limit = arrayLength(&immutable_plan_words);
  return start <= limit && count <= limit-start;
}

fn consolidate_finite(value:f32) -> bool {
  return (bitcast<u32>(value) & 0x7f800000u) != 0x7f800000u;
}

fn consolidate_array8_equal(left:array<u32,8>, right:array<u32,8>) -> bool {
  for (var index=0u; index<8u; index+=1u) {
    if (left[index] != right[index]) { return false; }
  }
  return true;
}

fn consolidate_reject(completion:u32) {
  if (atomic_span_within(completion,16u)) {
    atomicMax(&atomic_state_words[completion+3u],SLEEP_STATUS_REJECTED);
  }
}

// Match PlasticityReceptorPlan::is_delta_enabled on the immutable plan.
// Current-age and biochemical modulation do not make a synapse born-fixed.
// 0 = disabled, 1 = enabled, 2 = malformed plan (reject the transaction).
fn consolidate_synapse_delta_mode(extension:GpuBrainSlotExtensionRecord, synapse:u32) -> u32 {
  if (synapse > (0xffffffffu-extension.synapse_metadata_offset)/8u) { return 2u; }
  let metadata_base = extension.synapse_metadata_offset+synapse*8u;
  if (!consolidate_immutable_plan_span_within(metadata_base,8u)) { return 2u; }
  let metadata = load_synapse_learning_metadata(metadata_base);
  if (metadata.global_synapse_id != synapse
      || metadata.receptor_index > (0xffffffffu-extension.receptor_offset)/16u) { return 2u; }
  let receptor_base = extension.receptor_offset+metadata.receptor_index*16u;
  if (!consolidate_immutable_plan_span_within(receptor_base,16u)) { return 2u; }
  let receptor = load_plasticity_receptor(receptor_base);
  if (!consolidate_finite(receptor.learning_rate)
      || !consolidate_finite(receptor.sleep_replay_rate)
      || !consolidate_finite(receptor.normalization_rate)
      || receptor.learning_rate < 0.0 || receptor.learning_rate > 1.0
      || receptor.sleep_replay_rate < 0.0 || receptor.sleep_replay_rate > 1.0
      || receptor.normalization_rate < 0.0 || receptor.normalization_rate > 1.0) { return 2u; }
  return select(0u,1u,receptor.learning_rate != 0.0
      || receptor.sleep_replay_rate != 0.0 || receptor.normalization_rate != 0.0);
}

fn u32_mul_overflows(a:u32, b:u32) -> bool {
  let a_hi = a >> 16u;
  let b_hi = b >> 16u;
  if (a_hi != 0u && b_hi != 0u) { return true; }
  let a_lo = a & 0xffffu;
  let b_lo = b & 0xffffu;
  let mid = a_hi * b_lo + b_hi * a_lo;
  if (mid >= 0x10000u) { return true; }
  let lo = a_lo * b_lo;
  let hi = mid << 16u;
  return lo > 0xffffffffu - hi;
}

fn sleep_transaction_valid(
  header:GpuSleepHeader,
  brain:GpuBrainSlotRecord,
  extension:GpuBrainSlotExtensionRecord,
  learning:GpuSlotLearningStateRecord,
) -> bool {
  if (extension.schema_version != GPU_CLOSED_LOOP_LAYOUT_VERSION
      || extension.sleep_parameter_offset == 0xffffffffu
      || !consolidate_immutable_plan_span_within(extension.sleep_parameter_offset,8u)
      || !consolidate_frame_span_within(header.request_offset,44u)
      || brain.recurrent_synapse_count > brain.synapse_count
      || learning.replay_event_capacity > 0xffffffffu / 28u
      || learning.replay_span_count > 0xffffffffu / 4u
      || header.replay_event_count > 0xffffffffu / 28u
      || header.replay_span_count > 0xffffffffu / 4u
      || learning.replay_span_count == 0u
      || u32_mul_overflows(learning.replay_event_capacity, learning.replay_span_count)) {
    return false;
  }
  let req_base = header.request_offset;
  let parameters = load_sleep_parameter(extension.sleep_parameter_offset);
  if (header.schema_version != GPU_SLEEP_SCHEMA_VERSION
      || header.flags != 0u || header.reserved != 0u
      || header.class_id != brain.class_id
      || header.slot != brain.slot || header.slot_generation != brain.slot_generation
      || header.brain_slot_index != brain.slot
      || header.synapse_count != brain.synapse_count
      || header.replay_event_count > frame_payload_words[req_base + 32u]
      || header.replay_sample_count > frame_payload_words[req_base + 33u]
      || header.replay_span_count != learning.replay_span_count) {
    return false;
  }
  var hash_ok = true;
  for (var i = 0u; i < 8u; i += 1u) {
    if (frame_payload_words[req_base + 4u + i] != phenotype_identities[header.brain_slot_index].phenotype_hash[i]) {
      hash_ok = false;
    }
  }
  if (frame_payload_words[req_base] != GPU_SLEEP_SCHEMA_VERSION
      || frame_payload_words[req_base + 1u] != 0u
      || frame_payload_words[req_base + 2u] != header.cycle_id_lo
      || frame_payload_words[req_base + 3u] != header.cycle_id_hi
      || frame_payload_words[req_base + 12u] != learning.active_weight_generation_lo
      || frame_payload_words[req_base + 13u] != learning.active_weight_generation_hi
      || !hash_ok
      || frame_payload_words[req_base + 32u] != learning.replay_event_capacity
      || frame_payload_words[req_base + 33u] != learning.replay_sample_capacity
      || frame_payload_words[req_base + 42u] != 0u
      || frame_payload_words[req_base + 43u] != 0u) {
    return false;
  }
  if (parameters.schema_version != GPU_SLEEP_SCHEMA_VERSION
      || !consolidate_finite(parameters.staging_rate)
      || !consolidate_finite(parameters.weight_limit)
      || !consolidate_finite(parameters.fast_decay_rate)
      || parameters.staging_rate <= 0.0 || parameters.staging_rate > 1.0
      || parameters.weight_limit <= 0.0 || parameters.weight_limit > 8.0
      || parameters.fast_decay_rate < 0.0 || parameters.fast_decay_rate > 1.0
      || parameters.eligibility_reset_policy != 1u
      || parameters.replay_consume_policy != 1u
      || any(parameters.reserved != vec2<u32>(0u))) {
    return false;
  }
  if (learning.schema_version != GPU_LEARNING_SCHEMA_VERSION
      || learning.active_weight_bank > 1u
      || learning.active_eligibility_bank > 1u
      || learning.pending_valid != 0u
      || learning.replay_cursor >= learning.replay_event_capacity
      || learning.replay_event_count != header.replay_event_count) {
    return false;
  }
  let m_limit = arrayLength(&mutable_state_words);
  let f_limit = arrayLength(&frame_payload_words);
  let decoder_count = brain.synapse_count - brain.recurrent_synapse_count;
  var ok = true;
  ok = ok && brain.lifetime_weight_offset <= m_limit && brain.synapse_count <= m_limit - brain.lifetime_weight_offset;
  ok = ok && brain.fast_weight_offset <= m_limit && brain.synapse_count <= m_limit - brain.fast_weight_offset;
  ok = ok && extension.lifetime_bank_1_offset <= m_limit && brain.synapse_count <= m_limit - extension.lifetime_bank_1_offset;
  ok = ok && extension.fast_bank_1_offset <= m_limit && brain.synapse_count <= m_limit - extension.fast_bank_1_offset;
  ok = ok && brain.reserved[2] > brain.fast_weight_offset && extension.fast_bank_1_offset >= brain.fast_weight_offset;
  let eff_0 = brain.reserved[2];
  let eff_1 = eff_0 + (extension.fast_bank_1_offset - brain.fast_weight_offset);
  ok = ok && eff_0 <= m_limit && brain.synapse_count <= m_limit - eff_0;
  ok = ok && eff_1 <= m_limit && brain.synapse_count <= m_limit - eff_1;
  ok = ok && brain.recurrent_eligibility_offset <= m_limit && brain.recurrent_synapse_count <= m_limit - brain.recurrent_eligibility_offset;
  ok = ok && extension.recurrent_eligibility_bank_1_offset <= m_limit && brain.recurrent_synapse_count <= m_limit - extension.recurrent_eligibility_bank_1_offset;
  ok = ok && brain.decoder_eligibility_offset <= m_limit && decoder_count <= m_limit - brain.decoder_eligibility_offset;
  ok = ok && extension.decoder_eligibility_bank_1_offset <= m_limit && decoder_count <= m_limit - extension.decoder_eligibility_bank_1_offset;
  ok = ok && learning.replay_event_rows_offset <= m_limit && (learning.replay_event_capacity*28u) <= m_limit - learning.replay_event_rows_offset;
  ok = ok && learning.replay_sample_offset <= m_limit && learning.replay_sample_capacity <= m_limit - learning.replay_sample_offset;
  ok = ok && learning.replay_span_offset <= m_limit && (learning.replay_span_count*4u) <= m_limit - learning.replay_span_offset;
  ok = ok && extension.pending_eligibility_offset <= m_limit && SLEEP_PENDING_ELIGIBILITY_WORDS <= m_limit - extension.pending_eligibility_offset;
  ok = ok && header.request_offset <= f_limit && 44u <= f_limit - header.request_offset;
  ok = ok && header.replay_event_offset <= f_limit && (header.replay_event_count*28u) <= f_limit - header.replay_event_offset;
  ok = ok && header.replay_span_offset <= f_limit && (header.replay_span_count*4u) <= f_limit - header.replay_span_offset;
  ok = ok && header.replay_sample_offset <= f_limit && header.replay_sample_count <= f_limit - header.replay_sample_offset;
  return ok;
}

@compute @workgroup_size(1)
fn initialize_sleep_transaction(@builtin(global_invocation_id) gid:vec3<u32>) {
  let header = load_sleep_header(gid.y*20u);
  if (header.brain_slot_index >= arrayLength(&brain_slots)
      || !atomic_span_within(header.completion_offset,16u)) { return; }
  let brain = brain_slots[header.brain_slot_index];
  let completion = header.completion_offset;
  var prepared = false;
  var out_w4 = 0u;
  var out_w5 = 0u;
  var out_w6 = 0u;
  var out_w7 = 0u;
  var out_w8 = 0u;
  var out_w9 = 0u;
  var out_w12 = 0u;
  var out_w13 = 0u;
  if (state_span_within(brain.extension_record_offset,20u)) {
    let extension = load_slot_extension(brain);
    if (state_span_within(extension.learning_state_offset,24u)
        && extension.sleep_parameter_offset != 0xffffffffu
        && consolidate_immutable_plan_span_within(extension.sleep_parameter_offset,8u)
        && consolidate_frame_span_within(header.request_offset,44u)) {
      let learning = load_slot_learning_state(extension);
      let exp_out_lo = frame_payload_words[header.request_offset + 14u];
      let exp_out_hi = frame_payload_words[header.request_offset + 15u];
      if (sleep_transaction_valid(header,brain,extension,learning)
          && (exp_out_lo != 0u || exp_out_hi != 0u)) {
        prepared = true;
        out_w4 = learning.active_weight_generation_lo;
        out_w5 = learning.active_weight_generation_hi;
        out_w6 = exp_out_lo;
        out_w7 = exp_out_hi;
        out_w8 = learning.active_weight_bank^1u;
        out_w9 = header.replay_span_count;
        out_w12 = header.job_id_lo;
        out_w13 = header.job_id_hi;
      }
    }
  }
  var out_words = array<u32, 16>(
    GPU_SLEEP_SCHEMA_VERSION, brain.slot, brain.slot_generation, SLEEP_STATUS_REJECTED,
    out_w4, out_w5, out_w6, out_w7,
    out_w8, out_w9, 0u, 0u,
    out_w12, out_w13, 0u, 0u
  );
  if (prepared) {
    out_words[3] = SLEEP_STATUS_PREPARED;
  }
  for (var i = 0u; i < 16u; i += 1u) {
    if (i != 3u) {
      store_atomic_u32(completion + i, out_words[i]);
    }
  }
  storageBarrier();
  store_atomic_u32(completion + 3u, out_words[3]);
}

@compute @workgroup_size(64)
fn refresh_effective_weights(@builtin(global_invocation_id) gid:vec3<u32>) {
  let slot_index = dispatch_header_words[0];
  if (slot_index >= arrayLength(&brain_slots)) { return; }
  let brain = brain_slots[slot_index];
  if (brain.schema_version != GPU_CLOSED_LOOP_LAYOUT_VERSION || brain.slot_generation == 0u) { return; }
  let synapse = gid.x;
  if (synapse >= brain.synapse_count) { return; }
  let extension_base = brain.extension_record_offset;
  let fast_bank_1 = load_state_u32(extension_base + 10u);
  let lifetime_bank_1 = load_state_u32(extension_base + 11u);
  let eff_0 = brain.reserved[2];
  let eff_1 = eff_0 + (fast_bank_1 - brain.fast_weight_offset);
  let genetic = bitcast<f32>(immutable_weight_words[brain.genetic_weight_offset + synapse]);
  let alpha = bitcast<f32>(immutable_weight_words[brain.alpha_offset + synapse]);
  let l0 = load_state_f32(brain.lifetime_weight_offset + synapse);
  let f0 = load_state_f32(brain.fast_weight_offset + synapse);
  let l1 = load_state_f32(lifetime_bank_1 + synapse);
  let f1 = load_state_f32(fast_bank_1 + synapse);
  store_state_f32(eff_0 + synapse, canonicalize_state_zero(genetic + l0 + alpha * f0));
  store_state_f32(eff_1 + synapse, canonicalize_state_zero(genetic + l1 + alpha * f1));
}

@compute @workgroup_size(64)
fn copy_sleep_weight_banks(@builtin(global_invocation_id) gid:vec3<u32>) {
  let header = load_sleep_header(gid.y*20u);
  if (header.brain_slot_index >= arrayLength(&brain_slots)
      || load_atomic_u32(header.completion_offset+3u) != SLEEP_STATUS_PREPARED
      || gid.x >= header.synapse_count) { return; }
  let brain = brain_slots[header.brain_slot_index];
  let bank_pair = load_weight_bank_pair_direct(brain);
  let active_bases = bank_pair.active_bases;
  let inactive = bank_pair.staging_bases;
  let lifetime = load_state_f32(active_bases.lifetime+gid.x);
  let fast = load_state_f32(active_bases.fast+gid.x);
  let effective = load_state_f32(active_bases.effective+gid.x);
  if (!consolidate_finite(lifetime) || !consolidate_finite(fast) || !consolidate_finite(effective)) {
    consolidate_reject(header.completion_offset); return;
  }
  store_state_f32(inactive.lifetime+gid.x,lifetime);
  store_state_f32(inactive.fast+gid.x,canonicalize_state_zero(fast));
  store_state_f32(inactive.effective+gid.x,canonicalize_state_zero(effective));
}

var<workgroup> wg_con_header: GpuSleepHeader;
var<workgroup> wg_con_brain: GpuBrainSlotRecord;
var<workgroup> wg_con_ext: GpuBrainSlotExtensionRecord;
var<workgroup> wg_con_params: GpuSleepParameterRecord;
var<workgroup> wg_con_weight_pair: GpuWeightBankPair;
var<workgroup> wg_con_valid: u32;

@compute @workgroup_size(64)
fn consolidate_fast_weights(
  @builtin(global_invocation_id) gid:vec3<u32>,
  @builtin(local_invocation_id) lid:vec3<u32>,
) {
  if (lid.x == 0u) {
    wg_con_valid = 0u;
    let header = load_sleep_header(gid.y*20u);
    if (header.brain_slot_index < arrayLength(&brain_slots)
        && load_atomic_u32(header.completion_offset+3u) == SLEEP_STATUS_PREPARED) {
      let brain = brain_slots[header.brain_slot_index];
      let ext = load_slot_extension(brain);
      wg_con_header = header;
      wg_con_brain = brain;
      wg_con_ext = ext;
      wg_con_params = load_sleep_parameter(ext.sleep_parameter_offset);
      wg_con_weight_pair = load_weight_bank_pair_direct(brain);
      wg_con_valid = 1u;
    }
  }
  workgroupBarrier();
  if (wg_con_valid == 0u) { return; }

  let header = wg_con_header;
  if (gid.x >= header.synapse_count) { return; }
  let brain = wg_con_brain;
  let delta_mode = consolidate_synapse_delta_mode(wg_con_ext, gid.x);
  if (delta_mode == 2u) { consolidate_reject(header.completion_offset); return; }
  // The copy pass already staged both coefficients. Leave disabled rows intact.
  if (delta_mode == 0u) { return; }
  let parameters = wg_con_params;
  let active_bases = wg_con_weight_pair.active_bases;
  let inactive = wg_con_weight_pair.staging_bases;
  let active_lifetime = load_state_f32(active_bases.lifetime+gid.x);
  let replayed_fast = load_state_f32(inactive.fast+gid.x);
  let promoted = parameters.staging_rate*replayed_fast;
  let next_lifetime = clamp(active_lifetime+promoted,-parameters.weight_limit,parameters.weight_limit);
  let next_fast = replayed_fast*(1.0-parameters.fast_decay_rate);
  if (!consolidate_finite(active_lifetime) || !consolidate_finite(replayed_fast)
      || !consolidate_finite(promoted) || !consolidate_finite(next_lifetime)
      || !consolidate_finite(next_fast)) {
    consolidate_reject(header.completion_offset); return;
  }
  let next_lifetime_canon = canonicalize_state_zero(next_lifetime);
  let next_fast_canon = canonicalize_state_zero(next_fast);
  let genetic = bitcast<f32>(immutable_weight_words[brain.genetic_weight_offset + gid.x]);
  let alpha = bitcast<f32>(immutable_weight_words[brain.alpha_offset + gid.x]);
  let next_effective = canonicalize_state_zero(genetic + next_lifetime_canon + alpha * next_fast_canon);
  store_state_f32(inactive.lifetime+gid.x, next_lifetime_canon);
  store_state_f32(inactive.fast+gid.x, next_fast_canon);
  store_state_f32(inactive.effective+gid.x, next_effective);
}

var<workgroup> wg_fin_header: GpuSleepHeader;
var<workgroup> wg_fin_ext: GpuBrainSlotExtensionRecord;
var<workgroup> wg_fin_params: GpuSleepParameterRecord;
var<workgroup> wg_fin_active: GpuWeightBankBases;
var<workgroup> wg_fin_inactive: GpuWeightBankBases;
var<workgroup> wg_fin_valid: u32;
var<workgroup> wg_fin_err: atomic<u32>;
var<workgroup> wg_fin_promoted: array<f32, 64>;
var<workgroup> wg_fin_replay: array<f32, 64>;

@compute @workgroup_size(64)
fn finalize_sleep_staging(
  @builtin(global_invocation_id) gid:vec3<u32>,
  @builtin(local_invocation_id) lid:vec3<u32>,
) {
  let tid = lid.x;
  if (tid == 0u) {
    wg_fin_valid = 0u;
    atomicStore(&wg_fin_err, 0u);
    let header = load_sleep_header(gid.y*20u);
    if (header.brain_slot_index < arrayLength(&brain_slots)
        && atomic_span_within(header.completion_offset, 16u)) {
      if (load_atomic_u32(header.completion_offset+3u) == SLEEP_STATUS_PREPARED) {
        let brain = brain_slots[header.brain_slot_index];
        let extension = load_slot_extension(brain);
        let learning = load_slot_learning_state(extension);
        if (!sleep_transaction_valid(header, brain, extension, learning)) {
          consolidate_reject(header.completion_offset);
        } else {
          wg_fin_header = header;
          wg_fin_ext = extension;
          wg_fin_params = load_sleep_parameter(extension.sleep_parameter_offset);
          wg_fin_active = active_weight_bases(brain, extension, learning);
          wg_fin_inactive = inactive_weight_bases(brain, extension, learning);
          wg_fin_valid = 1u;
        }
      }
    }
  }
  workgroupBarrier();
  if (wg_fin_valid == 0u) { return; }

  let header = wg_fin_header;
  let extension = wg_fin_ext;
  let parameters = wg_fin_params;
  let active_bases = wg_fin_active;
  let inactive_bases = wg_fin_inactive;
  var promoted_fast_l1 = 0.0;
  var replay_induced_fast_l1 = 0.0;
  for (var synapse = tid; synapse < header.synapse_count; synapse += 64u) {
    let active_lifetime = load_state_f32(active_bases.lifetime+synapse);
    let active_fast = load_state_f32(active_bases.fast+synapse);
    let staged_lifetime = load_state_f32(inactive_bases.lifetime+synapse);
    let staged_fast = load_state_f32(inactive_bases.fast+synapse);
    let delta_mode = consolidate_synapse_delta_mode(extension, synapse);
    if (delta_mode == 2u) { atomicStore(&wg_fin_err, 1u); break; }
    let no_replay_fast = select(active_fast*(1.0-parameters.fast_decay_rate), active_fast, delta_mode == 0u);
    if (!consolidate_finite(active_lifetime) || !consolidate_finite(active_fast)
        || !consolidate_finite(staged_lifetime) || !consolidate_finite(staged_fast)
        || !consolidate_finite(no_replay_fast)) {
      atomicStore(&wg_fin_err, 1u); break;
    }
    promoted_fast_l1 += abs(staged_lifetime - active_lifetime);
    replay_induced_fast_l1 += abs(staged_fast - no_replay_fast);
  }
  wg_fin_promoted[tid] = promoted_fast_l1;
  wg_fin_replay[tid] = replay_induced_fast_l1;
  workgroupBarrier();
  if (atomicLoad(&wg_fin_err) != 0u) {
    if (tid == 0u) { consolidate_reject(header.completion_offset); }
    return;
  }
  for (var stride = 32u; stride > 0u; stride >>= 1u) {
    if (tid < stride) {
      let other = tid + stride;
      wg_fin_promoted[tid] += wg_fin_promoted[other];
      wg_fin_replay[tid] += wg_fin_replay[other];
    }
    workgroupBarrier();
  }
  var ok = 1u;
  if (tid == 0u) {
    let total_promoted = wg_fin_promoted[0];
    let total_replay = wg_fin_replay[0];
    if (!consolidate_finite(total_promoted) || !consolidate_finite(total_replay)) {
      consolidate_reject(header.completion_offset);
      ok = 0u;
    } else {
      // Sleep receipts use conservative Q12 upper rounding so a causal sub-quantum
      // GPU update is never reported as no update.
      store_atomic_u32(header.completion_offset+10u, u32(ceil(total_promoted*SLEEP_DIAGNOSTIC_Q12)));
      store_atomic_u32(header.completion_offset+11u, u32(ceil(total_replay*SLEEP_DIAGNOSTIC_Q12)));
    }
  }
  storageBarrier();
  if (tid == 0u && ok != 0u) {
    store_atomic_u32(header.completion_offset+3u, SLEEP_STATUS_SUCCESS);
  }
}

@compute @workgroup_size(64)
fn reset_sleep_mutable_state(@builtin(global_invocation_id) gid:vec3<u32>) {
  let header = load_sleep_header(gid.y*20u);
  if (header.brain_slot_index >= arrayLength(&brain_slots)
      || load_atomic_u32(header.completion_offset+3u) != SLEEP_STATUS_SUCCESS) { return; }
  let brain = brain_slots[header.brain_slot_index];
  let extension = load_slot_extension(brain);
  let learning = load_slot_learning_state(extension);
  if (!sleep_transaction_valid(header,brain,extension,learning)) {
    consolidate_reject(header.completion_offset); return;
  }
  let index = gid.x;
  let decoder_count = brain.synapse_count-brain.recurrent_synapse_count;
  if (index < brain.recurrent_synapse_count) {
    store_state_u32(brain.recurrent_eligibility_offset+index,0u);
    store_state_u32(extension.recurrent_eligibility_bank_1_offset+index,0u);
  }
  if (index < decoder_count) {
    store_state_u32(brain.decoder_eligibility_offset+index,0u);
    store_state_u32(extension.decoder_eligibility_bank_1_offset+index,0u);
  }
  if (index < learning.replay_event_capacity*28u) {
    store_state_u32(learning.replay_event_rows_offset+index,0u);
  }
  if (index < learning.replay_sample_capacity) {
    store_state_u32(learning.replay_sample_offset+index,0u);
  }
  if (index < learning.replay_span_count) {
    store_state_u32(learning.replay_span_offset+index*4u+2u,0u);
  }
  if (index < SLEEP_PENDING_ELIGIBILITY_WORDS) {
    store_state_u32(extension.pending_eligibility_offset+index,0u);
  }
}

@compute @workgroup_size(1)
fn finalize_sleep_commit(@builtin(global_invocation_id) gid:vec3<u32>) {
  let header = load_sleep_header(gid.y*20u);
  if (header.brain_slot_index >= arrayLength(&brain_slots)
      || load_atomic_u32(header.completion_offset+3u) != SLEEP_STATUS_SUCCESS) { return; }
  let brain = brain_slots[header.brain_slot_index];
  let extension = load_slot_extension(brain);
  let learning = load_slot_learning_state(extension);
  if (!sleep_transaction_valid(header,brain,extension,learning)) {
    consolidate_reject(header.completion_offset); return;
  }
  let next_eligibility_lo = learning.active_eligibility_generation_lo+1u;
  let next_eligibility_hi = learning.active_eligibility_generation_hi+select(0u,1u,next_eligibility_lo==0u);
  let next_replay_lo = learning.replay_generation_lo+1u;
  let next_replay_hi = learning.replay_generation_hi+select(0u,1u,next_replay_lo==0u);
  let next_transaction_lo = learning.transaction_generation_lo+1u;
  let next_transaction_hi = learning.transaction_generation_hi+select(0u,1u,next_transaction_lo==0u);
  if ((next_eligibility_lo == 0u && next_eligibility_hi == 0u)
      || (next_replay_lo == 0u && next_replay_hi == 0u)
      || (next_transaction_lo == 0u && next_transaction_hi == 0u)) {
    consolidate_reject(header.completion_offset); return;
  }
  let state = extension.learning_state_offset;
  let exp_out_lo = frame_payload_words[header.request_offset + 14u];
  let exp_out_hi = frame_payload_words[header.request_offset + 15u];
  store_state_u32(state+1u,learning.active_weight_bank^1u);
  store_state_u32(state+2u,0u);
  store_state_u32(state+3u,0u);
  store_state_u32(state+4u,exp_out_lo);
  store_state_u32(state+5u,exp_out_hi);
  store_state_u32(state+6u,next_eligibility_lo);
  store_state_u32(state+7u,next_eligibility_hi);
  store_state_u32(state+8u,0u); store_state_u32(state+9u,0u);
  store_state_u32(state+10u,next_replay_lo); store_state_u32(state+11u,next_replay_hi);
  store_state_u32(state+12u,0u); store_state_u32(state+13u,0u);
  store_state_u32(state+22u,next_transaction_lo); store_state_u32(state+23u,next_transaction_hi);
  storageBarrier();
  store_atomic_u32(header.completion_offset+3u,SLEEP_STATUS_COMMITTED);
}
