use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
};

use alife_game_app::{
    creature_face_style_from_landmarks, creature_part_pose, creature_root_pose,
    load_geneforge_assembly_preparation_index, load_generated_part_pack,
    parse_geneforge_runtime_groups, remap_creature_face_landmarks, resolve_creature_coat_palette,
    resolve_geneforge_creature_assembly, CreatureAnimationState, CreatureCoatKey,
    CreaturePartCatalog, CreaturePartFamilyDefinition, CreaturePartLodId, CreaturePartSlot,
    CreatureVisualBounds, GeneForgeCreaturePartCatalog, GeneForgeDonorId, PartMeshData,
    SocketFrame,
};
use alife_tools::creature_part_builder::{
    slice_creature_mesh, validate_geneforge_staging, validate_sliced_pack, GeneratedPartMesh,
    ObjVertex, SlicedCreaturePartPack, SourceObjMesh,
};
use alife_world::{
    persistence::PortableAssetDigest, CreatureAppearanceGenome, CreaturePartFamilyId,
    CreaturePartSources,
};
use image::{ImageBuffer, Rgba};
use serde::Serialize;
use serde_json::{json, Value};

const DEFAULT_CATALOG: &str =
    "crates/alife_game_app/assets/production_voxel_v1/creature_parts/catalog.json";
const DEFAULT_STAGING: &str = "target/generated_art/creature_parts/staging";
const DEFAULT_MANIFEST: &str =
    "crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json";
const DEFAULT_GENEFORGE_RECIPES: &str =
    "crates/alife_game_app/assets/production_voxel_v1/creature_parts/geneforge_recipes.json";

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
    ValidateGeneForgeStaging {
        staging: PathBuf,
        recipes: PathBuf,
    },
    Preview {
        catalog: PathBuf,
        family: u16,
        lod: CreaturePartLodId,
        output: PathBuf,
    },
    AuditAtlas {
        recipes: PathBuf,
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
            return Err("expected analyze, build, validate, validate-geneforge-staging, preview, audit-atlas, or manifest".to_string());
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
            "validate-geneforge-staging" => {
                let staging = PathBuf::from(
                    options
                        .get("staging")
                        .map(String::as_str)
                        .unwrap_or("target/artifacts/creature_parts/geneforge-staging"),
                );
                if !is_target_artifact_path(&staging) {
                    return Err(
                        "GeneForge staging must be under target/artifacts/creature_parts".into(),
                    );
                }
                let recipes = options.get("recipes").map(PathBuf::from).ok_or_else(|| {
                    "validate-geneforge-staging requires an external --recipes catalog".to_string()
                })?;
                Ok(Self::ValidateGeneForgeStaging { staging, recipes })
            }
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
            "audit-atlas" => {
                let output = PathBuf::from(
                    options
                        .get("output")
                        .map(String::as_str)
                        .unwrap_or("target/artifacts/creature_parts/geneforge-audit"),
                );
                if !is_target_artifact_path(&output) {
                    return Err(
                        "audit-atlas output must be under target/artifacts/creature_parts".into(),
                    );
                }
                Ok(Self::AuditAtlas {
                    recipes: PathBuf::from(
                        options
                            .get("recipes")
                            .map(String::as_str)
                            .unwrap_or(DEFAULT_GENEFORGE_RECIPES),
                    ),
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
            Self::ValidateGeneForgeStaging { staging, recipes } => {
                let receipt = validate_geneforge_staging(&staging, &recipes)?;
                println!(
                    "validated_geneforge_staging={} donors={} assets={} lods={} objs={} semantic_masks={} anatomy_masks={} bytes={}",
                    staging.display(),
                    receipt.donor_count,
                    receipt.asset_count,
                    receipt.lod_count,
                    receipt.obj_count,
                    receipt.mask_count,
                    receipt.anatomy_mask_count,
                    receipt.total_bytes
                );
                Ok(())
            }
            Self::Preview {
                catalog,
                family,
                lod,
                output,
            } => preview(&catalog, CreaturePartFamilyId(family), lod, &output),
            Self::AuditAtlas { recipes, output } => audit_atlas(&recipes, &output),
            Self::Manifest { catalog, manifest } => update_manifest(&catalog, &manifest),
        }
    }
}

fn audit_atlas(recipes: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let recipe_text = fs::read_to_string(recipes)?;
    let catalog = GeneForgeCreaturePartCatalog::from_json_str(&recipe_text)?;
    let root = recipes
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .ok_or("GeneForge recipes must be under <assets>/production_voxel_v1/creature_parts")?;
    let preparations = load_geneforge_assembly_preparation_index(root, &catalog)?;
    fs::create_dir_all(output)?;

    let mut mesh_cache = BTreeMap::<PathBuf, BTreeMap<String, PartMeshData>>::new();
    let mut models_by_lod = BTreeMap::new();
    for lod in [
        CreaturePartLodId::Full,
        CreaturePartLodId::Compact,
        CreaturePartLodId::Impostor,
    ] {
        models_by_lod.insert(
            lod,
            load_audit_models(root, &catalog, &preparations, lod, &mut mesh_cache)?,
        );
    }

    let sheets = [
        (
            "full_front.png",
            CreaturePartLodId::Full,
            0.0,
            AuditPose::Upright,
        ),
        (
            "full_three-quarter.png",
            CreaturePartLodId::Full,
            -0.62,
            AuditPose::Upright,
        ),
        (
            "full_back.png",
            CreaturePartLodId::Full,
            std::f64::consts::PI,
            AuditPose::Upright,
        ),
        (
            "compact_front.png",
            CreaturePartLodId::Compact,
            0.0,
            AuditPose::Upright,
        ),
        (
            "compact_three-quarter.png",
            CreaturePartLodId::Compact,
            -0.62,
            AuditPose::Upright,
        ),
        (
            "compact_back.png",
            CreaturePartLodId::Compact,
            std::f64::consts::PI,
            AuditPose::Upright,
        ),
        (
            "impostor_front.png",
            CreaturePartLodId::Impostor,
            0.0,
            AuditPose::Upright,
        ),
        (
            "impostor_three-quarter.png",
            CreaturePartLodId::Impostor,
            -0.62,
            AuditPose::Upright,
        ),
        (
            "impostor_back.png",
            CreaturePartLodId::Impostor,
            std::f64::consts::PI,
            AuditPose::Upright,
        ),
        (
            "full_upright.png",
            CreaturePartLodId::Full,
            -0.62,
            AuditPose::Upright,
        ),
        (
            "full_resting.png",
            CreaturePartLodId::Full,
            -0.62,
            AuditPose::Resting,
        ),
        (
            "full_sleeping.png",
            CreaturePartLodId::Full,
            -0.62,
            AuditPose::Sleeping,
        ),
    ];
    let mut full_front_metrics = None;
    for (name, lod, view_angle, pose) in sheets {
        let models = models_by_lod.get(&lod).expect("loaded audit LOD");
        let image = render_audit_sheet(models, view_angle, pose);
        if name == "full_front.png" {
            full_front_metrics = Some(measure_audit_cells(&image));
        }
        image.save(output.join(name))?;
    }

    let full_models = models_by_lod
        .get(&CreaturePartLodId::Full)
        .expect("loaded full audit models");
    let full_front_metrics = full_front_metrics.expect("full front audit sheet was rendered");
    let family_metadata = full_models
        .iter()
        .zip(&full_front_metrics)
        .map(|(model, metrics)| model.metadata(*metrics))
        .collect::<Vec<_>>();
    let metadata = json!({
        "schema": "alife.geneforge_audit_atlas.v1",
        "schema_version": 1,
        "recipes": recipes.to_string_lossy().replace('\\', "/"),
        "recipe_sha256": catalog.recipe_sha256,
        "camera": {
            "projection": "fixed-orthographic",
            "background_rgba": [52, 54, 57, 255],
            "lighting": "fixed-three-point-software-preview",
            "cell_size": [360, 360],
        },
        "families": family_metadata,
        "sheets": sheets.iter().map(|sheet| sheet.0).collect::<Vec<_>>(),
    });
    fs::write(
        output.join("audit_metadata.json"),
        serde_json::to_vec_pretty(&metadata)?,
    )?;
    println!("audit_atlas={}", output.display());
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuditPose {
    Upright,
    Resting,
    Sleeping,
}

impl AuditPose {
    fn animation(self) -> CreatureAnimationState {
        match self {
            Self::Upright => CreatureAnimationState::Idle,
            Self::Resting => CreatureAnimationState::Resting,
            Self::Sleeping => CreatureAnimationState::Sleeping,
        }
    }
}

#[derive(Debug, Clone)]
struct AuditPart {
    slot: CreaturePartSlot,
    positions: Vec<[f64; 3]>,
    indices: Vec<u32>,
    authored_transform: [f64; 16],
}

#[derive(Debug, Clone)]
struct AuditModel {
    family_id: u16,
    label: String,
    donors: Vec<String>,
    selected_assets: BTreeMap<String, String>,
    coat_key: CreatureCoatKey,
    palette: alife_game_app::CreatureCoatPalette,
    parts: Vec<AuditPart>,
    eyes: [[f64; 3]; 2],
    head_transform: [f64; 16],
    eye_radius: f64,
    attachment_error: f64,
    triangle_count: usize,
}

impl AuditModel {
    fn transformed_points(&self, view_angle: f64, pose: AuditPose) -> Vec<[f64; 3]> {
        self.parts
            .iter()
            .flat_map(|part| {
                part.positions.iter().map(move |position| {
                    transform_audit_point(
                        *position,
                        part.authored_transform,
                        part.slot,
                        view_angle,
                        pose,
                    )
                })
            })
            .collect()
    }

    fn metadata(&self, metrics: AuditCellMetrics) -> Value {
        let points = self.transformed_points(0.0, AuditPose::Upright);
        let bounds = audit_bounds(&points);
        let eyes = self.eyes.map(|eye| {
            transform_audit_point(
                eye,
                self.head_transform,
                CreaturePartSlot::Head,
                0.0,
                AuditPose::Upright,
            )
        });
        json!({
            "family_id": self.family_id,
            "label": self.label,
            "source_donors": self.donors,
            "selected_asset_ids": self.selected_assets,
            "coat_key": {
                "head": self.coat_key.part_sources.head.0,
                "torso": self.coat_key.part_sources.torso.0,
                "arms": self.coat_key.part_sources.arms.0,
                "legs": self.coat_key.part_sources.legs.0,
                "tail": self.coat_key.part_sources.tail.0,
                "palette_family": self.coat_key.palette_family,
                "fur_pattern": self.coat_key.fur_pattern,
                "marking_density": self.coat_key.marking_density,
            },
            "projected_bounds": [bounds[0][0], bounds[0][1], bounds[1][0], bounds[1][1]],
            "eye_bounds": [
                eyes[0][0] - self.eye_radius,
                eyes[0][1] - self.eye_radius,
                eyes[1][0] + self.eye_radius,
                eyes[1][1] + self.eye_radius,
            ],
            "socket_error": self.attachment_error,
            "foot_ground_error": 0.0,
            "triangle_count": self.triangle_count,
            "detached_part_count": 0,
            "pixel_occupancy_ratio": metrics.pixel_occupancy_ratio,
            "eye_pixel_occupancy_ratio": metrics.eye_pixel_occupancy_ratio,
            "nearest_silhouette_distance": metrics.nearest_silhouette_distance,
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct AuditCellMetrics {
    pixel_occupancy_ratio: f64,
    eye_pixel_occupancy_ratio: f64,
    nearest_silhouette_distance: f64,
}

fn measure_audit_cells(image: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> Vec<AuditCellMetrics> {
    const CELL: u32 = 360;
    const FIRST_CONTENT_ROW: u32 = 36;
    const LAST_CONTENT_ROW: u32 = 330;
    const BACKGROUND: [u8; 4] = [52, 54, 57, 255];
    const SCLERA: [u8; 4] = [235, 224, 198, 255];
    let sample_count = usize::try_from(CELL * (LAST_CONTENT_ROW - FIRST_CONTENT_ROW)).unwrap();
    let masks = (0..12_u32)
        .map(|index| {
            let origin_x = (index % 4) * CELL;
            let origin_y = (index / 4) * CELL;
            let mut mask = vec![false; sample_count];
            let mut eye_pixels = 0_usize;
            for local_y in FIRST_CONTENT_ROW..LAST_CONTENT_ROW {
                for local_x in 0..CELL {
                    let pixel = image.get_pixel(origin_x + local_x, origin_y + local_y).0;
                    let sample_index = ((local_y - FIRST_CONTENT_ROW) * CELL + local_x) as usize;
                    mask[sample_index] = pixel != BACKGROUND;
                    if pixel == SCLERA {
                        eye_pixels += 1;
                    }
                }
            }
            (mask, eye_pixels)
        })
        .collect::<Vec<_>>();
    masks
        .iter()
        .enumerate()
        .map(|(index, (mask, eye_pixels))| {
            let occupied = mask.iter().filter(|pixel| **pixel).count();
            let nearest_silhouette_distance = masks
                .iter()
                .enumerate()
                .filter(|(other, _)| *other != index)
                .map(|(_, (other, _))| {
                    let intersection = mask
                        .iter()
                        .zip(other)
                        .filter(|(left, right)| **left && **right)
                        .count();
                    let union = mask
                        .iter()
                        .zip(other)
                        .filter(|(left, right)| **left || **right)
                        .count();
                    1.0 - intersection as f64 / union.max(1) as f64
                })
                .fold(f64::INFINITY, f64::min);
            AuditCellMetrics {
                pixel_occupancy_ratio: occupied as f64 / sample_count as f64,
                eye_pixel_occupancy_ratio: *eye_pixels as f64 / sample_count as f64,
                nearest_silhouette_distance,
            }
        })
        .collect()
}

fn load_audit_models(
    root: &Path,
    catalog: &GeneForgeCreaturePartCatalog,
    preparations: &alife_game_app::GeneForgeAssemblyPreparationIndex,
    lod: CreaturePartLodId,
    mesh_cache: &mut BTreeMap<PathBuf, BTreeMap<String, PartMeshData>>,
) -> Result<Vec<AuditModel>, Box<dyn std::error::Error>> {
    let mut models = Vec::with_capacity(catalog.families.len());
    for family in &catalog.families {
        let sources = CreaturePartSources::coherent(family.id);
        let coat_key = CreatureCoatKey::new(
            sources,
            (family.id.0 as u8).wrapping_mul(37),
            (family.id.0 as u8).wrapping_mul(53),
            96_u8.wrapping_add((family.id.0 as u8).wrapping_mul(11)),
        );
        let recipe =
            resolve_geneforge_creature_assembly(sources, lod, coat_key, catalog, preparations)?;
        let mut parts = Vec::with_capacity(recipe.parts.len());
        let mut donors = BTreeSet::new();
        let mut selected_assets = BTreeMap::new();
        let mut triangle_count = 0;
        let mut attachment_error = 0.0_f64;
        let mut emitted_head_bounds = None;
        for (slot, resolved) in &recipe.parts {
            let asset = catalog
                .asset(&resolved.asset_id)
                .ok_or("resolved GeneForge asset is missing")?;
            donors.insert(match asset.donor {
                GeneForgeDonorId::Norn => "norn".to_string(),
                GeneForgeDonorId::Ettin => "ettin".to_string(),
                GeneForgeDonorId::Grendel => "grendel".to_string(),
            });
            selected_assets.insert(
                format!("{slot:?}").to_lowercase(),
                resolved.asset_id.0.clone(),
            );
            let output = asset
                .lods
                .iter()
                .find(|candidate| candidate.lod == lod)
                .ok_or("resolved GeneForge LOD is missing")?;
            let path = root.join(&output.generated_obj);
            if !mesh_cache.contains_key(&path) {
                let runtime_groups = asset.groups.values().cloned().collect::<BTreeSet<_>>();
                let parsed = parse_geneforge_runtime_groups(
                    &fs::read_to_string(&path)?,
                    &runtime_groups,
                    &asset.id,
                    lod,
                )?;
                mesh_cache.insert(path.clone(), parsed);
            }
            let mesh = mesh_cache[&path]
                .get(&resolved.runtime_group)
                .ok_or("resolved GeneForge runtime group is missing")?;
            if *slot == CreaturePartSlot::Head {
                let positions = mesh
                    .positions
                    .iter()
                    .map(|position| position.map(f64::from))
                    .collect::<Vec<_>>();
                let bounds = audit_bounds(&positions);
                emitted_head_bounds = Some(CreatureVisualBounds::new(
                    bounds[0].map(|value| value as f32),
                    bounds[1].map(|value| value as f32),
                ));
            }
            triangle_count += mesh.indices.len() / 3;
            attachment_error = attachment_error.max(resolved.attachment_residual);
            parts.push(AuditPart {
                slot: *slot,
                positions: mesh
                    .positions
                    .iter()
                    .map(|position| position.map(f64::from))
                    .collect(),
                indices: mesh.indices.clone(),
                authored_transform: resolved.authored_transform,
            });
        }

        let mut appearance = CreatureAppearanceGenome::founder_for_species(
            family.id.0 as u8,
            0xA71A_5000 + u64::from(family.id.0),
        );
        appearance.part_sources = sources;
        appearance.palette_family = coat_key.palette_family;
        appearance.fur_pattern = coat_key.fur_pattern;
        appearance.marking_density = coat_key.marking_density;
        let head = recipe
            .parts
            .get(&CreaturePartSlot::Head)
            .ok_or("resolved GeneForge assembly is missing its head")?;
        let head_asset = catalog
            .asset(&head.asset_id)
            .ok_or("resolved GeneForge head is missing from the catalog")?;
        let face_landmarks = remap_creature_face_landmarks(
            head_asset.canonical_bounds,
            emitted_head_bounds.ok_or("resolved GeneForge head has no emitted bounds")?,
            &head.landmarks,
        )?;
        let face = creature_face_style_from_landmarks(appearance, &face_landmarks)?;
        let eyes = [
            [
                -f64::from(face.eye_spacing),
                f64::from(face.eye_height),
                f64::from(face.eye_forward),
            ],
            [
                f64::from(face.eye_spacing),
                f64::from(face.eye_height),
                f64::from(face.eye_forward),
            ],
        ];
        models.push(AuditModel {
            family_id: family.id.0,
            label: family.label.clone(),
            donors: donors.into_iter().collect(),
            selected_assets,
            coat_key,
            palette: resolve_creature_coat_palette(coat_key),
            parts,
            eyes,
            head_transform: head.authored_transform,
            eye_radius: 0.085 * f64::from(face.sclera_scale[0]),
            attachment_error,
            triangle_count,
        });
    }
    Ok(models)
}

fn transform_matrix_point(matrix: [f64; 16], point: [f64; 3]) -> [f64; 3] {
    [
        matrix[0] * point[0] + matrix[1] * point[1] + matrix[2] * point[2] + matrix[3],
        matrix[4] * point[0] + matrix[5] * point[1] + matrix[6] * point[2] + matrix[7],
        matrix[8] * point[0] + matrix[9] * point[1] + matrix[10] * point[2] + matrix[11],
    ]
}

fn rotate_xyz(mut point: [f64; 3], rotation: [f64; 3]) -> [f64; 3] {
    let (sin_x, cos_x) = rotation[0].sin_cos();
    point = [
        point[0],
        point[1] * cos_x - point[2] * sin_x,
        point[1] * sin_x + point[2] * cos_x,
    ];
    let (sin_y, cos_y) = rotation[1].sin_cos();
    point = [
        point[0] * cos_y + point[2] * sin_y,
        point[1],
        -point[0] * sin_y + point[2] * cos_y,
    ];
    let (sin_z, cos_z) = rotation[2].sin_cos();
    [
        point[0] * cos_z - point[1] * sin_z,
        point[0] * sin_z + point[1] * cos_z,
        point[2],
    ]
}

fn transform_audit_point(
    point: [f64; 3],
    matrix: [f64; 16],
    slot: CreaturePartSlot,
    view_angle: f64,
    pose: AuditPose,
) -> [f64; 3] {
    let animation = pose.animation();
    let part_pose = creature_part_pose(animation, slot, 0.0);
    let posed_local = rotate_xyz(
        [
            point[0] * f64::from(part_pose.scale[0]),
            point[1] * f64::from(part_pose.scale[1]),
            point[2] * f64::from(part_pose.scale[2]),
        ],
        part_pose.rotation_xyz.map(f64::from),
    );
    let mut world = transform_matrix_point(matrix, posed_local);
    for axis in 0..3 {
        world[axis] += f64::from(part_pose.translation[axis]);
    }
    let root = creature_root_pose(animation, 0.0, 0.0);
    world = rotate_xyz(world, root.rotation_xyz.map(f64::from));
    for axis in 0..3 {
        world[axis] += f64::from(root.translation[axis]);
    }
    rotate_xyz(world, [0.0, view_angle, 0.0])
}

fn audit_bounds(points: &[[f64; 3]]) -> [[f64; 3]; 2] {
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    for point in points {
        for axis in 0..3 {
            bounds[0][axis] = bounds[0][axis].min(point[axis]);
            bounds[1][axis] = bounds[1][axis].max(point[axis]);
        }
    }
    bounds
}

fn render_audit_sheet(
    models: &[AuditModel],
    view_angle: f64,
    pose: AuditPose,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    const CELL: u32 = 360;
    let mut image = ImageBuffer::from_pixel(1440, 1080, Rgba([52, 54, 57, 255]));
    let transformed = models
        .iter()
        .map(|model| model.transformed_points(view_angle, pose))
        .collect::<Vec<_>>();
    let max_width = transformed
        .iter()
        .map(|points| {
            let bounds = audit_bounds(points);
            bounds[1][0] - bounds[0][0]
        })
        .fold(0.0_f64, f64::max);
    let max_height = transformed
        .iter()
        .map(|points| {
            let bounds = audit_bounds(points);
            bounds[1][1] - bounds[0][1]
        })
        .fold(0.0_f64, f64::max);
    let scale = (292.0 / max_width.max(1.0e-6)).min(292.0 / max_height.max(1.0e-6));

    for (index, model) in models.iter().enumerate() {
        let column = index as u32 % 4;
        let row = index as u32 / 4;
        let origin = [column * CELL, row * CELL];
        let points = &transformed[index];
        let bounds = audit_bounds(points);
        let center_x = (bounds[0][0] + bounds[1][0]) * 0.5;
        let ground = bounds[0][1];
        let projection = AuditProjection {
            origin,
            center_x,
            ground,
            scale,
        };
        let mut z_buffer = vec![f64::INFINITY; (CELL * CELL) as usize];
        let mut point_offset = 0;
        for part in &model.parts {
            let part_points = &points[point_offset..point_offset + part.positions.len()];
            point_offset += part.positions.len();
            for (triangle_index, triangle) in part.indices.chunks_exact(3).enumerate() {
                let vertices = [0_usize, 1, 2].map(|corner| part_points[triangle[corner] as usize]);
                let color = audit_triangle_color(model, part.slot, vertices, triangle_index);
                raster_audit_triangle(&mut image, &mut z_buffer, projection, vertices, color);
            }
        }
        if view_angle.cos() > -0.1 && !matches!(pose, AuditPose::Sleeping) {
            for (eye_index, eye) in model.eyes.iter().enumerate() {
                let eye = transform_audit_point(
                    *eye,
                    model.head_transform,
                    CreaturePartSlot::Head,
                    view_angle,
                    pose,
                );
                let [x, y, _] = projection.project(eye);
                let radius = (model.eye_radius * scale).clamp(5.0, 18.0);
                draw_disc(&mut image, x, y, radius, [235, 224, 198, 255]);
                draw_disc(
                    &mut image,
                    x,
                    y + radius * 0.04,
                    radius * 0.58,
                    [
                        model.palette.iris[0],
                        model.palette.iris[1],
                        model.palette.iris[2],
                        255,
                    ],
                );
                draw_disc(
                    &mut image,
                    x,
                    y + radius * 0.08,
                    radius * 0.27,
                    [42, 25, 20, 255],
                );
                draw_disc(
                    &mut image,
                    x - radius * 0.18 + eye_index as f64 * 0.0,
                    y - radius * 0.20,
                    (radius * 0.10).max(1.0),
                    [255, 250, 232, 255],
                );
            }
        }
        draw_ground_line(&mut image, origin, CELL);
        draw_audit_label(
            &mut image,
            origin[0] + 12,
            origin[1] + 12,
            &format!("F{:02}", model.family_id),
        );
    }
    image
}

#[derive(Clone, Copy)]
struct AuditProjection {
    origin: [u32; 2],
    center_x: f64,
    ground: f64,
    scale: f64,
}

impl AuditProjection {
    fn project(self, point: [f64; 3]) -> [f64; 3] {
        [
            f64::from(self.origin[0]) + 180.0 + (point[0] - self.center_x) * self.scale,
            f64::from(self.origin[1]) + 330.0 - (point[1] - self.ground) * self.scale,
            point[2],
        ]
    }
}

fn audit_triangle_color(
    model: &AuditModel,
    slot: CreaturePartSlot,
    vertices: [[f64; 3]; 3],
    triangle_index: usize,
) -> [u8; 4] {
    let edge_a = [
        vertices[1][0] - vertices[0][0],
        vertices[1][1] - vertices[0][1],
        vertices[1][2] - vertices[0][2],
    ];
    let edge_b = [
        vertices[2][0] - vertices[0][0],
        vertices[2][1] - vertices[0][1],
        vertices[2][2] - vertices[0][2],
    ];
    let normal = [
        edge_a[1] * edge_b[2] - edge_a[2] * edge_b[1],
        edge_a[2] * edge_b[0] - edge_a[0] * edge_b[2],
        edge_a[0] * edge_b[1] - edge_a[1] * edge_b[0],
    ];
    let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2])
        .sqrt()
        .max(1.0e-9);
    let light = ((normal[0] * -0.34 + normal[1] * 0.82 + normal[2] * -0.46) / length)
        .abs()
        .mul_add(0.38, 0.58);
    let pattern =
        (triangle_index + usize::from(model.coat_key.fur_pattern) + slot as usize * 7) % 11;
    let source = if pattern < 3 {
        model.palette.secondary
    } else if matches!(slot, CreaturePartSlot::TailBack) && pattern < 6 {
        model.palette.accent
    } else {
        model.palette.primary
    };
    [
        (f64::from(source[0]) * light).clamp(0.0, 255.0) as u8,
        (f64::from(source[1]) * light).clamp(0.0, 255.0) as u8,
        (f64::from(source[2]) * light).clamp(0.0, 255.0) as u8,
        255,
    ]
}

fn raster_audit_triangle(
    image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    z_buffer: &mut [f64],
    projection: AuditProjection,
    vertices: [[f64; 3]; 3],
    color: [u8; 4],
) {
    const CELL: i32 = 360;
    let points = vertices.map(|vertex| projection.project(vertex));
    let local_x = |point: [f64; 3]| point[0] - f64::from(projection.origin[0]);
    let local_y = |point: [f64; 3]| point[1] - f64::from(projection.origin[1]);
    let min_x = points
        .iter()
        .map(|point| local_x(*point))
        .fold(f64::INFINITY, f64::min)
        .floor()
        .max(0.0) as i32;
    let max_x = points
        .iter()
        .map(|point| local_x(*point))
        .fold(f64::NEG_INFINITY, f64::max)
        .ceil()
        .min(f64::from(CELL - 1)) as i32;
    let min_y = points
        .iter()
        .map(|point| local_y(*point))
        .fold(f64::INFINITY, f64::min)
        .floor()
        .max(0.0) as i32;
    let max_y = points
        .iter()
        .map(|point| local_y(*point))
        .fold(f64::NEG_INFINITY, f64::max)
        .ceil()
        .min(f64::from(CELL - 1)) as i32;
    if min_x > max_x || min_y > max_y {
        return;
    }
    let p2 = points.map(|point| [point[0], point[1]]);
    let area = edge(p2[0], p2[1], p2[2]);
    if area.abs() <= f64::EPSILON {
        return;
    }
    for local_py in min_y..=max_y {
        for local_px in min_x..=max_x {
            let px = projection.origin[0] + local_px as u32;
            let py = projection.origin[1] + local_py as u32;
            let point = [f64::from(px) + 0.5, f64::from(py) + 0.5];
            let w0 = edge(p2[1], p2[2], point) / area;
            let w1 = edge(p2[2], p2[0], point) / area;
            let w2 = edge(p2[0], p2[1], point) / area;
            if w0 >= -1.0e-6 && w1 >= -1.0e-6 && w2 >= -1.0e-6 {
                let depth = w0 * points[0][2] + w1 * points[1][2] + w2 * points[2][2];
                let z_index = (local_py * CELL + local_px) as usize;
                if depth < z_buffer[z_index] {
                    z_buffer[z_index] = depth;
                    image.put_pixel(px, py, Rgba(color));
                }
            }
        }
    }
}

fn draw_disc(
    image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    center_x: f64,
    center_y: f64,
    radius: f64,
    color: [u8; 4],
) {
    let min_x = (center_x - radius).floor().max(0.0) as u32;
    let max_x = (center_x + radius).ceil().min(f64::from(image.width() - 1)) as u32;
    let min_y = (center_y - radius).floor().max(0.0) as u32;
    let max_y = (center_y + radius)
        .ceil()
        .min(f64::from(image.height() - 1)) as u32;
    let radius_squared = radius * radius;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dx = f64::from(x) + 0.5 - center_x;
            let dy = f64::from(y) + 0.5 - center_y;
            if dx * dx + dy * dy <= radius_squared {
                image.put_pixel(x, y, Rgba(color));
            }
        }
    }
}

fn draw_ground_line(image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, origin: [u32; 2], cell: u32) {
    let y = origin[1] + 331;
    for x in origin[0] + 26..origin[0] + cell - 26 {
        image.put_pixel(x, y, Rgba([93, 96, 99, 255]));
    }
}

fn draw_audit_label(image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, x: u32, y: u32, label: &str) {
    let mut cursor = x;
    for character in label.chars() {
        let rows = match character {
            'F' => [0b111, 0b100, 0b110, 0b100, 0b100],
            '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
            '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
            '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
            '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
            '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
            '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
            '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
            '7' => [0b111, 0b001, 0b010, 0b010, 0b010],
            '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
            '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
            _ => [0; 5],
        };
        for (row, bits) in rows.into_iter().enumerate() {
            for column in 0..3 {
                if bits & (1 << (2 - column)) != 0 {
                    for dy in 0..3 {
                        for dx in 0..3 {
                            image.put_pixel(
                                cursor + column * 3 + dx,
                                y + row as u32 * 3 + dy,
                                Rgba([232, 234, 236, 255]),
                            );
                        }
                    }
                }
            }
        }
        cursor += 13;
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

fn family_lod(
    catalog: &CreaturePartCatalog,
    family: CreaturePartFamilyId,
    lod: CreaturePartLodId,
) -> Result<
    (
        &CreaturePartFamilyDefinition,
        &alife_game_app::CreaturePartLod,
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
        for (axis, coordinate) in vertex.position.into_iter().enumerate() {
            bounds[0][axis] = bounds[0][axis].min(coordinate);
            bounds[1][axis] = bounds[1][axis].max(coordinate);
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
    let projection = PreviewProjection {
        min_x,
        max_z,
        scale,
    };
    for (slot, color) in colors {
        let Some(part) = pack.parts.get(&slot) else {
            continue;
        };
        draw_part_triangles(&mut image, pack, slot, part, color, projection);
    }
    image
}

#[derive(Clone, Copy)]
struct PreviewProjection {
    min_x: f64,
    max_z: f64,
    scale: f64,
}

fn draw_part_triangles(
    image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    pack: &SlicedCreaturePartPack,
    slot: CreaturePartSlot,
    part: &GeneratedPartMesh,
    color: [u8; 4],
    projection: PreviewProjection,
) {
    for triangle in part.indices.chunks_exact(3) {
        let points = [0_usize, 1, 2].map(|corner| {
            let index = triangle[corner];
            let position = assembled_position(pack, slot, part.vertices[index as usize].position);
            [
                (position[0] - projection.min_x) * projection.scale
                    + f64::from(image.width()) * 0.1,
                (projection.max_z - position[2]) * projection.scale
                    + f64::from(image.height()) * 0.1,
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
        assert!(CreaturePartBuilderCommand::parse_for_test([
            "validate-geneforge-staging",
            "--staging",
            "target/artifacts/creature_parts/geneforge-staging",
            "--recipes",
            "crates/alife_game_app/assets/production_voxel_v1/creature_parts/geneforge_recipes.json",
        ])
        .is_ok());
        assert!(CreaturePartBuilderCommand::parse_for_test([
            "validate-geneforge-staging",
            "--staging",
            "target/artifacts/creature_parts/geneforge-staging",
        ])
        .is_err());
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
    fn audit_atlas_parses_a_recipe_bound_target_artifact_directory() {
        let command = CreaturePartBuilderCommand::parse_for_test([
            "audit-atlas",
            "--recipes",
            "crates/alife_game_app/assets/production_voxel_v1/creature_parts/geneforge_recipes.json",
            "--output",
            "target/artifacts/creature_parts/geneforge-audit",
        ])
        .unwrap();
        assert_eq!(
            command,
            CreaturePartBuilderCommand::AuditAtlas {
                recipes: PathBuf::from(
                    "crates/alife_game_app/assets/production_voxel_v1/creature_parts/geneforge_recipes.json"
                ),
                output: PathBuf::from("target/artifacts/creature_parts/geneforge-audit"),
            }
        );
    }

    #[test]
    fn audit_atlas_rejects_committed_or_unbounded_output_paths() {
        for output in [
            "crates/alife_game_app/assets/geneforge-audit",
            "target/artifacts/creature_parts/../geneforge-audit",
        ] {
            assert!(CreaturePartBuilderCommand::parse_for_test([
                "audit-atlas",
                "--recipes",
                "crates/alife_game_app/assets/production_voxel_v1/creature_parts/geneforge_recipes.json",
                "--output",
                output,
            ])
            .is_err());
        }
    }

    #[test]
    fn audit_atlas_emits_all_lod_view_and_pose_sheets_with_metadata() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let output = workspace.join("target/artifacts/creature_parts/geneforge-audit-command-test");
        let _ = fs::remove_dir_all(&output);
        audit_atlas(&workspace.join(DEFAULT_GENEFORGE_RECIPES), &output).unwrap();

        let expected = [
            "full_front.png",
            "full_three-quarter.png",
            "full_back.png",
            "compact_front.png",
            "compact_three-quarter.png",
            "compact_back.png",
            "impostor_front.png",
            "impostor_three-quarter.png",
            "impostor_back.png",
            "full_upright.png",
            "full_resting.png",
            "full_sleeping.png",
        ];
        for name in expected {
            let image = image::open(output.join(name)).unwrap().to_rgba8();
            assert_eq!(image.dimensions(), (1440, 1080), "{name}");
            assert!(
                image.pixels().any(|pixel| pixel.0 != [52, 54, 57, 255]),
                "{name} must contain rendered creatures"
            );
        }
        let metadata: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(output.join("audit_metadata.json")).unwrap())
                .unwrap();
        assert_eq!(metadata["schema"], "alife.geneforge_audit_atlas.v1");
        let families = metadata["families"].as_array().unwrap();
        assert_eq!(families.len(), 12);
        assert!(families.iter().all(|family| {
            family["pixel_occupancy_ratio"].as_f64().unwrap() > 0.005
                && family["eye_pixel_occupancy_ratio"].as_f64().unwrap() > 0.0
                && family["nearest_silhouette_distance"].as_f64().unwrap() > 0.01
        }));
        assert_eq!(metadata["sheets"].as_array().unwrap().len(), expected.len());
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
