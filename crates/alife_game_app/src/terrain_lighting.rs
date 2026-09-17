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

pub(crate) const PRODUCTION_CAMERA_MAX_ZOOM: f32 = 3.5;
pub(crate) const PRODUCTION_SHADOW_MINIMUM_DISTANCE: f32 = 0.1;
/// Covers local terrain relief and ordinary tall props above the camera focus.
const PRODUCTION_SHADOW_RELIEF_HEIGHT: f32 = 2.0;
const PRODUCTION_SHADOW_DEPTH_MARGIN: f32 = 0.25;
const PRODUCTION_CAMERA_FOCUS: Vec3 = Vec3::new(2.5, 0.45, 0.75);

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
    let base_distance = match settings.shadow_quality {
        "high" => 90.0,
        "medium" | "adaptive" => 72.0,
        _ => 56.0,
    };
    production_shadow_maximum_camera_distance(
        production_camera_extent(settings.profile_id),
        base_distance,
    )
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
            clear_color: ClearColorConfig::Custom(Color::srgb(0.52, 0.72, 0.82)),
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
            color: Color::srgb(0.72, 0.81, 0.92),
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
    production_shadow_safe_camera_transform(
        production_initial_camera_transform(mode, extent),
        PRODUCTION_CAMERA_FOCUS,
        extent,
    )
}

pub(crate) fn production_follow_camera_transform(
    mode: Fvr03ProductionVoxelCameraMode,
    extent: f32,
    target: Vec3,
) -> Transform {
    let focus = target + Vec3::Y * 0.70;
    production_shadow_safe_camera_transform(
        production_follow_camera_transform_unbacked(mode, extent, focus),
        focus,
        extent,
    )
}

fn production_initial_camera_transform(
    mode: Fvr03ProductionVoxelCameraMode,
    extent: f32,
) -> Transform {
    match mode {
        Fvr03ProductionVoxelCameraMode::OrthographicIsometric => {
            Transform::from_xyz(extent * 0.50, extent * 0.72, extent * 0.84)
                .looking_at(PRODUCTION_CAMERA_FOCUS, Vec3::Y)
        }
        Fvr03ProductionVoxelCameraMode::Orbit => {
            Transform::from_xyz(extent * 0.70, extent * 0.55, extent * 0.92)
                .looking_at(PRODUCTION_CAMERA_FOCUS, Vec3::Y)
        }
    }
}

fn production_follow_camera_transform_unbacked(
    mode: Fvr03ProductionVoxelCameraMode,
    extent: f32,
    focus: Vec3,
) -> Transform {
    let offset = match mode {
        Fvr03ProductionVoxelCameraMode::OrthographicIsometric => {
            Vec3::new(extent * 0.44, extent * 0.38, extent * 0.72)
        }
        Fvr03ProductionVoxelCameraMode::Orbit => {
            Vec3::new(extent * 0.72, extent * 0.52, extent * 0.94)
        }
    };
    Transform::from_translation(focus + offset).looking_at(focus, Vec3::Y)
}

fn production_shadow_safe_camera_transform(
    mut transform: Transform,
    focus: Vec3,
    extent: f32,
) -> Transform {
    let camera_forward = transform.rotation * Vec3::NEG_Z;
    let backoff = production_shadow_camera_backoff(transform, focus, extent);
    transform.translation -= camera_forward * backoff;
    transform
}

fn production_shadow_camera_backoff(transform: Transform, focus: Vec3, extent: f32) -> f32 {
    let camera_forward = transform.rotation * Vec3::NEG_Z;
    if camera_forward.y >= -f32::EPSILON {
        return 0.0;
    }
    let camera_up = transform.rotation * Vec3::Y;
    let max_half_height = extent * PRODUCTION_CAMERA_MAX_ZOOM * 0.5;
    let receiver_y = focus.y + PRODUCTION_SHADOW_RELIEF_HEIGHT;
    let nearest_receiver_depth =
        (receiver_y - transform.translation.y + camera_up.y * max_half_height) / camera_forward.y;
    (PRODUCTION_SHADOW_MINIMUM_DISTANCE + PRODUCTION_SHADOW_DEPTH_MARGIN - nearest_receiver_depth)
        .max(0.0)
}

fn production_shadow_maximum_camera_distance(extent: f32, base_distance: f32) -> f32 {
    let mut maximum_distance = base_distance;
    for mode in [
        Fvr03ProductionVoxelCameraMode::OrthographicIsometric,
        Fvr03ProductionVoxelCameraMode::Orbit,
    ] {
        let initial = production_initial_camera_transform(mode, extent);
        maximum_distance = maximum_distance.max(production_shadow_camera_coverage(
            initial,
            PRODUCTION_CAMERA_FOCUS,
            extent,
            base_distance,
        ));

        let follow_focus = Vec3::Y * 0.70;
        let follow = production_follow_camera_transform_unbacked(mode, extent, follow_focus);
        maximum_distance = maximum_distance.max(production_shadow_camera_coverage(
            follow,
            follow_focus,
            extent,
            base_distance,
        ));
    }
    maximum_distance
}

fn production_shadow_camera_coverage(
    transform: Transform,
    focus: Vec3,
    extent: f32,
    base_distance: f32,
) -> f32 {
    let backoff = production_shadow_camera_backoff(transform, focus, extent);
    let mut safe = transform;
    safe.translation -= (safe.rotation * Vec3::NEG_Z) * backoff;
    let far_receiver_depth = production_shadow_receiver_depth(
        safe,
        focus.y - PRODUCTION_SHADOW_RELIEF_HEIGHT,
        extent * PRODUCTION_CAMERA_MAX_ZOOM * 0.5,
    );
    (base_distance + backoff).max(far_receiver_depth)
}

fn production_shadow_receiver_depth(transform: Transform, receiver_y: f32, screen_y: f32) -> f32 {
    let forward = transform.rotation * Vec3::NEG_Z;
    let up = transform.rotation * Vec3::Y;
    (receiver_y - transform.translation.y - up.y * screen_y) / forward.y
}

pub(crate) fn production_camera_extent(profile_id: ProductionFrontendProfileId) -> f32 {
    match profile_id {
        ProductionFrontendProfileId::MinimumSettings30x30 => 17.2,
        ProductionFrontendProfileId::MinSpecComfort1080p => 9.8,
        ProductionFrontendProfileId::Balanced1080p => 30.0,
        ProductionFrontendProfileId::HighSpecScaleUp => 40.0,
        ProductionFrontendProfileId::ResearchScale => 34.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadow_safe_camera_backoff_covers_zoom_limits_without_changing_view_plane() {
        let follow_target = Vec3::new(4.0, 0.0, -3.0);

        for profile_id in ProductionFrontendProfileId::all() {
            let settings = Fvr03ProductionVoxelRendererSettings::for_profile(*profile_id);
            let extent = production_camera_extent(settings.profile_id);
            let shadow_maximum_distance = production_shadow_maximum_distance(&settings);

            for mode in [
                Fvr03ProductionVoxelCameraMode::OrthographicIsometric,
                Fvr03ProductionVoxelCameraMode::Orbit,
            ] {
                let initial = production_initial_camera_transform(mode, extent);
                let initial_safe = production_camera_transform(mode, extent);
                let follow_focus = follow_target + Vec3::Y * 0.70;
                let follow =
                    production_follow_camera_transform_unbacked(mode, extent, follow_focus);
                let follow_safe = production_follow_camera_transform(mode, extent, follow_target);

                for (before, after, focus) in [
                    (initial, initial_safe, PRODUCTION_CAMERA_FOCUS),
                    (follow, follow_safe, follow_focus),
                ] {
                    assert_eq!(before.rotation, after.rotation);
                    let before_view = before.rotation.conjugate() * (focus - before.translation);
                    let after_view = after.rotation.conjugate() * (focus - after.translation);
                    assert!((before_view.x - after_view.x).abs() < 1e-5);
                    assert!((before_view.y - after_view.y).abs() < 1e-5);

                    for scale in [0.35, PRODUCTION_CAMERA_MAX_ZOOM] {
                        let half_height = extent * scale * 0.5;
                        let near = production_shadow_receiver_depth(
                            after,
                            focus.y + PRODUCTION_SHADOW_RELIEF_HEIGHT,
                            -half_height,
                        );
                        let far = production_shadow_receiver_depth(
                            after,
                            focus.y - PRODUCTION_SHADOW_RELIEF_HEIGHT,
                            half_height,
                        );
                        assert!(near >= PRODUCTION_SHADOW_MINIMUM_DISTANCE - 1e-5);
                        assert!(far <= shadow_maximum_distance + 1e-5);
                    }
                }
            }
        }
    }
}
