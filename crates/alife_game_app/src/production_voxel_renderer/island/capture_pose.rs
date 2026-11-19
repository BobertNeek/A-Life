//! Opt-in camera repeatability for measured native A/Bs. No ordinary-play system or UI.
use super::*;

#[derive(serde::Deserialize, Resource)]
struct Receipt {
    terrain_binding: alife_world::TerrainBinding,
    camera: Pose,
}

#[derive(serde::Deserialize)]
struct Pose {
    position: [f32; 3],
    rotation: [f32; 4],
    view_height_m: f32,
}

impl Pose {
    fn valid(&self) -> bool {
        let q = Quat::from_array(self.rotation);
        self.position
            .iter()
            .all(|v| v.is_finite() && v.abs() < 10000.0)
            && q.is_finite()
            && (q.length_squared() - 1.0).abs() < 0.0001
            && (q * Vec3::NEG_Z).y < -0.05
            && (camera_navigation::ENCOUNTER_HEIGHT - 0.001..=1600.001)
                .contains(&self.view_height_m)
    }
}

pub(in crate::production_voxel_renderer) fn install(app: &mut App) {
    let Some(path) = std::env::var_os("ALIFE_ISLAND_CAPTURE_POSE") else {
        return;
    };
    // Never load a file just because the environment carries a stray pose path.
    if std::env::var_os("ALIFE_GRAPHICS_CAPTURE_DIR").is_none() {
        return;
    }
    let pose: Option<Receipt> = fs::metadata(&path)
        .ok()
        .filter(|m| m.is_file() && m.len() <= 2 * 1024 * 1024)
        .and_then(|_| fs::read(&path).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let Some(pose) = pose.filter(|p| p.camera.valid()) else {
        eprintln!("Island A/B: invalid camera receipt, leaving the player camera unchanged.");
        return;
    };
    let lock = hold
        .after(sync_fvr04_camera_follow)
        .before(constrain_camera)
        .in_set(ProductionVoxelPresentationSet::RootReaders);
    #[cfg(feature = "gpu-runtime")]
    let lock = lock.after(orchard::fx::frame_hand_focus);
    app.insert_resource(pose).add_systems(Update, lock);
}

fn hold(
    receipt: Res<Receipt>,
    active: Option<Res<creature_grounding::SelectedTerrain>>,
    mut cameras: Query<(&mut Transform, &mut Projection), With<Fvr03ProductionVoxelCamera>>,
) {
    if !active.is_some_and(|a| a.0.binding() == receipt.terrain_binding) {
        return;
    }
    let p = &receipt.camera;
    for (mut view, mut projection) in &mut cameras {
        let Projection::Orthographic(lens) = &*projection else {
            continue;
        };
        let bevy::camera::ScalingMode::FixedVertical { viewport_height } = lens.scaling_mode else {
            continue;
        };
        let scale = p.view_height_m / viewport_height;
        if lens.scale != scale {
            if let Projection::Orthographic(lens) = &mut *projection {
                lens.scale = scale;
            }
        }
        let mut locked = *view;
        locked.translation = Vec3::from_array(p.position);
        locked.rotation = Quat::from_array(p.rotation);
        view.set_if_neq(locked);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt(t: &alife_world::WorldTerrain) -> Receipt {
        Receipt {
            terrain_binding: t.binding(),
            camera: Pose {
                position: [340.0, 480.0, 380.0],
                rotation: Quat::from_rotation_x(-0.8).to_array(),
                view_height_m: 24.0,
            },
        }
    }

    #[test]
    fn only_a_matching_map_replays_the_real_orthographic_pose() {
        let t = alife_world::island_terrain();
        let mut r = receipt(&t);
        assert!(r.camera.valid());
        let good = r.terrain_binding;
        r.terrain_binding.digest ^= 1;
        let mut app = App::new();
        app.insert_resource(creature_grounding::SelectedTerrain(t))
            .insert_resource(r)
            .add_systems(Update, hold);
        let camera = app
            .world_mut()
            .spawn((
                Fvr03ProductionVoxelCamera {
                    mode: Fvr03ProductionVoxelCameraMode::OrthographicIsometric,
                },
                Transform::default(),
                Projection::Orthographic(OrthographicProjection {
                    scaling_mode: bevy::camera::ScalingMode::FixedVertical {
                        viewport_height: 12.0,
                    },
                    ..OrthographicProjection::default_3d()
                }),
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<Transform>(camera).unwrap().translation,
            Vec3::ZERO
        );
        app.world_mut().resource_mut::<Receipt>().terrain_binding = good;
        app.update();
        assert_eq!(
            app.world().get::<Transform>(camera).unwrap().translation,
            Vec3::new(340.0, 480.0, 380.0)
        );
        assert_eq!(
            view_height(app.world().get::<Projection>(camera).unwrap()),
            24.0
        );
        let mut p = app.world_mut().resource_mut::<Receipt>();
        p.camera.rotation = [0.0; 4];
        assert!(!p.camera.valid());
    }
}
