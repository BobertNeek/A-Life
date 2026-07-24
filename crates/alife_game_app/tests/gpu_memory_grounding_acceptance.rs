#![cfg(feature = "gpu-runtime")]

use std::sync::OnceLock;

use alife_core::{BrainCapacityClass, PolicyBackend, SensorProfile};
use alife_game_app::{
    run_gpu_memory_grounding_acceptance, GpuMemoryGroundingAcceptanceOptions,
    GpuMemoryGroundingEvidenceReceipt,
};

fn grounded_options() -> GpuMemoryGroundingAcceptanceOptions {
    GpuMemoryGroundingAcceptanceOptions {
        capacity: BrainCapacityClass::n512(),
        requested_ticks: 10_240,
        deterministic_seed: 4_303,
        sensor_profile: SensorProfile::GroundedObjectSlotsV1,
    }
}

fn grounded_receipt() -> &'static GpuMemoryGroundingEvidenceReceipt {
    static RECEIPT: OnceLock<GpuMemoryGroundingEvidenceReceipt> = OnceLock::new();
    RECEIPT.get_or_init(|| run_gpu_memory_grounding_acceptance(grounded_options()).unwrap())
}

#[test]
fn slice_c_receipt_proves_candidate_specific_grounded_memory() {
    let receipt = grounded_receipt();
    assert_eq!(
        receipt.sensor_profile.profile().unwrap(),
        SensorProfile::GroundedObjectSlotsV1
    );
    let saturation = receipt.capacity_saturation.as_ref().unwrap();
    assert_eq!(saturation.grounded_semantic_label_channels_nonzero, 0);
    assert!(receipt.poisoned_ingest_logit_after < receipt.poisoned_ingest_logit_before);
    assert!(receipt.poisoned_avoid_logit_after > receipt.poisoned_avoid_logit_before);
    assert!(receipt.safe_ingest_delta.abs() < receipt.poisoned_ingest_delta.abs());
    assert!(receipt.cyan_avoid_target_latent[2] > 0.0);
    assert_eq!(
        receipt.cyan_ingest_target_latent,
        receipt.cyan_avoid_target_latent
    );
    assert!(receipt.cyan_ingest_family_value[2] > 0.0);
    assert_eq!(receipt.cyan_avoid_family_value, [0.0; 4]);
    assert_eq!(receipt.amber_target_latent, [0.0; 8]);
    assert_eq!(
        receipt.memory_enabled.recurrent_activation_digest,
        receipt.memory_ablated.recurrent_activation_digest
    );
    assert_ne!(
        receipt.post_learning_selection,
        receipt.poisoned_ingest_candidate
    );
    assert_ne!(
        receipt.memory_enabled.selected_candidate,
        receipt.memory_ablated.selected_candidate
    );
    assert_eq!(receipt.policy_backend, PolicyBackend::NeuralClosedLoopGpu);
    assert_eq!(receipt.gpu_selection_count, receipt.completed_waking_ticks);
    assert_eq!(
        receipt.memory_enabled.fast_weight_digest,
        receipt.memory_ablated.fast_weight_digest
    );
    assert_eq!(
        receipt.memory_enabled.phenotype_hash,
        receipt.memory_ablated.phenotype_hash
    );
    assert!(
        receipt.memory_enabled.poisoned_ingest_delta
            < receipt.memory_ablated.poisoned_ingest_delta - receipt.tolerance
    );
    assert!(
        (receipt.memory_enabled.safe_ingest_delta - receipt.memory_ablated.safe_ingest_delta).abs()
            <= receipt.tolerance
    );
}

#[test]
fn slice_c_soak_degrades_without_terminal_capacity_failure() {
    let receipt = grounded_receipt();
    assert_eq!(receipt.completed_ticks, 10_240);
    let saturation = receipt.capacity_saturation.as_ref().unwrap();
    assert!(saturation.memory_records <= saturation.memory_capacity);
    assert!(saturation.tracked_object_records <= saturation.tracked_object_capacity);
    assert!(saturation.tracked_object_evictions > 0);
    assert_eq!(saturation.tracked_object_id_reuse_count, 0);
    assert!(saturation.topology_capacity.contains(
        saturation.topology_counts,
        saturation.max_observed_bindings_per_kind,
    ));
    assert!(saturation.memory_merges + saturation.memory_evictions > 0);
    assert!(saturation.topology_degradations > 0);
    assert_eq!(saturation.terminal_capacity_errors, 0);
    assert!(receipt.compact_readback_bytes <= 64);
}

#[test]
fn privileged_and_grounded_options_use_distinct_profile_qualified_artifacts() {
    let grounded = grounded_options();
    let privileged = GpuMemoryGroundingAcceptanceOptions {
        sensor_profile: SensorProfile::PrivilegedAffordanceV1,
        requested_ticks: 64,
        ..grounded
    };
    assert_ne!(
        grounded.artifact_path().unwrap(),
        privileged.artifact_path().unwrap()
    );
    assert_ne!(
        grounded.aggregate_key().unwrap(),
        privileged.aggregate_key().unwrap()
    );
    assert_eq!(
        grounded.artifact_slug().unwrap(),
        "gpu-memory-grounding-slice-c-grounded-object-slots-v1-n512",
    );
    let receipt = run_gpu_memory_grounding_acceptance(privileged).unwrap();
    assert_eq!(receipt.completed_ticks, 64);
    assert!(receipt.capacity_saturation.is_none());
}
