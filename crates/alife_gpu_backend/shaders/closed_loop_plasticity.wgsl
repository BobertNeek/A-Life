const PLASTICITY_DISPATCH_ROW_WORDS:u32 = 332u;
const PLASTICITY_HEADER_WORD_OFFSET:u32 = 272u;
const OUTCOME_CREDIT_WORDS:u32 = 48u;
const PENDING_ELIGIBILITY_WORDS_PLASTICITY:u32 = 36u;
const FAST_PLASTICITY_RECEIPT_WORDS:u32 = 16u;
const SYNAPSE_KIND_RECURRENT_PLASTICITY:u32 = 1u;
const SYNAPSE_KIND_DECODER_PLASTICITY:u32 = 2u;
const PLASTICITY_STATUS_PREPARED:u32 = 256u;
const PLASTICITY_STATUS_SUCCESS:u32 = 1u;
const PLASTICITY_STATUS_GUARD_REJECTED:u32 = 512u;
const PLASTICITY_GUARD_HEADER_LAYOUT:u32 = 1u;
const PLASTICITY_GUARD_EXTENSION_LAYOUT:u32 = 2u;
const PLASTICITY_GUARD_STATE_EVIDENCE:u32 = 3u;
const PLASTICITY_GUARD_SYNAPSE_FINITE:u32 = 4u;
const PLASTICITY_GUARD_REPLAY_SPAN:u32 = 5u;
const PLASTICITY_GUARD_REPLAY_ELIGIBILITY:u32 = 6u;
const PLASTICITY_GUARD_GENERATION_OVERFLOW:u32 = 7u;

fn finite_plasticity(value:f32) -> bool {
  return value == value && abs(value) <= 3.402823466e+38;
}

fn immutable_plan_span_within(start:u32, count:u32) -> bool {
  let limit = arrayLength(&immutable_plan_words);
  return start <= limit && count <= limit - start;
}

fn immutable_weight_span_within(start:u32, count:u32) -> bool {
  let limit = arrayLength(&immutable_weight_words);
  return start <= limit && count <= limit - start;
}

fn plasticity_frame_span_within(start:u32, count:u32) -> bool {
  let limit = arrayLength(&frame_payload_words);
  return start <= limit && count <= limit - start;
}

fn pair_equal(left:vec2<u32>, right:vec2<u32>) -> bool {
  return all(left == right);
}

fn pair_nonzero(value:vec2<u32>) -> bool {
  return value.x != 0u || value.y != 0u;
}

fn pair_less(left:vec2<u32>, right:vec2<u32>) -> bool {
  return left.y < right.y || (left.y == right.y && left.x < right.x);
}

fn increment_pair(value:vec2<u32>) -> vec2<u32> {
  let lo = value.x + 1u;
  return vec2<u32>(lo, value.y + select(0u, 1u, lo == 0u));
}

fn load_outcome_credit(base:u32) -> GpuOutcomeCreditRecord {
  return GpuOutcomeCreditRecord(
    frame_payload_words[base], frame_payload_words[base+1u],
    vec2<u32>(frame_payload_words[base+2u],frame_payload_words[base+3u]),
    array<u32,8>(
      frame_payload_words[base+4u],frame_payload_words[base+5u],frame_payload_words[base+6u],frame_payload_words[base+7u],
      frame_payload_words[base+8u],frame_payload_words[base+9u],frame_payload_words[base+10u],frame_payload_words[base+11u]
    ),
    vec2<u32>(frame_payload_words[base+12u],frame_payload_words[base+13u]),
    vec2<u32>(frame_payload_words[base+14u],frame_payload_words[base+15u]),
    vec2<u32>(frame_payload_words[base+16u],frame_payload_words[base+17u]),
    frame_payload_words[base+18u],frame_payload_words[base+19u],
    vec4<u32>(frame_payload_words[base+20u],frame_payload_words[base+21u],frame_payload_words[base+22u],frame_payload_words[base+23u]),
    array<u32,8>(
      frame_payload_words[base+24u],frame_payload_words[base+25u],frame_payload_words[base+26u],frame_payload_words[base+27u],
      frame_payload_words[base+28u],frame_payload_words[base+29u],frame_payload_words[base+30u],frame_payload_words[base+31u]
    ),
    vec2<u32>(frame_payload_words[base+32u],frame_payload_words[base+33u]),
    bitcast<f32>(frame_payload_words[base+34u]),bitcast<f32>(frame_payload_words[base+35u]),
    bitcast<f32>(frame_payload_words[base+36u]),bitcast<f32>(frame_payload_words[base+37u]),
    bitcast<f32>(frame_payload_words[base+38u]),bitcast<f32>(frame_payload_words[base+39u]),
    bitcast<f32>(frame_payload_words[base+40u]),bitcast<f32>(frame_payload_words[base+41u]),
    bitcast<f32>(frame_payload_words[base+42u]),bitcast<f32>(frame_payload_words[base+43u]),
    bitcast<f32>(frame_payload_words[base+44u]),
    vec3<u32>(frame_payload_words[base+45u],frame_payload_words[base+46u],frame_payload_words[base+47u])
  );
}

fn load_pending_eligibility(base:u32) -> GpuPendingEligibilityRecord {
  return GpuPendingEligibilityRecord(
    load_state_u32(base),load_state_u32(base+1u),load_state_u32(base+2u),load_state_u32(base+3u),
    array<u32,8>(
      load_state_u32(base+4u),load_state_u32(base+5u),load_state_u32(base+6u),load_state_u32(base+7u),
      load_state_u32(base+8u),load_state_u32(base+9u),load_state_u32(base+10u),load_state_u32(base+11u)
    ),
    vec2<u32>(load_state_u32(base+12u),load_state_u32(base+13u)),
    vec2<u32>(load_state_u32(base+14u),load_state_u32(base+15u)),
    vec2<u32>(load_state_u32(base+16u),load_state_u32(base+17u)),
    array<u32,8>(
      load_state_u32(base+18u),load_state_u32(base+19u),load_state_u32(base+20u),load_state_u32(base+21u),
      load_state_u32(base+22u),load_state_u32(base+23u),load_state_u32(base+24u),load_state_u32(base+25u)
    ),
    load_state_u32(base+26u),load_state_u32(base+27u),
    vec4<u32>(load_state_u32(base+28u),load_state_u32(base+29u),load_state_u32(base+30u),load_state_u32(base+31u)),
    vec2<u32>(load_state_u32(base+32u),load_state_u32(base+33u)),
    vec2<u32>(load_state_u32(base+34u),load_state_u32(base+35u))
  );
}

fn array8_equal(left:array<u32,8>, right:array<u32,8>) -> bool {
  for (var index=0u; index<8u; index+=1u) {
    if (left[index] != right[index]) { return false; }
  }
  return true;
}

fn receptor_valid_for_plasticity(receptor:GpuPlasticityReceptorRecord) -> bool {
  return finite_plasticity(receptor.eligibility_decay)
    && finite_plasticity(receptor.learning_rate)
    && finite_plasticity(receptor.sleep_replay_rate)
    && finite_plasticity(receptor.normalization_rate)
    && finite_plasticity(receptor.modulator_sign)
    && finite_plasticity(receptor.fast_min)
    && finite_plasticity(receptor.fast_max)
    && receptor.eligibility_decay >= 0.0 && receptor.eligibility_decay <= 1.0
    && receptor.learning_rate >= 0.0 && receptor.learning_rate <= 1.0
    && receptor.normalization_rate >= 0.0 && receptor.normalization_rate <= 1.0
    && receptor.normalization_rate <= receptor.learning_rate
    && (receptor.modulator_sign == -1.0 || receptor.modulator_sign == 1.0)
    && receptor.fast_min >= -8.0 && receptor.fast_max <= 8.0
    && receptor.fast_min < receptor.fast_max
    && bitcast<u32>(receptor.reserved) == 0u;
}

fn plasticity_guard_bit(bit:u32, failed:bool) -> u32 {
  return select(0u, 1u << bit, failed);
}

fn reject_plasticity(receipt_base:u32, guard_id:u32, operand0:u32, operand1:u32, context:u32) {
  if (state_span_within(receipt_base, FAST_PLASTICITY_RECEIPT_WORDS)) {
    let previous = atomicMin(&mutable_state_words[receipt_base+6u], guard_id);
    if (previous > guard_id) {
      store_state_u32(receipt_base+7u, operand0);
      store_state_u32(receipt_base+8u, operand1);
      store_state_u32(receipt_base+9u, context);
    }
    storageBarrier();
    atomicMax(&mutable_state_words[receipt_base+3u], PLASTICITY_STATUS_GUARD_REJECTED);
  }
}

fn load_staging_eligibility_value(
  brain:GpuBrainSlotRecord,
  extension:GpuBrainSlotExtensionRecord,
  learning:GpuSlotLearningStateRecord,
  metadata:GpuSynapseLearningMetadata,
) -> f32 {
  let staging = inactive_eligibility_bases(brain, extension, learning);
  let index = select(
    staging.decoder + metadata.eligibility_local_index,
    staging.recurrent + metadata.eligibility_local_index,
    metadata.kind == SYNAPSE_KIND_RECURRENT_PLASTICITY
  );
  return load_state_f32(index);
}

@compute @workgroup_size(1)
fn initialize_fast_plasticity(@builtin(global_invocation_id) gid:vec3<u32>) {
  let row = gid.y;
  let header = load_learning_header(row * PLASTICITY_DISPATCH_ROW_WORDS + PLASTICITY_HEADER_WORD_OFFSET);
  if (header.brain_slot_index >= arrayLength(&brain_slots)) { return; }
  let brain = brain_slots[header.brain_slot_index];
  let receipt_base = brain.diagnostic_offset;
  if (!state_span_within(receipt_base, FAST_PLASTICITY_RECEIPT_WORDS)) { return; }
  for (var word=0u; word<FAST_PLASTICITY_RECEIPT_WORDS; word+=1u) {
    store_state_u32(receipt_base+word, 0u);
  }
  store_state_u32(receipt_base+6u, 0xffffffffu);
  store_state_u32(receipt_base, GPU_LEARNING_SCHEMA_VERSION);
  store_state_u32(receipt_base+1u, brain.slot);
  store_state_u32(receipt_base+2u, brain.slot_generation);
  let header_guard =
      plasticity_guard_bit(0u, header.schema_version != GPU_LEARNING_SCHEMA_VERSION)
    | plasticity_guard_bit(1u, header.class_id != brain.class_id)
    | plasticity_guard_bit(2u, header.slot != brain.slot)
    | plasticity_guard_bit(3u, header.slot_generation != brain.slot_generation)
    | plasticity_guard_bit(4u, header.brain_slot_index != brain.slot)
    | plasticity_guard_bit(5u, header.active_activation_side > 1u)
    | plasticity_guard_bit(6u, !pair_nonzero(vec2<u32>(header.dispatch_generation_lo,header.dispatch_generation_hi)))
    | plasticity_guard_bit(7u, header.candidate_count != 0u)
    | plasticity_guard_bit(8u, header.candidate_offset != 0u)
    | plasticity_guard_bit(9u, header.decoder_learning_input_offset != 0u)
    | plasticity_guard_bit(10u, header.selection_offset != receipt_base)
    | plasticity_guard_bit(11u, brain.selection_offset != receipt_base + 4u)
    | plasticity_guard_bit(12u, header.recurrent_synapse_count != brain.recurrent_synapse_count)
    | plasticity_guard_bit(13u, header.decoder_synapse_count != brain.synapse_count - brain.recurrent_synapse_count)
    | plasticity_guard_bit(14u, header.decoder_input_stride != 0u)
    | plasticity_guard_bit(15u, header.scheduled_tile_visits != 0u)
    | plasticity_guard_bit(16u, header.scheduled_synapse_ops != 0u)
    | plasticity_guard_bit(17u, header.scheduled_work_checksum != 0u)
    | plasticity_guard_bit(18u, !state_span_within(brain.extension_record_offset,20u))
    | plasticity_guard_bit(19u, !plasticity_frame_span_within(header.outcome_offset,OUTCOME_CREDIT_WORDS));
  if (header_guard != 0u) {
    reject_plasticity(receipt_base, PLASTICITY_GUARD_HEADER_LAYOUT, header_guard, 0u, 0u); return;
  }
  let extension = load_slot_extension(brain);
  let extension_guard =
      plasticity_guard_bit(0u, extension.schema_version != GPU_CLOSED_LOOP_LAYOUT_VERSION)
    | plasticity_guard_bit(1u, extension.pending_eligibility_offset != header.pending_eligibility_offset)
    | plasticity_guard_bit(2u, !state_span_within(extension.learning_state_offset,24u))
    | plasticity_guard_bit(3u, !state_span_within(extension.pending_eligibility_offset,PENDING_ELIGIBILITY_WORDS_PLASTICITY));
  if (extension_guard != 0u) {
    reject_plasticity(receipt_base, PLASTICITY_GUARD_EXTENSION_LAYOUT, extension_guard, 0u, 0u); return;
  }
  let learning = load_slot_learning_state(extension);
  let outcome = load_outcome_credit(header.outcome_offset);
  let measured_rpe = outcome.raw_reward - outcome.expected_value;
  let measured_modulator = clamp(
      measured_rpe - outcome.pain - outcome.injury
      + 0.75 * outcome.homeostatic_improvement
      - 0.5 * outcome.frustration
      + 0.2 * outcome.novelty
      + 0.2 * outcome.social,
      -1.0,
      1.0
  );
  let pending = load_pending_eligibility(extension.pending_eligibility_offset);
  let family = (outcome.selected_candidate_and_family >> 16u) & 0xffu;
  let state_guard_lo =
      plasticity_guard_bit(0u, learning.schema_version != GPU_LEARNING_SCHEMA_VERSION)
    | plasticity_guard_bit(1u, learning.pending_valid != 1u)
    | plasticity_guard_bit(2u, learning.active_weight_bank > 1u)
    | plasticity_guard_bit(3u, learning.active_eligibility_bank > 1u)
    | plasticity_guard_bit(4u, learning.pending_eligibility_offset != extension.pending_eligibility_offset)
    | plasticity_guard_bit(5u, learning.replay_plan_identity_offset != extension.replay_plan_identity_offset)
    | plasticity_guard_bit(6u, !pair_nonzero(vec2<u32>(learning.active_weight_generation_lo,learning.active_weight_generation_hi)))
    | plasticity_guard_bit(7u, !pair_nonzero(vec2<u32>(learning.active_eligibility_generation_lo,learning.active_eligibility_generation_hi)))
    | plasticity_guard_bit(8u, !pair_nonzero(vec2<u32>(learning.inactive_eligibility_generation_lo,learning.inactive_eligibility_generation_hi)))
    | plasticity_guard_bit(9u, !pair_nonzero(vec2<u32>(learning.replay_generation_lo,learning.replay_generation_hi)))
    | plasticity_guard_bit(10u, learning.replay_event_capacity == 0u)
    | plasticity_guard_bit(11u, learning.replay_event_capacity > 65536u)
    | plasticity_guard_bit(12u, learning.replay_cursor >= learning.replay_event_capacity)
    | plasticity_guard_bit(13u, learning.replay_event_count > learning.replay_event_capacity)
    | plasticity_guard_bit(14u, learning.replay_span_count == 0u)
    | plasticity_guard_bit(15u, learning.replay_sample_capacity != learning.replay_event_capacity * learning.replay_span_count)
    | plasticity_guard_bit(16u, outcome.schema_version != GPU_LEARNING_SCHEMA_VERSION)
    | plasticity_guard_bit(17u, (outcome.selected_candidate_and_family >> 24u) != 0u)
    | plasticity_guard_bit(18u, family >= 8u)
    | plasticity_guard_bit(19u, !pair_nonzero(outcome.organism_id))
    | plasticity_guard_bit(20u, !pair_nonzero(outcome.sequence_id))
    | plasticity_guard_bit(21u, !pair_less(outcome.originating_tick,outcome.outcome_tick))
    | plasticity_guard_bit(22u, outcome.active_activation_side != header.active_activation_side)
    | plasticity_guard_bit(23u, !pair_equal(outcome.dispatch_generation,vec2<u32>(header.dispatch_generation_lo,header.dispatch_generation_hi)))
    | plasticity_guard_bit(24u,
        !finite_plasticity(outcome.raw_reward) || abs(outcome.raw_reward) > 1.0
        || !finite_plasticity(outcome.expected_value) || abs(outcome.expected_value) > 1.0
        || !finite_plasticity(outcome.reward_prediction_error) || abs(outcome.reward_prediction_error) > 2.0
        || abs(outcome.reward_prediction_error - measured_rpe) > 0.0001)
    | plasticity_guard_bit(25u,
        !finite_plasticity(outcome.pain) || outcome.pain < 0.0 || outcome.pain > 1.0
        || !finite_plasticity(outcome.injury) || outcome.injury < 0.0 || outcome.injury > 1.0)
    | plasticity_guard_bit(26u,
        !finite_plasticity(outcome.homeostatic_improvement) || abs(outcome.homeostatic_improvement) > 1.0
        || !finite_plasticity(outcome.frustration) || outcome.frustration < 0.0 || outcome.frustration > 1.0)
    | plasticity_guard_bit(27u,
        !finite_plasticity(outcome.novelty) || outcome.novelty < 0.0 || outcome.novelty > 1.0
        || !finite_plasticity(outcome.sensory_prediction_residual)
        || outcome.sensory_prediction_residual < 0.0 || outcome.sensory_prediction_residual > 1.0)
    | plasticity_guard_bit(28u,
        !finite_plasticity(outcome.social) || abs(outcome.social) > 1.0
        || !finite_plasticity(outcome.modulator_value) || abs(outcome.modulator_value) > 1.0)
    | plasticity_guard_bit(29u,
        abs(outcome.modulator_value - measured_modulator) > 0.0001
        || any(outcome.reserved != vec3<u32>(0u,0u,0u)))
    | plasticity_guard_bit(30u, pending.schema_version != GPU_LEARNING_SCHEMA_VERSION)
    | plasticity_guard_bit(31u, pending.slot != brain.slot);
  let state_guard_hi =
      plasticity_guard_bit(0u, pending.slot_generation != brain.slot_generation)
    | plasticity_guard_bit(1u, pending.active_activation_side != outcome.active_activation_side)
    | plasticity_guard_bit(2u, !pair_equal(pending.organism_id,outcome.organism_id))
    | plasticity_guard_bit(3u, !pair_equal(pending.dispatch_generation,outcome.dispatch_generation))
    | plasticity_guard_bit(4u, !pair_equal(pending.originating_tick,outcome.originating_tick))
    | plasticity_guard_bit(5u, pending.candidate_index_and_family != outcome.selected_candidate_and_family)
    | plasticity_guard_bit(6u, pending.action_id != outcome.selected_action)
    | plasticity_guard_bit(7u, any(pending.candidate_feature_digest != outcome.candidate_feature_digest))
    | plasticity_guard_bit(8u, !array8_equal(pending.phenotype_hash,outcome.phenotype_hash))
    | plasticity_guard_bit(9u, !array8_equal(pending.frame_digest,outcome.frame_digest))
    | plasticity_guard_bit(10u, !array8_equal(outcome.phenotype_hash,phenotype_identities[header.brain_slot_index].phenotype_hash))
    | plasticity_guard_bit(11u, !pair_equal(pending.active_eligibility_generation,vec2<u32>(learning.active_eligibility_generation_lo,learning.active_eligibility_generation_hi)))
    | plasticity_guard_bit(12u, !pair_equal(pending.staging_eligibility_generation,vec2<u32>(learning.inactive_eligibility_generation_lo,learning.inactive_eligibility_generation_hi)))
    | plasticity_guard_bit(13u, !state_span_within(learning.replay_event_rows_offset,learning.replay_event_capacity*24u))
    | plasticity_guard_bit(14u, !state_span_within(learning.replay_sample_offset,learning.replay_sample_capacity))
    | plasticity_guard_bit(15u, !state_span_within(learning.replay_span_offset,learning.replay_span_count*4u))
    | plasticity_guard_bit(16u, !state_span_within(brain.activation_a_offset,brain.neuron_count))
    | plasticity_guard_bit(17u, !state_span_within(brain.activation_b_offset,brain.neuron_count))
    | plasticity_guard_bit(18u, !state_span_within(brain.lifetime_weight_offset,brain.synapse_count))
    | plasticity_guard_bit(19u, !state_span_within(brain.fast_weight_offset,brain.synapse_count))
    | plasticity_guard_bit(20u, !state_span_within(extension.lifetime_bank_1_offset,brain.synapse_count))
    | plasticity_guard_bit(21u, !state_span_within(extension.fast_bank_1_offset,brain.synapse_count))
    | plasticity_guard_bit(22u, !immutable_weight_span_within(brain.genetic_weight_offset,brain.synapse_count))
    | plasticity_guard_bit(23u, !immutable_weight_span_within(brain.alpha_offset,brain.synapse_count))
    | plasticity_guard_bit(24u, !immutable_plan_span_within(extension.synapse_metadata_offset,brain.synapse_count*8u));
  if (state_guard_lo != 0u || state_guard_hi != 0u) {
    reject_plasticity(receipt_base, PLASTICITY_GUARD_STATE_EVIDENCE, state_guard_lo, state_guard_hi, 0u); return;
  }
  store_state_u32(receipt_base+4u,learning.active_weight_generation_lo);
  store_state_u32(receipt_base+5u,learning.active_weight_generation_hi);
  storageBarrier();
  store_state_u32(receipt_base+3u,PLASTICITY_STATUS_PREPARED);
}

@compute @workgroup_size(64)
fn apply_fast_plasticity(@builtin(global_invocation_id) gid:vec3<u32>) {
  let header = load_learning_header(gid.y * PLASTICITY_DISPATCH_ROW_WORDS + PLASTICITY_HEADER_WORD_OFFSET);
  if (header.brain_slot_index >= arrayLength(&brain_slots)) { return; }
  let brain = brain_slots[header.brain_slot_index];
  let receipt_base = brain.diagnostic_offset;
  if (load_state_u32(receipt_base+3u) != PLASTICITY_STATUS_PREPARED) { return; }
  let local_synapse = gid.x;
  if (local_synapse >= brain.synapse_count) { return; }
  let extension = load_slot_extension(brain);
  let learning = load_slot_learning_state(extension);
  let metadata_base = extension.synapse_metadata_offset + local_synapse*8u;
  let metadata = load_synapse_learning_metadata(metadata_base);
  let receptor_base = extension.receptor_offset + metadata.receptor_index*8u;
  let learning_rate = bitcast<f32>(immutable_plan_words[receptor_base+1u]);
  let normalization_rate = bitcast<f32>(immutable_plan_words[receptor_base+3u]);
  let modulator_sign = bitcast<f32>(immutable_plan_words[receptor_base+4u]);
  let fast_min = bitcast<f32>(immutable_plan_words[receptor_base+5u]);
  let fast_max = bitcast<f32>(immutable_plan_words[receptor_base+6u]);
  let active_weights = active_weight_bases(brain,extension,learning);
  let inactive_weights = inactive_weight_bases(brain,extension,learning);
  let active_lifetime_index = active_weights.lifetime + local_synapse;
  let inactive_lifetime_index = inactive_weights.lifetime + local_synapse;
  let active_fast_index = active_weights.fast + local_synapse;
  let inactive_fast_index = inactive_weights.fast + local_synapse;
  let staging_eligibility = load_staging_eligibility_value(brain,extension,learning,metadata);
  let activation_base = select(brain.activation_a_offset,brain.activation_b_offset,header.active_activation_side==1u);
  let post = load_state_f32(activation_base+metadata.target_neuron);
  let genetic = bitcast<f32>(immutable_weight_words[brain.genetic_weight_offset+local_synapse]);
  let alpha = bitcast<f32>(immutable_weight_words[brain.alpha_offset+local_synapse]);
  let lifetime = load_state_f32(active_lifetime_index);
  let fast = load_state_f32(active_fast_index);
  let joint_modulator = bitcast<f32>(frame_payload_words[header.outcome_offset+44u]);
  let effective = genetic + lifetime + alpha*fast;
  let outcome_gated_update = learning_rate*alpha*(joint_modulator*modulator_sign)*staging_eligibility;
  let slow_normalization = normalization_rate*post*post*effective;
  let delta = outcome_gated_update - slow_normalization;
  let next_fast = clamp(fast+delta,fast_min,fast_max);
  let synapse_guard =
      plasticity_guard_bit(0u, !finite_plasticity(staging_eligibility))
    | plasticity_guard_bit(1u, !finite_plasticity(post))
    | plasticity_guard_bit(2u, !finite_plasticity(genetic))
    | plasticity_guard_bit(3u, !finite_plasticity(alpha))
    | plasticity_guard_bit(4u, !finite_plasticity(lifetime))
    | plasticity_guard_bit(5u, !finite_plasticity(fast))
    | plasticity_guard_bit(6u, !finite_plasticity(delta))
    | plasticity_guard_bit(7u, !finite_plasticity(next_fast));
  if (synapse_guard != 0u) {
    reject_plasticity(receipt_base, PLASTICITY_GUARD_SYNAPSE_FINITE, synapse_guard, 0u, local_synapse); return;
  }
  store_state_f32(inactive_lifetime_index,lifetime);
  store_state_f32(inactive_fast_index,canonicalize_state_zero(next_fast));
  let applied = abs(next_fast-fast);
  if (applied > 0.0) {
    atomicAdd(&mutable_state_words[receipt_base+14u],1u);
    atomicMax(&mutable_state_words[receipt_base+15u],bitcast<u32>(applied));
  }
}

@compute @workgroup_size(64)
fn capture_fast_plasticity_replay(@builtin(global_invocation_id) gid:vec3<u32>) {
  let header = load_learning_header(gid.y * PLASTICITY_DISPATCH_ROW_WORDS + PLASTICITY_HEADER_WORD_OFFSET);
  if (header.brain_slot_index >= arrayLength(&brain_slots)) { return; }
  let brain = brain_slots[header.brain_slot_index];
  let receipt_base = brain.diagnostic_offset;
  if (load_state_u32(receipt_base+3u) != PLASTICITY_STATUS_PREPARED) { return; }
  let extension = load_slot_extension(brain);
  let learning = load_slot_learning_state(extension);
  if (gid.x >= learning.replay_span_count) { return; }
  let span_base = learning.replay_span_offset + gid.x*4u;
  let local_synapse = load_state_u32(span_base);
  let sample_start = load_state_u32(span_base+1u);
  let reserved = load_state_u32(span_base+3u);
  let replay_span_guard =
      plasticity_guard_bit(0u, local_synapse >= brain.synapse_count)
    | plasticity_guard_bit(1u, reserved != 0u)
    | plasticity_guard_bit(2u, sample_start > learning.replay_sample_capacity)
    | plasticity_guard_bit(3u, sample_start <= learning.replay_sample_capacity
      && learning.replay_event_capacity > learning.replay_sample_capacity-sample_start);
  if (replay_span_guard != 0u) {
    reject_plasticity(receipt_base, PLASTICITY_GUARD_REPLAY_SPAN, replay_span_guard, 0u, gid.x); return;
  }
  let metadata = load_synapse_learning_metadata(extension.synapse_metadata_offset+local_synapse*8u);
  let eligibility = load_staging_eligibility_value(brain,extension,learning,metadata);
  if (!finite_plasticity(eligibility)) {
    reject_plasticity(receipt_base, PLASTICITY_GUARD_REPLAY_ELIGIBILITY, bitcast<u32>(eligibility), 0u, gid.x); return;
  }
  let signed_q15 = i32(round(clamp(eligibility,-1.0,1.0)*32767.0));
  let packed = (learning.replay_cursor & 0xffffu) | ((u32(signed_q15)&0xffffu)<<16u);
  store_state_u32(learning.replay_sample_offset+sample_start+learning.replay_cursor,packed);
  store_state_u32(span_base+2u,min(learning.replay_event_count+1u,learning.replay_event_capacity));
}

@compute @workgroup_size(1)
fn finalize_fast_plasticity(@builtin(global_invocation_id) gid:vec3<u32>) {
  let header = load_learning_header(gid.y * PLASTICITY_DISPATCH_ROW_WORDS + PLASTICITY_HEADER_WORD_OFFSET);
  if (header.brain_slot_index >= arrayLength(&brain_slots)) { return; }
  let brain = brain_slots[header.brain_slot_index];
  let receipt_base = brain.diagnostic_offset;
  if (load_state_u32(receipt_base+3u) != PLASTICITY_STATUS_PREPARED) { return; }
  let extension = load_slot_extension(brain);
  let learning = load_slot_learning_state(extension);
  let outcome = load_outcome_credit(header.outcome_offset);
  let output_fast = increment_pair(vec2<u32>(learning.active_weight_generation_lo,learning.active_weight_generation_hi));
  let output_replay = increment_pair(vec2<u32>(learning.replay_generation_lo,learning.replay_generation_hi));
  let output_transaction = increment_pair(vec2<u32>(learning.transaction_generation_lo,learning.transaction_generation_hi));
  let generation_guard =
      plasticity_guard_bit(0u, !pair_nonzero(output_fast))
    | plasticity_guard_bit(1u, !pair_nonzero(output_replay))
    | plasticity_guard_bit(2u, !pair_nonzero(output_transaction));
  if (generation_guard != 0u) {
    reject_plasticity(receipt_base, PLASTICITY_GUARD_GENERATION_OVERFLOW, generation_guard, 0u, 0u); return;
  }
  let family = (outcome.selected_candidate_and_family>>16u)&0xffu;
  let event_base = learning.replay_event_rows_offset + learning.replay_cursor*24u;
  store_state_u32(event_base,outcome.sequence_id.x); store_state_u32(event_base+1u,outcome.sequence_id.y);
  store_state_u32(event_base+2u,outcome.originating_tick.x); store_state_u32(event_base+3u,outcome.originating_tick.y);
  for (var index=0u; index<8u; index+=1u) { store_state_u32(event_base+4u+index,outcome.frame_digest[index]); }
  for (var index=0u; index<4u; index+=1u) { store_state_u32(event_base+12u+index,outcome.candidate_feature_digest[index]); }
  store_state_u32(event_base+16u,outcome.selected_action); store_state_u32(event_base+17u,family);
  store_state_f32(event_base+18u,outcome.reward_prediction_error); store_state_f32(event_base+19u,outcome.pain);
  store_state_f32(event_base+20u,outcome.homeostatic_improvement); store_state_f32(event_base+21u,outcome.frustration);
  store_state_f32(event_base+22u,outcome.novelty); store_state_f32(event_base+23u,outcome.modulator_value);
  for (var word=0u; word<PENDING_ELIGIBILITY_WORDS_PLASTICITY; word+=1u) {
    store_state_u32(extension.pending_eligibility_offset+word,0u);
  }
  let state_base = extension.learning_state_offset;
  store_state_u32(state_base+1u,learning.active_weight_bank^1u);
  store_state_u32(state_base+2u,learning.active_eligibility_bank^1u);
  store_state_u32(state_base+4u,output_fast.x); store_state_u32(state_base+5u,output_fast.y);
  store_state_u32(state_base+6u,learning.inactive_eligibility_generation_lo);
  store_state_u32(state_base+7u,learning.inactive_eligibility_generation_hi);
  store_state_u32(state_base+8u,0u); store_state_u32(state_base+9u,0u);
  store_state_u32(state_base+10u,output_replay.x); store_state_u32(state_base+11u,output_replay.y);
  store_state_u32(state_base+12u,(learning.replay_cursor+1u)%learning.replay_event_capacity);
  store_state_u32(state_base+13u,min(learning.replay_event_count+1u,learning.replay_event_capacity));
  store_state_u32(state_base+22u,output_transaction.x); store_state_u32(state_base+23u,output_transaction.y);
  store_state_u32(state_base+3u,0u);
  store_state_u32(receipt_base+6u,output_fast.x); store_state_u32(receipt_base+7u,output_fast.y);
  store_state_u32(receipt_base+8u,learning.inactive_eligibility_generation_lo);
  store_state_u32(receipt_base+9u,learning.inactive_eligibility_generation_hi);
  store_state_u32(receipt_base+10u,output_replay.x); store_state_u32(receipt_base+11u,output_replay.y);
  store_state_u32(receipt_base+12u,output_transaction.x); store_state_u32(receipt_base+13u,output_transaction.y);
  storageBarrier();
  store_state_u32(receipt_base+3u,PLASTICITY_STATUS_SUCCESS);
}
