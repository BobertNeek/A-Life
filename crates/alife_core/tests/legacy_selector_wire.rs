//! Literal pre-change JSON and bincode-encode fixtures, generated only from the
//! frozen parent revision. JSON restore is supported; adjacent-tag bincode
//! decoding is probed honestly rather than assumed to be supported.
use alife_core::*;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
fn pair(id: BrainClassId) -> (BrainGenome, DevelopmentState) {
    let g = BrainGenome::scaffold(0x2048_2026, id);
    let d = DevelopmentState::new(g.id, Tick::ZERO, NormalizedScalar::new(1.).unwrap());
    (g, d)
}
fn cases() -> Vec<(
    &'static str,
    BrainPhenotype,
    Option<PhenotypeCompilerInputs>,
)> {
    let profile = SensorProfile::GroundedObjectSlotsV1;
    let mut cases = vec![];
    let (g, d) = pair(BrainCapacityClass::N2048_ID);
    let (p, i) =
        PhenotypeCompiler::compile_n2048_experimental_graph(g.clone(), d.clone(), profile).unwrap();
    cases.push(("experimental", p, Some(i)));
    let i = PhenotypeCompilerInputs::try_new(
        g.clone(),
        &BrainCapacityClass::n2048(),
        d.clone(),
        profile,
    )
    .unwrap();
    let p = PhenotypeCompiler::compile_validated(&i, &BrainCapacityClass::n2048()).unwrap();
    cases.push(("canonical", p, Some(i)));
    let asset = FoundationWeightAsset::builtin_n2048_v1(profile).unwrap();
    let p = PhenotypeCompiler::compile_from_foundation_asset(
        &g,
        &BrainCapacityClass::n2048(),
        &d,
        profile,
        &asset,
    )
    .unwrap();
    cases.push(("migrated", p, None));
    let asset = FoundationWeightAsset::builtin_nano512_v1(profile).unwrap();
    let (p, i, _) =
        PhenotypeCompiler::compile_fixed_legacy_nano512_compatibility_asset(profile, &asset)
            .unwrap()
            .into_runtime_parts();
    cases.push(("legacy", p.clone(), Some(i)));
    let asset = FoundationWeightAsset::from_nano512_readout_candidate(
        &p,
        asset.weights().to_vec(),
        TrainingStageManifest::new(1, 1, 1),
    )
    .unwrap();
    let (p, i) = PhenotypeCompiler::compile_nano512_readout_candidate(&asset).unwrap();
    cases.push(("readout", p, Some(i)));
    let c = Nano512ActionCreditCandidateV2::new(
        &asset,
        ActionCandidateCreditProfileV1::SignedConsequences,
    )
    .unwrap();
    let (p, i) = PhenotypeCompiler::compile_nano512_action_credit_candidate(&c).unwrap();
    cases.push(("credit", p, Some(i)));
    cases
}
fn literal<T: Serialize>(name: &str, value: &T) {
    let manifest: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/experiments/bounded-founder-v1/legacy-wire-capture.json"
    ))
    .unwrap();
    for (ext, bytes) in [
        ("json", serde_json::to_vec(value).unwrap()),
        (
            "bin",
            bincode::serde::encode_to_vec(value, bincode::config::standard()).unwrap(),
        ),
    ] {
        assert!(matches_literal(&manifest, &format!("{name}.{ext}"), &bytes));
    }
}
fn matches_literal(manifest: &serde_json::Value, name: &str, bytes: &[u8]) -> bool {
    let item = &manifest["files"][name];
    item["bytes"].as_u64() == Some(bytes.len() as u64)
        && item["sha256"].as_str() == Some(format!("{:x}", Sha256::digest(bytes)).as_str())
}
#[test]
fn missing_and_tampered_literal_receipts_fail_closed() {
    let bytes = b"old bytes";
    let m = serde_json::json!({"files":{"fixture":{"bytes":bytes.len(),"sha256":format!("{:x}",Sha256::digest(bytes))}}});
    assert!(matches_literal(&m, "fixture", bytes));
    assert!(!matches_literal(&m, "missing", bytes));
    assert!(!matches_literal(&m, "fixture", b"bad bytes"));
}
#[test]
fn preserved_old_json_restores_when_capture_is_provided() {
    let Some(root) = std::env::var_os("ALIFE_LEGACY_WIRE_RESTORE_DIR") else {
        return;
    };
    let root = PathBuf::from(root);
    for (name, p, i) in cases() {
        let old = std::fs::read(root.join(format!("{name}_phenotype.json"))).unwrap();
        let restored: BrainPhenotype = serde_json::from_slice(&old).unwrap();
        assert_eq!(restored, p);
        let old = std::fs::read(root.join(format!("{name}_selector.json"))).unwrap();
        let restored: FoundationAbiSelection = serde_json::from_slice(&old).unwrap();
        assert_eq!(&restored, p.foundation_abi());
        if let Some(i) = i {
            let old = std::fs::read(root.join(format!("{name}_inputs.json"))).unwrap();
            let restored: PhenotypeCompilerInputs = serde_json::from_slice(&old).unwrap();
            assert_eq!(restored, i);
            let capacity = BrainCapacityClass::supported_for_id(p.brain_class_id()).unwrap();
            assert_eq!(
                PhenotypeCompiler::compile_validated(&restored, &capacity).unwrap(),
                p
            );
        }
    }
}
#[test]
fn all_existing_selectors_and_nested_records_preserve_literal_bytes() {
    for (name, p, i) in cases() {
        literal(&format!("{name}_selector"), p.foundation_abi());
        literal(&format!("{name}_phenotype"), &p);
        let restored: BrainPhenotype =
            serde_json::from_slice(&serde_json::to_vec(&p).unwrap()).unwrap();
        assert_eq!(restored, p);
        let selector: FoundationAbiSelection =
            serde_json::from_slice(&serde_json::to_vec(p.foundation_abi()).unwrap()).unwrap();
        assert_eq!(&selector, p.foundation_abi());
        if let Some(i) = i {
            literal(&format!("{name}_inputs"), &i);
            let round: PhenotypeCompilerInputs =
                serde_json::from_slice(&serde_json::to_vec(&i).unwrap()).unwrap();
            assert_eq!(round, i);
            let capacity = BrainCapacityClass::supported_for_id(p.brain_class_id()).unwrap();
            assert_eq!(
                PhenotypeCompiler::compile_validated(&round, &capacity).unwrap(),
                p
            );
        }
    }
}
