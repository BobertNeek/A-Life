//! Pure compiler/provenance checks. No device, world or trained founder is used.
use alife_core::{
    BrainCapacityClass, BrainGenome, DevelopmentState, FoundationWeightAsset,
    N2048LivePolicyTransition, NormalizedScalar, PhenotypeCompiler, PhenotypeCompilerInputs,
    SensorProfile, Tick, TrainingStageManifest,
};

fn fixture() -> (PhenotypeCompilerInputs, FoundationWeightAsset) {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(0x10_11_09, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    let initial = PhenotypeCompiler::compile_testing_procedural_baseline(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let asset = FoundationWeightAsset::from_phenotype_for_genetic_birth(&initial).unwrap();
    let (compiled, inputs) =
        PhenotypeCompiler::compile_n2048_foundation_candidate(genome, development, asset).unwrap();
    let mut weights = compiled
        .synapses()
        .iter()
        .map(|s| s.genetic_weight())
        .collect::<Vec<_>>();
    weights[0] += 0.001;
    let trained = FoundationWeightAsset::from_trained_weights(
        &compiled,
        weights,
        TrainingStageManifest::new(1, 1, 1),
    )
    .unwrap();
    (inputs, trained)
}

#[test]
fn overlay_changes_only_learned_weights_and_bound_policy_identity() {
    let (before, trained) = fixture();
    let transition = N2048LivePolicyTransition::prepare(&before, 7, trained.clone()).unwrap();
    assert_eq!(transition.previous_policy(), 7);
    assert_eq!(transition.next_policy(), 8);
    assert_eq!(transition.next_inputs().genome(), before.genome());
    assert_eq!(transition.next_inputs().development(), before.development());
    assert_eq!(
        transition.next_inputs().sensor_profile(),
        before.sensor_profile()
    );
    let provenance = transition.next_inputs().live_policy().unwrap();
    assert_eq!(provenance.policy_version, 8);
    assert_eq!(
        provenance.birth_foundation_digest,
        before.n2048_candidate_asset().unwrap().digest()
    );
    assert_ne!(
        transition.previous_phenotype().phenotype_hash(),
        transition.next_phenotype().phenotype_hash()
    );
    assert_eq!(
        transition.next_phenotype().synapses()[0]
            .genetic_weight()
            .to_bits(),
        trained.weights()[0].to_bits()
    );
    let encoded = serde_json::to_vec(transition.next_inputs()).unwrap();
    let restored: PhenotypeCompilerInputs = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(restored, *transition.next_inputs());
    assert_eq!(
        PhenotypeCompiler::compile_validated(&restored, &BrainCapacityClass::n2048()).unwrap(),
        *transition.next_phenotype()
    );
}

#[test]
fn existing_inputs_stay_unmarked_and_roundtrip_exactly() {
    let (before, _) = fixture();
    let encoded = serde_json::to_vec(&before).unwrap();
    assert!(!String::from_utf8(encoded.clone())
        .unwrap()
        .contains("live_policy"));
    let restored: PhenotypeCompilerInputs = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(before, restored);
    assert!(restored.live_policy().is_none());
}

#[test]
fn stale_policy_overflow_and_forged_provenance_are_rejected() {
    let (before, trained) = fixture();
    assert!(N2048LivePolicyTransition::prepare(&before, u64::MAX, trained.clone()).is_err());
    let transition = N2048LivePolicyTransition::prepare(&before, 7, trained.clone()).unwrap();
    assert!(
        N2048LivePolicyTransition::prepare(transition.next_inputs(), 7, trained.clone()).is_err()
    );
    let mut forged = serde_json::to_value(transition.next_inputs()).unwrap();
    forged["live_policy"]["policy_version"] = serde_json::json!(9);
    assert!(serde_json::from_value::<PhenotypeCompilerInputs>(forged).is_err());
    let again = N2048LivePolicyTransition::prepare(transition.next_inputs(), 8, trained).unwrap();
    assert_eq!(again.next_policy(), 9);
    assert_eq!(
        again
            .next_inputs()
            .live_policy()
            .unwrap()
            .birth_foundation_digest,
        transition
            .next_inputs()
            .live_policy()
            .unwrap()
            .birth_foundation_digest
    );
}
