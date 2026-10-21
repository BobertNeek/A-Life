use alife_gpu_backend::{GpuRuntimeBackendConfig, GpuRuntimeBackendKind};

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
