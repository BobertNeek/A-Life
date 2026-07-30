use std::path::PathBuf;

use alife_core::PolicyBackend;
use alife_tools::ei0_exit_gate::run_ei0_lifecycle_gate;

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
