#![cfg(feature = "bevy-app")]

use std::collections::BTreeSet;

use alife_game_app::{
    default_environment_manifest_path, Fvr03ProductionVoxelCameraMode, Fvr03ProductionVoxelChunk,
    Fvr03ProductionVoxelMaterialKind, Fvr03ProductionVoxelSceneResource,
    Fvr03ProductionVoxelSelectionResource, Fvr03ProductionVoxelTerrainBatch,
    Fvr03ProductionVoxelTerrainTile, Fvr07ProductionDressingKind, Fvr07ProductionGpuVfxMarker,
    Fvr07ProductionVfxKind, Fvr07ProductionVisualDressing, Fvr09CuteBipedCreatureMarker,
    Fvr09MesherMode, ProductionFrontendProfileId, ProductionVoxelLaunchConfig,
    FVR03_PRODUCTION_VOXEL_RENDERER_SCHEMA,
};
use alife_world::{StableVoxelRefKind, FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA};

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
    assert!(scene.production_dressing_count <= 28);
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
        "fvr09-natural-materials-v1"
    );
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
        .contains("fvr09-natural-materials-v1"));
}

#[test]
fn fvr09_material_palette_uses_natural_top_side_texture_slots_not_debug_colors() {
    let settings = alife_game_app::Fvr03ProductionVoxelRendererSettings::for_profile(
        ProductionFrontendProfileId::MinSpecComfort1080p,
    );
    assert_eq!(
        settings.material_palette_version,
        "fvr09-natural-materials-v1"
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
        assert!(entry.natural_variation_seed.starts_with("fvr09-"));
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

    assert_eq!(creature_scene.visual_profile, "fvr09-cute-biped-v1");
    assert_eq!(
        creature_scene.mesh_material_version,
        "fvr09-soft-biped-materials-v1"
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
