//! Explicit GPU-only review, never a claim that the live smoke world bred a family.
use super::*;
use bevy::{
    app::AppExit,
    render::view::screenshot::{save_to_disk, Screenshot},
    winit::{WinitPlugin, WinitSettings},
};
use std::time::Instant;

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
    let mut next = start;
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
            1 if out.join("parents.png").is_file() => {
                select(world, Control::Visit(1));
                step = 2;
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
            3 if out.join("descendants.png").is_file() => {
                select(world, Control::Later);
                step = 4;
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
        final_png.is_file() && finished.load(std::sync::atomic::Ordering::Relaxed),
        "family navigation did not reach its descendant frame"
    );
}
