//! Explicit GPU-only review, never a claim that the live smoke world bred a family.
use super::*;
use bevy::{
    app::AppExit,
    render::view::screenshot::{save_to_disk, Screenshot},
    winit::{WinitPlugin, WinitSettings},
};
use std::time::Instant;

/// Startup may produce a valid all-black PNG before the UI pipeline is ready.
/// Require both the pale crest ink and the mint diagram color before advancing.
fn album_visible(path: &std::path::Path) -> bool {
    let Ok(frame) = ::image::open(path) else {
        return false;
    };
    let mut ink = 0;
    let mut mint = 0;
    for pixel in frame.to_rgb8().pixels() {
        let [r, g, b] = pixel.0;
        if r > 210 && g > 200 && b > 165 {
            ink += 1;
        }
        if g > 160 && g.saturating_sub(r) > 25 && b > 125 {
            mint += 1;
        }
        if ink >= 12 && mint >= 12 {
            return true;
        }
    }
    false
}

#[test]
#[ignore = "opens a real GPU window; explicitly run to review the family album"]
fn capture_family_navigation_using_real_accessibility_actions() {
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("target/artifacts/kinship-graph")
        .join(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .to_string(),
        );
    std::fs::create_dir_all(&out).unwrap();
    println!("Family fixture: {}", out.display());
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WinitPlugin {
                run_on_any_thread: true,
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Qualia family album | UI fixture".to_owned(),
                    resolution: (640, 360).into(),
                    ..default()
                }),
                ..default()
            }),
    )
    .insert_resource(WinitSettings::continuous());
    let mut ui = super::super::speech_completion_state_tests::state();
    ui.family_open = true;
    app.insert_resource(ui)
        .init_resource::<InputFocus>()
        .insert_resource(State {
            album: Some(super::tests::family()),
            dirty: true,
            ..default()
        });
    install(&mut app);
    app.add_systems(Startup, |mut commands: Commands| {
        commands.spawn(Camera2d);
    });
    let mut step = 0u8;
    let start = Instant::now();
    let mut next = start + std::time::Duration::from_secs(2);
    let final_png = out.join("descendants.png");
    let finished = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let observed = finished.clone();
    app.add_systems(Last, move |world: &mut World| {
        if start.elapsed().as_secs() > 50 {
            world.write_message(AppExit::Success);
            return;
        }
        if Instant::now() < next {
            return;
        }
        next = Instant::now() + std::time::Duration::from_millis(550);
        let pending = |w: &World| w.resource::<State>().dirty;
        if pending(world) {
            return;
        }
        let select = |w: &mut World, c: Control| {
            let e = w
                .query::<(Entity, &Control)>()
                .iter(w)
                .find_map(|(e, a)| (*a == c).then_some(e))
                .unwrap();
            w.write_message(ActionRequest(accesskit::ActionRequest {
                action: A11yAction::Click,
                target: accesskit::NodeId(e.to_bits()),
                data: None,
            }));
        };
        match step {
            0 if world.resource::<State>().root.is_some() => {
                world
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(out.join("parents.png")));
                step = 1;
            }
            1 if album_visible(&out.join("parents.png")) => {
                select(world, Control::Visit(1));
                step = 2;
            }
            1 if out.join("parents.png").is_file() => {
                step = 0;
            }
            2 if world
                .resource::<State>()
                .album
                .as_ref()
                .is_some_and(|a| a.focus == 1) =>
            {
                world
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(out.join("descendants.png")));
                step = 3;
            }
            3 if album_visible(&out.join("descendants.png")) => {
                select(world, Control::Later);
                step = 4;
            }
            3 if out.join("descendants.png").is_file() => {
                step = 2;
            }
            4 if world
                .resource::<State>()
                .album
                .as_ref()
                .is_some_and(|a| a.page == 1) =>
            {
                select(world, Control::Visit(6));
                step = 5;
            }
            5 if world
                .resource::<State>()
                .album
                .as_ref()
                .is_some_and(|a| a.focus == 6) =>
            {
                observed.store(true, std::sync::atomic::Ordering::Relaxed);
                world.write_message(AppExit::Success);
            }
            _ => {}
        }
    });
    assert!(app.run().is_success());
    assert!(
        album_visible(&final_png) && finished.load(std::sync::atomic::Ordering::Relaxed),
        "family navigation did not reach its descendant frame"
    );
}
