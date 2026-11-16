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

/// Invert the orthographic screen projection restricted to the horizontal ground plane.
/// Camera translation is opposite screen-right drag and along screen-up for downward drag.
fn drag_pan_world_delta(
    drag_pixels: Vec2,
    window_height: f32,
    extent: f32,
    scale: f32,
    right: Vec3,
    up: Vec3,
) -> Vec3 {
    if !drag_pixels.is_finite()
        || !window_height.is_finite()
        || window_height <= 1.0
        || !extent.is_finite()
        || extent <= 0.0
        || !scale.is_finite()
        || scale <= 0.0
        || !right.is_finite()
        || !up.is_finite()
    {
        return Vec3::ZERO;
    }
    // A level camera has no finite vertical-drag solution on a horizontal plane.
    // Reject near-singular views rather than amplifying cursor noise without bound.
    let determinant = right.x * up.z - right.z * up.x;
    if !determinant.is_finite() || determinant.abs() <= 1.0e-3 {
        return Vec3::ZERO;
    }
    let world_per_pixel = (extent * scale) / window_height;
    let screen = Vec2::new(-drag_pixels.x, drag_pixels.y) * world_per_pixel;
    let delta = Vec3::new(
        (screen.x * up.z - right.z * screen.y) / determinant,
        0.0,
        (right.x * screen.y - screen.x * up.x) / determinant,
    );
    if delta.is_finite() {
        delta
    } else {
        Vec3::ZERO
    }
}

fn sample_drag_delta(
    anchor: &mut Option<Vec2>,
    cursor: Option<Vec2>,
    active: bool,
    fresh_press: bool,
) -> Vec2 {
    if !active || fresh_press {
        *anchor = None;
    }
    let Some(cursor) = cursor.filter(|p| active && p.is_finite()) else {
        *anchor = None;
        return Vec2::ZERO;
    };
    let delta = anchor.map_or(Vec2::ZERO, |previous| cursor - previous);
    *anchor = Some(cursor);
    if delta.is_finite() {
        delta
    } else {
        Vec2::ZERO
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "Bevy injects independent ECS system parameters."
)]
pub(super) fn pan_camera(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    mut drag_anchor: Local<Option<Vec2>>,
    mut drag_frame: Local<Option<u32>>,
    frame: Option<Res<bevy::diagnostic::FrameCount>>,
    time: Res<Time>,
    windows: bevy::prelude::Query<&bevy::window::Window, With<bevy::window::PrimaryWindow>>,
    scene: Res<Fvr03ProductionVoxelSceneResource>,
    mut follow: ResMut<Fvr04ProductionCreatureFollowResource>,
    mut cameras: bevy::prelude::Query<
        (&mut Transform, &Projection),
        With<Fvr03ProductionVoxelCamera>,
    >,
    ux: Option<Res<Fvr05ProductionUxStateResource>>,
    #[cfg(feature = "gpu-runtime")] conversation: Option<
        Res<crate::ProductionConversationLineageUiState>,
    >,
) {
    // Input run conditions can skip this system while a menu owns the pointer.
    // Never replay the cursor travel accumulated during that interruption.
    if let Some(frame) = frame {
        if drag_frame.is_none_or(|previous| previous.wrapping_add(1) != frame.0) {
            *drag_anchor = None;
        }
        *drag_frame = Some(frame.0);
    }
    #[cfg(feature = "gpu-runtime")]
    if conversation
        .as_ref()
        .is_some_and(|state| state.blocks_world_shortcuts())
    {
        *drag_anchor = None;
        return;
    }
    let player_mode = ux.as_ref().is_none_or(|state| !state.debug_mode);
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
        || (player_mode
            && (keyboard.just_pressed(KeyCode::Tab) || keyboard.just_pressed(KeyCode::KeyN)))
    {
        *drag_anchor = None;
        return;
    }
    let Ok(window) = windows.single() else {
        *drag_anchor = None;
        return;
    };
    let middle_dragging = window.focused
        && mouse
            .as_ref()
            .is_some_and(|m| m.pressed(MouseButton::Middle));
    let drag_delta = sample_drag_delta(
        &mut drag_anchor,
        window.cursor_position(),
        middle_dragging,
        mouse
            .as_ref()
            .is_some_and(|m| m.just_pressed(MouseButton::Middle)),
    );
    if middle_dragging {
        if drag_delta == Vec2::ZERO {
            return;
        }
        follow.enabled = false;
        let extent = production_camera_extent(scene.profile_id);
        let window_height = window.height();
        for (mut camera, projection) in &mut cameras {
            let Projection::Orthographic(p) = projection else {
                continue;
            };
            let extent = match p.scaling_mode {
                ScalingMode::FixedVertical { viewport_height } => viewport_height,
                _ => extent,
            };
            let right = camera.rotation * Vec3::X;
            let up = camera.rotation * Vec3::Y;
            camera.translation +=
                drag_pan_world_delta(drag_delta, window_height, extent, p.scale, right, up);
        }
        return;
    }
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

    #[test]
    fn middle_mouse_drag_pan_scales_world_motion_to_viewport_and_ignores_non_finite_samples() {
        let delta = drag_pan_world_delta(
            Vec2::new(100.0, -50.0),
            600.0,
            30.0,
            2.0,
            Vec3::X,
            Vec3::NEG_Z,
        );
        // 60 world units across 600px => 0.1 world units per pixel.
        // Dragging right (+100x) pulls the world right by moving the camera left (-10x);
        // dragging up (-50y) moves the camera backward along -forward (+5z).
        assert!((delta - Vec3::new(-10.0, 0.0, 5.0)).length() < 1e-5);
        assert_eq!(
            drag_pan_world_delta(Vec2::NAN, 600.0, 30.0, 1.0, Vec3::X, Vec3::NEG_Z),
            Vec3::ZERO
        );
        assert_eq!(
            drag_pan_world_delta(Vec2::ONE, 0.0, 30.0, 1.0, Vec3::X, Vec3::NEG_Z),
            Vec3::ZERO
        );
    }
    #[test]
    fn drag_samples_reset_after_release_focus_loss_missing_cursor_and_new_press() {
        let mut anchor = None;
        assert_eq!(
            sample_drag_delta(&mut anchor, Some(Vec2::ONE), true, true),
            Vec2::ZERO
        );
        assert_eq!(
            sample_drag_delta(&mut anchor, Some(Vec2::splat(3.0)), true, false),
            Vec2::splat(2.0)
        );
        for (cursor, active, fresh) in [
            (Some(Vec2::splat(100.0)), false, false),
            (None, true, false),
            (Some(Vec2::NAN), true, false),
            (Some(Vec2::splat(200.0)), true, true),
        ] {
            assert_eq!(
                sample_drag_delta(&mut anchor, cursor, active, fresh),
                Vec2::ZERO
            );
            if !fresh {
                assert_eq!(anchor, None);
            }
            assert_eq!(
                sample_drag_delta(&mut anchor, Some(Vec2::splat(200.0)), true, true),
                Vec2::ZERO
            );
        }
    }

    #[test]
    fn invalid_and_overflowing_drag_projection_is_ignored() {
        for (extent, scale, right) in [
            (0.0, 1.0, Vec3::X),
            (30.0, -1.0, Vec3::X),
            (30.0, f32::INFINITY, Vec3::X),
            (30.0, 1.0, Vec3::NAN),
            (f32::MAX, f32::MAX, Vec3::X),
        ] {
            assert_eq!(
                drag_pan_world_delta(Vec2::ONE, 600.0, extent, scale, right, Vec3::NEG_Z),
                Vec3::ZERO
            );
        }
    }
    fn assert_drag_roundtrip(rotation: Quat, drag: Vec2, extent: f32, scale: f32, height: f32) {
        let right = rotation * Vec3::X;
        let up = rotation * Vec3::Y;
        let delta = drag_pan_world_delta(drag, height, extent, scale, right, up);
        assert!(delta.is_finite());
        assert_eq!(delta.y, 0.0);
        let pixels_per_world = height / (extent * scale);
        // Moving the camera shifts a fixed world point in the opposite direction.
        let observed_pixels = Vec2::new(-right.dot(delta), up.dot(delta)) * pixels_per_world;
        assert!(
            (observed_pixels - drag).length() < 0.002,
            "{observed_pixels:?} != {drag:?}"
        );
    }

    #[test]
    fn ground_drag_tracks_cursor_at_every_production_initial_and_follow_pitch() {
        for &profile in ProductionFrontendProfileId::all() {
            let extent = production_camera_extent(profile);
            for mode in [
                Fvr03ProductionVoxelCameraMode::OrthographicIsometric,
                Fvr03ProductionVoxelCameraMode::Orbit,
            ] {
                for transform in [
                    crate::terrain_lighting::production_camera_transform(mode, extent),
                    crate::terrain_lighting::production_follow_camera_transform(
                        mode,
                        extent,
                        Vec3::new(83.0, 7.0, -46.0),
                    ),
                ] {
                    for scale in [9.8 / extent, 1.0, 1600.0 / extent] {
                        for height in [480.0, 720.0, 2160.0] {
                            for drag in [
                                Vec2::new(100.0, 0.0),
                                Vec2::new(0.0, 100.0),
                                Vec2::new(-123.0, 71.0),
                            ] {
                                assert_drag_roundtrip(
                                    transform.rotation,
                                    drag,
                                    extent,
                                    scale,
                                    height,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn ground_drag_handles_top_down_roll_and_pitch_limits_without_poisoning_camera() {
        for pitch in [0.002_f32, 0.25, 0.75, std::f32::consts::FRAC_PI_2] {
            for yaw in [0.0_f32, 1.0, -2.0] {
                for roll in [0.0_f32, 0.6] {
                    let rotation = Quat::from_rotation_y(yaw)
                        * Quat::from_rotation_x(-pitch)
                        * Quat::from_rotation_z(roll);
                    assert_drag_roundtrip(rotation, Vec2::new(71.0, -93.0), 30.0, 2.0, 600.0);
                }
            }
        }
        for pitch in [0.0_f32, 1.0e-5, -1.0e-5] {
            let rotation = Quat::from_rotation_x(-pitch);
            assert_eq!(
                drag_pan_world_delta(
                    Vec2::ONE,
                    600.0,
                    30.0,
                    1.0,
                    rotation * Vec3::X,
                    rotation * Vec3::Y
                ),
                Vec3::ZERO
            );
        }
        assert_eq!(
            drag_pan_world_delta(Vec2::ONE, 600.0, 30.0, 1.0, Vec3::X, Vec3::X),
            Vec3::ZERO
        );
    }

    fn camera_scene(profile_id: ProductionFrontendProfileId) -> Fvr03ProductionVoxelSceneResource {
        Fvr03ProductionVoxelSceneResource {
            schema: FVR03_PRODUCTION_VOXEL_RENDERER_SCHEMA,
            schema_version: FVR03_PRODUCTION_VOXEL_RENDERER_SCHEMA_VERSION,
            snapshot_schema: "CPU camera fixture".into(),
            profile_id,
            population: 1,
            renderer_profile: String::new(),
            backend_id: FVR10_RENDERER_BACKEND_ID,
            uses_internal_voxel_terrain_mesh: false,
            visible_chunk_count: 0,
            resident_chunk_count: 0,
            tile_mesh_count: 0,
            creature_render_count: 0,
            creature_material_bucket_count: 0,
            creature_lod: Fvr04CreatureLod::CompactVoxel,
            creature_root_count: 0,
            creature_part_entity_count: 0,
            creature_join_cover_count: 0,
            creature_part_family_count: 0,
            creature_mixed_assembly_count: 0,
            creature_shared_mesh_handle_count: 0,
            selection_ref_count: 0,
            dirty_chunk_count: 0,
            estimated_resident_bytes: 0,
            draw_radius_chunks: 0,
            target_fps: 30,
            performance_artifact_path: None,
            material_counts: BTreeMap::new(),
            average_resource_bias: 0.,
            average_hazard_pressure: 0.,
            production_dressing_count: 0,
            production_vfx_marker_count: 0,
            production_gpu_vfx_emitter_count: 0,
            production_vfx_budget_state: "CPU fixture",
            production_visuals_display_only: true,
            production_vfx_uses_hanabi_gpu_particles: false,
            mesh_stats: Fvr09TerrainMeshStats {
                mode: Fvr09MesherMode::LayeredGridQuads,
                visible_voxels: 0,
                naive_visible_faces: 0,
                emitted_quads: 0,
                face_reduction_ratio: 0.,
                remesh_time_micros: 0,
                dirty_chunks: 0,
                cached_chunks: 0,
                skipped_chunks: 0,
                remesh_budget_chunks_per_frame: 0,
                material_palette_version: "fixture",
                vertex_color_face_variation: false,
                top_side_color_separation: false,
                variation_bucket_count: 0,
                cache_key: String::new(),
            },
            visible_tiles: BTreeSet::new(),
            visible_chunks: BTreeSet::new(),
            tile_summaries_by_tile: BTreeMap::new(),
            creature_refs_by_tile: BTreeMap::new(),
            selection_positions_by_raw_id: BTreeMap::new(),
        }
    }
    #[derive(Resource)]
    struct PanEnabled(bool);

    #[test]
    fn middle_drag_owns_edges_and_does_not_replay_skipped_modal_frames() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<Time>()
            .insert_resource(bevy::diagnostic::FrameCount(0))
            .insert_resource(PanEnabled(true))
            .insert_resource(camera_scene(ProductionFrontendProfileId::Balanced1080p))
            .insert_resource(Fvr04ProductionCreatureFollowResource {
                enabled: true,
                target_stable_id: None,
            })
            .add_systems(
                Update,
                pan_camera.run_if(|enabled: Res<PanEnabled>| enabled.0),
            );
        let window = app
            .world_mut()
            .spawn((
                Window {
                    resolution: WindowResolution::new(800, 600),
                    focused: true,
                    ..default()
                },
                PrimaryWindow,
            ))
            .id();
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .set_cursor_position(Some(Vec2::new(799.0, 300.0)));
        let production_transform = crate::terrain_lighting::production_follow_camera_transform(
            Fvr03ProductionVoxelCameraMode::OrthographicIsometric,
            30.0,
            Vec3::ZERO,
        );
        let initial_position = production_transform.translation;
        let camera = app
            .world_mut()
            .spawn((
                Fvr03ProductionVoxelCamera {
                    mode: Fvr03ProductionVoxelCameraMode::OrthographicIsometric,
                },
                production_transform,
                Projection::Orthographic(OrthographicProjection {
                    scaling_mode: ScalingMode::FixedVertical {
                        viewport_height: 60.0,
                    },
                    ..OrthographicProjection::default_3d()
                }),
            ))
            .id();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.016));
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Middle);
        app.update();
        assert_eq!(
            app.world().get::<Transform>(camera).unwrap().translation,
            initial_position
        );
        assert!(
            app.world()
                .resource::<Fvr04ProductionCreatureFollowResource>()
                .enabled
        );
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
        app.world_mut()
            .resource_mut::<bevy::diagnostic::FrameCount>()
            .0 += 1;
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .set_cursor_position(Some(Vec2::new(699.0, 350.0)));
        app.update();
        let delta = app.world().get::<Transform>(camera).unwrap().translation - initial_position;
        let right = production_transform.rotation * Vec3::X;
        let up = production_transform.rotation * Vec3::Y;
        let projected_motion = Vec2::new(-right.dot(delta), up.dot(delta)) * 10.0;
        assert!((projected_motion - Vec2::new(-100.0, 50.0)).length() < 0.002);
        assert!(
            !app.world()
                .resource::<Fvr04ProductionCreatureFollowResource>()
                .enabled
        );
        let before_modal = app.world().get::<Transform>(camera).unwrap().translation;
        app.world_mut().resource_mut::<PanEnabled>().0 = false;
        app.world_mut()
            .resource_mut::<bevy::diagnostic::FrameCount>()
            .0 += 1;
        app.update();
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .set_cursor_position(Some(Vec2::new(100.0, 300.0)));
        app.world_mut().resource_mut::<PanEnabled>().0 = true;
        app.world_mut()
            .resource_mut::<bevy::diagnostic::FrameCount>()
            .0 += 1;
        app.update();
        assert_eq!(
            app.world().get::<Transform>(camera).unwrap().translation,
            before_modal
        );
    }
}
