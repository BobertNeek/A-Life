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
  return value == value && abs(value) <= 3.402823466e+38;
}

fn consolidate_pair_equal(lo:u32, hi:u32, expected_lo:u32, expected_hi:u32) -> bool {
  return lo == expected_lo && hi == expected_hi;
}

fn consolidate_array8_equal(left:array<u32,8>, right:array<u32,8>) -> bool {
  for (var index=0u; index<8u; index+=1u) {
    if (left[index] != right[index]) { return false; }
  }
  return true;
}

fn consolidate_reject(completion:u32) {
  if (state_span_within(completion,16u)) {
    atomicMax(&mutable_state_words[completion+3u],SLEEP_STATUS_REJECTED);
  }
}

fn sleep_transaction_valid(
  header:GpuSleepHeader,
  brain:GpuBrainSlotRecord,
  extension:GpuBrainSlotExtensionRecord,
  learning:GpuSlotLearningStateRecord,
  request:GpuConsolidationRequestRecord,
) -> bool {
  let parameters = load_sleep_parameter(extension.sleep_parameter_offset);
  return header.schema_version == GPU_SLEEP_SCHEMA_VERSION
    && header.flags == 0u && header.reserved == 0u
    && header.class_id == brain.class_id
    && header.slot == brain.slot && header.slot_generation == brain.slot_generation
    && header.brain_slot_index == brain.slot
    && header.synapse_count == brain.synapse_count
    && header.replay_event_count <= request.max_replay_events
    && header.replay_sample_count <= request.max_replay_eligibility_samples
    && header.replay_span_count == learning.replay_span_count
    && request.schema_version == GPU_SLEEP_SCHEMA_VERSION
    && request.request_flags == 0u
    && request.cycle_id_lo == header.cycle_id_lo && request.cycle_id_hi == header.cycle_id_hi
    && request.input_generation_lo == learning.active_weight_generation_lo
    && request.input_generation_hi == learning.active_weight_generation_hi
    && consolidate_array8_equal(request.phenotype_hash,phenotype_identities[header.brain_slot_index].phenotype_hash)
    && request.max_replay_events == learning.replay_event_capacity
    && request.max_replay_eligibility_samples == learning.replay_sample_capacity
    && request.reserved_tail.x == 0u && request.reserved_tail.y == 0u
    && extension.schema_version == GPU_CLOSED_LOOP_LAYOUT_VERSION
    && extension.sleep_parameter_offset != 0xffffffffu
    && consolidate_immutable_plan_span_within(extension.sleep_parameter_offset,8u)
    && parameters.schema_version == GPU_SLEEP_SCHEMA_VERSION
    && consolidate_finite(parameters.staging_rate)
    && consolidate_finite(parameters.weight_limit)
    && consolidate_finite(parameters.fast_decay_rate)
    && parameters.staging_rate > 0.0 && parameters.staging_rate <= 1.0
    && parameters.weight_limit > 0.0 && parameters.weight_limit <= 8.0
    && parameters.fast_decay_rate >= 0.0 && parameters.fast_decay_rate <= 1.0
    && parameters.eligibility_reset_policy == 1u
    && parameters.replay_consume_policy == 1u
    && all(parameters.reserved == vec2<u32>(0u))
    && learning.schema_version == GPU_LEARNING_SCHEMA_VERSION
    && learning.active_weight_bank <= 1u
    && learning.active_eligibility_bank <= 1u
    && learning.pending_valid == 0u
    && learning.replay_cursor < learning.replay_event_capacity
    && learning.replay_event_count == header.replay_event_count
    && state_span_within(brain.lifetime_weight_offset,brain.synapse_count)
    && state_span_within(brain.fast_weight_offset,brain.synapse_count)
    && state_span_within(extension.lifetime_bank_1_offset,brain.synapse_count)
    && state_span_within(extension.fast_bank_1_offset,brain.synapse_count)
    && state_span_within(brain.recurrent_eligibility_offset,brain.recurrent_synapse_count)
    && state_span_within(extension.recurrent_eligibility_bank_1_offset,brain.recurrent_synapse_count)
    && state_span_within(brain.decoder_eligibility_offset,brain.synapse_count-brain.recurrent_synapse_count)
    && state_span_within(extension.decoder_eligibility_bank_1_offset,brain.synapse_count-brain.recurrent_synapse_count)
    && state_span_within(learning.replay_event_rows_offset,learning.replay_event_capacity*24u)
    && state_span_within(learning.replay_sample_offset,learning.replay_sample_capacity)
    && state_span_within(learning.replay_span_offset,learning.replay_span_count*4u)
    && state_span_within(extension.pending_eligibility_offset,SLEEP_PENDING_ELIGIBILITY_WORDS)
    && consolidate_frame_span_within(header.request_offset,44u)
    && consolidate_frame_span_within(header.replay_event_offset,header.replay_event_count*24u)
    && consolidate_frame_span_within(header.replay_span_offset,header.replay_span_count*4u)
    && consolidate_frame_span_within(header.replay_sample_offset,header.replay_sample_count);
}

@compute @workgroup_size(1)
fn initialize_sleep_transaction(@builtin(global_invocation_id) gid:vec3<u32>) {
  let header = load_sleep_header(gid.y*20u);
  if (header.brain_slot_index >= arrayLength(&brain_slots)
      || !state_span_within(header.completion_offset,16u)) { return; }
  let brain = brain_slots[header.brain_slot_index];
  let completion = header.completion_offset;
  for (var word=0u; word<16u; word+=1u) { store_state_u32(completion+word,0u); }
  store_state_u32(completion,GPU_SLEEP_SCHEMA_VERSION);
  store_state_u32(completion+1u,brain.slot);
  store_state_u32(completion+2u,brain.slot_generation);
  let extension = load_slot_extension(brain);
  let learning = load_slot_learning_state(extension);
  let request = load_consolidation_request(header.request_offset);
  if (!sleep_transaction_valid(header,brain,extension,learning,request)
      || request.expected_output_generation_lo == 0u && request.expected_output_generation_hi == 0u) {
    consolidate_reject(completion); return;
  }
  store_state_u32(completion+4u,learning.active_weight_generation_lo);
  store_state_u32(completion+5u,learning.active_weight_generation_hi);
  store_state_u32(completion+6u,request.expected_output_generation_lo);
  store_state_u32(completion+7u,request.expected_output_generation_hi);
  store_state_u32(completion+8u,learning.active_weight_bank^1u);
  store_state_u32(completion+9u,header.replay_span_count);
  store_state_u32(completion+12u,header.job_id_lo);
  store_state_u32(completion+13u,header.job_id_hi);
  storageBarrier();
  store_state_u32(completion+3u,SLEEP_STATUS_PREPARED);
}

@compute @workgroup_size(64)
fn copy_sleep_weight_banks(@builtin(global_invocation_id) gid:vec3<u32>) {
  let header = load_sleep_header(gid.y*20u);
  if (header.brain_slot_index >= arrayLength(&brain_slots)
      || load_state_u32(header.completion_offset+3u) != SLEEP_STATUS_PREPARED
      || gid.x >= header.synapse_count) { return; }
  let brain = brain_slots[header.brain_slot_index];
  let extension = load_slot_extension(brain);
  let learning = load_slot_learning_state(extension);
  let active_bases = active_weight_bases(brain,extension,learning);
  let inactive = inactive_weight_bases(brain,extension,learning);
  let lifetime = load_state_f32(active_bases.lifetime+gid.x);
  let fast = load_state_f32(active_bases.fast+gid.x);
  if (!consolidate_finite(lifetime) || !consolidate_finite(fast)) {
    consolidate_reject(header.completion_offset); return;
  }
  store_state_f32(inactive.lifetime+gid.x,lifetime);
  store_state_f32(inactive.fast+gid.x,fast);
}

@compute @workgroup_size(64)
fn consolidate_fast_weights(@builtin(global_invocation_id) gid:vec3<u32>) {
  let header = load_sleep_header(gid.y*20u);
  if (header.brain_slot_index >= arrayLength(&brain_slots)
      || load_state_u32(header.completion_offset+3u) != SLEEP_STATUS_PREPARED
      || gid.x >= header.synapse_count) { return; }
  let brain = brain_slots[header.brain_slot_index];
  let extension = load_slot_extension(brain);
  let learning = load_slot_learning_state(extension);
  let parameters = load_sleep_parameter(extension.sleep_parameter_offset);
  let active_bases = active_weight_bases(brain,extension,learning);
  let inactive = inactive_weight_bases(brain,extension,learning);
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
  store_state_f32(inactive.lifetime+gid.x,next_lifetime);
  store_state_f32(inactive.fast+gid.x,next_fast);
}

@compute @workgroup_size(1)
fn finalize_sleep_staging(@builtin(global_invocation_id) gid:vec3<u32>) {
  let header = load_sleep_header(gid.y*20u);
  if (header.brain_slot_index >= arrayLength(&brain_slots)
      || !state_span_within(header.completion_offset,16u)) { return; }
  let status = load_state_u32(header.completion_offset+3u);
  if (status != SLEEP_STATUS_PREPARED) { return; }
  let brain = brain_slots[header.brain_slot_index];
  let extension = load_slot_extension(brain);
  let learning = load_slot_learning_state(extension);
  let request = load_consolidation_request(header.request_offset);
  if (!sleep_transaction_valid(header,brain,extension,learning,request)) {
    consolidate_reject(header.completion_offset); return;
  }
  let parameters = load_sleep_parameter(extension.sleep_parameter_offset);
  let active_bases = active_weight_bases(brain,extension,learning);
  let inactive_bases = inactive_weight_bases(brain,extension,learning);
  var promoted_fast_l1 = 0.0;
  var replay_induced_fast_l1 = 0.0;
  for (var synapse=0u; synapse<header.synapse_count; synapse+=1u) {
    let active_lifetime = load_state_f32(active_bases.lifetime+synapse);
    let active_fast = load_state_f32(active_bases.fast+synapse);
    let staged_lifetime = load_state_f32(inactive_bases.lifetime+synapse);
    let staged_fast = load_state_f32(inactive_bases.fast+synapse);
    let no_replay_fast = active_fast*(1.0-parameters.fast_decay_rate);
    if (!consolidate_finite(active_lifetime) || !consolidate_finite(active_fast)
        || !consolidate_finite(staged_lifetime) || !consolidate_finite(staged_fast)
        || !consolidate_finite(no_replay_fast)) {
      consolidate_reject(header.completion_offset); return;
    }
    promoted_fast_l1 += abs(staged_lifetime-active_lifetime);
    replay_induced_fast_l1 += abs(staged_fast-no_replay_fast);
  }
  if (!consolidate_finite(promoted_fast_l1) || !consolidate_finite(replay_induced_fast_l1)) {
    consolidate_reject(header.completion_offset); return;
  }
  // Sleep receipts use conservative Q12 upper rounding so a causal sub-quantum
  // GPU update is never reported as no update.
  store_state_u32(header.completion_offset+10u,u32(ceil(promoted_fast_l1*SLEEP_DIAGNOSTIC_Q12)));
  store_state_u32(header.completion_offset+11u,u32(ceil(replay_induced_fast_l1*SLEEP_DIAGNOSTIC_Q12)));
  storageBarrier();
  store_state_u32(header.completion_offset+3u,SLEEP_STATUS_SUCCESS);
}

@compute @workgroup_size(64)
fn reset_sleep_mutable_state(@builtin(global_invocation_id) gid:vec3<u32>) {
  let header = load_sleep_header(gid.y*20u);
  if (header.brain_slot_index >= arrayLength(&brain_slots)
      || load_state_u32(header.completion_offset+3u) != SLEEP_STATUS_SUCCESS) { return; }
  let brain = brain_slots[header.brain_slot_index];
  let extension = load_slot_extension(brain);
  let learning = load_slot_learning_state(extension);
  let request = load_consolidation_request(header.request_offset);
  if (!sleep_transaction_valid(header,brain,extension,learning,request)) {
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
  if (index < learning.replay_event_capacity*24u) {
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
      || load_state_u32(header.completion_offset+3u) != SLEEP_STATUS_SUCCESS) { return; }
  let brain = brain_slots[header.brain_slot_index];
  let extension = load_slot_extension(brain);
  let learning = load_slot_learning_state(extension);
  let request = load_consolidation_request(header.request_offset);
  if (!sleep_transaction_valid(header,brain,extension,learning,request)) {
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
  store_state_u32(state+1u,learning.active_weight_bank^1u);
  store_state_u32(state+2u,0u);
  store_state_u32(state+3u,0u);
  store_state_u32(state+4u,request.expected_output_generation_lo);
  store_state_u32(state+5u,request.expected_output_generation_hi);
  store_state_u32(state+6u,next_eligibility_lo);
  store_state_u32(state+7u,next_eligibility_hi);
  store_state_u32(state+8u,0u); store_state_u32(state+9u,0u);
  store_state_u32(state+10u,next_replay_lo); store_state_u32(state+11u,next_replay_hi);
  store_state_u32(state+12u,0u); store_state_u32(state+13u,0u);
  store_state_u32(state+22u,next_transaction_lo); store_state_u32(state+23u,next_transaction_hi);
  storageBarrier();
  store_state_u32(header.completion_offset+3u,SLEEP_STATUS_COMMITTED);
}
