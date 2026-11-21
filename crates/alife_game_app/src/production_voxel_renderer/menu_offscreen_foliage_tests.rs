//! Public-only Bevy presentation fixture; no production or neural runtime.
use super::*;
use accesskit::{Action as KitAction, ActionData, ActionRequest as KitRequest, NodeId};
use bevy::{
    a11y::ActionRequest,
    app::PluginsState,
    asset::{AssetId, AssetPlugin},
    camera::{NormalizedRenderTarget, OrthographicProjection, RenderTarget, ScalingMode},
    core_pipeline::tonemapping::Tonemapping,
    ecs::message::Messages,
    gltf::{Gltf, GltfMesh},
    prelude::{DefaultPlugins, PluginGroup},
    render::{
        RenderApp, RenderPlugin,
        camera::ExtractedCamera,
        pipelined_rendering::PipelinedRenderingPlugin,
        render_asset::RenderAssets,
        render_resource::{TextureFormat, TextureUsages},
        renderer::RenderAdapterInfo,
        settings::{Backends, RenderCreation, WgpuSettings},
        texture::GpuImage,
        view::{Msaa, screenshot::{Screenshot, ScreenshotCaptured}},
    },
    ui::IsDefaultUiCamera,
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use std::{path::{Path, PathBuf}, thread, time::{Duration, Instant}};

const WIDTH: u32 = 512;
const HEIGHT: u32 = 320;
const FORMAT: TextureFormat = TextureFormat::Bgra8UnormSrgb;
type Witness = (&'static str, [u32; 2]);
const TAGGED: [Witness; 2] = [
    ("Island Fern", [0x42a8_a01e, 0x4359_7f7e]), // original placement 3555
    ("Island Grass", [0x42a7_162e, 0x4358_8a0c]), // original placement 3708
];
const OTHER: Witness = ("Island Sapling", [0x42a5_3d50, 0x4354_2ed0]); // 447

#[derive(Resource, Default)]
struct Captured(Option<(Entity, Image)>);

#[derive(Resource, Default)]
struct Isolation(Vec<Entity>);

fn isolate(mask: Res<Isolation>, mut visible: Query<&mut Visibility>) {
    for entity in &mask.0 {
        *visible.get_mut(*entity).expect("lost exact stock witness") = Visibility::Hidden;
    }
}

fn drive_until(app: &mut App, label: &str, mut check: impl FnMut(&mut App) -> bool) {
    let cutoff = Instant::now() + Duration::from_secs(40);
    while !check(app) {
        assert!(Instant::now() < cutoff, "unqualified render fixture: {label}");
        app.update();
        thread::sleep(Duration::from_millis(4));
    }
}

fn target(world: &mut World, width: u32, height: u32) -> Handle<Image> {
    let mut image = Image::new_target_texture(width, height, FORMAT, None);
    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    world.resource_mut::<Assets<Image>>().add(image)
}

struct Cut {
    target: AssetId<Image>,
    request: Entity,
    bytes: Vec<u8>,
}

fn extracted_view(app: &mut App, target: AssetId<Image>) -> bool {
    let render = app.sub_app_mut(RenderApp).world_mut();
    if !render.get_resource::<RenderAssets<GpuImage>>()
        .is_some_and(|assets| assets.get(target).is_some())
    {
        return false;
    }
    let mut cameras = render.query::<&ExtractedCamera>();
    cameras.iter(render).filter(|camera| {
        camera.physical_target_size == Some(UVec2::new(WIDTH, HEIGHT))
            && matches!(&camera.target, Some(NormalizedRenderTarget::Image(t)) if t.handle.id() == target)
    }).count() == 1
}

struct SceneAnchor {
    camera: Entity,
    pose: Transform,
    terrain: Entity,
    terrain_l0: AssetId<Mesh>,
}

impl SceneAnchor {
    fn check(&self, world: &World) {
        assert_eq!(world.get::<Transform>(self.camera), Some(&self.pose));
        assert!(world.get::<Camera>(self.camera).is_some_and(|camera| camera.is_active));
        let Projection::Orthographic(projection) = world.get::<Projection>(self.camera).unwrap() else {
            panic!("changed camera projection");
        };
        assert!(matches!(projection.scaling_mode, ScalingMode::FixedVertical { viewport_height }
            if viewport_height == 14.0));
        assert_eq!(world.get::<Mesh3d>(self.terrain).unwrap().0.id(), self.terrain_l0);
        assert_eq!(world.resource::<MenuState>().screen, Screen::Closed);
    }
}

fn cut(app: &mut App, scene: &SceneAnchor) -> Cut {
    scene.check(app.world());
    let previous_detent = app.world().resource::<Fvr05ProductionUxStateResource>().settings.foliage_density;
    let previous_mask = app.world().resource::<Isolation>().0.clone();
    assert!(app.world().resource::<Captured>().0.is_none());
    let image = target(app.world_mut(), WIDTH, HEIGHT);
    let id = image.id();
    app.world_mut().entity_mut(scene.camera).insert(RenderTarget::Image(image.clone().into()));
    drive_until(app, "no exact extracted camera and prepared target for this cut", |app| {
        extracted_view(app, id)
    });
    // The stock Screenshot render system binds this unique GPU attachment
    // after the render graph and copies its output for this request entity.
    let request = app.world_mut().spawn(Screenshot::image(image)).observe(
        |event: On<ScreenshotCaptured>, mut output: ResMut<Captured>| {
            assert!(output.0.is_none(), "duplicate/unconsumed screenshot");
            output.0 = Some((event.entity, event.image.clone()));
        },
    ).id();
    drive_until(app, "no renderer screenshot for this target", |a| {
        a.world().resource::<Captured>().0.is_some()
    });
    let (entity, image) = app.world_mut().resource_mut::<Captured>().0.take().unwrap();
    assert_eq!(entity, request, "stale or differently scoped screenshot");
    assert_eq!(image.size(), UVec2::new(WIDTH, HEIGHT));
    assert_eq!(image.texture_descriptor.format, FORMAT);
    scene.check(app.world());
    assert!(matches!(app.world().get::<RenderTarget>(scene.camera),
        Some(RenderTarget::Image(t)) if t.handle.id() == id));
    assert!(extracted_view(app, id));
    let world = app.world();
    assert_eq!(world.resource::<Fvr05ProductionUxStateResource>().settings.foliage_density, previous_detent);
    assert_eq!(world.resource::<Isolation>().0, previous_mask);
    assert!(previous_mask.iter().all(|&e| world.get::<Visibility>(e) == Some(&Visibility::Hidden)));
    let bytes = image.data.expect("render readback has no byte payload");
    assert_eq!(bytes.len(), (WIDTH * HEIGHT * 4) as usize);
    Cut { target: id, request, bytes }
}

fn input(app: &mut App, screen: Screen, action: Action, value: Option<u8>) {
    assert_eq!(app.world().resource::<MenuState>().screen, screen);
    let world = app.world_mut();
    let mut buttons = world.query::<(Entity, &MenuButton, &AccessibilityNode)>();
    let (id, node) = buttons.iter(world)
        .find_map(|(id, button, node)| {
            (button.screen == screen && button.action == action).then_some((id, node))
        })
        .expect("request must hit a real stock menu control");
    assert!(!node.is_hidden() && !node.is_disabled());
    let (action, data) = value.map_or((KitAction::Click, None), |value| {
        (KitAction::SetValue, Some(ActionData::NumericValue(f64::from(value))))
    });
    assert!(node.supports_action(action));
    world.resource_mut::<Messages<ActionRequest>>().write(ActionRequest(KitRequest {
        action, data, target: NodeId(id.to_bits()),
    }));
    // The request is sourced from the actual UI widget. Normal menu systems
    // consume it, release the modal transition, and update the visible page.
    for _ in 0..3 { app.update(); }
}

fn detent(app: &mut App, desired: u8) {
    input(app, Screen::Closed, Action::Open, None);
    input(app, Screen::Main, Action::Settings, None);
    input(app, Screen::Settings, Action::Graphics, None);
    input(app, Screen::Graphics, Action::Foliage, Some(desired));
    let ux = app.world().resource::<Fvr05ProductionUxStateResource>();
    assert_eq!(ux.settings.foliage_density, desired);
    assert!(ux.last_error.is_none(), "{:?}", ux.last_error);
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&ux.ui_settings_path).unwrap()).unwrap();
    assert_eq!(saved["foliage_density"].as_u64(), Some(u64::from(desired)));
    input(app, Screen::Graphics, Action::Back, None);
    input(app, Screen::Settings, Action::Back, None);
    input(app, Screen::Main, Action::Play, None);
    assert_eq!(app.world().resource::<MenuState>().screen, Screen::Closed);
}

fn witness(world: &mut World, (name, xz): Witness, tagged: bool) -> Entity {
    let mut q = world.query_filtered::<(
        Entity, &Name, &Transform, Option<&island::DecorativeGround>,
    ), With<island::PropRange>>();
    let mut rows = q.iter(world).filter(|(_, n, t, tag)| {
        n.as_str() == name && [t.translation.x.to_bits(), t.translation.z.to_bits()] == xz
            && tag.is_some() == tagged
    });
    let entity = rows.next().expect("signed prop was not admitted by the stock loader").0;
    assert!(rows.next().is_none(), "ambiguous signed placement");
    entity
}

fn masked_cut(app: &mut App, scene: &SceneAnchor, entities: &[Entity]) -> Cut {
    let saved: Vec<_> = entities.iter().map(|&id| {
        let v = *app.world().get::<Visibility>(id).unwrap();
        assert_ne!(v, Visibility::Hidden);
        (id, v)
    }).collect();
    app.world_mut().resource_mut::<Isolation>().0 = entities.to_vec();
    let image = cut(app, scene);
    app.world_mut().resource_mut::<Isolation>().0.clear();
    for (id, vis) in saved { app.world_mut().entity_mut(id).insert(vis); }
    image
}

fn pixel_distance(a: &[u8], b: &[u8]) -> u8 {
    a[..3].iter().zip(&b[..3]).map(|(x,y)| x.abs_diff(*y)).max().unwrap()
}

#[test]
#[ignore = "explicit Linux public graphics qualification; needs qualified CPU/Vulkan and scratch"]
fn public_stock_foliage_off_full_offscreen_software_vulkan() {
    let root = std::env::var_os("QUALIA_PUBLIC_FOLIAGE_CAPTURE_SCRATCH")
        .map(PathBuf::from).expect("explicit accepted scratch root is required");
    let project = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert!(root.is_absolute() && root.is_dir());
    let root = root.canonicalize().unwrap();
    assert!(!root.starts_with(project.join("../..").canonicalize().unwrap()));
    let dir = root.join(format!("foliage-offscreen-{}", std::process::id()));
    std::fs::create_dir(&dir).expect("new scratch subfolder; do not overwrite old evidence");

    let mut app = App::new();
    let render = RenderPlugin {
        render_creation: RenderCreation::Automatic(WgpuSettings {
            backends: Some(Backends::VULKAN), force_fallback_adapter: true,
            ..default()
        }),
        synchronous_pipeline_compilation: true, ..default()
    };
    app.add_plugins(DefaultPlugins
        .set(AssetPlugin { file_path: project.join("assets").to_string_lossy().into_owned(), ..default() })
        .set(WindowPlugin { primary_window: None, exit_condition: ExitCondition::DontExit, ..default() })
        .set(render)
        .disable::<WinitPlugin>()
        .disable::<PipelinedRenderingPlugin>());
    let deadline = Instant::now() + Duration::from_secs(40);
    while app.plugins_state() != PluginsState::Ready {
        assert!(Instant::now() < deadline, "Bevy render plugins did not become ready");
        bevy::tasks::tick_global_task_pools_on_main_thread();
        thread::sleep(Duration::from_millis(4));
    }
    app.finish();
    app.cleanup();
    let info = app.world().resource::<RenderAdapterInfo>();
    assert_eq!(info.backend, wgpu::Backend::Vulkan);
    assert_eq!(info.device_type, wgpu::DeviceType::Cpu);
    println!("PUBLIC_FOLIAGE_RENDER_ADAPTER {:?} {:?} {}", info.backend, info.device_type, info.name);

    let mut ux = load_tests::ux();
    ux.ui_settings_path = dir.join("settings.json");
    app.insert_resource(ux)
        .init_resource::<MenuState>()
        .init_resource::<Messages<ActionRequest>>()
        .init_resource::<Captured>()
        .init_resource::<Isolation>()
        .add_systems(Update, (handle_input, sync_ui, island::update, island::scatter::sync, isolate).chain());
    let world = app.world_mut();
    // Logical primary-window focus for the existing menu; no raw handle or
    // actual swapchain. UI and 3-D render on distinct native Image targets.
    world.spawn((Window { focused: true, resolution: (1280, 720).into(), ..default() }, PrimaryWindow));
    let ui = target(world, 1280, 720);
    world.spawn((Camera2d, RenderTarget::Image(ui.into()), IsDefaultUiCamera));
    spawn_ui(world);

    let terrain = alife_world::island_terrain();
    let focus = Vec3::new(84.0, terrain.surface().height(84.0, 217.0).unwrap(), 217.0);
    let surface = terrain.surface();
    assert!(surface.spacing.is_normal() && surface.spacing.is_sign_positive());
    let cx = ((focus.x - surface.origin_x) / surface.spacing).floor() as u32 / 32;
    let cz = ((focus.z - surface.origin_z) / surface.spacing).floor() as u32 / 32;
    let mesh_name = format!("Island_{cx}_{cz}_L0");
    let terrain_name = format!("Island chunk {cx}/{cz}");
    let gltf = world.resource::<AssetServer>().load::<Gltf>("landscape/island/terrain-chunks.glb");
    let pose = Transform::from_translation(focus + Vec3::new(78.0, 300.0, -81.0))
        .looking_at(focus, Vec3::Y);
    world.insert_resource(creature_grounding::SelectedTerrain(terrain));
    world.spawn((DirectionalLight { illuminance: 10_000.0, shadows_enabled: false, ..default() },
        Transform::from_rotation(Quat::from_rotation_x(-1.0))));
    let image = target(world, WIDTH, HEIGHT);
    let camera = world.spawn((
        Camera3d::default(), RenderTarget::Image(image.into()), Tonemapping::None, Msaa::Off,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical { viewport_height: 14.0 },
            far: 10_000.0, ..OrthographicProjection::default_3d()
        }),
        pose, Fvr03ProductionVoxelCamera { mode: Fvr03ProductionVoxelCameraMode::Orbit },
    )).id();
    island::start(world);
    drive_until(&mut app, "no terrain/prop import from the actual public GLBs", |a| {
        let world = a.world_mut();
        let chunks_present = world.query_filtered::<Entity, With<island::TerrainLod>>()
            .iter(world).next().is_some();
        let props_present = world.query_filtered::<Entity, With<island::PropRange>>()
            .iter(world).next().is_some();
        chunks_present && props_present
    });
    let scene = {
        let world = app.world_mut();
        let mut chunks = world.query_filtered::<(Entity, &Name), With<island::TerrainLod>>();
        let mut signed_chunks = chunks.iter(world).filter(|(_, n)| n.as_str() == terrain_name);
        let terrain = signed_chunks.next().expect("missing signed close tile").0;
        assert!(signed_chunks.next().is_none());
        let gltfs = world.resource::<Assets<Gltf>>();
        let meshes = world.resource::<Assets<GltfMesh>>();
        let l0 = &gltfs.get(&gltf).unwrap().named_meshes[mesh_name.as_str()];
        let terrain_l0 = meshes.get(l0).unwrap().primitives[0].mesh.id();
        SceneAnchor { camera, pose, terrain, terrain_l0 }
    };
    detent(&mut app, crate::production_voxel_frontend::FOLIAGE_MAX);
    let tagged = TAGGED.map(|w| witness(app.world_mut(), w, true));
    let other = witness(app.world_mut(), OTHER, false);
    drive_until(&mut app, "stock Full stream did not admit signed witnesses", |a| {
        tagged.iter().chain(std::iter::once(&other)).all(|&id| {
            a.world().get::<Visibility>(id).is_some_and(|v| *v != Visibility::Hidden)
        })
    });
    let witnessed_meshes = [tagged[0], tagged[1], other]
        .map(|id| app.world().get::<Mesh3d>(id).unwrap().0.id());
    let full = cut(&mut app, &scene);
    let no_tagged_control = masked_cut(&mut app, &scene, &tagged);
    let full_restored = cut(&mut app, &scene);
    assert!(full.bytes == full_restored.bytes, "Full failed exact post-control re-capture; scene not settled");
    assert!(full.bytes.chunks_exact(4).zip(no_tagged_control.bytes.chunks_exact(4))
        .filter(|(a,b)| pixel_distance(a,b) >= 16).count() >= 12,
        "the exact stock tagged props did not prove a positive rendered Full footprint");

    detent(&mut app, 0);
    assert!(tagged.iter().all(|&e| app.world().get::<Visibility>(e) == Some(&Visibility::Hidden)));
    assert_ne!(*app.world().get::<Visibility>(other).unwrap(), Visibility::Hidden);
    assert_eq!([tagged[0], tagged[1], other]
        .map(|id| app.world().get::<Mesh3d>(id).unwrap().0.id()), witnessed_meshes);
    let off = cut(&mut app, &scene);
    let no_other_control = masked_cut(&mut app, &scene, &[other]);
    let off_restored = cut(&mut app, &scene);
    assert!(off.bytes == off_restored.bytes, "Off failed exact post-control re-capture; scene not settled");

    let cuts = [&full, &no_tagged_control, &full_restored, &off, &no_other_control, &off_restored];
    for (i, a) in cuts.iter().enumerate() {
        assert!(cuts[..i].iter().all(|b| a.target != b.target && a.request != b.request));
    }
    let mut removed = 0;
    let mut retained = 0;
    for (((f, t), o), u) in full.bytes.chunks_exact(4).zip(no_tagged_control.bytes.chunks_exact(4))
        .zip(off.bytes.chunks_exact(4)).zip(no_other_control.bytes.chunks_exact(4))
    {
        removed += usize::from(pixel_distance(f,t) >= 16 && pixel_distance(o,t) <= 3);
        retained += usize::from(pixel_distance(o,u) >= 16 && pixel_distance(f,o) <= 3);
    }
    assert!(removed >= 12, "Off did not remove a positive stock tagged footprint");
    assert!(retained >= 12, "the exact separate untagged witness was not positively retained");
    println!("PUBLIC_FOLIAGE_OFFSCREEN {}x{} camera={:?} tile={} full={} off={} tagged_pixels={} untagged_pixels={}",
        WIDTH, HEIGHT, scene.camera, terrain_name, blake3::hash(&full.bytes), blake3::hash(&off.bytes), removed, retained);
    for (label, shot) in ["full", "full-tagged-mask", "full-restored", "off", "off-untagged-mask", "off-restored"].into_iter().zip(cuts) {
        println!("PUBLIC_FOLIAGE_CUT {label} target={:?} request={:?} bytes={} b3={}",
            shot.target, shot.request, shot.bytes.len(), blake3::hash(&shot.bytes));
        // Raw BGRA8 sRGB; no display compositor was used. Never leaves scratch.
        std::fs::write(dir.join(format!("{label}.bgra")), &shot.bytes).unwrap();
    }
}
