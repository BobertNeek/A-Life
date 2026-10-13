//! Read-only, wordless selected-creature physiology and playback readout.
//! Icon meters show measured quantities, not six interchangeable needs or health scores.
use super::*;
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
    values: [Option<f32>; 6],
}
impl Readout {
    fn from_row(
        stable_id: WorldEntityId,
        organism_id: OrganismId,
        row: Option<&WorldOrganismPresentationRow>,
    ) -> Self {
        let row = row.filter(|row| {
            row.world_entity_id == stable_id
                && row.organism_id == organism_id
                && row.object.id == stable_id
                && row.object.kind == WorldObjectKind::Agent
                && row.object.organism_id == Some(organism_id)
                && row.lifecycle.is_alive()
        });
        Self {
            stable_id: stable_id.raw(),
            values: QUANTITIES.map(|quantity| row.and_then(|row| quantity.value(row))),
        }
    }
}
#[derive(Component)]
struct HudRoot;
#[derive(Component)]
struct Identity;
#[derive(Component)]
struct QuantityGrid;
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
struct Icons(BTreeMap<&'static str, Handle<Image>>);
const PANEL: Color = Color::srgb(0.075, 0.115, 0.135);
const CARD: Color = Color::srgb(0.050, 0.075, 0.090);
const INK: Color = Color::srgb(0.98, 0.96, 0.84);
const EMBEDDED: [(&str, &[u8]); 12] = [
    ("hunger", include_bytes!("status_hud/icons/hunger.png")),
    ("energy", include_bytes!("status_hud/icons/energy.png")),
    ("fatigue", include_bytes!("status_hud/icons/fatigue.png")),
    ("calm", include_bytes!("status_hud/icons/calm.png")),
    ("sleep", include_bytes!("status_hud/icons/sleep.png")),
    ("praise", include_bytes!("status_hud/icons/praise.png")),
    (
        "selection",
        include_bytes!("status_hud/icons/selection.png"),
    ),
    (
        "population",
        include_bytes!("status_hud/icons/population.png"),
    ),
    ("playing", include_bytes!("status_hud/icons/playing.png")),
    ("paused", include_bytes!("status_hud/icons/paused.png")),
    ("fast", include_bytes!("status_hud/icons/fast.png")),
    ("warning", include_bytes!("status_hud/icons/warning.png")),
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
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(22.0),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                ..default()
            },
            ChildOf(root),
        ))
        .id();
    icon(world, identity, image("selection"), 16.0);
    world.spawn((
        Text::new(""),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(INK),
        Node {
            width: Val::Px(180.0),
            height: Val::Px(18.0),
            ..default()
        },
        Identity,
        ChildOf(identity),
    ));
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
        icon(world, card, image(quantity.name()), 32.0);
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
    // A readout never becomes an input target or blocks world/menu picking.
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
fn set_readout(world: &mut World, readout: Option<&Readout>, debug: bool) {
    for mut visibility in world
        .query_filtered::<&mut Visibility, With<StatusRoot>>()
        .iter_mut(world)
    {
        *visibility = if debug {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
    for mut visible in world
        .query_filtered::<&mut Visibility, With<HudRoot>>()
        .iter_mut(world)
    {
        let next = if readout.is_some() && !debug {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *visible != next {
            *visible = next;
        }
    }
    for mut text in world
        .query_filtered::<&mut Text, With<Identity>>()
        .iter_mut(world)
    {
        let next = readout.map_or_else(String::new, |r| format!("#{}", r.stable_id));
        if text.0 != next {
            text.0 = next;
        }
    }
    let values = readout.map_or([None; 6], |r| r.values);
    let value = |quantity| values[QUANTITIES.iter().position(|q| *q == quantity).unwrap()];
    for (fill, mut node, mut color) in world
        .query::<(&MeterFill, &mut Node, &mut BackgroundColor)>()
        .iter_mut(world)
    {
        let amount = value(fill.0);
        let width = Val::Percent(amount.unwrap_or(0.0) * 100.0);
        if node.width != width {
            node.width = width;
        }
        let next = amount.map_or(Color::NONE, |v| fill.0.meter_color(v));
        if color.0 != next {
            color.0 = next;
        }
    }
    for (track, mut visible) in world
        .query::<(&MeterTrack, &mut Visibility)>()
        .iter_mut(world)
    {
        *visible = if value(track.0).is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (unknown, mut visible) in world.query::<(&Unknown, &mut Visibility)>().iter_mut(world) {
        *visible = if value(unknown.0).is_none() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (warning, mut visible) in world.query::<(&Warning, &mut Visibility)>().iter_mut(world) {
        *visible = if value(warning.0).is_some_and(|v| warning.0.warning(v)) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
fn apply_layout(world: &mut World, size: Vec2) {
    let width = (size.x - 32.0).clamp(0.0, 370.0);
    for mut node in world
        .query_filtered::<&mut Node, With<HudRoot>>()
        .iter_mut(world)
    {
        if node.width != Val::Px(width) {
            node.width = Val::Px(width);
        }
    }
    let short = size.y < 300.0;
    for mut node in world
        .query_filtered::<&mut Node, With<Quantity>>()
        .iter_mut(world)
    {
        node.height = Val::Px(if short { 56.0 } else { 68.0 });
        node.row_gap = Val::Px(if short { 4.0 } else { 8.0 });
    }
}
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
    let row = world
        .get_resource::<LiveBrainPresentationFrameResource>()
        .and_then(|frame| frame.current.organism(stable_id));
    identity.map_or(
        Readout {
            stable_id: stable_id.raw(),
            values: [None; 6],
        },
        |organism_id| Readout::from_row(stable_id, organism_id, row),
    )
}
fn set_status(world: &mut World, count: usize, paused: bool, fast: bool) {
    let handle = world.resource::<Icons>().0[if paused {
        "paused"
    } else if fast {
        "fast"
    } else {
        "playing"
    }]
    .clone();
    for mut image in world
        .query_filtered::<&mut ImageNode, With<Playback>>()
        .iter_mut(world)
    {
        if image.image != handle {
            image.image = handle.clone();
        }
    }
    for mut text in world
        .query_filtered::<&mut Text, With<Count>>()
        .iter_mut(world)
    {
        let next = count.to_string();
        if text.0 != next {
            text.0 = next;
        }
    }
}
fn panel_bottom(size: Vec2) -> f32 {
    let width = (size.x - 32.0).clamp(0.0, 370.0);
    let columns = ((width - 18.0 + 8.0) / 60.0).floor().max(1.0) as usize;
    let rows = QUANTITIES.len().div_ceil(columns);
    76.0 + 18.0
        + 22.0
        + 4.0
        + rows as f32 * if size.y < 300.0 { 56.0 } else { 68.0 }
        + (rows - 1) as f32 * 6.0
}
fn adapt_control_strip_visibility(world: &mut World, size: Vec2, selected_visible: bool) {
    let mut status_needs_space = false;
    for (text, font, node, computed, mut visible) in world
        .query_filtered::<(
            &Text,
            &TextFont,
            &Node,
            Option<&ComputedNode>,
            &mut Visibility,
        ), With<V0PlayerControlStrip>>()
        .iter_mut(world)
    {
        let max_width = match node.max_width {
            Val::Percent(percent) => size.x * percent / 100.0,
            Val::Px(px) => px,
            _ => size.x,
        };
        let horizontal_padding = match node.padding.left {
            Val::Px(px) => px,
            _ => 0.0,
        } + match node.padding.right {
            Val::Px(px) => px,
            _ => 0.0,
        };
        let vertical_padding = match node.padding.top {
            Val::Px(px) => px,
            _ => 0.0,
        } + match node.padding.bottom {
            Val::Px(px) => px,
            _ => 0.0,
        };
        // Conservative before text shaping, and when help/error text changes:
        // one em per character and 1.3em per line. Actual measured height can
        // increase the budget. This changes visibility only, never shortcuts.
        let columns = ((max_width - horizontal_padding) / font.font_size)
            .floor()
            .max(1.0) as usize;
        let lines = text
            .0
            .split('\n')
            .map(|line| line.chars().count().max(1).div_ceil(columns))
            .sum::<usize>();
        let estimated = lines as f32 * font.font_size * 1.3 + vertical_padding;
        let height = estimated.max(computed.map_or(0.0, |node| node.size().y));
        let bottom = match node.bottom {
            Val::Px(px) => px,
            _ => 0.0,
        };
        let need = panel_bottom(size) + 8.0 + bottom + height;
        let collision = selected_visible && size.y < need;
        let default_hints = text.0 == V0_PLAYER_CONTROL_HINTS;
        status_needs_space |= collision && !default_hints;
        *visible = if collision && default_hints {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    if status_needs_space {
        for mut visible in world
            .query_filtered::<&mut Visibility, With<HudRoot>>()
            .iter_mut(world)
        {
            *visible = Visibility::Hidden;
        }
    }
}
pub(super) fn sync(world: &mut World) {
    let ux = world.resource::<Fvr05ProductionUxStateResource>();
    let debug = ux.debug_mode;
    let paused = ux.settings.paused;
    let fast = ux.settings.playback_mode() != crate::ProductionRunMode::OneX;
    let count = world
        .resource::<Fvr03ProductionVoxelSceneResource>()
        .creature_render_count;
    let selected = world
        .resource::<Fvr03ProductionVoxelSelectionResource>()
        .selected
        .filter(|r| r.kind == StableVoxelRefKind::Creature)
        .and_then(|r| r.stable_id);
    let readout = selected.map(|stable_id| selected_readout(world, stable_id));
    set_readout(world, readout.as_ref(), debug);
    set_status(world, count, paused, fast);
    let size = world
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .iter(world)
        .next()
        .map_or(Vec2::new(1280.0, 720.0), |window| {
            Vec2::new(window.width(), window.height())
        });
    apply_layout(world, size);
    adapt_control_strip_visibility(world, size, readout.is_some() && !debug);
}

#[cfg(test)]
#[path = "status_hud/data_tests.rs"]
mod data_tests;

#[cfg(test)]
#[path = "status_hud/layout_tests.rs"]
mod layout_tests;
