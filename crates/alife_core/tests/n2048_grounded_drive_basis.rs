use alife_core::*;
fn inputs(class: BrainClassId) -> (BrainGenome, DevelopmentState) {
    let g = BrainGenome::scaffold(0x2048_2026, class);
    let d = DevelopmentState::new(g.id, Tick::ZERO, NormalizedScalar::new(1.).unwrap());
    (g, d)
}
fn candidate() -> (BrainPhenotype, PhenotypeCompilerInputs) {
    let (g, d) = inputs(BrainCapacityClass::N2048_GROUNDED_DRIVE_RESEARCH_ID);
    PhenotypeCompiler::compile_n2048_grounded_drive_basis(
        g,
        d,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap()
}
#[test]
fn explicit_additive_capacity_and_exact_coordinates() {
    let (p, _) = candidate();
    let cap = BrainCapacityClass::n2048_grounded_drive_research();
    p.validate_against(&cap).unwrap();
    assert_eq!(p.synapses().len(), 34048);
    assert_eq!(cap.execution().max_total_synapses(), 34112);
    assert_eq!(cap.execution().max_action_decoder_synapses(), 5376);
    assert!(BrainCapacityClass::production_for_id(cap.id()).is_err());
    assert!(p.cognitive_decoder().is_none());
    assert_eq!(p.candidate_decoder().schema_version(), 2);
    assert_eq!(p.candidate_decoder().flattened_input_lane_count(), 54);
    assert_eq!(p.foundation_abi().fresh_dendritic_parameters(), (0.25, 0.1));
    let mut count = 0;
    for f in 0_u32..8 {
        for lane in 0_u32..34 {
            for motor in 0_u32..16 {
                let id = 24576 + 544 * f + 16 * lane + motor;
                let s = &p.synapses()[id as usize];
                let CompiledSynapseKind::Decoder(c) = s.kind() else {
                    panic!("not decoder")
                };
                assert_eq!(c.head(), DecoderHeadKind::ActionCandidate);
                assert_eq!(u32::from(c.family().raw()), f);
                assert_eq!(
                    u32::from(c.input_lane()),
                    if lane < 24 { lane } else { lane + 12 }
                );
                assert_eq!(u32::from(c.motor_index()), 16 * f + motor);
                assert_eq!(
                    (s.source(), s.target()),
                    (1728 + 16 * f + motor, 1728 + 16 * f + motor)
                );
                if lane >= 24 {
                    assert_eq!(s.genetic_weight().to_bits(), 0_f32.to_bits());
                    count += 1;
                }
            }
        }
    }
    assert_eq!(count, 1280);
}
#[test]
fn no_old_asset_relabeling_or_default_admission() {
    let (g, d) = inputs(BrainCapacityClass::N2048_ID);
    let old = PhenotypeCompiler::compile_n2048_bounded_founder(
        g.clone(),
        d.clone(),
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap()
    .0;
    let old_json = serde_json::to_vec(&old).unwrap();
    let old_asset = FoundationWeightAsset::from_phenotype_for_genetic_birth(&old).unwrap();
    let (p, i) = candidate();
    let new_asset = FoundationWeightAsset::from_phenotype_for_genetic_birth(&p).unwrap();
    assert!(N2048GroundedDriveBasisV1::for_asset(&old_asset).is_err());
    assert!(N2048ExperimentalGraphV1::for_asset(&new_asset).is_err());
    assert_ne!(
        old_asset.encode_canonical().unwrap()[..8],
        new_asset.encode_canonical().unwrap()[..8]
    );
    assert_eq!(
        new_asset,
        FoundationWeightAsset::decode_canonical(&new_asset.encode_canonical().unwrap()).unwrap()
    );
    assert!(p.validate_against(&BrainCapacityClass::n2048()).is_err());
    let (ng, nd) = inputs(BrainCapacityClass::N2048_GROUNDED_DRIVE_RESEARCH_ID);
    assert!(PhenotypeCompiler::compile(
        &ng,
        &BrainCapacityClass::n2048_grounded_drive_research(),
        &nd,
        SensorProfile::GroundedObjectSlotsV1
    )
    .is_err());
    let (q, _) =
        PhenotypeCompiler::compile_n2048_grounded_drive_basis_candidate(ng, nd, new_asset.clone())
            .unwrap();
    new_asset.validate_against(&q).unwrap();
    let round: PhenotypeCompilerInputs =
        serde_json::from_slice(&serde_json::to_vec(&i).unwrap()).unwrap();
    assert_eq!(
        p,
        PhenotypeCompiler::compile_validated(
            &round,
            &BrainCapacityClass::n2048_grounded_drive_research()
        )
        .unwrap()
    );
    let old_round: BrainPhenotype = serde_json::from_slice(&old_json).unwrap();
    assert_eq!(old, old_round);
    assert_eq!(
        old_json,
        serde_json::to_vec(
            &PhenotypeCompiler::compile_n2048_bounded_founder(
                g,
                d,
                SensorProfile::GroundedObjectSlotsV1
            )
            .unwrap()
            .0
        )
        .unwrap()
    );
}
#[test]
fn cross_profile_and_selector_tamper_rejected() {
    let (_, i) = candidate();
    let mut value = serde_json::to_value(&i).unwrap();
    value["sensor_profile"] = serde_json::to_value(SensorProfile::GroundedTerrainVisionV1).unwrap();
    let altered = serde_json::from_value::<PhenotypeCompilerInputs>(value);
    assert!(
        altered.is_err()
            || PhenotypeCompiler::compile_validated(
                &altered.unwrap(),
                &BrainCapacityClass::n2048_grounded_drive_research()
            )
            .is_err()
    );
    let (p, _) = candidate();
    let bytes = serde_json::to_string(&p)
        .unwrap()
        .replace("N2048GroundedDriveBasisV1", "N2048BoundedFounderV1");
    let altered = serde_json::from_str::<BrainPhenotype>(&bytes);
    assert!(
        altered.is_err()
            || altered
                .unwrap()
                .validate_against(&BrainCapacityClass::n2048_grounded_drive_research())
                .is_err()
    );
}
