use alife_core::*;
use alife_training::transfer_bounded_genetic_founder_to_grounded_drive;
#[test]
fn complete_address_transfer_preserves_negative_readouts_and_causal_parameters() {
    let g = BrainGenome::scaffold(0x2048_2026, BrainCapacityClass::N2048_ID);
    let d = DevelopmentState::new(g.id, Tick::ZERO, NormalizedScalar::new(1.).unwrap());
    let (fresh, _) = PhenotypeCompiler::compile_n2048_bounded_founder(
        g.clone(),
        d.clone(),
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let mut weights = fresh
        .synapses()
        .iter()
        .map(|s| s.genetic_weight())
        .collect::<Vec<_>>();
    let selected=fresh.synapses().iter().position(|s| matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate && c.family()==CandidateActionFamily::Approach && c.input_lane()==15)).unwrap();
    weights[selected] = -0.37;
    let source = FoundationWeightAsset::from_trained_weights(
        &fresh,
        weights,
        TrainingStageManifest::bootstrap(),
    )
    .unwrap();
    let (p, i) =
        PhenotypeCompiler::compile_n2048_bounded_founder_candidate(g, d, source.clone()).unwrap();
    let before = serde_json::to_vec(&i).unwrap();
    let result = transfer_bounded_genetic_founder_to_grounded_drive(&p, &i, &source).unwrap();
    assert_eq!(result.source_to_target.len(), 32768);
    assert_eq!(result.added_zero_rows, 1280);
    assert_eq!(result.source_asset_digest, source.digest());
    let (_, new) = result
        .source_to_target
        .iter()
        .find(|(old, _)| *old as usize == selected)
        .unwrap();
    assert_ne!(*new as usize, selected);
    assert_eq!(
        result.asset.weights()[*new as usize].to_bits(),
        (-0.37_f32).to_bits()
    );
    for (old, new) in result.source_to_target {
        assert_eq!(
            source.weights()[old as usize].to_bits(),
            result.asset.weights()[new as usize].to_bits()
        );
    }
    assert_eq!(before, serde_json::to_vec(&i).unwrap());
    assert_eq!(
        i.genome().genetic_prior_seed,
        result.inputs.genome().genetic_prior_seed
    );
    assert_eq!(p.neuron_dynamics(), result.phenotype.neuron_dynamics());
}

#[test]
fn fresh_unbound_source_is_rejected_with_no_implicit_relabeling() {
    let g = BrainGenome::scaffold(61_100, BrainCapacityClass::N2048_ID);
    let d = DevelopmentState::new(g.id, Tick::ZERO, NormalizedScalar::new(1.).unwrap());
    let (p, i) = PhenotypeCompiler::compile_n2048_bounded_founder(
        g,
        d,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let asset = FoundationWeightAsset::from_phenotype_for_genetic_birth(&p).unwrap();
    assert!(transfer_bounded_genetic_founder_to_grounded_drive(&p, &i, &asset).is_err());
}
