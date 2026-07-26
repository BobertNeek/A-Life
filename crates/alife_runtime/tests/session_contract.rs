use alife_core::{BrainCheckpointMode, ScaffoldContractError, Tick};
use alife_runtime::{
    DurableGpuCheckpointRef, GpuAuthoritativeSession, GpuSessionAuthority, GpuSessionConsumerKind,
    GpuSessionFailStopCause,
};

#[test]
fn checkpoint_modes_are_explicit_and_stable() {
    assert_eq!(
        BrainCheckpointMode::GeneticRebuild.slug(),
        "genetic-rebuild"
    );
    assert_eq!(
        BrainCheckpointMode::DurableLearnedFounder.slug(),
        "durable-learned-founder"
    );
    assert_eq!(BrainCheckpointMode::ExactResume.slug(), "exact-resume");
}

#[test]
fn device_loss_fail_stops_actions_and_retains_latest_durable_checkpoint() {
    let mut authority = GpuSessionAuthority::new(GpuSessionConsumerKind::Gameplay);
    let durable = DurableGpuCheckpointRef::try_new(
        Tick::new(41),
        "fnv1a64:0123456789abcdef".to_string(),
        [1, 2, 3, 4],
    )
    .unwrap();
    authority.note_durable_checkpoint(durable.clone()).unwrap();

    authority.fail_stop(GpuSessionFailStopCause::DeviceLost);

    assert_eq!(authority.latest_durable_checkpoint(), Some(&durable));
    assert_eq!(
        authority.ensure_neural_actions_available(),
        Err(ScaffoldContractError::NeuralBackendUnavailable)
    );
}

#[test]
fn shared_session_is_the_backend_owner_type() {
    let _constructor: fn(
        alife_gpu_backend::GpuClosedLoopBackend,
        GpuSessionConsumerKind,
    ) -> GpuAuthoritativeSession = GpuAuthoritativeSession::new;
}
