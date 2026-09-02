//! Creature-stage camera, atmosphere, and display-only grounding.

use std::collections::BTreeMap;

use alife_world::VoxelTileCoord;
use bevy::{
    asset::RenderAssetUsages,
    camera::ScalingMode,
    core_pipeline::tonemapping::Tonemapping,
    light::CascadeShadowConfigBuilder,
    mesh::Indices,
    picking::Pickable,
    prelude::{
        default, AlphaMode, AmbientLight, App, Assets, Camera, Camera3d, ClearColorConfig, Color,
        Component, DirectionalLight, DistanceFog, FogFalloff, Mesh, Mesh3d, MeshMaterial3d, Name,
        OrthographicProjection, Projection, Quat, StandardMaterial, Transform, Vec3,
    },
    render::{render_resource::PrimitiveTopology, view::Msaa},
};

use crate::{
    Fvr03ProductionVoxelCamera, Fvr03ProductionVoxelCameraMode, Fvr03ProductionVoxelCreatureMarker,
    Fvr03ProductionVoxelRendererSettings, Fvr07ProductionVisualDressing,
    Fvr09CuteBipedCreatureMarker, ProductionFrontendProfileId,
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

pub(crate) fn spawn_production_terrain_lighting(
    app: &mut App,
    settings: &Fvr03ProductionVoxelRendererSettings,
    tile_heights: &BTreeMap<VoxelTileCoord, f32>,
) {
    let shadow_cascades = production_shadow_cascade_count(settings);
    let directional_shadows = shadow_cascades > 0;
    let light = DirectionalLight {
        color: Color::srgb(1.0, 0.86, 0.66),
        illuminance: 5800.0,
        shadows_enabled: directional_shadows,
        ..default()
    };
    let transform = Transform::from_rotation(Quat::from_euler(
        bevy::prelude::EulerRot::XYZ,
        -1.05,
        0.62,
        -0.42,
    ));
    if directional_shadows {
        app.world_mut().spawn((
            Name::new(format!(
                "A-Life FVR11 warm {shadow_cascades}-cascade directional sun"
            )),
            light,
            CascadeShadowConfigBuilder {
                num_cascades: shadow_cascades,
                minimum_distance: 0.1,
                maximum_distance: production_shadow_maximum_distance(settings),
                first_cascade_far_bound: 28.0,
                overlap_proportion: 0.18,
            }
            .build(),
            transform,
        ));
    } else {
        app.world_mut().spawn((
            Name::new("A-Life FVR11 minimum-profile warm directional sun"),
            light,
            transform,
        ));
        spawn_minimum_contact_shadows(app, tile_heights);
    }
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

fn spawn_minimum_contact_shadows(app: &mut App, tile_heights: &BTreeMap<VoxelTileCoord, f32>) {
    let creature_shadows = {
        let mut query = app.world_mut().query::<(
            &Fvr09CuteBipedCreatureMarker,
            &Fvr03ProductionVoxelCreatureMarker,
            &Transform,
        )>();
        query
            .iter(app.world())
            .map(|(_, marker, transform)| (marker.tile, transform.translation, 1.0_f32, "creature"))
            .collect::<Vec<_>>()
    };
    let dressing_shadows = {
        let mut query = app
            .world_mut()
            .query::<(&Fvr07ProductionVisualDressing, &Transform)>();
        query
            .iter(app.world())
            .filter(|(_, transform)| transform.scale.y >= 1.0)
            .map(|(marker, transform)| (marker.tile, transform.translation, 0.78_f32, "dressing"))
            .collect::<Vec<_>>()
    };
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(contact_shadow_mesh());
    let material = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color: Color::srgba(0.055, 0.075, 0.038, 0.24),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 1.0,
            cull_mode: None,
            unlit: true,
            ..default()
        });
    for (tile, translation, scale, source_kind) in
        creature_shadows.into_iter().chain(dressing_shadows)
    {
        let y = tile_heights.get(&tile).copied().unwrap_or(0.0) + 0.018;
        app.world_mut().spawn((
            Name::new(format!(
                "A-Life FVR11 minimum contact shadow {source_kind} {}:{}",
                tile.x, tile.z
            )),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(Vec3::new(translation.x, y, translation.z))
                .with_scale(Vec3::splat(scale)),
            bevy::light::NotShadowCaster,
            Pickable::IGNORE,
            Fvr11ProductionContactShadow {
                source_kind,
                tile,
                display_only: true,
                no_renderer_authority_over_world_actions_or_cognition: true,
            },
        ));
    }
}

fn contact_shadow_mesh() -> Mesh {
    const SEGMENTS: u32 = 12;
    const RADIUS: f32 = 0.36;
    let mut positions = Vec::with_capacity((SEGMENTS + 1) as usize);
    let mut normals = Vec::with_capacity((SEGMENTS + 1) as usize);
    let mut uvs = Vec::with_capacity((SEGMENTS + 1) as usize);
    let mut indices = Vec::with_capacity((SEGMENTS * 3) as usize);
    positions.push([0.0, 0.0, 0.0]);
    normals.push([0.0, 1.0, 0.0]);
    uvs.push([0.5, 0.5]);
    for index in 0..SEGMENTS {
        let angle = index as f32 * std::f32::consts::TAU / SEGMENTS as f32;
        let x = angle.cos() * RADIUS;
        let z = angle.sin() * RADIUS;
        positions.push([x, 0.0, z]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([x / (RADIUS * 2.0) + 0.5, z / (RADIUS * 2.0) + 0.5]);
    }
    for index in 0..SEGMENTS {
        indices.extend([0, index + 1, (index + 1) % SEGMENTS + 1]);
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}
