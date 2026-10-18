//! CPU/ECS coverage of live sleep truth, identity, display following and cleanup.
use super::*;
use alife_core::{BrainCapacityClass, CreatureGenome, FoundationGeneticIdentity, SleepPhase};
use alife_world::{HeadlessScenarioBuilder, HeadlessWorld, WorldOrganismRecord};
use std::time::Duration;

fn authoritative_world(count: usize, phase: SleepPhase) -> HeadlessWorld {
    let mut builder = HeadlessScenarioBuilder::new(21);
    for index in 0..count {
        builder = builder.agent(
            &format!("agent{index}"),
            OrganismId(index as u64 + 1),
            Vec3f::new(index as f32 * 3.0, 0.0, 0.0),
        );
    }
    let mut world = builder.build().unwrap();
    for index in 0..count {
        let id = OrganismId(index as u64 + 1);
        let entity = world.entity_id(&format!("agent{index}")).unwrap();
        let genome = CreatureGenome::early_mammal_founder(
            21,
            FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
        )
        .unwrap();
        let phenotype = genome.express().unwrap();
        let mut chemistry = alife_core::BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
        chemistry.homeostasis.hormones.sleep_pressure = 0.99;
        let mut record =
            WorldOrganismRecord::new(id, entity, genome, phenotype, chemistry, Tick::ZERO).unwrap();
        record.seal_sleep_phase(phase, 1, Tick::ZERO, 0).unwrap();
        world.register_organism_record(record).unwrap();
    }
    world
}

fn root_and_visual(
    row: &WorldOrganismPresentationRow,
) -> (
    ProductionCreatureAssemblyRoot,
    Fvr04ProductionCreatureVisualMarker,
) {
    (
        ProductionCreatureAssemblyRoot {
            stable_id: row.world_entity_id,
            organism_id: row.organism_id,
            display_only: true,
        },
        Fvr04ProductionCreatureVisualMarker {
            stable_id: row.world_entity_id,
            organism_id: row.organism_id,
            tile: VoxelTileCoord::new(0, 0),
            expression: CreatureExpressionState::Tired,
            // Awake Rest and Waking also use this pose: it cannot establish sleep.
            animation: CreatureAnimationState::Resting,
            lod: Fvr04CreatureLod::CompactVoxel,
            base_translation: Vec3::ZERO,
            local_offset: Vec3::ZERO,
            base_scale: Vec3::ONE,
            local_bounds: hearthling::LOCAL_BOUNDS,
            surface_height: 0.0,
            body_yaw: 0.0,
            head_yaw: 0.0,
            phase: 0.0,
        },
    )
}

fn spawn_root(app: &mut App, row: &WorldOrganismPresentationRow, pose: Transform) -> Entity {
    let (root, visual) = root_and_visual(row);
    app.world_mut()
        .spawn((root, visual, pose, Fvr04ProductionRuntimeSceneRoot))
        .id()
}

fn fixture(world: &HeadlessWorld) -> App {
    let mut app = App::new();
    app.insert_resource(Time::<()>::default())
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<StandardMaterial>>()
        .insert_resource(
            LiveBrainPresentationFrameResource::from_authoritative_world(world).unwrap(),
        )
        .add_systems(Update, sync);
    app
}

fn advance(app: &mut App, millis: u64) {
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(millis));
    app.update();
}

fn cue_entities(app: &mut App) -> Vec<Entity> {
    app.world_mut()
        .query_filtered::<Entity, With<SleepCue>>()
        .iter(app.world())
        .collect()
}

fn ring_handles(app: &mut App) -> Vec<(Handle<Mesh>, Handle<StandardMaterial>)> {
    app.world_mut()
        .query::<(&Mesh3d, &MeshMaterial3d<StandardMaterial>)>()
        .iter(app.world())
        .map(|(mesh, material)| (mesh.0.clone(), material.0.clone()))
        .collect()
}

fn reseed(app: &mut App, world: &HeadlessWorld) {
    app.world_mut()
        .resource_mut::<LiveBrainPresentationFrameResource>()
        .reseed_from_loaded_world(world)
        .unwrap();
}

#[test]
fn only_authoritative_sleep_phases_mark_an_individual_sleeping() {
    for (phase, expected) in [
        (SleepPhase::Awake, false),
        (SleepPhase::EnteringSleep, true),
        (SleepPhase::Consolidating, true),
        (SleepPhase::Waking, false),
        (SleepPhase::ForcedRecoverySleep, true),
    ] {
        let world = authoritative_world(1, phase);
        let frame =
            LiveBrainPresentationFrame::from_authoritative_world(Vec::new(), &world).unwrap();
        let row = frame.organism_snapshots().remove(0);
        let (root, mut visual) = root_and_visual(&row);
        assert!(row.biochemistry.homeostasis.hormones.sleep_pressure >= 0.9);
        for animation in [
            CreatureAnimationState::Resting,
            CreatureAnimationState::Sleeping,
        ] {
            visual.animation = animation;
            assert_eq!(
                sleeping_individual(&frame, &root, &visual),
                expected,
                "{phase:?}, {animation:?}"
            );
        }
    }
}

#[test]
fn missing_authority_and_every_identity_or_lifecycle_mismatch_fail_closed() {
    let world = authoritative_world(1, SleepPhase::Consolidating);
    let row = world.presentation_snapshot().organisms.remove(0);
    let (root, visual) = root_and_visual(&row);
    assert!(sleeping_row(&root, &visual, Some(&row.object), Some(&row)));
    assert!(!sleeping_row(&root, &visual, None, Some(&row)));
    assert!(!sleeping_row(&root, &visual, Some(&row.object), None));

    let objects_without_rows =
        LiveBrainPresentationFrame::try_new(Vec::new(), Tick::ZERO, world.object_snapshots())
            .unwrap();
    assert!(!sleeping_individual(&objects_without_rows, &root, &visual));
    let empty = LiveBrainPresentationFrame::try_new(Vec::new(), Tick::ZERO, Vec::new()).unwrap();
    assert!(!sleeping_individual(&empty, &root, &visual));

    let mut bad_rows = Vec::new();
    let mut changed = row.clone();
    changed.world_entity_id = WorldEntityId(root.stable_id.raw() + 1);
    bad_rows.push(changed);
    let mut changed = row.clone();
    changed.organism_id = OrganismId(root.organism_id.raw() + 1);
    bad_rows.push(changed);
    let mut changed = row.clone();
    changed.object.id = WorldEntityId(root.stable_id.raw() + 1);
    bad_rows.push(changed);
    let mut changed = row.clone();
    changed.object.kind = WorldObjectKind::Food;
    bad_rows.push(changed);
    let mut changed = row.clone();
    changed.object.organism_id = None;
    bad_rows.push(changed);
    let mut changed = row.clone();
    changed.object.organism_id = Some(OrganismId(root.organism_id.raw() + 1));
    bad_rows.push(changed);
    let mut changed = row.clone();
    changed.lifecycle = alife_world::OrganismLifecycle::Dead {
        death_tick: Tick::ZERO,
    };
    bad_rows.push(changed);
    for changed in bad_rows {
        assert!(!sleeping_row(
            &root,
            &visual,
            Some(&row.object),
            Some(&changed)
        ));
    }
    let mut object = row.object.clone();
    object.id = WorldEntityId(root.stable_id.raw() + 1);
    assert!(!sleeping_row(&root, &visual, Some(&object), Some(&row)));
    object = row.object.clone();
    object.kind = WorldObjectKind::Food;
    assert!(!sleeping_row(&root, &visual, Some(&object), Some(&row)));
    object = row.object.clone();
    object.organism_id = None;
    assert!(!sleeping_row(&root, &visual, Some(&object), Some(&row)));
    object = row.object.clone();
    object.organism_id = Some(OrganismId(root.organism_id.raw() + 1));
    assert!(!sleeping_row(&root, &visual, Some(&object), Some(&row)));
    let mut bad_visual = visual;
    bad_visual.stable_id = WorldEntityId(root.stable_id.raw() + 1);
    assert!(!sleeping_row(
        &root,
        &bad_visual,
        Some(&row.object),
        Some(&row)
    ));
    bad_visual = visual;
    bad_visual.organism_id = OrganismId(root.organism_id.raw() + 1);
    assert!(!sleeping_row(
        &root,
        &bad_visual,
        Some(&row.object),
        Some(&row)
    ));
    let wrong_root = ProductionCreatureAssemblyRoot {
        organism_id: OrganismId(2),
        ..root
    };
    assert!(!sleeping_row(
        &wrong_root,
        &visual,
        Some(&row.object),
        Some(&row)
    ));
}

#[test]
fn cue_tracks_displayed_pose_and_camera_between_ticks_and_freezes_pulse_on_pause() {
    let world = authoritative_world(1, SleepPhase::Consolidating);
    let row = world.presentation_snapshot().organisms.remove(0);
    let mut app = fixture(&world);
    app.insert_resource(ux());
    let root = spawn_root(&mut app, &row, Transform::from_xyz(4.0, 7.0, -2.0));
    let camera = app
        .world_mut()
        .spawn((
            Fvr03ProductionVoxelCamera {
                mode: Fvr03ProductionVoxelCameraMode::OrthographicIsometric,
            },
            Transform::default(),
        ))
        .id();
    advance(&mut app, 0);
    let cue = cue_entities(&mut app)[0];
    assert_eq!(app.world().get::<ChildOf>(cue).unwrap().parent(), root);
    assert_eq!(app.world().get::<Children>(cue).unwrap().len(), 3);
    let frame = app
        .world()
        .resource::<LiveBrainPresentationFrameResource>()
        .clone();

    let displayed = Transform::from_xyz(8.0, 12.0, 3.0)
        .with_rotation(Quat::from_rotation_y(0.7))
        .with_scale(Vec3::new(1.2, 0.8, 1.1));
    *app.world_mut().get_mut::<Transform>(root).unwrap() = displayed;
    let camera_rotation = Quat::from_rotation_y(-0.6) * Quat::from_rotation_x(-0.3);
    app.world_mut()
        .get_mut::<Transform>(camera)
        .unwrap()
        .rotation = camera_rotation;
    advance(&mut app, 250);
    assert_eq!(
        cue_entities(&mut app),
        vec![cue],
        "unchanged live frames reuse the cue"
    );
    let seconds = app.world().get::<SleepCue>(cue).unwrap().seconds;
    assert!((seconds - 0.25).abs() < 1e-6);
    let local = *app.world().get::<Transform>(cue).unwrap();
    let composed = displayed.mul_transform(local);
    assert!(composed.rotation.abs_diff_eq(camera_rotation, 1e-6));
    assert!(
        composed.translation.y > displayed.translation.y + 1.5,
        "height follows the displayed scaled pose, not a fixed world height"
    );
    let xz_offset = Vec2::new(
        composed.translation.x - displayed.translation.x,
        composed.translation.z - displayed.translation.z,
    );
    assert!(xz_offset.length() < 0.4);
    assert_eq!(
        app.world().resource::<LiveBrainPresentationFrameResource>(),
        &frame,
        "presentation never changes authoritative state"
    );

    app.world_mut()
        .resource_mut::<Fvr05ProductionUxStateResource>()
        .settings
        .paused = true;
    advance(&mut app, 2_000);
    assert_eq!(app.world().get::<SleepCue>(cue).unwrap().seconds, seconds);
    assert_eq!(*app.world().get::<Transform>(cue).unwrap(), local);
    app.world_mut()
        .get_mut::<Transform>(root)
        .unwrap()
        .translation += Vec3::new(2.0, 5.0, -3.0);
    advance(&mut app, 2_000);
    assert_eq!(app.world().get::<SleepCue>(cue).unwrap().seconds, seconds);
    let moved_root = *app.world().get::<Transform>(root).unwrap();
    let moved = moved_root.mul_transform(*app.world().get::<Transform>(cue).unwrap());
    assert!((moved.translation - composed.translation - Vec3::new(2.0, 5.0, -3.0)).length() < 1e-5);
    app.world_mut()
        .resource_mut::<Fvr05ProductionUxStateResource>()
        .settings
        .paused = false;
    app.world_mut()
        .resource_mut::<Fvr05ProductionUxStateResource>()
        .animation_speed = 2.0;
    advance(&mut app, 250);
    assert!((app.world().get::<SleepCue>(cue).unwrap().seconds - seconds - 0.5).abs() < 1e-6);
}

#[test]
fn wake_missing_frame_identity_changes_and_root_removal_clear_the_whole_cue() {
    let sleeping = authoritative_world(1, SleepPhase::Consolidating);
    let awake = authoritative_world(1, SleepPhase::Awake);
    let row = sleeping.presentation_snapshot().organisms.remove(0);
    let mut app = fixture(&sleeping);
    let root = spawn_root(&mut app, &row, Transform::default());
    advance(&mut app, 0);
    assert_eq!(ring_handles(&mut app).len(), 3);
    reseed(&mut app, &awake);
    advance(&mut app, 0);
    assert!(cue_entities(&mut app).is_empty());
    assert!(ring_handles(&mut app).is_empty());
    reseed(&mut app, &sleeping);
    advance(&mut app, 0);
    assert_eq!(cue_entities(&mut app).len(), 1);
    app.world_mut()
        .remove_resource::<LiveBrainPresentationFrameResource>();
    advance(&mut app, 0);
    assert!(ring_handles(&mut app).is_empty());
    app.insert_resource(
        LiveBrainPresentationFrameResource::from_authoritative_world(&sleeping).unwrap(),
    );
    advance(&mut app, 0);
    app.world_mut()
        .get_mut::<ProductionCreatureAssemblyRoot>(root)
        .unwrap()
        .organism_id = OrganismId(2);
    advance(&mut app, 0);
    assert!(cue_entities(&mut app).is_empty());
    app.world_mut()
        .get_mut::<ProductionCreatureAssemblyRoot>(root)
        .unwrap()
        .organism_id = row.organism_id;
    advance(&mut app, 0);
    assert_eq!(cue_entities(&mut app).len(), 1);
    app.world_mut().entity_mut(root).despawn();
    advance(&mut app, 0);
    assert!(cue_entities(&mut app).is_empty());
    assert!(ring_handles(&mut app).is_empty());
}

#[test]
fn live_death_and_authoritative_retirement_clear_a_sleeping_cue() {
    let mut world = authoritative_world(1, SleepPhase::Consolidating);
    let row = world.presentation_snapshot().organisms.remove(0);
    let mut app = fixture(&world);
    spawn_root(&mut app, &row, Transform::default());
    advance(&mut app, 0);
    assert_eq!(cue_entities(&mut app).len(), 1);
    let mut record = world
        .organism_registry()
        .get(row.organism_id)
        .unwrap()
        .clone();
    record.mark_dead(Tick::ZERO).unwrap();
    world.replace_organism_record_exact(record).unwrap();
    reseed(&mut app, &world);
    advance(&mut app, 0);
    assert!(cue_entities(&mut app).is_empty());
    world.retire_dead_organism(row.organism_id).unwrap();
    reseed(&mut app, &world);
    advance(&mut app, 0);
    assert!(ring_handles(&mut app).is_empty());
}

#[test]
fn stable_identity_selects_a_bounded_set_and_transitions_reuse_shared_assets() {
    let sleeping = authoritative_world(MAX_CUES + 3, SleepPhase::Consolidating);
    let awake = authoritative_world(MAX_CUES + 3, SleepPhase::Awake);
    let mut rows = sleeping.presentation_snapshot().organisms;
    rows.sort_by_key(|row| row.world_entity_id);
    let expected = rows
        .iter()
        .take(MAX_CUES)
        .map(|row| row.world_entity_id)
        .collect::<BTreeSet<_>>();
    let mut app = fixture(&sleeping);
    // Reverse ECS insertion order to ensure capacity never selects an arbitrary individual.
    for row in rows.iter().rev() {
        spawn_root(&mut app, row, Transform::default());
    }
    advance(&mut app, 0);
    let marked = app
        .world_mut()
        .query::<&SleepCue>()
        .iter(app.world())
        .map(|cue| cue.stable_id)
        .collect::<BTreeSet<_>>();
    assert_eq!(marked, expected);
    let handles = ring_handles(&mut app);
    assert_eq!(handles.len(), MAX_CUES * 3);
    let shared = handles[0].clone();
    assert!(handles.iter().all(|handles| *handles == shared));
    for _ in 0..8 {
        reseed(&mut app, &awake);
        advance(&mut app, 0);
        assert!(ring_handles(&mut app).is_empty());
        reseed(&mut app, &sleeping);
        advance(&mut app, 0);
        assert_eq!(cue_entities(&mut app).len(), MAX_CUES);
        assert!(ring_handles(&mut app)
            .iter()
            .all(|handles| *handles == shared));
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 1);
        assert_eq!(app.world().resource::<Assets<StandardMaterial>>().len(), 1);
    }
}

#[cfg(feature = "gpu-runtime")]
#[test]
fn runtime_scene_load_cleanup_removes_cues_and_new_scene_reads_loaded_sleep_state() {
    let sleeping = authoritative_world(1, SleepPhase::Consolidating);
    let awake = authoritative_world(1, SleepPhase::Awake);
    let row = sleeping.presentation_snapshot().organisms.remove(0);
    let mut app = fixture(&sleeping);
    let old_root = spawn_root(&mut app, &row, Transform::default());
    advance(&mut app, 0);
    let old_cue = cue_entities(&mut app)[0];
    despawn_fvr04_runtime_scene(app.world_mut());
    assert!(app.world().get_entity(old_root).is_err());
    assert!(app.world().get_entity(old_cue).is_err());
    assert!(ring_handles(&mut app).is_empty());
    reseed(&mut app, &awake);
    let new_root = spawn_root(&mut app, &row, Transform::from_xyz(20.0, 10.0, 30.0));
    advance(&mut app, 0);
    assert!(
        cue_entities(&mut app).is_empty(),
        "same IDs in an awake loaded world cannot inherit sleep"
    );
    reseed(&mut app, &sleeping);
    advance(&mut app, 0);
    let cue = cue_entities(&mut app)[0];
    assert_eq!(app.world().get::<ChildOf>(cue).unwrap().parent(), new_root);
    assert_ne!(cue, old_cue);
    assert_eq!(app.world().get::<SleepCue>(cue).unwrap().seconds, 0.0);
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 1);
    assert_eq!(app.world().resource::<Assets<StandardMaterial>>().len(), 1);
}

// CPU-only playback fixture; no renderer, neural session, or simulation ticking.
fn ux() -> Fvr05ProductionUxStateResource {
    let profile = ProductionFrontendProfileId::MinimumSettings30x30;
    Fvr05ProductionUxStateResource {
        settings: Fvr05ProductionUxSettings {
            schema: "alife.fvr05.production_ux.v1".into(), schema_version: 1,
            selected_profile: profile, preferred_profile_for_next_launch: profile,
            active_inspector_tab: Fvr05ProductionInspectorTab::Creature, enabled_overlays: vec![],
            camera_mode: "orbit".into(), paused: false, run_mode: None, simulation_speed: 1.0,
            follow_selection: false, show_menu: false, show_settings: false, show_overlays: false,
            pause_on_focus_loss: true, selected_stable_id: None, source_save_path: String::new(),
            runtime_save_path: String::new(), created_world_save_path: String::new(),
            asset_manifest_path: String::new(), backend_descriptor: "CPU sleep fixture".into(),
            validation_receipt: String::new(),
        },
        animation_speed: 1.0, debug_mode: false, show_help: false, ui_settings_path: PathBuf::new(),
        source_save_path: PathBuf::new(), asset_root: PathBuf::new(), profile_id: profile,
        profile_budget: profile.budget(), population: 0, resolution: (1280, 720),
        save_metadata: ProductionSaveMetadata {
            save_id: "CPU sleep fixture".into(), deterministic_seed: 21, selected_profile: profile.label().into(),
            profile_budget_version: 1, save_schema: String::new(), save_schema_version: 1,
            config_schema: String::new(), config_schema_version: 1, object_count: 0, creature_count: 0,
            asset_count: 0, voxel_backend_schema: None, voxel_visible_chunk_signatures: 0,
            voxel_materialized_chunks: 0, voxel_resource_hazard_refs: 0, voxel_stable_selection_refs: 0,
            voxel_dirty_region_count: 0, voxel_roundtrip_signatures_match: false,
            no_renderer_tokens_in_voxel_save: true, gpu_runtime_schema: None,
            gpu_runtime_selected_backend: "Unavailable".into(),
            gpu_runtime_unavailable_reason: Some("CPU sleep fixture".into()),
            gpu_runtime_no_active_bulk_readback: true, gpu_runtime_compact_action_readback_bytes: 0,
            gpu_runtime_checkpoint_label: String::new(),
        },
        selected_backend: "Unavailable".into(), adapter_name: String::new(), backend_api: String::new(),
        graphics_backend: String::new(), unavailable_reason: "CPU sleep fixture".into(),
        renderer_profile: String::new(), state_trace: String::new(),
        authority: Fvr05ProductionDebugAuthorityReport::production_read_only(),
        gpu_runtime_state: serde_json::from_value(serde_json::json!({
            "schema":"CPU sleep fixture","schema_version":1,"requested_backend_mode":"GpuAuthoritative",
            "selected_backend_mode":"Unavailable","adapter_identity":{"adapter_name":null,
                "backend_api":null,"adapter_type":null,"driver":null,"driver_info":null},
            "validation_profile":"CPU sleep fixture","brain_residency_slots":{"hot_slots":0,
                "warm_slots":0,"cold_slots":0},"class_bucket_allocations":[],
            "active_profile_caps":{"target_fps":0,"target_frame_ms":0.0,"renderer_reserve_ms":0.0,
                "gpu_neural_budget_ms":0.0,"neural_heap_mb":0,"staging_readback_budget_kib":0,
                "chunk_activation_radius":0,"active_chunk_cap":0,"vfx_budget":"none",
                "adaptive_throttling_order":[]},
            "shader_abi_versions":{"shader_manifest":[],"abi_manifest":[]},
            "authority":{"authoritative":false,"failure_stops_learned_actions":true,"finite_rejections":0},
            "last_safe_checkpoint":{"save_id":"CPU sleep fixture","world_tick":0,
                "sealed_patch_boundary":true,"checkpoint_label":"CPU sleep fixture"},
            "unavailable_reason":"CPU sleep fixture","selected_scale_profile":"CPU sleep fixture",
            "compact_action_readback_bytes_per_creature":0,"no_active_bulk_readback":true
        })).unwrap(),
        last_action: String::new(), last_error: None, pending_food_move: None,
        #[cfg(feature = "gpu-runtime")]
        last_manual_checkpoint_status: None,
    }
}
