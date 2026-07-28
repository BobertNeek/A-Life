//! Strict vNext GPU-brain persistence and inspection-only migration contracts.

use alife_core::{
    BrainActivityPolicyV1, BrainClassId, BrainWorkCounters, NeuralThrottleLevel,
    ScaffoldContractError,
};
use alife_world::persistence::{
    load_legacy_large_tier_for_inspection, GpuBackendProvenanceSave, GpuBrainAssetRef,
    InspectionOnlyLegacyBrainState, NeuralGpuBackendApi, PortableAssetDigest,
    PortableThrottleCheckpoint, ProductionNeuralAvailability, ThrottleReplaySaveInput,
    ThrottleReplaySaveState, GPU_BACKEND_PROVENANCE_SAVE_SCHEMA_VERSION,
    INSPECTION_ONLY_BRAIN_SCHEMA_VERSION, PORTABLE_THROTTLE_CHECKPOINT_SCHEMA_VERSION,
    THROTTLE_REPLAY_SAVE_SCHEMA_VERSION,
};

fn asset(id: &str) -> GpuBrainAssetRef {
    GpuBrainAssetRef {
        asset_id: id.to_owned(),
        digest: PortableAssetDigest("fnv1a64:7a7a7a7a7a7a7a7a".to_owned()),
    }
}

fn provenance(name: &str) -> GpuBackendProvenanceSave {
    let mut provenance = GpuBackendProvenanceSave {
        schema_version: GPU_BACKEND_PROVENANCE_SAVE_SCHEMA_VERSION,
        backend_api_raw: NeuralGpuBackendApi::Vulkan.raw(),
        vendor_id: 0x1002,
        device_id: 0x744c,
        backend_version_major: 0,
        backend_version_minor: 1,
        backend_version_patch: 0,
        adapter_name_len: 0,
        adapter_name_utf8: [0; 128],
        driver_digest: [1, 2, 3, 4],
        required_features_digest: [5, 6, 7, 8],
        required_limits_digest: [9, 10, 11, 12],
        available_features_digest: [13, 14, 15, 16],
        adapter_limits_digest: [17, 18, 19, 20],
    };
    provenance.set_adapter_name(name).unwrap();
    provenance
}

fn portable_checkpoint(level: NeuralThrottleLevel) -> PortableThrottleCheckpoint {
    let policy = BrainActivityPolicyV1::production_v1();
    let work = BrainWorkCounters {
        microsteps: 2,
        neuron_updates: 1_024,
        tile_visits: 32,
        synapse_ops: 8_192,
        decoder_candidate_ops: 4,
        memory_context_ops: 4,
    };
    let neural_cost_q24 = policy.cost.neural_cost_q24(&work).unwrap();
    let atp_debit_q16 = policy
        .cost
        .q24_to_atp_q16_round_half_up(neural_cost_q24)
        .unwrap();
    let (completed_gpu_time_ns, queue_depth, logical_heap_pressure_q16, brain_atp_fraction_q16) =
        match level {
            NeuralThrottleLevel::Full => (1_000_000, 0, 16_384, 60_000),
            NeuralThrottleLevel::Reduced => (2_500_000, 1, 32_768, 49_151),
            NeuralThrottleLevel::EssentialOnly => (4_500_000, 2, 49_152, 32_767),
        };
    PortableThrottleCheckpoint {
        schema_version: PORTABLE_THROTTLE_CHECKPOINT_SCHEMA_VERSION,
        policy_version: policy.policy_version,
        organism_id_raw: 7,
        tick: 41,
        class_id_raw: 1,
        sequence_cursor: 3,
        dispatch_generation: 9,
        frame_digest: [11, 12, 13, 14],
        source_dispatch_generation: 8,
        source_frame_digest: [7, 8, 9, 10],
        completed_gpu_time_ns,
        queue_depth,
        logical_heap_pressure_q16,
        brain_atp_fraction_q16,
        level,
        microsteps: 2,
        enabled_route_ids: vec![0, 2, 5],
        route_schedule_digest: [21, 22, 23, 24],
        work,
        neural_cost_q24,
        atp_before_q16: 60_000,
        atp_debit_q16,
        atp_after_q16: 60_000 - atp_debit_q16,
        policy_digest: policy.policy_digest,
        portable_digest: [0; 4],
    }
    .seal()
    .unwrap()
}

#[test]
fn backend_api_wire_mapping_is_exact_and_rejects_unknown_spellings() {
    assert_eq!(NeuralGpuBackendApi::Vulkan.raw(), 1);
    assert_eq!(NeuralGpuBackendApi::Vulkan.slug(), "vulkan");
    assert_eq!(
        NeuralGpuBackendApi::try_from_raw(1),
        Ok(NeuralGpuBackendApi::Vulkan)
    );
    assert_eq!(
        NeuralGpuBackendApi::try_from_slug("vulkan"),
        Ok(NeuralGpuBackendApi::Vulkan)
    );
    assert!(NeuralGpuBackendApi::try_from_raw(0).is_err());
    assert!(NeuralGpuBackendApi::try_from_raw(2).is_err());
    assert!(NeuralGpuBackendApi::try_from_slug("Vulkan").is_err());
    assert_eq!(
        serde_json::to_string(&NeuralGpuBackendApi::Vulkan).unwrap(),
        "1"
    );
    assert_eq!(
        serde_json::from_str::<NeuralGpuBackendApi>("1").unwrap(),
        NeuralGpuBackendApi::Vulkan
    );
    assert!(serde_json::from_str::<NeuralGpuBackendApi>("2").is_err());
}

#[test]
fn provenance_bounds_adapter_name_and_excludes_it_from_portable_compatibility() {
    let first = provenance("first adapter");
    first.validate().unwrap();
    let portable = first.portable_compatibility_digest().unwrap();
    let same_adapter = first.same_adapter_digest().unwrap();

    let mut renamed = first.clone();
    renamed.set_adapter_name("renamed display adapter").unwrap();
    renamed.validate().unwrap();
    assert_eq!(renamed.portable_compatibility_digest().unwrap(), portable);
    assert_eq!(renamed.same_adapter_digest().unwrap(), same_adapter);

    assert!(renamed.set_adapter_name(&"x".repeat(129)).is_err());
    renamed.adapter_name_utf8[127] = 1;
    assert!(renamed.validate().is_err());
}

#[test]
fn restore_compatibility_rejects_required_capability_and_same_adapter_drift() {
    let saved = provenance("saved adapter");

    for mutate in [
        |value: &mut GpuBackendProvenanceSave| value.required_features_digest[0] ^= 1,
        |value: &mut GpuBackendProvenanceSave| value.required_limits_digest[0] ^= 1,
    ] {
        let mut current = saved.clone();
        mutate(&mut current);
        assert!(saved.validate_portable_restore_against(&current).is_err());
    }

    let mut renamed = saved.clone();
    renamed
        .set_adapter_name("driver supplied display rename")
        .unwrap();
    saved
        .validate_same_adapter_replay_against(&renamed)
        .unwrap();

    for mutate in [
        |value: &mut GpuBackendProvenanceSave| value.vendor_id ^= 1,
        |value: &mut GpuBackendProvenanceSave| value.device_id ^= 1,
        |value: &mut GpuBackendProvenanceSave| value.driver_digest[0] ^= 1,
        |value: &mut GpuBackendProvenanceSave| value.available_features_digest[0] ^= 1,
        |value: &mut GpuBackendProvenanceSave| value.adapter_limits_digest[0] ^= 1,
    ] {
        let mut current = saved.clone();
        mutate(&mut current);
        assert!(saved
            .validate_same_adapter_replay_against(&current)
            .is_err());
    }
}

#[test]
fn portable_throttle_roundtrip_covers_every_level_and_rejects_tamper() {
    for level in [
        NeuralThrottleLevel::Full,
        NeuralThrottleLevel::Reduced,
        NeuralThrottleLevel::EssentialOnly,
    ] {
        let checkpoint = portable_checkpoint(level);
        checkpoint.validate().unwrap();
        let encoded = serde_json::to_vec(&checkpoint).unwrap();
        let decoded: PortableThrottleCheckpoint = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, checkpoint);

        let mut tampered = checkpoint.clone();
        tampered.atp_after_q16 ^= 1;
        assert!(tampered.validate().is_err());

        let mut wrong_frame = checkpoint.clone();
        wrong_frame.frame_digest[0] ^= 1;
        assert!(wrong_frame.validate().is_err());

        let mut wrong_dispatch = checkpoint.clone();
        wrong_dispatch.dispatch_generation += 1;
        assert!(wrong_dispatch.validate().is_err());

        let mut impossible_source = checkpoint.clone();
        impossible_source.source_dispatch_generation = impossible_source.dispatch_generation;
        impossible_source.portable_digest = [0; 4];
        assert!(impossible_source.seal().is_err());

        let mut missing_source_frame = checkpoint;
        missing_source_frame.source_frame_digest = [0; 4];
        missing_source_frame.portable_digest = [0; 4];
        assert!(missing_source_frame.seal().is_err());
    }
}

#[test]
fn throttle_sequence_accepts_only_the_exact_next_cursor_and_bound_asset() {
    let checkpoint = portable_checkpoint(NeuralThrottleLevel::Reduced);
    let state = ThrottleReplaySaveState::try_new(
        ThrottleReplaySaveInput {
            schema_version: THROTTLE_REPLAY_SAVE_SCHEMA_VERSION,
            policy_version: checkpoint.policy_version,
            next_sequence_cursor: checkpoint.sequence_cursor + 1,
            last_committed_sequence_cursor: Some(checkpoint.sequence_cursor),
            policy_digest: checkpoint.policy_digest,
            next_completed_gpu_time_ns: 1_250_000,
            brain_atp_q16: checkpoint.atp_after_q16,
            last_world_atp_tick: Some(checkpoint.tick),
        },
        asset("throttle-sequence"),
        Some(checkpoint.clone()),
    )
    .unwrap();
    state.validate().unwrap();
    let decoded: ThrottleReplaySaveState =
        serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
    assert_eq!(decoded, state);
    assert_eq!(decoded.next_completed_gpu_time_ns, 1_250_000);
    assert_eq!(decoded.brain_atp_q16, checkpoint.atp_after_q16);
    assert_eq!(decoded.last_world_atp_tick, Some(checkpoint.tick));
    state
        .validate_next_dispatch(
            checkpoint.organism_id_raw,
            checkpoint.class_id_raw,
            checkpoint.sequence_cursor + 1,
            checkpoint.policy_version,
            checkpoint.policy_digest,
        )
        .unwrap();

    for wrong_cursor in [checkpoint.sequence_cursor, checkpoint.sequence_cursor + 2] {
        assert!(state
            .validate_next_dispatch(
                checkpoint.organism_id_raw,
                checkpoint.class_id_raw,
                wrong_cursor,
                checkpoint.policy_version,
                checkpoint.policy_digest,
            )
            .is_err());
    }
    assert!(state
        .validate_next_dispatch(
            checkpoint.organism_id_raw + 1,
            checkpoint.class_id_raw,
            checkpoint.sequence_cursor + 1,
            checkpoint.policy_version,
            checkpoint.policy_digest,
        )
        .is_err());
    assert!(state
        .validate_next_dispatch(
            checkpoint.organism_id_raw,
            checkpoint.class_id_raw + 1,
            checkpoint.sequence_cursor + 1,
            checkpoint.policy_version,
            checkpoint.policy_digest,
        )
        .is_err());
    assert!(state
        .validate_next_dispatch(
            checkpoint.organism_id_raw,
            checkpoint.class_id_raw,
            checkpoint.sequence_cursor + 1,
            checkpoint.policy_version + 1,
            checkpoint.policy_digest,
        )
        .is_err());
    let mut wrong_policy_digest = checkpoint.policy_digest;
    wrong_policy_digest[0] ^= 1;
    assert!(state
        .validate_next_dispatch(
            checkpoint.organism_id_raw,
            checkpoint.class_id_raw,
            checkpoint.sequence_cursor + 1,
            checkpoint.policy_version,
            wrong_policy_digest,
        )
        .is_err());
    let mut wrong_asset = state.clone();
    wrong_asset.sequence_asset.digest = PortableAssetDigest("fnv1a64:6b6b6b6b6b6b6b6b".to_owned());
    assert!(wrong_asset.validate().is_err());
}

#[test]
fn promoted_ids_are_ready_large_legacy_ids_are_inspection_only_and_unknown_is_rejected() {
    for class_id_raw in 1..=3 {
        assert!(matches!(
            ProductionNeuralAvailability::for_saved_class(
                BrainClassId(class_id_raw),
                [31; 4],
                [32; 4]
            )
            .unwrap(),
            ProductionNeuralAvailability::ReadyGpu { .. }
        ));
    }
    for class_id_raw in 4..=8 {
        assert!(matches!(
            ProductionNeuralAvailability::for_saved_class(
                BrainClassId(class_id_raw),
                [31; 4],
                [32; 4]
            )
            .unwrap(),
            ProductionNeuralAvailability::InspectionOnly { .. }
        ));
    }
    assert_eq!(
        ProductionNeuralAvailability::for_saved_class(BrainClassId(99), [31; 4], [32; 4]),
        Err(ScaffoldContractError::UnsupportedProductionBrainClass)
    );
}

#[test]
fn n4096_legacy_save_loads_for_inspection_without_compile_or_gpu_allocation() {
    let state = InspectionOnlyLegacyBrainState::try_new(
        INSPECTION_ONLY_BRAIN_SCHEMA_VERSION,
        4,
        asset("legacy-n4096-brain"),
        1,
    )
    .unwrap();
    let result = load_legacy_large_tier_for_inspection(state).unwrap();
    assert!(matches!(
        result.availability,
        ProductionNeuralAvailability::InspectionOnly { .. }
    ));
    assert_eq!(result.phenotype_compile_count, 0);
    assert_eq!(result.gpu_admission_count, 0);
}

#[test]
fn portable_throttle_checkpoint_contains_no_live_handle_fields() {
    let source = include_str!("../src/persistence/gpu_brain_vnext.rs");
    let start = source
        .find("pub struct PortableThrottleCheckpoint")
        .unwrap();
    let body = &source[start..source[start..].find("}\n").unwrap() + start];
    assert!(!body.contains("handle_slot"));
    assert!(!body.contains("handle_generation"));
    assert!(!body.contains("backend_instance"));
}
