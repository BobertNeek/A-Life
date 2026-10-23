#![cfg(feature = "receipt-lineage")]
use alife_core::*;
use alife_training::{AdamWConfig, FoundationTrainerCheckpoint, StageTrainableMask};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn source(root: &Path, name: &str, delta: f32, different_mask: bool, frozen_change: bool) -> Value {
    let dir = root.join(name);
    fs::create_dir(&dir).unwrap();
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(0x7A11, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.).unwrap());
    let base = PhenotypeCompiler::compile(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let builtin = FoundationWeightAsset::from_trained_weights(
        &base,
        base.synapses().iter().map(|s| s.genetic_weight()).collect(),
        TrainingStageManifest::new(1, 1, 1),
    )
    .unwrap();
    let compile = |asset: &FoundationWeightAsset| {
        PhenotypeCompiler::compile_n2048_foundation_candidate(
            genome.clone(),
            development.clone(),
            asset.clone(),
        )
        .unwrap()
    };
    let original = compile(&builtin).0;
    let index = original
        .synapses()
        .iter()
        .position(|s| matches!(s.kind(), CompiledSynapseKind::Decoder(_)))
        .unwrap();
    let mut weights = builtin.weights().to_vec();
    weights[index + usize::from(frozen_change)] += delta;
    let stage = TrainingStageManifest::new(1, 1, if name == "pc" { 9241 } else { 7975 });
    let asset = FoundationWeightAsset::from_trained_weights(&original, weights, stage).unwrap();
    let (phenotype, inputs) = compile(&asset);
    let actor = FoundationTrainerCheckpoint {
        schema_version: 2,
        phenotype_hash: phenotype.phenotype_hash(),
        source_foundation_digest: asset.digest(),
        optimizer_step: 3,
        config: AdamWConfig::default(),
        stage_mask: StageTrainableMask::from_synapse_indices(
            &phenotype,
            &[index as u32 + u32::from(different_mask)],
        )
        .unwrap(),
        weights: asset.weights().to_vec(),
        first_moment: vec![0.; asset.weights().len()],
        second_moment: vec![0.; asset.weights().len()],
        update_ages: vec![0; asset.weights().len()],
    };
    let p = dir.join("phenotype.json");
    let a = dir.join("trained.alife-foundation");
    let c = dir.join("actor-checkpoint.json");
    fs::write(&p, serde_json::to_vec(&phenotype).unwrap()).unwrap();
    fs::write(&a, asset.encode_canonical().unwrap()).unwrap();
    fs::write(&c, serde_json::to_vec(&actor).unwrap()).unwrap();
    if name == "cpu" {
        let i = dir.join("compiler-inputs.json");
        fs::write(&i, serde_json::to_vec(&inputs).unwrap()).unwrap();
        json!({"compiler_inputs":i,"foundation":a,"actor_checkpoint":c})
    } else {
        json!({"phenotype":p,"foundation":a,"actor_checkpoint":c})
    }
}
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "alife-inference-merge-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn request(&self, mismatch: bool, frozen_change: bool) -> Value {
        let audit = self.0.join("ancestry.json");
        fs::write(&audit, b"{\"fixture_only\":true}").unwrap();
        let request = json!({"parent":source(&self.0,"parent",0.,false,false),"pc":source(&self.0,"pc",0.02,false,frozen_change),
            "cpu":source(&self.0,"cpu",0.04,mismatch,false),"ancestry_audit":audit});
        fs::write(
            self.0.join("request.json"),
            serde_json::to_vec(&request).unwrap(),
        )
        .unwrap();
        request
    }
    fn run(&self) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_merge_founder_inference"))
            .arg(self.0.join("request.json"))
            .arg(self.0.join("output"))
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let path = self.0.canonicalize().unwrap();
        assert_eq!(
            path.parent(),
            Some(std::env::temp_dir().canonicalize().unwrap().as_path())
        );
        assert!(path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("alife-inference-merge-"));
        let _ = fs::remove_dir_all(path);
    }
}

#[test]
fn differing_real_stage_counts_create_bit_identical_views_without_changing_inputs() {
    let fixture = Fixture::new();
    let request = fixture.request(false, false);
    let before: Vec<_> = ["parent", "pc", "cpu"]
        .iter()
        .flat_map(|role| request[role].as_object().unwrap().values())
        .map(|path| {
            (
                path.as_str().unwrap().to_owned(),
                fs::read(path.as_str().unwrap()).unwrap(),
            )
        })
        .collect();
    let result = fixture.run();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let receipt: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(receipt["averaged_changes"], 1);
    assert_eq!(receipt["unique_changes"], 0);
    assert_eq!(receipt["sign_conflicts"], 0);
    assert_eq!(receipt["unchanged"], 32767);
    assert_eq!(
        receipt["evaluation_views"]["pc"]["original_stage"]["completed_stage_count"],
        9241
    );
    assert_eq!(
        receipt["evaluation_views"]["pc"]["view_stage"]["completed_stage_count"],
        7975
    );
    assert_eq!(receipt["evaluation_views"]["weights_bit_identical"], true);
    assert_ne!(
        receipt["evaluation_views"]["pc"]["original_digest"],
        receipt["evaluation_views"]["pc"]["view_digest"]
    );
    assert_eq!(receipt["non_resumable"], true);
    assert_eq!(receipt["adam_merged"], false);
    assert_eq!(receipt["private_state_merged"], false);
    assert_eq!(receipt["ancestry_authenticated_by_tool"], false);
    let merged = FoundationWeightAsset::decode_canonical(
        &fs::read(fixture.0.join("output/merged.alife-foundation")).unwrap(),
    )
    .unwrap();
    let source_asset = |role: &str| {
        FoundationWeightAsset::decode_canonical(
            &fs::read(request[role]["foundation"].as_str().unwrap()).unwrap(),
        )
        .unwrap()
    };
    let (pc, cpu) = (source_asset("pc"), source_asset("cpu"));
    for ((actual, left), right) in merged.weights().iter().zip(pc.weights()).zip(cpu.weights()) {
        assert_eq!(
            actual.to_bits(),
            (((f64::from(*left) + f64::from(*right)) * 0.5) as f32).to_bits()
        );
    }
    assert_eq!(fs::read_dir(fixture.0.join("output")).unwrap().count(), 2);
    for (path, bytes) in before {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    assert!(!fixture.run().status.success());
}
#[test]
fn incompatible_actual_masks_fail_before_output_creation() {
    let fixture = Fixture::new();
    fixture.request(true, false);
    let result = fixture.run();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("actual stage mask"));
    assert!(!fixture.0.join("output").exists());
}

#[test]
fn frozen_coordinate_change_is_rejected_without_output_or_input_changes() {
    let fixture = Fixture::new();
    let request = fixture.request(false, true);
    let before: Vec<_> = ["parent", "pc", "cpu"]
        .iter()
        .flat_map(|role| request[role].as_object().unwrap().values())
        .map(|path| {
            (
                path.as_str().unwrap().to_owned(),
                fs::read(path.as_str().unwrap()).unwrap(),
            )
        })
        .collect();
    let result = fixture.run();
    assert!(!result.status.success());
    let error = String::from_utf8_lossy(&result.stderr);
    assert!(
        error.contains("merge_founder_branches strict contract"),
        "{error}"
    );
    assert!(error.contains("PhenotypeCompile"), "{error}");
    assert!(!fixture.0.join("output").exists());
    for (path, bytes) in before {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}
