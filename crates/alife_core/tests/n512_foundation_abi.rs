//! Contract acceptance for the immutable Nano512 bootstrap foundation.

use std::fs;
use std::path::{Path, PathBuf};

use alife_core::{
    BrainCapacityClass, BrainGenome, BrainPhenotype, DevelopmentState, FoundationAbiBinding,
    FoundationWeightAsset, NormalizedScalar, PhenotypeCompiler, PhenotypeCompilerInputs,
    SensorProfile, Tick,
};

const N512_FOUNDATION_ID: u64 = 0x004E_3531_325F_5631;
const N512_FOUNDATION_FAMILY_ID: u64 = 0x4E35_3132_5F00_FA11;
const FOUNDATION_SEED: u64 = 0x4E35_3132_5F00_0001;

fn compile_n512(sensor_profile: SensorProfile, seed: u64) -> BrainPhenotype {
    let capacity = BrainCapacityClass::n512();
    let genome = BrainGenome::scaffold(seed, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    PhenotypeCompiler::compile_testing_procedural_baseline(
        &genome,
        &capacity,
        &development,
        sensor_profile,
    )
    .unwrap()
}

fn asset_path(file_name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/brain_foundations")
        .join(file_name)
}

#[test]
fn n512_genetic_birth_asset_is_bootstrap_and_canonical() {
    let phenotype = compile_n512(SensorProfile::PrivilegedAffordanceV1, 0x5120_0001);
    let asset = FoundationWeightAsset::from_phenotype_for_genetic_birth(&phenotype).unwrap();

    assert_eq!(asset.manifest().foundation_id().raw(), N512_FOUNDATION_ID);
    assert_eq!(
        asset.manifest().compatibility_family_id().raw(),
        N512_FOUNDATION_FAMILY_ID
    );
    assert_eq!(asset.manifest().training_stage().completed_stage_count(), 0);
    assert!(!asset.manifest().promotion_receipt().is_promoted());
    assert_ne!(asset.digest().bytes(), &[0; 32]);
    assert!(asset.asset_ref().weight_count() > 0);

    let bytes = asset.encode_canonical().unwrap();
    let decoded = FoundationWeightAsset::decode_canonical(&bytes).unwrap();
    assert_eq!(decoded, asset);
    assert_eq!(decoded.encode_canonical().unwrap(), bytes);
}

#[test]
fn checked_n512_assets_are_present_and_canonically_decodable() {
    for file_name in [
        "n512-v1-privileged.alife-foundation",
        "n512-v1-grounded.alife-foundation",
    ] {
        let path = asset_path(file_name);
        assert!(path.exists(), "checked Nano512 asset is missing: {path:?}");
        let bytes = fs::read(&path).unwrap();
        let asset = FoundationWeightAsset::decode_canonical(&bytes).unwrap();
        assert_eq!(asset.encode_canonical().unwrap(), bytes);
        assert_ne!(asset.digest().bytes(), &[0; 32]);
        assert!(asset.asset_ref().weight_count() > 0);
        assert_eq!(asset.manifest().training_stage().completed_stage_count(), 0);
        assert!(!asset.manifest().promotion_receipt().is_promoted());
    }
}

#[test]
fn n512_builtins_bind_exact_profiles_and_compile_validated_matches_asset_path() {
    let capacity = BrainCapacityClass::n512();
    for sensor_profile in [
        SensorProfile::PrivilegedAffordanceV1,
        SensorProfile::GroundedObjectSlotsV1,
    ]
    .into_iter()
    {
        let asset = FoundationWeightAsset::builtin_nano512_v1(sensor_profile).unwrap();
        let genome = BrainGenome::scaffold(FOUNDATION_SEED, capacity.id());
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());

        let from_asset = PhenotypeCompiler::compile_from_foundation_asset(
            &genome,
            &capacity,
            &development,
            sensor_profile,
            &asset,
        )
        .unwrap();
        asset.validate_against(&from_asset).unwrap();
        assert_eq!(from_asset.sensor_profile(), sensor_profile);
        assert_eq!(
            from_asset.foundation_abi().capacity_class_id(),
            BrainCapacityClass::N512_ID
        );
        assert_eq!(
            from_asset.foundation_abi().foundation_id().unwrap().raw(),
            N512_FOUNDATION_ID
        );
        assert_eq!(
            from_asset
                .foundation_abi()
                .compatibility_family_id()
                .unwrap()
                .raw(),
            N512_FOUNDATION_FAMILY_ID
        );
        assert_eq!(
            from_asset.foundation_abi().foundation_payload_digest(),
            Some(asset.digest())
        );

        let inputs = PhenotypeCompilerInputs::try_new_with_foundation_abi(
            genome,
            &capacity,
            development,
            sensor_profile,
            from_asset.foundation_abi().clone(),
        )
        .unwrap();
        assert_eq!(
            PhenotypeCompiler::compile_validated(&inputs, &capacity).unwrap(),
            from_asset
        );
    }
}

#[test]
fn n512_foundations_reject_class_profile_and_forged_cross_wires() {
    let n512 = BrainCapacityClass::n512();
    let n1024 = BrainCapacityClass::n1024();
    let n2048 = BrainCapacityClass::n2048();
    let privileged =
        FoundationWeightAsset::builtin_nano512_v1(SensorProfile::PrivilegedAffordanceV1).unwrap();
    let grounded =
        FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();

    assert!(FoundationAbiBinding::canonical_for_foundation_asset(&n1024, &privileged).is_err());
    assert!(FoundationAbiBinding::canonical_for_foundation_asset(&n2048, &privileged).is_err());

    let n512_genome = BrainGenome::scaffold(0x5120_2000, n512.id());
    let n512_development = DevelopmentState::new(
        n512_genome.id,
        Tick::ZERO,
        NormalizedScalar::new(1.0).unwrap(),
    );
    assert!(PhenotypeCompiler::compile_from_foundation_asset(
        &n512_genome,
        &n512,
        &n512_development,
        SensorProfile::GroundedObjectSlotsV1,
        &privileged,
    )
    .is_err());
    let privileged_binding =
        FoundationAbiBinding::canonical_for_foundation_asset(&n512, &privileged).unwrap();
    let mismatched_inputs = PhenotypeCompilerInputs::try_new_with_foundation_abi(
        n512_genome.clone(),
        &n512,
        n512_development.clone(),
        SensorProfile::GroundedObjectSlotsV1,
        privileged_binding,
    )
    .unwrap();
    assert!(PhenotypeCompiler::compile_validated(&mismatched_inputs, &n512).is_err());

    let n1024_genome = BrainGenome::scaffold(0x5120_2001, n1024.id());
    let n1024_development = DevelopmentState::new(
        n1024_genome.id,
        Tick::ZERO,
        NormalizedScalar::new(1.0).unwrap(),
    );
    assert!(PhenotypeCompiler::compile_from_foundation_asset(
        &n1024_genome,
        &n1024,
        &n1024_development,
        SensorProfile::PrivilegedAffordanceV1,
        &privileged,
    )
    .is_err());

    let n2048_genome = BrainGenome::scaffold(0x5120_2002, n2048.id());
    let n2048_development = DevelopmentState::new(
        n2048_genome.id,
        Tick::ZERO,
        NormalizedScalar::new(1.0).unwrap(),
    );
    assert!(PhenotypeCompiler::compile_from_foundation_asset(
        &n2048_genome,
        &n2048,
        &n2048_development,
        SensorProfile::PrivilegedAffordanceV1,
        &privileged,
    )
    .is_err());

    let mut wrong_magic = privileged.encode_canonical().unwrap();
    wrong_magic[..8].copy_from_slice(b"ALFN2048");
    assert!(FoundationWeightAsset::decode_canonical(&wrong_magic).is_err());

    let canonical = serde_json::to_value(&grounded).unwrap();
    for class_id in [
        BrainCapacityClass::N1024_ID.raw(),
        BrainCapacityClass::N4096_RESEARCH_ID.raw(),
        u16::MAX,
    ] {
        let mut forged = canonical.clone();
        forged["manifest"]["capacity_class_id"] = serde_json::json!(class_id);
        let forged: FoundationWeightAsset = serde_json::from_value(forged).unwrap();
        assert!(FoundationAbiBinding::canonical_for_foundation_asset(&n512, &forged).is_err());
    }
}

#[test]
fn n2048_builtin_payloads_retain_legacy_digests_and_canonical_bytes() {
    let expected = [
        (
            SensorProfile::PrivilegedAffordanceV1,
            "n2048-v1-privileged.alife-foundation",
            [
                0x54, 0x23, 0x1c, 0xa9, 0x5e, 0x6b, 0x9f, 0x65, 0xb9, 0x6e, 0x8b, 0x09, 0x68, 0x2d,
                0x72, 0x6a, 0x85, 0x60, 0x68, 0xc6, 0xb7, 0xaf, 0xe4, 0x0a, 0x31, 0xa6, 0xf1, 0xda,
                0x4f, 0x76, 0xba, 0x1a,
            ],
        ),
        (
            SensorProfile::GroundedObjectSlotsV1,
            "n2048-v1-grounded.alife-foundation",
            [
                0xd5, 0xc6, 0x9f, 0x36, 0x5b, 0x83, 0xf4, 0x6a, 0xbb, 0xe6, 0x00, 0x43, 0x26, 0x04,
                0x2b, 0x78, 0x05, 0xcf, 0x00, 0x0d, 0x0e, 0x7d, 0x0a, 0x63, 0xf9, 0x19, 0xd6, 0x28,
                0x4d, 0xc6, 0x6e, 0x11,
            ],
        ),
    ];

    for (sensor_profile, file_name, digest) in expected {
        let path = asset_path(file_name);
        let bytes = fs::read(&path).unwrap();
        let asset = FoundationWeightAsset::builtin_n2048_v1(sensor_profile).unwrap();
        assert_eq!(asset.digest().bytes(), &digest);
        assert_eq!(asset.encode_canonical().unwrap(), bytes);
    }
}
