use alife_core::*;

fn inputs() -> (BrainGenome, DevelopmentState) {
    let g = BrainGenome::scaffold(0x2048_2026, BrainCapacityClass::N2048_ID);
    let d = DevelopmentState::new(g.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    (g, d)
}
fn graph(version: u8) -> (BrainPhenotype, PhenotypeCompilerInputs) {
    let (g, d) = inputs();
    match version {
        1 => PhenotypeCompiler::compile_n2048_experimental_graph(
            g,
            d,
            SensorProfile::GroundedObjectSlotsV1,
        ),
        2 => PhenotypeCompiler::compile_n2048_anchored_v2_graph(
            g,
            d,
            SensorProfile::GroundedObjectSlotsV1,
        ),
        3 => PhenotypeCompiler::compile_n2048_signed_motor_v3_graph(
            g,
            d,
            SensorProfile::GroundedObjectSlotsV1,
        ),
        _ => unreachable!(),
    }
    .unwrap()
}
fn asset(p: &BrainPhenotype, weights: Vec<f32>) -> FoundationWeightAsset {
    FoundationWeightAsset::from_trained_weights(p, weights, TrainingStageManifest::bootstrap())
        .unwrap()
}
fn admit(
    a: FoundationWeightAsset,
) -> Result<(BrainPhenotype, PhenotypeCompilerInputs), ScaffoldContractError> {
    let (g, d) = inputs();
    PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(g, d, a)
}
fn index(p: &BrainPhenotype, projection: ProjectionType) -> usize {
    p.synapses()
        .iter()
        .position(|s| {
            s.kind() == CompiledSynapseKind::Recurrent
                && p.projections()[s.route_index() as usize].projection_type() == projection
        })
        .unwrap()
}

#[test]
fn explicit_v3_changes_identity_not_initial_coordinates_or_weights() {
    let (v2, i2) = graph(2);
    let (v3, i3) = graph(3);
    assert_eq!(v2.synapses(), v3.synapses());
    assert_eq!(v2.sensor_encoder(), v3.sensor_encoder());
    assert_ne!(v2.phenotype_hash(), v3.phenotype_hash());
    assert_ne!(v2.route_abi_digest(), v3.route_abi_digest());
    assert_ne!(v2.plasticity_abi_digest(), v3.plasticity_abi_digest());
    assert_ne!(i2.canonical_digest(), i3.canonical_digest());
    assert_ne!(
        v2.foundation_abi().foundation_id(),
        v3.foundation_abi().foundation_id()
    );
    assert_ne!(
        v2.foundation_abi().compatibility_family_id(),
        v3.foundation_abi().compatibility_family_id()
    );
    let bytes = serde_json::to_vec(&(v3.clone(), i3.clone())).unwrap();
    let restored: (BrainPhenotype, PhenotypeCompilerInputs) =
        serde_json::from_slice(&bytes).unwrap();
    assert_eq!(restored, (v3, i3));
}

#[test]
fn only_v3_admits_negative_motor_genes_and_other_route_signs_stay_restricted() {
    for version in [1, 2, 3] {
        let (p, _) = graph(version);
        for (projection, value) in [
            (ProjectionType::MotorProposal, -0.3),
            (ProjectionType::Homeostatic, -0.3),
            (ProjectionType::LateralInhibition, 0.3),
        ] {
            let mut weights = p
                .synapses()
                .iter()
                .map(|s| s.genetic_weight())
                .collect::<Vec<_>>();
            let selected = index(&p, projection);
            weights[selected] = value;
            let a = asset(&p, weights);
            let bytes = a.encode_canonical().unwrap();
            assert_eq!(FoundationWeightAsset::decode_canonical(&bytes).unwrap(), a);
            let result = admit(a);
            if version == 3 && projection == ProjectionType::MotorProposal {
                let (bound, inputs) = result.unwrap();
                assert_eq!(
                    bound.synapses()[selected].genetic_weight().to_bits(),
                    value.to_bits()
                );
                let encoded = serde_json::to_vec(&(bound.clone(), inputs)).unwrap();
                let decoded: (BrainPhenotype, PhenotypeCompilerInputs) =
                    serde_json::from_slice(&encoded).unwrap();
                assert_eq!(decoded.0, bound);
            } else {
                assert!(
                    result.is_err(),
                    "version={version}, projection={projection:?}"
                );
            }
        }
    }
}

#[test]
fn mismatched_selector_recipe_or_asset_cannot_enable_signed_policy() {
    let (p, _) = graph(3);
    let a = asset(
        &p,
        p.synapses().iter().map(|s| s.genetic_weight()).collect(),
    );
    let recipe = N2048ExperimentalGraphV1::for_asset(&a).unwrap();
    for selection in [
        FoundationAbiSelection::N2048ExperimentalGraphV1(recipe.clone()),
        FoundationAbiSelection::N2048ExperimentalGraphV2(recipe.clone()),
        FoundationAbiSelection::N2048ExperimentalGraphV3(
            N2048ExperimentalGraphV1::fresh_anchored_v2(SensorProfile::GroundedObjectSlotsV1)
                .unwrap(),
        ),
    ] {
        let (g, d) = inputs();
        assert!(PhenotypeCompilerInputs::try_new_with_foundation_selection(
            g,
            &BrainCapacityClass::n2048(),
            d,
            SensorProfile::GroundedObjectSlotsV1,
            selection
        )
        .is_err());
    }
    // Keep valid V3 schema and digest but inject V2 asset bytes.
    let (v2, _) = graph(2);
    let v2_asset = asset(
        &v2,
        v2.synapses().iter().map(|s| s.genetic_weight()).collect(),
    );
    let mut forged = serde_json::to_value(recipe).unwrap();
    forged["foundation_bytes"] =
        serde_json::to_value(v2_asset.encode_canonical().unwrap()).unwrap();
    let (g, d) = inputs();
    let selection =
        FoundationAbiSelection::N2048ExperimentalGraphV3(serde_json::from_value(forged).unwrap());
    assert!(PhenotypeCompilerInputs::try_new_with_foundation_selection(
        g,
        &BrainCapacityClass::n2048(),
        d,
        SensorProfile::GroundedObjectSlotsV1,
        selection
    )
    .is_err());
}

#[test]
fn signed_v3_genes_survive_multiple_generations_and_v2_mating_is_rejected() {
    let identity = FoundationGeneticIdentity::new(
        FoundationId::N2048_EXPERIMENTAL_SIGNED_MOTOR_V3.raw(),
        1,
        FoundationCompatibilityFamilyId::N2048_EXPERIMENTAL_SIGNED_MOTOR.raw(),
        BrainCapacityClass::N2048_ID,
    )
    .unwrap();
    let raw = CreatureGenome::early_mammal_founder(701, identity).unwrap();
    let e = raw.express().unwrap();
    let d = e
        .development_state_at(Tick::new(u64::from(
            e.development.maturation_duration_ticks,
        )))
        .unwrap();
    let (p, _) = PhenotypeCompiler::compile_n2048_signed_motor_v3_graph(
        e.brain_genome,
        d,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let motor: Vec<_> = p
        .synapses()
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            s.kind() == CompiledSynapseKind::Recurrent
                && p.projections()[s.route_index() as usize].projection_type()
                    == ProjectionType::MotorProposal
        })
        .map(|(i, _)| i)
        .collect();
    let mut weights = p
        .synapses()
        .iter()
        .map(|s| s.genetic_weight())
        .collect::<Vec<_>>();
    for &i in &motor {
        weights[i] = match i % 3 {
            0 => -0.3,
            1 => -0.000001,
            _ => 0.000001,
        };
    }
    let source = asset(&p, weights);
    let mut mother = raw.with_n2048_foundation_candidate(source.clone()).unwrap();
    let mut father = CreatureGenome::early_mammal_founder(702, identity)
        .unwrap()
        .with_n2048_foundation_candidate(source)
        .unwrap();
    let mut negative_crossings = 0;
    let mut negative_mutations = 0;
    for generation in 0..3 {
        let before = serde_json::to_vec(&(&mother, &father)).unwrap();
        let child = CreatureGenome::reproduce(&mother, &father, 703 + generation).unwrap();
        assert_eq!(serde_json::to_vec(&(&mother, &father)).unwrap(), before);
        let a = child.n2048_foundation_candidate.as_ref().unwrap();
        assert!(a.is_n2048_signed_motor_v3());
        assert!(motor
            .iter()
            .filter(|&&i| i % 3 == 0)
            .all(|&i| a.weights()[i] < 0.0));
        negative_crossings += child.provenance.mutations.iter().filter(|m| matches!(m,
            MutationRecord::GeneticWeight { index, before, after } if motor.contains(&usize::from(*index)) && *before > 0. && *after < 0.
        )).count();
        negative_mutations += child.provenance.mutations.iter().filter(|m| matches!(m,
            MutationRecord::GeneticWeight { index, before, after } if motor.contains(&usize::from(*index)) && *before < -0.2 && *after < 0. && before.to_bits() != after.to_bits()
        )).count();
        let e = child.express().unwrap();
        let d = e
            .development_state_at(Tick::new(u64::from(
                e.development.maturation_duration_ticks,
            )))
            .unwrap();
        // Developmental mutation can add sensor assignments. Compare the same
        // child's anchored layout, not its parent's assignment count.
        let (anchored, _) = PhenotypeCompiler::compile_n2048_anchored_v2_graph(
            e.brain_genome.clone(),
            d.clone(),
            SensorProfile::GroundedObjectSlotsV1,
        )
        .unwrap();
        let (bound, _) = PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
            e.brain_genome,
            d,
            a.clone(),
        )
        .unwrap();
        assert_eq!(bound.sensor_encoder(), anchored.sensor_encoder());
        let bytes = serde_json::to_vec(&child).unwrap();
        assert_eq!(
            serde_json::from_slice::<CreatureGenome>(&bytes).unwrap(),
            child
        );
        father = mother;
        mother = child;
    }
    assert!(
        negative_crossings > 0,
        "positive motor genes must mutate through zero"
    );
    assert!(
        negative_mutations > 0,
        "negative motor genes must mutate without abs folding"
    );
    let old_id = FoundationGeneticIdentity::new(
        FoundationId::N2048_EXPERIMENTAL_ANCHORED_V2.raw(),
        1,
        FoundationCompatibilityFamilyId::N2048_EXPERIMENTAL_ANCHORED.raw(),
        BrainCapacityClass::N2048_ID,
    )
    .unwrap();
    let raw = CreatureGenome::early_mammal_founder(801, old_id).unwrap();
    let e = raw.express().unwrap();
    let d = e
        .development_state_at(Tick::new(u64::from(
            e.development.maturation_duration_ticks,
        )))
        .unwrap();
    let (p, _) = PhenotypeCompiler::compile_n2048_anchored_v2_graph(
        e.brain_genome,
        d,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let old = raw
        .with_n2048_foundation_candidate(asset(
            &p,
            p.synapses().iter().map(|s| s.genetic_weight()).collect(),
        ))
        .unwrap();
    assert!(!mother.compatible_n2048_foundation_assets(&old));
    assert!(CreatureGenome::reproduce(&mother, &old, 900).is_err());
    assert!(CreatureGenome::reproduce(&old, &mother, 900).is_err());
}
