//! Embedded, scalable album signs and accessible controls.
use super::*;
use bevy::{
    asset::RenderAssetUsages,
    image::{CompressedImageFormats, ImageSampler, ImageType},
};
use std::collections::BTreeMap;

#[derive(Resource, Clone)]
pub(super) struct Signs(BTreeMap<&'static str, Handle<Image>>);
pub(super) fn signs(world: &mut World) -> Signs {
    if let Some(art) = world.get_resource::<Signs>() {
        return art.clone();
    }
    world.init_resource::<Assets<Image>>();
    let mut images = world.resource_mut::<Assets<Image>>();
    let bytes: [(&str, &[u8]); 8] = [
        (
            "cameo",
            include_bytes!("../../../../../assets/ui/kinship/cameo.png"),
        ),
        (
            "family",
            include_bytes!("../../../../../assets/ui/kinship/family.png"),
        ),
        (
            "seed",
            include_bytes!("../../../../../assets/ui/kinship/seed.png"),
        ),
        (
            "veil",
            include_bytes!("../../../../../assets/ui/kinship/veil.png"),
        ),
        (
            "memorial",
            include_bytes!("../../../../../assets/ui/kinship/memorial.png"),
        ),
        (
            "previous",
            include_bytes!("../../../../../assets/ui/kinship/previous.png"),
        ),
        (
            "next",
            include_bytes!("../../../../../assets/ui/kinship/next.png"),
        ),
        (
            "close",
            include_bytes!("../../../../../assets/ui/speech_composer/cancel.png"),
        ),
    ];
    let art = Signs(
        bytes
            .into_iter()
            .map(|(name, bytes)| {
                (
                    name,
                    images.add(
                        Image::from_buffer(
                            bytes,
                            ImageType::Extension("png"),
                            CompressedImageFormats::NONE,
                            true,
                            ImageSampler::linear(),
                            RenderAssetUsages::default(),
                        )
                        .expect("embedded family sign"),
                    ),
                )
            })
            .collect(),
    );
    world.insert_resource(art.clone());
    art
}
pub(super) fn a11y(role: Role, label: &str) -> AccessibilityNode {
    let mut n = accesskit::Node::new(role);
    n.set_label(label);
    if role == Role::Button {
        n.add_action(A11yAction::Click);
    }
    if role == Role::Dialog {
        n.set_modal();
    }
    AccessibilityNode(n)
}
pub(super) fn image(world: &mut World, parent: Entity, art: &Signs, icon: &str, w: f32, h: f32) {
    let mut a = accesskit::Node::new(Role::Image);
    a.set_hidden();
    world.spawn((
        ImageNode::new(art.0[icon].clone()),
        Node {
            width: Val::Px(w),
            height: Val::Px(h),
            ..default()
        },
        AccessibilityNode(a),
        FocusPolicy::Pass,
        bevy::picking::Pickable::IGNORE,
        ChildOf(parent),
    ));
}
pub(super) fn button(
    world: &mut World,
    parent: Entity,
    action: Control,
    art: &Signs,
    icon: &str,
    label: &str,
    size: f32,
) -> Entity {
    let id = world
        .spawn((
            Interaction::None,
            FocusPolicy::Block,
            action,
            Node {
                width: Val::Px(size),
                height: Val::Px(size),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(Val::Percent(36.)),
                ..default()
            },
            BackgroundColor(ink::WELL),
            a11y(Role::Button, label),
            ChildOf(parent),
        ))
        .id();
    image(world, id, art, icon, size * 0.60, size * 0.60);
    id
}
pub(crate) fn open_button(world: &mut World, parent: Entity) {
    let art = signs(world);
    button(
        world,
        parent,
        Control::Open,
        &art,
        "family",
        "Open family album",
        30.,
    );
}
