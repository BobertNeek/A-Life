//! Contract and real-GPU tests for shared-backend admission and complete VRAM accounting.

mod support;

#[cfg(feature = "gpu-tests")]
use alife_core::OrganismId;
use alife_core::{BrainCapacityClass, SensorProfile};
#[cfg(feature = "gpu-tests")]
use alife_gpu_backend::GpuRuntimeBudget;
use alife_gpu_backend::{GpuClassBucketPlan, GpuRuntimeProfile, GpuSlotAllocationReceipt};

#[cfg(feature = "gpu-tests")]
use support::scaling::finalized_memory_frame;
use support::scaling::{bounded_profile, populated};

#[test]
fn slot_receipt_accounts_for_every_gpu_buffer_category() {
    for capacity in BrainCapacityClass::production_classes() {
        let phenotype = populated(capacity, 4_411, SensorProfile::GroundedObjectSlotsV1);
        let plan = GpuClassBucketPlan::for_phenotype(&phenotype).unwrap();
        let receipt = plan.slot_allocation_receipt().unwrap();
        assert_eq!(
            receipt.logical_slot_commit_bytes,
            receipt.per_slot_component_bytes().checked_sum().unwrap()
        );
        assert!(receipt.shared_class_bytes > 0);
        assert!(receipt.immutable_topology_bytes > 0);
        assert!(receipt.activation_bytes > 0);
        assert!(receipt.learning_bytes > 0);
        assert!(receipt.candidate_and_memory_bytes > 0);
        assert!(receipt.diagnostic_and_readback_bytes > 0);
        assert!(receipt.staging_bytes > 0);
        receipt.validate_contract().unwrap();
    }
}

#[cfg(feature = "gpu-tests")]
#[test]
fn admission_is_runtime_budgeted_and_release_reclaims_exact_bytes() {
    use alife_gpu_backend::{
        GpuClosedLoopBackend, GpuClosedLoopMemoryBatchInput, GpuClosedLoopMemoryTickInput,
    };

    let phenotype = populated(
        BrainCapacityClass::n512(),
        4_412,
        SensorProfile::GroundedObjectSlotsV1,
    );
    let slot_bytes = GpuClassBucketPlan::for_phenotype(&phenotype)
        .unwrap()
        .slot_allocation_receipt()
        .unwrap()
        .logical_slot_commit_bytes;
    let profile = bounded_profile(slot_bytes * 2, 512 * 1024 * 1024, 2, 2);
    let mut backend = GpuClosedLoopBackend::new_required(profile).unwrap();
    let a = backend
        .insert_brain(OrganismId(1), phenotype.clone())
        .unwrap();
    let b = backend
        .insert_brain(OrganismId(2), phenotype.clone())
        .unwrap();
    assert_eq!(
        backend
            .admission_receipt()
            .last_event
            .expect("retained-slot admission event")
            .event_kind_raw,
        2
    );
    assert!(backend
        .insert_brain(OrganismId(3), phenotype.clone())
        .is_err());

    let (stale_frame, stale_recall) = finalized_memory_frame(1, 101);
    let stale_upload = backend
        .prepare_memory_context_upload(a, &stale_frame, &stale_recall)
        .unwrap();
    let (live_frame, live_recall) = finalized_memory_frame(2, 101);
    let live_upload = backend
        .prepare_memory_context_upload(b, &live_frame, &live_recall)
        .unwrap();

    let before_release = backend.admission_receipt().clone();
    backend.remove_brain(a).unwrap();
    let after_release = backend.admission_receipt().clone();
    assert_eq!(
        before_release.logical_committed_bytes - after_release.logical_committed_bytes,
        slot_bytes
    );
    assert_eq!(
        before_release.physical_allocated_bytes,
        after_release.physical_allocated_bytes
    );
    assert_eq!(
        after_release.physical_unused_retained_bytes,
        before_release.physical_unused_retained_bytes + slot_bytes
    );
    let release = after_release.last_event.unwrap();
    assert_eq!(release.event_kind_raw, 3);
    assert_eq!(
        release.logical_committed_before_bytes,
        before_release.logical_committed_bytes
    );
    assert_eq!(
        release.logical_committed_after_bytes,
        after_release.logical_committed_bytes
    );
    assert_eq!(
        release.physical_allocated_before_bytes,
        release.physical_allocated_after_bytes
    );
    assert_eq!(after_release.live_brains, 1);

    let stale =
        GpuClosedLoopMemoryBatchInput::try_new(vec![GpuClosedLoopMemoryTickInput::try_new(
            a,
            &stale_frame,
            &stale_upload,
        )
        .unwrap()])
        .unwrap();
    assert!(backend.tick_memory_batch(&stale).is_err());

    let live = GpuClosedLoopMemoryBatchInput::try_new(vec![GpuClosedLoopMemoryTickInput::try_new(
        b,
        &live_frame,
        &live_upload,
    )
    .unwrap()])
    .unwrap();
    let tick = backend.tick_memory_batch(&live).unwrap().remove(0);
    backend
        .discard_pending_eligibility(b, tick.pending_eligibility.identity())
        .unwrap();
}

#[cfg(feature = "gpu-tests")]
#[test]
fn zero_retention_profile_drops_the_empty_class_chunk() {
    use alife_gpu_backend::GpuClosedLoopBackend;

    let mut profile = bounded_profile(128 * 1024 * 1024, 256 * 1024 * 1024, 1, 1);
    profile.retain_empty_chunks = 0;
    let mut backend = GpuClosedLoopBackend::new_required(profile).unwrap();
    let phenotype = populated(
        BrainCapacityClass::n512(),
        4_414,
        SensorProfile::GroundedObjectSlotsV1,
    );
    let handle = backend
        .insert_brain(OrganismId(4), phenotype.clone())
        .unwrap();
    assert!(backend.admission_receipt().physical_allocated_bytes > 0);
    backend.remove_brain(handle).unwrap();
    let receipt = backend.admission_receipt();
    assert_eq!(receipt.live_brains, 0);
    assert_eq!(receipt.logical_committed_bytes, 0);
    assert_eq!(receipt.physical_allocated_bytes, 0);
    assert_eq!(backend.allocated_class_arena_count_for_test(), 0);
    assert_eq!(
        receipt
            .last_event
            .expect("drop-empty-chunk release event")
            .event_kind_raw,
        4
    );
    let rebound = backend.insert_brain(OrganismId(4), phenotype).unwrap();
    assert_ne!(handle, rebound);
    assert!(backend.remove_brain(handle).is_err());
    backend.remove_brain(rebound).unwrap();
}

#[cfg(feature = "gpu-tests")]
#[test]
fn shared_and_retained_bytes_are_never_counted_as_logically_committed_twice() {
    use alife_gpu_backend::GpuClosedLoopBackend;

    let profile = bounded_profile(512 * 1024 * 1024, 768 * 1024 * 1024, 3, 1);
    let mut backend = GpuClosedLoopBackend::new_required(profile).unwrap();
    for (index, capacity) in BrainCapacityClass::production_classes()
        .into_iter()
        .enumerate()
    {
        backend
            .insert_brain(
                OrganismId(index as u64 + 1),
                populated(
                    capacity,
                    4_420 + index as u64,
                    SensorProfile::GroundedObjectSlotsV1,
                ),
            )
            .unwrap();
    }
    let receipt = backend.admission_receipt();
    assert_eq!(
        receipt.physical_allocated_bytes,
        receipt.physical_shared_bytes
            + receipt.logical_committed_bytes
            + receipt.physical_unused_retained_bytes
            + receipt.physical_alignment_slack_bytes
    );
    assert!(receipt.logical_committed_bytes <= receipt.runtime.logical_neural_heap_budget_bytes);
    assert!(receipt.physical_allocated_bytes <= receipt.runtime.physical_allocation_ceiling_bytes);
    assert!(receipt.peak_physical_allocated_bytes >= receipt.physical_allocated_bytes);
}

#[cfg(feature = "gpu-tests")]
#[test]
fn heterogeneous_same_class_memory_batch_keeps_slot_state_disjoint() {
    use alife_core::Tick;
    use alife_gpu_backend::{
        GpuClosedLoopBackend, GpuClosedLoopMemoryBatchInput, GpuClosedLoopMemoryTickInput,
    };

    let phenotypes = [
        populated(
            BrainCapacityClass::n512(),
            4_450,
            SensorProfile::GroundedObjectSlotsV1,
        ),
        populated(
            BrainCapacityClass::n512(),
            4_451,
            SensorProfile::GroundedObjectSlotsV1,
        ),
    ];
    assert_ne!(
        phenotypes[0].phenotype_hash(),
        phenotypes[1].phenotype_hash()
    );
    let profile = bounded_profile(256 * 1024 * 1024, 512 * 1024 * 1024, 2, 2);
    let mut backend = GpuClosedLoopBackend::new_required(profile).unwrap();
    let handles = [
        backend
            .insert_brain(OrganismId(11), phenotypes[0].clone())
            .unwrap(),
        backend
            .insert_brain(OrganismId(12), phenotypes[1].clone())
            .unwrap(),
    ];
    assert_ne!(handles[0].slot(), handles[1].slot());
    assert_eq!(backend.shared_resource_counts_for_test(), (1, 1, 1));

    let (frame_a, recall_a) = finalized_memory_frame(11, 401);
    let (frame_b, recall_b) = finalized_memory_frame(12, 401);
    let upload_a = backend
        .prepare_memory_context_upload(handles[0], &frame_a, &recall_a)
        .unwrap();
    let upload_b = backend
        .prepare_memory_context_upload(handles[1], &frame_b, &recall_b)
        .unwrap();
    let batch = GpuClosedLoopMemoryBatchInput::try_new(vec![
        GpuClosedLoopMemoryTickInput::try_new(handles[0], &frame_a, &upload_a).unwrap(),
        GpuClosedLoopMemoryTickInput::try_new(handles[1], &frame_b, &upload_b).unwrap(),
    ])
    .unwrap();
    let ticks = backend.tick_memory_batch(&batch).unwrap();
    assert_eq!(
        ticks.iter().map(|tick| tick.handle).collect::<Vec<_>>(),
        handles
    );
    assert_eq!(ticks[0].dispatch_generation, ticks[1].dispatch_generation);
    for tick in &ticks {
        let binding = tick
            .memory_context_binding
            .expect("finalized memory binding");
        assert_eq!(binding.slot, tick.handle.slot());
        assert_eq!(binding.slot_generation, tick.handle.generation());
        backend
            .discard_pending_eligibility(tick.handle, tick.pending_eligibility.identity())
            .unwrap();
    }

    let checkpoint_tick = Tick::new(450);
    let untouched_before = backend
        .snapshot_brain(handles[1], checkpoint_tick)
        .unwrap()
        .canonical_digest();
    let (frame_a_only, recall_a_only) = finalized_memory_frame(11, 402);
    let upload_a_only = backend
        .prepare_memory_context_upload(handles[0], &frame_a_only, &recall_a_only)
        .unwrap();
    let only_a =
        GpuClosedLoopMemoryBatchInput::try_new(vec![GpuClosedLoopMemoryTickInput::try_new(
            handles[0],
            &frame_a_only,
            &upload_a_only,
        )
        .unwrap()])
        .unwrap();
    let tick_a = backend.tick_memory_batch(&only_a).unwrap().remove(0);
    backend
        .discard_pending_eligibility(handles[0], tick_a.pending_eligibility.identity())
        .unwrap();
    let untouched_after = backend
        .snapshot_brain(handles[1], checkpoint_tick)
        .unwrap()
        .canonical_digest();
    assert_eq!(untouched_before, untouched_after);
}

#[cfg(feature = "gpu-tests")]
#[test]
fn mixed_class_memory_batch_preserves_input_identity_on_one_backend() {
    use alife_gpu_backend::{
        GpuClosedLoopBackend, GpuClosedLoopMemoryBatchInput, GpuClosedLoopMemoryTickInput,
    };

    let capacities = [
        BrainCapacityClass::n512(),
        BrainCapacityClass::n1024(),
        BrainCapacityClass::n2048(),
    ];
    let profile = bounded_profile(512 * 1024 * 1024, 768 * 1024 * 1024, 3, 1);
    let mut backend = GpuClosedLoopBackend::new_required(profile).unwrap();
    let handles = capacities.map(|capacity| {
        let organism = OrganismId(20 + u64::from(capacity.id().raw()));
        backend
            .insert_brain(
                organism,
                populated(
                    capacity,
                    4_500 + u64::from(capacity.id().raw()),
                    SensorProfile::GroundedObjectSlotsV1,
                ),
            )
            .unwrap()
    });
    assert_eq!(backend.shared_resource_counts_for_test(), (1, 1, 1));
    assert_eq!(backend.allocated_class_arena_count_for_test(), 3);
    assert_eq!(backend.admission_receipt().live_brains, 3);

    let (frame_512, recall_512) = finalized_memory_frame(handles[0].organism_id().raw(), 501);
    let (frame_1024, recall_1024) = finalized_memory_frame(handles[1].organism_id().raw(), 501);
    let (frame_2048, recall_2048) = finalized_memory_frame(handles[2].organism_id().raw(), 501);
    let upload_512 = backend
        .prepare_memory_context_upload(handles[0], &frame_512, &recall_512)
        .unwrap();
    let upload_1024 = backend
        .prepare_memory_context_upload(handles[1], &frame_1024, &recall_1024)
        .unwrap();
    let upload_2048 = backend
        .prepare_memory_context_upload(handles[2], &frame_2048, &recall_2048)
        .unwrap();
    let expected_order = [handles[2], handles[0], handles[1]];
    let batch = GpuClosedLoopMemoryBatchInput::try_new(vec![
        GpuClosedLoopMemoryTickInput::try_new(handles[2], &frame_2048, &upload_2048).unwrap(),
        GpuClosedLoopMemoryTickInput::try_new(handles[0], &frame_512, &upload_512).unwrap(),
        GpuClosedLoopMemoryTickInput::try_new(handles[1], &frame_1024, &upload_1024).unwrap(),
    ])
    .unwrap();
    let ticks = backend.tick_memory_batch(&batch).unwrap();
    assert_eq!(
        ticks.iter().map(|tick| tick.handle).collect::<Vec<_>>(),
        expected_order
    );
    assert!(ticks
        .iter()
        .all(|tick| tick.dispatch_generation == ticks[0].dispatch_generation));
    for tick in ticks {
        let binding = tick
            .memory_context_binding
            .expect("finalized memory binding");
        assert_eq!(binding.slot, tick.handle.slot());
        assert_eq!(binding.slot_generation, tick.handle.generation());
        backend
            .discard_pending_eligibility(tick.handle, tick.pending_eligibility.identity())
            .unwrap();
    }
}

#[cfg(feature = "gpu-tests")]
#[test]
fn admission_receipt_rejects_aggregate_event_divergence() {
    use alife_gpu_backend::GpuClosedLoopBackend;

    let profile = bounded_profile(128 * 1024 * 1024, 256 * 1024 * 1024, 1, 1);
    let mut backend = GpuClosedLoopBackend::new_required(profile).unwrap();
    backend
        .insert_brain(
            OrganismId(31),
            populated(
                BrainCapacityClass::n512(),
                4_531,
                SensorProfile::GroundedObjectSlotsV1,
            ),
        )
        .unwrap();
    let mut divergent = backend.admission_receipt().clone();
    divergent.logical_committed_bytes += 4;
    divergent.logical_available_bytes -= 4;
    divergent.physical_allocated_bytes += 4;
    divergent.peak_logical_committed_bytes += 4;
    divergent.peak_physical_allocated_bytes += 4;
    assert!(divergent.validate_contract().is_err());
}

#[cfg(feature = "gpu-tests")]
#[test]
fn adapter_validation_rejects_each_missing_feature_limit_and_alignment() {
    let capacity = BrainCapacityClass::n512();
    let phenotype = populated(capacity, 4_430, SensorProfile::GroundedObjectSlotsV1);
    let profile = bounded_profile(128 * 1024 * 1024, 256 * 1024 * 1024, 4, 2);
    let sufficient = GpuRuntimeBudget::minimum_for_testing(profile, capacity.execution()).unwrap();
    GpuClassBucketPlan::validate_adapter(&phenotype, &sufficient).unwrap();

    let mut cases: Vec<(&str, GpuRuntimeBudget)> = Vec::new();
    macro_rules! below {
        ($name:literal, $field:ident) => {{
            let mut value = sufficient;
            value.$field = value.$field.saturating_sub(1);
            cases.push(($name, value));
        }};
    }
    below!("max buffer size", max_buffer_size);
    below!("storage binding size", max_storage_buffer_binding_size);
    below!("bind groups", max_bind_groups);
    below!("bindings per group", max_bindings_per_bind_group);
    below!("storage buffers", max_storage_buffers_per_shader_stage);
    below!("uniform buffers", max_uniform_buffers_per_shader_stage);
    below!(
        "dynamic storage buffers",
        max_dynamic_storage_buffers_per_pipeline_layout
    );
    below!(
        "dynamic uniform buffers",
        max_dynamic_uniform_buffers_per_pipeline_layout
    );
    below!("workgroup storage", max_compute_workgroup_storage_size);
    below!("workgroup x", max_compute_workgroup_size_x);
    below!("workgroup y", max_compute_workgroup_size_y);
    below!("workgroup z", max_compute_workgroup_size_z);
    below!(
        "workgroup invocations",
        max_compute_invocations_per_workgroup
    );
    below!(
        "workgroups per dimension",
        max_compute_workgroups_per_dimension
    );
    let mut missing_feature = sufficient;
    missing_feature.available_feature_mask &= !missing_feature.required_feature_mask;
    missing_feature.required_feature_mask = 1;
    cases.push(("required feature", missing_feature));
    let mut storage_alignment = sufficient;
    storage_alignment.storage_alignment_bytes =
        storage_alignment.storage_alignment_bytes.saturating_mul(2);
    cases.push(("storage alignment", storage_alignment));
    let mut uniform_alignment = sufficient;
    uniform_alignment.uniform_alignment_bytes =
        uniform_alignment.uniform_alignment_bytes.saturating_mul(2);
    cases.push(("uniform alignment", uniform_alignment));
    let mut copy_alignment = sufficient;
    copy_alignment.copy_buffer_alignment_bytes =
        copy_alignment.copy_buffer_alignment_bytes.saturating_mul(2);
    cases.push(("copy buffer alignment", copy_alignment));
    let mut row_alignment = sufficient;
    row_alignment.copy_bytes_per_row_alignment =
        row_alignment.copy_bytes_per_row_alignment.saturating_mul(2);
    cases.push(("copy row alignment", row_alignment));

    for (name, budget) in cases {
        assert!(
            GpuClassBucketPlan::validate_adapter(&phenotype, &budget).is_err(),
            "accepted insufficient {name}"
        );
    }
}

#[test]
fn runtime_profile_rejects_invalid_budget_and_retention_shapes() {
    let baseline = bounded_profile(1, 1, 1, 1);
    baseline.validate_contract().unwrap();
    for invalid in [
        GpuRuntimeProfile {
            logical_neural_heap_budget_bytes: 0,
            ..baseline
        },
        GpuRuntimeProfile {
            physical_allocation_ceiling_bytes: 0,
            ..baseline
        },
        GpuRuntimeProfile {
            max_hot_brains: 0,
            ..baseline
        },
        GpuRuntimeProfile {
            growth_chunk_slots: 0,
            ..baseline
        },
        GpuRuntimeProfile {
            retain_empty_chunks: 2,
            ..baseline
        },
    ] {
        assert!(invalid.validate_contract().is_err());
    }
}

fn _assert_receipt_is_public(_: GpuSlotAllocationReceipt) {}
