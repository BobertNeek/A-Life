//! Wordless player care controls. Feedback distinguishes queued care, physical
//! placement, and rejection; queued stimuli do not establish a creature response.
use super::*;
use accesskit::{Action as AccessibleAction, Role};
use bevy::a11y::{AccessibilityNode, ActionRequest};
use bevy::ecs::message::MessageCursor;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use bevy::ui::{BorderColor, BorderRadius, FocusPolicy, UiRect};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub(super) enum Action {
    Food,
    Pet,
    Toy,
    Praise,
    Scold,
    Slap,
    Speech,
}
pub(super) const ACTIONS: [Action; 7] = [
    Action::Food,
    Action::Pet,
    Action::Toy,
    Action::Praise,
    Action::Scold,
    Action::Slap,
    Action::Speech,
];
impl Action {
    fn index(self) -> usize {
        self as usize
    }
    pub(super) fn icon(self) -> &'static str {
        match self {
            Self::Food => "food",
            // The pet glyph asset keeps its legacy filename.
            Self::Pet => "touch",
            Self::Toy => "toy",
            Self::Praise => "praise",
            Self::Scold => "scold",
            Self::Slap => "slap",
            Self::Speech => "speech",
        }
    }
    pub(super) fn key(self) -> KeyCode {
        match self {
            Self::Food => KeyCode::KeyE,
            Self::Pet => KeyCode::KeyC,
            Self::Toy => KeyCode::KeyJ,
            Self::Praise => KeyCode::KeyK,
            // V is verbal (scold), B is blow (slap): bottom row, next to C.
            Self::Scold => KeyCode::KeyV,
            Self::Slap => KeyCode::KeyB,
            Self::Speech => KeyCode::Enter,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Food => "Food placement",
            Self::Pet => "Pet",
            Self::Toy => "Toy offer",
            Self::Praise => "Praise offer",
            Self::Scold => "Scold",
            Self::Slap => "Slap",
            Self::Speech => "Speech",
        }
    }
    fn accessible_label(self) -> &'static str {
        match self {
            Self::Food => "Place food. Select ground first. E.",
            Self::Pet => "Offer a pet. Select a living creature first. C.",
            Self::Toy => "Offer a toy to a selected creature, or roll a selected ball toward its chime table. J.",
            Self::Praise => "Offer praise. Select a living creature first. K.",
            Self::Scold => "Offer a scold. Select a living creature first. V.",
            Self::Slap => "Offer a slap. Select a living creature first. B.",
            Self::Speech => "Open speech composer. Enter.",
        }
    }
    pub(super) fn queued_status(self) -> String {
        format!(
            "{} offered; queued stimulus, effect unconfirmed",
            self.label()
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Evidence {
    #[default]
    None,
    /// Accepted queue only. Its eventual effect is unconfirmed.
    Queued,
    /// The world API returned a physical food/ball identity, not a biological effect.
    Placed(WorldEntityId),
    Rolled(WorldEntityId),
    Rejected,
    Unavailable,
}
#[derive(Default)]
struct FirstGesture {
    elapsed: f32,
    dismissed: bool,
    recalled: bool,
}
impl FirstGesture {
    fn visible(&mut self, eligible: bool, unknown: bool, delta: f32) -> bool {
        if !eligible || self.dismissed || !(unknown || self.recalled) {
            return false;
        }
        self.elapsed = (self.elapsed + delta.clamp(0., 1.)).min(16.);
        (1.0..16.).contains(&self.elapsed)
    }
    fn recall(&mut self) {
        *self = Self {
            elapsed: 1.,
            recalled: true,
            dismissed: false,
        };
    }
}
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(super) enum Accessory {
    Garden,
    Fieldglass,
}

#[cfg(feature = "gpu-runtime")]
#[derive(Clone, Copy)]
pub(super) struct HandFocus {
    pub guard: Option<StableVoxelObjectRef>,
    pub point: Vec2,
    pub gift: Option<WorldEntityId>,
}
#[cfg(feature = "gpu-runtime")]
impl HandFocus {
    pub(super) fn bed(t: VoxelTileCoord) -> Self {
        Self {
            guard: Some(super::orchard::selected_soil(t)),
            gift: None,
            point: Vec2::new(t.x as f32 + 0.5, t.z as f32 + 0.5),
        }
    }
}

#[derive(Resource, Default)]
pub(crate) struct State {
    pending: Option<Action>,
    garden_pending: bool,
    pub(super) fieldglass_pending: bool,
    pub(super) last_gift: Option<WorldEntityId>,
    pub(super) garden_armed: bool,
    pub(super) garden_pointer: bool,
    garden_last_cursor: Option<Vec2>,
    #[cfg(feature = "gpu-runtime")]
    pub(super) garden_cursor: Option<VoxelTileCoord>,
    pub(super) garden_ground_click: bool,
    #[cfg(feature = "gpu-runtime")]
    pub(super) hand_focus: Option<HandFocus>,
    #[cfg(feature = "gpu-runtime")]
    pub(super) framing: Option<(HandFocus, f32)>,
    #[cfg(feature = "gpu-runtime")]
    pub(super) garden_effect: Option<(VoxelTileCoord, alife_world::orchard::OrchardReceipt)>,
    invitation: FirstGesture,
    captured: bool,
    release_gate: bool,
    pressed: [bool; 7],
    pub(crate) speech_opened: bool,
    accessible_reader: MessageCursor<ActionRequest>,
    selected: Option<StableVoxelObjectRef>,
    evidence: [Evidence; 7],
}
impl State {
    pub(super) fn recall_invitation(&mut self) {
        self.invitation.recall();
    }
    #[cfg(feature = "gpu-runtime")]
    pub(super) fn take_garden(&mut self) -> bool {
        std::mem::take(&mut self.garden_pending)
    }
    pub(super) fn take_pending(&mut self) -> Option<Action> {
        self.pending.take()
    }
    pub(super) fn observe_selection(&mut self, selected: Option<StableVoxelObjectRef>) {
        if self.selected != selected {
            self.evidence = [Evidence::None; 7];
            self.selected = selected;
        }
    }
    pub(super) fn record(&mut self, action: Action, evidence: Evidence) {
        if let Evidence::Placed(id) | Evidence::Rolled(id) = evidence {
            self.last_gift = Some(id);
        }
        self.evidence[action.index()] = evidence;
        if matches!(
            evidence,
            Evidence::Queued | Evidence::Placed(_) | Evidence::Rolled(_)
        ) {
            self.invitation.dismissed = true;
        }
    }
    pub(super) fn reset(&mut self) {
        self.pending = None;
        self.garden_pending = false;
        self.fieldglass_pending = false;
        self.last_gift = None;
        self.garden_armed = false;
        self.garden_ground_click = false;
        self.garden_pointer = false;
        self.garden_last_cursor = None;
        #[cfg(feature = "gpu-runtime")]
        {
            self.hand_focus = None;
            self.framing = None;
            self.garden_cursor = None;
            self.garden_effect = None;
        }
        self.evidence = [Evidence::None; 7];
        self.selected = None;
    }
}
#[derive(Component)]
struct DockRoot;
#[derive(Component)]
struct Glyph(Action);
#[derive(Component)]
struct Marker(Action);
#[derive(Component)]
struct ErrorMarker;
#[derive(Component)]
struct Invitation;
#[derive(Resource)]
struct Icons(BTreeMap<&'static str, Handle<Image>>);

fn accessible(role: Role, label: &str) -> AccessibilityNode {
    let mut node = accesskit::Node::new(role);
    node.set_label(label);
    AccessibilityNode(node)
}
fn decoration_accessibility() -> AccessibilityNode {
    let mut node = accessible(Role::Image, "");
    node.set_hidden();
    node
}

const HEIGHT: f32 = 72.0;
const BOTTOM: f32 = 12.0;
const WIDTH: f32 = 408.0;
pub(crate) fn dock_size(viewport: Vec2) -> Vec2 {
    if viewport.x < WIDTH + 24.0 && viewport.y < 320.0 {
        Vec2::new(308.0, 56.0)
    } else if viewport.x < WIDTH + 24.0 {
        Vec2::new(240.0, 128.0)
    } else {
        Vec2::new(WIDTH, HEIGHT)
    }
}
const EMBEDDED: [(&str, &[u8]); 14] = [
    (
        "select",
        include_bytes!("../../../../assets/ui/status_hud/selection.png"),
    ),
    (
        "toward",
        include_bytes!("../../../../assets/ui/kinship/handpath.png"),
    ),
    (
        "food",
        include_bytes!("../../../../assets/ui/care_dock/food.png"),
    ),
    (
        "touch",
        include_bytes!("../../../../assets/ui/care_dock/touch.png"),
    ),
    (
        "toy",
        include_bytes!("../../../../assets/ui/care_dock/toy.png"),
    ),
    (
        "roll",
        include_bytes!("../../../../assets/ui/care_dock/roll.png"),
    ),
    (
        "praise",
        include_bytes!("../../../../assets/ui/care_dock/praise.png"),
    ),
    (
        "speech",
        include_bytes!("../../../../assets/ui/care_dock/speech.png"),
    ),
    (
        "scold",
        include_bytes!("../../../../assets/ui/care_dock/scold.png"),
    ),
    (
        "slap",
        include_bytes!("../../../../assets/ui/care_dock/slap.png"),
    ),
    (
        "pending",
        include_bytes!("../../../../assets/ui/care_dock/pending.png"),
    ),
    (
        "failed",
        include_bytes!("../../../../assets/ui/care_dock/failed.png"),
    ),
    (
        "unavailable",
        include_bytes!("../../../../assets/ui/care_dock/unavailable.png"),
    ),
    (
        "placement",
        include_bytes!("../../../../assets/ui/care_dock/placement.png"),
    ),
];
use crate::production_voxel_renderer::sanctuary_ink::{self as ink, bevel};
pub(super) fn spawn_accessory(
    world: &mut World,
    parent: Entity,
    accessory: Accessory,
    image: Handle<Image>,
    label: &'static str,
) -> (Entity, Entity) {
    let mut accessible = accesskit::Node::new(accesskit::Role::Button);
    accessible.set_label(label);
    accessible.add_action(accesskit::Action::Click);
    accessible.set_hidden();
    let root = world
        .spawn((
            accessory,
            Name::new(label),
            Node {
                position_type: PositionType::Absolute,
                left: if accessory == Accessory::Garden {
                    Val::Px(2.)
                } else {
                    Val::Auto
                },
                right: if accessory == Accessory::Fieldglass {
                    Val::Px(2.)
                } else {
                    Val::Auto
                },
                bottom: Val::Percent(115.),
                width: Val::Px(50.),
                height: Val::Px(42.),
                border: UiRect::all(Val::Px(2.)),
                border_radius: BorderRadius::all(Val::Px(15.)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            Visibility::Hidden,
            FocusPolicy::Block,
            Interaction::None,
            AccessibilityNode(accessible),
            BackgroundColor(ink::GLASS),
            ink::bevel(true),
            ChildOf(parent),
        ))
        .id();
    let mut decoration = accesskit::Node::new(accesskit::Role::Image);
    decoration.set_hidden();
    let icon = world
        .spawn((
            ImageNode::new(image),
            Node {
                width: Val::Px(32.),
                height: Val::Px(32.),
                ..default()
            },
            AccessibilityNode(decoration),
            FocusPolicy::Pass,
            bevy::picking::Pickable::IGNORE,
            ChildOf(root),
        ))
        .id();
    (root, icon)
}

pub(super) fn decode_png(world: &mut World, bytes: &[u8]) -> Handle<Image> {
    let image = Image::from_buffer(
        bytes,
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::linear(),
        RenderAssetUsages::default(),
    )
    .expect("embedded Hand glyph");
    world.resource_mut::<Assets<Image>>().add(image)
}

pub(super) fn spawn(world: &mut World) {
    world.init_resource::<State>();
    world.init_resource::<Assets<Image>>();
    let mut icons = BTreeMap::new();
    for (name, bytes) in EMBEDDED {
        icons.insert(name, decode_png(world, bytes));
    }
    let root = world
        .spawn((
            Name::new("Player care dock"),
            DockRoot,
            Interaction::None,
            accessible(Role::Toolbar, "Player care"),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(WIDTH),
                height: Val::Px(HEIGHT),
                bottom: Val::Px(BOTTOM),
                left: Val::Px(12.0),
                padding: UiRect::all(Val::Px(8.0)),
                border: UiRect::all(Val::Px(4.0)),
                border_radius: BorderRadius::all(Val::Px(22.0)),
                column_gap: Val::Px(8.0),
                row_gap: Val::Px(8.0),
                flex_wrap: FlexWrap::Wrap,
                align_content: AlignContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(ink::GLASS),
            bevel(true),
            GlobalZIndex(60),
            FocusPolicy::Block,
        ))
        .id();
    for action in ACTIONS {
        let mut node = accessible(Role::Button, action.accessible_label());
        node.add_action(AccessibleAction::Click);
        let button = world
            .spawn((
                Name::new(format!("Player {}", action.icon())),
                // Explicit Interaction preserves wordless labels: Bevy's
                // automatic Button label is derived from child Text only.
                Interaction::None,
                FocusPolicy::Block,
                node,
                action,
                Node {
                    width: Val::Px(48.0),
                    height: Val::Px(48.0),
                    flex_shrink: 0.0,
                    border: UiRect::all(Val::Px(3.0)),
                    border_radius: BorderRadius::all(Val::Px(15.0)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(ink::WELL),
                bevel(true),
                ChildOf(root),
            ))
            .id();
        if action == Action::Pet {
            let hint = world
                .spawn((
                    Invitation,
                    Name::new("First touch invitation"),
                    accessible(
                        Role::Image,
                        "Try selecting an individual, then touching it.",
                    ),
                    Visibility::Hidden,
                    FocusPolicy::Pass,
                    bevy::picking::Pickable::IGNORE,
                    Node {
                        position_type: PositionType::Absolute,
                        bottom: Val::Percent(112.),
                        left: Val::Px(-14.),
                        padding: UiRect::axes(Val::Px(7.), Val::Px(3.)),
                        column_gap: Val::Px(2.),
                        align_items: AlignItems::Center,
                        border_radius: BorderRadius::all(Val::Px(12.)),
                        ..default()
                    },
                    BackgroundColor(ink::GLASS),
                    ChildOf(button),
                ))
                .id();
            for name in ["select", "toward", "touch"] {
                world.spawn((
                    ImageNode::new(icons[name].clone()),
                    decoration_accessibility(),
                    FocusPolicy::Pass,
                    bevy::picking::Pickable::IGNORE,
                    Node {
                        width: Val::Px(16.),
                        height: Val::Px(16.),
                        ..default()
                    },
                    ChildOf(hint),
                ));
            }
        }
        world.spawn((
            Glyph(action),
            decoration_accessibility(),
            ImageNode::new(icons[action.icon()].clone()),
            Node {
                width: Val::Px(32.0),
                height: Val::Px(32.0),
                ..default()
            },
            bevy::picking::Pickable::IGNORE,
            FocusPolicy::Pass,
            ChildOf(button),
        ));
        world.spawn((
            Marker(action),
            decoration_accessibility(),
            ImageNode::new(icons["unavailable"].clone()),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(16.0),
                height: Val::Px(16.0),
                right: Val::Px(-11.0),
                bottom: Val::Px(-11.0),
                ..default()
            },
            bevy::picking::Pickable::IGNORE,
            FocusPolicy::Pass,
            Visibility::Hidden,
            ChildOf(button),
        ));
    }
    world.spawn((
        ErrorMarker,
        decoration_accessibility(),
        ImageNode::new(icons["failed"].clone()),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(16.0),
            height: Val::Px(16.0),
            right: Val::Px(0.0),
            top: Val::Px(-16.0),
            ..default()
        },
        Visibility::Hidden,
        bevy::picking::Pickable::IGNORE,
        FocusPolicy::Pass,
        ChildOf(root),
    ));
    super::orchard::spawn_tool(world, root);
    #[cfg(feature = "gpu-runtime")]
    super::field_signs::glass::spawn(world, root);
    super::live_care_feedback::prepare_art(world, &icons);
    world.insert_resource(Icons(icons));
}

/// Capture dock pointer input before menu and all world readers. Speech becomes
/// a real conversation capture before the menu's existing hand cancellation.
pub(crate) fn collect_input(world: &mut World) {
    let menu_blocked = world
        .get_resource::<menu::MenuState>()
        .is_some_and(|s| menu::is_open(s) || menu::is_captured(s));
    #[cfg(feature = "gpu-runtime")]
    let conversation_blocked = world
        .get_resource::<ProductionConversationLineageUiState>()
        .is_some_and(|s| s.blocks_world_shortcuts());
    #[cfg(not(feature = "gpu-runtime"))]
    let conversation_blocked = false;
    let debug = world
        .resource::<Fvr05ProductionUxStateResource>()
        .debug_mode;
    let left_down = world
        .resource::<ButtonInput<MouseButton>>()
        .pressed(MouseButton::Left);
    let pointer = world
        .query_filtered::<&Interaction, Or<(With<Action>, With<DockRoot>)>>()
        .iter(world)
        .any(|i| *i != Interaction::None);
    let mut pressed = [false; 7];
    for (action, interaction) in world.query::<(&Action, &Interaction)>().iter(world) {
        pressed[action.index()] = *interaction == Interaction::Pressed;
    }
    let visible_tools: Vec<_> = world
        .query::<(Entity, &Accessory, &Interaction, &Visibility)>()
        .iter(world)
        .filter(|(_, _, _, v)| **v != Visibility::Hidden)
        .map(|(e, a, i, _)| (e.to_bits(), *a, *i))
        .collect();
    let pointer = pointer
        || visible_tools
            .iter()
            .any(|(_, _, i)| *i != Interaction::None);
    let state = world.resource::<State>();
    let pointer_tool = (world
        .resource::<ButtonInput<MouseButton>>()
        .just_pressed(MouseButton::Left)
        && !state.release_gate)
        .then(|| {
            visible_tools
                .iter()
                .find_map(|(_, a, i)| (*i == Interaction::Pressed).then_some(*a))
        })
        .flatten();
    let click = ACTIONS.into_iter().find(|a| {
        left_down && !state.release_gate && pressed[a.index()] && !state.pressed[a.index()]
    });
    let cursor = world
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .iter(world)
        .next()
        .and_then(Window::cursor_position);
    let focused = world
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .iter(world)
        .next()
        .is_none_or(|window| window.focused);
    let blocked = menu_blocked || conversation_blocked || debug || !focused;
    // Drain even blocked/unknown requests, so a stale activation cannot become
    // care after returning from a menu, composer, debug view, or focus loss.
    let controls = world
        .query::<(Entity, &Action)>()
        .iter(world)
        .map(|(entity, action)| (entity.to_bits(), *action))
        .collect::<Vec<_>>();
    let (accessible_click, accessible_tool) =
        world.resource_scope(|world, mut state: Mut<State>| {
            let mut action = None;
            let mut tool = None;
            if let Some(messages) =
                world.get_resource::<bevy::ecs::message::Messages<ActionRequest>>()
            {
                for request in state.accessible_reader.read(messages) {
                    if !blocked && request.0.action == AccessibleAction::Click {
                        tool = visible_tools
                            .iter()
                            .find_map(|(e, a, _)| (*e == request.0.target.0).then_some(*a))
                            .or(tool);
                        action = controls
                            .iter()
                            .find_map(|(entity, action)| {
                                (*entity == request.0.target.0).then_some(*action)
                            })
                            .or(action);
                    }
                }
            }
            (action, tool)
        });
    let click = click.or(accessible_click);
    let ground_click = world
        .resource::<ButtonInput<MouseButton>>()
        .just_pressed(MouseButton::Left)
        && !pointer;
    let tool = pointer_tool
        .or(accessible_tool)
        .filter(|_| !blocked && click.is_none());
    {
        let mut state = world.resource_mut::<State>();
        state.speech_opened = false;
        state.garden_pending = tool == Some(Accessory::Garden);
        state.fieldglass_pending = tool == Some(Accessory::Fieldglass);
        if state.garden_armed
            && cursor
                .zip(state.garden_last_cursor)
                .is_some_and(|(a, b)| a != b)
        {
            state.garden_pointer = true;
        }
        state.garden_last_cursor = cursor;
        if blocked || click.is_some() || state.fieldglass_pending {
            state.garden_armed = false;
        }
        state.garden_ground_click = !blocked && state.garden_armed && ground_click;

        state.pressed = pressed;
        if !left_down {
            state.release_gate = false;
        }
        if !blocked && (click.is_some() || tool.is_some()) && left_down {
            state.release_gate = true;
        }
        state.captured = (!blocked && (pointer || accessible_click.is_some() || tool.is_some()))
            || state.release_gate;
        if !blocked {
            state.pending = click.filter(|a| *a != Action::Speech);
        } else {
            state.pending = None;
        }
    }
    if !blocked
        && (pointer || click.is_some() || tool.is_some() || world.resource::<State>().release_gate)
    {
        god_hand::cancel_for_menu(world);
    }
    if !blocked && click == Some(Action::Speech) {
        #[cfg(feature = "gpu-runtime")]
        if let Some(mut conversation) =
            world.get_resource_mut::<ProductionConversationLineageUiState>()
        {
            conversation.open_player_speech_input();
            let mut dock = world.resource_mut::<State>();
            dock.speech_opened = true;
            dock.record(Action::Speech, Evidence::None);
        } else {
            unavailable_speech(world);
        }
        #[cfg(not(feature = "gpu-runtime"))]
        unavailable_speech(world);
    }
    if world.resource::<State>().captured {
        if let Some(mut wheel) =
            world.get_resource_mut::<bevy::ecs::message::Messages<bevy::input::mouse::MouseWheel>>()
        {
            wheel.clear();
        }
    }
}
fn unavailable_speech(world: &mut World) {
    world
        .resource_mut::<State>()
        .record(Action::Speech, Evidence::Unavailable);
    let mut ux = world.resource_mut::<Fvr05ProductionUxStateResource>();
    ux.last_error = Some("GPU runtime unavailable".into());
    ux.last_action = "Speech unavailable".into();
}
pub(super) fn captures_input(state: Option<Res<State>>) -> bool {
    state.is_some_and(|s| s.captured)
}

pub(super) fn world_input_allowed(state: Option<Res<State>>) -> bool {
    !state.is_some_and(|s| s.captured)
}

/// Food can be placed from any stable selection with a ground tile.
pub(super) fn food_tile(selected: Option<StableVoxelObjectRef>) -> Option<VoxelTileCoord> {
    selected.and_then(|s| s.is_stable().then_some(s.tile).flatten())
}
#[cfg(any(feature = "gpu-runtime", test))]
fn care_target(
    world: &alife_world::HeadlessWorld,
    selected: Option<StableVoxelObjectRef>,
) -> Option<(OrganismId, Vec3f)> {
    let object = selected?
        .stable_id
        .and_then(|id| world.entity(id))
        .filter(|o| !o.consumed && o.kind == WorldObjectKind::Agent)?;
    let organism = object.organism_id?;
    let record = world.organism_registry().get(organism)?;
    (record.lifecycle().is_alive() && record.world_entity_id() == object.id)
        .then_some((organism, object.position))
}

/// A dock press operates on the selected creature, or the one being followed.
/// Keyboard food placement instead honors the ground under the Hand. Never
/// retarget a rejected correction to some other creature.
#[cfg(any(feature = "gpu-runtime", test))]
pub(super) fn contextual_target(
    action: Action,
    selected: Option<StableVoxelObjectRef>,
    pointed: Option<StableVoxelObjectRef>,
    followed: Option<WorldEntityId>,
    world: &alife_world::HeadlessWorld,
) -> Option<StableVoxelObjectRef> {
    let ground = |s: &StableVoxelObjectRef| {
        matches!(
            s.kind,
            StableVoxelRefKind::Tile | StableVoxelRefKind::Chunk | StableVoxelRefKind::Resource
        ) && food_tile(Some(*s)).is_some()
    };
    if action == Action::Food {
        if let Some(point) = pointed.filter(ground).or_else(|| selected.filter(ground)) {
            return tile_accepts_offering(world, point.tile?, &world.object_snapshots())
                .then_some(point);
        }
    }
    if action == Action::Toy {
        if let Some(ball) = selected.filter(|s| {
            s.stable_id.is_some_and(|id| {
                world
                    .entity(id)
                    .is_some_and(|o| o.kind == WorldObjectKind::Ball)
            })
        }) {
            // Even a held ball stays the target: never turn a rejected shove into a new offer.
            return Some(ball);
        }
    }
    let id = selected
        .filter(|s| s.kind == StableVoxelRefKind::Creature)
        .or_else(|| pointed.filter(|s| s.kind == StableVoxelRefKind::Creature))
        .and_then(|s| s.stable_id)
        .or(followed)?;
    let object = world.entity(id)?;
    let organism = object.organism_id?;
    let record = world.organism_registry().get(organism)?;
    if object.consumed
        || object.kind != WorldObjectKind::Agent
        || record.world_entity_id() != id
        || !record.lifecycle().is_alive()
    {
        return None;
    }
    let z = if world.terrain_binding().is_some() {
        object.position.z
    } else {
        object.position.y
    };
    let yaw = object.body_yaw + object.head_yaw;
    let tile_at = |x: f32, z: f32| VoxelTileCoord::new(x.floor() as i32, z.floor() as i32);
    let tile = if action == Action::Food {
        let objects = world.object_snapshots();
        [0.0, 0.8, -0.8, 1.6, -1.6]
            .into_iter()
            .map(|offset| {
                let angle = yaw + offset;
                tile_at(object.position.x + angle.cos() * 1.1, z + angle.sin() * 1.1)
            })
            .find(|&tile| tile_accepts_offering(world, tile, &objects))?
    } else {
        tile_at(object.position.x, z)
    };
    Some(StableVoxelObjectRef {
        kind: StableVoxelRefKind::Creature,
        stable_id: Some(id),
        chunk: VoxelChunkCoord::for_tile(16, tile),
        tile: Some(tile),
    })
}
/// The Hand cannot offer fruit under an obstacle or over impassable water.
#[cfg(any(feature = "gpu-runtime", test))]
fn tile_accepts_offering(
    world: &alife_world::HeadlessWorld,
    tile: VoxelTileCoord,
    objects: &[alife_world::WorldObject],
) -> bool {
    let (x, z) = (tile.x as f32 + 0.5, tile.z as f32 + 0.5);
    if world
        .terrain()
        .is_some_and(|terrain| !terrain.walkable(x, z))
    {
        return false;
    }
    objects
        .iter()
        .filter(|o| {
            !o.consumed && matches!(o.kind, WorldObjectKind::Obstacle | WorldObjectKind::Hazard)
        })
        .all(|o| {
            let depth = if world.terrain().is_some() {
                o.position.z
            } else {
                o.position.y
            };
            (o.position.x - x).hypot(depth - z) > o.radius + 0.20
        })
}

/// Runtime operations shared by keyboard, pointer, and CPU UI tests.
#[cfg(any(feature = "gpu-runtime", test))]
trait CarePort {
    fn world(&self) -> &alife_world::HeadlessWorld;
    fn food(&mut self, position: Vec3f) -> Result<WorldEntityId, GameAppShellError>;
    fn toy(&mut self, organism: OrganismId) -> Result<WorldEntityId, GameAppShellError>;
    fn roll(&mut self, id: WorldEntityId, offset: [f32; 2]) -> Result<(), GameAppShellError>;
    fn care(
        &mut self,
        organism: OrganismId,
        source: Vec3f,
        kind: alife_world::PlayerInteractionV1,
    ) -> Result<alife_world::PlayerInteractionQueueOutcomeV1, GameAppShellError>;
}
#[cfg(feature = "gpu-runtime")]
impl CarePort for crate::GpuLiveBrainRuntime {
    fn world(&self) -> &alife_world::HeadlessWorld {
        self.world()
    }
    fn food(&mut self, position: Vec3f) -> Result<WorldEntityId, GameAppShellError> {
        self.place_player_food(position).map(|r| r.world_entity_id)
    }
    fn toy(&mut self, organism: OrganismId) -> Result<WorldEntityId, GameAppShellError> {
        self.offer_player_play(organism)
    }
    fn roll(&mut self, id: WorldEntityId, offset: [f32; 2]) -> Result<(), GameAppShellError> {
        self.roll_player_toy(id, offset)
    }
    fn care(
        &mut self,
        organism: OrganismId,
        source: Vec3f,
        kind: alife_world::PlayerInteractionV1,
    ) -> Result<alife_world::PlayerInteractionQueueOutcomeV1, GameAppShellError> {
        self.provide_player_interaction_v1(organism, source, kind)
    }
}
#[cfg(any(feature = "gpu-runtime", test))]
fn dispatch_core(
    port: &mut impl CarePort,
    action: Action,
    selected: Option<StableVoxelObjectRef>,
) -> Result<Evidence, GameAppShellError> {
    if action == Action::Speech {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: "speech uses the conversation dispatcher".into(),
        });
    }
    if action == Action::Food {
        let tile =
            food_tile(selected).ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
                message: "select a visible terrain tile first".into(),
            })?;
        let position = if port.world().terrain_binding().is_some() {
            Vec3f::new(tile.x as f32 + 0.5, 0.0, tile.z as f32 + 0.5)
        } else {
            Vec3f::new(tile.x as f32 + 0.5, tile.z as f32 + 0.5, 0.0)
        };
        return port.food(position).map(Evidence::Placed);
    }
    if let Some(ball) = selected
        .and_then(|s| s.stable_id)
        .and_then(|id| port.world().entity(id))
        .filter(|o| action == Action::Toy && o.kind == WorldObjectKind::Ball)
    {
        let id = ball.id;
        let aim = toy_play::default_aim(port.world(), ball);
        return port.roll(id, aim.to_array()).map(|()| Evidence::Rolled(id));
    }
    let (organism, source) = care_target(port.world(), selected).ok_or_else(|| {
        GameAppShellError::InvalidProductionFrontend {
            message: "select a living creature first".into(),
        }
    })?;
    if action == Action::Toy {
        return port.toy(organism).map(Evidence::Placed);
    }
    let kind = match action {
        Action::Pet => alife_world::PlayerInteractionV1::Pet,
        Action::Praise => alife_world::PlayerInteractionV1::Praise,
        Action::Scold => alife_world::PlayerInteractionV1::Scold,
        Action::Slap => alife_world::PlayerInteractionV1::Slap,
        _ => unreachable!("food, toy and speech return above"),
    };
    match port.care(organism, source, kind)? {
        alife_world::PlayerInteractionQueueOutcomeV1::Queued => Ok(Evidence::Queued),
        // No dose received (a zero-hearing scold): the world accepted the
        // input and did nothing with it. Nothing was queued, so nothing
        // is marked and no delivery is claimed.
        alife_world::PlayerInteractionQueueOutcomeV1::NoReceivedDose => Ok(Evidence::None),
    }
}
pub(super) fn requested(
    action: Action,
    pointer: Option<Action>,
    keyboard: &ButtonInput<KeyCode>,
    debug: bool,
) -> bool {
    action != Action::Speech
        && (pointer == Some(action)
            || (keyboard.just_pressed(action.key()) && (action == Action::Food || !debug)))
}

/// The one frontend dispatcher used by E/C/J/K/V/B and real pointer controls.
pub(super) fn dispatch(
    action: Action,
    selected: Option<StableVoxelObjectRef>,
    ux: &mut Fvr05ProductionUxStateResource,
    #[cfg(feature = "gpu-runtime")] runtime: Option<&mut crate::GpuLiveBrainRuntime>,
    #[cfg(feature = "gpu-runtime")] frame: &mut LiveBrainPresentationFrameResource,
) -> Evidence {
    #[cfg(feature = "gpu-runtime")]
    {
        let Some(runtime) = runtime else {
            ux.last_error = Some("GPU runtime unavailable".into());
            ux.last_action = format!("{} unavailable", action.label());
            return Evidence::Unavailable;
        };
        let result = dispatch_core(runtime, action, selected);
        match result {
            Ok(evidence) => {
                frame.refresh_world_objects(runtime.world());
                ux.last_error = None;
                ux.last_action = match (action, evidence) {
                    (Action::Food, _) => "Food placed".into(),
                    (Action::Toy, Evidence::Rolled(_)) => {
                        "Ball nudged; no creature action inferred".into()
                    }
                    (Action::Toy, _) => {
                        "Plaything offered; physical ball placed, creature response unconfirmed"
                            .into()
                    }
                    (_, Evidence::Queued) => action.queued_status(),
                    // NoReceivedDose matches no strip arm, so the strip shows
                    // nothing; the sentence is visible verbatim only in the
                    // debug panel. It is not an error: the world accepted
                    // the input and delivered no dose.
                    (_, Evidence::None) => {
                        format!("{} offered; no dose received", action.label())
                    }
                    (_, _) => action.queued_status(),
                };
                evidence
            }
            Err(error) => {
                ux.last_error = Some(error.to_string());
                ux.last_action = format!(
                    "{} rejected; world authority declined the request",
                    action.label()
                );
                Evidence::Rejected
            }
        }
    }
    #[cfg(not(feature = "gpu-runtime"))]
    {
        let _ = selected;
        ux.last_error = Some("GPU runtime unavailable".into());
        ux.last_action = format!("{} unavailable", action.label());
        Evidence::Unavailable
    }
}

/// Bottom controls retain priority over readouts in short viewports. The
/// composer budget conservatively includes its preserved typed-text wrapping.
pub(super) fn occupied_top(world: &mut World, size: Vec2) -> f32 {
    let mut top = size.y - BOTTOM - dock_size(size).y;
    if world
        .get_resource::<Fvr05ProductionUxStateResource>()
        .is_some_and(|u| u.last_error.is_some())
    {
        top -= 16.0;
    }
    #[cfg(feature = "gpu-runtime")]
    if let Some(height) =
        crate::production_conversation_lineage_ui::speech_composer_height(world, size)
    {
        let bottom = crate::production_conversation_lineage_ui::speech_composer_bottom(world, size);
        top = top.min((size.y - bottom - height).max(0.0));
    }
    for (text, font, node, visibility, computed) in world
        .query_filtered::<(&Text, &TextFont, &Node, &Visibility, Option<&ComputedNode>), With<V0PlayerControlStrip>>()
        .iter(world)
    {
        if *visibility == Visibility::Hidden {
            continue;
        }
        let max_width = match node.max_width { Val::Percent(p) => size.x * p / 100.0, Val::Px(p) => p, _ => size.x - 32.0 };
        let padding = [node.padding.left, node.padding.right].iter().map(|v|if let Val::Px(p)=v {*p} else {0.0}).sum::<f32>();
        let width = (max_width - padding).max(font.font_size);
        let columns = (width / font.font_size).floor().max(1.0) as usize;
        let lines = text
            .0
            .split('\n')
            .map(|l| l.chars().count().max(1).div_ceil(columns))
            .sum::<usize>();
        let height = (lines as f32 * font.font_size * 1.3 + 20.0).max(computed.map_or(0.0, |n|n.size().y));
        let bottom = match node.bottom {
            Val::Px(p) => p,
            _ => 92.0,
        };
        top = top.min((size.y - bottom - height).max(0.0));
    }
    top
}
pub(super) fn sync(world: &mut World) {
    let selected = world
        .resource::<Fvr03ProductionVoxelSelectionResource>()
        .selected;
    world.resource_mut::<State>().observe_selection(selected);
    let ux = world.resource::<Fvr05ProductionUxStateResource>();
    let debug = ux.debug_mode;
    let error = ux.last_error.is_some();
    let size = world
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .iter(world)
        .next()
        .map_or(Vec2::new(1280.0, 720.0), |w| {
            Vec2::new(w.width(), w.height())
        });
    for (mut node, mut visible, mut accessible) in world
        .query_filtered::<(&mut Node, &mut Visibility, &mut AccessibilityNode), With<DockRoot>>()
        .iter_mut(world)
    {
        let dock = dock_size(size);
        let compact = dock.y == 56.0;
        node.padding = UiRect::all(Val::Px(if compact { 4.0 } else { 8.0 }));
        node.column_gap = Val::Px(if compact { 2.0 } else { 8.0 });
        node.width = Val::Px(dock.x);
        node.height = Val::Px(dock.y);
        node.left = Val::Px(((size.x - dock.x) / 2.0).max(0.0));
        if debug {
            accessible.set_hidden();
        } else {
            accessible.clear_hidden();
        }
        *visible = if debug {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    let button_size = if dock_size(size).y == 56.0 {
        40.0
    } else {
        48.0
    };
    for mut node in world
        .query_filtered::<&mut Node, With<Action>>()
        .iter_mut(world)
    {
        node.width = Val::Px(button_size);
        node.height = Val::Px(button_size);
    }
    for mut node in world
        .query_filtered::<&mut Node, With<Glyph>>()
        .iter_mut(world)
    {
        let glyph_size = if button_size == 40.0 { 24.0 } else { 32.0 };
        node.width = Val::Px(glyph_size);
        node.height = Val::Px(glyph_size);
    }
    for mut node in world
        .query_filtered::<&mut Node, With<Marker>>()
        .iter_mut(world)
    {
        let compact = button_size == 40.0;
        let marker_size = if compact { 8.0 } else { 16.0 };
        node.width = Val::Px(marker_size);
        node.height = Val::Px(marker_size);
        node.right = Val::Px(if compact { -3.0 } else { -11.0 });
        node.bottom = Val::Px(if compact { -3.0 } else { -11.0 });
    }
    #[cfg(feature = "gpu-runtime")]
    let selected_ball = world
        .get_non_send_resource::<ProductionGpuBrainRuntimeResource>()
        .and_then(|r| {
            let live = r.runtime.world();
            selected
                .and_then(|s| s.stable_id)
                .filter(|id| {
                    live.entity(*id)
                        .is_some_and(|o| o.kind == WorldObjectKind::Ball)
                })
                .map(|id| live.can_roll_toy(id))
        });
    #[cfg(not(feature = "gpu-runtime"))]
    let selected_ball = None::<bool>;
    #[cfg(feature = "gpu-runtime")]
    let availability = world
        .get_non_send_resource::<ProductionGpuBrainRuntimeResource>()
        .map(|r| {
            let live = r.runtime.world();
            let followed = world
                .get_resource::<Fvr04ProductionCreatureFollowResource>()
                .filter(|f| f.enabled)
                .and_then(|f| f.target_stable_id);
            let creature = contextual_target(Action::Pet, selected, None, followed, live).is_some();
            let free_toy = live
                .entity_id("player-plaything")
                .and_then(|id| live.entity(id))
                .is_none_or(|o| o.carried_by.is_none());
            [
                creature || food_tile(selected).is_some(),
                creature,
                selected_ball.unwrap_or(creature && free_toy),
                creature,
                creature,
                creature,
                world.contains_resource::<ProductionConversationLineageUiState>(),
            ]
        })
        .unwrap_or([false; 7]);
    #[cfg(not(feature = "gpu-runtime"))]
    let availability = [false; 7];
    let mut evidence = world.resource::<State>().evidence;
    #[cfg(feature = "gpu-runtime")]
    if let Some(conversation) = world.get_resource::<ProductionConversationLineageUiState>() {
        use crate::production_conversation_lineage_ui::SpeechInputCue;
        evidence[Action::Speech.index()] = match conversation.speech_input_cue() {
            SpeechInputCue::Pending => Evidence::Queued,
            SpeechInputCue::Failed => Evidence::Rejected,
            SpeechInputCue::Unavailable => Evidence::Unavailable,
            SpeechInputCue::None => evidence[Action::Speech.index()],
        };
    }
    let eligible = availability[Action::Pet.index()]
        && !debug
        && !error
        && !world
            .get_resource::<menu::MenuState>()
            .is_some_and(menu::captures_input_state);
    #[cfg(feature = "gpu-runtime")]
    let (eligible, unknown) = {
        let eligible = eligible
            && !world
                .get_resource::<ProductionConversationLineageUiState>()
                .is_some_and(|c| c.blocks_world_shortcuts());
        let row = world
            .get_resource::<LiveBrainPresentationFrameResource>()
            .and_then(|frame| {
                selected
                    .and_then(|s| s.stable_id)
                    .and_then(|id| frame.current.organism(id))
            });
        (
            eligible && row.is_some(),
            row.is_some_and(|r| {
                matches!(
                    r.last_player_care,
                    alife_world::PresentationEvidence::Unknown
                )
            }),
        )
    };
    #[cfg(not(feature = "gpu-runtime"))]
    let unknown = false;
    let delta = world.get_resource::<Time>().map_or(0., |t| t.delta_secs());
    let invite = world
        .resource_mut::<State>()
        .invitation
        .visible(eligible, unknown, delta);
    for (mut visible, mut node) in world
        .query_filtered::<(&mut Visibility, &mut AccessibilityNode), With<Invitation>>()
        .iter_mut(world)
    {
        *visible = if invite {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if invite {
            node.clear_hidden();
        } else {
            node.set_hidden();
        }
    }
    let toy_icon = world.resource::<Icons>().0[if selected_ball.is_some() {
        "roll"
    } else {
        "toy"
    }]
    .clone();
    for (glyph, mut image) in world.query::<(&Glyph, &mut ImageNode)>().iter_mut(world) {
        if glyph.0 == Action::Toy && image.image != toy_icon {
            image.image = toy_icon.clone();
        }
    }
    let label = if selected_ball.is_some() {
        "Roll this ball toward its chime table. J."
    } else {
        Action::Toy.accessible_label()
    };
    for (action, mut node) in world
        .query::<(&Action, &mut AccessibilityNode)>()
        .iter_mut(world)
    {
        if *action == Action::Toy && node.label() != Some(label) {
            node.set_label(label);
        }
    }
    apply_feedback(world, availability, evidence, error);
    super::orchard::sync_tool(world, selected, debug);
    #[cfg(feature = "gpu-runtime")]
    super::field_signs::glass::sync(world, debug);
}
fn apply_feedback(
    world: &mut World,
    availability: [bool; 7],
    evidence: [Evidence; 7],
    error: bool,
) {
    let handles = world.resource::<Icons>().0.clone();
    for (marker, mut image, mut visible) in world
        .query::<(&Marker, &mut ImageNode, &mut Visibility)>()
        .iter_mut(world)
    {
        let cue = match evidence[marker.0.index()] {
            Evidence::Rejected => Some("failed"),
            Evidence::Unavailable => Some("unavailable"),
            _ if !availability[marker.0.index()] => Some("unavailable"),
            Evidence::Queued => Some("pending"),
            Evidence::Placed(_) | Evidence::Rolled(_) => Some("placement"),
            Evidence::None => None,
        };
        if let Some(cue) = cue {
            image.image = handles[cue].clone();
        }
        *visible = if cue.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (glyph, mut image) in world.query::<(&Glyph, &mut ImageNode)>().iter_mut(world) {
        image.color = if availability[glyph.0.index()] {
            Color::WHITE
        } else {
            Color::srgba(0.62, 0.66, 0.66, 1.0)
        };
    }
    for (interaction, mut border) in world
        .query_filtered::<(&Interaction, &mut BorderColor), With<Action>>()
        .iter_mut(world)
    {
        *border = bevel(*interaction != Interaction::Pressed);
    }
    for mut visible in world
        .query_filtered::<&mut Visibility, With<ErrorMarker>>()
        .iter_mut(world)
    {
        *visible = if error {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

#[cfg(test)]
#[path = "care_dock/tests.rs"]
pub(crate) mod tests;
