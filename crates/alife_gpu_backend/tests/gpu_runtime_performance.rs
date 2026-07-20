use alife_gpu_backend::{
    required_storage_buffers, GpuRuntimeBackendConfig, GpuRuntimeBackendKind,
    GpuRuntimeCapabilityManifest, GpuRuntimeReadbackGuard, GPU_CLOSED_LOOP_STORAGE_BINDINGS,
};

#[test]
fn required_gpu_status_is_authoritative_only_after_admission() {
    let status = GpuRuntimeBackendConfig::request(GpuRuntimeBackendKind::GpuAuthoritative)
        .with_gpu_feature_enabled(true)
        .with_hardware_available(true)
        .with_validation_passed(true)
        .select_backend()
        .unwrap();
    assert_eq!(status.selected, GpuRuntimeBackendKind::GpuAuthoritative);
    assert!(status.authoritative);
    assert!(status.unavailable_reason.is_none());
}

#[test]
fn unavailable_or_invalid_gpu_fails_closed() {
    assert!(
        GpuRuntimeBackendConfig::request(GpuRuntimeBackendKind::GpuAuthoritative)
            .with_hardware_available(false)
            .select_backend()
            .is_err()
    );
    assert!(
        GpuRuntimeBackendConfig::request(GpuRuntimeBackendKind::GpuAuthoritative)
            .with_hardware_available(true)
            .with_validation_passed(false)
            .select_backend()
            .is_err()
    );
}

#[test]
fn authority_manifest_and_readback_boundary_are_bounded() {
    let manifest = GpuRuntimeCapabilityManifest::current_contract();
    assert!(manifest.authoritative_closed_loop_available);
    assert!(manifest.product_gpu_required_default);
    assert_eq!(GPU_CLOSED_LOOP_STORAGE_BINDINGS, 7);
    assert_eq!(manifest.closed_loop_storage_bindings, 7);
    assert_eq!(
        required_storage_buffers(GpuRuntimeBackendKind::GpuAuthoritative),
        7
    );
    let guard = GpuRuntimeReadbackGuard::active_tick();
    assert!(!guard.permits_bulk_neural_readback());
    assert!(!guard.permits_per_synapse_readback());
    assert!(!guard.permits_weight_readback());
}
