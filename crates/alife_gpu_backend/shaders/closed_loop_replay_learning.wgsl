const SLEEP_STATUS_PREPARED:u32 = 256u;
const SLEEP_STATUS_REJECTED:u32 = 512u;

fn sleep_frame_span_within(start:u32, count:u32) -> bool {
  let limit = arrayLength(&frame_payload_words);
  return start <= limit && count <= limit - start;
}

fn sleep_immutable_plan_span_within(start:u32, count:u32) -> bool {
  let limit = arrayLength(&immutable_plan_words);
  return start <= limit && count <= limit - start;
}

fn sleep_finite(value:f32) -> bool {
  return value == value && abs(value) <= 3.402823466e+38;
}

fn sleep_reject(completion:u32) {
  if (state_span_within(completion,16u)) {
    atomicMax(&mutable_state_words[completion+3u],SLEEP_STATUS_REJECTED);
  }
}

fn load_sleep_replay_span(base:u32) -> GpuReplaySynapseSpanRecord {
  return GpuReplaySynapseSpanRecord(
    frame_payload_words[base],frame_payload_words[base+1u],
    frame_payload_words[base+2u],frame_payload_words[base+3u]
  );
}

fn load_sleep_replay_event(base:u32) -> GpuReplayEventRecord {
  return GpuReplayEventRecord(
    vec2<u32>(frame_payload_words[base],frame_payload_words[base+1u]),
    vec2<u32>(frame_payload_words[base+2u],frame_payload_words[base+3u]),
    array<u32,8>(
      frame_payload_words[base+4u],frame_payload_words[base+5u],frame_payload_words[base+6u],frame_payload_words[base+7u],
      frame_payload_words[base+8u],frame_payload_words[base+9u],frame_payload_words[base+10u],frame_payload_words[base+11u]
    ),
    vec4<u32>(frame_payload_words[base+12u],frame_payload_words[base+13u],frame_payload_words[base+14u],frame_payload_words[base+15u]),
    frame_payload_words[base+16u],frame_payload_words[base+17u],
    bitcast<f32>(frame_payload_words[base+18u]),bitcast<f32>(frame_payload_words[base+19u]),
    bitcast<f32>(frame_payload_words[base+20u]),bitcast<f32>(frame_payload_words[base+21u]),
    bitcast<f32>(frame_payload_words[base+22u]),bitcast<f32>(frame_payload_words[base+23u])
  );
}

fn unpack_sleep_eligibility(word:u32) -> vec2<f32> {
  let event_index = f32(word & 0xffffu);
  let signed_q15 = bitcast<i32>(word) >> 16u;
  return vec2<f32>(event_index,f32(signed_q15)/32767.0);
}

@compute @workgroup_size(64)
fn replay_sleep_learning(@builtin(global_invocation_id) gid:vec3<u32>) {
  let header = load_sleep_header(gid.y*20u);
  if (header.brain_slot_index >= arrayLength(&brain_slots)) { return; }
  let brain = brain_slots[header.brain_slot_index];
  let completion = header.completion_offset;
  if (!state_span_within(completion,16u)
      || load_state_u32(completion+3u) != SLEEP_STATUS_PREPARED) { return; }
  if (gid.x >= header.replay_span_count) { return; }
  let extension = load_slot_extension(brain);
  let learning = load_slot_learning_state(extension);
  let span_base = header.replay_span_offset + gid.x*4u;
  if (!sleep_frame_span_within(span_base,4u)) { sleep_reject(completion); return; }
  let span = load_sleep_replay_span(span_base);
  if (span.local_synapse_id >= header.synapse_count
      || span.reserved != 0u
      || span.sample_count != header.replay_event_count
      || span.sample_start > header.replay_sample_count
      || span.sample_count > header.replay_sample_count-span.sample_start) {
    sleep_reject(completion); return;
  }
  let metadata_base = extension.synapse_metadata_offset + span.local_synapse_id*8u;
  if (!sleep_immutable_plan_span_within(metadata_base,8u)) { sleep_reject(completion); return; }
  let metadata = load_synapse_learning_metadata(metadata_base);
  let receptor_base = extension.receptor_offset + metadata.receptor_index*8u;
  if (metadata.global_synapse_id != span.local_synapse_id
      || !sleep_immutable_plan_span_within(receptor_base,8u)) {
    sleep_reject(completion); return;
  }
  let receptor = load_plasticity_receptor(receptor_base);
  let alpha = bitcast<f32>(immutable_weight_words[brain.alpha_offset+span.local_synapse_id]);
  if (!sleep_finite(receptor.sleep_replay_rate)
      || !sleep_finite(receptor.modulator_sign)
      || !sleep_finite(receptor.fast_min)
      || !sleep_finite(receptor.fast_max)
      || receptor.sleep_replay_rate < 0.0 || receptor.sleep_replay_rate > 1.0
      || (receptor.modulator_sign != -1.0 && receptor.modulator_sign != 1.0)
      || receptor.fast_min >= receptor.fast_max || !sleep_finite(alpha)) {
    sleep_reject(completion); return;
  }
  var replay_credit = 0.0;
  for (var index=0u; index<span.sample_count; index+=1u) {
    let sample_word_index = header.replay_sample_offset + span.sample_start + index;
    if (!sleep_frame_span_within(sample_word_index,1u)) { sleep_reject(completion); return; }
    let unpacked = unpack_sleep_eligibility(frame_payload_words[sample_word_index]);
    let event_index = u32(unpacked.x);
    if (event_index >= header.replay_event_count) { sleep_reject(completion); return; }
    let event_base = header.replay_event_offset + event_index*24u;
    if (!sleep_frame_span_within(event_base,24u)) { sleep_reject(completion); return; }
    let event = load_sleep_replay_event(event_base);
    if (!sleep_finite(unpacked.y) || !sleep_finite(event.modulator_value)
        || abs(event.modulator_value) > 1.0) { sleep_reject(completion); return; }
    replay_credit += unpacked.y*event.modulator_value*receptor.modulator_sign;
  }
  let inactive = inactive_weight_bases(brain,extension,learning);
  let fast_index = inactive.fast+span.local_synapse_id;
  let previous = load_state_f32(fast_index);
  let next = clamp(previous+receptor.sleep_replay_rate*alpha*replay_credit,receptor.fast_min,receptor.fast_max);
  if (!sleep_finite(replay_credit) || !sleep_finite(previous) || !sleep_finite(next)) {
    sleep_reject(completion); return;
  }
  store_state_f32(fast_index,next);
}
