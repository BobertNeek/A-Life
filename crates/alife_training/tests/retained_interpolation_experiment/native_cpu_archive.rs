//! Supported native CPU output and matched synthetic heads. No real parent or
//! Current export/conversion is involved; the legacy Current control is a fixture.
use super::*;
use std::{fs, path::PathBuf, process::Command};
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "alife-native-cpu-interpolation-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn native_cpu_archive_prepares_same_candidate_as_matched_heads_and_cli_without_current_export() {
    let root = Directory::new();
    let native = root.0.join("native");
    run_cpu_continuation(
        CpuContinuationConfig {
            previous: None,
            output: native.clone(),
            seed: 0x748800,
            ticks: 2,
            temperature: 1.,
            fixture: true,
        },
        &mut DefaultCpuEpisode,
    )
    .unwrap();
    assert!(!native.join("current.alife-checkpoint").exists());
    let source = read_cpu_founder_source(&native).unwrap();
    assert!(source.handoff.actor.optimizer_step > 0);
    let binding = inspect_native_cpu_archive(&native).unwrap();
    assert_eq!(binding.policy_version, source.handoff.policy_version);
    assert_eq!(
        binding.private_collector_policy_version,
        source.receipt["source_policy_version"].as_u64().unwrap()
    );
    assert_ne!(
        binding.policy_version,
        binding.private_collector_policy_version
    );
    let archive = capture_native_cpu_archive(&native, &binding.tree_sha256).unwrap();
    let original = archive.tree_sha256().unwrap();
    let admitted = source.handoff.clone().admit().unwrap();
    let mut weights = admitted.actor.weights.clone();
    let changed = admitted
        .phenotype
        .synapses()
        .iter()
        .position(|s| matches!(s.kind(), CompiledSynapseKind::Decoder(_)))
        .unwrap();
    weights[changed] += 0.001;
    let asset = FoundationWeightAsset::from_trained_weights(
        &admitted.phenotype,
        weights,
        admitted.foundation.manifest().training_stage(),
    )
    .unwrap();
    let (_, pc_inputs) = PhenotypeCompiler::compile_n2048_foundation_candidate(
        source.handoff.compiler_inputs.genome().clone(),
        source.handoff.compiler_inputs.development().clone(),
        asset,
    )
    .unwrap();
    let mut control = fixture_from_inputs(pc_inputs, source.handoff.compiler_inputs.clone(), 1);
    let mut pc = CurrentTrainingCheckpointV1::decode(&control.pc_current).unwrap();
    pc.handoff.actor.stage_mask = source.handoff.actor.stage_mask.clone();
    control.pc_current = pc.encode().unwrap();
    repin(&mut control);
    // A separately labeled synthetic Current fixture supplies exactly matching
    // global heads to test format independence. This is never a real exporter.
    let mut cloud_fixture = CurrentTrainingCheckpointV1::decode(cloud_current(&control)).unwrap();
    cloud_fixture.handoff = source.handoff.clone();
    set_cloud_current(&mut control, cloud_fixture.encode().unwrap());
    let matched = prepare_retained_interpolation_experiment(control.clone()).unwrap();
    let mut inputs = control.clone();
    inputs.cloud_parent = InterpolationCloudParent::NativeCpuArchive {
        archive: archive.clone(),
    };
    let candidate = prepare_retained_interpolation_experiment(inputs.clone()).unwrap();
    assert_eq!(
        serde_json::to_vec(candidate.compiler_inputs()).unwrap(),
        serde_json::to_vec(matched.compiler_inputs()).unwrap()
    );
    assert_eq!(
        serde_json::to_vec(candidate.retained()).unwrap(),
        serde_json::to_vec(matched.retained()).unwrap()
    );
    assert_eq!(candidate.receipt().cloud_parent_kind, "native_cpu_archive");
    let retained_archive = match &candidate.inputs().cloud_parent {
        InterpolationCloudParent::NativeCpuArchive { archive } => archive,
        _ => panic!("native archive replaced"),
    };
    assert_eq!(retained_archive.files, archive.files);
    assert_eq!(
        retained_archive.files["actor-checkpoint.json"],
        fs::read(native.join("actor-checkpoint.json")).unwrap()
    );
    assert_eq!(
        retained_archive.files["value-checkpoint.json"],
        fs::read(native.join("value-checkpoint.json")).unwrap()
    );
    assert_eq!(candidate.inputs().pc_current, inputs.pc_current);
    candidate.validate().unwrap();
    let mut zero = inputs.clone();
    zero.cloud_fraction = 0.;
    let zero = prepare_retained_interpolation_experiment(zero).unwrap();
    assert_eq!(
        zero.compiler_inputs()
            .n2048_candidate_asset()
            .unwrap()
            .weights(),
        pc.handoff.actor.weights
    );
    assert_eq!(
        zero.retained().fast.bank_0_bits,
        candidate.retained().fast.bank_0_bits
    );
    assert_eq!(
        zero.retained().lifetime.bank_1_bits,
        candidate.retained().lifetime.bank_1_bits
    );
    for mutation in 0..3 {
        let mut bad = archive.clone();
        if mutation == 0 {
            bad.expected_tree_sha256 = "0".repeat(64);
        } else if mutation == 1 {
            bad.files.get_mut("actor-checkpoint.json").unwrap()[0] ^= 1;
            bad.expected_tree_sha256 = bad.tree_sha256().unwrap();
        } else {
            let file = source.receipt["private_individual_archive"]
                .as_str()
                .unwrap();
            bad.files.remove(file);
            bad.expected_tree_sha256 = bad.tree_sha256().unwrap();
        }
        let mut invalid = inputs.clone();
        invalid.cloud_parent = InterpolationCloudParent::NativeCpuArchive { archive: bad };
        assert!(prepare_retained_interpolation_experiment(invalid).is_err());
    }
    let pc_path = root.0.join("pc.alife-checkpoint");
    fs::write(&pc_path, &inputs.pc_current).unwrap();
    let output = root.0.join("candidate");
    let spec = root.0.join("request.json");
    fs::write(&spec,serde_json::to_vec(&serde_json::json!({"pc_current":pc_path,"expected_pc_current_sha256":inputs.expected_pc_current_sha256,"cloud_native_cpu_root":native,"expected_cloud_tree_sha256":binding.tree_sha256,"cloud_fraction":0.5,"output":output})).unwrap()).unwrap();
    let binary = env!("CARGO_BIN_EXE_prepare_retained_interpolation");
    let run = Command::new(binary)
        .arg("prepare")
        .arg(&spec)
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let verify = Command::new(binary)
        .arg("verify")
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        verify.status.success(),
        "{}",
        String::from_utf8_lossy(&verify.stderr)
    );
    assert_eq!(
        fs::read(output.join("candidate-compiler-inputs.json")).unwrap(),
        serde_json::to_vec(candidate.compiler_inputs()).unwrap()
    );
    assert_eq!(
        fs::read(output.join("candidate-retained-neural.json")).unwrap(),
        serde_json::to_vec(candidate.retained()).unwrap()
    );
    assert!(!Command::new(binary)
        .arg("prepare")
        .arg(&spec)
        .status()
        .unwrap()
        .success());
    assert!(!output.join("training-handoff.json").exists());
    assert!(!output.join("actor-checkpoint.json").exists());
    assert_eq!(
        inspect_native_cpu_archive(&native).unwrap().tree_sha256,
        original
    );
    assert_eq!(fs::read(pc_path).unwrap(), inputs.pc_current);
}

#[cfg(unix)]
#[test]
fn archive_capture_rejects_symlinks_and_noncanonical_embedded_paths() {
    let root = Directory::new();
    fs::write(root.0.join("source"), b"preserved").unwrap();
    std::os::unix::fs::symlink(root.0.join("source"), root.0.join("link")).unwrap();
    assert!(inspect_native_cpu_archive(&root.0)
        .unwrap_err()
        .to_string()
        .contains("symlinks refused"));
    let archive = NativeCpuParentArchive {
        files: std::collections::BTreeMap::from([("../outside".into(), b"preserved".to_vec())]),
        expected_tree_sha256: String::new(),
    };
    assert!(archive.tree_sha256().is_err());
    assert_eq!(fs::read(root.0.join("source")).unwrap(), b"preserved");
}
