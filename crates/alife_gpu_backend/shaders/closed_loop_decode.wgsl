const ACTIVE_DISPATCH_ROW_WORDS:u32 = 332u;
const INVALID_LOGIT_BITS:u32 = 0x7fc00001u;
const DECODER_SCHEMA_VERSION:u32 = 1u;
const SELECTOR_DIAGNOSTIC_RECORD_WORDS:u32 = 29u;
const FACTORIZED_MOTOR_CHANNEL_SLOT_COUNT:u32 = 6u;
const ACTION_KIND_IDLE:u32 = 0u;
const ACTION_KIND_HOLD:u32 = 1u;
const ACTION_KIND_REST:u32 = 2u;
const ACTION_KIND_INSPECT:u32 = 3u;
const ACTION_KIND_MOVE:u32 = 4u;
const ACTION_KIND_INTERACT:u32 = 5u;
const ACTION_KIND_VOCALIZE:u32 = 6u;
const ACTION_KIND_WRITE:u32 = 7u;
const ACTION_KIND_GESTURE:u32 = 8u;
const ACTION_KIND_LOOK:u32 = 9u;

fn find_decoder_family(decoder:GpuDecoderPlanRecord, family_raw:u32) -> GpuDecoderFamilyRecord {
  if (family_raw < decoder.family_count) {
    let direct = load_decoder_family(decoder.family_offset + family_raw * 8u);
    if (direct.family_raw == family_raw) { return direct; }
  }
  for (var index=0u; index<decoder.family_count; index++) {
    let family = load_decoder_family(decoder.family_offset + index * 8u);
    if (family.family_raw == family_raw) { return family; }
  }
  return GpuDecoderFamilyRecord(0xffffffffu,0u,0u,0u,0u,0u,0u,0u);
}

fn finite_decode(value:f32) -> bool {
  return (bitcast<u32>(value) & 0x7f800000u) != 0x7f800000u;
}

fn span_within(start:u32, count:u32, limit:u32) -> bool {
  return start <= limit && count <= limit - start;
}

fn innate_family_contribution(
    header:GpuPerceptionHeader,
    candidate_record:GpuCandidateRecord,
    family:GpuDecoderFamilyRecord
) -> f32 {
  let mask = family.reserved0 & 0x1ffu;
  let gain = bitcast<f32>(family.reserved1);
  if (mask == 0u || gain == 0.0) { return 0.0; }
  var urgency = 0.0;
  for (var lane=0u; lane<9u; lane++) {
    if ((mask & (1u << lane)) != 0u) {
      urgency = max(urgency, bitcast<f32>(frame_payload_words[header.sensory_offset + 55u + lane]));
    }
  }
  var cue = 1.0;
  let cue_code = (family.reserved0 >> 16u) & 0xffu;
  if (cue_code != 0u) {
    let contact = bitcast<f32>(frame_payload_words[candidate_record.feature_offset + 18u]);
    let bearing_sin = bitcast<f32>(frame_payload_words[candidate_record.feature_offset]);
    let bearing_cos = bitcast<f32>(frame_payload_words[candidate_record.feature_offset + 1u]);
    let noncontact_bearing = (1.0 - contact) * clamp(abs(bearing_sin) + abs(bearing_cos), 0.0, 1.0);
    var signal = 0.0;
    if (cue_code == 25u) {
      let chem = bitcast<f32>(frame_payload_words[candidate_record.feature_offset + 15u]);
      var valence_scale = 0.75;
      if (chem < -0.2) {
        valence_scale = 0.0;
      } else if (chem > 0.2) {
        valence_scale = 1.0;
      }
      let scaled_bearing = noncontact_bearing * valence_scale;
      signal = select(scaled_bearing, -scaled_bearing, (family.reserved0 & 0x02000000u) != 0u);
    } else {
      let feature = bitcast<f32>(frame_payload_words[candidate_record.feature_offset + cue_code - 1u]);
      let inverted = (family.reserved0 & 0x02000000u) != 0u;
      signal = select(feature, -feature, inverted);
    }
    // Bound contact chemistry keeps its original meaning. Before contact,
    // danger urgency can instead withdraw from actually observed geometry.
    if (family.family_raw == 4u && cue_code == 16u
        && (family.reserved0 & 0x02000000u) != 0u
        && (family.reserved0 & 0x01000000u) == 0u
        && bitcast<f32>(frame_payload_words[candidate_record.feature_offset + 18u]) == 0.0) {
      let bearing_sin = bitcast<f32>(frame_payload_words[candidate_record.feature_offset]);
      let bearing_cos = bitcast<f32>(frame_payload_words[candidate_record.feature_offset + 1u]);
      signal = clamp(abs(bearing_sin) + abs(bearing_cos), 0.0, 1.0);
    }
    cue = clamp((signal - 0.4) / 0.6, 0.0, 1.0);
  }
  var reach = 1.0;
  if ((family.reserved0 & 0x01000000u) != 0u) {
    let distance = bitcast<f32>(frame_payload_words[candidate_record.feature_offset + 2u]);
    reach = clamp((0.25 - distance) / 0.15, 0.0, 1.0);
  }
  return max(0.0, urgency - 0.55) * gain * cue * reach;
}

var<workgroup> wg_dec_header: GpuPerceptionHeader;
var<workgroup> wg_dec_brain: GpuBrainSlotRecord;
var<workgroup> wg_dec_weight_bases: GpuWeightBankBases;
var<workgroup> wg_dec_decoder: GpuDecoderPlanRecord;
var<workgroup> wg_dec_final_side: u32;
var<workgroup> wg_dec_activation_offset: u32;
var<workgroup> wg_dec_valid: u32;
var<workgroup> wg_dec_base_valid: u32;
var<workgroup> wg_dec_use_shared_motor: u32;
var<workgroup> wg_dec_frame_words: u32;
var<workgroup> wg_dec_motor_activations: array<f32, 512>;

@compute @workgroup_size(64)
fn decode_candidates(
  @builtin(global_invocation_id) gid:vec3<u32>,
  @builtin(local_invocation_id) lid:vec3<u32>,
) {
  let tid = lid.x;
  if (tid == 0u) {
    wg_dec_valid = 0u;
    wg_dec_base_valid = 0u;
    wg_dec_use_shared_motor = 0u;
    let header = load_perception_header(gid.y * ACTIVE_DISPATCH_ROW_WORDS);
    if (activity_contract_prevalidated(header)) {
      let brain = brain_slots[header.brain_slot_index];
      let decoder = load_decoder_plan(brain.decoder_plan_offset);
      let final_side = atomicLoad(&atomic_state_words[brain.diagnostic_offset + 3u]);
      let frame_word_count = arrayLength(&frame_payload_words);
      let base_valid = final_side <= 1u
        && decoder.schema_version == DECODER_SCHEMA_VERSION
        && decoder.feature_count == 24u
        && decoder.flattened_input_lane_count >= decoder.feature_count
        && decoder.flattened_input_lane_count <= 64u
        && decoder.family_offset == brain.decoder_family_offset
        && decoder.family_count == 8u
        && span_within(decoder.motor_start, decoder.motor_width, brain.neuron_count)
        && brain.recurrent_synapse_count <= brain.synapse_count
        && span_within(
          brain.recurrent_synapse_count,
          decoder.decoder_synapse_count,
          brain.synapse_count
        );
      let activation_offset = select(brain.activation_a_offset, brain.activation_b_offset, final_side == 1u);
      wg_dec_header = header;
      wg_dec_brain = brain;
      wg_dec_weight_bases = load_weight_bank_pair_direct(brain).active_bases;
      wg_dec_decoder = decoder;
      wg_dec_final_side = final_side;
      wg_dec_activation_offset = activation_offset;
      wg_dec_frame_words = frame_word_count;
      wg_dec_base_valid = select(0u, 1u, base_valid);
      wg_dec_use_shared_motor = select(0u, 1u, base_valid && decoder.motor_width <= 512u);
      wg_dec_valid = 1u;
    }
  }
  workgroupBarrier();
  if (wg_dec_valid == 0u) { return; }

  let use_shared_motor = wg_dec_use_shared_motor != 0u;
  let decoder = wg_dec_decoder;
  let activation_offset = wg_dec_activation_offset;
  if (use_shared_motor) {
    for (var m = tid; m < decoder.motor_width; m += 64u) {
      wg_dec_motor_activations[m] = load_state_f32(activation_offset + decoder.motor_start + m);
    }
  }
  workgroupBarrier();

  let header = wg_dec_header;
  let candidate = gid.x;
  if (candidate >= header.candidate_count) { return; }
  let brain = wg_dec_brain;
  let weight_bases = wg_dec_weight_bases;
  let final_side = wg_dec_final_side;
  let candidate_record = load_candidate(header.candidate_offset + candidate * 8u);
  let frame_word_count = wg_dec_frame_words;
  var valid = wg_dec_base_valid != 0u
    && candidate_record.candidate_index == candidate
    && span_within(candidate_record.feature_offset, 24u, frame_word_count);
  var family = GpuDecoderFamilyRecord(0xffffffffu,0u,0u,0u,0u,0u,0u,0u);
  if (valid) { family = find_decoder_family(decoder, candidate_record.family); }
  valid = valid && family.family_raw == candidate_record.family
    && family.weight_index_count == family.decoder_synapse_count
    && (family.reserved0 & 0xfc00fe00u) == 0u
    && ((family.reserved0 >> 16u) & 0xffu) <= decoder.feature_count + 1u
    && (((family.reserved0 >> 16u) & 0xffu) != 25u
      || (family.family_raw == 3u && (family.reserved0 & 0x03000000u) == 0u))
    && finite_decode(bitcast<f32>(family.reserved1))
    && family.decoder_synapse_start >= brain.recurrent_synapse_count
    && span_within(family.decoder_synapse_start, family.decoder_synapse_count, brain.synapse_count)
    && family.weight_index_start >= brain.decoder_weight_indices_offset
    && family.weight_index_count <= 0x3fffffffu
    && decoder.decoder_synapse_count <= 0x3fffffffu
    && span_within(
      family.weight_index_start - brain.decoder_weight_indices_offset,
      family.weight_index_count * 2u,
      decoder.decoder_synapse_count * 2u
    );
  var logit = bitcast<f32>(family.bias_bits);
  if (valid) {
    logit += innate_family_contribution(header, candidate_record, family);
    valid = finite_decode(logit);
  }
  var diagnostic_requested = false;
  var diagnostic_base = 0u;
  var diagnostic_synapse_count = 0u;
  if (header.reserved != 0u) {
    let request_count = frame_payload_words[header.reserved];
    valid = valid && request_count > 0u && request_count <= 8u;
    for (var request = 0u; request < request_count && valid; request++) {
      let request_base = header.reserved + 1u + request * 3u;
      if (frame_payload_words[request_base] == candidate) {
        diagnostic_requested = true;
        diagnostic_base = header.reserved + frame_payload_words[request_base + 1u];
        diagnostic_synapse_count = frame_payload_words[request_base + 2u];
      }
    }
    valid = valid && (!diagnostic_requested || diagnostic_synapse_count == family.weight_index_count);
  }
  for (var index=0u; index<family.weight_index_count && valid; index++) {
    let map = load_decoder_weight_index(family.weight_index_start + index * 2u);
    let input_lane = map.packed_lane_motor & 0xffffu;
    let motor_index = map.packed_lane_motor >> 16u;
    valid = input_lane < decoder.feature_count
      && motor_index < decoder.motor_width
      && map.global_synapse_id == family.decoder_synapse_start + index
      && map.global_synapse_id >= brain.recurrent_synapse_count
      && map.global_synapse_id < brain.synapse_count;
    if (valid) {
      let motor = select(
        load_state_f32(activation_offset + decoder.motor_start + motor_index),
        wg_dec_motor_activations[motor_index],
        use_shared_motor
      );
      let feature = bitcast<f32>(frame_payload_words[candidate_record.feature_offset + input_lane]);
      var effective = 0.0;
      if (diagnostic_requested) {
        let genetic = bitcast<f32>(immutable_weight_words[brain.genetic_weight_offset + map.global_synapse_id]);
        let alpha = bitcast<f32>(immutable_weight_words[brain.alpha_offset + map.global_synapse_id]);
        let lifetime = load_state_f32(weight_bases.lifetime + map.global_synapse_id);
        let fast = load_state_f32(weight_bases.fast + map.global_synapse_id);
        effective = genetic + lifetime + alpha * fast;
        let contribution = motor * feature * effective;
        logit += contribution;
        let receipt_base = diagnostic_base + index * SELECTOR_DIAGNOSTIC_RECORD_WORDS;
        frame_payload_words[receipt_base] = candidate;
        frame_payload_words[receipt_base + 1u] = index;
        frame_payload_words[receipt_base + 2u] = map.global_synapse_id;
        frame_payload_words[receipt_base + 3u] = input_lane;
        frame_payload_words[receipt_base + 4u] = motor_index;
        frame_payload_words[receipt_base + 5u] = bitcast<u32>(motor);
        frame_payload_words[receipt_base + 6u] = bitcast<u32>(feature);
        frame_payload_words[receipt_base + 7u] = bitcast<u32>(genetic);
        frame_payload_words[receipt_base + 8u] = bitcast<u32>(lifetime);
        frame_payload_words[receipt_base + 9u] = bitcast<u32>(alpha);
        frame_payload_words[receipt_base + 10u] = bitcast<u32>(fast);
        frame_payload_words[receipt_base + 11u] = bitcast<u32>(effective);
        frame_payload_words[receipt_base + 12u] = bitcast<u32>(contribution);
        frame_payload_words[receipt_base + 13u] = bitcast<u32>(logit);
        frame_payload_words[receipt_base + 14u] = final_side;
        frame_payload_words[receipt_base + 15u] = activation_offset;
        frame_payload_words[receipt_base + 16u] = decoder.motor_start;
        frame_payload_words[receipt_base + 17u] = candidate_record.feature_offset;
        frame_payload_words[receipt_base + 18u] = brain.decoder_plan_offset;
        frame_payload_words[receipt_base + 19u] = family.decoder_synapse_start;
        frame_payload_words[receipt_base + 20u] = family.decoder_synapse_count;
        frame_payload_words[receipt_base + 21u] = family.weight_index_start;
        frame_payload_words[receipt_base + 22u] = family.weight_index_count;
        frame_payload_words[receipt_base + 23u] = brain.genetic_weight_offset;
        frame_payload_words[receipt_base + 24u] = brain.alpha_offset;
        frame_payload_words[receipt_base + 25u] = weight_bases.lifetime;
        frame_payload_words[receipt_base + 26u] = weight_bases.fast;
        frame_payload_words[receipt_base + 27u] = family.family_raw;
        frame_payload_words[receipt_base + 28u] = brain.decoder_family_offset;
      } else {
        effective = load_state_f32(weight_bases.effective + map.global_synapse_id);
        logit += motor * feature * effective;
      }
      valid = finite_decode(logit);
    }
  }
  if (!valid || !finite_decode(logit)) {
    store_state_u32(brain.candidate_logit_offset + candidate, INVALID_LOGIT_BITS);
    atomicAdd(&atomic_state_words[brain.diagnostic_offset + 2u], 1u);
    return;
  }
  store_state_f32(brain.candidate_logit_offset + candidate, logit);
}

// Game-only bounded exploration, not the fixed-temperature PPO sampler.
// Provisional absolute logit units deliberately preserve strong preferences:
// each perturbation is within [-1, 1], so a gap > 2 cannot be overturned.
const GAME_EXPLORATION_MAX_SCALE:f32 = 0.5;
const GAME_EXPLORATION_NOISE_LIMIT:f32 = 2.0;
const GAME_EXPLORATION_CRITICAL_NEED:f32 = 0.65;
const GAME_EXPLORATION_LEARNING_HEADER_OFFSET:u32 = 272u;
const GAME_EXPLORATION_COGNITIVE_LANE_END:u32 = 54u;
const GAME_EXPLORATION_GAP_LANE:u32 = 50u;

struct GpuGameExplorationState {
  organism_id:vec2<u32>, tick:vec2<u32>,
  novelty:f32, curiosity:f32, urgency:f32,
}

fn game_exploration_enabled(header:GpuPerceptionHeader) -> bool {
  // The feature-composed training shader replaces this single checked hook.
  return true;
}

fn game_exploration_unit(value:f32) -> bool {
  return finite_decode(value) && value >= 0.0 && value <= 1.0;
}

fn game_exploration_mix(input:u32) -> u32 {
  var value = input;
  value = (value ^ (value >> 16u)) * 0x7feb352du;
  value = (value ^ (value >> 15u)) * 0x846ca68bu;
  return value ^ (value >> 16u);
}

fn game_exploration_noise(state:GpuGameExplorationState, identity:vec4<u32>) -> f32 {
  // Durable logical identity only. Slots, packed offsets, dispatch generation,
  // request order, and retry count must never enter this counter-based draw.
  var value = 0x6d2b79f5u;
  value = game_exploration_mix(value ^ state.organism_id.x);
  value = game_exploration_mix(value ^ state.organism_id.y);
  value = game_exploration_mix(value ^ state.tick.x);
  value = game_exploration_mix(value ^ state.tick.y);
  value = game_exploration_mix(value ^ identity.x);
  value = game_exploration_mix(value ^ identity.y);
  value = game_exploration_mix(value ^ identity.z);
  value = game_exploration_mix(value ^ identity.w);
  // 23-bit midpoints are strictly inside (0,1), including in f32.
  let uniform = (f32(value >> 9u) + 0.5) / 8388608.0;
  return GAME_EXPLORATION_NOISE_LIMIT * (2.0 * uniform - 1.0);
}

fn game_exploration_state(header:GpuPerceptionHeader, row_base:u32) -> GpuGameExplorationState {
  let learning = load_learning_header(row_base + GAME_EXPLORATION_LEARNING_HEADER_OFFSET);
  // Host validates these exact pending-template bytes before dispatch. Reuse
  // their durable organism identity; no new organism/RNG state or readback is added.
  let pending = learning.outcome_offset;
  let owner = vec2<u32>(frame_payload_words[pending + 12u], frame_payload_words[pending + 13u]);
  let novelty = bitcast<f32>(frame_payload_words[header.sensory_offset + 41u]);
  let hunger = bitcast<f32>(frame_payload_words[header.sensory_offset + 55u]);
  let fatigue = bitcast<f32>(frame_payload_words[header.sensory_offset + 56u]);
  let fear = bitcast<f32>(frame_payload_words[header.sensory_offset + 57u]);
  let pain = bitcast<f32>(frame_payload_words[header.sensory_offset + 58u]);
  let curiosity = bitcast<f32>(frame_payload_words[header.sensory_offset + 60u]);
  let temperature_stress = bitcast<f32>(frame_payload_words[header.sensory_offset + 62u]);
  let critical_need = clamp(
    (max(hunger, max(fatigue, temperature_stress)) - GAME_EXPLORATION_CRITICAL_NEED)
      / (1.0 - GAME_EXPLORATION_CRITICAL_NEED), 0.0, 1.0);
  return GpuGameExplorationState(owner, vec2<u32>(header.tick_lo, header.tick_hi),
    novelty, curiosity, max(max(fear, pain), critical_need));
}

fn game_exploration_state_valid(header:GpuPerceptionHeader, row_base:u32) -> bool {
  let learning = load_learning_header(row_base + GAME_EXPLORATION_LEARNING_HEADER_OFFSET);
  let count = arrayLength(&frame_payload_words);
  if (!span_within(learning.outcome_offset, 36u + header.candidate_count * 4u, count)
      || !span_within(header.sensory_offset, 77u, count)) { return false; }
  let pending = learning.outcome_offset;
  if (frame_payload_words[pending + 12u] == 0u && frame_payload_words[pending + 13u] == 0u) { return false; }
  if (frame_payload_words[pending + 16u] != header.tick_lo
      || frame_payload_words[pending + 17u] != header.tick_hi) { return false; }
  let lanes = array<u32,7>(41u,55u,56u,57u,58u,60u,62u);
  for (var index=0u; index<7u; index++) {
    if (!game_exploration_unit(bitcast<f32>(frame_payload_words[header.sensory_offset + lanes[index]]))) {
      return false;
    }
  }
  return true;
}

fn game_candidate_exploration_scale(state:GpuGameExplorationState, stride:u32, candidate:GpuCandidateRecord) -> f32 {
  var gap = 0.0;
  if (stride >= GAME_EXPLORATION_COGNITIVE_LANE_END) {
    gap = bitcast<f32>(frame_payload_words[candidate.feature_offset + GAME_EXPLORATION_GAP_LANE]);
    if (!game_exploration_unit(gap)) { return bitcast<f32>(INVALID_LOGIT_BITS); }
  }
  // This is the existing candidate/target-bound unresolved gap. The adjacent
  // legacy `prior_residual` lane is remembered outcome magnitude, not error.
  let evidence = max(state.novelty, gap);
  return GAME_EXPLORATION_MAX_SCALE * evidence * (0.6 + 0.4 * state.curiosity) * (1.0 - state.urgency);
}

fn game_candidate_arbitration_score(state:GpuGameExplorationState, stride:u32, identity_base:u32, candidate:GpuCandidateRecord, logit:f32) -> f32 {
  let scale = game_candidate_exploration_scale(state, stride, candidate);
  if (!finite_decode(scale)) { return bitcast<f32>(INVALID_LOGIT_BITS); }
  if (scale == 0.0) { return logit; }
  let base = identity_base + candidate.candidate_index * 4u;
  let identity = vec4<u32>(frame_payload_words[base], frame_payload_words[base + 1u],
    frame_payload_words[base + 2u], frame_payload_words[base + 3u]);
  return logit + scale * game_exploration_noise(state, identity);
}

const DECODER_HEAD_SPEECH_PAYLOAD:u32 = 3u;
const SPEECH_INPUT_WIDTH:u32 = 32u;
const SPEECH_OUTPUT_WIDTH:u32 = 32u;
const SPEECH_SOURCE_OFFSET:u32 = 128u;
const SPEECH_TARGET_OFFSET:u32 = 160u;
const SPEECH_SYNAPSE_COUNT:u32 = 1024u;
const MAX_SPEECH_TOKENS:u32 = 6u;

fn factorized_motor_slot(kind:u32) -> u32 {
  if (kind == ACTION_KIND_MOVE) { return 0u; }
  if (kind == ACTION_KIND_LOOK) { return 1u; }
  if (kind == ACTION_KIND_INTERACT || kind == ACTION_KIND_WRITE) { return 2u; }
  if (kind == ACTION_KIND_VOCALIZE) { return 3u; }
  if (kind == ACTION_KIND_HOLD || kind == ACTION_KIND_REST || kind == ACTION_KIND_INSPECT) {
    return 4u;
  }
  return 0xffffffffu;
}

var<workgroup> wg_sel_header: GpuPerceptionHeader;
var<workgroup> wg_sel_brain: GpuBrainSlotRecord;
var<workgroup> wg_sel_output_base: u32;
var<workgroup> wg_sel_valid: u32;
var<workgroup> wg_sel_exploratory: u32;
var<workgroup> wg_sel_exploration: GpuGameExplorationState;
var<workgroup> wg_sel_stride: u32;
var<workgroup> wg_sel_identity_base: u32;
var<workgroup> wg_sel_err: atomic<u32>;
var<workgroup> wg_sel_found: array<u32, 32>;
var<workgroup> wg_sel_cand: array<u32, 32>;
var<workgroup> wg_sel_logit: array<f32, 32>;
var<workgroup> wg_sel_score: array<f32, 32>;
var<workgroup> wg_sel_conf: array<u32, 32>;
var<workgroup> wg_sel_slot: array<u32, 32>;
var<workgroup> wg_motor_found: array<u32, 6>;
var<workgroup> wg_motor_cand: array<u32, 6>;

@compute @workgroup_size(32)
fn select_candidate(
    @builtin(global_invocation_id) gid:vec3<u32>,
    @builtin(local_invocation_id) lid:vec3<u32>,
) {
  let tid = lid.x;
  let row_base = gid.y * ACTIVE_DISPATCH_ROW_WORDS;
  if (tid == 0u) {
    wg_sel_valid = 0u;
    atomicStore(&wg_sel_err, 0u);
    let header = load_perception_header(row_base);
    if (activity_contract_prevalidated(header)) {
      let brain = brain_slots[header.brain_slot_index];
      let output_base = load_state_u32(brain.extension_record_offset + 18u);
      if (atomic_span_within(output_base, 4u)) {
        store_atomic_u32(output_base, 0u);
        store_atomic_u32(output_base + 1u, 0u);
        store_atomic_u32(output_base + 2u, 0u);
        store_atomic_u32(output_base + 3u, 0u);
      }
      wg_sel_header = header;
      wg_sel_brain = brain;
      wg_sel_output_base = output_base;
      wg_sel_stride = load_decoder_plan(brain.decoder_plan_offset).flattened_input_lane_count;
      wg_sel_identity_base = load_learning_header(row_base
        + GAME_EXPLORATION_LEARNING_HEADER_OFFSET).outcome_offset + 36u;
      let exploratory = game_exploration_enabled(header);
      if (exploratory) {
        if (!game_exploration_state_valid(header, row_base)) {
          atomicOr(&atomic_state_words[brain.diagnostic_offset + 2u], CONTRACT_INVALID_DIAGNOSTIC_BIT);
        } else {
          wg_sel_exploration = game_exploration_state(header, row_base);
          wg_sel_exploratory = 1u;
          wg_sel_valid = 1u;
        }
      } else {
        wg_sel_exploratory = 0u;
        wg_sel_valid = 1u;
      }
    }
  }
  workgroupBarrier();
  if (wg_sel_valid == 0u) { return; }

  let header = wg_sel_header;
  let brain = wg_sel_brain;
  let exploratory = wg_sel_exploratory != 0u;
  var my_found = 0u;
  var my_cand = 0xffffffffu;
  var my_logit = 0.0;
  var my_score = 0.0;
  var my_conf = 0u;
  var my_slot = 0xffffffffu;
  if (tid < header.candidate_count) {
    let bits = load_state_u32(brain.candidate_logit_offset + tid);
    if (bits != INVALID_LOGIT_BITS) {
      let logit = bitcast<f32>(bits);
      if (finite_decode(logit)) {
        let candidate_record = load_candidate(header.candidate_offset + tid * 8u);
        var score = logit;
        if (exploratory) {
          score = game_candidate_arbitration_score(
            wg_sel_exploration,
            wg_sel_stride,
            wg_sel_identity_base,
            candidate_record,
            logit
          );
        }
        if (!finite_decode(score)) {
          atomicStore(&wg_sel_err, 1u);
        } else {
          my_found = 1u;
          my_cand = tid;
          my_logit = logit;
          my_score = score;
          my_conf = candidate_record.confidence_q16;
          my_slot = factorized_motor_slot(candidate_record.kind);
        }
      }
    }
  }
  wg_sel_found[tid] = my_found;
  wg_sel_cand[tid] = my_cand;
  wg_sel_logit[tid] = my_logit;
  wg_sel_score[tid] = my_score;
  wg_sel_conf[tid] = my_conf;
  wg_sel_slot[tid] = my_slot;
  workgroupBarrier();

  if (atomicLoad(&wg_sel_err) != 0u) {
    if (tid == 0u) {
      atomicOr(&atomic_state_words[brain.diagnostic_offset + 2u], CONTRACT_INVALID_DIAGNOSTIC_BIT);
    }
    return;
  }

  let joint_mode = brain.reserved[0] != 0u;
  let enabled_channels = brain.reserved[0] & 255u;
  if (tid < FACTORIZED_MOTOR_CHANNEL_SLOT_COUNT) {
    var m_found = 0u;
    var m_cand = 0u;
    var m_score = 0.0;
    if (!joint_mode || (enabled_channels & (1u << tid)) != 0u) {
      for (var c = 0u; c < header.candidate_count; c++) {
        if (wg_sel_found[c] != 0u && wg_sel_slot[c] == tid) {
          let s = wg_sel_score[c];
          if (m_found == 0u || s > m_score || (s == m_score && c < m_cand)) {
            m_found = 1u;
            m_cand = c;
            m_score = s;
          }
        }
      }
    }
    wg_motor_found[tid] = m_found;
    wg_motor_cand[tid] = m_cand;
  }
  workgroupBarrier();

  for (var stride = 16u; stride > 0u; stride >>= 1u) {
    if (tid < stride) {
      let other = tid + stride;
      if (wg_sel_found[other] != 0u) {
        let take_other = wg_sel_found[tid] == 0u
          || wg_sel_score[other] > wg_sel_score[tid]
          || (wg_sel_score[other] == wg_sel_score[tid] && wg_sel_cand[other] < wg_sel_cand[tid]);
        if (take_other) {
          wg_sel_found[tid] = 1u;
          wg_sel_cand[tid] = wg_sel_cand[other];
          wg_sel_logit[tid] = wg_sel_logit[other];
          wg_sel_score[tid] = wg_sel_score[other];
          wg_sel_conf[tid] = wg_sel_conf[other];
        }
      }
    }
    workgroupBarrier();
  }

  let base = brain.selection_offset;
  if (tid == 0u) {
    let found = wg_sel_found[0] != 0u;
    let selected_candidate = wg_sel_cand[0];
    let selected_logit = wg_sel_logit[0];
    let selected_confidence = wg_sel_conf[0];
    atomicStore(&atomic_state_words[base], brain.slot);
    atomicStore(&atomic_state_words[base + 1u], brain.slot_generation);
    atomicStore(&atomic_state_words[base + 2u], select(0xffffffffu, selected_candidate, found));
    atomicStore(&atomic_state_words[base + 3u], select(0u, bitcast<u32>(selected_logit), found));
    atomicStore(&atomic_state_words[base + 4u], select(0u, selected_confidence, found));
    atomicStore(&atomic_state_words[base + 5u], select(2u, 1u, found));
    atomicStore(&atomic_state_words[base + 6u], atomicLoad(&atomic_state_words[brain.diagnostic_offset]));
    atomicStore(&atomic_state_words[base + 7u], atomicLoad(&atomic_state_words[brain.diagnostic_offset + 1u]));
    atomicStore(&atomic_state_words[base + 8u], atomicLoad(&atomic_state_words[brain.diagnostic_offset + 2u]));
    atomicStore(&atomic_state_words[base + 9u], header.dispatch_generation_lo);
    atomicStore(&atomic_state_words[base + 10u], header.dispatch_generation_hi);
    atomicStore(&atomic_state_words[base + 11u], atomicLoad(&atomic_state_words[brain.diagnostic_offset + 3u]));

    let output_base = wg_sel_output_base;
    if (atomic_span_within(output_base, 4u)) {
      var motor_words:array<u32,2>;
      motor_words[0] = 0u;
      motor_words[1] = 0u;
      var motor_found:array<bool,6>;
      var motor_candidates:array<u32,6>;
      for (var slot = 0u; slot < FACTORIZED_MOTOR_CHANNEL_SLOT_COUNT; slot++) {
        motor_found[slot] = wg_motor_found[slot] != 0u;
        motor_candidates[slot] = wg_motor_cand[slot];
      }
      if (joint_mode && found && selected_candidate < header.candidate_count) {
        let selected = load_candidate(header.candidate_offset + selected_candidate * 8u);
        var slot = factorized_motor_slot(selected.kind);
        if (selected.kind == ACTION_KIND_IDLE || selected.kind == ACTION_KIND_GESTURE) { slot = 4u; }
        if (slot < FACTORIZED_MOTOR_CHANNEL_SLOT_COUNT) {
          motor_found[slot] = true;
          motor_candidates[slot] = selected_candidate;
        }
        motor_words[1] = JOINT_SELECTION_V1_MARKER;
      }
      for (var slot = 0u; slot < FACTORIZED_MOTOR_CHANNEL_SLOT_COUNT; slot++) {
        if (motor_found[slot] && motor_candidates[slot] < 255u) {
          let encoded = motor_candidates[slot] + 1u;
          motor_words[slot / 4u] |= encoded << ((slot % 4u) * 8u);
        }
      }
      store_atomic_u32(output_base + 2u, motor_words[0]);
      store_atomic_u32(output_base + 3u, motor_words[1]);
    }
  }
}

fn load_speech_selection(base:u32) -> GpuSelectionRecord {
  return GpuSelectionRecord(
    load_atomic_u32(base),load_atomic_u32(base+1u),load_atomic_u32(base+2u),load_atomic_u32(base+3u),
    load_atomic_u32(base+4u),load_atomic_u32(base+5u),load_atomic_u32(base+6u),load_atomic_u32(base+7u),
    load_atomic_u32(base+8u),load_atomic_u32(base+9u),load_atomic_u32(base+10u),load_atomic_u32(base+11u),
    load_atomic_u32(base+12u),load_atomic_u32(base+13u),load_atomic_u32(base+14u),load_atomic_u32(base+15u)
  );
}

var<workgroup> wg_sp_brain: GpuBrainSlotRecord;
var<workgroup> wg_sp_weight_bases: GpuWeightBankBases;
var<workgroup> wg_sp_output_base: u32;
var<workgroup> wg_sp_source_start: u32;
var<workgroup> wg_sp_activation_base: u32;
var<workgroup> wg_sp_active: u32;
var<workgroup> wg_sp_inputs: array<f32, 32>;
var<workgroup> wg_sp_outputs: array<f32, 32>;
var<workgroup> wg_sp_invalid: atomic<u32>;
var<workgroup> wg_sp_stop: u32;

// Runs after candidate selection in the same authoritative command buffer.
// The host receives only the packed act/token receipt and never reads neural
// activations or authors creature speech.
@compute @workgroup_size(32)
fn decode_speech_payload(
    @builtin(global_invocation_id) gid:vec3<u32>,
    @builtin(local_invocation_id) lid:vec3<u32>,
) {
  let tid = lid.x;
  if (tid == 0u) {
    wg_sp_active = 0u;
    atomicStore(&wg_sp_invalid, 0u);
    let header = load_perception_header(gid.y * ACTIVE_DISPATCH_ROW_WORDS);
    if (activity_contract_prevalidated(header)) {
      let brain = brain_slots[header.brain_slot_index];
      let output_base = load_state_u32(brain.extension_record_offset + 18u);
      if (atomic_span_within(output_base, 4u)) {
        let selection = load_speech_selection(brain.selection_offset);
        if (selection.status == 1u && selection.candidate_index < header.candidate_count) {
          let joint_mode = brain.reserved[0] != 0u;
          let motor_words_0 = load_atomic_u32(output_base + 2u);
          let vocal_motor_found = ((motor_words_0 >> 24u) & 255u) != 0u;
          let selected = load_candidate(header.candidate_offset + selection.candidate_index * 8u);
          let vocal_selected = select(selected.kind == ACTION_KIND_VOCALIZE, vocal_motor_found, joint_mode);
          if (vocal_selected && brain.reserved[1] != 0xffffffffu) {
            let decoder = load_decoder_plan(brain.decoder_plan_offset);
            let source_start = decoder.motor_start + SPEECH_SOURCE_OFFSET;
            let target_start = decoder.motor_start + SPEECH_TARGET_OFFSET;
            let decoder_count = brain.synapse_count - brain.recurrent_synapse_count;
            if (span_within(source_start, SPEECH_INPUT_WIDTH, brain.neuron_count)
                && span_within(target_start, SPEECH_OUTPUT_WIDTH, brain.neuron_count)
                && brain.reserved[1] <= decoder_count
                && SPEECH_SYNAPSE_COUNT <= decoder_count - brain.reserved[1]) {
              wg_sp_brain = brain;
              wg_sp_weight_bases = load_weight_bank_pair_direct(brain).active_bases;
              wg_sp_output_base = output_base;
              wg_sp_source_start = source_start;
              wg_sp_activation_base = select(
                brain.activation_a_offset,
                brain.activation_b_offset,
                selection.active_activation_side == 1u
              );
              wg_sp_active = 1u;
            }
          }
        }
      }
    }
  }
  workgroupBarrier();
  if (wg_sp_active == 0u) { return; }

  let brain = wg_sp_brain;
  let weight_bases = wg_sp_weight_bases;
  let speech_start = brain.reserved[1];
  wg_sp_inputs[tid] = load_state_f32(wg_sp_activation_base + wg_sp_source_start + tid);
  workgroupBarrier();

  var speech_act = 0u;
  var token_count = 0u;
  var confidence_byte = 0u;
  var tokens:array<u32,6>;
  for (var step = 0u; step < MAX_SPEECH_TOKENS; step++) {
    var acc = 0.0;
    for (var lane = 0u; lane < SPEECH_INPUT_WIDTH; lane++) {
      let global_synapse_id = brain.recurrent_synapse_count + speech_start + lane * SPEECH_OUTPUT_WIDTH + tid;
      acc += wg_sp_inputs[lane] * load_state_f32(weight_bases.effective + global_synapse_id);
      if (!finite_decode(acc)) {
        atomicStore(&wg_sp_invalid, 1u);
        break;
      }
    }
    let out_val = tanh(acc);
    if (!finite_decode(out_val)) {
      atomicStore(&wg_sp_invalid, 1u);
    }
    wg_sp_outputs[tid] = out_val;
    workgroupBarrier();
    if (atomicLoad(&wg_sp_invalid) != 0u) { return; }

    if (tid == 0u) {
      if (step == 0u) {
        var best = wg_sp_outputs[0];
        for (var act = 1u; act < 8u; act++) {
          if (wg_sp_outputs[act] > best) {
            best = wg_sp_outputs[act];
            speech_act = act;
          }
        }
        confidence_byte = u32(round(clamp(abs(best), 0.0, 1.0) * 255.0));
      }
      if (wg_sp_outputs[16] > 0.0) {
        var token = 0u;
        for (var bit = 0u; bit < 8u; bit++) {
          if (wg_sp_outputs[8u + bit] > 0.0) { token |= 1u << bit; }
        }
        if (token != 0u) {
          tokens[token_count] = token;
          token_count += 1u;
        }
      }
      wg_sp_stop = select(0u, 1u, wg_sp_outputs[17] > 0.0);
    }
    workgroupBarrier();
    if (wg_sp_stop != 0u) { break; }
    if (tid >= 18u) {
      wg_sp_inputs[tid] = wg_sp_outputs[tid];
    }
    workgroupBarrier();
  }

  if (tid == 0u && token_count != 0u) {
    let word0 = 1u
      | (speech_act << 2u)
      | (token_count << 5u)
      | (tokens[0] << 8u)
      | (tokens[1] << 16u)
      | (tokens[2] << 24u);
    let word1 = tokens[3]
      | (tokens[4] << 8u)
      | (tokens[5] << 16u)
      | (confidence_byte << 24u);
    store_atomic_u32(wg_sp_output_base, word0);
    store_atomic_u32(wg_sp_output_base + 1u, word1);
  }
}
