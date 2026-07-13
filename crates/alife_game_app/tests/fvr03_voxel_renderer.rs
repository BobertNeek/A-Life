#![cfg(feature = "bevy-app")]

use std::collections::BTreeSet;

use alife_game_app::{
    default_environment_manifest_path, CreaturePartSlot, Fvr03ProductionVoxelCamera,
    Fvr03ProductionVoxelCameraMode, Fvr03ProductionVoxelChunk, Fvr03ProductionVoxelMaterialKind,
    Fvr03ProductionVoxelSceneResource, Fvr03ProductionVoxelSelectionResource,
    Fvr03ProductionVoxelTerrainBatch, Fvr03ProductionVoxelTerrainTile,
    Fvr04ProductionCreatureVisualMarker, Fvr05ProductionUxStateResource,
    Fvr07ProductionDressingKind, Fvr07ProductionGpuVfxMarker, Fvr07ProductionVfxKind,
    Fvr07ProductionVisualDressing, Fvr09CreatureFaceFeatureMarker, Fvr09CuteBipedCreatureMarker,
    Fvr09MesherMode, Fvr10CreatureSpeciesMarker, Fvr10CreatureSurfaceDetailMarker,
    Fvr11ProductionContactShadow, Fvr11ProductionTerrainLayer,
    Fvr11ProductionTerrainLightingMarker, Fvr11ProductionTerrainMaterialContract,
    Fvr11ProductionTerrainSceneResource, Fvr11TerrainSurfaceRole, ProductionCreatureAssemblyRoot,
    ProductionCreatureJoinCoverMarker, ProductionCreaturePartMarker, ProductionFrontendProfileId,
    ProductionVoxelLaunchConfig, FVR03_PRODUCTION_VOXEL_RENDERER_SCHEMA,
    FVR11_PRODUCTION_TERRAIN_VISUAL_VERSION,
};
use alife_world::{
    CreatureAppearanceGenome, StableVoxelRefKind, CREATURE_APPEARANCE_SPECIES_COUNT,
    FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA,
};
use bevy::{
    mesh::VertexAttributeValues,
    prelude::{
        AlphaMode, AmbientLight, Assets, DirectionalLight, Mesh, Mesh3d, MeshMaterial3d,
        Projection, StandardMaterial, Transform,
    },
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
fn modular_creature_renderer_spawns_shared_heritable_part_hierarchies() {
    let launch = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();

    let mut root_query = app.world_mut().query::<&ProductionCreatureAssemblyRoot>();
    let roots = root_query.iter(app.world()).copied().collect::<Vec<_>>();
    assert_eq!(roots.len(), 30);
    assert!(roots.iter().all(|root| root.display_only));

    let mut part_query = app
        .world_mut()
        .query::<(&ProductionCreaturePartMarker, &Mesh3d)>();
    let parts = part_query
        .iter(app.world())
        .map(|(marker, mesh)| (*marker, mesh.0.id()))
        .collect::<Vec<_>>();
    let mut slots_by_root = std::collections::BTreeMap::new();
    let mut families = BTreeSet::new();
    let mut mesh_handles = BTreeSet::new();
    for (marker, mesh_id) in &parts {
        slots_by_root
            .entry(marker.stable_id.raw())
            .or_insert_with(BTreeSet::new)
            .insert(marker.slot);
        families.insert(marker.family);
        mesh_handles.insert(*mesh_id);
    }
    assert_eq!(slots_by_root.len(), roots.len());
    assert!(slots_by_root.values().all(|slots| {
        CreaturePartSlot::REQUIRED_RUNTIME_SLOTS
            .iter()
            .all(|slot| slots.contains(slot))
    }));
    assert!(families.len() >= 8);
    assert!(mesh_handles.len() < parts.len() / 3);

    let mut cover_query = app
        .world_mut()
        .query::<&ProductionCreatureJoinCoverMarker>();
    let covers = cover_query.iter(app.world()).copied().collect::<Vec<_>>();
    assert!(covers.len() >= roots.len() * 5);
    assert!(covers.iter().all(|cover| cover.display_only));

    let scene = app
        .world()
        .resource::<alife_game_app::Fvr04ProductionCreatureSceneResource>();
    assert_eq!(scene.visual_profile, "modular-heritable-part-assembly-v1");
    assert_eq!(scene.creature_root_count, roots.len());
    assert_eq!(scene.creature_part_entity_count, parts.len());
    assert_eq!(scene.creature_join_cover_count, covers.len());
    assert!(scene.creature_mixed_assembly_count <= scene.creature_root_count);
    assert!(scene.production_visuals_display_only);
}

#[test]
fn fvr11_terrain_contract_is_display_only() {
    let launch = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();

    let scene = app
        .world()
        .resource::<Fvr11ProductionTerrainSceneResource>();
    assert_eq!(
        scene.visual_version,
        FVR11_PRODUCTION_TERRAIN_VISUAL_VERSION
    );
    assert!(scene.sample_count > 0);
    assert!(scene.display_only);
    assert!(scene.no_renderer_authority_over_world_actions_or_cognition);
}

#[test]
fn fvr11_terrain_contract_is_display_only_and_layered() {
    let launch = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();

    let scene = app
        .world()
        .resource::<Fvr11ProductionTerrainSceneResource>()
        .clone();
    assert_eq!(scene.confetti_detail_quad_count, 0);
    assert!(scene.top_layer_count >= 7);
    assert!(scene.cliff_layer_count >= 3);
    assert!(scene.transition_edge_count > 0);

    let mut query = app.world_mut().query::<&Fvr11ProductionTerrainLayer>();
    let layers = query.iter(app.world()).copied().collect::<Vec<_>>();
    assert!(layers.iter().all(|layer| layer.display_only));
    assert!(layers
        .iter()
        .all(|layer| layer.no_renderer_authority_over_world_actions_or_cognition));
    assert!(layers.iter().all(|layer| layer.source_tile_count > 0));
    let roles = layers
        .iter()
        .map(|layer| layer.role)
        .collect::<BTreeSet<_>>();
    assert!(roles.contains(&Fvr11TerrainSurfaceRole::Top));
    assert!(roles.contains(&Fvr11TerrainSurfaceRole::Cliff));
    assert!(roles.contains(&Fvr11TerrainSurfaceRole::Transition));
    assert!(roles.contains(&Fvr11TerrainSurfaceRole::Water));
}

#[test]
fn fvr11_terrain_material_contract_binds_lit_layers_and_water() {
    let launch = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();

    let contract = app
        .world()
        .resource::<Fvr11ProductionTerrainMaterialContract>();
    assert_eq!(contract.material_count, 8);
    assert_eq!(contract.atlas_dimensions, [272, 272]);
    assert_eq!(
        contract.base_color_path,
        "production_voxel_v1/terrain/terrain_albedo_atlas.png"
    );
    assert_eq!(
        contract.normal_path,
        "production_voxel_v1/terrain/terrain_normal_atlas.png"
    );
    assert_eq!(
        contract.orm_path,
        "production_voxel_v1/terrain/terrain_orm_atlas.png"
    );
    assert!(!contract.real_assets_requested);
    assert!(contract.display_only);

    let mut query = app.world_mut().query::<(
        &Fvr11ProductionTerrainLayer,
        &MeshMaterial3d<StandardMaterial>,
    )>();
    let handles = query
        .iter(app.world())
        .map(|(layer, material)| (layer.role, material.0.clone()))
        .collect::<Vec<_>>();
    let materials = app.world().resource::<Assets<StandardMaterial>>();
    let mut saw_water = false;
    for (role, handle) in handles {
        let material = materials
            .get(&handle)
            .expect("terrain material remains resident");
        assert!(!material.unlit);
        if role == Fvr11TerrainSurfaceRole::Water {
            saw_water = true;
            assert_eq!(material.alpha_mode, AlphaMode::Blend);
            assert!(material.clearcoat > 0.0);
        }
    }
    assert!(saw_water);
}

#[test]
fn fvr11_profile_lighting_preserves_minimum_floor_and_comfort_depth() {
    let lighting = |profile_id| {
        let launch = production_launch(profile_id);
        let (mut app, _summary) =
            alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
        app.update();
        let (marker, ambient_brightness, vertical_area) = {
            let mut marker_query = app.world_mut().query::<(
                &Fvr11ProductionTerrainLightingMarker,
                &AmbientLight,
                &Projection,
            )>();
            let (marker, ambient, projection) = marker_query
                .iter(app.world())
                .next()
                .expect("terrain lighting marker");
            let Projection::Orthographic(orthographic) = projection else {
                panic!("production terrain camera should stay orthographic");
            };
            (*marker, ambient.brightness, orthographic.area.height())
        };
        let mut light_query = app.world_mut().query::<&DirectionalLight>();
        let sun_illuminance = light_query
            .iter(app.world())
            .next()
            .expect("production terrain sun")
            .illuminance;
        let mut shadow_query = app.world_mut().query::<&Fvr11ProductionContactShadow>();
        let contact_shadow_count = shadow_query.iter(app.world()).count();
        (
            marker,
            contact_shadow_count,
            ambient_brightness,
            vertical_area,
            sun_illuminance,
        )
    };

    let (
        minimum,
        minimum_contact_shadows,
        minimum_ambient_brightness,
        minimum_vertical_area,
        minimum_sun_illuminance,
    ) = lighting(ProductionFrontendProfileId::MinimumSettings30x30);
    let (
        comfort,
        comfort_contact_shadows,
        comfort_ambient_brightness,
        comfort_vertical_area,
        comfort_sun_illuminance,
    ) = lighting(ProductionFrontendProfileId::MinSpecComfort1080p);

    assert_eq!(minimum.tonemapping, "tony-mc-mapface");
    assert!(!minimum.directional_shadows);
    assert_eq!(minimum.shadow_cascades, 0);
    assert!(minimum.contact_grounding);
    assert!(minimum_contact_shadows >= 30);
    assert!(minimum.distance_fog);
    assert!(minimum_ambient_brightness >= 260.0);
    assert!(minimum_vertical_area <= 19.0);
    assert!(minimum_sun_illuminance <= 6_000.0);

    assert_eq!(comfort.tonemapping, "tony-mc-mapface");
    assert!(comfort.directional_shadows);
    assert_eq!(comfort.shadow_cascades, 2);
    assert!(comfort.distance_fog);
    assert!(comfort.cool_ambient_fill);
    assert!(comfort.contact_grounding);
    assert_eq!(comfort_contact_shadows, 0);
    assert!(comfort.display_only);
    assert!(comfort.no_renderer_authority_over_world_actions_or_cognition);
    assert!(comfort_ambient_brightness >= 360.0);
    assert!(comfort_vertical_area <= 17.5);
    assert!(comfort_sun_illuminance <= 6_000.0);
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
    assert!(scene.production_dressing_count <= 64);
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
    assert!(
        vfx.iter()
            .filter(|entry| {
                entry.stable_id.is_some()
                    && matches!(
                        entry.kind,
                        Fvr07ProductionVfxKind::SleepGlow
                            | Fvr07ProductionVfxKind::BirthDeathEffect
                            | Fvr07ProductionVfxKind::SelectedCreatureNeuralPulse
                    )
            })
            .all(|entry| entry.base_scale.x <= 0.32 && entry.base_scale.z <= 0.32),
        "creature-attached VFX markers must stay small enough to avoid covering body silhouettes"
    );

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
        "modular-heritable-part-assembly-v1"
    );
    assert_eq!(
        creature_scene.mesh_material_version,
        "modular-textured-part-material-v1"
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
        "modular-heritable-part-assembly-v1"
    );
    assert_eq!(
        creature_scene.mesh_material_version,
        "modular-textured-part-material-v1"
    );
    assert!(creature_scene.mesh_pool_count >= 5);

    let mut query = app
        .world_mut()
        .query::<(&ProductionCreaturePartMarker, &Mesh3d)>();
    let mesh_handle = query
        .iter(app.world())
        .find(|(marker, _)| marker.slot == CreaturePartSlot::Head)
        .map(|(_, mesh)| mesh.0.clone())
        .expect("at least one visible creature head part should spawn");
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
        positions.len() >= 24,
        "sliced source mesh must retain useful geometry"
    );
    let (mut min_x, mut max_x) = (f32::MAX, f32::MIN);
    let (mut min_y, mut max_y) = (f32::MAX, f32::MIN);
    let (mut min_z, mut max_z) = (f32::MAX, f32::MIN);
    for position in positions {
        min_x = min_x.min(position[0]);
        max_x = max_x.max(position[0]);
        min_y = min_y.min(position[1]);
        max_y = max_y.max(position[1]);
        min_z = min_z.min(position[2]);
        max_z = max_z.max(position[2]);
    }
    assert!(
        max_x > min_x && max_z > min_z && max_y > min_y,
        "creature part mesh must have three-dimensional bounds, spans=({:.2},{:.2},{:.2})",
        max_x - min_x,
        max_y - min_y,
        max_z - min_z
    );
}

#[test]
fn fvr10_creatures_use_all_selected_bipedal_caveman_species_not_color_swaps() {
    let launch = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();

    let creature_scene = app
        .world()
        .resource::<alife_game_app::Fvr04ProductionCreatureSceneResource>()
        .clone();
    assert_eq!(
        creature_scene.species_archetype_count,
        CREATURE_APPEARANCE_SPECIES_COUNT as usize
    );
    assert_eq!(
        creature_scene.mesh_material_version,
        "modular-textured-part-material-v1"
    );
    assert!(
        creature_scene.mesh_pool_count >= CREATURE_APPEARANCE_SPECIES_COUNT as usize,
        "selected sheet requires distinct species body-plan meshes, not one recolored rig"
    );
    assert!(
        creature_scene.material_bucket_count >= CREATURE_APPEARANCE_SPECIES_COUNT as usize,
        "selected sheet requires species-specific inherited body materials, not shared expression color buckets"
    );

    let mut query = app.world_mut().query::<&Fvr10CreatureSpeciesMarker>();
    let markers = query.iter(app.world()).copied().collect::<Vec<_>>();
    assert_eq!(markers.len(), creature_scene.rendered_creature_count);
    assert!(markers.iter().all(|marker| marker.bipedal));
    assert!(markers.iter().all(|marker| marker.caveman_furry_design));
    assert!(markers.iter().all(|marker| marker.heritable_appearance));
    assert!(markers
        .iter()
        .all(|marker| !marker.species_label.is_empty() && marker.species_label != "color-swap"));

    let species = markers
        .iter()
        .map(|marker| marker.species_archetype)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        species.len(),
        CREATURE_APPEARANCE_SPECIES_COUNT as usize,
        "production population should show every picked species archetype"
    );

    let body_plans = markers
        .iter()
        .map(|marker| marker.body_plan_signature)
        .collect::<BTreeSet<_>>();
    assert!(
        body_plans.len() >= 12,
        "species need different silhouettes/body plans, found only {}",
        body_plans.len()
    );
}

#[test]
fn fvr10_creatures_have_high_contrast_heritable_surface_markings() {
    let launch = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();

    let mut detail_query = app.world_mut().query::<&Fvr10CreatureSurfaceDetailMarker>();
    let details = detail_query.iter(app.world()).copied().collect::<Vec<_>>();
    let unique_species = details
        .iter()
        .map(|marker| marker.species_archetype)
        .collect::<BTreeSet<_>>();
    let unique_roles = details
        .iter()
        .map(|marker| marker.detail_role)
        .collect::<BTreeSet<_>>();

    assert_eq!(
        unique_species.len(),
        CREATURE_APPEARANCE_SPECIES_COUNT as usize
    );
    assert!(
        unique_roles.len() >= 10,
        "surface detail should include species-specific markings/accessories, found {unique_roles:?}"
    );
    assert!(details.iter().all(|marker| marker.display_only
        && marker.no_renderer_authority_over_actions_or_cognition
        && marker.high_contrast_marking
        && marker.heritable));
}

#[test]
fn fvr10_creature_appearance_genes_cover_sixteen_species_and_mutate_offspring() {
    let founders = (0..CREATURE_APPEARANCE_SPECIES_COUNT)
        .map(|slot| CreatureAppearanceGenome::founder_for_species(slot, 10_000 + u64::from(slot)))
        .collect::<Vec<_>>();
    assert_eq!(
        founders
            .iter()
            .map(|appearance| appearance.species_archetype)
            .collect::<BTreeSet<_>>()
            .len(),
        CREATURE_APPEARANCE_SPECIES_COUNT as usize
    );
    assert!(founders
        .iter()
        .all(|appearance| appearance.validate().is_ok()));
    assert!(founders
        .iter()
        .all(|appearance| appearance.bipedal_caveman_furry));

    let child = CreatureAppearanceGenome::offspring_from_parents(
        founders[2],
        founders[9],
        0xA11F_CAFE_2026,
    );
    child.validate().unwrap();
    assert!(child.inherited_from(founders[2], founders[9]));
    assert!(child.mutation_count > founders[2].mutation_count.max(founders[9].mutation_count));
    assert_ne!(
        child.signature_line(),
        founders[2].signature_line(),
        "offspring appearance should permit mutation, not clone parent A exactly"
    );
    assert_ne!(
        child.signature_line(),
        founders[9].signature_line(),
        "offspring appearance should permit mutation, not clone parent B exactly"
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

    let mut face_query = app
        .world_mut()
        .query::<(&Fvr09CreatureFaceFeatureMarker, &Transform)>();
    let face_offsets = face_query
        .iter(app.world())
        .map(|(_, transform)| transform.translation)
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
    let mut creature_tile_query = app
        .world_mut()
        .query::<&Fvr04ProductionCreatureVisualMarker>();
    let occupied_creature_tiles = creature_tile_query
        .iter(app.world())
        .map(|marker| marker.tile)
        .collect::<BTreeSet<_>>();
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
                dressing.tile,
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
    let mut lit_material_count = 0_usize;
    let mut new_biome_kinds = BTreeSet::new();
    for (tile, kind, mesh_handle, material_handle, scale_y, translation) in dressing_entries {
        assert!(
            !occupied_creature_tiles.contains(&tile),
            "dressing {kind:?} overlaps creature tile {tile:?}"
        );
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
                    | Fvr07ProductionDressingKind::FlowerPatch
                    | Fvr07ProductionDressingKind::ReedCluster
                    | Fvr07ProductionDressingKind::HazardFungus
                    | Fvr07ProductionDressingKind::AlienFern
                    | Fvr07ProductionDressingKind::CrimsonSpire
                    | Fvr07ProductionDressingKind::GlowBulbCluster
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
                    | Fvr07ProductionDressingKind::FlowerPatch
                    | Fvr07ProductionDressingKind::ReedCluster
                    | Fvr07ProductionDressingKind::HazardFungus
                    | Fvr07ProductionDressingKind::AlienFern
                    | Fvr07ProductionDressingKind::CrimsonSpire
                    | Fvr07ProductionDressingKind::GlowBulbCluster
            )
        {
            hero_cluster_prop_count = hero_cluster_prop_count.saturating_add(1);
        }
        if !material.unlit {
            lit_material_count = lit_material_count.saturating_add(1);
        }
        if matches!(
            kind,
            Fvr07ProductionDressingKind::FlowerPatch
                | Fvr07ProductionDressingKind::ReedCluster
                | Fvr07ProductionDressingKind::LichenRock
                | Fvr07ProductionDressingKind::HazardFungus
                | Fvr07ProductionDressingKind::DeadLeafPatch
                | Fvr07ProductionDressingKind::AlienFern
                | Fvr07ProductionDressingKind::CrimsonSpire
                | Fvr07ProductionDressingKind::GlowBulbCluster
        ) {
            new_biome_kinds.insert(kind);
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
        lit_material_count == composite_prop_count,
        "FVR11 composite props must use lit materials: lit={lit_material_count} composite={composite_prop_count}"
    );
    assert_eq!(new_biome_kinds.len(), 8);
}
