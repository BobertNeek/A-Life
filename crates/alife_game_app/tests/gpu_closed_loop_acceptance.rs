#![cfg(feature = "gpu-runtime")]

use alife_core::{BrainCapacityClass, PolicyBackend, SensorProfile};
use alife_game_app::{run_gpu_closed_loop_acceptance, GpuClosedLoopAcceptanceOptions};

fn test_options() -> GpuClosedLoopAcceptanceOptions {
    GpuClosedLoopAcceptanceOptions {
        capacity: BrainCapacityClass::n512(),
        requested_ticks: 4,
        deterministic_seed: 4_101,
        sensor_profile: SensorProfile::PrivilegedAffordanceV1,
    }
}

#[test]
fn gpu_closed_loop_slice_a_receipt_is_authoritative() {
    let receipt = run_gpu_closed_loop_acceptance(test_options()).unwrap();

    assert_eq!(receipt.header.slice_raw, 1);
    assert_eq!(receipt.header.profile_id_raw, 0);
    assert_eq!(receipt.header.profile_schema, 0);
    assert_eq!(receipt.header.status_raw, 1);
    assert_eq!(receipt.header.class_id_raw, receipt.capacity_class_id.raw());
    assert_eq!(
        receipt.header.capacity_digest,
        receipt.capacity.canonical_digest()
    );
    assert_eq!(
        receipt.header.phenotype_manifest_digest,
        receipt.phenotype_manifest.manifest_digest
    );
    assert_eq!(
        receipt.header.artifact_digest,
        receipt.recompute_artifact_digest().unwrap()
    );
    assert_eq!(receipt.backend_api, "vulkan");
    assert!(receipt.authoritative);
    assert_eq!(receipt.policy_backend, PolicyBackend::NeuralClosedLoopGpu);
    assert_eq!(
        receipt.neural_dispatch_count,
        u64::from(receipt.requested_ticks)
    );
    assert_eq!(
        receipt.gpu_selection_count,
        u64::from(receipt.requested_ticks)
    );
    assert_eq!(
        receipt.sealed_patch_count,
        u64::from(receipt.requested_ticks)
    );
    assert_eq!(
        receipt.selection_trace.len(),
        receipt.requested_ticks as usize
    );
    assert!(receipt.compact_readback_bytes <= 64);
    assert!(receipt.active_tiles > 0);
    assert!(receipt.active_synapses > 0);
    assert!(receipt.replay.passed);
    assert!(receipt.replay.max_abs_error <= receipt.replay.tolerance);
}

#[test]
fn gpu_closed_loop_slice_a_receipt_round_trips_and_rejects_tampering() {
    let receipt = run_gpu_closed_loop_acceptance(test_options()).unwrap();
    let encoded = serde_json::to_vec(&receipt).unwrap();
    let decoded: alife_game_app::GpuSliceAAcceptanceReceipt =
        serde_json::from_slice(&encoded).unwrap();

    assert_eq!(decoded, receipt);
    decoded.validate_in_memory().unwrap();

    let mut digest_tamper = decoded.clone();
    digest_tamper.header.artifact_digest[0] ^= 1;
    assert!(digest_tamper.validate_in_memory().is_err());

    let mut trace_tamper = decoded.clone();
    trace_tamper.selection_trace[0].candidate_index ^= 1;
    trace_tamper.header.artifact_digest = trace_tamper.recompute_artifact_digest().unwrap();
    assert!(trace_tamper.validate_in_memory().is_err());

    let mut non_finite = decoded;
    non_finite.replay.max_abs_error = f32::NAN;
    assert!(non_finite.recompute_artifact_digest().is_err());
}

#[test]
fn gpu_closed_loop_slice_a_options_reject_zero_ticks_before_gpu_creation() {
    let mut options = test_options();
    options.requested_ticks = 0;

    assert!(run_gpu_closed_loop_acceptance(options).is_err());
}
