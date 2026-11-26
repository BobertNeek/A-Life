use std::path::Path;
use std::process::Command;

use alife_game_app::brain_surgery::{
    apply_patch_to_bundle, init_base_bundle, load_and_verify_base_bundle, sample_v11_branch,
    verify_bundle_reopen_with_child_bin, BrainSurgeryPatchV1, DendriticPatchOp, WeightEdit,
    WeightEditMode, BRAIN_SURGERY_PATCH_SCHEMA,
};

#[test]
fn separate_process_reopen_preserves_edited_weights_and_v11_branches_bit_for_bit() {
    let root = std::env::temp_dir().join(format!(
        "alife-brain-surgery-gpu-reopen-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let base_dir = root.join("base");
    let cand_dir = root.join("cand");
    std::fs::create_dir_all(&root).unwrap();

    let base = init_base_bundle(&base_dir, "r41-gpu-base", None, None, None, 700_001).unwrap();
    let branch_a = sample_v11_branch(42, 7, 0.625).unwrap();
    let branch_b = sample_v11_branch(1024, 15, -0.375).unwrap();
    let patch = BrainSurgeryPatchV1 {
        schema: BRAIN_SURGERY_PATCH_SCHEMA.into(),
        schema_version: 1,
        patch_id: "r42-v11-reopen-test".into(),
        target_base_id: base.manifest.base_id.clone(),
        target_foundation_digest: base.manifest.foundation_digest.clone(),
        target_dendritic_branches_sha256: base.manifest.dendritic_branches_sha256.clone(),
        weight_edits: vec![],
        dendritic_edits: DendriticPatchOp::UpsertTargets {
            remove_targets: vec![],
            add_branches: vec![branch_a, branch_b],
        },
        notes: Some("GPU reopen bit-exact V11 branch preservation test".into()),
    };
    let patch_path = root.join("patch.json");
    std::fs::write(&patch_path, serde_json::to_vec_pretty(&patch).unwrap()).unwrap();
    apply_patch_to_bundle(&base_dir, &patch_path, &cand_dir, Some("r42-v11-cand")).unwrap();

    let cli_bin = Path::new(env!("CARGO_BIN_EXE_brain_surgery_eval"));
    let receipt = verify_bundle_reopen_with_child_bin(&cand_dir, 700_001, cli_bin, true).unwrap();
    assert!(receipt.verified);
    assert_eq!(receipt.parent_pid, std::process::id());
    assert_ne!(receipt.child_pid, std::process::id());
    assert!(receipt.tick > 0);
    assert_eq!(receipt.dendritic_branch_count, 2);
    assert!(receipt
        .semantic_prior_mode
        .contains("backend_deterministic"));

    let verified_bundle = load_and_verify_base_bundle(&cand_dir).unwrap();
    assert!(verified_bundle.manifest.reopen_verified);
    assert_eq!(verified_bundle.manifest.dendritic_branch_count, 2);
    assert!(verified_bundle.manifest.reopen_receipt_sha256.is_some());

    // Read-only verify-reopen (stamp_bundle = false) must never mutate manifest.json
    // or reopen_receipt.json, preserving the ledger-pinned manifest SHA-256.
    let manifest_before = std::fs::read(cand_dir.join("manifest.json")).unwrap();
    let receipt_before = std::fs::read(cand_dir.join("reopen_receipt.json")).unwrap();
    let ro_receipt =
        verify_bundle_reopen_with_child_bin(&cand_dir, 700_001, cli_bin, false).unwrap();
    assert!(ro_receipt.verified);
    assert_eq!(
        std::fs::read(cand_dir.join("manifest.json")).unwrap(),
        manifest_before
    );
    assert_eq!(
        std::fs::read(cand_dir.join("reopen_receipt.json")).unwrap(),
        receipt_before
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn score_forces_deterministic_semantic_prior_under_ambient_env_overrides() {
    let root = std::env::temp_dir().join(format!(
        "alife-brain-surgery-gpu-prior-lock-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let base_dir = root.join("base");
    let cand_dir = root.join("cand");
    let work_dir = root.join("work");
    std::fs::create_dir_all(&root).unwrap();

    let base = init_base_bundle(&base_dir, "r41-prior-base", None, None, None, 700_001).unwrap();
    let patch = BrainSurgeryPatchV1 {
        schema: BRAIN_SURGERY_PATCH_SCHEMA.into(),
        schema_version: 1,
        patch_id: "p-prior-lock".into(),
        target_base_id: base.manifest.base_id.clone(),
        target_foundation_digest: base.manifest.foundation_digest.clone(),
        target_dendritic_branches_sha256: base.manifest.dendritic_branches_sha256.clone(),
        weight_edits: vec![WeightEdit {
            index: 0,
            mode: WeightEditMode::Add,
            value: 0.001,
        }],
        dendritic_edits: DendriticPatchOp::Keep,
        notes: None,
    };
    let patch_path = root.join("patch.json");
    std::fs::write(&patch_path, serde_json::to_vec_pretty(&patch).unwrap()).unwrap();
    apply_patch_to_bundle(&base_dir, &patch_path, &cand_dir, Some("r42-prior-cand")).unwrap();

    let mut scorecard = base.scorecard.clone();
    scorecard.scorecard_id = "prior-lock-check".into();
    scorecard.cases.truncate(1);
    scorecard.cases[0].id = "prior-case".into();
    scorecard.cases[0].ticks = 8;
    scorecard.cases[0].request_token = None;
    let scorecard_path = root.join("scorecard.json");
    std::fs::write(
        &scorecard_path,
        serde_json::to_vec_pretty(&scorecard).unwrap(),
    )
    .unwrap();

    let report_path = root.join("report.json");
    let cli_bin = Path::new(env!("CARGO_BIN_EXE_brain_surgery_eval"));
    let output = Command::new(cli_bin)
        .args([
            "score",
            "--base",
            base_dir.to_str().unwrap(),
            "--candidate",
            cand_dir.to_str().unwrap(),
            "--scorecard",
            scorecard_path.to_str().unwrap(),
            "--patch-id",
            "p-prior-lock",
            "--out",
            report_path.to_str().unwrap(),
            "--work-dir",
            work_dir.to_str().unwrap(),
        ])
        .env("ALIFE_SLM_PRIOR", "off")
        .env("ALIFE_SLM_PRIOR_BACKEND", "local")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "score failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&report_path).unwrap()).unwrap();
    assert_eq!(report["failed_cases"].as_u64(), Some(0));

    for prefix in ["base-prior-case", "cand-prior-case"] {
        let diag_path = work_dir.join(prefix).join("lesson-diagnostic.json");
        let diag: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&diag_path).unwrap()).unwrap();
        assert_eq!(
            diag["semantic_prior"]["backend"].as_str(),
            Some("deterministic"),
            "{prefix} diagnostic did not enforce deterministic prior: {diag}"
        );
    }

    let _ = std::fs::remove_dir_all(&root);
}
