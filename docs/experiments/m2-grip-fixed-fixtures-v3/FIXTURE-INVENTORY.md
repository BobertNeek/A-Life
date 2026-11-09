# V3 continuation inventory

28 remaining fixtures execute exactly once;18 V2 passes are retained separately.

## alife_world / lib alife_world

Features: defaults only

- f019: headless::grip_options::tests::close_release_are_independent_idempotent_intents
- f020: headless::grip_options::tests::offscreen_hold_has_own_identity_without_a_visual_slot
- f021: headless::grip_options::tests::old_source_keeps_arbitrary_grip_ids_as_legacy_toggle
- f022: headless::grip_options::tests::failed_swap_retains_previous_constraint
- f023: headless::grip_options::tests::other_held_and_consumed_targets_cannot_be_acquired_or_released
- f024: headless::grip_options::tests::ordinary_move_can_make_a_far_close_reachable
- f025: headless::grip_options::tests::legacy_release_number_roundtrips_and_remains_old_toggle
- f026: headless::grip_options::tests::forged_valid_grip_draft_cannot_upgrade_old_source_admission
- f027: headless::grip_options::tests::empty_release_is_idempotent_but_stale_empty_cannot_open_a_new_hold

## alife_training / lib alife_training

Features: cpu-backend

- f028: cpu::grip_ports_v3::tests::unknown_relation_never_becomes_free_or_self_held
- f029: cpu::grip_ports_v3::tests::distinct_intents_remain_admissible_when_held
- f030: cpu::grip_ports_v3::tests::stale_wrong_target_or_operation_recall_is_rejected
- f031: cpu::grip_ports_v3::tests::unrelated_other_has_exact_zero_output_and_credit
- f032: cpu::grip_ports_v3::tests::cold_restore_preserves_nonzero_state_and_next_output
- f033: cpu::grip_ports_v3::tests::exact_memory_derivative_matches_finite_difference
- f034: cpu::grip_ports_v3::tests::unknown_held_release_does_not_displace_no_manip_maintenance
- f035: cpu::grip_ports_v3::tests::empty_release_is_body_effort_without_object_recall
- f036: cpu::grip_integration::maintenance_tests::release_native_base_is_maintenance_not_other_value
- f037: cpu::motor_mediation::confidence::policy::grip_options::coordination::tests::fixed_near_far_move_then_reach_and_noop
- f038: cpu::motor_mediation::confidence::policy::grip_options::coordination::tests::no_kind_or_mobility_or_held_semantic_oracle
- f039: cpu::motor_mediation::grip_maintenance_score_tests::matched_native_inputs_preserve_maintenance_before_grip_effort
- f040: cpu::motor_mediation::grip_maintenance_score_tests::warm_unequal_slots_use_exact_authoritative_maintenance_output

## alife_training / test grip_options_host

Features: cpu-backend

- f041: grounded_scene_pair_scores_and_cold_continuation

## alife_gpu_backend / test closed_loop_wgsl

Features: training-rollout

- f042: closed_loop_wgsl_parses_validates_and_exposes_only_the_required_entries
- f043: naga_reflection_matches_the_exact_seven_heap_bindings_and_access_modes

## alife_gpu_backend / test closed_loop_buffer_contracts

Features: training-rollout

- f044: perception::perception_upload_translates_the_validated_same_tick_frame_without_scores
- f045: perception::dynamic_upload_validation_rejects_activation_side_offsets_counts_and_reserved_words

## alife_gpu_backend / test grip_profile_upload_gate

Features: training-rollout

- f046: valid_grip_profile_is_rejected_while_matching_legacy_upload_succeeds

## Retained V2 passes, not executed in V3

- f001: legacy_profile_tags_and_ambiguous_rejection_are_literal
- f002: own_target_has_no_fabricated_visual_features
- f003: exact_grip_memory_operations_do_not_alias_contact_or_other
- f004: legacy_full_objects_and_sequences_keep_exact_eleven_fields
- f005: legacy_numeric_maps_decode_full_fields_without_extension
- f006: legacy_nested_bincode_does_not_consume_the_next_draft_or_tail
- f007: new_empty_and_nonempty_frames_roundtrip_all_transports
- f008: new_extension_is_required_after_digest_even_without_visual_slots
- f009: legacy_metadata_does_not_gain_authority_and_empty_grip_cannot_claim_a_held_target
- f010: legacy_named_own_grip_metadata_remains_ignored_before_and_after_profile
- f011: new_named_own_grip_metadata_is_typed_only_after_profile_and_duplicates_fail
- f012: legacy_non_string_nested_metadata_and_duplicate_numeric_unknowns_are_ignored
- f013: grip_sensory::tests::occluded_own_hold_retains_proprioception_without_visual_slots
- f014: grip_sensory::tests::contradictory_known_target_relation_is_rejected
- f015: grip_sensory::tests::other_holder_is_never_an_exposed_relation
- f016: grip_sensory::tests::sensory_extension_roundtrips_and_checks_its_version
- f017: memory::candidate_recall::grip_scope_tests::grip_family_recall_requires_exact_target_and_operation
- f018: memory::candidate_recall::grip_scope_tests::unknown_release_target_stays_zero

No broad test filters or old-life input. V3 is not one46/46 batch.
