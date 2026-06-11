use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("alife_tools should live under crates/")
        .to_path_buf()
}

fn collect_files(root: &Path, files: &mut Vec<PathBuf>) {
    let ignored_dirs = ["target", "graphify-out", ".git"];
    for entry in fs::read_dir(root).expect("read_dir should succeed") {
        let entry = entry.expect("directory entry should be readable");
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if ignored_dirs.contains(&name.as_ref()) {
                continue;
            }
            collect_files(&path, files);
        } else {
            files.push(path);
        }
    }
}

#[test]
fn plan_pack_and_progress_logs_are_discoverable() {
    let root = workspace_root();
    for required in [
        "docs/codex_plan_pack/prompts/CODEX_MASTER_PROMPT.md",
        "docs/codex_plan_pack/plan_manifest.json",
        "docs/codex_progress/PLAN_PROGRESS.md",
        "docs/codex_progress/DECISION_LOG.md",
        "docs/codex_progress/SPEC_TRACEABILITY.md",
        "docs/architecture/schema_versioning.md",
    ] {
        assert!(root.join(required).is_file(), "missing {required}");
    }
}

#[test]
fn progress_log_records_serial_baseline_plans() {
    let root = workspace_root();
    let progress = fs::read_to_string(root.join("docs/codex_progress/PLAN_PROGRESS.md"))
        .expect("progress log should be readable");

    for plan in ["P00", "P01", "P02"] {
        assert!(
            progress.contains(&format!("| {plan} |")) && progress.contains("| complete"),
            "progress log should record {plan} as complete"
        );
    }
}

#[test]
fn forbidden_engine_artifact_extensions_are_absent() {
    let root = workspace_root();
    let mut files = Vec::new();
    collect_files(&root, &mut files);

    let forbidden = ["cs", "csproj", "sln", "unity", "hlsl"];
    let offenders: Vec<_> = files
        .into_iter()
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| forbidden.contains(&ext.to_ascii_lowercase().as_str()))
                .unwrap_or(false)
        })
        .collect();

    assert!(
        offenders.is_empty(),
        "forbidden engine/shader artifacts found: {offenders:?}"
    );
}
