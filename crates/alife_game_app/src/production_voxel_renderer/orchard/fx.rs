//! The targeting light is a preview. Brief particles are emitted only from a real tool receipt.
use super::*;
use alife_world::orchard::OrchardReceipt;

#[derive(Component)]
pub(in crate::production_voxel_renderer) struct SoilPreview;
#[derive(Component)]
pub(in crate::production_voxel_renderer) struct Mote {
    base: Vec3,
    index: u8,
    kind: Kind,
    age: f32,
}
#[derive(Clone, Copy)]
enum Kind {
    Soil,
    Rain,
    Observe,
    Pollen,
}
#[derive(Default)]
pub(in crate::production_voxel_renderer) struct Art(Option<EffectArt>);
struct EffectArt {
    ring: Handle<Mesh>,
    bead: Handle<Mesh>,
    pigment: [Handle<StandardMaterial>; 4],
}

fn ground(tile: VoxelTileCoord, surface: &creature_grounding::RenderedTerrainSurface) -> Vec3 {
    let p = Vec3::new(tile.x as f32 + 0.5, 0., tile.z as f32 + 0.5);
    Vec3::new(p.x, surface.height(p).unwrap_or(0.) + 0.035, p.z)
}
#[allow(
    clippy::too_many_arguments,
    reason = "Independent scene, Hand, input and mesh resources."
)]
pub(in crate::production_voxel_renderer) fn project(
    mut commands: Commands,
    mut cache: Local<Art>,
    mut dock: ResMut<care_dock::State>,
    selection: Res<Fvr03ProductionVoxelSelectionResource>,
    runtime: Option<NonSend<ProductionGpuBrainRuntimeResource>>,
    surface: Res<creature_grounding::RenderedTerrainSurface>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut preview: Query<(Entity, &mut Transform), With<SoilPreview>>,
) {
    let target = if dock.garden_armed {
        (if dock.garden_pointer {
            soil_tile(selection.hovered)
        } else {
            dock.garden_cursor
        })
        .filter(|tile| {
            runtime
                .as_ref()
                .is_some_and(|r| r.runtime.world().orchard_gesture(*tile).is_some())
        })
    } else {
        None
    };
    let effect = dock.garden_effect.take();
    if target.is_none() && effect.is_none() {
        for (entity, _) in &mut preview {
            commands.entity(entity).despawn();
        }
        return;
    }
    let EffectArt {
        ring,
        bead,
        pigment,
    } = cache.0.get_or_insert_with(|| {
        let colors = [
            Color::srgb(0.93, 0.69, 0.34),
            Color::srgb(0.44, 0.84, 0.90),
            Color::srgb(0.54, 0.94, 0.72),
            Color::srgb(1., 0.65, 0.29),
        ];
        EffectArt {
            ring: meshes.add(Annulus::new(0.30, 0.33)),
            bead: meshes.add(Sphere::new(1.).mesh().ico(1).unwrap()),
            pigment: colors.map(|color| {
                materials.add(StandardMaterial {
                    base_color: color,
                    unlit: true,
                    ..default()
                })
            }),
        }
    });
    if let Some(tile) = target {
        let transform = Transform::from_translation(ground(tile, &surface))
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2));
        if let Ok((_, mut t)) = preview.single_mut() {
            t.set_if_neq(transform);
        } else {
            commands.spawn((
                Name::new("Soil under the Hand"),
                SoilPreview,
                Mesh3d(ring.clone()),
                MeshMaterial3d(pigment[2].clone()),
                transform,
                bevy::picking::Pickable::IGNORE,
            ));
        }
    } else {
        for (entity, _) in &mut preview {
            commands.entity(entity).despawn();
        }
    }
    if let Some((tile, receipt)) = effect {
        let (kind, index) = match receipt {
            OrchardReceipt::Sown => (Kind::Soil, 0),
            OrchardReceipt::Watered => (Kind::Rain, 1),
            OrchardReceipt::Watching => (Kind::Observe, 2),
            OrchardReceipt::Picked(_) => (Kind::Pollen, 3),
        };
        let base = ground(tile, &surface);
        for i in 0..7 {
            commands.spawn((
                Mote {
                    base,
                    index: i,
                    kind,
                    age: 0.,
                },
                Mesh3d(bead.clone()),
                MeshMaterial3d(pigment[index].clone()),
                Transform::from_translation(base).with_scale(Vec3::ZERO),
                bevy::picking::Pickable::IGNORE,
            ));
        }
    }
}
pub(in crate::production_voxel_renderer) fn drift(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Mote, &mut Transform)>,
) {
    for (entity, mut mote, mut pose) in &mut query {
        mote.age += time.delta_secs();
        let t = (mote.age - mote.index as f32 * 0.045) / 0.85;
        if t >= 1. {
            commands.entity(entity).despawn();
            continue;
        }
        if t < 0. {
            continue;
        }
        let angle = mote.index as f32 * 2.39996;
        let (rad, height, scale) = match mote.kind {
            Kind::Rain => (
                0.22 + 0.04 * angle.cos(),
                0.09 + (1. - t * t) * 0.81,
                Vec3::new(0.017, 0.053, 0.017),
            ),
            Kind::Soil => (
                0.12 + 0.23 * t,
                0.07 + 0.25 * (t * std::f32::consts::PI).sin(),
                Vec3::splat(0.025),
            ),
            Kind::Observe => (0.38 - 0.1 * t, 0.08 + 0.22 * t, Vec3::splat(0.019)),
            Kind::Pollen => (
                0.1 + 0.41 * t,
                0.60 + 0.32 * (t * std::f32::consts::PI).sin() - 0.25 * t,
                Vec3::splat(0.026),
            ),
        };
        pose.translation = mote.base + Vec3::new(rad * angle.cos(), height, rad * angle.sin());
        pose.scale = scale * (1. - t * t);
    }
}

/// A deliberate Hand inspection frames a real bed or keepsake, then relinquishes the camera.
/// Ordinary pan, scroll, follow, a new selection, or a modal cancels the move.
#[allow(
    clippy::too_many_arguments,
    reason = "Camera, modal and player input are separate scene resources."
)]
pub(in crate::production_voxel_renderer) fn frame_hand_focus(
    mut dock: ResMut<care_dock::State>,
    selection: Res<Fvr03ProductionVoxelSelectionResource>,
    follow: Res<Fvr04ProductionCreatureFollowResource>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut scroll: MessageReader<bevy::input::mouse::MouseWheel>,
    time: Res<Time>,
    menu: Option<Res<menu::MenuState>>,
    conversation: Option<Res<ProductionConversationLineageUiState>>,
    surface: Res<creature_grounding::RenderedTerrainSurface>,
    scene: Res<Fvr03ProductionVoxelSceneResource>,
    runtime: Option<NonSend<ProductionGpuBrainRuntimeResource>>,
    mut camera: Query<(&Fvr03ProductionVoxelCamera, &mut Transform, &mut Projection)>,
) {
    let new_target = dock.hand_focus.take();
    let wheel = scroll.read().count() > 0;
    if let Some(tile) = new_target {
        dock.framing = Some((tile, 0.));
    }
    let Some((focus, age)) = dock.framing else {
        return;
    };
    if selection.selected != focus.guard
        || follow.enabled
        || wheel
        || mouse.pressed(MouseButton::Middle)
        || keys.any_pressed([
            KeyCode::ArrowLeft,
            KeyCode::ArrowRight,
            KeyCode::ArrowUp,
            KeyCode::ArrowDown,
            KeyCode::Home,
        ])
        || menu.as_deref().is_some_and(menu::captures_input_state)
        || conversation
            .as_deref()
            .is_some_and(|s| s.blocks_world_shortcuts())
    {
        dock.framing = None;
        return;
    }
    let dt = time.delta_secs().min(0.1);
    let settle = age + dt >= 0.85;
    let blend = if settle { 1. } else { 1. - (-7.5 * dt).exp() };
    let point = if let Some(id) = focus.gift {
        let Some(point) = runtime
            .as_ref()
            .and_then(|r| field_signs::glass::current_point(r.runtime.world(), id))
        else {
            dock.framing = None;
            return;
        };
        point
    } else {
        focus.point
    };
    let mut position = Vec3::new(point.x, 0., point.y);
    position.y = surface.height(position).unwrap_or(0.) - 0.265;
    for (mode, mut transform, mut projection) in &mut camera {
        let target = production_follow_camera_transform(
            mode.mode,
            production_camera_extent(scene.profile_id),
            position,
        );
        transform.translation = transform.translation.lerp(target.translation, blend);
        transform.rotation = transform.rotation.slerp(target.rotation, blend);
        if let Projection::Orthographic(p) = projection.as_mut() {
            if let bevy::camera::ScalingMode::FixedVertical { viewport_height } = p.scaling_mode {
                let wanted = (5.6 / viewport_height).min(p.scale);
                p.scale = p.scale + (wanted - p.scale) * blend;
            }
        }
    }
    dock.framing = (!settle).then_some((focus, age + dt));
}
