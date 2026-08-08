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
fn current_documentation_authorities_are_discoverable() {
    let root = workspace_root();
    for required in [
        "README.md",
        "docs/VISION.md",
        "docs/STATUS.md",
        "docs/ARCHITECTURE.md",
        "docs/ROADMAP.md",
        "docs/DEVELOPMENT.md",
        "docs/EVIDENCE.md",
        "docs/REFERENCE.md",
    ] {
        assert!(root.join(required).is_file(), "missing {required}");
    }
}

#[test]
fn current_status_records_the_open_product_boundaries() {
    let root = workspace_root();
    let status =
        fs::read_to_string(root.join("docs/STATUS.md")).expect("current status should be readable");

    for required in [
        "active voxel renderer remains a save-derived projection",
        "Autonomous birth, ageing, reproduction, and death",
        "Its promotion verdict is `Blocked`",
    ] {
        assert!(
            status.contains(required),
            "current status should record: {required}"
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
