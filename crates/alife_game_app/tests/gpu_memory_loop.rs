//! Real-hardware gate for candidate-conditioned memory in the production GPU loop.
#![cfg(feature = "gpu-runtime")]

use std::collections::BTreeMap;

use alife_core::{BrainScaleTier, OrganismId, Vec3f};
#[cfg(feature = "gpu-tests")]
use alife_core::{BrainTickStatus, SleepPhase};
use alife_game_app::{GpuLiveBrainRuntime, LiveBrainCausalStage};
use alife_gpu_backend::GpuClosedLoopBackend;
use alife_world::HeadlessScenarioBuilder;

#[test]
fn live_loop_recalls_before_gpu_selection_and_observes_after_learning() {
    let backend = GpuClosedLoopBackend::new_required().expect("required GPU adapter");
    let world = HeadlessScenarioBuilder::new(9_301)
        .agent("learner", OrganismId(1), Vec3f::ZERO)
        .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.8)
        .build()
        .unwrap();
    let mut runtime =
        GpuLiveBrainRuntime::new(backend, world, 9_301, BrainScaleTier::Nano512).unwrap();

    let summaries = runtime.tick().unwrap();
    let summary = &summaries[0];
    let patch = &runtime.sealed_patches()[0];
    let recall = &runtime.last_memory_recall_receipts()[0];
    let update = runtime.last_memory_update_receipts()[0];
    let topology = runtime.last_topology_observations()[0]
        .receipt()
        .expect("sealed patch reaches its organism topology sidecar");

    assert_eq!(recall.organism_id_raw, 1);
    assert_eq!(recall.input_generation, 0);
    assert_eq!(usize::from(recall.candidate_count), recall.candidates.len());
    assert_eq!(update.organism_id_raw, 1);
    assert_eq!(update.sealed_sequence_id, patch.header().sequence_id);
    assert_eq!(update.input_generation, recall.input_generation);
    assert_eq!(update.output_generation, recall.input_generation + 1);
    assert_eq!(summary.memory_updates, 1);
    assert_eq!(summary.topology_updates, 1);
    assert_eq!(topology.sealed_sequence_id, patch.header().sequence_id);
    assert_eq!(topology.organism_id_raw, 1);
    assert!(patch.decision().episodic_key().is_some());
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

    runtime.tick().unwrap();
    assert_eq!(runtime.last_memory_recall_receipts()[0].input_generation, 1);
}

#[test]
fn memory_sidecars_are_isolated_by_organism_not_gpu_handle() {
    let backend = GpuClosedLoopBackend::new_required().expect("required GPU adapter");
    let world = HeadlessScenarioBuilder::new(9_302)
        .agent("first", OrganismId(1), Vec3f::new(-1.0, 0.0, 0.0))
        .agent("second", OrganismId(2), Vec3f::new(1.0, 0.0, 0.0))
        .food("food", Vec3f::ZERO, 1.0)
        .build()
        .unwrap();
    let mut runtime =
        GpuLiveBrainRuntime::new(backend, world, 9_302, BrainScaleTier::Nano512).unwrap();

    let summaries = runtime.tick().unwrap();
    assert_eq!(summaries.len(), 2);
    assert_eq!(runtime.last_memory_recall_receipts().len(), 2);
    assert_eq!(runtime.last_memory_update_receipts().len(), 2);
    assert_eq!(runtime.last_topology_observations().len(), 2);

    let recalls = runtime
        .last_memory_recall_receipts()
        .iter()
        .map(|receipt| (receipt.organism_id_raw, receipt.input_generation))
        .collect::<BTreeMap<_, _>>();
    let updates = runtime
        .last_memory_update_receipts()
        .iter()
        .map(|receipt| (receipt.organism_id_raw, receipt.output_generation))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(recalls, BTreeMap::from([(1, 0), (2, 0)]));
    assert_eq!(updates, BTreeMap::from([(1, 1), (2, 1)]));
    assert!(summaries.iter().all(|summary| {
        summary.memory_updates == 1
            && summary.topology_updates == 1
            && summary
                .causal_stages
                .contains(&LiveBrainCausalStage::ObserveMemory)
            && summary
                .causal_stages
                .contains(&LiveBrainCausalStage::ObserveTopology)
            && !summary
                .causal_stages
                .contains(&LiveBrainCausalStage::CpuBrainTick)
    }));
}

#[cfg(feature = "gpu-tests")]
#[test]
fn one_memory_preparation_rejection_does_not_abort_an_unrelated_organism() {
    let backend = GpuClosedLoopBackend::new_required().expect("required GPU adapter");
    let world = HeadlessScenarioBuilder::new(9_303)
        .agent("rejected", OrganismId(1), Vec3f::new(-1.0, 0.0, 0.0))
        .agent("healthy", OrganismId(2), Vec3f::new(1.0, 0.0, 0.0))
        .food("food", Vec3f::ZERO, 1.0)
        .build()
        .unwrap();
    let mut runtime =
        GpuLiveBrainRuntime::new(backend, world, 9_303, BrainScaleTier::Nano512).unwrap();
    runtime.force_memory_preparation_failure_for_test(OrganismId(1));

    let summaries = runtime.tick().unwrap();
    let by_organism = summaries
        .iter()
        .map(|summary| (summary.organism_id.raw(), summary))
        .collect::<BTreeMap<_, _>>();

    assert_eq!(summaries.len(), 2);
    assert_eq!(
        by_organism[&1].status,
        BrainTickStatus::TerminalInvalidState
    );
    assert!(!by_organism[&1].patch_sealed);
    assert_eq!(by_organism[&2].status, BrainTickStatus::Normal);
    assert!(by_organism[&2].patch_sealed);
    assert_eq!(runtime.sealed_patches().len(), 1);
    assert_eq!(
        runtime.sealed_patches()[0].pre_action().organism_id.raw(),
        2
    );
    assert_eq!(runtime.last_memory_preparation_errors().len(), 1);
    assert_eq!(runtime.last_memory_preparation_errors()[0].0, OrganismId(1));
}

#[cfg(feature = "gpu-tests")]
#[test]
fn post_seal_learning_rejection_retains_credit_and_still_observes_memory_and_topology() {
    let backend = GpuClosedLoopBackend::new_required().expect("required GPU adapter");
    let organism_id = OrganismId(1);
    let world = HeadlessScenarioBuilder::new(9_304)
        .agent("learner", organism_id, Vec3f::ZERO)
        .food("food", Vec3f::new(1.0, 0.0, 0.0), 1.0)
        .build()
        .unwrap();
    let mut runtime =
        GpuLiveBrainRuntime::new(backend, world, 9_304, BrainScaleTier::Nano512).unwrap();
    runtime.force_learning_rejections_for_test(1);

    let first = runtime.tick().unwrap();
    assert!(first[0].patch_sealed);
    assert_eq!(first[0].learning_updates, 0);
    assert_eq!(first[0].memory_updates, 1);
    assert_eq!(first[0].topology_updates, 1);
    assert_eq!(runtime.sealed_patches().len(), 1);
    assert_eq!(runtime.last_memory_update_receipts().len(), 1);
    assert_eq!(runtime.last_topology_observations().len(), 1);
    assert!(runtime.last_topology_observations()[0].was_observed());
    assert_eq!(runtime.last_post_seal_learning_failures().len(), 1);
    assert!(runtime.last_post_seal_learning_failures()[0].retained_for_recovery);
    let retained = runtime
        .retained_learning_recovery(organism_id)
        .expect("sealed learning transaction retained");
    assert_eq!(retained.attempts, 0);
    assert_eq!(
        retained.sequence_id,
        runtime.sealed_patches()[0].header().sequence_id
    );

    let second = runtime.tick().unwrap();
    assert!(runtime.retained_learning_recovery(organism_id).is_none());
    assert_eq!(runtime.sealed_patches().len(), 2);
    assert_eq!(runtime.last_memory_update_receipts().len(), 1);
    assert_eq!(second[0].learning_updates, 1);
    assert_eq!(second[0].memory_updates, 1);
    assert_eq!(second[0].topology_updates, 1);
    assert_eq!(runtime.last_learning_receipts().len(), 2);
}

#[cfg(feature = "gpu-tests")]
#[test]
fn three_failed_retries_force_recovery_sleep_without_a_second_dispatch() {
    let backend = GpuClosedLoopBackend::new_required().expect("required GPU adapter");
    let organism_id = OrganismId(1);
    let world = HeadlessScenarioBuilder::new(9_305)
        .agent("learner", organism_id, Vec3f::ZERO)
        .food("food", Vec3f::new(1.0, 0.0, 0.0), 1.0)
        .build()
        .unwrap();
    let mut runtime =
        GpuLiveBrainRuntime::new(backend, world, 9_305, BrainScaleTier::Nano512).unwrap();
    runtime.force_learning_rejections_for_test(5);

    runtime.tick().unwrap();
    let dispatches_after_seal = runtime.completed_dispatch_count_for_test();
    for expected_attempts in 1..=3 {
        let summaries = runtime.tick().unwrap();
        assert!(!summaries[0].patch_sealed);
        assert_eq!(
            runtime.completed_dispatch_count_for_test(),
            dispatches_after_seal
        );
        assert_eq!(
            runtime
                .retained_learning_recovery(organism_id)
                .unwrap()
                .attempts,
            expected_attempts
        );
    }

    assert_eq!(
        runtime.sleep_state_for_test(organism_id).unwrap().phase,
        SleepPhase::ForcedRecoverySleep
    );
    assert_eq!(runtime.sealed_patches().len(), 1);

    let still_recovering = runtime.tick().unwrap();
    assert!(!still_recovering[0].patch_sealed);
    assert_eq!(
        runtime.sleep_state_for_test(organism_id).unwrap().phase,
        SleepPhase::ForcedRecoverySleep
    );
    assert_eq!(
        runtime
            .retained_learning_recovery(organism_id)
            .unwrap()
            .attempts,
        4
    );
    assert_eq!(
        runtime.completed_dispatch_count_for_test(),
        dispatches_after_seal
    );
}
