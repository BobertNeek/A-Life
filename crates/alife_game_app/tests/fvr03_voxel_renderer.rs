#![cfg(feature = "bevy-app")]

use std::{
    collections::BTreeSet,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use alife_core::{ActionKind, BrainTickStatus, OrganismId, Tick, Vec3f, WorldEntityId};
use alife_game_app::bevy_shell::{LiveBrainPresentationFrame, LiveBrainPresentationFrameResource};
use alife_game_app::{
    default_environment_manifest_path, run_production_voxel_frontend_dry_run, CreaturePartSlot,
    Fvr03ProductionVoxelCamera, Fvr03ProductionVoxelCameraMode, Fvr03ProductionVoxelChunk,
    Fvr03ProductionVoxelMaterialKind, Fvr03ProductionVoxelSceneResource,
    Fvr03ProductionVoxelCreatureMarker,
    Fvr03ProductionVoxelSelectionMarker, Fvr03ProductionVoxelSelectionResource,
    Fvr03ProductionVoxelTerrainBatch, Fvr03ProductionVoxelTerrainTile,
    Fvr04ProductionCreatureFollowResource, Fvr04ProductionCreatureInspectorPanel,
    Fvr04ProductionCreatureVisualMarker, Fvr04ProductionCreatureWorldLabel,
    Fvr05ProductionInspectorTab, Fvr05ProductionRightInspectorPanel,
    Fvr05ProductionUxStateResource, Fvr07ProductionDressingKind, Fvr07ProductionGpuVfxMarker,
    Fvr07ProductionVfxKind, Fvr07ProductionVisualDressing, Fvr09CreatureFaceFeatureMarker,
    Fvr09CuteBipedCreatureMarker, Fvr09MesherMode, Fvr10CreatureSpeciesMarker,
    Fvr10CreatureSurfaceDetailMarker, Fvr11ProductionContactShadow, Fvr11ProductionTerrainLayer,
    Fvr11ProductionTerrainLightingMarker, Fvr11ProductionTerrainMaterialContract,
    Fvr11ProductionTerrainSceneResource, Fvr11TerrainSurfaceRole, LiveBrainCausalStage,
    LiveBrainTickSummary, ProductionCreatureAssemblyRoot, ProductionCreatureJoinCoverMarker,
    ProductionCreaturePartMarker, ProductionFrontendProfileId, ProductionVoxelLaunchConfig,
    V0PlayerControlStrip, V0PlayerCreaturePanel, V0PlayerStatusChip,
    FVR03_PRODUCTION_VOXEL_RENDERER_SCHEMA, FVR11_PRODUCTION_TERRAIN_VISUAL_VERSION,
};
use alife_world::{
    persistence::PortableSaveFile, CreatureAppearanceGenome, HeadlessActionIds,
    StableVoxelObjectRef, StableVoxelRefKind, WorldObjectKind, CREATURE_APPEARANCE_SPECIES_COUNT,
    FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA,
};
use bevy::{
    mesh::VertexAttributeValues,
    prelude::{
        AlphaMode, AmbientLight, Assets, ButtonInput, ChildOf, DirectionalLight, Entity, KeyCode,
        Mesh, Mesh3d, MeshMaterial3d, Projection, StandardMaterial, Text, Transform, Vec3,
        Visibility,
    },
};

fn successful_gpu_move_summary(
    organism_id: alife_core::OrganismId,
    target_entity: alife_core::WorldEntityId,
) -> LiveBrainTickSummary {
    LiveBrainTickSummary {
        schema: alife_game_app::G03_LIVE_BRAIN_LOOP_SCHEMA,
        schema_version: alife_game_app::G03_LIVE_BRAIN_LOOP_SCHEMA_VERSION,
        organism_id,
        tick_before: Tick::new(7),
        tick_after: Tick::new(8),
        world_tick_before: Tick::new(7),
        world_tick_after: Tick::new(8),
        status: BrainTickStatus::Normal,
        selected_action_kind: Some(ActionKind::Move),
        selected_action_id: Some(HeadlessActionIds::APPROACH),
        target_entity: Some(target_entity),
        patch_sealed: true,
        patch_sequence_id: Some(7),
        patch_success: Some(true),
        physical_contact: None,
        action_failure: None,
        sealed_patch_count: 1,
        packed_record_count: 1,
        memory_updates: 1,
        topology_updates: 0,
        learning_updates: 1,
        invalid_or_rejected_action_count: 0,
        last_diagnostic: None,
        causal_stages: vec![
            LiveBrainCausalStage::GpuBrainTick,
            LiveBrainCausalStage::ExecuteAction,
            LiveBrainCausalStage::MeasureOutcome,
            LiveBrainCausalStage::SealPatch,
            LiveBrainCausalStage::ApplyLearning,
        ],
    }
}

fn production_voxel_center(world_position: Vec3f) -> (f32, f32) {
    (
        world_position.x.round() as i32 as f32 + 0.5,
        world_position.z.round() as i32 as f32 + 0.5,
    )
}

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
fn dry_run_rebuilds_an_incompatible_derived_runtime_save_before_gpu_launch() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "alife-fvr03-incompatible-runtime-save-{}-{nonce}",
        std::process::id()
    ));
    let mut launch = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
    launch.manifest_path = root.join("crates/alife_game_app/environment_manifest.json");

    let summary = run_production_voxel_frontend_dry_run(&launch).unwrap();
    let runtime_save_path = PathBuf::from(&summary.ui_settings.runtime_save_path);
    fs::create_dir_all(runtime_save_path.parent().unwrap()).unwrap();
    fs::write(&runtime_save_path, br#"{"schema":"stale-derived-runtime"}"#).unwrap();

    let result = alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch);
    assert!(
        result.is_ok(),
        "dry-run launch must rebuild an incompatible derived runtime save: {:?}",
        result.as_ref().err()
    );
    drop(result);

    let rebuilt = PortableSaveFile::from_json_file(&runtime_save_path).unwrap();
    rebuilt
        .validate_with_asset_root(&summary.asset_root)
        .unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn modular_creature_renderer_spawns_shared_heritable_part_hierarchies() {
    let launch = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();

    let mut root_query = app
        .world_mut()
        .query::<(&ProductionCreatureAssemblyRoot, &bevy::prelude::Visibility)>();
    let roots = root_query
        .iter(app.world())
        .map(|(root, visibility)| (*root, *visibility))
        .collect::<Vec<_>>();
    assert_eq!(roots.len(), 30);
    assert!(roots.iter().all(|(root, visibility)| root.display_only
        && *visibility == bevy::prelude::Visibility::Inherited));

    let mut part_query = app.world_mut().query::<(
        &ProductionCreaturePartMarker,
        &Mesh3d,
        &MeshMaterial3d<StandardMaterial>,
        &Transform,
    )>();
    let parts = part_query
        .iter(app.world())
        .map(|(marker, mesh, material, transform)| {
            (
                marker.clone(),
                mesh.0.id(),
                material.0.id(),
                transform.translation,
            )
        })
        .collect::<Vec<_>>();
    let mut slots_by_root = std::collections::BTreeMap::new();
    let mut meshes_by_root = std::collections::BTreeMap::new();
    let mut families = BTreeSet::new();
    let mut mesh_handles = BTreeSet::new();
    let mut material_handles = BTreeSet::new();
    for (marker, mesh_id, material_id, translation) in &parts {
        slots_by_root
            .entry(marker.stable_id.raw())
            .or_insert_with(BTreeSet::new)
            .insert(marker.slot);
        meshes_by_root
            .entry(marker.stable_id.raw())
            .or_insert_with(std::collections::BTreeMap::new)
            .insert(marker.slot, *mesh_id);
        assert!(translation.is_finite());
        families.insert(marker.family);
        mesh_handles.insert(*mesh_id);
        material_handles.insert(*material_id);
    }
    assert_eq!(slots_by_root.len(), roots.len());
    assert!(slots_by_root.values().all(|slots| {
        CreaturePartSlot::REQUIRED_RUNTIME_SLOTS
            .iter()
            .all(|slot| slots.contains(slot))
    }));
    assert!(meshes_by_root.values().all(|meshes| {
        meshes[&CreaturePartSlot::LeftArm] != meshes[&CreaturePartSlot::RightArm]
            && meshes[&CreaturePartSlot::LeftLeg] != meshes[&CreaturePartSlot::RightLeg]
    }));
    assert!(families.len() >= 8);
    assert!(mesh_handles.len() < parts.len() / 3);
    assert!(material_handles.len() < parts.len() / 3);

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
    assert_eq!(
        scene.uses_bevy_voxel_world_backend,
        cfg!(feature = "voxel-backend")
    );
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
fn fvr04_live_world_projection_moves_matching_creature_and_creates_newborn_by_stable_id() {
    let launch = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
    let (mut app, launch_summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.world_mut()
        .resource_mut::<Fvr05ProductionUxStateResource>()
        .settings
        .paused = true;
    app.update();

    let (stable_id, organism_id, initial_translation) = {
        let mut roots = app.world_mut().query::<(
            &ProductionCreatureAssemblyRoot,
            &Fvr04ProductionCreatureVisualMarker,
            &Transform,
        )>();
        roots
            .iter(app.world())
            .map(|(root, visual, transform)| {
                (root.stable_id, visual.organism_id, transform.translation)
            })
            .next()
            .expect("production voxel scene must spawn a creature root")
    };
    let save = PortableSaveFile::from_json_file(&launch_summary.save_path).unwrap();
    let initial_object = save
        .world
        .objects
        .iter()
        .find(|object| object.id == stable_id)
        .cloned()
        .expect("production creature root must have a matching saved world object");
    assert_eq!(initial_object.id, stable_id);
    assert_eq!(initial_object.organism_id, Some(organism_id));
    let target_entity = save
        .world
        .objects
        .iter()
        .find(|object| object.id != stable_id && object.kind == WorldObjectKind::Food)
        .map(|object| object.id)
        .expect("launch save must contain a non-self food target");
    let mut authoritative_post_action_object = initial_object.clone();
    authoritative_post_action_object.position = Vec3f::new(
        initial_object.position.x + 2.0,
        initial_object.position.y,
        initial_object.position.z + 1.0,
    );
    let summary = successful_gpu_move_summary(organism_id, target_entity);
    assert_eq!(
        summary.selected_action_id,
        Some(HeadlessActionIds::APPROACH)
    );
    assert_eq!(summary.selected_action_kind, Some(ActionKind::Move));
    assert_eq!(summary.target_entity, Some(target_entity));
    let expected_world_position_a = authoritative_post_action_object.position;
    let (expected_x_a, expected_z_a) = production_voxel_center(expected_world_position_a);
    let expected_y = initial_translation.y;
    let live_tick_after_a = summary.world_tick_after.raw();
    assert!(
        (expected_x_a - initial_translation.x).abs() > f32::EPSILON
            || (expected_z_a - initial_translation.z).abs() > f32::EPSILON,
        "authoritative production position must differ from the initial rendered position"
    );
    let frame_a = LiveBrainPresentationFrame::try_new(
        vec![summary.clone()],
        summary.world_tick_after,
        vec![authoritative_post_action_object.clone().into()],
    )
    .unwrap();
    app.insert_resource(LiveBrainPresentationFrameResource::from_current_frame(
        frame_a,
    ));
    app.update();

    let actual_translation_a = {
        let mut roots = app
            .world_mut()
            .query::<(&ProductionCreatureAssemblyRoot, &Transform)>();
        roots
            .iter(app.world())
            .find(|(root, _)| root.stable_id == stable_id)
            .map(|(_, transform)| transform.translation)
            .expect("matching production creature root must remain present after update")
    };
    assert!(
        (actual_translation_a.x - expected_x_a).abs() < f32::EPSILON
            && (actual_translation_a.y - expected_y).abs() < f32::EPSILON
            && (actual_translation_a.z - expected_z_a).abs() < f32::EPSILON,
        "stable creature {} did not project the authoritative live position at live tick {}: initial=({:.3},{:.3},{:.3}) expected=({:.3},{:.3},{:.3}) actual=({:.3},{:.3},{:.3})",
        stable_id.raw(),
        live_tick_after_a,
        initial_translation.x,
        initial_translation.y,
        initial_translation.z,
        expected_x_a,
        expected_y,
        expected_z_a,
        actual_translation_a.x,
        actual_translation_a.y,
        actual_translation_a.z,
    );

    let mismatched_organism_id = OrganismId(organism_id.raw() + 20_000);
    let mut mismatched_identity_object = authoritative_post_action_object.clone();
    mismatched_identity_object.organism_id = Some(mismatched_organism_id);
    mismatched_identity_object.position = Vec3f::new(
        initial_object.position.x + 5.0,
        initial_object.position.y,
        initial_object.position.z - 2.0,
    );
    let mismatched_frame = LiveBrainPresentationFrame::try_new(
        vec![summary.clone()],
        Tick::new(summary.world_tick_after.raw() + 1),
        vec![mismatched_identity_object.into()],
    )
    .unwrap();
    app.insert_resource(LiveBrainPresentationFrameResource::from_current_frame(
        mismatched_frame,
    ));
    app.update();

    let child_id = WorldEntityId(stable_id.raw() + 10_000);
    let child_organism_id = OrganismId(organism_id.raw() + 10_000);
    let mut child_object = initial_object.clone();
    child_object.id = child_id;
    child_object.organism_id = Some(child_organism_id);
    child_object.label = "newborn-child".to_string();
    child_object.position = Vec3f::new(
        initial_object.position.x - 3.0,
        initial_object.position.y,
        initial_object.position.z + 4.0,
    );
    let second_child_id = WorldEntityId(stable_id.raw() + 20_000);
    let second_child_organism_id = OrganismId(organism_id.raw() + 30_000);
    let mut second_child_object = initial_object.clone();
    second_child_object.id = second_child_id;
    second_child_object.organism_id = Some(second_child_organism_id);
    second_child_object.label = "newborn-second-child".to_string();
    second_child_object.position = Vec3f::new(
        initial_object.position.x + 6.0,
        initial_object.position.y,
        initial_object.position.z - 3.0,
    );
    let expected_child_world_position = child_object.position;
    let (expected_child_x, expected_child_z) = production_voxel_center(expected_child_world_position);
    let expected_second_child_world_position = second_child_object.position;
    let (expected_second_child_x, expected_second_child_z) =
        production_voxel_center(expected_second_child_world_position);
    let frame_b = LiveBrainPresentationFrame::try_new(
        vec![summary.clone()],
        Tick::new(summary.world_tick_after.raw() + 2),
        vec![
            authoritative_post_action_object.into(),
            child_object.into(),
            second_child_object.into(),
        ],
    )
    .unwrap();
    app.insert_resource(LiveBrainPresentationFrameResource::from_current_frame(
        frame_b.clone(),
    ));
    app.update();

    let projected_roots = {
        let mut roots = app.world_mut().query::<(
            &ProductionCreatureAssemblyRoot,
            &Fvr03ProductionVoxelCreatureMarker,
            &Fvr04ProductionCreatureVisualMarker,
            &Transform,
        )>();
        roots
            .iter(app.world())
            .map(|(root, voxel, visual, transform)| {
                (
                    root.stable_id,
                    voxel.stable_id,
                    visual.stable_id,
                    visual.organism_id,
                    transform.translation,
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        projected_roots.len(),
        4,
        "mismatched identity plus snapshot B must create three distinct roots"
    );
    let existing_root = projected_roots
        .iter()
        .find(|(root_id, _, _, _, _)| *root_id == stable_id)
        .expect("snapshot B must retain the existing root by stable world identity");
    assert_eq!(existing_root.0, stable_id);
    assert_eq!(existing_root.1, stable_id);
    assert_eq!(existing_root.2, stable_id);
    assert_eq!(existing_root.3, organism_id);
    let mismatched_root = projected_roots
        .iter()
        .find(|(root_id, _, _, root_organism_id, _)| {
            *root_id == stable_id && *root_organism_id == mismatched_organism_id
        })
        .expect("a mismatched organism identity must not deduplicate the existing root");
    assert_eq!(mismatched_root.0, stable_id);
    assert_eq!(mismatched_root.1, stable_id);
    assert_eq!(mismatched_root.2, stable_id);
    assert_eq!(mismatched_root.3, mismatched_organism_id);
    let child_root = projected_roots
        .iter()
        .find(|(root_id, _, _, _, _)| *root_id == child_id)
        .expect("snapshot B must create the child root by stable world identity");
    assert_eq!(child_root.0, child_id);
    assert_eq!(child_root.1, child_id);
    assert_eq!(child_root.2, child_id);
    assert_eq!(child_root.3, child_organism_id);
    assert_eq!(child_root.4.x, expected_child_x);
    assert_eq!(child_root.4.y, expected_y);
    assert_eq!(child_root.4.z, expected_child_z);
    let second_child_root = projected_roots
        .iter()
        .find(|(root_id, _, _, _, _)| *root_id == second_child_id)
        .expect("snapshot B must create the second child root");
    assert_eq!(second_child_root.0, second_child_id);
    assert_eq!(second_child_root.1, second_child_id);
    assert_eq!(second_child_root.2, second_child_id);
    assert_eq!(second_child_root.3, second_child_organism_id);
    assert_eq!(second_child_root.4.x, expected_second_child_x);
    assert_eq!(second_child_root.4.y, expected_y);
    assert_eq!(second_child_root.4.z, expected_second_child_z);

    app.insert_resource(LiveBrainPresentationFrameResource::from_current_frame(frame_b));
    app.update();

    let root_count = {
        let mut roots = app.world_mut().query::<&ProductionCreatureAssemblyRoot>();
        roots.iter(app.world()).count()
    };
    assert_eq!(root_count, 4, "reapplying snapshot B must remain idempotent");
}

#[test]
fn curated_first_gpu_action_consumes_receipt_updates_registered_world_and_publishes_matching_live_frame(
) {
    let launch = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
    let (mut app, launch_summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.world_mut()
        .resource_mut::<Fvr05ProductionUxStateResource>()
        .settings
        .paused = true;
    app.update();

    let evidence = alife_game_app::GpuLiveBrainRuntime::run_curated_first_gpu_action_for_test(
        &launch_summary.save_path,
    )
    .expect("production cutover test seam must return causal evidence");
    assert_eq!(evidence.residency_gate_rejections, 5);
    assert_eq!(evidence.receipt.ordered_residents.len(), 2);
    assert_eq!(evidence.gpu_selection_count, 1);
    assert_eq!(evidence.sealed_patch_count, 1);

    let stable_id = evidence.selected_world_entity_id;
    let initial_translation = {
        let mut roots = app.world_mut().query::<(
            &ProductionCreatureAssemblyRoot,
            &Fvr04ProductionCreatureVisualMarker,
            &Transform,
        )>();
        roots
            .iter(app.world())
            .find(|(root, visual, _)| {
                root.stable_id == stable_id && visual.organism_id == evidence.selected_organism_id
            })
            .map(|(_, _, transform)| transform.translation)
            .expect("receipt-bound creature must have a production root");
    };
    let post_action_object = evidence
        .post_action_world
        .object_snapshots()
        .into_iter()
        .find(|object| object.id == stable_id)
        .expect("registered world must publish the receipt-bound object");
    assert_eq!(
        post_action_object.organism_id,
        Some(evidence.selected_organism_id)
    );
    assert_eq!(
        evidence.summary.world_tick_after,
        evidence.post_action_world.tick()
    );
    assert_eq!(evidence.summary.organism_id, evidence.selected_organism_id);
    assert!(evidence.summary.patch_sealed);
    assert_eq!(evidence.summary.patch_sequence_id, Some(1));
    assert_eq!(
        evidence.receipt.ordered_residents[0].organism_id,
        evidence.selected_organism_id
    );
    assert_eq!(
        evidence.receipt.ordered_residents[0]
            .opaque_target_identity
            .raw(),
        stable_id.raw()
    );
    assert!(
        (post_action_object.position.x - evidence.pre_action_position.x).abs() > f32::EPSILON
            || (post_action_object.position.z - evidence.pre_action_position.z).abs()
                > f32::EPSILON,
        "registered world action must change the bound object"
    );

    let frame = LiveBrainPresentationFrame::from_authoritative_world(
        vec![evidence.summary.clone()],
        &evidence.post_action_world,
    )
    .expect("post-action world must publish as a paired live frame");
    app.insert_resource(LiveBrainPresentationFrameResource::from_current_frame(
        frame,
    ));
    app.world_mut()
        .resource_mut::<Fvr05ProductionUxStateResource>()
        .settings
        .paused = false;
    app.update();

    let current_object = app
        .world()
        .resource::<LiveBrainPresentationFrameResource>()
        .current
        .object(stable_id)
        .expect("current live frame must retain the receipt-bound stable ID");
    assert_eq!(current_object.position, post_action_object.position);
    let (expected_x, expected_z) = production_voxel_center(current_object.position);
    let actual_translation = {
        let mut roots = app
            .world_mut()
            .query::<(&ProductionCreatureAssemblyRoot, &Transform)>();
        roots
            .iter(app.world())
            .find(|(root, _)| root.stable_id == stable_id)
            .map(|(_, transform)| transform.translation)
            .expect("stable-ID projection must keep the matching root")
    };
    assert!(
        (actual_translation.x - expected_x).abs() < f32::EPSILON
            && (actual_translation.y - initial_translation.y).abs() < f32::EPSILON
            && (actual_translation.z - expected_z).abs() < f32::EPSILON,
        "stable-ID projection must move the receipt-bound creature root"
    );

    let _ = launch_summary;
}

#[test]
fn fvr04_live_world_projection_ignores_unmatched_and_non_agent_objects() {
    let launch = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
    let (mut app, launch_summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.world_mut()
        .resource_mut::<Fvr05ProductionUxStateResource>()
        .settings
        .paused = true;
    app.update();

    let (stable_id, organism_id, initial_translation) = {
        let mut roots = app.world_mut().query::<(
            &ProductionCreatureAssemblyRoot,
            &Fvr04ProductionCreatureVisualMarker,
            &Transform,
        )>();
        roots
            .iter(app.world())
            .map(|(root, visual, transform)| {
                (root.stable_id, visual.organism_id, transform.translation)
            })
            .next()
            .expect("production voxel scene must spawn a creature root")
    };
    let save = PortableSaveFile::from_json_file(&launch_summary.save_path).unwrap();
    let initial_object = save
        .world
        .objects
        .iter()
        .find(|object| object.id == stable_id)
        .cloned()
        .expect("production creature root must have a matching saved world object");

    let mut moved_agent = initial_object.clone();
    moved_agent.position = Vec3f::new(
        initial_object.position.x + 2.0,
        initial_object.position.y,
        initial_object.position.z + 1.0,
    );
    app.insert_resource(LiveBrainPresentationFrameResource::from_current_frame(
        LiveBrainPresentationFrame::try_new(
            Vec::new(),
            Tick::new(8),
            vec![moved_agent.clone().into()],
        )
        .unwrap(),
    ));
    app.update();

    let moved_translation = {
        let mut roots = app
            .world_mut()
            .query::<(&ProductionCreatureAssemblyRoot, &Transform)>();
        roots
            .iter(app.world())
            .find(|(root, _)| root.stable_id == stable_id)
            .map(|(_, transform)| transform.translation)
            .expect("matching production creature root must remain present after update")
    };
    assert_ne!(moved_translation.x, initial_translation.x);
    assert_ne!(moved_translation.z, initial_translation.z);

    let mut unmatched_agent = moved_agent.clone();
    unmatched_agent.id = WorldEntityId(stable_id.raw() + 10_000);
    unmatched_agent.organism_id = Some(organism_id);
    unmatched_agent.position = Vec3f::new(-4.0, 0.0, 6.0);
    app.insert_resource(LiveBrainPresentationFrameResource::from_current_frame(
        LiveBrainPresentationFrame::try_new(Vec::new(), Tick::new(9), vec![unmatched_agent.into()])
            .unwrap(),
    ));
    app.update();

    let after_unmatched = {
        let mut roots = app
            .world_mut()
            .query::<(&ProductionCreatureAssemblyRoot, &Transform)>();
        roots
            .iter(app.world())
            .find(|(root, _)| root.stable_id == stable_id)
            .map(|(_, transform)| transform.translation)
            .expect("unmatched frame must not remove the production creature root")
    };
    assert_eq!(after_unmatched, moved_translation);

    let mut colliding_non_agent = moved_agent;
    colliding_non_agent.kind = WorldObjectKind::Food;
    colliding_non_agent.organism_id = None;
    colliding_non_agent.position = Vec3f::new(8.0, 0.0, -7.0);
    app.insert_resource(LiveBrainPresentationFrameResource::from_current_frame(
        LiveBrainPresentationFrame::try_new(
            Vec::new(),
            Tick::new(10),
            vec![colliding_non_agent.into()],
        )
        .unwrap(),
    ));
    app.update();

    let after_non_agent = {
        let mut roots = app
            .world_mut()
            .query::<(&ProductionCreatureAssemblyRoot, &Transform)>();
        roots
            .iter(app.world())
            .find(|(root, _)| root.stable_id == stable_id)
            .map(|(_, transform)| transform.translation)
            .expect("non-agent collision must not remove the production creature root")
    };
    assert_eq!(after_non_agent, moved_translation);
}

#[test]
fn fvr04_selection_marker_follows_the_moved_projected_creature_root() {
    let launch = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
    let (mut app, launch_summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.world_mut()
        .resource_mut::<Fvr05ProductionUxStateResource>()
        .settings
        .paused = true;
    app.update();

    let selected = app
        .world()
        .resource::<Fvr03ProductionVoxelSelectionResource>()
        .selected
        .expect("production voxel scene should select a creature at boot");
    assert_eq!(selected.kind, StableVoxelRefKind::Creature);
    let stable_id = selected
        .stable_id
        .expect("boot creature selection must have a stable id");
    let initial_translation = {
        let mut roots = app
            .world_mut()
            .query::<(&ProductionCreatureAssemblyRoot, &Transform)>();
        roots
            .iter(app.world())
            .find(|(root, _)| root.stable_id == stable_id)
            .map(|(_, transform)| transform.translation)
            .expect("selected creature must have a production assembly root")
    };
    let launch_scene_position = app
        .world()
        .resource::<Fvr03ProductionVoxelSceneResource>()
        .selection_position(stable_id)
        .expect("scene must retain the launch-time creature position for static metadata");
    let save = PortableSaveFile::from_json_file(&launch_summary.save_path).unwrap();
    let mut moved_object = save
        .world
        .objects
        .iter()
        .find(|object| object.id == stable_id)
        .cloned()
        .expect("selected creature must have a matching saved world object");
    moved_object.position = alife_core::Vec3f::new(
        moved_object.position.x + 2.0,
        moved_object.position.y,
        moved_object.position.z + 1.0,
    );
    let (expected_x, expected_z) = production_voxel_center(moved_object.position);
    assert_ne!(
        (expected_x, expected_z),
        (launch_scene_position.x, launch_scene_position.z),
        "the authoritative move must differ from the launch-time scene map"
    );

    app.insert_resource(LiveBrainPresentationFrameResource::from_current_frame(
        LiveBrainPresentationFrame::try_new(Vec::new(), Tick::new(8), vec![moved_object.into()])
            .unwrap(),
    ));
    app.update();

    let marker_translation = {
        let mut markers = app
            .world_mut()
            .query::<(&Fvr03ProductionVoxelSelectionMarker, &Transform)>();
        markers
            .iter(app.world())
            .next()
            .map(|(_, transform)| transform.translation)
            .expect("production selection marker must exist")
    };
    assert_eq!(marker_translation.y, 1.45);
    assert_eq!(
        (marker_translation.x, marker_translation.z),
        (expected_x, expected_z)
    );
    assert_ne!(
        (marker_translation.x, marker_translation.z),
        (launch_scene_position.x, launch_scene_position.z),
        "selection marker must not keep the launch-time creature position"
    );
    assert_ne!(initial_translation.x, marker_translation.x);
    assert_ne!(initial_translation.z, marker_translation.z);
}

#[test]
fn fvr04_camera_follow_recomputes_from_the_moved_projected_creature_root() {
    let launch = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
    let (mut app, launch_summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.world_mut()
        .resource_mut::<Fvr05ProductionUxStateResource>()
        .settings
        .paused = true;
    app.update();

    let selected = app
        .world()
        .resource::<Fvr03ProductionVoxelSelectionResource>()
        .selected
        .expect("production voxel scene should select a creature at boot");
    assert_eq!(selected.kind, StableVoxelRefKind::Creature);
    let stable_id = selected
        .stable_id
        .expect("boot creature selection must have a stable id");
    let initial_translation = {
        let mut roots = app
            .world_mut()
            .query::<(&ProductionCreatureAssemblyRoot, &Transform)>();
        roots
            .iter(app.world())
            .find(|(root, _)| root.stable_id == stable_id)
            .map(|(_, transform)| transform.translation)
            .expect("selected creature must have a production assembly root")
    };
    app.world_mut()
        .resource_mut::<Fvr04ProductionCreatureFollowResource>()
        .enabled = true;
    app.world_mut()
        .resource_mut::<Fvr04ProductionCreatureFollowResource>()
        .target_stable_id = Some(stable_id);
    app.update();

    let save = PortableSaveFile::from_json_file(&launch_summary.save_path).unwrap();
    let mut moved_object = save
        .world
        .objects
        .iter()
        .find(|object| object.id == stable_id)
        .cloned()
        .expect("selected creature must have a matching saved world object");
    moved_object.position = alife_core::Vec3f::new(
        moved_object.position.x + 2.0,
        moved_object.position.y,
        moved_object.position.z + 1.0,
    );
    let (expected_x, expected_z) = production_voxel_center(moved_object.position);
    app.insert_resource(LiveBrainPresentationFrameResource::from_current_frame(
        LiveBrainPresentationFrame::try_new(Vec::new(), Tick::new(8), vec![moved_object.into()])
            .unwrap(),
    ));
    app.update();

    let camera_transform = {
        let mut cameras = app
            .world_mut()
            .query::<(&Fvr03ProductionVoxelCamera, &Transform)>();
        cameras
            .iter(app.world())
            .next()
            .map(|(_, transform)| *transform)
            .expect("production terrain camera must exist")
    };
    let target = Vec3::new(expected_x, 0.0, expected_z);
    let expected_transform =
        Transform::from_translation(target + Vec3::new(17.2 * 0.56, 17.2 * 0.82, 17.2 * 0.58))
            .looking_at(target, Vec3::Y);
    assert!((camera_transform.translation - expected_transform.translation).length() < 1.0e-5);
    assert!((camera_transform.rotation.x - expected_transform.rotation.x).abs() < 1.0e-5);
    assert!((camera_transform.rotation.y - expected_transform.rotation.y).abs() < 1.0e-5);
    assert!((camera_transform.rotation.z - expected_transform.rotation.z).abs() < 1.0e-5);
    assert!((camera_transform.rotation.w - expected_transform.rotation.w).abs() < 1.0e-5);
    assert_ne!(camera_transform.translation.x, initial_translation.x);
    assert_ne!(camera_transform.translation.z, initial_translation.z);
}

#[test]
fn fvr04_missing_selected_creature_root_hides_marker_without_stale_scene_coordinates() {
    let launch = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.world_mut()
        .resource_mut::<Fvr05ProductionUxStateResource>()
        .settings
        .paused = true;
    app.update();

    let selected = app
        .world()
        .resource::<Fvr03ProductionVoxelSelectionResource>()
        .selected
        .expect("production voxel scene should select a creature at boot");
    assert_eq!(selected.kind, StableVoxelRefKind::Creature);
    let stable_id = selected
        .stable_id
        .expect("boot creature selection must have a stable id");
    let root_entity = {
        let mut roots = app
            .world_mut()
            .query::<(Entity, &ProductionCreatureAssemblyRoot)>();
        roots
            .iter(app.world())
            .find(|(_, root)| root.stable_id == stable_id)
            .map(|(entity, _)| entity)
            .expect("selected creature must have a production assembly root")
    };
    let stale_scene_position = app
        .world()
        .resource::<Fvr03ProductionVoxelSceneResource>()
        .selection_position(stable_id)
        .expect("scene must retain the launch-time creature position for this safety check");
    let sentinel = Vec3::new(-77.0, 1.45, 91.0);
    {
        let mut markers = app
            .world_mut()
            .query::<(&Fvr03ProductionVoxelSelectionMarker, &mut Transform)>();
        for (_, mut transform) in markers.iter_mut(app.world_mut()) {
            transform.translation = sentinel;
        }
    }
    app.world_mut().despawn(root_entity);
    app.world_mut()
        .resource_mut::<Fvr03ProductionVoxelSelectionResource>()
        .selected = None;
    app.update();
    app.world_mut()
        .resource_mut::<Fvr03ProductionVoxelSelectionResource>()
        .selected = Some(selected);
    app.update();

    let (marker_translation, marker_visibility) = {
        let mut markers = app.world_mut().query::<(
            &Fvr03ProductionVoxelSelectionMarker,
            &Transform,
            &Visibility,
        )>();
        markers
            .iter(app.world())
            .next()
            .map(|(_, transform, visibility)| (transform.translation, *visibility))
            .expect("selection marker must remain as a safe presentation entity")
    };
    assert_eq!(marker_visibility, Visibility::Hidden);
    assert_eq!(marker_translation, sentinel);
    assert_ne!(
        (marker_translation.x, marker_translation.z),
        (stale_scene_position.x, stale_scene_position.z),
        "a missing root must not restore launch-time creature coordinates"
    );
}

#[test]
fn fvr04_live_world_label_tracks_projected_root_by_stable_id() {
    let launch = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
    let (mut app, launch_summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.world_mut()
        .resource_mut::<Fvr05ProductionUxStateResource>()
        .settings
        .paused = true;
    app.update();

    let selected = app
        .world()
        .resource::<Fvr03ProductionVoxelSelectionResource>()
        .selected
        .expect("production voxel scene should select a creature at boot");
    assert_eq!(selected.kind, StableVoxelRefKind::Creature);
    let stable_id = selected
        .stable_id
        .expect("boot creature selection must have a stable id");
    let initial_label_translation = {
        let mut labels = app
            .world_mut()
            .query::<(&Fvr04ProductionCreatureWorldLabel, &Transform)>();
        labels
            .iter(app.world())
            .next()
            .map(|(_, transform)| transform.translation)
            .expect("production creature world label must exist")
    };

    let save = PortableSaveFile::from_json_file(&launch_summary.save_path).unwrap();
    let mut moved_object = save
        .world
        .objects
        .iter()
        .find(|object| object.id == stable_id)
        .cloned()
        .expect("selected creature must have a matching saved world object");
    moved_object.position = Vec3f::new(
        moved_object.position.x + 2.0,
        moved_object.position.y,
        moved_object.position.z + 1.0,
    );
    app.insert_resource(LiveBrainPresentationFrameResource::from_current_frame(
        LiveBrainPresentationFrame::try_new(Vec::new(), Tick::new(8), vec![moved_object.into()])
            .unwrap(),
    ));
    app.update();

    let root_translation = {
        let mut roots = app
            .world_mut()
            .query::<(&ProductionCreatureAssemblyRoot, &Transform)>();
        roots
            .iter(app.world())
            .find(|(root, _)| root.stable_id == stable_id)
            .map(|(_, transform)| transform.translation)
            .expect("selected creature must retain its production assembly root")
    };
    let label_translation = {
        let mut labels =
            app.world_mut()
                .query::<(&Fvr04ProductionCreatureWorldLabel, &Transform, &Visibility)>();
        labels
            .iter(app.world())
            .next()
            .map(|(_, transform, visibility)| (transform.translation, *visibility))
            .expect("production creature world label must remain present")
    };

    assert_eq!(label_translation.1, Visibility::Visible);
    assert_eq!(
        (label_translation.0.x, label_translation.0.z),
        (root_translation.x, root_translation.z),
        "the selected label must follow the projected root for stable creature {}",
        stable_id.raw()
    );
    assert_eq!(label_translation.0.y, 2.35);
    assert_ne!(
        (label_translation.0.x, label_translation.0.z),
        (initial_label_translation.x, initial_label_translation.z),
        "the label must update when the live root moves without changing selection"
    );
}

#[test]
fn fvr04_live_world_label_hides_when_selected_creature_root_missing() {
    let launch = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.world_mut()
        .resource_mut::<Fvr05ProductionUxStateResource>()
        .settings
        .paused = true;
    app.update();

    let selected = app
        .world()
        .resource::<Fvr03ProductionVoxelSelectionResource>()
        .selected
        .expect("production voxel scene should select a creature at boot");
    assert_eq!(selected.kind, StableVoxelRefKind::Creature);
    let stable_id = selected
        .stable_id
        .expect("boot creature selection must have a stable id");
    let root_entity = {
        let mut roots = app
            .world_mut()
            .query::<(Entity, &ProductionCreatureAssemblyRoot)>();
        roots
            .iter(app.world())
            .find(|(_, root)| root.stable_id == stable_id)
            .map(|(entity, _)| entity)
            .expect("selected creature must have a production assembly root")
    };
    let sentinel = Vec3::new(-77.0, 2.35, 91.0);
    {
        let mut labels = app.world_mut().query::<(
            &Fvr04ProductionCreatureWorldLabel,
            &mut Transform,
            &mut Visibility,
        )>();
        for (_, mut transform, mut visibility) in labels.iter_mut(app.world_mut()) {
            transform.translation = sentinel;
            *visibility = Visibility::Visible;
        }
    }
    app.world_mut().despawn(root_entity);
    app.update();

    let label_state = {
        let mut labels =
            app.world_mut()
                .query::<(&Fvr04ProductionCreatureWorldLabel, &Transform, &Visibility)>();
        labels
            .iter(app.world())
            .next()
            .map(|(_, transform, visibility)| (transform.translation, *visibility))
            .expect("production creature world label must remain present")
    };
    assert_eq!(label_state.1, Visibility::Hidden);
    assert_eq!(label_state.0, sentinel);
}

#[test]
fn fvr04_live_creature_inspectors_report_current_authoritative_position_and_tick() {
    let launch = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
    let (mut app, launch_summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    {
        let mut ux = app
            .world_mut()
            .resource_mut::<Fvr05ProductionUxStateResource>();
        ux.settings.paused = true;
        ux.settings.active_inspector_tab = Fvr05ProductionInspectorTab::Creature;
    }
    app.update();

    let selected = app
        .world()
        .resource::<Fvr03ProductionVoxelSelectionResource>()
        .selected
        .expect("production voxel scene should select a creature at boot");
    let stable_id = selected
        .stable_id
        .expect("boot creature selection must have a stable id");
    let save = PortableSaveFile::from_json_file(&launch_summary.save_path).unwrap();
    let mut moved_object = save
        .world
        .objects
        .iter()
        .find(|object| object.id == stable_id)
        .cloned()
        .expect("selected creature must have a matching saved world object");
    moved_object.position = Vec3f::new(11.25, 4.5, -12.75);
    let live_tick = Tick::new(42);
    app.insert_resource(LiveBrainPresentationFrameResource::from_current_frame(
        LiveBrainPresentationFrame::try_new(Vec::new(), live_tick, vec![moved_object.into()])
            .unwrap(),
    ));
    app.update();

    let fvr04_text = {
        let mut panels = app
            .world_mut()
            .query::<(&Fvr04ProductionCreatureInspectorPanel, &Text)>();
        panels
            .iter(app.world())
            .next()
            .map(|(_, text)| text.0.clone())
            .expect("FVR04 creature inspector panel must exist")
    };
    let fvr05_text = {
        let mut panels = app
            .world_mut()
            .query::<(&Fvr05ProductionRightInspectorPanel, &Text)>();
        panels
            .iter(app.world())
            .next()
            .map(|(_, text)| text.0.clone())
            .expect("FVR05 right inspector panel must exist")
    };
    let expected_position = format!(
        "world position: x={:.2} y={:.2} z={:.2}",
        11.25, 4.5, -12.75
    );
    for text in [&fvr04_text, &fvr05_text] {
        assert!(
            text.contains("world tick: 42"),
            "missing live tick in: {text}"
        );
        assert!(
            text.contains(&expected_position),
            "missing current authoritative position in: {text}"
        );
        assert!(
            text.contains("PRESENTATION METADATA (launch/save)"),
            "static expression/body values must be labeled as metadata in: {text}"
        );
    }
}

#[test]
fn fvr04_live_creature_inspectors_reject_stable_id_mismatch() {
    let launch = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
    let (mut app, launch_summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    {
        let mut ux = app
            .world_mut()
            .resource_mut::<Fvr05ProductionUxStateResource>();
        ux.settings.paused = true;
        ux.settings.active_inspector_tab = Fvr05ProductionInspectorTab::Creature;
    }
    app.update();

    let selected = app
        .world()
        .resource::<Fvr03ProductionVoxelSelectionResource>()
        .selected
        .expect("production voxel scene should select a creature at boot");
    let stable_id = selected
        .stable_id
        .expect("boot creature selection must have a stable id");
    let root_entity = {
        let mut roots = app
            .world_mut()
            .query::<(Entity, &ProductionCreatureAssemblyRoot)>();
        roots
            .iter(app.world())
            .find(|(_, root)| root.stable_id == stable_id)
            .map(|(entity, _)| entity)
            .expect("selected creature must have a production assembly root")
    };
    let mismatched_root_id = WorldEntityId(stable_id.raw() + 50_000);
    app.world_mut()
        .get_mut::<ProductionCreatureAssemblyRoot>(root_entity)
        .expect("selected creature root must remain mutable")
        .stable_id = mismatched_root_id;

    let save = PortableSaveFile::from_json_file(&launch_summary.save_path).unwrap();
    let mut moved_object = save
        .world
        .objects
        .iter()
        .find(|object| object.id == stable_id)
        .cloned()
        .expect("selected creature must have a matching saved world object");
    moved_object.position = Vec3f::new(19.25, 5.5, -23.75);
    app.insert_resource(LiveBrainPresentationFrameResource::from_current_frame(
        LiveBrainPresentationFrame::try_new(Vec::new(), Tick::new(77), vec![moved_object.into()])
            .unwrap(),
    ));
    app.update();

    let fvr04_text = {
        let mut panels = app
            .world_mut()
            .query::<(&Fvr04ProductionCreatureInspectorPanel, &Text)>();
        panels
            .iter(app.world())
            .next()
            .map(|(_, text)| text.0.clone())
            .expect("FVR04 creature inspector panel must exist")
    };
    let fvr05_text = {
        let mut panels = app
            .world_mut()
            .query::<(&Fvr05ProductionRightInspectorPanel, &Text)>();
        panels
            .iter(app.world())
            .next()
            .map(|(_, text)| text.0.clone())
            .expect("FVR05 right inspector panel must exist")
    };
    let expected_unavailable = format!(
        "live state: unavailable for selected stable {}",
        stable_id.raw()
    );
    let mismatched_position = "world position: x=19.25 y=5.50 z=-23.75";
    for text in [&fvr04_text, &fvr05_text] {
        assert!(
            text.contains(&expected_unavailable),
            "stable-ID mismatch must be explicit in: {text}"
        );
        assert!(!text.contains("world tick: 77"));
        assert!(!text.contains(mismatched_position));
    }
}

#[test]
fn fvr04_non_creature_selection_keeps_the_static_scene_coordinate_path() {
    let launch = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.world_mut()
        .resource_mut::<Fvr05ProductionUxStateResource>()
        .settings
        .paused = true;
    app.update();

    let mut static_selection: StableVoxelObjectRef = app
        .world()
        .resource::<Fvr03ProductionVoxelSelectionResource>()
        .selected
        .expect("production voxel scene should select a tile-bearing object at boot");
    static_selection.kind = StableVoxelRefKind::Tile;
    static_selection.stable_id = None;
    let tile = static_selection
        .tile
        .expect("the static selection fixture must retain a tile coordinate");
    app.world_mut()
        .resource_mut::<Fvr03ProductionVoxelSelectionResource>()
        .selected = Some(static_selection);
    app.update();

    let marker_translation = {
        let mut markers = app
            .world_mut()
            .query::<(&Fvr03ProductionVoxelSelectionMarker, &Transform)>();
        markers
            .iter(app.world())
            .next()
            .map(|(_, transform)| transform.translation)
            .expect("production selection marker must exist")
    };
    assert_eq!(
        marker_translation,
        Vec3::new(tile.x as f32 + 0.5, 1.45, tile.z as f32 + 0.5)
    );
}

#[test]
fn fvr04_selection_marker_has_explicit_visibility_for_root_readers() {
    let launch = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    let marker_entity = {
        let mut markers = app
            .world_mut()
            .query::<(Entity, &Fvr03ProductionVoxelSelectionMarker)>();
        markers
            .iter(app.world())
            .next()
            .map(|(entity, _)| entity)
            .expect("production selection marker must exist")
    };
    assert_eq!(
        app.world().get::<Visibility>(marker_entity),
        Some(&Visibility::Visible)
    );
}

#[test]
fn v0_default_player_view_is_compact_and_uses_real_selected_creature_state() {
    let launch = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();

    let ux = app.world().resource::<Fvr05ProductionUxStateResource>();
    assert!(!ux.settings.show_menu);
    assert!(!ux.settings.show_settings);
    assert!(!ux.settings.show_overlays);

    let mut status_query = app
        .world_mut()
        .query::<(&V0PlayerStatusChip, &Text, &Visibility)>();
    let statuses = status_query.iter(app.world()).collect::<Vec<_>>();
    assert_eq!(statuses.len(), 1);
    assert_ne!(*statuses[0].2, Visibility::Hidden);

    let mut panel_query = app
        .world_mut()
        .query::<(&V0PlayerCreaturePanel, &Text, &Visibility)>();
    let panels = panel_query.iter(app.world()).collect::<Vec<_>>();
    assert_eq!(panels.len(), 1);
    assert_ne!(*panels[0].2, Visibility::Hidden);
    let panel_text = panels[0].1 .0.as_str();
    for real_state_heading in ["NEEDS", "LEARNING", "SOCIAL"] {
        assert!(panel_text.contains(real_state_heading), "{panel_text}");
    }
    for debug_term in ["backend", "chunk", "wgpu", "GPU"] {
        assert!(!panel_text.contains(debug_term), "{panel_text}");
    }

    let mut controls_query = app
        .world_mut()
        .query::<(&V0PlayerControlStrip, &Text, &Visibility)>();
    let controls = controls_query.iter(app.world()).collect::<Vec<_>>();
    assert_eq!(controls.len(), 1);
    assert_ne!(*controls[0].2, Visibility::Hidden);
    assert!(controls[0].1 .0.contains("R Recover view"));
}

#[test]
fn v0_recovery_key_restores_the_clean_isometric_player_view() {
    let launch = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();

    {
        let mut ux = app
            .world_mut()
            .resource_mut::<Fvr05ProductionUxStateResource>();
        ux.settings.show_menu = true;
        ux.settings.show_settings = true;
        ux.settings.show_overlays = true;
    }
    app.world_mut()
        .resource_mut::<Fvr04ProductionCreatureFollowResource>()
        .enabled = true;
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyR);
    app.update();

    let ux = app.world().resource::<Fvr05ProductionUxStateResource>();
    assert!(!ux.settings.show_menu);
    assert!(!ux.settings.show_settings);
    assert!(!ux.settings.show_overlays);
    assert_eq!(ux.last_action, "Recovered the player view");
    assert!(
        !app.world()
            .resource::<Fvr04ProductionCreatureFollowResource>()
            .enabled
    );
    let mut camera_query = app.world_mut().query::<&Fvr03ProductionVoxelCamera>();
    assert!(camera_query
        .iter(app.world())
        .all(|camera| camera.mode == Fvr03ProductionVoxelCameraMode::OrthographicIsometric));
    let mut projection_query = app.world_mut().query::<&Projection>();
    assert!(projection_query.iter(app.world()).all(|projection| {
        matches!(projection, Projection::Orthographic(orthographic) if orthographic.area.height() <= 16.0)
    }));
}

#[test]
fn v0_render_world_direction_is_warm_readable_and_creature_led() {
    let launch = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();

    let terrain_handles = {
        let mut query = app.world_mut().query::<(
            &Fvr11ProductionTerrainLayer,
            &MeshMaterial3d<StandardMaterial>,
        )>();
        query
            .iter(app.world())
            .filter(|(layer, _)| layer.role == Fvr11TerrainSurfaceRole::Top)
            .map(|(layer, material)| (layer.material, material.0.clone()))
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    let materials = app.world().resource::<Assets<StandardMaterial>>();
    let terrain_color = |kind| {
        materials
            .get(&terrain_handles[&kind])
            .expect("terrain material remains resident")
            .base_color
            .to_srgba()
    };
    let grass = terrain_color(Fvr03ProductionVoxelMaterialKind::SafeGrass);
    let soil = terrain_color(Fvr03ProductionVoxelMaterialKind::Soil);
    assert!(grass.green > grass.red * 1.05 && grass.green > grass.blue * 1.12);
    assert!(soil.red > soil.green * 1.10 && soil.green > soil.blue * 1.08);

    let (ambient, vertical_area) = {
        let mut query = app.world_mut().query::<(&AmbientLight, &Projection)>();
        let (ambient, projection) = query.iter(app.world()).next().expect("player camera");
        let Projection::Orthographic(orthographic) = projection else {
            panic!("player camera must stay orthographic");
        };
        (ambient.clone(), orthographic.area.height())
    };
    let ambient_color = ambient.color.to_srgba();
    assert!(ambient_color.red >= ambient_color.blue * 0.92);
    assert!(ambient.brightness >= 700.0);
    assert!(vertical_area <= 16.0);
    let mut sun_query = app.world_mut().query::<&DirectionalLight>();
    let sun = sun_query.iter(app.world()).next().expect("warm sun");
    assert!(sun.shadows_enabled);
    assert!((5_600.0..=6_000.0).contains(&sun.illuminance));

    let mut roots = app.world_mut().query::<(
        &ProductionCreatureAssemblyRoot,
        &Fvr04ProductionCreatureVisualMarker,
    )>();
    assert!(roots
        .iter(app.world())
        .all(|(root, creature)| root.display_only && creature.base_scale.z >= 1.20));
    let creature_material_handle = {
        let mut parts = app.world_mut().query::<(
            &ProductionCreaturePartMarker,
            &MeshMaterial3d<StandardMaterial>,
        )>();
        parts
            .iter(app.world())
            .next()
            .map(|(_, material)| material.0.clone())
            .expect("visible creature part")
    };
    let creature_material = app
        .world()
        .resource::<Assets<StandardMaterial>>()
        .get(&creature_material_handle)
        .expect("creature material remains resident");
    assert!(creature_material.perceptual_roughness <= 0.70);
    assert!(creature_material.reflectance >= 0.30);
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
    assert_eq!(unique_roles, BTreeSet::from(["belly-coat-marking"]));
    assert!(details.iter().all(|marker| marker.display_only
        && marker.no_renderer_authority_over_actions_or_cognition
        && marker.high_contrast_marking
        && marker.heritable));
}

#[test]
fn fvr10_surface_details_are_rendered_children_not_invisible_markers() {
    let launch = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();

    let marker_count = {
        let mut query = app.world_mut().query::<&Fvr10CreatureSurfaceDetailMarker>();
        query.iter(app.world()).count()
    };
    let rendered_details = {
        let mut query = app.world_mut().query::<(
            &Fvr10CreatureSurfaceDetailMarker,
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
            &bevy::prelude::ChildOf,
            &Transform,
        )>();
        query
            .iter(app.world())
            .map(|(marker, mesh, material, parent, transform)| {
                (
                    *marker,
                    mesh.0.id(),
                    material.0.id(),
                    parent.parent(),
                    transform.translation,
                )
            })
            .collect::<Vec<_>>()
    };

    assert!(
        marker_count > 0,
        "production scene must declare surface details"
    );
    assert_eq!(
        rendered_details.len(),
        marker_count,
        "every surface detail marker must own visible geometry and a material"
    );
    assert!(rendered_details
        .iter()
        .all(|(marker, _, _, parent, offset)| {
            app.world()
                .get::<ProductionCreaturePartMarker>(*parent)
                .is_some_and(|part| part.stable_id == marker.stable_id)
                && offset.is_finite()
                && offset.length() < 1.0
        }));

    let face_parents = {
        let mut query = app
            .world_mut()
            .query::<(&Fvr09CreatureFaceFeatureMarker, &bevy::prelude::ChildOf)>();
        query
            .iter(app.world())
            .map(|(marker, parent)| (*marker, parent.parent()))
            .collect::<Vec<_>>()
    };
    assert!(!face_parents.is_empty());
    assert!(face_parents.iter().all(|(face, parent)| {
        app.world()
            .get::<ProductionCreaturePartMarker>(*parent)
            .is_some_and(|part| {
                part.stable_id == face.stable_id && part.slot == CreaturePartSlot::Head
            })
    }));

    let cover_parents = {
        let mut query = app
            .world_mut()
            .query::<(&ProductionCreatureJoinCoverMarker, &bevy::prelude::ChildOf)>();
        query
            .iter(app.world())
            .map(|(marker, parent)| (*marker, parent.parent()))
            .collect::<Vec<_>>()
    };
    assert!(!cover_parents.is_empty());
    assert!(cover_parents.iter().all(|(cover, parent)| {
        app.world()
            .get::<ProductionCreaturePartMarker>(*parent)
            .is_some_and(|part| part.stable_id == cover.stable_id)
    }));
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
    let face_features = face_query
        .iter(app.world())
        .map(|(marker, transform)| (marker.feature, transform.translation))
        .collect::<Vec<_>>();
    assert!(!face_features.is_empty());
    assert!(
        face_features.iter().all(|(_, offset)| offset.is_finite()),
        "source-space face landmarks must resolve to finite local offsets"
    );
    for required in [
        "left-eye-sclera",
        "right-eye-sclera",
        "left-eye-iris",
        "right-eye-iris",
        "left-eye-pupil",
        "right-eye-pupil",
        "left-eye-glint",
        "right-eye-glint",
        "left-eye-lid",
        "right-eye-lid",
    ] {
        assert!(
            face_features
                .iter()
                .any(|(feature, _)| *feature == required),
            "layered expressive face should include {required}"
        );
    }
    assert!(face_features.iter().all(|(feature, _)| !matches!(
        *feature,
        "soft-mouth" | "generic-muzzle" | "duplicate-face"
    )));
}

#[test]
fn fvr03_geneforge_children_preserve_real_asset_groups_and_one_coat_handle() {
    let launch = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();

    let mut query = app.world_mut().query::<(
        &ProductionCreaturePartMarker,
        &Mesh3d,
        &MeshMaterial3d<StandardMaterial>,
        &Transform,
    )>();
    let parts = query
        .iter(app.world())
        .map(|(marker, mesh, material, transform)| {
            (
                marker.stable_id.raw(),
                marker.asset_id.clone(),
                marker.runtime_group.clone(),
                marker.authored_matrix,
                mesh.0.id(),
                material.0.id(),
                *transform,
            )
        })
        .collect::<Vec<_>>();
    assert!(!parts.is_empty());
    assert!(parts
        .iter()
        .all(|(_, asset_id, group, matrix, _, _, transform)| {
            !asset_id.0.starts_with("legacy-family-")
                && !group.is_empty()
                && matrix[12..] == [0.0, 0.0, 0.0, 1.0]
                && matrix.iter().all(|value| value.is_finite())
                && transform.scale.is_finite()
                && transform.scale.min_element() > 0.0
        }));

    let mut materials_by_assembly = std::collections::BTreeMap::<u64, BTreeSet<_>>::new();
    for (stable_id, _, _, _, _, material_id, _) in &parts {
        materials_by_assembly
            .entry(*stable_id)
            .or_default()
            .insert(*material_id);
    }
    assert!(materials_by_assembly
        .values()
        .all(|handles| handles.len() == 1));
}

#[test]
fn fvr03_geneforge_face_is_embedded_and_renderer_is_display_only() {
    let launch = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();

    let mut head_query = app
        .world_mut()
        .query::<(Entity, &ProductionCreaturePartMarker, &Mesh3d)>();
    let heads = head_query
        .iter(app.world())
        .filter(|(_, marker, _)| marker.slot == alife_game_app::CreaturePartSlot::Head)
        .map(|(entity, _, mesh)| (entity, mesh.0.clone()))
        .collect::<Vec<_>>();
    let meshes = app.world().resource::<Assets<Mesh>>();
    let head_bounds = heads
        .into_iter()
        .map(|(entity, mesh)| {
            let Some(VertexAttributeValues::Float32x3(positions)) = meshes
                .get(&mesh)
                .expect("head mesh is resident")
                .attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                panic!("head mesh is missing positions");
            };
            let mut min = bevy::prelude::Vec3::splat(f32::INFINITY);
            let mut max = bevy::prelude::Vec3::splat(f32::NEG_INFINITY);
            for position in positions {
                min = min.min(bevy::prelude::Vec3::from_array(*position));
                max = max.max(bevy::prelude::Vec3::from_array(*position));
            }
            (entity, (min, max))
        })
        .collect::<std::collections::BTreeMap<_, _>>();

    let mut face_query = app
        .world_mut()
        .query::<(&Fvr09CreatureFaceFeatureMarker, &ChildOf, &Transform)>();
    let eyes = face_query
        .iter(app.world())
        .filter(|(marker, _, _)| {
            marker.feature.starts_with("left-eye") || marker.feature.starts_with("right-eye")
        })
        .collect::<Vec<_>>();
    assert!(
        eyes.len() >= 12,
        "two complete embedded eye structures are required per assembly"
    );
    assert!(eyes.iter().all(|(marker, parent, transform)| {
        let Some((min, max)) = head_bounds.get(&parent.parent()) else {
            return false;
        };
        let embedded = (0..3).all(|axis| {
            (min[axis] - 0.20..=max[axis] + 0.20).contains(&transform.translation[axis])
        });
        let readable = !marker.feature.ends_with("sclera") || transform.scale.x >= 1.15;
        parent.parent() != Entity::PLACEHOLDER && embedded && readable
    }));

    let scene = app
        .world()
        .resource::<alife_game_app::Fvr04ProductionCreatureSceneResource>();
    assert!(scene.production_visuals_display_only);
    assert!(scene.no_renderer_authority_over_actions_or_cognition);
    assert!(scene.expression_buffer_is_read_only_projection);
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
