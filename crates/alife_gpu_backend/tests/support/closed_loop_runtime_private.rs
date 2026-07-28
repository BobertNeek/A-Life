use super::*;

#[test]
fn unavailable_gpu_returns_typed_error_instead_of_cpu_fallback() {
    let result = GpuClosedLoopBackend::new_with_factory(&UnavailableGpuFactory);
    assert!(matches!(
        result,
        Err(ScaffoldContractError::NeuralBackendUnavailable)
    ));
}

#[test]
fn software_adapter_is_rejected_without_device_request() {
    let factory = SoftwareAdapterGpuFactory::default();
    let result = GpuClosedLoopBackend::new_with_factory(&factory);
    assert!(matches!(
        result,
        Err(ScaffoldContractError::NeuralBackendUnavailable)
    ));
    assert_eq!(factory.device_request_count(), 0);
}

#[test]
fn timestamp_queries_are_a_required_gpu_capability() {
    assert_eq!(required_device_features(), wgpu::Features::TIMESTAMP_QUERY);
    assert_eq!(
        validate_required_device_features(wgpu::Features::empty()),
        Err(ScaffoldContractError::GpuTimestampQueryUnavailable)
    );
    assert_eq!(
        validate_required_device_features(wgpu::Features::TIMESTAMP_QUERY),
        Ok(())
    );
}

#[test]
fn timestamp_period_bits_convert_ticks_without_float_arithmetic() {
    let one_ns = ExactGpuTimestampPeriod::try_from_f32_bits(1.0_f32.to_bits()).unwrap();
    assert_eq!(one_ns.elapsed_ns(100, 101).unwrap(), 1);

    let five_quarters = ExactGpuTimestampPeriod::try_from_f32_bits(1.25_f32.to_bits()).unwrap();
    assert_eq!(five_quarters.elapsed_ns(7, 11).unwrap(), 5);

    let half_ns = ExactGpuTimestampPeriod::try_from_f32_bits(0.5_f32.to_bits()).unwrap();
    assert_eq!(half_ns.elapsed_ns(3, 4).unwrap(), 1);
    assert_eq!(
        half_ns.elapsed_ns(4, 3),
        Err(ScaffoldContractError::GpuTimestampQueryUnavailable)
    );
    assert_eq!(
        ExactGpuTimestampPeriod::try_from_f32_bits(0.0_f32.to_bits()),
        Err(ScaffoldContractError::GpuTimestampQueryUnavailable)
    );
    assert_eq!(
        ExactGpuTimestampPeriod::try_from_f32_bits(f32::NAN.to_bits()),
        Err(ScaffoldContractError::GpuTimestampQueryUnavailable)
    );
}

#[test]
fn timestamp_mapping_status_is_checked_without_waiting_for_a_missing_callback() {
    let (_sender, receiver) = std::sync::mpsc::channel::<Result<(), wgpu::BufferAsyncError>>();
    assert!(!timestamp_mapping_completed(&receiver));

    let (sender, receiver) = std::sync::mpsc::channel::<Result<(), wgpu::BufferAsyncError>>();
    sender.send(Ok(())).unwrap();
    assert!(timestamp_mapping_completed(&receiver));
}

#[test]
fn stale_gpu_layout_is_rejected_before_slot_allocation() {
    let result = validate_required_gpu_layout_version(GPU_CLOSED_LOOP_LAYOUT_VERSION - 1);
    assert_eq!(result, Err(ScaffoldContractError::GpuLayoutMismatch));
}

#[test]
fn hardware_receipt_digests_are_canonical_complete_deterministic_and_sensitive() {
    let limits = wgpu::Limits::default();
    let expected_limit_words = [
        u64::from(limits.max_texture_dimension_1d),
        u64::from(limits.max_texture_dimension_2d),
        u64::from(limits.max_texture_dimension_3d),
        u64::from(limits.max_texture_array_layers),
        u64::from(limits.max_bind_groups),
        u64::from(limits.max_bindings_per_bind_group),
        u64::from(limits.max_dynamic_uniform_buffers_per_pipeline_layout),
        u64::from(limits.max_dynamic_storage_buffers_per_pipeline_layout),
        u64::from(limits.max_sampled_textures_per_shader_stage),
        u64::from(limits.max_samplers_per_shader_stage),
        u64::from(limits.max_storage_buffers_per_shader_stage),
        u64::from(limits.max_storage_textures_per_shader_stage),
        u64::from(limits.max_uniform_buffers_per_shader_stage),
        u64::from(limits.max_binding_array_elements_per_shader_stage),
        u64::from(limits.max_binding_array_acceleration_structure_elements_per_shader_stage),
        u64::from(limits.max_binding_array_sampler_elements_per_shader_stage),
        limits.max_uniform_buffer_binding_size,
        limits.max_storage_buffer_binding_size,
        u64::from(limits.max_vertex_buffers),
        limits.max_buffer_size,
        u64::from(limits.max_vertex_attributes),
        u64::from(limits.max_vertex_buffer_array_stride),
        u64::from(limits.max_inter_stage_shader_variables),
        u64::from(limits.min_uniform_buffer_offset_alignment),
        u64::from(limits.min_storage_buffer_offset_alignment),
        u64::from(limits.max_color_attachments),
        u64::from(limits.max_color_attachment_bytes_per_sample),
        u64::from(limits.max_compute_workgroup_storage_size),
        u64::from(limits.max_compute_invocations_per_workgroup),
        u64::from(limits.max_compute_workgroup_size_x),
        u64::from(limits.max_compute_workgroup_size_y),
        u64::from(limits.max_compute_workgroup_size_z),
        u64::from(limits.max_compute_workgroups_per_dimension),
        u64::from(limits.max_immediate_size),
        u64::from(limits.max_non_sampler_bindings),
        u64::from(limits.max_task_mesh_workgroup_total_count),
        u64::from(limits.max_task_mesh_workgroups_per_dimension),
        u64::from(limits.max_task_invocations_per_workgroup),
        u64::from(limits.max_task_invocations_per_dimension),
        u64::from(limits.max_mesh_invocations_per_workgroup),
        u64::from(limits.max_mesh_invocations_per_dimension),
        u64::from(limits.max_task_payload_size),
        u64::from(limits.max_mesh_output_vertices),
        u64::from(limits.max_mesh_output_primitives),
        u64::from(limits.max_mesh_output_layers),
        u64::from(limits.max_mesh_multiview_view_count),
        u64::from(limits.max_blas_primitive_count),
        u64::from(limits.max_blas_geometry_count),
        u64::from(limits.max_tlas_instance_count),
        u64::from(limits.max_acceleration_structures_per_shader_stage),
        u64::from(limits.max_multiview_view_count),
    ];
    assert_eq!(
        canonical_limit_words_for_test(&limits),
        expected_limit_words
    );
    let mut expected_limits =
        alife_core::CanonicalDigestBuilder::new(b"alife.gpu.hardware.limits.v1");
    expected_limits.write_sequence_len(expected_limit_words.len());
    for word in expected_limit_words {
        expected_limits.write_u64(word);
    }
    assert_eq!(
        canonical_limits_digest(&limits),
        expected_limits.finish256()
    );
    let mut changed_limits = limits.clone();
    changed_limits.max_multiview_view_count += 1;
    assert_ne!(
        canonical_limits_digest(&limits),
        canonical_limits_digest(&changed_limits)
    );

    let mut expected_driver =
        alife_core::CanonicalDigestBuilder::new(b"alife.gpu.hardware.driver.v1");
    expected_driver.write_sequence_len(2);
    expected_driver.write_utf8("driver");
    expected_driver.write_utf8("info");
    assert_eq!(
        canonical_driver_digest("driver", "info"),
        expected_driver.finish256()
    );
    assert_ne!(
        canonical_driver_digest("driver", "info"),
        canonical_driver_digest("driver", "changed")
    );
    let requested = wgpu::Features::empty();
    let enabled = wgpu::Features::TIMESTAMP_QUERY;
    let requested_words = requested.bits();
    let enabled_words = enabled.bits();
    let mut expected_features =
        alife_core::CanonicalDigestBuilder::new(b"alife.gpu.hardware.features.v1");
    expected_features.write_sequence_len(4);
    expected_features.write_u64(requested_words.0[0]);
    expected_features.write_u64(requested_words.0[1]);
    expected_features.write_u64(enabled_words.0[0]);
    expected_features.write_u64(enabled_words.0[1]);
    assert_eq!(
        canonical_feature_digest(requested, enabled),
        expected_features.finish256()
    );
    assert_ne!(
        canonical_feature_digest(requested, requested),
        canonical_feature_digest(requested, enabled)
    );
}

#[test]
fn backend_and_receipt_allocators_are_independent_checked_and_nonzero() {
    with_runtime_allocation_state_for_test(41, 91, || {
        assert_eq!(next_backend_instance_id().unwrap().get(), 41);
        assert_eq!(next_hardware_receipt_generation().unwrap().get(), 91);
        assert_eq!(next_backend_instance_id().unwrap().get(), 42);
        assert_eq!(next_hardware_receipt_generation().unwrap().get(), 92);
    });

    with_runtime_allocation_state_for_test(u64::MAX, u64::MAX, || {
        assert_eq!(next_backend_instance_id().unwrap().get(), u64::MAX);
        assert_eq!(
            next_backend_instance_id(),
            Err(GpuClosedLoopError::ArithmeticOverflow)
        );
        assert_eq!(next_hardware_receipt_generation().unwrap().get(), u64::MAX);
        assert_eq!(
            next_hardware_receipt_generation(),
            Err(GpuClosedLoopError::ArithmeticOverflow)
        );
    });
}

#[test]
fn removal_scrubs_every_reserved_range_before_slot_reuse() {
    let mut arena = RuntimeArenaTestHarness::n512(1);
    let first = arena
        .insert_fixture(OrganismId(1), PhenotypeHash([1; 4]))
        .unwrap();
    arena.fill_every_reserved_range(first, 0xa5a5_a5a5);
    arena.remove_fixture(first).unwrap();
    assert!(arena.every_reserved_range_is_zero(first.slot()));

    let second = arena
        .insert_fixture(OrganismId(2), PhenotypeHash([2; 4]))
        .unwrap();
    assert_eq!(first.slot(), second.slot());
    assert_ne!(first.generation(), second.generation());
    assert!(arena.every_reserved_range_is_zero(second.slot()));
}

#[test]
fn maximum_slot_generation_retires_permanently_instead_of_wrapping() {
    let mut arena = RuntimeArenaTestHarness::n512(1);
    let handle = arena
        .insert_fixture_with_generation(OrganismId(1), PhenotypeHash([1; 4]), u32::MAX)
        .unwrap();
    arena.remove_fixture(handle).unwrap();
    assert!(arena.slot_is_permanently_retired(handle.slot()));
    assert!(arena
        .insert_fixture(OrganismId(2), PhenotypeHash([2; 4]))
        .is_err());
    assert_eq!(arena.free_slot_count(), 0);
}

#[test]
fn failed_scrub_marks_device_lost_and_never_frees_or_reuses_the_slot() {
    let mut arena = RuntimeArenaTestHarness::n512(1);
    let handle = arena
        .insert_fixture(OrganismId(1), PhenotypeHash([1; 4]))
        .unwrap();
    arena.fail_next_scrub_after_submit();
    assert_eq!(
        arena.remove_fixture(handle),
        Err(ScaffoldContractError::NeuralBackendUnavailable)
    );
    assert!(matches!(arena.state(), GpuBackendState::DeviceLost { .. }));
    assert!(arena.owns(handle));
    assert_eq!(arena.free_slot_count(), 0);
    assert!(arena
        .insert_fixture(OrganismId(2), PhenotypeHash([2; 4]))
        .is_err());
}

#[test]
fn save_rebind_requires_explicit_matching_organism_ownership() {
    let mut arena = RuntimeArenaTestHarness::n512(1);
    let handle = arena
        .rebind_fixture_for_restore(OrganismId(7), PhenotypeHash([7; 4]))
        .unwrap();
    assert_eq!(handle.organism_id(), OrganismId(7));
    assert_eq!(
        arena.validate_frame_organism(handle, OrganismId(8)),
        Err(ScaffoldContractError::BrainOwnershipMismatch)
    );
    assert!(arena
        .rebind_fixture_for_restore(OrganismId(8), PhenotypeHash([7; 4]))
        .is_err());
}

#[test]
fn unsupported_n32k_class_rejects_before_arena_allocation() {
    let mut preflight = RuntimePreflightTestHarness::default();
    assert_eq!(
        preflight.validate_class(BrainClassId(5)),
        Err(ScaffoldContractError::UnsupportedProductionBrainClass)
    );
    assert_eq!(preflight.allocated_arena_count(), 0);
    assert_eq!(preflight.runtime_counters(), (0, 0, 0));
}

#[test]
fn tampered_frame_digest_rejects_before_upload_or_counter_mutation() {
    let mut preflight = RuntimePreflightTestHarness::default();
    let expected = PerceptionFrameDigest([1, 2, 3, 4]);
    let tampered = PerceptionFrameDigest([1, 2, 3, 5]);
    assert_eq!(
        preflight.validate_frame_digest(expected, tampered),
        Err(ScaffoldContractError::InvalidPerceptionFrame)
    );
    assert_eq!(preflight.runtime_counters(), (0, 0, 0));
    assert_eq!(preflight.perception_upload_count(), 0);
}
