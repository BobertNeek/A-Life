use super::*;
#[test]
fn native_serial_export_preserves_corrected_critic_age_and_original_source() {
    let f = crate::foundation_critic_correction::tests::full_residual_correction_fixture();
    let before =
        SERIAL_HANDOFF_SOURCE_FILES.map(|name| std::fs::read(f.target.join(name)).unwrap());
    let output = f.root.join("portable");
    let binding = export_native_training_handoff(&f.target, &output).unwrap();
    let restored_binding: FoundationSerialHandoffBindingV1 =
        serde_json::from_slice(&std::fs::read(output.join(SERIAL_HANDOFF_BINDING_FILE)).unwrap())
            .unwrap();
    assert_eq!(restored_binding, binding);
    let handoff_bytes = std::fs::read(output.join(SERIAL_HANDOFF_FILE)).unwrap();
    let handoff: FoundationTrainingHandoffV1 = serde_json::from_slice(&handoff_bytes).unwrap();
    let pretty_bytes = serde_json::to_vec_pretty(&handoff).unwrap();
    assert!(handoff_bytes.len() < pretty_bytes.len());
    let source_len: usize = before.iter().map(Vec::len).sum();
    let package_len: u64 = std::fs::read_dir(&output)
        .unwrap()
        .map(|entry| entry.unwrap().metadata().unwrap().len())
        .sum();
    eprintln!(
        "synthetic N2048 export: handoff compact={} pretty={}; unchanged sources={source_len}; package compact={package_len} pretty={}",
        handoff_bytes.len(), pretty_bytes.len(),
        package_len + (pretty_bytes.len() - handoff_bytes.len()) as u64,
    );
    // Both wire encodings restore every typed field and pass device-free
    // admission; retained packages keep the digest of their own exact bytes.
    let pretty_handoff: FoundationTrainingHandoffV1 =
        serde_json::from_slice(&pretty_bytes).unwrap();
    assert_eq!(handoff, pretty_handoff);
    let admitted = handoff.clone().admit().unwrap();
    pretty_handoff.admit().unwrap();
    assert_eq!(
        admitted.actor,
        serde_json::from_slice::<alife_training::FoundationTrainerCheckpoint>(&before[2]).unwrap()
    );
    assert_eq!(
        admitted.value,
        serde_json::from_slice::<alife_training::PpoValueHeadCheckpoint>(&before[3]).unwrap()
    );
    assert_eq!(admitted.foundation.encode_canonical().unwrap(), before[1]);
    let pinned = SERIAL_HANDOFF_SOURCE_FILES
        .iter()
        .zip(&before)
        .map(|(name, bytes)| ((*name).to_owned(), sha(bytes)))
        .collect();
    let mut legacy_binding = binding.clone();
    legacy_binding.training_handoff_sha256 = sha(&pretty_bytes);
    legacy_binding
        .validate(&pinned, &sha(&pretty_bytes))
        .unwrap();
    assert!(binding.validate(&pinned, &sha(&pretty_bytes)).is_err());
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
    binding.validate(&pinned, &sha(&handoff_bytes)).unwrap();
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
