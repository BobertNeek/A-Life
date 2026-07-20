#![cfg(feature = "gpu-tests")]

use std::{process::Command, sync::Mutex};

use alife_core::{BrainCapacityClass, PolicyBackend, SensorProfile, Validate};
use alife_game_app::{
    run_gpu_closed_loop_soak, GpuClosedLoopSoakOptions, GpuClosedLoopSoakReceipt,
    GPU_EVIDENCE_PASSING_STATUS_RAW, GPU_SLICE_D_RAW,
};

const SOAK_CHILD_TEST_ENV: &str = "ALIFE_GPU_SOAK_CHILD_TEST";
static SOAK_CHILD_PROCESS_LOCK: Mutex<()> = Mutex::new(());

fn run_isolated_soak(
    test_name: &'static str,
    capacity: BrainCapacityClass,
    sensor_profile: SensorProfile,
) -> Option<GpuClosedLoopSoakReceipt> {
    if std::env::var_os(SOAK_CHILD_TEST_ENV).as_deref() == Some(test_name.as_ref()) {
        return Some(
            run_gpu_closed_loop_soak(GpuClosedLoopSoakOptions {
                capacity,
                sensor_profile,
                completed_ticks: 10_240,
                deterministic_seed: 4_505,
            })
            .unwrap(),
        );
    }

    let _guard = SOAK_CHILD_PROCESS_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let status = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg(test_name)
        .arg("--nocapture")
        .env(SOAK_CHILD_TEST_ENV, test_name)
        .status()
        .unwrap();
    assert!(status.success(), "isolated GPU soak `{test_name}` failed");
    None
}

#[test]
fn ten_thousand_tick_receipt_proves_every_bound_and_gpu_authority() {
    let Some(receipt) = run_isolated_soak(
        "ten_thousand_tick_receipt_proves_every_bound_and_gpu_authority",
        BrainCapacityClass::n512(),
        SensorProfile::GroundedObjectSlotsV1,
    ) else {
        return;
    };

    receipt.validate_in_memory().unwrap();
    assert_eq!(receipt.header.common.slice_raw, GPU_SLICE_D_RAW);
    assert_eq!(
        receipt.header.common.status_raw,
        GPU_EVIDENCE_PASSING_STATUS_RAW
    );
    assert_eq!(
        receipt.header.common.phenotype_manifest_digest,
        receipt.phenotype_manifest.manifest_digest,
    );
    assert_eq!(
        receipt.header.common.artifact_digest,
        receipt.recompute_artifact_digest().unwrap(),
    );
    assert_eq!(receipt.policy_backend, PolicyBackend::NeuralClosedLoopGpu);
    assert_eq!(receipt.completed_ticks, 10_240);
    assert!(receipt.truncation.max_candidates <= receipt.capacity.execution().max_candidates());
    assert!(receipt.truncation.max_object_slots <= receipt.capacity.execution().max_object_slots());
    assert!(
        receipt.truncation.max_decoder_input_lanes
            <= receipt.capacity.execution().max_decoder_input_lanes()
    );
    assert!(receipt.memory.final_record_count <= receipt.memory.capacity);
    assert!(receipt.memory.capacity > 64);
    assert!(receipt.topology.capacity.contains(
        receipt.topology.final_counts,
        receipt.topology.max_observed_bindings_per_kind,
    ));
    assert!(receipt
        .route_budgets
        .iter()
        .all(|route| route.within_ceiling()));
    assert!(receipt.global_budget.within(receipt.capacity.execution()));
    assert!(
        receipt.admission.peak_logical_committed_bytes <= receipt.admission.logical_budget_bytes
    );
    assert!(
        receipt.admission.peak_physical_allocated_bytes <= receipt.admission.physical_ceiling_bytes
    );
    assert_eq!(
        receipt.admission.post_warmup_physical_min_bytes,
        receipt.admission.post_warmup_physical_max_bytes
    );
    assert_eq!(
        receipt.admission.post_warmup_logical_min_bytes,
        receipt.admission.post_warmup_logical_max_bytes
    );
    assert!(receipt.process_memory.rss_high_water_bytes <= receipt.process_memory.rss_budget_bytes);
    assert!(
        receipt.process_memory.post_warmup_growth_bytes
            <= receipt.process_memory.growth_envelope_bytes
    );
    assert!(receipt.gpu_selections > 0);
    assert!(receipt.activity.learning_commits > 0);
    assert!(receipt.save_restore.sleep_cycles > 0);
    assert!(receipt
        .save_restore
        .restore_receipts
        .iter()
        .all(|restore| restore.passed));
    assert!(receipt
        .activity
        .raw_dispatch_samples
        .iter()
        .all(|sample| sample.bindings_match()));
    assert_eq!(
        receipt.activity.raw_dispatch_samples.len() as u64,
        receipt.authoritative_gpu_dispatches
    );
    assert_eq!(receipt.admission.raw_samples.len(), 157);
    assert_eq!(receipt.process_memory.raw_samples.len(), 157);
    assert_eq!(
        receipt.replay.raw_comparisons.len() as u64,
        receipt.replay.compared_dispatches
    );
    assert_eq!(receipt.policy_switch.switch_count, 0);
    assert_eq!(receipt.terminal_capacity_errors, 0);
    assert!(receipt.memory.merges + receipt.memory.evictions > 0);
    assert!(receipt.memory.compactions > 0);
    assert!(receipt.topology.degradations > 0);
    assert!(receipt.truncation.candidate_truncations > 0);
    assert!(receipt.truncation.object_slot_truncations > 0);
    assert!(receipt.truncation.memory_context_truncations > 0);
    assert!(receipt.truncation.topology_binding_truncations > 0);
    assert!(receipt.truncation.compact_readback_bytes <= 64);
    assert!(receipt.replay.passed);
    assert!(receipt
        .save_restore
        .migration_receipts
        .iter()
        .all(|row| row.passed));
    receipt.sensor_profile.validate_contract().unwrap();
}

#[test]
fn privileged_soak_saturates_topology_bindings_through_ordinary_perception() {
    let Some(receipt) = run_isolated_soak(
        "privileged_soak_saturates_topology_bindings_through_ordinary_perception",
        BrainCapacityClass::n512(),
        SensorProfile::PrivilegedAffordanceV1,
    ) else {
        return;
    };

    receipt.validate_in_memory().unwrap();
    assert!(receipt.truncation.topology_binding_truncations > 0);
    assert!(receipt.topology.max_observed_bindings_per_kind > 0);
}

#[test]
fn n1024_soak_enters_canonical_sleep_before_activity_exhaustion() {
    let Some(receipt) = run_isolated_soak(
        "n1024_soak_enters_canonical_sleep_before_activity_exhaustion",
        BrainCapacityClass::n1024(),
        SensorProfile::PrivilegedAffordanceV1,
    ) else {
        return;
    };

    receipt.validate_in_memory().unwrap();
    assert!(receipt.save_restore.sleep_cycles > 0);
    assert!(receipt.authoritative_gpu_dispatches > 0);
}

#[test]
fn n2048_soak_releases_completed_sleep_payloads_and_stays_rss_bounded() {
    let Some(receipt) = run_isolated_soak(
        "n2048_soak_releases_completed_sleep_payloads_and_stays_rss_bounded",
        BrainCapacityClass::n2048(),
        SensorProfile::PrivilegedAffordanceV1,
    ) else {
        return;
    };

    receipt.validate_in_memory().unwrap();
    assert!(
        receipt.process_memory.post_warmup_growth_bytes
            <= receipt.process_memory.growth_envelope_bytes
    );
    assert!(receipt.save_restore.sleep_cycles > 0);
    assert!(receipt.authoritative_gpu_dispatches > 0);
}
