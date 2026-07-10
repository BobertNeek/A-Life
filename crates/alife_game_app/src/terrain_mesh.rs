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
    let heights = if role == Fvr11TerrainSurfaceRole::Water {
        [display_surface_height(sample); 4]
    } else {
        surface.heights
    };
    let positions = [
        [sample.center_x - half, heights[0], sample.center_z - half],
        [sample.center_x - half, heights[1], sample.center_z + half],
        [sample.center_x + half, heights[2], sample.center_z + half],
        [sample.center_x + half, heights[3], sample.center_z - half],
    ];
    let normals = if role == Fvr11TerrainSurfaceRole::Water {
        [[0.0, 1.0, 0.0]; 4]
    } else {
        top_surface_normals(heights, tile_stride)
    };
    let slot = terrain_atlas_slot(sample.material, role);
    batches
        .entry((role, sample.material))
        .or_default()
        .push_quad(
            positions,
            normals,
            atlas_uvs(atlas.slot_rect(slot), sample.visual_bucket),
            terrain_vertex_colors(sample.material, role, sample.visual_bucket),
            sample.tile,
        );
    *vertices_by_tile.entry(sample.tile).or_default() += 4;
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
    batches.entry((role, material)).or_default().push_quad(
        positions,
        [normal; 4],
        atlas_uvs(atlas.slot_rect(slot), sample.visual_bucket),
        terrain_vertex_colors(material, role, sample.visual_bucket),
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
    let half_width = tile_stride * 0.06;
    let y = display_surface_height(first).max(display_surface_height(second)) + 0.018;
    let positions = match edge {
        TerrainEdge::East => {
            let x = first.center_x + half;
            [
                [x - half_width, y, first.center_z - half],
                [x - half_width, y, first.center_z + half],
                [x + half_width, y, first.center_z + half],
                [x + half_width, y, first.center_z - half],
            ]
        }
        TerrainEdge::South => {
            let z = first.center_z + half;
            [
                [first.center_x - half, y, z - half_width],
                [first.center_x - half, y, z + half_width],
                [first.center_x + half, y, z + half_width],
                [first.center_x + half, y, z - half_width],
            ]
        }
        TerrainEdge::North | TerrainEdge::West => unreachable!("canonical interior edges only"),
    };
    let role = Fvr11TerrainSurfaceRole::Transition;
    let slot = terrain_atlas_slot(material, role);
    batches.entry((role, material)).or_default().push_quad(
        positions,
        [[0.0, 1.0, 0.0]; 4],
        atlas_uvs(atlas.slot_rect(slot), owner.visual_bucket),
        terrain_vertex_colors(material, role, owner.visual_bucket),
        owner.tile,
    );
    *vertices_by_tile.entry(owner.tile).or_default() += 4;
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

fn top_surface_normals(heights: [f32; 4], stride: f32) -> [[f32; 3]; 4] {
    let normal = |dx: f32, dz: f32| {
        Vec3::new(-dx / stride, 1.0, -dz / stride)
            .normalize()
            .to_array()
    };
    [
        normal(heights[3] - heights[0], heights[1] - heights[0]),
        normal(heights[2] - heights[1], heights[1] - heights[0]),
        normal(heights[2] - heights[1], heights[2] - heights[3]),
        normal(heights[3] - heights[0], heights[2] - heights[3]),
    ]
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

fn atlas_uvs(rect: TerrainAtlasUvRect, visual_bucket: u8) -> [[f32; 2]; 4] {
    let mut uvs = [
        [rect.min[0], rect.min[1]],
        [rect.min[0], rect.max[1]],
        [rect.max[0], rect.max[1]],
        [rect.max[0], rect.min[1]],
    ];
    uvs.rotate_left(usize::from(visual_bucket % 4));
    if visual_bucket & 4 != 0 {
        for uv in &mut uvs {
            uv[0] = rect.min[0] + rect.max[0] - uv[0];
        }
    }
    uvs
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
    visual_bucket: u8,
) -> [[f32; 4]; 4] {
    let base = match material {
        Fvr03ProductionVoxelMaterialKind::SafeGrass => [0.96, 1.00, 0.94],
        Fvr03ProductionVoxelMaterialKind::Soil => [1.00, 0.97, 0.92],
        Fvr03ProductionVoxelMaterialKind::Resource => [0.94, 1.00, 0.91],
        Fvr03ProductionVoxelMaterialKind::Hazard => [1.00, 0.94, 0.96],
        Fvr03ProductionVoxelMaterialKind::Decay => [0.96, 0.93, 0.90],
        Fvr03ProductionVoxelMaterialKind::Stone => [0.96, 0.98, 0.95],
        Fvr03ProductionVoxelMaterialKind::Water => [0.91, 1.00, 1.00],
        Fvr03ProductionVoxelMaterialKind::Sand => [1.00, 0.98, 0.91],
        _ => [1.0, 1.0, 1.0],
    };
    let role_shade = match role {
        Fvr11TerrainSurfaceRole::Top => 1.0,
        Fvr11TerrainSurfaceRole::Cliff => 0.74,
        Fvr11TerrainSurfaceRole::Transition => 0.88,
        Fvr11TerrainSurfaceRole::Water => 0.96,
    };
    let bucket = 0.95 + f32::from(visual_bucket % 5) * 0.022;
    [0.97, 1.025, 1.0, 0.945].map(|vertex| {
        let factor = role_shade * bucket * vertex;
        [
            (base[0] * factor).clamp(0.0, 1.0),
            (base[1] * factor).clamp(0.0, 1.0),
            (base[2] * factor).clamp(0.0, 1.0),
            1.0,
        ]
    })
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
    use bevy::prelude::Mesh;

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
        assert!(first.stats.max_vertices_per_source_tile <= 40);
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
}
