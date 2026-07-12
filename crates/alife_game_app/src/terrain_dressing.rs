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

pub(crate) const PRODUCTION_DRESSING_KINDS: [Fvr07ProductionDressingKind; 14] = [
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
    Fvr07ProductionDressingKind::AlienFern,
    Fvr07ProductionDressingKind::CrimsonSpire,
    Fvr07ProductionDressingKind::GlowBulbCluster,
];

const MINIMUM_CLUSTER_PRIORITY: [Fvr07ProductionDressingKind; 17] = [
    Fvr07ProductionDressingKind::ReedCluster,
    Fvr07ProductionDressingKind::HazardFungus,
    Fvr07ProductionDressingKind::LichenRock,
    Fvr07ProductionDressingKind::FlowerPatch,
    Fvr07ProductionDressingKind::DeadLeafPatch,
    Fvr07ProductionDressingKind::AlienFern,
    Fvr07ProductionDressingKind::CrimsonSpire,
    Fvr07ProductionDressingKind::GlowBulbCluster,
    Fvr07ProductionDressingKind::HazardFungus,
    Fvr07ProductionDressingKind::LeafPatch,
    Fvr07ProductionDressingKind::FlowerPatch,
    Fvr07ProductionDressingKind::MushroomCluster,
    Fvr07ProductionDressingKind::FoodResource,
    Fvr07ProductionDressingKind::PebbleCluster,
    Fvr07ProductionDressingKind::NestMarker,
    Fvr07ProductionDressingKind::LeafPatch,
    Fvr07ProductionDressingKind::HazardFungus,
];

const COMFORT_CLUSTER_PRIORITY: [Fvr07ProductionDressingKind; 33] = [
    Fvr07ProductionDressingKind::ReedCluster,
    Fvr07ProductionDressingKind::HazardFungus,
    Fvr07ProductionDressingKind::LichenRock,
    Fvr07ProductionDressingKind::FlowerPatch,
    Fvr07ProductionDressingKind::DeadLeafPatch,
    Fvr07ProductionDressingKind::HazardFungus,
    Fvr07ProductionDressingKind::CrimsonSpire,
    Fvr07ProductionDressingKind::GlowBulbCluster,
    Fvr07ProductionDressingKind::AlienFern,
    Fvr07ProductionDressingKind::CrimsonSpire,
    Fvr07ProductionDressingKind::GlowBulbCluster,
    Fvr07ProductionDressingKind::AlienFern,
    Fvr07ProductionDressingKind::LeafPatch,
    Fvr07ProductionDressingKind::FlowerPatch,
    Fvr07ProductionDressingKind::FoodResource,
    Fvr07ProductionDressingKind::MushroomCluster,
    Fvr07ProductionDressingKind::PebbleCluster,
    Fvr07ProductionDressingKind::NestMarker,
    Fvr07ProductionDressingKind::HazardFungus,
    Fvr07ProductionDressingKind::HazardFungus,
    Fvr07ProductionDressingKind::HazardFungus,
    Fvr07ProductionDressingKind::LeafPatch,
    Fvr07ProductionDressingKind::LeafPatch,
    Fvr07ProductionDressingKind::LeafPatch,
    Fvr07ProductionDressingKind::FlowerPatch,
    Fvr07ProductionDressingKind::FlowerPatch,
    Fvr07ProductionDressingKind::FoodResource,
    Fvr07ProductionDressingKind::MushroomCluster,
    Fvr07ProductionDressingKind::ReedCluster,
    Fvr07ProductionDressingKind::ReedCluster,
    Fvr07ProductionDressingKind::LichenRock,
    Fvr07ProductionDressingKind::DeadLeafPatch,
    Fvr07ProductionDressingKind::FlowerPatch,
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
    let priority = if minimum_floor {
        MINIMUM_CLUSTER_PRIORITY.as_slice()
    } else {
        COMFORT_CLUSTER_PRIORITY.as_slice()
    };
    let mut cluster_id = 1_u32;
    for &kind in priority {
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
            occupied_tiles,
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
            occupied_tiles,
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
    occupied_tiles: &BTreeSet<VoxelTileCoord>,
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
            footprint * (0.15 + 0.105 * f32::from((child % 3) as u8))
        };
        let profile_scale = if minimum_floor { 0.92 } else { 1.0 };
        let child_scale = 0.91 + f32::from(((hash >> (child % 12)) & 0x7) as u8) * 0.025;
        let axis_jitter = f32::from(((hash.rotate_right(child * 3) >> 8) & 0x3) as u8) * 0.012;
        let silhouette_scale = if child % 2 == 0 {
            Vec3::new(1.16 + axis_jitter, 0.96, 0.86 - axis_jitter * 0.5)
        } else {
            Vec3::new(0.86 - axis_jitter * 0.5, 1.08, 1.16 + axis_jitter)
        };
        let scale = dressing_scale(kind) * profile_scale * child_scale * silhouette_scale;
        let translation = Vec3::new(
            tile.tile.x as f32 + 0.5 + angle.cos() * radius,
            tile.height + dressing_y_offset(kind),
            tile.tile.z as f32 + 0.5 + angle.sin() * radius,
        );
        if !dressing_child_clears_occupied(translation, occupied_tiles) {
            continue;
        }
        spawns.push(ProductionTerrainDressingSpawn {
            kind,
            tile: tile.tile,
            translation,
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
        | Fvr07ProductionDressingKind::DeadLeafPatch
        | Fvr07ProductionDressingKind::MushroomCluster
        | Fvr07ProductionDressingKind::FoodResource
        | Fvr07ProductionDressingKind::AlienFern
        | Fvr07ProductionDressingKind::CrimsonSpire
        | Fvr07ProductionDressingKind::GlowBulbCluster => 2,
        Fvr07ProductionDressingKind::PebbleCluster
        | Fvr07ProductionDressingKind::LichenRock
        | Fvr07ProductionDressingKind::NestMarker
        | Fvr07ProductionDressingKind::CorpseMarker => 1,
    }
}

fn dressing_scale(kind: Fvr07ProductionDressingKind) -> Vec3 {
    match kind {
        Fvr07ProductionDressingKind::LeafPatch => Vec3::new(1.38, 1.58, 1.34),
        Fvr07ProductionDressingKind::MushroomCluster => Vec3::new(1.26, 1.46, 1.26),
        Fvr07ProductionDressingKind::PebbleCluster => Vec3::new(1.00, 0.82, 1.00),
        Fvr07ProductionDressingKind::NestMarker => Vec3::new(1.08, 0.76, 1.08),
        Fvr07ProductionDressingKind::FoodResource => Vec3::new(1.28, 1.50, 1.28),
        Fvr07ProductionDressingKind::CorpseMarker => Vec3::new(0.90, 0.52, 0.86),
        Fvr07ProductionDressingKind::FlowerPatch => Vec3::new(1.32, 1.62, 1.32),
        Fvr07ProductionDressingKind::ReedCluster => Vec3::new(1.24, 1.72, 1.24),
        Fvr07ProductionDressingKind::LichenRock => Vec3::new(1.12, 0.96, 1.12),
        Fvr07ProductionDressingKind::HazardFungus => Vec3::new(1.32, 1.58, 1.32),
        Fvr07ProductionDressingKind::DeadLeafPatch => Vec3::new(1.18, 0.76, 1.18),
        Fvr07ProductionDressingKind::AlienFern => Vec3::new(1.44, 1.42, 1.30),
        Fvr07ProductionDressingKind::CrimsonSpire => Vec3::new(1.05, 1.68, 1.05),
        Fvr07ProductionDressingKind::GlowBulbCluster => Vec3::new(1.16, 1.30, 1.16),
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
        Fvr07ProductionDressingKind::AlienFern => matches!(
            tile.material,
            Fvr03ProductionVoxelMaterialKind::SafeGrass
                | Fvr03ProductionVoxelMaterialKind::Resource
                | Fvr03ProductionVoxelMaterialKind::Decay
        ),
        Fvr07ProductionDressingKind::CrimsonSpire => matches!(
            tile.material,
            Fvr03ProductionVoxelMaterialKind::SafeGrass
                | Fvr03ProductionVoxelMaterialKind::Resource
                | Fvr03ProductionVoxelMaterialKind::Hazard
        ),
        Fvr07ProductionDressingKind::GlowBulbCluster => matches!(
            tile.material,
            Fvr03ProductionVoxelMaterialKind::Resource
                | Fvr03ProductionVoxelMaterialKind::Decay
                | Fvr03ProductionVoxelMaterialKind::Hazard
        ),
    }
}

fn biome_dressing_kind(tile: TerrainDressingTile) -> Fvr07ProductionDressingKind {
    let hash = dressing_hash(tile.tile);
    match tile.material {
        Fvr03ProductionVoxelMaterialKind::SafeGrass => match hash % 6 {
            0 => Fvr07ProductionDressingKind::FlowerPatch,
            1 => Fvr07ProductionDressingKind::AlienFern,
            2 => Fvr07ProductionDressingKind::CrimsonSpire,
            _ => Fvr07ProductionDressingKind::LeafPatch,
        },
        Fvr03ProductionVoxelMaterialKind::Resource => match hash % 6 {
            0 => Fvr07ProductionDressingKind::FoodResource,
            1 => Fvr07ProductionDressingKind::FlowerPatch,
            2 => Fvr07ProductionDressingKind::MushroomCluster,
            3 => Fvr07ProductionDressingKind::AlienFern,
            4 => Fvr07ProductionDressingKind::GlowBulbCluster,
            _ => Fvr07ProductionDressingKind::CrimsonSpire,
        },
        Fvr03ProductionVoxelMaterialKind::Hazard => match hash % 4 {
            0 => Fvr07ProductionDressingKind::GlowBulbCluster,
            1 => Fvr07ProductionDressingKind::CrimsonSpire,
            _ => Fvr07ProductionDressingKind::HazardFungus,
        },
        Fvr03ProductionVoxelMaterialKind::Decay => match hash % 4 {
            0 => Fvr07ProductionDressingKind::MushroomCluster,
            1 => Fvr07ProductionDressingKind::GlowBulbCluster,
            2 => Fvr07ProductionDressingKind::AlienFern,
            _ => Fvr07ProductionDressingKind::DeadLeafPatch,
        },
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

fn dressing_child_clears_occupied(
    translation: Vec3,
    occupied_tiles: &BTreeSet<VoxelTileCoord>,
) -> bool {
    occupied_tiles.iter().all(|occupied| {
        let dx = translation.x - (occupied.x as f32 + 0.5);
        let dz = translation.z - (occupied.z as f32 + 0.5);
        dx * dx + dz * dz >= 0.85 * 0.85
    })
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
        base_color: match kind {
            Fvr07ProductionDressingKind::LeafPatch => Color::srgb(0.54, 0.70, 0.46),
            Fvr07ProductionDressingKind::FlowerPatch => Color::srgb(0.88, 0.90, 0.82),
            Fvr07ProductionDressingKind::ReedCluster => Color::srgb(0.64, 0.76, 0.54),
            Fvr07ProductionDressingKind::LichenRock
            | Fvr07ProductionDressingKind::PebbleCluster => Color::srgb(0.70, 0.74, 0.66),
            Fvr07ProductionDressingKind::HazardFungus => Color::srgb(1.00, 0.62, 0.52),
            Fvr07ProductionDressingKind::MushroomCluster => Color::srgb(0.82, 0.76, 0.86),
            Fvr07ProductionDressingKind::FoodResource => Color::srgb(0.76, 0.84, 0.62),
            Fvr07ProductionDressingKind::DeadLeafPatch => Color::srgb(0.84, 0.70, 0.52),
            Fvr07ProductionDressingKind::NestMarker => Color::srgb(0.74, 0.60, 0.42),
            Fvr07ProductionDressingKind::CorpseMarker => Color::srgb(0.76, 0.70, 0.68),
            Fvr07ProductionDressingKind::AlienFern => Color::srgb(0.64, 0.82, 0.54),
            Fvr07ProductionDressingKind::CrimsonSpire => Color::srgb(0.92, 0.52, 0.46),
            Fvr07ProductionDressingKind::GlowBulbCluster => Color::srgb(0.62, 0.78, 0.92),
        },
        perceptual_roughness: match kind {
            Fvr07ProductionDressingKind::HazardFungus => 0.68,
            Fvr07ProductionDressingKind::MushroomCluster
            | Fvr07ProductionDressingKind::FoodResource
            | Fvr07ProductionDressingKind::CrimsonSpire => 0.76,
            Fvr07ProductionDressingKind::GlowBulbCluster => 0.70,
            Fvr07ProductionDressingKind::LichenRock
            | Fvr07ProductionDressingKind::PebbleCluster => 0.92,
            _ => 0.86,
        },
        metallic: 0.0,
        emissive: match kind {
            Fvr07ProductionDressingKind::HazardFungus => LinearRgba::rgb(0.08, 0.012, 0.018),
            Fvr07ProductionDressingKind::GlowBulbCluster => LinearRgba::rgb(0.018, 0.035, 0.055),
            _ => LinearRgba::BLACK,
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
        Fvr07ProductionDressingKind::AlienFern => append_alien_fern(&mut builder),
        Fvr07ProductionDressingKind::CrimsonSpire => append_crimson_spire(&mut builder),
        Fvr07ProductionDressingKind::GlowBulbCluster => append_glow_bulb_cluster(&mut builder),
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
            false,
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
        let root = center - direction * length * 0.45;
        let left = center - side * width;
        let ridge = center + direction * length * 0.08 + Vec3::Y * rise * 0.72;
        let right = center + side * width;
        let tip = center + direction * length * 0.55 + Vec3::Y * rise;
        self.triangle([root, left, ridge], color);
        self.triangle([root, ridge, right], color);
        self.triangle([left, tip, ridge], color);
        self.triangle([ridge, tip, right], color);
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
    let lower_colors = if dry {
        [[0.68, 0.36, 0.08, 1.0], [0.48, 0.24, 0.05, 1.0]]
    } else {
        [[0.26, 0.62, 0.08, 1.0], [0.40, 0.74, 0.12, 1.0]]
    };
    for index in 0..8 {
        let angle = index as f32 * std::f32::consts::TAU / 8.0 + 0.18;
        let center = Vec3::new(angle.cos() * 0.10, 0.02, angle.sin() * 0.10);
        builder.leaf(
            center,
            0.60 - (index % 2) as f32 * 0.055,
            0.20,
            angle,
            0.14 + (index % 3) as f32 * 0.018,
            lower_colors[index % lower_colors.len()],
        );
    }

    let upper_colors = if dry {
        [[0.76, 0.44, 0.10, 1.0], [0.56, 0.30, 0.06, 1.0]]
    } else {
        [[0.18, 0.50, 0.05, 1.0], [0.50, 0.80, 0.16, 1.0]]
    };
    for index in 0..6 {
        let angle = index as f32 * std::f32::consts::TAU / 6.0 + 0.70;
        let center = Vec3::new(angle.cos() * 0.08, 0.10, angle.sin() * 0.08);
        builder.leaf(
            center,
            0.47 - (index % 2) as f32 * 0.04,
            0.16,
            angle,
            0.24 + (index % 3) as f32 * 0.025,
            upper_colors[index % upper_colors.len()],
        );
    }

    if !dry {
        for index in 0..4 {
            let angle = index as f32 * std::f32::consts::FRAC_PI_2 + 0.42;
            builder.blade(
                Vec3::new(angle.cos() * 0.13, 0.02, angle.sin() * 0.13),
                0.38 + (index % 2) as f32 * 0.06,
                0.028,
                angle + 0.32,
                if index % 2 == 0 {
                    [0.72, 0.16, 0.08, 1.0]
                } else {
                    [0.56, 0.10, 0.22, 1.0]
                },
            );
        }
    }
}

fn append_flower_patch(builder: &mut DressingMeshBuilder) {
    for index in 0..8 {
        let angle = index as f32 * std::f32::consts::TAU / 8.0 + 0.12;
        builder.leaf(
            Vec3::new(angle.cos() * 0.08, 0.018, angle.sin() * 0.08),
            0.42 - (index % 2) as f32 * 0.045,
            0.13,
            angle,
            0.12 + (index % 3) as f32 * 0.015,
            if index % 2 == 0 {
                [0.22, 0.56, 0.07, 1.0]
            } else {
                [0.42, 0.70, 0.12, 1.0]
            },
        );
    }

    for index in 0..5 {
        let angle = index as f32 * std::f32::consts::TAU / 5.0 + 0.35;
        let radius = 0.10 + (index % 3) as f32 * 0.075;
        let center = Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
        let height = 0.30 + (index % 3) as f32 * 0.055;
        builder.tapered_prism(center, height, 0.030, 0.014, [0.30, 0.66, 0.10, 1.0]);
        for petal in 0..5 {
            builder.leaf(
                center + Vec3::Y * height,
                0.14,
                0.052,
                petal as f32 * std::f32::consts::TAU / 5.0,
                0.012,
                if index % 3 == 0 {
                    [1.00, 0.32, 0.72, 1.0]
                } else if index % 3 == 1 {
                    [0.90, 0.92, 1.00, 1.0]
                } else {
                    [0.48, 0.58, 1.00, 1.0]
                },
            );
        }
        builder.rock(
            center + Vec3::Y * height,
            0.034,
            0.020,
            [1.00, 0.82, 0.18, 1.0],
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
                [0.58, 0.05, 0.06, 1.0]
            } else {
                [0.86, 0.70, 0.48, 1.0]
            },
            if hazard {
                if index % 2 == 0 {
                    [1.00, 0.03, 0.04, 1.0]
                } else {
                    [1.00, 0.28, 0.02, 1.0]
                }
            } else {
                [0.72, 0.34, 0.92, 1.0]
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
            [0.26, 0.29, 0.25, 1.0],
            lichen.then_some([0.38, 0.56, 0.12, 1.0]),
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
            0.045,
            angle + 0.2,
            [0.32, 0.68, 0.20, 1.0],
        );
        if index % 2 == 0 {
            builder.tapered_prism(
                center + Vec3::Y * (0.48 + (index % 4) as f32 * 0.10),
                0.20,
                0.045,
                0.025,
                [0.72, 0.38, 0.12, 1.0],
            );
        }
    }
}

fn append_alien_fern(builder: &mut DressingMeshBuilder) {
    for frond in 0..5 {
        let angle = frond as f32 * std::f32::consts::TAU / 5.0 + 0.22;
        let direction = Vec3::new(angle.sin(), 0.0, angle.cos());
        let root = direction * (0.06 + (frond % 2) as f32 * 0.04);
        let height = 0.48 + (frond % 3) as f32 * 0.08;
        builder.blade(root, height, 0.035, angle, [0.16, 0.42, 0.05, 1.0]);
        for tier in 0..3 {
            let y = 0.13 + tier as f32 * 0.12;
            let center = root + direction * (tier as f32 * 0.035) + Vec3::Y * y;
            let length = 0.28 - tier as f32 * 0.045;
            for side in [-1.0_f32, 1.0] {
                builder.leaf(
                    center,
                    length,
                    0.075,
                    angle + side * 1.02,
                    0.055 + tier as f32 * 0.012,
                    if (frond + tier) % 2 == 0 {
                        [0.24, 0.62, 0.08, 1.0]
                    } else {
                        [0.42, 0.76, 0.12, 1.0]
                    },
                );
            }
        }
    }
}

fn append_crimson_spire(builder: &mut DressingMeshBuilder) {
    for index in 0..3 {
        let angle = index as f32 * std::f32::consts::TAU / 3.0 + 0.38;
        let radius = 0.08 + index as f32 * 0.055;
        let center = Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
        let height = 0.54 + index as f32 * 0.085;
        builder.tapered_prism(
            center,
            height,
            0.072,
            0.026,
            if index % 2 == 0 {
                [0.54, 0.045, 0.055, 1.0]
            } else {
                [0.72, 0.075, 0.035, 1.0]
            },
        );
        for tier in 0..3 {
            let y = height * (0.34 + tier as f32 * 0.22);
            for side in [-1.0_f32, 1.0] {
                builder.leaf(
                    center + Vec3::Y * y,
                    0.30 - tier as f32 * 0.045,
                    0.092,
                    angle + side * (0.82 + tier as f32 * 0.12),
                    0.090,
                    if tier == 0 {
                        [0.72, 0.08, 0.04, 1.0]
                    } else if tier == 1 {
                        [0.90, 0.16, 0.045, 1.0]
                    } else {
                        [1.00, 0.38, 0.08, 1.0]
                    },
                );
            }
        }
    }
}

fn append_glow_bulb_cluster(builder: &mut DressingMeshBuilder) {
    for index in 0..6 {
        let angle = index as f32 * std::f32::consts::TAU / 6.0 + 0.16;
        builder.leaf(
            Vec3::new(angle.cos() * 0.07, 0.018, angle.sin() * 0.07),
            0.34 - (index % 2) as f32 * 0.035,
            0.105,
            angle,
            0.090,
            if index % 2 == 0 {
                [0.12, 0.48, 0.42, 1.0]
            } else {
                [0.20, 0.62, 0.54, 1.0]
            },
        );
    }
    for index in 0..3 {
        let angle = index as f32 * std::f32::consts::TAU / 3.0 + 0.42;
        let radius = 0.07 + index as f32 * 0.055;
        let center = Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
        let height = 0.28 + index as f32 * 0.060;
        builder.tapered_prism(center, height, 0.034, 0.016, [0.18, 0.48, 0.42, 1.0]);
        builder.rock(
            center + Vec3::Y * height,
            0.105 + index as f32 * 0.010,
            0.13,
            if index % 3 == 0 {
                [0.20, 0.86, 0.96, 1.0]
            } else if index % 3 == 1 {
                [0.60, 0.38, 0.96, 1.0]
            } else {
                [1.00, 0.62, 0.12, 1.0]
            },
            None,
        );
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
                [0.78, 0.38, 0.10, 1.0]
            } else {
                [0.50, 0.28, 0.08, 1.0]
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
            [0.56, 0.32, 0.10, 1.0],
            None,
        );
    }
}

fn append_food_resource(builder: &mut DressingMeshBuilder) {
    builder.tapered_prism(Vec3::ZERO, 0.58, 0.055, 0.025, [0.30, 0.64, 0.12, 1.0]);
    for index in 0..6 {
        let angle = index as f32 * std::f32::consts::TAU / 6.0;
        builder.leaf(
            Vec3::Y * (0.34 + (index % 2) as f32 * 0.10),
            0.38,
            0.13,
            angle,
            0.10,
            [0.58, 0.82, 0.16, 1.0],
        );
        builder.rock(
            Vec3::new(
                angle.cos() * 0.22,
                0.56 + (index % 2) as f32 * 0.08,
                angle.sin() * 0.22,
            ),
            0.09,
            0.11,
            [1.00, 0.68, 0.10, 1.0],
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
    use bevy::mesh::VertexAttributeValues;

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
        assert!(first
            .iter()
            .all(|spawn| dressing_child_clears_occupied(spawn.translation, &occupied)));
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
            Fvr07ProductionDressingKind::AlienFern,
            Fvr07ProductionDressingKind::CrimsonSpire,
            Fvr07ProductionDressingKind::GlowBulbCluster,
        ] {
            assert!(kinds.contains(&required), "missing {required:?}");
        }
        let unique_clusters = |kind| {
            first
                .iter()
                .filter(|spawn| spawn.kind == kind)
                .map(|spawn| spawn.cluster_id)
                .collect::<BTreeSet<_>>()
                .len()
        };
        assert!(
            unique_clusters(Fvr07ProductionDressingKind::HazardFungus) >= 2,
            "hazard biome needs more than one token fungal cluster"
        );
        assert!(
            unique_clusters(Fvr07ProductionDressingKind::LeafPatch)
                + unique_clusters(Fvr07ProductionDressingKind::FlowerPatch)
                + unique_clusters(Fvr07ProductionDressingKind::AlienFern)
                + unique_clusters(Fvr07ProductionDressingKind::CrimsonSpire)
                + unique_clusters(Fvr07ProductionDressingKind::GlowBulbCluster)
                >= 3,
            "vegetated biomes need a readable living-plant cluster budget"
        );
        assert!(
            first.iter().any(|spawn| spawn.scale.y >= 1.45),
            "terrain dressing silhouettes are too small for the product camera"
        );
    }

    #[test]
    fn leaf_patch_forms_a_dense_grounded_two_tier_canopy() {
        let mesh = build_dressing_mesh(Fvr07ProductionDressingKind::LeafPatch);
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("leaf patch positions");
        };
        let Some(VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("leaf patch normals");
        };

        assert!(
            positions.len() >= 180,
            "the hero groundcover needs enough folded leaves to read as a canopy"
        );
        assert!(
            positions
                .iter()
                .filter(|position| position[1] <= 0.20)
                .count()
                >= 60,
            "broad foliage should grow from a grounded rosette, not sit on bare stems"
        );
        assert!(
            positions
                .iter()
                .map(|position| position[1])
                .fold(f32::NEG_INFINITY, f32::max)
                <= 0.62,
            "groundcover should stay below the creatures' primary silhouette"
        );
        assert!(
            normals.iter().filter(|normal| normal[1] > 0.25).count() >= 120,
            "broad leaves should face the creature-stage key light"
        );
    }

    #[test]
    fn cluster_children_use_nonuniform_silhouette_variation() {
        let tile = TerrainDressingTile {
            tile: VoxelTileCoord::new(12, -8),
            material: Fvr03ProductionVoxelMaterialKind::SafeGrass,
            height: 0.75,
            resource_bias: 0.2,
            hazard_pressure: 0.0,
        };
        let mut spawns = Vec::new();
        append_cluster(
            &mut spawns,
            8,
            Fvr07ProductionDressingKind::LeafPatch,
            &tile,
            2,
            false,
            1,
            &BTreeSet::new(),
        );

        assert_eq!(spawns.len(), 2);
        let first_ratio = spawns[0].scale.x / spawns[0].scale.z;
        let second_ratio = spawns[1].scale.x / spawns[1].scale.z;
        assert!(
            (first_ratio - second_ratio).abs() >= 0.20,
            "cluster children should not repeat one uniformly scaled silhouette"
        );
    }

    #[test]
    fn production_dressing_includes_three_additional_alien_plant_silhouettes() {
        for kind in [
            Fvr07ProductionDressingKind::AlienFern,
            Fvr07ProductionDressingKind::CrimsonSpire,
            Fvr07ProductionDressingKind::GlowBulbCluster,
        ] {
            assert!(PRODUCTION_DRESSING_KINDS.contains(&kind));
            let mesh = build_dressing_mesh(kind);
            let Some(VertexAttributeValues::Float32x3(positions)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                panic!("{kind:?} positions");
            };
            assert!(
                positions.len() >= 96,
                "{kind:?} needs a composite production silhouette"
            );
        }
    }

    #[test]
    fn crimson_spire_uses_three_branched_trunks_not_a_picket_fence() {
        let mesh = build_dressing_mesh(Fvr07ProductionDressingKind::CrimsonSpire);
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("crimson spire positions");
        };
        let grounded_vertices = positions
            .iter()
            .filter(|position| position[1] <= 0.03)
            .count();

        assert!(positions.len() >= 250);
        assert!(
            grounded_vertices <= 32,
            "too many separate vertical stems create a fence silhouette"
        );
    }

    #[test]
    fn glow_bulbs_form_a_compact_three_pod_rosette() {
        let mesh = build_dressing_mesh(Fvr07ProductionDressingKind::GlowBulbCluster);
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("glow bulb positions");
        };

        assert!(
            (240..=340).contains(&positions.len()),
            "glow bulbs should use three pods plus basal leaves, not a row of poles"
        );
    }

    #[test]
    fn comfort_dressing_spreads_across_many_anchors_without_cluster_entity_piles() {
        let materials = [
            Fvr03ProductionVoxelMaterialKind::SafeGrass,
            Fvr03ProductionVoxelMaterialKind::Resource,
            Fvr03ProductionVoxelMaterialKind::Hazard,
            Fvr03ProductionVoxelMaterialKind::Stone,
            Fvr03ProductionVoxelMaterialKind::Soil,
            Fvr03ProductionVoxelMaterialKind::Water,
            Fvr03ProductionVoxelMaterialKind::Decay,
            Fvr03ProductionVoxelMaterialKind::Sand,
        ];
        let mut tiles = BTreeMap::new();
        for z in 0..16 {
            for x in 0..16 {
                let tile = VoxelTileCoord::new(x * 2, z * 2);
                let material = materials[(x + z * 5) as usize % materials.len()];
                tiles.insert(
                    tile,
                    TerrainDressingTile {
                        tile,
                        material,
                        height: 0.75,
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
        let spawns = plan_production_terrain_dressing(&tiles, &BTreeSet::new(), 128, 2, false);
        let mut children_by_cluster = BTreeMap::<u32, usize>::new();
        for spawn in &spawns {
            *children_by_cluster.entry(spawn.cluster_id).or_default() += 1;
        }

        assert!(children_by_cluster.len() >= 48);
        assert!(children_by_cluster.values().all(|count| *count <= 2));
        assert!(spawns.len() <= 128);
    }

    #[test]
    fn hazard_fungus_avoids_planar_ground_fans_on_sloped_tiles() {
        let mesh = build_dressing_mesh(Fvr07ProductionDressingKind::HazardFungus);
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("hazard fungus positions");
        };
        let broad_ground_vertices = positions
            .iter()
            .filter(|position| {
                position[1] <= 0.04
                    && (position[0] * position[0] + position[2] * position[2]).sqrt() >= 0.45
            })
            .count();

        assert!(
            positions.len() >= 300,
            "hazard clusters should retain several faceted mushroom silhouettes"
        );
        assert!(
            broad_ground_vertices <= 8,
            "flat ground fans intersect smoothed terrain and expose radial wedges"
        );
    }

    #[test]
    fn flower_patch_uses_low_basal_foliage_instead_of_bare_poles() {
        let mesh = build_dressing_mesh(Fvr07ProductionDressingKind::FlowerPatch);
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("flower patch positions");
        };
        let max_height = positions
            .iter()
            .map(|position| position[1])
            .fold(f32::NEG_INFINITY, f32::max);

        assert!(
            positions
                .iter()
                .filter(|position| position[1] <= 0.18)
                .count()
                >= 80,
            "flower clusters need a broad low rosette around their stems"
        );
        assert!(
            max_height <= 0.52,
            "wildflowers should stay secondary to creature silhouettes"
        );
    }
}
