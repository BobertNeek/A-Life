//! Bounded presentation terrain around the camera. Never publishes world state.

use super::*;

#[derive(Resource)]
pub(super) struct CameraTerrainStream {
    backend: PersistentVoxelWorldBackend,
    anchor_id: Option<WorldEntityId>,
    focus_chunk: Option<VoxelChunkCoord>,
}

impl CameraTerrainStream {
    pub(super) fn new(
        backend: PersistentVoxelWorldBackend,
        anchor_id: Option<WorldEntityId>,
    ) -> Self {
        Self {
            backend,
            anchor_id,
            focus_chunk: None,
        }
    }
}

pub(super) fn stream_camera_terrain(world: &mut World) {
    let Some(stream) = world.get_resource::<CameraTerrainStream>() else {
        return;
    };
    let Some(anchor_id) = stream.anchor_id else {
        return;
    };
    let previous = stream.focus_chunk;
    let mut cameras = world.query_filtered::<&Transform, With<Fvr03ProductionVoxelCamera>>();
    let Some(camera) = cameras.iter(world).next() else {
        return;
    };
    let direction = camera.rotation * Vec3::NEG_Z;
    if direction.y >= -0.001 {
        return;
    }
    let focus = camera.translation + direction * (-camera.translation.y / direction.y);
    let chunk = VoxelChunkCoord::for_tile(
        16,
        VoxelTileCoord::new(focus.x.floor() as i32, focus.z.floor() as i32),
    );
    if previous == Some(chunk) {
        return;
    }
    // Build everything before replacing the old ground. Creature roots and the GPU
    // runtime remain untouched. The anchor only selects a read-only terrain region.
    let result = (|| -> Result<Fvr04RuntimeSceneCandidate, GameAppShellError> {
        let backend = world.resource::<CameraTerrainStream>().backend.clone();
        let anchor = CreatureWorldAnchor::new(anchor_id, Vec3f::new(focus.x, 0.0, focus.z))?;
        let mut snapshot = backend.snapshot_for_anchors(&[anchor])?;
        snapshot.creatures.clear();
        snapshot
            .selection_refs
            .retain(|reference| reference.kind != StableVoxelRefKind::Creature);
        let scene = world.resource::<Fvr03ProductionVoxelSceneResource>();
        let settings = Fvr03ProductionVoxelRendererSettings::for_profile(scene.profile_id);
        prepare_fvr04_runtime_scene_candidate(
            Fvr04RuntimeSceneState {
                backend,
                snapshot,
                creatures: Vec::new(),
            },
            settings,
            &world.resource::<Fvr05ProductionUxStateResource>().settings,
            world.resource::<Fvr04CreatureSpawnContext>(),
        )
    })();
    let candidate = match result {
        Ok(candidate) => candidate,
        Err(error) => {
            world
                .resource_mut::<Fvr05ProductionUxStateResource>()
                .last_error = Some(format!("Camera terrain: {error}"));
            return;
        }
    };
    let mut old_meshes = world.query_filtered::<(Entity, &Mesh3d), bevy::prelude::Or<(
        With<Fvr11ProductionTerrainLayer>,
        With<Fvr05ProductionOverlayBatch>,
    )>>();
    let old_meshes = old_meshes
        .iter(world)
        .map(|(entity, mesh)| (entity, mesh.0.id()))
        .collect::<Vec<_>>();
    for (entity, mesh) in old_meshes {
        world.despawn(entity);
        world.resource_mut::<Assets<Mesh>>().remove(mesh);
    }
    let mut dressing = world.query_filtered::<Entity, With<Fvr07ProductionVisualDressing>>();
    let dressing = dressing.iter(world).collect::<Vec<_>>();
    for entity in dressing {
        world.despawn(entity);
    }
    world.resource_scope(
        |world, assets: bevy::prelude::Mut<Fvr04RuntimeSceneAssets>| {
            let receipt = spawn_fvr11_layered_terrain_meshes(
                world,
                &assets.terrain_materials,
                &candidate.settings,
                &candidate.runtime_state.snapshot,
                &candidate.terrain_samples,
                candidate.terrain_build,
                candidate.tile_mesh_count,
            );
            spawn_fvr05_overlay_batches(world, candidate.overlay_spawns, &assets.overlay_materials);
            let polish = spawn_fvr07_production_visual_polish(
                world,
                &candidate.settings,
                candidate.dressing_spawns,
                Vec::new(),
                &assets.dressing_library,
                &assets.vfx_unit_mesh,
                &assets.vfx_materials,
            );
            let mut terrain = world.resource_mut::<Fvr11ProductionTerrainSceneResource>();
            terrain.sample_count = candidate.terrain_samples.len();
            terrain.top_layer_count = receipt.top_layer_count;
            terrain.cliff_layer_count = receipt.cliff_layer_count;
            terrain.transition_edge_count = receipt.transition_edge_count;
            terrain.water_layer_count = receipt.water_layer_count;
            terrain.confetti_detail_quad_count = receipt.confetti_detail_quad_count;
            let mut scene = world.resource_mut::<Fvr03ProductionVoxelSceneResource>();
            scene.visible_chunk_count = candidate.visible_chunks.len();
            scene.resident_chunk_count = candidate.visible_chunks.len();
            scene.tile_mesh_count = candidate.tile_mesh_count;
            scene.estimated_resident_bytes = fvr03_estimated_resident_bytes(
                candidate.tile_mesh_count,
                candidate.visible_chunks.len(),
            );
            scene.average_resource_bias =
                fvr05_average_resource_bias(&candidate.tile_summaries_by_tile);
            scene.average_hazard_pressure =
                fvr05_average_hazard_pressure(&candidate.tile_summaries_by_tile);
            scene.visible_tiles = candidate.visible_tiles;
            scene.visible_chunks = candidate.visible_chunks;
            scene.tile_summaries_by_tile = candidate.tile_summaries_by_tile;
            scene.material_counts = candidate.material_counts;
            scene.mesh_stats = receipt.mesh_stats;
            scene.production_dressing_count = polish.dressing_count;
        },
    );
    world.resource_mut::<CameraTerrainStream>().focus_chunk = Some(chunk);
}
