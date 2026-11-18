use super::*;
use crate::production_voxel_renderer::care_dock::tests::layout_app;

fn member(id: u64, genome: u64, born: u64, parents: &[u64]) -> Member {
    Member {
        id,
        genome,
        born,
        parents: parents.to_vec(),
        founder: parents.is_empty(),
        alive: true,
        name: format!("Individual {id}"),
    }
}
pub(super) fn family() -> Album {
    Album::new(
        [
            member(1, 101, 0, &[]),
            Member {
                alive: false,
                ..member(2, 102, 0, &[])
            },
            member(3, 103, 12, &[101, 102]),
            member(4, 104, 13, &[101, 102]),
            member(5, 105, 14, &[101, 102]),
            member(6, 106, 15, &[101, 102]),
            member(7, 107, 6, &[]),
            member(8, 108, 22, &[103, 107]),
        ],
        Some(3),
    )
    .unwrap()
}

#[test]
fn unambiguous_recorded_genomes_link_parents_and_children_in_birth_order() {
    let mut a = family();
    assert_eq!(a.parent(&a.members[&3], 101), Relative::Individual(1));
    assert_eq!(a.parent(&a.members[&3], 102), Relative::Individual(2));
    assert!(!a.members[&2].alive);
    assert_eq!(a.children(), [8]);
    a.visit(1);
    assert_eq!(a.children(), [3, 4, 5, 6]);
    a.activate(Control::Later);
    assert_eq!(a.page, 1);
    a.activate(Control::Later);
    assert_eq!(a.page, 1);
    a.visit(6);
    assert_eq!(a.page, 0);
    a.activate(Control::Back);
    assert_eq!(a.focus, 1);
    a.activate(Control::Back);
    assert_eq!(a.focus, 3);
}

#[test]
fn missing_future_and_shared_genomes_never_identify_a_parent() {
    let mut a = Album::new(
        [
            member(1, 90, 0, &[]),
            member(2, 90, 0, &[]),
            member(3, 92, 5, &[90, 91]),
            member(4, 91, 6, &[]),
            member(7, 93, 8, &[91, 94]),
        ],
        Some(3),
    )
    .unwrap();
    let child = &a.members[&3];
    assert_eq!(a.parent(child, 90), Relative::Unresolved(90));
    assert_eq!(a.parent(child, 91), Relative::Unresolved(91));
    assert_eq!(a.parent(&a.members[&7], 94), Relative::Unresolved(94));
    a.visit(1);
    assert!(a.children().is_empty());
    assert!(Album::new([member(1, 42, 0, &[])], Some(99)).is_none());
    assert!(Album::new([], None).is_none());
}

fn fixture(size: Vec2) -> App {
    let mut app = layout_app(size);
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>();
    let mut ui = super::super::speech_completion_state_tests::state();
    ui.family_open = true;
    app.insert_resource(ui)
        .init_resource::<InputFocus>()
        .insert_resource(State {
            album: Some(family()),
            dirty: true,
            ..default()
        });
    draw(app.world_mut());
    app.update();
    app.update();
    app
}
fn action(world: &mut World, c: Control) -> Entity {
    world
        .query::<(Entity, &Control)>()
        .iter(world)
        .find_map(|(e, x)| (*x == c).then_some(e))
        .unwrap()
}
fn fire(app: &mut App, c: Control) {
    let id = action(app.world_mut(), c);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    *app.world_mut().get_mut::<Interaction>(id).unwrap() = Interaction::Pressed;
    input(app.world_mut());
    draw(app.world_mut());
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .reset_all();
}
#[test]
fn responsive_wordless_album_navigates_parents_descendants_and_back() {
    for size in [Vec2::new(960., 540.), Vec2::new(320., 240.)] {
        let mut app = fixture(size);
        let root = app.world().resource::<State>().root.unwrap();
        let world = app.world_mut();
        let mut all = vec![root];
        for i in 0..128 {
            let Some(&e) = all.get(i) else { break };
            if let Some(children) = world.get::<Children>(e) {
                all.extend(children.iter());
            }
        }
        assert!(all.iter().all(|&e| world.get::<Text>(e).is_none()));
        let parent = action(world, Control::Visit(1));
        let node = world.get::<bevy::ui::ComputedNode>(parent).unwrap();
        assert!(node.size().min_element() >= 30.);
        let point = world
            .get::<bevy::ui::UiGlobalTransform>(parent)
            .unwrap()
            .affine()
            .translation;
        assert!(point.x > 0. && point.y > 0. && point.x < size.x && point.y < size.y);
        fire(&mut app, Control::Visit(1));
        assert_eq!(
            app.world()
                .resource::<State>()
                .album
                .as_ref()
                .unwrap()
                .focus,
            1
        );
        fire(&mut app, Control::Later);
        assert_eq!(
            app.world().resource::<State>().album.as_ref().unwrap().page,
            1
        );
        fire(&mut app, Control::Visit(6));
        assert_eq!(
            app.world()
                .resource::<State>()
                .album
                .as_ref()
                .unwrap()
                .focus,
            6
        );
        fire(&mut app, Control::Back);
        assert_eq!(
            app.world()
                .resource::<State>()
                .album
                .as_ref()
                .unwrap()
                .focus,
            1
        );
        fire(&mut app, Control::Close);
        assert!(app.world().resource::<State>().album.is_none());
        assert!(app
            .world()
            .resource::<ProductionConversationLineageUiState>()
            .blocks_world_shortcuts());
        input(app.world_mut());
        assert!(!app
            .world()
            .resource::<ProductionConversationLineageUiState>()
            .blocks_world_shortcuts());
    }
}
