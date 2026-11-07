//! Camera input in game-window coordinates, independent of monitor layout.

use super::*;
use crate::terrain_lighting::PRODUCTION_CAMERA_MAX_ZOOM;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};

fn scroll_lines(event: &MouseWheel, scale_factor: f32) -> Option<f32> {
    if !event.y.is_finite() {
        return None;
    }
    let lines = match event.unit {
        MouseScrollUnit::Line => event.y,
        MouseScrollUnit::Pixel => {
            if !scale_factor.is_finite() || scale_factor <= 0.0 {
                return None;
            }
            // Winit supplies physical pixels. Use logical pixels and Bevy's
            // approximate 100-pixel line equivalent; leave wheel lines unchanged.
            event.y / scale_factor / MouseScrollUnit::SCROLL_UNIT_CONVERSION_FACTOR
        }
    };
    lines.is_finite().then_some(lines)
}

fn zoom_scale(scale: f32, wheel: f32, terrain_viewport_height: Option<f32>) -> f32 {
    let maximum = terrain_viewport_height.map_or(PRODUCTION_CAMERA_MAX_ZOOM, |h| 1600.0 / h);
    // Profiles use different base extents. Clamp the actual terrain view in metres
    // so a low-end profile cannot zoom through the character-sized close view.
    let minimum = terrain_viewport_height.map_or(0.35, |h| 9.8 / h);
    (scale * (-wheel * 0.10).exp()).clamp(minimum, maximum)
}

pub(super) fn zoom_camera(
    mut wheel: bevy::prelude::MessageReader<MouseWheel>,
    windows: bevy::prelude::Query<(Entity, &Window), With<PrimaryWindow>>,
    terrain: Option<Res<creature_grounding::SelectedTerrain>>,
    mut cameras: bevy::prelude::Query<&mut Projection, With<Fvr03ProductionVoxelCamera>>,
) {
    let Ok((window_entity, window)) = windows.single() else {
        wheel.clear();
        return;
    };
    // Physical scroll deltas use the display scale, not an application UI override.
    let scale_factor = window.resolution.base_scale_factor();
    let delta: f32 = wheel
        .read()
        .filter(|event| event.window == window_entity && window.focused)
        .filter_map(|event| scroll_lines(event, scale_factor))
        .sum();
    if delta == 0.0 || !delta.is_finite() {
        return;
    }
    for mut projection in &mut cameras {
        if let Projection::Orthographic(camera) = &mut *projection {
            let height = if terrain.is_some() {
                match camera.scaling_mode {
                    bevy::camera::ScalingMode::FixedVertical { viewport_height } => {
                        Some(viewport_height)
                    }
                    _ => Some(32.0),
                }
            } else {
                None
            };
            camera.scale = zoom_scale(camera.scale, delta, height);
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
    mut cameras: bevy::prelude::Query<
        (&mut Transform, &Projection),
        With<Fvr03ProductionVoxelCamera>,
    >,
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
    follow.enabled = false;
    for (mut camera, projection) in &mut cameras {
        let scale = match projection {
            Projection::Orthographic(p) => p.scale,
            _ => 1.0,
        };
        let distance =
            production_camera_extent(scene.profile_id) * scale * 0.55 * time.delta_secs().min(0.05);
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
    use bevy::{camera::OrthographicProjection, window::WindowResolution};

    fn zoom_app(scale_factor: f32) -> (App, Entity, Entity) {
        let mut app = App::new();
        app.add_message::<MouseWheel>()
            .add_systems(Update, zoom_camera);
        let mut resolution = WindowResolution::new(1280, 720);
        resolution.set_scale_factor(scale_factor);
        let window = app
            .world_mut()
            .spawn((
                Window {
                    resolution,
                    focused: true,
                    ..default()
                },
                PrimaryWindow,
            ))
            .id();
        let camera = app
            .world_mut()
            .spawn((
                Projection::Orthographic(OrthographicProjection {
                    scaling_mode: ScalingMode::FixedVertical {
                        viewport_height: 30.0,
                    },
                    ..OrthographicProjection::default_3d()
                }),
                Fvr03ProductionVoxelCamera {
                    mode: Fvr03ProductionVoxelCameraMode::OrthographicIsometric,
                },
            ))
            .id();
        (app, window, camera)
    }

    fn scroll(app: &mut App, window: Entity, events: &[(MouseScrollUnit, f32)]) {
        for &(unit, y) in events {
            app.world_mut().write_message(MouseWheel {
                unit,
                x: 0.0,
                y,
                window,
            });
        }
        app.update();
    }

    fn camera_scale(app: &App, camera: Entity) -> f32 {
        match app.world().get::<Projection>(camera).unwrap() {
            Projection::Orthographic(projection) => projection.scale,
            _ => panic!("expected orthographic camera"),
        }
    }

    #[test]
    fn wheel_lines_keep_existing_zoom_at_every_display_scale() {
        for scale_factor in [1.0, 1.25, 2.0, 3.0] {
            for y in [-1.0, -0.25, 0.0, 0.25, 1.0] {
                let (mut app, window, camera) = zoom_app(scale_factor);
                scroll(&mut app, window, &[(MouseScrollUnit::Line, y)]);
                assert_eq!(camera_scale(&app, camera), zoom_scale(1.0, y, None));
            }
        }
    }

    #[test]
    fn pixel_scroll_uses_logical_line_equivalents_across_display_scales() {
        for scale_factor in [1.0, 1.25, 2.0, 3.0] {
            for logical_pixels in [-100.0, -5.0, 5.0, 100.0] {
                let (mut app, window, camera) = zoom_app(scale_factor);
                scroll(
                    &mut app,
                    window,
                    &[(MouseScrollUnit::Pixel, logical_pixels * scale_factor)],
                );
                assert_eq!(
                    camera_scale(&app, camera),
                    zoom_scale(1.0, logical_pixels / 100.0, None)
                );
            }
        }
    }

    #[test]
    fn mixed_scroll_units_are_normalized_before_summing() {
        let (mut app, window, camera) = zoom_app(2.0);
        scroll(
            &mut app,
            window,
            &[
                (MouseScrollUnit::Line, 1.0),
                (MouseScrollUnit::Pixel, -200.0),
            ],
        );
        assert_eq!(camera_scale(&app, camera), 1.0);
        scroll(
            &mut app,
            window,
            &[
                (MouseScrollUnit::Line, 0.25),
                (MouseScrollUnit::Pixel, 100.0),
            ],
        );
        assert_eq!(camera_scale(&app, camera), zoom_scale(1.0, 0.75, None));
    }

    #[test]
    fn ui_scale_override_does_not_change_physical_scroll_conversion() {
        let (mut app, window, camera) = zoom_app(2.0);
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .resolution
            .set_scale_factor_override(Some(1.0));
        scroll(&mut app, window, &[(MouseScrollUnit::Pixel, 200.0)]);
        assert_eq!(camera_scale(&app, camera), zoom_scale(1.0, 1.0, None));
    }

    #[test]
    fn other_windows_cannot_zoom_the_primary_camera() {
        let (mut app, window, camera) = zoom_app(1.0);
        let other_window = app.world_mut().spawn(Window::default()).id();
        app.world_mut().write_message(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: 100.0,
            window: other_window,
        });
        scroll(&mut app, window, &[(MouseScrollUnit::Line, -1.0)]);
        assert_eq!(camera_scale(&app, camera), zoom_scale(1.0, -1.0, None));
    }

    #[test]
    fn invalid_scroll_samples_do_not_poison_valid_input() {
        let (mut app, window, camera) = zoom_app(2.0);
        scroll(
            &mut app,
            window,
            &[
                (MouseScrollUnit::Line, f32::NAN),
                (MouseScrollUnit::Pixel, f32::INFINITY),
                (MouseScrollUnit::Line, f32::NEG_INFINITY),
                (MouseScrollUnit::Pixel, 100.0),
            ],
        );
        assert_eq!(camera_scale(&app, camera), zoom_scale(1.0, 0.5, None));
        for scale_factor in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::MIN_POSITIVE] {
            let (mut app, window, camera) = zoom_app(scale_factor);
            scroll(
                &mut app,
                window,
                &[
                    (MouseScrollUnit::Pixel, f32::MAX),
                    (MouseScrollUnit::Line, 1.0),
                ],
            );
            assert_eq!(camera_scale(&app, camera), zoom_scale(1.0, 1.0, None));
        }
    }

    #[test]
    fn overflowing_scroll_batch_leaves_the_projection_valid() {
        let (mut app, window, camera) = zoom_app(1.0);
        scroll(
            &mut app,
            window,
            &[
                (MouseScrollUnit::Line, f32::MAX),
                (MouseScrollUnit::Line, f32::MAX),
            ],
        );
        assert_eq!(camera_scale(&app, camera), 1.0);
    }

    #[test]
    fn unfocused_and_missing_windows_discard_scroll_without_replay() {
        let (mut app, window, camera) = zoom_app(1.0);
        app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
        scroll(&mut app, window, &[(MouseScrollUnit::Pixel, 100.0)]);
        app.world_mut().get_mut::<Window>(window).unwrap().focused = true;
        app.update();
        assert_eq!(camera_scale(&app, camera), 1.0);

        app.world_mut().entity_mut(window).remove::<PrimaryWindow>();
        scroll(&mut app, window, &[(MouseScrollUnit::Line, 1.0)]);
        app.world_mut().entity_mut(window).insert(PrimaryWindow);
        app.update();
        assert_eq!(camera_scale(&app, camera), 1.0);
    }

    #[test]
    fn zoom_roundtrip_and_extremes_keep_a_valid_projection() {
        let close = 1.0;
        let wide = zoom_scale(close, -30.0, Some(30.0));
        assert!(wide * 30.0 > 600.0);
        assert!((zoom_scale(wide, 30.0, Some(30.0)) - close).abs() < 0.0001);
        assert_eq!(zoom_scale(close, -10000.0, Some(30.0)) * 30.0, 1600.0);
        for height in [9.8, 17.2, 30.0, 34.0, 40.0] {
            assert!((zoom_scale(close, 10000.0, Some(height)) * height - 9.8).abs() < 1e-5);
            assert!((zoom_scale(close, -10000.0, Some(height)) * height - 1600.0).abs() < 1e-3);
        }
        assert_eq!(
            zoom_scale(close, -10000.0, None),
            PRODUCTION_CAMERA_MAX_ZOOM
        );
    }

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
