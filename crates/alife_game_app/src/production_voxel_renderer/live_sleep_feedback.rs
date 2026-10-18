//! Wordless sleep feedback, read only from the living individual's current sleep phase.
use super::*;
use bevy::prelude::Query;

// Three small rings per individual, bounded independently of population scale.
const MAX_CUES: usize = 8;

#[derive(Component)]
pub(super) struct SleepCue {
    root: Entity,
    stable_id: WorldEntityId,
    organism_id: OrganismId,
    seconds: f32,
}

#[derive(Component)]
pub(super) struct SleepBubble(usize);

#[derive(Default)]
pub(super) struct SharedAssets {
    handles: Option<(Handle<Mesh>, Handle<StandardMaterial>)>,
}

fn sleeping_individual(
    frame: &LiveBrainPresentationFrame,
    root: &ProductionCreatureAssemblyRoot,
    visual: &Fvr04ProductionCreatureVisualMarker,
) -> bool {
    sleeping_row(
        root,
        visual,
        frame.object(root.stable_id),
        frame.organism(root.stable_id),
    )
}

fn sleeping_row(
    root: &ProductionCreatureAssemblyRoot,
    visual: &Fvr04ProductionCreatureVisualMarker,
    object: Option<&alife_world::WorldObject>,
    row: Option<&WorldOrganismPresentationRow>,
) -> bool {
    let Some(object) = object else {
        return false;
    };
    let Some(row) = row else {
        return false;
    };
    visual.stable_id == root.stable_id
        && visual.organism_id == root.organism_id
        && object.id == root.stable_id
        && object.kind == WorldObjectKind::Agent
        && object.organism_id == Some(root.organism_id)
        && row.world_entity_id == root.stable_id
        && row.organism_id == root.organism_id
        && row.object.id == root.stable_id
        && row.object.kind == WorldObjectKind::Agent
        && row.object.organism_id == Some(root.organism_id)
        && row.lifecycle.is_alive()
        && matches!(
            row.sleep_phase,
            alife_core::SleepPhase::EnteringSleep
                | alife_core::SleepPhase::Consolidating
                | alife_core::SleepPhase::ForcedRecoverySleep
        )
}

fn cue_transform(root: &Transform, camera_rotation: Quat, seconds: f32) -> Transform {
    let breath = (seconds * 1.3).sin();
    // Follow the displayed pose and terrain height, including motion between ticks.
    // Keep the bubbles beside the silhouette, facing the player at either camera angle.
    let offset = camera_rotation * Vec3::new(0.32, 0.06 + breath * 0.025, 0.0);
    Transform {
        translation: Vec3::new(0.0, 1.90, 0.0) + root.rotation.inverse() * offset / root.scale,
        // Cancel nonuniform root scale before the children turn toward the camera.
        // Combining the inverse scale and billboard rotation here would shear them.
        rotation: Quat::IDENTITY,
        scale: root.scale.recip(),
    }
}

fn bubble_transform(
    root_rotation: Quat,
    camera_rotation: Quat,
    seconds: f32,
    index: usize,
) -> Transform {
    let (x, y, radius) = [(0.0, 0.0, 0.045), (0.14, 0.16, 0.065), (0.33, 0.35, 0.085)][index];
    let rotation = root_rotation.inverse() * camera_rotation;
    let pulse = 1.0 + (seconds * 1.3).sin() * 0.04;
    Transform {
        translation: rotation * Vec3::new(x, y, 0.0) * pulse,
        rotation: rotation * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
        scale: Vec3::splat(radius * pulse),
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "Bevy injects independent ECS parameters."
)]
pub(super) fn sync(
    mut commands: Commands,
    time: Res<Time>,
    ux: Option<Res<Fvr05ProductionUxStateResource>>,
    frame: Option<Res<LiveBrainPresentationFrameResource>>,
    roots: Query<
        (
            Entity,
            &ProductionCreatureAssemblyRoot,
            &Fvr04ProductionCreatureVisualMarker,
            &Transform,
        ),
        Without<SleepCue>,
    >,
    cameras: Query<&Transform, (With<Fvr03ProductionVoxelCamera>, Without<SleepCue>)>,
    mut cues: Query<
        (Entity, &mut SleepCue, &mut Transform, &Children),
        Without<ProductionCreatureAssemblyRoot>,
    >,
    mut bubbles: Query<
        (&SleepBubble, &mut Transform),
        (
            Without<SleepCue>,
            Without<ProductionCreatureAssemblyRoot>,
            Without<Fvr03ProductionVoxelCamera>,
        ),
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut shared: Local<SharedAssets>,
) {
    let paused = ux.as_ref().is_some_and(|ux| ux.settings.paused);
    let speed = ux.as_ref().map_or(1.0, |ux| ux.animation_speed);
    let camera_rotation = cameras.iter().next().map_or(Quat::IDENTITY, |t| t.rotation);
    // Reconcile even without a new world tick: missing authority must clear the cue,
    // and a held/interpolated root or moving camera still changes its displayed pose.
    let mut eligible = BTreeMap::new();
    if let Some(frame) = frame.as_ref() {
        for (entity, root, visual, transform) in &roots {
            if transform.scale.is_finite()
                && transform.scale.abs().min_element() > f32::EPSILON
                && sleeping_individual(&frame.current, root, visual)
            {
                eligible.insert(
                    (root.stable_id.raw(), root.organism_id.raw()),
                    (entity, *transform),
                );
            }
        }
    }
    // Stable IDs, rather than ECS iteration order, decide the bounded visible set.
    let mut eligible = eligible
        .into_iter()
        .take(MAX_CUES)
        .collect::<BTreeMap<_, _>>();
    for (entity, mut cue, mut transform, children) in &mut cues {
        let key = (cue.stable_id.raw(), cue.organism_id.raw());
        if let Some((_, pose)) = eligible.get(&key).filter(|(root, _)| *root == cue.root) {
            cue.seconds =
                advance_fvr04_animation_phase(cue.seconds, time.delta_secs(), speed, paused);
            let next = cue_transform(pose, camera_rotation, cue.seconds);
            if *transform != next {
                *transform = next;
            }
            for child in children {
                if let Ok((bubble, mut transform)) = bubbles.get_mut(*child) {
                    let next =
                        bubble_transform(pose.rotation, camera_rotation, cue.seconds, bubble.0);
                    if *transform != next {
                        *transform = next;
                    }
                }
            }
            eligible.remove(&key);
        } else {
            commands.entity(entity).despawn();
        }
    }
    if eligible.is_empty() {
        return;
    }
    let (mesh, material) = shared.handles.get_or_insert_with(|| {
        (
            meshes.add(Torus::new(0.72, 1.0)),
            materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 0.80, 0.43, 0.65),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
        )
    });
    for ((stable_id, organism_id), (root, pose)) in eligible {
        commands
            .spawn((
                Name::new("Live sleeping individual: quiet bubbles"),
                SleepCue {
                    root,
                    stable_id: WorldEntityId(stable_id),
                    organism_id: OrganismId(organism_id),
                    seconds: 0.0,
                },
                cue_transform(&pose, camera_rotation, 0.0),
                Visibility::Inherited,
                bevy::picking::Pickable::IGNORE,
                ChildOf(root),
            ))
            .with_children(|parent| {
                for index in 0..3 {
                    parent.spawn((
                        SleepBubble(index),
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(material.clone()),
                        bubble_transform(pose.rotation, camera_rotation, 0.0, index),
                        bevy::picking::Pickable::IGNORE,
                        bevy::light::NotShadowCaster,
                        bevy::light::NotShadowReceiver,
                    ));
                }
            });
    }
}

#[cfg(test)]
mod tests;
