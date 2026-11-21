//! A stepped slider in the existing, capturing graphics modal. No World input.
use super::*;
use crate::production_voxel_frontend::{FOLIAGE_DEFAULT, FOLIAGE_MAX};
use bevy::ecs::hierarchy::ChildSpawner;
use bevy::ui::UiGlobalTransform;

#[derive(Component)]
struct Track;
#[derive(Component)]
pub(super) struct Thumb;

#[derive(Default)]
pub(super) struct Drag {
    held: bool,
    dirty: bool,
}

fn percent(v: u8) -> f32 {
    100. * f32::from(v) / f32::from(FOLIAGE_MAX)
}
pub(super) fn thumb_offset(v: u8) -> Val {
    Val::Percent(percent(v))
}
fn snap(v: f64) -> Option<u8> {
    v.is_finite().then(|| v.round().clamp(0., f64::from(FOLIAGE_MAX)) as u8)
}

pub(super) fn accessibility() -> AccessibilityNode {
    let mut node = accessible(Role::Slider, "Foliage density");
    node.set_min_numeric_value(0.);
    node.set_max_numeric_value(f64::from(FOLIAGE_MAX));
    node.set_numeric_value_step(1.);
    node.set_numeric_value(f64::from(FOLIAGE_DEFAULT));
    for action in [AccessibleAction::Increment, AccessibleAction::Decrement, AccessibleAction::SetValue] {
        node.add_action(action);
    }
    node
}

pub(super) fn access_value(request: &accesskit::ActionRequest, value: u8) -> Option<u8> {
    match (&request.action, &request.data) {
        (AccessibleAction::Increment, _) => Some(value.saturating_add(1).min(FOLIAGE_MAX)),
        (AccessibleAction::Decrement, _) => Some(value.saturating_sub(1)),
        (AccessibleAction::SetValue, Some(accesskit::ActionData::NumericValue(next))) => snap(*next),
        _ => None,
    }
}

pub(super) fn mount(row: &mut ChildSpawner, glyph: &Handle<Image>) {
    row.spawn((
        ImageNode::new(glyph.clone()),
        FocusPolicy::Pass,
        Node {
            width: Val::Px(25.), height: Val::Px(25.),
            margin: UiRect::right(Val::Px(18.)), ..default()
        },
    ));
    row.spawn((
        Name::new("Foliage slider track"),
        FocusPolicy::Pass,
        Track,
        Node { width: Val::Px(130.), height: Val::Px(36.), ..default() },
    )).with_children(|track| {
        track.spawn((
            FocusPolicy::Pass,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(16.), width: Val::Percent(100.), height: Val::Px(4.),
                border_radius: BorderRadius::all(Val::Px(2.)), ..default()
            },
            BackgroundColor(ink::GLASS),
        ));
        for step in 0..=FOLIAGE_MAX {
            track.spawn((
                FocusPolicy::Pass,
                Node {
                    position_type: PositionType::Absolute,
                    left: thumb_offset(step), top: Val::Px(15.),
                    margin: UiRect::left(Val::Px(-3.)),
                    width: Val::Px(6.), height: Val::Px(6.),
                    border_radius: BorderRadius::all(Val::Percent(50.)), ..default()
                },
                BackgroundColor(ink::PORCELAIN),
            ));
        }
        track.spawn((
            FocusPolicy::Pass,
            Thumb,
            Node {
                position_type: PositionType::Absolute,
                left: thumb_offset(FOLIAGE_DEFAULT), top: Val::Px(11.),
                margin: UiRect::left(Val::Px(-7.)),
                width: Val::Px(14.), height: Val::Px(14.),
                border_radius: BorderRadius::all(Val::Percent(50.)),
                border: UiRect::all(Val::Px(2.)),
                ..default()
            },
            BackgroundColor(ink::JADE),
            bevy::ui::BorderColor::all(ink::PORCELAIN),
        ));
    });
}

#[allow(clippy::too_many_arguments, reason = "Share the native menu's frame and modal capture")]
pub(super) fn handle(
    world: &mut World,
    screen_before: Screen,
    keyboard: &ButtonInput<KeyCode>,
    focused: bool,
    down: bool,
    pressed: bool,
    cursor: Option<Vec2>,
    accessible: Option<u8>,
) {
    let state = world.resource::<MenuState>();
    let visible = screen_before == Screen::Graphics && state.screen == Screen::Graphics
        && focused && !state.busy() && !state.conversation_capture && !state.release_gate;
    if !visible && !state.foliage_drag.held && !state.foliage_drag.dirty {
        return;
    }
    // The final cursor motion and button-up may share an input frame.
    // Only this still-visible, focused owned drag may use that last coordinate.
    let dragging = state.foliage_drag.held;
    let pointer = cursor.filter(|_| visible && (down || pressed || dragging)).and_then(|cursor| {
        let (node, tr) = world
            .query_filtered::<(&ComputedNode, &UiGlobalTransform), With<Track>>()
            .single(world).ok()?;
        let position = node.normalize_point(*tr, cursor)?;
        let value = snap(f64::from(position.x + 0.5) * f64::from(FOLIAGE_MAX))?;
        Some((node.contains_point(*tr, cursor), value))
    });
    let current = world.resource::<Fvr05ProductionUxStateResource>().settings.foliage_density;
    let (next, persist, paused_without_menu) = {
        let mut state = world.resource_mut::<MenuState>();
        let allowed = visible;
        let slider_selected = state.rows().get(state.selected).is_some_and(|row| row.0 == Action::Foliage);
        let mut next = None;
        if allowed {
            if pressed && pointer.is_some_and(|(inside, _)| inside) {
                state.selected = 0;
                state.foliage_drag.held = true;
            }
            if state.foliage_drag.held {
                next = pointer.map(|(_, value)| value);
            } else {
                next = accessible.or_else(|| slider_selected.then(|| {
                    if keyboard.just_pressed(KeyCode::Home) {
                        0
                    } else if keyboard.just_pressed(KeyCode::End) {
                        FOLIAGE_MAX
                    } else if keyboard.just_pressed(KeyCode::ArrowLeft) {
                        current.saturating_sub(1)
                    } else if keyboard.just_pressed(KeyCode::ArrowRight) {
                        current.saturating_add(1).min(FOLIAGE_MAX)
                    } else {
                        current
                    }
                }));
            }
        }
        if !allowed || !down {
            state.foliage_drag.held = false;
        }
        let next = next.filter(|value| *value != current);
        state.foliage_drag.dirty |= next.is_some();
        let persist = !state.foliage_drag.held && std::mem::take(&mut state.foliage_drag.dirty);
        (next, persist, !state.resume_on_close)
    };
    let mut ux = world.resource_mut::<Fvr05ProductionUxStateResource>();
    if let Some(value) = next {
        ux.settings.foliage_density = value;
    }
    if persist {
        match ux.settings.persist_foliage(&ux.ui_settings_path, paused_without_menu) {
            Ok(()) => {
                ux.last_error = None;
                ux.last_action = "Saved foliage density".into();
            }
            Err(error) => {
                ux.last_error = Some(error.to_string());
                ux.last_action = "Foliage setting not saved".into();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_click_track_step_and_accessibility_requests_saturate() {
        assert_eq!(snap(-1.), Some(0));
        assert_eq!(snap(0.499), Some(0));
        assert_eq!(snap(0.5), Some(1));
        assert_eq!(snap(3.5), Some(4));
        assert_eq!(snap(50.), Some(FOLIAGE_MAX));
        assert_eq!(snap(f64::NAN), None);
        assert_eq!(thumb_offset(0), Val::Percent(0.));
        assert_eq!(thumb_offset(FOLIAGE_MAX), Val::Percent(100.));
        for (action, data, initial, expected) in [
            (AccessibleAction::Increment, None, FOLIAGE_MAX, Some(FOLIAGE_MAX)),
            (AccessibleAction::Decrement, None, 0, Some(0)),
            (AccessibleAction::SetValue, Some(accesskit::ActionData::NumericValue(0.)), 4, Some(0)),
            (AccessibleAction::SetValue, Some(accesskit::ActionData::NumericValue(f64::NAN)), 4, None),
            (AccessibleAction::Click, None, 0, None),
        ] {
            let request = accesskit::ActionRequest { action, target: accesskit::NodeId(1), data };
            assert_eq!(access_value(&request, initial), expected);
        }
    }
}
