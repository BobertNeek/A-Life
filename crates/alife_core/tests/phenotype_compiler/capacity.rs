//! Test-only RED contracts for canonical production brain capacity and compiled budget receipts.

mod task3_capacity_budget_red_tests {
    use std::collections::BTreeSet;

    use alife_core::{
        BrainCapacityClass, BrainClassId, BrainScaleTier, CompiledBudgets,
        GlobalPhenotypeBudgetReceipt, LegacyBrainClassAdapter, RouteBudgetReceipt,
        CANDIDATE_FEATURE_COUNT,
    };
    use serde_json::{json, Value};

    #[derive(Debug, Clone, Copy)]
    struct ExpectedClassBudget {
        id: BrainClassId,
        neurons: u32,
        total_synapses: u32,
        recurrent_synapses: u32,
        action_decoder_synapses: u32,
        memory_decoder_synapses: u32,
        active_tiles: u32,
        replay_events: u32,
        replay_eligibility_samples: u32,
    }

    fn assert_exact_canonical_budget(capacity: BrainCapacityClass, expected: ExpectedClassBudget) {
        let execution = capacity.execution();

        assert_eq!(
            (
                capacity.id(),
                execution.max_neurons(),
                execution.max_total_synapses(),
                execution.max_recurrent_synapses(),
                execution.max_action_decoder_synapses(),
                execution.max_memory_decoder_synapses(),
                execution.max_active_tiles(),
                execution.max_replay_events(),
                execution.max_replay_eligibility_samples(),
            ),
            (
                expected.id,
                expected.neurons,
                expected.total_synapses,
                expected.recurrent_synapses,
                expected.action_decoder_synapses,
                expected.memory_decoder_synapses,
                expected.active_tiles,
                expected.replay_events,
                expected.replay_eligibility_samples,
            ),
            "wrong class-specific capacity tuple for {:?}",
            expected.id,
        );
        assert_eq!(
            expected
                .recurrent_synapses
                .checked_add(expected.action_decoder_synapses)
                .and_then(|sum| sum.checked_add(expected.memory_decoder_synapses)),
            Some(expected.total_synapses),
            "the canonical split must exactly partition total synapses",
        );

        assert_eq!(
            (
                execution.schema_version(),
                execution.gpu_layout_version(),
                execution.max_candidates(),
                execution.max_object_slots(),
                execution.max_memory_context_records(),
                execution.microstep_range(),
                execution.max_compact_readback_bytes(),
                execution.microtile_edge(),
                execution.supertile_edge(),
                execution.candidate_feature_count(),
                execution.max_decoder_input_lanes(),
            ),
            (1, 2, 32, 16, 16, (2, 4), 64, 16, 128, 24, 64),
            "wrong common logical ABI tuple for {:?}",
            expected.id,
        );
        assert_eq!(CANDIDATE_FEATURE_COUNT, 24);

        assert_eq!(
            (
                execution.required_limits_schema_version(),
                execution.required_feature_mask_words(),
                execution.required_feature_mask(),
                execution.required_max_buffer_size(),
                execution.required_max_storage_buffer_binding_size(),
                execution.required_max_bind_groups(),
                execution.required_max_bindings_per_bind_group(),
            ),
            (1, 1, 0, 268_435_456, 134_217_728, 4, 1_000),
            "wrong versioned feature/buffer/binding floors for {:?}",
            expected.id,
        );
        assert_eq!(
            (
                execution.required_max_storage_buffers_per_shader_stage(),
                execution.required_max_uniform_buffers_per_shader_stage(),
                execution.required_max_dynamic_storage_buffers_per_pipeline_layout(),
                execution.required_max_dynamic_uniform_buffers_per_pipeline_layout(),
                execution.required_max_compute_workgroup_storage_size(),
            ),
            (8, 12, 4, 8, 16_384),
            "wrong stage/dynamic/workgroup-storage floors for {:?}",
            expected.id,
        );
        assert_eq!(
            (
                execution.required_max_compute_workgroup_size_x(),
                execution.required_max_compute_workgroup_size_y(),
                execution.required_max_compute_workgroup_size_z(),
                execution.required_max_compute_invocations_per_workgroup(),
                execution.required_max_compute_workgroups_per_dimension(),
            ),
            (256, 256, 64, 256, 65_535),
            "wrong compute-dispatch floors for {:?}",
            expected.id,
        );
        assert_eq!(
            (
                execution.storage_offset_alignment_bytes(),
                execution.uniform_offset_alignment_bytes(),
                execution.copy_buffer_alignment_bytes(),
                execution.copy_bytes_per_row_alignment(),
            ),
            (256, 256, 4, 256),
            "wrong storage/uniform/copy alignments for {:?}",
            expected.id,
        );

        capacity.validate_contract().unwrap();
    }

    #[test]
    fn canonical_capacity_budgets_match_every_approved_tuple_limit_and_alignment() {
        for (capacity, expected) in [
            (
                BrainCapacityClass::n512(),
                ExpectedClassBudget {
                    id: BrainCapacityClass::N512_ID,
                    neurons: 512,
                    total_synapses: 8_192,
                    recurrent_synapses: 6_144,
                    action_decoder_synapses: 1_024,
                    memory_decoder_synapses: 1_024,
                    active_tiles: 64,
                    replay_events: 32,
                    replay_eligibility_samples: 2_048,
                },
            ),
            (
                BrainCapacityClass::n1024(),
                ExpectedClassBudget {
                    id: BrainCapacityClass::N1024_ID,
                    neurons: 1_024,
                    total_synapses: 16_384,
                    recurrent_synapses: 12_288,
                    action_decoder_synapses: 2_048,
                    memory_decoder_synapses: 2_048,
                    active_tiles: 128,
                    replay_events: 64,
                    replay_eligibility_samples: 4_096,
                },
            ),
            (
                BrainCapacityClass::n2048(),
                ExpectedClassBudget {
                    id: BrainCapacityClass::N2048_ID,
                    neurons: 2_048,
                    total_synapses: 32_768,
                    recurrent_synapses: 24_576,
                    action_decoder_synapses: 4_096,
                    memory_decoder_synapses: 4_096,
                    active_tiles: 192,
                    replay_events: 128,
                    replay_eligibility_samples: 8_192,
                },
            ),
        ] {
            assert_exact_canonical_budget(capacity, expected);
        }
    }

    #[test]
    fn production_capacity_registry_contains_only_the_three_promoted_classes() {
        let promoted = BrainCapacityClass::production_classes();
        let promoted_ids = promoted.map(|capacity| capacity.id());
        assert_eq!(
            promoted_ids,
            [
                BrainCapacityClass::N512_ID,
                BrainCapacityClass::N1024_ID,
                BrainCapacityClass::N2048_ID,
            ],
        );
        assert_eq!(
            BrainCapacityClass::production_for_id(BrainCapacityClass::N512_ID),
            Ok(BrainCapacityClass::n512()),
        );
        assert_eq!(
            BrainCapacityClass::production_for_id(BrainCapacityClass::N1024_ID),
            Ok(BrainCapacityClass::n1024()),
        );
        assert_eq!(
            BrainCapacityClass::production_for_id(BrainCapacityClass::N2048_ID),
            Ok(BrainCapacityClass::n2048()),
        );

        let foreign_ids = [
            BrainClassId(0),
            BrainClassId(4),
            BrainClassId(u16::MAX),
            LegacyBrainClassAdapter::capacity_id_for_tier(BrainScaleTier::Large4096),
            LegacyBrainClassAdapter::capacity_id_for_tier(BrainScaleTier::Cognitive32768),
            LegacyBrainClassAdapter::capacity_id_for_tier(BrainScaleTier::Student131k),
            LegacyBrainClassAdapter::capacity_id_for_tier(BrainScaleTier::Ascended1M),
            LegacyBrainClassAdapter::capacity_id_for_tier(BrainScaleTier::Ascended5M),
            LegacyBrainClassAdapter::capacity_id_for_tier(BrainScaleTier::ResearchCustom),
        ];
        for foreign_id in foreign_ids {
            assert!(
                !promoted_ids.contains(&foreign_id),
                "test fixture accidentally treated {foreign_id:?} as promoted",
            );
            assert!(
                BrainCapacityClass::production_for_id(foreign_id).is_err(),
                "foreign capacity {foreign_id:?} entered production",
            );
        }

        let digests = promoted.map(|capacity| capacity.canonical_digest());
        assert_ne!(digests[0], digests[1]);
        assert_ne!(digests[0], digests[2]);
        assert_ne!(digests[1], digests[2]);
    }

    fn forged_execution_field_cases() -> Vec<(&'static str, Value)> {
        vec![
            ("schema_version", json!(2)),
            ("gpu_layout_version", json!(3)),
            ("max_neurons", json!(640)),
            ("max_total_synapses", json!(8_193)),
            ("max_recurrent_synapses", json!(6_145)),
            ("max_action_decoder_synapses", json!(1_025)),
            ("max_memory_decoder_synapses", json!(1_025)),
            ("max_active_tiles", json!(65)),
            ("max_candidates", json!(31)),
            ("max_object_slots", json!(15)),
            ("max_memory_context_records", json!(15)),
            ("min_microsteps", json!(1)),
            ("max_microsteps", json!(5)),
            ("max_replay_events", json!(31)),
            ("max_replay_eligibility_samples", json!(2_047)),
            ("max_compact_readback_bytes", json!(128)),
            ("microtile_edge", json!(32)),
            ("supertile_edge", json!(256)),
            ("candidate_feature_count", json!(23)),
            ("max_decoder_input_lanes", json!(63)),
            ("required_limits_schema_version", json!(2)),
            ("required_feature_mask_words", json!(2)),
            ("required_feature_mask", json!(1)),
            ("required_max_buffer_size", json!(536_870_912u64)),
            (
                "required_max_storage_buffer_binding_size",
                json!(268_435_456u64),
            ),
            ("required_max_bind_groups", json!(5)),
            ("required_max_bindings_per_bind_group", json!(1_001)),
            ("required_max_storage_buffers_per_shader_stage", json!(9)),
            ("required_max_uniform_buffers_per_shader_stage", json!(13)),
            (
                "required_max_dynamic_storage_buffers_per_pipeline_layout",
                json!(5),
            ),
            (
                "required_max_dynamic_uniform_buffers_per_pipeline_layout",
                json!(9),
            ),
            ("required_max_compute_workgroup_storage_size", json!(32_768)),
            ("required_max_compute_workgroup_size_x", json!(128)),
            ("required_max_compute_workgroup_size_y", json!(128)),
            ("required_max_compute_workgroup_size_z", json!(32)),
            ("required_max_compute_invocations_per_workgroup", json!(128)),
            (
                "required_max_compute_workgroups_per_dimension",
                json!(65_534),
            ),
            ("storage_offset_alignment_bytes", json!(512)),
            ("uniform_offset_alignment_bytes", json!(512)),
            ("copy_buffer_alignment_bytes", json!(8)),
            ("copy_bytes_per_row_alignment", json!(512)),
        ]
    }

    #[test]
    fn brain_capacity_wire_rejects_every_individually_forged_execution_field() {
        let canonical_wire = serde_json::to_value(BrainCapacityClass::n512()).unwrap();
        let serialized_execution_fields = canonical_wire["execution"]
            .as_object()
            .expect("canonical capacity execution must serialize as an object")
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        let cases = forged_execution_field_cases();
        let mutated_fields = cases
            .iter()
            .map(|(field, _)| (*field).to_owned())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            mutated_fields, serialized_execution_fields,
            "the negative table must mutate every serialized execution field exactly once",
        );
        assert_eq!(
            cases.len(),
            mutated_fields.len(),
            "the negative table must not contain duplicate field rows",
        );

        for (field, replacement) in cases {
            let mut forged_wire = canonical_wire.clone();
            let execution = forged_wire["execution"]
                .as_object_mut()
                .expect("canonical capacity execution must serialize as an object");
            let original = execution
                .insert(field.to_owned(), replacement.clone())
                .unwrap_or_else(|| panic!("missing serialized execution field {field}"));
            assert_ne!(
                original, replacement,
                "negative row for {field} did not change the serialized value",
            );

            let result = serde_json::from_value::<BrainCapacityClass>(forged_wire);
            assert!(
                result.is_err(),
                "custom deserialization accepted forged execution field {field}: {result:?}",
            );
        }
    }

    fn valid_n512_compiled_budgets() -> (BrainCapacityClass, CompiledBudgets) {
        let capacity = BrainCapacityClass::n512();
        let budgets = CompiledBudgets {
            capacity_class_id: capacity.id(),
            execution_abi_digest: capacity.canonical_digest(),
            routes: vec![
                RouteBudgetReceipt {
                    route_index: 0,
                    active_tiles: 1,
                    recurrent_synapses: 120,
                    action_decoder_synapses: 0,
                    memory_decoder_synapses: 0,
                    immutable_payload_words: 120,
                    tile_ceiling: 1,
                    synapse_ceiling: 120,
                    payload_word_ceiling: 120,
                },
                RouteBudgetReceipt {
                    route_index: 1,
                    active_tiles: 0,
                    recurrent_synapses: 0,
                    action_decoder_synapses: 8,
                    memory_decoder_synapses: 0,
                    immutable_payload_words: 8,
                    tile_ceiling: 0,
                    synapse_ceiling: 8,
                    payload_word_ceiling: 8,
                },
            ],
            global: GlobalPhenotypeBudgetReceipt {
                neuron_count: 512,
                active_tiles: 1,
                recurrent_synapses: 120,
                action_decoder_synapses: 8,
                memory_decoder_synapses: 0,
                total_synapses: 128,
                immutable_payload_words: 128,
                candidate_capacity: 32,
                object_slot_capacity: 16,
                memory_context_capacity: 16,
                decoder_input_lanes: 24,
                replay_event_capacity: 32,
                replay_eligibility_sample_capacity: 2_048,
            },
        };
        budgets
            .validate_against(&capacity)
            .expect("the baseline receipt must be valid before a one-field mutation");
        (capacity, budgets)
    }

    fn assert_budget_rejected(case: &str, capacity: &BrainCapacityClass, forged: &CompiledBudgets) {
        let result = forged.validate_against(capacity);
        assert!(
            result.is_err(),
            "CompiledBudgets accepted {case}: {forged:#?}",
        );
    }

    fn checked_route_sum(
        routes: &[RouteBudgetReceipt],
        value: impl Fn(&RouteBudgetReceipt) -> u32,
    ) -> u32 {
        routes.iter().fold(0u32, |sum, route| {
            sum.checked_add(value(route))
                .expect("test fixture route sum overflowed")
        })
    }

    fn refresh_global_route_sums(budgets: &mut CompiledBudgets) {
        budgets.global.active_tiles = checked_route_sum(&budgets.routes, |r| r.active_tiles);
        budgets.global.recurrent_synapses =
            checked_route_sum(&budgets.routes, |r| r.recurrent_synapses);
        budgets.global.action_decoder_synapses =
            checked_route_sum(&budgets.routes, |r| r.action_decoder_synapses);
        budgets.global.memory_decoder_synapses =
            checked_route_sum(&budgets.routes, |r| r.memory_decoder_synapses);
        budgets.global.immutable_payload_words =
            checked_route_sum(&budgets.routes, |r| r.immutable_payload_words);
        budgets.global.total_synapses = budgets
            .global
            .recurrent_synapses
            .checked_add(budgets.global.action_decoder_synapses)
            .and_then(|sum| sum.checked_add(budgets.global.memory_decoder_synapses))
            .expect("test fixture global synapse sum overflowed");
    }

    #[test]
    fn compiled_budgets_reject_route_overlap_omission_gaps_and_unsorted_ids() {
        let (capacity, baseline) = valid_n512_compiled_budgets();

        let mut overlap = baseline.clone();
        overlap.routes[1].route_index = overlap.routes[0].route_index;
        assert_budget_rejected("overlapping duplicate route IDs", &capacity, &overlap);

        let mut omitted = baseline.clone();
        omitted.routes.remove(0);
        omitted.routes[0].route_index = 0;
        assert_budget_rejected(
            "a route omitted from the global receipt",
            &capacity,
            &omitted,
        );

        let mut gap = baseline.clone();
        gap.routes[1].route_index = 2;
        assert_budget_rejected("a noncontiguous route-ID gap", &capacity, &gap);

        let mut unsorted = baseline.clone();
        unsorted.routes.swap(0, 1);
        assert_budget_rejected("contiguous but unsorted route IDs", &capacity, &unsorted);
    }

    #[test]
    fn compiled_budgets_reject_global_category_overlap_and_unclassified_omission() {
        let (capacity, baseline) = valid_n512_compiled_budgets();

        let mut overlap = baseline.clone();
        overlap.global.total_synapses -= 1;
        assert_budget_rejected(
            "a global total smaller than its disjoint category sum",
            &capacity,
            &overlap,
        );

        let mut omission = baseline.clone();
        omission.global.total_synapses += 1;
        assert_budget_rejected(
            "an unclassified synapse omitted from the global categories",
            &capacity,
            &omission,
        );
    }

    #[test]
    fn compiled_budgets_reject_decoder_double_count_even_when_global_sums_match() {
        let (capacity, baseline) = valid_n512_compiled_budgets();
        let mut forged = baseline.clone();

        let duplicated_decoder_count = forged.routes[1].action_decoder_synapses;
        forged.routes[0].action_decoder_synapses = duplicated_decoder_count;
        forged.routes[0].immutable_payload_words += duplicated_decoder_count;
        forged.routes[0].synapse_ceiling += duplicated_decoder_count;
        forged.routes[0].payload_word_ceiling += duplicated_decoder_count;
        refresh_global_route_sums(&mut forged);

        assert_eq!(forged.global.action_decoder_synapses, 16);
        assert_eq!(forged.global.total_synapses, 136);
        assert_budget_rejected(
            "action-decoder synapses counted in both a recurrent route and the reserved decoder route",
            &capacity,
            &forged,
        );
    }

    #[test]
    fn compiled_budgets_use_checked_route_and_global_sums() {
        let (capacity, baseline) = valid_n512_compiled_budgets();

        for counter in [
            "active_tiles",
            "recurrent_synapses",
            "action_decoder_synapses",
            "memory_decoder_synapses",
            "immutable_payload_words",
        ] {
            let mut forged = baseline.clone();
            match counter {
                "active_tiles" => forged.global.active_tiles += 1,
                "recurrent_synapses" => forged.global.recurrent_synapses += 1,
                "action_decoder_synapses" => forged.global.action_decoder_synapses += 1,
                "memory_decoder_synapses" => forged.global.memory_decoder_synapses += 1,
                "immutable_payload_words" => forged.global.immutable_payload_words += 1,
                _ => unreachable!(),
            }
            assert_budget_rejected(
                &format!("route-to-global checked-sum mismatch in {counter}"),
                &capacity,
                &forged,
            );
        }

        let mut route_accumulator_overflow = baseline.clone();
        route_accumulator_overflow.routes[0].recurrent_synapses = u32::MAX;
        route_accumulator_overflow.routes[0].immutable_payload_words = u32::MAX;
        route_accumulator_overflow.routes[0].synapse_ceiling = u32::MAX;
        route_accumulator_overflow.routes[0].payload_word_ceiling = u32::MAX;
        route_accumulator_overflow.global.recurrent_synapses = u32::MAX;
        route_accumulator_overflow.global.total_synapses = u32::MAX;
        route_accumulator_overflow.global.immutable_payload_words = u32::MAX;
        assert_budget_rejected(
            "u32 overflow while accumulating route/category counts",
            &capacity,
            &route_accumulator_overflow,
        );

        let mut global_category_overflow = baseline.clone();
        global_category_overflow.global.recurrent_synapses = u32::MAX;
        global_category_overflow.global.action_decoder_synapses = 1;
        global_category_overflow.global.memory_decoder_synapses = 0;
        global_category_overflow.global.total_synapses = u32::MAX;
        assert_budget_rejected(
            "u32 overflow while adding global synapse categories",
            &capacity,
            &global_category_overflow,
        );
    }

    #[test]
    fn compiled_budgets_reject_capacity_id_and_foreign_execution_digest() {
        let (capacity, baseline) = valid_n512_compiled_budgets();

        let mut wrong_class = baseline.clone();
        wrong_class.capacity_class_id = BrainCapacityClass::N1024_ID;
        assert_budget_rejected(
            "a receipt carrying another capacity class ID",
            &capacity,
            &wrong_class,
        );

        let mut foreign_digest = baseline.clone();
        foreign_digest.execution_abi_digest = BrainCapacityClass::n1024().canonical_digest();
        assert_ne!(
            foreign_digest.execution_abi_digest,
            capacity.canonical_digest()
        );
        assert_budget_rejected(
            "a receipt carrying another capacity class ABI digest",
            &capacity,
            &foreign_digest,
        );
    }
}
