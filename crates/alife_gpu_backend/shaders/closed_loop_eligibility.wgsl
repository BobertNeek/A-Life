const ACTIVE_DISPATCH_ROW_WORDS:u32 = 308u;
const LEARNING_HEADER_WORD_OFFSET:u32 = 272u;
const SYNAPSE_KIND_RECURRENT:u32 = 1u;
const DECODER_HEAD_ACTION_CANDIDATE:u32 = 1u;
const DECODER_HEAD_MEMORY_CONTEXT:u32 = 2u;
const DECODER_HEAD_SPEECH_PAYLOAD:u32 = 3u;
const CANDIDATE_FEATURE_COUNT:u32 = 24u;
const PENDING_ELIGIBILITY_WORDS:u32 = 36u;
const ELIGIBILITY_DIAGNOSTIC_LANE:u32 = 2u;
const ELIGIBILITY_DIAGNOSTIC_UNKNOWN_DECODER_HEAD:u32 = 0x80000000u;

fn finite_eligibility(value:f32) -> bool {
  return value == value && abs(value) <= 3.402823466e+38;
}

fn immutable_span_within(start:u32, count:u32) -> bool {
  let limit = arrayLength(&immutable_plan_words);
  return start <= limit && count <= limit - start;
}

fn dispatch_span_within(start:u32, count:u32) -> bool {
  let limit = arrayLength(&dispatch_header_words);
  return start <= limit && count <= limit - start;
}

fn frame_span_within(start:u32, count:u32) -> bool {
  let limit = arrayLength(&frame_payload_words);
  return start <= limit && count <= limit - start;
}

fn load_selection_record(base:u32) -> GpuSelectionRecord {
  return GpuSelectionRecord(
    load_state_u32(base),load_state_u32(base+1u),load_state_u32(base+2u),load_state_u32(base+3u),
    load_state_u32(base+4u),load_state_u32(base+5u),load_state_u32(base+6u),load_state_u32(base+7u),
    load_state_u32(base+8u),load_state_u32(base+9u),load_state_u32(base+10u),load_state_u32(base+11u)
  );
}

fn pending_row_is_zero(base:u32) -> bool {
  if (!state_span_within(base, PENDING_ELIGIBILITY_WORDS)) { return false; }
  var index = 0u;
  loop {
    if (index >= PENDING_ELIGIBILITY_WORDS) { break; }
    if (load_state_u32(base + index) != 0u) { return false; }
    index += 1u;
  }
  return true;
}

fn receptor_is_valid(receptor:GpuPlasticityReceptorRecord) -> bool {
  return finite_eligibility(receptor.eligibility_decay)
    && finite_eligibility(receptor.learning_rate)
    && finite_eligibility(receptor.sleep_replay_rate)
    && finite_eligibility(receptor.normalization_rate)
    && finite_eligibility(receptor.modulator_sign)
    && finite_eligibility(receptor.fast_min)
    && finite_eligibility(receptor.fast_max)
    && receptor.eligibility_decay >= 0.0 && receptor.eligibility_decay <= 1.0
    && receptor.learning_rate >= 0.0 && receptor.learning_rate <= 1.0
    && receptor.sleep_replay_rate >= 0.0 && receptor.sleep_replay_rate <= 1.0
    && receptor.normalization_rate >= 0.0 && receptor.normalization_rate <= 1.0
    && (receptor.modulator_sign == -1.0 || receptor.modulator_sign == 1.0)
    && receptor.fast_min >= -8.0 && receptor.fast_max <= 8.0
    && receptor.fast_min < receptor.fast_max
    && bitcast<u32>(receptor.reserved) == 0u;
}

fn learning_contract_is_valid(
  header:GpuLearningHeader,
  brain:GpuBrainSlotRecord,
  extension:GpuBrainSlotExtensionRecord,
  learning:GpuSlotLearningStateRecord,
) -> bool {
  return header.schema_version == GPU_LEARNING_SCHEMA_VERSION
    && header.class_id == brain.class_id
    && header.slot == brain.slot
    && header.slot_generation == brain.slot_generation
    && header.brain_slot_index == brain.slot
    && header.active_activation_side <= 1u
    && (header.dispatch_generation_lo != 0u || header.dispatch_generation_hi != 0u)
    && header.candidate_count > 0u
    && header.decoder_input_stride >= CANDIDATE_FEATURE_COUNT
    && header.decoder_input_stride <= 64u
    && brain.recurrent_synapse_count <= brain.synapse_count
    && header.recurrent_synapse_count == brain.recurrent_synapse_count
    && header.decoder_synapse_count == brain.synapse_count - brain.recurrent_synapse_count
    && extension.decoder_synapse_local_start == brain.recurrent_synapse_count
    && extension.decoder_synapse_count == header.decoder_synapse_count
    && header.pending_eligibility_offset == extension.pending_eligibility_offset
    && header.pending_eligibility_offset == learning.pending_eligibility_offset
    && extension.replay_plan_identity_offset == learning.replay_plan_identity_offset
    && extension.replay_plan_identity_offset != 0xffffffffu
    && learning.active_eligibility_bank <= 1u
    && learning.pending_valid == 0u
    && header.reserved[0] == 0u && header.reserved[1] == 0u && header.reserved[2] == 0u
    && state_span_within(header.pending_eligibility_offset, PENDING_ELIGIBILITY_WORDS)
    && pending_row_is_zero(header.pending_eligibility_offset)
    && dispatch_span_within(header.candidate_offset, header.candidate_count * 8u)
    && frame_span_within(
      header.decoder_learning_input_offset,
      header.candidate_count * (header.decoder_input_stride + 4u)
    )
    && frame_span_within(header.outcome_offset, PENDING_ELIGIBILITY_WORDS);
}

@compute @workgroup_size(64)
fn accumulate_recurrent_eligibility(@builtin(global_invocation_id) gid:vec3<u32>) {
  let learning_base = gid.y * ACTIVE_DISPATCH_ROW_WORDS + LEARNING_HEADER_WORD_OFFSET;
  let header = load_learning_header(learning_base);
  if (header.brain_slot_index >= arrayLength(&brain_slots)) { return; }
  let brain = brain_slots[header.brain_slot_index];
  if (!state_span_within(brain.extension_record_offset, 20u)) { return; }
  let extension = load_slot_extension(brain);
  if (!state_span_within(extension.learning_state_offset, 24u)) { return; }
  let learning = load_slot_learning_state(extension);
  if (!learning_contract_is_valid(header, brain, extension, learning)) { return; }
  let local_synapse = gid.x;
  if (local_synapse >= header.recurrent_synapse_count) { return; }
  let metadata_base = extension.synapse_metadata_offset + local_synapse * 8u;
  if (!immutable_span_within(metadata_base, 8u)) { return; }
  let metadata = load_synapse_learning_metadata(metadata_base);
  if (metadata.kind != SYNAPSE_KIND_RECURRENT
      || metadata.global_synapse_id != local_synapse
      || metadata.source_neuron >= brain.neuron_count
      || metadata.target_neuron >= brain.neuron_count
      || metadata.eligibility_local_index >= header.recurrent_synapse_count
      || metadata.decoder_metadata_local_or_max != 0xffffffffu
      || metadata.reserved != 0u) { return; }
  let receptor_base = extension.receptor_offset + metadata.receptor_index * 8u;
  if (!immutable_span_within(receptor_base, 8u)) { return; }
  let receptor = load_plasticity_receptor(receptor_base);
  if (!receptor_is_valid(receptor)) { return; }
  let post_activation_offset = select(
    brain.activation_a_offset,
    brain.activation_b_offset,
    header.active_activation_side == 1u
  );
  let pre_activation_offset = select(
    brain.activation_b_offset,
    brain.activation_a_offset,
    header.active_activation_side == 1u
  );
  let source = pre_activation_offset + metadata.source_neuron;
  let target_index = post_activation_offset + metadata.target_neuron;
  let active_bases = active_eligibility_bases(brain, extension, learning);
  let staging_bases = inactive_eligibility_bases(brain, extension, learning);
  let active_index = active_bases.recurrent + metadata.eligibility_local_index;
  let staging_index = staging_bases.recurrent + metadata.eligibility_local_index;
  if (!state_span_within(source, 1u) || !state_span_within(target_index, 1u)
      || !state_span_within(active_index, 1u) || !state_span_within(staging_index, 1u)) { return; }
  let local = load_state_f32(source) * load_state_f32(target_index);
  let previous = load_state_f32(active_index);
  if (!finite_eligibility(local) || !finite_eligibility(previous)) { return; }
  store_state_f32(staging_index, clamp(receptor.eligibility_decay * previous + local, -1.0, 1.0));
}

@compute @workgroup_size(64)
fn accumulate_decoder_eligibility(@builtin(global_invocation_id) gid:vec3<u32>) {
  let learning_base = gid.y * ACTIVE_DISPATCH_ROW_WORDS + LEARNING_HEADER_WORD_OFFSET;
  let header = load_learning_header(learning_base);
  if (header.brain_slot_index >= arrayLength(&brain_slots)) { return; }
  let brain = brain_slots[header.brain_slot_index];
  if (!state_span_within(brain.extension_record_offset, 20u)) { return; }
  let extension = load_slot_extension(brain);
  if (!state_span_within(extension.learning_state_offset, 24u)) { return; }
  let learning = load_slot_learning_state(extension);
  if (!learning_contract_is_valid(header, brain, extension, learning)) { return; }
  let local_synapse = gid.x;
  if (local_synapse >= header.decoder_synapse_count) { return; }
  let metadata_base = extension.decoder_metadata_offset + local_synapse * 8u;
  if (!immutable_span_within(metadata_base, 8u)) { return; }
  let metadata = load_decoder_eligibility_metadata(metadata_base);
  if (metadata.global_synapse_id != extension.decoder_synapse_local_start + local_synapse
      || metadata.eligibility_local_index != local_synapse
      || metadata.motor_index >= brain.neuron_count
      || metadata.reserved != 0u) { return; }
  let receptor_base = extension.receptor_offset + metadata.receptor_index * 8u;
  if (!immutable_span_within(receptor_base, 8u)) { return; }
  let receptor = load_plasticity_receptor(receptor_base);
  if (!receptor_is_valid(receptor)) { return; }
  let selection = load_selection_record(header.selection_offset);
  if (selection.status != 1u || selection.candidate_index >= header.candidate_count
      || selection.active_activation_side != header.active_activation_side) { return; }
  let selected = load_candidate(header.candidate_offset + selection.candidate_index * 8u);
  let active_bases = active_eligibility_bases(brain, extension, learning);
  let staging_bases = inactive_eligibility_bases(brain, extension, learning);
  let active_index = active_bases.decoder + metadata.eligibility_local_index;
  let staging_index = staging_bases.decoder + metadata.eligibility_local_index;
  if (!state_span_within(active_index, 1u) || !state_span_within(staging_index, 1u)) { return; }
  var local = 0.0;
  if (metadata.decoder_head == DECODER_HEAD_ACTION_CANDIDATE) {
    if (metadata.family == selected.family) {
      if (metadata.input_lane >= CANDIDATE_FEATURE_COUNT) { return; }
      let feature_index = header.decoder_learning_input_offset
        + selection.candidate_index * header.decoder_input_stride
        + metadata.input_lane;
      let activation_offset = select(
        brain.activation_a_offset,
        brain.activation_b_offset,
        header.active_activation_side == 1u
      );
      if (!frame_span_within(feature_index, 1u)
          || !state_span_within(activation_offset + metadata.motor_index, 1u)) { return; }
      local = load_state_f32(activation_offset + metadata.motor_index)
        * bitcast<f32>(frame_payload_words[feature_index]);
    }
  } else if (metadata.decoder_head == DECODER_HEAD_MEMORY_CONTEXT) {
    if (metadata.family == selected.family) {
      if (metadata.input_lane < CANDIDATE_FEATURE_COUNT
          || metadata.input_lane >= header.decoder_input_stride) { return; }
      let feature_index = header.decoder_learning_input_offset
        + selection.candidate_index * header.decoder_input_stride
        + metadata.input_lane;
      if (!frame_span_within(feature_index, 1u)) { return; }
      local = bitcast<f32>(frame_payload_words[feature_index]);
    }
  } else if (metadata.decoder_head == DECODER_HEAD_SPEECH_PAYLOAD) {
    // The dedicated speech pass owns speech-payload eligibility after Vocalize wins.
  } else {
    atomicOr(
      &mutable_state_words[brain.diagnostic_offset + ELIGIBILITY_DIAGNOSTIC_LANE],
      ELIGIBILITY_DIAGNOSTIC_UNKNOWN_DECODER_HEAD
    );
    return;
  }
  let previous = load_state_f32(active_index);
  if (!finite_eligibility(local) || !finite_eligibility(previous)) { return; }
  store_state_f32(staging_index, clamp(receptor.eligibility_decay * previous + local, -1.0, 1.0));
}

@compute @workgroup_size(1)
fn finalize_pending_eligibility(@builtin(global_invocation_id) gid:vec3<u32>) {
  let learning_base = gid.y * ACTIVE_DISPATCH_ROW_WORDS + LEARNING_HEADER_WORD_OFFSET;
  let header = load_learning_header(learning_base);
  if (header.brain_slot_index >= arrayLength(&brain_slots)) { return; }
  let brain = brain_slots[header.brain_slot_index];
  if (!state_span_within(brain.extension_record_offset, 20u)) { return; }
  let extension = load_slot_extension(brain);
  if (!state_span_within(extension.learning_state_offset, 24u)) { return; }
  let learning = load_slot_learning_state(extension);
  if (!learning_contract_is_valid(header, brain, extension, learning)) { return; }
  if ((atomicLoad(&mutable_state_words[brain.diagnostic_offset + ELIGIBILITY_DIAGNOSTIC_LANE])
      & ELIGIBILITY_DIAGNOSTIC_UNKNOWN_DECODER_HEAD) != 0u) { return; }
  let selection = load_selection_record(header.selection_offset);
  if (selection.status != 1u || selection.candidate_index >= header.candidate_count
      || selection.active_activation_side != header.active_activation_side) { return; }
  let selected = load_candidate(header.candidate_offset + selection.candidate_index * 8u);
  if (selected.candidate_index != selection.candidate_index || selected.family >= 8u) { return; }
  let template_base = header.outcome_offset;
  if (frame_payload_words[template_base] != GPU_LEARNING_SCHEMA_VERSION
      || frame_payload_words[template_base+1u] != brain.slot
      || frame_payload_words[template_base+2u] != brain.slot_generation
      || frame_payload_words[template_base+3u] != header.active_activation_side
      || frame_payload_words[template_base+14u] != header.dispatch_generation_lo
      || frame_payload_words[template_base+15u] != header.dispatch_generation_hi
      || frame_payload_words[template_base+26u] != 0xffffffffu
      || frame_payload_words[template_base+27u] != 0u) { return; }
  var identity_index = 0u;
  loop {
    if (identity_index >= 8u) { break; }
    if (frame_payload_words[template_base+4u+identity_index]
        != phenotype_identities[header.brain_slot_index].phenotype_hash[identity_index]) { return; }
    identity_index += 1u;
  }
  let active_generation_lo = learning.active_eligibility_generation_lo;
  let active_generation_hi = learning.active_eligibility_generation_hi;
  let next_generation_lo = active_generation_lo + 1u;
  let carry = select(0u, 1u, next_generation_lo == 0u);
  let next_generation_hi = active_generation_hi + carry;
  if (next_generation_lo == 0u && next_generation_hi == 0u
      || frame_payload_words[template_base+32u] != active_generation_lo
      || frame_payload_words[template_base+33u] != active_generation_hi
      || frame_payload_words[template_base+34u] != next_generation_lo
      || frame_payload_words[template_base+35u] != next_generation_hi) { return; }
  let transaction_lo = learning.transaction_generation_lo + 1u;
  let transaction_carry = select(0u, 1u, transaction_lo == 0u);
  let transaction_hi = learning.transaction_generation_hi + transaction_carry;
  if (transaction_lo == 0u && transaction_hi == 0u) { return; }
  let digest_base = header.decoder_learning_input_offset
    + header.candidate_count * header.decoder_input_stride
    + selection.candidate_index * 4u;
  if (!frame_span_within(digest_base, 4u)) { return; }
  var copy_index = 0u;
  loop {
    if (copy_index >= 26u) { break; }
    store_state_u32(header.pending_eligibility_offset + copy_index, frame_payload_words[template_base + copy_index]);
    copy_index += 1u;
  }
  store_state_u32(
    header.pending_eligibility_offset + 26u,
    selection.candidate_index | (selected.family << 16u)
  );
  store_state_u32(header.pending_eligibility_offset + 27u, selected.action_id);
  for (var digest_index = 0u; digest_index < 4u; digest_index += 1u) {
    store_state_u32(
      header.pending_eligibility_offset + 28u + digest_index,
      frame_payload_words[digest_base + digest_index]
    );
  }
  for (var generation_index = 32u; generation_index < 36u; generation_index += 1u) {
    store_state_u32(
      header.pending_eligibility_offset + generation_index,
      frame_payload_words[template_base + generation_index]
    );
  }
  let state_base = extension.learning_state_offset;
  store_state_u32(state_base + 8u, next_generation_lo);
  store_state_u32(state_base + 9u, next_generation_hi);
  store_state_u32(state_base + 22u, transaction_lo);
  store_state_u32(state_base + 23u, transaction_hi);
  storageBarrier();
  store_state_u32(state_base + 3u, 1u);
  storageBarrier();
  // Status 3 proves that the GPU completed eligibility after winner selection.
  // The host validates this proof and normalizes the public winner record back
  // to status 1; the 36-word pending identity remains resident on the GPU.
  store_state_u32(header.selection_offset + 5u, 3u);
}

fn discard_contract_is_valid(
  header:GpuLearningHeader,
  brain:GpuBrainSlotRecord,
  extension:GpuBrainSlotExtensionRecord,
  learning:GpuSlotLearningStateRecord,
) -> bool {
  if (header.schema_version != GPU_LEARNING_SCHEMA_VERSION
      || header.class_id != brain.class_id
      || header.slot != brain.slot
      || header.slot_generation != brain.slot_generation
      || header.brain_slot_index != brain.slot
      || header.active_activation_side > 1u
      || (header.dispatch_generation_lo == 0u && header.dispatch_generation_hi == 0u)
      || brain.recurrent_synapse_count > brain.synapse_count
      || header.recurrent_synapse_count != brain.recurrent_synapse_count
      || header.decoder_synapse_count != brain.synapse_count - brain.recurrent_synapse_count
      || extension.decoder_synapse_local_start != brain.recurrent_synapse_count
      || extension.decoder_synapse_count != header.decoder_synapse_count
      || header.pending_eligibility_offset != extension.pending_eligibility_offset
      || header.pending_eligibility_offset != learning.pending_eligibility_offset
      || learning.active_eligibility_bank > 1u
      || learning.pending_valid != 1u
      || header.reserved[0] != 0u || header.reserved[1] != 0u || header.reserved[2] != 0u
      || !state_span_within(header.pending_eligibility_offset, PENDING_ELIGIBILITY_WORDS)
      || !state_span_within(header.selection_offset, 12u)
      || !frame_span_within(header.outcome_offset, PENDING_ELIGIBILITY_WORDS)) {
    return false;
  }
  let expected_base = header.outcome_offset;
  if (frame_payload_words[expected_base] != GPU_LEARNING_SCHEMA_VERSION
      || frame_payload_words[expected_base + 1u] != brain.slot
      || frame_payload_words[expected_base + 2u] != brain.slot_generation
      || frame_payload_words[expected_base + 3u] != header.active_activation_side
      || frame_payload_words[expected_base + 14u] != header.dispatch_generation_lo
      || frame_payload_words[expected_base + 15u] != header.dispatch_generation_hi
      || frame_payload_words[expected_base + 32u] != learning.active_eligibility_generation_lo
      || frame_payload_words[expected_base + 33u] != learning.active_eligibility_generation_hi
      || frame_payload_words[expected_base + 34u] != learning.inactive_eligibility_generation_lo
      || frame_payload_words[expected_base + 35u] != learning.inactive_eligibility_generation_hi) {
    return false;
  }
  for (var index = 0u; index < PENDING_ELIGIBILITY_WORDS; index += 1u) {
    if (load_state_u32(header.pending_eligibility_offset + index)
        != frame_payload_words[expected_base + index]) {
      return false;
    }
  }
  return true;
}

@compute @workgroup_size(64)
fn discard_pending_eligibility_arrays(@builtin(global_invocation_id) gid:vec3<u32>) {
  let header = load_learning_header(LEARNING_HEADER_WORD_OFFSET);
  if (header.brain_slot_index >= arrayLength(&brain_slots)) { return; }
  let brain = brain_slots[header.brain_slot_index];
  if (!state_span_within(brain.extension_record_offset, 20u)) { return; }
  let extension = load_slot_extension(brain);
  if (!state_span_within(extension.learning_state_offset, 24u)) { return; }
  let learning = load_slot_learning_state(extension);
  if (!discard_contract_is_valid(header, brain, extension, learning)) { return; }
  let local_synapse = gid.x;
  let total = header.recurrent_synapse_count + header.decoder_synapse_count;
  if (local_synapse >= total) { return; }
  let staging_bases = inactive_eligibility_bases(brain, extension, learning);
  if (local_synapse < header.recurrent_synapse_count) {
    let index = staging_bases.recurrent + local_synapse;
    if (state_span_within(index, 1u)) { store_state_u32(index, 0u); }
    return;
  }
  let decoder_local = local_synapse - header.recurrent_synapse_count;
  let index = staging_bases.decoder + decoder_local;
  if (state_span_within(index, 1u)) { store_state_u32(index, 0u); }
}

@compute @workgroup_size(1)
fn finalize_discard_pending_eligibility(@builtin(global_invocation_id) gid:vec3<u32>) {
  let header = load_learning_header(LEARNING_HEADER_WORD_OFFSET);
  if (gid.x != 0u || header.brain_slot_index >= arrayLength(&brain_slots)) { return; }
  let brain = brain_slots[header.brain_slot_index];
  if (!state_span_within(brain.extension_record_offset, 20u)) { return; }
  let extension = load_slot_extension(brain);
  if (!state_span_within(extension.learning_state_offset, 24u)) { return; }
  let learning = load_slot_learning_state(extension);
  if (!discard_contract_is_valid(header, brain, extension, learning)) { return; }
  let staging_bases = inactive_eligibility_bases(brain, extension, learning);
  for (var recurrent = 0u; recurrent < header.recurrent_synapse_count; recurrent += 1u) {
    let index = staging_bases.recurrent + recurrent;
    if (!state_span_within(index, 1u) || load_state_u32(index) != 0u) { return; }
  }
  for (var decoder = 0u; decoder < header.decoder_synapse_count; decoder += 1u) {
    let index = staging_bases.decoder + decoder;
    if (!state_span_within(index, 1u) || load_state_u32(index) != 0u) { return; }
  }
  let transaction_lo = learning.transaction_generation_lo + 1u;
  let transaction_carry = select(0u, 1u, transaction_lo == 0u);
  let transaction_hi = learning.transaction_generation_hi + transaction_carry;
  if (transaction_lo == 0u && transaction_hi == 0u) { return; }
  let receipt = header.selection_offset;
  for (var word = 0u; word < 12u; word += 1u) {
    store_state_u32(receipt + word, 0u);
  }
  store_state_u32(receipt, GPU_LEARNING_SCHEMA_VERSION);
  store_state_u32(receipt + 1u, brain.slot);
  store_state_u32(receipt + 2u, brain.slot_generation);
  store_state_u32(receipt + 4u, learning.active_eligibility_bank);
  store_state_u32(receipt + 6u, learning.active_eligibility_generation_lo);
  store_state_u32(receipt + 7u, learning.active_eligibility_generation_hi);
  store_state_u32(receipt + 8u, learning.inactive_eligibility_generation_lo);
  store_state_u32(receipt + 9u, learning.inactive_eligibility_generation_hi);
  store_state_u32(receipt + 10u, transaction_lo);
  store_state_u32(receipt + 11u, transaction_hi);
  for (var pending_word = 0u; pending_word < PENDING_ELIGIBILITY_WORDS; pending_word += 1u) {
    store_state_u32(header.pending_eligibility_offset + pending_word, 0u);
  }
  let state_base = extension.learning_state_offset;
  store_state_u32(state_base + 8u, 0u);
  store_state_u32(state_base + 9u, 0u);
  store_state_u32(state_base + 22u, transaction_lo);
  store_state_u32(state_base + 23u, transaction_hi);
  storageBarrier();
  store_state_u32(state_base + 3u, 0u);
  storageBarrier();
  store_state_u32(receipt + 3u, 1u);
}
