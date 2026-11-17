use alife_core::*;
fn digest<T: serde::Serialize>(v: &T) -> String {
    blake3::hash(&serde_json::to_vec(v).unwrap())
        .to_hex()
        .to_string()
}
#[test]
fn legacy_serialization_witness() {
    let mut rows = Vec::new();
    for capacity in [BrainCapacityClass::n512(), BrainCapacityClass::n2048()] {
        let g = BrainGenome::scaffold(0x2048_2026, capacity.id());
        let d = DevelopmentState::new(g.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
        let i = PhenotypeCompilerInputs::try_new(
            g.clone(),
            &capacity,
            d.clone(),
            SensorProfile::GroundedObjectSlotsV1,
        )
        .unwrap();
        let p = PhenotypeCompiler::compile_validated(&i, &capacity).unwrap();
        rows.push(digest(&(p, i)));
        if capacity.id() == BrainCapacityClass::N2048_ID {
            let a = FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1)
                .unwrap();
            let p = PhenotypeCompiler::compile_from_foundation_asset(
                &g,
                &capacity,
                &d,
                SensorProfile::GroundedObjectSlotsV1,
                &a,
            )
            .unwrap();
            rows.push(digest(&p));
            rows.push(
                blake3::hash(&a.encode_canonical().unwrap())
                    .to_hex()
                    .to_string(),
            );
            for compile in [
                PhenotypeCompiler::compile_n2048_experimental_graph,
                PhenotypeCompiler::compile_n2048_anchored_v2_graph,
            ] {
                let (p, i) =
                    compile(g.clone(), d.clone(), SensorProfile::GroundedObjectSlotsV1).unwrap();
                let a = FoundationWeightAsset::from_trained_weights(
                    &p,
                    p.synapses().iter().map(|s| s.genetic_weight()).collect(),
                    TrainingStageManifest::bootstrap(),
                )
                .unwrap();
                rows.push(digest(&(p, i)));
                rows.push(
                    blake3::hash(&a.encode_canonical().unwrap())
                        .to_hex()
                        .to_string(),
                );
            }
        }
    }
    let a =
        FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    let (p, i, r) = PhenotypeCompiler::compile_fixed_legacy_nano512_compatibility_asset(
        SensorProfile::GroundedObjectSlotsV1,
        &a,
    )
    .unwrap()
    .into_runtime_parts();
    rows.push(digest(&(p, i, r)));
    rows.push(
        blake3::hash(&a.encode_canonical().unwrap())
            .to_hex()
            .to_string(),
    );
    let identity = FoundationGeneticIdentity::new(
        FoundationId::N2048_EXPERIMENTAL_ANCHORED_V2.raw(),
        1,
        FoundationCompatibilityFamilyId::N2048_EXPERIMENTAL_ANCHORED.raw(),
        BrainCapacityClass::N2048_ID,
    )
    .unwrap();
    // This witness predates the explicit procedural-distance birth prior.
    // Reconstruct the historical parents, rather than today's new founders.
    let mut raw = CreatureGenome::early_mammal_founder(701, identity).unwrap();
    raw.brain.construction.maternal_distance_gain = 1.0;
    raw.brain.construction.paternal_distance_gain = 1.0;
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
    let a = FoundationWeightAsset::from_trained_weights(
        &p,
        p.synapses().iter().map(|s| s.genetic_weight()).collect(),
        TrainingStageManifest::bootstrap(),
    )
    .unwrap();
    let mother = raw.with_n2048_foundation_candidate(a.clone()).unwrap();
    let mut father = CreatureGenome::early_mammal_founder(702, identity).unwrap();
    father.brain.construction.maternal_distance_gain = 1.0;
    father.brain.construction.paternal_distance_gain = 1.0;
    let father = father.with_n2048_foundation_candidate(a).unwrap();
    rows.push(digest(
        &CreatureGenome::reproduce(&mother, &father, 703).unwrap(),
    ));
    println!("LEGACY_WITNESS={}", serde_json::to_string(&rows).unwrap());
    // Captured on unchanged main 1c74ee33528f452895ae255caaac916b722d909c.
    // N512/N2048 defaults, migrated foundation, experimental V1/V2, legacy N512,
    // canonical assets and a V2 child's full mutation provenance.
    let expected: Vec<String> =
        serde_json::from_str(include_str!("fixtures/signed_motor_legacy_witness.json")).unwrap();
    assert_eq!(rows, expected);
}
