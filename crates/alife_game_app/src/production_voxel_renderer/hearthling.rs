//! Approved Blender character, projected onto existing organism identities.
use super::*;
use bevy::{
    camera::visibility::NoFrustumCulling, gltf::Gltf, prelude::*, scene::SceneInstanceReady,
};

const PATH: &str = "creatures/hearthling/hearthling.glb";

#[derive(Component)]
pub(super) struct HearthlingVisual {
    appearance: CreatureAppearanceGenome,
    previous_position: Vec3,
    facing: Quat,
}

#[derive(Component)]
pub(super) struct HearthlingPlayer {
    root: Entity,
    clips: Vec<AnimationNodeIndex>,
    state: usize,
}

pub(super) fn spawn(world: &mut World, root: Entity, appearance: CreatureAppearanceGenome) {
    // Headless preflight/continuity apps retain organism roots without loading scenes.
    let Some(server) = world.get_resource::<AssetServer>() else {
        return;
    };
    let scene = server.load(GltfAssetLabel::Scene(0).from_asset(PATH));
    let gltf: Handle<Gltf> = server.load(PATH);
    let previous_position = world.get::<Transform>(root).unwrap().translation;
    world.entity_mut(root).insert((
        HearthlingVisual {
            appearance,
            previous_position,
            facing: Quat::IDENTITY,
        },
        HearthlingSource(gltf),
    ));
    world
        .spawn((
            Name::new("Approved Hearthling skinned scene"),
            SceneRoot(scene),
            Transform::default(),
            ChildOf(root),
        ))
        .observe(ready);
}

#[derive(Component)]
struct HearthlingSource(Handle<Gltf>);

#[derive(Component)]
pub(super) struct InheritedBoneScale(Vec3);

pub(super) fn apply_inherited_proportions(mut bones: Query<(&mut Transform, &InheritedBoneScale)>) {
    // All three authored clips sample scale on these bones, including at speed zero.
    // Apply the inherited multiplier after animation has restored the sampled scale.
    for (mut transform, scale) in &mut bones {
        transform.scale *= scale.0;
    }
}

pub(super) fn scale(appearance: CreatureAppearanceGenome) -> Vec3 {
    let mass = f32::from(appearance.body_mass_trait)
        / f32::from(alife_world::CREATURE_APPEARANCE_GENE_BUCKETS - 1);
    Vec3::new(0.60 + mass * 0.14, 0.67 + mass * 0.08, 0.62 + mass * 0.10)
}

fn clip(animation: CreatureAnimationState) -> usize {
    match animation {
        CreatureAnimationState::Sleeping => 2,
        CreatureAnimationState::Moving => 1,
        _ => 0,
    }
}

fn ready(
    event: On<SceneInstanceReady>,
    mut commands: Commands,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    roots: Query<(
        &HearthlingSource,
        &HearthlingVisual,
        &Fvr04ProductionCreatureVisualMarker,
    )>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut players: Query<&mut AnimationPlayer>,
    mesh_materials: Query<&MeshMaterial3d<StandardMaterial>>,
    names: Query<&Name>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(parent) = parents.get(event.entity) else {
        return;
    };
    let root = parent.parent();
    let Ok((source, visual, marker)) = roots.get(root) else {
        return;
    };
    let Some(gltf) = gltfs.get(&source.0) else {
        return;
    };
    let (graph, clips) = AnimationGraph::from_clips(
        [
            "Hearthling_CuriousIdle",
            "Hearthling_Walk",
            "Hearthling_Sleep",
        ]
        .map(|name| gltf.named_animations[name].clone()),
    );
    let graph = graphs.add(graph);
    let state = clip(marker.animation);
    // Tint only the coat. Eyes, nose, tongue and cream muzzle keep authored colors.
    let tints = [
        [1.0, 1.0, 1.0],
        [0.76, 1.06, 1.28],
        [1.18, 0.86, 0.76],
        [0.87, 0.86, 1.24],
        [0.86, 1.13, 0.83],
        [1.16, 1.03, 0.78],
    ];
    let tint = tints[usize::from(visual.appearance.palette_family) % tints.len()];
    let coat_materials = [
        "Hearthling | vertex-colored coat",
        "Hearthling | warm ochre",
        "Hearthling | brows and tuft shadows",
    ]
    .map(|name| gltf.named_materials[name].id());
    let mut coat_handles = BTreeMap::new();
    let mut mesh_count = 0;
    for entity in children.iter_descendants(event.entity) {
        if let Ok(name) = names.get(entity) {
            let ear = f32::from(visual.appearance.ear_muzzle_trait) / 15.0;
            let tail = f32::from(visual.appearance.tail_trait) / 15.0;
            let scale = match name.as_str() {
                "ear.L" | "ear.R" => Some(Vec3::new(1.0, 0.85 + ear * 0.30, 1.0)),
                "head" => Some(Vec3::new(0.94 + ear * 0.12, 1.0, 1.0)),
                "tail.00" => Some(Vec3::splat(0.90 + tail * 0.20)),
                _ => None,
            };
            if let Some(scale) = scale {
                commands.entity(entity).insert(InheritedBoneScale(scale));
            }
        }
        if let Ok(mut player) = players.get_mut(entity) {
            player
                .play(clips[state])
                .repeat()
                .set_seek_time(marker.phase % 3.0)
                .set_speed(0.0);
            commands.entity(entity).insert((
                AnimationGraphHandle(graph.clone()),
                HearthlingPlayer {
                    root,
                    clips: clips.clone(),
                    state,
                },
            ));
        }
        if let Ok(handle) = mesh_materials.get(entity) {
            mesh_count += 1;
            commands.entity(entity).insert(NoFrustumCulling);
            if coat_materials.contains(&handle.0.id()) {
                let replacement = coat_handles.entry(handle.0.id()).or_insert_with(|| {
                    let mut material = materials
                        .get(&handle.0)
                        .expect("loaded glTF material")
                        .clone();
                    let color = material.base_color.to_linear();
                    material.base_color = Color::linear_rgba(
                        color.red * tint[0],
                        color.green * tint[1],
                        color.blue * tint[2],
                        color.alpha,
                    );
                    materials.add(material)
                });
                commands
                    .entity(entity)
                    .insert(MeshMaterial3d(replacement.clone()));
            }
        }
    }
    let shared_meshes = gltf.meshes.len();
    commands.queue(move |world: &mut World| {
        if let Some(mut receipt) = world.get_resource_mut::<Fvr04ProductionCreatureSceneResource>()
        {
            receipt.creature_part_entity_count += mesh_count;
            receipt.mesh_pool_count = shared_meshes;
            receipt.creature_shared_mesh_handle_count = shared_meshes;
        }
    });
    info!(
        "Hearthling ready: stable {}, palette {}, state {}",
        marker.stable_id.raw(),
        visual.appearance.palette_family,
        state
    );
}

pub(super) fn animate(
    time: Res<Time>,
    ux: Res<Fvr05ProductionUxStateResource>,
    roots: Query<&Fvr04ProductionCreatureVisualMarker>,
    mut players: Query<(&mut AnimationPlayer, &mut HearthlingPlayer)>,
    mut transforms: Query<(&mut Transform, &mut HearthlingVisual)>,
) {
    for (mut transform, mut visual) in &mut transforms {
        let delta = transform.translation - visual.previous_position;
        visual.previous_position = transform.translation;
        if !ux.settings.paused && Vec2::new(delta.x, delta.z).length_squared() > 0.0001 {
            visual.facing = Quat::from_rotation_y(delta.x.atan2(delta.z));
        }
        if !ux.settings.paused {
            transform.rotation = transform
                .rotation
                .slerp(visual.facing, 1.0 - (-10.0 * time.delta_secs()).exp());
        }
    }
    for (mut player, mut model) in &mut players {
        let Ok(marker) = roots.get(model.root) else {
            continue;
        };
        let next = clip(marker.animation);
        if next != model.state {
            player.stop_all();
            player
                .play(model.clips[next])
                .repeat()
                .set_seek_time(marker.phase % 3.0);
            model.state = next;
        }
        let speed = if ux.settings.paused {
            0.0
        } else {
            ux.settings.simulation_speed
        };
        for (_, active) in player.playing_animations_mut() {
            active.set_speed(speed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn renderless_preflight_retains_the_organism_root_without_an_asset_server() {
        let mut world = World::new();
        let root = world.spawn(Transform::default()).id();
        spawn(&mut world, root, CreatureAppearanceGenome::default());
        assert!(world.get::<Transform>(root).is_some());
        assert!(world.get::<HearthlingVisual>(root).is_none());
    }
    #[test]
    fn sleep_and_locomotion_use_distinct_authored_clips() {
        assert_ne!(
            clip(CreatureAnimationState::Sleeping),
            clip(CreatureAnimationState::Idle)
        );
        assert_ne!(
            clip(CreatureAnimationState::Moving),
            clip(CreatureAnimationState::Idle)
        );
    }
}
