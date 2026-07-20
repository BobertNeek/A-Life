const ACTIVE_DISPATCH_ROW_WORDS:u32 = 332u;

@compute @workgroup_size(1)
fn clear_diagnostics(@builtin(global_invocation_id) gid:vec3<u32>) {
  let header = load_perception_header(gid.y * ACTIVE_DISPATCH_ROW_WORDS);
  if (!validate_slice_a_slot(header.brain_slot_index, header)) { return; }
  let brain = brain_slots[header.brain_slot_index];
  atomicStore(&mutable_state_words[brain.diagnostic_offset], 0u);
  atomicStore(&mutable_state_words[brain.diagnostic_offset + 1u], 0u);
  atomicStore(&mutable_state_words[brain.diagnostic_offset + 2u], 0u);
  atomicStore(&mutable_state_words[brain.diagnostic_offset + 3u], header.active_activation_side);
}
