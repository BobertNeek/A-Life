//! Rebind retained scaled weights to the current fixed Nano512 ABI. CPU only.
//! This emits an unpromoted fixture, never new training or behavioral evidence.

use std::{error::Error, fs, path::PathBuf};

use alife_core::{
    ActionCandidateCreditProfileV1, FoundationWeightAsset, Nano512ActionCreditCandidateV2,
    PhenotypeCompiler, SensorProfile, TrainingStageManifest,
};
use serde_json::json;

const HISTORICAL_DIGEST: [u8; 32] = [
    162, 205, 184, 109, 162, 15, 136, 4, 200, 120, 232, 83, 105, 194, 153, 231, 150, 181, 202, 200,
    230, 97, 59, 164, 174, 244, 52, 65, 167, 166, 238, 30,
];
const HISTORICAL_FILE_BLAKE3: &str =
    "570e3faa82f1f6fccdf1018b7736e057dce4c700d3b549e49747982dce4d0c77";

fn hex(bytes: &[u8; 32]) -> String {
    blake3::Hash::from(*bytes).to_hex().to_string()
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let root = PathBuf::from(
        arguments
            .next()
            .ok_or("expected repository root argument")?,
    );
    if arguments.next().is_some() {
        return Err("usage: rebind_scaled_choice_founder REPOSITORY_ROOT".into());
    }
    let bundle = root.join("assets/founders/scaled-choice-nociceptive-v1");
    let historical_path = "historical/2026-09-08-scaled-choice-32416/candidate.alife-foundation";
    let historical_bytes = fs::read(bundle.join(historical_path))?;
    // Source-bound offline extraction of this exact rejected historical file.
    // There is deliberately no legacy parser or relaxed runtime admission.
    assert_eq!(historical_bytes.len(), 7673);
    assert_eq!(
        blake3::hash(&historical_bytes).to_hex().as_str(),
        HISTORICAL_FILE_BLAKE3
    );
    assert_eq!(&historical_bytes[7641..7673], &HISTORICAL_DIGEST);
    assert_eq!(&historical_bytes[405..437], &HISTORICAL_DIGEST);
    assert_eq!(&historical_bytes[437..441], &1799_u32.to_le_bytes());
    assert_eq!(&historical_bytes[441..445], &1799_u32.to_le_bytes());
    assert!(FoundationWeightAsset::decode_canonical(&historical_bytes).is_err());
    let historical_weights: Vec<_> = historical_bytes[445..7641]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|bytes| f32::from_bits(u32::from_le_bytes(*bytes)))
        .collect();
    assert_eq!(historical_weights.len(), 1799);
    let builtin = FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1)?;
    let baseline = PhenotypeCompiler::compile_fixed_legacy_nano512_compatibility_asset(
        SensorProfile::GroundedObjectSlotsV1,
        &builtin,
    )?
    .into_runtime_parts()
    .0;
    let current = FoundationWeightAsset::from_nano512_readout_candidate(
        &baseline,
        historical_weights.clone(),
        TrainingStageManifest::bootstrap(),
    )?;
    assert_eq!(current.weights().len(), 1799);
    for (original, rebound) in historical_weights.iter().zip(current.weights()) {
        assert_eq!(original.to_bits(), rebound.to_bits());
    }
    let manifest = current.manifest();
    assert_eq!(&historical_bytes[36..68], manifest.layout_digest().bytes());
    assert_eq!(
        &historical_bytes[166..198],
        manifest.route_abi_digest().bytes()
    );
    assert_eq!(
        &historical_bytes[198..230],
        manifest.plasticity_abi_digest().bytes()
    );
    assert_eq!(
        &historical_bytes[230..262],
        manifest.address_map_digest().bytes()
    );
    assert_eq!(
        manifest.address_map_digest(),
        baseline.persistent_address_map().digest()
    );
    assert_eq!(
        manifest.action_decoder_digest(),
        baseline.candidate_decoder().canonical_digest()
    );
    assert_eq!(
        manifest.training_stage(),
        TrainingStageManifest::bootstrap()
    );
    assert!(!manifest.promotion_receipt().is_promoted());

    let configured = Nano512ActionCreditCandidateV2::new(
        &current,
        ActionCandidateCreditProfileV1::SignedChoiceReadouts,
    )?;
    let (phenotype, _) = PhenotypeCompiler::compile_nano512_action_credit_candidate(&configured)?;
    current.validate_against(&phenotype)?;
    let current_bytes = current.encode_canonical()?;
    assert_eq!(
        FoundationWeightAsset::decode_canonical(&current_bytes)?,
        current
    );
    assert_ne!(current.digest().bytes(), &HISTORICAL_DIGEST);
    assert_eq!(current_bytes.len(), historical_bytes.len());
    let changed_ranges = [100..132, 262..306, 306..405, 405..437, 7641..7673];
    for (offset, (old, new)) in historical_bytes.iter().zip(&current_bytes).enumerate() {
        if !changed_ranges.iter().any(|range| range.contains(&offset)) {
            assert_eq!(old, new, "unexpected changed wire byte at {offset}");
        }
    }

    let receipt = json!({
        "schema_version": 1,
        "operation": "current-fixed-ABI rebind of retained historical scaled weights",
        "generator": "crates/alife_tools/examples/rebind_scaled_choice_founder.rs",
        "historical_source": {
            "asset": historical_path,
            "canonical_digest_hex": hex(&HISTORICAL_DIGEST),
            "file_blake3_hex": blake3::hash(&historical_bytes).to_hex().to_string(),
            "file_sha256_hex": "2d04957b75bda89f73c48c15aed28b9d71c5ec85b408db5bb0f0db1603f922d9",
            "manifest_wire_bytes": &historical_bytes[10..441],
            "historical_training_stage": {
                "schema_version": 1, "curriculum_version": 4,
                "evaluation_version": 16, "completed_stage_count": 1,
                "wire_bytes": &historical_bytes[262..306]
            },
            "historical_promotion_receipt_wire_bytes": &historical_bytes[306..405],
            "receipt_status": "historical README claims; detailed external receipts unavailable here"
        },
        "current_fixture": {
            "asset": "candidate.alife-foundation",
            "canonical_digest_hex": hex(current.digest().bytes()),
            "file_blake3_hex": blake3::hash(&current_bytes).to_hex().to_string(),
            "manifest": manifest,
            "fixed_source_canonical_digest_hex": hex(builtin.digest().bytes()),
            "fixed_source_phenotype_hash": baseline.phenotype_hash(),
            "current_behavioral_evidence": "Unknown",
            "current_training_stage": "bootstrap",
            "current_curriculum_version": 0,
            "current_evaluation_version": 0,
            "current_completed_stage_count": 0,
            "promoted": false
        },
        "weight_count": 1799,
        "preserved_weight_bits": true,
        "preserved_layout_route_plasticity_address_map": true,
        "source_bound_extraction": {"file_bytes": 7673, "weight_count_prefix": [441,445], "weight_payload": [445,7641], "trailing_canonical_digest": [7641,7673]},
        "allowed_changed_wire_ranges": {"action_decoder": [100,132], "training_stage": [262,306], "promotion_receipt": [306,405], "weight_asset_digest": [405,437], "trailing_canonical_digest": [7641,7673]},
        "optimizer_steps": 0,
        "new_random_weights": 0,
        "gpu_execution": false,
        "historical_measurements_reused_as_current_evidence": false
    });
    let mut receipt_bytes = serde_json::to_vec_pretty(&receipt)?;
    receipt_bytes.push(b'\n');
    fs::write(bundle.join("candidate.alife-foundation"), current_bytes)?;
    fs::write(bundle.join("rebind-receipt.json"), receipt_bytes)?;
    fs::write(
        bundle.join("candidate-digest.txt"),
        format!("{}\n", hex(current.digest().bytes())),
    )?;
    println!(
        "current unpromoted candidate: {}",
        hex(current.digest().bytes())
    );
    Ok(())
}
