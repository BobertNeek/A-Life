//! Approved Blender character, projected onto existing organism identities.
use super::*;
use bevy::{camera::primitives::Aabb, gltf::Gltf, prelude::*, scene::SceneInstanceReady};

const PATH: &str = "creatures/hearthling/hearthling.glb";

// export_hearthling.py: each foot travels from -0.24 to +0.24 model units.
// Two steps cover 0.96 units per 24-frame cycle. The GLB contains three cycles,
// sampled at 24 fps starting at frame 1, rather than at time zero.
const WALK_CYCLE_DISTANCE: f32 = 0.96;
const WALK_CLIP_START: f32 = 1.0 / 24.0;
const WALK_CLIP_SECONDS: f32 = 3.0;

fn advance_walk(distance: f32, forward_scale: f32, seconds: f32) -> f32 {
    (seconds + distance / (WALK_CYCLE_DISTANCE * forward_scale)).rem_euclid(WALK_CLIP_SECONDS)
}

#[derive(Resource, Default)]
struct SharedHearthlingAssets {
    graph: Option<(Handle<AnimationGraph>, Vec<AnimationNodeIndex>)>,
    coats: BTreeMap<(bevy::asset::AssetId<StandardMaterial>, usize), Handle<StandardMaterial>>,
}

#[derive(Component)]
pub(super) struct HearthlingVisual {
    appearance: CreatureAppearanceGenome,
    previous_position: Vec3,
    facing: Quat,
    walk_seconds: f32,
    moved: bool,
}

#[derive(Component)]
pub(super) struct HearthlingPlayer {
    root: Entity,
    clips: Vec<AnimationNodeIndex>,
    state: usize,
}

pub(super) fn spawn(world: &mut World, root: Entity, appearance: CreatureAppearanceGenome) {
    world.init_resource::<SharedHearthlingAssets>();
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
            walk_seconds: 0.0,
            moved: false,
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
    mut shared: ResMut<SharedHearthlingAssets>,
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
    let (graph, clips) = shared
        .graph
        .get_or_insert_with(|| {
            let (graph, clips) = AnimationGraph::from_clips(
                [
                    "Hearthling_CuriousIdle",
                    "Hearthling_Walk",
                    "Hearthling_Sleep",
                ]
                .map(|name| gltf.named_animations[name].clone()),
            );
            (graphs.add(graph), clips)
        })
        .clone();
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
    let palette = usize::from(visual.appearance.palette_family) % tints.len();
    let tint = tints[palette];
    let coat_materials = [
        "Hearthling | vertex-colored coat",
        "Hearthling | warm ochre",
        "Hearthling | brows and tuft shadows",
    ]
    .map(|name| gltf.named_materials[name].id());
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
            // Covers every authored pose and inherited ear/tail scale. Static bind-pose
            // bounds can clip animated extremities; disabling culling costs every draw.
            commands
                .entity(entity)
                .insert(Aabb::from_min_max(Vec3::splat(-6.0), Vec3::splat(6.0)));
            if coat_materials.contains(&handle.0.id()) {
                let replacement =
                    shared
                        .coats
                        .entry((handle.0.id(), palette))
                        .or_insert_with(|| {
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
    // Bevy creates a Mesh handle for each material primitive inside a glTF mesh.
    let shared_meshes = mesh_count;
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
    mut players: Query<(&mut AnimationPlayer, &mut HearthlingPlayer)>,
    mut transforms: Query<(
        &mut Transform,
        &mut HearthlingVisual,
        &Fvr04ProductionCreatureVisualMarker,
    )>,
) {
    for (mut transform, mut visual, _) in &mut transforms {
        let delta = transform.translation - visual.previous_position;
        visual.previous_position = transform.translation;
        let distance = Vec2::new(delta.x, delta.z).length();
        visual.moved = !ux.settings.paused && distance > f32::EPSILON;
        if visual.moved {
            visual.facing = Quat::from_rotation_y(delta.x.atan2(delta.z));
            visual.walk_seconds = advance_walk(
                distance,
                transform.scale.z.abs().max(f32::EPSILON),
                visual.walk_seconds,
            );
        }
        if !ux.settings.paused {
            transform.rotation = transform
                .rotation
                .slerp(visual.facing, 1.0 - (-10.0 * time.delta_secs()).exp());
        }
    }
    for (mut player, mut model) in &mut players {
        let Ok((_, visual, marker)) = transforms.get(model.root) else {
            continue;
        };
        // Actual displacement can outlast a selected Move action (or be blocked
        // despite it). Pose cadence follows the presented world displacement.
        let next = if visual.moved {
            1
        } else {
            clip(marker.animation)
        };
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
            if next == 1 {
                // Seeking prevents clock time, frame rate, and simulation speed
                // from advancing the feet a second time after world movement.
                active
                    .set_speed(0.0)
                    .set_seek_time(WALK_CLIP_START + visual.walk_seconds);
            } else {
                active.set_speed(speed);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_step_tracks_authored_foot_travel_at_each_inherited_size() {
        for mass in 0..alife_world::CREATURE_APPEARANCE_GENE_BUCKETS {
            let appearance = CreatureAppearanceGenome {
                body_mass_trait: mass,
                ..Default::default()
            };
            let forward_scale = scale(appearance).z;
            let seconds = advance_walk(0.48 * forward_scale, forward_scale, 0.0);
            assert!((seconds - 0.5).abs() < 1e-6);
        }
    }

    #[test]
    fn walk_distance_is_frame_partition_independent_and_stops_without_motion() {
        let scale = 0.7;
        let distance = 1.3;
        let once = advance_walk(distance, scale, 0.0);
        let mut partitioned = 0.0;
        for _ in 0..120 {
            partitioned = advance_walk(distance / 120.0, scale, partitioned);
        }
        assert!((once - partitioned).abs() < 1e-5);
        assert_eq!(advance_walk(0.0, scale, partitioned), partitioned);
        assert!(advance_walk(100.0, scale, partitioned) < WALK_CLIP_SECONDS);
    }
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
