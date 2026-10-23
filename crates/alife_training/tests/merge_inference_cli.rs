#![cfg(feature = "receipt-lineage")]
use alife_core::*;
use alife_training::{AdamWConfig, FoundationTrainerCheckpoint, StageTrainableMask};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn source(
    root: &Path,
    name: &str,
    deltas: &[f32],
    different_mask: bool,
    frozen_change: bool,
    stage: TrainingStageManifest,
) -> Value {
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
    for (offset, delta) in deltas.iter().enumerate() {
        weights[index + offset + usize::from(frozen_change)] += delta;
    }
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
            &(0..deltas.len())
                .map(|offset| index as u32 + offset as u32 + u32::from(different_mask))
                .collect::<Vec<_>>(),
        )
        .unwrap(),
        weights: asset.weights().to_vec(),
        first_moment: vec![0.; asset.weights().len()],
        second_moment: vec![0.; asset.weights().len()],
        update_ages: vec![0; asset.weights().len()],
    };
    asset.validate_against(&phenotype).unwrap();
    actor.validate_for(&phenotype, asset.digest()).unwrap();
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
        self.request_with_stage(mismatch, frozen_change, None)
    }
    fn request_with_stage(
        &self,
        mismatch: bool,
        frozen_change: bool,
        stage_override: Option<(&str, TrainingStageManifest)>,
    ) -> Value {
        self.request_with_deltas(mismatch, frozen_change, stage_override, &[0.02], &[0.04])
    }
    fn request_with_deltas(
        &self,
        mismatch: bool,
        frozen_change: bool,
        stage_override: Option<(&str, TrainingStageManifest)>,
        pc: &[f32],
        cpu: &[f32],
    ) -> Value {
        assert_eq!(pc.len(), cpu.len());
        let audit = self.0.join("ancestry.json");
        fs::write(&audit, b"{\"fixture_only\":true}").unwrap();
        let stage = |role: &str| match stage_override {
            Some((branch, stage)) if role == branch => stage,
            _ => TrainingStageManifest::new(1, 1, if role == "pc" { 9241 } else { 7975 }),
        };
        let request = json!({"parent":source(&self.0,"parent",&vec![0.;pc.len()],false,false,stage("parent")),
            "pc":source(&self.0,"pc",pc,false,frozen_change,stage("pc")),
            "cpu":source(&self.0,"cpu",cpu,mismatch,false,stage("cpu")),"ancestry_audit":audit});
        fs::write(
            self.0.join("request.json"),
            serde_json::to_vec(&request).unwrap(),
        )
        .unwrap();
        request
    }
    fn run(&self) -> std::process::Output {
        self.run_policy("output", None)
    }
    fn run_policy(&self, output: &str, policy: Option<&str>) -> std::process::Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_merge_founder_inference"));
        command
            .arg(self.0.join("request.json"))
            .arg(self.0.join(output));
        if let Some(policy) = policy {
            command.args(["--conflict-policy", policy]);
        }
        command.output().unwrap()
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
    for role in ["pc", "cpu"] {
        for kind in ["original_stage", "view_stage"] {
            assert_eq!(
                receipt["evaluation_views"][role][kind]["curriculum_version"],
                1
            );
            assert_eq!(
                receipt["evaluation_views"][role][kind]["evaluation_version"],
                1
            );
        }
    }
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
    assert!(!fixture
        .run_policy("output", Some("midpoint"))
        .status
        .success());
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
    assert!(!fixture
        .run_policy("output", Some("midpoint"))
        .status
        .success());
    assert!(!fixture.0.join("output").exists());
}

fn assert_stage_version_rejected(role: &str, version: &str, stage: TrainingStageManifest) {
    let fixture = Fixture::new();
    fixture.request_with_stage(false, false, Some((role, stage)));
    let before: Vec<_> = ["", "parent", "pc", "cpu"]
        .into_iter()
        .flat_map(|dir| fs::read_dir(fixture.0.join(dir)).unwrap())
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_file())
        .map(|path| {
            let bytes = fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect();
    let result = fixture.run();
    assert!(!result.status.success());
    let error = String::from_utf8_lossy(&result.stderr);
    assert!(
        error.contains(&format!(
            "{} {version} version differs from common parent",
            role.to_uppercase()
        )),
        "{error}"
    );
    assert!(!fixture.0.join("output").exists());
    let midpoint = fixture.run_policy("output", Some("midpoint"));
    assert!(!midpoint.status.success());
    assert!(String::from_utf8_lossy(&midpoint.stderr).contains(&format!(
        "{} {version} version differs from common parent",
        role.to_uppercase(),
    )));
    assert!(!fixture.0.join("output").exists());
    for (path, bytes) in before {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn pc_curriculum_version_mismatch_rejects_valid_original_without_changing_inputs() {
    assert_stage_version_rejected("pc", "curriculum", TrainingStageManifest::new(2, 1, 9241));
}

#[test]
fn pc_evaluation_version_mismatch_rejects_valid_original_without_changing_inputs() {
    assert_stage_version_rejected("pc", "evaluation", TrainingStageManifest::new(1, 2, 9241));
}

#[test]
fn cpu_curriculum_version_mismatch_rejects_valid_original_without_changing_inputs() {
    assert_stage_version_rejected("cpu", "curriculum", TrainingStageManifest::new(2, 1, 7975));
}

#[test]
fn cpu_evaluation_version_mismatch_rejects_valid_original_without_changing_inputs() {
    assert_stage_version_rejected("cpu", "evaluation", TrainingStageManifest::new(1, 2, 7975));
}

#[test]
fn midpoint_is_opt_in_changes_only_conflicts_and_records_policy_without_changing_inputs() {
    let fixture = Fixture::new();
    let request = fixture.request_with_deltas(
        false,
        false,
        None,
        &[0.02, 0.02, 0.02, 0.],
        &[-0.04, 0.04, 0., 0.04],
    );
    let before: Vec<_> = ["parent", "pc", "cpu"]
        .iter()
        .flat_map(|role| request[role].as_object().unwrap().values())
        .map(|path| {
            let path = path.as_str().unwrap();
            (path.to_owned(), fs::read(path).unwrap())
        })
        .collect();
    let default = fixture.run();
    assert!(
        default.status.success(),
        "{}",
        String::from_utf8_lossy(&default.stderr)
    );
    let explicit = fixture.run_policy("explicit-default", Some("ancestor-revert"));
    assert!(
        explicit.status.success(),
        "{}",
        String::from_utf8_lossy(&explicit.stderr)
    );
    let midpoint = fixture.run_policy("midpoint", Some("midpoint"));
    assert!(
        midpoint.status.success(),
        "{}",
        String::from_utf8_lossy(&midpoint.stderr)
    );
    let path = |output: &str| fixture.0.join(output).join("merged.alife-foundation");
    let default_bytes = fs::read(path("output")).unwrap();
    assert_eq!(default_bytes, fs::read(path("explicit-default")).unwrap());
    let original = FoundationWeightAsset::decode_canonical(&default_bytes).unwrap();
    let changed =
        FoundationWeightAsset::decode_canonical(&fs::read(path("midpoint")).unwrap()).unwrap();
    let source_asset = |role: &str| {
        FoundationWeightAsset::decode_canonical(
            &fs::read(request[role]["foundation"].as_str().unwrap()).unwrap(),
        )
        .unwrap()
    };
    let parent = source_asset("parent");
    let pc = source_asset("pc");
    let cpu = source_asset("cpu");
    let mut changed_count = 0;
    for (index, (old, new)) in original.weights().iter().zip(changed.weights()).enumerate() {
        let p = parent.weights()[index];
        let a = pc.weights()[index];
        let b = cpu.weights()[index];
        if (f64::from(a) - f64::from(p)) * (f64::from(b) - f64::from(p)) < 0. {
            assert_eq!(old.to_bits(), p.to_bits());
            assert_eq!(
                new.to_bits(),
                (((f64::from(a) + f64::from(b)) * 0.5) as f32).to_bits()
            );
            changed_count += 1;
        } else {
            assert_eq!(old.to_bits(), new.to_bits());
        }
    }
    assert_eq!(changed_count, 1);
    for (output, policy, algorithm) in [
        (
            "output",
            "ancestor_revert",
            "alife_training::merge_founder_branches",
        ),
        (
            "explicit-default",
            "ancestor_revert",
            "alife_training::merge_founder_branches",
        ),
        (
            "midpoint",
            "midpoint",
            "alife_training::merge_founder_branches_with_policy",
        ),
    ] {
        let receipt: Value = serde_json::from_slice(
            &fs::read(fixture.0.join(output).join("merge-receipt.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(receipt["conflict_policy"], policy);
        assert_eq!(receipt["algorithm"], algorithm);
        assert_eq!(receipt["sign_conflicts"], 1);
        assert_eq!(receipt["unique_changes"], 2);
        assert_eq!(receipt["averaged_changes"], 1);
        assert_eq!(receipt["optimizer_updates"], 0);
        assert_eq!(receipt["non_resumable"], true);
        assert_eq!(receipt["unpromoted"], true);
        for key in [
            "adam_merged",
            "critic_merged",
            "private_state_merged",
            "ancestry_authenticated_by_tool",
        ] {
            assert_eq!(receipt[key], false);
        }
    }
    let saved_candidate = fs::read(path("midpoint")).unwrap();
    let saved_receipt = fs::read(fixture.0.join("midpoint/merge-receipt.json")).unwrap();
    assert!(!fixture
        .run_policy("midpoint", Some("midpoint"))
        .status
        .success());
    assert_eq!(fs::read(path("midpoint")).unwrap(), saved_candidate);
    assert_eq!(
        fs::read(fixture.0.join("midpoint/merge-receipt.json")).unwrap(),
        saved_receipt
    );
    for (path, bytes) in before {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn midpoint_rejects_nonfinite_asset_weights_without_publishing() {
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let fixture = Fixture::new();
        let request = fixture.request(false, false);
        let path = Path::new(request["pc"]["foundation"].as_str().unwrap());
        let mut bytes = fs::read(path).unwrap();
        let asset = FoundationWeightAsset::decode_canonical(&bytes).unwrap();
        let offset = bytes.len() - 32 - asset.weights().len() * 4;
        bytes[offset..offset + 4].copy_from_slice(&invalid.to_le_bytes());
        fs::write(path, &bytes).unwrap();
        assert!(!fixture
            .run_policy("output", Some("midpoint"))
            .status
            .success());
        assert!(!fixture.0.join("output").exists());
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn midpoint_rejects_actor_numeric_overflow_without_publishing() {
    let fixture = Fixture::new();
    let request = fixture.request(false, false);
    let path = Path::new(request["pc"]["actor_checkpoint"].as_str().unwrap());
    let mut actor: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    actor["weights"][0] = json!(1e100);
    let bytes = serde_json::to_vec(&actor).unwrap();
    fs::write(path, &bytes).unwrap();
    assert!(!fixture
        .run_policy("output", Some("midpoint"))
        .status
        .success());
    assert!(!fixture.0.join("output").exists());
    assert_eq!(fs::read(path).unwrap(), bytes);
}

#[test]
fn unknown_conflict_policy_fails_before_admission_or_output_creation() {
    let fixture = Fixture::new();
    let result = fixture.run_policy("output", Some("unknown"));
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("usage:"));
    assert!(!fixture.0.join("output").exists());
}
