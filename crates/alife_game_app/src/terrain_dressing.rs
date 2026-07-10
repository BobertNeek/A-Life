//! Deterministic biome-aware dressing for the display-only terrain layer.

use std::collections::{BTreeMap, BTreeSet};

use alife_world::VoxelTileCoord;
use bevy::{
    asset::RenderAssetUsages,
    color::LinearRgba,
    mesh::Indices,
    prelude::{App, Assets, Color, Handle, Mesh, StandardMaterial, Vec3},
    render::render_resource::PrimitiveTopology,
};

use crate::{Fvr03ProductionVoxelMaterialKind, Fvr07ProductionDressingKind};

pub(crate) const PRODUCTION_DRESSING_KINDS: [Fvr07ProductionDressingKind; 11] = [
    Fvr07ProductionDressingKind::LeafPatch,
    Fvr07ProductionDressingKind::MushroomCluster,
    Fvr07ProductionDressingKind::PebbleCluster,
    Fvr07ProductionDressingKind::NestMarker,
    Fvr07ProductionDressingKind::FoodResource,
    Fvr07ProductionDressingKind::CorpseMarker,
    Fvr07ProductionDressingKind::FlowerPatch,
    Fvr07ProductionDressingKind::ReedCluster,
    Fvr07ProductionDressingKind::LichenRock,
    Fvr07ProductionDressingKind::HazardFungus,
    Fvr07ProductionDressingKind::DeadLeafPatch,
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TerrainDressingTile {
    pub tile: VoxelTileCoord,
    pub material: Fvr03ProductionVoxelMaterialKind,
    pub height: f32,
    pub resource_bias: f32,
    pub hazard_pressure: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ProductionTerrainDressingSpawn {
    pub kind: Fvr07ProductionDressingKind,
    pub tile: VoxelTileCoord,
    pub translation: Vec3,
    pub scale: Vec3,
    pub yaw_radians: f32,
    pub cluster_id: u32,
    pub display_only: bool,
    pub no_renderer_authority_over_actions_or_cognition: bool,
}

pub(crate) struct TerrainDressingLibrary {
    meshes: BTreeMap<Fvr07ProductionDressingKind, Handle<Mesh>>,
    materials: BTreeMap<Fvr07ProductionDressingKind, Handle<StandardMaterial>>,
}

impl TerrainDressingLibrary {
    pub fn mesh(&self, kind: Fvr07ProductionDressingKind) -> Handle<Mesh> {
        self.meshes
            .get(&kind)
            .unwrap_or_else(|| panic!("missing terrain dressing mesh {kind:?}"))
            .clone()
    }

    pub fn material(&self, kind: Fvr07ProductionDressingKind) -> Handle<StandardMaterial> {
        self.materials
            .get(&kind)
            .unwrap_or_else(|| panic!("missing terrain dressing material {kind:?}"))
            .clone()
    }
}

pub(crate) fn plan_production_terrain_dressing(
    tiles: &BTreeMap<VoxelTileCoord, TerrainDressingTile>,
    occupied_tiles: &BTreeSet<VoxelTileCoord>,
    cap: usize,
    tile_stride: u16,
    minimum_floor: bool,
) -> Vec<ProductionTerrainDressingSpawn> {
    if cap == 0 || tiles.is_empty() {
        return Vec::new();
    }
    let stride = i32::from(tile_stride.max(1));
    let bucket_size = stride * 2;
    let mut candidates = tiles
        .values()
        .filter(|tile| !occupied_tiles.contains(&tile.tile))
        .copied()
        .collect::<Vec<_>>();
    candidates.sort_by_key(|tile| {
        (
            nearest_occupied_distance_squared(tile.tile, occupied_tiles),
            dressing_hash(tile.tile),
            tile.tile.x,
            tile.tile.z,
        )
    });

    let mut anchors = BTreeSet::<(i32, i32)>::new();
    let mut spawns = Vec::with_capacity(cap);
    let required = [
        Fvr07ProductionDressingKind::LeafPatch,
        Fvr07ProductionDressingKind::FlowerPatch,
        Fvr07ProductionDressingKind::FoodResource,
        Fvr07ProductionDressingKind::MushroomCluster,
        Fvr07ProductionDressingKind::PebbleCluster,
        Fvr07ProductionDressingKind::LichenRock,
        Fvr07ProductionDressingKind::ReedCluster,
        Fvr07ProductionDressingKind::HazardFungus,
        Fvr07ProductionDressingKind::DeadLeafPatch,
        Fvr07ProductionDressingKind::NestMarker,
    ];
    let mut cluster_id = 1_u32;
    for kind in required {
        let Some(tile) = candidates.iter().copied().find(|tile| {
            kind_is_compatible(kind, tile, tiles, stride)
                && !anchors.contains(&anchor_bucket(tile.tile, bucket_size))
        }) else {
            continue;
        };
        anchors.insert(anchor_bucket(tile.tile, bucket_size));
        append_cluster(
            &mut spawns,
            cap,
            kind,
            &tile,
            tile_stride,
            minimum_floor,
            cluster_id,
        );
        cluster_id += 1;
    }

    for tile in candidates {
        if spawns.len() >= cap {
            break;
        }
        let bucket = anchor_bucket(tile.tile, bucket_size);
        if anchors.contains(&bucket) {
            continue;
        }
        let kind = biome_dressing_kind(tile);
        if !kind_is_compatible(kind, &tile, tiles, stride) {
            continue;
        }
        anchors.insert(bucket);
        append_cluster(
            &mut spawns,
            cap,
            kind,
            &tile,
            tile_stride,
            minimum_floor,
            cluster_id,
        );
        cluster_id += 1;
    }
    spawns
}

pub(crate) fn create_terrain_dressing_library(app: &mut App) -> TerrainDressingLibrary {
    let meshes = PRODUCTION_DRESSING_KINDS
        .into_iter()
        .map(|kind| {
            let handle = app
                .world_mut()
                .resource_mut::<Assets<Mesh>>()
                .add(build_dressing_mesh(kind));
            (kind, handle)
        })
        .collect();
    let materials = PRODUCTION_DRESSING_KINDS
        .into_iter()
        .map(|kind| {
            let handle = app
                .world_mut()
                .resource_mut::<Assets<StandardMaterial>>()
                .add(dressing_material(kind));
            (kind, handle)
        })
        .collect();
    TerrainDressingLibrary { meshes, materials }
}

fn append_cluster(
    spawns: &mut Vec<ProductionTerrainDressingSpawn>,
    cap: usize,
    kind: Fvr07ProductionDressingKind,
    tile: &TerrainDressingTile,
    tile_stride: u16,
    minimum_floor: bool,
    cluster_id: u32,
) {
    let count = cluster_child_count(kind);
    let footprint = f32::from(tile_stride.max(1));
    let hash = dressing_hash(tile.tile);
    for child in 0..count {
        if spawns.len() >= cap {
            break;
        }
        let angle = ((hash.rotate_left(child * 5) & 0xffff) as f32 / 65535.0
            + child as f32 * 0.618_034)
            * std::f32::consts::TAU;
        let radius = if count == 1 {
            0.0
        } else {
            footprint * (0.12 + 0.08 * f32::from((child % 3) as u8))
        };
        let profile_scale = if minimum_floor { 0.90 } else { 1.0 };
        let child_scale = 0.91 + f32::from(((hash >> (child % 12)) & 0x7) as u8) * 0.025;
        let scale = dressing_scale(kind) * profile_scale * child_scale;
        spawns.push(ProductionTerrainDressingSpawn {
            kind,
            tile: tile.tile,
            translation: Vec3::new(
                tile.tile.x as f32 + 0.5 + angle.cos() * radius,
                tile.height + dressing_y_offset(kind),
                tile.tile.z as f32 + 0.5 + angle.sin() * radius,
            ),
            scale,
            yaw_radians: angle + f32::from((hash & 0xff) as u8) / 255.0,
            cluster_id,
            display_only: true,
            no_renderer_authority_over_actions_or_cognition: true,
        });
    }
}

fn cluster_child_count(kind: Fvr07ProductionDressingKind) -> u32 {
    match kind {
        Fvr07ProductionDressingKind::LeafPatch
        | Fvr07ProductionDressingKind::FlowerPatch
        | Fvr07ProductionDressingKind::ReedCluster
        | Fvr07ProductionDressingKind::HazardFungus
        | Fvr07ProductionDressingKind::DeadLeafPatch => 4,
        Fvr07ProductionDressingKind::MushroomCluster
        | Fvr07ProductionDressingKind::FoodResource => 3,
        Fvr07ProductionDressingKind::PebbleCluster | Fvr07ProductionDressingKind::LichenRock => 2,
        Fvr07ProductionDressingKind::NestMarker | Fvr07ProductionDressingKind::CorpseMarker => 1,
    }
}

fn dressing_scale(kind: Fvr07ProductionDressingKind) -> Vec3 {
    match kind {
        Fvr07ProductionDressingKind::LeafPatch => Vec3::new(1.12, 1.20, 1.08),
        Fvr07ProductionDressingKind::MushroomCluster => Vec3::new(1.05, 1.16, 1.05),
        Fvr07ProductionDressingKind::PebbleCluster => Vec3::new(0.82, 0.70, 0.82),
        Fvr07ProductionDressingKind::NestMarker => Vec3::new(0.94, 0.68, 0.94),
        Fvr07ProductionDressingKind::FoodResource => Vec3::new(1.04, 1.18, 1.04),
        Fvr07ProductionDressingKind::CorpseMarker => Vec3::new(0.90, 0.52, 0.86),
        Fvr07ProductionDressingKind::FlowerPatch => Vec3::new(1.08, 1.22, 1.08),
        Fvr07ProductionDressingKind::ReedCluster => Vec3::new(1.05, 1.28, 1.05),
        Fvr07ProductionDressingKind::LichenRock => Vec3::new(0.90, 0.78, 0.90),
        Fvr07ProductionDressingKind::HazardFungus => Vec3::new(1.05, 1.18, 1.05),
        Fvr07ProductionDressingKind::DeadLeafPatch => Vec3::new(1.00, 0.64, 1.00),
    }
}

fn dressing_y_offset(kind: Fvr07ProductionDressingKind) -> f32 {
    match kind {
        Fvr07ProductionDressingKind::ReedCluster => 0.0,
        Fvr07ProductionDressingKind::PebbleCluster
        | Fvr07ProductionDressingKind::LichenRock
        | Fvr07ProductionDressingKind::NestMarker
        | Fvr07ProductionDressingKind::CorpseMarker
        | Fvr07ProductionDressingKind::DeadLeafPatch => 0.02,
        _ => 0.035,
    }
}

fn kind_is_compatible(
    kind: Fvr07ProductionDressingKind,
    tile: &TerrainDressingTile,
    tiles: &BTreeMap<VoxelTileCoord, TerrainDressingTile>,
    stride: i32,
) -> bool {
    match kind {
        Fvr07ProductionDressingKind::LeafPatch | Fvr07ProductionDressingKind::FlowerPatch => {
            matches!(
                tile.material,
                Fvr03ProductionVoxelMaterialKind::SafeGrass
                    | Fvr03ProductionVoxelMaterialKind::Resource
            )
        }
        Fvr07ProductionDressingKind::MushroomCluster => matches!(
            tile.material,
            Fvr03ProductionVoxelMaterialKind::Resource | Fvr03ProductionVoxelMaterialKind::Decay
        ),
        Fvr07ProductionDressingKind::FoodResource => {
            tile.material == Fvr03ProductionVoxelMaterialKind::Resource
                || tile.resource_bias >= 0.38
        }
        Fvr07ProductionDressingKind::PebbleCluster => matches!(
            tile.material,
            Fvr03ProductionVoxelMaterialKind::Stone
                | Fvr03ProductionVoxelMaterialKind::Sand
                | Fvr03ProductionVoxelMaterialKind::Soil
        ),
        Fvr07ProductionDressingKind::NestMarker => matches!(
            tile.material,
            Fvr03ProductionVoxelMaterialKind::SafeGrass
                | Fvr03ProductionVoxelMaterialKind::Resource
                | Fvr03ProductionVoxelMaterialKind::Soil
        ),
        Fvr07ProductionDressingKind::CorpseMarker => matches!(
            tile.material,
            Fvr03ProductionVoxelMaterialKind::Decay | Fvr03ProductionVoxelMaterialKind::Hazard
        ),
        Fvr07ProductionDressingKind::ReedCluster => {
            tile.material == Fvr03ProductionVoxelMaterialKind::Water
                || has_neighbor_material(
                    tile.tile,
                    Fvr03ProductionVoxelMaterialKind::Water,
                    tiles,
                    stride,
                )
        }
        Fvr07ProductionDressingKind::LichenRock => {
            tile.material == Fvr03ProductionVoxelMaterialKind::Stone
        }
        Fvr07ProductionDressingKind::HazardFungus => {
            tile.material == Fvr03ProductionVoxelMaterialKind::Hazard
                || tile.hazard_pressure >= 0.38
        }
        Fvr07ProductionDressingKind::DeadLeafPatch => matches!(
            tile.material,
            Fvr03ProductionVoxelMaterialKind::Decay
                | Fvr03ProductionVoxelMaterialKind::Soil
                | Fvr03ProductionVoxelMaterialKind::Sand
        ),
    }
}

fn biome_dressing_kind(tile: TerrainDressingTile) -> Fvr07ProductionDressingKind {
    let hash = dressing_hash(tile.tile);
    match tile.material {
        Fvr03ProductionVoxelMaterialKind::SafeGrass => {
            if hash % 3 == 0 {
                Fvr07ProductionDressingKind::FlowerPatch
            } else {
                Fvr07ProductionDressingKind::LeafPatch
            }
        }
        Fvr03ProductionVoxelMaterialKind::Resource => match hash % 3 {
            0 => Fvr07ProductionDressingKind::FoodResource,
            1 => Fvr07ProductionDressingKind::FlowerPatch,
            _ => Fvr07ProductionDressingKind::MushroomCluster,
        },
        Fvr03ProductionVoxelMaterialKind::Hazard => Fvr07ProductionDressingKind::HazardFungus,
        Fvr03ProductionVoxelMaterialKind::Decay => {
            if hash % 3 == 0 {
                Fvr07ProductionDressingKind::MushroomCluster
            } else {
                Fvr07ProductionDressingKind::DeadLeafPatch
            }
        }
        Fvr03ProductionVoxelMaterialKind::Stone => Fvr07ProductionDressingKind::LichenRock,
        Fvr03ProductionVoxelMaterialKind::Water => Fvr07ProductionDressingKind::ReedCluster,
        Fvr03ProductionVoxelMaterialKind::Soil | Fvr03ProductionVoxelMaterialKind::Sand => {
            if hash % 4 == 0 {
                Fvr07ProductionDressingKind::DeadLeafPatch
            } else {
                Fvr07ProductionDressingKind::PebbleCluster
            }
        }
        _ => Fvr07ProductionDressingKind::LeafPatch,
    }
}

fn has_neighbor_material(
    tile: VoxelTileCoord,
    material: Fvr03ProductionVoxelMaterialKind,
    tiles: &BTreeMap<VoxelTileCoord, TerrainDressingTile>,
    stride: i32,
) -> bool {
    [(0, -stride), (stride, 0), (0, stride), (-stride, 0)]
        .into_iter()
        .any(|(dx, dz)| {
            tiles
                .get(&VoxelTileCoord::new(tile.x + dx, tile.z + dz))
                .is_some_and(|neighbor| neighbor.material == material)
        })
}

fn nearest_occupied_distance_squared(
    tile: VoxelTileCoord,
    occupied_tiles: &BTreeSet<VoxelTileCoord>,
) -> i64 {
    occupied_tiles
        .iter()
        .map(|occupied| {
            let dx = i64::from(tile.x - occupied.x);
            let dz = i64::from(tile.z - occupied.z);
            dx * dx + dz * dz
        })
        .min()
        .unwrap_or_default()
}

fn anchor_bucket(tile: VoxelTileCoord, bucket_size: i32) -> (i32, i32) {
    (
        tile.x.div_euclid(bucket_size),
        tile.z.div_euclid(bucket_size),
    )
}

fn dressing_hash(tile: VoxelTileCoord) -> u32 {
    let mut value = (tile.x as u32).wrapping_mul(0x9e37_79b9)
        ^ (tile.z as u32).wrapping_mul(0x85eb_ca6b)
        ^ 0xf711_5eed;
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^ (value >> 15)
}

fn dressing_material(kind: Fvr07ProductionDressingKind) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: match kind {
            Fvr07ProductionDressingKind::HazardFungus => 0.68,
            Fvr07ProductionDressingKind::MushroomCluster
            | Fvr07ProductionDressingKind::FoodResource => 0.76,
            Fvr07ProductionDressingKind::LichenRock
            | Fvr07ProductionDressingKind::PebbleCluster => 0.92,
            _ => 0.86,
        },
        metallic: 0.0,
        emissive: if kind == Fvr07ProductionDressingKind::HazardFungus {
            LinearRgba::rgb(0.08, 0.012, 0.018)
        } else {
            LinearRgba::BLACK
        },
        cull_mode: None,
        unlit: false,
        ..Default::default()
    }
}

fn build_dressing_mesh(kind: Fvr07ProductionDressingKind) -> Mesh {
    let mut builder = DressingMeshBuilder::default();
    match kind {
        Fvr07ProductionDressingKind::LeafPatch => append_leaf_patch(&mut builder, false),
        Fvr07ProductionDressingKind::FlowerPatch => append_flower_patch(&mut builder),
        Fvr07ProductionDressingKind::MushroomCluster => {
            append_mushroom_cluster(&mut builder, false)
        }
        Fvr07ProductionDressingKind::HazardFungus => append_mushroom_cluster(&mut builder, true),
        Fvr07ProductionDressingKind::PebbleCluster => append_rock_cluster(&mut builder, false),
        Fvr07ProductionDressingKind::LichenRock => append_rock_cluster(&mut builder, true),
        Fvr07ProductionDressingKind::ReedCluster => append_reed_cluster(&mut builder),
        Fvr07ProductionDressingKind::DeadLeafPatch => append_dead_leaf_patch(&mut builder),
        Fvr07ProductionDressingKind::NestMarker => append_nest(&mut builder),
        Fvr07ProductionDressingKind::FoodResource => append_food_resource(&mut builder),
        Fvr07ProductionDressingKind::CorpseMarker => append_corpse_marker(&mut builder),
    }
    builder.finish()
}

#[derive(Default)]
struct DressingMeshBuilder {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl DressingMeshBuilder {
    fn quad(&mut self, quad: [Vec3; 4], color: [f32; 4], double_sided: bool) {
        self.quad_face(quad, color);
        if double_sided {
            self.quad_face([quad[3], quad[2], quad[1], quad[0]], color);
        }
    }

    fn quad_face(&mut self, quad: [Vec3; 4], color: [f32; 4]) {
        let normal = (quad[1] - quad[0])
            .cross(quad[2] - quad[0])
            .normalize_or_zero()
            .to_array();
        let base = self.positions.len() as u32;
        self.positions
            .extend(quad.map(|position| position.to_array()));
        self.normals.extend([normal; 4]);
        self.uvs
            .extend([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        self.colors.extend([color; 4]);
        self.indices
            .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    fn triangle(&mut self, triangle: [Vec3; 3], color: [f32; 4]) {
        let normal = (triangle[1] - triangle[0])
            .cross(triangle[2] - triangle[0])
            .normalize_or_zero()
            .to_array();
        let base = self.positions.len() as u32;
        self.positions
            .extend(triangle.map(|position| position.to_array()));
        self.normals.extend([normal; 3]);
        self.uvs.extend([[0.5, 1.0], [0.0, 0.0], [1.0, 0.0]]);
        self.colors.extend([color; 3]);
        self.indices.extend([base, base + 1, base + 2]);
    }

    fn tapered_prism(
        &mut self,
        center: Vec3,
        height: f32,
        base_radius: f32,
        top_radius: f32,
        color: [f32; 4],
    ) {
        let bottom = (0..4)
            .map(|index| {
                let angle =
                    std::f32::consts::FRAC_PI_4 + index as f32 * std::f32::consts::FRAC_PI_2;
                center + Vec3::new(angle.cos() * base_radius, 0.0, angle.sin() * base_radius)
            })
            .collect::<Vec<_>>();
        let top = (0..4)
            .map(|index| {
                let angle =
                    std::f32::consts::FRAC_PI_4 + index as f32 * std::f32::consts::FRAC_PI_2;
                center + Vec3::new(angle.cos() * top_radius, height, angle.sin() * top_radius)
            })
            .collect::<Vec<_>>();
        for index in 0..4 {
            let next = (index + 1) % 4;
            self.quad(
                [bottom[next], bottom[index], top[index], top[next]],
                color,
                false,
            );
        }
        let top_center = center + Vec3::Y * height;
        for index in 0..4 {
            let next = (index + 1) % 4;
            self.triangle([top_center, top[index], top[next]], color);
        }
    }

    fn blade(&mut self, center: Vec3, height: f32, width: f32, yaw: f32, color: [f32; 4]) {
        let direction = Vec3::new(yaw.sin(), 0.0, yaw.cos());
        let side = Vec3::new(direction.z, 0.0, -direction.x);
        let lean = direction * height * 0.12;
        self.quad(
            [
                center - side * width,
                center + side * width,
                center + lean + Vec3::Y * height + side * width * 0.12,
                center + lean + Vec3::Y * height - side * width * 0.12,
            ],
            color,
            true,
        );
    }

    fn leaf(
        &mut self,
        center: Vec3,
        length: f32,
        width: f32,
        yaw: f32,
        rise: f32,
        color: [f32; 4],
    ) {
        let direction = Vec3::new(yaw.sin(), 0.0, yaw.cos());
        let side = Vec3::new(direction.z, 0.0, -direction.x);
        self.quad(
            [
                center - direction * length * 0.45,
                center + side * width,
                center + direction * length * 0.55 + Vec3::Y * rise,
                center - side * width,
            ],
            color,
            true,
        );
    }

    fn mushroom(&mut self, center: Vec3, height: f32, radius: f32, stem: [f32; 4], cap: [f32; 4]) {
        self.tapered_prism(center, height, radius * 0.22, radius * 0.14, stem);
        let top_center = center + Vec3::Y * (height + radius * 0.24);
        let under_center = center + Vec3::Y * (height - radius * 0.06);
        let ring = (0..8)
            .map(|index| {
                let angle = index as f32 * std::f32::consts::TAU / 8.0;
                center + Vec3::new(angle.cos() * radius, height, angle.sin() * radius)
            })
            .collect::<Vec<_>>();
        for index in 0..8 {
            let next = (index + 1) % 8;
            self.triangle([top_center, ring[index], ring[next]], cap);
            self.triangle([under_center, ring[next], ring[index]], stem);
        }
    }

    fn rock(
        &mut self,
        center: Vec3,
        radius: f32,
        height: f32,
        color: [f32; 4],
        lichen: Option<[f32; 4]>,
    ) {
        let ring = (0..6)
            .map(|index| {
                let angle = index as f32 * std::f32::consts::TAU / 6.0;
                let variation = 0.86 + (index % 3) as f32 * 0.07;
                center
                    + Vec3::new(
                        angle.cos() * radius * variation,
                        0.0,
                        angle.sin() * radius / variation,
                    )
            })
            .collect::<Vec<_>>();
        let top = ring
            .iter()
            .map(|point| center + (*point - center) * 0.66 + Vec3::Y * height)
            .collect::<Vec<_>>();
        for index in 0..6 {
            let next = (index + 1) % 6;
            self.quad(
                [ring[next], ring[index], top[index], top[next]],
                color,
                false,
            );
        }
        let top_center = center + Vec3::Y * (height * 1.08);
        for index in 0..6 {
            let next = (index + 1) % 6;
            self.triangle([top_center, top[index], top[next]], lichen.unwrap_or(color));
        }
    }

    fn finish(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }
}

fn append_leaf_patch(builder: &mut DressingMeshBuilder, dry: bool) {
    let stem = if dry {
        [0.34, 0.24, 0.10, 1.0]
    } else {
        [0.18, 0.46, 0.08, 1.0]
    };
    let leaf = if dry {
        [0.58, 0.36, 0.10, 1.0]
    } else {
        [0.42, 0.72, 0.12, 1.0]
    };
    for index in 0..6 {
        let angle = index as f32 * 1.03;
        let center = Vec3::new(angle.cos() * 0.22, 0.0, angle.sin() * 0.22);
        builder.blade(center, 0.44 + index as f32 * 0.035, 0.045, angle, stem);
        builder.leaf(
            center + Vec3::Y * (0.28 + index as f32 * 0.02),
            0.38,
            0.12,
            angle,
            0.10,
            leaf,
        );
    }
}

fn append_flower_patch(builder: &mut DressingMeshBuilder) {
    for index in 0..4 {
        let angle = index as f32 * 1.57 + 0.35;
        let center = Vec3::new(angle.cos() * 0.24, 0.0, angle.sin() * 0.24);
        let height = 0.46 + index as f32 * 0.055;
        builder.tapered_prism(center, height, 0.035, 0.018, [0.20, 0.48, 0.08, 1.0]);
        for petal in 0..5 {
            builder.leaf(
                center + Vec3::Y * height,
                0.20,
                0.065,
                petal as f32 * std::f32::consts::TAU / 5.0,
                0.025,
                if index % 2 == 0 {
                    [0.85, 0.24, 0.56, 1.0]
                } else {
                    [0.35, 0.42, 0.92, 1.0]
                },
            );
        }
        builder.rock(
            center + Vec3::Y * height,
            0.045,
            0.035,
            [0.98, 0.76, 0.10, 1.0],
            None,
        );
    }
}

fn append_mushroom_cluster(builder: &mut DressingMeshBuilder, hazard: bool) {
    for (index, (x, z, height, radius)) in [
        (-0.22, -0.08, 0.38, 0.20),
        (0.08, 0.16, 0.54, 0.25),
        (0.28, -0.18, 0.32, 0.17),
        (-0.05, -0.28, 0.26, 0.14),
    ]
    .into_iter()
    .enumerate()
    {
        builder.mushroom(
            Vec3::new(x, 0.0, z),
            height,
            radius,
            if hazard {
                [0.32, 0.06, 0.08, 1.0]
            } else {
                [0.72, 0.58, 0.40, 1.0]
            },
            if hazard {
                if index % 2 == 0 {
                    [0.62, 0.04, 0.08, 1.0]
                } else {
                    [0.92, 0.16, 0.10, 1.0]
                }
            } else {
                [0.52, 0.24, 0.68, 1.0]
            },
        );
    }
}

fn append_rock_cluster(builder: &mut DressingMeshBuilder, lichen: bool) {
    for (x, z, radius, height) in [
        (-0.24, -0.10, 0.22, 0.16),
        (0.08, 0.12, 0.28, 0.22),
        (0.30, -0.18, 0.17, 0.13),
        (-0.04, -0.30, 0.19, 0.14),
    ] {
        builder.rock(
            Vec3::new(x, 0.0, z),
            radius,
            height,
            [0.35, 0.37, 0.34, 1.0],
            lichen.then_some([0.42, 0.58, 0.14, 1.0]),
        );
    }
}

fn append_reed_cluster(builder: &mut DressingMeshBuilder) {
    for index in 0..8 {
        let angle = index as f32 * 0.88;
        let radius = 0.10 + (index % 3) as f32 * 0.08;
        let center = Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
        builder.blade(
            center,
            0.58 + (index % 4) as f32 * 0.10,
            0.035,
            angle + 0.2,
            [0.18, 0.48, 0.16, 1.0],
        );
        if index % 2 == 0 {
            builder.tapered_prism(
                center + Vec3::Y * (0.48 + (index % 4) as f32 * 0.10),
                0.20,
                0.045,
                0.025,
                [0.58, 0.30, 0.10, 1.0],
            );
        }
    }
}

fn append_dead_leaf_patch(builder: &mut DressingMeshBuilder) {
    for index in 0..8 {
        let angle = index as f32 * 0.83;
        let radius = 0.10 + (index % 4) as f32 * 0.08;
        builder.leaf(
            Vec3::new(
                angle.cos() * radius,
                0.02 + (index % 2) as f32 * 0.008,
                angle.sin() * radius,
            ),
            0.32,
            0.11,
            angle,
            0.02,
            if index % 3 == 0 {
                [0.62, 0.28, 0.08, 1.0]
            } else {
                [0.34, 0.20, 0.07, 1.0]
            },
        );
    }
}

fn append_nest(builder: &mut DressingMeshBuilder) {
    for index in 0..10 {
        let angle = index as f32 * std::f32::consts::TAU / 10.0;
        builder.rock(
            Vec3::new(angle.cos() * 0.30, 0.0, angle.sin() * 0.30),
            0.15,
            0.10 + (index % 2) as f32 * 0.04,
            [0.42, 0.24, 0.08, 1.0],
            None,
        );
    }
}

fn append_food_resource(builder: &mut DressingMeshBuilder) {
    builder.tapered_prism(Vec3::ZERO, 0.58, 0.055, 0.025, [0.16, 0.42, 0.08, 1.0]);
    for index in 0..6 {
        let angle = index as f32 * std::f32::consts::TAU / 6.0;
        builder.leaf(
            Vec3::Y * (0.34 + (index % 2) as f32 * 0.10),
            0.38,
            0.13,
            angle,
            0.10,
            [0.38, 0.68, 0.10, 1.0],
        );
        builder.rock(
            Vec3::new(
                angle.cos() * 0.22,
                0.56 + (index % 2) as f32 * 0.08,
                angle.sin() * 0.22,
            ),
            0.09,
            0.11,
            [0.94, 0.62, 0.08, 1.0],
            None,
        );
    }
}

fn append_corpse_marker(builder: &mut DressingMeshBuilder) {
    for index in 0..5 {
        let angle = index as f32 * 1.17;
        builder.leaf(
            Vec3::new(angle.cos() * 0.18, 0.025, angle.sin() * 0.18),
            0.42,
            0.07,
            angle,
            0.01,
            if index % 2 == 0 {
                [0.68, 0.62, 0.48, 1.0]
            } else {
                [0.28, 0.18, 0.24, 1.0]
            },
        );
    }
    builder.rock(Vec3::ZERO, 0.18, 0.10, [0.30, 0.18, 0.24, 1.0], None);
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use alife_world::VoxelTileCoord;

    use super::*;
    use crate::{Fvr03ProductionVoxelMaterialKind, Fvr07ProductionDressingKind};

    fn fixed_biome_map() -> BTreeMap<VoxelTileCoord, TerrainDressingTile> {
        let materials = [
            Fvr03ProductionVoxelMaterialKind::SafeGrass,
            Fvr03ProductionVoxelMaterialKind::Resource,
            Fvr03ProductionVoxelMaterialKind::Stone,
            Fvr03ProductionVoxelMaterialKind::Water,
            Fvr03ProductionVoxelMaterialKind::Hazard,
            Fvr03ProductionVoxelMaterialKind::Decay,
            Fvr03ProductionVoxelMaterialKind::Soil,
            Fvr03ProductionVoxelMaterialKind::Sand,
        ];
        let mut tiles = BTreeMap::new();
        for z in 0..6 {
            for x in 0..6 {
                let tile = VoxelTileCoord::new(x * 2, z * 2);
                let material = materials[(x + z * 3) as usize % materials.len()];
                tiles.insert(
                    tile,
                    TerrainDressingTile {
                        tile,
                        material,
                        height: 0.75 + ((x + z) % 3) as f32 * 0.25,
                        resource_bias: if material == Fvr03ProductionVoxelMaterialKind::Resource {
                            0.8
                        } else {
                            0.2
                        },
                        hazard_pressure: if material == Fvr03ProductionVoxelMaterialKind::Hazard {
                            0.8
                        } else {
                            0.1
                        },
                    },
                );
            }
        }
        tiles
    }

    #[test]
    fn biome_cluster_plan_is_deterministic_bounded_and_unoccupied() {
        let tiles = fixed_biome_map();
        let occupied = [VoxelTileCoord::new(0, 0), VoxelTileCoord::new(4, 4)]
            .into_iter()
            .collect::<BTreeSet<_>>();
        let first = plan_production_terrain_dressing(&tiles, &occupied, 64, 2, false);
        let second = plan_production_terrain_dressing(&tiles, &occupied, 64, 2, false);

        assert_eq!(first, second);
        assert!(!first.is_empty());
        assert!(first.len() <= 64);
        assert!(first.iter().all(|spawn| !occupied.contains(&spawn.tile)));
        assert!(first.iter().all(|spawn| spawn.display_only));
        assert!(first
            .iter()
            .all(|spawn| spawn.no_renderer_authority_over_actions_or_cognition));
        let kinds = first
            .iter()
            .map(|spawn| spawn.kind)
            .collect::<BTreeSet<_>>();
        for required in [
            Fvr07ProductionDressingKind::ReedCluster,
            Fvr07ProductionDressingKind::HazardFungus,
            Fvr07ProductionDressingKind::LichenRock,
            Fvr07ProductionDressingKind::FlowerPatch,
            Fvr07ProductionDressingKind::DeadLeafPatch,
        ] {
            assert!(kinds.contains(&required), "missing {required:?}");
        }
    }
}
