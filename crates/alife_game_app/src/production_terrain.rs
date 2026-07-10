//! Display-only contracts shared by the production terrain presentation modules.

use std::collections::BTreeMap;

use alife_world::VoxelTileCoord;
use bevy::prelude::{Component, Resource};

use crate::Fvr03ProductionVoxelMaterialKind;

pub const FVR11_PRODUCTION_TERRAIN_VISUAL_VERSION: &str = "fvr11-lush-creature-stage-terrain-v1";

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ProductionTerrainSample {
    pub tile: VoxelTileCoord,
    pub material: Fvr03ProductionVoxelMaterialKind,
    pub center_x: f32,
    pub center_z: f32,
    pub height: f32,
    pub resource_bias: f32,
    pub hazard_pressure: f32,
    pub visual_bucket: u8,
}

pub(crate) type ProductionTerrainSampleMap = BTreeMap<VoxelTileCoord, ProductionTerrainSample>;

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub(crate) struct TerrainAtlasUvRect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct TerrainAtlasLayout {
    pub tile_size: u32,
    pub gutter: u32,
    pub columns: u32,
    pub rows: u32,
}

impl TerrainAtlasLayout {
    #[allow(dead_code)]
    pub const PRODUCTION: Self = Self {
        tile_size: 64,
        gutter: 2,
        columns: 4,
        rows: 4,
    };

    #[allow(dead_code)]
    pub fn slot_rect(self, slot: u8) -> TerrainAtlasUvRect {
        assert!(u32::from(slot) < self.columns * self.rows);
        let cell = self.tile_size + self.gutter * 2;
        let atlas_width = self.columns * cell;
        let atlas_height = self.rows * cell;
        let column = u32::from(slot) % self.columns;
        let row = u32::from(slot) / self.columns;
        let x = column * cell + self.gutter;
        let y = row * cell + self.gutter;
        TerrainAtlasUvRect {
            min: [
                x as f32 / atlas_width as f32,
                y as f32 / atlas_height as f32,
            ],
            max: [
                (x + self.tile_size) as f32 / atlas_width as f32,
                (y + self.tile_size) as f32 / atlas_height as f32,
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fvr11TerrainSurfaceRole {
    Top,
    Cliff,
    Transition,
    Water,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Fvr11ProductionTerrainLayer {
    pub role: Fvr11TerrainSurfaceRole,
    pub material: Fvr03ProductionVoxelMaterialKind,
    pub source_tile_count: usize,
    pub display_only: bool,
    pub no_renderer_authority_over_world_actions_or_cognition: bool,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct Fvr11ProductionTerrainSceneResource {
    pub visual_version: &'static str,
    pub sample_count: usize,
    pub top_layer_count: usize,
    pub cliff_layer_count: usize,
    pub transition_edge_count: usize,
    pub water_layer_count: usize,
    pub confetti_detail_quad_count: usize,
    pub display_only: bool,
    pub no_renderer_authority_over_world_actions_or_cognition: bool,
}

#[cfg(test)]
mod tests {
    use super::TerrainAtlasLayout;

    #[test]
    fn production_atlas_slots_exclude_the_two_pixel_gutter() {
        let layout = TerrainAtlasLayout::PRODUCTION;
        let first = layout.slot_rect(0);
        let last = layout.slot_rect(15);

        assert!(first.min[0] > 0.0);
        assert!(first.min[1] > 0.0);
        assert!(last.max[0] < 1.0);
        assert!(last.max[1] < 1.0);
    }
}
