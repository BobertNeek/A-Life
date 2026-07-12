//! Atlas-backed PBR materials for the display-only production terrain.

use std::collections::BTreeMap;

use bevy::{
    image::ImageLoaderSettings,
    prelude::{
        AlphaMode, App, AssetServer, Assets, Color, Handle, Image, Resource, StandardMaterial,
    },
};

use crate::{Fvr03ProductionVoxelMaterialKind, Fvr11TerrainSurfaceRole};

pub(crate) const TERRAIN_ALBEDO_ATLAS_PATH: &str =
    "production_voxel_v1/terrain/terrain_albedo_atlas.png";
pub(crate) const TERRAIN_NORMAL_ATLAS_PATH: &str =
    "production_voxel_v1/terrain/terrain_normal_atlas.png";
pub(crate) const TERRAIN_ORM_ATLAS_PATH: &str = "production_voxel_v1/terrain/terrain_orm_atlas.png";

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ProductionTerrainMaterialSpec {
    pub kind: Fvr03ProductionVoxelMaterialKind,
    pub top_slot: u8,
    pub side_slot: u8,
    pub base_tint: [f32; 4],
    pub perceptual_roughness: f32,
    pub base_color_path: &'static str,
    pub normal_path: &'static str,
    pub orm_path: &'static str,
}

impl ProductionTerrainMaterialSpec {
    pub fn atlas_slot(self, role: Fvr11TerrainSurfaceRole) -> u8 {
        if role == Fvr11TerrainSurfaceRole::Cliff {
            self.side_slot
        } else {
            self.top_slot
        }
    }
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct Fvr11ProductionTerrainMaterialContract {
    pub material_count: usize,
    pub atlas_dimensions: [u32; 2],
    pub base_color_path: &'static str,
    pub normal_path: &'static str,
    pub orm_path: &'static str,
    pub real_assets_requested: bool,
    pub display_only: bool,
}

pub(crate) struct TerrainMaterialLibrary {
    top: BTreeMap<Fvr03ProductionVoxelMaterialKind, Handle<StandardMaterial>>,
    side: BTreeMap<Fvr03ProductionVoxelMaterialKind, Handle<StandardMaterial>>,
    transition: BTreeMap<Fvr03ProductionVoxelMaterialKind, Handle<StandardMaterial>>,
    pub water: Handle<StandardMaterial>,
}

impl TerrainMaterialLibrary {
    pub fn handle_for(
        &self,
        role: Fvr11TerrainSurfaceRole,
        material: Fvr03ProductionVoxelMaterialKind,
    ) -> Handle<StandardMaterial> {
        if role == Fvr11TerrainSurfaceRole::Water {
            return self.water.clone();
        }
        let library = match role {
            Fvr11TerrainSurfaceRole::Top => &self.top,
            Fvr11TerrainSurfaceRole::Cliff => &self.side,
            Fvr11TerrainSurfaceRole::Transition => &self.transition,
            Fvr11TerrainSurfaceRole::Water => unreachable!(),
        };
        library
            .get(&material)
            .unwrap_or_else(|| panic!("missing FVR11 terrain material {role:?}/{material:?}"))
            .clone()
    }
}

#[derive(Clone)]
struct TerrainTextureHandles {
    albedo: Handle<Image>,
    normal: Handle<Image>,
    orm: Handle<Image>,
}

pub(crate) fn production_terrain_material_specs() -> [ProductionTerrainMaterialSpec; 8] {
    let spec = |kind, top_slot, side_slot, base_tint, perceptual_roughness| {
        ProductionTerrainMaterialSpec {
            kind,
            top_slot,
            side_slot,
            base_tint,
            perceptual_roughness,
            base_color_path: TERRAIN_ALBEDO_ATLAS_PATH,
            normal_path: TERRAIN_NORMAL_ATLAS_PATH,
            orm_path: TERRAIN_ORM_ATLAS_PATH,
        }
    };
    [
        spec(
            Fvr03ProductionVoxelMaterialKind::SafeGrass,
            0,
            1,
            [0.99, 1.00, 0.96, 1.0],
            0.86,
        ),
        spec(
            Fvr03ProductionVoxelMaterialKind::Soil,
            2,
            3,
            [1.00, 0.98, 0.94, 1.0],
            0.94,
        ),
        spec(
            Fvr03ProductionVoxelMaterialKind::Resource,
            4,
            5,
            [0.98, 1.00, 0.93, 1.0],
            0.78,
        ),
        spec(
            Fvr03ProductionVoxelMaterialKind::Hazard,
            6,
            7,
            [1.00, 0.95, 0.97, 1.0],
            0.72,
        ),
        spec(
            Fvr03ProductionVoxelMaterialKind::Decay,
            8,
            9,
            [0.98, 0.95, 0.90, 1.0],
            0.90,
        ),
        spec(
            Fvr03ProductionVoxelMaterialKind::Stone,
            10,
            11,
            [0.98, 1.00, 0.97, 1.0],
            0.92,
        ),
        spec(
            Fvr03ProductionVoxelMaterialKind::Water,
            12,
            13,
            [0.94, 1.00, 1.00, 1.0],
            0.18,
        ),
        spec(
            Fvr03ProductionVoxelMaterialKind::Sand,
            14,
            15,
            [1.00, 0.98, 0.92, 1.0],
            0.88,
        ),
    ]
}

pub(crate) fn production_terrain_material_spec(
    kind: Fvr03ProductionVoxelMaterialKind,
) -> ProductionTerrainMaterialSpec {
    production_terrain_material_specs()
        .into_iter()
        .find(|spec| spec.kind == kind)
        .unwrap_or_else(|| panic!("non-terrain material in FVR11 terrain mesh: {kind:?}"))
}

pub(crate) fn create_production_terrain_material_library(app: &mut App) -> TerrainMaterialLibrary {
    let textures = app
        .world()
        .get_resource::<AssetServer>()
        .map(|server| TerrainTextureHandles {
            albedo: server.load(TERRAIN_ALBEDO_ATLAS_PATH),
            normal: server.load_with_settings::<Image, ImageLoaderSettings>(
                TERRAIN_NORMAL_ATLAS_PATH,
                |settings| settings.is_srgb = false,
            ),
            orm: server.load_with_settings::<Image, ImageLoaderSettings>(
                TERRAIN_ORM_ATLAS_PATH,
                |settings| settings.is_srgb = false,
            ),
        });
    let real_assets_requested = textures.is_some();
    let specs = production_terrain_material_specs();
    let mut top = BTreeMap::new();
    let mut side = BTreeMap::new();
    let mut transition = BTreeMap::new();
    let water;
    {
        let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
        for spec in specs {
            top.insert(
                spec.kind,
                materials.add(opaque_terrain_material(spec, textures.as_ref(), 1.0)),
            );
            side.insert(
                spec.kind,
                materials.add(opaque_terrain_material(spec, textures.as_ref(), 0.96)),
            );
            transition.insert(
                spec.kind,
                materials.add(opaque_terrain_material(spec, textures.as_ref(), 1.0)),
            );
        }
        let water_spec = specs
            .iter()
            .find(|spec| spec.kind == Fvr03ProductionVoxelMaterialKind::Water)
            .copied()
            .expect("water terrain material spec");
        water = materials.add(animated_water_material(water_spec, textures.as_ref()));
    }
    app.insert_resource(Fvr11ProductionTerrainMaterialContract {
        material_count: specs.len(),
        atlas_dimensions: [272, 272],
        base_color_path: TERRAIN_ALBEDO_ATLAS_PATH,
        normal_path: TERRAIN_NORMAL_ATLAS_PATH,
        orm_path: TERRAIN_ORM_ATLAS_PATH,
        real_assets_requested,
        display_only: true,
    });
    TerrainMaterialLibrary {
        top,
        side,
        transition,
        water,
    }
}

fn opaque_terrain_material(
    spec: ProductionTerrainMaterialSpec,
    textures: Option<&TerrainTextureHandles>,
    role_tint: f32,
) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgba(
            spec.base_tint[0] * role_tint,
            spec.base_tint[1] * role_tint,
            spec.base_tint[2] * role_tint,
            1.0,
        ),
        base_color_texture: textures.map(|textures| textures.albedo.clone()),
        normal_map_texture: textures.map(|textures| textures.normal.clone()),
        metallic_roughness_texture: textures.map(|textures| textures.orm.clone()),
        occlusion_texture: textures.map(|textures| textures.orm.clone()),
        perceptual_roughness: spec.perceptual_roughness,
        metallic: 0.0,
        unlit: false,
        cull_mode: None,
        alpha_mode: AlphaMode::Opaque,
        ..Default::default()
    }
}

fn animated_water_material(
    spec: ProductionTerrainMaterialSpec,
    textures: Option<&TerrainTextureHandles>,
) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgba(
            spec.base_tint[0],
            spec.base_tint[1],
            spec.base_tint[2],
            0.78,
        ),
        base_color_texture: textures.map(|textures| textures.albedo.clone()),
        normal_map_texture: textures.map(|textures| textures.normal.clone()),
        metallic_roughness_texture: textures.map(|textures| textures.orm.clone()),
        occlusion_texture: textures.map(|textures| textures.orm.clone()),
        perceptual_roughness: 0.18,
        reflectance: 0.42,
        metallic: 0.0,
        clearcoat: 0.35,
        clearcoat_perceptual_roughness: 0.12,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        unlit: false,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn all_terrain_materials_have_unique_top_and_side_atlas_slots() {
        let specs = production_terrain_material_specs();
        assert_eq!(specs.len(), 8);
        assert_eq!(
            specs
                .iter()
                .map(|spec| spec.top_slot)
                .collect::<BTreeSet<_>>()
                .len(),
            8
        );
        assert_eq!(
            specs
                .iter()
                .map(|spec| spec.side_slot)
                .collect::<BTreeSet<_>>()
                .len(),
            8
        );
        for spec in specs {
            assert!(spec
                .base_color_path
                .starts_with("production_voxel_v1/terrain/"));
            assert!(spec.normal_path.starts_with("production_voxel_v1/terrain/"));
            assert!(spec.orm_path.starts_with("production_voxel_v1/terrain/"));
        }
    }

    #[test]
    fn transition_layers_use_surface_texture_while_cliffs_keep_rooted_side_texture() {
        for spec in production_terrain_material_specs() {
            assert_eq!(
                spec.atlas_slot(Fvr11TerrainSurfaceRole::Transition),
                spec.top_slot
            );
            assert_eq!(
                spec.atlas_slot(Fvr11TerrainSurfaceRole::Cliff),
                spec.side_slot
            );
        }
    }
}
