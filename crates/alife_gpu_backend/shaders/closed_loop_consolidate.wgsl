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

fn sleep_transaction_valid(
  header:GpuSleepHeader,
  brain:GpuBrainSlotRecord,
  extension:GpuBrainSlotExtensionRecord,
  learning:GpuSlotLearningStateRecord,
  request:GpuConsolidationRequestRecord,
) -> bool {
  if (extension.schema_version != GPU_CLOSED_LOOP_LAYOUT_VERSION
      || extension.sleep_parameter_offset == 0xffffffffu
      || !consolidate_immutable_plan_span_within(extension.sleep_parameter_offset,8u)
      || brain.recurrent_synapse_count > brain.synapse_count
      || learning.replay_event_capacity > 0xffffffffu / 28u
      || learning.replay_span_count > 0xffffffffu / 4u
      || header.replay_event_count > 0xffffffffu / 28u
      || header.replay_span_count > 0xffffffffu / 4u
      || learning.replay_span_count == 0u
      || learning.replay_event_capacity > 0xffffffffu / learning.replay_span_count) {
    return false;
  }
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
    && state_span_within(learning.replay_event_rows_offset,learning.replay_event_capacity*28u)
    && state_span_within(learning.replay_sample_offset,learning.replay_sample_capacity)
    && state_span_within(learning.replay_span_offset,learning.replay_span_count*4u)
    && state_span_within(extension.pending_eligibility_offset,SLEEP_PENDING_ELIGIBILITY_WORDS)
    && consolidate_frame_span_within(header.request_offset,44u)
    && consolidate_frame_span_within(header.replay_event_offset,header.replay_event_count*28u)
    && consolidate_frame_span_within(header.replay_span_offset,header.replay_span_count*4u)
    && consolidate_frame_span_within(header.replay_sample_offset,header.replay_sample_count);
}

// Scalar-only initializer: preserve one dispatch, all validation and publication
// ordering without constructing or passing aggregate ABI records. Mutable fields
// are snapshotted AFTER the original completion prelude, before final writes.
@compute @workgroup_size(1)
fn initialize_sleep_transaction(@builtin(global_invocation_id) gid:vec3<u32>) {
  // Every invocation reaches the same barrier. Validation and all existing
  // writes remain in order; rejected paths never publish through default 0.
  var completion = 0u;
  var prepared = false;
  loop {
    let header_base = gid.y*20u;
    let h_schema_version = dispatch_header_words[header_base+0u];
    let h_class_id = dispatch_header_words[header_base+1u];
    let h_slot = dispatch_header_words[header_base+2u];
    let h_slot_generation = dispatch_header_words[header_base+3u];
    let h_brain_slot_index = dispatch_header_words[header_base+4u];
    let h_request_offset = dispatch_header_words[header_base+5u];
    let h_replay_event_offset = dispatch_header_words[header_base+6u];
    let h_replay_event_count = dispatch_header_words[header_base+7u];
    let h_replay_span_offset = dispatch_header_words[header_base+8u];
    let h_replay_span_count = dispatch_header_words[header_base+9u];
    let h_replay_sample_offset = dispatch_header_words[header_base+10u];
    let h_replay_sample_count = dispatch_header_words[header_base+11u];
    let h_synapse_count = dispatch_header_words[header_base+12u];
    let h_completion_offset = dispatch_header_words[header_base+13u];
    let h_job_id_lo = dispatch_header_words[header_base+14u];
    let h_job_id_hi = dispatch_header_words[header_base+15u];
    let h_cycle_id_lo = dispatch_header_words[header_base+16u];
    let h_cycle_id_hi = dispatch_header_words[header_base+17u];
    let h_flags = dispatch_header_words[header_base+18u];
    let h_reserved = dispatch_header_words[header_base+19u];
    if (h_brain_slot_index >= arrayLength(&brain_slots)
        || !state_span_within(h_completion_offset,16u)) { break; }
    let b_class_id = brain_slots[h_brain_slot_index].class_id;
    let b_slot = brain_slots[h_brain_slot_index].slot;
    let b_slot_generation = brain_slots[h_brain_slot_index].slot_generation;
    let b_synapse_count = brain_slots[h_brain_slot_index].synapse_count;
    let b_recurrent_synapse_count = brain_slots[h_brain_slot_index].recurrent_synapse_count;
    let b_lifetime_weight_offset = brain_slots[h_brain_slot_index].lifetime_weight_offset;
    let b_fast_weight_offset = brain_slots[h_brain_slot_index].fast_weight_offset;
    let b_recurrent_eligibility_offset = brain_slots[h_brain_slot_index].recurrent_eligibility_offset;
    let b_decoder_eligibility_offset = brain_slots[h_brain_slot_index].decoder_eligibility_offset;
    let b_extension_record_offset = brain_slots[h_brain_slot_index].extension_record_offset;
    completion = h_completion_offset;
    for (var word=0u; word<16u; word+=1u) { store_state_u32(completion+word,0u); }
    store_state_u32(completion,GPU_SLEEP_SCHEMA_VERSION);
    store_state_u32(completion+1u,b_slot);
    store_state_u32(completion+2u,b_slot_generation);
    if (!state_span_within(b_extension_record_offset,20u)) { consolidate_reject(completion); break; }
    let e_schema_version = load_state_u32(b_extension_record_offset+0u);
    let e_recurrent_eligibility_bank_1_offset = load_state_u32(b_extension_record_offset+8u);
    let e_decoder_eligibility_bank_1_offset = load_state_u32(b_extension_record_offset+9u);
    let e_fast_bank_1_offset = load_state_u32(b_extension_record_offset+10u);
    let e_lifetime_bank_1_offset = load_state_u32(b_extension_record_offset+11u);
    let e_sleep_parameter_offset = load_state_u32(b_extension_record_offset+12u);
    let e_learning_state_offset = load_state_u32(b_extension_record_offset+15u);
    let e_pending_eligibility_offset = load_state_u32(b_extension_record_offset+16u);
    if (!state_span_within(e_learning_state_offset,24u)) { consolidate_reject(completion); break; }
    if (e_sleep_parameter_offset == 0xffffffffu) { consolidate_reject(completion); break; }
    if (!consolidate_immutable_plan_span_within(e_sleep_parameter_offset,8u)) { consolidate_reject(completion); break; }
    if (!consolidate_frame_span_within(h_request_offset,44u)) { consolidate_reject(completion); break; }
    let l_schema_version = load_state_u32(e_learning_state_offset+0u);
    let l_active_weight_bank = load_state_u32(e_learning_state_offset+1u);
    let l_active_eligibility_bank = load_state_u32(e_learning_state_offset+2u);
    let l_pending_valid = load_state_u32(e_learning_state_offset+3u);
    let l_active_weight_generation_lo = load_state_u32(e_learning_state_offset+4u);
    let l_active_weight_generation_hi = load_state_u32(e_learning_state_offset+5u);
    let l_replay_cursor = load_state_u32(e_learning_state_offset+12u);
    let l_replay_event_count = load_state_u32(e_learning_state_offset+13u);
    let l_replay_event_capacity = load_state_u32(e_learning_state_offset+14u);
    let l_replay_sample_capacity = load_state_u32(e_learning_state_offset+15u);
    let l_replay_span_count = load_state_u32(e_learning_state_offset+16u);
    let l_replay_event_rows_offset = load_state_u32(e_learning_state_offset+17u);
    let l_replay_sample_offset = load_state_u32(e_learning_state_offset+18u);
    let l_replay_span_offset = load_state_u32(e_learning_state_offset+19u);
    let r_schema_version = frame_payload_words[h_request_offset+0u];
    let r_request_flags = frame_payload_words[h_request_offset+1u];
    let r_cycle_id_lo = frame_payload_words[h_request_offset+2u];
    let r_cycle_id_hi = frame_payload_words[h_request_offset+3u];
    let r_input_generation_lo = frame_payload_words[h_request_offset+12u];
    let r_input_generation_hi = frame_payload_words[h_request_offset+13u];
    let r_expected_output_generation_lo = frame_payload_words[h_request_offset+14u];
    let r_expected_output_generation_hi = frame_payload_words[h_request_offset+15u];
    let r_max_replay_events = frame_payload_words[h_request_offset+32u];
    let r_max_replay_eligibility_samples = frame_payload_words[h_request_offset+33u];
    let r_reserved_tail_0 = frame_payload_words[h_request_offset+42u];
    let r_reserved_tail_1 = frame_payload_words[h_request_offset+43u];
    if (e_schema_version != GPU_CLOSED_LOOP_LAYOUT_VERSION) { consolidate_reject(completion); break; }
    if (e_sleep_parameter_offset == 0xffffffffu) { consolidate_reject(completion); break; }
    if (!consolidate_immutable_plan_span_within(e_sleep_parameter_offset,8u)) { consolidate_reject(completion); break; }
    if (b_recurrent_synapse_count > b_synapse_count) { consolidate_reject(completion); break; }
    if (l_replay_event_capacity > 0xffffffffu / 28u) { consolidate_reject(completion); break; }
    if (l_replay_span_count > 0xffffffffu / 4u) { consolidate_reject(completion); break; }
    if (h_replay_event_count > 0xffffffffu / 28u) { consolidate_reject(completion); break; }
    if (h_replay_span_count > 0xffffffffu / 4u) { consolidate_reject(completion); break; }
    if (l_replay_span_count == 0u) { consolidate_reject(completion); break; }
    if (l_replay_event_capacity > 0xffffffffu / l_replay_span_count) { consolidate_reject(completion); break; }
    let p_schema_version = immutable_plan_words[e_sleep_parameter_offset+0u];
    let p_staging_rate = bitcast<f32>(immutable_plan_words[e_sleep_parameter_offset+1u]);
    let p_weight_limit = bitcast<f32>(immutable_plan_words[e_sleep_parameter_offset+2u]);
    let p_fast_decay_rate = bitcast<f32>(immutable_plan_words[e_sleep_parameter_offset+3u]);
    let p_eligibility_reset_policy = immutable_plan_words[e_sleep_parameter_offset+4u];
    let p_replay_consume_policy = immutable_plan_words[e_sleep_parameter_offset+5u];
    let p_reserved_0 = immutable_plan_words[e_sleep_parameter_offset+6u];
    let p_reserved_1 = immutable_plan_words[e_sleep_parameter_offset+7u];
    if (!(h_schema_version == GPU_SLEEP_SCHEMA_VERSION)) { consolidate_reject(completion); break; }
    if (!(h_flags == 0u)) { consolidate_reject(completion); break; }
    if (!(h_reserved == 0u)) { consolidate_reject(completion); break; }
    if (!(h_class_id == b_class_id)) { consolidate_reject(completion); break; }
    if (!(h_slot == b_slot)) { consolidate_reject(completion); break; }
    if (!(h_slot_generation == b_slot_generation)) { consolidate_reject(completion); break; }
    if (!(h_brain_slot_index == b_slot)) { consolidate_reject(completion); break; }
    if (!(h_synapse_count == b_synapse_count)) { consolidate_reject(completion); break; }
    if (!(h_replay_event_count <= r_max_replay_events)) { consolidate_reject(completion); break; }
    if (!(h_replay_sample_count <= r_max_replay_eligibility_samples)) { consolidate_reject(completion); break; }
    if (!(h_replay_span_count == l_replay_span_count)) { consolidate_reject(completion); break; }
    if (!(r_schema_version == GPU_SLEEP_SCHEMA_VERSION)) { consolidate_reject(completion); break; }
    if (!(r_request_flags == 0u)) { consolidate_reject(completion); break; }
    if (!(r_cycle_id_lo == h_cycle_id_lo)) { consolidate_reject(completion); break; }
    if (!(r_cycle_id_hi == h_cycle_id_hi)) { consolidate_reject(completion); break; }
    if (!(r_input_generation_lo == l_active_weight_generation_lo)) { consolidate_reject(completion); break; }
    if (!(r_input_generation_hi == l_active_weight_generation_hi)) { consolidate_reject(completion); break; }
    var phenotype_matches = true;
    for (var lane=0u; lane<8u; lane+=1u) {
      if (frame_payload_words[h_request_offset+4u+lane] != phenotype_identities[h_brain_slot_index].phenotype_hash[lane]) { phenotype_matches = false; break; }
    }
    if (!phenotype_matches) { consolidate_reject(completion); break; }
    if (!(r_max_replay_events == l_replay_event_capacity)) { consolidate_reject(completion); break; }
    if (!(r_max_replay_eligibility_samples == l_replay_sample_capacity)) { consolidate_reject(completion); break; }
    if (!(r_reserved_tail_0 == 0u)) { consolidate_reject(completion); break; }
    if (!(r_reserved_tail_1 == 0u)) { consolidate_reject(completion); break; }
    if (!(p_schema_version == GPU_SLEEP_SCHEMA_VERSION)) { consolidate_reject(completion); break; }
    if (!(consolidate_finite(p_staging_rate))) { consolidate_reject(completion); break; }
    if (!(consolidate_finite(p_weight_limit))) { consolidate_reject(completion); break; }
    if (!(consolidate_finite(p_fast_decay_rate))) { consolidate_reject(completion); break; }
    if (!(p_staging_rate > 0.0)) { consolidate_reject(completion); break; }
    if (!(p_staging_rate <= 1.0)) { consolidate_reject(completion); break; }
    if (!(p_weight_limit > 0.0)) { consolidate_reject(completion); break; }
    if (!(p_weight_limit <= 8.0)) { consolidate_reject(completion); break; }
    if (!(p_fast_decay_rate >= 0.0)) { consolidate_reject(completion); break; }
    if (!(p_fast_decay_rate <= 1.0)) { consolidate_reject(completion); break; }
    if (!(p_eligibility_reset_policy == 1u)) { consolidate_reject(completion); break; }
    if (!(p_replay_consume_policy == 1u)) { consolidate_reject(completion); break; }
    if (!((p_reserved_0 == 0u && p_reserved_1 == 0u))) { consolidate_reject(completion); break; }
    if (!(l_schema_version == GPU_LEARNING_SCHEMA_VERSION)) { consolidate_reject(completion); break; }
    if (!(l_active_weight_bank <= 1u)) { consolidate_reject(completion); break; }
    if (!(l_active_eligibility_bank <= 1u)) { consolidate_reject(completion); break; }
    if (!(l_pending_valid == 0u)) { consolidate_reject(completion); break; }
    if (!(l_replay_cursor < l_replay_event_capacity)) { consolidate_reject(completion); break; }
    if (!(l_replay_event_count == h_replay_event_count)) { consolidate_reject(completion); break; }
    if (!(state_span_within(b_lifetime_weight_offset,b_synapse_count))) { consolidate_reject(completion); break; }
    if (!(state_span_within(b_fast_weight_offset,b_synapse_count))) { consolidate_reject(completion); break; }
    if (!(state_span_within(e_lifetime_bank_1_offset,b_synapse_count))) { consolidate_reject(completion); break; }
    if (!(state_span_within(e_fast_bank_1_offset,b_synapse_count))) { consolidate_reject(completion); break; }
    if (!(state_span_within(b_recurrent_eligibility_offset,b_recurrent_synapse_count))) { consolidate_reject(completion); break; }
    if (!(state_span_within(e_recurrent_eligibility_bank_1_offset,b_recurrent_synapse_count))) { consolidate_reject(completion); break; }
    if (!(state_span_within(b_decoder_eligibility_offset,b_synapse_count-b_recurrent_synapse_count))) { consolidate_reject(completion); break; }
    if (!(state_span_within(e_decoder_eligibility_bank_1_offset,b_synapse_count-b_recurrent_synapse_count))) { consolidate_reject(completion); break; }
    if (!(state_span_within(l_replay_event_rows_offset,l_replay_event_capacity*28u))) { consolidate_reject(completion); break; }
    if (!(state_span_within(l_replay_sample_offset,l_replay_sample_capacity))) { consolidate_reject(completion); break; }
    if (!(state_span_within(l_replay_span_offset,l_replay_span_count*4u))) { consolidate_reject(completion); break; }
    if (!(state_span_within(e_pending_eligibility_offset,SLEEP_PENDING_ELIGIBILITY_WORDS))) { consolidate_reject(completion); break; }
    if (!(consolidate_frame_span_within(h_request_offset,44u))) { consolidate_reject(completion); break; }
    if (!(consolidate_frame_span_within(h_replay_event_offset,h_replay_event_count*28u))) { consolidate_reject(completion); break; }
    if (!(consolidate_frame_span_within(h_replay_span_offset,h_replay_span_count*4u))) { consolidate_reject(completion); break; }
    if (!(consolidate_frame_span_within(h_replay_sample_offset,h_replay_sample_count))) { consolidate_reject(completion); break; }
    if (r_expected_output_generation_lo == 0u && r_expected_output_generation_hi == 0u) { consolidate_reject(completion); break; }
    store_state_u32(completion+4u,l_active_weight_generation_lo);
    store_state_u32(completion+5u,l_active_weight_generation_hi);
    store_state_u32(completion+6u,r_expected_output_generation_lo);
    store_state_u32(completion+7u,r_expected_output_generation_hi);
    store_state_u32(completion+8u,l_active_weight_bank^1u);
    store_state_u32(completion+9u,h_replay_span_count);
    store_state_u32(completion+12u,h_job_id_lo);
    store_state_u32(completion+13u,h_job_id_hi);
    prepared = true;
    break;
  }
  storageBarrier();
  if (prepared) { store_state_u32(completion+3u,SLEEP_STATUS_PREPARED); }
}

@compute @workgroup_size(64)
fn copy_sleep_weight_banks(@builtin(global_invocation_id) gid:vec3<u32>) {
  let header = load_sleep_header(gid.y*20u);
  if (header.brain_slot_index >= arrayLength(&brain_slots)
      || load_state_u32(header.completion_offset+3u) != SLEEP_STATUS_PREPARED
      || gid.x >= header.synapse_count) { return; }
  let brain = brain_slots[header.brain_slot_index];
  let bank_pair = load_weight_bank_pair_direct(brain);
  let active_bases = bank_pair.active_bases;
  let inactive = bank_pair.staging_bases;
  let lifetime = load_state_f32(active_bases.lifetime+gid.x);
  let fast = load_state_f32(active_bases.fast+gid.x);
  if (!consolidate_finite(lifetime) || !consolidate_finite(fast)) {
    consolidate_reject(header.completion_offset); return;
  }
  store_state_f32(inactive.lifetime+gid.x,lifetime);
  store_state_f32(inactive.fast+gid.x,canonicalize_state_zero(fast));
}

@compute @workgroup_size(64)
fn consolidate_fast_weights(@builtin(global_invocation_id) gid:vec3<u32>) {
  let header = load_sleep_header(gid.y*20u);
  if (header.brain_slot_index >= arrayLength(&brain_slots)
      || load_state_u32(header.completion_offset+3u) != SLEEP_STATUS_PREPARED
      || gid.x >= header.synapse_count) { return; }
  let brain = brain_slots[header.brain_slot_index];
  let delta_mode = consolidate_synapse_delta_mode(load_slot_extension(brain),gid.x);
  if (delta_mode == 2u) { consolidate_reject(header.completion_offset); return; }
  // The copy pass already staged both coefficients. Leave disabled rows intact.
  if (delta_mode == 0u) { return; }
  let parameters = load_sleep_parameter(
    load_state_u32(brain.extension_record_offset + 12u)
  );
  let bank_pair = load_weight_bank_pair_direct(brain);
  let active_bases = bank_pair.active_bases;
  let inactive = bank_pair.staging_bases;
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
  store_state_f32(inactive.lifetime+gid.x,canonicalize_state_zero(next_lifetime));
  store_state_f32(inactive.fast+gid.x,canonicalize_state_zero(next_fast));
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
    let delta_mode = consolidate_synapse_delta_mode(extension,synapse);
    if (delta_mode == 2u) { consolidate_reject(header.completion_offset); return; }
    let no_replay_fast = select(active_fast*(1.0-parameters.fast_decay_rate),active_fast,delta_mode == 0u);
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
