//! Creature-anchored procedural world chunks.
//!
//! This module is Bevy-independent. It provides a deterministic chunk/biome
//! substrate that can be sampled by creature-facing systems without requiring
//! a rendered camera surface. Rendering layers may mirror these samples, but
//! they are not the authority that creates the world.

use std::collections::{BTreeMap, BTreeSet};

use alife_core::{NormalizedScalar, ScaffoldContractError, Vec3f, WorldEntityId};

pub const PROCEDURAL_WORLD_CHUNKS_SCHEMA: &str = "alife.ca44a.procedural_world_chunks.v1";
pub const PROCEDURAL_WORLD_CHUNKS_SCHEMA_VERSION: u16 = 1;

pub const DEFAULT_CHUNK_TILE_SIZE: i32 = 16;
pub const DEFAULT_ACTIVATION_RADIUS_CHUNKS: i32 = 2;
pub const DEFAULT_MAX_ACTIVE_CHUNKS: usize = 256;
pub const DEFAULT_NEIGHBORHOOD_RADIUS_TILES: i32 = 6;
pub const DEFAULT_MAX_NEIGHBORHOOD_SAMPLES: usize = 96;
pub const DEFAULT_VIRTUAL_HALF_EXTENT_CHUNKS: i32 = 128;

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
}

impl ProceduralBiomeKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::SafeGrass => "safe-grass",
            Self::SoilPath => "soil-path",
            Self::ResourceGrove => "resource-grove",
            Self::HazardPressure => "hazard-pressure",
            Self::StoneRough => "stone-rough",
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
}

impl ProceduralTerrainMaterial {
    pub const fn material_id(self) -> &'static str {
        match self {
            Self::SafeGrass => "safe-grass",
            Self::NeutralSoil => "neutral-soil",
            Self::ResourceGrove => "resource-grove",
            Self::HazardPressure => "hazard-pressure",
            Self::StoneRough => "stone-dressing",
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
    pub neighborhood_radius_tiles: i32,
    pub max_neighborhood_samples: usize,
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
            neighborhood_radius_tiles: DEFAULT_NEIGHBORHOOD_RADIUS_TILES,
            max_neighborhood_samples: DEFAULT_MAX_NEIGHBORHOOD_SAMPLES,
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
            || !(0..=8).contains(&self.activation_radius_chunks)
            || self.max_active_chunks == 0
            || self.max_active_chunks > 4096
            || self.neighborhood_radius_tiles <= 0
            || self.neighborhood_radius_tiles > 64
            || self.max_neighborhood_samples == 0
            || self.max_neighborhood_samples > 4096
            || self.virtual_half_extent_chunks < self.activation_radius_chunks
            || self.virtual_half_extent_chunks > 8192
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(self)
    }

    pub fn virtual_width_tiles(self) -> usize {
        ((self.virtual_half_extent_chunks * 2 + 1) * self.chunk_tile_size) as usize
    }

    pub fn virtual_height_tiles(self) -> usize {
        self.virtual_width_tiles()
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
    pub generated_without_rendering: bool,
    pub rendering_required: bool,
}

impl ProceduralChunkActivationReport {
    pub fn validate(&self, config: ProceduralWorldConfig) -> Result<(), ScaffoldContractError> {
        config.validate()?;
        if self.schema != PROCEDURAL_WORLD_CHUNKS_SCHEMA
            || self.schema_version != PROCEDURAL_WORLD_CHUNKS_SCHEMA_VERSION
            || self.seed != config.seed
            || self.active_chunks.len() > config.max_active_chunks
            || !self.generated_without_rendering
            || self.rendering_required
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

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProceduralCreatureNeighborhood {
    pub stable_id: WorldEntityId,
    pub center_tile: ProceduralTileCoord,
    pub sample_count: usize,
    pub samples: Vec<ProceduralTerrainSample>,
    pub average_resource_bias: f32,
    pub average_hazard_pressure: f32,
    pub dominant_material: ProceduralTerrainMaterial,
    pub bounded_for_sensory: bool,
    pub can_emit_actions: bool,
    pub can_rewrite_weights: bool,
}

impl ProceduralCreatureNeighborhood {
    pub fn validate(&self, config: ProceduralWorldConfig) -> Result<(), ScaffoldContractError> {
        config.validate()?;
        self.stable_id.validate()?;
        if self.sample_count == 0
            || self.sample_count != self.samples.len()
            || self.sample_count > config.max_neighborhood_samples
            || !self.bounded_for_sensory
            || self.can_emit_actions
            || self.can_rewrite_weights
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        NormalizedScalar::new(self.average_resource_bias)?;
        NormalizedScalar::new(self.average_hazard_pressure)?;
        for sample in &self.samples {
            sample.validate()?;
        }
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
        generated_without_rendering: true,
        rendering_required: false,
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
    let biome = match chunk_hash % 100 {
        0..=14 => ProceduralBiomeKind::HazardPressure,
        15..=29 => ProceduralBiomeKind::ResourceGrove,
        30..=41 => ProceduralBiomeKind::StoneRough,
        42..=53 => ProceduralBiomeKind::SoilPath,
        _ => ProceduralBiomeKind::SafeGrass,
    };
    let safe_clearing = local_hash.is_multiple_of(17);
    let material = if safe_clearing {
        ProceduralTerrainMaterial::SafeGrass
    } else if path_band && biome != ProceduralBiomeKind::HazardPressure {
        ProceduralTerrainMaterial::NeutralSoil
    } else if biome == ProceduralBiomeKind::HazardPressure || local_hash % 97 < 5 {
        ProceduralTerrainMaterial::HazardPressure
    } else if biome == ProceduralBiomeKind::ResourceGrove || local_hash % 89 < 6 {
        ProceduralTerrainMaterial::ResourceGrove
    } else if biome == ProceduralBiomeKind::StoneRough || local_hash % 83 < 6 {
        ProceduralTerrainMaterial::StoneRough
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

pub fn sample_creature_procedural_neighborhood(
    config: ProceduralWorldConfig,
    anchor: CreatureWorldAnchor,
) -> Result<ProceduralCreatureNeighborhood, ScaffoldContractError> {
    let config = config.validate()?;
    let anchor = anchor.validate()?;
    let center_tile = anchor.tile_coord();
    let mut samples = Vec::new();
    for dz in -config.neighborhood_radius_tiles..=config.neighborhood_radius_tiles {
        for dx in -config.neighborhood_radius_tiles..=config.neighborhood_radius_tiles {
            if samples.len() >= config.max_neighborhood_samples {
                break;
            }
            if dx * dx + dz * dz
                > config.neighborhood_radius_tiles * config.neighborhood_radius_tiles
            {
                continue;
            }
            let tile = ProceduralTileCoord::new(center_tile.x + dx, center_tile.z + dz);
            samples.push(sample_procedural_terrain_tile(config, tile)?);
        }
        if samples.len() >= config.max_neighborhood_samples {
            break;
        }
    }
    if samples.is_empty() {
        return Err(ScaffoldContractError::ScalarOutOfRange);
    }
    let mut counts: BTreeMap<ProceduralTerrainMaterial, usize> = BTreeMap::new();
    let mut resource_sum = 0.0_f32;
    let mut hazard_sum = 0.0_f32;
    for sample in &samples {
        *counts.entry(sample.material).or_default() += 1;
        resource_sum += sample.resource_bias;
        hazard_sum += sample.hazard_pressure;
    }
    let dominant_material = counts
        .iter()
        .max_by_key(|(material, count)| (**count, **material as u8))
        .map(|(material, _)| *material)
        .ok_or(ScaffoldContractError::ScalarOutOfRange)?;
    let neighborhood = ProceduralCreatureNeighborhood {
        stable_id: anchor.stable_id,
        center_tile,
        sample_count: samples.len(),
        average_resource_bias: resource_sum / samples.len() as f32,
        average_hazard_pressure: hazard_sum / samples.len() as f32,
        dominant_material,
        samples,
        bounded_for_sensory: true,
        can_emit_actions: false,
        can_rewrite_weights: false,
    };
    neighborhood.validate(config)?;
    Ok(neighborhood)
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
