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
fn durable_checkpoint_permit_owns_the_exact_prevalidated_reference() {
    let mut authority = GpuSessionAuthority::new(GpuSessionConsumerKind::Gameplay);
    let initial = DurableGpuCheckpointRef::try_new(
        Tick::new(41),
        "fnv1a64:0123456789abcdef".to_string(),
        [1, 2, 3, 4],
    )
    .unwrap();
    authority.note_durable_checkpoint(initial).unwrap();

    let exact = DurableGpuCheckpointRef::try_new(
        Tick::new(42),
        "fnv1a64:fedcba9876543210".to_string(),
        [5, 6, 7, 8],
    )
    .unwrap();
    let permit = authority
        .prevalidate_durable_checkpoint(exact.clone())
        .unwrap();
    authority.install_prevalidated_durable_checkpoint(permit);
    assert_eq!(authority.latest_durable_checkpoint(), Some(&exact));

    let regression = DurableGpuCheckpointRef::try_new(
        Tick::new(40),
        "fnv1a64:1111111111111111".to_string(),
        [9, 10, 11, 12],
    )
    .unwrap();
    assert_eq!(
        authority.prevalidate_durable_checkpoint(regression),
        Err(ScaffoldContractError::BrainActivitySequenceMismatch)
    );
}

#[test]
fn shared_session_is_the_backend_owner_type() {
    let _constructor: fn(
        alife_gpu_backend::GpuClosedLoopBackend,
        GpuSessionConsumerKind,
    ) -> GpuAuthoritativeSession = GpuAuthoritativeSession::new;
}
