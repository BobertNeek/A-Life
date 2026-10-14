//! Wordless player care controls. Evidence is bounded by the existing world APIs.
//! Queued care has no command-completion receipt and never becomes "successful"
//! because a tick passes or a measured hormone changes.
use super::*;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use bevy::ui::{BorderColor, BorderRadius, FocusPolicy, UiRect};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub(super) enum Action {
    Food,
    Touch,
    Toy,
    Praise,
    Speech,
}
pub(super) const ACTIONS: [Action; 5] = [
    Action::Food,
    Action::Touch,
    Action::Toy,
    Action::Praise,
    Action::Speech,
];
impl Action {
    fn index(self) -> usize {
        self as usize
    }
    fn icon(self) -> &'static str {
        match self {
            Self::Food => "food",
            Self::Touch => "touch",
            Self::Toy => "toy",
            Self::Praise => "praise",
            Self::Speech => "speech",
        }
    }
    pub(super) fn key(self) -> KeyCode {
        match self {
            Self::Food => KeyCode::KeyE,
            Self::Touch => KeyCode::KeyC,
            Self::Toy => KeyCode::KeyJ,
            Self::Praise => KeyCode::KeyK,
            Self::Speech => KeyCode::Enter,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Food => "Food placement",
            Self::Touch => "Gentle touch",
            Self::Toy => "Toy offer",
            Self::Praise => "Praise offer",
            Self::Speech => "Speech",
        }
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
    Rejected,
    Unavailable,
}
#[derive(Resource, Default)]
pub(crate) struct State {
    pending: Option<Action>,
    captured: bool,
    release_gate: bool,
    pressed: [bool; 5],
    pub(crate) speech_opened: bool,
    selected: Option<StableVoxelObjectRef>,
    evidence: [Evidence; 5],
}
impl State {
    pub(super) fn take_pending(&mut self) -> Option<Action> {
        self.pending.take()
    }
    pub(super) fn observe_selection(&mut self, selected: Option<StableVoxelObjectRef>) {
        if self.selected != selected {
            self.evidence = [Evidence::None; 5];
            self.selected = selected;
        }
    }
    pub(super) fn record(&mut self, action: Action, evidence: Evidence) {
        self.evidence[action.index()] = evidence;
    }
    pub(super) fn reset(&mut self) {
        self.pending = None;
        self.evidence = [Evidence::None; 5];
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
#[derive(Resource)]
struct Icons(BTreeMap<&'static str, Handle<Image>>);

const HEIGHT: f32 = 72.0;
const BOTTOM: f32 = 12.0;
const WIDTH: f32 = 296.0;
const EMBEDDED: [(&str, &[u8]); 9] = [
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
        "praise",
        include_bytes!("../../../../assets/ui/care_dock/praise.png"),
    ),
    (
        "speech",
        include_bytes!("../../../../assets/ui/care_dock/speech.png"),
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
pub(super) fn spawn(world: &mut World) {
    world.init_resource::<State>();
    world.init_resource::<Assets<Image>>();
    let mut icons = BTreeMap::new();
    for (name, bytes) in EMBEDDED {
        let image = Image::from_buffer(
            bytes,
            ImageType::Extension("png"),
            CompressedImageFormats::NONE,
            true,
            ImageSampler::linear(),
            RenderAssetUsages::default(),
        )
        .expect("embedded care dock PNG");
        icons.insert(name, world.resource_mut::<Assets<Image>>().add(image));
    }
    let root = world
        .spawn((
            Name::new("Player care dock"),
            DockRoot,
            Button,
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
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.075, 0.115, 0.135)),
            bevel(true),
            GlobalZIndex(60),
            FocusPolicy::Block,
        ))
        .id();
    for action in ACTIONS {
        let button = world
            .spawn((
                Name::new(format!("Player {}", action.icon())),
                Button,
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
                BackgroundColor(Color::srgb(0.050, 0.075, 0.090)),
                bevel(true),
                ChildOf(root),
            ))
            .id();
        world.spawn((
            Glyph(action),
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
    world.insert_resource(Icons(icons));
}

/// Capture dock pointer input before menu and all world readers. Speech becomes
/// a real conversation capture before the menu's existing hand cancellation.
pub(super) fn collect_input(world: &mut World) {
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
    let mut pressed = [false; 5];
    for (action, interaction) in world.query::<(&Action, &Interaction)>().iter(world) {
        pressed[action.index()] = *interaction == Interaction::Pressed;
    }
    let state = world.resource::<State>();
    let click = ACTIONS.into_iter().find(|a| {
        left_down && !state.release_gate && pressed[a.index()] && !state.pressed[a.index()]
    });
    let blocked = menu_blocked || conversation_blocked || debug;
    {
        let mut state = world.resource_mut::<State>();
        state.speech_opened = false;
        state.pressed = pressed;
        if !left_down {
            state.release_gate = false;
        }
        if !blocked && click.is_some() {
            state.release_gate = true;
        }
        state.captured = (!blocked && pointer) || state.release_gate;
        if !blocked {
            state.pending = click.filter(|a| *a != Action::Speech);
        } else {
            state.pending = None;
        }
    }
    if !blocked && (pointer || world.resource::<State>().release_gate) {
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

/// A structurally valid object reference with a tile is eligible too. Do not
/// silently narrow E's existing contract to terrain-only selection.
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
        .filter(|o| o.kind == WorldObjectKind::Agent)?;
    let organism = object.organism_id?;
    let record = world.organism_registry().get(organism)?;
    (record.lifecycle().is_alive() && record.world_entity_id() == object.id)
        .then_some((organism, object.position))
}

/// This testable frontend port wraps existing runtime methods. It does not
/// implement a second live simulation or a neural CPU fallback.
#[cfg(any(feature = "gpu-runtime", test))]
trait CarePort {
    fn world(&self) -> &alife_world::HeadlessWorld;
    fn food(&mut self, position: Vec3f) -> Result<WorldEntityId, GameAppShellError>;
    fn toy(&mut self, organism: OrganismId) -> Result<WorldEntityId, GameAppShellError>;
    fn care(
        &mut self,
        organism: OrganismId,
        source: Vec3f,
        praise: bool,
    ) -> Result<(), GameAppShellError>;
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
    fn care(
        &mut self,
        organism: OrganismId,
        source: Vec3f,
        praise: bool,
    ) -> Result<(), GameAppShellError> {
        self.provide_player_care(organism, source, praise)
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
    let (organism, source) = care_target(port.world(), selected).ok_or_else(|| {
        GameAppShellError::InvalidProductionFrontend {
            message: "select a living creature first".into(),
        }
    })?;
    if action == Action::Toy {
        return port.toy(organism).map(Evidence::Placed);
    }
    port.care(organism, source, action == Action::Praise)?;
    Ok(Evidence::Queued)
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

/// The one frontend dispatcher used by E/C/J/K and real pointer controls.
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
                ux.last_action = match action {
                    Action::Food => "Food placed".into(),
                    Action::Toy => {
                        "Plaything offered; physical ball placed, creature response unconfirmed"
                            .into()
                    }
                    _ => format!(
                        "{} offered; queued stimulus, effect unconfirmed",
                        action.label()
                    ),
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
    let mut top = size.y - BOTTOM - HEIGHT;
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
        let bottom = crate::production_conversation_lineage_ui::speech_composer_bottom(world);
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
    for (mut node, mut visible) in world
        .query_filtered::<(&mut Node, &mut Visibility), With<DockRoot>>()
        .iter_mut(world)
    {
        node.left = Val::Px(((size.x - WIDTH) / 2.0).max(0.0));
        *visible = if debug {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    #[cfg(feature = "gpu-runtime")]
    let availability = world
        .get_non_send_resource::<ProductionGpuBrainRuntimeResource>()
        .map(|r| {
            [
                food_tile(selected).is_some(),
                care_target(r.runtime.world(), selected).is_some(),
                care_target(r.runtime.world(), selected).is_some()
                    && !r
                        .runtime
                        .world()
                        .entity_id("player-plaything")
                        .and_then(|id| r.runtime.world().entity(id))
                        .is_some_and(|o| o.carried_by.is_some()),
                care_target(r.runtime.world(), selected).is_some(),
                world.contains_resource::<ProductionConversationLineageUiState>(),
            ]
        })
        .unwrap_or([false; 5]);
    #[cfg(not(feature = "gpu-runtime"))]
    let availability = [false; 5];
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
    apply_feedback(world, availability, evidence, error);
}
fn apply_feedback(
    world: &mut World,
    availability: [bool; 5],
    evidence: [Evidence; 5],
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
            Evidence::Placed(_) => Some("placement"),
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
mod tests;
