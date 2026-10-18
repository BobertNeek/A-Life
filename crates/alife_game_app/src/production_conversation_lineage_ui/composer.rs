//! Wordless speech chrome. This module owns presentation and draft controls only;
//! spatial routing and asynchronous emission remain in the conversation owner.
use super::*;
use accesskit::{Action as AccessibleAction, ActionData, Live, Role, Toggled};
use bevy::{
    a11y::{AccessibilityNode, ActionRequest},
    asset::RenderAssetUsages,
    ecs::message::MessageCursor,
    image::{CompressedImageFormats, ImageSampler, ImageType},
    input_focus::InputFocus,
    prelude::*,
    ui::{BorderColor, BorderRadius, FocusPolicy, ScrollPosition},
};
use std::collections::BTreeMap;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Control {
    Address,
    Send,
    Cancel,
    Help,
}
const CONTROLS: [Control; 4] = [
    Control::Address,
    Control::Send,
    Control::Cancel,
    Control::Help,
];
#[derive(Component)]
struct Glyph(Control);
#[derive(Component)]
struct Status;
#[derive(Component)]
pub(crate) struct DraftViewport;
#[derive(Component)]
struct Help;
#[derive(Resource)]
struct Icons(BTreeMap<&'static str, Handle<Image>>);
#[derive(Resource)]
pub(super) struct SystemFontsLoaded;
#[derive(Default, Resource)]
pub(crate) struct InputState {
    pub(super) pending: Option<Control>,
    pressed: bool,
    reader: MessageCursor<ActionRequest>,
    previous_focus: Option<Entity>,
    owns_focus: bool,
}
const HELP: &str = "Enter send  Esc cancel\nTab address  F1 help";
const WIDTH: f32 = 640.0;
const INSET: f32 = 16.0;
const EMBEDDED: [(&str, &[u8]); 8] = [
    (
        "broadcast",
        include_bytes!("../../../../assets/ui/speech_composer/broadcast.png"),
    ),
    (
        "selected",
        include_bytes!("../../../../assets/ui/speech_composer/selected.png"),
    ),
    (
        "send",
        include_bytes!("../../../../assets/ui/speech_composer/send.png"),
    ),
    (
        "cancel",
        include_bytes!("../../../../assets/ui/speech_composer/cancel.png"),
    ),
    (
        "help",
        include_bytes!("../../../../assets/ui/speech_composer/help.png"),
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
];
fn accessible(role: Role, label: &str) -> AccessibilityNode {
    let mut node = accesskit::Node::new(role);
    node.set_label(label);
    AccessibilityNode(node)
}
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
fn decorative_accessibility() -> AccessibilityNode {
    let mut node = accessible(Role::Image, "");
    node.set_hidden();
    node
}
fn decoration() -> (bevy::picking::Pickable, FocusPolicy) {
    (bevy::picking::Pickable::IGNORE, FocusPolicy::Pass)
}
pub(crate) fn spawn(world: &mut World) {
    // Bevy's embedded default font contains only ASCII. Populate the shared
    // shaping database once so personal names can use installed fallback fonts.
    // Coverage follows the host's fonts; no network or bundled assets are used.
    if !world.contains_resource::<SystemFontsLoaded>() {
        world.init_resource::<bevy::text::CosmicFontSystem>();
        world
            .resource_mut::<bevy::text::CosmicFontSystem>()
            .db_mut()
            .load_system_fonts();
        world.insert_resource(SystemFontsLoaded);
    }
    world.init_resource::<InputState>();
    world.init_resource::<naming::OpenInput>();
    world.init_resource::<InputFocus>();
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
        .expect("embedded speech composer icon");
        icons.insert(name, world.resource_mut::<Assets<Image>>().add(image));
    }
    let root = world
        .spawn((
            Name::new("Player speech composer"),
            ProductionSpeechComposerBounds,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(INSET),
                right: Val::Px(INSET),
                bottom: Val::Px(92.0),
                height: Val::Px(120.0),
                padding: UiRect::all(Val::Px(6.0)),
                border: UiRect::all(Val::Px(3.0)),
                border_radius: BorderRadius::all(Val::Px(18.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            Interaction::None,
            FocusPolicy::Block,
            bevel(true),
            BackgroundColor(Color::srgb(0.075, 0.115, 0.135)),
            GlobalZIndex(80),
            Visibility::Hidden,
            accessible(Role::Group, "Speech from the Hand"),
        ))
        .id();
    let toolbar = world
        .spawn((
            Name::new("Speech controls"),
            ChildOf(root),
            Node {
                height: Val::Px(48.0),
                flex_shrink: 0.0,
                column_gap: Val::Px(6.0),
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .id();
    for control in CONTROLS {
        let (icon, label) = match control {
            Control::Address => ("broadcast", "Address: nearby listeners or a name in your draft. Tab toggles selected creature; a name takes precedence."),
            Control::Send => ("send", "Send speech. Enter."),
            Control::Cancel => ("cancel", "Cancel and discard draft. Escape."),
            Control::Help => ("help", "Keyboard help. F1. Typing returns to the draft."),
        };
        let mut a11y = accessible(Role::Button, label);
        a11y.add_action(AccessibleAction::Click);
        // Explicit Interaction gives real UI hit targets without Bevy's automatic
        // Button labelling clearing wordless controls' AccessKit labels.
        let button = world
            .spawn((
                Name::new(label),
                control,
                Interaction::None,
                FocusPolicy::Block,
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
                a11y,
                ChildOf(toolbar),
            ))
            .id();
        world.spawn((
            Glyph(control),
            decorative_accessibility(),
            ImageNode::new(icons[icon].clone()),
            Node {
                width: Val::Px(32.0),
                height: Val::Px(32.0),
                ..default()
            },
            decoration(),
            ChildOf(button),
        ));
    }
    let status = world
        .spawn((
            Status,
            Name::new("Speech dispatch status"),
            accessible(Role::Status, "No speech dispatch pending"),
            Node {
                width: Val::Px(24.0),
                height: Val::Px(24.0),
                margin: UiRect::left(Val::Auto),
                flex_shrink: 0.0,
                ..default()
            },
            ChildOf(toolbar),
        ))
        .id();
    world.spawn((
        ImageNode::new(icons["pending"].clone()),
        decorative_accessibility(),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        Visibility::Hidden,
        decoration(),
        ChildOf(status),
    ));
    let viewport = world
        .spawn((
            DraftViewport,
            Name::new("Scrollable speech draft"),
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                padding: UiRect::all(Val::Px(4.0)),
                flex_direction: FlexDirection::Column,
                overflow: bevy::ui::Overflow {
                    x: bevy::ui::OverflowAxis::Clip,
                    y: bevy::ui::OverflowAxis::Scroll,
                },
                ..default()
            },
            ScrollPosition::default(),
            BackgroundColor(Color::srgb(0.025, 0.040, 0.050)),
            ChildOf(root),
        ))
        .id();
    let mut a11y = accessible(Role::TextInput, "Speech draft. Enter sends, Tab changes address, Escape cancels. A name in the draft overrides selected addressing.");
    a11y.add_action(AccessibleAction::Focus);
    a11y.add_action(AccessibleAction::SetValue);
    world.spawn((
        Name::new("A-Life player Hand speech entry"),
        ProductionSpeechEntryPanel,
        Text::new("|"),
        TextLayout::new_with_linebreak(bevy::text::LineBreak::WordOrCharacter),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgb(0.98, 0.96, 0.84)),
        Node {
            width: Val::Percent(100.0),
            flex_shrink: 0.0,
            ..default()
        },
        a11y,
        decoration(),
        ChildOf(viewport),
    ));
    world.spawn((
        Help,
        Name::new("Speech keyboard help"),
        Text::new(HELP),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::srgb(0.98, 0.96, 0.84)),
        Node {
            width: Val::Percent(100.0),
            display: Display::None,
            flex_shrink: 0.0,
            ..default()
        },
        accessible(Role::Label, HELP),
        decoration(),
        ChildOf(viewport),
    ));
    world.insert_resource(Icons(icons));
}

/// Runs after dock opening and before the menu/world readers. All unread a11y
/// actions are drained even when closed, unfocused, or behind another modal.
pub(super) fn collect_input(world: &mut World) {
    naming::collect_input(world);
    let Some(state) = world.get_resource::<ProductionConversationLineageUiState>() else {
        return;
    };
    let open = state.input_open;
    let gated = state.input_capture_held;
    let held = world
        .resource::<ButtonInput<MouseButton>>()
        .get_pressed()
        .next()
        .is_some()
        || world
            .resource::<ButtonInput<KeyCode>>()
            .get_pressed()
            .next()
            .is_some();
    let next_capture = (open || gated) && held;
    if next_capture != gated {
        world
            .resource_mut::<ProductionConversationLineageUiState>()
            .input_capture_held = next_capture;
    }
    let blocked = world
        .get_resource::<crate::production_voxel_renderer::menu::MenuState>()
        .is_some_and(crate::production_voxel_renderer::menu::captures_input_state);
    collect_available_input(world, open, blocked);
}
fn collect_available_input(world: &mut World, open: bool, blocked: bool) {
    let focused = world
        .query_filtered::<&Window, With<bevy::window::PrimaryWindow>>()
        .iter(world)
        .next()
        .is_none_or(|w| w.focused);
    let down = world
        .resource::<ButtonInput<MouseButton>>()
        .pressed(MouseButton::Left);
    let controls = world
        .query::<(Entity, &Control, &Interaction)>()
        .iter(world)
        .map(|(e, c, i)| (e, *c, *i))
        .collect::<Vec<_>>();
    let entry = world
        .query_filtered::<Entity, With<ProductionSpeechEntryPanel>>()
        .iter(world)
        .next();
    let actions = world.resource_scope(|world, mut input: Mut<InputState>| {
        world
            .get_resource::<Messages<ActionRequest>>()
            .map(|messages| {
                input
                    .reader
                    .read(messages)
                    .map(|r| r.0.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    });
    let previous_down = world.resource::<InputState>().pressed;
    let mut pending = (down && !previous_down)
        .then(|| {
            controls
                .iter()
                .find_map(|(_, c, i)| (*i == Interaction::Pressed).then_some(*c))
        })
        .flatten();
    for request in actions {
        if !open || blocked || !focused {
            continue;
        }
        if request.action == AccessibleAction::Click {
            pending = controls
                .iter()
                .find_map(|(e, c, _)| (e.to_bits() == request.target.0).then_some(*c))
                .or(pending);
        } else if entry.is_some_and(|e| e.to_bits() == request.target.0)
            && request.action == AccessibleAction::Focus
        {
            world
                .resource_mut::<ProductionConversationLineageUiState>()
                .input_help_open = false;
        } else if entry.is_some_and(|e| e.to_bits() == request.target.0)
            && request.action == AccessibleAction::SetValue
        {
            if let Some(ActionData::Value(value)) = request.data {
                let mut state = world.resource_mut::<ProductionConversationLineageUiState>();
                let naming = state.naming.is_some();
                state.input = value
                    .chars()
                    .filter(|c| naming || !c.is_control())
                    .take(MAX_TYPED_CHARS)
                    .collect();
                if let Some(draft) = state.naming.as_mut() {
                    draft.error = None;
                    draft.preedit.clear();
                }
                state.input_error = None;
                state.input_help_open = false;
            }
        }
    }
    let mut input = world.resource_mut::<InputState>();
    input.pressed = down;
    input.pending = if open && !blocked && focused {
        pending
    } else {
        None
    };
}

/// Shared by actual pointer and keyboard paths; cancel wins combined requests.
pub(super) fn apply_controls(
    state: &mut ProductionConversationLineageUiState,
    keyboard: &ButtonInput<KeyCode>,
    pointer: Option<Control>,
) -> bool {
    if keyboard.just_pressed(KeyCode::Escape) || pointer == Some(Control::Cancel) {
        state.input_open = false;
        state.input.clear();
        state.input_cue = SpeechInputCue::None;
        state.input_error = None;
        state.input_help_open = false;
        state.status = "Player speech cancelled".to_string();
        return true;
    }
    if keyboard.just_pressed(KeyCode::Tab) || pointer == Some(Control::Address) {
        state.address_selected = !state.address_selected;
    }
    if keyboard.just_pressed(KeyCode::F1) || pointer == Some(Control::Help) {
        state.input_help_open = !state.input_help_open;
    }
    false
}

fn width(size: Vec2) -> f32 {
    (size.x - 2.0 * INSET).clamp(0.0, WIDTH)
}
fn height(state: &ProductionConversationLineageUiState, size: Vec2, bottom: f32) -> f32 {
    let columns = ((width(size) - 26.0) / 18.0).floor().max(1.0) as usize;
    let lines = entry_text(state).chars().count().max(1).div_ceil(columns);
    (80.0 + lines as f32 * 18.0 * 1.3)
        .max(120.0)
        .ceil()
        .min((size.y - bottom - 12.0).max(0.0))
}
fn entry_text(state: &ProductionConversationLineageUiState) -> String {
    if let Some(preedit) = state
        .naming
        .as_ref()
        .map(|draft| &draft.preedit)
        .filter(|text| !text.is_empty())
    {
        format!("{}{preedit}|", state.input)
    } else {
        speech_entry_text(state)
    }
}
pub(crate) fn composer_height(world: &World, size: Vec2) -> Option<f32> {
    let state = world.get_resource::<ProductionConversationLineageUiState>()?;
    state
        .input_open
        .then(|| height(state, size, speech_composer_bottom(world)))
}
pub(crate) fn adapt(world: &mut World) {
    naming::sync_ime_window(world);
    let size = world
        .query_filtered::<&Window, With<bevy::window::PrimaryWindow>>()
        .iter(world)
        .next()
        .map_or(Vec2::new(1280.0, 720.0), |w| {
            Vec2::new(w.width(), w.height())
        });
    let bottom = speech_composer_bottom(world);
    let state = world.resource::<ProductionConversationLineageUiState>();
    let open = state.input_open;
    let help = state.input_help_open;
    let selected = state.address_selected;
    let naming = state.naming.is_some();
    let text = entry_text(state);
    let draft = state.input.clone();
    let height = height(state, size, bottom);
    let mut cue = state.input_cue;
    if cue == SpeechInputCue::None
        && !naming
        && world
            .get_non_send_resource::<ProductionSpeechTranslationWorker>()
            .is_some_and(|w| w.active_key() == Some(SpeechTranslationJobKey::Player))
    {
        cue = SpeechInputCue::Pending;
    }
    let icons = world.resource::<Icons>().0.clone();
    for (mut node,mut visible,mut a11y) in world.query_filtered::<(&mut Node,&mut Visibility,&mut AccessibilityNode),With<ProductionSpeechComposerBounds>>().iter_mut(world) {
        node.width=Val::Px(width(size));node.right=Val::Auto;node.left=Val::Px((size.x-width(size))*0.5);
        node.height=Val::Px(height);node.bottom=Val::Px(bottom);
        *visible=if open {Visibility::Inherited}else{Visibility::Hidden};
        a11y.set_label(if naming {"Name the selected individual"}else{"Speech from the Hand"});
        if open {a11y.clear_hidden();} else {a11y.set_hidden();}
    }
    for mut scroll in world
        .query_filtered::<&mut ScrollPosition, With<DraftViewport>>()
        .iter_mut(world)
    {
        scroll.0.y = if open && !help { 1_000_000.0 } else { 0.0 };
    }
    for (mut shown,mut node,mut a11y) in world.query_filtered::<(&mut Text,&mut Node,&mut AccessibilityNode),With<ProductionSpeechEntryPanel>>().iter_mut(world) {
        // Preserve Text change detection so idle frames do not request shaping.
        if shown.0 != text { shown.0.clone_from(&text); }node.display=if help {Display::None}else{Display::Flex};a11y.set_value(draft.clone());
        a11y.set_label(if naming {"Individual name. Up to 32 characters. Enter confirms; Escape cancels."}else{"Speech draft. Enter sends, Tab changes address, Escape cancels. A name in the draft overrides selected addressing."});
        if open && !help {a11y.clear_hidden();}else{a11y.set_hidden();}
    }
    for (mut node, mut a11y, mut text) in world
        .query_filtered::<(&mut Node, &mut AccessibilityNode, &mut Text), With<Help>>()
        .iter_mut(world)
    {
        let next = if naming {
            "Enter confirm  Esc cancel\nName: up to 32 characters"
        } else {
            HELP
        };
        if text.0 != next {
            text.0 = next.into();
        }
        a11y.set_label(next);
        node.display = if help { Display::Flex } else { Display::None };
        if open && help {
            a11y.clear_hidden();
        } else {
            a11y.set_hidden();
        }
    }
    for (control, interaction, mut border, mut a11y, mut node) in world
        .query::<(
            &Control,
            &Interaction,
            &mut BorderColor,
            &mut AccessibilityNode,
            &mut Node,
        )>()
        .iter_mut(world)
    {
        node.display = if naming && *control == Control::Address {
            Display::None
        } else {
            Display::Flex
        };
        *border = bevel(
            *interaction != Interaction::Pressed
                && !(*control == Control::Address && selected)
                && !(*control == Control::Help && help),
        );
        if *control == Control::Address {
            a11y.set_toggled(if selected {
                Toggled::True
            } else {
                Toggled::False
            });
            a11y.set_label(if selected {"Prefer the selected creature when available, otherwise nearby listeners. Tab toggles nearby listeners. A name in the draft takes precedence."}else{"Address nearby listeners or a name in the draft. Tab toggles selected creature."});
        }
        if *control == Control::Help {
            a11y.set_toggled(if help { Toggled::True } else { Toggled::False });
        }
        if *control == Control::Send && draft.trim().is_empty() {
            a11y.set_disabled();
        } else {
            a11y.clear_disabled();
        }
        if *control == Control::Send {
            a11y.set_label(if naming {
                "Confirm individual name. Enter."
            } else {
                "Send speech. Enter."
            });
        }
        if open && !(naming && *control == Control::Address) {
            a11y.clear_hidden();
        } else {
            a11y.set_hidden();
        }
    }
    for (glyph, mut image) in world.query::<(&Glyph, &mut ImageNode)>().iter_mut(world) {
        if glyph.0 == Control::Address {
            image.image = icons[if selected { "selected" } else { "broadcast" }].clone();
        }
        image.color = if glyph.0 == Control::Send && draft.trim().is_empty() {
            Color::srgb(0.45, 0.50, 0.50)
        } else {
            Color::WHITE
        };
    }
    let (icon, label) = match cue {
        SpeechInputCue::None => ("pending", "No speech dispatch pending"),
        SpeechInputCue::Pending => (
            "pending",
            "Speech translation pending; emission unconfirmed",
        ),
        SpeechInputCue::Failed => (
            "failed",
            "Speech rejected or failed. Draft retained; F3 shows details.",
        ),
        SpeechInputCue::Unavailable => ("unavailable", "Speech unavailable. F3 shows details."),
    };
    let status = world
        .query_filtered::<Entity, With<Status>>()
        .single(world)
        .expect("speech status");
    let mut a11y = world.get_mut::<AccessibilityNode>(status).unwrap();
    a11y.set_label(label);
    a11y.set_live(Live::Polite);
    if open {
        a11y.clear_hidden();
    } else {
        a11y.set_hidden();
    }
    let child = world.get::<Children>(status).unwrap()[0];
    world.get_mut::<ImageNode>(child).unwrap().image = icons[icon].clone();
    *world.get_mut::<Visibility>(child).unwrap() = if cue == SpeechInputCue::None {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    sync_focus(world, open, help);
}
fn sync_focus(world: &mut World, open: bool, help: bool) {
    let target = if help {
        world
            .query::<(Entity, &Control)>()
            .iter(world)
            .find_map(|(e, c)| (*c == Control::Help).then_some(e))
    } else {
        world
            .query_filtered::<Entity, With<ProductionSpeechEntryPanel>>()
            .iter(world)
            .next()
    };
    let current = world.resource::<InputFocus>().0;
    let owns = world.resource::<InputState>().owns_focus;
    if open {
        if !owns {
            world.resource_mut::<InputState>().previous_focus = current;
        }
        world.resource_mut::<InputState>().owns_focus = true;
        world.resource_mut::<InputFocus>().0 = target;
    } else if owns {
        let previous = world
            .resource::<InputState>()
            .previous_focus
            .filter(|e| world.get_entity(*e).is_ok());
        world.resource_mut::<InputFocus>().0 = previous;
        world.resource_mut::<InputState>().owns_focus = false;
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) fn image_path(world: &World, handle: &Handle<Image>) -> Option<String> {
    world
        .get_resource::<Icons>()?
        .0
        .iter()
        .find_map(|(name, image)| {
            (image == handle).then(|| {
                format!(
                    "assets/ui/{}/{name}.png",
                    if ["pending", "failed", "unavailable"].contains(name) {
                        "care_dock"
                    } else {
                        "speech_composer"
                    }
                )
            })
        })
}
