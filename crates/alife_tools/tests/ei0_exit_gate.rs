#[cfg(feature = "gpu-tests")]
use std::collections::BTreeSet;
use std::{fs, path::PathBuf};

#[cfg(feature = "gpu-tests")]
use alife_archive::{CompositeGeneticArchiveInput, LineageLibrary, LineageLibraryConfig};
#[cfg(feature = "gpu-tests")]
use alife_core::{
    BrainCapacityClass, CreatureGenome, FoundationGeneticIdentity, FoundationWeightAsset,
    PhenotypeCompiler, PolicyBackend, SensorProfile,
};
use alife_core::{OrganismId, Tick};
#[cfg(feature = "gpu-tests")]
use alife_tools::ei0_exit_gate::{
    run_ei0_exit_gate, run_ei0_lifecycle_gate, write_ei0_exit_gate_report,
};
use alife_tools::ei0_exit_gate::{
    run_ei0_exit_gate_and_write, validate_committed_ei0_exit_gate_report, Ei0EvidenceStatus,
    Ei0ExitGateReport,
};
use alife_world::{HabitatActor, HabitatMode, HEADLESS_WORLD_SIGNATURE_SCHEMA_VERSION};

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

#[cfg(feature = "gpu-tests")]
#[test]
fn lifecycle_receipt_proves_restored_two_lane_multi_generation_population() {
    let root = temp_root("lifecycle");
    let mut preexisting = LineageLibrary::open(LineageLibraryConfig::profile_default(
        root.join("lineage-library"),
    ))
    .unwrap();
    let foundation_identity = FoundationGeneticIdentity::new(
        0x4E32_3034_385F_5631,
        1,
        0x4E32_3034_385F_FA11,
        BrainCapacityClass::N2048_ID,
    )
    .unwrap();
    let genome = CreatureGenome::early_mammal_founder(0xE10_F001, foundation_identity).unwrap();
    let foundation =
        FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    let expressed = genome.express().unwrap();
    let development = expressed
        .development_state_at(Tick::new(u64::from(
            expressed.development.maturation_duration_ticks,
        )))
        .unwrap();
    let phenotype = PhenotypeCompiler::compile_from_foundation_asset(
        &expressed.brain_genome,
        &BrainCapacityClass::n2048(),
        &development,
        SensorProfile::GroundedObjectSlotsV1,
        &foundation,
    )
    .unwrap();
    preexisting
        .archive_composite_birth(CompositeGeneticArchiveInput {
            source_run_id: "hostile-preexisting-run",
            organism_id: OrganismId(9_999),
            birth_tick: Tick::ZERO,
            creature_genome: &genome,
            phenotype: &phenotype,
            foundation_asset_bytes: &foundation.encode_canonical().unwrap(),
        })
        .unwrap();
    assert_eq!(preexisting.manifest_count().unwrap(), 1);
    drop(preexisting);
    let evidence = run_ei0_lifecycle_gate(&root).unwrap();
    let report = &evidence.report;

    assert_eq!(report.schema_version, 3);
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
    assert_eq!(report.stability.elapsed_ticks, 128);
    assert_ne!(
        report.stability.start_world_digest,
        report.stability.end_world_digest
    );
    assert_ne!(
        report.stability.start_ecology_metrics,
        report.stability.end_ecology_metrics
    );
    assert_eq!(
        report.stability.start_residents,
        report.stability.end_residents
    );
    assert!(report.same_seed_wrong_world_rejected);
    assert!(report.later_world_rejected);
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
                && birth.breeding_receipt.first_parent.raw() != 0
                && birth.child_phenotype_hash.0 != [0; 4]
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
    let wild = report
        .lanes
        .iter()
        .find(|lane| lane.mode == HabitatMode::Wild)
        .unwrap();
    let wild_sequences = wild
        .births
        .iter()
        .map(|birth| {
            (
                birth.breeding_receipt.first_parent.raw(),
                birth.gpu_intent_sequence_id.unwrap(),
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(wild_sequences.len(), wild.births.len());
    assert!(wild.births.iter().all(|birth| {
        let breeding = &birth.breeding_receipt;
        birth
            .gpu_intent_sequence_id
            .is_some_and(|sequence| sequence > 0)
            && birth.gpu_intent_world_tick == Some(breeding.tick)
            && birth.gpu_selected_mate == Some(breeding.second_parent)
            && birth.actor == HabitatActor::Organism(breeding.first_parent)
            && birth.gpu_pre_action_world_digest.is_some_and(|digest| {
                digest.schema_version == HEADLESS_WORLD_SIGNATURE_SCHEMA_VERSION
                    && digest.words != [0; 4]
            })
            && birth.gpu_same_seed_wrong_world_rejected == Some(true)
            && birth.gpu_later_world_rejected == Some(true)
    }));
    assert_eq!(
        wild.births[0].gpu_pre_action_world_digest,
        Some(report.stability.end_world_digest)
    );
    let managed = report
        .lanes
        .iter()
        .find(|lane| lane.mode == HabitatMode::Managed)
        .unwrap();
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
    assert_eq!(report.schema_version, 3);
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

#[test]
fn committed_report_recomputes_current_source_and_causal_evidence() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("reports")
        .join("ei0_exit_gate_report.json");
    let report: Ei0ExitGateReport =
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();

    validate_committed_ei0_exit_gate_report(&report).unwrap();
    let binding = report.artifact_binding.as_ref().unwrap();
    assert_eq!(binding.adapter_name, "NVIDIA GeForce RTX 3050");
    assert_eq!(binding.backend_api, "vulkan");

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
    assert!(validate_committed_ei0_exit_gate_report(&wrong_mate).is_err());

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
        assert_eq!(gpu.phenotype_hash, lane.births[2].child_phenotype_hash);
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
