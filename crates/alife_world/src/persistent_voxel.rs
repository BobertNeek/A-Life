//! FVR02 runtime contract: renderer-independent persistent voxel world backend.
//!
//! This module owns saved voxel terrain truth for later Bevy renderers. It
//! deliberately stores stable IDs, chunk coordinates, material summaries,
//! dirty regions, and profile budgets, never Bevy/wgpu/renderer handles.

use std::collections::BTreeSet;

use alife_core::{ScaffoldContractError, Vec3f, WorldEntityId};
use serde::{Deserialize, Serialize};

use crate::{
    persistence::PortableAssetDigest,
    procedural_chunks::{
        activate_procedural_chunks_around_creatures, generate_procedural_world_content,
        procedural_chunk_summary, CreatureWorldAnchor, ProceduralBiomeKind, ProceduralChunkCoord,
        ProceduralTerrainMaterial, ProceduralTileCoord, ProceduralWorldConfig,
        ProceduralWorldContentKind, DEFAULT_MAX_ACTIVE_CONTENT_CANDIDATES,
        DEFAULT_MAX_NEIGHBORHOOD_SAMPLES, DEFAULT_NEIGHBORHOOD_RADIUS_TILES,
        DEFAULT_VIRTUAL_HALF_EXTENT_CHUNKS,
    },
    WorldObjectKind, WorldObjectSaveState, WorldSaveState,
};

pub const FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA: &str = "alife.fvr02.persistent_voxel_world.v1";
pub const FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA_VERSION: u16 = 1;
pub const FVR02_GENERATOR_RULESET_ID: &str = "alife.fvr02.internal_procedural_voxel.v1";
pub const FVR02_GENERATOR_RULESET_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct VoxelChunkCoord {
    pub x: i32,
    pub z: i32,
}

impl VoxelChunkCoord {
    pub const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    pub fn for_tile(chunk_tile_size: u16, tile: VoxelTileCoord) -> Self {
        let size = i32::from(chunk_tile_size);
        Self {
            x: floor_div(tile.x, size),
            z: floor_div(tile.z, size),
        }
    }
}

impl From<ProceduralChunkCoord> for VoxelChunkCoord {
    fn from(value: ProceduralChunkCoord) -> Self {
        Self::new(value.x, value.z)
    }
}

impl From<VoxelChunkCoord> for ProceduralChunkCoord {
    fn from(value: VoxelChunkCoord) -> Self {
        Self::new(value.x, value.z)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct VoxelTileCoord {
    pub x: i32,
    pub z: i32,
}

impl VoxelTileCoord {
    pub const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }
}

impl From<ProceduralTileCoord> for VoxelTileCoord {
    fn from(value: ProceduralTileCoord) -> Self {
        Self::new(value.x, value.z)
    }
}

impl From<VoxelTileCoord> for ProceduralTileCoord {
    fn from(value: VoxelTileCoord) -> Self {
        Self::new(value.x, value.z)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum VoxelBiomeId {
    SafeGrass,
    SoilPath,
    ResourceGrove,
    HazardPressure,
    StoneRough,
    WaterLowland,
    SandBank,
}

impl From<ProceduralBiomeKind> for VoxelBiomeId {
    fn from(value: ProceduralBiomeKind) -> Self {
        match value {
            ProceduralBiomeKind::SafeGrass => Self::SafeGrass,
            ProceduralBiomeKind::SoilPath => Self::SoilPath,
            ProceduralBiomeKind::ResourceGrove => Self::ResourceGrove,
            ProceduralBiomeKind::HazardPressure => Self::HazardPressure,
            ProceduralBiomeKind::StoneRough => Self::StoneRough,
            ProceduralBiomeKind::WaterLowland => Self::WaterLowland,
            ProceduralBiomeKind::SandBank => Self::SandBank,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum VoxelTerrainMaterialId {
    SafeGrass,
    NeutralSoil,
    ResourceGrove,
    HazardPressure,
    StoneRough,
    Water,
    Sand,
    CultivatedResource,
    HazardCrystal,
}

impl VoxelTerrainMaterialId {
    pub const fn material_id(self) -> &'static str {
        match self {
            Self::SafeGrass => "safe-grass",
            Self::NeutralSoil => "neutral-soil",
            Self::ResourceGrove => "resource-grove",
            Self::HazardPressure => "hazard-pressure",
            Self::StoneRough => "stone-rough",
            Self::Water => "water",
            Self::Sand => "sand",
            Self::CultivatedResource => "cultivated-resource",
            Self::HazardCrystal => "hazard-crystal",
        }
    }
}

impl From<ProceduralTerrainMaterial> for VoxelTerrainMaterialId {
    fn from(value: ProceduralTerrainMaterial) -> Self {
        match value {
            ProceduralTerrainMaterial::SafeGrass => Self::SafeGrass,
            ProceduralTerrainMaterial::NeutralSoil => Self::NeutralSoil,
            ProceduralTerrainMaterial::ResourceGrove => Self::ResourceGrove,
            ProceduralTerrainMaterial::HazardPressure => Self::HazardPressure,
            ProceduralTerrainMaterial::StoneRough => Self::StoneRough,
            ProceduralTerrainMaterial::Water => Self::Water,
            ProceduralTerrainMaterial::Sand => Self::Sand,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PersistentVoxelProfileId {
    MinimumSettings30x30,
    MinSpecComfort1080p,
    Balanced1080p,
    HighSpecScaleUp,
    ResearchScale,
}

impl PersistentVoxelProfileId {
    pub const fn budget(self) -> PersistentVoxelProfileBudget {
        match self {
            Self::MinimumSettings30x30 => PersistentVoxelProfileBudget {
                profile_id: self,
                default_population: 30,
                target_fps: 30,
                chunk_tile_size: 16,
                activation_radius_chunks: 2,
                active_chunk_cap: 128,
                max_content_candidates: 512,
                neighborhood_radius_tiles: 6,
                max_neighborhood_samples: 96,
                virtual_half_extent_chunks: DEFAULT_VIRTUAL_HALF_EXTENT_CHUNKS,
                hot_brain_slots: 4,
                warm_brain_slots: 12,
                cold_brain_slots: 14,
            },
            Self::MinSpecComfort1080p => PersistentVoxelProfileBudget {
                profile_id: self,
                default_population: 30,
                target_fps: 60,
                chunk_tile_size: 16,
                activation_radius_chunks: 4,
                active_chunk_cap: 256,
                max_content_candidates: 768,
                neighborhood_radius_tiles: 8,
                max_neighborhood_samples: 128,
                virtual_half_extent_chunks: DEFAULT_VIRTUAL_HALF_EXTENT_CHUNKS,
                hot_brain_slots: 8,
                warm_brain_slots: 16,
                cold_brain_slots: 6,
            },
            Self::Balanced1080p => PersistentVoxelProfileBudget {
                profile_id: self,
                default_population: 50,
                target_fps: 60,
                chunk_tile_size: 16,
                activation_radius_chunks: 5,
                active_chunk_cap: 384,
                max_content_candidates: 1024,
                neighborhood_radius_tiles: 10,
                max_neighborhood_samples: 160,
                virtual_half_extent_chunks: DEFAULT_VIRTUAL_HALF_EXTENT_CHUNKS,
                hot_brain_slots: 12,
                warm_brain_slots: 24,
                cold_brain_slots: 14,
            },
            Self::HighSpecScaleUp => PersistentVoxelProfileBudget {
                profile_id: self,
                default_population: 100,
                target_fps: 60,
                chunk_tile_size: 16,
                activation_radius_chunks: 8,
                active_chunk_cap: 768,
                max_content_candidates: 2048,
                neighborhood_radius_tiles: 12,
                max_neighborhood_samples: 256,
                virtual_half_extent_chunks: DEFAULT_VIRTUAL_HALF_EXTENT_CHUNKS * 2,
                hot_brain_slots: 24,
                warm_brain_slots: 64,
                cold_brain_slots: 412,
            },
            Self::ResearchScale => PersistentVoxelProfileBudget {
                profile_id: self,
                default_population: 250,
                target_fps: 30,
                chunk_tile_size: 16,
                activation_radius_chunks: 10,
                active_chunk_cap: 1024,
                max_content_candidates: 4096,
                neighborhood_radius_tiles: 16,
                max_neighborhood_samples: 384,
                virtual_half_extent_chunks: DEFAULT_VIRTUAL_HALF_EXTENT_CHUNKS * 4,
                hot_brain_slots: 32,
                warm_brain_slots: 128,
                cold_brain_slots: 340,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistentVoxelProfileBudget {
    pub profile_id: PersistentVoxelProfileId,
    pub default_population: u16,
    pub target_fps: u16,
    pub chunk_tile_size: u16,
    pub activation_radius_chunks: u16,
    pub active_chunk_cap: u16,
    pub max_content_candidates: u16,
    pub neighborhood_radius_tiles: u16,
    pub max_neighborhood_samples: u16,
    pub virtual_half_extent_chunks: i32,
    pub hot_brain_slots: u16,
    pub warm_brain_slots: u16,
    pub cold_brain_slots: u16,
}

impl PersistentVoxelProfileBudget {
    fn procedural_config(self, world_seed: u64) -> ProceduralWorldConfig {
        ProceduralWorldConfig {
            schema_version: crate::PROCEDURAL_WORLD_CHUNKS_SCHEMA_VERSION,
            seed: world_seed,
            chunk_tile_size: i32::from(self.chunk_tile_size),
            activation_radius_chunks: i32::from(self.activation_radius_chunks),
            max_active_chunks: usize::from(self.active_chunk_cap),
            max_active_content_candidates: usize::from(self.max_content_candidates)
                .max(DEFAULT_MAX_ACTIVE_CONTENT_CANDIDATES.min(usize::from(self.active_chunk_cap))),
            neighborhood_radius_tiles: i32::from(self.neighborhood_radius_tiles)
                .max(DEFAULT_NEIGHBORHOOD_RADIUS_TILES),
            max_neighborhood_samples: usize::from(self.max_neighborhood_samples)
                .max(DEFAULT_MAX_NEIGHBORHOOD_SAMPLES.min(usize::from(self.active_chunk_cap))),
            virtual_half_extent_chunks: self.virtual_half_extent_chunks,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoxelChunkKey {
    pub coord: VoxelChunkCoord,
    pub chunk_seed: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoxelChunkSignature(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoxelGeneratorDescriptor {
    pub seed: u64,
    pub ruleset_id: String,
    pub ruleset_version: u16,
    pub output_digest: PortableAssetDigest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoxelTileEdit {
    pub tile: VoxelTileCoord,
    pub material: VoxelTerrainMaterialId,
    pub biome: VoxelBiomeId,
    pub elevation_delta: i16,
    pub resource_bias_override: Option<f32>,
    pub hazard_pressure_override: Option<f32>,
    pub author_stable_id: Option<WorldEntityId>,
    pub reason: String,
}

impl VoxelTileEdit {
    fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.reason.is_empty() || self.reason.len() > 96 {
            return Err(ScaffoldContractError::InvalidId);
        }
        if let Some(id) = self.author_stable_id {
            id.validate()?;
        }
        for value in [self.resource_bias_override, self.hazard_pressure_override]
            .into_iter()
            .flatten()
        {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(ScaffoldContractError::ScalarOutOfRange);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirtyVoxelRegion {
    pub chunk: VoxelChunkCoord,
    pub min_tile: VoxelTileCoord,
    pub max_tile: VoxelTileCoord,
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoxelMaterialCount {
    pub material: VoxelTerrainMaterialId,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterializedVoxelChunkMetadata {
    pub key: VoxelChunkKey,
    pub coord: VoxelChunkCoord,
    pub dominant_material: VoxelTerrainMaterialId,
    pub material_counts: Vec<VoxelMaterialCount>,
    pub average_resource_bias: f32,
    pub average_hazard_pressure: f32,
    pub tile_count: usize,
    pub signature: VoxelChunkSignature,
    pub saved_edit_count: usize,
    pub dirty_generation: u64,
    pub resident_profile: PersistentVoxelProfileId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatureVoxelAnchorRef {
    pub stable_id: WorldEntityId,
    pub tile: VoxelTileCoord,
    pub chunk: VoxelChunkCoord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldObjectVoxelRef {
    pub stable_id: WorldEntityId,
    pub kind: WorldObjectKind,
    pub tile: VoxelTileCoord,
    pub chunk: VoxelChunkCoord,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistentVoxelWorldSaveState {
    pub schema: String,
    pub schema_version: u16,
    pub world_seed: u64,
    pub generator: VoxelGeneratorDescriptor,
    pub profile_id: PersistentVoxelProfileId,
    pub profile_budget: PersistentVoxelProfileBudget,
    pub selected_backend_mode: String,
    pub visual_profile_reference: String,
    pub minimum_settings_budget_overrides: Vec<String>,
    pub scale_up_budget_overrides: Vec<String>,
    pub asset_manifest_refs: Vec<String>,
    pub creature_anchors: Vec<CreatureVoxelAnchorRef>,
    pub world_resource_refs: Vec<WorldObjectVoxelRef>,
    pub world_hazard_refs: Vec<WorldObjectVoxelRef>,
    pub materialized_chunks: Vec<MaterializedVoxelChunkMetadata>,
    pub materialized_chunk_count: usize,
    pub saved_edits: Vec<VoxelTileEdit>,
    pub dirty_regions: Vec<DirtyVoxelRegion>,
}

impl PersistentVoxelWorldSaveState {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA
            || self.schema_version != FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA_VERSION
            || self.world_seed == 0
            || self.generator.seed != self.world_seed
            || self.generator.ruleset_id != FVR02_GENERATOR_RULESET_ID
            || self.generator.ruleset_version != FVR02_GENERATOR_RULESET_VERSION
            || self.profile_budget.profile_id != self.profile_id
            || self.materialized_chunk_count != self.materialized_chunks.len()
            || self.materialized_chunk_count > usize::from(self.profile_budget.active_chunk_cap)
            || self.selected_backend_mode.is_empty()
            || self.visual_profile_reference.is_empty()
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        self.generator
            .output_digest
            .validate_format()
            .map_err(|_| ScaffoldContractError::ScalarOutOfRange)?;
        let mut chunks = BTreeSet::new();
        for chunk in &self.materialized_chunks {
            if !chunks.insert(chunk.coord) || chunk.coord != chunk.key.coord {
                return Err(ScaffoldContractError::InvalidId);
            }
            validate_unit(chunk.average_resource_bias)?;
            validate_unit(chunk.average_hazard_pressure)?;
            if chunk.tile_count == 0 || chunk.material_counts.is_empty() {
                return Err(ScaffoldContractError::ScalarOutOfRange);
            }
        }
        for edit in &self.saved_edits {
            edit.validate()?;
        }
        for anchor in &self.creature_anchors {
            anchor.stable_id.validate()?;
        }
        for reference in self
            .world_resource_refs
            .iter()
            .chain(self.world_hazard_refs.iter())
        {
            reference.stable_id.validate()?;
        }
        Ok(())
    }

    pub fn visible_chunk_signatures(&self) -> Vec<String> {
        self.materialized_chunks
            .iter()
            .map(|chunk| chunk.signature.0.clone())
            .collect()
    }

    fn sorted_materialized_chunks(&mut self) {
        self.materialized_chunks.sort_by_key(|chunk| chunk.coord);
        self.materialized_chunk_count = self.materialized_chunks.len();
    }
}

#[derive(Debug, Clone)]
pub struct PersistentVoxelWorldBackend {
    state: PersistentVoxelWorldSaveState,
}

impl PersistentVoxelWorldBackend {
    pub fn new(
        world_seed: u64,
        profile_id: PersistentVoxelProfileId,
    ) -> Result<Self, ScaffoldContractError> {
        if world_seed == 0 {
            return Err(ScaffoldContractError::InvalidId);
        }
        let profile_budget = profile_id.budget();
        let state = PersistentVoxelWorldSaveState {
            schema: FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA.to_string(),
            schema_version: FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA_VERSION,
            world_seed,
            generator: generator_descriptor(world_seed, profile_budget),
            profile_id,
            profile_budget,
            selected_backend_mode: "persistent-world-cpu-oracle".to_string(),
            visual_profile_reference: profile_label(profile_id).to_string(),
            minimum_settings_budget_overrides: Vec::new(),
            scale_up_budget_overrides: Vec::new(),
            asset_manifest_refs: Vec::new(),
            creature_anchors: Vec::new(),
            world_resource_refs: Vec::new(),
            world_hazard_refs: Vec::new(),
            materialized_chunks: Vec::new(),
            materialized_chunk_count: 0,
            saved_edits: Vec::new(),
            dirty_regions: Vec::new(),
        };
        state.validate()?;
        Ok(Self { state })
    }

    pub fn from_save_state(
        state: PersistentVoxelWorldSaveState,
    ) -> Result<Self, ScaffoldContractError> {
        state.validate()?;
        Ok(Self { state })
    }

    pub fn from_world_save(
        world: &WorldSaveState,
        profile_id: PersistentVoxelProfileId,
    ) -> Result<Self, ScaffoldContractError> {
        let mut backend = Self::new(world.seed, profile_id)?;
        backend.state.creature_anchors = world
            .objects
            .iter()
            .filter(|object| object.kind == WorldObjectKind::Agent)
            .map(|object| backend.creature_anchor_for_object(object))
            .collect::<Result<Vec<_>, _>>()?;
        backend.state.world_resource_refs = world
            .objects
            .iter()
            .filter(|object| object.kind == WorldObjectKind::Food)
            .map(|object| backend.object_ref(object))
            .collect::<Result<Vec<_>, _>>()?;
        backend.state.world_hazard_refs = world
            .objects
            .iter()
            .filter(|object| object.kind == WorldObjectKind::Hazard)
            .map(|object| backend.object_ref(object))
            .collect::<Result<Vec<_>, _>>()?;
        let anchors = backend
            .state
            .creature_anchors
            .iter()
            .map(|anchor| {
                CreatureWorldAnchor::new(
                    anchor.stable_id,
                    Vec3f::new(anchor.tile.x as f32, 0.0, anchor.tile.z as f32),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        for coord in backend.active_chunk_coords(&anchors)? {
            backend.upsert_materialized_chunk(coord, 0)?;
        }
        backend.state.validate()?;
        Ok(backend)
    }

    pub fn to_save_state(&self) -> Result<PersistentVoxelWorldSaveState, ScaffoldContractError> {
        let mut state = self.state.clone();
        state.sorted_materialized_chunks();
        state.validate()?;
        Ok(state)
    }

    pub fn virtual_chunk_count(&self) -> u64 {
        let width = i64::from(self.state.profile_budget.virtual_half_extent_chunks) * 2 + 1;
        (width * width) as u64
    }

    pub const fn allocates_far_chunks(&self) -> bool {
        false
    }

    pub fn apply_tile_edit(&mut self, edit: VoxelTileEdit) -> Result<(), ScaffoldContractError> {
        edit.validate()?;
        let chunk = VoxelChunkCoord::for_tile(self.state.profile_budget.chunk_tile_size, edit.tile);
        let next_generation = self
            .state
            .dirty_regions
            .iter()
            .map(|region| region.generation)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        self.state.saved_edits.push(edit.clone());
        self.state
            .saved_edits
            .sort_by_key(|edit| (edit.tile.x, edit.tile.z));
        self.state.dirty_regions.push(DirtyVoxelRegion {
            chunk,
            min_tile: edit.tile,
            max_tile: edit.tile,
            generation: next_generation,
        });
        self.upsert_materialized_chunk(chunk, next_generation)?;
        self.state.validate()?;
        Ok(())
    }

    pub fn chunk_signature(
        &self,
        coord: VoxelChunkCoord,
    ) -> Result<VoxelChunkSignature, ScaffoldContractError> {
        Ok(self
            .chunk_metadata(coord, self.dirty_generation(coord))?
            .signature)
    }

    pub fn snapshot_for_anchors(
        &self,
        anchors: &[CreatureWorldAnchor],
    ) -> Result<PersistentVoxelWorldSnapshot, ScaffoldContractError> {
        let active_coords = self.active_chunk_coords(anchors)?;
        let mut visible_chunks = Vec::new();
        for coord in active_coords {
            visible_chunks.push(self.chunk_metadata(coord, self.dirty_generation(coord))?);
        }
        visible_chunks.sort_by_key(|chunk| chunk.coord);
        let procedural_config = self.procedural_config();
        let activation = activate_procedural_chunks_around_creatures(procedural_config, anchors)?;
        let content = generate_procedural_world_content(procedural_config, &activation)?;

        let creatures = anchors
            .iter()
            .map(|anchor| {
                let tile = VoxelTileCoord::from(anchor.tile_coord());
                Ok(CreatureVoxelAnchorRef {
                    stable_id: anchor.stable_id,
                    tile,
                    chunk: VoxelChunkCoord::for_tile(
                        self.state.profile_budget.chunk_tile_size,
                        tile,
                    ),
                })
            })
            .collect::<Result<Vec<_>, ScaffoldContractError>>()?;
        let mut resources_and_hazards = content
            .candidates
            .iter()
            .filter_map(|candidate| {
                let kind = match candidate.kind {
                    ProceduralWorldContentKind::Food => VoxelResourceHazardKind::Resource,
                    ProceduralWorldContentKind::Hazard => VoxelResourceHazardKind::Hazard,
                    _ => return None,
                };
                Some(VoxelResourceHazardRef {
                    stable_id: candidate.stable_id,
                    kind,
                    tile: VoxelTileCoord::from(candidate.tile),
                    chunk: VoxelChunkCoord::from(candidate.chunk),
                    resource_bias: candidate.nutrition,
                    hazard_pressure: candidate.hazard_pain,
                })
            })
            .collect::<Vec<_>>();
        resources_and_hazards.sort_by_key(|entry| entry.stable_id.raw());

        let mut field_overlays = Vec::new();
        for chunk in &visible_chunks {
            if chunk.average_resource_bias > 0.0 {
                field_overlays.push(VoxelFieldOverlay {
                    kind: VoxelFieldOverlayKind::Resource,
                    chunk: chunk.coord,
                    strength: chunk.average_resource_bias,
                });
            }
            if chunk.average_hazard_pressure > 0.0 {
                field_overlays.push(VoxelFieldOverlay {
                    kind: VoxelFieldOverlayKind::Hazard,
                    chunk: chunk.coord,
                    strength: chunk.average_hazard_pressure,
                });
            }
        }

        let mut selection_refs = Vec::new();
        for chunk in &visible_chunks {
            selection_refs.push(StableVoxelObjectRef {
                kind: StableVoxelRefKind::Chunk,
                stable_id: None,
                chunk: chunk.coord,
                tile: None,
            });
        }
        for creature in &creatures {
            selection_refs.push(StableVoxelObjectRef {
                kind: StableVoxelRefKind::Creature,
                stable_id: Some(creature.stable_id),
                chunk: creature.chunk,
                tile: Some(creature.tile),
            });
        }
        for entry in &resources_and_hazards {
            selection_refs.push(StableVoxelObjectRef {
                kind: if entry.is_resource() {
                    StableVoxelRefKind::Resource
                } else {
                    StableVoxelRefKind::Hazard
                },
                stable_id: Some(entry.stable_id),
                chunk: entry.chunk,
                tile: Some(entry.tile),
            });
        }

        let chunk_signature_digest = digest_strings(
            visible_chunks
                .iter()
                .map(|chunk| chunk.signature.0.as_str())
                .collect::<Vec<_>>()
                .as_slice(),
        );
        let snapshot = PersistentVoxelWorldSnapshot {
            schema: FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA.to_string(),
            schema_version: FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA_VERSION,
            world_seed: self.state.world_seed,
            profile_id: self.state.profile_id,
            profile_budget: self.state.profile_budget,
            virtual_chunk_count: self.virtual_chunk_count(),
            visible_chunks,
            creatures,
            field_overlays,
            resources_and_hazards,
            dirty_regions: self.state.dirty_regions.clone(),
            selection_refs,
            chunk_signature_digest,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    fn procedural_config(&self) -> ProceduralWorldConfig {
        self.state
            .profile_budget
            .procedural_config(self.state.world_seed)
    }

    fn active_chunk_coords(
        &self,
        anchors: &[CreatureWorldAnchor],
    ) -> Result<Vec<VoxelChunkCoord>, ScaffoldContractError> {
        let activation =
            activate_procedural_chunks_around_creatures(self.procedural_config(), anchors)?;
        let mut coords = activation
            .active_chunks
            .into_iter()
            .map(|chunk| VoxelChunkCoord::from(chunk.coord))
            .collect::<Vec<_>>();
        coords.sort();
        coords.dedup();
        Ok(coords)
    }

    fn upsert_materialized_chunk(
        &mut self,
        coord: VoxelChunkCoord,
        dirty_generation: u64,
    ) -> Result<(), ScaffoldContractError> {
        let metadata = self.chunk_metadata(coord, dirty_generation)?;
        if let Some(existing) = self
            .state
            .materialized_chunks
            .iter_mut()
            .find(|chunk| chunk.coord == coord)
        {
            *existing = metadata;
        } else {
            self.state.materialized_chunks.push(metadata);
        }
        self.state.sorted_materialized_chunks();
        while self.state.materialized_chunks.len()
            > usize::from(self.state.profile_budget.active_chunk_cap)
        {
            self.state.materialized_chunks.pop();
        }
        self.state.materialized_chunk_count = self.state.materialized_chunks.len();
        Ok(())
    }

    fn chunk_metadata(
        &self,
        coord: VoxelChunkCoord,
        dirty_generation: u64,
    ) -> Result<MaterializedVoxelChunkMetadata, ScaffoldContractError> {
        let procedural_config = self.procedural_config();
        let summary = procedural_chunk_summary(procedural_config, coord.into())?;
        let edits = self
            .state
            .saved_edits
            .iter()
            .filter(|edit| {
                VoxelChunkCoord::for_tile(self.state.profile_budget.chunk_tile_size, edit.tile)
                    == coord
            })
            .collect::<Vec<_>>();
        let mut counts = summary
            .material_counts
            .iter()
            .map(|entry| VoxelMaterialCount {
                material: VoxelTerrainMaterialId::from(entry.material),
                count: entry.count,
            })
            .collect::<Vec<_>>();
        for edit in &edits {
            counts.push(VoxelMaterialCount {
                material: edit.material,
                count: 1,
            });
        }
        counts.sort_by_key(|entry| entry.material);
        let dominant_material = edits
            .last()
            .map(|edit| edit.material)
            .unwrap_or_else(|| VoxelTerrainMaterialId::from(summary.dominant_material));
        let average_resource_bias = edits
            .last()
            .and_then(|edit| edit.resource_bias_override)
            .unwrap_or(summary.average_resource_bias);
        let average_hazard_pressure = edits
            .last()
            .and_then(|edit| edit.hazard_pressure_override)
            .unwrap_or(summary.average_hazard_pressure);
        let key = VoxelChunkKey {
            coord,
            chunk_seed: chunk_seed(self.state.world_seed, coord),
        };
        let signature = VoxelChunkSignature(chunk_signature_text(
            self.state.world_seed,
            coord,
            &key,
            dominant_material,
            average_resource_bias,
            average_hazard_pressure,
            edits.as_slice(),
        ));
        Ok(MaterializedVoxelChunkMetadata {
            key,
            coord,
            dominant_material,
            material_counts: counts,
            average_resource_bias,
            average_hazard_pressure,
            tile_count: summary.tile_count,
            signature,
            saved_edit_count: edits.len(),
            dirty_generation,
            resident_profile: self.state.profile_id,
        })
    }

    fn dirty_generation(&self, coord: VoxelChunkCoord) -> u64 {
        self.state
            .dirty_regions
            .iter()
            .filter(|region| region.chunk == coord)
            .map(|region| region.generation)
            .max()
            .unwrap_or(0)
    }

    fn creature_anchor_for_object(
        &self,
        object: &WorldObjectSaveState,
    ) -> Result<CreatureVoxelAnchorRef, ScaffoldContractError> {
        let tile = tile_for_position(object.position);
        Ok(CreatureVoxelAnchorRef {
            stable_id: object.id.validate()?,
            tile,
            chunk: VoxelChunkCoord::for_tile(self.state.profile_budget.chunk_tile_size, tile),
        })
    }

    fn object_ref(
        &self,
        object: &WorldObjectSaveState,
    ) -> Result<WorldObjectVoxelRef, ScaffoldContractError> {
        let tile = tile_for_position(object.position);
        Ok(WorldObjectVoxelRef {
            stable_id: object.id.validate()?,
            kind: object.kind,
            tile,
            chunk: VoxelChunkCoord::for_tile(self.state.profile_budget.chunk_tile_size, tile),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistentVoxelWorldSnapshot {
    pub schema: String,
    pub schema_version: u16,
    pub world_seed: u64,
    pub profile_id: PersistentVoxelProfileId,
    pub profile_budget: PersistentVoxelProfileBudget,
    pub virtual_chunk_count: u64,
    pub visible_chunks: Vec<MaterializedVoxelChunkMetadata>,
    pub creatures: Vec<CreatureVoxelAnchorRef>,
    pub field_overlays: Vec<VoxelFieldOverlay>,
    pub resources_and_hazards: Vec<VoxelResourceHazardRef>,
    pub dirty_regions: Vec<DirtyVoxelRegion>,
    pub selection_refs: Vec<StableVoxelObjectRef>,
    pub chunk_signature_digest: PortableAssetDigest,
}

impl PersistentVoxelWorldSnapshot {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA
            || self.schema_version != FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA_VERSION
            || self.world_seed == 0
            || self.visible_chunks.len() > usize::from(self.profile_budget.active_chunk_cap)
            || self.virtual_chunk_count <= u64::from(self.profile_budget.active_chunk_cap)
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        for reference in &self.selection_refs {
            if !reference.is_stable() {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        Ok(())
    }

    pub fn lookup_tile(&self, tile: VoxelTileCoord) -> Option<StableVoxelObjectRef> {
        let chunk = VoxelChunkCoord::for_tile(self.profile_budget.chunk_tile_size, tile);
        self.visible_chunks
            .iter()
            .any(|visible| visible.coord == chunk)
            .then_some(StableVoxelObjectRef {
                kind: StableVoxelRefKind::Tile,
                stable_id: None,
                chunk,
                tile: Some(tile),
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoxelFieldOverlayKind {
    Resource,
    Hazard,
}

impl VoxelFieldOverlayKind {
    pub const fn is_resource(self) -> bool {
        matches!(self, Self::Resource)
    }

    pub const fn is_hazard(self) -> bool {
        matches!(self, Self::Hazard)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoxelFieldOverlay {
    pub kind: VoxelFieldOverlayKind,
    pub chunk: VoxelChunkCoord,
    pub strength: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoxelResourceHazardKind {
    Resource,
    Hazard,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoxelResourceHazardRef {
    pub stable_id: WorldEntityId,
    pub kind: VoxelResourceHazardKind,
    pub tile: VoxelTileCoord,
    pub chunk: VoxelChunkCoord,
    pub resource_bias: f32,
    pub hazard_pressure: f32,
}

impl VoxelResourceHazardRef {
    pub const fn is_resource(&self) -> bool {
        matches!(self.kind, VoxelResourceHazardKind::Resource)
    }

    pub const fn is_hazard(&self) -> bool {
        matches!(self.kind, VoxelResourceHazardKind::Hazard)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StableVoxelRefKind {
    Chunk,
    Tile,
    Creature,
    Resource,
    Hazard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StableVoxelObjectRef {
    pub kind: StableVoxelRefKind,
    pub stable_id: Option<WorldEntityId>,
    pub chunk: VoxelChunkCoord,
    pub tile: Option<VoxelTileCoord>,
}

impl StableVoxelObjectRef {
    pub fn is_stable(&self) -> bool {
        match self.kind {
            StableVoxelRefKind::Chunk => self.stable_id.is_none() && self.tile.is_none(),
            StableVoxelRefKind::Tile => self.stable_id.is_none() && self.tile.is_some(),
            StableVoxelRefKind::Creature
            | StableVoxelRefKind::Resource
            | StableVoxelRefKind::Hazard => {
                self.stable_id.is_some_and(|id| id.validate().is_ok()) && self.tile.is_some()
            }
        }
    }
}

pub fn migrated_voxel_backend_for_world(
    world: &WorldSaveState,
    profile_id: PersistentVoxelProfileId,
) -> Result<PersistentVoxelWorldSaveState, ScaffoldContractError> {
    PersistentVoxelWorldBackend::from_world_save(world, profile_id)?.to_save_state()
}

fn generator_descriptor(
    world_seed: u64,
    budget: PersistentVoxelProfileBudget,
) -> VoxelGeneratorDescriptor {
    let digest_payload = format!(
        "{}:{}:{}:{}:{}:{}",
        world_seed,
        FVR02_GENERATOR_RULESET_ID,
        FVR02_GENERATOR_RULESET_VERSION,
        budget.chunk_tile_size,
        budget.activation_radius_chunks,
        budget.active_chunk_cap
    );
    VoxelGeneratorDescriptor {
        seed: world_seed,
        ruleset_id: FVR02_GENERATOR_RULESET_ID.to_string(),
        ruleset_version: FVR02_GENERATOR_RULESET_VERSION,
        output_digest: PortableAssetDigest::for_bytes(digest_payload.as_bytes()),
    }
}

fn chunk_seed(world_seed: u64, coord: VoxelChunkCoord) -> u64 {
    let mut value = world_seed
        ^ ((coord.x as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
        ^ ((coord.z as i64 as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9));
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn chunk_signature_text(
    world_seed: u64,
    coord: VoxelChunkCoord,
    key: &VoxelChunkKey,
    dominant_material: VoxelTerrainMaterialId,
    resource_bias: f32,
    hazard_pressure: f32,
    edits: &[&VoxelTileEdit],
) -> String {
    let mut edit_text = edits
        .iter()
        .map(|edit| {
            format!(
                "{}:{}:{:?}:{:?}:{}:{:?}:{:?}",
                edit.tile.x,
                edit.tile.z,
                edit.material,
                edit.biome,
                edit.elevation_delta,
                edit.resource_bias_override,
                edit.hazard_pressure_override
            )
        })
        .collect::<Vec<_>>();
    edit_text.sort();
    let payload = format!(
        "{}:{}:{}:{}:{:?}:{:.3}:{:.3}:{}",
        world_seed,
        coord.x,
        coord.z,
        key.chunk_seed,
        dominant_material,
        resource_bias,
        hazard_pressure,
        edit_text.join("|")
    );
    PortableAssetDigest::for_bytes(payload.as_bytes()).0
}

fn digest_strings(values: &[&str]) -> PortableAssetDigest {
    let mut joined = values.to_vec();
    joined.sort_unstable();
    PortableAssetDigest::for_bytes(joined.join("|").as_bytes())
}

fn tile_for_position(position: Vec3f) -> VoxelTileCoord {
    VoxelTileCoord::new(position.x.round() as i32, position.z.round() as i32)
}

fn validate_unit(value: f32) -> Result<(), ScaffoldContractError> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(ScaffoldContractError::ScalarOutOfRange)
    }
}

fn profile_label(profile_id: PersistentVoxelProfileId) -> &'static str {
    match profile_id {
        PersistentVoxelProfileId::MinimumSettings30x30 => "MinimumSettings30x30",
        PersistentVoxelProfileId::MinSpecComfort1080p => "MinSpecComfort1080p",
        PersistentVoxelProfileId::Balanced1080p => "Balanced1080p",
        PersistentVoxelProfileId::HighSpecScaleUp => "HighSpecScaleUp",
        PersistentVoxelProfileId::ResearchScale => "ResearchScale",
    }
}

fn floor_div(value: i32, divisor: i32) -> i32 {
    let mut quotient = value / divisor;
    let remainder = value % divisor;
    if remainder != 0 && ((remainder > 0) != (divisor > 0)) {
        quotient -= 1;
    }
    quotient
}
