use std::path::Path;

use alife_game_app::brain_surgery::{
    apply_patch_to_bundle, init_base_bundle, load_and_verify_base_bundle, sample_v11_branch,
    verify_bundle_reopen_with_child_bin, BrainSurgeryPatchV1, DendriticPatchOp,
    BRAIN_SURGERY_PATCH_SCHEMA,
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
    apply_patch_to_bundle(&base_dir, &patch_path, &cand_dir, None).unwrap();

    let cli_bin = Path::new(env!("CARGO_BIN_EXE_brain_surgery_eval"));
    let receipt = verify_bundle_reopen_with_child_bin(&cand_dir, 700_001, cli_bin).unwrap();
    assert!(receipt.verified);
    assert_eq!(receipt.parent_pid, std::process::id());
    assert_ne!(receipt.child_pid, std::process::id());
    assert!(receipt.tick > 0);
    assert_eq!(receipt.dendritic_branch_count, 2);

    let verified_bundle = load_and_verify_base_bundle(&cand_dir).unwrap();
    assert!(verified_bundle.manifest.reopen_verified);
    assert_eq!(verified_bundle.manifest.dendritic_branch_count, 2);
    assert!(verified_bundle.manifest.reopen_receipt_sha256.is_some());

    let _ = std::fs::remove_dir_all(&root);
}
