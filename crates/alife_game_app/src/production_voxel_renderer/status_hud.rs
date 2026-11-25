//! Wordless selected-creature identity, physiology, possession and playback readout.
//! Icon meters show measured quantities, not six interchangeable needs or health scores.
use super::*;
use alife_world::{HeadlessWorld, WorldObject};
use bevy::a11y::AccessibilityNode;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use bevy::ui::{BorderColor, BorderRadius, FocusPolicy, UiRect};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
enum Quantity {
    Hunger,
    Energy,
    Fatigue,
    Calm,
    SleepPressure,
    PraiseSignal,
}
const QUANTITIES: [Quantity; 6] = [
    Quantity::Hunger,
    Quantity::Energy,
    Quantity::Fatigue,
    Quantity::Calm,
    Quantity::SleepPressure,
    Quantity::PraiseSignal,
];
impl Quantity {
    const fn index(self) -> usize {
        match self {
            Self::Hunger => 0,
            Self::Energy => 1,
            Self::Fatigue => 2,
            Self::Calm => 3,
            Self::SleepPressure => 4,
            Self::PraiseSignal => 5,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::Hunger => "hunger",
            Self::Energy => "energy",
            Self::Fatigue => "fatigue",
            Self::Calm => "calm",
            Self::SleepPressure => "sleep",
            Self::PraiseSignal => "praise",
        }
    }
    fn value(self, row: &WorldOrganismPresentationRow) -> Option<f32> {
        let h = &row.biochemistry.homeostasis;
        let raw = match self {
            Self::Hunger => h.drives.hunger,
            Self::Energy => row.biochemistry.body.energy,
            Self::Fatigue => h.drives.fatigue,
            Self::Calm => h.drives.fear,
            Self::SleepPressure => h.hormones.sleep_pressure,
            Self::PraiseSignal => h.hormones.extension[0],
        };
        // Validate before transformation: malformed fear must not become apparent calm.
        normalized(raw).map(|value| {
            if self == Self::Calm {
                1.0 - value
            } else {
                value
            }
        })
    }
    fn warning(self, value: f32) -> bool {
        match self {
            Self::Hunger | Self::Fatigue | Self::SleepPressure => value >= 0.65,
            Self::Energy | Self::Calm => value <= 0.35,
            Self::PraiseSignal => false,
        }
    }
    fn meter_color(self, value: f32) -> Color {
        if self.warning(value) {
            Color::srgb(0.96, 0.63, 0.28)
        } else if matches!(self, Self::Hunger | Self::Fatigue | Self::SleepPressure) {
            Color::srgb(0.65, 0.76, 0.82)
        } else {
            Color::srgb(0.42, 0.83, 0.71)
        }
    }
}
fn normalized(value: f32) -> Option<f32> {
    (value.is_finite() && (0.0..=1.0).contains(&value)).then_some(value)
}

#[derive(Debug, Clone, PartialEq)]
struct Readout {
    stable_id: u64,
    name: Option<String>,
    values: [Option<f32>; 6],
    holding_food: bool,
}
impl Readout {
    fn from_row<'a>(
        stable_id: WorldEntityId,
        organism_id: OrganismId,
        row: Option<&WorldOrganismPresentationRow>,
        objects: impl IntoIterator<Item = &'a WorldObject>,
    ) -> Self {
        let row = row.filter(|row| valid_row(stable_id, organism_id, row));
        Self {
            stable_id: stable_id.raw(),
            name: row.map(|row| row.object.label.clone()),
            values: QUANTITIES.map(|quantity| row.and_then(|row| quantity.value(row))),
            // Possession is physical ownership, never a nearby fruit or a motor intention.
            holding_food: row.is_some()
                && objects.into_iter().any(|object| {
                    object.kind == WorldObjectKind::Food
                        && !object.consumed
                        && object.carried_by == Some(organism_id)
                }),
        }
    }
}
fn valid_row(
    stable_id: WorldEntityId,
    organism_id: OrganismId,
    row: &WorldOrganismPresentationRow,
) -> bool {
    stable_id.is_valid()
        && organism_id.is_valid()
        && row.world_entity_id == stable_id
        && row.organism_id == organism_id
        && row.object.id == stable_id
        && row.object.kind == WorldObjectKind::Agent
        && row.object.organism_id == Some(organism_id)
        && !row.object.consumed
        && row.lifecycle.is_alive()
}
#[derive(Component)]
struct HudRoot;
#[derive(Component)]
struct Identity;
#[derive(Component)]
struct IdentityHeader;
#[derive(Component)]
struct CreatureName;
/// Pointer and accessibility entry point for the shared naming composer.
#[derive(Component)]
pub(crate) struct RenameButton;
#[derive(Component)]
struct HeldFood;
#[derive(Component)]
struct QuantityGrid;
#[derive(Component)]
struct QuantityIcon;
#[derive(Component)]
struct MeterTrack(Quantity);
#[derive(Component)]
struct MeterFill(Quantity);
#[derive(Component)]
struct Unknown(Quantity);
#[derive(Component)]
struct Warning(Quantity);
#[derive(Component)]
struct Count;
#[derive(Component)]
struct StatusRoot;
#[derive(Component)]
struct Playback;
#[derive(Resource)]
pub(super) struct Icons(BTreeMap<&'static str, Handle<Image>>);
const PANEL: Color = Color::srgb(0.075, 0.115, 0.135);
const CARD: Color = Color::srgb(0.050, 0.075, 0.090);
const INK: Color = Color::srgb(0.98, 0.96, 0.84);
const HEADER_HEIGHT: f32 = 36.0;
const EMBEDDED: [(&str, &[u8]); 13] = [
    (
        "holding_food",
        include_bytes!("../../../../assets/ui/status_hud/holding_food.png"),
    ),
    (
        "hunger",
        include_bytes!("../../../../assets/ui/status_hud/hunger.png"),
    ),
    (
        "energy",
        include_bytes!("../../../../assets/ui/status_hud/energy.png"),
    ),
    (
        "fatigue",
        include_bytes!("../../../../assets/ui/status_hud/fatigue.png"),
    ),
    (
        "calm",
        include_bytes!("../../../../assets/ui/status_hud/calm.png"),
    ),
    (
        "sleep",
        include_bytes!("../../../../assets/ui/status_hud/sleep.png"),
    ),
    (
        "praise",
        include_bytes!("../../../../assets/ui/status_hud/praise.png"),
    ),
    (
        "selection",
        include_bytes!("../../../../assets/ui/status_hud/selection.png"),
    ),
    (
        "population",
        include_bytes!("../../../../assets/ui/status_hud/population.png"),
    ),
    (
        "playing",
        include_bytes!("../../../../assets/ui/status_hud/playing.png"),
    ),
    (
        "paused",
        include_bytes!("../../../../assets/ui/status_hud/paused.png"),
    ),
    (
        "fast",
        include_bytes!("../../../../assets/ui/status_hud/fast.png"),
    ),
    (
        "warning",
        include_bytes!("../../../../assets/ui/status_hud/warning.png"),
    ),
];
fn bevel(raised: bool) -> BorderColor {
    let light = Color::srgb(0.43, 0.53, 0.55);
    let dark = Color::srgb(0.025, 0.040, 0.050);
    BorderColor {
        top: if raised { light } else { dark },
        left: if raised { light } else { dark },
        bottom: if raised { dark } else { light },
        right: if raised { dark } else { light },
    }
}
fn icon(world: &mut World, parent: Entity, handle: Handle<Image>, size: f32) -> Entity {
    world
        .spawn((
            ImageNode::new(handle),
            Node {
                width: Val::Px(size),
                height: Val::Px(size),
                flex_shrink: 0.0,
                ..default()
            },
            ChildOf(parent),
        ))
        .id()
}
pub(super) fn spawn(world: &mut World) {
    if !world.contains_resource::<Assets<Image>>() {
        world.init_resource::<Assets<Image>>();
    }
    let mut handles = BTreeMap::new();
    for (name, bytes) in EMBEDDED {
        let image = Image::from_buffer(
            bytes,
            ImageType::Extension("png"),
            CompressedImageFormats::NONE,
            true,
            ImageSampler::linear(),
            RenderAssetUsages::default(),
        )
        .expect("embedded HUD PNG is validated by CPU tests");
        handles.insert(name, world.resource_mut::<Assets<Image>>().add(image));
    }
    let image = |name| handles.get(name).unwrap().clone();
    let status = world
        .spawn((
            Name::new("Rendered creature count and playback"),
            StatusRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                top: Val::Px(16.0),
                height: Val::Px(48.0),
                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                border: UiRect::all(Val::Px(3.0)),
                border_radius: BorderRadius::all(Val::Px(16.0)),
                column_gap: Val::Px(8.0),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(PANEL),
            bevel(true),
            GlobalZIndex(30),
        ))
        .id();
    icon(world, status, image("population"), 26.0);
    world.spawn((
        Text::new("0"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(INK),
        Node {
            min_width: Val::Px(26.0),
            height: Val::Px(22.0),
            ..default()
        },
        Count,
        V0PlayerStatusChip,
        ChildOf(status),
    ));
    let playback = icon(world, status, image("playing"), 22.0);
    world.entity_mut(playback).insert(Playback);
    let root = world
        .spawn((
            Name::new("Selected creature physiology readout"),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                top: Val::Px(76.0),
                width: Val::Px(370.0),
                padding: UiRect::all(Val::Px(6.0)),
                border: UiRect::all(Val::Px(3.0)),
                border_radius: BorderRadius::all(Val::Px(18.0)),
                row_gap: Val::Px(4.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(PANEL),
            bevel(true),
            GlobalZIndex(30),
            HudRoot,
            V0PlayerCreaturePanel,
            Visibility::Hidden,
        ))
        .id();
    let identity = world
        .spawn((
            Name::new("Actual selected stable identity, no portrait"),
            IdentityHeader,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(HEADER_HEIGHT),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                ..default()
            },
            ChildOf(root),
        ))
        .id();
    icon(world, identity, image("selection"), 16.0);
    let identity_text = world
        .spawn((
            Node {
                min_width: Val::Px(0.0),
                height: Val::Px(32.0),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ChildOf(identity),
        ))
        .id();
    world.spawn((
        Text::new(""),
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(INK),
        TextLayout::new_with_linebreak(bevy::text::LineBreak::NoWrap),
        Node {
            width: Val::Percent(100.0),
            min_width: Val::Px(0.0),
            height: Val::Px(18.0),
            overflow: Overflow::clip(),
            flex_shrink: 0.0,
            ..default()
        },
        CreatureName,
        AccessibilityNode(accesskit::Node::new(accesskit::Role::Label)),
        ChildOf(identity_text),
    ));
    world.spawn((
        Text::new(""),
        TextFont {
            font_size: 10.0,
            ..default()
        },
        TextColor(INK),
        TextLayout::new_with_linebreak(bevy::text::LineBreak::NoWrap),
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(14.0),
            flex_shrink: 0.0,
            ..default()
        },
        Identity,
        ChildOf(identity_text),
    ));
    let held_food = icon(world, identity, image("holding_food"), 22.0);
    world.entity_mut(held_food).insert((
        Name::new("Selected creature holds unconsumed food"),
        HeldFood,
        Visibility::Hidden,
    ));
    let mut rename_accessibility = accesskit::Node::new(accesskit::Role::Button);
    rename_accessibility.set_label("Name selected creature");
    rename_accessibility.add_action(accesskit::Action::Click);
    rename_accessibility.set_disabled();
    rename_accessibility.set_hidden();
    let rename = world
        .spawn((
            Name::new("Name selected creature"),
            RenameButton,
            Interaction::None,
            Node {
                width: Val::Px(36.0),
                height: Val::Px(36.0),
                flex_shrink: 0.0,
                border: UiRect::all(Val::Px(2.0)),
                border_radius: BorderRadius::all(Val::Px(10.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(CARD),
            bevel(true),
            Visibility::Hidden,
            AccessibilityNode(rename_accessibility),
            ChildOf(identity),
        ))
        .id();
    // A pencil built from native UI shapes needs no font glyph or image asset.
    let pencil = world
        .spawn((
            Node {
                width: Val::Px(18.0),
                height: Val::Px(6.0),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                ..default()
            },
            UiTransform::from_rotation(Rot2::degrees(-45.0)),
            ChildOf(rename),
        ))
        .id();
    for (width, height, color) in [
        (3.0, 6.0, Color::srgb(0.78, 0.56, 0.43)),
        (11.0, 6.0, INK),
        (4.0, 3.0, Color::srgb(0.42, 0.83, 0.71)),
    ] {
        world.spawn((
            Node {
                width: Val::Px(width),
                height: Val::Px(height),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(color),
            ChildOf(pencil),
        ));
    }
    let grid = world
        .spawn((
            QuantityGrid,
            Node {
                width: Val::Percent(100.0),
                column_gap: Val::Px(8.0),
                row_gap: Val::Px(6.0),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            ChildOf(root),
        ))
        .id();
    for quantity in QUANTITIES {
        let card = world
            .spawn((
                Name::new(format!("{} measured quantity", quantity.name())),
                quantity,
                Node {
                    position_type: PositionType::Relative,
                    width: Val::Px(52.0),
                    height: Val::Px(68.0),
                    flex_shrink: 0.0,
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(13.0)),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    row_gap: Val::Px(8.0),
                    ..default()
                },
                BackgroundColor(CARD),
                bevel(false),
                ChildOf(grid),
            ))
            .id();
        let quantity_icon = icon(world, card, image(quantity.name()), 32.0);
        world.entity_mut(quantity_icon).insert(QuantityIcon);
        let track = world
            .spawn((
                Node {
                    width: Val::Px(38.0),
                    height: Val::Px(5.0),
                    border_radius: BorderRadius::all(Val::Px(2.5)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.17, 0.24, 0.27)),
                MeterTrack(quantity),
                ChildOf(card),
            ))
            .id();
        world.spawn((
            Node {
                width: Val::Percent(0.0),
                height: Val::Percent(100.0),
                border_radius: BorderRadius::all(Val::Px(2.5)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.42, 0.83, 0.71)),
            MeterFill(quantity),
            ChildOf(track),
        ));
        world.spawn((
            Text::new("?"),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgb(0.65, 0.75, 0.80)),
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(0.0),
                width: Val::Px(14.0),
                height: Val::Px(14.0),
                ..default()
            },
            Unknown(quantity),
            ChildOf(card),
        ));
        world.spawn((
            ImageNode::new(image("warning")),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                right: Val::Px(0.0),
                width: Val::Px(12.0),
                height: Val::Px(12.0),
                ..default()
            },
            Visibility::Hidden,
            Warning(quantity),
            ChildOf(card),
        ));
    }
    // Meters remain passive. Only the explicit naming control accepts input.
    let mut pending = vec![status, root];
    while let Some(entity) = pending.pop() {
        if let Some(children) = world.get::<Children>(entity) {
            pending.extend(children.iter());
        }
        world
            .entity_mut(entity)
            .insert((bevy::picking::Pickable::IGNORE, FocusPolicy::Pass));
    }
    world.insert_resource(Icons(handles));
}
#[derive(bevy::ecs::system::SystemParam)]
#[allow(
    clippy::type_complexity,
    reason = "Bevy SystemParam query bundle groups disjoint HUD queries."
)]
pub(super) struct HudSyncQueries<'w, 's> {
    status_roots: Query<
        'w,
        's,
        &'static mut Visibility,
        (
            With<StatusRoot>,
            Without<HudRoot>,
            Without<HeldFood>,
            Without<RenameButton>,
            Without<MeterTrack>,
            Without<Unknown>,
            Without<Warning>,
            Without<V0PlayerControlStrip>,
        ),
    >,
    hud_roots: Query<
        'w,
        's,
        (&'static mut Node, &'static mut Visibility),
        (
            With<HudRoot>,
            Without<StatusRoot>,
            Without<HeldFood>,
            Without<RenameButton>,
            Without<MeterTrack>,
            Without<Unknown>,
            Without<Warning>,
            Without<V0PlayerControlStrip>,
            Without<Quantity>,
            Without<QuantityIcon>,
            Without<MeterFill>,
        ),
    >,
    identities: Query<
        'w,
        's,
        &'static mut Text,
        (
            With<Identity>,
            Without<CreatureName>,
            Without<Count>,
            Without<V0PlayerControlStrip>,
        ),
    >,
    creature_names: Query<
        'w,
        's,
        (&'static mut Text, &'static mut AccessibilityNode),
        (
            With<CreatureName>,
            Without<Identity>,
            Without<Count>,
            Without<RenameButton>,
            Without<V0PlayerControlStrip>,
        ),
    >,
    rename_buttons: Query<
        'w,
        's,
        (
            &'static mut Visibility,
            &'static mut bevy::picking::Pickable,
            &'static mut FocusPolicy,
            &'static mut Interaction,
            &'static mut BorderColor,
            &'static mut AccessibilityNode,
        ),
        (
            With<RenameButton>,
            Without<StatusRoot>,
            Without<HudRoot>,
            Without<HeldFood>,
            Without<MeterTrack>,
            Without<Unknown>,
            Without<Warning>,
            Without<V0PlayerControlStrip>,
            Without<CreatureName>,
        ),
    >,
    held_foods: Query<
        'w,
        's,
        &'static mut Visibility,
        (
            With<HeldFood>,
            Without<StatusRoot>,
            Without<HudRoot>,
            Without<RenameButton>,
            Without<MeterTrack>,
            Without<Unknown>,
            Without<Warning>,
            Without<V0PlayerControlStrip>,
        ),
    >,
    meter_fills: Query<
        'w,
        's,
        (
            &'static MeterFill,
            &'static mut Node,
            &'static mut BackgroundColor,
        ),
        (
            Without<HudRoot>,
            Without<Quantity>,
            Without<QuantityIcon>,
            Without<V0PlayerControlStrip>,
        ),
    >,
    meter_tracks: Query<
        'w,
        's,
        (&'static MeterTrack, &'static mut Visibility),
        (
            Without<StatusRoot>,
            Without<HudRoot>,
            Without<HeldFood>,
            Without<RenameButton>,
            Without<Unknown>,
            Without<Warning>,
            Without<V0PlayerControlStrip>,
        ),
    >,
    unknowns: Query<
        'w,
        's,
        (&'static Unknown, &'static mut Visibility),
        (
            Without<StatusRoot>,
            Without<HudRoot>,
            Without<HeldFood>,
            Without<RenameButton>,
            Without<MeterTrack>,
            Without<Warning>,
            Without<V0PlayerControlStrip>,
        ),
    >,
    warnings: Query<
        'w,
        's,
        (&'static Warning, &'static mut Visibility),
        (
            Without<StatusRoot>,
            Without<HudRoot>,
            Without<HeldFood>,
            Without<RenameButton>,
            Without<MeterTrack>,
            Without<Unknown>,
            Without<V0PlayerControlStrip>,
        ),
    >,
    quantities: Query<
        'w,
        's,
        &'static mut Node,
        (
            With<Quantity>,
            Without<HudRoot>,
            Without<MeterFill>,
            Without<QuantityIcon>,
            Without<V0PlayerControlStrip>,
        ),
    >,
    quantity_icons: Query<
        'w,
        's,
        &'static mut Node,
        (
            With<QuantityIcon>,
            Without<HudRoot>,
            Without<MeterFill>,
            Without<Quantity>,
            Without<V0PlayerControlStrip>,
        ),
    >,
    playbacks: Query<'w, 's, &'static mut ImageNode, With<Playback>>,
    counts: Query<
        'w,
        's,
        &'static mut Text,
        (
            With<Count>,
            Without<Identity>,
            Without<CreatureName>,
            Without<V0PlayerControlStrip>,
        ),
    >,
}

fn apply_readout_queries(
    queries: &mut HudSyncQueries<'_, '_>,
    readout: Option<&Readout>,
    debug: bool,
    status_visible: Visibility,
    hud_visible: Visibility,
) {
    for mut visibility in &mut queries.status_roots {
        visibility.set_if_neq(status_visible);
    }
    for (_, mut visible) in &mut queries.hud_roots {
        visible.set_if_neq(hud_visible);
    }
    for mut text in &mut queries.identities {
        let next = readout.map_or_else(String::new, |r| format!("#{}", r.stable_id));
        if text.0 != next {
            text.0 = next;
        }
    }
    let name = readout.and_then(|r| r.name.as_deref());
    for (mut text, mut accessible) in &mut queries.creature_names {
        let next = name.unwrap_or("");
        if text.0 != next {
            text.0 = next.to_owned();
        }
        // The complete canonical name remains available when the line clips.
        if accessible.label() != Some(next) {
            accessible.set_label(next);
        }
    }
    for (mut visible, mut pickable, mut focus, mut interaction, mut border, mut accessible) in
        &mut queries.rename_buttons
    {
        let active_name = name.filter(|_| !debug);
        if let Some(name) = active_name {
            visible.set_if_neq(Visibility::Inherited);
            let next_pickable = bevy::picking::Pickable::default();
            if *pickable != next_pickable {
                *pickable = next_pickable;
            }
            if *focus != FocusPolicy::Block {
                *focus = FocusPolicy::Block;
            }
            let next_label = format!("Name {name}");
            if accessible.label() != Some(next_label.as_str()) {
                accessible.set_label(next_label);
            }
            if accessible.is_disabled() {
                accessible.clear_disabled();
            }
            if accessible.is_hidden() {
                accessible.clear_hidden();
            }
        } else {
            visible.set_if_neq(Visibility::Hidden);
            if *pickable != bevy::picking::Pickable::IGNORE {
                *pickable = bevy::picking::Pickable::IGNORE;
            }
            if *focus != FocusPolicy::Pass {
                *focus = FocusPolicy::Pass;
            }
            if *interaction != Interaction::None {
                *interaction = Interaction::None;
            }
            if accessible.label() != Some("Name selected creature") {
                accessible.set_label("Name selected creature");
            }
            if !accessible.is_disabled() {
                accessible.set_disabled();
            }
            if !accessible.is_hidden() {
                accessible.set_hidden();
            }
        }
        let pressed = active_name.is_some() && *interaction == Interaction::Pressed;
        let next_border = bevel(!pressed);
        if *border != next_border {
            *border = next_border;
        }
    }
    let values = readout.map_or([None; 6], |r| r.values);
    let held_food_visible = if readout.is_some_and(|readout| readout.holding_food) {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut visibility in &mut queries.held_foods {
        visibility.set_if_neq(held_food_visible);
    }
    for (fill, mut node, mut color) in &mut queries.meter_fills {
        let amount = values[fill.0.index()];
        let width = Val::Percent(amount.unwrap_or(0.0) * 100.0);
        if node.width != width {
            node.width = width;
        }
        let next = amount.map_or(Color::NONE, |v| fill.0.meter_color(v));
        if color.0 != next {
            color.0 = next;
        }
    }
    for (track, mut visible) in &mut queries.meter_tracks {
        visible.set_if_neq(if values[track.0.index()].is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
    for (unknown, mut visible) in &mut queries.unknowns {
        visible.set_if_neq(if values[unknown.0.index()].is_none() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
    for (warning, mut visible) in &mut queries.warnings {
        visible.set_if_neq(
            if values[warning.0.index()].is_some_and(|v| warning.0.warning(v)) {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            },
        );
    }
}

fn apply_layout_queries(queries: &mut HudSyncQueries<'_, '_>, size: Vec2) {
    let width = Val::Px((size.x - 32.0).clamp(0.0, 370.0));
    for (mut node, _) in &mut queries.hud_roots {
        if node.width != width {
            node.width = width;
        }
    }
    let short = size.y < 300.0;
    let quantity_height = Val::Px(if short { 48.0 } else { 68.0 });
    let quantity_gap = Val::Px(if short { 4.0 } else { 8.0 });
    for mut node in &mut queries.quantities {
        if node.height != quantity_height {
            node.height = quantity_height;
        }
        if node.row_gap != quantity_gap {
            node.row_gap = quantity_gap;
        }
    }
    let icon_size = Val::Px(if short { 24.0 } else { 32.0 });
    for mut node in &mut queries.quantity_icons {
        if node.width != icon_size {
            node.width = icon_size;
        }
        if node.height != icon_size {
            node.height = icon_size;
        }
    }
}

fn apply_status_queries(
    queries: &mut HudSyncQueries<'_, '_>,
    icons: Option<&Icons>,
    count: usize,
    paused: bool,
    fast: bool,
) {
    let key = if paused {
        "paused"
    } else if fast {
        "fast"
    } else {
        "playing"
    };
    if let Some(handle) = icons.and_then(|icons| icons.0.get(key)) {
        for mut image in &mut queries.playbacks {
            if image.image != *handle {
                image.image = handle.clone();
            }
        }
    }
    let next = count.to_string();
    for mut text in &mut queries.counts {
        if text.0 != next {
            text.0.clone_from(&next);
        }
    }
}

#[cfg(test)]
fn set_readout(world: &mut World, readout: Option<&Readout>, debug: bool) {
    let mut state: bevy::ecs::system::SystemState<HudSyncQueries<'_, '_>> =
        bevy::ecs::system::SystemState::new(world);
    let mut queries = state.get_mut(world);
    let status_visible = if debug {
        Visibility::Hidden
    } else {
        Visibility::Visible
    };
    let hud_visible = if readout.is_some() && !debug {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    apply_readout_queries(&mut queries, readout, debug, status_visible, hud_visible);
}

#[cfg(test)]
fn apply_layout(world: &mut World, size: Vec2) {
    let mut state: bevy::ecs::system::SystemState<HudSyncQueries<'_, '_>> =
        bevy::ecs::system::SystemState::new(world);
    let mut queries = state.get_mut(world);
    apply_layout_queries(&mut queries, size);
}

fn selected_readout_from_parts(
    entity_map: Option<&BevyEntityMap>,
    creature_roots: &Query<(
        &ProductionCreatureAssemblyRoot,
        &Fvr04ProductionCreatureVisualMarker,
    )>,
    frame: Option<&LiveBrainPresentationFrameResource>,
    canonical: Option<&HeadlessWorld>,
    stable_id: WorldEntityId,
) -> Readout {
    let identity = entity_map
        .and_then(|map| map.bevy_entity(stable_id))
        .and_then(|entity| {
            let (root, visual) = creature_roots.get(entity).ok()?;
            (root.stable_id == stable_id
                && visual.stable_id == stable_id
                && root.organism_id == visual.organism_id)
                .then_some(root.organism_id)
        });
    let row = frame.and_then(|frame| frame.current.organism(stable_id));
    identity.map_or(
        Readout {
            stable_id: stable_id.raw(),
            name: None,
            values: [None; 6],
            holding_food: false,
        },
        |organism_id| {
            let mut readout = Readout::from_row(
                stable_id,
                organism_id,
                row,
                frame.into_iter().flat_map(|frame| frame.current.objects()),
            );
            readout.name = selected_name(canonical, stable_id, organism_id, row);
            readout
        },
    )
}

#[cfg(test)]
fn selected_readout(world: &World, stable_id: WorldEntityId) -> Readout {
    let identity = world
        .get_resource::<BevyEntityMap>()
        .and_then(|map| map.bevy_entity(stable_id))
        .and_then(|entity| {
            let root = world.get::<ProductionCreatureAssemblyRoot>(entity)?;
            let visual = world.get::<Fvr04ProductionCreatureVisualMarker>(entity)?;
            (root.stable_id == stable_id
                && visual.stable_id == stable_id
                && root.organism_id == visual.organism_id)
                .then_some(root.organism_id)
        });
    let frame = world.get_resource::<LiveBrainPresentationFrameResource>();
    let row = frame.and_then(|frame| frame.current.organism(stable_id));
    identity.map_or(
        Readout {
            stable_id: stable_id.raw(),
            name: None,
            values: [None; 6],
            holding_food: false,
        },
        |organism_id| {
            let mut readout = Readout::from_row(
                stable_id,
                organism_id,
                row,
                frame.into_iter().flat_map(|frame| frame.current.objects()),
            );
            #[cfg(feature = "gpu-runtime")]
            let runtime_guard = world
                .get_resource::<ProductionGpuBrainRuntimeResource>()
                .map(|runtime| runtime.lock());
            #[cfg(feature = "gpu-runtime")]
            let canonical = runtime_guard.as_deref().map(|runtime| runtime.world());
            #[cfg(not(feature = "gpu-runtime"))]
            let canonical = None;
            readout.name = selected_name(canonical, stable_id, organism_id, row);
            readout
        },
    )
}
fn selected_name(
    canonical: Option<&HeadlessWorld>,
    stable_id: WorldEntityId,
    organism_id: OrganismId,
    row: Option<&WorldOrganismPresentationRow>,
) -> Option<String> {
    if let Some(canonical) = canonical {
        // Names are world metadata and can change while presentation is paused.
        // An available runtime is authoritative even when its target is absent.
        let object = canonical.entity(stable_id)?;
        let record = canonical.organism_registry().get(organism_id)?;
        return (stable_id.is_valid()
            && organism_id.is_valid()
            && object.id == stable_id
            && object.kind == WorldObjectKind::Agent
            && object.organism_id == Some(organism_id)
            && !object.consumed
            && record.world_entity_id() == stable_id
            && record.organism_id() == organism_id
            && record.lifecycle().is_alive())
        .then(|| object.label.clone());
    }
    row.filter(|row| valid_row(stable_id, organism_id, row))
        .map(|row| row.object.label.clone())
}

#[cfg(test)]
fn set_status(world: &mut World, count: usize, paused: bool, fast: bool) {
    let mut state: bevy::ecs::system::SystemState<(Option<Res<Icons>>, HudSyncQueries<'_, '_>)> =
        bevy::ecs::system::SystemState::new(world);
    let (icons, mut queries) = state.get_mut(world);
    apply_status_queries(&mut queries, icons.as_deref(), count, paused, fast);
}

fn panel_bottom(size: Vec2) -> f32 {
    let width = (size.x - 32.0).clamp(0.0, 370.0);
    let columns = ((width - 18.0 + 8.0) / 60.0).floor().max(1.0) as usize;
    let rows = QUANTITIES.len().div_ceil(columns);
    76.0 + 18.0
        + HEADER_HEIGHT
        + 4.0
        + rows as f32 * if size.y < 300.0 { 48.0 } else { 68.0 }
        + (rows - 1) as f32 * 6.0
}

#[cfg(test)]
fn adapt_bottom_controls_visibility(world: &mut World, size: Vec2, selected_visible: bool) {
    let occupied_top = care_dock::occupied_top(world, size);
    if occupied_top < 72.0 {
        for mut visible in world
            .query_filtered::<&mut Visibility, With<StatusRoot>>()
            .iter_mut(world)
        {
            visible.set_if_neq(Visibility::Hidden);
        }
    }
    if selected_visible && panel_bottom(size) + 8.0 > occupied_top {
        for mut visible in world
            .query_filtered::<&mut Visibility, With<HudRoot>>()
            .iter_mut(world)
        {
            visible.set_if_neq(Visibility::Hidden);
        }
    }
}

#[allow(
    clippy::too_many_arguments,
    clippy::type_complexity,
    reason = "Bevy injects independent ECS system parameters and disjoint HUD queries."
)]
pub(super) fn sync(
    ux: Res<Fvr05ProductionUxStateResource>,
    scene: Res<Fvr03ProductionVoxelSceneResource>,
    selection: Res<Fvr03ProductionVoxelSelectionResource>,
    windows: Query<&Window, With<PrimaryWindow>>,
    icons: Option<Res<Icons>>,
    entity_map: Option<Res<BevyEntityMap>>,
    frame: Option<Res<LiveBrainPresentationFrameResource>>,
    #[cfg(feature = "gpu-runtime")] runtime: Option<Res<ProductionGpuBrainRuntimeResource>>,
    #[cfg(feature = "gpu-runtime")] conversation: Option<
        Res<crate::ProductionConversationLineageUiState>,
    >,
    strips: Query<
        (&Text, &TextFont, &Node, &Visibility, Option<&ComputedNode>),
        With<V0PlayerControlStrip>,
    >,
    creature_roots: Query<(
        &ProductionCreatureAssemblyRoot,
        &Fvr04ProductionCreatureVisualMarker,
    )>,
    mut queries: HudSyncQueries<'_, '_>,
) {
    let debug = ux.debug_mode;
    let paused = ux.settings.paused;
    let fast = ux.settings.playback_mode() != crate::ProductionRunMode::OneX;
    let count = scene.creature_render_count;
    let selected = selection
        .selected
        .filter(|r| r.kind == StableVoxelRefKind::Creature)
        .and_then(|r| r.stable_id);
    #[cfg(feature = "gpu-runtime")]
    let runtime_guard = runtime.as_ref().map(|runtime| runtime.lock());
    #[cfg(feature = "gpu-runtime")]
    let canonical = runtime_guard.as_deref().map(|runtime| runtime.world());
    #[cfg(not(feature = "gpu-runtime"))]
    let canonical = None;
    let readout = selected.filter(|_| !debug).map(|stable_id| {
        selected_readout_from_parts(
            entity_map.as_deref(),
            &creature_roots,
            frame.as_deref(),
            canonical,
            stable_id,
        )
    });
    let size = windows
        .iter()
        .next()
        .map_or(Vec2::new(1280.0, 720.0), |window| {
            Vec2::new(window.width(), window.height())
        });
    let occupied_top = care_dock::occupied_top_from_parts(
        size,
        ux.last_error.is_some(),
        #[cfg(feature = "gpu-runtime")]
        conversation.as_deref(),
        strips,
    );
    let status_visible = if debug || occupied_top < 72.0 {
        Visibility::Hidden
    } else {
        Visibility::Visible
    };
    let selected_visible = readout.is_some() && !debug;
    let hud_visible = if selected_visible && panel_bottom(size) + 8.0 <= occupied_top {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    apply_readout_queries(
        &mut queries,
        readout.as_ref(),
        debug,
        status_visible,
        hud_visible,
    );
    apply_status_queries(&mut queries, icons.as_deref(), count, paused, fast);
    apply_layout_queries(&mut queries, size);
}

#[cfg(test)]
#[path = "status_hud/data_tests.rs"]
mod data_tests;

#[cfg(test)]
#[path = "status_hud/layout_tests.rs"]
mod layout_tests;
