//! Live authoritative creature projection into Bevy transforms and selection indexes.

use super::*;

pub(super) fn project_live_world_to_fvr04_creature_roots(world: &mut World) {
    if !world.contains_resource::<LiveBrainPresentationFrameResource>() {
        return;
    }

    world.resource_scope(
        |world, frame: bevy::prelude::Mut<LiveBrainPresentationFrameResource>| {
            if !frame.is_changed() {
                return;
            }
            let mut animation_by_stable_id = BTreeMap::new();
            world.resource_scope(
                |world, mut scene: bevy::prelude::Mut<Fvr03ProductionVoxelSceneResource>| {
                    scene.creature_refs_by_tile.clear();
                    scene.selection_positions_by_raw_id.clear();
                    let mut rendered_count = 0_usize;
                    let mut roots = world.query::<(
                        &ProductionCreatureAssemblyRoot,
                        &mut Fvr03ProductionVoxelCreatureMarker,
                        &mut Fvr04ProductionCreatureVisualMarker,
                        &mut Transform,
                    )>();
                    for (root, mut creature, mut visual, mut transform) in roots.iter_mut(world) {
                        let Some(object) = frame.current.object(root.stable_id) else {
                            continue;
                        };
                        if object.kind != WorldObjectKind::Agent
                            || object.organism_id != Some(root.organism_id)
                        {
                            continue;
                        }
                        let tile = VoxelTileCoord::new(
                            object.position.x.round() as i32,
                            object.position.z.round() as i32,
                        );
                        let surface_height = scene
                            .tile_summaries_by_tile
                            .get(&tile)
                            .map(|summary| summary.height_units)
                            .unwrap_or(visual.surface_height);
                        let mut projected = *transform;
                        if !project_authoritative_creature_root_transform(
                            root.stable_id,
                            root.organism_id,
                            &mut projected,
                            &frame.current,
                        ) {
                            continue;
                        }
                        projected.translation.y = grounded_root_height(
                            surface_height,
                            0.04,
                            visual.local_bounds,
                            visual.base_scale.to_array(),
                            bevy::math::Mat3::IDENTITY.to_cols_array(),
                        );
                        if *transform != projected {
                            *transform = projected;
                        }
                        if creature.tile != tile {
                            creature.tile = tile;
                        }
                        if visual.tile != tile {
                            visual.tile = tile;
                        }
                        if visual.surface_height != surface_height {
                            visual.surface_height = surface_height;
                        }
                        if visual.base_translation != projected.translation {
                            visual.base_translation = projected.translation;
                        }

                        if let Some(row) = frame.current.organism(root.stable_id) {
                            let selected_action_kind =
                                row.motor.as_ref().and_then(|motor| motor.action_kind);
                            if let Ok(snapshot) = crate::creature_visual_snapshot_from_parts(
                                row.organism_id,
                                row.world_entity_id,
                                row.object.position,
                                None,
                                None,
                                &row.biochemistry.homeostasis,
                                row.sleep_phase,
                                selected_action_kind,
                            ) {
                                if visual.expression != snapshot.expression {
                                    visual.expression = snapshot.expression;
                                }
                                if visual.animation != snapshot.animation {
                                    visual.animation = snapshot.animation;
                                }
                            }
                        }
                        animation_by_stable_id.insert(root.stable_id.raw(), visual.animation);
                        let stable_ref = StableVoxelObjectRef {
                            kind: StableVoxelRefKind::Creature,
                            stable_id: Some(root.stable_id),
                            chunk: scene
                                .tile_summaries_by_tile
                                .get(&tile)
                                .map(|summary| summary.chunk)
                                .unwrap_or_else(|| VoxelChunkCoord::for_tile(16, tile)),
                            tile: Some(tile),
                        };
                        scene.creature_refs_by_tile.insert(tile, stable_ref);
                        scene
                            .selection_positions_by_raw_id
                            .insert(root.stable_id.raw(), projected.translation);
                        rendered_count = rendered_count.saturating_add(1);
                    }
                    scene.creature_render_count = rendered_count;
                    scene.creature_root_count = rendered_count;
                },
            );
            let mut parts = world.query::<&mut ProductionCreaturePartMarker>();
            for mut part in parts.iter_mut(world) {
                if let Some(animation) = animation_by_stable_id.get(&part.stable_id.raw()) {
                    if part.animation != *animation {
                        part.animation = *animation;
                    }
                }
            }

            #[cfg(feature = "gpu-runtime")]
            {
                let pending_newborns = {
                    let Some(entity_map) = world.get_resource::<BevyEntityMap>() else {
                        return;
                    };
                    frame
                        .current
                        .objects()
                        .filter(|object| {
                            if object.kind != WorldObjectKind::Agent {
                                return false;
                            }
                            let Some(organism_id) = object.organism_id else {
                                return false;
                            };
                            let Some(entity) = entity_map.bevy_entity(object.id) else {
                                return true;
                            };
                            let root = world.get::<ProductionCreatureAssemblyRoot>(entity);
                            let visual = world.get::<Fvr04ProductionCreatureVisualMarker>(entity);
                            !matches!(
                                (root, visual),
                                (Some(root), Some(visual))
                                    if root.stable_id == object.id
                                        && root.organism_id == organism_id
                                        && visual.stable_id == object.id
                                        && visual.organism_id == organism_id
                            )
                        })
                        .collect::<Vec<_>>()
                };
                if pending_newborns.is_empty() {
                    return;
                }

                let Some(world_seed) = world
                    .get_non_send_resource::<ProductionGpuBrainRuntimeResource>()
                    .map(|runtime| runtime.runtime.world_seed())
                else {
                    return;
                };
                let tile_summaries = world
                    .get_resource::<Fvr03ProductionVoxelSceneResource>()
                    .map(|scene| scene.tile_summaries_by_tile.clone())
                    .unwrap_or_default();
                let mut newborns = Vec::with_capacity(pending_newborns.len());
                for object in pending_newborns {
                    let tile = VoxelTileCoord::new(
                        object.position.x.round() as i32,
                        object.position.z.round() as i32,
                    );
                    let chunk = tile_summaries
                        .get(&tile)
                        .map(|summary| summary.chunk)
                        .unwrap_or_else(|| VoxelChunkCoord::new(0, 0));
                    if let Some(record) = fvr04_live_creature_visual_record(
                        &frame.current,
                        world_seed,
                        object,
                        tile,
                        chunk,
                    ) {
                        newborns.push(record);
                    }
                }
                let max_visible = world
                    .get_resource::<Fvr04CreatureSpawnContext>()
                    .map(|context| usize::from(context.settings.max_visible_creatures))
                    .unwrap_or(0);
                let current_visible = world
                    .get_resource::<Fvr03ProductionVoxelSceneResource>()
                    .map(|scene| scene.creature_render_count)
                    .unwrap_or(0);
                newborns.truncate(max_visible.saturating_sub(current_visible));
                if newborns.is_empty() {
                    return;
                }

                let prepared = {
                    let Some(mut context) = world.remove_resource::<Fvr04CreatureSpawnContext>()
                    else {
                        return;
                    };
                    let result = prepare_fvr04_creature_batch(
                        world,
                        &newborns,
                        &tile_summaries,
                        &mut context,
                    );
                    world.insert_resource(context);
                    let Ok(prepared) = result else {
                        return;
                    };
                    prepared
                };
                let added_scene = spawn_fvr04_prepared_creature_batch(world, prepared);
                let added_count = added_scene.rendered_creature_count;
                if let Some(mut scene) =
                    world.get_resource_mut::<Fvr04ProductionCreatureSceneResource>()
                {
                    append_fvr04_creature_scene_resource(&mut scene, added_scene);
                }
                if let Some(mut scene) =
                    world.get_resource_mut::<Fvr03ProductionVoxelSceneResource>()
                {
                    scene.creature_render_count =
                        scene.creature_render_count.saturating_add(added_count);
                    scene.creature_root_count =
                        scene.creature_root_count.saturating_add(added_count);
                    for newborn in newborns.into_iter().take(added_count) {
                        scene
                            .creature_refs_by_tile
                            .insert(newborn.tile, newborn.stable_ref);
                        scene.selection_positions_by_raw_id.insert(
                            newborn.visual.stable_id.raw(),
                            Vec3::new(
                                newborn.tile.x as f32 + 0.5,
                                1.52,
                                newborn.tile.z as f32 + 0.5,
                            ),
                        );
                    }
                }
            }
        },
    );
}
