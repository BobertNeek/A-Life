//! Actual CPU UiSurface layout and frontend contract tests, not screenshots.
use super::*;
use alife_core::{BrainCapacityClass, CreatureGenome, FoundationGeneticIdentity};
use alife_world::{HeadlessScenarioBuilder, HeadlessWorld, WorldOrganismRecord};
use bevy::app::{HierarchyPropagatePlugin, PostUpdate, PropagateSet};
use bevy::camera::{ComputedCameraValues, RenderTargetInfo, Viewport};
use bevy::ecs::message::Messages;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::ui::update::propagate_ui_target_cameras;
use bevy::ui::{
    ui_layout_system, ui_surface::UiSurface, ComputedUiRenderTargetInfo, ComputedUiTargetCamera,
    UiGlobalTransform, UiScale,
};
use serde_json::{json, Value};

const VIEWPORTS: [Vec2; 4] = [
    Vec2::new(1280.0, 720.0),
    Vec2::new(640.0, 360.0),
    Vec2::new(320.0, 480.0),
    Vec2::new(320.0, 240.0),
];
const EPSILON: f32 = 0.1;

fn authoritative_world() -> HeadlessWorld {
    let organism = OrganismId(1);
    let mut world = HeadlessScenarioBuilder::new(21)
        .agent("agent", organism, Vec3f::ZERO)
        .agent("unregistered", OrganismId(2), Vec3f::new(6.0, 2.0, 0.0))
        .food("food", Vec3f::new(3.0, 1.0, 0.0), 0.5)
        .build()
        .unwrap();
    let entity = world.entity_id("agent").unwrap();
    let genome = CreatureGenome::early_mammal_founder(
        21,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    let phenotype = genome.express().unwrap();
    let chemistry =
        alife_core::BiochemistryState::new_with_age(&phenotype, Tick::ZERO, Tick::ZERO).unwrap();
    world
        .register_organism_record(
            WorldOrganismRecord::new(organism, entity, genome, phenotype, chemistry, Tick::ZERO)
                .unwrap(),
        )
        .unwrap();
    world
}
fn reference(kind: StableVoxelRefKind, id: Option<WorldEntityId>) -> StableVoxelObjectRef {
    let tile = VoxelTileCoord::new(4, 7);
    StableVoxelObjectRef {
        kind,
        stable_id: id,
        chunk: VoxelChunkCoord::for_tile(16, tile),
        tile: Some(tile),
    }
}

// UI-only resource metadata; this does not construct or select a GPU runtime.
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
            voxel_dirty_region_count:0,voxel_roundtrip_signatures_match:false,
            no_renderer_tokens_in_voxel_save:true,gpu_runtime_schema:None,
            gpu_runtime_selected_backend:"Unavailable".into(),
            gpu_runtime_unavailable_reason:Some("CPU UI fixture".into()),
            gpu_runtime_no_active_bulk_readback:true,gpu_runtime_compact_action_readback_bytes:0,
            gpu_runtime_checkpoint_label:String::new()},
        selected_backend:"Unavailable".into(),adapter_name:String::new(),backend_api:String::new(),
        graphics_backend:String::new(),unavailable_reason:"CPU UI fixture".into(),
        renderer_profile:String::new(),state_trace:String::new(),
        authority:Fvr05ProductionDebugAuthorityReport::production_read_only(),
        gpu_runtime_state:serde_json::from_value(json!({
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
pub(crate) fn layout_app(size: Vec2) -> App {
    let mut app = App::new();
    app.add_plugins(HierarchyPropagatePlugin::<ComputedUiTargetCamera>::new(
        PostUpdate,
    ));
    app.add_plugins(HierarchyPropagatePlugin::<ComputedUiRenderTargetInfo>::new(
        PostUpdate,
    ));
    app.init_resource::<UiScale>()
        .init_resource::<UiSurface>()
        .init_resource::<bevy::text::TextPipeline>()
        .init_resource::<bevy::text::CosmicFontSystem>()
        .init_resource::<bevy::text::SwashCache>();
    app.add_systems(
        PostUpdate,
        (ApplyDeferred, propagate_ui_target_cameras, ui_layout_system).chain(),
    );
    app.configure_sets(
        PostUpdate,
        PropagateSet::<ComputedUiTargetCamera>::default()
            .after(propagate_ui_target_cameras)
            .before(ui_layout_system),
    );
    app.configure_sets(
        PostUpdate,
        PropagateSet::<ComputedUiRenderTargetInfo>::default()
            .after(propagate_ui_target_cameras)
            .before(ui_layout_system),
    );
    app.world_mut().spawn((
        Camera2d,
        Camera {
            computed: ComputedCameraValues {
                target_info: Some(RenderTargetInfo {
                    physical_size: size.as_uvec2(),
                    scale_factor: 1.0,
                }),
                ..default()
            },
            viewport: Some(Viewport {
                physical_size: size.as_uvec2(),
                ..default()
            }),
            ..default()
        },
    ));
    app.world_mut().spawn((
        Window {
            resolution: (size.x as u32, size.y as u32).into(),
            ..default()
        },
        PrimaryWindow,
    ));
    app.insert_resource(ux())
        .insert_resource(Fvr03ProductionVoxelSelectionResource {
            hovered: None,
            selected: None,
        });
    spawn(app.world_mut());
    sync(app.world_mut());
    app
}
#[derive(Clone, Copy, Debug)]
struct Box2 {
    min: Vec2,
    max: Vec2,
}
impl Box2 {
    fn of(world: &World, entity: Entity) -> Self {
        let center = world
            .get::<UiGlobalTransform>(entity)
            .unwrap()
            .affine()
            .translation;
        let half = world.get::<ComputedNode>(entity).unwrap().size() * 0.5;
        assert!(center.is_finite() && half.is_finite());
        Self {
            min: center - half,
            max: center + half,
        }
    }
    fn size(self) -> Vec2 {
        self.max - self.min
    }
    fn contains(self, child: Self) -> bool {
        child.min.x >= self.min.x - EPSILON
            && child.min.y >= self.min.y - EPSILON
            && child.max.x <= self.max.x + EPSILON
            && child.max.y <= self.max.y + EPSILON
    }
    fn overlaps(self, other: Self) -> bool {
        self.min.x < other.max.x - EPSILON
            && other.min.x < self.max.x - EPSILON
            && self.min.y < other.max.y - EPSILON
            && other.min.y < self.max.y - EPSILON
    }
    fn json(self) -> Value {
        json!({"min":[self.min.x,self.min.y],"max":[self.max.x,self.max.y],
        "size":[self.size().x,self.size().y]})
    }
}
fn visible(world: &World, mut entity: Entity) -> bool {
    loop {
        if world.get::<Visibility>(entity) == Some(&Visibility::Hidden) {
            return false;
        }
        match world.get::<ChildOf>(entity) {
            Some(parent) => entity = parent.parent(),
            None => return true,
        }
    }
}
fn image_name(world: &World, entity: Entity) -> Option<&'static str> {
    let image = world.get::<ImageNode>(entity)?;
    world
        .resource::<Icons>()
        .0
        .iter()
        .find_map(|(&name, handle)| (*handle == image.image).then_some(name))
}
fn assert_size(bounds: Box2, expected: f32) {
    assert!(
        (bounds.size().x - expected).abs() < EPSILON,
        "{bounds:?}, expected {expected}"
    );
    assert!(
        (bounds.size().y - expected).abs() < EPSILON,
        "{bounds:?}, expected {expected}"
    );
}
fn assert_layout(world: &mut World, size: Vec2) {
    let viewport = Box2 {
        min: Vec2::ZERO,
        max: size,
    };
    let root = world
        .query_filtered::<Entity, With<DockRoot>>()
        .single(world)
        .unwrap();
    let dock = Box2::of(world, root);
    assert!(viewport.contains(dock), "dock outside {size:?}: {dock:?}");
    assert!((dock.size().x - WIDTH).abs() < EPSILON && (dock.size().y - HEIGHT).abs() < EPSILON);
    assert!((dock.min.x - (size.x - WIDTH) * 0.5).abs() < EPSILON);
    assert!((dock.max.y - (size.y - BOTTOM)).abs() < EPSILON);
    let buttons = world
        .query::<(Entity, &Action)>()
        .iter(world)
        .map(|(e, a)| (e, *a))
        .collect::<Vec<_>>();
    let glyphs = world
        .query::<(Entity, &Glyph)>()
        .iter(world)
        .map(|(e, g)| (e, g.0))
        .collect::<Vec<_>>();
    let markers = world
        .query::<(Entity, &Marker)>()
        .iter(world)
        .map(|(e, m)| (e, m.0))
        .collect::<Vec<_>>();
    assert_eq!(buttons.len(), 5);
    assert_eq!(glyphs.len(), 5);
    assert_eq!(markers.len(), 5);
    assert_eq!(
        world
            .get::<Children>(root)
            .unwrap()
            .iter()
            .filter_map(|e| world.get::<Action>(e).copied())
            .collect::<Vec<_>>(),
        ACTIONS.to_vec()
    );
    for (index, &(button, action)) in buttons.iter().enumerate() {
        let bounds = Box2::of(world, button);
        assert_size(bounds, 48.0);
        assert!(dock.contains(bounds));
        for &(other, _) in &buttons[index + 1..] {
            assert!(!bounds.overlaps(Box2::of(world, other)), "buttons overlap");
        }
        let glyph = glyphs.iter().find(|(_, a)| *a == action).unwrap().0;
        let glyph_box = Box2::of(world, glyph);
        assert_size(glyph_box, 32.0);
        assert!(bounds.contains(glyph_box));
        let marker = markers.iter().find(|(_, a)| *a == action).unwrap().0;
        let marker_box = Box2::of(world, marker);
        assert_size(marker_box, 16.0);
        assert!(
            dock.contains(marker_box),
            "{action:?} marker outside dock at {size:?}: {marker_box:?}"
        );
        for &(other, _) in &glyphs {
            assert!(
                !marker_box.overlaps(Box2::of(world, other)),
                "marker overlaps glyph"
            );
        }
        for &(other, a) in &buttons {
            if a != action {
                assert!(
                    !marker_box.overlaps(Box2::of(world, other)),
                    "marker overlaps adjacent button"
                );
            }
        }
        for &(other, a) in &markers {
            if a != action {
                assert!(
                    !marker_box.overlaps(Box2::of(world, other)),
                    "markers overlap"
                );
            }
        }
    }
    let alert = world
        .query_filtered::<Entity, With<ErrorMarker>>()
        .single(world)
        .unwrap();
    let alert_box = Box2::of(world, alert);
    assert_size(alert_box, 16.0);
    assert!(viewport.contains(alert_box));
    for &(entity, _) in &buttons {
        assert!(
            !alert_box.overlaps(Box2::of(world, entity)),
            "error marker overlaps button"
        );
    }
    assert_eq!(
        world.query::<&Text>().iter(world).count(),
        0,
        "care dock must be wordless"
    );
    assert_eq!(world.query::<&Text2d>().iter(world).count(), 0);
}
fn rgba(color: Color) -> [f32; 4] {
    let c = color.to_srgba();
    [c.red, c.green, c.blue, c.alpha]
}
fn no_green(color: Color) {
    let [r, g, b, a] = rgba(color);
    assert!(
        a == 0.0 || g <= r.max(b) + 0.08,
        "green success cue: {color:?}"
    );
}
fn val(value: Val) -> Value {
    match value {
        Val::Px(v) => json!({"unit":"px","value":v}),
        Val::Percent(v) => json!({"unit":"percent","value":v}),
        Val::Auto => json!({"unit":"auto"}),
        other => json!({"source_debug":format!("{other:?}")}),
    }
}
fn rect(r: UiRect) -> Value {
    json!({"top":val(r.top),"right":val(r.right),"bottom":val(r.bottom),"left":val(r.left)})
}
pub(crate) fn export_nodes(world: &mut World) -> Vec<Value> {
    let entities = world
        .query_filtered::<Entity, With<Node>>()
        .iter(world)
        .collect::<Vec<_>>();
    entities.into_iter().map(|e| {
        let node=world.get::<Node>(e).unwrap();
        let action=world.get::<Action>(e).copied().or_else(||world.get::<Glyph>(e).map(|g|g.0))
            .or_else(||world.get::<Marker>(e).map(|m|m.0));
        let role=if world.get::<DockRoot>(e).is_some(){"dock"}
            else if world.get::<Action>(e).is_some(){"action_button"}
            else if world.get::<Glyph>(e).is_some(){"action_glyph"}
            else if world.get::<Marker>(e).is_some(){"evidence_marker"}
            else if world.get::<ErrorMarker>(e).is_some(){"separate_alert_gutter"}else{"container"};
        #[cfg(feature = "gpu-runtime")]
        let role = if world.get::<crate::production_conversation_lineage_ui::composer::DraftViewport>(e).is_some() {
            "speech_draft_viewport"
        } else if world.get::<crate::production_conversation_lineage_ui::ProductionSpeechComposerBounds>(e).is_some() {
            "speech_composer"
        } else if world.get::<Text>(e).is_some() { "speech_text" } else { role };
        let border=world.get::<BorderColor>(e).map(|b|{
            for c in [b.top,b.right,b.bottom,b.left]{no_green(c);}
            json!({"top":rgba(b.top),"right":rgba(b.right),"bottom":rgba(b.bottom),"left":rgba(b.left)})});
        let image=world.get::<ImageNode>(e).map(|image|{
            no_green(image.color);let name=image_name(world,e);
            let source_path=name.map(|n|format!("assets/ui/care_dock/{n}.png")).or_else(||crate::production_conversation_lineage_ui::composer::image_path(world,&image.image)).expect("embedded UI icon");
            let asset=world.resource::<Assets<Image>>().get(&image.image).unwrap();
            json!({"name":name,"source_path":source_path,
                "decoded_size":[asset.width(),asset.height()],"tint":rgba(image.color)})});
        let background=world.get::<BackgroundColor>(e).map(|b|{no_green(b.0);rgba(b.0)});
        json!({"entity":format!("{e:?}"),"parent":world.get::<ChildOf>(e).map(|p|format!("{:?}",p.parent())),
            "name":world.get::<Name>(e).map(|n|n.as_str()),"role":role,"action":action.map(Action::icon),
            "computed_box":Box2::of(world,e).json(),"explicit_visibility":world.get::<Visibility>(e).map(|v|format!("{v:?}")),
            "visible_in_hierarchy":visible(world,e),"text":world.get::<Text>(e).map(|text| json!({"value":text.0,"font_size":world.get::<TextFont>(e).map(|font|font.font_size),"glyph_bounds_measured":false,"linebreak":world.get::<TextLayout>(e).map(|layout|format!("{:?}",layout.linebreak))})),
            "style":{"display":format!("{:?}",node.display),"position_type":format!("{:?}",node.position_type),"overflow":{"x":format!("{:?}",node.overflow.x),"y":format!("{:?}",node.overflow.y)},
                "left":val(node.left),"top":val(node.top),"right":val(node.right),"bottom":val(node.bottom),
                "width":val(node.width),"height":val(node.height),"min_width":val(node.min_width),"max_width":val(node.max_width),
                "min_height":val(node.min_height),"max_height":val(node.max_height),"padding":rect(node.padding),"border":rect(node.border),
                "border_radius":{"top_left":val(node.border_radius.top_left),"top_right":val(node.border_radius.top_right),
                    "bottom_right":val(node.border_radius.bottom_right),"bottom_left":val(node.border_radius.bottom_left)},
                "flex_direction":format!("{:?}",node.flex_direction),"flex_wrap":format!("{:?}",node.flex_wrap),
                "flex_shrink":node.flex_shrink,"align_items":format!("{:?}",node.align_items),
                "justify_content":format!("{:?}",node.justify_content),"column_gap":val(node.column_gap),"row_gap":val(node.row_gap),
                "background":background,"border_colors":border},"image":image})
    }).collect()
}
#[test]
fn cpu_layout_fits_four_viewports_and_exports_actual_wordless_feedback_nodes() {
    let mut queued = [Evidence::None; 5];
    queued[Action::Touch.index()] = Evidence::Queued;
    queued[Action::Praise.index()] = Evidence::Queued;
    let mut placed = [Evidence::None; 5];
    placed[Action::Food.index()] = Evidence::Placed(WorldEntityId(41));
    placed[Action::Toy.index()] = Evidence::Placed(WorldEntityId(42));
    let cases = [
        ("ready", [true; 5], [Evidence::None; 5], false),
        ("queued", [true; 5], queued, false),
        ("placed", [true; 5], placed, false),
        ("rejected", [true; 5], [Evidence::Rejected; 5], true),
        ("unavailable", [false; 5], [Evidence::Unavailable; 5], true),
    ];
    let mut fixtures = Vec::new();
    let mut embedded_assets = Vec::new();
    for size in VIEWPORTS {
        let mut app = layout_app(size);
        if embedded_assets.is_empty() {
            for (&name, handle) in &app.world().resource::<Icons>().0 {
                let asset = app.world().resource::<Assets<Image>>().get(handle).unwrap();
                assert!(asset.width() > 0 && asset.height() > 0);
                embedded_assets.push(
                    json!({"name":name,"source_path":format!("assets/ui/care_dock/{name}.png"),
                "decoded_size":[asset.width(),asset.height()]}),
                );
            }
            assert_eq!(embedded_assets.len(), EMBEDDED.len());
        }
        for &(name, availability, evidence, error) in &cases {
            apply_feedback(app.world_mut(), availability, evidence, error);
            app.update();
            app.update();
            assert_layout(app.world_mut(), size);
            let markers = app
                .world_mut()
                .query::<(Entity, &Marker)>()
                .iter(app.world())
                .map(|(e, m)| (e, m.0))
                .collect::<Vec<_>>();
            for (e, a) in markers {
                let expected = match evidence[a.index()] {
                    Evidence::None if availability[a.index()] => None,
                    Evidence::Queued => Some("pending"),
                    Evidence::Placed(_) => Some("placement"),
                    Evidence::Rejected => Some("failed"),
                    _ => Some("unavailable"),
                };
                assert_eq!(visible(app.world(), e), expected.is_some());
                if let Some(expected) = expected {
                    assert_eq!(image_name(app.world(), e), Some(expected));
                }
            }
            fixtures.push(json!({"case":name,"viewport":[size.x,size.y],"synthetic_availability":availability,
                "input_evidence":evidence.map(|e|format!("{e:?}")),"error":error,"nodes":export_nodes(app.world_mut())}));
        }
    }
    if let Ok(path) = std::env::var("ALIFE_CARE_DOCK_CPU_FIXTURE") {
        let receipt = json!({"evidence_kind":"source-derived CPU Bevy UiSurface layout","is_screenshot":false,
            "glyph_bounds_measured":false,"limitations":"CPU Bevy layout with synthetic care feedback. Checks marker, glyph, button, and alert bounds; excludes rendering and biological effects.",
            "embedded_assets":embedded_assets,"fixtures":fixtures,"composer_fixtures":composer_fixtures()});
        std::fs::write(path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    }
}
#[test]
fn embedded_cues_are_distinct_nonempty_and_never_green_success_badges() {
    let mut digests = BTreeSet::new();
    for (name, bytes) in EMBEDDED {
        assert!(!["success", "complete", "healthy", "healed"].contains(&name));
        let icon = image::load_from_memory(bytes).unwrap().to_rgba8();
        let edge = if ACTIONS.iter().any(|action| action.icon() == name) {
            64
        } else {
            16
        };
        assert_eq!(icon.dimensions(), (edge, edge));
        assert!(icon.pixels().any(|p| p[3] > 0));
        assert!(icon.pixels().any(|p| p[3] == 0));
        for p in icon.pixels().filter(|p| p[3] >= 64) {
            assert!(
                i16::from(p[1]) <= i16::from(p[0].max(p[2])) + 20,
                "green pixel in {name}: {p:?}"
            );
        }
        assert!(digests.insert(blake3::hash(bytes).to_hex().to_string()));
    }
}
#[test]
fn food_tile_preserves_valid_object_reference_with_tile_and_rejects_invalid_refs() {
    let tile = VoxelTileCoord::new(4, 7);
    assert_eq!(food_tile(None), None);
    assert_eq!(
        food_tile(Some(reference(StableVoxelRefKind::Tile, None))),
        Some(tile)
    );
    for kind in [
        StableVoxelRefKind::Creature,
        StableVoxelRefKind::Resource,
        StableVoxelRefKind::Hazard,
    ] {
        assert_eq!(
            food_tile(Some(reference(kind, Some(WorldEntityId(1))))),
            Some(tile)
        );
        assert_eq!(food_tile(Some(reference(kind, None))), None);
        assert_eq!(
            food_tile(Some(reference(kind, Some(WorldEntityId(0))))),
            None
        );
        let mut missing = reference(kind, Some(WorldEntityId(1)));
        missing.tile = None;
        assert_eq!(food_tile(Some(missing)), None);
    }
    assert_eq!(
        food_tile(Some(reference(
            StableVoxelRefKind::Tile,
            Some(WorldEntityId(1))
        ))),
        None
    );
    assert_eq!(
        food_tile(Some(reference(StableVoxelRefKind::Chunk, None))),
        None
    );
    let mut chunk = reference(StableVoxelRefKind::Chunk, None);
    chunk.tile = None;
    assert!(chunk.is_stable());
    assert_eq!(food_tile(Some(chunk)), None);
}
#[test]
fn care_target_uses_current_living_canonical_world_object_position() {
    let mut world = authoritative_world();
    let stable = world.entity_id("agent").unwrap();
    let selected = Some(reference(StableVoxelRefKind::Creature, Some(stable)));
    assert_eq!(
        care_target(&world, selected),
        Some((OrganismId(1), Vec3f::ZERO))
    );
    let moved = Vec3f::new(1.25, 2.5, 0.0);
    world.editor_move_object(stable, moved).unwrap();
    assert_eq!(
        care_target(&world, selected),
        Some((OrganismId(1), moved)),
        "stale selected tile is not the source position"
    );
    assert_eq!(care_target(&world, None), None);
    for id in [
        WorldEntityId(0),
        WorldEntityId(u64::MAX),
        world.entity_id("food").unwrap(),
        world.entity_id("unregistered").unwrap(),
    ] {
        assert_eq!(
            care_target(
                &world,
                Some(reference(StableVoxelRefKind::Creature, Some(id)))
            ),
            None
        );
    }
    let mut dead = world
        .organism_registry()
        .get(OrganismId(1))
        .unwrap()
        .clone();
    dead.mark_dead(Tick::ZERO).unwrap();
    world.replace_organism_record_exact(dead).unwrap();
    assert_eq!(care_target(&world, selected), None);
    let other = HeadlessScenarioBuilder::new(99)
        .food("same_numeric_id", Vec3f::ZERO, 0.5)
        .build()
        .unwrap();
    assert_eq!(
        care_target(&other, selected),
        None,
        "resolve against current world"
    );
}
#[test]
fn queued_evidence_never_advances_itself_and_selection_changes_clear_it() {
    let mut state = State::default();
    let selected = Some(reference(
        StableVoxelRefKind::Creature,
        Some(WorldEntityId(1)),
    ));
    state.observe_selection(selected);
    state.record(Action::Touch, Evidence::Queued);
    for _ in 0..100 {
        state.observe_selection(selected);
        assert_eq!(state.evidence[Action::Touch.index()], Evidence::Queued);
    }
    state.observe_selection(Some(reference(
        StableVoxelRefKind::Creature,
        Some(WorldEntityId(2)),
    )));
    assert_eq!(state.evidence, [Evidence::None; 5]);
    state.record(Action::Food, Evidence::Placed(WorldEntityId(41)));
    state.observe_selection(None);
    assert_eq!(state.evidence, [Evidence::None; 5]);
}
#[derive(Debug, PartialEq)]
enum Call {
    Food(Vec3f),
    Toy(OrganismId),
    Care(OrganismId, Vec3f, bool),
}
struct TestPort {
    world: HeadlessWorld,
    calls: Vec<Call>,
    reject: bool,
}
impl TestPort {
    fn new() -> Self {
        Self {
            world: authoritative_world(),
            calls: vec![],
            reject: false,
        }
    }
    fn result(&self) -> Result<WorldEntityId, GameAppShellError> {
        if self.reject {
            Err(GameAppShellError::InvalidProductionFrontend {
                message: "world declined fixture".into(),
            })
        } else {
            Ok(WorldEntityId(41))
        }
    }
}
impl CarePort for TestPort {
    fn world(&self) -> &HeadlessWorld {
        &self.world
    }
    fn food(&mut self, p: Vec3f) -> Result<WorldEntityId, GameAppShellError> {
        self.calls.push(Call::Food(p));
        self.result()
    }
    fn toy(&mut self, o: OrganismId) -> Result<WorldEntityId, GameAppShellError> {
        self.calls.push(Call::Toy(o));
        self.result()
    }
    fn care(&mut self, o: OrganismId, p: Vec3f, praise: bool) -> Result<(), GameAppShellError> {
        self.calls.push(Call::Care(o, p, praise));
        self.result().map(|_| ())
    }
}
#[test]
fn dispatcher_preserves_canonical_sources_physical_ids_and_world_rejections() {
    let mut port = TestPort::new();
    let stable = port.world.entity_id("agent").unwrap();
    let selected = Some(reference(StableVoxelRefKind::Creature, Some(stable)));
    let source = Vec3f::new(1.25, 2.5, 0.0);
    port.world.editor_move_object(stable, source).unwrap();
    for (action, expected, call) in [
        (
            Action::Food,
            Evidence::Placed(WorldEntityId(41)),
            Call::Food(Vec3f::new(4.5, 7.5, 0.0)),
        ),
        (
            Action::Touch,
            Evidence::Queued,
            Call::Care(OrganismId(1), source, false),
        ),
        (
            Action::Toy,
            Evidence::Placed(WorldEntityId(41)),
            Call::Toy(OrganismId(1)),
        ),
        (
            Action::Praise,
            Evidence::Queued,
            Call::Care(OrganismId(1), source, true),
        ),
    ] {
        assert_eq!(
            dispatch_core(&mut port, action, selected).unwrap(),
            expected
        );
        assert_eq!(port.calls.last(), Some(&call));
        port.reject = true;
        assert!(dispatch_core(&mut port, action, selected)
            .unwrap_err()
            .to_string()
            .contains("world declined fixture"));
        port.reject = false;
    }
    let before = port.calls.len();
    for action in ACTIONS {
        assert!(dispatch_core(&mut port, action, None).is_err());
    }
    assert!(dispatch_core(&mut port, Action::Speech, selected).is_err());
    assert_eq!(port.calls.len(), before);
}
#[test]
fn same_frame_pointer_and_keyboard_request_one_action_with_preserved_debug_shortcuts() {
    for action in ACTIONS {
        let mut keyboard = ButtonInput::<KeyCode>::default();
        keyboard.press(action.key());
        let mut port = TestPort::new();
        let selected = Some(reference(
            StableVoxelRefKind::Creature,
            port.world.entity_id("agent"),
        ));
        if requested(action, Some(action), &keyboard, false) {
            dispatch_core(&mut port, action, selected).unwrap();
        }
        assert_eq!(port.calls.len(), usize::from(action != Action::Speech));
        assert_eq!(
            requested(action, None, &keyboard, true),
            action == Action::Food
        );
        assert_eq!(
            requested(action, None, &keyboard, false),
            action != Action::Speech
        );
    }
}
#[derive(Resource, Default)]
struct WorldReaderRuns(u32);
fn world_reader(mut runs: ResMut<WorldReaderRuns>) {
    runs.0 += 1;
}
fn input_app() -> App {
    let mut app = App::new();
    app.insert_resource(ux())
        .init_resource::<ButtonInput<MouseButton>>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<Messages<MouseWheel>>()
        .init_resource::<menu::MenuState>()
        .init_resource::<god_hand::HandInteraction>()
        .init_resource::<WorldReaderRuns>();
    app.insert_resource(
        LiveBrainPresentationFrameResource::from_authoritative_world(&authoritative_world())
            .unwrap(),
    );
    spawn(app.world_mut());
    app.add_systems(
        Update,
        (
            collect_input,
            menu::handle_input,
            world_reader
                .run_if(world_input_allowed)
                .run_if(menu::world_input_allowed),
        )
            .chain(),
    );
    app
}
fn action_entity(world: &mut World, action: Action) -> Entity {
    world
        .query::<(Entity, &Action)>()
        .iter(world)
        .find_map(|(e, a)| (*a == action).then_some(e))
        .unwrap()
}
#[test]
fn collected_pointer_care_resolves_dispatch_selection_and_rejects_retired_snapshot_targets() {
    #[derive(Debug, Clone, Copy)]
    enum Change {
        SelectOther,
        ClearSelection,
        Die,
        Retire,
    }
    for action in [Action::Food, Action::Touch, Action::Toy, Action::Praise] {
        for change in [
            Change::SelectOther,
            Change::ClearSelection,
            Change::Die,
            Change::Retire,
        ] {
            let mut port = TestPort::new();
            let first = port.world.entity_id("agent").unwrap();
            let other_organism = OrganismId(37);
            let second = port
                .world
                .spawn_social_agent(
                    "dispatch-target",
                    other_organism,
                    Vec3f::new(6.0, 2.0, 0.0),
                    0.0,
                )
                .unwrap();
            assert_ne!(
                second.raw(),
                other_organism.raw(),
                "body and organism IDs are separate domains"
            );
            let template = port
                .world
                .organism_registry()
                .get(OrganismId(1))
                .unwrap()
                .clone();
            port.world
                .register_organism_record(
                    WorldOrganismRecord::new(
                        other_organism,
                        second,
                        template.genome().clone(),
                        template.phenotype().clone(),
                        *template.biochemistry(),
                        Tick::ZERO,
                    )
                    .unwrap(),
                )
                .unwrap();
            let pressed_selection = Some(reference(StableVoxelRefKind::Creature, Some(first)));
            let mut app = input_app();
            app.insert_resource(Fvr03ProductionVoxelSelectionResource {
                hovered: None,
                selected: pressed_selection,
            });
            app.insert_resource(
                LiveBrainPresentationFrameResource::from_authoritative_world(&port.world).unwrap(),
            );
            app.world_mut()
                .resource_mut::<State>()
                .observe_selection(pressed_selection);
            app.world_mut()
                .resource_mut::<State>()
                .record(action, Evidence::Queued);
            let button = action_entity(app.world_mut(), action);
            app.world_mut()
                .entity_mut(button)
                .insert(Interaction::Pressed);
            app.world_mut()
                .resource_mut::<ButtonInput<MouseButton>>()
                .press(MouseButton::Left);
            collect_input(app.world_mut());
            assert!(app.world().resource::<State>().captured);
            assert_eq!(app.world().resource::<State>().pending, Some(action));

            let moved = Vec3f::new(8.25, 3.5, 0.0);
            match change {
                Change::SelectOther => {
                    let mut current = reference(StableVoxelRefKind::Creature, Some(second));
                    current.tile = Some(VoxelTileCoord::new(9, 11));
                    app.world_mut()
                        .resource_mut::<Fvr03ProductionVoxelSelectionResource>()
                        .selected = Some(current);
                    port.world.editor_move_object(second, moved).unwrap();
                    assert_ne!(
                        app.world()
                            .resource::<LiveBrainPresentationFrameResource>()
                            .current
                            .object(second)
                            .unwrap()
                            .position,
                        moved
                    );
                }
                Change::ClearSelection => {
                    app.world_mut()
                        .resource_mut::<Fvr03ProductionVoxelSelectionResource>()
                        .selected = None;
                }
                Change::Die | Change::Retire => {
                    let mut dead = template;
                    dead.mark_dead(Tick::ZERO).unwrap();
                    port.world.replace_organism_record_exact(dead).unwrap();
                    if matches!(change, Change::Retire) {
                        port.world
                            .link_birth_manifest(
                                OrganismId(1),
                                alife_core::Blake3Digest::from_bytes([1; 32]),
                            )
                            .unwrap();
                        port.world
                            .link_life_manifest(
                                OrganismId(1),
                                alife_core::Blake3Digest::from_bytes([2; 32]),
                            )
                            .unwrap();
                        port.world.retire_dead_organism(OrganismId(1)).unwrap();
                        assert!(port.world.entity(first).is_none());
                        assert!(port.world.organism_registry().get(OrganismId(1)).is_none());
                    }
                    // A stale renderer row and selected tile remain; neither can authorize care.
                    assert!(app
                        .world()
                        .resource::<LiveBrainPresentationFrameResource>()
                        .current
                        .object(first)
                        .is_some());
                    assert_eq!(care_target(&port.world, pressed_selection), None);
                }
            }
            let selected = app
                .world()
                .resource::<Fvr03ProductionVoxelSelectionResource>()
                .selected;
            let mut state = app.world_mut().resource_mut::<State>();
            state.observe_selection(selected);
            if matches!(change, Change::SelectOther | Change::ClearSelection) {
                assert_eq!(state.evidence, [Evidence::None; 5]);
            }
            let pointer = state.take_pending();
            assert_eq!(
                pointer,
                Some(action),
                "selection changes keep the action, not the old target"
            );
            assert_eq!(state.take_pending(), None);
            let keyboard = app.world().resource::<ButtonInput<KeyCode>>();
            assert!(requested(action, pointer, keyboard, false));
            let result = dispatch_core(&mut port, action, selected);
            if matches!(change, Change::ClearSelection)
                || (action != Action::Food && matches!(change, Change::Die | Change::Retire))
            {
                assert!(result.is_err(), "{action:?}, {change:?}");
                assert!(
                    port.calls.is_empty(),
                    "invalid targets must not reach the runtime port"
                );
            } else {
                let (evidence, call) = match action {
                    Action::Food => {
                        // Food remains a tile-placement command even after the selected body retires.
                        let position = if matches!(change, Change::SelectOther) {
                            Vec3f::new(9.5, 11.5, 0.0)
                        } else {
                            Vec3f::new(4.5, 7.5, 0.0)
                        };
                        (Evidence::Placed(WorldEntityId(41)), Call::Food(position))
                    }
                    Action::Toy => (
                        Evidence::Placed(WorldEntityId(41)),
                        Call::Toy(other_organism),
                    ),
                    Action::Touch | Action::Praise => (
                        Evidence::Queued,
                        Call::Care(other_organism, moved, action == Action::Praise),
                    ),
                    Action::Speech => unreachable!(),
                };
                assert_eq!(result.unwrap(), evidence, "{action:?}, {change:?}");
                assert_eq!(port.calls, vec![call], "{action:?}, {change:?}");
            }
        }
    }
}
#[test]
fn actual_collect_input_captures_hover_press_wheel_and_release_without_repeat_dispatch() {
    let mut app = input_app();
    app.update();
    let entity = action_entity(app.world_mut(), Action::Touch);
    app.world_mut()
        .entity_mut(entity)
        .insert(Interaction::Hovered);
    app.world_mut()
        .resource_mut::<Messages<MouseWheel>>()
        .write(MouseWheel {
            unit: MouseScrollUnit::Line,
            x: 0.0,
            y: 1.0,
            window: Entity::PLACEHOLDER,
        });
    let before = app.world().resource::<WorldReaderRuns>().0;
    app.update();
    assert!(app.world().resource::<State>().captured);
    assert_eq!(app.world().resource::<WorldReaderRuns>().0, before);
    assert!(app.world().resource::<Messages<MouseWheel>>().is_empty());
    assert_eq!(app.world_mut().resource_mut::<State>().take_pending(), None);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.world_mut()
        .entity_mut(entity)
        .insert(Interaction::Pressed);
    app.world_mut()
        .resource_mut::<god_hand::HandInteraction>()
        .held = Some(WorldEntityId(41));
    app.update();
    assert_eq!(
        app.world_mut().resource_mut::<State>().take_pending(),
        Some(Action::Touch)
    );
    assert_eq!(
        app.world().resource::<god_hand::HandInteraction>().held,
        None
    );
    app.update();
    assert_eq!(
        app.world_mut().resource_mut::<State>().take_pending(),
        None,
        "held Interaction is not a second action"
    );
    let toy = action_entity(app.world_mut(), Action::Toy);
    app.world_mut().entity_mut(entity).insert(Interaction::None);
    app.world_mut().entity_mut(toy).insert(Interaction::Pressed);
    app.update();
    assert_eq!(
        app.world_mut().resource_mut::<State>().take_pending(),
        None,
        "dragging held click across buttons is not a new action"
    );
    app.world_mut().entity_mut(toy).insert(Interaction::None);
    app.update();
    assert!(app.world().resource::<State>().captured);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .release(MouseButton::Left);
    app.update();
    assert!(!app.world().resource::<State>().captured);
    assert_eq!(app.world().resource::<WorldReaderRuns>().0, before + 1);
}
#[test]
fn actual_collect_input_respects_debug_and_open_menu_and_keeps_speech_out_of_care_queue() {
    let mut app = input_app();
    app.update();
    let food = action_entity(app.world_mut(), Action::Food);
    app.world_mut()
        .resource_mut::<Fvr05ProductionUxStateResource>()
        .debug_mode = true;
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.world_mut()
        .entity_mut(food)
        .insert(Interaction::Pressed);
    app.update();
    assert_eq!(app.world_mut().resource_mut::<State>().take_pending(), None);
    assert!(!app.world().resource::<State>().captured);
    app.world_mut()
        .resource_mut::<Fvr05ProductionUxStateResource>()
        .debug_mode = false;
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    menu::handle_input(app.world_mut());
    assert!(menu::is_open(app.world().resource::<menu::MenuState>()));
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    app.update();
    assert_eq!(app.world_mut().resource_mut::<State>().take_pending(), None);
    app.world_mut().insert_resource(menu::MenuState::default());
    app.world_mut().entity_mut(food).insert(Interaction::None);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .release(MouseButton::Left);
    app.update();
    let speech = action_entity(app.world_mut(), Action::Speech);
    app.world_mut()
        .entity_mut(speech)
        .insert(Interaction::Pressed);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.update();
    assert_eq!(app.world_mut().resource_mut::<State>().take_pending(), None);
    assert!(app.world().resource::<State>().captured);
    assert_eq!(
        app.world().resource::<State>().evidence[Action::Speech.index()],
        Evidence::Unavailable
    );
    assert!(app
        .world()
        .resource::<Fvr05ProductionUxStateResource>()
        .last_error
        .is_some());
}

#[test]
fn escape_cancels_food_move_and_menu_captures_dismissal_until_all_inputs_release() {
    for paused in [false, true] {
        let mut app = input_app();
        let selected = Some(reference(
            StableVoxelRefKind::Creature,
            Some(WorldEntityId(1)),
        ));
        app.insert_resource(Fvr03ProductionVoxelSelectionResource {
            hovered: selected,
            selected,
        });
        app.world_mut()
            .resource_mut::<Fvr05ProductionUxStateResource>()
            .settings
            .paused = paused;
        app.update();
        let before = app.world().resource::<WorldReaderRuns>().0;
        app.world_mut()
            .resource_mut::<Fvr05ProductionUxStateResource>()
            .pending_food_move = Some(WorldEntityId(41));
        app.world_mut()
            .resource_mut::<god_hand::HandInteraction>()
            .held = Some(WorldEntityId(41));
        for key in [KeyCode::Escape, KeyCode::KeyC, KeyCode::ArrowRight] {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(key);
        }
        app.world_mut()
            .resource_mut::<Messages<MouseWheel>>()
            .write(MouseWheel {
                unit: MouseScrollUnit::Line,
                x: 0.0,
                y: 1.0,
                window: Entity::PLACEHOLDER,
            });
        app.update();
        assert!(menu::is_open(app.world().resource::<menu::MenuState>()));
        assert_eq!(app.world().resource::<WorldReaderRuns>().0, before);
        assert!(app.world().resource::<Messages<MouseWheel>>().is_empty());
        assert_eq!(
            app.world()
                .resource::<Fvr05ProductionUxStateResource>()
                .pending_food_move,
            None
        );
        assert_eq!(
            app.world().resource::<god_hand::HandInteraction>().held,
            None
        );
        assert!(
            app.world()
                .resource::<Fvr05ProductionUxStateResource>()
                .settings
                .paused
        );

        // A dock press while modal never queues care. Keep it and the arrow held
        // across Escape dismissal, including after the pointer leaves the dock.
        let touch = action_entity(app.world_mut(), Action::Touch);
        app.world_mut()
            .entity_mut(touch)
            .insert(Interaction::Pressed);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        {
            let mut keyboard = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keyboard.clear();
            keyboard.release(KeyCode::Escape);
        }
        app.update();
        assert_eq!(app.world_mut().resource_mut::<State>().take_pending(), None);
        assert_eq!(app.world().resource::<WorldReaderRuns>().0, before);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        assert!(!menu::is_open(app.world().resource::<menu::MenuState>()));
        assert!(menu::is_captured(app.world().resource::<menu::MenuState>()));
        assert_eq!(app.world().resource::<WorldReaderRuns>().0, before);
        assert_eq!(
            app.world()
                .resource::<Fvr05ProductionUxStateResource>()
                .settings
                .paused,
            paused
        );
        app.world_mut().entity_mut(touch).insert(Interaction::None);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        app.update();
        assert_eq!(app.world().resource::<WorldReaderRuns>().0, before);
        assert_eq!(app.world_mut().resource_mut::<State>().take_pending(), None);

        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        app.update();
        assert_eq!(app.world().resource::<WorldReaderRuns>().0, before);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        app.update();
        assert!(!menu::is_captured(
            app.world().resource::<menu::MenuState>()
        ));
        assert_eq!(app.world().resource::<WorldReaderRuns>().0, before + 1);
        assert_eq!(
            app.world()
                .resource::<Fvr03ProductionVoxelSelectionResource>()
                .selected,
            selected
        );

        app.world_mut()
            .entity_mut(touch)
            .insert(Interaction::Pressed);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        assert_eq!(
            app.world_mut().resource_mut::<State>().take_pending(),
            Some(Action::Touch),
            "a fresh deliberate press works after complete release"
        );
    }
}
#[test]
fn dock_uses_its_layer_and_is_hidden_only_in_debug() {
    let mut app = layout_app(Vec2::new(320.0, 240.0));
    let root = app
        .world_mut()
        .query_filtered::<Entity, With<DockRoot>>()
        .single(app.world())
        .unwrap();
    assert_eq!(app.world().get::<GlobalZIndex>(root).unwrap().0, 60);
    for debug in [true, false, true, false] {
        app.world_mut()
            .resource_mut::<Fvr05ProductionUxStateResource>()
            .debug_mode = debug;
        sync(app.world_mut());
        assert_eq!(visible(app.world(), root), !debug);
    }
}

#[cfg(feature = "gpu-runtime")]
fn conversation_state() -> ProductionConversationLineageUiState {
    let ui = ux();
    let summary = ProductionVoxelLaunchSummary {
        schema: "UI-only fixture",
        schema_version: 1,
        window_title: "UI-only fixture".into(),
        scenario_id: "UI-only fixture".into(),
        profile_id: ui.profile_id,
        profile_budget: ui.profile_budget,
        effective_population: 0,
        resolution: ui.resolution,
        state_trace: vec![],
        renderer_profile: String::new(),
        save_path: PathBuf::new(),
        config_path: PathBuf::new(),
        asset_manifest_path: PathBuf::new(),
        production_asset_manifest_path: PathBuf::new(),
        production_asset_count: 0,
        production_asset_license_metadata_validated: false,
        asset_root: PathBuf::new(),
        diagnostics: crate::ProductionRuntimeDiagnostics {
            requested_backend: "GpuAuthoritative".into(),
            selected_backend: "Unavailable".into(),
            adapter_name: None,
            backend_api: None,
            adapter_type: None,
            driver: None,
            driver_info: None,
            unavailable_reason: Some("CPU UI fixture".into()),
            renderer_profile: String::new(),
            save_path: PathBuf::new(),
            asset_manifest_path: PathBuf::new(),
            graphics_backend: String::new(),
            require_gpu: false,
            unavailable_state_visible: true,
            authoritative: false,
            failure_stops_learned_actions: true,
        },
        save_metadata: ui.save_metadata,
        real_save_loaded: false,
        mock_data_source: true,
        dry_run: true,
        record_performance: false,
        developer_overlay: false,
        ui_settings_path: PathBuf::new(),
        ui_settings: ui.settings,
        ui_settings_load_error: None,
        debug_authority: ui.authority,
        gpu_runtime_state: ui.gpu_runtime_state,
        gpu_gameplay_receipt: None,
    };
    ProductionConversationLineageUiState::new(&summary)
}

#[cfg(not(feature = "gpu-runtime"))]
fn composer_fixtures() -> Vec<Value> {
    vec![]
}

#[cfg(feature = "gpu-runtime")]
fn composer_fixtures() -> Vec<Value> {
    use crate::production_conversation_lineage_ui::{
        adapt_speech_composer, spawn_speech_entry, speech_entry_text,
        ProductionSpeechComposerBounds,
    };
    let mut fixtures = Vec::new();
    for size in VIEWPORTS {
        for (case, error, long) in [
            ("recorded_speech", false, false),
            ("recorded_speech_with_error", true, false),
            ("unbroken_512_speech", false, true),
            ("unbroken_512_speech_with_error", true, true),
        ] {
            let mut app = layout_app(size);
            let mut state = conversation_state();
            state.prepare_recorded_speech_capture();
            let input = if long {
                "x".repeat(512)
            } else {
                "what are you doing?".to_owned()
            };
            if long {
                state.prepare_typed_speech_fixture(&input);
            }
            let text = speech_entry_text(&state);
            assert!(text.contains(&format!("{input}|")));
            app.insert_resource(state);
            if error {
                app.world_mut()
                    .resource_mut::<Fvr05ProductionUxStateResource>()
                    .last_error = Some("Synthetic world rejection".into());
            }
            sync(app.world_mut());
            spawn_speech_entry(app.world_mut());
            let bounds = app
                .world_mut()
                .query_filtered::<Entity, With<ProductionSpeechComposerBounds>>()
                .single(app.world())
                .unwrap();
            let entry = app.world_mut().query_filtered::<Entity, With<crate::production_conversation_lineage_ui::ProductionSpeechEntryPanel>>()
                .single(app.world()).unwrap();
            let viewport = app.world_mut().query_filtered::<Entity, With<crate::production_conversation_lineage_ui::composer::DraftViewport>>()
                .single(app.world()).unwrap();
            app.world_mut().get_mut::<Text>(entry).unwrap().0 = text.clone();
            adapt_speech_composer(app.world_mut());
            assert_eq!(
                app.world()
                    .get::<bevy::ui::ScrollPosition>(viewport)
                    .unwrap()
                    .0
                    .y,
                1_000_000.0,
                "open composer requests scrolling toward existing end caret"
            );
            app.update();
            app.update();
            let composer = Box2::of(app.world(), bounds);
            let root = app
                .world_mut()
                .query_filtered::<Entity, With<DockRoot>>()
                .single(app.world())
                .unwrap();
            let dock = Box2::of(app.world(), root);
            assert!(Box2 {
                min: Vec2::ZERO,
                max: size
            }
            .contains(composer));
            assert!(composer.size().x > 0.0 && composer.size().y > 0.0);
            assert!(!composer.overlaps(dock));
            assert!(
                composer.max.y <= dock.min.y - 8.0 + EPSILON,
                "composer needs gap above dock at {size:?}"
            );
            let alert = app
                .world_mut()
                .query_filtered::<Entity, With<ErrorMarker>>()
                .single(app.world())
                .unwrap();
            let alert_box = Box2::of(app.world(), alert);
            assert_eq!(visible(app.world(), alert), error);
            if error {
                assert!(
                    !composer.overlaps(alert_box),
                    "composer overlaps visible alert gutter"
                );
                assert!(composer.max.y <= alert_box.min.y - 8.0 + EPSILON);
            }
            assert!(visible(app.world(), bounds));
            assert_eq!(
                app.world().get::<Text>(entry).unwrap().0,
                text,
                "layout preserves all entered teaching text"
            );
            assert_eq!(
                app.world().get::<TextLayout>(entry).unwrap().linebreak,
                bevy::text::LineBreak::WordOrCharacter
            );
            let node = app.world().get::<Node>(viewport).unwrap();
            assert_eq!(node.overflow.x, bevy::ui::OverflowAxis::Clip);
            assert_eq!(node.overflow.y, bevy::ui::OverflowAxis::Scroll);
            assert_eq!(
                app.world().get::<Node>(bounds).unwrap().bottom,
                Val::Px(if error { 108.0 } else { 92.0 })
            );
            assert!(occupied_top(app.world_mut(), size) <= composer.min.y + EPSILON);
            fixtures.push(json!({"case":case, "viewport":[size.x,size.y], "error":error,
                "composer_box":composer.json(), "dock_box":dock.json(), "alert_box":alert_box.json(),
                "typed_input_preserved":input, "glyph_bounds_measured":false,
                "limitations":"CPU Bevy layout checks parent bounds, clipping, text styles, height budget, and end-caret scroll target; excludes glyph shaping and rendering.",
                "nodes":export_nodes(app.world_mut())}));
            app.world_mut()
                .resource_mut::<ProductionConversationLineageUiState>()
                .clear_recorded_capture();
            adapt_speech_composer(app.world_mut());
            assert!(!visible(app.world(), bounds));
            assert_eq!(
                app.world()
                    .get::<bevy::ui::ScrollPosition>(viewport)
                    .unwrap()
                    .0
                    .y,
                0.0
            );
        }
    }
    fixtures
}

#[cfg(feature = "gpu-runtime")]
#[test]
fn actual_composer_bounds_fit_supported_viewports_and_capture_preserves_entered_text() {
    assert_eq!(composer_fixtures().len(), VIEWPORTS.len() * 4);
    let mut app = input_app();
    let mut state = conversation_state();
    state.prepare_recorded_speech_capture();
    let before = crate::production_conversation_lineage_ui::speech_entry_text(&state);
    app.insert_resource(state);
    let touch = action_entity(app.world_mut(), Action::Touch);
    app.world_mut()
        .entity_mut(touch)
        .insert(Interaction::Pressed);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.update();
    assert_eq!(app.world_mut().resource_mut::<State>().take_pending(), None);
    assert_eq!(
        crate::production_conversation_lineage_ui::speech_entry_text(
            app.world()
                .resource::<ProductionConversationLineageUiState>()
        ),
        before
    );
}

#[test]
fn unavailable_frontend_dispatch_never_invents_completion_or_biological_success() {
    let mut ui = ux();
    #[cfg(feature = "gpu-runtime")]
    let mut frame =
        LiveBrainPresentationFrameResource::from_authoritative_world(&authoritative_world())
            .unwrap();
    for action in ACTIONS {
        let evidence = dispatch(
            action,
            Some(reference(
                StableVoxelRefKind::Creature,
                Some(WorldEntityId(1)),
            )),
            &mut ui,
            #[cfg(feature = "gpu-runtime")]
            None,
            #[cfg(feature = "gpu-runtime")]
            &mut frame,
        );
        assert_eq!(evidence, Evidence::Unavailable);
        assert!(ui
            .last_error
            .as_deref()
            .unwrap()
            .contains("GPU runtime unavailable"));
        assert!(ui.last_action.ends_with("unavailable"));
        assert!(!ui.last_action.to_lowercase().contains("success"));
        assert!(!ui.last_action.to_lowercase().contains("complete"));
    }
}

#[derive(Component)]
struct CpuFocusModal;

// Explicit CPU fixture: hierarchy-order UiStack and ancestor-derived
// InheritedVisibility are supplied without renderer/extraction plugins. The
// hit-test, cursor conversion, clipping, Interaction updates and input capture
// themselves use Bevy's production ui_focus_system and collect_input.
pub(crate) fn cpu_focus_fixture_stack(world: &mut World) {
    fn walk(world: &World, entity: Entity, nodes: &mut Vec<Entity>) {
        nodes.push(entity);
        if let Some(children) = world.get::<Children>(entity) {
            for child in children.iter() {
                walk(world, child, nodes);
            }
        }
    }
    let root = world
        .query_filtered::<Entity, With<DockRoot>>()
        .single(world)
        .unwrap();
    let mut nodes = Vec::new();
    walk(world, root, &mut nodes);
    let composers = world.query_filtered::<Entity, With<crate::production_conversation_lineage_ui::ProductionSpeechComposerBounds>>().iter(world).collect::<Vec<_>>();
    for composer in composers {
        walk(world, composer, &mut nodes);
    }
    let modals = world
        .query_filtered::<Entity, With<CpuFocusModal>>()
        .iter(world)
        .collect::<Vec<_>>();
    for modal in modals {
        walk(world, modal, &mut nodes);
    }
    for &entity in &nodes {
        let inherited = if visible(world, entity) {
            bevy::camera::visibility::InheritedVisibility::VISIBLE
        } else {
            bevy::camera::visibility::InheritedVisibility::HIDDEN
        };
        world.entity_mut(entity).insert(inherited);
    }
    world.insert_resource(bevy::ui::UiStack {
        partition: std::iter::once(0..nodes.len()).collect(),
        uinodes: nodes,
    });
}
pub(crate) fn focus_cursor(world: &mut World, point: Vec2) {
    let window = world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(world)
        .unwrap();
    world
        .get_mut::<Window>(window)
        .unwrap()
        .set_cursor_position(Some(point));
}
fn focus_center(world: &World, entity: Entity) -> Vec2 {
    let bounds = Box2::of(world, entity);
    (bounds.min + bounds.max) * 0.5
}

#[test]
fn actual_cpu_ui_focus_hit_tests_buttons_gutter_and_modal_before_world_reader() {
    // Worst supported viewport. Four-viewport geometry is independently checked
    // above; this is a bounded CPU dispatch fixture, not full live integration.
    let size = Vec2::new(320.0, 240.0);
    let mut app = layout_app(size);
    app.update();
    app.update();
    app.init_resource::<ButtonInput<MouseButton>>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<bevy::input::touch::Touches>()
        .init_resource::<Messages<MouseWheel>>()
        .init_resource::<menu::MenuState>()
        .init_resource::<god_hand::HandInteraction>()
        .init_resource::<WorldReaderRuns>();
    app.insert_resource(
        LiveBrainPresentationFrameResource::from_authoritative_world(&authoritative_world())
            .unwrap(),
    );
    let camera = app
        .world_mut()
        .query_filtered::<Entity, With<Camera>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .entity_mut(camera)
        .insert(bevy::camera::RenderTarget::default());
    app.add_systems(
        Update,
        (
            cpu_focus_fixture_stack,
            bevy::ui::ui_focus_system,
            collect_input,
            world_reader
                .run_if(world_input_allowed)
                .run_if(menu::world_input_allowed),
        )
            .chain(),
    );
    let food = action_entity(app.world_mut(), Action::Food);
    let speech = action_entity(app.world_mut(), Action::Speech);
    let root = app
        .world_mut()
        .query_filtered::<Entity, With<DockRoot>>()
        .single(app.world())
        .unwrap();
    let food_point = focus_center(app.world(), food);
    let speech_point = focus_center(app.world(), speech);
    let dock = Box2::of(app.world(), root);
    let gutter_point = Vec2::new(dock.min.x + 6.0, (dock.min.y + dock.max.y) * 0.5);
    let outside = Vec2::new(4.0, 4.0);
    for (point, target, expected) in [
        (food_point, food, Some(Action::Food)),
        (speech_point, speech, None),
        (gutter_point, root, None),
    ] {
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
        focus_cursor(app.world_mut(), point);
        app.update();
        assert_eq!(
            app.world().get::<Interaction>(target),
            Some(&Interaction::Hovered)
        );
        let before = app.world().resource::<WorldReaderRuns>().0;
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        assert_eq!(
            app.world().get::<Interaction>(target),
            Some(&Interaction::Pressed)
        );
        assert_eq!(
            app.world_mut().resource_mut::<State>().take_pending(),
            expected
        );
        assert!(app.world().resource::<State>().captured);
        assert_eq!(app.world().resource::<WorldReaderRuns>().0, before);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
        focus_cursor(app.world_mut(), outside);
        app.update();
        assert!(
            app.world().resource::<State>().captured,
            "held press remains captured outside dock"
        );
        assert_eq!(app.world_mut().resource_mut::<State>().take_pending(), None);
        assert_eq!(app.world().resource::<WorldReaderRuns>().0, before);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        app.update();
        assert!(!app.world().resource::<State>().captured);
        assert_eq!(app.world().resource::<WorldReaderRuns>().0, before + 1);
    }
    // A manual 90-layer modal node sits above the actual dock stack. Opening the
    // real MenuState also gates the downstream reader when that node is hidden.
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .clear();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    menu::handle_input(app.world_mut());
    assert!(menu::is_open(app.world().resource::<menu::MenuState>()));
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    let modal = app
        .world_mut()
        .spawn((
            CpuFocusModal,
            Button,
            FocusPolicy::Block,
            GlobalZIndex(90),
            Visibility::Inherited,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Px(size.x),
                height: Val::Px(size.y),
                ..default()
            },
        ))
        .id();
    app.update(); // Supply actual modal layout/target-camera components.
    let before = app.world().resource::<WorldReaderRuns>().0;
    focus_cursor(app.world_mut(), food_point);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.update();
    assert_eq!(
        app.world().get::<Interaction>(modal),
        Some(&Interaction::Pressed)
    );
    assert_eq!(
        app.world().get::<Interaction>(food),
        Some(&Interaction::None)
    );
    assert_eq!(app.world_mut().resource_mut::<State>().take_pending(), None);
    assert_eq!(app.world().resource::<WorldReaderRuns>().0, before);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .clear();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .release(MouseButton::Left);
    app.world_mut().entity_mut(modal).insert(Visibility::Hidden);
    app.update();
    assert_eq!(
        app.world().get::<Interaction>(modal),
        Some(&Interaction::None)
    );
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .clear();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.update();
    assert_eq!(
        app.world().get::<Interaction>(food),
        Some(&Interaction::Pressed)
    );
    assert_eq!(
        app.world_mut().resource_mut::<State>().take_pending(),
        None,
        "open menu blocks care dispatch even when its visual fixture is hidden"
    );
    assert_eq!(app.world().resource::<WorldReaderRuns>().0, before);
    app.world_mut().entity_mut(root).insert(Visibility::Hidden);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .clear();
    app.update();
    assert_eq!(
        app.world().get::<Interaction>(food),
        Some(&Interaction::None)
    );
    assert_eq!(
        app.world().get::<Interaction>(root),
        Some(&Interaction::None)
    );
    assert_eq!(app.world_mut().resource_mut::<State>().take_pending(), None);
    assert_eq!(app.world().resource::<WorldReaderRuns>().0, before);
}
