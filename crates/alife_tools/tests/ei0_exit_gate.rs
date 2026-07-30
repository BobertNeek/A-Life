use std::path::PathBuf;

use alife_core::PolicyBackend;
use alife_tools::ei0_exit_gate::run_ei0_lifecycle_gate;
#[cfg(feature = "gpu-tests")]
use alife_tools::ei0_exit_gate::{
    run_ei0_exit_gate, write_ei0_exit_gate_report, Ei0ExitGateReport,
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

#[test]
fn lifecycle_receipt_proves_two_lane_multi_generation_population() {
    let root = temp_root("lifecycle");
    let evidence = run_ei0_lifecycle_gate(&root).unwrap();
    let report = &evidence.report;

    assert_eq!(report.schema_version, 1);
    assert_eq!(report.founder_count, 8);
    assert_eq!(report.live_population_count, 14);
    assert_eq!(report.generation_count, 3);
    assert_eq!(report.lanes.len(), 2);
    assert!(report.run_observed);
    assert!(report.portable_save_round_trip);
    assert!(report.tampered_save_rejected);
    assert!(report.tampered_provenance_rejected);
    assert_eq!(report.restored_population_count, 14);
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
        }));
    }

    assert_eq!(evidence.final_generation_genomes.len(), 2);
    assert!(evidence.final_generation_genomes.iter().all(|genome| {
        genome.parent_genome_ids.len() == 2
            && genome.provenance.ordinary_birth
            && genome.provenance.conception_seed == genome.conception_seed
    }));

    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(feature = "gpu-tests")]
#[test]
fn full_report_proves_the_exit_gate_without_promoting_the_heuristic_baseline() {
    let root = temp_root("full-report");
    let path = root.join("ei0_exit_gate_report.json");
    let report = run_ei0_exit_gate(&root).unwrap();
    write_ei0_exit_gate_report(&path, &report).unwrap();
    let restored: Ei0ExitGateReport =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

    assert!(restored.verdict.era0_exit_gate_passed);
    assert!(!restored.verdict.era1_promotion_evaluated);
    assert!(restored.clauses.run);
    assert!(restored.clauses.observe);
    assert!(restored.clauses.save_load);
    assert!(restored.clauses.wild_breed);
    assert!(restored.clauses.managed_breed);
    assert!(restored.clauses.test);
    assert!(restored.clauses.archive);
    assert!(restored.clauses.compare);
    assert!(restored.clauses.stable_multi_generation_population);
    assert!(restored.clauses.gpu_policy_identity);
    assert!(restored.clauses.no_hidden_policy_control);

    assert_eq!(restored.gpu_tests.len(), 2);
    for (lane, gpu) in restored.lifecycle.lanes.iter().zip(&restored.gpu_tests) {
        assert_eq!(gpu.source_creature_genome_id, lane.births[2].genome_id);
        assert_eq!(gpu.parent_genome_ids, lane.births[2].parent_genome_ids);
        assert_eq!(gpu.completed_challenges, 15);
        assert_eq!(gpu.gpu_dispatches, gpu.sealed_outcomes);
        assert!(gpu.sleep_consolidations >= 1);
        assert_eq!(gpu.policy_backend, PolicyBackend::NeuralClosedLoopGpu);
    }

    assert_eq!(
        restored.heuristic_baseline.source_backend,
        "HeuristicBaseline"
    );
    assert!(!restored.heuristic_baseline.promotion_eligible);
    assert_eq!(restored.heuristic_baseline.hidden_promotion_trials, 0);
    assert!(restored.heuristic_baseline.unknown_measures_preserved);
    assert_eq!(restored.heuristic_baseline.unknown_measures.len(), 9);

    std::fs::remove_dir_all(root).unwrap();
}
