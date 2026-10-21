//! Creature-anchored procedural world chunks.
//!
//! This module is Bevy-independent. It provides a deterministic chunk/biome
//! substrate that can be sampled by creature-facing systems without requiring
//! a rendered camera surface. Rendering layers may mirror these samples, but
//! they are not the authority that creates the world.

use std::collections::{BTreeMap, BTreeSet};

use alife_core::{NormalizedScalar, ScaffoldContractError, Vec3f, WorldEntityId};

use crate::WorldObjectKind;

pub const PROCEDURAL_WORLD_CHUNKS_SCHEMA: &str = "alife.ca44a.procedural_world_chunks.v1";
pub const PROCEDURAL_WORLD_CHUNKS_SCHEMA_VERSION: u16 = 1;

pub const DEFAULT_CHUNK_TILE_SIZE: i32 = 16;
pub const DEFAULT_ACTIVATION_RADIUS_CHUNKS: i32 = 2;
pub const DEFAULT_MAX_ACTIVE_CHUNKS: usize = 256;
pub const DEFAULT_MAX_ACTIVE_CONTENT_CANDIDATES: usize = 512;
pub const DEFAULT_MAX_CONTENT_CANDIDATES_PER_CHUNK: usize = 48;
pub const DEFAULT_VIRTUAL_HALF_EXTENT_CHUNKS: i32 = 128;
pub const PROCEDURAL_WORLD_CONTENT_SCHEMA: &str = "alife.ca44a.procedural_world_content.v1";
pub const PROCEDURAL_WORLD_CONTENT_SCHEMA_VERSION: u16 = 1;
pub const PROCEDURAL_CONTENT_ID_BASE: u64 = 8_800_000_000_000;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct ProceduralChunkCoord {
    pub x: i32,
    pub z: i32,
}

impl ProceduralChunkCoord {
    pub const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct ProceduralTileCoord {
    pub x: i32,
    pub z: i32,
}

impl ProceduralTileCoord {
    pub const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ProceduralBiomeKind {
    SafeGrass,
    SoilPath,
    ResourceGrove,
    HazardPressure,
    StoneRough,
    WaterLowland,
    SandBank,
}

impl ProceduralBiomeKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::SafeGrass => "safe-grass",
            Self::SoilPath => "soil-path",
            Self::ResourceGrove => "resource-grove",
            Self::HazardPressure => "hazard-pressure",
            Self::StoneRough => "stone-rough",
            Self::WaterLowland => "water-lowland",
            Self::SandBank => "sand-bank",
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum ProceduralTerrainMaterial {
    SafeGrass,
    NeutralSoil,
    ResourceGrove,
    HazardPressure,
    StoneRough,
    Water,
    Sand,
}

impl ProceduralTerrainMaterial {
    pub const fn material_id(self) -> &'static str {
        match self {
            Self::SafeGrass => "safe-grass",
            Self::NeutralSoil => "neutral-soil",
            Self::ResourceGrove => "resource-grove",
            Self::HazardPressure => "hazard-pressure",
            Self::StoneRough => "stone-dressing",
            Self::Water => "water",
            Self::Sand => "sand",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProceduralWorldConfig {
    pub schema_version: u16,
    pub seed: u64,
    pub chunk_tile_size: i32,
    pub activation_radius_chunks: i32,
    pub max_active_chunks: usize,
    pub max_active_content_candidates: usize,
    pub virtual_half_extent_chunks: i32,
}

impl Default for ProceduralWorldConfig {
    fn default() -> Self {
        Self {
            schema_version: PROCEDURAL_WORLD_CHUNKS_SCHEMA_VERSION,
            seed: 4_242,
            chunk_tile_size: DEFAULT_CHUNK_TILE_SIZE,
            activation_radius_chunks: DEFAULT_ACTIVATION_RADIUS_CHUNKS,
            max_active_chunks: DEFAULT_MAX_ACTIVE_CHUNKS,
            max_active_content_candidates: DEFAULT_MAX_ACTIVE_CONTENT_CANDIDATES,
            virtual_half_extent_chunks: DEFAULT_VIRTUAL_HALF_EXTENT_CHUNKS,
        }
    }
}

impl ProceduralWorldConfig {
    pub fn with_seed(seed: u64) -> Self {
        Self {
            seed,
            ..Self::default()
        }
    }

    pub fn validate(self) -> Result<Self, ScaffoldContractError> {
        if self.schema_version != PROCEDURAL_WORLD_CHUNKS_SCHEMA_VERSION
            || !(4..=64).contains(&self.chunk_tile_size)
            || !(0..=10).contains(&self.activation_radius_chunks)
            || self.max_active_chunks == 0
            || self.max_active_chunks > 4096
            || self.max_active_content_candidates == 0
            || self.max_active_content_candidates > 8192
            || self.virtual_half_extent_chunks < self.activation_radius_chunks
            || self.virtual_half_extent_chunks > 8192
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(self)
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum ProceduralWorldContentKind {
    Food,
    Hazard,
    Obstacle,
    DressingProp,
}

impl ProceduralWorldContentKind {
    pub const fn alpha_art_role(self) -> &'static str {
        match self {
            Self::Food => "food",
            Self::Hazard => "hazard",
            Self::Obstacle => "rock-obstacle",
            Self::DressingProp => "prop-dressing",
        }
    }

    pub const fn world_object_kind(self) -> Option<WorldObjectKind> {
        match self {
            Self::Food => Some(WorldObjectKind::Food),
            Self::Hazard => Some(WorldObjectKind::Hazard),
            Self::Obstacle => Some(WorldObjectKind::Obstacle),
            Self::DressingProp => None,
        }
    }

    const fn id_discriminator(self) -> u64 {
        match self {
            Self::Food => 1,
            Self::Hazard => 2,
            Self::Obstacle => 3,
            Self::DressingProp => 4,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Food => "procedural food",
            Self::Hazard => "procedural hazard",
            Self::Obstacle => "procedural rock",
            Self::DressingProp => "procedural dressing",
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProceduralWorldContentCandidate {
    pub stable_id: WorldEntityId,
    pub label: String,
    pub kind: ProceduralWorldContentKind,
    pub world_object_kind: Option<WorldObjectKind>,
    pub alpha_art_role: String,
    pub tile: ProceduralTileCoord,
    pub chunk: ProceduralChunkCoord,
    pub material: ProceduralTerrainMaterial,
    pub anchor_stable_id: WorldEntityId,
    pub position: Vec3f,
    pub radius: f32,
    pub nutrition: f32,
    pub hazard_pain: f32,
}

impl ProceduralWorldContentCandidate {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.stable_id.validate()?;
        self.anchor_stable_id.validate()?;
        self.position.validate()?;
        NormalizedScalar::new(self.radius)?;
        NormalizedScalar::new(self.nutrition)?;
        NormalizedScalar::new(self.hazard_pain)?;
        if self.alpha_art_role != self.kind.alpha_art_role()
            || self.world_object_kind != self.kind.world_object_kind()
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        match self.kind {
            ProceduralWorldContentKind::Food => {
                if self.nutrition <= 0.0 || self.hazard_pain != 0.0 {
                    return Err(ScaffoldContractError::ScalarOutOfRange);
                }
            }
            ProceduralWorldContentKind::Hazard => {
                if self.hazard_pain <= 0.0 || self.nutrition != 0.0 {
                    return Err(ScaffoldContractError::ScalarOutOfRange);
                }
            }
            ProceduralWorldContentKind::Obstacle | ProceduralWorldContentKind::DressingProp => {
                if self.nutrition != 0.0 || self.hazard_pain != 0.0 {
                    return Err(ScaffoldContractError::ScalarOutOfRange);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProceduralWorldContentReport {
    pub schema: String,
    pub schema_version: u16,
    pub seed: u64,
    pub active_chunk_count: usize,
    pub candidate_count: usize,
    pub skipped_due_to_cap: usize,
    pub candidates: Vec<ProceduralWorldContentCandidate>,
}

impl ProceduralWorldContentReport {
    pub fn validate(&self, config: ProceduralWorldConfig) -> Result<(), ScaffoldContractError> {
        config.validate()?;
        if self.schema != PROCEDURAL_WORLD_CONTENT_SCHEMA
            || self.schema_version != PROCEDURAL_WORLD_CONTENT_SCHEMA_VERSION
            || self.seed != config.seed
            || self.candidate_count != self.candidates.len()
            || self.candidate_count > config.max_active_content_candidates
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        let mut seen = BTreeSet::new();
        for candidate in &self.candidates {
            candidate.validate()?;
            if !seen.insert(candidate.stable_id.raw()) {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        Ok(())
    }

    pub fn count_kind(&self, kind: ProceduralWorldContentKind) -> usize {
        self.candidates
            .iter()
            .filter(|candidate| candidate.kind == kind)
            .count()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreatureWorldAnchor {
    pub stable_id: WorldEntityId,
    pub position: Vec3f,
}

impl CreatureWorldAnchor {
    pub fn new(stable_id: WorldEntityId, position: Vec3f) -> Result<Self, ScaffoldContractError> {
        let anchor = Self {
            stable_id,
            position,
        };
        anchor.validate()?;
        Ok(anchor)
    }

    pub fn validate(self) -> Result<Self, ScaffoldContractError> {
        self.stable_id.validate()?;
        self.position.validate()?;
        Ok(self)
    }

    pub fn tile_coord(self) -> ProceduralTileCoord {
        ProceduralTileCoord::new(
            self.position.x.round() as i32,
            self.position.z.round() as i32,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProceduralActiveChunk {
    pub coord: ProceduralChunkCoord,
    pub anchor_stable_id: WorldEntityId,
    pub anchor_tile: ProceduralTileCoord,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProceduralChunkActivationReport {
    pub schema: String,
    pub schema_version: u16,
    pub seed: u64,
    pub creature_anchor_count: usize,
    pub active_chunks: Vec<ProceduralActiveChunk>,
    pub skipped_due_to_cap: usize,
}

impl ProceduralChunkActivationReport {
    pub fn validate(&self, config: ProceduralWorldConfig) -> Result<(), ScaffoldContractError> {
        config.validate()?;
        if self.schema != PROCEDURAL_WORLD_CHUNKS_SCHEMA
            || self.schema_version != PROCEDURAL_WORLD_CHUNKS_SCHEMA_VERSION
            || self.seed != config.seed
            || self.active_chunks.len() > config.max_active_chunks
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        let mut seen = BTreeSet::new();
        for chunk in &self.active_chunks {
            chunk.anchor_stable_id.validate()?;
            if !seen.insert(chunk.coord) {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProceduralTerrainSample {
    pub tile: ProceduralTileCoord,
    pub chunk: ProceduralChunkCoord,
    pub biome: ProceduralBiomeKind,
    pub material: ProceduralTerrainMaterial,
    pub resource_bias: f32,
    pub hazard_pressure: f32,
    pub roughness: f32,
    pub traversal_cost: f32,
}

impl ProceduralTerrainSample {
    pub fn validate(self) -> Result<Self, ScaffoldContractError> {
        for value in [
            self.resource_bias,
            self.hazard_pressure,
            self.roughness,
            self.traversal_cost,
        ] {
            NormalizedScalar::new(value)?;
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProceduralMaterialCount {
    pub material: ProceduralTerrainMaterial,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProceduralChunkSummary {
    pub coord: ProceduralChunkCoord,
    pub dominant_material: ProceduralTerrainMaterial,
    pub material_counts: Vec<ProceduralMaterialCount>,
    pub average_resource_bias: f32,
    pub average_hazard_pressure: f32,
    pub tile_count: usize,
}

impl ProceduralChunkSummary {
    pub fn validate(&self, config: ProceduralWorldConfig) -> Result<(), ScaffoldContractError> {
        config.validate()?;
        if self.tile_count != (config.chunk_tile_size * config.chunk_tile_size) as usize
            || self.material_counts.is_empty()
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        NormalizedScalar::new(self.average_resource_bias)?;
        NormalizedScalar::new(self.average_hazard_pressure)?;
        Ok(())
    }
}

pub fn activate_procedural_chunks_around_creatures(
    config: ProceduralWorldConfig,
    anchors: &[CreatureWorldAnchor],
) -> Result<ProceduralChunkActivationReport, ScaffoldContractError> {
    let config = config.validate()?;
    let mut valid_anchors = anchors
        .iter()
        .map(|anchor| anchor.validate())
        .collect::<Result<Vec<_>, _>>()?;
    valid_anchors.sort_by_key(|anchor| anchor.stable_id.raw());

    let mut active_chunks = Vec::new();
    let mut seen = BTreeSet::new();
    let mut skipped_due_to_cap = 0_usize;
    for anchor in &valid_anchors {
        let anchor_tile = anchor.tile_coord();
        let center_chunk = chunk_coord_for_tile(config, anchor_tile)?;
        for dz in -config.activation_radius_chunks..=config.activation_radius_chunks {
            for dx in -config.activation_radius_chunks..=config.activation_radius_chunks {
                let coord = ProceduralChunkCoord::new(center_chunk.x + dx, center_chunk.z + dz);
                if !chunk_in_virtual_extent(config, coord) || !seen.insert(coord) {
                    continue;
                }
                if active_chunks.len() >= config.max_active_chunks {
                    skipped_due_to_cap = skipped_due_to_cap.saturating_add(1);
                    continue;
                }
                active_chunks.push(ProceduralActiveChunk {
                    coord,
                    anchor_stable_id: anchor.stable_id,
                    anchor_tile,
                });
            }
        }
    }
    active_chunks.sort_by_key(|chunk| (chunk.coord.x, chunk.coord.z, chunk.anchor_stable_id.raw()));
    let report = ProceduralChunkActivationReport {
        schema: PROCEDURAL_WORLD_CHUNKS_SCHEMA.to_string(),
        schema_version: PROCEDURAL_WORLD_CHUNKS_SCHEMA_VERSION,
        seed: config.seed,
        creature_anchor_count: valid_anchors.len(),
        active_chunks,
        skipped_due_to_cap,
    };
    report.validate(config)?;
    Ok(report)
}

pub fn sample_procedural_terrain_tile(
    config: ProceduralWorldConfig,
    tile: ProceduralTileCoord,
) -> Result<ProceduralTerrainSample, ScaffoldContractError> {
    let config = config.validate()?;
    let chunk = chunk_coord_for_tile(config, tile)?;
    if !chunk_in_virtual_extent(config, chunk) {
        return Err(ScaffoldContractError::InvalidBounds);
    }
    let chunk_hash = seeded_hash(config.seed, chunk.x, chunk.z);
    let local_hash = seeded_hash(config.seed ^ 0xA11F_EC01_0DDB_A5E5, tile.x, tile.z);
    let path_band =
        (tile.z - floor_div(tile.x, 4)).abs() <= 1 || (tile.z + floor_div(tile.x, 6)).abs() <= 1;
    let origin_nursery = {
        let x = tile.x as i64;
        let z = tile.z as i64;
        x * x + z * z <= 22_i64 * 22_i64
    };
    let local_hazard_pressure = {
        let dx = i64::from(tile.x) - 18;
        let dz = i64::from(tile.z) - 8;
        dx * dx + dz * dz <= 7 * 7 || ((18..=30).contains(&tile.x) && (3..=16).contains(&tile.z))
    };
    let local_resource_grove = {
        let dx = i64::from(tile.x) - 8;
        let dz = i64::from(tile.z) + 7;
        dx * dx + dz * dz <= 8 * 8 || ((-3..=7).contains(&tile.x) && (-13..=-5).contains(&tile.z))
    };
    let local_stone_rough = {
        let dx = i64::from(tile.x) + 7;
        let dz = i64::from(tile.z) - 4;
        dx * dx + dz * dz <= 7 * 7 || ((-15..=-6).contains(&tile.x) && (0..=10).contains(&tile.z))
    };
    let water_channel_metric = tile.z + floor_div(tile.x, 5) + 18;
    let local_water_lowland = water_channel_metric.abs() <= 2
        || ((-34..=-18).contains(&tile.x) && (-27..=-16).contains(&tile.z));
    let local_sand_bank = water_channel_metric.abs() <= 4
        || ((-38..=-14).contains(&tile.x) && (-30..=-12).contains(&tile.z));
    let hazard_basin = (18..=34).contains(&tile.x) && (4..=22).contains(&tile.z);
    let resource_glade = (8..=28).contains(&tile.x) && (-24..=-7).contains(&tile.z);
    let stone_ridge = (-28..=-10).contains(&tile.x) && (6..=24).contains(&tile.z);
    let biome = if local_water_lowland {
        ProceduralBiomeKind::WaterLowland
    } else if local_sand_bank {
        ProceduralBiomeKind::SandBank
    } else if local_hazard_pressure || hazard_basin {
        ProceduralBiomeKind::HazardPressure
    } else if local_resource_grove || resource_glade {
        ProceduralBiomeKind::ResourceGrove
    } else if local_stone_rough || stone_ridge {
        ProceduralBiomeKind::StoneRough
    } else if origin_nursery {
        ProceduralBiomeKind::SafeGrass
    } else {
        match chunk_hash % 100 {
            0..=5 => ProceduralBiomeKind::WaterLowland,
            6..=13 => ProceduralBiomeKind::SandBank,
            14..=21 => ProceduralBiomeKind::HazardPressure,
            22..=36 => ProceduralBiomeKind::ResourceGrove,
            37..=48 => ProceduralBiomeKind::StoneRough,
            49..=63 => ProceduralBiomeKind::SoilPath,
            _ => ProceduralBiomeKind::SafeGrass,
        }
    };
    let material = if local_water_lowland && local_hash % 100 < 82 {
        ProceduralTerrainMaterial::Water
    } else if local_sand_bank && local_hash % 100 < 72 {
        ProceduralTerrainMaterial::Sand
    } else if local_hazard_pressure {
        ProceduralTerrainMaterial::HazardPressure
    } else if local_resource_grove {
        ProceduralTerrainMaterial::ResourceGrove
    } else if local_stone_rough {
        ProceduralTerrainMaterial::StoneRough
    } else if hazard_basin {
        ProceduralTerrainMaterial::HazardPressure
    } else if resource_glade {
        ProceduralTerrainMaterial::ResourceGrove
    } else if stone_ridge {
        ProceduralTerrainMaterial::StoneRough
    } else if origin_nursery && path_band {
        ProceduralTerrainMaterial::NeutralSoil
    } else if origin_nursery {
        ProceduralTerrainMaterial::SafeGrass
    } else if path_band && biome != ProceduralBiomeKind::HazardPressure {
        ProceduralTerrainMaterial::NeutralSoil
    } else if let Some(blob_material) = procedural_blob_material(config, tile) {
        blob_material
    } else if biome == ProceduralBiomeKind::SoilPath || path_band {
        ProceduralTerrainMaterial::NeutralSoil
    } else {
        ProceduralTerrainMaterial::SafeGrass
    };
    let (resource_bias, hazard_pressure, roughness, traversal_cost) = match material {
        ProceduralTerrainMaterial::SafeGrass => (0.24, 0.04, 0.20, 0.18),
        ProceduralTerrainMaterial::NeutralSoil => (0.30, 0.06, 0.28, 0.22),
        ProceduralTerrainMaterial::ResourceGrove => (0.82, 0.05, 0.34, 0.26),
        ProceduralTerrainMaterial::HazardPressure => (0.08, 0.86, 0.60, 0.52),
        ProceduralTerrainMaterial::StoneRough => (0.14, 0.16, 0.78, 0.64),
        ProceduralTerrainMaterial::Water => (0.12, 0.10, 0.24, 0.44),
        ProceduralTerrainMaterial::Sand => (0.22, 0.07, 0.38, 0.30),
    };
    ProceduralTerrainSample {
        tile,
        chunk,
        biome,
        material,
        resource_bias,
        hazard_pressure,
        roughness,
        traversal_cost,
    }
    .validate()
}

fn procedural_blob_material(
    config: ProceduralWorldConfig,
    tile: ProceduralTileCoord,
) -> Option<ProceduralTerrainMaterial> {
    let cell_size = 18;
    let cell_x = floor_div(tile.x, cell_size);
    let cell_z = floor_div(tile.z, cell_size);
    let mut best: Option<(i32, ProceduralTerrainMaterial)> = None;

    for cx in cell_x - 1..=cell_x + 1 {
        for cz in cell_z - 1..=cell_z + 1 {
            let hash = seeded_hash(config.seed ^ 0x71E5_AA55_5EED_CAFE, cx, cz);
            let material = match hash % 100 {
                0..=7 => ProceduralTerrainMaterial::HazardPressure,
                8..=19 => ProceduralTerrainMaterial::ResourceGrove,
                20..=30 => ProceduralTerrainMaterial::StoneRough,
                31..=37 => ProceduralTerrainMaterial::NeutralSoil,
                38..=41 => ProceduralTerrainMaterial::Sand,
                _ => continue,
            };
            let offset_x = 3 + ((hash >> 8) % 12) as i32;
            let offset_z = 3 + ((hash >> 16) % 12) as i32;
            let center_x = cx * cell_size + offset_x;
            let center_z = cz * cell_size + offset_z;
            let radius = 4 + ((hash >> 24) % 5) as i32;
            let dx = tile.x - center_x;
            let dz = tile.z - center_z;
            let distance_sq = dx * dx + dz * dz;
            if distance_sq <= radius * radius {
                match best {
                    Some((best_distance, _)) if best_distance <= distance_sq => {}
                    _ => best = Some((distance_sq, material)),
                }
            }
        }
    }

    best.map(|(_, material)| material)
}

pub fn procedural_chunk_summary(
    config: ProceduralWorldConfig,
    coord: ProceduralChunkCoord,
) -> Result<ProceduralChunkSummary, ScaffoldContractError> {
    let config = config.validate()?;
    if !chunk_in_virtual_extent(config, coord) {
        return Err(ScaffoldContractError::InvalidBounds);
    }
    let base_x = coord.x * config.chunk_tile_size;
    let base_z = coord.z * config.chunk_tile_size;
    let mut counts: BTreeMap<ProceduralTerrainMaterial, usize> = BTreeMap::new();
    let mut resource_sum = 0.0_f32;
    let mut hazard_sum = 0.0_f32;
    let mut tile_count = 0_usize;
    for dz in 0..config.chunk_tile_size {
        for dx in 0..config.chunk_tile_size {
            let sample = sample_procedural_terrain_tile(
                config,
                ProceduralTileCoord::new(base_x + dx, base_z + dz),
            )?;
            *counts.entry(sample.material).or_default() += 1;
            resource_sum += sample.resource_bias;
            hazard_sum += sample.hazard_pressure;
            tile_count += 1;
        }
    }
    let dominant_material = counts
        .iter()
        .max_by_key(|(material, count)| (**count, **material as u8))
        .map(|(material, _)| *material)
        .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
    let material_counts = counts
        .into_iter()
        .map(|(material, count)| ProceduralMaterialCount { material, count })
        .collect::<Vec<_>>();
    let summary = ProceduralChunkSummary {
        coord,
        dominant_material,
        material_counts,
        average_resource_bias: resource_sum / tile_count as f32,
        average_hazard_pressure: hazard_sum / tile_count as f32,
        tile_count,
    };
    summary.validate(config)?;
    Ok(summary)
}

pub fn generate_procedural_world_content(
    config: ProceduralWorldConfig,
    activation: &ProceduralChunkActivationReport,
) -> Result<ProceduralWorldContentReport, ScaffoldContractError> {
    let config = config.validate()?;
    activation.validate(config)?;
    let mut candidates = Vec::new();
    let mut skipped_due_to_cap = 0_usize;
    let mut seen_ids = BTreeSet::new();

    let mut active_chunks = activation.active_chunks.clone();
    active_chunks.sort_by_key(|chunk| {
        let chunk_center_x = chunk.coord.x * config.chunk_tile_size + config.chunk_tile_size / 2;
        let chunk_center_z = chunk.coord.z * config.chunk_tile_size + config.chunk_tile_size / 2;
        let dx = chunk_center_x - chunk.anchor_tile.x;
        let dz = chunk_center_z - chunk.anchor_tile.z;
        (
            dx * dx + dz * dz,
            chunk.anchor_stable_id.raw(),
            chunk.coord.x,
            chunk.coord.z,
        )
    });

    for active_chunk in &active_chunks {
        let base_x = active_chunk.coord.x * config.chunk_tile_size;
        let base_z = active_chunk.coord.z * config.chunk_tile_size;
        let mut chunk_candidate_count = 0_usize;
        for dz in 0..config.chunk_tile_size {
            for dx in 0..config.chunk_tile_size {
                if chunk_candidate_count >= DEFAULT_MAX_CONTENT_CANDIDATES_PER_CHUNK {
                    continue;
                }
                let tile = ProceduralTileCoord::new(base_x + dx, base_z + dz);
                let sample = sample_procedural_terrain_tile(config, tile)?;
                let hash = seeded_hash(config.seed ^ 0xC0DE_C0DE_51A7_EED5, tile.x, tile.z);
                let Some(kind) = procedural_content_kind_for_sample(sample, hash) else {
                    continue;
                };
                if candidates.len() >= config.max_active_content_candidates {
                    skipped_due_to_cap = skipped_due_to_cap.saturating_add(1);
                    continue;
                }
                let stable_id = procedural_content_stable_id(config, kind, tile, &mut seen_ids)?;
                let candidate = procedural_world_content_candidate(
                    stable_id,
                    kind,
                    sample,
                    active_chunk,
                    hash,
                )?;
                candidate.validate()?;
                candidates.push(candidate);
                chunk_candidate_count = chunk_candidate_count.saturating_add(1);
            }
        }
    }

    candidates.sort_by_key(|candidate| candidate.stable_id.raw());
    let report = ProceduralWorldContentReport {
        schema: PROCEDURAL_WORLD_CONTENT_SCHEMA.to_string(),
        schema_version: PROCEDURAL_WORLD_CONTENT_SCHEMA_VERSION,
        seed: config.seed,
        active_chunk_count: activation.active_chunks.len(),
        candidate_count: candidates.len(),
        skipped_due_to_cap,
        candidates,
    };
    report.validate(config)?;
    Ok(report)
}

pub fn chunk_coord_for_tile(
    config: ProceduralWorldConfig,
    tile: ProceduralTileCoord,
) -> Result<ProceduralChunkCoord, ScaffoldContractError> {
    let config = config.validate()?;
    Ok(ProceduralChunkCoord::new(
        floor_div(tile.x, config.chunk_tile_size),
        floor_div(tile.z, config.chunk_tile_size),
    ))
}

fn chunk_in_virtual_extent(config: ProceduralWorldConfig, coord: ProceduralChunkCoord) -> bool {
    (-config.virtual_half_extent_chunks..=config.virtual_half_extent_chunks).contains(&coord.x)
        && (-config.virtual_half_extent_chunks..=config.virtual_half_extent_chunks)
            .contains(&coord.z)
}

fn procedural_content_kind_for_sample(
    sample: ProceduralTerrainSample,
    hash: u32,
) -> Option<ProceduralWorldContentKind> {
    match sample.material {
        ProceduralTerrainMaterial::ResourceGrove if hash.is_multiple_of(4) => {
            Some(ProceduralWorldContentKind::Food)
        }
        ProceduralTerrainMaterial::ResourceGrove if hash.is_multiple_of(2) => {
            Some(ProceduralWorldContentKind::DressingProp)
        }
        ProceduralTerrainMaterial::HazardPressure if hash.is_multiple_of(8) => {
            Some(ProceduralWorldContentKind::Hazard)
        }
        ProceduralTerrainMaterial::HazardPressure if hash.is_multiple_of(3) => {
            Some(ProceduralWorldContentKind::DressingProp)
        }
        ProceduralTerrainMaterial::StoneRough if hash.is_multiple_of(5) => {
            Some(ProceduralWorldContentKind::Obstacle)
        }
        ProceduralTerrainMaterial::StoneRough if hash.is_multiple_of(2) => {
            Some(ProceduralWorldContentKind::DressingProp)
        }
        ProceduralTerrainMaterial::Water if hash.is_multiple_of(7) => {
            Some(ProceduralWorldContentKind::DressingProp)
        }
        ProceduralTerrainMaterial::Sand if hash.is_multiple_of(3) => {
            Some(ProceduralWorldContentKind::DressingProp)
        }
        ProceduralTerrainMaterial::SafeGrass | ProceduralTerrainMaterial::NeutralSoil
            if hash.is_multiple_of(2) =>
        {
            Some(ProceduralWorldContentKind::DressingProp)
        }
        _ => None,
    }
}

fn procedural_content_stable_id(
    config: ProceduralWorldConfig,
    kind: ProceduralWorldContentKind,
    tile: ProceduralTileCoord,
    seen_ids: &mut BTreeSet<u64>,
) -> Result<WorldEntityId, ScaffoldContractError> {
    for salt in 0..16_u64 {
        let hash = seeded_hash(
            config.seed ^ (kind.id_discriminator() << 48) ^ salt,
            tile.x,
            tile.z,
        ) as u64;
        let raw = PROCEDURAL_CONTENT_ID_BASE
            + kind.id_discriminator() * 1_000_000_000
            + (hash % 900_000_000);
        if seen_ids.insert(raw) {
            return WorldEntityId(raw).validate();
        }
    }
    Err(ScaffoldContractError::InvalidId)
}

fn procedural_world_content_candidate(
    stable_id: WorldEntityId,
    kind: ProceduralWorldContentKind,
    sample: ProceduralTerrainSample,
    active_chunk: &ProceduralActiveChunk,
    hash: u32,
) -> Result<ProceduralWorldContentCandidate, ScaffoldContractError> {
    let offset_x = (((hash % 17) as f32) - 8.0) / 24.0;
    let offset_z = ((((hash / 17) % 17) as f32) - 8.0) / 24.0;
    let (radius, nutrition, hazard_pain) = match kind {
        ProceduralWorldContentKind::Food => (0.44, 0.70 + ((hash % 19) as f32) / 100.0, 0.0),
        ProceduralWorldContentKind::Hazard => (0.58, 0.0, 0.55 + ((hash % 23) as f32) / 100.0),
        ProceduralWorldContentKind::Obstacle => (0.66, 0.0, 0.0),
        ProceduralWorldContentKind::DressingProp => (0.36, 0.0, 0.0),
    };
    let candidate = ProceduralWorldContentCandidate {
        stable_id,
        label: format!("{} {}", kind.label(), stable_id.raw()),
        kind,
        world_object_kind: kind.world_object_kind(),
        alpha_art_role: kind.alpha_art_role().to_string(),
        tile: sample.tile,
        chunk: sample.chunk,
        material: sample.material,
        anchor_stable_id: active_chunk.anchor_stable_id,
        position: Vec3f::new(
            sample.tile.x as f32 + offset_x,
            0.0,
            sample.tile.z as f32 + offset_z,
        ),
        radius,
        nutrition: nutrition.min(0.95),
        hazard_pain: hazard_pain.min(0.95),
    };
    candidate.validate()?;
    Ok(candidate)
}

fn seeded_hash(seed: u64, x: i32, z: i32) -> u32 {
    let mut value = seed
        ^ ((x as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
        ^ ((z as i64 as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9));
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    (value & 0xFFFF_FFFF) as u32
}

fn floor_div(value: i32, divisor: i32) -> i32 {
    let mut quotient = value / divisor;
    let remainder = value % divisor;
    if remainder != 0 && ((remainder > 0) != (divisor > 0)) {
        quotient -= 1;
    }
    quotient
}
