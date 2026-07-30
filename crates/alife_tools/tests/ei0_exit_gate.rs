use std::{fs, path::PathBuf};

#[cfg(feature = "gpu-tests")]
use alife_core::PolicyBackend;
#[cfg(feature = "gpu-tests")]
use alife_tools::ei0_exit_gate::{
    run_ei0_exit_gate, run_ei0_lifecycle_gate, write_ei0_exit_gate_report,
};
use alife_tools::ei0_exit_gate::{
    run_ei0_exit_gate_and_write, Ei0EvidenceStatus, Ei0ExitGateReport,
};

fn temp_root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "alife-ei0-exit-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

#[cfg(feature = "gpu-tests")]
#[test]
fn lifecycle_receipt_proves_restored_two_lane_multi_generation_population() {
    let root = temp_root("lifecycle");
    let evidence = run_ei0_lifecycle_gate(&root).unwrap();
    let report = &evidence.report;

    assert_eq!(report.schema_version, 2);
    assert_eq!(report.founder_count, 8);
    assert_eq!(report.restored_population_count, 8);
    assert!(report.post_restore_ticks >= 128);
    assert_eq!(report.live_population_count, 14);
    assert_eq!(report.generation_count, 3);
    assert_eq!(report.lanes.len(), 2);
    assert!(report.run_observed);
    assert!(report.portable_save_round_trip);
    assert!(report.tampered_save_rejected);
    assert!(report.tampered_provenance_rejected);
    assert_eq!(report.archive_birth_manifest_count, 14);
    assert!(report.lineage_compare_passed);
    assert!(report.no_lifetime_state_inherited);
    assert!(report.player_directed_wild_breeding_rejected);
    assert!(report.creature_directed_managed_breeding_rejected);

    for lane in &report.lanes {
        assert_eq!(lane.births.len(), 3);
        assert_eq!(lane.births[0].generation, 1);
        assert_eq!(lane.births[1].generation, 1);
        assert_eq!(lane.births[2].generation, 2);
        assert!(lane.births.iter().all(|birth| {
            birth.ordinary_birth
                && birth.parent_genome_ids.len() == 2
                && birth.conception_seed != 0
                && birth.foundation_id != 0
                && birth.foundation_version != 0
                && birth.compatibility_family_id != 0
                && birth.cognition_policy == PolicyBackend::NeuralClosedLoopGpu
                && birth.child_lifetime.memory_records == 0
                && birth.child_lifetime.lifetime_weights == 0
        }));
        for birth in &lane.births[..2] {
            assert!(birth.first_parent_lifetime.memory_records > 0);
            assert!(birth.first_parent_lifetime.lifetime_weights > 0);
            assert!(birth.second_parent_lifetime.memory_records > 0);
            assert!(birth.second_parent_lifetime.lifetime_weights > 0);
            assert_ne!(
                birth.first_parent_lifetime.state_digest,
                birth.second_parent_lifetime.state_digest
            );
        }
    }
    let wild = &report.lanes[0];
    assert!(wild.births.iter().all(|birth| {
        birth.gpu_intent_sequence_id.is_some() && birth.gpu_selected_mate.is_some()
    }));
    let managed = &report.lanes[1];
    assert!(managed
        .births
        .iter()
        .all(|birth| birth.gpu_intent_sequence_id.is_none()));

    assert_eq!(evidence.final_generation_genomes.len(), 2);
    assert_eq!(report.evidence_digests.source_genomes.len(), 14);
    assert!(report.evidence_digests.foundation_weights.is_some());
    assert!(report.evidence_digests.shader_bundle.is_some());
    assert!(report.evidence_digests.portable_save.is_some());
    assert_eq!(report.evidence_digests.archive_manifests.len(), 14);
    assert_eq!(report.evidence_digests.archive_composite_assets.len(), 14);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn operational_failure_writes_a_schema_valid_typed_partial_report_before_erroring() {
    let root = temp_root("partial");
    fs::create_dir_all(&root).unwrap();
    let blocked_evidence_root = root.join("not-a-directory");
    fs::write(&blocked_evidence_root, b"block directory creation").unwrap();
    let output = root.join("partial.json");

    let result = run_ei0_exit_gate_and_write(&blocked_evidence_root, &output);

    assert!(result.is_err());
    let report: Ei0ExitGateReport =
        serde_json::from_str(&fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(report.schema_version, 2);
    assert!(!report.verdict.era0_exit_gate_passed);
    assert!(report.operational_error.is_some());
    assert!(report.lifecycle.is_none());
    assert_eq!(report.clauses.run.status, Ei0EvidenceStatus::Unavailable);
    assert_eq!(
        report.clauses.no_hidden_policy_control.status,
        Ei0EvidenceStatus::Unavailable
    );

    fs::remove_dir_all(root).unwrap();
}

#[cfg(feature = "gpu-tests")]
#[test]
fn full_report_proves_the_exit_gate_without_promoting_the_heuristic_baseline() {
    let root = temp_root("full-report");
    let path = root.join("ei0_exit_gate_report.json");
    let report = run_ei0_exit_gate(&root).unwrap();
    write_ei0_exit_gate_report(&path, &report).unwrap();
    let restored: Ei0ExitGateReport =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();

    assert!(restored.verdict.era0_exit_gate_passed);
    assert!(!restored.verdict.era1_promotion_evaluated);
    assert!(restored.clauses.all_passed());
    assert!(restored.operational_error.is_none());
    let lifecycle = restored.lifecycle.as_ref().unwrap();
    assert_eq!(restored.gpu_tests.len(), 2);
    for (lane, gpu) in lifecycle.lanes.iter().zip(&restored.gpu_tests) {
        assert_eq!(gpu.source_creature_genome_id, lane.births[2].genome_id);
        assert_eq!(gpu.parent_genome_ids, lane.births[2].parent_genome_ids);
        assert_eq!(gpu.completed_challenges, 15);
        assert_eq!(gpu.gpu_dispatches, gpu.sealed_outcomes);
        assert!(gpu.sleep_consolidations >= 1);
        assert_eq!(gpu.policy_backend, PolicyBackend::NeuralClosedLoopGpu);
    }

    let baseline = restored.heuristic_baseline.as_ref().unwrap();
    assert_eq!(baseline.source_backend, "HeuristicBaseline");
    assert!(!baseline.promotion_eligible);
    assert_eq!(baseline.hidden_promotion_trials, 0);
    assert!(baseline.unknown_measures_preserved);
    assert_eq!(baseline.unknown_measures.len(), 9);
    assert_eq!(restored.evidence_digests, lifecycle.evidence_digests);

    fs::remove_dir_all(root).unwrap();
}
