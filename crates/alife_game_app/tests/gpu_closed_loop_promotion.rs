//! Promotion is derived from complete, adapter-bound A/B/C/D evidence only.
#![cfg(feature = "gpu-runtime")]

use alife_core::{BrainCapacityClass, PhenotypeHash, SensorProfile};
use alife_game_app::{
    build_gpu_closed_loop_promotion_from_paths, canonical_gate_command_argv,
    ingest_promotion_evidence, load_gpu_closed_loop_promotion_manifest,
    write_gpu_closed_loop_gate_receipt_from_capture, write_gpu_closed_loop_promotion_manifest,
    BenchmarkManifestBinding, BenchmarkRowBinding, EvidenceAdapterBinding, EvidenceArtifactBinding,
    GateCommandCapture, GateCommandReceipt, GateEvidenceBinding, GitObjectId,
    GpuClosedLoopGateReceipt, GpuGateAdapterReceipt, GpuGateCaptureManifest,
    PromotionArtifactPaths, PromotionEvidenceInputs,
};

const PROFILE_SCHEMA: u16 = 1;
const ARTIFACT_SCHEMA: u16 = 1;
const PASSING: u16 = 1;
const MISSED: u16 = 2;
const POPULATIONS: [u32; 6] = [1, 10, 50, 100, 250, 500];

#[test]
fn artifact_path_loader_requires_the_exact_abcd_matrix() {
    let error =
        build_gpu_closed_loop_promotion_from_paths(&PromotionArtifactPaths::default()).unwrap_err();
    assert!(error
        .to_string()
        .contains("exactly three Slice A, three Slice B, six Slice C, and six Slice D"));
}

#[test]
fn committed_gate_script_has_the_exact_order_and_rust_receipt_writer() {
    let script = include_str!("../../../scripts/run_gpu_closed_loop_gates.ps1");
    let labels = [
        "01-fmt",
        "02-check",
        "03-workspace-tests",
        "04-core-brain",
        "05-gpu-brain",
        "06-world-save",
        "07-app-brain",
        "08-tools-benchmark",
        "09-docs",
        "10-boundaries",
        "11-authority-scan",
        "12-diff",
    ];
    let mut previous = 0;
    for label in labels {
        let index = script.find(label).unwrap();
        assert!(index >= previous, "gate command order changed at {label}");
        previous = index;
    }
    assert!(script.contains("git status --porcelain=v1"));
    assert!(script.contains("git rev-parse 'HEAD^{tree}'"));
    assert!(script.contains("gpu-closed-loop-gate-seal"));
    assert!(script.contains("--adapter-evidence $AdapterEvidence"));
    assert!(script.contains("ConvertTo-WindowsCommandLineArgument"));
    assert!(!script.contains(".ArgumentList.Add"));
    assert!(script.contains("authority-scan-v1"));
    assert!(script.contains("legacy_neural_policy_v1.rs"));
}

#[test]
fn git_object_ids_use_one_strict_lowercase_hex_string() {
    let id = oid(0x11);
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, format!("\"{}\"", "11".repeat(20)));
    assert_eq!(serde_json::from_str::<GitObjectId>(&json).unwrap(), id);
    assert!(serde_json::from_str::<GitObjectId>(&format!("\"{}\"", "AA".repeat(20))).is_err());
    assert!(serde_json::from_str::<GitObjectId>("[1,2,3]").is_err());
}

#[test]
fn no_class_promotes_without_complete_abcd_evidence() {
    let mut inputs = valid_receipt_set();
    inputs.artifact_bindings.retain(|binding| {
        !(binding.class_id_raw == BrainCapacityClass::N1024_ID.raw() && binding.slice_raw == 2)
    });
    let inputs = reseal(inputs);

    let manifest = ingest_promotion_evidence(inputs).unwrap();
    assert_eq!(
        manifest.promoted_classes,
        vec![BrainCapacityClass::N512_ID, BrainCapacityClass::N2048_ID]
    );
    assert!(!manifest
        .rows
        .iter()
        .find(|row| row.class_id_raw == BrainCapacityClass::N1024_ID.raw())
        .unwrap()
        .all_required_gates_pass());
}

#[test]
fn complete_matrix_promotes_exactly_three_classes() {
    let manifest = ingest_promotion_evidence(valid_receipt_set()).unwrap();
    assert_eq!(
        manifest.promoted_classes,
        vec![
            BrainCapacityClass::N512_ID,
            BrainCapacityClass::N1024_ID,
            BrainCapacityClass::N2048_ID,
        ]
    );
    assert!(manifest
        .rows
        .iter()
        .all(|row| row.all_required_gates_pass()));
}

#[test]
fn different_valid_per_slice_phenotype_hashes_are_allowed_but_tampering_is_not() {
    let mut inputs = valid_receipt_set();
    for (index, binding) in inputs.artifact_bindings.iter_mut().enumerate() {
        binding.phenotype_hash = PhenotypeHash([index as u64 + 1, 2, 3, 4]);
    }
    let inputs = reseal(inputs);
    assert!(ingest_promotion_evidence(inputs.clone()).is_ok());

    let mut tampered = inputs;
    tampered.artifact_bindings[0].phenotype_hash.0[0] ^= 1;
    assert!(ingest_promotion_evidence(tampered).is_err());
}

#[test]
fn promotion_retains_exact_gate_benchmark_and_adapter_bindings() {
    let manifest = ingest_promotion_evidence(valid_receipt_set()).unwrap();
    assert_ne!(manifest.gate.receipt_digest, [0; 4]);
    assert_ne!(manifest.benchmark.manifest_digest, [0; 4]);
    assert!(manifest
        .rows
        .iter()
        .all(|row| row.benchmark_rows.len() == 12));
    assert!(manifest
        .rows
        .iter()
        .flat_map(|row| &row.artifact_bindings)
        .all(|binding| binding.adapter == manifest.adapter));
    assert!(manifest
        .rows
        .iter()
        .flat_map(|row| &row.benchmark_rows)
        .all(|binding| binding.adapter == manifest.adapter));
    assert_eq!(manifest.gate.adapter, manifest.adapter);
    assert_eq!(manifest.benchmark.adapter, manifest.adapter);
}

#[test]
fn adapter_tree_capacity_and_trust_mismatches_are_rejected() {
    let mut adapter = valid_receipt_set();
    adapter.artifact_bindings[0].adapter.device_id ^= 1;
    assert!(ingest_promotion_evidence(adapter).is_err());

    let mut tree = valid_receipt_set();
    tree.artifact_bindings[0].source_tree = oid(0x77);
    assert!(ingest_promotion_evidence(tree).is_err());

    let mut capacity = valid_receipt_set();
    capacity.artifact_bindings[0].capacity_digest[0] ^= 1;
    assert!(ingest_promotion_evidence(capacity).is_err());

    let mut ancestry = valid_receipt_set();
    ancestry.artifact_bindings[0].evidence_commit = oid(0x88);
    let ancestry = reseal(ancestry);
    assert!(ingest_promotion_evidence(ancestry).is_err());
}

#[test]
fn malformed_slice_profiles_and_duplicate_benchmark_keys_are_rejected() {
    let mut profile = valid_receipt_set();
    profile.artifact_bindings[0].profile_id_raw = SensorProfile::GroundedObjectSlotsV1.raw();
    let profile = reseal(profile);
    assert!(ingest_promotion_evidence(profile).is_err());

    let mut duplicate = valid_receipt_set();
    duplicate.benchmark_rows.push(duplicate.benchmark_rows[0]);
    let duplicate = reseal(duplicate);
    assert!(ingest_promotion_evidence(duplicate).is_err());
}

#[test]
fn honest_missed_benchmark_row_blocks_only_its_class() {
    let mut inputs = valid_receipt_set();
    inputs
        .benchmark_rows
        .iter_mut()
        .find(|row| row.class_id_raw == BrainCapacityClass::N2048_ID.raw())
        .unwrap()
        .status_raw = MISSED;
    inputs.benchmark.row_bindings_digest =
        BenchmarkManifestBinding::digest_rows(&inputs.benchmark_rows);
    let manifest = ingest_promotion_evidence(reseal(inputs)).unwrap();
    assert_eq!(
        manifest.promoted_classes,
        vec![BrainCapacityClass::N512_ID, BrainCapacityClass::N1024_ID]
    );
}

#[test]
fn manifest_digest_detects_post_ingestion_tampering() {
    let manifest = ingest_promotion_evidence(valid_receipt_set()).unwrap();
    manifest.validate().unwrap();
    let mut tampered = manifest;
    tampered.promoted_classes.clear();
    assert!(tampered.validate().is_err());
}

#[test]
fn gate_receipt_requires_exact_order_and_binds_all_captured_bytes() {
    let adapter = GpuGateAdapterReceipt::new(
        0x10de,
        0x25a2,
        1,
        "test-vulkan-adapter",
        digest(10),
        digest(20),
        digest(30),
    )
    .unwrap();
    let commands = (1_u16..=12)
        .map(|id| {
            GateCommandReceipt::new(
                id,
                canonical_gate_command_argv(id).unwrap(),
                u64::from(id) * 10,
                u64::from(id) * 10 + 5,
                0,
                format!("stdout-{id}").as_bytes(),
                format!("stderr-{id}").as_bytes(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let receipt = GpuClosedLoopGateReceipt::new(
        oid(0x11),
        oid(0x33),
        adapter,
        b"committed gate script bytes",
        commands,
    )
    .unwrap();
    receipt.validate().unwrap();
    assert_eq!(receipt.commands.len(), 12);
    assert!(receipt.passed);

    let mut tampered = receipt;
    tampered.commands.swap(0, 1);
    assert!(tampered.validate().is_err());

    let wrong_commands = (1_u16..=12)
        .map(|id| {
            GateCommandReceipt::new(
                id,
                if id == 1 {
                    b"cargo\0fmt".to_vec()
                } else {
                    canonical_gate_command_argv(id).unwrap()
                },
                u64::from(id) * 10,
                u64::from(id) * 10 + 5,
                0,
                b"stdout",
                b"stderr",
            )
            .unwrap()
        })
        .collect();
    assert!(GpuClosedLoopGateReceipt::new(
        oid(0x11),
        oid(0x33),
        adapter,
        b"committed gate script bytes",
        wrong_commands,
    )
    .is_err());
}

#[test]
fn gate_capture_writer_hashes_exact_raw_streams_and_publishes_atomically() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("alife-gate-capture-{}-{nonce}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let script = root.join("run_gpu_closed_loop_gates.ps1");
    std::fs::write(&script, b"committed gate script bytes").unwrap();
    let mut commands = Vec::new();
    for id in 1_u16..=12 {
        let stdout_path = root.join(format!("{id:02}.stdout"));
        let stderr_path = root.join(format!("{id:02}.stderr"));
        std::fs::write(&stdout_path, format!("stdout-{id}\r\n").as_bytes()).unwrap();
        std::fs::write(&stderr_path, format!("stderr-{id}\n").as_bytes()).unwrap();
        commands.push(GateCommandCapture {
            command_id: id,
            argv_utf8: canonical_gate_command_argv(id).unwrap(),
            started_monotonic_ns: u64::from(id) * 100,
            ended_monotonic_ns: u64::from(id) * 100 + 10,
            exit_code: 0,
            stdout_path,
            stderr_path,
        });
    }
    let capture = GpuGateCaptureManifest {
        schema_version: 1,
        git_commit: "11".repeat(20),
        source_tree_digest: "33".repeat(20),
        commands,
    };
    let capture_path = root.join("capture.json");
    std::fs::write(&capture_path, serde_json::to_vec_pretty(&capture).unwrap()).unwrap();
    let output = root.join("gates.json");
    let adapter = GpuGateAdapterReceipt::new(
        0x10de,
        0x25a2,
        1,
        "test-vulkan-adapter",
        digest(10),
        digest(20),
        digest(30),
    )
    .unwrap();

    let receipt =
        write_gpu_closed_loop_gate_receipt_from_capture(&capture_path, &script, adapter, &output)
            .unwrap();
    assert_eq!(receipt.commands.len(), 12);
    assert_eq!(
        alife_game_app::load_gpu_closed_loop_gate_receipt(&output).unwrap(),
        receipt
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn promotion_manifest_round_trip_is_atomic_and_digest_checked() {
    let root = std::env::temp_dir().join(format!(
        "alife-promotion-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("promotion.json");
    let manifest = ingest_promotion_evidence(valid_receipt_set()).unwrap();
    write_gpu_closed_loop_promotion_manifest(&path, &manifest).unwrap();
    assert_eq!(
        load_gpu_closed_loop_promotion_manifest(&path).unwrap(),
        manifest
    );

    let mut json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    json["promoted_classes"] = serde_json::json!([]);
    std::fs::write(&path, serde_json::to_vec_pretty(&json).unwrap()).unwrap();
    assert!(load_gpu_closed_loop_promotion_manifest(&path).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

fn valid_receipt_set() -> PromotionEvidenceInputs {
    let promotion_commit = oid(0x11);
    let ancestor_commit = oid(0x22);
    let source_tree = oid(0x33);
    let adapter =
        EvidenceAdapterBinding::new(0x10de, 0x25a2, 1, digest(10), digest(20), digest(30)).unwrap();
    let mut artifact_bindings = Vec::new();
    let mut benchmark_rows = Vec::new();
    for class in [
        BrainCapacityClass::n512(),
        BrainCapacityClass::n1024(),
        BrainCapacityClass::n2048(),
    ] {
        let class_id = class.id().raw();
        for (slice, profile, schema) in [
            (1, 0, 0),
            (2, 0, 0),
            (
                3,
                SensorProfile::PrivilegedAffordanceV1.raw(),
                PROFILE_SCHEMA,
            ),
            (
                3,
                SensorProfile::GroundedObjectSlotsV1.raw(),
                PROFILE_SCHEMA,
            ),
            (
                4,
                SensorProfile::PrivilegedAffordanceV1.raw(),
                PROFILE_SCHEMA,
            ),
            (
                4,
                SensorProfile::GroundedObjectSlotsV1.raw(),
                PROFILE_SCHEMA,
            ),
        ] {
            artifact_bindings.push(EvidenceArtifactBinding {
                slice_raw: slice,
                class_id_raw: class_id,
                profile_id_raw: profile,
                profile_schema: schema,
                artifact_schema: ARTIFACT_SCHEMA,
                evidence_commit: ancestor_commit,
                source_tree,
                artifact_digest: digest(
                    u64::from(slice) * 100 + u64::from(class_id) * 10 + u64::from(profile),
                ),
                phenotype_hash: PhenotypeHash(digest(u64::from(slice) + u64::from(class_id))),
                phenotype_manifest_digest: digest(200 + u64::from(slice) + u64::from(class_id)),
                capacity_digest: class.canonical_digest(),
                adapter,
                status_raw: PASSING,
            });
        }
        for profile in [
            SensorProfile::PrivilegedAffordanceV1,
            SensorProfile::GroundedObjectSlotsV1,
        ] {
            for population in POPULATIONS {
                benchmark_rows.push(BenchmarkRowBinding {
                    class_id_raw: class_id,
                    profile_id_raw: profile.raw(),
                    profile_schema: PROFILE_SCHEMA,
                    population,
                    status_raw: PASSING,
                    row_digest: digest(
                        1_000
                            + u64::from(class_id) * 100
                            + u64::from(profile.raw()) * 10
                            + u64::from(population),
                    ),
                    phenotype_hash: PhenotypeHash(digest(
                        2_000 + u64::from(class_id) + u64::from(profile.raw()),
                    )),
                    phenotype_manifest_digest: digest(
                        3_000 + u64::from(class_id) + u64::from(profile.raw()),
                    ),
                    capacity_digest: class.canonical_digest(),
                    protocol_digest: digest(4_000),
                    adapter,
                });
            }
        }
    }
    let benchmark = BenchmarkManifestBinding {
        evidence_commit: ancestor_commit,
        source_tree,
        manifest_digest: digest(5_000),
        protocol_digest: digest(4_000),
        adapter,
        row_bindings_digest: BenchmarkManifestBinding::digest_rows(&benchmark_rows),
    };
    let gate = GateEvidenceBinding {
        evidence_commit: promotion_commit,
        source_tree,
        receipt_digest: digest(6_000),
        gate_script_digest: digest(6_001),
        commands_digest: digest(6_002),
        adapter,
    };
    PromotionEvidenceInputs::new(
        promotion_commit,
        source_tree,
        adapter,
        gate,
        benchmark,
        artifact_bindings,
        benchmark_rows,
        vec![ancestor_commit],
    )
    .unwrap()
}

fn reseal(inputs: PromotionEvidenceInputs) -> PromotionEvidenceInputs {
    PromotionEvidenceInputs::new(
        inputs.promotion_commit,
        inputs.source_tree_digest,
        inputs.adapter,
        inputs.gate,
        inputs.benchmark,
        inputs.artifact_bindings,
        inputs.benchmark_rows,
        inputs.trusted_ancestor_commits,
    )
    .unwrap()
}

fn oid(byte: u8) -> GitObjectId {
    GitObjectId([byte; 20])
}

fn digest(seed: u64) -> [u64; 4] {
    [seed, seed + 1, seed + 2, seed + 3]
}
