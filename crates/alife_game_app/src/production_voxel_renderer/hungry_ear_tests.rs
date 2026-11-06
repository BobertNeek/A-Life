//! Renderer-only fixtures; no organism, brain or world advancement.
use super::*;

#[test]
fn hungry_ears_gate_strict_idle_and_preserve_priority() {
    let mut homeostasis = alife_core::HomeostaticSnapshot::baseline(Tick::new(9));
    homeostasis.drives.hunger = 0.82;
    homeostasis.drives.pain = 0.0;
    homeostasis.drives.fear = 0.0;
    let derived = crate::creature_expression_state(alife_core::SleepPhase::Awake, &homeostasis);
    assert_eq!(derived, CreatureExpressionState::Hungry);
    assert!(HungryEars::eligible(
        derived,
        CreatureAnimationState::Idle,
        false,
        false
    ));
    homeostasis.drives.pain = 0.8;
    let derived = crate::creature_expression_state(alife_core::SleepPhase::Awake, &homeostasis);
    assert_eq!(derived, CreatureExpressionState::Pained);
    assert!(!HungryEars::eligible(
        derived,
        CreatureAnimationState::Idle,
        false,
        false
    ));
    homeostasis.drives.pain = 0.0;
    homeostasis.drives.fear = 0.8;
    let derived = crate::creature_expression_state(alife_core::SleepPhase::Awake, &homeostasis);
    assert_eq!(derived, CreatureExpressionState::Afraid);
    assert!(!HungryEars::eligible(
        derived,
        CreatureAnimationState::Idle,
        false,
        false
    ));
    let derived =
        crate::creature_expression_state(alife_core::SleepPhase::Consolidating, &homeostasis);
    assert_eq!(derived, CreatureExpressionState::Tired);
    assert!(!HungryEars::eligible(
        derived,
        CreatureAnimationState::Sleeping,
        false,
        false
    ));
    for animation in [
        CreatureAnimationState::Idle,
        CreatureAnimationState::Moving,
        CreatureAnimationState::Inspecting,
        CreatureAnimationState::Interacting,
        CreatureAnimationState::Resting,
        CreatureAnimationState::Sleeping,
        CreatureAnimationState::Signaling,
        CreatureAnimationState::Hurt,
        CreatureAnimationState::Afraid,
        CreatureAnimationState::Curious,
    ] {
        for expression in [
            CreatureExpressionState::Hungry,
            CreatureExpressionState::Neutral,
            CreatureExpressionState::Tired,
            CreatureExpressionState::Afraid,
            CreatureExpressionState::Pained,
            CreatureExpressionState::Curious,
            CreatureExpressionState::Energized,
        ] {
            for held in [false, true] {
                for moved in [false, true] {
                    let expected = animation == CreatureAnimationState::Idle
                        && matches!(
                            expression,
                            CreatureExpressionState::Hungry | CreatureExpressionState::Neutral
                        )
                        && !held
                        && !moved;
                    let eligible = HungryEars::eligible(expression, animation, held, moved);
                    assert_eq!(eligible, expected);
                    let ears = HungryEars { blend: 1.0 };
                    let sampled = Quat::from_rotation_z(0.07);
                    for side in [-1.0, 1.0] {
                        let result = ears.rotation(sampled, ExpressionBone::Ear(side), eligible);
                        if expected {
                            assert!(result.abs_diff_eq(sampled * Quat::from_rotation_x(0.15), 1e-6));
                        } else {
                            assert_eq!(result, sampled);
                        }
                    }
                    for part in [ExpressionBone::Head, ExpressionBone::Lid(1.07)] {
                        assert_eq!(ears.rotation(sampled, part, eligible), sampled);
                    }
                }
            }
        }
    }
}

#[test]
fn hungry_ears_blend_bounds_pause_invalid_time_and_exact_neutral_recovery() {
    let mut ears = HungryEars::default();
    for seconds in [0.0, -0.1, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        ears.advance(seconds, false, true);
        assert_eq!(ears.blend, 0.0);
    }
    ears.advance(0.05, false, true);
    assert!((ears.blend - 0.2).abs() < 1e-6);
    // Pause freezes both an active cue and a transition back to neutral.
    ears.advance(1.0, true, true);
    ears.advance(1.0, true, false);
    assert!((ears.blend - 0.2).abs() < 1e-6);
    ears.advance(f32::MAX, false, true);
    assert!((ears.blend - 0.6).abs() < 1e-6);
    for _ in 0..20 {
        ears.advance(0.1, false, true);
    }
    assert_eq!(ears.blend, 1.0);
    for seconds in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        ears.advance(seconds, false, false);
        assert_eq!(ears.blend, 1.0);
    }
    ears.advance(0.05, false, false);
    assert!((ears.blend - 0.8).abs() < 1e-6);
    for _ in 0..20 {
        ears.advance(0.1, false, false);
    }
    assert_eq!(ears.blend, 0.0);
    let sampled = Quat::from_rotation_y(0.3);
    assert_eq!(
        ears.rotation(sampled, ExpressionBone::Ear(1.0), true),
        sampled
    );
}

#[test]
fn hungry_ears_ecs_fresh_samples_leave_gaze_markers_and_feedback_intact() {
    let marker = marker();
    let mut visual = HearthlingVisual {
        appearance: CreatureAppearanceGenome::default(),
        previous_position: Vec3::ZERO,
        walk_seconds: 0.7,
        moved: false,
        feedback: OutcomeFeedback::default(),
        hungry_ears: HungryEars { blend: 1.0 },
        hungry_ears_eligible: true,
    };
    visual.feedback.observe(Some(8), Some(9));
    visual.feedback.advance(0.15, false);
    let feedback_before = visual.capture_feedback();
    let mut app = App::new();
    let root = app
        .world_mut()
        .spawn((marker, visual, Transform::default()))
        .id();
    let parts = [
        ExpressionBone::Head,
        ExpressionBone::Ear(1.0),
        ExpressionBone::Ear(-1.0),
        ExpressionBone::Lid(1.07),
    ];
    let bones: Vec<_> = parts
        .iter()
        .map(|&part| {
            app.world_mut()
                .spawn((
                    Transform::default(),
                    HearthlingPoseBone {
                        root,
                        part,
                        bind_rotation: Quat::IDENTITY,
                    },
                ))
                .id()
        })
        .collect();
    app.add_systems(Update, apply_head_direction);
    for sampled in [
        Quat::from_rotation_z(0.06),
        Quat::from_rotation_x(-0.03),
        Quat::from_rotation_z(0.06),
    ] {
        // Same contract as AnimationSystems: overwrite every bone with a fresh sample.
        for &bone in &bones {
            app.world_mut().get_mut::<Transform>(bone).unwrap().rotation = sampled;
        }
        app.update();
        let visual = app.world().get::<HearthlingVisual>(root).unwrap();
        for (&bone, &part) in bones.iter().zip(&parts) {
            let base = if matches!(part, ExpressionBone::Head) {
                head_rotation(sampled, marker.head_yaw)
            } else {
                sampled
            };
            let offset = if matches!(part, ExpressionBone::Ear(_)) {
                base * Quat::from_rotation_x(HUNGRY_EAR_ANGLE)
            } else {
                base
            };
            let expected = outcome_rotation(offset, part, &visual.feedback, marker.animation);
            assert!(app
                .world()
                .get::<Transform>(bone)
                .unwrap()
                .rotation
                .abs_diff_eq(expected, 1e-6));
        }
        assert_eq!(
            app.world().get::<Fvr04ProductionCreatureVisualMarker>(root),
            Some(&marker)
        );
        assert_eq!(visual.capture_feedback(), feedback_before);
        assert_eq!(visual.walk_seconds, 0.7);
        assert_eq!(
            *app.world().get::<Transform>(root).unwrap(),
            Transform::default()
        );
    }
}

#[test]
fn hungry_ears_all_shipping_clips_resample_ear_rotations() {
    let glb = include_bytes!("../../assets/creatures/hearthling/hearthling.glb");
    let json_bytes = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
    let gltf: serde_json::Value = serde_json::from_slice(&glb[20..20 + json_bytes]).unwrap();
    assert_eq!(gltf["animations"].as_array().unwrap().len(), 3);
    for clip in gltf["animations"].as_array().unwrap() {
        for name in ["ear.L", "ear.R"] {
            assert!(
                clip["channels"].as_array().unwrap().iter().any(|channel| {
                    let node = channel["target"]["node"].as_u64().unwrap() as usize;
                    channel["target"]["path"] == "rotation" && gltf["nodes"][node]["name"] == name
                }),
                "{} must freshly sample {name}",
                clip["name"]
            );
        }
    }
}

fn marker() -> Fvr04ProductionCreatureVisualMarker {
    Fvr04ProductionCreatureVisualMarker {
        stable_id: WorldEntityId(1),
        organism_id: OrganismId(2),
        tile: VoxelTileCoord::new(0, 0),
        expression: CreatureExpressionState::Hungry,
        animation: CreatureAnimationState::Idle,
        lod: Fvr04CreatureLod::CompactVoxel,
        base_translation: Vec3::ZERO,
        local_offset: Vec3::ZERO,
        base_scale: Vec3::ONE,
        local_bounds: LOCAL_BOUNDS,
        surface_height: 0.0,
        body_yaw: 0.8,
        head_yaw: -0.45,
        phase: 0.2,
    }
}

fn ux() -> Fvr05ProductionUxStateResource {
    let profile = ProductionFrontendProfileId::MinimumSettings30x30;
    Fvr05ProductionUxStateResource{
        settings:Fvr05ProductionUxSettings{
            schema:"alife.fvr05.production_ux.v1".into(),schema_version:1,
            selected_profile:profile,preferred_profile_for_next_launch:profile,
            active_inspector_tab:Fvr05ProductionInspectorTab::Creature,enabled_overlays:vec![],
            camera_mode:"orbit".into(),paused:false,run_mode:None,simulation_speed:1.0,
            follow_selection:false,show_menu:false,show_settings:false,show_overlays:false,
            pause_on_focus_loss:true,selected_stable_id:None,source_save_path:String::new(),
            runtime_save_path:String::new(),created_world_save_path:String::new(),
            asset_manifest_path:String::new(),backend_descriptor:"UI-only fixture".into(),
            validation_receipt:String::new()},
        animation_speed:0.0,debug_mode:false,show_help:false,ui_settings_path:PathBuf::new(),
        source_save_path:PathBuf::new(),asset_root:PathBuf::new(),profile_id:profile,
        profile_budget:profile.budget(),population:0,resolution:(1280,720),
        save_metadata:ProductionSaveMetadata{
            save_id:"UI-only fixture".into(),deterministic_seed:21,selected_profile:profile.label().into(),
            profile_budget_version:1,save_schema:String::new(),save_schema_version:1,
            config_schema:String::new(),config_schema_version:1,object_count:0,creature_count:0,
            asset_count:0,voxel_backend_schema:None,voxel_visible_chunk_signatures:0,
            voxel_materialized_chunks:0,voxel_resource_hazard_refs:0,voxel_stable_selection_refs:0,
            voxel_dirty_region_count:0,gpu_runtime_schema:None,
            gpu_runtime_selected_backend:"Unavailable".into(),
            gpu_runtime_unavailable_reason:Some("CPU UI fixture".into()),
            gpu_runtime_no_active_bulk_readback:true,gpu_runtime_compact_action_readback_bytes:0,
            gpu_runtime_checkpoint_label:String::new()},
        selected_backend:"Unavailable".into(),adapter_name:String::new(),backend_api:String::new(),
        graphics_backend:String::new(),unavailable_reason:"CPU UI fixture".into(),
        renderer_profile:String::new(),state_trace:String::new(),
        gpu_runtime_state:serde_json::from_value(serde_json::json!({
            "schema":"UI-only fixture","schema_version":1,"requested_backend_mode":"GpuAuthoritative",
            "selected_backend_mode":"Unavailable","adapter_identity":{"adapter_name":null,
                "backend_api":null,"adapter_type":null,"driver":null,"driver_info":null},
            "validation_profile":"UI-only fixture","brain_residency_slots":{"hot_slots":0,
                "warm_slots":0,"cold_slots":0},"class_bucket_allocations":[],
            "active_profile_caps":{"target_fps":0,"target_frame_ms":0.0,"renderer_reserve_ms":0.0,
                "gpu_neural_budget_ms":0.0,"neural_heap_mb":0,"staging_readback_budget_kib":0,
                "chunk_activation_radius":0,"active_chunk_cap":0,"vfx_budget":"none",
                "adaptive_throttling_order":[]},
            "shader_abi_versions":{"shader_manifest":[],"abi_manifest":[]},
            "authority":{"authoritative":false,"failure_stops_learned_actions":true,"finite_rejections":0},
            "last_safe_checkpoint":{"save_id":"UI-only fixture","world_tick":0,
                "sealed_patch_boundary":true,"checkpoint_label":"UI-only fixture"},
            "unavailable_reason":"CPU UI fixture","selected_scale_profile":"UI-only fixture",
            "compact_action_readback_bytes_per_creature":0,"no_active_bulk_readback":true})).unwrap(),
        last_action:String::new(),last_error:None,pending_food_move:None,
        #[cfg(feature="gpu-runtime")] last_manual_checkpoint_status:None,
    }
}

#[test]
fn hungry_ears_pause_retained_walk_clip_is_suppressed_until_idle_resumes() {
    let mut app = App::new();
    let mut ux = ux();
    ux.settings.paused = false;
    app.insert_resource(ux)
        .insert_resource(Time::<()>::default());
    let mut marker = marker();
    // World marker is Idle, but the displayed root is still catching up.
    marker.base_translation = Vec3::X;
    let root = app
        .world_mut()
        .spawn((
            marker,
            Transform::default(),
            HearthlingVisual {
                appearance: CreatureAppearanceGenome::default(),
                previous_position: Vec3::ZERO,
                walk_seconds: 0.0,
                moved: false,
                feedback: OutcomeFeedback::default(),
                hungry_ears: HungryEars { blend: 1.0 },
                hungry_ears_eligible: true,
            },
        ))
        .id();
    let clips = vec![
        AnimationNodeIndex::new(0),
        AnimationNodeIndex::new(1),
        AnimationNodeIndex::new(2),
    ];
    let player = app
        .world_mut()
        .spawn((
            AnimationPlayer::default(),
            HearthlingPlayer {
                root,
                clips,
                state: 0,
            },
        ))
        .id();
    let ear = app
        .world_mut()
        .spawn((
            Transform::default(),
            HearthlingPoseBone {
                root,
                part: ExpressionBone::Ear(1.0),
                bind_rotation: Quat::IDENTITY,
            },
        ))
        .id();
    app.add_systems(Update, (animate, apply_head_direction).chain());
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(16));
    app.update();
    assert!(app.world().get::<HearthlingVisual>(root).unwrap().moved);
    assert_eq!(
        app.world().get::<HearthlingPlayer>(player).unwrap().state,
        1
    );
    assert_eq!(
        app.world().get::<Transform>(ear).unwrap().rotation,
        Quat::IDENTITY
    );
    let weight = app
        .world()
        .get::<HearthlingVisual>(root)
        .unwrap()
        .hungry_ears
        .blend;
    app.world_mut()
        .resource_mut::<Fvr05ProductionUxStateResource>()
        .settings
        .paused = true;
    app.update();
    assert_eq!(
        app.world().get::<HearthlingPlayer>(player).unwrap().state,
        1
    );
    let visual = app.world().get::<HearthlingVisual>(root).unwrap();
    assert!(!visual.moved);
    assert!(!visual.hungry_ears_eligible);
    assert_eq!(visual.hungry_ears.blend, weight);
    assert_eq!(
        app.world().get::<Transform>(ear).unwrap().rotation,
        Quat::IDENTITY
    );
    // Catch up exactly, unpause, and let the real clip selection return to Idle.
    app.world_mut()
        .get_mut::<Transform>(root)
        .unwrap()
        .translation = Vec3::X;
    app.world_mut()
        .get_mut::<HearthlingVisual>(root)
        .unwrap()
        .previous_position = Vec3::X;
    app.world_mut()
        .resource_mut::<Fvr05ProductionUxStateResource>()
        .settings
        .paused = false;
    app.world_mut().get_mut::<Transform>(ear).unwrap().rotation = Quat::IDENTITY;
    app.update();
    assert_eq!(
        app.world().get::<HearthlingPlayer>(player).unwrap().state,
        0
    );
    let visual = app.world().get::<HearthlingVisual>(root).unwrap();
    assert!(visual.hungry_ears_eligible);
    assert!(app
        .world()
        .get::<Transform>(ear)
        .unwrap()
        .rotation
        .abs_diff_eq(
            Quat::from_rotation_x(HUNGRY_EAR_ANGLE * visual.hungry_ears.blend),
            1e-6
        ));
    assert_eq!(
        app.world().get::<Fvr04ProductionCreatureVisualMarker>(root),
        Some(&marker)
    );
}
