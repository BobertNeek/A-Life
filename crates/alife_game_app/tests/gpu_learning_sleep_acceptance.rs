#![cfg(feature = "gpu-runtime")]

use alife_core::{BrainCapacityClass, PolicyBackend};
use alife_game_app::{
    run_gpu_learning_sleep_acceptance, GpuLearningSleepAcceptanceOptions, GPU_SLICE_B_RAW,
};

#[test]
fn gpu_learning_sleep_receipt_proves_causal_learning() {
    for capacity in [
        BrainCapacityClass::n512(),
        BrainCapacityClass::n1024(),
        BrainCapacityClass::n2048(),
    ] {
        let receipt = run_gpu_learning_sleep_acceptance(GpuLearningSleepAcceptanceOptions {
            capacity,
            deterministic_seed: 4_202,
        })
        .unwrap();

        assert_eq!(receipt.header.slice_raw, GPU_SLICE_B_RAW);
        assert_eq!(receipt.header.status_raw, 1);
        assert_eq!(receipt.header.class_id_raw, capacity.id().raw());
        assert_eq!(receipt.header.capacity_digest, capacity.canonical_digest());
        assert_eq!(
            receipt.header.phenotype_manifest_digest,
            receipt.phenotype_manifest.manifest_digest
        );
        assert_eq!(
            receipt.header.artifact_digest,
            receipt.recompute_artifact_digest().unwrap()
        );
        assert_eq!(receipt.capacity_class_id, capacity.id());
        assert!(receipt.reward_target_delta > 0.0);
        assert!(receipt.pain_avoidance_delta > 0.0);
        assert!(receipt.unrelated_target_delta.abs() < receipt.reward_target_delta.abs());
        assert!(receipt.reward_target_delta > receipt.modulator_ablation_delta + receipt.tolerance);
        assert_eq!(receipt.consolidation_dispatches, 1);
        assert_eq!(receipt.genetic_digest_before, receipt.genetic_digest_after);
        assert!(receipt.post_wake_retained_delta > 0.0);
        assert!(receipt.replay_event_count > 0);
        assert!(receipt.replay_sample_count > 0);
        assert!(receipt.replay_induced_fast_l1 > receipt.tolerance);
        assert!(receipt.replay_vs_zero_sample_post_wake_delta > receipt.tolerance);
        assert_eq!(receipt.policy_backend, PolicyBackend::NeuralClosedLoopGpu);
        assert!(receipt.gpu_learning_dispatches > 0);
        assert!(receipt.restore.passed);
        assert_eq!(receipt.restore.checkpoint_phase_raw, 3);
        assert_eq!(receipt.restore.consolidation_state_raw, 3);
        assert_eq!(receipt.restore.expected_remaining_swaps, 1);
        assert_eq!(receipt.restore.actual_remaining_swaps, 1);
        assert_eq!(receipt.restore.duplicate_swaps, 0);
        assert_eq!(receipt.restore.actions_while_non_awake, 0);
        assert!(receipt.restore.reached_awake);
        assert!(receipt.restore.retained_target_delta > receipt.restore.tolerance);
    }
}
