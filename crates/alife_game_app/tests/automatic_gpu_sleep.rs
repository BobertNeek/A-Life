#![cfg(feature = "gpu-tests")]

use alife_core::{
    BrainScaleTier, BrainTickStatus, ConsolidationState, DriveSnapshot, EndocrineSnapshot,
    HomeostaticSnapshot, OrganismId, SleepPhase, Tick, Vec3f,
};
use alife_game_app::GpuLiveBrainRuntime;
use alife_gpu_backend::GpuClosedLoopBackend;
use alife_world::HeadlessScenarioBuilder;

#[test]
fn live_gpu_sleep_submits_commits_wakes_and_retains_learning() {
    let organism_id = OrganismId(1);
    let backend =
        GpuClosedLoopBackend::new_required(alife_gpu_backend::GpuRuntimeProfile::production_v1())
            .expect("required Vulkan adapter");
    let world = HeadlessScenarioBuilder::new(7_701)
        .agent("sleeper", organism_id, Vec3f::ZERO)
        .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.9)
        .hazard("hazard", Vec3f::new(-2.0, 0.0, 0.0), 0.7)
        .build()
        .unwrap();
    let mut runtime =
        GpuLiveBrainRuntime::new(backend, world, 7_701, BrainScaleTier::Nano512).unwrap();

    let learned = runtime.tick().unwrap();
    assert_eq!(learned.len(), 1);
    assert!(learned[0].patch_sealed);
    assert!(runtime
        .last_learning_receipts()
        .iter()
        .any(|receipt| receipt.fast_weights_changed > 0));
    let learning_before_sleep = runtime.learning_state_for_test(organism_id).unwrap();
    let fast_before_sleep = runtime.active_fast_weights_for_test(organism_id).unwrap();
    assert!(fast_before_sleep.iter().any(|value| *value != 0.0));

    let sleep_tick = runtime.world_tick_for_test();
    let mut drives = DriveSnapshot::baseline();
    drives.fatigue = 0.99;
    let mut hormones = EndocrineSnapshot::baseline();
    hormones.sleep_pressure = 0.99;
    runtime
        .set_homeostasis_for_test(
            organism_id,
            HomeostaticSnapshot::new(sleep_tick, drives, hormones).unwrap(),
        )
        .unwrap();

    let dispatches_before_sleep = runtime.completed_dispatch_count_for_test();
    let mut saw_submitted = false;
    let mut saw_completed = false;
    let mut woke = false;
    let mut memory_compactions = Vec::new();
    for _ in 0..64 {
        let summaries = runtime.tick().unwrap();
        memory_compactions.extend_from_slice(runtime.last_memory_compaction_receipts());
        let state = runtime.sleep_state_for_test(organism_id).unwrap();
        saw_submitted |= matches!(state.consolidation, ConsolidationState::Submitted { .. });
        saw_completed |= matches!(state.consolidation, ConsolidationState::Completed { .. });
        if state.phase != SleepPhase::Awake || state.last_consolidated_cycle_id == 1 {
            assert_eq!(summaries[0].status, BrainTickStatus::SafeIdle);
            assert_eq!(summaries[0].selected_action_id, None);
            assert!(!summaries[0].patch_sealed);
        }
        if state.phase == SleepPhase::Awake && state.last_consolidated_cycle_id == 1 {
            woke = true;
            break;
        }
    }

    assert!(saw_submitted);
    assert!(saw_completed);
    assert!(woke);
    assert_eq!(memory_compactions.len(), 1);
    assert_eq!(
        memory_compactions[0].identity.organism_id_raw,
        organism_id.raw()
    );
    assert_eq!(memory_compactions[0].identity.cycle_id, 1);
    let memory_checkpoint = runtime
        .memory_compaction_checkpoint(organism_id)
        .expect("organism-owned memory checkpoint");
    assert_eq!(memory_checkpoint.last_committed_cycle_id, Some(1));
    assert!(matches!(
        memory_checkpoint.phase,
        alife_core::MemoryCompactionPhase::Committed { cycle_id: 1, .. }
    ));
    assert_eq!(
        runtime.completed_dispatch_count_for_test(),
        dispatches_before_sleep
    );
    let learning_after_sleep = runtime.learning_state_for_test(organism_id).unwrap();
    assert_eq!(
        learning_after_sleep.active_weight_generation,
        learning_before_sleep.active_weight_generation + 1
    );
    assert_ne!(
        learning_after_sleep.active_weight_bank,
        learning_before_sleep.active_weight_bank
    );
    assert!(runtime
        .active_lifetime_weights_for_test(organism_id)
        .unwrap()
        .iter()
        .any(|value| *value != 0.0));

    let resumed = runtime.tick().unwrap();
    assert_eq!(resumed[0].status, BrainTickStatus::Normal);
    assert!(resumed[0].patch_sealed);
    assert_eq!(
        runtime.completed_dispatch_count_for_test(),
        dispatches_before_sleep + 1
    );
    assert!(runtime.world_tick_for_test().raw() > Tick::ZERO.raw());
}
