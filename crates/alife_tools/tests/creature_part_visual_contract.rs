use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use alife_tools::creature_part_builder::{sha256_hex, validate_geneforge_staging};
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
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
    validator_fixture().join("staging")
}

fn fixture_recipe() -> PathBuf {
    validator_fixture().join("fixture_recipes.json")
}

fn validator_fixture() -> &'static PathBuf {
    static FIXTURE: OnceLock<PathBuf> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let source = workspace_path("target/artifacts/geneforge-import-tests/staging-a");
        assert!(
            source.join("build_receipt.json").is_file(),
            "run python scripts/test_geneforge_creature_recipes.py first"
        );
        let root = workspace_path("target/artifacts/creature_parts/validator-fixture");
        let staging = root.join("staging");
        copy_tree(&source, &staging);

        let mut recipe: Value =
            serde_json::from_slice(&fs::read(production_recipe()).unwrap()).unwrap();
        for asset in recipe["part_assets"].as_array_mut().unwrap() {
            for lod in asset["lods"].as_array_mut().unwrap() {
                for (path_field, digest_field) in [
                    ("generated_obj", "generated_obj_sha256"),
                    ("socket_manifest", "socket_manifest_sha256"),
                    ("semantic_mask", "semantic_mask_sha256"),
                ] {
                    let relative = lod[path_field].as_str().unwrap();
                    lod[digest_field] =
                        serde_json::json!(sha256_hex(&fs::read(staging.join(relative)).unwrap()));
                }
            }
        }
        let recipe_path = root.join("fixture_recipes.json");
        fs::create_dir_all(&root).unwrap();
        let recipe_digest = write_recipe_with_canonical_digest(&recipe_path, recipe.clone());

        let receipt_path = staging.join("build_receipt.json");
        let mut receipt: Value = serde_json::from_slice(&fs::read(&receipt_path).unwrap()).unwrap();
        receipt["recipe_sha256"] = serde_json::json!(recipe_digest);
        receipt["source_sha256"] = serde_json::json!({
            "norn": recipe["sources"][0]["sha256"].as_str().unwrap(),
            "ettin": recipe["sources"][1]["sha256"].as_str().unwrap(),
            "grendel": recipe["sources"][2]["sha256"].as_str().unwrap(),
        });
        fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
        rewrite_all_receipt_digests(&staging);
        root
    })
}

fn production_recipe() -> PathBuf {
    workspace_path(
        "crates/alife_game_app/assets/production_voxel_v1/creature_parts/geneforge_recipes.json",
    )
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

fn replace_first_object_with_second(path: &Path) -> (String, String) {
    let text = fs::read_to_string(path).unwrap();
    let objects = text
        .lines()
        .filter(|line| line.starts_with("o "))
        .take(2)
        .collect::<Vec<_>>();
    assert_eq!(
        objects.len(),
        2,
        "{path:?} must declare at least two components"
    );
    replace_first_line(path, "o ", objects[1]);
    (objects[0][2..].to_string(), objects[1][2..].to_string())
}

fn assert_rejected_with_recipe(root: &Path, recipe: &Path, expected: &str) {
    let error = validate_geneforge_staging(root, recipe)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains(expected),
        "expected {expected:?} in validator error, got {error:?}"
    );
}

fn assert_rejected(root: &Path, expected: &str) {
    assert_rejected_with_recipe(root, &fixture_recipe(), expected);
}

#[test]
fn fixture_and_real_staged_outputs_pass_the_complete_visual_contract() {
    let fixture = fixture_staging();
    assert!(
        fixture.join("build_receipt.json").is_file(),
        "run python scripts/test_geneforge_creature_recipes.py first"
    );
    let fixture_receipt = validate_geneforge_staging(&fixture, &fixture_recipe()).unwrap();
    assert_eq!(fixture_receipt.donor_count, 3);
    assert_eq!(fixture_receipt.asset_count, 14);
    assert_eq!(fixture_receipt.lod_count, 42);
    assert_eq!(fixture_receipt.obj_count, 42);
    assert_eq!(fixture_receipt.mask_count, 42);

    let real = workspace_path("target/artifacts/creature_parts/geneforge-staging");
    assert!(
        real.join("build_receipt.json").is_file(),
        "run the real Task 4 staged build before this integration gate"
    );
    let real_receipt = validate_geneforge_staging(&real, &production_recipe()).unwrap();
    assert_eq!(real_receipt.donor_count, 3);
    assert_eq!(real_receipt.asset_count, 14);
    assert_eq!(real_receipt.lod_count, 42);
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
    mutate_socket_matching(root, |_| true, mutation);
}

fn mutate_socket_matching(
    root: &Path,
    predicate: impl Fn(&str) -> bool,
    mutation: impl FnOnce(&mut Value),
) {
    let socket = walk(root)
        .into_iter()
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with("_sockets.json") && predicate(name))
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
    mutate_socket_matching(
        &root,
        |name| name.contains("_legs_full_sockets.json"),
        |value| value["ground_contacts"][0][1] = serde_json::json!(4.0),
    );
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

    let root = mutation_root("uniform-mask");
    copy_tree(&source, &root);
    let mask = first_with_extension(&root, "png");
    let mut image = image::open(&mask).unwrap().to_rgba8();
    for pixel in image.pixels_mut() {
        if pixel[3] > 0 {
            pixel[3] = 127;
        }
    }
    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut bytes, ImageFormat::Png)
        .unwrap();
    fs::write(mask, bytes.into_inner()).unwrap();
    assert_rejected(&root, "microdetail");

    let root = mutation_root("budget");
    copy_tree(&source, &root);
    let obj = first_with_extension(&root, "obj");
    fs::write(&obj, vec![b' '; 512 * 1024 + 1]).unwrap();
    assert_rejected(&root, "512 KiB");
}

#[test]
fn staged_validator_rejects_component_loss_and_asset_independent_mask_colors() {
    let source = fixture_staging();

    let root = mutation_root("lod-component-loss");
    copy_tree(&source, &root);
    let compact_obj = walk(&root)
        .into_iter()
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with("_compact_parts.obj"))
        })
        .unwrap();
    let (removed_component, retained_component) = replace_first_object_with_second(&compact_obj);
    let socket_name = compact_obj
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .replace("_parts.obj", "_sockets.json");
    let socket_path = compact_obj.with_file_name(socket_name);
    let mut socket: Value = serde_json::from_slice(&fs::read(&socket_path).unwrap()).unwrap();
    let topology = &mut socket["lod_topology"];
    let removed_triangles = topology["component_triangle_counts"]
        .as_object_mut()
        .unwrap()
        .remove(&removed_component)
        .unwrap()
        .as_u64()
        .unwrap();
    let retained_triangles = topology["component_triangle_counts"][&retained_component]
        .as_u64()
        .unwrap();
    topology["component_triangle_counts"][&retained_component] =
        serde_json::json!(removed_triangles + retained_triangles);
    let removed_islands = topology["component_connected_counts"]
        .as_object_mut()
        .unwrap()
        .remove(&removed_component)
        .unwrap()
        .as_u64()
        .unwrap();
    let retained_islands = topology["component_connected_counts"][&retained_component]
        .as_u64()
        .unwrap();
    topology["component_connected_counts"][&retained_component] =
        serde_json::json!(removed_islands + retained_islands);
    topology["component_ids"]
        .as_array_mut()
        .unwrap()
        .retain(|value| value.as_str() != Some(removed_component.as_str()));
    fs::write(socket_path, serde_json::to_vec_pretty(&socket).unwrap()).unwrap();
    assert_rejected(&root, "component");

    let root = mutation_root("asset-independent-mask-colors");
    copy_tree(&source, &root);
    let mask = first_with_extension(&root, "png");
    let image = RgbaImage::from_fn(64, 64, |x, y| {
        let colors = [
            [230, 92, 88],
            [64, 166, 184],
            [244, 177, 76],
            [95, 177, 104],
        ];
        let color = colors[(y as usize / 16).min(colors.len() - 1)];
        Rgba([color[0], color[1], color[2], 32 + ((x + y) % 192) as u8])
    });
    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut bytes, ImageFormat::Png)
        .unwrap();
    fs::write(mask, bytes.into_inner()).unwrap();
    assert_rejected(&root, "semantic colors");
}

#[test]
fn staged_validator_binds_external_recipe_sources_importer_and_assembly_metadata() {
    let source = fixture_staging();

    let root = mutation_root("receipt-recipe");
    copy_tree(&source, &root);
    let receipt_path = root.join("build_receipt.json");
    let mut receipt: Value = serde_json::from_slice(&fs::read(&receipt_path).unwrap()).unwrap();
    receipt["recipe_sha256"] = serde_json::json!("0".repeat(64));
    fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    assert_rejected(&root, "recipe digest");

    let root = mutation_root("receipt-importer");
    copy_tree(&source, &root);
    let receipt_path = root.join("build_receipt.json");
    let mut receipt: Value = serde_json::from_slice(&fs::read(&receipt_path).unwrap()).unwrap();
    receipt["importer_version"] = serde_json::json!("unreviewed-importer");
    fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    assert_rejected(&root, "importer version");

    let root = mutation_root("receipt-source");
    copy_tree(&source, &root);
    let receipt_path = root.join("build_receipt.json");
    let mut receipt: Value = serde_json::from_slice(&fs::read(&receipt_path).unwrap()).unwrap();
    receipt["source_sha256"]["norn"] = serde_json::json!("f".repeat(64));
    fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    assert_rejected(&root, "source digest");

    let root = mutation_root("external-recipe");
    copy_tree(&source, &root);
    assert_rejected_with_recipe(&root, &production_recipe(), "recipe digest");

    let root = mutation_root("assembly-preparation");
    copy_tree(&source, &root);
    mutate_socket(&root, |value| {
        value["assembly_preparations"].as_array_mut().unwrap().pop();
    });
    assert_rejected(&root, "assembly preparation");

    let root = mutation_root("lod-topology");
    copy_tree(&source, &root);
    mutate_socket(&root, |value| {
        value["lod_topology"]["connected_components"] = serde_json::json!(999)
    });
    assert_rejected(&root, "topology");

    let root = mutation_root("external-output-digest");
    copy_tree(&source, &root);
    let obj = first_with_extension(&root, "obj");
    fs::write(
        &obj,
        [
            fs::read(&obj).unwrap(),
            b"# receipt-backed replacement\n".to_vec(),
        ]
        .concat(),
    )
    .unwrap();
    rewrite_receipt_digest(&root, &obj);
    assert_rejected(&root, "external catalog digest");

    let root = mutation_root("prepared-matrix-linear");
    copy_tree(&source, &root);
    mutate_socket(&root, |value| {
        value["assembly_preparations"][0]["prepared_matrix"][0] = serde_json::json!(9.0)
    });
    rewrite_all_receipt_digests(&root);
    assert_rejected(&root, "prepared matrix");

    let root = mutation_root("prepared-matrix-bottom-row");
    copy_tree(&source, &root);
    mutate_socket(&root, |value| {
        value["assembly_preparations"][0]["prepared_matrix"][15] = serde_json::json!(2.0)
    });
    rewrite_all_receipt_digests(&root);
    assert_rejected(&root, "prepared matrix");

    let root = mutation_root("bridge-target-anchor");
    copy_tree(&source, &root);
    mutate_socket(&root, |value| {
        value["assembly_preparations"][0]["bridge_geometry"][0]["target_anchor"][0] =
            serde_json::json!(99.0)
    });
    rewrite_all_receipt_digests(&root);
    assert_rejected(&root, "bridge geometry");

    let root = mutation_root("bridge-runtime-group");
    copy_tree(&source, &root);
    mutate_socket(&root, |value| {
        value["assembly_preparations"][0]["bridge_geometry"][0]["runtime_group"] =
            serde_json::json!("wrong-group")
    });
    rewrite_all_receipt_digests(&root);
    assert_rejected(&root, "bridge geometry");

    let root = mutation_root("full-catalog-validation");
    copy_tree(&source, &root);
    let mut recipe: Value = serde_json::from_slice(&fs::read(fixture_recipe()).unwrap()).unwrap();
    recipe["families"][0]["parts"]["head"]["fit"]["scale"] = serde_json::json!([1.0, 1.01, 1.0]);
    let recipe_digest =
        write_recipe_with_canonical_digest(&root.join("fixture_recipes.json"), recipe);
    let receipt_path = root.join("build_receipt.json");
    let mut receipt: Value = serde_json::from_slice(&fs::read(&receipt_path).unwrap()).unwrap();
    receipt["recipe_sha256"] = serde_json::json!(recipe_digest);
    fs::write(receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    assert_rejected_with_recipe(
        &root,
        &root.join("fixture_recipes.json"),
        "invalid external GeneForge recipe",
    );
}

fn write_recipe_with_canonical_digest(path: &Path, mut recipe: Value) -> String {
    recipe["recipe_sha256"] = serde_json::json!("0".repeat(64));
    let digest = sha256_hex(&serde_json::to_vec(&recipe).unwrap());
    recipe["recipe_sha256"] = serde_json::json!(digest.clone());
    fs::write(path, serde_json::to_vec_pretty(&recipe).unwrap()).unwrap();
    digest
}

fn rewrite_receipt_digest(root: &Path, changed: &Path) {
    let receipt_path = root.join("build_receipt.json");
    let mut receipt: Value = serde_json::from_slice(&fs::read(&receipt_path).unwrap()).unwrap();
    let relative = changed
        .strip_prefix(root)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    receipt["outputs"][relative] = serde_json::json!(sha256_hex(&fs::read(changed).unwrap()));
    fs::write(receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
}

fn rewrite_all_receipt_digests(root: &Path) {
    let receipt_path = root.join("build_receipt.json");
    let mut receipt: Value = serde_json::from_slice(&fs::read(&receipt_path).unwrap()).unwrap();
    let paths = receipt["outputs"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    for relative in paths {
        receipt["outputs"][&relative] =
            serde_json::json!(sha256_hex(&fs::read(root.join(&relative)).unwrap()));
    }
    fs::write(receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
}
