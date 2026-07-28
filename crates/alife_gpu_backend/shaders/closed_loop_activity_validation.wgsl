// Production GPU dispatch-contract validation shared by the one-thread row
// prepass and the parallel neural passes. The expensive schedule digest is
// computed only by `clear_diagnostics`; parallel invocations consume its
// fail-stop diagnostic bit.

const ACTIVITY_HEADER_OFFSET:u32 = 308u;
const CONTRACT_INVALID_DIAGNOSTIC_BIT:u32 = 0x80000000u;

struct GpuActivityDispatchHeader {
  schema_version:u32, policy_version:u32, class_id:u32, slot:u32,
  slot_generation:u32, brain_slot_index:u32, microsteps:u32, enabled_route_count:u32,
  enabled_route_mask:array<u32,8>, route_schedule_digest:array<u32,8>,
}

fn load_activity_header(base:u32) -> GpuActivityDispatchHeader {
  return GpuActivityDispatchHeader(
    dispatch_header_words[base], dispatch_header_words[base+1u], dispatch_header_words[base+2u], dispatch_header_words[base+3u],
    dispatch_header_words[base+4u], dispatch_header_words[base+5u], dispatch_header_words[base+6u], dispatch_header_words[base+7u],
    array<u32,8>(
      dispatch_header_words[base+8u], dispatch_header_words[base+9u], dispatch_header_words[base+10u], dispatch_header_words[base+11u],
      dispatch_header_words[base+12u], dispatch_header_words[base+13u], dispatch_header_words[base+14u], dispatch_header_words[base+15u]
    ),
    array<u32,8>(
      dispatch_header_words[base+16u], dispatch_header_words[base+17u], dispatch_header_words[base+18u], dispatch_header_words[base+19u],
      dispatch_header_words[base+20u], dispatch_header_words[base+21u], dispatch_header_words[base+22u], dispatch_header_words[base+23u]
    )
  );
}

fn route_enabled(activity:GpuActivityDispatchHeader, route_index:u32) -> bool {
  if (route_index >= 256u) { return false; }
  return (activity.enabled_route_mask[route_index / 32u] & (1u << (route_index % 32u))) != 0u;
}

fn route_enabled_at(mask_base:u32, route_index:u32) -> bool {
  if (route_index >= 256u) { return false; }
  let mask = dispatch_header_words[mask_base + route_index / 32u];
  return (mask & (1u << (route_index % 32u))) != 0u;
}

fn mix_route_digest_word(input:array<u32,8>, word:u32) -> array<u32,8> {
  let salts = array<u32,8>(
    0x00000000u, 0x85ebca6bu, 0xc2b2ae35u, 0x27d4eb2fu,
    0x165667b1u, 0xd3a2646cu, 0xfd7046c5u, 0xb55a4f09u
  );
  var output = input;
  for (var lane = 0u; lane < 8u; lane++) {
    output[lane] = ((output[lane] + salts[lane]) ^ word) * 16777619u;
  }
  return output;
}

fn compute_route_schedule_digest(activity:GpuActivityDispatchHeader) -> array<u32,8> {
  var digest = array<u32,8>(
    0x811c9dc5u, 0x9e3779b9u, 0x243f6a88u, 0xb7e15163u,
    0xa4093822u, 0x299f31d0u, 0x082efa98u, 0xec4e6c89u
  );
  let phenotype = phenotype_identities[activity.brain_slot_index];
  for (var word = 0u; word < 8u; word++) {
    digest = mix_route_digest_word(digest, phenotype.phenotype_hash[word]);
  }
  digest = mix_route_digest_word(digest, activity.microsteps);
  digest = mix_route_digest_word(digest, activity.enabled_route_count);
  for (var route_index = 0u; route_index < 256u; route_index++) {
    if (route_enabled(activity, route_index)) {
      digest = mix_route_digest_word(digest, route_index);
    }
  }
  return digest;
}

fn validate_activity_header(activity:GpuActivityDispatchHeader, header:GpuPerceptionHeader) -> bool {
  var enabled_count = 0u;
  for (var word = 0u; word < 8u; word++) {
    enabled_count += countOneBits(activity.enabled_route_mask[word]);
  }
  let expected_digest = compute_route_schedule_digest(activity);
  var digest_matches = true;
  for (var word = 0u; word < 8u; word++) {
    digest_matches = digest_matches && activity.route_schedule_digest[word] == expected_digest[word];
  }
  return activity.schema_version != 0u
    && activity.policy_version != 0u
    && activity.class_id == header.class_id
    && activity.slot == header.slot
    && activity.slot_generation == header.slot_generation
    && activity.brain_slot_index == header.brain_slot_index
    && activity.microsteps == header.microstep_count
    && activity.enabled_route_count == enabled_count
    && enabled_count != 0u
    && digest_matches;
}

fn compute_scheduled_work_checksum(
  activity:GpuActivityDispatchHeader,
  scheduled_tile_visits:u32,
  scheduled_synapse_ops:u32,
) -> u32 {
  var checksum = 0x811c9dc5u;
  for (var word = 0u; word < 8u; word++) {
    checksum = (checksum ^ activity.route_schedule_digest[word]) * 16777619u;
  }
  checksum = (checksum ^ scheduled_tile_visits) * 16777619u;
  checksum = (checksum ^ scheduled_synapse_ops) * 16777619u;
  return checksum;
}

fn validate_scheduled_work(
  learning:GpuLearningHeader,
  activity:GpuActivityDispatchHeader,
) -> bool {
  return learning.scheduled_tile_visits != 0u
    && learning.scheduled_synapse_ops != 0u
    && learning.scheduled_work_checksum == compute_scheduled_work_checksum(
      activity,
      learning.scheduled_tile_visits,
      learning.scheduled_synapse_ops
    );
}

fn activity_contract_prevalidated(header:GpuPerceptionHeader) -> bool {
  if (header.brain_slot_index >= arrayLength(&brain_slots)) { return false; }
  let brain = brain_slots[header.brain_slot_index];
  return (atomicLoad(&mutable_state_words[brain.diagnostic_offset + 2u])
      & CONTRACT_INVALID_DIAGNOSTIC_BIT) == 0u;
}
