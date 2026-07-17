use std::fs;
use std::path::{Path, PathBuf};

use alife_tools::creature_part_builder::validate_geneforge_staging;
use serde_json::Value;

fn workspace_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join(relative)
}

fn copy_tree(source: &Path, destination: &Path) {
    if destination.exists() {
        fs::remove_dir_all(destination).unwrap();
    }
    for entry in walk(source) {
        let relative = entry.strip_prefix(source).unwrap();
        let target = destination.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&target).unwrap();
        } else {
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::copy(entry, target).unwrap();
        }
    }
}

fn walk(root: &Path) -> Vec<PathBuf> {
    let mut pending = vec![root.to_path_buf()];
    let mut paths = Vec::new();
    while let Some(path) = pending.pop() {
        paths.push(path.clone());
        if path.is_dir() {
            let mut children = fs::read_dir(path)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect::<Vec<_>>();
            children.sort();
            pending.extend(children.into_iter().rev());
        }
    }
    paths
}

fn fixture_staging() -> PathBuf {
    workspace_path("target/artifacts/geneforge-import-tests/staging-a")
}

fn mutation_root(name: &str) -> PathBuf {
    workspace_path(&format!(
        "target/artifacts/creature_parts/validator-mutations/{name}"
    ))
}

fn first_with_extension(root: &Path, extension: &str) -> PathBuf {
    walk(root)
        .into_iter()
        .find(|path| path.extension().is_some_and(|value| value == extension))
        .unwrap()
}

fn replace_first_line(path: &Path, prefix: &str, replacement: &str) {
    let text = fs::read_to_string(path).unwrap();
    let mut replaced = false;
    let output = text
        .lines()
        .map(|line| {
            if !replaced && line.starts_with(prefix) {
                replaced = true;
                replacement
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert!(replaced, "{path:?} has no {prefix:?} line");
    fs::write(path, output).unwrap();
}

fn assert_rejected(root: &Path, expected: &str) {
    let error = validate_geneforge_staging(root).unwrap_err().to_string();
    assert!(
        error.contains(expected),
        "expected {expected:?} in validator error, got {error:?}"
    );
}

#[test]
fn fixture_and_real_staged_outputs_pass_the_complete_visual_contract() {
    let fixture = fixture_staging();
    assert!(
        fixture.join("build_receipt.json").is_file(),
        "run python scripts/test_geneforge_creature_recipes.py first"
    );
    let fixture_receipt = validate_geneforge_staging(&fixture).unwrap();
    assert_eq!(fixture_receipt.donor_count, 3);
    assert_eq!(fixture_receipt.lod_count, 9);
    assert_eq!(fixture_receipt.obj_count, 9);
    assert_eq!(fixture_receipt.mask_count, 9);

    let real = workspace_path("target/artifacts/creature_parts/geneforge-staging");
    assert!(
        real.join("build_receipt.json").is_file(),
        "run the real Task 4 staged build before this integration gate"
    );
    let real_receipt = validate_geneforge_staging(&real).unwrap();
    assert_eq!(real_receipt.donor_count, 3);
    assert_eq!(real_receipt.lod_count, 9);
    assert!(real_receipt.total_bytes <= 8 * 1024 * 1024);
}

#[test]
fn staged_validator_rejects_obj_uv_normal_and_digest_corruption() {
    let source = fixture_staging();

    let root = mutation_root("obj-index");
    copy_tree(&source, &root);
    replace_first_line(
        &first_with_extension(&root, "obj"),
        "f ",
        "f 999999/1/1 2/2/2 3/3/3",
    );
    assert_rejected(&root, "OBJ index");

    let root = mutation_root("uv");
    copy_tree(&source, &root);
    replace_first_line(
        &first_with_extension(&root, "obj"),
        "vt ",
        "vt 2.000000000 0.500000000",
    );
    assert_rejected(&root, "UV");

    let root = mutation_root("normal");
    copy_tree(&source, &root);
    replace_first_line(
        &first_with_extension(&root, "obj"),
        "vn ",
        "vn 0.000000000 0.000000000 0.000000000",
    );
    assert_rejected(&root, "normal");

    let root = mutation_root("digest");
    copy_tree(&source, &root);
    let obj = first_with_extension(&root, "obj");
    fs::write(
        &obj,
        [fs::read(&obj).unwrap(), b"# drift\n".to_vec()].concat(),
    )
    .unwrap();
    assert_rejected(&root, "digest");
}

fn mutate_socket(root: &Path, mutation: impl FnOnce(&mut Value)) {
    let socket = walk(root)
        .into_iter()
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with("_sockets.json"))
        })
        .unwrap();
    let mut value: Value = serde_json::from_str(&fs::read_to_string(&socket).unwrap()).unwrap();
    mutation(&mut value);
    fs::write(socket, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}

#[test]
fn staged_validator_rejects_bounds_sockets_landmarks_grounding_and_detachment() {
    let source = fixture_staging();

    let root = mutation_root("bounds");
    copy_tree(&source, &root);
    mutate_socket(&root, |value| {
        value["bounds"]["max"] = value["bounds"]["min"].clone()
    });
    assert_rejected(&root, "bounds");

    let root = mutation_root("socket");
    copy_tree(&source, &root);
    mutate_socket(&root, |value| {
        value["sockets"]["neck"]["rotation_xyzw"] = serde_json::json!([0, 0, 0, 0])
    });
    assert_rejected(&root, "socket");

    let root = mutation_root("landmark");
    copy_tree(&source, &root);
    mutate_socket(&root, |value| {
        value["landmarks"]
            .as_object_mut()
            .unwrap()
            .remove("left-foot");
    });
    assert_rejected(&root, "landmark");

    let root = mutation_root("grounding");
    copy_tree(&source, &root);
    mutate_socket(&root, |value| {
        value["ground_contacts"][0][1] = serde_json::json!(4.0)
    });
    assert_rejected(&root, "ground");

    let root = mutation_root("detachment");
    copy_tree(&source, &root);
    mutate_socket(&root, |value| {
        value["sockets"]["neck"]["translation"][0] = serde_json::json!(100.0)
    });
    assert_rejected(&root, "detached");
}

#[test]
fn staged_validator_rejects_missing_masks_and_budget_overrun() {
    let source = fixture_staging();

    let root = mutation_root("mask");
    copy_tree(&source, &root);
    fs::remove_file(first_with_extension(&root, "png")).unwrap();
    assert_rejected(&root, "mask");

    let root = mutation_root("budget");
    copy_tree(&source, &root);
    let obj = first_with_extension(&root, "obj");
    fs::write(&obj, vec![b' '; 512 * 1024 + 1]).unwrap();
    assert_rejected(&root, "512 KiB");
}
