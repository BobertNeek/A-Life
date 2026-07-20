//! Real-hardware ordering gate for the production world-to-GPU learning loop.
#![cfg(feature = "gpu-runtime")]

use alife_core::{BrainScaleTier, OrganismId, Vec3f};
use alife_game_app::{GpuLiveBrainRuntime, LiveBrainCausalStage};
use alife_gpu_backend::GpuClosedLoopBackend;
use alife_world::HeadlessScenarioBuilder;

#[test]
fn live_loop_seals_before_gpu_learning_commit() {
    let backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required GPU adapter");
    let world = HeadlessScenarioBuilder::new(9_201)
        .agent("learner", OrganismId(1), Vec3f::ZERO)
        .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.8)
        .build()
        .unwrap();
    let mut runtime =
        GpuLiveBrainRuntime::new(backend, world, 9_201, BrainScaleTier::Nano512).unwrap();

    let summaries = runtime.tick().unwrap();
    let summary = &summaries[0];
    let patch = &runtime.sealed_patches()[0];
    let receipt = runtime.last_learning_receipts()[0];

    assert_eq!(summary.learning_updates, 1);
    assert_eq!(summary.topology_updates, 1);
    assert_eq!(receipt.sequence_id, patch.header().sequence_id);
    assert_eq!(
        summary.causal_stages,
        vec![
            LiveBrainCausalStage::GatherSensory,
            LiveBrainCausalStage::RecallMemory,
            LiveBrainCausalStage::GpuBrainTick,
            LiveBrainCausalStage::ExecuteAction,
            LiveBrainCausalStage::MeasureOutcome,
            LiveBrainCausalStage::SealPatch,
            LiveBrainCausalStage::ApplyLearning,
            LiveBrainCausalStage::ObserveMemory,
            LiveBrainCausalStage::ObserveTopology,
            LiveBrainCausalStage::UpdateLogs,
        ]
    );
    let telemetry = runtime.authority_telemetry();
    assert_eq!(telemetry.learning_updates, 1);
    assert_eq!(telemetry.last_learning_delta, receipt.max_abs_delta);
}
