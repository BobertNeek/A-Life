//! CPU/ECS coverage of live care truth, identity, display following and cleanup.
//!
//! The word and the nod both fire from the world-owned receipt and only
//! from the receipt. A creature with no receipt shows nothing — unknown is
//! never rendered as a "nothing felt" bubble.
use super::*;
use alife_core::{
    BrainCapacityClass, CreatureGenome, FoundationGeneticIdentity, FoundationWeightAsset,
    SensorProfile, BASIC_VOCABULARY_V1,
};
use alife_world::{
    create_canonical_new_game, CanonicalNewGameConfig, HeadlessScenarioBuilder, HeadlessWorld,
    OrganismLifecycle, WorldOrganismRecord,
};
use std::time::Duration;

fn cared_game() -> HeadlessWorld {
    cared_population(1)
}

fn cared_population(population: u16) -> HeadlessWorld {
    let foundation =
        FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    create_canonical_new_game(
        &CanonicalNewGameConfig::phase3(240_824, population).unwrap(),
        &foundation,
    )
    .unwrap()
    .world
}

fn quiet_world() -> HeadlessWorld {
    let mut world = HeadlessScenarioBuilder::new(21)
        .agent("agent", OrganismId(1), Vec3f::ZERO)
        .build()
        .unwrap();
    let entity = world.entity_id("agent").unwrap();
    let genome = CreatureGenome::early_mammal_founder(
        21,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    let phenotype = genome.express().unwrap();
    let chemistry = alife_core::BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
    let record = WorldOrganismRecord::new(
        OrganismId(1),
        entity,
        genome,
        phenotype,
        chemistry,
        Tick::ZERO,
    )
    .unwrap();
    world.register_organism_record(record).unwrap();
    world
}

fn receipt_row(kind: alife_world::PlayerCareKind, tick: u64) -> WorldOrganismPresentationRow {
    let mut row = quiet_world().presentation_snapshot().organisms.remove(0);
    row.last_player_care = alife_world::PresentationEvidence::Observed {
        value: alife_world::PlayerCareReceipt {
            kinds: alife_world::PlayerCareKindSet::single(kind),
            applied_tick: Tick::new(tick),
            praise_signal_before: 0.10,
            praise_signal_after: 0.42,
            loneliness_before: 0.61,
            loneliness_after: 0.55,
        },
        tick: Tick::new(tick),
    };
    row
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
            expression: CreatureExpressionState::Neutral,
            animation: CreatureAnimationState::Idle,
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
        .insert_resource(ux())
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
        .query_filtered::<Entity, With<CareCue>>()
        .iter(app.world())
        .collect()
}

fn cue_word(app: &App, cue: Entity) -> (String, Color) {
    let child = app.world().get::<Children>(cue).unwrap()[0];
    let text = app.world().get::<Text2d>(child).unwrap().0.clone();
    let color = app.world().get::<TextColor>(child).unwrap().0;
    (text, color)
}

fn reseed(app: &mut App, world: &HeadlessWorld) {
    app.world_mut()
        .resource_mut::<LiveBrainPresentationFrameResource>()
        .reseed_from_loaded_world(world)
        .unwrap();
}

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
            asset_manifest_path: String::new(), backend_descriptor: "CPU care fixture".into(),
            validation_receipt: String::new(),
        },
        animation_speed: 1.0, debug_mode: false, show_help: false, ui_settings_path: PathBuf::new(),
        source_save_path: PathBuf::new(), asset_root: PathBuf::new(), profile_id: profile,
        profile_budget: profile.budget(), population: 0, resolution: (1280, 720),
        save_metadata: ProductionSaveMetadata {
            save_id: "CPU care fixture".into(), deterministic_seed: 21, selected_profile: profile.label().into(),
            profile_budget_version: 1, save_schema: String::new(), save_schema_version: 1,
            config_schema: String::new(), config_schema_version: 1, object_count: 0, creature_count: 0,
            asset_count: 0, voxel_backend_schema: None, voxel_visible_chunk_signatures: 0,
            voxel_materialized_chunks: 0, voxel_resource_hazard_refs: 0, voxel_stable_selection_refs: 0,
            voxel_dirty_region_count: 0, gpu_runtime_schema: None,
            gpu_runtime_selected_backend: "Unavailable".into(),
            gpu_runtime_unavailable_reason: Some("CPU care fixture".into()),
            gpu_runtime_no_active_bulk_readback: true, gpu_runtime_compact_action_readback_bytes: 0,
            gpu_runtime_checkpoint_label: String::new(),
        },
        selected_backend: "Unavailable".into(), adapter_name: String::new(), backend_api: String::new(),
        graphics_backend: String::new(), unavailable_reason: "CPU care fixture".into(),
        renderer_profile: String::new(), state_trace: String::new(),
        gpu_runtime_state: serde_json::from_value(serde_json::json!({
            "schema":"CPU care fixture","schema_version":1,"requested_backend_mode":"GpuAuthoritative",
            "selected_backend_mode":"Unavailable","adapter_identity":{"adapter_name":null,
                "backend_api":null,"adapter_type":null,"driver":null,"driver_info":null},
            "validation_profile":"CPU care fixture","brain_residency_slots":{"hot_slots":0,
                "warm_slots":0,"cold_slots":0},"class_bucket_allocations":[],
            "active_profile_caps":{"target_fps":0,"target_frame_ms":0.0,"renderer_reserve_ms":0.0,
                "gpu_neural_budget_ms":0.0,"neural_heap_mb":0,"staging_readback_budget_kib":0,
                "chunk_activation_radius":0,"active_chunk_cap":0,"vfx_budget":"none",
                "adaptive_throttling_order":[]},
            "shader_abi_versions":{"shader_manifest":[],"abi_manifest":[]},
            "authority":{"authoritative":false,"failure_stops_learned_actions":true,"finite_rejections":0},
            "last_safe_checkpoint":{"save_id":"CPU care fixture","world_tick":0,
                "sealed_patch_boundary":true,"checkpoint_label":"CPU care fixture"},
            "unavailable_reason":"CPU care fixture","selected_scale_profile":"CPU care fixture",
            "compact_action_readback_bytes_per_creature":0,"no_active_bulk_readback":true
        })).unwrap(),
        last_action: String::new(), last_error: None, pending_food_move: None,
        #[cfg(feature = "gpu-runtime")]
        last_manual_checkpoint_status: None,
    }
}

#[test]
fn care_words_come_from_the_codebook_and_carry_valence_in_colour() {
    use alife_world::PlayerCareKind;
    for kind in [
        PlayerCareKind::Pet,
        PlayerCareKind::Praise,
        PlayerCareKind::Scold,
        PlayerCareKind::Slap,
    ] {
        let word = care_word(kind);
        assert!(
            BASIC_VOCABULARY_V1.iter().any(|(vocab, _)| *vocab == word),
            "{word:?} for {kind:?} must be a codebook word"
        );
    }
    assert_eq!(care_word(PlayerCareKind::Pet), "play");
    assert_eq!(care_word(PlayerCareKind::Praise), "get");
    assert_eq!(care_word(PlayerCareKind::Scold), "look");
    assert_eq!(care_word(PlayerCareKind::Slap), "retreat");
    // A pet-and-praise set still says "play" through the display kind;
    // the set itself is pinned in the world tests.
    assert_eq!(
        alife_world::PlayerCareKindSet::single(PlayerCareKind::Pet).display_kind(),
        Some(PlayerCareKind::Pet)
    );
    assert_eq!(
        care_color(PlayerCareKind::Pet),
        care_color(PlayerCareKind::Praise)
    );
    assert_eq!(
        care_color(PlayerCareKind::Scold),
        care_color(PlayerCareKind::Slap)
    );
    assert_ne!(
        care_color(PlayerCareKind::Pet),
        care_color(PlayerCareKind::Scold)
    );
}

#[test]
fn cared_row_fails_closed_on_every_identity_or_lifecycle_mismatch_and_on_unknown() {
    use alife_world::PlayerCareKind;
    let row = receipt_row(PlayerCareKind::Pet, 3);
    let (root, visual) = root_and_visual(&row);
    assert_eq!(
        cared_row(&root, &visual, Some(&row.object), Some(&row))
            .map(|receipt| receipt.display_kind()),
        Some(Some(PlayerCareKind::Pet))
    );
    assert!(cared_row(&root, &visual, None, Some(&row)).is_none());
    assert!(cared_row(&root, &visual, Some(&row.object), None).is_none());

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
    changed.lifecycle = OrganismLifecycle::Dead {
        death_tick: Tick::ZERO,
    };
    bad_rows.push(changed);
    // Unknown is not a receipt: the same creature, same tick, no word.
    let mut changed = row.clone();
    changed.last_player_care = alife_world::PresentationEvidence::Unknown;
    bad_rows.push(changed);
    for changed in bad_rows {
        assert!(cared_row(&root, &visual, Some(&row.object), Some(&changed)).is_none());
    }
    let mut object = row.object.clone();
    object.id = WorldEntityId(root.stable_id.raw() + 1);
    assert!(cared_row(&root, &visual, Some(&object), Some(&row)).is_none());
    object = row.object.clone();
    object.kind = WorldObjectKind::Food;
    assert!(cared_row(&root, &visual, Some(&object), Some(&row)).is_none());
    object = row.object.clone();
    object.organism_id = None;
    assert!(cared_row(&root, &visual, Some(&object), Some(&row)).is_none());
    object = row.object.clone();
    object.organism_id = Some(OrganismId(root.organism_id.raw() + 1));
    assert!(cared_row(&root, &visual, Some(&object), Some(&row)).is_none());
    let mut bad_visual = visual;
    bad_visual.stable_id = WorldEntityId(root.stable_id.raw() + 1);
    assert!(cared_row(&root, &bad_visual, Some(&row.object), Some(&row)).is_none());
    bad_visual = visual;
    bad_visual.organism_id = OrganismId(root.organism_id.raw() + 1);
    assert!(cared_row(&root, &bad_visual, Some(&row.object), Some(&row)).is_none());
    let wrong_root = ProductionCreatureAssemblyRoot {
        organism_id: OrganismId(2),
        ..root
    };
    assert!(cared_row(&wrong_root, &visual, Some(&row.object), Some(&row)).is_none());

    // The frame-level helper reads the same rule. The frame has to be built
    // from a world that actually holds the receipt: try_new takes tick
    // summaries, not presentation rows, and builds an empty organism map,
    // so a row handed to it would never be found.
    let mut world = cared_game();
    let (organism, entity) = world.organism_entity_ids()[0];
    let position = world.entity(entity).unwrap().position;
    world.queue_player_care(organism, position, false).unwrap();
    world.try_advance_tick().unwrap();
    let frame = LiveBrainPresentationFrame::from_authoritative_world(Vec::new(), &world).unwrap();
    let frame_row = frame.organism_snapshots().remove(0);
    let (frame_root, frame_visual) = root_and_visual(&frame_row);
    assert_eq!(
        cared_receipt(&frame, &frame_root, &frame_visual).map(|receipt| receipt.display_kind()),
        Some(Some(PlayerCareKind::Pet))
    );
    let empty = LiveBrainPresentationFrame::try_new(Vec::new(), Tick::ZERO, Vec::new()).unwrap();
    assert!(cared_receipt(&empty, &root, &visual).is_none());
}

#[test]
fn word_appears_for_a_fresh_receipt_clears_after_its_moment_and_never_reappears() {
    let mut world = cared_game();
    let (organism, entity) = world.organism_entity_ids()[0];
    let position = world.entity(entity).unwrap().position;
    world.queue_player_care(organism, position, false).unwrap();
    let tick = world.try_advance_tick().unwrap();
    assert_eq!(world.last_player_care(organism).unwrap().applied_tick, tick);
    let row = world.presentation_snapshot().organisms.remove(0);
    let mut app = fixture(&world);
    let root = spawn_root(&mut app, &row, Transform::default());
    advance(&mut app, 0);
    let cues = cue_entities(&mut app);
    assert_eq!(cues.len(), 1);
    assert_eq!(app.world().get::<ChildOf>(cues[0]).unwrap().parent(), root);
    let (word, color) = cue_word(&app, cues[0]);
    assert_eq!(word, "play");
    assert_eq!(color, care_color(alife_world::PlayerCareKind::Pet));
    // The moment passes: the word clears and never comes back for the
    // same receipt, however long the creature sits there.
    advance(&mut app, 4_000);
    assert!(cue_entities(&mut app).is_empty());
    advance(&mut app, 5_000);
    assert!(cue_entities(&mut app).is_empty());
    // A newer receipt is a new moment: the word returns, with the new kind.
    world.queue_player_care(organism, position, true).unwrap();
    world.try_advance_tick().unwrap();
    reseed(&mut app, &world);
    advance(&mut app, 0);
    let cues = cue_entities(&mut app);
    assert_eq!(cues.len(), 1);
    let (word, _) = cue_word(&app, cues[0]);
    assert_eq!(word, "get");
}

#[test]
fn no_receipt_means_no_word_never_a_nothing_felt_bubble() {
    let world = quiet_world();
    let row = world.presentation_snapshot().organisms.remove(0);
    assert!(matches!(
        row.last_player_care,
        alife_world::PresentationEvidence::Unknown
    ));
    let mut app = fixture(&world);
    spawn_root(&mut app, &row, Transform::default());
    advance(&mut app, 0);
    assert!(cue_entities(&mut app).is_empty());
    advance(&mut app, 5_000);
    assert!(cue_entities(&mut app).is_empty());
}

#[test]
fn pet_and_praise_together_show_the_social_word() {
    let mut world = cared_game();
    let (organism, entity) = world.organism_entity_ids()[0];
    let position = world.entity(entity).unwrap().position;
    world.queue_player_care(organism, position, false).unwrap();
    world.queue_player_care(organism, position, true).unwrap();
    world.try_advance_tick().unwrap();
    let receipt = world.last_player_care(organism).unwrap();
    assert!(receipt.kinds.contains(alife_world::PlayerCareKind::Pet));
    assert!(receipt.kinds.contains(alife_world::PlayerCareKind::Praise));
    assert_eq!(
        receipt.display_kind(),
        Some(alife_world::PlayerCareKind::Pet)
    );
    let row = world.presentation_snapshot().organisms.remove(0);
    let mut app = fixture(&world);
    spawn_root(&mut app, &row, Transform::default());
    advance(&mut app, 0);
    let cues = cue_entities(&mut app);
    assert_eq!(cues.len(), 1);
    let (word, _) = cue_word(&app, cues[0]);
    assert_eq!(word, "play");
}

#[test]
fn scold_and_slap_together_show_the_strongest_negative_word() {
    let mut world = cared_game();
    let (organism, entity) = world.organism_entity_ids()[0];
    let position = world.entity(entity).unwrap().position;
    world
        .queue_player_interaction_v1(organism, position, alife_world::PlayerInteractionV1::Scold)
        .unwrap();
    world
        .queue_player_interaction_v1(organism, position, alife_world::PlayerInteractionV1::Slap)
        .unwrap();
    world.try_advance_tick().unwrap();
    let receipt = world.last_player_care(organism).unwrap();
    assert!(receipt.kinds.contains(alife_world::PlayerCareKind::Scold));
    assert!(receipt.kinds.contains(alife_world::PlayerCareKind::Slap));
    assert_eq!(
        receipt.display_kind(),
        Some(alife_world::PlayerCareKind::Slap)
    );
    let row = world.presentation_snapshot().organisms.remove(0);
    let mut app = fixture(&world);
    spawn_root(&mut app, &row, Transform::default());
    advance(&mut app, 0);
    let cues = cue_entities(&mut app);
    assert_eq!(cues.len(), 1);
    let (word, _) = cue_word(&app, cues[0]);
    assert_eq!(word, "retreat");
}

#[test]
fn word_tracks_displayed_pose_and_camera_and_freezes_on_pause() {
    let mut world = cared_game();
    let (organism, entity) = world.organism_entity_ids()[0];
    let position = world.entity(entity).unwrap().position;
    world.queue_player_care(organism, position, false).unwrap();
    world.try_advance_tick().unwrap();
    let row = world.presentation_snapshot().organisms.remove(0);
    let mut app = fixture(&world);
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
    assert_eq!(app.world().get::<Children>(cue).unwrap().len(), 1);

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
    let seconds = app.world().get::<CareCue>(cue).unwrap().seconds;
    assert!((seconds - 0.25).abs() < 1e-6);
    let root_affine = app.world().get::<Transform>(root).unwrap().compute_affine();
    let cue_pose = app.world().get::<Transform>(cue).unwrap().compute_affine();
    let child = app.world().get::<Children>(cue).unwrap()[0];
    let word_pose = app
        .world()
        .get::<Transform>(child)
        .unwrap()
        .compute_affine();
    let affine = root_affine * cue_pose * word_pose;
    let composed_position = affine.transform_point3(Vec3::ZERO);
    assert!(
        composed_position.y > displayed.translation.y + 1.5,
        "height follows the displayed scaled pose, not a fixed world height"
    );
    let axes = [Vec3::X, Vec3::Y, Vec3::Z].map(|axis| affine.transform_vector3(axis));
    let lengths = axes.map(Vec3::length);
    assert!(lengths[0] > 0.0);
    assert!((lengths[0] - lengths[1]).abs() < 1e-6);
    assert!((lengths[0] - lengths[2]).abs() < 1e-6);
    let axes = axes.map(Vec3::normalize);
    for (first, second) in [(0, 1), (0, 2), (1, 2)] {
        assert!(axes[first].dot(axes[second]).abs() < 1e-6);
    }
    // Text lies in local XY facing +Z: its axes match the camera's.
    for (actual, expected) in axes.into_iter().zip([
        camera_rotation * Vec3::X,
        camera_rotation * Vec3::Y,
        camera_rotation * Vec3::Z,
    ]) {
        assert!((actual - expected).length() < 1e-6);
    }
    // Pause freezes the word's lifetime; unpausing lets it clear.
    app.world_mut()
        .resource_mut::<Fvr05ProductionUxStateResource>()
        .settings
        .paused = true;
    advance(&mut app, 5_000);
    assert_eq!(cue_entities(&mut app), vec![cue]);
    app.world_mut()
        .resource_mut::<Fvr05ProductionUxStateResource>()
        .settings
        .paused = false;
    advance(&mut app, 3_000);
    assert!(cue_entities(&mut app).is_empty());
}

#[test]
fn expired_low_id_receipts_do_not_starve_a_later_creatures_fresh_word() {
    let mut world = cared_population((MAX_CUES + 1) as u16);
    let ids = world.organism_entity_ids();
    for &(organism, entity) in ids.iter().take(MAX_CUES) {
        let position = world.entity(entity).unwrap().position;
        world.queue_player_care(organism, position, false).unwrap();
    }
    world.try_advance_tick().unwrap();
    let mut app = fixture(&world);
    for row in world.presentation_snapshot().organisms {
        spawn_root(&mut app, &row, Transform::default());
    }
    advance(&mut app, 0);
    assert_eq!(cue_entities(&mut app).len(), MAX_CUES);
    advance(&mut app, 4_000);
    assert!(cue_entities(&mut app).is_empty());
    let (organism, entity) = ids[MAX_CUES];
    let position = world.entity(entity).unwrap().position;
    world.queue_player_care(organism, position, true).unwrap();
    world.try_advance_tick().unwrap();
    reseed(&mut app, &world);
    advance(&mut app, 0);
    let cues = cue_entities(&mut app);
    assert_eq!(cues.len(), 1);
    assert_eq!(
        app.world().get::<CareCue>(cues[0]).unwrap().organism_id,
        organism
    );
    assert_eq!(cue_word(&app, cues[0]).0, "get");
}

#[test]
fn replacing_frame_with_an_older_receipt_does_not_replay_it() {
    let mut world = cared_game();
    let (organism, entity) = world.organism_entity_ids()[0];
    let position = world.entity(entity).unwrap().position;
    world.queue_player_care(organism, position, false).unwrap();
    world.try_advance_tick().unwrap();
    let older_world = world.clone();
    let mut app = fixture(&world);
    let row = world.presentation_snapshot().organisms.remove(0);
    spawn_root(&mut app, &row, Transform::default());
    advance(&mut app, 0);
    world.queue_player_care(organism, position, true).unwrap();
    world.try_advance_tick().unwrap();
    reseed(&mut app, &world);
    advance(&mut app, 0);
    let cues = cue_entities(&mut app);
    assert_eq!(cue_word(&app, cues[0]).0, "get");
    reseed(&mut app, &older_world);
    advance(&mut app, 0);
    assert!(cue_entities(&mut app).is_empty());
    advance(&mut app, 1_000);
    assert!(cue_entities(&mut app).is_empty());
}
