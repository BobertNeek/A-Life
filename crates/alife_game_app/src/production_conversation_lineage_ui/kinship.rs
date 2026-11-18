//! Read-only, wordless family album. Genome links are evidence, not a license
//! to invent the individual behind an ambiguous or absent genome.
use super::*;
use crate::production_voxel_renderer::{
    menu, sanctuary_ink as ink, ProductionVoxelPresentationSet,
};
use accesskit::{Action as A11yAction, Role};
use bevy::{
    a11y::{AccessibilityNode, ActionRequest},
    ecs::message::MessageCursor,
    input_focus::InputFocus,
    prelude::*,
    ui::{FocusPolicy, RelativeCursorPosition},
    window::PrimaryWindow,
};

mod art;
mod pedigree;
mod view;
pub(crate) use art::open_button;
use art::{a11y, button, image, signs, Signs};
use pedigree::{Album, Member, Relative, PAGE};
use view::draw;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum Control {
    Open,
    Close,
    Back,
    Visit(u64),
    Earlier,
    Later,
}
#[derive(Resource, Default)]
struct State {
    album: Option<Album>,
    root: Option<Entity>,
    dirty: bool,
    compact: bool,
    reader: MessageCursor<ActionRequest>,
    held: bool,
    controls: Vec<Entity>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_systems(
            Update,
            input
                .before(crate::production_voxel_renderer::care_dock::collect_input)
                .in_set(ProductionVoxelPresentationSet::Input),
        )
        .add_systems(
            Update,
            (draw, focus_appearance)
                .chain()
                .in_set(ProductionVoxelPresentationSet::RootReaders),
        );
}
fn input(world: &mut World) {
    let was_open = world.resource::<State>().album.is_some();
    let keys = world.resource::<ButtonInput<KeyCode>>();
    let tab = keys.just_pressed(KeyCode::Tab);
    let reverse = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let activate = keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space);
    let up = keys.just_pressed(KeyCode::ArrowUp);
    let down = keys.just_pressed(KeyCode::ArrowDown);
    let mut key = if keys.just_pressed(KeyCode::Escape) {
        Some(Control::Close)
    } else if keys.just_pressed(KeyCode::KeyY) {
        Some(Control::Open)
    } else if keys.just_pressed(KeyCode::Backspace) {
        Some(Control::Back)
    } else if keys.just_pressed(KeyCode::ArrowLeft) {
        Some(Control::Earlier)
    } else if keys.just_pressed(KeyCode::ArrowRight) {
        Some(Control::Later)
    } else {
        None
    };
    let any_key = keys.get_pressed().next().is_some();
    if was_open {
        let state = world.resource::<State>();
        let focused = world.get_resource::<InputFocus>().and_then(|f| f.0);
        if tab && !state.controls.is_empty() {
            let count = state.controls.len();
            let index = state
                .controls
                .iter()
                .position(|&e| Some(e) == focused)
                .unwrap_or(0);
            let next = state.controls[(index + if reverse { count - 1 } else { 1 }) % count];
            if let Some(mut focus) = world.get_resource_mut::<InputFocus>() {
                focus.set(next);
            }
        } else if activate && key.is_none() {
            key = focused
                .and_then(|e| world.get::<Control>(e))
                .copied()
                .filter(|c| *c != Control::Open);
        } else if (up || down) && key.is_none() {
            let a = world.resource::<State>().album.as_ref().unwrap();
            let me = &a.members[&a.focus];
            let id = if up {
                me.parents.iter().find_map(|&g| match a.parent(me, g) {
                    Relative::Individual(id) => Some(id),
                    _ => None,
                })
            } else {
                a.children().get(a.page * PAGE).copied()
            };
            key = id.map(Control::Visit);
        }
    }
    let mouse = world.resource::<ButtonInput<MouseButton>>();
    let pressed = mouse.just_pressed(MouseButton::Left);
    let any_down = any_key || mouse.pressed(MouseButton::Left);
    let pointer = pressed
        .then(|| {
            world
                .query::<(&Control, &Interaction)>()
                .iter(world)
                .find_map(|(c, i)| {
                    (*i == Interaction::Pressed && (was_open != (*c == Control::Open)))
                        .then_some(*c)
                })
        })
        .flatten();
    let requested = world.resource_scope(|w, mut s: Mut<State>| {
        let mut choice = None;
        if let Some(msgs) = w.get_resource::<Messages<ActionRequest>>() {
            for request in s
                .reader
                .read(msgs)
                .filter(|r| r.0.action == A11yAction::Click)
            {
                if let Some(id) = Entity::try_from_bits(request.0.target.0) {
                    if let Some(c) = w.get::<Control>(id) {
                        if was_open != (*c == Control::Open) {
                            choice = Some(*c);
                        }
                    }
                }
            }
        }
        choice
    });
    let focused = world
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .iter(world)
        .next()
        .is_none_or(|w| w.focused);
    let blocked = world
        .get_resource::<menu::MenuState>()
        .is_some_and(menu::captures_input_state)
        || world
            .get_resource::<Fvr05ProductionUxStateResource>()
            .is_some_and(|u| u.debug_mode)
        || !focused;
    if world.resource::<State>().held {
        if !any_down {
            world.resource_mut::<State>().held = false;
            world
                .resource_mut::<ProductionConversationLineageUiState>()
                .family_capture = false;
        }
        return;
    }
    let action = key.or(pointer).or(requested);
    if was_open && (blocked || matches!(action, Some(Control::Close | Control::Open))) {
        let mut s = world.resource_mut::<State>();
        s.album = None;
        s.dirty = true;
        s.held = any_down;
        let mut ui = world.resource_mut::<ProductionConversationLineageUiState>();
        ui.family_open = false;
        ui.family_capture = any_down;
    } else if was_open {
        if let Some(a) = action {
            let mut s = world.resource_mut::<State>();
            if s.album.as_mut().unwrap().activate(a) {
                s.dirty = true;
            }
        }
    } else if !blocked
        && action == Some(Control::Open)
        && !world
            .resource::<ProductionConversationLineageUiState>()
            .blocks_world_shortcuts()
    {
        let selected = world
            .get_resource::<Fvr03ProductionVoxelSelectionResource>()
            .and_then(|sel| sel.selected)
            .and_then(|s| s.stable_id)
            .map(|id| id.raw());
        // A stale click on a now-hidden header cannot silently explore someone else.
        // The keyboard shortcut may deliberately open the world album without selection.
        if selected.is_none() && key != Some(Control::Open) {
            return;
        }
        let album = world
            .get_non_send_resource::<ProductionGpuBrainRuntimeResource>()
            .and_then(|r| Album::from_world(r.runtime.world(), selected));
        if let Some(album) = album {
            let mut s = world.resource_mut::<State>();
            s.album = Some(album);
            s.dirty = true;
            world
                .resource_mut::<ProductionConversationLineageUiState>()
                .family_open = true;
        }
    }
}

fn focus_appearance(
    state: Res<State>,
    focus: Option<Res<InputFocus>>,
    mut buttons: Query<(Entity, &Control, &Interaction, &mut BackgroundColor)>,
) {
    let focused = focus.as_deref().and_then(|f| f.0);
    for (entity, control, interaction, mut color) in &mut buttons {
        let active =
            (state.album.is_some() && focused == Some(entity)) || *interaction != Interaction::None;
        if *control == Control::Open && state.album.is_some() {
            continue;
        }
        color.set_if_neq(BackgroundColor(if active {
            Color::srgb(0.20, 0.38, 0.38)
        } else {
            ink::WELL
        }));
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tests_render;
