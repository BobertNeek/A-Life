const ACTIVE_DISPATCH_ROW_WORDS:u32 = 332u;
const LEARNING_HEADER_WORD_OFFSET:u32 = 272u;

@compute @workgroup_size(1)
fn clear_diagnostics(@builtin(global_invocation_id) gid:vec3<u32>) {
  let header = load_perception_header(gid.y * ACTIVE_DISPATCH_ROW_WORDS);
  if (header.brain_slot_index >= arrayLength(&brain_slots)) { return; }
  let brain = brain_slots[header.brain_slot_index];
  atomicStore(&mutable_state_words[brain.diagnostic_offset], 0u);
  atomicStore(&mutable_state_words[brain.diagnostic_offset + 1u], 0u);
  atomicStore(&mutable_state_words[brain.diagnostic_offset + 2u], CONTRACT_INVALID_DIAGNOSTIC_BIT);
  atomicStore(&mutable_state_words[brain.diagnostic_offset + 3u], header.active_activation_side);
  if (!validate_slice_a_slot(header.brain_slot_index, header)) { return; }
  let activity = load_activity_header(gid.y * ACTIVE_DISPATCH_ROW_WORDS + ACTIVITY_HEADER_OFFSET);
  let learning = load_learning_header(
    gid.y * ACTIVE_DISPATCH_ROW_WORDS + LEARNING_HEADER_WORD_OFFSET
  );
  if (validate_activity_header(activity, header)
      && validate_scheduled_work(learning, activity)) {
    atomicStore(
      &mutable_state_words[brain.diagnostic_offset],
      learning.scheduled_tile_visits
    );
    atomicStore(
      &mutable_state_words[brain.diagnostic_offset + 1u],
      learning.scheduled_synapse_ops
    );
    atomicStore(&mutable_state_words[brain.diagnostic_offset + 2u], 0u);
  }
}
