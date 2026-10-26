//! Exercises production capture registration and JSON against Bevy's CPU visibility pipeline.
use super::*;
use bevy::asset::AssetPlugin;
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::{
    check_visibility, NoCpuCulling, VisibilityPlugin, VisibilitySystems,
};
use bevy::ecs::schedule::{ExecutorKind, SystemSet};
use bevy::ecs::system::{IntoSystem, System};
use bevy::mesh::{skinning::SkinnedMesh, MeshPlugin};
use bevy::prelude::Rectangle;
use bevy::render::render_resource::PrimitiveTopology;
use bevy::transform::TransformPlugin;
use std::process::Command;

#[test]
fn capture_diagnostics_follow_final_visibility() {
    // A child test process isolates the capture environment without mutating
    // process-global environment variables while other tests may be running.
    if let Some(directory) = std::env::var_os("ALIFE_CAPTURE_ORDER_TEST") {
        run_visibility_fixture(PathBuf::from(directory));
        return;
    }
    let directory = std::env::temp_dir().join(format!(
        "alife-capture-order-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&directory).unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "production_voxel_renderer::graphics_capture::tests::capture_diagnostics_follow_final_visibility", "--nocapture", "--test-threads=1"])
        .env("ALIFE_CAPTURE_ORDER_TEST", &directory)
        .env("ALIFE_GRAPHICS_CAPTURE_DIR", &directory)
        .env("ALIFE_GRAPHICS_CAPTURE_MAX_FRAMES", "16")
        .env("ALIFE_GRAPHICS_CAPTURE_INTERVAL_SECONDS", "0.125")
        .env_remove("ALIFE_ACTION_TRACE_PATH")
        .output().unwrap();
    let completed = directory.join("completed").is_file();
    let _ = fs::remove_dir_all(&directory);
    assert!(
        output.status.success(),
        "headless capture regression failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(completed, "child fixture did not execute to completion");
    eprintln!("{}", String::from_utf8_lossy(&output.stdout));
}

fn run_visibility_fixture(directory: PathBuf) {
    for capture_first in [true, false] {
        let mut app = App::new();
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            AssetPlugin::default(),
            MeshPlugin,
            TransformPlugin,
        ));
        if capture_first {
            install(&mut app);
        }
        app.add_plugins(VisibilityPlugin);
        if !capture_first {
            install(&mut app);
        }
        app.get_schedule_mut(PostUpdate)
            .unwrap()
            .set_executor_kind(ExecutorKind::SingleThreaded);
        app.insert_resource(Time::<bevy::time::Real>::default());
        app.init_resource::<ButtonInput<KeyCode>>();
        app.insert_resource(ux());
        app.insert_resource(Fvr03ProductionVoxelSelectionResource {
            hovered: None,
            selected: None,
        });
        app.insert_resource(creature_grounding::RenderedTerrainSurface::from_meshes(
            std::iter::empty(),
            1.0,
        ));
        // Camera visibility is real, but bypass frustum math to keep this test
        // about visibility lifecycle/ordering, not camera projection parameters.
        app.world_mut().spawn((Camera::default(), NoCpuCulling));
        app.world_mut()
            .spawn(bevy::prelude::AnimationPlayer::default());
        let triangle = app.world_mut().resource_mut::<Assets<Mesh>>().add(
            Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::MAIN_WORLD,
            )
            .with_inserted_attribute(
                Mesh::ATTRIBUTE_POSITION,
                vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
            ),
        );
        let quad = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Mesh::from(Rectangle::new(1., 1.)));
        let root = app
            .world_mut()
            .spawn((Transform::default(), Visibility::Inherited))
            .id();
        app.world_mut()
            .spawn((Mesh3d(triangle.clone()), ChildOf(root)));
        // glTF skinned primitives use the same Mesh3d/ViewVisibility query.
        app.world_mut().spawn((
            Mesh3d(quad),
            ChildOf(root),
            SkinnedMesh {
                inverse_bindposes: Default::default(),
                joints: vec![root],
            },
        ));
        app.world_mut()
            .spawn((Mesh3d(triangle), Visibility::Hidden));
        let mut observed = Vec::new();
        let mut finalized = Vec::new();
        for (frame, shown) in [true, true, false, true].into_iter().enumerate() {
            let path = directory.join(format!("frame-{frame:04}.json"));
            if path.exists() {
                fs::remove_file(&path).unwrap();
            }
            *app.world_mut().get_mut::<Visibility>(root).unwrap() = if shown {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            app.world_mut()
                .resource_mut::<Time<bevy::time::Real>>()
                .advance_by(Duration::from_secs(3));
            app.update();
            finalized.push(
                app.world_mut()
                    .query::<(&Mesh3d, &ViewVisibility)>()
                    .iter(app.world())
                    .filter(|(_, visibility)| visibility.get())
                    .count(),
            );
            let receipt: serde_json::Value =
                serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
            observed.push((
                receipt["visible_mesh_primitives"].as_u64().unwrap(),
                receipt["visible_mesh_triangles_before_batching"]
                    .as_u64()
                    .unwrap(),
            ));
        }
        let schedule = app.get_schedule(PostUpdate).unwrap();
        let order = schedule.systems().unwrap().collect::<Vec<_>>();
        let capture_type = IntoSystem::into_system(capture_player_view).type_id();
        let check_type = IntoSystem::into_system(check_visibility).type_id();
        let capture_at = order
            .iter()
            .position(|(_, system)| system.as_ref().type_id() == capture_type)
            .unwrap();
        let check_at = order
            .iter()
            .position(|(_, system)| system.as_ref().type_id() == check_type)
            .unwrap();
        let positions = |set: VisibilitySystems| {
            let members = schedule.graph().systems_in_set(set.intern()).unwrap();
            order
                .iter()
                .enumerate()
                .filter_map(|(index, (key, _))| members.contains(key).then_some(index))
                .collect::<Vec<_>>()
        };
        println!("capture_first={capture_first}, observed={observed:?}, finalized={finalized:?}, reset_set={:?}, capture={capture_at}, check={check_at}, final_set={:?}",
            positions(VisibilitySystems::VisibilityPropagate), positions(VisibilitySystems::MarkNewlyHiddenEntitiesInvisible));
        assert_eq!(
            finalized,
            [2, 2, 0, 2],
            "real camera visibility did not match the fixture"
        );
        assert_eq!(observed, [(2, 3), (2, 3), (0, 0), (2, 3)]);
    }
    fs::write(
        directory.join("completed"),
        b"headless visibility fixture passed",
    )
    .unwrap();
}

pub(super) fn ux() -> Fvr05ProductionUxStateResource {
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
            asset_manifest_path: String::new(), backend_descriptor: "CPU capture fixture".into(),
            validation_receipt: String::new(),
        },
        animation_speed: 1.0, debug_mode: false, show_help: false, ui_settings_path: PathBuf::new(),
        source_save_path: PathBuf::new(), asset_root: PathBuf::new(), profile_id: profile,
        profile_budget: profile.budget(), population: 0, resolution: (1280, 720),
        save_metadata: ProductionSaveMetadata {
            save_id: "CPU capture fixture".into(), deterministic_seed: 21, selected_profile: profile.label().into(),
            profile_budget_version: 1, save_schema: String::new(), save_schema_version: 1,
            config_schema: String::new(), config_schema_version: 1, object_count: 0, creature_count: 0,
            asset_count: 0, voxel_backend_schema: None, voxel_visible_chunk_signatures: 0,
            voxel_materialized_chunks: 0, voxel_resource_hazard_refs: 0, voxel_stable_selection_refs: 0,
            voxel_dirty_region_count: 0, gpu_runtime_schema: None,
            gpu_runtime_selected_backend: "Unavailable".into(),
            gpu_runtime_unavailable_reason: Some("CPU capture fixture".into()),
            gpu_runtime_no_active_bulk_readback: true, gpu_runtime_compact_action_readback_bytes: 0,
            gpu_runtime_checkpoint_label: String::new(),
        },
        selected_backend: "Unavailable".into(), adapter_name: String::new(), backend_api: String::new(),
        graphics_backend: String::new(), unavailable_reason: "CPU capture fixture".into(),
        renderer_profile: String::new(), state_trace: String::new(),
        gpu_runtime_state: serde_json::from_value(serde_json::json!({
            "schema":"CPU capture fixture","schema_version":1,"requested_backend_mode":"GpuAuthoritative",
            "selected_backend_mode":"Unavailable","adapter_identity":{"adapter_name":null,
                "backend_api":null,"adapter_type":null,"driver":null,"driver_info":null},
            "validation_profile":"CPU capture fixture","brain_residency_slots":{"hot_slots":0,
                "warm_slots":0,"cold_slots":0},"class_bucket_allocations":[],
            "active_profile_caps":{"target_fps":0,"target_frame_ms":0.0,"renderer_reserve_ms":0.0,
                "gpu_neural_budget_ms":0.0,"neural_heap_mb":0,"staging_readback_budget_kib":0,
                "chunk_activation_radius":0,"active_chunk_cap":0,"vfx_budget":"none",
                "adaptive_throttling_order":[]},
            "shader_abi_versions":{"shader_manifest":[],"abi_manifest":[]},
            "authority":{"authoritative":false,"failure_stops_learned_actions":true,"finite_rejections":0},
            "last_safe_checkpoint":{"save_id":"CPU capture fixture","world_tick":0,
                "sealed_patch_boundary":true,"checkpoint_label":"CPU capture fixture"},
            "unavailable_reason":"CPU capture fixture","selected_scale_profile":"CPU capture fixture",
            "compact_action_readback_bytes_per_creature":0,"no_active_bulk_readback":true
        })).unwrap(),
        last_action: String::new(), last_error: None, pending_food_move: None,
        #[cfg(feature = "gpu-runtime")]
        last_manual_checkpoint_status: None,
    }
}
