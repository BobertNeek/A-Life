//! Focused RED/GREEN coverage for the explicit Nano512 founder projection.

use std::fs;
use std::path::Path;

use alife_core::{
    Blake3Digest, BrainCapacityClass, BrainGenome, CanonicalDigestBuilder, CreatureGenome,
    CreaturePhenotype, DevelopmentState, FoundationGeneticIdentity, FoundationWeightAsset,
    LobeRatioPlan, N512FounderFoundationProjection, N512FounderProjectionReceipt, NormalizedScalar,
    PhenotypeCompiler, ScaffoldContractError, SensorProfile, Tick, TrainingStageManifest,
};

const N512_FOUNDATION_ID: u64 = 0x004E_3531_325F_5631;
const N512_FOUNDATION_FAMILY_ID: u64 = 0x4E35_3132_5F00_FA11;
const N512_COORDINATE_RECIPE_SEED: u64 = 0x4E35_3132_5F00_0001;

fn n512_founder(seed: u64) -> CreaturePhenotype {
    let foundation = FoundationGeneticIdentity::new(
        N512_FOUNDATION_ID,
        1,
        N512_FOUNDATION_FAMILY_ID,
        BrainCapacityClass::N512_ID,
    )
    .unwrap();
    CreatureGenome::early_mammal_founder(seed, foundation)
        .unwrap()
        .express()
        .unwrap()
}

fn compile(
    phenotype: &CreaturePhenotype,
    sensor_profile: SensorProfile,
) -> Result<N512FounderFoundationProjection, ScaffoldContractError> {
    let foundation = FoundationWeightAsset::builtin_nano512_v1(sensor_profile).unwrap();
    N512FounderFoundationProjection::compile(phenotype, sensor_profile, &foundation)
}

fn valid_alternate_same_profile_foundation(sensor_profile: SensorProfile) -> FoundationWeightAsset {
    let builtin = FoundationWeightAsset::builtin_nano512_v1(sensor_profile).unwrap();
    let capacity = BrainCapacityClass::n512();
    let coordinate_genome = BrainGenome::scaffold(N512_COORDINATE_RECIPE_SEED, capacity.id());
    let coordinate_development = DevelopmentState::new(
        coordinate_genome.id,
        Tick::ZERO,
        NormalizedScalar::new(1.0).unwrap(),
    );
    let coordinate = PhenotypeCompiler::compile_from_foundation_asset(
        &coordinate_genome,
        &capacity,
        &coordinate_development,
        sensor_profile,
        &builtin,
    )
    .unwrap();
    let mut weights = coordinate
        .synapses()
        .iter()
        .map(|synapse| synapse.genetic_weight())
        .collect::<Vec<_>>();
    *weights.first_mut().unwrap() += 0.001;
    let alternate = FoundationWeightAsset::from_trained_weights(
        &coordinate,
        weights,
        TrainingStageManifest::bootstrap(),
    )
    .unwrap();
    let alternate_coordinate = PhenotypeCompiler::compile_from_foundation_asset(
        &coordinate_genome,
        &capacity,
        &coordinate_development,
        sensor_profile,
        &alternate,
    )
    .unwrap();
    alternate.validate_against(&alternate_coordinate).unwrap();
    assert_ne!(alternate.digest(), builtin.digest());
    alternate
}

#[test]
fn n512_projection_retains_two_distinct_world_founders_for_both_profiles() {
    let first = n512_founder(0x5120_1001);
    let second = n512_founder(0x5120_1002);

    for sensor_profile in [
        SensorProfile::PrivilegedAffordanceV1,
        SensorProfile::GroundedObjectSlotsV1,
    ] {
        let first_projection = compile(&first, sensor_profile).unwrap();
        let second_projection = compile(&second, sensor_profile).unwrap();

        assert_eq!(first_projection.source_brain_genome(), &first.brain_genome);
        assert_eq!(
            second_projection.source_brain_genome(),
            &second.brain_genome
        );
        assert_eq!(first_projection.source_genome_id(), first.source_genome_id);
        assert_eq!(
            second_projection.source_genome_id(),
            second.source_genome_id
        );
        assert_eq!(first_projection.lineage_id(), first.lineage_id);
        assert_eq!(second_projection.lineage_id(), second.lineage_id);
        assert_eq!(
            first_projection.genetic_provenance(),
            &first.genetic_provenance
        );
        assert_eq!(
            second_projection.genetic_provenance(),
            &second.genetic_provenance
        );
        assert_eq!(
            first_projection.runtime_development_state(),
            &first.development_state_at(Tick::ZERO).unwrap()
        );
        assert_eq!(
            second_projection.runtime_development_state(),
            &second.development_state_at(Tick::ZERO).unwrap()
        );

        assert_eq!(
            first_projection.frozen_abi().layout_digest(),
            second_projection.frozen_abi().layout_digest()
        );
        assert_eq!(
            first_projection.frozen_abi().address_map_digest(),
            second_projection.frozen_abi().address_map_digest()
        );
        assert_eq!(
            first_projection.frozen_abi().decoder_digest(),
            second_projection.frozen_abi().decoder_digest()
        );
        assert_eq!(
            first_projection.frozen_abi().route_abi_digest(),
            second_projection.frozen_abi().route_abi_digest()
        );
        assert_eq!(
            first_projection.frozen_abi().plasticity_abi_digest(),
            second_projection.frozen_abi().plasticity_abi_digest()
        );
        assert_eq!(
            first_projection.foundation_asset_digest(),
            second_projection.foundation_asset_digest()
        );
        assert_eq!(first_projection.sensor_profile(), sensor_profile);
        assert_eq!(second_projection.sensor_profile(), sensor_profile);
        assert_ne!(
            first_projection.overlay_seed(),
            second_projection.overlay_seed()
        );
        assert_ne!(
            first_projection.receipt().digest(),
            second_projection.receipt().digest()
        );
        assert_ne!(
            first_projection.compiled_phenotype().phenotype_hash(),
            second_projection.compiled_phenotype().phenotype_hash()
        );
    }
}

#[test]
fn n512_projection_maps_or_rejects_every_structural_delta() {
    let source = n512_founder(0x5120_1101);
    let baseline = compile(&source, SensorProfile::PrivilegedAffordanceV1).unwrap();

    let mut lobe_ratio = source.clone();
    if let LobeRatioPlan::InlineOverrides(rows) = &mut lobe_ratio.brain_genome.lobe_ratios {
        rows[0].ratio = NormalizedScalar::new(0.77).unwrap();
    }

    let mut density = source.clone();
    density.brain_genome.sparse_density_priors[0].density = NormalizedScalar::new(0.91).unwrap();

    let mut sensor = source.clone();
    sensor.brain_genome.sensor_layout.channels[0].receptor_count += 1;

    let mut development = source.clone();
    development.development.lobe_activation_maturation = NormalizedScalar::new(0.0).unwrap();

    let mut weight_prior = source.clone();
    weight_prior.brain_genome.genetic_prior_seed =
        weight_prior.brain_genome.genetic_prior_seed.wrapping_add(1);
    weight_prior.brain_genome.seeds.genetic_prior_seed =
        weight_prior.brain_genome.genetic_prior_seed;

    for (label, candidate) in [
        ("lobe ratio", lobe_ratio),
        ("density", density),
        ("sensor", sensor),
        ("development", development),
        ("weight prior", weight_prior),
    ] {
        match compile(&candidate, SensorProfile::PrivilegedAffordanceV1) {
            Ok(candidate) => assert!(
                candidate.overlay_seed() != baseline.overlay_seed()
                    || candidate.receipt().digest() != baseline.receipt().digest()
                    || candidate.compiled_phenotype().phenotype_hash()
                        != baseline.compiled_phenotype().phenotype_hash(),
                "{label} delta vanished from the projection"
            ),
            Err(ScaffoldContractError::PhenotypeCompile) => {}
            Err(error) => panic!("{label} returned an unrelated error: {error:?}"),
        }
    }
}

#[test]
fn n512_projection_rejects_forged_cross_class_and_wrong_profile_assets() {
    let source = n512_founder(0x5120_1201);
    let privileged =
        FoundationWeightAsset::builtin_nano512_v1(SensorProfile::PrivilegedAffordanceV1).unwrap();
    let grounded =
        FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    let n2048 =
        FoundationWeightAsset::builtin_n2048_v1(SensorProfile::PrivilegedAffordanceV1).unwrap();

    assert!(N512FounderFoundationProjection::compile(
        &source,
        SensorProfile::GroundedObjectSlotsV1,
        &privileged,
    )
    .is_err());
    assert!(N512FounderFoundationProjection::compile(
        &source,
        SensorProfile::PrivilegedAffordanceV1,
        &n2048,
    )
    .is_err());

    let mut forged = serde_json::to_value(&grounded).unwrap();
    forged["manifest"]["capacity_class_id"] = serde_json::json!(BrainCapacityClass::N1024_ID.raw());
    let forged: FoundationWeightAsset = serde_json::from_value(forged).unwrap();
    assert!(N512FounderFoundationProjection::compile(
        &source,
        SensorProfile::GroundedObjectSlotsV1,
        &forged,
    )
    .is_err());
}

#[test]
fn n512_projection_rejects_valid_same_class_same_profile_alternate_asset() {
    let source = n512_founder(0x5120_1251);
    for sensor_profile in [
        SensorProfile::PrivilegedAffordanceV1,
        SensorProfile::GroundedObjectSlotsV1,
    ] {
        let alternate = valid_alternate_same_profile_foundation(sensor_profile);
        let error = N512FounderFoundationProjection::compile(&source, sensor_profile, &alternate)
            .unwrap_err();
        assert_eq!(error, ScaffoldContractError::PhenotypeCompile);
    }
}

#[test]
fn n512_projection_never_substitutes_the_fixed_scaffold_as_source() {
    let source = n512_founder(0x5120_1301);
    let projection = compile(&source, SensorProfile::PrivilegedAffordanceV1).unwrap();

    assert_eq!(projection.source_brain_genome(), &source.brain_genome);
    assert_ne!(
        projection.source_brain_genome(),
        projection.frozen_abi().coordinate_genome()
    );
    assert_eq!(
        projection.runtime_development_state(),
        &source.development_state_at(Tick::ZERO).unwrap()
    );
    assert_ne!(
        projection.runtime_development_state(),
        projection.frozen_abi().coordinate_development_state()
    );
    assert_eq!(
        projection.compiled_phenotype().foundation_abi(),
        projection.frozen_abi().foundation_abi()
    );
}

#[test]
fn n512_projection_receipt_rejects_tampering() {
    let projection = compile(
        &n512_founder(0x5120_1401),
        SensorProfile::PrivilegedAffordanceV1,
    )
    .unwrap();
    let mut forged = serde_json::to_value(projection.receipt()).unwrap();
    forged["overlay_seed"] = serde_json::json!(projection.overlay_seed().wrapping_add(1));
    assert!(serde_json::from_value::<N512FounderProjectionReceipt>(forged).is_err());
}

fn write_json_digest4(digest: &mut CanonicalDigestBuilder, wire: &serde_json::Value, field: &str) {
    for word in wire[field].as_array().unwrap() {
        digest.write_u64(word.as_u64().unwrap());
    }
}

fn write_json_blake3(digest: &mut CanonicalDigestBuilder, wire: &serde_json::Value, field: &str) {
    let bytes = wire[field].as_array().unwrap();
    assert_eq!(bytes.len(), 32);
    let bytes = bytes
        .iter()
        .map(|byte| byte.as_u64().unwrap() as u8)
        .collect::<Vec<_>>();
    digest.write_bytes(&bytes);
}

fn recompute_receipt_digest(wire: &serde_json::Value) -> [u64; 4] {
    let mut digest = CanonicalDigestBuilder::new(b"alife.phenotype.n512-founder-receipt.v1");
    digest.write_u16(wire["schema_version"].as_u64().unwrap() as u16);
    digest.write_u64(wire["source_genome_id"].as_u64().unwrap());
    digest.write_u64(wire["lineage_id"].as_u64().unwrap());
    write_json_digest4(&mut digest, wire, "source_inputs_digest");
    digest.write_u64(wire["foundation_id"].as_u64().unwrap());
    digest.write_u32(wire["foundation_version"].as_u64().unwrap() as u32);
    digest.write_u64(wire["compatibility_family_id"].as_u64().unwrap());
    digest.write_u16(wire["capacity_class_id"].as_u64().unwrap() as u16);
    let sensor_profile = match wire["sensor_profile"].as_str().unwrap() {
        "PrivilegedAffordanceV1" => SensorProfile::PrivilegedAffordanceV1.raw(),
        "GroundedObjectSlotsV1" => SensorProfile::GroundedObjectSlotsV1.raw(),
        profile => panic!("unexpected serialized sensor profile: {profile}"),
    };
    digest.write_u16(sensor_profile);
    write_json_blake3(&mut digest, wire, "foundation_asset_digest");
    write_json_blake3(&mut digest, wire, "coordinate_layout_digest");
    write_json_blake3(&mut digest, wire, "coordinate_address_map_digest");
    write_json_digest4(&mut digest, wire, "coordinate_decoder_digest");
    write_json_blake3(&mut digest, wire, "coordinate_route_abi_digest");
    write_json_blake3(&mut digest, wire, "coordinate_plasticity_abi_digest");
    write_json_digest4(&mut digest, wire, "runtime_development_digest");
    write_json_digest4(&mut digest, wire, "genetic_provenance_digest");
    digest.write_u64(wire["overlay_seed"].as_u64().unwrap());
    write_json_digest4(&mut digest, wire, "phenotype_hash");
    digest.finish256()
}

fn reseal_receipt(wire: &mut serde_json::Value) {
    wire["digest"] = serde_json::to_value(recompute_receipt_digest(wire)).unwrap();
}

#[test]
fn n512_projection_receipt_rejects_recomputed_noncanonical_bindings() {
    let projection = compile(
        &n512_founder(0x5120_1451),
        SensorProfile::PrivilegedAffordanceV1,
    )
    .unwrap();
    let alternate = valid_alternate_same_profile_foundation(SensorProfile::PrivilegedAffordanceV1);

    let mut wrong_profile = serde_json::to_value(projection.receipt()).unwrap();
    wrong_profile["sensor_profile"] =
        serde_json::to_value(SensorProfile::GroundedObjectSlotsV1).unwrap();
    reseal_receipt(&mut wrong_profile);
    assert!(
        serde_json::from_value::<N512FounderProjectionReceipt>(wrong_profile).is_err(),
        "a recomputed receipt for the other canonical profile must be rejected"
    );

    let mut wrong_asset = serde_json::to_value(projection.receipt()).unwrap();
    wrong_asset["foundation_asset_digest"] = serde_json::to_value(alternate.digest()).unwrap();
    reseal_receipt(&mut wrong_asset);
    assert!(
        serde_json::from_value::<N512FounderProjectionReceipt>(wrong_asset).is_err(),
        "a recomputed receipt for a valid alternate asset must be rejected"
    );

    let mut wrong_abi = serde_json::to_value(projection.receipt()).unwrap();
    wrong_abi["coordinate_layout_digest"] =
        serde_json::to_value(Blake3Digest::from_bytes([0xA5; 32])).unwrap();
    reseal_receipt(&mut wrong_abi);
    assert!(
        serde_json::from_value::<N512FounderProjectionReceipt>(wrong_abi).is_err(),
        "a recomputed receipt with a noncanonical frozen ABI must be rejected"
    );
}

#[test]
fn n512_projection_receipt_rejects_cross_instance_binding() {
    let first = compile(
        &n512_founder(0x5120_1461),
        SensorProfile::PrivilegedAffordanceV1,
    )
    .unwrap();
    let second = compile(
        &n512_founder(0x5120_1462),
        SensorProfile::PrivilegedAffordanceV1,
    )
    .unwrap();

    assert!(first.receipt().validate_against_projection(&first).is_ok());
    assert!(first
        .receipt()
        .validate_against_projection(&second)
        .is_err());
}

#[test]
fn n512_projection_preserves_checked_foundation_payloads() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/brain_foundations");
    for (profile, file_name) in [
        (
            SensorProfile::PrivilegedAffordanceV1,
            "n512-v1-privileged.alife-foundation",
        ),
        (
            SensorProfile::GroundedObjectSlotsV1,
            "n512-v1-grounded.alife-foundation",
        ),
        (
            SensorProfile::PrivilegedAffordanceV1,
            "n2048-v1-privileged.alife-foundation",
        ),
        (
            SensorProfile::GroundedObjectSlotsV1,
            "n2048-v1-grounded.alife-foundation",
        ),
    ] {
        let path = root.join(file_name);
        let checked = fs::read(&path).unwrap();
        let asset = if file_name.starts_with("n512") {
            FoundationWeightAsset::builtin_nano512_v1(profile).unwrap()
        } else {
            FoundationWeightAsset::builtin_n2048_v1(profile).unwrap()
        };
        assert_eq!(asset.encode_canonical().unwrap(), checked, "{file_name}");
        assert_ne!(asset.digest().bytes(), &[0; 32], "{file_name}");
    }
}
