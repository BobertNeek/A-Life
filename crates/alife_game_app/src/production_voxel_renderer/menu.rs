//! A small modal player menu. Debug rails remain separate; world truth stays in runtime.
use super::*;
use bevy::ecs::message::MessageCursor;
use bevy::input::{keyboard::KeyboardInput, ButtonState};
use bevy::prelude::*;
use bevy::ui::{BorderColor, BorderRadius, UiRect};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Screen {
    #[default]
    Closed,
    Main,
    Speed,
    Settings,
    ConfirmLoad,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
    Open,
    Play,
    Save,
    Load,
    ConfirmLoad,
    CancelLoad,
    Speed,
    Settings,
    Quit,
    OneX,
    Max,
    Focus,
    Back,
}

const MAIN: [(Action, &str); 6] = [
    (Action::Play, "Play"),
    (Action::Save, "Save"),
    (Action::Load, "Load"),
    (Action::Speed, "Speed"),
    (Action::Settings, "Settings"),
    (Action::Quit, "Quit"),
];
const SPEED: [(Action, &str); 3] = [
    (Action::OneX, "1x"),
    (Action::Max, "Max"),
    (Action::Back, "Back"),
];
const SETTINGS: [(Action, &str); 2] = [(Action::Focus, "Autopause"), (Action::Back, "Back")];

const CONFIRM_LOAD: [(Action, &str); 2] = [
    (Action::CancelLoad, "Cancel"),
    (Action::ConfirmLoad, "Load"),
];
const LOAD_WARNING: &str = "Load replaces this world.\nUnsaved changes are lost.";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum LoadPhase {
    #[default]
    Idle,
    Confirming,
    Queued,
    Applying,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoadAvailability {
    Ready,
    Busy,
    Unavailable,
}

impl LoadAvailability {
    fn error(self) -> Option<&'static str> {
        match self {
            Self::Ready => None,
            Self::Busy => Some("Wait for checkpoint publication before loading"),
            Self::Unavailable => Some("GPU runtime unavailable; load was not queued"),
        }
    }
}

#[derive(Debug, Default)]
struct InputOutcome {
    opening: bool,
    resume: Option<bool>,
    load_error: Option<&'static str>,
}

#[derive(Debug, Default, Resource)]
pub(crate) struct MenuState {
    screen: Screen,
    resume_on_close: bool,
    external_pause: bool,
    captured: bool,
    conversation_capture: bool,
    release_gate: bool,
    initialized: bool,
    selected: usize,
    load_phase: LoadPhase,
    load_origin: Screen,
    pointer_confirm_armed: bool,
    last_cursor: Option<Vec2>,
    key_reader: MessageCursor<KeyboardInput>,
    pub(super) pending: Option<Action>,
    #[cfg(feature = "gpu-runtime")]
    quit: Option<QuitCheckpoint>,
}

#[cfg(feature = "gpu-runtime")]
#[derive(Debug)]
struct QuitCheckpoint {
    destination: PathBuf,
    tick: alife_core::Tick,
    started: std::time::Instant,
}

impl MenuState {
    pub(super) fn note_external_pause(&mut self) {
        self.resume_on_close = false;
        self.external_pause = true;
    }

    #[cfg(feature = "gpu-runtime")]
    pub(super) fn note_loaded_pause(&mut self, saved_paused: bool) {
        if self.screen != Screen::Closed && !self.external_pause {
            self.resume_on_close = !saved_paused;
        }
    }

    fn open(&mut self, was_paused: bool) {
        if self.screen != Screen::Closed {
            return;
        }
        self.screen = Screen::Main;
        self.resume_on_close = !was_paused;
        self.external_pause &= was_paused;
        self.selected = 0;
        self.captured = true;
    }
    fn close(&mut self, play: bool) -> bool {
        let resume = play || self.resume_on_close;
        self.screen = Screen::Closed;
        self.pending = None;
        self.load_phase = LoadPhase::Idle;
        self.pointer_confirm_armed = false;
        if play {
            self.external_pause = false;
        }
        self.release_gate = true;
        self.captured = true;
        resume
    }
    fn back(&mut self) {
        self.screen = Screen::Main;
        self.selected = 0;
    }
    fn rows(&self) -> &'static [(Action, &'static str)] {
        match self.screen {
            Screen::Main => &MAIN,
            Screen::Speed => &SPEED,
            Screen::Settings => &SETTINGS,
            Screen::ConfirmLoad => &CONFIRM_LOAD,
            Screen::Closed => &[],
        }
    }
    fn request_load(&mut self, paused: bool, availability: LoadAvailability) -> bool {
        if self.load_phase != LoadPhase::Idle || availability != LoadAvailability::Ready {
            return false;
        }
        self.load_origin = self.screen;
        if self.screen == Screen::Closed {
            self.open(paused);
        }
        self.screen = Screen::ConfirmLoad;
        self.load_phase = LoadPhase::Confirming;
        self.selected = 0;
        self.pending = None;
        self.release_gate = true;
        self.pointer_confirm_armed = false;
        self.captured = true;
        true
    }

    fn cancel_load(&mut self) -> Option<bool> {
        self.pending = None;
        self.load_phase = LoadPhase::Idle;
        self.pointer_confirm_armed = false;
        self.release_gate = true;
        self.selected = 0;
        if self.load_origin == Screen::Closed {
            Some(self.close(false))
        } else {
            self.screen = self.load_origin;
            None
        }
    }

    #[cfg(feature = "gpu-runtime")]
    fn queue_confirmed_load(&mut self, availability: LoadAvailability) -> bool {
        if self.screen != Screen::ConfirmLoad
            || self.load_phase != LoadPhase::Confirming
            || self.release_gate
        {
            return false;
        }
        if availability != LoadAvailability::Ready {
            self.selected = 0;
            self.pointer_confirm_armed = false;
            self.release_gate = true;
            return false;
        }
        self.load_phase = LoadPhase::Queued;
        self.pending = None;
        true
    }

    #[cfg(feature = "gpu-runtime")]
    pub(super) fn begin_load_application(&mut self, available: bool) -> bool {
        if self.screen != Screen::ConfirmLoad || self.load_phase != LoadPhase::Queued {
            return false;
        }
        if !available {
            self.finish_load();
            return false;
        }
        self.load_phase = LoadPhase::Applying;
        true
    }

    #[cfg(feature = "gpu-runtime")]
    pub(super) fn finish_load(&mut self) {
        self.load_phase = LoadPhase::Idle;
        self.pending = None;
        self.pointer_confirm_armed = false;
        self.release_gate = true;
        self.back();
    }

    fn busy(&self) -> bool {
        if matches!(self.load_phase, LoadPhase::Queued | LoadPhase::Applying) {
            return true;
        }
        #[cfg(feature = "gpu-runtime")]
        if self.quit.is_some() {
            return true;
        }
        false
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "One snapshot of the modal's input and readiness."
    )]
    fn input(
        &mut self,
        keyboard: &ButtonInput<KeyCode>,
        left_down: bool,
        left_pressed: bool,
        fresh_enter: bool,
        cursor: Option<Vec2>,
        confirm_hit: bool,
        clicks: &[(Screen, Action)],
        paused: bool,
        conversation_open: bool,
        availability: LoadAvailability,
    ) -> InputOutcome {
        let mut outcome = InputOutcome::default();
        let gated = self.release_gate;
        if gated && !left_down && keyboard.get_pressed().next().is_none() {
            self.release_gate = false;
        }
        self.captured = self.screen != Screen::Closed || self.release_gate || self.busy();
        let conversation_capture = conversation_open
            || (self.screen == Screen::Closed
                && (keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::KeyY)));
        outcome.opening = conversation_capture && !self.conversation_capture;
        self.conversation_capture = conversation_capture;
        if !self.initialized {
            self.initialized = true;
            outcome.opening |= self.screen != Screen::Closed;
        }
        let moved = self
            .last_cursor
            .zip(cursor)
            .is_some_and(|(old, new)| old != new);
        self.last_cursor = cursor;
        if self.load_phase == LoadPhase::Confirming {
            if !confirm_hit || availability != LoadAvailability::Ready {
                self.pointer_confirm_armed = false;
            } else if !gated && !left_down && moved {
                self.pointer_confirm_armed = true;
            }
        }
        if conversation_open {
            return outcome;
        }
        // Escape wins even if a confirm click arrives on the same frame.
        if keyboard.just_pressed(KeyCode::Escape)
            && self.screen == Screen::ConfirmLoad
            && self.load_phase != LoadPhase::Applying
        {
            outcome.resume = self.cancel_load();
            return outcome;
        }
        if self.busy() || gated {
            return outcome;
        }
        let mut action = clicks.iter().find_map(|&(screen, action)| {
            (screen == self.screen
                && (action != Action::ConfirmLoad
                    || (self.pointer_confirm_armed && left_pressed && confirm_hit)))
                .then_some(action)
        });
        if keyboard.just_pressed(KeyCode::Escape) {
            if self.screen == Screen::Closed {
                action = Some(Action::Open);
            } else if self.screen == Screen::Main {
                outcome.resume = Some(self.close(false));
                action = None;
            } else {
                action = Some(Action::Back);
            }
        } else if self.screen == Screen::Closed {
            if keyboard.just_pressed(KeyCode::KeyL) {
                action = Some(Action::Load);
            }
        } else {
            let count = self.rows().len();
            if keyboard.just_pressed(KeyCode::ArrowDown) {
                self.selected = (self.selected + 1) % count;
            }
            if keyboard.just_pressed(KeyCode::ArrowUp) {
                self.selected = (self.selected + count - 1) % count;
            }
            if keyboard.just_pressed(KeyCode::Enter)
                && (self.screen != Screen::ConfirmLoad || fresh_enter)
            {
                action = Some(self.rows()[self.selected].0);
            }
            if self.screen != Screen::ConfirmLoad {
                if keyboard.just_pressed(KeyCode::KeyS) {
                    action = Some(Action::Save);
                }
                if keyboard.just_pressed(KeyCode::KeyL) {
                    action = Some(Action::Load);
                }
            }
        }
        match action {
            Some(Action::Open) => {
                self.open(paused);
                outcome.opening = true;
            }
            Some(Action::Load) => {
                if self.request_load(paused, availability) {
                    outcome.opening = true;
                    self.conversation_capture = false;
                } else {
                    outcome.load_error = availability.error();
                }
            }
            Some(Action::ConfirmLoad) => {
                if availability == LoadAvailability::Ready {
                    self.pending = Some(Action::ConfirmLoad);
                } else {
                    self.selected = 0;
                    self.pointer_confirm_armed = false;
                    self.release_gate = true;
                    outcome.load_error = availability.error();
                }
            }
            Some(Action::CancelLoad) => outcome.resume = self.cancel_load(),
            Some(Action::Play) => outcome.resume = Some(self.close(true)),
            Some(Action::Speed) => {
                self.screen = Screen::Speed;
                self.selected = 0;
            }
            Some(Action::Settings) => {
                self.screen = Screen::Settings;
                self.selected = 0;
            }
            Some(Action::Back) => self.back(),
            Some(action) => self.pending = Some(action),
            None => {}
        }
        outcome
    }
}

pub(super) fn is_captured(state: &MenuState) -> bool {
    state.captured || state.conversation_capture
}

pub(crate) fn is_open(state: &MenuState) -> bool {
    state.screen != Screen::Closed || state.busy()
}

pub(crate) fn captures_input(state: Option<Res<MenuState>>) -> bool {
    state.is_some_and(|state| captures_input_state(&state))
}

pub(crate) fn captures_input_state(state: &MenuState) -> bool {
    state.captured || state.screen != Screen::Closed || state.busy()
}

pub(super) fn world_input_allowed(state: Option<Res<MenuState>>) -> bool {
    !state.is_some_and(|state| state.captured || is_open(&state) || state.conversation_capture)
}

#[derive(Component)]
struct MenuRoot(Screen);
#[derive(Component)]
struct MenuPanel(Screen);
#[derive(Component)]
struct StateLabel;
#[derive(Component)]
struct SelectionLamp(Action);
#[derive(Component)]
struct MenuButton {
    screen: Screen,
    action: Action,
    index: usize,
}
#[derive(Component)]
struct Status;
#[derive(Component)]
struct LoadWarning;

pub(super) fn install(app: &mut App, summary: &ProductionVoxelLaunchSummary) {
    let mut state = MenuState::default();
    // Recorded graphical performance keeps its existing uninterrupted launch path.
    if !summary.record_performance {
        state.open(summary.ui_settings.paused);
    }
    app.insert_resource(state);
    spawn_ui(app.world_mut());
    app.add_systems(
        Update,
        handle_input.in_set(ProductionVoxelPresentationSet::Input),
    )
    .add_systems(
        Update,
        (sync_ui, adapt_layout.after(sync_ui)).in_set(ProductionVoxelPresentationSet::RootReaders),
    );
    #[cfg(feature = "gpu-runtime")]
    app.add_systems(
        Update,
        poll_quit
            .in_set(ProductionVoxelPresentationSet::Input)
            .after(handle_fvr05_production_ux_input)
            .after(apply_production_runtime_load)
            .before(ProductionVoxelPresentationSet::LiveGpuTick),
    );
}

fn panel_node() -> Node {
    Node {
        width: Val::Px(256.0),
        padding: UiRect::all(Val::Px(16.0)),
        border: UiRect::all(Val::Px(4.0)),
        border_radius: BorderRadius::all(Val::Px(24.0)),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(10.0),
        ..default()
    }
}
fn button_node() -> Node {
    Node {
        width: Val::Px(216.0),
        height: Val::Px(46.0),
        flex_shrink: 0.0,
        border: UiRect::all(Val::Px(3.0)),
        border_radius: BorderRadius::all(Val::Px(15.0)),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    }
}
fn bevel(raised: bool) -> BorderColor {
    let light = Color::srgb(0.44, 0.49, 0.48);
    let dark = Color::srgb(0.055, 0.080, 0.092);
    BorderColor {
        top: if raised { light } else { dark },
        left: if raised { light } else { dark },
        right: if raised { dark } else { light },
        bottom: if raised { dark } else { light },
    }
}
fn spawn_ui(world: &mut World) {
    for (screen, rows) in [
        (Screen::Main, MAIN.as_slice()),
        (Screen::Speed, SPEED.as_slice()),
        (Screen::Settings, SETTINGS.as_slice()),
        (Screen::ConfirmLoad, CONFIRM_LOAD.as_slice()),
    ] {
        world
            .spawn((
                Name::new("Player menu modal"),
                MenuRoot(screen),
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.027, 0.041, 0.051, 0.94)),
                GlobalZIndex(100),
            ))
            .with_children(|root| {
                root.spawn((
                    MenuPanel(screen),
                    panel_node(),
                    BackgroundColor(Color::srgb(0.13, 0.18, 0.20)),
                    bevel(true),
                ))
                .with_children(|panel| {
                    if screen == Screen::ConfirmLoad {
                        panel.spawn((
                            Name::new(
                                "Load warning: current world replacement and unsaved changes",
                            ),
                            LoadWarning,
                            Text::new(LOAD_WARNING),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.96, 0.76, 0.48)),
                            Node {
                                height: Val::Px(48.0),
                                flex_shrink: 0.0,
                                ..default()
                            },
                        ));
                    }
                    for (index, (action, label)) in rows.iter().copied().enumerate() {
                        panel
                            .spawn((
                                Name::new(if action == Action::ConfirmLoad {
                                    "Load and discard unsaved changes".to_string()
                                } else {
                                    label.to_string()
                                }),
                                Button,
                                MenuButton {
                                    screen,
                                    action,
                                    index,
                                },
                                button_node(),
                                BackgroundColor(Color::srgb(0.047, 0.072, 0.083)),
                                bevel(false),
                            ))
                            .with_children(|button| {
                                if action == Action::Focus {
                                    button.spawn((
                                        StateLabel,
                                        Text::new("On"),
                                        TextFont {
                                            font_size: 14.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.55, 0.85, 0.78)),
                                        Node {
                                            position_type: PositionType::Absolute,
                                            right: Val::Px(12.0),
                                            bottom: Val::Px(4.0),
                                            ..default()
                                        },
                                    ));
                                }
                                if matches!(action, Action::OneX | Action::Max) {
                                    button.spawn((
                                        SelectionLamp(action),
                                        Node {
                                            position_type: PositionType::Absolute,
                                            right: Val::Px(12.0),
                                            width: Val::Px(8.0),
                                            height: Val::Px(8.0),
                                            border_radius: BorderRadius::all(Val::Percent(50.0)),
                                            ..default()
                                        },
                                        BackgroundColor(Color::srgb(0.55, 0.85, 0.78)),
                                    ));
                                }
                                button.spawn((
                                    Text::new(label),
                                    TextFont {
                                        font_size: if action == Action::Focus {
                                            18.0
                                        } else {
                                            20.0
                                        },
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.88, 0.86, 0.72)),
                                ));
                            });
                    }
                    panel.spawn((
                        Status,
                        Text::new(""),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.88, 0.86, 0.72)),
                        Node {
                            position_type: PositionType::Absolute,
                            bottom: Val::Px(-30.0),
                            align_self: AlignSelf::Center,
                            ..default()
                        },
                    ));
                });
            });
    }
    // The entry control uses three actual bars rather than a font-dependent glyph.
    world
        .spawn((
            Name::new("Open player menu"),
            Button,
            MenuButton {
                screen: Screen::Closed,
                action: Action::Open,
                index: 0,
            },
            MenuRoot(Screen::Closed),
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(16.0),
                top: Val::Px(16.0),
                width: Val::Px(48.0),
                height: Val::Px(48.0),
                border: UiRect::all(Val::Px(3.0)),
                border_radius: BorderRadius::all(Val::Px(15.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(5.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.13, 0.18, 0.20)),
            bevel(true),
            GlobalZIndex(90),
        ))
        .with_children(|button| {
            for _ in 0..3 {
                button.spawn((
                    Node {
                        width: Val::Px(24.0),
                        height: Val::Px(3.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.88, 0.86, 0.72)),
                ));
            }
        });
}

fn load_availability(world: &World) -> LoadAvailability {
    #[cfg(feature = "gpu-runtime")]
    if let Some(runtime) = world.get_non_send_resource::<ProductionGpuBrainRuntimeResource>() {
        return runtime_load_availability(Some(&runtime.runtime));
    }
    LoadAvailability::Unavailable
}

#[cfg(feature = "gpu-runtime")]
fn runtime_load_availability(runtime: Option<&crate::GpuLiveBrainRuntime>) -> LoadAvailability {
    match runtime {
        Some(runtime) if runtime.persistence_idle_for_shutdown() => LoadAvailability::Ready,
        Some(_) => LoadAvailability::Busy,
        None => LoadAvailability::Unavailable,
    }
}

#[cfg(feature = "gpu-runtime")]
pub(super) fn load_application_available(world: &World) -> bool {
    load_availability(world) == LoadAvailability::Ready
}

/// Runs before every world-input reader, including on the frame a modal closes.
pub(crate) fn handle_input(world: &mut World) {
    let availability = load_availability(world);
    handle_input_with_availability(world, availability);
}

fn handle_input_with_availability(world: &mut World, availability: LoadAvailability) {
    let keyboard = world.resource::<ButtonInput<KeyCode>>().clone();
    let mouse = world.resource::<ButtonInput<MouseButton>>();
    let left_down = mouse.pressed(MouseButton::Left);
    let left_pressed = mouse.just_pressed(MouseButton::Left);
    #[cfg(feature = "gpu-runtime")]
    let conversation_open = world
        .get_resource::<ProductionConversationLineageUiState>()
        .is_some_and(|state| state.blocks_world_shortcuts());
    #[cfg(not(feature = "gpu-runtime"))]
    let conversation_open = false;
    let clicks = world
        .query::<(&MenuButton, Ref<Interaction>)>()
        .iter(world)
        .filter(|(_, interaction)| {
            interaction.is_changed() && **interaction == Interaction::Pressed
        })
        .map(|(button, _)| (button.screen, button.action))
        .collect::<Vec<_>>();
    let paused = {
        #[cfg(feature = "gpu-runtime")]
        if let Some(schedule) = world.get_resource::<ProductionGpuBrainTickScheduleResource>() {
            schedule.is_paused()
        } else {
            world
                .resource::<Fvr05ProductionUxStateResource>()
                .settings
                .paused
        }
        #[cfg(not(feature = "gpu-runtime"))]
        {
            world
                .resource::<Fvr05ProductionUxStateResource>()
                .settings
                .paused
        }
    };
    let cursor = world
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .iter(world)
        .next()
        .and_then(Window::physical_cursor_position);
    let confirm_hit = cursor.is_some_and(|cursor| {
        world
            .query::<(&MenuButton, &ComputedNode, &bevy::ui::UiGlobalTransform)>()
            .iter(world)
            .any(|(button, node, transform)| {
                button.screen == Screen::ConfirmLoad
                    && button.action == Action::ConfirmLoad
                    && node.contains_point(*transform, cursor)
            })
    });
    let outcome = world.resource_scope(|world, mut state: Mut<MenuState>| {
        // Drain the whole batch so a second key cannot become later consent.
        let mut fresh_enter = false;
        if let Some(messages) = world.get_resource::<bevy::ecs::message::Messages<KeyboardInput>>()
        {
            for message in state.key_reader.read(messages) {
                fresh_enter |= message.key_code == KeyCode::Enter
                    && message.state == ButtonState::Pressed
                    && !message.repeat;
            }
        }
        state.input(
            &keyboard,
            left_down,
            left_pressed,
            fresh_enter,
            cursor,
            confirm_hit,
            &clicks,
            paused,
            conversation_open,
            availability,
        )
    });
    let InputOutcome {
        opening,
        resume,
        load_error,
    } = outcome;
    if let Some(error) = load_error {
        let mut ux = world.resource_mut::<Fvr05ProductionUxStateResource>();
        ux.last_error = Some(error.into());
        ux.last_action = "Load unavailable".into();
    }
    #[cfg(feature = "gpu-runtime")]
    if world.resource::<MenuState>().load_phase != LoadPhase::Queued {
        if let Some(mut request) = world.get_resource_mut::<ProductionRuntimeLoadRequest>() {
            request.take();
        }
    }
    if opening {
        world
            .resource_mut::<Fvr05ProductionUxStateResource>()
            .pending_food_move = None;
        god_hand::cancel_for_menu(world);
    }
    let modal = world.resource::<MenuState>().screen != Screen::Closed;
    #[cfg(feature = "gpu-runtime")]
    if let Some(mut schedule) = world.get_resource_mut::<ProductionGpuBrainTickScheduleResource>() {
        if modal {
            schedule.pause_for_menu();
        } else if resume == Some(true) && schedule.is_paused() {
            schedule.toggle_playback();
        }
    }
    if modal || resume.is_some() {
        world
            .resource_mut::<Fvr05ProductionUxStateResource>()
            .settings
            .paused = modal || resume == Some(false);
    }
    if world.resource::<MenuState>().captured || world.resource::<MenuState>().conversation_capture
    {
        if let Some(mut wheel) =
            world.get_resource_mut::<bevy::ecs::message::Messages<bevy::input::mouse::MouseWheel>>()
        {
            wheel.clear();
        }
    }
}

pub(super) fn save(
    ux: &mut Fvr05ProductionUxStateResource,
    #[cfg(feature = "gpu-runtime")] runtime: Option<&mut crate::GpuLiveBrainRuntime>,
) {
    // Never mask a settings-write failure with a later success, or present a source
    // snapshot as an authoritative save when the live GPU runtime is absent.
    ux.persist_ui_settings();
    if ux.last_error.is_some() {
        return;
    }
    #[cfg(feature = "gpu-runtime")]
    if let Some(runtime) = runtime {
        ux.write_gpu_runtime_save(false, runtime);
    } else {
        ux.last_error = Some("GPU runtime unavailable; save was not queued".into());
        ux.last_action = "Save failed".into();
    }
    #[cfg(not(feature = "gpu-runtime"))]
    ux.write_runtime_save(false);
}

pub(super) fn speed(
    mode: crate::ProductionRunMode,
    ux: &mut Fvr05ProductionUxStateResource,
    #[cfg(feature = "gpu-runtime")] schedule: Option<&mut ProductionGpuBrainTickScheduleResource>,
    record_performance: bool,
    modal: bool,
) {
    if mode == crate::ProductionRunMode::HeadlessMaxSpeed && record_performance {
        ux.last_error =
            Some("Finish graphical performance recording before entering headless mode".into());
        return;
    }
    #[cfg(feature = "gpu-runtime")]
    if let Some(schedule) = schedule {
        schedule.set_run_mode(mode);
        if mode == crate::ProductionRunMode::HeadlessMaxSpeed && !modal && schedule.is_paused() {
            schedule.toggle_playback();
        }
        ux.settings.paused = schedule.is_paused();
        ux.settings.run_mode = Some(mode);
        ux.last_error = None;
        ux.last_action = format!("Simulation: {}", mode.label());
    } else {
        ux.last_error = Some("Playback requires the GPU runtime".into());
    }
    #[cfg(not(feature = "gpu-runtime"))]
    {
        let _ = modal;
        ux.last_error = Some("Playback requires the GPU runtime".into());
    }
}

#[cfg(feature = "gpu-runtime")]
pub(super) fn queue_load(
    state: &mut MenuState,
    ux: &mut Fvr05ProductionUxStateResource,
    request: &mut ProductionRuntimeLoadRequest,
    runtime: Option<&crate::GpuLiveBrainRuntime>,
) {
    let availability = runtime_load_availability(runtime);
    if !state.queue_confirmed_load(availability) {
        if let Some(error) = availability.error() {
            ux.last_error = Some(error.into());
            ux.last_action = "Load unavailable".into();
        }
    } else if request.queue() {
        ux.last_error = None;
        ux.last_action = "Queued authoritative production runtime load".into();
    }
}

#[cfg(feature = "gpu-runtime")]
pub(super) fn begin_quit(
    state: &mut MenuState,
    ux: &mut Fvr05ProductionUxStateResource,
    runtime: Option<&mut crate::GpuLiveBrainRuntime>,
) {
    if state.quit.is_some() {
        return;
    }
    let Some(runtime) = runtime else {
        ux.last_error = Some("GPU runtime unavailable; quit save was not queued".into());
        return;
    };
    let tick = runtime.world().tick();
    let destination = PathBuf::from(&ux.settings.runtime_save_path);
    save(ux, Some(runtime));
    if ux.last_error.is_none() {
        state.quit = Some(QuitCheckpoint {
            destination,
            tick,
            started: std::time::Instant::now(),
        });
    }
}

#[cfg(feature = "gpu-runtime")]
fn poll_quit(world: &mut World) {
    let Some(quit) = world.resource::<MenuState>().quit.as_ref() else {
        return;
    };
    let destination = quit.destination.clone();
    let tick = quit.tick;
    let timed_out = quit.started.elapsed() >= std::time::Duration::from_secs(60);
    let result = (|| -> Result<bool, String> {
        let mut runtime = world
            .get_non_send_resource_mut::<ProductionGpuBrainRuntimeResource>()
            .ok_or("GPU runtime unavailable during quit")?;
        runtime
            .runtime
            .poll_persistence_for_shutdown()
            .map_err(|error| error.to_string())?;
        if runtime.runtime.persistence_failed_for_shutdown() {
            return Err(runtime.runtime.persistence_shutdown_diagnostics());
        }
        if let crate::GpuManualCheckpointStatus::Failed { message, .. } =
            runtime.runtime.manual_checkpoint_status()
        {
            return Err(message.clone());
        }
        if quit_checkpoint_complete(
            runtime.runtime.manual_checkpoint_status(),
            &destination,
            tick,
            runtime.runtime.persistence_idle_for_shutdown(),
            runtime.runtime.persistence_failed_for_shutdown(),
        ) {
            return Ok(true);
        }
        if timed_out {
            return Err(format!(
                "Quit checkpoint timed out: {}",
                runtime.runtime.persistence_shutdown_diagnostics()
            ));
        }
        Ok(false)
    })();
    match result {
        Ok(true) => {
            world.write_message(AppExit::Success);
        }
        Ok(false) => {}
        Err(message) => {
            world.resource_mut::<MenuState>().quit = None;
            let mut ux = world.resource_mut::<Fvr05ProductionUxStateResource>();
            ux.last_error = Some(message);
            ux.last_action = "Quit save failed; window remains open".into();
        }
    }
}

#[cfg(feature = "gpu-runtime")]
fn quit_checkpoint_complete(
    status: &crate::GpuManualCheckpointStatus,
    destination: &std::path::Path,
    tick: alife_core::Tick,
    idle: bool,
    failed: bool,
) -> bool {
    idle && !failed
        && matches!(status, crate::GpuManualCheckpointStatus::Complete { destination: saved, checkpoint_tick } if saved == destination && *checkpoint_tick >= tick)
}

fn button_color(interaction: Interaction, selected: bool, active: bool) -> Color {
    if interaction == Interaction::Pressed {
        Color::srgb(0.07, 0.20, 0.21)
    } else if interaction == Interaction::Hovered || selected {
        Color::srgb(0.12, 0.25, 0.27)
    } else if active {
        Color::srgb(0.08, 0.16, 0.18)
    } else {
        Color::srgb(0.047, 0.072, 0.083)
    }
}

fn sync_ui(
    state: Res<MenuState>,
    ux: Res<Fvr05ProductionUxStateResource>,
    mut roots: Query<(&MenuRoot, &mut Node)>,
    mut buttons: Query<(&MenuButton, &Interaction, &mut BackgroundColor)>,
    mut statuses: Query<&mut Text, (With<Status>, Without<StateLabel>)>,
    mut state_labels: Query<&mut Text, (With<StateLabel>, Without<Status>)>,
    mut lamps: Query<(&SelectionLamp, &mut Visibility)>,
) {
    for (root, mut node) in &mut roots {
        let display = if root.0 == state.screen {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
    for (button, interaction, mut background) in &mut buttons {
        let selected = button.screen == state.screen
            && button.index == state.selected
            && state.screen != Screen::Closed;
        let active = matches!(button.action, Action::OneX)
            && ux.settings.playback_mode() == crate::ProductionRunMode::OneX
            || matches!(button.action, Action::Max)
                && ux.settings.playback_mode() == crate::ProductionRunMode::MaxSpeed
            || matches!(button.action, Action::Focus) && ux.settings.pause_on_focus_loss;
        let color = button_color(*interaction, selected, active);
        if background.0 != color {
            background.0 = color;
        }
    }
    for mut text in &mut state_labels {
        let value = if ux.settings.pause_on_focus_loss {
            "On"
        } else {
            "Off"
        };
        if text.0 != value {
            text.0 = value.into();
        }
    }
    for (lamp, mut visibility) in &mut lamps {
        let active = (lamp.0 == Action::OneX
            && ux.settings.playback_mode() == crate::ProductionRunMode::OneX)
            || (lamp.0 == Action::Max
                && ux.settings.playback_mode() == crate::ProductionRunMode::MaxSpeed);
        let desired = if active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *visibility != desired {
            *visibility = desired;
        }
    }
    let status = if ux.last_error.is_some() {
        "Failed"
    } else if matches!(state.load_phase, LoadPhase::Queued | LoadPhase::Applying) {
        "Loading"
    } else if state.busy() || ux.last_action.starts_with("Queued exact") {
        "Saving"
    } else if ux.last_action.starts_with("Saved exact")
        || ux.last_action.starts_with("Saved production")
    {
        "Saved"
    } else if ux.last_action.starts_with("Queued authoritative") {
        "Loading"
    } else if ux.last_action.starts_with("Loaded authoritative") {
        "Loaded"
    } else {
        ""
    };
    for mut text in &mut statuses {
        if text.0 != status {
            text.0 = status.into();
        }
    }
}

fn apply_layout(world: &mut World, size: Vec2) {
    let compact = size.y < 420.0 && size.x >= 320.0;
    let bottom = if size.y >= 600.0 {
        100.0
    } else if size.y >= 420.0 {
        40.0
    } else {
        32.0
    };
    let mut nodes = world.query::<(
        &mut Node,
        Option<&MenuRoot>,
        Option<&MenuPanel>,
        Option<&MenuButton>,
    )>();
    for (mut node, root, panel, button) in nodes.iter_mut(world) {
        if root.is_some_and(|root| root.0 != Screen::Closed) {
            if node.padding.bottom != Val::Px(bottom) {
                node.padding.bottom = Val::Px(bottom);
            }
        } else if let Some(panel) = panel {
            let wide = compact && panel.0 == Screen::Main;
            let width = (if wide { 480.0 } else { 256.0_f32 }).min((size.x - 24.0).max(256.0));
            let direction = if wide {
                FlexDirection::Row
            } else {
                FlexDirection::Column
            };
            let wrap = if wide {
                FlexWrap::Wrap
            } else {
                FlexWrap::NoWrap
            };
            if node.width != Val::Px(width) {
                node.width = Val::Px(width);
            }
            if node.flex_direction != direction {
                node.flex_direction = direction;
            }
            if node.flex_wrap != wrap {
                node.flex_wrap = wrap;
            }
            if node.column_gap != Val::Px(10.0) {
                node.column_gap = Val::Px(10.0);
            }
        } else if let Some(button) = button {
            if button.screen == Screen::Closed {
                continue;
            }
            let wide = compact && button.screen == Screen::Main;
            let width = if wide {
                ((size.x - 24.0).clamp(256.0, 480.0) - 50.0) * 0.5
            } else {
                216.0
            };
            if node.width != Val::Px(width) {
                node.width = Val::Px(width);
            }
        }
    }
}

fn adapt_layout(world: &mut World) {
    let size = world
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .iter(world)
        .next()
        .map_or(Vec2::new(1280.0, 720.0), |window| {
            Vec2::new(window.width(), window.height())
        });
    apply_layout(world, size);
}

#[cfg(test)]
#[path = "menu_load_tests.rs"]
mod load_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn escape_restores_only_menu_owned_pause_and_back_stays_modal() {
        for paused in [false, true] {
            let mut state = MenuState::default();
            state.open(paused);
            state.open(!paused);
            state.screen = Screen::Speed;
            state.back();
            assert_eq!(state.screen, Screen::Main);
            assert!(state.captured);
            assert_eq!(state.close(false), !paused);
            assert!(state.release_gate);
            assert!(state.captured);
        }
    }
    #[test]
    fn play_is_explicit_resume_and_discards_pending_menu_action() {
        let mut state = MenuState::default();
        state.open(true);
        state.pending = Some(Action::Save);
        assert!(state.close(true));
        assert_eq!(state.pending, None);
        assert_eq!(state.screen, Screen::Closed);
    }
    #[test]
    fn menu_ui_has_real_buttons_and_single_word_main_labels() {
        let mut world = World::new();
        spawn_ui(&mut world);
        let mut query = world.query::<(&MenuButton, &Node, &Children)>();
        let rows = query
            .iter(&world)
            .filter(|(button, _, _)| button.screen == Screen::Main)
            .map(|(button, node, children)| {
                assert_eq!(node.height, Val::Px(46.0));
                let label = children
                    .iter()
                    .find_map(|entity| world.get::<Text>(entity))
                    .unwrap();
                (button.index, label.0.clone())
            })
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            rows.values().cloned().collect::<Vec<_>>(),
            MAIN.iter()
                .map(|(_, label)| label.to_string())
                .collect::<Vec<_>>()
        );
        assert!(rows
            .values()
            .all(|label| label.split_whitespace().count() == 1));
    }
    #[test]
    fn focus_loss_pause_is_not_owned_by_menu() {
        let mut state = MenuState::default();
        state.open(false);
        state.note_external_pause();
        assert!(!state.close(false));
    }
    #[test]
    fn input_gate_covers_modal_and_the_dismissal_frame() {
        let mut app = App::new();
        app.init_resource::<MenuState>();
        app.add_systems(
            Update,
            (|mut calls: ResMut<Calls>| {
                calls.0 += 1;
            })
            .run_if(world_input_allowed),
        );
        app.init_resource::<Calls>();
        app.update();
        assert_eq!(app.world().resource::<Calls>().0, 1);
        app.world_mut().resource_mut::<MenuState>().open(false);
        app.update();
        app.world_mut().resource_mut::<MenuState>().close(true);
        app.update();
        assert_eq!(app.world().resource::<Calls>().0, 1);
        app.world_mut().resource_mut::<MenuState>().captured = false;
        app.update();
        assert_eq!(app.world().resource::<Calls>().0, 2);
        app.world_mut()
            .resource_mut::<MenuState>()
            .conversation_capture = true;
        app.update();
        assert_eq!(app.world().resource::<Calls>().0, 2);
    }
    #[derive(Resource, Default)]
    struct Calls(usize);
    #[test]
    fn held_button_is_not_a_second_click_and_hidden_page_is_not_a_target() {
        let mut world = World::new();
        spawn_ui(&mut world);
        let button = world
            .query::<(Entity, &MenuButton)>()
            .iter(&world)
            .find(|(_, b)| b.screen == Screen::Main && b.action == Action::Save)
            .unwrap()
            .0;
        world.clear_trackers();
        world.entity_mut(button).insert(Interaction::Pressed);
        let clicked = |world: &mut World, screen| {
            world
                .query::<(&MenuButton, Ref<Interaction>)>()
                .iter(world)
                .filter(|(b, i)| {
                    b.screen == screen && i.is_changed() && **i == Interaction::Pressed
                })
                .count()
        };
        assert_eq!(clicked(&mut world, Screen::Main), 1);
        assert_eq!(clicked(&mut world, Screen::Speed), 0);
        world.clear_trackers();
        assert_eq!(clicked(&mut world, Screen::Main), 0);
    }
    #[cfg(feature = "gpu-runtime")]
    #[test]
    fn loaded_pause_replaces_previous_world_pause_ownership() {
        for saved in [false, true] {
            let mut state = MenuState::default();
            state.open(!saved);
            state.note_loaded_pause(saved);
            assert_eq!(state.close(false), !saved);
            let mut state = MenuState::default();
            state.open(false);
            state.note_external_pause();
            state.note_loaded_pause(false);
            assert!(!state.close(false));
        }
    }
    #[cfg(feature = "gpu-runtime")]
    #[test]
    fn quit_requires_exact_destination_current_tick_and_idle_without_failure() {
        let path = PathBuf::from("checkpoint.json");
        let tick = alife_core::Tick(7);
        let complete = crate::GpuManualCheckpointStatus::Complete {
            destination: path.clone(),
            checkpoint_tick: tick,
        };
        assert!(quit_checkpoint_complete(
            &complete, &path, tick, true, false
        ));
        assert!(!quit_checkpoint_complete(
            &complete, &path, tick, false, false
        ));
        assert!(!quit_checkpoint_complete(
            &complete, &path, tick, true, true
        ));
        assert!(!quit_checkpoint_complete(
            &complete,
            std::path::Path::new("other.json"),
            tick,
            true,
            false
        ));
        assert!(!quit_checkpoint_complete(
            &complete,
            &path,
            alife_core::Tick(8),
            true,
            false
        ));
        assert!(!quit_checkpoint_complete(
            &crate::GpuManualCheckpointStatus::Idle,
            &path,
            tick,
            true,
            false
        ));
        assert!(!quit_checkpoint_complete(
            &crate::GpuManualCheckpointStatus::Queued {
                destination: path.clone(),
                checkpoint_tick: tick,
            },
            &path,
            tick,
            true,
            false
        ));
        assert!(!quit_checkpoint_complete(
            &crate::GpuManualCheckpointStatus::Failed {
                destination: path.clone(),
                message: "disk".into()
            },
            &path,
            tick,
            true,
            false
        ));
    }
    #[test]
    fn bevy_cpu_layout_keeps_all_button_hit_bounds_inside_supported_viewports() {
        use bevy::app::{HierarchyPropagatePlugin, PostUpdate, PropagateSet};
        use bevy::camera::{ComputedCameraValues, RenderTargetInfo, Viewport};
        use bevy::ui::update::propagate_ui_target_cameras;
        use bevy::ui::{
            ui_layout_system, ui_surface::UiSurface, ComputedUiRenderTargetInfo,
            ComputedUiTargetCamera, UiGlobalTransform, UiScale,
        };
        let mut receipts = Vec::new();
        for size in [
            Vec2::new(1280.0, 720.0),
            Vec2::new(640.0, 360.0),
            Vec2::new(320.0, 480.0),
            Vec2::new(320.0, 240.0),
        ] {
            for screen in [
                Screen::Main,
                Screen::Speed,
                Screen::Settings,
                Screen::ConfirmLoad,
            ] {
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
                spawn_ui(app.world_mut());
                apply_layout(app.world_mut(), size);
                for (root, mut node) in app
                    .world_mut()
                    .query::<(&MenuRoot, &mut Node)>()
                    .iter_mut(app.world_mut())
                {
                    node.display = if root.0 == screen {
                        Display::Flex
                    } else {
                        Display::None
                    };
                }
                app.update();
                app.update();
                let world = app.world_mut();
                let rows=world.query::<(&MenuButton,&ComputedNode,&UiGlobalTransform)>().iter(world)
                    .filter(|(button,_,_)|button.screen==screen).map(|(button,node,transform)| {
                        let center=transform.affine().translation; let half=node.size()*0.5;
                        let min=center-half;let max=center+half;
                        assert!(node.size().x>=110.0 && node.size().y==46.0,"{:?} invalid target {:?}",screen,node.size());
                        assert!(min.x>=0.0 && min.y>=0.0 && max.x<=size.x && max.y<=size.y,"{:?} {:?} outside {:?}: {:?} {:?}",screen,button.action,size,min,max);
                        assert!(node.contains_point(*transform,center));
                        assert!(!node.contains_point(*transform,max+Vec2::splat(1.0)));
                        serde_json::json!({"action":format!("{:?}",button.action),"min":[min.x,min.y],"max":[max.x,max.y],"center_hit":true})
                    }).collect::<Vec<_>>();
                if screen == Screen::ConfirmLoad {
                    let (node, transform) = world
                        .query_filtered::<(&ComputedNode, &UiGlobalTransform), With<LoadWarning>>()
                        .single(world)
                        .unwrap();
                    let center = transform.affine().translation;
                    let half = node.size() * 0.5;
                    assert_eq!(node.size().y, 48.0);
                    assert!(
                        center.x - half.x >= 0.0
                            && center.y - half.y >= 0.0
                            && center.x + half.x <= size.x
                            && center.y + half.y <= size.y
                    );
                }
                assert_eq!(
                    rows.len(),
                    match screen {
                        Screen::Main => 6,
                        Screen::Speed => 3,
                        Screen::Settings | Screen::ConfirmLoad => 2,
                        Screen::Closed => 0,
                    }
                );
                receipts.push(serde_json::json!({"viewport":[size.x,size.y],"screen":format!("{:?}",screen),"buttons":rows}));
            }
        }
        for screens in receipts.as_chunks::<4>().0 {
            let load = |screen: usize, action: &str| {
                screens[screen]["buttons"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|row| row["action"] == action)
                    .unwrap()
            };
            let old = load(0, "Load");
            let confirm = load(3, "ConfirmLoad");
            let separated = (0..2).any(|axis| {
                old["max"][axis].as_f64().unwrap() <= confirm["min"][axis].as_f64().unwrap()
                    || confirm["max"][axis].as_f64().unwrap() <= old["min"][axis].as_f64().unwrap()
            });
            assert!(
                separated,
                "Load and confirm hit targets overlap: {:?}",
                screens[0]["viewport"]
            );
        }
        if let Ok(path) = std::env::var("ALIFE_MENU_CPU_HIT_BOUNDS") {
            fs::write(path, serde_json::to_vec_pretty(&receipts).unwrap()).unwrap();
        }
    }

    #[test]
    fn export_actual_menu_nodes_for_cpu_visual_fixture() {
        let mut world = World::new();
        spawn_ui(&mut world);
        fn px(v: Val) -> f32 {
            if let Val::Px(v) = v {
                v
            } else {
                panic!("fixture dimension must be px")
            }
        }
        fn color(c: Color) -> [f32; 4] {
            let c = c.to_srgba();
            [c.red, c.green, c.blue, c.alpha]
        }
        // Controlled fixture: first row focused, 1x selected, Autopause on.
        let mut buttons = world.query::<(&MenuButton, &mut BackgroundColor)>();
        for (button, mut color) in buttons.iter_mut(&mut world) {
            color.0 = button_color(
                Interaction::None,
                button.index == 0,
                matches!(button.action, Action::OneX | Action::Focus),
            );
        }
        let mut fixtures = Vec::new();
        for size in [
            Vec2::new(1280.0, 720.0),
            Vec2::new(640.0, 360.0),
            Vec2::new(320.0, 480.0),
            Vec2::new(320.0, 240.0),
        ] {
            apply_layout(&mut world, size);
            for status_value in ["", "Failed"] {
                for mut text in world
                    .query_filtered::<&mut Text, With<Status>>()
                    .iter_mut(&mut world)
                {
                    text.0 = status_value.into();
                }
                let panels = world.query::<(&MenuRoot, &Children)>().iter(&world)
            .filter(|(root, _)| root.0 != Screen::Closed).map(|(root, children)| {
                let panel = children[0]; let node = world.get::<Node>(panel).unwrap();
                let background = world.get::<BackgroundColor>(panel).unwrap(); let border = world.get::<BorderColor>(panel).unwrap();
                let rows = world.get::<Children>(panel).unwrap().iter().filter_map(|entity| {
                    let button = world.get::<MenuButton>(entity)?; let node = world.get::<Node>(entity)?;
                    let child = world.get::<Children>(entity)?.iter().find(|child| world.get::<Text>(*child).is_some() && world.get::<StateLabel>(*child).is_none())?;
                    Some(serde_json::json!({"index":button.index,"focused":button.index==0,"label":world.get::<Text>(child).unwrap().0,
                        "width":px(node.width),"height":px(node.height),"radius":px(node.border_radius.top_left),
                        "border":px(node.border.top),"background":color(world.get::<BackgroundColor>(entity).unwrap().0),
                        "top":color(world.get::<BorderColor>(entity).unwrap().top),"bottom":color(world.get::<BorderColor>(entity).unwrap().bottom),
                        "state":if button.action == Action::Focus {Some("On")} else {None},"active":button.action == Action::OneX,
                        "font_size":world.get::<TextFont>(child).unwrap().font_size,"text_color":color(world.get::<TextColor>(child).unwrap().0)}))
                }).collect::<Vec<_>>();
                serde_json::json!({"screen":format!("{:?}",root.0),"status":status_value,"status_font_size":16.0,"viewport":[size.x,size.y],"modal_background":color(world.get::<BackgroundColor>(world.get::<ChildOf>(panel).unwrap().parent()).unwrap().0),"root_bottom_padding":px(world.get::<Node>(world.get::<ChildOf>(panel).unwrap().parent()).unwrap().padding.bottom),"columns":if node.flex_direction == FlexDirection::Row {2} else {1},"width":px(node.width),"padding":px(node.padding.top),
                    "border":px(node.border.top),"radius":px(node.border_radius.top_left),"gap":px(node.row_gap),
                    "background":color(background.0),"top":color(border.top),"bottom":color(border.bottom),"rows":rows})
            }).collect::<Vec<_>>();
                assert_eq!(panels.len(), 4);
                fixtures.extend(panels);
            }
        }
        if let Ok(path) = std::env::var("ALIFE_MENU_CPU_FIXTURE") {
            fs::write(path, serde_json::to_vec_pretty(&fixtures).unwrap()).unwrap();
        }
    }
}
