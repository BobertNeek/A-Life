use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

use alife_game_app::{
    load_generated_part_pack, CreaturePartCatalog, CreaturePartFamilyDefinition, CreaturePartLodId,
    CreaturePartSlot, SocketFrame,
};
use alife_tools::creature_part_builder::{
    slice_creature_mesh, validate_sliced_pack, GeneratedPartMesh, ObjVertex,
    SlicedCreaturePartPack, SourceObjMesh,
};
use alife_world::{persistence::PortableAssetDigest, CreaturePartFamilyId};
use image::{ImageBuffer, Rgba};
use serde::Serialize;
use serde_json::{json, Value};

const DEFAULT_CATALOG: &str =
    "crates/alife_game_app/assets/production_voxel_v1/creature_parts/catalog.json";
const DEFAULT_STAGING: &str = "target/generated_art/creature_parts/staging";
const DEFAULT_MANIFEST: &str =
    "crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json";

#[derive(Debug, Clone, PartialEq, Eq)]
enum CreaturePartBuilderCommand {
    Analyze {
        catalog: PathBuf,
        family: u16,
        lod: CreaturePartLodId,
        json: PathBuf,
    },
    Build {
        catalog: PathBuf,
        family: Option<u16>,
        staging: PathBuf,
    },
    Validate {
        catalog: PathBuf,
    },
    Preview {
        catalog: PathBuf,
        family: u16,
        lod: CreaturePartLodId,
        output: PathBuf,
    },
    Manifest {
        catalog: PathBuf,
        manifest: PathBuf,
    },
}

impl CreaturePartBuilderCommand {
    #[cfg(test)]
    fn parse_for_test<I, S>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Self::parse(
            args.into_iter()
                .map(|arg| arg.as_ref().to_string())
                .collect(),
        )
    }

    fn parse(args: Vec<String>) -> Result<Self, String> {
        let Some(command) = args.first().map(String::as_str) else {
            return Err("expected analyze, build, validate, preview, or manifest".to_string());
        };
        let options = parse_options(&args[1..])?;
        let catalog = PathBuf::from(
            options
                .get("catalog")
                .map(String::as_str)
                .unwrap_or(DEFAULT_CATALOG),
        );
        match command {
            "analyze" => Ok(Self::Analyze {
                catalog,
                family: parse_u16_option(&options, "family", 0)?,
                lod: parse_lod(options.get("lod").map(String::as_str).unwrap_or("compact"))?,
                json: PathBuf::from(
                    options
                        .get("json")
                        .map(String::as_str)
                        .unwrap_or("target/artifacts/creature_parts/analysis.json"),
                ),
            }),
            "build" => Ok(Self::Build {
                catalog,
                family: options
                    .get("family")
                    .map(|value| {
                        value
                            .parse::<u16>()
                            .map_err(|_| "invalid --family".to_string())
                    })
                    .transpose()?,
                staging: PathBuf::from(
                    options
                        .get("staging")
                        .map(String::as_str)
                        .unwrap_or(DEFAULT_STAGING),
                ),
            }),
            "validate" => Ok(Self::Validate { catalog }),
            "preview" => {
                let output = PathBuf::from(
                    options
                        .get("output")
                        .map(String::as_str)
                        .unwrap_or("target/artifacts/creature_parts/preview.png"),
                );
                if !is_target_artifact_path(&output) {
                    return Err(
                        "preview output must be under target/artifacts/creature_parts".into(),
                    );
                }
                Ok(Self::Preview {
                    catalog,
                    family: parse_u16_option(&options, "family", 0)?,
                    lod: parse_lod(options.get("lod").map(String::as_str).unwrap_or("compact"))?,
                    output,
                })
            }
            "manifest" => Ok(Self::Manifest {
                catalog,
                manifest: PathBuf::from(
                    options
                        .get("manifest")
                        .map(String::as_str)
                        .unwrap_or(DEFAULT_MANIFEST),
                ),
            }),
            _ => Err(format!("unknown creature part builder command {command}")),
        }
    }

    fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            Self::Analyze {
                catalog,
                family,
                lod,
                json,
            } => analyze(&catalog, CreaturePartFamilyId(family), lod, &json),
            Self::Build {
                catalog,
                family,
                staging,
            } => build(&catalog, family.map(CreaturePartFamilyId), &staging),
            Self::Validate { catalog } => validate(&catalog),
            Self::Preview {
                catalog,
                family,
                lod,
                output,
            } => preview(&catalog, CreaturePartFamilyId(family), lod, &output),
            Self::Manifest { catalog, manifest } => update_manifest(&catalog, &manifest),
        }
    }
}

fn main() {
    let command =
        CreaturePartBuilderCommand::parse(env::args().skip(1).collect()).unwrap_or_else(|error| {
            eprintln!("creature_part_builder: {error}");
            std::process::exit(2);
        });
    if let Err(error) = command.run() {
        eprintln!("creature_part_builder: {error}");
        std::process::exit(1);
    }
}

fn parse_options(args: &[String]) -> Result<BTreeMap<String, String>, String> {
    let mut options = BTreeMap::new();
    let mut index = 0;
    while index < args.len() {
        let option = args[index]
            .strip_prefix("--")
            .ok_or_else(|| format!("expected option, found {}", args[index]))?;
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for --{option}"))?;
        if options.insert(option.to_string(), value.clone()).is_some() {
            return Err(format!("duplicate --{option}"));
        }
        index += 2;
    }
    Ok(options)
}

fn parse_u16_option(
    options: &BTreeMap<String, String>,
    name: &str,
    default: u16,
) -> Result<u16, String> {
    options
        .get(name)
        .map(|value| {
            value
                .parse::<u16>()
                .map_err(|_| format!("invalid --{name}"))
        })
        .transpose()
        .map(|value| value.unwrap_or(default))
}

fn parse_lod(value: &str) -> Result<CreaturePartLodId, String> {
    match value {
        "full" => Ok(CreaturePartLodId::Full),
        "compact" => Ok(CreaturePartLodId::Compact),
        "impostor" => Ok(CreaturePartLodId::Impostor),
        _ => Err(format!("invalid LOD {value}")),
    }
}

fn is_target_artifact_path(path: &Path) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/");
    normalized.starts_with("target/artifacts/creature_parts/") && !normalized.contains("../")
}

fn load_catalog(path: &Path) -> Result<CreaturePartCatalog, Box<dyn std::error::Error>> {
    Ok(CreaturePartCatalog::from_json_str(&fs::read_to_string(
        path,
    )?)?)
}

fn assets_root(catalog_path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(catalog_path
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .ok_or("catalog must be under <assets>/production_voxel_v1/creature_parts")?
        .to_path_buf())
}

fn family_lod<'a>(
    catalog: &'a CreaturePartCatalog,
    family: CreaturePartFamilyId,
    lod: CreaturePartLodId,
) -> Result<
    (
        &'a CreaturePartFamilyDefinition,
        &'a alife_game_app::CreaturePartLod,
    ),
    Box<dyn std::error::Error>,
> {
    let family = catalog.family(family).ok_or("unknown family")?;
    let lod = family
        .lods
        .iter()
        .find(|entry| entry.lod == lod)
        .ok_or("missing LOD")?;
    Ok((family, lod))
}

fn build_pack(
    catalog_path: &Path,
    catalog: &CreaturePartCatalog,
    family_id: CreaturePartFamilyId,
    lod_id: CreaturePartLodId,
) -> Result<SlicedCreaturePartPack, Box<dyn std::error::Error>> {
    let root = assets_root(catalog_path)?;
    let (family, lod) = family_lod(catalog, family_id, lod_id)?;
    let source_bytes = read_verified_source(&root, lod)?;
    let source = SourceObjMesh::parse(std::str::from_utf8(&source_bytes)?)?;
    Ok(slice_creature_mesh(&source, family, lod_id)?)
}

fn uses_canonical_authoring_builder(builder_version: &str) -> bool {
    builder_version.starts_with("scripts::generate_canonical_creature_parts.")
}

fn load_committed_canonical_pack(
    catalog_path: &Path,
    catalog: &CreaturePartCatalog,
    family_id: CreaturePartFamilyId,
    lod_id: CreaturePartLodId,
) -> Result<SlicedCreaturePartPack, Box<dyn std::error::Error>> {
    let root = assets_root(catalog_path)?;
    let (family, lod) = family_lod(catalog, family_id, lod_id)?;
    let parsed = load_generated_part_pack(&root, family, lod_id)?;
    let obj_bytes = fs::read(root.join(&lod.generated_obj))?;
    let socket_json_bytes = fs::read(root.join(&lod.socket_manifest))?;
    if obj_bytes.len() > 512 * 1024 || socket_json_bytes.len() > 512 * 1024 {
        return Err(format!(
            "canonical family {} {:?} exceeds the 512 KiB per-file budget",
            family.label, lod_id
        )
        .into());
    }

    let socket_manifest: Value = serde_json::from_slice(&socket_json_bytes)?;
    if socket_manifest["schema"] != "alife.creature_part_sockets.v1"
        || socket_manifest["schema_version"] != 1
        || socket_manifest["family_id"] != family.id.0
        || socket_manifest["lod"] != serde_json::to_value(lod_id)?
    {
        return Err(format!(
            "canonical family {} {:?} has invalid socket manifest identity",
            family.label, lod_id
        )
        .into());
    }
    let sockets: BTreeMap<String, SocketFrame> =
        serde_json::from_value(socket_manifest["sockets"].clone())?;
    if sockets != family.sockets {
        return Err(format!(
            "canonical family {} {:?} sockets drifted from the catalog",
            family.label, lod_id
        )
        .into());
    }

    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    let mut parts = BTreeMap::new();
    for slot in CreaturePartSlot::ALL {
        let data = parsed
            .parts
            .get(&slot)
            .ok_or_else(|| format!("canonical family {} is missing {slot:?}", family.label))?;
        if data.positions.is_empty()
            || data.positions.len() != data.uvs.len()
            || data.positions.len() != data.normals.len()
            || data.indices.len() < 3
            || data
                .indices
                .iter()
                .any(|index| *index as usize >= data.positions.len())
        {
            return Err(format!(
                "canonical family {} {:?} has invalid {slot:?} geometry",
                family.label, lod_id
            )
            .into());
        }
        let vertices = data
            .positions
            .iter()
            .zip(&data.uvs)
            .zip(&data.normals)
            .map(|((&position, &uv), &normal)| {
                for axis in 0..3 {
                    bounds[0][axis] = bounds[0][axis].min(f64::from(position[axis]));
                    bounds[1][axis] = bounds[1][axis].max(f64::from(position[axis]));
                }
                ObjVertex {
                    position: position.map(f64::from),
                    uv: uv.map(f64::from),
                    normal: normal.map(f64::from),
                }
            })
            .collect::<Vec<_>>();
        if !vertices.iter().all(|vertex| {
            vertex
                .position
                .into_iter()
                .chain(vertex.uv)
                .chain(vertex.normal)
                .all(f64::is_finite)
        }) {
            return Err(format!(
                "canonical family {} {:?} has non-finite {slot:?} geometry",
                family.label, lod_id
            )
            .into());
        }
        parts.insert(
            slot,
            GeneratedPartMesh {
                vertices,
                indices: data.indices.clone(),
            },
        );
    }

    Ok(SlicedCreaturePartPack {
        family_id,
        lod: lod_id,
        parts,
        source_triangle_count: 0,
        source_triangle_owners: BTreeMap::new(),
        source_triangle_fragment_slots: BTreeMap::new(),
        sockets,
        canonical_source_bounds: bounds,
        minimum_join_overlap: family
            .join_covers
            .iter()
            .map(|cover| cover.overlap_depth)
            .fold(f32::INFINITY, f32::min),
        obj_bytes,
        socket_json_bytes,
    })
}

fn read_verified_source(
    root: &Path,
    lod: &alife_game_app::CreaturePartLod,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let bytes = fs::read(root.join(&lod.source_obj))?;
    let actual = fnv1a_digest(&bytes);
    if actual != lod.source_digest {
        return Err(format!(
            "source digest mismatch for {}: expected {}, got {actual}",
            lod.source_obj, lod.source_digest
        )
        .into());
    }
    Ok(bytes)
}

#[derive(Serialize)]
struct AnalysisReceipt {
    schema: &'static str,
    family_id: u16,
    family_label: String,
    lod: CreaturePartLodId,
    source_triangle_count: usize,
    raw_bounds_min: [f64; 3],
    raw_bounds_max: [f64; 3],
}

fn analyze(
    catalog_path: &Path,
    family_id: CreaturePartFamilyId,
    lod_id: CreaturePartLodId,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let catalog = load_catalog(catalog_path)?;
    let root = assets_root(catalog_path)?;
    let (family, lod) = family_lod(&catalog, family_id, lod_id)?;
    let source_bytes = read_verified_source(&root, lod)?;
    let source = SourceObjMesh::parse(std::str::from_utf8(&source_bytes)?)?;
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    for vertex in source
        .triangles
        .iter()
        .flat_map(|triangle| triangle.vertices)
    {
        for axis in 0..3 {
            bounds[0][axis] = bounds[0][axis].min(vertex.position[axis]);
            bounds[1][axis] = bounds[1][axis].max(vertex.position[axis]);
        }
    }
    let receipt = AnalysisReceipt {
        schema: "alife.creature_part_analysis.v1",
        family_id: family.id.0,
        family_label: family.label.clone(),
        lod: lod_id,
        source_triangle_count: source.triangles.len(),
        raw_bounds_min: bounds[0],
        raw_bounds_max: bounds[1],
    };
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        output,
        format!("{}\n", serde_json::to_string_pretty(&receipt)?),
    )?;
    println!("analysis={}", output.display());
    Ok(())
}

fn build(
    catalog_path: &Path,
    requested_family: Option<CreaturePartFamilyId>,
    staging: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let catalog = load_catalog(catalog_path)?;
    let root = assets_root(catalog_path)?;
    fs::create_dir_all(staging)?;
    let families = catalog
        .families
        .iter()
        .filter(|family| requested_family.is_none_or(|id| id == family.id))
        .collect::<Vec<_>>();
    if families.is_empty() {
        return Err("requested family is not in the catalog".into());
    }
    let mut pending = Vec::new();
    for family in families {
        for lod in &family.lods {
            let pack = if uses_canonical_authoring_builder(&family.builder_version) {
                load_committed_canonical_pack(catalog_path, &catalog, family.id, lod.lod)?
            } else {
                let pack = build_pack(catalog_path, &catalog, family.id, lod.lod)?;
                validate_sliced_pack(&pack)?;
                pack
            };
            let staged_obj = staging.join(Path::new(&lod.generated_obj).file_name().unwrap());
            let staged_sockets = staging.join(Path::new(&lod.socket_manifest).file_name().unwrap());
            fs::write(&staged_obj, &pack.obj_bytes)?;
            fs::write(&staged_sockets, &pack.socket_json_bytes)?;
            pending.push((staged_obj, root.join(&lod.generated_obj)));
            pending.push((staged_sockets, root.join(&lod.socket_manifest)));
        }
    }
    for (staged, production) in pending {
        if let Some(parent) = production.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(staged, production)?;
    }
    println!(
        "generated_families={}",
        requested_family.map_or(catalog.families.len(), |_| 1)
    );
    Ok(())
}

fn validate(catalog_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let catalog = load_catalog(catalog_path)?;
    let root = assets_root(catalog_path)?;
    let mut validated = 0;
    for family in &catalog.families {
        for lod in &family.lods {
            if uses_canonical_authoring_builder(&family.builder_version) {
                load_committed_canonical_pack(catalog_path, &catalog, family.id, lod.lod)?;
            } else {
                let pack = build_pack(catalog_path, &catalog, family.id, lod.lod)?;
                validate_sliced_pack(&pack)?;
                if fs::read(root.join(&lod.generated_obj))? != pack.obj_bytes
                    || fs::read(root.join(&lod.socket_manifest))? != pack.socket_json_bytes
                {
                    return Err(format!(
                        "generated output drift for {} {:?}",
                        family.label, lod.lod
                    )
                    .into());
                }
            }
            validated += 1;
        }
    }
    println!("validated_part_packs={validated}");
    Ok(())
}

fn preview(
    catalog_path: &Path,
    family_id: CreaturePartFamilyId,
    lod_id: CreaturePartLodId,
    output: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let catalog = load_catalog(catalog_path)?;
    let family = catalog.family(family_id).ok_or("unknown family")?;
    let pack = if uses_canonical_authoring_builder(&family.builder_version) {
        load_committed_canonical_pack(catalog_path, &catalog, family_id, lod_id)?
    } else {
        build_pack(catalog_path, &catalog, family_id, lod_id)?
    };
    let image = render_preview(&pack, 768, 768);
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    image.save(output)?;
    println!("preview={}", output.display());
    Ok(())
}

fn render_preview(
    pack: &SlicedCreaturePartPack,
    width: u32,
    height: u32,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut image = ImageBuffer::from_pixel(width, height, Rgba([24, 28, 30, 255]));
    let colors = [
        (CreaturePartSlot::Head, [240, 185, 90, 255]),
        (CreaturePartSlot::Torso, [80, 170, 155, 255]),
        (CreaturePartSlot::LeftArm, [210, 100, 95, 255]),
        (CreaturePartSlot::RightArm, [225, 125, 105, 255]),
        (CreaturePartSlot::LeftLeg, [105, 135, 220, 255]),
        (CreaturePartSlot::RightLeg, [125, 155, 235, 255]),
        (CreaturePartSlot::TailBack, [180, 110, 205, 255]),
    ];
    let all_vertices = pack
        .parts
        .iter()
        .flat_map(|(slot, part)| {
            part.vertices
                .iter()
                .map(|vertex| assembled_position(pack, *slot, vertex.position))
        })
        .collect::<Vec<_>>();
    let min_x = all_vertices
        .iter()
        .map(|position| position[0])
        .fold(f64::INFINITY, f64::min);
    let max_x = all_vertices
        .iter()
        .map(|position| position[0])
        .fold(f64::NEG_INFINITY, f64::max);
    let min_z = all_vertices
        .iter()
        .map(|position| position[2])
        .fold(f64::INFINITY, f64::min);
    let max_z = all_vertices
        .iter()
        .map(|position| position[2])
        .fold(f64::NEG_INFINITY, f64::max);
    let scale = (f64::from(width) * 0.8 / (max_x - min_x).max(1.0e-6))
        .min(f64::from(height) * 0.8 / (max_z - min_z).max(1.0e-6));
    for (slot, color) in colors {
        let Some(part) = pack.parts.get(&slot) else {
            continue;
        };
        draw_part_triangles(&mut image, pack, slot, part, color, min_x, max_z, scale);
    }
    image
}

fn draw_part_triangles(
    image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    pack: &SlicedCreaturePartPack,
    slot: CreaturePartSlot,
    part: &GeneratedPartMesh,
    color: [u8; 4],
    min_x: f64,
    max_z: f64,
    scale: f64,
) {
    for triangle in part.indices.chunks_exact(3) {
        let points = [0_usize, 1, 2].map(|corner| {
            let index = triangle[corner];
            let position = assembled_position(pack, slot, part.vertices[index as usize].position);
            [
                (position[0] - min_x) * scale + f64::from(image.width()) * 0.1,
                (max_z - position[2]) * scale + f64::from(image.height()) * 0.1,
            ]
        });
        let min_px = points
            .iter()
            .map(|point| point[0])
            .fold(f64::INFINITY, f64::min)
            .floor()
            .max(0.0) as i32;
        let max_px = points
            .iter()
            .map(|point| point[0])
            .fold(f64::NEG_INFINITY, f64::max)
            .ceil()
            .min(f64::from(image.width() - 1)) as i32;
        let min_py = points
            .iter()
            .map(|point| point[1])
            .fold(f64::INFINITY, f64::min)
            .floor()
            .max(0.0) as i32;
        let max_py = points
            .iter()
            .map(|point| point[1])
            .fold(f64::NEG_INFINITY, f64::max)
            .ceil()
            .min(f64::from(image.height() - 1)) as i32;
        let area = edge(points[0], points[1], points[2]);
        if area.abs() <= f64::EPSILON {
            continue;
        }
        for py in min_py..=max_py {
            for px in min_px..=max_px {
                let point = [f64::from(px) + 0.5, f64::from(py) + 0.5];
                let w0 = edge(points[1], points[2], point) / area;
                let w1 = edge(points[2], points[0], point) / area;
                let w2 = edge(points[0], points[1], point) / area;
                if w0 >= -1.0e-6 && w1 >= -1.0e-6 && w2 >= -1.0e-6 {
                    image.put_pixel(px as u32, py as u32, Rgba(color));
                }
            }
        }
    }
}

fn assembled_position(
    pack: &SlicedCreaturePartPack,
    slot: CreaturePartSlot,
    local: [f64; 3],
) -> [f64; 3] {
    let socket_name = match slot {
        CreaturePartSlot::Head => Some("neck"),
        CreaturePartSlot::Torso => None,
        CreaturePartSlot::LeftArm => Some("left-shoulder"),
        CreaturePartSlot::RightArm => Some("right-shoulder"),
        CreaturePartSlot::LeftLeg => Some("left-hip"),
        CreaturePartSlot::RightLeg => Some("right-hip"),
        CreaturePartSlot::TailBack => Some("tail-base"),
    };
    let Some(socket_name) = socket_name else {
        return local;
    };
    let socket = &pack.sockets[socket_name];
    let scaled = [
        local[0] * f64::from(socket.scale[0]),
        local[1] * f64::from(socket.scale[1]),
        local[2] * f64::from(socket.scale[2]),
    ];
    let rotated = rotate_preview(scaled, socket.rotation_xyzw);
    [
        rotated[0] + f64::from(socket.translation[0]),
        rotated[1] + f64::from(socket.translation[1]),
        rotated[2] + f64::from(socket.translation[2]),
    ]
}

fn edge(a: [f64; 2], b: [f64; 2], point: [f64; 2]) -> f64 {
    (point[0] - a[0]) * (b[1] - a[1]) - (point[1] - a[1]) * (b[0] - a[0])
}

fn rotate_preview(vector: [f64; 3], quaternion: [f32; 4]) -> [f64; 3] {
    let q = quaternion.map(f64::from);
    let u = [q[0], q[1], q[2]];
    let uv = [
        u[1] * vector[2] - u[2] * vector[1],
        u[2] * vector[0] - u[0] * vector[2],
        u[0] * vector[1] - u[1] * vector[0],
    ];
    let uuv = [
        u[1] * uv[2] - u[2] * uv[1],
        u[2] * uv[0] - u[0] * uv[2],
        u[0] * uv[1] - u[1] * uv[0],
    ];
    [
        vector[0] + 2.0 * (q[3] * uv[0] + uuv[0]),
        vector[1] + 2.0 * (q[3] * uv[1] + uuv[1]),
        vector[2] + 2.0 * (q[3] * uv[2] + uuv[2]),
    ]
}

fn update_manifest(
    catalog_path: &Path,
    manifest_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let catalog = load_catalog(catalog_path)?;
    let root = assets_root(catalog_path)?;
    let mut manifest: Value = serde_json::from_str(&fs::read_to_string(manifest_path)?)?;
    let entries = manifest
        .get_mut("entries")
        .and_then(Value::as_array_mut)
        .ok_or("production manifest is missing entries")?;
    entries.retain(|entry| {
        let asset_id = entry
            .get("asset_id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        !(asset_id.starts_with("creature-part-")
            || asset_id.starts_with("creature-surface-family-")
            || asset_id.starts_with("quirky-texture-t-")
            || (asset_id.starts_with("quirky-model-") && asset_id.contains("lod")))
    });
    for family in &catalog.families {
        for lod in &family.lods {
            let seed = if uses_canonical_authoring_builder(&family.builder_version) {
                format!(
                    "family:{:04};lod:{}",
                    family.id.0,
                    format!("{:?}", lod.lod).to_ascii_lowercase()
                )
            } else {
                fnv1a_digest(&read_verified_source(&root, lod)?)
            };
            for (kind, relative) in [
                ("parts", lod.generated_obj.as_str()),
                ("sockets", lod.socket_manifest.as_str()),
            ] {
                let path = root.join(relative);
                let bytes = fs::read(&path)?;
                entries.push(json!({
                    "asset_id": creature_part_asset_id(family.id, lod.lod, kind),
                    "usage_category": "creatures",
                    "local_path": format!("crates/alife_game_app/assets/{relative}"),
                    "digest": PortableAssetDigest::for_file(&path)?.0,
                    "size_bytes": bytes.len(),
                    "source": "scripts/generate_canonical_creature_parts.py",
                    "license": "MIT",
                    "license_ref": "LICENSE",
                    "author": "A-Life contributors",
                    "generated": true,
                    "generator": {
                        "tool": family.builder_version.as_str(),
                        "output_schema": family.output_schema.as_str(),
                        "config_path": "crates/alife_game_app/assets/production_voxel_v1/creature_parts/catalog.json",
                        "seed": seed,
                        "date": "2026-07-16"
                    },
                    "external": false,
                    "replacement_policy": "regenerate-from-deterministic-canonical-generator",
                    "final_art": true,
                    "placeholder": false
                }));
            }
        }

        let texture_path = root.join(&family.texture_asset);
        let texture_bytes = fs::read(&texture_path)?;
        entries.push(json!({
            "asset_id": creature_surface_asset_id(family.id),
            "usage_category": "creatures",
            "local_path": format!("crates/alife_game_app/assets/{}", family.texture_asset),
            "digest": PortableAssetDigest::for_file(&texture_path)?.0,
            "size_bytes": texture_bytes.len(),
            "source": "scripts/generate_canonical_creature_parts.py",
            "license": "MIT",
            "license_ref": "LICENSE",
            "author": "A-Life contributors",
            "generated": true,
            "generator": {
                "tool": family.builder_version.as_str(),
                "config_path": "crates/alife_game_app/assets/production_voxel_v1/creature_parts/catalog.json",
                "seed": format!("family:{:04};surface", family.id.0),
                "date": "2026-07-16"
            },
            "external": false,
            "replacement_policy": "regenerate-from-deterministic-canonical-generator",
            "final_art": true,
            "placeholder": false
        }));
    }
    fs::write(
        manifest_path,
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )?;
    println!("manifest={}", manifest_path.display());
    Ok(())
}

fn creature_part_asset_id(
    family: CreaturePartFamilyId,
    lod: CreaturePartLodId,
    kind: &str,
) -> String {
    format!("creature-part-family-{:04}-{:?}-{kind}", family.0, lod).to_ascii_lowercase()
}

fn creature_surface_asset_id(family: CreaturePartFamilyId) -> String {
    format!("creature-surface-family-{:04}", family.0)
}

fn fnv1a_digest(bytes: &[u8]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    format!("fnv1a64:{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace_path(relative: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("alife_tools must be under <workspace>/crates")
            .join(relative)
    }

    #[test]
    fn cli_supports_all_required_commands() {
        for command in ["analyze", "build", "validate", "preview", "manifest"] {
            assert!(CreaturePartBuilderCommand::parse_for_test([command]).is_ok());
        }
    }

    #[test]
    fn preview_rejects_workspace_source_output() {
        let parsed = CreaturePartBuilderCommand::parse_for_test([
            "preview",
            "--output",
            "crates/alife_game_app/assets/preview.png",
        ]);
        assert!(parsed.is_err());
    }

    #[test]
    fn manifest_asset_ids_use_append_only_family_ids() {
        assert_eq!(
            creature_part_asset_id(CreaturePartFamilyId(7), CreaturePartLodId::Compact, "parts"),
            "creature-part-family-0007-compact-parts"
        );
        assert_eq!(
            creature_surface_asset_id(CreaturePartFamilyId(7)),
            "creature-surface-family-0007"
        );
    }

    #[test]
    fn canonical_launch_authoring_never_routes_back_through_source_animal_slicing() {
        assert!(uses_canonical_authoring_builder(
            "scripts::generate_canonical_creature_parts.v3"
        ));
        assert!(!uses_canonical_authoring_builder(
            "alife_tools::creature_part_builder.v1"
        ));
    }

    #[test]
    fn canonical_production_catalog_validates_committed_packs() {
        validate(&workspace_path(DEFAULT_CATALOG)).unwrap();
    }

    #[test]
    fn refreshed_manifest_records_canonical_parts_and_surfaces_as_mit_outputs() {
        let artifact =
            workspace_path("target/artifacts/creature_parts/canonical_manifest_validation.json");
        if let Some(parent) = artifact.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::copy(workspace_path(DEFAULT_MANIFEST), &artifact).unwrap();
        update_manifest(&workspace_path(DEFAULT_CATALOG), &artifact).unwrap();

        let manifest: Value =
            serde_json::from_str(&fs::read_to_string(&artifact).unwrap()).unwrap();
        let entries = manifest["entries"].as_array().unwrap();
        let canonical = entries
            .iter()
            .filter(|entry| {
                entry["asset_id"]
                    .as_str()
                    .is_some_and(|id| id.starts_with("creature-part-family-"))
            })
            .collect::<Vec<_>>();
        let surfaces = entries
            .iter()
            .filter(|entry| {
                entry["asset_id"]
                    .as_str()
                    .is_some_and(|id| id.starts_with("creature-surface-family-"))
            })
            .collect::<Vec<_>>();
        assert_eq!(canonical.len(), 48);
        assert_eq!(surfaces.len(), 8);
        assert!(canonical.iter().chain(&surfaces).all(|entry| {
            entry["license"] == "MIT"
                && entry["author"] == "A-Life contributors"
                && entry["source"] == "scripts/generate_canonical_creature_parts.py"
                && entry["generated"] == true
                && entry["external"] == false
        }));
        assert!(entries.iter().all(|entry| {
            !entry["asset_id"]
                .as_str()
                .is_some_and(|id| id.starts_with("quirky-texture-t-"))
        }));
    }
}
