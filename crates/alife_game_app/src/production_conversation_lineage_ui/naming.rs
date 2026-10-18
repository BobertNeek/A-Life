//! Selected-individual name entry reuses the captured, wordless text composer.
use super::*;
use accesskit::Action as AccessibleAction;
use bevy::{
    a11y::ActionRequest,
    ecs::message::MessageCursor,
    prelude::{Entity, Interaction, MouseButton, World},
    window::{Ime, PrimaryWindow},
};

#[derive(Debug, Clone)]
pub(super) struct Draft {
    stable_id: WorldEntityId,
    organism_id: OrganismId,
    original_name: String,
    runtime_identity: SpeechRuntimeIdentity,
    opening: Option<bool>, // true: F2; false: pointer/accessibility opening
    pub(super) preedit: String,
    ime_activity: bool,
    pub(super) error: Option<&'static str>,
}

#[derive(Default, Resource)]
pub(super) struct OpenInput {
    reader: MessageCursor<ActionRequest>,
    pressed: bool,
    ime_reader: MessageCursor<Ime>,
    ime_window: Option<(Entity, bool)>,
}

fn shown(world: &World, mut entity: Entity) -> bool {
    loop {
        if world.get::<Visibility>(entity) == Some(&Visibility::Hidden)
            || world
                .get::<Node>(entity)
                .is_some_and(|n| n.display == bevy::ui::Display::None)
        {
            return false;
        }
        match world.get::<ChildOf>(entity) {
            Some(parent) => entity = parent.parent(),
            None => return true,
        }
    }
}

/// Collect before the menu/world readers so the opening click/F2 is captured too.
fn collect_opening(world: &mut World) -> Option<bool> {
    use crate::production_voxel_renderer::{menu, status_hud::RenameButton};
    let buttons = world
        .query_filtered::<(Entity, &Interaction), With<RenameButton>>()
        .iter(world)
        .map(|(e, i)| (e, *i))
        .collect::<Vec<_>>();
    let down = world
        .resource::<ButtonInput<MouseButton>>()
        .pressed(MouseButton::Left);
    let previous = world.resource::<OpenInput>().pressed;
    let actions = world.resource_scope(|world, mut input: bevy::prelude::Mut<OpenInput>| {
        world
            .get_resource::<bevy::prelude::Messages<ActionRequest>>()
            .map(|m| {
                input
                    .reader
                    .read(m)
                    .map(|r| r.0.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    });
    world.resource_mut::<OpenInput>().pressed = down;
    let focused = world
        .query_filtered::<&bevy::prelude::Window, With<bevy::window::PrimaryWindow>>()
        .iter(world)
        .next()
        .is_none_or(|w| w.focused);
    let blocked = world
        .get_resource::<menu::MenuState>()
        .is_some_and(menu::captures_input_state)
        || world
            .resource::<Fvr05ProductionUxStateResource>()
            .debug_mode;
    let hover = !blocked
        && focused
        && buttons
            .iter()
            .any(|(e, i)| *i != Interaction::None && shown(world, *e));
    world
        .resource_mut::<ProductionConversationLineageUiState>()
        .name_pointer_capture = hover;
    if !focused
        && world
            .resource::<ProductionConversationLineageUiState>()
            .naming
            .is_some()
    {
        cancel(&mut world.resource_mut::<ProductionConversationLineageUiState>());
        return None;
    }
    let state = world.resource::<ProductionConversationLineageUiState>();
    if blocked || !focused || state.input_open || state.lineage_open || state.input_capture_held {
        return None;
    }
    let keyboard = world
        .resource::<ButtonInput<KeyCode>>()
        .just_pressed(KeyCode::F2);
    let pointer = (down
        && !previous
        && buttons
            .iter()
            .any(|(e, i)| *i == Interaction::Pressed && shown(world, *e)))
        || actions.iter().any(|r| {
            r.action == AccessibleAction::Click
                && buttons
                    .iter()
                    .any(|(e, _)| e.to_bits() == r.target.0 && shown(world, *e))
        });
    if !keyboard && !pointer {
        return None;
    }
    Some(keyboard)
}

pub(super) fn collect_input(world: &mut World) {
    if let Some(keyboard) = collect_opening(world) {
        open_selected(world, keyboard);
    }
    sync_ime_window(world);
    collect_ime(world);
}

fn open_selected(world: &mut World, keyboard: bool) {
    let selection = world.resource::<Fvr03ProductionVoxelSelectionResource>();
    let creatures = world.resource::<Fvr04ProductionCreatureSceneResource>();
    let Some(runtime) = world.get_non_send_resource::<ProductionGpuBrainRuntimeResource>() else {
        return;
    };
    let Some(stable_id) = selection
        .selected
        .filter(|s| s.kind == StableVoxelRefKind::Creature)
        .and_then(|s| s.stable_id)
    else {
        return;
    };
    let Some(organism_id) = selected_organism(selection, creatures) else {
        return;
    };
    let Some(object) = runtime.runtime.world().entity(stable_id).filter(|o| {
        o.kind == WorldObjectKind::Agent && o.organism_id == Some(organism_id) && !o.consumed
    }) else {
        return;
    };
    if !runtime
        .runtime
        .world()
        .organism_registry()
        .get(organism_id)
        .is_some_and(|r| r.world_entity_id() == stable_id && r.lifecycle().is_alive())
    {
        return;
    }
    let Ok(runtime_identity) = SpeechRuntimeIdentity::capture(&runtime.runtime) else {
        return;
    };
    let draft = Draft {
        stable_id,
        organism_id,
        original_name: object.label.clone(),
        runtime_identity,
        opening: Some(keyboard),
        preedit: String::new(),
        ime_activity: false,
        error: None,
    };
    open(
        &mut world.resource_mut::<ProductionConversationLineageUiState>(),
        draft,
    );
}

/// Own IME only while the naming entry has focus, preserving the host setting.
/// Called again during presentation so confirmation/cancellation restores it
/// in the same frame, after the keyboard handler closes the composer.
pub(super) fn sync_ime_window(world: &mut World) {
    let state = world.resource::<ProductionConversationLineageUiState>();
    let active = state.naming.is_some() && state.input_open && !state.input_help_open;
    let blocked = world
        .get_resource::<crate::production_voxel_renderer::menu::MenuState>()
        .is_some_and(crate::production_voxel_renderer::menu::captures_input_state);
    let target = if active && !blocked {
        world
            .query_filtered::<(Entity, &bevy::prelude::Window), With<PrimaryWindow>>()
            .iter(world)
            .find_map(|(entity, window)| window.focused.then_some(entity))
    } else {
        None
    };
    let previous = world.resource::<OpenInput>().ime_window;
    if previous.map(|(entity, _)| entity) != target {
        if let Some((entity, enabled)) = previous {
            if let Some(mut window) = world.get_mut::<bevy::prelude::Window>(entity) {
                window.ime_enabled = enabled;
            }
        }
        let next = target.map(|entity| {
            (
                entity,
                world
                    .get::<bevy::prelude::Window>(entity)
                    .unwrap()
                    .ime_enabled,
            )
        });
        world.resource_mut::<OpenInput>().ime_window = next;
    }
    if let Some(entity) = target {
        world
            .get_mut::<bevy::prelude::Window>(entity)
            .unwrap()
            .ime_enabled = true;
    } else if let Some(draft) = world
        .resource_mut::<ProductionConversationLineageUiState>()
        .naming
        .as_mut()
    {
        draft.preedit.clear();
    }
}

fn collect_ime(world: &mut World) {
    // Always advance this cursor, including while closed or behind another
    // modal, so a later name entry cannot replay an old OS commit.
    let messages = world.resource_scope(|world, mut input: bevy::prelude::Mut<OpenInput>| {
        world
            .get_resource::<bevy::prelude::Messages<Ime>>()
            .map(|messages| input.ime_reader.read(messages).cloned().collect::<Vec<_>>())
            .unwrap_or_default()
    });
    let target = world
        .resource::<OpenInput>()
        .ime_window
        .map(|(entity, _)| entity);
    let mut state = world.resource_mut::<ProductionConversationLineageUiState>();
    let Some(draft) = state.naming.as_mut() else {
        return;
    };
    draft.ime_activity = !draft.preedit.is_empty();
    if draft.opening.is_some() || target.is_none() {
        return;
    }
    for message in messages {
        let window = match &message {
            Ime::Preedit { window, .. }
            | Ime::Commit { window, .. }
            | Ime::Enabled { window }
            | Ime::Disabled { window } => *window,
        };
        if Some(window) != target {
            continue;
        }
        match message {
            Ime::Preedit { value, .. } => {
                let draft = state.naming.as_mut().unwrap();
                draft.ime_activity = true;
                draft.preedit = value
                    .chars()
                    .filter(|c| !c.is_control())
                    .take(MAX_TYPED_CHARS)
                    .collect();
            }
            Ime::Commit { value, .. } => {
                let draft = state.naming.as_mut().unwrap();
                draft.ime_activity = true;
                draft.preedit.clear();
                draft.error = None;
                let remaining = MAX_TYPED_CHARS.saturating_sub(state.input.chars().count());
                state
                    .input
                    .extend(value.chars().filter(|c| !c.is_control()).take(remaining));
                state.input_help_open = false;
            }
            Ime::Disabled { .. } => {
                state.naming.as_mut().unwrap().preedit.clear();
            }
            Ime::Enabled { .. } => {}
        }
    }
}

fn open(state: &mut ProductionConversationLineageUiState, draft: Draft) {
    state.input.clear();
    state.input_open = true;
    state.input_help_open = false;
    state.input_cue = SpeechInputCue::None;
    state.input_error = None;
    state.naming = Some(draft);
    state.status = "Naming the selected individual".into();
}

fn cancel(state: &mut ProductionConversationLineageUiState) {
    state.naming = None;
    state.input_open = false;
    state.input.clear();
    state.input_help_open = false;
    state.input_cue = SpeechInputCue::None;
    state.input_error = None;
    state.status = "Name entry cancelled".into();
}

fn target_matches(
    draft: &Draft,
    selection: &Fvr03ProductionVoxelSelectionResource,
    selected_organism: Option<OrganismId>,
    world: &HeadlessWorld,
) -> bool {
    selection.selected.is_some_and(|s| {
        s.kind == StableVoxelRefKind::Creature && s.stable_id == Some(draft.stable_id)
    }) && selected_organism == Some(draft.organism_id)
        && world.entity(draft.stable_id).is_some_and(|o| {
            o.kind == WorldObjectKind::Agent
                && !o.consumed
                && o.organism_id == Some(draft.organism_id)
                && o.label == draft.original_name
        })
        && world
            .organism_registry()
            .get(draft.organism_id)
            .is_some_and(|r| r.world_entity_id() == draft.stable_id && r.lifecycle().is_alive())
}

#[allow(
    clippy::too_many_arguments,
    reason = "Captured composer input and authoritative selected runtime are checked together."
)]
pub(super) fn handle_input<'a>(
    keyboard: &ButtonInput<KeyCode>,
    messages: impl Iterator<Item = &'a KeyboardInput>,
    pointer: Option<composer::Control>,
    selection: &Fvr03ProductionVoxelSelectionResource,
    creatures: &Fvr04ProductionCreatureSceneResource,
    runtime: &mut crate::GpuLiveBrainRuntime,
    state: &mut ProductionConversationLineageUiState,
) {
    let draft = state.naming.as_ref().unwrap();
    if !draft.runtime_identity.still_matches(runtime)
        || !target_matches(
            draft,
            selection,
            selected_organism(selection, creatures),
            runtime.world(),
        )
    {
        cancel(state);
        return;
    }
    let submit = edit(state, keyboard, messages, pointer);
    if submit {
        let id = state.naming.as_ref().unwrap().stable_id;
        let result = runtime.rename_player_creature(id, &state.input);
        complete(state, result);
    }
}

fn edit<'a>(
    state: &mut ProductionConversationLineageUiState,
    keyboard: &ButtonInput<KeyCode>,
    messages: impl Iterator<Item = &'a KeyboardInput>,
    pointer: Option<composer::Control>,
) -> bool {
    if keyboard.just_pressed(KeyCode::Escape) || pointer == Some(composer::Control::Cancel) {
        cancel(state);
        return false;
    }
    if keyboard.just_pressed(KeyCode::F1) || pointer == Some(composer::Control::Help) {
        state.input_help_open = !state.input_help_open;
    }
    let opening = state.naming.as_mut().unwrap().opening.take();
    let ime_activity = state.naming.as_ref().unwrap().ime_activity;
    let before = state.input.clone();
    let submit = if opening == Some(false) {
        false
    } else {
        // F2 consumes earlier shortcuts, just as Enter does for speech. A name
        // never confirms in the frame it opens, even with two raw Enter events.
        let mut after_open = opening.is_none();
        let mut submit = false;
        for message in messages {
            if message.state != ButtonState::Pressed {
                continue;
            }
            if !after_open {
                after_open = message.key_code == KeyCode::F2 && !message.repeat;
                continue;
            }
            // OS composition owns these keys. Its confirming Enter may arrive
            // in the same frame as Commit, so that frame never saves a name.
            if ime_activity {
                continue;
            }
            if message.key_code == KeyCode::Enter {
                submit |= opening.is_none() && !message.repeat;
                if submit {
                    break;
                }
            } else if message.key_code == KeyCode::Backspace {
                state.input.pop();
            } else if let Some(text) = &message.text {
                let remaining = MAX_TYPED_CHARS.saturating_sub(state.input.chars().count());
                state
                    .input
                    .extend(text.chars().filter(|c| !c.is_control()).take(remaining));
            }
        }
        submit
    };
    if before != state.input {
        state.naming.as_mut().unwrap().error = None;
        state.input_help_open = false;
    }
    opening.is_none() && !ime_activity && (submit || pointer == Some(composer::Control::Send))
}

fn complete(
    state: &mut ProductionConversationLineageUiState,
    result: Result<(), alife_world::RenameAgentError>,
) {
    match result {
        Ok(()) => {
            cancel(state);
            state.status =
                "Individual name saved in the world; save the game to keep it across sessions"
                    .into();
        }
        Err(error) => {
            use alife_world::RenameAgentError::*;
            let message = match error {
                EmptyName => "Choose a name.",
                NameInUse => "That name is already in use.",
                ReservedName => "That name is reserved. Choose another name.",
                TooLong { .. } => "Use at most 32 characters.",
                ControlCharacter => "Use a single line without control characters.",
                InvalidTarget => {
                    cancel(state);
                    return;
                }
            };
            state.naming.as_mut().unwrap().error = Some(message);
            state.status = message.into();
        }
    }
}

#[cfg(test)]
mod tests;
