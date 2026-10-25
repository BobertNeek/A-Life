use super::*;
#[test]
fn native_serial_export_preserves_corrected_critic_age_and_original_source() {
    let f = crate::foundation_critic_correction::tests::full_residual_correction_fixture();
    let before =
        SERIAL_HANDOFF_SOURCE_FILES.map(|name| std::fs::read(f.target.join(name)).unwrap());
    let output = f.root.join("portable");
    let binding = export_native_training_handoff(&f.target, &output).unwrap();
    let handoff: FoundationTrainingHandoffV1 =
        serde_json::from_slice(&std::fs::read(output.join(SERIAL_HANDOFF_FILE)).unwrap()).unwrap();
    assert_eq!(handoff.actor.optimizer_step, 1);
    assert_eq!(handoff.value.optimizer_step, 6);
    let original_receipt: FoundationCycleReceipt = serde_json::from_slice(&before[0]).unwrap();
    assert_eq!(original_receipt.value_optimizer_step, 1);
    assert_eq!(handoff.value.parameters[0], 0.25);
    assert!(
        handoff.actor.stage_mask
            == serde_json::from_slice::<alife_training::FoundationTrainerCheckpoint>(&before[2])
                .unwrap()
                .stage_mask
    );
    for (name, bytes) in SERIAL_HANDOFF_SOURCE_FILES.iter().zip(before) {
        assert_eq!(std::fs::read(f.target.join(name)).unwrap(), bytes);
        assert_eq!(std::fs::read(output.join(name)).unwrap(), bytes);
    }
    binding
        .validate(
            &binding.source_sha256,
            &sha(&std::fs::read(output.join(SERIAL_HANDOFF_FILE)).unwrap()),
        )
        .unwrap();
    assert!(!output.join("critic-correction.json").exists());
}
#[test]
fn native_serial_export_runs_correction_verifier_before_creating_output() {
    let f = crate::foundation_critic_correction::tests::full_residual_correction_fixture();
    let actor_path = f.target.join("actor-checkpoint.json");
    let mut actor: alife_training::FoundationTrainerCheckpoint =
        serde_json::from_slice(&std::fs::read(&actor_path).unwrap()).unwrap();
    actor.first_moment[0] = 0.125;
    let changed = serde_json::to_vec(&actor).unwrap();
    std::fs::write(&actor_path, &changed).unwrap();
    let output = f.root.join("rejected");
    assert!(export_native_training_handoff(&f.target, &output).is_err());
    assert!(!output.exists());
    assert_eq!(std::fs::read(&actor_path).unwrap(), changed);
}

#[test]
fn native_export_cannot_mutate_target_or_original_correction_tree() {
    let f = crate::foundation_critic_correction::tests::full_residual_correction_fixture();
    for root in [&f.target, &f.source] {
        let output = root.join("new-parent/portable");
        assert!(export_native_training_handoff(&f.target, &output)
            .unwrap_err()
            .to_string()
            .contains("overlap"));
        assert!(!root.join("new-parent").exists());
        crate::inspect_foundation_checkpoint(&f.target, 5124).unwrap();
    }
}
#[cfg(unix)]
#[test]
fn native_export_resolves_output_parent_symlinks_before_ownership() {
    let f = crate::foundation_critic_correction::tests::full_residual_correction_fixture();
    let alias = f.root.join("original-source-alias");
    std::os::unix::fs::symlink(&f.source, &alias).unwrap();
    let output = alias.join("not-created/portable");
    assert!(export_native_training_handoff(&f.target, &output)
        .unwrap_err()
        .to_string()
        .contains("overlap"));
    assert!(!f.source.join("not-created").exists());
    crate::inspect_foundation_checkpoint(&f.target, 5124).unwrap();
}
