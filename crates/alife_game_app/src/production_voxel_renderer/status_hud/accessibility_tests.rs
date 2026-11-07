//! CPU Bevy UI/AccessKit evidence, without an OS screen reader or renderer.
use super::*;
use accesskit::{Action, Role};
use bevy::ecs::message::Messages;

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((
        bevy::app::TaskPoolPlugin::default(),
        bevy::asset::AssetPlugin::default(),
        bevy::text::TextPlugin,
        bevy::ui::UiPlugin,
    ))
    .init_resource::<Assets<bevy::image::TextureAtlasLayout>>()
    .init_resource::<Assets<Image>>()
    .init_resource::<ButtonInput<MouseButton>>()
    .init_resource::<bevy::input::touch::Touches>()
    .init_resource::<bevy::picking::hover::HoverMap>()
    .init_resource::<bevy::picking::events::PointerState>()
    .init_resource::<Messages<bevy::picking::pointer::PointerInput>>()
    .init_resource::<Messages<bevy::picking::backend::PointerHits>>();
    spawn(app.world_mut());
    app
}

fn show(app: &mut App, readout: Option<&Readout>, debug: bool, size: Vec2) {
    set_readout(app.world_mut(), readout, debug);
    apply_layout(app.world_mut(), size);
    adapt_bottom_controls_visibility(app.world_mut(), size, readout.is_some() && !debug);
    app.update();
}

fn card(world: &mut World, quantity: Quantity) -> Entity {
    world
        .query::<(Entity, &Quantity)>()
        .iter(world)
        .find_map(|(entity, q)| (*q == quantity).then_some(entity))
        .unwrap()
}

fn assert_quantity(world: &mut World, quantity: Quantity, value: Option<f32>) {
    let entity = card(world, quantity);
    let accessible = world
        .get::<AccessibilityNode>(entity)
        .expect("each visible physiology card needs an accessible measured quantity");
    assert_eq!(accessible.role(), Role::Meter);
    let label = match quantity {
        Quantity::Hunger => "Hunger",
        Quantity::Energy => "Energy",
        Quantity::Fatigue => "Fatigue",
        Quantity::Calm => "Calm",
        Quantity::SleepPressure => "Sleep pressure",
        Quantity::PraiseSignal => "Praise signal",
    };
    assert_eq!(accessible.label(), Some(label));
    assert_eq!(accessible.min_numeric_value(), Some(0.0));
    assert_eq!(accessible.max_numeric_value(), Some(100.0));
    assert_eq!(
        accessible.numeric_value(),
        value.map(|v| f64::from(v * 100.0))
    );
    let expected = value.map_or_else(|| "Unknown".to_owned(), |v| format!("{:.0}%", v * 100.0));
    assert_eq!(accessible.value(), Some(expected.as_str()));
    assert_eq!(
        accessible.description(),
        value.filter(|v| quantity.warning(*v)).map(|_| "Warning")
    );
    assert!(!accessible.supports_action(Action::SetValue));
    assert!(!accessible.supports_action(Action::Click));
    // Accessible percentages use exactly the same normalized value and f32
    // conversion as the visible meter width, including the calm transformation.
    let width = world
        .query::<(&MeterFill, &Node)>()
        .iter(world)
        .find_map(|(fill, node)| (fill.0 == quantity).then_some(node.width))
        .unwrap();
    assert_eq!(width, Val::Percent(value.unwrap_or(0.0) * 100.0));
    let mut ancestor = entity;
    while let Some(parent) = world.get::<ChildOf>(ancestor) {
        ancestor = parent.parent();
        assert!(
            world.get::<AccessibilityNode>(ancestor).is_some(),
            "meter needs an unbroken accessible hierarchy"
        );
    }
}

#[test]
fn native_ui_exposes_all_six_actual_measured_values_and_warning_states() {
    let mut app = app();
    let mut row = data_tests::row();
    row.biochemistry.homeostasis.drives.hunger = 0.11;
    row.biochemistry.body.energy = 0.22;
    row.biochemistry.homeostasis.drives.fatigue = 0.73;
    row.biochemistry.homeostasis.drives.fear = 0.84;
    row.biochemistry.homeostasis.hormones.sleep_pressure = 0.95;
    row.biochemistry.homeostasis.hormones.extension[0] = 0.36;
    let readout = Readout::from_row(row.world_entity_id, row.organism_id, Some(&row), []);
    show(&mut app, Some(&readout), false, Vec2::new(1280.0, 720.0));
    for (quantity, value) in QUANTITIES.into_iter().zip(readout.values) {
        assert_quantity(app.world_mut(), quantity, value);
        let entity = card(app.world_mut(), quantity);
        assert!(!app
            .world()
            .get::<AccessibilityNode>(entity)
            .unwrap()
            .is_hidden());
    }
    // Decorative warning icons and question marks cannot become unnamed
    // alternatives to their semantic card after Bevy's image accessibility pass.
    for entity in app
        .world_mut()
        .query_filtered::<Entity, Or<(With<QuantityIcon>, With<Unknown>, With<Warning>)>>()
        .iter(app.world())
    {
        assert!(app
            .world()
            .get::<AccessibilityNode>(entity)
            .unwrap()
            .is_hidden());
    }
}

#[test]
fn selection_missing_and_invalid_rows_clear_old_values_instead_of_announcing_zero() {
    let mut app = app();
    let row = data_tests::row();
    let mut selected = Readout::from_row(row.world_entity_id, row.organism_id, Some(&row), []);
    selected.holding_food = true;
    for (id, name, values, holding) in [
        (1, "First", [Some(1.0); 6], true),
        (2, "Second", [Some(0.0); 6], false),
        (2, "Second", [None; 6], false),
    ] {
        selected.stable_id = id;
        selected.name = Some(name.into());
        selected.values = values;
        selected.holding_food = holding;
        show(&mut app, Some(&selected), false, Vec2::new(1280.0, 720.0));
        for (quantity, value) in QUANTITIES.into_iter().zip(values) {
            assert_quantity(app.world_mut(), quantity, value);
        }
        let world = app.world_mut();
        let root = world
            .query_filtered::<Entity, With<HudRoot>>()
            .single(world)
            .unwrap();
        assert_eq!(
            world.get::<AccessibilityNode>(root).unwrap().description(),
            holding.then_some("Holding unconsumed food")
        );
    }
    let mut invalid = row.clone();
    invalid.biochemistry.homeostasis.drives.hunger = f32::NAN;
    invalid.biochemistry.body.energy = f32::INFINITY;
    invalid.biochemistry.homeostasis.drives.fatigue = -0.1;
    invalid.biochemistry.homeostasis.drives.fear = 1.1;
    invalid.biochemistry.homeostasis.hormones.sleep_pressure = f32::NEG_INFINITY;
    invalid.biochemistry.homeostasis.hormones.extension[0] = f32::NAN;
    let invalid = Readout::from_row(row.world_entity_id, row.organism_id, Some(&invalid), []);
    show(&mut app, Some(&invalid), false, Vec2::new(1280.0, 720.0));
    for quantity in QUANTITIES {
        assert_quantity(app.world_mut(), quantity, None);
    }
    // Missing, stale-identity and dead rows cannot retain the previous
    // individual's percentages, name or possession claims.
    let mut stale = row.clone();
    stale.organism_id = OrganismId(row.organism_id.raw() + 1);
    let mut dead = row.clone();
    dead.lifecycle = alife_world::OrganismLifecycle::Dead {
        death_tick: Tick(1),
    };
    for source in [None, Some(&stale), Some(&dead)] {
        let unavailable = Readout::from_row(row.world_entity_id, row.organism_id, source, []);
        show(
            &mut app,
            Some(&unavailable),
            false,
            Vec2::new(1280.0, 720.0),
        );
        for quantity in QUANTITIES {
            assert_quantity(app.world_mut(), quantity, None);
        }
        assert!(unavailable.name.is_none());
        assert!(!unavailable.holding_food);
    }
}

#[test]
fn hidden_debug_deselected_and_short_viewport_hud_nodes_stay_hidden_and_restore() {
    let mut app = app();
    let row = data_tests::row();
    let readout = Readout::from_row(row.world_entity_id, row.organism_id, Some(&row), []);
    for (selected, debug, size, visible) in [
        (Some(&readout), false, Vec2::new(1280.0, 720.0), true),
        (Some(&readout), true, Vec2::new(1280.0, 720.0), false),
        (None, false, Vec2::new(1280.0, 720.0), false),
        (Some(&readout), false, Vec2::new(320.0, 180.0), false),
        (Some(&readout), false, Vec2::new(1280.0, 720.0), true),
    ] {
        show(&mut app, selected, debug, size);
        for quantity in QUANTITIES {
            let entity = card(app.world_mut(), quantity);
            assert_eq!(
                app.world()
                    .get::<AccessibilityNode>(entity)
                    .unwrap()
                    .is_hidden(),
                !visible
            );
            assert_quantity(
                app.world_mut(),
                quantity,
                selected.and_then(|r| {
                    r.values[QUANTITIES.iter().position(|q| *q == quantity).unwrap()]
                }),
            );
        }
        let world = app.world_mut();
        let root = world
            .query_filtered::<Entity, With<HudRoot>>()
            .single(world)
            .unwrap();
        assert_eq!(
            world.get::<AccessibilityNode>(root).unwrap().is_hidden(),
            !visible
        );
    }
}

#[test]
fn playback_and_rendered_count_semantics_follow_the_visible_status_and_remain_passive() {
    let mut app = app();
    for (count, paused, fast, label) in [
        (2, false, false, "Rendered creatures: 2. Playback: playing"),
        (0, true, true, "Rendered creatures: 0. Playback: paused"),
        (17, false, true, "Rendered creatures: 17. Playback: fast"),
    ] {
        set_status(app.world_mut(), count, paused, fast);
        show(&mut app, None, false, Vec2::new(1280.0, 720.0));
        let world = app.world_mut();
        let root = world
            .query_filtered::<Entity, With<StatusRoot>>()
            .single(world)
            .unwrap();
        let accessible = world
            .get::<AccessibilityNode>(root)
            .expect("wordless playback needs semantic status");
        assert_eq!(accessible.label(), Some(label));
        assert!(!accessible.is_hidden());
        assert!(!accessible.supports_action(Action::Click));
        let other = world
            .spawn(AccessibilityNode(accesskit::Node::new(Role::Label)))
            .id();
        show(&mut app, None, true, Vec2::new(1280.0, 720.0));
        assert!(app
            .world()
            .get::<AccessibilityNode>(root)
            .unwrap()
            .is_hidden());
        assert!(!app
            .world()
            .get::<AccessibilityNode>(other)
            .unwrap()
            .is_hidden());
        show(&mut app, None, false, Vec2::new(320.0, 100.0));
        assert!(app
            .world()
            .get::<AccessibilityNode>(root)
            .unwrap()
            .is_hidden());
        show(&mut app, None, false, Vec2::new(1280.0, 720.0));
        assert!(!app
            .world()
            .get::<AccessibilityNode>(root)
            .unwrap()
            .is_hidden());
    }
}
