//! Camera input in game-window coordinates, independent of monitor layout.

use super::*;

pub(super) fn zoom_camera(
    mut wheel: bevy::prelude::MessageReader<bevy::input::mouse::MouseWheel>,
    windows: bevy::prelude::Query<&bevy::window::Window, With<bevy::window::PrimaryWindow>>,
    mut cameras: bevy::prelude::Query<&mut Projection, With<Fvr03ProductionVoxelCamera>>,
) {
    let delta: f32 = wheel.read().map(|event| event.y).sum();
    if delta == 0.0 || !windows.single().is_ok_and(|window| window.focused) {
        return;
    }
    for mut projection in &mut cameras {
        if let Projection::Orthographic(camera) = &mut *projection {
            camera.scale = (camera.scale * (-delta * 0.10).exp()).clamp(0.35, 3.5);
        }
    }
}

use bevy::math::Vec2;

fn camera_pan_axis(keys: Vec2, cursor: Option<Vec2>, window_size: Vec2, focused: bool) -> Vec2 {
    if !focused {
        return Vec2::ZERO;
    }
    let mut axis = keys;
    if let Some(cursor) =
        cursor.filter(|p| p.x >= 0.0 && p.y >= 0.0 && p.x < window_size.x && p.y < window_size.y)
    {
        const EDGE: f32 = 16.0;
        axis.x += f32::from(cursor.x >= window_size.x - EDGE) - f32::from(cursor.x < EDGE);
        axis.y += f32::from(cursor.y < EDGE) - f32::from(cursor.y >= window_size.y - EDGE);
    }
    axis.clamp_length_max(1.0)
}

pub(super) fn pan_camera(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    windows: bevy::prelude::Query<&bevy::window::Window, With<bevy::window::PrimaryWindow>>,
    scene: Res<Fvr03ProductionVoxelSceneResource>,
    mut follow: ResMut<Fvr04ProductionCreatureFollowResource>,
    mut cameras: bevy::prelude::Query<&mut Transform, With<Fvr03ProductionVoxelCamera>>,
    #[cfg(feature = "gpu-runtime")] conversation: Option<
        Res<crate::ProductionConversationLineageUiState>,
    >,
) {
    #[cfg(feature = "gpu-runtime")]
    if conversation
        .as_ref()
        .is_some_and(|state| state.blocks_world_shortcuts())
    {
        return;
    }
    // A snap wins over a cursor left resting at an edge for this frame.
    if [
        KeyCode::Home,
        KeyCode::KeyF,
        KeyCode::PageUp,
        KeyCode::PageDown,
        KeyCode::KeyR,
    ]
    .into_iter()
    .any(|key| keyboard.just_pressed(key))
    {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let keys = Vec2::new(
        f32::from(keyboard.pressed(KeyCode::ArrowRight))
            - f32::from(keyboard.pressed(KeyCode::ArrowLeft)),
        f32::from(keyboard.pressed(KeyCode::ArrowUp))
            - f32::from(keyboard.pressed(KeyCode::ArrowDown)),
    );
    let axis = camera_pan_axis(
        keys,
        window.cursor_position(),
        Vec2::new(window.width(), window.height()),
        window.focused,
    );
    if axis == Vec2::ZERO {
        return;
    }
    let distance = production_camera_extent(scene.profile_id) * 0.55 * time.delta_secs().min(0.05);
    if distance <= 0.0 {
        return;
    }
    follow.enabled = false;
    for mut camera in &mut cameras {
        let right = camera.rotation * Vec3::X;
        let forward = camera.rotation * Vec3::NEG_Z;
        let right = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();
        let forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
        camera.translation += (right * axis.x + forward * axis.y) * distance;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_edges_and_arrows_pan_without_diagonal_acceleration() {
        let size = Vec2::new(800.0, 600.0);
        for (cursor, expected) in [
            (Vec2::new(1.0, 300.0), Vec2::NEG_X),
            (Vec2::new(799.0, 300.0), Vec2::X),
            (Vec2::new(400.0, 1.0), Vec2::Y),
            (Vec2::new(400.0, 599.0), Vec2::NEG_Y),
            (Vec2::new(400.0, 300.0), Vec2::ZERO),
        ] {
            assert_eq!(
                camera_pan_axis(Vec2::ZERO, Some(cursor), size, true),
                expected
            );
        }
        assert_eq!(camera_pan_axis(Vec2::X, None, size, true), Vec2::X);
        assert_eq!(camera_pan_axis(Vec2::Y, None, size, true), Vec2::Y);
        assert!((camera_pan_axis(Vec2::ONE, None, size, true).length() - 1.0).abs() < 1e-6);
        assert_eq!(
            camera_pan_axis(Vec2::X, Some(Vec2::ZERO), size, false),
            Vec2::ZERO
        );
        assert_eq!(
            camera_pan_axis(Vec2::ZERO, Some(Vec2::new(-100.0, 200.0)), size, true),
            Vec2::ZERO
        );
    }
}
