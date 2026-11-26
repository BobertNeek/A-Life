//! A separate 3D exposure photographs the player's composition, not the desktop,
//! HUD or menu. It neither pauses the world nor borrows the primary camera.
use super::*;
use bevy::{
    camera::RenderTarget,
    core_pipeline::tonemapping::Tonemapping,
    render::{
        render_resource::{TextureFormat, TextureUsages},
        view::screenshot::ScreenshotCaptured,
    },
    tasks::{block_on, poll_once, IoTaskPool, Task},
};
use std::{
    io,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
mod album;
mod plates;
pub(super) use album::{paper, shutter_button, PlateControl};

#[derive(Resource, Default)]
pub(super) struct Shelf {
    folder: PathBuf,
    entries: Vec<PathBuf>,
    selected: usize,
    displayed: Option<PathBuf>,
    art: Option<Handle<Image>>,
    flight: Option<Flight>,
    write: Option<Task<io::Result<PathBuf>>>,
    failed: bool,
    kept: bool,
    album_open: bool,
}
struct Flight {
    camera: Entity,
    texture: Handle<Image>,
    started: Instant,
    frames: u8,
    shutter: Option<Entity>,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<Shelf>().add_systems(
        Update,
        develop
            .after(super::handle_input)
            .before(super::sync_ui)
            .in_set(ProductionVoxelPresentationSet::RootReaders),
    );
}
fn refresh(s: &mut Shelf) -> io::Result<()> {
    s.entries = plates::index(&s.folder)?;
    s.selected = 0;
    s.displayed = None;
    Ok(())
}
fn expose(world: &mut World, s: &mut Shelf) -> bool {
    let Ok((pose, proj, camera, ambient, fog, tone, fxaa)) = world
        .query_filtered::<(
            &Transform,
            &Projection,
            &Camera,
            &AmbientLight,
            &DistanceFog,
            &Tonemapping,
            &bevy_anti_alias::fxaa::Fxaa,
        ), With<Fvr03ProductionVoxelCamera>>()
        .single(world)
    else {
        return false;
    };
    let (pose, projection, clear_color, ambient, fog, tonemapping, fxaa) = (
        *pose,
        proj.clone(),
        camera.clear_color,
        ambient.clone(),
        fog.clone(),
        *tone,
        fxaa.clone(),
    );
    let mut target = Image::new_target_texture(
        plates::WIDTH,
        plates::HEIGHT,
        TextureFormat::Rgba8UnormSrgb,
        None,
    );
    target.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let texture = world.resource_mut::<Assets<Image>>().add(target);
    let camera = world
        .spawn((
            Name::new("Disposable field camera"),
            Camera3d::default(),
            Camera {
                order: -2,
                clear_color,
                ..default()
            },
            RenderTarget::Image(texture.clone().into()),
            pose,
            projection,
            ambient,
            fog,
            tonemapping,
            Msaa::Off,
            fxaa,
        ))
        .id();
    s.flight = Some(Flight {
        camera,
        texture,
        frames: 0,
        started: Instant::now(),
        shutter: None,
    });
    s.failed = false;
    true
}
fn received(shot: On<ScreenshotCaptured>, mut shelf: ResMut<Shelf>, mut commands: Commands) {
    if shelf
        .flight
        .as_ref()
        .is_none_or(|s| s.shutter != Some(shot.entity))
    {
        return;
    }
    let flight = shelf.flight.take().unwrap();
    commands.entity(flight.camera).despawn();
    let image = shot.image.clone();
    let folder = shelf.folder.clone();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    shelf.write = Some(IoTaskPool::get().spawn(async move {
        let rgba = image
            .try_into_dynamic()
            .map_err(io::Error::other)?
            .to_rgba8();
        plates::store(&folder, stamp, &rgba)
    }));
}
fn develop(world: &mut World) {
    let action = world.resource_mut::<MenuState>().plate_action.take();
    let viewing = world.resource::<MenuState>().screen == Screen::Photos;
    let shelf = world.resource::<Shelf>();
    if !viewing
        && !shelf.album_open
        && action.is_none()
        && shelf.flight.is_none()
        && shelf.write.is_none()
    {
        return;
    }
    let can_expose = action == Some(Action::Expose)
        && !world
            .resource::<Fvr05ProductionUxStateResource>()
            .debug_mode
        && world
            .query_filtered::<&Window, With<PrimaryWindow>>()
            .iter(world)
            .next()
            .is_some_and(|w| w.focused);
    world.resource_scope(|world, mut s: Mut<Shelf>| {
        if s.folder.as_os_str().is_empty() {
            s.folder = world
                .resource::<Fvr05ProductionUxStateResource>()
                .ui_settings_path
                .parent()
                .unwrap_or(Path::new("."))
                .join("field-photographs");
        }
        advance_exposure(world, &mut s, viewing);
        match action {
            Some(Action::Photos) => s.failed = refresh(&mut s).is_err(),
            Some(Action::Expose) if can_expose && s.flight.is_none() && s.write.is_none() => {
                s.failed = !expose(world, &mut s);
            }
            Some(Action::Older) => {
                s.selected = (s.selected + 1).min(s.entries.len().saturating_sub(1))
            }
            Some(Action::Newer) => s.selected = s.selected.saturating_sub(1),
            _ => {}
        }
        if viewing
            && (!s.album_open
                || s.entries.get(s.selected) != s.displayed.as_ref()
                || (s.entries.is_empty() && s.art.is_some()))
        {
            album::display_album(world, &mut s);
        } else if !viewing && s.album_open {
            album::hide_print(world);
        }
        s.album_open = viewing;
        album::show_lamps(world, &s);
    });
}

fn advance_exposure(world: &mut World, s: &mut Shelf, viewing: bool) {
    if let Some(flight) = s.flight.as_mut() {
        flight.frames = flight.frames.saturating_add(1);
        if flight.started.elapsed() > Duration::from_secs(12) {
            let flight = s.flight.take().unwrap();
            if let Ok(e) = world.get_entity_mut(flight.camera) {
                e.despawn();
            }
            if let Some(id) = flight.shutter {
                if let Ok(e) = world.get_entity_mut(id) {
                    e.despawn();
                }
            }
            s.failed = true;
        } else if flight.frames >= 3 && flight.shutter.is_none() {
            flight.shutter = Some(
                world
                    .spawn(Screenshot::image(flight.texture.clone()))
                    .observe(received)
                    .id(),
            );
        }
    }
    if let Some(done) = s.write.as_mut().and_then(|task| block_on(poll_once(task))) {
        s.write = None;
        match done {
            Ok(_) => {
                s.failed = viewing && refresh(s).is_err();
                s.kept = true;
            }
            Err(e) => {
                s.failed = true;
                warn!("Field camera could not keep this plate: {e}");
            }
        }
    }
}
impl Shelf {
    pub(super) fn available(&self, action: Action) -> bool {
        match action {
            Action::Older => self.selected + 1 < self.entries.len(),
            Action::Newer => self.selected != 0,
            Action::Expose => self.flight.is_none() && self.write.is_none(),
            _ => true,
        }
    }
}
