mod support;

use std::fs;

use alife_core::ScaffoldContractError;
use alife_gpu_backend::{GpuBrainHandle, GpuClosedLoopBackend, GpuClosedLoopRuntimeConfig};

fn runtime_source() -> String {
    fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/closed_loop_runtime.rs"
    ))
    .expect("Task 7 must provide the required-GPU runtime module")
}

fn without_rust_comments(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut result = String::with_capacity(source.len());
    let mut cursor = 0;
    let mut block_depth = 0_u32;
    while cursor < bytes.len() {
        if block_depth == 0 && bytes[cursor..].starts_with(b"//") {
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                cursor += 1;
            }
        } else if bytes[cursor..].starts_with(b"/*") {
            block_depth += 1;
            cursor += 2;
        } else if block_depth > 0 && bytes[cursor..].starts_with(b"*/") {
            block_depth -= 1;
            cursor += 2;
        } else {
            if block_depth == 0 {
                result.push(bytes[cursor] as char);
            }
            cursor += 1;
        }
    }
    assert_eq!(block_depth, 0, "unterminated block comment");
    result
}

#[test]
fn required_gpu_api_is_public_without_constructing_a_device() {
    let _factory: fn() -> Result<GpuClosedLoopBackend, ScaffoldContractError> =
        GpuClosedLoopBackend::new_required;
    let _opaque_capability = std::mem::size_of::<GpuBrainHandle>();
    let config = GpuClosedLoopRuntimeConfig::default();
    assert_eq!(config.n512_slots, 64);
    assert_eq!(config.n1024_slots, 16);
    assert_eq!(config.n2048_slots, 4);
    assert_eq!(config.aggregate_resident_ceiling_bytes, 128 * 1024 * 1024);
}

#[test]
fn product_runtime_has_no_cpu_execution_or_fallback_boundary() {
    let source = runtime_source();
    for forbidden in [
        concat!("Cpu", "Reference"),
        concat!("cpu_", "shadow"),
        concat!("AutoWithCpu", "Fallback"),
        concat!("FullGpuRuntime", "Mode"),
        concat!("NeuralCompute", "Backend"),
    ] {
        assert!(
            !source.contains(forbidden),
            "forbidden runtime token: {forbidden}"
        );
    }
}

#[test]
fn runtime_structurally_includes_the_real_crate_private_unit_test_module() {
    let runtime = without_rust_comments(&runtime_source());
    assert!(runtime.contains("#[cfg(test)]"));
    assert!(runtime.contains("#[path = \"../tests/support/closed_loop_runtime_private.rs\"]"));
    assert!(runtime.contains("mod task7_private_tests;"));

    let private_tests = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/closed_loop_runtime_private.rs"
    ))
    .unwrap();
    let private_tests = without_rust_comments(&private_tests);
    for required_test in [
        "fn unavailable_gpu_returns_typed_error_instead_of_cpu_fallback()",
        "fn software_adapter_is_rejected_without_device_request()",
        "fn stale_gpu_layout_is_rejected_before_slot_allocation()",
        "fn hardware_receipt_digests_are_canonical_complete_deterministic_and_sensitive()",
        "fn backend_and_receipt_allocators_are_independent_checked_and_nonzero()",
        "fn removal_scrubs_every_reserved_range_before_slot_reuse()",
        "fn maximum_slot_generation_retires_permanently_instead_of_wrapping()",
        "fn failed_scrub_marks_device_lost_and_never_frees_or_reuses_the_slot()",
        "fn save_rebind_requires_explicit_matching_organism_ownership()",
        "fn unsupported_n32k_class_rejects_before_arena_allocation()",
        "fn tampered_frame_digest_rejects_before_upload_or_counter_mutation()",
    ] {
        let position = private_tests.find(required_test).unwrap();
        assert!(private_tests[..position].ends_with("#[test]\n"));
    }
    assert!(
        private_tests.contains("GpuClosedLoopBackend::new_with_factory(&UnavailableGpuFactory)")
    );
    assert!(private_tests.contains("factory.device_request_count()"));
    assert!(private_tests.contains("validate_required_gpu_layout_version("));
    let compact_private_tests = private_tests
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    for required_behavior_call in [
        "canonical_limit_words_for_test(&limits)",
        "CanonicalDigestBuilder::new(b\"alife.gpu.hardware.driver.v1\")",
        "CanonicalDigestBuilder::new(b\"alife.gpu.hardware.features.v1\")",
        "CanonicalDigestBuilder::new(b\"alife.gpu.hardware.limits.v1\")",
        "expected_driver.write_sequence_len(2)",
        "expected_driver.write_utf8(\"driver\")",
        "expected_features.write_sequence_len(4)",
        "expected_limits.write_sequence_len(expected_limit_words.len())",
        "canonical_driver_digest(\"driver\",\"info\")",
        "canonical_feature_digest(requested,enabled)",
        "with_runtime_allocation_state_for_test(41,91,||",
        "next_backend_instance_id()",
        "next_hardware_receipt_generation()",
        "with_runtime_allocation_state_for_test(u64::MAX,u64::MAX,||",
        "fill_every_reserved_range(first,0xa5a5_a5a5)",
        "insert_fixture_with_generation(OrganismId(1),PhenotypeHash([1;4]),u32::MAX)",
        "fail_next_scrub_after_submit()",
        "rebind_fixture_for_restore(OrganismId(7),PhenotypeHash([7;4]))",
        "validate_class(BrainClassId(5))",
        "validate_frame_digest(expected,tampered)",
    ] {
        assert!(
            compact_private_tests.contains(required_behavior_call),
            "missing executable private behavior call: {required_behavior_call}"
        );
    }
}

#[test]
fn brain_handle_source_keeps_capability_fields_private_and_nonserializable() {
    let source = runtime_source();
    let start = source
        .find("pub struct GpuBrainHandle")
        .expect("runtime must define the opaque handle");
    let body = &source[start..];
    let end = body.find('}').expect("handle must have a finite body");
    let body = &body[..end];
    for field in [
        "backend_instance_id",
        "class_id",
        "slot",
        "generation",
        "organism_id",
        "phenotype_hash",
    ] {
        assert!(body.contains(field), "missing handle binding: {field}");
        assert!(
            !body.contains(&format!("pub {field}")),
            "handle capability field must remain private: {field}"
        );
    }
    let derive_prefix = &source[start.saturating_sub(256)..start];
    assert!(!derive_prefix.contains("Serialize"));
    assert!(!derive_prefix.contains("Deserialize"));
}

#[cfg(feature = "gpu-tests")]
mod hardware {
    use alife_core::{
        BrainCapacityClass, OrganismId, PerceptionFrame, ScaffoldContractError, SensorProfile,
    };
    use alife_gpu_backend::{
        GpuBackendState, GpuBrainHandle, GpuClosedLoopBackend, GpuClosedLoopRuntimeConfig,
        GpuClosedLoopTick, GPU_CLOSED_LOOP_LAYOUT_VERSION,
    };

    use super::support::{
        expected_cadence_counts, heterogeneous_n512_phenotypes, n512_phenotype,
        n512_phenotype_for_profile_at_maturation, perception_frame_for_profile_at_tick,
        phenotype_for_capacity_at_maturation,
    };

    fn small_config() -> GpuClosedLoopRuntimeConfig {
        GpuClosedLoopRuntimeConfig {
            n512_slots: 4,
            n1024_slots: 2,
            n2048_slots: 2,
            aggregate_resident_ceiling_bytes: 128 * 1024 * 1024,
        }
    }

    fn required_backend() -> GpuClosedLoopBackend {
        GpuClosedLoopBackend::new_required_with_config(small_config())
            .expect("local required GPU backend")
    }

    fn assert_tick_identity(
        tick: &GpuClosedLoopTick,
        handle: GpuBrainHandle,
        frame: &alife_core::PerceptionFrame,
        receipt_generation: u64,
    ) {
        assert_eq!(tick.handle, handle);
        assert_eq!(tick.base_digest, frame.base_digest());
        assert_eq!(tick.frame_digest, frame.frame_digest());
        assert_eq!(tick.compact_readback_bytes, 48);
        assert_eq!(tick.hardware_receipt_generation, receipt_generation);
        assert_ne!(tick.dispatch_generation, 0);
        assert!(tick.active_activation_side <= 1);
        let candidate = &frame.candidates()[usize::from(tick.selection.candidate_index)];
        assert_eq!(tick.selection.confidence, candidate.sensor_confidence);
        assert!(tick.selection.logit.is_finite());
        assert!(tick.selection.active_tiles > 0);
        assert!(tick.selection.active_synapses > 0);
    }

    #[test]
    fn hardware_receipt_is_one_bounded_required_vulkan_contract() {
        let backend = required_backend();
        let receipt = backend.hardware_receipt();
        assert_ne!(receipt.schema_version, 0);
        assert_ne!(receipt.generation, 0);
        assert_eq!(receipt.backend_api, "vulkan");
        assert!(!receipt.adapter_name.is_empty());
        assert!(receipt.adapter_name.len() <= 256);
        assert!(!receipt.backend_version.is_empty());
        assert!(receipt.backend_version.len() <= 64);
        assert!(receipt.backend_version.is_ascii());
        let core_end = receipt
            .backend_version
            .find(['-', '+'])
            .unwrap_or(receipt.backend_version.len());
        let semver_core = receipt.backend_version[..core_end]
            .split('.')
            .collect::<Vec<_>>();
        assert_eq!(semver_core.len(), 3);
        assert!(semver_core.iter().all(|component| !component.is_empty()
            && component
                .chars()
                .all(|character| character.is_ascii_digit())));
        assert!(receipt
            .backend_version
            .bytes()
            .all(|byte| { byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+') }));
        assert!(receipt.backend_api.is_ascii());
        assert!(receipt.backend_api.len() <= 32);
        assert!(receipt
            .backend_api
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'));
        assert_eq!(
            receipt.gpu_layout_version,
            GPU_CLOSED_LOOP_LAYOUT_VERSION as u16
        );
        assert!(receipt.driver_digest.iter().any(|word| *word != 0));
        assert!(receipt.feature_digest.iter().any(|word| *word != 0));
        assert!(receipt.limits_digest.iter().any(|word| *word != 0));
        assert_eq!(backend.shared_resource_counts_for_test(), (1, 1, 1));
        assert_eq!(backend.shared_kernel_set_count_for_test(), 1);
        assert!(matches!(backend.state(), GpuBackendState::Ready));

        let second = required_backend();
        let other = second.hardware_receipt();
        assert_ne!(receipt.generation, other.generation);
        assert_eq!(receipt.driver_digest, other.driver_digest);
        assert_eq!(receipt.feature_digest, other.feature_digest);
        assert_eq!(receipt.limits_digest, other.limits_digest);
    }

    #[test]
    fn fixed_class_arena_allocates_lazily_rejects_exhaustion_and_never_grows() {
        let config = GpuClosedLoopRuntimeConfig {
            n512_slots: 2,
            n1024_slots: 1,
            n2048_slots: 1,
            aggregate_resident_ceiling_bytes: 128 * 1024 * 1024,
        };
        let mut backend = GpuClosedLoopBackend::new_required_with_config(config).unwrap();
        assert_eq!(backend.allocated_class_arena_count_for_test(), 0);
        backend
            .insert_brain(OrganismId(1), n512_phenotype(21))
            .unwrap();
        assert_eq!(backend.allocated_class_arena_count_for_test(), 1);
        backend
            .insert_brain(OrganismId(2), n512_phenotype(22))
            .unwrap();
        let before = backend.runtime_counters_for_test();
        assert!(backend
            .insert_brain(OrganismId(3), n512_phenotype(23))
            .is_err());
        assert_eq!(backend.allocated_class_arena_count_for_test(), 1);
        assert_eq!(backend.runtime_counters_for_test(), before);
        assert!(!backend.contains_organism_for_test(OrganismId(3)));
    }

    #[test]
    fn too_small_aggregate_resident_ceiling_rejects_without_any_runtime_mutation() {
        let config = GpuClosedLoopRuntimeConfig {
            n512_slots: 1,
            n1024_slots: 1,
            n2048_slots: 1,
            aggregate_resident_ceiling_bytes: 1,
        };
        let mut backend = GpuClosedLoopBackend::new_required_with_config(config).unwrap();
        assert_eq!(backend.allocated_class_arena_count_for_test(), 0);
        let before = backend.runtime_counters_for_test();
        assert!(backend
            .insert_brain(OrganismId(1), n512_phenotype(24))
            .is_err());
        assert_eq!(backend.allocated_class_arena_count_for_test(), 0);
        assert!(!backend.contains_organism_for_test(OrganismId(1)));
        assert_eq!(backend.runtime_counters_for_test(), before);
        assert_eq!(backend.last_compact_readback_bytes_for_test(), 0);
    }

    #[test]
    fn mixed_n512_n1024_n2048_uses_one_submit_generation_kernel_set_and_input_order() {
        let mut backend = required_backend();
        let phenotypes = [
            phenotype_for_capacity_at_maturation(
                BrainCapacityClass::n512(),
                81,
                0.35,
                SensorProfile::PrivilegedAffordanceV1,
            ),
            phenotype_for_capacity_at_maturation(
                BrainCapacityClass::n1024(),
                82,
                0.35,
                SensorProfile::PrivilegedAffordanceV1,
            ),
            phenotype_for_capacity_at_maturation(
                BrainCapacityClass::n2048(),
                83,
                0.35,
                SensorProfile::PrivilegedAffordanceV1,
            ),
        ];
        let handles = [
            backend
                .insert_brain(OrganismId(1), phenotypes[0].clone())
                .unwrap(),
            backend
                .insert_brain(OrganismId(2), phenotypes[1].clone())
                .unwrap(),
            backend
                .insert_brain(OrganismId(3), phenotypes[2].clone())
                .unwrap(),
        ];
        let frames = [
            perception_frame_for_profile_at_tick(
                1,
                100,
                SensorProfile::PrivilegedAffordanceV1,
                true,
                2,
            ),
            perception_frame_for_profile_at_tick(
                2,
                100,
                SensorProfile::PrivilegedAffordanceV1,
                false,
                1,
            ),
            perception_frame_for_profile_at_tick(
                3,
                100,
                SensorProfile::PrivilegedAffordanceV1,
                true,
                2,
            ),
        ];
        let order = [2_usize, 0, 1];
        let ordered_batch = order.map(|index| (handles[index], frames[index].clone()));
        let ticks = backend.tick_batch(&ordered_batch).unwrap();
        assert_eq!(ticks.len(), 3);
        assert_eq!(
            ticks.iter().map(|tick| tick.handle).collect::<Vec<_>>(),
            order.map(|index| handles[index])
        );
        assert!(ticks
            .iter()
            .all(|tick| tick.dispatch_generation == ticks[0].dispatch_generation));
        assert!(ticks.iter().all(|tick| tick.compact_readback_bytes == 48));
        assert_eq!(backend.last_compact_readback_bytes_for_test(), 3 * 48);
        assert_eq!(backend.completed_dispatch_count(), 1);
        assert_eq!(backend.perception_upload_count(), 3);
        assert_eq!(backend.completed_selection_count(), 3);
        assert_eq!(backend.shared_kernel_set_count_for_test(), 1);
        assert_eq!(backend.shared_resource_counts_for_test(), (1, 1, 1));
        for (tick, index) in ticks.iter().zip(order) {
            assert_eq!(
                tick.handle.phenotype_hash(),
                phenotypes[index].phenotype_hash()
            );
            assert_tick_identity(
                tick,
                handles[index],
                &frames[index],
                backend.hardware_receipt().generation,
            );
        }
    }

    #[test]
    fn n1024_and_n2048_each_execute_as_standalone_required_gpu_rows() {
        let mut backend = required_backend();
        for (index, capacity) in [BrainCapacityClass::n1024(), BrainCapacityClass::n2048()]
            .into_iter()
            .enumerate()
        {
            let organism_raw = 10 + index as u64;
            let phenotype = phenotype_for_capacity_at_maturation(
                capacity,
                91 + index as u64,
                0.35,
                SensorProfile::PrivilegedAffordanceV1,
            );
            let handle = backend
                .insert_brain(OrganismId(organism_raw), phenotype)
                .unwrap();
            let frame = perception_frame_for_profile_at_tick(
                organism_raw,
                200 + index as u64,
                SensorProfile::PrivilegedAffordanceV1,
                true,
                2,
            );
            let ticks = backend.tick_batch(&[(handle, frame.clone())]).unwrap();
            assert_eq!(ticks.len(), 1);
            assert_tick_identity(
                &ticks[0],
                handle,
                &frame,
                backend.hardware_receipt().generation,
            );
            assert_eq!(backend.last_compact_readback_bytes_for_test(), 48);
        }
        assert_eq!(backend.completed_dispatch_count(), 2);
        assert_eq!(backend.completed_selection_count(), 2);
        assert_eq!(backend.perception_upload_count(), 2);
        assert_eq!(backend.shared_kernel_set_count_for_test(), 1);
    }

    #[test]
    fn empty_and_duplicate_rows_fail_complete_preflight_without_consuming_state() {
        let mut backend = required_backend();
        let handle = backend
            .insert_brain(OrganismId(1), n512_phenotype(101))
            .unwrap();
        let empty: Vec<(GpuBrainHandle, PerceptionFrame)> = Vec::new();
        let before = backend.runtime_counters_for_test();
        assert!(backend.tick_batch(&empty).is_err());
        assert_eq!(backend.runtime_counters_for_test(), before);

        let frame = perception_frame_for_profile_at_tick(
            1,
            100,
            SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        assert!(backend
            .tick_batch(&[(handle, frame.clone()), (handle, frame)])
            .is_err());
        assert_eq!(backend.runtime_counters_for_test(), before);
        assert_eq!(backend.last_compact_readback_bytes_for_test(), 0);
    }

    #[test]
    fn all_invalid_row_commits_dispatch_and_side_but_returns_no_partial_selections() {
        let mut backend = required_backend();
        let first = backend
            .insert_brain(OrganismId(1), n512_phenotype(111))
            .unwrap();
        let second = backend
            .insert_brain(OrganismId(2), n512_phenotype(112))
            .unwrap();
        backend.force_all_invalid_after_next_decode_for_test(second);
        let first_frame = perception_frame_for_profile_at_tick(
            1,
            100,
            SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        let second_frame = perception_frame_for_profile_at_tick(
            2,
            100,
            SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        assert_eq!(
            backend
                .tick_batch(&[(first, first_frame), (second, second_frame)])
                .unwrap_err(),
            ScaffoldContractError::InvalidDecisionEvidence
        );
        assert!(matches!(backend.state(), GpuBackendState::Ready));
        assert_eq!(backend.completed_dispatch_count(), 1);
        assert_eq!(backend.perception_upload_count(), 2);
        assert_eq!(backend.completed_selection_count(), 0);
        assert_eq!(backend.last_compact_readback_bytes_for_test(), 2 * 48);

        let next_first = perception_frame_for_profile_at_tick(
            1,
            101,
            SensorProfile::PrivilegedAffordanceV1,
            false,
            2,
        );
        let next_second = perception_frame_for_profile_at_tick(
            2,
            101,
            SensorProfile::PrivilegedAffordanceV1,
            false,
            2,
        );
        let ticks = backend
            .tick_batch(&[(first, next_first), (second, next_second)])
            .unwrap();
        assert_eq!(ticks[0].active_activation_side, 0);
        assert_eq!(ticks[1].active_activation_side, 0);
        assert_eq!(backend.completed_dispatch_count(), 2);
        assert_eq!(backend.perception_upload_count(), 4);
        assert_eq!(backend.completed_selection_count(), 2);
    }

    #[test]
    fn profile_mismatch_rejects_the_whole_batch_before_upload_or_generation() {
        let mut backend = required_backend();
        let privileged = n512_phenotype(31);
        let grounded = n512_phenotype_for_profile_at_maturation(
            32,
            0.35,
            SensorProfile::GroundedObjectSlotsV1,
        );
        let first = backend.insert_brain(OrganismId(1), privileged).unwrap();
        let second = backend.insert_brain(OrganismId(2), grounded).unwrap();
        let frame_a = perception_frame_for_profile_at_tick(
            1,
            100,
            SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        let frame_b = perception_frame_for_profile_at_tick(
            2,
            100,
            SensorProfile::GroundedObjectSlotsV1,
            true,
            2,
        );
        let baseline = backend
            .tick_batch(&[(first, frame_a.clone()), (second, frame_b.clone())])
            .unwrap();
        let baseline_generation = baseline[0].dispatch_generation;
        let before = (
            backend.completed_dispatch_count(),
            backend.completed_selection_count(),
            backend.perception_upload_count(),
        );

        let wrong_profile = perception_frame_for_profile_at_tick(
            2,
            101,
            SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        assert_eq!(
            backend
                .tick_batch(&[(first, frame_a), (second, wrong_profile)])
                .unwrap_err(),
            ScaffoldContractError::SensorProfileMismatch
        );
        assert_eq!(
            (
                backend.completed_dispatch_count(),
                backend.completed_selection_count(),
                backend.perception_upload_count(),
            ),
            before
        );

        let next_a = perception_frame_for_profile_at_tick(
            1,
            102,
            SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        let next_b = perception_frame_for_profile_at_tick(
            2,
            102,
            SensorProfile::GroundedObjectSlotsV1,
            true,
            2,
        );
        let next = backend
            .tick_batch(&[(first, next_a), (second, next_b)])
            .unwrap();
        assert_eq!(next[0].dispatch_generation, baseline_generation + 1);
        assert_eq!(next[0].dispatch_generation, next[1].dispatch_generation);
    }

    #[test]
    fn organism_mismatch_and_duplicate_residency_reject_before_upload() {
        let mut backend = required_backend();
        let phenotype = n512_phenotype(41);
        let handle = backend
            .insert_brain(OrganismId(1), phenotype.clone())
            .unwrap();
        assert_eq!(handle.organism_id(), OrganismId(1));
        assert_eq!(
            backend.insert_brain(OrganismId(1), phenotype).unwrap_err(),
            ScaffoldContractError::BrainOwnershipMismatch
        );
        let wrong_frame = perception_frame_for_profile_at_tick(
            2,
            100,
            SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        let before = (
            backend.completed_dispatch_count(),
            backend.completed_selection_count(),
            backend.perception_upload_count(),
        );
        assert_eq!(
            backend.tick_batch(&[(handle, wrong_frame)]).unwrap_err(),
            ScaffoldContractError::BrainOwnershipMismatch
        );
        assert_eq!(
            (
                backend.completed_dispatch_count(),
                backend.completed_selection_count(),
                backend.perception_upload_count(),
            ),
            before
        );
    }

    #[test]
    fn foreign_backend_handle_is_rejected_all_or_nothing() {
        let mut a = required_backend();
        let mut b = required_backend();
        let phenotype = n512_phenotype(51);
        let handle_a = a.insert_brain(OrganismId(1), phenotype.clone()).unwrap();
        let handle_b = b.insert_brain(OrganismId(1), phenotype).unwrap();
        assert_eq!(
            (handle_a.class_id(), handle_a.slot(), handle_a.generation()),
            (handle_b.class_id(), handle_b.slot(), handle_b.generation())
        );
        let frame = perception_frame_for_profile_at_tick(
            1,
            100,
            SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        assert_eq!(
            b.tick_batch(&[(handle_a, frame)]).unwrap_err(),
            ScaffoldContractError::BrainOwnershipMismatch
        );
        assert_eq!(b.completed_dispatch_count(), 0);
        assert_eq!(b.completed_selection_count(), 0);
        assert_eq!(b.perception_upload_count(), 0);
        assert_eq!(
            b.remove_brain(handle_a).unwrap_err(),
            ScaffoldContractError::BrainOwnershipMismatch
        );
        assert_eq!(b.runtime_counters_for_test(), (0, 0, 0));
    }

    #[test]
    fn removed_handle_is_stale_after_generation_checked_slot_reuse() {
        let mut backend = required_backend();
        let first = backend
            .insert_brain(OrganismId(1), n512_phenotype(61))
            .unwrap();
        let first_frame = perception_frame_for_profile_at_tick(
            1,
            100,
            SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        let first_tick = backend.tick_batch(&[(first, first_frame)]).unwrap();
        assert_eq!(first_tick[0].active_activation_side, 1);
        backend.remove_brain(first).unwrap();
        assert_eq!(
            backend.remove_brain(first).unwrap_err(),
            ScaffoldContractError::BrainOwnershipMismatch
        );
        let second = backend
            .insert_brain(OrganismId(2), n512_phenotype(62))
            .unwrap();
        assert_eq!(first.slot(), second.slot());
        assert_ne!(first.generation(), second.generation());
        let frame = perception_frame_for_profile_at_tick(
            1,
            100,
            SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        assert_eq!(
            backend.tick_batch(&[(first, frame)]).unwrap_err(),
            ScaffoldContractError::BrainOwnershipMismatch
        );
        assert_eq!(backend.completed_dispatch_count(), 1);
        assert_eq!(backend.perception_upload_count(), 1);
        let second_frame = perception_frame_for_profile_at_tick(
            2,
            101,
            SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        let second_tick = backend.tick_batch(&[(second, second_frame)]).unwrap();
        assert_eq!(second_tick[0].active_activation_side, 1);
    }

    #[test]
    fn two_heterogeneous_brains_share_resources_but_keep_recurrent_state_disjoint() {
        let mut backend = required_backend();
        let [phenotype_a, phenotype_b] = heterogeneous_n512_phenotypes();
        let control_phenotype_b = phenotype_b.clone();
        let hash_a = phenotype_a.phenotype_hash();
        let hash_b = phenotype_b.phenotype_hash();
        let expected_a =
            expected_cadence_counts(&phenotype_a, u32::from(phenotype_a.microstep_count()));
        let expected_b =
            expected_cadence_counts(&phenotype_b, u32::from(phenotype_b.microstep_count()));
        let handle_a = backend.insert_brain(OrganismId(1), phenotype_a).unwrap();
        let handle_b = backend.insert_brain(OrganismId(2), phenotype_b).unwrap();
        assert_eq!(handle_a.phenotype_hash(), hash_a);
        assert_eq!(handle_b.phenotype_hash(), hash_b);
        assert_ne!(handle_a.slot(), handle_b.slot());
        assert_eq!(backend.shared_resource_counts_for_test(), (1, 1, 1));
        let receipt_generation = backend.hardware_receipt().generation;

        let frame_a_1 = perception_frame_for_profile_at_tick(
            1,
            100,
            SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        let frame_b_1 = perception_frame_for_profile_at_tick(
            2,
            100,
            SensorProfile::PrivilegedAffordanceV1,
            false,
            1,
        );
        let first = backend
            .tick_batch(&[(handle_a, frame_a_1.clone()), (handle_b, frame_b_1.clone())])
            .unwrap();
        assert_eq!(first.len(), 2);
        assert_eq!(first[0].dispatch_generation, first[1].dispatch_generation);
        assert_tick_identity(&first[0], handle_a, &frame_a_1, receipt_generation);
        assert_tick_identity(&first[1], handle_b, &frame_b_1, receipt_generation);
        assert_eq!(first[0].active_activation_side, 1);
        assert_eq!(first[1].active_activation_side, 1);
        assert_eq!(
            (
                first[0].selection.active_tiles,
                first[0].selection.active_synapses
            ),
            expected_a
        );
        assert_eq!(
            (
                first[1].selection.active_tiles,
                first[1].selection.active_synapses
            ),
            expected_b
        );

        let frame_a_2 = perception_frame_for_profile_at_tick(
            1,
            101,
            SensorProfile::PrivilegedAffordanceV1,
            false,
            2,
        );
        let only_a = backend
            .tick_batch(&[(handle_a, frame_a_2.clone())])
            .unwrap();
        assert_eq!(only_a[0].active_activation_side, 0);
        assert_eq!(
            (
                only_a[0].selection.active_tiles,
                only_a[0].selection.active_synapses
            ),
            expected_a
        );

        let frame_a_3 = perception_frame_for_profile_at_tick(
            1,
            102,
            SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        let frame_b_3 = perception_frame_for_profile_at_tick(
            2,
            102,
            SensorProfile::PrivilegedAffordanceV1,
            true,
            1,
        );
        let third = backend
            .tick_batch(&[(handle_a, frame_a_3.clone()), (handle_b, frame_b_3.clone())])
            .unwrap();
        assert_eq!(third[0].active_activation_side, 1);
        assert_eq!(third[1].active_activation_side, 0);
        assert_tick_identity(&third[0], handle_a, &frame_a_3, receipt_generation);
        assert_tick_identity(&third[1], handle_b, &frame_b_3, receipt_generation);
        assert_eq!(
            (
                third[0].selection.active_tiles,
                third[0].selection.active_synapses
            ),
            expected_a
        );
        assert_eq!(
            (
                third[1].selection.active_tiles,
                third[1].selection.active_synapses
            ),
            expected_b
        );
        assert_eq!(backend.shared_resource_counts_for_test(), (1, 1, 1));

        let mut control = required_backend();
        let control_b = control
            .insert_brain(OrganismId(2), control_phenotype_b)
            .unwrap();
        let control_first = control
            .tick_batch(&[(control_b, frame_b_1.clone())])
            .unwrap();
        let control_third = control
            .tick_batch(&[(control_b, frame_b_3.clone())])
            .unwrap();
        for (interleaved, isolated) in [
            (&first[1], &control_first[0]),
            (&third[1], &control_third[0]),
        ] {
            assert_eq!(
                interleaved.selection.candidate_index,
                isolated.selection.candidate_index
            );
            assert!((interleaved.selection.logit - isolated.selection.logit).abs() <= 1.0e-6);
            assert_eq!(
                interleaved.active_activation_side,
                isolated.active_activation_side
            );
            assert_eq!(
                (
                    interleaved.selection.active_tiles,
                    interleaved.selection.active_synapses
                ),
                (
                    isolated.selection.active_tiles,
                    isolated.selection.active_synapses
                )
            );
        }
    }

    #[test]
    fn deterministic_replay_matches_across_fresh_backends_excluding_process_local_ids() {
        let phenotype = n512_phenotype(121);
        let mut first_backend = required_backend();
        let mut second_backend = required_backend();
        let first_handle = first_backend
            .insert_brain(OrganismId(1), phenotype.clone())
            .unwrap();
        let second_handle = second_backend
            .insert_brain(OrganismId(1), phenotype)
            .unwrap();
        assert_ne!(first_handle, second_handle);
        assert_eq!(first_handle.class_id(), second_handle.class_id());
        assert_eq!(first_handle.slot(), second_handle.slot());
        assert_eq!(first_handle.generation(), second_handle.generation());
        assert_eq!(first_handle.organism_id(), second_handle.organism_id());
        assert_eq!(
            first_handle.phenotype_hash(),
            second_handle.phenotype_hash()
        );
        assert_ne!(
            first_backend.hardware_receipt().generation,
            second_backend.hardware_receipt().generation
        );

        for offset in 0..8_u64 {
            let frame = perception_frame_for_profile_at_tick(
                1,
                300 + offset,
                SensorProfile::PrivilegedAffordanceV1,
                offset % 2 == 0,
                if offset % 3 == 0 { 1 } else { 2 },
            );
            let first = first_backend
                .tick_batch(&[(first_handle, frame.clone())])
                .unwrap()
                .into_iter()
                .next()
                .unwrap();
            let second = second_backend
                .tick_batch(&[(second_handle, frame)])
                .unwrap()
                .into_iter()
                .next()
                .unwrap();
            assert_eq!(
                first.selection.candidate_index,
                second.selection.candidate_index
            );
            assert!((first.selection.logit - second.selection.logit).abs() <= 1.0e-6);
            assert_eq!(first.selection.confidence, second.selection.confidence);
            assert_eq!(first.selection.active_tiles, second.selection.active_tiles);
            assert_eq!(
                first.selection.active_synapses,
                second.selection.active_synapses
            );
            assert_eq!(first.active_activation_side, second.active_activation_side);
            assert_eq!(first.base_digest, second.base_digest);
            assert_eq!(first.frame_digest, second.frame_digest);
            assert_eq!(first.compact_readback_bytes, 48);
            assert_eq!(second.compact_readback_bytes, 48);
        }
    }

    #[test]
    fn device_loss_is_fail_stop_and_never_switches_policy() {
        let mut backend = required_backend();
        let handle_a = backend
            .insert_brain(OrganismId(1), n512_phenotype(71))
            .unwrap();
        let n1024 = phenotype_for_capacity_at_maturation(
            BrainCapacityClass::n1024(),
            72,
            0.35,
            SensorProfile::PrivilegedAffordanceV1,
        );
        let handle_b = backend.insert_brain(OrganismId(2), n1024).unwrap();
        backend.force_device_lost_after_next_submit_for_test();
        let frame_a = perception_frame_for_profile_at_tick(
            1,
            100,
            SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        let frame_b = perception_frame_for_profile_at_tick(
            2,
            100,
            SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        assert_eq!(
            backend
                .tick_batch(&[(handle_a, frame_a), (handle_b, frame_b)])
                .unwrap_err(),
            ScaffoldContractError::NeuralBackendUnavailable
        );
        assert!(matches!(
            backend.state(),
            GpuBackendState::DeviceLost { .. }
        ));
        assert_eq!(backend.completed_dispatch_count(), 0);
        assert_eq!(backend.completed_selection_count(), 0);

        let later_a = perception_frame_for_profile_at_tick(
            1,
            101,
            SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        let later_b = perception_frame_for_profile_at_tick(
            2,
            101,
            SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        let uploads = backend.perception_upload_count();
        assert_eq!(
            backend.tick_batch(&[(handle_a, later_a)]).unwrap_err(),
            ScaffoldContractError::NeuralBackendUnavailable
        );
        assert_eq!(
            backend.tick_batch(&[(handle_b, later_b)]).unwrap_err(),
            ScaffoldContractError::NeuralBackendUnavailable
        );
        assert_eq!(backend.perception_upload_count(), uploads);
        assert_eq!(backend.completed_selection_count(), 0);
        assert_eq!(
            backend.remove_brain(handle_a).unwrap_err(),
            ScaffoldContractError::NeuralBackendUnavailable
        );
        assert_eq!(
            backend.remove_brain(handle_b).unwrap_err(),
            ScaffoldContractError::NeuralBackendUnavailable
        );
    }
}
