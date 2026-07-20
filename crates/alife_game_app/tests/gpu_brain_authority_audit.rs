//! Positive GPU neural-authority evidence plus a guarded production-source audit.
#![cfg(feature = "gpu-runtime")]

use std::{fs, path::Path};

use alife_core::{BrainCapacityClass, PolicyBackend, SensorProfile};
use alife_game_app::{run_gpu_closed_loop_acceptance, GpuClosedLoopAcceptanceOptions};

#[test]
fn production_receipt_has_one_gpu_neural_authority() {
    let receipt = run_gpu_closed_loop_acceptance(GpuClosedLoopAcceptanceOptions {
        capacity: BrainCapacityClass::n512(),
        requested_ticks: 4,
        deterministic_seed: 4_101,
        sensor_profile: SensorProfile::PrivilegedAffordanceV1,
    })
    .unwrap();

    assert!(receipt.authoritative);
    assert_eq!(receipt.policy_backend, PolicyBackend::NeuralClosedLoopGpu);
    assert!(receipt.neural_dispatch_count > 0);
    assert_eq!(receipt.neural_dispatch_count, receipt.gpu_selection_count);
    assert!(receipt.compact_readback_bytes <= 64);
}

#[test]
fn production_sources_retain_only_the_gpu_neural_execution_path() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repository_root = crate_root
        .parent()
        .and_then(Path::parent)
        .expect("game app crate must be inside the workspace");
    let private_legacy = repository_root.join("crates/alife_world/src/legacy_neural_policy_v1.rs");
    let world_lib = fs::read_to_string(repository_root.join("crates/alife_world/src/lib.rs"))
        .expect("world crate root must be readable");
    assert!(world_lib.contains("mod legacy_neural_policy_v1;"));
    assert!(!world_lib.contains("pub mod legacy_neural_policy_v1;"));

    let forbidden_compact = [
        ["cpu", "shadow"].concat(),
        ["auto", "with", "cpu", "fallback"].concat(),
        ["cpu", "reference"].concat(),
        ["neural", "fallback"].concat(),
        ["full", "gpu", "runtime", "mode"].concat(),
        ["parity", "gate"].concat(),
        ["parity", "gated"].concat(),
        ["parity", "gating"].concat(),
    ];
    let mut files = Vec::new();
    for relative in [
        "crates/alife_core/src",
        "crates/alife_gpu_backend/src",
        "crates/alife_world/src",
        "crates/alife_game_app/src",
        "crates/alife_tools/src",
        "scripts",
    ] {
        collect_files(&repository_root.join(relative), &mut files);
    }
    files.sort();

    let mut violations = Vec::new();
    for path in files {
        if path == private_legacy {
            continue;
        }
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let compact = source
            .to_ascii_lowercase()
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .collect::<String>();
        for forbidden in &forbidden_compact {
            if compact.contains(forbidden) {
                violations.push(format!(
                    "{} contains superseded neural authority token `{forbidden}`",
                    path.strip_prefix(repository_root)
                        .unwrap_or(&path)
                        .display()
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "superseded production neural authority surfaces remain:\n{}",
        violations.join("\n")
    );
}

fn collect_files(root: &Path, output: &mut Vec<std::path::PathBuf>) {
    let mut entries = fs::read_dir(root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
        .map(|entry| {
            entry
                .expect("source directory entry must be readable")
                .path()
        })
        .collect::<Vec<_>>();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_files(&path, output);
        } else {
            output.push(path);
        }
    }
}
