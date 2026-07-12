//! Deterministic, display-only layered terrain mesh generation.

use std::collections::{BTreeMap, BTreeSet};

use alife_world::VoxelTileCoord;
use bevy::{
    asset::RenderAssetUsages,
    mesh::Indices,
    prelude::{Mesh, Vec3},
    render::render_resource::PrimitiveTopology,
};

use crate::{
    production_terrain::{
        ProductionTerrainSample, ProductionTerrainSampleMap, TerrainAtlasLayout, TerrainAtlasUvRect,
    },
    terrain_materials::production_terrain_material_spec,
    Fvr03ProductionVoxelMaterialKind, Fvr11TerrainSurfaceRole,
};

pub(crate) struct TerrainMeshLayer {
    pub role: Fvr11TerrainSurfaceRole,
    pub material: Fvr03ProductionVoxelMaterialKind,
    pub mesh: Mesh,
    pub source_tile_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TerrainMeshStats {
    pub source_tiles: usize,
    pub top_quads: usize,
    pub cliff_quads: usize,
    pub transition_edges: usize,
    pub water_quads: usize,
    pub confetti_detail_quads: usize,
    pub max_vertices_per_source_tile: usize,
}

pub(crate) struct TerrainMeshBuild {
    pub layers: Vec<TerrainMeshLayer>,
    pub stats: TerrainMeshStats,
}

#[derive(Default)]
struct MeshAccumulator {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
    source_tiles: BTreeSet<VoxelTileCoord>,
}

impl MeshAccumulator {
    fn push_quad(
        &mut self,
        positions: [[f32; 3]; 4],
        normals: [[f32; 3]; 4],
        uvs: [[f32; 2]; 4],
        colors: [[f32; 4]; 4],
        source_tile: VoxelTileCoord,
    ) {
        let base = self.positions.len() as u32;
        self.positions.extend(positions);
        self.normals.extend(normals);
        self.uvs.extend(uvs);
        self.colors.extend(colors);
        self.indices
            .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        self.source_tiles.insert(source_tile);
    }

    fn finish(self) -> (Mesh, usize) {
        let source_tile_count = self.source_tiles.len();
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh.generate_tangents()
            .expect("FVR11 terrain mesh needs valid tangents for normal mapping");
        (mesh, source_tile_count)
    }
}

#[derive(Debug, Clone, Copy)]
struct SurfaceCorners {
    // North-west, south-west, south-east, north-east.
    heights: [f32; 4],
}

#[derive(Debug, Clone, Copy)]
enum TerrainEdge {
    North,
    East,
    South,
    West,
}

impl TerrainEdge {
    const ALL: [Self; 4] = [Self::North, Self::East, Self::South, Self::West];

    const fn offset(self, step: i32) -> (i32, i32) {
        match self {
            Self::North => (0, -step),
            Self::East => (step, 0),
            Self::South => (0, step),
            Self::West => (-step, 0),
        }
    }

    const fn opposite(self) -> Self {
        match self {
            Self::North => Self::South,
            Self::East => Self::West,
            Self::South => Self::North,
            Self::West => Self::East,
        }
    }
}

pub(crate) fn build_production_terrain_meshes(
    samples: &ProductionTerrainSampleMap,
    tile_stride: f32,
    atlas: TerrainAtlasLayout,
) -> TerrainMeshBuild {
    assert!(tile_stride > 0.0);
    let tile_step = tile_stride.round().max(1.0) as i32;
    let surfaces = samples
        .iter()
        .map(|(tile, sample)| (*tile, smoothed_surface_corners(samples, sample, tile_step)))
        .collect::<BTreeMap<_, _>>();
    let mut batches = BTreeMap::<
        (Fvr11TerrainSurfaceRole, Fvr03ProductionVoxelMaterialKind),
        MeshAccumulator,
    >::new();
    let mut vertices_by_tile = BTreeMap::<VoxelTileCoord, usize>::new();
    let mut stats = TerrainMeshStats {
        source_tiles: samples.len(),
        top_quads: 0,
        cliff_quads: 0,
        transition_edges: 0,
        water_quads: 0,
        confetti_detail_quads: 0,
        max_vertices_per_source_tile: 0,
    };

    for sample in samples.values() {
        let surface = surfaces[&sample.tile];
        append_top_surface(
            &mut batches,
            sample,
            surface,
            tile_stride,
            atlas,
            &mut vertices_by_tile,
        );
        if sample.material == Fvr03ProductionVoxelMaterialKind::Water {
            stats.water_quads += 1;
        } else {
            stats.top_quads += 1;
        }
    }

    for sample in samples.values() {
        for edge in [TerrainEdge::East, TerrainEdge::South] {
            let (dx, dz) = edge.offset(tile_step);
            let neighbor_tile = VoxelTileCoord::new(sample.tile.x + dx, sample.tile.z + dz);
            let Some(neighbor) = samples.get(&neighbor_tile) else {
                continue;
            };
            let height_delta =
                (display_surface_height(sample) - display_surface_height(neighbor)).abs();
            let water_boundary = (sample.material == Fvr03ProductionVoxelMaterialKind::Water)
                != (neighbor.material == Fvr03ProductionVoxelMaterialKind::Water);
            if height_delta >= 0.24 || (water_boundary && height_delta >= 0.08) {
                append_interior_cliff(
                    &mut batches,
                    sample,
                    surfaces[&sample.tile],
                    neighbor,
                    surfaces[&neighbor.tile],
                    edge,
                    tile_stride,
                    atlas,
                    &mut vertices_by_tile,
                );
                stats.cliff_quads += 1;
                if !water_boundary && sample.material != neighbor.material {
                    append_transition_strip(
                        &mut batches,
                        sample,
                        neighbor,
                        edge,
                        tile_stride,
                        atlas,
                        &mut vertices_by_tile,
                    );
                    stats.transition_edges += 1;
                }
            } else if sample.material != neighbor.material {
                append_transition_strip(
                    &mut batches,
                    sample,
                    neighbor,
                    edge,
                    tile_stride,
                    atlas,
                    &mut vertices_by_tile,
                );
                stats.transition_edges += 1;
            }
        }
    }

    for sample in samples.values() {
        for edge in TerrainEdge::ALL {
            let (dx, dz) = edge.offset(tile_step);
            let neighbor_tile = VoxelTileCoord::new(sample.tile.x + dx, sample.tile.z + dz);
            if samples.contains_key(&neighbor_tile) {
                continue;
            }
            append_perimeter_cliff(
                &mut batches,
                sample,
                surfaces[&sample.tile],
                edge,
                tile_stride,
                atlas,
                &mut vertices_by_tile,
            );
            stats.cliff_quads += 1;
        }
    }

    stats.max_vertices_per_source_tile =
        vertices_by_tile.values().copied().max().unwrap_or_default();
    let layers = batches
        .into_iter()
        .map(|((role, material), batch)| {
            let (mesh, source_tile_count) = batch.finish();
            TerrainMeshLayer {
                role,
                material,
                mesh,
                source_tile_count,
            }
        })
        .collect();
    TerrainMeshBuild { layers, stats }
}

fn append_top_surface(
    batches: &mut BTreeMap<
        (Fvr11TerrainSurfaceRole, Fvr03ProductionVoxelMaterialKind),
        MeshAccumulator,
    >,
    sample: &ProductionTerrainSample,
    surface: SurfaceCorners,
    tile_stride: f32,
    atlas: TerrainAtlasLayout,
    vertices_by_tile: &mut BTreeMap<VoxelTileCoord, usize>,
) {
    let role = if sample.material == Fvr03ProductionVoxelMaterialKind::Water {
        Fvr11TerrainSurfaceRole::Water
    } else {
        Fvr11TerrainSurfaceRole::Top
    };
    let half = tile_stride * 0.5;
    let slot = terrain_atlas_slot(sample.material, role);
    let rect = atlas.slot_rect(slot);
    if role == Fvr11TerrainSurfaceRole::Water {
        let height = display_surface_height(sample);
        let positions = [
            [sample.center_x - half, height, sample.center_z - half],
            [sample.center_x - half, height, sample.center_z + half],
            [sample.center_x + half, height, sample.center_z + half],
            [sample.center_x + half, height, sample.center_z - half],
        ];
        let colors = terrain_vertex_colors(sample.material, role, positions);
        batches
            .entry((role, sample.material))
            .or_default()
            .push_quad(
                positions,
                [[0.0, 1.0, 0.0]; 4],
                atlas_uvs(rect, sample.visual_bucket),
                colors,
                sample.tile,
            );
        *vertices_by_tile.entry(sample.tile).or_default() += 4;
        return;
    }

    const SUBDIVISIONS: usize = 4;
    for column in 0..SUBDIVISIONS {
        for row in 0..SUBDIVISIONS {
            let u0 = column as f32 / SUBDIVISIONS as f32;
            let u1 = (column + 1) as f32 / SUBDIVISIONS as f32;
            let v0 = row as f32 / SUBDIVISIONS as f32;
            let v1 = (row + 1) as f32 / SUBDIVISIONS as f32;
            let positions = [
                terrain_top_position(sample, surface, tile_stride, u0, v0),
                terrain_top_position(sample, surface, tile_stride, u0, v1),
                terrain_top_position(sample, surface, tile_stride, u1, v1),
                terrain_top_position(sample, surface, tile_stride, u1, v0),
            ];
            let normals = [
                terrain_top_normal(sample, surface, tile_stride, u0, v0),
                terrain_top_normal(sample, surface, tile_stride, u0, v1),
                terrain_top_normal(sample, surface, tile_stride, u1, v1),
                terrain_top_normal(sample, surface, tile_stride, u1, v0),
            ];
            let colors = terrain_vertex_colors(sample.material, role, positions);
            batches
                .entry((role, sample.material))
                .or_default()
                .push_quad(
                    positions,
                    normals,
                    atlas_grid_uvs(rect, column, row, SUBDIVISIONS),
                    colors,
                    sample.tile,
                );
            *vertices_by_tile.entry(sample.tile).or_default() += 4;
        }
    }
}

fn terrain_top_position(
    sample: &ProductionTerrainSample,
    surface: SurfaceCorners,
    tile_stride: f32,
    u: f32,
    v: f32,
) -> [f32; 3] {
    let half = tile_stride * 0.5;
    let x = sample.center_x - half + tile_stride * u;
    let z = sample.center_z - half + tile_stride * v;
    let base_height = bilinear_scalar(surface.heights, u, v);
    let sin_u = (std::f32::consts::PI * u).sin();
    let sin_v = (std::f32::consts::PI * v).sin();
    let edge_envelope = sin_u * sin_u * sin_v * sin_v;
    let relief = ((x * 1.73 + z * 0.61).sin() * 0.050
        + (x * 0.47 - z * 1.37).cos() * 0.032
        + ((x + z) * 2.11).sin() * 0.014)
        * edge_envelope;
    [x, base_height + relief, z]
}

fn terrain_top_normal(
    sample: &ProductionTerrainSample,
    surface: SurfaceCorners,
    tile_stride: f32,
    u: f32,
    v: f32,
) -> [f32; 3] {
    let stride = tile_stride.max(f32::EPSILON);
    let half = stride * 0.5;
    let x = sample.center_x - half + stride * u;
    let z = sample.center_z - half + stride * v;
    let [southwest, northwest, northeast, southeast] = surface.heights;
    let base_dx = ((southeast - southwest) * (1.0 - v) + (northeast - northwest) * v) / stride;
    let base_dz = ((northwest - southwest) * (1.0 - u) + (northeast - southeast) * u) / stride;

    let phase_a = x * 1.73 + z * 0.61;
    let phase_b = x * 0.47 - z * 1.37;
    let phase_c = (x + z) * 2.11;
    let noise = phase_a.sin() * 0.050 + phase_b.cos() * 0.032 + phase_c.sin() * 0.014;
    let noise_dx =
        phase_a.cos() * 1.73 * 0.050 - phase_b.sin() * 0.47 * 0.032 + phase_c.cos() * 2.11 * 0.014;
    let noise_dz =
        phase_a.cos() * 0.61 * 0.050 + phase_b.sin() * 1.37 * 0.032 + phase_c.cos() * 2.11 * 0.014;
    let sin_u = (std::f32::consts::PI * u).sin();
    let sin_v = (std::f32::consts::PI * v).sin();
    let envelope = sin_u * sin_u * sin_v * sin_v;
    let envelope_dx =
        2.0 * std::f32::consts::PI * sin_u * (std::f32::consts::PI * u).cos() * sin_v * sin_v
            / stride;
    let envelope_dz =
        2.0 * std::f32::consts::PI * sin_u * sin_u * sin_v * (std::f32::consts::PI * v).cos()
            / stride;
    let relief_dx = noise_dx * envelope + noise * envelope_dx;
    let relief_dz = noise_dz * envelope + noise * envelope_dz;

    Vec3::new(-(base_dx + relief_dx), 1.0, -(base_dz + relief_dz))
        .normalize()
        .to_array()
}

fn bilinear_scalar(corners: [f32; 4], u: f32, v: f32) -> f32 {
    corners[0] * (1.0 - u) * (1.0 - v)
        + corners[1] * (1.0 - u) * v
        + corners[2] * u * v
        + corners[3] * u * (1.0 - v)
}

#[allow(clippy::too_many_arguments)]
fn append_interior_cliff(
    batches: &mut BTreeMap<
        (Fvr11TerrainSurfaceRole, Fvr03ProductionVoxelMaterialKind),
        MeshAccumulator,
    >,
    first: &ProductionTerrainSample,
    first_surface: SurfaceCorners,
    second: &ProductionTerrainSample,
    second_surface: SurfaceCorners,
    first_edge: TerrainEdge,
    tile_stride: f32,
    atlas: TerrainAtlasLayout,
    vertices_by_tile: &mut BTreeMap<VoxelTileCoord, usize>,
) {
    let (high, high_surface, edge, low) =
        if display_surface_height(first) >= display_surface_height(second) {
            (first, first_surface, first_edge, second)
        } else {
            (second, second_surface, first_edge.opposite(), first)
        };
    let material = if first.material == Fvr03ProductionVoxelMaterialKind::Water
        || second.material == Fvr03ProductionVoxelMaterialKind::Water
    {
        Fvr03ProductionVoxelMaterialKind::Water
    } else {
        high.material
    };
    append_cliff_quad(
        batches,
        high,
        high_surface,
        edge,
        display_surface_height(low),
        material,
        tile_stride,
        atlas,
        vertices_by_tile,
    );
}

#[allow(clippy::too_many_arguments)]
fn append_perimeter_cliff(
    batches: &mut BTreeMap<
        (Fvr11TerrainSurfaceRole, Fvr03ProductionVoxelMaterialKind),
        MeshAccumulator,
    >,
    sample: &ProductionTerrainSample,
    surface: SurfaceCorners,
    edge: TerrainEdge,
    tile_stride: f32,
    atlas: TerrainAtlasLayout,
    vertices_by_tile: &mut BTreeMap<VoxelTileCoord, usize>,
) {
    append_cliff_quad(
        batches,
        sample,
        surface,
        edge,
        0.0,
        sample.material,
        tile_stride,
        atlas,
        vertices_by_tile,
    );
}

#[allow(clippy::too_many_arguments)]
fn append_cliff_quad(
    batches: &mut BTreeMap<
        (Fvr11TerrainSurfaceRole, Fvr03ProductionVoxelMaterialKind),
        MeshAccumulator,
    >,
    sample: &ProductionTerrainSample,
    surface: SurfaceCorners,
    edge: TerrainEdge,
    bottom_height: f32,
    material: Fvr03ProductionVoxelMaterialKind,
    tile_stride: f32,
    atlas: TerrainAtlasLayout,
    vertices_by_tile: &mut BTreeMap<VoxelTileCoord, usize>,
) {
    let (top_a, top_b, normal) = edge_top_pair(sample, surface, edge, tile_stride);
    let positions = [
        [top_a[0], bottom_height, top_a[2]],
        [top_b[0], bottom_height, top_b[2]],
        top_b,
        top_a,
    ];
    let role = Fvr11TerrainSurfaceRole::Cliff;
    let slot = terrain_atlas_slot(material, role);
    let colors = terrain_vertex_colors(material, role, positions);
    batches.entry((role, material)).or_default().push_quad(
        positions,
        [normal; 4],
        atlas_uvs(atlas.slot_rect(slot), sample.visual_bucket),
        colors,
        sample.tile,
    );
    *vertices_by_tile.entry(sample.tile).or_default() += 4;
}

#[allow(clippy::too_many_arguments)]
fn append_transition_strip(
    batches: &mut BTreeMap<
        (Fvr11TerrainSurfaceRole, Fvr03ProductionVoxelMaterialKind),
        MeshAccumulator,
    >,
    first: &ProductionTerrainSample,
    second: &ProductionTerrainSample,
    edge: TerrainEdge,
    tile_stride: f32,
    atlas: TerrainAtlasLayout,
    vertices_by_tile: &mut BTreeMap<VoxelTileCoord, usize>,
) {
    let material = transition_material(first.material, second.material);
    let owner = if first.material == material {
        first
    } else {
        second
    };
    let half = tile_stride * 0.5;
    let edge_hash = terrain_edge_hash(first.tile, second.tile);
    let width_multiplier = transition_width_multiplier(first.material, second.material);
    let y = display_surface_height(first).max(display_surface_height(second)) + 0.018;
    let role = Fvr11TerrainSurfaceRole::Transition;
    let slot = terrain_atlas_slot(material, role);
    let rect = atlas.slot_rect(slot);
    const SEGMENTS: usize = 3;
    for segment in 0..SEGMENTS {
        let start = -half + tile_stride * segment as f32 / SEGMENTS as f32;
        let end = -half + tile_stride * (segment + 1) as f32 / SEGMENTS as f32;
        let (offset_a, width_a) =
            transition_control_point(edge_hash, segment, tile_stride, width_multiplier);
        let (offset_b, width_b) =
            transition_control_point(edge_hash, segment + 1, tile_stride, width_multiplier);
        let positions = match edge {
            TerrainEdge::East => {
                let x = first.center_x + half;
                [
                    [x + offset_a - width_a, y, first.center_z + start],
                    [x + offset_b - width_b, y, first.center_z + end],
                    [x + offset_b + width_b, y, first.center_z + end],
                    [x + offset_a + width_a, y, first.center_z + start],
                ]
            }
            TerrainEdge::South => {
                let z = first.center_z + half;
                [
                    [first.center_x + start, y, z + offset_a - width_a],
                    [first.center_x + start, y, z + offset_a + width_a],
                    [first.center_x + end, y, z + offset_b + width_b],
                    [first.center_x + end, y, z + offset_b - width_b],
                ]
            }
            TerrainEdge::North | TerrainEdge::West => {
                unreachable!("canonical interior edges only")
            }
        };
        let colors = terrain_vertex_colors(material, role, positions);
        batches.entry((role, material)).or_default().push_quad(
            positions,
            [[0.0, 1.0, 0.0]; 4],
            atlas_segment_uvs(rect, segment, SEGMENTS),
            colors,
            owner.tile,
        );
        *vertices_by_tile.entry(owner.tile).or_default() += 4;
    }
}

fn transition_control_point(
    edge_hash: u32,
    point: usize,
    tile_stride: f32,
    width_multiplier: f32,
) -> (f32, f32) {
    let bits = edge_hash.rotate_left((point as u32 * 7) % 31);
    let offset = (f32::from((bits & 0x0f) as u8) / 15.0 - 0.5) * tile_stride * 0.05;
    let half_width =
        tile_stride * (0.07 + f32::from(((bits >> 4) & 0x07) as u8) * 0.0035) * width_multiplier;
    (offset, half_width)
}

fn transition_width_multiplier(
    first: Fvr03ProductionVoxelMaterialKind,
    second: Fvr03ProductionVoxelMaterialKind,
) -> f32 {
    if matches!(
        (first, second),
        (
            Fvr03ProductionVoxelMaterialKind::Hazard,
            Fvr03ProductionVoxelMaterialKind::Decay
        ) | (
            Fvr03ProductionVoxelMaterialKind::Decay,
            Fvr03ProductionVoxelMaterialKind::Hazard
        )
    ) {
        1.65
    } else {
        1.0
    }
}

fn smoothed_surface_corners(
    samples: &ProductionTerrainSampleMap,
    sample: &ProductionTerrainSample,
    tile_step: i32,
) -> SurfaceCorners {
    if sample.material == Fvr03ProductionVoxelMaterialKind::Water {
        return SurfaceCorners {
            heights: [display_surface_height(sample); 4],
        };
    }
    let signs = [(-1, -1), (-1, 1), (1, 1), (1, -1)];
    let heights = signs.map(|(sign_x, sign_z)| {
        let coordinates = [
            (0, 0),
            (sign_x * tile_step, 0),
            (0, sign_z * tile_step),
            (sign_x * tile_step, sign_z * tile_step),
        ];
        let mut total = 0.0;
        let mut count = 0.0;
        for (dx, dz) in coordinates {
            let tile = VoxelTileCoord::new(sample.tile.x + dx, sample.tile.z + dz);
            if let Some(neighbor) = samples
                .get(&tile)
                .filter(|neighbor| neighbor.material != Fvr03ProductionVoxelMaterialKind::Water)
            {
                total += neighbor.height;
                count += 1.0;
            }
        }
        let average = if count > 0.0 {
            total / count
        } else {
            sample.height
        };
        average.clamp(sample.height - 0.20, sample.height + 0.20)
    });
    SurfaceCorners { heights }
}

fn edge_top_pair(
    sample: &ProductionTerrainSample,
    surface: SurfaceCorners,
    edge: TerrainEdge,
    stride: f32,
) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let half = stride * 0.5;
    let corners = [
        [
            sample.center_x - half,
            surface.heights[0],
            sample.center_z - half,
        ],
        [
            sample.center_x - half,
            surface.heights[1],
            sample.center_z + half,
        ],
        [
            sample.center_x + half,
            surface.heights[2],
            sample.center_z + half,
        ],
        [
            sample.center_x + half,
            surface.heights[3],
            sample.center_z - half,
        ],
    ];
    match edge {
        TerrainEdge::North => (corners[3], corners[0], [0.0, 0.0, -1.0]),
        TerrainEdge::East => (corners[2], corners[3], [1.0, 0.0, 0.0]),
        TerrainEdge::South => (corners[1], corners[2], [0.0, 0.0, 1.0]),
        TerrainEdge::West => (corners[0], corners[1], [-1.0, 0.0, 0.0]),
    }
}

fn atlas_uvs(rect: TerrainAtlasUvRect, _visual_bucket: u8) -> [[f32; 2]; 4] {
    [
        [rect.min[0], rect.min[1]],
        [rect.min[0], rect.max[1]],
        [rect.max[0], rect.max[1]],
        [rect.max[0], rect.min[1]],
    ]
}

fn atlas_grid_uvs(
    rect: TerrainAtlasUvRect,
    column: usize,
    row: usize,
    subdivisions: usize,
) -> [[f32; 2]; 4] {
    let width = rect.max[0] - rect.min[0];
    let height = rect.max[1] - rect.min[1];
    let u0 = rect.min[0] + width * column as f32 / subdivisions as f32;
    let u1 = rect.min[0] + width * (column + 1) as f32 / subdivisions as f32;
    let v0 = rect.min[1] + height * row as f32 / subdivisions as f32;
    let v1 = rect.min[1] + height * (row + 1) as f32 / subdivisions as f32;
    [[u0, v0], [u0, v1], [u1, v1], [u1, v0]]
}

fn atlas_segment_uvs(
    rect: TerrainAtlasUvRect,
    segment: usize,
    segment_count: usize,
) -> [[f32; 2]; 4] {
    let span = rect.max[1] - rect.min[1];
    let v0 = rect.min[1] + span * segment as f32 / segment_count as f32;
    let v1 = rect.min[1] + span * (segment + 1) as f32 / segment_count as f32;
    [
        [rect.min[0], v0],
        [rect.min[0], v1],
        [rect.max[0], v1],
        [rect.max[0], v0],
    ]
}

fn terrain_atlas_slot(
    material: Fvr03ProductionVoxelMaterialKind,
    role: Fvr11TerrainSurfaceRole,
) -> u8 {
    production_terrain_material_spec(material).atlas_slot(role)
}

fn terrain_vertex_colors(
    material: Fvr03ProductionVoxelMaterialKind,
    role: Fvr11TerrainSurfaceRole,
    positions: [[f32; 3]; 4],
) -> [[f32; 4]; 4] {
    let base = match material {
        Fvr03ProductionVoxelMaterialKind::SafeGrass => [0.94, 1.00, 0.88],
        Fvr03ProductionVoxelMaterialKind::Soil => [1.00, 0.96, 0.87],
        Fvr03ProductionVoxelMaterialKind::Resource => [0.94, 1.00, 0.82],
        Fvr03ProductionVoxelMaterialKind::Hazard => [1.00, 0.91, 0.94],
        Fvr03ProductionVoxelMaterialKind::Decay => [0.96, 0.88, 0.78],
        Fvr03ProductionVoxelMaterialKind::Stone => [0.95, 0.98, 0.90],
        Fvr03ProductionVoxelMaterialKind::Water => [0.88, 1.00, 1.00],
        Fvr03ProductionVoxelMaterialKind::Sand => [1.00, 0.96, 0.84],
        _ => [1.0, 1.0, 1.0],
    };
    let role_shade = match role {
        Fvr11TerrainSurfaceRole::Top => 1.0,
        Fvr11TerrainSurfaceRole::Cliff => 0.72,
        Fvr11TerrainSurfaceRole::Transition => 0.97,
        Fvr11TerrainSurfaceRole::Water => 0.96,
    };
    positions.map(|position| {
        let broad = (position[0] * 0.19).sin() * 0.032
            + (position[2] * 0.17).cos() * 0.026
            + ((position[0] + position[2]) * 0.083).sin() * 0.018;
        let factor = role_shade * (0.99 + broad).clamp(0.92, 1.06);
        [
            (base[0] * factor).clamp(0.0, 1.0),
            (base[1] * factor).clamp(0.0, 1.0),
            (base[2] * factor).clamp(0.0, 1.0),
            1.0,
        ]
    })
}

fn terrain_edge_hash(first: VoxelTileCoord, second: VoxelTileCoord) -> u32 {
    let mut value = (first.x as u32).wrapping_mul(0x9e37_79b9)
        ^ (first.z as u32).wrapping_mul(0x85eb_ca6b)
        ^ (second.x as u32).rotate_left(11)
        ^ (second.z as u32).rotate_left(23);
    value ^= value >> 16;
    value.wrapping_mul(0x7feb_352d) ^ (value >> 15)
}

fn transition_material(
    first: Fvr03ProductionVoxelMaterialKind,
    second: Fvr03ProductionVoxelMaterialKind,
) -> Fvr03ProductionVoxelMaterialKind {
    for preferred in [
        Fvr03ProductionVoxelMaterialKind::Water,
        Fvr03ProductionVoxelMaterialKind::Hazard,
        Fvr03ProductionVoxelMaterialKind::Decay,
        Fvr03ProductionVoxelMaterialKind::Soil,
        Fvr03ProductionVoxelMaterialKind::Resource,
        Fvr03ProductionVoxelMaterialKind::SafeGrass,
        Fvr03ProductionVoxelMaterialKind::Sand,
        Fvr03ProductionVoxelMaterialKind::Stone,
    ] {
        if first == preferred || second == preferred {
            return preferred;
        }
    }
    first
}

fn display_surface_height(sample: &ProductionTerrainSample) -> f32 {
    sample.height
        + if sample.material == Fvr03ProductionVoxelMaterialKind::Water {
            0.035
        } else {
            0.0
        }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alife_world::VoxelTileCoord;
    use bevy::{mesh::VertexAttributeValues, prelude::Mesh};

    use crate::{
        production_terrain::{
            ProductionTerrainSample, ProductionTerrainSampleMap, TerrainAtlasLayout,
        },
        Fvr03ProductionVoxelMaterialKind, Fvr11TerrainSurfaceRole,
    };

    fn sample_map() -> ProductionTerrainSampleMap {
        let specs = [
            (0, 0, Fvr03ProductionVoxelMaterialKind::SafeGrass, 1.00),
            (1, 0, Fvr03ProductionVoxelMaterialKind::Soil, 1.00),
            (2, 0, Fvr03ProductionVoxelMaterialKind::Stone, 1.50),
            (0, 1, Fvr03ProductionVoxelMaterialKind::SafeGrass, 1.00),
            (1, 1, Fvr03ProductionVoxelMaterialKind::Water, 0.70),
            (2, 1, Fvr03ProductionVoxelMaterialKind::Stone, 1.50),
            (0, 2, Fvr03ProductionVoxelMaterialKind::Resource, 1.00),
            (1, 2, Fvr03ProductionVoxelMaterialKind::Hazard, 0.75),
            (2, 2, Fvr03ProductionVoxelMaterialKind::Sand, 0.75),
        ];
        specs
            .into_iter()
            .enumerate()
            .map(|(index, (x, z, material, height))| {
                let tile = VoxelTileCoord::new(x, z);
                (
                    tile,
                    ProductionTerrainSample {
                        tile,
                        material,
                        center_x: x as f32 + 0.5,
                        center_z: z as f32 + 0.5,
                        height,
                        resource_bias: usize::from(
                            material == Fvr03ProductionVoxelMaterialKind::Resource,
                        ) as f32,
                        hazard_pressure: usize::from(
                            material == Fvr03ProductionVoxelMaterialKind::Hazard,
                        ) as f32,
                        visual_bucket: index as u8 % 5,
                    },
                )
            })
            .collect()
    }

    #[test]
    fn layered_mesh_build_is_deterministic_complete_and_bounded() {
        let samples = sample_map();
        let first = build_production_terrain_meshes(&samples, 1.0, TerrainAtlasLayout::PRODUCTION);
        let second = build_production_terrain_meshes(&samples, 1.0, TerrainAtlasLayout::PRODUCTION);

        assert_eq!(first.stats, second.stats);
        let receipts = |build: &TerrainMeshBuild| {
            build
                .layers
                .iter()
                .map(|layer| (layer.role, layer.material, layer.source_tile_count))
                .collect::<Vec<_>>()
        };
        assert_eq!(receipts(&first), receipts(&second));
        for role in [
            Fvr11TerrainSurfaceRole::Top,
            Fvr11TerrainSurfaceRole::Cliff,
            Fvr11TerrainSurfaceRole::Transition,
            Fvr11TerrainSurfaceRole::Water,
        ] {
            assert!(first.layers.iter().any(|layer| layer.role == role));
        }
        assert_eq!(first.stats.confetti_detail_quads, 0);
        assert!(first.stats.transition_edges > 0);
        assert!(
            first.stats.max_vertices_per_source_tile <= 128,
            "subdivided terrain exceeded its per-source-tile vertex budget: {}",
            first.stats.max_vertices_per_source_tile
        );
    }

    #[test]
    fn every_layer_has_pbr_vertex_attributes() {
        let build =
            build_production_terrain_meshes(&sample_map(), 1.0, TerrainAtlasLayout::PRODUCTION);
        assert!(!build.layers.is_empty());
        for layer in &build.layers {
            for attribute in [
                Mesh::ATTRIBUTE_POSITION,
                Mesh::ATTRIBUTE_NORMAL,
                Mesh::ATTRIBUTE_UV_0,
                Mesh::ATTRIBUTE_TANGENT,
                Mesh::ATTRIBUTE_COLOR,
            ] {
                assert!(
                    layer.mesh.attribute(attribute).is_some(),
                    "missing {:?} on {:?}/{:?}",
                    attribute,
                    layer.role,
                    layer.material
                );
            }
        }
    }

    #[test]
    fn solid_top_surfaces_use_bounded_subdivision_and_interior_relief() {
        let tile = VoxelTileCoord::new(0, 0);
        let samples = [(
            tile,
            ProductionTerrainSample {
                tile,
                material: Fvr03ProductionVoxelMaterialKind::SafeGrass,
                center_x: 0.5,
                center_z: 0.5,
                height: 1.0,
                resource_bias: 0.0,
                hazard_pressure: 0.0,
                visual_bucket: 0,
            },
        )]
        .into_iter()
        .collect::<ProductionTerrainSampleMap>();
        let build = build_production_terrain_meshes(&samples, 1.0, TerrainAtlasLayout::PRODUCTION);
        let top = build
            .layers
            .iter()
            .find(|layer| layer.role == Fvr11TerrainSurfaceRole::Top)
            .expect("solid top layer");
        let Some(VertexAttributeValues::Float32x3(positions)) =
            top.mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("solid top positions");
        };
        let unique_heights = positions
            .iter()
            .map(|position| (position[1] * 10_000.0).round() as i32)
            .collect::<BTreeSet<_>>();

        assert_eq!(
            positions.len(),
            64,
            "each solid tile should use a bounded four-by-four quad patch"
        );
        assert!(
            unique_heights.len() >= 5,
            "solid terrain needs deterministic interior relief instead of one flat slab"
        );

        let Some(VertexAttributeValues::Float32x3(normals)) =
            top.mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("solid top normals");
        };
        let surface = SurfaceCorners { heights: [1.0; 4] };
        let expected = terrain_top_normal(&samples[&tile], surface, 1.0, 0.25, 0.25);
        let actual = Vec3::from_array(normals[20]);
        assert!(
            actual.dot(Vec3::from_array(expected)) > 0.9999,
            "subdivided terrain normals must include the interior relief derivative"
        );
    }

    #[test]
    fn elevated_material_boundaries_add_a_mossy_transition_collar() {
        let samples = [
            (0, Fvr03ProductionVoxelMaterialKind::SafeGrass, 1.0),
            (1, Fvr03ProductionVoxelMaterialKind::Stone, 1.5),
        ]
        .into_iter()
        .map(|(x, material, height)| {
            let tile = VoxelTileCoord::new(x, 0);
            (
                tile,
                ProductionTerrainSample {
                    tile,
                    material,
                    center_x: x as f32 + 0.5,
                    center_z: 0.5,
                    height,
                    resource_bias: 0.0,
                    hazard_pressure: 0.0,
                    visual_bucket: x as u8,
                },
            )
        })
        .collect::<ProductionTerrainSampleMap>();
        let build = build_production_terrain_meshes(&samples, 1.0, TerrainAtlasLayout::PRODUCTION);

        assert!(build.stats.cliff_quads > 0);
        assert_eq!(
            build.stats.transition_edges, 1,
            "raised biome boundaries should receive one top-textured transition collar"
        );
    }

    #[test]
    fn material_transition_uses_a_segmented_irregular_ecotone() {
        let samples = [
            (0, Fvr03ProductionVoxelMaterialKind::SafeGrass),
            (1, Fvr03ProductionVoxelMaterialKind::Hazard),
        ]
        .into_iter()
        .map(|(x, material)| {
            let tile = VoxelTileCoord::new(x, 0);
            (
                tile,
                ProductionTerrainSample {
                    tile,
                    material,
                    center_x: x as f32 + 0.5,
                    center_z: 0.5,
                    height: 1.0,
                    resource_bias: 0.0,
                    hazard_pressure: usize::from(
                        material == Fvr03ProductionVoxelMaterialKind::Hazard,
                    ) as f32,
                    visual_bucket: x as u8,
                },
            )
        })
        .collect::<ProductionTerrainSampleMap>();
        let build = build_production_terrain_meshes(&samples, 1.0, TerrainAtlasLayout::PRODUCTION);
        let transition = build
            .layers
            .iter()
            .find(|layer| layer.role == Fvr11TerrainSurfaceRole::Transition)
            .expect("hazard ecotone layer");
        let Some(VertexAttributeValues::Float32x3(positions)) =
            transition.mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("transition positions");
        };
        let unique_cross_edge = positions
            .iter()
            .map(|position| (position[0] * 1_000.0).round() as i32)
            .collect::<BTreeSet<_>>();
        let min_x = positions
            .iter()
            .map(|position| position[0])
            .fold(f32::INFINITY, f32::min);
        let max_x = positions
            .iter()
            .map(|position| position[0])
            .fold(f32::NEG_INFINITY, f32::max);

        assert_eq!(
            positions.len(),
            12,
            "ecotones should use three joined quads"
        );
        assert!(
            unique_cross_edge.len() >= 6,
            "ecotone edges need deterministic lateral variation"
        );
        assert!(
            max_x - min_x >= 0.14,
            "ecotones should visibly overlap both biome tops"
        );
        assert!(
            max_x - min_x <= 0.24,
            "ecotones must not cover the path or form broad sawtooth overlays"
        );
    }

    #[test]
    fn fungal_decay_ecotones_blend_wider_than_walkable_path_edges() {
        let transition_span = |first_material, second_material| {
            let samples = [(0, first_material), (1, second_material)]
                .into_iter()
                .map(|(x, material)| {
                    let tile = VoxelTileCoord::new(x, 0);
                    (
                        tile,
                        ProductionTerrainSample {
                            tile,
                            material,
                            center_x: x as f32 + 0.5,
                            center_z: 0.5,
                            height: 1.0,
                            resource_bias: 0.0,
                            hazard_pressure: usize::from(
                                material == Fvr03ProductionVoxelMaterialKind::Hazard,
                            ) as f32,
                            visual_bucket: x as u8,
                        },
                    )
                })
                .collect::<ProductionTerrainSampleMap>();
            let build =
                build_production_terrain_meshes(&samples, 1.0, TerrainAtlasLayout::PRODUCTION);
            let transition = build
                .layers
                .iter()
                .find(|layer| layer.role == Fvr11TerrainSurfaceRole::Transition)
                .expect("transition layer");
            let Some(VertexAttributeValues::Float32x3(positions)) =
                transition.mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                panic!("transition positions");
            };
            let min_x = positions
                .iter()
                .map(|position| position[0])
                .fold(f32::INFINITY, f32::min);
            let max_x = positions
                .iter()
                .map(|position| position[0])
                .fold(f32::NEG_INFINITY, f32::max);
            max_x - min_x
        };

        let path_span = transition_span(
            Fvr03ProductionVoxelMaterialKind::SafeGrass,
            Fvr03ProductionVoxelMaterialKind::Soil,
        );
        let fungal_span = transition_span(
            Fvr03ProductionVoxelMaterialKind::Hazard,
            Fvr03ProductionVoxelMaterialKind::Decay,
        );
        assert!(fungal_span >= path_span * 1.50);
        assert!(fungal_span <= 0.42);
    }

    #[test]
    fn atlas_uv_orientation_is_stable_across_tiles_to_avoid_grid_seams() {
        let rect = TerrainAtlasLayout::PRODUCTION.slot_rect(0);
        assert_eq!(atlas_uvs(rect, 0), atlas_uvs(rect, 7));
    }

    #[test]
    fn shared_top_corners_use_identical_smoothed_normals() {
        let samples = [(0, 0, 1.00), (1, 0, 1.25), (0, 1, 0.85), (1, 1, 1.10)]
            .into_iter()
            .map(|(x, z, height)| {
                let tile = VoxelTileCoord::new(x, z);
                (
                    tile,
                    ProductionTerrainSample {
                        tile,
                        material: Fvr03ProductionVoxelMaterialKind::SafeGrass,
                        center_x: x as f32 + 0.5,
                        center_z: z as f32 + 0.5,
                        height,
                        resource_bias: 0.0,
                        hazard_pressure: 0.0,
                        visual_bucket: (x + z) as u8,
                    },
                )
            })
            .collect::<ProductionTerrainSampleMap>();
        let build = build_production_terrain_meshes(&samples, 1.0, TerrainAtlasLayout::PRODUCTION);
        let layer = build
            .layers
            .iter()
            .find(|layer| {
                layer.role == Fvr11TerrainSurfaceRole::Top
                    && layer.material == Fvr03ProductionVoxelMaterialKind::SafeGrass
            })
            .expect("safe grass top layer");
        let Some(VertexAttributeValues::Float32x3(positions)) =
            layer.mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("terrain positions");
        };
        let Some(VertexAttributeValues::Float32x3(normals)) =
            layer.mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("terrain normals");
        };
        let mut by_position = BTreeMap::<(i32, i32, i32), Vec<[f32; 3]>>::new();
        for (position, normal) in positions.iter().zip(normals) {
            let key = (
                (position[0] * 1_000.0).round() as i32,
                (position[1] * 1_000.0).round() as i32,
                (position[2] * 1_000.0).round() as i32,
            );
            by_position.entry(key).or_default().push(*normal);
        }
        let shared = by_position
            .values()
            .filter(|normals| normals.len() > 1)
            .collect::<Vec<_>>();
        assert!(!shared.is_empty());
        for normals in shared {
            let first = normals[0];
            assert!(normals.iter().all(|normal| {
                (normal[0] - first[0]).abs() < 1.0e-5
                    && (normal[1] - first[1]).abs() < 1.0e-5
                    && (normal[2] - first[2]).abs() < 1.0e-5
            }));
        }
    }
}
