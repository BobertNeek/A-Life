#![cfg(feature = "gpu-tests")]

use alife_core::ScaffoldContractError;
use alife_runtime::{GpuAuthoritativeSession, GpuSessionConsumerKind};

#[test]
#[ignore = "explicit software Vulkan offline training fixture; no gameplay qualification"]
fn software_session_exposes_offline_queue_without_live_authority() {
    let session = GpuAuthoritativeSession::new_software_vulkan_training_test().unwrap();
    assert_eq!(
        session.authority().consumer(),
        GpuSessionConsumerKind::Training
    );
    let info = session
        .backend()
        .software_training_test_adapter_info()
        .unwrap();
    assert_eq!(format!("{:?}", info.device_type), "Cpu");
    assert_eq!(format!("{:?}", info.backend), "Vulkan");
    assert!(session.backend().offline_training_device_queue().is_ok());
    assert_eq!(
        session.ensure_neural_actions_available(),
        Err(ScaffoldContractError::NeuralBackendUnavailable)
    );
    // Even an explicit caller cannot turn a test backend into usable gameplay.
    let backend = alife_gpu_backend::GpuClosedLoopBackend::new_software_vulkan_training_test(
        alife_gpu_backend::GpuRuntimeProfile::production_v1(),
    )
    .unwrap();
    let gameplay = GpuAuthoritativeSession::new(backend, GpuSessionConsumerKind::Gameplay);
    assert!(gameplay.ensure_neural_actions_available().is_err());
}
