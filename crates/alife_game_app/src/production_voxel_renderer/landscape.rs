//! Bounded, deterministic groves and ground cover using the approved Blender props.
use super::*;
use bevy::prelude::*;

pub(super) fn spawn(
    world: &mut World,
    samples: &ProductionTerrainSampleMap,
    snapshot: &PersistentVoxelWorldSnapshot,
    stride: f32,
) {
    let Some(server) = world.get_resource::<AssetServer>() else {
        return;
    };
    let paths = [
        "Oak_variant_1.glb",
        "Oak_variant_2.glb",
        "Oak_variant_3.glb",
        "Boulder_variant_1.glb",
        "Boulder_variant_2.glb",
        "Boulder_variant_3.glb",
        "Berry_shrub.glb",
        "Grass_clump.glb",
    ];
    let scenes: Vec<Handle<Scene>> = paths
        .iter()
        .map(|path| server.load(GltfAssetLabel::Scene(0).from_asset(format!("landscape/{path}"))))
        .collect();
    let mut roots = world.query_filtered::<&Transform, With<ProductionCreatureAssemblyRoot>>();
    let occupied: Vec<Vec3> = roots
        .iter(world)
        .map(|t| t.translation)
        .chain(
            snapshot
                .creatures
                .iter()
                .map(|c| Vec3::new(c.tile.x as f32 + 0.5, 0.0, c.tile.z as f32 + 0.5)),
        )
        .collect();
    let mut cameras = world.query_filtered::<&Transform, With<Fvr03ProductionVoxelCamera>>();
    let center = cameras
        .iter(world)
        .next()
        .map(|camera| {
            let ray = camera.rotation * Vec3::NEG_Z;
            camera.translation + ray * (-camera.translation.y / ray.y.min(-0.01))
        })
        .or_else(|| occupied.first().copied())
        .unwrap_or(Vec3::ZERO);
    // Fill the player's neighborhood first. Lexicographic tile order spent the
    // dressing budget at the far edge and left the actual playfield bare.
    let mut candidates = samples
        .values()
        .filter(|s| s.material != Fvr03ProductionVoxelMaterialKind::Water)
        .collect::<Vec<_>>();
    candidates.sort_by(|a, b| {
        let distance = |s: &ProductionTerrainSample| {
            (s.center_x - center.x).powi(2) + (s.center_z - center.z).powi(2)
        };
        distance(a).total_cmp(&distance(b))
    });
    let clear = |position: Vec3| {
        occupied
            .iter()
            .all(|p| (p.x - position.x).hypot(p.z - position.z) > 3.2)
    };
    let mut anchors: Vec<Vec3> = Vec::new();
    let mut count = 0;
    for sample in &candidates {
        let hash = tile_hash(sample.tile);
        if hash % 37 != 0 {
            continue;
        }
        let anchor = Vec3::new(sample.center_x, sample.height, sample.center_z);
        if anchors
            .iter()
            .any(|p| (p.x - anchor.x).hypot(p.z - anchor.z) < 4.2)
        {
            continue;
        }
        anchors.push(anchor);
        let grassy = matches!(
            sample.material,
            Fvr03ProductionVoxelMaterialKind::SafeGrass
                | Fvr03ProductionVoxelMaterialKind::Resource
        );
        if clear(anchor) {
            let choice = if grassy {
                hash as usize % 3
            } else {
                3 + hash as usize % 3
            };
            count += place(
                world,
                samples,
                stride,
                &scenes[choice],
                paths[choice],
                anchor,
                0.55 + (hash % 7) as f32 * 0.035,
                hash,
            );
            if grassy {
                let shrub = anchor + Vec3::new(1.7, 0.0, -1.2);
                if clear(shrub) {
                    count += place(
                        world,
                        samples,
                        stride,
                        &scenes[6],
                        paths[6],
                        shrub,
                        0.7,
                        hash.wrapping_add(1),
                    );
                }
                let other = (choice + 1) % 3;
                let second = anchor + Vec3::new(-2.0, 0.0, 1.4);
                if hash % 3 == 0 && clear(second) {
                    count += place(
                        world,
                        samples,
                        stride,
                        &scenes[other],
                        paths[other],
                        second,
                        0.44,
                        hash.wrapping_add(2),
                    );
                }
            }
        }
        for i in 0..18_u32 {
            let angle = i as f32 * 2.39996 + (hash % 100) as f32 * 0.1;
            let radius = 0.5 + (hash.wrapping_add(i * 7) % 19) as f32 * 0.115;
            let position = anchor + Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
            count += place(
                world,
                samples,
                stride,
                &scenes[7],
                paths[7],
                position,
                0.55 + (i % 5) as f32 * 0.16,
                hash.wrapping_add(i),
            );
        }
        if anchors.len() >= 32 {
            break;
        }
    }
    // Small irregular grass islands also connect groves across open meadow.
    for sample in candidates
        .into_iter()
        .filter(|s| {
            matches!(
                s.material,
                Fvr03ProductionVoxelMaterialKind::SafeGrass
                    | Fvr03ProductionVoxelMaterialKind::Resource
            )
        })
        .take(480)
    {
        let hash = tile_hash(sample.tile);
        if hash % 3 != 0 {
            continue;
        }
        let position = Vec3::new(
            sample.center_x + (hash % 11) as f32 * 0.06 - 0.3,
            sample.height,
            sample.center_z + (hash % 7) as f32 * 0.08 - 0.24,
        );
        count += place(
            world,
            samples,
            stride,
            &scenes[7],
            paths[7],
            position,
            0.65 + (hash % 5) as f32 * 0.12,
            hash,
        );
    }
    info!("Approved landscape: {count} grouped Blender props");
}

fn tile_hash(tile: VoxelTileCoord) -> u32 {
    (tile.x as u32).wrapping_mul(73856093) ^ (tile.z as u32).wrapping_mul(19349663)
}

fn place(
    world: &mut World,
    samples: &ProductionTerrainSampleMap,
    stride: f32,
    scene: &Handle<Scene>,
    name: &str,
    mut position: Vec3,
    scale: f32,
    hash: u32,
) -> usize {
    let coordinate = |x: f32| ((x + stride * 0.5 - 0.5) / stride).floor() as i32 * stride as i32;
    let tile = VoxelTileCoord::new(coordinate(position.x), coordinate(position.z));
    let Some(sample) = samples.get(&tile) else {
        return 0;
    };
    if sample.material == Fvr03ProductionVoxelMaterialKind::Water {
        return 0;
    }
    position.y = world
        .resource::<creature_grounding::RenderedTerrainSurface>()
        .height(position)
        .unwrap_or(sample.height)
        - 0.015;
    let mut entity = world.spawn((
        Name::new(format!("Approved landscape {name}")),
        SceneRoot(scene.clone()),
        Transform::from_translation(position)
            .with_scale(Vec3::splat(scale))
            .with_rotation(Quat::from_rotation_y((hash % 628) as f32 * 0.01)),
        Fvr04ProductionRuntimeSceneRoot,
        Fvr07ProductionVisualDressing {
            kind: Fvr07ProductionDressingKind::LeafPatch,
            tile,
            display_only: true,
            no_renderer_authority_over_actions_or_cognition: true,
        },
    ));
    if name.starts_with("Oak_") {
        entity.insert(Canopy);
    }
    1
}

#[derive(Component)]
pub(super) struct Canopy;

/// Keep creatures readable when a decorative tree crosses the camera's sight line.
pub(super) fn reveal_creatures(
    cameras: Query<&Transform, With<Fvr03ProductionVoxelCamera>>,
    creatures: Query<&Transform, With<ProductionCreatureAssemblyRoot>>,
    mut trees: Query<(&Transform, &mut Visibility), With<Canopy>>,
) {
    let Ok(camera) = cameras.single() else {
        return;
    };
    for (tree, mut visibility) in &mut trees {
        let center = tree.translation + Vec3::Y * (6.5 * tree.scale.y);
        // Bounds of the approved Oak GLB canopies, in their local coordinates.
        let radius = 4.5 * tree.scale.x;
        let obscures = creatures.iter().any(|creature| {
            let sight = creature.translation + Vec3::Y * 0.8 - camera.translation;
            let t = (center - camera.translation).dot(sight) / sight.length_squared().max(0.001);
            t > 0.0
                && t < 1.0
                && (center - camera.translation - sight * t).length_squared() < radius * radius
        });
        *visibility = if obscures {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}
