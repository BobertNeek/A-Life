#![cfg(feature = "bevy-app")]

use std::{collections::BTreeSet, fs, path::PathBuf};

use alife_core::{OrganismId, Tick, Vec3f, WorldEntityId};
use alife_game_app::bevy_shell::{LiveBrainPresentationFrame, LiveBrainPresentationFrameResource};
#[cfg(not(feature = "vfx-hanabi"))]
use alife_game_app::Fvr07ProductionVfxKind;
use alife_game_app::{
    run_production_voxel_frontend_dry_run, Fvr03ProductionVoxelCamera,
    Fvr03ProductionVoxelCameraMode, Fvr03ProductionVoxelMaterialKind,
    Fvr03ProductionVoxelSceneResource, Fvr03ProductionVoxelSelectionMarker,
    Fvr03ProductionVoxelSelectionResource, Fvr03ProductionVoxelTerrainBatch,
    Fvr04ProductionCreatureFollowResource, Fvr04ProductionCreatureVisualMarker,
    Fvr04ProductionCreatureWorldLabel, Fvr05ProductionInspectorTab,
    Fvr05ProductionRightInspectorPanel, Fvr05ProductionUxStateResource,
    Fvr07ProductionDressingKind, Fvr07ProductionGpuVfxMarker, Fvr07ProductionVisualDressing,
    Fvr09CreatureFaceFeatureMarker, Fvr09CuteBipedCreatureMarker, Fvr09MesherMode,
    Fvr10CreatureSpeciesMarker, Fvr10CreatureSurfaceDetailMarker, Fvr11ProductionContactShadow,
    Fvr11ProductionTerrainLayer, Fvr11ProductionTerrainLightingMarker,
    Fvr11ProductionTerrainMaterialContract, Fvr11ProductionTerrainSceneResource,
    Fvr11TerrainSurfaceRole, ProductionCreatureAssemblyRoot, ProductionCreatureJoinCoverMarker,
    ProductionCreaturePartMarker, ProductionFrontendProfileId, ProductionVoxelLaunchConfig,
    V0PlayerControlStrip, V0PlayerCreaturePanel, V0PlayerStatusChip,
    FVR03_PRODUCTION_VOXEL_RENDERER_SCHEMA, FVR11_PRODUCTION_TERRAIN_VISUAL_VERSION,
};
use alife_world::{
    persistence::PortableSaveFile, CreatureAppearanceGenome, StableVoxelObjectRef,
    StableVoxelRefKind, WorldObjectKind, CREATURE_APPEARANCE_SPECIES_COUNT,
    FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA,
};
use bevy::{
    color::ColorToComponents,
    mesh::VertexAttributeValues,
    prelude::{
        AlphaMode, AmbientLight, Assets, ButtonInput, DirectionalLight, Entity, KeyCode, Mesh,
        Mesh3d, MeshMaterial3d, Projection, StandardMaterial, Text, Transform, Vec3, Visibility,
    },
};

#[path = "support/current_production_launch.rs"]
mod current_production_launch;

fn production_ground_position(world_position: Vec3f) -> (f32, f32) {
    (world_position.x, world_position.z)
}

fn production_launch(
    profile_id: ProductionFrontendProfileId,
) -> (
    current_production_launch::CurrentProductionLaunchFixture,
    ProductionVoxelLaunchConfig,
) {
    let fixture = current_production_launch::CurrentProductionLaunchFixture::new();
    let mut launch = fixture.launch.clone();
    launch.profile_id = profile_id;
    launch.population = Some(profile_id.budget().default_population);
    launch.smoke_seconds = Some(1);
    launch.dry_run = true;
    (fixture, launch)
}

fn quantized_rgba(color: [f32; 4]) -> [i32; 4] {
    [
        (color[0] * 255.0).round() as i32,
        (color[1] * 255.0).round() as i32,
        (color[2] * 255.0).round() as i32,
        (color[3] * 255.0).round() as i32,
    ]
}

/// CPU preflight of shipping asset bytes. This does not prove scene loading or rendering.
fn shipping_glb(relative_path: &str) -> serde_json::Value {
    let bytes = fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join(relative_path),
    )
    .unwrap();
    assert_eq!(&bytes[..4], b"glTF");
    assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 2);
    assert_eq!(
        u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize,
        bytes.len()
    );
    assert_eq!(&bytes[16..20], b"JSON");
    let length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    serde_json::from_slice(&bytes[20..20 + length]).unwrap()
}

fn shipping_hearthling() -> serde_json::Value {
    let doc = shipping_glb("creatures/hearthling/hearthling.glb");
    assert_eq!(doc["skins"].as_array().unwrap().len(), 1);
    assert_eq!(doc["meshes"].as_array().unwrap().len(), 1);
    assert_eq!(doc["materials"].as_array().unwrap().len(), 6);
    let clips = doc["animations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["name"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        clips,
        BTreeSet::from([
            "Hearthling_CuriousIdle",
            "Hearthling_Walk",
            "Hearthling_Sleep"
        ])
    );
    let joints = doc["skins"][0]["joints"].as_array().unwrap();
    let names = joints
        .iter()
        .map(|i| {
            doc["nodes"][i.as_u64().unwrap() as usize]["name"]
                .as_str()
                .unwrap()
        })
        .collect::<BTreeSet<_>>();
    for name in [
        "head",
        "eye.L",
        "eye.R",
        "ear.L",
        "ear.R",
        "foot.L",
        "foot.R",
        "tail.00",
        "lid_upper.L",
        "lid_upper.R",
    ] {
        assert!(names.contains(name), "missing authored joint {name}");
    }
    let bind = doc["skins"][0]["inverseBindMatrices"].as_u64().unwrap() as usize;
    assert_eq!(
        doc["accessors"][bind]["count"].as_u64().unwrap() as usize,
        joints.len()
    );
    assert_eq!(doc["accessors"][bind]["type"], "MAT4");
    let primitives = doc["meshes"][0]["primitives"].as_array().unwrap();
    assert_eq!(primitives.len(), 6);
    let mut triangles = 0;
    for primitive in primitives {
        let attributes = primitive["attributes"].as_object().unwrap();
        let position = &doc["accessors"][attributes["POSITION"].as_u64().unwrap() as usize];
        let count = position["count"].as_u64().unwrap();
        assert!(count >= 24);
        for attribute in ["NORMAL", "TEXCOORD_0", "COLOR_0", "JOINTS_0", "WEIGHTS_0"] {
            let accessor = attributes[attribute].as_u64().unwrap() as usize;
            assert_eq!(doc["accessors"][accessor]["count"].as_u64().unwrap(), count);
        }
        for axis in 0..3 {
            let min = position["min"][axis].as_f64().unwrap();
            let max = position["max"][axis].as_f64().unwrap();
            assert!(min.is_finite() && max.is_finite() && max > min);
        }
        assert!(primitive["material"].as_u64().unwrap() < 6);
        let indices = primitive["indices"].as_u64().unwrap() as usize;
        triangles += doc["accessors"][indices]["count"].as_u64().unwrap() / 3;
    }
    assert!((10_000..=32_768).contains(&triangles));
    doc
}

#[test]
fn runtime_checkpoint_corruption_is_explicit_and_never_overwritten() {
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
    let summary = run_production_voxel_frontend_dry_run(&launch).unwrap();
    let runtime_save_path = PathBuf::from(&summary.ui_settings.runtime_save_path);
    let source_before = fs::read(&summary.save_path).unwrap();
    fs::create_dir_all(runtime_save_path.parent().unwrap()).unwrap();
    let corrupt = br#"{"schema":"stale-derived-runtime"}"#;
    fs::write(&runtime_save_path, corrupt).unwrap();
    assert!(matches!(
        PortableSaveFile::from_json_file(&runtime_save_path),
        Err(alife_world::persistence::PersistenceError::Schema { expected, actual })
            if expected == alife_world::persistence::P34_SAVE_FILE_SCHEMA
                && actual == "stale-derived-runtime"
    ));
    #[cfg(feature = "gpu-runtime")]
    {
        let result = alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch);
        assert!(matches!(
            result,
            Err(alife_game_app::GameAppShellError::GpuRuntime(
                alife_game_app::GpuRuntimeError::Persistence(
                    alife_world::persistence::PersistenceError::Schema { expected, actual }
                )
            )) if expected == alife_world::persistence::P34_SAVE_FILE_SCHEMA
                && actual == "stale-derived-runtime"
        ));
    }
    #[cfg(not(feature = "gpu-runtime"))]
    {
        let (mut app, _) =
            alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
        app.update();
    }
    assert_eq!(fs::read(&runtime_save_path).unwrap(), corrupt);
    assert_eq!(fs::read(&summary.save_path).unwrap(), source_before);
    PortableSaveFile::from_json_file(&summary.save_path)
        .unwrap()
        .validate_with_asset_root(&summary.asset_root)
        .unwrap();
}

#[test]
fn hearthling_renderer_preserves_stable_roots_without_generated_part_hierarchies() {
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
    let (mut app, summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();
    let save = PortableSaveFile::from_json_file(&summary.save_path).unwrap();
    let mut query = app.world_mut().query::<(
        &ProductionCreatureAssemblyRoot,
        &Fvr04ProductionCreatureVisualMarker,
        &Fvr10CreatureSpeciesMarker,
        &Visibility,
        &Transform,
    )>();
    let roots = query.iter(app.world()).collect::<Vec<_>>();
    assert_eq!(roots.len(), 30);
    assert_eq!(
        roots
            .iter()
            .map(|(r, _, _, _, _)| r.stable_id.raw())
            .collect::<BTreeSet<_>>()
            .len(),
        30
    );
    for (root, visual, species, visibility, pose) in roots {
        assert!(root.display_only);
        assert_eq!(*visibility, Visibility::Inherited);
        assert_eq!(root.stable_id, visual.stable_id);
        assert_eq!(root.organism_id, visual.organism_id);
        let object = save
            .world
            .objects
            .iter()
            .find(|o| o.id == root.stable_id)
            .unwrap();
        assert_eq!(
            (pose.translation.x, pose.translation.z),
            (object.position.x, object.position.z)
        );
        let saved = save
            .creatures
            .iter()
            .find(|c| c.organism_id == root.organism_id)
            .unwrap();
        assert_eq!(
            species.species_archetype,
            saved.appearance.species_archetype
        );
        assert_eq!(
            species.body_plan_signature,
            saved.appearance.body_plan_signature()
        );
        assert!(
            pose.translation.is_finite()
                && pose.scale.is_finite()
                && pose.scale.min_element() > 0.0
        );
    }
    let mut parts = app.world_mut().query::<&ProductionCreaturePartMarker>();
    assert_eq!(parts.iter(app.world()).count(), 0);
    let mut covers = app
        .world_mut()
        .query::<&ProductionCreatureJoinCoverMarker>();
    assert_eq!(covers.iter(app.world()).count(), 0);
    let scene = app
        .world()
        .resource::<alife_game_app::Fvr04ProductionCreatureSceneResource>();
    assert_eq!(scene.visual_profile, "approved-hearthling-skinned-v2");
    assert_eq!(scene.creature_root_count, 30);
    assert_eq!(scene.creature_part_entity_count, 0);
    assert_eq!(scene.creature_join_cover_count, 0);
    assert!(scene.production_visuals_display_only);
}

#[test]
fn fvr11_terrain_contract_is_display_only() {
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
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
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
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
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
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
        let (_fixture, launch) = production_launch(profile_id);
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
    assert!(minimum_sun_illuminance == 8_500.0);

    assert_eq!(comfort.tonemapping, "tony-mc-mapface");
    assert!(comfort.directional_shadows);
    assert_eq!(comfort.shadow_cascades, 1);
    assert!(comfort.distance_fog);
    assert!(comfort.cool_ambient_fill);
    assert!(comfort.contact_grounding);
    assert_eq!(comfort_contact_shadows, 0);
    assert!(comfort.display_only);
    assert!(comfort.no_renderer_authority_over_world_actions_or_cognition);
    assert!(comfort_ambient_brightness >= 360.0);
    assert!(comfort_vertical_area <= 17.5);
    assert!(comfort_sun_illuminance == 8_500.0);
}

#[test]
fn fvr03_voxel_app_spawns_real_persistent_chunks_by_default() {
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
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
    assert!(scene.uses_internal_voxel_terrain_mesh);
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
    if cfg!(feature = "vfx-hanabi") {
        assert_eq!(scene.production_vfx_marker_count, 0);
        assert!(scene.production_gpu_vfx_emitter_count > 0);
    } else {
        assert!(scene.production_vfx_marker_count >= 8);
        assert!(scene.production_vfx_marker_count <= 32);
        assert_eq!(scene.production_gpu_vfx_emitter_count, 0);
    }

    for required in [
        Fvr03ProductionVoxelMaterialKind::SafeGrass,
        Fvr03ProductionVoxelMaterialKind::Water,
        Fvr03ProductionVoxelMaterialKind::Resource,
        Fvr03ProductionVoxelMaterialKind::Hazard,
        Fvr03ProductionVoxelMaterialKind::Decay,
    ] {
        assert!(
            scene.material_counts.contains_key(&required),
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
    #[cfg(not(feature = "vfx-hanabi"))]
    {
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
    }

    let mut batch_query = app.world_mut().query::<&Fvr03ProductionVoxelTerrainBatch>();
    let batches = batch_query.iter(app.world()).copied().collect::<Vec<_>>();
    assert!(!batches.is_empty());
    assert!(
        batches.len()
            <= app
                .world()
                .resource::<Fvr11ProductionTerrainMaterialContract>()
                .material_count
    );
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
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
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
fn fvr04_live_projection_preserves_identity_and_rejects_unregistered_newborns() {
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
    let (mut app, summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.world_mut()
        .resource_mut::<Fvr05ProductionUxStateResource>()
        .settings
        .paused = true;
    app.update();
    let initial_roots = {
        let mut roots = app
            .world_mut()
            .query::<(&ProductionCreatureAssemblyRoot, &Transform)>();
        roots
            .iter(app.world())
            .map(|(r, t)| (r.stable_id, r.organism_id, t.translation))
            .collect::<Vec<_>>()
    };
    assert_eq!(initial_roots.len(), 30);
    let (stable_id, organism_id, initial_position) = initial_roots[0];
    let save = PortableSaveFile::from_json_file(&summary.save_path).unwrap();
    let mut object = save
        .world
        .objects
        .iter()
        .find(|o| o.id == stable_id)
        .unwrap()
        .clone();
    object.position.x += 2.25;
    object.position.z += 1.25; // Current worlds preserve continuous X/Z ground coordinates.
    object.position.y += 3.5; // Height must never be mistaken for the ground Z axis.
    object.body_yaw = 0.7;
    object.head_yaw = -0.2;
    let expected = production_ground_position(object.position);
    let frame =
        LiveBrainPresentationFrame::try_new(Vec::new(), Tick::new(8), vec![object.clone().into()])
            .unwrap();
    app.insert_resource(LiveBrainPresentationFrameResource::from_current_frame(
        frame,
    ));
    app.update();
    let projected = {
        let mut roots = app.world_mut().query::<(
            &ProductionCreatureAssemblyRoot,
            &Fvr04ProductionCreatureVisualMarker,
            &Transform,
        )>();
        let (root, visual, pose) = roots
            .iter(app.world())
            .find(|(r, _, _)| r.stable_id == stable_id)
            .unwrap();
        assert_eq!(root.organism_id, organism_id);
        assert_eq!(visual.organism_id, organism_id);
        assert_eq!(visual.body_yaw, object.body_yaw);
        assert_eq!(visual.head_yaw, object.head_yaw);
        assert_eq!((pose.translation.x, pose.translation.z), expected);
        assert_ne!(
            (pose.translation.x, pose.translation.z),
            (initial_position.x, initial_position.z)
        );
        pose.translation
    };
    let mut wrong_identity = object.clone();
    wrong_identity.organism_id = Some(OrganismId(organism_id.raw() + 20_000));
    wrong_identity.position.x += 4.0;
    let mut unregistered = object.clone();
    unregistered.id = WorldEntityId(stable_id.raw() + 10_000);
    unregistered.organism_id = Some(OrganismId(organism_id.raw() + 10_000));
    let frame = LiveBrainPresentationFrame::try_new(
        Vec::new(),
        Tick::new(9),
        vec![wrong_identity.into(), unregistered.clone().into()],
    )
    .unwrap();
    for _ in 0..2 {
        app.insert_resource(LiveBrainPresentationFrameResource::from_current_frame(
            frame.clone(),
        ));
        app.update();
        let mut roots = app
            .world_mut()
            .query::<(&ProductionCreatureAssemblyRoot, &Transform)>();
        assert_eq!(roots.iter(app.world()).count(), initial_roots.len());
        let roots = roots.iter(app.world()).collect::<Vec<_>>();
        assert_eq!(
            roots
                .iter()
                .map(|(r, _)| r.stable_id.raw())
                .collect::<BTreeSet<_>>()
                .len(),
            initial_roots.len()
        );
        assert!(!roots.iter().any(|(r, _)| r.stable_id == unregistered.id));
        let (root, pose) = roots
            .iter()
            .find(|(r, _)| r.stable_id == stable_id)
            .unwrap();
        assert_eq!(root.organism_id, organism_id);
        assert_eq!(pose.translation, projected);
    }
}

#[test]
#[cfg(feature = "gpu-runtime")]
fn curated_first_gpu_action_consumes_receipt_updates_registered_world_and_publishes_matching_live_frame(
) {
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
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
            .expect("receipt-bound creature must have a production root")
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
    let (expected_x, expected_z) = production_ground_position(current_object.position);
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
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
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
fn fvr04_selection_uses_current_root_position_without_retired_tile_ring() {
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
    let (mut app, summary) =
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
        .unwrap();
    assert_eq!(selected.kind, StableVoxelRefKind::Creature);
    let stable_id = selected.stable_id.unwrap();
    let initial = app
        .world()
        .resource::<Fvr03ProductionVoxelSceneResource>()
        .selection_position(stable_id)
        .unwrap();
    let save = PortableSaveFile::from_json_file(&summary.save_path).unwrap();
    let mut object = save
        .world
        .objects
        .iter()
        .find(|o| o.id == stable_id)
        .unwrap()
        .clone();
    object.position.x += 2.25;
    object.position.z += 1.25;
    let expected = production_ground_position(object.position);
    app.insert_resource(LiveBrainPresentationFrameResource::from_current_frame(
        LiveBrainPresentationFrame::try_new(Vec::new(), Tick::new(8), vec![object.into()]).unwrap(),
    ));
    app.update();
    assert_eq!(
        app.world()
            .resource::<Fvr03ProductionVoxelSelectionResource>()
            .selected,
        Some(selected)
    );
    let position = app
        .world()
        .resource::<Fvr03ProductionVoxelSceneResource>()
        .selection_position(stable_id)
        .unwrap();
    assert_eq!((position.x, position.z), expected);
    assert_ne!((position.x, position.z), (initial.x, initial.z));
    let mut roots = app
        .world_mut()
        .query::<(&ProductionCreatureAssemblyRoot, &Transform)>();
    let pose = roots
        .iter(app.world())
        .find(|(r, _)| r.stable_id == stable_id)
        .unwrap()
        .1;
    assert_eq!(position, pose.translation);
    let mut rings = app
        .world_mut()
        .query::<(&Fvr03ProductionVoxelSelectionMarker, &Visibility)>();
    assert!(rings
        .iter(app.world())
        .all(|(_, v)| *v == Visibility::Hidden));
}

#[test]
fn fvr04_camera_follow_preserves_framing_after_continuous_live_projection() {
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
    let (mut app, summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.world_mut()
        .resource_mut::<Fvr05ProductionUxStateResource>()
        .settings
        .paused = true;
    app.update();
    let stable_id = app
        .world()
        .resource::<Fvr03ProductionVoxelSelectionResource>()
        .selected
        .unwrap()
        .stable_id
        .unwrap();
    {
        let mut follow = app
            .world_mut()
            .resource_mut::<Fvr04ProductionCreatureFollowResource>();
        follow.enabled = true;
        follow.target_stable_id = Some(stable_id);
    }
    app.update();
    let camera_pose = |app: &mut bevy::prelude::App| {
        let mut cameras = app
            .world_mut()
            .query::<(&Fvr03ProductionVoxelCamera, &Transform)>();
        *cameras.iter(app.world()).next().unwrap().1
    };
    let root_pose = |app: &mut bevy::prelude::App| {
        let mut roots = app
            .world_mut()
            .query::<(&ProductionCreatureAssemblyRoot, &Transform)>();
        roots
            .iter(app.world())
            .find(|(r, _)| r.stable_id == stable_id)
            .unwrap()
            .1
            .translation
    };
    let before_camera = camera_pose(&mut app);
    let before_root = root_pose(&mut app);
    let save = PortableSaveFile::from_json_file(&summary.save_path).unwrap();
    let mut object = save
        .world
        .objects
        .iter()
        .find(|o| o.id == stable_id)
        .unwrap()
        .clone();
    object.position.x += 2.25;
    object.position.z += 1.25;
    let expected = production_ground_position(object.position);
    app.insert_resource(LiveBrainPresentationFrameResource::from_current_frame(
        LiveBrainPresentationFrame::try_new(Vec::new(), Tick::new(8), vec![object.into()]).unwrap(),
    ));
    app.update();
    let after_root = root_pose(&mut app);
    let after_camera = camera_pose(&mut app);
    assert_eq!((after_root.x, after_root.z), expected);
    assert!(
        ((after_camera.translation - before_camera.translation) - (after_root - before_root))
            .length()
            < 1e-5
    );
    let focus = after_root + Vec3::Y * 0.70;
    assert!(
        (after_camera.rotation * Vec3::NEG_Z).dot((focus - after_camera.translation).normalize())
            > 0.99999
    );
    assert!(after_camera.rotation.dot(before_camera.rotation).abs() > 0.99999);
}

#[test]
fn fvr04_missing_selected_creature_root_hides_marker_without_stale_scene_coordinates() {
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
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
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
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
    assert_eq!(label_translation.0.y, root_translation.y + 1.55);
    assert_ne!(
        (label_translation.0.x, label_translation.0.z),
        (initial_label_translation.x, initial_label_translation.z),
        "the label must update when the live root moves without changing selection"
    );
}

#[test]
fn fvr04_live_world_label_hides_when_selected_creature_root_missing() {
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
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
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
    let (mut app, launch_summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    {
        let mut ux = app
            .world_mut()
            .resource_mut::<Fvr05ProductionUxStateResource>();
        ux.settings.paused = true;
        ux.debug_mode = true;
        ux.settings.show_menu = true;
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
    {
        let text = &fvr05_text;
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
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
    let (mut app, launch_summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    {
        let mut ux = app
            .world_mut()
            .resource_mut::<Fvr05ProductionUxStateResource>();
        ux.settings.paused = true;
        ux.debug_mode = true;
        ux.settings.show_menu = true;
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
    {
        let text = &fvr05_text;
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
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
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
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
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
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
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
    let panel_text = panels[0].1 .0.clone();
    for real_state_heading in [
        "Hunger",
        "Energy",
        "Tiredness",
        "Safety",
        "Sleepiness",
        "Praise",
    ] {
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
    assert!(controls[0].1 .0.contains("F1 Help"));
    assert!(controls[0].1 .0.contains("Space Pause"));
    assert!(panel_text.contains("Energy  unavailable"));
    assert!(!panel_text.contains("LEARNING") && !panel_text.contains("SOCIAL"));
}

#[test]
fn v0_recovery_key_restores_the_clean_isometric_player_view() {
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
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
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();

    let layers = {
        let mut query = app.world_mut().query::<(
            &Fvr11ProductionTerrainLayer,
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
        )>();
        query
            .iter(app.world())
            .filter(|(layer, _, _)| layer.role == Fvr11TerrainSurfaceRole::Top)
            .map(|(layer, mesh, material)| (layer.material, mesh.0.clone(), material.0.clone()))
            .collect::<Vec<_>>()
    };
    let meshes = app.world().resource::<Assets<Mesh>>();
    let materials = app.world().resource::<Assets<StandardMaterial>>();
    let mean_color = |kind| {
        let mut sum = [0.0; 3];
        let mut count = 0;
        for (_, mesh, material) in layers.iter().filter(|(material, _, _)| *material == kind) {
            let material = materials.get(material).unwrap();
            assert!(material
                .base_color
                .to_srgba()
                .to_f32_array()
                .iter()
                .all(|v| (*v - 1.0).abs() < 1e-6));
            assert!(!material.unlit);
            let Some(VertexAttributeValues::Float32x4(colors)) =
                meshes.get(mesh).unwrap().attribute(Mesh::ATTRIBUTE_COLOR)
            else {
                panic!("terrain vertex colors missing")
            };
            for color in colors {
                for axis in 0..3 {
                    sum[axis] += color[axis];
                }
                count += 1;
            }
        }
        assert!(count >= 24);
        sum.map(|value| value / count as f32)
    };
    let grass = mean_color(Fvr03ProductionVoxelMaterialKind::SafeGrass);
    let soil = mean_color(Fvr03ProductionVoxelMaterialKind::Soil);
    assert!(grass[1] > grass[0] * 1.05 && grass[1] > grass[2] * 1.12);
    assert!(soil[0] > soil[1] * 1.10 && soil[1] > soil[2] * 1.08);

    let (ambient, vertical_area) = {
        let mut query = app.world_mut().query::<(&AmbientLight, &Projection)>();
        let (ambient, projection) = query.iter(app.world()).next().expect("player camera");
        let Projection::Orthographic(orthographic) = projection else {
            panic!("player camera must stay orthographic");
        };
        (ambient.clone(), orthographic.area.height())
    };
    let ambient_color = ambient.color.to_srgba();
    assert!(ambient_color.blue > ambient_color.red);
    assert!(ambient.brightness >= 700.0);
    assert!(vertical_area <= 16.0);
    let mut sun_query = app.world_mut().query::<&DirectionalLight>();
    let sun = sun_query.iter(app.world()).next().expect("warm sun");
    assert!(sun.shadows_enabled);
    assert!(sun.illuminance == 8_500.0);

    let mut roots = app.world_mut().query::<(
        &ProductionCreatureAssemblyRoot,
        &Fvr04ProductionCreatureVisualMarker,
    )>();
    assert!(roots
        .iter(app.world())
        .all(|(root, creature)| root.display_only
            && creature.base_scale.is_finite()
            && creature.base_scale.min_element() > 0.0));
    shipping_hearthling();
}

#[test]
fn fvr09_layered_grid_mesher_records_visible_face_reduction() {
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinimumSettings30x30);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();

    let scene = app
        .world()
        .resource::<Fvr03ProductionVoxelSceneResource>()
        .clone();

    assert_eq!(scene.mesh_stats.mode, Fvr09MesherMode::LayeredGridQuads);
    assert_eq!(
        scene.mesh_stats.material_palette_version,
        "fvr10-visible-surface-variation-v1"
    );
    assert!(scene.mesh_stats.vertex_color_face_variation);
    assert!(scene.mesh_stats.top_side_color_separation);
    assert!(scene.mesh_stats.variation_bucket_count >= 4);
    assert!(scene.mesh_stats.visible_voxels >= scene.tile_mesh_count);
    assert!(scene.mesh_stats.naive_visible_faces > scene.mesh_stats.emitted_quads);
    assert!(scene.mesh_stats.face_reduction_ratio >= 1.20);
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
    assert!(!settings.debug_primary_colors);

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
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
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
        "approved-hearthling-skinned-v2"
    );
    assert_eq!(
        creature_scene.mesh_material_version,
        "approved-blender-vertex-color-v2"
    );
    assert_eq!(
        creature_scene.rendered_creature_count,
        scene.creature_render_count
    );
    assert_eq!(creature_scene.mesh_pool_count, 0); // Dry-run roots load no mesh assets.
    shipping_hearthling();
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
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
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
fn approved_hearthling_asset_retains_low_poly_skin_and_animation_contract() {
    shipping_hearthling();
}

#[test]
fn hearthling_species_markers_preserve_inherited_body_plans_with_a_shared_skin() {
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
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
        "approved-blender-vertex-color-v2"
    );
    assert_eq!(creature_scene.mesh_pool_count, 0);
    assert_eq!(creature_scene.material_bucket_count, 0);
    shipping_hearthling();

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
fn hearthling_roots_use_no_retired_parts_face_overlays_or_surface_patches() {
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();

    let root_count = {
        let mut query = app.world_mut().query::<&ProductionCreatureAssemblyRoot>();
        query.iter(app.world()).count()
    };
    let part_count = {
        let mut query = app.world_mut().query::<&ProductionCreaturePartMarker>();
        query.iter(app.world()).count()
    };
    let join_cover_count = {
        let mut query = app
            .world_mut()
            .query::<&ProductionCreatureJoinCoverMarker>();
        query.iter(app.world()).count()
    };
    let face_overlay_count = {
        let mut query = app.world_mut().query::<&Fvr09CreatureFaceFeatureMarker>();
        query.iter(app.world()).count()
    };
    let surface_patch_count = {
        let mut query = app.world_mut().query::<&Fvr10CreatureSurfaceDetailMarker>();
        query.iter(app.world()).count()
    };

    assert!(root_count > 0);
    assert_eq!(part_count, 0);
    shipping_hearthling();
    assert_eq!(join_cover_count, 0);
    assert_eq!(face_overlay_count, 0);
    assert_eq!(surface_patch_count, 0);

    let scene = app
        .world()
        .resource::<alife_game_app::Fvr04ProductionCreatureSceneResource>();
    assert_eq!(scene.creature_part_entity_count, part_count);
    assert_eq!(scene.creature_join_cover_count, 0);
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
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
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
fn fvr10_product_camera_and_authored_heads_are_composed_for_readable_creatures() {
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
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

    shipping_hearthling();
    let mut legacy_heads = app.world_mut().query::<&ProductionCreaturePartMarker>();
    assert_eq!(legacy_heads.iter(app.world()).count(), 0);
}

#[test]
fn hearthling_skin_primitives_preserve_authored_material_and_vertex_color_bindings() {
    let doc = shipping_hearthling();
    let primitives = doc["meshes"][0]["primitives"].as_array().unwrap();
    assert_eq!(
        primitives
            .iter()
            .map(|p| p["material"].as_u64().unwrap())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([0, 1, 2, 3, 4, 5])
    );
    for material in doc["materials"].as_array().unwrap() {
        assert!(material["name"]
            .as_str()
            .unwrap()
            .starts_with("Hearthling |"));
        assert!(material["pbrMetallicRoughness"]["baseColorTexture"].is_null());
        assert_eq!(
            material["pbrMetallicRoughness"]["metallicFactor"]
                .as_f64()
                .unwrap_or(1.0),
            0.0
        );
        let roughness = material["pbrMetallicRoughness"]["roughnessFactor"]
            .as_f64()
            .unwrap_or(1.0);
        assert!(roughness > 0.0 && roughness <= 1.0);
    }
    assert!(doc["materials"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["normalTexture"].is_object()));
}

#[test]
fn hearthling_face_is_authored_in_the_skin_and_renderer_remains_display_only() {
    shipping_hearthling();
    let (_fixture, launch) = production_launch(ProductionFrontendProfileId::MinSpecComfort1080p);
    let (mut app, _) =
        alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell(&launch).unwrap();
    app.update();
    let mut overlay = app.world_mut().query::<&Fvr09CreatureFaceFeatureMarker>();
    assert_eq!(overlay.iter(app.world()).count(), 0);
    let scene = app
        .world()
        .resource::<alife_game_app::Fvr04ProductionCreatureSceneResource>();
    assert!(scene.production_visuals_display_only);
    assert!(scene.no_renderer_authority_over_actions_or_cognition);
    assert!(scene.expression_buffer_is_read_only_projection);
}
