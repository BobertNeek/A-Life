//! CPU layout receipts, not rendered screenshots or glyph-layout evidence.
use super::*;
use bevy::app::{HierarchyPropagatePlugin, PostUpdate, PropagateSet};
use bevy::camera::{ComputedCameraValues, RenderTargetInfo, Viewport};
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

// The same UiSurface/camera setup as menu's proven CPU layout test. No
// renderer, window, GPU, neural session, or production simulation is started.
fn layout_app(size: Vec2) -> App {
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
    spawn(app.world_mut());
    apply_layout(app.world_mut(), size);
    app
}

#[derive(Clone, Copy, Debug)]
struct Box2 {
    min: Vec2,
    max: Vec2,
}
impl Box2 {
    fn of(world: &World, entity: Entity) -> Self {
        let node = world.get::<ComputedNode>(entity).unwrap();
        let transform = world.get::<UiGlobalTransform>(entity).unwrap();
        let center = transform.affine().translation;
        let half = node.size() * 0.5;
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
        json!({"min": [self.min.x, self.min.y], "max": [self.max.x, self.max.y],
            "size": [self.size().x, self.size().y]})
    }
}

fn visible_in_hierarchy(world: &World, mut entity: Entity) -> bool {
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

fn assert_readout_components(world: &mut World, readout: Option<&Readout>) {
    let values = readout.map_or([None; 6], |readout| readout.values);
    let badge = world
        .query_filtered::<&Visibility, With<HeldFood>>()
        .single(world)
        .unwrap();
    assert_eq!(
        *badge,
        if readout.is_some_and(|readout| readout.holding_food) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        }
    );
    let value = |quantity| values[QUANTITIES.iter().position(|q| *q == quantity).unwrap()];
    assert_eq!(world.query::<&MeterFill>().iter(world).count(), 6);
    assert_eq!(world.query::<&MeterTrack>().iter(world).count(), 6);
    assert_eq!(world.query::<&Unknown>().iter(world).count(), 6);
    assert_eq!(world.query::<&Warning>().iter(world).count(), 6);
    for (fill, node, color) in world
        .query::<(&MeterFill, &Node, &BackgroundColor)>()
        .iter(world)
    {
        let amount = value(fill.0);
        assert_eq!(node.width, Val::Percent(amount.unwrap_or(0.0) * 100.0));
        assert_eq!(
            color.0,
            amount.map_or(Color::NONE, |v| fill.0.meter_color(v))
        );
    }
    for (track, visibility) in world.query::<(&MeterTrack, &Visibility)>().iter(world) {
        assert_eq!(
            *visibility,
            if value(track.0).is_some() {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            }
        );
    }
    for (unknown, visibility, text) in world.query::<(&Unknown, &Visibility, &Text)>().iter(world) {
        assert_eq!(text.0, "?");
        assert_eq!(
            *visibility,
            if value(unknown.0).is_none() {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            }
        );
    }
    for (warning, visibility) in world.query::<(&Warning, &Visibility)>().iter(world) {
        assert_eq!(
            *visibility,
            if value(warning.0).is_some_and(|v| warning.0.warning(v)) {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            }
        );
    }
}

fn assert_layout(world: &mut World, size: Vec2) {
    let root = world
        .query_filtered::<Entity, With<HudRoot>>()
        .single(world)
        .unwrap();
    let panel = Box2::of(world, root);
    assert!(
        Box2 {
            min: Vec2::ZERO,
            max: size
        }
        .contains(panel),
        "selected panel outside {size:?}: {panel:?}"
    );
    let tiles = world
        .get::<Children>(root)
        .unwrap()
        .iter()
        .collect::<Vec<_>>();
    // The identity header and quantity grid must fit and be disjoint. Badge
    // overlays are contained, but deliberately not treated as separate tiles.
    // Text node boxes are measured; text glyphs are not.
    assert_eq!(tiles.len(), 2);
    for (index, &tile) in tiles.iter().enumerate() {
        let bounds = Box2::of(world, tile);
        assert!(bounds.size().x > 0.0 && bounds.size().y > 0.0);
        assert!(
            panel.contains(bounds),
            "tile outside panel at {size:?}: {bounds:?}"
        );
        for &other in &tiles[index + 1..] {
            assert!(
                !bounds.overlaps(Box2::of(world, other)),
                "tiles overlap at {size:?}: {bounds:?} and {:?}",
                Box2::of(world, other)
            );
        }
    }
    let cards = world
        .query::<(Entity, &Quantity)>()
        .iter(world)
        .map(|(entity, quantity)| (entity, *quantity))
        .collect::<Vec<_>>();
    assert_eq!(cards.len(), QUANTITIES.len());
    let grid = world
        .query_filtered::<Entity, With<QuantityGrid>>()
        .single(world)
        .unwrap();
    let grid_box = Box2::of(world, grid);
    let ordered = world
        .get::<Children>(grid)
        .unwrap()
        .iter()
        .map(|entity| *world.get::<Quantity>(entity).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        ordered,
        QUANTITIES.to_vec(),
        "wrap must preserve quantity order"
    );
    for (index, &(card, _)) in cards.iter().enumerate() {
        let bounds = Box2::of(world, card);
        assert!(
            grid_box.contains(bounds) && panel.contains(bounds),
            "card outside grid at {size:?}"
        );
        for &(other, _) in &cards[index + 1..] {
            assert!(
                !bounds.overlaps(Box2::of(world, other)),
                "quantity cards overlap at {size:?}"
            );
        }
    }
    let identity = world
        .query_filtered::<Entity, With<Identity>>()
        .single(world)
        .unwrap();
    let header = world
        .query_filtered::<Entity, With<IdentityHeader>>()
        .single(world)
        .unwrap();
    let identity_column = world.get::<ChildOf>(identity).unwrap().parent();
    assert!(Box2::of(world, identity_column).contains(Box2::of(world, identity)));
    let header_box = Box2::of(world, header);
    let header_children = world
        .get::<Children>(header)
        .unwrap()
        .iter()
        .collect::<Vec<_>>();
    for (index, &child) in header_children.iter().enumerate() {
        let bounds = Box2::of(world, child);
        assert!(
            header_box.contains(bounds),
            "identity node outside header at {size:?}"
        );
        for &other in &header_children[index + 1..] {
            assert!(
                !bounds.overlaps(Box2::of(world, other)),
                "identity header nodes overlap"
            );
        }
    }
    for quantity in QUANTITIES {
        let matching = cards
            .iter()
            .filter(|(_, q)| *q == quantity)
            .collect::<Vec<_>>();
        assert_eq!(matching.len(), 1, "duplicate or absent {quantity:?} card");
        let card = matching[0].0;
        let bounds = Box2::of(world, card);
        let children = world
            .get::<Children>(card)
            .unwrap()
            .iter()
            .collect::<Vec<_>>();
        let icon = children
            .iter()
            .copied()
            .find(|&entity| image_name(world, entity) == Some(quantity.name()))
            .unwrap();
        let track = children
            .iter()
            .copied()
            .find(|&entity| {
                world
                    .get::<MeterTrack>(entity)
                    .is_some_and(|t| t.0 == quantity)
            })
            .unwrap();
        let warning = children
            .iter()
            .copied()
            .find(|&entity| {
                world
                    .get::<Warning>(entity)
                    .is_some_and(|w| w.0 == quantity)
            })
            .unwrap();
        let unknown = children
            .iter()
            .copied()
            .find(|&entity| {
                world
                    .get::<Unknown>(entity)
                    .is_some_and(|u| u.0 == quantity)
            })
            .unwrap();
        let icon_box = Box2::of(world, icon);
        let track_box = Box2::of(world, track);
        assert!(icon_box.size().x > 0.0 && icon_box.size().y > 0.0);
        assert!(track_box.size().x > 0.0 && track_box.size().y > 0.0);
        assert!(
            bounds.contains(icon_box),
            "{quantity:?} icon outside card at {size:?}"
        );
        assert!(
            bounds.contains(track_box),
            "{quantity:?} meter outside card at {size:?}"
        );
        assert!(
            !icon_box.overlaps(track_box),
            "{quantity:?} icon overlaps meter at {size:?}"
        );
        assert!(
            bounds.contains(Box2::of(world, warning)),
            "warning outside {quantity:?} card"
        );
        let unknown_box = Box2::of(world, unknown);
        assert!(
            bounds.contains(unknown_box),
            "unknown marker outside {quantity:?} card"
        );
        if world.get::<Visibility>(unknown) != Some(&Visibility::Hidden) {
            assert!(
                unknown_box.size().x > 0.0 && unknown_box.size().y > 0.0,
                "unknown marker needs explicit bounds; font shaping is not run"
            );
            assert!(
                !unknown_box.overlaps(icon_box),
                "unknown marker overlaps {quantity:?} icon at {size:?}"
            );
            assert_eq!(
                world.get::<Visibility>(track),
                Some(&Visibility::Hidden),
                "unknown marker must not obscure a visible meter"
            );
        }
        let fill = world
            .get::<Children>(track)
            .unwrap()
            .iter()
            .find(|&entity| {
                world
                    .get::<MeterFill>(entity)
                    .is_some_and(|f| f.0 == quantity)
            })
            .unwrap();
        let fill_box = Box2::of(world, fill);
        assert!(
            track_box.contains(fill_box),
            "{quantity:?} fill outside track"
        );
        let Val::Percent(percent) = world.get::<Node>(fill).unwrap().width else {
            panic!("meter fill must use measured percentage");
        };
        assert!(
            (fill_box.size().x - track_box.size().x * percent / 100.0).abs() <= 1.0,
            "{quantity:?} computed fill disagrees with ECS value"
        );
    }
    // The count/playback strip remains independent of selection visibility.
    let playback = world
        .query_filtered::<Entity, With<Playback>>()
        .single(world)
        .unwrap();
    let status = world.get::<ChildOf>(playback).unwrap().parent();
    let status_box = Box2::of(world, status);
    assert!(Box2 {
        min: Vec2::ZERO,
        max: size
    }
    .contains(status_box));
    assert!(
        !panel.overlaps(status_box),
        "status and selection panels overlap at {size:?}"
    );
    for child in world.get::<Children>(status).unwrap().iter() {
        if world.get::<ImageNode>(child).is_some() {
            assert!(status_box.contains(Box2::of(world, child)));
        }
    }
}

fn rgba(color: Color) -> [f32; 4] {
    let color = color.to_srgba();
    [color.red, color.green, color.blue, color.alpha]
}
fn val(value: Val) -> Value {
    match value {
        Val::Px(value) => json!({"unit": "px", "value": value}),
        Val::Percent(value) => json!({"unit": "percent", "value": value}),
        Val::Auto => json!({"unit": "auto"}),
        other => json!({"source_debug": format!("{other:?}")}),
    }
}
fn rect(rect: UiRect) -> Value {
    json!({"top": val(rect.top), "right": val(rect.right),
        "bottom": val(rect.bottom), "left": val(rect.left)})
}

fn export_nodes(world: &mut World) -> Vec<Value> {
    let entities = world
        .query_filtered::<Entity, With<Node>>()
        .iter(world)
        .collect::<Vec<_>>();
    entities.into_iter().map(|entity| {
        let node = world.get::<Node>(entity).unwrap();
        let quantity = world.get::<Quantity>(entity).copied()
            .or_else(|| world.get::<MeterTrack>(entity).map(|q| q.0))
            .or_else(|| world.get::<MeterFill>(entity).map(|q| q.0))
            .or_else(|| world.get::<Unknown>(entity).map(|q| q.0))
            .or_else(|| world.get::<Warning>(entity).map(|q| q.0));
        let role = if world.get::<HudRoot>(entity).is_some() { "selected_panel" }
            else if world.get::<StatusRoot>(entity).is_some() { "status_panel" }
            else if world.get::<QuantityGrid>(entity).is_some() { "quantity_grid" }
            else if world.get::<Quantity>(entity).is_some() { "quantity_card" }
            else if world.get::<MeterTrack>(entity).is_some() { "meter_track" }
            else if world.get::<MeterFill>(entity).is_some() { "meter_fill" }
            else if world.get::<Unknown>(entity).is_some() { "unknown_marker" }
            else if world.get::<Warning>(entity).is_some() { "warning_badge" }
            else if world.get::<Identity>(entity).is_some() { "stable_identity_text" }
            else if world.get::<HeldFood>(entity).is_some() { "held_food_badge" }
            else if world.get::<Count>(entity).is_some() { "rendered_count_text" }
            else if world.get::<Playback>(entity).is_some() { "playback_icon" }
            else if world.get::<ImageNode>(entity).is_some() { "icon" }
            else { "container" };
        let image = world.get::<ImageNode>(entity).map(|image| {
            let name = image_name(world, entity).expect("every HUD image maps to an embedded asset");
            let asset = world.resource::<Assets<Image>>().get(&image.image).unwrap();
            json!({"name": name,
                "source_path": format!("assets/ui/status_hud/{name}.png"),
                "decoded_size": [asset.width(), asset.height()]})
        });
        let border = world.get::<BorderColor>(entity).map(|border| {
            json!({"top": rgba(border.top), "right": rgba(border.right),
                "bottom": rgba(border.bottom), "left": rgba(border.left)})
        });
        let text = world.get::<Text>(entity).map(|text| {
            json!({"value": text.0, "font_size": world.get::<TextFont>(entity).map(|font| font.font_size),
                "color": world.get::<TextColor>(entity).map(|color| rgba(color.0)),
                "glyph_bounds_measured": false})
        });
        json!({"entity": format!("{entity:?}"),
            "parent": world.get::<ChildOf>(entity).map(|parent| format!("{:?}", parent.parent())),
            "name": world.get::<Name>(entity).map(|name| name.as_str()),
            "role": role, "quantity": quantity.map(Quantity::name),
            "computed_box": Box2::of(world, entity).json(),
            "explicit_visibility": world.get::<Visibility>(entity).map(|v| format!("{v:?}")),
            "visible_in_hierarchy": visible_in_hierarchy(world, entity),
            "style": {
                "display": format!("{:?}", node.display),
                "position_type": format!("{:?}", node.position_type),
                "left": val(node.left), "top": val(node.top), "right": val(node.right), "bottom": val(node.bottom),
                "width": val(node.width), "height": val(node.height),
                "min_width": val(node.min_width), "max_width": val(node.max_width),
                "min_height": val(node.min_height), "max_height": val(node.max_height),
                "padding": rect(node.padding), "border": rect(node.border),
                "border_radius": {"top_left": val(node.border_radius.top_left), "top_right": val(node.border_radius.top_right),
                    "bottom_right": val(node.border_radius.bottom_right), "bottom_left": val(node.border_radius.bottom_left)},
                "flex_direction": format!("{:?}", node.flex_direction), "flex_wrap": format!("{:?}", node.flex_wrap),
                "flex_shrink": node.flex_shrink,
                "align_items": format!("{:?}", node.align_items), "justify_content": format!("{:?}", node.justify_content),
                "column_gap": val(node.column_gap), "row_gap": val(node.row_gap),
                "background": world.get::<BackgroundColor>(entity).map(|background| rgba(background.0)),
                "border_colors": border
            }, "image": image, "text": text})
    }).collect()
}

#[test]
fn cpu_layout_fits_supported_viewports_and_exports_source_derived_fixtures() {
    let normal = [
        Some(0.25),
        Some(0.82),
        Some(0.20),
        Some(0.78),
        Some(0.30),
        Some(0.12),
    ];
    let cases = [
        (
            "normal_mixed_values",
            Some(Readout {
                name: None,
                holding_food: false,
                stable_id: 42,
                values: normal,
            }),
            false,
        ),
        (
            "high_burdens_low_reserves",
            Some(Readout {
                name: None,
                holding_food: false,
                stable_id: 42,
                values: [
                    Some(1.0),
                    Some(0.10),
                    Some(0.85),
                    Some(0.12),
                    Some(0.90),
                    Some(0.05),
                ],
            }),
            false,
        ),
        (
            "all_unknown",
            Some(Readout {
                name: None,
                holding_food: false,
                stable_id: 42,
                values: [None; 6],
            }),
            false,
        ),
        (
            "valid_zero",
            Some(Readout {
                name: None,
                holding_food: false,
                stable_id: 42,
                values: [Some(0.0); 6],
            }),
            false,
        ),
        (
            "high_signals",
            Some(Readout {
                name: None,
                holding_food: false,
                stable_id: 42,
                values: [
                    Some(0.40),
                    Some(0.65),
                    Some(0.35),
                    Some(0.60),
                    Some(1.0),
                    Some(1.0),
                ],
            }),
            false,
        ),
        (
            "partial_unknown",
            Some(Readout {
                name: None,
                holding_food: false,
                stable_id: 42,
                values: [Some(0.25), Some(0.82), None, Some(0.78), None, Some(0.12)],
            }),
            false,
        ),
        (
            "largest_stable_id",
            Some(Readout {
                name: None,
                holding_food: true,
                stable_id: u64::MAX,
                values: normal,
            }),
            false,
        ),
        (
            "debug_hidden",
            Some(Readout {
                name: None,
                holding_food: true,
                stable_id: 42,
                values: normal,
            }),
            true,
        ),
        ("selection_cleared", None, false),
    ];
    let mut fixtures = Vec::new();
    let mut embedded_assets = Vec::new();
    for size in VIEWPORTS {
        let mut app = layout_app(size);
        if embedded_assets.is_empty() {
            for (name, handle) in &app.world().resource::<Icons>().0 {
                let image = app.world().resource::<Assets<Image>>().get(handle).unwrap();
                assert!(
                    image.width() > 0 && image.height() > 0,
                    "embedded {name} PNG did not decode"
                );
                embedded_assets.push(json!({"name": name,
                    "source_path": format!("assets/ui/status_hud/{name}.png"),
                    "decoded_size": [image.width(), image.height()]}));
            }
            assert_eq!(embedded_assets.len(), EMBEDDED.len());
        }
        for (name, readout, debug) in &cases {
            set_readout(app.world_mut(), readout.as_ref(), *debug);
            let paused = *name == "high_burdens_low_reserves";
            let fast = *name == "high_signals";
            set_status(app.world_mut(), 3, paused, fast);
            app.update();
            app.update();
            assert_layout(app.world_mut(), size);
            assert_readout_components(app.world_mut(), readout.as_ref());
            let root = app
                .world_mut()
                .query_filtered::<Entity, With<HudRoot>>()
                .single(app.world())
                .unwrap();
            assert_eq!(
                visible_in_hierarchy(app.world(), root),
                readout.is_some() && !debug
            );
            let status = app
                .world_mut()
                .query_filtered::<Entity, With<StatusRoot>>()
                .single(app.world())
                .unwrap();
            assert_eq!(visible_in_hierarchy(app.world(), status), !debug);
            let identity = app
                .world_mut()
                .query_filtered::<&Text, With<Identity>>()
                .single(app.world())
                .unwrap();
            assert_eq!(
                identity.0,
                readout
                    .as_ref()
                    .map_or_else(String::new, |r| format!("#{}", r.stable_id))
            );
            fixtures.push(
                json!({"case": name, "viewport": [size.x, size.y], "debug": debug,
                "stable_id": readout.as_ref().map(|r| r.stable_id.to_string()),
                "input_values": readout.as_ref().map(|r| r.values),
                "holding_food": readout.as_ref().is_some_and(|r| r.holding_food),
                "nodes": export_nodes(app.world_mut())}),
            );
        }
    }
    if let Ok(path) = std::env::var("ALIFE_HUD_CPU_FIXTURE") {
        let receipt = json!({"evidence_kind": "source-derived CPU Bevy UiSurface layout",
            "is_screenshot": false, "glyph_bounds_measured": false,
            "limitations": "Synthetic readout inputs; no renderer, simulation, GPU, font shaping, or screenshot. Text values/font styles are actual ECS components; text computed boxes do not establish glyph fit. Visibility is resolved from actual ancestor Visibility components. Warning badges may overlay their own card icons.",
            "quantity_order": QUANTITIES.map(Quantity::name), "embedded_assets": embedded_assets,
            "fixtures": fixtures});
        std::fs::write(path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    }
}

#[test]
fn unknown_is_not_valid_zero_and_clear_or_debug_hides_selection() {
    let mut world = World::new();
    spawn(&mut world);
    for values in [[None; 6], [Some(0.0); 6], [Some(1.0); 6], [None; 6]] {
        let readout = Readout {
            name: None,
            holding_food: false,
            stable_id: u64::MAX,
            values,
        };
        set_readout(&mut world, Some(&readout), false);
        assert_readout_components(&mut world, Some(&readout));
    }
    let readout = Readout {
        name: None,
        holding_food: false,
        stable_id: u64::MAX,
        values: [Some(0.5); 6],
    };
    for (selected, debug, shown) in [
        (true, true, false),
        (true, false, true),
        (false, false, false),
    ] {
        set_readout(&mut world, selected.then_some(&readout), debug);
        let root = world
            .query_filtered::<Entity, With<HudRoot>>()
            .single(&world)
            .unwrap();
        assert_eq!(visible_in_hierarchy(&world, root), shown);
        for (entity, _) in world.query::<(Entity, &Quantity)>().iter(&world) {
            assert_eq!(visible_in_hierarchy(&world, entity), shown);
        }
        let playback = world
            .query_filtered::<Entity, With<Playback>>()
            .single(&world)
            .unwrap();
        assert_eq!(visible_in_hierarchy(&world, playback), !debug);
        let identity = world
            .query_filtered::<&Text, With<Identity>>()
            .single(&world)
            .unwrap();
        assert_eq!(
            identity.0,
            if selected {
                format!("#{}", u64::MAX)
            } else {
                String::new()
            }
        );
    }
}

#[test]
fn status_count_and_playback_follow_source_state_with_pause_precedence() {
    let mut world = World::new();
    spawn(&mut world);
    for (count, paused, fast, expected) in [
        (0, false, false, "playing"),
        (3, true, false, "paused"),
        (27, false, true, "fast"),
        (27, true, true, "paused"),
        (65535, false, false, "playing"),
    ] {
        set_status(&mut world, count, paused, fast);
        let text = world
            .query_filtered::<&Text, With<Count>>()
            .single(&world)
            .unwrap();
        assert_eq!(text.0, count.to_string());
        let icon = world
            .query_filtered::<Entity, With<Playback>>()
            .single(&world)
            .unwrap();
        assert_eq!(image_name(&world, icon), Some(expected));
    }
}

#[test]
fn only_rename_control_can_capture_input_and_hud_uses_its_layer() {
    let mut world = World::new();
    spawn(&mut world);
    for entity in world.query_filtered::<Entity, With<Node>>().iter(&world) {
        let pickable = world.get::<bevy::picking::Pickable>(entity).unwrap();
        assert!(!pickable.should_block_lower && !pickable.is_hoverable);
        assert_eq!(world.get::<FocusPolicy>(entity), Some(&FocusPolicy::Pass));
        assert!(world.get::<Button>(entity).is_none());
        assert_eq!(
            world.get::<Interaction>(entity).is_some(),
            world.get::<RenameButton>(entity).is_some(),
            "only the explicit naming control owns pointer interaction"
        );
    }
    for z in world.query::<&GlobalZIndex>().iter(&world) {
        assert_eq!(z.0, 30);
    }
}

#[test]
fn real_dock_retains_priority_and_status_restores_when_it_fits() {
    let mut world = World::new();
    spawn(&mut world);
    care_dock::spawn(&mut world);
    let readout = Readout {
        name: None,
        holding_food: false,
        stable_id: 42,
        values: [Some(0.5); 6],
    };
    let panel = world
        .query_filtered::<Entity, With<HudRoot>>()
        .single(&world)
        .unwrap();
    for (size, visible) in [
        (Vec2::new(320.0, 240.0), false),
        (Vec2::new(320.0, 480.0), true),
        (Vec2::new(640.0, 360.0), true),
    ] {
        set_readout(&mut world, Some(&readout), false);
        adapt_bottom_controls_visibility(&mut world, size, true);
        assert_eq!(visible_in_hierarchy(&world, panel), visible);
    }
    assert_eq!(panel_bottom(Vec2::new(320.0, 240.0)), 236.0);
}

#[test]
fn explicit_help_uses_space_above_dock_and_temporarily_hides_selected_status() {
    let mut world = World::new();
    spawn(&mut world);
    care_dock::spawn(&mut world);
    let strip = world
        .spawn((
            V0PlayerControlStrip,
            Text::new("F1 Close help | Space Pause"),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            Node {
                bottom: Val::Px(92.0),
                ..default()
            },
            Visibility::Inherited,
        ))
        .id();
    let readout = Readout {
        name: None,
        holding_food: false,
        stable_id: 42,
        values: [Some(0.5); 6],
    };
    set_readout(&mut world, Some(&readout), false);
    adapt_bottom_controls_visibility(&mut world, Vec2::new(320.0, 240.0), true);
    let panel = world
        .query_filtered::<Entity, With<HudRoot>>()
        .single(&world)
        .unwrap();
    assert!(!visible_in_hierarchy(&world, panel));
    assert_eq!(world.get::<Visibility>(strip), Some(&Visibility::Inherited));
}

#[test]
fn food_badge_clears_and_restores_without_resizing_the_header_or_intercepting_input() {
    let mut app = layout_app(Vec2::new(320.0, 480.0));
    let badge = app
        .world_mut()
        .query_filtered::<Entity, With<HeldFood>>()
        .single(app.world())
        .unwrap();
    let header = app.world().get::<ChildOf>(badge).unwrap().parent();
    let readout = Readout {
        name: None,
        stable_id: u64::MAX,
        values: [Some(0.5); 6],
        holding_food: true,
    };
    let mut original_box = None;
    for (holding_food, selected, debug, expected_visible) in [
        (true, true, false, true),
        (false, true, false, false),
        (true, true, false, true),
        (true, false, false, false),
        (true, true, true, false),
        (true, true, false, true),
    ] {
        let next = Readout {
            holding_food,
            ..readout.clone()
        };
        set_readout(app.world_mut(), selected.then_some(&next), debug);
        app.update();
        app.update();
        assert_eq!(visible_in_hierarchy(app.world(), badge), expected_visible);
        let bounds = Box2::of(app.world(), header);
        assert!(bounds.contains(Box2::of(app.world(), badge)));
        if let Some((min, max)) = original_box {
            assert_eq!((bounds.min, bounds.max), (min, max));
        } else {
            original_box = Some((bounds.min, bounds.max));
        }
        assert_layout(app.world_mut(), Vec2::new(320.0, 480.0));
    }
}

#[test]
fn long_canonical_names_keep_full_accessible_text_and_a_separate_complete_numeric_id() {
    for size in VIEWPORTS {
        let mut app = layout_app(size);
        for name in ["W".repeat(32), "Étoile 🌙".to_owned(), "星".repeat(32)] {
            let mut row = data_tests::row();
            row.world_entity_id = WorldEntityId(u64::MAX);
            row.object.id = row.world_entity_id;
            row.object.label = name.clone();
            let readout = Readout::from_row(row.world_entity_id, row.organism_id, Some(&row), []);
            set_readout(app.world_mut(), Some(&readout), false);
            app.update();
            app.update();
            assert_layout(app.world_mut(), size);
            let world = app.world_mut();
            let name_entity = world
                .query_filtered::<Entity, With<CreatureName>>()
                .single(world)
                .unwrap();
            assert_eq!(world.get::<Text>(name_entity).unwrap().0, name);
            assert_eq!(
                world.get::<AccessibilityNode>(name_entity).unwrap().label(),
                Some(name.as_str())
            );
            assert_eq!(
                world.get::<Node>(name_entity).unwrap().overflow,
                Overflow::clip()
            );
            assert_eq!(
                world.get::<TextLayout>(name_entity).unwrap().linebreak,
                bevy::text::LineBreak::NoWrap
            );
            let identity = world
                .query_filtered::<Entity, With<Identity>>()
                .single(world)
                .unwrap();
            assert_eq!(
                world.get::<Text>(identity).unwrap().0,
                format!("#{}", u64::MAX)
            );
            let name_bounds = Box2::of(world, name_entity);
            let id_bounds = Box2::of(world, identity);
            assert!(!name_bounds.overlaps(id_bounds));
            assert!(
                id_bounds.size().x >= 150.0,
                "ID line lost its reserved space at {size:?}"
            );
            let header = world
                .query_filtered::<Entity, With<IdentityHeader>>()
                .single(world)
                .unwrap();
            assert_eq!(Box2::of(world, header).size().y, HEADER_HEIGHT);
        }
    }
    // Component values and UiSurface boxes are evidence; these tests do not
    // shape fonts, render unsupported Unicode glyphs, or claim screenshot QA.
}

#[test]
fn naming_button_is_the_only_input_target_and_fails_closed_without_a_living_identity() {
    let row = data_tests::row();
    let named = Readout::from_row(row.world_entity_id, row.organism_id, Some(&row), []);
    let unknown = Readout::from_row(row.world_entity_id, row.organism_id, None, []);
    let mut world = World::new();
    spawn(&mut world);
    let button = world
        .query_filtered::<Entity, With<RenameButton>>()
        .single(&world)
        .unwrap();
    for (readout, debug, enabled) in [
        (Some(&named), false, true),
        (Some(&unknown), false, false),
        (Some(&named), true, false),
        (None, false, false),
        (Some(&named), false, true),
    ] {
        set_readout(&mut world, readout, debug);
        let accessible = world.get::<AccessibilityNode>(button).unwrap();
        assert_eq!(accessible.role(), accesskit::Role::Button);
        assert!(accessible.supports_action(accesskit::Action::Click));
        assert_eq!(accessible.is_disabled(), !enabled);
        assert_eq!(accessible.is_hidden(), !enabled);
        assert_eq!(visible_in_hierarchy(&world, button), enabled);
        assert!(world.get::<Interaction>(button).is_some());
        if enabled {
            assert_eq!(
                accessible.label(),
                Some(format!("Name {}", row.object.label).as_str())
            );
        }
        for entity in world.query_filtered::<Entity, With<Node>>().iter(&world) {
            let pickable = world.get::<bevy::picking::Pickable>(entity).unwrap();
            let input = entity == button && enabled;
            assert_eq!(pickable.is_hoverable, input);
            assert_eq!(pickable.should_block_lower, input);
            assert_eq!(
                world.get::<FocusPolicy>(entity),
                Some(if input {
                    &FocusPolicy::Block
                } else {
                    &FocusPolicy::Pass
                })
            );
        }
    }
}
