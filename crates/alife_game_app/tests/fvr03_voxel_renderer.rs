#![cfg(feature = "bevy-app")]

use std::collections::BTreeSet;

use alife_game_app::{
    default_environment_manifest_path, Fvr03ProductionVoxelCamera, Fvr03ProductionVoxelCameraMode,
    Fvr03ProductionVoxelChunk, Fvr03ProductionVoxelMaterialKind, Fvr03ProductionVoxelSceneResource,
    Fvr03ProductionVoxelSelectionResource, Fvr03ProductionVoxelTerrainBatch,
    Fvr03ProductionVoxelTerrainTile, Fvr04ProductionCreatureVisualMarker,
    Fvr05ProductionUxStateResource, Fvr07ProductionDressingKind, Fvr07ProductionGpuVfxMarker,
    Fvr07ProductionVfxKind, Fvr07ProductionVisualDressing, Fvr09CreatureFaceFeatureMarker,
    Fvr09CuteBipedCreatureMarker, Fvr09MesherMode, ProductionFrontendProfileId,
    ProductionVoxelLaunchConfig, FVR03_PRODUCTION_VOXEL_RENDERER_SCHEMA,
};
use alife_world::{StableVoxelRefKind, FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA};
use bevy::{
    mesh::VertexAttributeValues,
    prelude::{Assets, Mesh, Mesh3d, MeshMaterial3d, Projection, StandardMaterial, Transform},
};

fn production_launch(profile_id: ProductionFrontendProfileId) -> ProductionVoxelLaunchConfig {
    let mut launch = ProductionVoxelLaunchConfig::from_manifest(
        default_environment_manifest_path(),
        None,
        profile_id,
    )
    .unwrap();
    launch.population = Some(profile_id.budget().default_population);
    launch.smoke_seconds = Some(1);
    launch.dry_run = true;
    launch
}

fn quantized_rgba(color: [f32; 4]) -> [i32; 4] {
    [
        (color[0] * 255.0).round() as i32,
        (color[1] * 255.0).round() as i32,
        (color[2] * 255.0).round() as i32,
        (color[3] * 255.0).round() as i32,
    ]
}

#[test]
fn fvr03_voxel_app_spawns_real_persistent_chunks_by_default() {
    let launch = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
    let (mut app, summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();

    let scene = app
        .world()
        .resource::<Fvr03ProductionVoxelSceneResource>()
        .clone();
    assert_eq!(scene.schema, FVR03_PRODUCTION_VOXEL_RENDERER_SCHEMA);
    assert_eq!(scene.snapshot_schema, FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA);
    assert_eq!(
        scene.profile_id,
        ProductionFrontendProfileId::MinimumSettings30x30
    );
    assert_eq!(scene.population, 30);
    assert!(scene.uses_bevy_voxel_world_backend);
    assert!(scene.uses_internal_chunk_mesh_for_fvr02_contract);
    assert!(scene.visible_chunk_count > 0);
    assert_eq!(scene.visible_chunk_count, scene.resident_chunk_count);
    assert!(scene.resident_chunk_count <= summary.profile_budget.active_chunk_cap as usize);
    assert!(scene.tile_mesh_count >= scene.resident_chunk_count);
    assert!(scene.selection_ref_count >= summary.save_metadata.creature_count);
    assert!(scene.estimated_resident_bytes > 0);
    assert!(scene.no_renderer_authority_over_world_truth);
    assert_eq!(scene.production_vfx_budget_state, "conservative");
    assert!(scene.production_visuals_display_only);
    assert!(scene.production_dressing_count >= 8);
    assert!(scene.production_dressing_count <= 48);
    assert!(scene.production_vfx_marker_count >= 8);
    assert!(scene.production_vfx_marker_count <= 32);

    let mut chunk_query = app.world_mut().query::<&Fvr03ProductionVoxelChunk>();
    assert_eq!(
        chunk_query.iter(app.world()).count(),
        scene.resident_chunk_count
    );

    let mut tile_query = app.world_mut().query::<&Fvr03ProductionVoxelTerrainTile>();
    let tiles = tile_query.iter(app.world()).copied().collect::<Vec<_>>();
    assert!(tiles.len() >= scene.resident_chunk_count);
    assert!(tiles
        .iter()
        .all(|tile| tile.stable_ref.kind == StableVoxelRefKind::Tile));
    assert!(tiles
        .iter()
        .all(|tile| !format!("{:?}", tile.stable_ref).contains("Entity(")));

    let materials = tiles
        .iter()
        .map(|tile| tile.material)
        .collect::<BTreeSet<_>>();
    for required in [
        Fvr03ProductionVoxelMaterialKind::SafeGrass,
        Fvr03ProductionVoxelMaterialKind::Water,
        Fvr03ProductionVoxelMaterialKind::Resource,
        Fvr03ProductionVoxelMaterialKind::Hazard,
        Fvr03ProductionVoxelMaterialKind::Decay,
    ] {
        assert!(
            materials.contains(&required),
            "missing material {required:?}"
        );
    }

    let mut dressing_query = app.world_mut().query::<&Fvr07ProductionVisualDressing>();
    let dressing = dressing_query
        .iter(app.world())
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(dressing.len(), scene.production_dressing_count);
    assert!(dressing
        .iter()
        .all(|entry| entry.display_only && entry.no_renderer_authority_over_actions_or_cognition));
    let dressing_kinds = dressing
        .iter()
        .map(|entry| entry.kind)
        .collect::<BTreeSet<_>>();
    for required in [
        Fvr07ProductionDressingKind::LeafPatch,
        Fvr07ProductionDressingKind::MushroomCluster,
        Fvr07ProductionDressingKind::PebbleCluster,
        Fvr07ProductionDressingKind::NestMarker,
        Fvr07ProductionDressingKind::FoodResource,
    ] {
        assert!(
            dressing_kinds.contains(&required),
            "missing dressing {required:?}"
        );
    }

    let mut vfx_query = app.world_mut().query::<&Fvr07ProductionGpuVfxMarker>();
    let vfx = vfx_query.iter(app.world()).copied().collect::<Vec<_>>();
    assert_eq!(vfx.len(), scene.production_vfx_marker_count);
    assert!(vfx.iter().all(|entry| entry.display_only
        && entry.no_renderer_authority_over_actions_or_cognition
        && entry.budget_state == "conservative"));
    let vfx_kinds = vfx.iter().map(|entry| entry.kind).collect::<BTreeSet<_>>();
    for required in [
        Fvr07ProductionVfxKind::PheromoneTrail,
        Fvr07ProductionVfxKind::SporeDrift,
        Fvr07ProductionVfxKind::SleepGlow,
        Fvr07ProductionVfxKind::DangerHazardParticles,
        Fvr07ProductionVfxKind::EatingResourceEffect,
        Fvr07ProductionVfxKind::BirthDeathEffect,
        Fvr07ProductionVfxKind::WaterDecayAmbient,
        Fvr07ProductionVfxKind::SelectedCreatureNeuralPulse,
    ] {
        assert!(vfx_kinds.contains(&required), "missing VFX {required:?}");
    }

    let mut batch_query = app.world_mut().query::<&Fvr03ProductionVoxelTerrainBatch>();
    let batches = batch_query.iter(app.world()).copied().collect::<Vec<_>>();
    assert!(!batches.is_empty());
    assert!(batches.len() <= materials.len());
    assert_eq!(
        batches.iter().map(|batch| batch.tile_count).sum::<usize>(),
        scene.tile_mesh_count
    );
}

#[test]
fn fvr03_profiles_scale_renderer_residency_lod_and_camera_modes() {
    let minimum = alife_game_app::Fvr03ProductionVoxelRendererSettings::for_profile(
        ProductionFrontendProfileId::MinimumSettings30x30,
    );
    let comfort = alife_game_app::Fvr03ProductionVoxelRendererSettings::for_profile(
        ProductionFrontendProfileId::MinSpecComfort1080p,
    );
    let balanced = alife_game_app::Fvr03ProductionVoxelRendererSettings::for_profile(
        ProductionFrontendProfileId::Balanced1080p,
    );
    let high = alife_game_app::Fvr03ProductionVoxelRendererSettings::for_profile(
        ProductionFrontendProfileId::HighSpecScaleUp,
    );
    let research = alife_game_app::Fvr03ProductionVoxelRendererSettings::for_profile(
        ProductionFrontendProfileId::ResearchScale,
    );

    assert_eq!(minimum.draw_radius_chunks, 2);
    assert_eq!(minimum.target_fps, 30);
    assert_eq!(minimum.max_population, 30);
    assert!(minimum.minimum_floor);
    assert!(minimum.tile_stride <= comfort.tile_stride);
    assert!(comfort.estimated_tile_budget > minimum.estimated_tile_budget);
    assert!(balanced.estimated_tile_budget > comfort.estimated_tile_budget);
    assert_eq!(minimum.production_vfx_budget_state, "conservative");
    assert!(minimum.production_vfx_marker_cap <= comfort.production_vfx_marker_cap);
    assert!(minimum.production_dressing_cap <= comfort.production_dressing_cap);
    assert!(comfort.min_spec_comfort_default);
    assert!(comfort
        .default_camera_modes
        .contains(&Fvr03ProductionVoxelCameraMode::Orbit));
    assert!(comfort
        .default_camera_modes
        .contains(&Fvr03ProductionVoxelCameraMode::OrthographicIsometric));
    assert!(balanced.draw_radius_chunks > comfort.draw_radius_chunks);
    assert!(high.resident_chunk_budget > balanced.resident_chunk_budget);
    assert!(research.research_scale);

    let palette = comfort.material_palette();
    for material in [
        Fvr03ProductionVoxelMaterialKind::Water,
        Fvr03ProductionVoxelMaterialKind::Decay,
        Fvr03ProductionVoxelMaterialKind::Resource,
        Fvr03ProductionVoxelMaterialKind::Hazard,
        Fvr03ProductionVoxelMaterialKind::Stone,
    ] {
        assert!(
            palette.iter().any(|entry| entry.kind == material),
            "palette missing {material:?}"
        );
    }
}

#[test]
fn fvr03_stable_selection_returns_tile_coords_without_renderer_tokens() {
    let launch = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();

    let scene = app
        .world()
        .resource::<Fvr03ProductionVoxelSceneResource>()
        .clone();
    let selected = app
        .world()
        .resource::<Fvr03ProductionVoxelSelectionResource>()
        .selected
        .clone()
        .expect("production voxel scene should select a stable tile at boot");

    assert!(matches!(
        selected.kind,
        StableVoxelRefKind::Tile | StableVoxelRefKind::Creature
    ));
    assert!(selected.tile.is_some());
    assert!(scene.contains_tile(selected.tile.unwrap()));

    let selection_text = scene.selection_label(&selected);
    assert!(selection_text.contains("tile"));
    assert!(selection_text.contains("chunk"));
    assert!(!selection_text.to_ascii_lowercase().contains("entity("));
    assert!(!selection_text.to_ascii_lowercase().contains("bevy"));
    assert!(!selection_text.to_ascii_lowercase().contains("wgpu"));
}

#[test]
fn fvr09_greedy_mesher_records_material_aware_quad_reduction() {
    let launch = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();

    let scene = app
        .world()
        .resource::<Fvr03ProductionVoxelSceneResource>()
        .clone();

    assert_eq!(scene.mesh_stats.mode, Fvr09MesherMode::BinaryGreedyQuads);
    assert!(scene.mesh_stats.chunk_local_occupancy_masks);
    assert!(scene.mesh_stats.six_direction_face_masks);
    assert!(scene.mesh_stats.material_aware_merging);
    assert!(scene.mesh_stats.neighbor_border_seams_checked);
    assert_eq!(
        scene.mesh_stats.material_palette_version,
        "fvr10-visible-surface-variation-v1"
    );
    assert!(scene.mesh_stats.vertex_color_face_variation);
    assert!(scene.mesh_stats.top_side_color_separation);
    assert!(scene.mesh_stats.variation_bucket_count >= 4);
    assert!(scene.mesh_stats.visible_voxels >= scene.tile_mesh_count);
    assert!(scene.mesh_stats.naive_visible_faces > scene.mesh_stats.emitted_quads);
    assert!(scene.mesh_stats.merge_ratio >= 1.20);
    assert!(scene.mesh_stats.dirty_chunks <= scene.mesh_stats.remesh_budget_chunks_per_frame);
    assert!(
        scene.mesh_stats.cached_chunks + scene.mesh_stats.dirty_chunks >= scene.visible_chunk_count
    );
    assert!(scene
        .mesh_stats
        .cache_key
        .contains("fvr10-visible-surface-variation-v1"));
}

#[test]
fn fvr09_material_palette_uses_natural_top_side_texture_slots_not_debug_colors() {
    let settings = alife_game_app::Fvr03ProductionVoxelRendererSettings::for_profile(
        ProductionFrontendProfileId::MinSpecComfort1080p,
    );
    assert_eq!(
        settings.material_palette_version,
        "fvr10-visible-surface-variation-v1"
    );
    assert_eq!(settings.debug_primary_colors, false);

    let palette = settings.material_palette();
    for material in [
        Fvr03ProductionVoxelMaterialKind::SafeGrass,
        Fvr03ProductionVoxelMaterialKind::Soil,
        Fvr03ProductionVoxelMaterialKind::Stone,
        Fvr03ProductionVoxelMaterialKind::Sand,
        Fvr03ProductionVoxelMaterialKind::Water,
        Fvr03ProductionVoxelMaterialKind::Decay,
        Fvr03ProductionVoxelMaterialKind::Resource,
        Fvr03ProductionVoxelMaterialKind::Hazard,
    ] {
        let entry = palette
            .iter()
            .find(|entry| entry.kind == material)
            .unwrap_or_else(|| panic!("missing natural material {material:?}"));
        assert!(!entry.debug_primary_color);
        assert!(!entry.top_texture.is_empty());
        assert!(!entry.side_texture.is_empty());
        assert!(entry.natural_variation_seed.starts_with("fvr10-"));
    }

    let grass = palette
        .iter()
        .find(|entry| entry.kind == Fvr03ProductionVoxelMaterialKind::SafeGrass)
        .unwrap();
    assert_eq!(grass.top_texture, "grass-moss-top");
    assert_eq!(grass.side_texture, "dirt-rooted-side");
}

#[test]
fn fvr09_creatures_are_cute_bipedal_real_state_visuals() {
    let launch = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();

    let scene = app
        .world()
        .resource::<Fvr03ProductionVoxelSceneResource>()
        .clone();
    let creature_scene = app
        .world()
        .resource::<alife_game_app::Fvr04ProductionCreatureSceneResource>()
        .clone();

    assert_eq!(
        creature_scene.visual_profile,
        "fvr10-readable-cute-biped-rig-v1"
    );
    assert_eq!(
        creature_scene.mesh_material_version,
        "fvr10-soft-creature-materials-v1"
    );
    assert_eq!(
        creature_scene.rendered_creature_count,
        scene.creature_render_count
    );
    assert!(creature_scene.mesh_pool_count >= 3);
    assert!(creature_scene.expression_buffer_is_read_only_projection);
    assert!(creature_scene.no_renderer_authority_over_actions_or_cognition);

    let mut query = app.world_mut().query::<&Fvr09CuteBipedCreatureMarker>();
    let markers = query.iter(app.world()).copied().collect::<Vec<_>>();
    assert_eq!(markers.len(), scene.creature_render_count);
    assert!(markers.iter().all(|marker| marker.two_legs));
    assert!(markers.iter().all(|marker| marker.visible_face));
    assert!(markers.iter().all(|marker| marker.eye_markers >= 2));
    assert!(markers.iter().all(|marker| marker.front_back_orientation));
    assert!(markers.iter().all(|marker| marker.real_state_driven));
}

#[test]
fn fvr10_terrain_meshes_have_bound_visible_face_variation_not_texture_labels_only() {
    let launch = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();

    let scene = app
        .world()
        .resource::<Fvr03ProductionVoxelSceneResource>()
        .clone();
    assert_eq!(
        scene.mesh_stats.material_palette_version,
        "fvr10-visible-surface-variation-v1"
    );

    let mut query = app
        .world_mut()
        .query::<(&Fvr03ProductionVoxelTerrainBatch, &Mesh3d)>();
    let terrain_mesh_handles = query
        .iter(app.world())
        .filter(|(batch, _)| {
            !matches!(
                batch.material,
                Fvr03ProductionVoxelMaterialKind::ChunkBoundary
                    | Fvr03ProductionVoxelMaterialKind::Creature
                    | Fvr03ProductionVoxelMaterialKind::Selection
            )
        })
        .map(|(_, mesh)| mesh.0.clone())
        .collect::<Vec<_>>();
    assert!(terrain_mesh_handles.len() >= 6);

    let meshes = app.world().resource::<Assets<Mesh>>();
    let mut unique_colors = BTreeSet::new();
    let mut color_vertex_count = 0_usize;
    for handle in terrain_mesh_handles {
        let mesh = meshes
            .get(&handle)
            .expect("terrain batch mesh should remain resident");
        let Some(VertexAttributeValues::Float32x4(colors)) = mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("FVR10 terrain batch mesh is missing bound vertex color variation");
        };
        color_vertex_count = color_vertex_count.saturating_add(colors.len());
        unique_colors.extend(colors.iter().copied().map(quantized_rgba));
    }

    assert!(color_vertex_count > 0);
    assert!(
        unique_colors.len() >= 24,
        "terrain needs visibly varied face colors, found {} unique colors",
        unique_colors.len()
    );
}

#[test]
fn fvr10_creature_mesh_is_readable_low_poly_rig_not_cuboid_stack() {
    let launch = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();

    let creature_scene = app
        .world()
        .resource::<alife_game_app::Fvr04ProductionCreatureSceneResource>()
        .clone();
    assert_eq!(
        creature_scene.visual_profile,
        "fvr10-readable-cute-biped-rig-v1"
    );
    assert_eq!(
        creature_scene.mesh_material_version,
        "fvr10-soft-creature-materials-v1"
    );
    assert!(creature_scene.mesh_pool_count >= 5);

    let mut query = app
        .world_mut()
        .query::<(&Fvr09CuteBipedCreatureMarker, &Mesh3d)>();
    let mesh_handle = query
        .iter(app.world())
        .next()
        .map(|(_, mesh)| mesh.0.clone())
        .expect("at least one visible creature rig should spawn");
    let meshes = app.world().resource::<Assets<Mesh>>();
    let mesh = meshes
        .get(&mesh_handle)
        .expect("creature rig mesh should remain resident");
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("creature rig mesh is missing positions");
    };
    assert!(
        positions.len() >= 320,
        "creature rig should be rounded/generated, not a cuboid stack with {} vertices",
        positions.len()
    );
}

#[test]
fn fvr10_default_product_view_starts_clean_without_debug_panels_or_overlays() {
    let launch = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();

    let ux = app.world().resource::<Fvr05ProductionUxStateResource>();
    assert!(
        !ux.settings.show_menu,
        "product default should not put the menu panel over screenshots"
    );
    assert!(
        !ux.settings.show_settings,
        "product default should not put settings text over screenshots"
    );
    assert!(
        !ux.settings.show_overlays,
        "product default should not draw debug overlays over screenshots"
    );
}

#[test]
fn fvr10_product_camera_and_faces_are_composed_for_readable_creatures() {
    let launch = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();

    let mut camera_query = app
        .world_mut()
        .query::<(&Fvr03ProductionVoxelCamera, &Projection, &Transform)>();
    let (camera, projection, transform) = camera_query
        .iter(app.world())
        .next()
        .expect("production voxel camera should spawn");
    assert_eq!(
        camera.mode,
        Fvr03ProductionVoxelCameraMode::OrthographicIsometric
    );
    let Projection::Orthographic(orthographic) = projection else {
        panic!("production voxel camera should use orthographic projection");
    };
    assert!(
        orthographic.area.height() <= 24.0,
        "FVR10 product shot should be close enough for creature faces, got vertical area {:.2}",
        orthographic.area.height()
    );
    assert!(
        transform.translation.y <= 19.0,
        "FVR10 product shot should lower the camera for character readability, got y {:.2}",
        transform.translation.y
    );

    let mut face_query = app.world_mut().query::<(
        &Fvr09CreatureFaceFeatureMarker,
        &Fvr04ProductionCreatureVisualMarker,
    )>();
    let face_offsets = face_query
        .iter(app.world())
        .map(|(_, marker)| marker.local_offset)
        .collect::<Vec<_>>();
    assert!(!face_offsets.is_empty());
    assert!(
        face_offsets.iter().all(|offset| offset.z >= 0.36),
        "creature face markers must sit on the camera-facing side for the default screenshot"
    );
}

#[test]
fn fvr10_scene_dressing_uses_composite_vertical_props_not_unit_debug_cubes() {
    let launch = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();

    let mut creature_query = app
        .world_mut()
        .query::<(&Fvr09CuteBipedCreatureMarker, &Transform)>();
    let creature_positions = creature_query
        .iter(app.world())
        .map(|(_, transform)| transform.translation)
        .collect::<Vec<_>>();
    assert!(!creature_positions.is_empty());
    let creature_center = creature_positions
        .iter()
        .fold(bevy::prelude::Vec3::ZERO, |acc, position| acc + *position)
        / creature_positions.len() as f32;

    let mut query = app.world_mut().query::<(
        &Fvr07ProductionVisualDressing,
        &Mesh3d,
        &MeshMaterial3d<StandardMaterial>,
        &Transform,
    )>();
    let dressing_entries = query
        .iter(app.world())
        .map(|(dressing, mesh, material, transform)| {
            (
                dressing.kind,
                mesh.0.clone(),
                material.0.clone(),
                transform.scale.y,
                transform.translation,
            )
        })
        .collect::<Vec<_>>();

    let meshes = app.world().resource::<Assets<Mesh>>();
    let materials = app.world().resource::<Assets<StandardMaterial>>();
    let mut composite_prop_count = 0_usize;
    let mut vertical_prop_count = 0_usize;
    let mut hero_cluster_prop_count = 0_usize;
    let mut readable_hero_material_count = 0_usize;
    for (kind, mesh_handle, material_handle, scale_y, translation) in dressing_entries {
        let mesh = meshes
            .get(&mesh_handle)
            .expect("dressing mesh should remain resident");
        let material = materials
            .get(&material_handle)
            .expect("dressing material should remain resident");
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("dressing mesh is missing positions");
        };
        if positions.len() > 24 {
            composite_prop_count = composite_prop_count.saturating_add(1);
        }
        if scale_y >= 0.75
            && matches!(
                kind,
                Fvr07ProductionDressingKind::LeafPatch
                    | Fvr07ProductionDressingKind::MushroomCluster
                    | Fvr07ProductionDressingKind::FoodResource
            )
        {
            vertical_prop_count = vertical_prop_count.saturating_add(1);
        }
        let distance_to_creatures = bevy::prelude::Vec2::new(
            translation.x - creature_center.x,
            translation.z - creature_center.z,
        )
        .length();
        if scale_y >= 1.10
            && distance_to_creatures <= 8.0
            && matches!(
                kind,
                Fvr07ProductionDressingKind::LeafPatch
                    | Fvr07ProductionDressingKind::MushroomCluster
                    | Fvr07ProductionDressingKind::FoodResource
            )
        {
            hero_cluster_prop_count = hero_cluster_prop_count.saturating_add(1);
            if material.unlit {
                readable_hero_material_count = readable_hero_material_count.saturating_add(1);
            }
        }
    }

    assert!(
        composite_prop_count >= 24,
        "FVR10 scene dressing should use composite art meshes, found {composite_prop_count}"
    );
    assert!(
        vertical_prop_count >= 12,
        "FVR10 product screenshot needs visible upright flora/food props, found {vertical_prop_count}"
    );
    assert!(
        hero_cluster_prop_count >= 12,
        "FVR10 product screenshot needs hero-scale props near creatures, found {hero_cluster_prop_count}"
    );
    assert!(
        readable_hero_material_count >= 12,
        "FVR10 hero props must use readable display materials, found {readable_hero_material_count}"
    );
}
