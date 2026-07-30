use std::{collections::BTreeMap, fs, path::PathBuf};

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

fn digest_map<const N: usize>(entries: [(&str, &str); N]) -> BTreeMap<String, String> {
    entries
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
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

#[test]
fn committed_report_locks_exact_promotion_evidence_digests() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("reports")
        .join("ei0_exit_gate_report.json");
    let report: Ei0ExitGateReport =
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();

    assert_eq!(report.schema_version, 2);
    assert!(report.verdict.era0_exit_gate_passed);
    assert!(!report.verdict.era1_promotion_evaluated);
    assert!(report.clauses.all_passed());
    assert!(report.operational_error.is_none());

    let expected_source_genomes = digest_map([
        (
            "1001",
            "blake3-256:22502f1e0e54a13c8916a7ce7b2aeae7203a3e9ce2cc471cebb94a70ea6b6145",
        ),
        (
            "1002",
            "blake3-256:a24f214bae8f34cdea248a723ee514c18717c0eb354e3ad9701cbb313513231c",
        ),
        (
            "1003",
            "blake3-256:36fa4265f80a0de0121800821b6ec88111e40f900832a20dc281516cc841b045",
        ),
        (
            "1004",
            "blake3-256:d4dbb60bc7ba03a50b74bff1dfd4b4cdc86ff15d765895f67896cbaada83fc0c",
        ),
        (
            "2001",
            "blake3-256:0ba6200b86cb0c8a726e8cf3b38927ea69a5a0003d27301a7c9e12eaccedf508",
        ),
        (
            "2002",
            "blake3-256:0c0df8a97a3f635f67e97b164beae1e30878d3001c487385eff6f6f8ef9fb33f",
        ),
        (
            "2003",
            "blake3-256:1ce5c02e40774106742a46ab233f5f4f982b92447d573fefd4a2d857c3e5c8b3",
        ),
        (
            "2004",
            "blake3-256:fd31c498656ce40d913acacb6d3c645b8e1fa05516f2f5886702f293e6c73e9e",
        ),
        (
            "3001",
            "blake3-256:f60cf7db23b8d095ea4f74ae3bbed028041d1e0dee44260b2352843c529bd376",
        ),
        (
            "3002",
            "blake3-256:97f24972d9f923342a98571996d1360e48bf6ade947b62c847f18f0529202d53",
        ),
        (
            "3003",
            "blake3-256:accea0f5df2bffa8b29746dc4ef574209b0efcf43daa0e0a3d690defed717f6d",
        ),
        (
            "4001",
            "blake3-256:186722fc99677e5ed648abb82c4158d65a60591aaecdc3bbce6c0cdd8adc47f8",
        ),
        (
            "4002",
            "blake3-256:fcd7aee4a4c51aa4d7a0b13f2c41f7c4d8890a22cfe6e5eb066ab9fca45cc748",
        ),
        (
            "4003",
            "blake3-256:d7eb83586e2ca77c67ac25b26972c2e3c9b04901843ae690abf6e15e3817a33d",
        ),
    ]);
    let expected_archive_manifests = digest_map([
        (
            "1001",
            "blake3-256:940394f8b4398ea9da025effafb115ca0f73b33fd08a78851fa2b2f6af389edf",
        ),
        (
            "1002",
            "blake3-256:d81ada36ce604c01a79a27400c31a4808546708f660c33b7bb27d5a48a3d3b9c",
        ),
        (
            "1003",
            "blake3-256:501909a94f249e73c311915f9fd662d3bff047884d5608f41cdfe6a02448092a",
        ),
        (
            "1004",
            "blake3-256:59d55964959c619ac60be9c98e265bc35e367cb68c5a0c38e5cb8c01988cece7",
        ),
        (
            "2001",
            "blake3-256:c56f702d7ee68da5febf2d89d4727dce180bdf79ccac3a0be95c8b6904550062",
        ),
        (
            "2002",
            "blake3-256:ed9478ea429ffd16a2d21f009809c1f976e8ac31a9021200b944813c62c584b0",
        ),
        (
            "2003",
            "blake3-256:d8fc40da333f118e197ccb17f9693d402b392a20aff937b71d237ea85fbb1ad1",
        ),
        (
            "2004",
            "blake3-256:dbd235d8ecbabae47cb7942a7058ed744d00384845a94358a60388aab37e0b34",
        ),
        (
            "3001",
            "blake3-256:d3254998ab819f50e98e0e5c830c73a9646b9cbe0d021be85b72b03361967c81",
        ),
        (
            "3002",
            "blake3-256:ca0e111df670686ae22c18e4bfb8106428f5f3864e3ce57aea1b19b6a8c0e382",
        ),
        (
            "3003",
            "blake3-256:f2d4e4e42ec0d295d3ef62eff9bb8e4ea12d4127f1c4b8af2049f0b750cd52dc",
        ),
        (
            "4001",
            "blake3-256:ea5954441fe958db5eb6d46a11ad9b19a9ab9ffd189ab1c72fbbe1d8102f76eb",
        ),
        (
            "4002",
            "blake3-256:89bfa61a5f48d47d274e176dc36a2cee68144ddae4d63ff482d45bbf2c886829",
        ),
        (
            "4003",
            "blake3-256:1bac9ccabf5eef792f9f40817bacace1e44dcc2ac584622befdece5e5d4c3bc6",
        ),
    ]);

    assert_eq!(
        report.evidence_digests.source_genomes,
        expected_source_genomes
    );
    assert_eq!(
        report.evidence_digests.archive_composite_assets,
        expected_source_genomes
    );
    assert_eq!(
        report.evidence_digests.archive_manifests,
        expected_archive_manifests
    );
    assert_eq!(
        report.evidence_digests.foundation_weights.as_deref(),
        Some("blake3-256:d5c69f365b83f46abbe6004326042b7805cf000d0e7d0a63f919d6284dc66e11")
    );
    assert_eq!(
        report.evidence_digests.shader_bundle.as_deref(),
        Some("blake3-256:551303b3b5b48e671f7c98b19900567b36ee1d524ff66a6933794427377b5a1b")
    );
    assert_eq!(
        report.evidence_digests.portable_save.as_deref(),
        Some("blake3-256:4eba996d9de78c85b4b7149ae03735808c9172bab292a2165d85d4834ed1bf81")
    );

    let baseline = report.heuristic_baseline.unwrap();
    assert_eq!(baseline.source_backend, "HeuristicBaseline");
    assert!(!baseline.promotion_eligible);
    assert_eq!(baseline.hidden_promotion_trials, 0);
    assert!(baseline.unknown_measures_preserved);
    assert_eq!(baseline.unknown_measures.len(), 9);
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
