//! Creature-stage camera and atmosphere.

use alife_world::VoxelTileCoord;
use bevy::{
    camera::ScalingMode,
    core_pipeline::tonemapping::Tonemapping,
    prelude::{
        default, AmbientLight, App, Camera, Camera3d, ClearColorConfig, Color, Component,
        DistanceFog, FogFalloff, Name, OrthographicProjection, Projection, Transform, Vec3,
    },
    render::view::Msaa,
};

use crate::{
    Fvr03ProductionVoxelCamera, Fvr03ProductionVoxelCameraMode,
    Fvr03ProductionVoxelRendererSettings, ProductionFrontendProfileId,
};

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Fvr11ProductionTerrainLightingMarker {
    pub tonemapping: &'static str,
    pub directional_shadows: bool,
    pub shadow_cascades: u8,
    pub distance_fog: bool,
    pub cool_ambient_fill: bool,
    pub contact_grounding: bool,
    pub display_only: bool,
    pub no_renderer_authority_over_world_actions_or_cognition: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Fvr11ProductionContactShadow {
    pub source_kind: &'static str,
    pub tile: VoxelTileCoord,
    pub stable_id: Option<alife_core::WorldEntityId>,
    pub display_only: bool,
    pub no_renderer_authority_over_world_actions_or_cognition: bool,
}

pub(crate) fn production_shadow_cascade_count(
    settings: &Fvr03ProductionVoxelRendererSettings,
) -> usize {
    match settings.shadow_quality {
        "low" => 0,
        "high" => 2,
        _ => 1,
    }
}

pub(crate) fn production_shadow_maximum_distance(
    settings: &Fvr03ProductionVoxelRendererSettings,
) -> f32 {
    match settings.shadow_quality {
        "high" => 90.0,
        "medium" | "adaptive" => 72.0,
        _ => 56.0,
    }
}

pub(crate) fn spawn_production_terrain_camera(
    app: &mut App,
    settings: &Fvr03ProductionVoxelRendererSettings,
) {
    let camera_extent = production_camera_extent(settings.profile_id);
    let shadow_cascades = production_shadow_cascade_count(settings);
    let directional_shadows = shadow_cascades > 0;
    let fog_alpha = if settings.minimum_floor { 0.10 } else { 0.22 };
    let fog_start = if settings.minimum_floor { 44.0 } else { 38.0 };
    let fog_end = if settings.minimum_floor { 98.0 } else { 92.0 };
    app.world_mut().spawn((
        Name::new("A-Life FVR11 creature-stage terrain camera"),
        Camera3d::default(),
        Camera {
            order: 0,
            clear_color: ClearColorConfig::Custom(Color::srgb(0.075, 0.165, 0.145)),
            ..default()
        },
        Projection::from(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: camera_extent,
            },
            scale: 1.0,
            near: -200.0,
            far: 500.0,
            ..OrthographicProjection::default_3d()
        }),
        Tonemapping::TonyMcMapface,
        Msaa::Off,
        AmbientLight {
            color: Color::srgb(0.73, 0.76, 0.70),
            brightness: if settings.minimum_floor { 520.0 } else { 760.0 },
            affects_lightmapped_meshes: true,
        },
        DistanceFog {
            color: Color::srgba(0.22, 0.34, 0.25, fog_alpha),
            directional_light_color: Color::srgba(1.0, 0.74, 0.42, 0.28),
            directional_light_exponent: 14.0,
            falloff: FogFalloff::Linear {
                start: fog_start,
                end: fog_end,
            },
        },
        production_camera_transform(
            Fvr03ProductionVoxelCameraMode::OrthographicIsometric,
            camera_extent,
        ),
        Fvr03ProductionVoxelCamera {
            mode: Fvr03ProductionVoxelCameraMode::OrthographicIsometric,
        },
        Fvr11ProductionTerrainLightingMarker {
            tonemapping: "tony-mc-mapface",
            directional_shadows,
            shadow_cascades: shadow_cascades as u8,
            distance_fog: true,
            cool_ambient_fill: true,
            contact_grounding: true,
            display_only: true,
            no_renderer_authority_over_world_actions_or_cognition: true,
        },
    ));
}

pub(crate) fn production_camera_transform(
    mode: Fvr03ProductionVoxelCameraMode,
    extent: f32,
) -> Transform {
    match mode {
        Fvr03ProductionVoxelCameraMode::OrthographicIsometric => {
            Transform::from_xyz(extent * 0.50, extent * 0.72, extent * 0.84)
                .looking_at(Vec3::new(2.5, 0.45, 0.75), Vec3::Y)
        }
        Fvr03ProductionVoxelCameraMode::Orbit => {
            Transform::from_xyz(extent * 0.70, extent * 0.55, extent * 0.92)
                .looking_at(Vec3::new(2.5, 0.45, 0.75), Vec3::Y)
        }
    }
}

pub(crate) fn production_camera_extent(profile_id: ProductionFrontendProfileId) -> f32 {
    match profile_id {
        ProductionFrontendProfileId::MinimumSettings30x30 => 17.2,
        ProductionFrontendProfileId::MinSpecComfort1080p => 15.8,
        ProductionFrontendProfileId::Balanced1080p => 30.0,
        ProductionFrontendProfileId::HighSpecScaleUp => 40.0,
        ProductionFrontendProfileId::ResearchScale => 34.0,
    }
}
