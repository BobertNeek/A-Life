//! Wordless player controls and the paper print mount. These only present a shelf.
use super::*;

#[derive(Component)]
struct Paper;
#[derive(Component)]
struct PrintLabel;
#[derive(Component)]
struct EmptyPlate;
#[derive(Component)]
struct ShutterLamp;
#[derive(Component)]
pub(in super::super) struct PlateControl(pub(in super::super) Action);

pub(in super::super) fn shutter_button(world: &mut World, icon: &Handle<Image>) {
    let id = world
        .spawn((
            Name::new("Field camera shutter"),
            Interaction::None,
            FocusPolicy::Block,
            accessible(Role::Button, "Take a field photograph. F5."),
            MenuButton {
                screen: Screen::Closed,
                action: Action::Expose,
                index: 1,
            },
            MenuRoot(Screen::Closed),
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(72.),
                top: Val::Px(16.),
                width: Val::Px(44.),
                height: Val::Px(48.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(Val::Px(3.)),
                border_radius: BorderRadius::all(Val::Px(15.)),
                ..default()
            },
            GlobalZIndex(90),
            BackgroundColor(ink::GLASS),
            bevel(true),
        ))
        .id();
    world.spawn((
        ChildOf(id),
        ImageNode::new(icon.clone()),
        Node {
            width: Val::Px(26.),
            height: Val::Px(26.),
            ..default()
        },
        bevy::picking::Pickable::IGNORE,
    ));
    world.entity_mut(id).with_children(|mount| {
        mount.spawn(status_lamp());
    });
}
fn status_lamp() -> impl Bundle {
    (
        ShutterLamp,
        Visibility::Hidden,
        BackgroundColor(ink::JADE),
        bevy::picking::Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(8.),
            height: Val::Px(8.),
            right: Val::Px(4.),
            bottom: Val::Px(4.),
            border_radius: BorderRadius::all(Val::Percent(50.)),
            ..default()
        },
    )
}
pub(in super::super) fn paper(panel: &mut ChildSpawner, icon: &Handle<Image>) {
    panel
        .spawn((
            Name::new("Field camera print"),
            PrintLabel,
            accessible(Role::Image, "Your field photograph"),
            Node {
                width: Val::Percent(100.),
                aspect_ratio: Some(16. / 9.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(Val::Px(4.)),
                ..default()
            },
            BorderColor::all(ink::PORCELAIN),
            BackgroundColor(ink::WELL),
        ))
        .with_children(|mount| {
            mount.spawn((
                Name::new("Developed field photograph"),
                Paper,
                ImageNode::default(),
                Visibility::Hidden,
                Node {
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    ..default()
                },
                bevy::picking::Pickable::IGNORE,
            ));
            mount.spawn(status_lamp());
            mount.spawn((
                EmptyPlate,
                ImageNode::new(icon.clone()),
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(38.),
                    height: Val::Px(38.),
                    ..default()
                },
                bevy::picking::Pickable::IGNORE,
            ));
        });
}
pub(super) fn display_album(world: &mut World, s: &mut Shelf) {
    if let Some(path) = s
        .entries
        .get(s.selected)
        .cloned()
        .filter(|p| Some(p) != s.displayed.as_ref())
    {
        s.displayed = Some(path.clone());
        s.art = plates::read(&path)
            .ok()
            .and_then(|bytes| {
                Image::from_buffer(
                    &bytes,
                    bevy::image::ImageType::Extension("png"),
                    bevy::image::CompressedImageFormats::NONE,
                    true,
                    bevy::image::ImageSampler::linear(),
                    RenderAssetUsages::default(),
                )
                .ok()
            })
            .map(|image| world.resource_mut::<Assets<Image>>().add(image));
        if s.art.is_none() {
            s.failed = true;
        }
    }
    if s.entries.is_empty() {
        s.art = None;
    }
    for mut label in world
        .query_filtered::<&mut AccessibilityNode, With<PrintLabel>>()
        .iter_mut(world)
    {
        if s.art.is_some() {
            label.clear_hidden();
            label.set_label(format!(
                "Photograph {} of {}",
                s.selected + 1,
                s.entries.len()
            ));
        } else {
            label.set_hidden();
        }
    }
    for (mut paper, mut v) in world
        .query_filtered::<(&mut ImageNode, &mut Visibility), With<Paper>>()
        .iter_mut(world)
    {
        if let Some(image) = &s.art {
            if paper.image != *image {
                paper.image = image.clone();
            }
            v.set_if_neq(Visibility::Visible);
        } else {
            v.set_if_neq(Visibility::Hidden);
        }
    }
    for mut v in world
        .query_filtered::<&mut Visibility, With<EmptyPlate>>()
        .iter_mut(world)
    {
        v.set_if_neq(if s.art.is_some() {
            Visibility::Hidden
        } else {
            Visibility::Visible
        });
    }
}
pub(super) fn show_lamps(world: &mut World, s: &Shelf) {
    if s.album_open {
        for (action, mut icon) in world
            .query::<(&PlateControl, &mut ImageNode)>()
            .iter_mut(world)
        {
            let tint = if s.available(action.0) {
                Color::WHITE
            } else {
                Color::srgba(0.60, 0.70, 0.70, 0.46)
            };
            if icon.color != tint {
                icon.color = tint;
            }
        }
    }
    let (visible, color, width, height, radius) = if s.failed {
        (true, Color::srgb(0.92, 0.44, 0.29), 10., 10., 1.)
    } else if !s.available(Action::Expose) {
        (true, ink::PORCELAIN, 14., 4., 2.)
    } else {
        (s.kept, ink::JADE, 8., 8., 4.)
    };
    for (mut v, mut fill, mut node) in world
        .query_filtered::<(&mut Visibility, &mut BackgroundColor, &mut Node), With<ShutterLamp>>()
        .iter_mut(world)
    {
        v.set_if_neq(if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
        fill.set_if_neq(BackgroundColor(color));
        if node.width != Val::Px(width) {
            node.width = Val::Px(width);
            node.height = Val::Px(height);
            node.border_radius = BorderRadius::all(Val::Px(radius));
        }
    }
}
pub(super) fn hide_print(world: &mut World) {
    for mut label in world
        .query_filtered::<&mut AccessibilityNode, With<PrintLabel>>()
        .iter_mut(world)
    {
        label.set_hidden();
    }
}
