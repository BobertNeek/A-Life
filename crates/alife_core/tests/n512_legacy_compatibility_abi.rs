use alife_core::{
    BrainCapacityClass, BrainGenome, CompiledSynapseKind, DevelopmentState, FoundationWeightAsset,
    LegacyFoundationAbiId, NormalizedScalar, PhenotypeCompiler, ProductionRuntimeAbiId,
    ProductionRuntimePath, SensorProfile, Tick,
};

const FOUNDATION_SEED: u64 = 0x4E35_3132_5F00_0001;
const EXPECTED_ASSET_DIGEST: &str =
    "43e28117ab6f54f24361d737ae6c12c82dd9ef4cd63a2bd91158de6ecc800cfc";
const EXPECTED_ENDPOINT_DIGEST: &str =
    "d565d5b4b41f6fbc906f58ddab6ed6e3e63d48db9f26b82c3eaea8cadfefc0e9";
const EXPECTED_GRAPH_WEIGHT_DIGEST: &str =
    "25de49f6a7c8e5a1ac0221e22669b0b52f5af567f6f18678383a1e10fc63547c";

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn inputs() -> (
    BrainCapacityClass,
    BrainGenome,
    DevelopmentState,
    SensorProfile,
) {
    let capacity = BrainCapacityClass::n512();
    let genome = BrainGenome::scaffold(FOUNDATION_SEED, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    (
        capacity,
        genome,
        development,
        SensorProfile::GroundedObjectSlotsV1,
    )
}

#[test]
fn immutable_nano512_enters_only_through_explicit_legacy_abi_without_reindexing() {
    let (capacity, genome, development, profile) = inputs();
    let asset = FoundationWeightAsset::builtin_nano512_v1(profile).unwrap();

    assert!(PhenotypeCompiler::compile_from_foundation_asset(
        &genome,
        &capacity,
        &development,
        profile,
        &asset,
    )
    .is_err());

    let admission = PhenotypeCompiler::compile_from_legacy_nano512_compatibility_asset(
        &genome,
        &capacity,
        &development,
        profile,
        &asset,
    )
    .unwrap();
    let (phenotype, receipt) = admission.into_parts();
    phenotype.validate_against(&capacity).unwrap();

    let descriptor = phenotype
        .legacy_foundation_compatibility_abi()
        .expect("legacy compatibility must be explicit in phenotype state");
    assert_eq!(
        descriptor.source_abi_id(),
        LegacyFoundationAbiId::NANO512_V1
    );
    assert_eq!(descriptor.runtime_abi_id(), ProductionRuntimeAbiId::V2);
    assert_eq!(
        descriptor.runtime_path(),
        ProductionRuntimePath::ORDINARY_GPU_ORGANISM_V2
    );
    assert_eq!(receipt.source_abi_id(), LegacyFoundationAbiId::NANO512_V1);
    assert_eq!(receipt.runtime_abi_id(), ProductionRuntimeAbiId::V2);
    assert_eq!(
        receipt.runtime_path(),
        ProductionRuntimePath::ORDINARY_GPU_ORGANISM_V2
    );
    receipt.validate_against(&phenotype, &asset).unwrap();
    let mut forged_asset_json = serde_json::to_value(&asset).unwrap();
    forged_asset_json["weights"][0] = serde_json::json!(0.125);
    let forged_asset: FoundationWeightAsset = serde_json::from_value(forged_asset_json).unwrap();
    assert!(receipt.validate_against(&phenotype, &forged_asset).is_err());
    let receipt_json = serde_json::to_value(&receipt).unwrap();
    let decoded_receipt: alife_core::LegacyNano512CompatibilityReceipt =
        serde_json::from_value(receipt_json.clone()).unwrap();
    assert_eq!(decoded_receipt, receipt);
    let mut tampered_receipt = receipt_json.clone();
    tampered_receipt["receipt_digest"] = serde_json::json!([0, 0, 0, 0]);
    assert!(
        serde_json::from_value::<alife_core::LegacyNano512CompatibilityReceipt>(tampered_receipt)
            .is_err()
    );
    let mut unknown_receipt = receipt_json;
    unknown_receipt["descriptor"]["source_abi_id"] = serde_json::json!(0);
    assert!(
        serde_json::from_value::<alife_core::LegacyNano512CompatibilityReceipt>(unknown_receipt)
            .is_err()
    );

    let encoded = serde_json::to_value(&phenotype).unwrap();
    let restored: alife_core::BrainPhenotype = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(
        restored.foundation_abi_selection(),
        phenotype.foundation_abi_selection()
    );
    assert_eq!(restored.phenotype_hash(), phenotype.phenotype_hash());
    receipt.validate_against(&restored, &asset).unwrap();

    let mut omitted = encoded.clone();
    omitted
        .as_object_mut()
        .unwrap()
        .remove("foundation_abi_selection");
    assert!(serde_json::from_value::<alife_core::BrainPhenotype>(omitted).is_err());
    let mut unknown = encoded;
    unknown["foundation_abi_selection"]["abi_selection"] =
        serde_json::json!("LegacyNano512CompatibilityV9");
    assert!(serde_json::from_value::<alife_core::BrainPhenotype>(unknown).is_err());

    assert_eq!(phenotype.synapses().len(), 1_799);
    assert_eq!(hex(asset.digest().bytes()), EXPECTED_ASSET_DIGEST);
    assert!(phenotype
        .synapses()
        .iter()
        .zip(asset.weights())
        .all(|(synapse, weight)| synapse.genetic_weight().to_bits() == weight.to_bits()));

    let mut endpoint_hasher = blake3::Hasher::new();
    endpoint_hasher.update(b"alife.audit.nano512.endpoints.v1");
    let mut graph_hasher = blake3::Hasher::new();
    graph_hasher.update(b"alife.audit.nano512.graph-and-weights.v1");
    for (index, (synapse, weight)) in phenotype.synapses().iter().zip(asset.weights()).enumerate() {
        let index = u32::try_from(index).unwrap();
        for hasher in [&mut endpoint_hasher, &mut graph_hasher] {
            hasher.update(&index.to_le_bytes());
            hasher.update(&synapse.source().to_le_bytes());
            hasher.update(&synapse.target().to_le_bytes());
            hasher.update(&synapse.route_index().to_le_bytes());
            hasher.update(&synapse.kind().kind_raw().to_le_bytes());
            if let CompiledSynapseKind::Decoder(coordinate) = synapse.kind() {
                hasher.update(&coordinate.head().raw().to_le_bytes());
                hasher.update(&[coordinate.family().raw()]);
                hasher.update(&coordinate.input_lane().to_le_bytes());
                hasher.update(&coordinate.motor_index().to_le_bytes());
            }
        }
        graph_hasher.update(&weight.to_bits().to_le_bytes());
    }
    assert_eq!(
        endpoint_hasher.finalize().to_hex().to_string(),
        EXPECTED_ENDPOINT_DIGEST
    );
    assert_eq!(
        graph_hasher.finalize().to_hex().to_string(),
        EXPECTED_GRAPH_WEIGHT_DIGEST
    );
}

#[test]
fn malformed_and_unknown_assets_fail_closed_without_canonical_retry() {
    let (capacity, genome, development, profile) = inputs();
    let asset = FoundationWeightAsset::builtin_nano512_v1(profile).unwrap();

    let mut malformed_json = serde_json::to_value(&asset).unwrap();
    malformed_json["digest"] = serde_json::json!(vec![0_u8; 32]);
    let malformed: FoundationWeightAsset = serde_json::from_value(malformed_json).unwrap();
    assert!(
        PhenotypeCompiler::compile_from_legacy_nano512_compatibility_asset(
            &genome,
            &capacity,
            &development,
            profile,
            &malformed,
        )
        .is_err()
    );

    let unknown = FoundationWeightAsset::builtin_n2048_v1(profile).unwrap();
    assert!(
        PhenotypeCompiler::compile_from_legacy_nano512_compatibility_asset(
            &genome,
            &capacity,
            &development,
            profile,
            &unknown,
        )
        .is_err()
    );
}
