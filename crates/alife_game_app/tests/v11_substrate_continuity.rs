#![cfg(feature = "gpu-runtime")]

use alife_core::{BrainScaleTier, OrganismId, OutcomeCreditPacket, Vec3f};
use alife_game_app::GpuLiveBrainRuntime;
use alife_gpu_backend::{GpuClosedLoopBackend, GpuRuntimeProfile};
use alife_world::HeadlessScenarioBuilder;

#[test]
fn v11_context_reaches_recurrent_memory_and_learning() {
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())
        .expect("the focused substrate gate requires the production GPU backend");
    let world = HeadlessScenarioBuilder::new(11_201)
        .agent("substrate", OrganismId(1), Vec3f::ZERO)
        .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.8)
        .build()
        .expect("bounded causal world");
    let mut runtime = GpuLiveBrainRuntime::new(backend, world, 11_201, BrainScaleTier::Nano512)
        .expect("supported Nano512 runtime");

    let first = runtime.tick().expect("first causal tick");
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].learning_updates, 1);
    assert!(runtime.last_activity_work_receipts()[0].counters.microsteps > 0);
    assert_eq!(runtime.last_memory_recall_receipts().len(), 1);
    assert_eq!(runtime.last_memory_update_receipts().len(), 1);
    assert_eq!(runtime.last_cognitive_context_digests().len(), 1);
    let first_context_digest = runtime.last_cognitive_context_digests()[0];
    let first_memory_context_digest = runtime.last_memory_recall_receipts()[0].context_digest;
    let first_patch = runtime
        .sealed_patches()
        .first()
        .expect("first sealed GPU patch");
    let first_credit = OutcomeCreditPacket::from_sealed_patch(first_patch)
        .expect("sealed GPU patch yields outcome credit");
    let first_learning = runtime
        .last_learning_receipts()
        .first()
        .copied()
        .expect("first GPU learning receipt");
    assert_eq!(first_learning.sequence_id, first_credit.sequence_id());
    assert_eq!(
        first_learning.dispatch_generation,
        first_credit.dispatch_generation()
    );
    assert_eq!(
        first_learning.active_activation_side,
        first_credit.active_activation_side()
    );
    assert!(first_learning.output_fast_generation > first_learning.input_fast_generation);

    let second = runtime.tick().expect("second causal tick");
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].learning_updates, 1);
    assert!(runtime.last_activity_work_receipts()[0].counters.microsteps > 0);
    assert_eq!(runtime.last_memory_recall_receipts().len(), 1);
    let second_recall = &runtime.last_memory_recall_receipts()[0];
    assert!(second_recall.input_generation > 0);
    assert!(second_recall
        .candidates
        .iter()
        .any(|candidate| candidate.target_searched > 0 || candidate.family_searched > 0));
    assert_ne!(second_recall.context_digest, first_memory_context_digest);
    assert_eq!(runtime.last_memory_update_receipts().len(), 1);
    assert_eq!(runtime.last_cognitive_context_digests().len(), 1);
    let second_context_digest = runtime.last_cognitive_context_digests()[0];
    assert_ne!(second_context_digest, first_context_digest);
    let second_patch = runtime
        .sealed_patches()
        .get(1)
        .expect("second sealed GPU patch");
    let second_credit = OutcomeCreditPacket::from_sealed_patch(second_patch)
        .expect("second sealed GPU patch yields outcome credit");
    let second_learning = runtime
        .last_learning_receipts()
        .first()
        .copied()
        .expect("second GPU learning receipt");
    assert_eq!(second_learning.sequence_id, second_credit.sequence_id());
    assert_eq!(
        second_learning.dispatch_generation,
        second_credit.dispatch_generation()
    );
    assert_eq!(
        second_learning.active_activation_side,
        second_credit.active_activation_side()
    );
    assert_ne!(second_credit.frame_digest(), first_credit.frame_digest());
    assert_eq!(
        second_learning.input_fast_generation,
        first_learning.output_fast_generation
    );
    assert!(second_learning.output_fast_generation > second_learning.input_fast_generation);
    assert_eq!(runtime.sealed_patches().len(), 2);
}
