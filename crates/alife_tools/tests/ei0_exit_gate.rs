use std::{fs, path::PathBuf, process::Command};

use alife_core::{OrganismId, Tick};
use alife_tools::ei0_exit_gate::{
    validate_committed_ei0_exit_gate_report, Ei0ExitGateError, Ei0ExitGateReport,
};
use alife_world::{HabitatActor, HabitatMode};

fn refresh_binding_digests(report: &mut Ei0ExitGateReport) {
    let lifecycle = report.lifecycle.as_ref().unwrap();
    let binding = report.artifact_binding.as_mut().unwrap();
    binding.causal_birth_receipts_digest = format!(
        "blake3-256:{}",
        blake3::hash(&serde_json::to_vec(&lifecycle.lanes).unwrap()).to_hex()
    );
    binding.gpu_receipts_digest = format!(
        "blake3-256:{}",
        blake3::hash(&serde_json::to_vec(&report.gpu_tests).unwrap()).to_hex()
    );
}

#[test]
fn committed_historical_report_recomputes_bound_source_and_causal_evidence() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("reports")
        .join("ei0_exit_gate_report.json");
    let report: Ei0ExitGateReport =
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();

    assert!(report
        .lifecycle
        .as_ref()
        .unwrap()
        .population_genomes
        .iter()
        .all(|genome| genome.is_historical_v1()));
    validate_committed_ei0_exit_gate_report(&report).unwrap();
    let binding = report.artifact_binding.as_ref().unwrap();
    assert_eq!(binding.adapter_name, "NVIDIA GeForce RTX 3050");
    assert_eq!(binding.backend_api, "vulkan");

    let mut tampered_child_phenotype = report.clone();
    let birth = tampered_child_phenotype
        .lifecycle
        .as_mut()
        .unwrap()
        .lanes
        .iter_mut()
        .find(|lane| lane.mode == HabitatMode::Wild)
        .unwrap()
        .births
        .first_mut()
        .unwrap();
    birth.child_phenotype_hash.0[0] ^= 1;
    refresh_binding_digests(&mut tampered_child_phenotype);
    assert!(matches!(
        validate_committed_ei0_exit_gate_report(&tampered_child_phenotype),
        Err(Ei0ExitGateError::Evidence(
            "historical report differs from the committed baseline artifact"
        ))
    ));

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(std::path::Path::parent)
        .unwrap();
    let arbitrary_old_commit = Command::new("git")
        .current_dir(workspace_root)
        .args(["rev-parse", "HEAD^"])
        .output()
        .unwrap();
    assert!(arbitrary_old_commit.status.success());
    let mut arbitrary_old_report = report.clone();
    arbitrary_old_report
        .artifact_binding
        .as_mut()
        .unwrap()
        .producing_source_commit = String::from_utf8(arbitrary_old_commit.stdout)
        .unwrap()
        .trim()
        .to_string();
    refresh_binding_digests(&mut arbitrary_old_report);
    assert!(matches!(
        validate_committed_ei0_exit_gate_report(&arbitrary_old_report),
        Err(Ei0ExitGateError::Evidence(
            "non-current report is not the exact historical baseline"
        ))
    ));

    let mut wrong_hardware = report.clone();
    wrong_hardware
        .artifact_binding
        .as_mut()
        .unwrap()
        .adapter_name = "NVIDIA GeForce RTX 4090".to_string();
    for gpu in &mut wrong_hardware.gpu_tests {
        gpu.adapter_name = "NVIDIA GeForce RTX 4090".to_string();
    }
    refresh_binding_digests(&mut wrong_hardware);
    assert!(validate_committed_ei0_exit_gate_report(&wrong_hardware).is_err());

    let mut wrong_mate = report.clone();
    let birth = wrong_mate
        .lifecycle
        .as_mut()
        .unwrap()
        .lanes
        .iter_mut()
        .find(|lane| lane.mode == HabitatMode::Wild)
        .unwrap()
        .births
        .first_mut()
        .unwrap();
    birth.gpu_selected_mate = Some(birth.breeding_receipt.first_parent);
    refresh_binding_digests(&mut wrong_mate);
    assert!(
        validate_committed_ei0_exit_gate_report(&wrong_mate).is_err(),
        "a format-stable causal mate-link tamper must be rejected"
    );

    let mut wrong_tick = report.clone();
    let birth = wrong_tick
        .lifecycle
        .as_mut()
        .unwrap()
        .lanes
        .iter_mut()
        .find(|lane| lane.mode == HabitatMode::Wild)
        .unwrap()
        .births
        .first_mut()
        .unwrap();
    birth.gpu_intent_world_tick = Some(Tick::new(birth.breeding_receipt.tick.raw() + 1));
    refresh_binding_digests(&mut wrong_tick);
    assert!(validate_committed_ei0_exit_gate_report(&wrong_tick).is_err());

    let mut wrong_actor = report.clone();
    let birth = wrong_actor
        .lifecycle
        .as_mut()
        .unwrap()
        .lanes
        .iter_mut()
        .find(|lane| lane.mode == HabitatMode::Wild)
        .unwrap()
        .births
        .first_mut()
        .unwrap();
    let wrong_organism = birth.breeding_receipt.second_parent;
    birth.actor = HabitatActor::Organism(wrong_organism);
    birth.breeding_receipt.actor = birth.actor;
    refresh_binding_digests(&mut wrong_actor);
    assert!(validate_committed_ei0_exit_gate_report(&wrong_actor).is_err());

    let mut missing_sequence = report.clone();
    let birth = missing_sequence
        .lifecycle
        .as_mut()
        .unwrap()
        .lanes
        .iter_mut()
        .find(|lane| lane.mode == HabitatMode::Wild)
        .unwrap()
        .births
        .first_mut()
        .unwrap();
    birth.gpu_intent_sequence_id = Some(0);
    birth
        .gpu_pre_action_world_digest
        .as_mut()
        .unwrap()
        .schema_version = 0;
    refresh_binding_digests(&mut missing_sequence);
    assert!(validate_committed_ei0_exit_gate_report(&missing_sequence).is_err());
}

#[test]
fn committed_validator_rejects_self_consistent_parent_organism_rewrite() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("reports")
        .join("ei0_exit_gate_report.json");
    let mut report: Ei0ExitGateReport =
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();

    validate_committed_ei0_exit_gate_report(&report).unwrap();
    let alternate_parents = [OrganismId(2001), OrganismId(2002)];
    let alternate_parent_genomes = alternate_parents
        .map(|organism_id| {
            report
                .lifecycle
                .as_ref()
                .unwrap()
                .population_residents
                .iter()
                .find(|resident| resident.organism_id == organism_id)
                .unwrap()
                .genome_id
        })
        .to_vec();
    let birth = report
        .lifecycle
        .as_mut()
        .unwrap()
        .lanes
        .iter_mut()
        .find(|lane| lane.mode == HabitatMode::Wild)
        .unwrap()
        .births
        .first_mut()
        .unwrap();
    assert_ne!(birth.parent_genome_ids, alternate_parent_genomes);

    birth.breeding_receipt.first_parent = alternate_parents[0];
    birth.breeding_receipt.second_parent = alternate_parents[1];
    birth.actor = HabitatActor::Organism(alternate_parents[0]);
    birth.breeding_receipt.actor = birth.actor;
    birth.gpu_selected_mate = Some(alternate_parents[1]);
    refresh_binding_digests(&mut report);

    assert!(validate_committed_ei0_exit_gate_report(&report).is_err());
}
