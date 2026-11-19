//! Palette and camera-relative air for the approved island, not a global profile override.
use super::*;
use std::sync::OnceLock;

#[derive(serde::Deserialize)]
struct Palette {
    sun_ray_direction: [f32; 3],
    sun_srgb: [f32; 3],
    sky_fill_srgb: [f32; 3],
    clear_srgb: [f32; 3],
    air_srgb: [f32; 3],
    sun_air_srgb: [f32; 3],
    air_strength: f32,
    air_start_beyond_focus_m: f32,
    air_span_min_m: f32,
}

fn palette() -> &'static Palette {
    static P: OnceLock<Palette> = OnceLock::new();
    P.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../assets/landscape/island/coastal-daylight.json"
        ))
            .expect("approved island daylight")
    })
}

fn rgba([r, g, b]: [f32; 3], alpha: f32) -> Color {
    Color::srgba(r, g, b, alpha)
}

#[derive(Component)]
pub(in crate::production_voxel_renderer) struct CoastalAir;

#[derive(Default, Resource)]
struct PreviousLight {
    cameras: Vec<(Entity, bevy::camera::ClearColorConfig, Color, DistanceFog)>,
    suns: Vec<(Entity, Color, Quat)>,
}

pub(super) fn enter(world: &mut World) {
    if world.contains_resource::<PreviousLight>() {
        return;
    }
    let p = palette();
    let mut saved = PreviousLight::default();
    let mut cameras = world.query_filtered::<
        (Entity, &mut Camera, &mut AmbientLight, &mut DistanceFog),
        With<Fvr03ProductionVoxelCamera>,
    >();
    for (id, mut camera, mut fill, mut fog) in cameras.iter_mut(world) {
        saved.cameras.push((id, camera.clear_color.clone(), fill.color, fog.clone()));
        let old_alpha = fog.color.to_srgba().alpha;
        camera.clear_color = bevy::camera::ClearColorConfig::Custom(rgba(p.clear_srgb, 1.0));
        fill.color = rgba(p.sky_fill_srgb, 1.0);
        fog.color = rgba(p.air_srgb, (old_alpha / 0.22 * p.air_strength).min(0.45));
        fog.directional_light_color = rgba(p.sun_air_srgb, 0.18);
        fog.directional_light_exponent = 10.0;
    }
    for (id, ..) in &saved.cameras {
        world.entity_mut(*id).insert(CoastalAir);
    }
    let mut suns = world
        .query_filtered::<(Entity, &mut DirectionalLight, &mut Transform), With<TerrainSun>>();
    let rotation = Transform::default()
        .looking_to(Vec3::from_array(p.sun_ray_direction), Vec3::Y)
        .rotation;
    for (id, mut light, mut transform) in suns.iter_mut(world) {
        saved.suns.push((id, light.color, transform.rotation));
        light.color = rgba(p.sun_srgb, 1.0);
        transform.rotation = rotation;
    }
    world.insert_resource(saved);
}

pub(super) fn leave(world: &mut World) {
    let Some(saved) = world.remove_resource::<PreviousLight>() else {
        return;
    };
    for (id, clear, color, old_fog) in saved.cameras {
        if let Ok(mut e) = world.get_entity_mut(id) {
            e.remove::<CoastalAir>();
            if let Some(mut c) = e.get_mut::<Camera>() {
                c.clear_color = clear;
            }
            if let Some(mut c) = e.get_mut::<AmbientLight>() {
                c.color = color;
            }
            if let Some(mut c) = e.get_mut::<DistanceFog>() {
                *c = old_fog;
            }
        }
    }
    for (id, color, rotation) in saved.suns {
        if let Ok(mut e) = world.get_entity_mut(id) {
            if let Some(mut c) = e.get_mut::<DirectionalLight>() {
                c.color = color;
            }
            if let Some(mut c) = e.get_mut::<Transform>() {
                c.rotation = rotation;
            }
        }
    }
}

fn range(distance: f32, span: f32) -> (f32, f32) {
    let p = palette();
    let start = distance.max(0.0) + p.air_start_beyond_focus_m;
    (start, start + p.air_span_min_m + span.clamp(0.0, 1200.0) * 0.62)
}

pub(super) fn refocus(fog: &mut Mut<DistanceFog>, distance: f32, span: f32) {
    let (start, end) = range(distance, span);
    if matches!(fog.falloff, FogFalloff::Linear { start: old, end: far } if (old - start).abs() < 0.35 && (far - end).abs() < 0.35) {
        return;
    }
    fog.falloff = FogFalloff::Linear { start, end };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entering_twice_and_leaving_restores_the_previous_scene() {
        let mut world = World::new();
        let fill = Color::srgb(0.19, 0.44, 0.69);
        let mist = Color::srgba(0.13, 0.47, 0.26, 0.22);
        let sun = world.spawn((
            TerrainSun { enabled: true, cascades: 2 },
            DirectionalLight { color: fill, ..default() },
            Transform::from_rotation(Quat::from_rotation_y(0.61)),
        )).id();
        let view = world.spawn((
            Fvr03ProductionVoxelCamera { mode: Fvr03ProductionVoxelCameraMode::OrthographicIsometric },
            Camera { clear_color: bevy::camera::ClearColorConfig::None, ..default() },
            AmbientLight { color: fill, ..default() },
            DistanceFog { color: mist, ..default() },
        )).id();
        enter(&mut world);
        enter(&mut world);
        assert_ne!(world.get::<AmbientLight>(view).unwrap().color, fill);
        assert!(world.get::<CoastalAir>(view).is_some());
        assert_ne!(world.get::<DirectionalLight>(sun).unwrap().color, fill);
        leave(&mut world);
        assert!(world.get::<CoastalAir>(view).is_none());
        assert!(matches!(world.get::<Camera>(view).unwrap().clear_color, bevy::camera::ClearColorConfig::None));
        assert_eq!(world.get::<AmbientLight>(view).unwrap().color, fill);
        assert_eq!(world.get::<DistanceFog>(view).unwrap().color, mist);
        assert_eq!(world.get::<DirectionalLight>(sun).unwrap().color, fill);
        assert_eq!(world.get::<Transform>(sun).unwrap().rotation, Quat::from_rotation_y(0.61));
    }

    #[test]
    fn island_rebuild_rebinds_the_new_scene_sun_and_keeps_the_camera() {
        let mut world = World::new();
        world.insert_resource(creature_grounding::SelectedTerrain(alife_world::island_terrain()));
        let camera = world.spawn((
            Fvr03ProductionVoxelCamera { mode: Fvr03ProductionVoxelCameraMode::OrthographicIsometric },
            Camera::default(), AmbientLight::default(), DistanceFog::default(),
        )).id();
        let lighting = || Fvr04PreparedLighting {
            directional_shadows: true, shadow_cascades: 2, shadow_maximum_distance: 180.0,
            contact_shadow_mesh: None, contact_shadow_material: None, contact_shadows: vec![],
        };
        for _ in 0..2 {
            spawn_fvr04_prepared_lighting(&mut world, lighting());
            super::super::start(&mut world);
            assert!(world.get::<CoastalAir>(camera).is_some());
            let mut suns = world.query_filtered::<(Entity, &DirectionalLight), With<TerrainSun>>();
            let (id, light) = suns.single(&world).unwrap();
            assert_eq!(light.color, rgba(palette().sun_srgb, 1.0));
            world.despawn(id);
        }
        super::super::stop(&mut world);
        assert!(world.get::<CoastalAir>(camera).is_none());
    }

    #[test]
    fn focus_never_sits_inside_the_far_veil_when_an_orthographic_camera_backs_off() {
        for (distance, span) in [(18.0, 12.0), (270.0, 88.0), (1100.0, 760.0)] {
            let (start, end) = range(distance, span);
            assert!(distance < start && start + 100.0 < end);
        }
    }
}
