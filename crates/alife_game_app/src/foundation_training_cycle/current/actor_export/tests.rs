use super::*;

fn seeds() -> alife_training::CheckpointSeedsV1 {
    alife_training::CheckpointSeedsV1 {
        historical_freshness: "unknown".into(),
        excluded_ranges: vec![[1, u64::MAX]],
        pending_range: None,
    }
}

#[test]
fn actor_export_preserves_source_and_all_weight_bits_without_copying_private_or_adam_state() {
    let f =
        crate::foundation_critic_correction::tests::full_residual_correction_fixture_with_source(
            portable_handoff::tests::private_fixture,
        );
    let checkpoint = migrate_current_training_checkpoint(&f.target, seeds()).unwrap();
    let original = checkpoint.encode().unwrap();
    let source = f.root.join("disposable-current.alife-checkpoint");
    fs::write(&source, &original).unwrap();
    let output = f.root.join("actor-export");
    let report = export_current_actor_asset(&source, &output).unwrap();
    let bytes = fs::read(output.join("actor.alife-foundation")).unwrap();
    let exported = FoundationWeightAsset::decode_canonical(&bytes).unwrap();
    let expected = checkpoint
        .handoff
        .compiler_inputs
        .n2048_candidate_asset()
        .unwrap();
    assert_eq!(bytes, expected.encode_canonical().unwrap());
    assert_eq!(exported.digest(), expected.digest());
    assert_eq!(
        exported.weights().len(),
        checkpoint.handoff.actor.weights.len()
    );
    for (a, b) in exported
        .weights()
        .iter()
        .zip(&checkpoint.handoff.actor.weights)
    {
        assert_eq!(a.to_bits(), b.to_bits());
    }
    assert_eq!(fs::read(&source).unwrap(), original);
    assert_eq!(
        read_current_training_checkpoint(&source).unwrap(),
        checkpoint
    );
    assert_eq!(report["source_current_sha256"], sha(&original));
    assert_eq!(report["asset_sha256"], sha(&bytes));
    assert_eq!(report["all_actor_weight_bits_verified"], true);
    assert_eq!(report["private_state_exported"], false);
    assert_eq!(report["optimizer_state_exported"], false);
    let committed: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("actor-export.json")).unwrap()).unwrap();
    assert_eq!(committed, report);
    assert_eq!(fs::read_dir(&output).unwrap().count(), 2);
    assert!(export_current_actor_asset(&source, &output).is_err());
    assert_eq!(
        fs::read(output.join("actor.alife-foundation")).unwrap(),
        bytes
    );
    assert!(export_current_actor_asset(&source, &source).is_err());
    assert!(export_current_actor_asset(&source, &f.root).is_err());
    assert_eq!(fs::read(&source).unwrap(), original);

    // A broken full-state envelope must fail before any export is created.
    let mut corrupt = original.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    let bad = f.root.join("corrupt-current");
    fs::write(&bad, corrupt).unwrap();
    let rejected = f.root.join("rejected-export");
    assert!(export_current_actor_asset(&bad, &rejected).is_err());
    assert!(!rejected.exists());
    let mut no_life = checkpoint.clone();
    no_life.life = None;
    fs::write(&bad, no_life.encode().unwrap()).unwrap();
    assert!(export_current_actor_asset(&bad, &rejected).is_err());
    assert!(!rejected.exists());
    let mut corrupt_life = checkpoint;
    corrupt_life
        .life
        .as_mut()
        .unwrap()
        .insert_asset("private-sidecar.bin".into(), b"corrupt");
    fs::write(&bad, corrupt_life.encode().unwrap()).unwrap();
    assert!(export_current_actor_asset(&bad, &rejected).is_err());
    assert!(!rejected.exists());
}

#[test]
fn actor_export_checks_signed_zero_subnormal_bits_and_same_individual_overlay_identity() {
    use alife_core::{
        BrainCapacityClass, N2048LivePolicyProvenanceV1, PhenotypeCompiler, PhenotypeCompilerInputs,
    };
    let f =
        crate::foundation_critic_correction::tests::full_residual_correction_fixture_with_source(
            portable_handoff::tests::private_fixture,
        );
    let original = migrate_current_training_checkpoint(&f.target, seeds())
        .unwrap()
        .handoff;
    let admitted = original.clone().admit().unwrap();
    let mut weights = admitted.actor.weights.clone();
    let samples = [
        0.0_f32,
        -0.0,
        f32::from_bits(1),
        -f32::from_bits(1),
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
    ];
    weights[..samples.len()].copy_from_slice(&samples);
    let asset = FoundationWeightAsset::from_trained_weights(
        &admitted.phenotype,
        weights,
        admitted.foundation.manifest().training_stage(),
    )
    .unwrap();
    let inputs = PhenotypeCompilerInputs::try_new_with_live_n2048_policy(
        original.compiler_inputs.genome().clone(),
        &BrainCapacityClass::n2048(),
        original.compiler_inputs.development().clone(),
        original.compiler_inputs.sensor_profile(),
        asset.clone(),
        N2048LivePolicyProvenanceV1 {
            schema: 1,
            birth_foundation_digest: admitted.foundation.digest(),
            policy_version: original.policy_version,
        },
    )
    .unwrap();
    let phenotype =
        PhenotypeCompiler::compile_validated(&inputs, &BrainCapacityClass::n2048()).unwrap();
    let mut actor = admitted.actor;
    actor.weights = asset.weights().to_vec();
    actor.phenotype_hash = phenotype.phenotype_hash();
    actor.source_foundation_digest = asset.digest();
    let same = alife_training::FoundationTrainingHandoffV1::new_same_individual(
        original.origin,
        original.policy_version,
        inputs,
        actor,
        original.value,
    )
    .unwrap();
    let before = serde_json::to_vec(&same).unwrap();
    assert_eq!(
        exact_actor_asset(&same).unwrap(),
        asset.encode_canonical().unwrap()
    );
    assert_eq!(serde_json::to_vec(&same).unwrap(), before);
    for (i, value) in samples.iter().enumerate() {
        let mut wrong = same.clone();
        wrong.actor.weights[i] = f32::from_bits(value.to_bits() ^ 1);
        assert!(exact_actor_asset(&wrong).is_err(), "changed bit at {i}");
    }
    let mut wrong = same;
    wrong.actor.weights[1] = 0.0;
    assert!(
        exact_actor_asset(&wrong).is_err(),
        "signed-zero change must reject"
    );
}
