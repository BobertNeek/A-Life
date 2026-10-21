use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
};

use alife_game_app::{
    creature_face_style_from_landmarks, creature_part_pose, creature_root_pose,
    load_geneforge_assembly_preparation_index, parse_geneforge_runtime_groups,
    remap_creature_face_landmarks, resolve_creature_coat_palette,
    resolve_geneforge_creature_assembly, CreatureAnimationState, CreaturePartLodId,
    CreaturePartSlot, CreatureVisualBounds, GeneForgeCreaturePartCatalog, GeneForgeDonorId,
    PartMeshData,
};
use alife_world::{CreatureAppearanceGenome, CreaturePartSources};
use image::{ImageBuffer, Rgba};
use serde_json::{json, Value};

const DEFAULT_GENEFORGE_RECIPES: &str =
    "crates/alife_game_app/assets/production_voxel_v1/creature_parts/geneforge_recipes.json";

fn run(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let command = args.first().ok_or("expected audit-atlas")?;
    let options = parse_options(&args[1..])?;
    let recipes = options
        .get("recipes")
        .map(PathBuf::from)
        .unwrap_or_else(|| DEFAULT_GENEFORGE_RECIPES.into());
    match command.as_str() {
        "audit-atlas" => {
            let output = options
                .get("output")
                .map(PathBuf::from)
                .unwrap_or_else(|| "target/artifacts/creature_parts/geneforge-audit".into());
            audit_atlas(&recipes, &output)
        }
        _ => Err(format!("unknown creature part builder command {command}").into()),
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
    appearance: CreatureAppearanceGenome,
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
                "head": self.appearance.part_sources.head.0,
                "torso": self.appearance.part_sources.torso.0,
                "arms": self.appearance.part_sources.arms.0,
                "legs": self.appearance.part_sources.legs.0,
                "tail": self.appearance.part_sources.tail.0,
                "palette_family": self.appearance.palette_family,
                "fur_pattern": self.appearance.fur_pattern,
                "marking_density": self.appearance.marking_density,
            },
            "projected_bounds": [bounds[0][0], bounds[0][1], bounds[1][0], bounds[1][1]],
            "eye_bounds": [
                eyes[0][0] - self.eye_radius,
                eyes[0][1] - self.eye_radius,
                eyes[1][0] + self.eye_radius,
                eyes[1][1] + self.eye_radius,
            ],
            "socket_error": self.attachment_error,
            "triangle_count": self.triangle_count,
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
        let mut appearance = CreatureAppearanceGenome::founder_for_species(
            family.id.0 as u8,
            0xA71A_5000 + u64::from(family.id.0),
        );
        appearance.part_sources = sources;
        appearance.palette_family = (family.id.0 as u8).wrapping_mul(37);
        appearance.fur_pattern = (family.id.0 as u8).wrapping_mul(53);
        appearance.marking_density = 96_u8.wrapping_add((family.id.0 as u8).wrapping_mul(11));
        let recipe = resolve_geneforge_creature_assembly(
            sources,
            lod,
            appearance.palette_family,
            catalog,
            preparations,
        )?;
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
            appearance,
            palette: resolve_creature_coat_palette(appearance.palette_family),
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
    for (coordinate, translation) in world.iter_mut().zip(part_pose.translation) {
        *coordinate += f64::from(translation);
    }
    let root = creature_root_pose(animation, 0.0, 0.0);
    world = rotate_xyz(world, root.rotation_xyz.map(f64::from));
    for (coordinate, translation) in world.iter_mut().zip(root.translation) {
        *coordinate += f64::from(translation);
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
            for (triangle_index, triangle) in part.indices.as_chunks::<3>().0.iter().enumerate() {
                let vertices = [0_usize, 1, 2].map(|corner| part_points[triangle[corner] as usize]);
                let color = audit_triangle_color(model, part.slot, vertices, triangle_index);
                raster_audit_triangle(&mut image, &mut z_buffer, projection, vertices, color);
            }
        }
        if view_angle.cos() > -0.1 && !matches!(pose, AuditPose::Sleeping) {
            for eye in &model.eyes {
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
                    x - radius * 0.18,
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
        (triangle_index + usize::from(model.appearance.fur_pattern) + slot as usize * 7) % 11;
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

fn edge(a: [f64; 2], b: [f64; 2], point: [f64; 2]) -> f64 {
    (point[0] - a[0]) * (b[1] - a[1]) - (point[1] - a[1]) * (b[0] - a[0])
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
    if let Err(error) = run(env::args().skip(1).collect()) {
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
