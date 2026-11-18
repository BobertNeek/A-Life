//! A rattle can be pointed and rolled like a marble. Pearls show intention;
//! a ripple at the instrument means its physical body was actually struck.
use super::*;
use bevy::prelude::*;

pub(super) fn default_aim(
    world: &alife_world::HeadlessWorld,
    ball: &alife_world::WorldObject,
) -> Vec2 {
    world
        .entity_id("player-activity-toy")
        .and_then(|id| world.entity(id))
        .filter(|station| station.kind == WorldObjectKind::ActivityToy && !station.consumed)
        .map(|s| {
            Vec2::new(
                s.position.x - ball.position.x,
                s.position.z - ball.position.z,
            )
        })
        .filter(|offset| offset.length_squared() > 0.04)
        .unwrap_or(Vec2::new(2.2, 0.8))
        .clamp_length_max(6.)
}

#[cfg(feature = "gpu-runtime")]
pub(super) use live::install;
#[cfg(feature = "gpu-runtime")]
mod live {
    use super::*;

    #[derive(Resource, Default)]
    struct Gesture {
        anchor: Option<(WorldEntityId, Vec2)>,
        preview: Option<(Vec3, Vec2)>,
    }
    #[derive(Component)]
    enum Sign {
        Pearl(u8),
        Ripple(u8),
    }
    #[derive(Default)]
    struct Art {
        spawned: bool,
    }

    pub(in crate::production_voxel_renderer) fn install(app: &mut App) {
        app.init_resource::<Gesture>().add_systems(
            Update,
            (gesture.run_if(allowed), cancel.run_if(not(allowed)))
                .after(handle_fvr03_mouse_selection)
                .after(care_dock::collect_input)
                .after(menu::handle_input)
                .before(god_hand::input)
                .in_set(ProductionVoxelPresentationSet::Input),
        );
        app.add_systems(
            Update,
            draw.in_set(ProductionVoxelPresentationSet::AuthoritativeProjection),
        );
    }
    fn allowed(
        menu: Option<Res<menu::MenuState>>,
        dock: Option<Res<care_dock::State>>,
        conversation: Option<Res<ProductionConversationLineageUiState>>,
        ux: Option<Res<Fvr05ProductionUxStateResource>>,
    ) -> bool {
        menu::world_input_allowed(menu)
            && orchard::ordinary_hand(dock.as_ref().map(Res::clone))
            && care_dock::world_input_allowed(dock)
            && !conversation.is_some_and(|s| s.blocks_world_shortcuts())
            && !ux.is_some_and(|s| s.debug_mode)
    }
    fn cancel(mut state: ResMut<Gesture>) {
        state.anchor = None;
        state.preview = None;
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "Optical picking, physical world and window input are independent Bevy resources."
    )]
    fn gesture(
        mut state: ResMut<Gesture>,
        mouse: Res<ButtonInput<MouseButton>>,
        keys: Res<ButtonInput<KeyCode>>,
        windows: Query<&Window, With<PrimaryWindow>>,
        selection: Res<Fvr03ProductionVoxelSelectionResource>,
        hand: Res<god_hand::HandInteraction>,
        runtime: Option<NonSendMut<ProductionGpuBrainRuntimeResource>>,
    ) {
        state.preview = None;
        let Some(mut runtime) = runtime else {
            state.anchor = None;
            return;
        };
        let Some((cursor, ground)) = windows
            .single()
            .ok()
            .filter(|w| w.focused)
            .and_then(Window::cursor_position)
            .zip(hand.ground)
            .filter(|_| {
                selection.hovered.is_some()
                    && !keys.just_pressed(KeyCode::Escape)
                    && !mouse.pressed(MouseButton::Left)
            })
        else {
            state.anchor = None;
            return;
        };
        let world = runtime.runtime.world();
        let pointed = selection
            .hovered
            .and_then(|s| s.stable_id)
            .filter(|id| world.can_roll_toy(*id));
        if mouse.just_pressed(MouseButton::Right) {
            state.anchor = pointed.map(|id| (id, cursor));
        }
        if !mouse.pressed(MouseButton::Right) && !mouse.just_released(MouseButton::Right) {
            state.anchor = None;
        }
        let Some(id) = state
            .anchor
            .map(|p| p.0)
            .or(pointed)
            .filter(|id| world.can_roll_toy(*id))
        else {
            state.anchor = None;
            return;
        };
        let ball = world.entity(id).expect("loose ball was validated");
        let start = Vec3::new(ball.position.x, ball.position.y, ball.position.z);
        let aim = if state.anchor.is_some_and(|(_, p)| cursor.distance(p) > 7.) {
            (ground - start).xz().clamp_length_max(6.)
        } else {
            default_aim(world, ball)
        };
        state.preview = Some((start, aim));
        if mouse.just_released(MouseButton::Right) && state.anchor.take().is_some() {
            let _ = runtime.runtime.roll_player_toy(id, aim.to_array());
            state.preview = None;
        }
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "The two tiny optical effects share a persistent mesh pool."
    )]
    fn draw(
        mut commands: Commands,
        mut art: Local<Art>,
        state: Res<Gesture>,
        runtime: Option<NonSend<ProductionGpuBrainRuntimeResource>>,
        surface: Res<creature_grounding::RenderedTerrainSurface>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut signs: Query<(&Sign, &mut Transform, &mut Visibility)>,
    ) {
        let strike = runtime.as_ref().and_then(|r| {
            let world = r.runtime.world();
            let (tick, id) = world.last_chime_strike()?;
            let age = world.tick().raw().checked_sub(tick.raw())?;
            if age >= 25 {
                return None;
            }
            let p = world.entity(id)?.position;
            Some((Vec3::new(p.x, p.y, p.z), age as f32 / 25.))
        });
        if !art.spawned {
            if state.preview.is_none() && strike.is_none() {
                return;
            }
            let bead = meshes.add(Sphere::new(1.).mesh().ico(1).unwrap());
            let ring = meshes.add(Annulus::new(0.93, 1.));
            let pigment = |color: Color| StandardMaterial {
                base_color: color,
                unlit: true,
                ..default()
            };
            let pearl = materials.add(pigment(Color::srgb(0.91, 0.86, 0.55)));
            let chime = materials.add(pigment(Color::srgb(0.48, 0.96, 0.89)));
            for i in 0..5 {
                commands.spawn((
                    Name::new("Rattle aiming pearl"),
                    Sign::Pearl(i),
                    Mesh3d(bead.clone()),
                    MeshMaterial3d(pearl.clone()),
                    Transform::default(),
                    Visibility::Hidden,
                    bevy::picking::Pickable::IGNORE,
                ));
            }
            for i in 0..3 {
                commands.spawn((
                    Name::new("Struck chime ripple"),
                    Sign::Ripple(i),
                    Mesh3d(ring.clone()),
                    MeshMaterial3d(chime.clone()),
                    Transform::default(),
                    Visibility::Hidden,
                    bevy::picking::Pickable::IGNORE,
                ));
            }
            art.spawned = true;
        }
        for (sign, mut t, mut vis) in &mut signs {
            let pose = match sign {
                Sign::Pearl(i) => state
                    .preview
                    .filter(|(_, v)| v.length_squared() > 0.10)
                    .map(|(start, aim)| {
                        let f = (*i as f32 + 1.) / 6.;
                        let mut p = start + Vec3::new(aim.x, 0., aim.y) * f;
                        p.y = surface.height(p).unwrap_or(start.y)
                            + 0.085
                            + (f * std::f32::consts::PI).sin() * 0.10;
                        Transform::from_translation(p).with_scale(Vec3::splat(0.048 + f * 0.038))
                    }),
                Sign::Ripple(i) => strike.and_then(|(p, age)| {
                    let phase = age * 1.6 - f32::from(*i) * 0.29;
                    (0.0..1.).contains(&phase).then(|| {
                        let y = surface.height(p).unwrap_or(p.y) + 0.17 + phase * 0.42;
                        Transform::from_xyz(p.x, y, p.z)
                            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
                            .with_scale(Vec3::splat(0.32 + phase * 0.9))
                    })
                }),
            };
            if let Some(pose) = pose {
                t.set_if_neq(pose);
                vis.set_if_neq(Visibility::Visible);
            } else {
                vis.set_if_neq(Visibility::Hidden);
            }
        }
    }
}
