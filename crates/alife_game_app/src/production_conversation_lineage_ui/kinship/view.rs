//! Wordless, responsive portrait and relationship diagram.
use super::*;

fn row(world: &mut World, parent: Entity, height: f32, gap: f32) -> Entity {
    world
        .spawn((
            Node {
                width: Val::Percent(100.),
                height: Val::Px(height),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(gap),
                ..default()
            },
            ChildOf(parent),
        ))
        .id()
}
/// Join exactly the visible seats; never draw an edge to an invented individual.
fn branches(world: &mut World, panel: Entity, seats: usize, spacing: f32, up: bool) {
    let row = row(world, panel, 16., 0.);
    if seats == 0 {
        return;
    }
    let span = seats.saturating_sub(1) as f32 * spacing;
    let frame = world
        .spawn((
            Node {
                width: Val::Px(span + 2.),
                height: Val::Px(16.),
                ..default()
            },
            ChildOf(row),
        ))
        .id();
    let mut line = |x: f32, y: f32, w: f32, h: f32| {
        world.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(x),
                top: Val::Px(y),
                width: Val::Px(w),
                height: Val::Px(h),
                ..default()
            },
            BackgroundColor(ink::JADE),
            FocusPolicy::Pass,
            bevy::picking::Pickable::IGNORE,
            ChildOf(frame),
        ));
    };
    line(span * 0.5, if up { 7. } else { 0. }, 2., 9.);
    line(0., 7., span + 2., 2.);
    for i in 0..seats {
        line(i as f32 * spacing, if up { 0. } else { 7. }, 2., 9.);
    }
}
fn quiet_sign(world: &mut World, parent: Entity, art: &Signs, kind: &str, label: &str, size: f32) {
    let id = world
        .spawn((Node::default(), a11y(Role::Image, label), ChildOf(parent)))
        .id();
    image(world, id, art, kind, size, size);
}
fn person(world: &mut World, parent: Entity, member: &Member, portrait_height: Option<f32>) {
    let focal = portrait_height.is_some();
    let crest = ink::Crest::new(member.id);
    let label = format!(
        "{}: {}. {}",
        if focal {
            "Selected individual"
        } else {
            "Explore individual"
        },
        member.name,
        if member.alive { "Living" } else { "Deceased" }
    );
    let size = portrait_height.unwrap_or(34.);
    let mut entity = world.spawn((
        Node {
            width: Val::Px(size + 19.),
            height: Val::Px(size),
            border: UiRect::all(Val::Px(if focal { 2. } else { 1. })),
            border_radius: BorderRadius::all(Val::Px(12.)),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(2.),
            ..default()
        },
        a11y(if focal { Role::Image } else { Role::Button }, &label),
        BorderColor::all(if focal { ink::JADE } else { ink::WELL }),
        BackgroundColor(ink::WELL),
        ChildOf(parent),
    ));
    if !focal {
        entity.insert((
            Interaction::None,
            FocusPolicy::Block,
            Control::Visit(member.id),
        ));
    }
    let medal = entity.id();
    let id = if focal {
        let art = signs(world);
        let mut node = world.get_mut::<Node>(medal).unwrap();
        node.flex_direction = FlexDirection::Column;
        node.row_gap = Val::Px(1.);
        node.overflow = Overflow::clip();
        image(world, medal, &art, "cameo", size - 23., size - 23.);
        world
            .spawn((
                Node {
                    height: Val::Px(17.),
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(2.),
                    ..default()
                },
                ChildOf(medal),
            ))
            .id()
    } else {
        medal
    };
    world.spawn((
        Node {
            width: Val::Px(9.),
            height: Val::Px(9.),
            border_radius: BorderRadius::MAX,
            ..default()
        },
        BackgroundColor(crest.pigment()),
        ChildOf(id),
    ));
    for slot in 0..5 {
        world.spawn((
            Node {
                width: Val::Px(3.),
                height: Val::Px(crest.bar_height(slot)),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BackgroundColor(if member.alive {
                ink::PORCELAIN
            } else {
                Color::srgb(0.52, 0.62, 0.63)
            }),
            ChildOf(id),
        ));
    }
    if !member.alive {
        let art = signs(world);
        image(world, id, &art, "memorial", 10., 10.);
    }
}
pub(super) fn draw(world: &mut World) {
    let state = world.resource::<State>();
    if !state.dirty && state.album.is_none() {
        return;
    }
    let compact = world
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .iter(world)
        .next()
        .is_some_and(|w| w.height() < 320.);
    if !world.resource::<State>().dirty && compact == world.resource::<State>().compact {
        return;
    }
    let old = {
        let mut s = world.resource_mut::<State>();
        s.dirty = false;
        s.compact = compact;
        s.root.take()
    };
    if let Some(id) = old {
        world.despawn(id);
    }
    if world.resource::<State>().album.is_none() {
        let back = world
            .query::<(Entity, &Control)>()
            .iter(world)
            .find_map(|(e, c)| (*c == Control::Open).then_some(e));
        if let Some(mut focus) = world.get_resource_mut::<InputFocus>() {
            focus.0 = back;
        }
        return;
    }
    let art = signs(world);
    let root = world.spawn((
        Name::new("Family album"),
        a11y(Role::Dialog, "Family album. A snapshot of this world. Select a relative to explore. Escape closes."),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        Interaction::None,
        RelativeCursorPosition::default(),
        FocusPolicy::Block,
        GlobalZIndex(95),
        BackgroundColor(Color::srgba(0.035, 0.070, 0.085, 0.63)),
    )).id();
    let portrait_height = if compact { 49. } else { 84. };
    let panel = world
        .spawn((
            Node {
                width: Val::Percent(91.),
                max_width: Val::Px(358.),
                height: Val::Px(if compact { 224. } else { 272. }),
                padding: UiRect::all(Val::Px(6.)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(Val::Px(22.)),
                ..default()
            },
            BackgroundColor(ink::GLASS),
            ChildOf(root),
        ))
        .id();
    world.resource_scope(|w, mut state: Mut<State>| {
        let album = state.album.as_ref().unwrap();
        let me = &album.members[&album.focus];
        let head = row(w, panel, 32., 16.);
        let mut bar = w.get_mut::<Node>(head).unwrap();
        bar.justify_content = JustifyContent::SpaceBetween;
        bar.padding = UiRect::horizontal(Val::Px(7.));
        if !album.history.is_empty() {
            button(
                w,
                head,
                Control::Back,
                &art,
                "previous",
                "Go back to previous individual",
                31.,
            );
        }
        quiet_sign(w, head, &art, "family", "Recorded family", 25.);
        let close = button(
            w,
            head,
            Control::Close,
            &art,
            "close",
            "Close family album",
            31.,
        );
        let up = row(w, panel, 36., 26.);
        if me.parents.is_empty() {
            quiet_sign(
                w,
                up,
                &art,
                if me.founder { "seed" } else { "veil" },
                if me.founder {
                    "Founder genome; no recorded parents"
                } else {
                    "Parentage unrecorded"
                },
                30.,
            );
        }
        for &genome in &me.parents {
            match album.parent(me, genome) {
                Relative::Individual(id) => person(w, up, &album.members[&id], None),
                Relative::Unresolved(_) => {
                    let slot = w
                        .spawn((
                            Node {
                                width: Val::Px(53.),
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            ChildOf(up),
                        ))
                        .id();
                    quiet_sign(
                        w,
                        slot,
                        &art,
                        "veil",
                        "Recorded parent genome; individual unavailable or ambiguous",
                        30.,
                    );
                }
            }
        }
        branches(w, panel, me.parents.len().max(1), 79., true);
        let center = row(w, panel, portrait_height + 2., 0.);
        person(w, center, me, Some(portrait_height));
        let children = album.children();
        let visible_children = children.len().saturating_sub(album.page * PAGE).min(PAGE);
        branches(w, panel, visible_children, 61., false);
        let down = row(w, panel, 38., 8.);
        if children.is_empty() {
            quiet_sign(
                w,
                down,
                &art,
                "veil",
                "No individual children identified in this world",
                22.,
            );
        }
        for &id in children.iter().skip(album.page * PAGE).take(PAGE) {
            person(w, down, &album.members[&id], None);
        }
        if children.len() > PAGE {
            let paging = row(w, panel, 24., 26.);
            if album.page > 0 {
                button(
                    w,
                    paging,
                    Control::Earlier,
                    &art,
                    "previous",
                    "Previous descendants",
                    24.,
                );
            }
            if (album.page + 1) * PAGE < children.len() {
                button(
                    w,
                    paging,
                    Control::Later,
                    &art,
                    "next",
                    "More descendants",
                    24.,
                );
            }
        }
        if let Some(mut focus) = w.get_resource_mut::<InputFocus>() {
            focus.set(close);
        }
        // Entity indices are recycled; walk the actual rendered tree in reading order.
        state.controls.clear();
        tab_order(w, panel, &mut state.controls);
        state.root = Some(root);
    });
}

fn tab_order(world: &World, root: Entity, controls: &mut Vec<Entity>) {
    if world.get::<Control>(root).is_some() {
        controls.push(root);
    }
    if let Some(children) = world.get::<Children>(root) {
        for child in children.iter() {
            tab_order(world, child, controls);
        }
    }
}
